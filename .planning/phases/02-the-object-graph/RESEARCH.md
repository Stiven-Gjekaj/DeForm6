# Phase 2: The object graph - Research

**Researched:** 2026-09-07
**Domain:** VB6 compiled-object metadata recovery (object table, procedure
type descriptors, external `Declare` table) and the differential test harness
that measures it against the original source.
**Confidence:** HIGH on every claim tagged `[VERIFIED: local]` below (each one
is the output of a script run against all 44 corpus executables, this
session). LOW on the handful of claims that stay `[G]` after measurement,
because the corpus genuinely does not exercise them — that is reported as an
absence, not asserted as either presence or absence of the underlying
behaviour.

## Summary

Nine questions were asked. Six close, fully or partially, against the corpus.
Three stay open because the corpus does not exercise the byte pattern in
question, and no amount of re-reading closes an absence — this document says
so plainly rather than guessing.

**Closed by measurement:** the `Object` array walk (stride `0x30`, zero
resolve failures across 105 objects in 44 files); the `fObjectType` value
table (exactly three values occur — `0x18001` Module, `0x18083` Form,
`0x118003` Class — and every one of the 105 objects matches its `.vbp`
declaration one-to-one, using `Attribute VB_Name` as the join key, not the
`.vbp` filename); the `OptionalObjectInfo` presence test (`fObjectType & 2`
agrees with `ObjectInfo.lpPrivateObject != -1` on all 105 objects, zero
disagreements); `constFFFF == 0xFFFF` (193 of 193 `FuncTypDesc` records); the
`.bas` scope cap (a standard module's `Object.lpProcNamesArray` pointer is
**null**, not merely empty, in 8 of 8 corpus module objects — a stronger and
previously undocumented fact); and, unexpectedly, the `optionalVals` target
(gap 7) — two real functions in the corpus declare `Optional ... = <value>`
parameters, and their `optionalVals` pointer resolves to a small tagged-value
block whose bytes spell out the literal defaults `10000` and `50` exactly.

**Stay open, with the safe default given:** the `ParamArray` encoding (gap 4)
— zero occurrences anywhere in 44 programs' source, so the corpus cannot
confirm or refute the guessed `0x6F` encoding; fifteen unassigned type codes
(gap 5) — zero of them occur in 193 real `FuncTypDesc` records, only nine of
the ~21 documented codes appear at all; the `PubVarDesc` record stride (gap
8) — this one is worse than "unresolved," because measurement surfaced a new
problem: `cntPublicVars` does not count source-level `Public variable As
Type` declarations at all (a class with zero such declarations reports
`cntPublicVars = 4`; a form with zero reports `5`), so no stride hypothesis
was ever going to converge, and OBJ-05's public-variable claim should not be
attempted from this structure without a controlled compile-and-diff
experiment first.

**Primary recommendation:** build `ObjectTable` → `Object` array →
`ObjectInfo` → `PrivateObj` → `FuncTypDesc` exactly as `STRUCTURES.md` §4-§6
describes, with the `wTotalObjects` loop bound phase 1 already corrected.
Treat a `.bas` module's `lpProcNamesArray == 0` as a first-class case (not
merely "every entry happened to be null") and report it as a hard cap, not a
per-file anomaly. Ship the `PubVarDesc` walk as unimplemented in this phase
and record `cntPublicVars` as a raw, unexplained count in the report rather
than attempting to enumerate names from it.

## Architectural Responsibility Map

This is a single Rust library crate plus one CLI binary; there is no
browser/server tier split. The map below uses the crate's own internal
layering instead.

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| Object table / `Object` array walk | `deform6::vb` (library) | — | Pure structure reading, no I/O, matches phase 1's `vb/project.rs` pattern |
| `fObjectType` classification | `deform6::vb` (library) | — | A pure function over a `u32`, testable in isolation |
| Procedure names / `FuncTypDesc` / type codes | `deform6::vb` (library) | — | Same tier; reads through `PeImage::region_at_va` like everything else |
| External `Declare` table | `deform6::vb` (library) | — | Same tier |
| `.vbp` independent reader | `support/vbp.rs` (test-only) | — | **Must not** live in the library tier; ROADMAP and CONTEXT.md both require it call nothing in `src/` |
| Exclusion rules (VER-04) | `support/rules.rs` (test-only) | — | Same reasoning: the harness's notion of "what the compiler kept" must not be derived from the parser under test |
| Differential comparison, both directions | `tests/differential.rs` (integration test) | `support/vbp.rs`, `support/rules.rs` | Consumes both the library's output and the independent reader's output; owns neither |
| `ratios.toml` pin + `xtask update-ratios` | `crates/xtask` (new binary crate) | `tests/differential.rs` | A dev-tool binary, not part of the shipped library or CLI |
| `inspect` text report | `deform6-cli` | `deform6::vb::Report` | Presentation only, as established in phase 1 |

## User Constraints

`CONTEXT.md` for this phase is not organized under `## Decisions` / `## Claude's
Discretion` / `## Deferred Ideas` headers — it is a set of corrections and
binding rules written after phase 1 measured the corpus. Everything below is
copied over as locked, because it is measured fact plus explicit "must not"
rules, not an open design question.

### Locked (measured fact, not to be re-litigated)

- **The object array loop bound is `wTotalObjects` (offset `0x2A`), not
  `wCompiledObjects` (`0x2C`).** Measured twice independently against all 44
  corpus programs: `wTotalObjects` matches the `.vbp`-declared object count in
  44 of 44; `wCompiledObjects` matches in only 29 of 44 (it is the array
  **capacity**, rounded up). `Report::object_count()` already returns
  `wTotalObjects` as of plan 01-07 — phase 2 must not regress this.
- **A subset assertion cannot see an over-count.** `differential.rs` must
  compare both directions: every declared object is recovered, **and** every
  recovered object is declared. A one-directional assertion let a capacity
  slot with a dangling pointer through undetected for hours in the phase 1
  history this project keeps.
- **The expectation comes from the file list the `.vbp` declares, never from
  a directory glob.** Two corpus projects (`Hidden-Markov-model`,
  `Randomize-effects`) hold a `cCommonDialog.cls` the `.vbp` never lists. A
  glob counts it as a recovery failure; VER-02 forbids that.
- **Where a directory holds several `.vbp` files, select by `ExeName32`.**
  Two corpus projects need this: `SK-MCI-Sample__VB6` and
  `SK-TFTP-Sample__VB6`. See Q7 below for the exact mechanism and why a
  same-directory glob is not sufficient for either.
- **A `.vbp` value can contain a double quote.**
  `corpus/vb6-code/Sepia-effect/Sepia.vbp` holds
  `Title="Sepia / "Antique" Image Filter"`. Take everything between the
  first quote and the **last** quote on the line, never a `"([^"]*)"` regex.
- **An absent `.vbp` key is not an empty value.** One corpus project
  (`SK-Gradient-Sample__VB6`) has no `Title=` key at all — VB6 omits it when
  the title equals the project name. Fall back to `Name=`.
- **`support/vbp.rs` must not call anything in `src/`.** It is a second,
  independent reader. A harness that shares a reader agrees with a bug in
  that reader.
- **Do not guess a type code.** Fifteen are unassigned in `STRUCTURES.md`
  §6.5, and zero of them occur anywhere in the corpus (measured, Q4). An
  unknown code becomes a reported gap, not a guessed type name.
- **Do not refuse an object whose `fObjectType` is unknown.** The MDIForm
  value is in no source and does not occur in this corpus either (Q2).
  Classify it `Unknown`, flag it, carry on.
- **Do not present an inferred event name as recovered.** Out of scope for
  this phase (`EventDesc` names are a Phase 3 concern per `STRUCTURES.md`
  §6.4), noted here so a later plan does not smuggle it in early.
- **Do not discover the `.bas` cap when a ratio looks low.** State the cap in
  the report before the first ratio is pinned. Q5 below gives the exact
  numbers and a sharper mechanism than `STRUCTURES.md` documents.
- **Fix `Report`'s dropped defects (`WINDOWS.md` finding 3) and, if free,
  `runtime_dll`'s unprovable provenance (finding 4).** Carried over from
  phase 1, not new to this phase.

### Not addressed by CONTEXT.md (this document's own findings, flagged for
confirmation)

- The exact shape of `ratios.toml`'s two counts (which "item" they count) is
  not specified anywhere upstream. Q9 below gives a concrete, worked
  recommendation with real numbers and flags the ambiguity in the roadmap's
  own wording about which edit direction produces which failure message —
  see the Open Questions section.
- `PubVarDesc`'s stride cannot be determined from this corpus, and worse, the
  corpus shows `cntPublicVars` measuring something other than "declared
  public variables." This should be surfaced to the user before plan 02-05 is
  written, because OBJ-05 in the roadmap text ("recovers the `Declare`
  statements") does not actually cover public variables — re-check whether
  the roadmap intends public-variable recovery to be in scope for phase 2 at
  all, given `STRUCTURES.md`'s own gap register lists it as gap 8 with no
  closure path.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|---|---|---|
| OBJ-01 | Walk the object table and recover the name of every compiled object | Q1: stride, offsets, 105/105 resolve, zero failures |
| OBJ-02 | Tell a form, a module and a class apart | Q2: the closed three-value table, cross-checked 105/105 against `.vbp` |
| OBJ-03 | Recover public procedure names for every object | Q3, Q5: null-entry measurement and the `.bas` array-pointer-null finding |
| OBJ-04 | Recover procedure signatures: argument names, types, ByRef/Array/Optional/ParamArray | Q4: type code histogram, modifier bits, `optionalVals` closure, `ParamArray` absence |
| OBJ-05 | Recover `Declare` statements for external API calls | Q6: entry-type histogram, ordinal-usage measurement |
| OBJ-06 | Report a private procedure as private, not invent a name | Q3: null-entry semantics validated against real source (`Mandelbrot.frm`) |
| VER-01 | Differential test compares recovered output against original source | Q7, Q8: `support/vbp.rs` scope and the exclusion rules it needs |
| VER-02 | Expectation from `.vbp` file list, never a directory glob | Q7: the two orphan-`.cls` projects named |
| VER-03 | Select `.vbp` by `ExeName32` when a directory holds several | Q7: the two projects named, with the exact mechanism that resolves both |
| VER-04 | What the compiler does not keep is excluded by a written rule | Q8: the enumerated rules |
| VER-05 | A recovery ratio per program is pinned, two failure messages | Q9: `ratios.toml` shape, worked numbers, the flagged directional ambiguity |
</phase_requirements>

## Standard Stack

This phase adds exactly one dependency: `toml`, for `crates/xtask` to read
and rewrite `tests/ratios.toml`. Nothing else is new — the object graph is
read with the same `Region`/`Off`/`Rva`/`Va` primitives phase 1 already
built, and `support/vbp.rs` is hand-rolled text parsing over bytes read as
Latin-1, matching the pattern phase 1 already established for `VbHeader`
strings.

### Core

| Library | Version | Purpose | Why Standard |
|---|---|---|---|
| `toml` | `1.1.5+spec-1.1.0` `[VERIFIED: crates.io, package-legitimacy check OK, 16.3M weekly downloads, published 2014, github.com/toml-rs/toml]` | Read and write `tests/ratios.toml` from `crates/xtask` | The de facto standard Rust TOML crate; `serde`-compatible, already a transitive dependency of much of the ecosystem this workspace's other crates pull in |

### Supporting

None. `support/vbp.rs` and `support/rules.rs` are pure `std`, matching every
other module phase 1 wrote (no filesystem type crosses into `src/`; the test
harness's own `std::fs` use is the one place in the tree allowed to touch
disk, same as `deform6-cli`).

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|---|---|---|
| `toml` | `serde_yaml`, hand-rolled key=value | `STRUCTURES.md`-style TOML is what the roadmap names explicitly (`tests/ratios.toml`); no reason to introduce a second format |
| `toml` | `toml_edit` | `toml_edit` preserves formatting/comments on rewrite, which `xtask update-ratios` might want (a stable diff when only the numbers move). Worth a second look when 02-09 is planned, but `toml` is sufficient for read + rewrite-whole-file and is one dependency instead of two if serde derives are also wanted |

**Installation:**
```bash
cargo add --package xtask toml
```

**Version verification:** `cargo search toml` was run against the live
crates.io index this session: `toml = "1.1.5+spec-1.1.0"`. The
`package-legitimacy check` seam confirms: published 2014-11-11, 16,296,058
weekly downloads, repo `github.com/toml-rs/toml`, not deprecated, no
`postinstall`-equivalent concern (Rust has no install-time script analog).

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---|---|---|---|---|---|---|
| `toml` | crates.io | ~11 years (since 2014-11-11) | 16.3M/week | github.com/toml-rs/toml | OK | Approved |

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none.

## Architecture Patterns

### System Architecture Diagram

```
 corpus/**/*.exe (untrusted bytes)
        |
        v
 PeImage::parse  ------------------------------ [Phase 1, unchanged]
        |
        v
 vb::header::header_region + VbHeader  -------- [Phase 1, unchanged]
        |  lpProjectData (Va)
        v
 vb::project::ProjectInfo::read  --------------- [Phase 1, unchanged]
        |  lpObjectTable (Va)         |  lpExternalTable + dwExternalCount
        v                             v
 vb::project::ObjectTableHead   vb::declare::DeclareTable   <- NEW (02-06)
   (wTotalObjects, capacity,          |  dwEntryType==7 entries
    lpObjectArray)  <- corrected      v
        |                        Library name + export name
        v                        (no Alias, no arg types: emitted with
 vb::object::ObjectTable::walk        a comment, per STRUCTURES.md §7.2)
   (NEW, 02-01)
        |  for each of wTotalObjects entries, stride 0x30
        v
 vb::object::Object
   { lpObjectInfo, lpszObjectName, ProcCount, lpProcNamesArray, fObjectType }
        |                                  |
        v                                  v
 vb::classify::classify_object_kind   vb::procs::proc_names
   (NEW, 02-02: 3-value table,          (NEW, 02-03: null = private,
    Unknown fallback)                    array pointer 0 = .bas cap)
        |
        v (if fObjectType & 2)
 vb::object::ObjectInfo::read (NEW, 02-03)
        |  lpPrivateObject (Va, or -1 for a module)
        v
 vb::privateobj::PrivateObj::read (NEW, 02-03)
        |  lpFuncTypeInfo (parallel to lpProcNamesArray, length ProcCount)
        v
 vb::functyp::FuncTypDesc::read (NEW, 02-04)
        |  argSize, bFlags, optionalVals, lpAryArgNames
        v
 vb::functyp::walk_type_buffer (NEW, 02-04)
        |
        v
 vb::report::Report  (extended, not replaced)
        |
        v
 deform6-cli inspect  (extended: prints objects, kinds, prototypes, Declares)


 --- the harness, built from none of the above ---

 tests/support/vbp.rs (NEW, 02-07, independent reader)
   parses Form=/Module=/Class=/UserControl=/PropertyPage= lines,
   reads Attribute VB_Name out of the referenced source file,
   selects by ExeName32 when several .vbp are reachable
        |
        v
 tests/support/rules.rs (NEW, 02-07, the exclusion rules, Q8)
        |
        v
 tests/differential.rs (NEW, 02-08)
   compares deform6::vb output against support::vbp output,
   BOTH directions (declared subset of recovered AND recovered subset
   of declared)
        |
        v
 tests/ratios.toml (NEW, 02-09) <---> crates/xtask update-ratios (NEW, 02-09)
```

### Recommended Project Structure

```
crates/deform6/src/vb/
├── project.rs        # existing: ProjectInfo, ObjectTableHead (unchanged API)
├── object.rs          # NEW 02-01: Object, ObjectTable::walk, ObjectInfo
├── classify.rs         # NEW 02-02: fObjectType -> ObjectKind, the 3-value table + Unknown
├── privateobj.rs        # NEW 02-03: PrivateObj::read, the .bas null-pointer case
├── functyp.rs             # NEW 02-04: FuncTypDesc, walk_type_buffer, TYPE_CODES table
├── declare.rs               # NEW 02-06: the Declare import table (dwEntryType 6/7)
└── mod.rs                    # extended: Report gains objects, declares

crates/deform6/tests/
├── support/
│   ├── mod.rs         # NEW 02-07: re-exports vbp and rules to every test binary
│   ├── vbp.rs          # NEW 02-07: the independent .vbp reader, calls nothing in src/
│   └── rules.rs         # NEW 02-07: the exclusion rules (Q8), as data + predicates
├── differential.rs        # NEW 02-08: the both-directions gate over all 44
├── corpus_sweep.rs (existing, phase 1, unchanged)
└── refusal.rs (existing, phase 1, unchanged)

tests/ratios.toml            # NEW 02-09: the pinned two-counts-and-a-ratio file
                              # (repo root tests/, not crates/deform6/tests/,
                              #  because xtask and the differential test both
                              #  read it and neither should reach into the
                              #  other crate's tests/ directory)

crates/xtask/
├── Cargo.toml          # NEW 02-09: a workspace member, not excluded like fuzz
└── src/main.rs          # NEW 02-09: `cargo run -p xtask -- update-ratios`
```

**Root `Cargo.toml` change:** add `"crates/xtask"` to `[workspace] members`.
It is a normal member (unlike `crates/deform6/fuzz`, which is `exclude`d
because it needs nightly) — `xtask` builds on stable, so
`cargo test --workspace` and `cargo clippy --all-targets -- -D warnings`
both cover it, which is correct: a broken `xtask` should fail the gate like
anything else in this workspace.

### Pattern 1: the window-before-fields discipline, extended to the object array

**What:** every structure phase 1 reads is narrowed to its exact byte size
with `Region::subregion` before any field inside it is read (see
`vb/project.rs`'s doc comment). The object array is the first **array of
structures**, so this pattern needs a per-element application: take a
`0x30`-byte subregion at `arr_base + i * 0x30` for `Object`, not one giant
`w_total_objects * 0x30`-byte region up front.

**When to use:** every one of `Object`, `ObjectInfo`, `PrivateObj`,
`FuncTypDesc`, and the `Declare` entry array.

**Example (measured shape, not code — the offsets below are read from
`STRUCTURES.md` §5.1, and were exercised by this session's script against
all 44 corpus files with zero resolve failures):**
```
// Source: this session's /tmp/measure.py, run against all 44 corpus .exe
for i in range(w_total_objects):
    rec_off = arr_off + i * 0x30          # stride, not a running cursor
    lp_object_info   = u32(rec_off + 0x00)
    lpsz_object_name = u32(rec_off + 0x18)
    proc_count        = u32(rec_off + 0x1C)
    lp_proc_names      = u32(rec_off + 0x20)
    f_object_type        = u32(rec_off + 0x28)
```
`[VERIFIED: local, 105/105 objects across 44 files, this session]`

### Pattern 2: classify by exact match, fall back to `Unknown`, never refuse

**What:** `fObjectType` classification is a `match` over exactly the values
the corpus proves (three, this session), with every other value producing an
`Unknown` variant that carries the raw `u32` for the report, never a
`Refusal`.

**When to use:** `vb/classify.rs` (02-02).

**Example:**
```rust
// Source: this document's Q2, values measured against all 44 corpus files
pub enum ObjectKind {
    Form,
    Module,
    Class,
    Unknown(u32),
}

pub fn classify(f_object_type: u32) -> ObjectKind {
    match f_object_type {
        0x0001_8083 => ObjectKind::Form,
        0x0001_8001 => ObjectKind::Module,
        0x0011_8003 => ObjectKind::Class,
        other => ObjectKind::Unknown(other),
    }
}
```
This is deliberately narrower than `STRUCTURES.md` §5.5's full published
table (which lists `0x180A3`, `0x180C3` as additional Form values and
`0x18021`/`0x18041`/`0x18061` as additional Module values, `0x18023`,
`0x18803`, `0x138003` as additional Class values, plus UserControl,
PropertyPage and UserDocument values) — **none of those additional values
occur anywhere in this 44-file corpus** `[VERIFIED: local]`. Whether to
pre-populate the `match` with the full published table (accepting values
`[CITED: STRUCTURES.md §5.5]` this corpus cannot confirm) or start narrow and
widen only when a sample is found is a design choice for the planner; either
is defensible, but a narrow match must still route every uncovered value to
`Unknown`, never to a refusal.

### Anti-Patterns to Avoid

- **Treating a null `lpProcNamesArray` *entry* and a null `lpProcNamesArray`
  *pointer* as the same case.** They are not. A null entry inside a
  populated array means one private procedure (Q3). A null array pointer
  (measured in 8 of 8 `.bas` module objects, Q5) means the whole object has
  no procedure-name array to walk at all. Code that only checks
  `entry == 0` inside a loop over `proc_count` will either skip the loop
  body entirely by accident (harmless) or, if written to check the pointer
  first and unconditionally deref it when `proc_count > 0`, will read
  garbage at address `0`. Check the array pointer before the loop.
- **Assuming `fObjectType`'s presence bit and `lpPrivateObject`'s sentinel
  will ever disagree in this corpus.** They do not, 105 of 105 `[VERIFIED:
  local]`. Cross-checking them per `STRUCTURES.md`'s gap-2 recommendation is
  still correct practice (a future corpus file might disagree), but do not
  design the report format around "usually agrees, occasionally doesn't" —
  in this corpus it is "always agrees."
- **Building `ratios.toml`'s key from the leaf directory name.** Three
  top-level corpus directories (`Brightness-effect`, `SK-TFTP-Sample__VB6`,
  `machineLanguageConversion`) each contain more than one compiled program.
  A leaf-name key collides. Key by the `.exe`'s path relative to `corpus/`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| TOML read/write for `ratios.toml` | A hand-rolled `key = value` line parser | `toml` crate | The file has quoted keys with slashes and spaces (`corpus/vb6-code/Brightness-effect/Part 1 - Pure VB6/vbBrightness.exe`); a hand-rolled parser will get quoting wrong exactly the way the phase 1 `.vbp` reader almost did with `Title=`. `AGENTS.md`'s "match the shape of the file, not one form of it" applies directly. |
| Latin-1 / Windows-1252 string decoding | `String::from_utf8_lossy` | `char::from(byte)`, matching phase 1's `Region::cstr` doc comment | Already established; repeated here because `support/vbp.rs` reads `.vbp` and source files with the exact same non-ASCII risk (`©2020 Tanner Helland` in `Brightness.vbp`, measured this session — a naive UTF-8-aware `grep` silently treats the file as binary and returns nothing without `-a`; Python's UTF-8 decode would raise). |
| `.vbp` object-name resolution | Trusting the `.vbp` line's `Name;` prefix, or the filename | Read `Attribute VB_Name` out of the referenced source file | Measured this session: for a `Form=File.frm` line the `.vbp` gives **no** name at all — the compiled object name (`frmFractal` for `Mandelbrot.frm`) only exists inside the `.frm`'s `Attribute VB_Name` line, which can sit thousands of lines into the file when the form embeds picture data (the first attempt at this measurement used a 40-line-prefix scan and silently matched zero forms; scanning the whole file fixed it). |

**Key insight:** every "don't hand-roll" item above was learned by getting it
wrong first, in this session, against the real corpus — not by reading a
warning in a document. That is the argument for `support/vbp.rs` needing its
own test coverage independent of `deform6::vb`, not just independence of
implementation: a bug in the name-resolution rule silently produces a 0%
match rate that looks like "no forms in this corpus" rather than a loud
failure.

## Common Pitfalls

### Pitfall 1: confusing "not exercised by the corpus" with "does not exist"

**What goes wrong:** a value that never appears in 44 real programs gets
treated as impossible, and the code path for it (Unknown fObjectType, an
unassigned type code, a ParamArray) either panics or is simply missing.

**Why it happens:** 44 real-world hobby/utility programs is a large, diverse
corpus by this project's standards, but it is not exhaustive of the VB6
language. `STRUCTURES.md`'s own gap register lists MDIForm, UserControl,
PropertyPage and UserDocument as documented but unobserved-here kinds; this
session's measurement confirms none of them occur (Q2).

**How to avoid:** every classification function ends in a fallback arm that
carries the raw value forward (`Unknown(u32)`, `UnknownType(u8)`), never a
`Refusal` or a `panic!`. This is already `#![forbid(unsafe_code)]`-and-lint-
wall-enforced for the panic half; the `Unknown` fallback discipline is a code
review concern, not a compiler-enforced one.

**Warning signs:** a `match` with no wildcard arm over a value read directly
from the file (as opposed to over an internal enum built entirely by this
crate, where an exhaustive match is exactly right, e.g. `DefectKind::severity`
in `error.rs`).

### Pitfall 2: computing an entry count with the wrong shift, silently

**What goes wrong:** `FuncTypDesc.argSize >> 2` gives the type-entry count.
Get the shift wrong (say, `>> 1`) and the type buffer walk still "succeeds"
much of the time, because zero-padding between entries (documented in
`STRUCTURES.md` §6.6, "Padding is real and must be tolerated") absorbs a
short walk without an obvious crash — it just silently reads the wrong number
of argument names out of `lpAryArgNames`.

**Why it happens:** the two numbers (byte count, entry count) are related by
a shift that is easy to get backwards once, and the padding tolerance in the
format specifically hides the class of bug that would otherwise surface it
immediately.

**How to avoid:** this session's walk validated the shift is right by
checking `entries_found == argSize >> 2` on every one of 193 real records —
193 of 193 closed exactly `[VERIFIED: local]`. Keep an equivalent assertion
in the shipped parser (as a `Defect`, not a debug assertion, since it reads
attacker-controlled bytes): if the walk does not close exactly at
`argSize >> 2` entries within a bounded number of steps, report the record as
unrecoverable rather than accepting a partial result.

**Warning signs:** a prototype whose argument name count does not match its
type-entry count.

### Pitfall 3: `cntPublicVars` is not what its name says

**What goes wrong:** a naive reading of `STRUCTURES.md` §6.1
(`cntPublicVars`: "Number of entries in the `PubVarDesc` array") plus §6.4
("Inline records, `cntPublicVars` of them") suggests walking
`lpPublicVars` for `cntPublicVars` fixed-or-conditional-stride records will
recover every `Public variable As Type` declaration. It does not.

**Why it happens:** measured this session — `pdOpenSaveDialog.cls` (present
in 15 of the 44 corpus programs) declares **zero** `Public variable As Type`
lines (only a `Public Enum`), yet every corpus binary reports
`cntPublicVars = 4` for it. `Organism.cls` in `Artificial-life` declares 17
real public variables (some on comma-joined lines) and reports
`cntPublicVars = 23`. `frmMain.frm` in `Artificial-life` declares **zero**
`Public` anything and reports `cntPublicVars = 5`, while having 10 top-level
named controls. None of the four candidate fixed strides (`0x18`, `0x1C`,
`0x20`, `0x24`) nor the type-code-conditional stride GD's figure implies
produces a clean, fully-resolving walk on any of these arrays.

**How to avoid:** do not implement `PubVarDesc` enumeration in this phase.
Report `cntPublicVars` as a raw number in the confidence report if useful,
but do not claim it enumerates public variables until a controlled
compile-and-diff experiment (compile a class with a known, isolated set of
`Public` declarations, nothing else, and diff) resolves what the field
actually counts.

**Warning signs:** a "public variable" name that does not appear anywhere in
the corresponding `.cls`/`.frm` source file.

## Code Examples

### The `.vbp` object-name resolution `support/vbp.rs` needs

```python
# Source: this session's /tmp/measure.py, exercised against all 44 corpus
# programs, 105/105 objects matched 1:1 by this method (0 unmatched after
# the whole-file fix below; 53/105 unmatched with a 40-line-prefix scan).
def find_vb_name(src_path):
    """Read `Attribute VB_Name = "X"` out of a .frm/.bas/.cls/.ctl file.
    For a .bas or .cls this is near the top. For a .frm it comes AFTER the
    whole `Begin VB.Form ... End` block, which can run to thousands of
    lines when the form embeds picture data — scan the whole file."""
    text = read_bytes(src_path).decode("latin-1")
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("Attribute VB_Name"):
            eq = s.find("=")
            val = s[eq + 1:].strip()
            if val.startswith('"') and val.endswith('"'):
                return val[1:-1]
    return None
```

### The `.vbp` quoted-value rule

```python
# Source: this session's /tmp/measure.py. Verified against
# corpus/vb6-code/Sepia-effect/Sepia.vbp, which holds:
#   Title="Sepia / "Antique" Image Filter"
def vbp_quoted(line):
    first = line.find('"')
    last = line.rfind('"')
    if first == -1 or last == -1 or last <= first:
        return None
    return line[first + 1:last]
# Result: 'Sepia / "Antique" Image Filter'  -- the whole string, inner
# quotes included, [VERIFIED: local] this session.
```

### `ExeName32` selection, the two corpus shapes it must handle

```
# Source: this document's Q7 measurement.
# Case 1: corpus/public-domain/SK-MCI-Sample__VB6/
#   MCI.VBP (upper case) sits in the project root; the compiled binary
#   (Project1.exe) sits in a demo/ subdirectory. Neither a same-directory
#   search nor a case-sensitive *.vbp glob finds the .vbp from the .exe's
#   own directory. MCI.VBP's ExeName32="Project1.exe" is the only string
#   that ties the two together, and the .exe's own file name (Project1.exe)
#   bears no resemblance to the .vbp's own file name (MCI.VBP).
#
# Case 2: corpus/public-domain/SK-TFTP-Sample__VB6/
#   Client/TFTPClient.vbp (ExeName32="TFTPClient.exe") and
#   Server/Server.vbp (ExeName32="Server.exe") are both reachable from the
#   shared parent SK-TFTP-Sample__VB6/. A glob rooted at that shared
#   parent finds both files for either executable; ExeName32 is what
#   disambiguates.
```
`[VERIFIED: local, this session — both ExeName32 values read directly from
the two .vbp files]`

## State of the Art

Not applicable in the usual sense — this is reverse-engineering a fixed,
long-frozen binary format (VB6 shipped its last version in 1998), not a
library with an evolving API. The one relevant "old vs. current" fact is
internal to this project's own research history:

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| Loop the object array on `wCompiledObjects` (`STRUCTURES.md` §4, `python-vb`, the Gen Digital article) | Loop on `wTotalObjects` | 2026-09-07, phase 1 plan 01-07, re-confirmed this session | `wCompiledObjects` over-reports by exactly the array's rounding-up amount in 15 of 44 corpus files; looping on it would print `Objects 4` for a project declaring 1 object |

**Deprecated/outdated:** the `STRUCTURES.md` §4 original recommendation to
loop on `wCompiledObjects` and report a mismatch as damage — both parts are
withdrawn in §4.1 and confirmed again by this session's independent script.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | `ratios.toml`'s two counts should be "procedures recovered by name / non-`.bas` `ProcCount` declared" rather than object-level counts | Q9 | Object-level counts are 44/44 (no variance) in this corpus and would be a weak pin; procedure-level counts have real variance (worked examples: `Mandelbrot` 9/9, `Grayscale-effect` 12/34, `LockWorkStation` 0/1) but nothing upstream states this explicitly — if the intended metric is object-level, the pinned file needs a different shape |
| A2 | Which edit direction (`ratios.toml` number down vs. up) produces `REGRESSION` vs. `MOVED UP` | Q9 | ROADMAP's literal wording ("editing down → REGRESSION, editing up → MOVED UP") is, under the most natural comparison rule (fresh-actual vs. pinned-baseline), the opposite pairing from what a standard golden-file test produces. Getting this backwards makes the two failure messages swap meaning, which is exactly the kind of test that "passes for the wrong reason" `AGENTS.md` warns about |
| A3 | The full published `fObjectType` table (`STRUCTURES.md` §5.5, 17 values) should be pre-populated in `classify()` even though only 3 of the 17 occur in this corpus | Pattern 2 | If pre-populated and one of the untested values is subtly wrong in the source document, a future file gets silently misclassified instead of landing in `Unknown` and being flagged; if left narrow, a future common case (UserControl, say) is needlessly reported `Unknown` until a sample turns up |
| A4 | `support/vbp.rs` and `support/rules.rs` live under `crates/deform6/tests/support/`, and `tests/ratios.toml` lives at the repository root, not under `crates/deform6/tests/` | Recommended Project Structure | A different placement is equally defensible (e.g. everything under `crates/deform6/tests/`) and this is a naming/location choice, not a measured fact — flagged so the planner can override cheaply if it disagrees |

**If this table is empty:** not applicable — see rows above.

## Open Questions

1. **Which item does `ratios.toml` pin — objects, or procedures?**
   - What we know: object-level recovery is 44/44 across the whole corpus
     right now (Q1, Q2), which would make every pinned ratio `1.00` with no
     variance — a technically valid but low-information pin. Procedure-level
     recovery has real variance today (Q3, Q5) and is the harder, more
     representative target, and it is what OBJ-03/OBJ-04 actually promise.
   - What's unclear: nothing in `ROADMAP.md`, `REQUIREMENTS.md` or
     `CONTEXT.md` states which one `ratios.toml` tracks.
   - Recommendation: pin procedure-level recovery (recovered-by-name /
     `ProcCount` summed over non-`.bas` objects), using the worked numbers in
     Q9 as the concrete shape, and confirm with the user at discuss-phase
     before committing the design into a plan.

2. **`REGRESSION` vs. `MOVED UP` — which edit direction triggers which?**
   - What we know: the two words and the update workflow
     (`cargo run -p xtask -- update-ratios`) are named exactly.
   - What's unclear: the roadmap's literal wording produces a pairing this
     document's own derivation (Assumption A2) finds counter-intuitive under
     the standard fresh-vs-pinned comparison. This is a specification
     ambiguity, not a corpus fact, so it cannot be resolved by measurement.
   - Recommendation: confirm the intended direction explicitly before 02-09
     is planned; the safe default in the meantime is the standard golden-file
     rule (actual < pinned → `REGRESSION`; actual > pinned → `MOVED UP`),
     documented here as `[ASSUMED]`.

3. **Should `classify()` accept the full 17-value `STRUCTURES.md` §5.5 table,
   or only the 3 values this corpus proves?**
   - What we know: both are safe with respect to SAF-01 (neither panics on
     an unrecognized value); the difference is only how quickly a real
     UserControl/PropertyPage/MDIForm file gets a name instead of `Unknown`.
   - What's unclear: whether a future milestone widens the corpus with such
     a file before Phase 6, making this moot either way.
   - Recommendation: start narrow (3 values, this corpus), because
     `[VERIFIED: local]` beats `[CITED: STRUCTURES.md §5.5]` for something a
     wrong value would silently mis-tag — widen when a sample exists.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| `rustc` / `cargo` | The whole workspace | ✓ | pinned by `rust-toolchain.toml`, matches phase 1's gate | — |
| `toml` crate (crates.io) | `crates/xtask` | ✓ | `1.1.5+spec-1.1.0` on the live registry, this session | — |
| Python 3 (research only, not shipped) | This document's measurement scripts | ✓ | 3.14.7 | Not applicable — throwaway, not part of the build |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

## Validation Architecture

### Test Framework

| Property | Value |
|---|---|
| Framework | `cargo test`, the workspace's only test runner (established phase 1, unchanged) |
| Config file | none — `cargo test --workspace` is the whole invocation, per `AGENTS.md`'s "The gate" |
| Quick run command | `cargo test -p deform6 --lib` (unit tests only, no corpus sweep) |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| OBJ-01 | Object array walk, name resolves for every object | unit + integration | `cargo test -p deform6 --lib vb::object::` | ❌ Wave 0 (`vb/object.rs` new) |
| OBJ-02 | `fObjectType` classification | unit | `cargo test -p deform6 --lib vb::classify::` | ❌ Wave 0 (`vb/classify.rs` new) |
| OBJ-03 / OBJ-06 | Procedure names, null = private, `.bas` array-pointer-null | unit | `cargo test -p deform6 --lib vb::privateobj::` | ❌ Wave 0 (`vb/privateobj.rs` new) |
| OBJ-04 | Prototypes, type codes, modifiers, `optionalVals` | unit | `cargo test -p deform6 --lib vb::functyp::` | ❌ Wave 0 (`vb/functyp.rs` new) |
| OBJ-05 | `Declare` table | unit | `cargo test -p deform6 --lib vb::declare::` | ❌ Wave 0 (`vb/declare.rs` new) |
| VER-01/02/03 | Differential, both directions, `.vbp` selection | integration | `cargo test -p deform6 --test differential` | ❌ Wave 0 (`tests/differential.rs`, `tests/support/` new) |
| VER-04 | Exclusion rules | integration (part of `differential.rs`) | same as above | ❌ Wave 0 (`tests/support/rules.rs` new) |
| VER-05 | Pinned ratio, two failure messages | integration | `cargo test -p deform6 --test differential -- ratios` (or a dedicated `tests/ratios.rs`) | ❌ Wave 0 (`tests/ratios.toml`, `crates/xtask` new) |

### Sampling Rate

- **Per task commit:** `cargo test -p deform6 --lib` (the module under
  active work, fast)
- **Per wave merge:** `cargo test --workspace` (full suite, including the
  44-file differential and corpus sweeps)
- **Phase gate:** `cargo fmt --all --check && cargo clippy --all-targets --
  -D warnings && cargo test --workspace` green before `/gsd-verify-work`,
  per `AGENTS.md`'s "The gate," unchanged from phase 1

### Wave 0 Gaps

- [ ] `crates/deform6/src/vb/object.rs` — `Object` array walk (OBJ-01)
- [ ] `crates/deform6/src/vb/classify.rs` — `fObjectType` table (OBJ-02)
- [ ] `crates/deform6/src/vb/privateobj.rs` — `ObjectInfo`, `PrivateObj`, proc names (OBJ-03, OBJ-06)
- [ ] `crates/deform6/src/vb/functyp.rs` — `FuncTypDesc`, type codes (OBJ-04)
- [ ] `crates/deform6/src/vb/declare.rs` — the `Declare` table (OBJ-05)
- [ ] `crates/deform6/tests/support/mod.rs`, `vbp.rs`, `rules.rs` — the independent reader and exclusion rules (VER-01 to VER-04)
- [ ] `crates/deform6/tests/differential.rs` — the both-directions gate (VER-01, VER-02, VER-03)
- [ ] `tests/ratios.toml` + `crates/xtask` — the pin and the updater (VER-05)
- [ ] `crates/xtask/Cargo.toml` — new workspace member, `toml` dependency

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V2 Authentication | no | This tool has no auth surface — a local CLI reading a local file |
| V3 Session Management | no | Same |
| V4 Access Control | no | Same |
| V5 Input Validation | **yes** | Every count and pointer in this phase (`wTotalObjects`, `ProcCount`, `dwExternalCount`, `argSize`, every `Va`) is attacker-controlled and must go through `Region`/`checked_add`, exactly as phase 1's threat model already establishes. No new primitive is needed; every read in this phase reuses `Region::u32_le`/`va_le`/`cstr` and `PeImage::region_at_va` |
| V6 Cryptography | no | Not applicable to this domain |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| `ProcCount` (or `cntPublicVars`, `cntEvents`, `dwExternalCount`) used to size an allocation before being checked against the real file size | Denial of Service | `SAF-04`: bound every count against the real file length before any `Vec::with_capacity`-shaped allocation. This phase reads several new counts (`ProcCount` up to 59 arguments per `STRUCTURES.md` §6.6's own sanity bound, `dwExternalCount`, `cntPublicVars`) that phase 1 never touched |
| A `FuncTypDesc` type-buffer walk that does not bound its own step count | Denial of Service | This session's own walker (`walk_type_buffer`) caps `max_types` and treats a non-closing walk as a failure, not an infinite loop — carry the same discipline into the shipped parser; a hostile `argSize` could otherwise drive an unbounded scan for a padding zero that never resolves |
| Trusting `lpProcNamesArray`'s pointer without checking it against `0` before treating `ProcCount` as a valid loop bound over it | Tampering / Spoofing (a crafted file reports procedures that do not exist) | Explicit null-pointer check before the loop, per Pitfall 1 above — this is the exact shape of bug the `.bas` cap measurement surfaced as a real, common (not just theoretical) corpus case |
| The `Declare` table's `dwEntryType == 6` "internal" entries dereferenced as if they were external DLL imports | Information Disclosure (reading unrelated in-image bytes as a fabricated library/API name pair) | Skip `dwEntryType != 7` entries entirely, per `STRUCTURES.md` §7.1 and this session's own Q6 measurement (29 of 249 entries in the corpus are type 6) |

## Sources

### Primary (HIGH confidence — measured this session)

- `/tmp/measure.py` and `/tmp/analyze.py` (this session's throwaway scripts,
  independent minimal PE/VB parser, no dependency on `deform6`'s own code) —
  run against all 44 files in `corpus/vb6-code/` and `corpus/public-domain/`
- `corpus/**/*.vbp`, `corpus/**/*.frm`, `corpus/**/*.bas`, `corpus/**/*.cls`
  — read directly for the `Attribute VB_Name` join, the `ParamArray` /
  `Optional ... =` source grep, and the `Public` variable-declaration counts
- `cargo search toml`, `gsd_run query package-legitimacy check --ecosystem
  crates toml` — this session, live crates.io index

### Secondary (MEDIUM confidence — cited from this project's own prior research)

- `.planning/research/STRUCTURES.md` §4, §4.1, §5, §6, §7, §11, §12, §13 —
  the field layouts this session's script encodes; every offset used in the
  script was taken from this document and then validated against real bytes
- `.planning/phases/01-it-reads-the-file/01-07-SUMMARY.md`,
  `01-08-SUMMARY.md` — the phase 1 measurement that first corrected
  `wCompiledObjects` → `wTotalObjects`, re-confirmed independently this
  session

### Tertiary (LOW confidence — cited only where the corpus stayed silent)

- `.planning/research/STRUCTURES.md` §5.5's full 17-value `fObjectType`
  table, §6.5's fifteen unassigned type codes, §6.4's `PubVarDesc` stride
  discussion — all `[CITED]`, none `[VERIFIED]`, because this corpus does not
  exercise them

## Metadata

**Confidence breakdown:**
- Object array walk (Q1), classification (Q2), ObjectInfo/PrivateObj shape
  and null semantics (Q3), the `.bas` cap (Q5): **HIGH** — every number is a
  script output against all 44 files, cross-checked against `.vbp` ground
  truth via `Attribute VB_Name`
- `FuncTypDesc` type codes and `optionalVals` (Q4): **HIGH** on what was
  observed (193/193 records, two independent worked closures on
  `optionalVals`), **explicitly LOW/absent** on the fifteen unassigned codes
  and the `ParamArray` encoding — the corpus does not exercise them and this
  document does not pretend otherwise
- External `Declare` table (Q6): **HIGH** on the entry-type histogram and
  zero-ordinal-usage finding; **absent** on the ordinal encoding itself
  (gap 10 stays open, no sample anywhere in the corpus, source or binary)
- `.vbp` reading mechanics (Q7), exclusion rules (Q8): **HIGH** — every named
  project (`SK-MCI-Sample__VB6`, `SK-TFTP-Sample__VB6`, `Hidden-Markov-model`,
  `Randomize-effects`, `SK-Gradient-Sample__VB6`) was opened and its exact
  files/keys read this session
- `ratios.toml` shape (Q9): **LOW/design proposal** — this is a specification
  question the corpus cannot answer; flagged in Open Questions and the
  Assumptions Log rather than asserted

**Research date:** 2026-09-07
**Valid until:** indefinite for the corpus-measured facts (the corpus is
static and vendored, not a moving target); 30 days for the `toml` crate
version pin, standard for a Rust dependency
