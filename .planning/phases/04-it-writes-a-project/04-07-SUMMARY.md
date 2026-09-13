---
phase: 04-it-writes-a-project
plan: 07
subsystem: write
tags: [rust, vb6, code-generation, confidence-report, uncertainty-comments]

requires:
  - phase: 04-01
    provides: "report.rs's locked shape (ReportItem, Confidence, Evidence) this plan's emitter reads"
  - phase: 04-05
    provides: "write::code::write_code_region, the one shared code region emitter this plan wires the comment emitter into"
  - phase: 04-06
    provides: "report::path_for_form / path_for_code, the path shapes this plan's tests build fixtures against"
provides:
  - "deform6::write::comment::uncertainty_comments — the one function that turns report items into comment lines, filtered by a path prefix"
  - "deform6::write::comment::MARKER — the fixed, searchable line that opens a block of these comments"
  - "the one call site: write::code::write_code_region calls uncertainty_comments itself; write_cls, write_bas and write::frm's write_form all reach it through that one function and never call it directly"
affects: [04-08, 04-09]

actuals:
  tokens: 9500
  tasks: 2
  commits: 2
plan_head_before: 5629c1420bfe01f56dc0d4e396d61fc4c60b8c38

tech-stack:
  added: []
  patterns:
    - "uncertainty_comments takes report items only, never a free string parameter: every comment line in the written output traces back to a line in the JSON report through the one item that produced it."
    - "A comment's own path match is boundary aware (path == prefix, or prefix followed by '/'), never a bare string prefix: a form named frmMain never matches a sibling form's own path, frmMain2."
    - "write_code_region is the one call site the comment emitter has in the whole phase; the project file writer and the form writer never name it, they only call write_code_region, which does."

key-files:
  modified:
    - crates/deform6/src/write/comment.rs
    - crates/deform6/src/write/code.rs
    - crates/deform6/src/write/frm.rs

key-decisions:
  - "write_code_region's own signature changed from taking a pre-built comment line list to taking the report item list and a path prefix directly, and calling uncertainty_comments itself. This was the only way to satisfy the plan's own acceptance criterion that write_code_region is the one call site the comment emitter has, while frm.rs (a caller) never names the emitter by its own identifier. Because write_code_region is a shared function all three file writers call, this necessarily touched write_form's own call site in crates/deform6/src/write/frm.rs, one file outside this plan's own files_modified list in its frontmatter. Recorded here as a Rule 3 (blocking) deviation: the crate would not compile otherwise, since a caller of a changed function signature must update its own call to match."
  - "write_form's own items accumulator (already populated with real, per-form and per-control paths by the time the code region is written, from the property loop's own collect_pending_lines) is passed straight into write_code_region as the comment emitter's own item list, with crate::report::path_for_form(&form.name) as the prefix. No new items list needed to be built or threaded through: the exact facts the plan asks a comment to name were already flowing through write_form by the time this plan started, with real paths attached, and just needed to reach the one new call."
  - "Two pre-existing frm.rs tests asserted a control omitted from its own Begin block never appears anywhere in the written file's text. That assumption breaks under this plan by design: the comment block below the Attribute lines now legitimately names the same control by its own path. Both assertions were narrowed to the header region (everything above the first Attribute line, where a Begin block still may not name the control), which is exactly what each test was originally checking."
  - "The corpus wide sweep test (write::code::sweep) is the one runtime proof of both halves of the plan's own threat register: it asserts an absence (no apostrophe-led line above the boundary of any written form, class, module or project file, across every corpus program) and a presence (at least one real program produces at least one comment below it, naming the program when it does), so a not-wired emitter and a wrongly-wired one both fail it."

patterns-established:
  - "A comment line format: an apostrophe at column zero, the item's own path, its basis with every line ending byte removed, and the byte offset of its own first evidence record in parentheses. One block per code region, opened by one fixed MARKER line, never written when the block would otherwise be empty."

requirements-completed: [RPT-06]

coverage:
  - id: D1
    description: "uncertainty_comments turns report items into comment lines: an item of confidence proven produces none, inferred and unrecoverable each produce one, a marker line opens a non-empty block, an empty result never carries a marker line alone, and every line ending byte is stripped from a recovered basis first"
    requirement: "RPT-06"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/comment.rs#tests::every_produced_line_begins_with_an_apostrophe_at_column_zero"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/comment.rs#tests::an_item_of_the_first_confidence_word_produces_no_line"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/comment.rs#tests::a_basis_holding_a_carriage_return_and_a_line_feed_produces_exactly_one_comment_line"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/comment.rs#tests::an_empty_item_list_gives_zero_lines_including_no_marker_line"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/comment.rs#tests::a_prefix_that_matches_nothing_gives_zero_lines"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/comment.rs#tests::a_prefix_that_matches_some_items_and_not_others_gives_only_the_matching_ones"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/comment.rs#tests::a_prefix_never_matches_a_sibling_form_whose_name_it_is_a_string_prefix_of"
        status: pass
    human_judgment: false
  - id: D2
    description: "write_code_region is the one call site the comment emitter has: write_cls, write_bas and write_form all reach it through that one function and none names the emitter directly; a corpus wide sweep over every written text file for every corpus program finds no comment above the boundary and at least one below it"
    requirement: "RPT-06"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#tests::write_code_region_reaches_the_comment_emitter_and_supplies_no_comment_of_its_own"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#tests::a_written_module_files_first_comment_line_comes_after_its_one_header_line"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/frm.rs#tests::a_written_forms_first_comment_line_comes_after_the_last_attribute_line"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/write/code.rs#sweep::no_written_text_file_carries_a_comment_above_its_own_boundary_and_at_least_one_program_carries_one_below_it"
        status: pass
      - kind: other
        ref: "grep -c 'uncertainty_comments' crates/deform6/src/write/vbp.rs == 0, crates/deform6/src/write/frm.rs == 0, crates/deform6/src/write/code.rs >= 1"
        status: pass
    human_judgment: false
  - id: D3
    description: "Manual by-eye read of one real written form (FrmHex, from corpus/public-domain/HexScroll/Hex Scroll.exe): the Begin block carries zero comments, the five Attribute lines follow End directly, the marker line opens immediately after Attribute VB_Exposed with no blank line, and every comment line names a real path, a plain basis and a byte offset"
    verification: []
    human_judgment: true
    rationale: "The plan's own <verification> step asks for a by-eye read of a real written form; performed this session by temporarily printing the full written text for the first corpus form that earned a comment (removed before commit) and reading it against FILE-FORMATS.md section 2.6 and 2.7's own rules. A human should re-read that output once report::build and the full writers are wired into the shipped extract command (plan 04-08) to confirm nothing drifted."

duration: 1 session
completed: 2026-09-13
status: complete
---

# Phase 4 Plan 7: The uncertainty comment emitter, and its one legal call site, Summary

**`write::comment::uncertainty_comments` turns report items into apostrophe comment lines that can enter the written output through exactly one place, `write::code::write_code_region`, proven by a corpus wide sweep that finds no comment above the boundary of any written form, class, module or project file and at least one below it.**

## Performance

- **Duration:** 1 session
- **Completed:** 2026-09-13
- **Tasks:** 2 of 2
- **Files modified:** 3 (`crates/deform6/src/write/comment.rs`, `crates/deform6/src/write/code.rs`, `crates/deform6/src/write/frm.rs`)

## Accomplishments

- `uncertainty_comments(items, prefix)` is the one function that turns a `ReportItem` list into comment lines: an item of `Confidence::Proven` produces none; `Inferred` and `Unrecoverable` each produce one line naming the item's own path, its basis with every line ending byte stripped out, and the byte offset of its own first evidence record. A non-empty block opens with one fixed `MARKER` line; an empty result never carries the marker alone.
- The path match is boundary aware (`path == prefix` or `prefix` followed by `/`), so a form named `frmMain` never accidentally picks up a sibling form's own facts, `frmMain2`.
- `write_code_region` (in `write::code`) now calls `uncertainty_comments` itself, given the caller's own item list and path prefix, and is the one call site the comment emitter has in the whole phase. `write_cls` and `write_bas` pass their own items and prefix straight through.
- `write_form`'s own call to `write_code_region` (in `write::frm`) now passes the items that call already collected for this form, each already carrying a real path from `control_report_path`, and this form's own prefix from `crate::report::path_for_form`. `frm.rs` never names the comment emitter itself; it only calls `write_code_region`.
- A corpus wide sweep test walks every corpus executable, runs the full write path (`write_vbp`, `write_form`, `write_cls`, `write_bas`) over each one, and asserts no line above the boundary of any written file begins with an apostrophe as its first non space character, and that at least one real program (named in the failure message) produces at least one comment below it. This proves both halves of the threat register: the emitter reaches nowhere illegal, and it is genuinely wired, not merely silent because nothing calls it.
- A manual by-eye read of one real written form (`FrmHex`, `corpus/public-domain/HexScroll/Hex Scroll.exe`) confirms the exact shape FILE-FORMATS.md sections 2.6 and 2.7 describe: zero comments inside the `Begin` block, the five `Attribute` lines directly after `End`, and the comment block opening on the very next line with no blank line between.

## Task Commits

Each task was committed atomically (both carry `tdd="true"`; see "TDD Gate Compliance" below):

1. **Task 1: The emitter, generated from report items and from nothing else** — `1371eba` (feat)
2. **Task 2: One call site, in the one legal place, proved by sweeping the written output** — `521df37` (feat)

**Plan metadata:** commit pending (this SUMMARY, STATE.md, ROADMAP.md, REQUIREMENTS.md)

## Files Created/Modified

- `crates/deform6/src/write/comment.rs` — `uncertainty_comments`, `MARKER`, `path_is_under`, `comment_line`, `strip_line_endings`, and 9 tests (was a module doc comment stub left by plan 04-01)
- `crates/deform6/src/write/code.rs` — `write_code_region`, `write_cls`, `write_bas` widened to take a report item list and a path prefix instead of a pre-built comment line list; existing tests updated to match; a new `mod sweep` added with the corpus wide proof
- `crates/deform6/src/write/frm.rs` — `write_form`'s own call to `write_code_region` updated to pass its own items and the form's own path prefix; two pre-existing tests narrowed to the header region for the reason in Deviations below; one new test asserting a written form's first comment line comes after its last Attribute line

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential:

1. **`write_code_region`'s own signature changed to call the comment emitter itself**, rather than taking a pre-built comment line list from its caller. This is the only way the plan's own acceptance criteria could all hold at once: `write_code_region` reaches `uncertainty_comments`, and neither `write::vbp` nor `write::frm` ever names that function directly, since both only ever call `write_code_region`.
2. **`write_form`'s own already-accumulated `items` list is exactly the right input** for its own call to the comment emitter. No new bookkeeping was needed: the property loop already builds real, per-form and per-control paths onto every item before the code region is reached, so passing that same list straight through gives the comment emitter everything the plan's own behavior text asks for, with zero new plumbing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `write_form`'s own call site in `crates/deform6/src/write/frm.rs` needed updating for the crate to compile**
- **Found during:** Task 2, after widening `write_code_region`'s own signature
- **Issue:** This plan's own frontmatter names only `crates/deform6/src/write/comment.rs` and `crates/deform6/src/write/code.rs` in `files_modified`, but `write::frm::write_form` is an existing caller of `write_code_region`. Changing that function's signature without updating its one other caller is a compile error, not a choice.
- **Fix:** Updated the one call site (line ~1065) to pass `write_form`'s own `items` list and `crate::report::path_for_form(&form.name)` as the prefix, matching the new signature. No other line in `frm.rs` names `uncertainty_comments` itself, satisfying the plan's own acceptance criterion that the form writer has no comment call of its own.
- **Files modified:** `crates/deform6/src/write/frm.rs`
- **Verification:** `cargo build -p deform6 --tests` succeeds; `grep -c 'uncertainty_comments' crates/deform6/src/write/frm.rs` is `0`
- **Committed in:** `521df37` (Task 2 commit)

**2. [Rule 1 - Bug] Two pre-existing `frm.rs` tests asserted a control's own name never appears anywhere in the written file, which the plan's own design now contradicts on purpose**
- **Found during:** Task 2, running `cargo test -p deform6 --lib write::frm` after wiring the call site
- **Issue:** `an_unknown_control_kind_writes_no_block_and_names_the_raw_type_value` and `a_control_below_depth_seven_is_omitted_with_one_report_item_each` each asserted the omitted control's own name (`Weird1`, `Frame8`) never appears anywhere in the whole written `.frm` text. Both control omissions earn a report item whose own path names the control, and that path now legitimately reaches a comment line below the Attribute block, which is exactly this plan's own purpose.
- **Fix:** Narrowed both assertions to the header region only (`&text[..boundary]`, where `boundary` is the index of the first `Attribute VB_Name` line): the real intent of both tests, that no `Begin` block names the omitted control, still holds and is still checked; the new, legitimate appearance in the comment block below is no longer mistaken for a regression.
- **Files modified:** `crates/deform6/src/write/frm.rs`
- **Verification:** `cargo test -p deform6 --lib write::frm` passes, both tests green
- **Committed in:** `521df37` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking compile fix, 1 test assumption corrected by the plan's own design). **Impact:** Both were necessary consequences of wiring the one call site the plan itself specifies; neither reflects scope creep, and both are narrowly scoped to the one function signature this plan changed.

## Issues Encountered

None beyond the deviations above, all resolved.

## User Setup Required

None — no external service configuration required.

## Known Stubs

- `write::project` (`crates/deform6/src/write/mod.rs`) still calls the thin writers (`write_vbp_thin`, `write_form_thin`, `write_cls_thin`, `write_bas_thin`), none of which reach `write_code_region` at all. The comment emitter this plan builds and wires is exercised directly by this plan's own tests and by the corpus wide sweep, but the shipped `deform6 extract` command does not yet write a single comment, because it does not yet call the full writers this plan (and plans 04-02 through 04-06) built. This is the same staged pattern every prior wave-2/3 plan in this phase left behind, restated here so a reader of this SUMMARY alone knows the emitter exists without yet being load-bearing in the shipped command. Plan 04-08 wires the full writers, `report::build`, and this comment path together.

## TDD Gate Compliance

Both tasks carry `tdd="true"`. RED was genuinely run and observed for each, against a reverted stub of the task's own new logic, before the real implementation was restored. Neither RED state was committed separately, per this project's own `AGENTS.md`: the gate runs `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` before every commit, with no exception, and code and its tests share one commit. A committed RED state would fail the first rule and violate the second.

| Task | RED observed | GREEN commit | REFACTOR | Status |
|------|--------------|--------------|----------|--------|
| 1 (the emitter) | `uncertainty_comments` reverted to a stub always returning an empty `Vec`: 5 of 9 `write::comment` tests failed, each on an empty-list assertion where a real comment line was expected | `1371eba` | none needed | Pass |
| 2 (the one call site) | `write_code_region` reverted to ignore its own `items`/`path_prefix` parameters and never call `uncertainty_comments`: 5 tests failed across `write::code` and `write::frm`, including the corpus sweep's own positive count assertion (`no corpus program produced even one comment line below the boundary`) | `521df37` | none needed | Pass |

Both RED runs are documented verbatim in their own `feat(04-07)` commit messages, and both were genuinely executed this session (reverted, run, observed, restored), not merely asserted after the fact.

## Next Phase Readiness

- Plan 04-08 (`Command::Extract`'s full wiring) has a tested `uncertainty_comments`, a `write_code_region` that already reaches it, and full `write_vbp`/`write_form`/`write_cls`/`write_bas` writers (plans 04-02 through 04-07) ready to switch `write::project` over to, closing the "thin vs. full" gap every prior plan in this phase left staged, including this plan's own zero-comments-in-the-shipped-command gap.
- Plan 04-09 (the structural check) can rely on the corpus wide sweep's own boundary logic (the first `Attribute` line for a form or a class, the header line for a module, no boundary at all for a `.vbp`) as a second, independently written proof that the same rule holds once the full writers are wired into the shipped command.
- No blocker for wave 3's own remaining plan or for wave 4. `RPT-06` is not shared with any sibling plan's own `requirements:` frontmatter field in this phase (confirmed directly against every other `04-*-PLAN.md`), so it is marked complete now.

---
*Phase: 04-it-writes-a-project*
*Completed: 2026-09-13*

## Self-Check: PASSED

- `crates/deform6/src/write/comment.rs` exists on disk and defines `pub fn uncertainty_comments`, `pub const MARKER`.
- `crates/deform6/src/write/code.rs` exists on disk and defines `pub fn write_code_region(items: &[ReportItem], path_prefix: &str, procedures: &ObjectProcedures)`.
- Commits `1371eba` and `521df37` both exist in `git log --oneline --all`.
- `cargo test -p deform6 --lib write::comment` reports 9 passing tests.
- `cargo test -p deform6 --lib write::code` reports 32 passing tests (at or above the plan's own 30 test floor).
- `cargo test -p deform6 --lib write::frm` reports 40 passing tests, including the new form-comment-position test.
- `cargo test -p deform6 --test extract_tracer` passes (11 tests); the written `frmFire.frx` stays byte identical to the committed corpus source.
- Source greps: `uncertainty_comments` count is `2` in `code.rs`, `0` in `vbp.rs`, `0` in `frm.rs`; `MARKER` present in `comment.rs`; the free-text-entry-point grep (`pub fn [a-z_]+\((text|message|note): ?&str`) is `0` in `comment.rs`.
- The full gate (`cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`) passes: 598 tests in the `deform6` lib target alone, 0 failures across the whole workspace.
- The pre-existing staged deletion (`.planning/phases/01-it-reads-the-file/VERIFICATION.md`), the `.planning/config.json` modification, and the untracked `.gsd/`, `Notes/`, `.planning/milestone.lock` paths are untouched by either of this plan's two commits (`git status --short` before and after this plan's commits shows the identical set).
