---
phase: 03-forms
plan: 13
subsystem: forms
tags: [vb6, opcode-table, corpus-measurement, differential-gate, gap-closure]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-02's OpcodeTable, OpcodeEntry, PayloadType and the safe-provenance builtin subset this plan extends; plan 03-06's walk_properties, which already stops honestly at the first opcode with no table entry; plan 03-10's differential.rs corpus-wide two-directional property diff, which this plan wires two new value shapes into"
provides:
  - "Three corpus-measured Form/MDIForm opcode rows (1: Caption/Text, 3: BackColor/Long, 35: Icon/Picture), each proved in at least two independent corpus programs against the committed .frm source"
  - "CORPUS_MEASURED, a provenance constant distinct from SVBD_OPCODE_AND_TYPE/SVBD_POSITION_BLOCK, naming this repository's own corpus measurement per 03-CONTEXT.md D-01's second allowed source"
  - "insert_builtin_rows takes its default provenance string from the caller instead of choosing it from the payload shape, so a mixed-provenance rows array (STRUCTURES.md-transcribed and corpus-measured) is representable"
  - "An honest corpus-wide comparison for a Long-typed colour (rendered as a .frm hex literal) and for a declared value naming a resource file (excluded from the text comparison, with the reason recorded beside the skip)"
  - "A fix to support/frm.rs's own value parser: a doubled quote inside a quoted .frm value unescapes to one literal quote, VB6's own escaping convention, found the first time a Text payload reached the corpus-wide gate"
affects: [03-15-frx-wiring]

actuals:
  tokens: 8616
  tasks: 3
  commits: 3
plan_head_before: 0e5f1844e01ddc6035d821bf17c6376ca6afe7e2
commits: 3

tech-stack:
  added: []
  patterns:
    - "a rows array's default provenance string is a parameter insert_builtin_rows's caller passes, not a value the function derives from the payload shape; a PayloadType::Position row still always cites SVBD_POSITION_BLOCK regardless of the caller's default, because that fact's own source never changes"
    - "a value shape a .frm writer renders differently from how the library reports it (a colour: hex literal vs signed decimal) is reconciled in the differential test's own rendering function, never by changing the library's own recovered value; the render is proved non-collapsing (two distinct values still render two distinct texts) so the comparison it feeds can still fail"
    - "a declared value that names a resource file is detected by shape ($\"name.frx\":OFFSET or \"name.frx\":OFFSET) and excluded from a Text property's text comparison only, with the reason recorded beside the skip; the property itself is still counted as recovered"

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/opcodes.rs
    - crates/deform6/tests/differential.rs
    - crates/deform6/tests/support/frm.rs

key-decisions:
  - "insert_builtin_rows's signature gained a default_source: &'static str parameter instead of splitting FORM_ROWS into more call sites: every existing call site now passes SVBD_OPCODE_AND_TYPE explicitly (byte-for-byte identical resulting entries), and a new FORM_CORPUS_ROWS call site per Form/MDIForm passes CORPUS_MEASURED. A PayloadType::Position row still always resolves to SVBD_POSITION_BLOCK inside the function, unconditionally on the caller's default, because every position row this table carries today came from that one SVBD fact regardless of which rows array holds it."
  - "Opcode 35's property name is 'Icon', not the payload type's own name 'Picture': the committed .frm source names the property Icon on both measured corpus programs, and every other row in this table already uses the .frm's own property name, not its payload shape's name."
  - "A colour's own numeric semantics needed no reconciliation, only its text rendering: &H80000005& is VB6's literal for the exact 32-bit pattern 0x80000005, which is also what the file's own bytes hold and what the library already reports as a signed i32 (-2147483643). Only the differential test's own rendering changed (Long -> &H{bits:08X}&, a bit-for-bit reinterpretation via u32::cast_signed/cast_unsigned, never an arithmetic conversion); PropertyValue::Long's own value is untouched, since how a .frm writes it is the .frm writer's concern, which phase 4 owns."
  - "[Rule 1 - Bug, found via the corpus-wide gate] support/frm.rs's split_property_line stripped only a leading and trailing quote and left an interior doubled quote (\"\") untouched. Sepia.frm's own Caption line, 'Sepia / \"\"Antique\"\" Effect - www.tannerhelland.com', is VB6's own escape for one embedded quote on each side, not two: the compiled bytes hold a single quote, and the corpus-wide gate (the first plan to decode a Text payload against it) failed this exact form until the harness's own value parser was fixed to unescape a doubled quote to one, matching VB6's own convention."
  - "Every commit in this plan landed on main, per this session's explicit sequential-executor instructions ('work on the main working tree, use normal git commits') and this project's own git.branching_strategy: none, matching every prior plan in this phase (03-01 through 03-12)."
  - "The CORPUS_MEASURED constant's doc comment was written once, in Task 1's commit, naming all three facts' corpus programs and offsets together, rather than extended incrementally task by task as the plan's own action text suggests: all three measurements were taken together during this session's own research phase, before Task 1's code was written, so writing the doc comment in three separate edits would have documented facts already known rather than facts newly discovered mid-task. The code itself (the FORM_CORPUS_ROWS entries and their tests) still landed one row per task, one commit per task, as the plan requires."

patterns-established:
  - "A rows array whose entries share no single provenance fact (STRUCTURES.md-transcribed rows mixed with corpus-measured ones under the same control type) is represented as two separate arrays under the same control type, each inserted through its own insert_builtin_rows call with its own default_source, rather than one array carrying a source field per row."

requirements-completed: [FRM-03]

coverage:
  - id: D1
    description: "OpcodeTable::builtin carries a Form/MDIForm row for opcode 1 (Caption, Text), measured against two independent corpus programs and proved through a live inspect run against the committed .frm source; a declared value naming a resource file is excluded from the corpus-wide text comparison instead of failing it"
    requirement: "FRM-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::fast_flames_form_recovers_the_real_caption_from_the_committed_frm"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::winsock_sample_form_recovers_the_real_caption_from_the_committed_frm"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::form_opcode_1_gives_caption_with_a_text_payload_and_the_corpus_measured_source"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/differential.rs#declared_value_names_a_resource_file_recognises_both_frx_shapes_and_not_a_plain_literal"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/differential.rs#every_declared_form_and_control_is_recovered_and_every_recovered_one_is_declared_across_the_corpus"
        status: pass
    human_judgment: false
  - id: D2
    description: "OpcodeTable::builtin carries a Form/MDIForm row for opcode 3 (BackColor, Long), measured against a system colour and a literal colour in two independent corpus programs; the corpus-wide comparison renders a recovered colour the way a .frm writes it, and the rendering is proved to still fail on a genuine mismatch"
    requirement: "FRM-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::fast_flames_form_recovers_the_real_system_back_color_from_the_committed_frm"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::brightness_form_recovers_a_real_literal_back_color_from_the_committed_frm"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/differential.rs#recovered_property_text_renders_a_long_value_as_the_frm_colour_literal_shape"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/differential.rs#a_recovered_colour_one_greater_than_the_declared_value_still_renders_a_different_text"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/differential.rs#a_doctored_back_color_one_greater_than_the_declared_value_fails_the_full_comparison"
        status: pass
    human_judgment: false
  - id: D3
    description: "OpcodeTable::builtin carries a Form/MDIForm row for opcode 35 (Icon, Picture), so the property loop reaches the resource blob opcode instead of stopping four opcodes before it, proved reachable in two independent corpus programs at the byte offset this session measured by hand"
    requirement: "FRM-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::fast_flames_form_reaches_opcode_35_at_the_measured_offset"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::winsock_sample_form_reaches_opcode_35_at_the_measured_offset"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::a_control_type_outside_the_form_group_resolves_the_same_rows_it_resolved_before"
        status: pass
      - kind: other
        ref: "cargo run -p deform6-cli -- inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe (property opcode 35 at offset 0x13d4 on Form: not decoded)"
        status: pass
      - kind: other
        ref: "cargo run -p deform6-cli -- inspect corpus/public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe (property opcode 35 at offset 0x1301 on Form: not decoded)"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 13: The Three Form Opcode Rows That Reach The Resource Blob Summary

**Three corpus-measured Form/MDIForm opcode rows (Caption, BackColor, Icon) so the property loop reaches opcode 35 instead of stopping at opcode 1, with an honest colour rendering and a resource-file exclusion wired into the corpus-wide differential gate.**

## Performance

- **Duration:** 20 min (approximate; measured from the previous plan's completion commit to this plan's final code commit)
- **Started:** 2026-09-10T23:07:48+02:00 (approximate)
- **Completed:** 2026-09-10T23:27:42+02:00
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- `OpcodeTable::builtin` resolves Form/MDIForm opcode 1 to `Caption` (a `Text` payload), opcode 3 to `BackColor` (a `Long` payload), and opcode 35 to `Icon` (the `Picture`/resource blob shape), each measured directly against real corpus bytes rather than transcribed from prior art, and each carrying a new `CORPUS_MEASURED` provenance string distinct from the existing `SVBD_OPCODE_AND_TYPE`/`SVBD_POSITION_BLOCK` constants.
- `insert_builtin_rows` takes its default provenance string from the caller instead of choosing it from the payload shape. Every existing call site now passes `SVBD_OPCODE_AND_TYPE` explicitly, byte-for-byte unchanged; a `PayloadType::Position` row still always cites `SVBD_POSITION_BLOCK` regardless of the caller's default. Two new `FORM_CORPUS_ROWS` call sites (one per `CT_FORM`/`CT_MDIFORM`) pass `CORPUS_MEASURED`.
- The property loop now reaches opcode 35 on a real corpus form: `Fast_Flames.exe`'s own `frmFire` and `SubReality_WinsockSample.exe`'s own `frmMain` both advance past Caption, BackColor and the two already-handled special opcodes (25/ScaleMode, 0/no-output) to reach the resource blob, where before this plan the loop stopped at the very first opcode, 1.
- The corpus-wide two-directional property diff in `differential.rs` now compares two new value shapes honestly: a `Long`-typed colour renders as a `.frm` hex literal (`&H{bits:08X}&`, a bit-for-bit reinterpretation, never an arithmetic conversion) so `&H80000005&` and the recovered `-2147483643` compare equal while a genuine mismatch still fails; a declared value naming a resource file (`"name.frx":OFFSET` or `$"name.frx":OFFSET`) is excluded from a `Text` property's text comparison, with the reason recorded beside the skip, and the property is still counted as recovered.
- Found and fixed a real bug in the test harness's own second, independent `.frm` reader: `support/frm.rs`'s value parser did not unescape a doubled quote (`""`) inside a quoted value to one literal quote, VB6's own escaping convention. `Sepia.frm`'s own `Caption` line failed the corpus-wide gate until this was fixed — the first time a `Text` payload reached that gate at all.
- `crates/deform6/src/vb/propstream.rs` is unchanged by this plan: its `Picture` arm still reports the opcode as not decoded and stops the loop, correct while nothing reads the blob. Plan 03-15 wires `frx::extract_blob` and owns that file; this plan only makes the loop reach that arm on real corpus bytes.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: The provenance seam and the Form caption row** - `e54cdfe` (feat, tdd="true")
2. **Task 2: The Form colour row and an honest colour comparison** - `43aa563` (feat, tdd="true")
3. **Task 3: The Form resource blob row** - `2207870` (feat, tdd="true")

**Plan metadata:** commit follows this SUMMARY.

_Note: all three tasks carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase), each task's RED phase was run once against a deliberately broken row (a wrong payload shape for Tasks 1 and 2, a wrong opcode number for Task 3), its failure evidence captured below, then reverted to the correct row before committing code and tests together in one commit per task. Task 2 also ran a second RED phase against the colour-rendering fix itself, proving the mismatch-detection tests can fail._

## Files Created/Modified

- `crates/deform6/src/vb/opcodes.rs` — `CORPUS_MEASURED`, `FORM_CORPUS_ROWS` (opcodes 1, 3, 35), `insert_builtin_rows`'s new `default_source` parameter, and 12 new unit/corpus tests across the three tasks
- `crates/deform6/tests/differential.rs` — `declared_value_names_a_resource_file`, the resource-file exclusion in `form_control_mismatches`, `recovered_property_text`'s colour-hex rendering for `Long`, and 6 new tests
- `crates/deform6/tests/support/frm.rs` — `split_property_line`'s doubled-quote unescape (Rule 1 bug fix) and its own unit test

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The most consequential: `insert_builtin_rows` gained a `default_source` parameter rather than a per-row source field, keeping the mechanical seam change small; opcode 35's property name is `Icon` (the `.frm`'s own name), not `Picture` (the payload shape's own name); and a real bug in the test harness's own `.frm` value parser (doubled-quote unescaping) was found and fixed via the corpus-wide gate itself, the first time a `Text` payload reached it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `support/frm.rs`'s value parser did not unescape a doubled quote inside a quoted `.frm` value**
- **Found during:** Task 1, first `cargo test -p deform6 --test differential` run after adding the Caption row
- **Issue:** `Sepia.frm`'s own `Caption` line reads `"Sepia / ""Antique"" Effect - www.tannerhelland.com"`. VB6's own `.frm` writer escapes a literal `"` inside a quoted string by doubling it; the compiled binary holds a single `"` on each side. `split_property_line` only stripped the outer matching quote pair and left the interior `""` untouched, so the corpus-wide gate compared a doubled-quote declared value against a single-quote recovered one and failed a form the library actually reads correctly.
- **Fix:** `split_property_line` now calls `.replace("\"\"", "\"")` on the value after confirming the outer quote pair is present, so a value that is not quoted at all (a bare number, a `&H...&` colour, a `"name.frx":OFFSET` resource reference whose own trailing digits mean it never ends with a quote) is never touched.
- **Files modified:** `crates/deform6/tests/support/frm.rs`
- **Verification:** `a_doubled_quote_inside_a_quoted_value_unescapes_to_one_quote` (new, synthetic fixture); `every_declared_form_and_control_is_recovered_and_every_recovered_one_is_declared_across_the_corpus` (the corpus-wide gate itself, which surfaced this)
- **Committed in:** `e54cdfe` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1, bug).
**Impact on plan:** Necessary for the corpus-wide differential gate to pass once a `Text` payload reached it for the first time in this phase; the fix is scoped to the test harness's own value parser, touches no `src/` file, and is proved by both a synthetic unit test and the corpus-wide gate itself. No scope creep.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's Caption row** — payload changed from `PayloadType::Text` to `PayloadType::Byte`. `cargo test -p deform6 --lib vb::opcodes` run once:

```
thread 'vb::opcodes::tests::form_opcode_1_gives_caption_with_a_text_payload_and_the_corpus_measured_source' panicked:
  left: Byte
 right: Text

thread 'vb::opcodes::tests::winsock_sample_form_recovers_the_real_caption_from_the_committed_frm' panicked:
frmMain's own Caption must now resolve, not stop the loop at opcode 1

thread 'vb::opcodes::tests::fast_flames_form_recovers_the_real_caption_from_the_committed_frm' panicked:
frmFire's own Caption must now resolve, not stop the loop at opcode 1

test result: FAILED. 15 passed; 3 failed
```

Reverted to `PayloadType::Text` before committing `e54cdfe`.

**Task 2's BackColor row** — payload changed from `PayloadType::Long` to `PayloadType::Byte`. `cargo test -p deform6 --lib vb::opcodes` run once:

```
thread 'vb::opcodes::tests::form_opcode_3_gives_back_color_with_a_long_payload_and_the_corpus_measured_source' panicked:
  left: Byte
 right: Long

thread 'vb::opcodes::tests::brightness_form_recovers_a_real_literal_back_color_from_the_committed_frm' panicked:
frmBrightness's own BackColor must now resolve

thread 'vb::opcodes::tests::fast_flames_form_recovers_the_real_system_back_color_from_the_committed_frm' panicked:
frmFire's own BackColor must now resolve

test result: FAILED. 18 passed; 3 failed
```

Reverted to `PayloadType::Long` before committing `43aa563`.

**Task 2's colour-rendering fix, a second RED phase** — `recovered_property_text`'s `Long` arm changed to always return the fixed string `"&H80000005&"` regardless of the actual value, to prove the mismatch-detection tests can fail. `cargo test -p deform6 --test differential -- a_recovered_colour_one_greater a_doctored_back_color` run once:

```
thread 'a_recovered_colour_one_greater_than_the_declared_value_still_renders_a_different_text' panicked:
assertion `left != right` failed: a colour that differs by one must still render a different text
  left: Some("&H80000005&")
 right: Some("&H80000005&")

thread 'a_doctored_back_color_one_greater_than_the_declared_value_fails_the_full_comparison' panicked:
a BackColor doctored one greater than the declared value must still fail the comparison: []

test result: FAILED. 0 passed; 2 failed
```

Reverted to the bit-for-bit `&H{:08X}&` rendering before committing `43aa563`.

**Task 3's resource blob row** — the row's own opcode number changed from `35` to `36`. `cargo test -p deform6 --lib vb::opcodes` run once:

```
thread 'vb::opcodes::tests::form_opcode_35_gives_icon_with_a_picture_payload_and_the_corpus_measured_source' panicked:
called `Option::unwrap()` on a `None` value

test result: FAILED. 24 passed; 1 failed
```

Reverted to `35` before committing `2207870`.

## Issues Encountered

None beyond the deviation above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. `crates/deform6/src/vb/propstream.rs`'s `PayloadType::Picture` arm remains an intentional, already-documented stub from plan 03-06: it reports `PropertyValue::Undecoded` and stops the loop. This plan does not touch that file (verified: `grep -c extract_blob crates/deform6/src/vb/propstream.rs` gives `0`) and does not claim to close it; plan 03-15 owns wiring `frx::extract_blob` into that arm, and this plan's own purpose is only to make the loop reach it on a real corpus form.

## Threat Flags

None. Every trust boundary this plan's own `<threat_model>` names (a wrong payload width shifting the cursor for every later property, a text payload's declared length sizing a read past the block end, a row transcribed from a Microsoft type library, the corpus-wide comparison rendered into agreement rather than comparability, a skipped comparison hiding a property the gate no longer covers, the property loop reading past the control block end at a newly reachable opcode) is mitigated exactly as the threat register states: T-03-65 and T-03-70 by the corpus-wide two-directional property diff running on all 44 programs before each commit landed; T-03-66 by `VbStr::read`'s own unchanged bound check (this plan adds no new read path); T-03-67 by `CORPUS_MEASURED`'s own doc comment naming the corpus programs and by the `grep -ci OLB` acceptance check, which gives `0` outside comments; T-03-68 by the two colour-rendering tests proving the comparison can still fail; T-03-69 by the resource-file exclusion still counting the property as recovered and only skipping its text comparison, with the reason recorded beside the skip.

## Next Phase Readiness

- `FORM_CORPUS_ROWS` and `CORPUS_MEASURED` are ready for a later plan to extend with further corpus-measured facts, following the same `insert_builtin_rows`-with-`default_source` seam.
- The property loop reaches opcode 35 on real corpus forms, which is what plan 03-15's own end-to-end test depends on: wiring `frx::extract_blob` into `propstream.rs`'s `Picture` arm now has something to run against.
- No blockers for plan 03-14 or plan 03-15.

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

All 3 modified files (`crates/deform6/src/vb/opcodes.rs`,
`crates/deform6/tests/differential.rs`, `crates/deform6/tests/support/frm.rs`)
and this SUMMARY found on disk. All 3 task commits (`e54cdfe`, `43aa563`,
`2207870`) found in `git log`. `cargo fmt --all --check`, `cargo clippy
--all-targets -- -D warnings` and `cargo test --workspace` all pass clean,
including the corpus-wide differential gate across all 44 programs.
