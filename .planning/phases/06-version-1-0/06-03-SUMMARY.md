---
phase: 06-version-1-0
plan: 03
subsystem: docs
tags: [rustdoc, doc-comments, cargo, github-actions, ci-gate]

# Dependency graph
requires:
  - phase: 06-01
    provides: the report schema and the report.rs shape this plan's doc comments must not disturb
provides:
  - "cargo doc --no-deps --workspace at zero warnings, measured from a clean rebuild"
  - "a cargo doc step in .github/workflows/gate.yml that denies warnings with RUSTDOCFLAGS"
  - "a fix for the deform6-cli / deform6 cargo doc output-filename collision (rust-lang/cargo#6313)"
affects: [06-07]

# Actuals (#2632)
actuals:
  tokens: 6946
  tasks: 3
  commits: 5

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Doc-comment link fix: a link to a private or unresolved item drops its markdown brackets and stays as plain code text (backticks only); a link to a public, reachable item is fully qualified instead of dropped."

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/project.rs
    - crates/deform6/src/vb/opcodes.rs
    - crates/deform6/src/vb/controlinfo.rs
    - crates/deform6/src/vb/propstream.rs
    - crates/deform6/src/vb/privateobj.rs
    - crates/deform6/src/vb/controltree.rs
    - crates/deform6/src/vb/object.rs
    - crates/deform6/src/vb/ocx.rs
    - crates/deform6/src/vb/header.rs
    - crates/deform6/src/vb/gui.rs
    - crates/deform6/src/vb/frx.rs
    - crates/deform6/src/write/vbp.rs
    - crates/deform6/src/write/model.rs
    - crates/deform6/src/write/code.rs
    - crates/deform6/src/write/mod.rs
    - crates/deform6/src/report.rs
    - crates/deform6/src/error.rs
    - crates/deform6/tests/ratios.rs
    - crates/deform6/tests/differential.rs
    - crates/deform6-cli/Cargo.toml
    - .github/workflows/gate.yml

key-decisions:
  - "Fixed the deform6-cli/Cargo.toml output-filename collision (doc = false on the bin target) as a Rule 3 blocking-issue deviation: it was the one warning that survived every doc-comment fix, it is not a doc-comment defect, and it is a known Cargo limitation (rust-lang/cargo#6313), not a naming choice for this repository to revisit. Left unfixed, the plan's own zero-warning success criterion could not be met."
  - "Fully qualified links to public, reachable items (SafeName, SafeName::as_str, SafeName::file_name, FormReport::controls, std::fmt::Display) instead of dropping their brackets, matching 06-PATTERNS.md's preference for a resolving link over plain text when the target is public and reachable."
  - "Split the work into five commits rather than three (one per task), because AGENTS.md requires one change per commit: a doc-comment link fix, a Cargo.toml build-config fix, and a new gate.yml step are three different kinds of change even within one plan task."
  - "Measured the zero-warning state from cargo clean --doc rebuilds, not incremental ones. An incremental cargo doc run kept reporting six already-fixed warnings in crates/deform6/tests/ratios.rs and differential.rs after the fix was on disk and saved; cargo doc's caching did not pick up the doc-comment-only change to files xtask embeds by #[path]. The plan itself warned that cargo doc caches aggressively; a clean rebuild is how that warning was honoured, not just repeated."

patterns-established:
  - "Doc-comment link fix: drop [`X`] to `X` when X is private or unresolvable from that scope; write the fully qualified path inside the brackets when X is public and reachable."

requirements-completed: [OBJ-01, OBJ-02, OBJ-03, OBJ-04, OBJ-05, OBJ-06]

coverage:
  - id: D1
    description: "cargo doc --no-deps --workspace produces zero warning lines, down from the 52 measured at plan time."
    requirement: "OBJ-01"
    verification:
      - kind: other
        ref: "cargo clean --doc && cargo doc --no-deps --workspace 2>&1 | grep -c '^warning'  ->  0"
        status: pass
    human_judgment: false
  - id: D2
    description: "The zero-warning state is enforced by CI, not just true on the day it was reached: RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --workspace exits zero on the repaired tree (it exited 101 before this plan)."
    requirement: "OBJ-02"
    verification:
      - kind: other
        ref: "cargo clean --doc && RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --workspace  ->  exit 0"
        status: pass
    human_judgment: false
  - id: D3
    description: ".github/workflows/gate.yml holds a cargo doc step, after cargo test and before Prove the lint wall, with RUSTDOCFLAGS set to -D warnings and a comment block explaining why the base three commands cannot catch a doc warning."
    requirement: "OBJ-02"
    verification:
      - kind: other
        ref: "grep -q 'cargo doc --no-deps --workspace' .github/workflows/gate.yml && grep -q 'RUSTDOCFLAGS' .github/workflows/gate.yml"
        status: pass
    human_judgment: false
  - id: D4
    description: "No public item's promise changed anywhere in the tree; every edit in the eleven vb/ files, the six write/report/error files, and the two embedded test files is a link-syntax edit only (every added line begins with /// or //!)."
    requirement: "OBJ-03"
    verification:
      - kind: other
        ref: "git diff main -- <task files> | grep '^+' | grep -v '^+++' | sed -E 's/^\\+[[:space:]]*//' | grep -vcE '^(///|//!)'  ->  0, for each of the three task file sets"
        status: pass
    human_judgment: false
  - id: D5
    description: "cargo fmt --all --check, cargo clippy --all-targets -- -D warnings, and cargo test --workspace all still pass, and cargo test -p deform6 --test schema still reports three passing tests, so report.rs's serialised shape did not move."
    requirement: "OBJ-04"
    verification:
      - kind: other
        ref: "cargo fmt --all --check; cargo clippy --all-targets -- -D warnings; cargo test --workspace (637 total tests, 0 failed); cargo test -p deform6 --test schema (3 passed)"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-09-14
status: complete
---

# Phase 6 Plan 3: Zero rustdoc warnings, held by a gate step, Summary

**Repaired all 52 `cargo doc --no-deps --workspace` warning lines (49 located doc-comment links plus a Cargo bin/lib name collision) and added a `cargo doc` step to `gate.yml` with `RUSTDOCFLAGS=-D warnings` so the zero-warning state cannot regress silently.**

## Performance

- **Duration:** ~15 min
- **Started:** approx. 2026-09-14T18:10Z (STATE.md's last recorded activity before this plan)
- **Completed:** 2026-09-14T18:21:41Z
- **Tasks:** 3 (plan tasks), 5 commits (AGENTS.md granularity)
- **Files modified:** 21

## Accomplishments

- Repaired all 30 doc warnings under `crates/deform6/src/vb/` (11 files): every link to a private constant or helper drops its brackets and stays as code text; the one link to `std::fmt::Display` is fully qualified so it still resolves.
- Repaired all 13 doc warnings in the writer modules, `report.rs`, and `error.rs` (6 files): two private-helper links drop their brackets; four links to public, reachable items (`SafeName`, `SafeName::as_str`, `SafeName::file_name` twice, `FormReport::controls`) are fully qualified instead.
- Repaired the 6 doc warnings `xtask` reports for `crates/deform6/tests/ratios.rs` and `differential.rs` (embedded via `#[path]`): all six are links to test function names, which rustdoc can never resolve under `--no-deps`, so all six drop their brackets.
- Found and fixed the one warning that was not a doc-comment defect: `deform6-cli`'s bin target shares cargo's output name `deform6` with the `deform6` lib crate, so `cargo doc` reported an output-filename collision on every run (`rust-lang/cargo#6313`). `doc = false` on the bin target removes it without renaming anything a user runs.
- Added a `cargo doc` step to `.github/workflows/gate.yml`, placed after `cargo test` and before `Prove the lint wall`, with `RUSTDOCFLAGS` set to `-D warnings` so a future pull request cannot reintroduce a doc warning silently.

## Task Commits

Each task was split into commits at AGENTS.md's own granularity (one change per commit), so the 3 plan tasks produced 5 commits:

1. **Task 1: The reader modules, thirty warnings** - `f6fade3` (docs)
2. **Task 2: The writer modules and the two top level modules, thirteen warnings** - `4239308` (docs)
3. **Task 3a: The two test files xtask embeds** - `02bdf3b` (docs)
4. **Task 3b: The Cargo.toml collision fix (deviation, Rule 3)** - `81a3a59` (fix)
5. **Task 3c: The gate.yml step** - `11129d1` (feat)

**Plan metadata:** committed together with this SUMMARY.

## Files Created/Modified

- `crates/deform6/src/vb/{project,opcodes,controlinfo,propstream,privateobj,controltree,object,ocx,header,gui,frx}.rs` - link-syntax-only doc-comment fixes, 30 warnings
- `crates/deform6/src/write/{vbp,model,code,mod}.rs`, `crates/deform6/src/report.rs`, `crates/deform6/src/error.rs` - link-syntax-only doc-comment fixes, 13 warnings
- `crates/deform6/tests/ratios.rs`, `crates/deform6/tests/differential.rs` - link-syntax-only doc-comment fixes, 6 warnings (surfaced under the `xtask` crate via `#[path]`)
- `crates/deform6-cli/Cargo.toml` - `doc = false` on the `deform6` bin target, fixing the cargo output-filename collision warning (deviation, see below)
- `.github/workflows/gate.yml` - new `cargo doc` step, `RUSTDOCFLAGS=-D warnings`, placed after `cargo test` and before `Prove the lint wall`

## Decisions Made

See `key-decisions` in the frontmatter above: the Cargo.toml collision fix, the fully-qualified-path choice for public reachable links, the five-commit split, and the clean-rebuild measurement discipline.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed the deform6-cli / deform6 cargo doc output-filename collision**
- **Found during:** Task 3, while measuring the total warning count after fixing the six test-file links
- **Issue:** `cargo doc --no-deps --workspace` printed 49 located doc-comment warnings, 2 per-crate summary lines, and one more warning not accounted for by either: `output filename collision at target/doc/deform6/index.html`, because `deform6-cli`'s `[[bin]] name = "deform6"` produces the same rustdoc output path as the `deform6` lib crate. This is a known Cargo limitation (`rust-lang/cargo#6313`), and it is not a doc-comment defect, so no task in this plan names it. Left unfixed, `cargo doc --no-deps --workspace` could never reach the zero warnings the plan's success criterion 5 and the plan's own task 3 acceptance criteria require.
- **Fix:** Added `doc = false` to the `[[bin]]` table in `crates/deform6-cli/Cargo.toml`, with a comment naming the Cargo issue. The binary keeps the name `deform6`; only its rustdoc output is skipped.
- **Files modified:** `crates/deform6-cli/Cargo.toml`
- **Verification:** `cargo clean --doc && cargo doc --no-deps --workspace 2>&1 | grep -c '^warning'` reports `0`. `cargo build` and `cargo test --workspace` still pass, confirming the binary itself is unaffected.
- **Committed in:** `81a3a59`

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Required to meet the plan's own stated success criterion (zero warnings from `cargo doc --no-deps --workspace`). No doc-comment content changed as a result; no public item's promise moved.

## Issues Encountered

- **The plan's own task-level source assertion regex undercounts correctly, but only by accident, and its `+++`-exclusion is broken.** Each task's `<verify>` block includes `printf '%s\n' "$d" | grep '^+' | grep -vcE '^\+(///|//!|\+\+\+)'`, intended to count added lines that are not doc comments (should be zero). Tested directly: the `\+\+\+` alternative requires four consecutive `+` characters immediately after the leading `+` that `grep '^+'` already selected, but a `git diff` file header line reads `+++ b/path` (three `+`, then a space) — one `+` short of what the pattern needs — so it is never excluded. Worse, the `///` and `//!` alternatives are anchored directly after the leading `+` with no room for the indentation nearly every method-level doc comment carries (`+    /// text`), so almost every genuine doc-comment addition also fails to match and gets counted as "not a doc comment." Run as written on this plan's own diffs, the check reports a positive count (e.g. 31 for task 1's 11-file diff) even though every added line is, in fact, either a diff file header or an indented `///` line. I ran a corrected equivalent (`git diff main -- <paths> | grep '^+' | grep -v '^+++' | sed -E 's/^\+[[:space:]]*//' | grep -vcE '^(///|//!)'`) after each task, which reports `0` for all three task file sets — matching a full manual read of every diff hunk, which confirmed every added line does begin with `///` or `//!`. I did not edit `06-03-PLAN.md`, since the plan is a historical record of what was asked, not a deliverable this plan's `files_modified` names; this note exists so a reader who reruns the plan's literal command is not surprised by a false positive.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `cargo doc --no-deps` half of success criterion 5 is met: zero warnings, measured from a clean rebuild, and the state is held by `gate.yml`, not by the day it was reached.
- Plan 06-07's edge-probe ledger inherits the one `unclassified` OBJ-02 probe row unchanged (this plan touched no behavior, so it neither resolves nor worsens that row).
- No blockers for the next plan in this phase.

---
*Phase: 06-version-1-0*
*Completed: 2026-09-14*

## Self-Check: PASSED

- All files named in `key-files.modified` confirmed present with `[ -f ]`.
- All five commit hashes (`f6fade3`, `4239308`, `02bdf3b`, `81a3a59`, `11129d1`) confirmed present in `git log --oneline --all`.
- `cargo doc --no-deps --workspace` re-run clean after every edit: `0` warning lines.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace` re-run clean: exit `0`.
- `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` (637 tests, 0 failed) all re-run and pass after the final commit.
