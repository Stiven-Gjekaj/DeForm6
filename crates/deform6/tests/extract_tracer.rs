#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The end to end proof that one corpus program becomes a project
//! directory whose bytes match the committed source it was built from.
//!
//! `AGENTS.md`, "What a test may hold on to": the ground truth is the
//! committed source the executable was built from, never a second write
//! compared against itself. Every assertion below reads either the
//! committed `corpus/vb6-code/Fire-effect/*` files, or the bytes
//! `deform6::write::project` itself returned.

use std::path::{Path, PathBuf};

use deform6::vb::opcodes::OpcodeTable;
use deform6::write::{self, WrittenFile, WrittenProject};

/// `corpus/vb6-code/Fire-effect/Fast_Flames.exe`, this task's one tracer
/// program.
const FAST_FLAMES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/vb6-code/Fire-effect/Fast_Flames.exe"
));

/// Gives the absolute path to a file committed beside `Fast_Flames.exe`.
fn corpus_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/vb6-code/Fire-effect")
        .join(relative)
}

/// Runs `inspect` then `write::project` over `Fast_Flames.exe`, with the
/// builtin opcode table, the same table the command line uses when the
/// user supplies no `--opcode-table` of their own.
fn written_project() -> WrittenProject {
    let table = OpcodeTable::builtin();
    let report =
        deform6::inspect(FAST_FLAMES, &table).expect("Fast_Flames.exe must inspect cleanly");
    write::project(&report, FAST_FLAMES).expect("write::project must not refuse a clean report")
}

/// Gives the one written file named `name`, or fails loudly naming every
/// file that was actually written.
fn file_named<'a>(project: &'a WrittenProject, name: &str) -> &'a WrittenFile {
    project
        .files
        .iter()
        .find(|file| file.name == name)
        .unwrap_or_else(|| {
            let names: Vec<&str> = project.files.iter().map(|f| f.name.as_str()).collect();
            panic!("no written file named {name:?}; the run wrote {names:?}")
        })
}

/// Splits a written text file's bytes into its own lines, decoding every
/// byte as its own Latin-1 code point, this crate's own reading
/// convention.
fn lines_of(file: &WrittenFile) -> Vec<String> {
    let text: String = file.bytes.iter().copied().map(char::from).collect();
    text.split("\r\n").map(str::to_owned).collect()
}

#[test]
fn the_written_file_list_holds_exactly_the_five_expected_files_and_nothing_else() {
    let project = written_project();
    let mut names: Vec<&str> = project.files.iter().map(|f| f.name.as_str()).collect();
    names.sort_unstable();

    let mut expected = vec![
        "VBFire2.vbp",
        "frmFire.frm",
        "frmFire.frx",
        "FastDrawing.cls",
        "VBFire2.report.json",
    ];
    expected.sort_unstable();

    assert_eq!(names, expected, "written files: {names:?}");
}

#[test]
fn the_written_frx_equals_the_committed_source_byte_for_byte() {
    let project = written_project();
    let written = file_named(&project, "frmFire.frx");
    let committed =
        std::fs::read(corpus_path("frmFire.frx")).expect("reading the committed frmFire.frx");

    assert_eq!(
        committed.len(),
        1418,
        "the committed frmFire.frx is 1418 bytes"
    );
    assert_eq!(
        written.bytes, committed,
        "the written .frx must equal the committed source byte for byte, all 1418 bytes"
    );
}

#[test]
fn the_written_frm_opens_with_version_and_a_correctly_closed_begin_line() {
    let project = written_project();
    let written = file_named(&project, "frmFire.frm");
    let lines = lines_of(written);

    assert_eq!(lines.first().map(String::as_str), Some("VERSION 5.00"));

    let begin_line = lines
        .iter()
        .find(|line| line.starts_with("Begin VB.Form"))
        .expect("the .frm must open a VB.Form block");
    assert_eq!(
        begin_line, "Begin VB.Form frmFire ",
        "the Begin line must end with exactly one trailing space: {begin_line:?}"
    );
}

#[test]
fn the_icon_property_names_the_frx_file_and_an_uppercase_hex_offset_of_at_least_four_digits() {
    let project = written_project();
    let written = file_named(&project, "frmFire.frm");
    let lines = lines_of(written);

    let icon_line = lines
        .iter()
        .find(|line| line.trim_start().starts_with("Icon"))
        .expect("the .frm must carry an Icon property line");
    let value = icon_line
        .split("=   ")
        .nth(1)
        .expect("the Icon line must carry a value after the padded name and the =");

    assert!(
        value.starts_with("\"frmFire.frx\":"),
        "the Icon value must quote the .frx file name: {value:?}"
    );
    let hex = value.trim_start_matches("\"frmFire.frx\":");
    assert!(
        hex.len() >= 4,
        "the offset must be padded to at least four digits: {hex:?}"
    );
    assert!(
        hex.chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_lowercase()),
        "the offset must be upper case hexadecimal: {hex:?}"
    );
}

#[test]
fn the_icon_offset_names_a_record_that_ends_inside_the_written_frx() {
    let project = written_project();
    let frm = file_named(&project, "frmFire.frm");
    let frx = file_named(&project, "frmFire.frx");
    let lines = lines_of(frm);

    let icon_line = lines
        .iter()
        .find(|line| line.trim_start().starts_with("Icon"))
        .expect("the .frm must carry an Icon property line");
    let value = icon_line
        .split("=   ")
        .nth(1)
        .expect("the Icon line must carry a value");
    let hex = value.trim_start_matches("\"frmFire.frx\":");
    let offset = u32::from_str_radix(hex, 16).expect("the offset must parse as hexadecimal");

    let start = usize::try_from(offset).unwrap();
    let declared_len_bytes: [u8; 4] = frx.bytes[start..start + 4]
        .try_into()
        .expect("four bytes must be readable at the declared offset");
    let declared_len = u32::from_le_bytes(declared_len_bytes);
    let end = start + 4 + usize::try_from(declared_len).unwrap();

    assert!(
        end <= frx.bytes.len(),
        "the record ending at {end} must be inside the written .frx of {} bytes",
        frx.bytes.len()
    );
}

#[test]
fn the_written_frm_carries_the_five_attribute_lines_with_the_forms_own_creatable_and_predeclared_values()
 {
    let project = written_project();
    let written = file_named(&project, "frmFire.frm");
    let lines = lines_of(written);

    for expected in [
        "Attribute VB_Name = \"frmFire\"",
        "Attribute VB_GlobalNameSpace = False",
        "Attribute VB_Creatable = False",
        "Attribute VB_PredeclaredId = True",
        "Attribute VB_Exposed = False",
    ] {
        assert!(
            lines.iter().any(|line| line == expected),
            "missing {expected:?} in {lines:?}"
        );
    }
}

#[test]
fn the_written_vbp_holds_type_exe_first_one_form_one_class_and_a_matching_startup() {
    let project = written_project();
    let written = file_named(&project, "VBFire2.vbp");
    let lines: Vec<String> = lines_of(written)
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect();

    assert_eq!(lines.first().map(String::as_str), Some("Type=Exe"));

    let form_lines: Vec<&String> = lines
        .iter()
        .filter(|line| line.starts_with("Form="))
        .collect();
    assert_eq!(form_lines.len(), 1, "{lines:?}");
    assert_eq!(form_lines[0], "Form=frmFire.frm");

    let class_lines: Vec<&String> = lines
        .iter()
        .filter(|line| line.starts_with("Class="))
        .collect();
    assert_eq!(class_lines.len(), 1, "{lines:?}");
    assert_eq!(class_lines[0], "Class=FastDrawing; FastDrawing.cls");

    let startup = lines
        .iter()
        .find(|line| line.starts_with("Startup="))
        .expect("the .vbp must carry a Startup= line");
    assert_eq!(startup, "Startup=\"frmFire\"");
}

#[test]
fn the_written_cls_holds_the_locked_preamble_and_the_classs_own_creatable_and_predeclared_values() {
    let project = written_project();
    let written = file_named(&project, "FastDrawing.cls");
    let lines = lines_of(written);

    assert_eq!(lines.first().map(String::as_str), Some("VERSION 1.0 CLASS"));
    assert_eq!(lines.get(1).map(String::as_str), Some("BEGIN"));
    for expected in [
        "  MultiUse = -1  'True",
        "  Persistable = 0  'NotPersistable",
        "  DataBindingBehavior = 0  'vbNone",
        "  DataSourceBehavior  = 0  'vbNone",
        "  MTSTransactionMode  = 0  'NotAnMTSObject",
        "END",
        "Attribute VB_Name = \"FastDrawing\"",
        "Attribute VB_Creatable = True",
        "Attribute VB_PredeclaredId = False",
    ] {
        assert!(
            lines.iter().any(|line| line == expected),
            "missing {expected:?} in {lines:?}"
        );
    }
}

#[test]
fn every_written_text_file_ends_with_crlf_and_holds_no_byte_order_mark() {
    let project = written_project();
    for file in &project.files {
        if file.name.ends_with(".frx") || file.name.ends_with(".report.json") {
            continue;
        }
        assert!(
            file.bytes.ends_with(b"\r\n"),
            "{}: does not end with CRLF",
            file.name
        );
        assert!(
            !file.bytes.starts_with(&[0xEF, 0xBB, 0xBF]),
            "{}: begins with a UTF-8 byte order mark",
            file.name
        );
    }
}

#[test]
fn two_calls_to_write_project_give_byte_identical_output_for_every_file() {
    let table = OpcodeTable::builtin();
    let report = deform6::inspect(FAST_FLAMES, &table).unwrap();
    let first = write::project(&report, FAST_FLAMES).unwrap();
    let second = write::project(&report, FAST_FLAMES).unwrap();
    assert_eq!(first.files, second.files);
}

// --- Plan 04-08: `write::project` wired to the full writers and to
// `report::build`, so the shipped report carries real items and real
// limits instead of the `items: []`/`limits: []` every prior plan in this
// phase left in place. ---

/// Roadmap success criterion 5: `report.json`'s own `items` array holds at
/// least one item of confidence `inferred`, and every item carries a
/// non-empty basis and at least one evidence record with a byte offset.
#[test]
fn the_written_reports_items_hold_a_real_inferred_path_and_every_item_carries_evidence() {
    let project = written_project();

    assert!(
        !project.report.items.is_empty(),
        "write::project must no longer ship items: []"
    );

    let inferred_paths: Vec<&str> = project
        .report
        .items
        .iter()
        .filter(|item| item.confidence == deform6::report::Confidence::Inferred)
        .map(|item| item.path.as_str())
        .collect();
    assert!(
        !inferred_paths.is_empty(),
        "at least one item must be graded inferred, with a real path: {:?}",
        project.report.items
    );
    for path in &inferred_paths {
        assert!(
            !path.is_empty(),
            "an inferred item's own path must not be empty"
        );
    }

    for item in &project.report.items {
        assert!(
            !item.basis.is_empty(),
            "every item must carry a non-empty basis: {item:?}"
        );
        assert!(
            !item.evidence.is_empty(),
            "every item must carry at least one evidence record: {item:?}"
        );
    }
}

/// Roadmap named risk: the report must state that full recompilation did
/// not run, and it must never imply the IDE opened this project.
#[test]
fn the_written_reports_limits_state_that_full_recompilation_did_not_run() {
    let project = written_project();
    assert!(
        !project.report.limits.is_empty(),
        "write::project must no longer ship limits: []"
    );
    assert!(
        project
            .report
            .limits
            .iter()
            .any(|limit| limit.contains("recompilation did not run")),
        "{:?}",
        project.report.limits
    );
    assert!(
        project
            .report
            .limits
            .iter()
            .any(|limit| limit.contains("never opened this project in the IDE")),
        "a limit line must state, in plain words, that the IDE never opened this project: {:?}",
        project.report.limits
    );
}

// --- Plan 04-01, Task 3: `ProjectModel` observed on the tracer's own file --

/// One assertion per new model fact plan 04-01 task 3 adds, all observed on
/// `Fast_Flames.exe` itself, so `ProjectModel` is exercised by the end to
/// end path and not only by the unit tests in `write::model`.
#[test]
fn from_report_on_fast_flames_gives_one_form_one_class_and_a_matching_startup() {
    let table = OpcodeTable::builtin();
    let report = deform6::inspect(FAST_FLAMES, &table).unwrap();
    let (model, items) = deform6::write::model::from_report(&report, FAST_FLAMES);

    // One form, frmFire, whose control tree walk did not refuse.
    assert_eq!(model.forms.len(), 1, "{:?}", model.forms);
    let form = &model.forms[0];
    assert_eq!(form.name.as_str(), "frmFire");
    assert!(
        !form.tree_refused,
        "frmFire's own control tree walk did not refuse on this corpus file"
    );

    // One class, FastDrawing, and no module.
    assert_eq!(model.code.len(), 1, "{:?}", model.code);
    assert_eq!(model.code[0].name.as_str(), "FastDrawing");
    assert_eq!(model.code[0].kind, deform6::write::model::CodeKind::Class);

    // The startup form names frmFire, the only form this project declares,
    // and the choice is recorded as an inferred report item.
    match &model.startup {
        deform6::write::model::Startup::Form(name) => assert_eq!(name.as_str(), "frmFire"),
        other => panic!("expected Startup::Form(\"frmFire\"), got {other:?}"),
    }
    assert!(
        items.iter().any(
            |item| item.confidence == deform6::report::Confidence::Inferred
                && item.basis.contains("does not declare a startup form")
        ),
        "{items:?}"
    );

    // Exactly one resource blob, the form's own Icon, at control index 0
    // (the form's own outermost block).
    assert_eq!(form.blobs.len(), 1, "{:?}", form.blobs);
    assert_eq!(form.blobs[0].control_index, 0);
    assert_eq!(form.blobs[0].property_name, "Icon");

    // Every control's own depth is reachable and bounded by the tree's own
    // size; the form's own root sits at depth 0.
    assert_eq!(form.controls.first().map(|control| control.depth), Some(0));
    assert!(
        form.controls
            .iter()
            .all(|control| control.depth <= form.controls.len())
    );
}
