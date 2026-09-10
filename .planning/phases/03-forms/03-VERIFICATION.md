---
phase: 03-forms
verified: 2026-09-10T00:00:00Z
status: gaps_found
score: 3/6 truths verified
covered_files: [".planning/REQUIREMENTS.md", ".planning/WINDOWS.md", ".planning/phases/03-forms/03-01-PLAN.md", ".planning/phases/03-forms/03-01-SUMMARY.md", ".planning/phases/03-forms/03-02-PLAN.md", ".planning/phases/03-forms/03-02-SUMMARY.md", ".planning/phases/03-forms/03-03-PLAN.md", ".planning/phases/03-forms/03-03-SUMMARY.md", ".planning/phases/03-forms/03-04-PLAN.md", ".planning/phases/03-forms/03-04-SUMMARY.md", ".planning/phases/03-forms/03-05-PLAN.md", ".planning/phases/03-forms/03-05-SUMMARY.md", ".planning/phases/03-forms/03-06-PLAN.md", ".planning/phases/03-forms/03-06-SUMMARY.md", ".planning/phases/03-forms/03-07-PLAN.md", ".planning/phases/03-forms/03-07-SUMMARY.md", ".planning/phases/03-forms/03-08-PLAN.md", ".planning/phases/03-forms/03-08-SUMMARY.md", ".planning/phases/03-forms/03-09-PLAN.md", ".planning/phases/03-forms/03-09-SUMMARY.md", ".planning/phases/03-forms/03-10-PLAN.md", ".planning/phases/03-forms/03-10-SUMMARY.md", ".planning/phases/03-forms/03-CONTEXT.md", ".planning/phases/03-forms/03-RESEARCH.md", ".planning/phases/03-forms/03-REVIEW.md", "crates/deform6-cli/src/main.rs", "crates/deform6/src/error.rs", "crates/deform6/src/vb/controlinfo.rs", "crates/deform6/src/vb/controltree.rs", "crates/deform6/src/vb/frx.rs", "crates/deform6/src/vb/gui.rs", "crates/deform6/src/vb/mod.rs", "crates/deform6/src/vb/ocx.rs", "crates/deform6/src/vb/opcodes.rs", "crates/deform6/src/vb/project.rs", "crates/deform6/src/vb/propstream.rs", "crates/deform6/src/vb/vbstr.rs", "crates/xtask/src/opcode_table.rs", "tests/ratios.toml"]
covered_digest: "v1:sha256:bc94b88a004fd29b51f34e0b50f32cb4adcc2ee2f2465ddb125f8c47c50738e2"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "`deform6 inspect` recovers the event handler names bound to each control."
    status: failed
    reason: "Zero event handler names are recovered anywhere in the corpus, by design decision 03-CONTEXT.md D-02. `EventNameTable` ships with no entries and has no run-time loader in this phase. Every event slot in a live `inspect` run prints 'not decoded' regardless of whether the slot is bound. This is the literal opposite of the phase goal's own closing clause."
    artifacts:
      - path: "crates/deform6/src/vb/controlinfo.rs"
        issue: "EventNameTable::default() carries zero entries; no caller in this phase supplies a populated table"
    missing:
      - "A shipped or loadable event name source (with a lawful, cited provenance) so at least one control type's event ordinals resolve to real names, or an explicit ROADMAP amendment narrowing SC/goal to event structure only"
  - truth: "`deform6 inspect` recovers the resource blobs."
    status: failed
    reason: "`frx::extract_blob` exists and is unit-tested in isolation, but is never called from `compose_form`/`compose_control` in `vb/mod.rs`. `propstream.rs`'s `PayloadType::Picture` arm always returns `PropertyValue::Undecoded` and never invokes `extract_blob`. A live `inspect` run therefore never surfaces a resource blob for any control; the recovery path is unwired dead code from the product's point of view."
    artifacts:
      - path: "crates/deform6/src/vb/propstream.rs"
        issue: "PayloadType::Picture always reports Undecoded; extract_blob is never reached"
      - path: "crates/deform6/src/vb/mod.rs"
        issue: "compose_form/compose_control never reference vb::frx"
    missing:
      - "Wire walk_properties's Picture arm to frx::extract_blob and surface the recovered blob (or its honest refusal) in FormReport/ControlReport and the CLI printer"
  - truth: "A form that uses a third-party OCX reports the control's CLSID (SC3)."
    status: failed
    reason: "The value reported is mechanically produced by matching the class name against the external component table's GUIDoffset/GUIDlength field, but that value (2C49F800-C2DD-11CF-9AD6-0080C7E7B78D, confirmed by a live run against SubReality_WinsockSample.exe) is not the CLSID a user, a registry lookup, or the corpus's own .vbp Object= line would recognize (248DD890-BB45-11CF-9ABC-0080C7E7B78D) for MSWinsockLib.Winsock. 03-08-SUMMARY.md documents this discrepancy as an unresolved deviation from the plan's own asserted ground truth, and notes a near-miss binary GUID (oUuid, 248DD896-...) sits much closer to the true value but is not what the shipped code reads. No later plan reconciled this."
    artifacts:
      - path: "crates/deform6/src/vb/project.rs"
        issue: "Component::guid_text is decoded from GUIDoffset/GUIDlength, a field this session's own measurement shows does not match the control's real, registry-recognized CLSID"
      - path: "crates/deform6/src/vb/ocx.rs"
        issue: "join_component/Clsid propagate the mismatched value with no cross-check against oUuid or any other ground truth"
    missing:
      - "Reconcile which component-table field actually holds the CoClass CLSID (oUuid vs GUIDoffset/GUIDlength) against the .vbp Object= line for all 3 corpus MSWinsockLib.Winsock instances, or report the value with an honest caveat that it does not match the registered CLSID"
  - truth: "`inspect` prints the control tree of every corpus form (SC1's 'every')."
    status: partial
    reason: "49 of 53 corpus forms are recovered; 4 forms (HexScroll, PassGen, UUID2, Map Editor, one each) refuse honestly via the MAX_UNEXPLAINED_TAIL bound rather than emit a wrong tree, because the scope-byte grammar for closing out of a two-level-deep menu is not yet found (WINDOWS.md finding 7, open). The refusal discipline itself is correct per SC1's own third sentence, but the phase goal's literal 'every form' is not met for these 4."
    artifacts:
      - path: "crates/deform6/src/vb/controltree.rs"
        issue: "close_walk's MAX_UNEXPLAINED_TAIL bound is a defensive workaround, not a resolved grammar rule, for the two-level menu-close case"
    missing:
      - "The true scope-separator grammar rule for closing a menu nested two levels deep back to a form-level sibling menu, via the same byte-level corpus research 03-04 did for the single-level case"
deferred:
  - truth: "The `.frx` file writer emits blob bytes to disk with offsets the generated `.frm` agrees with."
    addressed_in: "Phase 4 (plan 04-04)"
    evidence: "ROADMAP.md Phase 4 named risks: 'The .frm writer and the .frx writer are therefore one component ... Plan 04-04 owns both.' 03-07-SUMMARY.md: 'This plan writes no file ... phase 4's plan 04-04 owns the .frx writer.'"
human_verification: []
---

# Phase 3: Forms Verification Report

**Phase Goal:** `deform6 inspect` reports the control tree of every form with the
correct parent for each control, the type and the name of each control, the
property values of each control and of the form, the CLSID of each third-party
OCX control, the resource blobs, and the event handler names bound to each
control.

**Verified:** 2026-09-10
**Status:** gaps_found
**Re-verification:** No — initial verification

## Full Gate (run live, not trusted from SUMMARY)

| Check | Command | Result |
| ----- | ------- | ------ |
| Format | `cargo fmt --all --check` | PASS (no output, exit 0) |
| Lint | `cargo clippy --all-targets -- -D warnings` | PASS (finished clean) |
| Tests | `cargo test --workspace` | PASS — every suite green: 384, 2, 21, 1, 32, 9, 43, 2, 0, 26, 44, 0 passed; 0 failed anywhere |

The gate itself is genuinely green. The gaps below are goal-achievement gaps,
not gate failures: the code compiles, lints clean, and every existing test
passes, but several things the phase goal and ROADMAP.md's success criteria
require are either not built, not wired into the product path, or wrong when
checked against real corpus bytes.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Control tree of every form, correct parent, never an unproven tree | ⚠️ PARTIAL | 49/53 forms recovered (`tests/ratios.toml`); tiling check holds on all 44 programs; the 4 unrecovered forms refuse honestly (see Q2 below), but "every form" is literally not met |
| 2 | Type and name of each control | ✓ VERIFIED | Live `deform6 inspect` run on `LockWorkStation.exe` and `SubReality_WinsockSample.exe` prints `cType`-derived control kind and Latin-1 name for every node in the tree |
| 3 | Property values of each control and of the form | ✓ VERIFIED | Live run shows decoded values (`WindowState = 1`, `BorderStyle = 0`) for opcodes in the 53-entry safe-provenance table, and an honest "not decoded" with byte offset for opcodes outside it — matches SC4's cursor-discipline requirement |
| 4 | CLSID of each third-party OCX control | ✗ FAILED | Live run reports `CLSID = {2C49F800-C2DD-11CF-9AD6-0080C7E7B78D}` for `MSWinsockLib.Winsock`, not the `{248DD890-BB45-11CF-9ABC-0080C7E7B78D}` on the corpus `.vbp`'s own `Object=` line — see Q3 below |
| 5 | The resource blobs | ✗ FAILED | `frx::extract_blob` is never called outside its own unit tests; `propstream.rs`'s `Picture` arm always returns `Undecoded` — see Q7 below |
| 6 | Event handler names bound to each control | ✗ FAILED | `EventNameTable` ships zero entries by design (D-02); every event slot in every live run reports "not decoded" — see Q1 below |

**Score:** 3/6 truths fully verified (2 fully failed as hard misses, 1 failed
as a correctness bug, 1 partial). VER-06 (frmHMM.frx exclusion) is verified
separately below and is not one of the six goal-level truths.

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/deform6/src/vb/gui.rs` | GUI table walk | ✓ VERIFIED | Compiles, tested, wired into `compose_form` |
| `crates/deform6/src/vb/controltree.rs` | Scope-byte control tree walk | ✓ VERIFIED, ⚠️ partial coverage | Wired; MAX_UNEXPLAINED_TAIL workaround documented as open gap |
| `crates/deform6/src/vb/vbstr.rs` | String reads landing on declared end | ✓ VERIFIED | Wired via `propstream.rs` |
| `crates/deform6/src/vb/propstream.rs` | Property value decoding | ✓ VERIFIED (partial by design) | Wired; honest `Undecoded` for out-of-table opcodes and for `Picture` |
| `crates/deform6/src/vb/opcodes.rs` | Safe-provenance opcode table | ✓ VERIFIED | 53-entry builtin subset, wired |
| `crates/deform6/src/vb/frx.rs` | Resource blob extraction | ⚠️ ORPHANED | Exists, substantive, unit-tested, but not called from any production path (`compose_form`/`compose_control`/`propstream.rs`) |
| `crates/deform6/src/vb/ocx.rs` | External control class name + CLSID join | ✓ VERIFIED (wired), ✗ value incorrect | Wired and printed, but the CLSID value does not match the control's real, recognizable identity |
| `crates/deform6/src/vb/controlinfo.rs` | ControlInfo, event table, EventNameTable | ✓ VERIFIED (structure), ✗ FAILED (names) | Event structure (index, bound state) recovered and wired; names never recovered by design |
| `crates/deform6/src/vb/project.rs` | ComponentTable, Component::guid_text | ✓ VERIFIED (wired), ✗ value incorrect | Decodes GUIDoffset/GUIDlength as documented, but that field does not hold the registry CLSID |
| `crates/deform6/src/vb/mod.rs` | `compose_form`/`compose_control`, `Report.forms` | ✓ VERIFIED | Composes GUI table, control tree, property stream, external control, event slots — but never `vb::frx` |
| `crates/deform6-cli/src/main.rs` | CLI printer | ✓ VERIFIED | Prints tree, properties, CLSID, event slots (all confirmed by live run); no blob printing exists because nothing produces one |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `propstream.rs` (Picture opcode) | `frx.rs::extract_blob` | direct call | ✗ NOT_WIRED | Picture always short-circuits to `Undecoded`; `extract_blob` has zero production call sites |
| `vb/mod.rs::compose_control` | `ocx.rs::join_component` | CLSID join | ✓ WIRED | Confirmed live; value is wired but factually wrong (see truth 4) |
| `vb/mod.rs::compose_control` | `controlinfo.rs::report_events` | event slot join by name | ✓ WIRED | Confirmed live; structure correct, names always absent |
| CLI `main.rs::print_forms` | `Report.forms` | direct field access | ✓ WIRED | Confirmed live |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ------------ | ------ | -------- |
| FRM-01 | 03-01, 03-04, 03-10 | Control tree, correct parent | ⚠️ PARTIAL | 49/53 forms; REQUIREMENTS.md marks `[x]` — overstated given the "every" wording |
| FRM-02 | 03-04, 03-10 | Type and name of every control | ✓ SATISFIED | Live-confirmed |
| FRM-03 | 03-02, 03-05, 03-06, 03-10 | Property values of control and form | ✓ SATISFIED | Honest partial-decode design matches SC4 |
| FRM-04 | 03-08, 03-10 | CLSID of third-party OCX, honest blob caveat | ✗ BLOCKED | Blob caveat text is present and correct; the CLSID value itself is wrong against ground truth. REQUIREMENTS.md marks `[x]` — overstated |
| FRM-05 | 03-07, 03-10 | Resource blobs recovered, `.frx` written | ✗ BLOCKED | Neither half holds in phase 3: extraction unwired, writer explicitly deferred to phase 4. REQUIREMENTS.md marks `[x]` — overstated |
| FRM-06 | 03-09, 03-10 | Event handler names | ✗ BLOCKED | Zero names recovered, by design (D-02). REQUIREMENTS.md marks `[x]` — overstated |
| VER-06 | 03-03, 03-10 | frmHMM.frx exclusion named, never silent | ✓ SATISFIED | `the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect` passes; deliberate-breakage test proves the gate is real, not tautological |

No orphaned requirement IDs: every ID in ROADMAP.md's Phase 3 requirements
list (FRM-01..06, VER-06) appears in at least one plan's `requirements:`
frontmatter, and 03-10 additionally re-declares all seven as the
composition/gate plan.

**REQUIREMENTS.md itself marks FRM-01, FRM-04, FRM-05 and FRM-06 as `[x]`
Complete. Three of those four checkmarks are not supported by the code as it
stands** (FRM-04's CLSID value, FRM-05's blob wiring and writer, FRM-06's
event names); FRM-01 is close but not literally "every form." This is a
requirements-tracking discrepancy, not just an implementation gap — the
project's own source of truth overstates what phase 3 delivered.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `crates/deform6/src/vb/controltree.rs` | 18, 21, 412-413, 528, 583, 666, 803, 1088-1089 | 10 em-dashes | ℹ️ Info | Violates AGENTS.md "Do not use an em-dash," already caught by 03-REVIEW.md IN-01 |
| `crates/deform6/src/vb/gui.rs`, `controltree.rs`, `frx.rs` | triplicated `damaged()` | ℹ️ Info | 03-REVIEW.md WR-01: unbounded leak under a fuzzing host that does not yet exist (see Q4) |
| `crates/deform6/src/vb/controltree.rs` | 631-658 | dead pop-bounding code in `close_walk` | ⚠️ Warning | 03-REVIEW.md WR-02: reads as a safety check it does not perform |
| `crates/deform6/tests/ratios.rs` | 362-365 | `format_ratio` divides with no zero guard | ⚠️ Warning | 03-REVIEW.md WR-03: would silently write `NaN` for a future zero-declared program |

No `TBD`/`FIXME`/`XXX` debt markers found in any file this phase touched
(checked directly with grep across all 12 reviewed `src/` files plus
`ratios.toml`).

## Answers to the Specific Questions

### 1. Event handler names

**Not met.** The phase goal's closing clause, "the event handler names bound
to each control," is not achieved. `EventNameTable` (`controlinfo.rs`) ships
with zero entries per `03-CONTEXT.md` D-02, and no plan in this phase gives
it a run-time loader. A live `deform6 inspect` run against every corpus
program that has events (confirmed on `LockWorkStation.exe`) prints every
slot as `event slot N: bound/unbound, not decoded. Run with
--event-name-table to supply one.` — never a name. What IS recovered is
event *structure*: which slot index is bound, whether it is bound or
unbound, and the handler's native stub address. That is a real, tested,
independently-cross-checked capability, but it is not the capability the
goal names. This is a deliberate scope limit (an explicit legal decision not
to derive a name table from `VB6.OLB`), correctly documented as such, and it
should not be counted as goal achievement. **FAILED**, not partial: 0 of N
corpus event names recovered, by design, with no seam left this phase that
would change that number without a policy change.

### 2. The form ratio (49 of 53)

The 4 unrecovered forms are **HexScroll** (1 of 2), **PassGen** (1 of 4),
**UUID2** (1 of 2), and **Map Editor** (1 of 2) — confirmed directly from
`tests/ratios.toml`'s pinned `form_declared`/`form_recovered` pairs. This is
a **refusal, not a failure**: `controltree.rs::close_walk` hits
`MAX_UNEXPLAINED_TAIL` (8 bytes) when it cannot account for a trailing span
while closing out of a menu nested two levels deep, and returns
`Refusal::Damaged` naming the byte offset and count, rather than emitting a
tree missing the sibling menu and its 3 children silently. `WINDOWS.md`
finding 7 records this as an open, named gap (the true scope-separator
grammar for this transition is not found), not a hidden defect. The
differential gate (`support::frm`, an independent second reader) is what
caught this during plan 03-10 in the first place — the honesty discipline
worked as designed. However, success criterion 1's literal text ("prints the
control tree of every corpus form") is still not met for these 4 forms:
"never prints a tree it cannot prove" is satisfied, "every form" is not.
Both halves of SC1 matter; only one holds.

### 3. The OCX CLSID field

**Success criterion 3 is not actually met**, though it appears to be at
first read. The mechanism SC3 names — "resolved by matching the class name
against the external component table" — is implemented and does execute:
`join_component` matches `ExternalControl.class_name` against
`Component::library`, exactly as SC3 describes. But the *value* that
mechanism produces is wrong. A live run against
`SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe` prints `CLSID =
{2C49F800-C2DD-11CF-9AD6-0080C7E7B78D}`. That is not a CLSID any user,
registry lookup, or the corpus's own `.vbp` file would recognize for
`MSWinsockLib.Winsock`: `SK-Winsock-Sample__VB6`'s `.vbp` declares `Object=
{248DD890-BB45-11CF-9ABC-0080C7E7B78D}#1.1#0`. `03-08-SUMMARY.md` documents
this exact discrepancy as an unresolved measurement: the field the plan
named to decode (`GUIDoffset`/`GUIDlength`) gives `2c49f800-...`, while a
different field at the same component table entry (`oUuid`, a 16-byte binary
GUID) gives `248DD896-BB45-11CF-9ABC-0080C7E7B78D` — differing from the true
value by only the low byte of `Data1`, i.e. a near-miss that the shipped
code does not use. No later plan (03-10 included) revisited this. **This
looks like the wrong field was chosen to decode, not like an inherent
ambiguity in the format.** A user who trusts this CLSID for anything (e.g.
re-registering the control) would get a value that does not identify
`MSWinsockLib.Winsock`. Criterion 3 is FAILED on the value, even though its
own described mechanism is present and wired.

### 4. The fuzzing gate

**Confirmed: there is no fuzz target anywhere in this repository, and the
stated gate is not in force.** `find` and `grep` across the whole tree
(excluding `target/`) found zero files or directories matching `fuzz*`.
`Cargo.toml`'s workspace `exclude = ["crates/deform6/fuzz"]` line references
a path that has never existed in this repository's git history (`git log
--all -- crates/deform6/fuzz` returns nothing, and no commit has ever
deleted a fuzz directory). This is not a partial implementation with a
missing corpus — it is an unwritten exclude entry pointing at nothing.
`AGENTS.md` states "Fuzzing is part of the gate, not an extra" and "A crash
that the fuzzer finds becomes a test case in the repository," and
03-REVIEW.md's own WR-01 explicitly reasons about "a fuzz target (or any
long-running host that embeds this library)" as something that does not yet
exist, corroborating this from the review side. Given the repeated hostile-
input discipline this phase otherwise demonstrates (checked arithmetic, no
panics, bounded loops — all independently verified in this session's review
of the gate output), the absence of any fuzzing harness is the single
largest gap between AGENTS.md's stated process guarantees and what is
actually enforced today.

### 5. STRUCTURES gaps 11-15

- **Gap 11** (control array index location): **CLOSED**, 2026-09-10, per
  `STRUCTURES.md` §14 and `03-04-SUMMARY.md`. The array `Index` is the
  2-byte value at control block offset `0x05`, measured across 30 array
  elements in 2 files. This is a genuine resolution, not a workaround.
- **Gap 12** (`uni` byte at control-block +0x02): **remains fully open**.
  Not referenced by that name anywhere in `src/`; no code reads or relies on
  it. `STRUCTURES.md`'s gap register still lists it unresolved.
- **Gap 13** (string encoding rule, §9.3): **worked around, not closed.**
  `VbStr` takes an explicit encoding when the property table supplies one,
  defaults to ASCII otherwise, validates the landing point, retries once as
  the other encoding, and refuses rather than guesses — exactly the ROADMAP
  named-risk mitigation. This is a durable, principled workaround (the
  refusal-on-doubt discipline), but the underlying research question (which
  rule actually governs encoding) is not answered; `STRUCTURES.md`'s
  register still lists gap 13 open.
- **Gap 14** (scope separator grammar, §8.9): **worked around for the
  single-level case (closed via corpus measurement, 03-04), still open for
  the two-level-deep menu-close case** (`WINDOWS.md` finding 7,
  `controltree.rs`'s `MAX_UNEXPLAINED_TAIL`). The tiling-check-as-gate
  strategy the ROADMAP named risk explicitly prescribes is what converts the
  unresolved grammar into an honest refusal instead of a silent wrong tree —
  correct process, incomplete research. `STRUCTURES.md`'s register entry for
  gap 14 is not marked closed.
- **Gap 15** (opcode-to-property table): **worked around by design, not
  closed, and not intended to close within legal constraints.** `D-01`
  authorizes a small, cited "safe-provenance subset" (53 entries,
  `OpcodeTable::builtin`), never a `VB6.OLB` dump. Every opcode outside that
  subset reports `Undecoded` with its offset and opcode number, honestly.
  This is the correct, deliberate boundary the roadmap itself anticipated
  ("this is a data build job, not a parsing job"), not a phase-3 shortfall.

Net: 1 of 5 gaps (11) is genuinely closed. Gaps 13, 14 and 15 are handled by
principled, tested refusal/subset mechanisms that keep the tool honest, which
is the right engineering response, but the underlying research questions
these gaps name are still open. Gap 12 has no workaround and no closure —
it is simply unexercised.

### 6. Research drift in 03-RESEARCH.md

**03-RESEARCH.md is not reliable and needs correction before phase 4 reads
it.** Three separate plans found and documented, in their own SUMMARY files,
that its prose does not reconcile against real corpus bytes:

- 03-04 found the scope-byte grammar's `0x02`/`0x03` symmetry 03-RESEARCH.md
  implies (and the plan's own `<behavior>` text copied) is wrong; `0x02`
  never behaves as an independent terminal the way the document assumes.
- 03-06 found section 8.3's `Length - 2` property-loop bound does not
  reconcile against `controltree.rs`'s own already-shipped `Length - 1`
  tiling accounting, and kept `Length - 1` instead.
- 03-06 also found the `-32768` position-block escape's illustrative code
  reads 18 bytes total while claiming to return 16 as its consumed count —
  an internal contradiction in the document itself, independent of any
  corpus measurement — and implemented the self-consistent 16-byte version
  instead.

**Critically, none of these three corrections were fed back into
`03-RESEARCH.md` itself.** A direct check of the document's current text
(lines 523-528) still states the `Length - 2` / `Length + 2` formulas as
confirmed-consistent for the zero-children case, with no note that plan
03-04 and 03-06 later found the general case does not follow that formula.
A reader of `03-RESEARCH.md` alone, including a phase-4 planner, would
implement the disproven `Length - 2` bound. `STRUCTURES.md` (the more
authoritative research document) does carry gap 11's closure correctly, but
`03-RESEARCH.md`'s own illustrative prose is stale. **Recommendation: before
phase 4 planning reads `03-RESEARCH.md`, amend section 8.3 and the position-
block escape passage to point at 03-04-SUMMARY.md/03-06-SUMMARY.md's
corrected figures, the same way 03-04 already amended `STRUCTURES.md` and
`GAPS.md` for gap 11 and the corpus-count corrections.**

### 7. Outstanding stubs

Both confirmed exactly as stated, and neither is counted as delivered
anywhere in this verification's scoring:

- **`windows_walk::run`** (`crates/xtask/src/opcode_table.rs`) is a
  documented `cfg(windows)` stub, never compiled on this (non-Windows) host,
  recorded as `WINDOWS.md` item 6 ("needs a human at a Windows host with a
  lawful VB6 install"), open.
- **The `.frx` file writer** does not exist. `grep` for a write function
  (`fn write`, `write_frx`, `fs::write`, `File::create`) in `frx.rs` returns
  nothing. `frx::extract_blob` recovers blob bytes and computes the running
  `.frx` cursor into memory only (`BlobCursor`), and both `03-07-SUMMARY.md`
  and `ROADMAP.md` Phase 4's own named risks explicitly assign the writer to
  phase 4 plan 04-04, because the `.frm` and `.frx` writers share one
  cursor and "cannot be split across two plans or two phases." This is
  correctly scoped to phase 4, not a phase 3 gap by itself — but see truth 5
  above: the deeper problem is that `extract_blob` is not even wired into
  phase 3's own composition path, so there is no in-memory blob for a phase
  4 writer to consume yet either.

## Gaps Summary

The gate (`cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`)
passes cleanly, and this session ran all three live rather than trusting the
SUMMARYs. Two of the phase's own control-flow requirements, FRM-01's
"every form" and FRM-04's CLSID correctness, and two whole capabilities the
phase goal names outright, FRM-05's resource blobs and FRM-06's event names,
are not delivered as the goal states them. FRM-05 and FRM-06 are honestly
documented as deliberate, principled limits in their own plan SUMMARYs (not
silently hidden), but REQUIREMENTS.md nonetheless marks all four as `[x]`
Complete, which overstates what phase 3 actually built. The fuzzing gate
AGENTS.md describes as mandatory does not exist anywhere in the repository.
`03-RESEARCH.md` carries stale, disproven formulas that three later plans
in this same phase found and worked around without correcting the source
document. None of this is a crash, a panic, or a silent wrong answer inside
the code that ships — the honesty discipline (refuse rather than guess) is
real and independently verified — but "goal achieved" is not an accurate
description of the current state.

---

_Verified: 2026-09-10_
_Verifier: Claude (gsd-verifier)_
