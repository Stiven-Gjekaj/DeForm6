---
phase: 03-forms
verified: 2026-09-11T00:00:00Z
status: passed
score: "8/8 goal-level truths resolved: 5 verified against the code (FRM-04, FRM-05 extraction half, FRM-06 bound state, FRM-06 handler address, VER-06), 3 accepted deferrals the human decided this phase (FRM-01, FRM-02, FRM-03; see 03-UAT.md items 2 and 3). The .frx writer half of FRM-05 is a fourth, separate deferral to Phase 4, tracked in the deferred: list but not one of the 8 top-level truths."
covered_files: [".planning/REQUIREMENTS.md", ".planning/ROADMAP.md", ".planning/WINDOWS.md", ".planning/phases/03-forms/03-01-PLAN.md", ".planning/phases/03-forms/03-01-SUMMARY.md", ".planning/phases/03-forms/03-02-PLAN.md", ".planning/phases/03-forms/03-02-SUMMARY.md", ".planning/phases/03-forms/03-03-PLAN.md", ".planning/phases/03-forms/03-03-SUMMARY.md", ".planning/phases/03-forms/03-04-PLAN.md", ".planning/phases/03-forms/03-04-SUMMARY.md", ".planning/phases/03-forms/03-05-PLAN.md", ".planning/phases/03-forms/03-05-SUMMARY.md", ".planning/phases/03-forms/03-06-PLAN.md", ".planning/phases/03-forms/03-06-SUMMARY.md", ".planning/phases/03-forms/03-07-PLAN.md", ".planning/phases/03-forms/03-07-SUMMARY.md", ".planning/phases/03-forms/03-08-PLAN.md", ".planning/phases/03-forms/03-08-SUMMARY.md", ".planning/phases/03-forms/03-09-PLAN.md", ".planning/phases/03-forms/03-09-SUMMARY.md", ".planning/phases/03-forms/03-10-PLAN.md", ".planning/phases/03-forms/03-10-SUMMARY.md", ".planning/phases/03-forms/03-11-PLAN.md", ".planning/phases/03-forms/03-11-SUMMARY.md", ".planning/phases/03-forms/03-12-PLAN.md", ".planning/phases/03-forms/03-12-SUMMARY.md", ".planning/phases/03-forms/03-13-PLAN.md", ".planning/phases/03-forms/03-13-SUMMARY.md", ".planning/phases/03-forms/03-14-PLAN.md", ".planning/phases/03-forms/03-14-SUMMARY.md", ".planning/phases/03-forms/03-15-PLAN.md", ".planning/phases/03-forms/03-15-SUMMARY.md", ".planning/phases/03-forms/03-16-PLAN.md", ".planning/phases/03-forms/03-16-SUMMARY.md", ".planning/phases/03-forms/03-17-PLAN.md", ".planning/phases/03-forms/03-17-SUMMARY.md", ".planning/phases/03-forms/03-18-PLAN.md", ".planning/phases/03-forms/03-18-SUMMARY.md", ".planning/phases/03-forms/03-CONTEXT.md", ".planning/phases/03-forms/03-RESEARCH.md", ".planning/phases/03-forms/03-REVIEW.md", ".planning/phases/03-forms/03-UAT.md", "crates/deform6-cli/src/main.rs", "crates/deform6-cli/tests/cli.rs", "crates/deform6/src/error.rs", "crates/deform6/src/vb/controlinfo.rs", "crates/deform6/src/vb/controltree.rs", "crates/deform6/src/vb/frx.rs", "crates/deform6/src/vb/gui.rs", "crates/deform6/src/vb/mod.rs", "crates/deform6/src/vb/ocx.rs", "crates/deform6/src/vb/opcodes.rs", "crates/deform6/src/vb/project.rs", "crates/deform6/src/vb/propstream.rs", "crates/deform6/src/vb/vbstr.rs", "crates/deform6/tests/blobs.rs", "crates/deform6/tests/differential.rs", "crates/deform6/tests/events.rs", "crates/deform6/tests/ratios.rs", "crates/deform6/tests/support/frm.rs", "crates/xtask/src/main.rs", "crates/xtask/src/opcode_table.rs", "tests/ratios.toml"]
covered_digest: "v1:sha256:1092a47456bbc39acfd4c41e758b448d8aa68db41996558f77aefd54e4534f96"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: "7/7 (that round's own narrow scope: the plan 03-18 gap only)"
  gaps_closed: []
  gaps_remaining: []
  regressions: []
deferred:
  - truth: "FRM-03, the property values of every control and of the form itself."
    addressed_in: "A human-run type library campaign named by decision D-01, outside every numbered phase; Phase 6 records the limit in the README."
    evidence: "REQUIREMENTS.md FRM-03's own deferral note: 'Decision D-01 predicts this result ... The remaining table is a data build job that needs a human at a working VB6 install ... That job is outside phase 3. Phase 6 records the limit in the README.' 03-UAT.md item 2 resolution: 'Accept as correctly unmet, defer coverage.'"
  - truth: "The .frx file writer emits blob bytes to disk with offsets the generated .frm agrees with (the second half of FRM-05)."
    addressed_in: "Phase 4 (plan 04-04)"
    evidence: "ROADMAP.md Phase 4 named risks: 'The .frm writer and the .frx writer are therefore one component ... Plan 04-04 owns both.' grep for a write function (fn write, write_frx, fs::write, File::create) in crates/deform6/src/vb/frx.rs returns nothing, confirmed this session."
  - truth: "FRM-01, the control tree of every form, with the parent of each control."
    addressed_in: "Accepted, documented deferral recorded in REQUIREMENTS.md this phase; the human decided this closes the phase, and no later phase owns the remaining grammar research."
    evidence: "03-UAT.md item 3 resolution: 'FRM-01 and FRM-02 keep their wording and stay open on purpose ... Record the deferral, close the phase.' REQUIREMENTS.md FRM-01's own deferral note, re-measured live this session: 52 of 53 corpus forms recover (cargo test -p deform6 --test ratios -- --exact the_pinned_file_holds_fifty_two_of_fifty_three_forms_and_six_hundred_eighty_six_of_six_hundred_eighty_six_controls, pass); Map Editor.exe's Main form refuses at file offset 0x170e, expecting byte 0xFF and finding 0x37 (deform6 inspect \"corpus/vb6-code/Map-editor-2D/Map Editor.exe\", re-run this session); WINDOWS.md finding 8 (status: open) holds that item; finding 9 (status: open) separately holds the frmPassGen menuAbout parent ambiguity. The note originally credited finding 9 with both; corrected this session, see corrections_applied."
  - truth: "FRM-02, the type and the name of every control."
    addressed_in: "Accepted, documented deferral recorded in REQUIREMENTS.md this phase; same root cause and the same human decision as FRM-01."
    evidence: "03-UAT.md item 3 resolution covers both FRM-01 and FRM-02 together: 'Record the deferral, close the phase.' REQUIREMENTS.md FRM-02's own deferral note, re-measured live this session: 686 of 686 controls recover over the forms whose tree the tool builds (same ratios test, pass); the refused Main form's controls are absent from both sides of that count by the harness's own documented design, confirmed by direct read of forms_controls_counts, crates/deform6/tests/differential.rs:1029-1055: recovered_form.controls.is_empty() continues the loop before either control_declared or control_recovered is touched."
corrections_applied:
  - item: "VER-06 checkbox in REQUIREMENTS.md"
    from: "[ ]"
    to: "[x]"
    reason: "This checkbox was reverted to [ ] by a bulk revert (commit e91192c, 2026-09-10) that ran requirements.revert-phase across all seven Phase 3 requirement IDs at once, undoing a mark the first verification round had earned on evidence. Nothing since has disputed the underlying code. This session re-verified all three clauses of VER-06's text directly against the shipped code (below) and found them independently met. Also corrected the matching Traceability row from 'Gaps Found' to 'Complete'."
  - item: "FRM-01 deferral note in REQUIREMENTS.md"
    from: "WINDOWS.md finding 9 holds the open question, and it also holds the menuAbout nesting of frmPassGen"
    to: "WINDOWS.md finding 8 holds this open question. Finding 9 holds a separate open question, the menuAbout nesting of frmPassGen"
    reason: "WINDOWS.md's own ledger names finding 8 (status: open) as Map Editor.exe's Main form scope-separator item at file offset 0x170e, and finding 9 (status: open) as the separate frmPassGen menuAbout parent ambiguity. The note as written credited finding 9 with both. Corrected this session by direct read of WINDOWS.md rows 8 and 9. No code changed; only the citation was wrong."
  - item: "covered_files in this file's own frontmatter"
    from: "45 entries, including this file (03-VERIFICATION.md) itself, and omitting 03-01-PLAN.md through 03-10-SUMMARY.md"
    to: "64 entries: this file removed, 03-01 through 03-10's PLAN.md and SUMMARY.md added"
    reason: "gsd_run query verification.status reported stale, not passed, after the digest recompute the brief asked for. Two independent causes, found by reading src/verification.cts's own routing logic (verification.cjs): (1) this file listed itself in covered_files, so its own digest field was part of the bytes the digest was computed over, an unsatisfiable self-reference confirmed by recomputing the fourth round's own digest against that round's own committed content (f105106e... recorded, 03838e1b... measured -- they never matched, so the file has been unable to read as non-stale since the fourth round, independent of anything in this brief). (2) covered_files never listed 03-01-PLAN.md through 03-10-SUMMARY.md, which verification.cjs's allCurrentArtifactsCovered requires when they exist on disk (they do, plans 01 through 10 predate the 03-11-PLAN.md floor covered_files started at). Fixed both: removed this file from its own covered list, added the twenty 03-01..03-10 PLAN/SUMMARY files, recomputed with gsd_run query verification.fingerprint. gsd_run query verification.status now reports passed. This finding contradicts the brief's own account of why the report read stale (it named only the REQUIREMENTS.md edit); the REQUIREMENTS.md edit was real but not sufficient on its own to explain the stale reading -- see the reply to the requester."
human_verification: []
---

# Phase 3: Forms Verification Report

**Phase Goal (as amended by plan 03-11, citing decision D-02, and by the
6198484 FRM-04 narrowing):** `deform6 inspect` reports the control tree of
every form with the correct parent for each control, the type and the name
of each control, the property values of each control and of the form, the
declared component identifier of each third-party OCX control (with the
honest caveat that it is not confirmed against the registered CLSID), the
resource blobs, and the event structure of each control: which event slots
are bound, the index of each slot, and the address of each bound handler.

**Verified:** 2026-09-11
**Status:** passed
**Re-verification:** Yes. Fourth verification (below, unchanged from that
round): requested as the intended final round after the human resolved the
two requirement-wording questions the third verification raised (FRM-04,
FRM-03) and after this session found and corrected one checkbox under-mark
left by an earlier bulk revert (VER-06). That round was not a code re-check
of plan 03-18 (already closed and confirmed by the third verification); it
was a full re-adjudication of every one of the seven Phase 3 requirement
checkboxes, in both directions, against the current wording of each. It
ended `human_needed`, with FRM-01 and FRM-02 routed to a human wording
decision. A fifth, narrowly scoped pass (below, under "Fifth pass") closes
that decision and moves the report to `passed`.

## Why this report differs from the prior one

The prior `03-VERIFICATION.md` graded FRM-04 against wording that no longer
exists (it asked for a CLSID; the requirement now asks for a declared
component identifier with a caveat) and left VER-06 unmarked without
re-deriving from first principles whether the mark was earned. This report
starts over on the checkbox question and treats the prior report's live-run
evidence (which did not depend on wording) as still valid where re-confirmed
below.

## Fifth pass: closing the two open wording decisions

Scope: check every factual claim in the FRM-01 and FRM-02 deferral notes
`REQUIREMENTS.md` now carries, move both items from `human_verification` to
`deferred`, recompute `covered_digest`, and set `status: passed` if nothing
else is outstanding. Not in scope: re-adjudicating the code, or re-running
the full corpus sweep.

**Claim check, FRM-01's note:**

| Claim | Measured | Result |
| --- | --- | --- |
| 52 of 53 corpus forms recover | `cargo test -p deform6 --test ratios -- --exact the_pinned_file_holds_fifty_two_of_fifty_three_forms_and_six_hundred_eighty_six_of_six_hundred_eighty_six_controls`, re-run live this session | pass |
| `Main` in `Map Editor.exe` refuses | `./target/debug/deform6 inspect "corpus/vb6-code/Map-editor-2D/Map Editor.exe"`, re-run live this session | `refused: ... expected a scope separator (0xFF) at file offset 0x170e, found 0x37` |
| Byte offset named is 0x170e, with the byte the code expected | same live run | `0x170e`, expected `0xFF`, found `0x37` — confirmed |
| `WINDOWS.md` finding 9 holds both the open grammar question and the `menuAbout` ambiguity | direct read of `.planning/WINDOWS.md`, ledger rows 8 and 9 | **Wrong as written.** Row 8 (`status: open`) is the Main-form scope-separator item at 0x170e. Row 9 (`status: open`) is a separate item, the `frmPassGen` `menuAbout` ambiguity; its own description names no byte offset for the Main form. Corrected in `REQUIREMENTS.md` this session (see `corrections_applied`, item 2). |

**Claim check, FRM-02's note:**

| Claim | Measured | Result |
| --- | --- | --- |
| 686 of 686 controls recover, over the forms whose tree the tool builds | same ratios test, re-run live this session | pass |
| The refusing form's controls are absent from BOTH sides of the count | direct read of `forms_controls_counts`, `crates/deform6/tests/differential.rs:1029-1055` | confirmed: `if recovered_form.controls.is_empty() { continue; }` runs before either `control_declared` or `control_recovered` is touched |

No other claim in either note was found wrong.

**Code-change check.** `git log --oneline` from the fourth round's own
commit (`d841b16`, "correct VER-06 to complete and record the fourth
verification") forward: `4903595` (touches `REQUIREMENTS.md`,
`03-UAT.md`), `b861b9f` (touches `03-VERIFICATION.md`'s status field only),
`384ce3a` (touches `03-VERIFICATION.md`'s YAML quoting only). `git show
--stat` on each confirms no `.rs` file in any of the three. The fourth
round's live gate result, regression table, and code-level findings (FRM-04,
FRM-05 extraction, FRM-06, VER-06) are re-affirmed without re-running them.

**Decision applied.** `03-UAT.md` item 3 records the human's decision for
FRM-01 and FRM-02 together: "Record the deferral, close the phase." Both
move from `human_verification` to `deferred` below (frontmatter). The
`human_verification` block is now empty.

**`covered_digest` recompute found a fingerprint bug, not just a stale
digest.** The brief's account was that `REQUIREMENTS.md` changing after the
digest was the reason `gsd_run query verification.status` read `stale`.
That edit is real, but recomputing the digest over the brief's own
`covered_files` list still left `verification.status` reporting `stale`.
Two independent causes, both in this file's own frontmatter, not in the
brief: this file listed itself in `covered_files` (an unsatisfiable
self-reference -- writing a digest into the file changes the file, which
changes the digest a fresh recompute would find; checked against the fourth
round's own committed digest, which never matched a recompute of that
round's own content either), and `covered_files` never listed the phase's
`03-01` through `03-10` `PLAN.md`/`SUMMARY.md` files, which
`verification.cjs`'s `allCurrentArtifactsCovered` check requires once they
exist on disk. Fixed both (see `corrections_applied`); `gsd_run query
verification.status` now reports `passed`, confirmed below.

## Full Gate (run live, not trusted from any SUMMARY)

| Check | Command | Result |
| ----- | ------- | ------ |
| Format | `cargo fmt --all --check` | PASS (exit 0, no output) |
| Lint | `cargo clippy --all-targets -- -D warnings` | PASS (finished clean) |
| Tests | `cargo test --workspace` | PASS — 14 binaries, 0 failed anywhere (411, 4, 2, 26, 2, 1, 40, 9, 44, 2, 0, 29, 52, 0 passed = 622 total) |

Run live in this session, from a clean working tree (`git status --porcelain`
showed only untracked `.gsd/`, `.planning/milestone.lock`, `Notes/`, none of
them tracked source). No test skipped, no test ignored.

## Requirement-by-requirement adjudication

### FRM-01 — `[ ]` — Correct, and now routed to a human wording decision

**Current wording:** "The tool recovers the control tree of every form, with
the parent of each control."

**Live re-measurement, this session:**

```
$ cargo test -p deform6 --test ratios -- --exact \
  the_pinned_file_holds_fifty_two_of_fifty_three_forms_and_six_hundred_eighty_six_of_six_hundred_eighty_six_controls \
  the_forms_and_controls_gate_passes_on_the_committed_file
test the_pinned_file_holds_fifty_two_of_fifty_three_forms_and_six_hundred_eighty_six_of_six_hundred_eighty_six_controls ... ok
test the_forms_and_controls_gate_passes_on_the_committed_file ... ok
test result: ok. 2 passed; 0 failed
```

```
$ ./target/debug/deform6 inspect "corpus/vb6-code/Map-editor-2D/Map Editor.exe"
...
refused: Site { offset: 5473, ... }: the structure at offset 0x1561 could not
be read: this Visual Basic 6 executable is damaged: expected a scope
separator (0xFF) at file offset 0x170e, found 0x37
```

52 of 53 forms. `Map Editor.exe`'s `Main` form refuses honestly, names the
byte offset, and prints no tree it cannot prove — ROADMAP success criterion
1's second sentence holds exactly. `WINDOWS.md` finding 9 (checked directly
this session, still `status: open`) records `frmPassGen`'s `menuAbout`
nesting ambiguity: "No byte in the header or property stream distinguishes
the two," so three controls (`menuAboutForm`, `menuSeparatorC`,
`menuWebsite`) recover with `menuHelp` as their parent instead of
`menuAbout`.

**Verdict: `[ ]` is correct as literal fact** — "every form" and "the
parent of each control" are not both true. **This is not a code defect.**
The refusal is honest and the ambiguity is provably undecidable from the
file's own bytes, per the tool's own documented finding. This is the same
shape FRM-03 was in before the human accepted D-01's deferral note.
**Newly routed to human_verification this session** (not routed by the
third verification, which treated FRM-01/02 as accepted, undocumented
limitations without asking): the checkbox mark is correct, but whether the
requirement's own wording should carry a recorded deferral, the way FRM-03
now does, is a decision only the human can make.

*Resolved in the fifth pass, above:* the human decided "record the
deferral, close the phase" (`03-UAT.md` item 3). This item is now in
`deferred`, and `human_verification` is empty.

### FRM-02 — `[ ]` — Correct, same root cause as FRM-01, same open question

**Current wording:** "The tool recovers the type and the name of every
control."

**Code trace, this session, read directly:**

```rust
// crates/deform6/tests/differential.rs:1029-1055
for declared_form in declared_forms(declared) {
    counts.form_declared = counts.form_declared.saturating_add(1);
    let Some(recovered_form) = report.forms.iter().find(|f| f.name == declared_form.name)
    else { continue; };
    if recovered_form.controls.is_empty() { continue; }
    counts.form_recovered = counts.form_recovered.saturating_add(1);
    ...
    counts.control_declared = counts.control_declared.saturating_add(...);
    counts.control_recovered = counts.control_recovered.saturating_add(...);
}
```

When a form's tree did not build (`recovered_form.controls.is_empty()`, true
for `Map Editor.exe`'s `Main` form), the loop `continue`s *before* touching
`control_declared` or `control_recovered`. `Main`'s own controls are absent
from **both sides** of the 686/686 count — not "685 of 686, one control
short." "Every control" is therefore not literally met, independent of
`frmPassGen`'s parent-assignment issue (a FRM-01 tree-structure fact, not a
FRM-02 type/name fact — `frmPassGen`'s three mis-parented controls still
carry the correct type and name).

**Verdict: `[ ]` is correct.** Same root cause as FRM-01 (`Map Editor.exe`'s
one refused form), no independent code defect. Routed to the same human
decision as FRM-01, since a deferral decision for one should apply to both.

*Resolved in the fifth pass, above:* same decision as FRM-01, "record the
deferral, close the phase" (`03-UAT.md` item 3). This item is now in
`deferred`.

### FRM-03 — `[ ]` — Correct, and already an accepted, documented deferral

**Current wording (amended by plan 03-18 measurement, unchanged in text):**
"The tool recovers the property values of every control, and of the form
itself." The bullet itself now carries a deferral note naming decision D-01,
122/805 named values, and Phase 6 as the place the limit gets documented.

`03-UAT.md` item 2 records the human's own decision: "Accept as correctly
unmet, defer coverage." **This is resolved, not open.** This session did not
re-litigate it; it re-confirms the checkbox matches the recorded decision
and moves this item to the `deferred:` list rather than `human_verification`,
since a human already decided it in this same phase.

### FRM-04 — `[x]` — Verified against the NEW wording, live

**Current wording (amended by commit `6198484`):** "The tool recovers the
component identifier that the executable declares for a third party OCX
control, and says plainly that this identifier is not confirmed against the
control's registered CLSID. The tool also says plainly that it cannot
interpret that control's property blob without the control's own type
library."

**Live measurement, this session, over all 44 corpus executables (null
delimited sweep):**

```
CLSID lines printed:                               3
lines carrying "is not confirmed to match the identifier ...":  3
lines carrying "needs the control's own type library":          3
```

Every one of the 3 reported identifiers (all `MSWinsockLib.Winsock`,
`{248DD896-BB45-11CF-9ABC-0080C7E7B78D}`) carries both caveats
unconditionally, confirmed with a direct read of one full live block:

```
wsPop  (External)
  class name = MSWinsockLib.Winsock
  CLSID = {248DD896-BB45-11CF-9ABC-0080C7E7B78D}
  the value {248DD896-BB45-11CF-9ABC-0080C7E7B78D}, read from the entry's
  own oUuid field at offset 0x1f18, is not confirmed to match the
  identifier a project file's own Object= line declares for this control;
  this repository's own research found no field of the external component
  table entry that does
  extents = 741 x 741 (HiMetric), version 393216
  property blob: not decoded. External control property blob at offset
  0x1d8d, length 35 bytes: value not decoded. Reading it needs the
  control's own type library, which this repository does not hold and may
  not redistribute.
```

The value is read from the component table's `oUuid` field (declared
identifier, per `03-16-SUMMARY.md`), not asserted as the registered CLSID.
The caveat sentence states plainly and unconditionally that the value is
"not confirmed to match the identifier a project file's own `Object=` line
declares." The property blob sentence states plainly that reading it "needs
the control's own type library."

**Verdict: `[x]` is earned against the current wording, confirmed by a live
run in this session, not by re-reading the prior report or any SUMMARY.**
Both facts the narrowed requirement asks for reach the product surface on
every one of the 3 corpus instances, unconditionally.

### FRM-05 — `[ ]` — Correct: extraction genuinely ships, writer genuinely absent

**Current wording:** "The tool recovers the resource blobs and writes an
`.frx` whose offsets the generated `.frm` agrees with."

**Extraction half, live this session:**

```
$ cargo test -p deform6 --test blobs
test winsock_sample_blob_declared_length_and_image_length_match_this_sessions_own_measurement ... ok
test winsock_sample_frm_main_recovers_a_blob_matching_the_committed_frx ... ok
test fast_flames_blob_declared_length_and_image_length_match_this_sessions_own_measurement ... ok
test fast_flames_frm_fire_recovers_a_blob_matching_the_committed_frx ... ok
test result: ok. 4 passed; 0 failed

$ ./target/debug/deform6 inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe | grep -i icon
Icon: resource blob at offset 0x13d5, declared length 1414, 1406 image byte(s), format ICO, .frx offset 0x0 (an offset into a file this run did not write)
```

The blob reaches the report and the terminal. **Writer half:**

```
$ grep -n "fn write\|write_frx\|fs::write\|File::create" crates/deform6/src/vb/frx.rs
(no output)
```

No `.frx` writer exists anywhere in `frx.rs`. This is not a gap left
unaddressed — `ROADMAP.md`'s own Phase 4 named risks state explicitly:
"The `.frm` writer and the `.frx` writer are therefore one component ...
Plan 04-04 owns both." **Verdict: `[ ]` is correct**, and the writer half is
already an accepted, documented deferral to Phase 4, moved to the
`deferred:` list below.

### FRM-06 — `[x]` — Verified, no change since the third verification

**Current wording (narrowed by plan 03-11, citing D-02):** which event slots
are bound, the index of each slot, and the native address of each bound
handler; never the event name.

Re-confirmed live this session, not re-derived from the SUMMARY:

```
$ ./target/debug/deform6 inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe | grep "event slot 8: bound"
event slot 8: bound, handler at 0x00404640, name not decoded. Run with --event-name-table to supply one.

$ ./target/debug/deform6 inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe | grep 'unbound' | grep -cE '0x[0-9a-f]{4,}'
0
```

`cargo test --workspace` (this session's live run) includes
`events.rs`'s two end-to-end tests, both green, reaching the value only
through `deform6::inspect`, with an independent hand-decoder that names
neither `decode_stub` nor `StubHandler`. No regression from the third
verification's own independent re-derivation of the same facts.
**Verdict: `[x]` is correct**, unchanged.

### VER-06 — corrected this session from `[ ]` to `[x]` — Verified, under-mark identified and fixed

**Wording:** "The known defect in `frmHMM.frx` is excluded by name, with the
reason recorded, and never passes silently."

**History, confirmed directly from `git log` this session:** commit
`e91192c` ("revert premature Complete requirements after gaps found",
2026-09-10) reverted **all seven** Phase 3 requirement checkboxes to `[ ]`
in one bulk edit, including VER-06, which the first verification round had
already earned on evidence delivered by plan 03-03. The bulk revert was a
blanket correction across every Phase 3 ID, not an individual finding
against VER-06's own code. Nothing committed since has disputed the
underlying mechanism.

**Independent re-verification of all three clauses, this session, direct
code read:**

```rust
// crates/deform6/tests/support/frm.rs:309-316
pub const EXCLUSIONS: &[Exclusion] = &[Exclusion {
    path: "corpus/vb6-code/Hidden-Markov-model/frmHMM.frx",
    reason: "the file is 56 bytes and its own record header declares 56 bytes of payload, but \
             only 55 are present, because the upstream repository sets text=auto and git's line \
             ending normalisation silently removed one carriage return; HMM.exe holds the same \
             string as a whole, correct record and is the second, independent source of truth \
             for this fault",
}];
```

1. **Excluded by name** — yes, `EXCLUSIONS[0].path` names the exact file.
2. **Reason recorded** — yes, the `reason` field carries the full,
   independently-corroborated explanation (`HMM.exe` as the second source).
3. **Never passes silently** — proven by an executed, currently-passing
   test, confirmed live this session:

```
$ cargo test -p deform6 --test differential -- --exact the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect
test the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect ... ok
test result: ok. 1 passed; 0 failed
```

This test asserts `EXCLUSIONS.len() == 1` and that the one entry names
`frmHMM.frx`, with an assertion message that reads: "`support::frm::
EXCLUSIONS` holds `{n}` entries; this differential gate expects exactly the
one named exclusion, `corpus/vb6-code/Hidden-Markov-model/frmHMM.frx`, or a
future `.frx` comparison would need to treat that file's own corrupted
bytes as real evidence." If a future edit emptied `EXCLUSIONS` (the literal
act ROADMAP success criterion 5 names — "Removing the exclusion"), this
exact test fails, in the exact gate (`cargo test --workspace`) this
requirement's own text and ROADMAP SC5 both name, with a named message that
states precisely what emptying it would cost. That is what "never passes
silently" asks for, proven by an already-passing, already-committed test —
not by a claim.

**This session attempted to additionally prove the counterfactual by live
demonstration (temporarily emptying `EXCLUSIONS` and re-running the test to
capture the exact failure text), the same "break it and watch it fail"
discipline `AGENTS.md` and every 03-1x/03-18 SUMMARY in this phase used.**
The sandbox's own auto-mode classifier blocked the edit ("Security Test
Removal") before any file changed; `git status --porcelain` confirmed no
source file was modified. The static evidence above (the assertion's own
`{n}` message and the exact-count check) is taken as sufficient without that
live demonstration: the assertion text is unambiguous about what fails and
why, and unlike the prior verification round's "conservative" stance, this
session finds no remaining ambiguity about whether the test would fire —
the code path is a direct, unconditional `assert_eq!` with a hard-coded
expected value of `1`.

**Verdict: all three clauses of VER-06 are met by the shipped code.** The
`[ ]` mark was an artifact of the blanket bulk revert, not a finding against
this requirement specifically. **Corrected to `[x]` this session** (see
`corrections_applied` in the frontmatter), with the matching Traceability
row corrected from "Gaps Found" to "Complete."

## Regression Check

All items the brief named, re-confirmed live in this session, not read from
any prior report:

| Item | Re-confirmed how | Result |
| --- | ----------------- | ------ |
| Handler address reaches the report and a live `inspect` prints it | `./target/debug/deform6 inspect .../Fast_Flames.exe \| grep "event slot 8: bound"` | `handler at 0x00404640` printed |
| An unbound slot prints none | `grep 'unbound' \| grep -cE '0x[0-9a-f]{4,}'` | `0` |
| Blob extraction reaches the report | `cargo test -p deform6 --test blobs` + live `inspect` grep for `icon` | 4/4 pass; Icon line with real offset/length/format printed |
| 52 of 53 forms, 686 of 686 controls | `cargo test -p deform6 --test ratios -- --exact <both names>` | 2/2 pass |
| WR-01 (`error::damaged` sole helper) | `grep -n "fn damaged" crates/deform6/src/error.rs` (1 hit) + 22 call sites across `vb/*.rs` | Confirmed sole definition, widely used |
| WR-02 (`close_walk` no longer takes `stack`) | direct read of `crates/deform6/src/vb/controltree.rs:759` | Signature is `fn close_walk(region, end_at, tiling)`, no `stack` parameter |
| WR-03 (zero-denominator guard) | direct read of `format_ratio` in `crates/deform6/tests/ratios.rs:385-391` + live test | `if declared == 0 { return "n/a" }` present; `ratios::a_declared_of_zero_gives_the_named_result_and_not_a_division` passes |
| IN-01 (em-dash removed from `controltree.rs`) | byte-count of U+2014 across the file | `0` occurrences |
| Full gate | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` | All three pass, 622 tests total, 0 failed |

No regressions found.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Control tree of every form, correct parent (FRM-01) | ✗ NOT MET as literally worded — accepted, documented deferral | 52/53 forms; Map Editor.exe's Main form refuses at file offset 0x170e (WINDOWS.md finding 8, open); frmPassGen's menuAbout parent ambiguity is a separate, open item (WINDOWS.md finding 9); human decided "record the deferral, close the phase" (03-UAT.md item 3); moved to `deferred` |
| 2 | Type and name of every control (FRM-02) | ✗ NOT MET as literally worded — accepted, documented deferral | Map Editor's Main form controls absent from both sides of the 686/686 count by the harness's own documented design; same human decision as #1 (03-UAT.md item 3); moved to `deferred` |
| 3 | Property values of every control and of the form (FRM-03) | ✗ NOT MET as literally worded — accepted, documented deferral | 122/805 named; D-01 predicted this; human already accepted (03-UAT.md item 2); moved to `deferred` |
| 4 | Declared component identifier of a third-party OCX control, with both caveats (FRM-04, amended wording) | ✓ VERIFIED | Live run over all 44 corpus programs: 3/3 CLSID lines carry both caveats unconditionally |
| 5 | Resource blobs (extraction) | ✓ VERIFIED | Live `inspect` + 4/4 `blobs` tests; writer half deferred to Phase 4 (`deferred`) |
| 6 | Event structure: bound state and slot index | ✓ VERIFIED | Re-confirmed live, no regression |
| 7 | Event structure: native address of each bound handler | ✓ VERIFIED | Re-confirmed at code, test, and live-run level, no regression |
| 8 | frmHMM.frx excluded by name, reason recorded, never passes silently (VER-06) | ✓ VERIFIED (corrected from an under-mark) | All three clauses independently confirmed against the shipped code; checkbox corrected `[ ] → [x]` |

**Score:** 8 of 8 goal-level truths resolved. Five verified against the code
(#4, #5's extraction half, #6, #7, #8). Three (#1, #2, #3) are accepted,
documented deferrals the human decided this phase (03-UAT.md items 2 and
3), not gaps. No item remains open.

### Requirement Checkbox Honesty (final state this pass leaves it in)

| Requirement | Checkbox | Supported by code? |
| ----------- | -------- | ------------------- |
| FRM-01 | `[ ]` | Correctly unchecked — 52/53, not "every"; wording question closed by the human this phase, recorded as an accepted deferral |
| FRM-02 | `[ ]` | Correctly unchecked — Map Editor's Main form controls absent from the count entirely; wording question closed by the human this phase, recorded as an accepted deferral |
| FRM-03 | `[ ]` | Correctly unchecked — 122/805; already an accepted, documented deferral |
| FRM-04 | `[x]` | **Correctly checked** — verified live against the amended wording |
| FRM-05 | `[ ]` | Correctly unchecked — extraction ships, writer deferred to Phase 4 |
| FRM-06 | `[x]` | **Correctly checked** — all three facts ship, live-confirmed, no regression |
| VER-06 | `[x]` | **Corrected this session** — was an under-mark from a bulk revert, not a code finding; all three clauses independently verified |

`REQUIREMENTS.md`'s Traceability table still reads `FRM-01 to FRM-06 |
Phase 3 | Gaps Found`. As of this pass that wording is stale in spirit:
FRM-01, 02, 03, 05 are no longer open questions, they are accepted,
human-decided deferrals, the same status FRM-03 and the FRM-05 writer half
already carried. Updating that one Traceability cell is outside this pass's
four-item scope (see brief); flagged here for a future, separate edit.
`VER-06 | Phase 3 | Gaps Found` was corrected to `Complete` in the fourth
round, unchanged this session.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` found in any file this
session inspected. No em-dash found in `controltree.rs` (`0` occurrences,
re-confirmed). Commit messages for every commit inspected this session
(`7293e64`, `b77e0b5`, `e18095e`, `a1f638b`, `20855d3`, `574165c`,
`e679afc`, `6198484`, `e21e186`, `1282be2`) carry no agent name, no
`Co-Authored-By` line, no session link, and no tool footer.

## Gaps Summary

No code gaps. The gate is genuinely green, confirmed by the fourth round's
live run (`cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`,
622 tests, 0 failed) and unaffected by any commit since, because every
commit since that run touches Markdown only (confirmed this session under
"Fifth pass" above). Every regression item the fourth round's brief named
was re-confirmed live in that round.

**Two corrections were made to REQUIREMENTS.md**, one in the fourth round
and one in this pass:

1. VER-06's checkbox and its Traceability row (fourth round). The fourth
   round's own independent re-derivation of all three of VER-06's clauses
   against the shipped code found the mark fully earned; it had been an
   under-mark from an earlier bulk revert across all seven Phase 3
   requirement IDs, not a fresh finding against the requirement.
2. FRM-01's deferral note (this pass). The note credited `WINDOWS.md`
   finding 9 with both the Main-form scope-separator item and the
   `frmPassGen` `menuAbout` ambiguity. Finding 9 holds only the latter;
   finding 8 holds the former. Corrected by direct read of the ledger; see
   `corrections_applied`, item 2.

**The two items the fourth round routed to `human_verification` are now
resolved.** `03-UAT.md` item 3 records the human's decision for FRM-01 and
FRM-02 together, since both trace to the one same refused form: "Record the
deferral, close the phase." Every claim in both deferral notes was checked
against the codebase this pass (see "Fifth pass" above); one citation was
wrong and is now fixed, no other claim was. Both items moved from
`human_verification` to `deferred`. The `human_verification` block is now
empty.

Status is `passed`: the gate is fully green, every truth that code alone
can settle is settled, and every truth that needed a human decision now has
one, recorded in `03-UAT.md` and `REQUIREMENTS.md`. Nothing remains open.

---

_Verified: 2026-09-11_
_Verifier: Claude (gsd-verifier)_
