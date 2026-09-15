//! Writing [`VbHeader`] back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 2
//!
//! Not one of them is copied from `vb::header`, which is the reader this
//! grades. See the module doc comment on `crate::fidelity` for why a shared
//! constant would make the whole measurement worthless.
//!
//! # What this reader models, and what it leaves
//!
//! Fourteen fields, 50 of the 104 bytes. The 54 bytes no field claims are not
//! a mystery: section 2 names every one of them at confidence `[C]`.
//!
//! | Bytes | Length | Fields section 2 names there |
//! |---|---|---|
//! | `0x06` | 38 | `szLangDll`, `szSecLangDll`, `wRuntimeRevision`, `dwLCID`, `dwSecLCID` |
//! | `0x3C` | 8 | `dwThreadFlags`, `dwThreadCount` |
//! | `0x48` | 4 | `dwThunkCount` |
//! | `0x54` | 4 | `lpComRegisterData` |
//!
//! So the header's map is a list of work not yet done, not a list of unknowns.
//! `dwThreadFlags` and `dwThreadCount` carry the `.vbp` `ThreadingModel` and
//! `ThreadPerObject` keys, which the project writer states nothing about
//! today.
//!
//! # The four strings are not part of this record
//!
//! [`VbHeader`] also carries `exe_name`, `title`, `help_file` and
//! `project_name`. Those live in a pool **after** the record, at the offsets
//! the four `o_*` fields hold, and they are not inside `LEN`. This emitter
//! must never write past `0x68`. Grading the pool needs a length that comes
//! from the file, which [`Emit`] deliberately cannot express.

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::header::VbHeader;

impl Emit for VbHeader {
    /// `STRUCTURES.md` section 2: `0x68` = 104 bytes, and all five sources
    /// agree.
    const LEN: u32 = 0x68;
    const STRUCTURE: &'static str = "VBHeader";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_bytes(Off::new(0x00), &self.signature)?; // szVbMagic
        slate.put_u16_le(Off::new(0x04), self.runtime_build)?; // wRuntimeBuild
        slate.put_va_le(Off::new(0x2C), self.lp_sub_main)?; // lpSubMain
        slate.put_va_le(Off::new(0x30), self.lp_project_data)?; // lpProjectData
        slate.put_u32_le(Off::new(0x34), self.f_mdl_int_ctls)?; // fMdlIntCtls
        slate.put_u32_le(Off::new(0x38), self.f_mdl_int_ctls2)?; // fMdlIntCtls2
        slate.put_u16_le(Off::new(0x44), self.w_form_count)?; // wFormCount
        slate.put_u16_le(Off::new(0x46), self.w_external_count)?; // wExternalCount
        slate.put_va_le(Off::new(0x4C), self.lp_gui_table)?; // lpGuiTable
        slate.put_va_le(Off::new(0x50), self.lp_external_table)?; // lpExternalTable
        slate.put_off_le(Off::new(0x58), self.o_project_exe_name)?; // oProjectExeName
        slate.put_off_le(Off::new(0x5C), self.o_project_title)?; // oProjectTitle
        slate.put_off_le(Off::new(0x60), self.o_help_file)?; // oHelpFile
        slate.put_off_le(Off::new(0x64), self.o_project_name)?; // oProjectName
        Some(())
    }
}

/// The bytes of the header no field of [`VbHeader`] reproduces, as
/// `(structure relative offset, length)` pairs.
///
/// Stated here so a test can pin it and so a reader can find it without
/// counting the gaps in `emit` by hand. It is derived from `emit`, never the
/// other way round: the corpus test asserts that the grading agrees with this
/// list, so a field added to `emit` without a change here fails.
pub const UNMODELLED: &[(u32, u32)] = &[(0x06, 38), (0x3C, 8), (0x48, 4), (0x54, 4)];

/// The number of header bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 50;

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
    use crate::vb::header::VbHeader;

    /// Builds a header whose every modelled field holds a value no other
    /// field holds, so a transposed pair of offsets cannot pass.
    fn header() -> VbHeader {
        VbHeader {
            signature: *b"VB5!",
            runtime_build: 0x231C,
            lp_sub_main: Va::new(0x0040_1000),
            lp_project_data: Va::new(0x0040_2000),
            f_mdl_int_ctls: 0x0000_0003,
            f_mdl_int_ctls2: 0x0000_0004,
            w_form_count: 0x0005,
            w_external_count: 0x0006,
            lp_gui_table: Va::new(0x0040_3000),
            lp_external_table: Va::new(0x0040_4000),
            o_project_exe_name: Off::new(0x0078),
            o_project_title: Off::new(0x0080),
            o_help_file: Off::new(0x0088),
            o_project_name: Off::new(0x0090),
            exe_name: String::new(),
            title: String::new(),
            help_file: String::new(),
            project_name: String::new(),
        }
    }

    /// Builds the 104 bytes that header would have been read out of, with
    /// every unmodelled byte set to a value the emitter could not produce.
    ///
    /// The `0xA5` fill is what proves the four gaps are graded on coverage
    /// and not on equality: if the grading compared bytes, these would come
    /// back as differences.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x68];
        let mut put = |at: usize, src: &[u8]| raw[at..at + src.len()].copy_from_slice(src);
        put(0x00, b"VB5!");
        put(0x04, &0x231C_u16.to_le_bytes());
        put(0x2C, &0x0040_1000_u32.to_le_bytes());
        put(0x30, &0x0040_2000_u32.to_le_bytes());
        put(0x34, &3_u32.to_le_bytes());
        put(0x38, &4_u32.to_le_bytes());
        put(0x44, &5_u16.to_le_bytes());
        put(0x46, &6_u16.to_le_bytes());
        put(0x4C, &0x0040_3000_u32.to_le_bytes());
        put(0x50, &0x0040_4000_u32.to_le_bytes());
        put(0x58, &0x78_u32.to_le_bytes());
        put(0x5C, &0x80_u32.to_le_bytes());
        put(0x60, &0x88_u32.to_le_bytes());
        put(0x64, &0x90_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_header_record_is_one_hundred_and_four_bytes() {
        assert_eq!(VbHeader::LEN, 0x68);
        assert_eq!(VbHeader::LEN, 104);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&header(), &Region::new(&raw, Off::new(0x400))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_fifty_of_the_one_hundred_and_four_header_bytes() {
        let raw = bytes();
        let ledger = compare(&header(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 50);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 54);
        assert_eq!(MODELLED_BYTES + 54, VbHeader::LEN);
    }

    #[test]
    fn the_four_unmodelled_ranges_are_the_ones_the_constant_names() {
        let raw = bytes();
        let ledger = compare(&header(), &Region::new(&raw, Off::new(0))).unwrap();
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
    fn a_form_count_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        // The check that the grading has teeth. Every field here is verbatim,
        // so nothing in the corpus can make this fire, and a measurement that
        // cannot fail is not a measurement.
        let raw = bytes();
        let mut wrong = header();
        wrong.w_form_count ^= 1;
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x400))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x444), 1)]);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 54);
        assert!(ledger.tiles());
    }

    #[test]
    fn the_emitter_never_writes_past_the_end_of_the_record() {
        // The four strings live in the pool after the record. An emitter that
        // reached them would need a length taken from the file.
        let slate = crate::fidelity::lay(&header(), Off::new(0)).unwrap();
        assert_eq!(slate.len(), VbHeader::LEN);
        assert_eq!(slate.covered_len(), MODELLED_BYTES);
    }
}
