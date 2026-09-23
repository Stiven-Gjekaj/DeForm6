//! Writing [`ProjectInfo`] back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 3
//!
//! Not one of them is copied from `vb::project`, which is the reader this
//! grades. See the module doc comment on `crate::fidelity` for why.
//!
//! # What this reader models, and what it leaves
//!
//! Five fields. The bytes no field claims are named by section 3:
//!
//! | Bytes | Fields section 3 names there |
//! |---|---|
//! | `0x08` | `dwNull`, `lpCodeStart`, `lpCodeEnd`, `dwDataSize`, `lpThreadSpace`, `lpVbaSeh` |
//! | `0x24` | `szPathInformation`, 528 bytes |
//!
//! `szPathInformation` is the largest structure in this file that no field
//! claims. Section 3 marks its extent `[C]` and its subdivision `[D]`: one
//! source splits it into two flag words and a path, another reads it whole.
//! Both readings cover the same bytes, so the region stays opaque here.
//!
//! # `lpNativeCode` is written as a number
//!
//! The reader keeps `lp_native_code` as a plain `u32` and not as a `Va`,
//! because its only job is to say whether the program is native code or
//! P-code: zero means P-code. This emitter writes the same four bytes either
//! way.

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::project::ProjectInfo;

impl Emit for ProjectInfo {
    /// `STRUCTURES.md` section 3: `0x23C` = 572 bytes.
    const LEN: u32 = 0x23C;
    const STRUCTURE: &'static str = "ProjectInfo";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_u32_le(Off::new(0x000), self.dw_version)?; // dwVersion
        slate.put_va_le(Off::new(0x004), self.lp_object_table)?; // lpObjectTable
        slate.put_u32_le(Off::new(0x020), self.lp_native_code)?; // lpNativeCode
        slate.put_va_le(Off::new(0x234), self.lp_external_table)?; // lpExternalTable
        slate.put_u32_le(Off::new(0x238), self.dw_external_count)?; // dwExternalCount
        Some(())
    }
}

/// The bytes of `ProjectInfo` no field of [`ProjectInfo`] reproduces, as
/// `(structure relative offset, length)` pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[(0x08, 24), (0x24, 528)];

/// The number of `ProjectInfo` bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 20;

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
    use crate::vb::project::ProjectInfo;

    /// A value whose every field holds a number no other field holds, so a
    /// transposed pair of offsets cannot pass.
    fn project() -> ProjectInfo {
        ProjectInfo {
            file_offset: Off::new(0x1000),
            rva: None,
            dw_version: 0x0000_01F4,
            lp_object_table: Va::new(0x0040_1111),
            lp_native_code: 0x0040_2222,
            lp_external_table: Va::new(0x0040_3333),
            dw_external_count: 0x0000_0044,
        }
    }

    /// The 572 bytes that value would have been read out of. Every byte no
    /// field claims is `0xA5`, which `emit` cannot produce, so a grading that
    /// compared bytes rather than coverage would call them differences.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x23C];
        let mut put = |at: usize, src: &[u8]| raw[at..at + src.len()].copy_from_slice(src);
        put(0x000, &0x0000_01F4_u32.to_le_bytes());
        put(0x004, &0x0040_1111_u32.to_le_bytes());
        put(0x020, &0x0040_2222_u32.to_le_bytes());
        put(0x234, &0x0040_3333_u32.to_le_bytes());
        put(0x238, &0x0000_0044_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_project_info_record_is_five_hundred_and_seventy_two_bytes() {
        assert_eq!(ProjectInfo::LEN, 0x23C);
        assert_eq!(ProjectInfo::LEN, 572);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&project(), &Region::new(&raw, Off::new(0x1000))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_twenty_of_the_five_hundred_and_seventy_two_bytes() {
        let raw = bytes();
        let ledger = compare(&project(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 20);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 552);
        assert_eq!(MODELLED_BYTES + 552, ProjectInfo::LEN);
    }

    #[test]
    fn the_unmodelled_ranges_are_the_ones_the_constant_names() {
        let raw = bytes();
        let ledger = compare(&project(), &Region::new(&raw, Off::new(0))).unwrap();
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
    fn a_native_code_address_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = project();
        wrong.lp_native_code ^= 1;
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x1000))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x1020), 1)]);
        assert!(ledger.tiles());
    }
}
