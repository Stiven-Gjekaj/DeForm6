---
phase: 03-forms
plan: 03
subsystem: testing
tags: [vb6, frm, differential-gate, test-harness, ver-06]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-01's window-before-fields discipline and phase 3 module scaffolding; this plan builds no code in src/ at all and depends on nothing in it"
provides:
  - "support::frm::forms(), the second, independent .frm reader's corpus walk, pinned at 54 paths including the upper case corpus/public-domain/SK-MCI-Sample__VB6/MCI.FRM"
  - "support::frm::Form::read, a byte-to-Latin-1 reader that keeps corpus/vb6-code/Threshold-effect/Threshold.frm's byte 0xA9 rather than refusing the file"
  - "support::frm::parse_blocks, Block and PropertyBlock: the Begin/BeginProperty/End/EndProperty grammar, pinned at 48 Index properties and 54 root form blocks across the whole corpus"
  - "support::frm::EXCLUSIONS and excluded_reason: the one named, reasoned VER-06 exclusion for corpus/vb6-code/Hidden-Markov-model/frmHMM.frx, with frmHMM.frm beside it proved never excluded"
affects: [03-04-controltree, 03-10-differential-gate]

actuals:
  tokens: 5456
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "support::frm follows support::vbp's exact shape: a copied walk_by_extension comparing with eq_ignore_ascii_case, and a byte-to-Latin-1 Form::read that never calls the string-returning file read. The plan's own D-04 promotion decision names this the generalisation worth keeping."
    - "Block and PropertyBlock are two separate types with two separate child lists (children vs property_blocks) on one Frame-enum parse stack, so a BeginProperty block can never be mistaken for a control by construction, not by a filter applied after parsing."
    - "EXCLUSIONS is a short, explicit (path, reason) list, deliberately the opposite shape to support::rules::Rule's generic Candidate predicates, so a named upstream defect cannot be confused with the five generic VER-04 rules."

key-files:
  created:
    - crates/deform6/tests/support/frm.rs
  modified:
    - crates/deform6/tests/support/mod.rs
    - crates/deform6/tests/support_selftest.rs

key-decisions:
  - "AGENTS.md's 'put the code and its tests in the same commit' rule takes precedence over the plan's implicit TDD RED-then-GREEN commit split, for all three tasks (tdd=true), per this project's own binding convention and 03-01's own precedent. Each task's RED phase was performed and its failure evidence captured (see Deviations), then committed together with the GREEN implementation in one commit per task."
  - "PropertyBlock carries its own nested property_blocks field, one level more general than the plan's literal description ('a property block whose only field is the block name'), so a BeginProperty block nested inside another BeginProperty block (not exercised by this corpus, but not forbidden by the grammar either) parses without a special case. No corpus file needed this; it is a structural consequence of reusing one Frame enum for the parse stack, not an added feature."
  - "The whole-corpus Index and root-form-block counts are proved as unit tests inside frm.rs's own test module, per the plan's explicit instruction to put Task 1 and 2's tests there, deliberately breaking this repository's usual convention (support_selftest.rs holds every test externally) because the plan sanctions the deviation directly."

patterns-established:
  - "Task-scoped RED evidence, captured once per task before the GREEN fix and recorded in the commit message and here, rather than as a separate RED commit, is the repeatable shape for every remaining TDD task in this phase under AGENTS.md's single-commit rule."

requirements-completed: [VER-06]

coverage:
  - id: D1
    description: "support::frm::forms() walks the corpus case-insensitively and finds all 54 forms, including the upper case corpus/public-domain/SK-MCI-Sample__VB6/MCI.FRM; Form::read reads bytes and keeps byte 0xA9 in corpus/vb6-code/Threshold-effect/Threshold.frm rather than refusing the file"
    requirement: "VER-06"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/support/frm.rs#tests::forms_gives_fifty_four_paths"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support/frm.rs#tests::one_of_the_returned_paths_ends_with_the_upper_case_mci_frm"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support/frm.rs#tests::form_read_succeeds_on_the_non_utf8_corpus_file_and_keeps_byte_0xa9"
        status: pass
    human_judgment: false
  - id: D2
    description: "The Begin/BeginProperty/End/EndProperty block parser reads nested control blocks and property blocks, keeps the two lists separate, does not depend on indentation or spacing, does not truncate a value holding an equals sign, and finds 48 Index properties and 54 root form blocks across the whole corpus"
    requirement: "VER-06"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/support/frm.rs#tests::a_begin_inside_a_begin_gives_one_child_on_the_outer_block"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support/frm.rs#tests::a_begin_property_font_block_does_not_appear_in_the_child_control_list"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support/frm.rs#tests::a_line_with_four_spaces_before_the_equals_sign_parses_the_same_as_one_space"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support/frm.rs#tests::a_value_holding_an_equals_sign_is_not_truncated"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/support/frm.rs#tests::the_whole_corpus_holds_forty_eight_index_properties"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/support/frm.rs#tests::the_whole_corpus_holds_fifty_four_root_form_blocks"
        status: pass
    human_judgment: false
  - id: D3
    description: "EXCLUSIONS names exactly one upstream defect, corpus/vb6-code/Hidden-Markov-model/frmHMM.frx, with a non-empty reason naming the line ending normalisation cause; frmHMM.frm beside it, and every other corpus path, is proved never excluded; .gitattributes still marks *.frx and *.ctx as binary"
    requirement: "VER-06"
    verification:
      - kind: unit
        ref: "crates/deform6/tests/support_selftest.rs#exclusions_holds_exactly_one_entry"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support_selftest.rs#every_exclusion_reason_is_non_empty"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support_selftest.rs#the_one_exclusion_names_frm_hmm_frx_and_its_reason_names_the_line_ending_cause"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support_selftest.rs#excluded_reason_gives_some_for_the_named_frx_and_none_for_the_form_beside_it"
        status: pass
      - kind: integration
        ref: "crates/deform6/tests/support_selftest.rs#excluded_reason_gives_none_for_every_other_corpus_form"
        status: pass
      - kind: unit
        ref: "crates/deform6/tests/support_selftest.rs#gitattributes_still_marks_the_resource_extensions_as_binary"
        status: pass
    human_judgment: false

duration: 18min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 3: The Second, Independent .frm Reader Summary

**support::frm, a second, independent `.frm` reader sharing no code with `src/`: a case-insensitive 54-form corpus walk, a Latin-1 byte reader, a Begin/End block parser pinned at 48 Index properties and 54 root form blocks, and VER-06's named `frmHMM.frx` exclusion.**

## Performance

- **Duration:** 18 min (approximate; the session did not capture a literal start epoch, so this is measured from the previous plan's completion commit to this plan's final code commit)
- **Started:** 2026-09-10T09:14:00Z (approximate)
- **Completed:** 2026-09-10T09:31:47Z
- **Tasks:** 3
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- `support::frm::forms()` walks `corpus/` recursively, matching the `frm` extension without regard to case, and gives all 54 corpus forms sorted, including `corpus/public-domain/SK-MCI-Sample__VB6/MCI.FRM`, which carries an upper case extension. A case-sensitive comparison finds only 53 and reports a full pass over a set that is missing one; this was proved directly as this task's own RED evidence.
- `Form::read` reads bytes, never a string, and maps each byte to its own Latin-1 code point, so it keeps byte 0xA9 in `corpus/vb6-code/Threshold-effect/Threshold.frm` at offset 5669 rather than refusing the file the way the string-returning read does.
- `parse_blocks` reads the `Begin`/`BeginProperty`/`End`/`EndProperty` grammar line by line, independent of indentation and independent of the exact spacing before an equals sign, into `Block` (class, name, properties, property blocks, child control blocks) and `PropertyBlock` (name, properties, nested property blocks). A `BeginProperty` block never appears in a control's `children` list; the two lists are structurally separate, not filtered apart after the fact. Across the whole corpus the parser finds 48 `Index` properties and 54 root form blocks, matching the shell counts this plan's own `<verification>` block pins.
- The line splitter matches only the first `=` sign, so a caption such as `"A = B"` is not truncated; this was proved by deliberately splitting on every `=` sign first and watching the caption test fail.
- `EXCLUSIONS` names exactly one upstream defect: `corpus/vb6-code/Hidden-Markov-model/frmHMM.frx`, 56 bytes, whose own record header declares 56 bytes of payload with only 55 present because the upstream repository's `text=auto` setting silently stripped one carriage return. `frmHMM.frm` beside it, and every other corpus path, is proved by a whole-corpus test to be never excluded, keeping VER-06's exclusion exactly as narrow as the proven fault, per `03-CONTEXT.md` D-04. A test confirms `.gitattributes` still marks `*.frx` and `*.ctx` as binary, so a later edit that removes the attribute fails the gate rather than corrupting the next resource fixture that lands here.

## Task Commits

Each task was committed atomically, code and its tests together per `AGENTS.md`:

1. **Task 1: The corpus walk and the byte read** - `94c2c79` (test)
2. **Task 2: The Begin and End block parser** - `bc3bd3b` (test)
3. **Task 3: The frmHMM.frx exclusion, named, with its reason** - `9653188` (test)

**Plan metadata:** commit follows this SUMMARY.

_Note: all three tasks carry `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN commit split, matching 03-01's own precedent), each task's RED phase was run once against a deliberately broken implementation, its failure evidence captured below and in the commit message, then reverted to the correct GREEN implementation before committing code and tests together in one commit._

## Files Created/Modified

- `crates/deform6/tests/support/frm.rs` - the second, independent `.frm` reader: `forms()`, `Form`, `Block`, `Property`, `PropertyBlock`, `parse_blocks`, `Exclusion`, `EXCLUSIONS`, `excluded_reason`, and their unit tests
- `crates/deform6/tests/support/mod.rs` - declares `pub mod frm;` in alphabetical order, extends the module doc comment to name the new reader
- `crates/deform6/tests/support_selftest.rs` - the six new self tests for `EXCLUSIONS` and `excluded_reason`, plus the `.gitattributes` binary-attribute test

## Decisions Made

- Followed `AGENTS.md`'s "put the code and its tests in the same commit" rule over the plan's generic TDD RED-then-GREEN commit split, for all three tasks. Each RED phase was captured as real, meaningful test failures (not a compile error and not a placeholder panic) by temporarily reproducing the exact defect this task's own acceptance criteria names, then fixed and committed as one commit per task. See Deviations for the exact failure text.
- `PropertyBlock` carries a `property_blocks: Vec<PropertyBlock>` field for uniform recursive nesting, one level more general than the plan's literal "a property block whose only field is the block name" description. This falls out of reusing a single `Frame` enum (`Control` or `Property`) for the parse stack rather than writing two separate stacks; no corpus file exercises a doubly-nested property block, so this is unproven-by-construction generality, not an added corpus-driven feature.
- Put all of Task 1 and Task 2's tests inside `frm.rs`'s own `#[cfg(test)] mod tests`, per the plan's explicit instruction, rather than in `support_selftest.rs` alongside every other harness test. Task 3's tests went into `support_selftest.rs` instead, per that task's own `<files>` list. Both choices follow the plan's own per-task instructions rather than one uniform convention.

## Deviations from Plan

None - plan executed exactly as written. The RED-evidence capture below documents required TDD gate compliance, not a deviation from the plan's own instructions.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 1** - `walk_by_extension` changed from `eq_ignore_ascii_case` to a plain `==` comparison, and `Form::read` changed from the byte-based Latin-1 read to `std::fs::read_to_string`. `cargo test -p deform6 --test support_selftest support::frm` run once:

```
test support::frm::tests::forms_gives_fifty_four_paths ... FAILED
assertion `left == right` failed: found 53 corpus forms, wanted 54
  left: 53
 right: 54

test support::frm::tests::one_of_the_returned_paths_ends_with_the_upper_case_mci_frm ... FAILED
expected one corpus form to end with the upper case extension MCI.FRM, found: [... 53 paths, none ending in MCI.FRM ...]

test support::frm::tests::form_read_succeeds_on_the_non_utf8_corpus_file_and_keeps_byte_0xa9 ... FAILED
panicked: reading .../Threshold.frm: stream did not contain valid UTF-8
```

Reverted to `eq_ignore_ascii_case` and the byte-based Latin-1 read before committing `94c2c79`.

**Task 2** - `split_property_line` changed to split on every `=` sign (`line.split('=').collect()`) rather than the first. `cargo test -p deform6 --test support_selftest support::frm` run once:

```
test support::frm::tests::a_value_holding_an_equals_sign_is_not_truncated ... FAILED
assertion `left == right` failed: a value holding an equals sign must not be truncated at the first inner =
  left: [Property { name: "Caption", value: "\"A" }]
 right: [Property { name: "Caption", value: "A = B" }]
```

The other nine tests in this task's run already passed against the buggy splitter, because none of their fixtures carry a second `=` inside a value; only this one test is designed to catch the defect. Reverted to `line.split_once('=')` before committing `bc3bd3b`.

**Task 3** - `EXCLUSIONS` left empty. `cargo test -p deform6 --test support_selftest` run once:

```
test exclusions_holds_exactly_one_entry ... FAILED
assertion `left == right` failed: EXCLUSIONS holds 0 entries, wanted exactly 1
  left: 0
 right: 1

test the_one_exclusion_names_frm_hmm_frx_and_its_reason_names_the_line_ending_cause ... FAILED
assertion `left == right` failed
  left: 0
 right: 1

test excluded_reason_gives_some_for_the_named_frx_and_none_for_the_form_beside_it ... FAILED
panicked: index out of bounds: the len is 0 but the index is 0
```

Filled `EXCLUSIONS` with the one `frmHMM.frx` entry and its reason before committing `9653188`. This is the same evidence this task's own acceptance criteria asks for under "emptying `EXCLUSIONS`" - captured as the natural RED phase rather than as a separate post-hoc break, since the entry did not exist yet.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. This plan is test-only; it adds no `src/` code and no placeholder values.

## Next Phase Readiness

- `support::frm` is ready for plan 03-04 (`controltree`) and plan 03-10 (the differential gate) to import and compare against: `forms()`, `Form::read`, `parse_blocks`, `Block`, `Property`, `PropertyBlock`, `EXCLUSIONS`, and `excluded_reason` are all in place and proved against the whole corpus.
- The 48 `Index` and 54 root-form-block counts this plan measured independently agree with `03-CONTEXT.md` D-03's own measurement, so plan 03-04's control array `Index` closure work has a second, independent confirmation to check against.
- No blockers for plan 03-04.

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

All 3 modified/created files found on disk. All 3 task commits (`94c2c79`,
`bc3bd3b`, `9653188`) found in `git log`. All 43 `support_selftest` tests
pass, `cargo test --workspace` passes, `cargo fmt --all --check` and
`cargo clippy --all-targets -- -D warnings` both pass clean, and all three
plan-level shell verifications (54 forms, 48 Index lines, 56-byte
frmHMM.frx) pass.
