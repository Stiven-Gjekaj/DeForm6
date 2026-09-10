---
phase: 03-forms
plan: 15
subsystem: forms
tags: [vb6, frx, resource-blob, offset-cursor, wiring, gap-closure]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-07's Blob, extract_blob and BlobCursor (the inline blob reader and the running .frx offset cursor); plan 03-13's Form/MDIForm opcode 35 row (Icon, Picture), which makes the property loop reach the resource blob opcode on real corpus forms; plan 03-10's compose_form/compose_control composition and the CLI's print_control"
provides:
  - "A production call site for frx::extract_blob: the resource blob arm of walk_properties calls it and advances the cursor only by the count it returns"
  - "PropertyValue::Blob and PropertyValue::BlobUnreadable, the two new report states a resource property can carry, distinguishable from Undecoded and from a property the file marks absent"
  - "One BlobCursor per form, owned by compose_form and threaded by mutable reference through compose_control into walk_properties, so a blob's .frx offset is assigned in control tree order"
  - "A corrected BlobCursor::take: the real advance is declared_len + 4 (the length field's own width), not declared_len + 12, measured against eleven real gaps across two committed .frx files"
  - "crates/deform6/tests/blobs.rs: the end to end proof, through the production inspect entry point, that a live run recovers a resource blob whose bytes equal the committed .frx exactly"
affects: [04-04-frm-and-frx-writer]

actuals:
  tokens: 16739
  tasks: 3
  commits: 3
plan_head_before: af169599d34f7357e4bf40893f05903d615c6797
commits: 3

tech-stack:
  added: []
  patterns:
    - "the resource blob arm of walk_properties computes no width of its own: it calls frx::extract_blob and advances the cursor by exactly the count that call returns, the same discipline every other variable-width payload reader in this module already follows"
    - "a per-form mutable cursor (BlobCursor) is owned by the form-level composer (compose_form) and threaded by mutable reference through the control-level composer (compose_control) into the leaf reader (walk_properties), never rebuilt per control, so state that must accumulate across a whole form's own control tree does not reset at the wrong scope"
    - "a read-only group of sibling lookup tables (opcode table, external components, event names) that every composer function needs unchanged is bundled into one struct (ComposeTables) rather than threaded as three separate parameters, keeping compose_form and compose_control under clippy::too_many_arguments once a fourth, mutable, per-form parameter (blob_cursor) had to be added"
    - "a recovered value's own byte offset and declared length are carried in the report; the bytes themselves are not, when nothing downstream of the report needs a second copy of them. A consumer that needs the bytes (a phase 4 .frx writer, or this plan's own end to end test) re-reads offset..offset + 4 + declared_len out of the same executable bytes the reader itself read them from"

key-files:
  created:
    - crates/deform6/tests/blobs.rs
  modified:
    - crates/deform6/src/vb/frx.rs
    - crates/deform6/src/vb/propstream.rs
    - crates/deform6/src/vb/mod.rs
    - crates/deform6/src/vb/opcodes.rs
    - crates/deform6-cli/src/main.rs
    - crates/deform6/tests/differential.rs

key-decisions:
  - "STRUCTURES.md section 8.8's own claim that a .frx item on disk carries a separate 12 byte FRXITEMHDR (dwSizeImageEx, dwKey, dwSizeImage) replacing the executable's own inline shape does not reconcile against two committed .frx files this repository vendors. Measured directly: Fast_Flames.exe's own inline blob at file offset 0x13d5 is byte for byte identical to the whole of the committed frmFire.frx, with nothing else in between. A .frx item on disk is the same four byte length field the executable already carries inline, followed directly by declared_len bytes (the eight byte header plus the image): 4 + declared_len bytes total, never 12 + declared_len. BlobCursor::take is corrected to that measured advance; FRX_ITEM_HEADER_LEN keeps its name (per this plan's own acceptance criteria) but its value is now 4 and its doc comment states what those four bytes are (the length field itself, the one part of an item declared_len does not count)."
  - "PropertyValue::Blob carries the property name, the byte offset, the declared length, the image byte count, the detected format and the .frx offset, and never the blob's own header or image bytes: extract_blob already holds them in memory for exactly as long as one call needs them, and a second copy inside every ControlReport a form's blobs pass through would serve no consumer this plan has. This plan's own end to end test proves the recovered offset and declared_len are correct by re-reading the same byte range directly out of the corpus executable it already holds in memory and comparing that reconstruction against the committed .frx, rather than by threading a duplicate Vec<u8> through the report."
  - "A resource property the file marks absent (0xFFFFFFFF) produces no PropertyValue at all for that opcode, not an empty Blob and not an Undecoded: the file itself states nothing is there, so nothing is reported, and the loop continues to the next opcode. A resource property the file marks present but that extract_blob's own bound check refused produces PropertyValue::BlobUnreadable, a new variant distinct from both, carrying the byte offset and stopping the loop the same way every other unreadable payload in this module already does; its own defect is carried into walk_properties's returned defect list unchanged."
  - "compose_control's own argument count crossed clippy::too_many_arguments (7) once blob_cursor was added as an eighth parameter. Rather than add a ninth unrelated field or silence the lint, the three read-only lookup tables every composer already threaded unchanged (opcode_table, components, event_names) are grouped into one new ComposeTables<'a> struct, shared by compose_form and compose_control. This is a Rule 3 (blocking) fix: crates/deform6/src/vb/mod.rs is in this plan's own files_modified list, and the lint is one of AGENTS.md's three mandatory gate commands."
  - "crates/deform6-cli/src/main.rs's print_property match had to gain arms for PropertyValue::Blob and PropertyValue::BlobUnreadable to stay exhaustive the moment those variants existed; the compiler enforces this, not a choice this plan could defer to Task 3's own action text without breaking Task 2's own cargo test --workspace gate. The arm was written once, in Task 2's commit, already in the shape Task 3's own acceptance criteria ask for (byte offset, declared length, image byte count, format, .frx offset for a recovered blob; \"present and unreadable\" with the byte offset and no format for an unreadable one); Task 3 adds no further change to this file beyond what Task 2's own Rule 3 fix already committed, and its own live-run verification confirms the printed line directly."
  - "crates/deform6/src/vb/opcodes.rs's two tests asserting that opcode 35 surfaced as PropertyValue::Undecoded (true only because nothing called extract_blob yet) had to change once the wiring landed: opcode 35 now decodes into PropertyValue::Blob. Both tests now assert the Blob's own offset (the length field's own position, one byte past the opcode byte this session originally measured by hand) instead of the Undecoded opcode's position. This file is not in this plan's own files_modified list; the fix is Rule 3 (blocking), required for cargo test --workspace to pass."

patterns-established:
  - "ComposeTables<'a>: the read-only lookup-table bundle every per-form and per-control composer function threads unchanged, introduced specifically to keep an argument list under clippy's limit once a genuinely per-form mutable field (blob_cursor) needed its own parameter slot."

requirements-completed: []

# FRM-05 requires BOTH halves ("recovers the resource blobs AND writes an
# .frx whose offsets the generated .frm agrees with"). This plan completes
# only the recovery half; the writer is explicitly out of scope (deferred to
# phase 4 plan 04-04, per this plan's own frontmatter and "What this plan
# does not do"). Marking FRM-05 [x] here would repeat exactly the
# requirements-tracking overstatement 03-VERIFICATION.md found and named as
# a problem in its own right, independent of the code gap. FRM-03 is already
# [x] in REQUIREMENTS.md from an earlier plan and is unchanged by this one;
# re-declaring it here would not be a lie, but it would not be an honest
# claim that THIS plan completed it either, so it is left off this list.
# Both IDs stay in this plan's own PLAN.md frontmatter `requirements` field
# for traceability; this field states what closed, which is neither.

coverage:
  - id: D1
    description: "BlobCursor::take advances by declared_len + 4 (the .frx item's own four byte length field), corrected from declared_len + 12, proved by driving the whole declared offset sequence of two committed .frx files through the cursor and asserting every offset, reading both files at run time"
    requirement: "FRM-05"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::the_cursor_reproduces_form_physics_frxs_own_ten_declared_offsets"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::the_cursor_reproduces_frm_transparencys_own_three_declared_offsets"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::three_blobs_of_length_8_108_and_8_give_offsets_0_12_and_124"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::take_gives_a_refusal_naming_the_offset_when_the_advance_overflows_a_u32"
        status: pass
    human_judgment: false
  - id: D2
    description: "The resource blob arm of walk_properties calls frx::extract_blob and advances the cursor by the count it returns; a recovered blob becomes PropertyValue::Blob, an unreadable one becomes PropertyValue::BlobUnreadable (distinguishable from absent, which produces neither), and compose_form threads one BlobCursor per form through compose_control in control tree order"
    requirement: "FRM-05"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/propstream.rs#tests::the_resource_blob_arm_advances_by_the_count_extract_blob_returned"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/propstream.rs#tests::an_absent_resource_property_gives_no_blob_no_defect_and_does_not_stop_the_loop"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/propstream.rs#tests::an_unreadable_blob_is_reported_present_and_unreadable_with_its_byte_offset"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/propstream.rs#tests::two_forms_in_one_program_each_start_their_own_blob_cursor_at_zero"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/differential.rs#every_declared_form_and_control_is_recovered_and_every_recovered_one_is_declared_across_the_corpus"
        status: pass
    human_judgment: false
  - id: D3
    description: "A live deform6::inspect run over two independent corpus programs recovers exactly one resource blob each, and the recovered blob's length field, header and image bytes reconstruct the committed .frx beside the executable exactly, byte for byte, with the detected format matching a hand-written signature check against those same committed bytes; a live CLI run prints the blob's own facts"
    requirement: "FRM-05"
    verification:
      - kind: e2e
        ref: "crates/deform6/tests/blobs.rs#fast_flames_frm_fire_recovers_a_blob_matching_the_committed_frx"
        status: pass
      - kind: e2e
        ref: "crates/deform6/tests/blobs.rs#winsock_sample_frm_main_recovers_a_blob_matching_the_committed_frx"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/blobs.rs#fast_flames_blob_declared_length_and_image_length_match_this_sessions_own_measurement"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/blobs.rs#winsock_sample_blob_declared_length_and_image_length_match_this_sessions_own_measurement"
        status: pass
      - kind: other
        ref: "cargo run -p deform6-cli -- inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe (prints: Icon: resource blob at offset 0x13d5, declared length 1414, 1406 image byte(s), format ICO, .frx offset 0x0)"
        status: pass
    human_judgment: false

duration: 24min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 15: Wire the Resource Blob Reader Into the Property Loop Summary

**`frx::extract_blob` now has a production call site — the resource blob arm of `walk_properties` calls it and a live `inspect` run recovers a blob whose bytes equal the committed `.frx` byte for byte — and `BlobCursor::take`'s own advance is corrected from `declared_len + 12` to the measured `declared_len + 4`.**

## Performance

- **Duration:** 24 min (measured from the previous plan's completion commit to this plan's final code commit)
- **Started:** 2026-09-10T22:31:44Z (approximate, previous plan's completion)
- **Completed:** 2026-09-10T22:55:46Z
- **Tasks:** 3
- **Files modified:** 6 (1 created, 6 modified — `crates/deform6/tests/blobs.rs` is new)

## Accomplishments

- `BlobCursor::take` advances by `declared_len + 4`, not `declared_len + 12`. Measured directly against two committed `.frx` files this repository vendors: `corpus/vb6-code/Game-physics-basic/FormPhysics.frx` (ten declared offsets, nine real gaps) and `corpus/vb6-code/Transparency-2D/frmTransparency.frx` (three declared offsets, two real gaps), eleven independent measurements in total, every one of them `declared_len + 4`, with each file ending exactly at its last item's own end. `Fast_Flames.exe`'s own inline blob at file offset `0x13d5` is byte for byte identical to the whole of the committed `frmFire.frx`: the `.frx` item on disk is the same four byte length field the executable already carries inline, followed directly by `declared_len` bytes, with no separate twelve byte header in between. Two new tests drive the whole declared offset sequence of both committed files through the cursor and assert every offset, reading both files at run time.
- `walk_properties`'s resource blob arm calls `frx::extract_blob` and advances its own cursor by exactly the count that call returns, computing no width of its own. `PropertyValue` gains two variants: `Blob` (the property name, the byte offset, the declared length, the image byte count, the detected format, and the `.frx` offset `BlobCursor::take` gave it — never the blob's own bytes) and `BlobUnreadable` (present in the file but this reader could not read it, carrying its own byte offset, distinguishable from a property the file marks absent, which produces neither a value nor a defect and does not stop the loop).
- `compose_form` (`vb/mod.rs`) owns one `BlobCursor` per form and threads it by mutable reference through `compose_control` into `walk_properties`, so a blob's `.frx` offset is assigned in the control tree order the property stream holds, and a second form never carries over the first form's cursor.
- `crates/deform6/tests/blobs.rs` is the end to end proof this phase's own verification report named as missing. It calls the production `deform6::inspect` entry point, never `frx::extract_blob` directly, over two independent corpus programs (`Fast_Flames.exe`'s `frmFire`, `SubReality_WinsockSample.exe`'s `frmMain`) and asserts each recovers exactly one resource blob whose length field, header and image bytes reconstruct the committed `.frx` beside the executable exactly, comparing against that committed file and never against anything DeForm6 produced.
- The CLI's `print_property` prints a recovered blob's own byte offset, declared length, image byte count, detected format and `.frx` offset on one line, and prints an unreadable blob as present and unreadable with its byte offset and no format, printing no image byte in either case. A live run confirms it: `Icon: resource blob at offset 0x13d5, declared length 1414, 1406 image byte(s), format ICO, .frx offset 0x0 (an offset into a file this run did not write)`.
- This plan writes no file. `inspect` still opens no file and writes none; `crates/deform6-cli/tests/cli.rs`'s own `inspecting_the_corpus_file_changes_no_file_on_disk` test is unchanged and still passes. Phase 4's plan 04-04 owns the `.frx` writer.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: Correct the .frx offset cursor against the committed offsets** - `b90ace6` (fix, tdd="true")
2. **Task 2: Call extract_blob from the property loop and carry the blob into the report** - `86745fd` (feat, tdd="true")
3. **Task 3: Print the blob and prove it end to end against the committed .frx** - `7901e15` (test, tdd="true")

**Plan metadata:** commit follows this SUMMARY.

_Note: all three tasks carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase), each task's RED phase was run once against a deliberately broken or reverted implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task._

## Files Created/Modified

- `crates/deform6/src/vb/frx.rs` — `FRX_ITEM_HEADER_LEN` corrected from 12 to 4 with a new doc comment; two new corpus-driven offset-sequence tests; module doc comment records the measurement (Task 1)
- `crates/deform6/src/vb/propstream.rs` — `PropertyValue::Blob`, `PropertyValue::BlobUnreadable`; the resource blob arm calls `frx::extract_blob`; `walk_properties` gains a `blob_cursor: &mut BlobCursor` parameter; `blob_cursor_defect`; four new tests (Task 2)
- `crates/deform6/src/vb/mod.rs` — `ComposeTables<'a>`; `compose_form` owns one `BlobCursor` per form; `compose_control` threads it and `ComposeTables` (Task 2)
- `crates/deform6/tests/differential.rs` — `recovered_property_name`/`recovered_property_text` gain `Blob`/`BlobUnreadable` arms (Task 2)
- `crates/deform6/src/vb/opcodes.rs` — two tests updated from asserting `Undecoded` to asserting the decoded `Blob`'s own offset (Task 2, Rule 3 deviation)
- `crates/deform6-cli/src/main.rs` — `print_property` gains `Blob`/`BlobUnreadable` arms; `format_image_format` (Task 2, Rule 3 deviation; already in the shape Task 3 needed)
- `crates/deform6/tests/blobs.rs` — new; the end to end proof, four tests (Task 3)

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential: `FRX_ITEM_HEADER_LEN`'s real value is `4`, not `12`, measured against eleven real gaps across two committed `.frx` files, directly contradicting `STRUCTURES.md` section 8.8's own cited `FRXITEMHDR` account; and `PropertyValue::Blob` carries a recovered blob's facts (offset, declared length, image length, format, `.frx` offset) but never its bytes, because nothing downstream of the report needs a second copy when the bytes are still one `offset..offset + 4 + declared_len` slice away from the same executable bytes the reader already read them from.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `compose_control`'s own argument count crossed clippy's limit**
- **Found during:** Task 2, adding `blob_cursor: &mut frx::BlobCursor` as an eighth parameter
- **Issue:** `cargo clippy --all-targets -- -D warnings` (one of `AGENTS.md`'s three mandatory gate commands) failed with `clippy::too_many_arguments` (8/7).
- **Fix:** Grouped the three read-only lookup tables every composer already threaded unchanged (`opcode_table`, `components`, `event_names`) into one new `ComposeTables<'a>` struct, shared by `compose_form` and `compose_control`, bringing both functions back under the limit.
- **Files modified:** `crates/deform6/src/vb/mod.rs`
- **Verification:** `cargo clippy --all-targets -- -D warnings` passes clean; `cargo test --workspace` unaffected
- **Committed in:** `86745fd` (Task 2 commit)

**2. [Rule 3 - Blocking] `crates/deform6-cli/src/main.rs`'s `print_property` match had to stay exhaustive**
- **Found during:** Task 2, adding `PropertyValue::Blob` and `PropertyValue::BlobUnreadable`
- **Issue:** `print_property`'s own `match property { ... }` is exhaustive with no wildcard arm; adding two new `PropertyValue` variants is a compile error in this file until it is updated. `main.rs` is not in Task 2's own `<files>` list (it is in Task 3's), but the compiler does not honor task boundaries.
- **Fix:** Added `Blob` and `BlobUnreadable` arms, already in the shape Task 3's own acceptance criteria describe (byte offset, declared length, image byte count, format, `.frx` offset for a recovered blob; present-and-unreadable with the byte offset and no format for an unreadable one), plus a `format_image_format` helper. Task 3 makes no further change to this file; its own live-run verification (`cargo run -p deform6-cli -- inspect ...`) confirms the printed line directly.
- **Files modified:** `crates/deform6-cli/src/main.rs`
- **Verification:** `cargo build --workspace --tests` compiles; live run prints the expected line; `crates/deform6-cli/tests/cli.rs`'s own suite (26 tests) still passes unchanged
- **Committed in:** `86745fd` (Task 2 commit)

**3. [Rule 3 - Blocking] Two tests in `crates/deform6/src/vb/opcodes.rs` asserted the pre-wiring behaviour**
- **Found during:** Task 2, running `cargo test --workspace` after wiring `extract_blob`
- **Issue:** `fast_flames_form_reaches_opcode_35_at_the_measured_offset` and `winsock_sample_form_reaches_opcode_35_at_the_measured_offset` (plan 03-13's own tests) asserted that opcode 35 surfaced as `PropertyValue::Undecoded` at the opcode byte's own offset — true only because nothing called `extract_blob` yet. Once wired, opcode 35 decodes into `PropertyValue::Blob`, and these two tests failed. `opcodes.rs` is not in this plan's own `files_modified` list.
- **Fix:** Both tests now assert the decoded `Blob`'s own `offset` field (the four byte length field's own position, one byte past the opcode byte this session originally measured by hand: `0x13d5` for `Fast_Flames.exe`, `0x1302` for `SubReality_WinsockSample.exe`) instead of the `Undecoded` opcode's position. The `undecoded_offset` helper is replaced with `blob_offset`.
- **Files modified:** `crates/deform6/src/vb/opcodes.rs`
- **Verification:** `cargo test -p deform6 --lib vb::opcodes` passes; both new assertions independently confirmed against the raw executable bytes before the fix was written
- **Committed in:** `86745fd` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 3, blocking).
**Impact on plan:** All three were necessary for the code to compile or for `cargo test --workspace` to pass, exactly the discipline every prior plan in this phase has followed for the same class of issue. No scope creep: each fix is narrowly scoped to the exhaustiveness or lint requirement that forced it.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's cursor advance** — `FRX_ITEM_HEADER_LEN` changed back to `12`. `cargo test -p deform6 --lib vb::frx::tests::the_cursor_reproduces` run once (both new offset-sequence tests):

```
thread 'vb::frx::tests::the_cursor_reproduces_frm_transparencys_own_three_declared_offsets' panicked at crates/deform6/src/vb/frx.rs:714:9:
assertion `left == right` failed: vb6-code/Transparency-2D/frmTransparency.frm's own declared offset sequence must round-trip through the cursor exactly; see the module doc comment for how this was measured
  left: [0, 20, 45781]
 right: [0, 12, 45765]

thread 'vb::frx::tests::the_cursor_reproduces_form_physics_frxs_own_ten_declared_offsets' panicked at crates/deform6/src/vb/frx.rs:714:9:
assertion `left == right` failed: vb6-code/Game-physics-basic/FormPhysics.frm's own declared offset sequence must round-trip through the cursor exactly; see the module doc comment for how this was measured
  left: [0, 124, 248, 7749, 15664, 22374, 29603, 36413, 43602, 51103]
 right: [0, 116, 232, 7725, 15632, 22334, 29555, 36357, 43538, 51031]

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 400 filtered out; finished in 0.00s
```

Reverted to `FRX_ITEM_HEADER_LEN = 4` before committing `b90ace6`.

**Task 2's resource blob arm** — the arm's body changed back to the version that reports the opcode as not decoded and stops the loop. `cargo test -p deform6 --lib vb::propstream::tests::the_resource_blob_arm_advances_by_the_count_extract_blob_returned` run once:

```
thread 'vb::propstream::tests::the_resource_blob_arm_advances_by_the_count_extract_blob_returned' panicked at crates/deform6/src/vb/propstream.rs:1598:9:
assertion `left == right` failed: [Undecoded { opcode: 1, offset: 12, control_type: "CommandButton", bytes_not_read: 19 }]
  left: 1
 right: 2

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 405 filtered out; finished in 0.00s
```

Only one property (`Undecoded`) reached the list instead of the expected two (`Blob` then `Byte`), because the arm stopped the loop at the very opcode it should have decoded. Reverted to the wired arm before committing `86745fd`.

**Task 3's whole end to end file, the single piece of evidence this plan most depends on** — the resource blob arm changed back to the version that reports the opcode as not decoded, exactly as in Task 2's own break above. `cargo test -p deform6 --test blobs` run once:

```
running 4 tests
test winsock_sample_blob_declared_length_and_image_length_match_this_sessions_own_measurement ... FAILED
test winsock_sample_frm_main_recovers_a_blob_matching_the_committed_frx ... FAILED
test fast_flames_frm_fire_recovers_a_blob_matching_the_committed_frx ... FAILED
test fast_flames_blob_declared_length_and_image_length_match_this_sessions_own_measurement ... FAILED

failures:

---- fast_flames_frm_fire_recovers_a_blob_matching_the_committed_frx stdout ----
thread 'fast_flames_frm_fire_recovers_a_blob_matching_the_committed_frx' panicked at crates/deform6/tests/blobs.rs:106:5:
assertion `left == right` failed: expected exactly one recovered blob in this form's own property list, found 0
  left: 0
 right: 1

test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

All four tests failed the same way: `the_one_recovered_blob` panicked with "found 0", because no `PropertyValue::Blob` reached the property list once the wiring was removed. This is the evidence that this test would have caught the gap the phase 3 verification found: a unit test on `extract_blob` alone (already twenty of them, since plan 03-07) would never have failed this way, because it was never calling the production path in the first place. Reverted to the wired arm before committing `7901e15`.

## The eleven measured gaps (Task 1's own acceptance criteria)

**`corpus/vb6-code/Game-physics-basic/FormPhysics.frx`** (58938 bytes total), against the ten offsets `corpus/vb6-code/Game-physics-basic/FormPhysics.frm` declares:

| Declared offset | Declared length at it | Real gap to the next declared offset (or to the file's own end) |
|---|---|---|
| `0x0000` | 112 | 116 |
| `0x0074` | 112 | 116 |
| `0x00e8` | 7489 | 7493 |
| `0x1e2d` | 7903 | 7907 |
| `0x3d10` | 6698 | 6702 |
| `0x573e` | 7217 | 7221 |
| `0x7373` | 6798 | 6802 |
| `0x8e05` | 7177 | 7181 |
| `0xaa12` | 7489 | 7493 |
| `0xc757` | 7903 | 7907 (the file ends exactly here: `0xc757 + 7907 = 58938`) |

**`corpus/vb6-code/Transparency-2D/frmTransparency.frx`** (67074 bytes total), against the three offsets `corpus/vb6-code/Transparency-2D/frmTransparency.frm` declares:

| Declared offset | Declared length at it | Real gap to the next declared offset (or to the file's own end) |
|---|---|---|
| `0x0000` | 8 | 12 |
| `0x000c` | 45749 | 45753 |
| `0xb2c5` | 21305 | 21309 (the file ends exactly here: `0xb2c5 + 21309 = 67074`) |

Every one of the eleven gaps equals the declared length plus 4. Which bytes the declared length already counts, in one sentence: `declared_len` (`blobLen`) counts the eight byte inline picture header plus the image bytes; it never counts the four byte length field itself, which is the one part of a `.frx` item on disk that `4` (not `12`) accounts for.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every symbol this plan's own artifact table names (`PropertyValue::Blob`, `PropertyValue::BlobUnreadable`, the resource blob arm's call to `frx::extract_blob`, `ComposeTables`, `compose_form`'s `BlobCursor`, `crates/deform6/tests/blobs.rs`) is implemented, wired into the production path, and tested — none is a placeholder. `crates/deform6-cli/src/main.rs`'s print arm, while written during Task 2 as a Rule 3 fix, is complete and matches Task 3's own acceptance criteria in full; it is not left half-done for a later plan.

## Threat Flags

None beyond what this plan's own `<threat_model>` already names and mitigates. T-03-77 (the four byte blob length sizing a copy through a newly reachable path): the resource blob arm computes no width of its own; every bound check stays inside `extract_blob`, unchanged from plan 03-07. T-03-78 (a wrong `.frx` offset cursor reaching phase 4's writer): closed by Task 1's own eleven-gap measurement and the two offset-sequence tests. T-03-79 (a recovered blob verified against this repository's own output): closed by `blobs.rs` comparing against the committed `.frx`, read at run time, never against anything DeForm6 produced. T-03-80 (a blob that could not be read reported as absent): `PropertyValue::BlobUnreadable` is a distinct, tested state. T-03-81 (a resource file written to disk): this plan writes no file, unchanged. T-03-82 (a fixture derived from a corpus binary entering the repository): `blobs.rs` reads both corpus files at run time; `grep -c 'sha256' crates/deform6/tests/blobs.rs` gives `0`. T-03-83 (image bytes printed to the terminal): the print arm names counts, offsets and a format name only; it prints no image byte, confirmed by reading `print_property`'s own `Blob` arm.

## Next Phase Readiness

- `frx::extract_blob` has a production call site, and phase 3's own product surface (a live `inspect` run) now surfaces a resource blob for the first time in this phase's history: the gap `03-VERIFICATION.md` called "the most important gap the phase 3 verification found" is closed.
- `BlobCursor::take`'s corrected advance is exactly the fact phase 4's plan 04-04 (the `.frm`/`.frx` writer) needs to place a blob's bytes at the offset the generated `.frm` will name; a wrong advance here would have silently corrupted every `.frx` file that writer produces.
- This plan writes no file, per its own scope boundary; plan 04-04 still owns the `.frx` writer in full, unstarted by this plan.
- No blockers for plan 03-16 (the OCX CLSID field) or plan 03-17 (the remaining code review findings), the next two gap closure plans in this phase's own wave ordering.

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

All 6 modified files (`crates/deform6/src/vb/frx.rs`, `crates/deform6/src/vb/propstream.rs`, `crates/deform6/src/vb/mod.rs`, `crates/deform6/src/vb/opcodes.rs`, `crates/deform6-cli/src/main.rs`, `crates/deform6/tests/differential.rs`) and the one created file (`crates/deform6/tests/blobs.rs`) found on disk. All 3 task commits (`b90ace6`, `86745fd`, `7901e15`) found in `git log`. `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` all pass clean. `cargo test -p deform6 --test blobs` gives 4 tests, meeting this plan's own stated minimum. `cargo run -p deform6-cli -- inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe` prints a resource blob line naming its byte offset, declared length, image byte count, format and `.frx` offset. `grep -n 'frx::extract_blob' crates/deform6/src/vb/propstream.rs` and `grep -n 'BlobCursor' crates/deform6/src/vb/mod.rs` both give non-empty results. `git status --short` and `git diff --stat` confirm no file under `corpus/` and no binary file of any kind entered the repository in this plan; `grep -c 'sha256' crates/deform6/tests/blobs.rs` gives `0`.
