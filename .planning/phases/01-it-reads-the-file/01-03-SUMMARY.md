---
phase: 01-it-reads-the-file
plan: 03
subsystem: errors
tags: [error-model, defect, severity, refusal, journal, exit-codes]
status: complete

requires:
  - the workspace, the lint wall and the module tree from plan 01-01
provides:
  - Site, DefectKind, Severity and Defect
  - DefectKind::severity, one match with no wildcard arm
  - Error, the internal error of the parsing layer
  - Refusal, the full public outcome set behind the six locked exit codes
  - Mode and Journal, with one record method that applies the policy
  - Refusal re-exported as deform6::Refusal and deform6::vb::Refusal
affects:
  - 01-04, which writes impl From<PeReject> for Refusal
  - 01-05, 01-06 and 01-07, which build a Defect at each parse site
  - 01-08, which maps each Refusal variant to an exit code
  - Phase 4, which serialises the same Defect value as report evidence
  - Phase 5, which adds --salvage and reaches Mode::Salvage

tech_stack:
  added: []
  patterns:
    - the message lives next to the variant, through thiserror, so there is no second match to drift
    - severity is one exhaustive match with no wildcard arm, so a new variant fails to compile until somebody decides it
    - the journal pushes the defect before it applies the policy
    - a refusal sentence is checked by predicate over the rendered string, never described in prose

key_files:
  created: []
  modified:
    - crates/deform6/src/error.rs
    - crates/deform6/src/journal.rs
    - crates/deform6/src/lib.rs
    - crates/deform6/src/vb/mod.rs

decisions:
  - STACK.md's literal BadMagic and ImplausibleCount messages carry no offset field, so they cannot name an offset. Both variants gained an offset field. Proved by breaking the message and watching the offset test fail.
  - Error::Io is dropped from the library, as the plan instructs. The library takes a byte slice and never opens a file.
  - Three variants the plan did not assign a severity to were decided here. OffsetOverflow is Fatal. NoNulTerminator and CountMismatch are Recoverable.
  - Error derives Clone, which STACK.md does not. Every field in it is already Clone and a caller that wants to keep the defect and return it needs no workaround.

metrics:
  duration: 1 session
  completed: 2026-09-07

actuals:
  tokens: 6560
  tasks: 2
  commits: 2
plan_head_before: 722c7e05fa402e0ba449408a097505b79837b14b
---

# Phase 01 Plan 03: The error model and the journal Summary

A two level error model where the message sits next to the variant, one
exhaustive `match` that decides severity, and one `record` method that is the
only place in the crate that reads the strict or salvage mode.

## What this plan built

`crates/deform6/src/error.rs` holds `Site`, `DefectKind`, `Severity`,
`Defect`, `Error` and `Refusal`. `crates/deform6/src/journal.rs` holds `Mode`
and `Journal`. `lib.rs` and `vb/mod.rs` each gained one re-export line and
nothing else.

`DefectKind` has eight variants. The five from `STACK.md` section 3 are
`BadMagic`, `OffsetOverflow`, `PastEndOfFile`, `ImplausibleCount` and
`UnmappedAddress`. The three this phase needs are `SectionOverlap` for plan
01-04, `NoNulTerminator` for a bounded `cstr` read, and `CountMismatch` for the
`wCompiledObjects` cross check in plan 01-07.

## The Refusal variant to exit code map

Plan 01-08 reads this table to write the mapping in the CLI. Every variant is
defined now, so the numbering never moves.

| Variant | Exit code | The sentence a person reads |
|---|---|---|
| `NotPe` | 1 | this file is not a portable executable |
| `NotI386` | 1 | this portable executable is for another processor, and DeForm6 reads i386 images only |
| `NotPe32` | 1 | this portable executable is not a 32 bit image, and DeForm6 reads 32 bit images only |
| `NoVbRuntime { dot_net: false }` | 2 | this portable executable holds no Visual Basic runtime |
| `NoVbRuntime { dot_net: true }` | 2 | this portable executable is a .NET assembly, and it holds no Visual Basic runtime |
| `IsVb5` | 3 | this file uses the Visual Basic 5 runtime, and DeForm6 reads Visual Basic 6 only |
| `IsVb4` | 3 | this file uses the Visual Basic 4 runtime, and DeForm6 reads Visual Basic 6 only |
| `Damaged(&'static str)` | 4 | this Visual Basic 6 executable is damaged: {reason} |

Exit code 0 is a file that was read. Exit code 5 is an internal error, and it
has no `Refusal` variant, because it is not a refusal. Plan 01-08 owns codes 0
and 5, and it maps a `clap` usage error to 5 so that `clap` never exits 2
itself.

`Damaged` is defined here and is not reachable from the command line until
Phase 5 adds `--salvage`.

Two things `Refusal` does not hold, both on purpose. It holds no
`object::Error`, because `read/pe.rs` is the only file that may name `object`,
and there is no value in showing nine English strings from a third party crate
to a person who typed a filename. It holds no `serde::Serialize`, because
nothing needs it yet.

## The severity table

One `match`, one arm per variant, no wildcard arm. A ninth variant fails to
compile until somebody decides its severity.

| Variant | Severity | Why |
|---|---|---|
| `BadMagic` | Fatal | The spine of the structure graph starts here. Nothing after it means anything. |
| `OffsetOverflow` | Fatal | The file describes a range that cannot exist. Every later bound check rests on that range. |
| `PastEndOfFile` | Fatal | There are no bytes to read. |
| `UnmappedAddress` | Fatal | A spine pointer that maps nowhere stops the walk. |
| `ImplausibleCount` | Recoverable | A count is a leaf. The rest of the graph still stands. |
| `SectionOverlap` | Recoverable | DeForm6 owns one predicate and picks one section, so the walk continues. |
| `NoNulTerminator` | Recoverable | One string is unreadable. The item that holds it keeps its other fields. |
| `CountMismatch` | Recoverable | The parser takes the smaller count and reports the disagreement. |

The plan assigned five of these eight. `OffsetOverflow`, `NoNulTerminator` and
`CountMismatch` were decided here, and the reason for each is in the table
above and in a comment on its arm.

## The gate

All three commands, run on the working tree at commit `fc6d84e`. The output
below is the real output.

| Command | Exit status |
|---|---|
| `cargo fmt --all --check` | 0, no output |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0 |

```
$ cargo clippy --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
```

```
$ cargo test --workspace
     Running unittests src/lib.rs (target/debug/deps/deform6-ef936274b9c3bf51)
running 17 tests
test error::tests::a_bad_magic_message_names_what_it_expected_there ... ok
test error::tests::a_defect_message_holds_both_the_site_and_the_kind ... ok
test error::tests::a_fatal_kind_is_fatal_and_a_recoverable_kind_is_recoverable ... ok
test error::tests::a_refusal_compares_by_variant_and_not_by_wording ... ok
test error::tests::a_site_a_kind_and_a_defect_all_serialise ... ok
test error::tests::every_defect_message_names_its_byte_offset_in_hexadecimal ... ok
test error::tests::every_refusal_sentence_is_one_non_empty_line ... ok
test error::tests::no_refusal_sentence_dumps_a_byte_or_an_offset ... ok
test error::tests::the_dot_net_flag_gives_a_second_sentence_and_not_a_second_variant ... ok
test error::tests::no_refusal_sentence_carries_a_path ... ok
test journal::tests::a_defect_is_recorded_in_all_four_cases ... ok
test journal::tests::a_fatal_defect_refuses_in_salvage_mode_as_well ... ok
test journal::tests::a_fatal_defect_refuses_in_strict_mode ... ok
test journal::tests::a_recoverable_defect_gives_back_the_fallback_in_salvage_mode ... ok
test journal::tests::a_recoverable_defect_refuses_in_strict_mode ... ok
test journal::tests::the_journal_reports_the_mode_it_was_built_with ... ok
test journal::tests::two_defects_appear_in_the_order_they_were_recorded ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo test --workspace` also runs the `deform6-cli` unit test binary and the
doc test binary, and each reports `0 passed; 0 failed`.

The two greps the plan names are also clean.

```
$ grep -nE '\bobject::' crates/deform6/src/error.rs      # exit 1, no output
$ grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/   # exit 1, no output
```

A third grep, from the plan's verification block, shows that `Mode` is named in
one file only.

```
$ grep -rn '\bMode\b' crates/deform6/src/ crates/deform6-cli/src/ | grep -v journal.rs
                                                          # exit 1, no output
```

## The deliberate breakages

`AGENTS.md` says to break the thing a test covers and watch it fail. Five
breakages were run. The plan asks for three. Two more were run on the
properties `AGENTS.md` names as a product surface, because a test of a message
string is the easiest kind of test to write in a shape that cannot fail.

Each break was reverted, and `cargo test -p deform6 --lib` was green again
before the commit. Neither commit holds a broken state.

| # | What was broken | What failed | Left green |
|---|---|---|---|
| 1 | `Self::BadMagic { .. } => Severity::Fatal` became `Severity::Recoverable` | `a_fatal_kind_is_fatal_and_a_recoverable_kind_is_recoverable` | 9 |
| 2 | The `Fatal` arm split, so `(Fatal, Salvage)` returned `Ok(fallback)` | `a_fatal_defect_refuses_in_salvage_mode_as_well` | 16 |
| 3 | The `push` moved inside the `(Recoverable, Salvage)` arm | `a_defect_is_recorded_in_all_four_cases` | 16 |
| 4 | The `BadMagic` message reverted to the literal string in STACK.md | `a_bad_magic_message_names_what_it_expected_there` and `every_defect_message_names_its_byte_offset_in_hexadecimal` | 8 |
| 5 | `NoVbRuntime` rendered the same sentence for both flag values | `the_dot_net_flag_gives_a_second_sentence_and_not_a_second_variant` | 16 |

A sixth break was run on the `0x` predicate. `Refusal::NotPe` was reworded to
`"this file is not a portable executable: no MZ signature at offset 0x0"`, and
`no_refusal_sentence_dumps_a_byte_or_an_offset` failed with:

```
NotPe puts a raw value in the sentence a person reads:
this file is not a portable executable: no MZ signature at offset 0x0
```

The observed failure text of breakage 1:

```
thread 'error::tests::a_fatal_kind_is_fatal_and_a_recoverable_kind_is_recoverable'
panicked at crates/deform6/src/error.rs:389:13:
assertion `left == right` failed: BadMagic { offset: 0, expected: "VB5!", found: 0 }
must be fatal, because nothing downstream of it means anything
  left: Recoverable
 right: Fatal
```

The observed failure text of breakage 2:

```
thread 'journal::tests::a_fatal_defect_refuses_in_salvage_mode_as_well'
panicked at crates/deform6/src/journal.rs:148:9:
fatal always refuses, in both modes, and salvage does not soften it: Ok(4242)
```

The observed failure text of breakage 3:

```
thread 'journal::tests::a_defect_is_recorded_in_all_four_cases'
panicked at crates/deform6/src/journal.rs:187:17:
assertion `left == right` failed: a run in Strict must hold the defect it saw,
including the run that refuses
  left: 0
 right: 1
```

The observed failure text of breakage 4:

```
thread 'error::tests::every_defect_message_names_its_byte_offset_in_hexadecimal'
panicked at crates/deform6/src/error.rs:354:13:
the message of BadMagic { offset: 17, expected: "VB5!", found: 1094861636 }
does not name its offset 0x11: expected VB5!, found 0x41424344
```

Breakage 3 is the interesting one. It shows the design is load bearing rather
than decorative: moving one line makes the strict run forget the defect it
refused on, and the test that names that property is the only thing that
catches it. That is threat T-01-09.

## The seventeen behaviours and the tests that hold them

Sixteen of the seventeen are runtime properties with a test. One is a compile
time property with no runtime test, and that is stated plainly below rather
than papered over.

| Behaviour | Test |
|---|---|
| Every `DefectKind` message names its offset in hexadecimal | `every_defect_message_names_its_byte_offset_in_hexadecimal` |
| `severity` is exhaustive with no wildcard arm | none at run time, see below |
| `BadMagic`, `UnmappedAddress`, `PastEndOfFile` are `Fatal` | `a_fatal_kind_is_fatal_and_a_recoverable_kind_is_recoverable` |
| `ImplausibleCount` and `SectionOverlap` are `Recoverable` | the same test |
| `Site`, `DefectKind` and `Defect` are `Serialize` | `a_site_a_kind_and_a_defect_all_serialise` |
| A `Refusal` compares by variant | `a_refusal_compares_by_variant_and_not_by_wording` |
| Every refusal sentence is non-empty | `every_refusal_sentence_is_one_non_empty_line` |
| Every refusal sentence holds no newline | the same test |
| Every refusal sentence holds no `0x` | `no_refusal_sentence_dumps_a_byte_or_an_offset` |
| Every refusal sentence holds no `/` and no `\` | `no_refusal_sentence_carries_a_path` |
| `NoVbRuntime` renders two different strings | `the_dot_net_flag_gives_a_second_sentence_and_not_a_second_variant` |
| `Fatal` refuses in `Strict` | `a_fatal_defect_refuses_in_strict_mode` |
| `Fatal` refuses in `Salvage` as well | `a_fatal_defect_refuses_in_salvage_mode_as_well` |
| `Recoverable` refuses in `Strict` | `a_recoverable_defect_refuses_in_strict_mode` |
| `Recoverable` gives back the fallback in `Salvage` | `a_recoverable_defect_gives_back_the_fallback_in_salvage_mode` |
| The defect is present in all four cases | `a_defect_is_recorded_in_all_four_cases` |
| Two defects appear in the order they were recorded | `two_defects_appear_in_the_order_they_were_recorded` |

**The no-wildcard behaviour has no test, and cannot have a useful one.** The
absence of a wildcard arm is a property of the source text, not of a value. A
runtime test cannot observe it. The property is enforced by the compiler: a
ninth variant with no arm is a `non-exhaustive patterns` error. No grep was
written for it either, because a grep for `_ =>` inside one function is a
weaker instrument than the compiler already is, and it would give the reader
the impression that something new is being checked.

Two tests exist that no behaviour in the plan asks for.
`a_defect_message_holds_both_the_site_and_the_kind` pins that the combined
`#[error]` string on `Defect` reaches both halves.
`the_journal_reports_the_mode_it_was_built_with` pins `Journal::mode`, which
would otherwise be a public method with no test at all.

## Deviations from Plan

### 1. `BadMagic` and `ImplausibleCount` gained an `offset` field

**Found during:** Task 1.

**Issue:** The plan's action says to copy the field names and the message
strings from `STACK.md` section 3. The plan's behaviour list says every
`DefectKind` variant renders a message that contains the byte offset it
carries. The two instructions do not agree. `STACK.md` writes

```rust
#[error("expected {expected}, found {found:#x}")]
BadMagic { expected: &'static str, found: u32 },
#[error("count {count} exceeds the {max} that the file can hold")]
ImplausibleCount { count: u32, max: u32 },
```

Neither variant has an offset field, so neither message can name one.

**Fix:** Both variants take `offset: u32` and both messages name it. The
behaviour wins over the literal copy, because `AGENTS.md` states the
requirement directly: "An error names the byte offset and what the code
expected to find there."

**Measured, not argued.** Breakage 4 above put the `STACK.md` string back and
two tests failed. This is not a style preference.

**Files modified:** `crates/deform6/src/error.rs`.

### 2. `Error::Io` is dropped from the library

**Found during:** Task 1, as the plan instructs.

`STACK.md` section 3 gives `Error` a fourth variant:

```rust
#[error("reading {path}: {source}")]
Io { path: std::path::PathBuf, #[source] source: std::io::Error },
```

It is not written. `RESEARCH.md`'s Architectural Responsibility Map puts all
file input and output in the CLI crate, and the library takes `&[u8]`. A
`PathBuf` in the library would be dead code, and it would break the grep that a
later plan runs over `crates/deform6/src/`, which is what keeps the Phase 5
fuzz target pointed at the real public API. Plan 01-08 owns the input and
output error and maps it to exit code 5.

**This is a correction to `STACK.md` section 3.** The next reader of that
document should treat `error.rs` as current and the `Io` variant as withdrawn.
The same section's `BadMagic` and `ImplausibleCount` messages are superseded by
deviation 1.

### 3. `Error` derives `Clone`

`STACK.md` writes `#[derive(Debug, thiserror::Error)]`. `Error` derives
`Clone, Debug, thiserror::Error` here. Every field it holds is already `Clone`,
and dropping `Io` removed the one field that was not. A caller that wants to
keep a refusal and return it needs no workaround.

### 4. Three severities were decided that the plan did not name

The plan names the severity of five variants. It also requires that every
variant has exactly one severity decided in one `match`. `OffsetOverflow`,
`NoNulTerminator` and `CountMismatch` were therefore decided here. The table
above records each choice and its reason, and each arm carries the same reason
as a comment.

## A contradiction in the plan, unresolved by me

The plan's action for task 2 says:

> Add `Journal::new(mode)`, `Journal::mode()` and `Journal::defects()`
> returning a slice. Nothing else. Do not add a method that lets a caller read
> the mode and branch on it.

`Journal::mode()` is a method that lets a caller read the mode. The plan's own
threat register says the same thing more strongly, under T-01-10: "`Journal`
exposes no branchable mode accessor to a parse site."

`Journal::mode()` was written, because the action names it and because the
Phase 4 report has to say which mode a run used. The instruction that follows
it is read as "add nothing beyond these three".

**The mitigation that actually holds is the grep, not the absence of the
method.** A parse site cannot branch on `journal.mode()` without naming the
type `Mode` in the comparison, and the plan's verification grep shows `Mode`
named in `journal.rs` only. That grep is run above and it is clean. If the
reviewer wants the stronger reading, delete `Journal::mode` and give the report
a different route to the mode. That is a one line change and nothing else
depends on it today.

## Findings that correct the sources

### `thiserror` 2.0.20 accepts an `if` expression directly in `#[error]`

`RESEARCH.md` and the plan both assume the conditional needs a trailing format
argument. It does not need one. This compiles and both branches render:

```rust
#[error("{}", if *dot_net {
    "this portable executable is a .NET assembly, and it holds no Visual Basic runtime"
} else {
    "this portable executable holds no Visual Basic runtime"
})]
NoVbRuntime { dot_net: bool },
```

The named field is in scope inside the expression as a reference, so `*dot_net`
is the deref and no `.dot_net` shorthand is needed. No helper function is
required. `[VERIFIED: local, cargo test]`

### The lint wall needed no relaxation in library code, again

`RESEARCH.md` correction 5 says the derive output of `serde::Serialize` and
`thiserror::Error` passes the wall clean. That holds for all six types here,
including the `if` expression above and the `const fn severity`. The only
`#[allow]` in either file is the `#[cfg(test)]` module attribute from
`RESEARCH.md` section 2.5, and both files use `#[allow]` and not `#[expect]`.

## Known Stubs

None. `error.rs` and `journal.rs` are complete for this phase. `Damaged` and
`Mode::Salvage` are defined and not reachable from the command line until Phase
5, which the plan and `CONTEXT.md` both require, and both are covered by tests
here.

## What this plan did not touch

`STATE.md`, `ROADMAP.md` and `REQUIREMENTS.md` were not edited. Plan 01-02 runs
in the same wave and shares those files, so the orchestrator owns them after
the merge. Plan 01-02's files, `read/region.rs`, `scripts/prove-region-wall.sh`
and `.github/workflows/gate.yml`, were not touched either.

## Threat Flags

None. This plan adds no network endpoint, no auth path and no file access path.
It removes one: `Error::Io` and its `PathBuf` are not written.

The four mitigations the plan's threat register names are in place.

- T-01-08: a refusal sentence holds no `0x`, no `/` and no `\`, proved by two
  predicate tests over every variant, and proved able to fail by breakage 6.
- T-01-09: the push is before the policy match, proved by
  `a_defect_is_recorded_in_all_four_cases`, and proved able to fail by
  breakage 3.
- T-01-10: `Mode` is read in `record` only, and `grep -rn '\bMode\b'` finds it
  in `journal.rs` only. See the unresolved contradiction above.
- T-01-SC: no package was added. `git diff 722c7e0 HEAD -- Cargo.toml
  Cargo.lock` is empty. `cargo tree -e normal --prefix none --no-dedupe` lists
  19 distinct name and version pairs, of which 2 are the workspace crates
  themselves, so 17 come from the registry. That is the number
  `RESEARCH.md` records. `grep -c '^name = ' Cargo.lock` gives 19, which counts
  the two workspace crates as well.

## Commits

| Commit | Subject |
|---|---|
| `eafe3b5` | Add the defect, severity and refusal error model |
| `fc6d84e` | Add the journal that applies the strict or salvage policy once |

Both hashes resolve with `git log --oneline <hash> -1`. Every file named in
`key_files.modified` exists on disk. `git diff --diff-filter=D --name-only
722c7e0 HEAD` is empty, so neither commit deleted a tracked file.

## Self-Check: PASSED

## Resolved after the merge: `Journal::mode` is gone

This summary flagged that the plan asked for `Journal::mode()` and, two lines
later, said not to add a method that lets a caller read the mode and branch on
it. Threat T-01-10 states the stronger form: `Journal` exposes no branchable
mode accessor to a parse site.

The orchestrator took the stronger reading and deleted the method and its test.

The reason is the one `AGENTS.md` gives for the whole lint wall. A grep that
says no file outside `journal.rs` names `Mode` is a rule somebody has to keep.
A method that does not exist cannot be called. The accessor had exactly one
caller in the whole workspace, which was its own test, so the deletion cost
nothing.

The gate stays green at 34 tests, one fewer than before, and that one is the
test of the deleted method.
