---
phase: 03-forms
plan: 18
subsystem: forms
tags: [vb6, event-handler-table, handler-address, wiring, gap-closure, requirements-honesty]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-09's StubHandler::handler_address and read_event_table (the value this plan carries); plan 03-11's narrowed FRM-06 wording, which names the address as one of its three claimed facts; plan 03-15's blobs.rs discipline (reach the value only through deform6::inspect, prove it fails), copied here for events.rs"
provides:
  - "EventReport::Named and EventReport::BoundUnnamed each carry a handler_address: Option<u32> field; report_events fills it from EventSlot::Bound's own decoded handler and invents nothing; EventReport::Unbound gains no field"
  - "crates/deform6/tests/events.rs: the end to end proof, through deform6::inspect only, that a live run surfaces the address for every bound slot in two corpus programs, with a hand-decoded second read that never calls decode_stub or reads StubHandler"
  - "print_event prints a bound slot's own handler address as eight hexadecimal digits with a 0x prefix, or states plainly that the address is not decoded; an unbound slot is unchanged"
  - "FRM-06 in REQUIREMENTS.md moved to [x], backed by this session's own corpus-wide measurement; FRM-03 moved back to [ ], backed by the same sweep"
affects: ["04-04-frm-and-frx-writer (a phase 4 writer that reads Report.forms now has a handler address on every bound slot, if a future requirement needs it)"]

actuals:
  tokens: 6803
  tasks: 3
  commits: 3
plan_head_before: 574165c
commits: 3

tech-stack:
  added: []
  patterns:
    - "a second, independent decoder written inside the test file itself (compute_handler_address_by_hand) rather than a call to the production decode_stub, matching blobs.rs's own discipline of checking a signature by hand instead of calling frx::sniff_format: a second read is what makes an end to end comparison mean something"
    - "the verify gate's own substring grep for 'decode_stub' matches any identifier that merely contains that substring, not only a call to the function; a helper name must be chosen to share no substring with either forbidden identifier, not only to avoid literally calling it"

key-files:
  created:
    - crates/deform6/tests/events.rs
  modified:
    - crates/deform6/src/vb/controlinfo.rs
    - crates/deform6-cli/src/main.rs
    - crates/deform6-cli/tests/cli.rs
    - .planning/REQUIREMENTS.md

key-decisions:
  - "This session's own null delimited sweep found Fast_Flames.exe carries three bound event slots, not the one this plan's own pre-measurement text named (frmFire slot 8, plus cmdStop slot 0 and cmdStart slot 0, both missed by whatever measurement the plan text drew its number from). The plan's own instructions warned explicitly to retake every count rather than trust the plan text, and this is exactly the trap: crates/deform6/tests/events.rs asserts the exact count this session measured (3), not the plan's stated 1."
  - "The verify gate's grep for 'decode_stub' and 'StubHandler' matches any occurrence of those characters, including inside a longer identifier. The first draft named the hand decoder decode_stub_by_hand, which matched and failed the gate even though the function never calls the production decode_stub. Renamed to compute_handler_address_by_hand, which shares no substring with either forbidden identifier."
  - "FRM-03 returns to [ ] and FRM-06 moves to [x], each decided from this session's own corpus-wide measurement, per the plan's own instruction to measure first and change a checkbox only after. Neither requirement's wording changed."
  - "The printed handler address uses eight hexadecimal digits with a 0x prefix ({:#010x}), matching print_report's own Header line's format for a full virtual address, rather than the unpadded {:#x} format print_property and the CLSID caveat use for a small in-file byte offset: a handler address is a full address the same shape the Header line already prints, not a small position inside one file."
  - "The seven controls this sweep found with no event slot line are every one the same case: a Line control (STRUCTURES.md's cType for Line carries no ControlInfo entry at all), so ControlInfoTable::read joins no entry to that name and read_event_table is never reached for it. None reaches read_event_table's own unsupported_control_type branch. This is the file itself declaring no event slots for that control, which FRM-06 already tolerates; no code change follows from this finding, per this task's own scope boundary."

patterns-established: []

requirements-completed: [FRM-06]

coverage:
  - id: D1
    description: "EventReport::Named and EventReport::BoundUnnamed each carry a handler_address: Option<u32> field, filled by report_events from EventSlot::Bound's own decoded handler; crates/deform6/tests/events.rs proves this end to end, through deform6::inspect only, over two corpus programs, with a hand-decoded second read that never calls decode_stub or reads StubHandler"
    requirement: "FRM-06"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/controlinfo.rs#tests::a_bound_slot_with_a_decoded_handler_carries_its_address_in_both_no_name_states"
        status: pass
      - kind: e2e
        ref: "crates/deform6/tests/events.rs#fast_flames_frm_fire_bound_handler_addresses_match_an_independent_hand_decode"
        status: pass
      - kind: e2e
        ref: "crates/deform6/tests/events.rs#mandelbrot_frm_fractal_bound_handler_addresses_match_an_independent_hand_decode"
        status: pass
    human_judgment: false
  - id: D2
    description: "print_event prints a bound slot's own handler address as eight hexadecimal digits with a 0x prefix, or states plainly the address is not decoded; an unbound slot prints no address, no placeholder, no zero, unchanged from before this plan"
    requirement: "FRM-06"
    verification:
      - kind: integration
        ref: "crates/deform6-cli/tests/cli.rs#gradient_sample_prints_the_bound_handler_address_and_no_unbound_slot_carries_one"
        status: pass
      - kind: other
        ref: "cargo run -p deform6-cli --bin deform6 -- inspect corpus/vb6-code/Fire-effect/Fast_Flames.exe | grep -E 'event slot 8: bound.*0x[0-9a-f]+' (prints: event slot 8: bound, handler at 0x00404640, name not decoded. Run with --event-name-table to supply one.)"
        status: pass
    human_judgment: false
  - id: D3
    description: "FRM-03 and FRM-06 checkboxes in REQUIREMENTS.md each match a measurement this session took over all 44 corpus programs with a null delimited loop; neither requirement's wording changed"
    requirement: "FRM-06"
    verification:
      - kind: other
        ref: "grep -n 'FRM-03\\|FRM-06' .planning/REQUIREMENTS.md (FRM-03 reads [ ], FRM-06 reads [x])"
        status: pass
      - kind: other
        ref: "SHA=$(git rev-parse HEAD) && git diff --name-only \"$SHA^..$SHA\" (names .planning/REQUIREMENTS.md only)"
        status: pass
    human_judgment: false

duration: single session
completed: 2026-09-11
status: complete
---

# Phase 3 Plan 18: Wire the Handler Address Into the Event Report Summary

**`StubHandler::handler_address` now reaches `EventReport`, a live `inspect` run and the terminal, closing the one code gap the phase 3 re-verification found; FRM-06 moves to `[x]` and FRM-03 returns to `[ ]`, each backed by a fresh corpus-wide measurement.**

## Performance

- **Duration:** single session
- **Completed:** 2026-09-11T16:39:10Z
- **Tasks:** 3
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments

- `EventReport::Named` and `EventReport::BoundUnnamed` each gained a `handler_address: Option<u32>` field. `report_events` fills it from `EventSlot::Bound`'s own decoded `handler` and invents nothing; `EventReport::Unbound` gained no field, since an unbound slot has no handler to carry one for.
- `crates/deform6/tests/events.rs` is the end to end proof, reached only through `deform6::inspect`, that this wiring works: over `Fast_Flames.exe`'s `frmFire` (3 bound slots this session measured: `frmFire` itself slot 8, `cmdStop` slot 0, `cmdStart` slot 0) and `Mandelbrot.exe`'s `frmFractal` (7 bound slots: `frmFractal` slot 6, `CmdReset` slot 0, `scrAccuracy` slot 0, `CmdRedraw` slot 0, `PicDraw` slots 13, 14, 15), the address `deform6::inspect` reports for every bound slot equals an address this test's own independent hand decoder computes from the same executable's own bytes at run time. The hand decoder reads the four signed bytes at the stub plus `0x09` and computes the stub plus 13 plus that value itself; it never calls `decode_stub` and never reads a `StubHandler`.
- `print_event` prints a bound slot's own handler address as eight hexadecimal digits with a `0x` prefix (`{:#010x}`, matching `print_report`'s own `Header` line's format for a full address), or states plainly `handler address not decoded` when the stub itself did not resolve. An unbound slot prints exactly what it printed before this plan.
- `crates/deform6-cli/tests/cli.rs` gained a test that reads the address out of a live run's own standard output and confirms no unbound slot line carries one, never comparing against a corpus address written in as a literal.
- `REQUIREMENTS.md`'s `FRM-06` checkbox moved to `[x]`: a fresh sweep of all 44 corpus executables, walked with a null delimited loop, found 686 controls, 679 carrying at least one event slot line, 396 bound slots, and every one of the 396 now carries a decoded address (0 unreadable pointer defects anywhere in the sweep). The 7 controls with no event slot line are every one a `Line` control that the `ControlInfo` array itself carries no entry for; none reaches `read_event_table`'s own `unsupported_control_type` branch. `FRM-03`'s checkbox returned to `[ ]`: the same sweep found 122 named property values against 683 records reporting present and not decoded, over 805 property records and five distinct property names (`Caption` 51, `BackColor` 36, `BorderStyle` 30, `Position` 4, `WindowState` 1), which does not support "the property values of every control."

## Task Commits

Each task was committed atomically:

1. **Task 1: Carry the handler address into the event report and prove it through a live inspect** - `7293e64` (feat, tdd="true")
2. **Task 2: Print the handler address of a bound slot, and no address for an unbound one** - `b77e0b5` (feat, tdd="true")
3. **Task 3: Set the FRM-03 and FRM-06 checkboxes to what a measurement supports** - `e18095e` (docs)

**Plan metadata:** commit follows this SUMMARY.

_Note: tasks 1 and 2 carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching every prior plan in this phase), each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below, then reverted to the correct GREEN implementation before committing code and tests together in one commit per task. Task 3 is documentation only and carries no source edit, in its own commit, per `AGENTS.md`'s "documentation in a different commit" rule._

## Files Created/Modified

- `crates/deform6/src/vb/controlinfo.rs` - `EventReport::Named`/`BoundUnnamed` gain `handler_address`; `report_events` fills it; `no_name_message`'s `BoundUnnamed` arm gains `..`; one new unit test; three existing tests updated for the new field (Task 1)
- `crates/deform6/tests/events.rs` - new; the end to end proof, two tests, over two corpus programs (Task 1)
- `crates/deform6-cli/src/main.rs` - `print_event` prints the address or states it is not decoded, for a bound slot; unbound unchanged (Task 2)
- `crates/deform6-cli/tests/cli.rs` - new test reading the address out of a live run (Task 2)
- `.planning/REQUIREMENTS.md` - `FRM-06` moved to `[x]`, `FRM-03` moved to `[ ]`, both wordings unchanged (Task 3)

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential: this session's own measurement found Fast_Flames.exe carries three bound event slots, not the plan text's own stated one, and `events.rs` asserts the exact count this session measured rather than the plan's stale number; and the verify gate's substring grep for `decode_stub` matches any identifier containing those characters, which forced the hand decoder's name away from `decode_stub_by_hand` to `compute_handler_address_by_hand`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Renamed the hand decoder to avoid the literal substring `decode_stub`**
- **Found during:** Task 1, running the verify gate's exact-substring grep after the first draft named the function `decode_stub_by_hand`
- **Issue:** `grep -v '^[[:space:]]*//' crates/deform6/tests/events.rs | grep -c 'decode_stub\|StubHandler'` matched the function's own name as a substring, even though the function is an independent decoder that never calls the production `decode_stub`.
- **Fix:** Renamed to `compute_handler_address_by_hand`, which shares no substring with either forbidden identifier.
- **Files modified:** `crates/deform6/tests/events.rs`
- **Verification:** the grep count returns `0`; `cargo test -p deform6 --test events` still passes both tests
- **Committed in:** `7293e64` (Task 1 commit)

**2. [Rule 3 - Blocking] Fixed a `clippy::len_zero` lint on the new test file**
- **Found during:** Task 1, running `cargo clippy --all-targets -- -D warnings`
- **Issue:** `recovered.len() > 0` triggered `clippy::len_zero`.
- **Fix:** Changed to `!recovered.is_empty()`.
- **Files modified:** `crates/deform6/tests/events.rs`
- **Verification:** `cargo clippy --all-targets -- -D warnings` passes clean
- **Committed in:** `7293e64` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3, blocking).
**Impact on plan:** Both fixes were necessary for the mandatory gate to pass. No scope creep: each fix is a rename or a lint-clean rewrite with no behavior change.

## Break-on-purpose evidence

**Task 1's `report_events` bound arm** - `let handler_address = handler.map(|h| h.handler_address);` changed to `let handler_address = None;`. `cargo test -p deform6 --test events` run once:

```
running 2 tests
test fast_flames_frm_fire_bound_handler_addresses_match_an_independent_hand_decode ... FAILED
test mandelbrot_frm_fractal_bound_handler_addresses_match_an_independent_hand_decode ... FAILED

failures:

---- fast_flames_frm_fire_bound_handler_addresses_match_an_independent_hand_decode stdout ----

thread 'fast_flames_frm_fire_bound_handler_addresses_match_an_independent_hand_decode' panicked at crates/deform6/tests/events.rs:216:5:
assertion `left == right` failed: frmFire: the handler addresses deform6::inspect reports must equal this file's own independent hand decode
  left: []
 right: [("cmdStart", 0, 4208800), ("cmdStop", 0, 4210192), ("frmFire", 8, 4212288)]

---- mandelbrot_frm_fractal_bound_handler_addresses_match_an_independent_hand_decode stdout ----

thread 'mandelbrot_frm_fractal_bound_handler_addresses_match_an_independent_hand_decode' panicked at crates/deform6/tests/events.rs:216:5:
assertion `left == right` failed: frmFractal: the handler addresses deform6::inspect reports must equal this file's own independent hand decode
  left: []
 right: [("CmdRedraw", 0, 4205056), ("CmdReset", 0, 4206944), ("PicDraw", 13, 4207552), ("PicDraw", 14, 4208208), ("PicDraw", 15, 4209232), ("frmFractal", 6, 4207408), ("scrAccuracy", 0, 4211216)]

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Reverted to `handler.map(|h| h.handler_address)` before committing `7293e64`.

**Task 2's `print_event` `BoundUnnamed` arm** - changed back to the pre-plan text with no address (`"{indent}  event slot {index}: bound, not decoded. Run with --event-name-table to supply one."`). `cargo test -p deform6-cli --test cli gradient_sample_prints_the_bound_handler_address` run once:

```
running 1 test
test gradient_sample_prints_the_bound_handler_address_and_no_unbound_slot_carries_one ... FAILED

failures:

---- gradient_sample_prints_the_bound_handler_address_and_no_unbound_slot_carries_one stdout ----

thread 'gradient_sample_prints_the_bound_handler_address_and_no_unbound_slot_carries_one' panicked at crates/deform6-cli/tests/cli.rs:713:5:
the bound slot line did not print a handler address: "        event slot 0: bound, not decoded. Run with --event-name-table to supply one."

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.38s
```

Reverted to the address-printing arms before committing `b77e0b5`.

## Measurements this session took and recorded

**Every count below was retaken with a null delimited loop** (`find corpus -type f -iname '*.exe' -print0`), walking all 44 corpus executables; none was dropped.

**FRM-06's event slot facts:** 686 controls, 679 carrying at least one event slot line, 7 carrying none, 396 bound slots, 12072 unbound slots (396 + 12072 = 12468 total slot lines), and all 396 bound slots now carry a decoded address (0 unreadable pointer defects across the sweep). The 7 no-slot controls, named and classified by the field that decides it (`ControlInfoTable::entries` carrying no entry for that name, never `EventTable::unsupported_control_type`):

| Program | Control | Case |
|---|---|---|
| `Artificial-life/Artificial Life.exe` | `Line1` | 1 (no `ControlInfo` entry) |
| `Artificial-life/Artificial Life.exe` | `Line2` | 1 (no `ControlInfo` entry) |
| `Histograms-advanced/Advanced Histogram Viewer.exe` | `Line1` (form 1) | 1 (no `ControlInfo` entry) |
| `Histograms-advanced/Advanced Histogram Viewer.exe` | `Line1` (form 2) | 1 (no `ControlInfo` entry) |
| `Histograms-advanced/Advanced Histogram Viewer.exe` | `Line2` | 1 (no `ControlInfo` entry) |
| `Histograms-basic/Basic Histogram Viewer.exe` | `Line1` | 1 (no `ControlInfo` entry) |
| `Levels-effect/Image Levels.exe` | `Line1` | 1 (no `ControlInfo` entry) |

Every one is case 1: a `Line` control the file itself declares no event slots for. None is case 2 (`read_event_table` choosing no header size for a control type it does not support). `FRM-06` is therefore satisfied for all 7, and the checkbox moves to `[x]`.

**FRM-03's property facts:** 122 named property values against 683 records reporting present and not decoded, over 805 property records and five distinct property names: `Caption` 51, `BackColor` 36, `BorderStyle` 30, `Position` 4, `WindowState` 1. Per decision D-01 (`03-CONTEXT.md`), this is the designed consequence of the safe-provenance opcode subset, not a defect this plan fixes. `FRM-03` claims the property values of every control; the measurement does not support that sentence, so the checkbox returns to `[ ]`.

These numbers agree exactly with the plan's own pre-measurement text for FRM-03 and for the aggregate event-slot counts. They disagree with the plan's own pre-measurement text for `Fast_Flames.exe`'s individual bound-slot count (the plan named one bound slot; this session measured three), confirming the plan's own stated trap: a number in the plan text is a lead, not a fact.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every symbol this plan's own artifact table names (`handler_address` on `EventReport::Named`/`BoundUnnamed`, `events.rs`, the `print_event` address lines, the `cli.rs` test, the two `REQUIREMENTS.md` checkboxes) is implemented, wired into the production path, and tested against real corpus bytes; none is a placeholder.

## Threat Flags

None beyond what this plan's own `<threat_model>` already names and mitigates. T-03-93 (a hostile `rel32` driving the address): no new arithmetic entered `src/`; `decode_stub` is unchanged, still `checked_add`/`checked_add_signed`. T-03-94 (a placeholder address printed as read from the file): the field is optional and a bound slot with no decoded handler prints `handler address not decoded`, confirmed by the verify command asserting no unbound line carries an address. T-03-95 (an address printed for a slot with no handler): `EventReport::Unbound` gains no field, so the type itself makes the value unreachable; confirmed by a live-run grep. T-03-96 (a wired value that silently stops reaching the report again): `events.rs` reaches the value only through `deform6::inspect` and the break-on-purpose evidence above proves it fails when the wiring is removed. T-03-97 (a corpus derived address committed as a test fixture): every expected address is computed from the committed corpus files at run time; the grep for `decode_stub`/`StubHandler` in `events.rs` returns `0`, and no address literal is written into either test file.

## Next Phase Readiness

- The one code gap phase 3's re-verification found (`handler_address` computed and unit-tested, reaching nothing) is closed: `EventReport`, a live `deform6 inspect` run and the terminal all carry the address now, and `crates/deform6/tests/events.rs` proves it end to end and proves it can fail.
- `FRM-06` is `[x]`, backed by a corpus-wide measurement; `FRM-03` is `[ ]`, backed by the same sweep. No requirement wording changed and no premature mark was made.
- `FRM-04` (the CLSID wording), the `.frx` writer, `Map Editor.exe`'s remaining refusal, `frmPassGen`'s menu nesting ambiguity, and fuzzing are all untouched, per this plan's own scope boundary; each stays exactly where the phase 3 re-verification routed it.
- Ready for `/gsd-verify-work` against this gap closure plan, and for the phase 3 re-verification to re-run against a shipped, live-confirmed `handler_address`.

---
*Phase: 03-forms*
*Completed: 2026-09-11*
