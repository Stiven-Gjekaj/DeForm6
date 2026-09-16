#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! How many bytes of the VB header and of each `Object` this reader
//! reproduces, and whether the bytes it does reproduce equal the ones the
//! compiler wrote, measured over all 44 corpus executables.
//!
//! # What a clean result here does and does not mean
//!
//! Thirteen of the fourteen header fields, and all five `Object` fields, are
//! carried verbatim: read at an offset, written back at the same offset, with
//! no arithmetic between. A verbatim field cannot disagree with itself, so a
//! clean map is the expected result and **is not** a proof that the reader is
//! correct.
//!
//! What it does prove is worth having. The coverage counts are real, and they
//! are pinned here, so a field dropped from the reader fails this file. And
//! the offsets in `fidelity/` were written from `docs/STRUCTURES.md`
//! independently of the offsets in `vb/`, so a clean run is two separate
//! statements of the layout agreeing. `fidelity::header::tests` and
//! `fidelity::object::tests` hold the mechanism to reporting a difference
//! when one exists; this file measures the corpus.
//!
//! # `ProcCount` is the one field that can differ, and it does not
//!
//! `vb::object::bound_proc_count` clamps `ProcCount` to the number of entries
//! the file can hold behind `lpProcNamesArray`. Where that clamp fires, the
//! model and the file disagree and this file reports it.
//!
//! Measured on 2026-09-15: across 105 objects in 44 programs, **the clamp
//! never fires**. Every `Object` byte this reader models equals the file's
//! own. That is the result, not an absence of one, and
//! `the_corpus_grades_one_hundred_and_five_objects` pins the denominator so
//! the claim cannot quietly become a claim about fewer objects.
//!
//! # This file keeps its own corpus walk
//!
//! Copied from `tests/corpus_sweep.rs` rather than shared, for the reason
//! that file and `tests/differential.rs` both give: two corpus tests must be
//! able to fail independently.

use deform6::fidelity::gui;
use deform6::fidelity::header;
use deform6::fidelity::ledger::{Ledger, Verdict};
use deform6::fidelity::object;
use deform6::fidelity::project;
use deform6::fidelity::walk::walk;
use deform6::vb::opcodes::OpcodeTable;
use std::path::{Path, PathBuf};

/// The number of executables the corpus vendors.
const EXPECTED_EXECUTABLE_COUNT: usize = 44;

/// The number of objects the 44 corpus programs declare between them.
///
/// The same number `vb::object::ObjectTable::walk` records in its own doc
/// comment from this session's measurement of the array.
const EXPECTED_OBJECT_COUNT: usize = 105;

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

/// Grades every corpus program and gives back each program with its ledgers.
fn graded() -> Vec<(PathBuf, Vec<Ledger>)> {
    let files = executables();
    let count = files.len();
    assert_eq!(
        count, EXPECTED_EXECUTABLE_COUNT,
        "found {count} corpus executables, wanted {EXPECTED_EXECUTABLE_COUNT}"
    );

    files
        .into_iter()
        .map(|path| {
            let data = std::fs::read(&path)
                .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
            let ledgers = walk(&data)
                .unwrap_or_else(|err| panic!("grading {}: {err}", path.display()))
                .ledgers;
            (path, ledgers)
        })
        .collect()
}

/// Renders one differing run with everything a person needs to open a hex
/// editor at the right place.
fn describe(path: &Path, ledger: &Ledger, at: u32, len: u32, data: &[u8]) -> String {
    let start = usize::try_from(at).unwrap();
    let end = start + usize::try_from(len).unwrap();
    let file_bytes = data.get(start..end).unwrap_or(&[]);
    format!(
        "{}: {} at file offset {:#x}, {} byte(s), the file holds {:02X?}",
        path.display(),
        ledger.structure,
        at,
        len,
        file_bytes
    )
}

#[test]
fn every_ledger_tiles_its_structure_in_all_forty_four_corpus_programs() {
    // The invariant the whole result rests on: every byte of every structure
    // is accounted for exactly once. A ledger that does not tile has lost or
    // double counted bytes, and its coverage figure would be arithmetic on
    // nothing.
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        for ledger in &ledgers {
            if !ledger.tiles() {
                failed.push(format!(
                    "{}: {} at {:#x} does not tile its {} bytes: {:?}",
                    path.display(),
                    ledger.structure,
                    ledger.base.get(),
                    ledger.len,
                    ledger.runs
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn every_byte_the_header_reader_models_matches_the_file_in_all_forty_four_corpus_programs() {
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        let data = std::fs::read(&path).unwrap();
        for ledger in ledgers.iter().filter(|l| l.structure == "VBHeader") {
            for run in ledger.runs_with(Verdict::Differs) {
                failed.push(describe(
                    &path,
                    ledger,
                    run.span.at.get(),
                    run.span.len,
                    &data,
                ));
            }
        }
    }
    assert!(
        failed.is_empty(),
        "{} header byte run(s) differ from the file:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn the_header_reader_models_fifty_of_the_hundred_and_four_header_bytes_in_all_forty_four_corpus_programs()
 {
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        for ledger in ledgers.iter().filter(|l| l.structure == "VBHeader") {
            let modelled = ledger.bytes_with(Verdict::Same) + ledger.bytes_with(Verdict::Differs);
            if modelled != header::MODELLED_BYTES || ledger.len != 104 {
                failed.push(format!(
                    "{}: models {} of {} header bytes, wanted {} of 104",
                    path.display(),
                    modelled,
                    ledger.len,
                    header::MODELLED_BYTES
                ));
            }

            let gaps: Vec<(u32, u32)> = ledger
                .runs_with(Verdict::Unmodelled)
                .map(|run| (run.span.at.get() - ledger.base.get(), run.span.len))
                .collect();
            if gaps.as_slice() != header::UNMODELLED {
                failed.push(format!(
                    "{}: the unmodelled header ranges are {:x?}, wanted {:x?}",
                    path.display(),
                    gaps,
                    header::UNMODELLED
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn the_header_ledger_lands_on_the_offset_that_inspect_reports() {
    // The second walk in `fidelity::walk` resolves the chain again rather
    // than carrying a file offset through every reader. This is what proves
    // the second walk arrives where the first one did. Without it, a clean
    // map could be a clean map of the wrong bytes.
    let table = OpcodeTable::builtin();
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        let data = std::fs::read(&path).unwrap();
        let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict)
            .unwrap_or_else(|err| panic!("inspecting {}: {err}", path.display()));
        let header_ledger = ledgers
            .iter()
            .find(|l| l.structure == "VBHeader")
            .expect("every program grades its header");
        if header_ledger.base != report.header_offset {
            failed.push(format!(
                "{}: the ledger is at {:#x} and inspect reports {:#x}",
                path.display(),
                header_ledger.base.get(),
                report.header_offset.get()
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn the_corpus_grades_one_hundred_and_five_objects() {
    // The denominator of every claim this file makes about objects. Pinned
    // so "no object byte differs" cannot quietly become a statement about
    // fewer objects than the corpus holds.
    let total: usize = graded()
        .iter()
        .map(|(_path, ledgers)| ledgers.iter().filter(|l| l.structure == "Object").count())
        .sum();
    assert_eq!(
        total, EXPECTED_OBJECT_COUNT,
        "graded {total} objects, wanted {EXPECTED_OBJECT_COUNT}"
    );
}

#[test]
fn the_object_reader_models_twenty_of_the_forty_eight_object_bytes_in_every_recovered_object() {
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        for (index, ledger) in ledgers
            .iter()
            .filter(|l| l.structure == "Object")
            .enumerate()
        {
            let modelled = ledger.bytes_with(Verdict::Same) + ledger.bytes_with(Verdict::Differs);
            if modelled != object::MODELLED_BYTES || ledger.len != 48 {
                failed.push(format!(
                    "{} object {}: models {} of {} bytes, wanted {} of 48",
                    path.display(),
                    index,
                    modelled,
                    ledger.len,
                    object::MODELLED_BYTES
                ));
            }

            let gaps: Vec<(u32, u32)> = ledger
                .runs_with(Verdict::Unmodelled)
                .map(|run| (run.span.at.get() - ledger.base.get(), run.span.len))
                .collect();
            if gaps.as_slice() != object::UNMODELLED {
                failed.push(format!(
                    "{} object {}: the unmodelled ranges are {:x?}, wanted {:x?}",
                    path.display(),
                    index,
                    gaps,
                    object::UNMODELLED
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn no_object_byte_this_reader_models_differs_from_the_file_in_any_corpus_program() {
    // Where `bound_proc_count` clamps, this fires. See the module doc
    // comment: measured on 2026-09-15 it does not fire anywhere.
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        let data = std::fs::read(&path).unwrap();
        for ledger in ledgers.iter().filter(|l| l.structure == "Object") {
            for run in ledger.runs_with(Verdict::Differs) {
                failed.push(describe(
                    &path,
                    ledger,
                    run.span.at.get(),
                    run.span.len,
                    &data,
                ));
            }
        }
    }
    assert!(
        failed.is_empty(),
        "{} object byte run(s) differ from the file. A run at structure offset 0x1C is \
         ProcCount, which vb::object::bound_proc_count clamps:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

// --- Segment 2: shared checks, one call per structure ------------------------

/// Gives every program's failure for one structure's coverage: the modelled
/// byte count, the record length, and the exact unmodelled ranges.
fn coverage_failures(structure: &str, len: u32, modelled: u32, gaps: &[(u32, u32)]) -> Vec<String> {
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        for (index, ledger) in ledgers
            .iter()
            .filter(|l| l.structure == structure)
            .enumerate()
        {
            let got = ledger.bytes_with(Verdict::Same) + ledger.bytes_with(Verdict::Differs);
            if got != modelled || ledger.len != len {
                failed.push(format!(
                    "{} {structure} {index}: models {got} of {} bytes, wanted {modelled} of {len}",
                    path.display(),
                    ledger.len
                ));
            }
            let measured: Vec<(u32, u32)> = ledger
                .runs_with(Verdict::Unmodelled)
                .map(|run| (run.span.at.get() - ledger.base.get(), run.span.len))
                .collect();
            if measured.as_slice() != gaps {
                failed.push(format!(
                    "{} {structure} {index}: the unmodelled ranges are {measured:x?}, wanted {gaps:x?}",
                    path.display()
                ));
            }
        }
    }
    failed
}

/// Gives every run of one structure whose bytes differ from the file.
fn difference_failures(structure: &str) -> Vec<String> {
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        let data = std::fs::read(&path).unwrap();
        for ledger in ledgers.iter().filter(|l| l.structure == structure) {
            for run in ledger.runs_with(Verdict::Differs) {
                failed.push(describe(
                    &path,
                    ledger,
                    run.span.at.get(),
                    run.span.len,
                    &data,
                ));
            }
        }
    }
    failed
}

/// Gives the number of records of one structure the walk graded, corpus wide.
fn graded_count(structure: &str) -> usize {
    graded()
        .iter()
        .map(|(_path, ledgers)| ledgers.iter().filter(|l| l.structure == structure).count())
        .sum()
}

#[test]
fn the_corpus_grades_forty_four_project_info_records() {
    assert_eq!(graded_count("ProjectInfo"), 44);
}

#[test]
fn the_project_info_reader_models_twenty_of_the_five_hundred_and_seventy_two_bytes_in_every_graded_record()
 {
    let failed = coverage_failures(
        "ProjectInfo",
        572,
        project::MODELLED_BYTES,
        project::UNMODELLED,
    );
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn no_project_info_byte_this_reader_models_differs_from_the_file_in_any_corpus_program() {
    let failed = difference_failures("ProjectInfo");
    assert!(
        failed.is_empty(),
        "{} ProjectInfo byte run(s) differ from the file:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn the_corpus_grades_fifty_three_gui_table_entry_records() {
    assert_eq!(graded_count("GuiTableEntry"), 53);
}

#[test]
fn the_gui_table_entry_reader_models_eight_of_the_eighty_bytes_in_every_graded_record() {
    let failed = coverage_failures("GuiTableEntry", 80, gui::MODELLED_BYTES, gui::UNMODELLED);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn no_gui_table_entry_byte_this_reader_models_differs_from_the_file_in_any_corpus_program() {
    let failed = difference_failures("GuiTableEntry");
    assert!(
        failed.is_empty(),
        "{} GUI table entry byte run(s) differ from the file:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn the_walk_grades_one_gui_table_entry_for_each_form_inspect_reports() {
    // Two independent routes to the same number: the walk's own ledgers, and
    // the forms inspect composes from the GUI table.
    let table = OpcodeTable::builtin();
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        let data = std::fs::read(&path).unwrap();
        let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict)
            .unwrap_or_else(|err| panic!("inspecting {}: {err}", path.display()));
        let graded = ledgers
            .iter()
            .filter(|l| l.structure == "GuiTableEntry")
            .count();
        if graded != report.forms.len() {
            failed.push(format!(
                "{}: the walk graded {graded} GUI table entries and inspect reports {} forms",
                path.display(),
                report.forms.len()
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
