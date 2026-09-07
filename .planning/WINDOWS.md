---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 0
total_count: 1
last_updated: 2026-09-07T15:06:30.501Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | unrun-verify | crates/deform6/src/read/pe.rs |  | The section overlap rule is untested by construction: no corpus file has overlapping sections | open |  | 2026-09-07T15:06:30.501Z |  |

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
  }
]
````
