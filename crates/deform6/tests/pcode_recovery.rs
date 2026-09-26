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
use deform6::vb::controlinfo::{
    ControlInfoTable, EventReport, EventSlot, StubShape, read_event_table,
};
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::object::{Object, ObjectTable};
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::project::{ObjectTableHead, ProjectInfo};
use object::LittleEndian as LE;
use object::pe::ImageNtHeaders32;
use object::read::pe::{ImageNtHeaders, ImageOptionalHeader, Import, PeFile32};
use support::{source, vbp};

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
    let mut lines: Vec<&[u8]> = bytes.split(|byte| *byte == b'\n').collect();
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
/// `corpus/` holds its objects in another order than its source. The test
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
/// `ControlInfo`.
const EXPECTED_REPORTED_HANDLERS: usize = 388;

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
