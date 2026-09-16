#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The shape of an event stub, as `inspect` reports it.
//!
//! `STRUCTURES.md` section 8.6 gives the native stub as `81 6C 24 04
//! <imm32>`, then `E9 <rel32>`. `inspect` decodes a stub only when it has
//! that shape. A stub of another shape gives `DefectKind::UnknownStubShape`
//! at the first byte of the stub, and its slot stays bound with no handler
//! address. The defect is `Tolerated`, so a strict run reports it and
//! continues.
//!
//! No corpus program has such a stub, so the tests below patch SK-Gradient
//! in memory. Its form's `Command1` names one stub, in slot 0. `AGENTS.md`
//! bars a committed patched program.
//!
//! # The address comes from the reading API, and the bytes from this file
//!
//! The public reading API gives the address that slot 0 holds, as in
//! `tests/events.rs`. This file resolves that address, writes the P-code
//! stub of section 8.6 over the native stub, and builds the defect it
//! expects from its own bytes.
//!
//! # This file keeps its own corpus walk
//!
//! Copied rather than shared, for the reason `tests/corpus_sweep.rs` gives:
//! two corpus tests must be able to fail independently.

use std::path::{Path, PathBuf};

use deform6::error::{Defect, DefectKind, Site};
use deform6::journal::Mode;
use deform6::read::pe::PeImage;
use deform6::read::region::Va;
use deform6::vb::classify::{self, ObjectKind};
use deform6::vb::controlinfo::{ControlInfoTable, EventReport, EventSlot, read_event_table};
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::object::ObjectTable;
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::project::{ObjectTableHead, ProjectInfo};

/// The number of executables the corpus vendors.
const EXPECTED_EXECUTABLE_COUNT: usize = 44;

/// The number of bound events whose handler address `inspect` reports
/// across the corpus.
///
/// Measured on 2026-09-16. This is not a number of stubs. `inspect` reports
/// the events of a `ControlInfo` for each control that joins it by name, and
/// all the elements of a control array join the same one. So one stub can
/// give more than one event here: 34 of the 396 events belong to elements of
/// control arrays.
const EXPECTED_REPORTED_HANDLERS: usize = 396;

/// The native stub bytes at `+0x00` (`STRUCTURES.md` section 8.6).
const SUB_OPCODE: [u8; 4] = [0x81, 0x6C, 0x24, 0x04];

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks `corpus/` recursively and gives every file whose extension is
/// `.exe`, compared case-insensitively.
fn executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_dir(&corpus_root(), &mut out);
    out.sort();
    out
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("reading an entry of {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push(path);
        }
    }
}

/// Reads SK-Gradient, the program every patch in this file starts from.
fn sk_gradient() -> Vec<u8> {
    let path = corpus_root().join("public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe");
    std::fs::read(&path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

/// The address that slot 0 of `Command1` holds, read through the public
/// reading API.
fn command1_stub(data: &[u8]) -> Va {
    let image = PeImage::parse(data).unwrap();
    let hdr = header_region(&image).unwrap();
    let header = VbHeader::read(&hdr).unwrap();
    let info = ProjectInfo::read(&image, header.lp_project_data).unwrap();
    let head = ObjectTableHead::read(&image, info.lp_object_table).unwrap();
    let table = ObjectTable::walk(&image, info.lp_object_table, &head).unwrap();
    let form = table
        .objects
        .iter()
        .find(|object| classify::classify(object.f_object_type) == ObjectKind::Form)
        .unwrap();
    let controls = ControlInfoTable::read(&image, form).unwrap();
    let command1 = controls
        .entries
        .iter()
        .find(|entry| entry.name == "Command1")
        .unwrap();
    let events = read_event_table(&image, command1).unwrap();
    match events.slots[0] {
        EventSlot::Bound { stub, .. } => stub,
        EventSlot::Unbound { .. } => panic!("slot 0 of Command1 must be bound"),
    }
}

/// The P-code stub of `STRUCTURES.md` section 8.6: `xor eax,eax`,
/// `mov edx,<addr>`, `push <addr>`, `ret`, 13 bytes.
fn p_code_stub(addr: u32) -> [u8; 13] {
    let mut stub = [0_u8; 13];
    stub[0x00..0x03].copy_from_slice(&[0x33, 0xC0, 0xBA]);
    stub[0x03..0x07].copy_from_slice(&addr.to_le_bytes());
    stub[0x07] = 0x68;
    stub[0x08..0x0C].copy_from_slice(&addr.to_le_bytes());
    stub[0x0C] = 0xC3;
    stub
}

/// SK-Gradient with a P-code stub written over the native stub that slot 0
/// of `Command1` names, and the defect that stub must give.
fn patched_sk_gradient() -> (Vec<u8>, Defect) {
    let mut data = sk_gradient();
    let stub = command1_stub(&data);
    let (at, rva) = {
        let pe = PeImage::parse(&data).unwrap();
        (
            pe.va_to_off(stub).unwrap().get(),
            stub.to_rva(pe.image_base()).unwrap().get(),
        )
    };
    let start = usize::try_from(at).unwrap();
    assert_eq!(
        data[start..start + 4],
        SUB_OPCODE,
        "the stub must be native before the patch, or this test proves nothing"
    );
    let p_code = p_code_stub(stub.get());
    data[start..start + 13].copy_from_slice(&p_code);

    let expected = Defect {
        site: Site {
            offset: at,
            rva: Some(rva),
            structure: "EventStub",
            field: "opcode",
        },
        kind: DefectKind::UnknownStubShape {
            offset: at,
            found: p_code,
        },
    };
    (data, expected)
}

/// Every defect that a run of `data` in `mode` raises and that a run of the
/// unpatched SK-Gradient in the same mode does not.
fn new_defects(data: &[u8], mode: Mode) -> Vec<Defect> {
    let table = OpcodeTable::builtin();
    let before = deform6::inspect(&sk_gradient(), &table, mode).unwrap();
    let after = deform6::inspect(data, &table, mode)
        .unwrap_or_else(|err| panic!("a {mode:?} run refused the patched file: {err}"));
    after
        .defects
        .into_iter()
        .filter(|defect| !before.defects.contains(defect))
        .collect()
}

/// The event that slot 0 of `Command1` reports in a strict run of `data`.
fn command1_slot_zero(data: &[u8]) -> EventReport {
    let report = deform6::inspect(data, &OpcodeTable::builtin(), Mode::Strict).unwrap();
    report
        .forms
        .iter()
        .flat_map(|form| &form.controls)
        .find(|control| control.name == "Command1")
        .and_then(|control| control.events.first())
        .cloned()
        .unwrap()
}

#[test]
fn no_corpus_program_raises_an_unknown_stub_shape() {
    let files = executables();
    assert_eq!(files.len(), EXPECTED_EXECUTABLE_COUNT);
    let mut handlers = 0_usize;
    let mut found = Vec::new();
    for path in &files {
        let data = std::fs::read(path).unwrap();
        let report = deform6::inspect(&data, &OpcodeTable::builtin(), Mode::Strict)
            .unwrap_or_else(|err| panic!("{}: inspect refused it: {err}", path.display()));
        for defect in &report.defects {
            if matches!(defect.kind, DefectKind::UnknownStubShape { .. }) {
                found.push(format!("{}: {defect}", path.display()));
            }
        }
        handlers += report
            .forms
            .iter()
            .flat_map(|form| &form.controls)
            .flat_map(|control| &control.events)
            .filter(|event| {
                matches!(
                    event,
                    EventReport::Named {
                        handler_address: Some(_),
                        ..
                    } | EventReport::BoundUnnamed {
                        handler_address: Some(_),
                        ..
                    }
                )
            })
            .count();
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    assert_eq!(handlers, EXPECTED_REPORTED_HANDLERS);
}

#[test]
fn a_p_code_stub_raises_one_unknown_stub_shape_at_the_stub_and_nothing_else() {
    // Before the shape check, this stub gave an UnreadablePointer at the
    // slot, because its last byte makes the jump go below address 0.
    let (data, expected) = patched_sk_gradient();
    assert_eq!(new_defects(&data, Mode::Salvage), vec![expected]);
}

#[test]
fn a_strict_run_reports_an_unknown_stub_shape_and_continues() {
    let (data, expected) = patched_sk_gradient();
    assert_eq!(
        expected.kind.severity(),
        deform6::error::Severity::Tolerated
    );
    assert_eq!(new_defects(&data, Mode::Strict), vec![expected]);
}

#[test]
fn the_slot_of_a_p_code_stub_stays_bound_and_reports_no_handler_address() {
    let before = command1_slot_zero(&sk_gradient());
    assert!(
        matches!(
            before,
            EventReport::BoundUnnamed {
                index: 0,
                handler_address: Some(_),
                ..
            }
        ),
        "{before:?}"
    );
    let (data, _expected) = patched_sk_gradient();
    let after = command1_slot_zero(&data);
    assert!(
        matches!(
            after,
            EventReport::BoundUnnamed {
                index: 0,
                handler_address: None,
                ..
            }
        ),
        "{after:?}"
    );
}
