#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The P-code record, `tests/pcode.toml`, the P-code corpus,
//! `corpus-pcode/`, and the gate that holds the tree to both.
//!
//! `pcode_record/shared.rs` holds the text and the parser of the record, and
//! `crates/xtask` compiles the same file to write the record. This file tests
//! that shared code, and then holds the tree to the record.
//!
//! # What the gate proves
//!
//! Each binary in `corpus-pcode/` is the binary that the record names, and
//! the record names a binary for each program that built, and for no other.
//! The source that each record entry holds is the source that the corpus
//! holds now: when a corpus file changes, the gate fails with `STALE` until
//! the host builds the program again. `ProjectInfo.lpNativeCode` is 0 in each
//! binary, so DeForm6 reads each one as P-code. No test runs the Visual
//! Basic 6 IDE: this machine cannot.

#[path = "build_record/shared.rs"]
#[allow(
    dead_code,
    reason = "this module is embedded in several binaries, and this one uses only the hash, \
              the outcome and the parse helpers"
)]
mod build_record;

#[path = "pcode_record/shared.rs"]
#[allow(
    dead_code,
    reason = "this module is embedded in two binaries, this test target and crates/xtask, and \
              each uses a different part of it"
)]
mod shared;

#[path = "support/mod.rs"]
#[allow(
    dead_code,
    reason = "cargo compiles this shared module into every test binary and this one uses only \
              the vbp reader"
)]
mod support;

use std::collections::{BTreeMap, BTreeSet};

use build_record::Outcome;
use deform6::read::pe::PeImage;
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::project::{CompileMode, ProjectInfo};
use shared::{PcodeBuild, PcodeRecord, exe_hash, parse, pcode_root, pcode_toml_path, render};
use support::vbp;

/// A hash of the right shape, built here from one hexadecimal digit.
fn hash(digit: char) -> String {
    format!("sha256:{}", digit.to_string().repeat(64))
}

/// A record with a program that built, one that failed with a message that
/// holds a quote and a backslash, and one that did not run, under a key
/// with a space. Built here and not read from a file.
fn synthetic_record() -> PcodeRecord {
    let mut programs = BTreeMap::new();
    programs.insert(
        "vb6-code/Part 3 - A/A.exe".to_owned(),
        PcodeBuild {
            source: hash('a'),
            result: Outcome::Built,
            exe: Some(hash('b')),
            messages: Vec::new(),
        },
    );
    programs.insert(
        "public-domain/B/B.exe".to_owned(),
        PcodeBuild {
            source: hash('c'),
            result: Outcome::Failed,
            exe: None,
            messages: vec![r#"File not found: 'p01\"x".cls'"#.to_owned()],
        },
    );
    programs.insert(
        "public-domain/C/C.exe".to_owned(),
        PcodeBuild {
            source: hash('d'),
            result: Outcome::NotRun,
            exe: None,
            messages: Vec::new(),
        },
    );
    PcodeRecord {
        windows: "Microsoft Windows XP [Version 5.1.2600]".to_owned(),
        vb6: "02/23/2004 12:00 AM 1,895,424 VB6.EXE".to_owned(),
        compilation_type: "CompilationType=-1".to_owned(),
        programs,
    }
}

/// A record that `render` writes reads back as the same record.
#[test]
fn a_record_that_render_writes_reads_back_the_same() {
    let record = synthetic_record();
    let text = render(&record);
    assert_eq!(parse(&text).unwrap(), record, "{text}");
}

/// An `exe` goes with a program that built, and only with one.
#[test]
fn the_parser_refuses_an_exe_that_does_not_go_with_the_result() {
    let text = render(&synthetic_record());

    let built_without_exe = text.replacen(&format!("exe = \"{}\"\n", hash('b')), "", 1);
    assert_ne!(built_without_exe, text);
    let err = parse(&built_without_exe).unwrap_err();
    assert!(
        err.contains("an exe goes with a program that built"),
        "{err}"
    );

    let failed_with_exe = text.replacen(
        "result = \"failed\"\n",
        &format!("result = \"failed\"\nexe = \"{}\"\n", hash('e')),
        1,
    );
    assert_ne!(failed_with_exe, text);
    let err = parse(&failed_with_exe).unwrap_err();
    assert!(
        err.contains("an exe goes with a program that built"),
        "{err}"
    );
}

/// The parser refuses messages on a program that built, a hash of another
/// shape, and an entry that the record does not define.
#[test]
fn the_parser_refuses_messages_on_a_build_a_bad_hash_and_an_unknown_entry() {
    let text = render(&synthetic_record());

    let built_with_messages = text.replacen(
        "result = \"built\"\n",
        "result = \"built\"\nmessages = [\"x\"]\n",
        1,
    );
    assert_ne!(built_with_messages, text);
    let err = parse(&built_with_messages).unwrap_err();
    assert!(err.contains("built, and holds messages"), "{err}");

    let bad_hash = text.replacen(&hash('a'), &hash('A'), 1);
    assert_ne!(bad_hash, text);
    assert!(parse(&bad_hash).is_err());

    let unknown = text.replacen("[environment]\n", "[environment]\nhost = \"x\"\n", 1);
    assert_ne!(unknown, text);
    let err = parse(&unknown).unwrap_err();
    assert!(err.contains("\"host\""), "{err}");
}

/// The hash of an executable is the SHA-256 of its bytes: the value that
/// FIPS 180-2 gives for the text `abc`.
#[test]
fn the_exe_hash_is_the_sha256_of_the_bytes() {
    assert_eq!(
        exe_hash(b"abc"),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

// --- The gate: the committed record and the committed binaries -----------

/// The committed record, parsed.
fn committed() -> PcodeRecord {
    let path = pcode_toml_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    parse(&text).unwrap_or_else(|err| panic!("{err}"))
}

/// The keys of the executables under `root`.
fn keys_under(root: &std::path::Path) -> BTreeSet<String> {
    build_record::executables(root)
        .unwrap()
        .iter()
        .map(|exe| build_record::program_key(exe, root).unwrap())
        .collect()
}

/// The committed file is the text that `render` writes for the record that
/// it holds, so a rewrite on a clean tree gives no diff.
#[test]
fn the_committed_record_is_the_text_that_render_writes() {
    let path = pcode_toml_path();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(render(&parse(&text).unwrap()), text);
}

/// The record names each corpus program, and no other.
#[test]
fn the_record_names_each_corpus_program_and_no_other() {
    let held: BTreeSet<String> = committed().programs.keys().cloned().collect();
    assert_eq!(held.len(), build_record::EXPECTED_PROGRAM_COUNT);
    assert_eq!(held, keys_under(&build_record::corpus_root()));
}

/// The source of each program is the source that the corpus holds now.
#[test]
fn each_source_is_the_source_that_the_corpus_holds_now() {
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let mut stale = Vec::new();
    for (key, program) in &committed().programs {
        let project = vbp::select_project_file(&root.join(key), &projects)
            .unwrap_or_else(|err| panic!("{key}: {err}"));
        let now = build_record::files_hash(
            &build_record::source_files(project.parent().unwrap()).unwrap(),
        );
        if now != program.source {
            stale.push(format!(
                "  {key}: the record holds {}, and the corpus now gives {now}",
                program.source
            ));
        }
    }
    assert!(
        stale.is_empty(),
        "STALE: the source of {} programs changed since the host built them:\n{}\nBuild them \
         again:\n  1. cargo run -p xtask -- export-pcode <dir>\n  2. On the Windows host, run \
         build.bat in <dir>, then sendpcode.bat.\n  3. cargo run -p xtask -- import-pcode \
         --capture <file> <dir>",
        stale.len(),
        stale.join("\n")
    );
}

/// Each program that built has its binary at its key under
/// `corpus-pcode/`, with the recorded hash. `corpus-pcode/` holds no other
/// executable.
#[test]
fn each_build_is_the_committed_binary_and_no_other() {
    let root = pcode_root();
    let mut built = BTreeSet::new();
    for (key, program) in &committed().programs {
        let Some(exe) = &program.exe else {
            continue;
        };
        built.insert(key.clone());
        let bytes = std::fs::read(root.join(key)).unwrap_or_else(|err| panic!("{key}: {err}"));
        assert_eq!(&exe_hash(&bytes), exe, "{key}");
    }
    assert!(!built.is_empty());
    assert_eq!(keys_under(&root), built);
}

/// The measurement of phase 9: `ProjectInfo.lpNativeCode` is 0 in each
/// committed binary, so DeForm6 reads each one as P-code.
#[test]
fn each_committed_binary_is_p_code() {
    let exes = build_record::executables(&pcode_root()).unwrap();
    assert!(!exes.is_empty());
    for exe in &exes {
        let bytes = std::fs::read(exe).unwrap();
        let pe = PeImage::parse(&bytes).unwrap_or_else(|err| panic!("{}: {err}", exe.display()));
        let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
        let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
        assert_eq!(info.lp_native_code, 0, "{}", exe.display());
        assert_eq!(info.mode(), CompileMode::PCode, "{}", exe.display());
    }
}

/// A program that did not build cleanly holds the lines that VB6 wrote, and
/// no line holds a path with a drive letter of the host.
#[test]
fn a_program_that_did_not_build_holds_its_messages_and_no_host_path() {
    for (key, program) in &committed().programs {
        if matches!(
            program.result,
            Outcome::Failed | Outcome::BuiltWithLoadErrors
        ) {
            assert!(!program.messages.is_empty(), "{key}");
        }
        for message in &program.messages {
            assert!(!message.contains(":\\"), "{key}: {message}");
        }
    }
}
