---
phase: 06-version-1-0
plan: 05
subsystem: release-tooling
tags: [xtask, cargo-metadata, licences, changelog, release-mechanics]

requires:
  - phase: 06-version-1-0
    provides: "the 1.0.0 workspace version (06-04) that the changelog and licence table must stay independent of"
provides:
  - "cargo run -p xtask -- licences, a re-derivable dependency licence audit"
  - "LICENSES.md, the rendered table of 139 third party packages and 20 distinct licence expressions"
  - "CHANGELOG.md, the 1.0.0 release entry naming what is delivered, what stays open, and what is not done"
affects: [06-06, 06-07, release-checklist]

actuals:
  tokens: 8023
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "xtask subcommand shape: an inner Result<_, String> function plus a thin run() -> i32 wrapper that prints or eprintln's, matching update_ratios"
    - "manual, re-derived audit document (LICENSES.md) rather than a cargo-deny/deny.toml dependency, per decision D-03"

key-files:
  created:
    - crates/xtask/src/licences.rs
    - LICENSES.md
    - CHANGELOG.md
  modified:
    - crates/xtask/Cargo.toml
    - crates/xtask/src/main.rs
    - Cargo.lock

key-decisions:
  - "Decision D-03 confirmed as delivered: the licence audit is a manual, cargo-metadata-derived table in LICENSES.md. No cargo-deny, no deny.toml were added."
  - "The rendered file's date line is produced by shelling out to the date command (std::process::Command), not by hand-rolled calendar arithmetic, to stay inside the workspace's arithmetic_side_effects clippy wall."
  - "CHANGELOG.md is a plain reverse-chronological Markdown list with one level two heading per release, since the repository had no prior format to follow; the note stating this lives at the top of the file itself."

patterns-established:
  - "A re-derivable release artifact (LICENSES.md) is checked by re-running its own generator and diffing the result, never by reading the file's prose."

requirements-completed: [FRM-01, FRM-02, FRM-03, WRT-03]

coverage:
  - id: D1
    description: "cargo run -p xtask -- licences renders LICENSES.md from a fresh cargo metadata run; re-running it is a no-op"
    requirement: ""
    verification:
      - kind: integration
        ref: "cargo run -q -p xtask -- licences && git diff --exit-code -- LICENSES.md"
        status: pass
      - kind: unit
        ref: "crates/xtask/src/licences.rs#licences::tests (5 tests: packages_from_metadata excludes workspace members and sorts, refuses a package with no name, check_minimum_package_count, licence_counts groups not-stated, render never names a workspace member)"
        status: pass
    human_judgment: false
  - id: D2
    description: "CHANGELOG.md opens with the 1.0.0 release entry, naming what is delivered, the four requirements that stay open, and what the release does not do"
    requirement: ""
    verification:
      - kind: other
        ref: "grep -qE '^## \\[1\\.0\\.0\\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$' CHANGELOG.md; test 4 -eq distinct FRM-0[123]/WRT-03 matches; ASCII-only byte check; no author/tool marker grep; lpNativeCode/P-code and Visual Basic 6 IDE mentions"
        status: pass
    human_judgment: false

duration: 35min
completed: 2026-09-14
status: complete
---

# Phase 6 Plan 05: Release Mechanics (Licences and Changelog) Summary

**A re-derivable licence audit (139 third party packages, 20 distinct licence expressions, none unstated) and a changelog opening with 1.0.0, both measured from the workspace as it stands today rather than typed by hand.**

## Performance

- **Duration:** 35 min
- **Started:** 2026-09-14T18:40:00Z
- **Completed:** 2026-09-14T19:15:00Z
- **Tasks:** 2
- **Files modified:** 6 (3 created, 3 modified)

## Accomplishments

- Added `cargo run -p xtask -- licences`, an xtask subcommand that runs
  `cargo metadata --format-version 1 --all-features --locked` as a subprocess,
  excludes the three workspace members by name, and renders `LICENSES.md`: a
  table of every third party package's name, version, licence and source,
  followed by a count of packages per distinct licence expression.
- Measured, this session: 139 third party packages, 20 distinct licence
  expressions, 0 packages with no stated licence.
- Confirmed decision D-03's acceptance check holds: re-running the
  subcommand and diffing `LICENSES.md` against the committed file is clean.
- Wrote `CHANGELOG.md`, opening with the `## [1.0.0]` release section: what
  the release delivers by requirement category, the four requirements that
  stay open at release (FRM-01, FRM-02, FRM-03, WRT-03), and three things
  the release does not do (no statement recovery, no P-code run, no
  recompilation test).

## Task Commits

Each task was committed atomically:

1. **Task 1: An xtask subcommand that renders the licence audit, and the file it renders** - `d8531ed` (feat)
2. **Task 2: The changelog, opening with 1.0.0** - `baffb0f` (docs)

**Plan metadata:** committed alongside this summary.

## Files Created/Modified

- `crates/xtask/src/licences.rs` - the licences subcommand: cargo metadata subprocess, package extraction and sort, minimum-row refusal, rendering, and five unit tests
- `LICENSES.md` - the rendered licence audit (139 packages, 20 distinct licence expressions)
- `CHANGELOG.md` - the 1.0.0 release entry and the format note at the top of the file
- `crates/xtask/Cargo.toml` - added `serde_json.workspace = true` (already a workspace dependency, no new external package)
- `crates/xtask/src/main.rs` - wired `mod licences;`, the `"licences"` match arm, and the `USAGE` string
- `Cargo.lock` - records `xtask`'s new dependency edge onto the already-present `serde_json`

## Decisions Made

- Decision D-03 delivered exactly as resolved during planning: a manual,
  `cargo metadata`-derived table, no `cargo-deny`, no `deny.toml`.
- The rendered file's date is produced by shelling out to `date -u
  +%Y-%m-%d` via `std::process::Command`, rather than computed by hand, so
  `licences.rs` carries no bespoke calendar arithmetic that would need to
  fight the workspace's `arithmetic_side_effects` clippy wall.
- `CHANGELOG.md`'s format (plain reverse-chronological Markdown, one level
  two heading per release) is stated once at the top of the file itself,
  since no earlier file in this repository set a precedent to follow.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `LICENSES.md` and `CHANGELOG.md` both exist, are re-derivable or
  self-describing, and pass every automated check the plan specified.
- `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and
  `cargo test --workspace` all pass with the new code in place (94 tests in
  the `xtask` binary, including 5 new `licences` module tests; full
  workspace suite green).
- Ready for 06-06 (the claim surface check) and 06-07 (the acceptance run
  and the release tag), both of which depend on this plan per the phase
  wave order.

## Self-Check: PASSED

- Created files verified present on disk: `crates/xtask/src/licences.rs`,
  `LICENSES.md`, `CHANGELOG.md`.
- Both task commits verified in `git log`: `d8531ed`, `baffb0f`.
- All plan-level `<verification>` commands re-run and passing: the
  licences re-derivation diff, the four CHANGELOG.md automated checks, and
  the full three-command gate (`cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo test --workspace`).

---
*Phase: 06-version-1-0*
*Completed: 2026-09-14*
