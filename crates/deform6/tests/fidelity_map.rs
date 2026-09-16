#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The committed fidelity map, `tests/fidelity.toml`, and the gate that
//! holds the tree to it.
//!
//! `shared.rs` holds the measurement, the text and the parser, and
//! `crates/xtask` compiles the same file to write the map. This file tests
//! that shared code, and then compares the committed map with a new
//! measurement of the corpus.
//!
//! # What the gate proves
//!
//! Every value in the file equals the value the walk measures, so a rewrite
//! changes no value. The file is also the exact text the writer renders from
//! those values, so a rewrite changes no line either. Together the two prove
//! that `cargo run -p xtask -- update-fidelity` on a clean tree gives no diff,
//! and no test writes the file to prove it.
//!
//! A value that moved fails with a direction word. `REGRESSION` means the walk
//! measures a worse number than the map holds, `MOVED UP` means a better one,
//! and `CHANGED` means that neither way is better. Each failure gives the table
//! to paste, in every direction.

#[path = "fidelity_map/shared.rs"]
#[allow(
    dead_code,
    reason = "this module is embedded in two binaries, this test target and crates/xtask, and \
              each uses a different part of it"
)]
mod shared;

use std::collections::BTreeMap;
use std::sync::OnceLock;

use shared::{
    ARRAYS, Change, Counted, Direction, EXPECTED_PROGRAM_COUNT, FidelityMap, Graded, HEADER,
    Layout, ProgramMap, STRUCTURES, array_name, changes, executables, fidelity_toml_path, measure,
    parse, render, render_layout, render_program,
};

/// A map with every layout and two programs, built here and not read from a
/// file.
fn synthetic_map() -> FidelityMap {
    let mut layouts = BTreeMap::new();
    for (index, name) in STRUCTURES.iter().copied().enumerate() {
        let index = u32::try_from(index).unwrap();
        layouts.insert(
            name,
            Layout {
                length: 40 + index,
                modelled: 16 + index,
                unmodelled: vec![(0x04, 4), (0x0C, 20 - index.min(20))],
            },
        );
    }
    let mut first = ProgramMap::empty();
    first.structures.insert(
        "Object",
        Graded {
            records: 3,
            same: 60,
            differs: 0,
            absent: 0,
            refused: 0,
        },
    );
    first.arrays.insert(
        "EventSlots",
        Counted {
            rows: 23,
            declared: 325,
            returned: 325,
            not_whole: 0,
        },
    );
    let mut second = ProgramMap::empty();
    second.structures.insert(
        "PrivateObj",
        Graded {
            records: 1,
            same: 16,
            differs: 2,
            absent: 1,
            refused: 1,
        },
    );
    let mut programs = BTreeMap::new();
    programs.insert("public-domain/HexScroll/Hex Scroll.exe".to_owned(), first);
    programs.insert("vb6-code/Sepia-effect/Sepia.exe".to_owned(), second);
    FidelityMap { layouts, programs }
}

#[test]
fn a_map_parses_back_to_the_values_it_was_rendered_from() {
    let map = synthetic_map();
    let text = render(&map);
    assert_eq!(parse(&text).unwrap(), map, "the text was:\n{text}");
}

#[test]
fn the_measured_corpus_map_renders_and_parses_back_to_itself() {
    let map = measure().unwrap();
    assert_eq!(map.programs.len(), EXPECTED_PROGRAM_COUNT);
    let text = render(&map);
    assert_eq!(parse(&text).unwrap(), map);
}

#[test]
fn an_unmodelled_offset_renders_as_hex() {
    let layout = Layout {
        length: 104,
        modelled: 50,
        unmodelled: vec![(0x06, 38), (0x3C, 8), (0x238, 4)],
    };
    let text = render_layout("VBHeader", &layout);
    assert!(
        text.contains("unmodelled = [[0x06, 38], [0x3C, 8], [0x238, 4]]"),
        "{text}"
    );
}

#[test]
fn a_layout_with_no_unmodelled_range_renders_an_empty_array() {
    let layout = Layout {
        length: 13,
        modelled: 13,
        unmodelled: Vec::new(),
    };
    let text = render_layout("ControlInfo", &layout);
    assert!(text.ends_with("unmodelled = []\n"), "{text}");
}

#[test]
fn a_program_key_with_a_space_renders_quoted() {
    let text = render_program(
        "vb6-code/Brightness-effect/Part 1 - Pure VB6/vbBrightness.exe",
        &ProgramMap::empty(),
    );
    assert!(
        text.starts_with(
            "[program.\"vb6-code/Brightness-effect/Part 1 - Pure VB6/vbBrightness.exe\"]\n"
        ),
        "{text}"
    );
}

#[test]
fn a_program_table_names_every_structure_and_every_array_in_order() {
    let text = render_program("a/A.exe", &ProgramMap::empty());
    let names: Vec<&str> = text
        .lines()
        .skip(1)
        .map(|line| line.split(" = ").next().unwrap())
        .collect();
    let mut wanted: Vec<String> = STRUCTURES
        .iter()
        .map(|name| format!("structure.{name}"))
        .collect();
    wanted.extend(
        ARRAYS
            .iter()
            .map(|array| format!("array.{}", array_name(*array))),
    );
    assert_eq!(names, wanted);
}

#[test]
fn the_change_list_names_every_value_that_moved_and_nothing_else() {
    let held = synthetic_map();
    let mut measured = held.clone();

    let hex = "public-domain/HexScroll/Hex Scroll.exe";
    let program = measured.programs.get_mut(hex).unwrap();
    program.structures.get_mut("Object").unwrap().records = 2;
    program.structures.get_mut("Object").unwrap().differs = 1;
    program.arrays.get_mut("EventSlots").unwrap().declared = 326;
    measured.layouts.get_mut("ProjectInfo").unwrap().modelled += 4;
    measured.layouts.get_mut("VBHeader").unwrap().unmodelled = vec![(0x06, 38)];
    measured.programs.remove("vb6-code/Sepia-effect/Sepia.exe");
    measured.programs.insert(
        "vb6-code/Sepia-effect/Other.exe".to_owned(),
        ProgramMap::empty(),
    );

    let found = changes(&held, &measured);
    let lines: Vec<String> = found.iter().map(ToString::to_string).collect();
    assert_eq!(found.len(), 7, "{lines:#?}");
    assert!(found.contains(&Change::Value {
        key: hex.to_owned(),
        field: "structure.Object.records".to_owned(),
        held: 3,
        measured: 2,
        direction: Direction::Regression,
    }));
    assert!(found.contains(&Change::Value {
        key: hex.to_owned(),
        field: "structure.Object.differs".to_owned(),
        held: 0,
        measured: 1,
        direction: Direction::Regression,
    }));
    assert!(found.contains(&Change::Value {
        key: hex.to_owned(),
        field: "array.EventSlots.declared".to_owned(),
        held: 325,
        measured: 326,
        direction: Direction::Changed,
    }));
    assert!(found.contains(&Change::LayoutValue {
        structure: "ProjectInfo",
        field: "modelled",
        held: 17,
        measured: 21,
        direction: Direction::MovedUp,
    }));
    assert!(lines.contains(&"layout.VBHeader: CHANGED unmodelled: the map holds [[0x04, 4], [0x0C, 20]], the walk measures [[0x06, 38]]".to_owned()), "{lines:#?}");
    assert!(found.contains(&Change::Stale {
        key: "vb6-code/Sepia-effect/Sepia.exe".to_owned()
    }));
    assert!(found.contains(&Change::Missing {
        key: "vb6-code/Sepia-effect/Other.exe".to_owned()
    }));
    assert!(
        lines.contains(&format!(
            "{hex}: REGRESSION structure.Object.records: the map holds 3, the walk measures 2"
        )),
        "{lines:#?}"
    );

    assert!(changes(&held, &held).is_empty());
}

/// Renders the synthetic map, applies `edit` to the text, and gives the
/// parser's error.
fn parse_error_after(edit: impl Fn(&str) -> String) -> String {
    let text = render(&synthetic_map());
    let edited = edit(&text);
    assert_ne!(
        edited, text,
        "the edit must change the text, or this test proves nothing"
    );
    parse(&edited).expect_err("the edited text must not parse")
}

#[test]
fn a_program_table_that_lacks_a_structure_does_not_parse() {
    let error = parse_error_after(|text| {
        text.lines()
            .filter(|line| !line.starts_with("structure.ObjectInfo = "))
            .map(|line| format!("{line}\n"))
            .collect()
    });
    assert!(error.contains("has no ObjectInfo"), "{error}");
}

#[test]
fn an_unknown_entry_does_not_parse() {
    let error = parse_error_after(|text| {
        text.replacen(
            "structure.VBHeader = {",
            "structure.Extra = { records = 1 }\nstructure.VBHeader = {",
            1,
        )
    });
    assert!(error.contains("unknown entry \"Extra\""), "{error}");
}

#[test]
fn a_missing_layout_does_not_parse() {
    let error = parse_error_after(|text| {
        let start = text.find("[layout.ObjectInfo]").unwrap();
        let end = start + text[start..].find("\n\n").unwrap() + 2;
        format!("{}{}", &text[..start], &text[end..])
    });
    assert!(error.contains("layout has no ObjectInfo"), "{error}");
}

#[test]
fn a_negative_count_does_not_parse() {
    let error = parse_error_after(|text| text.replacen("records = 3", "records = -3", 1));
    assert!(error.contains("records = -3 is not a count"), "{error}");
}

#[test]
fn a_count_that_is_not_an_integer_does_not_parse() {
    let error = parse_error_after(|text| text.replacen("same = 60", "same = \"60\"", 1));
    assert!(error.contains("same is not an integer"), "{error}");
}

#[test]
fn the_corpus_walk_skips_a_directory_named_fetched() {
    let root = std::env::temp_dir().join(format!(
        "deform6-fidelity-map-walker-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    for dir in ["a", "fetched", "b/fetched", "c"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    for file in [
        "a/A.exe",
        "fetched/B.exe",
        "b/fetched/C.EXE",
        "b/D.EXE",
        "c/E.txt",
    ] {
        std::fs::write(root.join(file), b"MZ").unwrap();
    }

    let mut found: Vec<String> = executables(&root)
        .unwrap()
        .iter()
        .map(|path| shared::program_key(path, &root).unwrap())
        .collect();
    found.sort();
    std::fs::remove_dir_all(&root).unwrap();

    assert_eq!(found, ["a/A.exe", "b/D.EXE"]);
}

#[test]
fn the_committed_map_parses_and_names_forty_four_programs() {
    let path = fidelity_toml_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    let map = parse(&text).unwrap();
    assert_eq!(map.programs.len(), EXPECTED_PROGRAM_COUNT);
    assert_eq!(map.layouts.len(), STRUCTURES.len());
}

/// The records each structure has across the corpus.
const RECORDS: &[(&str, u32)] = &[
    ("VBHeader", 44),
    ("ProjectInfo", 44),
    ("DeclareTableEntry", 249),
    ("DeclareDescriptor", 220),
    ("GuiTableEntry", 53),
    ("GuiObjectInfo", 53),
    ("ObjectTable", 44),
    ("Object", 105),
    ("ObjectInfo", 105),
    ("PrivateObj", 97),
    ("OptionalObjectInfo", 97),
    ("ControlInfo", 706),
    ("EventStub", 390),
];

/// The sum of [`RECORDS`].
const RECORD_TOTAL: u32 = 2207;

/// The records the corpus does not have: the `PrivateObj` and the
/// `OptionalObjectInfo` of each of the 8 standard modules.
const ABSENT_TOTAL: u32 = 16;

/// Each array's rows, declared total and returned total across the corpus.
const ARRAY_TOTALS: &[(&str, u32, u32, u32)] = &[
    ("DeclareEntries", 44, 249, 249),
    ("GuiTable", 44, 53, 53),
    ("Objects", 44, 105, 105),
    ("Controls", 97, 706, 706),
    ("EventSlots", 706, 11862, 11862),
];

/// The sum of the rows in [`ARRAY_TOTALS`].
const ROW_TOTAL: u32 = 935;

/// The program the mutation tests below change in memory.
const HEX_SCROLL: &str = "public-domain/HexScroll/Hex Scroll.exe";

const HOW_TO_FIX: &str = "Run `cargo run -p xtask -- update-fidelity`";

/// The walk's measurement of the corpus, taken once for this test binary.
fn measured() -> &'static FidelityMap {
    static MEASURED: OnceLock<FidelityMap> = OnceLock::new();
    MEASURED.get_or_init(|| measure().unwrap())
}

/// The text of the committed map.
fn committed_text() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        let path = fidelity_toml_path();
        std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
    })
}

/// The values of the committed map.
fn committed() -> FidelityMap {
    parse(committed_text()).unwrap()
}

/// One message for each layout and each program in which `held` and
/// `measured` disagree, then the derived and the canonical failures of `held`.
fn gate_failures(held: &FidelityMap, held_text: &str, measured: &FidelityMap) -> Vec<String> {
    let found = changes(held, measured);
    let mut failures = Vec::new();

    for change in found.iter().filter(|change| change.key().is_none()) {
        let structure = match change {
            Change::LayoutValue { structure, .. }
            | Change::LayoutRanges { structure, .. }
            | Change::LayoutPresence { structure, .. } => *structure,
            Change::Value { .. } | Change::Missing { .. } | Change::Stale { .. } => continue,
        };
        failures.push(match measured.layouts.get(structure) {
            Some(layout) => format!(
                "{change}\n{HOW_TO_FIX}, or put this table in place of the old one:\n{}",
                render_layout(structure, layout)
            ),
            None => format!("{change}\n{HOW_TO_FIX}, or delete this table.\n"),
        });
    }

    let mut by_key: BTreeMap<&str, Vec<&Change>> = BTreeMap::new();
    for change in &found {
        if let Some(key) = change.key() {
            by_key.entry(key).or_default().push(change);
        }
    }
    for (key, program_changes) in by_key {
        let values: Vec<String> = program_changes
            .iter()
            .filter_map(|change| match change {
                Change::Value {
                    field,
                    held,
                    measured,
                    direction,
                    ..
                } => Some(format!(
                    "  {direction} {field}: the map holds {held}, the walk measures {measured}\n"
                )),
                _ => None,
            })
            .collect();
        let mut message = if values.is_empty() {
            program_changes
                .iter()
                .map(|change| format!("{change}\n"))
                .collect::<String>()
        } else {
            format!(
                "{key}: {} value(s) in tests/fidelity.toml disagree with the walk:\n{}",
                values.len(),
                values.concat()
            )
        };
        match measured.programs.get(key) {
            Some(program) => message.push_str(&format!(
                "{HOW_TO_FIX}, or put this table in place of the old one:\n{}",
                render_program(key, program)
            )),
            None => message.push_str(&format!("{HOW_TO_FIX}, or delete this table.\n")),
        }
        failures.push(message);
    }

    failures.extend(derived_failures(held));
    failures.extend(canonical_failures(held, held_text));
    failures
}

/// Every structure entry whose byte counts do not follow from its record
/// count and its layout. Each graded record reproduces `modelled` bytes, and
/// each of those bytes is either the same as the file or different.
fn derived_failures(held: &FidelityMap) -> Vec<String> {
    let mut failures = Vec::new();
    for (key, program) in &held.programs {
        for name in STRUCTURES {
            let Some(layout) = held.layouts.get(name) else {
                continue;
            };
            let graded = program.structures[name];
            let wanted = u64::from(graded.records) * u64::from(layout.modelled);
            let got = u64::from(graded.same) + u64::from(graded.differs);
            if wanted != got {
                failures.push(format!(
                    "{key}: structure.{name} holds {} records and {} + {} bytes, and \
                     layout.{name} gives {} bytes a record, so the bytes must total {wanted}",
                    graded.records, graded.same, graded.differs, layout.modelled
                ));
            }
        }
    }
    failures
}

/// The first line in which `text` differs from the text the writer renders
/// from `held`.
///
/// Compares lines, not bytes, so a checkout with CRLF line endings compares
/// equal.
fn canonical_failures(held: &FidelityMap, text: &str) -> Vec<String> {
    if !text.ends_with('\n') {
        return vec![format!(
            "tests/fidelity.toml does not end with a newline. {HOW_TO_FIX}."
        )];
    }
    let rendered = render(held);
    let mut file_lines = text.lines();
    let mut writer_lines = rendered.lines();
    let mut number = 0_usize;
    loop {
        number += 1;
        let (file, writer) = (file_lines.next(), writer_lines.next());
        if file == writer {
            if file.is_none() {
                return Vec::new();
            }
            continue;
        }
        let show = |line: Option<&str>| {
            line.map_or_else(|| "no line".to_owned(), |line| format!("{line:?}"))
        };
        return vec![format!(
            "tests/fidelity.toml is not the text that the writer renders from its own values. \
             Line {number}: the file holds {}, and the writer renders {}. {HOW_TO_FIX}.",
            show(file),
            show(writer)
        )];
    }
}

/// The first line in which `text` differs from [`HEADER`].
fn header_failures(text: &str) -> Vec<String> {
    let header_lines = HEADER.lines().count();
    if text.lines().count() < header_lines {
        return vec!["tests/fidelity.toml is shorter than its own header".to_owned()];
    }
    for (index, (file, header)) in text.lines().zip(HEADER.lines()).enumerate() {
        if file != header {
            return vec![format!(
                "tests/fidelity.toml line {}: the file holds {file:?}, and the header in \
                 shared.rs holds {header:?}",
                index + 1
            )];
        }
    }
    Vec::new()
}

#[test]
fn the_gate_passes_on_the_committed_file() {
    assert_eq!(measured().programs.len(), EXPECTED_PROGRAM_COUNT);
    let failures = gate_failures(&committed(), committed_text(), measured());
    assert!(
        failures.is_empty(),
        "the committed map must pass on the committed tree:\n{}",
        failures.join("\n")
    );
}

#[test]
fn the_header_constant_matches_the_committed_files_own_header() {
    let failures = header_failures(committed_text());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_committed_map_is_the_text_the_writer_renders_from_its_values() {
    let failures = canonical_failures(&committed(), committed_text());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_program_table_reproduces_its_layout_bytes_once_per_record() {
    let failures = derived_failures(&committed());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The sum of each structure's entries over every program in `map`.
fn structure_totals(map: &FidelityMap) -> BTreeMap<&'static str, Graded> {
    let mut totals: BTreeMap<&'static str, Graded> = BTreeMap::new();
    for program in map.programs.values() {
        for (name, graded) in &program.structures {
            let total = totals.entry(name).or_default();
            total.records += graded.records;
            total.same += graded.same;
            total.differs += graded.differs;
            total.absent += graded.absent;
            total.refused += graded.refused;
        }
    }
    totals
}

#[test]
fn the_committed_map_holds_two_thousand_two_hundred_and_seven_graded_records_and_no_differing_byte()
{
    let totals = structure_totals(&committed());
    let records: Vec<(&str, u32)> = STRUCTURES
        .iter()
        .map(|name| (*name, totals[name].records))
        .collect();
    assert_eq!(records, RECORDS);
    assert_eq!(
        totals.values().map(|total| total.records).sum::<u32>(),
        RECORD_TOTAL
    );
    assert_eq!(totals.values().map(|total| total.differs).sum::<u32>(), 0);
}

#[test]
fn the_committed_map_holds_sixteen_absent_records_and_no_refused_one() {
    let totals = structure_totals(&committed());
    assert_eq!(
        totals.values().map(|total| total.absent).sum::<u32>(),
        ABSENT_TOTAL
    );
    assert_eq!(totals["PrivateObj"].absent, 8);
    assert_eq!(totals["OptionalObjectInfo"].absent, 8);
    assert_eq!(totals.values().map(|total| total.refused).sum::<u32>(), 0);
}

#[test]
fn the_committed_map_counts_nine_hundred_and_thirty_five_census_rows_and_none_that_is_not_whole() {
    let map = committed();
    let mut found = Vec::new();
    let mut rows = 0;
    for array in ARRAYS {
        let name = array_name(*array);
        let mut total = Counted::default();
        for program in map.programs.values() {
            let counted = program.arrays[name];
            total.rows += counted.rows;
            total.declared += counted.declared;
            total.returned += counted.returned;
            total.not_whole += counted.not_whole;
        }
        assert_eq!(total.not_whole, 0, "{name} has an array that is not whole");
        rows += total.rows;
        found.push((name, total.rows, total.declared, total.returned));
    }
    assert_eq!(found, ARRAY_TOTALS);
    assert_eq!(rows, ROW_TOTAL);
}

#[test]
fn the_committed_map_states_thirteen_layouts_that_agree_with_the_walk() {
    let map = committed();
    assert_eq!(map.layouts.len(), 13);
    assert_eq!(map.layouts, measured().layouts);
}

/// The committed map, changed by `edit`, with the text the writer renders
/// from it, so that only the values can disagree with the walk.
fn edited_committed(edit: impl Fn(&mut FidelityMap)) -> (FidelityMap, String) {
    let original = committed();
    let mut held = original.clone();
    edit(&mut held);
    assert_ne!(
        held, original,
        "the edit must change the map, or this test proves nothing"
    );
    let text = render(&held);
    (held, text)
}

#[test]
fn raising_a_pinned_record_count_fails_with_regression_and_the_table_to_paste() {
    let (held, text) = edited_committed(|map| {
        let modelled = map.layouts["Object"].modelled;
        let object = map
            .programs
            .get_mut(HEX_SCROLL)
            .unwrap()
            .structures
            .get_mut("Object")
            .unwrap();
        object.records += 1;
        object.same += modelled;
    });
    let failures = gate_failures(&held, &text, measured());
    assert_eq!(failures.len(), 1, "{failures:#?}");
    let message = &failures[0];
    assert!(message.starts_with(&format!(
        "{HEX_SCROLL}: 2 value(s) in tests/fidelity.toml disagree with the walk:\n"
    )));
    assert!(
        message.contains(
            "  REGRESSION structure.Object.records: the map holds 3, the walk measures 2\n"
        )
    );
    assert!(
        message.contains(
            "  REGRESSION structure.Object.same: the map holds 60, the walk measures 40\n"
        )
    );
    let block = render_program(HEX_SCROLL, &measured().programs[HEX_SCROLL]);
    assert!(
        message.ends_with(&format!(
            "{HOW_TO_FIX}, or put this table in place of the old one:\n{block}"
        )),
        "{message}"
    );
}

#[test]
fn lowering_a_pinned_record_count_fails_with_moved_up() {
    let (held, text) = edited_committed(|map| {
        let modelled = map.layouts["Object"].modelled;
        let object = map
            .programs
            .get_mut(HEX_SCROLL)
            .unwrap()
            .structures
            .get_mut("Object")
            .unwrap();
        object.records -= 1;
        object.same -= modelled;
    });
    let failures = gate_failures(&held, &text, measured());
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert!(
        failures[0].contains(
            "  MOVED UP structure.Object.records: the map holds 1, the walk measures 2\n"
        )
    );
}

#[test]
fn a_measured_differing_byte_fails_with_regression() {
    let mut walk = measured().clone();
    let object = walk
        .programs
        .get_mut(HEX_SCROLL)
        .unwrap()
        .structures
        .get_mut("Object")
        .unwrap();
    object.differs = 1;
    object.same -= 1;
    let failures = gate_failures(&committed(), committed_text(), &walk);
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert!(
        failures[0].contains(
            "  REGRESSION structure.Object.differs: the map holds 0, the walk measures 1\n"
        )
    );
}

#[test]
fn a_changed_declared_count_fails_with_changed() {
    let (held, text) = edited_committed(|map| {
        let program = map.programs.get_mut(HEX_SCROLL).unwrap();
        program.arrays.get_mut("Controls").unwrap().declared += 1;
    });
    let failures = gate_failures(&held, &text, measured());
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert!(
        failures[0].contains("  CHANGED array.Controls.declared: the map holds "),
        "{failures:#?}"
    );
}

#[test]
fn a_missing_program_and_a_stale_program_give_two_different_messages() {
    let (held, text) = edited_committed(|map| {
        let program = map.programs.remove(HEX_SCROLL).unwrap();
        map.programs
            .insert("vb6-code/Nowhere/None.exe".to_owned(), program);
    });
    let failures = gate_failures(&held, &text, measured());
    assert_eq!(failures.len(), 2, "{failures:#?}");
    let missing = failures
        .iter()
        .find(|message| message.starts_with(HEX_SCROLL))
        .unwrap();
    assert!(missing.contains("this corpus program has no table in tests/fidelity.toml"));
    assert!(missing.contains(&render_program(
        HEX_SCROLL,
        &measured().programs[HEX_SCROLL]
    )));
    let stale = failures
        .iter()
        .find(|message| message.starts_with("vb6-code/Nowhere/None.exe"))
        .unwrap();
    assert!(stale.contains("no corpus program has it"));
    assert!(stale.contains("or delete this table"));
}

#[test]
fn a_moved_layout_names_the_structure_and_gives_its_table() {
    let (held, text) = edited_committed(|map| {
        map.layouts.get_mut("Object").unwrap().unmodelled = vec![(0x04, 20), (0x28, 4), (0x2C, 4)];
    });
    let failures = gate_failures(&held, &text, measured());
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert!(failures[0].starts_with(
        "layout.Object: CHANGED unmodelled: the map holds [[0x04, 20], [0x28, 4], [0x2C, 4]], \
         the walk measures [[0x04, 20], [0x24, 4], [0x2C, 4]]\n"
    ));
    assert!(failures[0].ends_with(&render_layout("Object", &measured().layouts["Object"])));
}

#[test]
fn a_same_count_edited_alone_fails_the_derived_check() {
    let (held, _text) = edited_committed(|map| {
        let program = map.programs.get_mut(HEX_SCROLL).unwrap();
        program.structures.get_mut("Object").unwrap().same += 1;
    });
    let failures = derived_failures(&held);
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert!(failures[0].starts_with(&format!(
        "{HEX_SCROLL}: structure.Object holds 2 records and 41 + 0 bytes"
    )));
}

/// Gives the gate's failures for `text`, which must parse to the committed
/// values.
fn failures_for_text_with_the_committed_values(text: &str) -> Vec<String> {
    assert_ne!(text, committed_text(), "the edit must change the text");
    let held = parse(text).unwrap();
    assert_eq!(held, committed(), "the edit must keep every value");
    gate_failures(&held, text, measured())
}

#[test]
fn two_program_tables_in_the_wrong_order_fail_only_the_canonical_check() {
    let chunks: Vec<&str> = committed_text().split("\n\n").collect();
    let first = chunks
        .iter()
        .position(|chunk| chunk.starts_with("[program."))
        .unwrap();
    let mut swapped = chunks.clone();
    swapped.swap(first, first + 1);
    let failures = failures_for_text_with_the_committed_values(&swapped.join("\n\n"));
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert!(failures[0].contains("is not the text that the writer renders"));
}

#[test]
fn a_count_written_in_hex_fails_only_the_canonical_check() {
    let text = committed_text().replacen(
        "structure.VBHeader = { records = 1,",
        "structure.VBHeader = { records = 0x1,",
        1,
    );
    let failures = failures_for_text_with_the_committed_values(&text);
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert!(failures[0].contains("the file holds \"structure.VBHeader = { records = 0x1,"));
}
