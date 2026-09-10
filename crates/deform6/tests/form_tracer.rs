#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "this is the test harness, not the library under test: it reads a vendored, \
              fixed corpus this repository controls, so the strict input-hostility \
              discipline `src/` carries does not apply (the threat model's T-02-35 accepts \
              this, because the harness is not exposed to hostile input the way the parser \
              reading a real VB6 executable is)"
)]

//! One form, end to end: from `LockWorkStation.exe`'s bytes to a recovered
//! form name, checked against the committed `FrmLockWorkStation.frm`.
//!
//! **This test names one program and one form on purpose. It is the
//! phase's tracer, not its differential gate.** Plan 03-10 builds the
//! differential gate over all 44 corpus programs, comparing every recovered
//! form and control against `tests/support/frm.rs`'s independent reader.
//! This file proves the one path phase 3 depends on end to end, before any
//! later plan builds on it.

use std::path::Path;

use deform6::read::pe::PeImage;
use deform6::vb::gui::GuiTable;
use deform6::vb::header::{VbHeader, header_region};

/// The corpus program this tracer reads.
const LOCK_WORK_STATION: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/public-domain/LockWorkStation/LockWorkStation.exe"
));

/// The path to the committed source the executable was built from.
fn frm_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/public-domain/LockWorkStation/FrmLockWorkStation.frm")
}

/// Reads the `Begin VB.Form <name>` line out of the committed `.frm` and
/// gives the name.
///
/// The file is read as bytes, never with `fs::read_to_string`: a corpus
/// `.frm` this session measured is not valid UTF-8
/// (`corpus/vb6-code/Threshold-effect/Threshold.frm`), and a reader that
/// refuses such a file hides the one it dropped rather than reading past
/// it. Every byte becomes its own Latin-1 code point, the rule
/// `vb/project.rs` and `vb/object.rs` both use for a string the file holds.
/// The name is the third whitespace separated word of the line
/// (`Begin`, `VB.Form`, the name), trimmed.
fn declared_form_name() -> String {
    let bytes = std::fs::read(frm_path()).expect("reading the committed .frm");
    let text: String = bytes.iter().copied().map(char::from).collect();
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("Begin VB.Form"))
        .expect("the .frm holds a Begin VB.Form line");
    line.split_whitespace()
        .nth(2)
        .expect("a Begin VB.Form line names the form in its third word")
        .trim()
        .to_owned()
}

#[test]
fn the_recovered_form_name_and_type_agree_with_the_committed_source() {
    let image = PeImage::parse(LOCK_WORK_STATION).unwrap();
    let hdr = header_region(&image).unwrap();
    let header = VbHeader::read(&hdr).unwrap();

    let table = GuiTable::walk(&image, &header).unwrap();
    assert_eq!(
        table.entries.len(),
        1,
        "LockWorkStation.exe declares one form"
    );

    let info =
        deform6::vb::gui::GuiObjectInfo::read(&image, table.entries[0].a_form_pointer).unwrap();
    let stream = info.form_stream().unwrap();

    let recovered_name = stream.name().expect("the form's own block names it");
    let expected_name = declared_form_name();
    assert_eq!(
        recovered_name, expected_name,
        "the recovered form name does not match the name FrmLockWorkStation.frm declares"
    );
    assert_eq!(recovered_name, "FrmLockWorkStation");

    // STRUCTURES.md section 8.4.1: cType 13 is Form.
    assert_eq!(stream.control_type(), Some(13));
}
