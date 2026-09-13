---
phase: "5"
slug: "hostility"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-13"
---

# Phase 5: Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`). No external framework. The fuzz crate is the one exception. It builds only on nightly with `cargo fuzz`, and it stays outside `cargo test` entirely. |
| **Config file** | none, the gate lives in `AGENTS.md` |
| **Quick run command** | The narrowest slice for the module under work, for example `cargo test -p deform6 --lib vb::gui` or `cargo test -p deform6 --test regressions` |
| **Full suite command** | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` |
| **Estimated runtime** | ~90 seconds. This is the figure Phase 4 recorded for the same three commands. Phase 5 adds tests, so measure it again at the phase gate and correct this row. |

The three gate commands do not change in this phase. `[workspace] exclude =
["crates/deform6/fuzz"]` keeps the fuzz crate out of all three. The root
`Cargo.toml` already holds that line and already sets `panic = "abort"` in
`[profile.release]`.

---

## Sampling Rate

- **After every task commit:** Run the narrowest slice for the module just retrofitted or audited, for example `cargo test -p deform6 --lib vb::gui`.
- **After every plan wave:** Run `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`. `AGENTS.md` permits no smaller selection at this point.
- **Before `/gsd-verify-work`:** Full suite green, plus the bounded fuzz run of success criterion 1, plus the no-panic proof run of success criterion 5.
- **Max feedback latency:** 90 seconds for the gate. The bounded fuzz run adds 60 seconds and runs out of band.

---

## Per-Task Verification Map

Task IDs bind after the planner writes the plans. This table holds the
requirement level map that `05-RESEARCH.md` establishes. Replace each `05-NN-xx`
row with the real task IDs when the plans exist.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-xx | 05-01 | 1 | SAF-02, SAF-03 | V5 | A `Recoverable` defect refuses in `Strict` and names the byte offset and the expectation. The same file under `--salvage` produces output and records every assumption in the report. The defect list is the same in both modes. | unit + integration | `cargo test -p deform6 --lib journal` and `cargo test -p deform6-cli` | ❌ W0 | ⬜ pending |
| 05-02-xx | 05-02 | 1 | SAF-04 | V5 | Every count and every length field in the parse order is checked against the real file size before an allocation is sized from it. A file that declares `wFormCount = 0xFFFF` in a 4 KB image is refused with `ImplausibleCount` and allocates nothing. | unit | `cargo test -p deform6 --lib vb::gui` | ❌ W0 | ⬜ pending |
| 05-03-xx | 05-03 | 1 | SAF-01, SAF-05 | V5 | The fuzz target calls both `Strict` and `Salvage`. The fuzz crate builds on nightly only and breaks neither `cargo clippy --all-targets` nor `cargo test --workspace`. | fuzz | `cargo +nightly fuzz build --fuzz-dir crates/deform6/fuzz` | ❌ W0 | ⬜ pending |
| 05-07-xx | 05-07 | 1 | SAF-05 | V6, V12 | The manifest pins a SHA-256 for each fetched program. A hash mismatch fails the run loudly. A fetch that fails fails loudly. `corpus/fetched/` stays out of the repository. | unit + integration | `cargo test -p xtask` | ❌ W0 | ⬜ pending |
| 05-04-xx | 05-04 | 2 | SAF-05 | V5 | The bounded fuzz job runs on every pull request. The longer cron job runs on a schedule. Both seed from `corpus/`. | CI | `cargo +nightly fuzz run --fuzz-dir crates/deform6/fuzz parse -- -max_total_time=60 -rss_limit_mb=2048` | ❌ W0 | ⬜ pending |
| 05-05-xx | 05-05 | 2 | SAF-01, SAF-05 | V5 | `cargo test --workspace` replays every file in `tests/regressions/` through both modes on stable and names the file when one panics. An empty directory makes the test fail. | integration | `cargo test -p deform6 --test regressions` | ❌ W0 | ⬜ pending |
| 05-06-xx | 05-06 | 2 | SAF-01 | V5 | A fixed seed mutation of the corpus files runs in the normal gate on stable and needs no fuzzing dependency. | integration | `cargo test -p deform6 --test fuzz_smoke` | ❌ W0 | ⬜ pending |
| 05-08-xx | 05-08 | 3 | SAF-01 | V5 | The vendored corpus, the fetched set and every regression input run through both modes with `panic = "abort"`, and no process aborts. The run prints the number of inputs it read, and that number equals the number of files that exist. | integration | `cargo test -p deform6 --test no_panic_proof` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

The command column names the shape the test takes. The planner sets the final
test name. `05-RESEARCH.md` records the test that success criterion 4 names
directly: `vb::gui::gui_table_refuses_an_implausible_form_count`.

---

## Wave 0 Requirements

Every test file and every CI job in this phase is new. The `Journal`,
`Mode::{Strict, Salvage}` and `DefectKind::ImplausibleCount` machinery they wire
into already exists.

- [ ] `crates/deform6/fuzz/`. Does not exist yet. `cargo +nightly fuzz init --fuzzing-workspace true --fuzz-dir crates/deform6/fuzz` creates it. Do not hand write its `Cargo.toml`. Plan 05-03 does this.
- [ ] `crates/deform6/tests/regressions.rs` and `crates/deform6/tests/regressions/`. Do not exist yet. The directory needs at least one seed file when it is created, because the count assertion refuses an empty loop. Plan 05-05 does this.
- [ ] `corpus/manifest.toml`. Does not exist yet. Plan 05-07 creates it.
- [ ] `.github/workflows/fuzz.yml`. Does not exist yet. `.github/workflows/gate.yml` is the only workflow file today. Plan 05-04 creates it.
- [ ] `crates/xtask/src/fetch_corpus.rs` and the `xtask` subcommand that wraps every `cargo fuzz` call. Do not exist yet. Plans 05-04 and 05-07 create them.
- [ ] `cargo install cargo-fuzz` and `rustup toolchain install nightly`. Neither is confirmed present on this machine or in the CI runner image. `rust-toolchain.toml` pins stable 1.97.1. Confirm both before plan 05-03 starts.
- [ ] `sha2` and `ureq` in `crates/xtask/Cargo.toml`. Not yet added. `05-RESEARCH.md` audits both `OK` in its Package Legitimacy Audit, so no legitimacy checkpoint is required.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| A crash artifact is read before it enters `tests/regressions/` | SAF-05 | A human decides whether an input a person did not author may enter version control. A test cannot make that decision. | Read the artifact `cargo fuzz` wrote to `crates/deform6/fuzz/artifacts/parse/`. Confirm it holds fuzzer generated bytes only. Then copy it into `crates/deform6/tests/regressions/` and commit it with the fix. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
