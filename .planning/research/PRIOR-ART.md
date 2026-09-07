# Prior art: VB6 decompilation

Surveyed 2026-09-07.

## The field is empty of reusable open code

| Tool | Licence | Written in | Activity | Modes |
|---|---|---|---|---|
| VB Decompiler (DotFix) | Commercial, closed | Delphi (unconfirmed) | Active, v11+ | Both. Claims 85% P-code, 75% native |
| Semi VB Decompiler | Source available, **no LICENSE file** | VB6 | Most active free tool | P-code tokens, native structure only |
| vbdec (sandsprite) | Free, closed | Windows binary | Maintained | P-code disassembler and live debugger |
| VBReFormer | Commercial, closed | Undisclosed | Active | Both claimed |
| P32Dasm | Freeware, closed | Unknown | Dead, 2016 | P-code plus native listing |
| VBDE / WKTVBDE | Freeware, closed | Win32 | Dead, early 2000s | P-code runtime debugger |
| DaCodeChick/VBDecompiler | **LGPL-3.0** | Zig plus Qt6 | ~25 commits, early | Aspires to both, unusable today |
| openmsvbvm | Unstated | C++ | ~11 commits | Reimplements MSVBVM60 exports only |
| python-vb (Ballenthin) | Open | Python | Dead, ~2019 | Structure parsing |

**Conclusion.** Exactly one project is open source under a clear licence, and it
is barely started. Semi VB Decompiler is the best information source but its
code cannot be copied, because a repository with no licence reserves all rights.
There is no prior art to build on and no open competitor.

## The structure layer is well documented

- **Alex Ionescu, "Visual Basic Image Internal Structure Format"** is the
  canonical reference. It covers the `VB5!` signature (`0x21354256`), the
  VBHeader, ProjectInfo, ObjectTable, ObjectInfo, ComRegisterData,
  ExternalTable, and the GUI form data table.
  https://sandsprite.com/vb-reversing/files/Alex_Ionescu_vb_structures.pdf
- **Andrea Geddon, "Visual Basic Reversed: a decompiling approach"** is the
  second canonical text and is more decompilation oriented.
  https://sandsprite.com/vb-reversing/files/VISUAL%20BASIC%20REVERSED.pdf
- **Gen Digital, "Recovery of function prototypes in VB6 executables"** is
  modern and precise on `FuncTypDesc`, `PubVarDesc` and `EventDesc`. Its key
  claim: this type data lives in `.text` and **the compiler cannot strip it**,
  so argument names, types, and the ByRef, Array and Optional modifiers survive
  in every build.

The least uniform area is the procedure descriptor layer. It varies between
P-code and native builds and between VB5 and VB6.

## The P-code opcode table does not exist in public

No complete, authoritative, machine-readable table exists. Microsoft kept the
specification under NDA.

- The largest public table is at https://www.vb-decompiler.org/vb_pcode_table.htm
  It has roughly 800 to 900 named entries and many marked unknown. It is an HTML
  page, not a data file.
- "VB P-code Information" by Mr Silver explains the dispatch model: five prefix
  opcodes plus one standard set give 1536 theoretical slots, dispatched through
  `jmp [eax*4+ADDRESS]`.
- Gen Digital's "VB6 P-Code Disassembly" is the most current work. It counts
  about **822 unique handlers** against 1531 slots, identifies `_tblByteDisp` as
  the primary dispatch array, and uses an MSVBVM60 build that **ships debug
  symbols naming the handlers**. That build is the highest-leverage artifact for
  this project.

**Per-opcode argument byte widths are where every existing tool is weakest.**
Budget for building this table from the dispatch tables rather than copying a
public one.

## The honest ceiling for native code

Reliably recoverable from a native build: procedure and method names for public
members, full prototypes with argument names and types, API `Declare`
statements from the ExternalTable, the form and control layout, string
literals, and the object graph.

Best effort only: statement level reconstruction. The real output is annotated
x86 with the `__vba*` helpers resolved into VB idioms, not compilable Basic.

Not recoverable in any mode, because the compiler does not keep them: local
variable names, private procedure names, user defined type member names,
comments, and original formatting.

## Forms are recoverable regardless of compilation mode

Design time form data is serialised into the binary by the same mechanism for
native and P-code builds, so the control tree and property values come back
either way.

- The `.frm` holds the control tree and scalar properties. The `.frx` holds
  bulk data, appended one blob after another, referenced by byte offset.
- **Property encoding is inconsistent.** `Caption` and `Name` are stored as
  ASCII. `Tag` and `Connect` are Unicode. This will bite.
- Semi VB Decompiler resolves property names by binding to `VB6.OLB` over COM
  rather than hardcoding them. That technique needs VB6 installed, so it is not
  available here, but it explains why hardcoded property tables in other tools
  are incomplete.
- Third party OCX controls cannot be fully reconstructed. The CLSID and the
  serialised property blob come back, but interpreting the blob needs the OCX
  type library.

## Legal position

- **EU Software Directive 2009/24/EC Article 6** grants a statutory
  decompilation right for interoperability that contract cannot override.
- US law has no equivalent. DMCA 1201(f) is an anti-circumvention exemption, not
  a general permission, and EULA anti-reverse-engineering clauses have been
  upheld (Bowers v Baystate, Davidson v Internet Gateway).
- Distributing the tool itself has not been challenged. Commercial VB6
  decompilers have sold openly for about twenty years with no reported
  litigation. That is tolerance, not a holding.
- Hygiene the field observes: derive opcode semantics from behaviour and from
  the debug symbols Microsoft itself shipped, never from leaked source. Do not
  redistribute MSVBVM60.DLL or its symbol files. Ship the opcode table as
  derived data.

This is a summary of public sources, not legal advice.
