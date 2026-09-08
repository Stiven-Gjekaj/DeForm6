---
phase: 02-the-object-graph
verified: 2026-09-08T00:00:00Z
status: human_needed
score: 11/11 must-haves verified
behavior_unverified: 0
overrides_applied: 0
covered_files: [".planning/REQUIREMENTS.md", ".planning/ROADMAP.md", ".planning/phases/02-the-object-graph/02-01-PLAN.md", ".planning/phases/02-the-object-graph/02-01-SUMMARY.md", ".planning/phases/02-the-object-graph/02-02-PLAN.md", ".planning/phases/02-the-object-graph/02-02-SUMMARY.md", ".planning/phases/02-the-object-graph/02-03-PLAN.md", ".planning/phases/02-the-object-graph/02-03-SUMMARY.md", ".planning/phases/02-the-object-graph/02-04-PLAN.md", ".planning/phases/02-the-object-graph/02-04-SUMMARY.md", ".planning/phases/02-the-object-graph/02-05-PLAN.md", ".planning/phases/02-the-object-graph/02-05-SUMMARY.md", ".planning/phases/02-the-object-graph/02-06-PLAN.md", ".planning/phases/02-the-object-graph/02-06-SUMMARY.md", ".planning/phases/02-the-object-graph/02-07-PLAN.md", ".planning/phases/02-the-object-graph/02-07-SUMMARY.md", ".planning/phases/02-the-object-graph/02-08-PLAN.md", ".planning/phases/02-the-object-graph/02-08-SUMMARY.md", ".planning/phases/02-the-object-graph/02-09-PLAN.md", ".planning/phases/02-the-object-graph/02-09-SUMMARY.md", ".planning/phases/02-the-object-graph/02-10-PLAN.md", ".planning/phases/02-the-object-graph/02-10-SUMMARY.md", ".planning/phases/02-the-object-graph/CONTEXT.md", "crates/deform6-cli/src/main.rs", "crates/deform6/src/error.rs", "crates/deform6/src/vb/classify.rs", "crates/deform6/src/vb/functyp.rs", "crates/deform6/src/vb/mod.rs", "crates/deform6/src/vb/object.rs", "crates/deform6/src/vb/privateobj.rs", "crates/deform6/src/vb/project.rs", "crates/deform6/tests/differential.rs", "crates/deform6/tests/ratios.rs", "crates/deform6/tests/support/mod.rs", "crates/deform6/tests/support/rules.rs", "crates/deform6/tests/support/source.rs", "crates/deform6/tests/support/vbp.rs", "crates/deform6/tests/support_selftest.rs", "crates/deform6/tests/type_descriptors.rs", "crates/xtask/Cargo.toml", "crates/xtask/src/main.rs", "tests/ratios.toml"]
covered_digest: "v1:sha256:463b861f7a38d575c07abe30dcfa820ee1670a700a372d42b687d9d1f9d6cd77"
human_verification:
  - test: "Break vb::privateobj's is_plausible_identifier character-shape check for real (starts-with-letter/underscore, all-alphanumeric-or-underscore), run cargo test --workspace, confirm nothing catches it, then decide whether phase 2 needs a synthetic unit test before phase 3 relies on this validation."
    expected: "Either a new test is added that fails when the character-shape rule is weakened, or the team accepts the corpus-measured fact (every one of the 428 unresolvable entries fails at address resolution, none at the character-shape step) as sufficient justification to leave it unexercised, same as the accepted-risk treatment already given to ParamArray and the 15 unassigned type codes."
    why_human: "This is a design/priority call (add a test vs. accept the documented risk), not a mechanical pass/fail the verifier can decide unilaterally — reported in Anti-Patterns and Gaps Summary below with full reproduction steps."
---

# Phase 2: The object graph Verification Report

**Phase Goal:** `deform6 inspect` reports every object the program holds, says
whether each is a form, a module or a class, names every public procedure,
prints each prototype with its argument names, types and modifiers, and
prints the `Declare` statements for external API calls. Nothing is written to
disk. A differential test measures all of it against the original source.

**Verified:** 2026-09-08
**Status:** PASS WITH CONCERNS (see finding #1 below — routed as a human
decision item, not a blocker; it does not touch the goal's observable truths)
**Re-verification:** No — initial verification

## Overall verdict: PASS WITH CONCERNS

All five roadmap success criteria hold on real, executed commands (not
SUMMARY narration). All eleven requirements (OBJ-01..06, VER-01..05) are
genuinely delivered, each with named code and a named, run test. The
two-directional object and procedure comparisons, the ratio pin's two failure
messages, the type-code decode, and the `Declare` filter were each broken for
real in the working tree and each broke the correct test; every breakage was
reverted and the tree confirmed clean. The subset trap is closed: a real
capacity-vs-count breakage in `object.rs` produced exactly the
over-count-hides-behind-a-subset-check failure the risk register describes,
caught by the two-directional check. Harness independence is real: zero
non-comment references to `deform6` in `support/vbp.rs` and `support/source.rs`,
confirmed by grep and by reading both files' plain-`std` implementations.
The honesty audit passes: every one of the five named unreachable items
(`ParamArray`, the 15 unassigned type codes, event descriptors, `cntPublicVars`,
the standard-module cap) is reported where a CLI reader will see it, at the
severity the evidence supports, and the tool visibly distinguishes "not in
this file" (the `Declare` markers, backed by `STRUCTURES.md` §7.2) from "the
file holds it and the tool could not read it" (a `Defect` on `Report.defects`,
printed for the one gap category — unrecoverable prototypes — the CLI's
Gaps section currently surfaces from that channel).

One finding downgrades this from a clean PASS: `vb::privateobj::is_plausible_identifier`'s
character-shape rule (the third of the three validation facts a non-null
`lpProcNamesArray` entry must pass before it is trusted as a name) has zero
test coverage anywhere in the 307-test suite. Weakening it to accept any
non-empty byte string produces **zero test failures**, workspace-wide. This
is not a hypothetical: I made the change, ran the full suite, watched it stay
green, and reverted it. It is the same species of defect the task brief asked
me to hunt for, and it is real. It does not fail any of the five success
criteria (the corpus never reaches this code path — see finding #1's detail
below for why), so it is reported as a human-verification item rather than a
blocker, but it should not be waved through silently.

## Goal Achievement

### The five success criteria, each run for real

| # | Criterion | Command run | Result |
|---|-----------|-------------|--------|
| 1 | `cargo test --workspace` runs `differential.rs` over all 44 corpus programs; every declared object recovered by name and kind, 44/44 | `cargo test -p deform6 --test differential -- --nocapture` | PASS. `every_declared_object_is_recovered_and_every_recovered_object_is_declared_across_the_corpus` passes; 105/105 objects across 44 programs, both directions. |
| 2 | Expectation from the `.vbp` file list, never a directory glob; correct `.vbp` selected by `ExeName32` where several exist | `cargo test -p deform6 --test support_selftest` (27 tests) | PASS. `neither_orphan_common_dialog_class_appears_in_its_projects_declared_list` and `a_corpus_wide_index_keyed_on_exe_name_32_alone_collides` both pass; the two `Project1.exe` siblings (`SK-MCI-Sample__VB6`, `SK-Gradient-Sample__VB6`) resolve correctly only once the entry-directory scope is applied before the key. |
| 3 | `ratios.toml` pins procedures; editing a number **up** fails `REGRESSION`, editing **down** fails `MOVED UP` with the paste block | Hand-edited `tests/ratios.toml`'s `Grayscale.exe` entry live, both directions, reverted | PASS — see "Criterion 3, verbatim" below. Object recovery is asserted as an equality (105/105), never a ratio, as the corrected criterion text requires. |
| 4 | `inspect` prints a public prototype with argument names, types, and `ByRef`/`Array`/`Optional`/`ParamArray` modifiers; a private procedure prints `Private` with no name | `cargo run -p deform6-cli -- inspect corpus/vb6-code/Grayscale-effect/Grayscale.exe` | PASS — see "Criterion 4/5, verbatim" below. `ParamArray` never appears (zero corpus occurrences, correctly reported as a gap, not fabricated). |
| 5 | `inspect` prints one line per `Declare` import table entry (library + export), and marks — rather than silently omits — the alias and argument list the file does not hold | Same run as above | PASS. Every `Declarations` line carries all three explicit missing-part markers. |

#### Criterion 3, verbatim (real edits, both directions, reverted)

Editing `Grayscale.exe`'s pinned `recovered` **up** (12 → 13):

```
vb6-code/Grayscale-effect/Grayscale.exe: REGRESSION: the pin claims 13 recovered, the tool recovers 12. Procedures the source declares that the tool did not name: nothing the source declares is unrecovered; the tool named every public procedure this program's source declares, so this pin disagrees with a tool that lost nothing
```

Editing it **down** (12 → 11):

```
vb6-code/Grayscale-effect/Grayscale.exe: MOVED UP: the pin claims 11 recovered, the tool recovers 12. Paste this block into tests/ratios.toml:
["vb6-code/Grayscale-effect/Grayscale.exe"]
recovered = 12
declared = 34
ratio = 0.35
```

Both edits were reverted with `git diff --exit-code -- tests/ratios.toml`
confirmed clean afterward. The pairing matches the corrected direction stated
in `ROADMAP.md` and `CONTEXT.md`: "up" means the pin claims more than the
tool measures (something is missing → `REGRESSION`); "down" means the tool
measures more than the pin claims (the tool moved up → `MOVED UP`).

#### Criterion 4/5, verbatim (real run, `Grayscale.exe`)

```
Object graph
  frmGrayscale  (form)
    private
    ...
    ByteMeL(tempVar As Long) As Byte
    ...
    DrawGrayscaleDecompose(ByRef srcPic As Object (an external COM object, unresolved, raw address 0x00403fb8), ByRef dstPic As Object (an external COM object, unresolved, raw address 0x00403fb8), Optional minValue As Boolean = true)
  pdOpenSaveDialog  (class)
    private
    private
    private
    private
    private
    private
  ...

Declarations
  gdi32!StretchDIBits
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
  ...
```

Byte-for-byte matches the SUMMARY's claimed output. Private procedures print
`private` with no name and no index (OBJ-06). `Optional` and `ByRef` modifiers
appear (`Optional minValue As Boolean = true`); `ParamArray` never appears
anywhere in the corpus and is correctly never printed. Exit code 0; a
directory listing taken before and after the run is identical (confirmed with
`git status --short`, no changes).

### Requirements Coverage

| Requirement | Code | Test | Status |
|---|---|---|---|
| OBJ-01 (walk object table, recover every name) | `vb/object.rs::ObjectTable::walk`, bounded by `wTotalObjects` | `differential.rs::every_declared_object_is_recovered_and_every_recovered_object_is_declared_across_the_corpus` (105/105, run live) | SATISFIED |
| OBJ-02 (form/module/class apart) | `vb/classify.rs::classify` | `classify.rs` unit tests (9); differential's kind-total check (53/8/44) | SATISFIED |
| OBJ-03 (public procedure names) | `vb/privateobj.rs::ProcedureList::read` | `differential.rs::every_declared_public_procedure_is_recovered_and_every_recovered_one_is_declared_across_the_corpus` (185/185, run live) | SATISFIED |
| OBJ-04 (prototypes: args, types, modifiers) | `vb/functyp.rs::FuncTypeWalk::read`, `walk_type_buffer`, `walk_optional_vals` | `functyp.rs` unit tests (20); `tests/type_descriptors.rs` (193/193 corpus-wide records close) | SATISFIED |
| OBJ-05 (`Declare` statements) | `vb/project.rs::DeclareTable::read` | `project.rs` unit tests incl. `eight_of_nine_grayscale_entries_are_external_and_the_ninth_is_skipped` (broken and re-caught live) | SATISFIED |
| OBJ-06 (private reported as private, no invented name) | `vb/privateobj.rs::Procedure::Private`, `is_plausible_identifier` | `mandelbrot_frm_fractal_gives_nine_slots_all_private_and_uninitialised`; CLI shows `private` with no name | SATISFIED (see finding #1 on the untested third validation fact) |
| VER-01 (differential decompile-vs-source) | `tests/differential.rs` | Run live, 7/7 pass | SATISFIED |
| VER-02 (expectation from `.vbp` list, not a glob) | `support/vbp.rs::declared_objects` | `neither_orphan_common_dialog_class_appears_in_its_projects_declared_list` | SATISFIED |
| VER-03 (`.vbp` selection by `ExeName32`, scoped) | `support/vbp.rs::select_project_file` | `a_corpus_wide_index_keyed_on_exe_name_32_alone_collides`; `every_executable_resolves_to_exactly_one_project_file` | SATISFIED |
| VER-04 (exclusion rules as written data, not per-program) | `support/rules.rs::RULES`, `tally` | `every_rule_matches_at_least_one_real_corpus_case`; grep-proven zero program names in rule predicates | SATISFIED |
| VER-05 (recovery ratio pinned, two directional failures) | `tests/ratios.toml`, `crates/deform6/tests/ratios.rs`, `crates/xtask` | Both directions edited and reverted live (above); `cargo run -p xtask -- update-ratios` idempotent on the committed file | SATISFIED |

No orphaned requirements: `REQUIREMENTS.md`'s traceability table maps exactly
OBJ-01..06 and VER-01..05 to Phase 2, matching every plan's `requirements:`
frontmatter with no leftover.

### Hunt for tests that cannot fail — five targeted breakages, all run live

| Target | What was changed | Result | Reverted clean? |
|---|---|---|---|
| Subset trap (object over-count) | `object.rs::ObjectTable::walk` loop bound: `w_total_objects` → `w_compiled_objects` | **Caught.** 15/44 programs disagree, `recovered-but-not-declared` non-empty (e.g. `LockWorkStation.exe`: `[""]`) — the exact "extra capacity slots leak through" failure the risk register describes | Yes, `git diff --exit-code` clean |
| Two-directional object/procedure comparison | `differential.rs::two_directional_diff`: `recovered_not_declared` forced to `Vec::new()` | **Caught**, twice: `fabricated_object_passes_a_subset_check_and_fails_the_two_directional_check` and `a_fabricated_procedure_name_passes_a_subset_check_and_fails_the_two_directional_check` both fail | Yes |
| Ratio pin (both directions) | `tests/ratios.toml`'s `Grayscale.exe` entry, `recovered` +1 and −1 | **Caught** both ways — see Criterion 3 above | Yes |
| Type-code decode | `functyp.rs::vb_type_of`: `0x03 => VbType::Boolean` changed to `VbType::Byte` | **Caught.** `every_documented_type_code_maps_to_its_name` fails | Yes |
| `Declare` filter (internal vs. external) | `project.rs`: `6 => {}` merged into `7 => match ...` (internal entries now dereferenced as external) | **Caught.** `eight_of_nine_grayscale_entries_are_external_and_the_ninth_is_skipped` fails, printing garbage bytes as a library/export pair (the exact information-disclosure shape the threat model names) | Yes |

Four of five behave exactly as the SUMMARYs claim. **The fifth attempt found
a real gap**, not documented in any phase-2 SUMMARY:

**Finding #1 — `is_plausible_identifier`'s character-shape rule is untested,
and a full breakage produces zero failures.**

`vb/privateobj.rs::is_plausible_identifier` requires three independent facts
before a non-null `lpProcNamesArray` entry is trusted as `Public`: it must
resolve inside a mapped section, be NUL-terminated within 64 bytes, and pass
the character-shape test (leading letter/underscore, then alphanumeric or
underscore only). I replaced the function body with `!bytes.is_empty()`
(accepting any non-empty byte sequence) and ran `cargo test -p deform6 --lib
vb::privateobj` and `cargo test -p deform6 --test differential`: **23/23 and
7/7 tests still passed**, unchanged. `grep -rln is_plausible_identifier
crates/deform6/src crates/deform6/tests` returns only the one file that
defines and calls it — no test, unit or integration, ever calls it directly
or constructs a fixture that reaches it (resolves + NUL-terminates but fails
the character shape).

The reason this is not caught on real data matches what plan 02-03 and
02-08's SUMMARYs already measured: all 428 corpus-wide unresolvable entries
fail at the *first* fact (address resolution), and *zero* fail at NUL-
termination or the character-shape step. So on the real 44-program corpus
this breakage is inert by construction — which is exactly the "cannot fail"
shape `AGENTS.md` warns is worse than no test at all, because the code
exists, is reachable in principle (a hostile or unusual file could resolve
to mapped bytes that are NUL-terminated garbage failing only the character
check), and nothing proves it does what its own doc comment claims.

This is the same class of finding the executors and 02-08's verifier found
elsewhere in this phase (e.g. the event-descriptor walk, `ParamArray`), but
those were explicitly named and accepted as risks in `CONTEXT.md` and the
SUMMARYs. This one was not named anywhere. It does not fail SC1-SC5 (the
corpus never exercises the branch, so no wrong output is possible today), so
I am not marking it a blocker — but it is a real, reproducible, previously
unreported gap and belongs in front of a human before Phase 3 builds a
structurally similar validation (the event-name heuristic, D-09) on the same
pattern.

### Data-Flow / Wiring — the "not written to disk" claim

```
$ git status --short   # before and after several inspect runs
?? Notes/               # pre-existing, untouched by this verification
```

No new or modified files after `cargo run -p deform6-cli -- inspect ...` on
three different corpus executables. SC1 confirmed for real.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `crates/deform6/src/vb/privateobj.rs` | 643 | `is_plausible_identifier` — reachable, non-stub, but zero test coverage of its own decision boundary | ⚠️ Warning | See Finding #1 above |
| (workspace-wide) | — | `TBD`/`FIXME`/`XXX` grep | — | Zero matches in any phase-2 file. No debt-marker gate issue. |
| `crates/deform6-cli/src/main.rs` (defects loop, line ~413) | 413-419 | The Gaps section prints only defects whose `site.structure` is `"FuncTypDesc"` or `"PrivateObj"`/`"lpFuncTypeInfo"`; other defect kinds (an unmapped object-name pointer, an undocumented `dwEntryType`) reach `Report.defects` but have no dedicated text line in `inspect`'s output today | ℹ️ Info | None of these paths are exercised by any of the 44 corpus files (all 105 objects and all 249 Declare entries resolve cleanly per the differential run), so this is a latent reporting gap, not an observed dishonesty — flagged for awareness only, not a phase-2 blocker |
| `crates/deform6/src/error.rs` + several `vb/*.rs` modules | — | `DefectKind::UnmappedAddress` is hard-coded `Severity::Fatal` but is reused by `object.rs`/`project.rs` for genuinely-recoverable leaf-pointer failures; `Journal::record` consults `.severity()` but `inspect`'s composition path never calls `Journal`, so the mismatch is dormant today | ℹ️ Info | Already self-reported honestly in the 02-01 and 02-06 SUMMARYs as a cross-cutting gap for a future phase to close before any `Journal`-based caller is added; confirmed still open, confirmed still dormant (`Journal` is not invoked anywhere in the `inspect` path) |

### Honesty Audit

| Item | Reported where a reader sees it | Verified |
|---|---|---|
| `ParamArray` (zero corpus occurrences, encoding unknown) | Named only in a doc comment in `functyp.rs`; never emitted as a modifier or type. `grep -vE '^\s*//' .../functyp.rs \| grep -c ParamArray` → `0` | Confirmed live |
| 15 unassigned type codes (zero occurrences in 193 records) | `VbType::Unknown(u8)`; CLI prints `"{name}"'s argument "{name}" has an unrecognised type code {code:#04x}` when hit | Confirmed via code read; no corpus file triggers it (matches 193/193 closing cleanly) |
| Event descriptors (`cntEvents` 0 in 97/97) | `no_corpus_program_in_this_module_carries_an_event_descriptor` test names the file/object the day this changes; walk is implemented and proven only on synthetic fixtures | Confirmed via code read and test run |
| `cntPublicVars` (does not count public variables; a class declaring 0 reports 4, another declaring 17 reports 23) | CLI Gaps section: `"{name}": cntPublicVars is {count}, and its meaning is unresolved` — never presented as a count | Confirmed live on `Mandelbrot.exe` (`frmFractal`: 9) and `Grayscale.exe` (`pdOpenSaveDialog`: 4) |
| Standard-module cap (a `.bas` module's whole name-array pointer is 0) | CLI Gaps section states the cap unconditionally (even at zero); each capped module prints `"{N} procedure(s) declared; their names are not reachable through this structure"` instead of an empty list | Confirmed live on `Map Editor.exe`: `Declaration_Module` → "1 procedure(s) declared...", `Sub_Module` → "7 procedure(s) declared..." |
| Distinguishes "not in this file" vs. "not recovered" | `Declaration::NAME_MARKER`/`ARGUMENTS_MARKER`/`SCOPE_MARKER` are static, `STRUCTURES.md`-§7.2-backed claims about what never survives compilation (always printed); a descriptor that fails to resolve produces a `Defect` on `Report.defects` instead (a different code path) | Confirmed by reading both code paths; the two never merge |

No claim found that overstates what the corpus proves. Every named-gap
sentence in the CLI output is backed by a measured number, and every number I
spot-checked against a live run matched what the SUMMARY claimed.

### Harness Independence

```
$ grep -vE '^\s*//' crates/deform6/tests/support/vbp.rs | grep -c 'deform6'
0
$ grep -vE '^\s*//' crates/deform6/tests/support/source.rs | grep -c 'deform6'
0
```

Both files import only `std::{collections, path}`; both parse raw bytes/text
with hand-rolled logic (quoted-string scanning, `Attribute VB_Name` regex-free
text search, `Sub`/`Function`/`Property` keyword matching with continuation-
line skipping) — genuinely independent re-derivation, not delegation to a
subprocess or a shared type. Confirmed by reading both files in full.

### Gaps Summary

No gap blocks the phase goal. One finding (`is_plausible_identifier`'s
untested character-shape branch) is real, reproducible, and not previously
disclosed anywhere in the phase's own documentation — it is routed to human
verification above rather than treated as a blocker, because it cannot
produce a wrong result on any of the 44 corpus files today (the branch is
provably unreached by every one of the 428 unresolved entries) and closing
it is a scope/priority decision (add a targeted unit test vs. accept the
same "measured-absent, not hidden" treatment already given to sibling gaps
in this phase), not a mechanical failure.

---

*Verified: 2026-09-08*
*Verifier: Claude (gsd-verifier)*
