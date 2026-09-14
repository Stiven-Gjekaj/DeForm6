---
phase: 06-version-1-0
verified: 2026-09-14T21:00:00Z
status: passed
score: 5/5 roadmap success criteria verified
behavior_unverified: 0
overrides_applied: 0
covered_files:
  - ".github/workflows/gate.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/phases/06-version-1-0/06-01-PLAN.md"
  - ".planning/phases/06-version-1-0/06-01-SUMMARY.md"
  - ".planning/phases/06-version-1-0/06-02-PLAN.md"
  - ".planning/phases/06-version-1-0/06-02-SUMMARY.md"
  - ".planning/phases/06-version-1-0/06-03-PLAN.md"
  - ".planning/phases/06-version-1-0/06-03-SUMMARY.md"
  - ".planning/phases/06-version-1-0/06-04-PLAN.md"
  - ".planning/phases/06-version-1-0/06-04-SUMMARY.md"
  - ".planning/phases/06-version-1-0/06-05-PLAN.md"
  - ".planning/phases/06-version-1-0/06-05-SUMMARY.md"
  - ".planning/phases/06-version-1-0/06-06-PLAN.md"
  - ".planning/phases/06-version-1-0/06-06-SUMMARY.md"
  - ".planning/phases/06-version-1-0/06-07-PLAN.md"
  - ".planning/phases/06-version-1-0/06-07-SUMMARY.md"
  - ".planning/phases/06-version-1-0/06-EDGE-COVERAGE.json"
  - ".planning/phases/06-version-1-0/06-REVIEW.md"
  - "CHANGELOG.md"
  - "Cargo.toml"
  - "LICENSES.md"
  - "README.md"
  - "crates/deform6/schema/report.schema.json"
  - "crates/deform6/src/vb/controlinfo.rs"
  - "crates/deform6/src/vb/controltree.rs"
  - "crates/deform6/src/vb/header.rs"
  - "crates/deform6/src/vb/object.rs"
  - "crates/deform6/src/vb/ocx.rs"
  - "crates/deform6/src/vb/opcodes.rs"
  - "crates/deform6/src/vb/privateobj.rs"
  - "crates/deform6/src/vb/project.rs"
  - "crates/deform6/src/vb/propstream.rs"
  - "crates/deform6/tests/gitattributes.rs"
  - "crates/deform6/tests/schema.rs"
  - "crates/xtask/src/licences.rs"
  - "scripts/check-claim-surface.sh"
  - "scripts/check-release-version.sh"
covered_digest: "v1:sha256:cd025346389b4514695355338eb30ecce95d2fe8dccac2c1f7285ca57e1dc3b1"
human_verification:
  - test: "Decide whether the v1.0.0 tag should move, or a v1.0.1/new tag should be cut, before the tag is treated as the released artifact."
    result: resolved
    resolution: "The human chose to move the tag. v1.0.0 now points at commit fd72e10, the last commit that changes shipped content and the commit that carries the CR-01 fix. The tag is annotated and local. It is not pushed."
    expected: "The tagged commit is the one a downstream consumer checks out. It should carry the release's own honesty gate at full strength."
    why_human: "The tag on HEAD (v1.0.0, commit 8f1dae4) sits 9 commits behind the current HEAD (da6f3a0). Those 9 commits include the fix for CR-01, the code review's one Critical finding: the claim-surface scanner could not see a claim that wraps across two lines, which is how this README's own prose wraps throughout. A person who clones the repository at the v1.0.0 tag gets the pre-fix scanner. This is not an automated-check failure (`scripts/check-release-version.sh v1.0.0` passes, because the tag does match the version string in Cargo.toml at the tagged commit), so it cannot fail a truth. It is a release-management decision the task instructions name as being handled separately from this report, but it must be seen before the tag is treated as final."
---

# Phase 6: Version 1.0 Verification Report

**Phase Goal:** The metadata deliverable is finished, measured and documented. A
person who reads the README learns exactly what DeForm6 returns and exactly
what it does not, before they run it. There is no code recovery in version 1.0
and no claim of it anywhere in the program, the help text or the documentation.

**Verified:** 2026-09-14T21:00:00Z
**Status:** passed
**Re-verification:** No: initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `deform6 extract` runs to completion on all 44 corpus programs, the structural check passes for all 44, the JSON report validates against the schema for all 44, and every pinned ratio holds unchanged | VERIFIED | `cargo test -p deform6 --test extract_structural` → `the_structural_check_passes_for_all_forty_four_corpus_programs ... ok` (asserts `count == 44`). `cargo test -p deform6 --test schema` → `every_corpus_report_validates_against_the_committed_schema ... ok` (asserts `count == 44`). `cargo run -q -p xtask -- update-ratios` produces no diff on `tests/ratios.toml` (44 entries, re-derived, unchanged). `cargo test --workspace` → all suites report `0 failed` (615+94+47+42+29+25+... passed across every crate). |
| 2 | A grep over README.md, the `--help` output and the report vocabulary finds no claim of statement recovery, no claim of compilable Basic from native code, and no percentage figure outside `recovery.ratio` | VERIFIED | `sh scripts/check-claim-surface.sh` exits 0 on the real tree ("The claim surface holds no claim the measurement does not support"). I independently re-proved the CR-01 fix (see below), not just accepted the REVIEW.md resolution table. `grep -n "%\|percent"` over `README.md` returns nothing. `deform6 --help` / `deform6 extract --help` hold no capability claim beyond "reads ... and reports what it holds" / "writes a ... project directory that VB6 can open". |
| 3 | The README states the three plain-word facts: P-code branch of `lpNativeCode` untested, `frmHMM.frx` damaged upstream and excluded by name, full recompilation not tested (needs VB6 on Windows) | VERIFIED | `grep -n "lpNativeCode\|frmHMM.frx\|needs the Visual Basic 6 IDE"` over `README.md` finds all three (lines 66-86). The script's own FACTPASS probes (removing each sentence and requiring detection) all fire: `FACTPASS lpNativeCode`, `FACTPASS frmHMM.frx`, `FACTPASS needs the Visual Basic 6 IDE on`. |
| 4 | The README lists every still-open gap from STRUCTURES section 11 and FILE-FORMATS section 9, with the safe default chosen for each | VERIFIED | STRUCTURES.md section 11 is a 19-row register; rows 1, 11 and 19 are marked `CLOSED`, leaving 16 open (rows 2-10, 12-18). README.md holds exactly 16 `S-NN` rows (`S-02` through `S-10`, `S-12` through `S-18`). FILE-FORMATS.md section 9 lists 10 unresolved items, none marked closed; README.md holds exactly 10 `F-NN` rows (`F-01` through `F-10`). I spot-checked three defaults directly against the source: S-05 ("reports the type as `Unknown` and carries the raw byte forward") matches `VbType::Unknown(other)` at `crates/deform6/src/vb/functyp.rs:673`; F-01 ("writes the string inline when it is 97 characters or shorter") matches `pub const MAX_INLINE_STRING_LEN: usize = 97` at `crates/deform6/src/write/model.rs:39`; S-13 ("assumes a Western code page... replaced with a question mark") matches the literal sentence built in `crates/deform6/src/report.rs:389-390`. |
| 5 | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` passes on a clean clone, `cargo doc --no-deps` produces no warning, and the tag matches the version in Cargo.toml | VERIFIED, with a flagged release-management observation | `cargo fmt --all --check` exit 0. `cargo clippy --all-targets -- -D warnings` exit 0, no warnings. `cargo test --workspace`: every suite reports `0 failed` (verified suite-by-suite, no failures, no panics). `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace` exit 0, no warnings. `sh scripts/check-release-version.sh v1.0.0` → `PASS tag v1.0.0 matches Cargo.toml version 1.0.0`, and `Cargo.toml`'s `[workspace.package]` states `version = "1.0.0"` with all three crates using `version.workspace = true`. **Observation, not a truth failure:** the `v1.0.0` tag sits on commit `8f1dae4`, and the branch HEAD (`da6f3a0`) is 9 commits ahead. Those 9 commits include the entire code-review resolution, most importantly the CR-01 fix to `scripts/check-claim-surface.sh` (the claim surface holds no protection against a wrapped claim at the tagged commit; `git show v1.0.0:scripts/check-claim-surface.sh \| grep -c join_source` returns 0, HEAD's version returns 8). See Human Verification below. |

**Score:** 5/5 roadmap success criteria verified (0 present-but-behavior-unverified)

### Critical Finding Independently Re-Verified (CR-01)

The task instructions required proving the claim-surface fix myself rather than trusting
REVIEW.md's resolution table. I did this against the real, committed `README.md`, not a copy:

```
$ md5sum README.md
cb1a1c816615b63b1d47ab2bdf02add6  README.md

$ printf '\nDeForm6 will, after more work in a future release, recover every\nstatement from the compiled executable directly and precisely.\n' >> README.md

$ sh scripts/check-claim-surface.sh; echo "exit: $?"
FAIL    readme  line 200-201   stmt-fwd       DeForm6 will, after more work in a future release, recover every statement from the compiled executable directly and precisely.
FAIL    readme  line 200-201   compilable     DeForm6 will, after more work in a future release, recover every statement from the compiled executable directly and precisely.
exit: 1

$ cp /tmp/readme-backup-verify.md README.md
$ diff /tmp/readme-backup-verify.md README.md && echo "IDENTICAL"
IDENTICAL
$ md5sum README.md
cb1a1c816615b63b1d47ab2bdf02add6  README.md
$ git status --short README.md
(clean)
```

The fix holds against a hand-planted wrapped claim on the real README, and the file was
restored byte-for-byte, proved by an identical md5 sum and a clean `git status`.

### Required Artifacts (all 7 plans)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/deform6/schema/report.schema.json` | Committed JSON Schema, second source of truth, names all 17 `DefectKind` variants, admits 3 confidence words | VERIFIED | Contains `"unrecoverable"`; `"enum": ["proven", "inferred", "unrecoverable"]`; all 17 variants from `error.rs`'s `DefectKind` (BadMagic, OffsetOverflow, ..., StructureUnreadable) confirmed present by name. |
| `crates/deform6/tests/schema.rs` | Validates all 44 corpus reports, proves a doctored report fails | VERIFIED | `every_corpus_report_validates_against_the_committed_schema` (count==44) and `the_schema_refuses_a_doctored_report` (asserts the undoctored value validates first, then doctors 3 distinct ways and asserts each is refused) both pass. |
| `README.md` | Released claim surface: what the tool returns/does not, confidence vocabulary, three facts, full gap list | VERIFIED | 198 lines, no debt markers, no `measured:` deferred markers left unfilled, 16 S-rows + 10 F-rows, three facts stated, no `%`/percent anywhere. |
| `.github/workflows/gate.yml` | `cargo doc` step denying warnings; claim-surface step | VERIFIED | `RUSTDOCFLAGS: -D warnings` at line 71, `run: sh scripts/check-claim-surface.sh` at line 122. |
| `scripts/check-release-version.sh` | Measures tag vs Cargo.toml, proved able to fail | VERIFIED | Ran directly: passes with `v1.0.0` explicit; the summary records the deliberate-mismatch proof, and a run with no argument on the current (untagged) HEAD correctly errors (`HEAD carries no tag`). |
| `crates/deform6/tests/gitattributes.rs` | Audits `.gitattributes` against the writer's real extension set, `.frx`/`.ctx` binary | VERIFIED | `cargo test -p deform6 --test gitattributes`: both tests pass; `.gitattributes` lines 9-10 mark `*.frx` and `*.ctx` binary. |
| `crates/xtask/src/licences.rs` / `LICENSES.md` | Re-derived licence audit, excludes workspace members | VERIFIED | Re-ran `cargo run -q -p xtask -- licences`; `git status --short LICENSES.md` and `git diff --stat LICENSES.md` both empty after the rerun: file is unchanged, proving it is re-derived, not typed. |
| `CHANGELOG.md` | One 1.0.0 section naming the four open requirements | VERIFIED | Names FRM-01, FRM-02, FRM-03 and WRT-03 by ID under "What stays open", with the exact reason for each. |
| `scripts/check-claim-surface.sh` | Implements success criterion 2, self-test, audited allow list | VERIFIED | Self-test reports 18/18 single-line and 15/15 wrapped planted violations firing; independently re-proved above; allow list holds 2 entries, both negations already present in README.md. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `crates/deform6/tests/schema.rs` | `crates/deform6/schema/report.schema.json` | reads from disk under `CARGO_MANIFEST_DIR` | VERIFIED | `compiled_schema()` reads `schema/report.schema.json` via `env!("CARGO_MANIFEST_DIR")`, never rebuilt from Rust types. |
| `README.md` | `.planning/research/STRUCTURES.md` | S-NN rows keyed to register rows | VERIFIED | 16 of 16 open register rows (2-10, 12-18) appear as S-02..S-18 in README. |
| `README.md` | `.planning/research/FILE-FORMATS.md` | F-NN rows keyed to section 9 items | VERIFIED | 10 of 10 open items appear as F-01..F-10 in README. |
| `.github/workflows/gate.yml` | `scripts/check-claim-surface.sh` | gate step | VERIFIED | Line 122. |
| `.github/workflows/gate.yml` | `crates/deform6/src` | `cargo doc` with `RUSTDOCFLAGS` denying warnings | VERIFIED | Line 69-72. |
| `scripts/check-release-version.sh` | `Cargo.toml` | reads `[workspace.package]` version | VERIFIED | Ran directly, matched 1.0.0. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Empty file refuses cleanly | `deform6 inspect /tmp/empty.exe` | `this file is not a portable executable`, exit 1 | PASS |
| One-byte file refuses cleanly | `deform6 inspect /tmp/onebyte.exe` | `this file is not a portable executable`, exit 1 | PASS |
| No-panic proof over every reachable input, both modes and the writer | `cargo test -p deform6 --test no_panic_proof` | `every_input_this_repository_can_reach_runs_through_both_modes_and_the_writer ... ok` | PASS |
| Claim-surface self-test | `sh scripts/check-claim-surface.sh` | exit 0, 18/18 + 15/15 planted probes fire, 3/3 fact probes fire | PASS |
| Claim-surface catches a hand-planted wrapped claim on the real README | see CR-01 re-verification above | exit 1, `FAIL` line naming the wrapped sentence | PASS |
| `--help` / `extract --help` hold no overclaim | `deform6 --help`, `deform6 extract --help` | plain descriptive text only, no percentage, no "compil*", no "statement... recover" | PASS |
| Licence audit is re-derived | `cargo run -q -p xtask -- licences` then `git diff --stat LICENSES.md` | no diff | PASS |
| Ratio file is re-derived | `cargo run -q -p xtask -- update-ratios` then `git diff --stat tests/ratios.toml` | no diff, 44 entries | PASS |
| `cargo fmt --all --check` | | exit 0 | PASS |
| `cargo clippy --all-targets -- -D warnings` | | exit 0, no warnings | PASS |
| `cargo test --workspace` | | every suite 0 failed | PASS |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace` | | exit 0, no warnings | PASS |

### Requirements Coverage

All 42 requirement IDs the phase re-verifies (DET-01..06, OBJ-01..06, FRM-01..06, WRT-01..07,
RPT-01..06, SAF-01..05, VER-01..06) are checked `[x]` in `REQUIREMENTS.md` except `WRT-03`,
which stays `[ ]` on purpose (row: `Open, unit proved only`). This matches roadmap success
criterion 3's own statement that full recompilation was never tested, and matches
`CHANGELOG.md`'s "What stays open" list, which names `WRT-03` explicitly. No requirement ID
declared in the phase's PLAN frontmatter is orphaned; each of the 7 plans' `requirements:`
fields was cross-checked against `REQUIREMENTS.md`'s own phase-mapping table (lines 200-217).

### Edge-Probe Coverage (06-EDGE-COVERAGE.json)

107 applicable edge rows. All 107 rows carry `status: unresolved` in the raw probe file itself
- this is expected, because the file is the un-annotated source list the plans lift `must_haves`
from, not a tracker the plans write back into. Cross-checking each plan's own must-haves
against the probe rows: 97 of 107 rows are addressed by name inside a plan's `must_haves.truths`
(the `encoding`, `concurrency`, `adjacency`, `ordering`, `empty`, `boundary`, `precision`
categories are each answered explicitly, listing every requirement ID the row raised). The
remaining 10 rows are all `unclassified` ("review manually") rows, and each of the 7 plans
accounts for its own `unclassified` rows by name in a dedicated section, with a stated reason
each stays a flagged assumption rather than a closed edge (e.g. RPT-01/RPT-06 in 06-01, OBJ-02
in 06-03, FRM-05/WRT-01/WRT-06 in 06-04, DET-02/SAF-01/VER-04/WRT-07 in 06-07). These 10 are
reported here as flagged assumptions, per the task's explicit instruction, not as gaps.

### Anti-Patterns Found

None. `grep -nE "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER"` over every file this phase modified
(schema, tests, README, gate.yml, licences.rs, LICENSES.md, CHANGELOG.md, both shell scripts,
gitattributes.rs) returns no matches. `grep -rniE "claude|anthropic|co-authored-by|generated by"`
over README.md, CHANGELOG.md and LICENSES.md returns no matches.

### Prohibitions (judgment-tier, flagged per contract)

Every plan's `must_haves.prohibitions` block is `status: resolved, verification: judgment`. I
spot-checked the ones with an independently checkable surface rather than accepting the
`resolved` marking at face value:

- "README.md must not state a recovery figure as a share of one hundred": checked directly,
  no `%`/percent token anywhere in README.md.
- "`LICENSES.md` must not state a licence `cargo metadata` does not state... an absent licence
  field is written as absent, never guessed": `licences::tests::render_never_names_a_workspace_member_and_writes_not_stated_for_an_absent_licence`
  passes in the workspace test run.
- "No released text names an agent as the author of a commit or a release": checked directly,
  no match in README.md, CHANGELOG.md or LICENSES.md.
- "The check must not be written so it passes because it searched the wrong place or an empty
  set": independently re-proved above (CR-01 re-verification): the check does fire on a real,
  planted violation in the real scanned file.

The remaining prohibitions (schema not relaxed to fit a report; `.gitattributes` not weakened;
version not implying a stability guarantee beyond the public API shape; changelog not
describing an open limit as closed) rest on narrative judgment about intent across a diff I did
not re-review commit-by-commit. They are not contradicted by anything I found, but I did not
independently re-derive each one and record them here as flagged rather than fully verified,
per the backstop contract.

## Human Verification Required

### 1. The v1.0.0 tag predates the code review's Critical fix

**Test:** Decide whether the `v1.0.0` tag should move to `HEAD`, or a new tag should be cut,
before the tag is treated as the released artifact.

**Expected:** The tagged commit is the one a downstream consumer checks out and audits. It
should carry the release's own claim-surface gate at full strength, not the version the code
review found broken.

**Why human:** `git tag -v v1.0.0` shows the tag sits on commit `8f1dae4`. The branch HEAD
(`da6f3a0`) is 9 commits ahead of that tag. Those 9 commits are exactly the code review's
resolution: `eb0835c`, `129e0e7`, `a91612c`, `7550a4e`, `fd72e10` (the five fix commits,
including CR-01, the one Critical finding) and `c80a47f`, `da6f3a0` (recording the review and
its resolution). `git show v1.0.0:scripts/check-claim-surface.sh | grep -c join_source` returns
`0`; the same grep against the working tree returns `8`. A clone at the tag gets the scanner
that cannot see a claim wrapped across two lines, in the exact prose style this repository's own
README uses throughout. `scripts/check-release-version.sh v1.0.0` still reports `PASS`, because
the tag genuinely matches the version string frozen into `Cargo.toml` at that commit: so this
is not a truth failure, it is a release-sequencing question the task's own instructions named as
a decision for the human, not a defect for this report to fix.

## Gaps Summary

No gap blocks the phase goal. All 5 roadmap success criteria hold with direct evidence, not
SUMMARY.md claims: every command named in criterion 5 was run in this session and its exit
code recorded; the CR-01 fix was independently re-proved against the live README rather than
trusted from REVIEW.md's resolution table; both gap-register counts (16 STRUCTURES rows, 10
FILE-FORMATS rows) were counted directly from the source registers and matched against README's
own S-NN/F-NN rows, with three of the chosen safe defaults spot-checked against the real code.
The one item that keeps this report at `human_needed` rather than `passed` is not a failed
truth: it is that the `v1.0.0` tag, as currently placed, ships a version of the release's own
honesty gate that the phase's own code review found and fixed nine commits later. That is a
release-sequencing decision, and the task instructions explicitly route it to the human rather
than to this report's pass/fail judgment.

---

_Verified: 2026-09-14T21:00:00Z_
_Verifier: Claude (gsd-verifier)_

---

## Resolution of the one human item

The verifier raised one item for a human: where the `v1.0.0` tag belongs, given that the fix
for CR-01 landed after the tag was first cut.

The human chose to move the tag, on 2026-09-14.

`v1.0.0` pointed at commit `fd72e10` when this report was written. That is the last commit in
this phase that changes shipped content, and it carries the CR-01 fix. The earlier tag pointed at `8f1dae4`, whose
copy of `scripts/check-claim-surface.sh` holds no `join_source` function and therefore cannot
see a claim that wraps across two lines.

Measured after the move:
- `git show v1.0.0:scripts/check-claim-surface.sh | grep -c join_source` returns 8. At the old
  tag it returns 0.
- `sh scripts/check-release-version.sh v1.0.0` passes, so criterion 5 still holds.
- `git ls-remote --tags origin v1.0.0` returns nothing. The tag is local. Nobody outside this
  repository referenced the old tag, so moving it breaks no consumer.

The commits after `fd72e10` change only files under `.planning/`. They are planning records.
They do not change what a person gets when they check out the tag.

With this item resolved, every success criterion holds and no item is left for a human.

### The tag moved again, after this report

On the same day, the retrospective review of Phase 5 found a Critical defect in `--salvage`:
one unresolvable `Declare` descriptor refused the whole file in both modes, because four
per-item sites reused a `Fatal` defect kind. See
`.planning/phases/05-hostility/05-REVIEW.md`. The user approved the fix, and it changes shipped
behaviour.

`v1.0.0` therefore moved a second time, to commit `5581fdf`, the last commit that changes
shipped content across both phases. The tag is still annotated and still local. Measured after
the move:

- `git show v1.0.0:scripts/check-claim-surface.sh | grep -c join_source` returns 8, so the
  CR-01 fix of this phase is in the tagged tree.
- `git show v1.0.0:crates/deform6/src/error.rs | grep -c ItemAddressUnmapped` returns 4, so the
  Phase 5 salvage fix is in the tagged tree.
- `git diff --name-only v1.0.0..HEAD` lists no file outside `.planning/`, so nothing shipped
  sits after the tag.
- `sh scripts/check-release-version.sh v1.0.0` passes, so criterion 5 still holds.
