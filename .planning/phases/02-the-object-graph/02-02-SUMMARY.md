---
phase: 02-the-object-graph
plan: 02
subsystem: vb
tags: [classify, fobjecttype, objectkind, optionalobjectinfo, obj-02]
status: complete

requires:
  - Object.f_object_type from plan 02-01, the only input classify ever sees
provides:
  - ObjectKind, with Form, Module, Class and Unknown(u32), deriving
    Clone/Copy/PartialEq/Eq/Debug
  - classify, a pure function over a u32, matching the three fObjectType
    values this corpus proves and routing everything else to Unknown
  - has_optional_info, the fObjectType & 0x2 presence test for the optional
    half of ObjectInfo
  - agree, the cross-check against ObjectInfo.lpPrivateObject, returning a
    plain bool rather than a Defect
affects:
  - 02-08, which calls classify on Object.f_object_type to build the kind
    side of the .vbp differential
  - 02-10, which prints the classified kind
  - 02-03, which keys the .bas cap off ObjectKind::Module

tech_stack:
  added: []
  patterns:
    - a match over a u32 read from the file always ends in a binding
      fallback arm, never a wildcard that discards the value, so an
      unrecognised byte pattern is flagged rather than refused
    - a disputed presence test is resolved by picking the one bit the
      full value table supports, and the two rejected tests are kept in
      the doc comment with the reason each is wrong, so nobody
      reintroduces one
    - a cross-check that reports rather than resolves a disagreement
      returns a plain bool from a pure function, so the module that
      decides the check takes on no dependency on the module that turns
      a disagreement into a Defect

key_files:
  created: []
  modified:
    - crates/deform6/src/vb/classify.rs

decisions:
  - The match holds exactly the three fObjectType values this corpus
    measures (0x18001 Module, 0x18083 Form, 0x118003 Class) and not the
    full seventeen-value table STRUCTURES.md section 5.5 cites. The other
    fourteen are recorded in the doc comment as cited and unmeasured. A
    value taken from a document and pre-populated into the match silently
    mis-tags a real object if the document is wrong; left out, the same
    value lands in Unknown and is flagged.
  - agree returns bool, not a DefectKind. The plan offered a choice between
    reusing DefectKind::CountMismatch (a count-vs-count message, a poor fit
    for two boolean markers) and returning a bare value the caller reports.
    This plan takes the second option, so classify.rs stays a pure module
    over u32 with no error.rs dependency and no file access, matching the
    plan's own truth: "the classifier is a pure function over a u32."
  - has_optional_info uses fObjectType & 0x2. Both published tests
    (SVBD's 0x80, PVB's 0x01) are named in the doc comment together with
    the corpus fact that breaks each one, so a later reader who is tempted
    to reintroduce either one sees why it was rejected the first time.

metrics:
  duration: 1 session
  completed: 2026-09-08

actuals:
  tokens: 4089
  tasks: 2
  commits: 2
plan_head_before: bf99fae0f41d84fde7c0f22aa4d926045479e618
---

# Phase 02 Plan 02: Telling a form from a module from a class Summary

`vb/classify.rs` turns `Object.f_object_type` into an `ObjectKind` (`Form`,
`Module`, `Class` or `Unknown(u32)`) with a three-arm match measured against
the whole 44-program, 105-object corpus, and separately resolves the disputed
`OptionalObjectInfo` presence test as `fObjectType & 0x2`, cross-checked
against `ObjectInfo.lpPrivateObject` and reported, never resolved, when the
two disagree.

## What this plan built

| Item | What it gives |
|---|---|
| `ObjectKind` | `Form`, `Module`, `Class`, `Unknown(u32)`, deriving `Clone, Copy, PartialEq, Eq, Debug` |
| `classify(f_object_type: u32) -> ObjectKind` | a `const fn`, three measured arms plus a binding fallback, never a refusal |
| `has_optional_info(f_object_type: u32) -> bool` | `fObjectType & 0x2 != 0`, the one bit that separates a module from everything else across all seventeen tabulated values |
| `agree(f_object_type: u32, lp_private_object: u32) -> bool` | cross-checks the bit against the `-1` sentinel, `true` when they agree |

Both functions are pure: no file read, no address resolution, no allocation.
`crates/deform6/src/vb/classify.rs` is 359 lines with 9 tests covering all
eleven behaviours the plan names.

## Only three of the seventeen tabulated `fObjectType` values occur in this corpus

`STRUCTURES.md` section 5.5 tabulates seventeen values from one prior tool's
lookup table. This corpus proves exactly three of them (`0x18001` Module,
8 objects; `0x18083` Form, 53 objects; `0x118003` Class, 44 objects; 105
total, every one matching what its `.vbp` declares). The other fourteen are
cited in the doc comment, unmeasured, and not in the match; Phase 6 reads
this line for the release notes' limit.

## Task commits

| Commit | Subject |
|---|---|
| `69bb0cf` | Classify an object as a form, a module, a class or unknown |
| `3e699cd` | Report a disagreement between the two optional-info markers |

`git rev-list --count bf99fae..HEAD` is 2, one commit per task.

## The gate

Run after each task and once more on the final committed tree, clean working
tree throughout.

| Command | Exit |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0, 154 library tests |
| `sh scripts/prove-lint-wall.sh` | 0 |
| `sh scripts/prove-region-wall.sh` | 0 |

The library suite was 145 tests before this plan (`145 filtered out` shown by
`cargo test -p deform6 --lib vb::classify`), 9 tests were added in
`vb::classify`, and `cargo test --workspace` shows 154 library tests passing.
The workspace total across all five test binaries is 154 + 1 + 9 + 21 + 9 =
194.

## Twelve behaviours, or eleven, depending on how one is counted

The plan names six behaviours for task 1 and five for task 2, eleven total,
covered by nine tests (some tests carry more than one literal assertion,
per the plan's own grouping, e.g. the three named values in one test and the
two edge values in another).

`cargo test -p deform6 --lib -- --list` shows
`vb::classify::tests::no_corpus_file_makes_the_two_optional_info_markers_disagree`,
which is the exact test name the plan's own `<verification>` section asks
for.

## The deliberate breakages

Three, all recorded, all reverted before the following commit and confirmed
byte-identical to the pre-breakage file with `diff`.

### 1. `Form` and `Class` swapped in the match (task 1)

**Three tests failed, not the one the plan names:**

```
---- vb::classify::tests::every_tabulated_value_classifies_and_the_untested_ones_land_in_unknown ----
assertion `left == right` failed: 0x18083 must classify to Form
  left: Class
 right: Form

---- vb::classify::tests::the_three_measured_values_classify_by_name ----
assertion `left == right` failed
  left: Class
 right: Form

---- vb::classify::tests::walking_grayscale_classifies_form_class_class_in_array_order ----
assertion `left == right` failed
  left: [Class, Form, Form]
 right: [Form, Class, Class]
```

The plan named only the Grayscale ordered test; the two literal-value tests
also caught it, because they assert the exact same three values by name.
Restored, all 9 tests pass again.

### 2. The fallback arm replaced with `Refusal::Damaged(...)` (task 1)

**Compile failure, not a runtime test failure:**

```
error[E0433]: cannot find type `Refusal` in this scope
  --> crates/deform6/src/vb/classify.rs:93:19
   |
93 |         _other => Refusal::Damaged("an unrecognised fObjectType"),
   |                   ^^^^^^^ use of undeclared type `Refusal`
```

This is the stronger result the plan asks to record: `classify`'s own
signature returns `ObjectKind`, so a fallback arm that tries to produce a
`Refusal` fails to compile rather than merely failing a test at run time.
The signature itself forbids the refusal; nothing downstream can construct
one even by mistake. Restored, byte-identical to the pre-breakage file.

### 3. The presence test moved from `& 0x2` to `& 0x80` (task 2)

**Two tests failed, not the one the plan names:**

```
---- vb::classify::tests::the_form_and_the_class_each_carry_the_optional_block_and_the_module_does_not ----
panicked at crates/deform6/src/vb/classify.rs:291:9:
a class has one

---- vb::classify::tests::no_corpus_file_makes_the_two_optional_info_markers_disagree ----
panicked at crates/deform6/src/vb/classify.rs:321:13:
"pdOpenSaveDialog" disagrees, and this corpus is measured to have none that do
```

The plan named the class case of the first behaviour; the corpus
cross-check test also caught it, because `pdOpenSaveDialog` (`fObjectType`
`0x118003`) has no `0x80` bit, so `has_optional_info` under the broken test
disagrees with its own real `lpPrivateObject`, which this corpus is measured
to never do. Restored, all 9 tests pass again.

## Defects found in the plan

None that required a deviation from the plan's own text. Two points worth
recording because a future reader might reach for them:

1. **The plan's task 2 offered a real choice** between reusing
   `DefectKind::CountMismatch` and returning a bare value. `CountMismatch`'s
   fields (`count`, `expected`, `other_field`) are shaped for two numeric
   counts, not two boolean markers, and forcing a `bool` into `count: u32`
   would read as nonsense in the rendered message (`"count 1 ... does not
   match the 0 that lpPrivateObject gives"`). The bare-`bool` option was
   taken, and the doc comment on `agree` says so and gives the reason.
2. **The plan's task 2 asks for a fifth test naming the doc comment's two
   rejected presence tests.** There is no established pattern elsewhere in
   this crate for a test that reads its own source text, so
   `the_doc_comment_names_both_rejected_presence_tests_and_the_reason_each_is_wrong`
   uses `include_str!("classify.rs")` (a file including its own text, which
   is legal: `include_str!` reads bytes, it does not recurse the module
   tree) and asserts the doc comment still names `0x80`, `0x01`, and the
   reason each fails. This is a documentation-content test, not a
   behavioural one, and it exists only because the plan asked for exactly
   this assertion.

Neither is a deviation from the plan; both are choices the plan explicitly
left open, resolved and recorded per its own instruction to "choose one and
say which in the doc comment."

## Threat mitigations

| Threat | State |
|---|---|
| T-02-06, spoofing a kind name from a crafted `fObjectType` | Mitigated. The match holds only the three measured values; the fallback carries the raw number, never a guessed name. |
| T-02-07, denial of service in `classify` | Mitigated. Total over `u32` by construction: three arms and a binding fallback, no indexing, no arithmetic, no allocation. The deliberate compile-failure breakage above is direct evidence the signature forbids any other outcome. |
| T-02-08, the untested fourteen cited values | Accepted, as the plan directs. Recorded in the doc comment as cited and unmeasured; each reaches `Unknown` rather than a name. |
| T-02-09, the untested marker disagreement | Accepted, as the plan directs. Exercised only by the synthetic pair `agree(0x0001_8083, 0xFFFF_FFFF)`; the doc comment and this summary both say no corpus file reaches it. |

## Known Stubs

None. Both functions are fully implemented against the values this corpus
measures, and the fourteen untested `fObjectType` values plus the untested
marker-disagreement branch are accepted risks per the threat model, not
stubs standing in for missing work.

## Deferred Issues

None found in scope.

## Threat Flags

None. This plan adds no network endpoint, no auth path, and no file access
of its own; the two file-reading tests it does carry reuse the same
`include_bytes!` corpus fixture and public read-layer primitives every other
module in this phase already uses.

## Self-Check: PASSED

`crates/deform6/src/vb/classify.rs` exists and holds both `ObjectKind`,
`classify`, `has_optional_info` and `agree`. Both task commit hashes
(`69bb0cf`, `3e699cd`) resolve in this branch's history. The gate table
above was run on the committed tree before this file was written, and
`cargo test -p deform6 --lib -- --list` was run against the committed tree
to confirm the exact test name the plan's `<verification>` section names.
