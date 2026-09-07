---
phase: 01-it-reads-the-file
plan: 01
subsystem: build
tags: [workspace, lint-wall, toolchain, ci]
status: complete

requires: []
provides:
  - the cargo workspace and its two crates
  - the lint wall, in one place, proved to fire
  - the pinned toolchain
  - the module tree that plans 01-02 to 01-07 fill
  - scripts/prove-lint-wall.sh
  - .github/workflows/gate.yml
affects:
  - every later plan in phase 1

tech_stack:
  added:
    - object 0.40.0, default-features = false, features read_core, pe, std
    - thiserror 2.0.20
    - serde 1.0.229, feature derive
    - clap 4.6.6, default-features = false, features std, derive, help, error-context
  patterns:
    - the lint wall lives in [workspace.lints], and each crate carries [lints] workspace = true
    - no crate root repeats the deny list as an inner attribute

key_files:
  created:
    - Cargo.toml
    - Cargo.lock
    - rust-toolchain.toml
    - crates/deform6/Cargo.toml
    - crates/deform6/src/lib.rs
    - crates/deform6/src/error.rs
    - crates/deform6/src/journal.rs
    - crates/deform6/src/read/mod.rs
    - crates/deform6/src/read/region.rs
    - crates/deform6/src/read/pe.rs
    - crates/deform6/src/vb/mod.rs
    - crates/deform6/src/vb/header.rs
    - crates/deform6/src/vb/runtime.rs
    - crates/deform6/src/vb/project.rs
    - crates/deform6-cli/Cargo.toml
    - crates/deform6-cli/src/main.rs
    - scripts/prove-lint-wall.sh
    - .github/workflows/gate.yml
  modified: []

decisions:
  - The proof script reads the clippy diagnostics as json, not as the short format the plan named. The short format prints the message of a lint and not its name, and the default format names a lint once only.
  - The probe function takes three integers, not two. Two integers cannot hold a u64 addition, a u64 division, a u64 to u32 cast and an i32 to u32 cast at the same time.
  - Each bad line in the probe carries a marker comment, and the script finds its line number by that marker. The line numbers are therefore not written into the script twice.

metrics:
  duration: 1 session
  completed: 2026-09-07

actuals:
  tokens: 6000
  tasks: 3
  commits: 4
plan_head_before: 20e540e72e0798f03ae10fe1cb02be64859f8897
---

# Phase 01 Plan 01: The workspace and the lint wall Summary

A two crate cargo workspace on a pinned 1.97.1 toolchain, with the deny wall in
one place and a committed script that demonstrates the wall stopping eight bad
lines rather than asserting that it does.

## What this plan built

`Cargo.toml`, `rust-toolchain.toml` and the two crate manifests are copied from
`RESEARCH.md` sections 1.1, 1.2, 1.3 and 1.6 without a change. `Cargo.lock`
holds 17 packages and is committed, because the product is a binary.

The library declares the whole module tree that this phase fills. Each stub
holds one `//!` line that names the plan that fills it. The tree is created in
one commit so that no two plans in one wave edit one `mod.rs`.

`scripts/prove-lint-wall.sh` writes a probe example, runs clippy, checks that
each bad line is stopped by the lint that must stop it, removes the probe, and
then runs the three gate commands.

`.github/workflows/gate.yml` runs the three gate commands as three separate
steps, then the proof script.

## The observed output of scripts/prove-lint-wall.sh

Run by hand on this commit. Exit status 0.

```
The probe is at crates/deform6/examples/lint_wall_probe.rs. Running clippy on it.

PASS  line 7   clippy::indexing_slicing         an index of a slice with square brackets
PASS  line 8   clippy::indexing_slicing         a range slice with square brackets
PASS  line 9   clippy::arithmetic_side_effects  an addition with the plus operator
PASS  line 10  clippy::unwrap_used              a call to unwrap on an Option
PASS  line 11  clippy::cast_possible_truncation a cast from u64 to u32
PASS  line 12  clippy::cast_sign_loss           a cast from i32 to u32
PASS  line 13  clippy::integer_division         an integer division
PASS  line 14  unsafe_code                      an unsafe block
```

One line per bad shape. Eight lines, seven distinct lints, five categories.
The three names that ROADMAP success criterion 4 requires are `d[0]`, which
line 7 covers, `a + b` on a file derived value, which line 9 covers, and
`.unwrap()`, which line 10 covers.

The exact message of each lint, read from the clippy json output, matches the
table in `RESEARCH.md` section 2.3 word for word:

| Line | Lint | Message |
|---|---|---|
| 7 | `clippy::indexing_slicing` | indexing may panic |
| 8 | `clippy::indexing_slicing` | slicing may panic |
| 9 | `clippy::arithmetic_side_effects` | arithmetic operation that can potentially result in unexpected side-effects |
| 10 | `clippy::unwrap_used` | used `unwrap()` on an `Option` value |
| 11 | `clippy::cast_possible_truncation` | casting `u64` to `u32` may truncate the value |
| 12 | `clippy::cast_sign_loss` | casting `i32` to `u32` may lose the sign of the value |
| 13 | `clippy::integer_division` | integer division |
| 13 | `clippy::arithmetic_side_effects` | arithmetic operation that can potentially result in unexpected side-effects |
| 14 | `unsafe_code` | usage of an `unsafe` block |

## The proof script can fail

`AGENTS.md` says to break the thing a test covers and watch it fail. The wall
was relaxed for one measurement only, and it was not committed in that state.

With `clippy::indexing_slicing` allowed on the clippy command line, the two
pairs `7|clippy::indexing_slicing` and `8|clippy::indexing_slicing` are absent
from the observed set, and the check the script runs on them returns 1. The
script would print two FAIL lines and exit 1. The remaining seven observations
were still present, so the failure was local to the lint that was relaxed.

## The gate

All three commands, run on this commit in the working tree and again on a
clean clone of it. Every one exits 0.

| Command | Working tree | Clean clone |
|---|---|---|
| `cargo fmt --all --check` | 0 | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 | 0 |
| `cargo test --workspace` | 0 | 0 |
| `sh scripts/prove-lint-wall.sh` | 0 | 0 |

`cargo test --workspace` runs three test binaries and reports
`0 passed; 0 failed` for each. There is no test yet. Plan 01-02 adds the first.

The clean clone was made with `git clone` into a fresh directory, and its tree
was clean after the proof script ran.

## Other measurements

- `cargo tree -p deform6 --depth 0` and `cargo tree -p deform6-cli --depth 0`
  both succeed and both report a path source.
- `cargo metadata --no-deps --format-version 1` writes zero bytes to stderr, so
  there is no resolver warning.
- `git ls-files Cargo.lock` prints `Cargo.lock`.
- `cargo fetch` locked 17 packages.

## Deviations from Plan

### 1. The script reads json, not the short message format

**Found during:** Task 2.

**Issue:** The plan says to run
`cargo clippy --all-targets --message-format short` and to assert that the
output names each of the seven lints. Measured: the short format prints the
message of a lint and not its name. The name is absent from every line:

```
crates/deform6/examples/lint_wall_probe.rs:4:18: error: indexing may panic
```

The default format does print the name, but only once for each lint, in a
`= note: requested on the command line with ...` line. It fired twice for
`clippy::indexing_slicing` and named it once, so the default format cannot say
which line each lint caught either.

**Fix:** The script runs `--message-format json` and reads the `code.code`
field and the primary span of each diagnostic. This is a stronger instrument
than the plan asked for. It proves a lint caught a named line, not merely that
a lint fired somewhere in the file.

**Cost:** The script now needs `jq`. It checks for `jq` and exits with an
explanation if it is absent. The GitHub ubuntu runner image already holds `jq`.

**Files modified:** `scripts/prove-lint-wall.sh`.

### 2. The probe function takes three integers, not two

**Found during:** Task 2.

**Issue:** The plan says the probe holds "one function that takes a byte slice
and two integers". The eight lines the plan requires cannot fit in two
integers. `RESEARCH.md` section 2.3 asks for `a + b` and `a / b`, which need
two integers of the same type, and for `a as u32` where `a: u64` and
`a as u32` where `a: i32`, which need a signed integer as well.

A first draft used `a + a` and `a / a` to stay inside two parameters. That
draft fired `clippy::eq_op`, "equal expressions as operands to `/`", which is
a lint the wall does not configure and which the probe does not test.

**Fix:** The probe takes `d: &[u8], a: u64, b: u64, c: i32`. Every line then
matches the shape in the `RESEARCH.md` table, and no extra lint fires.

**Files modified:** the probe inside `scripts/prove-lint-wall.sh`.

### 3. The unwrap line unwraps a slice lookup, not `Some(a)`

**Found during:** Task 2.

**Issue:** The first draft wrote `Some(a).unwrap()`. That fires
`clippy::unnecessary_literal_unwrap` as well, which is another lint the wall
does not configure.

**Fix:** The line is `d.first().unwrap()`. It fires `clippy::unwrap_used` only,
and its message is the one `RESEARCH.md` records.

**Files modified:** the probe inside `scripts/prove-lint-wall.sh`.

### 4. Task 1 is two commits, and the first one carries the two crate roots

**Found during:** Task 1.

**Issue:** The plan asks for a commit that holds the manifests and a second
commit that holds the module skeleton. The gate must pass before every commit.
A commit that holds manifests alone does not pass: cargo cannot read a manifest
for a library whose `src/lib.rs` is absent, so `cargo fmt --all --check` fails
before it starts.

**Fix:** The first commit carries the smallest `lib.rs` and `main.rs` that let
cargo read the workspace. `lib.rs` holds `#![forbid(unsafe_code)]` and
`main.rs` holds `fn main() {}`. The second commit adds the module tree, the
`mod` declarations and the `//!` lines. Both commit subjects are the ones the
plan gives.

**Files modified:** `crates/deform6/src/lib.rs`,
`crates/deform6-cli/src/main.rs`.

## Findings that correct the sources

### RESEARCH.md section 2.3 says eight errors. Cargo reports nine.

`RESEARCH.md` tallies eight errors for the probe. Its tally lists clippy errors
only. The `unsafe_code` error is a rustc error, not a clippy one, and it is not
in the tally. Cargo counts all of them:

```
error: could not compile `deform6` (example "lint_wall_probe") due to 9 previous errors
```

The plan repeats the eight and explains it as "the division line fires two
lints", which is true and is not the whole reason. The numbers that hold are:
five categories, eight lines, seven distinct lints, nine errors. The script
prints the count it measures, so this does not have to be reconciled again.

### The `exclude` line for the fuzz crate is not load-bearing

Not re-measured here. `RESEARCH.md` section 1.5 measured all four combinations
and found the line makes no difference on cargo 1.97.1. The line is kept
because it states the intent, and no check was written that depends on it, as
the plan instructs.

## Known Stubs

Ten files are stubs by design. Each holds one `//!` line and no code. The plan
that fills each one is named in the file and below. They are not a defect: the
tree exists so that two plans in one wave never edit one `mod.rs`.

| File | Filled by |
|---|---|
| `crates/deform6/src/error.rs` | 01-03 |
| `crates/deform6/src/journal.rs` | 01-03 |
| `crates/deform6/src/read/region.rs` | 01-02 |
| `crates/deform6/src/read/pe.rs` | 01-04 |
| `crates/deform6/src/vb/header.rs` | 01-05 |
| `crates/deform6/src/vb/runtime.rs` | 01-06 |
| `crates/deform6/src/vb/project.rs` | 01-07 |
| `crates/deform6-cli/src/main.rs` | 01-08 |

`crates/deform6/src/read/mod.rs` and `crates/deform6/src/vb/mod.rs` hold their
`pub mod` lines and nothing else, which is all they are meant to hold.

## Threat Flags

None. This plan adds no network endpoint, no auth path and no file access
path. The three mitigations the plan's threat register names are in place:

- T-01-01: `arithmetic_side_effects = "deny"` and `overflow-checks = true`.
- T-01-02: `indexing_slicing = "deny"` and `unwrap_used = "deny"`, both proved.
- T-01-03: `unsafe_code = "forbid"`, proved, and `#![forbid(unsafe_code)]` in
  `lib.rs`.

## Commits

| Commit | Subject |
|---|---|
| `ea2b9b0` | Add the cargo workspace, the pinned toolchain and the lint wall |
| `bff482c` | Add the library module skeleton and the command line stub |
| `ce68d07` | Add a script that proves the lint wall rejects the five bad shapes |
| `828d162` | Add the continuous integration gate |
