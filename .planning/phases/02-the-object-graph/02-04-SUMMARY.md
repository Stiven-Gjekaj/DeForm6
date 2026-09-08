---
phase: 02-the-object-graph
plan: 04
subsystem: vb
tags: [functypdesc, prototype, obj-04, type-buffer, optionalvals, d-07]
status: complete

requires:
  - PrivateObj.lp_func_type_info and Object.proc_count from plan 02-03,
    index-parallel to Object.lp_proc_names_array, per STRUCTURES.md
    section 6.2
  - DefectKind::UnreadablePointer and DefectKind::CountMismatch from
    phase 2 plan 01, reused rather than added, since error.rs is outside
    this plan's file boundary
provides:
  - FuncTypDesc header read (RawHeader, read_raw_header), settling gap 6
    on real bytes: constFFFF == 0xFFFF and the word at 0x06 == 0 in
    193 of 193 measured records, which chose the aligned pubfunc_v2
    layout over the shifted func2Type one
  - the type buffer walk (walk_type_buffer), closing at exactly
    argSize >> 2 entries within a bounded step count, tolerating padding,
    and reading a trailing pointer for an internal class, an external
    COM interface or an external COM object
  - VbType (fifteen documented codes plus Unknown(u8)), the
    Optional/Array/ByRef modifier decode, and the corrected
    property-kind mask (0x03, not STRUCTURES.md section 6.6's stated
    0x07)
  - the optionalVals default-value grammar, closing gap 7: a u32
    cbValues, a VA at +4 confirmed equal to optionalVals + 8, then
    cbValues bytes of value records (six OLE variant tags), closing at
    exactly cbValues in 61 of 61 records over 89 value records
  - FuncTypeWalk::read, ProcedureSignature, PrototypeList: the
    top-level walk of PrivateObj.lpFuncTypeInfo, bounded by
    Object.proc_count, giving one Prototype, Unrecoverable or
    NoDescriptor per index
  - tests/type_descriptors.rs: the four corpus-wide aggregates over all
    44 vendored programs
affects:
  - a later phase's report/CLI layer, which prints the recovered
    Prototype values this plan produces and, for VbType::Internal, still
    needs to resolve the carried Va to a class name through
    ObjectInfo.lpObject -> Object.lpszObjectName, which this plan does
    not perform
  - plan 02-05, which owns EventDesc (lpEventsTypeInfo) and PubVarDesc
    (lpPublicVars); this plan touches neither

tech_stack:
  added: []
  patterns:
    - "a per-record failure (constFFFF mismatch, a type buffer that does
      not close, an unresolvable argument name, an optionalVals walk
      that does not close) is a Vec<Defect> returned alongside an
      Option<Prototype>, not a Result: more than one independent
      failure can occur on the same record, unlike the single-failure
      leaf pointers phase 2 has read so far"
    - "the type buffer walk and the optionalVals value-record walk are
      each bounded two independent ways: by the real mapped bytes a
      Region::subregion enforces, and by an explicit step count, so a
      hostile file cannot make either walk scan for a padding byte or a
      record boundary that never arrives, even inside a legitimately
      large mapped section"
    - "a reused DefectKind (UnreadablePointer for a resolved-but-wrong
      signature word or an unrecognised tag's location, CountMismatch
      for a walk that does not close or a tag number outside the known
      six) carries the same raw value CONTEXT.md asks a gap to carry,
      even when the kind's own message text is not literally accurate
      for the sub-case reusing it, matching the precedent
      vb/privateobj.rs set for its own third failure mode"

key_files:
  created:
    - crates/deform6/tests/type_descriptors.rs
  modified:
    - crates/deform6/src/vb/functyp.rs

decisions:
  - "GetImageWidth's arg_size is measured 0x08, not the plan's stated 4:
    it is a Function (Public Function GetImageWidth(ByRef srcPictureBox
    As PictureBox) As Long), so its type buffer holds two entries (the
    argument and the return type), and two entries is 2 << 2 = 0x08.
    The plan's task 1 test asserts the measured value; see 'Defects
    found in the plan' below."
  - "The optionalVals grammar is not one value record per Optional
    argument, as the plan's action text states. Measured over the whole
    corpus: a non-String Optional argument with no explicit = literal
    default gets no value record at all; only a String-typed Optional
    with no default gets an explicit Empty (tag 0) record. Four records
    in this corpus have fewer value records than Optional arguments
    (cCommonDialog::VBGetSaveFileName and VBChooseColor, each with a
    trailing Optional flags As Long with no default;
    cSystemColorDialog::ShowColorDialog, whose trailing
    showFullDialog/initColor both lack a default), and in every one the
    shortfall is exactly the trailing argument(s) in declaration order,
    consistent with VB6's rule that Optional parameters are contiguous
    and trailing. Value records are assigned to the first N Optional
    arguments front to back; any remaining trailing ones carry no
    default. This did not surface in the four hand-picked worked
    examples the plan names (all of which have an exact 1:1 count), only
    in the full 44-program sweep tests/type_descriptors.rs performs,
    which is exactly why that sweep exists."
  - "VbType::Internal, VbType::ComIFace and VbType::ComObj carry their
    trailing 32-bit value as a raw, unresolved Va, not a name. The
    plan's key_links note that vb/object.rs and vb/privateobj.rs can
    join an epvT_internal pointer to a class name through
    ObjectInfo.lpObject, but neither module's own read function exposes
    that field, and building the join here would mean either touching
    privateobj.rs or object.rs (outside this plan's declared file
    boundary) or duplicating a second, independent read of
    ObjectInfo + 0x18 for a fact a later report layer needs once, not
    a fact this plan's own must_haves require. Confirmed the join is
    reachable when needed: Artificial Life.exe's Organism.
    CreateFromCreature(ByRef srcCreature As Organism) resolves through
    exactly this chain to the string 'Organism', by hand, outside any
    committed test."
  - "read_one takes an already-resolved Region rather than re-resolving
    the FuncTypDesc address itself, so FuncTypeWalk::read is the one
    place that turns an unmapped lpFuncTypeInfo entry into a defect,
    with a Site naming structure: 'PrivateObj', field: 'lpFuncTypeInfo'
    (the array that held the bad pointer), while read_one's own
    internal failures (a header that does not fit, a constFFFF
    mismatch, a buffer that does not close) use structure: 'FuncTypDesc'
    instead. The two Sites therefore name different things for
    different failure classes on purpose, matching the level each
    defect actually happened at."

metrics:
  duration: 1 session
  completed: 2026-09-08

actuals:
  tokens: 23319
  tasks: 3
  commits: 3
plan_head_before: 0212f4ecb5b3c72a8f753a98d3a6255608f16c8c
---

# Phase 02 Plan 04: Procedure prototypes Summary

`vb/functyp.rs` reads `FuncTypDesc`, walks the type buffer and the
`optionalVals` default-value grammar, and gives every public procedure a
`Prototype` with its argument names, types, `ByRef`/`Array`/`Optional`
modifiers, and recovered default literals, closing `STRUCTURES.md` gaps 6
and 7 on real bytes and correcting section 6.6's property-kind mask.

## What this plan built

| Item | What it gives |
|---|---|
| `read_raw_header` | the fixed `0x20`-byte header, validating `constFFFF == 0xFFFF` before anything downstream trusts the record |
| `walk_type_buffer` | the variable-length type buffer, closing at exactly `argSize >> 2` entries within a bounded step count, tolerant of padding, reading a trailing pointer for `0x13`/`0x1C`/`0x1D` |
| `VbType`, `TypeEntry`, `PropertyKind` | fifteen documented type codes plus `Unknown(u8)`; the three modifier bits; the corrected `0x03` property-kind mask |
| `resolve_arg_names` | every argument name from `lpAryArgNames`, bounded by the argument count the buffer walk already closed on |
| `walk_optional_vals`, `DefaultValue`, `OptionalDefaultsOutcome` | the `optionalVals` grammar: `cbValues`, then value records, six OLE variant tags, closing at exactly `cbValues` |
| `FuncTypeWalk::read`, `ProcedureSignature`, `PrototypeList` | the top-level walk of `PrivateObj.lpFuncTypeInfo`, bounded by `Object.proc_count`, index-parallel with `Object.lpProcNamesArray` |
| `tests/type_descriptors.rs` | the four corpus-wide aggregates over all 44 vendored programs |

`crates/deform6/src/vb/functyp.rs` grew from a three-line stub to 1,818
new lines with 20 unit tests, in three commits. `crates/deform6/tests/
type_descriptors.rs` is new, 386 lines, 2 tests.

## What OBJ-04 delivers, and what it does not

**Delivered:** argument names, argument types (nine of the fifteen
documented codes occur in this corpus; the other six and all fifteen
unassigned codes carry `VbType::Unknown` if ever seen), `ByRef`, `Array`
and `Optional` modifiers, and the literal default value of an `Optional`
argument when one exists in the file.

**Not delivered, and named as a gap rather than guessed:**

- **`ParamArray`** has no observed encoding anywhere in this 44-program
  corpus (a grep of every source file finds zero occurrences of the
  keyword). No `VbType` variant or modifier bit ever emits it; the name
  appears only in this file's module doc comment, and
  `grep -vE '^\s*//' crates/deform6/src/vb/functyp.rs | grep -c 'ParamArray'`
  is `0`.
- **Fifteen unassigned type codes** (`0x00`-`0x02`, `0x04`, `0x07`,
  `0x09`, `0x0E`, `0x11`, `0x12`, `0x14`-`0x1A`) are unproven; zero occur
  in the 193 measured records. `VbType::Unknown(u8)` is the landing
  place, per D-07.
- **`VbType::Internal`, `ComIFace` and `ComObj`** carry their trailing
  32-bit value as a raw, unresolved `Va`, not a class or library name.
  See "Defects found in the plan" below for why, and for the worked
  example (`Organism.CreateFromCreature`) that confirms the join is
  reachable when a later phase wants it.

## Gaps 6 and 7, closed with the numbers that closed them

**Gap 6**, the disputed `FuncTypDesc` header layout: `constFFFF` reads
`0xFFFF` in 193 of 193 measured records, and the word at header offset
`0x06` reads `0` in 193 of 193. Both numbers chose the aligned
`pubfunc_v2` layout over the shifted `func2Type` one; the other layout
would have read `constFFFF` from the low half of `optionalVals` and would
not have produced `0xFFFF`. `Prototype` carries both raw fields so
`tests/type_descriptors.rs` re-asserts this over the whole corpus through
the public API, not only trusts the internal gate.

**Gap 7**, the `optionalVals` target: closed over 61 of 61 qualifying
records and 89 value records, six OLE variant tags (`0` empty, `3`
four-byte integer, `4` four-byte single, `8` length-prefixed text, `11`
two-byte boolean, `17` one-byte integer plus padding). A seventh tag has
no sample; an unrecognised tag stops the walk and reports the record's
defaults unrecoverable rather than guessing a width. The grammar's
association with `Optional` arguments needed a correction beyond what
the plan stated; see "Defects found in the plan".

## The property-kind mask correction

`STRUCTURES.md` section 6.6 states the low three bits of `argSize` hold
the property kind, mask `0x07`, in the same paragraph that states the
entry count is the whole byte shifted right by two. Those two claims
overlap on bit 2 and cannot both be right. Measured over 193 records:
masked with `0x07`, 75 read as kind `4`, not a documented kind (`001`
Get, `010` Let, `111` Set); masked with `0x03`, exactly 4 are properties,
matching `cCommonDialog`'s `APIReturn`, `ExtendedError` (each `Get`) and
`CustomColor` (`Get` and `Let`) exactly, including their argument counts,
against the source at `corpus/vb6-code/Hidden-Markov-model/cCommonDialog.cls`.
Bit 2 belongs to the count, not the kind.

## Defects found in the plan

Two, both measured before being encoded as a test, both documented in
`vb/functyp.rs`'s own module doc comment and its type doc comments, not
only here.

### 1. `GetImageWidth`'s `arg_size`: the plan states `4`, measured `0x08`

Task 1's action text states: "Reading the descriptor for `GetImageWidth`
gives `arg_size` `4`." `FastDrawing.GetImageWidth` is
`Public Function GetImageWidth(ByRef srcPictureBox As PictureBox) As Long`,
a `Function` with one argument. Its type buffer therefore holds two
entries (the argument, then the return type, because `bFlags` bit 0 is
set), and two entries is `2 << 2 = 0x08`, not one entry's `0x04`.
Verified directly against `Grayscale.exe`'s bytes before writing the
corresponding test:
`grayscale_get_image_width_header_fields_match_the_measured_bytes`
asserts the measured `0x08` (through the entry count, since `arg_size`
is not itself an exposed field on `Prototype`), not the plan's inherited
`4`.

### 2. `optionalVals` is not one record per `Optional` argument

Task 3's action text states: "`cbValues` bytes of value records, one per
`Optional` argument, in argument order," and the plan's own gap register
row 7 says the same. The four worked examples the plan names (`Randomiza-
tionFX.exe`, `Emboss_Engrave.exe`, `Colorize.exe`, and `Edge_Detection.exe`'s
`VBGetOpenFileName`) all happen to have an exact 1:1 count between
`Optional` arguments and value records, so this held for every hand-picked
case. `tests/type_descriptors.rs`'s corpus-wide sweep, written for exactly
this reason, found four counter-examples on its first run:
`cCommonDialog::VBGetSaveFileName` and `VBChooseColor` (each with a
trailing `Optional flags As Long` carrying no default at all), and
`cSystemColorDialog::ShowColorDialog` (whose trailing `showFullDialog`
and `initColor`, both `Optional` with no default, are two arguments
short). Measured: a non-`String` `Optional` argument with no explicit
`= literal` default gets no `optionalVals` record; only a `String`-typed
one always gets a record (`Empty`/tag `0` when it has no default). Every
one of the four shortfalls sits in the trailing argument position,
consistent with VB6's own rule that `Optional` parameters are declared
contiguously at the end of a parameter list. `read_one` assigns the
resolved value records to the first `N` `Optional` arguments, front to
back, and leaves any remaining trailing ones with `default: None` rather
than rejecting the whole record. `OptionalDefaultsOutcome::Resolved(n)`
still carries the true record count, so the aggregate "89 value records"
figure is unaffected; only the per-argument assignment changed.

## The deliberate breakages

Five, matching the plan's five named breakages across the three tasks,
each run against the exact committed state before that task's commit,
observed, and reverted.

### 1. Task 1: `member_id` read from `0x0A` instead of `0x0C`

**One test failed**, exactly the third behaviour the plan names:

```
---- vb::functyp::tests::grayscale_get_image_width_member_id_top_bits_are_0x6003 ----
assertion `left == right` failed
  left: 0
 right: 24579
```

`24579` is `0x6003`. Restored; all 5 task-1 tests pass again.

### 2. Task 2, first: the shift `argSize >> 2` changed to `>> 1`

**7 of 15 tests failed**, not only the one the plan names
(`randomization_fx_draw_triangle_effect_gives_the_measured_arguments`,
which did fail, along with `grayscale_get_image_width_gives_one_argument_
and_a_return_type`, `modifiers_strip_in_order_matching_the_documented_
worked_examples`, `deliberate_breakage_property_kind_masked_with_0x07_
misreads_a_non_property_as_kind_4`, `argument_name_count_excludes_the_
return_entry`, `a_record_whose_const_ffff_is_wrong_is_unrecoverable_
with_a_recoverable_defect`, and `a_descriptor_pointer_in_no_section_
keeps_the_name_and_reports_no_prototype`). A halved entry count breaks
every test whose fixture has more than one type buffer entry, which is
most of them. Restored; all 15 tests pass again.

### 3. Task 2, second: the closure check removed (`closed = leading_byte_valid` only)

**Exactly 1 test failed**, precisely the fifth behaviour the plan names:

```
---- vb::functyp::tests::a_synthetic_all_zero_buffer_with_a_nonzero_arg_size_returns_rather_than_hangs ----
an all-zero buffer must never close
```

An all-zero buffer's first byte (`0x00`) is a legal leading byte (the
documented event marker), so `leading_byte_valid` alone is wrongly
`true` even though zero entries were found for a non-zero `argSize`.
Every corpus-derived test still passed, because no real record in this
corpus has a short walk; only the adversarial synthetic buffer this
behaviour was written to catch caught it. Restored; all 15 tests pass
again.

### 4. Task 2, third: the property-kind mask changed from `0x03` to `0x07`

**2 of 15 tests failed**, both predicted:

```
---- vb::functyp::tests::edge_detection_common_dialog_gives_exactly_four_property_records ----
assertion `left == right` failed
  left: Set
 right: Get

---- vb::functyp::tests::deliberate_breakage_property_kind_masked_with_0x07_misreads_a_non_property_as_kind_4 ----
assertion `left == right` failed
  left: Set
 right: None
```

Restored; all 15 tests pass again.

### 5. Task 3: the one-byte tag's padding changed from two bytes to one

**Both integration tests failed**, and the failure message named the
exact file the plan predicts:

```
---- all_func_typ_desc_records_close_over_the_whole_corpus ----
1 corpus file(s) failed:
.../corpus/vb6-code/Emboss-engrave-effect/Emboss_Engrave.exe: frmEmbossEngrave:
2 defect(s): [Defect { ... kind: CountMismatch { ... count: 4352, expected: 0,
other_field: "a value tag this file's grammar holds" } }, ...]

---- four_worked_default_values_match_the_source_beside_the_executable ----
assertion `left == right` failed
  left: None
 right: Some(Byte(127))
```

The misalignment desynchronised the byte cursor one record early, which
this run's grammar then read as an unrecognised tag (`4352` /
`0x1100`) rather than a plain length mismatch: a more informative real
failure than the plan's own prediction, and still located at exactly
`Emboss_Engrave.exe`. Restored; both integration tests and all 20 unit
tests pass again.

## The gate

Run on the committed tree at `12fb444`, with a clean working tree.

| Command | Exit |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0, 231 total tests across the workspace (209 before this plan; 20 new to `vb::functyp`, 2 new in `tests/type_descriptors.rs`) |
| `sh scripts/prove-lint-wall.sh` | 0 |
| `sh scripts/prove-region-wall.sh` | 0 |

```
$ cargo test -p deform6 --lib vb::functyp -- --list | grep -c ': test$'
20
$ cargo test -p deform6 --test type_descriptors -- --list | grep -c ': test$'
2
$ grep -vE '^\s*//' crates/deform6/src/vb/functyp.rs | grep -c 'ParamArray'
0
```

Only `crates/deform6/src/vb/functyp.rs` and the new
`crates/deform6/tests/type_descriptors.rs` were touched across all three
commits. `privateobj.rs`, `object.rs`, `classify.rs`, `project.rs`,
`vb/mod.rs`, `error.rs` and every other test file were not opened for
writing.

## `FuncTypDesc` records found: 193, matching the measurement twice over

`tests/type_descriptors.rs`'s sweep over all 44 vendored programs finds
exactly 193 resolved `FuncTypDesc` records, matching the number
`RESEARCH.md` reached independently by counting `lpProcNamesArray`
entries that pass the four-part identifier test (also 193, plan 02-03).
All 193 have `const_ffff == 0xFFFF` and `nul1 == 0`; all 193 have a type
buffer that closes at exactly `argSize >> 2` with every argument name
resolving; 61 of the 193 have a resolved `optionalVals` walk, holding 89
value records between them.

## Threat mitigations

| Threat | State |
|---|---|
| T-02-15, the type buffer walk as a denial of service | Mitigated. Bounded by [`MAX_TYPE_BUFFER_STEPS`] (4096) independently of the region's real size, and `arg_size` is one byte so the entry count it can ever demand is at most 63. `a_synthetic_all_zero_buffer_with_a_nonzero_arg_size_returns_rather_than_hangs` proves a 100,000-byte all-zero buffer with a non-zero `argSize` returns rather than scanning to the end. |
| T-02-16, the default-value walk as a denial of service | Mitigated. `cbValues` is bounded by `Region::subregion` against real mapped bytes before any record is read, and every value record consumes at least its own two-byte tag, so the walk is bounded by `cbValues / 2` regardless of content; `MAX_OPTIONAL_VALS_STEPS` is a second, independent bound. |
| T-02-17, `lpAryArgNames` read for the wrong count | Mitigated. The argument count comes from the closed type buffer walk, computed before any name pointer is read, exactly as the array (not null-terminated) requires. |
| T-02-18, a record whose `constFFFF` is not `0xFFFF` | Mitigated. Reported unrecoverable before any other field of the record is trusted. `a_record_whose_const_ffff_is_wrong_is_unrecoverable_with_a_recoverable_defect` is the instrument. |
| T-02-19, the trailing 32-bit value of an internal or COM type entry | Mitigated for the DoS/spoofing angle: `Va::new(trailing.unwrap_or(0))` never dereferences the value; nothing reads through it in this file. A future resolution layer that does dereference it must apply the same `region_at_va` discipline every other pointer in this crate uses. |
| T-02-20, the unreachable half of OBJ-04 | Accepted, as the plan's own threat model designates. `ParamArray` and the unassigned type codes have no corpus sample; both are named as gaps in the module doc comment and in this SUMMARY, never guessed. |

## Known Stubs

None that OBJ-04 needs to promise. `VbType::Internal`/`ComIFace`/`ComObj`
carry an unresolved `Va` rather than a name; this is documented as a
scope boundary (see "What OBJ-04 delivers, and what it does not"), not a
stub standing in for missing data — the raw address is itself the
correct, complete recovery this plan's file boundary can produce, and a
later phase's report layer is where the join to a name belongs.

## Deferred Issues

None found in scope. `EventDesc` (`lpEventsTypeInfo`) and `PubVarDesc`
(`lpPublicVars`) belong to plan 02-05, which this plan does not touch.

## Threat Flags

None. This plan reads existing in-image structures through the same
`Region`/`Va`/`PeImage` primitives every other module in this crate uses.
No new network endpoint, no new auth path, no new file-system access
path (`tests/type_descriptors.rs`'s own `std::fs` use is the test
harness's, matching `corpus_sweep.rs`'s existing pattern, not a new
capability of the library).

## Commits

| Commit | Subject |
|---|---|
| `ffff73f` | Read the FuncTypDesc header and validate the disputed offsets |
| `6d6fa85` | Walk the type buffer and decode the argument types and modifiers |
| `12fb444` | Recover the default value of an optional argument |

`git rev-list --count 0212f4e..HEAD` is 3, one commit per task, each
carrying its own tests.

## Self-Check: PASSED

`crates/deform6/src/vb/functyp.rs` and `crates/deform6/tests/
type_descriptors.rs` both exist. All three task commit hashes (`ffff73f`,
`6d6fa85`, `12fb444`) resolve in this branch's history. The gate table
above was run on the committed tree before this file was written.
