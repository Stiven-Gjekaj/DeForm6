//! `ProjectInfo`, which is the spine of the file, and the compilation mode.
//!
//! `VBHeader.lpProjectData` is the only route to `ProjectInfo`. It is a
//! virtual address, so it reaches bytes only through
//! [`PeImage::region_at_va`].
//!
//! `ProjectInfo.lpObjectTable` is the only route to the object table, and it
//! is a virtual address as well.
//!
//! Each structure is narrowed to the size the format gives, with
//! `Region::subregion`, before any field inside it is read. A truncated file
//! therefore fails at the window rather than three reads later, and no read
//! can run out of the structure into unrelated bytes.
//!
//! # This module reads the head of the object table and stops there
//!
//! Phase 1 reaches the object table because the project name is stored in it,
//! so the number of objects is free and honest here and the number is
//! reported without naming any object. Walking the objects themselves is
//! Phase 2, plan 02-01. **Do not extend [`ObjectTableHead::read`] to follow
//! the address at `0x30`.** Nothing here sizes an allocation from a field in
//! the file, and SAF-04 requires the count to be checked against the real
//! file length before anything in Phase 2 does.

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Rva, Va};

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

/// The size of the `ObjectTable` structure.
///
/// `STRUCTURES.md` section 4 gives `0x54` = 84 bytes, and four sources agree.
const OBJECT_TABLE_SIZE: u32 = 0x54;

/// The bound on the project name string.
///
/// `Region::cstr` needs a mandatory maximum, so a file with no NUL byte after
/// the name cannot make the scan run to the end of the section.
const NAME_MAX: u32 = 0x104;

/// The head of the object table, which `STRUCTURES.md` section 4 describes.
///
/// Three fields are read: the two counts and the address of the project name.
///
/// # The two counts are not the same quantity
///
/// `STRUCTURES.md` section 4 calls `wCompiledObjects` "the loop bound for the
/// object array" and says the two counts are equal after a clean compile. It
/// then recommends, at confidence `[L]`, reading the count from
/// `wCompiledObjects`. **The corpus does not support that.**
///
/// A script read both fields from all 44 corpus executables and compared each
/// against the number of objects the matching `.vbp` declares, selected by
/// its `ExeName32` key.
///
/// | Field | Equals the number of objects the `.vbp` declares |
/// |---|---|
/// | `wTotalObjects` at `0x2A` | 44 of 44 |
/// | `wCompiledObjects` at `0x2C` | 29 of 44 |
///
/// `[VERIFIED: local, 44 of 44]` In the other 15 files `wCompiledObjects` is
/// larger, and it is larger by the amount that rounds the array up: a project
/// with 1, 2 or 3 objects reports 4, and a project with 5 reports 8. The
/// entries of the array past `wTotalObjects` hold a null pointer or a value
/// that resolves to nothing. So `wCompiledObjects` is the **capacity** of the
/// object array and `wTotalObjects` is the **number of objects**.
///
/// [`ObjectTableHead::object_count`] therefore gives `wTotalObjects`. Taking
/// the compiled count instead would print 4 for a project that declares 1,
/// for a third of the corpus, and `AGENTS.md` requires the number that can be
/// proved against the source the executable was built from.
///
/// Both fields are kept, because Phase 2 needs both: the count says how many
/// objects to read, and the capacity is the bound that the count must not
/// exceed.
#[derive(Clone, Debug)]
pub struct ObjectTableHead {
    /// The number of objects the project declares.
    pub w_total_objects: u16,
    /// The capacity of the object array. See the doc comment on this struct.
    pub w_compiled_objects: u16,
    /// The address of the project name string.
    ///
    /// This is a **virtual address**, which `STRUCTURES.md` section 4 marks
    /// at confidence `[C]`. It is a different kind of pointer from the four
    /// header relative offsets [`crate::vb::header::VbHeader`] carries, and
    /// the type is what keeps the two apart.
    pub lpsz_project_name: Va,
    /// The project name, which is the `.vbp` `Name` value.
    pub project_name: String,
    defects: Vec<Defect>,
}

impl ObjectTableHead {
    /// Reads the head of the object table.
    ///
    /// The window is exactly [`OBJECT_TABLE_SIZE`] bytes, taken before any
    /// field is read, for the reason [`ProjectInfo::read`] gives.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when the object table address is in no
    /// section, when the file ends inside the structure, when the project
    /// name address is in no section, and when no NUL byte follows the name
    /// within [`NAME_MAX`] bytes.
    ///
    /// A disagreement between the two counts is **not** an error. It is a
    /// [`DefectKind::CountMismatch`] at [`crate::error::Severity::Recoverable`]
    /// on [`ObjectTableHead::defects`], and the read continues. The count is
    /// one number in a report, and a wrong number is not a reason to refuse a
    /// file whose whole spine resolved.
    pub fn read(pe: &PeImage<'_>, lp_object_table: Va) -> Result<Self, Refusal> {
        let at = pe.region_at_va(lp_object_table).ok_or(Refusal::Damaged(
            "the object table pointer is in no section",
        ))?;
        let window = at
            .subregion(Off::new(0), OBJECT_TABLE_SIZE)
            .ok_or(Refusal::Damaged(
                "the file ends inside the object table structure",
            ))?;

        // The `dw` prefix that one prior source gives these two fields is a
        // typo. They are two bytes apart, so they are words, and four other
        // sources read them as `u16`.
        let w_total_objects = u16_at(&window, 0x2A, "the object table holds no object count")?;
        let w_compiled_objects = u16_at(
            &window,
            0x2C,
            "the object table holds no object array capacity",
        )?;
        let lpsz_project_name = va_at(
            &window,
            0x40,
            "the object table holds no address for the project name",
        )?;

        let name = pe.region_at_va(lpsz_project_name).ok_or(Refusal::Damaged(
            "the project name pointer is in no section",
        ))?;
        let bytes = name
            .cstr(Off::new(0), NAME_MAX)
            .ok_or(Refusal::Damaged("the project name is not a bounded string"))?;
        // Each byte becomes its Latin-1 code point, which is the rule
        // `vb/header.rs` uses for the four header strings.
        // `String::from_utf8_lossy` is wrong here: a byte in 0x80 to 0xFF
        // would become the replacement character and the name would be lost.
        let project_name = bytes.iter().copied().map(char::from).collect();

        let defects = count_defects(
            &window,
            lp_object_table,
            pe.image_base(),
            w_total_objects,
            w_compiled_objects,
        );

        Ok(Self {
            w_total_objects,
            w_compiled_objects,
            lpsz_project_name,
            project_name,
            defects,
        })
    }

    /// Gives the number of objects the project declares.
    ///
    /// This is `wTotalObjects`. Read the doc comment on
    /// [`ObjectTableHead`] for the measurement that decided which of the two
    /// count fields answers this question.
    #[must_use]
    pub const fn object_count(&self) -> u16 {
        self.w_total_objects
    }

    /// Gives the defects the read found, which is the count disagreement.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }
}

/// Reports a capacity that cannot hold the objects the table declares.
///
/// The test is `capacity < count` and it is not `capacity != count`. A
/// capacity larger than the count is what a clean compile produces in 15 of
/// the 44 corpus files, so reporting inequality would mark a third of the
/// corpus damaged. A capacity **below** the count is a real disagreement:
/// the array does not have room for the objects the same structure declares,
/// so one of the two numbers is wrong.
fn count_defects(
    window: &Region<'_>,
    lp_object_table: Va,
    image_base: u32,
    w_total_objects: u16,
    w_compiled_objects: u16,
) -> Vec<Defect> {
    if w_compiled_objects >= w_total_objects {
        return Vec::new();
    }
    // The window exists, so this sum lies inside it. `Region` has no
    // infallible accessor, so the fallback is written out. It names offset 0,
    // which is visibly not the site of a field and cannot be mistaken for one.
    let offset = window.file_offset(Off::new(0x2C)).map_or(0, Off::get);
    vec![Defect {
        site: Site {
            offset,
            rva: lp_object_table
                .to_rva(image_base)
                .and_then(|rva| rva.checked_add(0x2C))
                .map(Rva::get),
            structure: "ObjectTable",
            field: "wCompiledObjects",
        },
        kind: DefectKind::CountMismatch {
            offset,
            count: u32::from(w_compiled_objects),
            expected: u32::from(w_total_objects),
            other_field: "wTotalObjects",
        },
    }]
}

/// Reads an unsigned 16-bit value out of a structure window.
fn u16_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<u16, Refusal> {
    window.u16_le(Off::new(at)).ok_or(Refusal::Damaged(what))
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
    use super::{CompileMode, ObjectTableHead, PROJECT_INFO_SIZE, ProjectInfo};
    use crate::error::Refusal;
    use crate::error::{DefectKind, Severity};
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

    /// Reads the head of the object table out of a byte slice.
    fn object_table(data: &[u8]) -> Result<ObjectTableHead, Refusal> {
        let image = PeImage::parse(data).unwrap();
        let info = ProjectInfo::read(&image, project_data_va(data)).unwrap();
        ObjectTableHead::read(&image, info.lp_object_table)
    }

    /// Gives the absolute file offset of a field inside the object table.
    ///
    /// The route is the parser's own: the address the header holds, then the
    /// address `ProjectInfo` holds, then `file_offset`. Nothing searches for
    /// a byte pattern.
    fn object_table_field_offset(data: &[u8], field: u32) -> usize {
        let image = PeImage::parse(data).unwrap();
        let info = ProjectInfo::read(&image, project_data_va(data)).unwrap();
        let window = image.region_at_va(info.lp_object_table).unwrap();
        let at = window.file_offset(Off::new(field)).unwrap();
        usize::try_from(at.get()).unwrap()
    }

    /// Copies the corpus bytes and writes a `u16` into the object table.
    fn with_object_table_u16(field: u32, value: u16) -> Vec<u8> {
        let at = object_table_field_offset(MANDELBROT, field);
        let mut out = MANDELBROT.to_vec();
        assert_ne!(
            out[at..at + 2],
            value.to_le_bytes(),
            "the fixture writes the value the field already holds, so it proves nothing"
        );
        out[at..at + 2].copy_from_slice(&value.to_le_bytes());
        out
    }

    /// Copies the corpus bytes and writes a `u32` into the object table.
    fn with_object_table_u32(field: u32, value: u32) -> Vec<u8> {
        let at = object_table_field_offset(MANDELBROT, field);
        let mut out = MANDELBROT.to_vec();
        assert_ne!(
            out[at..at + 4],
            value.to_le_bytes(),
            "the fixture writes the value the field already holds, so it proves nothing"
        );
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        out
    }

    /// The `.vbp` beside `Mandelbrot.exe` declares
    /// `Name="Mandelbrot_Fractal_Demo"`.
    #[test]
    fn the_project_name_comes_from_the_object_table() {
        assert_eq!(
            object_table(MANDELBROT).unwrap().project_name,
            "Mandelbrot_Fractal_Demo"
        );
    }

    /// The two sources of the project name are two different fields, in two
    /// different structures, reached by two different kinds of pointer.
    ///
    /// The header holds a byte offset from the header base at `0x64`. The
    /// object table holds a virtual address at `0x40`. Neither is derived
    /// from the other, so this is a cross-check and not a tautology.
    #[test]
    fn the_object_table_and_the_header_agree_on_the_project_name() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let header = VbHeader::read(&header_region(&image).unwrap()).unwrap();
        let table = object_table(MANDELBROT).unwrap();
        assert_eq!(table.project_name, header.project_name);
        assert!(!table.project_name.is_empty());
    }

    #[test]
    fn the_corpus_file_declares_one_object_and_its_array_holds_one() {
        let table = object_table(MANDELBROT).unwrap();
        assert_eq!(table.w_total_objects, 1);
        assert_eq!(table.w_compiled_objects, 1);
        assert_eq!(table.object_count(), 1);
        assert!(table.defects().is_empty());
    }

    /// The count is the number of objects and not the capacity of the array.
    ///
    /// `Mandelbrot.exe` cannot tell the two apart, because both of its fields
    /// hold the value one. 15 of the 44 corpus files can tell them apart:
    /// their capacity is rounded up to 4 or to 8 while the project declares
    /// one, two, three or five objects, and the `.vbp` of each proves which
    /// number is the truth. This fixture reproduces that shape on the one
    /// file this module may read.
    #[test]
    fn a_capacity_above_the_object_count_is_normal_and_is_not_a_defect() {
        let bytes = with_object_table_u16(0x2C, 4);
        let table = object_table(&bytes).unwrap();
        assert_eq!(table.w_compiled_objects, 4);
        assert_eq!(
            table.object_count(),
            1,
            "the reported count must be the number of objects the project declares, and not \
             the capacity the compiler rounded the array up to"
        );
        assert!(
            table.defects().is_empty(),
            "a capacity above the count is what 15 of the 44 corpus files hold, so it must \
             not be reported as damage: {:?}",
            table.defects()
        );
    }

    /// A capacity below the count is a real disagreement, and it is not fatal.
    #[test]
    fn a_capacity_below_the_object_count_is_a_recoverable_defect_and_not_a_refusal() {
        let bytes = with_object_table_u16(0x2A, 3);
        let table = object_table(&bytes).expect("a count disagreement must not refuse the file");
        assert_eq!(table.w_total_objects, 3);
        assert_eq!(table.w_compiled_objects, 1);
        // The rest of the read still stands.
        assert_eq!(table.project_name, "Mandelbrot_Fractal_Demo");

        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert!(matches!(
            defect.kind,
            DefectKind::CountMismatch {
                count: 1,
                expected: 3,
                other_field: "wTotalObjects",
                ..
            }
        ));
        // The defect names the byte the parser read, not a byte near it.
        assert_eq!(
            defect.site.offset,
            u32::try_from(object_table_field_offset(MANDELBROT, 0x2C)).unwrap()
        );
        assert_eq!(defect.site.field, "wCompiledObjects");
    }

    #[test]
    fn an_object_table_pointer_in_no_section_is_damaged() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = Va::new(image.image_base() + 0x00F0_0000);
        assert!(image.region_at_va(nowhere).is_none());
        assert_eq!(
            ObjectTableHead::read(&image, nowhere).unwrap_err(),
            Refusal::Damaged("the object table pointer is in no section")
        );
    }

    #[test]
    fn a_project_name_pointer_in_no_section_is_damaged() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        let bytes = with_object_table_u32(0x40, nowhere);
        assert_eq!(
            object_table(&bytes).unwrap_err(),
            Refusal::Damaged("the project name pointer is in no section")
        );
    }
}
