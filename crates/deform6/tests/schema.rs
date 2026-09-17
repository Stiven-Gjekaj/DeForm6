#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The schema check.
//!
//! `crates/deform6/schema/report.schema.json` is a second source of truth
//! for the report [`deform6::write::project`] writes. This file proves that
//! every `report.json` a real run of the corpus produces holds no shape the
//! committed schema refuses, and that the schema itself refuses a shape it
//! was never asked to accept. A pass here is not proof that the schema is
//! complete: it proves only that the reports measured on this run hold no
//! shape the schema refuses.
//!
//! The schema is read from `schema/report.schema.json` on disk, once,
//! and compiled by [`jsonschema::validator_for`]. It is never rebuilt from
//! the Rust types at run time: a schema derived from the types would agree
//! with a bug in those types, and the disagreement this file exists to
//! catch would be invisible.

use std::path::{Path, PathBuf};

use deform6::error::{Defect, DefectKind, Site};
use deform6::vb::opcodes::OpcodeTable;
use jsonschema::Validator;

/// Reads the committed schema from disk and compiles it once.
fn compiled_schema() -> Validator {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/report.schema.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    let value: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("parsing {}: {err}", path.display()));
    jsonschema::validator_for(&value)
        .unwrap_or_else(|err| panic!("compiling {}: {err}", path.display()))
}

/// Every error `validator` finds in `value`, one line per error, naming the
/// instance path and the validator's own message.
fn describe_errors(validator: &Validator, value: &serde_json::Value) -> Vec<String> {
    validator
        .iter_errors(value)
        .map(|err| format!("{}: {err}", err.instance_path()))
        .collect()
}

/// Reads, inspects, writes and serialises one corpus program, and gives back
/// the parsed JSON value its report produces.
fn report_value_for(exe: &Path) -> serde_json::Value {
    let data = std::fs::read(exe).unwrap_or_else(|err| panic!("{}: reading: {err}", exe.display()));
    let table = OpcodeTable::builtin();
    let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict)
        .unwrap_or_else(|err| panic!("{}: inspect refused this program: {err}", exe.display()));
    let written = deform6::write::project(&report, &data, deform6::journal::Mode::Strict)
        .unwrap_or_else(|err| {
            panic!(
                "{}: write::project refused this program: {err}",
                exe.display()
            )
        });
    let text = written.report.to_json();
    serde_json::from_str(&text).unwrap_or_else(|err| {
        panic!(
            "{}: parsing the report this run produced: {err}",
            exe.display()
        )
    })
}

/// One corpus program, read, inspected, written and validated against the
/// committed schema. This is the tracer: it drives one program through the
/// whole path before the full 44-program sweep exists.
#[test]
fn one_corpus_report_validates_against_the_committed_schema() {
    let validator = compiled_schema();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let exe = root.join("public-domain/PassGen/PassGen.exe");
    let value = report_value_for(&exe);

    let errors = describe_errors(&validator, &value);
    assert!(
        errors.is_empty(),
        "{}: report failed schema validation:\n{}",
        exe.display(),
        errors.join("\n")
    );
}

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks `corpus/` recursively and gives every file whose extension is
/// `.exe`, compared case-insensitively, sorted.
///
/// Copied from `tests/corpus_sweep.rs` rather than walked a second way;
/// that file's own doc comment explains why the compare is
/// case-insensitive and why the count assertion below is what actually
/// catches a corpus that changed.
fn executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(&corpus_root(), &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("reading an entry of {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push(path);
        }
    }
}

/// Every one of the 44 corpus programs produces a `report.json` that holds
/// no shape the committed schema refuses.
#[test]
fn every_corpus_report_validates_against_the_committed_schema() {
    let validator = compiled_schema();
    let files = executables();
    let count = files.len();
    assert_eq!(count, 44, "found {count} corpus executables, wanted 44");

    let mut failed = Vec::new();
    for exe in &files {
        let key = exe
            .strip_prefix(corpus_root())
            .unwrap_or(exe)
            .display()
            .to_string();
        let value = report_value_for(exe);
        for error in describe_errors(&validator, &value) {
            failed.push(format!("{key}: {error}"));
        }
    }
    assert!(
        failed.is_empty(),
        "{} schema violation(s) across {count} corpus programs:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// Proves the schema has teeth: four separate doctored copies of one real
/// report each fail validation, one doctored shape at a time. A test that
/// cannot fail is worse than no test, and this test is the proof that the
/// other two can.
#[test]
fn the_schema_refuses_a_doctored_report() {
    let validator = compiled_schema();
    let exe = corpus_root().join("vb6-code/Fire-effect/Fast_Flames.exe");
    let value = report_value_for(&exe);

    assert!(
        validator.is_valid(&value),
        "{}: the undoctored report must itself validate, or this test proves nothing",
        exe.display()
    );

    // 1. A fourth confidence word the enum does not hold.
    let items = value["items"].as_array().expect("items must be an array");
    assert!(
        !items.is_empty(),
        "{}: this program produced no report items, and case 1 needs one",
        exe.display()
    );
    let mut confidence_doctored = value.clone();
    confidence_doctored["items"][0]["confidence"] = serde_json::json!("dubious");
    assert!(
        !validator.is_valid(&confidence_doctored),
        "a confidence value outside proven/inferred/unrecoverable must be refused"
    );

    // 2. The `limits` key removed.
    let mut limits_doctored = value.clone();
    limits_doctored
        .as_object_mut()
        .expect("the report's root must be a JSON object")
        .remove("limits");
    assert!(
        !validator.is_valid(&limits_doctored),
        "a report missing the required limits key must be refused"
    );

    // 3. An extra top-level key the root schema does not allow.
    let mut extra_key_doctored = value.clone();
    extra_key_doctored
        .as_object_mut()
        .expect("the report's root must be a JSON object")
        .insert("extra".to_owned(), serde_json::json!(true));
    assert!(
        !validator.is_valid(&extra_key_doctored),
        "a report carrying an extra top-level key must be refused"
    );

    // 4. The first defect's `kind` renamed to a variant name that is not in
    //    the enum. Skipped, loudly, if this program raised no defect.
    let defects = value["defects"]
        .as_array()
        .expect("defects must be an array");
    assert!(
        !defects.is_empty(),
        "{}: this program produced no defect, and case 4 needs one; pick a program \
         that raises at least one defect",
        exe.display()
    );
    let mut kind_doctored = value.clone();
    let payload = kind_doctored["defects"][0]["kind"]
        .as_object()
        .expect("a defect's kind must be a JSON object")
        .values()
        .next()
        .expect("a DefectKind object holds exactly one key")
        .clone();
    let mut renamed = serde_json::Map::new();
    renamed.insert("NotARealDefectKind".to_owned(), payload);
    kind_doctored["defects"][0]["kind"] = serde_json::Value::Object(renamed);
    assert!(
        !validator.is_valid(&kind_doctored),
        "a defect kind renamed to a variant the schema does not name must be refused"
    );
}

/// A report that carries a `ModuleMarkerMismatch` defect validates.
///
/// No corpus program raises this kind, so the sweep above never shows its
/// shape to the schema. The defect here is the serializer's own output for a
/// value this test builds, added to a real report. The schema is still the
/// committed file.
#[test]
fn a_report_that_carries_a_module_marker_mismatch_validates() {
    let validator = compiled_schema();
    let exe = corpus_root().join("vb6-code/Fire-effect/Fast_Flames.exe");
    let mut value = report_value_for(&exe);
    let defect = Defect {
        site: Site {
            offset: 0x1a3c,
            rva: Some(0x2a3c),
            structure: "ObjectInfo",
            field: "lpPrivateObject",
        },
        kind: DefectKind::ModuleMarkerMismatch {
            offset: 0x1a3c,
            pointer: 0xFFFF_FFFF,
            object_type: 0x0001_8083,
        },
    };
    value["defects"]
        .as_array_mut()
        .expect("defects must be an array")
        .push(serde_json::to_value(&defect).expect("a defect serializes"));

    let errors = describe_errors(&validator, &value);
    assert!(
        errors.is_empty(),
        "a report with a ModuleMarkerMismatch defect failed schema validation:\n{}",
        errors.join("\n")
    );
}

/// A report that carries an `UnknownStubShape` defect validates, and the
/// schema refuses the same defect with one byte too many or one byte out of
/// range.
///
/// No corpus program raises this kind, for the reason the test above gives
/// for its own kind.
#[test]
fn a_report_that_carries_an_unknown_stub_shape_validates() {
    let validator = compiled_schema();
    let exe = corpus_root().join("vb6-code/Fire-effect/Fast_Flames.exe");
    let mut value = report_value_for(&exe);
    let defect = Defect {
        site: Site {
            offset: 0x19d0,
            rva: Some(0x19d0),
            structure: "EventStub",
            field: "opcode",
        },
        kind: DefectKind::UnknownStubShape {
            offset: 0x19d0,
            found: [
                0x33, 0xC0, 0xBA, 0xD0, 0x19, 0x40, 0x00, 0x68, 0xD0, 0x19, 0x40, 0x00, 0xC3,
            ],
        },
    };
    let serialized = serde_json::to_value(&defect).expect("a defect serializes");
    let at = value["defects"]
        .as_array()
        .expect("defects must be an array")
        .len();
    value["defects"]
        .as_array_mut()
        .expect("defects must be an array")
        .push(serialized);

    let errors = describe_errors(&validator, &value);
    assert!(
        errors.is_empty(),
        "a report with an UnknownStubShape defect failed schema validation:\n{}",
        errors.join("\n")
    );

    let mut too_long = value.clone();
    too_long["defects"][at]["kind"]["UnknownStubShape"]["found"]
        .as_array_mut()
        .expect("found must be an array")
        .push(serde_json::json!(0));
    assert!(
        !validator.is_valid(&too_long),
        "a stub of fourteen bytes must be refused"
    );

    let mut out_of_range = value.clone();
    out_of_range["defects"][at]["kind"]["UnknownStubShape"]["found"][0] = serde_json::json!(256);
    assert!(
        !validator.is_valid(&out_of_range),
        "a byte above 255 must be refused"
    );
}

/// A report that carries an `ItemCutShort` defect validates, and the schema
/// refuses the same defect when one of its three fields is missing.
///
/// No corpus program raises this kind, for the reason the tests above give
/// for their own kinds.
#[test]
fn a_report_that_carries_an_item_cut_short_validates() {
    let validator = compiled_schema();
    let exe = corpus_root().join("vb6-code/Mandelbrot/Mandelbrot.exe");
    let mut value = report_value_for(&exe);
    let defect = Defect {
        site: Site {
            offset: 0x1ae0,
            rva: Some(0x1ae0),
            structure: "DeclareTableEntry",
            field: "lpImportDescriptor",
        },
        kind: DefectKind::ItemCutShort {
            offset: 0x1ae0,
            va: 0x0040_3FFC,
            len: 8,
        },
    };
    let serialized = serde_json::to_value(&defect).expect("a defect serializes");
    let at = value["defects"]
        .as_array()
        .expect("defects must be an array")
        .len();
    value["defects"]
        .as_array_mut()
        .expect("defects must be an array")
        .push(serialized);

    let errors = describe_errors(&validator, &value);
    assert!(
        errors.is_empty(),
        "a report with an ItemCutShort defect failed schema validation:\n{}",
        errors.join("\n")
    );

    for field in ["offset", "va", "len"] {
        let mut missing = value.clone();
        missing["defects"][at]["kind"]["ItemCutShort"]
            .as_object_mut()
            .expect("the kind must be an object")
            .remove(field);
        assert!(
            !validator.is_valid(&missing),
            "a defect with no {field} must be refused"
        );
    }
}
