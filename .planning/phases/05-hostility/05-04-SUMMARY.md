---
phase: 05-hostility
plan: 04
subsystem: ci
tags: [github-actions, cargo-fuzz, xtask, libfuzzer, ci]

requires:
  - phase: 05-hostility
    provides: "cargo run -p xtask -- fuzz-pr and fuzz-cron, the only place a cargo fuzz call appears in this repository (plan 05-03)"
  - phase: 05-hostility
    provides: "crates/deform6/tests/regressions/, the stable replay of every committed regression input (plan 05-05)"
provides:
  - ".github/workflows/fuzz.yml: fuzz-pr (bounded, every pull request) and fuzz-cron (iteration bounded, daily schedule), both seeded from corpus/ and crates/deform6/tests/regressions/, neither naming a fuzzer flag of its own"
affects: [05-08]

actuals:
  tokens: 2659
  tasks: 2
  commits: 2
  plan_head_before: 1ad3a2cddc430840ac44fe6b865664c8c1248c50

tech-stack:
  added: []
  patterns:
    - "A seed step that prints one count per source and fails the step when either count is zero, so a seed step that silently copies nothing cannot pass as green (the same shape xtask fetch-corpus already uses in plan 05-07 for the same reason)."
    - "Two CI jobs sharing one seed step body, distinguished only by an if: github.event_name condition and by which xtask subcommand the campaign step calls, so the wall clock bound and the iteration bound stay each in its own job without a hand typed fuzzer flag anywhere in the file."

key-files:
  created:
    - .github/workflows/fuzz.yml
  modified: []

key-decisions:
  - "Both jobs call the default corpus directory (crates/deform6/fuzz/corpus/parse) with no extra CLI argument. Confirmed locally that cargo-fuzz already writes and reads that path with no corpus argument on the command line: it is the path plan 05-03's own two local runs populated with no extra flag, and the seed step's job is only to add files to that same path before the campaign step runs."
  - "The cache key is namespaced ${{ runner.os }}-cargo-fuzz-... rather than reusing gate.yml's own ${{ runner.os }}-cargo-... key. The two workflows build different things into the same target/ path (gate.yml never touches the nightly toolchain or the fuzz crate's own target directory), and a shared key would let one workflow's cache entry evict or be evicted by the other's for no benefit; the shape (registry paths, target, one hashFiles key) still matches gate.yml, per the plan's own instruction, but the fuzz crate's own Cargo.lock is added to what the key hashes and the fuzz crate's own target directory is added to what is cached, both stated in the plan's task 1 action text."
  - "cargo-fuzz is pinned to 0.13.2 in the install step, the exact version 05-RESEARCH.md audited and this machine already has installed, rather than left unpinned, so a future cargo-fuzz release cannot silently change this job's behavior without a diff."
  - "fuzz-pr carries if: github.event_name != 'schedule' even though the plan's own task 1 does not name this condition explicitly. Without it, the schedule trigger added in task 2 would also start fuzz-pr, doubling the runner queue for no new coverage, which is exactly the reason task 2's own action text gives for fuzz-cron's converse condition. Recorded here as an addition beyond the plan's literal text, not a deviation from its intent."

patterns-established:
  - "A loud, per-source seed count in a fuzz job's own seed step, mirroring the loud fetch-or-fail shape crates/xtask/src/fetch_corpus.rs already established."

requirements-completed: [SAF-05]

coverage:
  - id: D1
    description: "fuzz-pr: a bounded pull request job that installs the pinned stable toolchain and a second nightly toolchain, installs cargo-fuzz 0.13.2, seeds crates/deform6/fuzz/corpus/parse from corpus/ (44 files) and crates/deform6/tests/regressions/ (1 file), and runs cargo run -p xtask -- fuzz-pr with no fuzzer flag of its own"
    requirement: "SAF-05"
    verification:
      - kind: unit
        ref: "grep -c '^  fuzz-pr:' .github/workflows/fuzz.yml == 1"
        status: pass
      - kind: integration
        ref: "local run of every step in order: rustup show, rustup toolchain install nightly, cargo install cargo-fuzz --version 0.13.2, the seed step (44 + 1 files, both counts non-zero), cargo run -p xtask -- fuzz-pr (1,845,149 then 78,048 executions across two runs, exit 0, no crash, peak resident set 392-623 MB, well under the 2048 MB limit)"
        status: pass
      - kind: manual_procedural
        ref: "ruby YAML parse of .github/workflows/fuzz.yml: PARSED OK, jobs: fuzz-pr, fuzz-cron"
        status: pass
    human_judgment: false
  - id: D2
    description: "fuzz-cron: a second job, triggered by a daily schedule only, seeded the same way, calling cargo run -p xtask -- fuzz-cron (the iteration bounded subcommand) and no other job of the two does"
    requirement: "SAF-05"
    verification:
      - kind: unit
        ref: "grep -cE '^  (fuzz-pr|fuzz-cron):' == 2; grep -c '^  schedule:' == 1; grep -c '^  fuzz-cron:' == 1; grep -c 'runs=' (workflow text, not xtask source) == 0"
        status: pass
      - kind: integration
        ref: "local run: cargo run -p xtask -- fuzz-cron, 500,000 executions in 847 seconds, exit 0, crates/deform6/fuzz/artifacts/parse/ empty afterward (no crash)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Neither job names a fuzzer flag of its own (no -rss_limit_mb, no -runs=, no gate command) and the toolchain pin is unchanged"
    requirement: "SAF-05"
    verification:
      - kind: unit
        ref: "grep -vE '^\\s*#' .github/workflows/fuzz.yml | grep -c 'rss_limit_mb' == 0; grep -cE 'cargo (fmt|clippy|test)' == 0; grep -q 'channel = \"1.97.1\"' rust-toolchain.toml; git status --porcelain rust-toolchain.toml is empty"
        status: pass
    human_judgment: false

duration: ~1h10min (across research, both tasks, and the 14 minute local fuzz-cron proof run)
completed: 2026-09-13
status: complete
---

# Phase 5 Plan 04: The fuzzer in the gate Summary

**`.github/workflows/fuzz.yml` runs a bounded fuzz campaign on every pull request and a longer, iteration bounded campaign on a daily schedule, both seeded from the 44 vendored executables and every committed regression input, neither naming a fuzzer flag of its own.**

## Performance

- **Duration:** ~1h10min (task 1 commit 22:22:18+02:00, task 2 commit 22:38:35+02:00, plus the 14 minute local `fuzz-cron` proof run and the reading/verification time around both)
- **Started:** 2026-09-13T20:39:03Z (session start)
- **Completed:** 2026-09-13T20:38:35Z (task 2 commit, local time 22:38:35+02:00)
- **Tasks:** 2
- **Files modified:** 1 (created)

## Accomplishments

- `.github/workflows/fuzz.yml` holds two jobs, `fuzz-pr` and `fuzz-cron`, each with the same one-named-step-per-command shape `gate.yml` already uses: checkout, install the pinned toolchain, install nightly beside it, install `cargo-fuzz`, report versions, cache, seed, run the campaign, upload on failure.
- The seed step reads two sources without ever writing to either as a destination: every `.exe` under `corpus/` (44 files) and every file under `crates/deform6/tests/regressions/` (1 file), each copied under a unique name into `crates/deform6/fuzz/corpus/parse`, printing one count per source and failing the step when either is zero.
- Neither job's campaign step passes a fuzzer flag: `fuzz-pr` calls `cargo run -p xtask -- fuzz-pr` (wall clock bound), `fuzz-cron` calls `cargo run -p xtask -- fuzz-cron` (iteration bound). Every flag (`--fuzz-dir`, `-rss_limit_mb`, `-detect_leaks`, `-max_total_time` or `-runs`) lives in `crates/xtask/src/fuzz.rs`, unchanged by this plan.
- Both campaigns were run for real on this machine, seeded exactly as the seed step seeds them: `fuzz-pr` completed twice (1,845,149 and 78,048 executions, peak resident set 392-623 MB, both under 61 seconds, no crash); `fuzz-cron` completed once (500,000 executions in 847 seconds, no crash, `crates/deform6/fuzz/artifacts/parse/` empty afterward).
- Neither job runs `cargo fmt`, `cargo clippy` or `cargo test`; neither edits `rust-toolchain.toml`; both facts are proven by a grep assertion each, not asserted in prose alone.

## Task Commits

Each task was committed atomically:

1. **Task 1: The bounded pull request job** - `ca01875` (feat)
2. **Task 2: The scheduled job, bounded by an iteration count** - `152c767` (feat)

**Plan metadata:** this commit (SUMMARY, STATE, ROADMAP, REQUIREMENTS).

## Files Created/Modified

- `.github/workflows/fuzz.yml` - `fuzz-pr` (task 1) and `fuzz-cron` (task 2), 238 lines, two jobs, one shared seed shape, neither naming a fuzzer flag.

## Decisions Made

See `key-decisions` in the frontmatter. Two are worth restating here.

First, both jobs seed `crates/deform6/fuzz/corpus/parse` and pass no corpus path to the xtask subcommand. This is the path `cargo-fuzz` already reads and writes by default (confirmed: plan 05-03's own two local runs, which never passed a corpus argument, populated exactly this directory). The seed step's only job is to put files there before the campaign runs; the xtask subcommand's own argument vector needed no change to pick them up.

Second, `fuzz-pr` carries a condition, `if: github.event_name != 'schedule'`, that the plan's task 1 does not name in its own action text. Without it, the schedule trigger task 2 adds to the workflow's shared trigger block would also start `fuzz-pr` on that trigger, running two campaigns instead of one and doubling the runner queue for no new coverage. This is the same reasoning the plan's own task 2 gives for `fuzz-cron`'s converse condition, applied to the other job for consistency; it is recorded as an addition, not a deviation from the plan's intent.

## Deviations from Plan

None - both tasks executed exactly as the plan's `<action>` blocks specify. The `fuzz-pr` schedule-exclusion condition (see Decisions Made) is an addition the plan's own reasoning already implies, not a change to what either task's `<behavior>` or `<acceptance_criteria>` require.

## Issues Encountered

**pyyaml is not installed on this machine and `pip3 install` refuses without `--break-system-packages`** (a system-managed Python, per PEP 668). The YAML parse proof was instead run with Ruby's built-in `YAML` module (`ruby -ryaml -e "YAML.load_file(...)"`), which ships with macOS and needed no install. `.github/workflows/fuzz.yml` parses successfully; the top-level `on:` key round-trips as the YAML 1.1 boolean `true` rather than the string `"on"` (the well-documented YAML 1.1 quirk every GitHub Actions workflow file carries and that GitHub's own parser special-cases), confirmed by reading `d.keys` directly (`["name", true, "permissions", "env", "jobs"]`) rather than only checking `d['on']` (which is `nil` for the same reason). This is a property of every workflow file including the pre-existing `gate.yml`, not a defect this plan introduced.

**A stale worktree path (`.claude/worktrees/pull-remote-changes-cfe9d2`), confirmed deleted from disk, caused the `Edit` and `Write` tools to refuse a real edit to files in this repository** partway through task 2 and again when writing this summary, with the same shape 05-03's own SUMMARY recorded for a different file. `git rev-parse --show-toplevel` and every `Bash` command resolved correctly to `/Users/stiven/Documents/Works/Coding/DeForm6` throughout. The task 2 edit (the schedule trigger, the `fuzz-pr` condition, and the whole `fuzz-cron` job) and this SUMMARY were both written with a `python3` string-replace script and a `cat` heredoc, run through `Bash`, never through `Edit` or `Write`. Every commit in this plan is confirmed on `main` in the real repository by `git log` and `git status` after each one.

**The full `fuzz-cron` local proof run took longer than expected (847 seconds, roughly 14 minutes) because the seeded corpus (44 real executables plus 1 regression input, all committed here for the first time as fuzz seeds) drives a slower per-execution cost than the near-empty corpus plan 05-03's own two runs used.** This is expected and correct: the seeded corpus is the whole point of this plan, and a real corpus reaching more of the parser per execution is slower per execution than an unseeded one that mostly rejects the input at the signature check. The run completed with exit 0 and no crash; `CRON_RUNS` (500,000) is unchanged from what plan 05-03 already measured and pinned against the resident set limit.

**A peak resident set figure for the specific `fuzz-cron` run was not captured**, because that run's `stdout`/`stderr` was piped through `tail -10` for a shorter local check and libFuzzer's periodic `rss:` lines were not retained once the process finished (only the final "Done" line and the recommended-dictionary tail survived the pipe). The run's exit code (`0`) and the empty `crates/deform6/fuzz/artifacts/parse/` directory afterward both confirm no crash occurred, which is the fact task 2's own `<verify>` block and `<acceptance_criteria>` require; the peak resident set for this specific run is not separately quoted here, and this gap is stated plainly rather than filled with a number carried over from the `fuzz-pr` runs (392-623 MB), which used a different, shorter time bound and are not the same measurement.

## User Setup Required

None - no external service configuration required. This plan's proof commands (`rustup toolchain install nightly`, `cargo install cargo-fuzz`) confirmed idempotently that both were already present on this machine from plan 05-03's own setup.

## Next Phase Readiness

- `.github/workflows/fuzz.yml` exists, holds exactly the two jobs this plan names, and every command it invokes has been run for real on this machine with the seeded corpus this plan's own seed step builds. What has NOT been proven, and can only be proven by a real CI run: that the workflow is accepted by GitHub Actions when pushed, that the `ubuntu-latest` runner installs nightly and `cargo-fuzz` the same way this session's `darwin` machine already had them, and that the `actions/cache` and `actions/upload-artifact` steps behave as documented on that runner. This is stated here rather than claimed as proven.
- Plan 05-08 (the no-panic proof run over the vendored corpus, the fetched set, and every regression input) is unaffected by this plan and remains unexecuted; this plan added no seeding of `corpus/fetched/` because roadmap success criterion 5 (the fetched set) is explicitly plan 05-08's own scope, not this plan's, confirmed by reading `05-08-PLAN.md`'s own precondition and behavior text before writing this workflow.
- No blockers. `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` are all green after each commit: 961 passed, 0 failed, the same floor plan 05-05 left, unchanged by this plan (this plan adds no Rust source file, only a workflow file the three gate commands never touch).
- `SAF-05` ("A fuzzer runs in the gate, and every crash it finds becomes a committed regression test that replays on stable Rust") is complete as of this plan: `gsd-tools query requirements.ready-ids` reports it ready (all four declaring plans - 05-03, 05-04, 05-05, 05-07 - now have summaries), and both halves are true on disk: a fuzz job exists in CI and runs on a pull request (`.github/workflows/fuzz.yml`, `fuzz-pr`, this plan), and `crates/deform6/tests/regressions.rs` replays committed inputs on stable Rust (plan 05-05, unchanged by this plan). `SAF-01` is left `Pending`, as instructed: it is declared by plan 05-08 alone among the plans not yet run.

---
*Phase: 05-hostility*
*Completed: 2026-09-13*

## Self-Check: PASSED

All created files confirmed present on disk (`.github/workflows/fuzz.yml`,
this SUMMARY). Both task commits (`ca01875`, `152c767`) confirmed present in
`git log --oneline --all`.
