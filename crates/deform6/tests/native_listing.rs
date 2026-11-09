#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! Asks the native corpus programs for their annotated disassembly, and
//! compares the answer with the source that each program was built from.

#[allow(dead_code, reason = "each test file uses a part of the harness")]
mod support;

use deform6::read::pe::{ImportName, PeImage};
use deform6::vb::native::{Note, disassemble, handler_names};
use deform6::vb::opcodes::OpcodeTable;
use support::vbp;

/// Each bound event handler that the report gives is an entry of the
/// listing, so the listing misses no procedure that a form names.
#[test]
fn each_event_handler_of_a_form_is_a_procedure_of_the_listing() {
    let table = OpcodeTable::builtin();
    let mut handlers = 0;
    for exe in &vbp::executables() {
        let data = std::fs::read(exe).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict).unwrap();
        let listing = disassemble(&pe);
        for (address, name) in handler_names(&report) {
            handlers += 1;
            assert!(
                listing
                    .procedures
                    .iter()
                    .any(|procedure| procedure.entry.get() == address),
                "{}: {name} at {address:08X}",
                exe.display()
            );
        }
    }
    assert!(handlers > 300, "{handlers} handlers");
}

/// The source of `LockWorkStation` holds one `Shell` of one literal. The
/// listing gives the literal, and the helper that copies it.
#[test]
fn lockworkstation_gives_its_literal_and_its_helper() {
    let path = vbp::corpus_root().join("public-domain/LockWorkStation/LockWorkStation.exe");
    let data = std::fs::read(path).unwrap();
    let pe = PeImage::parse(&data).unwrap();
    let listing = disassemble(&pe);
    assert_eq!(listing.procedures.len(), 1);
    let notes: Vec<&Note> = listing.procedures[0]
        .lines
        .iter()
        .flat_map(|line| &line.notes)
        .collect();
    assert!(
        notes
            .iter()
            .any(|note| **note == Note::Text("rundll32 user32,LockWorkStation".to_owned())),
        "{notes:?}"
    );
    assert!(
        notes
            .iter()
            .any(|note| matches!(note, Note::Import(function)
            if function.name == ImportName::Name("__vbaStrCopy".to_owned()))),
        "{notes:?}"
    );
}
