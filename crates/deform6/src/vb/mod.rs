//! The Visual Basic structures inside the executable.
//!
//! [`inspect`] is the whole public reading path. It takes a byte slice and it
//! returns a value. It opens no file and it writes no file, which is what
//! makes the Phase 5 fuzz target the real public API rather than an internal
//! one, and which leaves the file system to the command line crate.
//!
//! `classify`, `functyp`, `object` and `privateobj` are declared together, in
//! one commit, for the reason `lib.rs` gives for its own module list: plans
//! 02-02 through 02-05 each edit only the one new file this phase gives them,
//! so this file is written once and never becomes a merge point.
//!
//! The eight phase 3 modules (`gui`, `opcodes`, `controltree`, `vbstr`,
//! `propstream`, `frx`, `ocx`, `controlinfo`) are declared here as a set, in
//! one commit, for the same reason: each later plan in the phase then edits
//! only the one file it owns, and this file never becomes a merge point for
//! two plans in one wave.

pub mod classify;
pub mod controlinfo;
pub mod controltree;
pub mod frx;
pub mod functyp;
pub mod gui;
pub mod header;
pub mod object;
pub mod ocx;
pub mod opcodes;
pub mod privateobj;
pub mod project;
pub mod propstream;
pub mod runtime;
pub mod vbstr;

pub use crate::error::Refusal;

use crate::error::{Defect, DefectKind, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Rva, Va};
use classify::ObjectKind;
use functyp::{FuncTypeWalk, ProcedureSignature, Prototype, PrototypeList};
use header::{VbHeader, header_region};
use object::{Object, ObjectTable};
use privateobj::{Gap, ObjectInfo, PrivateObj, ProcNames, Procedure, ProcedureList};
use project::{Component, ComponentTable, Declaration, DeclareTable, ObjectTableHead, ProjectInfo};
use runtime::{Runtime, runtime_of};

/// One procedure slot, composed from two arrays that share one length
/// (`Object.proc_count`) and resolve independently.
///
/// [`privateobj::ProcedureList`] gives the recovered name or the fact that
/// the slot is private; [`functyp::FuncTypeWalk`] gives the prototype at the
/// same index, when its own array resolved. One can succeed while the other
/// does not, so the join keeps the name and the prototype as two facts, not
/// one.
#[derive(Clone, Debug, PartialEq)]
pub enum ProcedureEntry {
    /// A recovered public procedure name.
    Public {
        /// The name, resolved from `Object.lpProcNamesArray`.
        name: String,
        /// The argument list, the modifiers and the return type, when
        /// `PrivateObj.lpFuncTypeInfo`'s entry at the same index resolved.
        /// `None` when it did not, or when this object carries no
        /// `PrivateObj` at all to read it from.
        prototype: Option<Prototype>,
    },
    /// A private procedure, or a name-array entry this file could not
    /// validate as a name. Per OBJ-06, nothing is invented: no name, no
    /// index number, no placeholder.
    Private,
}

/// The procedure slots one object carries, or the fact that it carries none
/// through this structure at all.
///
/// Kept apart per D-13, matching [`privateobj::ProcNames`]'s own two states:
/// an empty list and "there is no array to read names from" are different
/// facts about the file, and collapsing them loses the standard-module cap.
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectProcedures {
    /// One [`ProcedureEntry`] per declared slot, in array order.
    Slots(Vec<ProcedureEntry>),
    /// The object carries no procedure name array at all. The
    /// standard-module cap, D-10: `proc_count` is the real number of
    /// procedures the object declares, and there is nothing here to print a
    /// name for any of them.
    NoNameArray {
        /// The number of procedures the object declares.
        proc_count: u32,
    },
}

/// One object in the object graph: OBJ-01 through OBJ-04 and OBJ-06 in one
/// value.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectReport {
    /// The object's name, resolved by `vb/object.rs`. Empty when the name
    /// pointer did not resolve; see [`Report::defects`] for the reason.
    pub name: String,
    /// A form, a standard module, a class, or a raw value this corpus has
    /// not measured yet. Per D-08, an unknown kind is never a refusal.
    pub kind: ObjectKind,
    /// Every procedure slot this object declares, joined from the name array
    /// and the type descriptor array.
    pub procedures: ObjectProcedures,
    /// The open questions this object's `PrivateObj` fields raise, per D-14.
    /// Empty for a standard module, which carries no `PrivateObj` to raise
    /// one.
    pub gaps: Vec<Gap>,
}

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
///
/// # `Eq` is not derived
///
/// [`functyp::DefaultValue::Single`] carries an `f32`, which has no total
/// order and does not implement `Eq`. `PartialEq` is enough for every
/// `assert_eq!` this crate's tests use, on this type and on
/// `Result<Report, Refusal>` alike.
#[derive(Clone, Debug, PartialEq)]
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
    /// Every object the project declares, in object array order.
    ///
    /// Serves OBJ-01 through OBJ-04 and OBJ-06: each one carries the name and
    /// the kind [`object::ObjectTable::walk`] and [`classify::classify`]
    /// give it, plus the procedures this phase joined from the name array
    /// and the type descriptor array.
    pub objects: Vec<ObjectReport>,
    /// Every external `Declare` statement the project imports, per OBJ-05.
    ///
    /// Only the entries the file marks external; an internal entry is
    /// resolved inside the runtime and is never in this list.
    pub declarations: Vec<Declaration>,
    /// Every OCX or type library component a form in this project
    /// references, read from the external component table.
    pub components: Vec<Component>,
    /// Every recoverable defect this run collected, across every structure
    /// this phase reads.
    ///
    /// `WINDOWS.md` finding 3: before this field existed, `inspect` collected
    /// these and had nowhere to put them, because `Report` needed to
    /// `derive(PartialEq)` and `Defect` could not. Fixed in `error.rs`, this
    /// plan.
    pub defects: Vec<Defect>,
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

    let mut defects: Vec<Defect> = Vec::new();
    defects.extend(table.defects().iter().cloned());

    // The object array walk is on the spine: an unmapped array pointer or a
    // truncated element refuses the file, unchanged from phase 1's own
    // `ObjectTableHead::read`. Every recoverable failure inside one already
    // reached object (a name, a `PrivateObj`, a type descriptor) is a
    // defect on that object, collected below, and never a refusal.
    let object_table = ObjectTable::walk(&pe, info.lp_object_table, &table)?;
    defects.extend(object_table.defects().iter().cloned());

    let objects: Vec<ObjectReport> = object_table
        .objects
        .iter()
        .map(|object| compose_object(&pe, object, &mut defects))
        .collect();

    let declare_table = DeclareTable::read(&pe, &info);
    defects.extend(declare_table.defects().iter().cloned());

    let component_table =
        ComponentTable::read(&pe, header.lp_external_table, header.w_external_count);
    defects.extend(component_table.defects().iter().cloned());

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
        objects,
        declarations: declare_table.declarations,
        components: component_table.components,
        defects,
    })
}

/// Composes one object's kind, its procedures and its open questions.
///
/// A leaf-pointer failure anywhere in this function is a defect on `object`,
/// pushed onto `defects`, and never loses the object's other fields: the
/// caller still gets the object's name, its kind and whatever this function
/// could still read.
fn compose_object(pe: &PeImage<'_>, object: &Object, defects: &mut Vec<Defect>) -> ObjectReport {
    let kind = classify::classify(object.f_object_type);
    let private = read_private(pe, object, defects);

    let proc_list = ProcedureList::read(pe, object);
    defects.extend(proc_list.defects().iter().cloned());
    let proto_walk = FuncTypeWalk::read(pe, object, &private);
    defects.extend(proto_walk.defects().iter().cloned());

    let gaps = private.gaps();
    let procedures = compose_procedures(proc_list.procs, proto_walk.signatures);

    ObjectReport {
        name: object.name.clone(),
        kind,
        procedures,
        gaps,
    }
}

/// Reads `ObjectInfo` and `PrivateObj` for one object, converting either
/// pointer's failure into a defect rather than a refusal of the whole file.
///
/// # No corpus program exercises this path
///
/// Every one of the 105 objects across all 44 vendored programs resolves
/// both `ObjectInfo` and `PrivateObj` cleanly. This function exists for the
/// hostile file the corpus does not contain, per `AGENTS.md`'s "no panic on
/// any input, ever": `Object.lpObjectInfo` and `ObjectInfo.lpPrivateObject`
/// are each a leaf pointer relative to the object that carries them, exactly
/// like the name pointer `vb/object.rs` already treats as recoverable, and
/// losing either one must not lose the object.
fn read_private(pe: &PeImage<'_>, object: &Object, defects: &mut Vec<Defect>) -> PrivateObj {
    let info = match ObjectInfo::read(pe, object.lp_object_info) {
        Ok(info) => info,
        Err(_) => {
            defects.push(unreadable_pointer(
                pe,
                object.lp_object_info,
                "Object",
                "lpObjectInfo",
            ));
            return PrivateObj::Absent;
        }
    };

    match PrivateObj::read(pe, info.lp_private_object) {
        Ok(private) => private,
        Err(_) => {
            defects.push(unreadable_pointer(
                pe,
                Va::new(info.lp_private_object),
                "ObjectInfo",
                "lpPrivateObject",
            ));
            PrivateObj::Absent
        }
    }
}

/// Builds the defect for a leaf pointer this file could not follow.
///
/// `offset` is `0`: the byte position the pointer itself was read from is
/// not carried by [`Object`] or [`ObjectInfo`] once composition reaches this
/// function. This is the same fallback `vb/object.rs`'s own `read_name` uses
/// (`.map_or(0, Off::get)`) whenever a file offset is unavailable; the
/// virtual address, in both `kind.va` and `site.rva`, is what a reader uses
/// to find the byte in question.
fn unreadable_pointer(
    pe: &PeImage<'_>,
    va: Va,
    structure: &'static str,
    field: &'static str,
) -> Defect {
    Defect {
        site: Site {
            offset: 0,
            rva: va.to_rva(pe.image_base()).map(Rva::get),
            structure,
            field,
        },
        kind: DefectKind::UnreadablePointer {
            offset: 0,
            va: va.get(),
        },
    }
}

/// Joins the recovered names with the recovered prototypes, by index, over
/// the same `Object.proc_count` length both arrays share.
///
/// A slot's name comes from `Object.lpProcNamesArray`
/// ([`privateobj::ProcedureList`]) and its prototype comes from
/// `PrivateObj.lpFuncTypeInfo` ([`functyp::FuncTypeWalk`]): two different
/// arrays that can resolve independently. When the prototype array itself
/// carries no slots for this object (`PrototypeList::NoPrivateObject` or
/// `PrototypeList::NoFuncTypeArray`, neither observed anywhere in this
/// corpus), every entry's prototype is `None` and its name still prints.
fn compose_procedures(names: ProcNames, prototypes: PrototypeList) -> ObjectProcedures {
    let slots = match names {
        ProcNames::NoNameArray { proc_count } => {
            return ObjectProcedures::NoNameArray { proc_count };
        }
        ProcNames::Slots(slots) => slots,
    };

    let proto_slots = match prototypes {
        PrototypeList::Slots(proto_slots) => proto_slots,
        PrototypeList::NoPrivateObject { .. } | PrototypeList::NoFuncTypeArray { .. } => Vec::new(),
    };

    let entries = slots
        .into_iter()
        .enumerate()
        .map(|(index, proc)| match proc {
            Procedure::Public(name) => {
                let prototype = proto_slots.get(index).and_then(|sig| match sig {
                    ProcedureSignature::Prototype(prototype) => Some(prototype.clone()),
                    ProcedureSignature::NoDescriptor | ProcedureSignature::Unrecoverable => None,
                });
                ProcedureEntry::Public { name, prototype }
            }
            Procedure::Private => ProcedureEntry::Private,
        })
        .collect();

    ObjectProcedures::Slots(entries)
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
    use super::{ObjectProcedures, ProcedureEntry, Refusal, Report, inspect};
    use crate::error::DefectKind;
    use crate::read::pe::PeImage;
    use crate::read::region::Off;
    use crate::vb::classify::ObjectKind;
    use crate::vb::header::{VbHeader, header_region};
    use crate::vb::privateobj::Gap;
    use crate::vb::project::{ObjectTableHead, ProjectInfo};

    /// The corpus program this module reads.
    ///
    /// A test inside `src/` reaches a corpus file this way and never through
    /// a path. Plan 01-08 owns the sweep over all 44, in a file under
    /// `tests/`, which is a separate crate root.
    const MANDELBROT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
    ));

    /// The program whose object count and object capacity differ: one form
    /// and two classes declared, a capacity of 4. `frmGrayscale` recovers 8
    /// of its 20 public names, `pdOpenSaveDialog` 0 of 6, `FastDrawing` 4 of
    /// 8: twelve of thirty-four, and eight of its nine `Declare` entries are
    /// external. `[VERIFIED: local]` against the real corpus file.
    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    /// The program with two standard modules: `Declaration_Module`
    /// (`proc_count` 1) and `Sub_Module` (`proc_count` 7), neither reachable
    /// through this structure at all. `[VERIFIED: local]`
    const MAP_EDITOR: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Map-editor-2D/Map Editor.exe"
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

    /// Gives the address of `ProjectInfo` that the file itself holds.
    ///
    /// A second copy of the helper `vb/object.rs`'s own test module holds,
    /// written on purpose, per `AGENTS.md`: a test builds the state it
    /// needs, and a shared fixture module would let a change to one file's
    /// tests silently break another's.
    fn project_data_va(data: &[u8]) -> crate::read::region::Va {
        let image = PeImage::parse(data).unwrap();
        let hdr = header_region(&image).unwrap();
        VbHeader::read(&hdr).unwrap().lp_project_data
    }

    /// Gives the address of the object table that the file itself holds.
    fn object_table_va(data: &[u8]) -> crate::read::region::Va {
        let image = PeImage::parse(data).unwrap();
        ProjectInfo::read(&image, project_data_va(data))
            .unwrap()
            .lp_object_table
    }

    /// Gives the absolute file offset of a field inside the object table.
    fn object_table_field_offset(data: &[u8], field: u32) -> usize {
        let image = PeImage::parse(data).unwrap();
        let window = image.region_at_va(object_table_va(data)).unwrap();
        let at = window.file_offset(Off::new(field)).unwrap();
        usize::try_from(at.get()).unwrap()
    }

    /// Copies `data` and writes a `u16` at an absolute file offset.
    fn with_u16_at(data: &[u8], at: usize, value: u16) -> Vec<u8> {
        let mut out = data.to_vec();
        assert_ne!(
            out[at..at + 2],
            value.to_le_bytes(),
            "the fixture writes the value the field already holds, so it proves nothing"
        );
        out[at..at + 2].copy_from_slice(&value.to_le_bytes());
        out
    }

    /// `WINDOWS.md` finding 3, closed: `Report` carries the defects
    /// `inspect` collected instead of dropping them.
    ///
    /// `Grayscale.exe` declares three objects (`wTotalObjects` 3) and its
    /// object array holds room for four (`wCompiledObjects` 4). That
    /// direction is normal, per `vb/project.rs`'s own `count_defects`, and
    /// produces no defect on a clean corpus file. The disagreement
    /// `ObjectTableHead::defects` actually reports is the other direction: a
    /// capacity **below** the count. Patched here to `1`, which is below the
    /// real `wTotalObjects` of `3`, this is the one shape of that structure's
    /// own defect a corpus file can be made to produce, and it is what the
    /// test proves reaches `Report.defects`.
    #[test]
    fn a_capacity_below_the_object_count_reaches_the_caller_as_a_defect() {
        let at = object_table_field_offset(GRAYSCALE, 0x2C);
        let bytes = with_u16_at(GRAYSCALE, at, 1);

        let report = inspect(&bytes).unwrap();
        let defect = report
            .defects
            .iter()
            .find(|d| matches!(d.kind, DefectKind::CountMismatch { .. }))
            .expect("the patched capacity must produce a CountMismatch defect on Report.defects");
        assert!(matches!(
            defect.kind,
            DefectKind::CountMismatch {
                count: 1,
                expected: 3,
                ..
            }
        ));
    }

    /// `assert_eq!` on a `Result<Report, Refusal>` needs both sides to
    /// compare, so `Report` must derive `PartialEq` and `Debug`. `Report`
    /// does not derive `Eq`: `functyp::DefaultValue::Single` carries an
    /// `f32`, which has none, and `assert_eq!` never needs it.
    ///
    /// A missing derive on the success side of the `Result` is a compile
    /// error that costs a cycle, and RESEARCH.md section 7.2 records it
    /// happening. This test is the instrument that keeps the derive.
    #[test]
    fn a_report_with_the_full_object_graph_still_compares_and_prints() {
        let report = mandelbrot_report();
        let same: Result<Report, Refusal> = Ok(report.clone());
        assert_eq!(same, Ok(report.clone()));
        assert_ne!(same, Err(Refusal::NotPe));
        assert!(format!("{report:?}").contains("frmFractal"));
    }

    /// `Report` carries the object graph, the external imports and the
    /// components, per this task's third behaviour.
    ///
    /// `Mandelbrot.exe`'s one object, `frmFractal`, gives every one of the
    /// four new pieces this task adds: a kind, a procedure list (nine
    /// slots, every one private, per `CONTEXT.md`'s corrected worked
    /// example), an open gap (`cnt_public_vars` is `9`, non-zero, so
    /// D-14 carries it as unexplained rather than as a count), and one
    /// external `Declare` (`gdi32!SetPixelV`). It references no OCX
    /// component, so `components` is empty without being absent.
    #[test]
    fn the_object_graph_the_declarations_and_the_components_reach_the_caller() {
        let report = mandelbrot_report();

        assert_eq!(report.objects.len(), 1);
        let object = &report.objects[0];
        assert_eq!(object.name, "frmFractal");
        assert_eq!(object.kind, ObjectKind::Form);
        assert_eq!(object.gaps, vec![Gap::UnexplainedPublicVarCount(9)]);

        let ObjectProcedures::Slots(slots) = &object.procedures else {
            panic!("frmFractal carries a PrivateObj, so this must be Slots");
        };
        assert_eq!(slots.len(), 9);
        assert!(
            slots.iter().all(|slot| *slot == ProcedureEntry::Private),
            "every one of frmFractal's nine procedures is Private: {slots:?}"
        );

        assert_eq!(report.declarations.len(), 1);
        assert_eq!(report.declarations[0].library, "gdi32");

        assert!(report.components.is_empty());
    }

    /// One recoverable failure must not lose the rest: an object whose
    /// `PrivateObj` does not resolve keeps its name, its kind and its
    /// procedure names, and loses only its prototypes.
    ///
    /// No corpus program exercises this path (every `ObjectInfo` and every
    /// `PrivateObj` resolves cleanly across all 44 vendored programs), so
    /// this test patches `frmGrayscale`'s `ObjectInfo.lpPrivateObject` field
    /// to an address in no section. This is also the instrument for this
    /// task's second deliberate breakage: turning `read_private`'s recovery
    /// into a `?`-propagated refusal makes this test fail, because `inspect`
    /// then refuses the whole file over one object's leaf pointer.
    #[test]
    fn an_unresolved_private_obj_loses_only_its_own_objects_prototypes() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let head = ObjectTableHead::read(&image, object_table_va(GRAYSCALE)).unwrap();
        let table = crate::vb::object::ObjectTable::walk(&image, object_table_va(GRAYSCALE), &head)
            .unwrap();
        let frm_grayscale = &table.objects[0];
        assert_eq!(frm_grayscale.name, "frmGrayscale");

        let object_info = image.region_at_va(frm_grayscale.lp_object_info).unwrap();
        let at = object_info.file_offset(Off::new(0x0C)).unwrap();
        let at = usize::try_from(at.get()).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        assert!(
            image
                .region_at_va(crate::read::region::Va::new(nowhere))
                .is_none()
        );
        let mut bytes = GRAYSCALE.to_vec();
        bytes[at..at + 4].copy_from_slice(&nowhere.to_le_bytes());

        let report = inspect(&bytes).unwrap();
        let object = report
            .objects
            .iter()
            .find(|o| o.name == "frmGrayscale")
            .unwrap();

        // The name and the kind still stand.
        assert_eq!(object.kind, ObjectKind::Form);
        // No open gap: `PrivateObj::Absent` raises none.
        assert!(object.gaps.is_empty());
        // The procedure names still resolve (they come from a different
        // array, `Object.lpProcNamesArray`, untouched by this patch), but
        // every prototype is now `None`.
        let ObjectProcedures::Slots(slots) = &object.procedures else {
            panic!("frmGrayscale's proc_count is non-zero and its array pointer is real");
        };
        let public_names: Vec<&str> = slots
            .iter()
            .filter_map(|slot| match slot {
                ProcedureEntry::Public { name, prototype } => {
                    assert!(
                        prototype.is_none(),
                        "a prototype must not survive: {slot:?}"
                    );
                    Some(name.as_str())
                }
                ProcedureEntry::Private => None,
            })
            .collect();
        assert_eq!(
            public_names.len(),
            8,
            "frmGrayscale recovers 8 public names"
        );

        // The defect names the leaf pointer that did not resolve.
        let defect = report
            .defects
            .iter()
            .find(|d| d.site.structure == "ObjectInfo" && d.site.field == "lpPrivateObject")
            .expect("the patched ObjectInfo.lpPrivateObject must produce a defect");
        assert!(matches!(
            defect.kind,
            DefectKind::UnreadablePointer { va, .. } if va == nowhere
        ));
    }

    /// On `Grayscale.exe`, `inspect` gives three objects with the right
    /// kinds, twelve recovered public procedure names and eight external
    /// imports, per this task's fourth behaviour.
    #[test]
    fn grayscale_gives_three_objects_twelve_public_names_and_eight_imports() {
        let report = inspect(GRAYSCALE).unwrap();

        assert_eq!(report.objects.len(), 3);
        let kinds: Vec<ObjectKind> = report.objects.iter().map(|o| o.kind).collect();
        assert_eq!(
            kinds,
            vec![ObjectKind::Form, ObjectKind::Class, ObjectKind::Class]
        );

        let public_count: usize = report
            .objects
            .iter()
            .map(|object| match &object.procedures {
                ObjectProcedures::Slots(slots) => slots
                    .iter()
                    .filter(|slot| matches!(slot, ProcedureEntry::Public { .. }))
                    .count(),
                ObjectProcedures::NoNameArray { .. } => 0,
            })
            .sum();
        assert_eq!(public_count, 12);

        assert_eq!(report.declarations.len(), 8);
    }

    /// On `Map Editor.exe`, `inspect` gives two objects whose procedures are
    /// reported unreachable rather than as an empty list, per this task's
    /// fifth behaviour: `Declaration_Module` (`proc_count` 1) and
    /// `Sub_Module` (`proc_count` 7), both standard modules, per D-13.
    #[test]
    fn map_editor_reports_its_two_modules_as_unreachable_and_not_as_empty() {
        let report = inspect(MAP_EDITOR).unwrap();

        let modules: Vec<(&str, &ObjectProcedures)> = report
            .objects
            .iter()
            .filter(|object| object.kind == ObjectKind::Module)
            .map(|object| (object.name.as_str(), &object.procedures))
            .collect();

        assert_eq!(
            modules.iter().map(|(name, _)| *name).collect::<Vec<&str>>(),
            vec!["Declaration_Module", "Sub_Module"]
        );
        for (name, procedures) in &modules {
            assert!(
                matches!(procedures, ObjectProcedures::NoNameArray { proc_count } if *proc_count > 0),
                "{name} must report a non-zero proc_count with no name array, not an empty list"
            );
        }
    }

    /// `inspect` still takes only a byte slice and returns a value, per this
    /// task's sixth behaviour. The grep the plan's own `<verify>` runs is
    /// the acceptance instrument for "no file system type anywhere under
    /// `src/`"; this test is the type-level half of the same claim.
    #[test]
    fn inspect_still_takes_a_byte_slice_and_returns_a_value() {
        fn assert_signature(_f: fn(&[u8]) -> Result<Report, Refusal>) {}
        assert_signature(inspect);
    }
}
