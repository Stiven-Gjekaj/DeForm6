//! The Visual Basic structures inside the executable.
//!
//! [`inspect`] is the whole public reading path. It takes a byte slice and it
//! returns a value. It opens no file and it writes no file, which is what
//! makes the Phase 5 fuzz target the real public API rather than an internal
//! one, and which leaves the file system to the command line crate.

pub mod header;
pub mod project;
pub mod runtime;

pub use crate::error::Refusal;

use crate::read::pe::PeImage;
use crate::read::region::Off;
use header::{VbHeader, header_region};
use project::{ObjectTableHead, ProjectInfo};
use runtime::{Runtime, runtime_of};

/// What DeForm6 read out of one executable.
///
/// # Two fields exist because a one variant enum carries no evidence
///
/// [`Runtime`] has one variant, so `runtime` alone proves nothing: a caller
/// that compared it against [`Runtime::Vb6`] would learn only that `inspect`
/// returned `Ok`. ROADMAP success criterion 2 requires the sweep over the
/// corpus to observe that each file names its runtime and reaches the Visual
/// Basic signature, and neither fact can be observed unless it travels out of
/// the parse.
///
/// `runtime_dll` therefore carries the import entry that matched, verbatim
/// from the file, and `signature` carries the four bytes that were read at
/// the head of the Visual Basic header. Both are also what the command line
/// prints on its Runtime and Header lines, so those two lines report values
/// that were read from the file instead of literals written into the printer.
///
/// # `exe_name` and `help_file` are carried and are not printed
///
/// Neither is in the eight line output shape this phase locks. They are
/// carried because Phase 4 writes `exe_name` plus the literal `.exe` as the
/// `.vbp` `ExeName32` key, the extension not being in the file, and because
/// plan 01-08's sweep asserts that all four header strings resolve on all 44
/// corpus programs.
///
/// # There is no second output shape
///
/// This is the one value the library returns. There is no serialiser and no
/// machine readable form yet. Phase 4 introduces the confidence report, which
/// is the project's machine readable surface, and one schema introduced once
/// is cheaper than two kept in step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// The real length of the byte slice the caller gave the library.
    pub file_len: u32,
    /// The number of sections the image declares.
    pub section_count: u16,
    /// The Visual Basic runtime the image imports.
    pub runtime: Runtime,
    /// The name of the imported runtime, verbatim from the import directory.
    pub runtime_dll: String,
    /// The four bytes at the head of the Visual Basic header.
    pub signature: [u8; 4],
    /// The file offset of those four bytes.
    pub header_offset: Off,
    /// The build number of the runtime the file was linked against.
    pub runtime_build: u16,
    /// The project name, which is the `.vbp` `Name` value.
    ///
    /// This comes from the object table, not from the header. The two agree
    /// on the corpus, and a test in `vb/project.rs` holds them to it.
    pub project_name: String,
    /// The project title, which is the `.vbp` `Title` value.
    pub title: String,
    /// The executable name with the extension removed.
    pub exe_name: String,
    /// The help file name, which is the `.vbp` `HelpFile` value.
    pub help_file: String,
    /// True when the project was compiled to native code.
    ///
    /// Read the doc comment on [`project::CompileMode`] before trusting a
    /// `false` here. No program in this repository produces one.
    pub native: bool,
    /// The number of objects the project declares.
    ///
    /// This is `wTotalObjects`. Read the doc comment on
    /// [`ObjectTableHead`] for the measurement that decided which of the two
    /// count fields in the object table answers this question.
    pub object_count: u16,
}

/// Reads one executable and reports what it holds.
///
/// # The order of the steps is load bearing
///
/// The runtime is decided **second**, before any Visual Basic structure is
/// read. A Visual Basic 5 file lays its header out differently after `0x30`,
/// so a run that read the header first would report a damaged file for one
/// that is merely the wrong version, and the person holding that file would
/// be told the wrong thing about it. DET-04 asks for the file to be refused
/// by name, and this ordering is what delivers it. A test destroys the
/// signature of a file whose import name says Visual Basic 5 and still
/// expects the Visual Basic 5 refusal.
///
/// The remaining steps follow the pointer chain, and each one is reached only
/// through the address the step before it read.
///
/// # Errors
///
/// Returns [`Refusal::NotPe`], [`Refusal::NotI386`] or [`Refusal::NotPe32`]
/// when the bytes are not a 32 bit i386 portable executable,
/// [`Refusal::NoVbRuntime`], [`Refusal::IsVb5`] or [`Refusal::IsVb4`] when the
/// image names no Visual Basic 6 runtime, and [`Refusal::Damaged`] when a
/// Visual Basic 6 structure does not resolve.
pub fn inspect(data: &[u8]) -> Result<Report, Refusal> {
    let pe = PeImage::parse(data)?;
    let (runtime, runtime_dll) = runtime_of(&pe)?;

    let hdr = header_region(&pe)?;
    let header_offset = hdr
        .file_offset(Off::new(0))
        .ok_or(Refusal::Damaged("the VB header window has no file offset"))?;
    let header = VbHeader::read(&hdr)?;

    let info = ProjectInfo::read(&pe, header.lp_project_data)?;
    let table = ObjectTableHead::read(&pe, info.lp_object_table)?;
    // Read before the name moves out of `table`.
    let object_count = table.object_count();

    Ok(Report {
        // The saturating conversions over-report a slice larger than 4 GiB
        // and a section table longer than 65535 entries. Neither is reachable
        // through a 32 bit portable executable, and neither value is used as
        // a bound for a read.
        file_len: u32::try_from(data.len()).unwrap_or(u32::MAX),
        section_count: u16::try_from(pe.sections().len()).unwrap_or(u16::MAX),
        runtime,
        runtime_dll,
        signature: header.signature,
        header_offset,
        runtime_build: header.runtime_build,
        project_name: table.project_name,
        title: header.title,
        exe_name: header.exe_name,
        help_file: header.help_file,
        native: info.mode() == project::CompileMode::Native,
        object_count,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{Refusal, Report, inspect};
    use crate::read::pe::PeImage;
    use crate::read::region::Off;

    /// The corpus program this module reads.
    ///
    /// A test inside `src/` reaches a corpus file this way and never through
    /// a path. Plan 01-08 owns the sweep over all 44, in a file under
    /// `tests/`, which is a separate crate root.
    const MANDELBROT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
    ));

    /// Gives a copy of `data` whose first imported DLL name is `replacement`.
    ///
    /// The offset comes from `PeImage::dll_name_sites`, which resolves the
    /// address in the import descriptor. It is never a byte search.
    ///
    /// This is a second copy of the helper `vb/runtime.rs` holds, written on
    /// purpose. The two tests must be able to fail independently, and a
    /// shared fixture module would make one change break both.
    fn with_the_runtime_named(data: &[u8], replacement: &[u8]) -> Vec<u8> {
        let image = PeImage::parse(data).unwrap();
        let sites = image.dll_name_sites().unwrap();
        let (offset, len) = sites[0];
        let at = usize::try_from(offset.get()).unwrap();
        let len = usize::try_from(len).unwrap();
        assert_eq!(
            replacement.len(),
            len,
            "the replacement must be as long as the name it covers"
        );
        let mut out = data.to_vec();
        out[at..at + len].copy_from_slice(replacement);
        out
    }

    /// Gives the file offset of the Visual Basic header the hard way.
    ///
    /// This walks the entry stub with the `read` layer only, so it still
    /// answers when the magic is destroyed and `header_region` refuses, and
    /// so it does not ask `inspect` where its own header is.
    fn header_offset_the_hard_way(data: &[u8]) -> usize {
        let image = PeImage::parse(data).unwrap();
        let entry = image.region_at(image.entry_rva()).unwrap();
        let va = entry.va_le(Off::new(1)).unwrap();
        usize::try_from(image.va_to_off(va).unwrap().get()).unwrap()
    }

    /// Gives a copy of `data` whose Visual Basic signature is destroyed.
    fn with_the_signature_destroyed(data: &[u8]) -> Vec<u8> {
        let at = header_offset_the_hard_way(data);
        let mut out = data.to_vec();
        assert_eq!(
            &out[at..at + 4],
            b"VB5!",
            "the fixture patches the wrong place"
        );
        out[at..at + 4].copy_from_slice(b"\0\0\0\0");
        out
    }

    fn mandelbrot_report() -> Report {
        inspect(MANDELBROT).unwrap()
    }

    /// The `.vbp` beside the executable declares these four values.
    ///
    /// ```text
    /// Title="Mandelbrot Fractal Demo"
    /// ExeName32="Mandelbrot.exe"
    /// Name="Mandelbrot_Fractal_Demo"
    /// CompilationType=0
    /// ```
    ///
    /// The object count is one, because the `.vbp` declares one `Form` and
    /// nothing else.
    #[test]
    fn the_corpus_file_reports_what_its_project_file_declares() {
        let report = mandelbrot_report();
        assert_eq!(report.project_name, "Mandelbrot_Fractal_Demo");
        assert_eq!(report.title, "Mandelbrot Fractal Demo");
        assert_eq!(report.exe_name, "Mandelbrot");
        assert!(report.native);
        assert_eq!(report.object_count, 1);
    }

    /// The two evidence fields hold what the file holds.
    ///
    /// Both are compared against bytes read out of the file by a second
    /// route, and not against a literal written here. A composer that filled
    /// either field from a constant in this crate would pass a comparison
    /// against a literal and would fail this one.
    #[test]
    fn the_runtime_name_and_the_signature_are_read_out_of_the_file() {
        let report = mandelbrot_report();

        // The runtime name, taken from the offset the import directory
        // itself names.
        let image = PeImage::parse(MANDELBROT).unwrap();
        let (offset, len) = image.dll_name_sites().unwrap()[0];
        let at = usize::try_from(offset.get()).unwrap();
        let len = usize::try_from(len).unwrap();
        assert_eq!(report.runtime_dll.as_bytes(), &MANDELBROT[at..at + len]);

        // The signature, taken from the header offset walked the hard way.
        let head = header_offset_the_hard_way(MANDELBROT);
        assert_eq!(report.signature, MANDELBROT[head..head + 4]);
        assert_eq!(report.header_offset, Off::new(u32::try_from(head).unwrap()));
    }

    #[test]
    fn an_empty_slice_is_not_a_portable_executable() {
        assert_eq!(inspect(&[]), Err(Refusal::NotPe));
    }

    /// The runtime is decided before any Visual Basic structure is read.
    ///
    /// The fixture names the Visual Basic 5 runtime **and** destroys the
    /// signature, so a run that read the header first would find no `VB5!`
    /// and would report the file as damaged. The user would then be told
    /// their file is broken when it is merely the wrong version of Visual
    /// Basic, which is the wrong thing to tell them.
    #[test]
    fn a_visual_basic_5_file_with_no_signature_is_refused_by_name_and_not_as_damaged() {
        let named = with_the_runtime_named(MANDELBROT, b"MSVBVM50.DLL");
        let bytes = with_the_signature_destroyed(&named);
        // The signature really is gone, so the ordering is what decides.
        let head = header_offset_the_hard_way(&bytes);
        assert_ne!(&bytes[head..head + 4], b"VB5!");
        assert_eq!(inspect(&bytes), Err(Refusal::IsVb5));
    }

    /// `assert_eq!` on a `Result<Report, Refusal>` needs both sides to
    /// compare, so `Report` must derive `PartialEq`, `Eq` and `Debug`.
    ///
    /// A missing derive on the success side of the `Result` is a compile
    /// error that costs a cycle, and RESEARCH.md section 7.2 records it
    /// happening. This test is the instrument that keeps the derives.
    #[test]
    fn a_result_of_a_report_compares_and_prints() {
        let report = mandelbrot_report();
        let same: Result<Report, Refusal> = Ok(report.clone());
        assert_eq!(same, Ok(report.clone()));
        assert_ne!(same, Err(Refusal::NotPe));
        assert!(format!("{report:?}").contains("Mandelbrot"));
    }

    /// The report describes the slice the caller handed in.
    #[test]
    fn the_report_measures_the_slice_it_was_given() {
        let report = mandelbrot_report();
        assert_eq!(report.file_len, u32::try_from(MANDELBROT.len()).unwrap());
        let image = PeImage::parse(MANDELBROT).unwrap();
        assert_eq!(
            report.section_count,
            u16::try_from(image.sections().len()).unwrap()
        );
    }
}
