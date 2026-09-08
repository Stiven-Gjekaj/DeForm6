---
phase: 02-the-object-graph
plan: 08
subsystem: test-harness
tags: [differential, both-directions, object-graph, procedure-comparison, ver-01, ver-02, ver-03, ver-04, d-02, d-06]
status: complete

requires:
  - Object, ObjectTable::walk, bounded by wTotalObjects (plan 02-01)
  - classify, ObjectKind (plan 02-02)
  - ProcNames, Procedure, ProcedureList::read, ProcedureCounts::of (plan 02-03)
  - ProjectInfo, ObjectTableHead (plan 02-06)
  - support::vbp (the second, independent .vbp reader) and support::rules
    (the five written exclusion rules), from plan 02-07
provides:
  - crates/deform6/tests/differential.rs, comparing the library's recovered
    object graph and public procedure names against support::vbp and
    support::source, both directions, over all 44 corpus programs
  - crates/deform6/tests/support/source.rs, a second, independent reader
    that gives the ordered list of procedures a source file declares public
  - the answer to the open question CONTEXT.md left this plan: all 428
    corpus-wide unresolvable lpProcNamesArray entries fail at address
    resolution, none at NUL-termination, none at the identifier test
affects:
  - a future plan pinning the recovery ratio (ratios.toml) and printing it,
    which can reuse program_counts (documented inclusion mechanism, see
    "Defects found in the plan" below) rather than recomputing the four
    numbers a third time

tech_stack:
  added: []
  patterns:
    - "both set differences computed together in one function
      (two_directional_diff), so a caller can never assert only one
      direction by accident, and a single deliberate breakage of that one
      function cripples every caller identically (proved directly: the
      breakage exercise below)"
    - "a doctored-input test starts from real, already-agreeing corpus data
      and doctors one side of it, rather than building a synthetic literal
      from scratch, so the proof is that a subset check fails on real data
      the two-directional check catches, not only on data built to order"
    - "the standard-module and absent-source rules are applied per object,
      inline in the same loop that does the name-set comparison, never as a
      separate filter pass that could drift from the comparison itself"

key_files:
  created:
    - crates/deform6/tests/differential.rs
    - crates/deform6/tests/support/source.rs
  modified:
    - crates/deform6/tests/support/mod.rs
    - crates/deform6/tests/support_selftest.rs

decisions:
  - "Task 1's first deliberate breakage ('loop the library's object walk on
    the array capacity instead of the count') as literally written requires
    editing crates/deform6/src/vb/object.rs. The user's prompt for this run
    is explicit and repeated: this plan owns only differential.rs and
    support/source.rs, must not modify anything under src/, and must report
    a library defect rather than patch it. Rule 4 (architectural conflict),
    resolved per the same precedent plan 02-01 set for its own
    files_modified/task-text disagreement: follow the explicit instruction
    rather than the plan's literal action text. The substance of the
    breakage was still proved, without touching src/: a temporary,
    uncommitted test built a capacity-bounded object walk using only public
    Region/PeImage/Va primitives (never object.rs's own code) and ran it
    against the real corpus. It found 4 programs where the capacity's extra
    slots resolve to real, non-empty garbage strings that the two-directional
    check catches (not 15, the plan's predicted count of programs where
    capacity exceeds count -- most of those extra slots are null and
    contribute no name at all). This is recorded here rather than left as a
    silent substitution, and the temporary test was removed before any
    commit."
  - "Task 3's first deliberate breakage ('drop the standard-module rule,
    confirm the comparison fails with 23 declared-not-recovered slots') was
    run for real, in the working tree, and produced a genuine failure -- but
    17 declared-not-recovered names across 6 of the 8 modules, not 23. The
    plan's 23 is the total procedure *slots* the standard-module rule caps
    across every visibility (Sub, Function, Property, Declare, public and
    private alike; support_selftest.rs's own
    the_standard_module_rule_excludes_exactly_eight_objects_and_twenty_three_procedure_slots
    test already asserts this number, from plan 02-07). This plan's
    procedure comparison only ever looks at *public* names, so removing the
    rule surfaces only the modules that happen to declare at least one
    public procedure (6 of 8; the other 2 modules declare none, so removing
    the rule produces no visible mismatch for them) and only the public
    subset of their slots (17 of 23). Recorded as a measurement correction,
    not silently matched to the plan's predicted number."
  - "The plan's task 2 prediction ('confirm the sixth behaviour fails with
    an empty list for the class whose four procedures carry the public
    keyword explicitly') does not hold: FastDrawing.cls's four public
    members (GetImageWidth, GetImageHeight, GetImageData2D,
    SetImageData2D) all carry the Public keyword explicitly in the real
    corpus file, so treating a *missing* scope keyword as private cannot
    touch them, and fast_drawing_gives_exactly_its_four_public_names_in_file_order
    passed unmodified under this breakage. The breakage was still caught,
    for real: the_corpus_declares_one_hundred_and_eighty_five_public_procedures_over_the_objects_the_rules_keep
    dropped to 179 (six procedures corpus-wide rely on the implicit-public
    default), and a_continued_declaration_is_counted_once_from_its_first_line
    failed as a side effect, because its own fixture (VBGetOpenFileName)
    happens to have no explicit scope keyword. Recorded rather than
    silently substituted."
  - "Task 2's second deliberate breakage ('allow a continuation line to
    match, confirm the total rises above 185') produced zero failures on
    the real corpus when first tried: a corpus-wide Python check (before
    writing any Rust) found no continuation line in any of the 44 programs'
    source files whose first token happens to equal a declaration keyword,
    so removing the continuation skip has no observable effect on real
    data. Per AGENTS.md ('a deliberate breakage that produces no failure
    means the covering test does not exist'), a new permanent test was
    added -- a_continuation_line_that_looks_like_a_declaration_opener_is_still_skipped
    -- built entirely from a literal string constructed inside the test
    (AGENTS.md's 'build the state a test needs inside the test'), because no
    real corpus file can exercise this path. Re-running the breakage against
    this new test failed exactly as expected (['Example', 'Bogus'] instead
    of ['Example']); restored and re-verified."
  - "program_counts (the four numbers a future ratio-pinning plan needs)
    lives in differential.rs, not in support/, because it must call
    deform6 for the recovered side, which support::vbp and support::source
    are forbidden from doing (D-06, grep-proven at zero for both). A future
    plan reaching for it from a second test binary can use
    #[path = \"../differential.rs\"] mod differential;, the same mechanism
    support/mod.rs already uses to share a file across binaries, pointed at
    a file not literally named mod.rs. Documented in a comment on the type,
    per the plan's own instruction to choose one location and say which."
  - "Object recovery stays an equality (105 of 105, both directions), never
    a pinned ratio, per D-11: the corpus has 44 of 44 with no variance, and
    AGENTS.md says a test that cannot fail is worse than none. The pinned
    ratio a future plan owns is over procedures, where real variance exists
    (0 of 1 through 16 of 34 across programs, per the per-program table
    below)."

metrics:
  duration: 1 session
  completed: 2026-09-08

actuals:
  tokens: 11860
  tasks: 3
  commits: 3
plan_head_before: 16d354c1244d820cbbf3673346a1fde268e3fe4e
---

# Phase 02 Plan 08: The differential gate Summary

`crates/deform6/tests/differential.rs` compares what `deform6` recovers
against the source the executable was built from, in both directions, over
all 44 corpus programs: 105 of 105 objects match by name and by kind, and
185 of 185 public procedure names match, with the standard-module and
absent-source rules applied to both sides. `crates/deform6/tests/support/source.rs`
is the second, independent source reader that gives the declared side of
the procedure comparison, sharing nothing with `deform6` (grep-proven at
zero).

## What this plan built

| Item | What it gives |
|---|---|
| `differential.rs` task 1 | The object comparison: both set differences computed and asserted for every one of the 44 programs, 105 objects total, kind totals 53/8/44, plus a doctored-input test proving a subset check passes where the two-directional check fails |
| `support/source.rs` | `declared_public_procedures(path) -> Vec<String>`, matching a `Sub`/`Function`/`Property Get`/`Let`/`Set` with no scope keyword or `Public`, skipping continuation lines by construction |
| `differential.rs` task 3 | The procedure comparison: both set differences over the objects the standard-module and absent-source rules keep, 185 of 185 matching, the four numbers (`ProgramCounts`) plan a future ratio-pinning plan needs, and the 13 zero-recovery programs named in the test output |

7 tests in `differential.rs`, 27 in `support_selftest.rs` (6 new). The
workspace test count rose from 239 to 252.

## The comparison is against the original source, never a round trip

Every declared-side value in this file comes from `support::vbp` (the
`.vbp` project file) or `support::source` (the source text the executable
was built from), never from a second call into `deform6`. Neither harness
reader names `deform6`; both are grep-proven at zero non-comment
occurrences of the literal `deform6`.

## The per-program table, measured (for a future ratio-pinning plan)

`recovered` and `declared_by_binary` are counted over the objects the
standard-module and absent-source rules keep; `capped` is the `proc_count`
sum of this program's standard-module objects (zero if it has none);
`excluded` is the object count the two rules drop for this program.

| Program | recovered | declared_by_binary | capped | excluded |
|---|---|---|---|---|
| public-domain/HexScroll/Hex Scroll.exe | 0 | 23 | 0 | 0 |
| public-domain/LockWorkStation/LockWorkStation.exe | 0 | 1 | 0 | 0 |
| public-domain/PassGen/PassGen.exe | 5 | 83 | 0 | 0 |
| public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe | 0 | 1 | 2 | 1 |
| public-domain/SK-MCI-Sample__VB6/demo/Project1.exe | 0 | 33 | 0 | 0 |
| public-domain/SK-TFTP-Sample__VB6/Client/demo/TFTPClient.exe | 0 | 15 | 0 | 0 |
| public-domain/SK-TFTP-Sample__VB6/Server/demo/Server.exe | 0 | 6 | 0 | 0 |
| public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe | 0 | 5 | 0 | 0 |
| public-domain/UUID2/VB6/UUID2.exe | 4 | 25 | 0 | 0 |
| public-domain/dump/VB6/rot47/Rot47.exe | 1 | 10 | 0 | 0 |
| public-domain/machineLanguageConversion/ASCII/Ascii.exe | 1 | 6 | 0 | 0 |
| public-domain/machineLanguageConversion/Hex/Hex.exe | 1 | 7 | 0 | 0 |
| public-domain/machineLanguageConversion/Octal/Octal.exe | 1 | 6 | 0 | 0 |
| vb6-code/Artificial-life/Artificial Life.exe | 13 | 37 | 3 | 1 |
| vb6-code/Blacklight-effect/Blacklight.exe | 5 | 22 | 0 | 0 |
| vb6-code/Brightness-effect/Part 1 - Pure VB6/vbBrightness.exe | 5 | 7 | 0 | 0 |
| vb6-code/Brightness-effect/Part 2 - API - GetPixel and SetPixel/apiBrightness.exe | 5 | 9 | 0 | 0 |
| vb6-code/Brightness-effect/Part 4 - Even faster DIBs/Realtime_Brightness.exe | 12 | 19 | 0 | 0 |
| vb6-code/Color-shift-effect/ColorShift.exe | 5 | 20 | 0 | 0 |
| vb6-code/Colorize-effect/Colorize.exe | 7 | 30 | 0 | 0 |
| vb6-code/Contrast-effect/Contrast.exe | 5 | 21 | 0 | 0 |
| vb6-code/Curves-effect/Curves.exe | 5 | 27 | 0 | 0 |
| vb6-code/Custom-image-filters/Custom_Filters.exe | 6 | 28 | 0 | 0 |
| vb6-code/Diffuse-effect/Diffuse.exe | 5 | 26 | 0 | 0 |
| vb6-code/Edge-detection/Edge_Detection.exe | 6 | 21 | 0 | 1 |
| vb6-code/Emboss-engrave-effect/Emboss_Engrave.exe | 12 | 34 | 0 | 0 |
| vb6-code/Fill-image-region/Filling.exe | 0 | 4 | 0 | 0 |
| vb6-code/Fire-effect/Fast_Flames.exe | 4 | 18 | 0 | 0 |
| vb6-code/Game-physics-basic/Physics_Demo.exe | 1 | 7 | 7 | 1 |
| vb6-code/Gradient-2D/Gradient.exe | 2 | 11 | 0 | 0 |
| vb6-code/Grayscale-effect/Grayscale.exe | 12 | 34 | 0 | 0 |
| vb6-code/Hidden-Markov-model/HMM.exe | 0 | 12 | 0 | 0 |
| vb6-code/Histograms-advanced/Advanced Histogram Viewer.exe | 9 | 36 | 1 | 1 |
| vb6-code/Histograms-basic/Basic Histogram Viewer.exe | 5 | 25 | 1 | 1 |
| vb6-code/Levels-effect/Image Levels.exe | 5 | 41 | 1 | 1 |
| vb6-code/Mandelbrot/Mandelbrot.exe | 0 | 9 | 0 | 0 |
| vb6-code/Map-editor-2D/Map Editor.exe | 0 | 28 | 8 | 2 |
| vb6-code/Nature-effects/Nature_Filters.exe | 16 | 34 | 0 | 0 |
| vb6-code/Randomize-effects/RandomizationFX.exe | 12 | 29 | 0 | 0 |
| vb6-code/Scanner-TWAIN/VB_Scanner_Support.exe | 0 | 11 | 0 | 0 |
| vb6-code/Screen-capture/ScreenCapture.exe | 0 | 5 | 0 | 0 |
| vb6-code/Sepia-effect/Sepia.exe | 5 | 20 | 0 | 0 |
| vb6-code/Threshold-effect/Threshold.exe | 5 | 23 | 0 | 0 |
| vb6-code/Transparency-2D/Transparency.exe | 5 | 12 | 0 | 0 |

**Totals:** recovered 185, declared_by_binary 881, capped 23, excluded 9
(8 standard-module objects + 1 absent-source object). `881 + 23 = 904`,
matching the prompt's independently-measured "185 of 904 procedure slots
recovered overall" exactly: `904` is the binary-declared slot total across
every non-absent-source object regardless of visibility (881 kept-object
slots plus the 23 standard-module slots the rule caps), and this plan's own
measurement reproduces it precisely as a cross-check.

## The 13 programs that recover no public procedure at all

`Hex Scroll.exe`, `LockWorkStation.exe`, `SK-Gradient-Sample__VB6`'s and
`SK-MCI-Sample__VB6`'s `Project1.exe`, `TFTPClient.exe`, `Server.exe`,
`SubReality_WinsockSample.exe`, `Filling.exe`, `HMM.exe`, `Mandelbrot.exe`,
`Map Editor.exe`, `VB_Scanner_Support.exe`, `ScreenCapture.exe`. Every
object in each of these programs is a form or a class whose only
procedures are private event handlers (or, for `Map Editor.exe`, entirely
capped by the standard-module rule) -- correct behaviour, not a shortfall.
`cargo test -p deform6 --test differential -- --nocapture` prints this list
from a real run, not a hand-typed copy.

## The 428 unresolvable entries: what the corpus-wide sweep found

CONTEXT.md left this open: across all 105 objects, 296 `lpProcNamesArray`
entries are null, 193 resolve to a plausible name, and 428 are neither.
Plan 02-03 answered this for the 3 vendored programs it read (all 9 of
Mandelbrot's failing entries fail at address resolution) and explicitly
left the corpus-wide question to this plan.

**Measured over all 44 programs, all 105 objects, all 917 total entries** (a
temporary diagnostic built for this investigation and removed before any
commit, using only public `Region`/`PeImage` primitives, never editing
`src/`):

- 296 null (private, as documented).
- 193 resolve to a mapped section, are NUL-terminated within 64 bytes, and
  pass the identifier test: recovered as `Public`. Matches the 193
  `FuncTypDesc` records plan 02-04 counted by an independent route.
- **428 are non-null and resolve to no mapped section at all.**
- **Zero** entries resolve to a mapped section and then fail either later
  check (no NUL within 64 bytes, or fails the identifier test).

**The answer: every one of the 428 unresolvable entries fails at the same
single step -- address resolution -- and none of them exercise the other
two validation branches on real data.** There is no evidence anywhere in
this corpus of a second, unrecognised encoding: the entire population is
uninitialised compiler-buffer content (the same shape as `Mandelbrot`'s
UTF-16 build-machine path fragment) that happens not to point into any
mapped section, not a structure this project has failed to decode. This
extends plan 02-03's 3-program finding to the full corpus with zero
exceptions.

## Defects found in the plan

Recorded in full under `decisions` in the frontmatter above (five entries):
the task 1 breakage's file-boundary conflict and its substitute proof; the
task 3 breakage's corrected number (17, not 23); the task 2 fixture
prediction that did not hold for FastDrawing.cls; the task 2 continuation
breakage that initially produced no failure and the new test that was
added to give it one; and the `program_counts` placement decision.

No defect was found in library code (`src/`). The full corpus sweep this
plan is built to run found no library bug: 105 of 105 objects match by
name and kind, 185 of 185 procedures match by name, in both directions,
across all 44 programs, with zero exceptions.

## The deliberate breakages (six required, six run)

All run in the working tree, observed, then restored; `cargo fmt --all
--check` confirmed a clean tree in every case where an equivalent
byte-identical restoration mattered.

### Task 1, breakage 1: the object walk bounded by capacity, not count

Could not be applied to `src/vb/object.rs` (out of this plan's file
boundary, per explicit instruction). Proved instead with a temporary,
uncommitted test-local re-implementation of the array walk, bounded by
`w_compiled_objects`, using only public `Region`/`PeImage`/`Va`. **4 of the
44 programs disagreed** (`vbBrightness.exe`, `apiBrightness.exe`,
`Realtime_Brightness.exe`, `Contrast.exe`), each with one extra
recovered-not-declared garbage name from the array's rounded-up capacity
slots. Removed before any commit.

### Task 1, breakage 2: the recovered-minus-declared direction deleted

Deleted from `two_directional_diff`, the one function every comparison in
the file calls. `fabricated_object_passes_a_subset_check_and_fails_the_two_directional_check`
failed exactly as predicted (the other two object-layer tests still
passed, because the real corpus has no actual over-count for the
whole-corpus test to catch). Restored; all 3 object tests pass again.

### Task 2, breakage 1: a missing scope keyword read as private

`fast_drawing_gives_exactly_its_four_public_names_in_file_order` did
**not** fail (its four procedures carry `Public` explicitly). Two other
tests did:
`the_corpus_declares_one_hundred_and_eighty_five_public_procedures_over_the_objects_the_rules_keep`
dropped from 185 to 179, and
`a_continued_declaration_is_counted_once_from_its_first_line` failed as a
side effect (its own real-corpus fixture, `VBGetOpenFileName`, has no
explicit scope keyword). Restored; both pass again, corpus total back to
185.

### Task 2, breakage 2: a continuation line allowed to match

Produced **zero failures** on the first run (no real corpus continuation
line happens to start with a declaration keyword). A new test,
`a_continuation_line_that_looks_like_a_declaration_opener_is_still_skipped`,
was added with a literal fixture built inside the test. Re-run against the
breakage: failed with `["Example", "Bogus"]` instead of `["Example"]`,
exactly as the fixture was built to prove. Restored; the new test (and all
26 others) pass.

### Task 3, breakage 1: the standard-module rule dropped

**17 declared-not-recovered names across 6 of the 8 modules** (not 23 --
see "Defects found in the plan"). Restored; the corpus procedure test
passes again at 185 of 185.

### Task 3, breakage 2: recovered-minus-declared deleted again, at the procedure layer

Same edit as task 1's breakage 2, re-run against the new procedure-layer
doctored-input test
(`a_fabricated_procedure_name_passes_a_subset_check_and_fails_the_two_directional_check`).
Failed exactly as predicted. Restored; all 7 tests in `differential.rs`
pass again.

## The gate

Run on the committed tree at `6d4ec4c`, clean working tree confirmed
before and after (`git status --short` empty).

| Command | Result |
|---|---|
| `cargo fmt --all --check` | 0, no output |
| `cargo clippy --all-targets -- -D warnings` | 0 (confirmed after a full `cargo clean`, not only an incremental run) |
| `cargo test -p deform6 --test differential` | 0, 7 tests pass |
| `cargo test --workspace` | 0, 252 tests pass (197 lib, 1 corpus_sweep, 7 differential, 9 refusal, 27 support_selftest, 2 type_descriptors, 9 cli, 0 doc-tests); 239 before this plan |
| `sh scripts/prove-lint-wall.sh` | 0, "The wall stops every bad shape, and the tree it leaves behind is clean." |
| `sh scripts/prove-region-wall.sh` | 0, "The type refuses every shape, and the tree it leaves behind is clean." |

```
$ grep -vE '^\s*//' crates/deform6/tests/support/vbp.rs | grep -c 'deform6'
0
$ grep -vE '^\s*//' crates/deform6/tests/support/source.rs | grep -c 'deform6'
0
```

## Threat mitigations

| Threat | State |
|---|---|
| T-02-36, a one-directional assertion | Mitigated. Both set differences computed and asserted, in `two_directional_diff`, at both the object and the procedure layer. Two doctored-input tests (real corpus data, one side doctored) prove a subset check passes where the two-directional check fails. |
| T-02-37, an exclusion applied to one side only | Mitigated. `removing_the_absent_source_object_from_only_the_declared_side_makes_recovered_exceed_declared` proves the one-sided-exclusion failure shape directly, using Edge-detection's real recovered and declared data. |
| T-02-38, the harness sharing a reader with the library | Mitigated and measured. Both grep commands above are 0. `differential.rs` itself does call `deform6` (it must, to get the recovered side); the two harness readers (`support/vbp.rs`, `support/source.rs`) do not. |
| T-02-39, a comparison against the tool's own output | Mitigated. Every declared-side value comes from `support::vbp` or `support::source`, never from a second `deform6` call. The module doc comment states this. |
| T-02-40, the harness over a hostile corpus | Accepted, as the plan's threat model states: a vendored, fixed corpus this repository controls. |

## Known Stubs

None.

## Deferred Issues

A future plan pinning the recovery ratio needs `program_counts`, which
lives in `differential.rs` per this plan's own documented choice; that
plan will need to decide its exact inclusion mechanism (`#[path = ...]`
against a file not named `mod.rs`, or moving the function). Not resolved
here, since it is out of this plan's scope and the mechanism choice
belongs to the plan that actually needs it.

## Threat Flags

None. This plan adds no network endpoint, no auth path, and no new file
access path beyond reading the same vendored corpus every other test in
this phase reads.

## Commits

| Commit | Subject |
|---|---|
| `52ac1ba` | Compare the recovered and declared object lists in both directions |
| `ca5b31c` | Add an independent source reader that counts public procedures |
| `6d4ec4c` | Compare the recovered and declared procedure names in both directions |

`git rev-list --count 16d354c..HEAD` is 3, one commit per task, each
carrying its own tests.

## Self-Check: PASSED

`crates/deform6/tests/differential.rs` exists, 767 lines.
`crates/deform6/tests/support/source.rs` exists, 155 lines.
`crates/deform6/tests/support/mod.rs` and `crates/deform6/tests/support_selftest.rs`
carry the expected additions. All three commit hashes (`52ac1ba`,
`ca5b31c`, `6d4ec4c`) resolve in this branch's history. The gate table
above was run on the committed tree, with a clean working tree confirmed
by `git status --short` before this file was written.
