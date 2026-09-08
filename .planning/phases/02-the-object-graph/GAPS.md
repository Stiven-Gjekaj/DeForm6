# Phase 2 gap audit: the object graph

**Audited:** 2026-09-08. Phase 2 status confirmed as claimed: `cargo test
--workspace` runs 307 tests, 0 failed (204 + 1 + 7 + 15 + 9 + 27 + 2 + 0 +
20 + 22 + 0, summed from the actual run). `tests/ratios.toml` holds 44
entries, totals 185 recovered / 904 declared, matches its own header
comment. This is a measured re-run, not a re-statement of the SUMMARY files.

Ranked by whether Phase 3 is blocked. Nothing here is invented; every claim
below cites a file, a test name, or a command actually run.

---

## Blocks Phase 3 — none found

No gap in this audit blocks Phase 3 from starting. Phase 3's own risks
(STRUCTURES gaps 11 through 16, the opcode table, the scope-separator
grammar) are already named in `ROADMAP.md`'s Phase 3 section and are new
Phase 3 work, not carried-over Phase 2 debt. The one item that could have
blocked Phase 3 — whether `ByRef`/`ByVal` is a recovery gap or a printing
gap — is resolved below as a printing gap only, so it does not block WRT-07
in Phase 4 either, provided the Phase 4 emitter is written to read
`by_ref` rather than re-deriving the omission convention.

---

## 1. Requirement-to-test coverage (OBJ-01..06, VER-01..05)

All eleven requirements have an executable test that would fail if the
requirement stopped holding, not just a SUMMARY claim. Traced by requirement:

| Req | Test that fails if unmet | File |
|---|---|---|
| OBJ-01 | `every_declared_object_is_recovered_and_every_recovered_object_is_declared_across_the_corpus` | `crates/deform6/tests/differential.rs` |
| OBJ-02 | `crates/deform6/src/vb/classify.rs` unit tests, e.g. `no_corpus_file_makes_the_two_optional_info_markers_disagree`, plus differential kind comparison | `classify.rs`, `differential.rs` |
| OBJ-03 | `every_declared_public_procedure_is_recovered_and_every_recovered_one_is_declared_across_the_corpus` | `differential.rs` |
| OBJ-04 | `crates/deform6/tests/type_descriptors.rs`, whole-corpus prototype comparisons in `differential.rs` | both |
| OBJ-05 | `vb::project::tests::eight_of_nine_grayscale_entries_are_external_and_the_ninth_is_skipped` and related `Declaration`/`ExternalComponent` tests | `vb/project.rs` |
| OBJ-06 | `vb::privateobj::tests` — a null entry asserted `Private`, e.g. the deliberate-breakage log for "a null entry given `Procedure::Public` instead of `Private`" | `privateobj.rs` |
| VER-01 | `differential.rs`, run over all 44 corpus programs | `differential.rs` |
| VER-02 | glob-vs-`.vbp` regression covered by the two `cCommonDialog.cls` corpus cases exercised inside `differential.rs`'s selection logic | `differential.rs`, `support/vbp.rs` |
| VER-03 | `SK-MCI-Sample__VB6` / `SK-Gradient-Sample__VB6` sibling-`ExeName32` resolution, exercised in `differential.rs` | `differential.rs` |
| VER-04 | `support/rules.rs` exclusion-rule tests | `support/rules.rs` |
| VER-05 | `ratios::raising_a_pinned_recovered_count_fails_with_regression`, `ratios::lowering_a_pinned_recovered_count_fails_with_moved_up_and_the_exact_paste_block`, `ratios::the_pinned_file_holds_forty_four_entries_and_the_totals_one_hundred_eighty_five_and_nine_hundred_four` | `crates/deform6/tests/ratios.rs`, `crates/xtask` |

No requirement in this set rests only on a SUMMARY claim. One caveat: OBJ-02's
`Unknown` fallback path (any `fObjectType` outside the three measured values)
has unit-test coverage using synthetic values, not a corpus file — recorded
below under "untested by construction," not treated as a coverage hole here.

---

## 2. STRUCTURES.md gap register — current state

19 numbered rows (not 18; row 19 was added during Phase 1 for the object-count
dispute and is already closed).

| # | Gap | State after Phase 2 | Blocks Phase 3? |
|---|---|---|---|
| 1 | `VBHeader` 0x58/0x5C | Closed (Phase 1) | No |
| 2 | `OptionalObjectInfo` presence test | **Closed by measurement.** `classify.rs`'s `has_optional_info` uses `fObjectType & 0x2`, both rejected alternatives documented in the doc comment. | No — but Phase 3 depends on this bit to decide whether to read `OptionalObjectInfo` for controls; already resolved. |
| 3 | MDIForm `fObjectType` value | **Still open.** `classify.rs` explicitly routes any unmeasured value to `Unknown(u32)` rather than guessing (line 57-61 doc comment). Corpus has 0 MDIForm objects — confirmed by a fresh grep, `find corpus -iname "*.frm" \| xargs grep -l "VB.MDIForm"` returns nothing. | **No, harmless to defer.** MDIForm is Phase 4/6 territory (FILE-FORMATS gap 9 already names it there); Phase 3's control tree work does not need this value, since no corpus form is an MDI child. |
| 4 | `ParamArray` type encoding | **Still open, correctly so.** `functyp.rs` doc comment: "`ParamArray` occurs nowhere in this corpus's source, in 44 programs." No fixture closes it. | **Does not block Phase 3** (forms/controls, not procedure signatures) but does cap OBJ-04 permanently until a compiled sample exists. |
| 5 | 15 unassigned type codes | **Still open.** `VbType::Unknown(u8)` carries the raw byte; no code guessed. | **Does not block Phase 3.** Blocks full OBJ-04 coverage, which is Phase 2/4 territory, not the control tree. |
| 6 | `FuncTypDesc` 0x06-0x0B | Not re-touched in Phase 2 beyond what 02-04's SUMMARY records (`constFFFF` behavior observed, not formally re-verified against a second independent source). Left open. | No |
| 7 | `optionalVals` target | **Closed by measurement**, but differently than the register states: 02-04's SUMMARY records the corrected grammar (61 of 61 value records over 89, six OLE tags, variable length 4/6/10/12/60), which is stronger than "resolve via debugging the runtime." This should be marked closed in the register text, not left as open resolution-path prose. | No |
| 8 | `PubVarDesc` record stride | **Still open**, and newly informed: `cnt_public_vars` is confirmed unreliable (a class with 0 `Public` vars reports 4, same shape as the withdrawn `wCompiledObjects` capacity bug). `PrivateObj::public_var_field()` exposes the raw count as a `Gap`, never builds a stride-based walk on top of it. | No — Phase 3 does not read `PubVarDesc`. |
| 9 | Event name strings | **Still open by design.** 02-05 ships `event_descriptor_addresses` with `cnt_events == 0` in 97 of 97 objects and a synthetic-fixture test (`crates/deform6/src/vb/privateobj.rs`) documented to fire "the day a real sample arrives." | **Partially blocks Phase 3.** FRM-06 needs event handler *names* bound to *controls*, which is a different table (the control-level event slot, §8.6) from this gap (the object-level `EventDesc` array, §6.4). Confirm this distinction is understood by whoever plans 03-09 — the two are not the same unknown. |
| 10 | Ordinal `Declare` encoding | **Still open, correctly marked.** `ExportName::OrdinalInferred(u32)` is a distinct variant from a resolved name, and the doc comment on `Declaration::NAME_MARKER` states plainly that neither an alias nor an argument list survives compilation. | No |
| 11-16 | Control/form-stream gaps | Untouched by Phase 2 (out of its scope). Named correctly as Phase 3 risks in `ROADMAP.md`. | **Yes — these are exactly what block Phase 3**, and the roadmap already names them as such. Not new findings from this audit. |
| 17 | External component entry unknown dwords | **Still open**, unchanged by Phase 2. `vb/project.rs`'s `ExternalComponent` reader leaves the five unknown dwords opaque. | No — CLSID/library-name join is what OBJ-05/FRM-04 need, and that part is recovered. |
| 18 | Entry-point stub variants | Closed as "do not implement," Phase 1. | No |
| 19 | Object count field | Closed (Phase 1). | No |

**Newly discovered during Phase 2, not in the original 18/19:**

- `cntPublicVars` unreliability (documents gap 8's stride problem further, does
  not add a new numbered gap but sharpens it — recorded in CONTEXT.md and in
  `privateobj.rs`).
- The procedure name array's 428-of-1029-slot "neither null nor a resolvable
  name" category (CONTEXT.md, "The procedure name array does not behave as
  documented"). This is validated and reported per-entry as a defect with its
  raw address, not silently dropped — confirmed by reading
  `crates/deform6/src/vb/privateobj.rs`'s handling of a non-resolving non-null
  entry. Not a blocker; it is a measured, reported behavior.

---

## 3. WINDOWS.md ledger

| id | Phase | Description | Closed / left / worsened | Blocks Phase 3? |
|---|---|---|---|---|
| 1 | 01 | Section overlap rule untested by construction | Left open, unchanged. Not Phase 2's to fix. | No — Phase 3 reads forms via `lpGuiTable`/`GUIObjectInfo`, not via section-overlap logic. |
| 2 | 01 | P-code branch untested (no corpus P-code file) | Left open, unchanged. Explicitly acknowledged as untestable-by-construction across the whole roadmap through Phase 6. | No |
| 3 | 01 | `inspect` dropped collected defects (`Report` derives `PartialEq`, `Defect` did not) | **Fixed**, timestamped 2026-09-08T10:55:45Z, one day before this audit. Phase 2 needed this fix since it reports gaps/inferences (CONTEXT.md says so explicitly) and the ledger confirms it landed. | N/A — resolved, and correctly a Phase-2 prerequisite that got done. |
| 4 | 01 | `runtime_dll` match cannot be proven case-sensitive (`imported_dlls` upper-cases, `classify` only compares upper-case) | **Left open.** CONTEXT.md flagged it as "take it only if it is free" for Phase 2; it was not taken. Still open. | No — unrelated to forms. |
| 5 | 02 | A non-null `lpProcNamesArray` entry that fails validation (bad NUL-termination or bad identifier chars) is handled in code but only exercised via the "address does not resolve" path in the 3 vendored programs actually walked with a corpus sample | **Left open, correctly acknowledged.** This is Phase 2's own new finding, filed at the ledger's `unrun-verify` kind (code path exists, exercise is thin), not `unmet-truth`. | No — orthogonal to Phase 3's forms work. Should be picked up with a synthetic fixture at low cost; see recommendation below. |

None of the five items make Phase 2 worse; ledger 3 improved during Phase 2's
window (fixed one day before this audit runs). No open WINDOWS.md item blocks
Phase 3.

---

## 4. Untested-by-construction items

| Item | Recorded where a reader finds it? | Test that fires on first real sample? | Cheap synthetic fixture possible? | Verdict |
|---|---|---|---|---|
| P-code branch | Yes — `ROADMAP.md` Phase 1 named risks, WINDOWS.md #2, `STRUCTURES.md` §12 (measured-against section) | No dedicated fire-on-arrival test; WINDOWS.md #2 is a standing ledger entry, not a test | Partially — a copy of a native program with `lpNativeCode` zeroed is already used per WINDOWS.md #2's own description, but that only proves the code path runs, not that it reads a *real* P-code layout correctly | **Correctly "cannot be tested," not "was not tested."** No further action needed for Phase 2; carried forward as documented debt into Phase 6. |
| Section overlap | Yes — WINDOWS.md #1 | No | A synthetic PE with two overlapping sections is cheap and was explicitly used elsewhere in Phase 1 for a similar case (Mandelbrot's RVA==offset identity trap) — the *pattern* for closing this exists in the codebase already | **"Was not tested" more than "cannot be tested."** Belongs to Phase 1/5 scope, not a Phase 2 gap, but worth flagging as cheap to close whenever picked up. |
| `ParamArray` | Yes — `ROADMAP.md` risk, `functyp.rs` doc comment, STRUCTURES gap 4 | No | **No** — the encoding itself is unknown; a synthetic fixture cannot manufacture the correct byte layout, only guess at one, which is exactly what D-07 forbids | **Correctly "cannot be tested."** A synthetic fixture here would violate the project's own "do not guess a type code" rule. |
| 15 unassigned type codes | Yes — STRUCTURES gap 5, `VbType::Unknown(u8)` doc comment | No | No, same reasoning as `ParamArray` | **Correctly "cannot be tested" without a real compiled sample.** |
| Event descriptors (`EventDesc`) | Yes — STRUCTURES gap 9, `privateobj.rs` module doc | **Yes** — a named unit test in `privateobj.rs` exercises `event_descriptor_addresses` against a synthetic non-zero `cnt_events` fixture, and the module doc explicitly promises it "fires the day a real sample arrives" | Already done | **Best-handled item in the audit.** No action needed. |
| MDIForm object kind | Yes — STRUCTURES gap 3, `classify.rs` doc comment | No dedicated "fires on arrival" test; `Unknown` fallback is generically tested with synthetic `fObjectType` values, not one specifically annotated as "this is the MDIForm slot" | Cannot manufacture the correct value without a real compiled MDIForm; a wrong guess here is exactly what D-07/D-08 forbid | **Correctly "cannot be tested."** Deferrable — MDIForm is out of the Phase 3 control-tree critical path (0 corpus MDI forms). |
| UserControl object kind | Same as MDIForm — grouped under `Unknown` generically, not distinguished | No | Same — cannot manufacture without a real compiled ActiveX control | **Correctly "cannot be tested."** PRJ-01 (ActiveX control projects) is v2 scope per REQUIREMENTS.md, so this is expected to stay open past v1. |
| Ordinal `Declare` aliases | Yes — STRUCTURES gap 10, `project.rs` doc comment on `ExportName::OrdinalInferred` | **Yes, partially** — `OrdinalInferred(u32)` unit tests exist for the parse rule (`"#12a"` is a plain name, `"#123"` is an ordinal), but no test fires specifically "the day a real ordinal-`Alias` sample arrives," because the corpus has zero `Alias "#N"` declarations to compare against | A synthetic fixture proving the *parse rule* is cheap and already exists; a fixture proving the *runtime encoding* cannot be manufactured without a real compiled sample, same reasoning as `ParamArray` | **Correctly split: parse rule is tested, encoding gap correctly left open.** |

Two items (event descriptors, ordinal Declare parsing) show the project doing
exactly what "cannot be tested" vs "was not tested" is supposed to mean:
build the cheap synthetic proof for the part that can be proved, and refuse
to guess the part that cannot. The section-overlap item is the one place
where the same cheap pattern used elsewhere in Phase 1 was not applied — low
cost, not Phase 2's responsibility to fix.

---

## 5. The corpus itself as a gap

Measured directly against the corpus on disk, not assumed:

| Phase 3 need | Corpus count | Source of count |
|---|---|---|
| `.frm` files at all | 54 | `find corpus -iname "*.frm" \| wc -l` |
| MDI forms (`VB.MDIForm`) | **0** | `grep -l "VB.MDIForm" corpus/**/*.frm` — no match |
| UserControls (`.ctl` files) | **0** | `find corpus -iname "*.ctl" \| wc -l` |
| Menus (`Begin VB.Menu`) | 17 files | `grep -l "Begin VB.Menu" corpus/**/*.frm` |
| Control arrays (`Index =` lines) | 35 files | `grep -l "Index *=" corpus/**/*.frm` |
| Third-party OCX controls (non-`VB.*` `Begin` lines in `.frm`) | **0** | `grep -ohE "^ *Begin [A-Za-z0-9_.]+\." corpus/**/*.frm \| grep -v "Begin VB\."` — empty result |
| Any OCX reference at the project level (`.vbp` `Object=` line) | **1**, and it is `COMDLG32.OCX` (the standard Windows common-dialog control, shipped with every VB6 install, not a third-party control) | `grep -h "^Object=" corpus/**/*.vbp` |

**What this means concretely for Phase 3:**

- **Menus and control arrays are covered.** 17 and 35 files respectively give
  real material to test FRM-01/FRM-02 tree recovery against.
- **FRM-04 (third-party OCX CLSID recovery) has essentially no corpus
  coverage.** The one `Object=` line in the whole 44-program corpus is
  `COMDLG32.OCX`, a system-shipped control most VB6 installs already have,
  not a genuine third-party control with its own unrecoverable property blob.
  Phase 3's plan 03-08 ("External OCX controls — `cType 255`, the class name,
  the CLSID join") will need a **synthetic or externally-sourced fixture**,
  because the corpus cannot exercise this path meaningfully even though it
  technically contains one OCX reference. This is worth raising before 03-08
  is planned, not discovered mid-plan.
- **MDIForm and UserControl are absent, matching the roadmap's own
  acknowledgment** (FILE-FORMATS gap 9 for MDIForm; PRJ-01 for UserControl,
  deferred to v2). Phase 3 does not list either as a success criterion, so
  this is consistent, not a surprise gap.
- **Menus need the `fMdlIntCtls` Menu bit cross-check** that STRUCTURES gap 14
  names (`Validate menus against the fMdlIntCtls Menu bit and a cType == 19
  count`). 17 files exist to validate against; this is a real, exercisable
  check, not an untestable one.

---

## 6. The `ByVal` finding — recovery gap or printing gap?

**Checked against the code, not just CONTEXT.md's own claim.**

`crates/deform6/src/vb/functyp.rs` defines `TypeEntry.by_ref: bool`, read at
line 620 as `byte & 0x20 != 0` — a genuine bit read from the file, not a
derived default. The type is exercised by real tests: `ByRef Long`, `ByRef
Long array`, `Optional ByRef Long`, and `ByRef Variant` are each asserted
with `by_ref == true` against real corpus bytes (`functyp.rs` test module,
lines ~1495-1521).

`crates/deform6-cli/src/main.rs` line 288 prints `"ByRef "` only when
`arg.entry.by_ref` is `true`, and prints nothing otherwise. There is no
separate `by_val` field, and no code path anywhere in the crate throws away
a recovered "explicit ByVal" fact — because the bit itself only ever encodes
one direction (`ByRef` present/absent), matching COM automation's own
`[in]`/`[in, out]` convention where by-value is the unmarked case.

**Conclusion: this is a printing gap, confirmed, not a recovery gap.** The
byte the compiler wrote distinguishes exactly two states, and DeForm6
recovers both — it simply omits the modifier word for the unmarked one.
`CONTEXT.md`'s own claim ("The recovery itself is correct. Only the printed
form is short.") holds up against the actual field definitions and the
actual printer.

**What Phase 4 needs to do about it:** WRT-07 needs `inspect`'s successor
writer to emit `ByVal` explicitly when `by_ref` is `false`, rather than
relying on the reader's own knowledge that VB6 defaults to `ByRef`. This is
a one-line change to the emitter, not new parsing work, and does not block
Phase 3 at all — Phase 3 does not print procedure signatures.

---

## Summary ranking

1. **Not a blocker, but plan around it:** FRM-04's OCX CLSID work (03-08) has
   no real third-party-OCX corpus material. Confirmed by direct grep, not
   assumed. Recommend a synthetic `.frm`/`.vbp` fixture with a fabricated
   third-party CLSID before or during 03-08's planning.
2. **Not a blocker, worth a one-line clarification in Phase 3 planning:**
   STRUCTURES gap 9 (object-level `EventDesc`) and Phase 3's event-handler-name
   work (FRM-06, plan 03-09) are two different unknowns — the control-level
   event slot table has no numbered STRUCTURES gap of its own beyond what
   `ROADMAP.md`'s "The event name table" risk already names. Make sure 03-09's
   plan does not conflate the two.
3. **Not a blocker, cheap to close whenever picked up:** WINDOWS.md #4
   (`runtime_dll` case-sensitivity) and #5 (`lpProcNamesArray` validation-failure
   path) remain open and were correctly left open rather than silently closed.
4. **Confirmed non-issue:** the `ByVal` question is a printing gap, not a
   recovery gap — verified against the actual `TypeEntry` struct and printer,
   not taken on the SUMMARY's word.
5. **Confirmed non-issue:** all eleven OBJ/VER requirements have real,
   executable, currently-passing tests; none rests only on a SUMMARY claim.

No implementation bug was found. No test was written or modified as part of
this audit — the existing 307 were re-run and matched their claimed count and
result exactly.
