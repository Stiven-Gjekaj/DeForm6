---
gsd_state_version: "1.0"
current_phase: 1
current_phase_name: It reads the file
status: complete
stopped_at: Completed 01-08-PLAN.md
last_updated: "2026-09-07T16:40:00.000Z"
last_activity: 2026-09-07
last_activity_desc: "Plan 01-08 executed. The deform6 binary parses with Cli::try_parse and maps every Refusal to one of six exit codes; the eight-line report is read from Report::runtime_dll and Report::signature, never a literal. 134 tests pass across the workspace: 115 library, 9 refusal.rs, 1 corpus_sweep.rs (all 44 executables), 9 cli.rs. Nine deliberate breakages run and reverted; two of the plan's own predictions did not match measurement and are recorded in the SUMMARY. Phase 1 is complete."
state_head: c259684
progress:
  total_phases: 6
  completed_phases: 1
  total_plans: 8
  completed_plans: 8
  percent: 100
---

## Continue

**Phase 1 is complete.** All eight plans executed. `deform6 inspect` runs end
to end: it reads a compiled Visual Basic 6 executable, prints the locked
eight-line shape, exits 0, and changes no file on disk; a VB5 file, a VB4
file, a .NET file and a non-PE file each exit a distinct, locked code; and
`cargo test --workspace` runs the whole gate, 134 tests, including a sweep
over all 44 corpus executables and nine refusal tests built from fixtures
patched in memory.

**Two measurements in 01-08-PLAN.md's own prompt did not match what running
the code actually showed**, and both are recorded in
`01-08-SUMMARY.md` rather than silently corrected to fit: forcing
`PeImage::has_clr_header` to a constant `false` fails three tests across the
workspace, not one, because plans 01-04 and 01-06 each already carry a unit
test that asserts the method directly; and the check that catches a swap of
`exe_name`/`title` in `Report` construction is a cross-check against an
independently-read header, not the raw ascending-offset check the plan calls
"the ordering assertion" — the offsets alone are blind to that class of bug.

**To continue:**

    /gsd-execute-phase 2

**Executors run in a worktree, not the primary checkout.** Verify what lands on
the main branch after each wave rather than assuming the merge did the right
thing.

**A test that the corpus alone cannot make fail is the recurring fault across
this phase.** Plan 01-04 found four of them in its own plan; plan 01-08 hit
the same limit again: filling `Report::runtime_dll` from the constant
`VB6_DLL` instead of the name `runtime_of` actually matched passes the whole
sweep, because every corpus file's imported name upper-cases to exactly that
constant, so no fixture on this corpus can tell the two apart. Only
`crates/deform6-cli/src/`'s grep for the bare literal, and `cargo clippy`'s
unused-binding check on the library composer, close that gap. Three corpus
facts that hide a bug are worth carrying into Phase 2: data directory 14 is
zero in all 44 files, both usable corpus files import exactly one DLL, and
in both of them the import directory sits in a section where the address
and the file offset are the same number. Build the fixture in memory when
the corpus cannot answer.

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

Progress: [██████████] 100% of the plans in phase 1, which is 8 of 8

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

Last session: 2026-09-07T16:40:00.000Z
Stopped at: Completed 01-08-PLAN.md

Phase 1 is complete: all eight plans executed, 134 tests pass across the
workspace, and every ROADMAP success criterion for the phase was run and
confirmed rather than assumed.

Next: `/gsd-execute-phase 2` to plan and execute Phase 2, The object graph.
Resume file: None
