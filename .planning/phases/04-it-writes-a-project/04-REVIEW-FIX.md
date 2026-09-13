---
phase: 04
fixed_at: 2026-09-13T00:00:00Z
review_path: .planning/phases/04-it-writes-a-project/04-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 4: Code Review Fix Report

**Fixed at:** 2026-09-13T00:00:00Z
**Source review:** .planning/phases/04-it-writes-a-project/04-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (CR-01, CR-02, CR-03, WR-01, WR-02, WR-03)
- Fixed: 6
- Skipped: 0
- Out of scope, untouched per the task's own instruction: IN-01

Verification for every commit below ran the full gate in the main
checkout (worktree isolation was off for this run, per the task's
execution context): `cargo fmt --all --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo test --workspace`. All three
passed after each commit. The final state (603 tests) is reproducible
from this same checkout.

## Fixed Issues

### CR-01: `append_blob` advanced the shared `BlobCursor` before checking the blob's range

**Status:** fixed
**Files modified:** `crates/deform6/src/write/frm.rs`
**Commit:** `1e20c6e`

**Applied fix:** Reordered `append_blob` so the byte-range bound check
against `data` runs first, and `blob_cursor.take(&blob)` is called only
once the range is known to fit and the bytes are about to be appended.
A refusal path now leaves the cursor exactly where it was, so no later
blob in the same form is assigned a `.frx` offset inflated by a
phantom advance.

**Test:** `a_blob_whose_range_does_not_fit_leaves_the_cursor_unmoved_for_the_next_blob`,
added to `crates/deform6/src/write/frm.rs`. It gives a form two blobs:
the first ("Icon") declares a range that runs past the end of `data`;
the second ("Picture") fits. It asserts the second blob's own `.frx`
offset is `0x0000`, the same offset it would carry had the first blob
never been seen.

**Failed before / passed after, in those words:** I built the test
against the fix already applied, then temporarily reverted only the
call-order inside `append_blob` (restoring `blob_cursor.take` before
the range check) and reran the single test. It failed: the assertion
message showed the second blob's own `.frx` offset had drifted to
`0x000C` instead of `0x0000`, because the first blob's phantom advance
(`declared_len + FRX_ITEM_HEADER_LEN` = `8 + 4` = `12`) had already
moved the cursor. I then restored the fix and reran the same test: it
passed, with the second blob's offset back at `0x0000`. The test
fails before the fix and passes after the fix.

The existing byte-identical assertion against the committed
`frmFire.frx` (`the_written_frx_equals_the_committed_source_byte_for_byte`,
`crates/deform6/tests/extract_tracer.rs`) still passes: this corpus
program's own blobs all fit inside `data`, so the reordering changes
nothing about its output.

---

### CR-02: A recovered argument name reached a procedure signature with no identifier-legality check

**Status:** fixed
**Files modified:** `crates/deform6/src/vb/privateobj.rs`, `crates/deform6/src/write/code.rs`
**Commit:** `bd01d86`

**Applied fix:** Marked `is_plausible_identifier` (already the check a
procedure name passes) `pub(crate)`, and changed
`write::code::format_argument` to call it against `arg.name`, falling
back to the existing `Arg<N>` placeholder whenever the name fails the
check, not only when it is empty. No new legality rule was written; the
existing one is reused, per the review's own preference.

**Test:** `an_illegal_argument_name_gets_a_generated_placeholder_not_a_raw_byte`,
added to `crates/deform6/src/write/code.rs`. It gives one procedure two
illegal argument names, `"my arg"` (a space) and `"bad\r\nname"` (a
line break), and asserts both fall back to `Arg1`/`Arg2` in the written
signature, matching the existing empty-name placeholder behavior.

---

### CR-03: `object_line` wrote a component's raw file name into an unquoted `.vbp` line with no line-break check

**Status:** fixed
**Files modified:** `crates/deform6/src/write/vbp.rs`
**Commit:** `7872ac6`

**Applied fix:** `object_line` now checks `component.file_name` for
`\r`/`\n` before building the `Object=` line, refusing (no line
written) and recording an `unrecoverable` report item naming the
reason, mirroring `write_quoted_setting`'s own existing pattern rather
than inventing a new one. `object_line`'s return type changed from
`Option<(String, ReportItem)>` to `Result<(String, ReportItem),
ReportItem>` so the caller (`write_components`) gets one clear report
item for either refusal reason (no resolved identifier, or a line
break in the file name); the "no resolved identifier" item text is
unchanged from before.

**Test:** `a_component_whose_file_name_holds_a_line_break_writes_no_object_line_and_an_item`,
added to `crates/deform6/src/write/vbp.rs`. It gives a component a
crafted file name `"MSWINSCK.OCX\r\nEvilKey=Injected"` and asserts no
`Object=` line is written, no line anywhere in the `.vbp` contains the
injected key, and a report item names the component.

---

### WR-01: `--force` overwrite could write through a pre-existing symlink

**Status:** fixed
**Files modified:** `crates/deform6-cli/src/main.rs`
**Commit:** `6c384d7`

**Applied fix:** Added `refuse_symlink_targets`, called in
`write_project` after `plan_writes` succeeds and before any byte is
written. It checks every planned path with `std::fs::symlink_metadata`
(which reads the entry itself rather than following it); if an entry
already exists there and is a symlink, the whole run refuses with
`Exit::Internal`, the same way every other containment failure already
does.

**Test:** `a_preexisting_symlink_at_a_planned_path_refuses_the_whole_run_under_force`,
added to `crates/deform6-cli/src/main.rs`. It pre-plants a symlink at
a planned output path (`Project1.vbp`) inside an otherwise empty
output directory, runs `write_project` with `force: true`, and asserts
the run refuses and the symlink's own target file is left with its
original bytes, untouched. Unix-only (`std::os::unix::fs::symlink`),
matching this crate's own existing symlink test convention in
`crates/deform6-cli/tests/cli.rs`.

---

### WR-02: `SafeName`'s identifier-legality check ran before the encoder that could turn a legal character into `?`

**Status:** fixed
**Files modified:** `crates/deform6/src/write/model.rs`
**Commit:** `b020ecc`

**Applied fix:** `is_legal_identifier_char` now rejects any character
above `U+00FF` directly (`ch.is_alphanumeric() && (ch as u32) <= 0xFF`),
so the legality guarantee holds unconditionally, rather than depending
on the invariant (enforced several modules away) that every string the
read side decodes never exceeds `U+00FF`.

**Test:** `safe_name_rejects_a_character_above_u_00ff_and_never_produces_a_question_mark`,
added to `crates/deform6/src/write/model.rs`. It calls `SafeName::new`
directly with `"\u{100}bc"` and asserts the result never contains `?`,
and that an `IllegalCharacter` fault names the offending character at
position 0.

---

### WR-03: Procedure-level uncertainty items never reached an uncertainty comment

**Status:** fixed
**Files modified:** `crates/deform6/src/write/code.rs`
**Commit:** `c2c727f`

**Applied fix:** `write_code_region` now fills the empty `path` on
every item `format_procedures` returns with its own `path_prefix`
before folding those items into the same `uncertainty_comments` call
its caller's own `items` already go through. The fill matches the
default `crate::write::merge_items` already applies to the same empty
path later, so the item's final path in the JSON report is unchanged;
only whether it also reaches a comment line changes.

**Test:** Two changes, both in `crates/deform6/src/write/code.rs`:
- `an_object_with_no_procedure_name_array_writes_a_comment_and_one_item_via_write_code_region`
  (renamed and updated from the existing
  `an_object_with_no_procedure_name_array_writes_no_lines_and_one_item_via_write_code_region`):
  this fix legitimately changes what this test expects, exactly per the
  task's own constraint 1. Before the fix, the test asserted `lines.is_empty()`
  even though an `unrecoverable` item existed for the same fact — that
  assertion demonstrated the bug, not correct behavior. The updated test
  now asserts the comment line exists and names the count, and the
  item's own path equals `path_prefix`.
- `a_public_procedure_with_no_prototype_produces_a_comment_line_naming_it`
  (new): covers the corpus-wide case named in the review, a public
  procedure with no recovered prototype (every procedure in a standard
  module, by construction), asserting a comment line names it.

The existing test `write_code_region_reaches_the_comment_emitter_and_supplies_no_comment_of_its_own`
still passes unchanged: it uses `ObjectProcedures::Slots(vec![])` (zero
procedures), so no procedure-level item exists for the fix to add a
line for.

## Skipped Issues

None. All six in-scope findings were fixed; none were rejected as
incorrect and none were skipped due to a code-context mismatch or a
failed verification.

---

_Fixed: 2026-09-13T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
