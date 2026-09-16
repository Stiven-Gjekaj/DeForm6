<div align="center">

<img src="https://raw.githubusercontent.com/Stiven-Gjekaj/DeForm6/main/docs/logo.svg" alt="DeForm6" width="112">

### Metadata recovery from a Visual Basic 6 executable

_Every fact traces to a byte this run read_

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.97.1-DEA584?style=for-the-badge&logo=rust&logoColor=white" alt="Rust 1.97.1"/>
  <img src="https://img.shields.io/badge/Edition-2024-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Edition 2024"/>
  <img src="https://img.shields.io/badge/unsafe-forbidden-success?style=for-the-badge" alt="unsafe forbidden"/>
</p>

<p align="center">
  <a href="https://github.com/Stiven-Gjekaj/DeForm6/actions/workflows/gate.yml"><img src="https://img.shields.io/github/actions/workflow/status/Stiven-Gjekaj/DeForm6/gate.yml?label=gate&style=flat-square" alt="Gate"/></a>
  <a href="https://github.com/Stiven-Gjekaj/DeForm6/actions/workflows/fuzz.yml"><img src="https://img.shields.io/github/actions/workflow/status/Stiven-Gjekaj/DeForm6/fuzz.yml?label=fuzz&style=flat-square" alt="Fuzz"/></a>
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="MIT License"/>
</p>

<p align="center">
  <a href="#overview"><b>Overview</b></a> |
  <a href="#what-it-returns"><b>What It Returns</b></a> |
  <a href="#what-it-will-not-do"><b>What It Will Not Do</b></a> |
  <a href="#quick-start"><b>Quick Start</b></a> |
  <a href="#how-it-works"><b>How It Works</b></a> |
  <a href="#the-numbers"><b>The Numbers</b></a>
</p>

</div>

---

## Overview

DeForm6 reads a compiled Visual Basic 6 executable and writes back a Visual Basic project. The first milestone recovers the metadata only: the forms, the control trees, the property values, the names, and the procedure signatures. It does not recover statements. Do not add a claim that it does.

One rule decides almost everything else in this repository.

**Every fact DeForm6 reports traces to a byte it read, and it names how sure it
is of each one.**

Tools in this field tend to state a single confident figure and leave the
reader to guess what sits behind it.
DeForm6 does the opposite.
It grades each recovered fact with one of three words, it writes down every
place the read did not resolve, and it lists every boundary the run reached.
A thing it cannot prove is reported as a thing it cannot prove.

| It gives | Because |
| -------- | ------- |
| A claim you can check | Each fact carries a confidence word, and the report says which byte or which assumption produced it |
| A refusal instead of a guess | The reader stops and names the offset it did not understand, rather than printing a tree it cannot prove |
| A limit you can read before you run it | Every open gap is a row in this file, with the safe default the tool chose |
| A number you can recompute | Every figure below is pinned in `tests/ratios.toml` and asserted by a named test on every run |

---

## What it returns

<table>
<tr>
<td valign="top">

### What a run writes

A run of `extract` writes:

- One `.vbp` project file.
- One `.frm` file for each form.
- One `.frx` file for each form that carries a resource blob.
- One `.bas` file for each standard module.
- One `.cls` file for each class module.
- One JSON report, beside the project, naming what the run recovered and how
  sure it is of each fact.

The JSON report holds exactly three top level keys: `items`, `defects`, and
`limits`.
`items` lists every recovered fact together with its confidence word.
`defects` lists every place the read did not resolve cleanly.
`limits` lists every boundary this run reached, in plain sentences.

</td>
<td valign="top">

### How it behaves

- Two subcommands, and no more. `inspect` reads one executable and prints what
  DeForm6 found in it, and writes nothing to disk. `extract` reads one
  executable and writes a project directory that VB6 can open.
- One run is one process over one file.
- The library never opens a file. The command line crate owns the file system.
- `extract` plans every write before it writes anything. It refuses the whole
  run when a planned path would leave the output directory, or when a symbolic
  link already sits at a planned path.
- An interrupted run leaves a part written directory. DeForm6 claims no
  atomicity.
- Length and equality are measured in encoded bytes, never in code points and
  never in grapheme clusters.

</td>
</tr>
</table>

### The confidence vocabulary

DeForm6 grades every recovered fact with one of three words: `proven`,
`inferred`, `unrecoverable`.
These are the only three words the JSON report's `confidence` field can hold.
There is no fourth tier and no numeric score.

| Word | Meaning |
| ---- | ------- |
| `proven` | The exact byte this run read names the fact directly. |
| `inferred` | This run chose the fact because the file gives no other answer, and the item's own `basis` field says so. |
| `unrecoverable` | This run could not recover the fact at all. |

### What it will not do

Three limits are deliberate, so that meeting one reads as a decision rather
than as something unfinished.

DeForm6 does not recover statements. The forms, the control trees, the property values, the names, and the procedure signatures come back. The code inside a procedure does not.

DeForm6 does not produce Basic source that a compiler accepts as a proof of
the original program's behaviour.
Every recovered item carries a confidence word, and a confidence word is not a
promise of correctness.

DeForm6 does not open the Visual Basic 6 IDE and it does not compile anything. No sentence in this file states a recovery figure as a share of one hundred. Every capability sentence in this file traces to a report field, a confidence word, or a limit the report states in its own words.

[`scripts/check-claim-surface.sh`](scripts/check-claim-surface.sh) holds those
three rules as a test.
It scans this file, the `--help` output and the report vocabulary for six
shapes of forbidden claim, and on every run it plants a violation of every
shape into every surface and requires the scan to catch each one.
A check that passes because it reads the wrong place is worse than no check.

---

## Three facts to read before you run it

1. **The P-code branch is untested.**
   `ProjectInfo.lpNativeCode` decides whether DeForm6 reports a project as
   native code or P-code, and nothing else does. Every program in the test
   corpus this project measures against carries `CompilationType` zero in its
   own `.vbp` file, which is native, so `lpNativeCode` is non-zero in every
   one of them. The P-code branch of that field has never run against a real
   P-code program. This states what was run, not what is supported:
   DeForm6 reads `lpNativeCode` and branches on it, and only the native
   branch has a real program behind it.

2. **`corpus/vb6-code/Hidden-Markov-model/frmHMM.frx` is damaged upstream and
   is excluded by name.**
   The file is 56 bytes, and its own record header declares 56 bytes of
   payload, but only 55 are present, because the upstream repository sets
   `text=auto` and git's line ending normalisation silently removed one
   carriage return. `HMM.exe` holds the same string as a whole, correct
   record, and is the second, independent source of truth for this fault.
   DeForm6's own test corpus excludes this one file by name, rather than
   papering over the missing byte.

3. **Full recompilation was not tested.**
   Full recompilation did not run. It needs the Visual Basic 6 IDE on
   Windows, and this run had neither. A structural check ran in its place,
   and it never opened this project in the IDE.

---

## Quick Start

You need the Rust toolchain the repository pins, which is 1.97.1.

```
git clone https://github.com/Stiven-Gjekaj/DeForm6
cd DeForm6
cargo build --release
```

Read one executable and print what DeForm6 found in it:

```
deform6 inspect <path-to-exe>
```

Read one executable and write a Visual Basic 6 project directory that VB6 can
open:

```
deform6 extract <path-to-exe> -o <output-directory>
```

Run everything the gate runs, in one command:

```
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace
```

DeForm6 gives one of six exit codes.

| Code | Meaning |
|---|---|
| 0 | The file was read. |
| 1 | Not a PE file. |
| 2 | A PE file, but it holds no Visual Basic runtime. |
| 3 | Visual Basic, but not version 6. |
| 4 | Visual Basic 6, but damaged. |
| 5 | An internal error, including a usage error. |

---

## How it works

### The path a file takes

```
one Visual Basic 6 executable
  -> the PE reader resolves the entry point through the section table
  -> the entry stub leads to the VB header, and the runtime DLL name
     separates VB6 from VB5
  -> the object table gives the objects, their kinds and their names
  -> the prototype walk gives the procedure signatures
  -> the GUI table gives the forms, and each form gives its control tree
  -> the property walk gives the property records and the resource blobs
  -> every write is planned, and the whole run refuses before it writes
     anything if one planned path escapes the output directory
  -> the project directory and one JSON report are written
```

### Two modes

`--salvage` changes what a recoverable defect costs.

| Severity | Strict | Salvage |
| -------- | ------ | ------- |
| `Fatal` | Refuses the run | Refuses the run |
| `Recoverable` | Refuses the run | Loses that one item and continues |
| `Tolerated` | Loses that one item and continues | Loses that one item and continues |

Both modes collect the same defect list, because the whole list is recorded
before the run refuses on any of it.
A strict run that stopped at the first error would only ever show a prefix of
it.

### The stack

| Layer | Choice |
| ----- | ------ |
| Language | Rust 2024, pinned at 1.97.1 |
| Memory safety | `#![forbid(unsafe_code)]` across the workspace |
| Arithmetic | `checked_add` on every file derived offset, with `clippy::arithmetic_side_effects` denied |
| Slicing | A bounded `Region` reader with no infallible accessor, with `clippy::indexing_slicing` denied |
| Panics | `unwrap`, `expect` and `panic` denied on any path that touches file derived data |
| Library | `crates/deform6`, which never opens a file |
| Command line | `crates/deform6-cli`, which owns the file system |
| Build tasks | `crates/xtask`, including the licence audit that writes `LICENSES.md` |
| Fuzzing | `cargo-fuzz` over the parser, run by its own workflow |

---

## The numbers

The numbers below are measured, not estimated.
Each one is the number a pinned file or a pinned constant states, and the gate
test named beside it, or above its table, asserts it on every run, never a
single derived figure calculated from a part.

| Measured | Against | Asserted by |
| -------- | ------- | ----------- |
| 44 Visual Basic 6 programs in the test corpus | | `cargo test -p deform6 --test corpus_sweep` |
| 185 procedure signatures recovered | 904 declared | `cargo test -p deform6 --test ratios` |
| 52 forms recovered | 53 declared | `cargo test -p deform6 --test ratios` |
| 686 controls recovered | 686 declared | `cargo test -p deform6 --test ratios` |
| 807 property records recovered, of which 136 written lines reach the `.frm` | | `cargo test -p deform6 --test ratios` |

One corpus form refuses.
The refusal names the byte offset and the byte the code expected to find
there, and a refusal is the correct result, because the tool does not print a
tree it cannot prove.

A single recovered `Position` record becomes four written lines and a single
`Font` record becomes seven, so the property pair is a coverage count for
DeForm6's own writer, not a recovery count against source.

### What the reader reproduces

DeForm6 writes each structure back over the place it was read from and
compares the result with what is there.
A byte that goes back unchanged is one the reader reproduces.
`cargo test -p deform6 --test byte_fidelity` asserts every row.

| Structure | Bytes reproduced | Record length | Records |
| --------- | ---------------- | ------------- | ------- |
| VB header | 50 | 104 | 44 |
| `ProjectInfo` | 20 | 572 | 44 |
| GUI table entry | 8 | 80 | 53 |
| `Object` | 20 | 48 | 105 |
| `ObjectInfo` | 6 | 56 | 105 |
| `PrivateObj` | 16 | 64 | 97 |
| `OptionalObjectInfo` | 8 | 64 | 97 |
| `ControlInfo` | 16 | 40 | 706 |

No reproduced byte differs from the file in any of the 1251 records, and no two
records claim the same byte.

This is a coverage figure and not a proof of correctness.
A field that is read at one offset and written back at the same offset cannot
disagree with itself.
The information is in the bytes that no field of the model claims at all.
`docs/STRUCTURES.md` names most of them, so they are mostly work not yet done.
Some are real unknowns: the GUI table entry holds several words that no
source names.

### What the reader returns against what the file declares

Most of the reader's clamps bound a loop and change no byte, so the table
above cannot see them.
A clamp makes the reader return fewer items than the file declares, so a
second measurement counts both.
`cargo test -p deform6 --test array_census` asserts every row.

| Array | Declared | Returned |
| ----- | -------- | -------- |
| GUI table entries | 53 | 53 |
| Objects | 105 | 105 |
| `ControlInfo` entries | 706 | 706 |
| Event slots | 11862 | 11862 |

No array in any corpus program comes up short.
The same test changes a file in memory to make each kind of shortfall happen,
and requires the count to name it.

The 706 `ControlInfo` entries are not controls.
`ControlInfo` is the table that binds controls to their events, and the 686 in
the first table count the nodes in the control tree, which is a different
quantity.

---

## Known limits still open at release

A gap that is still open at release is a documented limit, not a silent one.
The rows below come from two surveys: [`docs/STRUCTURES.md`](docs/STRUCTURES.md)
section 11, the structure gap register, and
[`docs/FILE-FORMATS.md`](docs/FILE-FORMATS.md) section 9, the file format gap
list.

### From the structure survey

| Limit | What is not known | What the tool does instead |
|---|---|---|
| S-02 | Which bit of `fObjectType` reliably marks that the optional half of `ObjectInfo` is present. Two published tests disagree with the corpus. | The tool tests bit `0x2`, the only bit that separates a standard module from every other kind across all seventeen tabulated values. |
| S-03 | No source names the raw `fObjectType` value an MDIForm carries. | The tool classifies any value outside the three the corpus proves as `Unknown`, and carries the raw number forward in the report instead of guessing a name. |
| S-04 | How a `ParamArray` argument modifier is encoded. No corpus source declares one. | The tool never emits the `ParamArray` modifier. The argument decodes under its ordinary type code, because the documented modifier bits are indistinguishable from a plain `ByRef Variant` argument. |
| S-05 | What Visual Basic type sixteen of the type code positions name: `0x00`-`0x02`, `0x04`, `0x07`, `0x09`, `0x0E`, `0x11`, `0x12`, `0x14`-`0x1A`. None occurs in the corpus. | The tool reports the type as `Unknown` and carries the raw byte forward, rather than guessing a name. |
| S-06 | Which of two disputed `FuncTypDesc` header layouts is correct. | The tool checks a fixed marker word at this position on every record, and reports a defect instead of building a prototype when the marker does not match, rather than parsing the disputed alternate layout. |
| S-07 | Where the `optionalVals` pointer's value comes from, and what governs it at the source level. | The tool walks a length prefixed run of default value records behind the pointer, and stops the walk and reports that record's own defaults as unrecoverable when it meets a value tag it does not recognise, rather than guessing a width. |
| S-08 | How wide one entry of the `PubVarDesc` array is, so the array can be walked. No stride hypothesis converges against the corpus. | The tool carries the count field forward as an unexplained open question in the report, and does not walk the array at all. |
| S-09 | How to recover the source level name of an event slot. No corpus program carries a single `EventDesc` record. | The tool walks the event pointer array and reports the address of each entry, and decodes no name from any of them. |
| S-10 | How a source level `Alias "#123"` ordinal import is encoded in the compiled file. | When the Visual Basic level name is not present in the file, the tool reports the export as an inferred ordinal number, rather than guessing a name. |
| S-12 | Whether the byte at control block offset `0x02`, named `uni` by one prior tool and never read by it, is a Unicode flag for the name that follows. | The tool does not read this byte into any field. It is skipped as part of the fixed header the tool does not interpret. |
| S-13 | The exact character range and code page a written string obeys. | This run assumes a Western code page. Every character above U+00FF was replaced with a question mark and reported, never guessed at a different code page. |
| S-14 | Whether a menu that is itself a sibling within an already open menu group, opening its own child, reads the same as a confirmed sibling case; and one separate case where an expected separator byte is not present at all. | The tool decodes the two level deep menu close transitions measured across five transitions in two programs, and reports a defect naming the byte offset when the walk meets a byte that is not the expected separator, rather than guessing an ambiguous depth. |
| S-15 | The property name behind most control type and opcode pairs. | The tool ships a small, safe provenance "Opcode table" built into the binary, states its own size in the report's limits array, and reports a property it cannot name as present and not decoded, rather than guessing a name. A user can supply a larger table built on their own lawful Visual Basic 6 install. |
| S-16 | What the nine unknown dwords in `GUIObjectInfo` hold. | The tool does not read these bytes into any field, and reports nothing from them. |
| S-17 | What the five unknown dwords in the external component entry hold. | The tool decodes only the two fields it uses, `oUuid` and the textual GUID offset and length. The other fields carry no struct field, and the tool reports nothing from them. |
| S-18 | Whether an entry point opcode other than the one all 44 corpus executables share is a valid, undocumented variant. | The tool accepts only the one push immediate opcode the whole corpus shares, and refuses any other opcode with an error naming what it expected at the entry point. |

### From the file format survey

| Limit | What is not known | What the tool does instead |
|---|---|---|
| F-01 | The exact character count where the IDE moves a string property from the `.frm` into the `.frx`. The corpus bounds it between 99 and 153 characters but does not fix it. | The tool always writes the string inline when it is 97 characters or shorter and holds no line break, the proven floor of the open range. |
| F-02 | Whether a second, healthy multi line `TextBox` sample would confirm the reconstructed `u8` count, or point to a different width. | The tool writes the `$` form with a `u32` count, proved exactly by one corpus file, for every string that must go to the `.frx`. |
| F-03 | Whether the loader tolerates a missing `Attribute` line. No corpus file omits one, and no document states a requirement. | The tool writes all five `Attribute` lines, always. |
| F-04 | Whether `.vbp` key order matters. Not documented, and the one independent parser the survey checked does not care. | The tool matches the order the IDE itself writes. |
| F-05 | The `List` record layout for `ComboBox` and `ListBox`. No `ComboBox` or `ListBox` in the corpus stores its `List` property in a `.frx`, so the record layout one prior tool documents is unverified. | The tool reports a `List` property as present and not decoded, the same generic fallback it uses for any property with no proven table entry, rather than writing the unverified layout. |
| F-06 | A second record shape one prior tool documents for small to medium text, including an off by one bug that tool warns about. Unverified against this corpus. | The tool never writes this record shape. Every string routed to the `.frx` uses the one shape the corpus proves, the `$` form with a `u32` count. |
| F-07 | How DeForm6 should choose a code page for a Japanese or Cyrillic Visual Basic 6 project. Every corpus file is Western. | The tool assumes a Western code page on every run, and replaces every character above U+00FF with a question mark, reporting that it did so, rather than guessing a different code page. |
| F-08 | The `.ctx` record layout, assumed identical to `.frx` and not checked. This matters only when ActiveX control projects come into scope. | The tool does not read or write `.ctl` or `.ctx` files. ActiveX control projects are out of scope for this milestone, per `PROJECT.md`. |
| F-09 | Whether an MDIForm project, root class `VB.MDIForm` and a child form carrying `MDIChild = -1`, reads the way the one cited source states. No corpus program is an MDIForm project. | The tool classifies an MDIForm's `fObjectType` value as `Unknown`, the same fallback S-03 describes, and writes whatever property values it reads with no MDIForm-specific handling. |
| F-10 | Whether the VB6 IDE ever wrote a comma decimal separator, such as `Size = 8,25`, on a non-English machine. The corpus is English only. | The tool always writes a period as the decimal separator, using Rust's own locale independent number formatting, regardless of the host machine's locale. |

Sixteen rows come from the structure survey and ten rows come from the file
format survey: twenty six limits, still open at release, each with the
default the tool chose in their place.
---

## Project structure

```
crates/
  deform6/          the library: it never opens a file
    src/read/       the PE reader and the bounded Region type
    src/vb/         the VB6 structures: header, objects, prototypes, forms
    src/write/      the project writer: .vbp, .frm, .frx, .bas, .cls
    src/report.rs   the JSON report: items, defects, limits
    src/error.rs    the defect vocabulary and its three severities
    src/journal.rs  the one place a severity decides whether a run continues
    schema/         the JSON Schema every report is validated against
    tests/          18 integration suites, including the corpus sweep
  deform6-cli/      the command line: it owns the file system
  xtask/            build tasks, including the licence audit
docs/               the reverse engineering surveys the source cites
scripts/            the walls: each one proves a property and fails loudly
corpus/             44 Visual Basic 6 programs, with sources, and a manifest
tests/ratios.toml   the pinned recovery figures the gate asserts
```

Two ideas carry the design.

**The bounded region.**
`Region` is the only way the library reads bytes, and it has no infallible
accessor.
A read that runs past the end of what the file actually holds returns an
error rather than a panic or a wrong value.
This is what lets `#![forbid(unsafe_code)]` and the clippy deny wall hold
across a parser that reads hostile input.

**One place decides whether a run continues.**
`Journal::record` holds one arm for each pair of severity and mode, six in
all, and no wildcard arm.
A new severity stops the build until somebody decides its policy in both
modes.
Every defect the run collected passes through it, in the order it was
collected.

---

## Testing

```
cargo test --workspace
```

1137 tests run. The suite includes:

- **The corpus sweep.** All 44 programs are read and report what they hold.
- **The structural check.** Every extracted project is checked against the
  shape the IDE expects.
- **The schema check.** Every one of the 44 reports is validated against
  `crates/deform6/schema/report.schema.json`, and a doctored report is proved
  to fail.
- **The ratio gate.** Every pinned figure in `tests/ratios.toml` must hold
  unchanged.
- **The no-panic proof.** Every corpus input is swept through both modes, the
  writer and the byte fidelity walk. Every public entry point that takes
  untrusted bytes belongs in that sweep.
- **The hostile corpus.** Mutated and crafted inputs, including a form count
  of `0xFFFF` in a 4096 byte file, which must allocate nothing.
- **The byte fidelity map.** The eight structures in the table above are
  written back over the bytes they were read from and compared against them,
  over all 44 programs. The bytes the reader reproduces and the bytes no field
  of it claims are both pinned.
- **The array census.** For the four arrays in the table above, the count the
  file declares is compared with the number of items the reader returns, and
  the count is read back from the file at the offset the census names.
- **The walls.** `scripts/prove-*.sh` each prove one property, by planting a
  violation and requiring the check to catch it.

The fuzz target lives in `crates/deform6/fuzz` and runs in its own workflow.
It is excluded from the stable workspace, so `cargo test --workspace` never
builds it.

---

## Contributing

Read [`AGENTS.md`](AGENTS.md) first. It states the rules this repository holds
itself to, and they are not style preferences:

- No `unsafe`, anywhere.
- `checked_add` on every offset derived from a file. No bare arithmetic.
- No `unwrap`, `expect` or `panic` on a path that touches file derived data.
- A fact the tool cannot prove is reported as unproven. It is never guessed.
- A new claim in this file needs a measurement behind it, and
  `scripts/check-claim-surface.sh` is the test that says so.

The gate is nine steps and every one has to pass. Run them all with one
command, which is what `.github/workflows/gate.yml` runs, in the same order:

```
sh scripts/gate.sh
```

An earlier version of this file said the gate was three commands. It is not,
and the three it named leave out `cargo doc`, which denies a broken or private
documentation link and which neither `cargo clippy` nor `cargo test` reports.
`scripts/gate.sh` reads the workflow and refuses to run when the workflow names
a check the script does not, so the two cannot drift apart in silence.

---

## License

MIT. See [`LICENSE`](LICENSE).

Every third party licence in the dependency tree is audited into
[`LICENSES.md`](LICENSES.md), which is re-derived from `cargo metadata` rather
than written by hand.
