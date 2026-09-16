//! Writing a [`GuiTableEntry`] back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 8.1
//!
//! Not one of them is copied from `vb::gui`, which is the reader this grades.
//! Section 8 as a whole is `[L]` at best unless a line says otherwise, and the
//! two fields this reader keeps are two of the few section 8.1 marks `[C]`.
//!
//! # What this reader models, and what it leaves
//!
//! Two fields: `lStructSize` at `0x00`, which the walk checks against the
//! entry size, and `aFormPointer` at `0x48`. The bytes no field claims:
//!
//! | Bytes | What section 8.1 says is there |
//! |---|---|
//! | `0x04` | `uuidObjectGUI` `[L]`, four unknown dwords `[G]`, `lObjectID` `[L]`, one unknown dword `[G]`, `fOLEMisc` `[L]`, `uuidObject` `[L]`, two unknown dwords `[G]` |
//! | `0x4C` | one unknown dword `[G]` |
//!
//! Unlike the header, several of these are genuine unknowns and not work that
//! is merely not done yet.

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::gui::GuiTableEntry;

impl Emit for GuiTableEntry {
    /// `STRUCTURES.md` section 8.1: the stride is `0x50` = 80 bytes, `[C]`.
    const LEN: u32 = 0x50;
    const STRUCTURE: &'static str = "GuiTableEntry";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_u32_le(Off::new(0x00), self.l_struct_size)?; // lStructSize
        slate.put_va_le(Off::new(0x48), self.a_form_pointer)?; // aFormPointer
        Some(())
    }
}

/// The bytes of a GUI table entry no field of [`GuiTableEntry`] reproduces,
/// as `(structure relative offset, length)` pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[(0x04, 68), (0x4C, 4)];

/// The number of GUI table entry bytes this reader reproduces.
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
    use crate::vb::gui::GuiTableEntry;

    fn entry() -> GuiTableEntry {
        GuiTableEntry {
            l_struct_size: 0x50,
            a_form_pointer: Va::new(0x0040_7777),
        }
    }

    /// The 80 bytes that entry would have been read out of, with every byte
    /// no field claims set to `0xA5`.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x50];
        raw[0x00..0x04].copy_from_slice(&0x50_u32.to_le_bytes());
        raw[0x48..0x4C].copy_from_slice(&0x0040_7777_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_gui_table_entry_record_is_eighty_bytes() {
        assert_eq!(GuiTableEntry::LEN, 0x50);
        assert_eq!(GuiTableEntry::LEN, 80);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&entry(), &Region::new(&raw, Off::new(0x2000))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_eight_of_the_eighty_bytes() {
        let raw = bytes();
        let ledger = compare(&entry(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 8);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 72);
        assert_eq!(MODELLED_BYTES + 72, GuiTableEntry::LEN);
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
    fn a_form_pointer_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = entry();
        wrong.a_form_pointer = Va::new(0x0040_7776);
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x2000))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x2048), 1)]);
        assert!(ledger.tiles());
    }
}
