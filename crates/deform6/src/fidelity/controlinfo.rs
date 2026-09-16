//! Writing a [`ControlInfo`] back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 8.6
//!
//! Not one of them is copied from `vb::controlinfo`, which is the reader this
//! grades. Section 8.6 marks the first two fields `[D]`: one published source
//! reads `fControlType` as four bytes and puts `wEventCount` at `0x04`, and
//! three independent implementations read two bytes at `0x00` and `0x02`. The
//! reader follows the three, and so does this emitter, from the same table.
//! The corpus diff is the check on that choice.
//!
//! # What this reader models, and what it leaves
//!
//! Five fields. The resolved `name` is never written: it is a reading of the
//! bytes at `lpszName`, not a field of this record.
//!
//! | Bytes | What section 8.6 says is there |
//! |---|---|
//! | `0x04` | one unknown word `[G]`, `bWEventsOffset` `[C]` |
//! | `0x0C` | `wIndex` `[D]`, three unknown words `[G]`, `dwNull2` `[C]` |
//! | `0x1C` | `lpIdeData` `[C]` |
//! | `0x24` | `dwIndexCopy` `[C]` |

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::controlinfo::ControlInfo;

impl Emit for ControlInfo {
    /// `STRUCTURES.md` section 8.6: the stride is `0x28` = 40 bytes.
    const LEN: u32 = 0x28;
    const STRUCTURE: &'static str = "ControlInfo";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_u16_le(Off::new(0x00), self.f_control_type)?; // fControlType
        slate.put_u16_le(Off::new(0x02), self.w_event_count)?; // wEventCount
        slate.put_va_le(Off::new(0x08), self.lp_guid)?; // lpGuid
        slate.put_va_le(Off::new(0x18), self.lp_event_table)?; // lpEventTable
        slate.put_va_le(Off::new(0x20), self.lpsz_name)?; // lpszName
        Some(())
    }
}

/// The bytes of a `ControlInfo` no field of [`ControlInfo`] reproduces, as
/// `(structure relative offset, length)` pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[(0x04, 4), (0x0C, 12), (0x1C, 4), (0x24, 4)];

/// The number of `ControlInfo` bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 16;

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
    use crate::vb::controlinfo::ControlInfo;

    fn control() -> ControlInfo {
        ControlInfo {
            file_offset: Off::new(0x6000),
            f_control_type: 0x0040,
            w_event_count: 0x0017,
            lp_guid: Va::new(0x0040_B008),
            lp_event_table: Va::new(0x0040_B018),
            lpsz_name: Va::new(0x0040_B020),
            name: "Command1".to_owned(),
        }
    }

    /// The 40 bytes that record would have been read out of. Every byte no
    /// field claims is `0xA5`, and that includes `0x04`, where the disputed
    /// layout puts `wEventCount`: an emitter that followed it would write
    /// there and leave `0x02` unwritten, and both would show.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x28];
        let mut put = |at: usize, src: &[u8]| raw[at..at + src.len()].copy_from_slice(src);
        put(0x00, &0x0040_u16.to_le_bytes());
        put(0x02, &0x0017_u16.to_le_bytes());
        put(0x08, &0x0040_B008_u32.to_le_bytes());
        put(0x18, &0x0040_B018_u32.to_le_bytes());
        put(0x20, &0x0040_B020_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_control_info_record_is_forty_bytes() {
        assert_eq!(ControlInfo::LEN, 0x28);
        assert_eq!(ControlInfo::LEN, 40);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&control(), &Region::new(&raw, Off::new(0x6000))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_resolved_name_is_never_written_back() {
        // A name is a reading of the bytes at lpszName, not a field of this
        // record. Two controls that differ only in the resolved name must lay
        // down the same bytes.
        let raw = bytes();
        let mut renamed = control();
        renamed.name = "SomethingElseEntirely".to_owned();
        let one = compare(&control(), &Region::new(&raw, Off::new(0))).unwrap();
        let two = compare(&renamed, &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(one, two);
    }

    #[test]
    fn the_reader_models_sixteen_of_the_forty_bytes() {
        let raw = bytes();
        let ledger = compare(&control(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 16);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 24);
        assert_eq!(MODELLED_BYTES + 24, ControlInfo::LEN);
    }

    #[test]
    fn the_unmodelled_ranges_are_the_ones_the_constant_names() {
        let raw = bytes();
        let ledger = compare(&control(), &Region::new(&raw, Off::new(0))).unwrap();
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
    fn an_event_count_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = control();
        wrong.w_event_count ^= 1;
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x6000))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x6002), 1)]);
        assert!(ledger.tiles());
    }
}
