---
phase: 05-hostility
plan: 05
subsystem: testing
tags: [rust, cargo-test, fuzzing, regression-testing, pe-format]

requires:
  - phase: 05-hostility
    provides: "plan 05-01's Mode::Strict/Mode::Salvage split, plan 05-02's ImplausibleCount bound check on the GUI table"
provides:
  - "support::hostile::gui_table_overcount_4k, a hostile PE image built from literals this repository owns"
  - "crates/deform6/tests/regressions.rs, the stable replay of every committed regression input in both modes"
  - "crates/deform6/tests/regressions/gui-table-overcount-4k.bin, the first committed regression input"
  - "the written crash-to-test procedure, as the module doc comment of regressions.rs"
affects: [05-04, 05-08]

actuals:
  tokens: 4555
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "A hostile fixture built from literals in a test file, never include_bytes! and never a corpus path, so a fixture may be committed without breaching the third-party-work rule in AGENTS.md."
    - "A regression replay that asserts nothing about the result variant, only that the process is still running, because every input is known bad and a refusal is a correct answer for most of them."

key-files:
  created:
    - crates/deform6/tests/support/hostile.rs
    - crates/deform6/tests/regressions.rs
    - crates/deform6/tests/regressions/gui-table-overcount-4k.bin
  modified:
    - crates/deform6/tests/support/mod.rs

key-decisions:
  - "The checkpoint decision (seed-with-an-owned-binary vs. text-seed vs. no-seed) was put to the human. The human chose seed-with-an-owned-binary: commit the synthetic PE seed now, in the same commit as the harness, so the count assertion is true from its first commit and the gate never goes red for a reason nobody chose."
  - "The seed's module doc comment states its measured, narrower scope rather than the plan's own framing: deform6::inspect refuses gui-table-overcount-4k.bin at runtime_of with Refusal::NoVbRuntime, in both modes, before the read reaches header_region or GuiTable::walk. The seed guards the earliest refusal path, not the GUI table bound check. That check is already covered by gui_table_refuses_an_implausible_form_count in crates/deform6/src/vb/gui.rs (plan 05-02), which is named directly in the doc comment so a later reader does not mistake this fixture for that test."
  - "hostile.rs does not embed a VBHeader in the image, matching what plan 05-02's own fixture did: it built a VbHeader value in memory and called GuiTable::walk directly, rather than writing header bytes into the image. The task 1 action steps never named a VB5! magic or an entry-point stub either, only PE/COFF/section fields plus the one GUI table entry."
  - "The directory walk in regressions.rs is a hand-copy of corpus_sweep.rs's recursive read_dir shape with the extension filter dropped, not a literal cross-crate call: each file under tests/ is its own crate root, and corpus_sweep.rs is out of this plan's files_modified scope, so the two walkers cannot literally share code without editing a file this plan does not own."

patterns-established:
  - "A crash-to-test procedure written as the module doc comment at the top of the replay file, numbered steps, so the next person finds it where they look."

requirements-completed: []

coverage:
  - id: D1
    description: "A hostile GUI-table-shaped PE image built from literals this repository owns, with three unit tests (exact length, determinism, PE signature) proving its own shape"
    requirement: "SAF-01"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/support/hostile.rs#support::hostile::tests::the_image_is_exactly_4096_bytes"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support/hostile.rs#support::hostile::tests::two_calls_give_equal_vectors"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support/hostile.rs#support::hostile::tests::the_image_opens_with_the_portable_executable_signature"
        status: pass
    human_judgment: false
  - id: D2
    description: "The stable replay of every committed regression input through both Mode::Strict and Mode::Salvage, asserting only that the process survives"
    requirement: "SAF-05"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/regressions.rs#every_regression_input_replays_in_both_modes_without_ending_the_process"
        status: pass
    human_judgment: false
  - id: D3
    description: "The count assertion refuses an emptied regression directory, proven by actually emptying it and reading the failure message"
    requirement: "SAF-05"
    verification:
      - kind: manual_procedural
        ref: "session transcript: emptied crates/deform6/tests/regressions/, ran cargo test -p deform6 --test regressions, restored the file, reran green"
        status: pass
    human_judgment: true
    rationale: "Proving a test can fail is a one-time act performed during this session, not a repeatable automated check; the resulting failure message is quoted in this SUMMARY and in the commit history for a human to re-verify if desired."
  - id: D4
    description: "The committed seed's provenance is checkable: a test asserts its bytes equal support::hostile::gui_table_overcount_4k's own output"
    requirement: "SAF-05"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/regressions.rs#the_committed_seed_equals_the_builder_that_produced_it"
        status: pass
    human_judgment: false

duration: ~30min (across two turns, paused at a human checkpoint between them)
completed: 2026-09-13
status: complete
---

# Phase 05 Plan 05: Hostility Summary

**A hostile PE image built from literals, a sorted-and-replayed regression directory in both reader modes, and one committed seed whose provenance a test proves**

## Performance

- **Duration:** ~30 min of active execution, split across two turns by a blocking `checkpoint:decision`
- **Started:** 2026-09-13T19:XX (task 1)
- **Completed:** 2026-09-13T21:27:14+02:00 (task 2 commit)
- **Tasks:** 2
- **Files modified:** 4 (2 created source files, 1 modified source file, 1 new binary fixture)

## Accomplishments
- `support::hostile::gui_table_overcount_4k` builds a 4096 byte, 32 bit i386 PE image with one mapped section and one valid GUI table entry, from literals only, with no `include_bytes!` and no path under `corpus/` anywhere in the file (both checked by grep, both zero).
- `crates/deform6/tests/regressions.rs` walks `crates/deform6/tests/regressions/` in sorted order (the same recursive shape `corpus_sweep.rs` uses, extension filter dropped) and replays every file through `deform6::inspect` in both `Mode::Strict` and `Mode::Salvage`, calling `deform6::write::project` on a successful salvage read, asserting nothing about the result variant, only that the process is still running.
- The first committed regression input, `gui-table-overcount-4k.bin`, arrived in the same commit as the harness, so `crates/deform6/tests/regressions/` was never empty in this project's history. A test proves the committed bytes equal the builder's output byte for byte.
- The count assertion (`MINIMUM_REGRESSION_INPUTS = 1`) was proven to actually fail: the seed was moved out of the directory, the test failed by name, and the seed was moved back before the commit. See "Issues Encountered" for the exact message.
- The replay itself was proven to actually fail: a temporary `panic!` was added to the first line of `deform6::vb::inspect`, the test ended the process, the log's last line named the seed file before the panic, and the panic was removed before the commit (confirmed via `git diff` showing no residual change).
- The crash-to-test procedure is written as `regressions.rs`'s own module doc comment, eight numbered steps, at the top of the file.

## Task Commits

Each task was committed atomically:

1. **Task 1: A hostile image builder this repository owns outright** - `4a8dbc4` (feat)
2. **Task 2: The stable replay, the seed, and the count assertion in one commit** - `93f68ac` (feat)

**Plan metadata:** pending (this commit)

## Files Created/Modified
- `crates/deform6/tests/support/hostile.rs` - `gui_table_overcount_4k`, the hostile PE builder, plus three unit tests
- `crates/deform6/tests/support/mod.rs` - declares `pub mod hostile;` and extends the module doc comment to say where its bytes come from
- `crates/deform6/tests/regressions.rs` - the sorted replay, the crash-to-test procedure, and four tests (replay, provenance, no-duplicates, zero-byte)
- `crates/deform6/tests/regressions/gui-table-overcount-4k.bin` - the first committed seed, `support::hostile::gui_table_overcount_4k`'s own output

## Decisions Made

**The checkpoint decision.** `05-05-PLAN.md`'s own `checkpoint:decision` task asked whether `crates/deform6/tests/regressions/` becomes a directory of committed binary fixtures, starting with one this repository builds itself. Before deciding, I measured the actual reachability of the proposed seed through `deform6::inspect` (a throwaway scratchpad probe, never part of the repository): both `Mode::Strict` and `Mode::Salvage` give `Err(NoVbRuntime { dot_net: false })`, refusing at `runtime_of`, before the read ever reaches `header_region`, `VbHeader::read`, or `GuiTable::walk`. I reported this measurement alongside the three options rather than picking one; the coordinator independently re-verified it with its own throwaway probe (same two `NoVbRuntime` results) and put the decision to the human with that measurement in hand. The human chose `seed-with-an-owned-binary`.

**How the seed's scope is documented.** Because the seed does not reach the GUI table, `regressions.rs`'s module doc comment states this plainly, quotes the measured `Err(NoVbRuntime { .. })` result in both modes, and names `gui_table_refuses_an_implausible_form_count` in `crates/deform6/src/vb/gui.rs` (plan 05-02) as the test that already covers the `wFormCount = 0xFFFF` bound check, so a later reader does not credit this fixture with proving something it does not.

**No `VBHeader` embedded in the image.** Task 1's `<behavior>` block names "a VB header whose declared form count is 0xFFFF," but its `<action>` block's literal byte recipe never mentions a `VB5!` magic or an entry-point stub, and it explicitly instructs following plan 05-02's own fixture, which built its `VbHeader` as an in-memory value passed directly to `GuiTable::walk`, never written into the image. `hostile.rs` follows that precedent. See "Issues Encountered" for this as a plan-vs-code discrepancy.

## Deviations from Plan

### Auto-fixed Issues

None - both tasks were completed as the plan's `<action>` blocks specify, adapted only where the plan's own instructions pointed at plan 05-02's precedent (see Decisions Made).

---

**Total deviations:** 0 auto-fixed.
**Impact on plan:** None. The one substantive gap between the plan's prose and the shipped code (see "Issues Encountered") was resolved by following the plan's own explicit fallback instruction, not by deviating from it.

## Issues Encountered

**The plan's `<behavior>` prose overstates what the seed's bytes contain.** Task 1's behavior line reads "a VB header whose declared form count is 0xFFFF." The image, built per the plan's own literal action steps and its own instruction to mirror plan 05-02's fixture, embeds no `VbHeader` structure at all. Measured this session: `deform6::inspect` on the committed seed gives `Err(NoVbRuntime { dot_net: false })` in both modes, confirmed independently by the coordinator's own throwaway probe before the checkpoint was put to the human. This is documented in `regressions.rs`'s own module doc comment rather than silently left for a future reader to discover.

**Proving the count assertion fails, quoted.** With `gui-table-overcount-4k.bin` moved out of `crates/deform6/tests/regressions/`, `cargo test -p deform6 --test regressions every_regression_input_replays_in_both_modes_without_ending_the_process` failed with:

```
crates/deform6/tests/regressions/ holds 0 files, wanted at least 1. This directory must never be emptied: a loop over an empty directory passes while proving nothing.
```

The seed was restored and the same test passed again (`1 passed; 0 failed`) before any commit was made.

**Proving the replay itself fails.** A temporary `panic!("TEMPORARY: proving regressions.rs can fail (task 2, plan 05-05)")` was added as the first line of `deform6::vb::inspect` in `crates/deform6/src/vb/mod.rs`. Running the replay test printed:

```
replaying /Users/stiven/Documents/Works/Coding/DeForm6/crates/deform6/tests/regressions/gui-table-overcount-4k.bin

thread '...' panicked at crates/deform6/src/vb/mod.rs:359:5:
TEMPORARY: proving regressions.rs can fail (task 2, plan 05-05)
```

The seed's own file name was the line printed immediately before the panic. The temporary panic was then removed; `git diff` against the committed state showed no residual change before task 2's commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`crates/deform6/tests/regressions/` now exists, holds one file, and is never empty in this project's history. Plans 05-04 and 05-08 (wave 4) both depend on this directory existing for their own seeding and no-panic runs; both remain unexecuted and are unaffected otherwise. `.planning/REQUIREMENTS.md`'s SAF-01 and SAF-05 remain `Pending`: both are shared with plans 05-04 and 05-08, which have not run, and this plan does not mark them complete.

---
*Phase: 05-hostility*
*Completed: 2026-09-13*

## Self-Check: PASSED

All created/modified files confirmed present on disk. Both task commits (`4a8dbc4`, `93f68ac`) confirmed present in `git log --oneline --all`.
