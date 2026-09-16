//! Writing the descriptor of an external `Declare` entry back over its own
//! bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` sections 7.1 and 21
//!
//! Not one of them is copied from `vb::project`, which is the reader this
//! grades. Section 7.1 gives the two addresses at the start. Section 21 gives
//! the length, 24 bytes, which it measured on the corpus: the machine code
//! that uses the descriptor starts right after the 24 bytes.
//!
//! # What this reader models, and what it leaves
//!
//! Two fields: `lpDllName` at `0x00` and `lpApiName` at `0x04`. The reader
//! reads only these 8 bytes, because the other 16 hold no name:
//!
//! | Bytes | What section 21 says is there |
//! |---|---|
//! | `0x08` | an unknown dword, `0x00040000` in the corpus `[G]`; the address of the thunk data `[L]`; two unknown dwords, 0 in the corpus `[G]` |
//!
//! # Only an entry of type 7 names one
//!
//! An entry of type 6 names a pair of a different shape, and the walk grades
//! nothing there. The walk grades a descriptor one time, however many entries
//! name it.

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::project::DeclareDescriptor;

impl Emit for DeclareDescriptor {
    /// `STRUCTURES.md` section 21: 24 bytes, `[C]` on the corpus.
    const LEN: u32 = 0x18;
    const STRUCTURE: &'static str = "DeclareDescriptor";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_va_le(Off::new(0x00), self.lp_dll_name)?; // lpDllName
        slate.put_va_le(Off::new(0x04), self.lp_api_name)?; // lpApiName
        Some(())
    }
}

/// The bytes of a `Declare` descriptor that no field of
/// [`DeclareDescriptor`] reproduces, as `(structure relative offset, length)`
/// pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[(0x08, 16)];

/// The number of `Declare` descriptor bytes this reader reproduces.
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
    use crate::vb::project::DeclareDescriptor;

    fn descriptor() -> DeclareDescriptor {
        DeclareDescriptor {
            lp_dll_name: Va::new(0x0040_350C),
            lp_api_name: Va::new(0x0040_35EC),
        }
    }

    /// The 24 bytes that descriptor would have been read out of, with every
    /// byte no field claims set to `0xA5`.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x18];
        raw[0x00..0x04].copy_from_slice(&0x0040_350C_u32.to_le_bytes());
        raw[0x04..0x08].copy_from_slice(&0x0040_35EC_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_declare_descriptor_record_is_twenty_four_bytes() {
        assert_eq!(DeclareDescriptor::LEN, 0x18);
        assert_eq!(DeclareDescriptor::LEN, 24);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&descriptor(), &Region::new(&raw, Off::new(0x2FFC))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_eight_of_the_twenty_four_bytes() {
        let raw = bytes();
        let ledger = compare(&descriptor(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 8);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 16);
        assert_eq!(MODELLED_BYTES + 16, DeclareDescriptor::LEN);
    }

    #[test]
    fn the_unmodelled_ranges_are_the_ones_the_constant_names() {
        let raw = bytes();
        let ledger = compare(&descriptor(), &Region::new(&raw, Off::new(0))).unwrap();
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
    fn the_two_addresses_swapped_show_up_at_both_offsets() {
        let raw = bytes();
        let original = descriptor();
        let swapped = DeclareDescriptor {
            lp_dll_name: original.lp_api_name,
            lp_api_name: original.lp_dll_name,
        };
        let ledger = compare(&swapped, &Region::new(&raw, Off::new(0x2FFC))).unwrap();

        // The two addresses differ in their low byte only.
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(
            differs,
            vec![
                Span::new(Off::new(0x2FFC), 1),
                Span::new(Off::new(0x3000), 1)
            ]
        );
        assert!(ledger.tiles());
    }

    #[test]
    fn a_library_name_address_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = descriptor();
        wrong.lp_dll_name = Va::new(0x0040_350D);
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x2FFC))).unwrap();

        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x2FFC), 1)]);
        assert!(ledger.tiles());
    }
}
