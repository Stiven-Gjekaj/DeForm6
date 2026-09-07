---
gsd_state_version: "1.0"
current_phase: 1
current_phase_name: It reads the file
status: planning
stopped_at: Completed 01-04-PLAN.md. Wave 3 done. Wave 4 is 01-05 and 01-06.
last_updated: "2026-09-07T15:07:13.298Z"
last_activity: 2026-09-07
last_activity_desc: "Plan 01-04 executed. PeImage reads a real binary and resolves an address to a file offset through one predicate. 58 tests pass. Six deliberate breakages found four tests in the plan that the corpus alone could not make fail."
state_head: 5ba4e33538193af1fb1c627c44fefd5ebf8c7d9d
progress:
  total_phases: 6
  completed_phases: 0
  total_plans: 8
  completed_plans: 4
  percent: 0
---

## Continue

**Resumed:** 2026-09-07. Waves 1, 2 and 3 are complete. Wave 4, plans 01-05 and
01-06, is next.

The workspace, the lint wall, the CI gate, the `Region` bounded window, the
`Off`/`Rva`/`Va` newtypes, the error model, the journal and the PE envelope all
exist. 58 tests pass. Two compile-fail proof scripts run in the gate and
between them refuse eleven bad shapes.

`PeImage` reads a real binary. It resolves the entry point of a corpus file to
a file offset through one address map, and `read/pe.rs` is the only file in the
crate that names the `object` crate.

**To continue:**

    /gsd-execute-phase 1

**The wave order is fixed and the tooling does not know it.**
`gsd-tools query init.execute-phase` reports every plan as runnable at once.
That is wrong: 01-02 cannot write `read/region.rs` before 01-01 creates the
workspace it lives in. Use the order from ROADMAP.md:

    [01-01 done]  [01-02 done, 01-03 done]  [01-04 done]  [01-05, 01-06]  [01-07]  [01-08]

**Executors run in a worktree, not the primary checkout.** Verify what lands on
the main branch after each wave rather than assuming the merge did the right
thing.

**A test that the corpus alone cannot make fail is the recurring fault in this
phase.** Plan 01-04 found four of them in its own plan. Three corpus facts that
hide a bug are worth carrying forward: data directory 14 is zero in all 44
files, both usable corpus files import exactly one DLL, and in both of them the
import directory sits in a section where the address and the file offset are
the same number. Build the fixture in memory when the corpus cannot answer.

**Watch plan 01-08.** The plan checker estimated it at 70k tokens with three
tasks and four new test files, and named it the plan most likely to need
splitting during execution.

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-09-07)

**Core value:** A person who has only a compiled VB6 executable gets back a
Visual Basic project that opens in the VB6 IDE, with the forms and the names
intact, and a report that says how much of it is proved and how much is
inferred.
**Current focus:** Phase 1 - It reads the file

## Current Position

Phase: 1 of 6 (It reads the file)
Plan: 4 of 8 in current phase
Status: Executing. Waves 1, 2 and 3 complete, wave 4 next.
Open defect: `.planning/WINDOWS.md` records that the section overlap rule has
no test, because no corpus file has overlapping sections. It can be given a
synthetic test the way `has_clr_header` was.
Last activity: 2026-09-07 - Plan 01-04 executed. PeImage reads a real binary and resolves an address to a file offset through one predicate. 58 tests pass. Six deliberate breakages found four tests in the plan that the corpus alone could not make fail.

Progress: [█████░░░░░] 50%

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
- STRUCTURES section 11 holds 18 open gaps. Each one is named as a risk on the
  phase where it lands.

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

## Session Continuity

Last session: 2026-09-07T15:07:13.288Z
Stopped at: Completed 01-04-PLAN.md. Wave 3 done. Wave 4 is 01-05 and 01-06.
the plan checker, and repaired. The checker returned PASS WITH CONCERNS with
two blockers and four warnings. All were applied, none were disputed, and the
planner found one further defect the checker missed: the zero filled tail test
was pointed at a file that has no such section, so it would have passed
vacuously. It now uses `corpus/public-domain/PassGen/PassGen.exe`, whose
`.data` declares 8948 virtual bytes against 4096 raw.

Next: `/gsd-execute-phase 1`. Wave order is
[01-01] [01-02, 01-03] [01-04] [01-05, 01-06] [01-07] [01-08].
Resume file: None
