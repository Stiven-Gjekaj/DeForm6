//! The `Off`, `Rva` and `Va` offset newtypes.
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{Off, Rva, Va};

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
}
