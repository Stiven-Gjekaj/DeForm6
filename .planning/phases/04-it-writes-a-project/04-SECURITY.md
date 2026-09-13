---
phase: "4"
slug: "it-writes-a-project"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-09-13"
---

# Phase 4 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

Phase 1 to Phase 3 read an untrusted executable. Phase 4 is the first phase
that writes. Two new boundaries come with that change.

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Recovered name to file path | A recovered object, control, class or module name becomes a file name on disk under `-o <dir>`. Phase 1 to Phase 3 only displayed such a name. | File-derived identifier strings, fully untrusted |
| Recovered text to written file | A recovered property value, procedure signature or comment becomes a line in a file that the Visual Basic 6 IDE parses. | File-derived text, fully untrusted |
| Output directory | `-o <dir>` and `--force` name a location on the user's disk that the run writes into. | File paths supplied by the user, plus every derived file name |
| Executable bytes to blob range | A declared blob offset and length drive a read out of the executable and a write into a `.frx`. | Untrusted integers |

---

## Threat Register

The register comes from the nine plans, each of which carries its own
`<threat_model>` block. `T-4-04` and `T-4-06` are not used. `T-4-SC` is a
supply-chain check the auditor consolidated across all nine plans.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-4-01 | Tampering / Elevation of Privilege | every writer | high | mitigate | `SafeName` in `write/model.rs:180-290`. No model type carries a bare `String` name field, so no writer can build a path from a raw `Report` field. Consumed through `.as_str()` and `.file_name()` only. | closed |
| T-4-02 | Tampering | `deform6-cli` output path | high | mitigate | `resolve_output_dir` canonicalizes (`main.rs:329-341`); `plan_writes` compares `PathBuf` parents, never a string prefix (`main.rs:405-429`). | closed |
| T-4-03 | Denial of Service | `write/frm.rs` blob read | high | mitigate | `append_blob` runs five checked-arithmetic guards before `blob_cursor.take` (`write/frm.rs:63-101`). Fixed in `1e20c6e`. | closed |
| T-4-05 | Tampering | inline string and component lines | medium | mitigate | `escape_inline_string` and `inline_decision` (`write/values.rs:145-198`); `write_quoted_setting` and `object_line` both guard for `\r` and `\n` (`write/vbp.rs:288-308,235-256`, fixed in `7872ac6`); `format_argument` reuses `is_plausible_identifier` (`write/code.rs:229-234`, fixed in `bd01d86`). | closed |
| T-4-07 | Information Disclosure | the JSON report | low | accept | The report is the user's own file and only the user reads it. Rationale in the 04-01 and 04-06 threat models. | closed |
| T-4-08 | Tampering | undecoded property | medium | mitigate | An `Undecoded` value returns `FormattedValue::Omit`, so no placeholder line enters a `Begin` block (`write/values.rs:391-412`). | closed |
| T-4-09 | Repudiation | omitted property | low | mitigate | Every omission returns a `ReportItem` that names the opcode, the control type and the byte offset (`write/values.rs:376-412`). | closed |
| T-4-10 | Denial of Service | the `.vbp` | medium | mitigate | `SETTING_ORDER` holds 34 keys and no `ResFile32` (`write/vbp.rs:30-65`). | closed |
| T-4-11 | Repudiation | inferred `.vbp` and code values | medium | mitigate | Compiler flags carry `Confidence::Inferred` (`write/vbp.rs:363-374`); procedure-level items reach `uncertainty_comments` (`write/code.rs:349-383,486-503`, fixed in `c2c727f`). | closed |
| T-4-12 | Tampering | the `.frx` offset | high | mitigate | `frx_offset` comes only from `blob_cursor.take` (`write/frm.rs:97`). No arithmetic in the form writer. | closed |
| T-4-13 | Repudiation | a refused control tree | high | mitigate | `tree_refused_item` runs whenever `form.tree_refused` (`write/frm.rs:203-231,709-719`), so an empty form and a refused form are different answers. | closed |
| T-4-14 | Tampering | procedure body | high | mitigate | `format_signature` and `format_procedure_entry` build a declaration and its closing only (`write/code.rs:169-193,344-383`). | closed |
| T-4-15 | Tampering | JSON escaping | medium | mitigate | `serde_json::to_string_pretty` is the one JSON-building call in the crate (`report.rs:51-52`). | closed |
| T-4-16 | Repudiation | the defect array | high | mitigate | `build()` clones the whole defect list with no filter (`report.rs:412-416`). | closed |
| T-4-17 | Repudiation | report determinism | medium | mitigate | No `HashMap` appears anywhere in `write/` or the CLI. | closed |
| T-4-18 | Tampering | uncertainty comment | high | mitigate | `strip_line_endings` removes every line-ending character before recovered text reaches a comment (`write/comment.rs:86-95`). | closed |
| T-4-19 | Denial of Service | comment placement | high | mitigate | `write_code_region` runs only after the `Begin...End` block and the five `Attribute` lines (`write/frm.rs:735-752`), proved corpus wide (`write/code.rs:1159-1358`). | closed |
| T-4-20 | Repudiation | comment coverage test | medium | mitigate | The sweep asserts a positive count and names a program, not only an absence (`write/code.rs:1349-1357`). | closed |
| T-4-21 | Denial of Service | partial write | high | mitigate | The whole project is built in memory before any path is touched (`main.rs:301-310`), then planned and written (`main.rs:405-429,491-547`). A refusal leaves the directory untouched. | closed |
| T-4-22 | Tampering | `--force` | medium | mitigate | `plan_writes` and `refuse_symlink_targets` run unconditionally, not only under `--force` (`main.rs:346-367,499-524`). Symlink refusal added in `6c384d7`. | closed |
| T-4-23 | Repudiation | `--report` path | low | accept | `resolve_report_path` states its rationale in its doc comment and prints the resolved path (`main.rs:462-489`). | closed |
| T-4-24 | Repudiation | the structural check | high | mitigate | `tests/extract_structural.rs:23-46` imports only `support::frm`, `support::vbp` and the `write::project` entry point. No writer-internal module. | closed |
| T-4-25 | Repudiation | corpus coverage | high | mitigate | The check asserts `programs.len() == 44`, `programs_checked == programs.len()` and `text_files_checked > 0` (`tests/extract_structural.rs:500-506,644-654`). | closed |
| T-4-26 | Repudiation | the recompilation statement | high | mitigate | The fixed sentence is asserted per program and verbatim (`tests/extract_structural.rs:66-69,632-641,706-720`). | closed |
| T-4-27 | Repudiation | check vacuity | high | mitigate | The `deliberate_breakages` module holds one deliberately broken case per named assertion, 8 tests (`tests/extract_structural.rs:726-938`). | closed |
| T-4-SC | Tampering (supply chain) | `serde_json` | high | mitigate | Package legitimacy audit in `04-RESEARCH.md` gives verdict `OK`, cross-checked against crates.io. No other package enters the workspace in this phase. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-4-01 | T-4-07 | The JSON report holds recovered strings and byte offsets from the user's own executable. The report is written beside the project the user asked for, and only the user reads it. The run discloses nothing the user does not already hold. | Planner, in the 04-01 and 04-06 threat models. Not reviewed by a human. | 2026-09-13 |
| R-4-02 | T-4-23 | `--report` can name a path outside the output directory. This is the user's own explicit instruction on their own command line, not a recovered value. The run prints the resolved path. | Planner, in the 04-08 threat model. Not reviewed by a human. | 2026-09-13 |

*Accepted risks do not resurface in future audit runs.*

Both entries carry an `accept` disposition the planner chose while it wrote the
threat model. No human has reviewed either one. Read them before the milestone
closes, and change the `Accepted By` column when you do.

---

## Residual Items

One code-review finding stays open on purpose. It is not a threat.

- **IN-01** (`write/vbp.rs:238,248,259`): `component.name`, a raw recovered
  `String`, is concatenated into a JSON report path. The auditor traced it and
  found it never reaches `std::fs::write`, `plan_writes` or any file name. It
  becomes a string value inside a `serde_json` document, which escapes it
  whatever it holds. It does not defeat T-4-01, because T-4-01's boundary is a
  file path and this value never becomes one. It stays an Info-level code
  quality item.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-13 | 26 | 26 | 0 | gsd-security-auditor, ASVS L1 |

The audit ran after the phase code review closed 3 Critical and 3 Warning
findings. Six of those fixes sit inside this register: `1e20c6e` (T-4-03),
`bd01d86` and `7872ac6` (T-4-05), `6c384d7` (T-4-22), `b020ecc` (T-4-01) and
`c2c727f` (T-4-11). The auditor verified the fixed state, not the state the
plans describe.

The auditor reverted the `1e20c6e` ordering fix and re-ran its regression test.
The test failed before the fix and passed after it. The orchestrator ran the
same cycle independently and got the same result.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-13
