---
status: clean
iteration: 2
critical: 0
warning: 0
info: 1
files_reviewed: 6
files_reviewed_list:
  - crates/deform6/src/write/frm.rs
  - crates/deform6/src/write/code.rs
  - crates/deform6/src/write/vbp.rs
  - crates/deform6/src/write/model.rs
  - crates/deform6/src/vb/privateobj.rs
  - crates/deform6-cli/src/main.rs
depth: standard
---

# Phase 4 Code Review: It Writes a Project (Iteration 2, re-review after fixes)

**Reviewed:** 2026-09-13
**Depth:** standard
**Files Reviewed:** 6 (the files the six fix commits touched)
**Status:** clean

## Summary

This is a targeted re-review of the six fix commits (`1e20c6e`, `bd01d86`,
`7872ac6`, `6c384d7`, `b020ecc`, `c2c727f`) written against the six
Critical/Warning findings from iteration 1. All six are closed. None of
the six introduces a new Critical or Warning defect.

CR-01 was already confirmed closed before this review started (revert-
and-observe cycle run by the orchestrator); this review did not re-derive
it and treats it as closed.

For CR-02, CR-03, WR-01, and WR-02 I read each diff, traced the changed
function against its callers, ran the specific regression test each
commit added, and ran the full gate (`cargo fmt --all --check`, `cargo
clippy --all-targets -- -D warnings`, `cargo test --workspace`, 872
tests, 0 failures). For the two findings the task flagged as needing
corpus verification rather than reasoning:

- **bd01d86** (argument-name legality) and **b020ecc** (`SafeName`
  character bound) both tighten an identifier-legality check to ASCII- or
  Latin-1-only. I grepped every `.bas`/`.cls`/`.frm` file in `corpus/` for
  a procedure argument list containing a non-ASCII byte and found none.
  Combined with the fact that `char::from(byte)` is the crate's universal
  read-side decoding convention (which can never produce a code point
  above `U+00FF` from a real executable), neither tightened check can
  reject a name that legitimately appears in the 44-program corpus. The
  differential corpus tests (`every_declared_public_procedure_is_
  recovered_...`, `every_declared_form_and_control_is_recovered_...`)
  still pass, which is consistent with this.
- **c2c727f**'s renamed test: I read `format_procedure_entry` and
  `format_procedures` directly. Every procedure-level `ReportItem` they
  build carries an empty `path` by construction (the function knows only
  the procedure, never the object). Before the fix, that empty path never
  matched `uncertainty_comments`'s own `path_is_under` filter, so the old
  assertion (`lines.is_empty()`) was asserting the absence of a comment
  that the same call's own `items` parameter already proved should exist
  (the returned `ReportItem` names the same fact). The old assertion
  demonstrated the bug; the fixer's characterization is accurate. The new
  fill (`item.path = path_prefix` when empty) exactly matches the default
  `write::mod::merge_items` already applies to the same empty path later,
  so the item's final JSON-report path is provably unchanged; only
  whether it also reaches a comment line changes.

I also checked the two specific regression questions the task named
directly:

- **6c384d7** (symlink refusal): `refuse_symlink_targets` checks with
  `symlink_metadata`, which is false for a plain file, so a legitimate
  `--force` overwrite of a regular file is untouched by the new check.
  Confirmed by running the existing (unmodified)
  `a_second_run_into_a_populated_directory_refuses_without_force_and_
  succeeds_with_it` test, which still passes.
- **7872ac6** (`.vbp` line-break guard): the tracer program's own
  component has no line break in its file name, so `object_line` takes
  its original path. Confirmed by running
  `the_written_vbp_holds_type_exe_first_one_form_one_class_and_a_
  matching_startup`, which still passes unchanged.

IN-01 (component report path built from a raw string, `write/vbp.rs:189,
241`) remains open by design; it was deliberately left unfixed and stays
Info per the task's instruction.

## Findings from Iteration 1: Current State

| ID | Title | State |
|----|-------|-------|
| CR-01 | `append_blob` advances `BlobCursor` before the range check | Closed (verified by orchestrator; bound check now runs before `blob_cursor.take`) |
| CR-02 | Recovered argument name has no identifier-legality check | Closed (reuses `is_plausible_identifier`, falls back to `Arg<N>`; test `an_illegal_argument_name_gets_a_generated_placeholder_not_a_raw_byte`) |
| CR-03 | `object_line` writes a raw file name with no line-break check | Closed (mirrors `write_quoted_setting`'s guard, refuses and records a report item; test `a_component_whose_file_name_holds_a_line_break_writes_no_object_line_and_an_item`) |
| WR-01 | `--force` can write through a pre-existing symlink | Closed (`refuse_symlink_targets` checked with `symlink_metadata` before any byte is written; test `a_preexisting_symlink_at_a_planned_path_refuses_the_whole_run_under_force`) |
| WR-02 | `SafeName`'s legality check runs before the `?`-producing encoder | Closed (`is_legal_identifier_char` now rejects anything above `U+00FF` directly; test `safe_name_rejects_a_character_above_u_00ff_and_never_produces_a_question_mark`) |
| WR-03 | Procedure-level uncertainty items never reach a comment | Closed (`write_code_region` fills the empty path with `path_prefix` before its `uncertainty_comments` call; tests `an_object_with_no_procedure_name_array_writes_a_comment_and_one_item_via_write_code_region`, `a_public_procedure_with_no_prototype_produces_a_comment_line_naming_it`) |
| IN-01 | Component report path built from a raw string, not `SafeName` | Out of scope, carried forward as Info, unchanged |

## Info

### IN-01: A component's report path is built from a raw recovered string, not a `SafeName`

**File:** `crates/deform6/src/write/vbp.rs:189, 241` (`format!("/components/{}", component.name)`)

**Issue:** Unchanged from iteration 1. Every other path this phase's
report items carry is built through `crate::report::path_for_form`/
`path_for_control`/`path_for_code`, each taking a `&SafeName`. A
component's own report path is built directly from `component.name`, a
raw `String` that can contain a `/` or other structurally significant
character. This path never reaches the file system, only the JSON
report, so the risk is low; it is inconsistent with the rest of this
file's own design rule.

**Fix:** Route `component.name` through a lightweight `SafeName`-style
sanitisation (even one that only strips `/` and control characters)
before it is embedded in a report path.

## New Findings from the Six Fix Commits

None. Each of the six fixes was traced to its caller, checked against
the codebase's existing convention for that class of defect, covered by
a specific regression test, and re-run against the full gate and (where
relevant) the corpus. No new Critical, Warning, or Info issue was found
in the changed code.

---

_Reviewed: 2026-09-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_Iteration: 2_
