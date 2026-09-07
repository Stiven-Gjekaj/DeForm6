# DeForm6

## What This Is

DeForm6 is an open-source decompiler for Visual Basic 6 executables, written in
Rust. It reads a compiled VB6 program and writes back a Visual Basic project
that VB6 can open and build: the project file, the forms with their control
trees and property values, the form resources, and the module and class
skeletons with their real procedure names and signatures.

It is a recovery tool. The person who uses it has lost the source of a program
that a business still runs, and needs it back.

## Core Value

A person who has only a compiled VB6 executable gets back a Visual Basic project
that opens in the VB6 IDE, with the forms and the names intact, and a report
that says exactly how much of it is proved and how much is inferred.

## Requirements

### Validated

(None yet. Ship to validate.)

### Active

- [ ] The tool identifies a VB6 executable and refuses everything else with a
      clear sentence.
- [ ] The tool reads the object graph: forms, modules, classes, public
      procedure names, and procedure signatures with argument names and types.
- [ ] The tool reads the form data: the control tree, the control names and
      types, the property values, and the resource blobs.
- [ ] The tool writes a project directory that VB6 can open.
- [ ] The tool writes a machine-readable report that grades each recovered item
      by confidence and names the evidence for it.
- [ ] The tool marks uncertain regions with comments in the code files.
- [ ] The tool refuses a damaged file by default, and recovers what it can
      under an explicit flag.
- [ ] The tool does not panic on any input.

### Out of Scope

- P-code statement recovery. It is the next milestone, not this one. The
  opcode table must be built first, and that is a project of its own.
- Native code statement recovery. No public tool recovers compilable Basic from
  native VB6, and this one will not claim to either.
- VB5 executables. The header signature is shared, so a VB5 file is detected
  and refused by name, not parsed.
- ActiveX DLL, OCX, and control projects. They add COM registration data and a
  different object model. A later milestone.
- Local variable names, private procedure names, comments, and source
  formatting. The compiler does not keep them. No tool can return them.
- A graphical interface. The library is the product and the command line is a
  thin shell over it.

## Context

The field is close to empty. A survey of every known VB6 decompiler found one
project that is genuinely open source under a clear license, and it is about 25
commits in with only a PE parser. Everything else is commercial and closed
(VB Decompiler, VBReFormer), freeware and closed (vbdec, P32Dasm, VBDE), or
source-available with no license file at all (Semi VB Decompiler). There is no
prior art to build on and no open competitor.

The structure layer is well documented and stable. Alex Ionescu's "Visual Basic
Image Internal Structure Format" is the canonical reference for the VB header,
the project info, the object table, and the form data. A Gen Digital writeup
documents recovery of full function prototypes, with argument names, types, and
ByRef and Optional modifiers, from type descriptors that the compiler cannot
strip. Form and control recovery does not depend on the compilation mode, so it
works on native builds exactly as it works on P-code builds.

The P-code opcode table is the opposite. No complete public table exists. The
largest one has many entries marked unknown, and per-opcode argument widths are
where every existing tool is weakest. The real source is a specific MSVBVM60
build that shipped with debug symbols naming the handlers, giving about 822 real
handlers out of 1531 theoretical slots. That work belongs to a later milestone.

The test corpus is scavenged from GitHub. A sweep of 366 VB6 repositories found
120 that hold both source and a compiled executable, of which 52 carry a license
that permits redistribution. `tannerhelland/vb6-code` is the single most
valuable item: BSD-2, 32 projects published as source plus prebuilt executable,
all Standard EXE, all native, all verified as MSVBVM60 binaries. About 13 more
programs are in the public domain. Microsoft's own VB98 samples are excluded:
the VS6 licence grants no redistribution right, and they shipped without
binaries, so they carry no ground truth.

## Constraints

- **Tech stack**: Rust, a workspace with a `deform6` library crate and a
  `deform6-cli` binary crate. The library is the product. It matches the
  author's other Rust project and gives a single static binary for a tool that
  reads untrusted input.
- **Licence**: MIT. The field has no reusable open code, so the licence that
  maximises reuse is the right one.
- **Input**: VB6 Standard EXE only. VB5 is detected and refused.
- **Safety**: No panic on any input, ever. No unchecked indexing. No allocation
  sized by a length field in the file without a bound check against the real
  file size. Fuzzing is part of the gate.
- **Corpus**: Only programs whose licence permits redistribution go in the
  repository. Everything else is fetched at run time from a manifest of pinned
  hashes, and its bytes are never committed.
- **Evidence**: No binary, no source file, and no derived fixture from any
  third-party system the author does not own may enter this repository.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust, library plus command line crate | Single static binary for a tool that reads untrusted input. Bounds safety is the core requirement. The library lets other tools embed it. | Pending |
| Metadata recovery first, code recovery later | Forms, names, and prototypes are where most of the practical value is, they are well documented, and they work on native and P-code alike. Code recovery is a separate hard problem. | Pending |
| Recompilable output as the stated bar | It gives a machine-checkable pass or fail instead of an opinion. Readable output is the honest bar for native code recovery only. | Pending |
| Scavenged source and binary pairs as the corpus | Bare executables give inputs but no ground truth. Pairs give both, and about 45 exist under a redistributable licence. | Pending |
| Vendor the permissive corpus, fetch the rest at run time | A corpus that can disappear is not a gate. The unlicensed pile is still useful for robustness, but its bytes cannot be committed. | Pending |
| Strict by default, `--salvage` on request | A recovery tool's users often have a damaged file. A tool that silently guesses is useless as evidence. | Pending |
| Uncertainty markers in code regions only | VB6 accepts apostrophe comments in code but not inside a `Begin VB.Form` block or a project file. Structural doubt goes to the report instead. | Pending |
| Two commands, `inspect` and `extract` | Reporting on a file and writing a project to disk are different jobs. Conflating them makes both worse. | Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition:**
1. Requirements invalidated? Move to Out of Scope with the reason.
2. Requirements validated? Move to Validated with the phase reference.
3. New requirements emerged? Add to Active.
4. Decisions to log? Add to Key Decisions.
5. Is "What This Is" still accurate? Update it if it drifted.

**After each milestone:**
1. Review all sections.
2. Check the Core Value. Is it still the right priority?
3. Audit Out of Scope. Are the reasons still valid?
4. Update Context with the current state.

---
*Last updated: 2026-09-07 after initialization*
