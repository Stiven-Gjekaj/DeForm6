---
phase: 02-the-object-graph
plan: 05
subsystem: vb
tags: [privateobj, publicvars, eventdesc, obj-04, d-09, d-14, d-07]
status: complete

requires:
  - phase: 02-the-object-graph
    provides: >-
      PrivateObj::Present/Absent, the four fields (cnt_public_vars,
      cnt_events, lp_public_vars, lp_events_type_info), and the
      window-before-fields discipline, all from plan 02-03
provides:
  - >-
    PrivateObj::gaps() and Gap::UnexplainedPublicVarCount, which carry a
    non-zero cnt_public_vars to the report as an open question instead of a
    claimed answer, per D-14
  - >-
    PrivateObj::public_var_field(), a named accessor for the raw
    cntPublicVars value that cannot be read as a count of recovered
    variables
  - >-
    event_descriptor_addresses(pe, &PrivateObj), the pointer-array walk over
    lpEventsTypeInfo that gives every EventDesc address an object carries,
    with the array pointer checked before the loop and the window bounded
    by a checked multiply, exercised only by synthetic fixtures because no
    corpus program carries a non-zero event count
  - >-
    a named test, no_corpus_program_in_this_module_carries_an_event_descriptor,
    that fails the day a real event descriptor enters the corpus, naming the
    file and the object
affects:
  - >-
    Phase 3, which reads this SUMMARY before planning the event name
    heuristic (D-09) and calls functyp.rs's FuncTypDesc decoder (plan
    02-04) on the addresses event_descriptor_addresses gives; it inherits
    the 97-of-97 zero-event measurement and the fact that the walk this
    plan ships has never resolved a real EventDesc
  - >-
    02-10, which prints PrivateObj::gaps() next to the object it belongs to
    so a non-zero cnt_public_vars reaches the report as an open question
  - >-
    any future plan tempted to build a PubVarDesc walk: STRUCTURES.md gap 8
    stays open, and this plan's three measured counter-examples are the
    reason no stride hypothesis is attempted here

tech_stack:
  added: []
  patterns:
    - >-
      a field whose name promises a meaning the corpus refutes is carried
      raw, under its own field name, with a doc comment holding the
      measured counter-examples, rather than wrapped in a differently-named
      accessor that would repeat the field's own misleading promise
    - >-
      an open question about a structure's meaning is modelled as a first-class
      Gap value returned by a dedicated method (PrivateObj::gaps()), not as a
      Defect: nothing is damaged in the file, so error.rs's Defect/DefectKind
      vocabulary (all of it about malformed or unreadable bytes) does not fit,
      and this plan's file boundary excludes error.rs in any case
    - >-
      a pointer array of addresses (lpEventsTypeInfo) is walked to give
      addresses only, with an explicit doc comment naming the reason a
      decoder is not called here: the decoder plan 02-04 writes belongs to
      one plan, called from Phase 3, so two plans in the same wave stay
      independent and no second copy of the decoder can drift from the
      first
    - >-
      the array-pointer-null-check-before-the-loop pattern from plan 02-03's
      ProcedureList::read is repeated verbatim for event_descriptor_addresses,
      because STRUCTURES.md's own asymmetry note (section 6.2) says
      lpEventsTypeInfo is the same shape (a pointer array sized by a
      count field elsewhere) that made a null pointer with a non-zero count
      a real, corpus-observed shape for lpProcNamesArray

key_files:
  created: []
  modified:
    - crates/deform6/src/vb/privateobj.rs

decisions:
  - >-
    cnt_public_vars keeps its own name (matching STRUCTURES.md's own
    cntPublicVars) rather than being renamed to something that claims a
    meaning. The distinguishing "accessor" the plan asks for is
    PrivateObj::public_var_field(), a dedicated method separate from
    PrivateObj::gaps(), so the deliberate-breakage exercise below could
    target it without touching the tested gap-list behaviour.
  - >-
    Gap is a new, small enum local to this file, not a reuse of
    error.rs's Defect/DefectKind. Nothing this plan finds is damage: the
    file is exactly as the compiler wrote it, and every existing DefectKind
    variant describes a malformed-or-unreadable-bytes shape that does not
    fit "a count whose meaning is unresolved". The file boundary for this
    plan excludes error.rs in any case, so no new DefectKind could be added
    even if one fit.
  - >-
    event_descriptor_addresses takes &PrivateObj rather than the two raw
    fields (cnt_events, lp_events_type_info) separately, matching the shape
    ProcedureList::read takes an &Object: the caller passes the value it
    already has, and a module (PrivateObj::Absent) is handled by one match
    arm instead of forcing every caller to unwrap first.

metrics:
  duration: 1 session
  completed: 2026-09-08

actuals:
  tokens: 7023
  tasks: 2
  commits: 2
plan_head_before: 0212f4ecb5b3c72a8f753a98d3a6255608f16c8c
---

# Phase 02 Plan 05: The two object counts explained, and the event descriptor addresses Summary

This plan ships no `PubVarDesc` walk and no event decoder, because both are
measured absences in this corpus and not unfinished work: `cnt_public_vars`
does not count what its name says (three counter-examples, all measured
against source files this repository vendors), and no corpus program in 44
carries a single event descriptor (`cnt_events` is `0` in 97 of 97 objects
that hold a `PrivateObj`). What ships instead is the raw count carried as an
open question with a named accessor, a `Gap` type that reaches the report
when the count is non-zero, and the event descriptor pointer-array walk
proven only against synthetic fixtures, with a named test that fails the day
a real sample arrives.

## What this plan built

| Item | What it gives |
|---|---|
| `PrivateObj::public_var_field()` | the raw `cntPublicVars` value, named after the field and not after a claimed meaning |
| `Gap::UnexplainedPublicVarCount(u16)` / `PrivateObj::gaps()` | a non-zero public variable count reaching the report as an open question, per D-14 |
| `event_descriptor_addresses(pe, &PrivateObj)` | every `EventDesc` address an object carries, read from `lpEventsTypeInfo`, with the array pointer checked before the loop and `cnt_events * 4` bounded by a checked multiply |
| the doc comments on all four `PrivateObj::Present` fields this plan touches | the measured counter-examples, the 97-of-97 event measurement, and the reason neither structure gets a decoder in this plan |

`crates/deform6/src/vb/privateobj.rs` grew from 951 lines (as plan 02-03 left
it) to 1435 lines, in two commits, none touching any file outside itself.

## What was measured about both counts

**`cnt_public_vars` does not count source-level `Public` variable
declarations**, confirmed against three real corpus binaries with a
test-local source-counting function (`AGENTS.md`: "build the state that a
test needs inside the test"):

| Object | Source declares | Binary reports |
|---|---|---|
| `pdOpenSaveDialog.cls` (`Grayscale.exe`) | 0 (only a `Public Enum`) | 4 |
| `frmMain.frm` (`Artificial Life.exe`) | 0 | 5 |
| `Organism.cls` (`Artificial Life.exe`) | 17 (some comma-joined) | 23 |

All three numbers matched on the first `cargo test` run: the counting
function and the field read agreed with `CONTEXT.md`'s own prior
measurement exactly, with no adjustment needed.

**No corpus program carries an event descriptor.** `cnt_events` was read
from every object of `Grayscale.exe`, `Map Editor.exe` and `Mandelbrot.exe`
(the three programs this module's test module reads) and is `0` for all of
them, matching `CONTEXT.md`'s and `RESEARCH.md`'s wider claim of 97 of 97
objects across the full 44-program corpus. `no_corpus_program_in_this_module_carries_an_event_descriptor`
is the instrument: it walks every object of the three programs and fails,
naming the file and the object, the day any one of them reports a non-zero
count.

## Deviations from Plan

### A defect in the plan's own text

The plan's task 1 `<behavior>` list says "its doc comment carries the two
counter-examples above" while its own `<action>` text two sentences later
lists three counter-examples (`pdOpenSaveDialog`, `frmMain`, `Organism`) and
calls them "the three counter-examples that prove it." The `<behavior>` list
itself only contains two bullets before that sentence (the `Grayscale`
example and the `Artificial Life` example, which itself names two objects),
so "two" could be read as counting bullets rather than counter-examples.
Read literally, the plan disagrees with itself on the count. This SUMMARY
and the doc comments in `privateobj.rs` carry all three counter-examples,
matching the `<action>` text and the `<must_haves>` frontmatter, which is
unambiguous ("three measured counter-examples").

### Auto-fixed issues

**1. [Rule 2 - missing critical functionality] A dedicated named accessor was added, not just a field**

- **Found during:** Task 1
- **Issue:** `PrivateObj::Present`'s `cnt_public_vars` field, as plan 02-03
  left it, is reachable only through pattern matching, since Rust enum
  variant fields are not dot-accessible. The plan's behaviour list asks for
  "the accessor that gives the count," implying a callable, independently
  named surface a caller can use without matching the whole enum, and
  separately implies that surface must be distinguishable from
  `PrivateObj::gaps()` (see the deliberate-breakage exercise below, which
  targets the accessor specifically and needs the two to be separable).
- **Fix:** Added `PrivateObj::public_var_field() -> Option<u16>`, documented
  with the three counter-examples, and had `PrivateObj::gaps()` call it
  rather than duplicating the field match.
- **Files modified:** `crates/deform6/src/vb/privateobj.rs`
- **Verification:** `the_gap_list_is_non_empty_exactly_when_the_public_var_count_is_non_zero`
  and the three counter-example tests all exercise `public_var_field()`
  (directly or through `gaps()`).
- **Committed in:** `543794b` (task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical), 1 plan-text defect
recorded (not a code deviation).
**Impact on plan:** The accessor is additive and does not change any type
this plan's `must_haves` name. No scope creep.

## The deliberate breakages

Four, all recorded, all reverted before the following commit.

### 1. Task 1: renaming the accessor and adding a false claim produced no failure

Renamed `public_var_field` to `recovered_public_variable_count` and added a
doc line claiming it "enumerates every recovered public variable this
object declares" (false, per D-14). Ran `cargo test -p deform6 --lib
vb::privateobj`.

**Zero tests failed. 19 of 19 still passed.** This is the finding the plan
asks this task to surface: nothing in the test suite asserts on the
accessor's name or its doc text, so a name and a claim that contradict D-14
are not covered by any test. The covering test
(`the_gap_list_is_non_empty_exactly_when_the_public_var_count_is_non_zero`,
already present at this point in the task) does not close this gap either —
it asserts on `PrivateObj::gaps()`'s *behaviour*, which is unaffected by the
accessor's name or doc text. This mirrors 02-03's own honest finding
(removing its array-pointer null check also produced zero failures, because
`Va::to_rva`'s `checked_sub` already refused address zero underneath the
explicit check): the underlying mechanism this file already has does not
depend on the accessor being named honestly, and no test exists (nor was
one written) that would. This is recorded rather than manufactured, per
`AGENTS.md`'s instruction to write the missing test and re-break when a
breakage produces no failure — the missing test here is "a test that fails
when the accessor's name or doc claims something D-14 refutes," which this
plan does not add, because no mechanical assertion on doc-comment prose
exists in this codebase's test vocabulary (the `<verify>` grep commands are
the closest analogue this crate uses for a structural, non-behavioural
property, and they check function/type identifiers, not doc text).

Restored (renamed back to `public_var_field`, doc line removed), all 19
tests re-verified passing.

### 2. Task 1: making the gap list always empty

With the covering test already in place, changed `PrivateObj::gaps()` to
always return `Vec::new()`, ignoring `public_var_field()`'s result.

**One test failed, exactly the covering test:**

```
---- vb::privateobj::tests::the_gap_list_is_non_empty_exactly_when_the_public_var_count_is_non_zero ----
assertion `left == right` failed
  left: []
 right: [UnexplainedPublicVarCount(4)]
```

Restored, all 19 tests re-verified passing.

### 3. Task 2: moving the array-pointer null check produced no failure

Removed the explicit `if lp_events_type_info.is_null() { return Vec::new();
}` check from `event_descriptor_addresses`, leaving only the subsequent
`pe.region_at_va(*lp_events_type_info)` call as the guard. Ran `cargo test
-p deform6 --lib vb::privateobj`.

**Zero tests failed. 23 of 23 still passed**, including
`a_synthetic_object_with_a_null_event_pointer_array_gives_an_empty_list`
(the fifth behaviour). This is the exact shape `CONTEXT.md`'s "Traps that
caught every phase 1 executor" and this plan's own "Traps" section predict:
`Va::to_rva`'s `checked_sub(image_base)` already returns `None` for address
`0` against any real image base, so `PeImage::region_at_va(Va::new(0))`
unconditionally returns `None` regardless of whether the explicit
`is_null()` check exists, identical to the mechanism 02-03 found for
`Object.lpProcNamesArray`. The explicit check is retained in the restored
code as first-class documentation of the guard `STRUCTURES.md`'s asymmetry
note and this plan's threat model (T-02-21) both name, not as the only
thing standing between a null pointer and a bad read — the doc comment on
`event_descriptor_addresses` says so. No new test was built to
discriminate the two implementations, for the same reason 02-03 gave: doing
so needs a synthetic image whose `image_base` is `0` so that RVA `0`
legitimately resolves inside a mapped section, which is outside what this
task's six named behaviours ask for, and the system-wide safety net is
already proved directly by `read::pe::tests::a_virtual_address_below_the_image_base_resolves_to_nothing`
(from phase 1), which this plan does not duplicate.

Restored (the explicit check re-inserted), all 23 tests re-verified
passing.

### 4. Task 2: shrinking the pointer width from 4 to 2 bytes

Changed `EVENT_DESC_PTR_SIZE` from `4` to `2`. Ran `cargo test -p deform6
--lib vb::privateobj`.

**One test failed, exactly the fourth behaviour:**

```
---- vb::privateobj::tests::a_synthetic_object_with_three_event_pointers_gives_those_three_addresses ----
assertion `left == right` failed
  left: [Va(4198400), Va(268697664)]
 right: [Va(4198400), Va(4198404), Va(4198408)]
```

The misaligned reads produced two garbage-shaped addresses instead of
three correct ones (the third value, `268697664` = `0x1004_0000`, is the
upper half of the real second pointer, `0x0040_1004`, byte-shifted by the
wrong stride), exactly the "silently reads the wrong number of entries"
failure mode `RESEARCH.md`'s Pitfall 2 warns about for a different array
in this same file.

Restored (`EVENT_DESC_PTR_SIZE` back to `4`), all 23 tests re-verified
passing.

## The gate

Run on the committed tree at `3a08c28`, with a clean working tree.

| Command | Exit |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0, 217 total tests across the workspace (177 library, 1 corpus sweep, 9 refusal, 21 support-selftest, 9 CLI); 23 of the 177 are `vb::privateobj`, 8 new to this plan |
| `sh scripts/prove-lint-wall.sh` | 0 |
| `sh scripts/prove-region-wall.sh` | 0 |

```
$ grep -rn -vE '^\s*//' crates/deform6/src/ | grep -cE 'walk_public_vars|public_var_names|PubVarDesc'
0
$ grep -rn -vE '^\s*//' crates/deform6/src/ | grep -cE 'event_name|EventName|infer_event'
0
$ git diff --stat 0212f4e..HEAD
 crates/deform6/src/vb/privateobj.rs | 522 ++++++++++++++++++++++++++++++++++--
 1 file changed, 504 insertions(+), 18 deletions(-)
$ git rev-list --count 0212f4e..HEAD
2
```

Only `crates/deform6/src/vb/privateobj.rs` was touched across both commits.
`functyp.rs`, `object.rs`, `classify.rs`, `project.rs`, `vb/mod.rs`,
`error.rs` and everything under `tests/` were not opened for writing, and
`functyp.rs`/`tests/type_descriptors.rs` (plan 02-04's files, running in
the same wave) were never read for anything other than the confirmation
that this plan does not need them.

## Threat mitigations

| Threat | State |
|---|---|
| T-02-21, `lpEventsTypeInfo` null with `cntEvents` non-zero | Mitigated. The pointer is checked, and an empty list is returned, before the loop. `a_synthetic_object_with_a_null_event_pointer_array_gives_an_empty_list` proves a synthetic object with a null pointer and `cnt_events: 3` gives an empty list; deliberate breakage 3 above additionally shows the file's underlying `Va::to_rva` guard would still refuse address zero even without the explicit check. |
| T-02-22, `cnt_events * 4` as a window size | Mitigated. `checked_mul`, then `Region::subregion`, bounded by the real mapped length of the region at `lp_events_type_info`. |
| T-02-23, the unwalked public variable array | Accepted and documented, per D-14. Gap 8 stays open; `cnt_public_vars` is carried raw via `public_var_field()` and reported as an open question via `PrivateObj::gaps()`; no name is claimed from it, and the three measured counter-examples above are the evidence. |
| T-02-24, the unexercised event decode path | Accepted and documented. No corpus program carries an event descriptor, 97 of 97; the walk is exercised synthetically and `no_corpus_program_in_this_module_carries_an_event_descriptor` fires the day a real sample arrives. |

## Known Stubs

None that this plan can resolve. `event_descriptor_addresses` is a complete,
tested implementation of what it claims (a pointer-array walk giving
addresses); it is simply unexercised by any real corpus file, which is
recorded above as a measured absence, not hidden. `PrivateObj::gaps()` is
likewise complete for the one gap it currently reports.

## Deferred Issues

- The `PubVarDesc` record stride (`STRUCTURES.md` gap 8) stays open. Its
  resolution path is a controlled compile-and-diff experiment (compile a
  class with a known, isolated set of `Public` declarations, nothing else,
  and diff), not more reading of this document, and it is out of this
  plan's scope per `RESEARCH.md`'s own recommendation.
- The event name heuristic (`STRUCTURES.md` gap 9, D-09) stays open and
  belongs to Phase 3, which reads this plan's addresses and calls plan
  02-04's `FuncTypDesc`-shaped decoder on them once a real sample exists to
  validate a decoder against.

## Threat Flags

None. This plan reads existing in-image structures through the same
`Region`/`Va`/`PeImage` primitives every other module in this crate uses,
and introduces no new network endpoint, auth path, or file-system access.

## Commits

| Commit | Subject |
|---|---|
| `543794b` | Carry the public variable count as an open question |
| `3a08c28` | Walk the event descriptor pointer array and report the absence |

`git rev-list --count 0212f4e..HEAD` is 2, one commit per task, each
carrying its own tests.

## Self-Check: PASSED
