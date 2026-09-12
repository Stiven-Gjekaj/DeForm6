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

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`). No external framework. |
| **Config file** | none — the gate lives in `AGENTS.md` |
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

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 4-01-xx | 04-01 | 1 | WRT-06 | T-4-01 | A recovered name is clamped and made a legal VB6 identifier at the model layer, before any file path is built from it | unit | `cargo test -p deform6 --lib write::model` | ❌ W0 | ⬜ pending |
| 4-06-xx | 04-06 | 1 | RPT-01..RPT-06 | — | N/A | unit | `cargo test -p deform6 --lib report` | ❌ W0 | ⬜ pending |
| 4-02-xx | 04-02 | 2 | WRT-03 | — | N/A | unit | `cargo test -p deform6 --lib write::values` | ❌ W0 | ⬜ pending |
| 4-03-xx | 04-03 | 2 | WRT-02 | — | N/A | unit | `cargo test -p deform6 --lib write::vbp` | ❌ W0 | ⬜ pending |
| 4-05-xx | 04-05 | 2 | WRT-05, WRT-07 | — | N/A | unit | `cargo test -p deform6 --lib write::code` | ❌ W0 | ⬜ pending |
| 4-04-xx | 04-04 | 3 | WRT-03, WRT-04 | — | N/A | unit | `cargo test -p deform6 --lib write::frm` | ❌ W0 | ⬜ pending |
| 4-07-xx | 04-07 | 3 | RPT-03 | — | N/A | unit | `cargo test -p deform6 --lib write::comment` | ❌ W0 | ⬜ pending |
| 4-08-xx | 04-08 | 4 | WRT-01 | T-4-02 | `-o <dir>` resolves to an absolute canonical path; every written file is a child of it; the run refuses rather than writing partly outside | integration | `cargo test -p deform6-cli` | ❌ W0 | ⬜ pending |
| 4-09-xx | 04-09 | 5 | WRT-01, WRT-03, WRT-04, WRT-06 | — | N/A | integration | `cargo test -p deform6 --test extract_structural` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Every test file for this phase is new. Phases 1 to 3 built the reading side only.

- [ ] `crates/deform6/src/write/mod.rs`, `model.rs`, `values.rs`, `vbp.rs`, `frm.rs`, `code.rs` — none exist yet
- [ ] `crates/deform6/src/report.rs` — does not exist yet
- [ ] `crates/deform6/tests/extract_structural.rs` — does not exist yet. It reuses `crates/deform6/tests/support/frm.rs`, which already exists, rather than a second independent reader.
- [ ] `serde_json` in the workspace `Cargo.toml` and in `crates/deform6/Cargo.toml` — not yet added
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
