//! Writing the object table back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 4
//!
//! Not one of them is copied from `vb::project`, which is the reader this
//! grades. See the module doc comment on `crate::fidelity` for why.
//!
//! # Why `Emit` is on a record type
//!
//! [`ObjectTableHead`] keeps its defects in a private field, so a test outside
//! `vb::project` cannot build one. [`ObjectTableRecord::of`] copies the four
//! fields that the emitter writes, and the tests below build the record
//! directly.
//!
//! `of` names the fields one by one, and the head keeps one value that is not
//! in these 84 bytes: the project name. So a field added to the head does not
//! stop this file compiling, as it does for `PrivateObjRecord`.
//!
//! # What this reader models, and what it leaves
//!
//! Four fields: `wTotalObjects`, `wCompiledObjects`, `lpObjectArray` and
//! `lpszProjectName`. The bytes no field claims are named by section 4:
//!
//! | Bytes | Fields section 4 names there |
//! |---|---|
//! | `0x00` | `lpHeapLink`, `lpExecProj`, `lpProjectInfo2`, `dwReserved`, `dwNull`, `lpProjectObject`, `uuidObject`, `fCompileState` |
//! | `0x2E` | `wObjectsInUse` |
//! | `0x34` | `fIdeFlag`, `lpIdeData`, `lpIdeData2` |
//! | `0x44` | `dwLcid`, `dwLcid2`, `lpIdeData3`, `dwIdentifier` |

use crate::fidelity::{Emit, Slate};
use crate::read::region::{Off, Va};
use crate::vb::project::ObjectTableHead;

/// The fields of the object table that the head keeps, as one record that
/// can be written back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectTableRecord {
    w_total_objects: u16,
    w_compiled_objects: u16,
    lp_object_array: Va,
    lpsz_project_name: Va,
}

impl ObjectTableRecord {
    /// Gives the record that `head` was read from.
    #[must_use]
    pub const fn of(head: &ObjectTableHead) -> Self {
        Self {
            w_total_objects: head.w_total_objects,
            w_compiled_objects: head.w_compiled_objects,
            lp_object_array: head.lp_object_array,
            lpsz_project_name: head.lpsz_project_name,
        }
    }
}

impl Emit for ObjectTableRecord {
    /// `STRUCTURES.md` section 4: `0x54` = 84 bytes.
    const LEN: u32 = 0x54;
    const STRUCTURE: &'static str = "ObjectTable";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_u16_le(Off::new(0x2A), self.w_total_objects)?; // wTotalObjects
        slate.put_u16_le(Off::new(0x2C), self.w_compiled_objects)?; // wCompiledObjects
        slate.put_va_le(Off::new(0x30), self.lp_object_array)?; // lpObjectArray
        slate.put_va_le(Off::new(0x40), self.lpsz_project_name)?; // lpszProjectName
        Some(())
    }
}

/// The bytes of the object table that no field of [`ObjectTableRecord`]
/// reproduces, as `(structure relative offset, length)` pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[(0x00, 42), (0x2E, 2), (0x34, 12), (0x44, 16)];

/// The number of object table bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 12;

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

    use super::{MODELLED_BYTES, ObjectTableRecord, UNMODELLED};
    use crate::fidelity::ledger::{Span, Verdict};
    use crate::fidelity::{Emit, compare};
    use crate::read::region::{Off, Region, Va};

    /// A value whose every field holds a number no other field holds, so a
    /// transposed pair of offsets cannot pass.
    fn record() -> ObjectTableRecord {
        ObjectTableRecord {
            w_total_objects: 0x0003,
            w_compiled_objects: 0x0004,
            lp_object_array: Va::new(0x0040_1054),
            lpsz_project_name: Va::new(0x0040_1170),
        }
    }

    /// The 84 bytes that record would have been read out of, with every byte
    /// no field claims set to `0xA5`.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x54];
        let mut put = |at: usize, src: &[u8]| raw[at..at + src.len()].copy_from_slice(src);
        put(0x2A, &0x0003_u16.to_le_bytes());
        put(0x2C, &0x0004_u16.to_le_bytes());
        put(0x30, &0x0040_1054_u32.to_le_bytes());
        put(0x40, &0x0040_1170_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_object_table_is_eighty_four_bytes() {
        assert_eq!(ObjectTableRecord::LEN, 0x54);
        assert_eq!(ObjectTableRecord::LEN, 84);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&record(), &Region::new(&raw, Off::new(0x1800))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_twelve_of_the_eighty_four_bytes() {
        let raw = bytes();
        let ledger = compare(&record(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 12);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 72);
        assert_eq!(MODELLED_BYTES + 72, ObjectTableRecord::LEN);
    }

    #[test]
    fn the_unmodelled_ranges_are_the_ones_the_constant_names() {
        let raw = bytes();
        let ledger = compare(&record(), &Region::new(&raw, Off::new(0))).unwrap();
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
    fn a_capacity_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = record();
        wrong.w_compiled_objects ^= 1;
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x1800))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x182C), 1)]);
        assert!(ledger.tiles());
    }
}
