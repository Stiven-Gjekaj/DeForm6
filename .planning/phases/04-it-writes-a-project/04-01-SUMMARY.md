---
phase: 04-it-writes-a-project
plan: 01
subsystem: write
tags: [rust, serde_json, vb6, code-generation, safe-name-sanitization]

requires:
  - phase: 03-forms
    provides: the whole read side (Report, FormReport, ControlReport, PropertyValue, BlobCursor) this plan writes from
provides:
  - "deform6::write::project — the whole write path, Report + executable bytes to in-memory project files"
  - "deform6::write::model::SafeName / SafeNameIssuer — the one way a recovered name becomes a file path, with a fault list and a collision suffix"
  - "deform6::write::model::ProjectModel and its whole model tree (FormModel, ControlModel, CodeModel, ProcedureModel, BlobRef)"
  - "deform6::report::{ProjectReport, ReportItem, Confidence, Evidence} — the locked JSON report shape"
  - "deform6-cli Command::Extract / run_extract"
affects: [04-02, 04-03, 04-04, 04-05, 04-06, 04-07, 04-08, 04-09]

actuals:
  tokens: 28285
  tasks: 3
  commits: 5
plan_head_before: 2f4ab9cbf270a3d08348f509c6d8b81efa996fee

tech-stack:
  added: [serde_json 1.0 (workspace dependency, verified OK against the crates.io registry and the package-legitimacy seam)]
  patterns:
    - "SafeName / SafeNameIssuer: a per-name sanitizer plus a per-run, ordered-Vec-backed collision registry, never a HashMap, so RPT-01 determinism holds"
    - "One BlobCursor per form, called by the writer at property-emission time, never reused from the read-time cursor: the writer's own future output order (post-04-04 alphabetization) can differ from the property stream's own read order"
    - "ProjectModel as an inert, fully-tested data model built by from_report, not yet wired into the actual writers (vbp.rs/frm.rs/code.rs still call SafeName::new independently) — wave 2 plans (04-02 through 04-06) are the ones that switch the writers over to consuming it"

key-files:
  created:
    - crates/deform6/src/write/mod.rs
    - crates/deform6/src/write/model.rs
    - crates/deform6/src/write/values.rs
    - crates/deform6/src/write/vbp.rs
    - crates/deform6/src/write/frm.rs
    - crates/deform6/src/write/code.rs
    - crates/deform6/src/write/comment.rs
    - crates/deform6/src/report.rs
    - crates/deform6/tests/extract_tracer.rs
  modified:
    - Cargo.toml
    - crates/deform6/Cargo.toml
    - crates/deform6/src/lib.rs
    - crates/deform6-cli/src/main.rs
    - crates/deform6-cli/tests/cli.rs

key-decisions:
  - "SafeName::new's four fixed rules run in this order: empty-name generation, illegal-character replacement, leading-letter fix, byte clamp. Clamping before the leading-letter fix would give a different 40-byte result than clamping after it, since the fix can both remove a byte (a leading underscore) and add one (a leading A)."
  - "Legal identifier characters are Unicode-alphanumeric (char::is_alphanumeric), not ASCII-only: a Latin-1 letter like é survives sanitization and is counted as one encoded byte under this crate's own Latin-1-as-code-point convention, proven by a test whose fixture's raw UTF-8 length (41) differs from its own char count and encoded-byte count (both 40)."
  - "Control names are never routed through the collision-resolving SafeNameIssuer, only through the pure SafeName::new: a repeated control name inside one form is a legitimate VB6 control array (told apart by Index), not a file-uniqueness collision. Only the root control (the form itself) and every project-level name (project/form/module/class) share the one issuer."
  - "BlobRef.frx_offset copies the read-time PropertyValue::Blob.frx_offset field rather than recomputing it via a second BlobCursor pass in model.rs: ProjectModel doesn't reorder anything relative to the read pass, so a fresh cursor run would give an identical value; the actual future writer (plan 04-04) still owns its own independent BlobCursor pass over its own (possibly reordered) output order, per Task 1's own frm.rs."
  - "TDD RED/GREEN was demonstrated (run, observed, reverted) but never committed as a separate failing-tests commit: AGENTS.md's own gate runs cargo test --workspace before every commit with no exception, and its Commits section says code and its tests share a commit. A commit with failing tests would violate both. Documented the observed RED evidence in each feat(...) commit message instead."
  - "vbp.rs and frm.rs still build each component's SafeName independently (not through the one issuer write::project threads), so a name that collides across two different writers is not yet resolved consistently between the .vbp and the .frm/.cls file names. No corpus form in this plan's own tracer collides, so this thin path is correct today; plan 04-03's own ProjectModel wiring is the fix, once vbp.rs/frm.rs/code.rs are switched over to consume it instead of building names independently."

patterns-established:
  - "Windows-1252 encoder is the hand-written functional inverse of this crate's own char::from(byte) read convention (crates/deform6/src/write/model.rs::encode_windows_1252) — never a general-purpose codec crate."
  - "LineWriter accumulates every line with a CRLF terminator, including the last, and writes no BOM — the one place every text writer in this phase gets that fact from."

requirements-completed: [WRT-01, WRT-06]

coverage:
  - id: D1
    description: "write::project turns a recovered Report plus the executable bytes into an in-memory project (.vbp, .frm, .frx, .cls, JSON report) with no file I/O"
    requirement: "WRT-01"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/extract_tracer.rs#the_written_file_list_holds_exactly_the_five_expected_files_and_nothing_else"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/extract_tracer.rs#the_written_frx_equals_the_committed_source_byte_for_byte"
        status: pass
      - kind: integration
        ref: "crates/deform6-cli/tests/cli.rs#extract_creates_the_directory_and_writes_the_expected_files_leaving_the_corpus_untouched"
        status: pass
    human_judgment: false
  - id: D2
    description: "The .frm's Icon property names the .frx file and the exact BlobCursor-given offset, and that offset's own record ends inside the written .frx"
    requirement: "WRT-06"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/extract_tracer.rs#the_icon_property_names_the_frx_file_and_an_uppercase_hex_offset_of_at_least_four_digits"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/extract_tracer.rs#the_icon_offset_names_a_record_that_ends_inside_the_written_frx"
        status: pass
    human_judgment: false
  - id: D3
    description: "SafeName is the only way a recovered name becomes a file path; hostile names (path separators, .., NUL, leading digit, 41-byte length, a Latin-1 letter, an empty name, two colliding names) are all made safe, proven by tests built inside the test file"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/model.rs#tests (12 SafeName/LineWriter tests)"
        status: pass
    human_judgment: false
  - id: D4
    description: "ProjectModel, FormModel, ControlModel, CodeModel, ProcedureModel and BlobRef carry every fact the five writers and the report builder need, built by from_report and exercised on Fast_Flames.exe"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/model.rs#tests (14 ProjectModel tests)"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/extract_tracer.rs#from_report_on_fast_flames_gives_one_form_one_class_and_a_matching_startup"
        status: pass
    human_judgment: false
  - id: D5
    description: "The confidence report's shape (ProjectReport, ReportItem, Confidence, Evidence) is locked for wave 2 to build against"
    verification:
      - kind: unit
        ref: "crates/deform6/src/report.rs#tests"
        status: pass
    human_judgment: true
    rationale: "The report's own JSON shape is exercised by two unit tests and is present as a file in every extract run, but whether the LOCKED shape is actually sufficient for four wave-2 plans to build against without widening it is a judgment 04-06's own execution will prove or disprove, not something this plan's own tests can assert."

duration: 1 session (multi-turn, includes a tracer feedback gate pause and human re-verification)
completed: 2026-09-13
status: complete
---

# Phase 4 Plan 1: One program, end to end, plus the complete name and model layer, Summary

**`deform6 extract` turns `Fast_Flames.exe` into a project directory whose `.frx` matches the committed source byte for byte, backed by a `SafeName` sanitizer that is the only way a recovered name reaches a path and a complete `ProjectModel` that later plans build against without widening it.**

## Performance

- **Duration:** one multi-turn session, paused once at the tracer feedback gate for human re-verification (approved)
- **Tasks:** 3 of 3 (all of plan 04-01)
- **Files created:** 9
- **Files modified:** 5
- **Commits:** 5 (`ec670b0` tracer, `31b1eb7` fix, `fd09905` SafeName, `b3b54b8` fix, `528130b` ProjectModel)

## Accomplishments

- `deform6::write::project` — the whole write path, opening no file and writing no file, exercised end to end against `corpus/vb6-code/Fire-effect/Fast_Flames.exe`: the written `frmFire.frx` equals the committed source byte for byte, all 1418 bytes.
- `deform6 extract <exe> -o <dir>` — a new CLI subcommand that builds the whole project in memory before writing any byte, so a refusal never leaves a partial directory.
- `SafeName` and `SafeNameIssuer` — the one way any recovered name becomes a file path, with a five-rule fixed order (empty-name generation, illegal-character replacement, leading-letter fix, byte clamp, collision suffix) and a `NameFault` naming every rule that fired.
- `ProjectModel` and its whole tree (`FormModel`, `ControlModel`, `CodeModel`, `ProcedureModel`, `BlobRef`) — every fact the five writers and the report builder need, built once by `from_report` and exercised by 26 unit tests plus one tracer assertion.
- `ProjectReport`, `ReportItem`, `Confidence`, `Evidence` — the confidence report's shape, locked for four wave-2 plans to build report items against.

## Task Commits

Each task was committed atomically (Tasks 2 and 3 carried `tdd="true"`; see "TDD Gate Compliance" below):

1. **Task 1: One program, end to end** — `ec670b0` (feat) + `31b1eb7` (fix, restoring a pre-existing staged deletion an earlier `git add` accidentally swept in)
2. **Task 2: Promote SafeName to the only path a name reaches a file** — `fd09905` (feat, code + tests together) + `b3b54b8` (fix, same restoration issue recurring once)
3. **Task 3: The complete ProjectModel** — `528130b` (feat, code + tests together)

## Files Created/Modified

- `crates/deform6/src/write/mod.rs` — the module set declaration and `write::project`, the whole public entry point
- `crates/deform6/src/write/model.rs` — `SafeName`, `SafeNameIssuer`, `NameKind`, `NameFault`, `encode_windows_1252`, `LineWriter`, and the complete `ProjectModel` tree with `from_report`
- `crates/deform6/src/write/values.rs`, `crates/deform6/src/write/comment.rs` — module stubs naming their owning plan (04-02, 04-07)
- `crates/deform6/src/write/vbp.rs` — the thin `.vbp` writer (`Type=Exe`, `Form=`, `Module=`/`Class=`, `Startup=`)
- `crates/deform6/src/write/frm.rs` — the thin `.frm`/`.frx` writer, owning the one `BlobCursor::take` call this whole phase must never duplicate
- `crates/deform6/src/write/code.rs` — the thin `.bas`/`.cls` writer (the fixed preambles)
- `crates/deform6/src/report.rs` — `ProjectReport`, `ReportItem`, `Confidence`, `Evidence`, `META_PATH`
- `crates/deform6/tests/extract_tracer.rs` — the end to end proof against `Fast_Flames.exe`
- `Cargo.toml`, `crates/deform6/Cargo.toml` — `serde_json` as a new workspace dependency
- `crates/deform6/src/lib.rs` — `pub mod write;` and `pub mod report;`
- `crates/deform6-cli/src/main.rs` — `Command::Extract`, `run_extract`
- `crates/deform6-cli/tests/cli.rs` — the `extract` smoke test

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential:

1. **Legal identifier characters are Unicode-alphanumeric, not ASCII-only.** This lets a Latin-1 letter like `é` survive sanitization, which is what makes the byte-vs-scalar-count acceptance test (a name whose raw UTF-8 length and encoded byte count differ) meaningful at all — under an ASCII-only rule, any non-ASCII character would always be replaced by a one-byte underscore before the byte count ever mattered.
2. **Control names never go through the collision-resolving issuer.** A VB6 control array legitimately repeats a name across several controls, distinguished only by `Index`; running that through the same collision suffix logic used for file-mapping names (forms, modules, classes) would corrupt every control array in the corpus. Only the root control (the form itself) shares the project-wide issuer with every other file-mapping name.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `SafeName::new`'s legality check used `is_ascii_alphanumeric`, failing the crate's own byte-vs-scalar-count acceptance test**
- **Found during:** Task 2, writing the hostile-name test for "a character above 0x7F whose scalar count and byte count differ"
- **Issue:** An ASCII-only legality check replaces every non-ASCII character with an underscore before any byte-counting logic runs, so no test could ever observe a byte-vs-scalar-count discrepancy — the acceptance criterion itself was unsatisfiable under that reading.
- **Fix:** Changed `is_legal_identifier_char` to `char::is_alphanumeric` (Unicode-aware), matching this crate's own `char::from(byte)` read-side convention of treating a Latin-1 byte as an ordinary letter.
- **Files modified:** `crates/deform6/src/write/model.rs`
- **Verification:** `safe_name_counts_a_character_above_0x7f_as_one_encoded_byte_not_as_two_utf8_bytes` passes
- **Committed in:** `fd09905`

**2. [Rule 1 - Bug] Threading the shared `SafeNameIssuer` through every control name corrupted VB6 control arrays**
- **Found during:** Task 3, writing the repeated-control-name test
- **Issue:** Three controls legitimately named `Text1` (a control array) were each run through the same collision-resolving issuer used for file names, so the second and third were silently renamed to distinct strings instead of keeping the shared name `Index` is supposed to distinguish.
- **Fix:** Only the root control (index 0, the form itself) issues through the shared, project-wide `SafeNameIssuer`; every other control uses the pure, non-colliding `SafeName::new` directly.
- **Files modified:** `crates/deform6/src/write/model.rs`
- **Verification:** `a_repeated_control_name_with_no_reader_supplied_index_gets_generated_indexes_in_issue_order` passes, asserting all three controls keep the name `Text1` with indexes `0, 1, 2`
- **Committed in:** `528130b`

**3. [Rule 1 - Bug] A pre-existing staged deletion was twice swept into a task commit**
- **Found during:** Tasks 1 and 2, immediately after committing
- **Issue:** `.planning/phases/01-it-reads-the-file/VERIFICATION.md` was already staged for deletion (not this plan's own change, and explicitly called out as pre-existing) before this plan started. `git add <task files>` does not touch other already-staged entries, and `git commit` with no pathspec commits the whole index, so both `ec670b0` and `fd09905` accidentally included that deletion.
- **Fix:** Restored the file byte-for-byte from the parent commit in a follow-up commit each time, then re-staged the deletion (`git rm --cached` + `rm`) without committing it, returning the tree to exactly the pre-existing condition. From `528130b` onward, every commit uses `git commit -- <pathspec>` to restrict it to only the paths this plan changed, which cannot pick up an unrelated already-staged entry.
- **Files modified:** `.planning/phases/01-it-reads-the-file/VERIFICATION.md` (restored, then re-staged for deletion, net effect zero)
- **Verification:** `git status --short` after `528130b` shows the same three pre-existing/orchestrator-owned entries (`D VERIFICATION.md`, `.gsd/`, `Notes/`, `.planning/milestone.lock`) as at the start of this plan, and nothing else
- **Committed in:** `31b1eb7`, `b3b54b8`

---

**Total deviations:** 3 auto-fixed (2 bugs in new code, 1 commit-hygiene bug). **Impact:** All three were necessary for correctness; none reflect scope creep. The commit-hygiene bug is fully reverted with no net change to `VERIFICATION.md`'s tracked state.

## TDD Gate Compliance

Tasks 2 and 3 both carry `tdd="true"`. RED was run and observed for both (see each task's own commit message for the exact failing-test count and reason), but **not committed as a separate commit**, per this project's own `AGENTS.md`:

- "The gate... before every commit. Not a selection." (`cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` must pass every time)
- "Put the code and its tests in the same commit."

A committed RED state, by definition, fails `cargo test --workspace`, which both of these rules forbid. Instead, RED was demonstrated by temporarily reverting the implementation to a deliberately incomplete stub (keeping the new type signatures so the new tests still compiled), running the real test suite, observing the specific tests fail for the right reason, then restoring the correct implementation — the same pattern Task 1 already used for the `.frx` offset cursor's own red phase. Both RED runs are documented verbatim in the `feat(04-01)` commit messages (`fd09905`, and the observed-then-reverted stub is not itself a commit).

| Task | RED observed | GREEN commit | REFACTOR | Status |
|------|--------------|--------------|----------|--------|
| 2 (SafeName) | 5 of 12 `write::model` tests failed against a reverted stub (missing fault entries, wrong per-kind generated name, no collision resolution) | `fd09905` | none needed | Pass |
| 3 (ProjectModel) | Not separately re-verified via a reverted stub (the type didn't exist before this task); iterative red/green cycles ran during authoring (documented inline: e.g. the control-array collision bug caught by `a_repeated_control_name_...` before the fix) | `528130b` | none needed | Pass |

## Issues Encountered

None beyond the deviations above, all resolved.

## User Setup Required

None — no external service configuration required.

## Known Stubs

- `crates/deform6/src/write/values.rs`, `crates/deform6/src/write/comment.rs` — module doc comments only, explicitly deferred to plans 04-02 and 04-07 by the plan's own artifact table. Not stubs in the "silently incomplete" sense; they are declared-empty by design so `write/mod.rs`'s module set is complete from this plan's first commit.
- `ProjectModel` is built and fully tested but **not yet wired into** `write::project`'s actual writers (`vbp.rs`, `frm.rs`, `code.rs` still build each `SafeName` independently, not through `ProjectModel`). This is explicit staging per the plan's own architecture (04-RESEARCH.md's own diagram shows `write::model::from_report` feeding the writers only from wave 2 onward) — recorded here so a reader of this SUMMARY alone knows the model exists without yet being load-bearing.
- The `.vbp`/`.frm`/`.cls` writers built in this plan are explicitly "thin": most non-Form/Label/CommandButton/ListBox/MDIForm control properties print as empty `Begin...End` blocks (the builtin opcode table subset decodes nothing else), and `BackColor` prints as a plain signed decimal rather than `&H80000005&` (the colour/enum formatting table is plan 04-02's own job, `write/values.rs`). Neither is a defect against this plan's own acceptance criteria, which test only the `.frx` byte match, the `Icon` line, the `Begin`/`Attribute` structure, and the `.vbp`/`.cls` facts explicitly listed.

## Next Phase Readiness

- Plan 04-02 (`values.rs`) can build the colour/enum/plain-number formatting table against `ControlModel::properties` (`Vec<PropertyValue>`) or the raw `PropertyValue` type directly.
- Plan 04-03 (`vbp.rs` full grammar) and plan 04-04 (`frm.rs` full grammar) both have a complete, tested `ProjectModel` to switch their writers over to, closing the "independent naming per writer" gap noted above.
- Plan 04-06 (report builder) has the locked `ProjectReport`/`ReportItem`/`Confidence`/`Evidence` shapes and a working `from_report` that already produces real `ReportItem`s for four of the five behaviors 04-01's own plan text names (unknown object kind, orphaned form/object joins, inferred startup, generated control array indexes) — 04-06's own job is wiring these into the actual JSON report `write::project` currently ships with an empty `items: []`.
- No blocker for wave 2. The one open architectural note (independent per-writer naming vs. the shared `ProjectModel`) is explicitly plan 04-03/04-04's own scope, not a gap this plan silently introduced.

---
*Phase: 04-it-writes-a-project*
*Completed: 2026-09-13*
