//! Writing the `GUIObjectInfo` of a form back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 8.2
//!
//! Not one of them is copied from `vb::gui`, which is the reader this grades.
//! Section 8.2 rates the layout `[L]`, and it warns that the single byte at
//! `0x04` moves every later field to an odd offset. `tests/byte_fidelity.rs`
//! checks that warning against the GUI table entry, which holds the same GUID.
//!
//! # Why `Emit` is on a record type
//!
//! [`GuiObjectInfo`] keeps a private window on the file, so a test outside
//! `vb::gui` cannot build one. [`GuiObjectInfoRecord::of`] copies the one field
//! that the reader keeps, and the tests below build the record directly.
//!
//! The window is private, so `of` cannot name every field of the reader. A
//! field added to the reader therefore does not stop this file compiling, as
//! it does for `PrivateObjRecord`.
//!
//! # What this reader models, and what it leaves
//!
//! One field: `lPropertiesLength` at `0x59`. The reader reads nothing else.
//!
//! | Bytes | What section 8.2 says is there |
//! |---|---|
//! | `0x00` | `lUnknown1`, `bUnknown2`, `guidObjectGUI`, `uuidUnknown1`, `guidCOMEventsIID`, nine unknown dwords |

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::gui::GuiObjectInfo;

/// The field of a `GUIObjectInfo` that the reader keeps, as one record that
/// can be written back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuiObjectInfoRecord {
    l_properties_length: u32,
}

impl GuiObjectInfoRecord {
    /// Gives the record that `value` was read from.
    #[must_use]
    pub const fn of(value: &GuiObjectInfo<'_>) -> Self {
        Self {
            l_properties_length: value.l_properties_length,
        }
    }
}

impl Emit for GuiObjectInfoRecord {
    /// `STRUCTURES.md` section 8.2: `0x5D` = 93 bytes, `[L]`.
    const LEN: u32 = 0x5D;
    const STRUCTURE: &'static str = "GuiObjectInfo";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_u32_le(Off::new(0x59), self.l_properties_length)?; // lPropertiesLength
        Some(())
    }
}

/// The bytes of a `GUIObjectInfo` that no field of [`GuiObjectInfoRecord`]
/// reproduces, as `(structure relative offset, length)` pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[(0x00, 89)];

/// The number of `GUIObjectInfo` bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 4;

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

    use super::{GuiObjectInfoRecord, MODELLED_BYTES, UNMODELLED};
    use crate::fidelity::ledger::{Span, Verdict};
    use crate::fidelity::{Emit, compare};
    use crate::read::region::{Off, Region};

    fn record() -> GuiObjectInfoRecord {
        GuiObjectInfoRecord {
            l_properties_length: 0x0000_00B7,
        }
    }

    /// The 93 bytes that record would have been read out of, with every byte
    /// no field claims set to `0xA5`.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x5D];
        raw[0x59..0x5D].copy_from_slice(&0x0000_00B7_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_gui_object_info_block_is_ninety_three_bytes() {
        assert_eq!(GuiObjectInfoRecord::LEN, 0x5D);
        assert_eq!(GuiObjectInfoRecord::LEN, 93);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&record(), &Region::new(&raw, Off::new(0x1C00))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_four_of_the_ninety_three_bytes() {
        let raw = bytes();
        let ledger = compare(&record(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 4);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 89);
        assert_eq!(MODELLED_BYTES + 89, GuiObjectInfoRecord::LEN);
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
    fn a_properties_length_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = record();
        wrong.l_properties_length ^= 1;
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x1C00))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x1C59), 1)]);
        assert!(ledger.tiles());
    }
}
