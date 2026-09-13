---
phase: 05-hostility
plan: 06
subsystem: fuzzing
tags: [rust, mutation-testing, deterministic, dos, performance]

requires:
  - phase: 05-hostility
    provides: "Mode::{Strict, Salvage} threaded through deform6::inspect and deform6::write::project (plan 05-01)"
  - phase: 05-hostility
    provides: "crates/deform6/tests/corpus_sweep.rs's directory walk and sorted executables() shape, reused rather than reimplemented"
provides:
  - "crates/deform6/tests/fuzz_smoke.rs: a deterministic, dependency-free, seeded mutation sweep over the vendored corpus, run inside cargo test --workspace on stable, with no nightly toolchain and no cargo-fuzz dependency"
  - "A fix for a real O(n^2) denial-of-service defect the sweep found in write::project: PathIssuer (crate::report) and SafeNameIssuer (crate::write::model) now use a HashSet plus a per-key next-suffix cache instead of a linearly-scanned Vec"
affects: [05-07, 05-08]

actuals:
  tokens: 6005
  tasks: 2
  commits: 4
  plan_head_before: 9189ee4

tech-stack:
  added: []
  patterns:
    - "A fixed-width wrapping LCG (Knuth's MMIX constants) written directly in the test file, with no external RNG crate, so the smoke test adds zero dependency and its failures are reproducible by rerunning the same command."
    - "Print-before-panic logging: the sweep builds and prints its failure message (file, iteration, seed, byte changes) before each iteration runs, because a panic ends the process and nothing after the panic point ever executes."
    - "A per-key next-suffix cache alongside a HashSet existence check, so a collision-heavy issuer resumes its suffix search from where it left off for that exact key instead of restarting from the first candidate every time."

key-files:
  created:
    - crates/deform6/tests/fuzz_smoke.rs
  modified:
    - crates/deform6/src/report.rs
    - crates/deform6/src/write/model.rs

key-decisions:
  - "SMOKE_ITERATIONS is 440 (ten mutations of each of the 44 vendored corpus executables), distributing the budget evenly across files rather than spending it all on one, matching the rule 05-05's own walk uses."
  - "SMOKE_SEED is a literal, arbitrary u64 constant (0x5EED_BEEF_C0FF_EE01). The generator is a Knuth MMIX linear congruential generator using only wrapping_mul/wrapping_add, written as named wrapping calls rather than plain operators, because this workspace's dev and test profiles still check for overflow at runtime even though the file's own allow header turns off the corresponding clippy lint at compile time."
  - "The mutation sweep found a genuine algorithmic-complexity defect during task 2's own measurement pass: a specific three-byte mutation of corpus/vb6-code/Curves-effect/Curves.exe made write::project take over two minutes to finish, because PathIssuer and SafeNameIssuer both scanned a growing Vec<String> with .contains() for every issued path or name, and both restarted their collision-suffix search from the first candidate on every call. This is a real hostile-input denial-of-service bug (T-5-26's own threat category), not an artifact of the test, so it was fixed rather than dodged: both issuers now hold a HashSet<String> for O(1) average membership tests and a HashMap<String, u32> that remembers, per colliding key, the next suffix to try. The fix reduced the same mutated input's write time from over two minutes to under 150 milliseconds and let SMOKE_ITERATIONS stay at its originally planned value of 440 instead of being lowered to avoid the slow case."
  - "The doc comment ordering concern already on both issuers (a process-dependent hash iteration order could make the written report non-deterministic) does not apply to this fix: issue() never iterates the HashSet or HashMap, it only tests membership and inserts, so the suffix a collision receives still depends solely on the order a caller calls issue(), never on either collection's own internal layout. Both doc comments were rewritten to state this precisely rather than repeat the original, slightly imprecise 'never a hash keyed map' blanket claim."
  - "Task 1 could not include the corpus walk, the Mode/OpcodeTable/inspect/write imports, or the sweep's own MUTATIONS_PER_INPUT constant, even though the plan's task 1 action text describes SMOKE_ITERATIONS's doc comment referencing measured costs: including them before task 2 exists would have left them unused and failed cargo clippy -D warnings (unused imports, dead code) as its own gate check. Task 1 committed only the generator, the mutation function, the two named constants, and four unit tests (including a compile-time sanity check on SMOKE_ITERATIONS, with a scoped #[allow(clippy::assertions_on_constants)]); task 2 added the corpus walk and the sweep itself."

requirements: [SAF-01]
requirements-completed: []

coverage:
  - id: D1
    description: "A deterministic, seeded, dependency-free mutation sweep over the vendored corpus runs inside cargo test --workspace on stable, with no nightly toolchain, exercising both Mode::Strict and Mode::Salvage plus the writer on the salvage result, asserting nothing about the result variant and only that it ran the claimed iteration count"
    requirement: "SAF-01"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/fuzz_smoke.rs#mutated_corpus_bytes_never_panic_the_reader_in_either_mode"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/fuzz_smoke.rs#tests::two_generators_from_the_same_seed_give_the_same_first_ten_values"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/fuzz_smoke.rs#tests::a_mutation_keeps_the_length_and_changes_exactly_the_recorded_offsets"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/fuzz_smoke.rs#tests::the_original_slice_is_unchanged_after_a_mutation"
        status: pass
    human_judgment: false
  - id: D2
    description: "PathIssuer and SafeNameIssuer no longer scan a growing Vec for every issued path or name, and no longer restart their collision-suffix search from the first candidate on every call, closing the O(n^2) denial-of-service defect the sweep found"
    verification:
      - kind: unit
        ref: "crates/deform6/src/report.rs#tests::a_thousand_items_with_the_same_base_path_each_get_a_distinct_path"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/report.rs#tests::two_issuers_fed_the_same_calls_in_the_same_order_give_the_same_paths"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/write/model.rs#tests::a_thousand_controls_with_the_same_raw_name_each_get_a_distinct_name"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-09-13
status: complete
---

# Phase 5 Plan 6: Deterministic mutation smoke test (and the DoS bug it found) Summary

**A seeded, dependency-free mutation sweep runs inside `cargo test --workspace` on stable and found (and its fix now proves closed) a real O(n^2) denial-of-service bug in the report writer's path/name collision handling.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 2 completed
- **Files modified:** 3 (1 created, 2 modified)
- **Commits:** 4

## Accomplishments

- `crates/deform6/tests/fuzz_smoke.rs` adds a fixed-width wrapping linear congruential generator, a single-byte mutation rule, and a sweep that mutates every one of the 44 vendored corpus executables, ten times each (440 total), running each mutated input through `Mode::Strict`, `Mode::Salvage`, and `write::project` on the salvage result.
- The sweep is fully reproducible: the same `SMOKE_SEED` literal gives the same mutations, in the same order, on any machine, every time. No system clock, process ID, address, or OS entropy source is used anywhere in the generator.
- Running the sweep for the first time (its own "prove the fixture works" measurement step) surfaced a genuine defect: one specific 3-byte mutation of `corpus/vb6-code/Curves-effect/Curves.exe` made `write::project` take over two minutes. Root-caused via `sample` (macOS's built-in sampling profiler) to `PathIssuer::issue` and `SafeNameIssuer::issue`, both of which scanned a growing `Vec<String>` linearly for every issued path or name and restarted their collision-suffix search from the first candidate on every call, an `O(n^2)` blowup under a mutation-inflated run of colliding paths/names.
- Fixed both issuers to use a `HashSet<String>` (O(1) average membership) plus a `HashMap<String, u32>` remembering the next suffix to try per colliding key. The same mutated input now writes in well under 150ms. This is a genuine hardening fix in production code, not test-only work, closing a real hostile-input DoS vector.
- The fix let `SMOKE_ITERATIONS` stay at the originally intended value of 440 (rather than being reduced to dodge the slow case), and the sweep's own measured cost (~0.65s for `cargo test -p deform6 --test fuzz_smoke`, ~0.6s added to `cargo test --workspace`) stays under one percent of this workspace's ~90 second three-command gate.

## Task Commits

Each task was committed atomically. Two additional commits landed between task 1 and task 2 to fix the DoS defect the sweep's own measurement pass found:

1. **Task 1: A deterministic generator and a mutation rule** - `d0b695c` (test)
2. **[Deviation, Rule 1] Fix PathIssuer's O(n^2) collision handling** - `5ec64bc` (fix)
3. **[Deviation, Rule 1] Fix SafeNameIssuer's O(n^2) collision handling** - `03db005` (fix)
4. **Task 2: The sweep itself, over both modes** - `42132d5` (test)

**Plan metadata commit:** pending (this SUMMARY, STATE.md, ROADMAP.md, REQUIREMENTS.md)

## Files Created/Modified

- `crates/deform6/tests/fuzz_smoke.rs` - the deterministic mutation sweep (created)
- `crates/deform6/src/report.rs` - `PathIssuer` now uses `HashSet` + per-path next-suffix cache (modified)
- `crates/deform6/src/write/model.rs` - `SafeNameIssuer` now uses `HashSet` + per-name next-suffix cache (modified)

## Decisions Made

See `key-decisions` in the frontmatter above for the full rationale on: the 440/seed choice, the wrapping-arithmetic requirement, the discovered defect and its fix, the doc-comment correction about hash-collection determinism, and why task 1 could not include the corpus walk or sweep body.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug, security] `PathIssuer::issue` (crates/deform6/src/report.rs) had O(n^2) worst-case cost under attacker-controlled path collisions**

- **Found during:** Task 2's own required measurement step ("measure the real cost... time `cargo test -p deform6 --test fuzz_smoke`")
- **Issue:** `issued: Vec<String>` was scanned with `.contains()` on every call, and the collision-suffix search restarted from `suffix = 2` on every call regardless of how many prior collisions the same base path had already resolved. A specific 3-byte mutation of `corpus/vb6-code/Curves-effect/Curves.exe` drove a large number of items to the same base path, making `write::project` take over two minutes (confirmed with `sample`, macOS's built-in sampling profiler, which showed nearly the entire wall-clock time inside `PathIssuer::issue`'s `HashSet`/`Vec` `contains` calls before the fix).
- **Fix:** `issued` is now a `HashSet<String>` for O(1) average membership tests. A new `next_suffix: HashMap<String, u32>` remembers, per base path, the next suffix to try, so the n-th collision on the same path no longer replays every earlier suffix. The doc comment's prior claim ("never a hash keyed map... an iteration order that varies per process would make the written report vary run to run") was corrected: `issue()` never iterates either collection, so this determinism concern does not apply to a membership-test-only usage; the suffix still depends solely on caller call order.
- **Files modified:** `crates/deform6/src/report.rs`
- **Verification:** Added `a_thousand_items_with_the_same_base_path_each_get_a_distinct_path` (correctness under heavy collision) and `two_issuers_fed_the_same_calls_in_the_same_order_give_the_same_paths` (determinism). Re-ran the exact reproducer (the specific 3-byte mutation of Curves.exe) before and after: 130.4s before the `HashSet` swap alone, 4.04s after the `HashSet` swap alone, 140.9ms after adding the `next_suffix` cache.
- **Committed in:** `5ec64bc`

**2. [Rule 1 - Bug, security] `SafeNameIssuer::issue` (crates/deform6/src/write/model.rs) had the identical defect**

- **Found during:** Same investigation as above; the same class of bug existed in the sibling name issuer.
- **Issue:** Same shape: `issued: Vec<String>`, linear `.contains()`/`.iter().any()`, and a suffix search that restarted from `suffix = 1` on every call.
- **Fix:** Same shape: `HashSet<String>` plus a `next_suffix: HashMap<String, u32>` per original raw name.
- **Files modified:** `crates/deform6/src/write/model.rs`
- **Verification:** Added `a_thousand_controls_with_the_same_raw_name_each_get_a_distinct_name`. Full workspace suite re-run green after both fixes (928 passed, 0 failed).
- **Committed in:** `03db005`

---

**Total deviations:** 2 auto-fixed (both Rule 1, both security/correctness bugs in production code, both discovered by the smoke test this plan built).
**Impact on plan:** Both fixes were necessary for the smoke test itself to stay inside its own stated performance budget (under 1% of the gate), and both close a genuine hostile-input denial-of-service vector that Phase 5 exists to close. No scope creep beyond that: no other production files were touched, and SAF-01 (declared by this plan and by 05-03, 05-05, 05-08) is left pending exactly as instructed, since 05-05 and 05-08 have not yet run.

## Issues Encountered

`lldb -p <pid>` could not attach to the running test process in this sandboxed environment (`attach failed: attach failed (attached to process, but could not pause execution; attach failed)`, and `sudo` required a password that was not available). Used macOS's built-in `sample <pid> <seconds>` tool instead, which does not require the same attach permission and produced a full call-graph sample pointing directly at `PathIssuer::issue`'s hash/collection `.contains()` calls.

The plan's task 1 read_first/action text implies `SMOKE_ITERATIONS`'s doc comment (with its measured-cost numbers) is written entirely in task 1, but the corpus walk and the sweep itself do not exist until task 2, so no real measurement was possible until task 2. Task 1 wrote a first-pass estimate from a narrower manual timing experiment (repeatedly mutating only the largest single file); task 2 then re-measured with the real sweep in place, as the plan's own task 2 action text separately requires ("Measure the real cost... If the sweep costs more than the share task 1's constant claimed, lower the iteration count and say so"), and rewrote the doc comment with the final, honest numbers plus the defect-and-fix story. This is not a deviation from the plan's substance, just a sequencing note: the plan's own two action blocks (task 1 estimate, task 2 re-measurement) already anticipated the estimate could be revised.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`crates/deform6/tests/fuzz_smoke.rs` runs in the normal `cargo test --workspace` gate today, on stable, with no nightly toolchain and no new dependency, and its `SMOKE_SEED`/`SMOKE_ITERATIONS` constants are available for any later plan in this phase that wants to reference them. The write-path DoS fix in `PathIssuer`/`SafeNameIssuer` is unrelated to any specific later plan but keeps the whole corpus-writing gate (including 05-05's and 05-08's future work) from randomly regressing to multi-minute runs on a future corpus addition or fuzzer-found input.

SAF-01 remains `- [ ]` / `Pending` in `.planning/REQUIREMENTS.md`: it is also declared by 05-03 (already summarized), 05-05, and 05-08 (neither yet run). `gsd-tools query requirements.ready-ids` confirms SAF-01 is currently `blocked`, not `ready`.

## Self-Check: PASSED

- FOUND: crates/deform6/tests/fuzz_smoke.rs
- FOUND: crates/deform6/src/report.rs (modified, diff present)
- FOUND: crates/deform6/src/write/model.rs (modified, diff present)
- FOUND commit d0b695c
- FOUND commit 5ec64bc
- FOUND commit 03db005
- FOUND commit 42132d5
- `cargo test --workspace`: 928 passed, 0 failed (up from a 920-passed floor: +4 fuzz_smoke task-1 tests, +1 sweep test, +3 issuer regression tests)
- `cargo fmt --all --check`: exit 0
- `cargo clippy --all-targets -- -D warnings`: exit 0
- `.planning/REQUIREMENTS.md` SAF-01: confirmed still `- [ ]` / `Pending`
