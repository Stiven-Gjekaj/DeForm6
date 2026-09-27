# Changelog

This file has no format precedent in this repository, so this note states
the one it uses. Every release gets one level two heading in the form
`## [X.Y.Z] - YYYY-MM-DD`. The newest release comes first. Each section
states what the release delivers, what stays open, and what the release
does not do. A change that no release holds yet goes under one
`## [Unreleased]` heading above the newest release. A release gives that
heading its version and its date.

## [Unreleased]

### What this delivers

- The P-code corpus: `corpus-pcode/` holds a P-code build of 42 of the 44
  corpus programs. The Visual Basic 6 IDE built each one on the author's
  Windows XP host from the corpus source, with one changed line in the
  project file: `CompilationType=-1`. `corpus-pcode/NOTICES` gives the origin
  and the licence of each one.
- `tests/pcode.toml` records the result of each build, the hash of each
  source and the SHA-256 of each binary. `cargo test -p deform6 --test
  pcode_record` holds the tree to it, and fails with `STALE` when a corpus
  source changes.
- `cargo run -p xtask -- export-pcode [--probe] <dir>` writes the projects for
  the host, and `cargo run -p xtask -- import-pcode [--capture <file>] <dir>`
  reads the build back. The probe measured that `-1` and `1` both give
  P-code, and that the serial line carries a binary unchanged.
- The first run of the P-code branch: `ProjectInfo.lpNativeCode` is 0 in each
  of the 42 binaries, and DeForm6 reports each one as P-code. The no-panic
  sweep reads the 42 binaries as a fourth source.
- What DeForm6 recovers from P-code: `cargo test -p deform6 --test
  pcode_recovery` holds each P-code binary to its source and to the native
  build of the same source. Each one gives the objects of its source project
  file, by name and by kind, in the order of that file, and DeForm6 writes
  the same project from it as from the native build. Over the 42 programs,
  the P-code builds recover 179 of 871 public procedure names, 50 forms, 626
  controls and 741 property records, which are the numbers of the native
  builds.
- The P-code event stub: DeForm6 decodes it. The stub is 20 bytes, and each
  bound event slot of a P-code program now gives a handler address: the
  value that the stub loads into `edx`. In each P-code corpus program, that
  value is the entry of the method table of the object for the handler
  procedure, and the stub goes to `MethCallEngine`. The 42 programs give 388
  handler addresses and no `UnknownStubShape` defect.
- The fidelity walk records a P-code stub as an absent `EventStub` record,
  because its emitter writes the native stub only.
- `vb::procdesc` reads the method table of an object and each P-code
  procedure descriptor that it names. The body of a procedure is the
  `ProcSize` bytes before its descriptor. The 42 P-code programs give 680
  descriptors, and a test holds each body to its descriptor. `inspect` does
  not report the bodies yet.
- `cargo run -p xtask -- fetch-pcode-symbols <dll>` fetches the symbol file
  of a copy of `MSVBVM60.DLL` from the symbol server of Microsoft, and
  `cargo run -p xtask -- derive-pcode-table <dll> <pdb>` writes the P-code
  dispatch table with the name and the argument width of each handler.
  Neither file enters the repository. `cargo run -p xtask --
  check-pcode-table` decodes each P-code body of the corpus with that table:
  680 of 680 decode to their end.
- `vb::pcode` decodes a P-code body with that table, and `deform6 disasm
  <exe> --pcode-table <table>` prints the P-code of each procedure. The
  listing gives opcodes, not Basic.
- `vb::lift` follows the stack of a decoded body and gives its statements,
  and `deform6 disasm --lift` prints them. It lifts a procedure only when it
  knows the stack effect of each opcode, so a procedure with a call does not
  lift. `check-pcode-table` counts the bodies that lift: 69 of 680. The
  statements name each variable by its offset, and `extract` writes none of
  them.

### What stays open

- Two corpus programs have no P-code build, because their source does not
  build: `Edge_Detection` and `HMM`.

### What this does not do

- It does not change the version in `Cargo.toml`, and it has no tag.
- It adds `StubShape`, the field `StubHandler::shape`, and the modules
  `vb::procdesc`, `vb::pcode` and `vb::lift` to the library. It
  removes no public item, and it changes the type of none. For a P-code
  program, a report now gives a handler address where it gave none, and the
  text of an `UnknownStubShape` defect names both shapes.

## [2.0.0] - 2026-09-26

### What this release delivers

- Byte fidelity: `deform6::fidelity::walk::walk` writes thirteen
  structures back over the bytes they were read from. It grades each byte as
  the same, as different, or as not modelled. Across the 44 corpus programs
  it grades 2207 records, and no byte that a reader models differs from the
  file.
- The thirteen include `GUIObjectInfo` and the event stub. The event stub
  emitter writes all 13 bytes. It works the jump back out of the handler
  address that the reader keeps, and it writes the five opcode bytes that
  the reader assumes and does not read.
- The thirteen also include the object table, each `Declare` table entry,
  and the descriptor of each external entry. In the corpus the descriptor is
  24 bytes, and machine code that uses it follows it. The reader reads its
  first 8 bytes, which hold the addresses of the two names.
- The committed fidelity map: `tests/fidelity.toml` holds the totals of each
  corpus program, and `cargo run -p xtask -- update-fidelity` writes it.
  `cargo test -p deform6 --test fidelity_map` fails when a value moves. The
  message names the value, says whether the move is a regression, and gives
  the table to paste.
- The array census: the same walk counts five arrays, which are the
  `Declare` table, the GUI table, the object array, the controls of each
  object and the event slots of each control. For each one it compares the
  count that the file declares with the number of entries that the reader
  returns. No array in the corpus comes up short.
- Corpus evidence: gap 2, the `OptionalObjectInfo` presence test, is
  closed. The dispute about `fControlType` and `wEventCount` at the start of
  `ControlInfo` is settled. Sections 16 and 17 of `docs/STRUCTURES.md` give
  the numbers.
- More corpus evidence: the single byte at `GUIObjectInfo + 0x04` is real,
  and no event slot names a method stub. Sections 18 and 19 of
  `docs/STRUCTURES.md` give the numbers.
- The object table and the `Declare` table as the corpus holds them:
  sections 20 and 21 of `docs/STRUCTURES.md`. Two rows of section 4 are
  corrected, because `lpExecProj` and `lpProjectObject` hold an address in
  each corpus program. Section 8.7 gives the value that each corpus OCX
  header holds at `+0x10`.
- `inspect` checks the two opcodes of each event stub. A stub of another
  shape, such as a P-code stub, gets an `UnknownStubShape` defect at the
  stub, and its slot keeps no handler address.
- The gap 2 cross-check runs: `inspect` compares bit `0x2` of
  `fObjectType` with `lpPrivateObject` for each object, and reports a
  disagreement as a `ModuleMarkerMismatch` defect at `ObjectInfo + 0x0C`.
  The defect is `Tolerated`, so a strict run reports it and continues. No
  corpus program raises it.
- `sh scripts/gate.sh` runs the whole gate on a local machine.
- The build record: `tests/builds.toml` holds what the Visual Basic 6 IDE
  built for each corpus program, on the author's Windows XP host. It built
  all 44 projects that DeForm6 wrote, and 42 of the 44 projects in the
  corpus. `cargo run -p xtask -- export-builds <dir>` writes the projects,
  `build.bat` and `sendlogs.bat`. `cargo run -p xtask -- import-builds
  [--capture <file>] <dir>` reads the logs back, from the directory or from a
  serial capture. `export-builds --probe` writes the four small projects
  that measured how VB6 reports a build.
- `cargo test -p deform6 --test build_record` holds the tree to the record.
  When DeForm6 writes different files for a program than the host built, the
  gate fails with `STALE`, until the host builds the new files.
- The record has a fourth result, `built with load errors`. The importer
  gives it when VB6 changes a project to load it, and then builds it. For
  example, VB6 puts a picture box in place of a control whose class it
  cannot load.
- The `Object=` line of the Winsock control is the line that the corpus
  projects declare. The executable holds the class identifier of the
  control, not the type library identifier of the line. A table of one row,
  measured in the corpus, joins the two, and the line is graded `inferred`.
- An external control gets the class name that the file gives, such as
  `MSWinsockLib.Winsock`, on its `Begin` line. It got `VB.Control`, which
  VB6 does not know. A class name that is not two names joined by one dot
  writes no block, and an `unrecoverable` item names the control.

### What changes for a caller

These changes break code that was written against 1.0.0. This release is
therefore 2.0.0, not 1.1.0.

- New public fields: `VbHeader::file_offset`, `VbHeader::rva`,
  `ProjectInfo::file_offset`, `ProjectInfo::rva`,
  `ControlInfo::file_offset`, `ControlInfo::rva`, `ControlInfo::lpsz_name`,
  `Object::file_offset`, `Object::rva`, `Object::lpsz_object_name`,
  `GuiTableEntry::l_struct_size` and `StubHandler::imm32`. Code that builds
  one of these structures with a struct expression does not compile. Code
  that names every field of one in a pattern does not compile. `imm32` does
  not go into the JSON report.
- These six structures and the new `OptionalObjectInfo` are now
  `#[non_exhaustive]`. Outside this crate, do not build them with a struct
  expression, and put `..` in each pattern that names their fields. Then a
  field that a later release adds does not break a caller again.
- Three defects in the JSON report now give the offset of the count that
  was clamped, not the offset of the table that the count bounds. A clamped
  `wFormCount` names the structure `VBHeader`, at `VBHeader + 0x44`, where
  1.0.0 named `GuiTable`, at the start of the GUI table. A clamped
  `wEventCount` gives `ControlInfo + 0x02`. A clamped `dwExternalCount`
  gives `ProjectInfo + 0x238`, and its `rva` is the address of that byte,
  where 1.0.0 gave the address of the `Declare` table.
- Three `Declare` defects now give the offset of the pointer that held the
  address, as `ItemAddressUnmapped` documents. A defect about
  `lpImportDescriptor` gives `+ 0x04` of the entry. A defect about
  `lpDllName` or `lpApiName` gives `+ 0x00` or `+ 0x04` of the descriptor,
  and names the structure `DeclareDescriptor`. 1.0.0 gave the first byte of
  the entry for all three, and named the structure `DeclareTableEntry`.
- A `Declare` table whose address is in no section, while
  `dwExternalCount` is not zero, now gives `ItemAddressUnmapped` at
  `ProjectInfo + 0x234`. 1.0.0 gave an empty list and no defect, which said
  that the project declares nothing. A strict run now refuses such a file,
  and a salvage run reports no `Declare` statement and the defect.
- A component table whose address is in no section, while `wExternalCount`
  is not zero, now gives `ItemAddressUnmapped` at `VBHeader + 0x50`, the
  `lpExternalTable` field of the header. 1.0.0 gave an empty list and no
  defect, which said that the program uses no component. A strict run now
  refuses such a file, and a salvage run reports no component and the
  defect.
- `ComponentTable::read` now takes the `VbHeader` in place of the address
  and the count of the table, as `DeclareTable::read` takes the
  `ProjectInfo`. Code that calls it with the address and the count does not
  compile.
- Two `Declare` failures now give a defect that states what the file holds.
  A descriptor whose address maps, in a section that ends before its 8
  bytes, gives `ItemCutShort`. A library name or an export name with no NUL
  in the bytes that the reader searched gives `NoNulTerminator`, at the
  offset of the text, with the number of bytes searched. 1.0.0 gave
  `ItemAddressUnmapped` for both, and its message says that the address is
  in no section. All three kinds are `Recoverable`, so a strict run refuses
  these files as before.
- `Site::rva` is now the address of the byte at `Site::offset`. It never
  holds an address that the byte points at. At each pointer defect, 1.0.0
  gave the address that the pointer holds: at `lpszObjectName`,
  `lpszName`, an event slot, `lpProcNamesArray`, `lpFuncTypeInfo`,
  `lpAryArgNames`, `optionalVals`, `lpObjectInfo`, `lpPrivateObject` and
  the three `Declare` pointers. The kind of each of these defects still
  gives that address. A `constFFFF` defect gave the address where its
  `FuncTypDesc` starts, and a section overlap gave the address where the
  other section starts.
- Most defects about a count, a `Declare` or component entry, or the form
  stream gave `null` in `Site::rva`, and they now give the address of their
  byte too. `Site::rva` is `null` only when the byte is in no section, as
  for a section overlap, or when the reader does not know where the byte
  is, as for a structure that a form cannot read and whose offset is 0.
- Each `NoNulTerminator` defect now gives the file offset where the text
  starts, and the number of bytes that the search read, as the kind
  documents. For the names of objects, controls, procedures and arguments,
  1.0.0 gave the offset of the pointer and the bound. A search reads fewer
  bytes than the bound when the section ends first. The site of each of
  these defects is still the pointer.
- A defect about `ObjectInfo.lpPrivateObject`, when the reader cannot read
  the private object, now gives `ObjectInfo + 0x0C` in the site and in the
  kind, as the defect of the module marker check does. 1.0.0 gave offset 0.
- A defect about `Object.lpObjectInfo`, when the reader cannot read the
  `ObjectInfo`, now gives the first byte of the `Object` element, which
  holds `lpObjectInfo`, in the site and in the kind. `Site::rva` gives the
  address of that byte. 1.0.0 gave offset 0.
- A section overlap now gives the offset of the `VirtualAddress` field of
  the second section header, 12 bytes into the header, in the site and in
  the kind. 1.0.0 gave the first byte of the header, although the site named
  `VirtualAddress`. The message now says that the two sections claim the
  same addresses, which is what the check compares.
- When the 72 bytes at `GUIDoffset` or the 16 bytes at `oUuid` run past the
  end of their component entry, the defect is now `RunsPastEnd`, and its
  site is that offset field, at `+ 0x1C` or `+ 0x04` of the entry. 1.0.0
  gave `ImplausibleCount` at the first byte of the entry, and neither field
  is a count. The severity moves from `Recoverable` to `Tolerated`, as
  `GuidLengthUnexpected` already is for the same loss, so a strict run no
  longer refuses a file for these two defects.
- A component entry that gives no component now gives a defect that names
  the failure. An entry shorter than its `0x34` bytes of fixed fields gives
  the new kind `ItemLengthTooSmall` at `StructLength`, with the length and
  the 52 bytes that the fixed fields need. A string with no NUL gives
  `NoNulTerminator` at the offset field that names it, which is
  `FileNameOffset`, `SourceOffset` or `NameOffset`. 1.0.0 gave
  `NoNulTerminator` at the first byte of the entry, named `NameOffset`, for
  each of these. Both kinds are `Recoverable`, so a strict run refuses these
  files as before.
- A `StructLength` of 0 also gives `ItemLengthTooSmall`, and the walk still
  stops there. 1.0.0 gave `CountMismatch` between 0 and 1, and neither
  number is a count in the file. The kind is `Recoverable`, as before.
- `GuidLengthUnexpected` now gives the offset of the `GUIDlength` field, at
  `+ 0x20` of the component entry, in the site and in the kind. 1.0.0 gave
  the first byte of the entry, which is `StructLength`, although the site
  named `GUIDlength`.
- The three defects of the control header now give the field that each one
  is about, in the site and in the kind. `IndexHighByteSet` gives the
  two-byte index at `+ 0x05` of the control block. `EmptyName`, and
  `ImplausibleCount` for a name that runs past the block, give the two-byte
  length of the name. That length is at `+ 0x07` in an element of a control
  array, and at `+ 0x05` in another block. 1.0.0 gave the first byte of the
  control block for all three.
- A string whose declared length runs past its window gives
  `ImplausibleCount`. Its `max` is now the largest length that the window
  allows: the bytes from the length field to the end of the window, less 2
  for the length field and 1 for the trailing NUL. 1.0.0 gave the bytes from
  the length field to the end of the window.
- A type buffer that does not close gives `CountMismatch` at `argSize`. Its
  `count` is now the count that `argSize` gives, as `CountMismatch`
  documents, and `expected` is the number of entries that the type buffer
  holds. `other_field` is now `the type buffer`. 1.0.0 gave the two counts
  in the other order, and gave `argSize` as the other field.
- The evidence of an `Object=` line in the JSON report now has a note that
  its offset is of the sixteen bytes that `oUuid` names, and not of the
  field. The caveat on a joined CLSID says the same. The offset does not
  change.
- New public methods that break no caller: `Region::rva` gives the address
  of a byte in a window that `PeImage::region_at` built, and
  `Region::cstr_span` gives the number of bytes that `Region::cstr`
  searches.
- New public fields that break no caller: `ObjectTableHead::lp_object_array`
  and `DeclareTable::entries`. Each of the two structures already has a
  private field, so no code outside this crate builds one or names every
  field of one.
- New `#[non_exhaustive]` types: `vb::project::DeclareTableEntry` and
  `vb::project::DeclareDescriptor`. `DeclareTable::entries` holds one entry
  for each 8 bytes that the reader read, of every type, in table order.
- `fidelity::census::Array` has the new variant `DeclareEntries`, and
  `fidelity::census::Owner` has the new variant `Declare`. A `match` on
  either with no wildcard arm does not compile until it names the new
  variant.
- A `dwExternalCount` of `0x2000_0000` or more now gives
  `ImplausibleCount`. 1.0.0 gave no defect for such a count, because the
  size of the table in bytes left a `u32`, and the loop still stopped at the
  end of the table's region. A strict run can now refuse such a file even
  when no entry gives a defect.
- `DefectKind` has ten new variants, `ModuleMarkerMismatch`,
  `UnknownStubShape`, `ItemCutShort`, `RunsPastEnd`, `UnexpectedConstant`,
  `NotAnIdentifier`, `JumpOutOfRange`, `UnknownValue`, `ItemTypeUnknown`
  and `ItemLengthTooSmall`, and `schema/report.schema.json` accepts all ten.
  A `match` on `DefectKind` with no wildcard arm does not compile until it
  names them.
- An event stub without the native opcodes now gives `UnknownStubShape` at
  the stub. 1.0.0 gave `UnreadablePointer` at the slot when the jump left
  the address space, and otherwise a wrong handler address with no defect.
- Failures whose address resolves no longer give `UnreadablePointer`, whose
  message says that the address resolves to nothing. Bytes that their
  section cuts short now give `RunsPastEnd`, with the offset of the bytes,
  their number and the offset where their section or their block ends. This
  holds for an event stub, a `FuncTypDesc` header, an `ObjectInfo`, a
  `PrivateObj`, and the bytes of `optionalVals`, which can also run past the
  end that `cbValues` gives. A `constFFFF` that is not `0xFFFF` now gives
  `UnexpectedConstant`, with the value that the format gives and the value
  that the field holds. A procedure name that is not an identifier now
  gives `NotAnIdentifier`, with the offset and the length of the text. A
  native stub whose jump leaves the address space now gives
  `JumpOutOfRange`.
- The four new kinds are `Tolerated`, as `UnreadablePointer` is, so a
  strict run refuses the same files. Two of these defects name a different
  byte. A `FuncTypDesc` header that its section cuts short names the slot
  of `lpFuncTypeInfo` that held its address, where 1.0.0 named the first
  byte of the record as `argSize`. A jump that leaves the address space
  names the `rel32` field of the stub, where 1.0.0 named the slot.
- Six defects gave a kind about a count for a value that is not a count.
  Each one now gives a kind that says what the file holds:
  - A property payload that runs past the end of its control block gives
    `RunsPastEnd`, with the bytes that the read needs. For a fixed payload
    and for a string, the site is now the first byte of the payload, where
    1.0.0 gave the opcode. 1.0.0 gave `ImplausibleCount`.
  - A blob length field whose four bytes run past the end of the control
    block gives `RunsPastEnd`. 1.0.0 gave `ImplausibleCount` with a count
    of 4.
  - A value record of `optionalVals` whose padding runs past the end that
    `cbValues` gives now gives `RunsPastEnd`, with all the bytes of the
    record. 1.0.0 gave `CountMismatch` between the cursor and `cbValues`.
  - A value tag of `optionalVals` that is not one of the six tags gives the
    new kind `UnknownValue`, at the tag. 1.0.0 gave `CountMismatch` between
    the tag and 0, at `optionalVals`.
  - A leading byte of a type buffer that is neither `0x1E` nor `0x00` gives
    `UnknownValue` at the byte. 1.0.0 gave `CountMismatch` at `argSize`,
    although the two counts could agree.
  - An `optionalVals` block that holds more value records than the reader
    reads gives `StructureUnreadable` at the block. 1.0.0 gave
    `CountMismatch` between the cursor and `cbValues`.
- These six defects were `Recoverable` and are now `Tolerated`, so a strict
  run no longer refuses a file for them. The reader loses the same item as
  before, and invents nothing in its place.
- A `Declare` entry whose `dwEntryType` is neither 6 nor 7 gives the new
  kind `ItemTypeUnknown`, with the offset of the type and its value. 1.0.0
  gave `CountMismatch` between the type and 7. The kind is `Recoverable`, as
  the other kinds for a skipped `Declare` entry are, so a strict run refuses
  such a file as before.
- `vb::classify::agree` now treats an `lpPrivateObject` of `0` as no private
  object, as `PrivateObj::read` already did. It now gives `false` for a form
  whose pointer is `0`, and `true` for a module whose pointer is `0`. The new
  `vb::classify::names_no_private_object` states that rule for both.
- New public field: `write::model::ControlModel::external_class`. Code that
  builds a `ControlModel` with a struct literal must give it.
- A component table that ends before the count of `wExternalCount` now
  gives an `ImplausibleCount` defect at `VBHeader + 0x46`, with the number of
  entries that the walk found. 1.0.0 gave those entries and no defect. The
  kind is `Recoverable`, so a strict run now refuses such a file.

### What stays open

- DeForm6 knows the `Object=` line of the Winsock control only. For
  another control, it writes the class identifier of the control on that
  line, and VB6 does not load the line.
- `--verify-build`, where DeForm6 runs the compiler of the user, waits for a
  Windows 10 host. DeForm6 does not run on Windows XP.
- The fidelity walk grades thirteen structures. `docs/ROADMAP.md` names
  the work on the other structures that is left.
- The census does not count the type buffer. The clamp on that array only
  bounds a loop, so the byte diff cannot see it either.
- The internal `damaged` helper still leaks one message for each refusal.
  The scheduled fuzz job is held to a measured peak resident set, not to a
  model of the leak.

### What this release does not do

- DeForm6 does not recover statements. The code inside a procedure does not
  come back.
- The P-code branch was never run. Every program in the test corpus is
  native, so this release has never proved the P-code branch against a real
  program.
- No test starts VB6. The author's Windows host built the projects, and
  `tests/builds.toml` holds the result.

## [1.0.0] - 2026-09-14

### What this release delivers

- Identification: the tool reads a PE file, confirms the `VB5!` signature,
  tells VB6 from VB5 by the imported runtime DLL, refuses anything else with
  one clear sentence, and reports whether a program is native or P-code.
- The object graph: the tool walks the object table and recovers the name of
  every form, module and class, tells them apart, recovers public procedure
  names and signatures, and recovers the `Declare` statements for external
  API calls.
- The forms: the tool recovers the control tree, the control types and
  names, property values, a third party control's declared component
  identifier, the resource blobs, and the event structure of each control,
  within the limits FRM-01, FRM-02 and FRM-03 name below.
- The written output: `extract` writes a Visual Basic 6 project directory
  that VB6 can open: one `.vbp`, one `.frm` per form, one `.frx` per form
  that carries a resource blob, one `.bas` per standard module, and one
  `.cls` per class module, within the limit WRT-03 names below.
- The JSON report: `extract` writes one JSON report beside the project,
  naming what the run recovered and how sure it is of each fact, with the
  byte evidence for every item and every defect the run met.
- Hostile input handling: the tool does not panic on any input, refuses a
  damaged file by default and names the byte offset, recovers what it can
  under `--salvage`, and runs a fuzzer in the gate.
- Verification: a differential test compares each corpus program's
  recovered result against the original source it was built from, and a
  pinned recovery ratio per program fails the build on a fall or a rise.

### What stays open

- **FRM-01**: the control tree of one corpus form, `Main` in
  `Map Editor.exe`, does not resolve. The scope separator grammar for that
  one form is not known, and the tool refuses rather than print a tree it
  cannot prove.
- **FRM-02**: the type and name count that follows from FRM-01 stays open
  for the same cause. The tool recovers the type and name of 686 of 686
  controls, over the forms whose tree it builds.
- **FRM-03**: the property values of most controls are not yet decoded.
  `tests/ratios.toml` pins 807 property records across the corpus, and 136
  written property lines that reach the `.frm` files. The gate asserts both
  numbers on every run. Most of the remaining records report present and not
  decoded, and the report names the opcode it holds no decoder for.
- **WRT-03**: the written `.frm` follows the byte level layout the IDE
  produces, proved by targeted unit test only. The corpus wide structural
  check trims each line by design and cannot confirm the exact indentation.

The README's two limit tables, one from the structure survey and one from
the file format survey, name the full list of gaps still open at this
release and the default the tool chose for each one.

### What this release does not do

- DeForm6 does not recover statements. The code inside a procedure does not
  come back.
- The P-code branch was never run. `ProjectInfo.lpNativeCode` decides
  whether a program is native or P-code, and every program in the test
  corpus is native, so this release has never proved the P-code branch
  against a real program.
- Full recompilation was not tested. It needs the Visual Basic 6 IDE on
  Windows, and this release had neither.
