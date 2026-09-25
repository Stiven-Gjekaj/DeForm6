#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The committed build record, `tests/builds.toml`, and the gate that holds
//! the tree to it.
//!
//! `shared.rs` holds the files that a build covers, their hash, the text and
//! the parser, and `crates/xtask` compiles the same file to write the record.
//! This file tests that shared code, and then holds the tree to the record.
//!
//! # What the gate proves
//!
//! Each result in the record is true for the files that the Windows host
//! built. The gate writes the files of each program again and compares
//! their hash with the hash in the record. When DeForm6 writes different
//! files, the gate fails with `STALE` until the host builds the new files.
//! No test runs the Visual Basic 6 IDE: this machine cannot.

#[path = "build_record/shared.rs"]
#[allow(
    dead_code,
    reason = "this module is embedded in two binaries, this test target and crates/xtask, and \
              each uses a different part of it"
)]
mod shared;

use std::collections::BTreeMap;

use shared::{
    BuildRecord, Outcome, ProgramBuild, Side, build_files, builds_toml_path, corpus_root,
    executables, files_hash, parse, program_key, project_files, render,
};

/// A file with a name and bytes, built here.
fn file(name: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    (name.to_owned(), bytes.to_vec())
}

/// A record with every outcome, a message with a quote and a backslash, and
/// a key with a space, built here and not read from a file.
fn synthetic_record() -> BuildRecord {
    let mut programs = BTreeMap::new();
    programs.insert(
        "vb6-code/Part 3 - A/A.exe".to_owned(),
        ProgramBuild {
            files: files_hash(&[file("A.vbp", b"Type=Exe\r\n")]),
            original: Side {
                outcome: Outcome::Built,
                messages: Vec::new(),
            },
            extracted: Side {
                outcome: Outcome::Failed,
                messages: vec![
                    "Line 12: Property \"Bogus\" in Form1 could not be set.".to_owned(),
                    r"Errors during load. Refer to 'p01\Form1.log'".to_owned(),
                ],
            },
        },
    );
    programs.insert(
        "public-domain/B/B.exe".to_owned(),
        ProgramBuild {
            files: files_hash(&[file("B.vbp", b"Type=Exe\r\n")]),
            original: Side {
                outcome: Outcome::BuiltWithLoadErrors,
                messages: vec![
                    "Form1.log: Line 5: Class VB.Control of control X was not a loaded control \
                     class."
                        .to_owned(),
                ],
            },
            extracted: Side {
                outcome: Outcome::NotRun,
                messages: Vec::new(),
            },
        },
    );
    BuildRecord {
        windows: "Microsoft Windows XP [Version 5.1.2600]".to_owned(),
        vb6: r#"C:\Program Files\Microsoft Visual Studio\VB98\VB6.EXE "1,880,064""#.to_owned(),
        programs,
    }
}

/// A record that `render` writes reads back as the same record. The text
/// holds quotes, backslashes and a key with spaces, and the `toml` crate
/// reads them.
#[test]
fn a_record_that_render_writes_reads_back_the_same() {
    let record = synthetic_record();
    let text = render(&record);
    assert_eq!(parse(&text).unwrap(), record, "{text}");
}

/// The parser refuses a word that is not an outcome, a hash of another
/// shape, and an entry that the record does not define.
#[test]
fn the_parser_refuses_an_unknown_outcome_a_bad_hash_and_an_unknown_entry() {
    let text = render(&synthetic_record());

    let bad_word = text.replacen("original = \"built\"", "original = \"compiled\"", 1);
    assert_ne!(bad_word, text);
    let err = parse(&bad_word).unwrap_err();
    assert!(err.contains("\"compiled\""), "{err}");

    let bad_hash = text.replacen("files = \"sha256:", "files = \"md5:", 1);
    assert_ne!(bad_hash, text);
    let err = parse(&bad_hash).unwrap_err();
    assert!(err.contains("md5:"), "{err}");

    // One hexadecimal character short, and the same hash in upper case.
    let held = files_hash(&[file("A.vbp", b"Type=Exe\r\n")]);
    let short = text.replacen(&held, &held[..held.len() - 1], 1);
    assert_ne!(short, text);
    assert!(parse(&short).is_err());
    let upper = text.replacen(&held, &format!("sha256:{}", held[7..].to_uppercase()), 1);
    assert_ne!(upper, text);
    assert!(parse(&upper).is_err());

    let unknown = text.replacen("[environment]\n", "[environment]\nhost = \"x\"\n", 1);
    assert_ne!(unknown, text);
    let err = parse(&unknown).unwrap_err();
    assert!(err.contains("\"host\""), "{err}");
}

/// The hash changes when one byte of one file changes, and when one name
/// changes.
#[test]
fn one_changed_byte_or_one_changed_name_changes_the_hash() {
    let files = [
        file("A.vbp", b"Type=Exe\r\n"),
        file("Form1.frm", b"VERSION 5.00\r\n"),
    ];
    let held = files_hash(&files);

    let mut byte = files.clone();
    byte[1].1[0] = b'W';
    assert_ne!(files_hash(&byte), held);

    let mut name = files.clone();
    name[1].0 = "Form2.frm".to_owned();
    assert_ne!(files_hash(&name), held);
}

/// The order in which the files come does not change the hash.
#[test]
fn the_order_of_the_files_does_not_change_the_hash() {
    let one = [file("A.vbp", b"1"), file("B.bas", b"2")];
    let two = [file("B.bas", b"2"), file("A.vbp", b"1")];
    assert_eq!(files_hash(&one), files_hash(&two));
}

/// The length keeps the boundary between two files. Without it, one file
/// that holds the name and the bytes of a second file would give the same
/// stream of bytes as the two files.
#[test]
fn the_length_keeps_the_boundary_between_two_files() {
    let one = [file("a", b"1b\x002")];
    let two = [file("a", b"1"), file("b", b"2")];
    assert_ne!(files_hash(&one), files_hash(&two));
}

/// The filter keeps the five kinds of file that VB6 reads, in any case, and
/// drops the JSON report. A changed report therefore does not change the
/// hash.
#[test]
fn build_files_keeps_the_five_extensions_and_drops_the_report() {
    let files = vec![
        file("P.vbp", b"1"),
        file("Form1.FRM", b"2"),
        file("Form1.frx", b"3"),
        file("Module1.Bas", b"4"),
        file("Class1.cls", b"5"),
        file("P.report.json", b"{}"),
        file("P.vbw", b"6"),
    ];
    let kept: Vec<String> = build_files(files.clone())
        .into_iter()
        .map(|(name, _bytes)| name)
        .collect();
    assert_eq!(
        kept,
        [
            "P.vbp",
            "Form1.FRM",
            "Form1.frx",
            "Module1.Bas",
            "Class1.cls"
        ]
    );

    let mut other_report = files;
    other_report[5].1 = b"{\"changed\": true}".to_vec();
    assert_eq!(
        files_hash(&build_files(other_report)),
        files_hash(&build_files(vec![
            file("P.vbp", b"1"),
            file("Form1.FRM", b"2"),
            file("Form1.frx", b"3"),
            file("Module1.Bas", b"4"),
            file("Class1.cls", b"5"),
            file("P.report.json", b"{}"),
        ]))
    );
}

/// The files of a real program are only files that VB6 reads, and one of
/// them is the project file. The corpus holds 44 programs, and none of them
/// is in a directory named `fetched`.
#[test]
fn the_files_of_a_corpus_program_are_the_files_that_vb6_reads() {
    let root = corpus_root();
    let exes = executables(&root).unwrap();
    assert_eq!(exes.len(), shared::EXPECTED_PROGRAM_COUNT);
    let exe = exes
        .iter()
        .find(|exe| {
            program_key(exe, &root)
                .unwrap()
                .ends_with("LockWorkStation.exe")
        })
        .expect("the corpus holds LockWorkStation.exe");

    let files = project_files(&std::fs::read(exe).unwrap()).unwrap();
    assert!(!files.is_empty());
    assert_eq!(
        files
            .iter()
            .filter(|(name, _bytes)| name.to_ascii_lowercase().ends_with(".vbp"))
            .count(),
        1,
        "{:?}",
        files.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    for (name, _bytes) in &files {
        assert!(!name.ends_with(".json"), "{name}");
    }
}

/// The walk skips each directory named `fetched`, at any depth, and keeps an
/// executable whose extension is in upper case. The directory tree is built
/// here, in the temporary directory, and removed at the end.
#[test]
fn the_walk_skips_each_directory_named_fetched() {
    let root =
        std::env::temp_dir().join(format!("deform6-build-record-walk-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("kept/deeper")).unwrap();
    std::fs::create_dir_all(root.join("fetched")).unwrap();
    std::fs::create_dir_all(root.join("kept/fetched")).unwrap();
    for name in [
        "kept/A.exe",
        "kept/deeper/B.EXE",
        "fetched/C.exe",
        "kept/fetched/D.exe",
        "kept/E.txt",
    ] {
        std::fs::write(root.join(name), b"MZ").unwrap();
    }

    let found = executables(&root);
    std::fs::remove_dir_all(&root).unwrap();
    let keys: Vec<String> = found
        .unwrap()
        .iter()
        .map(|exe| program_key(exe, &root).unwrap())
        .collect();
    assert_eq!(keys, ["kept/A.exe", "kept/deeper/B.EXE"]);
}

/// The committed record, read once.
fn committed() -> BuildRecord {
    let path = builds_toml_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    parse(&text).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// The committed record is the exact text that `render` writes from its own
/// values, so an import on a clean tree changes no line.
#[test]
fn the_committed_record_is_the_text_that_render_writes() {
    let path = builds_toml_path();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(render(&committed()), text, "{}", path.display());
}

/// The record names each corpus program, and no other program.
#[test]
fn the_record_names_each_corpus_program_and_no_other() {
    let root = corpus_root();
    let keys: Vec<String> = executables(&root)
        .unwrap()
        .iter()
        .map(|exe| program_key(exe, &root).unwrap())
        .collect();
    let record = committed();
    let held: Vec<&String> = record.programs.keys().collect();
    assert_eq!(held, keys.iter().collect::<Vec<_>>());
    assert_eq!(held.len(), shared::EXPECTED_PROGRAM_COUNT);
}

/// Each program's files are the files that the host built. A program whose
/// files changed is `STALE`, and the failure names each such program.
#[test]
fn each_result_covers_the_files_that_deform6_writes_now() {
    let root = corpus_root();
    let record = committed();
    let mut stale = Vec::new();
    for exe in executables(&root).unwrap() {
        let key = program_key(&exe, &root).unwrap();
        let files = project_files(&std::fs::read(&exe).unwrap()).unwrap();
        let now = files_hash(&files);
        let Some(program) = record.programs.get(&key) else {
            continue;
        };
        if program.files != now {
            stale.push(format!(
                "  {key}: the record holds {}, and DeForm6 now writes {now}",
                program.files
            ));
        }
    }
    assert!(
        stale.is_empty(),
        "STALE: DeForm6 writes different files for {} programs than the Windows host built:\n{}\n\
         Build them again:\n  1. cargo run -p xtask -- export-builds <dir>\n  2. On the Windows \
         host, run build.bat in <dir>.\n  3. cargo run -p xtask -- import-builds <dir>",
        stale.len(),
        stale.join("\n")
    );
}

/// A side that failed, or that VB6 built with load errors, holds the lines
/// that VB6 wrote, and no line holds a path with a drive letter of the host.
#[test]
fn a_side_with_errors_holds_its_messages_and_no_host_path() {
    for (key, program) in &committed().programs {
        for (side, result) in [
            ("original", &program.original),
            ("extracted", &program.extracted),
        ] {
            if matches!(
                result.outcome,
                Outcome::Failed | Outcome::BuiltWithLoadErrors
            ) {
                assert!(!result.messages.is_empty(), "{key} {side}");
            }
            for message in &result.messages {
                assert!(!message.contains(":\\"), "{key} {side}: {message}");
            }
        }
    }
}
