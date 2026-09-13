---
phase: 04-it-writes-a-project
plan: 05
subsystem: write
tags: [rust, vb6, code-generation, procedure-signatures, code-region]

requires:
  - phase: 04-01
    provides: "write::model (SafeName, LineWriter, encode_windows_1252), report.rs (ReportItem, Confidence)"
  - phase: 04-02
    provides: "write::values::escape_inline_string, the inline-string quoting rule this plan reuses for a Text default value"
provides:
  - "deform6::write::code::{write_bas, write_cls} — the complete .bas and .cls writers: preamble, code region, byte-substitution report item"
  - "deform6::write::code::write_code_region — the one code region emitter write::frm (plan 04-04) also calls"
  - "deform6::write::code::form_attribute_block — the shared five-line Attribute block, callable by class and form writers alike"
  - "deform6::write::code::{format_signature, format_procedure_entry, format_procedures} — the empty-body procedure signature for every recovered procedure shape"
affects: [04-04, 04-06, 04-08, 04-09]

actuals:
  tokens: 10377
  tasks: 3
  commits: 3
plan_head_before: f1382f9053e6eff5be08e4ae1dd08298b9ee0bb6

tech-stack:
  added: []
  patterns:
    - "The five class property lines are held as (name, spaces-before-equals, value, comment) tuples, with the space count stored as measured data rather than derived from a single pad-width formula, because the real corpus bytes do not fit one formula for all five names."
    - "format_signature collapses two different facts (a public procedure with no resolved prototype, and a private slot) into the same no-argument-Sub geometry, and lets the caller supply the name and the scope word, keeping the report-item distinction at the caller."
    - "Two main.rs display-only formatters (a boolean default, a blank argument name) were not safe to reuse verbatim for compilable output and were corrected in this plan: True/False instead of Rust's true/false, and a generated Arg<N> placeholder instead of a blank identifier."

key-files:
  modified:
    - crates/deform6/src/write/code.rs

key-decisions:
  - "The 'pad to twenty' formula FILE-FORMATS.md section 5.2 states does not reproduce the real corpus bytes for two of the five class property names (MultiUse, Persistable get one space regardless; only the three longest names actually land on width twenty). CLS_PREAMBLE_PROPERTIES stores each row's own measured space count as data instead of deriving it from CLS_NAME_PAD, which stays as a documented fact about the three longest rows, not a computation."
  - "cls_preamble_lines, bas_header_line, format_procedure_entry and format_procedures are pub (not private or pub(crate)): each task's own commit needed to compile and pass the full gate standing alone, and a private helper reachable only from a #[cfg(test)] module is dead code in the non-test build. Making the phase's own building blocks pub avoided reordering or hiding functionality between commits."
  - "A public procedure with no recovered prototype and a private procedure slot both write a no-argument Sub: neither carries an argument list, so format_signature treats prototype: None as one case and lets the caller (format_procedure_entry) supply the differing name, scope word and report item (confidence Inferred vs Unrecoverable)."
  - "An unresolved COM type (VbType::Internal/ComIFace/ComObj) writes the keyword Object; an unrecognised type code (VbType::Unknown) writes Variant; VbType::HResult writes Long. None of these substitutions is a corpus-measured fact — no corpus procedure reaches them — but WRT-07's own objective is a project that still builds, and a signature naming an invented or absent keyword would not compile."
  - "A private procedure's generated name is UnnamedProcedure<slot-index>, not a single fixed word: two private slots in the same object must not collide on the same Sub name, which a single fixed generated name (mirroring model.rs's own per-kind 'Unnamed<Kind>' convention) would not guarantee."

patterns-established:
  - "write_code_region is the single code region emitter for all three file kinds (write_bas, write_cls, and write::frm's own form writer in plan 04-04): none of the three holds a code region of its own, and it reaches no process global mutable state."

requirements-completed: [WRT-05, WRT-07]

coverage:
  - id: D1
    description: "The .bas header is the one Attribute VB_Name line, and the .cls preamble is the thirteen lines FILE-FORMATS.md section 5.2 gives, byte identical to the corpus apart from the name"
    requirement: "WRT-05"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#tests::the_thirteen_class_preamble_lines_match_the_corpus_byte_for_byte"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#tests::a_written_module_files_first_line_is_the_one_line_header"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#tests::the_class_and_form_attribute_blocks_differ_in_exactly_two_values_and_agree_in_three"
        status: pass
    human_judgment: false
  - id: D2
    description: "A public procedure with a prototype writes its full signature and empty body; a public procedure with no prototype and a private slot each write a no-argument Sub under the right name and scope word, with a distinct report item; an object with no procedure name array writes no signature and one report item carrying the real declared count"
    requirement: "WRT-07"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#signatures::a_function_with_two_arguments_one_optional_and_one_an_array_gives_an_exact_signature"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#signatures::a_public_procedure_with_no_prototype_gives_a_no_argument_signature_and_one_inferred_item"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#signatures::a_private_procedure_gives_a_private_no_argument_signature_under_a_generated_name"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#signatures::an_object_with_no_procedure_name_array_gives_no_lines_and_one_item_naming_the_count"
        status: pass
    human_judgment: false
  - id: D3
    description: "One code region emitter serves write_bas and write_cls (and is ready for write::frm to call, plan 04-04); it takes comment lines as a parameter and supplies none of its own; two calls on the same input give byte identical output with no shared mutable state"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#tests::write_bas_called_twice_on_one_input_gives_byte_identical_output"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#tests::write_cls_called_twice_on_one_input_gives_byte_identical_output"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/code.rs#tests::write_code_region_emits_the_comment_lines_it_is_given_and_supplies_none_of_its_own"
        status: pass
      - kind: other
        ref: "grep -vE '^\\s*//|^\\s*///' crates/deform6/src/write/code.rs | grep -cE 'static mut|OnceLock|LazyLock|RefCell' == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "Byte-level manual verification: a written .cls preamble compared line by line against corpus/vb6-code/Fire-effect/FastDrawing.cls, and the extract_tracer end-to-end test (frmFire.frx still byte identical) still passes"
    verification:
      - kind: integration
        ref: "crates/deform6/tests/extract_tracer.rs (11 tests, run this session)"
        status: pass
    human_judgment: true
    rationale: "The plan's own <verification> step asks for a by-eye read of the two files side by side, which this session performed (via a Python byte dump of FastDrawing.cls) in addition to the automated literal-string test; a human should confirm the recorded space counts in this SUMMARY match what a second reading of the corpus file shows."

duration: 1 session
completed: 2026-09-13
status: complete
---

# Phase 4 Plan 5: The module writer, the class writer, and the empty procedure signature, Summary

**`write::code` now writes complete `.bas` and `.cls` files (preamble plus one shared code region) and gives every recovered procedure shape — full prototype, no-prototype public, private, and no-name-array — its own compilable empty signature and its own report item.**

## Performance

- **Duration:** 1 session
- **Completed:** 2026-09-13
- **Tasks:** 3 of 3
- **Files modified:** 1 (`crates/deform6/src/write/code.rs`)

## Accomplishments

- `write_cls`/`write_bas` write the complete file: the fixed preamble, then `write_code_region`'s output, then a report item for any character that could not be represented in Windows-1252 — on top of `write_cls_thin`/`write_bas_thin`, which plan 04-01's tracer still calls unchanged.
- The five `.cls` property lines' own space counts were measured by hand this session, byte for byte, against `corpus/vb6-code/Fire-effect/FastDrawing.cls`: `MultiUse` and `Persistable` each get exactly one space before `=`; `DataBindingBehavior` (19 chars + 1), `DataSourceBehavior` and `MTSTransactionMode` (18 chars + 2 each) all land on width twenty. `FILE-FORMATS.md`'s own "padded to a width of 20" description does not reproduce the first two rows; `CLS_PREAMBLE_PROPERTIES` stores the real per-row count as data instead.
- `format_attribute_block` is the one function giving the five `Attribute` lines for a class or a form (`.planning/research/FILE-FORMATS.md` section 9 gap 3: always all five, since the loader's tolerance for a missing one is unverified).
- `format_signature` reuses the exact join order `crates/deform6-cli/src/main.rs` already established in Phase 2 (`Optional`/`ByRef` prefix, name, `()` for array, `As <type>`, `= <default>`), with two corrections needed for genuinely compilable output rather than display text: `True`/`False` instead of Rust's `true`/`false`, and a generated `Arg<N>` placeholder instead of a blank recovered argument name.
- `format_procedure_entry`/`format_procedures` tell the four procedure-recovery shapes apart (full prototype, public-no-prototype, private, no-name-array-at-all) and give each of the last three its own report item, never silently collapsing "no signature" into "no procedures."
- `write_code_region` is the one function `write_bas`, `write_cls`, and `write::frm`'s own form writer (plan 04-04) all call: it reaches no process global mutable state, confirmed both by a passing determinism test (two calls, byte identical output) and by a source grep for `static mut`/`OnceLock`/`LazyLock`/`RefCell` (0 matches).
- 29 tests in `write::code`, all new this plan (22 tests spread across tasks 1 and 3's `mod tests`, 7 more counted within task 2's own `mod signatures`, for 13 total there); `cargo test -p deform6 --test extract_tracer` still passes (11 tests), and the written `frmFire.frx` stays byte identical to the committed corpus source.

## Task Commits

Each task was committed atomically:

1. **Task 1: The two preambles, measured apart** — `575c6e4` (feat)
2. **Task 2: The empty procedure with the right signature** — `ff89577` (feat)
3. **Task 3: The shared code region, and determinism with no shared state** — `f006baf` (feat)

**Plan metadata:** commit pending (this SUMMARY, STATE.md, ROADMAP.md, REQUIREMENTS.md)

## Files Created/Modified

- `crates/deform6/src/write/code.rs` — the module writer, the class writer, the shared attribute block, the signature formatter, and the shared code region emitter (994 lines total; the module doubled in size over the 42-line thin stub plan 04-01 left)

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The three most consequential:

1. **The class property lines' own space counts are stored as measured data, not derived from a single "pad to twenty" formula.** `FILE-FORMATS.md` section 5.2's own prose gives a formula that predicts the wrong byte for `MultiUse` and `Persistable`. Measuring `corpus/vb6-code/Fire-effect/FastDrawing.cls` directly this session (a Python byte dump, not a visual estimate) settled it: only the three longest names actually reach width twenty.
2. **Several helper functions are `pub` rather than private**, so each of the three tasks' own commits could compile and pass the full gate standing alone — a function reachable only from a `#[cfg(test)]` module is dead code in the primary build, and this project's gate runs `cargo clippy --all-targets -- -D warnings` (which denies `dead_code`) before every commit. This is a visibility choice, not a behavior change.
3. **A public procedure with no prototype and a private slot share one code path** (`format_signature` with `prototype: None`), with the caller supplying the differing name and scope word. This keeps the three-shapes distinction (full recovery / name-without-a-shape / null-name) at the report-item layer, where `OBJ-06`'s "never invent a name" rule and the standard-module cap actually live.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `DefaultValue::Boolean`'s display-only formatting is not valid Visual Basic**
- **Found during:** Task 2, writing `format_default`
- **Issue:** `crates/deform6-cli/src/main.rs`'s own `format_default` (reused per the plan's own instruction to reuse the existing join order) writes Rust's `bool` display, `true`/`false`, for a boolean default. Visual Basic has no lower-case boolean literal; a signature carrying `= true` would not compile.
- **Fix:** `format_default` writes the capitalised keywords `True`/`False` instead.
- **Files modified:** `crates/deform6/src/write/code.rs`
- **Verification:** `a_default_boolean_value_writes_the_capitalised_vb6_keyword_not_rusts_lower_case` passes
- **Committed in:** `ff89577` (Task 2 commit)

**2. [Rule 1 - Bug] A blank recovered argument name is not a legal Visual Basic identifier**
- **Found during:** Task 2, writing `format_argument`
- **Issue:** `Argument::name` is documented as empty when the reader's own name resolution failed. Writing an empty identifier into a signature line (`As Long` with no name before it) does not compile.
- **Fix:** `format_argument` substitutes a generated `Arg<N>` placeholder (`N` the argument's own 1-based position) when the recovered name is empty.
- **Files modified:** `crates/deform6/src/write/code.rs`
- **Verification:** `an_empty_argument_name_gets_a_generated_placeholder_not_a_blank_identifier` passes
- **Committed in:** `ff89577` (Task 2 commit)

**3. [Rule 2 - Missing Critical] Three `VbType` variants have no direct Visual Basic keyword**
- **Found during:** Task 2, writing the exhaustive `format_vb_type` match
- **Issue:** `VbType::Internal`/`ComIFace`/`ComObj` carry only a raw, unresolved address (Phase 2 punted on resolving them to a real class name), and `VbType::Unknown`/`HResult` have no direct Visual Basic keyword at all. The project's own no-wildcard-arm convention means every variant must produce something, and writing the raw address or an invented name would either not compile or misrepresent a fact this crate never recovered.
- **Fix:** `Internal`/`ComIFace`/`ComObj`/`Object` all write `Object` (the one legal keyword covering an unresolved reference type); `Unknown` writes `Variant` (the one type every value can hold); `HResult` writes `Long` (its own COM representation, since Visual Basic has no `HResult` keyword).
- **Files modified:** `crates/deform6/src/write/code.rs`
- **Verification:** `an_unknown_vb_type_writes_variant_and_a_com_type_writes_object` passes
- **Committed in:** `ff89577` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 missing critical functionality). **Impact:** All three were necessary so the written signature is legal, compilable Visual Basic; none of the corpus's own 44 programs exercises the three substituted `VbType` variants, so this is defensive completeness against WRT-07's own objective, not a corpus-measured correction. No scope creep.

## Issues Encountered

None beyond the deviations above, all resolved.

## User Setup Required

None — no external service configuration required.

## Known Stubs

- No corpus program exercises `VbType::Internal`/`ComIFace`/`ComObj`/`Unknown`/`HResult` inside a signature; the three-way `Object`/`Variant`/`Long` substitution (see Deviations #3) is untested against real recovered bytes, only against synthetic fixtures built inside this plan's own tests. This is stated, not hidden: `crate::vb::functyp::VbType`'s own doc comments already name these as unresolved by construction.
- `write::project` (`crates/deform6/src/write/mod.rs`) still calls `write_cls_thin`/`write_bas_thin`, not this plan's `write_cls`/`write_bas`: this plan's own `files_modified` is `code.rs` alone, matching the same staged pattern plans 04-01 through 04-03 already established (`write_vbp_thin` versus the later `write_vbp`). Plan 04-08, which already owns `Command::Extract`'s full wiring, is the natural place to switch `write::project` over.

## Next Phase Readiness

- Plan 04-04 (`frm.rs`, the form writer) has a tested, complete `write_code_region` and `form_attribute_block` (for `AttributeFileKind::Form`) to call, closing the "one code region emitter for all three file kinds" architecture this plan's own objective named.
- Plan 04-06 (report builder) has `format_procedure_entry`/`format_procedures`'s own `ReportItem`s (path left empty, per the same convention `write::values::format_value` established) to fold into the flat report array, once it knows the real object or form path.
- Plan 04-08 has a complete, tested `write_bas`/`write_cls` pair to switch `write::project` over to, closing the "thin vs. full" gap this plan's own scope did not touch.
- No blocker for the rest of the phase. `WRT-05` and `WRT-07` are not shared with any sibling plan's own `requirements:` field in this phase (checked directly against every other `04-*-PLAN.md`), so both are marked complete now.

---
*Phase: 04-it-writes-a-project*
*Completed: 2026-09-13*

## Self-Check: PASSED

- `crates/deform6/src/write/code.rs` exists on disk and defines `pub fn write_cls`, `pub fn write_bas`, `pub fn write_code_region`, `pub fn form_attribute_block`, `pub fn format_signature`.
- Commits `575c6e4`, `ff89577`, `f006baf` all exist in `git log --oneline --all`.
- `cargo test -p deform6 --lib write::code` reports 29 passing tests; `cargo test -p deform6 --lib write::code::signatures` reports 13.
- `cargo test -p deform6 --test extract_tracer` passes (11 tests); the written `frmFire.frx` stays byte identical to the committed corpus source.
- The full gate (`cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`) passes: 522 tests in the `deform6` lib target alone, 0 failures across the whole workspace.
- Source greps: `CLS_NAME_PAD: usize = 20`, `CLS_PREAMBLE_PROPERTIES`, `pub fn form_attribute_block`, `pub fn format_signature`, `pub fn write_code_region` are all present; no `static mut`/`OnceLock`/`LazyLock`/`RefCell` outside a comment.
- The pre-existing staged deletion (`.planning/phases/01-it-reads-the-file/VERIFICATION.md`), the `.planning/config.json` modification, and the untracked `.gsd/`, `Notes/`, `.planning/milestone.lock` paths are untouched by any of this plan's three commits (`git status --short` before and after this plan's commits shows the identical set).
