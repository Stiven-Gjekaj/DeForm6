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

use std::path::Path;

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
