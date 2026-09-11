---
phase: 03-forms
verified: 2026-09-11T00:00:00Z
status: gaps_found
score: 4/7 truths verified
covered_files: [".planning/REQUIREMENTS.md", ".planning/ROADMAP.md", ".planning/WINDOWS.md", ".planning/phases/03-forms/03-11-PLAN.md", ".planning/phases/03-forms/03-11-SUMMARY.md", ".planning/phases/03-forms/03-12-PLAN.md", ".planning/phases/03-forms/03-12-SUMMARY.md", ".planning/phases/03-forms/03-13-PLAN.md", ".planning/phases/03-forms/03-13-SUMMARY.md", ".planning/phases/03-forms/03-14-PLAN.md", ".planning/phases/03-forms/03-14-SUMMARY.md", ".planning/phases/03-forms/03-15-PLAN.md", ".planning/phases/03-forms/03-15-SUMMARY.md", ".planning/phases/03-forms/03-16-PLAN.md", ".planning/phases/03-forms/03-16-SUMMARY.md", ".planning/phases/03-forms/03-17-PLAN.md", ".planning/phases/03-forms/03-17-SUMMARY.md", ".planning/phases/03-forms/03-CONTEXT.md", ".planning/phases/03-forms/03-RESEARCH.md", ".planning/phases/03-forms/03-REVIEW.md", "crates/deform6-cli/src/main.rs", "crates/deform6-cli/tests/cli.rs", "crates/deform6/src/error.rs", "crates/deform6/src/vb/controlinfo.rs", "crates/deform6/src/vb/controltree.rs", "crates/deform6/src/vb/frx.rs", "crates/deform6/src/vb/gui.rs", "crates/deform6/src/vb/mod.rs", "crates/deform6/src/vb/ocx.rs", "crates/deform6/src/vb/opcodes.rs", "crates/deform6/src/vb/project.rs", "crates/deform6/src/vb/propstream.rs", "crates/deform6/src/vb/vbstr.rs", "crates/deform6/tests/blobs.rs", "crates/deform6/tests/ratios.rs", "crates/deform6/tests/support/frm.rs", "crates/xtask/src/main.rs", "crates/xtask/src/opcode_table.rs", "tests/ratios.toml"]
covered_digest: "v1:sha256:164bb18fc61332f28a7e0a0b11829d3bc4a06e736fc8b9512ede3c829b1bab8e"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 3/6
  gaps_closed:
    - "The resource blobs: `frx::extract_blob` is now called from the production property-decode path (`propstream.rs`'s `Picture` arm) and a live `deform6 inspect` run surfaces the blob for every corpus program that has one. `crates/deform6/tests/blobs.rs` proves this through the public `deform6::inspect` entry point, not through `extract_blob` directly, and the test's own doc comment records the break-on-purpose evidence that it fails when the wiring is removed."
    - "The event handler names: FRM-06 and the Phase 3 goal were amended, citing decision D-02, to describe event structure (bound state and slot index) rather than a name the compiled file does not hold. The narrowed claim's bound-state and index sub-clauses are live-confirmed against `Fast_Flames.exe` exactly as the amended success criterion states."
    - "The control tree ratio: 52 of 53 forms and 686 of 686 controls, confirmed by running `the_forms_and_controls_gate_passes_on_the_committed_file`, which re-measures against a live `inspect` run and an independent `.frm` reader, not by reading `tests/ratios.toml` alone. The one remaining refusal (`Map Editor.exe`) names its own byte offset."
    - "Code review findings WR-01 (one shared `error::damaged` helper, no per-module duplication), WR-02 (dead pop-bounding code removed from `close_walk`), WR-03 (`format_ratio` now returns `\"n/a\"` for a zero-declared denominator instead of dividing), and IN-01 (zero em-dashes in `controltree.rs`) are all confirmed fixed in the shipped code."
    - "The three measured bugs: `BlobCursor::take` advances by `declared_len + FRX_ITEM_HEADER_LEN` where `FRX_ITEM_HEADER_LEN = 4` (not `+12`); `support/frm.rs` unescapes a doubled VB6 quote to one; `xtask update-ratios` writes all four form/control keys, confirmed by `format_entry` rendering all seven fields from one function."
    - "`03-RESEARCH.md`'s three disproven passages (the `Length - 2` zero-children bound, the 18-vs-16-byte position-block escape sample, and the `0x02`/`0x03` scope-run symmetry) are corrected in place, each citing the SUMMARY that disproved it, with the still-open scope-separator research (WINDOWS.md finding 7) named rather than silently resolved."
  gaps_remaining:
    - "FRM-04's CLSID value: still not confirmed to match any project file's own declared identifier for the one third-party control the corpus can test, even after selecting the closer of two candidate fields (`oUuid`). This is not a code defect this session can close from the executable alone; see the human-decision item below."
    - "SC1's literal 'every form': one form (`Map Editor.exe`'s `Main` form) still refuses, for a distinct, newly measured, unresolved cause (WINDOWS.md finding 8), and one form (`frmPassGen`) 'succeeds' while assigning the wrong parent to three menu controls because the byte grammar is genuinely ambiguous at that transition (WINDOWS.md finding 9). Both are self-disclosed by the executing plans, not hidden, and both are open."
  regressions: []
gaps:
  - truth: "The narrowed FRM-06 delivers all three of its own stated facts: which event slots are bound, the index of each slot, and the native address of each bound handler."
    status: failed
    reason: "The amendment that narrowed FRM-06 (plan 03-11) explicitly names the native handler address as one of three facts the requirement now claims, quoting `controlinfo.rs`'s own `report_events` and `03-09-SUMMARY.md`. `StubHandler.handler_address` is genuinely computed and unit-tested inside `controlinfo.rs` (including a cross-checked byte-for-byte proof against a real corpus stub), but `report_events`'s own return type, `EventReport`, has no field for it: `EventReport::Named`, `BoundUnnamed` and `Unbound` carry only `control_name` and `index`. `ControlReport.events: Vec<EventReport>` in `vb/mod.rs` therefore never carries it either, and `print_event` in `crates/deform6-cli/src/main.rs` has no address to print. A live `deform6 inspect` run against every bound slot in the corpus (confirmed on `Fast_Flames.exe`'s slot 8) never prints an address anywhere, and a whole-crate grep finds zero production call sites of `handler_address` outside `controlinfo.rs`'s own module and its own unit tests. This is the same class of defect the original verification caught for FRM-05's blob extraction: computed and tested in isolation, never wired to the product surface, and here it survives the wording amendment that was supposed to make FRM-06's claim match what ships."
    artifacts:
      - path: "crates/deform6/src/vb/controlinfo.rs"
        issue: "EventReport (Named/BoundUnnamed/Unbound) drops StubHandler.handler_address; report_events never receives or forwards it"
      - path: "crates/deform6/src/vb/mod.rs"
        issue: "ControlReport.events: Vec<EventReport> carries no address for any slot"
      - path: "crates/deform6-cli/src/main.rs"
        issue: "print_event has no address to print for any of its three EventReport arms"
    missing:
      - "Either add the handler address to EventReport's bound variants and print it (matching what the amended FRM-06 and ROADMAP goal actually claim), or amend FRM-06 and the goal a second time to state that the address is computed and cross-checked internally but not yet surfaced in the report, the same honesty discipline plan 03-11 already applied to the name."
deferred:
  - truth: "The `.frx` file writer emits blob bytes to disk with offsets the generated `.frm` agrees with (the second half of FRM-05)."
    addressed_in: "Phase 4 (plan 04-04)"
    evidence: "ROADMAP.md Phase 4 named risks: 'The .frm writer and the .frx writer are therefore one component ... Plan 04-04 owns both.' 03-15-SUMMARY.md documents the extraction half as this phase's own scope and the writer as phase 4's."
human_verification:
  - test: "Decide whether FRM-04 ('The tool recovers the CLSID of a third party OCX control...') is satisfied by a mechanically-produced, honestly-caveated value that eighteen searches across three corpus programs and six encodings never found matching the control's own registered identifier anywhere in the executable, or whether FRM-04 requires a confirmed-correct CLSID that this file format may not make recoverable at all."
    expected: "A human reads `STRUCTURES.md` section 7.3.1 (the eighteen-search record) and `03-16-SUMMARY.md`, and either (a) accepts the caveated `oUuid` value as satisfying FRM-04's letter if not its spirit, narrowing the requirement's wording the same way D-02 narrowed FRM-06, or (b) declares FRM-04 unmeetable from the compiled executable alone and defers the question (e.g. to a future phase that can read a `.vbp`/type library, or documents the limit permanently in the README per the Phase 6 goal)."
    why_human: "This is a requirement-wording and product-scope decision, not a code defect: the mechanism SC3 names (join by class name against the external component table) is implemented and executes exactly as described, and the shipped code is honest about the value's uncertainty (an unconditional caveat, confirmed printed beside every reported CLSID in a live run, with no claim of a match anywhere). Whether that honesty satisfies the word 'recovers' in FRM-04 is a policy call, not something grep or a test can settle."
---

# Phase 3: Forms Verification Report

**Phase Goal (as amended by plan 03-11, citing decision D-02):** `deform6
inspect` reports the control tree of every form with the correct parent for
each control, the type and the name of each control, the property values of
each control and of the form, the CLSID of each third-party OCX control, the
resource blobs, and the event structure of each control: which event slots
are bound, the index of each slot, and the address of each bound handler.

**Verified:** 2026-09-11
**Status:** gaps_found
**Re-verification:** Yes — after a seven-plan gap closure run (03-11 through 03-17)

## Full Gate (run live, not trusted from SUMMARY)

| Check | Command | Result |
| ----- | ------- | ------ |
| Format | `cargo fmt --all --check` | PASS (no output, exit 0) |
| Lint | `cargo clippy --all-targets -- -D warnings` | PASS (finished clean) |
| Tests | `cargo test --workspace` | PASS — every suite green, 13 binaries, 0 failed anywhere (410, 4, 2, 26, 1, 40, 9, 44, 2, 0, 28, 52, 0 passed) |

The gate is genuinely green, run live in this session, not read from a
SUMMARY. As before, the gaps below are goal-achievement gaps, not gate
failures.

## Re-Adjudication of the Four Original Gaps

### 1. FRM-06 event handler names — narrowing authorized; shipped code PARTIALLY delivers the narrowed claim

`REQUIREMENTS.md`'s FRM-06 bullet and `ROADMAP.md`'s Phase 3 goal, success
criteria and named-risk section all now describe event **structure**, cite
`D-02` at the amendment site (confirmed: `grep -c 'D-02'` is non-zero in
both files), and the withdrawn "event handler names" wording is gone from
both files. This narrowing is the authorized human decision named in this
verification's own brief and is not re-litigated here.

The narrowed text itself, however, claims three specific facts:

> which event slots the compiled form binds, the index of each slot, and
> the native address of each bound handler

A live run confirms the first two exactly as `ROADMAP.md`'s own sixth
success criterion states:

```
$ ./target/debug/deform6 inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe | grep "event slot"
      event slot 8: bound, not decoded. Run with --event-name-table to supply one.
      (slots 0-7 and 9-30: unbound, not decoded...)
```

One control shows slot 8 bound and every other slot unbound, matching SC6's
own literal claim. **The third fact, the native address of each bound
handler, is computed and unit-tested inside `controlinfo.rs`
(`StubHandler::handler_address`, cross-checked byte for byte against a real
corpus stub in `03-09-SUMMARY.md`) but is never wired to the product
surface.** `report_events`'s return type, `EventReport`, carries only
`control_name` and `index` in all three of its variants; `ControlReport`
in `vb/mod.rs` and `print_event` in the CLI inherit that omission. A live
run and a whole-crate grep both confirm no address is ever printed. This is
a new gap, not one of the original four, but it sits squarely inside the
truth this verification was asked to re-check: **the shipped code does not
fully deliver what the narrowed wording claims.** See the `gaps:`
frontmatter entry.

### 2. FRM-05 resource blobs — extraction half CLOSED; writer half correctly deferred

`crates/deform6/tests/blobs.rs` exists, and its own doc comment states
directly: "every test in this file therefore goes through
`deform6::inspect`, the one public entry point the command line crate
itself calls, and never through `extract_blob` directly. If the resource
blob arm stops calling `extract_blob`, every test here fails." This is the
break-on-purpose evidence the brief asked for, and it is credible: the test
reconstructs the recovered blob's exact bytes from `Report.forms` and
compares them against the committed `.frx`, read at run time, never against
anything DeForm6 itself produced.

```
$ cargo test -p deform6 --test blobs
test winsock_sample_blob_declared_length_and_image_length_match_this_sessions_own_measurement ... ok
test winsock_sample_frm_main_recovers_a_blob_matching_the_committed_frx ... ok
test fast_flames_blob_declared_length_and_image_length_match_this_sessions_own_measurement ... ok
test fast_flames_frm_fire_recovers_a_blob_matching_the_committed_frx ... ok
test result: ok. 4 passed; 0 failed
```

A live run confirms the same thing from the command line:

```
$ ./target/debug/deform6 inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe | grep -i icon
      Icon: resource blob at offset 0x13d5, declared length 1414, 1406 image byte(s), format ICO, .frx offset 0x0 (an offset into a file this run did not write)
```

**FRM-05 as written also demands an `.frx` writer.** `grep` for a write
function (`fn write`, `write_frx`, `fs::write`, `File::create`) in `frx.rs`
returns nothing; no writer exists. This is correctly, explicitly deferred:
`REQUIREMENTS.md`'s FRM-05 bullet is still unchecked, `ROADMAP.md` Phase
4's own named risks assign the writer to plan 04-04 by name, and
`03-15-SUMMARY.md` states its own scope boundary the same way. **Verdict:
the extraction half is genuinely closed in phase 3; the writer half cannot
close in phase 3 by the phase's own design and is correctly not claimed
here.**

### 3. FRM-04 OCX CLSID — caveat confirmed unconditional; no claim of a match anywhere; requirement wording needs a human decision

`join_component` now reports `oUuid` (`248DD896-BB45-11CF-9ABC-0080C7E7B78D`),
one byte off the corpus `.vbp`'s declared `248DD890-BB45-11CF-9ABC-0080C7E7B78D`,
and `print_clsid`'s own code (`crates/deform6-cli/src/main.rs:766-779`)
prints the caveat inside the same `Some(clsid) =>` arm that prints the
value itself, on every path, with no condition gating it on anything the
run measures. A live run confirms this:

```
$ ./target/debug/deform6 inspect .../SubReality_WinsockSample.exe | grep -A1 CLSID
CLSID = {248DD896-BB45-11CF-9ABC-0080C7E7B78D}
the value {248DD896-BB45-11CF-9ABC-0080C7E7B78D}, read from the entry's own oUuid field at offset 0x1f18, is not confirmed to match the identifier a project file's own Object= line declares for this control; this repository's own research found no field of the external component table entry that does
```

No sentence anywhere in the caveat, in `print_clsid`, or in
`STRUCTURES.md` section 7.3.1 claims a match. `STRUCTURES.md` records
eighteen searches (six encodings, three corpus programs) that never find
the declared identifier anywhere in the external component table entry, a
result confirmed by this session reading the cited section directly.
`REQUIREMENTS.md`'s FRM-04 checkbox is correctly still `[ ]`, and
`03-16-SUMMARY.md` explicitly declines to mark it complete, citing the
exact overstatement the original verification found as the thing not to
repeat.

**Judgment: FRM-04 is not met, not simply failed, and not closeable from
the executable alone with the evidence this repository can lawfully hold.**
The mechanism SC3 names (join by class name against the external component
table) is implemented and executes exactly as described; the value it
produces is honestly reported as unconfirmed rather than asserted as
correct. Whether "the tool recovers the CLSID" is satisfied by an honestly
caveated, mechanically-selected value, or requires a value proven correct
(which this file format may never allow, per the eighteen-search record),
is a requirement-wording question. It is recorded as a human-verification
item, not scored as a plain failure.

### 4. SC1 "every form" — 49/53 to 52/53, confirmed from a real run; one refusal remains, one ambiguity is honestly recorded

```
$ cargo test -p deform6 --test ratios -- --exact the_forms_and_controls_gate_passes_on_the_committed_file the_pinned_file_holds_fifty_two_of_fifty_three_forms_and_six_hundred_eighty_six_of_six_hundred_eighty_six_controls
test the_pinned_file_holds_fifty_two_of_fifty_three_forms_and_six_hundred_eighty_six_of_six_hundred_eighty_six_controls ... ok
test the_forms_and_controls_gate_passes_on_the_committed_file ... ok
```

`the_forms_and_controls_gate_passes_on_the_committed_file` re-measures form
and control counts from a live `inspect` run against `support::frm`, an
independent second reader, on every run; it is not reading the pinned TOML
file as ground truth. This is a real measurement, not a repeated assertion
of the same static number: 52 of 53 forms, 686 of 686 controls.

The one remaining refusal is `Map Editor.exe`'s `Main` form:

```
$ ./target/debug/deform6 inspect "corpus/vb6-code/Map-editor-2D/Map Editor.exe" 2>&1 | grep refused
refused: ... the structure at offset 0x1561 could not be read: this Visual Basic 6 executable is damaged: expected a scope separator (0xFF) at file offset 0x170e, found 0x37
```

An honest refusal naming an exact byte offset, matching `tests/ratios.toml`'s
own header comment ("The one form still refusing is named in `WINDOWS.md`,
with its own byte offset") and `WINDOWS.md` finding 8 (open, distinct from
the now-fixed finding 7).

`frmPassGen`'s `menuAbout` nesting ambiguity (WINDOWS.md finding 9) is
confirmed directly from a live run: `menuAboutForm`, `menuSeparatorC` and
`menuWebsite` print at the same indent level as `menuHotkeys` and
`menuSeparatorB` under `menuHelp`, not nested under `menuAbout`. This is
recorded in `WINDOWS.md` (open) and in `controltree.rs`'s own
`read_scope_run` doc comment, both cited by `03-14-SUMMARY.md`, which
states plainly that this transition is byte-for-byte indistinguishable from
the confirmed sibling case with the data the parser reads. It is not a
guess dressed as a result: the count and tiling checks pass (all 39 menu
controls recovered), but the parent assignment for three of them is a
documented, open unknown, not a silently wrong answer presented as correct.

**Verdict: PARTIAL, materially improved (49/53 to 52/53), both remaining
causes honestly named with byte offsets, neither hidden.** SC1's literal
"every form" is still not met.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Control tree of every form, correct parent | ⚠️ PARTIAL | 52/53 forms, live-measured; 1 honest refusal with byte offset (Map Editor); 1 form with a documented, open parent ambiguity for 3 controls (frmPassGen) |
| 2 | Type and name of each control | ✓ VERIFIED | Carried forward from prior verification; full gate re-run this session with no regression |
| 3 | Property values of each control and of the form | ✓ VERIFIED | Carried forward and extended: plan 03-13 adds 3 corpus-measured Form/MDIForm rows (Caption, BackColor, Icon), reaching the resource blob opcode |
| 4 | CLSID of each third-party OCX control | ? NEEDS HUMAN | Mechanism implemented and wired exactly as SC3 describes; value is honestly caveated as unconfirmed, never claimed as a match; see human-verification item |
| 5 | The resource blobs (extraction) | ✓ VERIFIED | `crates/deform6/tests/blobs.rs` proves this through `deform6::inspect`; live run confirms; break-on-purpose evidence recorded in the test's own doc comment |
| 6 | Event structure: which slots are bound, index of each | ✓ VERIFIED | Live-confirmed against `Fast_Flames.exe`, exact match to SC6's own literal claim (slot 8 bound, all others unbound) |
| 7 | Event structure: native address of each bound handler | ✗ FAILED | Computed and unit-tested inside `controlinfo.rs`; never reaches `EventReport`, `ControlReport`, or the CLI printer; zero production call sites outside its own module |

**Score:** 4/7 truths fully verified (2 carried forward unchanged, 2 newly
closed this session). 1 partial (materially improved). 1 needs a human
wording decision. 1 newly identified failure. VER-06 (frmHMM.frx exclusion)
is verified separately below and is not one of the seven goal-level truths.

### Code Review Findings (WR-01, WR-03, IN-01)

| Finding | Fix claimed | Verified |
| ------- | ----------- | -------- |
| WR-01 (triplicated `damaged()`, unbounded leak) | One shared `pub(crate) fn damaged` in `error.rs` | ✓ Confirmed: only definition is `crates/deform6/src/error.rs:489`; `gui.rs`, `controltree.rs`, `frx.rs` no longer define their own copies |
| WR-03 (`format_ratio` divide-by-zero) | Explicit `declared == 0` guard returning `"n/a"` | ✓ Confirmed at `crates/deform6/tests/ratios.rs:385-391`, plus a dedicated test (`a_declared_of_zero_gives_the_named_result_and_not_a_division`, passing) |
| IN-01 (10 em-dashes in `controltree.rs`) | Removed | ✓ Confirmed: `grep -c` for the UTF-8 em-dash sequence returns 0 in `controltree.rs` and in every other reviewed file |
| WR-02 (dead pop-bounding code in `close_walk`, not part of the original three findings but tracked in `03-REVIEW.md`) | `close_walk` no longer takes a `stack` parameter at all | ✓ Confirmed: signature is now `close_walk(region, end_at, tiling)`; a dedicated test (`close_walk_no_longer_refuses_an_end_form_pop_count_larger_than_the_stack_depth`) documents the accepted behavior change |

### Bugs Found By Measurement

| Bug | Claimed fix | Verified |
| --- | ----------- | -------- |
| `BlobCursor` advance | `declared_len + 4`, not `+ 12` | ✓ `FRX_ITEM_HEADER_LEN: u32 = 4` (`frx.rs:244`), used directly in `BlobCursor::take` |
| VB6 doubled-quote unescape | `support/frm.rs` unescapes `""` to `"` | ✓ `value[1..value.len()-1].replace("\"\"", "\"")` (`frm.rs:185`), test passing |
| `xtask update-ratios` four keys | Writes `form_declared`/`form_recovered`/`control_declared`/`control_recovered` | ✓ Confirmed in `crates/xtask/src/main.rs`; `tests/ratios.toml` carries all four for all 44 programs, summed and cross-checked live |

### `03-RESEARCH.md` Staleness

Plan 03-12 corrected the three passages the original verification's Q6
named: the `Length - 2` zero-children bound (now marked disproven as a
general rule, `Length - 1` cited with `03-06-SUMMARY.md`), the
position-block escape's self-contradictory 18-vs-16-byte illustration (now
`checked_add(12)`, matching the shipped 16-byte read), and the `0x02`/`0x03`
scope-run symmetry (now split into the shipped `ScopeRun::` variants, citing
`03-04-SUMMARY.md` and naming `WINDOWS.md` finding 7 as still open rather
than silently resolved). All three corrections are confirmed present in the
current file by direct grep. **Verified.**

### Requirement Checkbox Honesty

| Requirement | Checkbox | Supported by code? |
| ----------- | -------- | ------------------- |
| FRM-01 | `[ ]` | Correctly unchecked — 52/53, not "every" |
| FRM-02 | `[ ]` | Conservatively unchecked (previously verified) |
| FRM-03 | `[x]` | ✓ Supported — extended coverage this session, no regression |
| FRM-04 | `[ ]` | Correctly unchecked — value unconfirmed, human decision pending |
| FRM-05 | `[ ]` | Correctly unchecked — extraction closed, writer deferred |
| FRM-06 | `[ ]` | Correctly unchecked — bound-state/index delivered, handler address not |
| VER-06 | `[ ]` | Conservatively unchecked (satisfied, test passing) |

No new premature marks found. The four overstated checkmarks the original
verification caught (FRM-01, FRM-04, FRM-05, FRM-06) are now all correctly
`[ ]`, and none of the seven Phase 3 requirement rows overstates what the
code as it stands supports. The Traceability table still reads "Gaps Found"
for FRM-01 to FRM-06 and VER-06, which remains accurate.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX` debt markers found across all 37 covered files
(checked directly with grep). No `TODO`/`HACK`/`PLACEHOLDER` found. No
em-dash found in any of the 17 `src`/`tests` files this phase's gap closure
touched.

## Gaps Summary

The gate is genuinely green, run live: `cargo fmt`, `cargo clippy -D
warnings`, and all 618 tests across 13 binaries pass with zero failures.
Five of the original verification's concerns are now solidly closed: the
resource blob extraction is wired to the product path and proven through
the public entry point, not a unit test in isolation; the control tree
ratio genuinely improved from a real re-measurement (49/53 to 52/53) with
both remaining causes honestly named; all four code review findings are
fixed; the three measurement bugs are fixed; and `03-RESEARCH.md`'s stale
prose is corrected in place.

Two items remain open by the phase's own honest design and are not scored
as failures: FRM-05's `.frx` writer (correctly deferred to phase 4) and
FRM-04's CLSID value (a requirement-wording question this session cannot
settle from the executable alone, routed to a human decision).

One genuinely new gap surfaced during this re-verification: the amendment
that narrowed FRM-06 (plan 03-11) explicitly claims the tool recovers "the
native address of each bound handler," and that fact is computed and
tested inside `controlinfo.rs` but never reaches `Report`, `inspect`, or
any CLI output. This is not one of the four original gaps and it is not
self-disclosed anywhere in `03-09-SUMMARY.md` or `03-11-SUMMARY.md` as an
unwired seam; it was found in this session by tracing `EventReport`,
`ControlReport` and `print_event` end to end and confirming a whole-crate
grep for `handler_address` outside `controlinfo.rs`. Given the amendment's
own text made this exact claim, "goal achieved" is still not an accurate
description of the current state until either the address is wired to the
report and the CLI, or the requirement is narrowed a second time the same
way D-02 narrowed the name.

---

_Verified: 2026-09-11_
_Verifier: Claude (gsd-verifier)_
