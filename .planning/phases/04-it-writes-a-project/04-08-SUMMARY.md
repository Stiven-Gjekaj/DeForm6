---
phase: 04-it-writes-a-project
plan: 08
subsystem: write
tags: [rust, vb6, cli, path-containment, security, code-generation]

requires:
  - phase: 04-01
    provides: "write::project, WrittenProject, WrittenFile, write::model::from_report, report.rs's locked shape"
  - phase: 04-02
    provides: "write::values::format_value, the property formatter write_form calls"
  - phase: 04-03
    provides: "write_vbp, the complete .vbp writer"
  - phase: 04-04
    provides: "write_form, FormFiles, the complete .frm/.frx writer"
  - phase: 04-05
    provides: "write_cls, write_bas, the complete .bas/.cls writer"
  - phase: 04-06
    provides: "report::build, path_for_form/path_for_control/path_for_property/path_for_code, PathIssuer"
  - phase: 04-07
    provides: "write::comment::uncertainty_comments, wired through write_code_region"
provides:
  - "write::project wired to the full writers (write_vbp, write_form, write_cls, write_bas) and to report::build, closing the thin-vs-full gap every prior plan in this phase left staged"
  - "deform6-cli Command::Extract's --report and --force flags, resolve_output_dir, plan_writes, write_project (the CLI's own file-system-owning function, distinct from the library's write::project)"
  - "the containment check: every recovered file name is verified to be a direct child of the resolved, canonicalized output directory before any byte reaches disk"
affects: [04-09]

actuals:
  tokens: 17806
  tasks: 3
  commits: 4
plan_head_before: 0f1fd88

tech-stack:
  added: []
  patterns:
    - "write::project routes every item four independent sources collect (write_vbp, write_form per form, write_cls/write_bas per code object, report::build) through one shared PathIssuer and report::with_header_evidence (widened to pub(crate) for this), so a path collision or a missing evidence record is caught the same way regardless of which writer produced the item."
    - "plan_writes is a pure, filesystem-free function: it lexically normalises a joined candidate path (collapsing . and .. with no syscall, since the candidate does not exist yet to canonicalize) and compares its own parent against the resolved directory by value, never by a string prefix. This is the one place a recovered file name becomes a path, and a test drives it with names built inside the test."
    - "The whole project is built in memory (deform6::write::project) before the CLI's own write_project touches the file system at all; a refusal at any later stage (a populated directory without --force, a hostile file name) leaves the directory with zero entries, never a partial project."
  removed:
    - "write_vbp_thin, write_form_thin, write_cls_thin, write_bas_thin, and their own thin-only helpers (ThinFormOutput, RenderedProperty, children_of, write_control_block, collect_properties, position_fields, write_rendered_properties, write_font_block, name_kind_for) -- dead code once write::project called the full writers instead."

key-files:
  created: []
  modified:
    - crates/deform6/src/write/mod.rs
    - crates/deform6/src/write/vbp.rs
    - crates/deform6/src/write/frm.rs
    - crates/deform6/src/write/code.rs
    - crates/deform6/src/report.rs
    - crates/deform6/tests/extract_tracer.rs
    - crates/deform6-cli/src/main.rs
    - crates/deform6-cli/tests/cli.rs

key-decisions:
  - "All three deferred wirings the phase's own integration note named are done in this plan, not just the one this plan's own files_modified list named (crates/deform6-cli/src/main.rs, tests/cli.rs): write::project (crates/deform6/src/write/mod.rs) now calls write_vbp, write_form, write_cls, write_bas and report::build. This widens files_modified beyond the plan's own frontmatter, authorized by this session's own explicit instructions; tracked as a Rule 2 deviation below."
  - "report::with_header_evidence changed from private to pub(crate): write::project calls it directly on every item the four writers collect, not only on the model_items report::build already walks, so every item in the shipped report carries at least one evidence record (RPT-04), regardless of which of the four sources produced it."
  - "report::build's own property-level item derivation (item_for_property, walking model.forms[].controls[].properties) is not unified with the control-level items write_form's own collect_pending_lines already produces for an Undecoded or unreadable-Blob property. Both run, both are correct, and a property that earns both now earns two items at two different paths (a control path and a more specific property path) -- redundant, not wrong, and recorded as WINDOWS.md finding 13 for a future plan to unify."
  - "extract's own new failure modes (an input this run could not read, a populated output directory without --force, an unwritable report path, a recovered name that would escape) all map to Exit::Internal (5): the exit code table's own doc comment gains one prose line, not a new row, naming them."
  - "The containment check (plan_writes) is lexical, not syscall-based: std::fs::canonicalize refuses a path that does not exist yet, so a candidate that has not been written cannot be resolved that way. A hand-written component walk (collapsing . and .. without touching disk) is the one way to check a path before it exists, and its result's own parent is compared against the resolved directory by PartialEq, never by starts_with."
  - "When --report is given, the JSON report goes only to the named path, not also into the output directory: the CLI filters the .report.json entry out of the containment-checked plan for the directory and writes the report separately, unchecked for containment (the user named it directly, per T-4-23), printing its own resolved path."

patterns-established:
  - "A CLI-owned write step splits into three pure-then-effectful stages: resolve_output_dir (fs: create + canonicalize), plan_writes (pure: lexical containment check, no fs), then the actual std::fs::write loop -- so the containment logic itself is unit-testable with no temp directory needed to exist beforehand for the file names under test, only for the resolved directory itself."

requirements-completed: []

coverage:
  - id: D1
    description: "write::project calls the complete writers (write_vbp, write_form, write_cls, write_bas) and report::build instead of the thin writers every prior plan in this phase staged; the shipped report now carries real items (at least one graded inferred, with a real path) and real limits (stating full recompilation did not run), not the empty arrays every prior SUMMARY in this phase recorded"
    requirement: "WRT-01"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/extract_tracer.rs#the_written_reports_items_hold_a_real_inferred_path_and_every_item_carries_evidence"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/extract_tracer.rs#the_written_reports_limits_state_that_full_recompilation_did_not_run"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/extract_tracer.rs (all 11 pre-existing tests, including the_written_frx_equals_the_committed_source_byte_for_byte)"
        status: pass
    human_judgment: false
  - id: D2
    description: "extract gains --report and --force; every one of the subcommand's own new failure modes maps to exit code 5, with no row in the published exit code table renumbered, and a file that is not a portable executable gives the same code from extract as from inspect"
    requirement: "WRT-01"
    verification:
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#extract_on_a_file_that_is_not_a_portable_executable_gives_the_same_exit_code_as_inspect"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#extract_with_no_output_flag_exits_five_and_not_two"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#extract_with_an_unwritable_output_path_exits_five_and_names_the_path"
        status: pass
      - kind: other
        ref: "grep -cE '^//! \\| [0-5] \\|' crates/deform6-cli/src/main.rs == 6 (unchanged)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The output directory is resolved to an absolute, canonicalized path before any file is written; every file this run is about to write is checked to be a direct child of it, by comparing resolved parents, never a string prefix; a hostile name refuses the whole run and writes zero files; a run over a symbolic link writes at the real directory"
    requirement: "WRT-01"
    verification:
      - kind: unit
        ref: "crates/deform6-cli/src/main.rs#tests::a_file_name_holding_a_path_separator_refuses_the_whole_run_and_writes_nothing"
        status: pass
      - kind: unit
        ref: "crates/deform6-cli/src/main.rs#tests::a_file_name_holding_a_parent_directory_sequence_refuses_the_whole_run_and_writes_nothing"
        status: pass
      - kind: unit
        ref: "crates/deform6-cli/src/main.rs#tests::an_absolute_file_name_refuses_the_whole_run_and_writes_nothing"
        status: pass
      - kind: unit
        ref: "crates/deform6-cli/src/main.rs#tests::a_hostile_name_after_a_legitimate_one_still_writes_nothing_at_all"
        status: pass
      - kind: unit
        ref: "crates/deform6-cli/src/main.rs#tests::the_sanitized_name_type_cannot_produce_a_separator_a_parent_sequence_or_an_absolute_path"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#extract_into_a_symlinked_directory_writes_at_the_real_directory"
        status: pass
      - kind: manual_procedural
        ref: "ln -s \"$D\" \"$L\"; extract ... -o \"$L\"; ls -la \"$D\" (this plan's own <verification> step, run by hand this session)"
        status: pass
    human_judgment: false
  - id: D4
    description: "A second run into a populated output directory refuses without --force, naming the directory, and succeeds and overwrites with it; a run writes nothing anywhere in the repository working tree outside the resolved output directory it was given"
    requirement: "WRT-01"
    verification:
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#a_second_run_into_a_populated_directory_refuses_without_force_and_succeeds_with_it"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#extract_changes_no_file_anywhere_in_the_repository_working_tree"
        status: pass
    human_judgment: false
  - id: D5
    description: "All 44 corpus programs extract with exit 0, and the written file count per kind matches the count crate::vb::inspect's own Report proves for that program; the one form whose control tree refuses (Map Editor.exe's Main) still gets a .frm file and an unrecoverable report item"
    requirement: "WRT-01"
    verification:
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#all_corpus_programs_extract_with_exit_zero_and_the_written_file_count_matches_the_report"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#map_editor_writes_a_form_file_for_its_refused_form_and_the_report_marks_it_unrecoverable"
        status: pass
    human_judgment: false

duration: 1 session
completed: 2026-09-13
status: complete
---

# Phase 4 Plan 8: The extract subcommand, its containment check, and the whole-phase wiring, Summary

**`deform6 extract` now runs the complete write pipeline every prior plan in this phase staged (full `.vbp`/`.frm`/`.frx`/`.bas`/`.cls` writers plus `report::build`), resolves its output directory to an absolute canonical path before writing a byte, refuses a hostile or escaping file name with nothing written, and sweeps all 44 corpus programs with exit 0.**

## Performance

- **Duration:** 1 session
- **Started:** 2026-09-13
- **Completed:** 2026-09-13
- **Tasks:** 3 of 3
- **Files modified:** 8 (2 more than the plan's own frontmatter named; see Deviations)

## Accomplishments

- **All three deferred wirings the phase's own integration note named are done**, not staged behind this plan any further:
  1. `write::project` calls `write_vbp` (the full `.vbp` writer, plan 04-03), not `write_vbp_thin`.
  2. `write::project` calls `write_form` (the full `.frm`/`.frx` writer, plan 04-04), not `write_form_thin`. The shipped `frmFire.frm` now writes `BackColor = &H80000005&`, not a plain decimal — checked by hand this session (see "Next Phase Readiness").
  3. `write::project` calls `report::build` (plan 04-06). The shipped `.report.json` now carries real `items` (at least one graded `inferred`, with a real path, every item carrying a basis and at least one evidence record) and real `limits` (stating in plain words that full recompilation did not run and never implying the IDE opened this project) — roadmap success criterion 5, proven end to end by two new `extract_tracer.rs` tests.
- The four thin writers (`write_vbp_thin`, `write_form_thin`, `write_cls_thin`, `write_bas_thin`) and every helper that existed only to serve them are removed as dead code; `cargo clippy -D warnings` would otherwise fail on them once nothing called them.
- `Command::Extract` gains `--report <path>` (moves the JSON report to a named path, printed once written, never checked for containment since the user named it directly) and `--force` (overwrites a non-empty output directory, still checking every path for containment first).
- The containment check (`plan_writes` in `deform6-cli`) is a pure, filesystem-free function: it lexically collapses `.`/`..` in the joined candidate path (no `std::fs::canonicalize`, which refuses a path that does not exist yet) and compares its own parent against the resolved output directory by value, never a string prefix (`grep -c starts_with` is `0`). Five unit tests drive it with hostile names built inside the test — a path separator, a `..` sequence, an absolute path, a hostile name after a legitimate one, and a check that `SafeName` itself can never produce any of the three shapes — since the corpus holds none.
- A run resolves the output directory to an absolute, canonicalized path before creating it or writing a byte; a directory reached through a symbolic link resolves to the real directory (checked by hand and by a new CLI test), and a second run into a populated directory refuses without `--force`, naming the directory.
- All 44 corpus programs extract with exit 0; the written file count per kind (one `.vbp`, one `.frm` per form, one `.frx` per form holding a blob, one `.bas`/`.cls` per module and class, one report) matches the count independently read from the same program's own `Report`. Totals this session: 44 `.vbp`, 53 `.frm`, 2 `.frx`, 52 `.bas`/`.cls`, 44 `.report.json` — **195 files across all 44 programs.** `Map Editor.exe`'s own `Main` form, whose control tree walk refuses, still gets a `.frm` and an `unrecoverable` report item at `/forms/Main`.
- A repository-wide directory listing, taken before and after both the single-run tests and the whole 44-program sweep, is identical: no run wrote anywhere outside the resolved output directory it was given.

## Task Commits

Each task was committed atomically. A fourth commit, before task 1, does the write::project wiring this plan's own integration note requires (see Deviations):

0. **Wiring: write::project to the full writers and report::build** — `bc3fb4a` (feat)
1. **Task 1: The subcommand, its three flags, and the exit codes it reuses** — `7f8d270` (feat)
2. **Task 2: Resolve the directory, check every path against it, and build before you write** — `884c14c` (feat)
3. **Task 3: All 44 corpus programs, exit 0, nothing outside the directory** — `757dd1f` (test)

**Plan metadata:** commit follows this SUMMARY.

## Files Created/Modified

- `crates/deform6/src/write/mod.rs` — `write::project` wired to the full writers and `report::build`, through one shared `PathIssuer`
- `crates/deform6/src/write/vbp.rs`, `crates/deform6/src/write/frm.rs`, `crates/deform6/src/write/code.rs` — the now-dead thin writers and their own helpers removed
- `crates/deform6/src/report.rs` — `with_header_evidence` widened from private to `pub(crate)`
- `crates/deform6/tests/extract_tracer.rs` — two new tests proving the shipped report's own items and limits
- `crates/deform6-cli/src/main.rs` — `Command::Extract`'s `--report`/`--force` flags, `resolve_output_dir`, `plan_writes`, `write_project`, `lexically_normalize`, `resolve_report_path`, `ensure_directory_is_writable`, and the containment unit tests
- `crates/deform6-cli/tests/cli.rs` — the flag/exit-code tests, the force-flag test, the repository-wide read-only test, the symlink test, and the 44-program sweep

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential:

1. **All three deferred wirings are done in this plan, widening `files_modified` beyond the plan's own frontmatter.** The plan's own frontmatter names only `crates/deform6-cli/src/main.rs` and `crates/deform6-cli/tests/cli.rs`, but every prior plan's own SUMMARY in this phase (04-01 through 04-07) named this plan as the place `write::project` would be switched over to the full writers, and this session's own instructions were explicit and detailed about it. Doing the CLI's own flags without this wiring would have shipped `extract` with the flags this plan promises but the thin, unwired writers underneath — passing every one of this plan's own literal acceptance criteria while failing the roadmap's own success criterion 5 and the phase's own stated purpose.
2. **`report::build`'s own property-level item derivation is not unified with what `write_form` already collects.** Both are correct, both carry a real path and evidence, and a property that earns both now shows up twice in the report at two different granularities. Unifying them would mean threading every writer's own collected items back into `report::build` (or vice versa), which is a real design change beyond a wiring pass; recorded as WINDOWS.md finding 13 rather than attempted under time pressure or hidden.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `write::project` was not wired to the full writers or `report::build`, and this plan's own `files_modified` did not name the file that needed it**
- **Found during:** Before Task 1, reading this session's own integration note and all seven prior plans' SUMMARYs
- **Issue:** `crates/deform6/src/write/mod.rs`'s `project` function still called `write_vbp_thin`, `write_form_thin`, `write_cls_thin` and `write_bas_thin`, and built `ProjectReport` with `items: Vec::new()` and `limits: Vec::new()`, exactly as plan 04-01 left it. Every one of plans 04-02 through 04-06's own SUMMARYs named this plan as the place this gets fixed (WINDOWS.md findings 10 and 12). Giving `extract` its `--report`/`--force` flags without this wiring would ship a subcommand whose own report never satisfies roadmap success criterion 5, and whose own `.frm` output never reflects the phase's own accumulated work.
- **Fix:** `write::project` now calls `write_vbp`, `write_form` (once per form, zipped with `Report::forms`' own defects), `write_cls`/`write_bas` (once per `Class`/`Module` object, zipped with `Report::objects`), and `report::build`. Every item any of these four sources returns with an empty path gets that writer's own default path (`META_PATH` for `.vbp`, `path_for_form` for a form, `path_for_code` for a module or class), then backfilled evidence via `report::with_header_evidence` (widened from private to `pub(crate)` for this call), then a final path from one `PathIssuer` shared across the whole run. The four thin writers and their own thin-only helpers are removed as dead code.
- **Files modified:** `crates/deform6/src/write/mod.rs`, `crates/deform6/src/write/vbp.rs`, `crates/deform6/src/write/frm.rs`, `crates/deform6/src/write/code.rs`, `crates/deform6/src/report.rs`, `crates/deform6/tests/extract_tracer.rs`
- **Verification:** All 11 pre-existing `extract_tracer.rs` tests still pass unchanged (including the `.frx` byte-for-byte match); two new tests (`the_written_reports_items_hold_a_real_inferred_path_and_every_item_carries_evidence`, `the_written_reports_limits_state_that_full_recompilation_did_not_run`) pass; the full gate passes; `BackColor` in the shipped `frmFire.frm` reads `&H80000005&` by hand-run this session.
- **Committed in:** `bc3fb4a`

---

**Total deviations:** 1 auto-fixed (missing critical functionality, spanning six files beyond this plan's own two-file `files_modified` list). **Impact:** Necessary for the plan's own stated purpose and the phase's own roadmap success criterion 5; without it, this plan's own CLI-level work would have shipped flags over a report and a form writer that still produced the thin, staged output every prior plan in this phase left in place. No unrelated scope creep: the wiring touches only the four writer modules and the report module this phase itself built.

## Issues Encountered

None beyond the deviation above, resolved before any task commit.

## TDD Gate Compliance

Every task, and the write::project wiring commit, carries a genuine RED-observed-and-reverted cycle, run this session and documented in each commit message. None was committed as a separate failing-tests commit, per this project's own `AGENTS.md`: the gate runs `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` before every commit with no exception, and code and its tests share one commit — a committed RED state would fail both.

| Commit | RED observed | GREEN commit | REFACTOR | Status |
|--------|--------------|--------------|----------|--------|
| Wiring | `project_report`'s `items`/`limits` hard-coded back to empty `Vec`s: the two new report-content tests failed with "write::project must no longer ship items: []" / "... limits: []" | `bc3fb4a` | none needed | Pass |
| Task 1 | The reading-step refusal mapped unconditionally to `Exit::Internal` instead of through `exit_for`: `extract_on_a_file_that_is_not_a_portable_executable_gives_the_same_exit_code_as_inspect` failed, `left: 5, right: 1` | `7f8d270` | none needed | Pass |
| Task 2 | The parent-equality check short-circuited to always pass (`if false && parent != resolved_dir`): `a_file_name_holding_a_parent_directory_sequence_refuses_the_whole_run_and_writes_nothing` failed, `left: Ok, right: Internal` — the run would have written `escape.frm` one directory above the resolved output directory. The stray file this created in the OS temp directory was found and removed before restoring the check. | `884c14c` | none needed | Pass |
| Task 3 | Two separate reverts: `CORPUS_EXECUTABLE_COUNT` dropped to 43 (failed naming both numbers before touching a program); `forms_holding_a_blob` hard-coded to `0` (failed on the first program that actually holds one, `SubReality_WinsockSample.exe`, naming the program, the wrong count and the real one) | `757dd1f` | none needed | Pass |

## User Setup Required

None — no external service configuration required.

## Known Stubs

None new. The three staged gaps every prior plan in this phase left (`write::project` calling the thin writers; `report::build` never wired in) are exactly what this plan closes. Two pre-existing, out-of-scope gaps remain, both already recorded and both explicitly untouched by this plan:

- `write_vbp` never writes a `Reference=` line (WINDOWS.md finding 10): `Report` carries no field for the header's type library reference table.
- `write::model::from_report`'s own generated-control-array-index item still uses a literal `/forms/*/controls/<name>` path (WINDOWS.md finding 11): `write/model.rs` is off-limits to this plan, matching every prior plan's own scoping.
- New this plan: `report::build`'s own property-level item derivation is not unified with the items `write_form` already collects for the same fact (WINDOWS.md finding 13, recorded above).

## Threat Flags

None. This plan's own `<threat_model>` already names T-4-01 and T-4-02 as the surface this plan closes, and both are mitigated: T-4-01 by `SafeName` (already closed structurally, reused unchanged) and T-4-02 by `plan_writes`'s own containment check, proven with hostile names built inside the test since the corpus holds none.

## Next Phase Readiness

- Plan 04-09 (the structural check, corpus-wide, reusing `tests/support/frm.rs`/`support/vbp.rs`) now runs against the real, fully-wired `extract` output — every `.frm`/`.vbp`/`.bas`/`.cls` this plan's own sweep test wrote is the genuine output of the complete pipeline, not the thin stand-in every prior plan's own test suite exercised in isolation.
- The one visible proof the phase's own integration note asked for: `cargo run -p deform6-cli -- extract corpus/vb6-code/Fire-effect/Fast_Flames.exe -o "$(mktemp -d)"` now writes `BackColor       =   &H80000005&` in `frmFire.frm`, checked by hand this session. Before this plan, the shipped tool wrote a plain signed decimal.
- `WRT-01` was already marked complete in `REQUIREMENTS.md` by plan 04-01's own (premature) `requirements-completed` list; `04-09` also declares it in its own frontmatter, so the shared-ID gate correctly reports `0/1 ready` for this plan's own attempt to mark it again — no action taken, and REQUIREMENTS.md is left exactly as it was.
- No blocker for plan 04-09. The one open architectural note (report::build's own property items not unified with the writers' own) is recorded, not hidden, and does not affect determinism, correctness, or any of this phase's own roadmap success criteria.

---
*Phase: 04-it-writes-a-project*
*Completed: 2026-09-13*

## Self-Check: PASSED

- All 8 modified files exist on disk (verified with `[ -f ]`, listed above).
- Commits `bc3fb4a`, `7f8d270`, `884c14c`, `757dd1f` all exist in `git log --oneline --all`.
- `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` passes: 836 tests total across the workspace, 0 failures.
- `cargo test -p deform6 --test extract_tracer` passes: 13 tests (11 pre-existing plus 2 new), the written `.frx` still byte-identical to the committed corpus source.
- `cargo test -p deform6-cli` passes: 5 unit tests in `main.rs`, 38 integration tests in `tests/cli.rs`, including the 44-program sweep.
- Source assertions re-run: `grep -cE '^//! \| [0-5] \|' crates/deform6-cli/src/main.rs` gives `6` (unchanged); `grep -c starts_with` (non-comment) gives `0`; `grep -c canonicalize` (non-comment) gives `3`; `grep -c exit_for` (non-comment) gives `3`.
- `test 44 -eq "$(find corpus -iname '*.exe' | wc -l | tr -d ' ')"` passes.
- Manual verification re-run: `ln -s "$D" "$L"; extract ... -o "$L"` writes all five files into `$D`, confirmed by `ls -la "$D"` this session.
- Manual verification re-run: `extract corpus/vb6-code/Fire-effect/Fast_Flames.exe -o "$(mktemp -d)"` writes `BackColor       =   &H80000005&` in `frmFire.frm`.
- The pre-existing staged deletion (`.planning/phases/01-it-reads-the-file/VERIFICATION.md`), the `.planning/config.json` modification, and the untracked `.gsd/`, `Notes/`, `.planning/milestone.lock` paths are untouched by any of this plan's four commits (`git status --short` before and after this plan's commits shows the identical set).
