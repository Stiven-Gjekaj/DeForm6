# Phase 6: Version 1.0 - Pattern Map

**Mapped:** 2026-09-14
**Files analyzed:** 9 (3 pure-new prose, 2 new code/schema, 1 new script, 3 modified)
**Analogs found:** 8 / 9 (one prose file, `CHANGELOG.md`, has no in-repo format precedent)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `README.md` | documentation | transform (facts -> prose) | `AGENTS.md` (voice/register), `corpus/NOTICES` (structured facts prose) | role-match |
| `CHANGELOG.md` | documentation | batch (append-only log) | none in repo | no analog |
| `LICENSES.md` | documentation | batch (manual audit table) | `corpus/NOTICES` | exact |
| `crates/deform6/schema/report.schema.json` | config/schema | transform | `crates/deform6/src/report.rs` (source of truth types) | role-match (source, not schema) |
| `crates/deform6/tests/schema.rs` | test | request-response (build report, validate) | `crates/deform6/tests/ratios.rs`, `crates/deform6/tests/extract_structural.rs` | exact |
| `scripts/check-claim-surface.sh` | utility/script | batch (grep sweep) | `scripts/prove-lint-wall.sh` | exact |
| `.github/workflows/gate.yml` | config | batch (CI pipeline) | itself (existing steps) | exact |
| `Cargo.toml` | config | — | itself (`[workspace.package] version`) | exact |
| doc comments in `crates/deform6/src/*.rs`, `crates/xtask/src/*.rs` | source (docs only) | transform | existing doc comments in same files (`report.rs`, `controltree.rs`, `object.rs`, `error.rs`) | exact |

## Pattern Assignments

### `README.md` (documentation)

**Analogs:** `AGENTS.md` (repo root, for voice) and `corpus/NOTICES` (for how this repo turns measured facts into short structured prose).

**Voice constraint** — every sentence in the README must follow ASD-STE100 the same way `AGENTS.md` line 125-132 does:
```
DeForm6 reads a compiled Visual Basic 6 executable and writes back a Visual
Basic project.
The first milestone recovers the metadata only: the forms, the control trees,
the property values, the names, and the procedure signatures.
It does not recover statements.
Do not add a claim that it does.
```
Short sentences, active voice, present tense, no em-dash, no emoji, no unmeasured claim.

**Structured-facts pattern** (`corpus/NOTICES`, full file read this session):
```
# corpus/NOTICES

Every program in `corpus/` is third party work.
It is here because its licence permits redistribution.
This file names the origin, the licence, and the upstream commit of each one.
...
## corpus/vb6-code

Origin: https://github.com/tannerhelland/vb6-code
Upstream commit: f2703861b9e6a7ed4acdc6eedb2632ca8903f895
Licence: BSD 2-Clause, Copyright (c) 2018, Tanner Helland
...
```
Copy this shape for the README's gap list and confidence-vocabulary section: a short lead sentence stating what the section is, then a flat list or table of concrete facts, each one traceable to a specific source (`STRUCTURES.md` §11, `FILE-FORMATS.md` §9, or a `report.rs` string), never a paraphrase.

**Reuse verbatim** (per RESEARCH.md's own Code Examples, already extracted from `crates/deform6/src/report.rs` lines 377-382 and `crates/deform6/tests/support/frm.rs` lines 307-315):
- The recompilation-limit sentence.
- The `frmHMM.frx` exclusion reason.

---

### `LICENSES.md` (documentation)

**Analog:** `corpus/NOTICES` (exact structural match — a manual, hand-audited table of third-party origin/licence facts, already the project's own idiom for this exact kind of audit).

**Pattern to copy** (same file, header + per-entry shape shown above): one lead paragraph stating why the file exists and how it was built (`re-derived from cargo metadata`, per the phase's resolved decision), then either a flat list (for the small number of `[[package]]` entries with a distinct licence) or a table with columns `Crate | Version | Licence | Source`. Do not add `cargo-deny`; this stays a manual, `corpus/NOTICES`-shaped document per the phase's resolved decision.

---

### `crates/deform6/schema/report.schema.json` (new schema artifact)

**Analog:** `crates/deform6/src/report.rs` is the source of truth to translate, not a schema-shaped file — read for field derivation.

**Imports / type shape** (lines 1-40, already read this session):
```rust
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ProjectReport {
    pub items: Vec<ReportItem>,
    pub defects: Vec<Defect>,
    pub limits: Vec<String>,
}
```
`report.json`'s top level has exactly three keys: `items`, `defects`, `limits` (verified live-run this session, quoted in RESEARCH.md). The schema's top-level `"required"` and `"properties"` must list exactly these three, no more.

`Confidence` (lines 78-92 region, `#[serde(rename_all = "lowercase")]`) — schema enum must be exactly `["proven", "inferred", "unrecoverable"]`.

`Defect`/`Evidence`/`Severity` types live in `crates/deform6/src/error.rs`; read that file's struct/enum definitions the same way before finalizing the schema's `defects` item shape (not yet read this session — read it during planning/execution, not copied here, since RESEARCH.md names it but this pass did not need its exact fields for pattern extraction).

---

### `crates/deform6/tests/schema.rs` (new test)

**Analog:** `crates/deform6/tests/extract_structural.rs` (closest shape: drives the real path over all 44 corpus programs and checks each output against an external, independently-sourced expectation) and `crates/deform6/tests/ratios.rs` (closest shape: a pinned/committed artifact gate with a clear two-directional failure message).

**Header lint-allow block** (`extract_structural.rs` lines 1-9, `ratios.rs` lines 1-9 — identical block in both, copy verbatim):
```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
```

**Module doc-comment pattern** (`extract_structural.rs` lines 11-25): state plainly what this file proves and, just as important, what it does *not* claim (no VB6 IDE, no recompilation). `schema.rs`'s own doc comment should state it proves all 44 produced `report.json` files validate against the committed schema, and that a passing run never claims the schema itself is complete or hand-checked beyond that.

**Shared support module import** (`extract_structural.rs` lines 27-32):
```rust
#[path = "support/mod.rs"]
#[allow(
    dead_code,
    reason = "cargo compiles this shared module into every test binary and this one uses only \
              the frm and vbp readers"
)]
mod support;
```
`schema.rs` likely needs the corpus-listing helper from `tests/support/` (check `support/mod.rs` for a corpus-iteration function before writing a new one — do not re-walk `corpus/` by hand).

**Validator dependency:** RESEARCH.md's Don't Hand-Roll table recommends a real `jsonschema`-crate-based validator, or a `serde`-derived round-trip self-test, over hand-rolled `assert!` field checks. No `jsonschema`-shaped dependency exists in `Cargo.toml` today — this is new, and should be added to `[workspace.dependencies]` following the existing one-line style (e.g. `serde_json = "1.0"`).

---

### `scripts/check-claim-surface.sh` (new script)

**Analog:** `scripts/prove-lint-wall.sh` (full file read this session — the canonical shape all four existing `prove-*-wall.sh` scripts share).

**Skeleton to copy** (structure only, not the lint-probe specifics):
```sh
#!/bin/sh
#
# <one paragraph: what invariant this proves, and why the three gate
#  commands (fmt/clippy/test) cannot catch it>
#
# Run it from anywhere:
#
#     sh scripts/check-claim-surface.sh
#
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

WORK=$(mktemp -d)
cleanup() {
	rm -rf "$WORK"
}
trap cleanup EXIT

# ... probe / check body ...

if [ "$fail" -ne 0 ]; then
	echo
	echo "<clear failure message naming what was found and where>"
	exit 1
fi

echo
echo "<clear pass message stating what was proved>"
```

**Probe-and-check pattern:** `prove-lint-wall.sh` writes a temp probe file, runs a real tool against it (`cargo clippy --message-format json`), parses structured output with `jq`, and reports PASS/FAIL per line before cleaning up via a `trap cleanup EXIT`. `check-claim-surface.sh`'s probe is simpler (no source file to write): grep `README.md`, `./target/debug/deform6 --help`/`inspect --help`/`extract --help` output, and the report vocabulary (`basis`/`limits` strings in `report.rs`, or a live `report.json`) for forbidden shapes (a bare `%` sign, a numeric confidence score, a claim of statement recovery). Per RESEARCH.md's Open Questions #2, explicitly exclude `tests/ratios.toml` from the grep scope — it is pinned test data, not a claim surface.

**Failure-message discipline** (`prove-lint-wall.sh`, the FAIL branch): name exactly what shape was found and in which file/line, mirroring:
```sh
printf 'FAIL  line %-3s %-32s %s reached the compiler and no lint stopped it\n' \
	"$line" "$lint" "$description"
```

---

### `.github/workflows/gate.yml` (modified — add `cargo doc --no-deps` step)

**Analog:** the file's own existing steps (full file read this session). Every wall-proving step already follows one shape: a short comment explaining *why* the three base gate commands cannot catch this, then a one-line `run: sh scripts/prove-*.sh`.

**Pattern for the new step** (model on the existing `Prove the ordered output wall` step, lines ~76-86):
```yaml
      # cargo fmt, cargo clippy and cargo test never run cargo doc, so a
      # broken intra-doc link or a public doc that names a private item
      # passes the gate silently. This step is the only guard on that.
      - name: cargo doc
        run: cargo doc --no-deps --workspace
```
Since `cargo doc` does not exit non-zero on warnings by default, either add `RUSTDOCFLAGS: -D warnings` to the step's `env:` (matching the existing top-level `env: CARGO_TERM_COLOR: always` block, lines 20-21) or pipe through a warning-count check the way `check-claim-surface.sh` does. Place the step after `cargo test` and before the four `Prove the * wall` steps, since it is a base-gate-shaped check, not a wall-proving script.

---

### `Cargo.toml` (modified — version bump)

**Analog:** itself. Single-line change:
```toml
[workspace.package]
version = "0.1.0"
```
becomes `version = "1.0.0"`, per the phase's resolved decision. This is the only workspace-level version pin (all three crates inherit it via `crates/*/Cargo.toml` — confirm each crate's `Cargo.toml` says `version.workspace = true` before assuming a single-line edit suffices; not verified this session, verify during execution).

---

### Doc-comment fixes in `crates/deform6/src/*.rs` and `crates/xtask/src/*.rs`

**Measured this session:** `cargo doc --no-deps --workspace` currently emits two dominant warning shapes (grep counts above):

1. **"public documentation for `X` links to private item `Y`"** — e.g. `crates/deform6/src/vb/object.rs:118`, `crates/deform6/src/vb/controltree.rs:782`, `crates/deform6/src/report.rs:436`, `crates/deform6/src/error.rs:512`. The idiomatic fix, matching this codebase's own convention of naming a private helper in prose rather than a broken intra-doc link, is to drop the `[\`Y\`]` markdown-link brackets and keep the identifier as plain code-formatted text:
   ```rust
   // before
   /// Each element is narrowed to its own [`OBJECT_SIZE`]-byte window before
   // after
   /// Each element is narrowed to its own `OBJECT_SIZE`-byte window before
   ```
2. **"unresolved link to `X`"** — e.g. `crates/deform6/src/vb/ocx.rs:150` (`[\`Display\`]` should resolve to `std::fmt::Display`, needs a full path or import-scoped link), and several links to test function names from doc comments in library modules (`SafeName::file_name`, `ComponentTable::walk`, and test names like `the_gate_passes_on_the_committed_file`). For the `std` case, fully qualify (`[\`std::fmt::Display\`]`); for cross-crate links into `tests/*.rs` (which rustdoc cannot resolve since `--no-deps` never compiles the test crate), the same private-item fix applies: drop the brackets.

**Fix-in-place is correct; no new file needed.** This is a bulk sed-shaped or per-instance manual edit across the ~15 distinct source locations already located by `cargo doc --no-deps --workspace 2>&1 | grep -B1 "^warning"` (grep, not Read, keeps this cheap — do not `Read` all 43+6 warnings' full files during planning; the executor should grep the exact line numbers again at execution time since line numbers shift after each fix).

## Shared Patterns

### ASD-STE100 prose discipline
**Source:** `AGENTS.md` lines 23-34, and this repo's own `AGENTS.md`/`CLAUDE.md` global override.
**Apply to:** `README.md`, `CHANGELOG.md`, `LICENSES.md`, and every doc-comment edit.
```
- Write all text for this repository in ASD-STE100 Simplified Technical
  English.
- Use short sentences. Use the active voice. Use the present tense.
- Do not use an em-dash. Do not use an emoji.
```

### The `prove-*-wall.sh` acceptance-script shape
**Source:** `scripts/prove-lint-wall.sh` (full file), `scripts/prove-region-wall.sh`, `scripts/prove-capacity-wall.sh`, `scripts/prove-ordered-output-wall.sh` (named, not re-read — same shape confirmed by their shared invocation pattern in `gate.yml`).
**Apply to:** `scripts/check-claim-surface.sh`.
`set -eu`, `ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)`, a `trap cleanup EXIT`, PASS/FAIL lines with concrete names, and a nonzero `exit 1` only on real failure.

### Test-file lint-allow header
**Source:** `crates/deform6/tests/ratios.rs` lines 1-9, `crates/deform6/tests/extract_structural.rs` lines 1-9 (identical block).
**Apply to:** `crates/deform6/tests/schema.rs`.
```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
```

### `gate.yml` step shape
**Source:** `.github/workflows/gate.yml`, every step from `cargo fmt` through `Prove the ordered output wall`.
**Apply to:** the new `cargo doc` step.
A `# why this and not the base three commands` comment block, then a `- name:` / `run:` pair, no extra shell wrapping unless the check needs a temp file.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `CHANGELOG.md` | documentation | batch | RESEARCH.md states explicitly: "no format precedent in this repository to follow, a plain reverse-chronological Markdown list is a reasonable default." Use `## [1.0.0] - <date>` headed sections, ASD-STE100 prose per the shared pattern above, and no other project file to imitate. |

## Metadata

**Analog search scope:** repo root, `scripts/`, `.github/workflows/`, `crates/deform6/src/report.rs`, `crates/deform6/src/error.rs` (named not fully read), `crates/deform6/tests/*.rs`, `corpus/NOTICES`, `AGENTS.md`, `Cargo.toml`.
**Files scanned:** ~15 (4 `prove-*-wall.sh` scripts named/1 read in full, `gate.yml` read in full, `Cargo.toml` read in full, `report.rs` partially read, `ratios.rs`/`extract_structural.rs` headers read, `corpus/NOTICES` read in full, `AGENTS.md` read in full, live `cargo doc --no-deps --workspace` run for warning inventory).
**Pattern extraction date:** 2026-09-14

## PATTERN MAPPING COMPLETE
