---
phase: 03-forms
plan: 11
subsystem: docs
tags: [requirements, roadmap, event-structure, d-02, gap-closure]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "03-09's report_events and EventReport, the three honest states (Named, BoundUnnamed, Unbound) the amended text now describes; 03-CONTEXT.md's D-02 decision text, cited at both amendment sites"
provides:
  - "FRM-06 in REQUIREMENTS.md, narrowed to event structure (which slots are bound, their index, the handler address) and never to event names, citing D-02"
  - "The Phase 3 goal, a new sixth success criterion, the Overview sentence, the phase list entry and the wave 3 plan-list line in ROADMAP.md, all narrowed to event structure"
  - "The event name table named risk in ROADMAP.md marked withdrawn, in the same shape the gap 15 / D-01 withdrawal already uses, with its original text kept readable"
affects: ["03-forms (verification re-run)", "phase 4 planning (reads ROADMAP.md's Phase 3 requirements)"]

actuals:
  tokens: 1334
  tasks: 2
  commits: 2
plan_head_before: 4c9a384

tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "The Wave 3 plan-list line for 03-09-PLAN.md also carried the withdrawn phrase 'event handler names.' The task's own prose said not to touch the Phase 3 plan list, but the task's own <verify> and <acceptance_criteria> both assert the withdrawn phrase is gone from the whole file with no scoping. The verify gate is the authoritative, blocking check; the prose is scope guidance. Made the minimal surgical edit (wording only, no checkbox, no reorder, no other content change) to satisfy the gate without otherwise touching the list."
  - "The sixth ROADMAP.md success criterion cites a corpus program actually run during this plan: `deform6 inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe`, the same program the plan's own <verification> block names. The frmFire control's slot 8 is bound and slots 0-7 and 9-30 are unbound, confirmed by a live run before the criterion text was written, not assumed."

patterns-established: []

requirements-completed: [FRM-06]

coverage:
  - id: D1
    description: "FRM-06 in REQUIREMENTS.md states event structure (bound slots, slot index, handler address), states plainly that event names are not recovered, and cites D-02 and 03-CONTEXT.md at the amendment site"
    requirement: "FRM-06"
    verification:
      - kind: other
        ref: "grep -c 'D-02' .planning/REQUIREMENTS.md"
        status: pass
      - kind: other
        ref: "grep -A 8 'FRM-06' .planning/REQUIREMENTS.md | grep -c 'event structure'"
        status: pass
      - kind: other
        ref: "! grep -q 'event handler names' .planning/REQUIREMENTS.md"
        status: pass
      - kind: other
        ref: "git show --name-only --format= 32ccffd (exactly one file, no path under crates/)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The Phase 3 goal, a new sixth success criterion checkable against a live run, the Overview sentence, the phase list entry and the wave 3 plan-list line all name event structure instead of event names; the event name table named risk is marked withdrawn with its text kept readable"
    requirement: "FRM-06"
    verification:
      - kind: other
        ref: "! grep -q 'event handler names' .planning/ROADMAP.md"
        status: pass
      - kind: other
        ref: "sed -n '/### Phase 3: Forms/,/### Phase 4/p' .planning/ROADMAP.md | grep -c 'event structure'"
        status: pass
      - kind: other
        ref: "sed -n '/### Phase 3: Forms/,/### Phase 4/p' .planning/ROADMAP.md | grep -c 'D-02'"
        status: pass
      - kind: other
        ref: "sed -n '/### Phase 3: Forms/,/\\*\\*Named risks\\*\\*/p' .planning/ROADMAP.md | grep -cE '^  6\\.'"
        status: pass
      - kind: other
        ref: "./target/debug/deform6 inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe | grep 'event slot' (live run backing the sixth criterion)"
        status: pass
      - kind: other
        ref: "git show --name-only --format= d0be98b (exactly one file, no path under crates/)"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 11: Narrow FRM-06 to Event Structure Summary

**FRM-06 and the Phase 3 goal amended in REQUIREMENTS.md and ROADMAP.md to describe event structure (bound slots, slot index, handler address) instead of event names, citing decision D-02, with a live run against Fast_Flames.exe backing the new sixth success criterion.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-09-10T20:30:00Z (approximate)
- **Completed:** 2026-09-10T20:57:40Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `REQUIREMENTS.md`'s FRM-06 bullet now names the three facts `report_events` gives (which event slots are bound, the index of each slot, the native address of each bound handler), states in one sentence that the tool does not recover the name of an event, and cites D-02 and `03-CONTEXT.md` for the reason.
- `ROADMAP.md`'s Phase 3 goal closing clause is amended to event structure with every other clause (control tree, parent, type and name, property values, CLSID, resource blobs) left unchanged.
- A new sixth Phase 3 success criterion is added, checkable against a live run: `deform6 inspect` on `corpus/vb6-code/Fire-effect/Fast_Flames.exe`, actually run during this plan, shows the `frmFire` control's slot 8 bound and slots 0-7 and 9-30 unbound, with every slot stating `not decoded` rather than a guessed name.
- The Overview paragraph, the Phase 3 phase-list entry, and the Wave 3 plan-list line for `03-09-PLAN.md` are all reworded from "event handler names" to "event structure" / "event slot structure," so the withdrawn wording is gone from the whole file.
- The event name table named risk is marked withdrawn in a new note, in the same shape the existing D-01 / gap 15 withdrawal note already uses: it names the decision (D-02), names the `AGENTS.md` rule behind it, and states what the phase delivers instead. The named risk's own original text is untouched and still readable.

## Task Commits

Each task was committed atomically:

1. **Task 1: Narrow FRM-06 in REQUIREMENTS.md and cite D-02** - `32ccffd` (docs)
2. **Task 2: Amend the Phase 3 goal, success criteria and named risk in ROADMAP.md** - `d0be98b` (docs)

**Plan metadata:** commit follows this SUMMARY.

## Files Created/Modified

- `.planning/REQUIREMENTS.md` - FRM-06 bullet narrowed to event structure, citing D-02
- `.planning/ROADMAP.md` - Phase 3 goal, Overview sentence, phase list entry, a new sixth success criterion, the wave 3 plan-list line, and a new named-risk withdrawal note

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The most consequential: the Wave 3 plan-list line for 03-09-PLAN.md also carried the withdrawn "event handler names" phrase, and the task's own hard `<verify>` and `<acceptance_criteria>` both assert whole-file absence with no scoping to the four named edit sites. Made the minimal wording-only edit to that one line (no checkbox change, no reorder) to satisfy the gate, rather than leave a known-failing verify unresolved.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Task 2's own prose and its own verify gate disagreed over the Wave 3 plan-list line**
- **Found during:** Task 2, running `! grep -q 'event handler names' .planning/ROADMAP.md` after the four named edits
- **Issue:** Task 2's action text says "Do not touch the Phase 3 plan list or the wave note," but the Wave 3 plan-list line for `03-09-PLAN.md` reads "event handler names and the honest report for a slot with no name," which the task's own `<verify>` and `<acceptance_criteria>` both require gone from the whole file with no scoping to the four listed edit sites.
- **Fix:** Reworded only the withdrawn phrase on that one line, from "event handler names" to "event slot structure." No checkbox, ordering, or other content on the line changed.
- **Files modified:** `.planning/ROADMAP.md`
- **Verification:** `! grep -q 'event handler names' .planning/ROADMAP.md` passes; `git diff` on that line shows only the phrase substitution.
- **Committed in:** `d0be98b` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug: a conflict between the task's own scope prose and its own hard verify gate, resolved in favor of the gate).
**Impact on plan:** Necessary for the plan's own acceptance criterion (withdrawn wording gone from the whole file) to hold. No scope creep: the fix is a one-line wording substitution with no structural change to the plan list.

## Issues Encountered

None beyond the deviation above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. This plan changes documentation only; no source file, symbol, or test was touched. `git show --name-only --format= HEAD` on both commits names exactly one file each, neither under `crates/`.

## Threat Flags

None. This plan's own `<threat_model>` names four threats (T-03-57 through T-03-60), all mitigated as stated: the amended text names only the three facts `report_events` gives today, read from `controlinfo.rs` before either edit was written; both amendment sites cite D-02 and the file that holds it; every edit was a scoped `Edit` call inside the named section, never a whole-file rewrite; and both commits were checked with `git show --name-only --format= HEAD` to confirm no path under `crates/` was touched.

## Next Phase Readiness

- FRM-06 and the Phase 3 goal now describe a capability this repository is allowed to build and already built. A re-run of `/gsd-verify-work` against Phase 3 should score this truth against the amended wording rather than the withdrawn one.
- No blockers for the remaining gap closure plans (03-12 through 03-17).

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

Both modified files found on disk: `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`. Both task commits (`32ccffd`, `d0be98b`) found in `git log`. Each commit's `git show --name-only --format=` names exactly one file, neither under `crates/`. All four automated `<verify>` checks per task pass (8 total, shown above). `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` all pass clean before each commit. The plan's own `<verification>` block was re-run after both commits: FRM-06 and the Phase 3 section both read as shown above, and the live `deform6 inspect` run against `Fast_Flames.exe` confirms the event slot output the sixth success criterion describes.
