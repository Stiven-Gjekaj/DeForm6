#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The written procedure that turns a crash the fuzzer finds into a test
//! case, and the stable replay that runs every input this procedure has
//! already added.
//!
//! Read this file, do not run it, the moment a `cargo fuzz run` ends with
//! a crash. The steps:
//!
//! 1. A fuzz run that ends writes its input into the artifacts directory
//!    for the target under the fuzz crate, and names the path in its own
//!    output.
//! 2. Read the file before you copy it. Confirm it holds fuzzer generated
//!    bytes only. This is the one manual verification `05-VALIDATION.md`
//!    records; no test in this repository can make that decision.
//! 3. Compare its bytes against every file already in
//!    `crates/deform6/tests/regressions/`. Stop if one matches: a
//!    duplicate replays the same path twice and costs time without adding
//!    coverage.
//! 4. Copy it into `crates/deform6/tests/regressions/` under a name that
//!    says what it breaks, using lower case words joined by hyphens, and
//!    give it the same extension the seed uses.
//! 5. Run this test and watch it end the process. That proves the input
//!    still reproduces on the pinned stable toolchain, not only under a
//!    nightly sanitizer build.
//! 6. Fix the reader.
//! 7. Run this test again and watch it pass.
//! 8. Commit the input and the fix together, in one commit. `AGENTS.md`
//!    asks for the code and its tests in the same commit.
//!
//! # What the one committed seed proves, and what it does not
//!
//! `gui-table-overcount-4k.bin` is `support::hostile::gui_table_overcount_4k`'s
//! own output, committed byte for byte; the provenance test below proves
//! the equality, so this claim is checkable and not merely asserted here.
//! Its job is narrower than its byte layout suggests. Run through
//! `deform6::inspect`, in both modes, it refuses at `runtime_of` with
//! `Refusal::NoVbRuntime`, before the read ever reaches `header_region`,
//! `VbHeader::read`, or `GuiTable::walk`: the image carries no import
//! directory, so no Visual Basic runtime name is there for `runtime_of` to
//! find. Measured this session, both calls give the same refusal:
//!
//! ```text
//! Strict:  Err(NoVbRuntime { dot_net: false })
//! Salvage: Err(NoVbRuntime { dot_net: false })
//! ```
//!
//! This seed therefore guards the earliest refusal path in the reader, the
//! one `runtime_of` gives, and not the GUI table bound check its own name
//! evokes. The `wFormCount = 0xFFFF` case, an implausible declared form
//! count against a region that can hold only one entry, is already
//! covered by `gui_table_refuses_an_implausible_form_count` in
//! `crates/deform6/src/vb/gui.rs`, which plan 05-02 added: that test
//! builds the same PE and section shape and calls `GuiTable::walk`
//! directly with a `VbHeader` value it holds in memory, bypassing
//! `deform6::inspect` entirely, which is the only way to reach that
//! defect with this image. Do not read this file's replay as a second
//! proof of the GUI table bound check. This seed's actual job is two
//! narrower things: it makes the count assertion below true from the
//! commit that introduces it, so the directory is never empty in this
//! project's history, and it gives the procedure above a worked, checked
//! example to point at.

use std::path::{Path, PathBuf};

use deform6::inspect;
use deform6::journal::Mode;
use deform6::vb::opcodes::OpcodeTable;

#[path = "support/mod.rs"]
#[allow(
    dead_code,
    reason = "cargo compiles this shared module into every test binary and each binary uses a \
              different subset"
)]
mod support;

use support::hostile::gui_table_overcount_4k;

/// The minimum number of files `crates/deform6/tests/regressions/` must
/// hold.
///
/// A loop over an empty directory passes and proves nothing, and roadmap
/// success criterion 2 requires that an empty directory fail this test.
/// The directory must never be emptied below this count; the seed this
/// file's own provenance test checks arrives in the same commit as this
/// harness so the directory is never empty in the project's history.
const MINIMUM_REGRESSION_INPUTS: usize = 1;

/// Gives the directory that holds the committed regression inputs.
fn regressions_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/regressions")
}

/// Walks `crates/deform6/tests/regressions/` recursively and gives every
/// file it holds, sorted.
///
/// This is the same recursive `read_dir` shape `corpus_sweep.rs` walks
/// the corpus with, with its extension filter dropped: a regression input
/// carries no fixed extension, and every file in this directory is an
/// input, never a project file or a stray reference.
///
/// The result is sorted for a stated reason: an unsorted directory read
/// gives a different order on a different file system, so a failure would
/// name a different first file on a different machine, and two people
/// looking at the same failing run would be looking at two different
/// inputs.
fn regression_inputs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(&regressions_root(), &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("reading an entry of {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Roadmap success criterion 1 and 2: every committed regression input
/// replays through both modes on the pinned stable toolchain, and an
/// empty directory fails this test.
///
/// Neither call's result variant is asserted. Every one of these inputs
/// is known bad; a refusal is the correct answer for most of them, and an
/// assertion on the variant would fail the moment a later plan improves
/// the reader. The assertion this test makes is that the process is still
/// running afterwards: the release profile aborts on panic, so a panic
/// ends it, and `cargo test` fails loudly the moment one iteration aborts
/// the process instead of returning. Each file name prints before its own
/// replay, so a process that ends names the input in the log.
#[test]
fn every_regression_input_replays_in_both_modes_without_ending_the_process() {
    let files = regression_inputs();
    assert!(
        files.len() >= MINIMUM_REGRESSION_INPUTS,
        "crates/deform6/tests/regressions/ holds {} files, wanted at least \
         {MINIMUM_REGRESSION_INPUTS}. This directory must never be emptied: a loop over an \
         empty directory passes while proving nothing.",
        files.len()
    );

    let opcode_table = OpcodeTable::builtin();
    for path in &files {
        println!("replaying {}", path.display());
        let data =
            std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));

        if let Ok(report) = inspect(&data, &opcode_table, Mode::Salvage) {
            // The writer is the second entry point a hostile file reaches,
            // once the reader has accepted it.
            let _ = deform6::write::project(&report, &data, Mode::Salvage);
        }
        let _ = inspect(&data, &opcode_table, Mode::Strict);
    }
}

/// Proves the one file this fuzzer never produced is this repository's
/// own output, so the rule `AGENTS.md` states on a fixture calculated
/// from a third party file is checkable and not merely asserted in prose.
#[test]
fn the_committed_seed_equals_the_builder_that_produced_it() {
    let seed_path = regressions_root().join("gui-table-overcount-4k.bin");
    let committed = std::fs::read(&seed_path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", seed_path.display()));
    assert_eq!(
        committed,
        gui_table_overcount_4k(),
        "the committed seed no longer equals support::hostile::gui_table_overcount_4k's own \
         output; its provenance claim is no longer checkable"
    );
}

/// Two identical inputs replay the same path twice and cost time without
/// adding coverage. The written procedure above tells a person to check
/// before copying; this test holds the same rule for whatever already
/// made it past that check.
#[test]
fn no_two_regression_inputs_hold_the_same_bytes() {
    let files = regression_inputs();
    let mut contents: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for path in &files {
        let bytes =
            std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
        contents.push((path.clone(), bytes));
    }
    for i in 0..contents.len() {
        for j in (i + 1)..contents.len() {
            let (path_a, bytes_a) = &contents[i];
            let (path_b, bytes_b) = &contents[j];
            assert_ne!(
                bytes_a,
                bytes_b,
                "{} and {} hold the same bytes; one of them replays a path the other one \
                 already covers",
                path_a.display(),
                path_b.display()
            );
        }
    }
}

/// A zero byte input is replayed like any other input and must not end
/// the process. The state a test needs is built inside the test: this
/// input is never committed to `crates/deform6/tests/regressions/`.
#[test]
fn a_zero_byte_input_gives_not_pe_in_both_modes() {
    let opcode_table = OpcodeTable::builtin();
    let data: Vec<u8> = Vec::new();
    assert_eq!(
        inspect(&data, &opcode_table, Mode::Strict),
        Err(deform6::Refusal::NotPe)
    );
    assert_eq!(
        inspect(&data, &opcode_table, Mode::Salvage),
        Err(deform6::Refusal::NotPe)
    );
}
