# Phase 1 context: It reads the file

**Captured:** 2026-09-07

Most of this phase's design was settled before planning began, in the grilling
session that produced `PROJECT.md` and in the three research documents. This
file records what is decided, so the planner does not re-open it, and records
the two decisions taken at the planning gate.

## Decided at the planning gate

### `inspect` prints text. Machine-readable output waits for Phase 4.

`deform6 inspect` writes human-readable text only in this phase. There is no
`--json` yet.

**Why.** Phase 4 introduces the confidence report, which is the project's
machine-readable surface (RPT-01 to RPT-05). Adding a second JSON shape here
would commit to a schema before the report schema exists, and the two would
then have to be kept in step for no gain. One schema, introduced once.

The person this tool is for is at a terminal looking at a file they do not
recognise. Text serves them. A script that wants structure can wait one phase.

The shape:

```
File      Mandelbrot.exe  (28672 bytes)
Format    PE32, 3 sections
Runtime   MSVBVM60.DLL  (Visual Basic 6)
Header    VB5! at 0x00001760  build 0x2636
Project   Mandelbrot_Fractal_Demo
Title     Mandelbrot Fractal Demo
Mode      native
Objects   1
```

Those numbers are measured from `corpus/vb6-code/Mandelbrot/Mandelbrot.exe`,
not invented. An earlier draft of this file carried made-up values. Do not pin
them in a test either way: pin the shape, and read the values from the file.

### The exit code names the reason.

| Code | Meaning |
|---|---|
| 0 | The file was read |
| 1 | Not a PE file |
| 2 | A PE file, but it holds no Visual Basic runtime |
| 3 | Visual Basic 5, not Visual Basic 6 |
| 4 | Visual Basic 6, but damaged. `--salvage` may get something |
| 5 | An internal error in DeForm6 |

**Why.** A person recovering a system points this tool at a directory of files
whose origin nobody remembers. A loop over that directory must be able to sort
the files without reading English. It also makes the refusal tests assert on a
number rather than on the wording of a message, so a reworded sentence does not
fail the suite and a changed meaning does.

Code 4 is defined here and is not reachable until Phase 5, because `--salvage`
does not exist yet. Define the variant now so the numbering never moves.

## Already decided, not open

From `.planning/research/STACK.md`, all measured rather than assumed:

- **PE parsing**: `object` 0.40.0, `default-features = false`, features
  `read_core, pe, std`. Two crates. It is the only candidate whose README
  states that malformed input yields an error rather than a panic, and that was
  tested against four hostile inputs before the choice was made.
- **Binary reading**: hand-written. A bounded `Region` with no infallible
  accessor, plus `Off`, `Rva` and `Va` newtypes that do not implement `Add`.
  Zero crates.
- **Errors**: a two-level enum with `thiserror` for `Display`. A `Defect`
  carries a site, a kind and a severity. A `Journal` has one `record` method
  that applies the strict or salvage policy.
- **CLI**: `clap` 4.6.6 with `default-features = false`.
- **Layout**: `crates/deform6` and `crates/deform6-cli`. `corpus/` stays at the
  repository root.

From `.planning/research/STRUCTURES.md`, verified against all 44 corpus
binaries rather than cited:

- The entry stub is `push imm32` then `call rel32` in 44 of 44.
- The pushed pointer lands on `VB5!` in 44 of 44.
- `VBHeader + 0x30` reaches ProjectInfo in 44 of 44.
- `ProjectInfo + 0x20` is the native discriminator, non-zero in 44 of 44.
- `ObjectTable + 0x40` gives a readable project name in 44 of 44.

## What this phase must not do

- Do not implement an entry stub variant that no corpus file shows.
  `STRUCTURES.md` §1.3 names `0x5A` and `0x11` and has no sample of either.
  A variant with no sample is a guess with a code path.
- Do not claim the P-code branch is tested. Every vendored project carries
  `CompilationType=0`. The branch that reports P-code exists and no file in the
  repository exercises it. Say so in the test, do not paper over it.
- Do not add a lint allowance to get the deny wall to pass. The wall exists to
  make the wrong shape fail to compile.

## The gap to close here

`STRUCTURES.md` §2.3 disputes the meaning of `VBHeader` 0x58 and 0x5C. It is
unresolved and it blocks the `.vbp` `Title=` and `ExeName32=` keys in Phase 4.

Close it in this phase by reading both fields from all 44 corpus binaries and
diffing the values against the `Title=` and `ExeName32=` lines in the matching
`.vbp`. The corpus answers this question. Record the answer in
`STRUCTURES.md`, and say which of the two candidate readings the evidence
supports.

## Three decisions taken after research, 2026-09-07

Phase 1 research built a working prototype rather than describing one, and it
raised three things the gate decisions did not cover.

### `clap` already uses exit code 2, so the CLI must not let it exit

`clap` exits with status 2 on a usage error. Exit code 2 is locked in the table
above as "a PE file, but it holds no Visual Basic runtime". A user who
mistypes a flag would get the code that means "this file is not Visual Basic",
which is worse than no code at all.

**`Cli::try_parse` with a hand written mapping is required, not a preference.**
Do not call `Cli::parse`. A usage error maps to code 5.

### Code 3 widens to "Visual Basic, but not version 6"

The table said "Visual Basic 5, not Visual Basic 6". Research found VB4-32,
which imports `VB40032.DLL`, has no code at all. Rather than add a sixth code
for a fourth runtime, code 3 now means any Visual Basic runtime that is not
version 6, and the message names which one it found.

VB4-16 is unreachable and needs no handling. A 16 bit VB4 image is an NE file,
not a PE file, so it never gets past the format check and it exits 1.

### `inspect` prints the object count in this phase

The count comes from `wCompiledObjects` in the object table, and Phase 1
already reaches the object table because the project name is read from it. The
count is therefore free and honest here. Walking the objects is Phase 2, and
this phase prints the number without naming any of them.
