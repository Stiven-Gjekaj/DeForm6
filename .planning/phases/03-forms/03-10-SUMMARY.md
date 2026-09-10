---
phase: 03-forms
plan: 10
subsystem: forms
tags: [vb6, differential-gate, inspect, cli, recovery-ratio, forms, controls]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-01's GuiTable/GuiObjectInfo/Tiling, plan 03-03's support::frm second reader, plan 03-04's ControlTree/ControlNode/classify_control_type, plan 03-05's VbStr, plan 03-06's walk_properties/PropertyValue, plan 03-07's frx blob primitives, plan 03-08's ocx ExternalControl/Clsid/OcxHeader, plan 03-09's controlinfo ControlInfoTable/EventTable/EventNameTable/report_events -- every one of these is composed here, none re-implemented"
provides:
  - "Report.forms: Vec<FormReport>, the whole phase 3 path composed into deform6::inspect, per form: the control tree, the property values, the external control facts, and the event slots"
  - "inspect prints the form tree in the CLI, one section per form, one line per control, indented to show parent, with a shared 'not decoded' wording for every honest gap"
  - "The differential gate's forms section and controls section, against support::frm, in both directions"
  - "tests/ratios.toml's four new keys per program: form_declared, form_recovered, control_declared, control_recovered, pinned at 49 of 53 forms and 607 of 607 controls"
affects: [04-writer, 05-safety-fuzzing, 06-documentation]

actuals:
  tokens: 28165
  tasks: 3
  commits: 3
plan_head_before: c10ffa4ede745c14f6a4c0de1fe7ee713e145bf2
commits: 3

tech-stack:
  added: []
  patterns:
    - "compose_form/compose_control in vb/mod.rs: every fallible step past the GUI table entry itself (GuiObjectInfo, the property stream, the control tree, ControlInfoTable, one control's own event table) is caught and turned into a per-form or per-control Defect, never a whole-file Refusal, matching compose_object's own established shape for the object graph."
    - "ControlNode<'a> carries its own bounded byte block (Region<'a>), added specifically so a composer reached after the walk finishes (this plan) can read a control's own property stream and, for an external control, its class name and OCX header, without re-walking the tree. FormStream::region() now gives Region<'a> by value (Region is Copy) instead of &Region<'_>, so the block's own lifetime is tied to the underlying file bytes, not to the borrow of the FormStream that built it."
    - "A single new DefectKind, StructureUnreadable{offset, reason}, is the one conversion point from any per-form or per-control Refusal into a Defect, reused at every one of compose_form's and compose_control's own fallible steps, rather than one bespoke variant per call site."
    - "The differential gate's controls comparison excludes a form's own controls entirely when that form's own tree failed to build (FormReport.controls is empty): that shortfall is already the reason form_recovered is short one, and comparing an empty list against the full declared list would double-count the same gap as a flood of missing-control failures."
  patterns-note: "See key-decisions for the close_walk fix and the MAX_UNEXPLAINED_TAIL bound."

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/mod.rs
    - crates/deform6/src/vb/gui.rs
    - crates/deform6/src/vb/controltree.rs
    - crates/deform6/src/vb/propstream.rs
    - crates/deform6/src/vb/controlinfo.rs
    - crates/deform6/src/error.rs
    - crates/deform6-cli/src/main.rs
    - crates/deform6-cli/tests/cli.rs
    - crates/deform6/tests/corpus_sweep.rs
    - crates/deform6/tests/refusal.rs
    - crates/deform6/tests/differential.rs
    - crates/deform6/tests/ratios.rs
    - crates/xtask/src/main.rs
    - tests/ratios.toml

key-decisions:
  - "AGENTS.md's 'put the code and its tests in the same commit' rule takes precedence over the plan's implicit TDD RED-then-GREEN commit split, for all three tasks (tdd=\"true\"), matching every prior plan in this phase. Each task's own RED phase was run once against a deliberately broken implementation, its failure evidence captured (see Deviations and Break-on-purpose evidence below), then reverted to the correct GREEN implementation before committing code and tests together in one commit per task."
  - "inspect's own signature changed from fn(&[u8]) -> Result<Report, Refusal> to fn(&[u8], &OpcodeTable) -> Result<Report, Refusal>, per the plan's own text ('the opcode table arrives as a parameter, already parsed by the command line crate'). This is a breaking change to the library's one public entry point, propagated to every existing call site: crates/deform6-cli/src/main.rs (now passes the table the CLI already loads), crates/deform6/tests/refusal.rs and crates/deform6/tests/corpus_sweep.rs (neither in this plan's own files_modified list, both required for cargo test --workspace to compile -- Rule 3, blocking). The event name table is not threaded through the same way: 03-CONTEXT.md D-02 gives EventNameTable no run-time loader in this phase, so inspect builds EventNameTable::default() internally."
  - "ControlNode<'a> gained a pub(crate) block: Region<'a> field, and FormStream::region() changed from returning &Region<'_> to returning Region<'a> by value (Region is Copy). Neither controltree.rs nor gui.rs is in this plan's own files_modified list; both changes were necessary (Rule 3, blocking) because composing a control's own property stream and external-control facts after the tree walk finishes needs the control's own byte span, which the walk already computes internally (in read_block) but previously discarded. This mirrors 03-01's own precedent of adding a narrow pub(crate) accessor (FormStream::region() itself) specifically to unblock a later plan's composition needs."
  - "A real, corpus-discovered bug, not carried in by this plan: the scope-byte walk's own close_walk function silently swallowed real control data (a sibling menu and its own three children) as an 'unexplained trailing span,' the same mechanism 03-01 built for LockWorkStation's own genuinely unexplained 3-byte footer. The differential gate (comparing against support::frm, a second independent reader) found this on real data across three corpus programs (HexScroll, PassGen, UUID2), all of which declare two sibling top-level menus where the first has a nested submenu. The fix is deliberately narrow and defensive, not a new grammar rule: close_walk now refuses when the trailing span exceeds a small, named margin (MAX_UNEXPLAINED_TAIL = 8 bytes, more than double the 3-byte margin 03-RESEARCH.md assumption A3 measured), converting the silent wrong tree into the same honest per-form refusal every other unreadable structure already gets. The true grammar rule for closing out of a two-level-deep menu structure is not found; it needs the same byte-level, multi-sample corpus research 03-04 did for the single-level case, and is recorded as WINDOWS.md finding 7 rather than guessed at here."
  - "The differential gate's controls comparison and property-value comparison both exclude a form entirely once that form's own control tree failed to build (FormReport.controls is empty): its own missing controls are already the reason form_recovered is short one for that program, and asserting the full declared control list against an empty recovered list would report the same shortfall a second time, as a flood of individually-named missing controls, rather than as the one honest form-level gap it actually is."
  - "The property-value comparison strips a trailing VB6 inline comment ('WindowState = 1  'Minimized') from the declared side before comparing a non-Text property, because support::frm::split_property_line (03-03's own file, not touched here) keeps the comment as part of the raw value for an unquoted line. Never applied to a Text property's own declared value, which is already quote-stripped and where a genuine apostrophe is real text, not a comment marker."
  - "EXPECTED_FORM_RECOVERY_TOTAL is 53, not support/frm.rs's own whole-corpus, filesystem-walk total of 54: the gap is the same orphaned project support/vbp.rs's own doc comment already names (Brightness-effect/Part 3 - DIBs/Brightness3.vbp declares dibBrightness.exe, which this repository does not vendor). This session measured, directly: the 44 .vbp files that do have a matching executable declare 54 Form= lines in total (matching the 54 files on disk), but the differential gate walks executables, one .vbp per executable via vbp::select_project_file, so the one form the orphaned project alone declares is never reached: there is no compiled binary for deform6::inspect to recover it from."
  - "xtask/src/main.rs's own three PinnedEntry test literals needed the four new fields added (Rule 3, blocking compile error, not in this plan's own files_modified list). xtask's own update-ratios command was deliberately left unwired to the four new keys: format_entry (which xtask's writer and this plan's own MOVED_UP message both still use, unchanged) only ever wrote the procedure ratio's own three fields. Wiring the new fields into that shared writer would have required extending format_entry's own signature and, in turn, xtask's own hardcoded-string tests, well beyond this plan's own scope. tests/ratios.toml's own header and WINDOWS.md finding 8 both record, honestly, that a future run of cargo run -p xtask -- update-ratios would drop the four new keys until that wiring is done."
  - "FormReport.controls[0] is always the form's own outermost control block (the form itself), not only its child controls: this is what lets the form's own properties (WindowState, Caption, and so on) and the form's own event slots (joined against the ControlInfo entry literally named 'Form', per 03-09's own measurement) reach the differential gate and the CLI printer through the exact same ControlReport shape every other control uses, rather than a second, parallel path."

patterns-established:
  - "structure_defect(offset, structure, &refusal) -> Defect: the one, reused conversion from a Refusal into a Defect for every composed-walk failure this plan introduces, via the single new DefectKind::StructureUnreadable variant."

requirements-completed: [FRM-01, FRM-02, FRM-03, FRM-04, FRM-05, FRM-06, VER-06]

coverage:
  - id: D1
    description: "Report gains a forms field; inspect composes the GUI table walk, the control tree, the property stream, the external control facts and the event slots into one FormReport per form; a form whose own structure refuses gives an empty tree and a per-form Defect, and the rest of the report still builds"
    requirement: "FRM-01"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/mod.rs#tests::a_program_with_zero_forms_gives_an_empty_forms_list_and_no_fault"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/mod.rs#tests::two_runs_over_the_same_bytes_give_equal_reports"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/mod.rs#tests::a_tiling_failure_in_one_form_costs_only_that_form"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/corpus_sweep.rs#every_form_across_the_corpus_gives_a_tree_or_a_named_defect"
        status: pass
    human_judgment: false
  - id: D2
    description: "inspect prints the control tree of every corpus form, one section per form, one line per control indented to show its parent, with a control's type, array index, properties, external-control facts and event slots; the eight line header and every existing exit code are unchanged"
    requirement: "FRM-01"
    verification:
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#gradient_sample_names_the_form_and_its_three_controls"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#a_control_inside_a_container_indents_deeper_than_its_parent"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#inspecting_the_corpus_file_prints_the_locked_eight_line_head_unchanged"
        status: pass
    human_judgment: false
  - id: D3
    description: "A property this repository can name prints its name and its value; a property, an external control's own property blob, and an event slot it cannot name share one 'not decoded' wording, naming the byte offset or the slot index and the command line flag that would supply a table"
    requirement: "FRM-03"
    verification:
      - kind: other
        ref: "test 3 -le \"$(grep -c 'not decoded' crates/deform6-cli/src/main.rs)\""
        status: pass
    human_judgment: false
  - id: D4
    description: "An external control prints its class name, its CLSID or the stated reason for having none, its extents, and the opaque blob statement naming the type library this repository does not hold"
    requirement: "FRM-04"
    verification:
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#winsock_sample_prints_the_clsid_and_the_opaque_blob_statement"
        status: pass
    human_judgment: false
  - id: D5
    description: "The differential gate compares forms by name and controls by form/name/array-index against support::frm, in both directions; the recovered control type maps to the declared class with the VB. prefix removed, and an external control's class name is compared whole; every recovered, named, plain-line property agrees with the declared text"
    requirement: "VER-06"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/differential.rs#every_declared_form_and_control_is_recovered_and_every_recovered_one_is_declared_across_the_corpus"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/differential.rs#a_fabricated_form_fails_the_two_directional_check_and_names_it"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/differential.rs#removing_a_recovered_form_fails_the_two_directional_check_in_the_other_direction"
        status: pass
    human_judgment: false
  - id: D6
    description: "tests/ratios.toml gains form_declared, form_recovered, control_declared and control_recovered per program; tests/ratios.rs checks them with the same REGRESSION/MOVED UP vocabulary the procedure ratio uses, and the committed pin passes"
    requirement: "VER-06"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/ratios.rs#the_forms_and_controls_gate_passes_on_the_committed_file"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/ratios.rs#raising_a_pinned_form_recovered_count_fails_with_regression"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/ratios.rs#lowering_a_pinned_control_declared_count_fails_with_moved_up"
        status: pass
    human_judgment: false
  - id: D7
    description: "The frmHMM.frx exclusion still names the one upstream defect, and frmHMM.frm itself is never excluded"
    requirement: "VER-06"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/differential.rs#the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect"
        status: pass
    human_judgment: false
  - id: D8
    description: "The opcode table beyond the safe-provenance subset (about 158 of 198 pairs) and closing one remaining (control type, property) pair both need a human at a working VB6 install; neither is a blocker for this phase"
    verification: []
    human_judgment: true
    rationale: "Requires a lawful Visual Basic 6 install on a Windows host, which this session's environment does not have. 03-VALIDATION.md and this plan's own <human-check> block name both items as follow-up work outside phase 3."

duration: single session
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 10: Wire the Whole Phase Into Inspect Summary

**`Report` gains `forms`, `inspect` composes every phase 3 reader into one `FormReport` per form, the CLI prints the whole control tree, and the differential gate measures forms and controls against `support::frm` at 49 of 53 forms and 607 of 607 controls recovered, over the forms whose own tree the tool built.**

## Performance

- **Duration:** single session
- **Started:** 2026-09-10T15:39:31+02:00 (previous plan's completion commit)
- **Tasks:** 3
- **Files modified:** 14 (0 created, 14 modified)

## Accomplishments

- `Report` gains a `forms: Vec<FormReport>` field. `inspect`'s own signature changed to `fn(data: &[u8], opcode_table: &OpcodeTable) -> Result<Report, Refusal>`, so the command line's already-loaded `--opcode-table` (or the builtin safe-provenance subset) now actually reaches the property walk. `compose_form` and `compose_control` (new, in `vb/mod.rs`) walk the GUI table, each form's `GuiObjectInfo`, the control tree, the property stream, the external control facts (class name, CLSID join, fixed OCX header, opaque blob statement) and the event slots (joined by name against `ControlInfoTable`, per 03-09's own measured `"Form"` sentinel for the root). A form or a control whose own structure refuses gives a per-item `Defect` (the one new `DefectKind::StructureUnreadable` variant) and no tree for that one item; every other form and control in the same program still builds.
- `ControlNode` now carries its own bounded byte block (`pub(crate) block: Region<'a>`), added because composing a control's property stream and external-control facts after the walk finishes needs the exact bytes `controltree::read_block` already computed but previously discarded. `FormStream::region()` now gives `Region<'a>` by value instead of `&Region<'_>`, so that block's lifetime is tied to the file bytes, not to the borrow of the `FormStream` that produced it.
- The CLI prints one section per form and one line per control, indented by depth to show each control's own parent. A property, an external control's own property blob, and an event slot this repository cannot name all share one "not decoded" wording, naming the byte offset (or the slot index) and the flag (`--opcode-table` or `--event-name-table`) that would supply a table. The locked eight line header and every existing exit code are unchanged.
- The differential gate gains a forms section and a controls section in `crates/deform6/tests/differential.rs`, comparing against `support::frm` (never against `deform6`'s own output) in both directions: a declared form or control with no recovered match, and a recovered one with no declared match. The recovered control type maps to the declared class with the `VB.` prefix removed; an external control's class name is compared whole. Every recovered, named, plain-line property (`Byte`/`Boolean`/`Integer`/`Long`/`Single`/`Text`) agrees with the declared text, once a trailing VB6 inline comment is stripped from the declared side.
- `tests/ratios.toml` gains `form_declared`, `form_recovered`, `control_declared` and `control_recovered` for each of the 44 programs, pinned at 49 of 53 forms and 607 of 607 controls (over the forms whose own tree the tool built). `crates/deform6/tests/ratios.rs` checks the new counts with the same `REGRESSION`/`MOVED UP` vocabulary and failure-message shape the procedure ratio already established.
- The differential gate, comparing against real corpus data through `support::frm`, found a genuine bug in 03-04's own scope-byte walk: closing out of a menu control nested two levels deep, back to a sibling menu at the form's own top level (`HexScroll.exe`, `PassGen.exe`, `UUID2.exe`, each declaring two sibling top-level menus, the first with a nested submenu), left the walk silently swallowing the sibling menu and its own children as an "unexplained trailing span" -- the exact mechanism 03-01 built for a genuinely unexplained 3-byte footer. Fixed narrowly: `close_walk` now refuses when the trailing span exceeds `MAX_UNEXPLAINED_TAIL` (8 bytes), turning a silent wrong tree into the same honest per-form refusal every other unreadable structure already gets. The true grammar rule for this transition is not found; it is recorded as an open, named gap (`WINDOWS.md` finding 7), not guessed at.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: Report gains forms and controls, and inspect walks the whole path** - `668ceb9` (feat, tdd="true")
2. **Task 2: inspect prints the form tree** - `21d8281` (feat, tdd="true")
3. **Task 3: The differential gate and the two pinned ratios** - `d75e5f8` (feat, tdd="true")

**Plan metadata:** commit follows this SUMMARY.

_Note: all three tasks carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase), each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task._

## Files Created/Modified

- `crates/deform6/src/vb/mod.rs` - `FormReport`, `ControlReport`, `PropertyReport` (alias), `EventReport` (re-export), `compose_form`, `compose_control`, `structure_defect`; `inspect`'s new signature; new tests (Task 1)
- `crates/deform6/src/vb/gui.rs` - `FormStream::region()` now returns `Region<'a>` by value (Task 1, Rule 3)
- `crates/deform6/src/vb/controltree.rs` - `ControlNode<'a>`/`ControlTree<'a>` carry a lifetime and `ControlNode.block: Region<'a>`; `read_block` returns the block too; `close_walk`'s new `MAX_UNEXPLAINED_TAIL` refusal (Task 1 for the lifetime threading, Task 3 for the bug fix)
- `crates/deform6/src/vb/propstream.rs` - one test updated for `FormStream::region()`'s new by-value signature (Task 1)
- `crates/deform6/src/vb/controlinfo.rs` - `FORM_SELF_ENTRY_NAME` widened to `pub(crate)` (Task 1, Rule 3); test lifetime annotation fix
- `crates/deform6/src/error.rs` - `DefectKind::StructureUnreadable` (Task 1)
- `crates/deform6-cli/src/main.rs` - the forms printer: `print_forms`, `print_form`, `control_depth`, `print_control`, `format_control_kind`, `print_property`, `print_external`, `print_clsid`, `print_event` (Task 2)
- `crates/deform6-cli/tests/cli.rs` - three new tests over real corpus programs (Task 2)
- `crates/deform6/tests/corpus_sweep.rs` - `inspect` call site updated for the new signature; `every_form_across_the_corpus_gives_a_tree_or_a_named_defect` (Task 1, Rule 3)
- `crates/deform6/tests/refusal.rs` - nine `inspect` call sites updated for the new signature (Task 1, Rule 3)
- `crates/deform6/tests/differential.rs` - the forms/controls comparison, `FormsControlsCounts`, `forms_controls_counts`, `form_control_mismatches`, and seven new tests (Task 3)
- `crates/deform6/tests/ratios.rs` - `PinnedEntry` gains four fields; `parse_ratios_toml` parses them; `check_program_forms_controls`; four new tests (Task 3)
- `crates/xtask/src/main.rs` - three `PinnedEntry` test literals updated for the four new fields (Task 3, Rule 3)
- `tests/ratios.toml` - four new keys per program, and an extended header paragraph (Task 3)

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The four most consequential: `inspect`'s own public signature changed to take an opcode table, propagating to every existing caller; `ControlNode` now carries its own byte block with an explicit lifetime, requiring `FormStream::region()` to change from a reference to a by-value return; a real scope-grammar bug the differential gate found (closing out of a two-level-deep menu) is fixed narrowly and defensively rather than by guessing the true grammar rule; and the controls comparison excludes a form entirely once its own tree failed to build, so one form-level gap is never double-counted as a flood of missing-control failures.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `inspect`'s new signature broke every existing call site**
- **Found during:** Task 1, changing `inspect` to take an opcode table
- **Issue:** `crates/deform6-cli/src/main.rs`, `crates/deform6/tests/refusal.rs` and `crates/deform6/tests/corpus_sweep.rs` all called `inspect(data)` with one argument. Only `main.rs` is in this plan's own `files_modified` list.
- **Fix:** Updated all nine call sites in `refusal.rs`, the one in `corpus_sweep.rs`, and wired the CLI's own already-loaded table into `main.rs`'s call.
- **Files modified:** `crates/deform6-cli/src/main.rs`, `crates/deform6/tests/refusal.rs`, `crates/deform6/tests/corpus_sweep.rs`
- **Verification:** `cargo test --workspace`
- **Committed in:** `668ceb9` (Task 1 commit)

**2. [Rule 3 - Blocking] Composing a control's own property stream needed its own byte span, which `ControlTree` did not expose**
- **Found during:** Task 1, writing `compose_control`
- **Issue:** `controltree::read_block` already computes each control's own bounded window internally but discarded it; without it, `vb/mod.rs` would have had to re-walk the scope-byte grammar itself, which the plan's own action text forbids ("This task composes them; it re-implements none of them").
- **Fix:** Added `pub(crate) block: Region<'a>` to `ControlNode`, threaded a lifetime through `ControlTree`/`ControlNode`, and changed `FormStream::region()` to return `Region<'a>` by value (`Region` is `Copy`) so the block's lifetime ties to the file bytes, not to the borrow of `FormStream`.
- **Files modified:** `crates/deform6/src/vb/controltree.rs`, `crates/deform6/src/vb/gui.rs`, `crates/deform6/src/vb/propstream.rs` (one test call site)
- **Verification:** `cargo test --workspace`
- **Committed in:** `668ceb9` (Task 1 commit)

**3. [Rule 3 - Blocking] The root `ControlInfo` sentinel name was private**
- **Found during:** Task 1, joining the form's own root control to its event slots
- **Issue:** `controlinfo::FORM_SELF_ENTRY_NAME` (the literal `"Form"` sentinel 03-09 measured) was a private `const`, unreachable from `vb/mod.rs`.
- **Fix:** Widened visibility to `pub(crate)`.
- **Files modified:** `crates/deform6/src/vb/controlinfo.rs`
- **Verification:** `compose_form`'s root-node event join, exercised by the corpus sweep
- **Committed in:** `668ceb9` (Task 1 commit)

**4. [Rule 1 - Bug] `close_walk` silently swallowed real control data as an unexplained tail**
- **Found during:** Task 3, running the corpus-wide differential test against real data
- **Issue:** `HexScroll.exe`, `PassGen.exe` and `UUID2.exe` each declare a sibling top-level menu (`menuAbout`) after a first menu (`menuFile`) that has one nested child (`menuExit`). Closing out of `menuExit` back to the form's own top level needs the scope-byte walk to pop two levels; the real bytes measured (`FF 03 02`, at file offset `0x16fd` in `HexScroll.exe`) do not match either of 03-04's own established rules (`0x03` as an immediate terminal, `0x02` as a pop), and the walk's own `close_walk` fallback silently accounted the real `menuAbout` block and its own three children as an "unexplained trailing span," reporting a tree that looked complete but was missing four real controls.
- **Fix:** `close_walk` now refuses (`Refusal::Damaged`, naming the byte offset and the byte count) when the trailing span exceeds `MAX_UNEXPLAINED_TAIL` (8 bytes, more than double 03-RESEARCH.md assumption A3's own 3-byte measurement), so the affected form gives a `Defect` and no tree instead of a silently incomplete one. The true grammar rule for this transition is not found; `WINDOWS.md` finding 7 records it as open, tracked follow-up work.
- **Files modified:** `crates/deform6/src/vb/controltree.rs`
- **Verification:** `crates/deform6/tests/differential.rs#every_declared_form_and_control_is_recovered_and_every_recovered_one_is_declared_across_the_corpus`; `crates/deform6/src/vb/mod.rs`'s own tiling-failure test switched fixtures once this fix changed which corpus programs give a clean two-form pair
- **Committed in:** `d75e5f8` (Task 3 commit)

**5. [Rule 1 - Bug] A trailing VB6 inline comment made a real property value look like a mismatch**
- **Found during:** Task 3, comparing `LockWorkStation.exe`'s own `WindowState` and `BorderStyle` properties
- **Issue:** `LockWorkStation.frm` writes `WindowState = 1  'Minimized`; `support::frm::split_property_line` (03-03's own file) keeps the comment as part of the raw value for an unquoted line, so the naive declared-versus-recovered string comparison reported a mismatch (`"1  'Minimized"` versus `"1"`) that was not a real disagreement.
- **Fix:** Added `strip_frm_comment`, applied only to a non-`Text` property's own declared value (a `Text` property's value is already quote-stripped, and a genuine apostrophe inside a caption is real text, not a comment marker).
- **Files modified:** `crates/deform6/tests/differential.rs`
- **Verification:** `every_declared_form_and_control_is_recovered_and_every_recovered_one_is_declared_across_the_corpus`
- **Committed in:** `d75e5f8` (Task 3 commit)

**6. [Rule 3 - Blocking] `xtask`'s own test literals needed the four new `PinnedEntry` fields**
- **Found during:** Task 3, running `cargo clippy --all-targets -- -D warnings`
- **Issue:** `crates/xtask/src/main.rs`'s own three `PinnedEntry { .. }` test literals did not compile once `PinnedEntry` gained four new required fields.
- **Fix:** Added the four fields (`0` in each case; these tests do not exercise the new fields) to all three literals.
- **Files modified:** `crates/xtask/src/main.rs`
- **Verification:** `cargo clippy --all-targets -- -D warnings`
- **Committed in:** `d75e5f8` (Task 3 commit)

---

**Total deviations:** 6 auto-fixed (4 blocking, 2 bugs).
**Impact on plan:** All six were necessary for the code to compile, pass the gate, or be correct. The two bug fixes (the scope-grammar tail bound and the comment-stripping) were both found by this plan's own differential gate operating on real corpus data, which is exactly the class of finding the gate exists to surface. No scope creep: every fix is narrowly scoped and documented, and the one grammar question the tail-bound fix does not resolve is recorded as open, tracked follow-up work rather than guessed at.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's per-form refusal (the plan's own named acceptance criterion: "The executor changes the per form refusal to a whole report refusal, sees the multi form test fail, and records which forms were lost in the SUMMARY")** - `compose_form`'s own `controltree::walk` error arm changed from converting the `Refusal` into a per-form `Defect` to `panic!("RED-PHASE DELIBERATE BREAKAGE: {refusal}")`. `cargo test -p deform6 --lib vb::tests::a_tiling_failure_in_one_form_costs_only_that_form` run once:

```
thread 'vb::tests::a_tiling_failure_in_one_form_costs_only_that_form' panicked at crates/deform6/src/vb/mod.rs:612:25:
RED-PHASE DELIBERATE BREAKAGE: this Visual Basic 6 executable is damaged: a control block at file offset 0x2fd1 declares 73 bytes, which runs past the end of the file
```

`Advanced Histogram Viewer.exe`'s own two forms (`frmMain`, `frmHistogram`) are the pair this test corrupts one of; with the per-form handling reverted, both are named in `compose_form`'s own doc comment rather than in this panic message, since the panic itself aborts the whole test process before the second form is ever reached. Reverted to the `Defect`-converting `match` arm before committing `668ceb9`.

**Task 2's indentation (the plan's own named acceptance criterion: "The executor changes the indentation to a fixed zero... records the two counts from the failure message")** - `print_control`'s own indent computation changed from `"  ".repeat(depth.saturating_add(2))` to a fixed `"  ".repeat(2)`. `cargo test -p deform6-cli a_control_inside_a_container_indents_deeper_than_its_parent` run once:

```
thread 'a_control_inside_a_container_indents_deeper_than_its_parent' panicked at crates/deform6-cli/tests/cli.rs:667:5:
hscrShades (indent 4) must indent deeper than its own parent frameShades (indent 4): parent line "    frameShades  (Frame)", child line "    hscrShades  (HScrollBar)"
```

Both indents came back equal at 4 spaces. Reverted to the depth-based indent before committing `21d8281`.

**Task 3's three named breakages (the plan's own acceptance criteria: "Dropping one recovered form... Adding one form the source does not declare... Emptying support::frm::EXCLUSIONS"):**

Breakage 1 and 2 (one break, both tests) - `form_control_mismatches`'s own form-diff push condition changed from `if !form_diff.is_empty()` to `if false && !form_diff.is_empty()`. `cargo test -p deform6 --test differential -- fails_the_two_directional_check` run once:

```
thread 'a_fabricated_form_fails_the_two_directional_check_and_names_it' panicked at crates/deform6/tests/differential.rs:1206:5:
a form nothing declares must be named in the failure list: []

thread 'removing_a_recovered_form_fails_the_two_directional_check_in_the_other_direction' panicked at crates/deform6/tests/differential.rs:1238:5:
the missing form "frmFractal" must be named in the failure list: []
```

Both the fabricated form (`fabricatedFormNothingDeclares`) and the removed form (`frmFractal`, `Mandelbrot.exe`'s own one form) were silently dropped from the failure list once the form-diff push was disabled. Reverted before committing `d75e5f8`.

Breakage 3 - `the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect`'s own length check changed from `frm::EXCLUSIONS.len()` to a simulated empty slice's `len()`. `cargo test -p deform6 --test differential the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect` run once:

```
thread 'the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect' panicked at crates/deform6/tests/differential.rs:1254:5:
assertion `left == right` failed: support::frm::EXCLUSIONS holds 1 entries; this differential gate expects exactly the one named exclusion, corpus/vb6-code/Hidden-Markov-model/frmHMM.frx, or a future .frx comparison would need to treat that file's own corrupted bytes as real evidence
  left: 0
 right: 1
```

The message names `frmHMM.frx` even under the simulated-empty condition, satisfying ROADMAP success criterion 5's own wording. Reverted before committing `d75e5f8`.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

- **`crates/deform6/src/vb/propstream.rs`, `PayloadType::Picture` in `walk_properties`**: still reports `PropertyValue::Undecoded`, unchanged from 03-06/03-07's own documented gap. This plan's own "extract any blob" composition step (Task 1) is satisfied by the external control's own property blob (`ocx::read_ocx_blob`, corpus-exercised via `SK-Winsock-Sample__VB6`'s `wsPop`), which is the only blob-extraction path any row in the builtin safe-provenance opcode table can actually reach: no builtin row uses `PayloadType::Picture`, so wiring `frx::extract_blob` into the generic property walk would be unreachable and untestable with the table this phase ships. Deferred to whichever later plan first supplies a custom opcode table naming a `Picture` opcode, or to phase 4's `.frx` writer.
- **The scope-byte grammar for a two-level-deep menu close** (see key-decisions and Deviation 4): `MAX_UNEXPLAINED_TAIL` converts the failure from silent-and-wrong to honest-and-refused, but does not recover the four real controls (`menuAbout` and its own three children) on `HexScroll.exe`, `PassGen.exe` and `UUID2.exe`. Tracked as `WINDOWS.md` finding 7.
- **`xtask update-ratios` does not yet write the four new `tests/ratios.toml` keys**: a future run of that command on a clean tree would drop `form_declared`/`form_recovered`/`control_declared`/`control_recovered` from every entry. `tests/ratios.toml`'s own header states this, and it is tracked as `WINDOWS.md` finding 8.

## Threat Flags

None. Every trust boundary this plan's own `<threat_model>` names is mitigated exactly as the threat register states: T-03-52 by the declared side of every comparison coming from `support::frm`, which names nothing from `deform6`; T-03-53 by the property-value comparison never excluding an undecoded property from consideration, only from the value check itself, which is a structural impossibility (an `Undecoded` property carries no name to look up on the declared side, per `recovered_property_name`'s own exhaustive match) rather than a chosen exclusion; T-03-54 by both directions asserted for forms and for controls; T-03-55 by the per-form refusal discipline `compose_form` establishes; T-03-56 by `the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect` and the deliberate breakage proving it; T-03-41 by `inspect` still writing nothing to disk, proven by the existing `inspecting_the_corpus_file_changes_no_file_on_disk` CLI test, unchanged by this plan; T-03-09 unchanged, the whole crate still forbids unsafe code.

## Next Phase Readiness

- `Report.forms`, `FormReport`, `ControlReport` and the CLI's own printer are the phase 3 milestone's own recovered surface: phase 4's `.frf`/`.frx` writer reads exactly this shape.
- The differential gate and the pinned ratio are the honest measurement phase 3 promises: 49 of 53 forms and 607 of 607 controls recovered, with the remaining 4 forms' own gap named and tracked (`WINDOWS.md` finding 7), not hidden.
- Two follow-up items remain outside this phase, per `03-VALIDATION.md`'s own `<human-check>` block: the opcode table beyond the safe-provenance subset (about 158 of 198 pairs), and closing one remaining `(control type, property)` pair by the compile-and-diff method `STRUCTURES.md` section 13 already used. Neither blocks this phase.
- No blockers for phase 4.

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

All 14 modified files found on disk. All 3 task commits (`668ceb9`, `21d8281`, `d75e5f8`) found in `git log`. `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` (487 tests total across the workspace), `sh scripts/prove-lint-wall.sh` and `sh scripts/prove-region-wall.sh` all pass clean. `cargo test -p deform6 --test differential`, `--test ratios`, `--test corpus_sweep` and `--test form_tracer` all pass. The plan's own literal source assertions (`inspect` opens no file; the "not decoded" wording count; no em-dash; printable ASCII only; both `recovered_not_declared` occurrences; both `control_declared`/`form_declared` keys present) all pass.
