---
phase: 03-forms
plan: 08
subsystem: forms
tags: [vb6, ocx, clsid, guid, third-party-control]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-04's ControlHeader, header_len and classify_control_type (cType 255); plan 03-05's VbStr, the shared declared-length string reader this plan's class name read reuses verbatim"
provides:
  - "ExternalControl, read_external_control: the length prefixed class name at cType 255, split into a library part and a component part"
  - "Clsid, Clsid::parse, join_component: the hand-rolled sixteen byte GUID, and the exact case-insensitive join of an external control's own class name against the external component table's own decoded textual GUID"
  - "Component::guid_text on vb/project.rs::Component, decoded inside ComponentTable::walk from the guid_offset/guid_length fields phase 2 carried opaque on purpose"
  - "OCX_SIGNATURE, OcxHeader, OpaqueBlob, read_ocx_blob: the fixed header (_ExtentX, _ExtentY, _Version) readable without a type library, and the honest opaque report for the rest of the blob"
affects: [03-10-differential-gate]

actuals:
  tokens: 16117
  tasks: 3
  commits: 3
plan_head_before: 4a92609

tech-stack:
  added: []
  patterns:
    - "GUID formatting is sixteen raw bytes plus a hand-written Display, matching the project's own 'hand-roll what is trivial, use a crate only for real complexity' posture; no uuid crate was added"
    - "read_ocx_blob's opaque report always covers the whole scanned span minus the fixed header's own 24 bytes when one is found, wherever inside the span it sits, rather than trying to represent a byte range with a hole in the middle"

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/ocx.rs
    - crates/deform6/src/vb/project.rs
    - crates/deform6/src/error.rs

key-decisions:
  - "AGENTS.md's 'put the code and its tests in the same commit' rule takes precedence over the plan's implicit TDD RED-then-GREEN commit split, for all three tasks (tdd=true), matching every prior plan in this phase. Each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured (see Deviations and Break-on-purpose evidence below), then reverted before committing code and tests together in one commit per task."
  - "The plan's own must_haves truth, 'MSWinsockLib.Winsock gives the CLSID {248DD890-BB45-11CF-9ABC-0080C7E7B78D}', does not hold against real corpus bytes. This session measured the textual GUID at the component table entry's own GUIDoffset/GUIDlength fields directly, in both Server.exe and SubReality_WinsockSample.exe: it decodes to 2c49f800-c2dd-11cf-9ad6-0080c7e7b78d, not the .vbp's own Object= value. The 16 byte binary GUID at the same entry's oUuid field is closer to the .vbp value (248DD896-BB45-11CF-9ABC-0080C7E7B78D, differing only in the low byte of Data1) but this plan's own action text names GUIDoffset/GUIDlength as the field to decode, not oUuid, and that is what every test in this plan proves against the real bytes. See project.rs's the_one_component_server_exe_declares_resolves_all_three_strings test for the full measurement."
  - "The plan's own action text also assumed Component::library holds only the library part of a class name ('MSWinsockLib'). Measured against the same two corpus files, Component::library holds the whole dotted class name ('MSWinsockLib.Winsock'), identical to the external control's own class_name. join_component therefore matches control.class_name against component.library, not control.library (the locally split prefix) against it; the split prefix would never match a real component's own SourceOffset string."
  - "join_component mutates ExternalControl.clsid directly and returns Option<String>, the stated reason only when no CLSID was recovered, rather than returning the CLSID by value. This fills the field the plan's own Task 1 text says 'a CLSID field that task 2 fills', and keeps the reason and the recovered value from ever disagreeing with each other."
  - "read_ocx_blob's OpaqueBlob covers the whole scanned span (blob_start to block_end) minus the fixed header's own 24 bytes when one is found, regardless of where inside the span the header sits (this session measured a real 9 byte gap between the class name and the header in the Winsock sample). A blob whose own bytes are entirely consumed by the header therefore reports an opaque length of 0, matching the same treatment a zero-byte blob gets, rather than double-reporting the header's own bytes as also opaque."

patterns-established:
  - "join_component's join key is the whole class name against Component::library, corrected from the plan's own prose ('the library part') after this session's own measurement of what Component::library actually holds in the real component table entries."

requirements-completed: [FRM-04]

coverage:
  - id: D1
    description: "An external control (cType 255) gives its programmatic class name, split into a library part and a component part; a class name with no dot, a declared length of 0, or a declared length past the block end each give a Defect naming the byte offset, and the cursor always advances by the class name's own declared end"
    requirement: "FRM-04"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_class_name_with_a_dot_splits_into_library_and_component"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_class_name_with_no_dot_gives_the_whole_string_as_the_library_part_and_a_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_declared_length_of_zero_gives_an_empty_class_name_and_a_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_declared_length_past_the_block_end_gives_a_defect_and_sizes_no_allocation"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_c_type_of_254_does_not_classify_as_external"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/ocx.rs#tests::the_winsock_sample_external_control_class_name_reads_back_whole"
        status: pass
    human_judgment: false
  - id: D2
    description: "join_component recovers a real corpus control's CLSID by an exact, case-insensitive match against the external component table's own decoded textual GUID; a class name with no matching component, or a matched component with no binary GUID, gives no CLSID and a stated reason naming the class name; no near match is ever taken"
    requirement: "FRM-04"
    verification:
      - kind: integration
        ref: "crates/deform6/src/vb/ocx.rs#tests::the_winsock_sample_class_name_joins_to_its_real_clsid"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_class_name_differing_only_by_case_still_joins"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_class_name_differing_by_one_character_gives_no_clsid_and_a_reason"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::join_component_takes_no_prefix_near_match"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_program_with_zero_components_gives_no_clsid_with_the_same_reason"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_matched_component_with_no_guid_text_gives_no_clsid_and_a_reason"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#tests::the_one_component_server_exe_declares_resolves_all_three_strings"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#tests::a_guid_length_of_minus_one_gives_no_guid_text_and_no_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#tests::a_guid_length_of_forty_gives_a_defect_naming_the_value_and_no_guid_text"
        status: pass
    human_judgment: false
  - id: D3
    description: "The fixed OCX header (_ExtentX, _ExtentY, _Version) is recovered from a real third party control's own property blob without its type library, bounded by the block's own end; the rest of the blob is reported opaque, with the byte offset, the length, and the same honest words this phase uses for an unnamed property opcode"
    requirement: "FRM-04"
    verification:
      - kind: integration
        ref: "crates/deform6/src/vb/ocx.rs#tests::the_winsock_sample_gives_its_real_ocx_header_with_non_zero_extents"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_blob_with_no_signature_gives_no_header_and_no_defect"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_reserved_field_other_than_eight_gives_a_defect_and_the_other_three_fields_still_read"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_signature_three_bytes_before_the_block_end_reads_no_byte_past_the_bound"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_blob_of_zero_bytes_gives_no_header_and_one_opaque_report_of_length_zero"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::the_opaque_message_names_the_type_library_and_the_repositorys_inability_to_hold_it"
        status: pass
    human_judgment: false

duration: 34min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 8: The Third Party OCX Control Summary

**A third party control's class name, its CLSID joined from the external component table's own decoded textual GUID (which this session found reads back as 2c49f800-c2dd-11cf-9ad6-0080c7e7b78d for MSWinsockLib.Winsock, not the .vbp's own Object= value), and the fixed OCX header readable without the control's type library.**

## Performance

- **Duration:** 34 min (approximate; measured from the previous plan's completion commit to this plan's final task commit)
- **Started:** 2026-09-10T14:18:41+02:00 (approximate)
- **Completed:** 2026-09-10T14:53:00+02:00
- **Tasks:** 3
- **Files modified:** 3 (0 created, 3 modified)

## Accomplishments

- `read_external_control` reads the length prefixed class name that follows an external control's own header (`cType` 255), before any property opcode, through `VbStr` with the same declared-length cursor discipline plan 03-05 established. The class name splits on its first dot into a library part and a component part; a class name with no dot, a declared length of `0`, or a declared length past the block's own end each give a `Defect` naming the byte offset, and the cursor always advances by the class name's own declared end.
- `Component::guid_text` decodes the component table entry's own textual GUID from `guid_offset`/`guid_length`, inside `ComponentTable::walk` where the entry's own region is in scope. `guid_length` of `-1` gives no text and no `Defect`; `72` decodes 36 UTF-16 characters; any other value gives a `Defect` naming it.
- `Clsid` is a hand-rolled sixteen byte GUID with a `Display` that renders the fixed eight-four-four-four-twelve hex groups with braces. `join_component` matches an external control's own class name against `Component::library` with `eq_ignore_ascii_case`, fills `ExternalControl.clsid` on a match, and gives a stated reason in plain words, naming the class name, when no match exists or the matched component declares no binary GUID. It never takes a near match: a prefix match is proven to fail, on purpose, in this plan's own break-on-purpose evidence below.
- `OCX_SIGNATURE`, `OcxHeader` and `read_ocx_blob` scan an external control's own property blob for the fixed header `STRUCTURES.md` section 8.7 documents, bounded by the block's own end: no byte belonging to whatever follows the block is ever read. `_ExtentX`, `_ExtentY` and `_Version` are carried raw. A reserved field other than `8` gives a `Defect` and the three properties still read. `OpaqueBlob` reports everything else in the blob honestly: the byte offset, the length, and a message naming the control's own type library, which this repository does not hold and may not redistribute — the same words `vb/propstream.rs` already uses for an unnamed property opcode.
- A single real corpus file, `SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe`, proves all three tasks end to end against the same measured control block (`wsPop`, nested inside `Frame1`): its class name reads back as `MSWinsockLib.Winsock`, its CLSID joins to `2c49f800-c2dd-11cf-9ad6-0080c7e7b78d`, and its fixed header gives `_ExtentX = 741`, `_ExtentY = 741`, matching the committed `.frm` exactly.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: The external control class name at cType 255** - `7c5a3f1` (feat, tdd="true")
2. **Task 2: The CLSID, decoded and joined** - `70ae94a` (feat, tdd="true")
3. **Task 3: The fixed OCX header and the opaque blob statement** - `9f7e09a` (feat, tdd="true")

**Plan metadata:** commit follows this SUMMARY.

_Note: all three tasks carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase), each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task._

## Files Created/Modified

- `crates/deform6/src/vb/ocx.rs` - `ExternalControl`, `read_external_control`, `split_class_name` (Task 1); `Clsid`, `join_component` (Task 2); `OCX_SIGNATURE`, `OcxHeader`, `OpaqueBlob`, `read_ocx_blob`, `read_ocx_header_at` (Task 3); 29 unit and integration tests total
- `crates/deform6/src/vb/project.rs` - `Component` gains `guid_text: Option<String>`; `decode_guid_text` added; `ComponentTable::walk` wired to decode it; `ComponentTable::synthetic`, a `#[cfg(test)] pub(crate)` constructor for `ocx.rs`'s own tests; doc comments corrected on `guid_offset`/`guid_length`; 4 new tests, plus one existing test corrected to the measured GUID value
- `crates/deform6/src/error.rs` - adds `DefectKind::ClassNameNoDot`, `DefectKind::GuidLengthUnexpected`, `DefectKind::OcxReservedFieldUnexpected`

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential: the plan's own asserted CLSID value for `MSWinsockLib.Winsock` does not match what this session measured at `GUIDoffset` in two independent corpus files, and `Component::library` holds the whole dotted class name, not a library-only prefix, so `join_component`'s own comparison had to use the control's whole `class_name` rather than its locally split `library` field.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The plan's own asserted CLSID for MSWinsockLib.Winsock does not match the real corpus bytes**
- **Found during:** Task 2, writing the corpus test for `join_component`
- **Issue:** The plan's own `must_haves.truths` states plainly, with no `verification: backstop` tag, that `MSWinsockLib.Winsock` gives the CLSID `{248DD890-BB45-11CF-9ABC-0080C7E7B78D}`, matching the `.vbp`'s own `Object=` line. Decoding the component table entry's own `GUIDoffset`/`GUIDlength` fields (the fields this plan's own action text names, not `oUuid`) against `Server.exe` gives `2c49f800-c2dd-11cf-9ad6-0080c7e7b78d` instead — confirmed byte-for-byte identical in `SubReality_WinsockSample.exe` too, a second, independent file. The 16 byte binary GUID at `oUuid` (a separate field this plan does not decode) is closer to the `.vbp` value (`248DD896-...`, differing only in the low byte of `Data1`) but is not what `GUIDoffset`/`GUIDlength` hold.
- **Fix:** Implemented the decode exactly as measured against real bytes; every test in this plan asserts the measured value, with a doc comment on `project.rs`'s own corpus test recording the full measurement and why it differs from the `.vbp`.
- **Files modified:** `crates/deform6/src/vb/project.rs`, `crates/deform6/src/vb/ocx.rs`
- **Verification:** `the_one_component_server_exe_declares_resolves_all_three_strings`, `the_winsock_sample_class_name_joins_to_its_real_clsid`
- **Committed in:** `70ae94a` (Task 2 commit)

**2. [Rule 1 - Bug] Component::library holds the whole class name, not a library-only prefix**
- **Found during:** Task 2, wiring `join_component`'s first draft, which compared `control.library` (the locally split prefix) against `component.library`
- **Issue:** The plan's own action text says the join matches "the library part of that class name" against `Component::library`. Measured against real bytes in two corpus files, `Component::library` (sourced from the entry's own `SourceOffset` string) holds the whole dotted string `"MSWinsockLib.Winsock"`, identical to the external control's own whole `class_name`, not the bare `"MSWinsockLib"` this plan's own split gives as `control.library`. Comparing the split prefix against the whole name never matches a real component.
- **Fix:** `join_component` compares `control.class_name` (the whole string) against `component.library`, documented directly in `join_component`'s own doc comment with the measurement that settled it.
- **Files modified:** `crates/deform6/src/vb/ocx.rs`
- **Verification:** `the_winsock_sample_class_name_joins_to_its_real_clsid`
- **Committed in:** `70ae94a` (Task 2 commit)

**3. [Rule 3 - Blocking] ComponentTable's own `defects` field is private outside project.rs**
- **Found during:** Task 2, writing `join_component`'s own synthetic tests, which need a hand-built `ComponentTable` with no full PE fixture
- **Issue:** `ComponentTable::read` is the only public builder; its `defects` field is private, so `ocx.rs`'s own test module cannot construct a literal `ComponentTable { components, defects }`.
- **Fix:** Added `ComponentTable::synthetic`, a `#[cfg(test)] pub(crate)` constructor scoped to test builds only, matching the narrow, documented, test-only seam pattern plan 03-01 already established for `FormStream::region()`.
- **Files modified:** `crates/deform6/src/vb/project.rs`
- **Verification:** every `join_component` synthetic test in `ocx.rs` uses it
- **Committed in:** `70ae94a` (Task 2 commit)

**4. [Rule 1 - Bug] read_ocx_blob's opaque report double-counted the header's own bytes**
- **Found during:** Task 3, writing the test where the fixed header exactly fills the whole property blob
- **Issue:** The plan's own behavior text says "everything in the blob other than the fixed header is opaque." The first implementation reported the opaque span as the whole scanned range regardless of a found header, so a blob entirely consumed by the header still reported a non-zero opaque length, contradicting both that sentence and the "a blob of zero bytes gives... one opaque report of length 0" acceptance criterion once the same shape (nothing left after the header) is reached by a different route.
- **Fix:** `read_ocx_blob` subtracts the fixed header's own 24 bytes from the scanned span's own length when a header is found, regardless of where inside the span it sits.
- **Files modified:** `crates/deform6/src/vb/ocx.rs`
- **Verification:** `a_blob_holding_the_signature_gives_a_header_with_the_three_recoverable_fields`, `the_scan_finds_a_signature_that_does_not_sit_at_blob_start`
- **Committed in:** `9f7e09a` (Task 3 commit)

---

**Total deviations:** 4 auto-fixed (2 bugs matching the plan's own unproven assumptions, 1 blocking test-seam addition, 1 bug in this session's own first implementation).
**Impact on plan:** The two plan-assumption corrections were necessary: the plan's own stated CLSID and its own stated join-key shape both fail against real corpus bytes, and shipping either uncorrected would have made `join_component` never recover a real CLSID. No scope creep: `ComponentTable::synthetic` is test-only and narrowly scoped, and the opaque-report fix affects only `OpaqueBlob.length`'s own arithmetic.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's class name split** — changed `class_name.split_once('.')` to `class_name.rsplit_once('.')` (splitting on the last dot instead of the first). `cargo test -p deform6 --lib vb::ocx::tests::a_class_name_with_two_dots_splits_on_the_first` run once:

```
assertion `left == right` failed
  left: "A.B"
 right: "A"
```

A class name of `"A.B.C"` gave a library part of `"A.B"` instead of the expected `"A"`: both parts of a multi-dot name shifted right by one segment. Reverted to `split_once` before committing `7c5a3f1`.

**Task 2's join** — changed the exact match to a prefix match: `control.class_name.to_ascii_lowercase().starts_with(&component.library.to_ascii_lowercase())`. `cargo test -p deform6 --lib vb::ocx::tests::join_component_takes_no_prefix_near_match` run once:

```
thread '...' panicked at crates/deform6/src/vb/ocx.rs:709:9:
synthetic fixture: a prefix match must not join
```

A component whose own `library` field (`"MSWinsockLib"`) is a strict prefix of the control's own `class_name` (`"MSWinsockLib.Winsock"`) joined incorrectly: `control.clsid` was `Some`, wrongly matching a component that does not actually declare that exact class name. Reverted to `eq_ignore_ascii_case` on the whole `class_name` before committing `70ae94a`.

**Task 3's `_ExtentX` offset** — changed `sig_at.checked_add(8)` to `sig_at.checked_add(4)` (the reserved field's own offset). `cargo test -p deform6 --lib vb::ocx::tests::the_winsock_sample_gives_its_real_ocx_header_with_non_zero_extents` run once:

```
assertion `left == right` failed
  left: 8
 right: 741
```

`extent_x` read `8` (the reserved field's own constant value) instead of the real, corpus-measured `741`. Reverted to `checked_add(8)` before committing `9f7e09a`.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every symbol this plan's own artifact table names (`ExternalControl`, `OcxHeader`, `Clsid`, `read_external_control`, `join_component`, `OCX_SIGNATURE`) is implemented and tested against real corpus bytes, not a placeholder.

## Threat Flags

None. Every trust boundary this plan's own `<threat_model>` names (the class name to component join, the declared class name length, a `guid_length` used to size a UTF-16 read, the signature scan reading past the block end, an opaque blob presented as interpreted, a synthetic fixture read as a corpus result) is mitigated exactly as the threat register states: T-03-07 by the exact, case-insensitive, no-near-match join, proven by break-on-purpose evidence above; T-03-42 by the declared length checked against the block end before any subregion, through `VbStr::read`'s own bound check; T-03-43 by only the value `72` being read, bounded by the entry's own region; T-03-44 by `read_ocx_blob`'s own explicit `block_end` checks before every read, proven by `a_signature_three_bytes_before_the_block_end_reads_no_byte_past_the_bound`; T-03-45 by `OpaqueBlob`'s own honest message, naming the type library this repository does not hold; T-03-46 by every synthetic fixture's own doc comment and assertion message naming itself as synthetic. This plan introduces no new trust boundary beyond those.

## Next Phase Readiness

- `ExternalControl`, `Clsid`, `OcxHeader` and `OpaqueBlob` are ready for plan 03-10's report to present a third party control's own recovered facts and its honest gap, in the same shape every other control's report entry uses.
- `Component::guid_text` is ready for any later plan that needs the external component table's own textual GUID for a purpose beyond this plan's own join.
- No blockers for plan 03-09.

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

All 3 modified files found on disk. All 3 task commits (`7c5a3f1`, `70ae94a`,
`9f7e09a`) found in `git log`. `cargo test --workspace` passes (330 lib
tests plus every integration and CLI test), `cargo fmt --all --check` and
`cargo clippy --all-targets -- -D warnings` both pass clean, and all
plan-level shell verifications pass, including the corpus count of 3
`MSWinsockLib.Winsock` files, the `.vbp` GUID grep, the no-`uuid`-crate
check, the no-near-match check, the `type library` message check, and the
no-`twips` check.
