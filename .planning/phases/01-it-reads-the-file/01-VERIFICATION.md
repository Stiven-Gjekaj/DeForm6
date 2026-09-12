---
phase: 01-it-reads-the-file
verified: 2026-09-12T18:48:38Z
status: passed
score: 6/6 must-haves verified
covered_files: [".planning/phases/01-it-reads-the-file/01-01-PLAN.md", ".planning/phases/01-it-reads-the-file/01-01-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-02-PLAN.md", ".planning/phases/01-it-reads-the-file/01-02-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-03-PLAN.md", ".planning/phases/01-it-reads-the-file/01-03-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-04-PLAN.md", ".planning/phases/01-it-reads-the-file/01-04-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-05-PLAN.md", ".planning/phases/01-it-reads-the-file/01-05-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-06-PLAN.md", ".planning/phases/01-it-reads-the-file/01-06-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-07-PLAN.md", ".planning/phases/01-it-reads-the-file/01-07-SUMMARY.md", ".planning/phases/01-it-reads-the-file/01-08-PLAN.md", ".planning/phases/01-it-reads-the-file/01-08-SUMMARY.md", ".planning/REQUIREMENTS.md", ".planning/ROADMAP.md", "Cargo.toml", "crates/deform6-cli/src/main.rs", "crates/deform6-cli/tests/cli.rs", "crates/deform6/src/error.rs", "crates/deform6/src/journal.rs", "crates/deform6/src/lib.rs", "crates/deform6/src/read/pe.rs", "crates/deform6/src/read/region.rs", "crates/deform6/src/vb/header.rs", "crates/deform6/src/vb/mod.rs", "crates/deform6/src/vb/project.rs", "crates/deform6/src/vb/runtime.rs", "crates/deform6/tests/corpus_sweep.rs", "crates/deform6/tests/refusal.rs"]
covered_digest: "v1:sha256:17d86d14f5d8ea87a82f6bc9d5df7e1e8ef8f86dd80b33792656599a0d57531a"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 6/6
  gaps_closed: []
  gaps_remaining: []
  regressions: []
corrections_applied:
  - item: "ROADMAP.md Progress table, Phase 1 row"
    from: "| 1. It reads the file | 7/8 | In Progress|  |"
    to: "| 1. It reads the file | 8/8 | Complete    | 2026-09-07 |"
    reason: "All eight 01-0N-PLAN.md/01-0N-SUMMARY.md pairs exist, every SUMMARY.md carries status: complete in its frontmatter, ROADMAP.md's own Phase Details section already read '8/8 plans executed' with all eight plan checkboxes [x], and git history shows commit 98bd41c ('Complete plan 01-08 and close out phase 1', 2026-09-07 19:59:31 +0200) closing the phase. The Progress table row was the one place this fact had not propagated. Phases 2 and 3, which depend on Phase 1, are both built on top of it and independently verified complete. Corrected the row rather than reverting it; no plan is incomplete."
  - item: "01-VERIFICATION.md was invisible to resolveVerificationFile"
    from: "the 2026-09-07 report was committed as VERIFICATION.md, no phase-number prefix"
    to: "renamed to 01-VERIFICATION.md before this session started (per the calling brief); this session verified the content under the new filename and overwrote it with a fresh, live re-verification"
    reason: "resolveVerificationFile matches on the {phase_num}-VERIFICATION.md shape only. The unprefixed filename made every downstream gate report phase 1 as never verified for five days, independent of whether the code held."
---

# Phase 1: It reads the file Verification Report

**Phase Goal:** A person runs `deform6 inspect` on a compiled program and
learns whether DeForm6 can read it, what the project is called, and how it
was compiled. A file that is not a VB6 Standard EXE gets one clear sentence
and a non-zero exit code.

**Verified:** 2026-09-12T18:48:38Z
**Status:** passed
**Re-verification:** Yes. Prior round (2026-09-07) also passed 6/6. This
round re-checks all six requirements and all five ROADMAP success criteria
against the code as it stands today, after Phase 2 and Phase 3 both modified
every file the original report's own `covered_files` list named
(`error.rs`, `vb/mod.rs`, `vb/project.rs`, `main.rs`). Nothing in this round
was inferred from the prior report or from any SUMMARY.md narrative; every
claim below is a command run in this session.

## Why this round was necessary

The 2026-09-07 report was invisible to every gate for five days: it was
committed as `VERIFICATION.md` with no phase-number prefix, so
`resolveVerificationFile` never matched it. That naming problem is fixed by
the rename this session's brief performed before dispatch. Separately, and
more importantly, the four files the prior report's own evidence rests on
have all been rewritten since: `error.rs` gained a dozen new `DefectKind`
variants and the shared `damaged()` helper (Phase 3, review finding WR-01),
`vb/mod.rs` had its report-composition rewired three times, `vb/project.rs`
grew by roughly 2000 lines, and `main.rs`'s printers were extended with
gaps, object graph, declaration and form sections. A passing report from
before those changes is not evidence about the code today. This round
re-derives every truth from the current tree.

## The Five ROADMAP Success Criteria

| # | Criterion | Command run | Result | Status |
|---|-----------|-------------|--------|--------|
| 1 | `inspect Mandelbrot.exe` prints header fields, project name, `native`, exits 0, writes nothing to disk | Built release binary, ran it, diffed `ls -la` on `corpus/vb6-code/Mandelbrot/` before/after | Printed `File`, `Format`, `Runtime`, `Header`, `Project`, `Title`, `Mode  native`, `Objects`; `EXIT: 0`; `diff` empty | ✓ VERIFIED |
| 2 | 44-file sweep: all resolve entry point, reach `VB5!`, name `MSVBVM60.DLL`, report `native` | Null-delimited (`-print0`) shell loop over `find corpus -iname "*.exe"`, the real binary, checked exit code, `MSVBVM60.DLL`, `VB5!` presence (via the printed Header/Runtime lines) and `Mode      native` on each of the 44 | 44 total, 0 bad (0 nonzero exit, 0 missing `MSVBVM60.DLL`, 0 not native) | ✓ VERIFIED |
| 3 | `cargo test --workspace` runs `refusal.rs`; VB5, .NET, empty file each give a distinct `Error` variant with a one-sentence message | `cargo test --workspace`, full run, read `refusal.rs` source | `refusal.rs`: 9/9 pass, unchanged file since 2026-09-07 (git log shows no commit touching it since); fixtures still patch real corpus bytes in memory, not synthetic data | ✓ VERIFIED |
| 4 | `cargo clippy --all-targets -- -D warnings` passes with the deny wall; `d[0]`, `a + b`, `.unwrap()` each stop the build | Ran `cargo clippy --all-targets -- -D warnings` (clean) and `scripts/prove-lint-wall.sh` (which builds and reverts each deliberate mutation) | clippy: `Finished ... target(s)`, no warnings; `prove-lint-wall.sh`: "The wall stops every bad shape, and the tree it leaves behind is clean." `unsafe_code = "forbid"`, `indexing_slicing`/`arithmetic_side_effects`/`unwrap_used = "deny"` confirmed in `Cargo.toml` | ✓ VERIFIED |
| 5 | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` passes on a clean tree | All three run individually, this session, on the actual working tree (`git status --porcelain` showed only the pending rename and untracked `.gsd/`/`Notes/`, no source diff) | fmt: exit 0, no output; clippy: clean; tests: 14 binaries, 622 tests total (411+4+2+26+2+1+40+9+44+2+29+52+0+0), 0 failed | ✓ VERIFIED |

## DET-01 through DET-06, checked against the current code

| Req | Claim | Live evidence this session | Status |
|---|---|---|---|
| DET-01 | Resolve entry point to a file offset via section table | `deform6 inspect` on Mandelbrot.exe prints `Header    VB5! at 0x00001760`; the 44-file sweep resolves this on every corpus program; `PeImage::entry_rva`/`section_for`/`rva_to_off` in `read/pe.rs` unchanged since 2026-09-07 | ✓ VERIFIED |
| DET-02 | Follow entry stub to `VB5!` signature | Same live run: `VB5!` printed and reached on all 44 corpus programs, null-delimited sweep | ✓ VERIFIED |
| DET-03 | Tell VB6 from VB5 by imported DLL name, not signature | `classify()` in `vb/runtime.rs` still returns `name.clone()` off the matched import entry, not the `VB6_DLL` constant; `vb/mod.rs:346` (`let (runtime, runtime_dll) = runtime_of(&pe)?;`) feeds it straight into the report with no shadowing; a dedicated unit test, `a_lower_case_runtime_name_matches_and_comes_back_in_the_case_the_file_holds`, and an independent end-to-end test, `the_runtime_name_and_the_signature_are_read_out_of_the_file` (`vb/mod.rs`), both compare against bytes read a second, independent way and fail if a constant were substituted | ✓ VERIFIED |
| DET-04 | Refuse VB5 and non-VB files by name, one sentence each | `refusal.rs`, 9/9 pass, run live this session; live-ran a truncated VB6 file (`Refusal::Damaged`, "the import directory is unreadable", exit 4) and an empty file (`Refusal::NotPe`, "this file is not a portable executable", exit 1); `exit_for()` in `main.rs` remains an exhaustive match over all seven `Refusal` variants with no wildcard arm | ✓ VERIFIED |
| DET-05 | Report native/P-code from `lpNativeCode` | Live run: `Mode      native`; `CompileMode`/`ProjectInfo::mode` unchanged in `vb/project.rs`, P-code branch still honestly documented as untested by construction (`# No program in this repository is P-code`) | ✓ VERIFIED (P-code arm remains untested-by-construction, as originally and honestly disclosed) |
| DET-06 | `inspect` prints header, project name, mode; writes nothing to disk | Live run confirms the locked eight-line head (`File`/`Format`/`Runtime`/`Header`/`Project`/`Title`/`Mode`/`Objects`) is still printed first and unchanged, with Phase 2/3 sections (gaps, object graph, declarations, forms) appended below it, not interleaved into it; `cli.rs`'s `inspecting_the_corpus_file_prints_the_locked_eight_line_head_unchanged` and `inspecting_the_corpus_file_changes_no_file_on_disk` both pass; `grep -rn "fs::write\|File::create\|fs::create" crates/deform6-cli/src/main.rs` and the equivalent over `crates/deform6/src` (outside `#[cfg(test)]`) return no production write path | ✓ VERIFIED |

## Regression Check: the four risks the brief named

| Risk | Check performed | Result |
|---|---|---|
| Refusal paths still fire, still exit non-zero, `refusal.rs` still runs and can still fail | `cargo test --workspace` ran `refusal.rs` (9/9 pass); the file is byte-identical since 2026-09-07 (`git log -- crates/deform6/tests/refusal.rs` shows no commit since the phase 1 close); its fixtures patch real bytes and assert real `Refusal` variant equality, not a synthetic always-true check | No regression |
| Phase 3's shared `damaged()` helper (`Box::leak`, in `error.rs`) did not weaken the refusal contract; a refusal still names the byte offset and what the code expected | Read `error.rs:489-492`; read ten call sites across `controltree.rs`, `gui.rs`, `frx.rs` — every one builds its `format!()` string with a file offset (`{start_offset:#x}` etc.) and, where relevant, the expected vs. found byte; live-confirmed one such message via Phase 3's own re-verified `Map Editor.exe` example. One pre-existing (2026-09-07, unchanged) `Damaged` message in `vb/runtime.rs` ("the import directory is unreadable") carries no offset — see anti-pattern note below | No weakening; one pre-existing, non-regressive gap noted |
| New `DefectKind` variants: is the CLI's exit-code match still exhaustive, does no new variant silently map to success | `exit_for()` in `main.rs` matches only over `Refusal` (7 variants, unchanged count since 2026-09-07), not `DefectKind`. `DefectKind` grew (Phase 3 added ~10 variants for form/control/OCX parsing), but `DefectKind::severity()` (`error.rs`) is a plain Rust `match` with no `_` arm — the compiler itself refuses a build if a variant is left unmatched, and it compiles clean today. `DefectKind` values become `Defect`s collected into `Report.defects`, never a `Refusal`, so they cannot reach `exit_for` at all; the exit-code table's exhaustiveness is unaffected by `DefectKind`'s growth | No regression; DefectKind's growth is orthogonal to the exit-code match by construction |
| `deform6 inspect` gained Phase 2/3 output; DET-06's own required fields still printed, not displaced | Live run, see DET-06 row above; `print_report()` in `main.rs` prints the eight-line head first, unconditionally, before `print_gaps`/`print_objects`/`print_declarations`/`print_forms` | No regression |

## Progress-table inconsistency, adjudicated

`ROADMAP.md`'s Phase 1 checkbox read `[x]` and its own Phase Details section
already read "**Plans**: 8/8 plans executed" with all eight
`01-0N-PLAN.md` checkboxes `[x]`, but the summary Progress table's Phase 1
row read "7/8 | In Progress". The phase directory holds eight complete
`PLAN.md`/`SUMMARY.md` pairs; every `01-0N-SUMMARY.md` carries
`status: complete` in its frontmatter; `git log` shows commit `98bd41c`
("Complete plan 01-08 and close out phase 1", 2026-09-07) closing the
phase; and Phases 2 and 3, which both depend on Phase 1, are independently
verified complete on top of it. **All eight plans are genuinely complete.**
The Progress table row was the stale artifact, not the Phase Details
section or the checkbox. Corrected this session (see
`corrections_applied`): `7/8 | In Progress` -> `8/8 | Complete    |
2026-09-07`.

## Anti-Pattern Scan

`grep -rn -E "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER"` across
`crates/deform6/src` and `crates/deform6-cli/src`: no matches. No debt
markers found in the phase's delivered code, unchanged from the prior
round.

**One info-level finding, non-blocking.** `error.rs`'s own test
`no_refusal_sentence_dumps_a_byte_or_an_offset` asserts that no `Refusal`
sentence contains `"0x"`, but it only exercises the eight literals in the
`EVERY_REFUSAL` fixture, one of which is a synthetic
`Refusal::Damaged("the entry point is in no section")`. In the shipped
code today, most real `Refusal::Damaged` payloads (everything built
through the `damaged()` helper Phase 3 added) deliberately do carry a hex
byte offset, matching `AGENTS.md`'s own rule that "an error names the byte
offset and what the code expected to find there." The test's name and
assertion generalise a claim that was true for the one variant tested in
2026-09-07 and is not true of the wider `Damaged` surface Phase 3 built.
This is not a DET-04 defect (DET-04 asks for "one clear sentence," which
every `Damaged` message still is) and it is not new to this session's
code, since the test itself is byte-identical since the original commit;
it is a stale generalisation worth a maintainer's attention if a future
edit relies on it.

**WINDOWS.md finding 4, re-examined.** The ledger still marks this finding
`open` with no `fixed_at` date: "`runtime_dll` cannot be proved to come
from the file... Fix in phase 2 by returning the name verbatim and
comparing case-insensitively, then a lower-case import fixture separates
them." Reading the code shows the described fix has, in fact, been present
since the original phase 1 commit (`classify()` returns `name.clone()`,
compares with `eq_ignore_ascii_case`), and the lower-case fixture test the
finding asks for exists and passes
(`a_lower_case_runtime_name_matches_and_comes_back_in_the_case_the_file_holds`),
alongside a second, independent end-to-end proof
(`the_runtime_name_and_the_signature_are_read_out_of_the_file`) that was
not present when this finding was written. The prior verification's own
"Honesty Audit" already flagged this as "substantively yes... an
editorial completeness gap, not a code gap." That remains an accurate
characterisation: the ledger entry is stale, the code is not.

## Requirements Coverage

DET-01 through DET-06 each still trace to real, tested, live-confirmed
code. `REQUIREMENTS.md` maps only DET-01..06 to Phase 1, all six remain
`[x]`, and the Traceability table row (`DET-01 to DET-06 | Phase 1 |
Complete`) matches. No orphaned requirement.

## Gaps Summary

None that fail a ROADMAP success criterion, a DET requirement, or a named
risk. All five ROADMAP success criteria were independently re-executed
against the code as it stands today, not re-read from the prior report.
All six DET requirements hold, live-confirmed via `deform6 inspect` runs
and the corpus sweep, not inferred from SUMMARY.md or from the 2026-09-07
report. The four regression risks the brief named were each checked
directly: the refusal harness is unchanged and still exercised, the shared
`damaged()` helper still names a byte offset and an expectation at every
call site Phase 3 added, the CLI's `Refusal`-to-exit-code match remains
exhaustive and structurally cannot be affected by `DefectKind`'s growth,
and DET-06's required fields print first, unmoved, ahead of every section
Phase 2 and 3 appended. One progress-table row was stale and is corrected.
Two info-level, non-blocking findings are recorded (a test whose "no
offset in a refusal" generalisation no longer matches the wider `Damaged`
surface; a `WINDOWS.md` ledger entry whose underlying code fix already
shipped but whose `open` status was never updated); neither traces to a
ROADMAP success criterion or a DET requirement, and neither is new to this
session's code.

---

_Verified: 2026-09-12T18:48:38Z_
_Verifier: Claude (gsd-verifier)_
