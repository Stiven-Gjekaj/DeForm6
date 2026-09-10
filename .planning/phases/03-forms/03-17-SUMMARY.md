---
phase: 03-forms
plan: 17
subsystem: forms
tags: [vb6, refusal, memory-leak, ratio-format, text-style, code-review-gap, gap-closure]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "03-REVIEW.md's own three open findings (WR-01, WR-03, IN-01); plan 03-14's controltree.rs, plan 03-15's frx.rs, and plan 03-16's error.rs, the current state of every file this plan touches"
provides:
  - "crate::error::damaged, the one shared pub(crate) helper for a Refusal::Damaged whose message names a run-time value, replacing the three byte for byte identical private copies review finding WR-01 named"
  - "format_ratio's zero-denominator guard: a declared count of zero gives the named text n/a instead of the literal text NaN a bare f64 division would write into the pin file (review finding WR-03)"
  - "controltree.rs with zero em-dashes, matching every other file this gap run has touched (review finding IN-01)"
affects: []

actuals:
  tokens: 4846
  tasks: 3
  commits: 3
plan_head_before: 7c5ca35c28ed5965d0b1e582ca088c28a33d1f55

tech-stack:
  added: []
  patterns:
    - "A helper that leaks memory to satisfy a locked, crate-wide &'static str constraint lives in exactly one place, next to the type it serves (error.rs, next to Refusal), and states its own memory cost in its own doc comment rather than leaving that cost to be rediscovered independently by every module that needs the same escape hatch."
    - "A denominator read from a corpus that has never produced zero is still guarded before the division that assumes it is non-zero, because the guard's job is to keep a future corpus program from silently writing a value that reads like a measurement and is not one, not to handle a case today's 44 programs can reach."

key-files:
  created: []
  modified:
    - crates/deform6/src/error.rs
    - crates/deform6/src/vb/gui.rs
    - crates/deform6/src/vb/controltree.rs
    - crates/deform6/src/vb/frx.rs
    - crates/deform6/tests/ratios.rs

key-decisions:
  - "Extracted one shared helper (crate::error::damaged) rather than widening Refusal::Damaged to carry an owned String. Measured before choosing: 145 lines mention Refusal::Damaged across 13 files (108 of those lines are an actual Refusal::Damaged(...) construction or destructuring pattern), and three files (gui.rs, controltree.rs, frx.rs) each carried a byte for byte identical private copy of the leaking helper. Extracting the shared helper touches 4 files; widening the variant to String would touch every one of the 108 construction/pattern sites. The review's own stated fix is the extraction, and its own stated reason (a later widening decision then touches one call site instead of three) still holds after this session's own remeasurement. The leak itself is not removed, only centralised; the helper's own doc comment states the memory cost plainly so a later reader deciding whether to widen the variant does not have to rediscover it."
  - "format_ratio's zero-denominator guard returns the literal text \"n/a\", one of the two fixes the review names (the other, refusing the write outright, was rejected because a whole-program write refusal over one ratio field is a larger blast radius than the one field the guard needs to protect). tests/ratios.rs's own hand-rolled TOML reader stores `ratio` as raw text (`ratio_text: String`, parsed with `value.to_owned()`, no numeric parse), so `ratio = n/a` round-trips through the committed pin file exactly like any other ratio text would, with no format change needed elsewhere."
  - "REQUIREMENTS.md is left unchanged. FRM-01 and FRM-05 are both still [ ] after this plan, matching this plan's own must_haves prohibition (\"No behaviour change... This plan changes how the code is written and not what it reports\") and the precedent 03-15-SUMMARY.md and 03-16-SUMMARY.md both set: a plan that closes a code review finding without recovering a new value or writing a new file does not mark the requirement its own PLAN.md frontmatter lists as complete, because doing so would repeat the exact requirements-tracking overstatement 03-VERIFICATION.md already found and named as a problem in its own right."

patterns-established: []

requirements-completed: []
# This plan's own PLAN.md frontmatter lists FRM-01 and FRM-05 under
# `requirements`, but neither is completed here. Every one of this plan's
# own must_haves truths and prohibitions states the opposite of completion:
# "No recovered tree, no recovered property and no pinned count changes.
# This plan changes how the code is written and not what it reports." A
# refactor of the damaged() helper, a format guard on a test-only ratio
# formatter, and a text-only em-dash sweep touch no code path that recovers
# a form, a control, or a resource blob. Marking either requirement [x] here
# would be the same overstatement 03-VERIFICATION.md found and 03-15/03-16
# both explicitly declined to repeat.

coverage:
  - id: D1
    description: "crate::error::damaged is the one shared pub(crate) helper for a Refusal::Damaged whose message names a run-time value; gui.rs, controltree.rs and frx.rs call it and none keeps a private copy; the helper's own doc comment states its memory cost; every refusal message these three modules produce is unchanged, proved by the existing test suite and by a deliberate break-and-restore"
    requirement: "N/A (code review finding WR-01, not a REQUIREMENTS.md ID)"
    verification:
      - kind: other
        ref: "grep -rc 'fn damaged' crates/deform6/src/vb | awk -F: '{s+=$2} END {print s+0}' (gives 0)"
        status: pass
      - kind: other
        ref: "cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml (gives no diff)"
        status: pass
      - kind: other
        ref: "reverting crate::error::damaged to a fixed placeholder string once and running cargo test -p deform6 --lib: 8 of 410 tests fail (vb::gui and vb::controltree's own message-content assertions), proving those messages are asserted, not merely produced"
        status: pass
    human_judgment: false
  - id: D2
    description: "format_ratio refuses to divide by a declared count of zero and gives the named text n/a instead; every declared count above zero gives exactly the text it gives today; the committed pin file is proved byte for byte unchanged"
    requirement: "N/A (code review finding WR-03, not a REQUIREMENTS.md ID)"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/ratios.rs#a_declared_of_zero_gives_the_named_result_and_not_a_division"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/ratios.rs#a_declared_above_zero_gives_the_same_text_it_gives_today"
        status: pass
      - kind: other
        ref: "cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml (gives no diff)"
        status: pass
      - kind: other
        ref: "removing the guard once and running the new zero-denominator test: the unguarded division gives the literal text NaN, captured in this SUMMARY's own break-on-purpose section"
        status: pass
    human_judgment: false
  - id: D3
    description: "controltree.rs holds zero em-dashes; a fresh sweep of the whole crates/ and tests/ tree (not only the one file the review named) finds the same: zero, everywhere"
    requirement: "N/A (code review finding IN-01, not a REQUIREMENTS.md ID)"
    verification:
      - kind: other
        ref: "! grep -rq $'\\xe2\\x80\\x94' crates tests"
        status: pass
      - kind: other
        ref: "git diff --stat -- crates/deform6/src/vb/controltree.rs shows only doc-comment lines (/// or //!) changed; no executable statement and no string literal moved"
        status: pass
    human_judgment: false

duration: single session
completed: 2026-09-11
status: complete
---

# Phase 3 Plan 17: The Remaining Code Review Findings Summary

**One shared `crate::error::damaged` helper replaces three byte for byte identical leaking copies (review finding WR-01); `format_ratio` refuses a zero denominator and gives `"n/a"` instead of a bare division's literal `"NaN"` (WR-03); and `controltree.rs`'s 22 measured em-dashes (not the 10 the review counted before four later plans wrote more prose) are gone (IN-01) — with every recovered value, every pinned ratio, and every existing refusal message proved unchanged.**

## Performance

- **Duration:** single session
- **Tasks:** 3
- **Files modified:** 5 (0 created, 5 modified)

## Accomplishments

- **WR-01 closed.** Counted first, as the plan's own action text requires: 145 lines mention `Refusal::Damaged` across 13 files (108 of those are an actual construction or destructuring pattern, not a doc comment), and three files (`gui.rs`, `controltree.rs`, `frx.rs`) each carried its own byte for byte identical private `damaged()` helper. `crate::error::damaged` (`pub(crate)`, next to `Refusal` itself) is now the one copy; the three files call it. After: 139 mentions across 12 files, 106 construction/pattern sites — a net reduction of 2 construction sites (three duplicate copies removed, one shared copy added), and 0 files under `crates/deform6/src/vb` carry their own `fn damaged` (measured: `grep -rc 'fn damaged' crates/deform6/src/vb` sums to 0). The chosen fix (extraction, not widening `Refusal::Damaged` to an owned `String`) touches 4 files; widening would have touched all 108 construction/pattern sites. The leak itself is not removed — `Box::leak` is still there — only centralised; the helper's own doc comment states the memory cost this session measured: a single command-line run leaks one short string harmlessly, but a long-running host that calls this library in a loop over hostile files (a fuzz target, or a batch scanner — the class of host phase 5 owns) grows the leak without bound, one string per refusal that reaches the function.
- **WR-03 closed.** `format_ratio` now returns the literal text `"n/a"` when `declared` is `0`, before the division that would otherwise run. No corpus program among the 44 declares zero of anything today, so a dedicated test drives the branch directly rather than waiting for the corpus to grow one, and a second test pins two real corpus values (`UUID2.exe`, 4/25 → `"0.16"`; `LockWorkStation.exe`, 0/1 → `"0.00"`) to prove every non-zero denominator still gives exactly the text it gives today. `tests/ratios.rs`'s own hand-rolled TOML reader already stores `ratio` as raw text, so `"n/a"` needs no new parsing path anywhere the pin file is read.
- **IN-01 closed, and re-measured rather than trusted from the review.** The review counted ten em-dashes in `controltree.rs` at review time; plans 03-12 through 03-16 wrote new prose into the file since, and this session's own sweep found 22. All 22 are gone, replaced with a period, a comma, or a parenthesis, matching the plain style the file already uses elsewhere. A whole-tree sweep (`crates/` and `tests/`, not only the one named file) confirms zero remain anywhere in the repository.
- **No behaviour changed anywhere.** `cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml` was run, and passed, after every one of the three tasks, not only at the end. The 52 of 53 forms and 686 of 686 controls plan 03-14 pinned are unchanged; the 44-program totals (185 recovered, 904 declared) are unchanged.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: One helper for a refusal that names a run-time value** - `43d8b53` (refactor, tdd="true")
2. **Task 2: A ratio format that refuses a zero denominator** - `7cb79ff` (fix, tdd="true")
3. **Task 3: Remove the em-dashes from the source tree** - `81583d0` (docs, no source file's meaning changed)

**Plan metadata:** commit follows this SUMMARY.

_Note: all three tasks carry (or, for task 3, imply) `AGENTS.md`'s own "code and its tests in the same commit" rule, binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase. Task 1 and task 2's own RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task. Task 3 changes no test and no executable statement, so it carries no RED phase of its own; its own required verification is the whole-tree grep and the unchanged pin file, both run after the change._

## Files Created/Modified

- `crates/deform6/src/error.rs` - new `pub(crate) fn damaged`, next to `Refusal`, with a doc comment stating its own memory cost and the reason the leak is centralised rather than removed
- `crates/deform6/src/vb/gui.rs` - private `fn damaged` removed; imports `crate::error::damaged`
- `crates/deform6/src/vb/controltree.rs` - private `fn damaged` removed; imports `crate::error::damaged`; 22 em-dashes removed from doc comments (comment-only change)
- `crates/deform6/src/vb/frx.rs` - private `fn damaged` removed; imports `crate::error::damaged`
- `crates/deform6/tests/ratios.rs` - `format_ratio` guards a zero `declared`; two new tests

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The most consequential: the shared helper is extracted rather than the `Refusal::Damaged` variant being widened to an owned `String`, because the extraction touches 4 files against the 108 construction/pattern sites a widening would touch, and the review's own stated reasoning for preferring extraction (a later widening decision then touches one call site) still holds after this session's own remeasurement.

## Deviations from Plan

None. The plan's own three tasks map directly to the three review findings, each in its own commit, and no auto-fix, blocker, or architectural question arose beyond what the tasks themselves already named as the byte counts to remeasure (145 mentions/108 construction sites instead of the plan's own "143" lead, and 22 em-dashes instead of the review's own "ten").

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's shared helper** — `crate::error::damaged` was changed to ignore its argument and always return a fixed placeholder string (`Refusal::Damaged("RED-PHASE DELIBERATE BREAKAGE, no run-time value carried")`). `cargo test -p deform6 --lib` run once, against the whole library:

```
test vb::controltree::tests::a_scope_run_of_sixty_five_non_terminating_bytes_refuses_and_names_the_offset ... FAILED
test vb::controltree::tests::a_pop_count_that_would_empty_the_stack_still_refuses_and_names_the_offset ... FAILED
test vb::controltree::tests::a_control_block_with_a_length_of_zero_refuses_and_names_the_offset ... FAILED
test vb::gui::tests::account_refuses_on_overflow_and_names_the_running_total ... FAILED
test vb::gui::tests::a_properties_length_larger_than_the_file_is_refused ... FAILED
test vb::gui::tests::an_under_consuming_walk_is_refused_and_the_message_differs_from_over_consumption ... FAILED
test vb::gui::tests::a_gui_table_entry_with_the_wrong_lstructsize_is_refused_and_names_its_offset ... FAILED
test vb::gui::tests::the_same_spans_against_a_declared_length_of_31_report_one_unaccounted_byte ... FAILED

test result: FAILED. 402 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Eight tests across `gui.rs` and `controltree.rs` fail on the missing byte offset text their own assertions require, proving those refusal messages are actually asserted by the existing suite and not merely produced. (`frx.rs`'s own one `damaged()`-produced test, `take_gives_a_refusal_naming_the_offset_when_the_advance_overflows_a_u32`, only asserts the message is non-empty, a pre-existing gap this task's own scope does not extend to closing.) Reverted to the real helper before committing `43d8b53`.

**Task 2's zero-denominator guard** — the guard's own early return was removed, leaving the bare division. `cargo test -p deform6 --test ratios a_declared_of_zero_gives_the_named_result_and_not_a_division` run once:

```
thread 'a_declared_of_zero_gives_the_named_result_and_not_a_division' panicked at crates/deform6/tests/ratios.rs:703:5:
assertion `left == right` failed: a zero denominator must give a result a reader cannot mistake for a measurement
  left: "NaN"
 right: "n/a"

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 39 filtered out; finished in 0.00s
```

The unguarded division gives the literal text `"NaN"`, exactly the degenerate value review finding WR-03 named. Reverted to the guarded implementation before committing `7cb79ff`.

## Issues Encountered

None beyond the deviations section above (empty).

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every symbol this plan's own artifact table names (`crate::error::damaged`, `format_ratio`'s guard) is implemented and wired into the production path this plan's own three tasks describe; none is a placeholder.

## Threat Flags

None beyond what this plan's own `<threat_model>` already names and mitigates: T-03-89 (the unbounded leak) — the leak sites are counted before and after (3 private copies → 1 shared copy) and the accepted cost is written in the helper's own doc comment and in this SUMMARY; T-03-90 (a degenerate ratio written as if measured) — the denominator is guarded before the division, and a test drives the zero case directly; T-03-91 (a refactor changing a reported value) — every task's own verify step ran the ratio update command and asserted the committed pin file unchanged, and it stayed unchanged after all three tasks; T-03-92 (a refusal message changed with nobody noticing) — the break-on-purpose evidence above is exactly this proof, for both Task 1 and Task 2.

## Next Phase Readiness

- This is the last plan of phase 3's gap closure run. All three open code review findings (WR-01, WR-03, IN-01) are closed; the review's own two other findings (WR-02, closed by plan 03-14; IN-02, an acknowledged low-priority dead-code arm the review itself marked optional and this gap run's own scope never named) are accounted for.
- `tests/ratios.toml` is unchanged by this whole plan, confirmed by a fresh `xtask update-ratios` run after every task, not only at the end: 52 of 53 forms, 686 of 686 controls, 185 of 904 procedures, all as plan 03-14 left them.
- FRM-01 and FRM-05 remain `[ ]` in `REQUIREMENTS.md`, unchanged by this plan, per this plan's own must_haves and the precedent 03-15 and 03-16 both set.
- No blockers for whatever plan or phase follows phase 3's own gap closure wave.

---
*Phase: 03-forms*
*Completed: 2026-09-11*

## Self-Check: PASSED

All 5 modified files (`crates/deform6/src/error.rs`, `crates/deform6/src/vb/gui.rs`, `crates/deform6/src/vb/controltree.rs`, `crates/deform6/src/vb/frx.rs`, `crates/deform6/tests/ratios.rs`) found on disk. All 3 task commits (`43d8b53`, `7cb79ff`, `81583d0`) found in `git log`. `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` all pass clean. `cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml` gives no diff. `grep -rc 'fn damaged' crates/deform6/src/vb` sums to `0`. `! grep -rq` for the em-dash byte sequence against the whole of `crates` and `tests` passes.
