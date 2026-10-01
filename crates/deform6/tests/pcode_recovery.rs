#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! What DeForm6 recovers from the P-code corpus, against the source and
//! against the native build of the same source.
//!
//! `corpus-pcode/` holds a P-code build of 42 corpus programs, which the
//! Visual Basic 6 IDE built from the committed source. The native build of
//! each one is in `corpus/`, and the native gates hold it to its source. This
//! file asks each P-code binary the same questions:
//!
//! - **The objects.** Each P-code binary gives the objects of its source
//!   project file, by name and by kind, in the order of that file.
//! - **The written project.** DeForm6 writes the same project from the
//!   P-code binary as from the native binary. A report comment names a byte
//!   offset, and the offsets differ between the two binaries, so they are
//!   masked. The lines of the project file are compared in any order,
//!   because the objects of one native binary are not in the order of its
//!   source.
//! - **The event handlers.** Each bound event slot of a P-code program
//!   gives a handler address, as in the native build. It is the entry of the
//!   method table of the object for the procedure that the source names for
//!   that control. Each P-code stub returns into `MethCallEngine`, and it
//!   loads into `eax` the word at `+0x04` of its `ControlInfo` record.

#[path = "build_record/shared.rs"]
#[allow(
    dead_code,
    reason = "this module is embedded in several binaries, and this one uses only the corpus \
              walk and the files that DeForm6 writes"
)]
mod build_record;

#[path = "support/mod.rs"]
#[allow(
    dead_code,
    reason = "cargo compiles this shared module into every test binary and this one uses only \
              the vbp reader"
)]
mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use deform6::error::DefectKind;
use deform6::journal::Mode;
use deform6::read::pe::PeImage;
use deform6::read::region::{Off, Va};
use deform6::vb::Report;
use deform6::vb::classify::ObjectKind as RecoveredKind;
use deform6::vb::constants::{constant_declare, constant_procedure};
use deform6::vb::context::{callees_of_project, callees_of_project_named};
use deform6::vb::controlinfo::{
    ControlInfoTable, EventReport, EventSlot, StubShape, read_event_table,
};
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::lift::Callees;
use deform6::vb::links::{LinkSlot, read_method_links};
use deform6::vb::object::{Object, ObjectTable};
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::pcode::PcodeTable;
use deform6::vb::procdesc::{MethodEntry, module_fixed_arrays, read_method_table};
use deform6::vb::project::{DeclareTable, ObjectTableHead, ProjectInfo};
use deform6::vb::types::{VbTypes, guid_text};
use object::LittleEndian as LE;
use object::pe::ImageNtHeaders32;
use object::read::pe::{ImageNtHeaders, ImageOptionalHeader, Import, PeFile32};
use support::{frm, source, vbp};

/// The directory of the committed P-code corpus.
fn pcode_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus-pcode")
}

/// Each P-code binary, with its key: the path of the native executable of
/// the same source, relative to `corpus/`.
fn pcode_programs() -> Vec<(String, PathBuf)> {
    let root = pcode_root();
    let programs: Vec<(String, PathBuf)> = build_record::executables(&root)
        .unwrap()
        .into_iter()
        .map(|exe| (build_record::program_key(&exe, &root).unwrap(), exe))
        .collect();
    assert!(!programs.is_empty(), "{} holds no binary", root.display());
    programs
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

/// The label of an object kind that a project file declares.
fn declared_label(kind: vbp::ObjectKind) -> &'static str {
    match kind {
        vbp::ObjectKind::Form => "Form",
        vbp::ObjectKind::Module => "Module",
        vbp::ObjectKind::Class => "Class",
        vbp::ObjectKind::UserControl => "UserControl",
        vbp::ObjectKind::PropertyPage => "PropertyPage",
        vbp::ObjectKind::UserDocument => "UserDocument",
        vbp::ObjectKind::Designer => "Designer",
        vbp::ObjectKind::RelatedDoc => "RelatedDoc",
    }
}

/// The label of an object kind that DeForm6 recovers.
fn recovered_label(kind: RecoveredKind) -> &'static str {
    match kind {
        RecoveredKind::Form => "Form",
        RecoveredKind::Module => "Module",
        RecoveredKind::Class => "Class",
        RecoveredKind::Unknown(_) => "Unknown",
    }
}

/// Each P-code binary gives the objects of its source project file, by name
/// and by kind, in the order of that file.
#[test]
fn each_pcode_program_recovers_the_objects_of_its_source_in_order() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let table = OpcodeTable::builtin();
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let declared: Vec<(String, &str)> = vbp::Project::read(&project)
            .declared_objects()
            .iter()
            .map(|object| {
                (
                    object.name.clone().unwrap_or_default(),
                    declared_label(object.kind),
                )
            })
            .collect();
        let report = deform6::inspect(&read(&exe), &table, Mode::Strict)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let recovered: Vec<(String, &str)> = report
            .objects
            .iter()
            .map(|object| (object.name.clone(), recovered_label(object.kind)))
            .collect();
        if recovered != declared {
            failures.push(format!(
                "{key}: recovered {recovered:?}, and the source declares {declared:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} P-code programs give other objects than their source:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Gives `bytes` with each byte offset that a report comment names masked:
/// `(byte offset 0x1242)` becomes `(byte offset X)`. Any other text stays.
fn mask_offsets(bytes: &[u8]) -> Vec<u8> {
    const MARK: &[u8] = b"(byte offset 0x";
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at..].starts_with(MARK) {
            let digits = at + MARK.len();
            let mut end = digits;
            while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
                end += 1;
            }
            if end > digits && bytes.get(end) == Some(&b')') {
                out.extend_from_slice(b"(byte offset X)");
                at = end + 1;
                continue;
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    out
}

/// Gives the lines of a file, sorted, so that their order does not count.
fn sorted_lines(bytes: &[u8]) -> Vec<&[u8]> {
    let mut lines: Vec<&[u8]> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.starts_with(b"RevisionVer="))
        .collect();
    lines.sort_unstable();
    lines
}

/// A byte offset in a report comment is masked, and nothing else changes.
#[test]
fn only_the_byte_offset_of_a_report_comment_is_masked() {
    assert_eq!(
        mask_offsets(b"'x: opcode 36 (byte offset 0x1242)\r\n"),
        b"'x: opcode 36 (byte offset X)\r\n"
    );
    for kept in [
        &b"'x: (no recorded byte offset)"[..],
        b"(byte offset 0x)",
        b"(byte offset 0x12",
        b"(byte offset 0x12G)",
    ] {
        assert_eq!(
            mask_offsets(kept),
            kept,
            "{}",
            String::from_utf8_lossy(kept)
        );
    }
}

/// DeForm6 writes the same project from each P-code binary as from the
/// native binary of the same source, when the byte offsets of the report
/// comments are masked. The files pair by name, and the lines of the
/// project file are compared in any order, because `Grayscale.exe` in
/// `corpus/` holds its objects in another order than its source. The
/// `RevisionVer` line is left out: each build holds its own revision, and
/// a project with `AutoIncrementVer=1` raised it after each build. The test
/// above holds the P-code order to the source.
#[test]
fn each_pcode_program_writes_the_project_of_its_native_build() {
    let root = build_record::corpus_root();
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let mut pcode = build_record::project_files(&read(&exe))
            .unwrap_or_else(|err| panic!("{key} P-code: {err}"));
        let mut native = build_record::project_files(&read(&root.join(&key)))
            .unwrap_or_else(|err| panic!("{key} native: {err}"));
        // The files come in the order of the objects, so they pair by name.
        pcode.sort_by(|a, b| a.0.cmp(&b.0));
        native.sort_by(|a, b| a.0.cmp(&b.0));
        let pcode_names: Vec<&String> = pcode.iter().map(|(name, _bytes)| name).collect();
        let native_names: Vec<&String> = native.iter().map(|(name, _bytes)| name).collect();
        if pcode_names != native_names {
            failures.push(format!(
                "{key}: writes {pcode_names:?}, and native {native_names:?}"
            ));
            continue;
        }
        for ((name, pcode_bytes), (_name, native_bytes)) in pcode.iter().zip(&native) {
            let (pcode_bytes, native_bytes) =
                (mask_offsets(pcode_bytes), mask_offsets(native_bytes));
            let same = if name.to_ascii_lowercase().ends_with(".vbp") {
                sorted_lines(&pcode_bytes) == sorted_lines(&native_bytes)
            } else {
                pcode_bytes == native_bytes
            };
            if !same {
                failures.push(format!("{key}: {name} differs from the native one"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} files that DeForm6 writes from P-code differ from the native ones:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Counts the bound event slots of a report, and the ones that give a
/// handler address.
fn bound_slots(report: &Report) -> (usize, usize) {
    let mut bound = 0;
    let mut with_address = 0;
    for form in &report.forms {
        for control in &form.controls {
            for event in &control.events {
                if let EventReport::Named {
                    handler_address, ..
                }
                | EventReport::BoundUnnamed {
                    handler_address, ..
                } = event
                {
                    bound += 1;
                    if handler_address.is_some() {
                        with_address += 1;
                    }
                }
            }
        }
    }
    (bound, with_address)
}

/// The number of bound events whose handler address `inspect` reports across
/// the P-code corpus. As in the native corpus, this is not a number of stubs:
/// all the elements of a control array report the events of one
/// `ControlInfo`. The form `Main` of `Map Editor.exe` gives 20 of them, one
/// for each handler of its source.
const EXPECTED_REPORTED_HANDLERS: usize = 408;

/// The number of bound event slots that the `ControlInfo` records of the
/// P-code corpus hold. Each names one stub.
const EXPECTED_BOUND_SLOTS: usize = 382;

/// Each P-code program has the bound event slots of its native build, each
/// of them gives a handler address, and no stub has an unknown shape.
#[test]
fn each_bound_event_slot_of_a_pcode_program_gives_a_handler_address() {
    let root = build_record::corpus_root();
    let table = OpcodeTable::builtin();
    let mut addresses = 0;
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let native = deform6::inspect(&read(&root.join(&key)), &table, Mode::Salvage)
            .unwrap_or_else(|err| panic!("{key} native: {err}"));
        let pcode = deform6::inspect(&read(&exe), &table, Mode::Salvage)
            .unwrap_or_else(|err| panic!("{key} P-code: {err}"));
        let (native_bound, native_addresses) = bound_slots(&native);
        let (pcode_bound, pcode_addresses) = bound_slots(&pcode);
        let unknown_stubs = pcode
            .defects
            .iter()
            .filter(|defect| matches!(defect.kind, DefectKind::UnknownStubShape { .. }))
            .count();
        addresses += pcode_addresses;
        let found = (pcode_bound, pcode_addresses, unknown_stubs);
        if found != (native_bound, native_addresses, 0) {
            failures.push(format!(
                "{key}: P-code gives {pcode_bound} bound slots, {pcode_addresses} handler \
                 addresses and {unknown_stubs} unknown stubs; native gives {native_bound} bound \
                 slots and {native_addresses} handler addresses"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} P-code programs do not give the handler addresses of their native build:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        addresses, EXPECTED_REPORTED_HANDLERS,
        "the P-code corpus now gives {addresses} handler addresses"
    );
}

/// The objects of one image, by name, read through the public reading API.
fn objects_by_name(pe: &PeImage<'_>) -> BTreeMap<String, Object> {
    let header = VbHeader::read(&header_region(pe).unwrap()).unwrap();
    let info = ProjectInfo::read(pe, header.lp_project_data).unwrap();
    let head = ObjectTableHead::read(pe, info.lp_object_table).unwrap();
    ObjectTable::walk(pe, info.lp_object_table, &head)
        .unwrap()
        .objects
        .into_iter()
        .map(|object| (object.name.clone(), object))
        .collect()
}

/// The method table of one object: the `wMethodCount` values at
/// `lpMethods`. `STRUCTURES.md` section 5.2 puts `wMethodCount` at
/// `ObjectInfo + 0x20` and `lpMethods` at `ObjectInfo + 0x24`. DeForm6 does
/// not read this table, so this test reads it.
fn method_table(pe: &PeImage<'_>, object: &Object) -> Vec<u32> {
    let info = pe.region_at_va(object.lp_object_info).unwrap();
    let count = info.u16_le(Off::new(0x20)).unwrap();
    if count == 0 {
        return Vec::new();
    }
    let methods = pe
        .region_at_va(info.va_le(Off::new(0x24)).unwrap())
        .unwrap();
    (0..u32::from(count))
        .map(|at| methods.u32_le(Off::new(4 * at)).unwrap())
        .collect()
}

/// Tells whether `procedure` is an event procedure of `owner`: the name of
/// the owner, then `_`, in any case.
fn is_named_for(procedure: &str, owner: &str) -> bool {
    procedure
        .to_ascii_lowercase()
        .starts_with(&format!("{}_", owner.to_ascii_lowercase()))
}

/// Each handler address of a P-code program is an entry of the method table
/// of its object, and the procedure of the source at that entry is named for
/// the control of the event: the name of the control, or `Form` for the form
/// itself, then `_`.
///
/// The last entries of the method table are the procedures of the source
/// file, in the order of the file. The entries before them are not
/// procedures of the file, and their number is not the same in each object.
/// So the test counts the procedures of the source file, and it takes that
/// many entries from the end of the table.
#[test]
fn each_pcode_handler_is_the_method_of_a_source_procedure_of_its_control() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let table = OpcodeTable::builtin();
    let mut checked = 0;
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{key}: {err}"));
        let objects = objects_by_name(&pe);
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let sources: BTreeMap<String, PathBuf> = vbp::Project::read(&project)
            .declared_objects()
            .into_iter()
            .filter_map(|object| Some((object.name?, object.source_file)))
            .collect();
        let report = deform6::inspect(&bytes, &table, Mode::Strict)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        for form in &report.forms {
            let methods = method_table(&pe, &objects[&form.name]);
            let procedures = source::declared_procedures(&sources[&form.name]);
            let Some(first) = methods.len().checked_sub(procedures.len()) else {
                failures.push(format!(
                    "{key}: {} has {} methods and {} source procedures",
                    form.name,
                    methods.len(),
                    procedures.len()
                ));
                continue;
            };
            for control in &form.controls {
                let owner = if control.parent.is_none() {
                    "Form"
                } else {
                    control.name.as_str()
                };
                for event in &control.events {
                    let (EventReport::Named {
                        index,
                        handler_address: Some(address),
                        ..
                    }
                    | EventReport::BoundUnnamed {
                        index,
                        handler_address: Some(address),
                        ..
                    }) = event
                    else {
                        continue;
                    };
                    checked += 1;
                    let procedure = methods
                        .iter()
                        .position(|method| method == address)
                        .and_then(|at| at.checked_sub(first))
                        .and_then(|at| procedures.get(at));
                    if !procedure.is_some_and(|name| is_named_for(name, owner)) {
                        failures.push(format!(
                            "{key}: {}.{} slot {index} gives {address:#x}, which names the \
                             procedure {procedure:?}",
                            form.name, control.name
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} P-code handlers do not name a source procedure of their control:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        checked, EXPECTED_REPORTED_HANDLERS,
        "the test checked {checked} handler addresses"
    );
}

/// The function that each import slot of `bytes` imports, by the address
/// of the slot. Read with the `object` crate, not with DeForm6.
fn imports_by_slot(bytes: &[u8]) -> BTreeMap<u32, (String, String)> {
    let file = PeFile32::parse(bytes).unwrap();
    let base = u32::try_from(file.nt_headers().optional_header().image_base()).unwrap();
    let table = file.import_table().unwrap().unwrap();
    let mut out = BTreeMap::new();
    let mut descriptors = table.descriptors().unwrap();
    while let Some(descriptor) = descriptors.next().unwrap() {
        let dll =
            String::from_utf8_lossy(table.name(descriptor.name.get(LE)).unwrap()).into_owned();
        let first_thunk = descriptor.first_thunk.get(LE);
        let names = match descriptor.original_first_thunk.get(LE) {
            0 => first_thunk,
            original => original,
        };
        let mut thunks = table.thunks(names).unwrap();
        let mut slot = base + first_thunk;
        while let Some(thunk) = thunks.next::<ImageNtHeaders32>().unwrap() {
            if let Import::Name(_hint, name) = table.import::<ImageNtHeaders32>(thunk).unwrap() {
                out.insert(
                    slot,
                    (dll.clone(), String::from_utf8_lossy(name).into_owned()),
                );
            }
            slot += 4;
        }
    }
    out
}

/// Each stub that a bound event slot of a P-code program names has the
/// P-code shape, and it returns into `MethCallEngine`. The address that the
/// stub pushes holds `ff 25` and the address of an import slot, which is
/// `jmp dword ptr [slot]`, and that slot imports `MethCallEngine` from
/// `MSVBVM60.DLL`.
#[test]
fn each_pcode_stub_returns_into_meth_call_engine() {
    let mut stubs = 0;
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{key}: {err}"));
        let imports = imports_by_slot(&bytes);
        for object in objects_by_name(&pe).values() {
            let controls =
                ControlInfoTable::read(&pe, object).unwrap_or_else(|err| panic!("{key}: {err}"));
            for control in &controls.entries {
                let events =
                    read_event_table(&pe, control).unwrap_or_else(|err| panic!("{key}: {err}"));
                for slot in &events.slots {
                    let EventSlot::Bound {
                        index,
                        stub,
                        handler,
                    } = *slot
                    else {
                        continue;
                    };
                    stubs += 1;
                    let what = format!("{key}: {}.{} slot {index}", object.name, control.name);
                    let Some(StubShape::PCode { engine }) = handler.map(|handler| handler.shape)
                    else {
                        failures.push(format!(
                            "{what}: the stub at {:#x} gives {handler:?}",
                            stub.get()
                        ));
                        continue;
                    };
                    let target = pe
                        .region_at_va(Va::new(engine))
                        .and_then(|region| region.take(Off::new(0), 6))
                        .filter(|thunk| thunk[..2] == [0xFF, 0x25])
                        .map(|thunk| u32::from_le_bytes(thunk[2..6].try_into().unwrap()));
                    let import = target.and_then(|slot| imports.get(&slot));
                    if import.map(|(dll, name)| (dll.as_str(), name.as_str()))
                        != Some(("MSVBVM60.DLL", "MethCallEngine"))
                    {
                        failures.push(format!(
                            "{what}: the stub returns into {engine:#x}, which imports {import:?}"
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} P-code stubs do not return into MethCallEngine:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        stubs, EXPECTED_BOUND_SLOTS,
        "the P-code corpus now holds {stubs} bound event slots"
    );
}

/// Each stub that a bound event slot of a P-code program names loads into
/// `eax` the word at `+0x04` of its `ControlInfo` record. A native stub
/// holds one less than that word (`tests/byte_fidelity.rs`). The word is
/// read here from the bytes of the file, and not by DeForm6.
#[test]
fn each_pcode_stub_loads_the_word_at_four_of_its_control_into_eax() {
    let mut stubs = 0;
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{key}: {err}"));
        for object in objects_by_name(&pe).values() {
            let controls =
                ControlInfoTable::read(&pe, object).unwrap_or_else(|err| panic!("{key}: {err}"));
            for control in &controls.entries {
                let at = usize::try_from(control.file_offset.get()).unwrap() + 4;
                let word = u32::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]));
                let events =
                    read_event_table(&pe, control).unwrap_or_else(|err| panic!("{key}: {err}"));
                for slot in &events.slots {
                    let EventSlot::Bound { index, handler, .. } = *slot else {
                        continue;
                    };
                    stubs += 1;
                    let eax = handler.map(|handler| handler.imm32);
                    if eax != Some(word) {
                        failures.push(format!(
                            "{key}: {}.{} slot {index} loads {eax:x?}, and the word at 0x04 is \
                             {word:#x}",
                            object.name, control.name
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} P-code stubs do not load the word at 0x04 of their control:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(stubs, EXPECTED_BOUND_SLOTS);
}

/// The method table entries of the P-code corpus, in all 99 objects of the 42
/// programs.
const EXPECTED_METHOD_ENTRIES: usize = 871;

/// The entries that name a procedure descriptor.
const EXPECTED_DESCRIPTORS: usize = 680;

/// The entries that map into no section.
const EXPECTED_NOT_ADDRESSES: usize = 191;

/// DeForm6 reads the method table of each object of a P-code program.
/// Checked against the test's own reading of the table, and against the
/// layout of the file:
///
/// - Each entry that the test finds to be an address is a descriptor, in the
///   same order. Each other entry comes before the first descriptor.
/// - `ProcTable` of each descriptor is the address of the `ObjectInfo` of its
///   object.
/// - Each body is the `ProcSize` bytes that end at its descriptor, with
///   `ProcSize` read here from the file. It starts on a four-byte boundary,
///   and no two bodies of one program share a byte.
/// - Each handler address that `inspect` reports is a descriptor of the
///   method table of its form.
#[test]
fn each_method_table_of_a_pcode_program_names_the_descriptors_of_its_procedures() {
    let table = OpcodeTable::builtin();
    let (mut entries, mut descriptors, mut not_addresses) = (0, 0, 0);
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{key}: {err}"));
        let objects = objects_by_name(&pe);
        let mut bodies = Vec::new();
        let mut by_object = BTreeMap::new();
        for object in objects.values() {
            let methods = read_method_table(&pe, object.lp_object_info)
                .unwrap_or_else(|err| panic!("{key}: {}: {err}", object.name));
            if !methods.defects().is_empty() {
                failures.push(format!("{key}: {}: {:?}", object.name, methods.defects()));
            }
            entries += methods.entries.len();
            not_addresses += methods
                .entries
                .iter()
                .filter(|entry| matches!(entry, MethodEntry::NotAnAddress { .. }))
                .count();
            let found: Vec<u32> = methods.descriptors().map(|d| d.va.get()).collect();
            descriptors += found.len();
            let addresses: Vec<u32> = method_table(&pe, object)
                .into_iter()
                .filter(|value| pe.region_at_va(Va::new(*value)).is_some())
                .collect();
            if found != addresses {
                failures.push(format!(
                    "{key}: {} gives {found:x?}, and the test reads {addresses:x?}",
                    object.name
                ));
            }
            let first = methods
                .entries
                .iter()
                .position(|entry| matches!(entry, MethodEntry::Descriptor { .. }))
                .unwrap_or(methods.entries.len());
            if methods.entries[first..]
                .iter()
                .any(|entry| !matches!(entry, MethodEntry::Descriptor { .. }))
            {
                failures.push(format!(
                    "{key}: {}: an entry after a descriptor is not one",
                    object.name
                ));
            }
            for descriptor in methods.descriptors() {
                if descriptor.proc_table != object.lp_object_info {
                    failures.push(format!(
                        "{key}: {}: the descriptor at {:#x} names {:#x}",
                        object.name,
                        descriptor.va.get(),
                        descriptor.proc_table.get()
                    ));
                }
                let body = descriptor.body(&pe).unwrap();
                let start = body.file_offset(Off::new(0)).unwrap().get();
                let at = pe
                    .region_at_va(descriptor.va)
                    .unwrap()
                    .file_offset(Off::new(0))
                    .unwrap()
                    .get();
                let proc_size = u32::from(u16::from_le_bytes([
                    bytes[usize::try_from(at).unwrap() + 8],
                    bytes[usize::try_from(at).unwrap() + 9],
                ]));
                if (body.len(), start + body.len()) != (proc_size, at) {
                    failures.push(format!(
                        "{key}: the body at {start:#x} has {} bytes, and the descriptor at \
                         {at:#x} gives {proc_size}",
                        body.len()
                    ));
                }
                if start % 4 != 0 {
                    failures.push(format!("{key}: a body starts at {start:#x}"));
                }
                bodies.push((start, body.len()));
            }
            by_object.insert(object.name.clone(), found);
        }
        bodies.sort_unstable();
        for pair in bodies.windows(2) {
            if pair[0].0 + pair[0].1 > pair[1].0 {
                failures.push(format!("{key}: the bodies {pair:x?} share bytes"));
            }
        }
        let report = deform6::inspect(&bytes, &table, Mode::Strict)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        for form in &report.forms {
            for control in &form.controls {
                for event in &control.events {
                    if let EventReport::Named {
                        handler_address: Some(address),
                        ..
                    }
                    | EventReport::BoundUnnamed {
                        handler_address: Some(address),
                        ..
                    } = event
                        && !by_object[&form.name].contains(address)
                    {
                        failures.push(format!(
                            "{key}: {}.{} gives {address:#x}, which is no descriptor",
                            form.name, control.name
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} method table facts do not hold:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        (entries, descriptors, not_addresses),
        (
            EXPECTED_METHOD_ENTRIES,
            EXPECTED_DESCRIPTORS,
            EXPECTED_NOT_ADDRESSES
        ),
        "the P-code corpus now gives {entries} entries, {descriptors} descriptors and \
         {not_addresses} values that are not addresses"
    );
}

/// The word at `+0x04` of each descriptor is the argument size of its
/// procedure in the source: 4 for `Me`, 4 for each argument by reference,
/// the size of each argument by value, and 4 for the result of a `Function`
/// or a `Property Get`. `support::source` reads the size from the
/// declaration. The descriptors of an object are the procedures of its
/// source file, in the order of that file.
#[test]
fn each_pcode_descriptor_gives_the_argument_size_of_its_source_procedure() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let mut checked = 0;
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{key}: {err}"));
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let sources: BTreeMap<String, PathBuf> = vbp::Project::read(&project)
            .declared_objects()
            .into_iter()
            .filter_map(|object| Some((object.name?, object.source_file)))
            .collect();
        for object in objects_by_name(&pe).values() {
            let methods = read_method_table(&pe, object.lp_object_info)
                .unwrap_or_else(|err| panic!("{key}: {}: {err}", object.name));
            let found: Vec<u32> = methods
                .descriptors()
                .map(|descriptor| u32::from(descriptor.arg_size))
                .collect();
            let declared: Vec<(String, u32)> =
                source::declared_argument_sizes(&sources[&object.name]);
            let expected: Vec<u32> = declared.iter().map(|(_, size)| *size).collect();
            let bodies: Vec<String> = source::procedure_bodies(&sources[&object.name])
                .into_iter()
                .map(|(name, _)| name)
                .collect();
            let names: Vec<String> = declared.iter().map(|(name, _)| name.clone()).collect();
            if bodies != names {
                failures.push(format!(
                    "{key}: {} gives the bodies {bodies:?}, and the declarations {names:?}",
                    object.name
                ));
            }
            checked += found.len();
            if found != expected {
                failures.push(format!(
                    "{key}: {} gives {found:?}, and the source declares {declared:?}",
                    object.name
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} objects do not give the argument sizes of their source:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(checked, EXPECTED_DESCRIPTORS);
}

/// `GetImageDataStream` of `FastDrawing.cls` gives the lines of its body,
/// with no comment, and not the lines of the next procedure.
#[test]
fn a_procedure_body_gives_its_own_lines_and_no_comment() {
    let path = build_record::corpus_root()
        .join("vb6-code/Brightness-effect/Part 4 - Even faster DIBs/FastDrawing.cls");
    let bodies = source::procedure_bodies(&path);
    let (_, body) = bodies
        .iter()
        .find(|(name, _)| name == "GetImageDataStream")
        .unwrap();
    assert_eq!(body.first().map(String::as_str), Some("Dim bm As Bitmap"));
    assert!(body.contains(&"ReDim ImageData(0 To GetImageStreamLength(SrcPictureBox))".to_owned()));
    assert!(body.iter().all(|line| !line.contains('\'')
        && !line.starts_with("Public Sub")
        && !line.starts_with("End ")));
}

/// The method slots of the link tables of the P-code corpus: the 680
/// descriptors, less the 17 of the 8 standard modules, which have no link
/// table.
const EXPECTED_METHOD_SLOTS: usize = 663;

/// The objects of the P-code corpus with a link table: the 99 objects, less
/// the 8 standard modules.
const EXPECTED_LINK_TABLES: usize = 91;

/// The accessor slots of the public variables in the link tables of the
/// P-code corpus.
const EXPECTED_ACCESSOR_SLOTS: usize = 124;

/// The link table of each object of a P-code program holds a method slot
/// for each procedure of its source. The public procedures come first, then
/// the private ones, each group in the order of the file. Before them come
/// two accessor slots for each public variable of the source, and the two of
/// a pair name the same field.
#[test]
fn each_link_table_of_a_pcode_program_gives_the_procedures_of_its_source() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let mut methods_found = 0;
    let mut accessors = 0;
    let mut tables = 0;
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{key}: {err}"));
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let sources: BTreeMap<String, PathBuf> = vbp::Project::read(&project)
            .declared_objects()
            .into_iter()
            .filter_map(|object| Some((object.name?, object.source_file)))
            .collect();
        for object in objects_by_name(&pe).values() {
            let links = read_method_links(&pe, object)
                .unwrap_or_else(|err| panic!("{key}: {}: {err}", object.name));
            if links.slots.is_empty() {
                continue;
            }
            tables += 1;
            let descriptors: Vec<u32> = read_method_table(&pe, object.lp_object_info)
                .unwrap_or_else(|err| panic!("{key}: {}: {err}", object.name))
                .descriptors()
                .map(|descriptor| descriptor.va.get())
                .collect();
            let path = &sources[&object.name];
            let scopes = source::declared_procedure_scopes(path);
            let expected: Vec<u32> = [true, false]
                .into_iter()
                .flat_map(|public| {
                    scopes
                        .iter()
                        .zip(&descriptors)
                        .filter(move |((is_public, _), _)| *is_public == public)
                        .map(|(_, va)| *va)
                })
                .collect();
            let found: Vec<u32> = links
                .slots
                .iter()
                .filter_map(|slot| match slot {
                    LinkSlot::Method(va) => Some(va.get()),
                    _ => None,
                })
                .collect();
            let fields: Vec<u32> = links
                .slots
                .iter()
                .filter_map(|slot| match slot {
                    LinkSlot::Variable { field } => Some(*field),
                    _ => None,
                })
                .collect();
            let others = links.slots.len() - found.len();
            let variables = source::declared_public_variables(path);
            methods_found += found.len();
            accessors += fields.len();
            let paired = fields
                .chunks(2)
                .all(|pair| pair.len() == 2 && pair[0] == pair[1]);
            if found != expected || others != 2 * variables || fields.len() != others || !paired {
                failures.push(format!(
                    "{key}: {} gives the methods {found:x?}, {others} other slots and the \
                     accessor fields {fields:x?}; the source gives {expected:x?} and {variables} \
                     public variables",
                    object.name
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} link tables do not give the procedures of their source:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        (tables, methods_found, accessors),
        (
            EXPECTED_LINK_TABLES,
            EXPECTED_METHOD_SLOTS,
            EXPECTED_ACCESSOR_SLOTS
        )
    );
}

/// The class and the name of each `Begin` block of a form, depth first,
/// each name one time: the form first, then its controls. The elements of a
/// control array share one name.
fn begin_blocks(blocks: &[frm::Block], out: &mut Vec<(String, String)>) {
    for block in blocks {
        if !out.iter().any(|(_, name)| *name == block.name) {
            out.push((block.class.clone(), block.name.clone()));
        }
        begin_blocks(&block.children, out);
    }
}

/// The `ControlInfo` records of the P-code corpus that the test below
/// compares with their source.
const EXPECTED_INDEXED_CONTROLS: usize = 613;

/// The `wIndex` of each `ControlInfo` record of a form of a P-code program is
/// the position of the name of its control in the source: the form at 0,
/// then the `Begin` blocks of a class of `VB.` in the order of the file,
/// then the other `Begin` blocks, such as a Winsock control, in the order of
/// the file. The record of the form itself, named `Form`, holds `0xFFFF`.
#[test]
fn each_control_index_of_a_pcode_form_is_the_position_of_its_source_block() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let mut checked = 0;
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{key}: {err}"));
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let sources: BTreeMap<String, PathBuf> = vbp::Project::read(&project)
            .declared_objects()
            .into_iter()
            .filter_map(|object| Some((object.name?, object.source_file)))
            .collect();
        for object in objects_by_name(&pe).values() {
            let controls =
                ControlInfoTable::read(&pe, object).unwrap_or_else(|err| panic!("{key}: {err}"));
            let path = &sources[&object.name];
            if controls.entries.is_empty()
                || !path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("frm"))
            {
                continue;
            }
            let mut blocks = Vec::new();
            begin_blocks(&frm::Form::read(path).blocks(), &mut blocks);
            let (form, controls_of_source) = blocks.split_first().unwrap();
            let intrinsic = |class: &String| class.starts_with("VB.");
            let names: Vec<&String> = std::iter::once(&form.1)
                .chain(
                    controls_of_source
                        .iter()
                        .filter(|(class, _)| intrinsic(class))
                        .map(|(_, name)| name),
                )
                .chain(
                    controls_of_source
                        .iter()
                        .filter(|(class, _)| !intrinsic(class))
                        .map(|(_, name)| name),
                )
                .collect();
            for control in &controls.entries {
                checked += 1;
                let expected = if control.name == "Form" {
                    Some(0xFFFF)
                } else {
                    names
                        .iter()
                        .position(|name| **name == control.name)
                        .and_then(|at| u16::try_from(at).ok())
                };
                if Some(control.w_index) != expected {
                    failures.push(format!(
                        "{key}: {}.{} holds wIndex {}, and the source gives {expected:?}",
                        object.name, control.name, control.w_index
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} control indexes do not agree with the source:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(checked, EXPECTED_INDEXED_CONTROLS);
}

/// Adds the class and the name of each `Begin` block of `blocks` that has an
/// `Index` property: an element of a control array.
fn array_blocks(blocks: &[frm::Block], out: &mut Vec<(String, String)>) {
    for block in blocks {
        if block
            .properties
            .iter()
            .any(|property| property.name == "Index")
            && !out.iter().any(|(_, name)| *name == block.name)
        {
            out.push((block.class.clone(), block.name.clone()));
        }
        array_blocks(&block.children, out);
    }
}

/// The control arrays of a class of `VB.` in the forms of the P-code
/// corpus.
const EXPECTED_CONTROL_ARRAYS: usize = 5;

/// The `ControlInfo` record of a control array of a class of `VB.` names
/// the GUID that a single control of the same class names, plus 1 in its
/// first 32 bits. No single control names the GUID of an array.
#[test]
fn a_control_array_names_the_guid_of_its_class_plus_one() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let mut singles: BTreeMap<String, Vec<[u8; 16]>> = BTreeMap::new();
    let mut arrays = Vec::new();
    for (key, exe) in pcode_programs() {
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{key}: {err}"));
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let sources: BTreeMap<String, PathBuf> = vbp::Project::read(&project)
            .declared_objects()
            .into_iter()
            .filter_map(|object| Some((object.name?, object.source_file)))
            .collect();
        for object in objects_by_name(&pe).values() {
            let path = &sources[&object.name];
            if !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("frm"))
            {
                continue;
            }
            let controls =
                ControlInfoTable::read(&pe, object).unwrap_or_else(|err| panic!("{key}: {err}"));
            let form = frm::Form::read(path).blocks();
            let mut blocks = Vec::new();
            begin_blocks(&form, &mut blocks);
            let mut array_names = Vec::new();
            array_blocks(&form, &mut array_names);
            for control in &controls.entries {
                let Some((class, _)) = blocks.iter().find(|(_, name)| *name == control.name) else {
                    continue;
                };
                if !class.starts_with("VB.") {
                    continue;
                }
                let guid: [u8; 16] = pe
                    .region_at_va(control.lp_guid)
                    .and_then(|region| region.take(Off::new(0), 16))
                    .unwrap()
                    .try_into()
                    .unwrap();
                if array_names.iter().any(|(_, name)| *name == control.name) {
                    arrays.push((format!("{key}: {}", control.name), class.clone(), guid));
                } else {
                    singles.entry(class.clone()).or_default().push(guid);
                }
            }
        }
    }
    let mut failures = Vec::new();
    for (control, class, guid) in &arrays {
        let mut single = *guid;
        let first = u32::from_le_bytes(single[..4].try_into().unwrap());
        single[..4].copy_from_slice(&(first - 1).to_le_bytes());
        let of_class = singles.get(class).cloned().unwrap_or_default();
        if !of_class.contains(&single) || singles.values().flatten().any(|one| one == guid) {
            failures.push(format!(
                "{control} ({class}) names {}, and the single controls of its class name {:?}",
                guid_text(guid),
                of_class.iter().map(guid_text).collect::<Vec<_>>()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} control arrays do not name the GUID of their class plus one:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(arrays.len(), EXPECTED_CONTROL_ARRAYS);
}

/// The control accessors that `VbTypes::with_controls` gives across the
/// forms of the P-code corpus.
const EXPECTED_ACCESSORS: usize = 562;

/// The vtable size that the types file of the test below gives the form.
const FORM_VTABLE_SIZE: u16 = 760;

/// `VbTypes::with_controls` gives each control of a form of a P-code program
/// an accessor at the vtable size of the form plus 4 times the position of
/// the control in the source. The types file is built here from the GUIDs
/// that the binary itself names, with placeholder interfaces: the GUID of
/// the record of the form gets `_Form0` of 760 bytes, and each other GUID an
/// interface of its own.
#[test]
fn each_control_of_a_pcode_form_gets_the_accessor_of_its_source_position() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let mut accessors = 0;
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{key}: {err}"));
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let sources: BTreeMap<String, PathBuf> = vbp::Project::read(&project)
            .declared_objects()
            .into_iter()
            .filter_map(|object| Some((object.name?, object.source_file)))
            .collect();
        for object in objects_by_name(&pe).values() {
            let controls =
                ControlInfoTable::read(&pe, object).unwrap_or_else(|err| panic!("{key}: {err}"));
            let path = &sources[&object.name];
            if controls.entries.is_empty()
                || !path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("frm"))
            {
                continue;
            }
            let mut by_guid: BTreeMap<String, (String, u16)> = BTreeMap::new();
            for (n, control) in controls.entries.iter().enumerate() {
                let guid: [u8; 16] = pe
                    .region_at_va(control.lp_guid)
                    .and_then(|region| region.take(Off::new(0), 16))
                    .unwrap()
                    .try_into()
                    .unwrap();
                let entry = if control.name == "Form" {
                    ("_Form0".to_owned(), FORM_VTABLE_SIZE)
                } else {
                    (format!("_Control{n}"), 4)
                };
                by_guid.entry(guid_text(&guid)).or_insert(entry);
            }
            let mut text = String::from("[controls]\n");
            for (guid, (interface, _)) in &by_guid {
                text.push_str(&format!("\"{guid}\" = \"{interface}\"\n"));
            }
            for (interface, size) in by_guid.values() {
                text.push_str(&format!("[interfaces.{interface}]\nvtable_size = {size}\n"));
            }
            let types = VbTypes::parse(text.as_bytes()).unwrap();
            let callees = types.with_controls(Callees::default(), &pe, object);
            // A file that names the form and no control: each control gets
            // its accessor with no interface.
            let form_guid = by_guid
                .iter()
                .find(|(_, (interface, _))| interface == "_Form0")
                .map(|(guid, _)| guid.clone())
                .unwrap();
            let bare = VbTypes::parse(
                format!(
                    "[controls]\n\"{form_guid}\" = \"_Form0\"\n\
                     [interfaces._Form0]\nvtable_size = {FORM_VTABLE_SIZE}\n"
                )
                .as_bytes(),
            )
            .unwrap();
            let untyped = bare.with_controls(Callees::default(), &pe, object);

            let mut blocks = Vec::new();
            begin_blocks(&frm::Form::read(path).blocks(), &mut blocks);
            let (_, controls_of_source) = blocks.split_first().unwrap();
            let intrinsic = |class: &String| class.starts_with("VB.");
            let order: Vec<&String> = controls_of_source
                .iter()
                .filter(|(class, _)| intrinsic(class))
                .chain(
                    controls_of_source
                        .iter()
                        .filter(|(class, _)| !intrinsic(class)),
                )
                .map(|(_, name)| name)
                .collect();
            for control in controls.entries.iter().filter(|c| c.name != "Form") {
                accessors += 1;
                let position = order
                    .iter()
                    .position(|name| **name == control.name)
                    .unwrap();
                let offset = u16::try_from(4 * (position + 1)).unwrap() + FORM_VTABLE_SIZE;
                if untyped.control(offset) != Some((control.name.as_str(), None)) {
                    failures.push(format!(
                        "{key}: {}.{} has no untyped accessor at {offset:#x}",
                        object.name, control.name
                    ));
                }
                if callees.control(offset).map(|(name, _)| name) != Some(control.name.as_str()) {
                    failures.push(format!(
                        "{key}: {}.{} is not at {offset:#x}: {:?}",
                        object.name,
                        control.name,
                        callees.control(offset)
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} controls are not at the accessor of their source position:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(accessors, EXPECTED_ACCESSORS);
}

/// The GUID of the interface of a `PictureBox`, as the side structure of an
/// argument names it.
const PICTURE_BOX_IID: &str = "{33AD4ED1-6699-11CF-B70C-00AA0060D393}";

/// Each public method of `FastDrawing` in the P-code `Realtime_Brightness.exe`
/// takes a `PictureBox` as its first argument in the source, and
/// `callees_of_project` gives that argument, at frame offset `0x0C`, the
/// interface that the types file names for its GUID. The types file is
/// built here, with a placeholder name. Each method also gets the name of
/// its source procedure.
#[test]
fn a_public_method_gives_the_interface_of_its_control_argument() {
    let key = "vb6-code/Brightness-effect/Part 4 - Even faster DIBs/Realtime_Brightness.exe";
    let bytes = read(&pcode_root().join(key));
    let pe = PeImage::parse(&bytes).unwrap();
    let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
    let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
    let head = ObjectTableHead::read(&pe, info.lp_object_table).unwrap();
    let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
        .unwrap()
        .objects;
    let types =
        VbTypes::parse(format!("[iids]\n\"{PICTURE_BOX_IID}\" = \"_Picture\"\n").as_bytes())
            .unwrap();
    let callees = callees_of_project(&pe, &objects, &PcodeTable::default(), Some(&types));
    let (at, object) = objects
        .iter()
        .enumerate()
        .find(|(_, object)| object.name == "FastDrawing")
        .unwrap();
    let source = build_record::corpus_root()
        .join("vb6-code/Brightness-effect/Part 4 - Even faster DIBs/FastDrawing.cls");
    let declared = source::declared_procedure_scopes(&source);
    let indexes: Vec<u16> = read_method_table(&pe, object.lp_object_info)
        .unwrap()
        .entries
        .iter()
        .filter_map(|entry| match entry {
            MethodEntry::Descriptor { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(indexes.len(), declared.len());
    for ((is_public, name), index) in declared.iter().zip(&indexes) {
        assert!(is_public, "{name}");
        assert_eq!(
            callees[at].arguments_of(*index),
            [(0x0C, "_Picture".to_owned())],
            "{name}"
        );
        assert_eq!(callees[at].procedure(*index), *name);
    }
}

/// Gives the name that each `Declare` statement of `path` binds in the DLL:
/// the text of its `Alias`, or else its own name.
fn declared_exports(path: &Path) -> Vec<String> {
    let text = String::from_utf8_lossy(&read(path)).into_owned();
    let mut out = Vec::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(at) = words.iter().position(|word| *word == "Declare") else {
            continue;
        };
        let Some(name) = words.get(at + 2) else {
            continue;
        };
        let alias = words
            .iter()
            .position(|word| *word == "Alias")
            .and_then(|at| words.get(at + 1))
            .map(|alias| alias.trim_matches('"'));
        out.push(alias.unwrap_or(name).to_owned());
    }
    out
}

/// `ObjectInfo + 0x28`: `wConstants`, the number of entries of the
/// constant table.
const CONSTANT_COUNT_AT: u32 = 0x28;

/// Each entry of a constant table that is the stub of a `Declare` call
/// names a `Declare` entry of the program, and its export name is the
/// `Alias` or the name of a `Declare` statement of the source. No two
/// descriptors of one program give one name.
#[test]
fn each_declare_stub_of_a_pcode_program_names_a_declare_of_its_source() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let (mut stubs, mut failures) = (0, Vec::new());
    for (key, exe) in pcode_programs() {
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let exports: Vec<String> = vbp::Project::read(&project)
            .declared_objects()
            .iter()
            .flat_map(|object| declared_exports(&object.source_file))
            .collect();
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap();
        let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
        let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
        let declares = DeclareTable::read(&pe, &info);
        let mut named = BTreeMap::new();
        for object in objects_by_name(&pe).values() {
            let count = pe
                .region_at_va(object.lp_object_info)
                .and_then(|region| region.u16_le(Off::new(CONSTANT_COUNT_AT)))
                .unwrap_or(0);
            for index in 0..count {
                let Some(descriptor) = constant_declare(&pe, object.lp_object_info, index) else {
                    continue;
                };
                stubs += 1;
                match declares.export_at(&pe, descriptor) {
                    Some(name) if exports.contains(&name) => {
                        named.insert(descriptor, name);
                    }
                    other => failures.push(format!(
                        "{key}: {} constant {index:#x} gives {other:?}",
                        object.name
                    )),
                }
            }
        }
        let mut names: Vec<&String> = named.values().collect();
        names.sort();
        names.dedup();
        if names.len() != named.len() {
            failures.push(format!("{key}: two descriptors give one name: {named:x?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!(stubs, 186);
}

/// Each entry of a constant table that is the stub of a procedure of the
/// project names a descriptor of the method table of one object. A stub
/// that takes no object goes to a module of the source, and a stub that
/// takes an object goes to an object that is not a module.
#[test]
fn each_procedure_stub_of_a_pcode_program_goes_to_an_object_of_its_kind() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let (mut modules, mut objects, mut failures) = (0, 0, Vec::new());
    for (key, exe) in pcode_programs() {
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let kinds: BTreeMap<String, vbp::ObjectKind> = vbp::Project::read(&project)
            .declared_objects()
            .into_iter()
            .filter_map(|object| Some((object.name?, object.kind)))
            .collect();
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap();
        let by_name = objects_by_name(&pe);
        let owners: BTreeMap<u32, &String> = by_name
            .iter()
            .flat_map(|(name, object)| {
                method_table(&pe, object)
                    .into_iter()
                    .map(move |descriptor| (descriptor, name))
            })
            .collect();
        for object in by_name.values() {
            let count = pe
                .region_at_va(object.lp_object_info)
                .and_then(|region| region.u16_le(Off::new(CONSTANT_COUNT_AT)))
                .unwrap_or(0);
            for index in 0..count {
                let Some(stub) = constant_procedure(&pe, object.lp_object_info, index) else {
                    continue;
                };
                let kind = owners
                    .get(&stub.descriptor.get())
                    .and_then(|owner| kinds.get(*owner));
                let is_module = kind == Some(&vbp::ObjectKind::Module);
                if stub.of_object {
                    objects += 1;
                } else {
                    modules += 1;
                }
                if kind.is_none() || stub.of_object == is_module {
                    failures.push(format!(
                        "{key}: {} constant {index:#x} goes to {kind:?}",
                        object.name
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!((modules, objects), (21, 19));
}

/// A types file with the events of the Winsock control, in the order of
/// their vtable offsets, by the GUID of its events interface.
const WINSOCK_EVENTS: &str = r#"
[[events."{248DD893-BB45-11CF-9ABC-0080C7E7B78D}"]]
name = "Error"
parameters = ["ByVal Number As Integer", "Description As String", "ByVal Scode As Long", "ByVal Source As String", "ByVal HelpFile As String", "ByVal HelpContext As Long", "CancelDisplay As Boolean"]
[[events."{248DD893-BB45-11CF-9ABC-0080C7E7B78D}"]]
name = "DataArrival"
parameters = ["ByVal bytesTotal As Long"]
[[events."{248DD893-BB45-11CF-9ABC-0080C7E7B78D}"]]
name = "Connect"
parameters = []
[[events."{248DD893-BB45-11CF-9ABC-0080C7E7B78D}"]]
name = "ConnectionRequest"
parameters = ["ByVal requestID As Long"]
[[events."{248DD893-BB45-11CF-9ABC-0080C7E7B78D}"]]
name = "Close"
parameters = []
[[events."{248DD893-BB45-11CF-9ABC-0080C7E7B78D}"]]
name = "SendProgress"
parameters = ["ByVal bytesSent As Long", "ByVal bytesRemaining As Long"]
[[events."{248DD893-BB45-11CF-9ABC-0080C7E7B78D}"]]
name = "SendComplete"
parameters = []
"#;

/// Each bound event slot of a Winsock control of the two P-code programs
/// that hold one gets the name of its event from the types file, and the
/// handler is the procedure of the source named for the control and that
/// event. The control of `Server.exe` is a control array.
#[test]
fn each_bound_slot_of_a_winsock_control_names_the_event_of_its_source_handler() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let types = VbTypes::parse(WINSOCK_EVENTS.as_bytes()).unwrap();
    let mut named = Vec::new();
    for key in [
        "public-domain/SK-TFTP-Sample__VB6/Client/demo/TFTPClient.exe",
        "public-domain/SK-TFTP-Sample__VB6/Server/demo/Server.exe",
    ] {
        let bytes = read(&pcode_root().join(key));
        let pe = PeImage::parse(&bytes).unwrap();
        let objects = objects_by_name(&pe);
        let project = vbp::select_project_file(&root.join(key), &projects).unwrap();
        let sources: BTreeMap<String, PathBuf> = vbp::Project::read(&project)
            .declared_objects()
            .into_iter()
            .filter_map(|object| Some((object.name?, object.source_file)))
            .collect();
        let report = deform6::inspect_with_types(
            &bytes,
            &OpcodeTable::builtin(),
            Some(&types),
            Mode::Strict,
        )
        .unwrap();
        for form in &report.forms {
            let methods = method_table(&pe, &objects[&form.name]);
            let procedures = source::declared_procedures(&sources[&form.name]);
            let first = methods.len() - procedures.len();
            for control in &form.controls {
                for event in &control.events {
                    let EventReport::Named {
                        control_name,
                        event_name,
                        handler_address: Some(address),
                        ..
                    } = event
                    else {
                        continue;
                    };
                    let at = methods.iter().position(|method| method == address).unwrap();
                    assert_eq!(
                        procedures[at - first],
                        format!("{control_name}_{event_name}"),
                        "{key}"
                    );
                    named.push(format!("{control_name}_{event_name}"));
                }
            }
        }
    }
    assert_eq!(
        named,
        [
            "WskClient_Error",
            "WskClient_DataArrival",
            "WskClient_Connect",
            "WskClient_Close",
            "WskClient_SendProgress",
            "WskClient_SendComplete",
            "WskServer_Error",
            "WskServer_DataArrival",
            "WskServer_ConnectionRequest",
            "WskServer_Close",
        ]
    );
}

/// `extract` writes each handler of a Winsock control of the two P-code
/// programs that hold one with the declaration line of its source, and
/// with `Index As Integer` first for the control array of `Server.exe`.
#[test]
fn each_handler_of_a_winsock_control_is_written_with_its_source_declaration() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let types = VbTypes::parse(WINSOCK_EVENTS.as_bytes()).unwrap();
    let mut written = 0;
    for key in [
        "public-domain/SK-TFTP-Sample__VB6/Client/demo/TFTPClient.exe",
        "public-domain/SK-TFTP-Sample__VB6/Server/demo/Server.exe",
    ] {
        let bytes = read(&pcode_root().join(key));
        let report = deform6::inspect_with_types(
            &bytes,
            &OpcodeTable::builtin(),
            Some(&types),
            Mode::Strict,
        )
        .unwrap();
        let project = deform6::write::project(&report, &bytes, Mode::Strict).unwrap();
        let project_file = vbp::select_project_file(&root.join(key), &projects).unwrap();
        let source: Vec<String> = vbp::Project::read(&project_file)
            .declared_objects()
            .iter()
            .flat_map(|object| {
                String::from_utf8_lossy(&read(&object.source_file))
                    .lines()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect();
        for file in project
            .files
            .iter()
            .filter(|file| file.name.ends_with(".frm"))
        {
            for line in String::from_utf8_lossy(&file.bytes).lines() {
                if line.starts_with("Private Sub Wsk") {
                    assert!(source.iter().any(|own| own == line), "{key}: {line}");
                    written += 1;
                }
            }
        }
    }
    assert_eq!(written, 10);
}

/// Gives the profile of each object of the P-code binary at `path`.
fn profiles(path: &Path) -> Vec<Callees> {
    let bytes = read(path);
    let pe = PeImage::parse(&bytes).unwrap();
    let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
    let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
    let head = ObjectTableHead::read(&pe, info.lp_object_table).unwrap();
    let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
        .unwrap()
        .objects;
    callees_of_project(&pe, &objects, &PcodeTable::default(), None)
}

/// Gives the form that each profile of the P-code program `key` names.
fn named_forms(key: &str) -> Vec<String> {
    profiles(&pcode_root().join(key))
        .iter()
        .filter_map(Callees::form_name)
        .map(str::to_owned)
        .collect()
}

/// Each profile of a P-code program names its object, and the names are
/// the names of the objects that the source project declares. `New` of a
/// class of the project writes this name.
#[test]
fn each_profile_names_an_object_of_the_source() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let mut declared: Vec<String> = vbp::Project::read(&project)
            .declared_objects()
            .iter()
            .map(|object| object.name.clone().unwrap_or_default())
            .collect();
        let mut named: Vec<String> = profiles(&exe)
            .iter()
            .map(|profile| profile.object_name().unwrap_or_default().to_owned())
            .collect();
        declared.sort();
        named.sort();
        if declared != named {
            failures.push(format!("{key}: {named:?} against {declared:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `callees_of_project` names the form of each profile that is a form, and
/// no other: `PassGen.exe` holds 4 forms, in the order of its project file,
/// and `Blacklight.exe` holds 1 form and 2 classes.
#[test]
fn the_profile_of_a_form_names_the_form() {
    assert_eq!(
        named_forms("public-domain/PassGen/PassGen.exe"),
        ["frmPassGen", "frmSpecial", "frmOverride", "frmAbout"]
    );
    assert_eq!(
        named_forms("vb6-code/Blacklight-effect/Blacklight.exe"),
        ["frmBlacklight"]
    );
}

/// A name that `callees_of_project_named` takes replaces the name of the
/// procedure at its index, in the profile of its object only.
#[test]
fn a_given_name_replaces_the_name_of_a_procedure() {
    let bytes = read(&pcode_root().join("public-domain/PassGen/PassGen.exe"));
    let pe = PeImage::parse(&bytes).unwrap();
    let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
    let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
    let head = ObjectTableHead::read(&pe, info.lp_object_table).unwrap();
    let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
        .unwrap()
        .objects;
    let names = vec![vec![
        (0, "Renamed".to_owned()),
        (7, "cmdClose_Click".to_owned()),
    ]];
    let callees = callees_of_project_named(&pe, &objects, &PcodeTable::default(), None, &names);
    assert_eq!(callees[0].procedure(0), "Renamed");
    assert_eq!(callees[0].procedure(7), "cmdClose_Click");
    assert_eq!(callees[0].procedure(3), "makeSpecialString");
    assert_eq!(callees[1].procedure(0), "method_0");
}

/// `lift_objects` gives each procedure of `TFTPClient.exe` the declaration
/// that `extract` writes: a Winsock handler by its event, with the
/// parameters of the event. With no P-code table, each body says that the
/// lift stopped, and `write::project` writes that body into the form.
#[test]
fn each_lifted_procedure_takes_the_declaration_that_extract_writes() {
    let key = "public-domain/SK-TFTP-Sample__VB6/Client/demo/TFTPClient.exe";
    let bytes = read(&pcode_root().join(key));
    let types = VbTypes::parse(WINSOCK_EVENTS.as_bytes()).unwrap();
    let report =
        deform6::inspect_with_types(&bytes, &OpcodeTable::builtin(), Some(&types), Mode::Strict)
            .unwrap();
    let lifted =
        deform6::vb::bodies::lift_objects(&bytes, &report, &PcodeTable::default(), Some(&types))
            .unwrap();
    assert_eq!(lifted.len(), report.objects.len());
    let form = report
        .objects
        .iter()
        .position(|object| object.name == "Form1")
        .unwrap();
    let procedures = &lifted[form].as_ref().unwrap().procedures;
    let declarations: Vec<&str> = procedures
        .iter()
        .map(|procedure| procedure.declaration.as_str())
        .collect();
    assert!(
        declarations.contains(&"Private Sub WskClient_DataArrival(ByVal bytesTotal As Long)"),
        "{declarations:#?}"
    );
    for procedure in procedures {
        assert!(
            procedure.lines[0].contains("The lift stopped"),
            "{:?}",
            procedure.lines
        );
    }
    let mut report = report;
    for (object, own) in report.objects.iter_mut().zip(lifted) {
        object.lifted = own;
    }
    let project = deform6::write::project(&report, &bytes, Mode::Strict).unwrap();
    let form = project
        .files
        .iter()
        .find(|file| file.name == "Form1.frm")
        .unwrap();
    let text = String::from_utf8_lossy(&form.bytes);
    assert!(
        text.contains(
            "Private Sub WskClient_DataArrival(ByVal bytesTotal As Long)\r\n    ' The lift stopped"
        ),
        "{text}"
    );
}

/// Counts the public variables that the source file at `path` declares at
/// the level of the module: each `Public` line that declares no procedure
/// and no other item, one per name.
fn public_variables(path: &Path) -> usize {
    let text = String::from_utf8_lossy(&read(path)).into_owned();
    let items = [
        "Sub ",
        "Function ",
        "Property ",
        "Const ",
        "Declare ",
        "Enum ",
        "Type ",
        "Event ",
    ];
    text.lines()
        .filter_map(|line| line.strip_prefix("Public "))
        .filter(|rest| !items.iter().any(|item| rest.starts_with(item)))
        .map(|rest| rest.split(',').count())
        .sum()
}

/// Each public variable of the source of `frmPassGen` has an accessor in
/// the vtable of the form, and so a field that the lift declares `Public`.
#[test]
fn each_public_variable_of_a_form_gives_a_public_field() {
    let dir = build_record::corpus_root().join("public-domain/PassGen");
    let form = profiles(&pcode_root().join("public-domain/PassGen/PassGen.exe"))
        .into_iter()
        .find(|profile| profile.object_name() == Some("frmPassGen"))
        .unwrap();
    let expected = public_variables(&dir.join("frmPassGen.frm"));
    assert!(expected > 0);
    assert_eq!(form.variable_fields().len(), expected);
}

/// `GetImageData2D` of `FastDrawing` takes `fixOrientation` `ByVal` at
/// 0x14, and `SetImageData2D` takes `imgWidth`, `imgHeight` and
/// `fixOrientation` `ByVal` at 0x10, 0x14 and 0x1C. Each other argument of
/// the class is `ByRef`.
#[test]
fn each_by_value_argument_of_a_public_method_gives_its_slot() {
    let class = profiles(&pcode_root().join("vb6-code/Transparency-2D/Transparency.exe"))
        .into_iter()
        .find(|profile| profile.object_name() == Some("FastDrawing"))
        .unwrap();
    let found: Vec<Vec<i16>> = (0..8)
        .map(|method| class.value_arguments_of(method))
        .filter(|slots| !slots.is_empty())
        .collect();
    assert_eq!(found, [vec![0x14], vec![0x10, 0x14, 0x1C]]);
}

/// Counts the fixed-size arrays that the procedures of the source file at
/// `path` declare with `Dim`: each name of a `Dim` line in a procedure with
/// bounds between its parentheses, such as `bTable(0 To 255)`.
fn source_fixed_arrays(path: &Path) -> usize {
    let text = String::from_utf8_lossy(&read(path)).into_owned();
    let mut inside = false;
    let mut count = 0;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        let starts = |word: &str| {
            words.iter().take(3).any(|w| *w == word)
                && !trimmed.starts_with("End ")
                && !trimmed.starts_with("Exit ")
                && !trimmed.starts_with("Declare ")
                && !trimmed.contains(" Declare ")
        };
        if starts("Sub") || starts("Function") || starts("Property") {
            inside = true;
        }
        if trimmed.starts_with("End Sub")
            || trimmed.starts_with("End Function")
            || trimmed.starts_with("End Property")
        {
            inside = false;
        }
        if !inside || !trimmed.starts_with("Dim ") {
            continue;
        }
        let mut rest = trimmed;
        while let Some(open) = rest.find('(') {
            let after = &rest[open + 1..];
            let Some(close) = after.find(')') else {
                break;
            };
            if !after[..close].trim().is_empty() {
                count += 1;
            }
            rest = &after[close + 1..];
        }
    }
    count
}

/// Each fixed-size local array of the source is a fixed-array entry of the
/// descriptor of its procedure, and no other entry is one.
#[test]
fn each_fixed_array_of_a_source_procedure_is_an_entry_of_its_descriptor() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let mut failures = Vec::new();
    let mut total = 0;
    for (key, exe) in pcode_programs() {
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let expected: usize = vbp::Project::read(&project)
            .declared_objects()
            .iter()
            .filter(|object| object.source_file.exists())
            .map(|object| source_fixed_arrays(&object.source_file))
            .sum();
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap();
        let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
        let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
        let head = ObjectTableHead::read(&pe, info.lp_object_table).unwrap();
        let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
            .unwrap()
            .objects;
        let found: usize = objects
            .iter()
            .filter_map(|object| read_method_table(&pe, object.lp_object_info).ok())
            .flat_map(|table| table.entries)
            .map(|entry| match entry {
                MethodEntry::Descriptor { descriptor, .. } => descriptor.fixed_arrays(&pe).len(),
                _ => 0,
            })
            .sum();
        total += found;
        if found != expected {
            failures.push(format!("{key}: {found} found, {expected} in the source"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(total, EXPECTED_FIXED_ARRAYS);
}

/// The fixed-size local arrays of the P-code corpus.
const EXPECTED_FIXED_ARRAYS: usize = 24;

/// Counts the fixed-size arrays that the declarations section of the source
/// file at `path` declares: each name with bounds between its parentheses
/// on a `Dim`, `Private`, `Public` or `Global` line before the first
/// procedure, and outside a `Type` block.
fn source_module_arrays(path: &Path) -> usize {
    let text = String::from_utf8_lossy(&read(path)).into_owned();
    let mut in_type = false;
    let mut count = 0;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let code = trimmed.split('\'').next().unwrap_or_default();
        let words: Vec<&str> = code.split_whitespace().collect();
        if words
            .iter()
            .take(3)
            .any(|w| matches!(*w, "Sub" | "Function" | "Property"))
            && !words.contains(&"Declare")
        {
            break;
        }
        if words.first() == Some(&"End") && words.get(1) == Some(&"Type") {
            in_type = false;
            continue;
        }
        if words.iter().take(2).any(|w| *w == "Type") {
            in_type = true;
            continue;
        }
        let declares = matches!(
            words.first(),
            Some(&("Dim" | "Private" | "Public" | "Global"))
        ) && !words
            .iter()
            .take(2)
            .any(|w| matches!(*w, "Declare" | "Const" | "Enum" | "Event"));
        if in_type || !declares {
            continue;
        }
        let mut rest = code;
        while let Some(open) = rest.find('(') {
            let after = &rest[open + 1..];
            let Some(close) = after.find(')') else {
                break;
            };
            if !after[..close].trim().is_empty() {
                count += 1;
            }
            rest = &after[close + 1..];
        }
    }
    count
}

/// Each fixed-size array of the declarations section of a source file is an
/// entry of the table of the public variables of its object, and no other
/// entry is one.
#[test]
fn each_fixed_array_of_a_source_module_is_an_entry_of_its_object() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let mut failures = Vec::new();
    let mut total = 0;
    for (key, exe) in pcode_programs() {
        let project = vbp::select_project_file(&root.join(&key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let expected: usize = vbp::Project::read(&project)
            .declared_objects()
            .iter()
            .filter(|object| object.source_file.exists())
            .map(|object| source_module_arrays(&object.source_file))
            .sum();
        let bytes = read(&exe);
        let pe = PeImage::parse(&bytes).unwrap();
        let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
        let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
        let head = ObjectTableHead::read(&pe, info.lp_object_table).unwrap();
        let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
            .unwrap()
            .objects;
        let found: usize = objects
            .iter()
            .map(|object| module_fixed_arrays(&pe, object.lp_public_bytes).len())
            .sum();
        total += found;
        if found != expected {
            failures.push(format!("{key}: {found} found, {expected} in the source"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(total, EXPECTED_MODULE_ARRAYS);
}

/// The fixed-size arrays of the declarations sections of the P-code corpus.
const EXPECTED_MODULE_ARRAYS: usize = 10;

/// A fixed array of a module as its offset, its `VARTYPE`, its bounds and
/// the bytes of one element.
type ModuleArrayFacts = (u16, u16, Vec<(u32, i32)>, u32);

/// `Public Bullets(0 To NumOfBullets) As Bullet` and `Public StarArray(0 To
/// NumOfStars) As Star` of `Physics_Logic.bas` are at 0x2C and 0x48 of the
/// data of the module. `NumOfBullets` is 75 and `NumOfStars` is 500. Each
/// record is 16 bytes: `Star` holds three `Single` and a `Byte`, and `Bullet`
/// holds three `Long` and a `Boolean`. A record has no `VARTYPE`.
#[test]
fn a_module_array_of_records_gives_its_offset_its_bounds_and_its_record_bytes() {
    let bytes = read(&pcode_root().join("vb6-code/Game-physics-basic/Physics_Demo.exe"));
    let pe = PeImage::parse(&bytes).unwrap();
    let objects = objects_by_name(&pe);
    let found: Vec<ModuleArrayFacts> =
        module_fixed_arrays(&pe, objects["Logic_Module"].lp_public_bytes)
            .into_iter()
            .map(|array| (array.slot, array.vartype, array.bounds, array.element_bytes))
            .collect();
    assert_eq!(
        found,
        [(0x2C, 0, vec![(76, 0)], 16), (0x48, 0, vec![(501, 0)], 16)]
    );
    assert!(module_fixed_arrays(&pe, objects["frmMain"].lp_public_bytes).is_empty());
}

/// `Dim hData(0 To 3, 0 To 255) As Single` of the basic histogram viewer
/// is at 0x38 of the data of its form. The template holds the last
/// dimension first.
#[test]
fn a_module_array_template_holds_the_last_dimension_first() {
    let bytes = read(&pcode_root().join("vb6-code/Histograms-basic/Basic Histogram Viewer.exe"));
    let pe = PeImage::parse(&bytes).unwrap();
    let objects = objects_by_name(&pe);
    let found: Vec<ModuleArrayFacts> =
        module_fixed_arrays(&pe, objects["frmHistogram"].lp_public_bytes)
            .into_iter()
            .map(|array| (array.slot, array.vartype, array.bounds, array.element_bytes))
            .collect();
    assert_eq!(found.first(), Some(&(0x38, 4, vec![(256, 0), (4, 0)], 4)));
}

/// A fixed array as its slot, its `VARTYPE` and its bounds.
type FixedArrayFacts = (u16, u16, Vec<(u32, i32)>);

/// Gives the fixed arrays of the descriptor at `va` of the P-code program
/// `key`, as the slot, the `VARTYPE` and the bounds.
fn fixed_arrays_at(key: &str, va: u32) -> Vec<FixedArrayFacts> {
    let bytes = read(&pcode_root().join(key));
    let pe = PeImage::parse(&bytes).unwrap();
    let descriptor = deform6::vb::procdesc::ProcDescriptor::read(&pe, Va::new(va)).unwrap();
    descriptor
        .fixed_arrays(&pe)
        .into_iter()
        .map(|array| (array.slot, array.vartype, array.bounds))
        .collect()
}

/// `Dim bTable(0 To 255) As Long` of `DrawVBBrightness` is at `-0xB0`, and
/// `Dim rData(0 To 255) As Long, gData(0 To 255) As Long, bData(0 To 255)
/// As Long` of the advanced histogram are at `-0x100`, `-0xE4` and `-0xC8`.
/// `VARTYPE` 3 is `Long`.
#[test]
fn a_fixed_array_gives_its_slot_its_type_and_its_bounds() {
    assert_eq!(
        fixed_arrays_at(
            "vb6-code/Brightness-effect/Part 1 - Pure VB6/vbBrightness.exe",
            0x0040_d61c
        ),
        [(0xB0, 3, vec![(256, 0)])]
    );
    assert_eq!(
        fixed_arrays_at(
            "vb6-code/Histograms-advanced/Advanced Histogram Viewer.exe",
            0x0040_7440
        ),
        [
            (0x100, 3, vec![(256, 0)]),
            (0xE4, 3, vec![(256, 0)]),
            (0xC8, 3, vec![(256, 0)])
        ]
    );
}
