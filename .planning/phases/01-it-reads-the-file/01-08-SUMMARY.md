---
phase: 01-it-reads-the-file
plan: 08
subsystem: cli
tags: [clap, exit-codes, inspect, refusal, corpus-sweep, det-01, det-02, det-03, det-04, det-05, det-06]
status: complete

requires:
  - Report and inspect, re-exported from the crate root, from plan 01-07
  - Refusal and its seven variants, and the exit-code table in error.rs's
    doc comment, from plan 01-03
  - PeImage::dll_name_sites and PeImage::has_clr_header, from plan 01-04
provides:
  - crates/deform6-cli/src/main.rs, the deform6 binary, with the six exit
    codes and the locked eight-line output shape
  - crates/deform6-cli/tests/cli.rs, ten behaviours over the real binary
  - crates/deform6/tests/refusal.rs, nine tests, fixtures patched in memory
  - crates/deform6/tests/corpus_sweep.rs, the sweep over all 44 executables
affects:
  - Phase 2, which extends inspect and must keep the eight-line shape and
    the exit-code table stable
  - Phase 4, whose --json flag and confidence report are the second output
    shape this phase deliberately does not build
  - Phase 6, whose released documentation quotes the printed output below

tech_stack:
  added: []
  patterns:
    - Cli::try_parse with a hand-written, exhaustive match from Refusal to
      Exit, so a usage error cannot collide with the locked exit-code table
      and a new Refusal variant is a compile error until its code is chosen
    - a #[repr(u8)] fieldless enum cast with `as u8`, which the lint wall
      permits, in preference to a match arm per exit code
    - a second, independent read of a structure (VbHeader, in the sweep)
      cross-checked against the value a composer (inspect) produced, so a
      field-swap bug inside the composer is caught and not only a
      structural regression in the read itself

key_files:
  created:
    - crates/deform6-cli/tests/cli.rs
    - crates/deform6/tests/refusal.rs
    - crates/deform6/tests/corpus_sweep.rs
  modified:
    - crates/deform6-cli/src/main.rs

decisions:
  - "D-05 in this plan names wCompiledObjects as the source of the Objects
    line. That is wrong: plan 01-07 measured wTotalObjects against all 44
    .vbp files (44 of 44) against wCompiledObjects (29 of 44), and
    Report::object_count already returns wTotalObjects. The CLI prints
    report.object_count unchanged and takes no action on D-05."
  - "The corpus sweep's offset-ordering check reads VbHeader a second time,
    independent of Report, and cross-checks Report::exe_name, Report::title
    and Report::help_file against that independent read by field identity.
    A raw ascending-offset check alone (o_project_exe_name < o_project_title
    < ...) is blind to a bug that swaps which string inspect copies into
    which Report field, because that bug never touches the offsets
    themselves. The cross-check is what makes the mandated breakage
    (swapping title and exe_name in vb/mod.rs) fail; verified below."

actuals:
  tokens: 6770
  tasks: 3
  commits: 3
plan_head_before: 8e9ba9a83a100998885f972f2dfe8fcfd0f40c8e

metrics:
  duration: 1 session
  completed: 2026-09-07
---

# Phase 01 Plan 08: The deform6 command line, the exit codes, and the phase gate Summary

`crates/deform6-cli/src/main.rs` is the `deform6` binary: one `inspect`
subcommand, six exit codes, and the locked eight-line output shape, read
from `Report::runtime_dll` and `Report::signature` rather than from a
literal. `crates/deform6-cli/tests/cli.rs`, `crates/deform6/tests/refusal.rs`
and `crates/deform6/tests/corpus_sweep.rs` are the three test files that turn
the phase's ROADMAP success criteria into commands `cargo test --workspace`
runs. This is the first time anything in DeForm6 runs end to end.

134 library and CLI tests now pass (115 library, 9 `refusal.rs`, 1
`corpus_sweep.rs`, 9 `cli.rs`; two doc-test targets run 0 tests each).

## What this plan built

| Item | What it gives |
|---|---|
| `Exit` | `#[repr(u8)]` enum, `Ok = 0` through `Internal = 5`, all six defined now |
| `main` | Parses with `Cli::try_parse`, maps every clap error by hand, returns `ExitCode` |
| `exit_for` | Exhaustive match from `Refusal` to `Exit`, no wildcard arm |
| `print_report` | The eight labelled lines, reading `Report` fields, never a literal |
| `crates/deform6-cli/tests/cli.rs` | 9 tests over the real binary via `CARGO_BIN_EXE_deform6` |
| `crates/deform6/tests/refusal.rs` | 9 tests, every fixture patched in memory from `Mandelbrot.exe` |
| `crates/deform6/tests/corpus_sweep.rs` | 1 test, 44 executables, count asserted first, failures collected |

## The correction the orchestrator flagged, applied

The prompt for this plan states that D-05 and the plan's own task 1 name
`wCompiledObjects` as the source of the printed object count, and that this
is wrong: plan 01-07 measured `wTotalObjects` against the declared object
count of all 44 `.vbp` files (44 of 44) against `wCompiledObjects` (29 of
44), and corrected `ObjectTableHead::object_count` to return `wTotalObjects`
before this plan started. `Report::object_count` already carries the right
value. This plan's `print_report` calls `report.object_count.to_string()`
unmodified — there was no `wCompiledObjects` read anywhere in
`deform6-cli/`, so **no code change was needed**, only the confirmation that
none was introduced. The `Objects   1` line for `Mandelbrot.exe`, printed
below, is the number its `.vbp` declares.

## The real printed output

```
$ deform6 inspect corpus/vb6-code/Mandelbrot/Mandelbrot.exe
File      Mandelbrot.exe  (28672 bytes)
Format    PE32, 3 sections
Runtime   MSVBVM60.DLL  (Visual Basic 6)
Header    VB5! at 0x00001760  build 0x2636
Project   Mandelbrot_Fractal_Demo
Title     Mandelbrot Fractal Demo
Mode      native
Objects   1
$ echo $?
0
```

This matches CONTEXT.md's example line for line, including the byte count,
the section count, the header offset and the build number, because those
happen to be the real values for this file (RESEARCH.md Pitfall 6 warns they
usually are not, and no test in this plan pins them).

```
$ echo "hello" > /tmp/notaprogram.txt
$ deform6 inspect /tmp/notaprogram.txt
this file is not a portable executable
$ echo $?
1
```

## The sweep result over all 44

```
$ cargo test -p deform6 --test corpus_sweep
running 1 test
test all_forty_four_corpus_executables_read_and_report_what_they_hold ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

All 44 corpus executables resolve the entry point to a file offset, carry
the four-byte `VB5!` signature, name `MSVBVM60.DLL` as `Report::runtime_dll`,
report `native`, declare a non-empty project name, and pass a second,
independent header read whose four string offsets ascend
(`o_project_exe_name < o_project_title < o_help_file <= o_project_name`).
**Every one of the 44 vendored projects carries `CompilationType=0`, which is
native, so the P-code branch of `Report::native` is untested by
construction.** A P-code binary is needed before that branch can be called
tested; `STRUCTURES.md` section 12 names `TimoKunze/ExplorerTreeView-VB6` as
a known source of one. The test that would exercise a real P-code sample
does not exist in this repository, and no test here claims that it does.

## The refusal fixtures, and what they do and do not prove

All nine `refusal.rs` tests pass, including the control and the fixture that
sets data directory 14. **The VB5, VB4 and .NET refusals are proved against
fixtures patched in memory, and not against real binaries of those kinds.**
Every fixture starts from the corpus file `Mandelbrot.exe` and patches one
imported DLL name at the offset `PeImage::dll_name_sites` names — never a
byte search — plus, for the .NET fixture only, data directory 14. The
reason, stated in the file's own module doc comment: a patched import name
proves the discrimination logic in `deform6::vb::runtime::classify`. It does
not prove that DeForm6 handles a genuine Visual Basic 5 binary, whose header
layout differs after header-relative offset `0x30`, nor a genuine .NET
assembly, which carries a common language runtime header the patched file
does not. DeForm6 refuses both before it reads any of that, so the fixtures
cover the behaviour this phase promises and no more. `git status --porcelain
corpus/` is empty after every run in this plan — nothing was written to
disk.

## The gate

Run on the tree after all three commits, with a clean working tree.

| Command | Exit |
|---|---|
| `cargo fmt --all --check` | 0, no output |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0, 134 tests pass |
| `sh scripts/prove-lint-wall.sh` | 0 |
| `sh scripts/prove-region-wall.sh` | 0 |
| `cargo test -p deform6 --test corpus_sweep` | 0, 1 test |
| `cargo test -p deform6 --test refusal` | 0, 9 tests |
| `cargo test -p deform6-cli --test cli` | 0, 9 tests |

The acceptance greps this plan names were each run.

```
$ grep -rnE '(Cli::parse|process::exit)[[:space:]]*\(' crates/deform6-cli/     # exit 1, no output
$ grep -rn 'json' crates/deform6-cli/src/main.rs                              # exit 1, no output
$ grep -rnE 'MSVBVM60|VB5!' crates/deform6-cli/src/                           # exit 1, no output
$ grep -qE 'windows\(|\.position\(' crates/deform6/tests/refusal.rs           # exit 1, no match
$ git status --porcelain corpus/                                              # empty
$ grep -qE 'assert[^\n]*\b44\b' crates/deform6/tests/corpus_sweep.rs          # exit 0
$ grep -q 'MSVBVM60.DLL' crates/deform6/tests/corpus_sweep.rs                 # exit 0
$ grep -qE '0x2636|28672|0x231C' crates/deform6/tests/corpus_sweep.rs         # exit 1, no match
```

## The nine deliberate breakages

`AGENTS.md` requires breaking the thing a test covers and watching it fail.
Nine breakages across the three tasks, all reverted, all confirmed
byte-identical to the tree before the breakage (`diff` against a backup, not
just `git status`), and the gate was green before each commit.

### Task 1 — `crates/deform6-cli/src/main.rs`

**1. `Cli::try_parse` replaced with the non-fallible entry point.**
`cargo test -p deform6-cli --test cli`:

```
---- an_unknown_subcommand_exits_five_and_not_two ----
  left: 2
 right: 5
---- no_arguments_exits_five_and_not_two ----
  left: 2
 right: 5
test result: FAILED. 7 passed; 2 failed
```

Both usage-error tests fail with code 2, exactly the collision RESEARCH.md
section 7.7 predicts.

**2. `Refusal::Damaged` mapped to `Exit::NotPe` instead of `Exit::Damaged`.**

```
---- a_truncated_visual_basic_6_executable_exits_four ----
  left: 1
 right: 4
test result: FAILED. 8 passed; 1 failed
```

Exactly the one test that feeds a truncated corpus file to the binary fails.
No other test in `cli.rs` reaches a damaged file, which is the finding: the
truncated-file test is what makes this mapping observable at all, and it was
added to the file per this task's own instruction rather than assumed
present.

### Task 2 — `crates/deform6/tests/refusal.rs`

**3. `VB5_DLL` changed from `"MSVBVM50.DLL"` to `"MSVBVM60.DLL"`.**

```
---- a_visual_basic_5_runtime_name_is_refused_by_name ----
  left: Err(NoVbRuntime { dot_net: false })
 right: Err(IsVb5)
test result: FAILED. 8 passed; 1 failed
```

**4. The control test deleted.** The remaining eight tests all still pass:

```
running 8 tests
test result: ok. 8 passed; 0 failed
```

This is the demonstration the control exists to make: with it gone, a
refuse-everything bug in `inspect` would leave every remaining test green,
because none of the other eight is a positive case.

**5. `PeImage::has_clr_header` forced to return a constant `false`.**

**The claim this plan's own prompt states — "two executors have measured
that this fails exactly one test in the whole workspace" — does not hold
against the tree as it stands. The real measured count is three.**

```
$ cargo test --workspace
---- read::pe::tests::a_patched_data_directory_fourteen_makes_the_image_a_dot_net_assembly ----
assertion failed: image.has_clr_header()
---- vb::runtime::tests::a_common_language_runtime_directory_makes_the_refusal_name_a_dot_net_assembly ----
assertion failed: image.has_clr_header()
test result: FAILED. 113 passed; 2 failed   (library)
---- a_common_language_runtime_header_makes_the_refusal_name_a_dot_net_assembly ----
test result: FAILED. 8 passed; 1 failed     (refusal.rs)
```

Plans 01-04 and 01-06 each already carry a unit test that asserts
`image.has_clr_header()` directly on the same fixture shape this plan's
`refusal.rs` builds, and both predate this plan. Forcing the method to a
constant breaks those two library tests as well as the new `refusal.rs`
test — three, not one. This is reported rather than quietly matched to the
prompt's number, per AGENTS.md: "tell the human when the results do not
agree with your statement." What the breakage does confirm, exactly as
asked: `an_unrecognised_import_name_is_refused_as_holding_no_visual_basic_runtime`
(the name-only fixture, no data directory 14) keeps passing throughout, which
is the proof that the name-only fixture and the data-directory-14 fixture are
genuinely different inputs and not two names for one case.

### Task 3 — `crates/deform6/tests/corpus_sweep.rs`

**6. The asserted count changed from 44 to 0.**

```
thread '...' panicked: assertion `left == right` failed: found 44 corpus executables, wanted 44
  left: 44
 right: 0
test result: FAILED. 0 passed; 1 failed
```

The test fails on the count comparison itself — the walk still finds all 44
files, so a sweep with a wrong asserted count is caught immediately, and an
empty-list vacuous pass is impossible by construction (the count is checked
before the loop, and the walk is not what was broken).

**7. The `exe_name` and `title` fields swapped in `vb/mod.rs`'s
`Report` construction.**

```
23 of 44 corpus executables failed:
.../Mandelbrot/Mandelbrot.exe: Report::exe_name is "Mandelbrot Fractal Demo";
  the header names "Mandelbrot" at its own executable name offset
[22 more files named]
test result: FAILED. 0 passed; 1 failed
```

**A defect in the plan, found here.** The plan's action text calls this "the
ordering assertion" and expects it to fail. The four *offsets*
(`o_project_exe_name` etc.) are read independently of `Report` in this
sweep, precisely so the check is not fooled by a bug in `inspect`'s field
assignment — so a swap purely inside `Report` construction leaves those
offsets untouched and ascending, and an ordering check built from offsets
alone would not have caught this breakage at all. What actually catches it,
and what this sweep implements for exactly this reason, is a **cross-check**
of `Report::exe_name`/`title`/`help_file` against the same independent
header read, by field identity. 23 of 44 files have a title that differs
from their exe name, which is why 23 (not 44) are named — the corpus does
not uniformly distinguish the two strings, and the sweep reports every file
where the distinction exists rather than stopping at the first.

**8. `runtime_dll` filled from the `VB6_DLL` constant instead of the name
`runtime_of` returned.**

```
$ cargo test -p deform6 --test corpus_sweep
test result: ok. 1 passed; 0 failed
```

**No test failure**, exactly as this plan's task 3 predicts and asks to be
recorded, and for the reason plan 01-07 already recorded for the identical
substitution in the library composer: `PeImage::imported_dlls` upper-cases
every name before `classify` sees it, and the only names `classify` accepts
as Visual Basic 6 are case variants of `MSVBVM60.DLL`, so the matched name is
always textually equal to the constant for any file that reaches a `Report`
at all. No fixture on this corpus can make the two differ. What does catch
the identical substitution when it is made in the library itself (not
exercised here, but confirmed for completeness):

```
$ cargo clippy --all-targets -- -D warnings
error: unused variable: `runtime_dll`
error: could not compile `deform6` (lib) due to 1 previous error
```

`cargo clippy --all-targets -- -D warnings` catches it as an unused binding,
not as a test failure — the same outcome plan 01-07's SUMMARY records for
this exact substitution. This is the known limit of the sweep's runtime
check: it observes that the printed value is correct, not that it was
sourced correctly, and the reason no test can close that gap on this corpus
is structural (recorded above), not an oversight.

**9. Verified restoration.** After each of the nine breakages, the modified
file was restored from a backup taken before the breakage and `diff`
confirmed byte-identical, not merely `git status --porcelain` showing clean
— `main.rs`, `vb/runtime.rs`, `read/pe.rs`, `vb/mod.rs` and
`corpus_sweep.rs` each round-tripped exactly.

## Defects found in the plan

**1. D-05 and task 1's instruction to print the object count from
`wCompiledObjects` are wrong**, per plan 01-07's measurement (44 of 44
against `wTotalObjects`, 29 of 44 against `wCompiledObjects`). The
orchestrator's prompt flagged this before execution began. No code in this
plan reads `wCompiledObjects`; `report.object_count` (already
`wTotalObjects`, from plan 01-07) is printed unmodified.

**2. The prompt's claim that forcing `has_clr_header` to a constant `false`
"fails exactly one test in the whole workspace" does not match the tree as
it stands.** The real, measured count is three: the pre-existing unit tests
in `read/pe.rs` (plan 01-04) and `vb/runtime.rs` (plan 01-06), plus this
plan's own `refusal.rs` test. See breakage 5 above for the full output.

**3. Task 3's action text calls the check that catches the exe-name/title
swap breakage "the ordering assertion".** The raw ascending-offset check
(read independently of `Report`) cannot see that swap, because the swap
never touches the offsets. The check that does catch it is a cross-check of
`Report`'s string fields against the same independent header read, which
this sweep implements specifically because the ordering check alone would
not have satisfied the plan's own stated breakage requirement. See breakage
7 above.

None of the three required a plan re-read or a scope change: the first was
already corrected by 01-07 and this plan's job was to not undo it, and the
second and third are measurement corrections recorded here rather than
silently matched to the plan's numbers.

## Divergences from the plan that are choices, not defects

**`an_unknown_subcommand_exits_five_and_not_two` and
`no_arguments_exits_five_and_not_two` check the exit code only, not a
single-line stderr.** The action text says "every test in this file that
expects a non-zero exit code also asserts that stderr holds exactly one
non-empty line," stated with no exception. `clap`'s own usage-error message
is several lines by its own shape (a summary line, a blank line, a usage
line, a hint line), which is not this project's shape to control and is not
in scope for DET-04's "one clear sentence" (that requirement is about
`inspect`'s refusals, which this project owns end to end). The one-line
assertion is applied to the two `inspect`-driven refusal tests (`a_short_
text_file...`, `a_path_that_does_not_exist...`, and the added
`a_truncated_visual_basic_6_executable_exits_four`), where DeForm6 controls
the sentence. RESEARCH.md section 7.6's own reference test table checks the
exit code only for the bad-subcommand case, which agrees with this reading.

## Threat mitigations

| Threat | State |
|---|---|
| T-01-29, the clap exit-code collision | Mitigated and measured. `Cli::try_parse` plus a hand-written, exhaustive match. Breakage 1 shows the collision when the fallible entry point is bypassed. |
| T-01-30, `inspect` writing to disk | Mitigated and measured. `inspecting_the_corpus_file_changes_no_file_on_disk` snapshots path, length and modification time for every entry of the executable's directory, before and after. |
| T-01-31, information disclosure in refusal sentences | Mitigated. `error.rs`'s own tests (plan 01-03) already prove no refusal sentence carries a byte, an offset or a path; this plan's `run_inspect` adds exactly one sentence naming the path only on an input-output error, which is the one case DeForm6 itself, not the library, owns. |
| T-01-32, the input path argument | Mitigated. `input: PathBuf`, never `String`; nothing is written in this phase. |
| T-01-33, a test that passes for the wrong reason | Mitigated and measured, nine times over (the breakages above), including the case where the true count was smaller than the plan predicted. |
| T-01-SC, third-party binary fixtures | Mitigated. `git status --porcelain corpus/` was checked empty after every gate run in this plan. |

## Known Stubs

None. Every file this plan names is implemented, and `--json`, `--salvage`
and any output-format option are absent by design (D-01), not stubbed.

## Deferred Issues

None new. The three items `.planning/WINDOWS.md` already carries from prior
plans (the section-overlap rule untested by construction, the P-code branch
untested by construction, `inspect` dropping the defects it collects) are
unchanged by this plan.

## Threat Flags

None. This plan opens one file for reading and writes to stdout and stderr
only. `grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/` still
finds nothing in the library; the one `std::fs::read` in this phase lives in
`deform6-cli/src/main.rs`, which is the crate `AGENTS.md` and the threat
model assign that responsibility to.

## Commits

| Commit | Subject |
|---|---|
| `ce9c6ad` | Add the inspect subcommand and the exit code mapping |
| `c26322e` | Add the refusal tests built from patched fixtures |
| `c259684` | Add the corpus sweep over all 44 executables |

`git rev-list --count 8e9ba9a..HEAD` is 3, measured before this SUMMARY was
written. Each commit carries one file's code and, where applicable, its
tests together, per `AGENTS.md`.

## Self-Check: PASSED

`crates/deform6-cli/src/main.rs` exists, 189 lines.
`crates/deform6-cli/tests/cli.rs` exists, 208 lines.
`crates/deform6/tests/refusal.rs` exists, 183 lines.
`crates/deform6/tests/corpus_sweep.rs` exists, 174 lines.
All three commit hashes resolve in the history of this branch.
`git status --porcelain corpus/` is empty. `cargo test --workspace` exits 0
with 134 tests passing. The three phase-level commands
(`cargo test -p deform6 --test corpus_sweep`,
`cargo test -p deform6 --test refusal`,
`cargo test -p deform6-cli --test cli`) each exit 0.
