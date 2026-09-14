---
phase: "6"
slug: "version-1-0"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-14"
---

# Phase 6 Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` harness via `cargo test`. No external test framework. |
| **Config file** | None. Tests are plain `#[test]` functions under `crates/deform6/tests/*.rs` and `crates/deform6-cli/tests/cli.rs`. |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` |
| **Estimated runtime** | ~60 seconds for the full three-command gate (974 tests measured in the release profile during phase research) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace`
- **After every plan wave:** Run the full three-command gate, plus `cargo doc --no-deps --workspace` once the zero-warning task lands
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

Phase 6 is a release phase. Its phase gate measures all five success criteria
directly, not by sample. Every one of the 44 corpus programs, every doc-comment
warning, and the whole claim surface is checked, not a subset.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| {N}-01-01 | 01 | 1 | REQ-{XX} | T-{N}-01 / none | {expected secure behavior or "N/A"} | unit | `{command}` | ✅ / ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*This table is seeded empty by plan-phase and filled by `/gsd-validate-phase`
once the PLAN.md task IDs exist.*

---

## Criterion to Check Map

Phase 6 owns no new requirement ID. It re-verifies DET, OBJ, FRM, WRT, RPT, SAF
and VER end to end. The table below maps the five success criteria to the checks
that prove them. It is the input the per-task map above is built from.

| Criterion | Check | Automated Command | Exists Today |
|-----------|-------|-------------------|--------------|
| SC1 extract, structural check, pinned ratios | 44/44 extract plus structural plus pinned ratios | `cargo test --workspace -- extract_structural::the_structural_check_passes_for_all_forty_four_corpus_programs ratios::the_gate_passes_on_the_committed_file corpus_sweep::all_forty_four_corpus_executables_read_and_report_what_they_hold` | Yes |
| SC1 schema validation | All 44 reports validate against a published JSON schema | New test, e.g. `crates/deform6/tests/schema.rs` | No, Wave 0 gap |
| SC2 no forbidden claim | grep over `README.md`, `--help` output, report vocabulary | `sh scripts/check-claim-surface.sh` | No, Wave 0 gap |
| SC3 README states the three facts | grep for the three named sentences | `sh scripts/check-claim-surface.sh` or a dedicated script | No, Wave 0 gap |
| SC4 README lists every open gap | README cross-referenced against both gap registers | Manual review, plus a count check against the two registers | No, inherently a document review |
| SC5 fmt, clippy, test, doc, tag | Full gate plus zero doc warnings plus tag equals `Cargo.toml` version | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`, then `cargo doc --no-deps` with a zero-warning assertion, then a tag-versus-version check | fmt, clippy, test: yes. Doc warnings and tag check: no, Wave 0 gaps |

---

## Wave 0 Requirements

- [ ] `crates/deform6/schema/report.schema.json`: the schema SC1 validates against. Does not exist.
- [ ] `crates/deform6/tests/schema.rs`: validates all 44 produced reports against that schema. Does not exist.
- [ ] `scripts/check-claim-surface.sh`: implements the SC2 and SC3 greps, in the existing `scripts/prove-*-wall.sh` style. Does not exist.
- [ ] A `cargo doc --no-deps` step in `.github/workflows/gate.yml`, asserting zero warnings, after the current warnings are fixed (52 lines begin `warning` today: 49 located plus 3 per-crate summary lines).
- [ ] `README.md`: does not exist. The whole file is new.
- [ ] `CHANGELOG.md`: does not exist.
- [ ] `LICENSES.md`: does not exist. Holds the manual dependency licence audit, re-derived from `cargo metadata`.
- [ ] A tag-versus-`Cargo.toml`-version check. No automation exists.

Framework install: none needed. `cargo test` is already the only test runner.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| README lists every open gap from STRUCTURES section 11 and FILE-FORMATS section 9, each with the safe default the tool chose | SC4 | A prose cross-reference against two registers. String matching against prose is brittle and would pass for the wrong reason. | Read `.planning/research/STRUCTURES.md` section 11 and `.planning/research/FILE-FORMATS.md` section 9. For each row still open, find its entry in the README limits list and confirm the stated default matches the code. A count check (open rows equals listed limits) is automatable and complements, but does not replace, this review. |
| Full recompilation of an extracted project | SC3 | Needs the VB6 IDE on Windows, which this project does not have. The phase documents this gap rather than closing it. | None. The README states it was not tested and why. |
| P-code branch of `lpNativeCode` | SC3 | Untested by construction. All 44 corpus programs carry `CompilationType=0`. | None. The README states the branch is untested and why. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
