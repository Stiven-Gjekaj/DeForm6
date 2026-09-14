---
phase: 06-version-1-0
plan: 01
subsystem: testing
tags: [json-schema, jsonschema-crate, report, corpus-sweep]

requires:
  - phase: 04-report
    provides: "ProjectReport, ReportItem, Confidence, Evidence in crates/deform6/src/report.rs, and Defect/Site/DefectKind in crates/deform6/src/error.rs"
provides:
  - "crates/deform6/schema/report.schema.json: a committed JSON Schema (draft 2020-12) for the report deform6::write::project writes"
  - "crates/deform6/tests/schema.rs: proves all 44 corpus programs produce a report that validates, and that the schema refuses a doctored one"
  - "the jsonschema crate as a deform6 dev-dependency, default-features off, no network or TLS code enters the build"
affects: [06-02, 06-03, 06-07]

actuals:
  tokens: 5746
  tasks: 3
  commits: 2

tech-stack:
  added: ["jsonschema 0.56.0 (dev-dependency, default-features = false)"]
  patterns:
    - "A JSON Schema file is a second source of truth, read from disk at test time via CARGO_MANIFEST_DIR, never rebuilt from the Rust types."
    - "A test that doctors a real, passing value into several invalid shapes and asserts each is refused, to prove the positive test is not agreeing with itself."

key-files:
  created:
    - crates/deform6/schema/report.schema.json
    - crates/deform6/tests/schema.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/deform6/Cargo.toml

key-decisions:
  - "jsonschema 0.56.0 approved at the Task 1 checkpoint after confirming the crates.io repository link, download history, and MIT licence."
  - "Integer bounds for defect-kind payload fields follow the u32 pattern the plan states (minimum 0, maximum 4294967295) extended to the field's own Rust width: u8 (0..255), u16 (0..65535), u64 (0..18446744073709551615), and i32 (-2147483648..2147483647) for the one signed field, GuidLengthUnexpected.value."
  - "Fast_Flames.exe was chosen as the base report for the doctored-report test because an existing test (salvage.rs) already establishes, unpatched, that it raises at least one defect in strict mode; the doctored-kind case needs a real defect to rename."
  - "Before committing, the doctored-report test was proved able to fail: additionalProperties was temporarily relaxed on the root schema (not committed), the extra-top-level-key case then failed as expected, and the schema was restored byte-for-byte from a backup."

patterns-established:
  - "crates/deform6/tests/schema.rs is the model for any future schema-shaped test: compile the validator once from disk, walk the corpus for the sweep, and keep the doctoring test's four cases wired to a real report, not a hand-built value."

requirements-completed: [RPT-01, RPT-02, RPT-03, RPT-04, RPT-05, RPT-06]

coverage:
  - id: D1
    description: "A committed JSON Schema file describes the report, read from disk, not derived from the Rust types at run time."
    requirement: "RPT-01"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/schema.rs#one_corpus_report_validates_against_the_committed_schema"
        status: pass
    human_judgment: false
  - id: D2
    description: "All 44 corpus programs produce a report that validates against the committed schema."
    requirement: "RPT-02"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/schema.rs#every_corpus_report_validates_against_the_committed_schema"
        status: pass
    human_judgment: false
  - id: D3
    description: "The schema admits exactly the three lower case confidence words and refuses a fourth."
    requirement: "RPT-03"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/schema.rs#the_schema_refuses_a_doctored_report"
        status: pass
    human_judgment: false
  - id: D4
    description: "A doctored report fails validation: a bad confidence word, a missing limits key, an extra top-level key, and a defect kind renamed to an unknown variant."
    requirement: "RPT-05"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/schema.rs#the_schema_refuses_a_doctored_report"
        status: pass
    human_judgment: false
  - id: D5
    description: "The schema names every defect kind the library can emit, and the count matches the enum in crates/deform6/src/error.rs (17)."
    requirement: "RPT-05"
    verification:
      - kind: unit
        ref: "shell: jq '.[\"$defs\"].defectKind.oneOf | length' crates/deform6/schema/report.schema.json"
        status: pass
    human_judgment: false
  - id: D6
    description: "The jsonschema dev-dependency was approved by a human before it entered the build."
    requirement: "RPT-04"
    verification: []
    human_judgment: true
    rationale: "Task 1 is a blocking-human checkpoint by design; the approval itself is a human decision, not something a test can assert."

duration: 45min
completed: 2026-09-14
status: complete
---

# Phase 6 Plan 1: Report Schema Summary

**A draft 2020-12 JSON Schema for `report.json`, read from disk and proved against all 44 corpus programs, with a doctored-report test that shows the schema can refuse a shape.**

## Performance

- **Duration:** ~45 min (continuation from the Task 1 checkpoint)
- **Completed:** 2026-09-14T17:57:04Z
- **Tasks:** 3
- **Files modified:** 5 (2 created, 3 modified)

## Accomplishments

- `jsonschema` 0.56.0 approved and added as a `deform6` dev-dependency, `default-features = false`, confirmed to pull in no `reqwest` or TLS stack of its own (the `rustls` entries `Cargo.lock` already carries come from `xtask`'s pre-existing `ureq` dependency, unrelated to this change).
- `crates/deform6/schema/report.schema.json` written from `crates/deform6/src/report.rs` and `crates/deform6/src/error.rs`: three required top-level keys, the three-word `confidence` enum, and a `oneOf` over all seventeen `DefectKind` variants with each variant's own payload fields and types.
- `crates/deform6/tests/schema.rs` holds three tests: the tracer (one program, `PassGen.exe`), the full sweep (all 44 corpus programs, count asserted), and the doctored-report proof (four separate invalid shapes, each asserted refused).
- Measured this run: the corpus produces defects of exactly 2 distinct kinds out of the 17 possible (`StructureUnreadable`, `UnreadablePointer`), out of 429 total defect instances (`severity_census.rs`'s own figure). Recorded here so plan 06-07 can state this as a measured number.

## Task Commits

1. **Task 1: Confirm the jsonschema package is the crate it claims to be** - checkpoint, human approved, no separate commit (nothing installs until Task 2)
2. **Task 2: One corpus program, end to end, from bytes to a schema-validated report** - `cba14db` (feat)
3. **Task 3: All 44 programs, and a schema that refuses a doctored report** - `39849c1` (feat)

**Plan metadata:** committed separately, see below.

## Files Created/Modified

- `crates/deform6/schema/report.schema.json` - the committed JSON Schema for `report.json`
- `crates/deform6/tests/schema.rs` - three tests: tracer, full sweep, doctored-report proof
- `Cargo.toml` - adds `jsonschema = { version = "0.56.0", default-features = false }` to `[workspace.dependencies]`
- `crates/deform6/Cargo.toml` - adds `[dev-dependencies]` with `jsonschema.workspace = true`
- `Cargo.lock` - records the resolved dependency tree for `jsonschema` 0.56.0 and its transitive crates

## Decisions Made

- **jsonschema 0.56.0 approved.** The Task 1 checkpoint confirmed the crates.io page: the repository link, the published version, download history, and the MIT licence. Approval was already recorded before this continuation agent started; this agent verified the plan's own measured facts (`cargo add --dry-run` feature list) still held and proceeded.
- **Integer bounds extended past the plan's explicit u32 example.** The plan states the pattern for `u32` fields (`minimum` 0, `maximum` 4294967295) but does not spell out bounds for the narrower or wider integer fields `DefectKind` payloads also carry. This plan applied the same pattern at each field's own Rust width: `u8` (`IndexHighByteSet.high`), `u16` (`UnrecoverableString.declared_len`), `u64` (`PastEndOfFile.file_len`), and the one signed field, `i32` (`GuidLengthUnexpected.value`, bounded `-2147483648..2147483647`). This is a direct extension of the plan's own instruction, not a deviation from it.
- **Fast_Flames.exe chosen for the doctored-report test's base.** The fourth doctored case needs a program with at least one real defect. `crates/deform6/tests/salvage.rs` already establishes, unpatched, that `Fast_Flames.exe` raises at least one `Tolerated` defect in strict mode, so it was reused rather than probed for fresh.
- **The doctored-report test was proved able to fail before it was committed.** `additionalProperties` was temporarily relaxed to `true` on the root schema (never committed), which made the extra-top-level-key case fail as expected; the schema was then restored from a backup and the diff checked clean. This follows `AGENTS.md`'s own rule: "When you add a test, break the thing it covers on purpose and watch it fail."

## Deviations from Plan

None - plan executed exactly as written. The two integer-bound extensions and the base-program choice above are completions of underspecified detail the plan explicitly asked the executor to derive from `crates/deform6/src/error.rs`, not departures from the plan's instructions.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The schema and its test file are the pattern plan 06-07's own edge-probe ledger and honesty audit can build on.
- `RPT-06` (the apostrophe-comment requirement) stays an unclassified, manually-reviewed item per the plan's own "Flagged assumptions" table; it is not resolved by this plan and is carried forward to 06-07 as the plan states.
- The measured defect-kind count (2 of 17 kinds observed in the corpus) is recorded above for plan 06-07 to quote directly rather than re-deriving it.

---
*Phase: 06-version-1-0*
*Completed: 2026-09-14*

## Self-Check: PASSED

- `crates/deform6/schema/report.schema.json` found on disk.
- `crates/deform6/tests/schema.rs` found on disk.
- `.planning/phases/06-version-1-0/06-01-SUMMARY.md` found on disk.
- Commits `cba14db` and `39849c1` found in `git log --oneline --all`.
- `cargo test -p deform6 --test schema` re-run: 3 passed, 0 failed.
- Root `required` array holds exactly 3 names (measured with `jq`).
- `$defs.defectKind.oneOf` holds exactly 17 branches (measured with `jq`).
