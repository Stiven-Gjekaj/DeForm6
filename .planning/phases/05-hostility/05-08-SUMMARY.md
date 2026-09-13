---
phase: 05-hostility
plan: 08
subsystem: testing
tags: [rust, cargo-test, fuzzing, corpus, panic-abort]

requires:
  - phase: 05-hostility
    provides: "Mode::Strict/Mode::Salvage on inspect and write::project (05-01), corpus_sweep.rs's own walk-and-pinned-count shape (03-10), regressions.rs's own walk shape and MINIMUM_REGRESSION_INPUTS (05-05), corpus/manifest.toml and fetch-corpus (05-07)"
provides:
  - "crates/deform6/tests/no_panic_proof.rs: one run that reads the vendored corpus, the fetched robustness set and the regression directory, counts each where it lives, and drives every input through both modes and the writer"
  - "A measured peak resident set for a process that reads many files in one call, resolving the open question WINDOWS.md finding 14 named"
affects: []

actuals:
  tokens: 4546
  tasks: 2
  commits: 2
  plan_head_before: 6577e3d53d80aa6daee3b80d35e89360e8c9c0ab

tech-stack:
  added: []
  patterns:
    - "Three independent counting functions, one per input source, each asserted against the place its own true count lives (a pinned literal, a manifest's own table count, a stated minimum), never against each other or against a directory listing standing in for a different source."
    - "An explicit Present(usize)/Absent enum for a source that may not exist on this machine, so a missing directory can never render as a verified zero."
    - "Print the source, the path and the mode before the call that might not return, not after: the log names the last input reached even when the process itself does not survive to log anything more."

key-files:
  created:
    - crates/deform6/tests/no_panic_proof.rs
  modified: []

key-decisions:
  - "The regression minimum (MINIMUM_REGRESSION_INPUTS) is restated as a literal (1) rather than read from regressions.rs's own constant of the same name: a #[test]-bearing file under tests/ compiles as its own independent binary, and no_panic_proof cannot reach a private constant defined in a sibling test binary's crate root. The comment on the restated literal names the file it must move with."
  - "All three walks sort their own result independently (three separate .sort() calls), rather than sharing one sorted helper, so a failure in any one source names the same first file on every machine and the file's own literal sort-call count stays auditable."
  - "fetched_programs(dir) takes a &Path parameter rather than reading corpus/fetched/ internally, specifically so the absent-directory test can call it with a path that does not exist without touching the real fixture."
  - "The deliberate-break proof (task 2's own fail-can-fail check) targeted the regression input specifically, not the first vendored file reached, so the last printed line before the panic named a source other than \"vendored\": it read \"no_panic_proof: regression .../gui-table-overcount-4k.bin Mode::Strict\", proving the per-source label is threaded through correctly and not hard-coded to the first source in the list."

patterns-established:
  - "A run that must prove a negative (nothing panicked) prints its evidence before the risk, not after, so the log is still useful if the negative turns out false."

requirements-completed: [SAF-01]

coverage:
  - id: D1
    description: "Three independent counts (vendored corpus excluding corpus/fetched/, the fetched robustness set, the regression directory), each checked against its own true source (the pinned 44, corpus/manifest.toml's own table count, and a stated minimum), with an absent fetched directory reported as absent and never as a verified zero"
    requirement: "SAF-01"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/no_panic_proof.rs#the_vendored_walk_finds_the_pinned_forty_four, the_manifest_table_count_matches_the_committed_manifest, the_regression_walk_meets_the_stated_minimum, fetched_programs_of_an_absent_directory_reports_absent_not_zero, fetched_programs_of_the_populated_directory_matches_the_manifest_when_present, the_total_is_the_sum_of_the_three_counts_taken, an_absent_fetched_set_still_reports_as_absent_while_the_total_counts_it_as_zero_files, the_counted_report_prints_a_line_per_source_and_a_total_line"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every input from all three sources (48 total this session: 44 vendored, 3 fetched, 1 regression) driven through Mode::Strict and Mode::Salvage, with write::project run on every successful salvage result, no process aborting, and the read count matching the counted total, in both the test profile and the release profile"
    requirement: "SAF-01"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/no_panic_proof.rs#every_input_this_repository_can_reach_runs_through_both_modes_and_the_writer (cargo test -p deform6 --test no_panic_proof; cargo test --release -p deform6 --test no_panic_proof)"
        status: pass
    human_judgment: true
    rationale: "cargo test --release does not actually compile the test binary with panic=\"abort\" (Cargo forces panic=unwind on --test targets so libtest can catch each test's panic and keep running the rest of the suite; confirmed this session with cargo test --release --verbose, whose rustc invocation carries no -C panic=abort flag). The test proves no panic occurs on any of the 48 inputs in either mode, in both profiles; it does not exercise the actual abort-on-panic behavior a real `deform6` binary gets from the workspace's [profile.release]. A human should read this caveat before treating the release run as proof of abort semantics specifically, not only proof of no panic."
---

# Phase 5 Plan 08: The no-panic proof over every input this repository can reach Summary

**One test sweeps 44 vendored programs, 3 fetched robustness-set programs and 1 committed regression input through `Mode::Strict`, `Mode::Salvage` and the writer, in one process, printing a per-source count that matches the file system and asserting only that the process is still running.**

## Performance

- **Duration:** 1 session
- **Completed:** 2026-09-13
- **Tasks:** 2
- **Files modified:** 1 (created)

## Accomplishments

- `crates/deform6/tests/no_panic_proof.rs` counts three sources independently: the vendored corpus (`corpus/`, excluding `corpus/fetched/`) against the same pinned 44 `corpus_sweep.rs`, `differential.rs` and `xtask::MINIMUM_PROGRAM_COUNT` all share; the fetched robustness set against the number of tables `corpus/manifest.toml` declares (parsed with `text.parse::<toml::Table>()`, the same call `fetch_corpus.rs` uses), never against a directory listing; and the regression directory against a restated minimum of 1.
- An absent `corpus/fetched/` reports as `FetchedCount::Absent`, never as `Present(0)`, and names `cargo run -p xtask -- fetch-corpus` as the command that populates it. This is proved by a direct unit test that calls the counting function with a path that does not exist.
- The sweep (task 2) drives all 48 inputs this session finds (44 vendored, 3 fetched, 1 regression) through `deform6::inspect` in both modes and through `deform6::write::project` on every successful salvage result, printing the source, the path and the mode before each call. It asserts nothing about the result variant, only that the number of inputs read equals the total the three counting functions took.
- `cargo run -p xtask -- fetch-corpus` populated `corpus/fetched/` with all three pinned programs (`map-editor-2d`, `passgen`, `transparency-2d`), verified against their SHA-256 twice; `git status --porcelain corpus/` shows nothing staged, confirming no fetched byte was offered to git.
- The deliberate-panic check (see Deviations) confirmed the per-input log line names the exact source, path and mode reached before a panic ends the run.
- Peak resident set measured for the whole reading run: 5,406,720 bytes maximum resident set size (about 5.16 MiB) via `/usr/bin/time -l`, resolving the open question `.planning/WINDOWS.md` finding 14 named. `.planning/WINDOWS.md` finding 14 is updated with this measurement; it stays `open` because the underlying leak (`crate::error::damaged`'s `Box::leak`) is unchanged, only measured.

## Task Commits

Each task was committed atomically:

1. **Task 1: Three sources, three counts, each taken where it lives** - `fac5af6` (test)
2. **Task 2: The sweep, both modes, and the recorded proof** - `7e42f13` (test)

**Plan metadata:** this commit (SUMMARY, STATE, ROADMAP, REQUIREMENTS).

## Files Created/Modified

- `crates/deform6/tests/no_panic_proof.rs` - Three counting functions (vendored, fetched, regression) with their own tests (task 1, 8 tests), plus the sweep over all three sources through both modes and the writer (task 2, 1 test), 9 tests total in this file.

## Decisions Made

See `key-decisions` in the frontmatter. The one decision worth restating here: `MINIMUM_REGRESSION_INPUTS` is restated as a literal in this file rather than read from `regressions.rs`'s own constant, because the two files compile as separate test binaries and cannot share a private item. Both currently hold `1`; a comment on each names the other.

## Deviations from Plan

None against the plan's own tasks - both tasks were executed exactly as written, including the required deliberate-panic check and the release-profile run. One finding surfaced during execution that the plan itself anticipated and asked to be reported honestly rather than treated as a deviation to fix:

**`cargo test --release` does not give this proof real `panic = "abort"` semantics.**

- **Found during:** Task 2, while preparing the release-profile run the plan's own verify block requires.
- **What was expected:** The plan's `plan_specifics` point 4 already flagged this as an open question rather than a settled fact, asking the executor to "say honestly how the run is configured and what that does and does not prove."
- **What was found:** `cargo test --release -p deform6 --test no_panic_proof --verbose` shows the `rustc` invocation for the test binary carries no `-C panic=abort` flag, even though the workspace root `Cargo.toml`'s `[profile.release]` sets `panic = "abort"`. This is a Cargo constraint, not a bug in this file: Cargo forces `panic=unwind` on any `--test` harness target (stable Rust has no `-Z panic-abort-tests` equivalent; that flag is nightly-only), because the built-in test harness must catch each test's panic with `catch_unwind` to keep running the rest of the suite.
- **What this means:** Verified with the deliberate-panic check below: a real panic inside the release-profile test binary is caught by libtest and reported as `test result: FAILED`, not an aborting process. Neither `cargo test` nor `cargo test --release` can demonstrate the abort-on-panic behavior a real `deform6`/`deform6-cli` binary built with `cargo build --release` actually gets. What this file's passing test run genuinely proves is narrower and still true: none of the 48 inputs, across both modes, causes a panic in either compilation profile. What it does not prove is that the compiled release binary's `panic = "abort"` setting is itself exercised by this test; that setting only matters for a caller outside the test harness (the CLI, or a future library embedder), and this file cannot reach that code path while running as a `#[test]`.
- **Action taken:** No code changed. This is reported here, in the frontmatter's `coverage.D2.rationale`, and is the reason `D2` carries `human_judgment: true` rather than an unconditional auto-pass.

## Issues Encountered

None. The deliberate-panic check (temporarily inserted, run, observed, then reverted byte-for-byte, confirmed via `diff` against a backup copy) worked as designed: with a panic injected on the regression input's `Mode::Strict` call, `cargo test --release -p deform6 --test no_panic_proof` printed every vendored and fetched input's own log lines up through `no_panic_proof: regression /.../gui-table-overcount-4k.bin Mode::Strict`, then the panic message, then `test result: FAILED`. The last printed line named the exact source, path and mode the plan's own acceptance criterion requires. The file was restored to its pre-panic state and the full gate re-verified green before committing task 2.

## User Setup Required

None - no external service configuration required. `cargo run -p xtask -- fetch-corpus` needs network access, matching the same requirement `05-07-SUMMARY.md` already recorded.

## Next Phase Readiness

- **The three counts, and the number the run printed, this session:** vendored 44 (counted by walking `corpus/`, excluding `corpus/fetched/`, filtered to `.exe`); fetched 3 (counted by walking `corpus/fetched/` after `fetch-corpus` populated it, matching `corpus/manifest.toml`'s own 3 tables); regression 1 (counted by walking `crates/deform6/tests/regressions/`). The run printed `no_panic_proof: total inputs: 48` in both the test-profile and the release-profile invocation, taken by reading the printed `--nocapture` output of each, not calculated from any one of the three.
- **The measured peak resident set:** 5,406,720 bytes maximum resident set size (about 5.16 MiB), 4,145,488 bytes peak memory footprint (about 3.95 MiB), measured with `/usr/bin/time -l` wrapping the compiled release test binary directly (`target/release/deps/no_panic_proof-<hash> --test-threads=1`), wall clock 0.02s for the whole 9-test binary including the sweep. `.planning/WINDOWS.md` finding 14 is updated with this number and the reasoning that the `damaged()` leak's own worst case (237 bytes per call, from plan 05-03) contributes at most a few tens of kilobytes across every call this run makes, undetectable against the measured 5 MiB baseline; the finding stays `open` because the leak itself is not fixed, only measured at this input-set size.
- **What the run proves about `panic = "abort"`, stated plainly:** it proves no panic occurs on any of the 48 inputs in either mode, in both the test and release compilation profiles. It does not prove the aborting behavior itself, because Cargo compiles every `--test` target with `panic=unwind` regardless of the `[profile.release]` setting, to keep the test harness's per-test `catch_unwind` working. See Deviations above and `coverage.D2.rationale` in the frontmatter.
- **SAF-01** ("The tool does not panic on any input") is marked complete. `gsd-tools query requirements.ready-ids` reported it ready (all four declaring plans - 05-03, 05-05, 05-06, 05-08 - have summaries), and the proof exists on disk: `crates/deform6/tests/no_panic_proof.rs` passes 9/9 tests under both `cargo test` and `cargo test --release`, the full workspace gate is green at 970 passed / 0 failed, and `corpus/fetched/` is populated and verified. The caveat above (what the release run does and does not prove about abort semantics) is recorded rather than hidden; SAF-01's own wording is about panicking, not about process-exit mechanics, and no panic occurred in either profile.
- **This is the last plan in Phase 5.** All eight plans (05-01 through 05-08) now have summaries. Phase 5's own roadmap success criteria (fuzzing in the gate, the salvage mode, the regression harness, the run time robustness corpus, and this no-panic proof) are each backed by a committed artifact; `/gsd-verify-work 5` is the next step before opening Phase 6.
- No blockers. `cargo test --workspace` is green at 970 passed, 0 failed, the new floor for Phase 6.

---
*Phase: 05-hostility*
*Completed: 2026-09-13*

## Self-Check: PASSED
