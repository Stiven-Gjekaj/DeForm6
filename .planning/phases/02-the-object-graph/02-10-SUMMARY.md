---
phase: 02-the-object-graph
plan: 10
subsystem: vb, cli
tags: [report, inspect, object-graph, defects, cli-output, obj-01, obj-02, obj-03, obj-04, obj-05, obj-06, d-07, d-08, d-09, d-10, d-13, d-15]
status: complete

requires:
  - phase: 02-the-object-graph
    provides: >-
      Object, ObjectTable::walk (02-01); classify, ObjectKind (02-02);
      ObjectInfo, PrivateObj, ProcNames, Procedure, ProcedureList,
      ProcedureCounts, Gap, event_descriptor_addresses (02-03, 02-05);
      FuncTypDesc, VbType, TypeEntry, Argument, DefaultValue, Prototype,
      ProcedureSignature, PrototypeList, FuncTypeWalk (02-04); DeclareTable,
      Declaration, ExportName, ComponentTable, Component (02-06); the
      differential harness confirming 105/105 objects and 185/185
      procedures match the corpus in both directions (02-08)
provides:
  - >-
    Site, DefectKind and Defect deriving PartialEq and Eq, closing
    WINDOWS.md finding 3: Report can carry a Vec<Defect> and the whole
    crate still compares with assert_eq!
  - >-
    Report extended with objects: Vec<ObjectReport>, declarations:
    Vec<Declaration>, components: Vec<Component> and defects: Vec<Defect>,
    composed inside inspect from every phase 2 module
  - >-
    ObjectReport, ObjectProcedures, ProcedureEntry in vb/mod.rs: the
    composed object graph, joining the name array (privateobj) and the
    type descriptor array (functyp) by index into one printable value
  - >-
    the deform6 CLI's extended print_report: a Gaps section (the
    standard-module cap, an unknown object type, an unknown argument type
    code, an unresolved public variable count, an unrecoverable type
    descriptor) printed before the object graph, per D-10; the object
    graph itself (name, kind, prototypes, private markers); a Declarations
    section (library!export plus the three missing-parts markers)
affects:
  - >-
    Phase 4, which reads Report.objects, Report.declarations,
    Report.components and Report.defects to build the .vbp/.frm/.bas/.cls
    writer and the confidence report; the field names and shapes fixed
    here are the schema that plan inherits
  - >-
    Phase 6, which documents the release notes from this SUMMARY's "What
    the report says, and does not overstate" section

tech_stack:
  added: []
  patterns:
    - >-
      a leaf-pointer failure inside an already-reached object (ObjectInfo,
      PrivateObj) is converted to a Defect and a PrivateObj::Absent
      fallback, never propagated as a Refusal: read_private in vb/mod.rs
      is the one place this decision is made, and the deliberate breakage
      that turns it into a `?` propagation is the direct proof that
      inspect would otherwise refuse the whole file over one object's
      unreadable pointer
    - >-
      a procedure slot is composed from two independently-resolving arrays
      by index (ProcNames from Object.lpProcNamesArray, PrototypeList from
      PrivateObj.lpFuncTypeInfo): compose_procedures joins them without
      assuming either one succeeded, so a name can print with no prototype
      and neither array's own failure loses the other's data
    - >-
      the CLI's Gaps section is derived entirely from Report's public
      fields (objects, defects) rather than from new library-side
      aggregation functions, keeping the interpretation ("this defect
      means the type descriptor was unrecoverable") a CLI-layer concern
      the library does not have to also encode

key_files:
  created: []
  modified:
    - crates/deform6/src/error.rs
    - crates/deform6/src/vb/mod.rs
    - crates/deform6-cli/src/main.rs
    - crates/deform6-cli/tests/cli.rs
    - .planning/WINDOWS.md

decisions:
  - >-
    Site, DefectKind and Defect derive PartialEq and Eq (not just
    PartialEq): every field in all three is plain data (numbers, static
    text, Option<u32>), so Eq costs nothing and keeps Defect usable
    anywhere Site or DefectKind is used as a map key or set member later.
    Report itself drops Eq (kept before this plan) because
    functyp::DefaultValue::Single carries an f32, which has no Eq;
    PartialEq is all any assert_eq! in this crate needs.
  - >-
    ObjectReport carries procedures as one ObjectProcedures value (Slots
    or NoNameArray) rather than two separate fields for names and
    prototypes, because a caller (the CLI) always wants them joined by
    index and never one without the other for a given slot. compose_procedures
    is the one place this join happens.
  - >-
    A leaf-pointer failure in ObjectInfo::read or PrivateObj::read becomes
    a defect and a PrivateObj::Absent fallback, not a Refusal. No corpus
    program exercises this path (all 105 objects across 44 programs
    resolve both structures cleanly, per plan 02-08's differential), so
    the covering test patches Grayscale.exe's frmGrayscale in memory. This
    mirrors the same recoverable-vs-fatal precedent plan 02-01 set for a
    name pointer and plan 02-06 set for a Declare descriptor.
  - >-
    The CLI's Gaps section always states the standard-module cap, even at
    zero, per D-10's "the cap is stated in the report" read as a blanket
    policy fact rather than a conditional one. The other four gap
    categories (unknown object type, unknown argument type code,
    unresolved public variable count, unrecoverable type descriptor) list
    only what the run actually found.
  - >-
    The Gaps section prints immediately after the locked eight-line head
    and before the object graph, not "below the graph" as one bullet in
    the plan's own behaviour list states. The plan's own deliberate
    breakage instruction ("print the gaps section after the object list,
    confirm the fourth behaviour fails") only makes sense under the
    before-the-graph ordering, and it is the version with an executable
    test attached, so it is the version this plan implements. See
    "Defects found in the plan" below.

metrics:
  duration: 1 session
  completed: 2026-09-08

actuals:
  tokens: 15374
  tasks: 3
  commits: 3
plan_head_before: 73e6cc2
---

# Phase 02 Plan 10: `inspect` reports the object graph Summary

`vb/mod.rs` composes the object graph, the external declarations, the
components and every recoverable defect into `Report`; the `deform6` CLI
prints a Gaps section (the standard-module cap and every other open
question, stated before any recovery number, per D-10), the object graph
itself (name, kind, prototype per public procedure, `private` for
everything else), and a Declarations section (library and export name,
plus the three markers naming what a `Declare` statement does not hold).
`WINDOWS.md` finding 3 is fixed: `Report` carries the defects `inspect`
collects instead of dropping them.

## What this plan built

| Item | What it gives |
|---|---|
| `Site`, `DefectKind`, `Defect` (`error.rs`) | now derive `PartialEq, Eq`, closing finding 3 |
| `ObjectReport`, `ObjectProcedures`, `ProcedureEntry` (`vb/mod.rs`) | the composed object graph: name, kind, procedures joined from the name array and the type descriptor array by index |
| `Report.objects`, `.declarations`, `.components`, `.defects` | the four new fields `inspect` fills |
| `compose_object`, `read_private`, `compose_procedures` (`vb/mod.rs`) | the composition: a leaf-pointer failure (`ObjectInfo`, `PrivateObj`) becomes a defect, never a refusal |
| `print_gaps`, `print_objects`, `print_declarations` and their formatters (`deform6-cli/src/main.rs`) | the three printed sections below the locked eight-line head |

`crates/deform6/src/error.rs` grew by 69 lines (three derives, one new
test). `crates/deform6/src/vb/mod.rs` grew by 571 lines (the composed
types, `inspect`'s extension, nine new tests). `crates/deform6-cli/src/main.rs`
grew by 302 lines (the three printed sections and their formatters).
`crates/deform6-cli/tests/cli.rs` grew by 364 lines (eleven new tests, one
rewritten). The workspace test count rose from 252 to 270.

## What the report says, and does not overstate

Phase 2 recovers a great deal and cannot recover some things at all. This
plan's job is to make the difference visible to a reader, not to collapse
it. Measured facts, carried forward from plans 02-01 through 02-09 and
confirmed again by this plan's own runs:

- **105 objects across 44 programs**, all matching their `.vbp` by name and
  kind: 53 forms, 8 modules, 44 classes.
- **185 public procedure names recovered, out of 904 slots.** 13 programs
  recover zero — not a bug: those programs' procedures are all `Private`,
  and the compiler keeps no private procedure's name. The CLI's per-object
  `private` line, with no name and no index, is what a reader sees for
  every one of those.
- **23 slots are capped by the `.bas` rule.** For a standard module the
  whole `lpProcNamesArray` pointer is `0` — module procedures are
  name-less through this structure, not merely prototype-less. The CLI's
  Gaps section states this cap's object and slot counts unconditionally,
  even when both are zero, and each capped module prints its declared
  count with the D-13 sentence, never an empty list. `Map Editor.exe`'s
  `Declaration_Module` (1 slot) and `Sub_Module` (7 slots) are the real
  example this plan's tests and human check both exercise.
- **`ParamArray` occurs nowhere in the corpus** and its encoding is
  unknown. Nothing in `functyp.rs`, `vb/mod.rs` or the CLI ever names it
  outside a comment; no modifier or type code claims it.
- **No corpus program carries an event descriptor** (`cntEvents` is 0 in
  97 of 97 objects), so no event name is recovered, none is printed, and
  none is invented.
- **`cntPublicVars` does not count public variables.** The CLI's Gaps
  section prints it as an open question ("cntPublicVars is N, and its
  meaning is unresolved"), never as a count. `Mandelbrot.exe`'s
  `frmFractal` (declares 0, reports 9) and `Grayscale.exe`'s
  `pdOpenSaveDialog` (declares 0, reports 4) and `Map Editor.exe`'s
  `pdOpenSaveDialog` (same, reports 4) are the real examples a run
  surfaces.
- **All 428 unresolvable name-array entries are uninitialised
  compiler-buffer content**, per plan 02-08's corpus-wide sweep. Every one
  of `Mandelbrot.exe`'s nine `frmFractal` entries is this shape:
  non-null, resolves to no section, carried as a `Defect` with its raw
  address, printed as `private` with no name, never presented as a
  recovered name.

## The verbatim printed report, for two corpus programs

### `Mandelbrot.exe` — recovers zero procedure names

```
$ cargo run -p deform6-cli -- inspect corpus/vb6-code/Mandelbrot/Mandelbrot.exe
File      Mandelbrot.exe  (28672 bytes)
Format    PE32, 3 sections
Runtime   MSVBVM60.DLL  (Visual Basic 6)
Header    VB5! at 0x00001760  build 0x2636
Project   Mandelbrot_Fractal_Demo
Title     Mandelbrot Fractal Demo
Mode      native
Objects   1

Gaps
  the standard-module cap applies to 0 object(s) and 0 procedure slot(s) in this file; a standard module's procedure names are not reachable through this structure at all
  "frmFractal": cntPublicVars is 9, and its meaning is unresolved

Object graph
  frmFractal  (form)
    private
    private
    private
    private
    private
    private
    private
    private
    private

Declarations
  gdi32!SetPixelV
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
$ echo $?
0
```

### `Grayscale.exe` — three objects, twelve recovered prototypes, eight external imports

```
$ cargo run -p deform6-cli -- inspect corpus/vb6-code/Grayscale-effect/Grayscale.exe
File      Grayscale.exe  (45056 bytes)
Format    PE32, 3 sections
Runtime   MSVBVM60.DLL  (Visual Basic 6)
Header    VB5! at 0x00001aec  build 0x2636
Project   Grayscale_Dialog
Title     Grayscale Application
Mode      native
Objects   3

Gaps
  the standard-module cap applies to 0 object(s) and 0 procedure slot(s) in this file; a standard module's procedure names are not reachable through this structure at all
  "pdOpenSaveDialog": cntPublicVars is 4, and its meaning is unresolved

Object graph
  frmGrayscale  (form)
    private
    private
    private
    private
    private
    private
    private
    private
    ByteMeL(tempVar As Long) As Byte
    private
    private
    private
    private
    DrawGrayscaleAverageMethod(ByRef srcPic As Object (an external COM object, unresolved, raw address 0x00403fb8), ByRef dstPic As Object (an external COM object, unresolved, raw address 0x00403fb8))
    DrawGrayscaleHumanMethod(ByRef srcPic As Object (an external COM object, unresolved, raw address 0x00403fb8), ByRef dstPic As Object (an external COM object, unresolved, raw address 0x00403fb8))
    DrawDesaturate(ByRef srcPic As Object (an external COM object, unresolved, raw address 0x00403fb8), ByRef dstPic As Object (an external COM object, unresolved, raw address 0x00403fb8))
    DrawGrayscaleDecompose(ByRef srcPic As Object (an external COM object, unresolved, raw address 0x00403fb8), ByRef dstPic As Object (an external COM object, unresolved, raw address 0x00403fb8), Optional minValue As Boolean = true)
    DrawGrayscaleSingleChannel(ByRef srcPic As Object (an external COM object, unresolved, raw address 0x00403fb8), ByRef dstPic As Object (an external COM object, unresolved, raw address 0x00403fb8), Optional cChannel As Long = 0)
    DrawGrayscaleCustomShades(ByRef srcPic As Object (an external COM object, unresolved, raw address 0x00403fb8), ByRef dstPic As Object (an external COM object, unresolved, raw address 0x00403fb8), Optional numOfShades As Long = 256)
    DrawGrayscaleCustomShadesDithered(ByRef srcPic As Object (an external COM object, unresolved, raw address 0x00403fb8), ByRef dstPic As Object (an external COM object, unresolved, raw address 0x00403fb8), Optional numOfShades As Long = 256)
  pdOpenSaveDialog  (class)
    private
    private
    private
    private
    private
    private
  FastDrawing  (class)
    private
    private
    private
    private
    GetImageWidth(ByRef srcPictureBox As Object (an external COM object, unresolved, raw address 0x00403fb8)) As Long
    GetImageHeight(ByRef srcPictureBox As Object (an external COM object, unresolved, raw address 0x00403fb8)) As Long
    GetImageData2D(ByRef srcPictureBox As Object (an external COM object, unresolved, raw address 0x00403fb8), ByRef dstPixelData() As Byte, Optional fixOrientation As Boolean = false)
    SetImageData2D(ByRef dstPictureBox As Object (an external COM object, unresolved, raw address 0x00403fb8), imgWidth As Long, imgHeight As Long, ByRef srcPixelData() As Byte, Optional fixOrientation As Boolean = false)

Declarations
  gdi32!StretchDIBits
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
  gdi32!GetDIBits
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
  gdi32!SetStretchBltMode
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
  gdi32!GetObjectA
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
  kernel32!lstrlenW
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
  comdlg32!CommDlgExtendedError
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
  comdlg32!GetSaveFileNameW
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
  comdlg32!GetOpenFileNameW
    the Visual Basic procedure name and its Alias are not in this file; only the export name that survives compilation is shown
    the argument names and types of this Declare are not in this file
    the owning module and the Public or Private marker of this Declare are not in this file
$ echo $?
0
```

`Grayscale.exe` gives three objects with the right kinds, twelve recovered
prototypes (8 on `frmGrayscale`, 0 on `pdOpenSaveDialog`, 4 on
`FastDrawing`) and eight external imports, matching the plan's own
must-have exactly. `Mandelbrot.exe` gives one declaration and nine private
procedures, also matching.

## What was broken on purpose, and what failed

Six deliberate breakages, two per task, all run in the working tree,
observed, and reverted before the following commit.

### Task 1

**1. The defect list dropped from the composition.** `defects: Vec::new()`
in place of the real list.

```
---- vb::tests::a_capacity_below_the_object_count_reaches_the_caller_as_a_defect ----
panicked at crates/deform6/src/vb/mod.rs:663:14:
the patched capacity must produce a CountMismatch defect on Report.defects
```

Exactly the one test that proves finding 3 is fixed failed — the rest of
the 203-test suite passed, which is itself evidence that nothing else in
this crate reads `Report.defects` yet. Restored; all 204 tests pass again.

**2. An unresolved `PrivateObj` turned into a refusal instead of a
defect.** Required a temporary, larger edit than the plan's one-line
framing suggests: `read_private` and `compose_object` had to actually
return `Result` and propagate with `?`, and `inspect`'s object-composition
step had to `collect::<Result<Vec<_>, Refusal>>()?` instead of `collect()`,
because the real code never had a `Result` in this path to begin with —
turning a caught error into "a refusal" means adding the fallible
signature back, not flipping one match arm. Restored to the caught-and-
defected version afterward.

```
---- vb::tests::an_unresolved_private_obj_loses_only_its_own_objects_prototypes ----
called `Result::unwrap()` on an `Err` value: Damaged("the PrivateObj pointer is in no section")
```

Exactly the covering test failed, and `inspect` genuinely returned
`Err(Refusal::Damaged(..))` for a file that should still produce a
`Report` with one defect. Restored; all 204 tests pass again.

### Task 2

**3. A private procedure printed as `private` followed by its index.**

```
---- grayscale_prints_three_objects_and_twelve_prototypes_and_exits_zero ----
assertion `left == right` failed: stdout was: "...private 0\nprivate 1\n..."
  left: 34
 right: 12
---- grayscale_prints_prototypes_with_modifiers_and_defaults_and_marks_private_procedures ----
at least one private procedure must print: "..."
```

**Two tests failed, not the one the plan's "confirm the fifth behaviour
fails" predicts.** The prototype-counting test's line-shape filter
(`trim() == "private"`) no longer matched `"private 3"`, so it undercounted
private lines to zero and its own assertion about the *twelve public*
count broke as a side effect of the changed total line shape; the
modifier/default test's own "every private line is the bare word" loop
caught the shape change directly. Restored; both tests, and all 20 in
`cli.rs`, pass again.

**4. A standard module's procedures printed as nothing (an empty match
arm).**

```
---- map_editor_prints_its_standard_modules_procedure_count_and_not_an_empty_list ----
stdout was: "...Declaration_Module  (module)\n  Sub_Module  (module)\n..."
```

Exactly the one test the plan predicts failed. Restored; all 20 tests
pass again.

### Task 3

**5. The Gaps section printed after the object list.**

```
---- the_gaps_section_prints_before_the_object_graph ----
the Gaps section must print before the Object graph section
```

Exactly the fourth behaviour's test failed, as predicted. Restored; all
20 tests pass again.

**6. Internal `Declare` entries printed as declarations too.** As literally
written this needs an edit to `vb/project.rs`'s `DeclareTable::read`,
which already filters `dwEntryType == 6` before `Report.declarations` is
ever built — that filter is upstream of every file this plan owns
(`crates/deform6/src/error.rs`, `crates/deform6/src/vb/mod.rs`,
`crates/deform6-cli/`), and `project.rs` belongs to plan 02-06, sealed
before this plan started. Rule 4 (an architectural boundary, not a
decision this plan can make): the substance was proved instead, without
touching `project.rs`, by temporarily duplicating one real declaration in
`print_declarations`'s own loop (`report.declarations.iter().chain(report.declarations.first())`),
which raises Grayscale's declaration-line count from 8 to 9 exactly the
way a leaked internal entry would.

```
---- grayscale_prints_one_declaration_line_per_external_import_and_none_for_the_internal_one ----
assertion `left == right` failed
  left: 9
 right: 8
```

Exactly the first behaviour's test failed at nine lines instead of eight,
matching the plan's own prediction. Restored; all 20 tests pass again.

## Defects found in the plan

Three, all measurement or wording corrections, none requiring a scope
change.

### 1. Task 1's must-have: "capacity exceeds count" is the wrong direction

The plan's own text (both the `<must_haves>` list and the task 1
behaviour list) asks for a test proving "a corpus program whose object
array capacity exceeds its count reaches the caller with the count
defect on it." Measured against `vb/project.rs`'s own `count_defects`
(sealed, from plan 01-07/02-06): the `CountMismatch` defect fires only
when the capacity is **below** the count — the opposite direction. A
capacity above the count is the normal, silent shape 15 of 44 corpus
files already have (the array rounded up), and flagging it would mark a
third of the corpus damaged, which is exactly the shape phase 1's own
plan 01-07 SUMMARY already rejected. The test this plan ships
(`a_capacity_below_the_object_count_reaches_the_caller_as_a_defect`)
patches `Grayscale.exe`'s `wCompiledObjects` from 4 down to 1 (below
`wTotalObjects` 3), which is the one direction that can actually produce
this defect on a real corpus file, and asserts the real behaviour rather
than the plan's inverted phrasing.

### 2. Task 2's must-have: "eight private markers" on the friend class is six

Task 2's seventh behaviour and the plan's own `<must_haves>` list both
say `Grayscale.exe` gives "eight private markers for the class whose
procedures are declared friend." Measured directly, and matching plan
02-07's own independent correction of the identical claim: `pdOpenSaveDialog.cls`
declares six procedure slots (two `Friend Function` members plus four
`Private Declare Function` lines), and every one of the six is `private`
in this run's output, not eight. `grayscale_prints_three_objects_and_twelve_prototypes_and_exits_zero`
asserts the measured `22` total private lines across all three objects
(34 total slots minus 12 public), with `pdOpenSaveDialog` contributing
exactly 6 of those, and its doc comment records the correction rather
than silently matching the plan's number.

### 3. Task 3's behaviour list contradicts itself on where the Gaps section goes

One bullet says "a gaps section prints **below the graph**"; two bullets
later, another says "the gaps section prints **before** any recovery
number," and the task's own deliberate-breakage instruction ("print the
gaps section after the object list, and confirm the fourth behaviour
fails") only has an executable meaning under the second reading. This
plan implements Gaps immediately after the locked head and before the
object graph, matching the version with a test attached, and records the
contradiction here rather than silently picking one.

## Deviations from Plan

### Auto-fixed issues

None beyond the plan-text corrections recorded above. No Rule 1, 2 or 3
deviation was needed: every behaviour the plan asks for was implementable
as specified once the three corrections above were applied.

## The gate

Run on the working tree before each commit and once more on the final
committed tree.

| Command | Result |
|---|---|
| `cargo fmt --all --check` | 0, no output |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0, 270 tests pass (204 lib, 1 corpus_sweep, 7 differential, 9 refusal, 27 support_selftest, 2 type_descriptors, 20 cli); 252 before this plan |
| `sh scripts/prove-lint-wall.sh` | 0, "The wall stops every bad shape, and the tree it leaves behind is clean." |
| `sh scripts/prove-region-wall.sh` | 0, "The type refuses every shape, and the tree it leaves behind is clean." |

```
$ test 0 -eq "$(grep -rn -vE '^\s*//' crates/deform6/src/ | grep -cE 'std::fs|std::path|PathBuf')"
$ echo $?
0
```

## The human check

Per task 3's `<human-check>`, both commands were run and their whole
output read.

`cargo run -p deform6-cli -- inspect corpus/vb6-code/Grayscale-effect/Grayscale.exe`:
no procedure carries an invented name (every private slot is the bare
word `private`, every public slot's name and prototype come from bytes
the file holds); the Gaps section (the standard-module cap, stated at 0
objects and 0 slots, and the `pdOpenSaveDialog` `cntPublicVars` gap)
prints before the Object graph section, which is where any per-object
recovery number appears; every declaration under `Declarations` carries
all three markers naming what the file does not hold.

`cargo run -p deform6-cli -- 'inspect' 'corpus/vb6-code/Map-editor-2D/Map Editor.exe'`:
the two standard modules, `Declaration_Module` and `Sub_Module`, each
print their declared procedure count (1 and 7) with the D-13 sentence
("their names are not reachable through this structure"), never as an
empty list.

## Threat mitigations

| Threat | State |
|---|---|
| T-02-46, defects collected and dropped | Mitigated. `Site`, `DefectKind`, `Defect` derive `PartialEq, Eq`; `Report.defects` carries every defect `inspect` collects. `a_capacity_below_the_object_count_reaches_the_caller_as_a_defect` is the instrument. Closes `WINDOWS.md` finding 3. |
| T-02-47, an inferred item printed without its marker | Mitigated. The ordinal alias path (the only inferred item this phase produces) prints `(inferred alias)` in `print_declarations`. `no_declaration_in_a_real_run_carries_the_inferred_marker` checks a real run's output directly, over `Grayscale.exe`'s eight declarations; no corpus file exercises the inferred branch itself (0 of 220 external entries anywhere are ordinal), so this test proves the negative: nothing recovered is ever marked inferred. |
| T-02-48, an unknown value printed as a name | Mitigated. `format_kind` prints `unknown, raw value {:#010x}` for `ObjectKind::Unknown`; `format_vb_type` prints `unrecognised type, raw value {:#04x}` for `VbType::Unknown`. `a_patched_unknown_object_kind_prints_its_raw_value_and_exits_zero` patches a real corpus file's `fObjectType` and confirms the run still exits 0. |
| T-02-49, denial of service from a printed name | Mitigated, per phase 1/2's existing string bounds. Every name this plan prints already passed through a bounded `cstr` read (`NAME_MAX` or `PROC_NAME_MAX`) before it reached `Report`; nothing in `main.rs` allocates from a file-derived length. |
| T-02-50, a criterion met by inventing what the file lacks | Mitigated. `Declaration::NAME_MARKER`, `ARGUMENTS_MARKER` and `SCOPE_MARKER` print beside every declaration; the argument list is genuinely empty (no argument names or types exist for a `Declare` in this format) rather than a fabricated empty list standing in for a recovered one. `mandelbrot_prints_one_declaration_with_its_three_markers` is the instrument. |

## Known Stubs

None. Every field this plan adds to `Report` is populated from a real
walk; nothing is a placeholder standing in for missing work.

## Deferred Issues

- `WINDOWS.md` findings 1, 2, 4 and 5 are untouched by this plan (finding
  3 is fixed here); they remain open for a future plan.
- `crates/deform6/tests/ratios.toml`, the pinned recovery ratio and
  `crates/xtask` belong to plan 02-09, running in parallel; this plan
  neither reads nor writes any of them.

## Threat Flags

None. This plan adds no network endpoint, no auth path, and no new file
access path: the library still takes only a byte slice
(`grep -rn -vE '^\s*//' crates/deform6/src/ | grep -cE 'std::fs|std::path|PathBuf'`
is 0), and the CLI's one `std::fs::read` predates this plan.

## Commits

| Commit | Subject |
|---|---|
| `ee66a6c` | Carry the collected defects out of inspect |
| `29b3200` | Print the objects, the kinds and the procedure prototypes |
| `b40563d` | Print the external declarations and the open gaps |

`git rev-list --count 73e6cc2..HEAD` is 3 before this SUMMARY's own
documentation commit, one per task, each carrying its own tests.

`WINDOWS.md` finding 3 is marked `fixed` in the documentation commit that
carries this SUMMARY, per `AGENTS.md`'s rule that documentation sits in
its own commit, separate from code.

## Self-Check: PASSED
