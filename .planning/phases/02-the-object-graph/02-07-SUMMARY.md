---
phase: 02-the-object-graph
plan: 07
subsystem: test-harness
tags: [vbp-reader, exclusion-rules, differential, ver-02, ver-03, ver-04]
status: complete

requires:
  - crates/deform6/tests/corpus_sweep.rs's corpus_root and walk pattern, from
    phase 1 plan 01-08 (copied, not shared, per D-06)
provides:
  - crates/deform6/tests/support/vbp.rs, a second, independent .vbp reader
    that names nothing from deform6
  - crates/deform6/tests/support/rules.rs, the five written exclusion rules
    VER-04 requires, as data, plus apply_symmetrically for the both-sides
    contract
  - crates/deform6/tests/support/mod.rs, the shared module declaration
  - crates/deform6/tests/support_selftest.rs, 21 tests proving all of the
    above against the real 44-executable, 45-project-file corpus
affects:
  - Plan 02-08, whose differential.rs compares deform6::vb output against
    support::vbp output, both directions, and applies support::rules to
    decide what a mismatch is allowed to drop
  - Plan 02-09, which counts the same exclusions when it pins the recovery
    ratio

tech_stack:
  added: []
  patterns:
    - "A second, independent reader (support/vbp.rs) that imports nothing
      from the crate under test, proved by a non-comment grep for the
      literal 'deform6'"
    - "Rules as data: an identifier, a one-sentence reason and a predicate
      over a generic structural fact (Candidate), never a branch that
      names a program; a grep proves no rule predicate names one"
    - "A generic tally instrument (support::rules::tally) that counts how
      many real candidates each rule matches, so a rule that matches
      nothing fails loudly instead of silently becoming a blanket
      allowance"
    - "apply_symmetrically derives both sides of a comparison from one
      rule evaluation, so a future caller cannot let the declared-side and
      recovered-side exclusion drift apart"

key_files:
  created:
    - crates/deform6/tests/support/mod.rs
    - crates/deform6/tests/support/vbp.rs
    - crates/deform6/tests/support/rules.rs
    - crates/deform6/tests/support_selftest.rs
  modified: []

decisions:
  - "The plan's claim that a 40-line prefix scan for Attribute VB_Name
    'matches none of the 53 forms' is wrong. Measured against the real
    corpus: 2 of 53 (LockWorkStation's FrmLockWorkStation.frm at line 16,
    SK-Gradient-Sample__VB6's Form1.frm at line 38) are short enough for
    the attribute to land inside the bound by chance. The test asserts the
    measured number, 2, not the plan's claimed 0; the trap the test exists
    to prove (a bounded scan cannot be trusted for a form) holds regardless,
    since the scan still fails on 51 of 53."
  - "The plan's claim that Grayscale-effect's pdOpenSaveDialog.cls 'declares
    its two procedures friend' undercounts. Measured: the class declares
    six procedure slots, four Private Declare Function lines (API imports)
    plus two Friend Function members. A Private Declare consumes a
    procedure slot exactly like a Private Sub, per CONTEXT.md's own
    frmFractal measurement, so all six (not two) are non-public and all six
    are excluded by the scope rule. Plain grep (no -a) silently treats this
    Latin-1 file as binary and reports zero matches for 'Friend', which is
    why a hand check with grep alone would have missed both the two real
    Friend lines and the four Declare lines."
  - "support::rules exposes apply_symmetrically(rule, candidate) -> (bool,
    bool), deriving both return values from one rule evaluation. This is
    the mechanism, not just a convention, that keeps the absent-source
    rule's both-sides requirement true by construction for any future
    caller (plan 02-08)."

actuals:
  tokens: 14938
  tasks: 3
  commits: 3
plan_head_before: a2564f95ec161150dc5f958a6ebe279242fdcb55

metrics:
  duration: 1 session
  completed: 2026-09-08
---

# Phase 02 Plan 07: The differential harness support Summary

`crates/deform6/tests/support/vbp.rs` is a second, independent `.vbp`
reader that imports nothing from `deform6`; `crates/deform6/tests/support/rules.rs`
is the five written exclusion rules VER-04 requires, expressed as data
rather than as per-program branches. `crates/deform6/tests/support_selftest.rs`
proves both against the real 44-executable, 45-project-file corpus, in
21 tests, independent of the differential comparison plan 02-08 will
build on top of this.

## What this plan built

| Item | What it gives |
|---|---|
| `support/vbp.rs` | Project file selection (entry-directory scope, then `ExeName32`), quoted-value reading (first quote to last quote), the declared object list (the eight object keys only), and `Attribute VB_Name` resolution (whole-file scan, prefix fallback) |
| `support/rules.rs` | `Visibility`, `Candidate`, `Rule`, the five `RULES`, `tally`, `apply_symmetrically`, and `scan_procedure_visibilities` |
| `support/mod.rs` | Declares `pub mod rules; pub mod vbp;`, loaded via `#[path = "support/mod.rs"]` in every test binary that needs it |
| `support_selftest.rs` | 21 tests, all passing, covering every behaviour named in the plan |

## The selection rule, measured

All 44 corpus executables resolve to exactly one project file once the
search is scoped to the executable's own corpus entry directory before
`ExeName32` is applied. 8 of the 44 needed the key to choose among
siblings: the two under `SK-TFTP-Sample__VB6`, the three under
`machineLanguageConversion`, and the three under `Brightness-effect`. A
corpus-wide index built without that scope collides:
`SK-MCI-Sample__VB6/MCI.VBP` and `SK-Gradient-Sample__VB6/Project1.vbp`
both declare `ExeName32="Project1.exe"`. The repository holds 45
project files and 44 executables (`Brightness-effect/Part 3 -
DIBs/Brightness3.vbp` declares `dibBrightness.exe`, not vendored); the
harness iterates executables, never project files, and a test proves
that unvendored project file is never selected.

## The declared object list, measured

Summed over the 44 selected project files: 105 objects, 53 forms, 8
standard modules, 44 classes, no other kind. The list comes from the
eight object keys the `.vbp` declares and from nothing else; a
directory glob would have counted `Hidden-Markov-model` and
`Randomize-effects`'s orphan `cCommonDialog.cls` sources as recovery
failures, and both are proved absent from the declared list.

An object's name comes from `Attribute VB_Name` inside its referenced
source file, scanned over the whole file. Exactly one object in the
corpus needs the prefix fallback: `Edge-detection/EdgeDetection.vbp`
lists a `cCommonDialog.cls` the repository does not hold. Everywhere
else both the prefix and the attribute are available, they agree, 51
of 51.

## The exclusion rules, measured

| Rule | What it excludes | Measured |
|---|---|---|
| `standard-module` | Every procedure in a standard module | 8 objects, 23 procedure slots (17 Sub/Function/Property + 6 Declare, across the 8 `.bas` files) |
| `scope` | A non-public procedure outside a standard module | Proved on `Grayscale-effect/pdOpenSaveDialog.cls`: 6 slots, all non-public (see correction below) |
| `unlisted-source` | A source file on disk the project file never lists | The two orphan `cCommonDialog.cls` files |
| `absent-source` | An object whose source the repository does not hold | Exactly 1 (`Edge-detection`), excluded from both sides via `apply_symmetrically` |
| `never-kept` | A local variable, a comment, formatting, a private name | A real `Dim` declaration in `Mandelbrot.frm`'s `Form_Load` |

`grep -vE '^\s*//' crates/deform6/tests/support/rules.rs | grep -cE 'Hidden-Markov|Randomize-effects|Edge-detection|Grayscale|corpus/'`
is 0: no rule predicate names a program. `support::rules::tally` is the
generic instrument that proves every rule matches at least one real
candidate; `support_selftest.rs` supplies the real, named cases and
asserts non-zero counts.

## Defects found in the plan

**1. The 40-line prefix scan does not match zero of the 53 forms.** It
matches 2: `LockWorkStation/FrmLockWorkStation.frm` (`Attribute
VB_Name` at line 16) and `SK-Gradient-Sample__VB6/Form1.frm` (line 38),
both short forms with no embedded picture data. The test
(`a_forty_line_prefix_scan_fails_on_most_forms_because_the_attribute_sits_after_the_form_block`)
asserts the measured number, 2, and documents why the plan's claim of
zero does not hold, while still proving the point the trap exists to
make: the bounded scan fails on 51 of 53, which is why the reader scans
the whole file.

**2. `Grayscale-effect`'s `pdOpenSaveDialog.cls` declares six
procedures, not two.** The plan's text says this class "declares its
two procedures friend and ... recovers no name," and that the class's
*recovered names* are none is still true, but the count of *why* is
wrong. Measured: two `Friend Function` members (`GetOpenFileName`,
`GetSaveFileName`) at lines 139 and 257, plus four `Private Declare
Function` lines (`GetOpenFileNameW`, `GetSaveFileNameW`,
`CommDlgExtendedError`, `lstrlenW`) at lines 41-44. Per `CONTEXT.md`'s
own `frmFractal` measurement, a `Private Declare` consumes a procedure
slot exactly like a `Private Sub` does, so this class has six
non-public slots, not two, and the scope rule excludes all six. **Plain
`grep` (no `-a` flag) silently treats this Latin-1-encoded file as
binary and returns zero matches for `"Friend"` or any other substring**
— confirmed by direct comparison (`grep -n "riend"` printed nothing;
`grep -an "riend"` printed both lines). Anyone re-verifying this plan's
claim by hand with a bare `grep` would see zero matches and wrongly
conclude the class has no `Friend` procedures at all, which is the
opposite defect from the plan's undercount. The test
(`the_scope_rule_excludes_every_non_public_procedure_grayscale_effects_dialog_class_declares`)
asserts the measured shape: 6 total, 2 Friend, 4 Private, all excluded.

Neither defect required a scope change or a plan re-read; both are
measurement corrections recorded here because plans 02-08 and 02-09
will reuse these numbers.

## Two disagreements with the inherited documents (per this plan's own output spec)

**The corpus-wide collision on the executable name key** — neither
`ROADMAP.md`, `CONTEXT.md` nor `RESEARCH.md` states that a corpus-wide
index collides; `CONTEXT.md` only states the corrected rule (scope
first, then apply the key). This plan's own measurement
(`a_corpus_wide_index_keyed_on_exe_name_32_alone_collides`) proves the
collision directly: `SK-MCI-Sample__VB6/MCI.VBP` and
`SK-Gradient-Sample__VB6/Project1.vbp` both declare
`ExeName32="Project1.exe"`.

**The one object whose source file the repository does not hold** —
`RESEARCH.md`'s Q2 claims all 105 objects match by the attribute alone
("every one of the 105 objects matches its `.vbp` declaration
one-to-one, using `Attribute VB_Name` as the join key"). That cannot be
true for `Edge-detection`'s `cCommonDialog.cls`, whose source is not in
the repository: the attribute is unreachable for it, and its name comes
from the project line's prefix instead. `exactly_one_object_in_the_corpus_needs_the_fallback_name`
proves the count is exactly 1.

## The selection rule, in three lines (for plan 02-08)

1. Scope the search to the executable's own corpus entry directory (the
   immediate child of `corpus/vb6-code/` or `corpus/public-domain/` that
   holds it) before looking at any project file's `ExeName32`.
2. Inside that scope, select the one project file whose `ExeName32`
   equals the executable's file name, compared case-insensitively.
3. Zero matches and two matches are both loud failures
   (`SelectionError::NoCandidate` / `MultipleCandidates`). A corpus-wide
   index built without the scope collides on this corpus, which is why
   the scope comes first.

## The seven deliberate breakages (task-level requirement: two or three per task)

Every breakage below was made in the working tree, run, observed to
fail, then reverted with a `diff` confirming byte-identical restoration
before the next command ran.

### Task 1 — `support/vbp.rs`'s selection and quoting

**1. Quoting truncated to the next quote instead of the last.**
`the_sepia_title_carries_its_inner_quotes` failed:
```
left: Some("Sepia / ")
right: Some("Sepia / \"Antique\" Image Filter")
```

**2. An absent key returned `Some(String::new())` instead of `None`.**
`an_absent_title_key_is_reported_as_absent_not_as_an_empty_string`
failed:
```
left: Some("")
right: None
```

**3. `select_project_file`'s entry-directory filter replaced with a
no-op (`filter(|_| true)`), removing the scope.**
`every_executable_resolves_to_exactly_one_project_file` failed:
```
2 of 44 executables did not resolve to exactly one project file:
.../SK-Gradient-Sample__VB6/demo/Project1.exe: more than one project file
  declares the executable name ...: .../SK-Gradient-Sample__VB6/Project1.vbp,
  .../SK-MCI-Sample__VB6/MCI.VBP
.../SK-MCI-Sample__VB6/demo/Project1.exe: more than one project file
  declares the executable name ...: (same two candidates)
```
Both `Project1.exe` executables become ambiguous rather than one
silently resolving to the other's project — a louder failure mode than
a silent misdirection, and it still demonstrates that removing the
scope breaks resolution.

### Task 2 — `support/vbp.rs`'s declared object list and name resolution

**4. The object name taken from the file name (extension removed)
instead of the attribute scan.**
`a_forms_object_name_is_the_attribute_not_the_file_name` failed:
```
left: Some("Mandelbrot")
right: Some("frmFractal")
```
(This test was added during this task specifically to catch this
breakage; the aggregate total/count tests do not change under this
breakage, because a filename-derived name still resolves to `Some`.)

**5. The fallback removed (`None => (None, None)` for every
unresolved name).**
`exactly_one_object_in_the_corpus_needs_the_fallback_name` failed:
```
left: 0
right: 1
```

### Task 3 — `support/rules.rs`'s rules and both-sides contract

**6. A sixth rule added with a predicate that always returns
`false`.**
`five_rules_exist_each_with_an_identifier_and_a_reason` failed:
```
left: 6
right: 5
```
`every_rule_matches_at_least_one_real_corpus_case` also failed and
named it:
```
the following rules matched zero real corpus cases:
  ["deliberately-unreachable"]
```

**7. `apply_symmetrically` changed to return `(excluded, false)`
instead of `(excluded, excluded)`.**
`the_absent_source_rule_excludes_from_both_sides_and_never_lets_recovered_exceed_declared`
failed:
```
left: 4
right: 3
```
Exactly the shape the rule exists to prevent: the recovered count (4)
exceeded the declared count (3) once the exclusion was applied to one
side only.

## The gate

Run after each commit, on a clean tree, and again after all three
commits:

| Command | Result |
|---|---|
| `cargo fmt --all --check` | 0, no output |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test -p deform6 --test support_selftest` | 0, 21 tests pass |
| `cargo test --workspace` | 0, 21 + 9 (cli) + 0 (doc-tests) pass |
| `sh scripts/prove-lint-wall.sh` | 0, "The wall stops every bad shape, and the tree it leaves behind is clean." |
| `sh scripts/prove-region-wall.sh` | 0, "The type refuses every shape, and the tree it leaves behind is clean." |
| `grep -vE '^\s*//' crates/deform6/tests/support/vbp.rs \| grep -c 'deform6'` | 0 |
| `grep -vE '^\s*//' crates/deform6/tests/support/rules.rs \| grep -cE 'Hidden-Markov\|Randomize-effects\|Edge-detection\|Grayscale\|corpus/'` | 0 |

## Threat mitigations

| Threat | State |
|---|---|
| T-02-31, `support/vbp.rs` sharing code with the library | Mitigated and measured. The grep above is 0; deliberate breakage 3 shows what happens when the harness's own scoping logic is wrong, independent of anything in `src/`. |
| T-02-32, project file selection by name key alone | Mitigated and measured. `a_corpus_wide_index_keyed_on_exe_name_32_alone_collides` proves the collision; breakage 3 shows the harness cannot silently misresolve once scoped. |
| T-02-33, an exclusion rule that matches nothing | Mitigated and measured. `support::rules::tally` plus `every_rule_matches_at_least_one_real_corpus_case`; breakage 6 shows a zero-match rule is caught and named. |
| T-02-34, an exclusion applied to one side only | Mitigated and measured. `apply_symmetrically` derives both sides from one evaluation; breakage 7 shows the recovered-exceeds-declared failure mode when that guarantee is removed. |
| T-02-35, the harness reading corpus files | Accepted, as the plan's threat model states: a vendored, fixed corpus this repository controls, not hostile input. |

## Known Stubs

None. `differential.rs` (plan 02-08) and `ratios.toml`/`xtask`
(plan 02-09) are out of scope for this plan by design, not stubbed
here.

## Deferred Issues

None new.

## Threat Flags

None. This plan only reads files under `tests/`; it introduces no new
network endpoint, auth path, or schema change at a trust boundary.

## Commits

| Commit | Subject |
|---|---|
| `5f65812` | Add an independent project file reader to the test harness |
| `9b200c2` | Build the declared object list from the project file only |
| `15803bb` | Add the written exclusion rules and prove each one matches a real case |

`git rev-list --count a2564f9..HEAD` is 3, measured before this
SUMMARY was written.

## Self-Check: PASSED

`crates/deform6/tests/support/mod.rs` exists, 10 lines.
`crates/deform6/tests/support/vbp.rs` exists, 451 lines.
`crates/deform6/tests/support/rules.rs` exists, 272 lines.
`crates/deform6/tests/support_selftest.rs` exists, 800 lines.
All three commit hashes resolve in the history of this branch.
`cargo test -p deform6 --test support_selftest` exits 0 with 21 tests
passing. `cargo test --workspace`, both proof scripts, and both grep
gates all exit 0.
