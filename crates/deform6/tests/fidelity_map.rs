#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The committed fidelity map, `tests/fidelity.toml`.
//!
//! `shared.rs` holds the measurement, the text and the parser, and
//! `crates/xtask` compiles the same file to write the map. This file tests
//! that shared code.

#[path = "fidelity_map/shared.rs"]
#[allow(
    dead_code,
    reason = "this module is embedded in two binaries, this test target and crates/xtask, and \
              each uses a different part of it"
)]
mod shared;

use std::collections::BTreeMap;

use shared::{
    ARRAYS, Change, Counted, Direction, EXPECTED_PROGRAM_COUNT, FidelityMap, Graded, Layout,
    ProgramMap, STRUCTURES, array_name, changes, executables, fidelity_toml_path, measure, parse,
    render, render_layout, render_program,
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
