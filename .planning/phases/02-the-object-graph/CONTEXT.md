# Phase 2 context: The object graph

**Captured:** 2026-09-07, after phase 1 completed and verified.

## What phase 1 leaves you

134 tests. `deform6 inspect` identifies a VB6 executable, names its project and
title, reports the compilation mode, counts its objects, and refuses everything
else with one sentence and a meaningful exit code.

Available and proved: `Region` with no infallible accessor; `Off`, `Rva` and
`Va` newtypes that do not implement `Add`; `Site`, `DefectKind`, `Severity`,
`Defect`, `Error`, `Refusal`, `Mode`, `Journal`; `PeImage` with one
address-to-offset predicate, the import walk and `has_clr_header`;
`header_region` and `VbHeader` with 18 fields; `classify` and `runtime_of`;
`ProjectInfo`, `ObjectTableHead`, `Report` and `inspect`.

The lint wall and two compile-fail proof scripts run in CI and between them
refuse eleven bad shapes.

## The one correction phase 1 already made to this phase's ground

**Do not read the object count from `wCompiledObjects` at `ObjectTable + 0x2C`.**

Measured twice, independently, against all 44 corpus programs with the `.vbp`
selected by `ExeName32`:

| Field | Equals the declared object count |
|---|---|
| `wTotalObjects` at 0x2A | 44 of 44 |
| `wCompiledObjects` at 0x2C | **29 of 44** |
| `wObjectsInUse` at 0x2E | 44 of 44 |

In the other 15 files `wCompiledObjects` is the capacity of the object array
rounded up, so a program declaring 1, 2 or 3 objects reports 4. The slots past
`wTotalObjects` hold null or garbage. `LockWorkStation` declares 1 and reports
4. `Grayscale` declares 3 and reports 4.

`Report::object_count()` already returns `wTotalObjects`. **The object array
loop bound is `wTotalObjects`.** `STRUCTURES.md` §4.1 and §4.1.1 record the
measurement; the earlier recommendation in §4 is withdrawn.

This also corrects two of the three open references this project leans on:
`python-vb` and the Gen Digital article both loop on `wCompiledObjects`.
Semi VB Decompiler loops on `wTotalObjects` and is right.

## The harness rule this phase exists to get right

**A subset assertion cannot see an over-count.**

The first corpus walk in this repository looped on `wCompiledObjects` and
reported that every declared object was recovered in 44 of 44. It asserted that
the expected names were a **subset** of the recovered names, so the extra slots
the capacity introduced added unexpected names without failing anything. The
over-count was invisible for hours behind a green check.

`differential.rs` must compare **both directions**: every declared object is
recovered, and every recovered object is declared. A one-directional assertion
is how this phase ships a wrong ratio that looks right.

Two more rules the same episode produced, both already requirements:

- The expectation comes from the file list the `.vbp` declares, never from a
  directory glob. Two corpus projects hold a `cCommonDialog.cls` that the
  `.vbp` never lists, so the compiler never built it in. A glob counts it as a
  recovery failure. (VER-02)
- Where a directory holds several `.vbp` files, select the one whose
  `ExeName32` names the executable under test. Two corpus projects need this.
  (VER-03)

## A `.vbp` value can contain a double quote

`corpus/vb6-code/Sepia-effect` carries:

    Title="Sepia / "Antique" Image Filter"

A reader written as `"([^"]*)"` stops at the first inner quote and returns
`Sepia / ` without failing. Take everything between the first quote and the
**last** quote on the line.

One corpus project has no `Title=` key at all, and the compiler wrote the
project name into the binary field instead. **An absent key is not an empty
value.**

## What this phase must not do

- **Do not let the harness share code with the thing it tests.**
  `support/vbp.rs` is a second, independent reader and must not call anything
  in `src/`. A harness that shares a reader agrees with a bug in that reader.
- **Do not guess a type code.** Fifteen are unassigned. An unknown code becomes
  a reported gap, not a guessed type name.
- **Do not refuse an object whose `fObjectType` is unknown.** The MDIForm value
  is in no source. Classify it `Unknown`, flag it, carry on.
- **Do not present an inferred event name as recovered.** They come from a
  positional heuristic. Mark every one `inferred`.
- **Do not discover the `.bas` cap when a ratio looks low.** A standard module
  is not a COM object and carries no type descriptors at all, so OBJ-04 is
  unreachable for a procedure in a `.bas`. State the cap in the report before
  the first ratio is pinned.

## Traps that caught every phase 1 executor

- A private item added in one task whose only caller arrives in the next fails
  the gate as dead code. `-D warnings` makes that an error and `AGENTS.md`
  forbids an allowance. Move the item to sit with its caller.
- A deliberate breakage that produces no failure means the covering test does
  not exist. Four of those were found in phase 1. Write the missing test and
  re-break; do not record "no failure" as a pass.
- An identity can hide a confusion. `Mandelbrot`'s header sits at RVA `0x1760`
  and file offset `0x1760`, and both corpus files put the import directory
  where RVA equals file offset. A test that could pass under such an identity
  proves nothing. Build a synthetic image where the two differ.

## Open defects carried in from phase 1

`.planning/WINDOWS.md` holds four, all of the "cannot be proved" kind rather
than "is broken". Two touch this phase:

- **Finding 3**: `inspect` drops the defects it collects, because `Report`
  derives `PartialEq` and `Defect` does not. This phase reports gaps and
  inferences, so it needs those defects to reach the caller. Fix it here.
- **Finding 4**: `runtime_dll` cannot be proved to come from the file, because
  `imported_dlls` upper-cases every name and `classify` accepts only case
  variants of the constant. The fix is to return the name verbatim and compare
  case-insensitively, after which a lower-case import fixture separates them.
  Cheap, and this phase touches neither file, so take it only if it is free.

## Two decisions taken after research, 2026-09-07

Phase 2 research measured the corpus rather than reading the documents, and it
raised two questions measurement cannot answer.

### The ratio is pinned over procedures, not objects

Object recovery is 44 of 44 across the whole corpus with no variance. A pinned
ratio on it could never move, so it could never fail, and `AGENTS.md` says a
test that cannot fail is worse than none.

**Object recovery is asserted as an equality, in both directions**: every
object the `.vbp` declares is recovered, and every object recovered is
declared. That is an invariant, not a ratio.

**The pinned ratio is over procedures**, where the variance is real. Worked
examples the research measured: `Mandelbrot` 9 of 9, `Grayscale-effect` 12 of
34, `LockWorkStation` 0 of 1.

### The roadmap had the two failure messages the wrong way round

Success criterion 3 said that editing a pin **down** produces `REGRESSION`.
That is backwards, and the criterion is corrected in `ROADMAP.md`.

- Editing a pin **up** means the pin now claims more than the tool recovers.
  Something is missing. That is `REGRESSION`, and the message lists what.
- Editing a pin **down** means the tool now recovers more than the pin claims.
  That is `MOVED UP`, and the message prints the TOML block to paste.

The messages describe what happened to the **tool**, not to the file.

### One finding that changes what OBJ-03 can promise

Research measured something stronger than the documented behaviour. For a
`.bas` standard module the whole `lpProcNamesArray` **pointer** is `0`, in 8 of
8 corpus module objects, even though `ProcCount` correctly reports 1 to 7
procedures each.

So a module's procedures are not merely prototype-less, as the `.bas` cap in
`ROADMAP.md` says. They are **name-less through this structure**. OBJ-03
promises public procedure names for every object; for a `.bas` that is not
reachable from the object table at all. Say so in the report and in the ratio,
and do not present the count as a recovery failure.

### One field to distrust by default

`cntPublicVars` does not count source-level `Public` variables. A corpus class
with zero `Public` declarations reports 4. This is the same shape as
`wCompiledObjects`, which turned out to be a rounded capacity rather than a
count. Do not build the `PubVarDesc` stride on the assumption that this field
means what its name says.
