---
phase: 02-the-object-graph
verified: 2026-09-11T00:00:00Z
status: passed
score: "11/11 requirements confirmed; 5/5 roadmap success criteria confirmed"
covered_files: [".planning/REQUIREMENTS.md", ".planning/ROADMAP.md", ".planning/WINDOWS.md", ".planning/phases/02-the-object-graph/02-01-PLAN.md", ".planning/phases/02-the-object-graph/02-01-SUMMARY.md", ".planning/phases/02-the-object-graph/02-02-PLAN.md", ".planning/phases/02-the-object-graph/02-02-SUMMARY.md", ".planning/phases/02-the-object-graph/02-03-PLAN.md", ".planning/phases/02-the-object-graph/02-03-SUMMARY.md", ".planning/phases/02-the-object-graph/02-04-PLAN.md", ".planning/phases/02-the-object-graph/02-04-SUMMARY.md", ".planning/phases/02-the-object-graph/02-05-PLAN.md", ".planning/phases/02-the-object-graph/02-05-SUMMARY.md", ".planning/phases/02-the-object-graph/02-06-PLAN.md", ".planning/phases/02-the-object-graph/02-06-SUMMARY.md", ".planning/phases/02-the-object-graph/02-07-PLAN.md", ".planning/phases/02-the-object-graph/02-07-SUMMARY.md", ".planning/phases/02-the-object-graph/02-08-PLAN.md", ".planning/phases/02-the-object-graph/02-08-SUMMARY.md", ".planning/phases/02-the-object-graph/02-09-PLAN.md", ".planning/phases/02-the-object-graph/02-09-SUMMARY.md", ".planning/phases/02-the-object-graph/02-10-PLAN.md", ".planning/phases/02-the-object-graph/02-10-SUMMARY.md", ".planning/phases/02-the-object-graph/CONTEXT.md", ".planning/phases/02-the-object-graph/GAPS.md", ".planning/phases/02-the-object-graph/RESEARCH.md", ".planning/phases/02-the-object-graph/VERIFICATION.md", "crates/deform6-cli/src/main.rs", "crates/deform6/src/error.rs", "crates/deform6/src/vb/classify.rs", "crates/deform6/src/vb/functyp.rs", "crates/deform6/src/vb/mod.rs", "crates/deform6/src/vb/object.rs", "crates/deform6/src/vb/privateobj.rs", "crates/deform6/src/vb/project.rs", "crates/deform6/tests/differential.rs", "crates/deform6/tests/ratios.rs", "crates/deform6/tests/support/mod.rs", "crates/deform6/tests/support/rules.rs", "crates/deform6/tests/support/source.rs", "crates/deform6/tests/support/vbp.rs", "crates/deform6/tests/support_selftest.rs", "crates/deform6/tests/type_descriptors.rs", "crates/xtask/Cargo.toml", "crates/xtask/src/main.rs", "tests/ratios.toml"]
covered_digest: "v1:sha256:4d7fbc021e135311ce4117fd000f3b7962ebc1aa66fb17bd5d78b6be269387a2"
behavior_unverified: 0
overrides_applied: 0
human_verification: []
---

# Phase 2: The object graph Verification Report

**Phase Goal:** `deform6 inspect` reports every object the program holds, says
whether each one is a form, a module or a class, names every public
procedure, prints each prototype with its argument names, types and
modifiers, and prints the `Declare` statements for external API calls.
Nothing is written to disk. A differential test measures all of it against
the original source.

**Verified:** 2026-09-11
**Status:** passed
**Re-verification:** No — this is the first verification filed under the
`02-VERIFICATION.md` name the tooling looks for. An earlier report exists at
`.planning/phases/02-the-object-graph/VERIFICATION.md` (dated 2026-09-08,
filed without the phase-number prefix, so `gsd_run query verification.status`
has never been able to find it). This report treats that file as prior
evidence, re-checks every one of its live claims against the code as it
stands today, and does not take any of its numbers on trust.

## Why this report exists

`git log` shows the phase closed by hand: commit `5a8c346` ("Mark phase 2
complete in the project state") touched only `.planning/STATE.md`. It never
ticked the `ROADMAP.md` Phase 2 checkbox and it produced no correctly-named
verification report. `ROADMAP.md`'s own Phase 2 heading still reads `[ ]`
and its own Progress table still reads "4/10" plans complete, even though
all ten `02-*-SUMMARY.md` files exist and every one reads `status: complete`.
Neither of those two numbers has been true since 2026-09-08. Fixing them is
outside this report's scope (it edits `REQUIREMENTS.md` only, per the task
brief) but both are flagged here so the human closing this phase sees them.

## Full Gate (run live, not trusted from any SUMMARY)

| Check | Command | Result |
| ----- | ------- | ------ |
| Format | `cargo fmt --all --check` | PASS (exit 0, no output) |
| Lint | `cargo clippy --all-targets -- -D warnings` | PASS (finished clean) |
| Tests | `cargo test --workspace` | PASS — every binary green: lib 411, blobs 4, corpus_sweep 2, differential 26, events 2, form_tracer 1, ratios 40, refusal 9, support_selftest 44, type_descriptors 2, cli 29, xtask 52 (0 failed anywhere) |

Run live in this session. `git log --since="2026-09-08"` over the phase's
own source and test files shows only Phase 3 commits touching
`tests/ratios.toml` (re-pinning the form/control keys 02-09 introduced) and
`format_ratio` (a zero-denominator guard, 03-17) — no Phase 2 requirement's
own code changed since the 2026-09-08 report, and the full gate confirms
nothing regressed.

## Goal Achievement

### The five roadmap success criteria, each run for real this session

| # | Criterion | Command run | Result |
|---|-----------|-------------|--------|
| 1 | `differential.rs` recovers every declared object by name and kind, 44/44 | `cargo test -p deform6 --test differential every_declared_object_is_recovered -- --nocapture` | PASS |
| 2 | Expectation from the `.vbp` file list, never a directory glob; correct `.vbp` selected by `ExeName32` where several exist | Read `Project::declared_objects` (`crates/deform6/tests/support/vbp.rs:412-449`) — walks the eight object keys in the `.vbp` text only, joins by path, never lists a directory; `select_project_file` (`vbp.rs:255`) scopes to the executable's own directory before matching `ExeName32`. `cargo test -p deform6 --test support_selftest` | PASS — 44/44. `neither_orphan_common_dialog_class_appears_in_its_projects_declared_list` and `a_corpus_wide_index_keyed_on_exe_name_32_alone_collides` both pass. |
| 3 | `ratios.toml` pins procedures; editing a number up fails `REGRESSION`, editing down fails `MOVED UP` with the paste block | Hand-edited `tests/ratios.toml`'s `Grayscale.exe` entry live, both directions, `cargo test -p deform6 --test ratios`, reverted | PASS both directions — see verbatim output below. Object recovery is asserted as a two-directional equality in `differential.rs`, never a ratio, matching the corrected criterion text. |
| 4 | `inspect` prints a public prototype with argument names, types, `ByRef`/`Array`/`Optional`/`ParamArray`; a private procedure prints `Private` with no name | `./target/debug/deform6 inspect corpus/vb6-code/Grayscale-effect/Grayscale.exe` and `.../Custom-image-filters/Custom_Filters.exe` | PASS — see verbatim output below. `ParamArray` never appears; zero corpus occurrences, confirmed by a fresh grep this session, correctly reported as an unreachable gap rather than fabricated. |
| 5 | `inspect` prints one line per `Declare` import entry (library + export name), marking rather than omitting the alias and argument list | Same run | PASS — every `Declarations` line carries all three explicit missing-part markers. |

#### Criterion 3, verbatim (re-run live this session, both directions, reverted)

Up (`recovered` 12 → 13):

```
vb6-code/Grayscale-effect/Grayscale.exe: REGRESSION: the pin claims 13
recovered, the tool recovers 12. Procedures the source declares that the
tool did not name: nothing the source declares is unrecovered; the tool
named every public procedure this program's source declares, so this pin
disagrees with a tool that lost nothing
```

Down (`recovered` 12 → 11):

```
vb6-code/Grayscale-effect/Grayscale.exe: MOVED UP: the pin claims 11
recovered, the tool recovers 12. Paste this block into tests/ratios.toml:
```

Both edits reverted from a saved copy; `git status --short -- tests/ratios.toml`
confirmed clean, and `cargo test -p deform6 --test ratios` passed 40/40
afterward.

#### Criteria 4/5, verbatim (real run, `Grayscale.exe`)

```
Object graph
  frmGrayscale  (form)
    private
    ByteMeL(tempVar As Long) As Byte
    DrawGrayscaleDecompose(ByRef srcPic As Object (an external COM object,
    unresolved, raw address 0x00403fb8), ByRef dstPic As Object (an external
    COM object, unresolved, raw address 0x00403fb8), Optional minValue As
    Boolean = true)
  pdOpenSaveDialog  (class)
    private
    private

Declarations
  gdi32!StretchDIBits
    the Visual Basic procedure name and its Alias are not in this file;
    only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are
    not in this file
```

`Custom_Filters.exe` additionally confirms the `Array` modifier live:
`GetImageData2D(ByRef srcPictureBox As Object ..., ByRef dstPixelData() As
Byte, Optional fixOrientation As Boolean = false)`.

### Requirement-by-requirement adjudication

The task brief asked for CONFIRMED or OVERSTATED on all eleven, against the
code as it stands, with particular attention to OBJ-05 by analogy with
Phase 3's `frx::extract_blob` (20 passing unit tests, zero production call
sites) and `handler_address` (computed, unit-tested, reaching no report and
no CLI output) defects.

| # | Requirement | Verdict | Evidence |
|---|---|---|---|
| OBJ-01 | Walks the object table, recovers every name | **CONFIRMED** | `vb/object.rs::ObjectTable::walk`; live `inspect` on `Grayscale.exe` names `frmGrayscale`, `pdOpenSaveDialog`, `FastDrawing`; `differential.rs` — 105/105 objects, both directions, run live this session |
| OBJ-02 | Tells a form, a module and a class apart | **CONFIRMED** | `vb/classify.rs::classify`; live `inspect` on `Map Editor.exe` prints `Main (form)`, `Declaration_Module (module)`, `Sub_Module (module)`, `pdOpenSaveDialog (class)` in the same run — all three kinds distinguished on one file |
| OBJ-03 | Public procedure names for every object | **CONFIRMED** | `vb/privateobj.rs::ProcedureList::read`; live output above names `ByteMeL`, `DrawGrayscaleDecompose`, and others by name; `differential.rs` — 185/185 procedures, both directions |
| OBJ-04 | Prototypes with argument names, types, `ByRef`/`Array`/`Optional`/`ParamArray` | **CONFIRMED, with the honest cap the roadmap itself names** | Live output shows `ByRef`, `Array` (`dstPixelData() As Byte`), and `Optional ... = value` all rendered from real corpus bytes. `ParamArray` never appears — zero occurrences in 44 programs' source, re-confirmed by `grep -rln ParamArray corpus/vb6-code/*/*.frm corpus/vb6-code/*/*.bas corpus/vb6-code/*/*.cls` returning nothing this session — reported as a gap in `functyp.rs`'s own doc comment, never guessed. This is the named risk the roadmap itself states ("caps the procedure ratio... state the cap in the report"), not an unmet requirement. |
| OBJ-05 | `Declare` statements for external API calls | **CONFIRMED — reaches the product surface, does not repeat the Phase 3 pattern** | Traced end to end: `vb/project.rs::DeclareTable::read` populates `Report.declarations`; `crates/deform6-cli/src/main.rs:288` calls `print_declarations(report)` inside the live `inspect` path (confirmed by grep — the call site is not dead code, and the printed function is not a stub); a live run on `Grayscale.exe` prints `gdi32!StretchDIBits`, `gdi32!GetDIBits` and the three honest missing-part markers per entry. Unlike `frx::extract_blob` (Phase 3), this function is called from the command actually shipped, not merely unit-tested in isolation. |
| OBJ-06 | Private procedure reported as private, no invented name | **CONFIRMED** | `vb/privateobj.rs::Procedure::Private`, `is_plausible_identifier`; live output shows bare `private` lines with no name or index; `is_plausible_identifier`'s character-shape branch — flagged as untested by the 2026-09-08 report — now carries `the_identifier_rule_refuses_what_is_not_an_identifier` (confirmed present in `privateobj.rs` this session, asserting both `Form_Load`-shaped accepts and `1abc`/empty-string rejects), closing that gap |
| VER-01 | Differential test decompiles each corpus program against source | **CONFIRMED** | `differential.rs` run live this session, `EXPECTED_EXECUTABLE_COUNT: usize = 44` enforced in the test itself; independently re-counted this session with a null-delimited sweep (`find corpus -iname "*.exe" -print0 \| tr '\0' '\n' \| wc -l` → 44, correctly counting the nine space-containing corpus paths) |
| VER-02 | Expectation from the `.vbp` file list, never a directory glob | **CONFIRMED** | Read directly: `Project::declared_objects` walks only the eight object-declaring keys inside the `.vbp` text, joins each to a file on disk, and never calls a directory-listing function. Its own doc comment names the two orphan `cCommonDialog.cls` files this design keeps out. |
| VER-03 | `.vbp` selection by `ExeName32` where several sit in one directory | **CONFIRMED** | `select_project_file` scopes to the executable's own containing directory before matching `ExeName32`; `a_corpus_wide_index_keyed_on_exe_name_32_alone_collides` proves a bare `ExeName32`-only index breaks on the corpus's two `Project1.exe` siblings, and the scoped selector is what the harness actually calls |
| VER-04 | Exclusion rule as written data, not a per-program allowance | **CONFIRMED** | `support/rules.rs`'s `RULES` array and its own module doc: "No predicate ever compares against a source tree path, a program name..."; confirmed by reading every `Rule::excludes` predicate — each is structural (`in_standard_module`, `visibility`, `declared_in_project`), none names a corpus file |
| VER-05 | Recovery ratio pinned; a rise and a fall each fail the build with the correct message | **CONFIRMED — reproduced live, both directions, this session** | See Criterion 3 above; `tests/ratios.toml` holds 44 entries (`grep -c '^\[' tests/ratios.toml`), matching the corpus count |

**No overstated `[x]` found.** All eleven requirement checkboxes in
`REQUIREMENTS.md` are correct as marked. No correction was made to that
file.

### The OBJ-05 trace in full (the pattern this task was written to hunt)

1. `crates/deform6/src/vb/project.rs::DeclareTable::read` parses the external
   import table and returns `Vec<Declaration>`.
2. That value is assigned into `Report.declarations` in the composition path
   `inspect` builds its `Report` from (`vb/mod.rs`).
3. `crates/deform6-cli/src/main.rs:288` calls `print_declarations(report)`
   from inside the same function that prints the object graph, header and
   project fields — one linear `inspect` run, not a second unreached
   command.
4. A live run (`./target/debug/deform6 inspect
   corpus/vb6-code/Grayscale-effect/Grayscale.exe`) prints nine `Declarations`
   entries with real library and export names.

This is the opposite shape from Phase 3's `frx::extract_blob` (twenty unit
tests, zero call sites in `src/`) and `handler_address` (computed, tested,
reaching no report field). OBJ-05's value is computed once and flows through
exactly one path to the terminal.

### Data-Flow / Wiring — the "nothing written to disk" claim

```
$ git status --short   # before and after several inspect runs this session
?? .gsd/
?? .planning/milestone.lock
?? Notes/
```

Identical before and after `./target/debug/deform6 inspect` on three
different corpus executables this session (`Grayscale.exe`, `Map
Editor.exe`, `Custom_Filters.exe`). No new or modified file. The three
untracked entries above pre-date this verification and are unrelated to
`inspect`.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| (workspace-wide, phase-2 files) | — | `TBD`/`FIXME`/`XXX` grep across `object.rs`, `classify.rs`, `privateobj.rs`, `functyp.rs`, `project.rs`, `differential.rs`, `ratios.rs`, `support/vbp.rs`, `support/rules.rs` | — | Zero matches. No debt-marker gate issue. |
| (workspace-wide, phase-2 files) | — | `placeholder`/`not yet implemented`/`coming soon` grep | ℹ️ Info | Five hits, all doc-comment assertions that the code does *not* use a placeholder (e.g. `main.rs:785` "placeholder, no zero", `privateobj.rs:363` "no placeholder and no name derived from a vtable offset") or a description of test-fixture text in `project.rs`'s CLSID-join doc comment. None is a stub marker. |

### Requirements Coverage

All eleven requirements in `REQUIREMENTS.md`'s Traceability table map to
Phase 2 and are already marked `[x]`. No orphaned requirement: `grep -E
"Phase 2" .planning/REQUIREMENTS.md` names exactly OBJ-01..06 and
VER-01..05, matching the union of every plan's `requirements:` frontmatter
(`02-01` through `02-10`) with no leftover.

### Plans and Summaries

All ten `02-*-PLAN.md`/`02-*-SUMMARY.md` pairs exist. Every SUMMARY reads
`status: complete`; none halted. `ROADMAP.md`'s own "Plans: 4/10 plans
executed" summary line and its bottom Progress table ("4/10", "In
Progress") are both stale — every one of the ten plan checkboxes in the
same ROADMAP section already reads `[x]`. This report does not edit
`ROADMAP.md` (out of the stated scope of the REQUIREMENTS.md correction
instruction) but flags the inconsistency for whoever closes this phase.

### Gaps Summary

No gap blocks the phase goal. The one gap the 2026-09-08 report found
(`is_plausible_identifier`'s character-shape rule, zero test coverage) was
fixed the same day — `the_identifier_rule_refuses_what_is_not_an_identifier`
exists in the current `privateobj.rs` and was confirmed present this
session. No new gap was found by this round's independent re-check,
including the targeted hunt for an OBJ-05-shaped wiring defect the task
brief specifically asked for.

---

*Verified: 2026-09-11*
*Verifier: Claude (gsd-verifier)*
