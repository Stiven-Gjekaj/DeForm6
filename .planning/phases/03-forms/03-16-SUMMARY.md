---
phase: 03-forms
plan: 16
subsystem: forms
tags: [vb6, ocx, clsid, guid, third-party-control, gap-closure]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-08's ExternalControl, Clsid, join_component and Component::guid_text; the deviation that first documented this exact discrepancy and left it unresolved"
provides:
  - "Component::o_uuid, Component::ouuid_field_offset, Component::ouuid_text: the component table entry's own oUuid field, decoded and carrying the byte offset it was read from"
  - "join_component reporting oUuid, not GUIDoffset/GUIDlength, as a control's CLSID, with an honest caveat attached to every successful join"
  - "print_clsid printing that caveat beneath the reported value"
  - "STRUCTURES.md section 7.3.1: the eighteen-search measurement record, both candidate fields' decoded values and byte offsets across all three corpus programs"
affects: [03-17-remaining-review-findings]

actuals:
  tokens: 12035
  tasks: 3
  commits: 3
plan_head_before: 6dca43514f96be3e6fcd9aff5f9639cd8515f533

tech-stack:
  added: []
  patterns:
    - "A reported value with no confirmed ground truth carries its own caveat unconditionally, attached at the point of recovery (join_component), not computed per run: the caveat is a property of which field this repository selected, decided once by measurement, not a live comparison against a project file the tool never reads."
    - "The eighteen-search sweep (six encodings, three files) is a one-time research record, kept in STRUCTURES.md and in this SUMMARY, not shipped as runtime code: the runtime has no project file to compare against for an arbitrary future program, so it cannot repeat the search live."

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/project.rs
    - crates/deform6/src/vb/ocx.rs
    - crates/deform6/src/vb/mod.rs
    - crates/deform6-cli/src/main.rs
    - crates/deform6-cli/tests/cli.rs
    - .planning/research/STRUCTURES.md

key-decisions:
  - "Neither candidate field holds the identifier the corpus .vbp files declare. Eighteen searches (six encodings: sixteen byte binary in both field orders, plain text upper/lower case, sixteen bit text upper/lower case) across the whole declared length of the component entry, in all three corpus programs that reference MSWinsockLib.Winsock, never find 248DD890-BB45-11CF-9ABC-0080C7E7B78D anywhere in the entry. A whole-file search (not bounded to the entry) gives the same result."
  - "oUuid is selected over GUIDoffset/GUIDlength as the field this repository reports as a control's CLSID. Its own shape (a fixed sixteen byte binary identifier) matches a CLSID's own shape, and its decoded value, 248DD896-BB45-11CF-9ABC-0080C7E7B78D, differs from the declared identifier by one byte (the low byte of Data1) in all three files, the closer of the two candidates. GUIDoffset/GUIDlength decodes to 2c49f800-c2dd-11cf-9ad6-0080c7e7b78d, sharing no digit pattern with the declared identifier."
  - "A near match is never a match. The one byte difference between the reported value and the declared identifier does not authorize treating the two as equal; Clsid's own derived equality stays exact, and a dedicated test proves a one byte difference is refused."
  - "A successful join always carries a caveat. Since this repository's own research never confirms either candidate field against a project file's own declared identifier, join_component attaches a caveat to every recovered CLSID, naming the byte offset the value was read from and stating plainly that the value is not confirmed. No caveat is hardcoded with a specific third party CLSID: AGENTS.md bars a lookup table, so the caveat names what the value is not in general terms, never a specific alternate GUID a future, different third party control would not share."
  - "REQUIREMENTS.md is left unchanged, and FRM-04 is not added to requirements-completed, matching 03-15-SUMMARY.md's own precedent for FRM-05: this plan raises the FRM-04 wording question rather than answering it, and marking the requirement complete here would repeat the exact overstatement 03-VERIFICATION.md already found in REQUIREMENTS.md's own FRM-04 row."
  - "Fixed twelve pre-existing em-dashes elsewhere in STRUCTURES.md (sections 6.5, 8, 8.2, 8.4, 8.9, 10.1, 10.3), unrelated to section 7.3 but required for this task's own no-em-dash verification, which scans the whole file, to pass. Rule 3 (blocking): the task's own <verify> would otherwise fail on pre-existing content this task did not introduce."
  - "Corrected a documentation error at STRUCTURES.md section 7.3's GUIDlength row and project.rs's own doc comment, both of which had said '-1 means no binary identifier at oUuid', conflating GUIDlength (which governs GUIDoffset alone) with the separate oUuid field. Rule 1 (bug): a doc comment stating a false relationship between two fields this same plan decodes."

patterns-established:
  - "A field selected by measurement, not by the plan's own prior assumption, documents the measurement that selected it directly in the field's own doc comment (Component::ouuid_text, Component::guid_text), so a later reader does not have to reconstruct the reasoning from a SUMMARY or a commit message."

requirements-completed: []
# FRM-04 requires the CLSID to be one "a registry lookup would recognise."
# This plan measures that neither field this repository can decode meets
# that bar, for the one control the corpus can test. Marking FRM-04 [x]
# here would repeat exactly the requirements-tracking overstatement
# 03-VERIFICATION.md found and named as a problem in its own right,
# independent of the code gap it also found. The open wording question is
# raised below, for the human, per this plan's own explicit scope boundary:
# it is not this plan's decision to narrow or complete FRM-04's own text.

coverage:
  - id: D1
    description: "The component table entry's oUuid field is decoded (Component::o_uuid, Component::ouuid_field_offset, Component::ouuid_text), and selected as the field join_component reports as a control's CLSID, after an eighteen-search measurement across all three corpus MSWinsockLib.Winsock programs found neither this field nor GUIDoffset/GUIDlength holds the identifier the matching .vbp declares"
    requirement: "FRM-04"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#tests::the_one_component_server_exe_declares_resolves_all_three_strings"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#tests::an_o_uuid_offset_leaving_fewer_than_sixteen_bytes_gives_a_defect_and_no_ouuid_text"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/ocx.rs#tests::the_winsock_sample_class_name_joins_to_its_measured_ouuid_clsid"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/ocx.rs#tests::the_tftp_server_sample_joins_to_its_measured_ouuid_clsid"
        status: pass
      - kind: integration
        ref: "crates/deform6/src/vb/ocx.rs#tests::the_tftp_client_sample_joins_to_its_measured_ouuid_clsid"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/ocx.rs#tests::a_clsid_one_byte_from_another_is_not_equal_to_it"
        status: pass
    human_judgment: false
  - id: D2
    description: "join_component attaches an honest caveat to every successful CLSID join, naming the byte offset the value was read from and stating it is not confirmed against a project file's own declared identifier; print_clsid prints the caveat beneath the reported value; a control with no matching component still prints the unjoined reason unchanged"
    requirement: "FRM-04"
    verification:
      - kind: automated_ui
        ref: "crates/deform6-cli/tests/cli.rs#winsock_sample_prints_the_clsid_and_the_opaque_blob_statement"
        status: pass
      - kind: automated_ui
        ref: "crates/deform6-cli/tests/cli.rs#winsock_sample_prints_the_caveat_beside_the_joined_clsid"
        status: pass
      - kind: automated_ui
        ref: "crates/deform6-cli/tests/cli.rs#the_real_joined_winsock_sample_never_prints_the_unjoined_reason_wording"
        status: pass
    human_judgment: false
  - id: D3
    description: "STRUCTURES.md section 7.3.1 records the eighteen-search measurement, both candidate fields' decoded values and byte offsets across all three corpus programs, and raises the FRM-04 wording question for the human without answering it; the gap register's own row 17 is cross-referenced, not closed"
    verification:
      - kind: other
        ref: "sed -n '/7.3/,/7.4/p' .planning/research/STRUCTURES.md | grep -c '248DD890' (gives 2)"
        status: pass
      - kind: other
        ref: "sed -n '/7.3/,/7.4/p' .planning/research/STRUCTURES.md | grep -c 'oUuid' (gives 8)"
        status: pass
      - kind: other
        ref: "grep -c 'cannot interpret that' .planning/REQUIREMENTS.md (gives 1, unchanged)"
        status: pass
      - kind: other
        ref: "! grep -q em-dash .planning/research/STRUCTURES.md"
        status: pass
    human_judgment: false

duration: 55min
completed: 2026-09-11
status: complete
---

# Phase 3 Plan 16: The OCX CLSID Field, Measured and Reported Honestly Summary

**Eighteen searches across three corpus programs never find the identifier a project file declares anywhere in the external component table entry; `oUuid` (`248DD896-BB45-11CF-9ABC-0080C7E7B78D`), the closer of the two candidate fields by one byte, is now what `deform6 inspect` reports, with an unconditional, honest caveat printed beside it.**

## Performance

- **Duration:** 55 min (measured from the previous plan's completion commit to this plan's final code commit)
- **Started:** 2026-09-11 (session start)
- **Completed:** 2026-09-11
- **Tasks:** 3
- **Files modified:** 6 (0 created, 6 modified)

## Accomplishments

- `Component::o_uuid`, `Component::ouuid_field_offset` and `Component::ouuid_text` decode the component table entry's own `oUuid` field (entry offset `0x04`, pointing to a sixteen byte binary GUID), bounded by the entry's own region, with a `Defect` naming the byte offset when fewer than sixteen bytes remain.
- `join_component` now reads `ouuid_text`, not `guid_text` (`GUIDoffset`/`GUIDlength`), as the CLSID source, and every successful join carries an honest caveat: the reported value, the byte offset it came from, and a plain statement that this repository's own research never confirmed it against a project file's own declared identifier.
- `print_clsid` prints that caveat on the line beneath the reported CLSID. A control whose class name joins no component still prints the stated reason plan 03-08 already gives, unchanged.
- `STRUCTURES.md` section 7.3.1 records the eighteen-search measurement: six encodings (binary in both field orders, plain text and sixteen bit text in both cases), across the whole declared length of the component entry, in each of the three corpus programs that reference `MSWinsockLib.Winsock`. None of the eighteen searches, nor a whole-file search, finds the declared identifier anywhere.
- A live `deform6 inspect` run against all three corpus programs (`SubReality_WinsockSample.exe`, `Server.exe`, `TFTPClient.exe`) reports the same value, `{248DD896-BB45-11CF-9ABC-0080C7E7B78D}`, at each program's own measured `oUuid` byte offset (`0x1f18`, `0x1738`, `0x2180` respectively), with the caveat printed beneath it in every case.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: Search the whole entry and select the field by measurement** - `1b4358e` (feat, tdd="true")
2. **Task 2: Carry the honest statement into the report and print it** - `094c197` (feat, tdd="true")
3. **Task 3: Write the component table finding into the research register** - `f35a14d` (docs)

**Plan metadata:** commit follows this SUMMARY.

_Note: tasks 1 and 2 carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase), each task's RED phase was run once against a deliberately reverted implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task. Task 3 is documentation only, per its own action text ("Change no source file in this task"), and carries no RED phase._

## Files Created/Modified

- `crates/deform6/src/vb/project.rs` - `Component` gains `o_uuid`, `ouuid_field_offset`, `ouuid_text`; `decode_ouuid_text` added; `ComponentTable::walk` wired to decode it; doc comments corrected and expanded with the full measurement; 2 new tests, 1 existing test extended, 1 existing test's Component literal updated
- `crates/deform6/src/vb/ocx.rs` - `join_component` reads `ouuid_text` and always attaches a caveat on a successful join; doc comments rewritten with the measurement and selection rationale; 6 existing test literals updated, 3 tests renamed and rewritten (one per corpus program), 1 new `Clsid` near-match test
- `crates/deform6/src/vb/mod.rs` - `ControlReport::external_reason`'s own doc comment updated for its new dual role (unjoined reason, or a joined CLSID's own caveat)
- `crates/deform6-cli/src/main.rs` - `print_clsid` prints the caveat beneath a joined CLSID; `print_external`'s doc comment updated
- `crates/deform6-cli/tests/cli.rs` - existing winsock CLSID test updated to the new value and byte offset; 2 new tests (the caveat's own presence, the unjoined path's own wording left unchanged)
- `.planning/research/STRUCTURES.md` - new section 7.3.1 with the full eighteen-search record and the field-value table; gap register row 17 cross-referenced; a documentation error at the `GUIDlength` row corrected; twelve pre-existing em-dashes removed elsewhere in the file

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential: neither candidate field in the component table entry holds the identifier the corpus `.vbp` files declare (measured directly, eighteen searches, three files), and `oUuid` is selected over `GUIDoffset`/`GUIDlength` because its own shape and its one-byte-off value are the closer of two unconfirmed candidates, reported with an unconditional caveat rather than presented as a confirmed match.

## The Eighteen Searches (Task 1's own acceptance criterion)

**Ground truth.** All three `.vbp` files declare the same line:
`Object={248DD890-BB45-11CF-9ABC-0080C7E7B78D}#1.0#0; MSWINSCK.OCX`. The
version part is `#1.0#0`, not `#1.1#0` as `03-VERIFICATION.md` quotes; this
session verified all three `.vbp` files directly and the plan's own
correction holds.

**The three executables**, verified directly this session (one directory
deeper than `03-VERIFICATION.md` and `03-08-SUMMARY.md` state):

    corpus/public-domain/SK-TFTP-Sample__VB6/Client/demo/TFTPClient.exe   (entry at file offset 0x2148, StructLength 0x170)
    corpus/public-domain/SK-TFTP-Sample__VB6/Server/demo/Server.exe      (entry at file offset 0x1700, StructLength 0x170)
    corpus/public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe (entry at file offset 0x1ee0, StructLength 0x170)

**The six forms searched**, for the declared identifier
`248DD890-BB45-11CF-9ABC-0080C7E7B78D`, across the whole 368 byte declared
entry span of each of the three executables above (eighteen searches total),
and also, as a superset check, across each whole file:

| # | Form | TFTPClient.exe | Server.exe | SubReality_WinsockSample.exe |
|---|---|---|---|---|
| 1 | 16 byte binary, standard MS layout (`90d88d2445bbcf119abc0080c7e7b78d`) | not found | not found | not found |
| 2 | 16 byte binary, first field reversed (`248dd890bb4511cf9abc0080c7e7b78d`) | not found | not found | not found |
| 3 | plain text, upper case | not found | not found | not found |
| 4 | plain text, lower case | not found | not found | not found |
| 5 | UTF-16LE text, upper case | not found | not found | not found |
| 6 | UTF-16LE text, lower case | not found | not found | not found |

**Zero of eighteen searches found the declared identifier.** A whole-file
search (not bounded to the entry) for all six forms in all three files gave
the same result: not found, everywhere, every form.

**Every identifier-shaped field, decoded, with its entry offset:**

| Field | Entry offset | Decodes to (all three programs, identically) | Matches declared? |
|---|---|---|---|
| `oUuid` | `0x04` (an offset field; the 16 raw bytes sit at entry offset `0x38` in all three) | `248DD896-BB45-11CF-9ABC-0080C7E7B78D` | No — one byte off (`Data1` low byte `0x96` vs declared `0x90`) |
| `GUIDoffset`/`GUIDlength` | `0x1C`/`0x20` (the text sits at entry offset `0xF8` in all three) | `2c49f800-c2dd-11cf-9ad6-0080c7e7b78d` | No — shares no digit pattern with the declared identifier |

**The selection, in one sentence:** `oUuid` is read because it is the one
candidate whose own shape (a fixed sixteen byte binary identifier) matches a
CLSID's own shape and whose decoded value is the closer of the two to the
declared identifier, and it is reported with an unconditional caveat because
neither candidate is confirmed.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1's field selection** — `join_component`'s own match arm changed from
`component.ouuid_text` back to `component.guid_text` (the field this session
did not select). `cargo test -p deform6 --lib -- vb::ocx::tests::the_winsock_sample_class_name_joins_to_its_measured_ouuid_clsid vb::ocx::tests::the_tftp_server_sample_joins_to_its_measured_ouuid_clsid vb::ocx::tests::the_tftp_client_sample_joins_to_its_measured_ouuid_clsid`
run once, against all three named corpus files:

```
thread 'vb::ocx::tests::the_winsock_sample_class_name_joins_to_its_measured_ouuid_clsid' panicked at crates/deform6/src/vb/ocx.rs:751:9:
assertion `left == right` failed
  left: "{2C49F800-C2DD-11CF-9AD6-0080C7E7B78D}"
 right: "{248DD896-BB45-11CF-9ABC-0080C7E7B78D}"

thread 'vb::ocx::tests::the_tftp_client_sample_joins_to_its_measured_ouuid_clsid' panicked at crates/deform6/src/vb/ocx.rs:793:9:
assertion `left == right` failed
  left: "{2C49F800-C2DD-11CF-9AD6-0080C7E7B78D}"
 right: "{248DD896-BB45-11CF-9ABC-0080C7E7B78D}"

thread 'vb::ocx::tests::the_tftp_server_sample_joins_to_its_measured_ouuid_clsid' panicked at crates/deform6/src/vb/ocx.rs:775:9:
assertion `left == right` failed
  left: "{2C49F800-C2DD-11CF-9AD6-0080C7E7B78D}"
 right: "{248DD896-BB45-11CF-9ABC-0080C7E7B78D}"

test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 407 filtered out; finished in 0.00s
```

Both values from the failure output are recorded: `left` (`2C49F800-...`,
what the unselected `GUIDoffset`/`GUIDlength` field gives) and `right`
(`248DD896-...`, what the fixed test expects from `oUuid`). Reverted to
`component.ouuid_text` before committing `1b4358e`.

**Task 2's caveat** — `join_component`'s successful-join arm changed to
return `None` instead of the caveat. `cargo test -p deform6-cli -- winsock_sample_prints_the_caveat_beside_the_joined_clsid`
run once:

```
thread 'winsock_sample_prints_the_caveat_beside_the_joined_clsid' panicked at crates/deform6-cli/tests/cli.rs:719:5:
the caveat line did not repeat the reported CLSID: "          extents = 741 x 741 (HiMetric), version 393216"
```

With the caveat removed, the line printed immediately after `CLSID = ...`
became the extents line instead of the caveat, and the test failed on the
missing caveat text. Reverted to the caveat-attaching arm before committing
`094c197`.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every symbol this plan's own artifact table names (`Component::o_uuid`, `Component::ouuid_field_offset`, `Component::ouuid_text`, `decode_ouuid_text`, `join_component`'s new caveat, `print_clsid`'s new print arm) is implemented, wired into the production path, and tested against all three real corpus programs, not a placeholder.

## Threat Flags

None. Every trust boundary this plan's own `<threat_model>` names is mitigated as the threat register states: T-03-84 (a near match presented as a match) by `Clsid`'s exact derived equality plus the new `a_clsid_one_byte_from_another_is_not_equal_to_it` test; T-03-85 (a reported identifier with no recorded provenance) by `ouuid_field_offset` carried into the printed caveat, and by STRUCTURES.md section 7.3.1 recording every field, offset and decoded value across all three programs; T-03-86 (a length field sizing a read) by `decode_ouuid_text`'s own bound check through `Region::take`, proven by `an_o_uuid_offset_leaving_fewer_than_sixteen_bytes_gives_a_defect_and_no_ouuid_text`; T-03-87 (a caveat the measurement does not support) by the caveat being unconditional on a fixed, measured field selection rather than a live per-file guess; T-03-88 (one control generalised to every third party control) is unchanged from this plan's own threat register, which already names it `accept` with the sample size recorded in STRUCTURES.md section 7.3.1.

## The FRM-04 Wording Question (raised, not answered, per this plan's own scope)

FRM-04's own requirement text promises the CLSID of each third-party OCX
control. This plan's own measurement, across the one control this corpus
can test (`MSWinsockLib.Winsock`, three independent programs), found no
field of the external component table entry that holds a value a registry
lookup or the control's own project file would recognise. The closer of the
two candidates is now reported, honestly caveated, but it is not a confirmed
CLSID. If this measurement generalises to third-party controls this corpus
cannot test, FRM-04's own wording may promise more than the on-disk format
can deliver, the same shape of narrowing FRM-06 already received for event
names (`03-11-SUMMARY.md`). This plan does not amend `REQUIREMENTS.md` and
does not decide this question: it is raised here, with the measurement
behind it, for the human.

## Next Phase Readiness

- `Component::ouuid_text`, `join_component`'s caveat, and `print_clsid`'s new print arm are ready for plan 03-17's own remaining code review findings; nothing in this plan's own scope blocks it.
- The FRM-04 wording question above is open and unanswered; a future plan or a human decision may narrow FRM-04's own text, matching FRM-06's own precedent, but this plan does not perform that narrowing itself.
- No blockers for plan 03-17.

---
*Phase: 03-forms*
*Completed: 2026-09-11*

## Self-Check: PASSED

All 6 modified files (`crates/deform6/src/vb/project.rs`, `crates/deform6/src/vb/ocx.rs`, `crates/deform6/src/vb/mod.rs`, `crates/deform6-cli/src/main.rs`, `crates/deform6-cli/tests/cli.rs`, `.planning/research/STRUCTURES.md`) found on disk. All 3 task commits (`1b4358e`, `094c197`, `f35a14d`) found in `git log`. `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` all pass clean (grep for `FAILED` or `error[` across the full `cargo test --workspace` run returns nothing). A live `cargo run -p deform6-cli -- inspect` against all three named corpus programs prints `{248DD896-BB45-11CF-9ABC-0080C7E7B78D}` and the caveat, at each program's own correct byte offset (`0x1f18`, `0x1738`, `0x2180`). `sed -n '/7.3/,/7.4/p' .planning/research/STRUCTURES.md | grep -c '248DD890'` gives `2`; the same for `'oUuid'` gives `8`. `grep -c 'cannot interpret that' .planning/REQUIREMENTS.md` gives `1`, unchanged. `! grep -q` for the em-dash byte sequence against the whole of `STRUCTURES.md` passes. `git show --name-only --format= f35a14d` names exactly one file, `.planning/research/STRUCTURES.md`, and no path under `crates/`.
