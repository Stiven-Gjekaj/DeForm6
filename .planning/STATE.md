---
gsd_state_version: "1.0"
current_phase: 1
current_phase_name: It reads the file
status: planning
stopped_at: Completed 01-07-PLAN.md
last_updated: "2026-09-07T15:53:37.478Z"
last_activity: 2026-09-07
last_activity_desc: "Plan 01-07 executed. inspect takes a byte slice and returns a Report, and it reads all 44 corpus executables. 115 tests pass. A measurement over the 44 project files showed that the object count is wTotalObjects and not wCompiledObjects, which the plan, CONTEXT.md and STRUCTURES.md all named."
state_head: 5fc9144c3c66a7b075e433b25047e08280fff8e5
progress:
  total_phases: 6
  completed_phases: 0
  total_plans: 8
  completed_plans: 7
  percent: 88
---

## Continue

**Resumed:** 2026-09-07. Waves 1 to 5 are complete. Wave 6, plan 01-08, is
next, and it is the last plan of the phase.

The whole library exists. `deform6::inspect` takes a byte slice and returns a
`Report`, and it reads all 44 corpus executables: every one is native, every
one names `MSVBVM60.DLL`, and every object count equals the number its `.vbp`
declares. 115 tests pass. Two compile-fail proof scripts run in the gate and
between them refuse eleven bad shapes. `crates/deform6-cli/src/main.rs` is
still `fn main() {}`.

**Plan 01-08 must not print the object count from `wCompiledObjects`.** Plan
01-07 measured both count fields against all 44 project files:
`wTotalObjects` is the declared object count in 44 of 44 and
`wCompiledObjects` in 29 of 44, because `wCompiledObjects` is the capacity of
the object array. `Report.object_count` already holds the right number. D-05
in 01-08-PLAN.md still names the wrong field. See `STRUCTURES.md` section 4.1.

**To continue:**

    /gsd-execute-phase 1

**The wave order is fixed and the tooling does not know it.**
`gsd-tools query init.execute-phase` reports every plan as runnable at once.
That is wrong: 01-02 cannot write `read/region.rs` before 01-01 creates the
workspace it lives in. Use the order from ROADMAP.md:

    [01-01 done]  [01-02 done, 01-03 done]  [01-04 done]  [01-05 done, 01-06 done]  [01-07]  [01-08]

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
Plan: 7 of 8 in current phase
Status: Executing. Waves 1 to 5 complete, wave 6 next, which is plan 01-08.
Open defects: `.planning/WINDOWS.md` holds three, all of them limits that are
stated rather than hidden. The section overlap rule has no test because no
corpus file overlaps. The P-code branch has no real sample because every
vendored project is native. `inspect` drops the defects it collects, because
`Report` derives `PartialEq` and `Defect` does not.
Last activity: 2026-09-07 - Plan 01-07 executed. `inspect` takes a byte slice and returns a `Report`, and it reads all 44 corpus executables. 115 tests pass. Seven deliberate breakages, and a measurement over the 44 project files showed the plan named the wrong object count field.

Progress: [█████████░] 88% of the plans in phase 1, which is 7 of 8

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

Last session: 2026-09-07T15:53:37.469Z
Stopped at: Completed 01-07-PLAN.md
the plan checker, and repaired. The checker returned PASS WITH CONCERNS with
two blockers and four warnings. All were applied, none were disputed, and the
planner found one further defect the checker missed: the zero filled tail test
was pointed at a file that has no such section, so it would have passed
vacuously. It now uses `corpus/public-domain/PassGen/PassGen.exe`, whose
`.data` declares 8948 virtual bytes against 4096 raw.

Next: `/gsd-execute-phase 1`. Wave order is
[01-01] [01-02, 01-03] [01-04] [01-05, 01-06] [01-07] [01-08].
Resume file: None
