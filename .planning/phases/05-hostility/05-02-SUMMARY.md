---
phase: 05-hostility
plan: 02
subsystem: safety
tags: [rust, capacity, defect, gui-table, audit, ci]

requires:
  - phase: 05-hostility
    provides: "Severity::Tolerated and the wired Journal from plan 05-01, so a Recoverable ImplausibleCount defect refuses in strict mode and clamps in salvage mode"
provides:
  - "gui::bound_form_count, closing the one named SAF-04 gap: GuiTable::walk now bounds wFormCount against the GUI table's own mapped region before the loop starts"
  - "GuiTable::defects(), matching ControlInfoTable::defects, wired into inspect's defect list"
  - "A measured, written audit of every capacity sized allocation under crates/deform6/src/ (14 sites, 9 files) and every named count/length field in the parse order, each traced to the check that bounds it"
  - "scripts/prove-capacity-wall.sh, a fourth gate step beside the lint wall and the region wall, that fails when a new unaudited capacity site enters the library or an audited row goes stale"
affects: [05-03, 05-04, 05-05, 05-06, 05-08]

actuals:
  tokens: 7167
  tasks: 3
  commits: 4
  plan_head_before: cf707362743d008ca877c66ec9074232446c7fc7

tech-stack:
  added: []
  patterns:
    - "The four step bound shape (null/short-circuit region check, checked division, comparison, ImplausibleCount defect construction) that object.rs::bound_proc_count set is now proven to generalize losslessly: gui.rs::bound_form_count is a direct copy with no new abstraction, per the plan's own prohibition on a shared generic utility."
    - "A capacity sized allocation is safe by one of exactly two routes: `internal` (the value is the length of a collection this crate already built in memory, or a compile time constant) or `bounded` (a `Region::subregion` call, or an explicit comparison against a region length, runs before the allocation and the None/too-large branch returns early with no allocation at all). Every one of the 14 measured sites falls cleanly into one of the two; no third category was needed."
    - "A capacity wall proves an audited list against the tree both ways (a real site missing from the list, and a list row missing from the tree), the same two-directional discipline ratios.rs's pinned-file gate and ControlInfoTable/GuiTable's own defect join already use elsewhere in this codebase."

key-files:
  created:
    - scripts/prove-capacity-wall.sh
  modified:
    - crates/deform6/src/vb/gui.rs
    - crates/deform6/src/vb/mod.rs
    - crates/deform6/src/vb/privateobj.rs
    - crates/deform6/src/vb/functyp.rs
    - .github/workflows/gate.yml

key-decisions:
  - "Task 2's audit measured 14 capacity sized allocations across 9 files in production code, not the 13 across 8 files this plan's own planning session measured, and not the 6 the research document states. The full list and the counting method are in this file's own Task 2 section below. Per AGENTS.md, the measured number is recorded and neither of the other two is treated as correct."
  - "Task 2 found zero unbounded capacity sites and zero unbounded count/length fields. Every one of the five sites the plan named for particular attention (two in privateobj.rs, three in functyp.rs) is already bounded by its own preceding array.subregion check, independent of any upstream clamp. No code fix was needed; two regression tests were added per file (three total) to prove the claim empirically rather than leave it as a comment, matching this repository's own precedent (object.rs's an_implausible_proc_count_is_bounded_and_clamped)."
  - "The end to end test for Task 1's success criterion 4 (inspect() refusing a hand made 4096 byte image with a 0xFFFF form count) could not be built as literally specified: a fully synthetic image has no import table, so inspect refuses at runtime detection, long before the GUI table; a real corpus file with a patched wFormCount reaches GuiTable::walk's own bound and defect, but the clamped loop's second iteration then reads unrelated file bytes and refuses on lStructSize before the defect ever reaches Journal::record, identically in both modes. Per the plan's own documented fallback, the direct proof lives at the GuiTable::walk level (gui::tests::gui_table_refuses_an_implausible_form_count), and a second test (vb::tests::an_implausible_form_count_against_a_real_program_still_refuses_and_never_panics) proves the real-program case never panics and refuses identically in both modes, for the documented different reason."
  - "The capacity wall script matches an audited row against the real tree by exact file-plus-trimmed-source-line text, not by line number: a line number drifts every time an unrelated edit lands above it, and AGENTS.md's own testing philosophy (match the shape of the file, not one form of it) argues against a fragile line-number key."

patterns-established:
  - "A two-directional wall script (audited-but-gone is a FAIL, present-but-unaudited is a FAIL) as the fourth gate step, following prove-lint-wall.sh and prove-region-wall.sh's own shape: write nothing to the tree, run the three gate commands at the end, and name the exact file and line a FAIL corresponds to."

requirements-completed: [SAF-04]

coverage:
  - id: D1
    description: "GuiTable::walk bounds wFormCount against its own mapped region before the loop starts; a 4096 byte image declaring 0xFFFF forms gives exactly one entry and one ImplausibleCount defect with a maximum of 1, and allocates nothing from the declared count"
    requirement: "SAF-04"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/gui.rs#tests::gui_table_refuses_an_implausible_form_count"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/gui.rs#tests::a_declared_form_count_of_zero_gives_no_entry_and_no_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/gui.rs#tests::a_gui_table_region_of_zero_usable_bytes_gives_a_maximum_of_zero"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/mod.rs#tests::an_implausible_form_count_against_a_real_program_still_refuses_and_never_panics"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every capacity sized allocation under crates/deform6/src/ is traced to internal or bounded, written down in an audited list, and a new unaudited one fails the gate"
    requirement: "SAF-04"
    verification:
      - kind: other
        ref: "sh scripts/prove-capacity-wall.sh"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/privateobj.rs#tests::a_proc_count_the_array_cannot_back_gives_the_no_name_array_state_and_no_allocation"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/privateobj.rs#tests::a_cnt_events_the_array_cannot_back_gives_an_empty_list_and_no_allocation"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/functyp.rs#tests::a_proc_count_the_array_cannot_back_gives_the_no_func_type_array_state"
        status: pass
    human_judgment: false

duration: 1 session
completed: 2026-09-13
status: complete
---

# Phase 5 Plan 02: The GUI table bound check, and the capacity audit wall Summary

**GuiTable::walk now bounds wFormCount against its own mapped region before it loops (closing the one named SAF-04 gap), and a fourth gate script, prove-capacity-wall.sh, proves the other 14 capacity sized allocations this library ever builds are all already internal or bounded, zero found unbounded.**

## Performance

- **Duration:** 1 session
- **Tasks:** 3
- **Files modified:** 6 (1 new script, 5 modified)

## Accomplishments

- `gui::bound_form_count` added, copying `object.rs::bound_proc_count`'s four steps exactly (no shared generic utility). `GuiTable` carries the defects the walk finds and exposes them through a `defects()` accessor, matching `ControlInfoTable`. `inspect` extends its report-level defect list from the GUI table, so a hostile `wFormCount` refuses in strict mode and clamps in salvage mode, same as every other bounded count in this codebase.
- A hand made 4096 byte image whose mapped GUI table region holds exactly one entry and whose declared `wFormCount` is `0xFFFF` gives exactly one entry and one `ImplausibleCount` defect naming count `65535` and maximum `1`, proven directly at the `GuiTable::walk` level. A companion test proves a real corpus program with the same field patched never panics and refuses safely, in both modes, for a different, documented reason.
- Task 2's audit measured every capacity sized allocation under `crates/deform6/src/`: 14 sites across 9 files, not the 13/8 this plan's own planning session recorded and not the 6 the research document states. All 14 trace cleanly to `internal` (the value is the length of a collection this crate already built, or a compile time constant) or `bounded` (a `Region::subregion` check runs before the allocation). Zero `unbounded` sites found. Every named count/length field in the parse order (11 fields) was walked and traced to its own bounding comparison; all 11 already had one, the eleventh being this plan's own new `bound_form_count`.
- `scripts/prove-capacity-wall.sh` holds the 14-row audited list and checks the real tree against it both ways: a new, un-audited `with_capacity` call fails the script and names the file and line; a stale row whose site is gone also fails. Wired into `.github/workflows/gate.yml` as a fourth named step, `Prove the capacity wall`, beside the lint wall and the region wall.

## Task Commits

1. **Task 1: The GUI table form count, bounded before the loop starts** - `e9db447` (feat)
2. **Task 2: The audit of every allocation this library sizes from a file field** - `cfafbea` (test, privateobj.rs), `e01bb82` (test, functyp.rs)
3. **Task 3: The capacity wall, so the audit stays done** - `4572b36` (feat)

**Plan metadata:** this commit (SUMMARY, STATE, ROADMAP).

## Files Created/Modified

- `crates/deform6/src/vb/gui.rs` - `bound_form_count`, `GuiTable::defects()`, the walk now clamps before it loops, four new tests, and the `synthetic_image_with_no_bytes_in_its_section` and `synthetic_image_of_len` test fixtures.
- `crates/deform6/src/vb/mod.rs` - `inspect` extends its defect list from the GUI table; one new end to end test proving a real corpus program with an implausible `wFormCount` never panics.
- `crates/deform6/src/vb/privateobj.rs` - Two new regression tests proving `ProcedureList::read` and `event_descriptor_addresses` each bound their own `with_capacity` call independently of any upstream clamp.
- `crates/deform6/src/vb/functyp.rs` - One new regression test proving `FuncTypeWalk::read` bounds its own `with_capacity` call against `lpFuncTypeInfo`'s own region.
- `scripts/prove-capacity-wall.sh` - New. The capacity wall: an audited list and a two-directional check against the real tree.
- `.github/workflows/gate.yml` - One new named step, `Prove the capacity wall`, after the region wall step.

## Task 2: The Capacity Site Audit

**Measurement method.** `find crates/deform6/src -name '*.rs'`, then for each file: drop every line from the first line reading exactly `#[cfg(test)]` to the end of the file, drop every whole-line comment, and grep the remainder for `with_capacity(`. This is the same rule `vb/controlinfo.rs::production_code_only` already uses for its own single-file check, generalized to the whole crate. `scripts/prove-capacity-wall.sh` runs this exact method on every gate run, not only once at planning time.

**Result: 14 sites, 9 files.** Neither the plan's own 13/8 nor the research document's 6 is the number recorded; this is the number the method above produces against the tree as this plan leaves it.

| # | File | Expression (trimmed) | Verdict | Check |
|---|------|----------------------|---------|-------|
| 1 | `write/values.rs` | `String::with_capacity(text.len().saturating_add(2))` | internal | `text: &str` is already resident in memory; no file field drives its length |
| 2 | `report.rs` | `Vec::with_capacity(model_items.len())` | internal | `model_items: Vec<ReportItem>` is a Vec this crate already built, taken by value |
| 3 | `write/frm.rs` | `Vec::with_capacity(order.len())` | internal | `order` is `(0..pending.len()).collect()`, built locally from a Vec this function already built |
| 4 | `write/comment.rs` | `Vec::with_capacity(lines.len().saturating_add(1))` | internal | `lines` is built locally in this function from items already recovered |
| 5 | `read/pe.rs` | `Vec::with_capacity(table.len())` | bounded | `table` is the `object` crate's own `SectionTable`, already parsed and length-checked against the real file bytes by `SectionTable::parse` (a fallible `read_slice_at`) before `PeImage::parse` ever returns it |
| 6 | `write/model.rs` | `Vec::with_capacity(text.len())` (`encode_windows_1252`) | internal | `text: &str` parameter, already resident in memory |
| 7 | `write/model.rs` | `String::with_capacity(raw.len())` (`SafeNameIssuer::new`) | internal | `raw: &str` parameter, already resident in memory |
| 8 | `write/model.rs` | `Vec::with_capacity(controls.len())` (`build_controls`) | internal | `controls: &[ControlReport]`, a slice this crate already built during `inspect()` |
| 9 | `vb/privateobj.rs` | `Vec::with_capacity(usize::try_from(object.proc_count).unwrap_or(0))` (`ProcedureList::read`) | bounded | `object.rs::bound_proc_count` clamps `object.proc_count` upstream against `lpProcNamesArray`'s own region (`object.rs` line ~292), and this function's own preceding `array.subregion(Off::new(0), window_size)` proves the bytes exist before this line, independent of the upstream clamp |
| 10 | `vb/privateobj.rs` | `Vec::with_capacity(usize::from(*cnt_events))` (`event_descriptor_addresses`) | bounded | This function's own preceding `array.subregion(Off::new(0), window_size)` proves the bytes exist before this line; there is no upstream clamp for `cnt_events` (the corpus never carries a non-zero value, per the module's own doc comment) |
| 11 | `vb/project.rs` | `Vec::with_capacity(36)` (`decode_guid_text`) | internal | A compile time constant, reached only inside the `guid_length == 72` arm, after `entry.take(guid_offset, 72)` already proved 72 bytes present |
| 12 | `vb/functyp.rs` | `Vec::with_capacity(usize::try_from(object.proc_count).unwrap_or(0))` (`FuncTypeWalk::read`) | bounded | Same upstream clamp as row 9, and this function's own preceding `array.subregion(Off::new(0), window_size)` against `lpFuncTypeInfo`'s own region proves the bytes exist before this line |
| 13 | `vb/functyp.rs` | `Vec::with_capacity(wanted)` (`walk_type_buffer`) | bounded | `arg_size` is a `u8`; `wanted = arg_size >> 2` is domain-bounded to a maximum of 63 entries whatever the file declares, independent of any region length check |
| 14 | `vb/functyp.rs` | `Vec::with_capacity(arg_count)` (line ~1059) | internal | `arg_count = entries.len()`, the length of a `Vec` `walk_type_buffer` already built and returned |

**Rows 9, 10, 12: the five sites the plan named for particular attention.** All five are bounded by their own preceding `array.subregion` check, which is what actually stops an over-large allocation, proven directly, not assumed, by deliberately removing the check in `privateobj.rs::ProcedureList::read` and watching the new regression test fail (see Deliberate Breakages below). Two of the five (rows 9 and 12) are additionally bounded upstream by `object.rs::bound_proc_count`, but neither depends on that upstream clamp alone.

### Count and length field table

Every count field and every length field the parse order names, per the plan's own list, with the comparison that bounds it:

| Field | Bounded by | File : line |
|---|---|---|
| Object count (`wTotalObjects`) | No capacity allocation reads it (`ObjectTable::walk` uses `Vec::new()`); each element is gated by its own `array.subregion` check, so the loop cannot run past real data. A disagreement with `wCompiledObjects` raises `CountMismatch` | `vb/object.rs::ObjectTable::walk` (~144-149); `vb/project.rs::count_defects` |
| Procedure count (`ProcCount`) | Compared against `lpProcNamesArray`'s own region; clamped | `vb/object.rs::bound_proc_count` (~279-311) |
| Public variable count (`cntPublicVars`) | Never used to size a loop or an allocation anywhere in this codebase; carried as inert data only (`let _ = cnt_public_vars;`) | `vb/privateobj.rs` (~957) |
| Event count (`wEventCount`, per control) | Compared against the event table's own region; clamped | `vb/controlinfo.rs::bound_event_count` (~563) |
| Event count (`cnt_events`, per object) | This function's own preceding `array.subregion` check | `vb/privateobj.rs::event_descriptor_addresses` (~707-711) |
| Argument count (`argSize`-derived) | Domain-bounded by `argSize`'s own `u8` width (max 63); `arg_count` itself is `entries.len()` of an already-built Vec | `vb/functyp.rs::walk_type_buffer` (~588); `vb/functyp.rs` (~1050) |
| External count (`dwExternalCount`) | Compared against the declare table's own region; `ImplausibleCount` raised, loop separately bounded by `subregion` | `vb/project.rs::DeclareTable::read` (~547) |
| External count (`wExternalCount`) | No capacity allocation reads it (`Vec::new()`); the loop self-terminates when `table.file_offset` fails | `vb/project.rs::ComponentTable::read` (~866-900) |
| Form count (`wFormCount`) | This plan's new check | `vb/gui.rs::bound_form_count` |
| Control count (`dwControlCount`) | Compared against the `ControlInfo` array's own region; clamped | `vb/controlinfo.rs::bound_control_count` (~273-298) |
| Property stream length (`lPropertiesLength`) | `Region::subregion` (uses `checked_add` internally); a length past the file end refuses rather than allocates | `vb/gui.rs::GuiObjectInfo::form_stream` (~171-186) |
| Control block length (`Length`, per control) | `checked_add` then `Region::subregion`; cross-checked against the running total by `Tiling` | `vb/controltree.rs::read_block` (~646-652) |
| Resource blob length (`blob_len`) | Compared against the block's own remaining bytes before any `take`/allocation | `vb/frx.rs::extract_blob` (~165-182) |

**Zero `unbounded` rows.** Every named field already had a bounding comparison before this plan started, except `wFormCount`, which Task 1 closed. This matches the plan's own framing: "the discipline is uneven today, and the gap is located", the gap was singular.

## Decisions Made

See `key-decisions` in the frontmatter.

## Deviations from Plan

### Auto-fixed Issues

None. Every step of every task executed as the plan specified, once the count-field and capacity-site audits produced zero `unbounded` rows (a measured outcome, not a deviation: Task 2's own action text explicitly allows for "the audit is worth doing once" to find nothing new to fix).

**Total deviations:** 0. **Impact:** none.

## Deliberate Breakages (required by the plan, reverted before commit)

- **Task 1:** Removed the call to `bound_form_count` in `gui.rs::GuiTable::walk` (used the raw, unclamped `header.w_form_count` directly), ran `gui_table_refuses_an_implausible_form_count`. It failed with `Damaged("the file ends inside a GUI table entry")`: the second loop iteration (index 1) tried to read a second `GUI_ENTRY_SIZE`-byte entry from a mapped region sized to hold exactly one, and `subregion` refused it, instead of the test's expected one entry plus one `ImplausibleCount` defect. Reverted; gate re-confirmed green.
- **Task 2:** Removed the `array.subregion(Off::new(0), window_size)` check (and its preceding `checked_mul`) from `privateobj.rs::ProcedureList::read`, using the unbounded `array` directly as the read window, then ran `a_proc_count_the_array_cannot_back_gives_the_no_name_array_state_and_no_allocation`. It failed on an `assert_eq!` mismatch: instead of `NoNameArray { proc_count: 10_000_000 }`, the walk produced `Slots([...])`, a list of 10 million entries built by reading past `FastDrawing`'s real 8-entry array into unrelated file bytes, mostly `Private` (unreadable pointers), with at least one bizarre `Public("VB")` resolved from a garbage address that happened to land on readable, plausible-identifier bytes elsewhere in the file. This is the exact failure mode the bound check exists to prevent: an allocation and a read walk driven by a declared count with nothing checking it against the region that must back it. Reverted; gate re-confirmed green.
- **Task 3:** Ran `sh scripts/prove-capacity-wall.sh` on the tree as Task 2 left it (passed, 14/14). Removed the `String::with_capacity` call from `write/values.rs` temporarily (replaced with `String::new()`) and re-ran the script: it failed with `FAIL  stale audit row, no longer in the source: crates/deform6/src/write/values.rs: ...`, naming the exact row to remove. Reverted. Then added a throwaway `fn deliberate_probe_allocation(n: usize) -> Vec<u8> { Vec::with_capacity(n) }` to `vb/ocx.rs` (before its test module) and re-ran the script: it failed with `FAIL  un-audited capacity site: crates/deform6/src/vb/ocx.rs:33` and printed the offending line and the trace-and-add instructions. Removed the probe function; gate re-confirmed green.

## Issues Encountered

The plan's own success-criterion-4 end to end test (through `inspect()`, not `GuiTable::walk` directly) could not be built as literally written; the plan itself anticipated and permitted this fallback (see key-decisions). No blocker: the direct proof exists at `GuiTable::walk`, and a second test proves the real-program refusal path never panics.

## User Setup Required

None. No external service configuration required.

## Next Phase Readiness

- `Severity::Tolerated`, the wired `Journal`, `bound_form_count`, and the capacity wall are all in place for plan 05-03 (the fuzz target), which the roadmap already names as needing both `Mode::Strict` and `Mode::Salvage` exercised directly.
- The capacity wall runs on every gate invocation from this commit forward; a future plan that adds an unaudited `with_capacity` call anywhere under `crates/deform6/src/` will fail CI with the file and line named, per success criterion 4 of the phase roadmap.
- No blockers. `cargo test --workspace` is green at 916 passed, 0 failed, the new floor for plan 05-03 onward (up from the 909 recorded after plan 05-07: +4 tests from Task 1, +3 tests from Task 2, 0 from Task 3).

---

*Phase: 05-hostility*
*Completed: 2026-09-13*

## Self-Check: PASSED
