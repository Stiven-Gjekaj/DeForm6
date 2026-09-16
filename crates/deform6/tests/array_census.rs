#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! For every array the fidelity walk counts, the count each corpus program
//! declares equals the number of instances the reader returns, and the file
//! holds that count at the offset the census names.
//!
//! # Why this file exists beside `byte_fidelity.rs`
//!
//! Most of this reader's clamps bound a loop and change no byte, so the byte
//! diff cannot see them. They make the reader return fewer instances than the
//! file declares, and this file is where that would show.
//!
//! Measured on 2026-09-16: no array in any of the 44 programs comes up short.
//! That is a result, and `the_rows_that_are_not_whole_are_exactly_the_rows_this_file_names`
//! pins it, so a clamp that appears fails this file and so does a clamp that
//! disappears.
//!
//! # The declared count is read back from the file
//!
//! A census row names the offset of its count field. That offset comes from
//! the walk's own statement of the layout, not from the reader.
//! `every_count_the_census_carries_is_the_count_the_file_holds_at_the_offset_it_names`
//! reads the bytes at that offset and requires them to hold the declared count,
//! which is what makes the offset a checked fact rather than a label.
//!
//! # This file keeps its own corpus walk
//!
//! Copied rather than shared, for the reason `tests/corpus_sweep.rs` gives:
//! two corpus tests must be able to fail independently.

use deform6::fidelity::census::{Array, Count, Outcome, Owner};
use deform6::fidelity::walk::walk;
use std::path::{Path, PathBuf};

/// The number of executables the corpus vendors.
const EXPECTED_EXECUTABLE_COUNT: usize = 44;

/// The rows that are not whole, as `(program, array, owner, declared,
/// returned)`. Measured on 2026-09-16: none.
const NOT_WHOLE: &[(&str, Array, Owner, u32, u32)] = &[];

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

/// Gives a program's path relative to `corpus/`, with `/` separators.
fn key(path: &Path) -> String {
    path.strip_prefix(corpus_root())
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// One corpus program: its key, its bytes, and every row the walk counted.
struct Census {
    key: String,
    data: Vec<u8>,
    counts: Vec<Count>,
}

fn census() -> Vec<Census> {
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
            let counts = walk(&data)
                .unwrap_or_else(|err| panic!("walking {}: {err}", path.display()))
                .counts;
            Census {
                key: key(&path),
                data,
                counts,
            }
        })
        .collect()
}

/// Reads the count field a row names, straight out of the file.
fn held(data: &[u8], row: &Count) -> Option<u32> {
    let at = usize::try_from(row.declared_at.get()).ok()?;
    match row.array.width() {
        2 => data
            .get(at..at + 2)
            .map(|b| u32::from(u16::from_le_bytes([b[0], b[1]]))),
        4 => data
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        other => panic!("no count field is {other} bytes wide"),
    }
}

/// Renders one row with what a person needs to find it in a hex editor.
fn describe(program: &Census, row: &Count) -> String {
    let at = usize::try_from(row.declared_at.get()).unwrap();
    let width = usize::try_from(row.array.width()).unwrap();
    let bytes = program.data.get(at..at + width).unwrap_or(&[]);
    format!(
        "{}: {}.{} of {:?} at file offset {:#x} holds {:02X?}: the file declares {} and the \
         reader returned {} ({:?})",
        program.key,
        row.array.structure(),
        row.array.field(),
        row.owner,
        row.declared_at.get(),
        bytes,
        row.declared,
        row.recovered,
        row.outcome
    )
}

/// Totals `(declared, returned)` over every row for one array.
fn totals(programs: &[Census], array: Array) -> (u64, u64) {
    programs
        .iter()
        .flat_map(|program| program.counts.iter())
        .filter(|row| row.array == array)
        .fold((0, 0), |(declared, returned), row| {
            (
                declared + u64::from(row.declared),
                returned + u64::from(row.recovered),
            )
        })
}

#[test]
fn the_census_counts_one_hundred_and_five_objects_declared_and_one_hundred_and_five_returned() {
    assert_eq!(totals(&census(), Array::Objects), (105, 105));
}

#[test]
fn the_census_counts_fifty_three_gui_table_entries_declared_and_fifty_three_returned() {
    assert_eq!(totals(&census(), Array::GuiTable), (53, 53));
}

#[test]
fn every_corpus_program_carries_exactly_one_gui_table_count_row() {
    let mut failed = Vec::new();
    for program in census() {
        let rows = program
            .counts
            .iter()
            .filter(|row| row.array == Array::GuiTable)
            .count();
        if rows != 1 {
            failed.push(format!(
                "{}: {rows} GUI table count rows, wanted 1",
                program.key
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn every_corpus_program_carries_exactly_one_object_count_row() {
    let mut failed = Vec::new();
    for program in census() {
        let rows = program
            .counts
            .iter()
            .filter(|row| row.array == Array::Objects)
            .count();
        if rows != 1 {
            failed.push(format!(
                "{}: {rows} object count rows, wanted 1",
                program.key
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn every_count_the_census_carries_is_the_count_the_file_holds_at_the_offset_it_names() {
    let mut failed = Vec::new();
    let mut checked = 0_u32;
    for program in census() {
        for row in &program.counts {
            checked += 1;
            if held(&program.data, row) != Some(row.declared) {
                failed.push(describe(&program, row));
            }
        }
    }
    assert!(checked > 0, "the census carried no rows to check");
    assert!(
        failed.is_empty(),
        "{} row(s) name an offset that does not hold their count:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn no_census_row_in_any_corpus_program_is_unexplained() {
    let mut failed = Vec::new();
    for program in census() {
        for row in &program.counts {
            if row.outcome == Outcome::Unexplained {
                failed.push(describe(&program, row));
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn the_rows_that_are_not_whole_are_exactly_the_rows_this_file_names() {
    let mut found = Vec::new();
    for program in census() {
        for row in &program.counts {
            if row.outcome != Outcome::Whole {
                found.push((
                    program.key.clone(),
                    row.array,
                    row.owner,
                    row.declared,
                    row.recovered,
                ));
            }
        }
    }
    let pinned: Vec<(String, Array, Owner, u32, u32)> = NOT_WHOLE
        .iter()
        .map(|(program, array, owner, declared, returned)| {
            ((*program).to_owned(), *array, *owner, *declared, *returned)
        })
        .collect();
    assert_eq!(
        found, pinned,
        "the rows that are not whole moved. A new row is a clamp, an unmapped array or a \
         refusal the corpus did not have before. A missing row is one that went away."
    );
}

#[test]
fn the_census_counts_seven_hundred_and_six_control_information_entries_declared_and_returned() {
    // ControlInfo entries, not controls. ControlInfo is the event binding
    // table, and the README's 686 controls counts nodes in the control tree:
    // a different quantity, and not a disagreement.
    assert_eq!(totals(&census(), Array::Controls), (706, 706));
}

#[test]
fn the_census_carries_ninety_seven_control_count_rows_one_for_each_object_with_a_block() {
    let rows: usize = census()
        .iter()
        .map(|program| {
            program
                .counts
                .iter()
                .filter(|row| row.array == Array::Controls)
                .count()
        })
        .sum();
    assert_eq!(rows, 97);
}

/// Walks SK-Gradient with one four byte value patched in memory, and gives
/// back its control rows. The patch lands relative to the first control row
/// that declares at least one entry, at the offset that row itself names.
fn patched_control_rows(delta: u32, value: u32) -> Vec<Count> {
    let path = corpus_root().join("public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe");
    let original = std::fs::read(&path).unwrap();
    let row = walk(&original)
        .unwrap()
        .counts
        .into_iter()
        .find(|row| row.array == Array::Controls && row.declared > 0)
        .expect("the fixture declares at least one control entry");

    let at = usize::try_from(row.declared_at.get() + delta).unwrap();
    let mut patched = original.clone();
    assert_ne!(
        patched[at..at + 4],
        value.to_le_bytes(),
        "the patch must change the file, or this test proves nothing"
    );
    patched[at..at + 4].copy_from_slice(&value.to_le_bytes());

    walk(&patched)
        .expect("a patched control array must not stop the walk")
        .counts
        .into_iter()
        .filter(|row| row.array == Array::Controls)
        .collect()
}

#[test]
fn a_control_array_whose_address_maps_nowhere_is_counted_as_unmapped() {
    // lpControls sits four bytes after dwControlCount.
    let rows = patched_control_rows(4, 0x00F0_0000);
    let unmapped: Vec<&Count> = rows
        .iter()
        .filter(|row| row.outcome == Outcome::Unmapped)
        .collect();
    assert_eq!(unmapped.len(), 1, "{rows:?}");
    assert!(unmapped[0].declared > 0);
    assert_eq!(unmapped[0].recovered, 0);
}

#[test]
fn a_control_count_larger_than_the_file_can_hold_is_counted_as_clamped() {
    let rows = patched_control_rows(0, 0xFFFF_FFFF);
    let clamped: Vec<&Count> = rows
        .iter()
        .filter(|row| matches!(row.outcome, Outcome::Clamped { .. }))
        .collect();
    assert_eq!(clamped.len(), 1, "{rows:?}");
    let row = clamped[0];
    assert_eq!(row.declared, 0xFFFF_FFFF);
    assert_eq!(row.outcome, Outcome::Clamped { max: row.recovered });
    assert!(row.recovered < row.declared);
}
