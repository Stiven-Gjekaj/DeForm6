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
The measured numbers in this project live in `tests/ratios.toml`, in
`tests/fidelity.toml` and in the README, and each one names the test that
asserts it. Nothing here does.

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

#### Segment 3, done on 2026-09-16

The committed fidelity map, and two more structures. Measured over all 44
corpus programs.

**The map.** `tests/fidelity.toml` holds one table for each corpus program.
The table gives the totals of each graded structure and of each counted
array. The file also holds one layout table for each structure.
`cargo run -p xtask -- update-fidelity` writes the file, and
`cargo test -p deform6 --test fidelity_map` holds the tree to it. When a
value moves, the test names the value, says whether the move is a
regression, and gives the new table to paste. A rewrite on a clean tree
gives no diff.

| Structure | Bytes reproduced | Record length | Records |
|---|---|---|---|
| `GUIObjectInfo` | 4 | 93 | 53 |
| Event stub | 13 | 13 | 390 |

With the eight structures of segments 1 and 2, the walk grades 1694 records.
No byte differs from the file in any of them, and no two claim the same byte.
16 records are absent: the `PrivateObj` and the `OptionalObjectInfo` of each
of the 8 standard modules. The reader refuses no record.

The event stub is the second structure in which a byte can differ. The reader
keeps the handler address, and the emitter works the jump back out of that
address. The emitter also writes the five opcode bytes, which the reader
assumes and does not read. All 13 bytes agree in all 390 stubs, so the
handler arithmetic is correct and every stub has the native shape.

`GUIObjectInfo` needs a record type, as `PrivateObj` does, but for a
different reason. Its `window` field is private, so a test outside `vb::gui`
cannot build the value that the reader gives.

#### What segment 3 found

**The single byte at `GUIObjectInfo` + 0x04 is real.** In all 53 forms, the
GUID of the GUI table entry is at 0x05. In no form is it at the aligned 0x04.
Section 18 of `STRUCTURES.md` gives the numbers.

**No event slot names a method stub.** No stub that a slot names holds
`0xFFFF`. The native stub bytes occur at 311 more places. Each of them holds
`0xFFFF`, and no four bytes of its file hold its address. So the reader's
`is_method` flag is never true on the corpus.

**`imm32` is one less than the word at `ControlInfo` + 0x04**, in all 390
stubs. Section 17 of `STRUCTURES.md` could not name that word, and it still
cannot. Section 19 gives the numbers.

**A P-code stub did not give a wrong handler address.** The reader did not
read the opcodes. At the addresses of a corpus program, the last byte of a
P-code stub makes the jump go below address 0, so the reader kept no handler
and reported an `UnreadablePointer` defect. That defect named the wrong fault:
the address is readable, and the stub has another shape. The reader now
checks the opcodes, and such a stub gets an `UnknownStubShape` defect at the
stub. `tests/stub_shapes.rs` requires this, and the fidelity walk records the
stub as refused.

**`lObjectID` gives the object of each form.** In all 44 programs, the
`lObjectID` values of the GUI table entries, in order, are the indexes of the
form objects.

**The reader keeps `imm32`.** `StubHandler` has a new field for it. The
struct is now `#[non_exhaustive]`, so a later field does not break a caller.

**The fuzz target found no fault.** Two campaigns ran on commit `69b5618`,
seeded as `.github/workflows/fuzz.yml` seeds them. The campaign of 60
seconds ran 90592 inputs and reached coverage 5934. The campaign of 500000
runs reached coverage 6234, and its highest `rss:` was 979 MiB, below the
1159 MiB of the campaign on `dee22c0`. `crates/xtask/src/fuzz.rs` records
it.

**The walk does not make the fuzz target slower.** The campaign of 500000
runs took 1936 seconds, and the one on `dee22c0` took 537, but other work
ran on the machine at the same time. So the two fuzz targets were timed
again over the 1551 inputs of the final corpus, one after the other, two
times each. The target of `1298a46` took 23.5 and 21.3 seconds. The target
of `77663e6`, before this segment, took 22.6 and 21.6 seconds.

#### Segment 4, done on 2026-09-17

The object table and the `Declare` table, and a census row for the
`Declare` table. Measured over all 44 corpus programs.

| Structure | Bytes reproduced | Record length | Records |
|---|---|---|---|
| Object table | 12 | 84 | 44 |
| `Declare` table entry | 8 | 8 | 249 |
| `Declare` descriptor | 8 | 24 | 220 |

With the ten structures of segments 1 to 3, the walk grades 2207 records.
No byte differs from the file in any of them, and no two claim the same byte.
The reader refuses no record.

| Array | Declared | Returned |
|---|---|---|
| `Declare` table entries | 249 | 249 |

The census now counts five arrays in 935 rows, and no row comes up short.

The walk grades a descriptor only for an entry of type 7, and it grades each
descriptor one time. An entry of type 6 names a pair of a different shape,
so it gets no row. An entry of a different type gets a refused row.

Two reader changes came first, each in its own commit. The head of the
object table keeps `lpObjectArray`, and the `Declare` table keeps each entry
that it reads, of every type, with the two addresses at the start of each
descriptor. Each change is additive, because each of the two structures
already has a private field.

#### What segment 4 found

**A `Declare` count could fail its check and give no defect.** The reader
multiplied `dwExternalCount` by 8 before it compared the count with the
table. For a count of `0x2000_0000` or more, the product left a `u32`, and
the check raised no defect. The loop still stopped at the end of the
table's region, so the reader read fewer entries than the file declared and
said nothing. The census would have called that row unexplained. On
Mandelbrot patched in memory, counts of `0xFFFF` and `0x1FFF_FFFF` gave a
defect, and `0x2000_0000` and `0xFFFF_FFFF` did not. The reader now divides
the length of the region by 8, so each count that is larger than the table
can hold gives the defect. A unit test requires it for `0x2000_0000` and
`0xFFFF_FFFF`, and the census test requires a clamped row for `0x2000_0000`.

**The descriptor of an external `Declare` entry is 24 bytes.**
`STRUCTURES.md` section 7.1 names 8. In all 220 descriptors, machine code
starts at 0x18. Its first instruction loads from the address at 0x0C plus 8,
and a later instruction pushes the address of the descriptor. The bytes at
0x08, 0x10 and 0x14 hold the same values in all 220, and no source names
them. Section 21 of `STRUCTURES.md` gives the numbers.

**Two rows of `STRUCTURES.md` section 4 were wrong for the corpus.** The
sources say that `lpExecProj` is zero on disk, and that `lpProjectObject` is
used only in memory. In all 44 programs, each holds an address in `.data`.
Section 20 of `STRUCTURES.md` gives the values of all the fields. The object
array starts where the 84 bytes of the table end, in all 44.

**29 entries are internal, one in each of 29 programs.** It is entry 0 in 28
programs and entry 17 in `Edge_Detection.exe`. The reader comments said that
the largest count is 9 and that every third program takes the internal path.
The largest count is 26, and 29 of the 44 programs have an internal entry.

**The OCX header holds `0x248DD892` at `+0x10`** in all three corpus
headers. `STRUCTURES.md` section 8.7 calls that field reserved.

**Three `Declare` defects name the wrong byte.** A defect about
`lpImportDescriptor`, `lpDllName` or `lpApiName` gives the first byte of the
entry. The first field is at `+0x04` of the entry, and the other two are in
the descriptor. This segment did not change them. A later commit makes each
of the three give the byte of its own pointer.

**The fuzz target found no fault.** Two campaigns ran on commit `ce7df1f`,
seeded as `.github/workflows/fuzz.yml` seeds them. The campaign of 60
seconds ran 91428 inputs and reached coverage 6198. The campaign of 500000
runs took 467 seconds and reached coverage 6575. Its highest `rss:` was
977 MiB, below the 1159 MiB of the campaign on `dee22c0`.
`crates/xtask/src/fuzz.rs` records it.

**The walk does not make the fuzz target slower.** A hostile input can make
the walk grade one `Declare` entry for each 8 bytes of a section, so the two
fuzz targets were timed over the 1538 files of the final corpus, one after
the other, two times each. The target of `ce7df1f` took 6.5 and 6.5
seconds. The target of `09c3cfe`, before this segment, took 8.0 and 6.5
seconds.

#### Did Phase 7 reach its exit

**The exit, as written, is met.** The fidelity map is committed, and it
covers all 44 corpus programs. It holds no byte that differs, so no
difference needs a closed gap or a named limit.

**The goal is not met.** The goal asks for an emitter for every structure
that the reader parses. These structures have none yet:

- **Fixed-length structures.** The OCX header (24 bytes) sits inside the
  property stream, so the walk can reach it only through the stream. The
  pair that an internal `Declare` entry names has no reader.
- **Arrays of pointers.** The procedure name array, the event descriptor
  array, and the event slots. The census counts the event slots, but no
  emitter writes back the bytes of any of the three arrays.
- **The `FuncTypDesc` header.** It keeps flags as single bits, and a slate
  counts coverage by the byte, so it cannot say which bits of a byte are
  modelled. That needs coverage by the bit, and a reader that keeps the raw
  flag bytes.
- **Variable-length structures.** Component table entries, control blocks
  and the property stream. `Emit` needs a fixed length, so these need a
  different contract.

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

#### Part 1, done on 2026-09-25

**The host.** The author's Windows XP virtual machine in UTM (`ver` gives
5.1.2600), with the Visual Basic 6 IDE (`VB6.EXE` is 1,895,424 bytes, dated
2004-02-23). The install media beside it are XP Professional with SP3, VB6
Enterprise Edition, and VB6 SP6. The machine has no network, on purpose.

**DeForm6 does not run on this host.** The Rust standard library needs
Windows 10, or Windows 7 through tier-3 targets. So `--verify-build`, where
DeForm6 runs the compiler on the same machine, waits for a Windows 10 host.
Part 1 splits the work into three steps:

1. `cargo run -p xtask -- export-builds <dir>` writes the 44 corpus programs.
   For each one, it writes the files that DeForm6 writes, and a copy of the
   project in the corpus. It also writes a manifest, `build.bat` and
   `sendlogs.bat`.
2. The host builds each project with `VB6.EXE /make`. The export goes in on a
   CD image, which `hdiutil makehybrid -iso -joliet` makes from the export
   directory. `build.bat` writes one log and one exit code for each build.
   `sendlogs.bat` sends them out through the serial port `COM1`, which UTM
   joins to a terminal device on the Mac.
3. `cargo run -p xtask -- import-builds --capture <file> <dir>` reads the
   serial capture into `tests/builds.toml`. It checks the size of each file,
   so a byte that the serial line lost or added stops the import.

**What VB6 reports.** A probe of four small projects, which `export-builds
--probe` writes, measured the shapes before the importer was written:

| Probe | Exit code | What `/make` writes |
|---|---|---|
| A project that builds | 0 | `Build of '<name>' succeeded.` |
| A syntax error | 1 | the error, then `Build of '<name>' failed.` |
| A component that is not on the host | 1 | `'<file>' could not be loaded`, and no line about the build |
| A property that VB6 does not know | 1 | three lines, and a `.log` file beside the form with the line of the property |

Each project gave the same result twice, and no dialog stopped a build.

**The first run.**

| Side | Built | Failed |
|---|---|---|
| The projects that DeForm6 writes | 41 of 44 | 3 |
| The projects in the corpus | 42 of 44 | 2 |

- The three projects of DeForm6 that fail are the three that use the Winsock
  control. VB6 writes `'MSWINSCK.OCX' could not be loaded`. The `Object=` line
  gives an identifier that is one byte away from the one that the original
  project declares, which this section expected.
- `Edge_Detection` in the corpus fails, because its project names
  `cCommonDialog.cls`, and that file is not in its directory. DeForm6's
  project for the same program builds.
- `HMM` in the corpus fails, because `frmHMM.frx` is damaged upstream, as the
  README says. VB6 cannot set the `Text` property at line 509 of
  `frmHMM.frm`. DeForm6's project for the same program builds.

**The record stays true.** `tests/builds.toml` holds a hash of the files that
the host built, for each program. `cargo test -p deform6 --test build_record`
writes the files again and compares the hashes. When DeForm6 writes
different files, the gate fails with `STALE` until the host builds the new
files.

#### Part 2, done on 2026-09-25

**Two faults, fixed one at a time.** Each fix changed the files of the three
Winsock projects. So the host built the 44 programs again after each fix.

1. The `Object=` line. The executable does not hold the type library
   identifier that the line declares, as `docs/STRUCTURES.md` section 7.3.1
   records. It holds the class identifier of the control. DeForm6 now keeps a
   table of one row, measured in the corpus: the class identifier of the
   Winsock control gives the line that the three corpus projects declare.
   The line is graded `inferred`. VB6 then loaded the control library, and
   the next fault showed.
2. The class of the control. The form gave the class `VB.Control`, which VB6
   does not know. The form now gives the class name that the file holds,
   `MSWinsockLib.Winsock`.

**A shape that the probe did not show.** A control class that VB6 cannot load
does not stop `/make`. VB6 puts a picture box in place of the control, gives
exit code 0, writes `Build of '<name>' succeeded.`, and writes a `.log` file
beside the form. The record calls this result `built with load errors`, so
that a build of a changed project does not count as `built`.

| Run | Built | Built with load errors | Failed |
|---|---|---|---|
| After the `Object=` fix | 41 of 44 | 3 | 0 |
| After the class fix | 44 of 44 | 0 | 0 |

The projects in the corpus gave the same result in each run: 42 of 44 built.

**The exit is met.** VB6 builds each of the 44 projects that DeForm6 writes.
The two corpus projects that do not build fail because of faults upstream,
which the first run names.

**What stays open.** The table has one row. For another control, DeForm6
writes the class identifier on the `Object=` line, and VB6 does not load that
line. A second control in the corpus can add a row. `--verify-build` waits for
a Windows 10 host.

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
Phase 8  (exit met on the author's XP host; --verify-build waits)
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

Three phases needed hardware and a licence. The author's host now gives both:
Windows XP with the Visual Basic 6 IDE, in a virtual machine.

- A Windows host that runs 32-bit programs. Done.
- One compiler. The Visual Basic 6 IDE, if a lawful copy is available, or
  twinBASIC. Done.
- A Windows 10 host, for `--verify-build`, where DeForm6 itself must run.
- For automated builds in a pipeline, twinBASIC requires its Professional
  Edition. The free edition permits commercial use but not unattended command
  line builds.

## Open questions to close before planning around them

- Does a hosted Windows runner still carry the Visual Basic 6 runtime? This
  no longer blocks the build gate: the record is a measurement from the
  author's host, and the gate checks that it covers the files that DeForm6
  writes now. It matters only for a gate that builds on each push.
- Does the twinBASIC command line accept a Visual Basic project file directly,
  or does it need a conversion step first?
