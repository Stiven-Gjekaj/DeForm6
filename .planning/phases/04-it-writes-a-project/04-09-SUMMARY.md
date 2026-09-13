---
phase: 04-it-writes-a-project
plan: 09
subsystem: testing
tags: [rust, vb6, integration-test, independent-reader, pinned-ratio, tdd]

requires:
  - phase: 04-01
    provides: "write::project, WrittenProject, WrittenFile, the whole write module set"
  - phase: 04-02
    provides: "write::values::format_value, FormattedValue, the property formatting rules this plan's own property-count function reuses"
  - phase: 04-03
    provides: "write_vbp, the complete .vbp writer this plan reads back"
  - phase: 04-04
    provides: "write_form, the complete .frm/.frx writer, the BlobCursor offset convention this plan verifies from outside"
  - phase: 04-05
    provides: "write_cls, write_bas, the .bas/.cls writer this plan reads back"
  - phase: 04-06
    provides: "report::build, the limits list, the exact recompilation sentence this plan asserts verbatim"
  - phase: 04-08
    provides: "the fully wired extract command whose real output (not a thin stand-in) this plan's own extraction now exercises"
  - phase: 02-the-object-graph
    provides: "tests/ratios.toml, the two failure words, the rewriter this plan extends rather than replaces"
  - phase: 03-forms
    provides: "the form and control pin keys this plan's write side keys sit beside"
provides:
  - "crates/deform6/tests/extract_structural.rs: the structural recompilation check, run over all 44 corpus programs, reading every written file back through tests/support/frm.rs and tests/support/vbp.rs alone"
  - "the six named structural assertions the roadmap's own success criterion 4 lists, each independently testable against a hand-built fixture"
  - "the resource offset resolution check: every .frx offset a written .frm names is seeked in the matching .frx and its own record header verified to end inside the file"
  - "the whole tree encoding sweep: no byte order mark, no bare line feed, the file's own last two bytes are CRLF, no leaked UTF-8 character above plain ASCII"
  - "property_declared and property_written, two new keys per program in tests/ratios.toml, and the property_counts measuring function both the gate and the rewriter read"
affects: [05-hostility, 06-the-readme-and-the-release]

actuals:
  tokens: 20700
  tasks: 3
  commits: 4
plan_head_before: 40930181f9c4997867d200043a68809329702835

tech-stack:
  added: []
  patterns:
    - "extract_structural.rs's only reach into deform6::write is the entry point, deform6::write::project; every fact it checks about the files that entry point returns comes from tests/support/frm.rs and tests/support/vbp.rs, never from deform6::write::frm/vbp/code/values/comment, proven by a source grep run before every commit."
    - "Each of the roadmap's six named structural rules is its own pure function (check_components_name_existing_files, check_startup_names_a_declared_form, check_control_identifiers, check_nesting_depth, check_property_order, check_menus_last) taking already-parsed independent-reader types, so each one is independently unit-testable against a hand-built fixture without re-running the whole corpus."
    - "The alphabetical-order check skips a block whose own class is the literal VB.Control, the one class name an external (OCX) control's Begin line carries: this mirrors write::frm's own is_external sort skip as a written fact (the class name), never as shared code."
    - "property_counts(key, data, table) in tests/ratios.rs measures both the declared and the written count from one run of deform6::inspect and deform6::write::model::from_report, so crates/xtask's rewriter and the test gate can never silently diverge into two arithmetics (the same discipline program_counts and forms_controls_counts already established)."
    - "A blob's own written-or-not decision is measured by reproducing write::frm::append_blob's own bound check (offset..offset+4+declared_len fits inside the executable's bytes) rather than assuming every Resource decision becomes a line: an over-length Text property under the same FormattedValue::Resource variant is correctly counted as never written."

key-files:
  created:
    - crates/deform6/tests/extract_structural.rs
  modified:
    - crates/deform6/tests/ratios.rs
    - crates/xtask/src/main.rs
    - tests/ratios.toml
    - .planning/REQUIREMENTS.md

key-decisions:
  - "The structural check's alphabetical-order and menus-last assertions read the same tests/support/frm.rs Block type differential.rs already trusts, never a second reader: independence is enforced by a source grep (grep -vE comment lines | grep -cE deform6::write::(frm|vbp|code|values|comment) must be 0), run before every commit in this plan."
  - "The text-file-count cross-check compares the write side's own file list against an independent tally built from deform6::vb::Report (one .vbp, one file per declared form, one per non-form object) computed before write::project ever ran, rather than against write::project's own output a second time, so the check is a real cross-check and not a tautology."
  - "The leaked-UTF-8 encoding check asserts that a written text file, if it happens to validate as UTF-8 at all, holds only plain ASCII: a genuine Windows-1252 byte for a character above U+007F (a single byte such as 0xA9) is not a legal UTF-8 lead byte on its own and almost never validates, so a file that does validate and holds a non-ASCII character is the loud, breakable sign the encoder was bypassed. This is a chosen formalisation, not a corpus-proven rule, and is documented as such in the check's own doc comment."
  - "WRT-04 (alphabetical order, menus last) is marked complete: this plan's own check_property_order and check_menus_last prove it corpus-wide, over every control block this session's own extraction actually wrote for all 44 programs, not only the hand-built fixtures plan 04-04 exercised. WRT-03 (the byte-level column layout: 3-space indent, 16-column name pad, three spaces after =, the exact Begin/End trailing-space rule) is left unmarked on purpose: tests/support/frm.rs's own Block parser trims every line by its own design (\"this parser does not depend on the exact indentation\"), so nothing in this plan's structural check can serve as new, corpus-wide evidence for a byte-level column claim. WRT-03 stays proven only by plan 04-04's own targeted unit tests against real corpus samples. Marking it here would repeat the exact requirements-tracking overstatement 03-VERIFICATION.md and this project's own review history have already caught once."
  - "The write side property pin is split into two commits, code+data then the header prose, per this repository's own rule that documentation goes in its own commit: the header paragraph explaining what property_declared/property_written measure was added to the HEADER constant, and tests/ratios.toml regenerated a second time, producing a diff confined to the header comment alone (verified: the second regeneration's diff touched no program entry)."
  - "check_program, recovered_mismatch_message, declared_mismatch_message and check_program_forms_controls all now thread a freshly measured PropertyCounts into their own paste-block builders, so a MOVED UP or REGRESSION message for any pinned key always prints a paste block with every field freshly measured, not a mix of fresh and stale pinned values."

patterns-established:
  - "A structural/independent-reader test file names its own local copies of any roadmap-stated numeric limit (MAX_LEGAL_NAME_LEN = 40, MAX_LEGAL_NESTING_DEPTH = 7) rather than importing the writer's own constants, so the check proves the roadmap's own number, never merely that the writer agrees with itself."

requirements-completed: [WRT-04]

coverage:
  - id: D1
    description: "The structural check reads every file this phase writes, for all 44 corpus programs, back through tests/support/frm.rs and tests/support/vbp.rs alone, and asserts all six of the roadmap's own structural rules (component lines name existing files, Startup= names a declared form, every control/class name is a legal identifier of 40 characters or fewer, nesting depth is 7 or less, properties are case-insensitively alphabetical, every menu comes after every other control)."
    requirement: "WRT-04"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/extract_structural.rs#the_structural_check_passes_for_all_forty_four_corpus_programs"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/extract_structural.rs#deliberate_breakages::a_component_line_naming_a_missing_file_fails_the_component_check"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/extract_structural.rs#deliberate_breakages::a_reversed_property_order_fails_the_alphabetical_check"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/extract_structural.rs#deliberate_breakages::a_menu_before_a_non_menu_sibling_fails_the_menus_last_check"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/extract_structural.rs#deliberate_breakages::a_resource_offset_shifted_by_one_byte_fails_the_offset_resolution_check"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every .frx offset a written .frm names is seeked in the matching, actually-written .frx; the record header there is read and its own end asserted inside the file, and the last record is asserted to end exactly at the file's own end."
    requirement: "WRT-01"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/extract_structural.rs#the_structural_check_passes_for_all_forty_four_corpus_programs"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every written text file (excluding .frx and .report.json) holds no byte order mark, no bare line feed, ends with CRLF, and holds no leaked UTF-8 character above plain ASCII; the report's own limits list states, in the same words the check's own constant holds, that full recompilation did not run."
    requirement: "WRT-06"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/extract_structural.rs#the_structural_check_passes_for_all_forty_four_corpus_programs"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/extract_structural.rs#the_reports_limits_state_the_exact_recompilation_sentence"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/extract_structural.rs#deliberate_breakages::a_file_beginning_with_a_byte_order_mark_fails_the_encoding_sweep"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/extract_structural.rs#deliberate_breakages::a_bare_line_feed_fails_the_encoding_sweep"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/extract_structural.rs#deliberate_breakages::a_byte_above_the_windows_1252_range_fails_the_encoding_sweep"
        status: pass
    human_judgment: false
  - id: D4
    description: "tests/ratios.toml gains property_declared and property_written for all 44 programs (807 records recovered, 136 lines written, 26 distinct ratio values), the rewriter and the gate read one measuring function, and both failure directions were run by hand and reverted."
    verification:
      - kind: integration
        ref: "crates/deform6/tests/ratios.rs#the_properties_gate_passes_on_the_committed_file"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/ratios.rs#raising_a_pinned_property_declared_count_fails_with_regression"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/ratios.rs#lowering_a_pinned_property_written_count_fails_with_moved_up"
        status: pass
      - kind: other
        ref: "cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml"
        status: pass
    human_judgment: false

duration: 1 session
completed: 2026-09-13
status: complete
---

# Phase 4 Plan 9: The structural recompilation check, and the ratios re-pinned Summary

**A second, independent reader (tests/support/frm.rs, tests/support/vbp.rs) now reads every file this whole phase writes back, for all 44 corpus programs, and proves six structural rules and a resource-offset invariant the writer itself cannot certify; the write side's own property coverage (807 records recovered, 136 lines written) is pinned in tests/ratios.toml alongside the existing recovery ratios.**

## Performance

- **Duration:** 1 session
- **Started:** 2026-09-13
- **Completed:** 2026-09-13
- **Tasks:** 3 of 3
- **Files modified:** 4 (1 created, 3 modified) plus REQUIREMENTS.md

## Accomplishments

- **`crates/deform6/tests/extract_structural.rs` is new** and imports nothing from `deform6::write::frm`, `deform6::write::vbp`, `deform6::write::code`, `deform6::write::values` or `deform6::write::comment` (a source grep proves this before every commit). Its only reach into the writing side of the library is `deform6::write::project`, the entry point. It runs the real write path over all 44 corpus programs, writes every file to a fresh temporary directory, and reads every one back through `tests/support/frm.rs` and `tests/support/vbp.rs` alone — the same readers `differential.rs` already trusts for the original corpus.
- **Six named assertions**, each its own function, prove the roadmap's own success criterion 4 list: every `Form=`/`Module=`/`Class=` line names a file that exists; `Startup=` names a form a `Form=` line brings in; every control and class name is a legal VB6 identifier of 40 characters or fewer; nesting depth is 7 or less; properties inside a block are case-insensitively alphabetical (skipping the generic `VB.Control` class an external OCX control writes, matching the writer's own skip); every menu comes after every other control.
- **A separate resource-offset check** parses every `.frx` offset a written `.frm` names, seeks it in the matching, actually-written `.frx`, reads the four byte length header there, and asserts the record ends inside the file — and that the last (highest-offset) record ends exactly at the file's own end.
- **The whole tree encoding sweep** (task 2) checks every written text file (excluding `.frx`, binary by kind, and `.report.json`, this phase's own JSON output) for no byte order mark, no bare line feed, a CRLF ending, and no leaked UTF-8 character above plain ASCII — the sign the Windows-1252 encoder was bypassed. Its own file count is cross-checked against an independent tally built from the read side's own `Report`.
- **The recompilation statement is asserted verbatim**: a dedicated test holds this check's own independent copy of the sentence stating that full recompilation did not run, and asserts the shipped report's own first limit line equals it exactly, so the two can never silently drift apart. Neither this file nor `crates/deform6/src/report.rs` anywhere states that the IDE opened, loaded or compiled the project (a source grep proves this too).
- **Every one of the plan's own four named deliberate breakages was run and reverted by hand this session**, watched failing before being committed: deleting a component line, reversing a block's property order, moving a menu before a non-menu child, and shifting a resource offset by one byte. Three more (a byte order mark, a bare line feed, a leaked UTF-8 character) were run the same way for task 2. Two further breakages (an illegal control name, a control nested past depth seven) were run as extra coverage beyond the plan's own four.
- **`tests/ratios.toml` gains `property_declared` and `property_written`** for all 44 programs, extending the existing pin mechanism rather than adding a second one. Totals this session: 807 property records recovered against 136 property lines written, across 26 distinct ratio values. A second `cargo run -p xtask -- update-ratios` produces no diff. Both failure directions were run by hand against `Grayscale-effect/Grayscale.exe` and reverted by the rewriter, not by hand.
- **The full gate passes**: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` — 866 tests total across the workspace (up from 836 before this plan).

## Task Commits

Each task was committed atomically. Task 3 split into two commits per this repository's own rule that documentation goes in its own commit:

1. **Task 1: Read the written tree back through the independent reader** — `c520766` (test)
2. **Task 2: The whole tree encoding sweep, and the statement that recompilation did not run** — `2f69380` (test)
3. **Task 3a: Pin the write side numbers** — `4b4ec28` (feat)
4. **Task 3b: Explain the two new keys in the pin's own header** — `3f57de4` (docs)

**Plan metadata:** commit follows this SUMMARY.

## Files Created/Modified

- `crates/deform6/tests/extract_structural.rs` — the structural recompilation check (new)
- `crates/deform6/tests/ratios.rs` — `PinnedEntry`, `parse_ratios_toml`, `format_entry`, `HEADER` extended with `property_declared`/`property_written`; `PropertyCounts`, `property_counts`, `property_lines_written`, `blob_range_fits`, `check_program_properties` added; `check_program`, `recovered_mismatch_message`, `declared_mismatch_message`, `check_program_forms_controls` and `gate_failures` all thread the new measurement through
- `crates/xtask/src/main.rs` — `Measured` gains the two new fields; `measure_all` and `render` thread them through `ratios::property_counts`/`ratios::format_entry`
- `tests/ratios.toml` — 44 entries gain `property_declared`/`property_written`; header comment explains what the two keys measure
- `.planning/REQUIREMENTS.md` — WRT-04 marked complete

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential:

1. **WRT-03 stays unmarked.** This plan's own frontmatter lists WRT-03 as a requirement, but `tests/support/frm.rs`'s own `Block` parser deliberately trims every line and ignores exact indentation, so nothing this plan's structural check does can serve as new, corpus-wide evidence for WRT-03's byte-level column claims (3-space indent, 16-column name pad, three spaces after `=`, the exact `Begin`/`End` trailing-space rule). Those claims remain proven only by plan 04-04's own targeted unit tests. Marking WRT-03 complete here, on evidence this plan does not actually produce, would repeat the exact requirements-tracking overstatement this project's own review history has already caught and corrected once (03-VERIFICATION.md).
2. **The write side property pin measures a run's own internal coverage, not a recovery against source.** `property_declared`/`property_written` compare the write path against itself (how much of what the read side recovered the writer then managed to emit), unlike every other pair in `tests/ratios.toml`, which compares the tool against the original source. The header comment states this distinction explicitly, in the same words in both the file and the `HEADER` constant that generates it, so a future reader cannot mistake a coverage number for a recovery number.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `crates/xtask/src/main.rs`'s own embedding of `ratios.rs` required the property-counting function to be reachable without a private `Program` type**
- **Found during:** Task 3, wiring `crates/xtask`'s own `measure_all` to the new property counts
- **Issue:** `tests/ratios.rs`'s own `Program` struct (holding a corpus program's key and bytes) is module-private, unreachable from `crates/xtask/src/main.rs` once `ratios.rs` is embedded via `#[path]` into the `xtask` crate — private items are visible to a module's descendants, never to its ancestors, and `main.rs` is `ratios`'s parent module once embedded. A `property_counts_for(program: &Program, ...)` signature therefore could not be called from `xtask`.
- **Fix:** Split the function in two: `pub(crate) fn property_counts(key: &str, data: &[u8], table: &OpcodeTable) -> PropertyCounts`, reachable from `xtask` the same way `declared_total` and `format_entry` already are, and a private `property_counts_for(program: &Program, ...)` wrapper this file's own tests use.
- **Files modified:** `crates/deform6/tests/ratios.rs`, `crates/xtask/src/main.rs`
- **Verification:** `cargo check -p xtask -p deform6 --tests` compiles clean; `cargo test -p xtask` and `cargo test -p deform6 --test ratios` both pass.
- **Committed in:** `4b4ec28`

**2. [Rule 1 - Bug] A `clippy::collapsible_match` failure in the first draft of `property_lines_written`**
- **Found during:** Task 3, running the full gate before the first commit
- **Issue:** A nested `if`/`else` inside a `match` arm on `PropertyValue::Blob` tripped `clippy::collapsible_match`, which `-D warnings` promotes to a hard failure.
- **Fix:** Replaced the nested `if`/`else` with a match guard (`PropertyValue::Blob { .. } if blob_range_fits(...) => 1`), matching the exhaustive-match-with-guard style already used elsewhere in this crate.
- **Files modified:** `crates/deform6/tests/ratios.rs`
- **Verification:** `cargo clippy --all-targets -- -D warnings` passes clean.
- **Committed in:** `4b4ec28`

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug). **Impact:** Both necessary to make the plan's own task 3 compile and pass the gate. No scope creep: neither touches a file outside this plan's own `files_modified`.

## Issues Encountered

None beyond the two deviations above, both resolved before their own task's commit.

## TDD Gate Compliance

Every task carries a genuine RED-observed-and-reverted cycle, run this session and documented in each commit message. None was committed as a separate failing-tests commit, per this project's own `AGENTS.md`: the gate runs `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` before every commit with no exception, and code and its tests share one commit — a committed RED state would fail both.

| Commit | RED observed | GREEN commit | REFACTOR | Status |
|--------|--------------|--------------|----------|--------|
| Task 1 | Four functions neutered in turn (`check_components_name_existing_files`, `check_property_order`, `check_menus_last`, `check_resource_offsets`), each returning no failures: observed "a component line naming a file that does not exist must fail the check", "reversed property order must fail the check", "a non-menu control after a menu control must fail the check", "an offset shifted by one byte must fail the resolution check: []" | `c520766` | none needed | Pass |
| Task 2 | `check_text_file_encoding` neutered to always return no failures: observed three failures at once (BOM, bare line feed, leaked UTF-8 test names), each with its own assertion message | `2f69380` | none needed | Pass |
| Task 3a | The committed `tests/ratios.toml` genuinely lacked the two new keys before this commit: `cargo test -p deform6 --test ratios` failed with "this corpus program has no entry in tests/ratios.toml" for all 44 programs, because `parse_ratios_toml`'s own tuple pattern could not build an entry missing `property_declared`/`property_written`. `cargo run -p xtask -- update-ratios` then wrote real numbers and the gate turned green. | `4b4ec28` | none needed | Pass |
| Task 3b | Not applicable: a documentation-only commit, verified by diffing that the rewrite touched only the header comment | `3f57de4` | none needed | Pass |

Both pinned-ratio failure directions were run by hand against `Grayscale-effect/Grayscale.exe` and recorded verbatim, per this task's own acceptance criteria:

- Editing `property_declared` up by one: `"vb6-code/Grayscale-effect/Grayscale.exe: REGRESSION: the pin claims 23 property_declared, the tool measures 22. Paste this block into tests/ratios.toml:..."`
- Editing `property_written` down by one: `"vb6-code/Grayscale-effect/Grayscale.exe: MOVED UP: the pin claims 6 property_written, the tool measures 7. Paste this block into tests/ratios.toml:..."`

The file was restored both times by `cargo run -p xtask -- update-ratios`, never by hand, and a rerun of the gate passed.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None new. This plan closes the phase's own last staged gap (the structural check itself, and the write side's own coverage pin).

## Roadmap Success Criteria — Criterion by Criterion

The roadmap names five success criteria for Phase 4. This plan's own job was criteria 2, 3 and 4; criteria 1 and 5 were already proven by earlier plans in this phase and are restated here, not re-derived, so this table is complete rather than partial:

| # | Criterion | Proven by | This plan's contribution |
|---|-----------|-----------|---------------------------|
| 1 | `extract` writes one `.vbp`, one `.frm` per form, one `.frx` per form holding a blob, one `.bas`/`.cls` per module and class, one JSON report; exits 0; writes nothing outside `out/` | Plan 04-08's own corpus-wide sweep (`all_corpus_programs_extract_with_exit_zero_and_the_written_file_count_matches_the_report`, `extract_changes_no_file_anywhere_in_the_repository_working_tree`) | Not re-derived. This plan's own text-file-count cross-check (task 2) adds an independent second tally of the same file-count fact, over the same 44 programs, from the read side's `Report` rather than from `write::project`'s own output. |
| 2 | Every text file holds Windows-1252 bytes with CRLF on every line, including the last, and no byte order mark | This plan, task 2 | New: the encoding sweep, run over every written text file across all 44 programs, checks no BOM, no bare line feed, a CRLF ending, and (as a corpus-scale proxy for "Windows-1252, not UTF-8") no leaked UTF-8 character above plain ASCII. It does not independently re-derive the encoder's own byte-value correctness (that a specific character maps to a specific byte); that remains `write::model::encode_windows_1252`'s own unit-tested contract. |
| 3 | Every `.frx` offset resolves inside the `.frx` that was actually written, checked through `support/frm.rs`, for all 44 programs | This plan, task 1 | New: the resource-offset check, over both forms that carry a blob in this corpus (`frmFire`, and the one form in `SubReality_WinsockSample.exe`), reads the four-byte length header at every declared offset and asserts every record ends inside the file, and the last one ends exactly at the file's own end. |
| 4 | The structural check passes: component lines name existing files, `Startup=` names a declared form, legal identifiers of 40 characters or fewer, nesting depth 7 or less, alphabetical properties, menus last | This plan, task 1 | New: all six named assertions, each independently unit-tested against a hand-built fixture and run corpus-wide. |
| 5 | `jq` query for inferred items returns paths; every item carries a `basis` and an `evidence` record with a byte offset; `confidence` is one of three words, never a number; two runs give byte-identical reports | Plans 04-06 and 04-08 (`a_query_for_inferred_items_returns_paths_for_fast_flames_exe`, `every_item_in_a_built_report_has_a_non_empty_basis_and_at_least_one_evidence_record`, `confidence_serialises_to_the_three_lower_case_words`, `the_written_reports_items_hold_a_real_inferred_path_and_every_item_carries_evidence`, `serialising_a_built_report_twice_gives_two_byte_identical_strings`) | Not re-derived. This plan's own dedicated test (`the_reports_limits_state_the_exact_recompilation_sentence`) adds one further, narrower fact within the same report: the limits list's own first line is asserted verbatim against this check's own independent copy. |

**What this check cannot do, stated plainly:** full recompilation did not run anywhere in this session. It needs the Visual Basic 6 IDE on a Windows host, and this sandbox has neither. The structural check is what ran instead, and its own module doc comment, the shipped report's own limits list, and this SUMMARY all say so in the same words. Nothing in this repository states that the IDE opened, loaded or compiled any project (a source grep over `extract_structural.rs` and `report.rs` proves this before every commit).

## Next Phase Readiness

- Phase 4 is now fully closed: all nine plans executed, every roadmap success criterion for the phase is stated above with what proves it and what does not, and the full gate passes with 866 tests.
- `WINDOWS.md` findings 10, 11 and 13 (the missing `Reference=` line, the literal `/forms/*/controls/<name>` path in a generated-index report item, and the unreconciled property-item derivation between `write_form` and `report::build`) remain open. This plan does not touch any of them; none blocks a roadmap success criterion.
- WRT-03 remains the one open write-side requirement in `REQUIREMENTS.md`, for the reason stated above: it needs a check that reads raw `.frm` bytes directly (not through the whitespace-trimming independent `Block` parser) to close honestly. That is future work, not part of this plan.
- Phase 5 ("Hostility") can proceed; nothing in this plan introduces a new dependency or a new open question for it.

---
*Phase: 04-it-writes-a-project*
*Completed: 2026-09-13*

## Self-Check: PASSED

- `crates/deform6/tests/extract_structural.rs` exists (verified with `[ -f ]`).
- `crates/deform6/tests/ratios.rs`, `crates/xtask/src/main.rs`, `tests/ratios.toml`, `.planning/REQUIREMENTS.md` all exist and hold the stated changes.
- Commits `c520766`, `2f69380`, `4b4ec28`, `3f57de4` all exist in `git log --oneline --all`.
- `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` passes: 866 tests total across the workspace, 0 failures.
- `cargo test -p deform6 --test extract_structural` passes: 22 tests.
- `cargo test -p deform6 --test ratios` passes: 44 tests.
- `cargo test -p xtask` passes: 56 tests.
- Source assertions re-run: `grep -vE '^\s*//|^\s*//!' crates/deform6/tests/extract_structural.rs | grep -cE 'deform6::write::(frm|vbp|code|values|comment)'` gives `0`; `grep -rlE 'the IDE (opened|loaded|compiled) the project' crates/deform6/tests/extract_structural.rs crates/deform6/src/report.rs | wc -l` gives `0`; `grep -c 'mod support' crates/deform6/tests/extract_structural.rs` gives `1`.
- `test 44 -eq "$(find corpus -iname '*.exe' | wc -l | tr -d ' ')"` passes.
- `test 44 -eq "$(grep -c '^property_declared = ' tests/ratios.toml)"` and the same for `property_written` both pass.
- `cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml` passes: a rewrite on a clean tree produces no diff.
- The pre-existing staged deletion (`.planning/phases/01-it-reads-the-file/VERIFICATION.md`), the untracked `.gsd/`, `Notes/`, `.planning/milestone.lock` paths are untouched by any of this plan's four commits (`git status --short` before and after this plan's commits shows the identical set, aside from this plan's own intentional `.planning/REQUIREMENTS.md` change).
