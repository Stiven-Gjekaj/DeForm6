# Roadmap

## The finished product

A person has a compiled program and no source. They run DeForm6. They get a
Visual Basic project back. They open that project in a compiler they already
own, they edit it, and they build it.

DeForm6 is the decompiler and the proof that its output builds. It is not the
compiler and it is not the editor.

Two compilers exist and both accept the output shape: the Visual Basic 6 IDE,
and twinBASIC. DeForm6 ships neither. It finds what the user owns and drives
it, the same way `--opcode-table` already reads a table the user builds on
their own install.

## Two finished products, not one

The phases below deliver two different things, and the first one is close.

| | What the user gets | Distance |
|---|---|---|
| **1.1** | The shell of the program: forms, controls, properties, names, signatures, API declarations. Verified to build. The user writes the logic. | About 3 months |
| **2.0** | The logic as well, for a P-code program. | About 2 to 3 years after 1.1 |

Version 1.1 is worth shipping on its own. A person who lost a program still
has to rebuild the user interface by hand today, and that is most of the
typing.

## A note on the numbers below

Every effort figure in this file is an estimate. It is not a measurement.
The measured numbers in this project live in `tests/ratios.toml` and in the
README, and each one names the test that asserts it. Nothing here does.

## Phases

### Phase 7: Byte fidelity

**Goal.** Measure what the reader does not understand, against the bytes
Microsoft's own compiler wrote.

For every structure the reader parses at offset `O` with length `L`, add one
method that writes exactly `L` bytes back. Re-emit each structure over its own
location in a copy of the original file, then compare against the original. No
addresses are computed, because the original's own addresses are reused.

Every byte that differs is a field the reader misreads or does not model.

**Why first.** It needs no compiler, no Windows host, and no purchase. It can
start today. It also de-risks every phase after it: if the reader misunderstands
a structure, statement recovery builds on sand.

**Exit.** A committed fidelity map over all 44 corpus programs. Every difference
is either a closed gap or a named limit in the register.

**Estimate.** 2 to 4 weeks.

#### Segment 1, done on 2026-09-15

The mechanism plus the VB header and the `Object` record. Measured over all 44
corpus programs and asserted by
`cargo test -p deform6 --test byte_fidelity`:

| Structure | Bytes reproduced | Of | Instances |
|---|---|---|---|
| VB header | 50 | 104 | 44 |
| `Object` | 20 | 48 | 105 |

No byte differs anywhere. `bound_proc_count` clamps `ProcCount` to what the
file can hold, and across 105 objects that clamp never fires.

Segment 1 also found one real gap in the reader and closed it. `Object` kept
only the name it resolved and threw away `lpszObjectName`, the address it read
the name through, so four bytes the reader plainly read could not be written
back. `Object` now carries that address.

#### What segment 1 learned, and how it reorders segment 2

**A verbatim field cannot disagree with itself.** A field read at one offset
and written back at the same offset, with no arithmetic between, is green by
construction. Most fields in most structures are verbatim, so most of this
phase reports coverage rather than correctness.

**The byte diff can only find a fault where the reader keeps a cooked value
in a field that the emitter writes back.** `bound_proc_count` is that shape.
It clamps `ProcCount`, and `Object.proc_count` keeps the clamped value, which
the emitter writes at `Object + 0x1C`.

**A correction.** An earlier version of this section listed five more sites
that cook a value, and said that three of them have the same shape as
`bound_proc_count`. That is not correct. No site in this list keeps its result
in a field that a structure writes back:

| Where | What it does | Why no byte can differ |
|---|---|---|
| `gui::bound_form_count` | clamps `wFormCount` | The clamped value only bounds the loop. `VbHeader.w_form_count` keeps the raw value. |
| `controlinfo::bound_control_count` | clamps `dwControlCount` | The clamped value only bounds the loop. The raw value is a local that the reader discards. |
| `controlinfo::bound_event_count` | clamps `wEventCount` | The clamped value only bounds the loop. `ControlInfo.w_event_count` keeps the raw value. |
| `functyp::vb_type_of` | substitutes zero for a missing type address | The default cannot occur. The type buffer walk stops before it keeps an entry that has no address. |
| `functyp::walk_type_buffer` | derives an argument count from `argSize` by a shift | The derived count only bounds the loop. |

The table names functions and not line numbers, because segment 2 moves those
lines.

A clamp that only bounds a loop makes the reader return fewer instances. It
never changes a byte, so the byte diff cannot see it. A census finds it: for
each array, the count that the file declares against the number of instances
that the reader returns.

**Segment 2 therefore adds that census.** First, three readers keep what they
read and discard today: `lStructSize` on the GUI table entry, `lpszName` on
`ControlInfo`, and `dwControlCount` and `lpControls` in a new
`OptionalObjectInfo` structure. Segment 2 then grades `ProjectInfo`,
`GuiTableEntry`, `ObjectInfo`, `PrivateObj`, `OptionalObjectInfo` and
`ControlInfo`. Every field that these six structures keep is verbatim, so their
ledgers report coverage only. The census is the part that can find a fault.
The `FuncTypDesc` header is not part of segment 2.

**Two limits cap what this phase can ever claim.** They are properties of the
method and not faults to fix.

The `Unmodelled` verdict means "this model cannot reproduce these bytes",
which is wider than "the reader never read them". A reader that reads a field,
uses it and does not keep it understands more than its coverage figure
reports. So a coverage figure is a floor, never a measure.

The grading is by byte and not by field, so a count that is wrong by a small
amount shows as a one byte run rather than a whole field. A person reading the
map needs the `emit` function open beside it.

#### Segment 2, done on 2026-09-16

Six more structures, and a census of four arrays. Measured over all 44 corpus
programs.

| Structure | Bytes reproduced | Record length | Records |
|---|---|---|---|
| `ProjectInfo` | 20 | 572 | 44 |
| GUI table entry | 8 | 80 | 53 |
| `ObjectInfo` | 6 | 56 | 105 |
| `PrivateObj` | 16 | 64 | 97 |
| `OptionalObjectInfo` | 8 | 64 | 97 |
| `ControlInfo` | 16 | 40 | 706 |

With the header and `Object` from segment 1, the walk grades 1251 records. No
byte differs from the file in any of them, and no two claim the same byte.

| Array | Declared | Returned |
|---|---|---|
| GUI table entries | 53 | 53 |
| Objects | 105 | 105 |
| `ControlInfo` entries | 706 | 706 |
| Event slots | 11862 | 11862 |

No array in the corpus comes up short. The census test changes a file in
memory to make each kind of shortfall happen, a clamp, an unmapped array, a
refused table and an unknown control type, and requires the census to name it.

Every constant above was computed by laying a value and reading back which
bytes the emitter wrote. None was worked out by hand, because segment 1 had
three arithmetic slips of exactly this kind.

#### What segment 2 found

**Three defects named the wrong byte, and are fixed.** `ImplausibleCount`
documents its offset as that of the count field. Three readers recorded the
start of the table the count bounds instead, so a report pointed a person at
the wrong byte. The first two rows were measured on corpus files patched in
memory, before the fix:

| Count | The defect named | The field is at |
|---|---|---|
| `wEventCount`, SK-Gradient | 6144, the event table | 5982 |
| `dwExternalCount`, Mandelbrot | 5896, the import table | 6732 |
| `wFormCount` | the start of the GUI table | `VBHeader + 0x44` |

The third row comes from a synthetic file. No corpus program can make that
clamp fire, for the reason given below.

The cause was structural. Each reader was handed a structure that did not know
where it sat in the file, so the table was the only offset it had.
`VbHeader`, `ProjectInfo` and `ControlInfo` now keep the file offset of their
own first byte. **This bends a rule segment 1 set**, that readers carry no
provenance and the fidelity walk works positions out for itself. The walk still
does. These three fields exist so that a defect can name its own byte.

A test now compares the census, which states where `wEventCount` sits from its
own reading of the layout, with the reader's own defect, and requires the two
to name one byte. Before the fix they named bytes 162 apart.

**The corpus rejects the disputed `ControlInfo` layout.** `STRUCTURES.md`
section 8.6 marks the first two fields `[D]`: one source puts `wEventCount` at
`0x04`, and three put it at `0x02`. The byte diff cannot settle this, because
the reader and the emitter put both fields at the same offsets. The event slots
settle it. In all 706 records, the count at `0x02` covers only nulls and
native stubs, and the word at `0x04` runs past the last slot. Section 17 of
`STRUCTURES.md` gives the numbers, and
`the_event_slots_fit_the_word_at_two_and_refuse_the_word_at_four_in_seven_hundred_and_six_control_info_records`
holds them.

**The presence rules agree.** The 8 objects with no `PrivateObj` are exactly
the 8 standard modules `inspect` reports, and exactly the 8 objects with no
`OptionalObjectInfo`. Two structures, two independent rules, one answer.

**Every class carries exactly one `ControlInfo` entry**, 44 of 44. The 706
`ControlInfo` entries are rows of the table that binds controls to events, and
the README's 686 controls count nodes in the control tree, which is a
different quantity.

**The GUI table clamp fires only when the table ends its section.** Anywhere
else, a raised `wFormCount` makes the walk read past the real entries and
refuse the table on the next `lStructSize`.

**The cost is small.** The walk costs 0.435 of what both `inspect` calls cost
over the corpus, so it caches nothing.

**The fuzz target found no fault in the walk.** A campaign of 500000 runs on
commit `dee22c0`, seeded as the scheduled job seeds it, reached coverage 5982.
An earlier run of 2045712 inputs is not evidence for the walk: its corpus was
not seeded, so almost no input got past the PE header.

#### What is left of Phase 7

- **The committed fidelity map.** An `xtask` subcommand that writes the
  coverage and census into a file, and a gate that holds the tree to it, in
  the shape `tests/ratios.toml` already has.
- **Variable-length structures.** Component table entries, control blocks and
  the property stream. `Emit` needs a fixed length, so these need a different
  contract.
- **The `FuncTypDesc` header.** It keeps flags as single bits, and a slate
  counts coverage by the byte, so it cannot say which bits of a byte are
  modelled. That needs coverage by the bit, and a reader that keeps the raw
  flag bytes.
- **The event stub.** `StubHandler.handler_address` is computed as
  `stub + 13 + rel32`. Written back independently, it would test that
  arithmetic. The stub's own address is kept on `EventSlot::Bound`, not on
  the handler, so an emitter needs the two together.
- **`GuiObjectInfo`.** The reader keeps one field of its 93 bytes,
  `lPropertiesLength`, and the structure holds a borrowed window, so it needs
  a record type the way `PrivateObj` did. Nothing here has graded it yet.

### Phase 8: The build gate

**Goal.** DeForm6 proves its own output compiles, on the user's machine, with
the user's compiler.

Add `--verify-build`. DeForm6 locates the Visual Basic 6 IDE or the twinBASIC
command line tool, runs it over the project it just wrote, captures the exit
status and the compiler's own diagnostics, and records the result in the JSON
report. It redistributes neither compiler.

**What this replaces.** The README states today that full recompilation did not
run, and that a structural check stood in for it. That sentence becomes a
measured field instead.

**A weakness to design around.** The build gate is necessary and it is weak. An
unknown type code becomes `Variant` at `write/code.rs:295`, and `Variant`
compiles. Four distinct COM types become `Object` at line 291. The project will
build while the signatures are wrong. Keep the differential harness. The build
gate does not replace it.

**Expect a tail of format faults.** The `Object=` lines for OCX references, the
five `Attribute` lines, the `.frx` byte offsets, and the code page assumption
are the likely first failures.

**Exit.** All 44 corpus programs extract and build, or each failure is a named
limit with the compiler's own error beside it.

**Estimate.** 4 to 8 weeks. Needs one Windows host and one compiler licence.

### Phase 9: The P-code corpus

**Goal.** 44 P-code binaries whose exact source is already held.

Every corpus program is native. 38 project files declare `CompilationType=0`
and none declares P-code. Statement recovery cannot be measured without P-code
inputs, and scavenging them gives binaries with no matching source.

Rebuild the existing corpus projects with `CompilationType=1` on the Phase 8
host. The ground truth is already in the repository.

This is also the first real exercise of the `lpNativeCode` P-code branch, which
the README records as never having run against a real P-code program.

**Estimate.** Days to 2 weeks, once the Phase 8 host exists.

### Phase 10: The P-code opcode table

**Goal.** A table that names each opcode and gives its argument width.

Derive it from the dispatch array in the runtime, together with the Microsoft
build that ships debug symbols naming the handlers. Public counts put this near
822 real handlers against 1531 slots.

Reuse the `xtask derive-opcode-table` pattern exactly: the tool is committed,
the table it produces is not, and the user builds it on a copy they lawfully
own.

**The hard part is argument width, not opcode names.** Every existing tool is
weakest there. The Phase 9 matched pairs let a width be verified against known
source rather than guessed.

**Estimate.** 4 to 8 months.

### Phase 11: P-code statement recovery

**Goal.** Procedure bodies come back as Visual Basic.

Retire one risk in week one: `STRUCTURES.md` section 10.3 has a single source at
confidence `[L]`, and the claim that a body occupies the `ProcSize` bytes
immediately before its descriptor is unverified. Check it against a real file
before anything is built on it.

Then: disassemble, lift to an intermediate form, recover control flow, and emit
Basic.

**The gate.** The recovered code compiles, which Phase 8 already measures, and
the rebuilt program behaves the same as the original.

**Estimate.** 12 to 24 months. This is the long pole.

### Phase 12: Native code, at its honest ceiling

**Goal.** Annotated disassembly for native builds, with the runtime helper calls
resolved into Visual Basic idioms.

Not compilable Basic. No public tool recovers compilable Basic from a native
build, and this one will not claim to either. State that limit in the README
before the first line is written, the way the current three limits are stated.

This phase is independent of 10 and 11 and can run beside them.

**Estimate.** 6 to 12 months.

### Phase 13: The workflow

**Goal.** One documented path from a compiled program to a rebuilt one.

Document the edit and rebuild route for both compilers. Make the failure
messages name the next action.

**Estimate.** 2 to 4 weeks.

## What blocks what

```
Phase 7  (no dependency, starts today)
Phase 8  (needs a Windows host and a compiler licence)
   |
   +-- Phase 9  (needs the Phase 8 host)
          |
          +-- Phase 10 (needs the Phase 9 corpus to verify widths)
                 |
                 +-- Phase 11 (needs the Phase 10 table)

Phase 12 runs beside 10 and 11.
Phase 13 closes the milestone.
```

Version 1.1 ships after Phase 8. Version 2.0 ships after Phase 11 and 13.

## Procurement

Three phases are blocked on hardware and a licence, so start this before the
code.

- A Windows host that runs 32-bit programs.
- One compiler. The Visual Basic 6 IDE, if a lawful copy is available, or
  twinBASIC.
- For automated builds in a pipeline, twinBASIC requires its Professional
  Edition. The free edition permits commercial use but not unattended command
  line builds.

## Open questions to close before planning around them

- Does a hosted Windows runner still carry the Visual Basic 6 runtime? If it
  does not, the build gate needs a different host.
- Does the twinBASIC command line accept a Visual Basic project file directly,
  or does it need a conversion step first?
