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

- Byte fidelity: `deform6::fidelity::walk::walk` writes eight structures
  back over the bytes they were read from. It grades each byte as the same,
  as different, or as not modelled. Across the 44 corpus programs it grades
  1251 records, and no byte that a reader models differs from the file.
- The array census: the same walk counts four arrays, which are the GUI
  table, the object array, the controls of each object and the event slots
  of each control. For each one it compares the count that the file
  declares with the number of entries that the reader returns. No array in
  the corpus comes up short.
- Corpus evidence: gap 2, the `OptionalObjectInfo` presence test, is
  closed. The dispute about `fControlType` and `wEventCount` at the start of
  `ControlInfo` is settled. Sections 16 and 17 of `docs/STRUCTURES.md` give
  the numbers.
- `sh scripts/gate.sh` runs the whole gate on a local machine.

### What changes for a caller

These changes break code that was written against 1.0.0. The next release
is therefore 2.0.0, not 1.1.0.

- New public fields: `VbHeader::file_offset`, `ProjectInfo::file_offset`,
  `ControlInfo::file_offset`, `ControlInfo::lpsz_name`,
  `Object::lpsz_object_name` and `GuiTableEntry::l_struct_size`. Code that
  builds one of these structures with a struct expression does not compile.
  Code that names every field of one in a pattern does not compile.
- These five structures and the new `OptionalObjectInfo` are now
  `#[non_exhaustive]`. Outside this crate, do not build them with a struct
  expression, and put `..` in each pattern that names their fields. Then a
  field that a later release adds does not break a caller again.
- Three defects in the JSON report now give the offset of the count that
  was clamped, not the offset of the table that the count bounds. A clamped
  `wFormCount` names the structure `VBHeader`, at `VBHeader + 0x44`, where
  1.0.0 named `GuiTable`, at the start of the GUI table. A clamped
  `wEventCount` gives `ControlInfo + 0x02`. A clamped `dwExternalCount`
  gives `ProjectInfo + 0x238`, and its `rva` is now `null`, because that
  byte was not reached through the address of the `Declare` table that
  1.0.0 gave.

### What stays open

- The fidelity walk grades eight structures. `docs/ROADMAP.md` names the
  work on the other structures that is left.
- The census does not count the `Declare` table or the type buffer. The
  clamps on those two arrays only bound a loop, so the byte diff cannot see
  them either.
- `vb::classify::agree` states the gap 2 cross-check, but no production
  code calls it. A file whose two markers disagree is not reported.
- The internal `damaged` helper still leaks one message for each refusal.
  The scheduled fuzz job is held to a measured peak resident set, not to a
  model of the leak.

### What this does not do

- It does not change the version in `Cargo.toml`, and it has no tag.

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
