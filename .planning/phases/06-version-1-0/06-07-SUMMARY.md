---
phase: 06-version-1-0
plan: 07
subsystem: release
tags: [readme, honesty-audit, release-tag, acceptance-run]

# Dependency graph
requires:
  - phase: 06-version-1-0
    provides: "06-01 the report schema, 06-02 the README with five deferred markers, 06-03 zero doc warnings, 06-04 the confirmed 1.0.0/v1.0.0 strings and check-release-version.sh, 06-05 LICENSES.md and CHANGELOG.md, 06-06 the claim surface check and its audited allow list"
provides:
  - "README.md with all five deferred number markers filled from a real acceptance run: 44 corpus programs, 185 of 904 procedures, 52 of 53 forms, 686 of 686 controls, 807 property records against 136 written lines"
  - "A corrected S-05 limit row (sixteen undocumented type code positions, not ten), found and fixed by the honesty audit"
  - "The v1.0.0 annotated tag, created locally on HEAD, matching Cargo.toml, pushed nowhere"
affects: []

# Actuals (#2632)
actuals:
  tokens: 1065
  tasks: 3
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A README numeric claim states both the recovered and the declared count as a pair, names the gate test that asserts the pair, and never stands alone as a single derived figure."

key-files:
  created: []
  modified:
    - README.md

key-decisions:
  - "Each of the five README number markers was filled with the exact pair tests/ratios.toml asserts (recovered/declared, or record/written-line), never a single derived percentage, per AGENTS.md's own measurement rule and the plan's own acceptance criterion."
  - "The honesty audit found one real defect: S-05 stated the undocumented VB6 type code count as ten. STRUCTURES.md section 11's gap register (row 5) and crates/deform6/src/vb/functyp.rs's own vb_type_of match table both give sixteen undocumented positions (0x00-0x02, 0x04, 0x07, 0x09, 0x0E, 0x11, 0x12, 0x14-0x1A). Fixed as a Rule 1 auto-fix, in its own commit, separate from the task 1 number-filling commit."
  - "The v1.0.0 tag was created on the commit that holds the corrected, green task 2 measurement, not on the task 1 commit, so the tagged commit is the one every acceptance check in this plan was proven against."

patterns-established: []

requirements-completed: [DET-01, DET-02, DET-03, DET-06, FRM-06, SAF-01, SAF-04, SAF-05, VER-01, VER-02, VER-03, VER-04, VER-05, WRT-02, WRT-04, WRT-07]

coverage:
  - id: D1
    description: "The acceptance run over all 44 corpus programs (corpus_sweep, extract_structural, schema, ratios, differential) passes, and README.md's five deferred number markers are filled with the exact pairs those tests assert."
    requirement: "VER-01"
    verification:
      - kind: integration
        ref: "cargo test -p deform6 --test corpus_sweep (2 passed)"
        status: pass
      - kind: integration
        ref: "cargo test -p deform6 --test extract_structural (25 passed)"
        status: pass
      - kind: integration
        ref: "cargo test -p deform6 --test schema (3 passed)"
        status: pass
      - kind: integration
        ref: "cargo test -p deform6 --test ratios (47 passed)"
        status: pass
      - kind: integration
        ref: "cargo test -p deform6 --test differential (29 passed)"
        status: pass
      - kind: other
        ref: "shell: grep -c 'measured:' README.md (=0); grep -q 44/52/686 README.md; LC_ALL=C tr -d printable-ascii < README.md | wc -c (=0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The honesty audit: the three gate commands, cargo doc, the claim surface check, the licence re-derivation, all six hostile-input test files, and the six named edge tests all pass, and every README capability sentence traces to a measured result or a cited fact. One defect (S-05's wrong count) was found and fixed."
    requirement: "SAF-01"
    verification:
      - kind: integration
        ref: "cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace (23 test binaries, 0 failed)"
        status: pass
      - kind: integration
        ref: "RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --workspace"
        status: pass
      - kind: other
        ref: "sh scripts/check-claim-surface.sh (exit 0, 18 of 18 planted violations fired, 3 of 3 fact probes fired)"
        status: pass
      - kind: other
        ref: "cargo run -q -p xtask -- licences && git diff --exit-code -- LICENSES.md"
        status: pass
      - kind: integration
        ref: "cargo test -p deform6 --test fuzz_smoke (5 passed), no_panic_proof (9 passed), regressions (18 passed), salvage (8 passed), refusal (9 passed), severity_census (1 passed)"
        status: pass
      - kind: unit
        ref: "cargo test -p deform6 --lib two_names_that_clamp_to_the_same_forty_bytes_are_made_distinct_by_the_issuer, a_thousand_controls_with_the_same_raw_name_each_get_a_distinct_name, two_objects_whose_raw_names_collide_name_two_different_files, gui_table_refuses_an_implausible_form_count; --test salvage -- two_salvage_runs_over_the_same_bytes_give_byte_identical_json; --test ratios -- editing_only_the_ratio_fails_because_it_disagrees_with_its_own_counts (each 1 passed)"
        status: pass
      - kind: manual_procedural
        ref: "Read STRUCTURES.md section 11 and FILE-FORMATS.md section 9 against README.md's 26 limit rows, with code-level spot checks in classify.rs, header.rs, gui.rs, ocx.rs, functyp.rs and write/values.rs. One row (S-05) was wrong and was corrected."
        status: pass
    human_judgment: false
  - id: D3
    description: "The v1.0.0 annotated tag exists locally on the commit holding the green task 2 measurement, matches Cargo.toml, carries no author/tool/session marker, and reached no remote."
    requirement: "WRT-07"
    verification:
      - kind: other
        ref: "git tag -l v1.0.0; git cat-file -t v1.0.0 (= tag); sh scripts/check-release-version.sh; git tag -l --format='%(contents)' v1.0.0 | grep -ciE co-authored-by|generated|claude|assistant (=0); git ls-remote --tags origin | grep -c refs/tags/v1.0.0 (=0)"
        status: pass
    human_judgment: false

duration: 35min
completed: 2026-09-14
status: complete
---

# Phase 6 Plan 7: The Acceptance Run, the Honesty Audit, and the v1.0.0 Tag Summary

**Fills README.md's five deferred numbers from a real 44-program acceptance run (185 of 904 procedures, 52 of 53 forms, 686 of 686 controls, 807 property records against 136 written lines), finds and fixes one factual error in the limit tables during the honesty audit, and creates the local, unpushed `v1.0.0` tag on the commit holding the green result.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-09-14
- **Tasks:** 3
- **Files modified:** 1 (`README.md`)

## Accomplishments

- Ran all five success-criterion-1 checks over all 44 corpus programs: `corpus_sweep` (2 passed), `extract_structural` (25 passed), `schema` (3 passed), `ratios` (47 passed), `differential` (29 passed). All five green on the first run.
- Filled README.md's five deferred markers with the exact pairs `tests/ratios.toml` asserts, never a single derived figure: 44 corpus programs; 185 of 904 procedure signatures; 52 of 53 forms, with a sentence naming the one refusing form as a designed refusal; 686 of 686 controls; 807 property records against 136 written lines, with a sentence explaining why the written count can exceed the record count. Each pair names the gate test that asserts it.
- Ran the honesty audit's full measurement set: the three gate commands, `cargo doc --no-deps` at zero warnings, `sh scripts/check-claim-surface.sh` (18 of 18 planted violations fired, 3 of 3 fact probes fired), the licence re-derivation (clean diff), all six hostile-input test files, and the six named edge tests, each matched by name and each reporting at least one passing test.
- Read `STRUCTURES.md` section 11 and `FILE-FORMATS.md` section 9 against every row of README.md's two limit tables, with a direct code read for a sample of rows (S-02, S-05, S-16, S-17, S-18, F-01, F-10) to confirm the stated default matches what the code actually does. Found one real defect: S-05 stated the undocumented VB6 type code count as ten; the register and `functyp.rs`'s own match table both give sixteen. Fixed in its own commit.
- Created the annotated `v1.0.0` tag locally, on the commit holding the corrected measurement, matching `Cargo.toml`'s `1.0.0`, with a two-sentence message naming no author, tool or session. Confirmed no remote holds it.

## Task Commits

1. **Task 1: The acceptance run over all 44, and the numbers it reported** - `d88cdcb` (feat)
2. **Task 2: The honesty audit** - `8f1dae4` (fix: the one defect the audit found, S-05's wrong count)
3. **Task 3: The v1.0.0 tag, created locally and pushed nowhere** - no file commit; the annotated tag `v1.0.0` itself, on `8f1dae4`

**Plan metadata:** committed separately, see below.

## Files Created/Modified

- `README.md` - the five deferred number markers filled from the acceptance run, and the S-05 limit row corrected from "ten" to the accurate "sixteen" undocumented type code positions

## Decisions Made

See `key-decisions` in the frontmatter above: the pair-of-counts discipline for every filled number, the S-05 fix as a Rule 1 auto-fix in its own commit, and tagging the corrected commit rather than the first task's commit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected the S-05 limit row's wrong count**
- **Found during:** Task 2, while cross-reading `STRUCTURES.md` section 11 against README.md's limit table
- **Issue:** S-05 read "What Visual Basic type ten of the documented type codes name," but the gap register's row 5 lists sixteen undocumented type code positions (`0x00`-`0x02`, `0x04`, `0x07`, `0x09`, `0x0E`, `0x11`, `0x12`, `0x14`-`0x1A`), matching `crates/deform6/src/vb/functyp.rs`'s own `vb_type_of` match table (fifteen documented codes out of the mask's range, the rest falling to `Unknown`). The sentence was both grammatically garbled and numerically wrong.
- **Fix:** Rewrote the row to name the count as sixteen and list the sixteen positions directly, so a reader can check the claim against the register without re-deriving it.
- **Files modified:** `README.md`
- **Verification:** `sh scripts/check-claim-surface.sh` re-run clean after the fix (exit 0); the row's "what the tool does instead" cell was already accurate and needed no change.
- **Committed in:** `8f1dae4`

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Required for success criterion 4 (README limit tables must match the two gap registers). No scope change; this is exactly the honesty audit's stated job.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 6 is now fully executed: all seven plans (06-01 through 06-07) have summaries.
- `README.md` holds zero deferred markers, every number is a pair of counts, and every limit row was checked against its register during this plan.
- The `v1.0.0` tag exists locally on commit `8f1dae4`, matches `Cargo.toml`, and reached no remote. To publish it, the human runs: `git push origin v1.0.0`. This plan did not run that command.
- No blockers. `/gsd-verify-work 6` is the next step.

---
*Phase: 06-version-1-0*
*Completed: 2026-09-14*

## Self-Check: PASSED

- `README.md` found on disk, holds no `measured:` marker.
- Commits `d88cdcb` and `8f1dae4` found in `git log --oneline --all`.
- Tag `v1.0.0` found via `git tag -l v1.0.0`, `git cat-file -t v1.0.0` reports `tag`.
- `sh scripts/check-release-version.sh` re-run: `PASS tag v1.0.0 matches Cargo.toml version 1.0.0`.
- Re-ran the plan's full acceptance-criteria set for all three tasks: all pass.
- `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` (23 test binaries, 0 failed) all re-run clean.
- `commits: 2` measured via `git rev-list --count` against the ledger recorded before task 1's commit; matches the two task commits above (task 3 created a tag, not a file commit, per its own `<files>` note).
