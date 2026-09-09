# Phase 3 context: Forms

**Captured:** 2026-09-09, after the phase 3 research completed.

This file holds the decisions the human took after reading
`03-RESEARCH.md` and before the planner ran. It also holds two measurements
this session made that no earlier document holds. Read `03-RESEARCH.md` for
the evidence behind each one.

## D-01: `ROADMAP.md` named risk 15 does not stand. `AGENTS.md` wins

`ROADMAP.md` tells plan 03-02 to build the opcode-to-property table from a
type library dump and to commit it as derived data.

`AGENTS.md`, "What may enter this repository", says: "No binary, no source
file, and no derived fixture from a third party system that the author does
not own may enter this repository. This includes a fixture that was
calculated from such a file."

A table calculated from Microsoft's `VB6.OLB` is such a fixture. The two
documents disagree. `AGENTS.md` is the binding rule. The roadmap instruction
is withdrawn.

**Plan 03-02 delivers three things and no table.**

1. **The derivation tool, committed. The table, never committed.** The tool
   is DeForm6's own original code. It produces the table. It is not the
   table. Its output goes to a path that `.gitignore` excludes. This follows
   the pattern `AGENTS.md` already states for other third party bytes:
   commit the manifest, never commit the bytes.
2. **The table format and the run time loader.** `deform6 inspect
   --opcode-table <path>` reads a table that the user built on their own
   machine from their own lawful VB6 install. DeForm6 ships with no table.
3. **The safe-provenance subset, transcribed.** `STRUCTURES.md` §8.5.1
   already holds a partial table for Form, CommandButton, Label and ListBox.
   Its citation says it came from reading Semi VB Decompiler's authored
   source comments, not from reading `VB6.OLB`. `AGENTS.md`'s Prior Art rule
   permits this: read the other tools for facts, do not copy their code.
   Extend the subset the same way and cite the exact function name for each
   fact. This is about 40 of the 198 pairs.

**Never open, copy or derive from the copy of `VB6.OLB` that Semi VB
Decompiler commits to its own repository.** That the author of that tool
chose to take the risk is not evidence that the choice is safe.

**Every property outside the safe subset reports honestly.** Present, byte
offset given, opcode number given, name and value undecoded, no table
supplied. These are the same words FRM-04 already requires for a third party
OCX control's property blob. A property this phase cannot name is the same
kind of gap, and the report says so in the same way.

**The worklist is 198 pairs, not the whole VB6 API.** The research counted
every distinct `(control type, property name)` pair that the corpus `.frm`
files actually set: 198 pairs across 20 control types. The differential gate
can only score a property that the `.frm` text form names, so 198 is the
real target. Roughly 158 of them stay open after plan 03-02.

**The rest is follow-up work behind a human checkpoint.** Each remaining
pair is closeable by the method `STRUCTURES.md` §13 already used to close
gap 1: compile a small original program that sets one property, and diff the
compiled bytes against a blank control. A program the human writes and
compiles is the human's own work, so it may enter `corpus/` with a
`corpus/NOTICES` entry. That campaign needs a human at a working VB6
install. It does not belong inside phase 3.

## D-02: the event name table takes the same discipline

`STRUCTURES.md` §8.6 names a second derived table: which event ordinal maps
to which event name. The same rule applies. Build it from independent public
sources and cite each fact. Do not dump `VB6.OLB`. The event *names* are
common knowledge and appear in many freely licensed references. The *vtable
ordering* is not commonly published, so treat the ordering with the same
"commit the tool, not the table" discipline as D-01.

## D-03: plan 03-04 writes the reference document corrections

Plan 03-04 already owns the control tree, the control names and the control
array index, so it consumes the gap 11 closure. It also records it.

**Gap 11 is closed.** The control array `Index` sits at control block offset
`0x05`. The research confirmed this across 30 array elements in 2 files, with
values 0 to 24, including one 25 element array. `STRUCTURES.md` gap 11 moves
to closed with this evidence.

**Two `GAPS.md` corpus counts are wrong.** Both came from a `bash` glob
(`corpus/**/*.frm`) that misses nested directories when `globstar` is off.

| Claim in `GAPS.md` | True value |
|---|---|
| 0 third party OCX instances | **3**, all `MSWinsockLib.Winsock` |
| control arrays in 35 elements | **48 elements in 6 files** |

Both true values are confirmed independently this session.

## D-04: VER-06 excludes the `.frx` only, never the `.frm`

`corpus/vb6-code/Hidden-Markov-model/frmHMM.frx` is corrupted upstream by
git's `text=auto` line ending normalization. The research proved this byte
for byte against the compiled executable. The fault is not DeForm6's and it
is not a parsing question.

`frmHMM.frm` is intact. It holds 14 of the corpus's 48 `Index` lines, which
is 29 percent of the evidence that closes gap 11. **It stays in the
differential gate.** The exclusion stays as narrow as the proven fault.

**DeForm6 needs the same fix in its own `.gitattributes`** before any `.frx`
fixture enters this repository. A `.frx` is a binary file. Git must not
normalize it.

## Two traps this session measured, which no other document holds

Both are the failure `AGENTS.md` names: "a search for some names is not a
search for all of them." Both make the phase report a form ratio that looks
right and is not.

### One corpus form does not have a lower case extension

`corpus/public-domain/SK-MCI-Sample__VB6/MCI.FRM` is one of the 54 forms. So
is `MCI.VBP` one of the 45 project files.

`crates/deform6/tests/support/vbp.rs` already handles this. Its
`walk_by_extension` compares with `eq_ignore_ascii_case`, and its own comment
already records 45 project files and 44 executables. **`support/frm.rs` must
walk the same way.** A new reader that matches `*.frm` exactly finds 53 of
54 forms and reports a full pass over a set that is missing one.

### One corpus form is not valid UTF-8

`corpus/vb6-code/Threshold-effect/Threshold.frm` holds byte `0xA9` at offset
5669. The file is ISO-8859, not ASCII.

`fs::read_to_string` fails on this file. `grep` without `-a` treats it as
binary and prints nothing, which is how this session first missed its
`Begin VB.Form` line.

**`support/frm.rs` reads bytes, never a `String`.** It decodes with an
explicit encoding and states which one, the same rule `VbStr` follows on the
binary side. A reader that skips a file it cannot decode as UTF-8 scores 53
of 53 and hides the one it dropped.

## What the phase gate is

`AGENTS.md` states it. Every one of these, before every commit, not a
selection.

    cargo fmt --all --check
    cargo clippy --all-targets -- -D warnings
    cargo test --workspace

`cargo test --workspace` is the build check, never `cargo build`.

## What the file is

`AGENTS.md`, "The file is hostile", binds every parser in this phase. No
panic on any input. No `[]` on a slice. No `unwrap` and no `expect` on a
value from the file. No allocation sized from a length field before that
length is checked against the real file size. No `+` on two offsets from the
file; use `checked_add`. Every error names the byte offset and what the code
expected to find there. A crash the fuzzer finds becomes a test case here.

## What a test may hold on to

A round trip through DeForm6's own writer and DeForm6's own parser is not
verification. It proves only that the code agrees with itself. The
differential gate compares against the committed `.frm` source through
`support/frm.rs`, a second independent reader.

Measure the ratio against the original source the executable was built from.
Never measure it against what DeForm6 produced.

When you add a test, break the thing it covers on purpose and watch it fail.
