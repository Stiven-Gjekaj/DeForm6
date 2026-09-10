---
phase: 03-forms
plan: 04
subsystem: forms
tags: [vb6, control-tree, scope-bytes, tiling, gap-closure]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-01's FormStream, GuiObjectInfo and the Tiling accounting primitive; plan 03-03's independent .frm reader, cited in this plan's own SUMMARY only as a second measurement, not called from src/"
provides:
  - "ControlHeader, read_control_header, read_array_index: the control block header reader for both the non-array and array layouts, STRUCTURES.md section 8.4"
  - "ControlKind, classify_control_type: the cType to name table, carrying an unassigned value raw, following vb/classify.rs's own shape"
  - "ScopeRun, read_scope_run, ControlNode, ControlTree, walk: the scope-separator walk over the whole control tree, gated on gui::Tiling"
  - "STRUCTURES.md gap 11 closed: the control array Index is the two byte value at control block offset 0x05"
  - "GAPS.md's two wrong corpus counts corrected: 3 third party OCX instances (not 0), 48 control array elements across 6 files (not 35 files)"
affects: [03-06-propstream, 03-09-controlinfo, 03-10-differential-gate]

actuals:
  tokens: 14309
  tasks: 3
  commits: 3
plan_head_before: 550e464

tech-stack:
  added: []
  patterns:
    - "the control block's own Length field locates the scope separator that follows it at blockStart + Length - 1, measured across four independent real transitions in Grayscale.exe, not derived from the research document's own approximate Length+2/Length-2 prose"
    - "a menu control (cType 19) reads a bare 0x02 as its own child-opening terminal, distinct from every other control type, where 0x02 is a pop that continues the run; both readings are corpus-measured, not guessed"
    - "a next block position with too few bytes left for a plausible header closes the walk gracefully, accounting whatever remains as a trailing span, the same treatment the zero-children case's own unexplained tail already needed"

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/controltree.rs
    - crates/deform6/src/vb/gui.rs
    - crates/deform6/src/error.rs
    - .planning/research/STRUCTURES.md
    - .planning/phases/02-the-object-graph/GAPS.md

key-decisions:
  - "AGENTS.md's 'put the code and its tests in the same commit' rule takes precedence over the plan's implicit TDD RED-then-GREEN commit split, for both code tasks (tdd=true), matching 03-01's and 03-03's own precedent. Each task's RED phase was run and its failure evidence captured (see Deviations and Break-on-purpose evidence below), then reverted and committed together with the GREEN implementation in one commit per task."
  - "Task 1 and Task 2, though both edit controltree.rs, are still two separate commits: Task 1 ships the header reader and its own synthetic-byte tests; Task 2 adds the scope-separator walk, ControlTree, and the corpus-wide tests that exercise walk() (including the TxtF 25-element test, which the plan's own Task 1 acceptance criteria names but which functionally requires Task 2's own walk to enumerate). The module doc comment and the corpus test for the full TxtF array moved to Task 2's commit for this reason."
  - "The scope byte grammar (STRUCTURES.md section 8.9, 'the least certain part of the entire format') was not implemented from the research document's own prose alone. This session re-measured it directly against corpus/vb6-code/Grayscale-effect/Grayscale.exe: a control block's own separator starts at blockStart + Length - 1 (not the Length+2 / Length-2 the research prose gives, which does not reconcile byte-for-byte against real transitions), 0x02 pops one level and the run continues, and 0x03 is the sibling terminal and adds no pop of its own. Four independent real transitions confirm this exactly (Form to frameShades, frameShades to hscrShades, hscrShades to lblShades, lblShades to frameDecompose)."
  - "A menu control (cType 19) reads a bare 0x02 as an OpenChild-equivalent terminal, not a pop, per this session's own measurement of Grayscale.exe's mnuFile (parent) and mnuOpenImage (child) transition. This is a corpus-proven special case, not a guess: STRUCTURES.md section 8.9 already names SVBD's own separate, heuristic handling for menus."
  - "A next control position with too few bytes left in the property stream for a plausible header (Length field readable, and Length + 2 not exceeding what remains) closes the walk as if it had reached EndForm, rather than refusing. This is the same 'unexplained trailing span, accounted directly' treatment 03-01 already used for the zero-children case's own 3-byte tail (03-RESEARCH.md assumption A3), generalised to the case where a form's stream ends inside a menu section with no further scope byte at all. A Length of 0 always still refuses, because it would not advance the cursor regardless of how many bytes remain."
  - "read_control_header does not read a cId for an array-layout control. The plan's own <behavior> list gives no offset for cId in the array layout (only Index at 0x05, name length at 0x07, name at 0x09, cType at 0x09+n+1), matching this session's own hex evidence. c_id is 0 for an array-layout ControlHeader, documented on the struct."
  - "Two narrowly scoped DefectKind variants, EmptyName and IndexHighByteSet, were added to error.rs (not in this plan's own files_modified list) because no existing variant fits a declared-zero name length or a non-zero array-index high byte, and the plan's own acceptance criteria require a Defect for each. ImplausibleCount, which already existed, covers the 'name length exceeds remaining block' case."
  - "gui.rs gains one new method, FormStream::region() (pub(crate)), because controltree::walk needs to read past the form's own outermost block and FormStream exposed no accessor for its own bytes. Not in this plan's files_modified list either; a Rule 3 blocking-issue fix."
  - "The plan's own read_first list and Task 1's fixture text name corpus/vb6-code/Custom-image-filters/CustomFilters.exe. That file does not exist in this corpus; the .vbp's own ExeName32 is Custom_Filters.exe (with an underscore). Every reference in code and in this plan's own STRUCTURES.md closure section uses the real path, with the discrepancy noted."
  - "Section 5's Menus row in GAPS.md (not one of the plan's named 'two wrong rows') was also corrected, from 17 files to 22 files (76 entries), because it is sourced from the exact same broken corpus/**/*.frm glob this plan's own D-03 assignment already identifies as the cause of the two rows it does name, and 03-RESEARCH.md's own 'State of the Art' table already gives the corrected number (22 files, matching CORPUS.md exactly). Leaving the table's own Menus row at the old, disproven number while correcting the prose sentence beneath it that cites the same figure would have left the document internally contradictory."

patterns-established:
  - "damaged(message: String) -> Refusal in controltree.rs, the same Box::leak escape hatch gui.rs's own damaged() documents, for the same reason: a control block or a scope run that must name a runtime byte offset in its refusal message."

requirements-completed: [FRM-01, FRM-02]

coverage:
  - id: D1
    description: "read_control_header reads both control block layouts (non-array and array), gives the array Index as the two byte value at offset 0x05 with a Defect when the high byte is non-zero, and carries an unrecognised cType raw via classify_control_type"
    requirement: "FRM-02"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::the_three_txt_f_elements_give_index_zero_one_and_two"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::a_two_byte_array_index_with_a_non_zero_high_byte_gives_a_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::a_non_array_header_reads_c_id_name_and_c_type"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::a_declared_name_length_of_zero_gives_an_empty_name_and_a_defect_and_still_gives_c_type"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::a_declared_name_length_larger_than_the_remaining_block_gives_an_empty_name_and_a_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::a_name_byte_of_0xa9_becomes_its_own_latin1_code_point"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::c_type_39_is_carried_raw_and_refuses_no_control"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::a_repeated_index_in_a_synthetic_two_element_array_is_reported_twice_not_deduplicated"
        status: pass
    human_judgment: false
  - id: D2
    description: "The 25 element TxtF array (Custom_Filters.exe) gives Index 0 through 24, and Grayscale's optChannel and optDecompose arrays give Index 2,1,0 and 1,0, every one matching the .frm source"
    requirement: "FRM-02"
    verification:
      - kind: integration
        ref: "crates/deform6/src/vb/controltree.rs#tests::the_txt_f_array_gives_twenty_five_elements_with_index_zero_through_twenty_four"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/controltree.rs#tests::grayscale_gives_opt_channel_two_one_zero_and_opt_decompose_one_zero"
        status: pass
    human_judgment: false
  - id: D3
    description: "walk recovers the correct control tree with the correct parent for every control: LockWorkStation's zero children, SK-Gradient-Sample's three flat children, and Grayscale's nine direct form children including a menu with its own nested submenu, and Tiling::finish succeeds on all three"
    requirement: "FRM-01"
    verification:
      - kind: integration
        ref: "crates/deform6/src/vb/controltree.rs#tests::lock_work_station_gives_one_root_form_with_zero_children_and_tiling_holds"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/controltree.rs#tests::sk_gradient_sample_gives_one_root_form_with_exactly_three_children"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/controltree.rs#tests::grayscale_gives_the_form_its_own_nine_direct_children_by_name"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/controltree.rs#tests::grayscale_gives_at_least_one_frame_with_children_and_the_tiling_holds"
        status: pass
    human_judgment: false
  - id: D4
    description: "A scope run of 65 non-terminating bytes and a control block declaring a Length of 0 both refuse, each naming the byte offset"
    requirement: "FRM-01"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::a_scope_run_of_sixty_five_non_terminating_bytes_refuses_and_names_the_offset"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controltree.rs#tests::a_control_block_with_a_length_of_zero_refuses_and_names_the_offset"
        status: pass
    human_judgment: false
  - id: D5
    description: "STRUCTURES.md gap 11 is closed with the offset, the evidence and what the measurement did not settle; GAPS.md's third party OCX and control array counts are corrected, with the shell glob cause named"
    requirement: "FRM-02"
    verification:
      - kind: other
        ref: "grep -q '0x05' .planning/research/STRUCTURES.md && grep -qi 'gap 11' .planning/research/STRUCTURES.md"
        status: pass
      - kind: other
        ref: "test 0 -eq \"$(grep -c '35 files' .planning/phases/02-the-object-graph/GAPS.md)\""
        status: pass
      - kind: other
        ref: "test 3 -eq \"$(find corpus -iname '*.frm' -print0 | xargs -0 grep -l 'MSWinsockLib.Winsock' | wc -l | tr -d ' ')\""
        status: pass
    human_judgment: false

duration: 53min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 4: The Control Tree and the Scope-Separator Walk Summary

**The control block header reader for both layouts, a scope-separator walk re-derived from Grayscale.exe's own bytes (not the research document's approximate prose) and gated on Tiling, and the STRUCTURES.md gap 11 closure with GAPS.md's two corrected corpus counts.**

## Performance

- **Duration:** 53 min (approximate; measured from the previous plan's completion commit to this plan's final commit)
- **Started:** 2026-09-10T12:23:13+02:00 (approximate)
- **Completed:** 2026-09-10T13:16:17+02:00
- **Tasks:** 3
- **Files modified:** 5 (0 created, 5 modified)

## Accomplishments

- `read_control_header` reads both the non-array and array control block layouts `STRUCTURES.md` section 8.4 gives. The array `Index` is the two byte little-endian value at control block offset `0x05`, read defensively with a `Defect` when the high byte is non-zero (research assumption A4). A declared name length of `0`, or one larger than the remaining block, gives an empty name and a `Defect` rather than an allocation sized from the file. `classify_control_type` carries an unrecognised `cType` raw, following `vb/classify.rs`'s own "carry raw, never guess" shape.
- `walk` reads the whole control tree: the form's own outermost block, then every child and sibling the scope-separator bytes describe, gated end to end on `gui::Tiling`. `LockWorkStation.exe` (0 children), `SK-Gradient-Sample__VB6/demo/Project1.exe` (3 flat children) and `Grayscale-effect/Grayscale.exe` (9 direct children, including nested `Frame` containers with `OptionButton` arrays and a menu with its own submenu) all tile `lPropertiesLength` exactly.
- The scope byte grammar — `STRUCTURES.md`'s own words, "the least certain part of the entire format" — was re-measured directly against `Grayscale.exe`'s real bytes rather than implemented from the research document's approximate `Length+2`/`Length-2` prose, which this session found does not reconcile byte-for-byte against four independent real transitions. The actual rule: a control block's own separator starts at `blockStart + Length - 1`; `0x02` pops one level and the run continues; `0x03` is the sibling terminal and adds no pop of its own; `0x01` makes the block just read the parent of the next one.
- A menu control (`cType` 19) reads a bare `0x02` as its own child-opening terminal, a corpus-measured special case (`Grayscale.exe`'s `mnuFile` and `mnuOpenImage`) distinct from every other control type, where `0x02` is a pop.
- `STRUCTURES.md` gap 11 is closed: the offset, 30 array elements across 2 files and 2 control types, and what the measurement did not settle (the one byte against two byte question). `GAPS.md`'s third party OCX count (0 → 3, `MSWinsockLib.Winsock`) and control array count (35 files → 48 elements across 6 files) are corrected, with the shell glob that caused both named directly, in a commit that changes no Rust file.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: The control block header, the type, the name and the array Index** - `a89ec52` (feat, tdd="true")
2. **Task 2: The scope separator walk and the control tree, gated by the tiling check** - `bf97be9` (feat, tdd="true")
3. **Task 3: Record the reference document corrections** - `ef7e55f` (docs, no Rust file)

**Plan metadata:** commit follows this SUMMARY.

_Note: Task 1 and Task 2 both edit `controltree.rs`, but ship as two separate commits per the plan's own task boundary — Task 1 is the header reader alone (11 tests, all synthetic byte fixtures), Task 2 adds the scope-separator walk and the corpus-wide tests (9 more tests, 20 total). The module doc comment describing the walk, and the corpus test proving the full 25-element `TxtF` array, moved into Task 2's commit, because they depend on `walk()`, which does not exist until that commit — even though the plan's own Task 1 acceptance criteria names the 25-element test. Both tasks carry `tdd="true"`; per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching 03-01's and 03-03's own precedent), each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task."_

## Files Created/Modified

- `crates/deform6/src/vb/controltree.rs` — `ControlHeader`, `read_control_header`, `read_array_index`, `ControlKind`, `classify_control_type` (Task 1); `ScopeRun`, `read_scope_run`, `ControlNode`, `ControlTree`, `walk`, `read_block`, `apply_pops`, `block_fits`, `close_walk` (Task 2); 20 unit and integration tests
- `crates/deform6/src/vb/gui.rs` — adds `FormStream::region()` (`pub(crate)`), the one accessor `walk` needs that plan 03-01 did not expose
- `crates/deform6/src/error.rs` — adds `DefectKind::EmptyName` and `DefectKind::IndexHighByteSet`, two narrowly scoped variants neither existing variant fits
- `.planning/research/STRUCTURES.md` — gap 11 closed in the section 11 register; new section 14, "Gap 11 closed," with the offset, the evidence and the open one-byte-versus-two-byte question; a pointer added below section 8.4's own array header table
- `.planning/phases/02-the-object-graph/GAPS.md` — section 5's third party OCX row (0 → 3), control array row (35 files → 48 elements across 6 files) and menus row (17 → 22 files, same broken glob) corrected, with the shell glob cause named; the "Summary ranking" section's own stale "no real third-party-OCX corpus material" claim corrected to match

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The most consequential: the scope byte grammar was re-measured directly against `Grayscale.exe`'s own bytes rather than trusted from `03-RESEARCH.md`'s own prose, because that prose's `Length+2`/`Length-2` formulas do not reconcile byte-for-byte against four independently verified real transitions in this corpus. The `Length - 1` rule this plan implements is confirmed identically across all four.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `error.rs` needed two new `DefectKind` variants**
- **Found during:** Task 1, writing `read_control_header`'s name-length-zero and array-index-high-byte cases
- **Issue:** The plan's own acceptance criteria require a `Defect` for a declared name length of zero and for a non-zero array index high byte. No existing `DefectKind` variant fits either case (checked the whole file per the pattern map's own instruction).
- **Fix:** Added `DefectKind::EmptyName { offset }` and `DefectKind::IndexHighByteSet { offset, high }`, following the file's own exhaustive per-variant severity match (both `Recoverable`, since the control keeps every other field).
- **Files modified:** `crates/deform6/src/error.rs`
- **Verification:** `a_declared_name_length_of_zero_gives_an_empty_name_and_a_defect_and_still_gives_c_type`, `a_two_byte_array_index_with_a_non_zero_high_byte_gives_a_defect`
- **Committed in:** `a89ec52` (Task 1 commit)

**2. [Rule 3 - Blocking] `gui.rs`'s `FormStream` exposed no accessor to its own bytes**
- **Found during:** Task 2, writing `walk`, which must read past the form's own outermost block
- **Issue:** `FormStream`'s `region` field is private, and its only public accessors (`length`, `name`, `control_type`) read fixed offsets within the form's own block alone. `controltree::walk` needs the whole bounded region to read children and the scope-separator runs between them.
- **Fix:** Added `pub(crate) const fn region(&self) -> &Region<'_>` to `FormStream`, a minimal, additive accessor. No existing behaviour changed.
- **Files modified:** `crates/deform6/src/vb/gui.rs`
- **Verification:** every corpus-wide `controltree` test exercises it, since `walk` calls it first
- **Committed in:** `bf97be9` (Task 2 commit)

**3. [Rule 1 - Bug] The plan's own cited corpus path does not exist**
- **Found during:** Task 1, writing the `TxtF` fixture and the corpus test
- **Issue:** The plan's `<read_first>` and fixture text both name `corpus/vb6-code/Custom-image-filters/CustomFilters.exe`. That file is not in the corpus; the directory holds `Custom_Filters.exe` (with an underscore), and its own `.vbp` confirms `ExeName32="Custom_Filters.exe"`.
- **Fix:** Every reference — the Rust `include_bytes!` path, the fixture's own doc comment, and this plan's own `STRUCTURES.md` closure section — uses the real, corpus-verified path, with the discrepancy noted so a future reader is not confused by the mismatch against the plan text.
- **Files modified:** `crates/deform6/src/vb/controltree.rs`, `.planning/research/STRUCTURES.md`
- **Verification:** the corpus test compiles and passes against the real file
- **Committed in:** `a89ec52` (Task 1), `ef7e55f` (Task 3)

**4. [Rule 1 - Bug] The plan's own scope-byte grammar text does not reconcile against real corpus bytes**
- **Found during:** Task 2, implementing the scope-separator walk against `SK-Gradient-Sample__VB6` and `Grayscale.exe`
- **Issue:** `03-RESEARCH.md` Pattern 2's illustrative code, and the plan's own `<behavior>` summary ("until one is greater than 3 or equal to 0"), both imply `0x01` and `0x03` behave symmetrically as immediate terminals with no accumulated pops. Measured directly against `Grayscale.exe`, `0x03` genuinely is always an immediate terminal with the pops seen so far, but `0x02` always continues the run rather than being read as a second, independent terminal — and a menu control's own bare `0x02` (`mnuFile` to `mnuOpenImage`) is a third, distinct case: an immediate `OpenChild`-equivalent terminal, not a pop at all.
- **Fix:** Implemented the three-way rule this session's own byte-level measurement gives (see the module doc comment and key-decisions), not the research document's approximate prose. Verified against four independent real transitions plus the menu case, and against all three required corpus files' own `Tiling::finish` success.
- **Files modified:** `crates/deform6/src/vb/controltree.rs`
- **Verification:** `sk_gradient_sample_gives_one_root_form_with_exactly_three_children`, `grayscale_gives_the_form_its_own_nine_direct_children_by_name`, `lock_work_station_gives_one_root_form_with_zero_children_and_tiling_holds`
- **Committed in:** `bf97be9` (Task 2 commit)

**5. [Rule 1 - Bug] `GAPS.md`'s Menus row was not one of the plan's named "two wrong rows," but is wrong for the identical reason**
- **Found during:** Task 3, correcting the control array and third-party-OCX rows
- **Issue:** `03-CONTEXT.md` D-03 names only two `GAPS.md` corrections. The Menus row (17 files) is sourced from the exact same broken `corpus/**/*.frm` glob, and `03-RESEARCH.md`'s own "State of the Art" table already gives the corrected figure (22 files, 76 entries, matching `CORPUS.md` exactly). The plan's own Task 3 text separately instructs correcting a coverage sentence that cites "17 and 35 files" together.
- **Fix:** Corrected the Menus row's own number in the table (17 → 22 files, 76 entries) alongside the sentence beneath it, so the same document does not state two different counts for the same fact.
- **Files modified:** `.planning/phases/02-the-object-graph/GAPS.md`
- **Verification:** `find corpus -iname "*.frm" -print0 | xargs -0 grep -h "Begin VB.Menu" | wc -l` gives 76; the file count gives 22
- **Committed in:** `ef7e55f` (Task 3 commit)

---

**Total deviations:** 5 auto-fixed (2 blocking, 3 bugs).
**Impact on plan:** All five were necessary for correctness or for the plan's own stated behaviour to compile and hold against real corpus bytes. No scope creep: `error.rs` and `gui.rs` each gained one narrowly scoped, documented addition; the two path/count corrections replace a wrong citation with a measured one.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's array `Index` read** — `INDEX_AT` changed from `0x05` to `0x03`. `cargo test -p deform6 --lib vb::controltree::tests::the_txt_f_array_gives_twenty_five_elements_with_index_zero_through_twenty_four` run once:

```
assertion `left == right` failed: TxtF indices found: [640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640]
  left: [640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640, 640]
 right: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24]
```

Every one of the 25 elements gave the same wrong value, 640 (`0x0280`, the flags byte `0x80` and the group constant `0x02` read together as one little-endian `u16` at the wrong offset). Reverted `INDEX_AT` to `0x05` before committing `a89ec52`.

**Task 2's scope byte pop count** — the plan's own suggested target, `corpus/public-domain/SK-Gradient-Sample__VB6`, does not exercise a pop at all: its three children (`Command1`, `Picture1`, `Label1`) are flat siblings of the form with no intervening container, confirmed both by direct byte inspection and by re-running its own child-count test with the pop handling broken (it still passed, 0 pops needed either way). `Grayscale.exe` is the file that genuinely exercises one, between `frameShades`'s last child `lblShades` and its sibling `frameDecompose`. The `0x02` handler was changed to close zero levels instead of one (`0x02 => {}` instead of incrementing `pops`). `cargo test -p deform6 --lib vb::controltree::tests::grayscale_gives_the_form_its_own_nine_direct_children_by_name` run once:

```
assertion `left == right` failed
  left: ["frameShades"]
 right: ["frameShades", "frameDecompose", "frameChannel", "lstFilters", "cmdReset", "picMain", "picBack", "Label2", "mnuFile"]
```

The form's own direct child count collapsed from 9 to 1: every control after `frameShades` was nested one level too deep instead of returning to the form's own child level. Reverted the `0x02` handler before committing `bf97be9`. This plan's own SUMMARY documents why `SK-Gradient-Sample__VB6` was not the instrument the plan's acceptance criteria expected it to be, and substitutes the file that genuinely proves the fix.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every symbol this plan's own artifact table names (`ControlTree`, `ControlNode`, `ScopeRun`, `ControlHeader`, `walk`, `read_control_header`, `read_array_index`, `MAX_SCOPE_RUN`, `ARRAY_FLAG`) is implemented and tested against real corpus bytes, not a placeholder.

## Threat Flags

None. Every trust boundary this plan's own `<threat_model>` names (the scope byte run, a control block `Length` of 0, cursor arithmetic, a declared name length, a mis-nested tree, an unassigned `cType`, the array index high byte) is mitigated exactly as the threat register states, and this plan introduces no new boundary beyond those.

## Next Phase Readiness

- `ControlTree`, `ControlNode` and `ControlHeader` are ready for plan 03-06 (`propstream.rs`) to build the real property-value walk against, using `ControlHeader::header_len()` to locate where each control's own property stream begins.
- `classify_control_type` and `ControlKind` are ready for plan 03-10's report to name a control's type without a separate lookup.
- `STRUCTURES.md` gap 11 is closed and does not block any later plan. The one-byte-versus-two-byte question for the array `Index` field stays explicitly open, for a future corpus sample with an index above 255.
- No blockers for plan 03-05.

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

All 5 modified files found on disk. All 3 task commits (`a89ec52`, `bf97be9`,
`ef7e55f`) found in `git log`. `cargo test --workspace` passes (253 lib
tests plus every integration and CLI test), `cargo fmt --all --check` and
`cargo clippy --all-targets -- -D warnings` both pass clean, and all three
plan-level shell verifications (3 `MSWinsockLib.Winsock` files, 6 control
array files, 48 control array elements) pass.
