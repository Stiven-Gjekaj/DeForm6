---
phase: 05-hostility
plan: 01
subsystem: safety
tags: [rust, severity, journal, salvage, cli, error-handling]

requires:
  - phase: 01-foundation
    provides: "Journal and Mode::{Strict, Salvage} in journal.rs, built and tested, wired to nothing"
provides:
  - "Severity::Tolerated, the third rung between Recoverable and nothing"
  - "Journal::record wired into deform6::inspect, so strict mode refuses a Recoverable defect by default"
  - "--salvage on both inspect and extract, threaded as one Mode to every call that reads or writes"
  - "report::assumption_lines, one free text limits line per Recoverable defect in a salvage report"
affects: [05-02, 05-03, 05-04, 05-05, 05-06, 05-07, 05-08]

actuals:
  tokens: 24000
  tasks: 3
  commits: 6
  plan_head_before: 7e3a455138cd7a4c5c14a4bb1c74e2b19caa1e01

tech-stack:
  added: []
  patterns:
    - "A post-hoc policy loop: every read function keeps pushing defects into one local Vec exactly as before; the mode never enters a parse site; inspect() feeds the whole finished list through Journal::record once, after the read, before it returns."
    - "Mechanical signature commits: a parameter that changes a public function's arity, and every one of its call sites, lands alone, with every call site passing the value that reproduces today's behaviour; the actual policy change is a separate, later commit."

key-files:
  created:
    - crates/deform6/tests/severity_census.rs
    - crates/deform6/tests/salvage.rs
  modified:
    - crates/deform6/src/error.rs
    - crates/deform6/src/journal.rs
    - crates/deform6/src/vb/mod.rs
    - crates/deform6/src/report.rs
    - crates/deform6/src/write/mod.rs
    - crates/deform6-cli/src/main.rs
    - crates/deform6-cli/tests/cli.rs
    - crates/deform6/tests/extract_structural.rs
    - crates/deform6/tests/extract_tracer.rs
    - crates/deform6/tests/corpus_sweep.rs
    - crates/deform6/tests/blobs.rs
    - crates/deform6/tests/differential.rs
    - crates/deform6/tests/events.rs
    - crates/deform6/tests/ratios.rs
    - crates/deform6/tests/refusal.rs
    - crates/deform6/src/vb/functyp.rs
    - crates/deform6/src/vb/object.rs
    - crates/deform6/src/vb/project.rs
    - crates/deform6/src/vb/opcodes.rs
    - crates/deform6/src/write/code.rs
    - crates/deform6/src/write/frm.rs
    - crates/xtask/src/main.rs

key-decisions:
  - "Severity holds three values, not two. Tolerated names a defect that costs one item and assumes nothing in its place; Recoverable keeps its old meaning, a defect the reader continued past by assuming a value the file does not state. Eight DefectKind arms move from Recoverable to Tolerated: UnreadablePointer, StructureUnreadable, EmptyName, IndexHighByteSet, GuidLengthUnexpected, OcxReservedFieldUnexpected, BlobLenTooSmall, UnrecoverableString. Five stay Recoverable: ImplausibleCount, SectionOverlap, NoNulTerminator, CountMismatch, ClassNameNoDot."
  - "The mode never enters a read function. inspect() builds a Journal from mode only after every read has finished and the local defect list holds every entry, feeds the whole list through Journal::record in order, and refuses (naming the first Recoverable defect through the new error::refusal_for_defect) only after the whole list has been recorded. This is what keeps the strict list and the salvage list the same list, per roadmap success criterion 3."
  - "Every mode-parameter change (inspect, then write::project) landed as its own mechanical commit first, with every call site passing Mode::Strict, before the commit that actually wires the policy. AGENTS.md's one-change-per-commit rule applied at function-signature granularity, not just file granularity."
  - "Assumption lines go into the existing free text ProjectReport.limits array, one line per Recoverable defect, via a new report::assumption_lines helper. No new field on ProjectReport and no new Confidence value, per research assumption A2: the JSON shape Phase 4 shipped is the shape its consumers expect, and this stays reversible."
  - "--salvage is declared independently on both Command::Inspect and Command::Extract, matching this codebase's existing convention of per-variant flags with no #[command(flatten)] shared struct."
  - "The severity census walks Report::defects alone, not Report::defects plus every FormReport::defects. compose_form already folds every form-level defect into the shared, report-level list before it returns FormReport; walking both double counts the form-scoped ones. The test now asserts this invariant directly (every form-level defect is contained in Report::defects) rather than assuming it."

patterns-established:
  - "A defect list assembled once, policed once, at the end of the one function that owns the whole read: the choke point pattern journal.rs's own doc comment already named, now actually reached from a real call site."

requirements-completed: [SAF-02, SAF-03]

coverage:
  - id: D1
    description: "Severity::Tolerated added, eight DefectKind arms reclassified, and a corpus wide test proves every defect the 44 vendored programs raise is Tolerated"
    requirement: "SAF-02"
    verification:
      - kind: unit
        ref: "crates/deform6/src/journal.rs#journal::tests (8 tests, one per severity/mode pair plus ordering)"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/severity_census.rs#every_defect_the_corpus_raises_is_tolerated"
        status: pass
    human_judgment: false
  - id: D2
    description: "deform6::inspect refuses a Recoverable defect by default (exit 4, names the byte offset); --salvage on inspect continues past it and produces the report"
    requirement: "SAF-02, SAF-03"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/salvage.rs#a_patched_external_count_refuses_in_strict_and_names_the_offset"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/salvage.rs#the_same_patched_file_succeeds_in_salvage_with_one_more_defect"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#inspect_on_a_patched_file_exits_four_and_names_the_offset"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#inspect_with_salvage_on_the_same_patched_file_exits_zero_and_prints_the_report"
        status: pass
    human_judgment: false
  - id: D3
    description: "--salvage on extract writes the project and a report naming every assumption with its byte offset; a strict report names none"
    requirement: "SAF-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/report.rs#tests (build/build_limits, mode threaded)"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/salvage.rs#a_salvage_report_over_the_patched_bytes_holds_the_mode_line_and_one_assumption_line"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/salvage.rs#two_salvage_runs_over_the_same_bytes_give_byte_identical_json"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#extract_with_salvage_on_a_patched_file_writes_the_project_and_the_assumption"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#extract_on_a_patched_file_without_salvage_exits_four_and_writes_nothing"
        status: pass
    human_judgment: false

duration: 1 session
completed: 2026-09-13
status: complete
---

# Phase 5 Plan 01: Strict-by-default with a --salvage escape hatch Summary

**A third severity, `Tolerated`, lets `deform6 inspect`/`extract` refuse any file whose read had to assume a value (exit 4, byte offset named) while still passing every one of the 44 vendored corpus programs; `--salvage` on both subcommands continues past the assumption and names it in the JSON report's own limits array.**

## Performance

- **Duration:** 1 session (includes a `blocking-human` tracer feedback gate, cleared by the human between task 2 and task 3)
- **Tasks:** 3
- **Files modified:** 24 (2 new test files, 22 modified)

## Accomplishments

- `Severity::Tolerated` added to `error.rs`; eight `DefectKind` arms reclassified from `Recoverable` to `Tolerated`; a corpus wide test (`severity_census.rs`) proves every defect the 44 vendored programs raise is `Tolerated`, never higher.
- `Journal::record` wired into `deform6::inspect`: a strict run now refuses on the first `Recoverable` defect, naming it through the new `error::refusal_for_defect`; a salvage run continues past it. Both modes read the same bytes and collect the same defect list, because the whole list is recorded before the policy is applied to any of it.
- `--salvage` added to both the `inspect` and `extract` subcommands, threaded through as one `deform6::journal::Mode`, never a different value to the read call than to the write call.
- `report::assumption_lines` adds one free text limits line per `Recoverable` defect in a salvage run's JSON report, naming the byte offset, the structure, the field, and the defect's own sentence. `ProjectReport` still holds exactly three public fields.

## Task Commits

Each task was committed atomically. Task 2 and task 3 each split into a mechanical signature commit (no behaviour change) followed by the commit that wires the actual policy, per `AGENTS.md`'s one-change-per-commit rule applied at function-signature granularity.

1. **Task 1: The third severity, measured against the corpus** - `6aa638d` (feat)
2. **Task 2a: Mode parameter on `inspect`, mechanical** - `1c67dc8` (feat)
3. **Task 2b: Wire the strict/salvage policy into `inspect`, add `--salvage`** - `1b98753` (feat)
   - *(interleaved, not this executor's commit: `76189e8`, the human's own correction of the plan's planning-time defect count, 430/428/2 to 429/428/1, made after reviewing the tracer feedback gate)*
4. **Task 3a: Mode parameter on `write::project`, mechanical** - `778e950` (feat)
5. **Task 3b: `--salvage` on `extract`, name every assumption** - `ff80249` (feat)

**Plan metadata:** this commit (SUMMARY, STATE, ROADMAP).

## Files Created/Modified

- `crates/deform6/tests/severity_census.rs` - New. Walks all 44 vendored executables and asserts every defect is `Tolerated`.
- `crates/deform6/tests/salvage.rs` - New. The end to end proof: one patched field refuses in strict, salvages, and the report names the assumption.
- `crates/deform6/src/error.rs` - `Severity::Tolerated`, eight reclassified arms, `refusal_for_defect`.
- `crates/deform6/src/journal.rs` - Six match arms (one per severity/mode pair), updated doc comment, two new tests.
- `crates/deform6/src/vb/mod.rs` - `inspect` gains `mode: Mode`, wires `Journal`, updated doc comment.
- `crates/deform6/src/report.rs` - `build`/`build_limits` gain `mode`, new `assumption_lines`.
- `crates/deform6/src/write/mod.rs` - `project` gains `mode: Mode`, threads it to `report::build`.
- `crates/deform6-cli/src/main.rs` - `--salvage` on both subcommands, `mode_for` helper, `run_inspect`/`run_extract` threaded.
- `crates/deform6-cli/tests/cli.rs` - Four new command line tests for the patched file, in both modes, on both subcommands.
- 15 other test/source files - mechanical `, Mode::Strict` argument added at 56 (`inspect`) + 7 (`write::project`) pre-existing call sites, and four pre-existing tests fixed where their own patched defect now refuses under the new default policy (see Deviations).

## Decisions Made

See `key-decisions` in the frontmatter. The one decision worth restating here: the census test originally walked `Report::defects` and every `FormReport::defects`, per this plan's own task 1 instruction, and got 430 total defects, matching the plan's planning-time figure. Tracing the double count down to `compose_form`'s own `defects.extend(form_defects.iter().cloned())` (which already folds every form-level defect into the shared list before returning `FormReport`) showed the true, non-duplicated count is 429 (428 `UnreadablePointer`, 1 `StructureUnreadable`, not 2). The test was fixed to walk `Report::defects` alone and assert the containment invariant, not bent to match the plan's number. The human verified this independently and corrected `05-01-PLAN.md` and `ROADMAP.md` to the measured figures.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Four existing tests asserted the pre-reclassification severity for a defect kind this plan moved to `Tolerated`**
- **Found during:** Task 1, full-workspace test run after reclassifying `DefectKind::severity`
- **Issue:** `object.rs`, `functyp.rs` (twice) and `project.rs` each had a test asserting `defect.kind.severity() == Severity::Recoverable` for `UnreadablePointer` or `GuidLengthUnexpected`, both now `Tolerated`
- **Fix:** Updated the four assertions to `Severity::Tolerated`; renamed one test and its doc comment (`..._with_a_recoverable_defect` to `..._with_a_tolerated_defect`) where the old name baked in the old severity
- **Files modified:** `crates/deform6/src/vb/object.rs`, `crates/deform6/src/vb/functyp.rs`, `crates/deform6/src/vb/project.rs`
- **Verification:** `cargo test --workspace` green
- **Committed in:** `6aa638d`

**2. [Rule 1 - Bug] A test's own patched `CountMismatch` defect now refuses under the new default strict policy**
- **Found during:** Task 2, full-workspace test run after wiring `Journal` into `inspect`
- **Issue:** `vb::tests::a_capacity_below_the_object_count_reaches_the_caller_as_a_defect` patches a `CountMismatch` (`Recoverable`) defect into `Grayscale.exe` and asserted `inspect(..., Mode::Strict).unwrap()`, which now panics on `Err`
- **Fix:** Changed the call to `Mode::Salvage`, which is what the test's own purpose (proving the defect reaches `Report.defects`) actually needs now that strict refuses
- **Files modified:** `crates/deform6/src/vb/mod.rs`
- **Verification:** `cargo test --workspace` green
- **Committed in:** `1b98753`

**3. [Rule 1 - Bug] A test pinned the recompilation sentence as the report's own first limit line**
- **Found during:** Task 3, full-workspace test run after adding the mode line to `build_limits`
- **Issue:** `the_reports_limits_state_the_exact_recompilation_sentence` asserted `limits.first() == RECOMPILATION_LIMIT_STATEMENT`; the new mode line now occupies that position
- **Fix:** Changed the assertion to check the sentence is present (`.iter().any(...)`) rather than first
- **Files modified:** `crates/deform6/tests/extract_structural.rs`
- **Verification:** `cargo test -p deform6 --test extract_structural` green
- **Committed in:** `ff80249`

**4. [Rule 1 - Bug] Stale `Mode` enum doc comments after the third severity landed**
- **Found during:** Task 1, reviewing `journal.rs` after adding `Tolerated`
- **Issue:** `Mode::Strict`'s own doc comment said "Any defect refuses the file", no longer true once `Tolerated` never refuses in either mode
- **Fix:** Rewrote both variants' doc comments to name all three severities' disposition correctly
- **Files modified:** `crates/deform6/src/journal.rs`
- **Verification:** Doc comment reviewed against the shipped `record` match
- **Committed in:** `6aa638d`

**5. [Rule 1 - Bug] The severity census test's own methodology double counted form-scoped defects**
- **Found during:** Task 1, cross-checking the census test's first result (430) against a standalone probe
- **Issue:** Walking `Report::defects` and every `FormReport::defects` double counts a form-scoped defect, since `compose_form` already folds it into the shared list
- **Fix:** Walk `Report::defects` alone; added an assertion proving every form-level defect is already contained in it, rather than assuming so
- **Files modified:** `crates/deform6/tests/severity_census.rs`
- **Verification:** `cargo test -p deform6 --test severity_census -- --nocapture` now reports 429, matching a hand-built probe
- **Committed in:** `6aa638d`

---

**Total deviations:** 5 auto-fixed (4 bug fixes in test assertions/doc comments made stale by this plan's own reclassification, 1 test-methodology correction). **Impact:** all five are necessary corrections to keep the test suite honest about the behaviour this plan actually shipped; none is scope creep.

## Deliberate Breakages (required by the plan, reverted before commit)

- **Task 1:** Changed `DefectKind::UnreadablePointer`'s arm in `severity()` back to `Severity::Recoverable`, ran `cargo test -p deform6 --test severity_census`. It failed, naming every corpus file and offset that now exceeded `Tolerated`, e.g. `.../ScreenCapture.exe: offset 0x19ec, structure Object, field lpProcNamesArray, severity Recoverable: address 0x490049 at offset 0x19ec resolves to nothing, and the item keeps its other fields`. Reverted; gate re-confirmed green.
- **Task 3:** Changed `assumption_lines`'s filter from `severity() == Severity::Recoverable` to `severity() != Severity::Fatal` (keeping `Tolerated` too), ran `cargo test -p deform6 --test salvage`. Three tests failed: the strict report now held 14 assumption lines it should hold none of; the salvage report held 15 assumption lines instead of 1; and `the_defect_array_holds_more_entries_than_the_assumption_line_count` failed with `the Tolerated defects must be reported and not counted as assumptions: 15 defects, 15 assumption line(s)`. Reverted; gate re-confirmed green.

## Issues Encountered

A `blocking-human` tracer feedback gate fired after task 2, per the plan's own `type="tracer"` marking. The gate reported the tracer's proof (one patched field, both modes, both exit codes) and a numeric discrepancy the census test found against the plan's own planning-time figures (430 vs the measured 429). The human reviewed, approved the strict-by-default behaviour change, verified the gate/hook/authorship claims independently, and corrected the plan text to the measured numbers before signalling task 3 to proceed. Not a fault; the gate did what it exists to do.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 05-02 (the bound-check audit, starting with `GuiTable::walk`) can proceed: `Severity::Tolerated` and the wired `Journal` are the load-bearing pieces it builds on, and the corpus census gives it a green baseline to measure against.
- Plan 05-03 (the fuzz target) can call `deform6::inspect(data, &table, Mode::Strict)` and `Mode::Salvage` directly, per the roadmap's own named risk that the target must exercise both.
- No blockers. `cargo test --workspace` is green at 887 passed, 0 failed, the new floor for plan 05-02 onward.

---

*Phase: 05-hostility*
*Completed: 2026-09-13*

## Self-Check: PASSED
