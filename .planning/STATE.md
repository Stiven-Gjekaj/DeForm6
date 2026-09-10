---
gsd_state_version: "1.0"
current_phase: 03
current_phase_name: Forms
status: executing
stopped_at: Completed 03-05-PLAN.md
last_updated: "2026-09-10T11:44:31.724Z"
last_activity: 2026-09-10
last_activity_desc: Phase 03 execution started
state_head: 29c893e5fc609e8085971fda134e4a39376a8c26
progress:
  total_phases: 6
  completed_phases: 0
  total_plans: 28
  completed_plans: 23
  percent: 0
---

## Continue

**Phase 3 is planned.** Ten plans in five waves. Next is execution.

    /gsd-execute-phase 3

Read `.planning/phases/03-forms/03-CONTEXT.md` first. Its four decisions bind
every plan. D-01 withdraws the ROADMAP instruction to commit an opcode table.

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
**Current focus:** Phase 03 — Forms

## Current Position

Phase: 03 (Forms) — EXECUTING
Plan: 6 of 10
Status: Ready to execute
`deform6 inspect` reports the object graph. It reads a form's control tree
now: every control's type, name and array index, gated on the tiling check.
Open defects: `.planning/WINDOWS.md` holds three, all of them limits that are
stated rather than hidden. The section overlap rule has no test because no
corpus file overlaps. The P-code branch has no real sample because every
vendored project is native. `inspect` drops the defects it collects, because
`Report` derives `PartialEq` and `Defect` does not.
Last activity: 2026-09-10 — Phase 03 execution started
Plan 03-01 leads with an end to end tracer. Plan 03-02 withdraws the ROADMAP
instruction to commit an opcode table built from a type library dump, because
`AGENTS.md` bars a fixture calculated from a third party file. Plan 03-04
re-measures the scope-separator grammar directly against Grayscale.exe,
since the research document's own prose does not reconcile byte-for-byte,
and closes STRUCTURES.md gap 11.

Progress: [░░░░░░░░░░] 0% of the 50 plans in the roadmap, which is 18 of 50

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
| Phase 03 P01 | 56min | 3 tasks | 10 files |
| Phase 03 P03 | 18min | 3 tasks | 3 files |
| Phase 03 P02 | 46min | 3 tasks | 9 files |
| Phase 03 P04 | 53min | 3 tasks | 5 files |
| Phase 03 P05 | 22min | 2 tasks | 2 files |

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
- [Phase 03]: Refusal::Damaged stays &'static str crate-wide; a local damaged(String)->Refusal helper in gui.rs uses Box::leak for the two refusals that must name a runtime byte offset.
- [Phase 03]: A full sweep of all 54 corpus .frm forms found LockWorkStation is the only zero-children form; no constant is written for the measured 3-byte tail, per 03-RESEARCH.md's own fallback.
- [Phase 03]: 03-02: safe-provenance subset leaves out conditional/unwidth-stated STRUCTURES.md rows (ScaleMode, ClientLeft/Top/Width/Height, List, DataSource, DataFormat) rather than guess a payload width.
- [Phase 03]: 03-02: derive-opcode-table's default output path is derived/opcode-table.toml (not top-level) so git check-ignore, called with the extracted .gitignore path, does not see a leading slash as an OS-absolute path.
- [Phase 03]: 03-04: the scope-separator grammar is implemented from this session's own byte-level measurement against Grayscale.exe, not from 03-RESEARCH.md's approximate Length+2/Length-2 prose, which does not reconcile against four independent real transitions. A control block's own separator starts at blockStart + Length - 1; 0x02 pops and continues; 0x03 is the sibling terminal with no pop of its own; a menu control (cType 19) reads a bare 0x02 as an OpenChild-equivalent terminal instead.
- [Phase 03]: 03-04: STRUCTURES.md gap 11 closed. The control array Index is the two byte value at control block offset 0x05, not cId as the published array header table states.
- [Phase 03]: 03-04: corpus/vb6-code/Custom-image-filters/CustomFilters.exe, named in the plan text, does not exist in this corpus; the real ExeName32 is Custom_Filters.exe.
- [Phase 03]: 03-05: A length field that does not fit at the given offset is read as declared length 0, matching vb/controltree.rs::read_control_header's own precedent for a count field it cannot read; the downstream declared-end bound check still catches an offset that leaves no room for anything.
- [Phase 03]: 03-05: The landing check has two independent halves, byte-count match and a trailing-null check at a fixed position; ASCII always trivially matches the byte-count half, so only the trailing-null half can ever refuse an ASCII attempt, and that same check applies identically on retry.

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

Last session: 2026-09-10T11:44:31.708Z
Stopped at: Completed 03-05-PLAN.md

Phase 1 is complete: all eight plans executed, 134 tests pass across the
workspace, and every ROADMAP success criterion for the phase was run and
confirmed rather than assumed.

Next: `/gsd-execute-phase 3` to continue Phase 3 execution with plan 03-05.
Resume file: None
