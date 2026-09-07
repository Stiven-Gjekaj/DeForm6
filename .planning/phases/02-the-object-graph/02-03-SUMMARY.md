---
phase: 02-the-object-graph
plan: 03
subsystem: vb
tags: [privateobj, objectinfo, procnames, obj-03, obj-06, d-13, d-14]
status: complete

requires:
  - Object.lp_proc_names_array, Object.proc_count and Object.lp_object_info
    from phase 2 plan 01
  - DefectKind::UnreadablePointer and Severity::Recoverable, added by plan
    02-01 for exactly this shape of leaf pointer
provides:
  - ObjectInfo::read and PrivateObj::read, each narrowed to its documented
    window before any field is read, with PrivateObj::Absent modelling the
    module sentinel (-1 and a plain 0) as a case rather than a refusal
  - Procedure (Public(String) / Private), ProcNames (Slots(Vec<Procedure>) /
    NoNameArray { proc_count }), ProcedureList::read and ProcedureCounts::of
  - the validated procedure-name walk: a non-null lpProcNamesArray entry
    earns Public only when it resolves inside a section, is NUL terminated
    within 64 bytes, starts with a letter or underscore, and holds only
    alphanumerics and underscores
affects:
  - 02-04, whose FuncTypDesc walk is index-parallel to lpProcNamesArray and
    of the same length, reached from PrivateObj.lp_func_type_info
  - 02-05, which owns the explanation of cnt_public_vars, carried here
    unexplained per D-14, and reads PrivateObj.lp_public_vars
  - 02-08, whose differential test needs the corrected Mandelbrot numbers
    (0 of 9, not 9 of 9) rather than the plan's inherited worked example
  - 02-09, which pins the recovery ratio ProcedureCounts::of feeds, and
    which must state the .bas cap (8 of 8, D-10) before the first ratio

tech_stack:
  added: []
  patterns:
    - "the module sentinel (-1, and a plain 0) is modelled as a two-state
      enum (PrivateObj::Present / Absent) rather than a Result, because a
      standard module carrying no PrivateObj is a documented case and not
      damage: refusing here would lose every module in the corpus"
    - "the array-pointer null check happens strictly before any per-entry
      resolution, so a null lpProcNamesArray never reaches
      PeImage::region_at_va and the object's real ProcCount (1 to 7 for a
      .bas) is carried through unclamped rather than erased to zero"
    - "a non-null array entry is validated, not merely resolved: an address
      that resolves to real bytes is still rejected as Private plus a
      defect unless the bytes pass a four-part identifier test, because a
      resolving address is not sufficient evidence of a name"
    - "the per-object result carries a raw value in its defect rather than
      inventing a name or reporting silently, reusing
      DefectKind::UnreadablePointer for every one of the three ways a
      non-null entry can fail validation (unmapped address, no NUL within
      64 bytes, resolves but fails the identifier test), because this
      plan's file boundary does not include error.rs and no fourth kind
      exists to add without touching it"

key_files:
  created: []
  modified:
    - crates/deform6/src/vb/privateobj.rs

decisions:
  - "The plan's own worked example was wrong, and CONTEXT.md's correction
    (added after the plan checker flagged it) is what this plan actually
    implements: Mandelbrot.exe's frmFractal does not carry nine null
    lpProcNamesArray entries. Verified directly against the corpus bytes
    before writing any code: the array sits at a real, non-null address,
    and its nine dwords are non-null values that read as UTF-16 text
    spelling a fragment of a build-machine path
    (mData\\Oracle\\Java\\...), never overwritten by the compiler. Every
    one of the nine entries is a non-null address that resolves to no
    section, which is what a validated implementation reports: nine
    Private slots, each with its own recoverable defect naming the raw
    address, not nine null entries with no defect at all. The test
    the_three_vendored_programs_recover_the_measured_number_of_names and
    mandelbrot_frm_fractal_gives_nine_slots_all_private_and_uninitialised
    assert the measured 0 of 9 and the nine defects, not the plan's
    inherited 9 of 9."
  - "A non-null entry is validated on four independent facts before it
    earns Public, matching the exact rule CONTEXT.md's own measurement
    script used: resolves inside a section, NUL terminated within 64
    bytes, starts with a letter or underscore, and every byte is an ASCII
    alphanumeric or underscore. This rule was proved against the real
    corpus files with a standalone script before any Rust was written
    (see 'Verification performed' below), and it reproduces the plan's own
    required numbers exactly: Grayscale.exe 12 of 34, Mandelbrot.exe 0 of
    9, Map Editor.exe 0 of 36 with 8 capped. No looser or tighter rule was
    tried, because this one already closes exactly."
  - "Every failure mode for a non-null entry (address resolves nowhere, no
    NUL within 64 bytes, or a NUL-terminated run that fails the identifier
    test) reuses DefectKind::UnreadablePointer. The plan's own action text
    names DefectKind::UnmappedAddress for this, but error.rs fixes that
    variant's severity at Fatal for a spine pointer, and a leaf pointer
    inside an already-reached object must stay Recoverable per the same
    precedent plan 02-01 already established for object.rs's own name
    pointer. This plan's file boundary excludes error.rs, so no fourth
    kind could be added for the third failure mode (resolves, NUL found,
    fails the identifier test); reusing UnreadablePointer still carries the
    raw address CONTEXT.md asks a gap to carry, which is the property that
    matters, even though its message text ('resolves to nothing') is not
    literally accurate for that one sub-case. Documented in the function's
    own doc comment rather than silently smoothed over."
  - "cnt_public_vars is carried on PrivateObj::Present with a doc comment
    that states the measured, name-contradicting value (4, for a class
    that declares zero Public variables) per D-14, and nothing in this
    file is built on the assumption that the field means what its name
    says. Plan 02-05 owns the explanation."

metrics:
  duration: 1 session
  completed: 2026-09-08

actuals:
  tokens: 10631
  tasks: 3
  commits: 3
plan_head_before: bf99fae0f41d84fde7c0f22aa4d926045479e618
---

# Phase 02 Plan 03: The procedure name walk Summary

`vb/privateobj.rs` reads `ObjectInfo` and `PrivateObj`, models the
standard-module sentinel as a case rather than an error, and walks
`Object.lpProcNamesArray` with every non-null entry validated before it is
trusted, correcting the plan's own inherited claim that `Mandelbrot.exe`
holds nine null entries: measured, it holds nine non-null, unmapped ones.

## What this plan built

| Item | What it gives |
|---|---|
| `ObjectInfo::read` | the object index and the raw `lp_private_object` address, narrowed to its documented `0x38`-byte window |
| `PrivateObj::Present` / `PrivateObj::Absent` | the real structure, or the module sentinel (`-1` and a plain `0`, both) as a first-class case |
| `Procedure::Public(String)` / `Procedure::Private` | one procedure slot; `Private` carries nothing, per OBJ-06 |
| `ProcNames::Slots(Vec<Procedure>)` / `ProcNames::NoNameArray { proc_count }` | the two facts D-13 keeps apart: an empty-looking list is not the same value as no array at all |
| `ProcedureList::read` | the walk itself: the array pointer is checked before the loop, and every non-null entry is validated on four facts before it earns `Public` |
| `ProcedureCounts::of` | the three numbers (declared, recovered, no-name-array) plan 02-09 pins and plan 02-10 prints |

`crates/deform6/src/vb/privateobj.rs` grew from a five-line stub to 951
lines with 15 tests, in three commits, none touching any file outside
itself.

## Verification performed before writing any Rust

Before implementing the validation rule, I wrote a standalone Python script
against the real corpus bytes of `Mandelbrot.exe`, `Grayscale.exe` and
`Map Editor.exe`, applying exactly the four-part identifier test the brief
described, and it reproduced the plan's own required numbers exactly:
`Grayscale.exe` 12 of 34 (`frmGrayscale` 8/20, `pdOpenSaveDialog` 0/6,
`FastDrawing` 4/8), `Mandelbrot.exe` 0 of 9, `Map Editor.exe` 0 of 36 with 8
capped. This is why the Rust implementation passed all 15 tests on its
first compile: the algorithm was proven against real bytes first.

## Defects found in the plan

Two, both found before writing the corresponding code, both already
anticipated in different words by the brief that corrected this plan.

### 1. `Mandelbrot.exe`'s worked example: 9 of 9 assumed, 0 of 9 measured, and not because of null entries

Documented above under "decisions". The plan's task 2 action text says
"a script this planner ran gives 0 named of 9 slots, because every
procedure in `Mandelbrot.frm` is declared `Private`" in one place, and
separately, its `<behavior>` list still says "frmFractal gives nine slots
and every one of the nine is `Private`" without repeating the corrected
reason. The must_haves truth ("Every public procedure name an object
carries is recovered, 193 across the 44 corpus programs") is unaffected;
only the object-level Mandelbrot claim needed the correction, which the
brief supplied and this plan's tests assert directly
(`mandelbrot_frm_fractal_gives_nine_slots_all_private_and_uninitialised`
asserts 9 defects, proving these are unmapped addresses and not null
entries).

### 2. `pdOpenSaveDialog.cls` already states six slots correctly in the plan text itself

The brief's corrective note claimed the plan said "two" non-public
procedures for `pdOpenSaveDialog`. Reading the plan text directly: task 2's
second behaviour already says "six slots and every one is `Private`,
because the two procedures its source declares are `Friend`". The plan
text is already correct on this point; the "two" the brief describes must
refer to some other draft or to 02-07's summary text (which itself records
finding and correcting the same "two" claim independently, in its own
task). No fix was needed here; recorded so the discrepancy between the
brief and the actual plan text on disk is not silently dropped.

## The 428 unresolvable entries: what this plan found out

The brief asked this plan to own the open question of what the 428
corpus-wide unresolvable `lpProcNamesArray` entries are. Measured directly
against the three vendored programs this module reads (not the full
44-program sweep, which is plan 02-08's differential harness):

- `Mandelbrot.exe`'s nine entries are all **non-null addresses that resolve
  to no section** (`region_at_va` returns `None` for all nine). The array
  sits at a real file location and its bytes are non-zero, UTF-16-shaped
  text from a build machine, but interpreted as 32-bit virtual addresses
  they are all implausibly large or otherwise outside every mapped
  section.
- `Grayscale.exe` and `Map Editor.exe` produced **zero** unresolvable
  non-null entries across all of their objects: every non-null entry in
  those two programs either passed the four-part identifier test (and
  became `Public`) or the array pointer itself was null (the `.bas` case).

So, for the three programs this plan reads, the answer to "are all 428
uninitialised, or does some subset carry a recognised encoding" is: **every
non-null entry this plan's corpus subset produced that failed validation,
failed at the address-resolution step** (`region_at_va` returning `None`),
never at the NUL-termination step or the identifier-character step. This
plan's code path for those latter two failure modes exists and is
exercised by the algorithm's logic, but no corpus file among the three
this module reads exercises it, matching the pattern `RESEARCH.md` records
elsewhere in this phase (`cntEvents`, `ParamArray`): a real and necessary
code path with no corpus sample to prove it against a real file. Whether
the wider 428-entry corpus figure (all 44 programs, 105 objects) contains
any entry that resolves but fails NUL-termination or the identifier test
is not established by this plan; that question belongs to plan 02-08's
full sweep, which can observe it directly.

## The deliberate breakages

Four, all recorded, all reverted before the following commit.

### 1. `lp_private_object` read from `0x08` instead of `0x0C` (task 1)

**Three tests failed**, not the one the plan predicted:

```
---- grayscale_object_info_resolves_for_all_three_objects_with_a_real_private_object ----
assertion `left != right` failed
  left: 0
 right: 0

---- grayscale_fast_drawing_private_obj_gives_non_null_func_type_info_and_carries_counts ----
panicked at: FastDrawing is a class, not a module

---- map_editor_module_objects_give_private_obj_absent_and_object_info_still_reads ----
assertion `left == right` failed
  left: 0
 right: 4294967295
```

Reading `lpIdeData` (`0x08`) instead of `lpPrivateObject` (`0x0C`) happens
to read `0` for every object in these three programs (an in-memory field
zeroed after compilation), which broke the "real address, not `0`" check
for Grayscale, broke the module-sentinel check for Map Editor (`0` is not
`0xFFFF_FFFF`), and broke the `PrivateObj::Present` match for FastDrawing
(a plain `0` now reads as `PrivateObj::Absent`). Restored, all 7 task-1
tests pass again.

### 2. The array-pointer null check removed entirely (task 2, first breakage)

**Zero tests failed.** This is the finding CONTEXT.md and `AGENTS.md` both
warn about directly: "a deliberate breakage that produces no failure means
the covering test does not exist." Investigated rather than silently
accepted. The reason: `Va::to_rva`'s `checked_sub(image_base)` already
returns `None` for address `0` against any real image base (`0x400000` in
this corpus), so `PeImage::region_at_va(Va::new(0))` unconditionally
returns `None` regardless of whether the explicit `is_null()` check exists.
Removing the check does not remove the safety; it removes only the
explicit, first-class modelling of the D-13 distinction. I looked for a
test that could discriminate the two implementations (one would need a
synthetic image whose `image_base` is `0`, so that RVA `0` legitimately
resolves inside a mapped section) and did not build one: doing so
correctly requires a hand-built PE image (DOS header, COFF header, PE32
optional header, section table) of the kind `vb/object.rs`'s own test
module builds for an unrelated purpose, and constructing one is outside
what task 2's four named behaviours ask for. The existing system-wide
safety net is already proved directly by `read::pe::tests::a_virtual_address_below_the_image_base_resolves_to_nothing`,
which this plan does not duplicate. The check is kept in the code as
documentation of a deliberate, first-class decision (D-13) rather than as
the only thing standing between a null pointer and a bad read; the doc
comment on `ProcedureList::read` now says so. Restored (a two-line
re-insertion), all 13 task-2 tests re-verified passing.

### 3. A null entry given `Procedure::Public(String::new())` instead of `Private` (task 2, second breakage)

**Two tests failed**, and neither is "the third behaviour" the plan
predicted:

```
---- grayscale_fast_drawing_gives_eight_slots_four_public_four_private ----
  left: Slots([Public(""), Public(""), Public(""), Public(""), Public("GetImageWidth"), ...])
 right: Slots([Private, Private, Private, Private, Public("GetImageWidth"), ...])

---- grayscale_pd_open_save_dialog_gives_six_slots_all_private ----
  left: Slots([Public(""), Public(""), Public(""), Public(""), Public(""), Public("")])
 right: Slots([Private, Private, Private, Private, Private, Private])
```

The plan's own instruction ("confirm the third behaviour fails") was
written against the plan's original, uncorrected premise that
`frmFractal`'s nine entries are null. They are not (see "Defects found in
the plan" above): every one of Mandelbrot's nine entries is non-null and
unmapped, so a change to the *null-entry* branch of `resolve_entry` cannot
touch Mandelbrot's result at all. The two tests that actually exercise
literal null entries (`FastDrawing`, `pdOpenSaveDialog`) failed instead,
exactly as this correction predicts. Restored, all 15 tests (task 1
through task 3) re-verified passing.

### 4. The recovered count includes every slot, private or public (task 3)

**Two tests failed**, one of them the exact one the plan named plus one
extra test this plan added beyond the plan's four:

```
---- the_three_vendored_programs_recover_the_measured_number_of_names ----
  left: (34, 34, 0)
 right: (34, 12, 0)

---- procedure_counts_of_gives_the_three_numbers_for_one_object ----
  left: ProcedureCounts { declared: 3, recovered: 3, no_name_array: false }
 right: ProcedureCounts { declared: 3, recovered: 2, no_name_array: false }
```

Restored, all 15 tests pass again.

## The gate

Run on the committed tree at `5cb6e44`, with a clean working tree.

| Command | Exit |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0, 199 total tests across the workspace (160 library, plus 1 corpus sweep, 9 refusal, 21 support-selftest, 9 CLI); 15 of the 160 are new to `vb::privateobj` |
| `sh scripts/prove-lint-wall.sh` | 0 |
| `sh scripts/prove-region-wall.sh` | 0 |

```
$ cargo test -p deform6 --lib vb::privateobj -- --list | wc -l
15 tests + trailing summary line
$ git diff --stat bf99fae..HEAD -- crates/deform6/src/vb/privateobj.rs
 crates/deform6/src/vb/privateobj.rs | 954 +++++++++++++++++++++++++++++++++++-
$ git diff --stat bf99fae..HEAD
 crates/deform6/src/vb/privateobj.rs | 954 +++++++++++++++++++++++++++++++++++-
 1 file changed, 949 insertions(+), 5 deletions(-)
```

Only `crates/deform6/src/vb/privateobj.rs` was touched across all three
commits. `classify.rs`, `object.rs`, `project.rs`, `vb/mod.rs`, `error.rs`
and everything under `tests/` were not opened for writing.

## Threat mitigations

| Threat | State |
|---|---|
| T-02-10, `lpProcNamesArray` null with `ProcCount` non-zero | Mitigated. The array pointer is checked, and the absent state is returned, strictly before the loop. `a_synthetic_object_with_a_null_array_pointer_and_a_nonzero_count_gives_the_absent_state` proves a synthetic object with a null pointer and `proc_count: 7` never enters the loop. |
| T-02-11, `proc_count * 4` as a window size | Mitigated. `checked_mul`, then `Region::subregion`, bounded by the real mapped length of the region at `lpProcNamesArray`. |
| T-02-12, a name entry resolving into unrelated bytes | Mitigated, and strengthened beyond the plan's original text: an entry that resolves is still validated on three further facts (NUL termination within 64 bytes, leading letter/underscore, all-alphanumeric-or-underscore) before it earns `Public`. `mandelbrot_frm_fractal_gives_nine_slots_all_private_and_uninitialised` is the corpus instrument: nine non-null entries, all rejected, none presented as a name. |
| T-02-13, the `ObjectInfo` and `PrivateObj` windows | Mitigated. Each is narrowed to its documented size (`0x38`, `0x40`) before any field is read. `an_object_info_pointer_in_no_section_is_damaged` and `a_file_truncated_inside_object_info_is_damaged_and_is_not_read_in_part` are the instruments. |
| T-02-14, `lpPrivateObject` read as an address when it is `-1` | Mitigated. Read as a raw `u32` on `ObjectInfo`, and the sentinel (`-1` and a plain `0`) is matched inside `PrivateObj::read` before anything resolves it. `a_plain_zero_private_object_address_is_also_absent` proves the `0` case does not need a `PeImage` at all. |

## Known Stubs

None. Every branch `ProcedureList::read` and `resolve_entry` can take is
implemented; some (a non-null entry resolving but failing NUL-termination
or the identifier test) are not exercised by any of the three vendored
programs this module reads, which is recorded above under "The 428
unresolvable entries" rather than hidden.

## Deferred Issues

The full 44-program, 105-object sweep, and whether it contains any entry
that resolves but fails validation on grounds other than address
resolution, belongs to plan 02-08's `tests/differential.rs`, which this
plan does not touch.

## Threat Flags

None. This plan reads existing in-image structures through the same
`Region`/`Va`/`PeImage` primitives every other module in this crate uses.
No new network endpoint, no new auth path, no new file-system access path.

## Commits

| Commit | Subject |
|---|---|
| `f583478` | Read ObjectInfo and PrivateObj and model the module sentinel |
| `3e5c370` | Report a private procedure as private and a missing array as missing |
| `5cb6e44` | Count the recovered names and the capped slots per object |

`git rev-list --count bf99fae..HEAD` is 3, one commit per task, each
carrying its own tests.

## Self-Check: PASSED

`crates/deform6/src/vb/privateobj.rs` exists and holds all three types
this plan built. All three task commit hashes (`f583478`, `3e5c370`,
`5cb6e44`) resolve in this branch's history. The gate table above was run
on the committed tree before this file was written.
