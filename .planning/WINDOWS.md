---
schema_version: 1
open_count: 6
waived_count: 0
fixed_count: 3
total_count: 9
last_updated: 2026-09-10T22:25:56.340Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | unrun-verify | crates/deform6/src/read/pe.rs |  | The section overlap rule is untested by construction: no corpus file has overlapping sections | open |  | 2026-09-07T15:06:30.501Z |  |
| 2 | 01 | unrun-verify | crates/deform6/src/vb/project.rs |  | No corpus program is P-code, so CompileMode::PCode is exercised only by zeroing lpNativeCode in a copy of a native program | open |  | 2026-09-07T15:53:05.293Z |  |
| 3 | 01 | unmet-truth | crates/deform6/src/vb/mod.rs |  | inspect drops the defects it collects: Report derives PartialEq and Defect does not, so the count mismatch and the section overlap reach no caller until the Phase 4 report | fixed |  | 2026-09-07T15:53:11.332Z | 2026-09-08T10:55:45.000Z |
| 4 | 01 | unrun-verify | crates/deform6/src/vb/runtime.rs |  | runtime_dll cannot be proved to come from the file: imported_dlls upper-cases every name and classify accepts only case variants of the constant, so the matched name always equals it. A discard written as _matched compiles clean under the whole gate. Fix in phase 2 by returning the name verbatim and comparing case-insensitively, then a lower-case import fixture separates them. | open |  | 2026-09-07T18:10:34.445Z |  |
| 5 | 02 | unrun-verify | crates/deform6/src/vb/privateobj.rs |  | A non-null lpProcNamesArray entry that resolves but fails NUL-termination-within-64-bytes or the identifier-character test is handled but not exercised: the three vendored programs this module reads only exercise the address-does-not-resolve failure mode | fixed |  | 2026-09-07T22:59:56.143Z | 2026-09-08T11:19:57.959Z |
| 6 | 03 | stub | crates/xtask/src/opcode_table.rs |  | windows_walk::run: the COM walk over a real VB6.OLB type library is a documented stub (cfg(windows), never compiled on this host); needs a human at a Windows host with a lawful VB6 install to implement | open |  | 2026-09-10T10:22:03.247Z |  |
| 7 | 03 | deviation | crates/deform6/src/vb/controltree.rs |  | The scope-byte grammar for closing out of a menu control nested two levels deep, back to a sibling menu at the form's own top level, is unresolved. A defensive trailing-span bound (MAX_UNEXPLAINED_TAIL) converts the silent wrong tree into an honest refusal for the affected form, but the real grammar rule (HexScroll.exe, PassGen.exe, UUID2.exe, and Map Editor.exe's own separate gap) is not yet found. Needs the same byte-level corpus research 03-04 did for the single-level menu case. | fixed |  | 2026-09-10T14:30:20.446Z | 2026-09-10T22:25:34.660Z |
| 8 | 03 | deviation | crates/deform6/src/vb/controltree.rs |  | Map Editor.exe's Main form fails for a reason distinct from the two-level-deep menu close finding 7 named (now fixed): the walk expects a scope separator (0xFF) at file offset 0x170e and finds 0x37 instead. Not a tail-bound refusal; the scope-byte read itself fails at a position finding 7's own fix does not reach. Measured but not explained this session; needs its own byte-level research. | open |  | 2026-09-10T22:25:47.439Z |  |
| 9 | 03 | deviation | crates/deform6/src/vb/controltree.rs |  | frmPassGen.frm's own menuHelp section (corpus/public-domain/PassGen/PassGen.exe) exposes a third scope-byte shape the two-level-deep-close rule (finding 7, fixed) does not settle: menuAbout, itself a sibling within an already-open menu, genuinely opens its own child. Its own trailing separator at file offset 0x21d0 is a bare 0xFF 0x02 with the parent stack top a menu and zero pops, byte for byte identical to the confirmed sibling case (frmUUID2, HexScroll.exe), but needs the opposite role. No byte in the header or property stream distinguishes the two. The walk still succeeds for frmPassGen (every control is present, the byte count tiles exactly), but menuAboutForm, menuSeparatorC and menuWebsite recover with menuHelp as their parent instead of menuAbout. Documented in crates/deform6/src/vb/controltree.rs's own read_scope_run doc comment; needs research beyond scope-byte measurement (a property-opcode-aware signal or a different source) to settle. | open |  | 2026-09-10T22:25:56.340Z |  |

````json
[
  {
    "id": 1,
    "kind": "unrun-verify",
    "phase": "01",
    "file": "crates/deform6/src/read/pe.rs",
    "line": null,
    "description": "The section overlap rule is untested by construction: no corpus file has overlapping sections",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-07T15:06:30.501Z",
    "resolved_at": null
  },
  {
    "id": 2,
    "kind": "unrun-verify",
    "phase": "01",
    "file": "crates/deform6/src/vb/project.rs",
    "line": null,
    "description": "No corpus program is P-code, so CompileMode::PCode is exercised only by zeroing lpNativeCode in a copy of a native program",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-07T15:53:05.293Z",
    "resolved_at": null
  },
  {
    "id": 3,
    "kind": "unmet-truth",
    "phase": "01",
    "file": "crates/deform6/src/vb/mod.rs",
    "line": null,
    "description": "inspect drops the defects it collects: Report derives PartialEq and Defect does not, so the count mismatch and the section overlap reach no caller until the Phase 4 report",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-09-07T15:53:11.332Z",
    "resolved_at": "2026-09-08T10:55:45.000Z"
  },
  {
    "id": 4,
    "kind": "unrun-verify",
    "phase": "01",
    "file": "crates/deform6/src/vb/runtime.rs",
    "line": null,
    "description": "runtime_dll cannot be proved to come from the file: imported_dlls upper-cases every name and classify accepts only case variants of the constant, so the matched name always equals it. A discard written as _matched compiles clean under the whole gate. Fix in phase 2 by returning the name verbatim and comparing case-insensitively, then a lower-case import fixture separates them.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-07T18:10:34.445Z",
    "resolved_at": null
  },
  {
    "id": 5,
    "kind": "unrun-verify",
    "phase": "02",
    "file": "crates/deform6/src/vb/privateobj.rs",
    "line": null,
    "description": "A non-null lpProcNamesArray entry that resolves but fails NUL-termination-within-64-bytes or the identifier-character test is handled but not exercised: the three vendored programs this module reads only exercise the address-does-not-resolve failure mode",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-09-07T22:59:56.143Z",
    "resolved_at": "2026-09-08T11:19:57.959Z"
  },
  {
    "id": 6,
    "kind": "stub",
    "phase": "03",
    "file": "crates/xtask/src/opcode_table.rs",
    "line": null,
    "description": "windows_walk::run: the COM walk over a real VB6.OLB type library is a documented stub (cfg(windows), never compiled on this host); needs a human at a Windows host with a lawful VB6 install to implement",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-10T10:22:03.247Z",
    "resolved_at": null
  },
  {
    "id": 7,
    "kind": "deviation",
    "phase": "03",
    "file": "crates/deform6/src/vb/controltree.rs",
    "line": null,
    "description": "The scope-byte grammar for closing out of a menu control nested two levels deep, back to a sibling menu at the form's own top level, is unresolved. A defensive trailing-span bound (MAX_UNEXPLAINED_TAIL) converts the silent wrong tree into an honest refusal for the affected form, but the real grammar rule (HexScroll.exe, PassGen.exe, UUID2.exe, and Map Editor.exe's own separate gap) is not yet found. Needs the same byte-level corpus research 03-04 did for the single-level menu case.",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-09-10T14:30:20.446Z",
    "resolved_at": "2026-09-10T22:25:34.660Z"
  },
  {
    "id": 8,
    "kind": "deviation",
    "phase": "03",
    "file": "crates/deform6/src/vb/controltree.rs",
    "line": null,
    "description": "Map Editor.exe's Main form fails for a reason distinct from the two-level-deep menu close finding 7 named (now fixed): the walk expects a scope separator (0xFF) at file offset 0x170e and finds 0x37 instead. Not a tail-bound refusal; the scope-byte read itself fails at a position finding 7's own fix does not reach. Measured but not explained this session; needs its own byte-level research.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-10T22:25:47.439Z",
    "resolved_at": null
  },
  {
    "id": 9,
    "kind": "deviation",
    "phase": "03",
    "file": "crates/deform6/src/vb/controltree.rs",
    "line": null,
    "description": "frmPassGen.frm's own menuHelp section (corpus/public-domain/PassGen/PassGen.exe) exposes a third scope-byte shape the two-level-deep-close rule (finding 7, fixed) does not settle: menuAbout, itself a sibling within an already-open menu, genuinely opens its own child. Its own trailing separator at file offset 0x21d0 is a bare 0xFF 0x02 with the parent stack top a menu and zero pops, byte for byte identical to the confirmed sibling case (frmUUID2, HexScroll.exe), but needs the opposite role. No byte in the header or property stream distinguishes the two. The walk still succeeds for frmPassGen (every control is present, the byte count tiles exactly), but menuAboutForm, menuSeparatorC and menuWebsite recover with menuHelp as their parent instead of menuAbout. Documented in crates/deform6/src/vb/controltree.rs's own read_scope_run doc comment; needs research beyond scope-byte measurement (a property-opcode-aware signal or a different source) to settle.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-10T22:25:56.340Z",
    "resolved_at": null
  }
]
````
