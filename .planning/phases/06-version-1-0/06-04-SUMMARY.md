---
phase: 06-version-1-0
plan: 04
subsystem: release
tags: [semver, gitattributes, release-tooling, shell-script]

# Dependency graph
requires:
  - phase: 06-version-1-0
    provides: "06-01's report schema, and the confirmed roadmap scope for this milestone's release mechanics"
provides:
  - "The workspace version at 1.0.0, inherited by all three crates through version.workspace = true"
  - "scripts/check-release-version.sh, which measures the git tag on HEAD against Cargo.toml and is proved able to report a mismatch"
  - "crates/deform6/tests/gitattributes.rs, an audit of the committed .gitattributes against the extension set the writer emits"
  - "The confirmed release strings: version 1.0.0, tag v1.0.0"
affects: [06-07]

# Actuals (#2632)
actuals:
  tokens: 2550
  tasks: 3
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "scripts/prove-*-wall.sh shape reused for a release-mechanics check: header comment naming why the gate commands cannot catch it, set -eu, ROOT resolved from the script's own path, a self test that proves the check can fail"

key-files:
  created:
    - scripts/check-release-version.sh
    - crates/deform6/tests/gitattributes.rs
  modified:
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Decision D-02 confirmed: the workspace version moves to 1.0.0 and the release tag is v1.0.0. After this release, a breaking change to the public library API of the deform6 crate needs a 2.0.0 major bump, not a minor bump."
  - "crates/deform6/tests/gitattributes.rs excludes the writer's own *.report.json output from the audited extension set: it is a diagnostic artifact this tool writes beside a VB6 project, not a file the IDE reads or the corpus commits, matching the same distinction tests/extract_structural.rs's is_report_file helper already draws."
  - ".gitattributes needed no edit. Both new tests pass against the file exactly as it already stood; the plan's own research call was correct that this file was written in an earlier phase."

patterns-established:
  - "A release-mechanics check (the tag-vs-version comparison) follows the same prove-*-wall.sh shape as the existing lint/region/capacity/ordered-output walls: a self-test mode is the probe that proves the check can fail before it is trusted to pass."

requirements-completed: [FRM-05, WRT-01, WRT-05, WRT-06]

coverage:
  - id: D1
    description: "The workspace version is 1.0.0, and every crate inherits it through version.workspace = true."
    requirement: "WRT-05"
    verification:
      - kind: unit
        ref: "cargo run -q -p deform6-cli -- -V"
        status: pass
      - kind: unit
        ref: "grep -c '^version = ' Cargo.toml"
        status: pass
    human_judgment: false
  - id: D2
    description: "scripts/check-release-version.sh measures the git tag on HEAD against Cargo.toml's version, refuses to pass silently when HEAD carries no tag, and its --self-test mode proves the comparison can report a mismatch before the release trusts it to report a match."
    requirement: "WRT-05"
    verification:
      - kind: manual_procedural
        ref: "sh scripts/check-release-version.sh --self-test"
        status: pass
      - kind: manual_procedural
        ref: "sh scripts/check-release-version.sh v1.0.0"
        status: pass
      - kind: manual_procedural
        ref: "sh scripts/check-release-version.sh v9.9.9 (expected exit 1)"
        status: pass
      - kind: manual_procedural
        ref: "sh scripts/check-release-version.sh with no argument on this untagged commit (expected exit 1, named message)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every file extension the writer emits (other than its own JSON report) is named in .gitattributes with a binary or -text rule, measured from deform6::write::project's own output rather than a hand-written list."
    requirement: "WRT-01, WRT-06"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/gitattributes.rs#every_extension_the_writer_emits_is_named_in_gitattributes"
        status: pass
    human_judgment: false
  - id: D4
    description: "*.frx and *.ctx are both marked binary in .gitattributes."
    requirement: "FRM-05"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/gitattributes.rs#frx_and_ctx_are_marked_binary"
        status: pass
      - kind: unit
        ref: "grep -cE '^\\*\\.(frx|ctx)[[:space:]]+binary' .gitattributes (expected 2)"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-09-14
status: complete
---

# Phase 6 Plan 4: Version 1.0.0 and a Measured Release-Tag Check Summary

**Bumps the workspace to 1.0.0, ships a self-testing script that measures the release tag against it, and adds a test that audits the already-correct `.gitattributes` against the extensions the writer actually emits.**

## Performance

- **Duration:** 20 min
- **Tasks:** 3 (one confirmation checkpoint, two `auto` tasks)
- **Files modified:** 4 (`Cargo.toml`, `Cargo.lock`, `scripts/check-release-version.sh`, `crates/deform6/tests/gitattributes.rs`)

## Accomplishments

- Confirmed the release strings recorded by decision D-02: workspace version `1.0.0`, release tag `v1.0.0`. After this release, a breaking change to the public library API of the `deform6` crate needs a `2.0.0` major bump.
- Moved the `[workspace.package]` `version` key in `Cargo.toml` from `0.1.0` to `1.0.0`. All three crates already declared `version.workspace = true`, so no crate manifest needed editing; `Cargo.lock`'s three workspace-member entries updated to match on the next build.
- Added `scripts/check-release-version.sh`. It reads the workspace version, compares it against a tag (given explicitly, or read from `git describe --tags --exact-match` on HEAD), refuses to pass silently when HEAD carries no tag, and carries a `--self-test` mode that plants a wrong tag and proves the comparison reports a mismatch before the release trusts it to report a match.
- Added `crates/deform6/tests/gitattributes.rs`. `every_extension_the_writer_emits_is_named_in_gitattributes` drives `deform6::inspect` then `deform6::write::project` over `corpus/public-domain/PassGen/PassGen.exe`, collects the lower case extension of every file the writer returned (excluding its own `.report.json` output), and asserts each one is named in `.gitattributes` with a `binary` or `-text` rule. `frx_and_ctx_are_marked_binary` asserts `*.frx` and `*.ctx` are both marked `binary`, the roadmap's own named deliverable for this plan.
- `.gitattributes` needed no change. Both new tests pass against the file exactly as committed by an earlier phase.

## Task Commits

Each task was committed atomically:

1. **Task 1: Confirm the move to 1.0.0 and the tag name** - checkpoint decision, no code change; the confirmed strings are recorded in this summary (see Decisions below). No commit — a decision checkpoint carries no file change to stage.
2. **Task 2: The version, and a check that measures the tag against it** - `29bf93f` (feat)
3. **Task 3: Audit .gitattributes against the extensions the writer emits** - `71a314d` (test)

**Plan metadata:** committed as part of this summary's own commit.

## Files Created/Modified

- `Cargo.toml` - the single `version` key in `[workspace.package]` moved from `0.1.0` to `1.0.0`; nothing else in the file changed
- `Cargo.lock` - the three workspace-member version fields (`deform6`, `deform6-cli`, `xtask`) updated to `1.0.0` by a real `cargo build`
- `scripts/check-release-version.sh` - new, executable; reads the manifest version, compares it against a tag, and self-tests its own ability to fail
- `crates/deform6/tests/gitattributes.rs` - new; audits `.gitattributes` against the extension set `deform6::write::project` actually emits, and pins `*.frx`/`*.ctx` as marked binary

## Decisions Made

**Task 1's checkpoint confirmed the two exact strings this plan and plan 06-07 both depend on:**

- **Version string:** `1.0.0`
- **Tag string:** `v1.0.0`
- **The promise this records:** after this release, a breaking change to the public library API of the `deform6` crate (`deform6::inspect`, `deform6::write::project`, `ProjectReport`, `ReportItem`, `Confidence`, `Evidence`, `Defect`, `Site`, `DefectKind`, `Severity`, `SafeName`) needs a `2.0.0` major bump, and a breaking change to the committed `crates/deform6/schema/report.schema.json` top-level shape needs the same. This was Decision D-02, recorded during planning; this checkpoint confirmed the strings rather than re-deciding them, per the plan's own text.

`crates/deform6/tests/gitattributes.rs` excludes `*.report.json` from the audited extension set. That file is a diagnostic artifact this tool writes beside a recovered VB6 project, not a file the VB6 IDE reads or the corpus commits, so `.gitattributes`'s line-ending and binary rules (which exist to protect files a VB6 project holds) do not need to name it. This mirrors the same distinction `tests/extract_structural.rs`'s own `is_report_file` helper already draws for the same reason.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `scripts/check-release-version.sh v1.0.0` and `sh scripts/check-release-version.sh --self-test` both pass on this commit; `sh scripts/check-release-version.sh v9.9.9` exits 1 as required.
- The workspace is at `1.0.0` and `cargo run -q -p deform6-cli -- -V` prints `deform6 1.0.0`.
- Plan 06-07 can use the confirmed strings above (`1.0.0` / `v1.0.0`) directly and does not need to re-derive or re-confirm them. Plan 06-07 creates the `v1.0.0` tag itself, locally, after the acceptance run is green, and pushes nothing unless the human asks.
- `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` all pass clean on this commit (workspace test totals, summed from every `test result:` line this session: 979 tests across all crates, 0 failed).

---
*Phase: 06-version-1-0*
*Completed: 2026-09-14*

## Self-Check: PASSED

- FOUND: `scripts/check-release-version.sh` (executable)
- FOUND: `crates/deform6/tests/gitattributes.rs`
- FOUND: `version = "1.0.0"` in `Cargo.toml`
- FOUND: commit `29bf93f` (Task 2)
- FOUND: commit `71a314d` (Task 3)
- Re-ran acceptance criteria: `cargo run -q -p deform6-cli -- -V` prints `deform6 1.0.0`; `scripts/check-release-version.sh --self-test`, `v1.0.0`, and `! v9.9.9` all pass; `cargo test -p deform6 --test gitattributes` reports 2 passed; `git diff main -- .gitattributes` is empty; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace` all pass clean.
