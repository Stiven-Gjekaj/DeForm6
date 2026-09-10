---
phase: 03-forms
plan: 09
subsystem: forms
tags: [vb6, control-info, event-handler-table, native-stub, event-name-table]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-04's ControlTree, ControlNode and the control name this plan joins against; plan 03-01's window-before-fields discipline and Object::lp_object_info, resolved independently the same way plan 01-07's vb/object.rs already resolves lpObjectArray; plan 03-02's OpcodeTable seam shape ('commit the tool, not the table'), mirrored here for the event name table"
provides:
  - "ControlInfo, ControlInfoTable, ControlInfoTable::read: the ControlInfo array read from OptionalObjectInfo at Object.lpObjectInfo plus 0x38, with fControlType and wEventCount read as two byte values at 0x00 and 0x02"
  - "join_by_name, ControlJoin: the control tree joined to ControlInfo by control name, both directions reported, gated on the measured fact that the form's own entry always carries the literal name \"Form\""
  - "EventTable, EventSlot, StubHandler, read_event_table: the event handler table, header size chosen from fControlType (0x18 for 0x40, 0x28 for 0x2E, no slots for any other value), the native stub decoded with checked signed arithmetic"
  - "EventNameTable, EventReport, report_events: the honest event-name report per 03-CONTEXT.md D-02, three states, zero event names compiled in"
affects: [03-10-differential-gate]

actuals:
  tokens: 16844
  tasks: 3
  commits: 3
plan_head_before: 44d51af

tech-stack:
  added: []
  patterns:
    - "bound_control_count and bound_event_count both mirror vb/object.rs::bound_proc_count exactly: a count field straight out of the file is clamped against the real length of the file that remains from its own array pointer, before it becomes a loop bound, and the clamp itself is the only Defect, never a second one per element the clamp already removed."
    - "decode_stub reads a native stub with checked arithmetic over the whole signed range (u32::checked_add_signed), so a negative relative jump gives a handler address below the stub start rather than wrapping around a u32."
    - "EventNameTable follows the same seam OpcodeTable (plan 03-02) established: a caller-supplied lookup table plugs into one lookup path, and the default gives every slot no name, so a supplied table can never behave differently from the empty one."

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/controlinfo.rs

key-decisions:
  - "AGENTS.md's 'put the code and its tests in the same commit' rule takes precedence over the plan's implicit TDD RED-then-GREEN commit split, for all three tasks (tdd=true), matching every prior plan in this phase (03-01 through 03-08). Each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task."
  - "The join excludes two entries, one on each side, not one: this session measured, against three independent corpus programs (SK-Gradient-Sample__VB6, Grayscale-effect, LockWorkStation, no two sharing a form name), that the ControlInfo array always carries one entry for the form itself, and that entry is never the form's own declared name. It is always the literal string \"Form\", with wEventCount 31 in all three samples. STRUCTURES.md section 8.6 does not name this. join_by_name therefore excludes the tree's own root (the form) from the tree side and the literal entry named \"Form\" from the info side, so the form's own slot is never reported as unjoined. The plan's own acceptance criterion (both unmatched lists empty on a real corpus program) does not hold without this exclusion: the first implementation, excluding only the tree's own root, left \"Form\" unmatched on the info side on every corpus file tried."
  - "A control array shares one ControlInfo entry across all of its own elements, not one per element. Measured on Grayscale.exe: optDecompose (2 elements) and optChannel (3 elements) each give exactly one ControlInfo entry, not two or three. join_by_name's own BTreeSet-based name comparison already collapses a repeated tree name to one set member, so this needed no special handling; a dedicated corpus test (grayscale_joins_both_directions_with_no_unmatched_entry_despite_two_control_arrays) proves it rather than assuming it."
  - "The plan's own artifact table names read_control_info as a new symbol. This plan implements the equivalent behaviour as the associated function ControlInfoTable::read, backed by a private read_raw_control_info that reads one element's fixed-width fields (mirroring vb/controltree.rs::read_control_header's own pure, Region-only shape) plus a separate read_name step for the pointer-resolved name. No test in this plan's own acceptance criteria greps for the literal function name, and the plan's own \"Artifacts this phase produces\" table documents new symbols for drift detection rather than functioning as a strict allowlist (03-04's own SUMMARY records the same kind of addition, read_block/apply_pops/block_fits/close_walk, none named in its own table). Recorded here for the same reason: honesty about what shipped against what was named."
  - "report_events attempts a name lookup only for a bound slot, never for an unbound one. VB6 declares a source-level event procedure only for a bound slot, so naming an event nothing calls serves no recovery this repository writes. This is what makes the report exactly three states (Named, BoundUnnamed, Unbound) rather than four: an unbound slot is never subdivided into named/unnamed, because no lookup is attempted for it at all."
  - "EventNameTable ships with zero entries, not the safe-provenance subset OpcodeTable (plan 03-02) carries for properties. 03-CONTEXT.md D-02 treats the event ordering as needing the tool, not a table, with no carve-out for a small cited subset the way D-01 carves one out for properties: the vtable ordering (which ordinal is which event) is what is not commonly published, and this plan's own read_first list names no source that publishes it safely enough to transcribe even a handful of facts."

patterns-established:
  - "A defensive two-state clamp (bound_control_count, bound_event_count) that reuses one existing DefectKind (ImplausibleCount) rather than adding a new variant per caller, following vb/object.rs::bound_proc_count's own precedent exactly, including the 'clamp gives no defect when the array pointer itself does not resolve' rule."

requirements-completed: [FRM-06]

coverage:
  - id: D1
    description: "ControlInfoTable::read resolves OptionalObjectInfo at Object.lpObjectInfo plus 0x38, reads fControlType and wEventCount as two byte values, bounds dwControlCount against the real file length before it sizes a loop, and joins the recovered control tree to ControlInfo by name in both directions, correctly for a form with two control arrays"
    requirement: "FRM-06"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::f_control_type_and_w_event_count_read_as_two_byte_values_at_0x00_and_0x02"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::the_second_array_element_starts_forty_bytes_after_the_first"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::a_dw_control_count_larger_than_the_file_can_hold_gives_a_defect_and_sizes_no_allocation"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::an_lpsz_name_address_in_no_section_gives_an_empty_name_and_a_defect_naming_the_address"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::sk_gradient_sample_joins_both_directions_with_no_unmatched_entry"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::grayscale_joins_both_directions_with_no_unmatched_entry_despite_two_control_arrays"
        status: pass
    human_judgment: false
  - id: D2
    description: "read_event_table selects the header size from fControlType (0x18 for 0x40, 0x28 for 0x2E, no slots for any other value), reports every slot by ascending index without renumbering, and decodes the native stub with checked signed arithmetic, proven byte for byte against one real corpus stub"
    requirement: "FRM-06"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::an_f_control_type_of_0x40_puts_the_first_slot_at_offset_0x18"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::an_f_control_type_of_0x2e_puts_the_first_slot_at_offset_0x28"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::an_f_control_type_of_0x41_gives_no_slots_and_a_reason_naming_0x41"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::a_slot_of_zero_reports_at_its_own_index_as_unbound_and_the_next_index_is_unchanged"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::a_negative_relative_jump_gives_a_handler_address_below_the_stub_start"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::an_immediate_value_of_0xffff_marks_a_method_and_a_smaller_value_marks_an_event"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::sk_gradient_sample_command1_slot_zero_decodes_to_the_measured_stub"
        status: pass
    human_judgment: false
  - id: D3
    description: "report_events gives three honest states (named, bound with no name, unbound), never inventing a name from the slot index or a different control type, with the name source taken as a parameter and zero event names compiled in"
    requirement: "FRM-06"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::a_bound_slot_with_a_named_table_entry_gives_the_named_state"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::a_bound_slot_with_no_table_gives_the_bound_unnamed_state_and_a_reason"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::an_unbound_slot_gives_the_unbound_state_and_a_reason"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::a_slot_on_a_control_type_with_no_table_entry_reports_no_name"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::two_runs_over_the_same_bytes_give_the_same_report_in_the_same_order"
        status: pass
      - kind: other
        ref: "TRACKED=$(git ls-files) && test 0 -eq \"$(printf '%s\\n' \"$TRACKED\" | grep -ci 'event.*name.*table\\.\\(toml\\|txt\\|bin\\|csv\\)')\""
        status: pass
    human_judgment: false

duration: 23min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 9: The Event Handler Table Summary

**ControlInfo joined to the control tree by name (both directions, gated on the measured fact that a form's own slot is always literally named "Form"), the native event stub decoded with checked signed arithmetic and proven against one real corpus stub, and every event name reported honestly with zero names compiled in, per D-02.**

## Performance

- **Duration:** 23 min (measured from plan 03-08's completion commit to this plan's final task commit)
- **Started:** 2026-09-10T14:56:10+02:00 (approximate, plan 03-08's completion commit)
- **Completed:** 2026-09-10T15:18:55+02:00
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- `ControlInfoTable::read` resolves `OptionalObjectInfo` at `Object.lpObjectInfo` plus `0x38`, a fresh `0x40`-byte window taken before any field inside it is read. `fControlType` and `wEventCount` are read as two byte values, at offsets `0x00` and `0x02`: three independent sources agree against the one that reads a four byte value at `0x00` and a word at `0x04`. `dwControlCount` is bounded against the real length of the file that remains from `lpControls` before it sizes a loop, mirroring `vb/object.rs::bound_proc_count` exactly. A name pointer that resolves to no section gives an empty name and a `Defect` naming the address.
- `join_by_name` joins the control tree to `ControlInfo` by control name, in both directions. This session measured, against three independent corpus programs, that the `ControlInfo` array always carries one entry for the form itself, and that entry is never the form's own declared name — it is always the literal string `"Form"`. The join excludes the tree's own root and the `"Form"` entry, one on each side, so the form's own slot is never reported as unjoined. Proven clean (both unmatched lists empty) against `SK-Gradient-Sample__VB6` (3 hosted controls, no arrays) and `Grayscale.exe` (14 hosted controls including two control arrays, `optDecompose` and `optChannel`, each collapsing correctly to the one `ControlInfo` entry its own array shares).
- `read_event_table` selects the event table header size from `fControlType`: `0x18` (6 dwords) for `0x40`, `0x28` (10 dwords) for `0x2E`, and no slots with a stated reason naming the value for any other. The header end and the first slot offset come from the one computation, so the two cannot drift apart. A slot of zero reports as unbound at its own index; the index after it is never renumbered. `wEventCount` is bounded the same way `dwControlCount` is.
- `decode_stub` reads the native stub `81 6C 24 04 <imm32> E9 <rel32>`: an `imm32` of `0xFFFF` marks a method, a smaller value marks an event, and the handler address is the stub start plus 13 plus the signed `rel32`, computed with `u32::checked_add_signed` so a negative jump gives an address below the stub start rather than wrapping. Proven byte for byte against `SK-Gradient-Sample__VB6`'s own `Command1`, event slot 0: bytes `81 6c 24 04 3f 00 00 00 e9 23 04 00`, `imm32 = 0x3F` (an event, matching `STRUCTURES.md`'s own citation of AG's sample), `rel32 = 0x423`, handler address `4,201,984`. This module reads the native stub shape only; the corpus holds no P-code sample, and the doc comment names that absence directly rather than silently.
- `report_events` gives three honest states: `Named` (a bound slot whose name a supplied `EventNameTable` gave), `BoundUnnamed`, and `Unbound`. Per `03-CONTEXT.md` D-02, `EventNameTable` ships zero entries: the vtable ordering an ordinal maps into is not commonly published, so this task builds no table, only the honest report and the seam a table would plug into later. The two no-name states carry a reason in the same shape `PropertyValue::undecoded_message` (plan 03-06) gives for an unnamed property opcode: the slot index, the control name, the fact that no table is loaded, and `--event-name-table` as the way to supply one.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: ControlInfo and the join by control name** - `1ab5e30` (feat, tdd="true")
2. **Task 2: The event handler table and the stub** - `e8146e9` (feat, tdd="true")
3. **Task 3: The event name, reported honestly per D-02** - `a2a9ffb` (feat, tdd="true")

**Plan metadata:** commit follows this SUMMARY.

_Note: all three tasks carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase), each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task._

## Files Created/Modified

- `crates/deform6/src/vb/controlinfo.rs` - `ControlInfo`, `ControlInfoTable`, `read_raw_control_info`, `bound_control_count`, `read_name`, `ControlJoin`, `join_by_name`, `FORM_SELF_ENTRY_NAME` (Task 1); `EventSlot`, `StubHandler`, `EventTable`, `read_event_table`, `bound_event_count`, `decode_stub`, `event_table_header_len` (Task 2); `EventNameTable`, `EventReport`, `report_events` (Task 3); 31 unit and integration tests total

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential: the join excludes the `ControlInfo` entry literally named `"Form"` (not the form's own declared name) alongside the tree's own root, a fact this session measured directly rather than assumed from `STRUCTURES.md`, which does not name it; and `EventNameTable` ships zero entries with no safe-provenance carve-out, because `03-CONTEXT.md` D-02 gives the vtable ordering no such carve-out the way D-01 gives one for properties.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The join's first implementation left the form's own "Form" entry unjoined on every corpus file**
- **Found during:** Task 1, writing the corpus join test against `SK-Gradient-Sample__VB6`
- **Issue:** The first implementation excluded only the tree's own root from the tree side. Measured against real bytes, the `ControlInfo` array always carries an extra entry for the form itself, literally named `"Form"` (never the form's own declared name), which then appeared as an unmatched `info_only` entry on every corpus file tried, failing the plan's own acceptance criterion that both unmatched lists be empty.
- **Fix:** Added a symmetric exclusion on the info side: a `ControlInfo` entry named `"Form"` (the literal, measured constant `FORM_SELF_ENTRY_NAME`) is excluded from the join's own name comparison, documented with the measurement that justifies it.
- **Files modified:** `crates/deform6/src/vb/controlinfo.rs`
- **Verification:** `sk_gradient_sample_joins_both_directions_with_no_unmatched_entry`, `grayscale_joins_both_directions_with_no_unmatched_entry_despite_two_control_arrays`
- **Committed in:** `1ab5e30` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug, found and fixed before the task's own commit; no defect shipped).
**Impact on plan:** Necessary for the plan's own acceptance criterion (both unmatched lists empty on a real corpus program) to hold at all. No scope creep: the fix is one named constant and one filter predicate, both documented with the measurement behind them.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's two-byte field reading** — `w_event_count` changed to read at offset `0x04` instead of `0x02`. `cargo test -p deform6 --lib vb::controlinfo::tests::f_control_type_and_w_event_count_read_as_two_byte_values_at_0x00_and_0x02` run once:

```
assertion `left == right` failed
  left: 48042
 right: 5
```

The fixture's own distinguishing filler byte at offset `0x04`-`0x05` (`0xBBAA` = 48042) is exactly what a wrong-offset read would return instead of the real `wEventCount` (5), proving the offset the test targets is load-bearing. Reverted `w_event_count`'s own offset to `0x02` before committing `1ab5e30`.

**Task 2's COM event table header size** — `COM_EVENT_HEADER_LEN` changed from `0x28` to `0x18`. `cargo test -p deform6 --lib vb::controlinfo::tests::an_f_control_type_of_0x2e_puts_the_first_slot_at_offset_0x28` run once:

```
assertion failed: matches!(table.slots[0], EventSlot::Bound { index: 0, stub, .. } if stub ==
    Va::new(0x0040_1100))
```

With the wrong header size (`0x18`), the reader looked for the first event slot at offset `0x18` instead of the correct `0x28`. The fixture only wrote the marker address `0x00401100` at offset `0x28`; offset `0x18` in this fixture holds all-zero bytes, so the wrong-offset read gave `EventSlot::Unbound { index: 0 }` instead of the expected `EventSlot::Bound { stub: Va(0x00401100), .. }`, and the `matches!` assertion failed. Both offsets — the wrong one read (`0x18`) and the correct one the fixture actually wrote the marker at (`0x28`) — are recorded here. Reverted `COM_EVENT_HEADER_LEN` to `0x28` before committing `e8146e9`.

**Task 3's no-name branch** — the `None` arm of `report_events`'s own lookup match changed to synthesize `EventReport::Named { event_name: format!("Event{index}"), .. }` instead of `EventReport::BoundUnnamed`. `cargo test -p deform6 --lib vb::controlinfo::tests::a_bound_slot_with_no_table_gives_the_bound_unnamed_state_and_a_reason` run once:

```
assertion `left == right` failed
  left: Named { control_name: "Command1", index: 3, event_name: "Event3" }
 right: BoundUnnamed { control_name: "Command1", index: 3 }
```

The invented name the broken branch produced was `"Event3"`, derived mechanically from the slot index — exactly the shape D-02 forbids. Reverted to `EventReport::BoundUnnamed` before committing `a2a9ffb`.

## Issues Encountered

None beyond the deviation above.

## What this plan recovers, and what it plainly does not

Per this repository's own measurement rule (`AGENTS.md`, "Measurement"): give the number that can be proven, never a bare success number.

- **Control-to-event-table join (structure):** `SK-Gradient-Sample__VB6` joins 3 of 3 hosted controls (0 unmatched on either side); `Grayscale.exe` joins 14 of 14 hosted controls, including both control arrays collapsing correctly to their own one shared entry (0 unmatched on either side). Every control this repository's own control tree recovers for these two programs also resolves a `ControlInfo` entry.
- **Event slot structure (bound or unbound, and the index):** fully recovered for every corpus file this session measured. `SK-Gradient-Sample__VB6`'s `Command1` gives 17 of 17 event slots, correctly split into 1 bound (slot 0) and 16 unbound, with the bound slot's stub decoded to a real handler address.
- **Event names:** **zero recovered, anywhere in this corpus, by design.** `EventNameTable` ships no entries, per `03-CONTEXT.md` D-02: the vtable ordering an ordinal maps into is not commonly published, and this repository does not commit a table built from dumping `VB6.OLB`. Every event slot in every corpus file this plan touches reports `BoundUnnamed` or `Unbound`, never a name. This is the honest, deliberate outcome D-02 asks for, not a shortfall this plan silently absorbed: a future plan that supplies a caller-built `EventNameTable` (built from independently cited public sources, never a type library dump) plugs into the exact same `report_events` lookup path with no code change.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every symbol this plan's own artifact table names (`ControlInfo`, `ControlInfoTable`, `EventTable`, `EventSlot`, `read_event_table`, `CONTROL_INFO_SIZE`) is implemented and tested against real corpus bytes, not a placeholder. `ControlInfoTable::read` is this plan's own name for the reader the table's own `read_control_info` entry names; see key-decisions for why.

## Threat Flags

None. Every trust boundary this plan's own `<threat_model>` names is mitigated exactly as the threat register states: T-03-06 by `bound_control_count`/`bound_event_count`, both checked before any loop; T-03-47 by `event_table_header_len`'s own closed match and the single shared computation for the header end and the first slot offset; T-03-48 by `u32::checked_add_signed` over the whole signed range; T-03-49 by `NAME_MAX`-bounded `cstr`; T-03-50 by `report_events` never deriving a name from the slot index and `EventNameTable` shipping zero entries; T-03-51 by the doc comment naming the P-code stub shapes as not read. This plan introduces no new trust boundary beyond those.

## Next Phase Readiness

- `ControlInfoTable`, `EventTable`, `EventSlot`, `StubHandler`, `EventNameTable` and `report_events` are ready for plan 03-10's differential gate and `Report` to compose into the phase's final output.
- `join_by_name`'s own measured exclusion (the tree's own root, and the `ControlInfo` entry literally named `"Form"`) is the fact a later reader needs before joining these two structures by name anywhere else in this codebase.
- No blockers for plan 03-10.

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

`crates/deform6/src/vb/controlinfo.rs` found on disk. All 3 task commits
(`1ab5e30`, `e8146e9`, `a2a9ffb`) found in `git log`. `cargo test --workspace`
passes (361 lib tests, up from 330 before this plan, plus every integration
and CLI test), `cargo fmt --all --check` and `cargo clippy --all-targets -- -D
warnings` both pass clean, and `cargo test -p deform6 --lib vb::controlinfo`
gives 31 tests, exceeding all three tasks' own stated minimums (12, 24, 31).
The plan's own D-02 verification command (`git ls-files` grep for a tracked
event name table file) passes.
