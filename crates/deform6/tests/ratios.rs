#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The pinned procedure recovery ratio: `tests/ratios.toml`, read and
//! checked against the corpus, per VER-05.
//!
//! This is the gate `tests/ratios.toml` exists to be. It parses the
//! committed file, computes the same two counts for every corpus program
//! through [`differential::program_counts`] (plan 02-08's own function,
//! never a second arithmetic -- `AGENTS.md`'s Measurement section is
//! explicit that two arithmetics eventually prove two different numbers),
//! and fails loudly in one of two directions when a pinned number and the
//! measured number disagree:
//!
//! - The pinned number is **above** the measured number: the pin claims
//!   more than the tool recovers, so something the pin expects is missing.
//!   That is [`REGRESSION`].
//! - The pinned number is **below** the measured number: the tool recovers
//!   more than the pin claims. That is [`MOVED_UP`].
//!
//! Both words describe what happened to **the tool**, never to the file.
//!
//! # Object recovery is not here
//!
//! Per D-11, object recovery is 44 of 44 (105 of 105 objects) with no
//! variance across the corpus, and `differential.rs` already asserts it as
//! a two-directional equality. A pin on it could never move, so it could
//! never fail, and `AGENTS.md` says a test that cannot fail is worse than
//! none. This file pins procedures, where the variance is real.
//!
//! # Where the two counts come from
//!
//! `#[path = "differential.rs"] mod differential;` below is the same
//! mechanism `differential.rs` documents on [`differential::ProgramCounts`]:
//! this file is compiled as its own crate by `cargo test`, and `#[path]`
//! recompiles `differential.rs`'s source directly into it as a submodule,
//! so `differential::program_counts` here is the literal same function
//! `differential.rs`'s own tests call, not a copy. `crates/xtask` reaches
//! the same function the same way (`#[path =
//! "../../deform6/tests/differential.rs"] mod differential;`), which is
//! the whole point of T-02-45: both the gate and the writer take their
//! counts from the one function plan 02-08 wrote, so they cannot silently
//! drift into two different arithmetics that happen to agree today.
//!
//! One side effect of that embedding: `differential.rs`'s own seven tests
//! are recompiled into this binary too, as `differential::*`, and `cargo
//! test --test ratios` runs them a second time (they already run once as
//! `--test differential`). This is not a second, independent proof; it is
//! the same tests re-executed because they physically live in the file
//! this one embeds. Documented here rather than left as a silent surprise
//! in the test count.

// `pub(crate)`: `crates/xtask` reaches `program_counts` and the rest
// through this exact module, `ratios::differential`, rather than a second,
// parallel `#[path]` embedding of its own -- one embedding of
// `differential.rs`, not two, so there is only ever one `ProgramCounts`
// type in this binary's build graph. Two independent embeddings of the
// same file compile as two distinct, incompatible types with the same
// name, which a second, parallel `#[path] mod differential;` in
// `crates/xtask` hit directly (E0308) before this was worked out.
//
// `#[allow(unused_imports, dead_code, ...)]`: this same module gets
// embedded into two different binaries -- this file's own `--test`
// target, where `differential.rs`'s own `#[test]` functions run and use
// every import, and `crates/xtask`'s plain, non-test binary, where those
// `#[test]` function bodies compile out entirely and the handful of
// imports they alone used (`HashMap`, `classify`, `support::source`)
// would otherwise warn as unused. The same shape `support/mod.rs`
// documents on its own module doc comment, one level up.
#[path = "differential.rs"]
#[allow(
    unused_imports,
    dead_code,
    reason = "differential.rs's own #[test] functions, and the imports only they use, compile \
              out entirely in a non-test embed (crates/xtask); the plain functions a non-test \
              embed needs, program_counts and recovered_objects, stay live either way"
)]
pub(crate) mod differential;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use deform6::read::pe::PeImage;
use differential::support::{source, vbp};
use differential::{ProgramCounts, program_counts};

/// The number of corpus programs this file pins. Asserted before anything
/// else runs, so a corpus file added or removed is a loud failure rather
/// than a silently shorter run.
const EXPECTED_PROGRAM_COUNT: usize = 44;

/// The pinned totals over all 44 programs, per plan 02-08's own
/// measurement (`02-08-SUMMARY.md`'s per-program table, reproduced by
/// `differential.rs`'s own corpus-wide test).
const EXPECTED_TOTAL_RECOVERED: u32 = 185;
const EXPECTED_TOTAL_DECLARED: u32 = 904;

/// Editing a pinned number **up** means the pin now claims more procedures
/// than the tool recovers: something the pin expects went missing. This
/// describes what happened to the tool, never to the file -- reasoning
/// from the file instead of from the tool gives the opposite pairing.
pub(crate) const REGRESSION: &str = "REGRESSION";

/// Editing a pinned number **down** means the tool now recovers more
/// procedures than the pin claims: the tool moved up. This describes what
/// happened to the tool, never to the file.
pub(crate) const MOVED_UP: &str = "MOVED UP";

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Gives the path of the pinned file: the workspace root's `tests/`
/// directory, not `crates/deform6/tests/`. Per `RESEARCH.md`'s own layout
/// note, neither this file nor `crates/xtask` should reach into the
/// other's own `tests/` directory, so the pin lives at the workspace root,
/// one level above every crate.
///
/// `pub(crate)`: `crates/xtask` reaches this through the same `#[path]`
/// embedding used for [`format_entry`], so the writer and the gate can
/// never disagree about which file either one means.
pub(crate) fn ratios_toml_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/ratios.toml")
}

/// The exact header block `tests/ratios.toml` carries above its first
/// entry, reproduced verbatim so `crates/xtask update-ratios` writes the
/// identical bytes on every run. This file owns the constant; `xtask`
/// reaches it through the same `#[path]` embedding as [`format_entry`].
pub(crate) const HEADER: &str = r#"# The pinned procedure recovery ratio, one entry per corpus program.
#
# Object recovery is NOT pinned here. It is 105 of 105 across the whole
# corpus with no variance (D-11): a pin on it could never move, so it could
# never fail, and AGENTS.md calls a test that cannot fail worse than none.
# `crates/deform6/tests/differential.rs` asserts object recovery as a
# two-directional equality instead. The ratio this file pins is over
# public procedures, where the variance is real: 24 distinct values from
# 0.00 to 0.71 across these 44 programs.
#
# Each entry is keyed by the executable's path relative to `corpus/`,
# quoted because a path holds spaces and forward slashes. Three corpus
# directories hold more than one compiled program, so a key built from the
# leaf directory name would collide; the full relative path never does.
#
# `recovered` is the count of public procedure names the library recovered.
# `declared` is the count of procedure slots the compiled binary declares
# over the same objects, including the slots the standard-module rule caps
# (a `.bas` module carries no procedure name array at all, so its
# procedures are name-less through the object table, not merely
# prototype-less; stating the cap here rather than excluding it from the
# denominator is the honest number, per CONTEXT.md's "What this phase must
# not do"). `ratio` is `recovered / declared`, rounded to two decimal
# places and recomputed from the two counts on every run: a hand-edited
# ratio that disagrees with its own counts fails the same as an edited
# count does.
#
# The totals over all 44 programs are 185 recovered and 904 declared.
#
# Rewrite this file with `cargo run -p xtask -- update-ratios`. Never edit
# the counts by hand; a rewrite on a clean tree produces no diff, and that
# is the check that the file and the tool agree.

"#;

/// One pinned entry: the two counts and the ratio text exactly as the file
/// holds it, so a hand-edited ratio can be compared as text against the
/// text a fresh computation would produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinnedEntry {
    pub(crate) recovered: u32,
    pub(crate) declared: u32,
    pub(crate) ratio_text: String,
}

/// Parses `tests/ratios.toml`'s exact shape: `#`-prefixed comment lines and
/// blank lines are skipped; a `["key"]` line opens an entry; `recovered =`,
/// `declared =` and `ratio =` lines fill it in.
///
/// This is a purpose-built reader for the one shape this file's own writer
/// produces, in the same spirit as `support/vbp.rs`: match the shape of
/// the file, not one form of it. It does not pull in the `toml` crate,
/// which `RESEARCH.md` adds for `crates/xtask` alone.
pub(crate) fn parse_ratios_toml(text: &str) -> BTreeMap<String, PinnedEntry> {
    let mut out = BTreeMap::new();
    let mut current_key: Option<String> = None;
    let mut recovered: Option<u32> = None;
    let mut declared: Option<u32> = None;
    let mut ratio_text: Option<String> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let (Some(key), Some(r), Some(d), Some(rt)) = (
                current_key.take(),
                recovered.take(),
                declared.take(),
                ratio_text.take(),
            ) {
                out.insert(
                    key,
                    PinnedEntry {
                        recovered: r,
                        declared: d,
                        ratio_text: rt,
                    },
                );
            }
            let path = ratios_toml_path();
            let first = inner.find('"').unwrap_or_else(|| {
                panic!("{}: a table header with no opening quote", path.display())
            });
            let last = inner.rfind('"').unwrap_or_else(|| {
                panic!("{}: a table header with no closing quote", path.display())
            });
            current_key = Some(inner[first + 1..last].to_owned());
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            panic!(
                "{}: a line with no `=` and no `[`: {line:?}",
                ratios_toml_path().display()
            );
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "recovered" => {
                recovered = Some(value.parse().unwrap_or_else(|err| {
                    panic!(
                        "{}: recovered {value:?}: {err}",
                        ratios_toml_path().display()
                    )
                }));
            }
            "declared" => {
                declared = Some(value.parse().unwrap_or_else(|err| {
                    panic!(
                        "{}: declared {value:?}: {err}",
                        ratios_toml_path().display()
                    )
                }));
            }
            "ratio" => {
                ratio_text = Some(value.to_owned());
            }
            other => panic!("{}: an unknown key {other:?}", ratios_toml_path().display()),
        }
    }
    if let (Some(key), Some(r), Some(d), Some(rt)) = (current_key, recovered, declared, ratio_text)
    {
        out.insert(
            key,
            PinnedEntry {
                recovered: r,
                declared: d,
                ratio_text: rt,
            },
        );
    }
    out
}

/// Reads and parses the committed `tests/ratios.toml`.
fn read_pinned() -> BTreeMap<String, PinnedEntry> {
    let text = std::fs::read_to_string(ratios_toml_path())
        .unwrap_or_else(|err| panic!("{}: {err}", ratios_toml_path().display()));
    parse_ratios_toml(&text)
}

/// `recovered / declared`, rounded to two decimal places. The one place
/// this ratio is computed, so the pinned column, the ratio-consistency
/// check and the `MOVED UP` paste block can never disagree by accident.
pub(crate) fn format_ratio(recovered: u32, declared: u32) -> String {
    let ratio = f64::from(recovered) / f64::from(declared);
    format!("{ratio:.2}")
}

/// The exact four-line block `crates/xtask update-ratios` writes for one
/// program, and the exact block a `MOVED UP` message prints to paste back
/// in. This file, `crates/deform6/tests/ratios.rs`, owns this function;
/// `crates/xtask` reaches it with `#[path =
/// "../../deform6/tests/ratios.rs"] mod ratios;`, the same embedding this
/// file itself uses to reach `differential.rs`. Task 3's own behaviour
/// four ("the block the `MOVED UP` message prints is byte for byte the
/// block the command writes") holds by construction: there is exactly one
/// function that renders this shape.
pub(crate) fn format_entry(key: &str, recovered: u32, declared: u32) -> String {
    format!(
        "[\"{key}\"]\nrecovered = {recovered}\ndeclared = {declared}\nratio = {}\n",
        format_ratio(recovered, declared)
    )
}

/// One program's key, alongside the recovered object list, the declared
/// object list and the parsed image: everything [`program_counts`] and
/// [`declared_not_recovered_names`] need, read once per program.
struct Program {
    key: String,
    image_bytes: Vec<u8>,
}

/// Gives every corpus program's key (the executable's path relative to
/// `corpus/`, with the platform's own separator normalised to `/` so the
/// key never depends on which operating system built it) alongside the
/// bytes of the executable.
fn programs() -> Vec<Program> {
    let root = corpus_root();
    let mut out: Vec<Program> = vbp::executables()
        .into_iter()
        .map(|exe| {
            let rel = exe.strip_prefix(&root).unwrap_or_else(|err| {
                panic!("{}: not under {}: {err}", exe.display(), root.display())
            });
            let key = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let image_bytes = std::fs::read(&exe)
                .unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
            Program { key, image_bytes }
        })
        .collect();
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

/// Computes [`ProgramCounts`] for one already-read program, through
/// [`differential::program_counts`] alone.
fn counts_for(program: &Program, projects: &[PathBuf]) -> ProgramCounts {
    let image = PeImage::parse(&program.image_bytes)
        .unwrap_or_else(|err| panic!("{}: PeImage::parse: {err:?}", program.key));
    let recovered = differential::recovered_objects(&image);

    let exe_path = corpus_root().join(&program.key);
    let project_path = vbp::select_project_file(&exe_path, projects)
        .unwrap_or_else(|err| panic!("{}: {err}", program.key));
    let project = vbp::Project::read(&project_path);
    let declared = project.declared_objects();

    program_counts(&image, &declared, &recovered)
}

/// The declared count [`ProgramCounts`] means when this file says
/// "declared": the binary's own procedure slots, plus the slots the
/// standard-module rule caps. See `tests/ratios.toml`'s own header
/// comment for why the cap is stated rather than excluded.
///
/// `pub(crate)`: `crates/xtask` reaches this the same way it reaches
/// [`format_entry`], so the writer never repeats this addition under its
/// own, un-allow-listed `arithmetic_side_effects` lint.
pub(crate) fn declared_total(counts: &ProgramCounts) -> u32 {
    counts.declared_by_binary + counts.capped_by_standard_module
}

/// Gives the public procedure names the source declares that the library
/// did not recover, over the objects [`program_counts`] keeps, for one
/// program. This is the list a [`REGRESSION`] message prints. On the real
/// corpus this is always empty (185 of 185 match in both directions,
/// per `differential.rs`), and the message says so explicitly rather than
/// printing nothing and leaving the reader to guess.
fn declared_not_recovered_names(program: &Program, projects: &[PathBuf]) -> Vec<String> {
    let image = PeImage::parse(&program.image_bytes)
        .unwrap_or_else(|err| panic!("{}: PeImage::parse: {err:?}", program.key));
    let recovered = differential::recovered_objects(&image);

    let exe_path = corpus_root().join(&program.key);
    let project_path = vbp::select_project_file(&exe_path, projects)
        .unwrap_or_else(|err| panic!("{}: {err}", program.key));
    let project = vbp::Project::read(&project_path);
    let declared = project.declared_objects();

    let mut missing = Vec::new();
    for d in &declared {
        let Some(name) = &d.name else { continue };
        if d.kind == vbp::ObjectKind::Module {
            continue;
        }
        if !d.source_file.exists() {
            continue;
        }
        let Some(r) = recovered.iter().find(|o| &o.name == name) else {
            continue;
        };
        let recovered_names: std::collections::BTreeSet<String> =
            match deform6::vb::privateobj::ProcedureList::read(&image, r).procs {
                deform6::vb::privateobj::ProcNames::Slots(slots) => slots
                    .into_iter()
                    .filter_map(|p| match p {
                        deform6::vb::privateobj::Procedure::Public(n) => Some(n),
                        deform6::vb::privateobj::Procedure::Private => None,
                    })
                    .collect(),
                deform6::vb::privateobj::ProcNames::NoNameArray { .. } => {
                    std::collections::BTreeSet::new()
                }
            };
        let declared_names: std::collections::BTreeSet<String> =
            source::declared_public_procedures(&d.source_file)
                .into_iter()
                .collect();
        for missing_name in declared_names.difference(&recovered_names) {
            missing.push(format!("{name}.{missing_name}"));
        }
    }
    missing.sort();
    missing
}

/// Whether a pinned number sits above, at, or below a measured number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
}

fn compare(pinned: u32, measured: u32) -> Option<Direction> {
    match pinned.cmp(&measured) {
        std::cmp::Ordering::Greater => Some(Direction::Up),
        std::cmp::Ordering::Less => Some(Direction::Down),
        std::cmp::Ordering::Equal => None,
    }
}

/// Checks one program's pinned entry against its measured counts, giving
/// every failure message this program produces (zero, one, or two: the
/// recovered count and the declared count are checked independently).
fn check_program(
    key: &str,
    pinned: &PinnedEntry,
    measured_recovered: u32,
    measured_declared: u32,
    missing: &[String],
) -> Vec<String> {
    let mut failures = Vec::new();

    if let Some(direction) = compare(pinned.recovered, measured_recovered) {
        failures.push(recovered_mismatch_message(
            key,
            direction,
            pinned.recovered,
            measured_recovered,
            measured_declared,
            missing,
        ));
    }
    if let Some(direction) = compare(pinned.declared, measured_declared) {
        failures.push(declared_mismatch_message(
            key,
            direction,
            pinned.declared,
            measured_recovered,
            measured_declared,
        ));
    }

    let expected_ratio_text = format_ratio(pinned.recovered, pinned.declared);
    if pinned.ratio_text != expected_ratio_text {
        failures.push(format!(
            "{key}: the stored ratio {:?} does not equal {:?}, recomputed from the pinned counts \
             {} and {}",
            pinned.ratio_text, expected_ratio_text, pinned.recovered, pinned.declared
        ));
    }

    failures
}

fn recovered_mismatch_message(
    key: &str,
    direction: Direction,
    pinned: u32,
    measured: u32,
    measured_declared: u32,
    missing: &[String],
) -> String {
    match direction {
        Direction::Up => {
            let missing_text = if missing.is_empty() {
                "nothing the source declares is unrecovered; the tool named every public \
                 procedure this program's source declares, so this pin disagrees with a tool \
                 that lost nothing"
                    .to_owned()
            } else {
                format!("{missing:?}")
            };
            format!(
                "{key}: {REGRESSION}: the pin claims {pinned} recovered, the tool recovers \
                 {measured}. Procedures the source declares that the tool did not name: \
                 {missing_text}"
            )
        }
        Direction::Down => format!(
            "{key}: {MOVED_UP}: the pin claims {pinned} recovered, the tool recovers {measured}. \
             Paste this block into tests/ratios.toml:\n{}",
            format_entry(key, measured, measured_declared)
        ),
    }
}

/// The message a corpus program with no `tests/ratios.toml` entry
/// produces. One function, so [`the_gate_passes_on_the_committed_file`] and
/// [`a_stale_key_and_a_missing_key_fail_with_two_different_messages`] can
/// never let their wording drift apart.
fn missing_key_message(key: &str) -> String {
    format!("{key}: this corpus program has no entry in tests/ratios.toml")
}

/// The message a `tests/ratios.toml` entry with no matching corpus program
/// produces. See [`missing_key_message`] for why this is its own function.
fn stale_key_message(key: &str) -> String {
    format!("{key}: this key in tests/ratios.toml names no corpus program")
}

fn declared_mismatch_message(
    key: &str,
    direction: Direction,
    pinned: u32,
    measured_recovered: u32,
    measured_declared: u32,
) -> String {
    match direction {
        Direction::Up => format!(
            "{key}: {REGRESSION}: the pin claims {pinned} declared procedure slots, the binary \
             declares {measured_declared}"
        ),
        Direction::Down => format!(
            "{key}: {MOVED_UP}: the pin claims {pinned} declared procedure slots, the binary \
             declares {measured_declared}. Paste this block into tests/ratios.toml:\n{}",
            format_entry(key, measured_recovered, measured_declared)
        ),
    }
}

#[test]
fn the_header_constant_matches_the_committed_files_own_header() {
    let text = std::fs::read_to_string(ratios_toml_path())
        .unwrap_or_else(|err| panic!("{}: {err}", ratios_toml_path().display()));
    assert!(
        text.starts_with(HEADER),
        "HEADER must reproduce tests/ratios.toml's own header byte for byte, so \
         crates/xtask update-ratios writes the identical file; HEADER holds {} bytes",
        HEADER.len()
    );
}

#[test]
fn the_pinned_file_holds_forty_four_entries_and_the_totals_one_hundred_eighty_five_and_nine_hundred_four()
 {
    let pinned = read_pinned();
    assert_eq!(
        pinned.len(),
        EXPECTED_PROGRAM_COUNT,
        "tests/ratios.toml holds {} entries, wanted {EXPECTED_PROGRAM_COUNT}",
        pinned.len()
    );

    let total_recovered: u32 = pinned.values().map(|e| e.recovered).sum();
    let total_declared: u32 = pinned.values().map(|e| e.declared).sum();
    assert_eq!(total_recovered, EXPECTED_TOTAL_RECOVERED);
    assert_eq!(total_declared, EXPECTED_TOTAL_DECLARED);
}

/// Runs the whole gate: every corpus program checked against `pinned`, in
/// both directions on the key set (a program with no entry, and an entry
/// with no program, per the both-directions rule `differential.rs`
/// established for names). This is the one function both
/// [`the_gate_passes_on_the_committed_file`] and
/// [`a_missing_key_fails_the_real_gate`] run, so a broken key-set check can
/// never pass one and silently ship in the other.
fn gate_failures(
    pinned: &BTreeMap<String, PinnedEntry>,
    progs: &[Program],
    projects: &[PathBuf],
) -> Vec<String> {
    let mut failures = Vec::new();
    let mut seen_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for program in progs {
        seen_keys.insert(program.key.clone());
        let Some(entry) = pinned.get(&program.key) else {
            failures.push(missing_key_message(&program.key));
            continue;
        };
        let counts = counts_for(program, projects);
        let missing = declared_not_recovered_names(program, projects);
        failures.extend(check_program(
            &program.key,
            entry,
            counts.recovered,
            declared_total(&counts),
            &missing,
        ));
    }

    for stale_key in pinned.keys().filter(|k| !seen_keys.contains(*k)) {
        failures.push(stale_key_message(stale_key));
    }

    failures
}

#[test]
fn the_gate_passes_on_the_committed_file() {
    let pinned = read_pinned();
    let projects = vbp::project_files();
    let progs = programs();
    assert_eq!(progs.len(), EXPECTED_PROGRAM_COUNT);

    let failures = gate_failures(&pinned, &progs, &projects);
    assert!(
        failures.is_empty(),
        "the committed pin must pass on the committed tree:\n{}",
        failures.join("\n")
    );
}

#[test]
fn a_missing_key_fails_the_real_gate() {
    // Not a check on `missing_key_message`'s wording alone (that would
    // pass even if the gate stopped calling it): this runs the real
    // `gate_failures` the production path uses, against a pinned map with
    // one real entry deleted, and proves the gate actually notices.
    let mut pinned = read_pinned();
    let projects = vbp::project_files();
    let progs = programs();

    let removed_key = progs
        .first()
        .expect("at least one corpus program")
        .key
        .clone();
    pinned.remove(&removed_key);

    let failures = gate_failures(&pinned, &progs, &projects);
    assert!(
        failures
            .iter()
            .any(|f| f == &missing_key_message(&removed_key)),
        "deleting a real entry from the pinned map must make the gate report the missing key: \
         {failures:?}"
    );
}

#[test]
fn raising_a_pinned_recovered_count_fails_with_regression() {
    let pinned = read_pinned();
    let projects = vbp::project_files();
    let progs = programs();
    let program = progs
        .iter()
        .find(|p| p.key.contains("Grayscale-effect"))
        .expect("Grayscale-effect is a corpus program");
    let entry = pinned
        .get(&program.key)
        .expect("Grayscale-effect is pinned");
    let counts = counts_for(program, &projects);
    let missing = declared_not_recovered_names(program, &projects);

    let doctored = PinnedEntry {
        recovered: entry.recovered + 1,
        ..entry.clone()
    };
    let failures = check_program(
        &program.key,
        &doctored,
        counts.recovered,
        declared_total(&counts),
        &missing,
    );
    assert!(
        !failures.is_empty(),
        "raising the pinned recovered count above what the tool measures must fail"
    );
    let message = failures.join("\n");
    assert!(
        message.contains(REGRESSION),
        "raising a pin must print {REGRESSION:?}, got: {message}"
    );
    assert!(message.contains(&program.key));
    assert!(message.contains(&doctored.recovered.to_string()));
    assert!(message.contains(&counts.recovered.to_string()));
    assert!(
        message.contains("nothing the source declares is unrecovered"),
        "the missing list is empty on real corpus data, and the message must say so rather \
         than print nothing: {message}"
    );
}

#[test]
fn lowering_a_pinned_recovered_count_fails_with_moved_up_and_the_exact_paste_block() {
    let pinned = read_pinned();
    let projects = vbp::project_files();
    let progs = programs();
    let program = progs
        .iter()
        .find(|p| p.key.contains("Grayscale-effect"))
        .expect("Grayscale-effect is a corpus program");
    let entry = pinned
        .get(&program.key)
        .expect("Grayscale-effect is pinned");
    assert!(
        entry.recovered > 0,
        "Grayscale-effect must recover at least one procedure for this test to lower it"
    );
    let counts = counts_for(program, &projects);
    let missing = declared_not_recovered_names(program, &projects);

    let doctored = PinnedEntry {
        recovered: entry.recovered - 1,
        ..entry.clone()
    };
    let failures = check_program(
        &program.key,
        &doctored,
        counts.recovered,
        declared_total(&counts),
        &missing,
    );
    assert!(!failures.is_empty());
    let message = failures.join("\n");
    assert!(
        message.contains(MOVED_UP),
        "lowering a pin must print {MOVED_UP:?}, got: {message}"
    );

    let expected_block = format_entry(&program.key, counts.recovered, declared_total(&counts));
    assert!(
        message.contains(&expected_block),
        "the MOVED UP message must hold the exact block to paste:\nwanted:\n{expected_block}\ngot:\n{message}"
    );
}

#[test]
fn editing_only_the_ratio_fails_because_it_disagrees_with_its_own_counts() {
    let pinned = read_pinned();
    let (key, entry) = pinned
        .iter()
        .next()
        .expect("tests/ratios.toml holds at least one entry");

    let doctored = PinnedEntry {
        ratio_text: "0.99".to_owned(),
        ..entry.clone()
    };
    // Compare the doctored entry against itself: no drift from the corpus
    // is involved, only the ratio-consistency check, so the counts (which
    // agree with themselves) never fail and the ratio mismatch is the only
    // possible failure.
    let failures = check_program(key, &doctored, entry.recovered, entry.declared, &[]);
    assert!(
        !failures.is_empty(),
        "a ratio that disagrees with its own pinned counts must fail even when both counts are \
         correct"
    );
    assert!(failures.iter().any(|f| f.contains("0.99")));
}

#[test]
fn a_stale_key_and_a_missing_key_fail_with_two_different_messages() {
    // Runs the real `gate_failures`, not just the message-formatting
    // functions in isolation: a covering test that only checks
    // `stale_key_message(x) != missing_key_message(x)` would still pass
    // even if the gate stopped calling either one.
    let pinned = read_pinned();
    let projects = vbp::project_files();
    let progs = programs();

    let stale_key = "nowhere/NotAProgram.exe";
    let mut with_stale = pinned.clone();
    with_stale.insert(
        stale_key.to_owned(),
        PinnedEntry {
            recovered: 0,
            declared: 1,
            ratio_text: "0.00".to_owned(),
        },
    );
    let stale_failures = gate_failures(&with_stale, &progs, &projects);
    let stale_failure = stale_failures
        .iter()
        .find(|f| f.contains(stale_key))
        .unwrap_or_else(|| {
            panic!("adding an unmatched key must produce a failure: {stale_failures:?}")
        });
    assert_eq!(*stale_failure, stale_key_message(stale_key));

    let removed_key = progs
        .first()
        .expect("at least one corpus program")
        .key
        .clone();
    let mut without_first = pinned.clone();
    without_first.remove(&removed_key);
    let missing_failures = gate_failures(&without_first, &progs, &projects);
    let missing_failure = missing_failures
        .iter()
        .find(|f| f.contains(&removed_key))
        .unwrap_or_else(|| {
            panic!("deleting a real entry must produce a failure: {missing_failures:?}")
        });
    assert_eq!(*missing_failure, missing_key_message(&removed_key));

    assert_ne!(
        stale_failure, missing_failure,
        "a stale key and a missing key must produce two different messages"
    );
}
