---
phase: 03-forms
plan: 12
subsystem: docs
tags: [research-correction, gap-closure, scope-bytes, property-loop, position-block]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-04's corpus-measured scope-separator grammar (Length - 1, the ScopeRun variants, the menu special case) and plan 03-06's corpus-measured property-loop bound and position-block escape width, both already committed in their own SUMMARY files and now fed back into 03-RESEARCH.md"
provides:
  - "03-RESEARCH.md's zero-children reconciliation passage marked disproven as a general bound, with the shipped Length - 1 bound and 03-06-SUMMARY.md cited beside it"
  - "03-RESEARCH.md's illustrative read_position_block sample corrected to read the same 16-byte span it returns as consumed, matching the shipped propstream.rs"
  - "03-RESEARCH.md's illustrative read_scope_run sample corrected to match the shipped ScopeRun variants, with 03-04-SUMMARY.md, STRUCTURES.md section 8.9, and WINDOWS.md finding 7 all cited"
affects: [phase-4-planning]

actuals:
  tokens: 1798
  tasks: 3
  commits: 3
plan_head_before: 345b8e9

tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - .planning/phases/03-forms/03-RESEARCH.md

key-decisions:
  - "This plan does not run `gsd_run query requirements.mark-complete` for FRM-01 or FRM-03, even though the plan's own frontmatter names both. 03-VERIFICATION.md already found REQUIREMENTS.md overstates both IDs as complete (FRM-01 is 49/53 forms, not every form; FRM-03's opcode-table scope is a documented partial by design). This plan corrects a document's own stale prose; it recovers no additional form and decodes no additional property. Marking either ID complete again would repeat the exact overstatement the verifier already named. REQUIREMENTS.md's checkboxes for FRM-01 and FRM-03 are left as the verifier's own prior pass set them."
  - "The plan's own Task 1 acceptance criterion, a whole-file grep asserting no em-dash character appears anywhere in 03-RESEARCH.md, cannot pass as a literal whole-file check: the file already carried 10 em-dash characters, in unrelated table rows (the Architectural Responsibility Map and the Environment Availability table, both using em-dash as a table-cell placeholder), before this plan's first commit (confirmed against the pre-plan HEAD, 345b8e9). This is a pre-existing condition this plan's own scope boundary forbids fixing (unrelated to the three passages this plan corrects). This plan introduces zero new em-dash characters; every line this plan added was checked individually and carries none."

patterns-established: []

requirements-completed: []

coverage:
  - id: D1
    description: "The property-loop bound passage in the zero-children reconciliation is marked disproven as a general bound, and a correction names the shipped Length - 1 expression from propstream.rs, citing 03-06-SUMMARY.md"
    requirement: "FRM-03"
    verification:
      - kind: other
        ref: "grep -c '03-06-SUMMARY' .planning/phases/03-forms/03-RESEARCH.md (non-zero) && grep -n 'Length - 1' (present) && grep -c 'Length - 2' (non-zero, original kept)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The illustrative read_position_block sample no longer contradicts itself: it reads the same 16-byte span it reports as consumed, matching the shipped propstream.rs::read_position_block, with a sentence citing 03-06-SUMMARY.md"
    requirement: "FRM-03"
    verification:
      - kind: other
        ref: "grep -c 'checked_add(12)' .planning/phases/03-forms/03-RESEARCH.md (non-zero) && grep -c '03-06-SUMMARY' (>= 2)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The illustrative read_scope_run sample no longer joins 0x02 and 0x03 in one arm; it matches the shipped ScopeRun variants and the menu special case, cites 03-04-SUMMARY.md and Grayscale.exe, points at STRUCTURES.md section 8.9, and records WINDOWS.md finding 7 as still open"
    requirement: "FRM-01"
    verification:
      - kind: other
        ref: "sed range fn read_scope_run..### Pattern 3 | grep -c 'ScopeRun::' (>= 4) && grep -c '03-04-SUMMARY' (non-zero) && grep -c 'WINDOWS.md' (non-zero) && same range grep -c 'MAX_SCOPE_RUN' (non-zero)"
        status: pass
    human_judgment: false

duration: 35min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 12: 03-RESEARCH.md Gap-Closure Corrections Summary

**Corrects three passages in `03-RESEARCH.md` that plans 03-04 and 03-06 already disproved against real corpus bytes and worked around, but never fed back into the document itself.**

## Performance

- **Duration:** 35 min
- **Started:** 2026-09-10 (session start)
- **Completed:** 2026-09-10
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- The zero-children reconciliation passage (around line 523) still states the original `Length - 2`/`Length + 2` SVBD-derived formulas as "internally consistent," but that sentence is now marked `[DISPROVEN AS A GENERAL BOUND]`, with a correction directly beneath it naming the exact bound the shipped code uses (`block_end = u32::from(length).saturating_sub(1)` in `crates/deform6/src/vb/propstream.rs`) and citing `03-06-SUMMARY.md`'s `key-decisions` block as the source of the measurement.
- The illustrative `read_position_block` sample no longer contradicts itself. It used to peek two bytes, then read four `i32` values starting two bytes further in (18 bytes total from `at`) while returning `16` as its own consumed count. The corrected sample re-reads the same 16-byte span the short form's own four `i16` values would occupy, starting at `at`, matching the shipped `crates/deform6/src/vb/propstream.rs::read_position_block` exactly. A sentence beneath the sample names `03-06-SUMMARY.md` as the plan that settled which reading ships.
- The illustrative `read_scope_run` sample no longer joins `0x02` and `0x03` in one match arm. The corrected sample defines the same `ScopeRun` variants (`OpenChild`, `Sibling`, `EndForm`, `Menu`, `Unrecognised`) the shipped `crates/deform6/src/vb/controltree.rs` returns, including the `current_is_menu` parameter and the menu special case. A sentence names `03-04-SUMMARY.md` and the four real transitions measured in `corpus/vb6-code/Grayscale-effect/Grayscale.exe`, sends the reader to `STRUCTURES.md` section 8.9 as the authoritative grammar statement, and records that `.planning/WINDOWS.md` finding 7 leaves the two-level menu-close transition open.
- Every original disproven sentence and sample stays in the document, readable, with the correction placed directly beside it rather than replacing it. A reader sees the claim was tested and answered.

## Task Commits

Each task was committed atomically:

1. **Task 1: Correct the property-loop bound in the zero-children reconciliation** - `f52d06e` (docs)
2. **Task 2: Correct the position-block escape's own byte count** - `545b766` (docs)
3. **Task 3: Correct the scope-run sample and point at the authoritative grammar** - `5125126` (docs)

**Plan metadata:** commit follows this SUMMARY.

_Note: this plan changes documentation only. `AGENTS.md`'s mandatory gate (`cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`) was run before every commit, per `AGENTS.md`'s "every one of these, before every commit, not a selection," even though no Rust file changed._

## Files Created/Modified

- `.planning/phases/03-forms/03-RESEARCH.md` - three corrected passages: the zero-children property-loop bound reconciliation, the `read_position_block` illustrative sample, and the `read_scope_run` illustrative sample

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The most consequential: this plan does not mark FRM-01 or FRM-03 complete in `REQUIREMENTS.md`, even though the plan's own frontmatter names both, because `03-VERIFICATION.md` already found both IDs overstated and this plan recovers no additional form and decodes no additional property; it only corrects a document's own stale prose.

## Deviations from Plan

### Auto-fixed Issues

None - every correction landed exactly as the plan's own action text describes, using the exact expressions read from the shipped source files named in each task's `read_first` list.

### Notable Finding (not an auto-fix, no code touched)

**1. [Scope boundary - plan-defect finding, matching 03-06-SUMMARY's own precedent] Task 1's whole-file em-dash acceptance check cannot literally pass**
- **Found during:** Task 1, running the em-dash source assertion after the first edit
- **Issue:** The plan's own acceptance criterion for Task 1 is `! grep -q $'\xe2\x80\x94' .planning/phases/03-forms/03-RESEARCH.md`, a whole-file check. `.planning/phases/03-forms/03-RESEARCH.md` already carried 10 em-dash characters before this plan's first commit (confirmed directly against `345b8e9`, the commit immediately before this plan's Task 1 commit), in two unrelated tables (the Architectural Responsibility Map and the Environment Availability table), both using em-dash as a table-cell placeholder for "not applicable."
- **Fix:** None applied. Fixing those 10 pre-existing occurrences is out of this plan's scope: `AGENTS.md`'s "Scope boundary" rule reserves auto-fixes for issues the current task's own changes cause, and these predate every commit this plan makes. Every line this plan itself added was checked individually for the em-dash byte sequence (`\xe2\x80\x94`) after each edit and before each commit; none was found. The intent behind the acceptance criterion (this plan introduces no em-dash) is satisfied; its literal whole-file form is not, because of content this plan did not write.
- **Files modified:** none (finding only)
- **Verification:** `git show 345b8e9:.planning/phases/03-forms/03-RESEARCH.md | grep -c $'\xe2\x80\x94'` gives `10`, all present before this plan's first commit; a per-edit grep restricted to each task's own added line range gives `0` for all three tasks
- **Committed in:** not applicable, no fix committed

---

**Total deviations:** 0 auto-fixed. One scope-boundary finding recorded above, matching the same class of finding 03-06-SUMMARY documented for its own unsatisfiable acceptance check.
**Impact on plan:** None on the three corrections themselves; all three landed as specified. The finding above is recorded so a future reader is not confused when the literal whole-file em-dash assertion does not pass on its own.

## Issues Encountered

None beyond the notable finding above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. This plan writes no code and stubs nothing; it corrects prose and illustrative samples in an existing document.

## Threat Flags

None. This plan's own `<threat_model>` names four threats (T-03-61 through T-03-64), all `mitigate`, all satisfied: every corrected figure is read from a committed SUMMARY or the shipped source file and named as such (T-03-61); every disproven claim is marked, not deleted (T-03-62, checked directly, `grep -c 'Length - 2'` still non-zero after Task 1); Task 3 forbids restating the grammar and points at `STRUCTURES.md` section 8.9 instead (T-03-63); every task's `verify` step ran `git show --name-only --format= HEAD` and confirmed the commit names exactly one file, never a path under `crates/` (T-03-64).

## Next Phase Readiness

- `03-RESEARCH.md` no longer states the disproven `Length - 2` bound, the self-contradicting position-block byte count, or the symmetric scope-byte pair as confirmed formulas. A phase 4 planner who reads only this document now implements the formulas that hold against real corpus bytes, with a citation trail back to the plan that measured each one.
- `03-VERIFICATION.md` question 6 (research drift) is addressed: the three corrections it names as missing are now present in `03-RESEARCH.md` itself, not only in `03-04-SUMMARY.md` and `03-06-SUMMARY.md`.
- REQUIREMENTS.md's FRM-01 and FRM-03 checkboxes are unchanged by this plan and remain whatever state `03-VERIFICATION.md`'s own prior pass left them in; this plan does not touch requirement completion state.
- No blockers for the remaining gap-closure plans in this run (03-13 through 03-17, per the outstanding-plans list).

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED
