---
phase: 03-forms
plan: 14
subsystem: forms
tags: [vb6, control-tree, scope-bytes, menu, gap-closure, review-fix]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-04's ControlTree/ScopeRun/walk and its own single-level scope-separator measurement method; plan 03-10's MAX_UNEXPLAINED_TAIL defensive bound and WINDOWS.md finding 7; plan 03-REVIEW.md finding WR-02"
provides:
  - "read_scope_run's stack_top_is_menu rule: closes a menu nested two levels deep back to a sibling menu at the form's own top level, measured against five real transitions across two programs (HexScroll.exe, UUID2.exe)"
  - "FrmHex and frmUUID2 recover their full control tree, verified parent for parent against the committed .frm by name"
  - "frmPassGen recovers its full control count (39), with one open, documented nesting ambiguity"
  - "close_walk no longer carries dead pop-count code (review finding WR-02, fixed)"
  - "xtask update-ratios writes the four form/control keys plan 03-10 added, closing the gap 03-10-SUMMARY.md itself named"
  - "STRUCTURES.md section 15 and gap 14's own register row record the measured rule and what remains open"
  - "WINDOWS.md finding 7 fixed; two new findings (Map Editor's Main form, frmPassGen's menuAbout ambiguity) record what is still open"
affects: [04-writer]

actuals:
  tokens: 16303
  tasks: 3
  commits: 3
plan_head_before: de2e6aa

tech-stack:
  added: []
  patterns:
    - "read_scope_run's role decision depends on TWO facts, not one: whether the control just read is itself a menu (current_is_menu, plan 03-04's own finding), and whether the parent stack's own top is itself a menu (stack_top_is_menu, this plan's own finding). When the stack top is a menu, 0x03 pops and continues (the role 0x02 plays for every other control) and 0x02 becomes the sibling terminal, at whatever pop count has accumulated."
    - "close_walk takes no stack and no pop count at all, following review finding WR-02's second named fix: the terminal EndForm pop count routinely exceeds the real parent stack depth on real corpus data, and unlike a mid-walk pop, an excess terminal pop closes nothing that still matters because no more siblings follow."

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/controltree.rs
    - crates/deform6/tests/ratios.rs
    - crates/xtask/src/main.rs
    - tests/ratios.toml
    - .planning/research/STRUCTURES.md
    - .planning/WINDOWS.md

key-decisions:
  - "The single-condition rule plan 03-04 measured (a bare 0x02 after a menu always means OpenChild) is corpus-measured but under-determined: this plan found a real corpus transition (frmUUID2's menuLicense to menuSep, and the identical shape in HexScroll's own menuAbout section) where the byte-identical pattern needs Sibling instead. The correct condition adds a second fact: whether the parent stack's own top (not only the control just read) is itself a menu. Found only because a test asserted the recovered parent by name rather than trusting that a form recovering without refusing meant success — the differential gate does not check parent relationships, only flat name/type/index membership, a pre-existing gap this plan did not close (out of this plan's own files_modified scope)."
  - "frmPassGen.frm's own menuHelp section exposes a THIRD shape (a menu, itself a sibling within an already-open menu, that genuinely opens its own child) that is byte-identical to the confirmed sibling case above and cannot be told apart from it: no byte in the control header or the property stream up to the separator distinguishes the two, in this session's own measurement. Because frmUUID2's own exact-parent recovery is a hard, literal acceptance criterion for this plan, and because the two contexts are provably indistinguishable from data controltree.rs reads, the stack_top_is_menu rule is kept as measured and confirmed (two independent programs). frmPassGen's own recovery keeps this one open, documented, narrowly-scoped limitation rather than a guess: menuAboutForm, menuSeparatorC and menuWebsite recover with menuHelp as their parent instead of menuAbout. Recorded in the code's own doc comment and in WINDOWS.md, not silently shipped as solved."
  - "close_walk's review finding WR-02 has two named acceptable fixes: make the check real (call apply_pops, matching the mid-walk pop discipline), or drop the dead code. This session tried the first and measured that it refuses real corpus data: Grayscale.exe, UUID2.exe and HexScroll.exe all reach EndForm with a pop count larger than the real parent stack depth at that point, because no more siblings follow a terminal pop, unlike a mid-walk one where an excess pop misplaces every later sibling. Took the second fix: close_walk no longer takes the stack or the pop count."
  - "xtask update-ratios did not write the four form/control keys plan 03-10 added to tests/ratios.toml (03-10-SUMMARY.md's own documented gap, never given its own WINDOWS.md ledger row despite the SUMMARY's own prose calling it 'finding 8'). format_entry now renders all seven fields (the two procedure counts plus the four form/control counts) as one block, the single function both the writer and every MOVED UP/REGRESSION message use, and the writer measures the four new fields from the same Report deform6::inspect already builds, via differential::forms_controls_counts. Closing this was necessary (Rule 3, blocking): task 1's own acceptance criteria requires cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml to pass, which is impossible while the writer drops fields the committed file already carries."
  - "REQUIREMENTS.md's FRM-01, FRM-02 and VER-06 checkboxes are left unchanged by this plan. FRM-01 ('every form') and FRM-02 ('every control') are still not literally met: Map Editor.exe's Main form still refuses. VER-06 was already independently verified in plan 03-10 and this plan touches no code related to it; its checkbox state is not this plan's own artifact to correct."

patterns-established: []

requirements-completed: []

coverage:
  - id: D1
    description: "read_scope_run's stack_top_is_menu rule closes a menu nested two levels deep back to a sibling menu at the form's own top level; FrmHex and frmUUID2 both recover their full control tree, verified parent for parent against the committed .frm by name"
    requirement: "FRM-01"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::frm_hex_gives_menu_file_and_menu_about_as_form_level_siblings_with_their_own_children"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::frm_uuid2_gives_three_top_level_menus_with_their_own_children_by_name"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::grayscale_gives_the_form_its_own_nine_direct_children_by_name"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::a_pop_count_that_would_empty_the_stack_still_refuses_and_names_the_offset"
        status: pass
    human_judgment: false
  - id: D2
    description: "frmPassGen recovers its full control count (39, matching the .frm's own Begin VB. count); one group of parent relationships (menuAboutForm, menuSeparatorC, menuWebsite under menuAbout) is not settled by this session's own measurement and is recorded, not silently shipped as correct"
    requirement: "FRM-01"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::frm_pass_gen_recovers_every_declared_menu_control_by_name"
        status: pass
    human_judgment: true
    rationale: "The parent-child structure for menuAbout's own children is a known, documented open gap (WINDOWS.md, new finding), not asserted by any test; a human reviewing this plan's own claim should read that finding before treating frmPassGen as fully solved."
  - id: D3
    description: "close_walk performs no computation whose result nothing reads; review finding WR-02 is closed by dropping the stack and pop-count parameters entirely, proven by a direct test and by the unchanged recovered tree across the whole corpus"
    requirement: "VER-06"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::close_walk_no_longer_refuses_an_end_form_pop_count_larger_than_the_stack_depth"
        status: pass
      - kind: other
        ref: "cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml"
        status: pass
    human_judgment: false
  - id: D4
    description: "tests/ratios.toml is re-pinned from a fresh cargo run -p xtask -- update-ratios, never hand-edited; the totals rise to 52 of 53 forms and 686 of 686 controls recovered, and xtask now writes the four form/control keys it previously dropped"
    requirement: "VER-06"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/ratios.rs#the_pinned_file_holds_fifty_two_of_fifty_three_forms_and_six_hundred_eighty_six_of_six_hundred_eighty_six_controls"
        status: pass
      - kind: unit
        ref: "crates/xtask/src/main.rs#tests::the_writers_per_entry_block_is_byte_for_byte_format_entrys_output"
        status: pass
      - kind: other
        ref: "cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml"
        status: pass
    human_judgment: false

duration: single session
completed: 2026-09-11
status: complete
---

# Phase 3 Plan 14: The Two-Level-Deep Menu Close Summary

**Measured the byte grammar for closing out of a menu nested two levels deep back to a sibling menu, across five real transitions in two programs; `FrmHex` and `frmUUID2` now recover in full and match their `.frm` source parent for parent, `frmPassGen` recovers its full control count with one open, documented nesting gap, `close_walk` no longer carries dead pop-count code, and the pinned counts rise to 52 of 53 forms and 686 of 686 controls.**

## Performance

- **Duration:** single session
- **Tasks:** 3
- **Files modified:** 6 (0 created, 6 modified)

## Accomplishments

- **The measured rule.** `read_scope_run` now decides a menu's own closing role from two facts: whether the control just read is itself a menu (`current_is_menu`, plan 03-04's own finding), and whether the parent stack's own top is itself a menu (`stack_top_is_menu`, this plan's own finding). When the stack top is a menu, `0x03` pops and continues (the role `0x02` plays for every other control) and `0x02` becomes the sibling terminal. Five independent real transitions across two programs (`HexScroll.exe`, `UUID2.exe`) confirm this, all verified against each program's own `.frm` source by name.
- **Byte-level evidence, before the rule was written.** For the three transitions the plan named (`FrmHex` offset `0x16fd`, `UUID2.exe` offsets `0x1918` and `0x1986`): the real bytes are `0xFF 0x03 0x02`, and reading `0x03` as an immediate terminal (plan 03-04's own single-level rule) stops the run two bytes too early, leaving the real `0x02` byte to be misread as the start of a bogus next control block — the exact and only cause of the `MAX_UNEXPLAINED_TAIL` refusal. This session additionally measured a fourth and fifth transition (`UUID2.exe` offset `0x19cc`, `HexScroll.exe` offset `0x1743`, both `menuLicense` to `menuSep`) that plan 03-04's own single-condition rule (bare `0x02` after a menu always means `OpenChild`) mis-nests silently, with no refusal at all — caught only because a test asserted the recovered parent by name.
- **What the rule does not settle.** `frmPassGen.frm`'s own `menuHelp` section exposes a third shape: a menu, itself a sibling within an already-open menu, that genuinely opens its own child. Its own trailing separator (`PassGen.exe`, file offset `0x21d0`) is byte for byte identical to the confirmed sibling case, and no byte this module reads distinguishes the two. `frmPassGen` still recovers every declared menu control by name (39, matching the `.frm`'s own count), but `menuAboutForm`, `menuSeparatorC` and `menuWebsite` recover with `menuHelp` as their parent instead of `menuAbout`. Documented in the code's own doc comment and in a new `WINDOWS.md` finding, not silently shipped as solved.
- **`close_walk` (review finding WR-02).** The function used to compute a pop count against the parent stack and never read the result. This session tried making the check real (call `apply_pops`, matching the mid-walk discipline) and measured that it refuses real corpus data: the terminal `EndForm` pop count routinely exceeds the real stack depth on `Grayscale.exe`, `UUID2.exe` and `HexScroll.exe`, because no more siblings follow a terminal pop. Took the finding's other named fix: `close_walk` no longer takes the stack or the pop count at all.
- **`xtask update-ratios` writes all four form/control keys.** Plan 03-10's own writer never wrote `form_declared`/`form_recovered`/`control_declared`/`control_recovered`, a gap its own SUMMARY documented but never gave a ledger entry. `format_entry` now renders all seven fields as one block; the writer measures the four new ones from the same `Report` `deform6::inspect` builds. Closing this was required for task 1's own acceptance criterion (`cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml`) to be satisfiable at all.
- **Re-pinned totals:** 52 of 53 forms and 686 of 686 controls recovered (up from 49 of 53 and 607 of 607), the output of a fresh `xtask update-ratios` run, never a hand edit.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: Measure the transition and replace the defensive bound** - `0cd20e9` (feat, tdd="true")
2. **Task 2: Make close_walk perform the check it appears to perform** - `0db014c` (fix, tdd="true")
3. **Task 3: Write the finding into the research register** - `bd71f57` (docs, no source file)

**Plan metadata:** commit follows this SUMMARY.

_Note: `AGENTS.md`'s "put the code and its tests in the same commit" rule takes precedence over the plan's own implicit RED-then-GREEN commit split, matching every prior plan in this phase. Each task's own RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task._

## Files Created/Modified

- `crates/deform6/src/vb/controltree.rs` - `read_scope_run`'s `stack_top_is_menu` parameter and rule; `close_walk`'s signature drops `stack`/`pops`; new tests for `FrmHex`, `frmUUID2`, `frmPassGen`, the crafted pop-past-root refusal, and `close_walk`'s own pop-count independence
- `crates/deform6/tests/ratios.rs` - `format_entry` renders all seven fields; `check_program`/`check_program_forms_controls`/message builders thread `FormsControlsCounts` through; new totals test (52/53 forms, 686/686 controls)
- `crates/xtask/src/main.rs` - `measure_all` computes and writes the four form/control keys via `forms_controls_counts`; test literals and the byte-for-byte formatting test updated for the new seven-field shape
- `tests/ratios.toml` - re-pinned via `cargo run -p xtask -- update-ratios`; header comment states the new totals
- `.planning/research/STRUCTURES.md` - gap 14's own register row narrowed (not closed) with the measured evidence; new section 15 records the transition, the bytes, the programs, and the rule, plus what remains open
- `.planning/WINDOWS.md` - finding 7 marked fixed; two new findings record Map Editor's own separate cause and frmPassGen's own menuAbout ambiguity

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The most consequential: `stack_top_is_menu` is kept as measured and confirmed (two independent programs, five transitions) even though it does not resolve `frmPassGen`'s own third shape, because `frmPassGen`'s ambiguous transition is byte-identical to `frmUUID2`'s own confirmed-correct transition, and `frmUUID2`'s exact recovery is a hard, literal acceptance criterion for this plan. The alternative (refusing whenever this exact byte pattern occurs) would also refuse `frmUUID2`, which this plan cannot do.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `xtask update-ratios` did not write the four form/control keys plan 03-10 added**
- **Found during:** Task 1, running the plan's own required verify step (`cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml`)
- **Issue:** Running the command deleted all 176 form/control lines from the committed `tests/ratios.toml` (03-10-SUMMARY.md's own documented, never-ledgered gap): the writer only ever called `format_entry` with the two procedure counts.
- **Fix:** `format_entry` now takes and renders all seven fields; `xtask`'s `measure_all` computes the four new ones via `differential::forms_controls_counts` and `deform6::inspect`, from the same table (`OpcodeTable::builtin()`) `crates/deform6/tests/ratios.rs` already uses for the same purpose.
- **Files modified:** `crates/deform6/tests/ratios.rs`, `crates/xtask/src/main.rs`
- **Verification:** `cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml` (passes after the fix); `crates/xtask/src/main.rs#tests::the_writers_per_entry_block_is_byte_for_byte_format_entrys_output`
- **Committed in:** `0cd20e9` (Task 1 commit)

**2. [Rule 1 - Bug] Plan 03-04's own single-condition menu rule silently mis-nests a real corpus control**
- **Found during:** Task 1, writing the dedicated `frmUUID2` parent-by-name test the plan's own acceptance criteria require
- **Issue:** With only `current_is_menu` deciding a bare `0x02`'s role, `frmUUID2` built a tree with no refusal, but `menuSep` recovered as `menuLicense`'s own child instead of its sibling (both children of `menuAbout`) — a silent wrong tree, the exact failure mode this repository's own gate exists to catch, and invisible to the corpus-wide differential gate, which checks flat name/type/index membership, not parent relationships.
- **Fix:** Added `stack_top_is_menu` as a second condition (see key-decisions), measured against two independent programs (`UUID2.exe`, `HexScroll.exe`).
- **Files modified:** `crates/deform6/src/vb/controltree.rs`
- **Verification:** `frm_uuid2_gives_three_top_level_menus_with_their_own_children_by_name`, `frm_hex_gives_menu_file_and_menu_about_as_form_level_siblings_with_their_own_children`
- **Committed in:** `0cd20e9` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug).
**Impact on plan:** Both were necessary: the first to satisfy the plan's own required verify step, the second to satisfy the plan's own explicit prohibition against a silently mis-nested tree. No scope creep beyond `controltree.rs`, `ratios.rs` and `xtask/main.rs`, all named or implied by the plan's own artifact table.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's own named criterion ("The executor changes the new rule to the reading it replaced, sees the `FrmHex` tree test fail, records the failure output in the SUMMARY, and reverts before committing")** - `read_scope_run`'s match arms were reverted to plan 03-04's own single-condition rule (`0x02 if current_is_menu && pops == 0 => OpenChild`, no `stack_top_is_menu`). `cargo test -p deform6 --lib vb::controltree::tests::frm_hex_gives_menu_file_and_menu_about_as_form_level_siblings_with_their_own_children` run once:

```
thread 'vb::controltree::tests::frm_hex_gives_menu_file_and_menu_about_as_form_level_siblings_with_their_own_children' panicked at crates/deform6/src/vb/controltree.rs:1284:62:
walk failed: this Visual Basic 6 executable is damaged: the control tree walk at file offset 0x16ff would leave 141 bytes unaccounted for, more than the 8 byte margin this repository trusts as an unexplained footer; refusing rather than silently dropping what those bytes might hold
```

Reverted to the `stack_top_is_menu`-based rule before committing `0cd20e9`.

**Task 2's own named criterion ("The executor inverts the chosen behaviour on purpose, sees the new test fail, records the output in the SUMMARY, and reverts before committing")** - `close_walk` was changed to unconditionally return `Refusal::Damaged("RED-PHASE DELIBERATE BREAKAGE")` before its own tail accounting. `cargo test -p deform6 --lib vb::controltree::tests::close_walk_no_longer_refuses_an_end_form_pop_count_larger_than_the_stack_depth` run once:

```
thread 'vb::controltree::tests::close_walk_no_longer_refuses_an_end_form_pop_count_larger_than_the_stack_depth' panicked at crates/deform6/src/vb/controltree.rs:1565:13:
close_walk must not refuse on account of the pop count: this Visual Basic 6 executable is damaged: RED-PHASE DELIBERATE BREAKAGE
```

Reverted to the real tail-accounting body before committing `0db014c`.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

- **`frmPassGen`'s own `menuAbout` children** (`corpus/public-domain/PassGen/PassGen.exe`, offset `0x21d0`): `menuAboutForm`, `menuSeparatorC` and `menuWebsite` recover with `menuHelp` as their parent instead of `menuAbout`. Byte-identical to a confirmed sibling case; not a guess, an open, measured gap. Tracked in `.planning/WINDOWS.md` (new finding).
- **`Map Editor.exe`'s `Main` form** still refuses: "expected a scope separator (0xFF) at file offset 0x170e, found 0x37." Not the tail-bound refusal this plan resolved; a distinct, unexplained failure this session measured but did not settle. Tracked in `.planning/WINDOWS.md` (new finding) and named in `STRUCTURES.md` gap 14's own row.
- **The differential gate does not check parent/nesting relationships**, only flat name/type/array-index membership (pre-existing, not introduced by this plan). This is why the `menuSep`-as-`menuLicense`'s-child bug (deviation 2, above) passed the corpus-wide differential test while being wrong. Out of this plan's own `files_modified` scope (`differential.rs` is not named); worth a future plan's attention given it is the exact class of bug this repository's own gate exists to catch.

## Threat Flags

None. Every trust boundary this plan's own `<threat_model>` names is mitigated as stated: T-03-71 (a guessed rule producing a mis-nested tree) by keeping the two required forms' own recovered parents asserted against the committed `.frm` by name, and by refusing to stretch the rule to `frmPassGen`'s own unsettled third case; T-03-72 (the tail bound widened) — `MAX_UNEXPLAINED_TAIL` is unchanged, its doc comment still names what remains unexplained; T-03-73 (unbounded scope run) — `MAX_SCOPE_RUN` is unchanged, its own test still passes; T-03-74 (a pop count emptying the parent stack) — `apply_pops` is unchanged and still refuses mid-walk, proven by a dedicated test; T-03-75 (a hand-edited ratio) — the re-pinned counts come from a fresh `xtask update-ratios` run, proven by the source assertion; T-03-76 (one ledger entry hiding an open cause) — `WINDOWS.md` finding 7 is fixed for the transition it named, and the two remaining open causes (`Map Editor.exe`, `frmPassGen`) each have their own separate entry with their own byte offset.

## Next Phase Readiness

- `ControlTree`/`walk` recover 52 of 53 forms and 686 of 686 controls; the remaining gap (`Map Editor.exe`'s `Main` form, and `frmPassGen`'s own `menuAbout` nesting) is named, measured where possible, and tracked in `WINDOWS.md`, not hidden.
- `close_walk` carries one discipline for its own trailing tail accounting, and no dead computation.
- `STRUCTURES.md` and `WINDOWS.md` both reflect the current, honest state of the scope-separator grammar; a future plan researching `Map Editor.exe`'s own separate failure, or `frmPassGen`'s own byte-identical ambiguity, starts from an accurate register rather than a stale one.
- No blockers for the remaining gap-closure plans in this run.

---
*Phase: 03-forms*
*Completed: 2026-09-11*

## Self-Check: PASSED
