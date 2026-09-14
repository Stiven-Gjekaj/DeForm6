#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! Audits the committed `.gitattributes` against the extension set the
//! writer actually emits.
//!
//! This file never rewrites `.gitattributes`. It reads the committed file
//! and checks two things against it: that every file extension
//! [`deform6::write::project`] emits, over one real corpus program, carries
//! a `binary` or `-text` rule, and that `*.frx` and `*.ctx`, the two
//! extensions the roadmap names for this plan, are both marked `binary`.
//! `corpus/vb6-code/Hidden-Markov-model/frmHMM.frx` is the recorded case
//! where a missing binary rule already cost one byte of a resource file to
//! git's own line ending normalisation.

use std::path::Path;

use deform6::journal::Mode;
use deform6::vb::opcodes::OpcodeTable;

/// The repository root, one level above `crates/deform6` and one level above
/// `crates`. `.gitattributes` lives there, alongside `Cargo.toml`.
fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The one corpus program this audit drives through the real write path.
fn corpus_exe() -> std::path::PathBuf {
    repo_root().join("corpus/public-domain/PassGen/PassGen.exe")
}

/// `deform6::write::project` also writes one JSON report file per run,
/// named `<project>.report.json`. That file is not a Visual Basic project
/// file: `.gitattributes` governs the line ending and binary treatment of
/// the files a VB6 project holds, and the JSON report is a diagnostic
/// artifact this tool writes beside that project, not a file the IDE reads
/// or the corpus commits. Excluding it here draws the same line
/// `tests/extract_structural.rs`'s own `is_report_file` helper already
/// draws.
fn is_report_file(name: &str) -> bool {
    name.ends_with(".report.json")
}

/// The lower case extension of a file name, read through [`Path::extension`]
/// rather than a hand rolled split on the last dot, so a name with more than
/// one dot is read the same way the standard library reads it everywhere
/// else in this codebase.
fn extension_of(name: &str) -> Option<String> {
    Path::new(name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
}

/// A line in `.gitattributes` names `extension` when it starts with a
/// `*.<extension>` pattern, followed by any amount of whitespace, followed
/// by an attribute token holding `binary` or `-text`. Whitespace is matched
/// by amount, not by one exact string: `AGENTS.md` names the trap of a
/// helper that looks for one space before an equals sign, and this file
/// must not repeat it.
fn gitattributes_names_extension(gitattributes: &str, extension: &str) -> bool {
    let pattern = format!("*.{extension}");
    gitattributes.lines().any(|line| {
        let Some(rest) = line.strip_prefix(&pattern) else {
            return false;
        };
        let attribute = rest.trim_start();
        attribute == "binary" || attribute.starts_with("-text") || attribute.starts_with("binary")
    })
}

#[test]
fn every_extension_the_writer_emits_is_named_in_gitattributes() {
    let exe = corpus_exe();
    let data = std::fs::read(&exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
    let table = OpcodeTable::builtin();
    let report = deform6::inspect(&data, &table, Mode::Strict)
        .unwrap_or_else(|err| panic!("inspect refused {}: {err}", exe.display()));
    let written = deform6::write::project(&report, &data, Mode::Strict)
        .unwrap_or_else(|err| panic!("write::project refused {}: {err}", exe.display()));

    let gitattributes_path = repo_root().join(".gitattributes");
    let gitattributes = std::fs::read_to_string(&gitattributes_path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", gitattributes_path.display()));

    let mut checked = 0;
    for file in &written.files {
        if is_report_file(&file.name) {
            continue;
        }
        let Some(extension) = extension_of(&file.name) else {
            panic!("{} carries no extension at all", file.name);
        };
        checked += 1;
        assert!(
            gitattributes_names_extension(&gitattributes, &extension),
            "{} has extension .{extension}, produced by write::project for {}, and \
             .gitattributes holds no binary or -text rule for it. Add one, and weaken no \
             existing rule.",
            file.name,
            exe.display()
        );
    }

    assert!(
        checked > 0,
        "this run produced no non-report file to check; the audit proved nothing"
    );
}

#[test]
fn frx_and_ctx_are_marked_binary() {
    let gitattributes_path = repo_root().join(".gitattributes");
    let gitattributes = std::fs::read_to_string(&gitattributes_path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", gitattributes_path.display()));

    for extension in ["frx", "ctx"] {
        let pattern = format!("*.{extension}");
        let marked_binary = gitattributes.lines().any(|line| {
            line.strip_prefix(&pattern)
                .is_some_and(|rest| rest.trim_start() == "binary")
        });
        assert!(
            marked_binary,
            "{pattern} must be marked binary in .gitattributes; \
             corpus/vb6-code/Hidden-Markov-model/frmHMM.frx is the recorded case where a missing \
             binary rule already cost one byte of a resource file"
        );
    }
}
