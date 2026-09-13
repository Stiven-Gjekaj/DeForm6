#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! Tasks 2 and 3's own end to end proof: one patched field refuses in
//! strict and produces the report under salvage, both `inspect` and
//! `write::project` read the same bytes in both modes, and a salvage
//! report's own JSON names every assumption it made.
//!
//! The patched bytes never reach disk. `AGENTS.md` bars a fixture
//! calculated from a third party file from entering this repository; a byte
//! array built in memory, once per test run, and never written anywhere,
//! is not a fixture and does not enter it. This follows `tests/refusal.rs`'s
//! own precedent: start from a corpus file already committed under a
//! redistributable licence (`corpus/NOTICES`), patch one field in a
//! `Vec<u8>`, and keep it there for the one test function that needs it.

use deform6::Refusal;
use deform6::inspect;
use deform6::journal::Mode;
use deform6::report::ProjectReport;
use deform6::vb::opcodes::OpcodeTable;

/// `corpus/vb6-code/Fire-effect/Fast_Flames.exe`, read once at compile time.
///
/// Chosen because plan 05-01's own research measured, this session, that
/// patching its `VBHeader.wExternalCount` field to `1` raises exactly one
/// `NoNulTerminator` defect at file offset `0x1da4`: the file declares zero
/// external components, so a phantom entry walks off the end of a table
/// that holds nothing, and the name field the walk reaches next runs off
/// the end of its own bounded window with no terminator in it.
const FAST_FLAMES: &[u8] = include_bytes!("../../../corpus/vb6-code/Fire-effect/Fast_Flames.exe");

/// Copies `data` and writes `value` into the two bytes at
/// `VBHeader.wExternalCount`, learning the header's own file offset from a
/// salvage run over the unpatched bytes rather than a literal, so a future
/// header layout change would move this patch site along with it.
fn with_external_count(data: &[u8], table: &OpcodeTable, value: u16) -> Vec<u8> {
    let unpatched = inspect(data, table, Mode::Salvage)
        .expect("the shipped file must inspect cleanly in salvage mode");
    let at = unpatched
        .header_offset
        .get()
        .checked_add(0x46)
        .expect("the header offset plus 0x46 must not overflow a u32");
    let at = usize::try_from(at).expect("a file offset must fit a usize on every supported host");
    let mut patched = data.to_vec();
    assert_ne!(
        &patched[at..at + 2],
        value.to_le_bytes(),
        "the fixture writes the value the field already holds, so it proves nothing"
    );
    patched[at..at + 2].copy_from_slice(&value.to_le_bytes());
    patched
}

/// Runs `inspect` then `write::project` over `data`, both in `mode`, and
/// gives back the `ProjectReport` the write side built. Never lets the two
/// calls take different modes, the same rule `run_extract` follows.
fn built_project_report(data: &[u8], table: &OpcodeTable, mode: Mode) -> ProjectReport {
    let inspected = inspect(data, table, mode).expect("this call must inspect cleanly");
    deform6::write::project(&inspected, data, mode)
        .expect("write::project must not refuse a clean report")
        .report
}

/// The prefix every assumption line `report::assumption_lines` builds
/// starts with. A test that greps for this exact prefix cannot mistake one
/// of the four fixed limits lines (the recompilation notice, the opcode
/// table summary, the inline string notice, the code page notice) for an
/// assumption line, because none of those four starts with it.
const ASSUMPTION_LINE_PREFIX: &str = "Assumed at offset";

#[test]
fn the_unpatched_file_succeeds_in_both_modes_with_equal_defect_lists() {
    let table = OpcodeTable::builtin();
    let strict = inspect(FAST_FLAMES, &table, Mode::Strict)
        .expect("Fast_Flames.exe raises only Tolerated defects, so strict mode must succeed");
    let salvage = inspect(FAST_FLAMES, &table, Mode::Salvage)
        .expect("Fast_Flames.exe must inspect cleanly in salvage mode");
    assert_eq!(
        strict.defects, salvage.defects,
        "both modes read the same bytes, so an undamaged file's defect list must be identical"
    );
}

#[test]
fn a_patched_external_count_refuses_in_strict_and_names_the_offset() {
    let table = OpcodeTable::builtin();
    let patched = with_external_count(FAST_FLAMES, &table, 1);

    let refusal = inspect(&patched, &table, Mode::Strict)
        .expect_err("a phantom external component table entry must refuse in strict mode");
    let message = format!("{refusal}");
    assert!(
        message.contains("0x1da4"),
        "the refusal must name the byte offset 0x1da4: {message}"
    );
    assert!(
        message.contains("no nul terminator"),
        "the refusal must name what the parser expected there: {message}"
    );
}

#[test]
fn the_same_patched_file_succeeds_in_salvage_with_one_more_defect() {
    let table = OpcodeTable::builtin();
    let unpatched = inspect(FAST_FLAMES, &table, Mode::Salvage)
        .expect("the shipped file must inspect cleanly in salvage mode");
    let patched = with_external_count(FAST_FLAMES, &table, 1);

    let salvage_report = inspect(&patched, &table, Mode::Salvage)
        .expect("salvage mode must continue past the recoverable defect and produce a report");

    assert_eq!(
        salvage_report.defects.len(),
        unpatched.defects.len() + 1,
        "the patched salvage report must hold exactly one more defect than the unpatched one"
    );

    let new_defect_names_the_offset = salvage_report
        .defects
        .iter()
        .any(|defect| format!("{defect}").contains("0x1da4"));
    assert!(
        new_defect_names_the_offset,
        "the new defect must name the byte offset 0x1da4: {:?}",
        salvage_report.defects
    );
}

#[test]
fn an_empty_input_and_a_one_byte_input_refuse_as_not_pe_in_both_modes() {
    let table = OpcodeTable::builtin();
    for mode in [Mode::Strict, Mode::Salvage] {
        assert_eq!(
            inspect(&[], &table, mode),
            Err(Refusal::NotPe),
            "a zero length input must refuse as NotPe in {mode:?}"
        );
        assert_eq!(
            inspect(&[0], &table, mode),
            Err(Refusal::NotPe),
            "a one byte input must refuse as NotPe in {mode:?}"
        );
    }
}

#[test]
fn a_strict_report_holds_the_mode_line_and_no_assumption_line() {
    let table = OpcodeTable::builtin();
    let report = built_project_report(FAST_FLAMES, &table, Mode::Strict);

    assert!(
        report
            .limits
            .iter()
            .any(|line| line.contains("strict run") && line.contains("assumed none")),
        "a strict report must state that it assumed nothing: {:?}",
        report.limits
    );
    assert!(
        report
            .limits
            .iter()
            .all(|line| !line.starts_with(ASSUMPTION_LINE_PREFIX)),
        "a strict run raised only Tolerated defects, so it must hold no assumption line: {:?}",
        report.limits
    );
}

#[test]
fn a_salvage_report_over_the_patched_bytes_holds_the_mode_line_and_one_assumption_line() {
    let table = OpcodeTable::builtin();
    let patched = with_external_count(FAST_FLAMES, &table, 1);
    let report = built_project_report(&patched, &table, Mode::Salvage);

    assert!(
        report
            .limits
            .iter()
            .any(|line| line.contains("salvage run") && line.contains("continued past")),
        "a salvage report must state that it continued past a defect: {:?}",
        report.limits
    );

    let assumption_lines: Vec<&String> = report
        .limits
        .iter()
        .filter(|line| line.starts_with(ASSUMPTION_LINE_PREFIX))
        .collect();
    assert_eq!(
        assumption_lines.len(),
        1,
        "exactly one Recoverable defect was patched in: {:?}",
        report.limits
    );
    assert!(
        assumption_lines[0].contains("0x1da4"),
        "the one assumption line must name the byte offset 0x1da4: {}",
        assumption_lines[0]
    );
}

#[test]
fn the_defect_array_holds_more_entries_than_the_assumption_line_count() {
    let table = OpcodeTable::builtin();
    let patched = with_external_count(FAST_FLAMES, &table, 1);
    let report = built_project_report(&patched, &table, Mode::Salvage);

    let assumption_count = report
        .limits
        .iter()
        .filter(|line| line.starts_with(ASSUMPTION_LINE_PREFIX))
        .count();
    assert!(
        report.defects.len() > assumption_count,
        "the Tolerated defects must be reported and not counted as assumptions: \
         {} defects, {assumption_count} assumption line(s)",
        report.defects.len()
    );
}

#[test]
fn two_salvage_runs_over_the_same_bytes_give_byte_identical_json() {
    let table = OpcodeTable::builtin();
    let patched = with_external_count(FAST_FLAMES, &table, 1);
    let first = built_project_report(&patched, &table, Mode::Salvage).to_json();
    let second = built_project_report(&patched, &table, Mode::Salvage).to_json();
    assert_eq!(first, second);
}
