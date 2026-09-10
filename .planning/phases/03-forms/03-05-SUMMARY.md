---
phase: 03-forms
plan: 05
subsystem: forms
tags: [vb6, string-encoding, cursor-discipline, form-property-stream]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-01's window-before-fields discipline and the vbstr.rs stub; plan 03-04's read_name precedent for the same Latin-1, ImplausibleCount, EmptyName shapes this plan reuses"
provides:
  - "VbStr, StrEncoding, VbStr::read, VbStr::declared_end: the string reader every property reader in phase 3 shares, with the declared-length cursor discipline that must never be relaxed"
  - "the landing check (byte count plus trailing null), the one retry as the other encoding, and DefectKind::UnrecoverableString for the refused case"
affects: [03-06-propstream]

actuals:
  tokens: 6003
  tasks: 2
  commits: 2
plan_head_before: 233a809

tech-stack:
  added: []
  patterns:
    - "the declared end is computed once, from the length field alone, before any text byte is read, and returned unchanged in every successful case and every refused case alike; no method on VbStr gives an alternate advance"
    - "a length field or landing check that cannot be evaluated defaults to a safe, bounded value (declared_len = 0 when unreadable) rather than an unwrap or a panic, matching vb/controltree.rs::read_control_header's own precedent for a count field it cannot read"

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/vbstr.rs
    - crates/deform6/src/error.rs

key-decisions:
  - "AGENTS.md's 'put the code and its tests in the same commit' rule takes precedence over the plan's implicit TDD RED-then-GREEN commit split, for both tasks (tdd=true), matching every prior plan in this phase (03-01, 03-02, 03-03, 03-04). Each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task."
  - "Every commit in this plan landed on main, per this session's explicit sequential-executor instructions and this project's own git.branching_strategy: none, matching every prior plan in this phase's own precedent under this identical configuration."
  - "A length field that does not fit at the given offset is read as a declared length of 0, the same defensive default vb/controltree.rs::read_control_header already uses for a count field it cannot read, rather than a distinct failure path. The declared-end bound check downstream still catches the case where the offset itself leaves no room for anything, so this adds no unproven behaviour, only reuses an existing precedent for a case the plan's own acceptance criteria does not name."
  - "An arithmetic overflow in the declared-end computation returns Off::new(u32::MAX) as the cursor, since no real declared end exists to give. This is a saturating sentinel, not a guess: a caller's own bound check against a region refuses cleanly against it rather than reading past the file."

patterns-established:
  - "The landing check is two independent halves: a byte-count match (only ever real for UTF-16, since an ASCII decode of n declared bytes always consumes exactly n) and a trailing-null check (encoding-independent, checked at the same declared_end - 1 position regardless of which encoding is being tried). A future property reader that reuses this shape should keep both halves, not just the byte-count half, or a record whose trailing byte is corrupted but whose byte count coincidentally matches would silently land."

requirements-completed: [FRM-03]

coverage:
  - id: D1
    description: "VbStr::read computes the declared end from the length field alone, before any text byte is read, and returns it unchanged in every case, including when the declared length overflows a u32 or runs past the enclosing region"
    requirement: "FRM-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::a_declared_length_of_zero_gives_an_empty_string_and_a_declared_end_three_bytes_past_the_start"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::the_same_declared_end_holds_when_the_five_bytes_are_all_0xff"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::an_ascii_read_of_the_byte_0xa9_gives_its_own_latin1_code_point"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::a_declared_length_larger_than_the_region_gives_a_defect_naming_both_numbers_and_still_gives_the_declared_end"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::a_declared_length_whose_end_overflows_a_u32_gives_a_defect_naming_the_offset"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::a_utf16_read_pairs_bytes_little_endian"
        status: pass
    human_judgment: false
  - id: D2
    description: "A decode that does not land is retried once as the other encoding with the same landing check; a decode that lands under neither encoding refuses with a Defect naming both encodings tried, an empty text, and the unchanged declared end"
    requirement: "FRM-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::a_string_that_lands_on_the_first_encoding_gives_no_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::an_odd_declared_length_under_utf16_does_not_land_and_the_retry_as_ascii_does"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::a_record_whose_trailing_byte_is_not_null_is_reported_unrecoverable_and_names_the_offset"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::the_declared_end_is_identical_whether_the_string_lands_or_is_refused"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/vbstr.rs#tests::the_refused_case_gives_an_empty_text_never_a_partial_one"
        status: pass
      - kind: integration
        ref: "cargo test --workspace"
        status: pass
    human_judgment: false

duration: 22min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 5: The String Reader Summary

**VbStr, the encoding-validating string reader every property reader in the phase shares: a declared-length cursor that never depends on the decode, a landing check with one retry as the other encoding, and a named refusal when neither lands.**

## Performance

- **Duration:** 22 min (approximate; measured from the previous plan's completion commit to this plan's final code commit)
- **Started:** 2026-09-10T11:20:46Z (approximate)
- **Completed:** 2026-09-10T11:42:37Z
- **Tasks:** 2
- **Files modified:** 2 (0 created, 2 modified)

## Accomplishments

- `VbStr::read` computes the declared end entirely from the length field at the start offset, before any text byte is read: 2 bytes for the length field, then the declared length, then 1 byte for the trailing null, every step through `Off::checked_add` on a bare `u32`. The cursor it returns is that declared end in every case, success or refusal alike; `VbStr` exposes no method that derives a cursor from the decoded text.
- A declared length that runs past the enclosing region, or whose end overflows a `u32`, gives a `Defect` naming the byte offset before any allocation is sized from the declared length. ASCII decodes each byte to its own Latin-1 code point; UTF-16 pairs the declared bytes little endian.
- The landing check: a decode must consume exactly the declared byte count, and the byte at the declared end minus 1 must be a null byte. A decode that does not land is retried once, as the other encoding, with the same check. A decode that lands under neither encoding refuses: `DefectKind::UnrecoverableString` names the byte offset, the declared length, and both encodings tried, and the text is empty. The reader tries at most two encodings; there is no fall back scan to the next null byte, the heuristic `STRUCTURES.md` section 9.3 shows to be unsound by its own byte accounting.
- All three retry branches (lands first, lands on retry, lands on neither) were each broken on purpose, one at a time, and seen to fail on the matching test before the correct implementation was committed. See Break-on-purpose evidence below.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: The declared length cursor** - `3f8b628` (feat, tdd="true")
2. **Task 2: The landing check, the one retry, and the refusal** - `29c893e` (feat, tdd="true")

**Plan metadata:** commit follows this SUMMARY.

_Note: both tasks carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase), each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task._

## Files Created/Modified

- `crates/deform6/src/vb/vbstr.rs` - `StrEncoding`, `VbStr`, `VbStr::read`, `VbStr::declared_end`, `VbStr::text`, the `decode`, `lands`, `other_encoding` and `encoding_name` helpers, and 16 unit tests
- `crates/deform6/src/error.rs` - adds `DefectKind::UnrecoverableString`, the variant for a string that lands under neither encoding

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The most consequential: the landing check is two independent halves (byte count, and a trailing-null check at a fixed position independent of which encoding is tried), because an ASCII decode always trivially matches the byte-count half — only the trailing-null half can ever refuse an ASCII attempt, and that same check applies identically to the retry.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `error.rs` needed a new `DefectKind` variant**
- **Found during:** Task 2, writing the refusal path for a string that lands under neither encoding
- **Issue:** The plan's own acceptance criteria require a `Defect` naming the byte offset, the declared length, and both encodings tried. No existing `DefectKind` variant carries "both encodings tried" as a field (checked the whole file per this phase's own established practice, following 03-04's identical precedent for `EmptyName` and `IndexHighByteSet`).
- **Fix:** Added `DefectKind::UnrecoverableString { offset, declared_len, first_encoding, second_encoding }`, following the file's own exhaustive per-variant severity match (`Recoverable`, since the block around the string is not lost).
- **Files modified:** `crates/deform6/src/error.rs`
- **Verification:** `a_record_whose_trailing_byte_is_not_null_is_reported_unrecoverable_and_names_the_offset`, `the_unrecoverable_defect_names_both_encodings_tried`
- **Committed in:** `29c893e` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 3, blocking).
**Impact on plan:** Necessary for the plan's own acceptance criteria to compile and hold. No scope creep: the addition is one narrowly scoped, documented `DefectKind` variant, matching this phase's own established pattern for the identical situation in plan 03-04.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's declared-end cursor** — changed the returned cursor from the declared end to `text_start` plus the decoded text's own UTF-8 byte length, then ran `cargo test -p deform6 --lib vb::vbstr::tests::the_same_declared_end_holds_when_the_five_bytes_are_all_0xff` once:

```
assertion `left == right` failed
  left: Off(12)
 right: Off(8)
```

Five declared bytes, all `0xFF`, decode as five Latin-1 characters at code point U+00FF, which each take 2 bytes in UTF-8 — so the broken cursor (`text_start` + 10 UTF-8 bytes = 12) diverges from the correct declared end (8) by exactly the width the content-dependent bug introduces. Reverted before committing `3f8b628`.

**Task 2, branch 1 ("lands first")** — `lands` changed to unconditionally return `false`. `cargo test -p deform6 --lib vb::vbstr::tests::a_string_that_lands_on_the_first_encoding_gives_no_defect` run once:

```
assertion `left == right` failed
  left: ""
 right: "Tag"
```

**Task 2, branch 2 ("lands on retry")** — `other_encoding` changed to return its own argument instead of the other variant. `cargo test -p deform6 --lib vb::vbstr::tests::an_odd_declared_length_under_utf16_does_not_land_and_the_retry_as_ascii_does` run once:

```
assertion `left == right` failed
  left: ""
 right: "Tag"
```

**Task 2, branch 3 ("lands on neither")** — `lands` changed to unconditionally return `true`. `cargo test -p deform6 --lib vb::vbstr::tests::a_record_whose_trailing_byte_is_not_null_is_reported_unrecoverable_and_names_the_offset` run once:

```
assertion `left == right` failed
  left: "Tag"
 right: ""
```

All three breaks reverted to the correct implementation before committing `29c893e`.

## Issues Encountered

None beyond the deviation above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every symbol this plan's own artifact table names (`VbStr`, `StrEncoding`, `VbStr::read`, `VbStr::declared_end`) is implemented and tested, not a placeholder.

## Threat Flags

None. Every trust boundary this plan's own `<threat_model>` names (the cursor advance, a declared string length used to size a read, an overflowing declared end, a partial string after a failed decode, repeated decode attempts) is mitigated exactly as the threat register states: T-03-03 by the single, unconditional `declared_end` return; T-03-27 by the region bound check before any allocation; T-03-28 by `Off::checked_add` at every step; T-03-29 by the empty text on refusal; T-03-30 by the fixed two-attempt cap. This plan introduces no new trust boundary beyond those.

## Next Phase Readiness

- `VbStr::read`, `VbStr::text` and `VbStr::declared_end` are ready for plan 03-06 (`propstream.rs`) to build the real `String`-typed property walk against: every `String` payload width `STRUCTURES.md` section 8.5 gives (`2 + n + 1`) is exactly what `VbStr::read` already accounts for.
- `DefectKind::UnrecoverableString` is ready for `Report`'s own defect list to carry a string property that could not be recovered, the same way every other per-item defect in this crate is carried.
- No blockers for plan 03-06.

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

Both modified files (`crates/deform6/src/vb/vbstr.rs`, `crates/deform6/src/error.rs`)
found on disk. Both task commits (`3f8b628`, `29c893e`) found in `git log`.
`cargo test --workspace` passes (269 lib tests plus every integration and
CLI test, up from 253 before this plan), `cargo fmt --all --check` and
`cargo clippy --all-targets -- -D warnings` both pass clean, and all three
plan-level verification commands pass, including `cargo test -p deform6
--lib vb::vbstr` (16 tests, exceeding both tasks' own minimums of 9 and 16).
