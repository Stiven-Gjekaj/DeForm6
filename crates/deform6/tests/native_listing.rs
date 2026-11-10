#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! Asks each native corpus program for its annotated disassembly, and
//! compares the answer with the source that the program was built from.
//!
//! `tests/native.toml` pins the numbers. Set `DEFORM6_PRINT_NATIVE` to print
//! the table that the run measures.

#[allow(dead_code, reason = "each test file uses a part of the harness")]
mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use deform6::read::pe::{ImportName, PeImage};
use deform6::vb::helpers::idiom;
use deform6::vb::native::{Listing, Names, Note, disassemble, handler_names, object_methods};
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::types::VbTypes;
use support::{source, vbp};

/// What one program gives.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Measure {
    /// The procedures that the listing holds.
    procedures: usize,
    /// The procedures that the source declares.
    declared: usize,
    /// The instructions that the listing holds.
    instructions: usize,
    /// The string notes of the listing.
    strings: usize,
    /// The string notes whose text is in the source.
    strings_in_source: usize,
    /// The notes that name a helper or an ordinal of the runtime.
    runtime_calls: usize,
    /// The notes of a named helper that has an idiom.
    idioms: usize,
    /// The object calls that a check block names.
    checked_calls: usize,
    /// The calls through the vtable of the object of the procedure.
    me_calls: usize,
    /// The calls through the vtable of the default instance of a form.
    instance_calls: usize,
    /// The floating point constants that the listing gives.
    numbers: usize,
    /// The floating point constants that the source holds as a literal.
    numbers_in_source: usize,
}

/// Gives the source text of each object of the project of `exe`, and the
/// count of the procedures that it declares.
fn source_of(exe: &Path, projects: &[PathBuf]) -> (String, usize) {
    let project_path = vbp::select_project_file(exe, projects)
        .unwrap_or_else(|err| panic!("{}: {err}", exe.display()));
    let project = vbp::Project::read(&project_path);
    let mut text = String::new();
    let mut declared = 0;
    for object in project.declared_objects() {
        if !object.source_file.is_file() {
            continue;
        }
        declared += source::declared_procedures(&object.source_file).len();
        text.extend(
            std::fs::read(&object.source_file)
                .unwrap()
                .iter()
                .map(|byte| char::from(*byte)),
        );
        text.push('\n');
    }
    (text, declared)
}

/// Tells whether the source holds the literal `found`. A tab is the
/// intrinsic constant `vbTab`, which the source names and does not quote.
fn in_source(found: &str, text: &str) -> bool {
    text.contains(&found.replace('"', "\"\"")) || (found == "\t" && text.contains("vbTab"))
}

/// Tells whether two numbers round to the same `Single`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the test asks what the conversion to a Single gives"
)]
fn same_single(a: f64, b: f64) -> bool {
    a as f32 == b as f32
}

/// Tells whether a number of the source text gives the same value as
/// `found`. The source can write it with no leading zero, with a type
/// sign, with an exponent, or as a `Single` that rounds to the same bits.
fn number_in_source(found: &str, text: &str) -> bool {
    let value: f64 = found.parse().unwrap();
    let wanted = value.abs();
    let is_number = |c: char| c.is_ascii_digit() || c == '.';
    let bytes: Vec<char> = text.chars().collect();
    let mut at = 0;
    while at < bytes.len() {
        if !is_number(bytes[at])
            || (at > 0 && (bytes[at - 1].is_alphanumeric() || bytes[at - 1] == '_'))
        {
            at += 1;
            continue;
        }
        let start = at;
        while at < bytes.len() && is_number(bytes[at]) {
            at += 1;
        }
        if at + 1 < bytes.len() && matches!(bytes[at], 'e' | 'E' | 'd' | 'D') {
            let mut end = at + 1;
            if matches!(bytes[end], '+' | '-') {
                end += 1;
            }
            let digits = end;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > digits {
                at = end;
            }
        }
        let literal: String = bytes[start..at].iter().collect();
        if let Ok(number) = literal.replace(['d', 'D'], "e").parse::<f64>()
            && (number == wanted || same_single(number, wanted))
        {
            return true;
        }
    }
    false
}

/// Measures `listing` against `text`, and fails on a fault of the walk.
fn measure(exe: &Path, listing: &Listing, text: &str, declared: usize) -> Measure {
    let mut result = Measure {
        procedures: listing.procedures.len(),
        declared,
        instructions: listing.procedures.iter().map(|p| p.lines.len()).sum(),
        strings: 0,
        strings_in_source: 0,
        runtime_calls: 0,
        idioms: 0,
        checked_calls: 0,
        me_calls: 0,
        instance_calls: 0,
        numbers: 0,
        numbers_in_source: 0,
    };
    assert!(!listing.truncated, "{}: truncated", exe.display());
    for procedure in &listing.procedures {
        assert!(
            procedure.open_jumps.is_empty() && procedure.undecoded.is_empty(),
            "{} at {:08X}: open {:?}, undecoded {:?}",
            exe.display(),
            procedure.entry.get(),
            procedure.open_jumps,
            procedure.undecoded
        );
        for note in procedure.lines.iter().flat_map(|line| &line.notes) {
            match note {
                Note::Text(found) => {
                    result.strings += 1;
                    if in_source(found, text) {
                        result.strings_in_source += 1;
                    } else if std::env::var_os("DEFORM6_PRINT_NATIVE").is_some() {
                        println!("# not in source: {found:?}");
                    }
                }
                Note::Import(function) => {
                    result.runtime_calls += 1;
                    if let ImportName::Name(name) = &function.name
                        && idiom(name).is_some()
                    {
                        result.idioms += 1;
                    }
                }
                Note::Interface { .. } => result.checked_calls += 1,
                Note::Me(_) => result.me_calls += 1,
                Note::Instance { .. } => result.instance_calls += 1,
                Note::Number(found) => {
                    result.numbers += 1;
                    if number_in_source(found, text) {
                        result.numbers_in_source += 1;
                    } else if std::env::var_os("DEFORM6_PRINT_NATIVE").is_some() {
                        println!("# no literal: {found}");
                    }
                }
                _ => {}
            }
        }
    }
    result
}

/// Reads the pinned numbers.
fn pinned() -> BTreeMap<String, toml::Table> {
    let path = vbp::corpus_root().join("../tests/native.toml");
    let text = std::fs::read_to_string(path).unwrap();
    let table: toml::Table = text.parse().unwrap();
    table["program"]
        .as_table()
        .unwrap()
        .iter()
        .map(|(key, value)| (key.clone(), value.as_table().unwrap().clone()))
        .collect()
}

/// Gives the path of `exe` relative to the corpus, with forward slashes.
fn key_of(exe: &Path) -> String {
    exe.strip_prefix(vbp::corpus_root())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn each_native_program_gives_the_pinned_listing_with_no_fault() {
    let projects = vbp::project_files();
    let pins = pinned();
    let print = std::env::var_os("DEFORM6_PRINT_NATIVE").is_some();
    let mut failures = Vec::new();
    let mut seen = 0;
    for exe in &vbp::executables() {
        let data = std::fs::read(exe).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        let listing = disassemble(&pe);
        let (text, declared) = source_of(exe, &projects);
        let got = measure(exe, &listing, &text, declared);
        let key = key_of(exe);
        if print {
            println!(
                "[program.\"{key}\"]\nprocedures = {}\ndeclared = {}\ninstructions = {}\nstrings = {}\nstrings_in_source = {}\nruntime_calls = {}\nidioms = {}\nchecked_calls = {}\nme_calls = {}\ninstance_calls = {}\nnumbers = {}\nnumbers_in_source = {}\n",
                got.procedures,
                got.declared,
                got.instructions,
                got.strings,
                got.strings_in_source,
                got.runtime_calls,
                got.idioms,
                got.checked_calls,
                got.me_calls,
                got.instance_calls,
                got.numbers,
                got.numbers_in_source
            );
            continue;
        }
        seen += 1;
        let Some(pin) = pins.get(&key) else {
            failures.push(format!("{key}: no entry in tests/native.toml, got {got:?}"));
            continue;
        };
        let number = |name: &str| usize::try_from(pin[name].as_integer().unwrap()).unwrap();
        let want = Measure {
            procedures: number("procedures"),
            declared: number("declared"),
            instructions: number("instructions"),
            strings: number("strings"),
            strings_in_source: number("strings_in_source"),
            runtime_calls: number("runtime_calls"),
            idioms: number("idioms"),
            checked_calls: number("checked_calls"),
            me_calls: number("me_calls"),
            instance_calls: number("instance_calls"),
            numbers: number("numbers"),
            numbers_in_source: number("numbers_in_source"),
        };
        if got != want {
            failures.push(format!("{key}: pinned {want:?}, now {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if !print {
        assert_eq!(seen, pins.len(), "tests/native.toml holds a stale entry");
    }
}

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

/// Each procedure slot of a link table goes to a procedure of the listing.
#[test]
fn each_native_link_slot_goes_to_a_procedure_of_the_listing() {
    let mut slots = 0;
    for exe in &vbp::executables() {
        let data = std::fs::read(exe).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        let listing = disassemble(&pe);
        let entries: BTreeSet<u32> = listing.procedures.iter().map(|p| p.entry.get()).collect();
        for (address, method) in object_methods(&pe) {
            slots += 1;
            assert!(
                entries.contains(&address),
                "{}: {} at {address:08X}",
                exe.display(),
                method.text()
            );
        }
    }
    assert_eq!(slots, 701, "the number of the link slots moved");
}

/// The link table gives the place of each procedure of an object in its
/// source. A public procedure gives the name at that place, and an event
/// handler that the control records name gives its name at that place.
#[test]
fn each_link_slot_gives_the_place_of_its_procedure_in_the_source() {
    let table = OpcodeTable::builtin();
    let projects = vbp::project_files();
    let (mut methods, mut public, mut handlers) = (0, 0, 0);
    for exe in &vbp::executables() {
        let data = std::fs::read(exe).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict).unwrap();
        let named = handler_names(&report);
        let project_path = vbp::select_project_file(exe, &projects).unwrap();
        let declared: BTreeMap<String, Vec<String>> = vbp::Project::read(&project_path)
            .declared_objects()
            .into_iter()
            .filter(|object| object.source_file.is_file())
            .filter_map(|object| {
                let names = source::declared_procedures(&object.source_file);
                Some((object.name?, names))
            })
            .collect();
        for (address, method) in object_methods(&pe) {
            let Some(names) = declared.get(&method.object) else {
                continue;
            };
            methods += 1;
            let place = &names[usize::from(method.index)];
            if let Some(name) = &method.name {
                public += 1;
                assert!(
                    place.eq_ignore_ascii_case(name),
                    "{}: {name} is {place}",
                    exe.display()
                );
            }
            let Some(handler) = named.get(&address) else {
                continue;
            };
            let (_, event) = handler.split_once('.').unwrap();
            if event.contains("_event") {
                let (owner, _) = event.split_once("_event").unwrap();
                assert!(
                    place
                        .to_ascii_lowercase()
                        .starts_with(&owner.to_ascii_lowercase()),
                    "{}: {handler} is {place} in the source",
                    exe.display()
                );
            } else {
                assert!(
                    place.eq_ignore_ascii_case(event),
                    "{}: {handler} is {place}",
                    exe.display()
                );
            }
            handlers += 1;
        }
    }
    assert_eq!(
        (methods, public, handlers),
        (684, 185, 389),
        "the number of the checked procedures moved"
    );
}

/// The default member of a control or of `Err`. A statement can leave the
/// name out, such as `Text1 = "a"`.
const DEFAULT_MEMBERS: [&str; 4] = ["Text", "Caption", "Value", "Number"];

/// Gives the types file of this machine, when `derive-vb-types` wrote it.
fn derived_types() -> Option<VbTypes> {
    let path = vbp::corpus_root().join("../derived/vb-types.toml");
    VbTypes::parse(&std::fs::read(path).ok()?).ok()
}

/// Gives the name that the text of an object call gives: the member at the
/// end of the text, with no `(get)`.
fn member_of(text: &str) -> &str {
    let name = text.rsplit_once('.').map_or(text, |(_, name)| name);
    name.split(' ').next().unwrap_or(name)
}

/// The source of the procedure at the place of an object call names the
/// member that the listing names for the call. A call of a default member
/// can have no name in the source. The rest of the calls have no name in
/// the listing, and the test counts them.
#[test]
fn each_object_call_names_a_member_that_its_source_procedure_names() {
    let Some(types) = derived_types() else {
        println!("no types file in derived/, so this test checks nothing");
        return;
    };
    let table = OpcodeTable::builtin();
    let projects = vbp::project_files();
    let (mut total, mut found, mut unnamed) = (0, 0, 0);
    let mut misses = BTreeSet::<String>::new();
    for exe in &vbp::executables() {
        let data = std::fs::read(exe).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        let report = deform6::inspect_with_types(
            &data,
            &table,
            Some(&types),
            deform6::journal::Mode::Strict,
        )
        .unwrap();
        let names = Names::read(&pe, &disassemble(&pe), Some(&report), Some(&types));
        let project_path = vbp::select_project_file(exe, &projects).unwrap();
        let bodies: BTreeMap<String, Vec<(String, Vec<String>)>> =
            vbp::Project::read(&project_path)
                .declared_objects()
                .into_iter()
                .filter(|object| object.source_file.is_file())
                .filter_map(|object| {
                    let bodies = source::procedure_bodies(&object.source_file);
                    Some((object.name?, bodies))
                })
                .collect();
        let listing = disassemble(&pe);
        for procedure in &listing.procedures {
            let Some(method) = names.methods.get(&procedure.entry.get()) else {
                continue;
            };
            let Some(objects) = bodies.get(&method.object) else {
                continue;
            };
            let (_, lines) = &objects[usize::from(method.index)];
            let body = lines.join("\n").to_ascii_lowercase();
            for note in procedure.lines.iter().flat_map(|line| &line.notes) {
                if !matches!(
                    note,
                    Note::Me(_) | Note::Interface { .. } | Note::Instance { .. }
                ) {
                    continue;
                }
                let text = names.note(&pe, note, procedure.entry);
                let member = member_of(&text);
                if text.contains("vtable offset") || text.contains("public variable") {
                    unnamed += 1;
                    continue;
                }
                if matches!(member, "AddRef" | "Release" | "QueryInterface" | "Item") {
                    continue;
                }
                total += 1;
                let word = match member.strip_prefix("proc") {
                    Some(place) if place.parse::<usize>().is_ok() => objects
                        [place.parse::<usize>().unwrap()]
                    .0
                    .to_ascii_lowercase(),
                    _ => member.to_ascii_lowercase(),
                };
                if body.contains(&word) {
                    found += 1;
                } else {
                    misses.insert(text);
                }
            }
        }
    }
    for text in &misses {
        assert!(
            DEFAULT_MEMBERS.contains(&member_of(text)),
            "{text} is in no statement of its procedure"
        );
    }
    assert_eq!(
        (total, found, unnamed),
        (4895, 4795, 8),
        "the object calls moved"
    );
}

/// The helpers whose idiom a statement cannot carry out by itself: each
/// call of the helper comes from a word of the source, and each such word
/// compiles to the helper.
const STATEMENT_HELPERS: [(&str, &str); 21] = [
    ("__vbaEnd", "end"),
    ("__vbaErase", "erase"),
    ("__vbaFPInt", "int("),
    ("__vbaFileClose", "close"),
    ("__vbaFileOpen", "open"),
    ("__vbaGet3", "get"),
    ("__vbaGet4", "get"),
    ("__vbaInStr", "instr"),
    ("__vbaLenBstr", "len("),
    ("__vbaLenBstrB", "lenb("),
    ("__vbaLineInputStr", "line input"),
    ("__vbaNew", "new"),
    ("__vbaOnError", "on error"),
    ("__vbaPowerR8", "^"),
    ("__vbaPrintFile", "print"),
    ("__vbaPut3", "put"),
    ("__vbaPut4", "put"),
    ("__vbaRedim", "redim"),
    ("__vbaRedimPreserve", "redim preserve"),
    ("__vbaStrCat", "&"),
    ("__vbaUbound", "ubound"),
];

/// The words of the source that always compile to a call of one of these
/// helpers.
const WORD_HELPERS: [(&str, &[&str]); 6] = [
    ("redim ", &["__vbaRedim", "__vbaRedimPreserve"]),
    ("erase ", &["__vbaErase"]),
    ("instr(", &["__vbaInStr", "__vbaInStrVar"]),
    ("ubound(", &["__vbaUbound"]),
    ("line input ", &["__vbaLineInputStr", "__vbaLineInputVar"]),
    ("on error ", &["__vbaOnError"]),
];

/// One procedure of the listing, with its source.
struct Sourced {
    /// The text of the source body.
    body: String,
    /// The name of each helper that the procedure calls.
    helpers: Vec<String>,
    /// The text of each string note.
    strings: Vec<String>,
    /// The member of each late-bound call, and the class of each `New`.
    named_words: Vec<String>,
}

/// Gives each procedure of `exe` whose place in the source the listing
/// knows.
fn sourced_procedures(exe: &Path, projects: &[PathBuf]) -> Vec<Sourced> {
    let data = std::fs::read(exe).unwrap();
    let pe = PeImage::parse(&data).unwrap();
    let listing = disassemble(&pe);
    let names = Names::read(&pe, &listing, None, None);
    let project_path = vbp::select_project_file(exe, projects).unwrap();
    let bodies: BTreeMap<String, Vec<(String, Vec<String>)>> = vbp::Project::read(&project_path)
        .declared_objects()
        .into_iter()
        .filter(|object| object.source_file.is_file())
        .filter_map(|object| {
            let bodies = source::procedure_bodies(&object.source_file);
            Some((object.name?, bodies))
        })
        .collect();
    let mut out = Vec::new();
    for procedure in &listing.procedures {
        let Some(method) = names.methods.get(&procedure.entry.get()) else {
            continue;
        };
        let Some(objects) = bodies.get(&method.object) else {
            continue;
        };
        let body = objects
            .get(usize::from(method.index))
            .map(|(_, lines)| lines.join("\n"))
            .unwrap_or_default();
        let helpers = procedure
            .lines
            .iter()
            .filter(|line| line.text.starts_with("call"))
            .flat_map(|line| &line.notes)
            .filter_map(|note| match note {
                Note::Import(function) => match &function.name {
                    ImportName::Name(name) => Some(name.clone()),
                    ImportName::Ordinal(_) => None,
                },
                _ => None,
            })
            .collect();
        let strings = procedure
            .lines
            .iter()
            .flat_map(|line| &line.notes)
            .filter_map(|note| match note {
                Note::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect();
        let named_words = procedure
            .lines
            .iter()
            .flat_map(|line| &line.notes)
            .filter_map(|note| match note {
                Note::LateMember(name) => Some(name.clone()),
                Note::NewObject(_) => {
                    let text = names.note(&pe, note, procedure.entry);
                    text.strip_prefix("New ")
                        .filter(|class| !class.contains(' '))
                        .map(str::to_owned)
                }
                _ => None,
            })
            .collect();
        out.push(Sourced {
            body,
            helpers,
            strings,
            named_words,
        });
    }
    out
}

/// Gives `body` in lower case, with one form of `On Error`.
fn normal_body(body: &str) -> String {
    body.to_ascii_lowercase()
        .replace("on local error", "on error")
}

/// Tells whether `body` holds the string `found`. The compiler joins a
/// literal and an intrinsic constant into one string, so each part between
/// control characters must be in `body`. A string of control characters
/// alone is an intrinsic constant, which the source names and does not
/// quote.
fn in_procedure(found: &str, body: &str) -> bool {
    let parts: Vec<&str> = found
        .split(['\r', '\n', '\t'])
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return body.to_ascii_lowercase().contains("vb");
    }
    parts.iter().all(|part| in_source(part, body))
}

/// Each string note is a literal of the source of its own procedure. So the
/// place of each procedure of an object is the place in the source.
#[test]
fn each_string_note_is_a_literal_of_its_source_procedure() {
    let projects = vbp::project_files();
    let mut strings = 0;
    for exe in &vbp::executables() {
        for procedure in sourced_procedures(exe, &projects) {
            for text in &procedure.strings {
                assert!(
                    in_procedure(text, &procedure.body),
                    "{}: {text:?} is not in\n{}",
                    exe.display(),
                    procedure.body
                );
                strings += 1;
            }
        }
    }
    assert_eq!(strings, 1020, "the string notes moved");
}

/// The member of each late-bound call and the class of each `New` is a
/// word of the source of the procedure.
#[test]
fn each_late_bound_member_and_class_is_a_word_of_its_source_procedure() {
    let projects = vbp::project_files();
    let mut words = 0;
    for exe in &vbp::executables() {
        for procedure in sourced_procedures(exe, &projects) {
            let body = normal_body(&procedure.body);
            for word in &procedure.named_words {
                assert!(
                    body.contains(&word.to_ascii_lowercase()),
                    "{}: {word:?} is not in\n{body}",
                    exe.display()
                );
                words += 1;
            }
        }
    }
    assert_eq!(words, 171, "the late-bound and New notes moved");
}

/// Each call of a helper of [`STATEMENT_HELPERS`] is in a procedure whose
/// source holds the word of its idiom.
#[test]
fn the_idiom_of_each_statement_helper_is_a_word_of_its_source_procedure() {
    let projects = vbp::project_files();
    let mut calls = 0;
    for exe in &vbp::executables() {
        for procedure in sourced_procedures(exe, &projects) {
            let body = normal_body(&procedure.body);
            for name in &procedure.helpers {
                let Some((helper, word)) =
                    STATEMENT_HELPERS.iter().find(|(helper, _)| helper == name)
                else {
                    continue;
                };
                let text = idiom(helper).unwrap().to_ascii_lowercase();
                assert!(
                    text.contains(word.trim_end_matches('(')),
                    "{helper}: {text}"
                );
                assert!(
                    body.contains(word),
                    "{}: {helper} and no {word:?} in\n{body}",
                    exe.display()
                );
                calls += 1;
            }
        }
    }
    assert_eq!(calls, 1264, "the calls of the statement helpers moved");
}

/// Each procedure whose source holds a word of [`WORD_HELPERS`] calls one
/// of its helpers.
#[test]
fn each_word_of_the_source_gives_its_helper_in_the_listing() {
    let projects = vbp::project_files();
    let mut words = 0;
    for exe in &vbp::executables() {
        for procedure in sourced_procedures(exe, &projects) {
            let body = normal_body(&procedure.body);
            let helpers = &procedure.helpers;
            for (word, wanted) in WORD_HELPERS {
                if !body.contains(word) {
                    continue;
                }
                words += 1;
                assert!(
                    helpers.iter().any(|name| wanted.contains(&name.as_str())),
                    "{}: {word:?} and no {wanted:?} in\n{body}",
                    exe.display()
                );
            }
        }
    }
    assert_eq!(words, 117, "the procedures with these words moved");
}

/// A joined note names a control and its member. The source of the
/// procedure uses that control, and uses that member of it. A statement
/// can leave out a default member, and a control array gives `Item`.
#[test]
fn each_joined_call_names_a_control_and_a_member_of_its_source_procedure() {
    let Some(types) = derived_types() else {
        println!("no types file in derived/, so this test checks nothing");
        return;
    };
    let table = OpcodeTable::builtin();
    let projects = vbp::project_files();
    let (mut total, mut named, mut bare) = (0, 0, 0);
    for exe in &vbp::executables() {
        let data = std::fs::read(exe).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        let report = deform6::inspect_with_types(
            &data,
            &table,
            Some(&types),
            deform6::journal::Mode::Strict,
        )
        .unwrap();
        let listing = disassemble(&pe);
        let names = Names::read(&pe, &listing, Some(&report), Some(&types));
        let project_path = vbp::select_project_file(exe, &projects).unwrap();
        let bodies: BTreeMap<String, Vec<(String, Vec<String>)>> =
            vbp::Project::read(&project_path)
                .declared_objects()
                .into_iter()
                .filter(|object| object.source_file.is_file())
                .filter_map(|object| {
                    let bodies = source::procedure_bodies(&object.source_file);
                    Some((object.name?, bodies))
                })
                .collect();
        for procedure in &listing.procedures {
            let Some(method) = names.methods.get(&procedure.entry.get()) else {
                continue;
            };
            let Some(objects) = bodies.get(&method.object) else {
                continue;
            };
            let body = objects[usize::from(method.index)]
                .1
                .join("\n")
                .to_ascii_lowercase();
            for line in &procedure.lines {
                if !line
                    .notes
                    .iter()
                    .any(|note| matches!(note, Note::Receiver { .. }))
                {
                    continue;
                }
                let Some(text) = names.line_notes(&pe, line, procedure.entry).pop() else {
                    continue;
                };
                let lower = text.to_ascii_lowercase();
                let path = lower.split(' ').next().unwrap();
                let parts: Vec<&str> = path.split('.').collect();
                if parts.len() < 2 {
                    continue;
                }
                let member = parts[parts.len() - 1];
                let control = parts[parts.len() - 2];
                total += 1;
                if body.contains(&format!("{control}.{member}")) {
                    named += 1;
                } else {
                    assert!(
                        body.contains(control)
                            && matches!(member, "text" | "caption" | "value" | "item" | "number"),
                        "{}: {text} is in no statement of {}\n{body}",
                        exe.display(),
                        method.text()
                    );
                    bare += 1;
                }
            }
        }
    }
    assert_eq!(
        (total, named, bare),
        (1201, 1095, 106),
        "the joined calls moved"
    );
}

/// The name that the listing gives an argument is a word of the source
/// of its procedure.
#[test]
fn each_argument_name_is_a_word_of_its_source_procedure() {
    let Some(types) = derived_types() else {
        println!("no types file in derived/, so this test checks nothing");
        return;
    };
    let table = OpcodeTable::builtin();
    let projects = vbp::project_files();
    let mut total = 0;
    for exe in &vbp::executables() {
        let data = std::fs::read(exe).unwrap();
        let pe = PeImage::parse(&data).unwrap();
        let report = deform6::inspect_with_types(
            &data,
            &table,
            Some(&types),
            deform6::journal::Mode::Strict,
        )
        .unwrap();
        let listing = disassemble(&pe);
        let names = Names::read(&pe, &listing, Some(&report), Some(&types));
        let project_path = vbp::select_project_file(exe, &projects).unwrap();
        let bodies: BTreeMap<String, Vec<(String, Vec<String>)>> =
            vbp::Project::read(&project_path)
                .declared_objects()
                .into_iter()
                .filter(|object| object.source_file.is_file())
                .filter_map(|object| {
                    let bodies = source::procedure_bodies(&object.source_file);
                    Some((object.name?, bodies))
                })
                .collect();
        for procedure in &listing.procedures {
            let Some(method) = names.methods.get(&procedure.entry.get()) else {
                continue;
            };
            let Some(objects) = bodies.get(&method.object) else {
                continue;
            };
            let body = objects[usize::from(method.index)]
                .1
                .join("\n")
                .to_ascii_lowercase();
            for note in procedure.lines.iter().flat_map(|line| &line.notes) {
                let Note::Argument(_) = note else {
                    continue;
                };
                let text = names.note(&pe, note, procedure.entry);
                if text.is_empty() {
                    continue;
                }
                total += 1;
                assert!(
                    body.contains(&text.to_ascii_lowercase()),
                    "{}: {} has no {text}",
                    exe.display(),
                    method.text()
                );
            }
        }
    }
    assert_eq!(total, 557, "the argument notes moved");
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
