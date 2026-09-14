# Phase 6: Version 1.0 - Research

**Researched:** 2026-09-14
**Domain:** Release documentation and re-verification of a Rust CLI/library (no new code recovery feature)
**Confidence:** HIGH

<phase_requirements>
## Phase Requirements

Phase 6 owns no new requirement ID. Per `REQUIREMENTS.md`'s Traceability
table and `ROADMAP.md`'s own phase text (both read this session), it
re-verifies the requirement categories already delivered in Phases 1-5 end
to end and turns the measured result into the released documentation.

| Category | Description | Research Support |
|----------|-------------|--------------------|
| DET (DET-01 to DET-06) | Entry point, header, runtime discrimination, refusals, native/P-code report | Re-verified by `cargo test --workspace` (all green this session) and the corpus sweep test; the P-code-untested fact is documented in `crates/deform6/src/vb/project.rs` doc comments (read this session) and must reach the README per success criterion 3 |
| OBJ (OBJ-01 to OBJ-06) | Object graph, kinds, procedures, prototypes, `Declare` table | Re-verified by `differential.rs`'s two-directional checks (present, passing) |
| FRM (FRM-01 to FRM-06) | Control tree, types, properties, OCX identifiers, `.frx` blobs, event structure | FRM-01/02/03 stay intentionally open per `REQUIREMENTS.md`; their exact open-gap wording must feed success criterion 4's README gap list, sourced from `STRUCTURES.md` section 11 (quoted in full in this research) |
| WRT (WRT-01 to WRT-07) | `.vbp`/`.frm`/`.frx`/`.bas`/`.cls` writers | WRT-03 stays open per `REQUIREMENTS.md` (unit-proved only, no corpus-wide check); must also appear in the README's open-gap list |
| RPT (RPT-01 to RPT-06) | The JSON confidence report | Fully verified this session by both source reading and a live `extract` run; see Code Examples for the exact shape success criterion 1's schema must describe |
| SAF (SAF-01 to SAF-05) | Hostile-input handling, `--salvage`, fuzzing | Fully complete per Phase 5's UAT (23/23 passed); this research's Security Domain section maps each to its ASVS-equivalent control for re-verification, not new work |
| VER (VER-01 to VER-06) | Differential harness, pinned ratios, `frmHMM.frx` exclusion | `tests/ratios.toml` confirmed to hold exactly 44 entries this session; the pinned-ratio gate tests (`ratios.rs`) all pass; VER-06's exclusion reason is quoted verbatim in Code Examples for README reuse |

</phase_requirements>

## Summary

Phase 6 has no new requirement and no new parsing code. It closes the milestone
by measuring the finished tool end to end and writing that measurement down.
Three facts dominate the research: **there is no `README.md` in this repository
today** (the whole file is new work for plan 06-01), **no JSON schema file
exists** for the report success criterion 1 asks the planner to validate
against, and **`cargo doc --no-deps` currently emits 49 real warnings** (43 in
`deform6`, 6 in `xtask`) that must reach zero for success criterion 5. Every
other measured signal is good news: `cargo fmt --all --check`, `cargo clippy
--all-targets -- -D warnings` and `cargo test --workspace` (974 tests, release
profile) all pass clean today, the corpus is confirmed at exactly 44 programs,
`tests/ratios.toml` holds exactly 44 pinned entries, and the CLI `--help` text
and the report vocabulary (`Confidence::Proven` / `Inferred` / `Unrecoverable`)
already contain no percentage figure and no claim of statement recovery. The
`.gitattributes` file the roadmap names for `*.frx`/`*.ctx` already exists and
already marks both binary, so that piece of plan 06-05 is already done; what is
missing from release mechanics is a `CHANGELOG.md`, a version tag (only one
unrelated tag exists in the repo), and any automated licence check
(`cargo-deny` is not installed, no `deny.toml` exists).

**Primary recommendation:** Treat 06-01 (README) and 06-04 (honesty audit) as
the two plans that do new writing; treat 06-02 (doc comments), 06-03
(acceptance run) and 06-05 (release mechanics) as measurement-and-fix plans
against the gaps this research found (broken intra-doc links, a missing
schema, missing changelog, missing licence check, missing version tag). Do not
plan new parsing, new CLI flags, or new report fields; the roadmap explicitly
forbids scope growth in this phase.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| README claim surface | Documentation (repo root) | — | Read before the tool runs; no code tier owns it |
| `--help` / `about` text | CLI (`deform6-cli`) | — | `clap` derive reads doc comments on `Cli`/`Command` |
| Report vocabulary (`Confidence`, `basis`, `limits`) | Library (`deform6` crate, `report.rs`) | CLI (prints it) | The library builds the JSON; the CLI only writes it to disk |
| JSON schema for the report | Library (new artifact, `crates/deform6/schema/` or similar) | Test harness (validates against it) | The schema describes the library's own serialised type; it belongs beside the crate that owns `ProjectReport` |
| Acceptance run over 44 corpus programs | Test harness (`crates/deform6/tests/*.rs`) | CI (`gate.yml`) | Existing `#[test]` functions already do this; Phase 6 adds no new read path |
| Gap inventory (STRUCTURES §11, FILE-FORMATS §9) | Documentation (`.planning/research/`, mirrored into README) | — | Source of truth already exists; Phase 6 is a copy-and-verify job, not a new survey |
| Licence check / release mechanics | Build tooling (`crates/xtask` or a new `deny.toml`) | CI (`gate.yml` or a new `release.yml`) | Consistent with how `xtask` already owns `update-ratios`, `fetch-corpus`, `pin-corpus` |
| Version tag | Git / release process | CI (optional tag-triggered workflow) | No workflow currently reacts to a tag push |

## Package Legitimacy Audit

Phase 6 adds no required runtime dependency. One optional tool is worth
naming for plan 06-05's licence-check requirement:

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `cargo-deny` | crates.io | Long-established (EmbarkStudios tool, widely used in the Rust ecosystem) | High (in the thousands/week range for a dev-tool) | github.com/EmbarkStudios/cargo-deny | Not run through the automated `package-legitimacy check` seam this session (offline verdict unavailable); version confirmed present on the registry | Optional — recommend, do not mandate |

`cargo search cargo-deny` was run this session and returned
`cargo-deny = "0.20.2"` at the head of the result list
`[VERIFIED: crates.io registry, cargo search cargo-deny, this session]`. This
package name is well known from training data as well
`[ASSUMED: package identity/purpose beyond what cargo search's one-line
description states]`. Because it is a `cargo install`-time dev tool, not a
crate the workspace links against, it never enters `Cargo.lock` or the
built binary; if the planner adopts it, gate it behind a
`checkpoint:human-verify` task per the package-legitimacy protocol, since its
full legitimacy signals (download counts, exact maintainer identity) were not
machine-checked this session.

**Packages removed due to [SLOP] verdict:** none — no new required dependency.
**Packages flagged as suspicious [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

Phase 6 touches no runtime data flow. The diagram below shows the
*documentation and verification* flow this phase adds around the existing,
unchanged `inspect`/`extract` pipeline.

```
                 ┌────────────────────────────────────────┐
                 │   Existing pipeline (Phases 1-5, frozen) │
                 │  PE bytes -> vb::inspect -> Report        │
                 │             -> write::project -> files    │
                 │             -> report::build -> JSON       │
                 └───────────────────┬──────────────────────┘
                                     │ (44 corpus programs, unchanged)
                                     v
   ┌───────────────┐   measures   ┌─────────────────────────┐
   │ tests/ratios   │◄─────────────┤ 06-03: acceptance run    │
   │ .toml (pinned) │   compares   │ (existing #[test] fns,   │
   └───────┬────────┘              │  cargo test --workspace) │
           │                       └────────────┬─────────────┘
           │ numbers recorded                   │ numbers recorded
           v                                    v
   ┌──────────────────────────────────────────────────────┐
   │ 06-04: honesty audit                                   │
   │ every claim source (README draft, --help, report vocab) │
   │   checked against a measured number or a cited fact     │
   └───────────────────────┬──────────────────────────────┘
                            v
   ┌──────────────────────────────────────────────────────┐
   │ 06-01: README.md (new file)                            │
   │ 06-02: doc comments -> cargo doc --no-deps (0 warnings) │
   │ 06-05: CHANGELOG, licence check, .gitattributes, tag    │
   └──────────────────────────────────────────────────────┘
```

A reader traces the primary use case (does the released claim match a
measured fact?) by following: pipeline output -> pinned ratios -> acceptance
run -> honesty audit -> README/docs/release artifacts.

### Recommended Project Structure

No new `src/` module is needed. New files this phase plausibly adds:

```
README.md                       # new: 06-01
CHANGELOG.md                    # new: 06-05
deny.toml                       # new, optional: 06-05 (only if cargo-deny adopted)
crates/deform6/schema/          # new: report JSON schema, likely 06-03 or 06-02
  report.schema.json
scripts/
  check-claim-surface.sh        # new: implements success criterion 2's grep, in the
                                 #      existing scripts/prove-*-wall.sh style
```

### Pattern 1: The existing `scripts/prove-*-wall.sh` acceptance-script shape

**What:** Every cross-cutting invariant this repository already enforces
(lint wall, `Region` wall, capacity wall, ordered-output wall) is a small
POSIX `sh` script under `scripts/`, invoked as its own named step in
`.github/workflows/gate.yml`, each with a comment block explaining *why* the
three gate commands alone cannot catch the thing it proves.

**When to use:** Success criterion 2 ("a grep over `README.md`, the `--help`
output and the report vocabulary finds no claim...") is exactly this shape: a
check the three gate commands cannot express. Write it as
`scripts/check-claim-surface.sh` and wire it into `gate.yml` as a new step,
matching the existing five-step pattern exactly.

**Example:**
```sh
# Source: crates/deform6/../scripts/prove-lint-wall.sh (this repository, read this session)
#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
# ... probe, check, cleanup, matching the existing four scripts' shape ...
```

### Pattern 2: Confidence vocabulary is a closed three-word `enum`, already shipped

**What:** `Confidence` in `crates/deform6/src/report.rs` (read this session)
has exactly three variants and a `#[serde(rename_all = "lowercase")]`
attribute, so the JSON field can only ever be `"proven"`, `"inferred"` or
`"unrecoverable"`.

**When to use:** The README's "confidence vocabulary" (roadmap plan 06-01)
should define these three words and no others; do not invent a fourth tier
or a numeric score in the README, since the code cannot produce one and the
grep in success criterion 2 would then find a claim the tool cannot back.

```rust
// Source: crates/deform6/src/report.rs, lines 78-92 (read this session)
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// The exact byte this repository read names the fact directly.
    Proven,
    /// This repository chose the fact because the file gives no other
    /// answer, and the basis says so.
    Inferred,
    /// This repository could not recover the fact at all.
    Unrecoverable,
}
```

### Anti-Patterns to Avoid

- **Writing the README before the measurement.** The roadmap's own named risk
  ("The README is the last place a claim can drift") and the wave order
  (`[06-01, 06-02, 06-05]` then `[06-03]` then `[06-04]`) already put the
  README plan *before* the acceptance-run plan. This research flags that the
  planner should treat 06-01's own numeric claims (corpus count, ratio
  values) as provisional until 06-03 and 06-04 confirm them, even though
  06-01 runs in wave 1. A safe pattern: 06-01 writes the prose structure and
  the confidence vocabulary (neither depends on fresh measurement, both
  already match the code), and defers every numeric figure to a placeholder
  06-04 fills in, rather than writing a number in wave 1 that 06-03 might
  move in wave 2.
- **Re-deriving the gap register.** STRUCTURES.md section 11 and
  FILE-FORMATS.md section 9 are already complete, cited registers (see
  below). Do not re-survey the codebase for gaps; copy and verify against
  the register, then check each row against current code behaviour.
- **Treating `tests/ratios.toml`'s `ratio` key as a JSON report field.** It
  is not. The written `report.json` has exactly three top-level keys
  (`items`, `defects`, `limits`) and carries no field literally named
  `recovery.ratio` or `ratio` anywhere `[VERIFIED: crates/deform6/src/report.rs,
  read this session, and a live report.json produced this session — see Open
  Questions]`. The roadmap's "measured `recovery.ratio`" phrase refers to
  `tests/ratios.toml`'s per-program `ratio` field, not a JSON API surface.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Grep the claim surface (criterion 2) | A one-off ad hoc grep run by hand at release time | `scripts/check-claim-surface.sh`, wired into `gate.yml`, matching the four existing `prove-*-wall.sh` scripts | The existing four scripts are the project's own established idiom for "a rule no lint and no type can express"; a fifth script keeps the gate homogeneous and re-runnable, rather than a manual step someone forgets next release |
| Licence auditing dependency tree | Hand-checking each of the 81 crates in `Cargo.lock` | `cargo-deny` with a `deny.toml`, or at minimum `cargo license` if a lighter tool is preferred | 81 unique crate names are in `Cargo.lock` today `[VERIFIED: grep -c "^name = " Cargo.lock, this session]`; hand-auditing that many transitive dependencies on every release is exactly the "measure it, don't eyeball it" discipline AGENTS.md asks for |
| JSON schema validation | A hand-rolled set of `assert!` checks on parsed JSON in a new test | A real JSON Schema file plus a small validation step (either `jsonschema` crate, or a `serde`-derived round-trip check that doubles as the schema's own self-test) | Success criterion 1 explicitly names "validates against the schema," which presumes a schema artifact exists; none does today, so this is new work, not a re-verification, and should use a real validator rather than ad hoc field presence checks that silently drift from the schema file |

**Key insight:** This phase's don't-hand-roll risk is not a parsing risk (Phase
6 parses nothing new); it is a release-tooling risk. The project already has
a strong, consistent idiom (`xtask` subcommands, `scripts/prove-*-wall.sh`,
`tests/*.rs`) for exactly this kind of cross-cutting check. Every new check
this phase needs should extend one of those three idioms, not invent a
fourth.

## Common Pitfalls

### Pitfall 1: Writing a README claim that outruns the report's own vocabulary

**What goes wrong:** A README sentence like "DeForm6 recovers most of a
project's structure" reads as harmless but has no measured backing and no
grep can catch loose prose the way it catches a bare percentage sign.
**Why it happens:** Prose is easier to write persuasively than honestly, and
the roadmap's own named risk calls this out directly ("A commercial tool in
this field advertises 85% recovery with no evidence behind it").
**How to avoid:** Every quantitative or qualitative capability claim in the
README should be traceable to one of: a `tests/ratios.toml` total (the file's
own header comment already states two, `185 recovered and 904 declared` for
procedures, and `52 of 53 forms`/`686 of 686 controls`), a `Confidence`
variant name, or a `limits` string the report already emits verbatim
(`build_limits` in `report.rs`, read this session, already states the
recompilation and code-page limits in exact words the README can reuse).
**Warning signs:** A sentence with no noun that maps to a report field, a
ratio, or a gap-register row.

### Pitfall 2: `cargo doc --no-deps` warnings hiding behind a passing gate

**What goes wrong:** `gate.yml` (read this session) never runs `cargo doc`.
`cargo fmt`, `cargo clippy` and `cargo test` all pass today with the 49 doc
warnings present, so nothing in CI currently signals that success criterion 5
is unmet.
**Why it happens:** `cargo doc` warnings (broken intra-doc links, links to
private items from public docs) are a `rustdoc` lint family separate from
`clippy`'s and are opt-in to check.
**How to avoid:** Plan 06-02 should both fix the 49 current warnings (listed
below, verbatim from this session's run) and add a `cargo doc --no-deps`
step to `gate.yml` (or a dedicated script) so a future PR cannot reintroduce
one silently.
**Warning signs:** `cargo doc --no-deps --workspace 2>&1 | grep -c
"^warning:"` returning nonzero.

### Pitfall 3: Assuming `.gitattributes` still needs writing

**What goes wrong:** The roadmap plan list for 06-05 says "`.gitattributes`
for `*.frx` and `*.ctx`," which reads like new work.
**Why it happens:** The file was actually written in an earlier phase (the
file's own header comment cites the exact `frmHMM.frx` corruption this
research also found independently in `support/frm.rs`), so a planner who
does not check first risks duplicating or, worse, weakening an existing,
well-reasoned rule.
**How to avoid:** `.gitattributes` (repo root, read this session) already
marks `*.frx`, `*.ctx`, `*.dsx`, `*.exe`, `*.dll`, `*.ocx` as `binary` and
`*.frm`, `*.bas`, `*.cls`, `*.ctl`, `*.vbp`, `*.vbw`, `*.mak` as `-text`. Plan
06-05's real work here, if any, is auditing this list is still complete (for
example: does it need `*.res`, or a written project's own newly-created
files?), not authoring it from scratch.
**Warning signs:** A task description that says "add `.gitattributes`" rather
than "verify `.gitattributes`."

## Code Examples

### The exact `--help` text this session measured (already honest, no change needed)

```
// Source: `./target/debug/deform6 --help`, run this session
`deform6`: reads a compiled Visual Basic 6 executable and reports what it holds

Usage:

Commands:
  inspect  Reads one executable and prints what DeForm6 found in it
  extract  Reads one executable and writes a Visual Basic 6 project directory that VB6 can open
  help     Print this message or the help of the given subcommand(s)
```

### The exact `report.json` shape (from a live `extract` run this session)

```json
// Source: /tmp/d6out/passgen/PassGen.report.json, produced this session by
// `./target/debug/deform6 extract corpus/public-domain/PassGen/PassGen.exe -o /tmp/d6out/passgen --force`
{
  "items": [
    {
      "path": "/meta",
      "confidence": "inferred",
      "basis": "a native executable does not carry the CompilationType compiler flag; the IDE default is written",
      "evidence": [
        { "offset": 8780, "structure": "VbHeader", "field": "header_offset", "note": "..." }
      ]
    }
  ],
  "defects": [
    { "site": { "offset": 25008, "rva": null, "structure": "Object", "field": "lpProcNamesArray" },
      "kind": { "UnreadablePointer": { "offset": 25008, "va": 12 } } }
  ],
  "limits": [
    "This is a strict run. It refused any file whose read had to assume a value, so this run assumed none.",
    "Full recompilation did not run. It needs the Visual Basic 6 IDE on Windows, and this run had neither. A structural check ran in its place, and it never opened this project in the IDE.",
    "Opcode table  builtin subset, 59 entries",
    "Every inline string this run wrote inline holds 97 encoded bytes or fewer, the proven floor of an open threshold. A longer string became a resource reference instead of a guessed cutoff.",
    "This run assumes a Western code page. Every character above U+00FF was replaced with a question mark and reported, never guessed at a different code page."
  ]
}
```

The top level has exactly three keys: `items`, `defects`, `limits`
`[VERIFIED: crates/deform6/src/report.rs ProjectReport struct, lines 26-38,
read this session; confirmed by this live run]`. A JSON Schema for this phase
should describe exactly these three arrays and their element shapes; no
fourth key exists to schematize.

### The report's own pre-written recompilation sentence (reuse verbatim in the README)

```rust
// Source: crates/deform6/src/report.rs, lines 377-382 (read this session)
"Full recompilation did not run. It needs the Visual Basic 6 IDE on \
 Windows, and this run had neither. A structural check ran in its \
 place, and it never opened this project in the IDE."
    .to_owned(),
```

### The one named `.frx` exclusion (reuse verbatim in the README)

```rust
// Source: crates/deform6/tests/support/frm.rs, lines 307-315 (read this session)
pub const EXCLUSIONS: &[Exclusion] = &[Exclusion {
    path: "corpus/vb6-code/Hidden-Markov-model/frmHMM.frx",
    reason: "the file is 56 bytes and its own record header declares 56 bytes of payload, but \
             only 55 are present, because the upstream repository sets text=auto and git's line \
             ending normalisation silently removed one carriage return; HMM.exe holds the same \
             string as a whole, correct record and is the second, independent source of truth \
             for this fault",
}];
```

## State of the Art

Not applicable in the usual sense: this phase does not adopt a new external
technology. The one relevant shift is internal-to-the-project:

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| No project-level README (current state) | A README stating capability, confidence vocabulary, and every open gap | Phase 6, plan 06-01 | First-time external-facing documentation; nothing to migrate away from |
| No JSON schema for the report | A committed schema file the acceptance run validates against | Phase 6, plan 06-03 or 06-02 | New artifact; success criterion 1 presumes it exists |

**Deprecated/outdated:** Nothing in this phase deprecates prior work. Phases
1-5 are described by the roadmap as complete and frozen; Phase 6 measures and
documents, it does not rewrite.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `cargo-deny` (or an equivalent tool) is the right mechanism for the licence check success criterion 5 does not explicitly name but plan 06-05's roadmap text does ("the licence check") | Don't Hand-Roll, Package Legitimacy Audit | Low — the roadmap names "the licence check" as a plan 06-05 deliverable but does not name a tool; if the planner or user prefers a lighter manual audit (documented in the README or a `LICENSES.md`) instead of a new dependency, that is equally valid and avoids adding a dev-dependency at v1.0 |
| A2 | The JSON schema file should live under `crates/deform6/schema/` | Recommended Project Structure | Low — purely a location choice; any sensible path works as long as the acceptance run in 06-03 references it |
| A3 | Cargo.toml's `version = "0.1.0"` should be bumped to a `1.0.0`-shaped version for this release, matching the phase name "Version 1.0" | Open Questions | Medium — if the user intends to keep semver `0.x` (common for a tool that still calls itself pre-1.0 in spirit while the *milestone* is named "Version 1.0"), planning a version bump the user did not want would need reverting; this must be confirmed with the user before plan 06-05 commits to a specific tag string |
| A4 | `cargo-deny` on crates.io (`0.20.2`, confirmed via `cargo search` this session) is the same well-known EmbarkStudios tool and not a similarly-named impostor | Package Legitimacy Audit | Low — `cargo search` alone does not prove provenance the way an official-docs citation would; treat as `[ASSUMED]` per the package-name provenance rule and gate behind `checkpoint:human-verify` if adopted |

## Open Questions (RESOLVED)

All three questions below were settled by the user during the /gsd-plan-phase 6
run, before the planner was spawned. Each resolution is recorded inline.

1. **What exactly does "the JSON report validates against the schema" (success criterion 1) mean, given no schema exists today?**
   - What we know: `report.json`'s shape is fully determined by
     `ProjectReport`/`ReportItem`/`Confidence`/`Evidence`/`Defect` in
     `crates/deform6/src/report.rs` and `error.rs`, and is stable (no
     `#[serde(untagged)]` or dynamic shape) `[VERIFIED: report.rs, read this
     session]`.
   - What's unclear: Whether "the schema" means a new JSON Schema file this
     phase must author (most likely reading, since success criterion 1
     phrases it as an existing fact to check, "validates... for all 44,"
     which presumes the schema already exists at execution time, i.e. by the
     time 06-03 runs) or a looser sense of "the report's own Rust type
     definition, unchanged."
   - Recommendation: Plan 06-02 or an early task of 06-03 should author
     `crates/deform6/schema/report.schema.json` (or equivalent) from the
     current Rust types, and 06-03's acceptance run should validate all 44
     produced reports against it. This is new work, not re-verification, and
     the planner should size it as such.
   - **RESOLVED (decision D-01):** Yes, this phase authors the schema. Plan
     06-01 owns it as a dedicated task, ahead of the acceptance run, and adds
     `crates/deform6/tests/schema.rs` to validate all 44 produced reports
     against it. The planner moved the schema to wave 1 for this reason.

2. **Does "the measured `recovery.ratio`" (success criterion 2) refer to a JSON field or to `tests/ratios.toml`'s `ratio` key?**
   - What we know: No field literally named `ratio` or `recovery.ratio`
     exists in `report.json`'s three top-level keys
     `[VERIFIED: report.rs + a live report.json this session]`.
     `tests/ratios.toml` has a `ratio` key per program
     `[VERIFIED: tests/ratios.toml, read this session]`.
   - What's unclear: Whether the grep in criterion 2 is meant to also cover
     `tests/ratios.toml` (which does contain many percentage-shaped decimal
     values like `0.06`) or only `README.md`, `--help`, and `report.json`'s
     `basis`/`limits` strings, as criterion 2's own sentence names.
   - Recommendation: Scope the grep script to the three sources criterion 2
     names literally (`README.md`, `--help` output, report vocabulary) and
     explicitly exclude `tests/ratios.toml`, since that file's numeric ratios
     are pinned test data, not a claim surface a reader encounters before
     running the tool.
   - **RESOLVED (decision D-04):** The grep scopes to the three sources
     criterion 2 names literally, and excludes `tests/ratios.toml`. Plan 06-06
     records the reason in the script, so a later reader does not close the
     gap by mistake.

3. **Should `Cargo.toml`'s version move from `0.1.0` to a `1.0.0`-shaped number for this release?**
   - What we know: `Cargo.toml` currently pins `version = "0.1.0"` workspace-wide
     `[VERIFIED: Cargo.toml, read this session]`. The only git tag in the
     repository is `backup-pre-footer-rewrite`, unrelated to a release
     `[VERIFIED: git tag -l, this session]`. The phase and milestone are both
     named "Version 1.0."
   - What's unclear: Whether "Version 1.0" is a milestone/marketing name only,
     or whether the Cargo package version itself must become `1.0.0` (or
     `1.0.0-rc1`, etc.) to satisfy "the tag matches the version in
     `Cargo.toml`."
   - Recommendation: Confirm with the user before 06-05 locks a tag string;
     this is exactly the kind of decision `/gsd-discuss-phase` normally
     captures, and no `CONTEXT.md` exists for this phase.
   - **RESOLVED (decision D-02):** The workspace version moves to `1.0.0` and
     the tag is `v1.0.0`. Plan 06-04 carries the one-way decision checkpoint,
     and the tag-versus-version match is measured, not asserted.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (`rust-toolchain.toml` pin) | Whole workspace | Yes | rust-version `1.97.1` per workspace manifest; toolchain resolved and built clean this session | — |
| `cargo fmt` | Success criterion 5 | Yes | Passed clean this session | — |
| `cargo clippy` | Success criterion 5 | Yes | Passed clean this session | — |
| `cargo doc` | Success criterion 5 | Yes, but currently produces 49 warnings | Ran this session; 43 in `deform6` lib, 6 in `xtask` bin | Fix the warnings; no tool fallback needed |
| `cargo-deny` (optional) | Plan 06-05 licence check | No — not installed, no `deny.toml` in repo | — | Manual licence audit documented in a `LICENSES.md` or the README, or install `cargo-deny 0.20.2` per `cargo search` this session |
| `jq` | Existing `scripts/prove-lint-wall.sh` (per its own comment) | Not directly probed this session, but the script's own comment states "the ubuntu runner image already holds" it, so CI availability is assumed, not local | — | CI-only reliance is already the existing pattern; no new risk from Phase 6 |
| VB6 IDE on Windows | Full recompilation (explicitly NOT in scope this phase per named risk) | No, and the phase's own goal states it will not be tested | — | None needed — the phase explicitly documents this gap rather than closing it |

**Missing dependencies with no fallback:** none — every gap above has a
documented fallback or is explicitly out of scope by the phase's own goal.

**Missing dependencies with fallback:**
- `cargo-deny`: manual licence audit is a viable fallback if the user prefers
  not to add a new dev-dependency for a v1.0 release.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust's built-in `#[test]` harness (via `cargo test`), no external test framework |
| Config file | None — tests are plain `#[test]` functions under `crates/deform6/tests/*.rs` and `crates/deform6-cli/tests/cli.rs` |
| Quick run command | `cargo test --workspace` (measured this session at well under 1 second wall time per file; full workspace run, release profile, completed in a few seconds) |
| Full suite command | `cargo test --workspace --release` (same command; this project has no separate "quick" vs "full" tier) |

### Phase Requirements -> Test Map

Phase 6 owns no new requirement ID. The table below maps the phase's five
success criteria to concrete, already-existing (or needed) checks.

| Criterion | Behavior | Test Type | Automated Command | File Exists? |
|-----------|----------|-----------|--------------------|--------------|
| SC1 (extract completes, structural check, schema, ratios) | 44/44 extract + structural + pinned ratios | integration | `cargo test --workspace -- extract_structural::the_structural_check_passes_for_all_forty_four_corpus_programs ratios::the_gate_passes_on_the_committed_file ratios::the_forms_and_controls_gate_passes_on_the_committed_file ratios::the_properties_gate_passes_on_the_committed_file corpus_sweep::all_forty_four_corpus_executables_read_and_report_what_they_hold` | Yes, all five, verified present this session |
| SC1 (schema validation) | Report JSON validates against a schema | integration | Not yet defined — needs a new test, e.g. `crates/deform6/tests/schema.rs` | No — Wave 0 gap |
| SC2 (no forbidden claim in claim surface) | grep over README/--help/report vocabulary | acceptance script | `sh scripts/check-claim-surface.sh` (new, modeled on `scripts/prove-lint-wall.sh`) | No — Wave 0 gap |
| SC3 (README states the three facts) | Manual/grep check for three specific sentences | acceptance script or manual UAT | Could extend `check-claim-surface.sh`, or a dedicated `scripts/check-readme-facts.sh` | No — Wave 0 gap |
| SC4 (README lists every open STRUCTURES/FILE-FORMATS gap) | Cross-reference README against the two gap registers | manual UAT (structural cross-reference is not easily automatable without brittle string matching against prose) | Manual review against `.planning/research/STRUCTURES.md` section 11 and `.planning/research/FILE-FORMATS.md` section 9 | N/A — inherently a document review |
| SC5 (fmt/clippy/test/doc/tag) | Full gate plus doc warnings plus tag check | integration + acceptance script | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`, then `cargo doc --no-deps` (warning count must be 0), then a tag/version match check | fmt/clippy/test: yes, already in `gate.yml`. `cargo doc --no-deps`: not in `gate.yml` today — Wave 0 gap. Tag/version check: no automation exists — Wave 0 gap |

### Sampling Rate

- **Per task commit:** `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` (the existing three-command gate; unchanged by this phase).
- **Per wave merge:** Full suite plus the new `check-claim-surface.sh` and (once written) `cargo doc --no-deps --workspace` with a zero-warning assertion.
- **Phase gate:** All five success criteria measured directly, not sampled — this is a release phase, and every corpus program, every doc-comment warning, and the whole claim surface must be checked, not a subset.

### Wave 0 Gaps

- [ ] `crates/deform6/schema/report.schema.json` (or equivalent) — the schema success criterion 1 asks to validate against; does not exist yet
- [ ] `crates/deform6/tests/schema.rs` (or a task inside `extract_structural.rs`) — validates all 44 produced reports against the schema
- [ ] `scripts/check-claim-surface.sh` — implements success criterion 2's grep, in the existing `scripts/prove-*-wall.sh` style; wire into `gate.yml`
- [ ] A `cargo doc --no-deps` step added to `.github/workflows/gate.yml`, gated on zero warnings, after the 49 current warnings are fixed
- [ ] `README.md` itself — does not exist; the whole file is new (plan 06-01)
- [ ] `CHANGELOG.md` — does not exist (plan 06-05); no format precedent in this repository to follow, a plain reverse-chronological Markdown list is a reasonable default
- [ ] A tag-vs-`Cargo.toml`-version check — no automation exists; could be a one-line script (`grep '^version' Cargo.toml` compared against `git describe --tags`) or a manual release-checklist step

*(Framework install: none needed — `cargo test` is already the project's only test runner.)*

## Security Domain

`security_enforcement` is `true` and `security_asvs_level` is `1` in
`.planning/config.json` `[VERIFIED: .planning/config.json, read this
session]`. DeForm6 is a local CLI/library with no network listener, no
authentication, and no session state; most ASVS categories do not apply. The
one category that applies fully is V5 (input validation), and Phase 5 already
delivered its controls; Phase 6 re-verifies rather than adds.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | No | No login, no credentials anywhere in this tool |
| V3 Session Management | No | No session state; each run is a single process over one file |
| V4 Access Control | No | Single-user local CLI; no authorization boundary |
| V5 Input Validation | Yes — already implemented, re-verify only | Bounded `Region` reader type with no infallible accessor, `checked_add` for every file-derived offset arithmetic, `#![forbid(unsafe_code)]`, the clippy deny wall (`indexing_slicing`, `arithmetic_side_effects`, `unwrap_used`, `expect_used`, `panic`, `cast_*` lints) `[VERIFIED: Cargo.toml workspace.lints.clippy table, read this session]` |
| V6 Cryptography | Partial — `xtask` only | `sha2 = "=0.11.0"` pinned exact version for corpus manifest hash verification `[VERIFIED: crates/xtask/Cargo.toml, read this session]`; not used by the `deform6` library itself |
| V12 File Handling | Yes — already implemented, re-verify only | `extract`'s output-directory containment check refuses the whole run if a lexically normalized candidate path's parent is not the resolved output directory: `if parent != resolved_dir { return Err(format!("{:?} would write outside {}, at {}; refusing the whole run", file.name, resolved_dir.display(), candidate.display())); }` `[VERIFIED: crates/deform6-cli/src/main.rs, fn plan_writes, lines 469-493, read this session]`. A second, adjacent control (a symlink pre-plant check, same file, lines ~495-508 per its own doc comment) additionally refuses when a symbolic link already sits at a planned path, closing the `--force` overwrite-through-a-symlink case `[VERIFIED: crates/deform6-cli/src/main.rs, lines 495-508, read this session]` |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| Integer overflow in offset+length arithmetic from an untrusted binary | Tampering | `checked_add` mandated by AGENTS.md and enforced by `clippy::arithmetic_side_effects = "deny"` `[VERIFIED: Cargo.toml, read this session]` |
| Allocation sized from an attacker-controlled length field (memory exhaustion DoS) | Denial of Service | Every count/length is checked against real file size before use; SAF-04, proved in Phase 5's `GuiTable::walk` bound check (measured: a 4096-byte image declaring `wFormCount = 0xFFFF` allocates nothing and yields exactly one `ImplausibleCount` defect) `[CITED: STATE.md decision log, Phase 5, and 05-UAT.md test 4, read this session]` |
| Path traversal via a recovered/sanitized file name escaping the output directory | Tampering / Elevation of Privilege | `SafeName` clamping plus the `extract` containment check (see V12 row above) |
| Panic-as-DoS on malformed input | Denial of Service | `#[forbid(unsafe_code)]`, no `.unwrap()`/`.expect()`/`panic!` on file-derived data (clippy-denied), and the fuzzer/regression-test gate (Phase 5, SAF-05) |
| Quadratic-time algorithmic complexity DoS from a crafted collision-heavy input | Denial of Service | Already found and fixed in Phase 5 (`PathIssuer`/`SafeNameIssuer` O(n^2) fix, one mutated input drove a sub-second test to 130+ seconds before the fix) `[CITED: 05-LEARNINGS.md "Surprises" section, read this session]` — Phase 6 should re-run the fuzz smoke test as part of its re-verification, not re-derive the fix |

Phase 6 introduces no new attack surface (no new parser, no new network
call, no new file write path). Its own security-relevant task is making sure
the README and `--help` text do not *overclaim* what the input-validation
work actually proved — an honesty concern, not a new control.

## Sources

### Primary (HIGH confidence — read/run directly this session)

- `Cargo.toml`, `crates/*/Cargo.toml` — workspace structure, dependency versions, lint configuration
- `crates/deform6/src/report.rs` — `ProjectReport`, `ReportItem`, `Confidence`, `Evidence`, `build_limits`
- `crates/deform6-cli/src/main.rs` — CLI shell, exit codes, `--help` doc comments
- `crates/deform6/tests/ratios.toml`, `crates/deform6/tests/differential.rs`, `crates/deform6/tests/extract_structural.rs`, `crates/deform6/tests/corpus_sweep.rs` — test entry points and pinned counts
- `.planning/research/STRUCTURES.md` section 11 (gap register), `.planning/research/FILE-FORMATS.md` section 9 (gap register)
- `.planning/WINDOWS.md` — broken-windows ledger (12 open, 4 fixed, 16 total)
- `.planning/phases/05-hostility/05-LEARNINGS.md`, `05-UAT.md` — Phase 5 handoff
- `.gitattributes`, `AGENTS.md`, `.planning/config.json` — project constraints
- Live commands run this session: `cargo build --workspace`, `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --release`, `cargo doc --no-deps --workspace`, `./target/debug/deform6 --help`/`inspect --help`/`extract --help`, `./target/debug/deform6 extract ... --force` (live report.json), `cargo search cargo-deny`, `git tag -l`, corpus/test-count greps

### Secondary (MEDIUM confidence)

- None consulted this session — no web search was needed; every finding for
  this phase was answerable from the repository itself, per the research
  focus directive to prioritize measurement over external lookup.

### Tertiary (LOW confidence)

- `cargo-deny`'s exact download counts, maintainer identity, and full
  legitimacy signal set were not independently checked beyond `cargo
  search`'s one-line listing; treat that recommendation as `[ASSUMED]`.

## Metadata

**Confidence breakdown:**
- Standard stack (no new deps): HIGH — nothing new to adopt except an optional dev-tool
- Claim surface (README/help/report vocabulary): HIGH — every claim traced to a read file or a live run
- Gap inventory (STRUCTURES §11 / FILE-FORMATS §9): HIGH — both registers read in full this session, quoted verbatim
- Release mechanics (version, changelog, licence, tag): MEDIUM — current state is fully measured (HIGH), but the *target* state (what version string, what changelog format) is a user decision this research flags rather than resolves
- Security domain: HIGH for what already exists (re-verification only); this phase adds no new attack surface

**Research date:** 2026-09-14
**Valid until:** Short — 7 days. This research is a snapshot of an actively-executing codebase; any commit to `main`/the phase branch before planning starts could change the doc-warning count, the test count, or the corpus count. Re-run the measurement commands in this file before planning if more than a few days have passed.
