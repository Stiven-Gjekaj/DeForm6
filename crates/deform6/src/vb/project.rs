//! `ProjectInfo`, which is the spine of the file, and the compilation mode.
//!
//! `VBHeader.lpProjectData` is the only route to `ProjectInfo`. It is a
//! virtual address, so it reaches bytes only through
//! [`PeImage::region_at_va`].
//!
//! Each structure is narrowed to the size the format gives, with
//! `Region::subregion`, before any field inside it is read. A truncated file
//! therefore fails at the window rather than three reads later, and no read
//! can run out of the structure into unrelated bytes.

use crate::error::Refusal;
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Va};

/// The size of the `ProjectInfo` structure.
///
/// `STRUCTURES.md` section 3 gives `0x23C` = 572 bytes, and four sources
/// agree.
const PROJECT_INFO_SIZE: u32 = 0x23C;

/// How the program was compiled.
///
/// `ProjectInfo.lpNativeCode` decides this and nothing else does. Non-zero is
/// native and zero is P-code. `STRUCTURES.md` section 3 marks the rule at
/// confidence `[C]`, and section 10.2 records that one prior tool branches on
/// the same field in six places.
///
/// # No program in this repository is P-code
///
/// Every one of the 44 vendored projects carries `CompilationType=0` in its
/// `.vbp`, which is native, and `lpNativeCode` is non-zero in all 44 of the
/// executables. `[VERIFIED: local, 44 of 44]`
///
/// [`CompileMode::PCode`] is therefore never produced by a real program in
/// this repository. One test produces it by zeroing the field in a copy of a
/// native program held in memory. That proves the branch is reachable and
/// that it reads the field it says it reads. It proves nothing about a real
/// P-code binary, whose method pointers reach a different structure that this
/// crate does not read. A P-code binary is needed before that branch can be
/// called tested, and `STRUCTURES.md` section 12 names
/// `TimoKunze/ExplorerTreeView-VB6` as a known source of one.
///
/// [`PeImage::region_at_va`]: crate::read::pe::PeImage::region_at_va
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompileMode {
    /// The project was compiled to machine code.
    Native,
    /// The project was compiled to P-code.
    PCode,
}

/// The head of `ProjectInfo`, which `STRUCTURES.md` section 3 describes.
///
/// Five fields are read. The 528 bytes at `0x24` are left alone: three
/// sources subdivide them differently, the arithmetic of all three readings
/// is identical, and nothing in this phase needs the compile-time path.
///
/// `dw_version` is read and kept and **nothing branches on it**.
/// `STRUCTURES.md` section 3 says plainly that it is a template version and
/// that it is not a discriminator between Visual Basic 5 and Visual Basic 6.
/// The version comes from the name of the imported runtime, which
/// [`crate::vb::runtime::runtime_of`] decides before this structure is
/// reached.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProjectInfo {
    /// The template version of the structure. Nothing branches on it.
    pub dw_version: u32,
    /// The address of the object table, which holds the project name.
    pub lp_object_table: Va,
    /// The native code address, which is the compilation mode discriminator.
    ///
    /// This is read as a plain `u32` and never as a [`Va`]. Only its zero or
    /// non-zero state is used, nothing dereferences it, and typing it as an
    /// address would invite a later reader to.
    pub lp_native_code: u32,
    /// The address of the `Declare` import table, which Phase 3 walks.
    pub lp_external_table: Va,
    /// The number of entries in that table.
    pub dw_external_count: u32,
}

impl ProjectInfo {
    /// Reads `ProjectInfo` at the address `VBHeader.lpProjectData` holds.
    ///
    /// The window is exactly [`PROJECT_INFO_SIZE`] bytes, taken before any
    /// field is read. That ordering is the whole mitigation: a file truncated
    /// in the middle of the structure is refused at the window, rather than
    /// yielding three plausible fields and then failing on the fourth.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when the address is in no section, and
    /// when the file holds fewer than 572 bytes there.
    ///
    /// The five field refusals below the window check cannot be reached,
    /// because the window is the exact size of the structure and every field
    /// lies inside it. They stay because `Region` has no infallible accessor
    /// and this function must not unwrap an `Option`. No test covers them and
    /// no test can.
    pub fn read(pe: &PeImage<'_>, lp_project_data: Va) -> Result<Self, Refusal> {
        let at = pe.region_at_va(lp_project_data).ok_or(Refusal::Damaged(
            "the project data pointer is in no section",
        ))?;
        let window = at
            .subregion(Off::new(0), PROJECT_INFO_SIZE)
            .ok_or(Refusal::Damaged(
                "the file ends inside the ProjectInfo structure",
            ))?;
        Ok(Self {
            dw_version: u32_at(&window, 0x00, "ProjectInfo holds no template version")?,
            lp_object_table: va_at(
                &window,
                0x04,
                "ProjectInfo holds no address for the object table",
            )?,
            lp_native_code: u32_at(&window, 0x20, "ProjectInfo holds no native code address")?,
            lp_external_table: va_at(
                &window,
                0x234,
                "ProjectInfo holds no address for the import table",
            )?,
            dw_external_count: u32_at(&window, 0x238, "ProjectInfo holds no import count")?,
        })
    }

    /// Gives the compilation mode.
    ///
    /// Read the doc comment on [`CompileMode`] before you trust
    /// [`CompileMode::PCode`]. No program in this repository produces it.
    #[must_use]
    pub const fn mode(&self) -> CompileMode {
        if self.lp_native_code == 0 {
            CompileMode::PCode
        } else {
            CompileMode::Native
        }
    }
}

/// Reads an unsigned 32-bit value out of a structure window.
fn u32_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<u32, Refusal> {
    window.u32_le(Off::new(at)).ok_or(Refusal::Damaged(what))
}

/// Reads a virtual address out of a structure window.
fn va_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<Va, Refusal> {
    window.va_le(Off::new(at)).ok_or(Refusal::Damaged(what))
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
    use super::{CompileMode, PROJECT_INFO_SIZE, ProjectInfo};
    use crate::error::Refusal;
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Va};
    use crate::vb::header::{VbHeader, header_region};

    /// The corpus program the whole phase is worked against.
    ///
    /// A test module under `src/` reaches a corpus file this way and no other
    /// way. The library names no file system type, and the grep that proves
    /// it does not know a `#[cfg(test)]` module from library code. Plan 01-08
    /// owns the sweep over all 44, in a file under `tests/`, which is a
    /// separate crate root.
    const MANDELBROT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
    ));

    /// Gives the address of `ProjectInfo` that the file itself holds.
    fn project_data_va(data: &[u8]) -> Va {
        let image = PeImage::parse(data).unwrap();
        let hdr = header_region(&image).unwrap();
        VbHeader::read(&hdr).unwrap().lp_project_data
    }

    /// Gives the absolute file offset of a field inside `ProjectInfo`.
    ///
    /// The offset is resolved through the same path the parser uses, that is
    /// `region_at_va` on the address the header holds, then `file_offset`.
    /// **Nothing searches for a byte pattern.** A search could hit the same
    /// bytes somewhere no pointer in the file names, and the fixture would
    /// then patch a place the parser never reads.
    fn project_info_field_offset(data: &[u8], field: u32) -> usize {
        let image = PeImage::parse(data).unwrap();
        let window = image.region_at_va(project_data_va(data)).unwrap();
        let at = window.file_offset(Off::new(field)).unwrap();
        usize::try_from(at.get()).unwrap()
    }

    /// Reads `ProjectInfo` out of a byte slice.
    fn project_info(data: &[u8]) -> Result<ProjectInfo, Refusal> {
        let image = PeImage::parse(data).unwrap();
        ProjectInfo::read(&image, project_data_va(data))
    }

    #[test]
    fn the_project_data_address_reaches_an_object_table_that_resolves() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let info = ProjectInfo::read(&image, project_data_va(MANDELBROT)).unwrap();
        assert!(!info.lp_object_table.is_null());
        let table = image.region_at_va(info.lp_object_table).unwrap();
        assert!(!table.is_empty());
    }

    #[test]
    fn the_corpus_file_is_native_because_its_native_code_address_is_not_zero() {
        let info = project_info(MANDELBROT).unwrap();
        assert_ne!(info.lp_native_code, 0);
        assert_eq!(info.mode(), CompileMode::Native);
    }

    /// Copies the corpus bytes and writes four zeros over `lpNativeCode`.
    ///
    /// The offset comes from the parser's own resolution of the address the
    /// file holds. Nothing is written to disk.
    fn with_zeroed_native_code() -> Vec<u8> {
        let at = project_info_field_offset(MANDELBROT, 0x20);
        let mut out = MANDELBROT.to_vec();
        assert_ne!(
            &out[at..at + 4],
            &[0, 0, 0, 0],
            "the fixture writes zeros over a field that is already zero, so it proves nothing"
        );
        out[at..at + 4].copy_from_slice(&[0; 4]);
        out
    }

    /// The name of this test states the limit of what it proves.
    ///
    /// It zeroes `lpNativeCode` in a copy of a native program and watches the
    /// mode read `PCode`. That shows the branch is reachable and that it
    /// reads the field it claims to read.
    ///
    /// **It is not evidence that DeForm6 reads a P-code program.** All 44
    /// vendored projects carry `CompilationType=0`, which is native, so no
    /// file in this repository exercises the branch as a real program would.
    /// The doc comment on [`CompileMode`] carries the measurement and names a
    /// known source of a P-code binary.
    #[test]
    fn zeroing_lp_native_code_reports_p_code_but_no_corpus_program_is_p_code() {
        let bytes = with_zeroed_native_code();
        let info = project_info(&bytes).unwrap();
        assert_eq!(info.lp_native_code, 0);
        assert_eq!(info.mode(), CompileMode::PCode);
        // The rest of the structure still reads, so the fixture changed the
        // one field and not the shape of the file.
        assert_eq!(
            info.lp_object_table,
            project_info(MANDELBROT).unwrap().lp_object_table
        );
    }

    #[test]
    fn a_project_data_pointer_in_no_section_is_damaged() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        // An address inside the image base but above every section.
        let nowhere = Va::new(image.image_base() + 0x00F0_0000);
        assert!(image.region_at_va(nowhere).is_none());
        assert_eq!(
            ProjectInfo::read(&image, nowhere).unwrap_err(),
            Refusal::Damaged("the project data pointer is in no section")
        );
    }

    /// A file cut off inside `ProjectInfo` is refused at the window.
    ///
    /// The cut lands after the first fields and before the last two, so a
    /// parser that read field by field would report a template version, an
    /// object table address and a native code address, and would fail only at
    /// `0x234`. That is the partial read the window exists to stop.
    #[test]
    fn a_file_that_ends_inside_project_info_is_damaged_and_is_not_read_in_part() {
        let start = project_info_field_offset(MANDELBROT, 0);
        let cut = start + 0x100;
        assert!(cut < MANDELBROT.len());
        assert!(cut < start + usize::try_from(PROJECT_INFO_SIZE).unwrap());
        let bytes = MANDELBROT[..cut].to_vec();

        // The fields before the cut are readable, so the refusal below comes
        // from the window and not from a short file in general.
        let image = PeImage::parse(&bytes).unwrap();
        let window = image.region_at_va(project_data_va(&bytes)).unwrap();
        assert!(window.u32_le(Off::new(0x20)).is_some());
        assert!(window.u32_le(Off::new(0x234)).is_none());

        assert_eq!(
            ProjectInfo::read(&image, project_data_va(&bytes)).unwrap_err(),
            Refusal::Damaged("the file ends inside the ProjectInfo structure")
        );
    }
}
