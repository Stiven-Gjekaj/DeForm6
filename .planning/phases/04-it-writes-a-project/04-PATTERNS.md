# Phase 4: It writes a project - Pattern Map

**Mapped:** 2026-09-12
**Files analyzed:** 9 new files, 1 modified file (`crates/deform6-cli/src/main.rs`)
**Analogs found:** 9 / 9

## Project rules every new file must follow (from AGENTS.md / CLAUDE.md)

- Write all prose (code, comments, docs, commit messages) in ASD-STE100
  Simplified Technical English: short sentences, active voice, present
  tense, no em-dash, no emoji.
- `#![forbid(unsafe_code)]` is set crate-wide in `crates/deform6/src/lib.rs:1`
  and `unsafe_code = "forbid"` is a workspace lint (`Cargo.toml:19-20`). No
  new file writes `unsafe`.
- Workspace clippy lints deny: `indexing_slicing`, `arithmetic_side_effects`,
  `unwrap_used`, `expect_used`, `panic`, `todo`, `unreachable`,
  `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`,
  `integer_division` (`Cargo.toml:21-31`). Every new writer must use checked
  arithmetic (`checked_add`, `saturating_add`) exactly as `BlobCursor::take`
  does, never a bare `+`.
- The gate before every commit: `cargo fmt --all --check && cargo clippy
  --all-targets -- -D warnings && cargo test --workspace`. Not a selection.
- One commit does one step. Code and its tests share a commit; documentation
  is a separate commit. Do not rename and change behavior in the same commit.
- Never iterate a `HashMap` to produce writer output or report items (source:
  04-RESEARCH.md Anti-Patterns, and `vb/mod.rs:37` / `vb/opcodes.rs:136`
  already show the crate's own precedent of confining `HashMap` to internal
  lookup only) — always an ordered `Vec`/`BTreeMap`, matching
  `crates/xtask/src/opcode_table.rs:26,77`'s own `BTreeMap` precedent for
  reproducible output.
- Commit messages: no attribution footer beyond what the session's own
  system reminder requires; a human is the writer of record.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/deform6/src/write/mod.rs` | module-declaration | transform | `crates/deform6/src/vb/mod.rs` (lines 1-33) | exact |
| `crates/deform6/src/write/model.rs` | model / transform | transform | `crates/deform6/src/vb/vbstr.rs` (decode/encode primitive) + `crates/deform6/src/vb/mod.rs` (`Report`-building tree) | role-match |
| `crates/deform6/src/write/values.rs` | transform (serializer) | transform | `crates/deform6/src/vb/propstream.rs` (`PropertyValue` enum + its match-based printer in `main.rs::print_property`) | exact |
| `crates/deform6/src/write/vbp.rs` | service (text emitter) | transform | `crates/deform6/src/vb/project.rs` (`Component`, `Declaration` reader, the mirror-image writer target) | role-match |
| `crates/deform6/src/write/frm.rs` | service (text + blob emitter) | streaming (cursor-driven) | `crates/deform6/src/vb/frx.rs` (`BlobCursor`, `extract_blob`) + `crates/deform6/src/vb/controltree.rs` (tree walk order) | exact |
| `crates/deform6/src/write/code.rs` | service (text emitter) | transform | `crates/deform6/src/vb/functyp.rs` (`Prototype`, `Argument`, `TypeEntry`, `DefaultValue`) | role-match |
| `crates/deform6/src/report.rs` | model / service (serializer) | transform | `crates/deform6/src/error.rs` (`Site`, `DefectKind`, `Severity`, `Defect`, already `serde::Serialize`) | exact |
| `crates/deform6/tests/extract_structural.rs` | test (integration) | batch | `crates/deform6/tests/corpus_sweep.rs` (corpus-wide sweep test) + `crates/deform6/tests/support/frm.rs` (independent reader) | exact |
| `crates/deform6-cli/src/main.rs` (add `Command::Extract`) | CLI / controller | request-response | same file, `Command::Inspect` + `run_inspect` + `exit_for` (lines 50-225) | exact |

## Pattern Assignments

### `crates/deform6/src/write/mod.rs` (module declaration)

**Analog:** `crates/deform6/src/vb/mod.rs:1-33`

**Module-set doc-comment pattern to copy** (lines 1-17):
```rust
//! The Visual Basic structures inside the executable.
//!
//! [`inspect`] is the whole public reading path. ...
//!
//! The eight phase 3 modules (...) are declared here as a set, in
//! one commit, for the same reason: each later plan in the phase then edits
//! only the one file it owns, and this file never becomes a merge point for
//! two plans in one wave.

pub mod classify;
pub mod controlinfo;
...
```
`write/mod.rs` must declare `pub mod model; pub mod values; pub mod vbp; pub
mod frm; pub mod code;` together in plan 04-01's commit, with the identical
"declared as a set, one commit, so this file never becomes a merge point"
rationale in its own doc comment (04-RESEARCH.md Architecture Patterns
already names this as the intended pattern, quoting `vb/mod.rs:13-17`
verbatim).

---

### `crates/deform6/src/write/model.rs` (04-01)

**Analog for the Windows-1252 write primitive:** `crates/deform6/src/vb/vbstr.rs:225-243`

**Read-side convention this file must invert exactly** (lines 225-231):
```rust
/// ASCII maps each byte to its own Latin-1 code point, the rule
/// `vb/object.rs::read_name` and `vb/controltree.rs::read_name` both use.
/// `String::from_utf8_lossy` is never used here: a byte in 0x80 to 0xFF
/// would become the replacement character and the text would be lost.
```
```rust
// crates/deform6/src/vb/vbstr.rs:245
let text = bytes.iter().copied().map(char::from).collect();
```
`model.rs`'s encoder must be the literal functional inverse: for each `char`
`ch` in a `&str`, if `ch as u32 <= 0xFF`, push `ch as u32 as u8`; otherwise
push `b'?'` and record the substitution. Never use a real Windows-1252 codec
(`encoding_rs` or similar) — see 04-RESEARCH.md "Alternatives Considered" for
why the two are incompatible in the `0x80`-`0x9F` range.

**Analog for the crate-set/doc-comment/module discipline:** `crates/deform6/src/lib.rs:1-19`
```rust
#![forbid(unsafe_code)]

//! DeForm6 reads a compiled Visual Basic 6 executable.
//! ...
//! Every input is untrusted. No function in this crate panics on any input.
```
`model.rs` is where the model tree (`ProjectModel`, per-form/per-object
trees), the 40-character name clamp, and the VB6-identifier legality check
live. Treat every recovered name the same way `vb/vbstr.rs` treats every
byte: never trust it, always bound it, and never panic on it (no
`unwrap`/`expect`/`panic!`, matching the workspace lint wall).

**Overflow/error idiom to copy:** `crates/deform6/src/vb/frx.rs:283-296`
(`BlobCursor::take`, quoted under 04-04 below) — every arithmetic operation
on a length or an offset in `model.rs` must be `checked_add`/`checked_sub`,
never a bare operator, and a checked-arithmetic failure returns
`Result<_, Refusal>` via `crate::error::damaged(...)`, not a panic.

---

### `crates/deform6/src/write/values.rs` (04-02)

**Analog:** `crates/deform6/src/vb/propstream.rs` (the `PropertyValue` enum
this file's serializer inverts) and `crates/deform6-cli/src/main.rs:671-713`
(`print_property`, the existing exhaustive match over every `PropertyValue`
variant).

**Core pattern to copy** (`main.rs:671-713`, exhaustive match, no wildcard arm):
```rust
fn print_property(indent: &str, property: &PropertyValue) {
    match property {
        PropertyValue::Byte { name, value } => println!("{indent}  {name} = {value}"),
        PropertyValue::Boolean { name, value } => println!("{indent}  {name} = {value}"),
        ...
        PropertyValue::Undecoded { opcode, offset, control_type, .. } => {
            println!(
                "{indent}  property opcode {opcode} at offset {offset:#x} on {control_type}: \
                 not decoded. Run with --opcode-table to supply one."
            );
        }
    }
}
```
`values.rs` must exhaustively match every `PropertyValue` variant with no
wildcard arm (same discipline `DefectKind::severity` uses in `error.rs:304`,
see below), formatting each typed variant per FILE-FORMATS.md's grammar
(`&H..&` for colour, bare decimal for plain numbers, `"..."` for text) and
**omitting the line entirely** for `Undecoded`/`BlobUnreadable`, recording
the omission as a report item instead of writing a placeholder — per
04-RESEARCH.md Pitfall 1 and 3, `Undecoded` is the *common* case (683 of 805
property records), not an edge case, and no comment may go inside a `Begin`
block.

**Exhaustive-match-with-no-wildcard idiom to copy:** `crates/deform6/src/error.rs:297-361`
(`DefectKind::severity`):
```rust
/// The `match` has one arm for each variant and no wildcard arm. A new
/// variant therefore fails to compile until somebody decides its
/// severity. A wildcard arm lets a new variant take a default in silence.
#[must_use]
pub const fn severity(&self) -> Severity {
    match *self {
        Self::BadMagic { .. } => Severity::Fatal,
        ...
    }
}
```
Apply the same "no wildcard, a new variant is a compile error" discipline to
`values.rs`'s formatter and to the colour/enum/plain-number name-keyed hint
table 04-RESEARCH.md Pitfall 3 and Assumption A2 call for.

---

### `crates/deform6/src/write/vbp.rs` (04-03)

**Analog:** `crates/deform6/src/vb/project.rs` (`Component`, `Declaration`,
`ComponentTable`, `DeclareTable`, `ProjectInfo` — the exact reader this file
mirrors) and `crates/deform6-cli/src/main.rs:552-573` (`print_declarations`,
today's only place that walks `Report.declarations` in order and formats a
line per entry).

**Ordered-iteration pattern to copy** (`main.rs:558-572`):
```rust
for declaration in &report.declarations {
    let export = match &declaration.export {
        ExportName::Name(name) => name.clone(),
        ExportName::OrdinalInferred(ordinal) => format!("#{ordinal} (inferred alias)"),
    };
    println!("  {}!{export}", declaration.library);
    ...
}
```
`vbp.rs` walks `Report.objects`/`Report.components` (already ordered `Vec`
fields per `vb/mod.rs:292,300`) in that same declared order to emit
`Form=`/`Module=`/`Class=`/`Object=` lines — never a `HashMap`. Per
FILE-FORMATS.md §1.1 (cited in 04-RESEARCH.md), `.vbp` has no space around
`=` and no per-line indent: this is the one format in Pattern 3's table with
the flattest grammar, so `vbp.rs` must not share a line-template helper with
`frm.rs`/`code.rs`.

---

### `crates/deform6/src/write/frm.rs` (04-04, `.frm` + `.frx` as one component)

**Analog:** `crates/deform6/src/vb/frx.rs:244-296` (`FRX_ITEM_HEADER_LEN`,
`BlobCursor`) — reuse directly, do not re-derive.

**Exact code to reuse, not copy-and-modify** (`frx.rs:244-296`):
```rust
pub const FRX_ITEM_HEADER_LEN: u32 = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlobCursor {
    offset: u32,
}

impl BlobCursor {
    #[must_use]
    pub const fn new() -> Self {
        Self { offset: 0 }
    }

    pub fn take(&mut self, blob: &Blob) -> Result<u32, Refusal> {
        let current = self.offset;
        let Some(next) = current
            .checked_add(blob.declared_len)
            .and_then(|v| v.checked_add(FRX_ITEM_HEADER_LEN))
        else {
            return Err(damaged(format!(
                "the .frx offset cursor at {current} overflows a u32 advancing past this blob"
            )));
        };
        self.offset = next;
        Ok(current)
    }
}
```
`frm.rs` calls `BlobCursor::new()` once per form and `BlobCursor::take` once
per blob-carrying property, in the exact order the `.frm` writer emits
those properties — never computes an `.frx` offset any other way. This is
the one named risk 04-RESEARCH.md and ROADMAP.md both flag: `blobLen + 4`,
never `blobLen + 12`.

**Control-tree walk order to copy:** `crates/deform6/src/vb/controltree.rs`
(the reader's own parent/depth walk that `main.rs:602-626`'s
`control_depth`/`print_control` already consumes) — `frm.rs` must walk
`FormReport.controls` in the same order the reader produced them (already a
`Vec` with `parent: Option<usize>` links), reproducing nesting depth from
that same `parent` chain, not by re-deriving depth independently.

**Line-template grammar to copy (verbatim from research, not from existing
Rust code — no writer exists yet):**
```
<indent><name padded to 16><=><3 spaces><value><CRLF>

   BackColor       =   &H80000005&
   BorderStyle     =   1  'Fixed Single
   Caption         =   "Image Curves - tannerhelland.com"
```
3-space indent per depth level, `Begin`/`BeginProperty` end with exactly one
trailing space, `End`/`EndProperty` with none (FILE-FORMATS.md §2.4-2.5,
corpus-proved on 5899 lines, cited in 04-RESEARCH.md).

**Pitfall-2 defect check to copy:** `crates/deform6/src/vb/mod.rs:135-139`'s
own doc comment on `FormReport` (a form whose `controls` list is empty and
whose control-tree walk refused still gets a `.frm`, with every fact for it
reported `unrecoverable`, distinguished from a genuinely empty form only by
checking `FormReport.defects` for `DefectKind::StructureUnreadable`).

---

### `crates/deform6/src/write/code.rs` (04-05)

**Analog:** `crates/deform6/src/vb/functyp.rs:151-352` (`Prototype`,
`Argument`, `TypeEntry`, `DefaultValue`) and
`crates/deform6-cli/src/main.rs:352-437` (`format_prototype`,
`format_argument`, `format_type_entry`, `format_vb_type`, `format_default` —
the existing formatter this writer's signature-line emitter mirrors).

**Core pattern to copy** (`main.rs:365-387`, argument formatting):
```rust
fn format_argument(arg: &Argument) -> String {
    let mut prefix = String::new();
    if arg.entry.optional {
        prefix.push_str("Optional ");
    }
    if arg.entry.by_ref {
        prefix.push_str("ByRef ");
    }

    let mut piece = format!("{prefix}{}", arg.name);
    if arg.entry.array {
        piece.push_str("()");
    }
    piece.push_str(" As ");
    piece.push_str(&format_type_entry(&arg.entry));
    if let Some(default) = &arg.default {
        piece.push_str(" = ");
        piece.push_str(&format_default(default));
    }
    piece
}
```
`code.rs` reuses this exact join order (`Optional`/`ByRef` prefix, name,
`()` for array, `As <type>`, `= <default>`) to build
`Public Function mySub(Arg1 As Long, V As Variant) As Long` signature lines,
followed by an empty body and the matching `End Sub`/`End Function`/`End
Property` — WRT-07 asks for exactly this, no statement recovery.

**Attribute preamble grammar:** FILE-FORMATS.md §5.1-5.2 (corpus-proved on 7
`.bas` and 46 `.cls` files) — no existing Rust code emits this yet; treat it
as new, research-sourced text, and keep it in its own function so `vbp.rs`
and `frm.rs` never share a line-template helper with it (Pattern 3/Pitfall 4).

---

### `crates/deform6/src/report.rs` (04-06)

**Analog:** `crates/deform6/src/error.rs` (whole file) — `Site`,
`DefectKind`, `Severity`, `Defect` already derive `serde::Serialize` and
already carry a byte offset on every variant.

**Serialize-derive pattern to copy** (`error.rs:27-37`):
```rust
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Site {
    pub offset: u32,
    pub rva: Option<u32>,
    pub structure: &'static str,
    pub field: &'static str,
}
```
`report.rs`'s own `ReportItem`/`Confidence`/`Evidence` types derive
`serde::Serialize` the same way, and `Report.defects: Vec<Defect>` (already
`Serialize`, `vb/mod.rs:313`) plugs directly into the new report's defect
array (RPT-05) with no re-derivation.

**Exhaustive-match-no-wildcard discipline to copy:** `error.rs:297-361`
(`DefectKind::severity`, quoted above under `values.rs`) — the `Confidence`
enum (`Proven`/`Inferred`/`Unrecoverable`, RPT-03) and any match building a
`ReportItem` from a `PropertyValue` must have no wildcard arm.

**Determinism idiom to copy:** `crates/xtask/src/opcode_table.rs:26,77`
(`BTreeMap`, not `HashMap`, for reproducible output) — cited directly in
04-RESEARCH.md's own "Deterministic JSON output" code example. `report.rs`
must build its flat item array from ordered `Vec` fields only
(`Report.objects`, `Report.forms`, etc.), never from a `HashMap` iteration,
and use `serde_json::to_string_pretty` (or `to_writer_pretty`) on a struct,
never build JSON text by hand.

---

### `crates/deform6/tests/extract_structural.rs` (04-09)

**Analog:** `crates/deform6/tests/corpus_sweep.rs` (whole-corpus sweep
pattern) and `crates/deform6/tests/support/frm.rs` (the independent `.frm`
reader this test must reuse, not re-derive).

**Corpus-walk pattern to copy** (`corpus_sweep.rs:18-58`):
```rust
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

fn executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(&corpus_root(), &mut out);
    out.sort();
    out
}
```
`extract_structural.rs` walks the same `corpus/` root, runs `extract` (or
its library entry point) over each of the 44 programs into a temp `-o`
directory, then re-reads each written `.frm` through
`tests/support/frm.rs`'s own `Form`/`Block`/`parse_blocks` (already used by
`differential.rs`) — the independent second reader closing the loop the
roadmap's success criterion 3 names.

**Independent-reader discipline to copy** (`support/frm.rs:1-20`):
```rust
//! A second, independent `.frm` reader.
//!
//! **This file names nothing from `deform6`.** No `use deform6::...`, no
//! shared type, no shared constant. ...
```
The structural check must not import any type from `write::frm` to verify
`write::frm`'s own output; it uses `support/frm.rs`, exactly as
`differential.rs` already does for the read side, so a bug shared between
writer and checker cannot look like a pass.

**Test-file lint-allow header to copy** (`corpus_sweep.rs:1-8`):
```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
```

---

### `crates/deform6-cli/src/main.rs` (04-08, add `Command::Extract`)

**Analog:** the same file's existing `Command::Inspect` + `run_inspect` +
`exit_for` (lines 50-225).

**Subcommand-declaration pattern to copy** (`main.rs:50-70`):
```rust
#[derive(clap::Subcommand)]
enum Command {
    /// Reads one executable and prints what DeForm6 found in it.
    Inspect {
        input: PathBuf,
        #[arg(long)]
        opcode_table: Option<PathBuf>,
    },
}
```
Add `Extract { input: PathBuf, #[arg(short, long)] output: PathBuf, #[arg(long)]
report: Option<PathBuf>, #[arg(long)] force: bool }` beside `Inspect`, taking
paths (not strings) for the same not-valid-UTF-8 reason `Inspect::input`
already documents.

**Locked exit-code table to extend, not replace** (`main.rs:6-15`):
```rust
//! | Code | Meaning |
//! |---|---|
//! | 0 | The file was read |
//! | 1 | Not a PE file |
//! | 2 | A PE file, but it holds no Visual Basic runtime |
//! | 3 | Visual Basic, but not version 6 |
//! | 4 | Visual Basic 6, but damaged |
//! | 5 | An internal error, including a usage error |
```
`extract`'s own new failure modes (`-o` path already exists without
`--force`; `--report` path unwritable; a recovered name that would escape
`-o <dir>`) all map to `Exit::Internal` (5), the same usage-error bucket
`load_opcode_table`'s own failures already use — never a new exit code, the
doc comment states "the numbering must never move".

**Error-mapping match-with-no-wildcard idiom to copy** (`main.rs:216-225`):
```rust
fn exit_for(refusal: deform6::Refusal) -> Exit {
    match refusal {
        deform6::Refusal::NotPe | deform6::Refusal::NotI386 | deform6::Refusal::NotPe32 => {
            Exit::NotPe
        }
        deform6::Refusal::NoVbRuntime { .. } => Exit::NoVbRuntime,
        deform6::Refusal::IsVb5 | deform6::Refusal::IsVb4 => Exit::NotVb6,
        deform6::Refusal::Damaged(_) => Exit::Damaged,
    }
}
```
`run_extract` reuses `exit_for` unchanged for the `inspect` step, and adds
its own small match (also no wildcard arm) for the write-side's own new
failure modes (path escape, `--force` missing, unwritable report path) onto
`Exit::Internal`.

**File-read-then-dispatch pattern to copy** (`main.rs:186-210`,
`run_inspect`):
```rust
fn run_inspect(path: &Path, opcode_table_path: Option<&Path>) -> Exit {
    let (table, table_summary) = match load_opcode_table(opcode_table_path) {
        Ok(loaded) => loaded,
        Err(exit) => return exit,
    };
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("could not read {}: {err}", path.display());
            return Exit::Internal;
        }
    };
    match deform6::inspect(&data, &table) {
        Ok(report) => { print_report(path, &report, &table_summary); Exit::Ok }
        Err(refusal) => { eprintln!("{refusal}"); exit_for(refusal) }
    }
}
```
`run_extract` follows the identical shape: load the opcode table, read the
file into `data` (kept alive for the whole run per 04-RESEARCH.md Pitfall
5 — the `.frx` writer re-reads blob bytes out of these same original
`data` bytes), call `deform6::inspect`, then on `Ok(report)` build the
`ProjectModel` and drive the five writers plus the report, on `Err(refusal)`
reuse `exit_for` unchanged.

## Shared Patterns

### Exhaustive match, no wildcard arm
**Source:** `crates/deform6/src/error.rs:297-361` (`DefectKind::severity`),
`crates/deform6-cli/src/main.rs:216-225` (`exit_for`)
**Apply to:** `write/values.rs`'s property formatter, `report.rs`'s
`Confidence`-building match, any match over `PropertyValue`, `ControlKind`,
or `Refusal` a new file adds.
```rust
/// The `match` has one arm for each variant and no wildcard arm. A new
/// variant therefore fails to compile until somebody decides its
/// severity. A wildcard arm lets a new variant take a default in silence.
```

### Checked arithmetic for every offset/length computation
**Source:** `crates/deform6/src/vb/frx.rs:283-291` (`BlobCursor::take`)
**Apply to:** every byte-offset or length computation in `write/frm.rs`,
`write/model.rs`, and `report.rs`.
```rust
let Some(next) = current
    .checked_add(blob.declared_len)
    .and_then(|v| v.checked_add(FRX_ITEM_HEADER_LEN))
else {
    return Err(damaged(format!(
        "the .frx offset cursor at {current} overflows a u32 advancing past this blob"
    )));
};
```

### Never a `HashMap` for output-shaping iteration
**Source:** `crates/xtask/src/opcode_table.rs:26,77` (`BTreeMap` for
reproducible TOML output); contrast `crates/deform6/src/vb/mod.rs:37` and
`opcodes.rs:136`, which confine `HashMap` to internal lookup only.
**Apply to:** `write/vbp.rs`, `write/frm.rs`, `write/code.rs`, `report.rs` —
every one of these must iterate an ordered `Vec` (`Report.objects`,
`Report.forms`, `FormReport.controls`, `PropertyStream.properties`) to
produce deterministic byte-identical output (RPT-05/roadmap success
criterion 5).

### The Latin-1-as-codepoint convention, not a real Windows-1252 codec
**Source:** `crates/deform6/src/vb/vbstr.rs:225-231,245`
**Apply to:** `write/model.rs`'s CRLF/Windows-1252 emitter, used by every
one of the five text writers.
```rust
let text = bytes.iter().copied().map(char::from).collect();
```
Inverted for the write side: `byte = ch as u32 as u8` when `ch as u32 <=
0xFF`, else substitute `?` and report it. Never `encoding_rs` or any
standards-correct Windows-1252 codec.

### `#![forbid(unsafe_code)]` and no panics on hostile input
**Source:** `crates/deform6/src/lib.rs:1,8`
**Apply to:** every file in this phase. `crates/deform6/src/write/*.rs` and
`crates/deform6/src/report.rs` sit inside the same crate this forbid
attribute already covers; no new file adds its own `#[allow(unsafe_code)]`.
Test files (`tests/extract_structural.rs`) are the one place
`unwrap`/`expect`/`panic` are locally allowed, matching
`corpus_sweep.rs:1-8`'s own `#![allow(...)]` header, and only there.

### The independent second reader, never the writer checking itself
**Source:** `crates/deform6/tests/support/frm.rs:1-20`
**Apply to:** `tests/extract_structural.rs` — reuse `support/frm.rs`
unchanged; do not write a new `.frm` reader and do not call into
`write::frm` to verify `write::frm`'s own output.

## No Analog Found

| File/Section | Role | Data Flow | Reason |
|---|---|---|---|
| The `.vbp`/`.frm`/`.cls` line-template grammars themselves | transform | transform | No writer of any kind exists yet in this crate; the grammar is sourced from `.planning/research/FILE-FORMATS.md` §1-§5, corpus-measured but not yet expressed as Rust. Use the Code Examples in `04-RESEARCH.md` directly. |
| The 40-character name clamp and VB6-identifier legality check | utility | transform | New logic; no existing reader enforces an output-side length clamp (the reader only ever reads a declared length, never re-derives or shortens a name). Build fresh in `model.rs`, following the checked-arithmetic and no-panic conventions above. |
| The colour/enum/plain-number name-keyed formatting-hint table (Pitfall 3 / Assumption A2) | model / lookup table | transform | `PayloadType` (`opcodes.rs:58-82`) has no `Colour`/`Enum` variant; this table is new, additive, and deliberately kept out of `opcodes.rs` per 04-RESEARCH.md Assumption A2. |

## Metadata

**Analog search scope:** `crates/deform6/src/vb/*.rs`, `crates/deform6/src/error.rs`,
`crates/deform6/src/lib.rs`, `crates/deform6-cli/src/main.rs`,
`crates/deform6/tests/*.rs`, `crates/deform6/tests/support/frm.rs`,
`crates/xtask/src/opcode_table.rs`, both `Cargo.toml` files.
**Files scanned:** 9 source files read directly this session, plus the full
`04-RESEARCH.md` and the Phase 4 `ROADMAP.md` section.
**Pattern extraction date:** 2026-09-12
