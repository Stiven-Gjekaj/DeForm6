---
phase: 04-it-writes-a-project
plan: 02
subsystem: write
tags: [rust, vb6, property-value-serialization, colour-formatting, code-generation]

requires:
  - phase: 04-01
    provides: "write::model (SafeName, ProjectModel), report.rs (ReportItem, Confidence, Evidence), the whole write module set"
provides:
  - "deform6::write::values::format_value — turns one decoded (or undecoded) crate::vb::propstream::PropertyValue into a FormattedValue (a formatted line, several named lines, a resource decision, or an omission plus a report item)"
  - "the name-keyed colour and enumeration formatting tables (COLOUR_PROPERTIES, ENUM_MEMBERS, shape_for, enum_member_name), separate from the reader's own payload type"
  - "the inline-string-versus-resource-reference decision (InlineDecision, inline_decision) and the resource reference builder (format_resource_reference)"
affects: [04-04, 04-06, 04-09]

actuals:
  tokens: 8431
  tasks: 3
  commits: 3
plan_head_before: fadfef357c9b02fc63b8f70450b0c2a5b6afdc90

tech-stack:
  added: []
  patterns:
    - "A small, additive, name-keyed shape table (COLOUR_PROPERTIES, ENUM_MEMBERS) sits beside the reader's own payload type and never widens it, because the payload type was built to decode bytes, not to remember display intent."
    - "Exhaustive match, no wildcard arm: format_value's own match over PropertyValue's eleven variants has one arm per variant, verified by a source grep in the plan's own <verify> block, the same discipline crate::error::DefectKind::severity already uses."
    - "A compound payload (Position, Font) expands into several separate named lines (FormattedValue::Multi) rather than collapsing into one omitted or one wrong-shaped line, because FILE-FORMATS.md names no single 'Position' or 'Font' key anywhere in the grammar."
    - "A font size fraction is reconstructed by trimming the reader's own remainder digits, never by rebuilding a floating point value, so no rounding step can silently change a provable number."

key-files:
  created: []
  modified:
    - crates/deform6/src/write/values.rs

key-decisions:
  - "FormattedValue carries four states, not the two the plan's own opening line names (a line, or the omission decision): Multi for a Position or a Font payload's own several named lines, and Resource for a value that must leave the form file (a Blob, or a Text value over the inline threshold). Collapsing Position and Font into the two-state design would have forced this formatter to omit a control's own geometry and font, which are fully recoverable facts, not undecoded ones; that would be a Rule 2 correctness gap far worse than the two-state framing anticipated. The undecoded and unreadable-blob arms, which the plan's opening line describes exactly, keep the true two-state shape."
  - "The path field of a ReportItem this module builds is always empty. format_value knows only the property, never the form or the control it belongs to; the caller (plan 04-04's frm.rs, and plan 04-06's report builder) fills in the real path before the item enters the project report."
  - "The sixteen enumeration member names FILE-FORMATS.md section 3.5 lists are matched to the one property each belongs to by grepping every corpus .frm file this session for the exact line shape 'Name = Value  'Member', since section 3.5's own prose names the sixteen strings but not, in one place, which property each is measured against. All sixteen resolved to exactly one property and one value each, with no ambiguity."
  - "format_value's own signature gained an is_external: bool parameter in task 2's commit, not task 1's: task 1 has no colour distinction yet, and an unused parameter would have failed the workspace's own -D warnings gate."

patterns-established:
  - "Never reconstruct a fixed-point value (a font size) through floating point arithmetic when the exact digits are already available; trim the decimal digit string instead, per AGENTS.md's own 'give the number that can be proved' rule."

requirements-completed: []

coverage:
  - id: D1
    description: "An undecoded property and an unreadable blob each produce no line and a distinct report item naming the opcode or the property, and the exact byte offset the reader recorded"
    requirement: "WRT-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/values.rs#tests::an_undecoded_property_gives_omit_and_a_report_item_naming_the_opcode_and_offset"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/values.rs#tests::an_unreadable_blob_gives_omit_and_a_report_item_distinct_from_undecoded"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every decoded value shape (integer/long, boolean, enumeration, colour on an intrinsic and an external control, float, string, font block) produces the exact bytes FILE-FORMATS.md section 3 measures, anchored against two committed corpus .frm lines"
    requirement: "WRT-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/values.rs#tests::the_fast_flames_back_color_line_matches_the_committed_frm_byte_for_byte"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/values.rs#tests::the_curves_border_style_line_matches_the_committed_frm_byte_for_byte"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/values.rs#tests::a_font_value_gives_seven_lines_in_the_fixed_order"
        status: pass
    human_judgment: false
  - id: D3
    description: "A string of 97 encoded bytes or fewer with no line break goes inline; everything else, and every resource blob, gives the resource decision, with the file name and the offset supplied by the caller and computed nowhere in this module"
    requirement: "WRT-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/write/values.rs#tests::a_97_byte_string_goes_inline_and_a_98_byte_string_does_not"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/values.rs#tests::a_string_holding_a_carriage_return_and_line_feed_never_goes_inline"
        status: pass
    human_judgment: false

duration: 1 session
completed: 2026-09-13
status: complete
---

# Phase 4 Plan 2: One property value at a time, from the omission path to the resource reference, Summary

**`write::values::format_value` turns every decoded VB6 property value into the exact bytes the IDE writes, branches colour and enumeration formatting by property name rather than payload type, and gives an undecoded property a report item instead of a placeholder line.**

## Performance

- **Duration:** 1 session
- **Started:** 2026-09-13
- **Completed:** 2026-09-13
- **Tasks:** 3 of 3
- **Files modified:** 1

## Accomplishments

- `format_value` is the whole public entry point: an exhaustive match over the reader's eleven-variant `PropertyValue` enum with no wildcard arm, so a variant added later fails to compile here until somebody decides how to write it.
- An undecoded property and an unreadable blob each give the omission decision and a distinct `ReportItem`, carrying the reader's own byte offset, never a computed one. Re-measured this session against the whole 44-program corpus: 124 property records now decode to a name and a value, 683 report present and not decoded, over 807 total property records across six distinct property names (`BackColor`, `BorderStyle`, `Caption`, `Icon`, `Position`, `WindowState`). The undecoded count (683) is unchanged from the number recorded in `REQUIREMENTS.md`; the named and total counts shifted by two, which this session measured directly rather than copying.
- The colour and enumeration formatting tables (`COLOUR_PROPERTIES`, three names; `ENUM_MEMBERS`, sixteen `(property, value, member name)` triples) are name-keyed and sit beside the reader's own payload type without widening it. A colour writes eight upper case hex digits on an intrinsic control and a plain signed decimal on an external one, from the one signed value the reader gave. A value one of the sixteen proven pairs names gets its member-name comment; every other value stays a bare number.
- A `Position` payload decomposes into `Left`, `Top`, `Width` and `Height`; a `Font` payload decomposes into its own seven fixed-order keys, with `Size` reconstructed from the reader's own points and remainder by trimming decimal digits, never by rebuilding a float.
- A recovered string of 97 encoded bytes or fewer with no line break goes inline, escaped by doubling every inner double quote; everything else, and every resource blob, gives the resource decision. The resource reference builder takes the file name and the offset from the caller and computes neither.

## Task Commits

Each task was committed atomically:

1. **Task 1: The undecoded path first, because it is the common path** - `f9fec4d` (feat)
2. **Task 2: The six measured value grammars** - `ce047f5` (feat)
3. **Task 3: The inline string rule and the resource reference forms** - `bb99bee` (feat)

**Plan metadata:** commit pending (this SUMMARY, STATE.md, ROADMAP.md, REQUIREMENTS.md)

## Files Created/Modified

- `crates/deform6/src/write/values.rs` - the property value formatter, the colour and enumeration shape tables, the inline string rule, and the resource reference forms (874 lines total; 869 lines added by this plan over the module-doc-comment-only stub plan 04-01 left)

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential:

1. **`FormattedValue` carries four states, not two.** The plan's own opening line describes a formatter that gives back "either one formatted value string or the decision to omit the line." That framing fits the undecoded and unreadable-blob arms exactly, which this task's own job is. But a `Position` or a `Font` payload is a fully recoverable fact that FILE-FORMATS.md's own grammar never expresses as one line: forcing either into the two-state design would have meant either a wrong single line or an incorrect `Omit`, silently dropping a control's own geometry or font on every form that carries one — nearly every form in the corpus. `FormattedValue::Multi` (several named lines) and `FormattedValue::Resource` (a value that must leave the form file) are the two additions, both auto-added under deviation Rule 2.
2. **The colour test's own stated character count was wrong, and the test follows the number that can be proved.** Task 2's acceptance criteria state a colour on an intrinsic control formats to "a string of exactly twelve characters." Counted directly against the literal example FILE-FORMATS.md section 3.6 and the corpus both give, `&H80000005&`, the true count is eleven: `&H` (2) plus eight hex digits (8) plus `&` (1). The implementation and its test both use eleven, per `AGENTS.md`'s own "give the number that can be proved" rule.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `FormattedValue` needed a fourth and fifth state beyond the plan's own two-state framing**
- **Found during:** Task 1, writing the exhaustive match's `Position` and `Font` arms
- **Issue:** `FILE-FORMATS.md` names no `Position` or `Font` line anywhere in its own grammar; both payloads decompose into several separately-named lines in the real `.frm` output (`Left`/`Top`/`Width`/`Height`; `Name`/`Size`/`Charset`/`Weight`/`Underline`/`Italic`/`Strikethrough`). A formatter limited to "one line or omit" would have to omit these, which is false: the data is fully recoverable, not undecoded.
- **Fix:** Added `FormattedValue::Multi(Vec<(&'static str, String)>)` for a payload that expands into several named lines, and `FormattedValue::Resource` for a value that must leave the form file (a `Blob`, or an over-threshold `Text`), reusing the same `Resource` state task 3 also needed for the inline string overflow case.
- **Files modified:** `crates/deform6/src/write/values.rs`
- **Verification:** `a_position_value_through_format_value_gives_a_multi_of_four`, `a_font_value_through_format_value_gives_a_multi_of_seven` pass
- **Committed in:** `f9fec4d` (Task 1 commit)

**2. [Rule 1 - Bug] Task 2's own acceptance text stated the wrong character count for a colour value**
- **Found during:** Task 2, writing the intrinsic-colour test
- **Issue:** The plan's acceptance criteria say a colour on an intrinsic control formats to "a string of exactly twelve characters." `&H` (2) + eight hex digits (8) + `&` (1) is eleven, not twelve, and the literal corpus example the plan itself cites, `&H80000005&`, is eleven characters long when counted.
- **Fix:** Wrote the test against eleven, with a comment naming the exact arithmetic and citing the corpus example that proves it.
- **Files modified:** `crates/deform6/src/write/values.rs`
- **Verification:** `a_colour_on_an_intrinsic_control_writes_eight_upper_case_hex_digits_bracketed` passes
- **Committed in:** `ce047f5` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 missing critical functionality, 1 bug in the plan's own stated number). **Impact:** Both auto-fixes were necessary for correctness. The `Multi`/`Resource` extension is a strict widening of the return shape (the undecoded and unreadable-blob arms this task's own job covers are unchanged); no scope creep.

## Issues Encountered

None beyond the deviations above, all resolved.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. `format_value` is not yet called by any writer: wiring it into `write::frm` and `write::code` is plan 04-04's and plan 04-05's own job, exactly as plan 04-01's own SUMMARY already staged (`write/frm.rs` and `write/code.rs` still build each line independently of `values.rs`). This is declared staging, not an incomplete or placeholder behavior inside this plan's own file.

## Next Phase Readiness

- Plan 04-04 (`frm.rs`, full grammar) can call `format_value` for every property a control's stream decoded, use `FormattedValue::Multi` to emit `Position`'s and `Font`'s own several lines (`Font` still needs the `BeginProperty`/`EndProperty` wrapping, which stays this later plan's own job), and use `FormattedValue::Resource` plus `format_resource_reference` for a blob or an over-threshold string, supplying the file name and the `BlobCursor`-given offset it already owns.
- Plan 04-05 (`code.rs`) has no direct dependency on this plan.
- Plan 04-06 (report builder) can take a `ReportItem` this module returns and set its own `path` before adding it to the flat report array.
- No blocker for wave 3. `WRT-03` stays open: it is shared with plans 04-04 and 04-09, and the shared-ID gate correctly holds it open until both finish.

---
*Phase: 04-it-writes-a-project*
*Completed: 2026-09-13*

## Self-Check: PASSED

- `crates/deform6/src/write/values.rs` exists on disk.
- Commits `f9fec4d`, `ce047f5`, `bb99bee` all exist in `git log --oneline --all`.
- `cargo test -p deform6 --lib write::values` reports 33 passing tests (33 `#[test]` functions counted in the file).
- `cargo test -p deform6 --test extract_tracer` passes; the written `frmFire.frx` stays byte-identical to the committed corpus source.
- The full gate (`cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`) passes.
- No wildcard match arm exists in the file (`grep -vE '^\s*//|^\s*///' ... | grep -cE '_\s*=>'` gives `0`).
- The formatter never mentions `BlobCursor` outside a comment (`grep -c 'BlobCursor'` over non-comment lines gives `0`).
