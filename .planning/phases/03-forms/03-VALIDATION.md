---
phase: "3"
slug: "forms"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-09"
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test`, unchanged from phase 1 and phase 2 |
| **Config file** | none, `cargo test --workspace` is the whole invocation |
| **Quick run command** | `cargo test -p deform6 --lib` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds for the full suite at the phase 2 test count of 308 |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p deform6 --lib`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Phase gate, from `AGENTS.md`, every one of these before every commit:**
  `cargo fmt --all --check`, then
  `cargo clippy --all-targets -- -D warnings`, then
  `cargo test --workspace`
- **Max feedback latency:** 30 seconds

`cargo test --workspace` is the build check, never `cargo build`. `build` does
not compile a `#[cfg(test)]` module.

---

## Per-Task Verification Map

The planner fills the Task ID and Wave columns. The requirement rows and the
commands come from `03-RESEARCH.md` "Validation Architecture" and hold as
written.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| {planner} | 01 | 1 | FRM-01 | — | Refuses a tree it cannot tile; names the byte offset | unit + integration | `cargo test -p deform6 --lib vb::gui:: vb::controltree::` | ❌ W0 | ⬜ pending |
| {planner} | 02 | 1 | FRM-03 | — | Reports an undecoded property with its opcode and offset; never guesses a name | unit | `cargo test -p deform6 --lib vb::opcodes::` | ❌ W0 | ⬜ pending |
| {planner} | 03 | 1 | VER-06 | — | Excludes `frmHMM.frx` by name; removing the exclusion fails with a named message | integration | `cargo test -p deform6 --test differential` | ❌ W0 | ⬜ pending |
| {planner} | 04 | 2 | FRM-02 | — | Control type, name and array `Index` from offset `0x05` | unit | `cargo test -p deform6 --lib vb::controltree::` | ❌ W0 | ⬜ pending |
| {planner} | 05 | 2 | FRM-03 | — | A string read that does not land on the field end is unrecoverable, with its offset | unit | `cargo test -p deform6 --lib vb::vbstr::` | ❌ W0 | ⬜ pending |
| {planner} | 06 | 3 | FRM-03 | — | Position block escape at `-32768` reads 16 bytes, not 8 | unit | `cargo test -p deform6 --lib vb::propstream::` | ❌ W0 | ⬜ pending |
| {planner} | 08 | 3 | FRM-04 | — | OCX CLSID resolved by class name; blob stated as uninterpretable | unit + integration | `cargo test -p deform6 --lib vb::ocx::` | ❌ W0 | ⬜ pending |
| {planner} | 09 | 3 | FRM-06 | — | Event handler names joined per control | unit | `cargo test -p deform6 --lib vb::controlinfo::` | ❌ W0 | ⬜ pending |
| {planner} | 07 | 4 | FRM-05 | — | `.frx` blob extraction with an offset cursor and an image format sniff | unit | `cargo test -p deform6 --lib vb::frx::` | ❌ W0 | ⬜ pending |
| {planner} | 10 | 5 | all of the above | — | Differential gate compares both directions against `support/frm.rs` | integration | `cargo test -p deform6 --test differential` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Every file below is new. No phase 3 task has an automated verify command that
resolves until its Wave 0 file exists.

- [ ] `crates/deform6/src/vb/gui.rs` — `GuiTable`, `GuiObjectInfo`, the tiling invariant (FRM-01)
- [ ] `crates/deform6/src/vb/controltree.rs` — scope-byte walk, `cType`, name, array `Index` (FRM-01, FRM-02)
- [ ] `crates/deform6/src/vb/vbstr.rs` — the encoding-validating string reader (FRM-03)
- [ ] `crates/deform6/src/vb/propstream.rs` — typed payloads, position block escape, `Font` block (FRM-03)
- [ ] `crates/deform6/src/vb/opcodes.rs` — table format, optional loader, the safe-provenance subset (FRM-03)
- [ ] `crates/deform6/src/vb/frx.rs` — blob extraction, offset cursor, image sniffing (FRM-05)
- [ ] `crates/deform6/src/vb/ocx.rs` — `cType 255`, CLSID join, fixed OCX header (FRM-04)
- [ ] `crates/deform6/src/vb/controlinfo.rs` — `ControlInfo`, event handler table (FRM-06)
- [ ] `crates/deform6/tests/support/frm.rs` — the independent `.frm` reader, and the named `frmHMM.frx` exclusion (VER-06)
- [ ] `.gitattributes` entries for `*.frx` and `*.ctx`, `-text` or `binary`, before any `.frx`-bearing fixture enters the repository

### Two rules `support/frm.rs` must satisfy at Wave 0

Both come from `03-CONTEXT.md`. Both are measured, not assumed. Either one,
missed, makes the pinned form ratio report a full pass over an incomplete set.

- [ ] It walks with `eq_ignore_ascii_case`, the way `tests/support/vbp.rs`
      already does. `corpus/public-domain/SK-MCI-Sample__VB6/MCI.FRM` has an
      upper case extension. A `*.frm` match finds 53 of 54 forms.
- [ ] It reads bytes, never a `String`.
      `corpus/vb6-code/Threshold-effect/Threshold.frm` holds byte `0xA9` at
      offset 5669 and is not valid UTF-8. `fs::read_to_string` fails on it.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The opcode-to-property table beyond the safe-provenance subset, about 158 of the 198 corpus pairs | FRM-03 | Needs a human at a working VB6 install. Per `03-CONTEXT.md` D-01 the table is never committed, so no automated test in this repository can assert its contents. | Build the table locally with the committed derivation tool. Run `deform6 inspect --opcode-table <path>` and compare against the `.frm` source. Follow-up work, gated behind a `checkpoint:human-verify` task, outside phase 3. |
| Closing a remaining `(control type, property)` pair | FRM-03 | Needs VB6 to compile a single-property probe program. | Use the `STRUCTURES.md` §13 method: compile a small original program that sets one property, and diff the compiled bytes against a blank control. |

Every phase 3 behavior other than the two rows above has automated
verification.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] Every new test was broken on purpose once and seen to fail, per `AGENTS.md`
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
