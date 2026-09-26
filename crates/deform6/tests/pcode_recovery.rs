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
//! - **The event handlers.** DeForm6 does not decode the event stub of a
//!   P-code program. Each bound event slot has no handler address, and gives
//!   an `UnknownStubShape` defect. This is a named limit, and its test fails
//!   when the limit closes.

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

use std::path::{Path, PathBuf};

use deform6::error::DefectKind;
use deform6::journal::Mode;
use deform6::vb::Report;
use deform6::vb::classify::ObjectKind as RecoveredKind;
use deform6::vb::controlinfo::EventReport;
use deform6::vb::opcodes::OpcodeTable;
use support::vbp;

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

/// A named limit: DeForm6 does not decode the event stub of a P-code
/// program. Each P-code program has the bound event slots of its native
/// build, none of them gives a handler address, and each handler address of
/// the native build is one `UnknownStubShape` defect in the P-code build.
#[test]
fn each_bound_event_slot_of_a_pcode_program_has_no_handler_address() {
    let root = build_record::corpus_root();
    let table = OpcodeTable::builtin();
    let mut native_addresses = 0;
    let mut failures = Vec::new();
    for (key, exe) in pcode_programs() {
        let native = deform6::inspect(&read(&root.join(&key)), &table, Mode::Salvage)
            .unwrap_or_else(|err| panic!("{key} native: {err}"));
        let pcode = deform6::inspect(&read(&exe), &table, Mode::Salvage)
            .unwrap_or_else(|err| panic!("{key} P-code: {err}"));
        let (native_bound, addresses) = bound_slots(&native);
        let (pcode_bound, pcode_addresses) = bound_slots(&pcode);
        let unknown_stubs = pcode
            .defects
            .iter()
            .filter(|defect| matches!(defect.kind, DefectKind::UnknownStubShape { .. }))
            .count();
        native_addresses += addresses;
        let found = (pcode_bound, pcode_addresses, unknown_stubs);
        if found != (native_bound, 0, addresses) {
            failures.push(format!(
                "{key}: P-code gives {pcode_bound} bound slots, {pcode_addresses} handler \
                 addresses and {unknown_stubs} unknown stubs; native gives {native_bound} bound \
                 slots and {addresses} handler addresses"
            ));
        }
    }
    assert!(
        native_addresses > 0,
        "the native builds give no handler address"
    );
    assert!(
        failures.is_empty(),
        "The limit on P-code event stubs moved. If DeForm6 now decodes a P-code stub, the limit \
         is closed: change this test and the README.\n{}",
        failures.join("\n")
    );
}
