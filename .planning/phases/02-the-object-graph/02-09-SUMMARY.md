---
phase: 02-the-object-graph
plan: 09
subsystem: test-harness
tags: [ratios, xtask, ver-05, d-11, d-12, t-02-45]
status: complete

requires:
  - program_counts, ProgramCounts (plan 02-08's differential.rs), the one
    function this plan reuses rather than recomputing
  - support::vbp, support::source (plan 02-07/02-08), the independent
    readers program_counts calls for the declared side
provides:
  - tests/ratios.toml, the pinned recovery ratio: 185 recovered, 904
    declared, over 44 programs, 24 distinct ratios from 0.00 to 0.71
  - crates/deform6/tests/ratios.rs, the gate: parses the pin, computes the
    same two counts through differential::program_counts, and fails with
    REGRESSION (pin claims more than measured) or MOVED UP (measured
    exceeds the pin), never the other pairing
  - crates/xtask, cargo run -p xtask -- update-ratios, which rewrites the
    pin through the same format_entry function the MOVED UP message prints
affects:
  - Phase 3 and Phase 4, which raise entries in this file rather than
    replacing it (per the plan's own reversibility rating)

tech_stack:
  added:
    - "toml 1.1.5+spec-1.1.0 (crates.io, approved in RESEARCH.md's package
      legitimacy audit), used in crates/xtask only, as a second,
      independent validity check that the hand-rendered file is syntactically
      correct TOML before it is written -- not as the primary reader or
      writer, which stays the hand-rolled format shared with the gate (see
      Deviations, item 7)"
  patterns:
    - "one #[path] embedding chain per binary: crates/xtask embeds
      crates/deform6/tests/ratios.rs, which itself embeds differential.rs;
      xtask never declares its own, second, parallel #[path] to
      differential.rs, because two independent embeddings of the same file
      compile as two distinct, incompatible types with the same name
      (confirmed directly: E0308 on ProgramCounts, see Deviations, item 4)"
    - "pub(crate) widening, not duplication, to cross a #[path] embedding's
      module boundary: a private item is invisible to the parent module an
      embedding #[path] creates, so every item a downstream embedder needs
      (program_counts, ProgramCounts and its fields, recovered_objects, the
      nested support module, plus this plan's own declared_total,
      ratios_toml_path and HEADER) is pub(crate), never re-implemented"
    - "one shared formatting function, format_entry, for both the MOVED UP
      failure message and the file the writer produces, so the two can
      never disagree over the TOML grammar by construction (T-02-45)"

key_files:
  created:
    - tests/ratios.toml
    - crates/deform6/tests/ratios.rs
    - crates/xtask/Cargo.toml
    - crates/xtask/src/main.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/deform6/tests/differential.rs

decisions:
  - "cargo add --package xtask toml, run exactly as the plan's action text
    specifies, places the dependency in crates/xtask/Cargo.toml's own
    [dependencies] as a plain version string -- not in
    [workspace.dependencies] as the same paragraph also requires. Both
    cannot be followed literally by one command. Resolved by running the
    command for the resolved version (1.1.5+spec-1.1.0, recorded rather
    than hand-typed), then relocating that exact string to
    [workspace.dependencies] and changing crates/xtask/Cargo.toml to
    toml.workspace = true, so the root manifest still gains exactly the one
    dependency line the plan's own enumeration promises."
  - "Task 3's <files> list (crates/xtask/src/main.rs,
    crates/deform6/tests/ratios.rs) omits crates/xtask/Cargo.toml, but
    update-ratios cannot call program_counts without xtask depending on
    deform6. Added deform6.workspace = true to crates/xtask/Cargo.toml as
    part of task 3's commit, the minimal change the task's own action text
    already implies (\"computes the two counts for each program with the
    library\")."
  - "Reusing plan 02-08's program_counts and ProgramCounts from
    crates/deform6/tests/ratios.rs (task 2) and from crates/xtask (task 3)
    requires those items, and the nested support module, to be at least
    pub(crate) in differential.rs: a private item is invisible to the
    parent module a #[path] embedding creates, so the reuse mechanism
    02-08-SUMMARY.md itself documents (#[path = \"../differential.rs\"] mod
    differential;) cannot compile without this. differential.rs is not in
    this plan's own files_modified list. Widened program_counts,
    ProgramCounts and its four fields, recovered_objects, and the mod
    support declaration from private to pub(crate), with no logic change,
    across two commits (task 2 and task 3), each documented in its own
    commit body. differential.rs's own seven tests were re-run after each
    change and still pass unmodified."
  - "Two independent #[path] embeddings of the same file compile as two
    distinct, incompatible types with the same name. crates/xtask's first
    draft declared its own #[path = \"../../deform6/tests/differential.rs\"]
    mod differential;, alongside its #[path =
    \"../../deform6/tests/ratios.rs\"] mod ratios; (which itself embeds
    differential.rs), and calling ratios::declared_total(&counts) with a
    ProgramCounts built from the first embedding failed to compile: E0308,
    \"ratios::differential::ProgramCounts and differential::ProgramCounts
    have similar names, but are actually distinct types\". Fixed by
    deleting xtask's own direct embedding and reaching program_counts,
    ProgramCounts and recovered_objects exclusively through
    ratios::differential -- one embedding of differential.rs in the whole
    binary, not two."
  - "differential.rs's own #[test] functions, and the handful of imports
    (HashMap, classify::self, support::source) only those function bodies
    use, are exempt from clippy's dead_code lint in every build (the
    compiler treats #[test] items as always-reachable roots regardless of
    whether --test is passed), but their BODIES compile out entirely under
    a non-test embed, so imports referenced only inside them do trigger
    unused_imports there. Observed directly: cargo build -p xtask warned on
    exactly those three imports, and nothing else, the first time
    differential.rs was embedded into a non-test binary. Fixed with
    #[allow(unused_imports, dead_code, reason = ...)] on the mod
    declarations in ratios.rs and crates/xtask/src/main.rs, the same shape
    support/mod.rs's own module doc comment already documents for the
    identical per-binary-subset reason."
  - "RESEARCH.md's own suggestion (\"toml read/write for ratios.toml ... a
    hand-rolled parser will get quoting wrong\") points at the toml crate
    as the primary reader and writer. Task 3's own, later requirement is
    stronger and is in the threat model as T-02-45: the MOVED UP message
    and the writer must share one function, so the two can never disagree.
    A serde-based serializer as the writer, alongside a hand-formatted
    string in the failure message, cannot give that guarantee without
    itself becoming the shared function -- and crates/deform6 is not
    allowed to gain a toml dev-dependency (RESEARCH.md: \"this phase adds
    exactly one dependency: toml, for crates/xtask alone\"). Resolved by
    keeping the hand-rolled format_entry/parse_ratios_toml pair (matching
    support/vbp.rs's own established \"match the shape of the file\"
    precedent) as the single source of the file's shape, shared by both the
    message and the writer, and using the toml crate in crates/xtask only
    as a second, independent check that the string format_entry rendered
    parses as valid TOML before it is written -- meaningfully used, never
    the primary mechanism."
  - "No #[test] in crates/xtask calls run([\"update-ratios\"]) against the
    real, committed tests/ratios.toml: cargo test --workspace runs separate
    test binaries concurrently by default, and crates/deform6/tests/ratios.rs
    reads that same file in a sibling process, so a #[test] here that
    writes it is a real, not hypothetical, race. Behaviours one through
    three (rewrites and reports the count; a clean-tree rerun produces no
    diff; an edited entry is restored and the change reported) are instead
    proven by the plan's own sequential acceptance command, run for real
    multiple times in this session, including through both deliberate
    breakages below."

metrics:
  duration: 1 session
  completed: 2026-09-08

actuals:
  tokens: 16466
  tasks: 3
  commits: 3
plan_head_before: 73e6cc216432641cca49f94313fc0466bf8196d8
---

# Phase 02 Plan 09: The recovery ratio pin Summary

`tests/ratios.toml` pins the procedure recovery of every one of the 44
corpus programs -- 185 recovered, 904 declared, 24 distinct ratios from
0.00 to 0.71 -- against a gate in `crates/deform6/tests/ratios.rs` that
fails `REGRESSION` when a pin claims more than the tool measures and
`MOVED UP` when the tool measures more than the pin claims, and
`cargo run -p xtask -- update-ratios` rewrites the file through the exact
function that message prints.

## What this plan built

| Item | What it gives |
|---|---|
| Task 1: `crates/xtask` | A fourth workspace member, `toml` added to `[workspace.dependencies]`, a hand-parsed `update-ratios`/`--help` argument dispatch, both proof scripts re-run and green after the new member |
| Task 2: `tests/ratios.toml` + `crates/deform6/tests/ratios.rs` | The pinned file (44 entries, totals 185/904) and the seven behaviours: the gate passes on the committed file; raising a count fails `REGRESSION`; lowering fails `MOVED UP` with the exact paste block; an edited ratio fails against its own counts; a stale key and a missing key fail with two different messages, proven against the real gate loop, not just the message strings |
| Task 3: `update-ratios` | Walks the corpus once through `program_counts`, renders the file through `format_entry` (the same function the `MOVED UP` message calls), refuses to write fewer than 44 programs, and is idempotent on a clean tree |

15 tests in `crates/deform6/tests/ratios.rs` (7 of its own, plus
`differential.rs`'s 7 re-run because `ratios.rs` embeds it; see
Deviations). The workspace test count rose from 252 to 289 (197 lib, 1
corpus_sweep, 7 differential, 15 ratios, 9 refusal, 27 support_selftest, 2
type_descriptors, 9 cli, 22 xtask; 0 doc-tests).

## The pinned numbers

44 programs, keyed by their path relative to `corpus/`. Totals: **185
recovered, 904 declared**. 24 distinct ratios, ranging from **0.00 to
0.71**. Both totals and every individual entry come from
`differential::program_counts`, computed fresh for this plan and matching
plan 02-08's own per-program table exactly (cross-checked against
`02-08-SUMMARY.md`'s table before writing the file).

## The pairing, stated once, for Phase 3 to reuse without reversing it

Both words describe what happened to **the tool**, never to the file.

- Editing a pinned number **up** means the pin now claims more than the
  tool recovers: something the pin expects went missing. That is
  `REGRESSION`.
- Editing a pinned number **down** means the tool now recovers more than
  the pin claims: the tool moved up. That is `MOVED UP`.

## The two failure messages, verbatim

Produced by editing `tests/ratios.toml`'s committed `Grayscale.exe` entry
in place and running `cargo test -p deform6 --test ratios -- --nocapture
the_gate_passes_on_the_committed_file`, then restoring the file with `git
checkout -- tests/ratios.toml` (confirmed clean afterward: `git diff
--exit-code -- tests/ratios.toml` exits 0).

**Raising `recovered` from 12 to 13 (`REGRESSION`):**

```
vb6-code/Grayscale-effect/Grayscale.exe: REGRESSION: the pin claims 13 recovered, the tool recovers 12. Procedures the source declares that the tool did not name: nothing the source declares is unrecovered; the tool named every public procedure this program's source declares, so this pin disagrees with a tool that lost nothing
vb6-code/Grayscale-effect/Grayscale.exe: the stored ratio "0.35" does not equal "0.38", recomputed from the pinned counts 13 and 34
```

**Lowering `recovered` from 12 to 11 (`MOVED UP`):**

```
vb6-code/Grayscale-effect/Grayscale.exe: MOVED UP: the pin claims 11 recovered, the tool recovers 12. Paste this block into tests/ratios.toml:
["vb6-code/Grayscale-effect/Grayscale.exe"]
recovered = 12
declared = 34
ratio = 0.35

vb6-code/Grayscale-effect/Grayscale.exe: the stored ratio "0.35" does not equal "0.32", recomputed from the pinned counts 11 and 34
```

The missing-procedures list is empty on all real corpus data (185 of 185
match in both directions, per `differential.rs`), so `REGRESSION` always
carries the explanatory sentence rather than an empty list -- exactly the
shape the plan's action text anticipates.

## The root manifest, exactly

Two lines added, nothing else touched:

```diff
-members = ["crates/deform6", "crates/deform6-cli"]
+members = ["crates/deform6", "crates/deform6-cli", "crates/xtask"]
```
```diff
 clap = { version = "4.6.6", ... }
+toml = "1.1.5"
```

`exclude = ["crates/deform6/fuzz"]`, both `[workspace.lints]` tables,
`[profile.release]` and `[workspace.package]` are byte-for-byte unchanged.
`crates/xtask/Cargo.toml` carries `[lints]\nworkspace = true`. Both proof
scripts were re-run after Task 1's change and again after every later
commit; each exited 0 every time (see the Gate table).

**Resolved dependency version:** `toml v1.1.5+spec-1.1.0`, from
`cargo add --package xtask toml`, never hand-typed. `RESEARCH.md`'s package
legitimacy audit approves it (crates.io, ~11 years, 16.3M weekly
downloads, `github.com/toml-rs/toml`); no human verification checkpoint
applied.

## Deviations from Plan

All five recorded in full under `decisions` in the frontmatter above. In
short:

1. `cargo add --package xtask toml` places the dependency in
   `crates/xtask/Cargo.toml`'s own table, not `[workspace.dependencies]`;
   relocated the resolved version string by hand afterward.
2. Task 3's `<files>` list omits `crates/xtask/Cargo.toml`; added
   `deform6.workspace = true` there, required to call `program_counts`.
3. Widened `program_counts`, `ProgramCounts` and its fields,
   `recovered_objects`, and `mod support` in `differential.rs` (not in
   this plan's `files_modified`) from private to `pub(crate)`, the minimal
   change the reuse mechanism `02-08-SUMMARY.md` itself documents actually
   requires to compile.
4. Two independent `#[path]` embeddings of `differential.rs` in one binary
   compile as two distinct, incompatible `ProgramCounts` types (E0308,
   confirmed directly); fixed by giving `crates/xtask` exactly one
   embedding chain, through `ratios::differential`.
5. `differential.rs`'s own `#[test]` function bodies compile out entirely
   under a non-test embed, making the three imports only they use warn as
   unused; fixed with the same `#[allow(unused_imports, dead_code,
   reason = ...)]` shape `support/mod.rs` already documents.
6. `RESEARCH.md` suggests the `toml` crate as the primary reader/writer;
   task 3's own stronger requirement (one shared function for the message
   and the writer, T-02-45) is incompatible with that, so the hand-rolled
   `format_entry`/`parse_ratios_toml` pair stays the primary mechanism and
   `toml` is used only as a second, independent validity check on the
   rendered output.
7. No `#[test]` in `crates/xtask` runs `update-ratios` against the real
   committed file, because `cargo test --workspace` runs test binaries
   concurrently and such a test would race `crates/deform6/tests/ratios.rs`'s
   own tests reading the same file; behaviours one through three are
   proven by the plan's own sequential acceptance command instead, run for
   real multiple times in this session.

None of the seven touch `crates/deform6/src/` or `crates/deform6-cli/`.

## Side effect: the test count is inflated by design, not by accident

Because `crates/deform6/tests/ratios.rs` embeds `differential.rs` via
`#[path]`, and `crates/xtask` embeds `ratios.rs` the same way,
`differential.rs`'s 7 tests run three times across the workspace (once as
`--test differential`, once nested inside `--test ratios`, once nested
again inside `-p xtask`'s unit tests), and `ratios.rs`'s 15 tests run
twice (once as `--test ratios`, once nested inside `-p xtask`). This is
not three independent proofs; it is the same tests re-executed because
they physically live in a file each binary embeds. Documented in both
files' own module doc comments rather than left as a silent surprise in
the reported count.

## The deliberate breakages (five required, five run)

All run in the working tree, observed, then restored; `git diff --exit-code
-- tests/ratios.toml` and `cargo clippy --all-targets -- -D warnings`
confirmed a clean, green tree after every restoration.

### Task 2, breakage 1: the two words swapped

Swapped which constant (`REGRESSION`/`MOVED_UP`) each direction's message
prints. Both `raising_a_pinned_recovered_count_fails_with_regression` and
`lowering_a_pinned_recovered_count_fails_with_moved_up_and_the_exact_paste_block`
failed immediately, each printing the wrong word. Restored; both pass
again.

### Task 2, breakage 2: comparing rounded ratios instead of the counts

The obvious version of this breakage (raise/lower `Grayscale.exe`'s
pinned `recovered` by one, leaving `ratio_text` untouched at its original,
now-stale value) still produced a failure -- but from the *separate*
ratio-consistency check (behaviour six), not from the recovered-count
comparison the breakage targets, because the corpus's declared counts are
all under 100 and a ±1 change never happens to round to the same two
decimal places. Per `AGENTS.md` ("a deliberate breakage that produces no
failure means the covering test does not exist"), built a synthetic,
isolated case instead (a temporary test, removed before commit): pinned
`recovered: 100, declared: 1000, ratio_text: "0.10"` against a measured
`101, 1000` (ratio `0.101`, which also rounds to `"0.10"`). Under the
ratio-based comparison this produced **zero failures**, confirming the
bug lets a real one-count divergence through whenever the rounded ratio
happens to coincide. Restored to comparing the raw integer counts.

### Task 2, breakage 3: dropping the check that every corpus program has a key

The first version of this breakage's *covering test* only compared
`stale_key_message(x) != missing_key_message(x)` as strings -- which would
keep passing even if the gate stopped calling either function, exactly the
kind of test that cannot fail `AGENTS.md` warns is worse than none.
Refactored the gate's key-set logic into a shared `gate_failures` function
both `the_gate_passes_on_the_committed_file` and the covering tests call,
then re-ran the breakage (commented out the missing-key push) against the
real function: `a_missing_key_fails_the_real_gate` and
`a_stale_key_and_a_missing_key_fail_with_two_different_messages` both
failed with `deleting a real entry ... must produce a failure: []`,
confirming the check's absence is now genuinely caught. Restored.

### Task 3, breakage 1: entries written in walk order, not sorted

`support::vbp::executables()` already sorts its own output, so simply
deleting `measure_all`'s `out.sort_by(...)` line left the vector sorted
anyway and produced **no observable difference** on rerun -- the fourth
time this exact "no failure" shape has appeared in this phase. Replaced
the deletion with `out.reverse()` to stand in for "some other walk order"
and re-ran `cargo run -p xtask -- update-ratios`: `git diff --exit-code --
tests/ratios.toml` then exited 1, with a 298-line diff reordering every
entry while every value stayed identical -- exactly the shape "never
reorders the file" exists to refuse. Restored; a rerun is byte-for-byte
identical again (`git diff --exit-code` exits 0).

### Task 3, breakage 2: the short-file guard removed

Commented out `check_minimum_program_count(measured.len())?;` and stood in
for "the walk found a directory holding two programs" by truncating the
real 44-program walk to 2. `cargo run -p xtask -- update-ratios` happily
overwrote the committed 44-entry pin with a 2-entry file (`wrote 2 entries
...`), and `git diff` showed 42 programs' entries deleted. Restored both
the truncation and the guard; a rerun writes all 44 entries again with no
diff against the committed file.

## Threat mitigations

| Threat | State |
|---|---|
| T-02-SC, the `toml` package | Mitigated. `RESEARCH.md`'s audit approves it; resolved version `1.1.5+spec-1.1.0` committed in `Cargo.lock` and recorded above. Used only as a secondary validity check in `crates/xtask`, never as the shared formatter (see Deviations, item 6). |
| T-02-41, the two failure messages paired to the wrong direction | Mitigated. Both words are `pub(crate)` constants with a one-sentence reason each; breakage 1 above swapped them for real and both directional tests failed. |
| T-02-42, a pinned ratio that cannot move | Mitigated. Object recovery stays an equality in `differential.rs`, never pinned here; this file's own ratio has 24 distinct values across 44 programs. |
| T-02-43, a stale or missing key passing silently | Mitigated and re-proven after a real gap: breakage 3 above found the first covering test could pass even with the check removed, so it was rewritten to exercise the real `gate_failures` function, and both directions were re-broken and re-caught. |
| T-02-44, `update-ratios` overwriting a good pin with a short file | Mitigated and demonstrated: breakage 2 above removed the guard and it silently wrote a 2-entry file over the 44-entry pin. Restored guard confirmed to refuse the same short walk. |
| T-02-45, the writer's arithmetic disagreeing with the gate | Mitigated. Both take their counts from `differential::program_counts` alone (Deviations, item 3), and both the `MOVED UP` message and the writer format one entry through the single `format_entry` function (Deviations, item 6); `the_writers_per_entry_block_is_byte_for_byte_format_entrys_output` and `lowering_a_pinned_recovered_count_fails_with_moved_up_and_the_exact_paste_block` both assert this directly. |

## The gate

Run on the committed tree at `a7d8b2f`, clean working tree confirmed
before and after (`git status --short` empty).

| Command | Result |
|---|---|
| `cargo fmt --all --check` | 0, no output |
| `cargo clippy --all-targets -- -D warnings` | 0, no warnings, across all four crates including `xtask` |
| `cargo test -p deform6 --test ratios` | 0, 15 tests pass |
| `cargo run -p xtask -- update-ratios && git diff --exit-code -- tests/ratios.toml` | 0, "wrote 44 entries", no diff |
| `cargo test --workspace` | 0, 289 tests pass (197 lib, 1 corpus_sweep, 7 differential, 15 ratios, 9 refusal, 27 support_selftest, 2 type_descriptors, 9 cli, 22 xtask); 252 before this plan |
| `sh scripts/prove-lint-wall.sh` | 0, "The wall stops every bad shape, and the tree it leaves behind is clean." |
| `sh scripts/prove-region-wall.sh` | 0, "The type refuses every shape, and the tree it leaves behind is clean." |
| `grep -c 'REGRESSION' crates/deform6/tests/ratios.rs` | 7 |
| `grep -c 'MOVED UP' crates/deform6/tests/ratios.rs` | 5 |

## Known Stubs

None.

## Deferred Issues

None. `crates/xtask`'s `update-ratios` is fully implemented; no follow-up
plan is needed for this file's own scope.

## Threat Flags

None beyond the six already in this plan's own threat model, all
mitigated above. No new network endpoint, auth path, or file access path
beyond the vendored corpus and the two files (`tests/ratios.toml`,
`Cargo.lock`) every other plan in this phase already touches.

## Commits

| Commit | Subject |
|---|---|
| `4fb0006` | Add the xtask crate to the workspace |
| `4988348` | Pin the procedure recovery of every corpus program |
| `a7d8b2f` | Add the update-ratios command that rewrites the pinned file |

`git rev-list --count 73e6cc2..HEAD` is 3, one commit per task, each
carrying its own tests.

## Self-Check: PASSED

`tests/ratios.toml` exists, 44 entries. `crates/deform6/tests/ratios.rs`
exists. `crates/xtask/Cargo.toml` and `crates/xtask/src/main.rs` exist.
All three commit hashes (`4fb0006`, `4988348`, `a7d8b2f`) resolve in this
branch's history. The gate table above was run on the committed tree, with
a clean working tree confirmed by `git status --short` before this file
was written.
