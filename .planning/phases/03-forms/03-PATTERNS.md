# Phase 3: Forms - Pattern Map

**Mapped:** 2026-09-09
**Files analyzed:** 10 (Wave 0 list from `03-VALIDATION.md`)
**Analogs found:** 10 / 10

## Plain facts the planner needs

- `STRUCTURES.md` exists at `.planning/research/STRUCTURES.md`. Plan 03-04
  edits it (gap 11 closure, per `03-CONTEXT.md` D-03).
- `GAPS.md` exists at `.planning/phases/02-the-object-graph/GAPS.md`. Plan
  03-04 edits it (the two corrected corpus counts, per D-03).
- Both files are git-tracked source, not a mirror. `git ls-files` confirms
  both paths.

## File Classification

| New File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/deform6/src/vb/gui.rs` | model/parser | request-response (bounded structure read) | `crates/deform6/src/vb/object.rs` | exact |
| `crates/deform6/src/vb/controltree.rs` | model/parser | event-driven walk (scope-byte tree, bounded) | `crates/deform6/src/vb/object.rs` (loop-and-window shape) | role-match |
| `crates/deform6/src/vb/vbstr.rs` | utility | transform (string decode, cursor discipline) | `crates/deform6/src/vb/object.rs::read_name` | role-match |
| `crates/deform6/src/vb/propstream.rs` | model/parser | transform (typed payload decode) | `crates/deform6/src/vb/object.rs` (per-field windowed reads) | role-match |
| `crates/deform6/src/vb/opcodes.rs` | config/loader | file-I/O (optional external table) | `crates/deform6/src/vb/classify.rs` (raw-value-to-name table, no guessing) | role-match |
| `crates/deform6/src/vb/frx.rs` | utility | file-I/O (blob extraction, running cursor) | `crates/deform6/src/vb/object.rs::read_name` (bounded region read) | partial |
| `crates/deform6/src/vb/ocx.rs` | model/parser | request-response (fixed header + join) | `crates/deform6/src/vb/project.rs` (CLSID/component join, phase 2) | role-match |
| `crates/deform6/src/vb/controlinfo.rs` | model/parser | request-response (structure + join) | `crates/deform6/src/vb/object.rs` (pointer-chain resolve, join by name) | role-match |
| `crates/deform6/tests/support/frm.rs` | test (test-only, no `src/` reach) | file-I/O (independent text-form reader) | `crates/deform6/tests/support/vbp.rs` | exact |
| `crates/deform6/tests/differential.rs` (extended) | test | request-response (both-directions compare) | itself, phase 2's own object/procedure sections | exact |
| `tests/ratios.toml` (extended) | config/test-data | batch (pinned per-program ratio) | `tests/ratios.toml` (existing procedure-ratio rows) + `crates/deform6/tests/ratios.rs` | exact |

## Pattern Assignments

### `crates/deform6/src/vb/gui.rs` and `crates/deform6/src/vb/controltree.rs`

**Analog:** `crates/deform6/src/vb/object.rs`

**Module doc / window-before-fields discipline** (lines 1-27):
```rust
//! `Object`, the array `lpObjectArray` points at, and the walk that recovers
//! the name of every one.
//!
//! `STRUCTURES.md` section 4.1 corrects the loop bound: it is `wTotalObjects`,
//! never `wCompiledObjects`, ...
//!
//! Each `Object` is narrowed to its own `0x30`-byte window before any field
//! inside it is read, which is the window-before-fields discipline
//! `vb/project.rs` documents, applied for the first time to an array of
//! structures rather than to one structure: a fresh `subregion` is taken per
//! element, never one region for the whole array indexed by hand.
```
Use this same discipline for `GuiObjectInfo` (fixed-size subregion, then
field reads inside it) and for each `ControlNode`'s own `Length`-bounded
subregion.

**Imports** (lines 29-32):
```rust
use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Rva, Va};
use crate::vb::project::ObjectTableHead;
```

**Bounded loop with `checked_mul`/`checked_add`, never `+`** (lines 145-151):
```rust
for i in 0_u32..u32::from(head.w_total_objects) {
    let at = i
        .checked_mul(OBJECT_SIZE)
        .ok_or(Refusal::Damaged("the object array index overflows a u32"))?;
    let element = array
        .subregion(Off::new(at), OBJECT_SIZE)
        .ok_or(Refusal::Damaged("the file ends inside an Object element"))?;
```
`controltree.rs`'s scope-byte scan should use the same shape: a fixed
`MAX_SCOPE_RUN` bound (research's Pattern 2), never a loop bound taken from
the file.

**`get`/`None` field read, never `[]` or `unwrap`** (lines 313-321):
```rust
fn u32_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<u32, Refusal> {
    window.u32_le(Off::new(at)).ok_or(Refusal::Damaged(what))
}

fn va_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<Va, Refusal> {
    window.va_le(Off::new(at)).ok_or(Refusal::Damaged(what))
}
```

**Refusal vs. per-item `Defect`, and how a `Defect` names the byte offset**
(lines 220-262, `read_name`):
```rust
fn read_name(
    pe: &PeImage<'_>,
    element: &Region<'_>,
    lpsz_object_name: Va,
) -> (String, Option<Defect>) {
    let offset = element.file_offset(Off::new(0x18)).map_or(0, Off::get);
    let site = Site {
        offset,
        rva: lpsz_object_name.to_rva(pe.image_base()).map(Rva::get),
        structure: "Object",
        field: "lpszObjectName",
    };

    let Some(name_region) = pe.region_at_va(lpsz_object_name) else {
        let kind = DefectKind::UnreadablePointer {
            offset,
            va: lpsz_object_name.get(),
        };
        return (String::new(), Some(Defect { site, kind }));
    };
    ...
}
```
`Refusal` (an `Err` that stops the whole walk, e.g. `Refusal::Damaged(&str)`)
is for a structural fact the walk cannot proceed past (file ends inside a
block, pointer resolves nowhere for the whole table). A `Defect` (pushed into
a `Vec<Defect>`, walk continues) is for one item's field that could not be
read; the object/control keeps its other fields. `controltree.rs`'s tiling
mismatch (research Pattern 1) is a `Refusal`, named with the byte offset in
the message, because it invalidates the whole tree, not one control.

**Overflow-safe pointer arithmetic** (lines 204-218):
```rust
fn object_array_va(pe: &PeImage<'_>, lp_object_table: Va) -> Result<Va, Refusal> {
    let at = pe.region_at_va(lp_object_table).ok_or(Refusal::Damaged(
        "the object table pointer is in no section",
    ))?;
    let window = at
        .subregion(Off::new(0), OBJECT_TABLE_SIZE)
        .ok_or(Refusal::Damaged(
            "the file ends inside the object table structure",
        ))?;
    va_at(&window, 0x30, "the object table holds no address for the object array")
}
```

---

### `crates/deform6/src/vb/vbstr.rs`

**Analog:** `crates/deform6/src/vb/object.rs::read_name` (lines 220-262
above) plus the module's own `NAME_MAX` bound (lines 47-52):
```rust
/// The bound on an object name string.
///
/// The same value and the same reason `vb/project.rs` gives for the project
/// name: `Region::cstr` needs a mandatory maximum, so a file with no NUL byte
/// after the name cannot make the scan run to the end of the section.
const NAME_MAX: u32 = 0x104;
```
`vbstr.rs` needs the stronger rule research Pattern 3 states: the cursor
always advances by the *declared* length, never by what the decode found.
`object.rs::read_name` already shows the weaker half of this (bounded `cstr`
scan, `None` on no terminator becomes a `Defect`, not a panic); `vbstr.rs`
adds the "retry once as the other encoding, same landing check" step
`03-RESEARCH.md`'s own illustrative `read_vb_str` sketches (see
`03-RESEARCH.md` Pattern 3's code block for the concrete shape to follow;
this document does not restate it because the research doc already gives an
executable skeleton against these same `Region`/`Off`/`Defect` types).

**Latin-1, byte-by-byte, never `String::from_utf8_lossy`** (lines 248-253):
```rust
match name_region.cstr(Off::new(0), NAME_MAX) {
    // Each byte becomes its Latin-1 code point, which is the rule
    // `vb/project.rs` uses for the project name. `String::from_utf8_lossy`
    // is wrong here: a byte in 0x80 to 0xFF would become the replacement
    // character and the name would be lost.
    Some(bytes) => (bytes.iter().copied().map(char::from).collect(), None),
    None => { ... }
}
```

---

### `crates/deform6/src/vb/propstream.rs`, `frx.rs`, `ocx.rs`, `controlinfo.rs`

**Analog:** `crates/deform6/src/vb/object.rs` for the read shape (`u32_at`,
`va_at`, per-field `Result<_, Refusal>` with a static message naming what was
expected); `crates/deform6/src/vb/classify.rs` for `opcodes.rs`'s "carry raw,
never guess" discipline, echoed directly in `object.rs`:
```rust
/// The form / module / class discriminator, carried raw.
///
/// Plan 02-02 classifies this value. This file does not, and it refuses
/// no object on the strength of an unrecognised one.
pub f_object_type: u32,
```
`opcodes.rs` must report an unnamed opcode the same way (present, offset
given, opcode given, name undecoded), matching `03-RESEARCH.md` Pattern 4's
exact wording:
```
Property opcode 31 at offset 0x1a04: value not decoded, no opcode table
  loaded for control type CommandButton. Run with --opcode-table to
  supply one.
```

**Bounded scan, never an unbounded loop on file-controlled bytes** (already
quoted above under gui.rs/controltree.rs; `frx.rs`'s running-cursor blob walk
and `propstream.rs`'s position-block escape both need the identical
`checked_add`-only, fixed-iteration-cap shape).

---

### `crates/deform6/tests/support/frm.rs`

**Analog:** `crates/deform6/tests/support/vbp.rs`, in full for the two rules
`03-CONTEXT.md` requires.

**`walk_by_extension`, quoted in full** (lines 54-72):
```rust
/// Walks a directory recursively and gives every file whose extension
/// matches `ext`, compared case-insensitively.
fn walk_by_extension(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_dir() {
            walk_by_extension(&path, ext, out);
        } else if path
            .extension()
            .is_some_and(|found| found.eq_ignore_ascii_case(ext))
        {
            out.push(path);
        }
    }
}
```

**The doc comment `03-CONTEXT.md` names, on 45 project files and 44
executables** (lines 74-88):
```rust
/// Gives every executable under `corpus/`, sorted.
///
/// **Iterate this, never [`project_files`].** The repository holds 45
/// project files and 44 executables:
/// `Brightness-effect/Part 3 - DIBs/Brightness3.vbp` declares
/// `dibBrightness.exe`, which the repository does not vendor. A harness
/// that walked project files would test one project with no binary to
/// check it against.
#[must_use]
pub fn executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_by_extension(&corpus_root(), "exe", &mut out);
    out.sort();
    out
}
```
`support/frm.rs`'s own `forms()` function copies this exact shape:
`walk_by_extension(&corpus_root(), "frm", &mut out)`. `03-CONTEXT.md`
requires it name 54, not 53, because `MCI.FRM` is upper-case and
`eq_ignore_ascii_case` is what catches it.

**"reads bytes, never a `String`" rule, precedent in `Project::read`**
(lines 159-177):
```rust
/// A `.vbp` file, read as Latin-1 bytes and held as the exact text the
/// compiler wrote.
///
/// Every byte becomes its own Latin-1 code point, the same rule
/// `crates/deform6/src/vb/project.rs` uses for the strings it reads.
/// `Brightness-effect/Part 4 - Even faster DIBs/Brightness.vbp` carries a
/// `©2020 Tanner Helland` copyright line; a UTF-8-aware decode would
/// either lose that line or refuse the file, and neither is correct for
/// a file this corpus vendors on purpose.
pub struct Project {
    pub path: PathBuf,
    text: String,
}

impl Project {
    #[must_use]
    pub fn read(path: &Path) -> Self {
        let bytes =
            std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
        let text: String = bytes.iter().copied().map(char::from).collect();
        Self { path: path.to_owned(), text }
    }
    ...
}
```
`support/frm.rs` copies this exactly: `std::fs::read`, then
`bytes.iter().copied().map(char::from).collect()`, never
`fs::read_to_string`. This is what survives `Threshold.frm`'s byte `0xA9` at
offset 5669 without failing the whole read.

**No `src:` reach, stated as a hard rule at the top of the file** (lines
1-22 of `vbp.rs`), including the `#![allow(clippy::unwrap_used, ...)]` block
that is scoped to test-harness code reading a fixed, vendored corpus, not to
`src/`:
```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "this is the test harness, not the library under test: it reads a vendored, \
              fixed corpus this repository controls, so the strict input-hostility \
              discipline `src/` carries does not apply ..."
)]

//! A second, independent `.vbp` reader.
//!
//! **This file names nothing from `deform6`.** No `use deform6::...`, no
//! shared type, no shared constant.
```
`support/frm.rs` needs the equivalent allow block and the equivalent
"names nothing from `deform6`" sentence, plus its own named
`frmHMM.frx` exclusion mechanism: `03-RESEARCH.md`'s dedicated section
says this must be a short, explicit `(file path, reason)` list, a
different, narrower shape than `support/rules.rs`'s generic predicates
(`support/rules.rs` was not read in full for this map; the planner should
open it directly when writing the exclusion list, since its five rules are
deliberately generic and forbidden from naming a program, the opposite of
what VER-06 asks for).

---

### `crates/deform6/tests/differential.rs` (extended)

**Analog:** itself. Phase 2 already built the both-directions compare shape
this phase extends to forms and controls (object recovery, lines around
300-430, and procedure recovery, lines 440-720).

**Two-directional diff, never a subset assertion** (from the file's own
module doc, lines 29-33, and echoed in `object.rs`'s own test comment: "A
subset assertion cannot see an over-count, which is the whole reason this
phase exists.") The forms/controls extension must compute and assert
**both** `declared_not_recovered` and `recovered_not_declared`, matching:
```rust
assert!(diff.declared_not_recovered.is_empty());
...
assert!(diff.recovered_not_declared.is_empty());
```
(pattern present at lines 399 and 428 of the existing file).

---

### `tests/ratios.toml` (extended) and `crates/deform6/tests/ratios.rs`

**Analog:** the existing procedure-ratio pin, whose pinned totals and
REGRESSION/MOVED_UP vocabulary are the shape the new form ratio and control
ratio columns copy directly.

**Pinned constants with a named reason for each direction** (lines 96-114):
```rust
const EXPECTED_PROGRAM_COUNT: usize = 44;
const EXPECTED_TOTAL_RECOVERED: u32 = 185;
const EXPECTED_TOTAL_DECLARED: u32 = 904;

/// Editing a pinned number **up** means the pin now claims more procedures
/// than the tool recovers: something the pin expects went missing. This
/// describes what happened to the tool, never to the file -- reasoning
/// from the file instead of from the tool gives the opposite pairing.
pub(crate) const REGRESSION: &str = "REGRESSION";

/// Editing a pinned number **down** means the tool now recovers more
/// procedures than the pin claims: the tool moved up. This describes what
/// happened to the tool, never to the file.
pub(crate) const MOVED_UP: &str = "MOVED UP";
```
Per `AGENTS.md`'s Measurement rule ("when the ratio moves, the failure
message gives the new number"), the new form/control ratio assertions must
print the measured number in the failure message the same way; the planner
should look at `crates/deform6/tests/ratios.rs`'s own comparison function
(not fully quoted here, found immediately below `HEADER` in that file) for
the exact `assert_eq!`/format shape before writing the new one, since it is
long and this map already exceeds the excerpt budget for it.

**`tests/ratios.toml`'s own explanatory header, the shape the new columns'
own header comment should match** (from the file itself):
```
# The pinned procedure recovery ratio, one entry per corpus program.
#
# Object recovery is NOT pinned here. It is 105 of 105 across the whole
# corpus with no variance (D-11): a pin on it could never move, so it could
# never fail, and AGENTS.md calls a test that cannot fail worse than none.
```

---

## Shared Patterns

### `Region`/`Off`/`Rva`/`Va`, and why none of the three implement `Add`

**Source:** `crates/deform6/src/read/region.rs`, lines 1-60
**Apply to:** every file in this phase that reads bytes (`gui.rs`,
`controltree.rs`, `vbstr.rs`, `propstream.rs`, `frx.rs`, `ocx.rs`,
`controlinfo.rs`)
```rust
//! None of the three implements `Add`, `Sub`, `From<u32>` or `Deref`. Every
//! addition is a `checked_add` that takes a bare `u32` length. Adding two
//! offsets is meaningless, so no signature accepts it. `Va::to_rva` is the
//! only bridge between the three spaces, and it is a `checked_sub`.

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Off(u32);

impl Off {
    #[must_use]
    pub const fn checked_add(self, n: u32) -> Option<Self> {
        match self.0.checked_add(n) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }
}
```
This is the mechanism, not a convention: the type system itself refuses to
compile `off_a + off_b`. Any new offset arithmetic in phase 3 (the running
`.frx` blob cursor, the control block `Length + 2` sibling-jump, the
property-loop cursor) goes through `checked_add` on a bare `u32`, never `+`
between two `Off`/`Va` values.

### The `get`/`None` pattern, never `[]`

**Source:** `crates/deform6/src/vb/object.rs`, lines 313-321 (quoted above
under gui.rs)
**Apply to:** every byte and field read in every new `vb/*.rs` file.

### Refusal, Defect, DefectKind, Severity — the exact names

**Source:** `crates/deform6/src/error.rs`, lines 1-120 (and the file
continues past what was read here; the planner should read the rest for
`Severity` and any variant not shown)
```rust
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Site {
    pub offset: u32,
    pub rva: Option<u32>,
    pub structure: &'static str,
    pub field: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, thiserror::Error)]
pub enum DefectKind {
    #[error("expected {expected} at offset {offset:#x}, found {found:#x}")]
    BadMagic { offset: u32, expected: &'static str, found: u32 },

    #[error("offset {offset:#x} plus length {len:#x} overflows a u32")]
    OffsetOverflow { offset: u32, len: u32 },

    #[error("offset {offset:#x} is past the end of the {file_len} byte file")]
    PastEndOfFile { offset: u32, file_len: u64 },

    #[error("count {count} at offset {offset:#x} exceeds the {max} that the file can hold")]
    ImplausibleCount { offset: u32, count: u32, max: u32 },

    #[error("address {va:#x} at offset {offset:#x} is in no section")]
    UnmappedAddress { offset: u32, va: u32 },

    #[error("the section header at offset {offset:#x} overlaps the section that starts at {other:#x}")]
    SectionOverlap { offset: u32, other: u32 },

    #[error("the text at offset {offset:#x} has no nul terminator in the {limit} bytes that follow")]
    NoNulTerminator { offset: u32, limit: u32 },
    // (more variants follow past line 120; read the rest of error.rs before
    // adding a new DefectKind, since an existing one such as ImplausibleCount
    // or UnmappedAddress likely already fits the tiling-mismatch or
    // unreadable-pointer case phase 3 needs, rather than a new variant)
}
```
`gui.rs`'s tiling-mismatch refusal is a `Refusal` naming the byte offset in
its own static message string, following `object.rs`'s
`Refusal::Damaged("the file ends inside an Object element")` shape exactly.
An undecoded property (`opcodes.rs`, `ocx.rs`, `controlinfo.rs`'s
name-unavailable event) is a `Defect`, pushed to a `Vec<Defect>`, using
`DefectKind::UnreadablePointer` or a new, narrowly-scoped variant if none of
the existing ones fits — check the rest of `error.rs` first.

### Compile-fail proof scripts

**Source:** `scripts/prove-lint-wall.sh` (quoted above in relevant part),
`scripts/prove-region-wall.sh` (present at that path; not read in full for
this map, budget-limited, but its name and its sibling script's shape make
clear it proves the `Region`/`Off` type wall the same way `prove-lint-wall.sh`
proves the clippy lint wall: a probe file with one bad shape per line,
compiled, and the failure checked line-by-line, then removed).
**Apply to:** if phase 3 adds a new refused shape (for example, a compile-time
check that `Off + Off` cannot be written, or a new refused pattern the tiling
check introduces), extend one of these two scripts rather than writing a
third mechanism. `prove-lint-wall.sh`'s own structure (`EXPECTED` table of
`shape|lint|description`, a `PROBE` heredoc with `// shape: X` markers, a
`jq` pass over `cargo clippy --message-format json`) is the template to copy
if a new bad-shape-per-line probe is needed; `prove-region-wall.sh` should be
read directly by the planner before extending it, since this map did not
load its contents.

## No Analog Found

None. Every Wave 0 file has at least a role-match analog in `vb/object.rs`,
`vb/project.rs`, `vb/classify.rs`, or `tests/support/vbp.rs`.

## Metadata

**Analog search scope:** `crates/deform6/src/vb/`, `crates/deform6/tests/`,
`crates/deform6/tests/support/`, `scripts/`, `crates/deform6/src/error.rs`,
`crates/deform6/src/read/region.rs`
**Files read in full or in targeted, non-overlapping ranges:** `object.rs`
(full), `support/vbp.rs` (full), `error.rs` (lines 1-120), `region.rs`
(lines 1-60), `differential.rs` (grep-located line ranges only, not fully
read), `ratios.rs` (lines 1-140), `prove-lint-wall.sh` (full)
**Pattern extraction date:** 2026-09-09
