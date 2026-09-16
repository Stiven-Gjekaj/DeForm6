//! Writing a [`DeclareTableEntry`] back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 7.1
//!
//! Not one of them is copied from `vb::project`, which is the reader this
//! grades. Section 7.1 marks the entry `[C]`.
//!
//! # What this reader models, and what it leaves
//!
//! Both fields: `dwEntryType` at `0x00` and `lpImportDescriptor` at `0x04`.
//! The emitter writes all 8 bytes, so no byte is unmodelled.
//!
//! The reader keeps each entry that it reads, of every type, in table order.
//! So the walk places an entry by its position in that list alone.
//!
//! [`DeclareTableEntry::descriptor`] is not in these 8 bytes. It is the start
//! of a different structure, and this emitter does not write it.

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::project::DeclareTableEntry;

impl Emit for DeclareTableEntry {
    /// `STRUCTURES.md` section 7.1: an entry is 8 bytes, `[C]`.
    const LEN: u32 = 8;
    const STRUCTURE: &'static str = "DeclareTableEntry";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_u32_le(Off::new(0x00), self.dw_entry_type)?; // dwEntryType
        slate.put_va_le(Off::new(0x04), self.lp_import_descriptor)?; // lpImportDescriptor
        Some(())
    }
}

/// The bytes of a `Declare` table entry that no field of
/// [`DeclareTableEntry`] reproduces, as `(structure relative offset, length)`
/// pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[];

/// The number of `Declare` table entry bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 8;

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
    use crate::read::region::{Off, Region, Va};
    use crate::vb::project::{DeclareDescriptor, DeclareTableEntry};

    /// An external entry. The descriptor it keeps is not in the entry's own
    /// bytes, so its values appear nowhere below.
    fn entry() -> DeclareTableEntry {
        DeclareTableEntry {
            dw_entry_type: 7,
            lp_import_descriptor: Va::new(0x0040_35FC),
            descriptor: Some(DeclareDescriptor {
                lp_dll_name: Va::new(0x0040_350C),
                lp_api_name: Va::new(0x0040_35EC),
            }),
        }
    }

    /// The 8 bytes that entry would have been read out of.
    fn bytes() -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&7_u32.to_le_bytes());
        raw.extend_from_slice(&0x0040_35FC_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_declare_table_entry_record_is_eight_bytes() {
        assert_eq!(DeclareTableEntry::LEN, 8);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&entry(), &Region::new(&raw, Off::new(0x1A54))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_all_eight_bytes() {
        let raw = bytes();
        let ledger = compare(&entry(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 8);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 0);
        assert_eq!(MODELLED_BYTES, DeclareTableEntry::LEN);
    }

    #[test]
    fn the_unmodelled_ranges_are_the_ones_the_constant_names() {
        let raw = bytes();
        let ledger = compare(&entry(), &Region::new(&raw, Off::new(0))).unwrap();
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
    fn an_entry_type_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = entry();
        wrong.dw_entry_type = 6;
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x1A54))).unwrap();

        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x1A54), 1)]);
        assert!(ledger.tiles());
    }

    #[test]
    fn a_descriptor_address_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = entry();
        wrong.lp_import_descriptor = Va::new(0x0040_35FD);
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x1A54))).unwrap();

        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x1A58), 1)]);
        assert!(ledger.tiles());
    }
}
