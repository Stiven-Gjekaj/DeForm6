---
phase: 01-it-reads-the-file
verified: 2026-09-07T18:07:36Z
status: passed
score: 6/6 must-haves verified
covered_files: [".planning/phases/01-it-reads-the-file/01-01-PLAN.md", ".planning/phases/01-it-reads-the-file/01-01-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-02-PLAN.md", ".planning/phases/01-it-reads-the-file/01-02-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-03-PLAN.md", ".planning/phases/01-it-reads-the-file/01-03-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-04-PLAN.md", ".planning/phases/01-it-reads-the-file/01-04-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-05-PLAN.md", ".planning/phases/01-it-reads-the-file/01-05-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-06-PLAN.md", ".planning/phases/01-it-reads-the-file/01-06-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-07-PLAN.md", ".planning/phases/01-it-reads-the-file/01-07-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-08-PLAN.md", ".planning/phases/01-it-reads-the-file/01-08-SUMMARY.md", "Cargo.toml", "crates/deform6-cli/src/main.rs", "crates/deform6-cli/tests/cli.rs", "crates/deform6/src/error.rs", "crates/deform6/src/journal.rs", "crates/deform6/src/lib.rs", "crates/deform6/src/read/pe.rs", "crates/deform6/src/read/region.rs", "crates/deform6/src/vb/header.rs", "crates/deform6/src/vb/mod.rs", "crates/deform6/src/vb/project.rs", "crates/deform6/src/vb/runtime.rs", "crates/deform6/tests/corpus_sweep.rs", "crates/deform6/tests/refusal.rs"]
covered_digest: "v1:sha256:d0fe495a5a9816714532742e0f09221ebbf536bba82a9f949cb152eba4db920d"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 1: It reads the file Verification Report

**Phase Goal:** A person runs `deform6 inspect` on a compiled program and
learns whether DeForm6 can read it, what the project is called, and how it
was compiled. A file that is not a VB6 Standard EXE gets one clear sentence
and a non-zero exit code.

**Verified:** 2026-09-07T18:07:36Z
**Status:** passed
**Re-verification:** No — initial verification

## Method

Every claim below was checked by executing a real command against the real
tree, not by reading SUMMARY.md and accepting its narrative. Where the task
asked for mutation testing, the code was actually edited, the suite actually
run, the failure actually observed, and the file actually reverted and
diffed clean. `git status --porcelain` was empty before and after every
mutation, confirming no residue.

## The Five ROADMAP Success Criteria

| # | Criterion | Command run | Result | Status |
|---|-----------|-------------|--------|--------|
| 1 | `inspect Mandelbrot.exe` prints header fields, project name, `native`, exits 0, writes nothing to disk | Built release binary, ran it, diffed a directory listing before/after | Printed all 8 lines correctly, `EXIT: 0`, `diff` of `ls -la` before/after was empty ("DIRECTORY LISTING IDENTICAL") | ✓ VERIFIED |
| 2 | 44-file sweep: all resolve entry point, reach `VB5!`, name `MSVBVM60.DLL`, report `native` | Shell loop over `find corpus -iname "*.exe"` (44 files) calling the real binary and grepping its output | 0 non-zero exits, 0 missing `MSVBVM60.DLL`, 0 missing `VB5!`, 0 not-native, across all 44 | ✓ VERIFIED |
| 3 | `cargo test --workspace` runs `refusal.rs`; VB5, .NET, empty file each give a distinct `Error` variant with a one-sentence message | Ran `cargo test -p deform6 --test refusal`; read the test source | 9/9 pass: `NotPe` (empty/short-text), `IsVb5`, `IsVb4`, `NoVbRuntime{dot_net:false}`, `NoVbRuntime{dot_net:true}`, `Damaged` are all distinct variants, each backed by `error.rs`'s own tests proving one non-empty line, no `0x`, no path | ✓ VERIFIED |
| 4 | `cargo clippy --all-targets -- -D warnings` passes with the deny wall; `d[0]`, `a + b`, `.unwrap()` each stop the build | Ran `sh scripts/prove-lint-wall.sh` (exit 0) and separately confirmed the deny list in `Cargo.toml`'s `[workspace.lints.clippy]` | Script output: "The wall stops every bad shape, and the tree it leaves behind is clean." `indexing_slicing`, `arithmetic_side_effects`, `unwrap_used` all present as `deny` | ✓ VERIFIED |
| 5 | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` passes on a clean tree | Ran all three individually, fresh, in the actual working tree (which matches HEAD — `git status --porcelain` empty at session start) | fmt exit 0, clippy exit 0 ("Finished ... target(s)"), test suite: 6 `test result: ok` blocks, 134 tests total | ✓ VERIFIED |

Exit codes were also spot-checked directly against the real binary, not
only through the test suite: no path -> 5, empty file -> 1, truncated VB6
file -> 4, no arguments -> 5, unknown subcommand -> 5 (clap's own exit code
2 is never reached, confirming the `Cli::try_parse` mapping named in the
plan's design decision actually holds at runtime).

## DET-01 through DET-06

| Req | Claim | Code | Test | Status |
|---|---|---|---|---|
| DET-01 | Resolve entry point to a file offset via section table | `PeImage::entry_rva`, `section_for`, `rva_to_off` in `read/pe.rs` | `read::pe::tests::*`, exercised end-to-end by `corpus_sweep.rs` on all 44 files | ✓ VERIFIED |
| DET-02 | Follow entry stub to `VB5!` signature | `header_region` in `vb/header.rs`, magic check at line 96 | `vb::header::tests::*` (22 tests), corpus sweep reads it a second, independent way | ✓ VERIFIED |
| DET-03 | Tell VB6 from VB5 by imported DLL name, not signature | `classify`/`runtime_of` in `vb/runtime.rs` | `vb::runtime::tests::*` (17 tests); mutation-tested below | ✓ VERIFIED |
| DET-04 | Refuse VB5 and non-VB files by name, one sentence each | `Refusal::IsVb5`, `Refusal::IsVb4`, `Refusal::NotPe` in `error.rs`; `exit_for` in `main.rs` | `error.rs` sentence-shape tests, `refusal.rs` (9 tests), `cli.rs` | ✓ VERIFIED |
| DET-05 | Report native/P-code from `lpNativeCode` | `ProjectInfo::mode` in `vb/project.rs` line ~145 | `the_corpus_file_is_native_...` plus a synthetic-fixture test for the P-code arm | ✓ VERIFIED, with the P-code arm honestly flagged untested-by-construction (see below) |
| DET-06 | `inspect` prints header, project name, mode; writes nothing to disk | `pub fn inspect` in `vb/mod.rs`; library has no `std::fs`/`PathBuf` | Structural grep (`grep -rnE 'std::fs\|std::path\|PathBuf' crates/deform6/src/` → exit 1, no matches) plus `inspecting_the_corpus_file_changes_no_file_on_disk` in `cli.rs` | ✓ VERIFIED |

## Mutation Testing: Hunting for Tests That Cannot Fail

Four targeted mutations were made directly, run, observed, and reverted.
This is the highest-value check the task asked for, and it was done by
editing real files, not by reading the SUMMARY's own account of having done
so.

| # | Mutation | Command | Observed | Right test failed? |
|---|---|---|---|---|
| 1 | `ObjectTableHead::object_count()` changed to return `w_compiled_objects` instead of `w_total_objects` | `cargo test --workspace` | `a_capacity_above_the_object_count_is_normal_and_is_not_a_defect` failed: `left: 4, right: 1` | Yes — exactly the synthetic-fixture test that exists because no corpus file can distinguish the two fields (`Mandelbrot.exe` has both = 1) |
| 2 | `exit_for` mapped `Refusal::Damaged(_)` to `Exit::NotPe` instead of `Exit::Damaged` | `cargo test -p deform6-cli --test cli` | `a_truncated_visual_basic_6_executable_exits_four` failed: `left: 1, right: 4` | Yes |
| 3 | `vb/mod.rs`'s `inspect` set `runtime_dll` from the `runtime::VB6_DLL` constant instead of the value `runtime_of` returned (mirroring the shadowing shape the SUMMARY itself used) | `cargo clippy --all-targets -- -D warnings` | `error: unused variable: 'runtime_dll'` — build fails, 0 tests even run | Confirmed as claimed: no test in the suite can distinguish this substitution, only the lint wall catches it, and **only because of the specific shadowing shape used**. A rewrite discarding the unused binding as `_matched` instead (also tried) compiles cleanly under both `cargo test` and `cargo clippy` with **no signal at all**. See finding below. |
| 4 | `VbHeader::read` swapped the `0x58`/`0x5C` offsets that decide which is `o_project_exe_name` and which is `o_project_title` | `cargo test --workspace` | 6 tests failed (`the_four_string_offsets_ascend`, `the_executable_name_is_the_vbp_exe_name_without_its_extension`, `the_four_strings_resolve_when_the_address_and_the_file_offset_differ`, `a_string_offset_that_leaves_the_header_window_is_refused`, `the_title_is_the_vbp_title`, `the_corpus_file_reports_what_its_project_file_declares`). Additionally `cargo test -p deform6 --test corpus_sweep` failed separately, naming `Transparency.exe` specifically by its non-ascending offsets. | Yes, redundantly — both the unit-level and the corpus-sweep level catch it |

After each mutation, the file was restored from a backup and `git status
--porcelain` was confirmed empty (only pre-existing untracked `Notes/` and
`.claude/` remained throughout, unrelated to this project). `git diff` was
empty after every revert.

**Finding, info-level, non-blocking:** Mutation 3 shows the "the lint wall
still catches it" claim in the 01-07 and 01-08 SUMMARYs is real but
narrower than the prose implies — it depends on the exact shadow-and-discard
syntax the executor happened to use when testing it. A structurally
equivalent bug written with an underscore-prefixed discard binding compiles
silently under the whole gate, with no test and no lint signal at all. This
is not a functional defect against any ROADMAP success criterion: nothing
in the corpus can ever make `runtime_dll` disagree with the constant by
construction, because every accepted import name is a case-fold of
`MSVBVM60.DLL`, and Phase 2 does not depend on this field's provenance
being independently checkable. It is worth a maintainer's attention only if
a future refactor changes which names `classify` accepts.

## Honesty Audit: The Three Known-Untested Items

| Item | Claimed untested-by-construction? | Where recorded | Verified honest? |
|---|---|---|---|
| P-code branch of `lpNativeCode` (all 44 corpus programs are native) | Yes | Rustdoc on `CompileMode` and `ProjectInfo::mode` in `vb/project.rs` (`# No program in this repository is P-code`), `.planning/WINDOWS.md` item 2, `.planning/research/STRUCTURES.md` | Yes — code doc comment is reachable via `cargo doc`, and `WINDOWS.md`'s open ledger entry #2 states it plainly |
| Section overlap rule (no corpus file has overlapping sections) | Yes | Rustdoc on `overlap_defects` in `read/pe.rs` (`**This rule is untested by construction.**`), `.planning/WINDOWS.md` item 1 | Yes — same discoverability |
| `runtime_dll` filled from the matched name rather than the constant (structurally unobservable) | Yes, in SUMMARY prose (01-07 breakage 7, 01-08 breakage 8) | 01-07-SUMMARY.md, 01-08-SUMMARY.md, and a partial rationale in the `vb/mod.rs` doc comment on `Report`'s `runtime_dll` field | Substantively yes — the fact itself is true and disclosed at length, though only in prose SUMMARY documents rather than in the same systematic `WINDOWS.md` ledger the other two items use. A future maintainer reading only `WINDOWS.md` would miss this third item; one reading either SUMMARY would not. This is an editorial completeness gap, not a code gap, and does not change the phase's verdict. |

No claim in the summaries or in the code overstates what was proved. If
anything the summaries under-claim: 01-05's SUMMARY explicitly documents
that its own plan predicted a wrong observable value (`MZ`) for one
breakage and corrects the record rather than papering over it, and both
01-07 and 01-08 flag their own source plan's D-05 instruction (read
`wCompiledObjects` for the object count) as measurably wrong against the
corpus, and change course before shipping code that would have printed a
wrong count for 15 of 44 corpus programs.

## Anything the Corpus Disproves

None found. The independently-measured facts embedded in the SUMMARYs (all
44 files import exactly one DLL, all 44 reach `VB5!`, `wTotalObjects` equals
the `.vbp`-declared count in 44 of 44 against `wCompiledObjects`'s 29 of 44,
the four header string offsets ascend in 44 of 44) were spot-checked
against the running binary in this session's sweep and found consistent.
The corpus sweep test, run fresh, passed with no modification.

## Anti-Pattern Scan

`grep -rn -E "TBD|FIXME|XXX|TODO|HACK|placeholder|coming soon|not yet implemented"` across
`crates/deform6/src` and `crates/deform6-cli/src`: no matches. No debt
markers, no stubs, no empty-return placeholders found in the phase's
delivered code.

## CI Gate

`.github/workflows/gate.yml` runs all three gate commands as separate steps
plus both proof scripts (`prove-lint-wall.sh`, `prove-region-wall.sh`) on
every push and PR — this matches, and was independently re-run locally
against, everything reported above. `[profile.release]` in `Cargo.toml`
also sets `overflow-checks = true`, so the named "overflow trap" risk is
mitigated in release builds too, not only in debug.

## Requirements Coverage

DET-01 through DET-06 are each claimed by exactly one plan (`01-04`
DET-01, `01-05` DET-02, `01-06` DET-03/04, `01-03` DET-04, `01-07`
DET-05/06, `01-08` re-asserts all six at the integration level). No
orphaned requirement: `REQUIREMENTS.md` maps only DET-01..06 to Phase 1 and
all six are checked `[x]`. SAF-01/SAF-02/SAF-04 references inside Phase 1
plans are the roadmap-documented mechanism-built-here notes (the lint wall,
`forbid(unsafe_code)`, `Region`) for requirements whose proof lands in
Phase 5, not orphans.

## Gaps Summary

None that fail a ROADMAP success criterion, a DET requirement, or a named
risk. All five ROADMAP success criteria were independently executed and
confirmed, not merely re-read from the SUMMARYs. All six DET requirements
trace to real, tested code. Four targeted mutations each produced exactly
the failure the corresponding test exists to catch, with one info-level
finding on the syntactic fragility of the `runtime_dll` "the gate still
catches it" claim (non-blocking — no ROADMAP criterion depends on it). The
three honestly-flagged untested-by-construction items are genuinely
untestable given the vendored corpus (native-only, non-overlapping-sections
-only), and two of the three are recorded in the systematic `WINDOWS.md`
ledger; the third is recorded at length in SUMMARY prose, just not yet
promoted to that same ledger.

---

_Verified: 2026-09-07T18:07:36Z_
_Verifier: Claude (gsd-verifier)_
