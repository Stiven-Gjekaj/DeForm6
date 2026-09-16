#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! How many bytes of each structure the fidelity walk grades this reader
//! reproduces, and whether the bytes it does reproduce equal the ones the
//! compiler wrote, measured over all 44 corpus executables.
//!
//! # What a clean result here does and does not mean
//!
//! Thirteen of the fourteen header fields, all five `Object` fields, every
//! field of the other six structures, and the `imm32` of an event stub are
//! carried verbatim: read at an
//! offset, written back at the same offset, with no arithmetic between. A
//! verbatim field cannot disagree with itself, so a clean map is the expected
//! result and **is not** a proof that the reader is correct.
//!
//! What it does prove is worth having. The coverage counts are real, and they
//! are pinned here, so a field dropped from the reader fails this file. And
//! the offsets in `fidelity/` were written from `docs/STRUCTURES.md`
//! independently of the offsets in `vb/`, so a clean run is two separate
//! statements of the layout agreeing. The unit tests beside each emitter hold
//! the mechanism to reporting a difference when one exists; this file
//! measures the corpus.
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
//! # The event stub is the second structure that can differ
//!
//! The reader keeps the handler address of each stub, not its jump. The
//! emitter works the jump back out of that address, and it writes the opcode
//! bytes of the native stub, which the reader assumes and never reads. A wrong
//! handler arithmetic, or a stub of another shape, therefore shows here.
//!
//! Measured on 2026-09-16: all 390 stubs reproduce all 13 of their bytes.
//!
//! # The disputed front of `ControlInfo` is checked another way
//!
//! `docs/STRUCTURES.md` section 8.6 disputes the first two `ControlInfo`
//! fields. The reader and the emitter put both at the same offsets, so the
//! diff cannot settle the dispute. The event slots can.
//! `the_event_slots_fit_the_word_at_two_and_refuse_the_word_at_four_in_seven_hundred_and_six_control_info_records`
//! reads them by hand.
//!
//! # This file keeps its own corpus walk
//!
//! Copied from `tests/corpus_sweep.rs` rather than shared, for the reason
//! that file and `tests/differential.rs` both give: two corpus tests must be
//! able to fail independently.

use deform6::fidelity::census::{Array, Owner};
use deform6::fidelity::controlinfo;
use deform6::fidelity::eventstub;
use deform6::fidelity::gui;
use deform6::fidelity::guiobjectinfo;
use deform6::fidelity::header;
use deform6::fidelity::ledger::{Ledger, Verdict};
use deform6::fidelity::object;
use deform6::fidelity::objectinfo;
use deform6::fidelity::optionalobjectinfo;
use deform6::fidelity::privateobj;
use deform6::fidelity::project;
use deform6::fidelity::walk::Reason;
use deform6::fidelity::walk::walk;
use deform6::read::pe::PeImage;
use deform6::read::region::{Off, Va};
use deform6::vb::classify::ObjectKind;
use deform6::vb::opcodes::OpcodeTable;
use std::collections::BTreeSet;
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

/// `STRUCTURES.md` section 8.1: `uuidObjectGUI` sits at GUI table entry
/// `+ 0x04`.
const ENTRY_OBJECT_GUID: usize = 0x04;

/// `STRUCTURES.md` section 8.1: `aFormPointer` sits at GUI table entry
/// `+ 0x48`.
const ENTRY_FORM_POINTER: usize = 0x48;

/// `STRUCTURES.md` section 8.2: `guidObjectGUI` sits at `GUIObjectInfo +
/// 0x05`, one byte past the aligned place, because of the single byte at
/// `0x04`.
const INFO_OBJECT_GUID: usize = 0x05;

#[test]
fn the_object_gui_guid_sits_at_five_and_never_at_four_in_fifty_three_gui_object_info_records() {
    // STRUCTURES.md section 8.2 rates the GUIObjectInfo layout [L] and warns
    // that the single byte at 0x04 moves every later field to an odd offset.
    // The GUI table entry holds the same GUID, so the two records test the
    // layout against each other. Everything below is read by hand: the walk
    // gives only where each GUI table entry starts.
    //
    // Measured on 2026-09-16: the GUID matches at 0x05 in all 53 forms and at
    // the aligned 0x04 in none. The head dword is 0x0031CCFF in all 53, and
    // the byte at 0x04 takes more than one value.
    let mut forms = 0_usize;
    let mut at_five = 0_usize;
    let mut at_four = 0_usize;
    let mut constant_head = 0_usize;
    let mut bytes_at_four = BTreeSet::new();
    for (path, ledgers) in graded() {
        let data = std::fs::read(&path).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        for ledger in ledgers.iter().filter(|l| l.structure == "GuiTableEntry") {
            forms += 1;
            let entry = usize::try_from(ledger.base.get()).unwrap();
            let guid = &data[entry + ENTRY_OBJECT_GUID..entry + ENTRY_OBJECT_GUID + 16];
            let pointer = u32::from_le_bytes(
                data[entry + ENTRY_FORM_POINTER..entry + ENTRY_FORM_POINTER + 4]
                    .try_into()
                    .unwrap(),
            );
            let info = pe
                .region_at_va(Va::new(pointer))
                .and_then(|region| region.file_offset(Off::new(0)))
                .unwrap_or_else(|| {
                    panic!("{}: aFormPointer {pointer:#x} maps nowhere", path.display())
                });
            let info = usize::try_from(info.get()).unwrap();

            if data[info + INFO_OBJECT_GUID..info + INFO_OBJECT_GUID + 16] == *guid {
                at_five += 1;
            }
            if data[info + 0x04..info + 0x04 + 16] == *guid {
                at_four += 1;
            }
            if data[info..info + 4] == 0x0031_CCFF_u32.to_le_bytes() {
                constant_head += 1;
            }
            bytes_at_four.insert(data[info + 0x04]);
        }
    }
    assert_eq!(forms, 53);
    assert_eq!(at_five, 53, "the GUID must sit at GUIObjectInfo + 0x05");
    assert_eq!(at_four, 0, "the GUID must never sit at the aligned + 0x04");
    assert_eq!(constant_head, 53);
    assert!(
        bytes_at_four.len() > 1,
        "the byte at 0x04 holds one value in every form: {bytes_at_four:?}"
    );
}

#[test]
fn the_corpus_grades_fifty_three_gui_object_info_records() {
    assert_eq!(graded_count("GuiObjectInfo"), 53);
}

#[test]
fn the_gui_object_info_reader_models_four_of_the_ninety_three_bytes_in_every_graded_record() {
    let failed = coverage_failures(
        "GuiObjectInfo",
        93,
        guiobjectinfo::MODELLED_BYTES,
        guiobjectinfo::UNMODELLED,
    );
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn no_gui_object_info_byte_this_reader_models_differs_from_the_file_in_any_corpus_program() {
    let failed = difference_failures("GuiObjectInfo");
    assert!(
        failed.is_empty(),
        "{} GuiObjectInfo byte run(s) differ from the file:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn the_walk_grades_one_gui_object_info_at_the_address_each_gui_table_entry_names() {
    // The entry's aFormPointer is read by hand and resolved through the PE
    // image, so the place is not the walk's own statement.
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        let data = std::fs::read(&path).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        let named: Vec<u32> = ledgers
            .iter()
            .filter(|l| l.structure == "GuiTableEntry")
            .map(|entry| {
                let at = usize::try_from(entry.base.get()).unwrap() + ENTRY_FORM_POINTER;
                let pointer = u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
                pe.region_at_va(Va::new(pointer))
                    .and_then(|region| region.file_offset(Off::new(0)))
                    .unwrap()
                    .get()
            })
            .collect();
        let graded: Vec<u32> = ledgers
            .iter()
            .filter(|l| l.structure == "GuiObjectInfo")
            .map(|l| l.base.get())
            .collect();
        if named != graded {
            failed.push(format!(
                "{}: the GUI table entries name {named:x?}, and the walk graded GUIObjectInfo at {graded:x?}",
                path.display()
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn one_form_whose_gui_object_info_is_unmapped_loses_its_own_record_and_nothing_else() {
    // Patched in memory only. AGENTS.md bars committing a patched program.
    let path = corpus_root().join("public-domain/HexScroll/Hex Scroll.exe");
    let original = std::fs::read(&path).unwrap();
    let before = walk(&original).unwrap();

    let entries: Vec<&Ledger> = before
        .ledgers
        .iter()
        .filter(|l| l.structure == "GuiTableEntry")
        .collect();
    assert_eq!(entries.len(), 2, "the fixture holds two forms");
    let at = usize::try_from(entries[1].base.get()).unwrap() + ENTRY_FORM_POINTER;

    let unmapped = 0x00F0_0000_u32.to_le_bytes();
    let mut patched = original.clone();
    assert_ne!(
        patched[at..at + 4],
        unmapped,
        "the patch must change the file, or this test proves nothing"
    );
    patched[at..at + 4].copy_from_slice(&unmapped);

    let after = walk(&patched).expect("one bad form must not stop the walk");

    let added: Vec<_> = after
        .ungraded
        .iter()
        .filter(|row| !before.ungraded.contains(row))
        .collect();
    assert_eq!(added.len(), 1, "{added:?}");
    assert_eq!(added[0].structure, "GuiObjectInfo");
    assert_eq!(added[0].owner, Owner::Form { form: 1 });
    assert!(matches!(added[0].reason, Reason::Refused(_)));

    for ledger in &after.ledgers {
        assert!(
            before.ledgers.contains(ledger),
            "the patch changed a ledger it should not have touched: {} at {:#x}",
            ledger.structure,
            ledger.base.get()
        );
    }
    assert_eq!(after.ledgers.len() + 1, before.ledgers.len());
    assert_eq!(after.counts, before.counts);
}

#[test]
fn the_corpus_grades_one_hundred_and_five_object_info_records() {
    assert_eq!(graded_count("ObjectInfo"), 105);
}

#[test]
fn the_object_info_reader_models_six_of_the_fifty_six_bytes_in_every_graded_record() {
    let failed = coverage_failures(
        "ObjectInfo",
        56,
        objectinfo::MODELLED_BYTES,
        objectinfo::UNMODELLED,
    );
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn no_object_info_byte_this_reader_models_differs_from_the_file_in_any_corpus_program() {
    let failed = difference_failures("ObjectInfo");
    assert!(
        failed.is_empty(),
        "{} ObjectInfo byte run(s) differ from the file:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn every_object_inspect_reports_has_one_object_info_ledger() {
    let table = OpcodeTable::builtin();
    let mut failed = Vec::new();
    for path in executables() {
        let data = std::fs::read(&path).unwrap();
        let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict)
            .unwrap_or_else(|err| panic!("inspecting {}: {err}", path.display()));
        let found = walk(&data).unwrap();
        let graded = found
            .ledgers
            .iter()
            .filter(|l| l.structure == "ObjectInfo")
            .count();
        let skipped = found
            .ungraded
            .iter()
            .filter(|u| u.structure == "ObjectInfo")
            .count();
        if graded + skipped != report.objects.len() || skipped != 0 {
            failed.push(format!(
                "{}: inspect reports {} objects; the walk graded {graded} ObjectInfo records and \
                 left {skipped} ungraded",
                path.display(),
                report.objects.len()
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn one_object_whose_object_info_is_unmapped_loses_its_own_records_and_nothing_else() {
    // Patched in memory only. AGENTS.md bars committing a patched program.
    let path = corpus_root().join("vb6-code/Grayscale-effect/Grayscale.exe");
    let original = std::fs::read(&path).unwrap();
    let before = walk(&original).unwrap();

    // lpObjectInfo is the first field of an Object record, and the walk's own
    // Object ledger names where that record starts.
    let objects: Vec<&Ledger> = before
        .ledgers
        .iter()
        .filter(|l| l.structure == "Object")
        .collect();
    assert_eq!(objects.len(), 3, "the fixture holds three objects");
    let at = usize::try_from(objects[1].base.get()).unwrap();

    let unmapped = 0x00F0_0000_u32.to_le_bytes();
    let mut patched = original.clone();
    assert_ne!(
        patched[at..at + 4],
        unmapped,
        "the patch must change the file, or this test proves nothing"
    );
    patched[at..at + 4].copy_from_slice(&unmapped);

    let after = walk(&patched).expect("one bad object must not stop the walk");

    // Only the rows the patch added are judged. A module is Absent in every
    // walk, patched or not, so it is not evidence about the patch.
    let added: Vec<_> = after
        .ungraded
        .iter()
        .filter(|row| !before.ungraded.contains(row))
        .collect();
    assert!(
        !added.is_empty(),
        "the patched object must be reported as ungraded"
    );
    for row in &added {
        assert_eq!(
            row.owner,
            Owner::Object { object: 1 },
            "an ungraded row belongs to an object that was not patched: {row:?}"
        );
    }
    assert!(
        after
            .ungraded
            .iter()
            .any(|row| row.structure == "ObjectInfo"),
        "the patched object's ObjectInfo must be the row that was not graded"
    );

    // Every ledger that survives is byte for byte one that existed before.
    for ledger in &after.ledgers {
        assert!(
            before.ledgers.contains(ledger),
            "the patch changed a ledger it should not have touched: {} at {:#x}",
            ledger.structure,
            ledger.base.get()
        );
    }
    let info = |walk: &deform6::fidelity::walk::Walk| {
        walk.ledgers
            .iter()
            .filter(|l| l.structure == "ObjectInfo")
            .count()
    };
    assert_eq!(info(&before), 3);
    assert_eq!(info(&after), 2);
}

#[test]
fn the_corpus_grades_ninety_seven_private_obj_records() {
    assert_eq!(graded_count("PrivateObj"), 97);
}

#[test]
fn the_private_obj_reader_models_sixteen_of_the_sixty_four_bytes_in_every_graded_record() {
    let failed = coverage_failures(
        "PrivateObj",
        64,
        privateobj::MODELLED_BYTES,
        privateobj::UNMODELLED,
    );
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn no_private_obj_byte_this_reader_models_differs_from_the_file_in_any_corpus_program() {
    let failed = difference_failures("PrivateObj");
    assert!(
        failed.is_empty(),
        "{} PrivateObj byte run(s) differ from the file:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn the_objects_with_no_private_obj_are_exactly_the_modules_inspect_reports() {
    // STRUCTURES.md section 5.5 settles the module test on fObjectType bit
    // 0x2. The private object's own sentinel is an independent route to the
    // same answer, so the two are compared as sets, in both directions.
    let table = OpcodeTable::builtin();
    let mut failed = Vec::new();
    let mut modules_seen = 0_usize;
    for path in executables() {
        let data = std::fs::read(&path).unwrap();
        let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict)
            .unwrap_or_else(|err| panic!("inspecting {}: {err}", path.display()));
        let found = walk(&data).unwrap();

        let absent = absent_objects(&found, "PrivateObj");
        let modules: BTreeSet<u32> = report
            .objects
            .iter()
            .enumerate()
            .filter(|(_index, object)| object.kind == ObjectKind::Module)
            .map(|(index, _object)| u32::try_from(index).unwrap())
            .collect();
        modules_seen += modules.len();

        if absent != modules {
            failed.push(format!(
                "{}: objects with no PrivateObj {absent:?}, modules {modules:?}",
                path.display()
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert_eq!(modules_seen, 8, "the corpus holds eight standard modules");
}

/// Gives the indexes of the objects a walk found to have no record of one
/// structure.
fn absent_objects(found: &deform6::fidelity::walk::Walk, structure: &str) -> BTreeSet<u32> {
    found
        .ungraded
        .iter()
        .filter(|row| row.structure == structure && row.reason == Reason::Absent)
        .filter_map(|row| match row.owner {
            Owner::Object { object } => Some(object),
            Owner::Program | Owner::Form { .. } | Owner::Control { .. } | Owner::Slot { .. } => {
                None
            }
        })
        .collect()
}

#[test]
fn the_corpus_grades_ninety_seven_optional_object_info_records() {
    assert_eq!(graded_count("OptionalObjectInfo"), 97);
}

#[test]
fn the_optional_object_info_reader_models_eight_of_the_sixty_four_bytes_in_every_graded_record() {
    let failed = coverage_failures(
        "OptionalObjectInfo",
        64,
        optionalobjectinfo::MODELLED_BYTES,
        optionalobjectinfo::UNMODELLED,
    );
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn no_optional_object_info_byte_this_reader_models_differs_from_the_file_in_any_corpus_program() {
    let failed = difference_failures("OptionalObjectInfo");
    assert!(
        failed.is_empty(),
        "{} OptionalObjectInfo byte run(s) differ from the file:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn the_objects_with_no_optional_object_info_are_exactly_the_objects_with_no_private_obj() {
    // Two structures, two presence rules: fObjectType bit 0x2 for the block,
    // the lpPrivateObject sentinel for the private object. Both say which
    // objects are standard modules, and they must say the same thing.
    let mut failed = Vec::new();
    for path in executables() {
        let data = std::fs::read(&path).unwrap();
        let found = walk(&data).unwrap();
        let no_block = absent_objects(&found, "OptionalObjectInfo");
        let no_private = absent_objects(&found, "PrivateObj");
        if no_block != no_private {
            failed.push(format!(
                "{}: no OptionalObjectInfo {no_block:?}, no PrivateObj {no_private:?}",
                path.display()
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn the_corpus_grades_seven_hundred_and_six_control_info_records() {
    assert_eq!(graded_count("ControlInfo"), 706);
}

#[test]
fn the_control_info_reader_models_sixteen_of_the_forty_bytes_in_every_graded_record() {
    let failed = coverage_failures(
        "ControlInfo",
        40,
        controlinfo::MODELLED_BYTES,
        controlinfo::UNMODELLED,
    );
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn no_control_info_byte_this_reader_models_differs_from_the_file_in_any_corpus_program() {
    // The first two fields are the disputed ones in STRUCTURES.md section
    // 8.6. A clean result here does not settle that dispute: the reader and
    // the emitter put both fields at the same offsets, so they cannot differ.
    // The next test settles it from the event slots.
    let failed = difference_failures("ControlInfo");
    assert!(
        failed.is_empty(),
        "{} ControlInfo byte run(s) differ from the file:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// `STRUCTURES.md` section 8.6: the event table of a control whose
/// `fControlType` is `0x40` has a header of six dwords.
const NATIVE_EVENT_HEADER_LEN: u32 = 0x18;

/// `STRUCTURES.md` section 8.6: the two control kinds the format names.
const KNOWN_CONTROL_KINDS: [u32; 2] = [0x40, 0x2E];

/// Reads slot `index` of the native event table at `table`, by hand.
fn event_slot(pe: &PeImage<'_>, table: Va, index: u32) -> Option<Va> {
    pe.region_at_va(table)?
        .va_le(Off::new(NATIVE_EVENT_HEADER_LEN + index * 4))
}

/// Tells whether `va` holds the native stub section 8.6 shows:
/// `81 6C 24 04 <imm32>`, then `E9 <rel32>`.
///
/// Read by hand, so this file does not take the reader's word for what a
/// stub is.
fn is_native_stub(pe: &PeImage<'_>, va: Va) -> bool {
    pe.region_at_va(va)
        .and_then(|stub| stub.take(Off::new(0), 13))
        .is_some_and(|bytes| bytes[..4] == [0x81, 0x6C, 0x24, 0x04] && bytes[8] == 0xE9)
}

/// Tells whether a slot holds what a slot may hold: a null for an event with
/// no handler, or a native stub.
fn is_null_or_stub(pe: &PeImage<'_>, slot: Option<Va>) -> bool {
    slot.is_some_and(|va| va.is_null() || is_native_stub(pe, va))
}

#[test]
fn the_event_slots_fit_the_word_at_two_and_refuse_the_word_at_four_in_seven_hundred_and_six_control_info_records()
 {
    // STRUCTURES.md section 8.6: one source reads a four byte fControlType
    // and puts wEventCount at 0x04. Three read a two byte fControlType and
    // put wEventCount at 0x02. The event slots tell the two apart.
    //
    // Measured on 2026-09-16: every record is a 0x40 control. Its first four
    // bytes never read as a known kind. Every slot below the word at 0x02 is
    // a null or a stub, and the slot at that index is not a stub. The word
    // at 0x04 is always larger, and the slots it adds always include one that
    // is neither.
    let mut records = 0_usize;
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        let data = std::fs::read(&path).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        for ledger in ledgers.iter().filter(|l| l.structure == "ControlInfo") {
            records += 1;
            let at = usize::try_from(ledger.base.get()).unwrap();
            let word =
                |offset: usize| u16::from_le_bytes([data[at + offset], data[at + offset + 1]]);
            let dword = |offset: usize| {
                u32::from_le_bytes(data[at + offset..at + offset + 4].try_into().unwrap())
            };
            let place = format!("{}: ControlInfo at file offset {at:#x}", path.display());
            let (kind, word_at_two, word_at_four) = (word(0x00), word(0x02), word(0x04));
            let table = Va::new(dword(0x18));

            if kind != 0x40 {
                failed.push(format!(
                    "{place}: kind {kind:#x}, and this test knows only the header of kind 0x40"
                ));
                continue;
            }
            if KNOWN_CONTROL_KINDS.contains(&dword(0x00)) {
                failed.push(format!(
                    "{place}: the four byte kind {:#x} is a known kind",
                    dword(0x00)
                ));
            }
            if !(0..u32::from(word_at_two)).all(|i| is_null_or_stub(&pe, event_slot(&pe, table, i)))
            {
                failed.push(format!(
                    "{place}: a slot below the word at 0x02 ({word_at_two}) is not a null or a stub"
                ));
            }
            if event_slot(&pe, table, u32::from(word_at_two))
                .is_some_and(|va| !va.is_null() && is_native_stub(&pe, va))
            {
                failed.push(format!(
                    "{place}: slot {word_at_two} is a stub, so the word at 0x02 stops short"
                ));
            }
            if word_at_four <= word_at_two
                || (u32::from(word_at_two)..u32::from(word_at_four))
                    .all(|i| is_null_or_stub(&pe, event_slot(&pe, table, i)))
            {
                failed.push(format!(
                    "{place}: the word at 0x04 ({word_at_four}) adds no slot that is neither a null nor a stub"
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert_eq!(records, 706);
}

#[test]
fn no_two_structures_the_walk_grades_share_a_byte_in_any_corpus_program() {
    // Measured on 2026-09-16 before this was asserted: 1694 records across
    // the corpus, and no two of them overlap. Two structures claiming the
    // same byte would mean one of them is placed wrongly.
    let mut failed = Vec::new();
    for (path, ledgers) in graded() {
        let mut spans: Vec<(u32, u32, &str)> = ledgers
            .iter()
            .map(|l| (l.base.get(), l.base.get() + l.len, l.structure))
            .collect();
        spans.sort_unstable();
        for pair in spans.windows(2) {
            let (first, second) = (pair[0], pair[1]);
            if second.0 < first.1 {
                failed.push(format!(
                    "{}: {} at [{:#x}, {:#x}) overlaps {} at [{:#x}, {:#x})",
                    path.display(),
                    first.2,
                    first.0,
                    first.1,
                    second.2,
                    second.0,
                    second.1
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn the_walk_grades_one_thousand_six_hundred_and_ninety_four_records_across_the_corpus() {
    let total: usize = graded().iter().map(|(_path, ledgers)| ledgers.len()).sum();
    assert_eq!(total, 1694);
}

#[test]
fn the_corpus_grades_three_hundred_and_ninety_event_stubs() {
    assert_eq!(graded_count("EventStub"), 390);
}

#[test]
fn the_event_stub_emitter_writes_all_thirteen_bytes_in_every_graded_stub() {
    let failed = coverage_failures(
        "EventStub",
        13,
        eventstub::MODELLED_BYTES,
        eventstub::UNMODELLED,
    );
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn no_event_stub_byte_differs_from_the_file_in_any_corpus_program() {
    // Five of these bytes are opcodes the reader assumes, and four are a
    // jump the emitter works out again. A clean result here says that every
    // stub has the native shape and that the reader's handler arithmetic
    // agrees with section 8.6.
    let failed = difference_failures("EventStub");
    assert!(
        failed.is_empty(),
        "{} EventStub byte run(s) differ from the file:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// One bound slot of a native event table, read by hand.
struct BoundSlot {
    /// The slot's index in its event table.
    index: u16,
    /// The file offset of the slot.
    at: usize,
    /// The stub address the slot holds.
    stub: u32,
}

/// Reads, by hand, the bound slots of the event table that the `ControlInfo`
/// at file offset `base` names. Only the `0x40` header is known here, as in
/// `the_event_slots_fit_the_word_at_two_and_refuse_the_word_at_four_in_seven_hundred_and_six_control_info_records`.
fn bound_slots(data: &[u8], pe: &PeImage<'_>, base: usize) -> Vec<BoundSlot> {
    let word = |at: usize| u16::from_le_bytes([data[at], data[at + 1]]);
    let dword = |at: usize| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
    assert_eq!(word(base), 0x40, "this helper reads the 0x40 header only");
    let table = pe
        .region_at_va(Va::new(dword(base + 0x18)))
        .and_then(|region| region.file_offset(Off::new(0)))
        .unwrap();
    let table = usize::try_from(table.get()).unwrap();
    (0..word(base + 0x02))
        .filter_map(|index| {
            let at =
                table + usize::try_from(NATIVE_EVENT_HEADER_LEN).unwrap() + usize::from(index) * 4;
            let stub = dword(at);
            (stub != 0).then_some(BoundSlot { index, at, stub })
        })
        .collect()
}

fn sk_gradient() -> Vec<u8> {
    std::fs::read(corpus_root().join("public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe"))
        .unwrap()
}

#[test]
fn a_stub_whose_first_opcode_byte_is_changed_differs_at_that_byte_and_nowhere_else() {
    // Patched in memory only. The reader decodes the stub all the same,
    // because it never reads that byte.
    let original = sk_gradient();
    let before = walk(&original).unwrap();
    let stub = before
        .ledgers
        .iter()
        .find(|l| l.structure == "EventStub")
        .unwrap()
        .clone();
    let at = usize::try_from(stub.base.get()).unwrap();
    let mut patched = original.clone();
    assert_eq!(
        patched[at], 0x81,
        "the stub must open with the native opcode"
    );
    patched[at] = 0x80;

    let after = walk(&patched).unwrap();
    assert_eq!(after.ledgers.len(), before.ledgers.len());
    let changed: Vec<&Ledger> = after
        .ledgers
        .iter()
        .filter(|l| !before.ledgers.contains(l))
        .collect();
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert_eq!(changed[0].base, stub.base);
    let differs: Vec<(u32, u32)> = changed[0]
        .runs_with(Verdict::Differs)
        .map(|run| (run.span.at.get(), run.span.len))
        .collect();
    assert_eq!(differs, vec![(stub.base.get(), 1)]);
}

#[test]
fn a_bound_slot_whose_stub_maps_nowhere_is_ungraded_and_owned_by_its_slot() {
    let original = sk_gradient();
    let before = walk(&original).unwrap();
    let pe = PeImage::parse(&original).unwrap();
    let (control_base, slot) = before
        .ledgers
        .iter()
        .filter(|l| l.structure == "ControlInfo")
        .find_map(|l| {
            let base = usize::try_from(l.base.get()).unwrap();
            bound_slots(&original, &pe, base)
                .into_iter()
                .next()
                .map(|slot| (l.base.get(), slot))
        })
        .unwrap();
    // The census names each control by the offset of its wEventCount.
    let row = before
        .counts
        .iter()
        .find(|c| c.array == Array::EventSlots && c.declared_at.get() == control_base + 2)
        .unwrap();
    let Owner::Control { object, control } = row.owner else {
        panic!("an event slot row must belong to a control: {row:?}");
    };

    let unmapped = 0x00F0_0000_u32.to_le_bytes();
    let mut patched = original.clone();
    assert_ne!(patched[slot.at..slot.at + 4], unmapped);
    patched[slot.at..slot.at + 4].copy_from_slice(&unmapped);

    let after = walk(&patched).expect("one bad stub must not stop the walk");
    let added: Vec<_> = after
        .ungraded
        .iter()
        .filter(|row| !before.ungraded.contains(row))
        .collect();
    assert_eq!(added.len(), 1, "{added:?}");
    assert_eq!(added[0].structure, "EventStub");
    assert_eq!(
        added[0].owner,
        Owner::Slot {
            object,
            control,
            slot: slot.index
        }
    );
    assert!(matches!(added[0].reason, Reason::Refused(_)));
    assert_eq!(after.ledgers.len() + 1, before.ledgers.len());
    assert_eq!(after.counts, before.counts);
}

#[test]
fn a_stub_that_two_slots_name_is_graded_once() {
    // No corpus control names one stub twice, so the first control with two
    // bound slots is patched in memory: its second slot takes the first
    // slot's stub.
    for path in executables() {
        let original = std::fs::read(&path).unwrap();
        let pe = PeImage::parse(&original).unwrap();
        let before = walk(&original).unwrap();
        let Some(slots) = before
            .ledgers
            .iter()
            .filter(|l| l.structure == "ControlInfo")
            .map(|l| bound_slots(&original, &pe, usize::try_from(l.base.get()).unwrap()))
            .find(|slots| slots.len() >= 2)
        else {
            continue;
        };

        let mut patched = original.clone();
        let first = slots[0].stub.to_le_bytes();
        assert_ne!(patched[slots[1].at..slots[1].at + 4], first);
        patched[slots[1].at..slots[1].at + 4].copy_from_slice(&first);

        let after = walk(&patched).unwrap();
        let stubs = |found: &deform6::fidelity::walk::Walk| -> Vec<u32> {
            found
                .ledgers
                .iter()
                .filter(|l| l.structure == "EventStub")
                .map(|l| l.base.get())
                .collect()
        };
        let (old, new) = (stubs(&before), stubs(&after));
        assert_eq!(new.len() + 1, old.len(), "{}", path.display());
        let distinct: BTreeSet<u32> = new.iter().copied().collect();
        assert_eq!(distinct.len(), new.len(), "a stub was graded twice");
        assert_eq!(after.ungraded, before.ungraded);
        return;
    }
    panic!("no corpus control has two bound slots");
}
