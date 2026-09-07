//! The `Region` bounded window and the `Off`, `Rva` and `Va` offset newtypes.
//!
//! Every read in this project goes through a [`Region`]. It holds a byte slice
//! and the absolute file offset of its first byte, and it exposes neither.
//! [`Region::take`] is the only route out, it takes a length, and it returns
//! `Option`, so no read can skip a bounds check.
//!
//! A compiled Visual Basic 6 executable mixes two kinds of pointer inside one
//! structure. `VBHeader + 0x30` holds a virtual address. `VBHeader + 0x58`,
//! `+ 0x5C`, `+ 0x60` and `+ 0x64` hold byte offsets from the start of the
//! header. A parser that carries only `u32` reads one as the other, resolves
//! the wrong space, and reports a real string from the wrong place with no
//! error. One survey read the string `MZ` out of the DOS stub that way. These
//! three types make that mistake fail to compile.
//!
//! None of the three implements `Add`, `Sub`, `From<u32>` or `Deref`. Every
//! addition is a `checked_add` that takes a bare `u32` length. Adding two
//! offsets is meaningless, so no signature accepts it. `Va::to_rva` is the
//! only bridge between the three spaces, and it is a `checked_sub`.
//!
//! This module names nothing outside the standard library.

/// A byte offset inside a window of the file.
///
/// An `Off` is relative to the base of the [`Region`] that reads it. Use
/// [`Region::file_offset`] to convert one into an absolute file offset for an
/// error report.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Off(u32);

impl Off {
    /// Builds an offset from a raw value.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Gives the raw value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Adds a length to this offset.
    ///
    /// Returns `None` when the sum passes the end of a `u32`. A wrapped sum is
    /// small, a small sum passes a bounds check, and the parser then reads
    /// real bytes from the wrong place. That is the fault this method refuses.
    ///
    /// The right-hand side is a bare `u32`, never another offset.
    #[must_use]
    pub const fn checked_add(self, n: u32) -> Option<Self> {
        // `Option::map` is not a `const fn` on 1.97.1, so this is a `match`.
        // Keep the `const`. It lets a table of fixed offsets be built at
        // compile time in a later phase.
        match self.0.checked_add(n) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Subtracts a length from this offset.
    ///
    /// Returns `None` when the result goes below zero.
    #[must_use]
    pub const fn checked_sub(self, n: u32) -> Option<Self> {
        match self.0.checked_sub(n) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Gives this offset as an index into a slice.
    ///
    /// This is private, because an index is not a value a caller needs.
    ///
    /// `usize::try_from` is used rather than an `as` cast. A `u32 as usize`
    /// cast compiles clean under the lint wall, so the wall does not catch a
    /// wrong cast here, and a reader then has to work out which direction is
    /// safe. The saturating fallback is correct: an index of `usize::MAX`
    /// fails the `get` that follows it, and a failure is a refusal, not a
    /// read.
    fn index(self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX)
    }
}

/// A relative virtual address, measured from the image base.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Rva(u32);

impl Rva {
    /// Builds a relative virtual address from a raw value.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Gives the raw value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Adds a length to this address.
    ///
    /// Returns `None` when the sum passes the end of a `u32`.
    #[must_use]
    pub const fn checked_add(self, n: u32) -> Option<Self> {
        match self.0.checked_add(n) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }
}

/// A virtual address, as the loader sees it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Va(u32);

impl Va {
    /// Builds a virtual address from a raw value.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Gives the raw value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Tells whether this address is the null pointer.
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Converts this virtual address into a relative virtual address.
    ///
    /// This is the only bridge between the three integer spaces, and it is a
    /// `checked_sub`. An address below the image base returns `None`. A
    /// wrapping subtraction gives a huge address that lands back inside the
    /// file and yields a structure that looks correct.
    #[must_use]
    pub const fn to_rva(self, image_base: u32) -> Option<Rva> {
        match self.0.checked_sub(image_base) {
            Some(v) => Some(Rva(v)),
            None => None,
        }
    }
}

/// A bounded window on the bytes of the file.
///
/// A `Region` holds a byte slice and the absolute file offset of its first
/// byte. It exposes neither. There is no `as_bytes`, no `as_slice`, no
/// `Deref<Target = [u8]>` and no `Index`. [`Region::take`] is the only route
/// out, it takes a length, and it returns `Option`. No accessor on this type
/// is infallible, and that is a property of the type rather than a convention
/// that a reviewer has to enforce.
///
/// # Why every accessor returns `Option` and never `Result`
///
/// This layer does not know the name of the structure it reads or the name of
/// the field. It therefore cannot build a `Site`, and a `Region` that returned
/// a half-populated `Defect` would be worse than one that returned `None`. The
/// caller turns the `None` into a `Defect` at the site that holds the names,
/// and it uses [`Region::file_offset`] to name the absolute byte offset.
#[derive(Clone, Copy, Debug)]
pub struct Region<'a> {
    bytes: &'a [u8],
    base: Off,
}

impl<'a> Region<'a> {
    /// Builds a window over `bytes` whose first byte is at file offset `base`.
    #[must_use]
    pub const fn new(bytes: &'a [u8], base: Off) -> Self {
        Self { bytes, base }
    }

    /// Gives the length of the window in bytes.
    ///
    /// `u32::try_from` is used rather than an `as` cast. The saturating
    /// fallback over-reports a window larger than 4 GiB, which then fails
    /// every bounds check that uses it. That failure is a refusal, not a read.
    #[must_use]
    pub fn len(&self) -> u32 {
        u32::try_from(self.bytes.len()).unwrap_or(u32::MAX)
    }

    /// Tells whether the window holds no bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Converts a window-relative offset into an absolute file offset.
    ///
    /// This is what lets a defect three levels down name a byte offset in the
    /// file without any caller threading one through. The sum is a
    /// `checked_add`, so this returns `Option` like every other reader.
    #[must_use]
    pub fn file_offset(&self, at: Off) -> Option<Off> {
        self.base.checked_add(at.get())
    }

    /// Takes `len` bytes at `at`.
    ///
    /// This is the only route out of a `Region`. It returns `None` when the
    /// end of the read passes the end of the window, and it returns `None`
    /// when `at` plus `len` leaves a `u32`. The second case is the important
    /// one: a wrapped sum is small, a small sum passes a naive bounds check,
    /// and the parser then reports real bytes from the wrong place.
    #[must_use]
    pub fn take(&self, at: Off, len: u32) -> Option<&'a [u8]> {
        let end = at.checked_add(len)?;
        self.bytes.get(at.index()..end.index())
    }

    /// Takes a window of `len` bytes at `at`.
    ///
    /// The new window carries its own base, so `file_offset` on it still
    /// gives an absolute file offset.
    #[must_use]
    pub fn subregion(&self, at: Off, len: u32) -> Option<Region<'a>> {
        let bytes = self.take(at, len)?;
        Some(Region {
            bytes,
            base: self.file_offset(at)?,
        })
    }

    /// Reads one unsigned byte.
    #[must_use]
    pub fn u8(&self, at: Off) -> Option<u8> {
        self.take(at, 1)?.first().copied()
    }

    /// Reads an unsigned 16-bit little-endian value.
    #[must_use]
    pub fn u16_le(&self, at: Off) -> Option<u16> {
        let b: [u8; 2] = self.take(at, 2)?.try_into().ok()?;
        Some(u16::from_le_bytes(b))
    }

    /// Reads an unsigned 32-bit little-endian value.
    #[must_use]
    pub fn u32_le(&self, at: Off) -> Option<u32> {
        // `try_into` on a slice of the wrong length gives `Err`, and `.ok()?`
        // turns that into `None`. No panic and no `unwrap`.
        let b: [u8; 4] = self.take(at, 4)?.try_into().ok()?;
        Some(u32::from_le_bytes(b))
    }

    /// Reads a signed 16-bit little-endian value.
    #[must_use]
    pub fn i16_le(&self, at: Off) -> Option<i16> {
        let b: [u8; 2] = self.take(at, 2)?.try_into().ok()?;
        Some(i16::from_le_bytes(b))
    }

    /// Reads a signed 32-bit little-endian value.
    #[must_use]
    pub fn i32_le(&self, at: Off) -> Option<i32> {
        let b: [u8; 4] = self.take(at, 4)?.try_into().ok()?;
        Some(i32::from_le_bytes(b))
    }

    /// Reads a 32-bit little-endian value as a file offset.
    #[must_use]
    pub fn off_le(&self, at: Off) -> Option<Off> {
        Some(Off::new(self.u32_le(at)?))
    }

    /// Reads a 32-bit little-endian value as a virtual address.
    #[must_use]
    pub fn va_le(&self, at: Off) -> Option<Va> {
        Some(Va::new(self.u32_le(at)?))
    }

    /// Reads the bytes of a NUL-terminated string, without the NUL.
    ///
    /// `max` is mandatory and it is not a convenience. A file with no NUL byte
    /// after `at` would otherwise make the scan run to the end of the window.
    /// This returns `None` when no NUL appears in the first `max` bytes, which
    /// is a refusal, not a truncation.
    ///
    /// The scan stops at `max` bytes or at the end of the window, whichever
    /// comes first. A short file whose last field does hold a NUL therefore
    /// still reads, and the scan is still bounded by `max`.
    ///
    /// This returns bytes. It does not decode them. `char::from(byte)` maps a
    /// byte to the Latin-1 code point, which is the caller's job. Never use
    /// `String::from_utf8_lossy` on a Visual Basic header string: a byte in
    /// 0x80 to 0xFF becomes U+FFFD and the name is lost.
    #[must_use]
    pub fn cstr(&self, at: Off, max: u32) -> Option<&'a [u8]> {
        let rest = self.bytes.get(at.index()..)?;
        // The same saturating conversion as `Off::index`, and safe for the
        // same reason: a limit of `usize::MAX` is cut down by the `min`.
        let limit = usize::try_from(max).unwrap_or(usize::MAX).min(rest.len());
        let window = rest.get(..limit)?;
        let end = window.iter().position(|&b| b == 0)?;
        window.get(..end)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{Off, Region, Rva, Va};

    /// Eight bytes with a known value for every width the readers cover.
    ///
    /// The last four bytes are chosen so the signed readers give a negative
    /// number. A buffer of small positive values cannot tell `i32_le` from
    /// `u32_le`.
    const BUF: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0xFF, 0xFF, 0xFE, 0xFF];

    fn region() -> Region<'static> {
        Region::new(&BUF, Off::new(0))
    }

    #[test]
    fn off_checked_add_gives_the_sum() {
        assert_eq!(Off::new(0x10).checked_add(4), Some(Off::new(0x14)));
    }

    #[test]
    fn off_checked_add_refuses_a_sum_that_leaves_a_u32() {
        assert_eq!(Off::new(u32::MAX).checked_add(1), None);
    }

    #[test]
    fn off_checked_sub_refuses_a_result_below_zero() {
        assert_eq!(Off::new(4).checked_sub(8), None);
    }

    #[test]
    fn va_to_rva_subtracts_the_image_base() {
        assert_eq!(
            Va::new(0x0040_1760).to_rva(0x0040_0000),
            Some(Rva::new(0x1760))
        );
    }

    #[test]
    fn va_to_rva_refuses_an_address_below_the_image_base() {
        assert_eq!(Va::new(0x1000).to_rva(0x0040_0000), None);
    }

    #[test]
    fn va_zero_is_null() {
        assert!(Va::new(0).is_null());
        assert!(!Va::new(1).is_null());
    }

    #[test]
    fn rva_checked_add_refuses_a_sum_that_leaves_a_u32() {
        assert_eq!(Rva::new(u32::MAX).checked_add(1), None);
    }

    #[test]
    fn u32_le_reads_the_last_four_bytes_and_refuses_one_byte_later() {
        let r = region();
        assert_eq!(r.u32_le(Off::new(4)), Some(0xFFFE_FFFF));
        assert_eq!(r.u32_le(Off::new(5)), None);
    }

    #[test]
    fn take_refuses_a_length_that_runs_one_byte_past_the_end() {
        let r = region();
        assert_eq!(r.take(Off::new(4), 4), Some(&BUF[4..8]));
        assert_eq!(r.take(Off::new(4), 5), None);
    }

    #[test]
    fn take_refuses_a_length_that_wraps_a_u32_and_does_not_read_short() {
        let r = region();
        // 8 plus this length wraps to 4, which is inside the window. A naive
        // bounds check would pass and would hand back four real bytes from
        // the wrong place.
        assert_eq!(Off::new(8).checked_add(u32::MAX - 3), None);
        assert_eq!(r.take(Off::new(8), u32::MAX - 3), None);
        assert_eq!(r.take(Off::new(1), u32::MAX), None);
    }

    #[test]
    fn every_fixed_width_reader_agrees_with_the_buffer() {
        let r = region();
        // Plan 01-05 reads the entry-point opcode with `u8`, so it is not an
        // afterthought.
        assert_eq!(r.u8(Off::new(0)), Some(0x01));
        assert_eq!(r.u8(Off::new(7)), Some(0xFF));
        assert_eq!(r.u16_le(Off::new(0)), Some(0x0201));
        assert_eq!(r.u32_le(Off::new(0)), Some(0x0403_0201));
        assert_eq!(r.i16_le(Off::new(4)), Some(-1));
        assert_eq!(r.i32_le(Off::new(4)), Some(-65_537));
    }

    #[test]
    fn every_fixed_width_reader_refuses_a_read_that_runs_past_the_end() {
        let r = region();
        assert_eq!(r.u8(Off::new(8)), None);
        assert_eq!(r.u16_le(Off::new(7)), None);
        assert_eq!(r.u32_le(Off::new(5)), None);
        assert_eq!(r.i16_le(Off::new(7)), None);
        assert_eq!(r.i32_le(Off::new(5)), None);
    }

    #[test]
    fn cstr_gives_the_bytes_before_the_nul() {
        let buf = *b"Main\0XYZ";
        let r = Region::new(&buf, Off::new(0));
        assert_eq!(r.cstr(Off::new(0), 8), Some(&b"Main"[..]));
        // An empty string is a NUL at the first byte, not a refusal.
        assert_eq!(r.cstr(Off::new(4), 4), Some(&b""[..]));
    }

    #[test]
    fn cstr_refuses_when_no_nul_appears_inside_max() {
        let buf = *b"ABCD";
        let r = Region::new(&buf, Off::new(0));
        assert_eq!(r.cstr(Off::new(0), 4), None);

        // The NUL is at index 3, which is one byte outside a max of 3.
        let bounded = *b"ABC\0";
        let r = Region::new(&bounded, Off::new(0));
        assert_eq!(r.cstr(Off::new(0), 3), None);
        assert_eq!(r.cstr(Off::new(0), 4), Some(&b"ABC"[..]));
    }

    #[test]
    fn subregion_rebases_so_file_offset_stays_absolute() {
        let buf = [0_u8; 0x30];
        let image = Region::new(&buf, Off::new(0x1760));
        let sub = image.subregion(Off::new(0x10), 0x20).unwrap();

        assert_eq!(sub.file_offset(Off::new(0)), Some(Off::new(0x1770)));
        assert_eq!(sub.file_offset(Off::new(4)), Some(Off::new(0x1774)));
        assert_eq!(sub.len(), 0x20);
        // The parent is unchanged.
        assert_eq!(image.file_offset(Off::new(0)), Some(Off::new(0x1760)));
    }

    #[test]
    fn off_le_and_va_le_give_the_typed_value() {
        let r = region();
        assert_eq!(r.off_le(Off::new(0)), Some(Off::new(0x0403_0201)));
        assert_eq!(r.va_le(Off::new(0)), Some(Va::new(0x0403_0201)));
        assert_eq!(r.off_le(Off::new(5)), None);
        assert_eq!(r.va_le(Off::new(5)), None);
    }

    #[test]
    fn an_empty_region_has_length_zero() {
        let r = Region::new(&[], Off::new(0x1760));
        assert_eq!(r.len(), 0);
        assert!(r.is_empty());
        assert_eq!(r.take(Off::new(0), 1), None);
        // An empty window still knows where it sits in the file.
        assert_eq!(r.file_offset(Off::new(0)), Some(Off::new(0x1760)));

        assert!(!region().is_empty());
        assert_eq!(region().len(), 8);
    }

    #[test]
    fn file_offset_refuses_a_sum_that_leaves_a_u32() {
        let r = Region::new(&BUF, Off::new(u32::MAX - 2));
        assert_eq!(r.file_offset(Off::new(2)), Some(Off::new(u32::MAX)));
        assert_eq!(r.file_offset(Off::new(3)), None);
        assert!(r.subregion(Off::new(3), 1).is_none());
    }
}
