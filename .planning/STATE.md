---
gsd_state_version: "1.0"
current_phase: 2
current_phase_name: The object graph
status: complete
stopped_at: "Phase 2 complete, verified, gaps audited"
last_updated: "2026-09-08T00:00:00.000Z"
last_activity: 2026-09-08
last_activity_desc: "Phase 2 complete. inspect reports the object graph: 105 of 105 objects by name and kind, 185 of 185 public procedures, prototypes with argument names, types, ByRef and recovered Optional defaults, and the Declare table. The differential gate compares all 44 programs against their original source in both directions. 308 tests. Verified PASS, gaps audited, and one untested branch found and closed."
state_head: 3fd74203b151c48646077d52bf13138a07f4f9a4
progress:
  total_phases: 6
  completed_phases: 2
  total_plans: 50
  completed_plans: 18
  percent: 36
---

## Continue

**Phase 2 is complete, verified and audited.** Next is phase 3, Forms.

    /gsd-plan-phase 3

**What phase 3 inherits, measured rather than assumed.**

The corpus is strong for the intrinsic control set and weak in one named place.
`.planning/research/CORPUS.md` holds the counts: 143 Label, 90 CommandButton,
88 TextBox, 87 PictureBox, 76 Menu across 22 forms, 71 CheckBox, 54 Form,
31 Frame. Container nesting is real. **48 controls carry an `Index` property**,
so the unresolved control array index location can be settled from the corpus.

**The hole is third party controls.** The whole corpus declares two `Object=`
lines and holds three control instances, all `MSWinsockLib.Winsock`. FRM-04
needs a synthetic fixture, built the way `has_clr_header` and the event
descriptor walk were, and a synthetic result must never be presented as a
corpus result.

**Absent entirely: 0 MDIForm, 0 UserControl, 0 PropertyPage.** The MDIForm type
value is in no public source and the corpus cannot close it. Plan 02-02 built
and proved the `Unknown` path with a synthetic value, so an unknown kind is
reported rather than refused.

**A defect species to keep hunting.** The phase 2 verifier found a branch with
no test at all: `is_plausible_identifier` could be replaced with `true` and all
307 tests still passed, because every corpus program fails earlier at address
resolution. It is closed now. Phase 3 has the same shape of risk wherever a
validation rule guards against data the corpus does not contain.

**Open in `.planning/WINDOWS.md`:** four findings, none blocking phase 3.

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
