#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The P-code record, `tests/pcode.toml`.
//!
//! `pcode_record/shared.rs` holds the text and the parser of the record, and
//! `crates/xtask` compiles the same file to write the record. This file tests
//! that shared code.

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

use std::collections::BTreeMap;

use build_record::Outcome;
use shared::{PcodeBuild, PcodeRecord, exe_hash, parse, render};

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
