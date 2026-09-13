# Phase 5: Hostility - Research

**Researched:** 2026-09-13
**Domain:** Fuzzing a Rust parser with `cargo-fuzz`/libFuzzer, retrofitting an existing strict/salvage policy split into 24 read functions across 8 modules, and a run-time-fetched robustness corpus with pinned hashes.
**Confidence:** HIGH on what the codebase already contains (read directly this session, with line numbers) and on `cargo-fuzz`/libFuzzer flag semantics (verified against the project's own README and LLVM's own docs this session). MEDIUM on the exact shape the `Journal` retrofit should take, because two viable designs exist and the codebase does not yet commit to either (logged as Assumption A1).

## Summary

Phase 5 is not a green field. Three of its five requirements already have working, tested infrastructure sitting unused in the tree, built ahead of time in Phase 1 for exactly this day. `Journal` and `Mode::{Strict, Salvage}` (`crates/deform6/src/journal.rs`, whole file read this session) already implement the strict/salvage policy split, with full unit test coverage, and `DefectKind::ImplausibleCount` (`crates/deform6/src/error.rs:82-89`) already exists and is already raised at nine call sites across six modules. `panic = "abort"` is already set in the release profile and `exclude = ["crates/deform6/fuzz"]` is already in the root `Cargo.toml` (both read this session, `Cargo.toml:1-24`) — half of the named risk "the isolation needs two statements, not one" is already done, apparently staged ahead of time by whoever wrote Phase 1.

The single fact every plan in this phase must treat as load-bearing: **`Journal` is built but wired into nothing.** A workspace-wide grep this session (`grep -rn "Journal" crates/deform6/src --include="*.rs" | grep -v journal.rs`) returns zero results. Every one of the 24 `read`/`parse`/`walk` functions in `crates/deform6/src/vb/*.rs` still does what Phase 1-4 always did: push a `Defect` onto a locally-owned `Vec<Defect>` and keep going, with no consultation of any `Mode` at all. `pub fn inspect(data: &[u8], opcode_table: &OpcodeTable) -> Result<Report, Refusal>` (`crates/deform6/src/vb/mod.rs:344`) and `pub fn project(report: &Report, data: &[u8]) -> Result<WrittenProject, Refusal>` (`crates/deform6/src/write/mod.rs:99`) both take no `Mode` parameter today. This means the crate's actual current behavior is **"always salvage" for every `Recoverable` defect** (it already continues past every one, unconditionally) and **"always strict" for every `Fatal` one** (it already refuses, unconditionally) — there is no live code path today that ever *refuses* on a `Recoverable` defect, which is exactly what Phase 5's default ("strict") behavior requires. Wiring `--salvage` in is therefore not "add a flag" — it is "add the missing half of a policy switch to roughly 24 functions across 8 files," the largest and riskiest single task in this phase, bigger in blast radius than the fuzz harness itself.

The second load-bearing fact: **the bound-check discipline SAF-04 asks for is unevenly applied today, and the gap is concrete and located.** Six `Vec::with_capacity` call sites exist in the crate (a corpus-wide grep this session found exactly six, plus a seventh site — `controlinfo.rs` — that has a dedicated test asserting **zero** `with_capacity` calls in its own production code: `crates/deform6/src/vb/controlinfo.rs:1103-1105`, quoted below). Five of the six are already safe by construction (bounded by a prior `subregion` call, a `u8`-range shift, a fixed constant, or a compile-time literal — traced line-by-line below). But `crates/deform6/src/vb/gui.rs:79-111`, `GuiTable::walk`, the function that reads `header.w_form_count` — **the exact field the roadmap's own success criterion 4 names** (`wFormCount = 0xFFFF`) — has no pre-loop bound check against the real file size at all. It loops `0_u32..u32::from(header.w_form_count)` and relies solely on each iteration's `array.subregion(...)` call failing once the file runs out of bytes. This is memory-safe (no panic, no oversized allocation — `Vec::new()` grows incrementally) but it does **not** produce `DefectKind::ImplausibleCount`, and it returns a hard `Refusal::Damaged` (bypassing `Defect`/`Journal` entirely) rather than a `Recoverable` defect a salvage run could continue past. This is a real, precisely located gap, not a generic warning: `object.rs`'s own `bound_proc_count` (`crates/deform6/src/vb/object.rs:279-306`, quoted below) is the canonical *correct* pattern already living twelve files away, and plan 05-02 is "make `GuiTable::walk` (and every other count field the audit finds in the same shape) look like `bound_proc_count`," not "invent a new mechanism."

**Primary recommendation:** Treat plan 05-01 as a `Journal`-threading retrofit, not new design work — the type, its policy table, and its full test suite already exist and are already correct; the work is call-site surgery. Treat plan 05-02 as an audit against the existing `bound_proc_count`/`DeclareTable::read` pattern, not new invention — find every count/length field STRUCTURES.md catalogs, classify each against the pattern that already works, and fix the ones (starting with `w_form_count`) that do not yet match it. Treat plans 05-03/05-04 as pure infrastructure with no novel design risk: `cargo-fuzz`, `libfuzzer-sys` and `arbitrary` are all `OK`-verdict, long-lived, `rust-fuzz`-org-maintained crates, and the exact flags the roadmap already names (`--fuzz-dir`, `-max_total_time`, `-rss_limit_mb`, `--fuzzing-workspace`) are all real, current, and spelled correctly in the roadmap text.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Strict/salvage policy decision | Library (`deform6::journal::Journal`) | - | `journal.rs`'s own doc comment: "A parse site does not know which mode the run is in... The policy lives in `record` and nowhere else." This is already the architecture; Phase 5 wires call sites into it, it does not redesign it. |
| `--salvage` flag surface | CLI (`deform6-cli`) | Library (`Mode` parameter on `inspect`/`write::project`) | Matches the existing split (`main.rs`'s own doc comment: the library never opens a file, the CLI owns exit codes) — a new `Mode`-typed parameter on the two public library entry points, selected by a new CLI flag. |
| Bound checking of a file-derived count before allocation | Library (`deform6::vb::*`, per-module, at the read site) | - | Every existing correct instance (`bound_proc_count`, `DeclareTable::read`) lives directly beside the field it bounds; there is no shared "count checker" utility today, and none should be invented that the audit does not need — see "Don't Hand-Roll" below for the one exception (a possible small shared helper, logged as an open question). |
| The fuzz target itself | New crate, `crates/deform6/fuzz` (excluded from the workspace) | - | `cargo-fuzz`'s own convention: a `#![no_main]` crate that only nightly builds; the roadmap's own named risk requires it stay out of `cargo test --workspace` and `cargo clippy --all-targets`, which an excluded, non-member crate structurally guarantees. |
| Fuzz CI orchestration (`--fuzz-dir`, PR-bounded run, cron run) | `xtask` (new subcommand) | GitHub Actions (`.github/workflows/`, new job) | The roadmap's own named risk: "Put the call in an `xtask` command so nobody types it by hand," matching this repository's existing convention of hiding every non-trivial multi-flag invocation behind `xtask` (`update-ratios`, `derive-opcode-table`). |
| Regression replay (crash-to-test) | Test harness (`crates/deform6/tests/regressions.rs`, new) | Corpus (`tests/regressions/`, new directory) | Mirrors the existing `crates/deform6/tests/corpus_sweep.rs`/`differential.rs` pattern: a `--test` binary that walks a directory and asserts over every file in it. |
| Run-time robustness corpus fetch | `xtask fetch-corpus` (new subcommand) | `corpus/manifest.toml` (new, committed) | Same tier split the project already uses for the vendored corpus vs. everything else: `AGENTS.md`'s "What may enter this repository" — a program without a redistribution licence is fetched at run time from a pinned-hash manifest, never committed. |

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `cargo-fuzz` | crates.io | 2017-02-21, ~9.5 years | 102,276/week | github.com/rust-fuzz/cargo-fuzz | OK | Approved |
| `libfuzzer-sys` | crates.io | 2019-09-10, ~7 years | 1,178,261/week | github.com/rust-fuzz/libfuzzer | OK | Approved |
| `arbitrary` | crates.io | 2017-05-08, ~9.3 years | 3,139,223/week | github.com/rust-fuzz/arbitrary | OK | Approved |
| `ureq` | crates.io | 2018-06-22, ~8.2 years | 4,497,370/week | github.com/algesten/ureq | OK | Approved |
| `sha2` | crates.io | 2016-05-06, ~10.3 years | 19,358,921/week | github.com/RustCrypto/hashes | OK | Approved |

Verified via `gsd_run query package-legitimacy check --ecosystem crates <name>` this session, all five returning `"verdict":"OK"` with `"deprecated":false` and `"postinstall":null` [VERIFIED: gsd-tools package-legitimacy seam, this session]. Cross-checked against the live registry with `cargo search` this session: `cargo-fuzz = "0.13.2"`, `libfuzzer-sys = "0.4.13"`, `arbitrary = "1.4.2"`, `ureq = "3.4.1"`, `sha2 = "0.11.0"` [VERIFIED: crates.io registry, `cargo search`, this session]. All five are maintained by either the `rust-fuzz` GitHub organization (the first three — a coherent, single-maintainer trust boundary already implied by choosing `cargo-fuzz` at all) or `RustCrypto` (`sha2`, the same organization pattern this project already trusts nowhere yet, but a well-known one). `ureq` was chosen over an async HTTP client (`reqwest` + `tokio`) because `xtask fetch-corpus` is a one-shot CLI tool with no concurrency requirement, and this workspace has zero async runtime dependencies today — adding one for a single sequential download would be a new, heavier dependency class this project has never needed.

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `cargo-fuzz` | 0.13.2 (verified, `cargo search` this session) [VERIFIED: crates.io] | The `cargo fuzz` subcommand: scaffolds the fuzz crate, drives libFuzzer, manages the corpus directory | The de facto standard fuzzing front end for Rust; `rust-fuzz`-org-maintained, matches the roadmap's own named risks and success-criterion command line verbatim. |
| `libfuzzer-sys` | 0.4.13 [VERIFIED: crates.io] | The `#[no_main]` harness macro (`fuzz_target!`) that `cargo fuzz init` generates a dependency on | Standard companion to `cargo-fuzz`; every `cargo fuzz init`-generated crate depends on it by default. |
| `arbitrary` | 1.4.2 [VERIFIED: crates.io] | Structured-input generation from raw fuzzer bytes, if the fuzz target needs anything beyond a raw `&[u8]` | The `parse` target the roadmap names (`cargo +nightly fuzz run ... parse`) most naturally takes a raw byte slice directly (the same shape `deform6::inspect(data: &[u8], ...)` already takes) with no `Arbitrary` derive needed; keep this as an optional dependency the fuzz crate can add if the target later needs to fuzz `Mode` or the opcode table alongside the bytes (see Open Questions). |
| `ureq` | 3.4.1 [VERIFIED: crates.io] | Synchronous HTTP client for `xtask fetch-corpus` | A blocking, minimal-dependency HTTP client; matches this workspace's existing dependency minimalism (no async runtime exists anywhere in the tree today: confirmed by grepping every `Cargo.toml` this session, zero hits for `tokio`, `async-std`, `reqwest`). |
| `sha2` | 0.11.0 [VERIFIED: crates.io] | SHA-256 hashing to verify a fetched file against `corpus/manifest.toml`'s pinned hash | RustCrypto's own reference implementation; the named risk requires the pinned hash to be checked, not merely stored. |

### Supporting

None. No dependency is needed for the `Journal` retrofit, the bound-check audit, the regression test harness, or the CI workflow file — all four are either already-shipped code (`Journal`, `DefectKind::ImplausibleCount`) or a few lines of new code following an already-established pattern in the same file.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `ureq` for `xtask fetch-corpus` | `reqwest` + `tokio` | `reqwest` is the more common choice in general Rust web work, but it drags in an async runtime this workspace has never needed; `xtask` is a synchronous, sequential CLI tool (see `crates/xtask/src/main.rs`, whole file read this session — every existing subcommand is synchronous), so an async client buys nothing and costs a large new dependency tree. |
| `ureq` for `xtask fetch-corpus` | `std::process::Command` shelling out to `curl` | Avoids a new dependency entirely, but silently depends on `curl` being installed on the CI runner and the developer's machine, which is an unstated environment dependency this project's own `AGENTS.md` "what to measure" discipline would flag; a Rust dependency is a measured, pinned, `Cargo.lock`-tracked fact, a shelled-out binary is not. |
| A hand-rolled crash-to-regression-test script | `cargo fuzz fmt`/manual copy | `cargo-fuzz` itself ships `cargo fuzz fmt <target> <crash-file>` to pretty-print a crash input; the "written crash-to-test procedure" plan 05-05 owns is documentation of a manual `cp` + a new `#[test]` function reading the copied file, not a new tool — see Code Examples. |

**Installation:**
```toml
# root Cargo.toml — already present, verified this session:
# [workspace]
# exclude = ["crates/deform6/fuzz"]
# [profile.release]
# panic = "abort"

# crates/deform6/fuzz/Cargo.toml — created by `cargo +nightly fuzz init
# --fuzzing-workspace true --fuzz-dir crates/deform6/fuzz`, NOT hand written;
# `cargo fuzz init` generates its own [dependencies] block with
# libfuzzer-sys and its own [workspace] table (making it independent, per
# --fuzzing-workspace true), and its own [[bin]] entry for the `parse` target.

# crates/xtask/Cargo.toml, [dependencies], hand-added for fetch-corpus:
ureq = "3"
sha2 = "0.11"
```

**Version verification:** `cargo search cargo-fuzz`, `cargo search libfuzzer-sys`, `cargo search arbitrary`, `cargo search ureq`, `cargo search sha2` were all run this session against the live crates.io registry [VERIFIED: crates.io registry, `cargo search`, this session]; results quoted above.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SAF-01 | The tool does not panic on any input | Already substantially achieved crate-wide by `#![forbid(unsafe_code)]` (`lib.rs:1`) and `Region`'s no-infallible-accessor design (`read/region.rs`, every accessor returns `Option`, `checked_add`/`checked_sub` are the only offset arithmetic — verified this session). Phase 5's job is to *prove* this (the fuzz target, plan 05-03) and *close the `--salvage` gap* that would let a hostile file panic a code path only reachable under salvage (named risk: "Salvage reaches code that strict refuses before it gets there"), not to add new panic-safety machinery. |
| SAF-02 | Refuses a damaged file by default, names the byte offset and expectation | `DefectKind`'s own `#[error(...)]` messages already name the offset and the expectation for all sixteen existing variants (`error.rs:49-286`, every variant's `#[error]` attribute interpolates `offset` and a structure-specific fact) [VERIFIED: `crates/deform6/src/error.rs:52,63,72,81,92,101,112,123,144,158,173,192,213,227,244,261,277`]. What is missing is the *default-refuses* behavior for `Recoverable` defects (see Summary) — plan 05-01's core task. |
| SAF-03 | `--salvage` recovers what it can, marks every assumption in the report | `Journal::record` (`journal.rs:64-71`) already returns the fallback value in `Mode::Salvage`; the report side (`crate::report::ProjectReport.limits: Vec<String>`, `report.rs:34-38`) already carries a free-text list that a salvage run's assumptions can extend, and `Confidence::Inferred`/`Confidence::Unrecoverable` (`report.rs:81-91`) already exist as the vocabulary for "this had to be assumed." Plan 05-01 must decide whether a `--salvage`-only assumption becomes a `limits` line, an `Inferred`-confidence `ReportItem`, or a new field — none of the three existing extension points is obviously wrong, and none has been chosen yet (Assumption A2). |
| SAF-04 | Never sizes an allocation from a file length field before checking it against the real file size | Substantially, but not completely, true today: 5 of 6 `Vec::with_capacity` sites are already safe by construction (see Summary and Common Pitfalls), and the canonical correct pattern (`object.rs::bound_proc_count`, `error.rs::DefectKind::ImplausibleCount`) already exists and is already used at nine call sites. The one located, concrete gap is `gui.rs::GuiTable::walk`'s `header.w_form_count` (the exact field the roadmap's own success criterion 4 names), which has no pre-loop check and produces a generic `Refusal::Damaged` instead of a `Recoverable` `ImplausibleCount` defect. Plan 05-02's audit must also check every other count/length field STRUCTURES.md catalogs (object counts, GUI table, external table, property stream lengths, `.frx` blob lengths, control counts) against this same pattern; several are already correct (`ObjectTable`, `DeclareTable`, the `.frx` `BlobCursor`) and should not be re-touched. |
| SAF-05 | A fuzzer runs in the gate; every crash becomes a committed regression test that replays on stable | No fuzzing infrastructure exists in the tree today (`find crates/deform6/fuzz` returns nothing; `.github/workflows/` holds only `gate.yml`, read in full this session, with no fuzz job). This is genuinely new infrastructure, built from the `cargo-fuzz`/libFuzzer stack verified in "Standard Stack" above, following the exact CLI shape the roadmap's own success criterion 1 already specifies verbatim. |

</phase_requirements>

## Architecture Patterns

### System Architecture Diagram

```
                     hostile bytes (fuzzer, corpus file, or user input)
                                    |
                                    v
                    +-------------------------------+
                    |   deform6::inspect(data, mode) |   <- Mode param is NEW
                    |   deform6::write::project(     |      (05-01)
                    |     report, data, mode)         |
                    +---------------+-----------------+
                                    |
                    each read/parse/walk site (24 fns, 8 files) now:
                                    |
                                    v
                    +-------------------------------+
                    |  1. bound-check the count/len   |   <- audited/fixed
                    |     field against real length    |      (05-02)
                    |     (checked_mul, compare vs.    |
                    |     region.len(), BEFORE any     |
                    |     Vec::with_capacity call)     |
                    +---------------+-----------------+
                                    |
                                    v
                    +-------------------------------+
                    |  2. build a Defect, hand it to   |   <- retrofit target
                    |     Journal::record(defect,      |      (05-01)
                    |     fallback)? -- ALREADY BUILT, |
                    |     ALREADY TESTED, NOT WIRED    |
                    +---------------+-----------------+
                        Strict: Fatal or Recoverable -> refuse, name offset
                        Salvage: Recoverable -> continue with fallback,
                                 mark report; Fatal -> still refuses
                                    |
                                    v
                    +-------------------------------+
                    |   Report / ProjectReport         |
                    |   .defects (same list, both       |
                    |    modes, per SC3)                |
                    |   .limits / new assumption items  |   <- SAF-03 wiring
                    |    (Salvage-only)                 |      (05-01)
                    +---------------+-----------------+
                                    |
              +---------------------+----------------------+
              |                                             |
              v                                             v
  +-----------------------+                    +----------------------------+
  | fuzz target: parse.rs |                    | tests/regressions.rs (new) |
  | calls inspect() AND   |  <- crash found ->  | replays every committed    |
  | write::project() in   |     copy input to    | crash file through BOTH   |
  | BOTH Strict and        |     tests/           | Strict and Salvage, on    |
  | Salvage (named risk)   |     regressions/     | stable Rust               |
  +-----------------------+                    +----------------------------+
```

A reader can trace a hostile byte slice through the bound check, the journal decision, the two modes' divergent outcomes, and into either the fuzz target or the stable regression suite, by following the arrows above.

### Recommended Project Structure

```
crates/deform6/
├── fuzz/                       # New, excluded from [workspace] (05-03)
│   ├── Cargo.toml              # generated by `cargo fuzz init`, not hand-written
│   └── fuzz_targets/
│       └── parse.rs            # calls inspect() + write::project() in both modes
├── src/
│   ├── journal.rs              # UNCHANGED type; every read fn threads Mode/&mut Journal through it (05-01)
│   ├── vb/gui.rs                # GuiTable::walk gets the missing pre-loop bound check (05-02)
│   └── ...                      # 23 other read/parse/walk fns audited the same way
├── tests/
│   └── regressions.rs          # New (05-05): replays tests/regressions/*, both modes, stable
└── tests/regressions/           # New, empty at first commit but the count assertion
                                  # (SC2: "emptying the directory makes the test fail")
                                  # means it must never be allowed to reach zero files
                                  # once populated -- needs at least one seed file
                                  # checked in immediately, not left for the fuzzer
                                  # to populate later (see Common Pitfalls).
crates/xtask/src/
├── main.rs                     # new subcommands: fuzz-ci, fetch-corpus (05-04, 05-07)
└── fetch_corpus.rs             # new: reads corpus/manifest.toml, fetches, verifies SHA-256
corpus/
├── manifest.toml                # New (05-07): pinned SHA-256 per program, committed
└── fetched/                     # New, gitignored: where fetch-corpus writes bytes, never committed
.github/workflows/
├── gate.yml                     # UNCHANGED
└── fuzz.yml                     # New (05-04): PR-bounded job (SC1's exact command) + cron job
```

### Pattern 1: The `Journal`/`Mode` retrofit follows the shape `object.rs::bound_proc_count` already proves works

**What:** `Journal::record` (already shipped, `journal.rs:64-71`) takes a `Defect` and a fallback value, pushes the defect unconditionally, then returns `Ok(fallback)` in `Mode::Salvage` or `Err(Error::Refused(defect))` in `Mode::Strict` for a `Recoverable` defect (both modes refuse a `Fatal` one).

**When to use:** Every one of the ~24 `read`/`parse`/`walk` functions that currently builds a `Defect` and pushes it to a local `Vec<Defect>` without ever consulting anything must instead route that same `Defect` through a `Journal` (passed by `&mut` or via a `Mode` parameter threaded to the same depth) and propagate the `Result` it returns.

**Example — the already-correct pattern to generalize (not new code, quoted from the shipped file):**
```rust
// Source: crates/deform6/src/vb/object.rs:279-306 (VERIFIED, already shipped)
fn bound_proc_count(
    pe: &PeImage<'_>,
    element: &Region<'_>,
    lp_proc_names_array: Va,
    raw_proc_count: u32,
) -> (u32, Option<Defect>) {
    let Some(array_region) = pe.region_at_va(lp_proc_names_array) else {
        return (raw_proc_count, None);
    };
    let max_entries = array_region
        .len()
        .checked_div(PROC_NAME_PTR_SIZE)
        .unwrap_or(0);
    if raw_proc_count <= max_entries {
        return (raw_proc_count, None);
    }
    // ... builds a Defect { kind: DefectKind::ImplausibleCount { .. }, .. }
    // and returns (max_entries, Some(defect)) -- the CLAMPED count, safe to
    // allocate with, plus the defect describing what was clamped.
}
```
This function already does everything SAF-04 asks: bound the count against the real region length, before any allocation, and name the excess in an `ImplausibleCount` defect. What it does **not** yet do is consult a `Mode`: today, the caller always accepts the clamped `max_entries` value regardless of mode (i.e., it always "salvages" this particular defect). Plan 05-01's retrofit must decide, for a `Recoverable` defect like this one, whether `Mode::Strict` should now refuse the whole read instead of silently clamping — which is the actual behavior change SAF-02 requires ("refuses by default").

**The gap this pattern has NOT yet reached — quoted verbatim, this is the concrete fix target:**
```rust
// Source: crates/deform6/src/vb/gui.rs:79-111 (VERIFIED, already shipped, HAS THE GAP)
pub fn walk(pe: &PeImage<'_>, header: &VbHeader) -> Result<Self, Refusal> {
    let array = pe
        .region_at_va(header.lp_gui_table)
        .ok_or(Refusal::Damaged("the GUI table pointer is in no section"))?;

    let mut entries = Vec::new();
    for i in 0_u32..u32::from(header.w_form_count) {
        let at = i
            .checked_mul(GUI_ENTRY_SIZE)
            .ok_or(Refusal::Damaged("the GUI table index overflows a u32"))?;
        let entry = array
            .subregion(Off::new(at), GUI_ENTRY_SIZE)
            .ok_or(Refusal::Damaged("the file ends inside a GUI table entry"))?;
        // ...
    }
    Ok(Self { entries })
}
```
`header.w_form_count` (a `u16`, so its maximum possible value is `0xFFFF` — exactly the roadmap's own success-criterion-4 example) is never compared against `array.len() / GUI_ENTRY_SIZE` before the loop starts. The loop is memory-safe (no `with_capacity`, `subregion` bounds every read), so no crash and no oversized allocation occurs today — but the failure mode a hostile `wFormCount = 0xFFFF` in a 4 KB file produces is a generic `Refusal::Damaged("the file ends inside a GUI table entry")`, a `Fatal`-shaped, non-`Defect` refusal with no `ImplausibleCount`, not the `Recoverable` `ImplausibleCount` defect success criterion 4 explicitly names. Fixing this means adding the same four lines `bound_proc_count` already has, before the loop: compute `array.len() / GUI_ENTRY_SIZE`, compare against `w_form_count`, and if it exceeds, build and record an `ImplausibleCount` defect (clamping the loop bound in `Mode::Salvage`, refusing in `Mode::Strict`).

### Pattern 2: `cargo fuzz init` scaffolds the fuzz crate; do not hand-write its `Cargo.toml`

**What:** `cargo +nightly fuzz init --fuzzing-workspace true --fuzz-dir crates/deform6/fuzz` generates `crates/deform6/fuzz/Cargo.toml`, its own independent `[workspace]` table (because `--fuzzing-workspace true` was passed), a `fuzz_targets/fuzz_target_1.rs` stub, and a `libfuzzer-sys` dependency — matching exactly what the root `Cargo.toml`'s pre-existing `exclude = ["crates/deform6/fuzz"]` line already anticipates.

**Verified from the project's own README this session** [VERIFIED: cargo-fuzz README, WebFetch this session]: `--fuzzing-workspace` "allows the fuzz/ directory can either be a member of the parent workspace (default) or maintain its own independent workspace," defaulting to the parent-workspace case — confirming the roadmap's own named risk ("Pass `--fuzzing-workspace true`, because the flag defaults to `false`") is stated correctly [CITED: rust-fuzz/cargo-fuzz README]. Also confirmed this session: `cargo fuzz` "needs a nightly compiler since it uses some unstable command-line flags" [CITED: rust-fuzz/cargo-fuzz README] — matching `rust-toolchain.toml`'s pin to stable `1.97.1` (`channel = "1.97.1"`, read this session), meaning the fuzz job's CI step must install a nightly toolchain *in addition to* the pinned stable one, not replace it: `rustup toolchain install nightly` alongside the existing `rustup show` step in `gate.yml`, never editing `rust-toolchain.toml` itself.

**When to use:** Plan 05-03, once. Do not write `crates/deform6/fuzz/Cargo.toml` by hand — run the real command and commit what it generates, then hand-edit only `fuzz_targets/parse.rs` to call this crate's own `inspect`/`write::project`.

**`--fuzz-dir` is required on every subsequent invocation**, per the roadmap's own named risk; verified this session that the fuzz crate, once created at a non-default path (`crates/deform6/fuzz` rather than a repository-root `fuzz/`), needs `--fuzz-dir crates/deform6/fuzz` on `cargo fuzz build`, `cargo fuzz run`, and `cargo fuzz add` alike, which is exactly why the roadmap's own named risk insists this call live inside `xtask` rather than be typed by hand.

### Pattern 3: The fuzz target must exercise both modes, per the roadmap's own named risk

**What:** "Salvage reaches code that strict refuses before it gets there. That code is the least exercised in the crate, so the fuzz target must call both modes, not one."

**Example — the target's shape:**
```rust
// crates/deform6/fuzz/fuzz_targets/parse.rs (recommended shape, not yet written)
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let table = deform6::vb::opcodes::OpcodeTable::builtin();
    // Strict: proves SAF-01/SAF-02 hold on the default path.
    let _ = deform6::inspect(data, &table, deform6::journal::Mode::Strict);
    // Salvage: proves SAF-01/SAF-03 hold on the path the roadmap's own
    // named risk says is "the least exercised in the crate" -- this line
    // is the whole reason this named risk exists.
    if let Ok(report) = deform6::inspect(data, &table, deform6::journal::Mode::Salvage) {
        let _ = deform6::write::project(&report, data, deform6::journal::Mode::Salvage);
    }
});
```
This assumes plan 05-01 lands a `Mode` parameter on both `inspect` and `write::project`, in that exact shape — see Assumption A1 for the one open design choice (a `Mode` parameter vs. a `&mut Journal` parameter) this snippet's exact signature depends on.

### Anti-Patterns to Avoid

- **Writing a new, parallel "is this count safe" helper function instead of generalizing `bound_proc_count`:** the pattern already exists, is already tested, and is already used correctly at nine call sites (`error.rs`'s own `ImplausibleCount` grep this session found it in `frx.rs`, `controlinfo.rs`, `object.rs`, `project.rs`, `vbstr.rs`, `propstream.rs`, `controltree.rs`). A new helper with a different shape would create two conventions where one already works.
- **Hand-writing the fuzz crate's `Cargo.toml`:** see Pattern 2. `cargo fuzz init`'s generated file is what `cargo fuzz build`/`run` expect; a hand-rolled one risks a subtly wrong `[workspace]` table that silently re-joins the parent workspace, which is exactly the failure mode the named risk warns about.
- **Editing `rust-toolchain.toml` to pin nightly workspace-wide:** would break the pinned-stable guarantee every other gate command relies on (`gate.yml`'s own comment: `rustup show` reads the one pinned channel). Nightly is installed as a second, additional toolchain for the fuzz job only, invoked with `cargo +nightly fuzz ...`, never by changing the default.
- **Trusting `subregion`'s eventual failure as a substitute for an explicit `ImplausibleCount` check:** `GuiTable::walk` proves this is memory-safe but not requirement-compliant — success criterion 4 asks for the specific defect kind and the specific "before any allocation" ordering, not merely "does not crash."

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fuzzing harness, corpus management, crash minimization | A custom `AFL`-style or `honggfuzz`-style harness, or a hand-rolled random-byte-mutation loop in a `#[test]` | `cargo-fuzz` + libFuzzer | `cargo-fuzz` is the roadmap's own explicit choice, already reflected in every named risk and success criterion; libFuzzer's coverage-guided mutation and crash minimization (`cargo fuzz tmin`) are exactly the kind of "deceptively complex" problem no hand-rolled loop should re-derive. |
| SHA-256 verification of a fetched corpus file | A hand-rolled hash comparison using `std::hash` (which is not cryptographic and not stable across Rust versions) | `sha2` | `std::hash::Hash` is explicitly documented by the standard library as unstable across compiler versions and unsuitable for this purpose; `sha2` is the RustCrypto reference implementation. |
| The strict/salvage policy decision | A `match mode { ... }` re-implemented at each of the 24 call sites | `Journal::record`, already shipped | This is precisely the "choke point" `journal.rs`'s own doc comment describes: "Nothing else in this crate reads a `Mode`." Twenty-four independent `match mode` blocks would be twenty-four places the policy could drift; one `Journal` is one place it cannot. |
| Bound-checking a count field against file size | A new generic `checked_count(len, size) -> Result<u32, Defect>` utility invented from scratch | Generalize `object.rs::bound_proc_count`'s existing shape at each site the audit finds wanting | The existing pattern already handles the null/no-region case, the division, the clamp, and the defect construction correctly; a from-scratch utility risks re-deriving the same four lines with a subtly different edge case (e.g., forgetting the null-pointer early return `bound_proc_count` has). |

**Key insight:** Every piece of "don't hand-roll" advice in this phase points at code *already in this repository*, not at an external library. The domain-specific risk here is not "reinventing a wheel npm already has" — it is "reinventing a wheel `object.rs` already has, twelve files away, without noticing."

## Common Pitfalls

### Pitfall 1: Treating the `Journal` retrofit as "add a flag" instead of "change 24 functions' refusal behavior"

**What goes wrong:** A plan that scopes 05-01 as "add `--salvage` to the CLI and pass it through to `inspect`" without touching any of the 24 `read`/`parse`/`walk` functions will compile, will accept the flag, and will do nothing: `inspect` and `write::project` do not currently accept a `Mode` at all, and every one of their internal call sites already hard-codes "continue past `Recoverable`, refuse on `Fatal`" with no branch to attach a flag to.
**Why it happens:** `Journal` exists, is fully tested, and reads as "already done" on a quick grep for the word `Mode`. Only a grep for the word `Journal` (which appears nowhere outside its own file) reveals it is not wired to anything.
**How to avoid:** Scope 05-01 explicitly as touching every file `error.rs`'s `ImplausibleCount` grep already lists (`frx.rs`, `controlinfo.rs`, `object.rs`, `project.rs`, `vbstr.rs`, `propstream.rs`, `controltree.rs`) plus `gui.rs` (once 05-02 adds its check) plus `mod.rs`'s own `inspect` and `write/mod.rs`'s own `project` — roughly nine files, not one.
**Warning signs:** A plan whose `files_modified` list for 05-01 is three files or fewer (the CLI, `mod.rs`, `journal.rs`) has almost certainly not accounted for the call-site retrofit.

### Pitfall 2: `tests/regressions/` starting empty defeats its own count assertion

**What goes wrong:** Success criterion 2 requires "emptying the directory makes the test fail, because the count assertion refuses a loop that proves nothing." If plan 05-05 lands the test harness before any crash has ever been found (which is the normal order — the harness must exist before the fuzzer that will eventually populate it runs in CI), the directory has zero files at that commit, and the count assertion must therefore fail on that same commit unless at least one seed file is checked in immediately.
**Why it happens:** "Committed regression test that replays" implies files arrive from fuzzing; but fuzzing has not run yet when the harness is first built, and the roadmap's own wave order (`[05-01, 05-02, 05-03, 05-07]` then `[05-04, 05-05, 05-06]`) puts 05-05 in the second wave, after the fuzz crate exists in 05-03 but very likely before it has found anything.
**How to avoid:** Plan 05-05 must include, in the same commit as the test harness, at least one hand-built or corpus-derived truncated/malformed file as the first `tests/regressions/` entry — a file that is already known to be handled correctly (a regression test for a fix that already happened, e.g., a truncated version of an existing corpus file), so the directory is never empty at any commit after this one.

### Pitfall 3: The `-max_total_time` vs. `-runs=N` choice is not free — and the roadmap's own success criterion 1 already picks one

**What goes wrong:** The roadmap's own named risk says "Choose one deliberately," which can read as leaving the choice open — but success criterion 1 already gives the literal PR-job command: `cargo +nightly fuzz run --fuzz-dir crates/deform6/fuzz parse -- -max_total_time=60 -rss_limit_mb=2048`. A plan that reinterprets this as "use `-runs=N` for the PR job instead" contradicts an already-fixed success criterion.
**Why it happens:** The named risk's own prose ("A wall clock bound is a flaky gate on a varying CI machine... Choose one deliberately") reads as general fuzzing advice and can be mistaken for an open decision affecting the PR job, when success criterion 1 has already made that specific decision.
**How to avoid:** Read the named risk as applying to the **second** job the roadmap's own plan 05-04 title names ("the bounded pull request run, **the longer cron run**") — the PR job's exact command is locked by success criterion 1 (verified this session, libFuzzer's own defaults: `-max_total_time` default is `0`, meaning "run indefinitely," and `-runs` default is `-1`, meaning the same [CITED: LLVM LibFuzzer docs, WebSearch this session] — so *some* bound is mandatory on any CI job regardless of which flag is chosen). The cron job, which has no success-criterion-mandated command, is the one place `-runs=N` (a fixed iteration count, deterministic regardless of CI machine speed) is the better choice, and where the named risk's "choose deliberately" genuinely applies.
**Warning signs:** A plan that gives both jobs the identical flag set has not actually made the deliberate choice the named risk asks for.

### Pitfall 4: `-rss_limit_mb=2048` looks redundant with libFuzzer's own default, but the named risk requires it stated anyway

**What goes wrong:** Verified this session: libFuzzer's own default for `-rss_limit_mb` is already `2048` [CITED: LLVM LibFuzzer docs, WebSearch this session]. A plan might reasonably drop the explicit flag as "redundant with the default" — but the roadmap's own named risk states "`-rss_limit_mb` is the check for an allocation sized from a length field. That is a named project constraint, so set the value explicitly rather than take the default," precisely so that a future libFuzzer version changing its own default does not silently change this project's safety bound without anyone noticing in a diff.
**How to avoid:** Keep the flag explicit in both the `xtask` command construction and the CI workflow file, even though its current numeric value happens to match libFuzzer's own default.

### Pitfall 5: `corpus/manifest.toml`'s pinned SHA-256 says nothing about whether the URL still resolves

**What goes wrong:** A silent fetch failure (404, redirect to a fork, repository deleted) that the fetch code treats as "skip this entry" turns the run-time robustness corpus into a set of zero files — a set of zero files trivially "passes" any no-panic check, exactly the named risk's own words: "A silent skip turns the robustness set into a set of zero files that passes."
**Why it happens:** The natural, easy implementation of a fetch loop is `for entry in manifest { if fetch(entry).is_err() { continue; } }`, which is exactly the shape that produces this failure silently.
**How to avoid:** `xtask fetch-corpus` must fail the whole command (non-zero exit) on any single fetch failure or hash mismatch, and success criterion 5's own "the run prints the number of inputs it read, and that number equals the number of files that exist" must be checked against `corpus/manifest.toml`'s own declared entry count, not merely against however many files happen to be sitting in `corpus/fetched/` when the no-panic run starts.

## Code Examples

### The `Journal`/`Mode` type, already shipped, in full

```rust
// Source: crates/deform6/src/journal.rs:22-72 (VERIFIED, already shipped, fully tested)
use crate::error::{Defect, Error, Severity};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Strict,
    Salvage,
}

pub struct Journal {
    mode: Mode,
    defects: Vec<Defect>,
}

impl Journal {
    pub const fn new(mode: Mode) -> Self { /* ... */ }
    pub fn defects(&self) -> &[Defect] { &self.defects }

    pub fn record<T>(&mut self, defect: Defect, fallback: T) -> Result<T, Error> {
        self.defects.push(defect.clone());
        match (defect.kind.severity(), self.mode) {
            (Severity::Fatal, _) => Err(Error::Refused(defect)),
            (Severity::Recoverable, Mode::Strict) => Err(Error::Refused(defect)),
            (Severity::Recoverable, Mode::Salvage) => Ok(fallback),
        }
    }
}
```
Note `Journal::record` returns `Result<T, crate::error::Error>`, **not** `Result<T, Refusal>` — `Error::Refused(Defect)` is a different type from the public `Refusal` enum `inspect`/`write::project` currently return. Plan 05-01 must decide how `Error` maps to `Refusal` at the point `inspect`/`write::project` propagate a `Journal::record` failure outward (there is no existing `From<Error> for Refusal` impl; a grep this session for `impl From<Error>` in `error.rs` finds none) — this is a second concrete open design point beyond the `Mode`-vs-`&mut Journal` parameter question (see Assumption A1).

### `ImplausibleCount`'s existing message shape

```rust
// Source: crates/deform6/src/error.rs:80-89 (VERIFIED, already shipped)
/// A count field asks for more items than the file holds.
#[error("count {count} at offset {offset:#x} exceeds the {max} that the file can hold")]
ImplausibleCount {
    offset: u32,
    count: u32,
    max: u32,
},
```
Severity: `Recoverable` (`error.rs:319`). This is already the exact shape success criterion 4 needs; `gui.rs::GuiTable::walk` needs a call site that constructs one, not a new variant.

### The already-correct bound-and-clamp pattern (full function, for direct reuse as a template)

```rust
// Source: crates/deform6/src/vb/object.rs:279-306 (VERIFIED, already shipped)
fn bound_proc_count(
    pe: &PeImage<'_>,
    element: &Region<'_>,
    lp_proc_names_array: Va,
    raw_proc_count: u32,
) -> (u32, Option<Defect>) {
    let Some(array_region) = pe.region_at_va(lp_proc_names_array) else {
        return (raw_proc_count, None);
    };
    let max_entries = array_region.len().checked_div(PROC_NAME_PTR_SIZE).unwrap_or(0);
    if raw_proc_count <= max_entries {
        return (raw_proc_count, None);
    }
    let offset = element.file_offset(Off::new(0x1C)).map_or(0, Off::get);
    let defect = Defect {
        site: Site { offset, rva: None, structure: "Object", field: "ProcCount" },
        kind: DefectKind::ImplausibleCount { offset, count: raw_proc_count, max: max_entries },
    };
    (max_entries, Some(defect))
}
```

### The crash-to-regression-test procedure (SAF-05, plan 05-05), following `cargo-fuzz`'s own convention

```sh
# cargo-fuzz already writes a crashing input to
# crates/deform6/fuzz/artifacts/parse/crash-<hash> when a fuzz run finds one.
# The committed procedure (documented in a new doc comment, not a new tool):

cp crates/deform6/fuzz/artifacts/parse/crash-<hash> \
   crates/deform6/tests/regressions/<short-descriptive-name>

# tests/regressions.rs (new) then walks the directory, replaying each file
# through both Strict and Salvage, asserting neither panics, and naming the
# file on failure -- the same shape crates/deform6/tests/corpus_sweep.rs
# already uses to walk the vendored corpus (read this session for the
# pattern; corpus_sweep.rs's own directory-walk helper is a candidate for
# direct reuse rather than a second directory-walking implementation).
```

## State of the Art

Not applicable in the "library X was superseded by Y" sense for the VB6 format itself (unchanged since 2008). For the tooling: `cargo-fuzz` 0.13.2 and libFuzzer are the current, actively maintained state of the art for Rust fuzzing as of this session (`rust-fuzz` org, last README update reflected in the fetched content this session). No newer alternative (e.g., a hypothetical stable-Rust-only fuzzer) displaces the nightly-only libFuzzer requirement; this remains the reason the fuzz crate must stay excluded from the stable-toolchain-gated workspace.

**Deprecated/outdated:** None found specific to this phase's stack.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The `Journal` retrofit should thread a `Mode` parameter through `inspect`/`write::project` and down to each of the ~24 call sites (each site constructing its own short-lived `Journal` or sharing one built at the top), rather than restructuring every function to accept and return a shared `&mut Journal` across the whole call graph | Architecture Patterns, Pattern 1 and 3; Common Pitfalls, Pitfall 1 | If the planner instead chooses a single `Journal` threaded by mutable reference through the entire `inspect` call graph, every one of the ~24 function signatures changes shape differently (adding a `&mut Journal` parameter and typically dropping their own local `Vec<Defect>` return value entirely) than the `Mode`-only design (which could let each function keep building its own local `Journal` from a `Mode` it receives, then merge `.defects()` afterward, a smaller per-function diff). Both are viable; this document does not adjudicate between them because the codebase does not yet show a precedent for either at this scale. The planner must pick one explicitly, since it changes the signature of every one of the ~24 functions differently. |
| A2 | A `--salvage`-only "assumption" (SAF-03) should be recorded as a new, salvage-specific field or reuse the existing `ProjectReport.limits: Vec<String>` free-text list / `Confidence::Inferred` `ReportItem` vocabulary, rather than requiring a new typed "Assumption" struct | Phase Requirements, SAF-03 | If the planner adds a new typed field instead of reusing `limits`/`Confidence`, the JSON report schema gains a new top-level shape Phase 4's own consumers (and any future Phase 6 documentation of the report schema) do not expect; reusing the existing vocabulary keeps RPT-01 through RPT-06 (Phase 4, already `[x]`) unchanged. This is a real design choice the plan must make explicitly, not a fact this research can verify from code that does not exist yet. |
| A3 | `Error::Refused(Defect)` (the type `Journal::record` returns) needs a new `From<Error> for Refusal` (or an equivalent explicit mapping function) added to `error.rs`, since none exists today | Code Examples, "The `Journal`/`Mode` type" | Low-medium risk: this is a small, mechanical addition, but if missed, `inspect`/`write::project`'s public signature (`Result<Report, Refusal>`) cannot compile against a `Journal::record` call inside them without an explicit conversion at every call site, which would be a much larger, repetitive diff than one shared `From` impl. |
| A4 | `tests/regressions/` needs at least one seed file checked in in the same commit as the harness (plan 05-05), rather than starting genuinely empty and accepting a failing gate until the fuzzer finds its first crash | Common Pitfalls, Pitfall 2 | If wrong (i.e., if the intended design really is "the gate is red until the first crash is found and committed"), the phase cannot reach a green gate through its own normal wave order, which contradicts every other phase's practice in this project of landing each plan with a passing gate. |

## Open Questions

1. **Does the fuzz target need `arbitrary`, or is a raw `&[u8]` sufficient?**
   - What we know: `deform6::inspect(data: &[u8], opcode_table: &OpcodeTable, ...)` already takes a raw byte slice as its primary input; libFuzzer's `fuzz_target!(|data: &[u8]| ...)` form needs no `Arbitrary` derive at all for this shape.
   - What's unclear: whether the target should also fuzz which `OpcodeTable` is used (the builtin vs. a corrupted user-supplied one) or hold that fixed at `OpcodeTable::builtin()` for the whole campaign.
   - Recommendation: hold the opcode table fixed at `OpcodeTable::builtin()` for plan 05-03; `OpcodeTable::parse` (a hostile-file-shaped input in its own right, from `--opcode-table`) is a second, independent input surface Phase 3 already built and could get its own fuzz target in a later milestone, but the roadmap's own success criterion 1 names exactly one target, `parse`, and does not ask for a second.

2. **Where does the `--salvage` flag live on the CLI: a global flag, or per-subcommand?**
   - What we know: `deform6-cli`'s `Command` enum currently has `Inspect` and `Extract` as two independent subcommand variants (`main.rs:57-118`, read in full this session), each with its own argument list; the roadmap's own plan 05-01 title says "`--salvage` on both subcommands."
   - What's unclear: whether `--salvage` is declared identically on both `Inspect` and `Extract` (two separate `#[arg(long)] salvage: bool` fields, one per variant) or hoisted to a shared parent (clap supports a `#[command(flatten)]` shared-args struct, which this codebase does not use anywhere yet).
   - Recommendation: given this codebase's existing convention of independent, non-flattened per-variant argument lists (every existing flag — `opcode_table`, `output`, `report`, `force` — is declared directly on its own variant, none shared), add `#[arg(long)] salvage: bool` independently to both `Inspect` and `Extract`, matching the established pattern rather than introducing `#[command(flatten)]` as this phase's one new clap idiom.

3. **Does `xtask fetch-corpus`'s manifest format need a schema beyond "path, URL, SHA-256"?**
   - What we know: the named risk says "The manifest pins a SHA-256 for each program but cannot pin availability," and `AGENTS.md`'s "what may enter this repository" rule requires a licence-permits-redistribution check only for `corpus/` proper (the vendored, committed set) — the fetched set is explicitly the "unlicensed pile," per `PROJECT.md`'s own Key Decisions table ("Vendor the permissive corpus, fetch the rest at run time").
   - What's unclear: whether `corpus/manifest.toml` needs a licence/origin field at all (since these files are never committed and `corpus/NOTICES`'s own stated scope is "every program in `corpus/`," i.e., the committed set only), or whether some minimal provenance note is still useful for a human auditing what a CI run downloaded.
   - Recommendation: keep the manifest minimal (a table keyed by program name, each entry giving a URL and a SHA-256 hex string) unless the discuss-phase step surfaces a stronger requirement; this is a Claude's-discretion-shaped design choice, not a fact this research needs to adjudicate, since no CONTEXT.md exists for this phase to record a locked decision either way.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` / Rust toolchain (stable) | Everything in this phase | ✓ | edition 2024, `rust-version = "1.97.1"`, pinned in `rust-toolchain.toml` (verified this session) | - |
| Rust nightly toolchain | `cargo fuzz build`/`run` (plan 05-03, 05-04) | Not verified installed on this development machine or the CI runner image; `rustup toolchain install nightly` is a standard, low-risk step | latest nightly (cargo-fuzz tracks no specific pin) | None needed — installing nightly alongside the pinned stable channel is the standard, expected setup for any `cargo-fuzz` user; `rustup` supports multiple installed toolchains natively. |
| `cargo-fuzz` binary | Plans 05-03, 05-04 | Not verified installed; installed via `cargo install cargo-fuzz` | 0.13.2 current on crates.io | None needed — a one-line CI step (`cargo install cargo-fuzz`) fully resolves this; not a blocking gap. |
| Network access (for `xtask fetch-corpus`) | Plan 05-07, 05-08 | Assumed available in CI (the existing `gate.yml` already fetches `actions/checkout`, `actions/cache` from the network) | - | If the run-time corpus source disappears (named risk), the fetch must fail loudly per Common Pitfalls Pitfall 5; there is no fallback that keeps the robustness set meaningful with zero files. |

**Missing dependencies with no fallback:** none — every gap above has a standard, low-risk resolution (installing a toolchain or a cargo subcommand), not a structural blocker.

**Missing dependencies with fallback:** nightly toolchain and `cargo-fuzz` binary, both resolved by a one-line CI/local install step.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test --workspace` (Rust's built-in harness) for everything except the fuzz crate itself, which uses `cargo +nightly fuzz run`/`cargo +nightly fuzz build` outside `cargo test` entirely |
| Config file | none — `AGENTS.md`'s gate is exactly `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, unchanged by this phase; the fuzz crate is structurally excluded from all three by `[workspace] exclude` |
| Quick run command | `cargo test --workspace -p deform6 journal::` (once retrofit lands) or `cargo test --workspace -p deform6 --test regressions` (once 05-05 lands) |
| Full suite command | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` (unchanged), plus, out-of-band, `cargo +nightly fuzz run --fuzz-dir crates/deform6/fuzz parse -- -max_total_time=60 -rss_limit_mb=2048` (success criterion 1's own command) |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SAF-01 | No panic on any input | fuzz + regression replay | `cargo +nightly fuzz run --fuzz-dir crates/deform6/fuzz parse -- -max_total_time=60 -rss_limit_mb=2048` and `cargo test --workspace --test regressions` | ❌ Wave 0 (05-03, 05-05) |
| SAF-02 | Refuses by default, names offset and expectation | unit, per retrofitted call site | `cargo test --workspace -p deform6 vb::gui::` (and per other retrofitted module) | ❌ Wave 0 (05-01) |
| SAF-03 | `--salvage` recovers, marks assumptions | integration, both subcommands | `cargo test --workspace -p deform6-cli` (new salvage-flag tests) | ❌ Wave 0 (05-01) |
| SAF-04 | No allocation sized before a bound check | unit, one per audited count field | `cargo test --workspace -p deform6 vb::gui::gui_table_refuses_an_implausible_form_count` (new, named for the exact roadmap example) | ❌ Wave 0 (05-02) |
| SAF-05 | Fuzzer in the gate, crashes become committed tests | CI job + regression replay | `.github/workflows/fuzz.yml` (new) + `cargo test --workspace --test regressions` | ❌ Wave 0 (05-03, 05-04, 05-05) |
| Roadmap SC4 | `wFormCount = 0xFFFF` in a 4 KB file refused with `ImplausibleCount`, allocates nothing | unit, literal byte fixture | `cargo test --workspace -p deform6 vb::gui::` (new, fixture built inside the test per `AGENTS.md`'s "build the state a test needs inside the test") | ❌ Wave 0 (05-02) |
| Roadmap SC5 | Whole corpus + fetched set + regressions, `panic = "abort"`, no abort | integration, corpus-wide | new `xtask` command or `--test` binary walking `corpus/`, `corpus/fetched/`, `tests/regressions/` | ❌ Wave 0 (05-08) |

### Sampling Rate

- **Per task commit:** the narrowest `cargo test --workspace -p deform6 <module>::` slice for the module just retrofitted or audited.
- **Per wave merge:** the full three-command gate, unchanged, per `AGENTS.md`.
- **Phase gate:** full gate green, plus the PR-bounded fuzz run (success criterion 1) green, plus the full no-panic proof run (success criterion 5) green, before `/gsd-verify-work`.

### Wave 0 Gaps

- [ ] `crates/deform6/fuzz/` — does not exist yet; created by `cargo fuzz init`, not hand-written (05-03).
- [ ] `crates/deform6/tests/regressions.rs` and `crates/deform6/tests/regressions/` — do not exist yet; the directory needs at least one seed file at creation time (Pitfall 2) (05-05).
- [ ] `corpus/manifest.toml` — does not exist yet (05-07).
- [ ] `.github/workflows/fuzz.yml` — does not exist yet; `.github/workflows/gate.yml` is the only workflow file today (05-04).
- [ ] `crates/xtask/src/fetch_corpus.rs` and a new `fuzz-ci` (or similarly named) `xtask` subcommand — do not exist yet (05-04, 05-07).
- [ ] Framework install: `cargo install cargo-fuzz`, `rustup toolchain install nightly` — neither is confirmed present in the CI runner image or verified on this development machine.

*(Every test file and CI job this phase needs is new; only the `Journal`/`ImplausibleCount` machinery it wires into is pre-existing.)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | offline CLI tool, no auth surface |
| V3 Session Management | no | no sessions |
| V4 Access Control | no | single-user local CLI |
| V5 Input Validation | yes | This entire phase *is* the input-validation hardening pass: every count/length field bound-checked before use (SAF-04), every panic-shaped path closed (SAF-01), a fuzzer continuously probing for the ones this research and any human audit missed (SAF-05). |
| V6 Cryptography | yes (new in this phase) | `sha2` for corpus manifest hash verification — the one legitimate cryptographic primitive this phase introduces (integrity checking, not confidentiality); `sha2` is the correct, non-hand-rolled choice per "Don't Hand-Roll" above. |
| V12 File and Resources | yes | `xtask fetch-corpus` writes fetched bytes to `corpus/fetched/`; this is a new file-write surface (unlike Phase 4's `extract`, which writes recovered project files, this writes third-party downloaded bytes) and should reuse the same path-containment discipline `deform6-cli::plan_writes`/`lexically_normalize` already established in Phase 4, applied to the manifest-declared destination path, not a name recovered from a hostile executable. |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A hand-crafted count field (`wFormCount`, `dwExternalCount`, a property stream length, a `.frx` blob length) drives an unbounded allocation | Denial of Service | The `bound_proc_count`/`ImplausibleCount` pattern, generalized to every count field the 05-02 audit finds (see Architecture Patterns, Pattern 1). |
| Two file-derived offsets summed with `+` overflow and wrap to point back inside the file, defeating a bounds check that ran before the overflow | Tampering | Already crate-wide policy (`AGENTS.md`: "Do not add two offsets that come from the file with `+`. Use `checked_add`") and already the exclusive pattern in `read/region.rs` (verified this session: every offset arithmetic site uses `checked_add`/`checked_sub`, never a bare `+`); Phase 5's fuzz target is the mechanism that *proves* no call site has silently reintroduced a bare `+`, not a new mitigation to design. |
| A fetched run-time corpus URL is compromised (supply-chain substitution) between manifest authoring and CI fetch time | Tampering | The pinned SHA-256 in `corpus/manifest.toml` is exactly this mitigation: a hash mismatch after fetch must fail the whole `xtask fetch-corpus` run loudly (Common Pitfalls, Pitfall 5), never silently substitute or skip the file. |
| A crash artifact `cargo fuzz` writes to `crates/deform6/fuzz/artifacts/` contains sensitive process memory (a libFuzzer artifact is a copy of the fuzzer-generated input, not process memory, but this is worth stating explicitly since the artifact is a candidate for committing to `tests/regressions/`) | Information Disclosure | Low risk in practice — the artifact is fuzzer-generated bytes, not the target process's actual memory contents — but the crash-to-test procedure (Code Examples) should note that a crash artifact is reviewed before committing it to `tests/regressions/`, the same way any input a human did not author is reviewed before it enters version control. |

## Sources

### Primary (HIGH confidence)

- `crates/deform6/src/journal.rs` (whole file read this session) — `Mode`, `Journal`, `Journal::record`, full test suite
- `crates/deform6/src/error.rs` (whole file read this session) — `DefectKind` (all sixteen variants), `Severity`, `Defect`, `Refusal`, `Error`, `damaged`
- `crates/deform6/src/vb/object.rs` (lines 270-320 read this session) — `bound_proc_count`, the canonical correct bound-check pattern
- `crates/deform6/src/vb/gui.rs` (lines 54-153 read this session) — `GuiTable::walk`, the located gap
- `crates/deform6/src/vb/project.rs` (lines 490-590 read this session) — `DeclareTable::read`, a second correct bound-check instance
- `crates/deform6/src/vb/functyp.rs`, `crates/deform6/src/vb/privateobj.rs` (grep + targeted line reads this session) — all six `Vec::with_capacity` sites, traced to their bounding logic
- `crates/deform6/src/write/model.rs` (lines 1-50 read this session), `crates/deform6/src/report.rs` (whole file read this session) — `ProjectReport`, `Confidence`, `ReportItem`, `Evidence`, `limits`
- `crates/deform6-cli/src/main.rs` (whole file read this session) — `Command`, `Cli`, the exit-code table, `run_inspect`, `run_extract`
- `crates/xtask/src/main.rs` (whole file read this session) — the existing subcommand pattern (`update-ratios`, `derive-opcode-table`)
- `Cargo.toml` (root, read this session) — `exclude`, `[profile.release] panic = "abort"`, `[workspace.dependencies]`
- `rust-toolchain.toml` (read this session) — `channel = "1.97.1"`
- `.github/workflows/gate.yml` (whole file read this session) — the only existing CI workflow
- `corpus/NOTICES` (head read this session) — the existing vendored-corpus provenance record
- `.planning/ROADMAP.md` Phase 5 section (read this session) — success criteria, named risks, plan list, waves, verbatim
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `AGENTS.md`, `.planning/PROJECT.md` (fully read this session)
- gsd-tools `package-legitimacy check` seam (run this session) — `cargo-fuzz`, `libfuzzer-sys`, `arbitrary`, `ureq`, `sha2`, all `OK`
- `cargo search cargo-fuzz` / `libfuzzer-sys` / `arbitrary` / `ureq` / `sha2` (run this session against the live crates.io registry)

### Secondary (MEDIUM confidence)

- `rust-fuzz/cargo-fuzz` README (fetched this session via WebFetch) — `--fuzzing-workspace` default, the nightly requirement
- LLVM LibFuzzer documentation (searched this session via WebSearch, aggregated from multiple LLVM release-doc mirrors) — `-runs`, `-max_total_time`, `-rss_limit_mb` defaults and meaning

### Tertiary (LOW confidence)

- None used directly; every claim above traces to a file opened directly this session or a tool call (`cargo search`, the gsd-tools legitimacy seam, WebFetch/WebSearch against the project's own upstream docs) run this session.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every package verified against both the gsd-tools legitimacy seam and the live crates.io registry this session.
- Architecture: HIGH — every claim about what exists in the codebase today (`Journal`, `ImplausibleCount`, the `with_capacity` sites, the CLI shape) was read directly this session, with line numbers, not inferred.
- Pitfalls: HIGH for Pitfalls 1, 3, 4, 5 (each traces to a specific, quoted, already-shipped fact or a verified external flag default); MEDIUM for Pitfall 2 (the *problem* — a count assertion on an empty directory — is a direct reading of success criterion 2's own text, but the *exact fix* — seed the directory at harness-creation time — is this document's own recommendation, logged as Assumption A4).

**Research date:** 2026-09-13
**Valid until:** No expiry driven by the VB6 format itself. Re-verify the `cargo-fuzz`/libFuzzer flag claims if more than a few months pass before this phase executes (fuzzing tooling version churn is faster than this project's other dependencies), and re-verify every `Journal`/`ImplausibleCount`/`with_capacity` line reference if any of Phase 1-4's code changes before Phase 5 executes, since this document's HIGH-confidence architectural claims are pinned to exact line numbers as of 2026-09-13.
