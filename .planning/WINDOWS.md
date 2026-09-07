---
schema_version: 1
open_count: 3
waived_count: 0
fixed_count: 0
total_count: 3
last_updated: 2026-09-07T15:53:11.332Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | unrun-verify | crates/deform6/src/read/pe.rs |  | The section overlap rule is untested by construction: no corpus file has overlapping sections | open |  | 2026-09-07T15:06:30.501Z |  |
| 2 | 01 | unrun-verify | crates/deform6/src/vb/project.rs |  | No corpus program is P-code, so CompileMode::PCode is exercised only by zeroing lpNativeCode in a copy of a native program | open |  | 2026-09-07T15:53:05.293Z |  |
| 3 | 01 | unmet-truth | crates/deform6/src/vb/mod.rs |  | inspect drops the defects it collects: Report derives PartialEq and Defect does not, so the count mismatch and the section overlap reach no caller until the Phase 4 report | open |  | 2026-09-07T15:53:11.332Z |  |

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
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-07T15:53:11.332Z",
    "resolved_at": null
  }
]
````
