---
phase: 04-it-writes-a-project
verified: 2026-09-13T04:00:00Z
status: passed
score: 5/5 roadmap success criteria verified; 1 phase-goal clause waived, not verified
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
  - ".planning/phases/04-it-writes-a-project/04-SECURITY.md"
  - ".planning/phases/04-it-writes-a-project/04-UAT.md"
  - ".planning/phases/04-it-writes-a-project/04-VALIDATION.md"
  - "crates/deform6-cli/src/main.rs"
  - "crates/deform6-cli/tests/cli.rs"
  - "crates/deform6/src/report.rs"
  - "crates/deform6/src/vb/privateobj.rs"
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
covered_digest: "v1:sha256:9e42b0972686d7d80b1efed742ae1eb0de64f0b18df3741a87ab77d96bf31179"
behavior_unverified: 0
overrides_applied: 1
overrides:
  - must_have: "A Visual Basic project directory that the VB6 IDE can open (phase goal clause; WRT-01's Manual-Only Verification row in 04-VALIDATION.md; 04-09-PLAN.md's own <human-check> block)"
    reason: "No Windows host with Visual Basic 6 is available anywhere in this project's toolchain. The human waived this check on 2026-09-13 rather than wait for that hardware. This is a waiver of the check, not evidence that the check would pass. Nobody has opened a written project in the VB6 IDE. The structural check (roadmap success criteria 1-5, all independently confirmed below) proves the files are shaped correctly by every rule this repository can state; it does not and cannot prove the IDE accepts them."
    accepted_by: "human, recorded in 04-UAT.md test 1 (result: skipped)"
    accepted_at: "2026-09-13"
re_verification:
  previous_status: human_needed
  previous_score: 5/5 (roadmap success criteria), one human item outstanding
  gaps_closed:
    - "The one outstanding human-verification item (open a written project in the real VB6 IDE) is now resolved by an explicit, recorded human waiver in 04-UAT.md, not by new evidence. See overrides above."
  gaps_remaining: []
  regressions: []
deferred:
  - truth: "The README states that full recompilation did not run and that the IDE never opened the project"
    addressed_in: "Phase 6"
    evidence: "ROADMAP.md Phase 6 success criterion 3 requires the README to state this fact in plain words. No README.md exists yet, which is correct: Phase 4 never claimed to own it, and the shipped report's own limits array already carries the required sentence verbatim, confirmed live: 'A structural check ran in its place, and it never opened this project in the IDE.'"
human_verification: []
---

# Phase 4: It writes a project Verification Report

**Phase Goal:** `deform6 extract <exe> -o <dir>` writes a Visual Basic project
directory that the VB6 IDE can open, and one JSON report beside it that grades
each recovered item by confidence and names the bytes it came from.

**Verified:** 2026-09-13
**Status:** passed — resting in part on a human waiver, not on evidence, for one clause of the goal. Read "The clause this phase cannot prove" below before treating this as an unconditional pass.
**Re-verification:** Yes — this supersedes the prior `04-VERIFICATION.md` (`status: human_needed`), which is stale: it predates the six code-review fix commits, the iteration-2 re-review, the security audit, and the human's UAT waiver.

## The line this report will not cross

The phase goal names two outcomes. The first — "a Visual Basic project
directory **that the VB6 IDE can open**" — is not proven anywhere in this
repository, and this report does not claim it is proven. No test in this
codebase can open the VB6 IDE, because the IDE runs only on Windows and no
Windows host with VB6 exists in this project's toolchain.

What actually happened: on 2026-09-13 the human closed `04-UAT.md`'s one test
("Open a written project in the real VB6 IDE") with `result: skipped` and this
reason, recorded verbatim: "Waived by the human on 2026-09-13. No Windows host
with Visual Basic 6 is available. Nobody has opened a written project in the
VB6 IDE. The human chose to close the phase without this evidence. This is a
waiver, not a pass: no part of this repository may state that the IDE opened a
written project."

This report honors that distinction. The overall status below is `passed`
because every truth this repository *can* test is independently confirmed
against the current codebase (not against SUMMARY.md's word), and the one
truth it cannot test has been explicitly waived by the person with the
authority to accept that risk, not silently absorbed. A future reader of only
this file should walk away knowing: the structural evidence is strong, and the
IDE-opens clause is unproven and waived, not demonstrated.

I independently re-confirmed, at current HEAD, that the codebase still makes
no contrary claim:

- `crates/deform6/src/report.rs` line 346 still states, verbatim: "Full
  recompilation did not run. It needs the Visual Basic 6 IDE on..."
- A live extraction run in this session (`Fast_Flames.exe` -> fresh output
  directory) shows the shipped report's `limits` array reads: "A structural
  check ran in its place, and it never opened this project in the IDE." —
  an explicit negative statement, not merely an absence of a positive one.
- `crates/deform6/src/report.rs` lines 844-845 hold a source-level assertion,
  read directly, that a limit line must never contain the string "the ide
  opened": `!line.to_lowercase().contains("the ide opened")`. This is the one
  and only place that phrase appears in the phase's source and test files; it
  appears as the negative check itself, not as a claim.
- A grep across `extract_structural.rs`, `report.rs`, and every file under
  `write/` and `deform6-cli/src/main.rs` for "the IDE (opened|loaded|compiled)"
  returns zero occurrences of the claim in prose form. Unchanged from the
  prior verification.

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `extract <exe> -o out/` on each of 44 corpus programs writes one `.vbp`, one `.frm` per form, one `.frx` per form holding a blob, one `.bas`/`.cls` per module/class, one JSON report; exits 0; writes nothing outside `out/` | VERIFIED | Ran independently in this session: `cargo test --workspace` (fresh run at HEAD, not reused from a prior session) — 872 tests, 0 failures, matching the orchestrator's measured count exactly (603+4+2+26+2+22+13+1+44+9+44+2+6+38+56 = 872). `crates/deform6-cli/tests/cli.rs`'s corpus-wide exit-code and file-count tests are part of that run. Live spot-check: extracted `Fast_Flames.exe` to a fresh scratch directory in this session; exit 0; `frmFire.frx` byte-identical to the committed source (`cmp` clean); re-ran into a second fresh directory and `diff -rq` reported the two trees identical (determinism holds at the CLI level, not only in unit tests). |
| 2 | Every text file holds Windows-1252 bytes, CRLF on every line including the last, no BOM | VERIFIED | Independently confirmed in the live extraction: `xxd` on the first bytes of the written `.frm` shows `56 45 52 53...` ("VER..."), no BOM; a CRLF count (`grep -Uc $'\r$'`) equals the file's own line count (94 of 94); `crates/deform6/tests/extract_structural.rs`'s whole-tree encoding sweep (part of the fresh 872-test run) still passes corpus-wide. |
| 3 | Every `.frx` offset a `.frm` names resolves inside the `.frx` actually written, for all 44 programs | VERIFIED | Part of the same fresh `cargo test --workspace` run (`extract_structural.rs`, 44/44 programs). Ran the single named regression test `write::frm::tests::a_blob_whose_range_does_not_fit_leaves_the_cursor_unmoved_for_the_next_blob` directly in this session (`cargo test -p deform6 --lib ... --exact`): passed. This is the CR-01 fix's own regression test — the `BlobCursor` desync that would have drifted every later offset in a form. Source grep confirms `extract_structural.rs` imports zero types from `deform6::write::{frm,vbp,code,values,comment}`. |
| 4 | The structural check passes (component-line existence, `Startup=`, identifier legality, nesting depth, alphabetical properties, menus last) | VERIFIED | Part of the same fresh 872-test run, 44/44 programs, unchanged from the prior verification's method (each assertion function individually broken-and-restored per its own SUMMARY). |
| 5 | `jq` query for `inferred` items returns paths; every item carries `basis` and an `evidence` record with a byte offset; `confidence` is one of three words, never a number; two runs give byte-identical reports | VERIFIED | Independently re-measured in the live extraction, not reused from the orchestrator's numbers: `jq -r '.items[].confidence' \| sort -u` returns exactly `inferred`, `proven`, `unrecoverable` (3 values); `jq '.items \| length'` = 62; `jq` over `inferred` paths returns 11 rows; `jq '[.items[] \| select(.basis==null or .basis=="")] \| length'` = 0 (every item carries a basis); `jq '[.items[].evidence[]? \| select(.offset==null)] \| length'` = 0 (no null byte offset). Ran the three named determinism tests directly: `write::vbp::tests::two_calls_to_the_writer_on_one_model_give_byte_identical_output`, `write::code::tests::write_bas_called_twice_on_one_input_gives_byte_identical_output`, `write::code::tests::write_cls_called_twice_on_one_input_gives_byte_identical_output`, `report::tests::the_whole_write_path_run_twice_over_fast_flames_gives_byte_identical_report_files` — all four `ok`. |

**Score:** 5/5 roadmap success criteria independently verified against current HEAD, all with a fresh test run in this session (not reused from a prior session's numbers).

### The clause this phase cannot prove (waived, not verified)

The phase goal's first clause — "...that the VB6 IDE can open" — is outside
the five roadmap success criteria's own text (none of the five names the IDE)
but is the phase goal's own stated outcome. See "The line this report will
not cross" above. Recorded here as `PASSED (override)` in the frontmatter,
matched against `04-VALIDATION.md`'s "Manual-Only Verifications" row for
WRT-01 and `04-09-PLAN.md`'s `<human-check>` block, which are the same single
test `04-UAT.md` records as waived.

### `verification: backstop` Must-Haves (Plans 04-03, 04-05)

Two must-have truths in this phase carry `verification: backstop`, meaning
presence-plus-wiring is not enough by this workflow's own rule — an explicit,
runnable, passing test is required, or the truth must be recorded as an
abstained residual.

| Backstop truth | Plan | Status | Evidence |
|---|---|---|---|
| "Two calls to the project file writer with the same model give byte identical output, and the writer holds no process global mutable state." | 04-03 | VERIFIED | `write::vbp::tests::two_calls_to_the_writer_on_one_model_give_byte_identical_output`, run directly in this session (`--exact`), passed. Source grep for `HashMap`/`static mut`/`OnceLock`/`LazyLock`/`thread_local` in `write/vbp.rs` returns zero. |
| "Two calls to the code writer with the same model give byte identical output, and the writer reaches no process global mutable state." | 04-05 | VERIFIED | `write::code::tests::write_bas_called_twice_on_one_input_gives_byte_identical_output` and `write::code::tests::write_cls_called_twice_on_one_input_gives_byte_identical_output`, both run directly in this session (`--exact`), both passed. Same zero-mutable-state grep applies to `write/code.rs`. |

Neither backstop truth is a residual: both have a named, currently-passing
test that exercises exactly the claim, run independently in this
verification session, not merely present in the file tree.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/deform6/src/write/mod.rs` | write module set, `write::project` entry point | VERIFIED | Live extraction in this session produced a non-empty report (`items: 62`, `limits: 4`) through this exact entry point; `report::build` is wired in, not the earlier `items: []` stub WINDOWS.md finding 12 recorded (finding 12 status is `fixed`, confirmed by both the ledger and the live report). |
| `crates/deform6/src/write/model.rs` | `SafeName`, `ProjectModel`, name sanitization | VERIFIED | WR-02 fix (`b020ecc`) confirmed present: `is_legal_identifier_char` source read (unchanged check re-run not repeated here; prior verification's direct read stands, file unmodified since). |
| `crates/deform6/src/write/vbp.rs` | `.vbp` writer | VERIFIED | CR-03 fix (`7872ac6`) test `a_component_whose_file_name_holds_a_line_break_writes_no_object_line_and_an_item` is part of the fresh 872-test run. |
| `crates/deform6/src/write/frm.rs` | `.frm`/`.frx` writer, one `BlobCursor` | VERIFIED | CR-01 fix confirmed by directly running its own named regression test in this session (see truth 3 above), not merely by reading the SUMMARY's claim of a prior revert-and-observe cycle. |
| `crates/deform6/src/write/code.rs` | `.bas`/`.cls` writer | VERIFIED | CR-02 fix (`bd01d86`) and WR-03 fix (`c2c727f`) tests are part of the fresh 872-test run. |
| `crates/deform6/src/report.rs` | `ProjectReport`, `Confidence`, `Evidence` | VERIFIED | Confidence vocabulary and the anti-overclaim assertion independently re-read at current HEAD (see "The line this report will not cross"). |
| `crates/deform6/tests/extract_structural.rs` | independent-reader structural check | VERIFIED | Fresh run, 44/44 programs, zero writer-module imports (source grep). |
| `crates/deform6-cli/src/main.rs` | `extract` subcommand, `--force` symlink guard | VERIFIED | WR-01 fix (`6c384d7`) test `a_preexisting_symlink_at_a_planned_path_refuses_the_whole_run_under_force` is part of the fresh 872-test run. |
| `tests/ratios.toml` | write-side pinned ratios | VERIFIED | Part of the fresh 872-test run (`ratios::the_gate_passes_on_the_committed_file` and related, all `ok`). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `deform6-cli::run_extract` | `deform6::write::project` | `write::project` call | VERIFIED | Live extraction in this session produced a populated report through the CLI entry point, not a stub. |
| `write::frm` | `vb::frx::BlobCursor` | one cursor per form | VERIFIED | CR-01's own regression test run directly and confirmed passing in this session. |
| `write::code::write_code_region` | `write::comment::uncertainty_comments` | one legal call site | VERIFIED | WR-03 fix test is part of the fresh 872-test run. |
| `report::build` | `write::model::from_report` items | `with_header_evidence` backfill | VERIFIED | Live-extracted report shows every one of its 62 items with a non-null `basis` and a non-null evidence offset, confirmed by direct `jq` query in this session. |

### Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| WRT-01 | SATISFIED (structural half); IDE-opens half waived, not evidenced | `[x]` in REQUIREMENTS.md. `04-VALIDATION.md`'s Manual-Only Verifications table names WRT-01's IDE-open behavior specifically and that row is the one waived in `04-UAT.md`. The written half of WRT-01 (a project directory gets written) is independently confirmed by the live extraction in this session. |
| WRT-02 | SATISFIED | `[x]`; 21 `.vbp` tests, part of the fresh run. |
| WRT-03 | **Correctly left open — verified as accurate, not a gap** | `[ ]` in REQUIREMENTS.md, with this note: "The corpus wide structural check cannot confirm it, because `crates/deform6/tests/support/frm.rs` trims each line by design and does not read the exact indentation. To close WRT-03, a check must read the raw `.frm` bytes directly." I confirmed this reasoning is technically sound: `tests/support/frm.rs`'s `Block` parser trims lines (unchanged from the prior verification's direct read), so a corpus-wide sweep genuinely cannot serve as evidence for a byte-level column claim. Column-level correctness rests only on plan 04-04's targeted unit tests anchored to real corpus bytes (e.g. the `BackColor` column check the live extraction reproduced: `BackColor       =   &H80000005&`, matching the corpus convention). This is a correctly-scoped open item, not a phase-4 defect, and does not block this phase's pass — REQUIREMENTS.md's own status text is accurate as written. |
| WRT-04 to WRT-07 | SATISFIED | `[x]`; corpus-wide checks in the fresh run. |
| RPT-01 to RPT-06 | SATISFIED | `[x]`; report shape, confidence vocabulary, evidence, defects, determinism and comment rules all independently re-confirmed in this session via a live extraction plus four re-run named tests. |

No orphaned requirements: every ID REQUIREMENTS.md maps to Phase 4 (WRT-01..07,
RPT-01..06) appears in at least one plan's `requirements:` frontmatter field.

**Judgment on WRT-03 and the phase pass:** one requirement in "open, unit
proved only" status does not block this phase's pass. REQUIREMENTS.md's own
traceability table (line 205) already reads `WRT-03 | Phase 4 | Open, unit
proved only` — an honest, non-blocking status, not a claim of completion. The
gap is narrow (byte-level column indentation only, not the property values or
structure themselves, which WRT-04 and the structural check do cover
corpus-wide) and the documentation explaining why it cannot close without new
test infrastructure is accurate. Treating this as a blocking gap would
effectively demand a corpus-wide raw-byte column checker that no plan in this
phase was scoped to build; the roadmap's own five success criteria do not
require it (criterion 4 covers structure and ordering, not column position).

### Anti-Patterns Found

No `TBD`, `FIXME`, `XXX`, `TODO`, `HACK`, or `PLACEHOLDER` marker found in any
Phase-4-touched source file (checked directly at current HEAD:
`crates/deform6/src/write/*.rs`, `report.rs`, `vb/privateobj.rs`,
`deform6-cli/src/main.rs`, `extract_tracer.rs`, `extract_structural.rs`,
`xtask/src/main.rs`). No `HashMap`, `static mut`, `OnceLock`, `LazyLock`, or
`thread_local` in the write/report modules. `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets -- -D warnings` both run clean in this
session, on the full workspace including the six post-plan fix commits.

Code review (`04-REVIEW.md`, iteration 2): `status: clean`, 0 Critical,
0 Warning, 1 Info (IN-01, a report-path-only string-concatenation item the
security auditor separately traced and confirmed never reaches a file path;
carried forward by design, not a defect).

Security audit (`04-SECURITY.md`): `status: verified`, 26 threats registered,
26 closed, `threats_open: 0`, ASVS L1. Two `accept`-disposition items
(T-4-07, T-4-23) are explicitly flagged in the register itself as "Not
reviewed by a human" — this is the security document's own honest disclosure,
not a phase-4 gap, and does not block this pass since both are low-severity
and their rationale is stated inline.

### Declared Open Items (WINDOWS.md, checked against roadmap success criteria)

| Finding | Status | Undercuts a success criterion? |
|---------|--------|-------------------------------|
| #10: No `Reference=` (type-library) line ever written | open | No. Not named in any of the 5 roadmap success criteria or in WRT-01..07. `Report` carries no field for it by design; needs a new reader in a future phase. |
| #11: Generic control-array-index report item uses literal `/forms/*/controls/<name>` path | open | No. RPT-02 requires "a path such as" the given example, not that every item follow it exactly. Narrow, disclosed edge case. |
| #12: `write::project` shipped `items: []`/`limits: []`, never calling `report::build` | **fixed** (resolved_at 2026-09-13T03:34:21) | N/A — closed. Independently reconfirmed live: this session's own extraction produced 62 items and 4 limits through this exact path. |
| #13: A property earns two independently-derived report items (control-path and property-path) | open | No. Both entries are individually correct with real evidence; redundant, not wrong. Recorded for a future unification pass. |

None of the four open/fixed findings undercuts a stated roadmap success
criterion. This matches the prior verification's conclusion, independently
re-checked against the current ledger state (`open_count: 9`, `fixed_count: 4`,
`total_count: 13` — findings 1-9 predate this phase and are Phase 1/2/3's own
open items, not Phase 4's).

## Human Verification Required

None outstanding. The one item the prior verification routed to human
verification — "Open a written project in the real VB6 IDE" — has been
explicitly closed by human decision, recorded as a waiver rather than a pass,
in `04-UAT.md` (`status: complete`, test 1 `result: skipped`). See "The line
this report will not cross" above and the `overrides` entry in this file's
frontmatter for the full text of that waiver. This report does not treat the
waiver as evidence that the IDE opens a written project — it treats it as the
human's informed decision to close the phase without that evidence, which is
a different, and weaker, thing, stated here in those words.

## Gaps Summary

No coding gap blocks this phase's roadmap success criteria. All five are
independently re-verified against current HEAD in this session: a fresh
872-test `cargo test --workspace` run, a live extraction with direct `jq`
inspection of the produced report, a byte-for-byte `.frx` comparison, a
two-run determinism diff at the CLI level, and four individually-run named
unit tests covering the two `backstop`-tagged determinism truths and the
CR-01 fix. `cargo fmt --all --check` and `cargo clippy --workspace
--all-targets -- -D warnings` both pass clean at current HEAD.

The phase receives `passed`, not `human_needed`, because the one item that
previously routed to human verification has been explicitly resolved by the
person with authority to accept that risk — not because new evidence closed
it. This is recorded as an override in this file's frontmatter, not silently
folded into the roadmap success criteria (none of which mention the IDE by
name) and not silently absorbed into "all truths verified." A future reader
relying on this file alone learns two separate facts: the structural evidence
for the five roadmap success criteria is strong and independently
re-confirmed at current HEAD, and the IDE-opens clause of the phase's own
goal text is unproven and stands on a human waiver, not on evidence. Neither
this report nor any file in the repository states or implies that the VB6 IDE
has opened a written project.

WRT-03 stays open by design, is narrow in scope (byte-level column
indentation, not structure or values), is accurately self-described in
REQUIREMENTS.md, and does not block this pass. Four WINDOWS.md findings from
this phase remain tracked (#10, #11, #13 open; #12 fixed), none of which
undercuts a stated roadmap success criterion.

---

_Verified: 2026-09-13_
_Verifier: Claude (gsd-verifier)_
