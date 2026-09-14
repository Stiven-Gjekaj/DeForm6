---
phase: 06-version-1-0
plan: 06
subsystem: testing
tags: [ci, gate, shell-script, claim-surface, honesty-audit]

requires:
  - phase: 06-version-1-0
    provides: README.md (06-02), the gate.yml cargo doc step (06-03)
provides:
  - "scripts/check-claim-surface.sh: a re-runnable check that README.md, the CLI --help output and the report vocabulary hold no claim of statement recovery, no claim of compilable Basic from native code, and no figure stated as a share of one hundred"
  - "A gate step that runs the check on every push"
affects: [06-07]

actuals:
  tokens: 2528
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "The prove-*-wall.sh script shape (set -eu, ROOT via CDPATH= cd, WORK=$(mktemp -d) with trap cleanup EXIT, PASS/FAIL lines naming the exact violation) extended to a prose scanner, not just a code scanner"
    - "An audited allow list of exact sentence text, read inside the script and checked by equality, for the case where a legitimate negation shares a regular expression with the claim it negates"
    - "A self-test stage that plants one violation per hunted shape into a copy of every scanned source and requires the same detection function to catch each one, before the script ever reports a pass on the real tree"

key-files:
  created:
    - scripts/check-claim-surface.sh
  modified:
    - .github/workflows/gate.yml

key-decisions:
  - "Decision D-04 (from planning) implemented literally: the check scans exactly README.md, the concatenated --help/inspect --help/extract --help output, and a live extract's report.json plus the three confidence words. tests/ratios.toml is not scanned; the script's header comment states the reason so a later reader does not treat the gap as an oversight."
  - "The audited allow list holds exactly two entries, both from README.md, both negations of the exact claim the stmt-fwd shape hunts. No entry was added to make a real claim pass."
  - "The three-fact check (lpNativeCode, frmHMM.frx, the recompilation sentence) was added as an extra check success criterion 3 also names, per the task's explicit instruction, with its own remove-and-require-failure probe."

requirements-completed: [DET-04, SAF-02, SAF-03]

coverage:
  - id: D1
    description: "scripts/check-claim-surface.sh scans README.md, the --help output and the report vocabulary for six forbidden shapes, and proves detection by planting one violation of each shape into every source before it ever reports a pass"
    requirement: "SAF-02"
    verification:
      - kind: other
        ref: "sh scripts/check-claim-surface.sh (measured: exit 0, 18 of 18 planted violations fired, 3 of 3 fact probes fired)"
        status: pass
      - kind: other
        ref: "measured: a real violation appended to README.md ('DeForm6 recovers 92% of statements from the file.') was caught by the pct-sign and stmt-fwd shapes and made the script exit 1; README.md was restored byte-for-byte and git status --porcelain confirmed clean afterward"
        status: pass
    human_judgment: false
  - id: D2
    description: "The check carries an audited allow list of the exact sentences permitted to hold a forbidden shape, each with its reason, so a new sentence that holds one fails until added on purpose"
    requirement: "SAF-03"
    verification:
      - kind: other
        ref: "sh scripts/check-claim-surface.sh output: the two real README negations report ALLOWED, not FAIL, and the run still exits 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "Decision D-04 is delivered: the check scopes to the three named sources, excludes tests/ratios.toml by name, and states the reason where the next reader finds it"
    requirement: "DET-04"
    verification:
      - kind: other
        ref: "grep -q 'ratios.toml' scripts/check-claim-surface.sh"
        status: pass
    human_judgment: false
  - id: D4
    description: "The check runs in the gate on every push, as the thirteenth named step, after Prove the ordered output wall"
    requirement: "SAF-02"
    verification:
      - kind: other
        ref: "grep -q 'sh scripts/check-claim-surface.sh' .github/workflows/gate.yml; test 13 -eq $(grep -cE '^      - name:' .github/workflows/gate.yml); git diff main -- .github/workflows/gate.yml holds only added lines"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-09-14
status: complete
---

# Phase 6 Plan 06: Claim Surface Check Summary

**A `sh scripts/check-claim-surface.sh` gate step that greps README.md, the CLI's three `--help` surfaces and a live `.report.json` for six forbidden claim shapes, proves it can catch every one by planting a violation of each before it ever reports a pass, and excludes `tests/ratios.toml` by name per decision D-04.**

## Performance

- **Duration:** 20 min
- **Started:** 2026-09-14T18:33:00Z (approximate)
- **Completed:** 2026-09-14T18:53:46Z
- **Tasks:** 2
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- `scripts/check-claim-surface.sh` scans README.md, the concatenated `--help`/`inspect --help`/`extract --help` output, and a live `extract` run's `.report.json` plus the three confidence words, for six forbidden shapes: a percentage figure in digits, a percentage figure in words, a statement-recovery claim in either word order, a compilability claim, and a source-recovery-from-native-code claim.
- Stage 4 of the script plants one violating line per shape into a copy of every one of the three sources (18 combinations) and requires the same detection code to catch each one before the script ever reports a pass on the real tree. It additionally probes success criterion 3's three fact literals (`lpNativeCode`, `frmHMM.frx`, the recompilation sentence) by removing each from a copy and requiring the fact check to fail.
- The check runs on the real, unmodified tree today: exit 0, all 18 planted violations fire, all 3 fact probes fire, and `git status --porcelain` is byte-identical before and after a run.
- The check is wired into `.github/workflows/gate.yml` as the thirteenth named step, `Check the claim surface`, immediately after `Prove the ordered output wall`.
- Proved, as real measured output rather than a claim: appending a genuine violation (`DeForm6 recovers 92% of statements from the file.`) to README.md made the script report two `FAIL` lines (`pct-sign` and `stmt-fwd`) and exit 1; the README was then restored byte-for-byte and the working tree confirmed clean.

## Task Commits

1. **Task 1: The claim surface check, with a planted violation for every shape it hunts** - `4bb8206` (feat)
2. **Task 2: Run the check in the gate** - `e6825ce` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified

- `scripts/check-claim-surface.sh` - the check: collects the three sources into a temp directory, scans them against six forbidden-shape regular expressions, classifies hits against an audited allow list, self-tests detection by planting one violation of every shape into every source, and separately checks and probes success criterion 3's three fact literals
- `.github/workflows/gate.yml` - adds the `Check the claim surface` step, after `Prove the ordered output wall`, with a comment block stating why the three base gate commands cannot catch a prose claim and naming `tests/ratios.toml` as deliberately out of scope

## The audited allow list

Per the plan's own `<output>` instruction, the full allow list, for plan 06-07's honesty audit to review directly rather than re-deriving from the regular expressions:

1. **"It does not recover statements."** (README.md, opening section)
   Reason: the README's own opening negation of the claim the `stmt-fwd` shape hunts (`recover ... statement`), not a claim that the tool recovers statements.

2. **"DeForm6 does not recover statements. The forms, the control trees, the"** (README.md, the full text of the first line of "What version 1.0 does not return")
   Reason: the same negation, restated as the opening claim of the section whose entire purpose is to say what the tool does not do.

No other line in README.md, the `--help` output, or the live report vocabulary matched any of the six forbidden shapes on the real tree. The report vocabulary's three confidence words (`proven`, `inferred`, `unrecoverable`) hold none of the six shapes.

## Decisions Made

- Decision D-04 (settled during planning) implemented literally: the check's three sources are README.md, the concatenated three `--help` surfaces, and the live report plus confidence words. `tests/ratios.toml` is not read by the script at all; the header comment states the reason (pinned test data, not a claim surface a reader meets before running the tool) so a later reader does not "fix" the exclusion.
- The violating lines the self-test plants are held in their own table (`PROBES`), separate from the `SHAPES` table that defines the regular expressions, so a shape's detection regex and its test fixture are not the same piece of text.
- The three-fact check (success criterion 3) was added as the plan's task 1 action explicitly requires, using the same collect-then-probe pattern as the six-shape scanner: a positive presence check on the real tree, then a removal probe per literal that requires the check to fail.

## Deviations from Plan

None - plan executed exactly as written.

(One implementation bug was found and fixed during authoring, before any task's `<verify>` was run: a shell variable name collision inside the first draft, where the fact-check probe loop and the `fact_check` function it called both used a variable named `literal`, silently clobbering the outer loop's value since POSIX `sh` functions share the calling shell's variable scope. This was caught by the script's own output showing blank `FACTPASS` lines, fixed by renaming the function-internal variable, and re-verified before task 1's commit. It never reached a commit in its broken form and is not a deviation from the plan's specification, only a normal authoring correction.)

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Success criterion 2 is now met by a re-runnable, self-proving check, not a grep somebody ran once.
- The audited allow list above is ready for plan 06-07's honesty audit to review directly.
- `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` all pass on the current tree (94 tests in the largest suite; 615+ across the workspace, 0 failed).
- No blockers for plan 06-07.

---
*Phase: 06-version-1-0*
*Completed: 2026-09-14*

## Self-Check: PASSED

- `scripts/check-claim-surface.sh` exists on disk: FOUND
- Commit `4bb8206` (task 1) is in git log: FOUND
- Commit `e6825ce` (task 2) is in git log: FOUND
- `sh scripts/check-claim-surface.sh` re-run at self-check time: exit 0, 18 of 18 PASS, 3 of 3 FACTPASS
- `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`: all pass

## Note on requirement traceability

`requirements.mark-complete DET-04 SAF-02 SAF-03` reported SAF-02 and SAF-03
as `already_complete` (Phase 5) and DET-04 as `table_unmatched`. DET-04's
checkbox is already `[x]` in REQUIREMENTS.md (from Phase 1); the traceability
table row groups it as "DET-01 to DET-06 | Phase 1 | Complete" rather than a
single-ID row, which is why the tool cannot match it as a standalone entry.
This plan's `requirements` frontmatter names these three IDs because its
`<threat_model>` ties the claim-surface check back to the same capabilities
(refusal behaviour) those IDs already cover, not because it introduces a new
requirement. No edit to REQUIREMENTS.md's grouping format was made; that
formatting predates this plan and is out of scope for it.
