# Changelog

This file has no format precedent in this repository, so this note states
the one it uses. Every release gets one level two heading in the form
`## [X.Y.Z] - YYYY-MM-DD`. The newest release comes first. Each section
states what the release delivers, what stays open, and what the release
does not do.

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
- **FRM-03**: the property values of most controls are not yet decoded. The
  tool recovers 122 named property values against 683 records that report
  present and not decoded, over 805 property records.
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
