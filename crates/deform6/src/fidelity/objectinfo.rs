//! Writing an [`ObjectInfo`] back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 5.2
//!
//! Not one of them is copied from `vb::privateobj`, which is the reader this
//! grades.
//!
//! # What this reader models, and what it leaves
//!
//! Two fields: `wObjectIndex` at `0x02` and `lpPrivateObject` at `0x0C`. The
//! reader keeps `lpPrivateObject` as a plain `u32`, because `0xFFFFFFFF` in it
//! means "this object has no private object" and is not an address. This
//! emitter writes the same four bytes either way.
//!
//! The bytes no field claims are all named by section 5.2:
//!
//! | Bytes | Fields section 5.2 names there |
//! |---|---|
//! | `0x00` | `wRefCount` |
//! | `0x04` | `lpObjectTable`, `lpIdeData` |
//! | `0x10` | `dwReserved`, `dwNull`, `lpObject`, `lpProjectData`, `wMethodCount`, `wMethodCount2`, `lpMethods`, `wConstants`, `wMaxConstants`, `lpIdeData2`, `lpIdeData3`, `lpConstants` |
//!
//! `OptionalObjectInfo` starts at `0x38`, immediately after this record. It is
//! graded as a structure of its own and is not part of `LEN`.

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::privateobj::ObjectInfo;

impl Emit for ObjectInfo {
    /// `STRUCTURES.md` section 5.2: `0x38` = 56 bytes.
    const LEN: u32 = 0x38;
    const STRUCTURE: &'static str = "ObjectInfo";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_u16_le(Off::new(0x02), self.w_object_index)?; // wObjectIndex
        slate.put_u32_le(Off::new(0x0C), self.lp_private_object)?; // lpPrivateObject
        Some(())
    }
}

/// The bytes of an `ObjectInfo` no field of [`ObjectInfo`] reproduces, as
/// `(structure relative offset, length)` pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[(0x00, 2), (0x04, 8), (0x10, 40)];

/// The number of `ObjectInfo` bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 6;

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

    use super::{MODELLED_BYTES, UNMODELLED};
    use crate::fidelity::ledger::{Span, Verdict};
    use crate::fidelity::{Emit, compare};
    use crate::read::region::{Off, Region};
    use crate::vb::privateobj::ObjectInfo;

    fn info() -> ObjectInfo {
        ObjectInfo {
            w_object_index: 0x0003,
            lp_private_object: 0x0040_8888,
        }
    }

    /// The 56 bytes that value would have been read out of, with every byte
    /// no field claims set to `0xA5`.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x38];
        raw[0x02..0x04].copy_from_slice(&0x0003_u16.to_le_bytes());
        raw[0x0C..0x10].copy_from_slice(&0x0040_8888_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_object_info_record_is_fifty_six_bytes() {
        assert_eq!(ObjectInfo::LEN, 0x38);
        assert_eq!(ObjectInfo::LEN, 56);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&info(), &Region::new(&raw, Off::new(0x3000))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_six_of_the_fifty_six_bytes() {
        let raw = bytes();
        let ledger = compare(&info(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 6);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 50);
        assert_eq!(MODELLED_BYTES + 50, ObjectInfo::LEN);
    }

    #[test]
    fn the_unmodelled_ranges_are_the_ones_the_constant_names() {
        let raw = bytes();
        let ledger = compare(&info(), &Region::new(&raw, Off::new(0))).unwrap();
        let measured: Vec<Span> = ledger
            .runs_with(Verdict::Unmodelled)
            .map(|run| run.span)
            .collect();
        let declared: Vec<Span> = UNMODELLED
            .iter()
            .map(|(at, len)| Span::new(Off::new(*at), *len))
            .collect();
        assert_eq!(measured, declared);
    }

    #[test]
    fn a_private_object_address_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = info();
        wrong.lp_private_object ^= 1;
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x3000))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x300C), 1)]);
        assert!(ledger.tiles());
    }
}
