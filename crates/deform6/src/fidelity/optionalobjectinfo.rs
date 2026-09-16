//! Writing an [`OptionalObjectInfo`] back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 5.3
//!
//! Not one of them is copied from `vb::controlinfo`, which is the reader this
//! grades. The block sits at `lpObjectInfo + 0x38`, immediately after
//! `ObjectInfo`, and it is not a separate allocation.
//!
//! # What this reader models, and what it leaves
//!
//! Two fields: `dwControlCount` at `0x20` and `lpControls` at `0x24`. Both are
//! `[C]`. The reader keeps the count exactly as the file holds it, never the
//! clamped copy it loops on, so this emitter writes back the file's own number.
//!
//! The bytes no field claims:
//!
//! | Bytes | What section 5.3 says is there |
//! |---|---|
//! | `0x00` | `dwObjectGuiGuids` `[C]`, `lpObjectCLSID` `[C]`, `dwNull` `[C]`, `lpGuidObjectGUITable` `[L]`, `dwObjectDefaultIIDCount` `[L]`, three words the sources disagree on `[D]` |
//! | `0x28` | `wMethodLinkCount` `[L]`, `wPCodeCount`, `bWInitializeEvent`, `bWTerminateEvent`, `lpMethodLinkTable`, `lpBasicClassObject`, `dwNull3`, `lpIdeData` |

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::controlinfo::OptionalObjectInfo;

impl Emit for OptionalObjectInfo {
    /// `STRUCTURES.md` section 5.3: `0x40` = 64 bytes, `[C]`.
    const LEN: u32 = 0x40;
    const STRUCTURE: &'static str = "OptionalObjectInfo";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_u32_le(Off::new(0x20), self.dw_control_count)?; // dwControlCount
        slate.put_va_le(Off::new(0x24), self.lp_controls)?; // lpControls
        Some(())
    }
}

/// The bytes of an `OptionalObjectInfo` block no field of
/// [`OptionalObjectInfo`] reproduces, as `(structure relative offset,
/// length)` pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[(0x00, 32), (0x28, 24)];

/// The number of `OptionalObjectInfo` bytes this reader reproduces.
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
    use crate::vb::controlinfo::OptionalObjectInfo;

    fn info() -> OptionalObjectInfo {
        OptionalObjectInfo {
            dw_control_count: 0x0000_0009,
            lp_controls: Va::new(0x0040_AAAA),
        }
    }

    /// The 64 bytes that block would have been read out of, with every byte
    /// no field claims set to `0xA5`.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x40];
        raw[0x20..0x24].copy_from_slice(&0x0000_0009_u32.to_le_bytes());
        raw[0x24..0x28].copy_from_slice(&0x0040_AAAA_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_optional_object_info_block_is_sixty_four_bytes() {
        assert_eq!(OptionalObjectInfo::LEN, 0x40);
        assert_eq!(OptionalObjectInfo::LEN, 64);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&info(), &Region::new(&raw, Off::new(0x5038))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_eight_of_the_sixty_four_bytes() {
        let raw = bytes();
        let ledger = compare(&info(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 8);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 56);
        assert_eq!(MODELLED_BYTES + 56, OptionalObjectInfo::LEN);
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
    fn a_control_count_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = info();
        wrong.dw_control_count ^= 1;
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x5038))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x5058), 1)]);
        assert!(ledger.tiles());
    }
}
