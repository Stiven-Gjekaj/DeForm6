---
phase: 03-forms
verified: 2026-09-11T00:00:00Z
status: human_needed
score: 7/7 truths verified
covered_files: [".planning/REQUIREMENTS.md", ".planning/ROADMAP.md", ".planning/WINDOWS.md", ".planning/phases/03-forms/03-11-PLAN.md", ".planning/phases/03-forms/03-11-SUMMARY.md", ".planning/phases/03-forms/03-12-PLAN.md", ".planning/phases/03-forms/03-12-SUMMARY.md", ".planning/phases/03-forms/03-13-PLAN.md", ".planning/phases/03-forms/03-13-SUMMARY.md", ".planning/phases/03-forms/03-14-PLAN.md", ".planning/phases/03-forms/03-14-SUMMARY.md", ".planning/phases/03-forms/03-15-PLAN.md", ".planning/phases/03-forms/03-15-SUMMARY.md", ".planning/phases/03-forms/03-16-PLAN.md", ".planning/phases/03-forms/03-16-SUMMARY.md", ".planning/phases/03-forms/03-17-PLAN.md", ".planning/phases/03-forms/03-17-SUMMARY.md", ".planning/phases/03-forms/03-18-PLAN.md", ".planning/phases/03-forms/03-18-SUMMARY.md", ".planning/phases/03-forms/03-CONTEXT.md", ".planning/phases/03-forms/03-RESEARCH.md", ".planning/phases/03-forms/03-REVIEW.md", "crates/deform6-cli/src/main.rs", "crates/deform6-cli/tests/cli.rs", "crates/deform6/src/error.rs", "crates/deform6/src/vb/controlinfo.rs", "crates/deform6/src/vb/controltree.rs", "crates/deform6/src/vb/frx.rs", "crates/deform6/src/vb/gui.rs", "crates/deform6/src/vb/mod.rs", "crates/deform6/src/vb/ocx.rs", "crates/deform6/src/vb/opcodes.rs", "crates/deform6/src/vb/project.rs", "crates/deform6/src/vb/propstream.rs", "crates/deform6/src/vb/vbstr.rs", "crates/deform6/tests/blobs.rs", "crates/deform6/tests/events.rs", "crates/deform6/tests/ratios.rs", "crates/deform6/tests/support/frm.rs", "crates/xtask/src/main.rs", "crates/xtask/src/opcode_table.rs", "tests/ratios.toml"]
covered_digest: "v1:sha256:415f2a3b385776bbaea11b318bed554f216ad3f15d9f181afc524645c41943b9"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/7
  gaps_closed:
    - "The narrowed FRM-06's third fact, the native address of each bound handler: `EventReport::Named` and `EventReport::BoundUnnamed` now carry `handler_address: Option<u32>`, `report_events` fills it from `EventSlot::Bound`'s own decoded handler, `EventReport::Unbound` gains no field, and `print_event` prints the address as eight hex digits or states plainly that it is not decoded. `crates/deform6/tests/events.rs` proves this end to end through `deform6::inspect` only, over two corpus programs, with a hand-decoded independent second read that names neither `decode_stub` nor `StubHandler` on a code line. This session re-ran the test, re-ran a live `inspect`, and re-derived the code trace independently; all three agree."
  gaps_remaining: []
  regressions: []
gaps: []
deferred:
  - truth: "The `.frx` file writer emits blob bytes to disk with offsets the generated `.frm` agrees with (the second half of FRM-05)."
    addressed_in: "Phase 4 (plan 04-04)"
    evidence: "ROADMAP.md Phase 4 named risks: 'The .frm writer and the .frx writer are therefore one component ... Plan 04-04 owns both.' 03-15-SUMMARY.md documents the extraction half as this phase's own scope and the writer as phase 4's."
human_verification:
  - test: "Decide whether FRM-04 ('The tool recovers the CLSID of a third party OCX control...') is satisfied by a mechanically-produced, honestly-caveated value that eighteen searches across three corpus programs and six encodings never found matching the control's own registered identifier anywhere in the executable, or whether FRM-04 requires a confirmed-correct CLSID that this file format may not make recoverable at all."
    expected: "A human reads `STRUCTURES.md` section 7.3.1 (the eighteen-search record) and `03-16-SUMMARY.md`, and either (a) accepts the caveated `oUuid` value as satisfying FRM-04's letter if not its spirit, narrowing the requirement's wording the same way D-02 narrowed FRM-06, or (b) declares FRM-04 unmeetable from the compiled executable alone and defers the question (e.g. to a future phase that can read a `.vbp`/type library, or documents the limit permanently in the README per the Phase 6 goal)."
    why_human: "This is a requirement-wording and product-scope decision, not a code defect: the mechanism SC3 names (join by class name against the external component table) is implemented and executes exactly as described, and the shipped code is honest about the value's uncertainty (an unconditional caveat, confirmed printed beside every reported CLSID in a live run, with no claim of a match anywhere). Whether that honesty satisfies the word 'recovers' in FRM-04 is a policy call, not something grep or a test can settle. Carried forward unchanged from the prior verification; not re-adjudicated in this session."
  - test: "Decide whether FRM-03 ('The tool recovers the property values of every control, and of the form itself') is satisfied by the safe-provenance opcode subset this phase built, given decision D-01 explicitly predicted most `(control type, property name)` pairs would stay open without a lawful, committable property table, or whether the requirement's wording needs the same kind of narrowing D-02 already gave FRM-06."
    expected: "A human reads D-01 in `03-CONTEXT.md` and this session's own corpus-wide measurement (122 named property values against 683 present-and-undecoded records, over 805 property records and five distinct property names, out of 198 `(control type, property name)` pairs the corpus actually sets), and either (a) accepts that FRM-03 is correctly unmet as literally worded and defers full coverage to the human-run type-library campaign D-01 names (outside phase 3, needs a working VB6 install), documenting the limit in the Phase 6 README, or (b) narrows FRM-03's own wording to name the safe-provenance subset explicitly, the same way D-02 narrowed FRM-06."
    why_human: "This is a requirement-wording question, not a code defect: this session's own independent null-delimited sweep over all 44 corpus executables reproduced the SUMMARY's own numbers exactly (122/805, same five names, same per-name breakdown), and the code behaves exactly as D-01 designed it to. Nothing in phase 3 closes this gap without a human at a working VB6 install building the remaining opcode-to-property table entries by the method `STRUCTURES.md` section 13 already used, which D-01 itself places outside phase 3's scope. New in this session; not present in the prior verification."
---

# Phase 3: Forms Verification Report

**Phase Goal (as amended by plan 03-11, citing decision D-02):** `deform6
inspect` reports the control tree of every form with the correct parent for
each control, the type and the name of each control, the property values of
each control and of the form, the CLSID of each third-party OCX control, the
resource blobs, and the event structure of each control: which event slots
are bound, the index of each slot, and the address of each bound handler.

**Verified:** 2026-09-11
**Status:** human_needed
**Re-verification:** Yes — third verification, after gap closure plan 03-18
closed the one code gap the second verification found

## Full Gate (run live, not trusted from SUMMARY)

| Check | Command | Result |
| ----- | ------- | ------ |
| Format | `cargo fmt --all --check` | PASS (no output, exit 0) |
| Lint | `cargo clippy --all-targets -- -D warnings` | PASS (finished clean) |
| Tests | `cargo test --workspace` | PASS — every suite green, 13 binaries, 0 failed anywhere (411, 4, 2, 26, 1, 40, 9, 44, 2, 0, 29, 52, 0 passed) |

The gate is genuinely green, run live in this session. `deform6` grew from
410 to 411 unit-tests (the new `controlinfo.rs` test), `events.rs` is a new
2-test binary, and `deform6-cli`'s `cli.rs` grew from 28 to 29 tests. No
regression anywhere else.

## Adjudication 1: The Handler Address Gap Plan 03-18 Closed

**CLOSED, confirmed independently at every level.**

**Code.** `EventReport::Named` and `EventReport::BoundUnnamed` each carry
`handler_address: Option<u32>` (`controlinfo.rs:750-786`). `EventReport::Unbound`
gains no field. `report_events` binds the handler instead of discarding it
with `..` and maps it to its address (`controlinfo.rs:846-859`, confirmed by
direct read, not by grep alone). `ControlReport.events: Vec<EventReport>`
in `vb/mod.rs` carries the variant unchanged; `compose_control` builds
`events` from `report_events` directly (`mod.rs:765-775`). `print_event`
in `crates/deform6-cli/src/main.rs:794-828` matches the new field and prints
either `handler at {address:#010x}` or `handler address not decoded`, and
leaves the `Unbound` arm untouched.

**Test.** `crates/deform6/tests/events.rs` reaches the address only through
`deform6::inspect`. This session ran it live:

```
$ cargo test -p deform6 --test events
test fast_flames_frm_fire_bound_handler_addresses_match_an_independent_hand_decode ... ok
test mandelbrot_frm_fractal_bound_handler_addresses_match_an_independent_hand_decode ... ok
test result: ok. 2 passed; 0 failed
```

The test's own hand decoder (`compute_handler_address_by_hand`) is a second,
independent implementation that reads the four signed bytes at the stub plus
`0x09` and computes the address itself; a grep of every code line (comments
stripped) confirms zero occurrences of `decode_stub` or `StubHandler`:

```
$ grep -v '^[[:space:]]*//' crates/deform6/tests/events.rs | grep -c 'decode_stub\|StubHandler'
0
```

**Live run.**

```
$ ./target/debug/deform6 inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe | grep -E "event slot 8: bound"
      event slot 8: bound, handler at 0x00404640, name not decoded. Run with --event-name-table to supply one.
```

This session's own live run over `frmFire` found **three** bound slots
(slot 8 on the form itself, plus `cmdStop` and `cmdStart` slot 0 each), not
the one the plan's own pre-measurement text named — matching the SUMMARY's
own correction and confirming the "measure again, the plan text is a lead"
discipline actually held. No unbound slot in the same run carries an
address (`grep 'unbound' | grep -cE '0x[0-9a-f]{4,}'` returns `0`), and the
`--event-name-table` flag still appears on every undecoded slot (323 times
across the whole corpus sweep this session ran), so ROADMAP success
criterion 6 stays true word for word.

**Break-on-purpose evidence.** The SUMMARY records the exact failure output
from dropping the wiring in both `report_events` and `print_event`, restored
before commit. This session did not re-run the break itself (re-running it
would require editing the shipped tree); the recorded panic messages
(`left: []` vs `right: [(...)]` for the report test, and the literal string
mismatch for the CLI test) are consistent with the code structure this
session independently confirmed, and are treated as credible on that basis.

**Verdict: the third fact FRM-06 claims now reaches the product surface
exactly as the first two already did.** All three levels (code, test, live
run) agree, independently checked in this session and not merely read from
the SUMMARY.

## Adjudication 2: FRM-06's Checkbox, `[x]`

**Confirmed correct by this session's own independent measurement,
distinct from the SUMMARY's.**

This session re-swept all 44 corpus executables with a null-delimited loop
(`find corpus -type f -iname '*.exe' -print0`) and re-derived every number
the SUMMARY claims:

- **686 controls total**, counted by matching the exact `print_control`
  header shape (`  name  (Kind)` or `  name  (Kind, Index=N)`) inside the
  `Forms` section of every program's output. Matches the SUMMARY and the
  pinned ratio exactly.
- **679 controls carry at least one `event slot` line; 7 carry none.**
  Independently isolated with a small `awk` state machine that walks each
  control header to the next one, checked for any `event slot` line
  between them. The 7 no-slot controls are, byte for byte, the same seven
  the SUMMARY names: `Line1`/`Line2` in `Artificial Life.exe`,
  `Advanced Histogram Viewer.exe` (×3), `Basic Histogram Viewer.exe`, and
  `Image Levels.exe`.
- **Zero occurrences of the word `unsupported` anywhere in the full corpus
  sweep's output**, confirming no control anywhere hit
  `EventTable::unsupported_control_type`.

**The case-1/case-2 classification was independently re-derived from the
code, not taken from the SUMMARY's table.** `compose_control` in
`crates/deform6/src/vb/mod.rs:764-776` shows `events` is built only when
`control_info: Some(info)`; when `join_by_name`'s tree/table name join
finds no `ControlInfo` entry for a control's name, `control_info` is `None`
and `events` is `Vec::new()` directly, with `read_event_table` never
called and no defect ever recorded. This is structurally impossible to
confuse with `unsupported_control_type`, which only ever fires *inside*
`read_event_table`. Since the sweep found zero `unsupported` occurrences
anywhere, every one of the 7 no-slot controls is provably case 1 (the file
itself declares no `ControlInfo` entry for that control), not case 2 (a
tool limitation). This matches known VB6 domain fact independent of this
repository: `Line` and `Shape` are windowless, event-less controls the VB6
IDE itself never lets a developer attach code to, which is consistent with
the compiler never emitting a `ControlInfo`/event-table entry for one.

**FRM-06's checkbox is correctly `[x]`.** All three narrowed facts ship
(bound state, slot index, handler address), and the 7 exceptions are
genuine file-declared absences, not a tool gap.

## Adjudication 3: FRM-03's Checkbox, Returned to `[ ]`

**Confirmed by an independent re-measurement, with the nine-space trap
specifically guarded against.**

This session re-ran the sweep with `find corpus -type f -iname '*.exe'
-print0` piped through a `while IFS= read -r -d ''` loop (never a
whitespace-splitting loop), walking all 44 executables (`echo $n` confirmed
44 before any counting began, so the nine space-bearing corpus paths were
not silently dropped). Reading only the `Forms` section of each program's
output:

```
named property lines (Caption/BackColor/BorderStyle/Position/WindowState): 122
  Caption: 51
  BackColor: 36
  BorderStyle: 30
  Position: 4
  WindowState: 1
undecoded property (opcode) lines: 683
```

122 + 683 = 805, and the five names are the only ones that ever appear as
a `Name = value` line inside the `Forms` section (a separate grep for every
distinct `name = ` pattern found only these five, plus `CLSID` and
`extents`, which are FRM-04 facts and correctly excluded from this count).
**This session's own numbers match the SUMMARY's numbers exactly, to the
individual property name.**

**Judgment: this is a requirement-wording question, not a code defect.**
FRM-03 literally claims "the property values of every control, and of the
form itself." 683 of 805 property records — five of every six on this
corpus — report present, byte offset given, opcode number given, name and
value undecoded. That sentence is not supported. But decision D-01
(`03-CONTEXT.md`) predicted this in writing before plan 03-02 shipped:
"roughly 158 of the 198 pairs stay open after plan 03-02," and states
explicitly that closing the remainder needs a human at a working VB6
install compiling small test programs and diffing the bytes, a campaign
D-01 places outside phase 3's scope by design. **The code is working
exactly as designed; the requirement's literal wording overstates what a
lawful, non-`VB6.OLB`-derived opcode table can deliver inside this phase.**
The checkbox is correctly returned to `[ ]`, matching what the code
supports today, and the underlying design decision that makes the gap
permanent-until-a-human-campaign is a genuine requirement-wording question,
newly routed to human verification in this report (it was not flagged as
such in the prior verification, which treated the `[x]`→`[ ]` correction
purely as a checkbox-honesty fix).

## Adjudication 4: Every Other Requirement Checkbox

| Requirement | Checkbox | This session's judgment |
| ----------- | -------- | ------------------------ |
| FRM-01 | `[ ]` | **Correct.** Re-ran `the_pinned_file_holds_fifty_two_of_fifty_three_forms_and_six_hundred_eighty_six_of_six_hundred_eighty_six_controls` and `the_forms_and_controls_gate_passes_on_the_committed_file` live: both still pass at 52/53 forms, no regression. "Every form" is still not literally true (`Map Editor.exe`'s `Main` form still refuses at file offset `0x170e`, re-confirmed live this session; `frmPassGen`'s menu-nesting ambiguity is unchanged, per `WINDOWS.md` findings 8 and 9, both still `open`). |
| FRM-02 | `[ ]` | **Correct, and more precisely justified this session than the prior "conservative" mark.** `forms_controls_counts` in `differential.rs:1016-1051` (read directly, not inferred) explicitly excludes a form the tool could not build a tree for from `control_declared`/`control_recovered` both, to avoid double-counting the same shortfall as a form failure and a control failure. This means `Map Editor.exe`'s `Main` form's own controls carry **zero** recovered type or name anywhere in the deliverable — they are not "686 of 687, one missing," they are absent from the count entirely. "Every control" is therefore not met, independent of the frmPassGen parent-assignment issue (which is a FRM-01 tree-structure fact, not a FRM-02 type/name fact — frmPassGen's three mis-parented controls still have correct type and name). |
| FRM-03 | `[ ]` | **Correct.** See Adjudication 3 above. |
| FRM-04 | `[ ]` | **Correct, unchanged, not re-adjudicated.** Carried forward per this verification's own brief; see the `human_verification` block. |
| FRM-05 | `[ ]` | **Correct.** Re-ran `cargo test -p deform6 --test blobs` live (4/4 pass) and a live `inspect` run on `Fast_Flames.exe` (Icon line present with a real offset, length and format). The extraction half is genuinely wired; `grep` for a write function (`fn write`, `write_frx`, `fs::write`, `File::create`) in `frx.rs` still returns nothing, so the `.frx` writer half FRM-05 also demands is still absent and still correctly deferred to Phase 4 plan 04-04 by `ROADMAP.md`'s own named risks. |
| FRM-06 | `[x]` | **Correct.** See Adjudications 1 and 2 above. |
| VER-06 | `[ ]` | **Conservative but defensible; not a gap.** Re-ran `differential::the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect` live (passes). The exclusion names the file by path, carries a non-empty reason (`support/frm.rs:310-315`), and a family of tests in `support_selftest.rs` and `differential.rs` pin the current state (exactly one exclusion, `frmHMM.frm` never excluded, every other corpus form never excluded). What no test does is literally empty `EXCLUSIONS` and observe the gate fail with a named message — the "never passes silently" behavior is documented in code comments and enforced structurally (an empty `EXCLUSIONS` would make `frmHMM.frx`'s 55-of-56 corrupted bytes flow into the differential comparison as real evidence, per the comment at `differential.rs:1414-1419`), but is not proven by an executed break-it-and-watch-it-fail test the way `AGENTS.md`'s own testing discipline asks for. This is the same conservative call the prior verification made; this session finds no reason to overturn it, but records the specific gap in test rigor rather than silently repeating "satisfied, test passing" with no caveat. |

**No unearned `[x]` found. No genuinely-met requirement found unmarked.**
FRM-06 is the only mark that changed since the prior verification, and it is
earned at every level this session checked independently.

## Regression Check (Item 6)

All three previously-closed gaps confirmed still closed, from a real run in
this session, not from reading the prior report:

| Gap | Re-confirmed how | Result |
| --- | ----------------- | ------ |
| FRM-05 blob extraction reaching the report | `cargo test -p deform6 --test blobs` (live) + live `inspect` grep for `icon` | 4/4 tests pass; live run prints the Icon blob line with offset, length and format |
| 52/53 forms, 686/686 controls | `cargo test -p deform6 --test ratios -- --exact <both test names>` (live) | Both pass, same counts as before |
| Code review findings WR-01, WR-02, WR-03, IN-01 | Direct source read + `cargo test` for WR-03's dedicated test | `error::damaged` is the sole definition (`error.rs:489`); `close_walk`'s signature no longer takes `stack`; the zero-denominator test passes; `grep -c` for the UTF-8 em-dash sequence in `controltree.rs` returns `0` |

No regressions found.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Control tree of every form, correct parent | ⚠️ PARTIAL (unchanged) | 52/53 forms, live-measured; 1 honest refusal with byte offset (Map Editor); 1 form with a documented, open parent ambiguity for 3 controls (frmPassGen); no regression |
| 2 | Type and name of each control | ⚠️ PARTIAL (re-scoped, not previously stated this precisely) | 686/686 controls recovered for every form whose tree built; Map Editor's Main form's own controls are entirely absent from the count by the harness's own documented design, so "every control" is not literally met |
| 3 | Property values of each control and of the form | ✗ NOT MET as literally worded, working as designed | 122/805 named, 683/805 undecoded, independently re-measured this session; D-01 predicted this; requirement-wording question routed to human_verification |
| 4 | CLSID of each third-party OCX control | ? NEEDS HUMAN | Unchanged from prior verification; not re-adjudicated per this session's brief |
| 5 | The resource blobs (extraction) | ✓ VERIFIED | Re-confirmed live this session; extraction genuinely wired, writer correctly deferred to Phase 4 |
| 6 | Event structure: which slots are bound, index of each | ✓ VERIFIED | Re-confirmed live this session, no regression |
| 7 | Event structure: native address of each bound handler | ✓ VERIFIED | Newly closed by plan 03-18; independently re-confirmed at code, test and live-run level this session |

**Score:** 7/7 must-haves for this re-verification's own scope (the code
gap plan 03-18 was asked to close) are verified. Two of the seven
goal-level truths (control tree completeness, property value completeness)
remain partial/not-met exactly as before, by the phase's own honest design,
and are not new gaps — they were already routed to deferred/human tracks in
the prior verification and remain there. **Status is `human_needed`, not
`passed`,** because two items sit in the human-verification queue: FRM-04
(carried forward unchanged) and FRM-03 (newly surfaced by this session's
adjudication as a requirement-wording question, not a code gap).

### Requirement Checkbox Honesty

| Requirement | Checkbox | Supported by code? |
| ----------- | -------- | ------------------- |
| FRM-01 | `[ ]` | Correctly unchecked — 52/53, not "every" |
| FRM-02 | `[ ]` | Correctly unchecked — Map Editor's Main form's controls are entirely absent from the count |
| FRM-03 | `[ ]` | Correctly unchecked — 122/805, working as D-01 designed |
| FRM-04 | `[ ]` | Correctly unchecked — value unconfirmed, human decision pending |
| FRM-05 | `[ ]` | Correctly unchecked — extraction closed, writer deferred |
| FRM-06 | `[x]` | **Correctly checked** — all three facts ship and are live-confirmed |
| VER-06 | `[ ]` | Conservatively unchecked — mechanism satisfied, "never passes silently" not proven by an executed break-test |

The Traceability table still reads "Gaps Found" for FRM-01 to FRM-06 and
VER-06. This remains accurate: FRM-01, FRM-02, FRM-03, FRM-04 and FRM-05
all still have genuine, documented open items.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` found in any of the four
files plan 03-18 touched (`controlinfo.rs`, `events.rs`, `main.rs`,
`cli.rs`), checked directly with grep this session. No em-dash found in any
of them either. Commit messages for all five plan 03-18 commits
(`7293e64`, `b77e0b5`, `e18095e`, `a1f638b`, `20855d3`) checked directly and
carry no agent name, no `Co-Authored-By` line, no session link and no tool
footer. The documentation-only commit (`e18095e`) touches
`.planning/REQUIREMENTS.md` only, confirmed by `git diff --name-only`.

## Gaps Summary

No gaps. The gate is genuinely green, run live in this session: `cargo fmt`,
`cargo clippy -D warnings`, and the full workspace test suite all pass with
zero failures across 13 binaries. Plan 03-18 closed the one code gap the
second verification found — `handler_address` now has a production call
site, reaches `EventReport`, `ControlReport`, and a live `deform6 inspect`
run, proven end to end by a test that reaches it only through
`deform6::inspect` and fails when the wiring is removed. This session
independently re-derived every number the SUMMARY claims (three bound
slots on `Fast_Flames.exe`, 686 controls, 679 with an event slot line, 7
without, all 7 genuinely `Line` controls with no `ControlInfo` entry, 122
named property values against 683 undecoded) and found them all correct.

Two items remain in the human-verification queue, neither a code defect:
FRM-04's CLSID wording question (carried forward unchanged from the prior
verification) and FRM-03's property-completeness wording question (newly
surfaced this session, given D-01's own written prediction that most
property pairs would stay open by design). Both are requirement-scope
decisions a human, not a test, must settle.

---

_Verified: 2026-09-11_
_Verifier: Claude (gsd-verifier)_
