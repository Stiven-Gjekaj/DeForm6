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
//! reported without naming any object. Walking the objects themselves is the
//! work of [`crate::vb::object::ObjectTable::walk`]. The head keeps the
//! address at `0x30` as [`ObjectTableHead::lp_object_array`]. **Do not extend
//! [`ObjectTableHead::read`] to follow that address.** Nothing here sizes an
//! allocation from a field in the file, and SAF-04 requires the count to be
//! checked against the real file length before anything reads the array. The
//! object walk does that check, and it reads the address again for itself.
//! `vb/object.rs` says why.

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Rva, Va};

/// The size of the `ProjectInfo` structure.
///
/// `STRUCTURES.md` section 3 gives `0x23C` = 572 bytes, and four sources
/// agree.
const PROJECT_INFO_SIZE: u32 = 0x23C;

/// `STRUCTURES.md` section 3: `lpExternalTable` sits at `ProjectInfo + 0x234`.
///
/// One constant for the read and for the defect that names this field, so the
/// two cannot come apart.
const LP_EXTERNAL_TABLE_AT: u32 = 0x234;

/// `STRUCTURES.md` section 3: `dwExternalCount` sits at `ProjectInfo + 0x238`.
///
/// One constant for the read and for the defect that names this field, so the
/// two cannot come apart.
const DW_EXTERNAL_COUNT_AT: u32 = 0x238;

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
#[non_exhaustive]
pub struct ProjectInfo {
    /// The absolute file offset of the structure's first byte.
    ///
    /// Kept so a defect about a field in this structure can name that field's
    /// own byte. [`DeclareTable::read`] is handed only this value, not the
    /// window it came from, and has no other way to know where it sits.
    pub file_offset: Off,
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
    /// The window is exactly `PROJECT_INFO_SIZE` bytes, taken before any
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
        let file_offset = window.file_offset(Off::new(0)).ok_or(Refusal::Damaged(
            "the ProjectInfo window has no file offset",
        ))?;
        Ok(Self {
            file_offset,
            dw_version: u32_at(&window, 0x00, "ProjectInfo holds no template version")?,
            lp_object_table: va_at(
                &window,
                0x04,
                "ProjectInfo holds no address for the object table",
            )?,
            lp_native_code: u32_at(&window, 0x20, "ProjectInfo holds no native code address")?,
            lp_external_table: va_at(
                &window,
                LP_EXTERNAL_TABLE_AT,
                "ProjectInfo holds no address for the import table",
            )?,
            dw_external_count: u32_at(
                &window,
                DW_EXTERNAL_COUNT_AT,
                "ProjectInfo holds no import count",
            )?,
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
/// Four fields are read: the two counts, the address of the object array and
/// the address of the project name.
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
    /// The address of the object array, `lpObjectArray` at `0x30`.
    ///
    /// This is a virtual address. The head keeps it and does not follow it,
    /// so an address that maps nowhere is not a reason to refuse the head.
    /// [`crate::vb::object::ObjectTable::walk`] reads the array.
    pub lp_object_array: Va,
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
    /// The window is exactly `OBJECT_TABLE_SIZE` bytes, taken before any
    /// field is read, for the reason [`ProjectInfo::read`] gives.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when the object table address is in no
    /// section, when the file ends inside the structure, when the project
    /// name address is in no section, and when no NUL byte follows the name
    /// within `NAME_MAX` bytes.
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
        let lp_object_array = va_at(
            &window,
            0x30,
            "the object table holds no address for the object array",
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
            lp_object_array,
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

/// # Two tables share the name "external". This module reads both, and this
/// comment is the one place that tells them apart.
///
/// [`ProjectInfo.lp_external_table`] holds the `Declare` import table, read
/// by [`DeclareTable::read`]. Its count is [`ProjectInfo::dw_external_count`].
/// This is the table `STRUCTURES.md` section 7.1 describes.
///
/// [`crate::vb::header::VbHeader`]`.lp_external_table` holds the component
/// table: the OCX and type library references a form pulls in, read by
/// [`ComponentTable::read`]. Its count is `w_external_count` on that same
/// structure. This is `STRUCTURES.md` section 7.3.
///
/// Both pointers are virtual addresses named `lpExternalTable` in the
/// sources this project reads from, on two different structures, with two
/// different counts, and they answer two different questions. `STRUCTURES.md`
/// section 7 opens with the same warning, because getting this wrong means
/// walking one table with the other's count, or resolving one table's
/// address against the wrong structure's field.
///
/// The size of one `Declare` import table entry.
const DECLARE_ENTRY_SIZE: u32 = 8;

/// The number of bytes this reader reads from the descriptor that a
/// `dwEntryType == 7` entry points at.
///
/// `STRUCTURES.md` section 21 finds that the descriptor is 24 bytes. The
/// first 8 hold the two addresses of the names. The dword at `0x0C` holds the
/// address of the thunk data, a module handle and a resolved address that
/// the runtime fills in after loading. The dword at `0x08` holds
/// `0x00040000`, and `0x10` and `0x14` hold 0. None of the other 16 bytes
/// names the library or the export, so this reader reads only the first 8.
const DECLARE_DESCRIPTOR_SIZE: u32 = 8;

/// One `Declare` statement recovered from the external import table.
///
/// # What survives compilation, and what does not
///
/// `STRUCTURES.md` section 7.2 names exactly two things that survive: the
/// library name and the export name. Three do not, and this type marks each
/// one as missing rather than inventing it, per decision D-07:
///
/// - The Visual Basic level procedure name and its `Alias`. A source that
///   wrote `Declare Function FindWindow Lib "user32" Alias "FindWindowA"`
///   leaves only the export name in the file. [`Declaration::NAME_MARKER`]
///   is the comment a caller prints beside it. One prior tool works around
///   the gap with a bundled 800 kilobyte table of known declarations.
///   `AGENTS.md` calls that a database and not recovery, and this crate
///   ships no such table: `grep -rE 'FindWindowA|winapi\.dat|KNOWN_APIS'`
///   over `crates/deform6/src/` finds nothing outside a comment.
/// - The argument names and types. [`Declaration::ARGUMENTS_MARKER`] is the
///   comment. This type carries no argument list, because there is not one
///   to carry: inventing an empty list would look like a recovered
///   signature with zero parameters, which is a different, false claim.
/// - The owning module, and whether the declaration was `Public` or
///   `Private`. [`Declaration::SCOPE_MARKER`] is the comment. One prior
///   tool's own comment records that the executable does not keep this.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Declaration {
    /// The library name, verbatim.
    ///
    /// Neither the presence nor the absence of a `.dll` extension is
    /// normalised. A script run over this corpus found `comdlg32` next to
    /// `comdlg32.dll` in two different programs, and `EZTW32.dll` next to
    /// `kernel32` inside one single program. The extension is part of what
    /// the author wrote in the source, so normalising it would change what
    /// the recovered declaration says.
    pub library: String,
    /// The export name, or the ordinal it encodes.
    pub export: ExportName,
}

impl Declaration {
    /// What a caller prints beside [`Declaration::export`] when it prints a
    /// [`ExportName::Name`]: the file holds no Visual Basic level procedure
    /// name and no `Alias`, only the export name that survives compilation.
    pub const NAME_MARKER: &'static str = "the Visual Basic procedure name and its Alias are not in this file; only the export \
         name that survives compilation is shown";
    /// What a caller prints beside every [`Declaration`]: the file holds no
    /// argument name and no argument type for this statement.
    pub const ARGUMENTS_MARKER: &'static str =
        "the argument names and types of this Declare are not in this file";
    /// What a caller prints beside every [`Declaration`]: the file holds no
    /// record of which module owned this statement, and no `Public` or
    /// `Private` marker for it.
    pub const SCOPE_MARKER: &'static str =
        "the owning module and the Public or Private marker of this Declare are not in this file";
}

/// What an export name gives: a name, or an ordinal it encodes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ExportName {
    /// The export name, exactly as the file holds it.
    Name(String),
    /// An ordinal alias, decoded from a name of the shape `"#123"`.
    ///
    /// `STRUCTURES.md` gap 10: a source level `Alias "#123"` should leave the
    /// literal string `"#123"` in the file, because Visual Basic stores an
    /// alias verbatim. But no source confirms this and no sample was ever
    /// inspected, and a script run over every corpus program and every
    /// corpus binary in this repository, this session, found not one export
    /// name beginning with `#` anywhere, in source or in a compiled file.
    /// This path is therefore flagged inferred wherever it is printed, per
    /// decision D-09, and it stays untested against a real file for the same
    /// reason the P-code branch on [`CompileMode::PCode`] does: there is
    /// nothing in this corpus to test it against.
    OrdinalInferred(u32),
}

/// Parses an export name into a plain name or an inferred ordinal alias.
///
/// The rule is section 7.2's: the whole string after a leading `#` must
/// parse as a decimal integer, or this is a plain name. `"#12a"` is a plain
/// name and not ordinal 12, because the character after the digits is not a
/// digit. An empty string after the `#` is also a plain name, not ordinal 0,
/// because `str::parse` on an empty string returns an error.
fn parse_export_name(name: String) -> ExportName {
    if let Some(digits) = name.strip_prefix('#')
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && let Ok(ordinal) = digits.parse::<u32>()
    {
        return ExportName::OrdinalInferred(ordinal);
    }
    ExportName::Name(name)
}

/// One entry of the `Declare` import table, as the file holds it.
///
/// `STRUCTURES.md` section 7.1 gives an entry as two dwords: the entry type
/// and the address of a descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeclareTableEntry {
    /// `dwEntryType` at `0x00`. The runtime resolves an entry of type 6, and
    /// an entry of type 7 names a library and an export.
    pub dw_entry_type: u32,
    /// `lpImportDescriptor` at `0x04`, the address of the descriptor.
    pub lp_import_descriptor: Va,
    /// The start of the descriptor, for an entry of type 7.
    ///
    /// `None` for an entry of any other type, because only type 7 names a
    /// descriptor of this shape. `None` also for an entry of type 7 whose
    /// descriptor address is in no section, or whose file ends inside the
    /// bytes that this reader reads. The value is kept when a name that it
    /// points at does not resolve.
    pub descriptor: Option<DeclareDescriptor>,
}

/// The two addresses at the start of the descriptor that an entry of type 7
/// names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeclareDescriptor {
    /// `lpDllName` at `0x00`, the address of the library name.
    pub lp_dll_name: Va,
    /// `lpApiName` at `0x04`, the address of the export name.
    pub lp_api_name: Va,
}

/// The `Declare` import table, reached from [`ProjectInfo::lp_external_table`].
///
/// Read the doc comment above `DECLARE_ENTRY_SIZE` before reaching for this
/// type from the wrong pointer: [`crate::vb::header::VbHeader`] holds a
/// different table under the same field name.
#[derive(Clone, Debug)]
pub struct DeclareTable {
    /// Every `dwEntryType == 7` entry the walk resolved, in table order.
    pub declarations: Vec<Declaration>,
    /// Every entry the walk read, of every type, in table order.
    ///
    /// The position of an entry in this list is its index in the table. The
    /// walk skips no entry that it read, so an entry that gives a defect is
    /// here too. When the count is larger than the table can hold, this list
    /// holds each whole entry that the table holds.
    pub entries: Vec<DeclareTableEntry>,
    defects: Vec<Defect>,
}

impl DeclareTable {
    /// Walks the `Declare` import table.
    ///
    /// This never refuses. A zero count gives empty lists without touching
    /// [`ProjectInfo::lp_external_table`] at all, which is what
    /// `LockWorkStation.exe` needs: this corpus's smallest program declares no
    /// external table, and there is no reason to dereference a pointer this
    /// walk will not use.
    ///
    /// A count that is not zero, with a table address in no section, gives
    /// empty lists and one [`DefectKind::ItemAddressUnmapped`] at
    /// `lpExternalTable`. The whole table is the item that is lost. Empty
    /// lists with no defect would say that the project declares nothing,
    /// which is a different claim.
    ///
    /// # Bounding `dwExternalCount`
    ///
    /// The count is a `u32` straight out of the file. Before the loop, it is
    /// checked against the real length of the region the table's own address
    /// resolves to: it must not be larger than the number of whole entries
    /// that the region holds. The check multiplies nothing, so a count whose
    /// size in bytes leaves a `u32` is checked too. A count that does not fit
    /// is `DefectKind::ImplausibleCount` at `Recoverable`, and
    /// the loop below still bounds itself independently, one entry at a
    /// time, through [`Region::subregion`]. The largest count measured in
    /// this corpus is 26, in `Edge_Detection.exe`, and the whole corpus holds
    /// 249 entries.
    ///
    /// # A gap in the shared defect vocabulary, closed
    ///
    /// The three recoverable outcomes below once reused
    /// [`DefectKind::UnmappedAddress`], a variant whose own `severity()` was
    /// `Fatal`, written for a spine pointer whose loss means nothing
    /// downstream resolves. Losing one entry's descriptor is not that: the
    /// other entries still resolve, which every test in this module proves.
    /// Phase 5 wired [`crate::journal::Journal::record`] into `inspect`'s
    /// choke point, and that caller *does* consult `severity()` on every
    /// defect this walk collects, so the mismatch was no longer harmless: a
    /// `Fatal` defect here refused the whole file in both `Mode::Strict` and
    /// `Mode::Salvage`. These three outcomes now build
    /// [`DefectKind::ItemAddressUnmapped`] instead, which `severity()` marks
    /// `Recoverable`: `Mode::Strict` still refuses, and `Mode::Salvage`
    /// loses the one entry and keeps walking the rest of the table. A table
    /// whose own address maps nowhere gives the same variant, because the
    /// rest of the program still resolves without it.
    #[must_use]
    pub fn read(pe: &PeImage<'_>, info: &ProjectInfo) -> Self {
        let mut declarations = Vec::new();
        let mut entries = Vec::new();
        let mut defects = Vec::new();

        if info.dw_external_count == 0 {
            return Self {
                declarations,
                entries,
                defects,
            };
        }

        let Some(table) = pe.region_at_va(info.lp_external_table) else {
            // The count says that the table holds entries, and the table's
            // address maps nowhere. The defect names lpExternalTable, which
            // lives in ProjectInfo. The reader knows where ProjectInfo sits
            // in the file and not its address, so the site gives no address.
            // The kind carries the address that the pointer holds.
            let offset = info
                .file_offset
                .checked_add(LP_EXTERNAL_TABLE_AT)
                .map_or(0, Off::get);
            defects.push(Defect {
                site: Site {
                    offset,
                    rva: None,
                    structure: "ProjectInfo",
                    field: "lpExternalTable",
                },
                kind: DefectKind::ItemAddressUnmapped {
                    offset,
                    va: info.lp_external_table.get(),
                },
            });
            return Self {
                declarations,
                entries,
                defects,
            };
        };

        // The number of whole entries the region holds. The loop below bounds
        // itself through `Region::subregion`, one entry at a time, and never
        // reads this value. A division by a non-zero constant cannot fail.
        let max_entries = table.len().checked_div(DECLARE_ENTRY_SIZE).unwrap_or(0);
        if info.dw_external_count > max_entries {
            // The defect names dwExternalCount itself, which lives in
            // ProjectInfo and not in the table it bounds. ImplausibleCount
            // documents its offset as that of the count field.
            let offset = info
                .file_offset
                .checked_add(DW_EXTERNAL_COUNT_AT)
                .map_or(0, Off::get);
            defects.push(Defect {
                site: Site {
                    offset,
                    // The offset came from where ProjectInfo sits, not from
                    // lpExternalTable, so the table's address is not the
                    // address of this byte.
                    rva: None,
                    structure: "ProjectInfo",
                    field: "dwExternalCount",
                },
                kind: DefectKind::ImplausibleCount {
                    offset,
                    count: info.dw_external_count,
                    max: max_entries,
                },
            });
        }

        for i in 0..info.dw_external_count {
            let Some(byte_off) = i.checked_mul(DECLARE_ENTRY_SIZE) else {
                break;
            };
            let Some(entry) = table.subregion(Off::new(byte_off), DECLARE_ENTRY_SIZE) else {
                break;
            };
            let entry_offset = entry.file_offset(Off::new(0)).map_or(0, Off::get);

            // The subregion above is exactly `DECLARE_ENTRY_SIZE` bytes, so
            // both reads below always succeed. The fallback stays because
            // `Region` has no infallible accessor. No test covers it and no
            // test can.
            let Some(entry_type) = entry.u32_le(Off::new(0x00)) else {
                break;
            };
            let Some(descriptor_va) = entry.va_le(Off::new(0x04)) else {
                break;
            };

            // Only an entry of type 7 names a descriptor of this shape. The
            // arm for type 6 below says what the other shape is.
            let descriptor = if entry_type == 7 {
                read_declare_descriptor(pe, descriptor_va)
            } else {
                None
            };
            entries.push(DeclareTableEntry {
                dw_entry_type: entry_type,
                lp_import_descriptor: descriptor_va,
                descriptor,
            });

            match entry_type {
                // Resolved inside the runtime. Its descriptor points at a
                // different shape entirely: a pair of addresses whose first
                // four words one source reports as identical across every
                // Visual Basic application. Dereferencing one as a library
                // and export name pair would present unrelated in-image
                // bytes as a recovered declaration. This is not a defect: 29
                // of the 249 entries in this corpus are this type, one in
                // each of 29 of the 44 programs, so most programs take this
                // path. `[VERIFIED: local]`
                6 => {}
                7 => match descriptor
                    .ok_or_else(|| DeclareDescriptorFailure::no_descriptor(pe, descriptor_va))
                    .and_then(|pair| read_declare_names(pe, pair))
                {
                    Ok((library, export)) => declarations.push(Declaration {
                        library,
                        export: parse_export_name(export),
                    }),
                    Err(failure) => {
                        defects.push(failure.into_defect(pe, &entry, descriptor_va));
                    }
                },
                // No source describes any value but 6 and 7. A value found
                // here is undocumented, so it is skipped and named in a
                // defect rather than guessed at.
                other => defects.push(Defect {
                    site: Site {
                        offset: entry_offset,
                        rva: None,
                        structure: "DeclareTableEntry",
                        field: "dwEntryType",
                    },
                    kind: DefectKind::CountMismatch {
                        offset: entry_offset,
                        count: other,
                        expected: 7,
                        other_field: "dwEntryType, which this table defines only as 6 \
                                      (internal) or 7 (external)",
                    },
                }),
            }
        }

        Self {
            declarations,
            entries,
            defects,
        }
    }

    /// Gives the defects the walk found: an implausible count, an
    /// undocumented entry type, or a descriptor that resolves nowhere.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }
}

/// Why a `dwEntryType == 7` entry did not resolve to a declaration.
enum DeclareDescriptorFailure {
    /// The reader could not use the descriptor that `lpImportDescriptor`
    /// names.
    Descriptor(Miss),
    /// The reader could not use the library name at this address, which
    /// `lpDllName` holds.
    DllName(Va, Miss),
    /// The reader could not use the export name at this address, which
    /// `lpApiName` holds.
    ApiName(Va, Miss),
}

/// What the reader found at an address that it could not use.
enum Miss {
    /// The address is in no section.
    Unmapped,
    /// The address is in a section, and the file holds fewer than `len`
    /// bytes there.
    CutShort {
        /// The number of bytes that the reader must read.
        len: u32,
    },
    /// The address is in a section, and no NUL comes in the bytes that the
    /// reader searched.
    Unterminated {
        /// The absolute file offset of the text.
        at: Option<Off>,
        /// The number of bytes that the reader searched.
        searched: u32,
    },
}

impl DeclareDescriptorFailure {
    /// Tells why `read_declare_descriptor` gave nothing for `descriptor_va`.
    ///
    /// That function gives nothing only when the address is in no section,
    /// or when its window is too short. So an address that maps is a
    /// descriptor that the file cuts short.
    fn no_descriptor(pe: &PeImage<'_>, descriptor_va: Va) -> Self {
        if pe.region_at_va(descriptor_va).is_some() {
            Self::Descriptor(Miss::CutShort {
                len: DECLARE_DESCRIPTOR_SIZE,
            })
        } else {
            Self::Descriptor(Miss::Unmapped)
        }
    }

    /// Builds the defect a caller records for this failure.
    ///
    /// The site names the pointer that held the address, which is the
    /// offset that [`DefectKind::ItemAddressUnmapped`] and
    /// [`DefectKind::ItemCutShort`] document. A
    /// [`DefectKind::NoNulTerminator`] gives the offset of the text itself,
    /// as its own doc comment says, and the number of bytes that the reader
    /// searched.
    /// `lpImportDescriptor` is at `0x04` of the entry. `lpDllName` and
    /// `lpApiName` are at `0x00` and `0x04` of the descriptor, which is a
    /// different structure. The site's `rva` is the address of the pointer's
    /// own byte, and the kind carries the address that the pointer holds.
    fn into_defect(self, pe: &PeImage<'_>, entry: &Region<'_>, descriptor_va: Va) -> Defect {
        // The file offset and the address of the byte at `at` of `window`.
        let place = |window: Option<Region<'_>>, at: u32| {
            let at = Off::new(at);
            (
                window.and_then(|w| w.file_offset(at)),
                window.and_then(|w| w.rva(at)),
            )
        };
        let descriptor = pe.region_at_va(descriptor_va);
        let (structure, field, (at, rva), va, miss) = match self {
            Self::Descriptor(miss) => (
                "DeclareTableEntry",
                "lpImportDescriptor",
                place(Some(*entry), 0x04),
                descriptor_va,
                miss,
            ),
            Self::DllName(va, miss) => (
                "DeclareDescriptor",
                "lpDllName",
                place(descriptor, 0x00),
                va,
                miss,
            ),
            Self::ApiName(va, miss) => (
                "DeclareDescriptor",
                "lpApiName",
                place(descriptor, 0x04),
                va,
                miss,
            ),
        };
        // Each pointer lies inside a window that the reader already read, so
        // it has a file offset. `Region` has no infallible accessor, so the
        // fallback is written out. It names offset 0, which is visibly not
        // the site of a field.
        let offset = at.map_or(0, Off::get);
        let kind = match miss {
            Miss::Unmapped => DefectKind::ItemAddressUnmapped {
                offset,
                va: va.get(),
            },
            Miss::CutShort { len } => DefectKind::ItemCutShort {
                offset,
                va: va.get(),
                len,
            },
            Miss::Unterminated { at, searched } => DefectKind::NoNulTerminator {
                offset: at.map_or(0, Off::get),
                limit: searched,
            },
        };
        Defect {
            site: Site {
                offset,
                rva: rva.map(Rva::get),
                structure,
                field,
            },
            kind,
        }
    }
}

/// Reads the two addresses at the start of the descriptor that a
/// `dwEntryType == 7` entry names.
///
/// The descriptor is 24 bytes, and the other 16 bytes hold no name. See
/// `DECLARE_DESCRIPTOR_SIZE`.
///
/// `None` when the address is in no section, or when the file ends inside
/// the `DECLARE_DESCRIPTOR_SIZE` bytes that this function reads.
fn read_declare_descriptor(pe: &PeImage<'_>, descriptor_va: Va) -> Option<DeclareDescriptor> {
    let descriptor = pe
        .region_at_va(descriptor_va)?
        .subregion(Off::new(0), DECLARE_DESCRIPTOR_SIZE)?;
    Some(DeclareDescriptor {
        lp_dll_name: descriptor.va_le(Off::new(0x00))?,
        lp_api_name: descriptor.va_le(Off::new(0x04))?,
    })
}

/// Reads the library name and the export name that a descriptor names.
fn read_declare_names(
    pe: &PeImage<'_>,
    descriptor: DeclareDescriptor,
) -> Result<(String, String), DeclareDescriptorFailure> {
    let DeclareDescriptor {
        lp_dll_name,
        lp_api_name,
    } = descriptor;
    let library = read_declare_name(pe, lp_dll_name)
        .map_err(|miss| DeclareDescriptorFailure::DllName(lp_dll_name, miss))?;
    let export = read_declare_name(pe, lp_api_name)
        .map_err(|miss| DeclareDescriptorFailure::ApiName(lp_api_name, miss))?;
    Ok((library, export))
}

/// Resolves a virtual address to a NUL terminated string, Latin-1 decoded.
///
/// This is the same rule `vb/header.rs` and this module's own
/// [`ObjectTableHead::read`] use for every string this crate reads:
/// `char::from(byte)` gives the Latin-1 code point. `String::from_utf8_lossy`
/// is wrong here, because a byte in `0x80` to `0xFF` would become the
/// replacement character and the name would be lost.
fn read_declare_name(pe: &PeImage<'_>, va: Va) -> Result<String, Miss> {
    let region = pe.region_at_va(va).ok_or(Miss::Unmapped)?;
    let bytes = region
        .cstr(Off::new(0), NAME_MAX)
        .ok_or_else(|| Miss::Unterminated {
            at: region.file_offset(Off::new(0)),
            searched: region.cstr_span(Off::new(0), NAME_MAX),
        })?;
    Ok(bytes.iter().copied().map(char::from).collect())
}

/// One entry of the external component table: an OCX or type library
/// reference a form pulls in.
///
/// Read the doc comment above `DECLARE_ENTRY_SIZE` first: this is reached
/// from [`crate::vb::header::VbHeader::lp_external_table`], never from
/// [`ProjectInfo::lp_external_table`].
///
/// `[`Component::library`]` is the key Phase 3 joins a form's external
/// control to its CLSID, and this table is what produces the `Object=` lines
/// of the `.vbp` in Phase 4.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Component {
    /// The OCX or DLL file name, e.g. `"MSWINSCK.OCX"`.
    pub file_name: String,
    /// The library name, e.g. `"MSWinsockLib.Winsock"`.
    pub library: String,
    /// The component name, e.g. `"Winsock"`.
    pub name: String,
    /// The entry-relative offset of the textual GUID. `STRUCTURES.md` gap 17
    /// leaves three other fields opaque; nothing in this phase needs them.
    /// This offset and [`Self::guid_length`] are decoded, as of plan 03-08,
    /// into [`Self::guid_text`]. See that field's own doc comment.
    pub guid_offset: Off,
    /// `-1` means the textual GUID at `GUIDoffset` is absent. `72` means it
    /// is 36 UTF-16 characters. Decoded, as of plan 03-08, into
    /// [`Self::guid_text`]. See that field's own doc comment.
    ///
    /// Plan 03-08's own doc comment named this "no binary identifier at
    /// `oUuid`", which conflates two different entry fields. `oUuid` is a
    /// separate field, entry offset `0x04`, decoded independently into
    /// [`Self::ouuid_text`]; `guid_length` governs `GUIDoffset` alone.
    pub guid_length: i32,
    /// The textual GUID, decoded from [`Self::guid_offset`] and
    /// [`Self::guid_length`] inside `ComponentTable::walk`, where the
    /// entry's own region is in scope: `guid_offset` is relative to the
    /// entry, not to the table, the same warning `STRUCTURES.md` section 7.3
    /// opens with.
    ///
    /// `None` when `guid_length` is `-1`, a normal state and not a
    /// [`Defect`]. `Some` of 36 characters (no braces) when `guid_length` is
    /// `72`.
    ///
    /// # Plan 03-16: measured, and not what `vb/ocx.rs::join_component`
    /// reports as a control's CLSID
    ///
    /// Measured against all three corpus programs that declare a
    /// `MSWinsockLib.Winsock` component: this field decodes to
    /// `2c49f800-c2dd-11cf-9ad6-0080c7e7b78d` in every one, identically. The
    /// matching `.vbp` file's own `Object=` line declares
    /// `248DD890-BB45-11CF-9ABC-0080C7E7B78D`. The two share no digit
    /// pattern. A whole-entry search of six encodings (the sixteen byte
    /// binary layout in both field orders, and plain text and sixteen bit
    /// text in both cases) never found the declared identifier anywhere in
    /// the entry of any of the three programs. `STRUCTURES.md` section 7.3
    /// carries the full eighteen-search record. See [`Self::ouuid_text`]'s
    /// own doc comment for the field this repository reports instead.
    pub guid_text: Option<String>,
    /// The entry-relative offset value the `oUuid` field (entry offset
    /// `0x04`) holds: an offset to a sixteen byte binary GUID,
    /// `STRUCTURES.md` section 7.3. Decoded, as of plan 03-16, into
    /// [`Self::ouuid_text`].
    pub o_uuid: Off,
    /// The absolute file offset of the sixteen raw bytes [`Self::o_uuid`]
    /// points to, computed once regardless of whether the sixteen bytes
    /// could be read. `vb/ocx.rs::join_component` carries this offset into
    /// the caveat it attaches to a reported CLSID, so a reader can open the
    /// file at this offset and see the same bytes `STRUCTURES.md` section
    /// 7.3 documents, the same convention `error.rs`'s own `Site::offset`
    /// uses for every other byte offset this crate reports.
    pub ouuid_field_offset: u32,
    /// The textual form of the sixteen raw bytes at [`Self::o_uuid`],
    /// decoded inside `ComponentTable::walk`, where the entry's own
    /// region is in scope, as a standard Microsoft binary GUID: the first
    /// three fields (four, two and two bytes) are stored little-endian and
    /// are reversed back to their textual byte order; the fourth field
    /// (eight bytes) is stored raw and is never reversed.
    ///
    /// `None` when the entry's own region holds fewer than sixteen bytes at
    /// `o_uuid`, a [`Defect`] naming the byte offset.
    ///
    /// # Plan 03-16: measured, and selected as the field this repository
    /// reports as a control's CLSID
    ///
    /// Measured against all three corpus programs that declare a
    /// `MSWinsockLib.Winsock` component: this field decodes to
    /// `248DD896-BB45-11CF-9ABC-0080C7E7B78D` in every one, identically.
    /// That differs from the matching `.vbp` file's own declared identifier,
    /// `248DD890-BB45-11CF-9ABC-0080C7E7B78D`, by exactly one byte: the low
    /// byte of `Data1`. The same eighteen-search sweep [`Self::guid_text`]'s
    /// own doc comment describes never found the declared identifier here
    /// either.
    ///
    /// Neither field is confirmed against the declared identifier. This one
    /// is selected because its own shape, a fixed sixteen byte binary
    /// identifier read at a fixed entry offset, is the shape a CLSID takes,
    /// and because it is the closer of the two candidates to the declared
    /// value. `vb/ocx.rs::join_component` reports it with an honest caveat
    /// attached, naming the byte offset and stating plainly that this value
    /// is not confirmed against the control's own project file. It is never
    /// presented as a confirmed match: a value differing by even one byte is
    /// a different identifier.
    pub ouuid_text: Option<String>,
}

/// The external component table, reached from
/// [`crate::vb::header::VbHeader::lp_external_table`].
#[derive(Clone, Debug)]
pub struct ComponentTable {
    /// Every entry the walk resolved, in table order.
    pub components: Vec<Component>,
    defects: Vec<Defect>,
}

impl ComponentTable {
    /// Walks the external component table.
    ///
    /// Entries are variable length and self-describing: `StructLength` at
    /// entry offset `0x00` gives the length of this entry, and the next
    /// entry starts at this entry plus that length. **Every sub-offset below
    /// is relative to the start of the entry that names it, never to the
    /// start of the table.** `STRUCTURES.md` section 7.3 opens with the same
    /// warning.
    ///
    /// This never refuses. A zero count gives an empty list without
    /// touching `lp_external_table` at all, matching `Mandelbrot.exe` and
    /// `Grayscale.exe`, neither of which references a component.
    ///
    /// # What this corpus proves and what it does not
    ///
    /// A script run over all 44 vendored programs, this session, found
    /// three component entries in total, in three programs, and all three
    /// name the same control: file `MSWINSCK.OCX`, library
    /// `MSWinsockLib.Winsock`, component `Winsock`, each with a declared
    /// entry length of 368. One distinct sample proves the three string
    /// offsets resolve. It does not prove the walk advances correctly from
    /// one entry to the next, because no corpus file holds a second entry to
    /// advance to. A synthetic two-entry table in this module's own tests
    /// proves that: [`crate::vb::header::header_region`]'s test module holds
    /// a synthetic image builder that plan 02-06's action text points to,
    /// but that helper is private to that module's own `#[cfg(test)]` block
    /// and is not reachable from here, so this module builds an independent
    /// synthetic image of the same shape rather than importing one.
    #[must_use]
    pub fn read(pe: &PeImage<'_>, lp_external_table: Va, w_external_count: u16) -> Self {
        let mut components = Vec::new();
        let mut defects = Vec::new();

        if w_external_count == 0 {
            return Self {
                components,
                defects,
            };
        }

        let Some(table) = pe.region_at_va(lp_external_table) else {
            return Self {
                components,
                defects,
            };
        };

        let mut cursor = Off::new(0);
        for _ in 0..w_external_count {
            let Some(entry_offset) = table.file_offset(cursor) else {
                break;
            };
            let Some(struct_len) = table.u32_le(cursor) else {
                break;
            };

            if struct_len == 0 {
                // A zero length would leave the cursor standing still for
                // the rest of the count. Stop rather than loop.
                defects.push(Defect {
                    site: Site {
                        offset: entry_offset.get(),
                        rva: None,
                        structure: "ExternalComponentEntry",
                        field: "StructLength",
                    },
                    kind: DefectKind::CountMismatch {
                        offset: entry_offset.get(),
                        count: 0,
                        expected: 1,
                        other_field: "StructLength, which must be at least 1 byte for the \
                                      cursor to advance",
                    },
                });
                break;
            }

            let Some(entry) = table.subregion(cursor, struct_len) else {
                // The declared length runs past what the region holds.
                let remaining = table.len().saturating_sub(cursor.get());
                defects.push(Defect {
                    site: Site {
                        offset: entry_offset.get(),
                        rva: None,
                        structure: "ExternalComponentEntry",
                        field: "StructLength",
                    },
                    kind: DefectKind::ImplausibleCount {
                        offset: entry_offset.get(),
                        count: struct_len,
                        max: remaining,
                    },
                });
                break;
            };

            match read_component_entry(&entry) {
                Ok(mut component) => {
                    let (guid_text, guid_defect) = decode_guid_text(
                        &entry,
                        entry_offset.get(),
                        component.guid_offset,
                        component.guid_length,
                    );
                    component.guid_text = guid_text;
                    if let Some(defect) = guid_defect {
                        defects.push(defect);
                    }

                    component.ouuid_field_offset = entry
                        .file_offset(component.o_uuid)
                        .map_or(entry_offset.get(), Off::get);
                    let (ouuid_text, ouuid_defect) = decode_ouuid_text(&entry, component.o_uuid);
                    component.ouuid_text = ouuid_text;
                    if let Some(defect) = ouuid_defect {
                        defects.push(defect);
                    }

                    components.push(component);
                }
                // The corpus's one distinct sample resolves cleanly on all
                // three of its occurrences, so this is defensive and not
                // corpus-exercised: five of thirteen fields in this
                // structure are unknown, and a file that declares a length
                // too short to hold the fixed fields, or a string offset
                // with no terminator, must not panic and must not invent a
                // component out of the bytes that are there.
                Err(failure) => defects.push(failure.into_defect(&entry)),
            }

            let Some(next) = cursor.checked_add(struct_len) else {
                break;
            };
            cursor = next;
        }

        Self {
            components,
            defects,
        }
    }

    /// Gives the defects the walk found: a zero or overrunning declared
    /// length, or an entry whose fixed fields or strings did not resolve.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }

    /// Builds a `ComponentTable` directly from a component list, for a
    /// synthetic fixture in a sibling module's own test suite (`vb/ocx.rs`,
    /// plan 03-08's `join_component`). [`ComponentTable::read`] is
    /// otherwise the only builder; this exists because [`Self::defects`] is
    /// private everywhere except inside this module, and `join_component`'s
    /// own tests need a table with no components, or with a hand-written
    /// one, not a full synthetic portable executable.
    #[cfg(test)]
    pub(crate) const fn synthetic(components: Vec<Component>) -> Self {
        Self {
            components,
            defects: Vec::new(),
        }
    }
}

/// The length of the fixed fields of a component entry. `NameOffset`, at
/// `0x30`, is the last of them (`STRUCTURES.md` section 7.3).
const COMPONENT_FIXED_LEN: u32 = 0x34;

/// Why one component entry gives no component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryFailure {
    /// `StructLength` is shorter than the fixed fields.
    Short,
    /// The string that an offset field names has no NUL in the bytes that
    /// the search read. `field_at` is where the offset field is in the
    /// entry, and `text` is the offset that the field holds.
    Unterminated {
        field: &'static str,
        field_at: Off,
        text: Off,
    },
}

impl EntryFailure {
    /// Builds the defect for this failure in `entry`.
    ///
    /// A short entry gives its defect at `StructLength`, as a length of zero
    /// does. A string with no NUL gives its defect at the offset field that
    /// names the string. The kind gives where the text starts and the number
    /// of bytes that the search read. That number is `0` when the offset
    /// names no byte of the entry.
    fn into_defect(self, entry: &Region<'_>) -> Defect {
        let site = |at: Off, field: &'static str| Site {
            offset: entry.file_offset(at).map_or(0, Off::get),
            rva: entry.rva(at).map(Rva::get),
            structure: "ExternalComponentEntry",
            field,
        };
        match self {
            Self::Short => {
                let site = site(Off::new(0), "StructLength");
                let kind = DefectKind::CountMismatch {
                    offset: site.offset,
                    count: entry.len(),
                    expected: COMPONENT_FIXED_LEN,
                    other_field: "the fixed part of an ExternalComponentEntry",
                };
                Defect { site, kind }
            }
            Self::Unterminated {
                field,
                field_at,
                text,
            } => Defect {
                site: site(field_at, field),
                kind: DefectKind::NoNulTerminator {
                    offset: entry.file_offset(text).map_or(0, Off::get),
                    limit: entry.cstr_span(text, NAME_MAX),
                },
            },
        }
    }
}

/// Reads the three strings and the GUID fields of one component entry.
///
/// Every offset read here is relative to `entry`'s own base, never to the
/// table. `entry` already spans exactly `StructLength` bytes, so a string
/// offset that names a byte inside this entry resolves inside `entry`
/// directly, with no second address to follow.
///
/// An entry shorter than `COMPONENT_FIXED_LEN` bytes gives
/// [`EntryFailure::Short`] before any field is read. The field reads after
/// that check cannot fail. They keep a failure because `Region` has no
/// infallible accessor, and no test covers them for that reason.
fn read_component_entry(entry: &Region<'_>) -> Result<Component, EntryFailure> {
    if entry.len() < COMPONENT_FIXED_LEN {
        return Err(EntryFailure::Short);
    }
    let o_uuid = entry.off_le(Off::new(0x04)).ok_or(EntryFailure::Short)?;
    let guid_offset = entry.off_le(Off::new(0x1C)).ok_or(EntryFailure::Short)?;
    let guid_length = entry.i32_le(Off::new(0x20)).ok_or(EntryFailure::Short)?;

    let file_name = component_string(entry, "FileNameOffset", Off::new(0x28))?;
    let library = component_string(entry, "SourceOffset", Off::new(0x2C))?;
    let name = component_string(entry, "NameOffset", Off::new(0x30))?;

    Ok(Component {
        file_name,
        library,
        name,
        guid_offset,
        guid_length,
        // Decoded by the caller, `ComponentTable::walk`, where `entry` (this
        // function's own borrow of it ends here) is still in scope.
        guid_text: None,
        o_uuid,
        // Both filled by the caller, for the same reason `guid_text` is.
        ouuid_field_offset: 0,
        ouuid_text: None,
    })
}

/// Decodes a component entry's own textual GUID, per `STRUCTURES.md`
/// section 7.3: `guid_offset` is relative to the entry's own base, never to
/// the table, the same warning that section opens with.
///
/// `guid_length` of `-1` means no binary identifier and gives `None` with no
/// `Defect`: a normal state [`Component::guid_text`]'s own doc comment
/// already names. `72` means the textual GUID is 36 UTF-16 characters; the
/// 72 bytes are read bounded by the entry's own region, so a `guid_offset`
/// that leaves too little room does not size an allocation from
/// `guid_length` before that bound is checked. Such a `guid_offset` gives a
/// [`DefectKind::RunsPastEnd`] defect at the `GUIDoffset` field. Any other
/// value of `guid_length` gives a `Defect` naming it, and gives `None`.
fn decode_guid_text(
    entry: &Region<'_>,
    entry_offset: u32,
    guid_offset: Off,
    guid_length: i32,
) -> (Option<String>, Option<Defect>) {
    match guid_length {
        -1 => (None, None),
        72 => match entry.take(guid_offset, 72) {
            Some(bytes) => {
                let mut units = Vec::with_capacity(36);
                for pair in bytes.chunks_exact(2) {
                    let Ok(raw) = <[u8; 2]>::try_from(pair) else {
                        continue;
                    };
                    units.push(u16::from_le_bytes(raw));
                }
                (Some(String::from_utf16_lossy(&units)), None)
            }
            None => {
                let defect = past_entry_end(entry, "GUIDoffset", Off::new(0x1C), guid_offset, 72);
                (None, Some(defect))
            }
        },
        other => {
            let defect = Defect {
                site: Site {
                    offset: entry_offset,
                    rva: None,
                    structure: "ExternalComponentEntry",
                    field: "GUIDlength",
                },
                kind: DefectKind::GuidLengthUnexpected {
                    offset: entry_offset,
                    value: other,
                },
            };
            (None, Some(defect))
        }
    }
}

/// Decodes a component entry's own sixteen byte binary GUID at `oUuid`, per
/// `STRUCTURES.md` section 7.3, entry offset `0x04`: `o_uuid` is itself an
/// offset relative to the entry's own base, never to the table, read
/// bounded by the entry's own region.
///
/// Gives `None` and a [`DefectKind::RunsPastEnd`] defect when the entry
/// holds fewer than sixteen bytes at `o_uuid`. Its site is the `oUuid`
/// field. The sixteen bytes decode as
/// a standard Microsoft binary GUID: the first three fields (four, two and
/// two bytes) are stored little-endian and are reversed back to their
/// textual byte order; the fourth field (eight bytes) is stored raw and is
/// never reversed. [`Component::ouuid_text`]'s own doc comment carries the
/// measurement that selected this field.
fn decode_ouuid_text(entry: &Region<'_>, o_uuid: Off) -> (Option<String>, Option<Defect>) {
    match entry.take(o_uuid, 16) {
        Some(bytes) => {
            // `entry.take` gives exactly sixteen bytes or `None`, never a
            // shorter slice, so this copy always succeeds.
            let mut b = [0_u8; 16];
            b.copy_from_slice(bytes);
            let text = format!(
                "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-\
                 {:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                b[3],
                b[2],
                b[1],
                b[0],
                b[5],
                b[4],
                b[7],
                b[6],
                b[8],
                b[9],
                b[10],
                b[11],
                b[12],
                b[13],
                b[14],
                b[15],
            );
            (Some(text), None)
        }
        None => {
            let defect = past_entry_end(entry, "oUuid", Off::new(0x04), o_uuid, 16);
            (None, Some(defect))
        }
    }
}

/// Builds the defect for the `len` bytes at `at` in `entry` that the offset
/// field `field` at `field_at` names, when they run past the end that
/// `StructLength` gives the entry.
///
/// The site is the offset field. The kind gives the bytes and the end of
/// the entry.
fn past_entry_end(
    entry: &Region<'_>,
    field: &'static str,
    field_at: Off,
    at: Off,
    len: u32,
) -> Defect {
    Defect {
        site: Site {
            offset: entry.file_offset(field_at).map_or(0, Off::get),
            rva: entry.rva(field_at).map(Rva::get),
            structure: "ExternalComponentEntry",
            field,
        },
        kind: DefectKind::RunsPastEnd {
            offset: entry.file_offset(at).map_or(0, Off::get),
            len,
            end: entry.file_offset(Off::new(entry.len())).map_or(0, Off::get),
        },
    }
}

/// Reads the NUL terminated, Latin-1 decoded string that the offset field
/// `field` at `field_at` names. The offset is relative to the entry.
fn component_string(
    entry: &Region<'_>,
    field: &'static str,
    field_at: Off,
) -> Result<String, EntryFailure> {
    let text = entry.off_le(field_at).ok_or(EntryFailure::Short)?;
    let bytes = entry
        .cstr(text, NAME_MAX)
        .ok_or(EntryFailure::Unterminated {
            field,
            field_at,
            text,
        })?;
    Ok(bytes.iter().copied().map(char::from).collect())
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
    use super::{
        CompileMode, Component, ComponentTable, Declaration, DeclareTable, ExportName, NAME_MAX,
        ObjectTableHead, PROJECT_INFO_SIZE, ProjectInfo, parse_export_name,
    };
    use crate::error::Refusal;
    use crate::error::{Defect, DefectKind, Severity, Site};
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Rva, Va};
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

    /// Nine `Declare` table entries, eight of them external. `[VERIFIED:
    /// local]` `dw_external_count == 9`.
    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    /// A zero-entry `Declare` table, and this corpus's smallest program.
    const LOCK_WORK_STATION: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/LockWorkStation/LockWorkStation.exe"
    ));

    /// One library name carries a `.dll` extension and another does not,
    /// inside the same program: `EZTW32.dll` next to `kernel32`.
    const VB_SCANNER_SUPPORT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Scanner-TWAIN/VB_Scanner_Support.exe"
    ));

    /// One of the three corpus programs whose component table holds an
    /// entry: file `MSWINSCK.OCX`, library `MSWinsockLib.Winsock`, component
    /// `Winsock`.
    const SERVER: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/SK-TFTP-Sample__VB6/Server/demo/Server.exe"
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

    /// The value is read by hand at `0x30`, through the route that
    /// `object_table_field_offset` documents, and not through the head.
    #[test]
    fn the_head_keeps_the_object_array_address_that_the_table_holds() {
        let at = object_table_field_offset(MANDELBROT, 0x30);
        let by_hand = u32::from_le_bytes(MANDELBROT[at..at + 4].try_into().unwrap());
        let head = object_table(MANDELBROT).unwrap();
        assert_eq!(head.lp_object_array, Va::new(by_hand));
        // A null or unmapped value here would make the comparison above weak.
        let image = PeImage::parse(MANDELBROT).unwrap();
        assert!(image.region_at_va(head.lp_object_array).is_some());
    }

    /// The head keeps the address and does not follow it, so an array
    /// address that maps nowhere is kept as the file holds it. The object
    /// walk is the reader that refuses it.
    #[test]
    fn an_object_array_address_in_no_section_is_kept_and_is_not_damage() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        assert!(image.region_at_va(Va::new(nowhere)).is_none());
        let bytes = with_object_table_u32(0x30, nowhere);
        let head = object_table(&bytes).expect("the head must not follow the array address");
        assert_eq!(head.lp_object_array, Va::new(nowhere));
        assert_eq!(head.project_name, "Mandelbrot_Fractal_Demo");
        assert_eq!(head.object_count(), 1);
        assert!(head.defects().is_empty(), "{:?}", head.defects());
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

    /// Walks the `Declare` import table out of a byte slice.
    fn declare_table(data: &[u8]) -> DeclareTable {
        let image = PeImage::parse(data).unwrap();
        let info = ProjectInfo::read(&image, project_data_va(data)).unwrap();
        DeclareTable::read(&image, &info)
    }

    /// Gives every recovered declaration as a `(library, export)` pair of
    /// plain strings, so a test can compare against an ordered literal.
    ///
    /// A declaration whose export is an inferred ordinal has no place in this
    /// shape; no test that calls this reaches one.
    fn as_pairs(table: &DeclareTable) -> Vec<(&str, &str)> {
        table
            .declarations
            .iter()
            .map(|d| {
                let ExportName::Name(export) = &d.export else {
                    panic!("an ordinal alias has no place in a plain (library, export) pair");
                };
                (d.library.as_str(), export.as_str())
            })
            .collect()
    }

    #[test]
    fn the_corpus_file_declares_one_external_import() {
        let table = declare_table(MANDELBROT);
        assert_eq!(as_pairs(&table), vec![("gdi32", "SetPixelV")]);
        assert!(table.defects().is_empty());
    }

    /// Eight of the nine entries are external. The other one, entry 0, is
    /// `dwEntryType == 6`, resolved inside the runtime, and it must not
    /// appear here.
    #[test]
    fn eight_of_nine_grayscale_entries_are_external_and_the_internal_one_is_skipped() {
        let table = declare_table(GRAYSCALE);
        assert_eq!(
            as_pairs(&table),
            vec![
                ("gdi32", "StretchDIBits"),
                ("gdi32", "GetDIBits"),
                ("gdi32", "SetStretchBltMode"),
                ("gdi32", "GetObjectA"),
                ("kernel32", "lstrlenW"),
                ("comdlg32", "CommDlgExtendedError"),
                ("comdlg32", "GetSaveFileNameW"),
                ("comdlg32", "GetOpenFileNameW"),
            ],
            "a walk that reorders, drops, or adds an entry must fail here with both lists \
             printed"
        );
        assert_eq!(
            table.declarations.len(),
            8,
            "entry 0 is internal (dwEntryType == 6) and must not be dereferenced"
        );
        assert!(table.defects().is_empty());
    }

    #[test]
    fn the_smallest_corpus_program_declares_no_external_import_and_is_not_refused() {
        let table = declare_table(LOCK_WORK_STATION);
        assert!(table.declarations.is_empty());
        assert!(table.entries.is_empty());
        assert!(table.defects().is_empty());
    }

    /// `EZTW32.dll` carries its extension and `kernel32` does not, inside the
    /// same program. Neither is normalised.
    #[test]
    fn a_library_name_is_given_verbatim_extension_and_all() {
        let table = declare_table(VB_SCANNER_SUPPORT);
        let pairs = as_pairs(&table);
        assert!(
            pairs.contains(&("EZTW32.dll", "TWAIN_IsAvailable")),
            "{pairs:?}"
        );
        assert!(pairs.contains(&("kernel32", "LoadLibraryA")), "{pairs:?}");
        assert!(table.defects().is_empty());
    }

    /// Gives the absolute file offset of a field inside one `Declare` table
    /// entry.
    ///
    /// The route is the parser's own: the header, then `ProjectInfo`, then
    /// the external table address, then the entry stride. Nothing searches
    /// for a byte pattern.
    fn declare_entry_field_offset(data: &[u8], entry_index: u32, field: u32) -> usize {
        let image = PeImage::parse(data).unwrap();
        let info = ProjectInfo::read(&image, project_data_va(data)).unwrap();
        let table = image.region_at_va(info.lp_external_table).unwrap();
        let entry = table.subregion(Off::new(entry_index * 8), 8).unwrap();
        let at = entry.file_offset(Off::new(field)).unwrap();
        usize::try_from(at.get()).unwrap()
    }

    /// Copies `MANDELBROT` and writes a `u32` into its one `Declare` entry.
    fn with_mandelbrot_entry_u32(field: u32, value: u32) -> Vec<u8> {
        let at = declare_entry_field_offset(MANDELBROT, 0, field);
        let mut out = MANDELBROT.to_vec();
        assert_ne!(
            out[at..at + 4],
            value.to_le_bytes(),
            "the fixture writes the value the field already holds, so it proves nothing"
        );
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        out
    }

    /// An undocumented entry type is skipped, and the byte offset and the
    /// value found are both named in a recoverable defect.
    #[test]
    fn an_entry_type_that_is_neither_six_nor_seven_is_skipped_and_flagged() {
        let bytes = with_mandelbrot_entry_u32(0x00, 99);
        let table = declare_table(&bytes);
        assert!(
            table.declarations.is_empty(),
            "an undocumented entry type must not be dereferenced as a library import"
        );
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert!(matches!(
            defect.kind,
            DefectKind::CountMismatch {
                count: 99,
                expected: 7,
                ..
            }
        ));
        assert_eq!(
            defect.site.offset,
            u32::try_from(declare_entry_field_offset(MANDELBROT, 0, 0x00)).unwrap()
        );
    }

    /// A descriptor address in no section produces a defect, and the walk
    /// does not stop: it is the only entry `Mandelbrot.exe` has, so this
    /// proves the walk finishes rather than that another entry survives.
    /// [`eight_of_nine_grayscale_entries_are_external_and_the_internal_one_is_skipped`]
    /// is the test that a real defect on one entry leaves the others intact,
    /// because `Grayscale.exe` has more than one.
    #[test]
    fn a_descriptor_address_in_no_section_produces_a_defect_and_the_walk_finishes() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        let bytes = with_mandelbrot_entry_u32(0x04, nowhere);
        let table = declare_table(&bytes);
        assert!(table.declarations.is_empty());
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert!(matches!(
            defect.kind,
            DefectKind::ItemAddressUnmapped { va, .. } if va == nowhere
        ));
        assert_eq!(defect.site.field, "lpImportDescriptor");
    }

    /// The two addresses at the start of a descriptor: the library name,
    /// then the export name.
    type Pair = (u32, u32);

    /// Reads a `u32` at an absolute file offset, by hand.
    fn u32_at_offset(data: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
    }

    /// Gives the file offset of a descriptor, by hand from its address.
    fn descriptor_offset(data: &[u8], descriptor: u32) -> usize {
        let image = PeImage::parse(data).unwrap();
        usize::try_from(image.va_to_off(Va::new(descriptor)).unwrap().get()).unwrap()
    }

    /// Gives the relative virtual address of `va`, by hand.
    fn rva_of(data: &[u8], va: u32) -> u32 {
        va - PeImage::parse(data).unwrap().image_base()
    }

    /// Gives the relative virtual address of a field inside one `Declare`
    /// table entry, by hand from the table address that `ProjectInfo` holds.
    fn declare_entry_field_rva(data: &[u8], entry_index: u32, field: u32) -> u32 {
        let table = project_info(data).unwrap().lp_external_table.get();
        rva_of(data, table) + entry_index * 8 + field
    }

    /// Gives the entry type and the descriptor address of one entry, and the
    /// two addresses at the start of its descriptor, all read by hand.
    fn entry_by_hand(data: &[u8], index: u32) -> (u32, u32, Pair) {
        let entry_type = u32_at_offset(data, declare_entry_field_offset(data, index, 0x00));
        let descriptor = u32_at_offset(data, declare_entry_field_offset(data, index, 0x04));
        let at = descriptor_offset(data, descriptor);
        (
            entry_type,
            descriptor,
            (u32_at_offset(data, at), u32_at_offset(data, at + 4)),
        )
    }

    /// Gives the two addresses that the reader kept for one entry.
    fn kept_pair(table: &DeclareTable, index: usize) -> Option<Pair> {
        table.entries[index]
            .descriptor
            .map(|pair| (pair.lp_dll_name.get(), pair.lp_api_name.get()))
    }

    /// Every entry is kept in table order, of every type. Entry 0 is the
    /// internal one, so it keeps no descriptor.
    ///
    /// The literal values were measured on 2026-09-16, and the test reads
    /// each of them again by hand.
    #[test]
    fn grayscale_keeps_its_nine_entries_in_table_order_and_the_descriptor_of_each_external_one() {
        let measured: [(u32, u32, Option<Pair>); 9] = [
            (6, 0x0040_3758, None),
            (7, 0x0040_35FC, Some((0x0040_350C, 0x0040_35EC))),
            (7, 0x0040_35B4, Some((0x0040_350C, 0x0040_35A8))),
            (7, 0x0040_3570, Some((0x0040_350C, 0x0040_355C))),
            (7, 0x0040_3524, Some((0x0040_350C, 0x0040_3518))),
            (7, 0x0040_329C, Some((0x0040_3280, 0x0040_3290))),
            (7, 0x0040_3248, Some((0x0040_3188, 0x0040_3230))),
            (7, 0x0040_31F8, Some((0x0040_3188, 0x0040_31E4))),
            (7, 0x0040_31AC, Some((0x0040_3188, 0x0040_3198))),
        ];
        let table = declare_table(GRAYSCALE);
        assert_eq!(table.entries.len(), measured.len());
        for (index, (entry_type, descriptor, pair)) in measured.into_iter().enumerate() {
            let (type_by_hand, descriptor_by_hand, pair_by_hand) =
                entry_by_hand(GRAYSCALE, u32::try_from(index).unwrap());
            assert_eq!(
                (type_by_hand, descriptor_by_hand),
                (entry_type, descriptor),
                "entry {index}"
            );
            if let Some(pair) = pair {
                assert_eq!(pair_by_hand, pair, "entry {index}");
            }

            let entry = table.entries[index];
            assert_eq!(entry.dw_entry_type, entry_type, "entry {index}");
            assert_eq!(
                entry.lp_import_descriptor,
                Va::new(descriptor),
                "entry {index}"
            );
            assert_eq!(kept_pair(&table, index), pair, "entry {index}");
        }
    }

    #[test]
    fn mandelbrot_keeps_its_one_external_entry_and_its_descriptor() {
        let (entry_type, descriptor, pair) = entry_by_hand(MANDELBROT, 0);
        assert_eq!(
            (entry_type, descriptor, pair),
            (7, 0x0040_1CDC, (0x0040_1CC4, 0x0040_1CD0))
        );
        let table = declare_table(MANDELBROT);
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].dw_entry_type, 7);
        assert_eq!(table.entries[0].lp_import_descriptor, Va::new(descriptor));
        assert_eq!(kept_pair(&table, 0), Some(pair));
    }

    /// The entry is kept although its descriptor is not.
    #[test]
    fn an_external_entry_whose_descriptor_is_in_no_section_is_kept_with_no_descriptor() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        let bytes = with_mandelbrot_entry_u32(0x04, nowhere);
        let table = declare_table(&bytes);
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].dw_entry_type, 7);
        assert_eq!(table.entries[0].lp_import_descriptor, Va::new(nowhere));
        assert_eq!(table.entries[0].descriptor, None);
    }

    /// The descriptor keeps both addresses as the file holds them, although
    /// the declaration is lost.
    #[test]
    fn an_external_entry_whose_library_name_is_in_no_section_keeps_both_addresses() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        let (_type, descriptor, (dll_name, api_name)) = entry_by_hand(MANDELBROT, 0);
        assert_ne!(dll_name, nowhere);
        let at = descriptor_offset(MANDELBROT, descriptor);
        let mut bytes = MANDELBROT.to_vec();
        bytes[at..at + 4].copy_from_slice(&nowhere.to_le_bytes());

        let table = declare_table(&bytes);
        assert!(table.declarations.is_empty());
        assert_eq!(table.defects().len(), 1);
        assert_eq!(table.defects()[0].site.field, "lpDllName");
        assert_eq!(table.entries.len(), 1);
        assert_eq!(kept_pair(&table, 0), Some((nowhere, api_name)));
    }

    /// Copies `MANDELBROT` and writes a `u32` into the descriptor that its
    /// one `Declare` entry names.
    fn with_mandelbrot_descriptor_u32(field: usize, value: u32) -> Vec<u8> {
        let (_type, descriptor, _pair) = entry_by_hand(MANDELBROT, 0);
        let at = descriptor_offset(MANDELBROT, descriptor) + field;
        let mut out = MANDELBROT.to_vec();
        assert_ne!(
            out[at..at + 4],
            value.to_le_bytes(),
            "the fixture writes the value the field already holds, so it proves nothing"
        );
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        out
    }

    /// The defect names the pointer that held the address, as
    /// `ItemAddressUnmapped` documents. For `lpImportDescriptor`, that is
    /// `0x04` of the entry, and not the first byte of the entry.
    #[test]
    fn an_unmapped_descriptor_address_is_reported_at_lp_import_descriptor_in_the_entry() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        let at = u32::try_from(declare_entry_field_offset(MANDELBROT, 0, 0x04)).unwrap();
        let table = declare_table(&with_mandelbrot_entry_u32(0x04, nowhere));
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(
            defect.kind,
            DefectKind::ItemAddressUnmapped {
                offset: at,
                va: nowhere,
            }
        );
        assert_eq!(defect.site.offset, at);
        assert_eq!(defect.site.structure, "DeclareTableEntry");
        assert_eq!(defect.site.field, "lpImportDescriptor");
        // The site gives the address of the pointer's own byte. The kind
        // gives the address that the pointer holds.
        assert_eq!(
            defect.site.rva,
            Some(declare_entry_field_rva(MANDELBROT, 0, 0x04))
        );
    }

    /// Gives the virtual address where the mapped bytes of the section that
    /// holds `va` end.
    fn section_end(data: &[u8], va: u32) -> u32 {
        let image = PeImage::parse(data).unwrap();
        let section = image
            .section_for(Rva::new(va - image.image_base()))
            .unwrap();
        image.image_base() + section.virtual_address.get() + section.mapped_len()
    }

    /// The descriptor address maps, and its section ends 4 bytes later. The
    /// file holds 4 of the 8 bytes that the reader reads, so the address is
    /// not in "no section", and the defect says what is wrong.
    #[test]
    fn a_descriptor_that_its_section_cuts_short_is_reported_as_cut_short() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let (_type, descriptor, _pair) = entry_by_hand(MANDELBROT, 0);
        let cut = section_end(MANDELBROT, descriptor) - 4;
        assert_eq!(image.region_at_va(Va::new(cut)).unwrap().len(), 4);
        let at = u32::try_from(declare_entry_field_offset(MANDELBROT, 0, 0x04)).unwrap();

        let table = declare_table(&with_mandelbrot_entry_u32(0x04, cut));
        assert!(table.declarations.is_empty());
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].descriptor, None);
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(
            defect.kind,
            DefectKind::ItemCutShort {
                offset: at,
                va: cut,
                len: 8,
            }
        );
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert_eq!(defect.site.offset, at);
        assert_eq!(
            defect.site.rva,
            Some(declare_entry_field_rva(MANDELBROT, 0, 0x04))
        );
        assert_eq!(defect.site.structure, "DeclareTableEntry");
        assert_eq!(defect.site.field, "lpImportDescriptor");
    }

    /// Copies `data` and writes `bytes` at the virtual address `va`.
    fn with_bytes_at_va(data: &[u8], va: u32, bytes: &[u8]) -> Vec<u8> {
        let at = descriptor_offset(data, va);
        let mut out = data.to_vec();
        out[at..at + bytes.len()].copy_from_slice(bytes);
        out
    }

    /// The library name address maps 4 bytes before the end of its section,
    /// and those 4 bytes hold no NUL. The defect gives the text and the 4
    /// bytes that the reader searched, and its site is `lpDllName`.
    #[test]
    fn a_library_name_that_its_section_ends_before_a_nul_is_reported_as_unterminated_text() {
        let (_type, descriptor, (dll_name, _api_name)) = entry_by_hand(MANDELBROT, 0);
        let text = section_end(MANDELBROT, dll_name) - 4;
        let text_at = descriptor_offset(MANDELBROT, text);
        assert!(text_at > descriptor_offset(MANDELBROT, descriptor) + 24);
        let mut bytes = with_bytes_at_va(MANDELBROT, text, b"AAAA");
        let pointer_at = descriptor_offset(MANDELBROT, descriptor);
        bytes[pointer_at..pointer_at + 4].copy_from_slice(&text.to_le_bytes());

        let table = declare_table(&bytes);
        assert!(table.declarations.is_empty());
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(
            defect.kind,
            DefectKind::NoNulTerminator {
                offset: u32::try_from(text_at).unwrap(),
                limit: 4,
            }
        );
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert_eq!(defect.site.offset, u32::try_from(pointer_at).unwrap());
        assert_eq!(defect.site.rva, Some(rva_of(MANDELBROT, descriptor)));
        assert_eq!(defect.site.structure, "DeclareDescriptor");
        assert_eq!(defect.site.field, "lpDllName");
    }

    /// The export name address maps well before the end of its section, and
    /// no NUL comes in the next `NAME_MAX` bytes. The reader searched all
    /// `NAME_MAX` of them.
    #[test]
    fn an_export_name_with_no_nul_in_the_bytes_searched_is_reported_as_unterminated_text() {
        let (_type, descriptor, (_dll_name, api_name)) = entry_by_hand(MANDELBROT, 0);
        let text = section_end(MANDELBROT, api_name) - 300;
        let text_at = descriptor_offset(MANDELBROT, text);
        assert!(text_at > descriptor_offset(MANDELBROT, descriptor) + 24);
        let mut bytes = with_bytes_at_va(MANDELBROT, text, &[b'A'; 300]);
        let pointer_at = descriptor_offset(MANDELBROT, descriptor) + 4;
        bytes[pointer_at..pointer_at + 4].copy_from_slice(&text.to_le_bytes());

        let table = declare_table(&bytes);
        assert!(table.declarations.is_empty());
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(
            defect.kind,
            DefectKind::NoNulTerminator {
                offset: u32::try_from(text_at).unwrap(),
                limit: NAME_MAX,
            }
        );
        assert_eq!(NAME_MAX, 260);
        assert_eq!(defect.site.offset, u32::try_from(pointer_at).unwrap());
        assert_eq!(defect.site.rva, Some(rva_of(MANDELBROT, descriptor) + 4));
        assert_eq!(defect.site.structure, "DeclareDescriptor");
        assert_eq!(defect.site.field, "lpApiName");
    }

    /// `lpDllName` is in the descriptor, so the defect names the descriptor.
    #[test]
    fn an_unmapped_library_name_is_reported_at_lp_dll_name_in_the_descriptor() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        let (_type, descriptor, _pair) = entry_by_hand(MANDELBROT, 0);
        let at = u32::try_from(descriptor_offset(MANDELBROT, descriptor)).unwrap();
        let table = declare_table(&with_mandelbrot_descriptor_u32(0x00, nowhere));
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(
            defect.kind,
            DefectKind::ItemAddressUnmapped {
                offset: at,
                va: nowhere,
            }
        );
        assert_eq!(defect.site.offset, at);
        assert_eq!(defect.site.structure, "DeclareDescriptor");
        assert_eq!(defect.site.field, "lpDllName");
        assert_eq!(defect.site.rva, Some(rva_of(MANDELBROT, descriptor)));
    }

    /// `lpApiName` is at `0x04` of the descriptor.
    #[test]
    fn an_unmapped_export_name_is_reported_at_lp_api_name_in_the_descriptor() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        let (_type, descriptor, _pair) = entry_by_hand(MANDELBROT, 0);
        let at = u32::try_from(descriptor_offset(MANDELBROT, descriptor) + 4).unwrap();
        let table = declare_table(&with_mandelbrot_descriptor_u32(0x04, nowhere));
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(
            defect.kind,
            DefectKind::ItemAddressUnmapped {
                offset: at,
                va: nowhere,
            }
        );
        assert_eq!(defect.site.offset, at);
        assert_eq!(defect.site.structure, "DeclareDescriptor");
        assert_eq!(defect.site.field, "lpApiName");
        assert_eq!(defect.site.rva, Some(rva_of(MANDELBROT, descriptor) + 4));
    }

    /// Only an entry of type 7 names a descriptor of the shape that this
    /// reader reads, so an undocumented type keeps no descriptor.
    #[test]
    fn an_entry_of_an_undocumented_type_is_kept_with_no_descriptor() {
        let (_type, descriptor, _pair) = entry_by_hand(MANDELBROT, 0);
        let bytes = with_mandelbrot_entry_u32(0x00, 99);
        let table = declare_table(&bytes);
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].dw_entry_type, 99);
        assert_eq!(table.entries[0].lp_import_descriptor, Va::new(descriptor));
        assert_eq!(table.entries[0].descriptor, None);
    }

    /// Copies `MANDELBROT` and writes a `u32` into its `ProjectInfo`.
    fn with_mandelbrot_project_info_u32(data: &[u8], field: u32, value: u32) -> Vec<u8> {
        let at = project_info_field_offset(MANDELBROT, field);
        let mut out = data.to_vec();
        assert_ne!(
            out[at..at + 4],
            value.to_le_bytes(),
            "the fixture writes the value the field already holds, so it proves nothing"
        );
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        out
    }

    /// The count says that the table holds one entry, and the table maps
    /// nowhere. The whole table is lost, and the defect says so at
    /// `lpExternalTable`.
    ///
    /// `ProjectInfo` keeps its file offset and not its address, so the site
    /// gives no address. The address that the pointer holds is in the kind.
    #[test]
    fn a_declare_table_whose_address_maps_nowhere_gives_one_defect_at_lp_external_table() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        let bytes = with_mandelbrot_project_info_u32(MANDELBROT, 0x234, nowhere);
        let at = u32::try_from(project_info_field_offset(MANDELBROT, 0x234)).unwrap();

        let table = declare_table(&bytes);
        assert!(table.declarations.is_empty());
        assert!(table.entries.is_empty());
        assert_eq!(
            table.defects(),
            [Defect {
                site: Site {
                    offset: at,
                    rva: None,
                    structure: "ProjectInfo",
                    field: "lpExternalTable",
                },
                kind: DefectKind::ItemAddressUnmapped {
                    offset: at,
                    va: nowhere,
                },
            }]
        );
        assert_eq!(table.defects()[0].kind.severity(), Severity::Recoverable);
    }

    /// Each `Declare` table of this corpus sits at an address that is equal
    /// to its file offset, so a corpus test cannot tell the two apart. Here
    /// the table starts at file offset `0x400` and at address `0x1000`.
    ///
    /// Each case writes an address in no section into one pointer. The site
    /// gives the file offset and the address of that pointer. The kind gives
    /// the address that the pointer holds.
    #[test]
    fn a_declare_defect_gives_the_address_of_its_pointer_and_not_its_file_offset() {
        const NOWHERE: u32 = 0x0130_0000;
        const DESCRIPTOR: u32 = 0x0040_1008;
        const DLL_NAME: u32 = 0x0040_1020;
        const API_NAME: u32 = 0x0040_1028;
        // One entry of type 7 at 0x00, its descriptor at 0x08, and the two
        // names after the 24 bytes of the descriptor.
        let payload = |descriptor: u32, dll_name: u32, api_name: u32| {
            let mut out = vec![0_u8; 0x30];
            out[0x00..0x04].copy_from_slice(&7_u32.to_le_bytes());
            out[0x04..0x08].copy_from_slice(&descriptor.to_le_bytes());
            out[0x08..0x0C].copy_from_slice(&dll_name.to_le_bytes());
            out[0x0C..0x10].copy_from_slice(&api_name.to_le_bytes());
            out[0x20..0x24].copy_from_slice(b"Lib\0");
            out[0x28..0x2C].copy_from_slice(b"Api\0");
            out
        };
        let read = |payload: &[u8]| {
            let (bytes, table) = a_synthetic_pe_image(payload);
            assert_eq!(table, Va::new(0x0040_1000));
            let image = PeImage::parse(&bytes).unwrap();
            assert_eq!(image.va_to_off(table), Some(Off::new(0x400)));
            let info = ProjectInfo {
                file_offset: Off::new(0),
                dw_version: 0,
                lp_object_table: Va::new(0),
                lp_native_code: 0,
                lp_external_table: table,
                dw_external_count: 1,
            };
            DeclareTable::read(&image, &info)
        };

        let whole = read(&payload(DESCRIPTOR, DLL_NAME, API_NAME));
        assert!(whole.defects().is_empty(), "{:?}", whole.defects());
        assert_eq!(as_pairs(&whole), [("Lib", "Api")]);

        let cases = [
            (
                payload(NOWHERE, DLL_NAME, API_NAME),
                "lpImportDescriptor",
                0x04,
            ),
            (payload(DESCRIPTOR, NOWHERE, API_NAME), "lpDllName", 0x08),
            (payload(DESCRIPTOR, DLL_NAME, NOWHERE), "lpApiName", 0x0C),
        ];
        for (bytes, field, at) in cases {
            let table = read(&bytes);
            assert!(table.declarations.is_empty(), "{field}");
            assert_eq!(table.defects().len(), 1, "{field}");
            let defect = &table.defects()[0];
            assert_eq!(defect.site.field, field);
            assert_eq!(defect.site.offset, 0x400 + at, "{field}");
            assert_eq!(defect.site.rva, Some(0x1000 + at), "{field}");
            assert_eq!(
                defect.kind,
                DefectKind::ItemAddressUnmapped {
                    offset: 0x400 + at,
                    va: NOWHERE,
                },
                "{field}"
            );
        }
    }

    /// A zero count never reads the table address, so an address that maps
    /// nowhere is not a defect.
    #[test]
    fn a_zero_count_gives_no_defect_whatever_the_table_address_holds() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        let bytes = with_mandelbrot_project_info_u32(MANDELBROT, 0x238, 0);
        let bytes = with_mandelbrot_project_info_u32(&bytes, 0x234, nowhere);

        let table = declare_table(&bytes);
        assert!(table.declarations.is_empty());
        assert!(table.entries.is_empty());
        assert!(table.defects().is_empty(), "{:?}", table.defects());
    }

    /// A bound `dwExternalCount` still resolves every entry the region can
    /// hold, and it does not panic on the ones it cannot.
    ///
    /// This corpus never exercises an implausible count: the largest is 26.
    /// The fixture inflates `Mandelbrot.exe`'s own count by a wide margin,
    /// which the file's own external table region cannot hold.
    #[test]
    fn an_implausible_external_count_is_clamped_and_flagged() {
        let at = {
            let image = PeImage::parse(MANDELBROT).unwrap();
            let window = image.region_at_va(project_data_va(MANDELBROT)).unwrap();
            usize::try_from(window.file_offset(Off::new(0x238)).unwrap().get()).unwrap()
        };
        let mut bytes = MANDELBROT.to_vec();
        bytes[at..at + 4].copy_from_slice(&0xFFFF_u32.to_le_bytes());
        let table = declare_table(&bytes);
        assert_eq!(
            table.declarations.len(),
            1,
            "the one real entry must still resolve"
        );
        assert!(
            table
                .defects()
                .iter()
                .any(|d| matches!(d.kind, DefectKind::ImplausibleCount { count: 0xFFFF, .. }))
        );

        // The defect must name the byte this test patched, which is
        // dwExternalCount itself, and not the start of the import table.
        let defect = table
            .defects()
            .iter()
            .find(|d| matches!(d.kind, DefectKind::ImplausibleCount { .. }))
            .unwrap();
        assert_eq!(defect.site.structure, "ProjectInfo");
        assert_eq!(defect.site.field, "dwExternalCount");
        assert_eq!(usize::try_from(defect.site.offset).unwrap(), at);
        let DefectKind::ImplausibleCount { max, .. } = defect.kind else {
            panic!("the defect found above is an ImplausibleCount");
        };
        assert_eq!(
            u32::try_from(table.entries.len()).unwrap(),
            max,
            "the table keeps each whole entry that its region holds"
        );
        // Site::rva is the address the offset came from. This offset did not
        // come from lpExternalTable, so the defect must not give that address.
        assert_eq!(defect.site.rva, None);
    }

    /// A count whose size in bytes leaves a `u32` is bounded and flagged
    /// too.
    ///
    /// `0x2000_0000 * 8` leaves a `u32`. A check that multiplied the count
    /// first had no size to compare, so it raised no defect, while the loop
    /// still stopped at the end of the region.
    #[test]
    fn a_declare_count_whose_size_in_bytes_leaves_a_u32_is_clamped_and_flagged() {
        let (at, holds) = {
            let image = PeImage::parse(MANDELBROT).unwrap();
            let window = image.region_at_va(project_data_va(MANDELBROT)).unwrap();
            let table = image
                .region_at_va(project_info(MANDELBROT).unwrap().lp_external_table)
                .unwrap();
            (
                usize::try_from(window.file_offset(Off::new(0x238)).unwrap().get()).unwrap(),
                table.len().checked_div(8).unwrap(),
            )
        };
        let unpatched = declare_table(MANDELBROT).declarations;
        assert_eq!(unpatched.len(), 1);
        for count in [0x2000_0000_u32, 0xFFFF_FFFF] {
            let mut bytes = MANDELBROT.to_vec();
            bytes[at..at + 4].copy_from_slice(&count.to_le_bytes());
            let table = declare_table(&bytes);
            let clamps: Vec<_> = table
                .defects()
                .iter()
                .filter(|d| matches!(d.kind, DefectKind::ImplausibleCount { .. }))
                .collect();
            assert_eq!(clamps.len(), 1, "count {count:#x}: {clamps:?}");
            assert_eq!(
                clamps[0].kind,
                DefectKind::ImplausibleCount {
                    offset: u32::try_from(at).unwrap(),
                    count,
                    max: holds,
                }
            );
            assert_eq!(clamps[0].site.structure, "ProjectInfo");
            assert_eq!(clamps[0].site.field, "dwExternalCount");
            assert_eq!(u32::try_from(table.entries.len()).unwrap(), holds);
            assert_eq!(
                table.declarations.first(),
                unpatched.first(),
                "the one real entry must still resolve"
            );
        }
    }

    #[test]
    fn project_info_keeps_the_file_offset_of_its_own_first_byte() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let window = image.region_at_va(project_data_va(MANDELBROT)).unwrap();
        let info = ProjectInfo::read(&image, project_data_va(MANDELBROT)).unwrap();
        assert_eq!(Some(info.file_offset), window.file_offset(Off::new(0)));
    }

    #[test]
    fn the_name_marker_the_arguments_marker_and_the_scope_marker_each_name_what_is_missing() {
        assert!(Declaration::NAME_MARKER.contains("procedure name"));
        assert!(Declaration::NAME_MARKER.contains("Alias"));
        assert!(Declaration::ARGUMENTS_MARKER.contains("argument"));
        assert!(Declaration::SCOPE_MARKER.contains("Public"));
        assert!(Declaration::SCOPE_MARKER.contains("Private"));
        assert!(Declaration::SCOPE_MARKER.contains("module"));
    }

    #[test]
    fn a_hash_then_all_decimal_digits_is_an_inferred_ordinal() {
        assert_eq!(
            parse_export_name("#123".to_string()),
            ExportName::OrdinalInferred(123)
        );
    }

    /// The character after the digits is not a digit, so this is a plain
    /// name and not ordinal 12. A lazy parse that stopped at the first
    /// non-digit would turn this into `OrdinalInferred(12)`.
    #[test]
    fn a_hash_then_a_non_decimal_tail_is_a_plain_name_and_not_an_ordinal() {
        assert_eq!(
            parse_export_name("#12a".to_string()),
            ExportName::Name("#12a".to_string())
        );
    }

    #[test]
    fn a_bare_hash_with_no_digits_is_a_plain_name() {
        assert_eq!(
            parse_export_name("#".to_string()),
            ExportName::Name("#".to_string())
        );
    }

    #[test]
    fn a_name_with_no_leading_hash_is_a_plain_name() {
        assert_eq!(
            parse_export_name("SetPixelV".to_string()),
            ExportName::Name("SetPixelV".to_string())
        );
    }

    /// No corpus program uses an ordinal alias anywhere. A script run over
    /// every `Declare`-bearing program in this repository, this session,
    /// confirms it: 220 external entries, zero of them ordinal.
    #[test]
    fn no_corpus_program_recovers_an_ordinal_alias() {
        for data in [MANDELBROT, GRAYSCALE, VB_SCANNER_SUPPORT] {
            let table = declare_table(data);
            for decl in &table.declarations {
                assert!(
                    matches!(decl.export, ExportName::Name(_)),
                    "no sample anywhere confirms the ordinal encoding; this corpus must not \
                     manufacture one: {decl:?}"
                );
            }
        }
    }

    /// Walks the external component table out of a byte slice, reached from
    /// the header, never from `ProjectInfo`.
    fn component_table(data: &[u8]) -> ComponentTable {
        let image = PeImage::parse(data).unwrap();
        let hdr = header_region(&image).unwrap();
        let header = VbHeader::read(&hdr).unwrap();
        ComponentTable::read(&image, header.lp_external_table, header.w_external_count)
    }

    /// The `.vbp` beside `Server.exe` declares
    /// `Object={248DD890-BB45-11CF-9ABC-0080C7E7B78D}#1.0#0; MSWINSCK.OCX`.
    /// Plan 03-08 measured `GUIDoffset`'s own textual GUID as
    /// `2c49f800-c2dd-11cf-9ad6-0080c7e7b78d`, byte for byte, confirmed
    /// identically in `SubReality_WinsockSample.exe` too. Plan 03-16
    /// measured the sixteen byte binary GUID at `oUuid` (entry offset
    /// `0x04`) as well: `248DD896-BB45-11CF-9ABC-0080C7E7B78D`, differing
    /// from the `.vbp`'s own declared value by one byte, the low byte of
    /// `Data1`. A whole-entry search of six encodings (the sixteen byte
    /// binary layout in both field orders, and plain text and sixteen bit
    /// text in both cases) never found the declared value
    /// `248DD890-BB45-11CF-9ABC-0080C7E7B78D` anywhere in this entry, in any
    /// of the three corpus programs that hold one. `oUuid`, not
    /// `GUIDoffset`, is the field `vb/ocx.rs::join_component` reports, with
    /// an honest caveat: see [`Component::ouuid_text`]'s own doc comment.
    #[test]
    fn the_one_component_server_exe_declares_resolves_all_three_strings() {
        let table = component_table(SERVER);
        assert_eq!(table.components.len(), 1);
        let component = &table.components[0];
        assert_eq!(component.file_name, "MSWINSCK.OCX");
        assert_eq!(component.library, "MSWinsockLib.Winsock");
        assert_eq!(component.name, "Winsock");
        assert_eq!(component.guid_length, 72);
        assert_eq!(
            component.guid_text.as_deref(),
            Some("2c49f800-c2dd-11cf-9ad6-0080c7e7b78d"),
            "measured this session at GUIDoffset; see this test's own doc comment for why \
             this differs from the .vbp's Object= line"
        );
        assert_eq!(
            component.ouuid_text.as_deref(),
            Some("248DD896-BB45-11CF-9ABC-0080C7E7B78D"),
            "measured this session at oUuid; differs from the .vbp's declared identifier by \
             one byte, the low byte of Data1"
        );
        assert!(table.defects().is_empty());
    }

    #[test]
    fn a_guid_length_of_minus_one_gives_no_guid_text_and_no_defect() {
        let payload = build_component_entry_with_guid(
            "None.ocx",
            "NoneLib.None",
            "None",
            -1,
            "this text is never read when guid_length is -1",
        );
        let (bytes, va) = a_synthetic_pe_image(&payload);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ComponentTable::read(&image, va, 1);

        assert_eq!(table.components.len(), 1, "{:?}", table.defects());
        assert_eq!(table.components[0].guid_text, None);
        assert!(table.defects().is_empty());
    }

    #[test]
    fn a_guid_length_of_forty_gives_a_defect_naming_the_value_and_no_guid_text() {
        let payload = build_component_entry_with_guid(
            "Odd.ocx",
            "OddLib.Odd",
            "Odd",
            40,
            "this text is never read when guid_length is neither -1 nor 72",
        );
        let (bytes, va) = a_synthetic_pe_image(&payload);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ComponentTable::read(&image, va, 1);

        assert_eq!(table.components.len(), 1, "{:?}", table.defects());
        assert_eq!(table.components[0].guid_text, None);
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Tolerated);
        assert!(matches!(
            defect.kind,
            DefectKind::GuidLengthUnexpected { value: 40, .. }
        ));
        let message = format!("{}", defect.kind);
        let wanted = "40";
        assert!(message.find(wanted).is_some(), "{message}");
    }

    /// A synthetic fixture, proving the UTF-16 decode itself against a
    /// hand-written GUID rather than only against the real bytes
    /// [`the_one_component_server_exe_declares_resolves_all_three_strings`]
    /// already proves.
    #[test]
    fn a_guid_length_of_seventy_two_decodes_thirty_six_utf16_characters() {
        let payload = build_component_entry_with_guid(
            "Synthetic.ocx",
            "SyntheticLib.Synthetic",
            "Synthetic",
            72,
            "248DD890-BB45-11CF-9ABC-0080C7E7B78D",
        );
        let (bytes, va) = a_synthetic_pe_image(&payload);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ComponentTable::read(&image, va, 1);

        assert_eq!(table.components.len(), 1, "{:?}", table.defects());
        assert_eq!(
            table.components[0].guid_text.as_deref(),
            Some("248DD890-BB45-11CF-9ABC-0080C7E7B78D"),
            "synthetic fixture"
        );
        assert!(table.defects().is_empty());
    }

    /// A synthetic fixture: `oUuid`'s own offset points close enough to the
    /// entry's own end that fewer than sixteen bytes remain there. The
    /// sixteen byte binary GUID is not decoded. The site is the `oUuid`
    /// field, at `0x04` of the entry, and the kind names the sixteen bytes
    /// and the end of the entry.
    #[test]
    fn an_o_uuid_offset_leaving_fewer_than_sixteen_bytes_gives_a_defect_and_no_ouuid_text() {
        let mut payload = build_component_entry("Short.ocx", "ShortLib.Short", "Short");
        let entry_len = u32::try_from(payload.len()).unwrap();
        // Five bytes remain after this offset: not enough for the sixteen
        // byte binary GUID.
        let too_close = entry_len - 5;
        payload[0x04..0x08].copy_from_slice(&too_close.to_le_bytes());
        let table = one_entry_table(&payload);

        assert_eq!(table.components.len(), 1, "{:?}", table.defects());
        assert_eq!(table.components[0].ouuid_text, None);
        assert_eq!(
            table.defects(),
            [Defect {
                site: Site {
                    offset: 0x404,
                    rva: Some(0x1004),
                    structure: "ExternalComponentEntry",
                    field: "oUuid",
                },
                kind: DefectKind::RunsPastEnd {
                    offset: 0x400 + too_close,
                    len: 16,
                    end: 0x400 + entry_len,
                },
            }]
        );
        assert_eq!(table.defects()[0].kind.severity(), Severity::Tolerated);
    }

    /// A synthetic fixture: `GUIDoffset` points close enough to the entry's
    /// own end that fewer than the 72 bytes that `GUIDlength` asks for remain
    /// there. The textual GUID is not decoded, and the component keeps its
    /// other fields. The site is the `GUIDoffset` field, at `0x1C` of the
    /// entry, and the kind names the 72 bytes and the end of the entry.
    #[test]
    fn a_guid_offset_leaving_fewer_than_seventy_two_bytes_gives_a_defect_and_no_guid_text() {
        let mut payload = build_component_entry("Short.ocx", "ShortLib.Short", "Short");
        let entry_len = u32::try_from(payload.len()).unwrap();
        assert_eq!(entry_u32(&payload, 0x20), 72);
        let too_close = entry_len - 10;
        payload[0x1C..0x20].copy_from_slice(&too_close.to_le_bytes());
        let table = one_entry_table(&payload);

        assert_eq!(table.components.len(), 1, "{:?}", table.defects());
        assert_eq!(table.components[0].guid_text, None);
        assert_eq!(table.components[0].name, "Short");
        assert_eq!(
            table.defects(),
            [Defect {
                site: Site {
                    offset: 0x41C,
                    rva: Some(0x101C),
                    structure: "ExternalComponentEntry",
                    field: "GUIDoffset",
                },
                kind: DefectKind::RunsPastEnd {
                    offset: 0x400 + too_close,
                    len: 72,
                    end: 0x400 + entry_len,
                },
            }]
        );
    }

    #[test]
    fn a_program_with_no_component_reference_gives_an_empty_list_without_refusing() {
        for data in [MANDELBROT, GRAYSCALE] {
            let table = component_table(data);
            assert!(table.components.is_empty());
            assert!(table.defects().is_empty());
        }
    }

    /// A minimal synthetic portable executable, built for this module's
    /// component table tests only.
    ///
    /// Every other test in this file exercises the real corpus. This exists
    /// for one reason: both corpus files vendored in this repository place
    /// every structure this module reads at a virtual address that is
    /// numerically equal to its file offset, so a test built against either
    /// one cannot tell a bug that reads the RVA in place of the file offset
    /// from a bug that does not exist. Here the two differ by `0xC00`.
    ///
    /// Plan 02-06's action text points at a synthetic builder held in
    /// `vb/header.rs`'s own `#[cfg(test)]` module. That function is private
    /// to that module and is not reachable from here, so this is an
    /// independent construction of the same PE shape, not a shared import.
    /// `AGENTS.md` favours this anyway: "build the state a test needs inside
    /// the test."
    fn a_synthetic_pe_image(payload: &[u8]) -> (Vec<u8>, Va) {
        const IMAGE_BASE: u32 = 0x0040_0000;
        const SECTION_RVA: u32 = 0x1000;
        const SECTION_OFF: usize = 0x400;
        const LFANEW: usize = 0x40;
        const OPTIONAL: usize = LFANEW + 24;
        const SECTION: usize = OPTIONAL + 224;

        let total = (SECTION_OFF + payload.len() + 0x100).max(0x800);
        let mut out = vec![0_u8; total];
        out[0] = b'M';
        out[1] = b'Z';
        out[0x3c..0x40].copy_from_slice(&u32::try_from(LFANEW).unwrap().to_le_bytes());
        out[LFANEW..LFANEW + 4].copy_from_slice(b"PE\0\0");
        out[LFANEW + 4..LFANEW + 6].copy_from_slice(&0x014c_u16.to_le_bytes());
        out[LFANEW + 6..LFANEW + 8].copy_from_slice(&1_u16.to_le_bytes());
        out[LFANEW + 20..LFANEW + 22].copy_from_slice(&224_u16.to_le_bytes());
        out[LFANEW + 22..LFANEW + 24].copy_from_slice(&0x0102_u16.to_le_bytes());

        out[OPTIONAL..OPTIONAL + 2].copy_from_slice(&0x010b_u16.to_le_bytes());
        out[OPTIONAL + 0x1c..OPTIONAL + 0x20].copy_from_slice(&IMAGE_BASE.to_le_bytes());
        out[OPTIONAL + 0x20..OPTIONAL + 0x24].copy_from_slice(&0x1000_u32.to_le_bytes());
        out[OPTIONAL + 0x24..OPTIONAL + 0x28].copy_from_slice(&0x200_u32.to_le_bytes());
        out[OPTIONAL + 0x38..OPTIONAL + 0x3c].copy_from_slice(&0x2000_u32.to_le_bytes());
        out[OPTIONAL + 0x3c..OPTIONAL + 0x40]
            .copy_from_slice(&u32::try_from(total).unwrap().to_le_bytes());
        out[OPTIONAL + 0x5c..OPTIONAL + 0x60].copy_from_slice(&16_u32.to_le_bytes());

        let section_bytes = u32::try_from(total - SECTION_OFF).unwrap();
        out[SECTION..SECTION + 8].copy_from_slice(b".text\0\0\0");
        out[SECTION + 8..SECTION + 12].copy_from_slice(&section_bytes.to_le_bytes());
        out[SECTION + 12..SECTION + 16].copy_from_slice(&SECTION_RVA.to_le_bytes());
        out[SECTION + 16..SECTION + 20].copy_from_slice(&section_bytes.to_le_bytes());
        out[SECTION + 20..SECTION + 24]
            .copy_from_slice(&u32::try_from(SECTION_OFF).unwrap().to_le_bytes());
        out[SECTION + 36..SECTION + 40].copy_from_slice(&0x6000_0020_u32.to_le_bytes());

        out[SECTION_OFF..SECTION_OFF + payload.len()].copy_from_slice(payload);

        let va = Va::new(IMAGE_BASE + SECTION_RVA);
        (out, va)
    }

    /// Builds one component entry: the fixed fields, then the three strings
    /// and a placeholder textual GUID, each written once and referenced by
    /// its own entry-relative offset. `GUIDlength` is `72`, matching the
    /// UTF-16 encoded placeholder text this writes.
    fn build_component_entry(file_name: &str, library: &str, name: &str) -> Vec<u8> {
        build_component_entry_with_guid(
            file_name,
            library,
            name,
            72,
            "00000000-0000-0000-0000-000000000000",
        )
    }

    /// Builds one component entry like [`build_component_entry`], but lets
    /// the caller choose `GUIDlength` and the textual GUID `decode_guid_text`
    /// reads at `GUIDoffset`. `guid_text` is encoded as UTF-16, the shape
    /// `STRUCTURES.md` section 7.3 gives for `GUIDlength == 72`; a test
    /// exercising a different `guid_length` passes whatever text it needs to
    /// prove that field's own handling, not this encoding.
    fn build_component_entry_with_guid(
        file_name: &str,
        library: &str,
        name: &str,
        guid_length: i32,
        guid_text: &str,
    ) -> Vec<u8> {
        let mut buf = vec![0_u8; 0x34];
        let mut pool = Vec::new();

        let mut guid_utf16 = Vec::new();
        for unit in guid_text.encode_utf16() {
            guid_utf16.extend_from_slice(&unit.to_le_bytes());
        }

        let place = |bytes: &[u8], pool: &mut Vec<u8>| -> u32 {
            let at = u32::try_from(0x34 + pool.len()).unwrap();
            pool.extend_from_slice(bytes);
            pool.push(0);
            at
        };

        let guid_off = place(&guid_utf16, &mut pool);
        let file_name_off = place(file_name.as_bytes(), &mut pool);
        let source_off = place(library.as_bytes(), &mut pool);
        let name_off = place(name.as_bytes(), &mut pool);

        buf[0x1C..0x20].copy_from_slice(&guid_off.to_le_bytes());
        buf[0x20..0x24].copy_from_slice(&guid_length.to_le_bytes());
        buf[0x28..0x2C].copy_from_slice(&file_name_off.to_le_bytes());
        buf[0x2C..0x30].copy_from_slice(&source_off.to_le_bytes());
        buf[0x30..0x34].copy_from_slice(&name_off.to_le_bytes());

        buf.extend_from_slice(&pool);
        let total_len = u32::try_from(buf.len()).unwrap();
        buf[0x00..0x04].copy_from_slice(&total_len.to_le_bytes());
        buf
    }

    /// Two entries, each with its own unique strings, back to back. Reading
    /// the second entry's strings correctly is only possible if every
    /// sub-offset is resolved relative to that entry's own start: this
    /// corpus's one distinct sample has no second entry to prove this
    /// against, so it is proved synthetically here.
    #[test]
    fn a_synthetic_two_entry_table_proves_offsets_are_relative_to_the_entry() {
        let mut payload = build_component_entry("First.ocx", "FirstLib.First", "First");
        payload.extend(build_component_entry(
            "Second.ocx",
            "SecondLib.Second",
            "Second",
        ));
        let (bytes, va) = a_synthetic_pe_image(&payload);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ComponentTable::read(&image, va, 2);

        assert_eq!(table.components.len(), 2, "{:?}", table.defects());
        assert_eq!(
            table.components[0],
            Component {
                file_name: "First.ocx".to_string(),
                library: "FirstLib.First".to_string(),
                name: "First".to_string(),
                guid_offset: table.components[0].guid_offset,
                guid_length: 72,
                guid_text: table.components[0].guid_text.clone(),
                o_uuid: table.components[0].o_uuid,
                ouuid_field_offset: table.components[0].ouuid_field_offset,
                ouuid_text: table.components[0].ouuid_text.clone(),
            }
        );
        assert_eq!(table.components[1].file_name, "Second.ocx");
        assert_eq!(table.components[1].library, "SecondLib.Second");
        assert_eq!(table.components[1].name, "Second");
        assert!(table.defects().is_empty());
    }

    #[test]
    fn a_declared_length_of_zero_stops_the_walk_with_a_recoverable_defect() {
        let mut payload = build_component_entry("Zero.ocx", "ZeroLib.Zero", "Zero");
        // Corrupt only the declared length of this one entry to zero, after
        // it was built correctly, so the fixture proves the guard and
        // nothing else.
        payload[0x00..0x04].copy_from_slice(&0_u32.to_le_bytes());
        let (bytes, va) = a_synthetic_pe_image(&payload);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ComponentTable::read(&image, va, 3);

        assert!(table.components.is_empty());
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert!(matches!(
            defect.kind,
            DefectKind::CountMismatch { count: 0, .. }
        ));
    }

    #[test]
    fn a_declared_length_past_the_end_of_the_region_stops_the_walk_with_a_recoverable_defect() {
        let mut payload = build_component_entry("Over.ocx", "OverLib.Over", "Over");
        let real_len = u32::try_from(payload.len()).unwrap();
        // A declared length far larger than what the region holds.
        payload[0x00..0x04].copy_from_slice(&(real_len + 10_000).to_le_bytes());
        let (bytes, va) = a_synthetic_pe_image(&payload);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ComponentTable::read(&image, va, 1);

        assert!(table.components.is_empty());
        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert!(matches!(
            defect.kind,
            DefectKind::ImplausibleCount { count, .. } if count == real_len + 10_000
        ));
    }

    /// Reads a table of one entry, built from `payload`, out of a synthetic
    /// image.
    fn one_entry_table(payload: &[u8]) -> ComponentTable {
        let (bytes, va) = a_synthetic_pe_image(payload);
        let image = PeImage::parse(&bytes).unwrap();
        ComponentTable::read(&image, va, 1)
    }

    /// Reads the `u32` at `at` of a built entry.
    fn entry_u32(payload: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(payload[at..at + 4].try_into().unwrap())
    }

    /// The fixed fields end at `0x34`. A declared length that is shorter
    /// gives no component and one defect at `StructLength`. The entry
    /// starts at file offset `0x400` and at address `0x1000`.
    #[test]
    fn a_declared_length_shorter_than_the_fixed_fields_gives_a_defect_at_struct_length() {
        for short in [0x04_u32, 0x33] {
            let mut payload = build_component_entry("Short.ocx", "ShortLib.Short", "Short");
            payload[0x00..0x04].copy_from_slice(&short.to_le_bytes());
            let table = one_entry_table(&payload);

            assert!(table.components.is_empty(), "{short:#x}");
            assert_eq!(
                table.defects(),
                [Defect {
                    site: Site {
                        offset: 0x400,
                        rva: Some(0x1000),
                        structure: "ExternalComponentEntry",
                        field: "StructLength",
                    },
                    kind: DefectKind::CountMismatch {
                        offset: 0x400,
                        count: short,
                        expected: 0x34,
                        other_field: "the fixed part of an ExternalComponentEntry",
                    },
                }],
                "{short:#x}"
            );
            assert_eq!(table.defects()[0].kind.severity(), Severity::Recoverable);
        }
    }

    /// A declared length of exactly `0x34` holds the fixed fields and none
    /// of the strings. The file name offset names no byte of the entry, so
    /// the search reads no byte, and the defect is at `FileNameOffset`.
    #[test]
    fn a_string_offset_past_the_entry_names_the_text_and_no_byte_searched() {
        let mut payload = build_component_entry("Past.ocx", "PastLib.Past", "Past");
        let file_name = entry_u32(&payload, 0x28);
        assert!(file_name > 0x34);
        payload[0x00..0x04].copy_from_slice(&0x34_u32.to_le_bytes());
        let table = one_entry_table(&payload);

        assert!(table.components.is_empty());
        assert_eq!(
            table.defects(),
            [Defect {
                site: Site {
                    offset: 0x428,
                    rva: Some(0x1028),
                    structure: "ExternalComponentEntry",
                    field: "FileNameOffset",
                },
                kind: DefectKind::NoNulTerminator {
                    offset: 0x400 + file_name,
                    limit: 0,
                },
            }]
        );
    }

    /// The search for a string stops at the end of the entry, or after
    /// `NAME_MAX` bytes. Each case gives no component and one defect at the
    /// offset field that names the string.
    #[test]
    fn a_component_string_with_no_nul_gives_a_defect_at_its_own_offset_field() {
        // The component name is the last string. With its NUL gone, the
        // search reads to the end of the entry.
        let mut payload = build_component_entry("End.ocx", "EndLib.End", "End");
        let end = payload.len();
        assert_eq!(payload[end - 1], 0);
        payload[end - 1] = b'A';
        let name = entry_u32(&payload, 0x30);
        let entry_len = u32::try_from(end).unwrap();
        let table = one_entry_table(&payload);

        assert!(table.components.is_empty());
        assert_eq!(entry_len - name, 4);
        assert_eq!(
            table.defects(),
            [Defect {
                site: Site {
                    offset: 0x430,
                    rva: Some(0x1030),
                    structure: "ExternalComponentEntry",
                    field: "NameOffset",
                },
                kind: DefectKind::NoNulTerminator {
                    offset: 0x400 + name,
                    limit: 4,
                },
            }]
        );

        // A library name longer than `NAME_MAX` has its NUL after the last
        // byte that the search reads.
        let long = "L".repeat(0x110);
        let payload = build_component_entry("Long.ocx", &long, "Long");
        let source = entry_u32(&payload, 0x2C);
        let table = one_entry_table(&payload);

        assert!(table.components.is_empty());
        assert_eq!(NAME_MAX, 0x104);
        assert_eq!(
            table.defects(),
            [Defect {
                site: Site {
                    offset: 0x42C,
                    rva: Some(0x102C),
                    structure: "ExternalComponentEntry",
                    field: "SourceOffset",
                },
                kind: DefectKind::NoNulTerminator {
                    offset: 0x400 + source,
                    limit: NAME_MAX,
                },
            }]
        );
    }
}
