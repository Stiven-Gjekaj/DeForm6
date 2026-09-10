# Phase 3: Forms - Research

**Researched:** 2026-09-09
**Domain:** The VB6 GUI table and form data stream: the control tree, control
types and names, property values, `.frx` resource blobs, the control array
index, and the event handler table. Also the licence question the roadmap
raises for the two derived data tables the phase needs.
**Confidence:** HIGH on every claim tagged `[VERIFIED: local]` below. Each one
is the output of a script this session ran against real corpus executables,
byte for byte, with the file offset shown. LOW, and marked so, on the parts of
`STRUCTURES.md` section 8 and 9 that stay `[G]` after this session's
measurement, because the format itself is silent there, not because nobody
looked.

## Summary

This phase has two problems that are not parsing problems. The first is a
licence conflict inside the project's own documents. `ROADMAP.md` tells plan
03-02 to build the opcode-to-property table "from a type library dump and
commit it as derived data." `AGENTS.md` says: "No binary, no source file, and
no derived fixture from a third party system that the author does not own may
enter this repository. This includes a fixture that was calculated from such
a file." A table calculated from Microsoft's `VB6.OLB` is exactly that. This
document resolves the conflict in favour of `AGENTS.md`, because `AGENTS.md`
is the binding rule of this repository and the roadmap's wording is not a
licence opinion, it is a plan a human wrote before this tension was checked.
The resolution below gives a path that stays inside `AGENTS.md`, and it
changes what plan 03-02 delivers.

The second problem is `STRUCTURES.md` gap 14, the scope separator grammar,
which the roadmap correctly names as the least certain part of the whole
format. This session ran a byte-level walk of two real corpus forms, one with
no children and one with three, against the property stream at
`GUIObjectInfo.lPropertiesLength`. The walk shows the grammar is genuinely
uncertain in the way `STRUCTURES.md` says, and it also shows the one fact the
roadmap needs to make the tiling check work: every control block carries its
own `Length` field, and that field alone is what lets a parser bound one
control's bytes and move to the next scope-separator run without first
understanding the tree shape those bytes describe. The tiling check is not a
nice-to-have added after the tree is built. It is the mechanism that lets the
tree be built honestly in the presence of an unproven grammar.

This session also closed a `STRUCTURES.md` gap the roadmap did not expect to
close: gap 11, the control array index location. Reading five real array
elements across two files, two control types, and array sizes of 2, 3 and 25,
the single byte at control-block offset `0x05` (in the array header layout)
tracks the source's `Index =` value exactly, in every one of the 30 elements
measured. This is not a proposal. It is a corpus-proven offset, with the hex
evidence below, and plan 03-04 can implement it directly rather than treating
it as open.

Two corpus counts in `GAPS.md`, the phase 2 gap audit, are also corrected
here, both from the same class of mistake: a shell glob that did not expand
the way its author expected. `GAPS.md` reports zero third party OCX control
instances in the corpus. There are three, `MSWinsockLib.Winsock`, across
three files, one of them inside a control array. `GAPS.md` reports control
arrays in 35 files. The real number, counted with the exact property name
rather than a substring that also matches `TabIndex` and `ListIndex`, is six
files and 48 elements. `CORPUS.md`'s own "48 controls carry an `Index`
property" figure was already right; only `GAPS.md`'s file count was wrong.

**Primary recommendation.** Do not implement the scope separator grammar as a
guess with no check. Implement it exactly as `STRUCTURES.md` recommends: read
`0xFF`, then scope bytes until one is greater than 3 or equal to 0, treat `02`
and `03` as pops, and after the walk finishes assert that the number of bytes
consumed equals `GUIObjectInfo.lPropertiesLength` exactly. Refuse and name the
byte offset when it does not. Build the opcode-to-property table as an
external, optional, never-committed artifact plus the small subset this
project can already source safely (prior art read for facts, per `AGENTS.md`
Prior Art), and report every property DeForm6 cannot name as present-but-
undecoded, the same honest pattern FRM-04 already uses for OCX controls.

## Architectural Responsibility Map

Single library crate plus one CLI binary, same as phase 1 and phase 2. The map
below extends phase 2's with the phase 3 capabilities.

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| GUI table walk, `GUIObjectInfo`, tiling invariant | `deform6::vb` (library) | — | Pure structure reading, same pattern as `vb/object.rs` |
| Scope separator walk, control tree | `deform6::vb` (library) | — | Reads `Region` only; the tiling check is a `Defect`, not a panic |
| Control type, name, array index | `deform6::vb` (library) | — | Header fields, no external table needed |
| `VbStr`, the encoding-validating string reader | `deform6::vb` (library) | — | Shared by every property reader in the phase |
| Property stream decode (named properties) | `deform6::vb` (library) | *external, optional table* | The library owns the walk; the opcode-to-name table is supplied at run time, never compiled in as Microsoft-derived data |
| `.frx` blob extraction, image sniffing | `deform6::vb` (library) | — | Byte-signature sniffing only, no external table |
| External OCX CLSID join | `deform6::vb` (library) | — | Matches class name against the external component table already recovered in phase 2 |
| Event handler table, control-to-event join | `deform6::vb` (library) | *external, optional table* | Same shape as the property table: the join (control name to `ControlInfo`) is structural; the event *name* needs a table this phase cannot commit either, see below |
| `support/frm.rs`, independent `.frm` reader | test harness (test-only) | — | **Must not** call `src/`, same rule as `support/vbp.rs` |
| `frmHMM.frx` exclusion | test harness (test-only) | — | A named exclusion, not a generic predicate; see the dedicated section below |
| `inspect` text report | `deform6-cli` | `deform6::vb::Report` | Presentation only, unchanged pattern |

<user_constraints>
## User Constraints

No `CONTEXT.md` exists for this phase. The user chose to plan without one.
There are no locked decisions, no discretion notes, and no deferred ideas to
copy forward from a discuss-phase session. Everything this document
recommends is open to the planner's judgement, subject to `AGENTS.md`, which
is binding project policy and is treated below as if it were a locked
decision, because it is one.
</user_constraints>

## Project Constraints (from AGENTS.md)

`AGENTS.md`, not `CLAUDE.md`, is this repository's instructions file.
`./CLAUDE.md` does not exist. Every directive below is binding on the plans
this research feeds.

- **A human writes every commit.** No agent name, no co-author line, no
  session link, in any commit message this phase produces.
- **Simplified Technical English everywhere.** Source, comments, docs, commit
  messages, examples. Short sentences, active voice, present tense, no
  em-dash, no emoji. This applies to code comments the phase 3 parser writes,
  not only to this document.
- **One change per commit.** Split a feature into steps. Code and its tests
  in the same commit. Documentation in a different commit.
- **The gate, every time, in order:** `cargo fmt --all --check`, then
  `cargo clippy --all-targets -- -D warnings`, then `cargo test --workspace`.
  `cargo test --workspace` is the build check. `cargo build` does not compile
  a `#[cfg(test)]` module and misses a broken test helper.
- **The file is hostile.** No panic on any input. No slice index with `[]`;
  use `get`. No `unwrap` or `expect` on a file-derived value. No allocation
  sized from a file length field before that length is checked against the
  real file size. No `+` on two file-derived offsets; use `checked_add`.
  Every error names the byte offset and what the code expected there. This
  applies with full force to phase 3: the scope byte run, the control block
  `Length` field, `wEventCount`, `dwControlCount`, and every property payload
  width are all attacker-controlled and are exactly the shape of field this
  rule exists for.
- **Fuzzing is part of the gate, not an extra.** A crash the fuzzer finds
  becomes a committed regression test. Phase 5 owns the fuzz harness itself,
  but phase 3's parser is written to the same no-panic standard from the
  first line, matching the project's own established practice in phases 1
  and 2.
- **What may enter this repository.** A program enters `corpus/` only under a
  redistribution licence, recorded in `corpus/NOTICES`. Everything else is
  fetched at run time from a manifest of pinned hashes; the manifest is
  committed, the bytes never are. **No binary, no source file, and no derived
  fixture from a third party system that the author does not own may enter
  this repository. This includes a fixture that was calculated from such a
  file.** This is the rule the ROADMAP.md wording for plan 03-02 conflicts
  with. See the dedicated section below.
- **Measurement.** The recovery ratio is measured against the original source
  the executable was built from, never against DeForm6's own output. A test
  pins the ratio per program; a fall fails loudly, a rise prints the number to
  paste.
- **What a test may hold on to.** A round trip through DeForm6's own writer
  and its own parser is not verification; it proves only self-agreement. When
  a test is added, break the thing it covers on purpose and watch it fail
  first. Phase 2's own verifier found a test-shaped hole this rule exists to
  catch (`is_plausible_identifier`, `VERIFICATION.md` finding 1); phase 3
  should expect the same class of gap in the scope-byte walk, precisely
  because the grammar is a heuristic and a heuristic is where a test that
  cannot fail hides best.
- **Prior art.** Read other tools, Semi VB Decompiler named specifically, to
  learn what a structure holds. Do not copy their code. SVBD has no licence
  file, so all rights in it are reserved. `STRUCTURES.md` section 8 already
  does this correctly: it cites SVBD's field names and its special-cased
  opcode facts, in prose, with attribution, never a code excerpt.

## Derived Data and the AGENTS.md Redistribution Rule

This is the single most important finding in this document, because it
changes what FRM-03 and part of FRM-06 can deliver, and no later phase can
undo a plan built on the wrong assumption.

### The conflict, stated plainly

`ROADMAP.md`'s Phase 3 named risks say, about `STRUCTURES.md` gap 15:

> Semi VB Decompiler reads `VB6.OLB` over COM at run time, which DeForm6
> cannot do and has no right to redistribute. The table must be built once
> from a type library dump and committed as derived data.

`AGENTS.md`, "What may enter this repository," says:

> No binary, no source file, and no derived fixture from a third party
> system that the author does not own may enter this repository. This
> includes a fixture that was calculated from such a file.

`VB6.OLB` is Microsoft's compiled type library, installed with the VB6 IDE.
A table of `(control type, opcode, property name, VB type)` tuples read out
of it by a tool such as `oleview.exe` is a fixture calculated from that file,
whether the person running the tool personally owns a lawful VB6 install or
not. "Owns" a licensed copy of Windows and Visual Studio 6 is not the same
thing as owning the copyright in Microsoft's type library, and nothing in
`AGENTS.md` reads "unless the author owns a licence to run it." The plain
reading of the rule bars committing this table to the repository, full stop.
`ROADMAP.md`'s "committed as derived data" instruction cannot stand as
written. This document says so directly rather than quietly building the
table and hoping nobody asks.

### The legal reasoning, and where it runs out

This is not legal advice, and DeForm6's own docs should not present it as
settled. What can be said with reasonable confidence, cited:

- *Sega Enterprises Ltd. v. Accolade, Inc.*, 977 F.2d 1510 (9th Cir. 1992),
  and Article 6 of the EU Software Directive both support the position that
  disassembling or otherwise reverse engineering a program to discover
  functional interface information for interoperability is lawful, and that
  the underlying functional facts (what an interface requires) are not
  themselves protected by copyright. `[CITED: 9th Circuit opinion; EU
  Directive 2009/24/EC Art. 6]`
- That doctrine supports *reading* `VB6.OLB` to learn facts, the same
  posture `AGENTS.md`'s own Prior Art section already takes toward SVBD's
  source. It does not, on its own, establish a right to *redistribute* a
  systematic, near-complete extraction of Microsoft's own type library
  content as a data file. The *selection and arrangement* of hundreds of
  `(opcode, name, type)` tuples across every intrinsic control type, taken
  wholesale, is a different act from citing a handful of facts in prose with
  attribution, and it is the act `AGENTS.md`'s rule is written to prevent.
  `[ASSUMED]` this reading is more conservative than some published reverse
  engineering practice, and it is the reading this document uses because
  `AGENTS.md` is unambiguous and binding.
- SVBD's own GitHub repository (`pmachapman/semi-vb-decompiler`) commits a
  copy of `VB6.OLB` directly. `[VERIFIED: github.com/pmachapman/semi-vb-
  decompiler, this session, file present at path VB6.OLB]` This is evidence
  of what SVBD's author chose to risk, not evidence that the choice is safe,
  and `AGENTS.md`'s Prior Art section already tells DeForm6 not to imitate
  SVBD's code. The same reasoning extends to not imitating this choice.

### What the pinned-hash manifest mechanism enables instead

`AGENTS.md` already describes the shape of the answer, for a different case:
"Everything else is fetched at run time from a manifest of pinned hashes.
Commit the manifest. Never commit the bytes." Phase 5's plan 05-07
(`corpus/manifest.toml`, `xtask fetch-corpus`, `corpus/fetched/` kept out of
the repository) is the working precedent inside this project's own roadmap.
The opcode table generalises the same pattern:

1. **Commit the tooling, never the table.** A small, reviewable derivation
   utility (documentation of the expected `VB6.OLB` layout, or a script that
   walks its `ITypeLib` COM interface) is DeForm6's own original code and may
   be committed freely. It produces the table; it is not the table.
2. **The table is a local build artifact, not a repository file.** A
   developer who owns a lawful VB6 install runs the tool once, locally, and
   the output goes in a path this project's own `.gitignore` excludes, the
   same way `corpus/fetched/` stays out of git in phase 5's plan.
3. **DeForm6 ships with no opcode table baked in.** `deform6 inspect --opcode-
   table <path>` accepts a user-supplied table at run time. Absent the flag,
   every property this phase cannot name from the small safe-provenance
   subset below is reported honestly: present, at a byte offset, opcode
   number known, name and value undecoded, no table supplied. This is the
   same honesty pattern FRM-04 already requires for a third party OCX
   control's property blob. Phase 3's own intrinsic-control property values
   become, for everything outside the safe subset, exactly that same kind of
   gap, and the report should say so in the same words.
4. **A "clean clone" gate still passes.** `cargo test --workspace` on a
   machine with no local opcode table exercises the undecoded-property path
   and the safe-subset path, and both are real, committed test material.
   Nothing in the gate requires the full table to be present.

### The safe-provenance subset that already exists

`STRUCTURES.md` section 8.5.1 already contains a partial opcode table for
Form, CommandButton, Label and ListBox. Its own citation says it was built by
reading SVBD's *source code comments*, not by reading `VB6.OLB`: `ReturnGui-
Opcode`, `ReturnDataType`, and the hardcoded special-case branches SVBD's own
author wrote for the properties it does not resolve dynamically. That is
prior art read for facts, exactly as `AGENTS.md`'s Prior Art rule permits, and
it is safe to extend the same way: read more of SVBD's *authored source*
(never its committed copy of `VB6.OLB`, which must not be opened, copied, or
derived from even incidentally), transcribe the facts it states in its own
comments and hardcoded tables, and cite the exact function name per fact,
the same way `STRUCTURES.md` already does.

**How large the real worklist is.** This session counted every distinct
`(control type, property name)` pair that the corpus's own committed `.frm`
files actually set, across all 54 form files:

```
VB.Form: 26 distinct properties
VB.PictureBox: 22 distinct properties
VB.Label: 17 distinct properties
VB.TextBox: 16 distinct properties
VB.CommandButton: 13 distinct properties
VB.CheckBox: 11 distinct properties
VB.OptionButton: 11 distinct properties
VB.Frame: 10 distinct properties
VB.ComboBox: 10 distinct properties
VB.HScrollBar: 9 distinct properties
Font: 7 distinct properties
VB.ListBox: 7 distinct properties
MSWinsockLib.Winsock: 6 distinct properties
VB.FileListBox: 6 distinct properties
VB.DirListBox: 6 distinct properties
VB.DriveListBox: 6 distinct properties
VB.VScrollBar: 5 distinct properties
VB.Line: 4 distinct properties
VB.Timer: 4 distinct properties
VB.Menu: 2 distinct properties

Total: 198 (control type, property) pairs across 20 control types.
```
`[VERIFIED: local, this session, counted from every corpus .frm file's own
`Name = Value` lines]`

This is the bounded, corpus-relevant worklist, not the full VB6 API surface
(roughly 24 control types with up to 60 properties each). 198 pairs is what
the differential gate (VER-01 extended to forms) actually needs to score
against real committed source, because `support/frm.rs` can only check a
property DeForm6's recovered value against a property the `.frm` text form
actually names. Every one of the 198 pairs is, in principle, closeable by
`STRUCTURES.md`'s own established gap-closure method: **compile a small,
original test program that sets one property, and diff the compiled bytes
against a blank control.** This is not new. It is the exact method
`STRUCTURES.md` section 13 used to close gap 1, and the exact method its own
gap register recommends for gaps 4, 5, 10, 11, 12 and 13. A synthetic program
a human writes and compiles is the human's own original work; it is not a
fixture derived from a third party system, and it may be committed to
`corpus/` under whatever licence the human chooses, with a `corpus/NOTICES`
entry, the same as any other corpus program.

**What this means for scope.** 198 facts is still a large, deliberate,
manual undertaking that needs a human with a working VB6 install, and this
document cannot execute it: this research session runs on a machine with no
Windows and no VB6. Plan 03-02 should be scoped as: (a) transcribe the
already-known SVBD-facts subset (Form, CommandButton, Label, ListBox, roughly
40 of the 198 pairs) using the existing safe method, (b) ship the
opcode-table format and the `--opcode-table` loading mechanism, (c) leave the
remaining pairs as an explicitly tracked, honestly reported gap, and (d) treat
the full compile-and-diff campaign as follow-up work gated behind a
`checkpoint:human-verify` task, because it needs a human, a VB6 install, and
real time this research session cannot substitute for.

### The event name table has the same shape, on firmer ground

`STRUCTURES.md` section 8.6 names a second derived data table: which event
ordinal, in a control's default source interface, corresponds to which event
*name* (`Click`, `MouseDown`, and so on). The same `AGENTS.md` rule applies to
a table built by dumping `VB6.OLB`'s event interfaces. But this table sits on
materially firmer legal ground than the property table, for a concrete
reason: the *names* of an intrinsic control's events, and their fixed
declaration order, are not obscure. They are what every VB6 programmer sees
in the IDE's own code-pane dropdown, and they are repeated verbatim across
hundreds of independently authored, freely licensed tutorials and reference
pages that have nothing to do with Microsoft's compiled type library. A table
built by cross-referencing several such independent public sources, with each
fact's source cited, is a materially different act from dumping a Microsoft
binary, and it is far closer to the "fact, not expression" side of the line.
This document flags it as a comparatively lower-risk item, not a resolved
one: the planner should still treat it as needing the same "commit the
tool, not a `VB6.OLB`-derived table" discipline for the *ordering*, since
ordering-in-the-vtable is the one part that is not commonly published.

## Phase Requirements

| ID | Description | Research Support |
|---|---|---|
| FRM-01 | Recover the control tree with the correct parent for each control | The tiling-gate section below; the two byte-level walkthroughs; the scope-separator algorithm |
| FRM-02 | Recover the type and the name of every control | Control block header layout, `cType` table (already `[L]`, corpus-usable); the control array Index closure below |
| FRM-03 | Recover the property values of every control and of the form | The `VbStr` encoding-validation design; the opcode table conflict and its resolution above; the 198-pair bounded worklist |
| FRM-04 | Recover a third party OCX control's CLSID and state plainly the property blob needs the control's own type library | The corrected corpus count (3 real `MSWinsockLib.Winsock` instances, not 0); the `_ExtentX`/`_ExtentY`/`_Version` fixed header that is readable without a type library |
| FRM-05 | Recover the resource blobs | `.frx` inline-blob layout, offset-cursor synthesis, image-signature sniffing; the phase boundary note below (writing a `.frx` file is Phase 4's WRT-01, this phase recovers the blob bytes) |
| FRM-06 | Recover the event handler names bound to each control | `ControlInfo`, the event handler table, the event-name-table conflict and its firmer-ground resolution above |
| VER-06 | Exclude `frmHMM.frx` by name, with the reason recorded | The exact git-line-ending corruption story below, with the byte evidence |

## Standard Stack

No new external dependency. Phase 3 reads more of the same file with the same
`Region`/`Off`/`Rva`/`Va` primitives phase 1 built and phase 2 already reused.
Image-format detection (§8.8) is a first-bytes signature match
(`42 4D` BMP, `47 49 46` GIF, `FF D8` JPEG, and so on), which is a `match` over
a few bytes, not a job for an image-parsing crate; DeForm6 never decodes the
image, only classifies and copies it. GUID formatting (the external component
table's CLSID, and `ControlInfo.lpGuid`) is 16 bytes rendered as
`{8-4-4-4-12}` hex groups by hand, matching the project's existing "hand-roll
what is trivial, use a crate only for real complexity" posture; the workspace
carries no `uuid` crate today and phase 3 does not need to add one.

### Core

None new.

### Supporting

None new.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|---|---|---|
| Hand-rolled GUID formatting | `uuid` crate | The format is fixed, 16 bytes, one output shape (`{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`). A crate buys nothing this phase needs and adds a dependency for a five-line function. |
| Hand-rolled image signature sniff | `infer` or `image` crate | DeForm6 never decodes pixel data, only classifies the container format for the `.frx` writer (Phase 4) to preserve verbatim. A decoding crate is the wrong tool for a task that never decodes. |

**Installation:** none required.

## Package Legitimacy Audit

Not applicable. This phase installs no external package. No package
legitimacy check was run because there is nothing to check.

## Architecture Patterns

### System Architecture Diagram

```
 VBHeader (phase 1)
   | wFormCount, lpGuiTable
   v
 vb::gui::GuiTable::walk                         <- NEW (03-01)
   | wFormCount entries, stride 0x50
   | each entry's aFormPointer (Va)
   v
 vb::gui::GuiObjectInfo::read                     <- NEW (03-01)
   | lPropertiesLength (u32 at +0x59)
   | the tiling invariant this whole phase gates on
   v
 vb::controltree::walk                            <- NEW (03-04)
   | reads control blocks by their own Length field
   | reads scope-separator runs (0xFF + bytes, bounded scan)
   | asserts consumed bytes == lPropertiesLength, or refuses
   |   with the byte offset where the mismatch is found
   v
 vb::controltree::ControlNode
   { name, cType, array_index: Option<u16>, parent, children }
        |                                    |
        v                                    v
 vb::propstream::walk (needs an opcode table)   vb::ocx::read (cType 255)
   | vb::vbstr::VbStr for String properties          | class-name string, matched
   | position-block escape (-32768 => 16 bytes)       | against the external
   | Font block (§8.5.2)                              | component table (phase 2,
   | Picture / blob (=> vb::frx)                       | vb/project.rs)
   |                                                    v
   v                                              CLSID join + fixed OCX header
 vb::frx::extract_blob                            (_ExtentX, _ExtentY, _Version)
   | running cursor per form, +12 per blob         readable without a type library;
   | image signature sniff                          the rest of the blob is opaque
   v
 (blob bytes + synthesised .frx offset, held
  in memory this phase; Phase 4 writes the file)

 --- joined by control name, not by tree position ---

 OptionalObjectInfo.lpControls (phase 2 groundwork)
   v
 vb::controlinfo::ControlInfo::read                <- NEW (03-09)
   | lpGuid (Va) -> the control's CLSID
   | lpEventTable (Va) -> event handler stubs
   | lpszName (Va) -> the join key back to ControlNode
   v
 event ordinal -> event NAME needs an external table too (see above);
 without one, DeForm6 reports "event slot N, bound, name unavailable"

 --- the harness, built from none of the above ---

 tests/support/frm.rs                              <- NEW (03-03)
   independent .frm reader, parses Begin/End text,
   calls nothing in src/
        |
        v
 tests/differential.rs (extended, not replaced)
   compares recovered tree/types/names/properties
   against support::frm output, both directions
        |
        v
 tests/ratios.toml (extended: form ratio, control ratio added)
```

### Recommended Project Structure

```
crates/deform6/src/vb/
├── gui.rs               # NEW 03-01: GuiTable, GuiObjectInfo, the tiling check
├── opcodes.rs            # NEW 03-02: the table FORMAT and an optional loader;
│                          # ships with only the SVBD-facts safe subset built in
├── controltree.rs          # NEW 03-04: scope-byte walk, ControlNode, cType,
│                            # name, array Index (offset 0x05, closed below)
├── vbstr.rs                  # NEW 03-05: the encoding-validating string reader
├── propstream.rs               # NEW 03-06: typed payloads, position-block
│                                # escape, Font block, special opcodes
├── frx.rs                        # NEW 03-07: inline blob extraction, the
│                                  # running-cursor offset, image sniffing
├── ocx.rs                          # NEW 03-08: cType 255, class name, CLSID
│                                    # join, the fixed OCX header
├── controlinfo.rs                   # NEW 03-09: ControlInfo, event handler
│                                     # table, the control-to-event join
└── mod.rs                            # extended: Report gains forms, controls

crates/deform6/tests/
├── support/
│   ├── mod.rs           # extended: `pub mod frm;`
│   └── frm.rs             # NEW 03-03: independent .frm reader, and the
│                           # frmHMM.frx exclusion (see the dedicated section)
├── differential.rs          # extended: forms and controls, both directions
└── (existing files unchanged)

tests/ratios.toml            # extended: form ratio and control ratio columns
```

### Pattern 1: the control block's own `Length` field is the tiling gate's foundation

**What:** every control block, from the form itself down to the last leaf
control, carries a `u16 Length` at its own offset `0x00`. This value is what
lets a parser bound one block's bytes without first resolving the scope
grammar around it. The tiling check (ROADMAP.md success criterion 1: "the sum
of the control block `Length` fields tiles `GUIObjectInfo.lPropertiesLength`
exactly") is not a check added after the tree is built. It is the mechanism
that makes an unproven scope grammar safe to implement at all: a
mis-interpreted scope byte makes the walk consume the wrong number of total
bytes, and the tiling check catches that on the very same walk, by name and
offset, rather than emitting a tree nobody can trust.

**Evidence, measured this session.** `corpus/public-domain/LockWorkStation/
LockWorkStation.exe` has one form, no child controls. Its
`GUIObjectInfo.lPropertiesLength` is `0x4f` (79). The property stream at
`aFormPointer + 0x5D` (file offset `0x1219`) begins:

```
00001219: 4a 00 00 00 00 12 00 46 72 6d 4c 6f 63 6b 57 6f  J......FrmLockWo
00001229: 72 6b 53 74 61 74 69 6f 6e 00 0d 0a 01 19 01 00  rkStation.......
00001239: 42 00 22 00 23 ff ff ff ff 24 05 00 46 6f 72 6d  B.".#....$..Form
00001249: 31 00 2e 00 35 00 00 00 00 00 00 00 00 5a 00 00  1...5........Z..
00001259: 00 5a 00 00 00 44 00 46 02 ff 04 50 00 00 00 05  .Z...D.F...P....
```
`[VERIFIED: local, this session, file offset 0x1219 of
corpus/public-domain/LockWorkStation/LockWorkStation.exe]`

Reading the header: `Length = 0x004a` (74), `cId = 0x00`, name length `0x0012`
(18), name `"FrmLockWorkStation"` (18 characters, matches), `cType = 0x0d`
(13, Form, matches §8.4.1). The property loop runs to relative offset 71
(`Length - 2 - 1`), and the two bytes at relative offset 73-74 are `FF 04`,
which is the `vbFormEnd` scope terminator (§8.9, `0x04FF` = 1279). That
terminator sits **inside** this block's own `Length + 2` span (76 bytes:
relative offset 0 to 75), confirming `STRUCTURES.md`'s own two SVBD-derived
formulas ("the property loop runs while the cursor is below
`blockStart + Length - 2`" and "the next sibling starts at
`blockStart + Length + 2`") are both internally consistent for the
zero-children case `[DISPROVEN AS A GENERAL BOUND: see the correction
below]`. Beyond `Length + 2` (relative offset 76) sit three more
zero bytes, and only then does `lPropertiesLength` (79) end: `76 + 3 = 79`
exactly. `[VERIFIED: local, this session]` This 3-byte tail is unexplained;
it may be a fixed footer, or it may be specific to this sample. Plan 03-01
should check it against at least one more zero-children form before treating
it as general.

**Correction (plan 03-12, gap closure).** The "internally consistent"
finding above holds only for this one zero-children `LockWorkStation`
sample shown in the hex dump. It does not generalise to a control with
children, and the general property-loop bound is not `Length - 2`. The
shipped code, `crates/deform6/src/vb/propstream.rs`, computes the bound as
`block_end = u32::from(length).saturating_sub(1)`, that is, `Length - 1`,
matching the scope-separator start `blockStart + Length - 1` that plan
03-04 measured directly against `Grayscale.exe`'s real bytes (four
independent transitions, not the `Length - 2`/`Length + 2` prose above).
Plan 03-06 measured the property-loop bound the same way and kept the
shipped `Length - 1` bound; see `03-06-SUMMARY.md`'s `key-decisions` block
for the full account. A reader implementing a property loop uses the
shipped `Length - 1` bound, not the `Length - 2` formula the sentence
above states.

**A form with children breaks the naive hypothesis, and that is useful.**
`corpus/public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe` has one form
with three children. `lPropertiesLength = 183`. A flat walk that jumps by
`Length + 2` with no scope-byte interpretation at all reads the form's own
block correctly (`Length = 0x3e`, span 64 bytes) and then finds `Length = 0`
at the very next position, an implausible value. `[VERIFIED: local, this
session]` This falsifies, directly and immediately, any hope that Length
alone (without a real scope-byte walk) can traverse a form with children. It
confirms `STRUCTURES.md`'s own warning: **the scope separator bytes between
sibling blocks are not counted the same way for a block with children as they
appear to be for the outermost zero-children terminator case above.** A real
implementation must walk the scope-byte run between blocks, not skip past it
by arithmetic.

**When to use this pattern:** `vb/gui.rs` (03-01) implements the check as a
single assertion at the end of the whole-form walk: total bytes consumed
(sum of every block's `Length + 2`, plus every scope-separator run's byte
count) must equal `lPropertiesLength` exactly. `vb/controltree.rs` (03-04)
is what performs the walk this check verifies. A mismatch is a `Defect`
naming the file offset where the walk's own byte count first diverges from
what `lPropertiesLength` promised, not a silent partial tree.

### Pattern 2: bound the scope-byte scan (SAF-04, this phase's own version)

**What:** the scope-byte run (`0xFF`, then bytes until one is greater than 3
or equal to 0) is read from attacker-controlled bytes, exactly the shape of
input `AGENTS.md`'s "the file is hostile" rule exists for. A crafted file
that never emits a terminating byte (every scope byte is `01`, `02` or `03`
forever) must not be allowed to loop unbounded.

**When to use:** every scope-byte read in `vb/controltree.rs`.

**Example:**
```rust
// Source: this document's Pattern 1 and STRUCTURES.md §8.9. Illustrative
// only; the planner's own executor writes the real implementation against
// the project's Region/Off/Defect types.
const MAX_SCOPE_RUN: usize = 64; // generous; no real form nests this deep

fn read_scope_run(region: &Region, at: Off) -> Option<(ScopeRun, Off)> {
    if region.u8(at)? != 0xFF {
        return None; // not a separator here; caller decides what that means
    }
    let mut cursor = at.checked_add(1)?;
    let mut pops = 0u8;
    for _ in 0..MAX_SCOPE_RUN {
        let b = region.u8(cursor)?;
        cursor = cursor.checked_add(1)?;
        match b {
            0x01 => return Some((ScopeRun::OpenSibling, cursor)),
            0x02 | 0x03 => pops = pops.saturating_add(1),
            0x04 => return Some((ScopeRun::EndForm { pops }, cursor)),
            0x05 => return Some((ScopeRun::Menu { pops }, cursor)),
            _ => return Some((ScopeRun::Unrecognised { byte: b, pops }, cursor)),
        }
    }
    None // ran MAX_SCOPE_RUN bytes with no terminator; refuse, do not loop
}
```
The bound is a defensive limit, not a value read from the corpus; no real
form in this session's measurement needed more than one pop before its
terminator.

### Pattern 3: `VbStr` never lets a string read decide its own advance

**What:** `STRUCTURES.md` §9.3 gives the design already: `VbStr` takes an
explicit encoding, defaults to ASCII, validates that the cursor lands exactly
on the declared field end after the read, retries once as the other encoding
if it does not, and refuses if neither lands. **The cursor always advances by
the declared length, never by however far the string decoder happened to
read.** This is the one rule in this pattern that must never be relaxed:
STRUCTURES.md's own account of SVBD's heuristic shows exactly what goes wrong
when a string read is allowed to set its own pace.

**When to use:** every `String`-typed property in `vb/propstream.rs`, and the
control name and class name reads in `vb/controltree.rs` and `vb/ocx.rs`
(those are already-fixed ASCII per §9.1, so `VbStr` is not needed there, but
the same "advance by the declared length, never by what the decode found"
discipline still applies).

**Example:**
```rust
// Source: STRUCTURES.md §9.3's recommendation, given a concrete shape.
// Illustrative; matches the project's existing Region/Defect conventions.
enum StrEncoding { Ascii, Utf16 }

fn read_vb_str(region: &Region, at: Off, encoding: StrEncoding) -> Result<(String, Off), Defect> {
    let len = region.u16_le(at).ok_or_else(|| /* Defect: field end unreadable */)?;
    let declared_end = at.checked_add(2)
        .and_then(|o| o.checked_add(u32::from(len)))
        .and_then(|o| o.checked_add(1)) // the trailing NUL
        .ok_or_else(|| /* Defect: OffsetOverflow */)?;
    let text_start = at.checked_add(2).ok_or_else(|| /* Defect */)?;
    let decoded = match encoding {
        StrEncoding::Ascii => decode_ascii(region, text_start, len),
        StrEncoding::Utf16 => decode_utf16(region, text_start, len),
    };
    // The cursor ALWAYS advances to declared_end, whether or not the
    // decode below produced a plausible string. That is what makes a
    // wrong encoding a caption defect, not a whole-form derailment.
    match decoded {
        Some(s) if cursor_lands_correctly(region, text_start, &s, declared_end) => {
            Ok((s, declared_end))
        }
        _ => {
            // Retry once as the other encoding, same landing check.
            // If that also fails, refuse this property; the cursor
            // still advances to declared_end so the rest of the form
            // is not lost to one bad property.
            Err(/* Defect: unrecoverable string property, byte offset at */)
        }
    }
}
```

### Pattern 4: honest gaps use the same words FRM-04 already established

**What:** phase 2 already built the vocabulary for "the file holds this and
the tool cannot read it" versus "this never survives compilation." Phase 3's
opcode-table gap and its OCX-property gap are the same kind of fact and
should use the same reporting shape, not two different phrasings that make a
reader guess whether they mean the same thing.

**When to use:** any property phase 3 cannot decode: `vb/propstream.rs` for
an unnamed opcode, `vb/ocx.rs` for an OCX property blob, `vb/controlinfo.rs`
for an event slot with no name table loaded.

**Example, matching the CLI's existing style (`crates/deform6-cli/src/
main.rs`'s `Declarations` markers, phase 2):**
```
Property opcode 31 at offset 0x1a04: value not decoded, no opcode table
  loaded for control type CommandButton. Run with --opcode-table to
  supply one.
```

### Anti-Patterns to Avoid

- **Treating the scope-byte grammar as solved because it compiles and runs on
  the corpus without crashing.** A heuristic that never crashes on 44 files
  can still build a mis-nested tree quietly. The tiling check is what turns
  silence into a caught defect; skipping it because "the corpus passes" is
  exactly the failure mode `STRUCTURES.md` names gap 14 to prevent.
- **Guessing a property's type from its name.** `Enabled` is not always
  `Boolean` at a fixed opcode across every control type; the opcode space is
  per control type by the roadmap's own stated risk, and this session's
  corpus counts confirm real variety (position opcode alone is 4, 5, 2 or 53
  depending on control type). A property with no table entry is undecoded,
  never inferred from its name.
- **Committing anything derived from `VB6.OLB`, directly or indirectly.**
  This includes a table one control type at a time, a table gated behind a
  feature flag, or a table hidden inside a test fixture. `AGENTS.md`'s rule
  makes no exception for partial or test-only derived fixtures.
- **Silently widening `cType` or `fObjectType` beyond what a real file
  proves**, the same discipline phase 2's `classify.rs` already established.
  `STRUCTURES.md` §8.4.1 lists several unassigned `cType` values (12, 14, 15,
  21, 25 through 36, 39); an unknown `cType` is reported raw, never guessed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| Opcode-to-property mapping | A committed table dumped from `VB6.OLB` | An external, user-supplied table plus the small safe-provenance subset | `AGENTS.md`'s redistribution rule; see the dedicated section above |
| String decoding in the property stream | A single fixed-encoding reader, or SVBD's own unsound length-then-retry heuristic copied verbatim | `VbStr`, with the declared-length-always-wins discipline | STRUCTURES.md §9.3 shows SVBD's own heuristic is provably wrong by its own byte accounting; copying it copies the bug |
| Scope-byte tree walk validation | Trusting the walk because it produced a plausible-looking tree | The `lPropertiesLength` tiling assertion, every time | A plausible tree and a correct tree are not the same claim, and only the byte count proves the second |
| GUID formatting | A crate | Sixteen bytes rendered as fixed hex groups | The output shape never varies; a crate buys nothing a five-line function does not already give |

**Key insight:** every "don't hand-roll" item in this phase is really the
same lesson twice: do not let a plausible-looking result stand in for a
byte-accounted one. The scope grammar and the string encoding are both places
where "it looks right" is exactly the failure mode `STRUCTURES.md`'s own
research already caught SVBD making.

## Common Pitfalls

### Pitfall 1: assuming the `Length` field means the same thing at every nesting depth

**What goes wrong:** this session's own two measurements (Pattern 1 above)
show the zero-children terminator case and the with-children case do not
obviously reconcile under one simple formula. A parser that hard-codes the
zero-children reconciliation ("the terminator sits inside `Length + 2`") as a
universal rule will misparse the very next form that has children, silently.

**Why it happens:** the easiest test case to reach for first (a form with no
controls) is also the least representative one, because it has no sibling or
child scope-byte transitions to get wrong.

**How to avoid:** validate the tiling check against forms with children
before trusting it against forms without. `SK-Gradient-Sample__VB6` (3
children, one level of nesting) and `Grayscale-effect` (nested `Frame`
containers with `OptionButton` control arrays inside them) are both in the
corpus and both are more representative than the zero-children case.

**Warning signs:** a tiling check that passes on every zero-children form and
fails, or worse silently under-consumes, on every form with a `Frame` or
other container control.

### Pitfall 2: confusing the control array Index byte with `cId`

**What goes wrong:** `STRUCTURES.md`'s own array-header table places `cId`
at offset `0x05` in the array layout. This session's measurement (see the
dedicated closure section below) shows the byte that actually varies with
`Index` sits at that same offset, `0x05`, and the byte `STRUCTURES.md` calls
"array flag" at `0x03`-`0x04` does not vary with `Index` at all; it is
constant per array group. Reading `STRUCTURES.md`'s table literally, without
this session's correction, assigns the wrong field to the wrong name.

**Why it happens:** `STRUCTURES.md` §8.4 itself flags this exact byte range
as gap 11, unresolved, so the published table was already a best guess by its
own account.

**How to avoid:** use this session's corpus-proven offset (`0x05`, a single
byte, values 0 to 24 observed) rather than `STRUCTURES.md`'s published table
for this one field, and update `STRUCTURES.md` itself as part of plan 03-04's
own work, the same way plan 01-05 updated it for gap 1.

**Warning signs:** a control array whose recovered `Index = N` values do not
match the array's declaration order in a hand-inspected `.frm`.

### Pitfall 3: treating "the corpus has zero third party OCX controls" as true

**What goes wrong:** `GAPS.md`, the phase 2 gap audit, states the corpus has
zero third party OCX control instances in any `.frm`, based on a grep that
used `corpus/**/*.frm` without `shopt -s globstar`, which in a plain `bash`
invocation expands `**` the same as a single `*` and therefore misses any
file more than one directory level deep. Every `MSWinsockLib.Winsock`
instance in this corpus sits inside a nested project directory
(`SK-Winsock-Sample__VB6/frmMain.frm`, `SK-TFTP-Sample__VB6/Server/
frmMain.frm`, `SK-TFTP-Sample__VB6/Client/frmMain.frm`) and every one of
those was silently skipped by the earlier audit's glob.

**Why it happens:** `**` is a `bash` `globstar` feature, off by default, and
a script that assumes it is on fails quietly rather than with an error;
`corpus/**/*.frm` under a non-globstar shell simply matches fewer files, with
no warning.

**How to avoid:** use `find corpus -iname "*.frm" -print0 | xargs -0 ...` (or
an explicitly `shopt -s globstar`-enabled invocation), which this session
used to re-count every corpus figure quoted here. Any future audit of this
corpus should do the same, and should re-verify a "zero" finding with a
second method before reporting it, per `AGENTS.md`'s "give the number you can
prove" rule.

**Warning signs:** a "zero" or "none" finding about the corpus that a
different traversal method has not cross-checked.

## Code Examples

### The GUI table entry, following the phase 1/2 window-before-fields pattern

```rust
// Source: STRUCTURES.md §8.1, offsets [VERIFIED: local] this session
// against corpus/public-domain/LockWorkStation/LockWorkStation.exe and
// corpus/public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe.
// Illustrative shape; matches vb/project.rs's existing subregion discipline.
struct GuiTableEntry {
    a_form_pointer: Va, // +0x48
}

fn read_gui_table_entry(region: &Region, at: Off) -> Option<GuiTableEntry> {
    let entry = region.subregion(at, 0x50)?;
    let l_struct_size = entry.u32_le(Off::new(0x00))?;
    if l_struct_size != 0x50 {
        return None; // the one cheap validation gate STRUCTURES.md names
    }
    Some(GuiTableEntry {
        a_form_pointer: entry.va_le(Off::new(0x48))?,
    })
}
```

### The `GUIObjectInfo` unaligned read, and the tiling length

```rust
// Source: STRUCTURES.md §8.2. GUIObjectInfo is NOT 4-byte aligned
// internally (a single byte at +0x04 throws every later field onto an
// odd offset); Region's field-at-a-time readers make this safe by
// construction, since none of them assume alignment.
struct GuiObjectInfo {
    l_properties_length: u32, // +0x59
}

fn read_gui_object_info(region: &Region, at: Off) -> Option<GuiObjectInfo> {
    // Size is 0x5D (93) plus the property stream that follows it; take
    // only the fixed header here, and hand the caller the stream's own
    // start offset separately.
    let header = region.subregion(at, 0x5D)?;
    Some(GuiObjectInfo {
        l_properties_length: header.u32_le(Off::new(0x59))?,
    })
}
```

### The control array Index, the corpus-closed offset

```rust
// Source: this session's measurement, closing STRUCTURES.md gap 11.
// [VERIFIED: local] against 30 real array elements: optChannel (Index
// 2, 1, 0) and optDecompose (Index 1, 0) in
// corpus/vb6-code/Grayscale-effect/Grayscale.exe, and TxtF (Index 0
// through 24, all 25 elements) in
// corpus/vb6-code/Custom-image-filters/CustomFilters.exe. Every one of
// the 30 matches its .frm source's declared Index value exactly.
//
// Raw evidence, TxtF, three consecutive elements (Index 0, 1, 2):
//   ...80 02 00 00 04 00 54 78 74 46...   (Index=0: byte at +0x05 is 00)
//   ...80 02 01 00 04 00 54 78 74 46...   (Index=1: byte at +0x05 is 01)
//   ...80 02 02 00 04 00 54 78 74 46...   (Index=2: byte at +0x05 is 02)
// where 0x80 is the array-selector flags byte at +0x03, the constant
// 0x02 at +0x04 is unexplained (it differs by array GROUP: 0x02 for
// TxtF, 0x05 for optDecompose, 0x07 for optChannel, but is the same
// for every element within one group), and "54 78 74 46" is "TxtF".
//
// Whether the field is a single byte or the low byte of a u16 spanning
// +0x05..+0x06 cannot be told apart from this corpus: no array index
// in any corpus program exceeds 24, so the high byte is always 0
// either way. Treat it as a u16 defensively (read both bytes, refuse
// if a future sample needs a value the low byte alone cannot hold)
// rather than hard truncating to u8.
fn read_array_index(region: &Region, block_at: Off) -> Option<u16> {
    let flags = region.u8(block_at.checked_add(0x03)?)?;
    if flags != 0x80 {
        return None; // not an array control; caller reads cId at 0x04 instead
    }
    region.u16_le(block_at.checked_add(0x05)?)
}
```

### The position block escape

```rust
// Source: STRUCTURES.md §8.5.1. A real escape hatch, not a theoretical
// one: a form with any coordinate outside i16 range needs the 16-byte
// form. No corpus file in this session's measurement exercises the
// escape (every corpus coordinate fits in i16), so this remains
// [L]/untested-by-construction until a sample is found; implement it
// anyway, because SAF-01 forbids assuming an untested branch is dead.
//
// Correction (plan 03-12, gap closure): this sample used to start the
// four i32 reads two bytes past `at` (18 bytes read from `at`: 2 for the
// peek plus 16 for the four i32 values) while still returning 16 as the
// consumed count, an internal contradiction. Plan 03-06 settled which
// reading the shipped code uses; see 03-06-SUMMARY.md's key-decisions
// block. The corrected sample below re-reads the same 16-byte span the
// short form's own four i16 values would have occupied, starting at
// `at`, matching crates/deform6/src/vb/propstream.rs's shipped
// read_position_block.
enum PositionBlock {
    Short { left: i16, top: i16, width: i16, height: i16 },
    Long { left: i32, top: i32, width: i32, height: i32 },
}

fn read_position_block(region: &Region, at: Off) -> Option<(PositionBlock, u32)> {
    let first = region.i16_le(at)?;
    if first == -32768 {
        Some((
            PositionBlock::Long {
                left: region.i32_le(at)?,
                top: region.i32_le(at.checked_add(4)?)?,
                width: region.i32_le(at.checked_add(8)?)?,
                height: region.i32_le(at.checked_add(12)?)?,
            },
            16,
        ))
    } else {
        Some((
            PositionBlock::Short {
                left: first,
                top: region.i16_le(at.checked_add(2)?)?,
                width: region.i16_le(at.checked_add(4)?)?,
                height: region.i16_le(at.checked_add(6)?)?,
            },
            8,
        ))
    }
}
```

## The `frmHMM.frx` exclusion, by name, with the reason recorded

`corpus/vb6-code/Hidden-Markov-model/frmHMM.frx` is 56 bytes:

```
00000000: 3820 596f 7572 2048 4d4d 2061 6e61 6c79  8 Your HMM analy
00000010: 7365 7320 6f75 7470 7574 2077 696c 6c20  ses output will
00000020: 6170 7065 6172 2069 6e20 7468 6973 2074  appear in this t
00000030: 6578 7420 626f 780a                      ext box.
```
`[VERIFIED: local, this session, xxd of the file as committed]`

This is not a valid `.frx` record. `FILE-FORMATS.md` §4.5 already reconstructs
why, with the compiled EXE as the second, independent source of truth:
`corpus/vb6-code/Hidden-Markov-model/HMM.exe` holds the same string as a
proper record, `0B 38 00 20 "Your HMM analyses output will appear in this
text box" 0D 0A 00`, a one-byte record-length header (`0x38` = 56) followed
by 56 bytes of Windows-1252 text with a `CRLF` inside it. The upstream
repository's `.gitattributes` sets `* text=auto`. Git therefore classified
this `.frx` as text, because it holds no NUL byte, and normalised its line
endings on commit or checkout: one `CR` was silently removed, and `57`
becomes `56`. `FILE-FORMATS.md` §6.2 already names this exact file as the
worked example of the general warning: "the corpus `.gitattributes` applies
`* text=auto` and `*.frx -diff`. The `-diff` setting stops a diff. It does
not stop line ending normalisation."

**This is upstream corruption, proven byte-for-byte, not a DeForm6 parsing
question.** The exclusion this phase's success criterion 5 asks for is
therefore a *known, named, upstream defect*, structurally different from the
generic `support/rules.rs` exclusion pattern phase 2 built for VER-04: those
five rules are deliberately generic, predicate-shaped, and forbidden from
naming a program, precisely so a rule cannot become a silent per-program
exception. `VER-06`'s exclusion is the opposite shape on purpose: the roadmap
explicitly asks for a *named* exclusion, with the reason recorded next to it,
and a test that fails loudly if the exclusion is ever removed. Plan 03-03
should build this as its own small, separate mechanism (a short, explicit
list of `(file path, reason)` pairs, distinct from `RULES`), not force it
into the generic predicate shape phase 2 built for a different purpose.

**What DeForm6 itself must not repeat.** `FILE-FORMATS.md` §6.2 already
states the fix: DeForm6's own `.gitattributes` must set `-text` or `binary`
on `*.frx` and `*.ctx` before any fixture that includes one is committed,
including any synthetic OCX fixture plan 03-08 needs (see below). This is a
one-line repository configuration change, and it belongs in plan 03-03 or
wherever the phase first commits a `.frx`-bearing fixture, whichever comes
first.

## The third party OCX corpus material, corrected

`GAPS.md` reports the corpus holds zero third party OCX control instances,
based on a `grep` over `corpus/**/*.frm` that silently missed every nested
project directory (Pitfall 3, above). The real count, re-measured this
session with `find ... -print0 | xargs -0`:

```
corpus/public-domain/SK-Winsock-Sample__VB6/frmMain.frm:89:
   Begin MSWinsockLib.Winsock wsPop
corpus/public-domain/SK-TFTP-Sample__VB6/Server/frmMain.frm:13:
   Begin MSWinsockLib.Winsock WskServer  (Index = 0, a control array member)
corpus/public-domain/SK-TFTP-Sample__VB6/Client/frmMain.frm:67:
   Begin MSWinsockLib.Winsock WskClient
```
`[VERIFIED: local, this session]`

Three real instances, across three programs, one of them a control array
element (`WskServer`, `Index = 0`), matching `CORPUS.md`'s own already-correct
"48 controls carry an `Index` property" figure and its "3 third party control
instances" figure, both written after `GAPS.md` and both accurate.
`COMDLG32.OCX` is referenced at the project level in one `.vbp`
(`Transparency-2D/Transparency.vbp`) but is never instantiated as a `Begin`
block in that project's `.frm`; the common dialog control is invoked from
code at run time in that program, not placed on a form at design time, so it
gives FRM-04 no control-block material, only a project-level `Object=`
reference to exercise the external component table join.

**What this means for plan 03-08.** FRM-04 has real, if thin, corpus
material: three instances, one control type, one array member. This is
enough to prove the CLSID join (`MSWinsockLib.Winsock`'s `Object=` line gives
`{248DD890-BB45-11CF-9ABC-0080C7E7B78D}` and `MSWINSCK.OCX`, matched against
the class name string SVBD documents appearing in the control block itself)
against real bytes, and enough to prove the fixed `_ExtentX`/`_ExtentY`/
`_Version` header (§8.7's `0x12344321` signature) against a real OCX blob.
It is not enough to prove the general `cType == 255` control-block header
shape against more than one control type. `GAPS.md`'s original recommendation
to build a synthetic fixture for broader OCX coverage still stands; this
correction narrows what "coverage" already exists, it does not remove the
gap.

## Phase boundary note: FRM-05 and the `.frx` file itself

`REQUIREMENTS.md`'s FRM-05 text says "recovers the resource blobs and writes
an `.frx` whose offsets the generated `.frm` agrees with." `ROADMAP.md`'s
Phase 3 success criteria, by contrast, describe only `inspect` reporting the
control tree, properties, CLSIDs, resource blobs, and event names; nothing in
Phase 3's own success criteria asks for a written `.frx` file on disk, and
`inspect` (unlike `extract`) writes nothing to disk by design, the same
invariant DET-06 established in Phase 1 and Phase 2 both keep. Plan 03-07 is
titled "`.frx` blob extraction from the inline property stream," which
matches the ROADMAP success criteria (recover the blob bytes into memory,
synthesise the offset a `.frm` writer would need) rather than the literal
`WRT`-shaped "writes an `.frx`" half of FRM-05's own wording. `WRT-01`
through `WRT-07` (Phase 4) own the actual file-writing work, and `.frx`
writing specifically is called out as inseparable from `.frm` writing in
Phase 4's own named risk ("The `.frx` offset is not stored in the
executable... The `.frm` writer and the `.frx` writer are one component...
Plan 04-04 owns both").

This document does not resolve the wording mismatch; it flags it, because a
planner reading FRM-05's literal text could reasonably plan a Phase 3 task to
write a `.frx` file, which would duplicate Phase 4's plan 04-04 and violate
the "`inspect` writes nothing to disk" invariant SC1 restates for this phase.
The recommendation is: Phase 3 recovers and holds the blob bytes and computes
the offset cursor in memory (matching plan 03-07's own title and ROADMAP's
success criteria), and Phase 4 is where a `.frx` file is actually written.

## State of the Art

Not applicable in the framework-version sense; this is a fixed, decades-old
binary format. The relevant "old versus current" facts are internal
corrections this session made to this project's own prior research:

| Old Claim | Corrected Claim | Source | Impact |
|---|---|---|---|
| Control array index location unresolved (`STRUCTURES.md` gap 11) | Index is the byte at control-block offset `0x05` in the array header layout | This session, 30 array elements across 2 files | Plan 03-04 can implement `Index = N` directly rather than treating it as open |
| Zero third party OCX control instances in the corpus (`GAPS.md`) | Three `MSWinsockLib.Winsock` instances, across 3 files, one a control array member | This session, corrected glob | Plan 03-08 has real (if thin) corpus material, not zero |
| Control arrays present in 35 corpus files (`GAPS.md`) | Control arrays present in 6 files, 48 elements total | This session, exact-property-name regex | `CORPUS.md`'s "48 controls" figure was already right; only the file count in `GAPS.md` was wrong |
| Menus in 17 files (`ROADMAP.md`'s own research trail as read by an earlier grep) | 22 files, 76 menu entries | This session, `find -print0 \| xargs -0`, matches `CORPUS.md` exactly | Confirms `CORPUS.md`'s number is the one to trust; any figure derived from a bare `**` glob in this repository's research history should be re-checked the same way |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | The AGENTS.md "no derived fixture from a third party system the author does not own" rule bars committing an opcode table calculated from `VB6.OLB`, even when the person running the derivation tool personally owns a lawful VB6 install | Derived Data and the AGENTS.md Redistribution Rule | If the human maintainer reads the rule more permissively, plan 03-02 could commit a full table instead of building the external-table mechanism this document recommends, which is a materially smaller amount of implementation work. This should be confirmed with the human before 03-02 is planned in detail. |
| A2 | The event-name table sits on "materially firmer" legal ground than the property-opcode table, because event names and their fixed order are independently, widely published outside Microsoft's own type library | Derived Data and the AGENTS.md Redistribution Rule | If this is wrong, plan 03-09's event-name table needs the exact same external-table treatment as 03-02's property table, which changes FRM-06's achievable scope further |
| A3 | The 3-byte tail after the zero-children `LockWorkStation` form's control block span (`Length + 2` = 76, `lPropertiesLength` = 79) is a general fixed footer, not specific to this one sample | Pattern 1 | If it varies by form, the tiling check's exact accounting needs a term this document has not identified; the check's overall shape (assert total consumed equals `lPropertiesLength`, refuse on mismatch) still holds regardless |
| A4 | The array-index field (control-block offset `0x05`) is a full byte, sufficient for every real control array; whether it is actually the low byte of a `u16` spanning `0x05`-`0x06` cannot be told apart from this corpus, since no array index observed exceeds 24 | Code Examples, "The control array Index" | A VB6 control array index can, in principle, be much larger than 24; if the true field is `u8` and DeForm6 reads it as `u16`, a high byte that happens to be non-zero in some future file (if `0x06` is actually `cId` and not part of the index) would silently corrupt the recovered index. The `u16` reading matches the byte evidence with the corpus available today; a synthetic fixture with `Index > 255` would close this fully. |
| A5 | `FRM-05`'s literal "writes an `.frx`" wording is a wording carryover from `REQUIREMENTS.md` and Phase 3's actual scope, per `ROADMAP.md`'s own success criteria and plan 03-07's title, is blob recovery only, with file-writing deferred to Phase 4 | Phase boundary note | If the human intends FRM-05 literally, Phase 3 needs a plan to write a `.frx` file, which conflicts with the `inspect`-writes-nothing invariant unless a new subcommand or flag is introduced this document has not scoped |

## Open Questions

1. **Does the human confirm the external-table resolution to the AGENTS.md
   conflict, or does the project want a different path?**
   - What we know: `AGENTS.md`'s rule is unambiguous about not committing a
     derived fixture from an unowned third party system. This document's
     proposed resolution (commit the tool, never the table; ship with no
     table by default; accept a user-supplied one at run time) stays inside
     that rule.
   - What's unclear: whether the human maintainer wants FRM-03's scope
     reduced this much, or would rather see the phase descoped differently
     (for example, shipping only the tree/type/name recovery this phase can
     do fully, and deferring all property-value decoding past the small safe
     subset to a later milestone).
   - Recommendation: raise this explicitly at discuss-phase or plan-review,
     before 03-02 is planned in detail, because it changes what "done" means
     for FRM-03 and the differential gate's control-property comparison.

2. **Should `STRUCTURES.md` itself be updated with this session's two
   closures (gap 11, and the corrected corpus counts), and by which plan?**
   - What we know: `STRUCTURES.md` §13 shows the established pattern (a
     dated, in-place update with the method and the worked example, keeping
     the old dispute recorded for history) for exactly this kind of
     closure.
   - What's unclear: which plan owns the update. Plan 03-04 is the natural
     owner for gap 11 (it consumes the finding directly); no plan currently
     owns a `GAPS.md`/`CORPUS.md` correction pass.
   - Recommendation: fold the `STRUCTURES.md` gap 11 update into plan 03-04's
     own scope, and note the `GAPS.md` corpus-count corrections in this
     phase's own SUMMARY rather than editing `GAPS.md` retroactively, since
     `GAPS.md` is itself a dated audit artifact, not a living document.

3. **What does `frmHMM.frm` (not `.frx`) contribute to the differential
   gate, once its `.frx` is excluded?**
   - What we know: only the `.frx` file is corrupted; the `.frm` text is
     intact and has 14 `Index =` lines, more control-array material than
     any other single corpus file.
   - What's unclear: whether `support/frm.rs`'s exclusion should drop the
     whole `frmHMM.frm`/`.exe` pair from the differential gate, or only
     the one property (`Text = "frmHMM.frx":0000`) that resolves into the
     damaged file.
   - Recommendation: exclude only the one property that resolves into the
     damaged `.frx`, and keep the rest of `frmHMM.frm`'s real control-array
     material in the gate, since VER-06's own wording ("excluded by name")
     does not specify a whole-file exclusion and the roadmap's own success
     criterion 5 only requires the `.frx` file itself to be named and
     excluded.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| `rustc` / `cargo` | The whole workspace | Yes | pinned by `rust-toolchain.toml`, unchanged from phase 1/2 | — |
| A lawful, locally-owned VB6 install (Windows) | The full opcode-to-property table and the full event-name table, if the project pursues them beyond the safe subset | **No**, not in this research session's environment | — | Ships as an external, optional, user-supplied artifact; DeForm6 itself needs none to build, test, or run its own gate |
| Python 3 (research only, not shipped) | This session's byte-level measurement scripts | Yes | 3.13 (this session's runtime) | Not applicable, throwaway, not part of the build |

**Missing dependencies with no fallback:** none. The one dependency this
phase cannot satisfy in this environment (a lawful VB6 install) has a
fallback by design: the external-table mechanism this document recommends.

**Missing dependencies with fallback:** the VB6 install, as above.

## Validation Architecture

### Test Framework

| Property | Value |
|---|---|
| Framework | `cargo test`, unchanged from phase 1 and phase 2 |
| Config file | none, `cargo test --workspace` is the whole invocation |
| Quick run command | `cargo test -p deform6 --lib` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| FRM-01 | GUI table walk, control tree, the tiling gate | unit + integration | `cargo test -p deform6 --lib vb::gui:: vb::controltree::` | No, Wave 0 (`vb/gui.rs`, `vb/controltree.rs` new) |
| FRM-02 | Control type, name, array index | unit | `cargo test -p deform6 --lib vb::controltree::` | No, Wave 0 |
| FRM-03 | Property values, `VbStr`, position block, Font block | unit | `cargo test -p deform6 --lib vb::vbstr:: vb::propstream::` | No, Wave 0 |
| FRM-04 | OCX CLSID join, fixed OCX header | unit + integration | `cargo test -p deform6 --lib vb::ocx::` | No, Wave 0 |
| FRM-05 | `.frx` blob extraction, offset cursor, image sniff | unit | `cargo test -p deform6 --lib vb::frx::` | No, Wave 0 |
| FRM-06 | `ControlInfo`, event handler table, control-to-event join | unit | `cargo test -p deform6 --lib vb::controlinfo::` | No, Wave 0 |
| VER-06 | `frmHMM.frx` excluded by name, reason recorded, removing it fails loudly | integration | `cargo test -p deform6 --test differential -- frmhmm` (or a dedicated name) | No, Wave 0 (`tests/support/frm.rs` new) |
| (all of the above) | Recovered tree/types/names/properties compared against `support/frm.rs`, both directions | integration | `cargo test -p deform6 --test differential` | No, Wave 0, `differential.rs` extended |

### Sampling Rate

- **Per task commit:** `cargo test -p deform6 --lib` (the module under active
  work, fast)
- **Per wave merge:** `cargo test --workspace` (full suite, including the
  extended differential gate)
- **Phase gate:** `cargo fmt --all --check && cargo clippy --all-targets --
  -D warnings && cargo test --workspace`, unchanged from phase 1 and phase 2

### Wave 0 Gaps

- [ ] `crates/deform6/src/vb/gui.rs` - `GuiTable`, `GuiObjectInfo`, the
      tiling invariant (FRM-01)
- [ ] `crates/deform6/src/vb/controltree.rs` - scope-byte walk, `cType`,
      name, array `Index` (FRM-01, FRM-02)
- [ ] `crates/deform6/src/vb/vbstr.rs` - the encoding-validating string
      reader (FRM-03)
- [ ] `crates/deform6/src/vb/propstream.rs` - typed payloads, position block
      escape, `Font` block (FRM-03)
- [ ] `crates/deform6/src/vb/opcodes.rs` - table format, optional loader, the
      SVBD-facts safe subset (FRM-03)
- [ ] `crates/deform6/src/vb/frx.rs` - blob extraction, offset cursor, image
      sniffing (FRM-05)
- [ ] `crates/deform6/src/vb/ocx.rs` - `cType 255`, CLSID join, fixed OCX
      header (FRM-04)
- [ ] `crates/deform6/src/vb/controlinfo.rs` - `ControlInfo`, event handler
      table (FRM-06)
- [ ] `crates/deform6/tests/support/frm.rs` - the independent `.frm` reader,
      and the named `frmHMM.frx` exclusion (VER-06)
- [ ] `.gitattributes` entries for `*.frx` and `*.ctx`, `-text` or `binary`,
      before any `.frx`-bearing fixture is committed

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V2 Authentication | No | Local CLI, no auth surface |
| V3 Session Management | No | Same |
| V4 Access Control | No | Same |
| V5 Input Validation | **Yes** | Every count and length this phase reads (`wFormCount`, `lPropertiesLength`, every control block `Length`, `wEventCount`, `dwControlCount`, every scope-byte run, every property payload width) is attacker-controlled and goes through `Region`/`checked_add`, exactly as phases 1 and 2 already establish. The scope-byte run additionally needs an explicit bound (Pattern 2 above), which phases 1 and 2 did not need in this exact shape, because nothing before this phase read a variable-length run with no declared length field. |
| V6 Cryptography | No | Not applicable |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| A crafted scope-byte run with no terminating byte | Denial of Service | Bounded scan (Pattern 2), refuse rather than loop; the bound is a defensive constant, not a value read from the file |
| A crafted `Length` field that, summed with a base offset, overflows a `u32` and wraps to a small in-bounds value | Tampering | `checked_add` on every offset arithmetic step, per `AGENTS.md`'s explicit rule; this is the identical class of bug `read/region.rs`'s own doc comment already names ("A survey read the string `MZ` out of the DOS stub that way") |
| A `VbStr` read that is allowed to determine its own advance from a mis-decode, corrupting every property that follows it in the same block | Tampering (silent corruption of unrelated data) | The declared-length-always-wins discipline in Pattern 3; this is the single most important rule in the whole phase, because unlike most `Defect` cases, a wrong `VbStr` advance does not fail loudly, it cascades |
| A property opcode this phase has no table entry for, decoded anyway by guessing a payload width from the opcode number's proximity to a known one | Tampering / Information Disclosure | Never decode an unnamed opcode; report it undecoded, at its byte offset, and stop advancing past it only via the block's own `Length` field, never by a guessed payload width |
| A `dwControlCount` or `wEventCount` used to size a `Vec::with_capacity` before being checked against the real file size | Denial of Service | `SAF-04`, unchanged discipline from phases 1 and 2: bound every count against the real file length before any allocation is sized from it |

## Sources

### Primary (HIGH confidence, measured this session)

- `/tmp/frm_probe.py`, this session's throwaway PE/VB header/GUI table/
  property-stream reader, independent of `deform6`'s own code, run against
  `corpus/public-domain/LockWorkStation/LockWorkStation.exe`,
  `corpus/public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe`,
  `corpus/vb6-code/Grayscale-effect/Grayscale.exe`, and
  `corpus/vb6-code/Custom-image-filters/CustomFilters.exe`
- `corpus/**/*.frm`, `corpus/**/*.vbp`, `corpus/vb6-code/Hidden-Markov-model/
  frmHMM.frx` and its compiled `HMM.exe` - read directly for the property
  name census, the control array `Index` closure, the third party OCX
  correction, and the `frmHMM.frx` corruption evidence
- `github.com/pmachapman/semi-vb-decompiler` - checked for the presence of a
  committed `VB6.OLB` file, to confirm what SVBD's own repository does and
  does not do

### Secondary (MEDIUM confidence, cited from this project's own prior research)

- `.planning/research/STRUCTURES.md` §8, §9, §11 - the field layouts this
  session's script encodes and validates; every offset used was taken from
  this document first and then checked against real bytes
- `.planning/research/FILE-FORMATS.md` §4.5, §6.2 - the `frmHMM.frx`
  corruption story and the `.gitattributes` fix it names
- `.planning/research/CORPUS.md` "What the corpus holds for phase 3" -
  cross-checked against this session's own independent re-count and found
  accurate
- `.planning/phases/02-the-object-graph/GAPS.md` - the source of the two
  corrected corpus counts; cited as the thing being corrected, not as a
  reliable count for those two figures
- Sega Enterprises Ltd. v. Accolade, Inc., 977 F.2d 1510 (9th Cir. 1992);
  EU Software Directive 2009/24/EC, Article 6 - the reverse-engineering-for-
  interoperability doctrine this document's licence reasoning rests on. This
  is not legal advice and the document says so.

### Tertiary (LOW confidence, marked as such where the corpus stays silent)

- `STRUCTURES.md` §8.4's published array-header table for offsets `0x03`
  through `0x06` - superseded for the `Index` field specifically by this
  session's measurement; `STRUCTURES.md` itself should be updated, not
  silently distrusted
- `STRUCTURES.md` §8.5.1's position-block-escape case (`-32768` => 16 bytes)
  - no corpus sample exercises it; implemented per the format's own stated
    rule, untested by construction, same treatment phase 1 gave the P-code
    branch
- The 3-byte tail after `LockWorkStation`'s single control block (A3 in the
  Assumptions Log) - one sample, not generalised

## Metadata

**Confidence breakdown:**
- The AGENTS.md conflict and its resolution: **HIGH** that the conflict
  exists and that `AGENTS.md` is binding; **MEDIUM** on the specific legal
  reasoning, which is not legal advice and should not be presented as
  settled to an end user of DeForm6
- The tiling-gate mechanism (Pattern 1): **HIGH** on the two measured
  examples; **LOW**, and marked so, on whether the zero-children reconciliation
  generalises to every zero-children form
- The control array Index closure: **HIGH** - 30 real elements, 2 files, 2
  control types, array sizes 2 through 25, zero disagreements
- The corrected corpus counts (OCX instances, control array file count,
  menu count): **HIGH** - each re-measured this session with a traversal
  method that does not depend on shell `globstar`
- `frmHMM.frx` exclusion mechanism: **HIGH** - the corruption is proven
  byte-for-byte against the compiled EXE as a second source
- Property stream decoding beyond the safe subset (FRM-03's full scope):
  **explicitly LOW/blocked**, pending the human confirmation this document's
  Open Question 1 asks for

**Research date:** 2026-09-09
**Valid until:** indefinite for the corpus-measured facts, the corpus is
static and vendored; indefinite for the `AGENTS.md` conflict finding, until
the human resolves Open Question 1; 30 days would apply to any dependency
version pin, but this phase adds none.
