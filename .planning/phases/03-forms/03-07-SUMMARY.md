---
phase: 03-forms
plan: 07
subsystem: forms
tags: [vb6, frx, resource-blob, image-format, offset-cursor]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-06's PayloadType::Picture path (already reported as an honest Undecoded gap, per 03-06's own Known Stubs); plan 03-03's read_first precedent for the bounded-read and Defect-message shape this plan reuses"
provides:
  - "Blob, extract_blob: the inline resource blob reader, separating the absent sentinel, the deleted-icon shape, and a normal blob, with every bound checked against the real remaining bytes before any subregion is taken"
  - "BlobCursor, FRX_ITEM_HEADER_LEN: the one place this repository computes a .frx offset, a running cursor that resets to 0 per form"
  - "ImageFormat, sniff_format: the seven named signatures STRUCTURES.md section 8.8 lists, plus an honest Unknown carrying the bytes it saw"
affects: [04-04-frm-and-frx-writer]

actuals:
  tokens: 6706
  tasks: 2
  commits: 2
plan_head_before: 0ca7ac5dc7f00dc3e6d0adf00d62c5df3b78018f
commits: 2

tech-stack:
  added: []
  patterns:
    - "ends_within(start, width, block_end): the same shared bound-check shape propstream.rs's own ends_within already established, reused here for the four byte length field, the declared length against the remaining bytes, and every payload width, so a value that would end past the block's own end is refused the same way every time."
    - "declared_len is carried on Blob itself, not recomputed from image.len() plus 8, so BlobCursor::take reads one field and never repeats the arithmetic extract_blob already did."
    - "damaged(String) -> Refusal: the same local Box::leak escape hatch vb/gui.rs and vb/controltree.rs already establish, for the one refusal in this file that must name a runtime value (the running offset on overflow)."

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/frx.rs
    - crates/deform6/src/error.rs

key-decisions:
  - "AGENTS.md's 'put the code and its tests in the same commit' rule takes precedence over the plan's implicit TDD RED-then-GREEN commit split, for both tasks (tdd=true), matching every prior plan in this phase (03-01 through 03-06). Each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task."
  - "The RED-phase evidence for Task 1's checked subtraction diverges from the plan's own illustrative wording. The plan's acceptance criteria say breaking the checked subtraction into a plain one and re-running the length-3 test would show 'the wrapped value from the failure message'. This workspace's dev/test profile leaves Rust's default overflow-checks on (Cargo.toml sets overflow-checks only under [profile.release]; the dev profile inherits the Cargo default of on), so `3u32 - 8u32` panics with 'attempt to subtract with overflow' rather than silently wrapping to a very large number. The real evidence (a hard panic, not a wrapped value reaching the assertion) is recorded below instead of the plan's assumed wrapped-value message, and it is the stronger proof: a plain subtraction here would have failed the test by crashing the whole process, not merely by returning a wrong number."
  - "extract_blob's first test fixture for a declared length of 3 needed three padding bytes after the length field. Without them, the block's own remaining bytes (0) is smaller than 3, and the test would exercise the 'declared length exceeds the remaining bytes' refusal instead of the 'too small to hold its own header' refusal the acceptance criteria specifically ask for. This is a fixture-construction detail, not a deviation from the plan's own instructions: both refusals give a Defect, so the wrong fixture would still have passed a looser assertion, but it would have proven the wrong one of the two checks Task 1 requires."
  - "DefectKind::BlobLenTooSmall added to error.rs (not in this plan's own files_modified list), because no existing variant names 'a declared length too small to hold its own header'. Recoverable severity, matching this crate's own rule that one unreadable property never loses the control block around it."
  - "A local damaged(String) -> Refusal helper was added to frx.rs, matching the identical helper already established in vb/gui.rs and vb/controltree.rs, because Refusal::Damaged takes &'static str and BlobCursor::take's own overflow refusal must name the running offset, computed at run time."

patterns-established:
  - "declared_len carried on the value itself (Blob), not recomputed by a second reader, is the same discipline VbStr::declared_end and the position/font block readers already established in plans 03-05 and 03-06: whichever function first decides a length is the only place a later caller may read it from."

requirements-completed: [FRM-05]

coverage:
  - id: D1
    description: "extract_blob separates the absent sentinel (0xFFFFFFFF, 4 bytes consumed, no blob, no defect) from a real blob, checks the declared length against the remaining bytes of the block before any subregion is taken, and uses a checked subtraction (never a plain one) to decide whether a declared length can hold its own 8 byte header"
    requirement: "FRM-05"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_length_of_0xffffffff_consumes_four_bytes_gives_no_blob_and_no_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_length_of_eight_gives_a_blob_with_zero_image_bytes_and_consumes_twelve_bytes"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_length_of_108_gives_a_blob_with_100_image_bytes_and_consumes_112_bytes"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_length_of_three_gives_a_defect_naming_the_value_and_the_offset"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_length_of_0xfffffffe_gives_a_defect_and_attempts_no_four_gigabyte_allocation"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_declared_length_larger_than_the_remaining_bytes_of_the_block_gives_a_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_declared_length_whose_end_overflows_a_u32_gives_a_defect_naming_the_offset"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_declared_length_that_lands_exactly_on_the_block_end_is_accepted"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::extract_blob_at_a_nonzero_offset_names_the_absolute_file_offset_in_a_defect"
        status: pass
    human_judgment: false
  - id: D2
    description: "BlobCursor::take gives the current running offset then advances by a blob's declared length plus the 12 byte item header, resets to 0 per fresh form, and refuses on u32 overflow naming the running offset; sniff_format names all six first-byte signatures plus the EMF signature at offset 40, and gives an honest Unknown carrying the bytes it saw for anything else, reading no byte past the end of a short blob"
    requirement: "FRM-05"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::frx_item_header_len_is_twelve"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_fresh_cursor_starts_at_zero"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::three_blobs_of_length_8_108_and_8_give_offsets_0_20_and_140"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_new_form_starts_its_cursor_at_zero_after_a_previous_form_advanced_it"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::take_gives_a_refusal_naming_the_offset_when_the_advance_overflows_a_u32"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::sniff_format_recognises_each_of_the_six_first_byte_signatures"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::sniff_format_finds_the_emf_signature_at_offset_forty"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::sniff_format_on_an_unrecognised_prefix_gives_unknown_and_carries_the_bytes"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::sniff_format_on_a_blob_of_zero_image_bytes_gives_unknown_with_no_panic"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::sniff_format_on_a_blob_shorter_than_the_signature_it_needs_reads_no_byte_past_the_end"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/frx.rs#tests::a_zero_length_blob_still_advances_the_cursor_by_the_item_header_alone"
        status: pass
    human_judgment: false

duration: 14min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 7: The Inline Resource Blob and the .frx Offset Cursor Summary

**extract_blob reads the inline bulk-property blob (absent, deleted-icon-empty, or a real image) with every bound checked before any subregion is taken, and BlobCursor is the one place this repository computes the running .frx offset a writer will need.**

## Performance

- **Duration:** 14 min (approximate; measured from the previous plan's completion commit to this plan's final code commit)
- **Started:** 2026-09-10T15:22:12+02:00 (approximate, previous plan's completion)
- **Completed:** 2026-09-10T15:36:17+02:00
- **Tasks:** 2
- **Files modified:** 2 (0 created, 2 modified)

## Accomplishments

- `extract_blob` reads the four byte little endian `blobLen`, separating three states: `0xFFFFFFFF` (absent, 4 bytes consumed, no blob, no defect), a declared length of 8 (the deleted form icon shape, a blob with zero image bytes), and a normal blob (the eight byte inline picture header plus `blobLen - 8` image bytes). Every bound is checked against the real remaining bytes of the block before any subregion is taken and before any allocation is sized: a crafted `0xFFFFFFFE` never reaches a `take()` call at all.
- The image byte count is a checked subtraction, `blob_len.checked_sub(8)`, which is itself the refusal mechanism for a declared length below 8: a plain subtraction would wrap (or, as this workspace's dev profile actually proved, panic under overflow-checks) rather than refuse cleanly.
- `BlobCursor` is the one place this repository computes a `.frx` offset. `take` gives the current running offset, then advances by the blob's own declared length plus `FRX_ITEM_HEADER_LEN` (12, the item header a writer adds that the executable does not hold). A fresh `BlobCursor::new()` per form starts at 0; three blobs of declared length 8, 108 and 8 give offsets 0, 20 and 140, proving the plus-12 applies once per blob and not once per form.
- `sniff_format` names a blob's own container format from its first bytes: BMP, GIF, JPEG, WMF, ICO and CUR from a first-byte signature, EMF from the four bytes at offset 40. An unrecognised prefix, or a blob too short for the signature it would need, gives `ImageFormat::Unknown` carrying whatever bytes were actually there, reading no byte past the end of the blob. DeForm6 never decodes the image; it classifies the container and copies the blob whole, so an unknown format costs nothing.
- This plan writes no file, matching `03-RESEARCH.md`'s own "Phase boundary note" and the plan's own flagged assumption A5: the blob bytes and the offset cursor are recovered into memory here, and phase 4's plan 04-04 owns the `.frx` writer that actually emits them, because the `.frm` writer and the `.frx` writer are one component sharing this same cursor.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: The inline blob and its bounds** - `9b649b8` (feat, tdd="true")
2. **Task 2: The running offset cursor and the image format sniff** - `f33f614` (feat, tdd="true")

**Plan metadata:** commit follows this SUMMARY.

_Note: both tasks carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase), each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task._

## Files Created/Modified

- `crates/deform6/src/vb/frx.rs` - `Blob`, `extract_blob`, `ends_within`, `site` (Task 1); `FRX_ITEM_HEADER_LEN`, `BlobCursor`, `damaged`, `ImageFormat`, `signature_at`, `sniff_format` (Task 2); 20 unit tests total
- `crates/deform6/src/error.rs` - adds `DefectKind::BlobLenTooSmall`, the variant for a declared length too small to hold its own 8 byte picture header

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The most consequential: the plan's own illustrative RED-phase wording ("records the wrapped value") does not hold in this workspace, because the dev/test profile leaves Rust's default overflow-checks on; the real evidence is a hard panic, which is the stronger proof of the checked subtraction's necessity, and is recorded honestly below rather than substituted with the plan's assumed wording.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `error.rs` needed a new `DefectKind` variant**
- **Found during:** Task 1, writing the "declared length too small to hold its own header" refusal
- **Issue:** No existing `DefectKind` variant names a value that is too small for a fixed structure it must contain (checked the whole file, per this phase's own established practice for the identical situation in plans 03-04 and 03-05).
- **Fix:** Added `DefectKind::BlobLenTooSmall { offset, blob_len }`, `Recoverable` severity, following the file's own exhaustive per-variant severity match.
- **Files modified:** `crates/deform6/src/error.rs`
- **Verification:** `a_length_of_three_gives_a_defect_naming_the_value_and_the_offset`
- **Committed in:** `9b649b8` (Task 1 commit)

**2. [Rule 3 - Blocking] `Refusal::Damaged` cannot literally carry the running offset**
- **Found during:** Task 2, writing `BlobCursor::take`'s own overflow refusal
- **Issue:** `Refusal::Damaged` takes `&'static str`, and the plan's own acceptance criteria require a refusal that names the running offset, a value computed at run time.
- **Fix:** Added a local `damaged(message: String) -> Refusal` helper in `frx.rs`, the identical `Box::leak` escape hatch `vb/gui.rs` and `vb/controltree.rs` already establish for the same reason.
- **Files modified:** `crates/deform6/src/vb/frx.rs`
- **Verification:** `take_gives_a_refusal_naming_the_offset_when_the_advance_overflows_a_u32`
- **Committed in:** `f33f614` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3, blocking, both matching identical precedent already established elsewhere in this phase).
**Impact on plan:** Both were necessary for the plan's own acceptance criteria to compile and hold. No scope creep.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's checked subtraction** — changed `blob_len.checked_sub(PICTURE_HEADER_LEN)` to a plain `blob_len - PICTURE_HEADER_LEN`, and ran `cargo test -p deform6 --lib vb::frx::tests::a_length_of_three_gives_a_defect_naming_the_value_and_the_offset` once:

```
thread 'vb::frx::tests::a_length_of_three_gives_a_defect_naming_the_value_and_the_offset' panicked at crates/deform6/src/vb/frx.rs:151:21:
attempt to subtract with overflow
```

The plan's own acceptance criterion text expected this break to make the test fail by producing "the wrapped value" in an assertion message. It did not: this workspace's `[profile.release]` in `Cargo.toml` sets `overflow-checks = true` explicitly, and the default `dev`/test profile already carries `overflow-checks = true` from Cargo's own default (tied to `debug-assertions`), so a `u32` subtraction that would underflow panics immediately rather than wrapping silently to `4294967291`. The real failure — an unconditional panic, not a passed-but-wrong assertion — is the evidence this repository's own gate would have caught the bug even without this specific test's own assertion running to completion. Reverted to the checked subtraction before committing `9b649b8`.

**Task 2's cursor advance** — changed `BlobCursor::take` to advance by `blob.declared_len` alone, dropping the `+ FRX_ITEM_HEADER_LEN` term entirely, then ran `cargo test -p deform6 --lib vb::frx::tests::three_blobs_of_length_8_108_and_8_give_offsets_0_20_and_140` once:

```
assertion `left == right` failed: the plus 12 must apply once per blob, not once per form
  left: [0, 8, 116]
 right: [0, 20, 140]
```

Three blobs of declared length 8, 108 and 8 gave offsets `[0, 8, 116]` instead of `[0, 20, 140]`: each offset was short by exactly `12 * n` for the `n`-th blob, proving the item header was never added. Reverted to the full `checked_add(blob.declared_len).and_then(|v| v.checked_add(FRX_ITEM_HEADER_LEN))` chain before committing `f33f614`.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every symbol this plan's own artifact table names (`Blob`, `BlobCursor`, `ImageFormat`, `extract_blob`, `sniff_format`, `FRX_ITEM_HEADER_LEN`) is implemented and tested, not a placeholder. `extract_blob` is not yet wired into `propstream.rs`'s own `PayloadType::Picture` branch, which plan 03-06's SUMMARY already recorded as an honest, tracked gap owned by this plan's own module boundary — wiring the two together is a differential-gate or phase-4 concern, not a stub this plan silently leaves behind.

## Threat Flags

None. Every trust boundary this plan's own `<threat_model>` names (the four byte blob length sizing a copy, the image byte count computed as a subtraction, the running offset cursor overflowing a `u32`, an image signature read past the end of a short blob, an unrecognised container format guessed as a known one) is mitigated exactly as the threat register states: T-03-36 by the remaining-bytes check running before any subregion is taken; T-03-37 by the checked subtraction refusing rather than wrapping; T-03-38 by `checked_add` on a bare `u32` in `BlobCursor::take`, refusing on overflow; T-03-39 by every signature read going through `Region::take`, which gives `None` rather than reading past the end; T-03-40 by an unrecognised signature carrying its own bytes raw rather than a guessed name. T-03-41 (writing a resource file to disk) is out of this plan's scope by design: this plan writes no file. This plan introduces no new trust boundary beyond those the plan's own threat model already names.

## Next Phase Readiness

- `Blob`, `extract_blob`, `BlobCursor` and `sniff_format` are ready for phase 4's plan 04-04, the `.frx` writer, which needs exactly these: the recovered bytes and the one cursor that computes the offset the generated `.frm` must agree with.
- `propstream.rs`'s own `PayloadType::Picture` branch still reports `Undecoded` rather than calling `extract_blob`; wiring the two together, and giving `walk_properties` a way to continue past a resolved `Picture` property rather than stopping the loop, is deferred to whichever later plan (04-04, or an earlier differential-gate plan) needs the wired path. This is the same honest gap plan 03-06's own SUMMARY already tracked under Known Stubs, not a new one this plan introduces.
- No blockers for plan 03-08 or plan 03-09 (both already executed, ahead of this plan in this phase's wave ordering).

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

Both modified files (`crates/deform6/src/vb/frx.rs`, `crates/deform6/src/error.rs`)
found on disk. Both task commits (`9b649b8`, `f33f614`) found in `git log`.
`cargo test --workspace` passes, `cargo fmt --all --check` and `cargo
clippy --all-targets -- -D warnings` both pass clean, and `cargo test -p
deform6 --lib vb::frx` gives 20 tests, meeting Task 2's own stated minimum
of 20 and exceeding Task 1's own stated minimum of 8. `git status --short`
and `git diff --stat` confirm only `crates/deform6/src/vb/frx.rs` and
`crates/deform6/src/error.rs` changed; no file under `corpus/` and no
binary file of any kind entered the repository in this plan.
