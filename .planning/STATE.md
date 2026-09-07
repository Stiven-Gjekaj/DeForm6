---
gsd_state_version: "1.0"
current_phase: 2
current_phase_name: The object graph
status: executing
stopped_at: "Phase 2 wave 1 complete and merged: 02-01, 02-06, 02-07"
last_updated: "2026-09-08T00:00:00.000Z"
last_activity: 2026-09-08
last_activity_desc: "Phase 2 wave 1 executed and merged. The object array walk bounded by wTotalObjects, the Declare import table and the external component table, and the independent .vbp harness reader with the five written exclusion rules. 185 tests pass: 145 library, 21 support_selftest, 9 refusal, 9 cli, 1 corpus_sweep. The harness names deform6 zero times, proved by grep."
state_head: d404830a26633780a583368c93c41cc6e28b3fac
progress:
  total_phases: 6
  completed_phases: 1
  total_plans: 50
  completed_plans: 11
  percent: 22
---

## Continue

**Phase 2, wave 1 is complete and merged.** Next is wave 2.

    /gsd-execute-phase 2

Wave order, which the tooling does not know and will not tell you:

    [02-01 done, 02-06 done, 02-07 done]
    [02-02, 02-03]
    [02-04, 02-05]
    [02-08]
    [02-09, 02-10]

**Three things wave 1 established that later plans depend on.**

- `DefectKind::UnreadablePointer` now exists, `Recoverable`, as the leaf-pointer
  twin of `UnmappedAddress`, which stays `Fatal` because a spine pointer that
  maps nowhere stops the walk. Plans 02-01 and 02-06 both hit this; 02-06
  worked around it and its workaround can now be simplified.
- The harness reader is independent and proved so by grep. It must stay that
  way. A harness that shares a reader agrees with a bug in that reader.
- Two corpus numbers were corrected by measurement and later plans reuse them:
  the 40-line prefix scan matches 2 of 53 forms, not zero, and
  `Grayscale-effect/pdOpenSaveDialog.cls` declares six non-public procedure
  slots, not two.

**A tooling defect to work around.** `gsd-tools query state.*` recalculates the
global progress block as a side effect, and a worktree agent cannot see its
siblings, so every parallel-wave agent writes a wrong global figure. All three
wave 1 agents corrupted it independently. The orchestrator owns this file after
a parallel wave; do not trust an agent's edit to it.

**Executors run in a worktree.** Verify what lands on `main` after each wave
rather than assuming the merge did the right thing.

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-09-07)

**Core value:** A person who has only a compiled VB6 executable gets back a
Visual Basic project that opens in the VB6 IDE, with the forms and the names
intact, and a report that says how much of it is proved and how much is
inferred.
**Current focus:** Phase 1 complete - Phase 2, The object graph, is next

## Current Position

Phase: 1 of 6 (It reads the file) - complete
Plan: 8 of 8 in current phase - complete
Status: Phase 1 done. `deform6 inspect` runs end to end on a real corpus
file and on all 44. Ready to plan Phase 2.
Open defects: `.planning/WINDOWS.md` holds three, all of them limits that are
stated rather than hidden. The section overlap rule has no test because no
corpus file overlaps. The P-code branch has no real sample because every
vendored project is native. `inspect` drops the defects it collects, because
`Report` derives `PartialEq` and `Defect` does not.
Last activity: 2026-09-07 - Plan 01-08 executed. The `deform6` binary, six exit codes, the locked eight-line report, and the three test files that turn the phase's ROADMAP success criteria into commands. 134 tests pass across the workspace. Nine deliberate breakages; two of the plan's own predicted measurements did not hold and are recorded.

Progress: [░░░░░░░░░░] 0% of the plans in phase 1, which is 8 of 8

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: -
- Total execution time: -

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**

- Last 5 plans: -
- Trend: -

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P04 | 1 session | 3 tasks | 1 files |
| Phase 01 P07 | 1 session | 3 tasks | 4 files |
| Phase 01 P08 | 1 session | 3 tasks | 4 files |
| Phase 02 P01 | 1 session | 3 tasks | 6 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Roadmap: The phase order is fixed by the user. Six phases. Do not reorder.
- Roadmap: The safety primitives go in Phase 1, before the first parser. A lint
  wall added later fires on every line already written.
- Roadmap: The `.frm` writer and the `.frx` writer are one component in one
  plan (04-04). The `.frx` offset is a synthesised cursor, not a stored field.
- Roadmap: VER-05 is introduced in Phase 2. Phase 3 and Phase 4 each raise the
  pinned numbers.
- Roadmap: Phase 6 owns no new requirement. It measures the whole set and turns
  the result into the released documentation.
- [Phase 1]: 01-04: DeForm6 owns one address to file offset predicate. mapped_len is min(virtual_size, size_of_raw_data) with a strict less-than test, and object's contains_rva and pe_file_range_at are never called.
- [Phase 1]: 01-04: A case the corpus cannot exercise gets a fixture the test builds in memory. has_clr_header, the two import descriptors and the address that differs from its file offset all come from one.
- [Phase 1]: The object count comes from wTotalObjects at 0x2A, not wCompiledObjects at 0x2C. Measured against the .vbp of all 44 corpus programs: 44 of 44 against 29 of 44. wCompiledObjects is the capacity of the object array.
- [Phase 1]: A count disagreement is reported only when the capacity is below the count. Reporting inequality would attach a defect to 15 of the 44 corpus files.
- [Phase 1]: Report carries runtime_dll and signature read from the file, because a one variant Runtime enum makes an equality assertion a tautology.
- [Phase 1]: 01-08: Cli::try_parse with a hand-written, exhaustive Refusal-to-Exit match, never Cli::parse or process::exit. A usage error is exit 5, never exit 2.
- [Phase 1]: 01-08: The corpus sweep's header-string check cross-checks Report's fields against a second, independent header read by field identity, not only an ascending-offset check, because the offsets alone cannot see a field-swap bug in Report construction.
- [Phase 1]: vb/object.rs resolves lpObjectArray independently rather than extending ObjectTableHead, so plan 02-06 keeps sole ownership of vb/project.rs
- [Phase 1]: DefectKind::UnreadablePointer added to error.rs as the Recoverable leaf-pointer twin of the Fatal UnmappedAddress, for a name pointer that resolves nowhere

### Pending Todos

None yet.

### Blockers/Concerns

- REQUIREMENTS.md counted 36 v1 requirements. There are 42 IDs. The
  Verification category, VER-01 to VER-06, was left out of the total. The
  traceability table is corrected.
- The corpus is 44 executables and all of them are native. The P-code branch of
  `ProjectInfo.lpNativeCode` is untested by construction. Do not claim it.
- Two derived data tables have no public source and must be built from a type
  library dump in Phase 3: the opcode-to-property table per control type, and
  the event name table per control type.
- STRUCTURES section 11 holds 19 gaps, of which 2 are closed. Each open one is
  named as a risk on the phase where it lands.

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

## Session Continuity

Last session: 2026-09-07T22:12:36.234Z
Stopped at: Completed 02-01-PLAN.md

Phase 1 is complete: all eight plans executed, 134 tests pass across the
workspace, and every ROADMAP success criterion for the phase was run and
confirmed rather than assumed.

Next: `/gsd-execute-phase 2` to plan and execute Phase 2, The object graph.
Resume file: None
