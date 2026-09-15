//! The `Slate` bounded write window, the mirror of
//! [`Region`](crate::read::region::Region).
//!
//! A [`Slate`] holds a byte buffer and the absolute file offset of its first
//! byte, and it exposes neither. There is no `as_bytes`, no `as_slice`, no
//! `Deref` and no `Index`. [`Slate::region`] is the only route out, and every
//! write returns `Option`. No accessor on this type is infallible, and that is
//! a property of the type rather than a convention a reviewer has to enforce.
//!
//! # Why every write returns `Option` and never `Result`
//!
//! The same reason `Region` gives for its reads: this layer does not know the
//! name of the structure it writes or the name of the field, so it cannot
//! build a [`crate::error::Site`]. The caller turns the `None` into a
//! [`crate::fidelity::Fault`] at the place that holds the names.
//!
//! `Option` is `#[must_use]` and the gate runs `cargo clippy --all-targets --
//! -D warnings`, so an emitter that drops a write by writing `slate.put_u8(a,
//! b);` instead of `slate.put_u8(a, b)?;` fails the build. The compiler is
//! what catches the dropped write here, not review.
//!
//! # Coverage is what the writes did, never what a table declares
//!
//! A slate remembers which of its bytes were written. That record is the
//! answer to "which bytes does this reader model", and it is built by the
//! writes themselves. There is no list of holes to keep in step with the
//! code, because there is no list at all: a field that is not written is the
//! statement that nothing is modelled there.
//!
//! The buffer starts zero filled and the grading never reads a byte that was
//! not written, so the fill value cannot reach a result. The test
//! `an_unwritten_byte_that_happens_to_equal_the_original_is_never_covered`
//! is what holds that property down.
//!
//! # A second write over a byte is refused
//!
//! Two fields claiming the same bytes is an emitter bug, and it leaves a
//! third range unwritten while looking complete. That would make the coverage
//! figure wrong in the quiet direction. A repeated `0x30` where the second
//! field meant `0x34` therefore fails on the first file it meets.

use crate::read::region::{Off, Region, Va};

/// The largest structure a slate holds.
///
/// Every caller passes a length that comes from `docs/STRUCTURES.md` and
/// never from the file. This cap is what keeps that true if a later caller
/// forgets: a length taken from the file cannot size this allocation.
const MAX_SLATE: u32 = 0x1_0000;

/// A bounded surface a structure writes itself back onto, with the record of
/// which of its bytes were written.
#[derive(Clone, Debug)]
pub struct Slate {
    bytes: Vec<u8>,
    covered: Vec<bool>,
    base: Off,
    refused: Option<Off>,
}

impl Slate {
    /// Builds a slate of `len` bytes whose first byte is at file offset
    /// `base`.
    ///
    /// Returns `None` when `len` is larger than [`MAX_SLATE`], and when
    /// `base` plus `len` leaves a `u32`. The second case matters for the same
    /// reason it matters in `Region::take`: a wrapped sum is small, and a
    /// small sum looks like a valid place.
    #[must_use]
    pub fn new(len: u32, base: Off) -> Option<Self> {
        if len > MAX_SLATE {
            return None;
        }
        base.checked_add(len)?;
        let size = usize::try_from(len).ok()?;
        Some(Self {
            bytes: vec![0; size],
            covered: vec![false; size],
            base,
            refused: None,
        })
    }

    /// Gives the length of the slate in bytes.
    ///
    /// `u32::try_from` rather than an `as` cast, for the reason
    /// `Region::len` gives: the saturating fallback over-reports, and an
    /// over-report then fails every bounds check that uses it.
    #[must_use]
    pub fn len(&self) -> u32 {
        u32::try_from(self.bytes.len()).unwrap_or(u32::MAX)
    }

    /// Tells whether the slate holds no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Converts a slate relative offset into an absolute file offset.
    #[must_use]
    pub fn file_offset(&self, at: Off) -> Option<Off> {
        self.base.checked_add(at.get())
    }

    /// Gives the slate relative offset of the first write this slate refused.
    ///
    /// Kept so a failure can name a place, which is what `Tiling::at` in
    /// `vb::gui` is kept for.
    #[must_use]
    pub const fn refused(&self) -> Option<Off> {
        self.refused
    }

    /// Tells whether the byte at `at` was written.
    ///
    /// Out of range reads `false`: a byte outside the slate was certainly not
    /// written by it.
    #[must_use]
    pub fn is_covered(&self, at: Off) -> bool {
        self.covered.get(index(at)).copied().unwrap_or(false)
    }

    /// Gives the number of bytes that were written.
    #[must_use]
    pub fn covered_len(&self) -> u32 {
        let counted = self.covered.iter().filter(|c| **c).count();
        u32::try_from(counted).unwrap_or(u32::MAX)
    }

    /// Writes one unsigned byte.
    pub fn put_u8(&mut self, at: Off, value: u8) -> Option<()> {
        self.put(at, &value.to_le_bytes())
    }

    /// Writes one unsigned 16-bit value, little endian.
    pub fn put_u16_le(&mut self, at: Off, value: u16) -> Option<()> {
        self.put(at, &value.to_le_bytes())
    }

    /// Writes one unsigned 32-bit value, little endian.
    pub fn put_u32_le(&mut self, at: Off, value: u32) -> Option<()> {
        self.put(at, &value.to_le_bytes())
    }

    /// Writes one signed 16-bit value, little endian.
    pub fn put_i16_le(&mut self, at: Off, value: i16) -> Option<()> {
        self.put(at, &value.to_le_bytes())
    }

    /// Writes one signed 32-bit value, little endian.
    pub fn put_i32_le(&mut self, at: Off, value: i32) -> Option<()> {
        self.put(at, &value.to_le_bytes())
    }

    /// Writes one window relative offset, little endian.
    ///
    /// The type is what keeps a header relative offset from being written
    /// into a field that holds a virtual address. See the `read::region`
    /// module doc comment for the survey that read `MZ` out of the DOS stub
    /// by confusing the two.
    pub fn put_off_le(&mut self, at: Off, value: Off) -> Option<()> {
        self.put(at, &value.get().to_le_bytes())
    }

    /// Writes one virtual address, little endian.
    pub fn put_va_le(&mut self, at: Off, value: Va) -> Option<()> {
        self.put(at, &value.get().to_le_bytes())
    }

    /// Writes a run of bytes.
    pub fn put_bytes(&mut self, at: Off, src: &[u8]) -> Option<()> {
        self.put(at, src)
    }

    /// Gives a read window over what was written, carrying the same base.
    ///
    /// This is the only route out, and `Region` already polices it. No
    /// `as_bytes` is added on either side.
    #[must_use]
    pub fn region(&self) -> Region<'_> {
        Region::new(&self.bytes, self.base)
    }

    /// The one funnel every write goes through.
    ///
    /// Returns `None`, and remembers `at`, when the write leaves the slate
    /// and when any byte of the run was already written.
    fn put(&mut self, at: Off, src: &[u8]) -> Option<()> {
        let len = u32::try_from(src.len()).ok()?;
        let end = at.checked_add(len)?;
        if end.get() > self.len() {
            self.refuse(at);
            return None;
        }

        let start = index(at);
        let stop = index(end);
        if self.covered.get(start..stop)?.iter().any(|c| *c) {
            self.refuse(at);
            return None;
        }

        // `copy_from_slice` panics when the two lengths disagree, and no lint
        // catches that. A zipped loop cannot panic at any pair of lengths,
        // and the range was bounds checked by the `get_mut` that produced it.
        for (slot, byte) in self
            .bytes
            .get_mut(start..stop)?
            .iter_mut()
            .zip(src.iter().copied())
        {
            *slot = byte;
        }
        for flag in self.covered.get_mut(start..stop)?.iter_mut() {
            *flag = true;
        }
        Some(())
    }

    /// Records the first refused write and keeps it.
    fn refuse(&mut self, at: Off) {
        if self.refused.is_none() {
            self.refused = Some(at);
        }
    }
}

/// Gives an offset as an index into a slice.
///
/// `Off::index` in `read::region` is the same two lines, and it is private
/// there deliberately, because an index is not a value a caller needs. This
/// restates it rather than widening that type's surface, the way
/// `vb::object` restates `OBJECT_TABLE_SIZE` rather than importing it.
///
/// The saturating fallback is correct: an index of `usize::MAX` fails the
/// `get` that follows it, and a failure is a refusal, not a write.
fn index(at: Off) -> usize {
    usize::try_from(at.get()).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::integer_division,
        reason = "a test builds the state it needs and must fail loudly when that state is wrong"
    )]

    use super::{MAX_SLATE, Slate};
    use crate::read::region::{Off, Va};

    #[test]
    fn a_new_slate_holds_the_length_it_was_asked_for_and_covers_nothing() {
        let slate = Slate::new(8, Off::new(0x40)).unwrap();
        assert_eq!(slate.len(), 8);
        assert!(!slate.is_empty());
        assert_eq!(slate.covered_len(), 0);
        assert_eq!(slate.refused(), None);
    }

    #[test]
    fn a_slate_larger_than_the_cap_is_refused() {
        assert!(Slate::new(MAX_SLATE, Off::new(0)).is_some());
        assert!(Slate::new(MAX_SLATE + 1, Off::new(0)).is_none());
    }

    #[test]
    fn a_slate_whose_base_plus_length_leaves_a_u32_is_refused() {
        // base + len == u32::MAX exactly is the last place a slate fits.
        assert!(Slate::new(16, Off::new(u32::MAX - 16)).is_some());
        assert!(Slate::new(16, Off::new(u32::MAX - 15)).is_none());
    }

    #[test]
    fn a_write_lands_little_endian_and_marks_exactly_its_own_bytes() {
        let mut slate = Slate::new(8, Off::new(0)).unwrap();
        slate.put_u32_le(Off::new(2), 0x1122_3344).unwrap();
        assert_eq!(
            slate.region().take(Off::new(0), 8).unwrap(),
            &[0x00, 0x00, 0x44, 0x33, 0x22, 0x11, 0x00, 0x00]
        );
        assert_eq!(slate.covered_len(), 4);
        assert!(!slate.is_covered(Off::new(1)));
        assert!(slate.is_covered(Off::new(2)));
        assert!(slate.is_covered(Off::new(5)));
        assert!(!slate.is_covered(Off::new(6)));
    }

    #[test]
    fn every_width_writes_its_own_number_of_bytes() {
        let mut slate = Slate::new(32, Off::new(0)).unwrap();
        slate.put_u8(Off::new(0), 1).unwrap();
        slate.put_u16_le(Off::new(1), 2).unwrap();
        slate.put_i16_le(Off::new(3), -2).unwrap();
        slate.put_u32_le(Off::new(5), 3).unwrap();
        slate.put_i32_le(Off::new(9), -3).unwrap();
        slate.put_off_le(Off::new(13), Off::new(4)).unwrap();
        slate.put_va_le(Off::new(17), Va::new(5)).unwrap();
        slate.put_bytes(Off::new(21), &[7, 7, 7]).unwrap();
        assert_eq!(slate.covered_len(), 1 + 2 + 2 + 4 + 4 + 4 + 4 + 3);
    }

    #[test]
    fn a_second_write_over_a_byte_is_refused_and_the_slate_names_the_offset() {
        let mut slate = Slate::new(16, Off::new(0)).unwrap();
        slate.put_u32_le(Off::new(4), 0xAAAA_AAAA).unwrap();
        // The classic fault: the second field meant 0x08 and repeated 0x04.
        assert!(slate.put_u32_le(Off::new(4), 0xBBBB_BBBB).is_none());
        assert_eq!(slate.refused(), Some(Off::new(4)));
        // The first write survives untouched.
        assert_eq!(
            slate.region().take(Off::new(4), 4).unwrap(),
            &[0xAA, 0xAA, 0xAA, 0xAA]
        );
        assert_eq!(slate.covered_len(), 4);
    }

    #[test]
    fn a_write_that_overlaps_an_earlier_one_by_a_single_byte_is_refused() {
        let mut slate = Slate::new(16, Off::new(0)).unwrap();
        slate.put_u32_le(Off::new(4), 0).unwrap();
        assert!(slate.put_u16_le(Off::new(7), 0).is_none());
        assert_eq!(slate.refused(), Some(Off::new(7)));
    }

    #[test]
    fn the_first_refused_offset_is_the_one_that_is_kept() {
        let mut slate = Slate::new(8, Off::new(0)).unwrap();
        assert!(slate.put_u32_le(Off::new(6), 0).is_none());
        assert!(slate.put_u32_le(Off::new(7), 0).is_none());
        assert_eq!(slate.refused(), Some(Off::new(6)));
    }

    #[test]
    fn a_write_that_runs_past_the_end_is_refused_and_changes_nothing() {
        let mut slate = Slate::new(8, Off::new(0)).unwrap();
        assert!(slate.put_u32_le(Off::new(6), 0xFFFF_FFFF).is_none());
        assert_eq!(slate.covered_len(), 0);
        assert_eq!(slate.region().take(Off::new(0), 8).unwrap(), &[0; 8]);
    }

    #[test]
    fn a_write_whose_offset_plus_length_leaves_a_u32_is_refused() {
        let mut slate = Slate::new(8, Off::new(0)).unwrap();
        assert!(slate.put_u32_le(Off::new(u32::MAX - 1), 0).is_none());
    }

    #[test]
    fn the_region_a_slate_gives_back_carries_the_slates_own_base() {
        let mut slate = Slate::new(4, Off::new(0x1234)).unwrap();
        slate.put_u32_le(Off::new(0), 0).unwrap();
        assert_eq!(slate.file_offset(Off::new(0)), Some(Off::new(0x1234)));
        assert_eq!(
            slate.region().file_offset(Off::new(2)),
            Some(Off::new(0x1236))
        );
    }

    #[test]
    fn an_unwritten_byte_that_happens_to_equal_the_original_is_never_covered() {
        // The whole three way grading rests on this. A slate starts zero
        // filled, so an unwritten byte equals a zero in the file. If coverage
        // were inferred by comparing bytes, that byte would read as modelled
        // and the coverage figure would be wrong in the quiet direction.
        let slate = Slate::new(4, Off::new(0)).unwrap();
        assert_eq!(slate.region().take(Off::new(0), 4).unwrap(), &[0, 0, 0, 0]);
        assert_eq!(slate.covered_len(), 0);
        assert!(!slate.is_covered(Off::new(0)));
    }

    #[test]
    fn an_empty_slate_is_empty_and_takes_no_write() {
        let mut slate = Slate::new(0, Off::new(0)).unwrap();
        assert!(slate.is_empty());
        assert_eq!(slate.len(), 0);
        assert!(slate.put_u8(Off::new(0), 1).is_none());
    }
}
