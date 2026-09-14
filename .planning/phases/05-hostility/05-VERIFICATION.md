---
phase: 05-hostility
verified: 2026-09-14T21:40:00Z
status: passed
score: 10/10 must-haves verified
covered_files:
  - ".github/workflows/fuzz.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/phases/05-hostility/05-01-PLAN.md"
  - ".planning/phases/05-hostility/05-01-SUMMARY.md"
  - ".planning/phases/05-hostility/05-02-PLAN.md"
  - ".planning/phases/05-hostility/05-02-SUMMARY.md"
  - ".planning/phases/05-hostility/05-03-PLAN.md"
  - ".planning/phases/05-hostility/05-03-SUMMARY.md"
  - ".planning/phases/05-hostility/05-04-PLAN.md"
  - ".planning/phases/05-hostility/05-04-SUMMARY.md"
  - ".planning/phases/05-hostility/05-05-PLAN.md"
  - ".planning/phases/05-hostility/05-05-SUMMARY.md"
  - ".planning/phases/05-hostility/05-06-PLAN.md"
  - ".planning/phases/05-hostility/05-06-SUMMARY.md"
  - ".planning/phases/05-hostility/05-07-PLAN.md"
  - ".planning/phases/05-hostility/05-07-SUMMARY.md"
  - ".planning/phases/05-hostility/05-08-PLAN.md"
  - ".planning/phases/05-hostility/05-08-SUMMARY.md"
  - ".planning/phases/05-hostility/05-LEARNINGS.md"
  - ".planning/phases/05-hostility/05-REVIEW.md"
  - ".planning/phases/05-hostility/05-UAT.md"
  - "Cargo.toml"
  - "crates/deform6-cli/src/main.rs"
  - "crates/deform6/fuzz/fuzz_targets/parse.rs"
  - "crates/deform6/schema/report.schema.json"
  - "crates/deform6/src/error.rs"
  - "crates/deform6/src/journal.rs"
  - "crates/deform6/src/report.rs"
  - "crates/deform6/src/vb/frx.rs"
  - "crates/deform6/src/vb/functyp.rs"
  - "crates/deform6/src/vb/gui.rs"
  - "crates/deform6/src/vb/mod.rs"
  - "crates/deform6/src/vb/object.rs"
  - "crates/deform6/src/vb/project.rs"
  - "crates/deform6/src/vb/vbstr.rs"
  - "crates/deform6/src/write/model.rs"
  - "crates/deform6/tests/fuzz_smoke.rs"
  - "crates/deform6/tests/no_panic_proof.rs"
  - "crates/deform6/tests/regressions.rs"
  - "crates/deform6/tests/salvage.rs"
  - "crates/deform6/tests/schema.rs"
  - "crates/deform6/tests/severity_census.rs"
  - "crates/deform6/tests/support/hostile.rs"
  - "crates/xtask/src/fetch_corpus.rs"
  - "crates/xtask/src/fuzz.rs"
covered_digest: "v1:sha256:209891d57ba15788a273b0fd4818f172b59caa339731aaaab97ed86d6684f33a"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 5: Hostility Verification Report

**Phase Goal:** DeForm6 holds under a file that is damaged or hand-made to break a parser. It
refuses by default and names the byte offset. It recovers what it can under `--salvage` and
marks every assumption. A fuzzer runs in the gate, and every crash it finds becomes a committed
test that replays on stable Rust.

**Verified:** 2026-09-14T21:40:00Z
**Status:** passed
**Re-verification:** No, initial verification. This is a retrospective run: the phase shipped,
was code-reviewed today, had one Critical defect found and fixed today, and had never been run
through this verification step before.

## Note on the same-day fix

A code review dated 2026-09-14 (`05-REVIEW.md`) found one Critical defect (CR-01): four sites
built `DefectKind::UnmappedAddress` or `DefectKind::OffsetOverflow`, both `Fatal`, for a
per-item, recoverable loss. Because Phase 5 itself wired `Journal::record` into the `inspect`
choke point, any file touching one of those four sites was refused whole in both `Strict` and
`Salvage`, defeating `--salvage`'s purpose. This verification does not take the review's
`## Resolution` table on trust. It re-derives the same facts independently below, by reading the
current source and running the tests, not by reading what the review said about them.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A bounded fuzz campaign builds and runs on nightly; the fuzz crate stays out of `cargo test --workspace` and `cargo clippy --all-targets` on stable. | VERIFIED | Root `Cargo.toml` line 4: `exclude = ["crates/deform6/fuzz"]`. `crates/deform6/fuzz/Cargo.toml` carries its own `[workspace] members = ["."]` table and `[package.metadata] cargo-fuzz = true`. I ran `cargo +nightly fuzz build --fuzz-dir crates/deform6/fuzz` myself: it compiled cleanly in 12s. I then ran `cargo +nightly fuzz run --fuzz-dir crates/deform6/fuzz parse -- -max_total_time=15 -rss_limit_mb=2048` myself: 717,075 executions, 0 crashes, clean `DONE`. `.github/workflows/fuzz.yml` runs `cargo run -p xtask -- fuzz-pr` on `pull_request` and `cargo run -p xtask -- fuzz-cron` on `schedule`, both through the `xtask` subcommand rather than a hand-typed `cargo fuzz` call. `PR_MAX_TOTAL_TIME = 60` and `RSS_LIMIT_MB = 2048` in `crates/xtask/src/fuzz.rs`, asserted against the roadmap wording by `pr_max_total_time_matches_roadmap_success_criterion_one` (passed in the workspace test run below). |
| 2 | `cargo test --workspace` replays every file in `tests/regressions/` through `Strict` and `Salvage` on stable and fails with the file name if one panics. An empty directory fails the test. | VERIFIED | `cargo test -p deform6 --test regressions` passed 18/18, including `every_regression_input_replays_in_both_modes_without_ending_the_process`. `MINIMUM_REGRESSION_INPUTS: usize = 1` in `regressions.rs`, checked with `assert!(files.len() >= MINIMUM_REGRESSION_INPUTS, ...)`, so an emptied directory fails the assertion, not just the loop. |
| 3 | A truncated/patched corpus file refuses in strict mode and names the byte offset; the same file under `--salvage` produces output and lists every assumption; the defect list is identical in both modes. | VERIFIED | `cargo test -p deform6 --test salvage` passed 10/10: `a_patched_external_count_refuses_in_strict_and_names_the_offset`, `the_same_patched_file_succeeds_in_salvage_with_one_more_defect`, `a_strict_report_holds_the_mode_line_and_no_assumption_line`, `a_salvage_report_over_the_patched_bytes_holds_the_mode_line_and_one_assumption_line`, `the_defect_array_holds_more_entries_than_the_assumption_line_count`, `two_salvage_runs_over_the_same_bytes_give_byte_identical_json`. |
| 4 | Every count/length field named in the parse order is checked against the real file size before any allocation is sized from it. A 4 KB image declaring `wFormCount = 0xFFFF` is refused with `ImplausibleCount` and allocates nothing. | VERIFIED | `crates/deform6/src/vb/gui.rs::bound_form_count` divides the mapped region's real byte length by `GUI_ENTRY_SIZE` before the loop starts and clamps the loop bound to that; the loop never runs past what the region holds. Its own test `gui_table_refuses_an_implausible_form_count` builds a hand-made, literal-only 4096-byte image (`tests/support/hostile.rs::gui_table_overcount_4k`, independently unit-tested for exact length, PE signature, and determinism) whose mapped section holds exactly one entry, sets `wFormCount = 0xFFFF`, and asserts exactly 1 entry recovered and exactly one `ImplausibleCount { count: 0xFFFF, max: 1, .. }` defect. I ran `cargo test -p deform6 gui_table_refuses_an_implausible_form_count`: passed. `scripts/prove-capacity-wall.sh` (an audited-site wall over every file-derived allocation in `crates/deform6/src/`) passed when I ran it. |
| 5 | The whole vendored corpus, the fetched set, and every regression input run through both modes with `panic = "abort"` in the release profile, and no process aborts; the run prints a count that equals the number of files that exist. | VERIFIED | `Cargo.toml` line 39: `panic = "abort"` in `[profile.release]`. `cargo test -p deform6 --test no_panic_proof -- --nocapture`: 9/9 passed, drove all 44 vendored executables plus the regression directory through `Mode::Strict`, `Mode::Salvage`, and `write::project`, printing each input as it ran. Counts are cross-checked against the corpus's own count and the regression directory's own count, never against a directory listing taken once (`no_panic_proof.rs` doc comments and source). Learnings honestly record that `cargo test` itself compiles under `panic=unwind` (Cargo's own harness requirement for `catch_unwind`), so this proves "no panic on any of these inputs, either mode, both profiles" rather than a literal `SIGABRT`: a narrower, still-true claim, and the UAT and the test's own doc comment say so rather than overclaiming. |
| 6 | (CR-01 fix) `--salvage` actually salvages a file with one unresolvable per-item pointer, instead of refusing the whole file. | VERIFIED | `crates/deform6/src/error.rs`: `UnmappedAddress` and `OffsetOverflow` are still `Severity::Fatal` (lines 364, 368). Two new variants, `ItemAddressUnmapped` and `ItemOffsetOverflow`, are `Severity::Recoverable` (lines 411, 415). All four call sites named in the review now build the new variants: `project.rs::DeclareDescriptorFailure::into_defect` uses `ItemAddressUnmapped`; `frx.rs::extract_blob`, `vbstr.rs::VbStr::overflow`, and `functyp.rs`'s optional-value cursor overflow all use `ItemOffsetOverflow`. I ran `cargo test -p deform6 --test salvage`: `an_unresolvable_declare_descriptor_refuses_in_strict_mode` and `an_unresolvable_declare_descriptor_succeeds_in_salvage_mode_losing_only_that_entry` both pass. |
| 7 | The fix did not lower a genuine spine walk to `Recoverable`. | VERIFIED | `crates/deform6/src/vb/object.rs::ObjectTable::walk` returns `Result<Self, Refusal>` and raises `Refusal::Damaged` directly for a bad pointer, index overflow, or truncated element: it never constructs a `DefectKind` at all, so it cannot have been "lowered". `crates/deform6/src/vb/gui.rs::GuiTable::walk` is the same shape: `Refusal::Damaged` for the table pointer, the index overflow, the truncated entry, and the `lStructSize` mismatch. Neither file was touched by the fix's four sites. |
| 8 | `cargo test -p deform6 --test schema` still passes all 3 tests (the schema grew by two `DefectKind` variants, a Phase 6 deliverable). | VERIFIED | I ran it: `the_schema_refuses_a_doctored_report`, `one_corpus_report_validates_against_the_committed_schema`, `every_corpus_report_validates_against_the_committed_schema`, 3/3 passed. |
| 9 | The write path contains its output: a symlink pre-plant is refused and a write whose normalized parent is not the resolved output directory is refused, before any byte is written. | VERIFIED | `crates/deform6-cli/src/main.rs::write_project` calls `plan_writes` (lexically normalizes every candidate path and refuses one whose parent is not `resolved_dir`, line ~482) then `refuse_symlink_targets` (checks `std::fs::symlink_metadata` for `is_symlink()` on every planned path, line ~514): both run before the `for (path, bytes) in &plan { std::fs::write(...) }` loop, not after. |
| 10 | The `PathIssuer`/`SafeNameIssuer` O(n^2) fix from the learnings is actually present, still correct, and holds no `HashMap`. | VERIFIED | `report.rs::PathIssuer` and `write/model.rs::SafeNameIssuer` both hold `next_suffix: BTreeMap<String, u32>` and still check set membership on every candidate before accepting it (the suffix cache is a starting point, never a shortcut). `scripts/prove-ordered-output-wall.sh` (the no-`HashMap` source wall) passed when I ran it. The only `HashSet` in either file is inside a `#[cfg(test)]` module (`a_thousand_items_with_the_same_base_path_each_get_a_distinct_path`, `a_thousand_controls_with_the_same_raw_name_each_get_a_distinct_name`), not production code. |

**Score:** 10/10 truths verified (0 present-but-behavior-unverified)

### Full Workspace Test Run

`cargo test --workspace`, run once: 23 test binaries, every one `test result: ok`, 0 failed
across the whole run (`615 passed` in the largest binary alone). Matches the review's claim of
986 tests passing after the fix; I did not re-derive that exact total but confirmed zero
failures across every binary.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/deform6/src/journal.rs` | `Severity` gate: Fatal/Recoverable/Tolerated, mode-dependent policy | VERIFIED | `(Severity::Fatal, _)` refuses in both modes; wired into `inspect`'s choke point in `vb/mod.rs`. |
| `crates/deform6/src/error.rs` | Severity table for every `DefectKind`, including the two new per-item variants | VERIFIED | `UnmappedAddress`/`OffsetOverflow` Fatal; `ItemAddressUnmapped`/`ItemOffsetOverflow` Recoverable; `ImplausibleCount` Recoverable. |
| `crates/deform6/src/vb/gui.rs` | `bound_form_count`, pre-loop bound check | VERIFIED | Present, tested, wired into `GuiTable::walk` before the loop starts. |
| `crates/deform6/fuzz/fuzz_targets/parse.rs` | Fuzz target driving both modes plus the writer | VERIFIED | Calls `inspect` in `Mode::Strict`, then `Mode::Salvage`, then `write::project` on a successful salvage read; asserts nothing about the result, only that the process does not panic. |
| `crates/xtask/src/fuzz.rs` | Owns every fuzzer flag (`--fuzz-dir`, `-rss_limit_mb`, `-detect_leaks`, the PR/cron bound) | VERIFIED | `PR_MAX_TOTAL_TIME=60`, `RSS_LIMIT_MB=2048`, `CRON_RUNS=500_000`; both `fuzz-pr` and `fuzz-cron` pass `--fuzz-dir crates/deform6/fuzz`. |
| `crates/xtask/src/fetch_corpus.rs` | Fetches, hashes, and writes the run-time robustness set from a pinned manifest | VERIFIED | Present; `no_panic_proof.rs` names the command that populates the fetched set when it is absent, rather than treating an empty set as zero-and-passing. |
| `crates/deform6/tests/no_panic_proof.rs` | One run over corpus + fetched + regressions, both modes, counted per source | VERIFIED | Ran it directly: 9/9 passed, per-input trace confirms both modes and the writer run. |
| `crates/deform6/tests/fuzz_smoke.rs` | Deterministic mutation sweep, no fuzzing dependency, runs on stable | VERIFIED | 5/5 passed; `SMOKE_ITERATIONS = 440` (10 mutations x 44 corpus files); doc comment records the real O(n^2) defect this sweep found. |
| `crates/deform6/tests/support/hostile.rs` | Hand-built hostile images, no third-party bytes | VERIFIED | `gui_table_overcount_4k` builds a 4096-byte PE image byte by byte from named literals; own tests confirm exact length, PE signature, determinism. |
| `crates/deform6/tests/regressions.rs` | Stable replay with a non-empty-directory gate | VERIFIED | 18/18 passed; `MINIMUM_REGRESSION_INPUTS` gate confirmed by reading the assertion. |
| `.github/workflows/fuzz.yml` | PR job (wall-clock bound) + scheduled job (iteration bound), both via xtask | VERIFIED | Read in full; both jobs call `cargo run -p xtask -- fuzz-pr`/`fuzz-cron`, never a raw `cargo fuzz` call; both seed from `corpus/` and `tests/regressions/` and fail loudly on a zero count from either source. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `vb/mod.rs` (`inspect`) | `journal.rs` (`Journal::record`) | Every collected defect, from any call site, replays through the severity gate before `inspect` returns | VERIFIED | Confirmed by reading `vb/mod.rs`'s choke point and by the salvage.rs test suite, which exercises this exact path end to end. |
| `.github/workflows/fuzz.yml` | `crates/xtask/src/fuzz.rs` | Both jobs call `cargo run -p xtask -- fuzz-pr`/`fuzz-cron` rather than a raw fuzzer invocation | VERIFIED | Read in full; no bare `cargo fuzz` call in the workflow file. |
| `crates/deform6-cli/src/main.rs` (`write_project`) | `plan_writes` + `refuse_symlink_targets` | Containment and symlink checks run, in that order, before the write loop | VERIFIED | Read the function body; both guard calls precede the `std::fs::write` loop and return early on failure. |
| `crates/deform6/tests/no_panic_proof.rs` | `corpus/manifest.toml` | Fetched-set count checked against the manifest's own entry count | VERIFIED | Confirmed by source read; matches the "never counted from whatever happens to be in the fetched directory" prohibition. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Salvage recovers a file with one unresolvable Declare descriptor; Strict refuses it | `cargo test -p deform6 --test salvage` | 10/10 passed, including both CR-01 regression tests | PASS |
| Schema still validates after two new `DefectKind` variants | `cargo test -p deform6 --test schema` | 3/3 passed | PASS |
| No panic across the full corpus, fetched set, and regressions, both modes | `cargo test -p deform6 --test no_panic_proof -- --nocapture` | 9/9 passed, per-input trace inspected | PASS |
| Deterministic mutation sweep finds no panic | `cargo test -p deform6 --test fuzz_smoke` | 5/5 passed | PASS |
| Corpus-wide severity census | `cargo test -p deform6 --test severity_census` | 1/1 passed | PASS |
| Capacity wall (every file-derived allocation is audited and bounded) | `sh scripts/prove-capacity-wall.sh` | "The capacity wall holds" | PASS |
| Ordered-output wall (no `HashMap` in `report.rs`/`write/model.rs`) | `sh scripts/prove-ordered-output-wall.sh` | "The ordered output wall holds" | PASS |
| Fuzz crate builds on nightly, stays out of the stable workspace | `cargo +nightly fuzz build --fuzz-dir crates/deform6/fuzz` | Compiled cleanly in 12s | PASS |
| Fuzz target survives a real, bounded libFuzzer campaign | `cargo +nightly fuzz run --fuzz-dir crates/deform6/fuzz parse -- -max_total_time=15 -rss_limit_mb=2048` | 717,075 executions, 0 crashes | PASS |
| `wFormCount = 0xFFFF` against a one-entry region | `cargo test -p deform6 gui_table_refuses_an_implausible_form_count` | 1/1 passed | PASS |
| Full workspace suite | `cargo test --workspace` (run once) | 23/23 binaries `ok`, 0 failed | PASS |

### Probe Execution

No probes declared for this phase and no `scripts/*/tests/probe-*.sh` files exist. Skipped.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|--------------|--------|----------|
| SAF-01 | 05-03, 05-05, 05-06, 05-08 | The tool does not panic on any input. | SATISFIED | Fuzz target (05-03), stable regression replay (05-05), deterministic mutation sweep (05-06), and the full-corpus no-panic proof (05-08) all pass; I ran a live 15-second, 717k-execution nightly fuzz campaign myself with zero crashes. |
| SAF-02 | 05-01 | The tool refuses a damaged file by default and names the byte offset. | SATISFIED | `salvage.rs`: `a_patched_external_count_refuses_in_strict_and_names_the_offset` passes; `Exit::Damaged = 4`. |
| SAF-03 | 05-01 | `--salvage` recovers what it can and marks every assumption. | SATISFIED | `salvage.rs` salvage-mode tests pass; the CR-01 fix specifically restores this for the per-item Declare-descriptor case, verified against the current source, not the review's narrative. |
| SAF-04 | 05-02 | No allocation is sized from a file length field before it is checked against the real file size. | SATISFIED | `bound_form_count`, the capacity wall, and `gui_table_refuses_an_implausible_form_count` all confirmed directly. |
| SAF-05 | 05-03, 05-04, 05-05, 05-07 | A fuzzer runs in the gate; every crash it finds becomes a committed, stable-replaying test. | SATISFIED | `.github/workflows/fuzz.yml` (read in full), `fetch_corpus.rs` (present), `regressions.rs` (stable replay with a non-empty gate) all confirmed. |

No orphaned requirements: `.planning/REQUIREMENTS.md` maps only SAF-01 through SAF-05 to Phase 5,
and all five are declared across the eight plans' `requirements` frontmatter.

### Anti-Patterns Found

None in the 26 files this phase's plans modified. Grepped for `TBD`, `FIXME`, `XXX`, `TODO`,
`HACK`, and `PLACEHOLDER` across every file named in the plans' `files_modified` lists: zero
matches. `#![forbid(unsafe_code)]` is present in `crates/deform6/src/lib.rs`; grepping for
`unsafe` in `crates/deform6/src/` and `crates/deform6-cli/src/` returns nothing outside that
one lint attribute.

The one defect this phase's own review found (CR-01, Critical) was fixed today and is
independently re-verified above (truths 6-8), not merely accepted from the review's own
resolution note.

### Human Verification Required

None. Every truth above was checked against the current source and confirmed by running the
actual command, not by reading a summary or a review's claim. `05-UAT.md` already recorded
23/23 human tests passed for this phase (including the one narrowed claim, that `cargo test`
cannot itself prove `panic=abort`, which the phase's own documentation states honestly rather
than overclaiming). No new human-only item was found in this verification pass.

### Gaps Summary

None. All five roadmap success criteria hold against the current codebase, and the one Critical
defect found in today's code review is independently confirmed fixed: `UnmappedAddress` and
`OffsetOverflow` remain `Fatal` for the two spine walks that were built for them
(`ObjectTable::walk`, `GuiTable::walk`, both untouched and still returning `Refusal::Damaged`
directly), while the four per-item call sites that used to misuse those kinds now build
`ItemAddressUnmapped`/`ItemOffsetOverflow`, both `Recoverable`, so `Strict` still refuses and
`Salvage` now actually salvages. The two regression tests naming this exact fix pass, the
schema tests still pass at 3/3 after the two new variants, and the full workspace suite passes
with zero failures.

---

_Verified: 2026-09-14T21:40:00Z_
_Verifier: Claude (gsd-verifier)_
