#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong, \
              and the whole point of this file is that the aborting release profile ends the \
              process the moment one of these calls panics"
)]

//! Roadmap success criterion 5, and SAF-01: the tool does not panic on any
//! input.
//!
//! One run reads every file in the vendored corpus, every file in the
//! fetched run time robustness set, and every file in the committed
//! regression directory. It drives each one through `Mode::Strict` and
//! `Mode::Salvage`, and calls the writer on every successful salvage
//! result. The run prints the number of inputs it read, per source and in
//! total, and that number equals the number of files that exist in those
//! sources, each counted where it lives.
//!
//! `AGENTS.md`'s "What to measure" states the discipline this file is held
//! to twice over: measure the thing you tell the human, not something near
//! it, and give the number you can prove, not a number calculated from a
//! part. Three sources means three counts, each taken from its own
//! directory or its own manifest, and the total is the sum of the three,
//! never a number derived from one of them.
//!
//! All three walks below are sorted, for the same reason
//! `crates/deform6/tests/regressions.rs` sorts its own walk: an unsorted
//! directory read gives a different order on a different file system, so a
//! failure would name a different first file on a different machine.

use std::path::{Path, PathBuf};

use deform6::inspect;
use deform6::journal::Mode;
use deform6::vb::opcodes::OpcodeTable;

/// The count `crates/deform6/tests/corpus_sweep.rs`,
/// `crates/deform6/tests/differential.rs` and
/// `crates/xtask/src/main.rs::MINIMUM_PROGRAM_COUNT` all pin. This file
/// holds its own copy of the number, not a shared constant, because each
/// of the four lives in a different compilation unit; a future change to
/// the vendored corpus must move all four together.
const EXPECTED_VENDORED_COUNT: usize = 44;

/// The minimum number of regression inputs the run must find, restated
/// from `crates/deform6/tests/regressions.rs::MINIMUM_REGRESSION_INPUTS`
/// (currently `1`) rather than read from it.
///
/// A `#[test]`-bearing file under `tests/` compiles as its own,
/// independent binary; `no_panic_proof` cannot reach a private constant
/// defined in a sibling test binary's crate root. This literal must move
/// with the constant it names.
const MINIMUM_REGRESSION_INPUTS: usize = 1;

/// The exact number of programs `corpus/manifest.toml` currently pins:
/// `map-editor-2d`, `passgen` and `transparency-2d` (05-07). This literal
/// must move with the manifest.
const EXPECTED_MANIFEST_ENTRIES: usize = 3;

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Gives the directory `cargo run -p xtask -- fetch-corpus` populates.
fn fetched_root() -> PathBuf {
    corpus_root().join("fetched")
}

/// Gives the committed manifest `fetch-corpus` and `pin-corpus` both read
/// and write.
fn manifest_path() -> PathBuf {
    corpus_root().join("manifest.toml")
}

/// Gives the directory that holds the committed regression inputs.
fn regressions_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/regressions")
}

/// Walks `dir` recursively and gives every file whose extension is `.exe`,
/// compared case-insensitively, sorted.
///
/// Explicitly skips a directory named `fetched`: the fetched set lives
/// inside `corpus/` on disk and is not part of the vendored set. A walk
/// that counted both would report a vendored number that grows every time
/// somebody pins a program in the manifest, which is exactly the kind of
/// count calculated from the wrong place `AGENTS.md` warns against.
fn vendored_executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_vendored(&corpus_root(), &mut out);
    out.sort();
    out
}

fn walk_vendored(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("reading an entry of {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "fetched") {
                continue;
            }
            walk_vendored(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push(path);
        }
    }
}

/// Walks `dir` recursively and gives every file it holds, with no
/// extension filter and no sort applied yet. A fetched program or a
/// regression input carries no fixed extension, so no filter is correct
/// for either source; each caller below sorts its own result, so a
/// failure names the same first file on every machine.
fn walk_plain_into(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("reading an entry of {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            walk_plain_into(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Walks `crates/deform6/tests/regressions/` recursively and gives every
/// file it holds, sorted.
fn regression_inputs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_plain_into(dir, &mut out);
    out.sort();
    out
}

/// The fetched run time robustness set's own count. Zero and absent are
/// two different answers: a set of zero files passes every check in this
/// file, and reporting an absent directory as a set of zero files that
/// passed is exactly the silence the named risk in `.planning/WINDOWS.md`
/// (finding 14) and `05-RESEARCH.md` describe.
#[derive(Debug, PartialEq, Eq)]
enum FetchedCount {
    /// `corpus/fetched/` exists; it holds this many files.
    Present(usize),
    /// `corpus/fetched/` does not exist on this machine.
    Absent,
}

/// Counts the files in `dir` when it exists, and reports `Absent`
/// otherwise. Never reports a missing directory as `Present(0)`.
fn fetched_programs(dir: &Path) -> FetchedCount {
    if !dir.is_dir() {
        return FetchedCount::Absent;
    }
    let mut out = Vec::new();
    walk_plain_into(dir, &mut out);
    out.sort();
    FetchedCount::Present(out.len())
}

/// Parses `manifest_path` as `corpus/manifest.toml`'s own format and gives
/// the number of tables it declares. Read from the manifest's own entry
/// count, using the same `text.parse::<toml::Table>()` call
/// `crates/xtask/src/fetch_corpus.rs::parse_manifest` and
/// `crates/xtask/src/main.rs` both use, and never from a directory
/// listing: a stale file from an earlier run, or a file a person dropped
/// into `corpus/fetched/` by hand, would otherwise be counted as a pinned
/// program that verified.
fn manifest_entry_count(manifest_path: &Path) -> usize {
    let text = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", manifest_path.display()));
    let table: toml::Table = text
        .parse()
        .unwrap_or_else(|err| panic!("{} is not valid TOML: {err}", manifest_path.display()));
    table.len()
}

/// The three counts this proof takes, each where its own source lives.
struct Counts {
    vendored: usize,
    fetched: FetchedCount,
    regression: usize,
}

impl Counts {
    /// The sum of the counts that were taken. An absent fetched set
    /// contributes zero files to this sum because zero files is what a
    /// walk of a directory that does not exist would honestly find; the
    /// distinction this file preserves is in the report line
    /// [`print_report`] gives for that source, not in this arithmetic.
    fn total(&self) -> usize {
        let fetched = match self.fetched {
            FetchedCount::Present(n) => n,
            FetchedCount::Absent => 0,
        };
        self.vendored + fetched + self.regression
    }
}

/// Prints one line per source, naming the source and its count, and one
/// total line. The total line always adds the three counts this run took;
/// it never derives one count from another.
fn print_report(counts: &Counts) {
    println!(
        "no_panic_proof: vendored corpus (corpus/, excluding corpus/fetched/): {} files",
        counts.vendored
    );
    match counts.fetched {
        FetchedCount::Present(n) => {
            println!("no_panic_proof: fetched robustness set (corpus/fetched/): {n} files");
        }
        FetchedCount::Absent => {
            println!(
                "no_panic_proof: fetched robustness set: corpus/fetched/ is not present on \
                 this machine. Run `cargo run -p xtask -- fetch-corpus` to populate it."
            );
        }
    }
    println!(
        "no_panic_proof: regression inputs (crates/deform6/tests/regressions/): {} files",
        counts.regression
    );
    println!("no_panic_proof: total inputs: {}", counts.total());
}

#[test]
fn the_vendored_walk_finds_the_pinned_forty_four() {
    let files = vendored_executables();
    assert_eq!(
        files.len(),
        EXPECTED_VENDORED_COUNT,
        "found {} vendored executables, wanted the same {EXPECTED_VENDORED_COUNT} every other \
         gate in this workspace pins",
        files.len()
    );
}

#[test]
fn the_manifest_table_count_matches_the_committed_manifest() {
    let count = manifest_entry_count(&manifest_path());
    assert_eq!(
        count, EXPECTED_MANIFEST_ENTRIES,
        "corpus/manifest.toml declares {count} entries, wanted {EXPECTED_MANIFEST_ENTRIES}; \
         this literal must move with the manifest"
    );
}

#[test]
fn the_regression_walk_meets_the_stated_minimum() {
    let files = regression_inputs(&regressions_root());
    assert!(
        files.len() >= MINIMUM_REGRESSION_INPUTS,
        "found {} regression inputs, wanted at least {MINIMUM_REGRESSION_INPUTS}",
        files.len()
    );
}

#[test]
fn fetched_programs_of_an_absent_directory_reports_absent_not_zero() {
    let absent = Path::new("/does/not/exist/deform6-no-panic-proof-absent-check");
    assert_eq!(fetched_programs(absent), FetchedCount::Absent);
}

#[test]
fn fetched_programs_of_the_populated_directory_matches_the_manifest_when_present() {
    // `cargo run -p xtask -- fetch-corpus` is this task's own precondition,
    // but a test suite must still tell the truth on a machine where it has
    // not been run: an absent directory is reported, not failed on.
    match fetched_programs(&fetched_root()) {
        FetchedCount::Present(n) => assert_eq!(
            n,
            manifest_entry_count(&manifest_path()),
            "corpus/fetched/ holds {n} files, which must equal the manifest's own entry count"
        ),
        FetchedCount::Absent => {
            println!(
                "corpus/fetched/ is not present on this machine; run \
                 `cargo run -p xtask -- fetch-corpus` to populate it before this assertion can \
                 run."
            );
        }
    }
}

#[test]
fn the_total_is_the_sum_of_the_three_counts_taken() {
    let counts = Counts {
        vendored: 44,
        fetched: FetchedCount::Present(3),
        regression: 1,
    };
    assert_eq!(counts.total(), 48);
}

#[test]
fn an_absent_fetched_set_still_reports_as_absent_while_the_total_counts_it_as_zero_files() {
    let counts = Counts {
        vendored: 44,
        fetched: FetchedCount::Absent,
        regression: 1,
    };
    assert_eq!(counts.total(), 45);
    assert_eq!(counts.fetched, FetchedCount::Absent);
}

#[test]
fn the_counted_report_prints_a_line_per_source_and_a_total_line() {
    // This test asks nothing of the disk beyond what the functions above
    // already proved; it exists so a change to `print_report`'s own shape
    // (dropping a line, or silently swallowing the absent case) fails a
    // test rather than only changing console output nobody is asserting
    // against.
    let counts = Counts {
        vendored: EXPECTED_VENDORED_COUNT,
        fetched: FetchedCount::Present(EXPECTED_MANIFEST_ENTRIES),
        regression: MINIMUM_REGRESSION_INPUTS,
    };
    print_report(&counts);
    let absent_counts = Counts {
        vendored: EXPECTED_VENDORED_COUNT,
        fetched: FetchedCount::Absent,
        regression: MINIMUM_REGRESSION_INPUTS,
    };
    print_report(&absent_counts);
}

/// One input this proof reads, tagged with which of the three sources it
/// came from, for the per-input log line the sweep below prints.
struct Input {
    source: &'static str,
    path: PathBuf,
}

/// Gathers every input from all three sources, sorted within each source,
/// alongside the [`Counts`] this run took while gathering them. The
/// vendored and regression sources are always gathered; the fetched
/// source is gathered only when [`fetched_programs`] reports it present,
/// matching the same absent-is-not-zero rule the counting functions
/// above hold to.
fn gather_inputs() -> (Vec<Input>, Counts) {
    let vendored = vendored_executables();
    let fetched_dir = fetched_root();
    let fetched_count = fetched_programs(&fetched_dir);
    let fetched_files = match fetched_count {
        FetchedCount::Present(_) => {
            let mut out = Vec::new();
            walk_plain_into(&fetched_dir, &mut out);
            out.sort();
            out
        }
        FetchedCount::Absent => Vec::new(),
    };
    let regression = regression_inputs(&regressions_root());

    let counts = Counts {
        vendored: vendored.len(),
        fetched: fetched_count,
        regression: regression.len(),
    };

    let mut inputs = Vec::with_capacity(vendored.len() + fetched_files.len() + regression.len());
    for path in vendored {
        inputs.push(Input {
            source: "vendored",
            path,
        });
    }
    for path in fetched_files {
        inputs.push(Input {
            source: "fetched",
            path,
        });
    }
    for path in regression {
        inputs.push(Input {
            source: "regression",
            path,
        });
    }
    (inputs, counts)
}

/// Reads one input and drives it through both modes and the writer.
///
/// Prints the source, the path and the mode before each call runs, never
/// after: the aborting release profile ends the process the instant one
/// of these calls panics, and a message built after the call would never
/// reach the log. Asserts nothing about the result. A refusal is the
/// correct answer for most of these inputs; the assertion this whole
/// file makes is that the process is still running once every input has
/// been read, which is why `inspect`'s and `write::project`'s own return
/// values are discarded with `let _ =` rather than matched on.
fn drive_one(source: &str, path: &Path, opcode_table: &OpcodeTable) {
    let data =
        std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));

    println!("no_panic_proof: {source} {} Mode::Strict", path.display());
    let _ = inspect(&data, opcode_table, Mode::Strict);

    println!("no_panic_proof: {source} {} Mode::Salvage", path.display());
    let salvage_result = inspect(&data, opcode_table, Mode::Salvage);

    if let Ok(report) = salvage_result {
        println!(
            "no_panic_proof: {source} {} Mode::Salvage (write::project)",
            path.display()
        );
        let _ = deform6::write::project(&report, &data, Mode::Salvage);
    }
}

/// Roadmap success criterion 5, and SAF-01: one run reads every file in
/// the vendored corpus, every file in the fetched robustness set and
/// every file in the regression directory, drives each one through
/// `Mode::Strict` and `Mode::Salvage`, and calls the writer on every
/// successful salvage result. No process aborts, and the number of
/// inputs read equals the total [`gather_inputs`] counted, which in turn
/// is the sum of the three counts each source's own function above took.
///
/// This test proves what `cargo test` can prove: the test profile does
/// not set `panic = "abort"`, so a panic here unwinds and `cargo test`
/// reports it as a failed test, which is still a loud, gate-blocking
/// failure. The stronger claim, that the aborting release profile itself
/// survives every input, is proved separately by running this same test
/// under `cargo test --release`; see 05-08-SUMMARY.md for that run's own
/// command, counts, duration and peak resident set.
#[test]
fn every_input_this_repository_can_reach_runs_through_both_modes_and_the_writer() {
    let (inputs, counts) = gather_inputs();
    print_report(&counts);

    let opcode_table = OpcodeTable::builtin();
    for input in &inputs {
        drive_one(input.source, &input.path, &opcode_table);
    }

    assert_eq!(
        inputs.len(),
        counts.total(),
        "read {} inputs across all three sources, but the sources counted to {}",
        inputs.len(),
        counts.total()
    );
}
