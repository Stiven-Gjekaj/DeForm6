# Requirements: DeForm6

**Defined:** 2026-09-07
**Core Value:** A person who has only a compiled VB6 executable gets back a
Visual Basic project that opens in the VB6 IDE, with the forms and the names
intact, and a report that says how much of it is proved and how much is
inferred.

## v1 Requirements

Version 1 recovers metadata. It does not recover statements.

### Identification

- [x] **DET-01**: The tool reads a PE file and resolves its entry point to a
      file offset through the section table.
- [x] **DET-02**: The tool follows the entry point stub to the VB header and
      confirms the `VB5!` signature.
- [x] **DET-03**: The tool tells VB6 from VB5 by the imported runtime DLL name,
      not by the signature, because both runtimes write `VB5!`.
- [x] **DET-04**: The tool refuses a VB5 file by name, and refuses a file that
      is not Visual Basic at all, each with one clear sentence.
- [x] **DET-05**: The tool reports whether the file is native or P-code from
      `ProjectInfo.lpNativeCode`.
- [x] **DET-06**: `deform6 inspect <exe>` prints the header, the project name,
      and the compilation mode, and writes nothing to disk.

### Object graph

- [x] **OBJ-01**: The tool walks the object table and recovers the name of
      every compiled object.
- [x] **OBJ-02**: The tool tells a form, a module, and a class apart.
- [x] **OBJ-03**: The tool recovers public procedure names for every object.
- [x] **OBJ-04**: The tool recovers procedure signatures with argument names
      and types, and the ByRef, Array, Optional and ParamArray modifiers.
- [x] **OBJ-05**: The tool recovers the `Declare` statements for external API
      calls from the external table.
- [x] **OBJ-06**: The tool reports a private procedure as private rather than
      inventing a name for it.

### Forms

- [ ] **FRM-01**: The tool recovers the control tree of every form, with the
      parent of each control.
- [ ] **FRM-02**: The tool recovers the type and the name of every control.
- [ ] **FRM-03**: The tool recovers the property values of every control, and
      of the form itself.
      This requirement stays open on purpose. Phase 3 recovers 122 named
      property values against 683 records that report present and not
      decoded, over 805 property records and five distinct property names.
      Decision D-01 predicts this result: most pairs of a control type and a
      property name stay open until a lawful property table exists. The
      remaining table is a data build job that needs a human at a working
      VB6 install, by the method of `STRUCTURES.md` section 13. That job is
      outside phase 3. Phase 6 records the limit in the README.
- [x] **FRM-04**: The tool recovers the component identifier that the
      executable declares for a third party OCX control, and says plainly
      that this identifier is not confirmed against the control's registered
      CLSID. The tool also says plainly that it cannot interpret that
      control's property blob without the control's own type library.
      The wording names the declared identifier, not the registered CLSID,
      because eighteen searches over three corpus programs and six encodings
      never find the registered identifier in the executable. `STRUCTURES.md`
      section 7.3.1 holds that record. This narrowing repeats the method that
      decision D-02 used for FRM-06.
- [ ] **FRM-05**: The tool recovers the resource blobs and writes an `.frx`
      whose offsets the generated `.frm` agrees with.
- [x] **FRM-06**: The tool recovers the event structure of each control:
      which event slots the compiled form binds, the index of each slot, and
      the native address of each bound handler. The tool does not recover
      the name of an event. The compiled file holds no such name. A name
      needs a table built from a Microsoft type library, and decision D-02
      in `.planning/phases/03-forms/03-CONTEXT.md` bars that table from this
      repository.

### Written output

- [ ] **WRT-01**: `deform6 extract <exe> -o <dir>` writes a project directory.
- [ ] **WRT-02**: The written `.vbp` lists every object and every control
      dependency the binary declares.
- [ ] **WRT-03**: The written `.frm` follows the byte level layout the IDE
      produces: three space indent per level, name padded to sixteen columns,
      an equals sign followed by exactly three spaces, one trailing space on
      `Begin` and `BeginProperty`, none on `End` and `EndProperty`.
- [ ] **WRT-04**: Properties are written in alphabetical order within a block,
      and menus are written last, because the IDE refuses a file otherwise.
- [ ] **WRT-05**: The tool writes `.bas` and `.cls` files with the attribute
      preamble the IDE requires.
- [ ] **WRT-06**: The tool writes CRLF line endings and the code page the IDE
      expects.
- [ ] **WRT-07**: A procedure whose body cannot be recovered is written as a
      valid empty procedure with the correct signature, so the project still
      builds.

### The report

- [ ] **RPT-01**: The tool writes one JSON report beside the project.
- [ ] **RPT-02**: The report is a flat array of items, each keyed by a path
      such as `/forms/frmMain/controls/cmdOk`.
- [ ] **RPT-03**: Each item carries a confidence of one of three named values,
      never a number that implies a precision the tool does not have.
- [ ] **RPT-04**: Each item names the evidence for it: the byte offset it came
      from and the structure it was read out of.
- [ ] **RPT-05**: The report records every defect the run met, whether or not
      the run continued past it.
- [ ] **RPT-06**: An uncertain region of recovered code carries an apostrophe
      comment, in code regions only, never inside a `Begin` block or the
      `.vbp`, because the IDE refuses those.

### Hostile input

- [ ] **SAF-01**: The tool does not panic on any input.
- [ ] **SAF-02**: The tool refuses a damaged file by default, and names the
      byte offset and what it expected to find there.
- [ ] **SAF-03**: `--salvage` recovers what it can from a damaged file and
      marks in the report everything it had to assume.
- [ ] **SAF-04**: The tool never sizes an allocation from a length field in the
      file without checking that length against the real size of the file.
- [ ] **SAF-05**: A fuzzer runs in the gate, and every crash it finds becomes a
      committed regression test that replays on stable Rust.

### Verification

- [x] **VER-01**: A differential test decompiles each corpus program and
      compares the result against the original source that the executable was
      built from.
- [x] **VER-02**: The expectation for each program comes from the file list the
      `.vbp` declares, never from a directory glob, because a project directory
      holds source that was never compiled.
- [x] **VER-03**: Where several `.vbp` files sit in one directory, the harness
      selects the one whose `ExeName32` names the executable under test.
- [x] **VER-04**: What the compiler does not keep is excluded by a written
      rule, not by a per-program allowance.
- [x] **VER-05**: A recovery ratio per program is pinned in the repository. A
      fall fails the build and names what went missing. A rise fails the build
      and prints the new value to record.
- [ ] **VER-06**: The known defect in `frmHMM.frx` is excluded by name, with
      the reason recorded, and never passes silently.

## v2 Requirements

Deferred. Tracked, not in this roadmap.

### P-code

- **PCD-01**: An opcode table built from the MSVBVM60 dispatch tables, with the
  argument byte width of every opcode.
- **PCD-02**: A P-code disassembler.
- **PCD-03**: A lifter from P-code to Visual Basic statements.
- **PCD-04**: A P-code corpus, since every program vendored today is native.

### Native code

- **NAT-01**: An x86 disassembler and a control flow graph.
- **NAT-02**: Recognition of the `__vba*` runtime helpers as Visual Basic
  idioms.
- **NAT-03**: A readable annotated listing. Not compilable Basic.

### Other project types

- **PRJ-01**: ActiveX DLL, OCX, and control projects.
- **PRJ-02**: VB5 executables.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Local variable names | The compiler does not keep them. No tool can return them. |
| Private procedure names | The name array holds a null for a private procedure. |
| Comments and source formatting | Not present in a compiled file. |
| Compilable Basic from native code | No public tool achieves it. Claiming it would be dishonest. |
| A graphical interface | The library is the product. The command line is a thin shell. |
| Copying code from another decompiler | The most useful one has no licence, so all rights in it are reserved. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DET-01 to DET-06 | Phase 1 | Complete |
| OBJ-01 to OBJ-06 | Phase 2 | Complete |
| VER-01 to VER-05 | Phase 2 | Complete |
| FRM-01 to FRM-06 | Phase 3 | Gaps Found |
| VER-06 | Phase 3 | Gaps Found |
| WRT-01 to WRT-07 | Phase 4 | Pending |
| RPT-01 to RPT-06 | Phase 4 | Pending |
| SAF-01 to SAF-05 | Phase 5 | Pending |

Phase 6 owns no new requirement. It measures the whole set end to end and turns
the result into the released documentation.

VER-05 pins the recovery ratio. Phase 2 introduces the file and the two failure
messages. Phase 3 and Phase 4 each raise the pinned numbers.

SAF-01 and SAF-04 are proved in Phase 5. The mechanism that makes them possible
is built in Phase 1: the lint wall, `#![forbid(unsafe_code)]`, and the `Region`
type with no infallible accessor.

**Coverage:**

- v1 requirements: 42 total in 7 categories
- Mapped to phases: 42
- Unmapped: 0

An earlier count in this file said 36. That number left out the Verification
category, VER-01 to VER-06. The correct total is 42.

---
*Requirements defined: 2026-09-07*
