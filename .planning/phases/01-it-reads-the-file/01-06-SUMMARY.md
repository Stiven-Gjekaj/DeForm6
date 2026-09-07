---
phase: 01-it-reads-the-file
plan: 06
subsystem: vb
tags: [runtime, imports, discrimination, refusal, exit-codes, det-03, det-04]
status: complete

requires:
  - Refusal and its seven variants from plan 01-03
  - PeImage, imported_dlls, delay_loaded_dlls, dll_name_sites and
    has_clr_header from plan 01-04
  - the vb/runtime.rs stub and the pub mod line from plan 01-01
provides:
  - VB6_DLL, VB5_DLL, VB4_32_DLL and VB4_16_DLL, the four runtime names
  - Runtime, an enum with one variant
  - classify, a pure decision over two name lists and a flag
  - runtime_of, the same decision over a parsed image
affects:
  - 01-07, which prints the matched runtime name on the Runtime line
  - 01-08, which maps IsVb5 and IsVb4 to exit code 3 and NoVbRuntime to 2,
    and whose sweep reads the matched name for each of the 44 corpus files
  - Phase 5, which adds the salvage path for a packed import directory

tech_stack:
  added: []
  patterns:
    - a decision that takes data and never takes a file, so every case it
      answers is reachable from a literal in a test
    - the matched value travels out, because a one variant enum makes the
      enum itself a tautology
    - one fixture helper serves two data directories, so neither copy can
      drift
    - a fixture helper asserts that its write changes something, so a fixture
      that has become a no-op fails loudly

key_files:
  created: []
  modified:
    - crates/deform6/src/vb/runtime.rs

decisions:
  - runtime_of returns the pair (Runtime, String), not Runtime. The plan's
    action names one type and its own behaviour list, its prose and its
    success criterion 5 name the other. The pair wins, because the name is
    the only value that carries evidence.
  - runtime_of has two arms and not three. The empty list and a list with no
    Visual Basic runtime give the same answer, so a third arm would be a
    second copy of one rule in classify.
  - The "no import data directory" fixture zeroes data directory 1 on a copy
    of the corpus bytes, through the same helper that sets directory 14. The
    alternative, a synthetic image built in the test, would duplicate about
    forty lines of a builder that already exists and is private in
    read/pe.rs.
  - classify holds `let _ = delay_loaded;`. An unused parameter is a rustc
    warning and the gate denies warnings, so the plan's instruction as
    written does not compile.

metrics:
  duration: 1 session
  completed: 2026-09-07

actuals:
  tokens: 12400
  tasks: 2
  commits: 2
plan_head_before: a7e4e24bcbc3785bf100df64c398d2ef17cfe2d0
---

# Phase 01 Plan 06: The runtime discriminator Summary

One file that decides the Visual Basic version from the name of the imported
runtime DLL, gives that name back to the caller, and separates an import
directory that cannot be read from one that is not there.

## What this plan built

`crates/deform6/src/vb/runtime.rs`, 486 lines, holds four name constants, the
`Runtime` enum, the two functions below, and 17 tests. It names no file system
type and it names no third party parser crate.

| Item | What it is |
|---|---|
| `VB6_DLL` | `MSVBVM60.DLL` |
| `VB5_DLL` | `MSVBVM50.DLL` |
| `VB4_32_DLL` | `VB40032.DLL` |
| `VB4_16_DLL` | `VB40016.DLL`, kept and unreachable |
| `Runtime` | one variant, `Vb6` |
| `classify(imports, delay_loaded, dot_net)` | `Result<(Runtime, String), Refusal>` |
| `runtime_of(pe)` | `Result<(Runtime, String), Refusal>` |

`runtime_of` is four lines:

```rust
let imports = pe
    .imported_dlls()
    .map_err(|_| Refusal::Damaged("the import directory is unreadable"))?;
let delay_loaded = pe.delay_loaded_dlls().unwrap_or_default();
classify(&imports, &delay_loaded, pe.has_clr_header())
```

## The corpus facts, measured again here

The prompt gave these numbers and told me not to re-derive them. I measured
them anyway, because a fixture in this plan rests on the first one and a claim
I cannot prove is not a claim. A script read the import data directory of every
`.exe` under `corpus/` and walked the descriptors itself.

| Fact | Result |
|---|---|
| Executables under `corpus/` | 44 |
| Files importing exactly one DLL | 44 of 44 |
| That DLL is `MSVBVM60.DLL`, in the literal upper case, 12 bytes | 44 of 44 |
| Files with a delay load data directory | 0 of 44 |
| Files with a common language runtime data directory | 0 of 44 |

Every number the prompt gave held. The third row is what lets
`with_the_runtime_named` assert that the site it patches holds the runtime
name and that the replacement is 12 bytes long.

**The tests in this file cover one corpus executable, `Mandelbrot.exe`, not
all 44.** The sweep over all 44 belongs to plan 01-08, whose integration test
file is a separate crate root and may walk a directory. The table above is a
measurement I ran beside the suite, not a test in the repository.

## The gate

Run on the committed tree at `320fc17`, with a clean working tree.

| Command | Exit |
|---|---|
| `cargo fmt --all --check` | 0, no output |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0 |
| `sh scripts/prove-lint-wall.sh` | 0 |
| `sh scripts/prove-region-wall.sh` | 0 |

```
$ cargo test --workspace
     Running unittests src/lib.rs (target/debug/deps/deform6-ef936274b9c3bf51)
test result: ok. 75 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running unittests src/main.rs (target/debug/deps/deform6-31e29b1143ef8fee)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   Doc-tests deform6
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

75 library tests, of which 17 are in `vb::runtime::tests`. The phase held 58
before this plan, so the 17 are all new and none was displaced.

```
$ sh scripts/prove-lint-wall.sh          # exit 0
The wall stops every bad shape, and the tree it leaves behind is clean.

$ sh scripts/prove-region-wall.sh        # exit 0
The type refuses every shape, and the tree it leaves behind is clean.
```

The acceptance greps.

```
$ grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/         # exit 1
$ grep -nE 'b"VB5!"|0x21354256|windows\(4\)' crates/deform6/src/vb/runtime.rs
                                                                    # exit 1
$ grep -qE '\bto_uppercase\b' crates/deform6/src/vb/runtime.rs      # exit 1
$ grep -nE '\bobject\b' crates/deform6/src/vb/runtime.rs            # exit 1
$ git status --porcelain corpus/                                    # empty
```

The last grep is stronger than the plan asks for. The word does not appear in
this file at all, in code, in a comment or in a string. It did once, in the
sentence that explains why the delay load table is not trusted. That sentence
now names `PeImage::delay_loaded_dlls` and points at its doc comment, which is
where the measurement lives.

`git status --porcelain corpus/` is empty, so the four patched fixtures and
the truncated one wrote nothing to disk.

## The five deliberate breakages

`AGENTS.md` says to break the thing a test covers and watch it fail. Five
breakages, five runs, and every one failed the test the plan predicts, first
time. Each was reverted and the gate was green before the commit. Neither
commit holds a broken state.

### 1. The name comparison stops folding case

`.find(|name| name.eq_ignore_ascii_case(VB6_DLL))` became
`.find(|name| name.as_str() == VB6_DLL)`.

One test failed. 66 passed.

```
---- vb::runtime::tests::a_lower_case_runtime_name_matches_and_comes_back_in_the_case_the_file_holds ----
panicked at crates/deform6/src/vb/runtime.rs:171:69:
called `Result::unwrap()` on an `Err` value: NoVbRuntime { dot_net: false }
```

### 2. The delay load list gets a vote

`imports.iter()` became `imports.iter().chain(delay_loaded.iter())`, and the
`let _ = delay_loaded;` line went with it.

One test failed. 66 passed.

```
---- vb::runtime::tests::a_delay_loaded_visual_basic_6_runtime_never_decides_the_runtime ----
panicked at crates/deform6/src/vb/runtime.rs:226:54:
called `Result::unwrap_err()` on an `Ok` value: (Vb6, "MSVBVM60.DLL")
```

The failure text is the useful part. The broken build does not merely accept
the file, it reports the runtime name it read out of the delay load table, so
a file that referenced the runtime only there would have printed a Runtime
line that looked exactly like a real one.

### 3. An unreadable import directory becomes "no runtime"

`Refusal::Damaged("the import directory is unreadable")` became
`Refusal::NoVbRuntime { dot_net: false }`.

One test failed. 74 passed.

```
---- vb::runtime::tests::a_truncated_file_is_damaged_and_not_a_file_without_a_runtime ----
panicked at crates/deform6/src/vb/runtime.rs:355:9:
an unreadable import directory is damage and exit code 4, and it is not the
exit code 2 that means no Visual Basic runtime: NoVbRuntime { dot_net: false }
```

### 4. An empty import list becomes damage

An arm was added before the call to `classify`:

```rust
if imports.is_empty() {
    return Err(Refusal::Damaged("the image imports nothing"));
}
```

One test failed. 74 passed.

```
---- vb::runtime::tests::an_image_with_no_import_data_directory_is_not_damaged ----
panicked at crates/deform6/src/vb/runtime.rs:371:9:
assertion `left == right` failed
  left: Damaged("the image imports nothing")
 right: NoVbRuntime { dot_net: false }
```

This breakage is the shape the plan asks for as a third arm. It fails the
test, which is the point, and it is also the reason the third arm is not in
the shipped code. See finding 3.

### 5. The .NET flag is hard coded

`classify(&imports, &delay_loaded, pe.has_clr_header())` became
`classify(&imports, &delay_loaded, false)`.

One test failed. **74 passed.**

```
---- vb::runtime::tests::a_common_language_runtime_directory_makes_the_refusal_name_a_dot_net_assembly ----
panicked at crates/deform6/src/vb/runtime.rs:386:9:
assertion `left == right` failed
  left: NoVbRuntime { dot_net: false }
 right: NoVbRuntime { dot_net: true }
```

The 74 is the number that matters, and it is the same finding plan 01-04 made
about `has_clr_header` itself. Data directory 14 is zero in all 44 corpus
files, measured above, so every other test in this file, and every other test
in the phase, passes while the flag is a constant. One fixture, built in
memory, is the whole difference between a flag that works and a flag that
looks like it works.

## The seventeen behaviours and the tests that hold them

| Behaviour | Test |
|---|---|
| `MSVBVM60.DLL` classifies as version 6, and the name comes back | `a_list_holding_the_visual_basic_6_runtime_classifies_as_version_6` |
| `msvbvm60.dll` classifies too, and comes back in the case the file holds | `a_lower_case_runtime_name_matches_and_comes_back_in_the_case_the_file_holds` |
| `MSVBVM50.DLL` gives `IsVb5` | `the_visual_basic_5_runtime_is_refused_by_name` |
| `VB40032.DLL` gives `IsVb4` | `the_32_bit_visual_basic_4_runtime_is_refused_by_name` |
| `KERNEL32.DLL` alone gives `NoVbRuntime` with the flag false | `a_list_with_no_visual_basic_runtime_is_refused` |
| The same list with the flag set gives the flag true | `the_dot_net_flag_travels_into_the_refusal` |
| An empty list gives `NoVbRuntime` | `an_empty_import_list_is_refused` |
| A delay load list holding the runtime still refuses | `a_delay_loaded_visual_basic_6_runtime_never_decides_the_runtime` |
| The matched name is the runtime entry, not the first entry | `the_matched_name_is_the_runtime_entry_and_not_the_first_entry` |
| The corpus file is version 6 | `the_corpus_file_uses_the_visual_basic_6_runtime` |
| The corpus file gives back `MSVBVM60.DLL` | `the_corpus_file_names_its_runtime_and_the_name_reaches_the_caller` |
| The name patched to `MSVBVM50.DLL` gives `IsVb5` | `a_patched_visual_basic_5_runtime_name_is_refused` |
| The name patched to `VB40032.DLL` and a NUL gives `IsVb4` | `a_patched_32_bit_visual_basic_4_runtime_name_is_refused` |
| The name patched to `KERNEL32.DLL` gives `NoVbRuntime`, flag false | `a_patched_name_that_is_no_visual_basic_runtime_is_refused` |
| The file truncated to half gives `Damaged` | `a_truncated_file_is_damaged_and_not_a_file_without_a_runtime` |
| No import data directory gives `NoVbRuntime`, not `Damaged` | `an_image_with_no_import_data_directory_is_not_damaged` |
| Data directory 14 set gives `NoVbRuntime` with the flag true | `a_common_language_runtime_directory_makes_the_refusal_name_a_dot_net_assembly` |

Nine of them take a literal list and never touch a file. Eight of them read
`Mandelbrot.exe` through `include_bytes!`.

## What the patched fixtures prove, and what they do not

The plan asks for this to be recorded plainly, and it should be.

**A patched import name proves the discrimination logic and nothing more.**
`a_patched_visual_basic_5_runtime_name_is_refused` proves that a file naming
`MSVBVM50.DLL` reaches `Refusal::IsVb5`. It does not prove that DeForm6
handles a genuine Visual Basic 5 binary, whose header layout differs after
0x30. It does not need to: DeForm6 refuses the file at the import name and
never reads that header.

**The directory 14 fixture is not a .NET assembly.** It is a Visual Basic 6
executable with one import name overwritten and eight bytes of a data
directory set. A real .NET assembly carries a common language runtime header
at that address, a metadata root, and streams. DeForm6 reads none of them,
because it refuses at exit code 2 first, and the sentence it prints is the
only thing the flag changes.

Both are the right shape for what this phase promises. Phase 5 adds a real
sample of each through the fetched manifest of pinned hashes, which is what
`AGENTS.md` requires for a binary the author does not own.

**`VB40016.DLL` has no test and can have none.** A 16 bit Visual Basic 4
program is an NE executable. `PeImage::parse` refuses it at
`Refusal::NotPe` and exit code 1, and plan 01-04 already tests that with
`an_ne_image_is_not_a_portable_executable`. The constant is kept and the doc
comment on it says that nothing compares against it. This is decision D-04,
and it is why code 3 did not need a sixth sibling.

## Findings that correct the plan

Four things in the plan did not survive contact. Three are contradictions
inside the plan itself, and one would not compile.

### 1. The signature of `runtime_of` contradicts the plan's own behaviour list

**Found during:** Task 2. **Rule 1, the plan states two incompatible things.**

The action says:

> Add `runtime_of(pe: &PeImage) -> Result<Runtime, Refusal>` to
> `vb/runtime.rs`.

Ten lines later the same action says:

> `runtime_of` returns the pair `classify` returns, so the matched runtime
> name reaches the caller.

Behaviour 8 of the same task says `runtime_of` returns `MSVBVM60.DLL`.
Success criterion 5 says the same. `must_haves.truths` says the matched name
travels out of the classifier so the sweep and the printed Runtime line both
read it from the file.

`Result<(Runtime, String), Refusal>` is written. Four statements against one,
and the four are the ones that carry the requirement. A `runtime_of` with the
literal signature would strand the name inside `classify`, and plan 01-08
could not observe it.

### 2. The task 2 action asks for six tests and the task lists eight behaviours

**Found during:** Task 2.

The behaviour block holds eight entries. The action says "Write the six
behaviours above as tests". The `<done>` block says "All eight behaviours
pass", and success criterion 1 says seventeen, which is nine plus eight. Eight
are written. The "six" appears to be a leftover from an earlier draft, before
the directory 14 case and the matched name case were added, and those two are
exactly the ones success criteria 4 and 5 name.

### 3. The reason given for three arms in `runtime_of` is not true

**Found during:** Task 2. **Rule 4 territory, decided against the plan and
recorded here.**

The action says:

> The three cases it can produce are distinct and each maps to a different
> answer, so write them as three arms rather than as one.

They do not map to three different answers. An error maps to `Damaged`. An
empty list maps to `NoVbRuntime`. A non-empty list with no Visual Basic
runtime in it also maps to `NoVbRuntime`, and that rule is the last line of
`classify`, which task 1 behaviour 7 already requires and
`an_empty_import_list_is_refused` already covers. A third arm in `runtime_of`
would be a second copy of one rule, in a second file position, free to drift
from the first.

`runtime_of` has two arms. The error becomes `Damaged`, and every list goes to
`classify`.

**There is a second reason, and it is the stronger one.** The plan's third arm
is written as "an empty list means `Refusal::NoVbRuntime`", with no mention of
the flag. Written that way it would report `dot_net: false` for an assembly
that has a common language runtime directory and no import directory, and no
test in the plan would catch it, because the plan's directory 14 fixture keeps
its import directory. The two arm shape cannot make that mistake: there is one
construction site for `NoVbRuntime` in the crate's decision path, and the flag
reaches it.

Breakage 4 above is the plan's third arm, in its worst form, and the existing
test fails on it. The distinction the plan wants is enforced. It is enforced
by the arm that is there rather than by the arm that is not.

### 4. `classify` as the plan describes it does not compile under the gate

**Found during:** Task 1. **Rule 3, a blocking issue.**

The action says the `delay_loaded` argument "is taken and is deliberately not
used in the decision", and says nothing about how. A function parameter that
no expression reads is `unused_variables`, which is a rustc warning, and
`cargo clippy --all-targets -- -D warnings` turns it into an error. `AGENTS.md`
also forbids adding a lint allowance to get the wall to pass.

`classify` holds one statement, with the reason in a comment beside it:

```rust
let _ = delay_loaded;
```

The alternative was to rename the parameter `_delay_loaded`. That compiles and
it puts an underscore in the public signature that every reader of the
generated documentation sees, on a parameter the doc comment spends a
paragraph explaining is deliberate. The discarding statement keeps the name
honest and puts the explanation where an editor who reaches for the list will
be standing.

## Deviations from Plan

### 1. The "no import data directory" fixture patches a data directory

**Found during:** Task 2. **A deviation, not a defect in the plan.**

The plan says:

> For the "no import data directory" case, build the input inside the test
> rather than patching a corpus file.

`read/pe.rs` already holds `a_small_image(with_imports: bool)`, a 40 line
builder for exactly this case. It is private to that file's test module and
nothing outside it can call it. Following the instruction literally meant
copying those 40 lines into a second test module, where the two copies would
then drift, and `AGENTS.md` says a fixture the author edits is a thing a test
must not lean on.

Instead, one helper, `with_data_directory(data, index, address, size)`, serves
both cases in this file. Index 1 with two zeroes takes the import data
directory away. Index 14 with a real section address and `0x48` adds the
common language runtime directory. The helper asserts that the write changes
the entry, so a fixture that has silently become a no-op fails loudly rather
than passing for the wrong reason. `an_image_with_no_import_data_directory_is_not_damaged`
also asserts that `imported_dlls` really is empty before it asserts the
refusal, so the test cannot pass because the directory patch missed.

The property under test is "an empty import list is not damage". Both
fixtures reach it the same way, through `Ok(None)` from the import walk. This
one reaches it on a real file as well.

### 2. `Refusal::Damaged` is matched by variant, not by payload

**Found during:** Task 2.

`a_truncated_file_is_damaged_and_not_a_file_without_a_runtime` asserts
`matches!(refusal, Refusal::Damaged(_))` rather than comparing against the
literal sentence. `Refusal::Damaged` holds a `&'static str` and derives
`PartialEq`, so an equality test would pin the wording. `CONTEXT.md` says the
refusal tests exist so "a reworded sentence does not fail the suite and a
changed meaning does". The property is exit code 4 against exit code 2, and
the variant is that property. The assertion message names both codes, and
breakage 3 shows it printing the wrong one.

## Threat mitigations

| Threat | State |
|---|---|
| T-01-20, the rejected signature byte scan | Not implemented. The file holds no byte literal for the magic, no `u32` for it, and no sliding window over section bytes. The grep in success criterion 6 is clean. The doc comment on `classify` says why, and says which phase and which flag it belongs behind. |
| T-01-21, the delay load table | The list is taken as an argument and appears in no decision branch. Breakage 2 shows the test that holds the property, and shows that a build which gave the table a vote would print a runtime name read out of it. |
| T-01-22, a packed or rewritten import directory | Accepted, and stated. The doc comment on `runtime_of` carries the three sub-cases and the answer to each, and says do not add a fallback here. |
| T-01-23, the name comparison | `eq_ignore_ascii_case`, in three places. `grep -qE '\bto_uppercase\b'` over this file exits 1. Breakage 1 shows the test that holds the fold. |

## Known Stubs

None. Both functions the plan names are implemented, and all seventeen
behaviours have a test.

Two things are stated in the source rather than papered over.

- **`VB4_16_DLL` is compared against nothing and can be.** The doc comment on
  the constant says so and says why. See the fixtures section above.
- **The tests here read one corpus file, not 44.** A test module inside
  `src/` cannot walk a directory, because the library names no file system
  type and the grep that proves it does not know a test module from library
  code. Plan 01-08 owns the sweep.

## Deferred Issues

None.

## Threat Flags

None. This plan adds no network endpoint, no auth path and no file access
path. `grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/` finds
nothing.

No package was added. `git diff a7e4e24 HEAD -- Cargo.toml Cargo.lock` is
empty.

## What this plan did not touch

`STATE.md`, `ROADMAP.md` and `REQUIREMENTS.md` were not edited. Plan 01-05
runs in the same wave and shares them, so the orchestrator owns them after the
merge. This is what plans 01-03 and 01-04 did for the same reason.
`crates/deform6/src/vb/header.rs`, `vb/mod.rs`, `lib.rs`, `error.rs` and
`read/pe.rs` were not edited either. No file outside
`crates/deform6/src/vb/runtime.rs` is in either commit.

`.planning/WINDOWS.md` gained no entry, because this plan produced no stub, no
skipped test and no verification it could not run.

## Commits

| Commit | Subject |
|---|---|
| `9ff801a` | Classify the imported Visual Basic runtime by name |
| `320fc17` | Map the import directory result to a refusal |
| (this file) | Record what plan 01-06 built and the four contradictions in its plan |

`git rev-list --count a7e4e24..HEAD` is 2 at the time the two code commits
landed. Each commit holds one task, and each holds its code and its tests
together, as `AGENTS.md` requires.

## Self-Check: PASSED

`crates/deform6/src/vb/runtime.rs` exists and is 486 lines. Both commit hashes
resolve with `git log --oneline <hash> -1`. `git diff --diff-filter=D
--name-only a7e4e24..HEAD` is empty, so neither commit deleted a tracked file.
`git show --stat` on each commit names one file. Every command in the gate
table was run and its exit status is recorded above.
