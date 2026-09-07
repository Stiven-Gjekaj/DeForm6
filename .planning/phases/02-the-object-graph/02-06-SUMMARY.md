---
phase: 02-the-object-graph
plan: 06
subsystem: parsing
tags: [vb6, pe-format, declare-statement, external-imports, ocx-component-table]

requires:
  - phase: 01-it-reads-the-file
    provides: "ProjectInfo.lp_external_table/dw_external_count and VbHeader.lp_external_table/w_external_count, both read but unused"
provides:
  - "DeclareTable::read, walking the Declare import table (dwEntryType == 7) and skipping internal entries (dwEntryType == 6) silently"
  - "Declaration and ExportName, marking what a Declare statement does not hold (procedure name/Alias, argument list, module/scope) and what an ordinal alias looks like when inferred"
  - "ComponentTable::read, walking the external component table by declared entry length, self-describing and variable-length"
  - "Component, carrying the file name, library name and component name a form's external control joins to"
affects: [02-10, phase-3-object-graph-consumers, phase-4-project-file-writer]

actuals:
  tokens: 11893
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "A table walk that never refuses: bounds itself via Region::subregion per entry and records a Defect for the item it cannot resolve, not for the whole read."
    - "A marker constant (Declaration::NAME_MARKER etc.) forces a caller that prints a field to print what the file does not hold beside it, rather than letting an absent field go unremarked."

key-files:
  created: []
  modified:
    - crates/deform6/src/vb/project.rs

key-decisions:
  - "D-07 applied: no bundled table of known Win32 declarations. A grep proves it (0 matches for FindWindowA/winapi.dat/KNOWN_APIS outside comments)."
  - "D-09 applied: the ordinal alias path (#123) is flagged inferred; a corpus-wide script found zero ordinal exports in 220 external entries, so the encoding stays unconfirmed by design."
  - "Two recoverable outcomes (an undocumented dwEntryType, a descriptor address in no section) reuse existing DefectKind variants (CountMismatch, UnmappedAddress) rather than adding a new variant to error.rs, which sits outside this plan's declared file scope. UnmappedAddress's own severity() is Fatal in error.rs; this module never consults severity() to decide whether to continue, so behaviourally the walk still treats the loss as recoverable, but the shared severity() metadata now disagrees with that treatment. Flagged as a defect below."

requirements-completed: [OBJ-05]

coverage:
  - id: D1
    description: "Every external (dwEntryType == 7) entry of the Declare table gives a library name and an export name; internal (dwEntryType == 6) entries are skipped and never dereferenced."
    requirement: "OBJ-05"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#eight_of_nine_grayscale_entries_are_external_and_the_ninth_is_skipped"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#the_corpus_file_declares_one_external_import"
        status: pass
    human_judgment: false
  - id: D2
    description: "What the file does not hold (VB-level procedure name/Alias, argument list, owning module/scope) is marked explicitly rather than invented, and the ordinal alias path is flagged inferred."
    requirement: "OBJ-05"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#the_name_marker_the_arguments_marker_and_the_scope_marker_each_name_what_is_missing"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#a_hash_then_all_decimal_digits_is_an_inferred_ordinal"
        status: pass
    human_judgment: false
  - id: D3
    description: "The external component table (OCX/type library references) is read by declared entry length, with every sub-offset resolved relative to the entry and not the table."
    requirement: "OBJ-05"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#the_one_component_server_exe_declares_resolves_all_three_strings"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/project.rs#a_synthetic_two_entry_table_proves_offsets_are_relative_to_the_entry"
        status: pass
    human_judgment: false

duration: ~50min
completed: 2026-09-08
status: complete
---

# Phase 2 Plan 06: The Declare import table and the OCX component table Summary

**DeclareTable and ComponentTable in `vb/project.rs`, reading 220 external API declarations and 3 OCX component references across the 44-program corpus, with the Alias/argument-list/scope gap marked explicitly rather than filled from a bundled table.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 3
- **Files modified:** 1 (`crates/deform6/src/vb/project.rs`)

## Accomplishments

- `DeclareTable::read` walks `ProjectInfo.lp_external_table`, resolving every `dwEntryType == 7` entry to a `Declaration { library, export }` and silently skipping every `dwEntryType == 6` entry (measured: 29 of 249 entries in this corpus).
- `Declaration`/`ExportName` mark the three things `STRUCTURES.md` section 7.2 records as not surviving compilation (procedure name and `Alias`, argument list, owning module and `Public`/`Private`) via named constants, and implement the `"#123"` ordinal-alias rule flagged `OrdinalInferred`, since no sample anywhere confirms the encoding.
- `ComponentTable::read` walks `VbHeader.lp_external_table`/`w_external_count` (a different pointer and count from the Declare table, on a different structure), reading a self-describing, variable-length entry by its own declared `StructLength`, with every string offset resolved relative to the entry's own start.
- A doc comment at the top of the Declare table's section states plainly that this format holds two unrelated tables both called "external," and names which pointer, which count, and which structure each one is read from.

## Task Commits

Each task was committed atomically:

1. **Task 1: The Declare import table, with the internal entries skipped and not dereferenced** - `d38a3c4`
2. **Task 2: What does not survive, named as missing rather than filled in** - `9930ab9`
3. **Task 3: The component table, self-describing, with one sample and a doc comment that says so** - `b1ac7e6`

**Plan metadata:** committed alongside this SUMMARY.

## Files Created/Modified

- `crates/deform6/src/vb/project.rs` - Added `DeclareTable`, `Declaration`, `ExportName`, `parse_export_name`, `DeclareDescriptorFailure`, `ComponentTable`, `Component`, and their reader/helper functions, plus 18 new tests (12 → 30 in this file).

## Decisions Made

- No bundled table of known Win32 declarations (`winapi.dat`-style) was added. `grep -rn -vE '^\s*//' crates/deform6/src/ | grep -cE 'FindWindowA|winapi\.dat|KNOWN_APIS'` returns 0.
- The ordinal alias path is implemented per `STRUCTURES.md` section 7.2's rule (`#` followed by an all-decimal tail) and flagged `OrdinalInferred` unconditionally, per D-09, because a corpus-wide script found zero ordinal exports.
- Reused existing `DefectKind` variants (`CountMismatch`, `ImplausibleCount`, `UnmappedAddress`, `NoNulTerminator`) for every new recoverable outcome rather than adding new variants to `error.rs`. See "Defects found in the plan" below for the one place this reuse is imperfect.
- Built an independent synthetic PE image builder in `project.rs`'s own test module for the component table's two-entry proof, rather than reusing `vb/header.rs`'s `a_synthetic_vb_image` (private to that module's test block, not reachable from here — see defects below).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2/3 - closest-available-type reuse] Reused `DefectKind::CountMismatch` and `DefectKind::UnmappedAddress` for defect shapes the plan implies but `error.rs` does not literally provide**
- **Found during:** Task 1 (an undocumented `dwEntryType`, a descriptor address in no section) and Task 3 (a zero or overrunning `StructLength`)
- **Issue:** The plan calls for "a recoverable defect naming the value and the offset" for an unrecognised `dwEntryType`, and "a recoverable defect" for a descriptor address in no section, a zero `StructLength`, and an overrunning `StructLength`. `error.rs`'s `DefectKind` enum (not in this plan's file scope) has no variant shaped for "an enum discriminant holds an undocumented value" or "one item in an array resolves nowhere, but the rest of the array still stands" — its closest textual fit, `UnmappedAddress`, is hard-coded `Severity::Fatal`, written for a spine pointer whose loss means nothing downstream resolves.
- **Fix:** `CountMismatch` (already `Recoverable`) is reused for the undocumented-`dwEntryType` case and the zero-`StructLength` case, with `count`/`expected`/`other_field` repurposed to name the value/minimum and the offset. `ImplausibleCount` (already `Recoverable`) is reused, correctly, for the count-bound and overrun-length cases. `UnmappedAddress` is reused for the descriptor-in-no-section case; this module never calls `.severity()` on any defect to decide whether to continue (matching this file's existing `count_defects` precedent), so behaviourally every one of these outcomes is recoverable exactly as the plan requires — but a caller that later feeds this defect through `Journal::record` and consults `severity()` would see `Fatal` and refuse, which is not what happened here.
- **Files modified:** `crates/deform6/src/vb/project.rs` only, by design (see below).
- **Verification:** Every reused-defect test asserts the specific value, offset, and (where the variant's own severity happens to be `Recoverable`) `Severity::Recoverable`; the `UnmappedAddress` tests assert the value and offset only, not severity, since that would assert a false fact.
- **Committed in:** `d38a3c4` (Task 1), `b1ac7e6` (Task 3)

**2. [Rule 2 - closest-available-fixture reuse] Built an independent synthetic PE builder rather than importing `vb/header.rs`'s**
- **Found during:** Task 3
- **Issue:** The plan's action text says to "build the images with the synthetic builder `vb/header.rs`'s test module already holds." That function (`a_synthetic_vb_image`) is a private `fn` inside a private (non-`pub`) `#[cfg(test)] mod tests` in `header.rs`, and Rust's visibility rules make it unreachable from `project.rs`'s own test module. Reaching it would require adding `pub(crate)` visibility inside `header.rs`, which is outside this plan's `files_modified` (and outside the file set the orchestrator scoped to this plan).
- **Fix:** Wrote an independent, smaller synthetic PE builder (`a_synthetic_pe_image`) local to `project.rs`'s own test module, with the same load-bearing property (RVA differs from file offset by `0xC00`) that makes the shared trap ("both corpus files put the import directory where RVA equals file offset") unreachable. This also matches `AGENTS.md`'s "build the state a test needs inside the test" more literally than importing a fixture across module boundaries would have.
- **Files modified:** `crates/deform6/src/vb/project.rs` only.
- **Verification:** `a_synthetic_two_entry_table_proves_offsets_are_relative_to_the_entry`, `a_declared_length_of_zero_stops_the_walk_with_a_recoverable_defect`, and `a_declared_length_past_the_end_of_the_region_stops_the_walk_with_a_recoverable_defect` all pass against this independent builder.
- **Committed in:** `b1ac7e6` (Task 3)

---

**Total deviations:** 2 auto-fixed (both Rule 2/3 shaped: closest-available-primitive reuse, forced by the plan's own file-scope restriction to `crates/deform6/src/vb/project.rs`)
**Impact on plan:** Both deviations are necessary consequences of the plan restricting this executor to one file while asking for defect shapes and a test fixture that live (or would need to live) in other files. No scope creep; no behaviour was invented beyond what each task's `<behavior>` list specified.

## Defects Found in the Plan

1. **The plan asks for defect shapes `error.rs`'s `DefectKind` does not literally provide, and `error.rs` is outside every phase-2 plan's declared file set.** Plan 02-01 (running in parallel, not read by this executor beyond its frontmatter) independently states "one `DefectKind::UnmappedAddress` at `Severity::Recoverable` is on `defects`" for an unrelated case (a name pointer resolving nowhere in the object array) — but `error.rs`'s actual `UnmappedAddress` variant is `Severity::Fatal`, written for a spine pointer. Both plans appear to assume a richer, per-context severity that `error.rs` does not implement, and neither plan grants its executor write access to `error.rs` to add it. This is a cross-cutting infrastructure gap, not a per-plan mistake, and it likely needs a dedicated task (or a widened file scope on one plan) to add a `Recoverable`-severity variant shaped for "one item in an array or table could not be resolved, but the array itself still stands," distinct from the spine-pointer `UnmappedAddress`.
2. **The plan's Task 3 action text points at a test helper (`vb/header.rs`'s `a_synthetic_vb_image`) that is not actually reachable from `project.rs`.** It is a private function inside a private test module. Either the plan intended `header.rs` to expose it (outside this plan's file scope, and outside `header.rs`'s own listed owner in this phase, plan 02-01, which does not touch `header.rs` either) or the plan intended each module to build its own equivalent fixture, which is what this executor did. Worth confirming which was intended before another plan hits the same wall.
3. **ROADMAP success criterion 5's original wording ("the library name, the alias and the argument list") was already corrected by the user before this plan ran** (see the plan's own header). Confirmed by measurement: no corpus program's `Declare` table entry holds an alias distinct from the export name, or an argument list, so the corrected criterion (library name, export name, plus an explicit missing-marker for what is not held) is what this plan implements. No further correction needed; recorded here only because the plan's own `<user_decisions>` table already flags D-07/D-09 as the resolution, and this SUMMARY confirms both held against the real corpus.

## Issues Encountered

None beyond the two deviations above. Both deliberate-breakage pairs for Task 1 and Task 3, and the single deliberate breakage for Task 2, were run, observed to fail, and reverted before their commit — see "Deliberate breakages" below.

## Deliberate Breakages

Per `AGENTS.md`'s "when you add a test, break the thing it covers on purpose and watch it fail," each pair below was run against the exact code that was about to be committed, the failure was recorded, and the code was restored before running the gate again.

**Task 1, break 1 — every entry processed regardless of type (`6 | 7 =>` instead of a separate skip for `6`):**
```
test vb::project::tests::eight_of_nine_grayscale_entries_are_external_and_the_ninth_is_skipped ... FAILED
  left: [("#=ûüú\u{a0}h\u{10}§8\u{8}", ""), ("gdi32", "StretchDIBits"), ...]
 right: [("gdi32", "StretchDIBits"), ...]
```
The ninth (type-6) entry's descriptor was dereferenced as a library/export pair and produced garbage bytes from runtime scratch data, exactly the information-disclosure threat T-02-25 describes. Restored; the ordered-list test passed again.

**Task 1, break 2 — entry stride changed from 8 bytes to 4:**
```
test vb::project::tests::eight_of_nine_grayscale_entries_are_external_and_the_ninth_is_skipped ... FAILED
  left: [("gdi32", "StretchDIBits"), ("gdi32", "GetDIBits"), ("gdi32", "SetStretchBltMode"), ("gdi32", "GetObjectA")]
 right: [... all eight pairs ...]
```
The walk read half of each real entry as if it were two overlapping half-entries, and the list truncated to 4 recovered pairs before running out of correctly-shaped data. Restored; the ordered-list test passed again.

**Task 2, break — the ordinal parse accepts a trailing non-digit (stops at the first non-digit instead of requiring the whole tail to be decimal):**
```
test vb::project::tests::a_hash_then_a_non_decimal_tail_is_a_plain_name_and_not_an_ordinal ... FAILED
  left: OrdinalInferred(12)
 right: Name("#12a")
```
`"#12a"` was turned into ordinal 12, exactly the lazy-parse bug the plan's action text warns against. Restored; the test passed again.

**Task 3, break 1 — string offsets resolved relative to the table rather than to the entry:**
```
test vb::project::tests::a_synthetic_two_entry_table_proves_offsets_are_relative_to_the_entry ... FAILED
  left: "First.ocx"
 right: "Second.ocx"
```
The first entry (at cursor 0) still resolved correctly by coincidence — table-relative and entry-relative addressing agree when the entry starts at offset 0 — and the second entry's strings silently reused the first entry's string pool instead of its own, exactly the shape the plan's action text predicts and exactly why a one-entry corpus sample cannot catch this. Restored; both entries resolved correctly again.

**Task 3, break 2 — the zero-length guard removed:**
```
test vb::project::tests::a_declared_length_of_zero_stops_the_walk_with_a_recoverable_defect ... FAILED
  left: 3
 right: 1
```
With the guard removed, the walk did **not** hang: `w_external_count` is a bounded `u16` loop count (3, in this fixture), so the loop terminated after 3 iterations regardless. But it produced three defects at the same standing-still cursor instead of stopping cleanly at the first one, which is the finding this deliberate breakage surfaces: on a hostile file with `w_external_count` near its `u16` maximum (65535), the same non-progress would repeat up to 65535 times rather than hang forever, but it would still waste the whole count doing nothing useful instead of stopping at the first defect. Restored; the walk again stops at one defect.

## Next Phase Readiness

- `ProjectInfo.lp_external_table`/`dw_external_count` and `VbHeader.lp_external_table`/`w_external_count` both now have a caller. Neither field is unused past this plan.
- `Component.library` (the `SourceOffset` string, e.g. `"MSWinsockLib.Winsock"`) is the join key Phase 3 needs to resolve a form's external control to a CLSID, and it is carried verbatim and untested against a second real sample — only 3 of the 44 corpus programs exercise this table, and all 3 name the same one control (`MSWinsockLib.Winsock`). Phase 3 should not assume the join key is unique or well-formed beyond what this one control proves.
- `Declaration::export` being `ExportName::OrdinalInferred` is implemented but has never been exercised by a real file. The first real `Alias "#123"` sample this project encounters should get a dedicated regression test.
- The cross-cutting `error.rs` gap (defect 1 above) should be resolved before further phase-2 plans reuse the same imprecise-severity workaround; a small number of independent plans making the same choice compounds the mismatch between what a defect's `Severity` claims and what its caller actually does.

---
*Phase: 02-the-object-graph*
*Completed: 2026-09-08*

## Self-Check: PASSED

- FOUND: crates/deform6/src/vb/project.rs
- FOUND: commit d38a3c4 (Read the Declare import table and skip the internal entries)
- FOUND: commit 9930ab9 (Mark the parts of a Declare statement the file does not hold)
- FOUND: commit b1ac7e6 (Read the external component table entry by declared length)
