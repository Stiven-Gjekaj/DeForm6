---
phase: 05-hostility
reviewed: 2026-09-14T20:06:45Z
depth: standard
files_reviewed: 26
files_reviewed_list:
  - crates/deform6/src/journal.rs
  - crates/deform6/src/error.rs
  - crates/deform6/src/report.rs
  - crates/deform6/src/vb/mod.rs
  - crates/deform6/src/vb/gui.rs
  - crates/deform6/src/vb/privateobj.rs
  - crates/deform6/src/vb/functyp.rs
  - crates/deform6/src/vb/object.rs
  - crates/deform6/src/vb/opcodes.rs
  - crates/deform6/src/vb/project.rs
  - crates/deform6/src/vb/frx.rs
  - crates/deform6/src/vb/vbstr.rs
  - crates/deform6/src/vb/propstream.rs
  - crates/deform6/src/write/code.rs
  - crates/deform6/src/write/frm.rs
  - crates/deform6/src/write/mod.rs
  - crates/deform6/src/write/model.rs
  - crates/deform6-cli/src/main.rs
  - crates/deform6/fuzz/fuzz_targets/parse.rs
  - crates/xtask/src/fuzz.rs
  - crates/xtask/src/fetch_corpus.rs
  - crates/deform6/tests/no_panic_proof.rs
  - crates/deform6/tests/fuzz_smoke.rs
  - crates/deform6/tests/regressions.rs
  - crates/deform6/tests/support/hostile.rs
  - .github/workflows/fuzz.yml
findings:
  critical: 1
  warning: 1
  info: 0
  total: 2
status: fixed
resolution: CR-01 and WR-01 fixed in commits 9f48ead, 1c955a3, 994a3e8, bb5cc3f, 6e798c1, 5502497, 5581fdf.
---

# Phase 05: Code Review Report (hostility)

**Reviewed:** 2026-09-14T20:06:45Z
**Depth:** standard
**Files Reviewed:** 26 (of 45 in the named scope; the remainder are `Cargo.lock`, `corpus/manifest.toml`, a binary fixture, and `.github/workflows/gate.yml`'s unrelated pre-existing lines, all low signal per the review brief)
**Status:** issues_found

## Summary

This review covers commits `7e3a455..fcb3bc7` (phase 05, "hostility"), already shipped
underneath a released 1.0.0 and underneath phase 6.

The phase's positive engineering is real and verifiable: `GuiTable::bound_form_count` bounds
`wFormCount` against the GUI table's own mapped region before any allocation, proven by a
literal-built 4096-byte fixture that gives exactly one entry and one `ImplausibleCount` defect
for a declared count of `0xFFFF` (`crates/deform6/src/vb/gui.rs:94-172`). The capacity audit in
plan 05-02 is exhaustive and reproducible. The `PathIssuer`/`SafeNameIssuer` O(n^2) fix
(`crates/deform6/src/report.rs`, `crates/deform6/src/write/model.rs`) is correct: the loop still
checks set membership on every candidate, so the per-key suffix cache is a fast path, never a
shortcut that could hand out a duplicate name. The fuzz harness
(`crates/deform6/fuzz/fuzz_targets/parse.rs`) genuinely drives both `Mode::Strict` and
`Mode::Salvage` plus the writer, asserting nothing about the result, which is the correct shape
for a parser that legitimately refuses. `crates/deform6/tests/no_panic_proof.rs` is not a token
sweep: it drives all 44 real vendored executables (plus the fetched set and the regression
directory) through both modes and the writer, and proves the count it read equals the count
each source independently reports. `crates/deform6/tests/fuzz_smoke.rs`'s deterministic mutation
sweep is what actually found the O(n^2) defect the learnings document names, which is real
evidence the harness has teeth.

Against that backdrop, one finding below is a genuine regression with a concrete, empirically
reproduced failure scenario: **the severity model plan 05-01 wired in refuses whole files that
the surrounding code was explicitly written to salvage, in both `Strict` and `Salvage` mode,
because several `DefectKind` sites tag a per-item, recoverable condition with a `DefectKind`
whose declared severity is `Fatal`.** This directly contradicts the phase's own stated contract
("A `Recoverable` defect refuses in `Mode::Strict` and gives back `fallback` in `Mode::Salvage`")
and defeats `--salvage`'s purpose for a class of real, not merely hypothetical, VB6 executables.

## Critical Issues

### CR-01: `--salvage` cannot salvage a file with an unresolvable per-item pointer, because several `DefectKind` construction sites reuse the `Fatal`-severity `UnmappedAddress`/`OffsetOverflow` kinds for conditions the surrounding code explicitly designed to be non-fatal

**File:** `crates/deform6/src/vb/project.rs:660-681` (primary site), with the same pattern at
`crates/deform6/src/vb/frx.rs:169-175`, `crates/deform6/src/vb/vbstr.rs:165-176`, and
`crates/deform6/src/vb/functyp.rs:923-930`.

**Issue:**

`crate::error::DefectKind::severity()` (`crates/deform6/src/error.rs:319-372`) hard-codes
`UnmappedAddress` and `OffsetOverflow` as `Severity::Fatal`, documented there as "a spine pointer
that maps nowhere stops the walk" and "every later bound check rests on that range". That is the
correct severity for the *one* spine use each kind was designed for (`ObjectTableHead`/`GuiTable`
walk failures that really do make the rest of the file unreadable).

Plan 05-01 wired `Journal::record` into `deform6::inspect`'s choke point
(`crates/deform6/src/vb/mod.rs:424-441`): every defect collected anywhere in the read, from
whatever call site, is now recorded through `journal.record`, and per
`crates/deform6/src/journal.rs:78-88`, `(Severity::Fatal, Mode::Strict)` and
`(Severity::Fatal, Mode::Salvage)` both give `Err(Error::Refused(defect))`. Before this wiring
(pre-phase-5), `inspect` never consulted `severity()` for a collected defect at all; the git diff
for `crates/deform6/src/vb/mod.rs` shows this choke point is new in this phase's commit range.

Several non-spine call sites construct a `Defect` with one of these two Fatal kinds for a
condition that is, by the surrounding code's own design and its own doc comments, a per-item,
recoverable loss that should cost one item and let the read continue:

- `crates/deform6/src/vb/project.rs:521-527` (module doc comment, `DeclareTable::read`): *"Three
  recoverable outcomes below reuse a `DefectKind` variant whose own `severity()` disagrees with
  the word 'recoverable' used here: `DefectKind::UnmappedAddress` is `Fatal` in `error.rs`...
  Losing one entry's descriptor is not that: the other entries still resolve... this doc comment
  names the mismatch rather than silently curing it, and the caller here never consults
  `severity()` to decide whether to continue."* The author identified the vocabulary mismatch and
  explicitly chose not to fix it, apparently not realizing (or the fix landing after this code)
  that a downstream caller: `Journal::record`, wired in by this same phase: *would* consult
  `severity()`.
- `crates/deform6/src/vb/frx.rs:169-175` (`extract_blob`): a `checked_add` overflow while
  resolving one resource blob's declared length becomes `DefectKind::OffsetOverflow`, returned
  through the same `(Option<Blob>, u32, Option<Defect>)` signature used for the adjacent
  `ImplausibleCount` (Recoverable) case one branch above it. `crates/deform6/src/vb/propstream.rs:874-879`
  confirms this defect is pushed onto the running per-form defect list and the property is
  marked `PropertyValue::BlobUnreadable`, exactly the "cost one item, keep going" shape
  `Tolerated`/`Recoverable` defects use elsewhere in this codebase.
- `crates/deform6/src/vb/vbstr.rs:165-176` (`VbStr::overflow`): a string's declared-end
  arithmetic overflow becomes `DefectKind::OffsetOverflow`, returned as `(Self, Option<Defect>)`
  for the caller to record and move on, the same per-property shape `UnrecoverableString`
  (`Tolerated`) uses two branches away.
- `crates/deform6/src/vb/functyp.rs:918-930` (`OptionalValsWalk`): a `checked_add` overflow while
  walking one prototype's optional-value records becomes `DefectKind::OffsetOverflow`, returned
  as `OptionalValsWalk::Unrecoverable` for that one prototype, while the rest of the read
  continues.

**Empirical reproduction** (not merely reasoned about: run against this tree, then reverted):

Using the same fixture recipe `crates/deform6/src/vb/project.rs`'s own unit test
`a_descriptor_address_in_no_section_produces_a_defect_and_the_walk_finishes` already builds
(patch `Mandelbrot.exe`'s one `Declare` entry's `lpImportDescriptor` field to an address in no
section), calling `deform6::inspect` directly (rather than `DeclareTable::read` directly, which
is what the existing unit test does and which is why this was never caught) gives:

```
strict  = Err(Damaged("address 0x1300000 at offset 0x1708 is in no section (DeclareTableEntry.lpImportDescriptor)"))
salvage = Err(Damaged("address 0x1300000 at offset 0x1708 is in no section (DeclareTableEntry.lpImportDescriptor)"))
```

in **both** modes. `Mandelbrot.exe` is otherwise a complete, valid VB6 executable (its own test
suite proves its form, its declarations, and its objects all read cleanly). A user running
`deform6 extract --salvage` against any VB6 executable whose import table holds one
declare-style entry with an unresolvable descriptor: a plausible condition for a packed,
protected, or simply unusually-linked executable outside the 44-program vendored corpus, not an
exotic hypothetical: gets a total refusal (exit code 4) and recovers nothing, even though every
other form, object, and declaration in the file is perfectly readable. This is exactly the
scenario `--salvage` exists to rescue, per its own `--help` text
(`crates/deform6-cli/src/main.rs:76-85`: *"Continues past a defect this run had to assume a
value for, instead of refusing"*).

This was not caught by `crates/deform6/tests/severity_census.rs`, which asserts that every
defect the 44 vendored programs raise is `Tolerated`: none of the 44 happens to exercise this
code path (confirmed: the census passes today), so the census proves nothing about it. It was
not caught by the fuzz harness or `fuzz_smoke.rs` either, because both assert nothing about the
result variant; a false full-file refusal is not a panic and produces no crash artifact. It is a
silent availability/correctness regression, invisible to every one of this phase's own safety
nets, which is why it survived a green gate and a 23/23 UAT pass.

**Fix:**

Give `DeclareTable::read`'s three "recoverable outcomes" (and the `frx.rs`, `vbstr.rs`, and
`functyp.rs` per-item sites above) their own `DefectKind` variant(s) with an honest severity -
`Tolerated` matches the stated behaviour best ("the item... keeps its other fields", nothing
invented in its place): rather than reusing `UnmappedAddress`/`OffsetOverflow`, whose
`severity()` is deliberately `Fatal` for the one spine-pointer use case those kinds were built
for. For example:

```rust
// error.rs
#[error("address {va:#x} at offset {offset:#x} resolves to nothing, and the item is skipped")]
DeclareDescriptorUnresolved { offset: u32, va: u32 },
// ...
Self::DeclareDescriptorUnresolved { .. } => Severity::Tolerated,
```

and update `DeclareDescriptorFailure::into_defect` (`crates/deform6/src/vb/project.rs:660-681`)
and the three `frx.rs`/`vbstr.rs`/`functyp.rs` sites to build the new kind(s) instead of
`UnmappedAddress`/`OffsetOverflow`. Then extend `severity_census.rs` (or a dedicated test) to
prove, for each of these four sites, that `inspect()` itself: not the private `*::read` helper -
succeeds when only that one item is unresolvable, matching the pattern
`crates/deform6/src/vb/gui.rs`'s own `an_implausible_form_count_against_a_real_program_still_refuses_and_never_panics`
already uses to test through the public entry point rather than the internal walker alone.

## Warnings

### WR-01: The one existing unit test for this exact defect does not assert severity, and would have caught CR-01 if it had

**File:** `crates/deform6/src/vb/project.rs:1670-1684`

**Issue:** `a_descriptor_address_in_no_section_produces_a_defect_and_the_walk_finishes` asserts
`table.declarations.is_empty()`, `table.defects().len() == 1`, the `DefectKind` variant, and the
site's field name: but never `defect.kind.severity()`. Its sibling test seven lines above,
`an_entry_type_that_is_neither_six_nor_seven_is_skipped_and_flagged`, does assert
`defect.kind.severity() == Severity::Recoverable` for its own defect. Had this test made the
same assertion, it would have failed the moment `error.rs`'s own severity table classified
`UnmappedAddress` as `Fatal`, since the module's own doc comment
(`crates/deform6/src/vb/project.rs:517-527`) already documents that this specific use is meant
to be recoverable. The test proves the walk does not stop, but never proves the walk's own
notion of "not stopping" (a private `*::read` call) agrees with what `inspect()`'s choke point
will actually do with the defect it returns.

**Fix:** Add `assert_eq!(defect.kind.severity(), Severity::Tolerated);` (or `Recoverable`,
whichever the fix for CR-01 lands on) to this test, and add the missing severity assertion to any
other test in `frx.rs`, `vbstr.rs`, and `functyp.rs` that exercises one of the sibling call sites
named in CR-01.

---

_Reviewed: 2026-09-14T20:06:45Z_
_Depth: standard_

---

## Resolution

Applied on 2026-09-14, after the review. The user approved the fix.

### What was wrong

`DefectKind::UnmappedAddress` and `DefectKind::OffsetOverflow` are `Fatal`. That is correct for
the spine pointer and the spine range each was written for. Four sites reused them for a
per-item loss. Phase 5 then wired `Journal::record` into the `inspect` choke point in
`vb/mod.rs`, which replays every collected defect through the severity gate. From that moment,
one unresolvable `Declare` descriptor refused the whole file in both `Strict` and `Salvage`.
That defeats the feature Phase 5 exists to deliver.

A doc comment in `project.rs` had already named the vocabulary mismatch and judged it harmless,
because at the time "the caller here never consults `severity()`". Phase 5 made that sentence
false and nobody updated it. `severity_census.rs` did not catch the defect because it proves a
property over the 44 vendored corpus programs, and none of them reaches these four sites.

### What changed

Two new `DefectKind` variants, both `Severity::Recoverable`, so the loss refuses in `Strict`
and degrades in `Salvage`:

| Variant | Mirrors | For |
|---|---|---|
| `ItemAddressUnmapped` | `UnmappedAddress` | a per-item address that resolves nowhere |
| `ItemOffsetOverflow` | `OffsetOverflow` | a per-item offset and length that overflow |

`UnmappedAddress` and `OffsetOverflow` keep `Fatal`. Lowering them would break the spine case
they were written for.

Four sites move to the new variants. Each was checked against its caller first:

| Site | Why it is per-item |
|---|---|
| `project.rs` `DeclareDescriptorFailure::into_defect` | `DeclareTable::read` loops over independent entries |
| `frx.rs` `extract_blob` length overflow | called per control inside `walk_properties` |
| `vbstr.rs` `VbStr::overflow` | called per control, scoped to one property block |
| `functyp.rs` optional-value cursor overflow | scoped to one prototype |

None was left `Fatal`. The genuine spine walks (`ObjectTable::walk`, `GuiTable::walk`) return
`Refusal::Damaged` directly and never went through `DefectKind`, so they are untouched.

The stale doc comment in `project.rs` is rewritten. It no longer claims the caller ignores
`severity()`.

### Ripple

`crates/deform6/schema/report.schema.json` is a Phase 6 deliverable and enumerates the variants
in a `oneOf`. It now carries 19 rather than 17. `severity_census.rs` gained the two match arms
its `kind_name` needs, since `severity()` has no wildcard arm by design. The corpus census
numbers are unchanged at 429 defects across 36 of 44 programs, because none of the 44 vendored
programs reaches these four sites.

### Measured

A regression test pair now holds the property, in the shape `salvage.rs` already uses: patch a
committed corpus file's bytes in memory, never on disk.

```
an_unresolvable_declare_descriptor_refuses_in_strict_mode                          ok
an_unresolvable_declare_descriptor_succeeds_in_salvage_mode_losing_only_that_entry ok
```

Before the fix both modes returned `Err(Damaged(...))`. After it, `Strict` still refuses and
`Salvage` succeeds with `declarations` empty and every other fact unchanged.

WR-01 is also fixed: the unit tests in `project.rs`, `frx.rs` and `vbstr.rs` now assert
`defect.kind.severity()`, which is what would have caught this before it shipped.

Full suite after the fix: 986 tests pass, `cargo doc --no-deps --workspace` emits zero
warnings, and every wall script plus the claim surface check exits 0.
