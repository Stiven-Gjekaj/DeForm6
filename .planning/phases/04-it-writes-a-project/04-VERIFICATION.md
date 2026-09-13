---
phase: 04-it-writes-a-project
verified: 2026-09-13T00:00:00Z
status: human_needed
score: 5/5 must-haves verified (roadmap success criteria), 43/43 plan-level must-have truths verified across nine plans
covered_files:
  - ".planning/REQUIREMENTS.md"
  - ".planning/ROADMAP.md"
  - ".planning/WINDOWS.md"
  - ".planning/phases/04-it-writes-a-project/04-01-PLAN.md"
  - ".planning/phases/04-it-writes-a-project/04-01-SUMMARY.md"
  - ".planning/phases/04-it-writes-a-project/04-02-PLAN.md"
  - ".planning/phases/04-it-writes-a-project/04-02-SUMMARY.md"
  - ".planning/phases/04-it-writes-a-project/04-03-PLAN.md"
  - ".planning/phases/04-it-writes-a-project/04-03-SUMMARY.md"
  - ".planning/phases/04-it-writes-a-project/04-04-PLAN.md"
  - ".planning/phases/04-it-writes-a-project/04-04-SUMMARY.md"
  - ".planning/phases/04-it-writes-a-project/04-05-PLAN.md"
  - ".planning/phases/04-it-writes-a-project/04-05-SUMMARY.md"
  - ".planning/phases/04-it-writes-a-project/04-06-PLAN.md"
  - ".planning/phases/04-it-writes-a-project/04-06-SUMMARY.md"
  - ".planning/phases/04-it-writes-a-project/04-07-PLAN.md"
  - ".planning/phases/04-it-writes-a-project/04-07-SUMMARY.md"
  - ".planning/phases/04-it-writes-a-project/04-08-PLAN.md"
  - ".planning/phases/04-it-writes-a-project/04-08-SUMMARY.md"
  - ".planning/phases/04-it-writes-a-project/04-09-PLAN.md"
  - ".planning/phases/04-it-writes-a-project/04-09-SUMMARY.md"
  - ".planning/phases/04-it-writes-a-project/04-REVIEW-FIX.md"
  - ".planning/phases/04-it-writes-a-project/04-REVIEW.md"
  - ".planning/phases/04-it-writes-a-project/04-VALIDATION.md"
  - "crates/deform6-cli/src/main.rs"
  - "crates/deform6-cli/tests/cli.rs"
  - "crates/deform6/src/report.rs"
  - "crates/deform6/src/write/code.rs"
  - "crates/deform6/src/write/comment.rs"
  - "crates/deform6/src/write/frm.rs"
  - "crates/deform6/src/write/mod.rs"
  - "crates/deform6/src/write/model.rs"
  - "crates/deform6/src/write/values.rs"
  - "crates/deform6/src/write/vbp.rs"
  - "crates/deform6/tests/extract_structural.rs"
  - "crates/deform6/tests/extract_tracer.rs"
  - "crates/deform6/tests/ratios.rs"
  - "crates/xtask/src/main.rs"
  - "tests/ratios.toml"
covered_digest: "v1:sha256:fe1fcfe72cfd327e3154a6002fd25809ba35e881254681355d8c4ede237eebfc"
behavior_unverified: 0
overrides_applied: 0
deferred:
  - truth: "The README states that full recompilation did not run and that the IDE never opened the project (a Phase 4 named risk)"
    addressed_in: "Phase 6"
    evidence: "ROADMAP.md Phase 6 success criterion 3: 'The README states three facts in plain words: ... full recompilation was not tested, because it needs VB6 on Windows.' Plan 06-01 is named 'The README - what it returns, what it does not...'. No README.md exists in the repository yet, which is correct: Phase 4 never claimed to own it, and the report's own limits list already carries the required sentence verbatim (crates/deform6/src/report.rs, asserted in extract_structural.rs)."
human_verification:
  - test: "Open one written project (for example the output of `deform6 extract corpus/vb6-code/Fire-effect/Fast_Flames.exe -o out/`) in the real VB6 IDE on a Windows host, per 04-VALIDATION.md's own Manual-Only Verifications row and 04-09-PLAN.md's own <human-check> block."
    expected: "The IDE opens the .vbp with no fatal load error, or produces a .log file naming what failed to load. Recording the true result (pass, partial, or fail) is the only way to know whether the structural check's 'shaped correctly' claim also means 'loads correctly' in the actual consuming application."
    why_human: "The CI sandbox has no Windows host and no VB6 install. This is a testing-infrastructure gap the roadmap itself accepts (\"Full recompilation cannot run in this CI\") and explicitly defers to a human with the right hardware. Nothing in the codebase can close this gap without that hardware; it is not a code defect."
---

# Phase 4: It writes a project Verification Report

**Phase Goal:** `deform6 extract <exe> -o <dir>` writes a Visual Basic project
directory that the VB6 IDE can open, and one JSON report beside it that grades
each recovered item by confidence and names the bytes it came from.

**Verified:** 2026-09-13
**Status:** human_needed
**Re-verification:** No — initial verification (no prior `04-VERIFICATION.md` existed; `.planning/phases/01-it-reads-the-file/VERIFICATION.md` shown deleted in `git status` is Phase 1's file, unrelated to this phase)

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `extract <exe> -o out/` on each of 44 corpus programs writes one `.vbp`, one `.frm` per form, one `.frx` per form holding a blob, one `.bas`/`.cls` per module/class, one JSON report; exits 0; writes nothing outside `out/` | VERIFIED | Orchestrator-measured directly (44/0 exit codes; `.frx` byte match). Independently confirmed: `crates/deform6-cli/tests/cli.rs#all_corpus_programs_extract_with_exit_zero_and_the_written_file_count_matches_the_report`, `#extract_changes_no_file_anywhere_in_the_repository_working_tree` (04-08-SUMMARY.md D5). 195 files across 44 programs, counted against the independently-read `Report`, not against the writer's own output. |
| 2 | Every text file holds Windows-1252 bytes, CRLF on every line including the last, no BOM | VERIFIED | Orchestrator-measured directly (CRLF, no BOM). Independently confirmed: `crates/deform6/tests/extract_structural.rs`'s whole-tree encoding sweep (04-09-SUMMARY.md D3), which counts line-feed bytes against pair counts and asserts the last two bytes of every written text file, excluding `.frx` (binary by kind) and `.report.json`. Broken on purpose three times (BOM, bare LF, byte above range) and confirmed failing before being fixed back. |
| 3 | Every `.frx` offset a `.frm` names resolves inside the `.frx` actually written, checked through `support/frm.rs`, for all 44 programs | VERIFIED | `crates/deform6/tests/extract_structural.rs` reads every written `.frm` back through the independent reader (`tests/support/frm.rs`), seeks every offset in the matching written `.frx`, and asserts the record ends inside the file and the last record ends exactly at file end. Deliberately broken (`a_resource_offset_shifted_by_one_byte_fails_the_offset_resolution_check`) and reverted. No type from `deform6::write::{frm,vbp,code,values,comment}` is imported by the checker (source grep = 0), so the writer and its checker cannot share a bug. |
| 4 | The structural check passes: `Form=`/`Module=`/`Class=` lines name existing files, `Startup=` names a declared form, every control/class name is a legal identifier of 40 characters or fewer, nesting depth ≤ 7, properties alphabetical, menus last | VERIFIED | Six independently-named assertion functions in `extract_structural.rs`, each unit-tested against a hand-built fixture and run corpus-wide over all 44 programs, each broken on purpose once (component line to missing file, reversed property order, menu before non-menu sibling, plus two extra breakages — illegal name, over-depth control) and confirmed failing before being restored. |
| 5 | `jq` query for `inferred` items returns paths; every item carries `basis` and an `evidence` record with a byte offset; `confidence` is one of three words, never a number; two runs give byte-identical reports | VERIFIED | Orchestrator-measured directly (62 items, exactly three confidence words, no null offset, byte-identical two-run diff). Independently confirmed: `crates/deform6/src/report.rs` `Confidence` enum has exactly 3 variants (source read), no wildcard match arm (grep = 0), `report::build` backfills evidence for every model-built item via `with_header_evidence` so RPT-04 holds for all sources, not only property-level items. `write::project` is wired to call `report::build` (confirmed by reading `write/mod.rs` call sites via 04-08-SUMMARY.md and by the orchestrator's own live report inspection). |

**Score:** 5/5 roadmap success criteria verified. All are presence-plus-behavior verified (passing tests exist and were watched failing at least once per the SUMMARY's own TDD-gate records, not merely presence-checked).

### The one truth this phase cannot verify by construction

The phase goal's own first clause — "a Visual Basic project directory **that the VB6 IDE can open**" — is not, and cannot be, verified by any test in this repository. The structural check (success criterion 4) proves the files are *shaped* correctly: legal identifiers, correct nesting, correct alphabetical order, resolvable offsets. It does not and cannot prove the IDE opens them, because that needs the VB6 IDE on a Windows host, which this environment does not have. The roadmap names this explicitly as an accepted, structural risk ("Full recompilation cannot run in this CI... Say that in the report and in the README. Do not imply that the IDE opened the project"), and the codebase honors the negative half of that promise correctly:

- `crates/deform6/src/report.rs`'s `build_limits` states, verbatim: *"Full recompilation did not run. It needs the Visual Basic 6 IDE on..."* — confirmed present by direct read (line 346).
- `crates/deform6/tests/extract_structural.rs` holds its own independent copy of the same sentence and asserts the shipped report's own first limit line equals it exactly (`the_reports_limits_state_the_exact_recompilation_sentence`), so the two cannot silently drift apart.
- A source grep for `the IDE (opened|loaded|compiled) the project` across `extract_structural.rs` and `report.rs` returns zero matches (confirmed directly), so no in-repo message overstates the result.
- The README half of the same roadmap sentence is **not** yet written, because no `README.md` exists in the repository at all. This is correct scope, not a gap: Phase 6 owns the README (`ROADMAP.md` Phase 6, plan `06-01`, and Phase 6 success criterion 3 states the identical three facts, including this one, must appear there). Recorded under `deferred` in the frontmatter, not as a gap.

This is why the phase cannot receive an unconditional `passed`: the phase's own stated goal names IDE-openability as the outcome, the codebase is honest that this was never tested, and the only way to actually resolve that open question is a human with a Windows host and VB6 running the one manual check `04-VALIDATION.md` and `04-09-PLAN.md`'s own `<human-check>` block both already specify and neither has yet been run. This is not a code defect — it is disclosed, tested-for-absence-of-overclaiming, and squarely a human-verification item per the phase's own design.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/deform6/src/write/mod.rs` | write module set, `write::project` entry point | VERIFIED | Exists; wired to full writers + `report::build` per 04-08 (thin writers removed as dead code). |
| `crates/deform6/src/write/model.rs` | `SafeName`, `ProjectModel`, name sanitization | VERIFIED | `SafeName` proven the only path to a file name; hostile-name tests pass; no `HashMap`. |
| `crates/deform6/src/write/values.rs` | property value formatter | VERIFIED | Exhaustive match, no wildcard arm (grep = 0); byte-anchored against real corpus lines. |
| `crates/deform6/src/write/vbp.rs` | `.vbp` writer | VERIFIED | 21 tests; byte-checked against `FlameTest.vbp` except the disclosed `Reference=` gap. |
| `crates/deform6/src/write/frm.rs` | `.frm`/`.frx` writer, one `BlobCursor` | VERIFIED | One cursor per form; CR-01 desync fix confirmed by orchestrator's revert-and-observe. |
| `crates/deform6/src/write/code.rs` | `.bas`/`.cls` writer | VERIFIED | 13-line class preamble byte-matched to corpus; empty-body procedures compile-shaped. |
| `crates/deform6/src/write/comment.rs` | uncertainty comment emitter | VERIFIED | Corpus-wide sweep proves no comment above the boundary, at least one below it, for the real corpus. |
| `crates/deform6/src/report.rs` | `ProjectReport`, `Confidence`, `Evidence` | VERIFIED | 3-variant enum confirmed by direct read; no wildcard arm; determinism proven twice (unit + full-pipeline). |
| `crates/deform6/tests/extract_structural.rs` | independent-reader structural check | VERIFIED | 22 tests; imports zero writing-module types (source grep = 0); all 44 programs pass. |
| `tests/ratios.toml` | write-side pinned ratios | VERIFIED | `property_declared`=807, `property_written`=136 sums independently recomputed via `awk` and matched exactly to the SUMMARY's claimed totals; rewrite-produces-no-diff confirmed as the file/tool agreement check. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `deform6-cli::run_extract` | `deform6::write::project` | `write::project` call | VERIFIED | Confirmed via 04-08's wiring commit `bc3fb4a` and orchestrator's live report inspection (non-empty items/limits). |
| `write::frm` | `vb::frx::BlobCursor` | one cursor per form | VERIFIED | Source-level: `FRX_ITEM_HEADER_LEN` referenced, `BlobCursor` referenced ≥1 non-comment time; CR-01 fix confirmed by revert-and-observe. |
| `write::code::write_code_region` | `write::comment::uncertainty_comments` | one legal call site | VERIFIED | `grep -c uncertainty_comments` = 2 in `code.rs`, 0 in `vbp.rs` and `frm.rs` (confirmed via SUMMARY's own self-check, consistent with plan's key_links). |
| `report::build` | `write::model::from_report` items | `with_header_evidence` backfill | VERIFIED | Confirmed via direct read of `report.rs` (`with_header_evidence`, `pub(crate)`) and 04-06/04-08 SUMMARYs. |

### Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| WRT-01 | SATISFIED | `[x]` in REQUIREMENTS.md; corpus-wide sweep, containment check, build-then-write ordering all independently confirmed. |
| WRT-02 | SATISFIED | `[x]`; 21 `.vbp` tests, byte-checked against `FlameTest.vbp`. |
| WRT-03 | **Correctly left open** | `[ ]` in REQUIREMENTS.md. Column-layout claims (3-space indent, 16-column pad, 3 spaces after `=`, exact trailing-space rule) are proven only by plan 04-04's targeted unit tests anchored to real corpus bytes (confirmed: `a_property_lines_equals_sign_sits_at_the_same_column_as_a_real_corpus_line` reads a real offset out of `frmFire.frm` and asserts the writer's column matches it). `tests/support/frm.rs`'s independent `Block` parser trims every line by design, so `extract_structural.rs` genuinely cannot serve as corpus-wide evidence for a byte-level column claim — the 04-09 executor's reasoning for leaving WRT-03 unchecked is technically correct, not evasive. Not a phase-4 gap; a correctly-scoped open item for a future plan that reads raw `.frm` bytes directly. |
| WRT-04 | SATISFIED | `[x]`; `extract_structural.rs`'s alphabetical-order and menus-last checks run corpus-wide, closing this requirement more thoroughly than plan 04-04 alone did. |
| WRT-05 | SATISFIED | `[x]`; preamble byte-matched to `FastDrawing.cls`. |
| WRT-06 | SATISFIED | `[x]`; encoding sweep corpus-wide, no BOM, CRLF, no byte above range. |
| WRT-07 | SATISFIED | `[x]`; four procedure-recovery shapes each produce a compilable empty-body signature and a distinct report item. |
| RPT-01 to RPT-06 | SATISFIED | All `[x]`; report shape, confidence vocabulary, evidence, defects, determinism, and comment-emission rules all independently confirmed by direct source reads plus passing tests. |

No orphaned requirements: every ID REQUIREMENTS.md maps to Phase 4 (WRT-01..07, RPT-01..06) appears in at least one plan's `requirements:` frontmatter field.

### Anti-Patterns Found

No `TBD`, `FIXME`, `XXX`, `TODO`, `HACK`, or `PLACEHOLDER` marker found in any Phase-4-touched source file (`crates/deform6/src/write/*.rs`, `report.rs`, `deform6-cli/src/main.rs`, `extract_tracer.rs`, `extract_structural.rs`, `xtask/src/main.rs` — all checked directly). No `HashMap`, `static mut`, `OnceLock`, `LazyLock`, or `thread_local` in the write/report modules (checked directly, confirms the determinism prohibitions phase-wide). Code review (`04-REVIEW.md`, iteration 2) is `status: clean`, 0 Critical, 0 Warning, 1 Info (IN-01, a low-risk report-path-only sanitization gap, carried forward by design).

### Declared open items (checked against roadmap success criteria)

| Finding | Source | Undercuts a success criterion? |
|---------|--------|-------------------------------|
| No `Reference=` (type-library) line ever written | WINDOWS.md #10 | No. Not named in any of the 5 roadmap success criteria or in WRT-01..07. Honest staging: `Report` carries no field for it; would need a new reader. |
| Generic `VB.Control` class for external (OCX) controls | 04-04-SUMMARY.md | No. The corpus holds zero forms with a real external control placed on them, so it is untested by construction, not a guessed/wrong answer against any measured case. Documented, not hidden. |
| Generated-control-array-index report item uses literal `/forms/*/controls/<name>` path | WINDOWS.md #11 | Minor tension with RPT-02's path-shape example, but RPT-02 only requires "a path such as" the example, not that every item follow it exactly; this is a narrow, disclosed edge case (an asterisk placeholder), not a fabricated or silently wrong path. WARNING-level, not a gap against a stated success criterion. |
| Redundant report entries: a property earns two items (control-path and property-path) from two independent derivations | WINDOWS.md #13, fixed=false | No. Both entries are individually correct and carry real evidence; redundant, not wrong. Explicitly recorded for a future unification pass. |
| `tests/ratios.toml`'s new write-side pin (807/136) vs. REQUIREMENTS.md's stale FRM-03 prose (122/683/805, five names) | REQUIREMENTS.md, dated to Phase 3 | No. The pin is self-consistent with the code (`update-ratios` rewrite produces no diff, confirmed directly) and with 04-02's own re-measurement (124/683/807, six names) — the pin tracks the tool's current output. REQUIREMENTS.md's FRM-03 paragraph is Phase 3 prose that was never asked to be re-worded in Phase 4; a documentation staleness note, not a code defect. |
| `ROADMAP.md`'s traceability table still reads "WRT-01 to WRT-07 \| Phase 4 \| Pending" | ROADMAP.md line 660 | No functional effect; inconsistent with the per-requirement `[x]`/`[ ]` checkboxes in REQUIREMENTS.md (6 of 7 checked). Cosmetic staleness, worth a one-line fix in a documentation-only commit, not a phase-4 code gap. |

None of the above undercuts a stated roadmap success criterion. All are honestly recorded, none silently absorbed.

## Human Verification Required

### 1. Open a written project in the real VB6 IDE

**Test:** Run `deform6 extract` on one corpus program (for example `Fast_Flames.exe`) into a directory, copy the directory to a Windows host with VB6 installed, and open the written `.vbp` in the IDE.

**Expected:** The IDE opens the project with no fatal load error, or — if it fails — produces a `.log` file beside the form naming the line and the message, which should be recorded here.

**Why human:** This CI sandbox has no Windows host and no VB6 install. `04-VALIDATION.md`'s own "Manual-Only Verifications" table and `04-09-PLAN.md`'s own `<human-check>` block both name this exact test and both state plainly that nothing in the repository may claim this step ran until it actually does. It has not yet been run. This is the one part of the phase's stated goal ("...that the VB6 IDE can open") that the structural check cannot stand in for, and the codebase is honest about that rather than overclaiming it.

## Gaps Summary

No coding gap blocks this phase's roadmap success criteria. All five roadmap success criteria are independently verified against the codebase, not merely asserted by the SUMMARYs: source reads confirm the `Confidence` enum, the report's exact recompilation sentence, the absence of any "IDE opened" claim, the absence of forbidden constructs (HashMap, static mutable state, debt markers), and the write-side ratio pin's internal self-consistency. The code review's Critical/Warning findings were all closed and the most consequential fix (CR-01, the `BlobCursor` desync) was independently confirmed by the orchestrator's revert-and-observe test.

The phase is withheld from an unconditional `passed` for exactly one reason: the phase goal's own text asserts an outcome ("a project directory **that the VB6 IDE can open**") that no test in this repository can exercise, and the one manual step that could resolve it has not been run. This is a pre-existing, roadmap-accepted testing-infrastructure limit (no Windows/VB6 host in CI), not a defect introduced by this phase's plans — but it is also not something the verifier can wave through as "presumably fine," since it is literally half of the stated goal. It is routed to human verification, not marked as a gap, because the codebase does everything within its power (a real structural check, honest limits language, no overclaiming) and the remaining question needs hardware this environment does not have.

A secondary, much smaller item worth a human decision: `ROADMAP.md`'s Phase 4 traceability line ("WRT-01 to WRT-07 | Phase 4 | Pending") is stale against REQUIREMENTS.md's own per-item checkboxes (6 of 7 marked `[x]`, WRT-03 correctly left `[ ]`) and against the Progress table's "In Progress" row for Phase 4 — both are expected to be updated as part of closing out this verification, not treated as phase-4 code gaps.

---

_Verified: 2026-09-13_
_Verifier: Claude (gsd-verifier)_
