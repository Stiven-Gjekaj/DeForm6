---
phase: "4"
slug: "it-writes-a-project"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-12"
---

# Phase 4: Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`). No external framework. |
| **Config file** | none, the gate lives in `AGENTS.md` |
| **Quick run command** | `cargo test -p deform6 --lib write` |
| **Full suite command** | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` |
| **Estimated runtime** | ~90 seconds for the full gate |

---

## Sampling Rate

- **After every task commit:** Run the narrowest slice for the module just written, for example `cargo test -p deform6 --lib write::frm`.
- **After every plan wave:** Run `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`. `AGENTS.md` permits no smaller selection at this point.
- **Before `/gsd-verify-work`:** Full suite green, plus the corpus-wide `extract` run over all 44 programs.
- **Max feedback latency:** 90 seconds.

---

## Per-Task Verification Map

Task IDs bind after the planner writes the plans. Each row names the requirement, the test type, and the command that proves it.

Bound after the planner wrote the nine plans on 2026-09-12. Plan 04-01 leads
with an end to end tracer, so it carries an integration row of its own, and
plan 04-06 moved from wave 1 to wave 2 because the tracer creates and owns
`crates/deform6/src/report.rs`.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 4-01-01 | 04-01 | 1 | WRT-01, WRT-06 | T-4-03, T-4-SC | The whole project is built in memory before any byte is written, and the library returns file names and never paths | integration | `cargo test -p deform6 --test extract_tracer` | ❌ W0 | ⬜ pending |
| 4-01-02 | 04-01 | 1 | WRT-06 | T-4-01 | A recovered name is clamped to 40 encoded bytes and made a legal VB6 identifier at the model layer, before any file path is built from it | unit | `cargo test -p deform6 --lib write::model` | ❌ W0 | ⬜ pending |
| 4-01-03 | 04-01 | 1 | WRT-06 | T-4-01 | `ProjectModel` carries no bare string name field, so no writer can reach a raw recovered string to build a path | unit | `cargo test -p deform6 --lib write::model` | ❌ W0 | ⬜ pending |
| 4-02-xx | 04-02 | 2 | WRT-03 | T-4-05, T-4-08 | An inner double quote is doubled, a value holding a line break never goes inline, and an undecoded property produces no line | unit | `cargo test -p deform6 --lib write::values` | ❌ W0 | ⬜ pending |
| 4-03-xx | 04-03 | 2 | WRT-02 | T-4-01, T-4-10 | Every component line name comes from a sanitized name, and the resource script key can never be written | unit | `cargo test -p deform6 --lib write::vbp` | ❌ W0 | ⬜ pending |
| 4-05-xx | 04-05 | 2 | WRT-05, WRT-07 | T-4-01, T-4-14 | Every file name comes from a sanitized name, and every procedure body holds zero lines | unit | `cargo test -p deform6 --lib write::code` | ❌ W0 | ⬜ pending |
| 4-06-xx | 04-06 | 2 | RPT-01, RPT-02, RPT-03, RPT-04, RPT-05 | T-4-15, T-4-16 | Escaping comes from the serialisation crate, and the defect array is never filtered | unit | `cargo test -p deform6 --lib report` | ❌ W0 | ⬜ pending |
| 4-04-xx | 04-04 | 3 | WRT-03, WRT-04 | T-4-03, T-4-12, T-4-13 | The blob byte range is checked against the real file length before any buffer is sized, every resource offset comes from the shipped cursor, and a refused control tree always produces an unrecoverable report item | unit | `cargo test -p deform6 --lib write::frm` | ❌ W0 | ⬜ pending |
| 4-07-xx | 04-07 | 3 | RPT-06 | T-4-18, T-4-19 | Line ending characters are stripped from recovered text before it reaches a comment, and no comment can reach a `Begin` block or the `.vbp` | unit | `cargo test -p deform6 --lib write::comment` | ❌ W0 | ⬜ pending |
| 4-08-xx | 04-08 | 4 | WRT-01 | T-4-01, T-4-02, T-4-21 | `-o <dir>` resolves to an absolute canonical path; every written file is verified to be a direct child of it; the run refuses rather than writing partly outside; a refusal leaves the directory untouched | integration | `cargo test -p deform6-cli` | ❌ W0 | ⬜ pending |
| 4-09-xx | 04-09 | 5 | WRT-01, WRT-03, WRT-04, WRT-06 | T-4-24, T-4-25, T-4-26 | The checker imports no writing module, it asserts the count it read against the count that exists, and no message claims the IDE opened the project | integration | `cargo test -p deform6 --test extract_structural` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Every test file for this phase is new. Phases 1 to 3 built the reading side only.

Plan 04-01 closes every one of these in wave 1. No plan in a later wave waits
on a file that does not exist.

- [ ] `crates/deform6/src/write/mod.rs`, `model.rs`, `values.rs`, `vbp.rs`, `frm.rs`, `code.rs`, `comment.rs`. None exist yet. Plan 04-01 creates all seven as a set in one commit, the pattern `crates/deform6/src/vb/mod.rs` states in its own doc comment.
- [ ] `crates/deform6/src/report.rs`. Does not exist yet. Plan 04-01 creates it and locks the shape of `ReportItem`, `Confidence` and `Evidence`; plan 04-06 completes the builder.
- [ ] `crates/deform6/tests/extract_tracer.rs`. Does not exist yet. Plan 04-01 creates it as the end to end proof over `corpus/vb6-code/Fire-effect/Fast_Flames.exe`.
- [ ] `crates/deform6/tests/extract_structural.rs`. Does not exist yet. Plan 04-09 creates it. It reuses `crates/deform6/tests/support/frm.rs` and `support/vbp.rs`, which already exist, rather than a second independent reader.
- [ ] `Command::Extract` in `crates/deform6-cli/src/main.rs`. Does not exist yet. Plan 04-01 adds it thin with the output flag; plan 04-08 adds the report flag, the force flag, the resolved directory and the containment check.
- [ ] `serde_json` in the workspace `Cargo.toml` and in `crates/deform6/Cargo.toml`. Not yet added. Plan 04-01 adds it. It is audited `OK` in `04-RESEARCH.md`'s Package Legitimacy Audit, so no legitimacy checkpoint is required.
- [ ] No test framework install is needed. `cargo test` is the workspace harness.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The VB6 IDE opens the written project and compiles it | WRT-01 | Needs VB6 on Windows. This CI has neither. | Open the written `.vbp` in the VB6 IDE on a Windows host. Read `out/`'s `.log` file if the load reports an error. The report and the README must both say that this step did not run. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
