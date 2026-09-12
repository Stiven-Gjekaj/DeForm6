# Phase 4: It writes a project - Research

**Researched:** 2026-09-12
**Domain:** Writing a VB6 IDE-loadable project directory (`.vbp`, `.frm`, `.frx`, `.bas`, `.cls`) plus a deterministic JSON confidence report, from the `deform6::vb::Report` model Phases 1-3 already build.
**Confidence:** HIGH on the on-disk grammar (measured against a 44-program, 121-file corpus by an earlier session of this same project) and on what Phase 1-3 code already exposes (read directly, this session). MEDIUM-LOW on several named gaps (list records, non-Latin code pages, `.ctl`/`.ctx`, MDIForm) that no corpus file exercises.

## Summary

Phase 4 is a pure consumer of `deform6::vb::Report`, the value `deform6::inspect()` already returns (`crates/deform6/src/vb/mod.rs:249-314`). Nothing in Phase 4 re-reads the executable; every fact the writer needs is already a field on `Report`, `FormReport`, `ControlReport`, `PropertyValue`, `Component`, `Declaration`, or `Prototype`. This reframes the whole phase: it is a **serialization** problem (recovered model -> VB6 text grammar, and recovered model -> JSON), not a parsing problem. The two on-disk formats are already fully documented in this repository at corpus-measured precision (`.planning/research/FILE-FORMATS.md`, `.planning/research/STRUCTURES.md` §8, §12-15), and the `.frx` write cursor is already implemented and tested (`crates/deform6/src/vb/frx.rs`, `BlobCursor`).

The single fact every plan in this phase must treat as load-bearing: **`FRX_ITEM_HEADER_LEN = 4`**, not 12, and the `.frm`/`.frx` writer is one component (roadmap plan 04-04) because the offset a `.frm` line names is not stored anywhere - it is the running total `BlobCursor` produces, and any writer that computes it independently of that exact struct will drift silently, producing a `.frm` that references a byte range past the end of its own `.frx`.

The second fact every plan must internalize: the majority of control properties are **not decoded**. `REQUIREMENTS.md`'s own FRM-03 status states 122 named property values recovered against 683 records that report "present, not decoded" out of 805 total records across five distinct property names. `PropertyValue::Undecoded` (`crates/deform6/src/vb/propstream.rs:166-185`) is therefore the *common* case a property-serializing plan (04-02) must handle, not an edge case, and per FILE-FORMATS.md §2.6 an apostrophe comment cannot go inside a `Begin` block, so an undecoded property cannot be written as `PropertyName = ??? 'not decoded`. The only legal moves are: omit the line (let the IDE default it) and record the omission in the JSON report, or refuse writing this control's line and record why - the report is the only place doubt can be expressed for anything inside a `Begin` block.

**Primary recommendation:** Build `model.rs` (04-01) as a translation layer from `deform6::vb::Report` to a plain, `serde`-friendly tree that owns no reference into the original bytes; drive the `.frm`/`.frx` pair from one shared `BlobCursor` per form exactly as `vb/frx.rs` already implements it; add `serde_json` as a new workspace dependency (verified OK on the crates.io registry, already paired with the `serde` dependency this workspace carries); and never introduce a general-purpose Windows-1252 codec crate, because the whole crate already commits to a specific, narrower convention (`char::from(byte)`, i.e. each byte is its own Latin-1 code point) that a "correct" Windows-1252 decoder for the 0x80-0x9F range would silently disagree with on the write side.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Recovered-model translation (`Report` -> write-side tree) | Library (`deform6` crate, new `write` module) | - | `deform6-cli` owns no domain logic today (`main.rs`'s own doc comment: the CLI maps errors to exit codes and prints); the library is the one place `Report`'s internal shape is visible. |
| `.vbp`/`.frm`/`.frx`/`.bas`/`.cls` text emission | Library (`deform6::write::*`) | - | Pure functions over the model, no I/O: matches `inspect`'s own "opens no file, writes no file" discipline (`vb/mod.rs:3-6`), so the Phase 5 fuzz target can drive the writer in-memory the same way it drives the reader. |
| Filesystem I/O (`-o <dir>`, `--force`, writing bytes) | CLI (`deform6-cli`) | - | `main.rs`'s own doc comment: "the library never opens a file... The command line crate owns the file system." Unchanged convention from `run_inspect`. |
| JSON report serialization | Library (`deform6::report`) | CLI (writes the file) | Same split as above: the library builds the `serde::Serialize` tree, the CLI writes bytes to `-o/<name>.report.json` (or `--report <path>`). |
| Structural recompilation check | Test harness (`crates/deform6/tests/`, reusing `tests/support/frm.rs`) | - | `AGENTS.md` "What a test may hold on to": the check is a second, independent reader over the *written* output, mirroring how `differential.rs` already reads the *original* corpus through `support/frm.rs`. Full VB6 IDE recompilation is out of scope for this CI (no Windows/VB6 host). |

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `serde_json` | crates.io | 2015-08-07, ~11 years | 24,058,123/week | github.com/serde-rs/json | OK | Approved |

Verified via `gsd_run query package-legitimacy check --ecosystem crates serde_json` this session: `{"name":"serde_json","verdict":"OK","signals":{"exists":true,"publishedAt":"2015-08-07T19:04:18.632088Z","weeklyDownloads":24058123,"repoUrl":"https://github.com/serde-rs/json","deprecated":false,"postinstall":null}}` [VERIFIED: gsd-tools package-legitimacy seam]. Cross-checked against the crates.io registry directly with `cargo search serde_json`, which returned `serde_json = "1.0.151"` [VERIFIED: crates.io registry, `cargo search`, this session]. `serde_json` is maintained by the same `serde-rs` organization whose `serde` and `serde_derive` crates this workspace already depends on (`Cargo.toml:9-10`), so adding it introduces no new maintainer trust boundary.

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none.

No other new external package is needed. See "Standard Stack" below for why a Windows-1252 encoding crate is explicitly *not* recommended, despite being the obvious first idea for WRT-06.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde_json` | 1.0.151 (current on crates.io, `cargo search` this session) [VERIFIED: crates.io registry] | Serializes the Phase 4 confidence report to JSON | Pairs directly with the `serde` 1.0.229 dependency this workspace already declares with the `derive` feature (`Cargo.toml:10`) [VERIFIED: `Cargo.toml:10`, quoted: `serde = { version = "1.0.229", features = ["derive"] }`]. `error.rs` already derives `serde::Serialize` on `Site`, `DefectKind`, `Severity` and `Defect` (`crates/deform6/src/error.rs:27`, `:49`, `:289`, `:377`) [VERIFIED: `crates/deform6/src/error.rs:27,49,289,377`], so the report's evidence trail already has a `Serialize` impl waiting to be used; only the write-side model types (Phase 4's own) need new derives. |

### Supporting

None. No dependency is needed for Windows-1252 encoding, CRLF writing, or VB6 identifier validation; all three are a few lines of hand-written code (see "Don't Hand-Roll" and "Code Examples").

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-written Windows-1252 encoder (inverse of `char::from(byte)`) | `encoding_rs` crate | `encoding_rs`'s `WINDOWS_1252` encoder is a real, standards-correct Windows-1252 codec. But every reader in this crate (`vb/object.rs`, `vb/gui.rs`, `vb/controltree.rs`, `vb/vbstr.rs`, `vb/project.rs`, `vb/header.rs`, `vb/functyp.rs`, `vb/privateobj.rs`, `vb/frx.rs`, `crates/deform6/tests/support/frm.rs`) decodes with `bytes.iter().copied().map(char::from).collect()` [VERIFIED: `crates/deform6/src/vb/vbstr.rs:245`, quoted: `let text = bytes.iter().copied().map(char::from).collect();`] - this is a **Latin-1-as-codepoint** mapping, not real Windows-1252. Real Windows-1252 maps byte `0x80` to U+20AC (€); `char::from(0x80u8)` gives U+0080 (a C1 control code). A "correct" `encoding_rs` encoder fed a `String` that came from this crate's own reader would encode a genuine U+20AC character back to `0x80` correctly, but it would reject or mis-encode a `char::from`-decoded `\u{80}` differently than the reader that produced it expects, because the reader never produced a real €. Using a standards-correct crate for the write side, paired with a non-standard read side, is a two-codec system that silently disagrees on the one byte range (`0x80`-`0x9F`) where Latin-1 and Windows-1252 diverge. The corpus only ever exercises `0xA9` and `0xAE` (`.planning/research/FILE-FORMATS.md` §6.1: "Two distinct non-ASCII bytes appear: `0xA9` in 7 `.vbp` files... and `0xAE` in 20 `.cls` files") [CITED: `.planning/research/FILE-FORMATS.md` §6.1], both of which are identical in Latin-1 and Windows-1252, so this divergence has never been exercised - but a hand-written encoder that inverts `char::from(byte)` exactly (write `byte` back for any `char` in `\u{0}..=\u{FF}`, substitute and report anything above) is both simpler and provably round-trip-consistent with every existing reader. |
| Manual JSON string-building | `serde_json` | Manual building of quoted/escaped JSON strings is exactly the kind of "deceptively complex" text-escaping problem `AGENTS.md`'s prior-art principle warns about (get it from a real library, don't hand-roll escaping); the size and trust of `serde_json` outweigh the cost of one added dependency. |

**Installation:**
```toml
# crates/deform6/Cargo.toml, [dependencies]
serde_json.workspace = true

# root Cargo.toml, [workspace.dependencies]
serde_json = "1.0"
```

**Version verification:** `cargo search serde_json` was run this session and returned `serde_json = "1.0.151"` [VERIFIED: crates.io registry, `cargo search`, this session]. No `pip`/`npm` verification applies; this is a Rust workspace (`Cargo.toml:1-3`, `members = ["crates/deform6", "crates/deform6-cli", "crates/xtask"]`).

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| WRT-01 | `deform6 extract <exe> -o <dir>` writes a project directory | See "Architecture Patterns", "extract subcommand" below; mirrors the existing `Inspect` subcommand shape in `crates/deform6-cli/src/main.rs:50-70` and the locked exit-code table at `main.rs:6-14`. |
| WRT-02 | `.vbp` lists every object and control dependency | `Report.objects`, `Report.components` already carry this (`vb/mod.rs:292,300`); `.planning/research/FILE-FORMATS.md` §1 gives the exact `.vbp` grammar, corpus-proved on 32 files. |
| WRT-03 | `.frm` byte-level layout (3-space indent, 16-column name pad, `=` + 3 spaces, trailing-space rules) | `.planning/research/FILE-FORMATS.md` §2.4-§2.5, §3.1, corpus-proved on 487 `Begin` blocks and 5899 property lines. |
| WRT-04 | Alphabetical property order within a block, menus last | `.planning/research/FILE-FORMATS.md` §2.5, corpus-proved with 0 violations across 487 blocks in 36 files. |
| WRT-05 | `.bas`/`.cls` attribute preamble | `.planning/research/FILE-FORMATS.md` §5.1-§5.2, corpus-proved on 7 `.bas` and 46 `.cls` files (one MD5 over the normalised preamble of all 46). |
| WRT-06 | CRLF and code page | `.planning/research/FILE-FORMATS.md` §6, corpus-proved on all 121 text files; see "Windows-1252 and CRLF" in Code Examples for the exact hand-rolled emitter, matching this crate's existing `char::from(byte)` read convention. |
| WRT-07 | Empty-body procedure with correct signature | `deform6::vb::functyp::Prototype`, `Argument`, `TypeEntry`, `DefaultValue` (`crates/deform6/src/vb/functyp.rs:151-352`) already carry everything a signature line needs; see "Code Examples". |
| RPT-01 | One JSON report beside the project | See "Standard Stack" (`serde_json`) and "Deterministic JSON output" below. |
| RPT-02 | Flat array of items keyed by path | New model in `report.rs`; no existing type matches this shape (`Report` is a tree, not a flat array) - this is genuinely new code, not reused code. |
| RPT-03 | Confidence is one of three named values, never a number | New enum, e.g. `Confidence { Proven, Inferred, Unrecoverable }` (three variants named to match the roadmap's own vocabulary: "proven" is implied by the exact byte offset a `.frm` line's value came from, "inferred" is the `jq` example in the roadmap's own success criterion 5, and a third state is needed for `PropertyValue::Undecoded`/`BlobUnreadable`/`UnrecoverableString`). |
| RPT-04 | `basis` and at least one `evidence` record with a byte offset | `Defect.site.offset` (`error.rs:30`) and `PropertyValue::Blob.offset`/`Text.offset`-equivalent fields already carry byte offsets; every `PropertyValue` variant that names an `offset` field can supply this directly. |
| RPT-05 | Defect array, whether or not the run continued | `Report.defects: Vec<Defect>` (`vb/mod.rs:313`) already collects exactly this, and `Defect`/`Site`/`DefectKind` already derive `serde::Serialize`. |
| RPT-06 | Uncertainty comment in code regions only | `.planning/research/FILE-FORMATS.md` §2.6 proves (0 of 7379 header lines are comments or blank) that a `Begin` block cannot carry one; the code region after `Attribute VB_Exposed` can (§2.7: "From there the file is ordinary Basic source, and an apostrophe comment is legal"). |

</phase_requirements>

## Architecture Patterns

### System Architecture Diagram

```
                    +---------------------------+
   <exe> bytes ---> |  deform6::inspect(data)   |   (Phases 1-3, unchanged)
                    |  -> Result<Report, Refusal>|
                    +-------------+-------------+
                                  |
                                  v
                    +---------------------------+
                    | write::model::from_report |   (Plan 04-01)
                    | Report -> ProjectModel      |   pure translation, no I/O
                    +-------------+-------------+
                       |          |            |
                       v          v            v
              +-----------+ +-----------+ +-----------+
              | write::vbp| | write::frm| |write::code|   (Plans 04-03, 04-04, 04-05)
              | .vbp text | | .frm+.frx | | .bas/.cls |
              +-----------+ | ONE shared| +-----------+
                       |    | BlobCursor|      |
                       |    | per form  |      |
                       |    +-----------+      |
                       |          |            |
                       +----------+------------+
                                  |
                                  v
                    +---------------------------+
                    |    report::build(model)    |   (Plan 04-06)
                    | flat item array, evidence, |
                    | defects, deterministic     |
                    +-------------+-------------+
                                  |
             CLI (deform6-cli) writes bytes to disk:
                    -o <dir>/*.vbp,*.frm,*.frx,*.bas,*.cls
                    -o <dir>/<name>.report.json  (or --report <path>)
                                  |
                                  v
                    +---------------------------+
                    | structural recompilation   |   (Plan 04-09)
                    | check: reads the WRITTEN   |
                    | tree back through          |
                    | tests/support/frm.rs, a    |
                    | second independent reader  |
                    +---------------------------+
```

A reader can trace `.exe` -> `Report` -> `ProjectModel` -> five file kinds + one report -> the structural check that closes the loop, following the arrows above. `.frm` and `.frx` are drawn as one box because roadmap plan 04-04 makes them one component: the `.frx` byte offset a `.frm` line names does not exist anywhere until the writer synthesizes it (`.planning/research/STRUCTURES.md` §8.8, and see "The `.frx` write cursor" below), so splitting the two into separate plans would let two independently-computed offsets drift out of step.

### Recommended Project Structure

```
crates/deform6/src/
├── vb/                    # unchanged: the reading side (Phases 1-3)
└── write/                 # new for Phase 4
    ├── mod.rs             # pub mod model; pub mod vbp; pub mod frm; pub mod code; declares the set (matches vb/mod.rs's own precedent, see below)
    ├── model.rs           # Plan 04-01: ProjectModel, the Windows-1252/CRLF emitter primitives, the 40-char clamp, the identifier check
    ├── values.rs          # Plan 04-02: property-value serialisation (int/string/bool/enum/colour/float/Font/blob-ref)
    ├── vbp.rs             # Plan 04-03: .vbp emitter
    ├── frm.rs             # Plan 04-04: .frm + .frx emitter, one shared BlobCursor
    └── code.rs            # Plan 04-05: .bas/.cls emitter
crates/deform6/src/
└── report.rs              # Plan 04-06: the JSON report model and builder (report.rs sits beside error.rs/journal.rs, not under write/, since it consumes both vb:: and write:: output)
crates/deform6-cli/src/
└── main.rs                # Plan 04-08: adds Command::Extract
crates/deform6/tests/
└── extract_structural.rs  # Plan 04-09: the structural recompilation check, reusing tests/support/frm.rs
```

Declaring all five `write/` submodules together in one commit, as a set, in `write/mod.rs`, follows the exact pattern `vb/mod.rs`'s own doc comment states for why it declares eight modules at once: *"each later plan in the phase then edits only the one file it owns, and this file never becomes a merge point for two plans in one wave"* [VERIFIED: `crates/deform6/src/vb/mod.rs:13-17`]. Waves `[04-01, 04-06]` then `[04-02, 04-03, 04-05]` then `[04-04, 04-07]` (per `ROADMAP.md`) match this: 04-01 must own `write/mod.rs`'s module declarations before 04-02/04-03/04-05 can each add their own file to it, the same ordering constraint `vb/mod.rs` already documents for 03-01 versus 03-02.

### Pattern 1: The `.frx` write cursor (one `BlobCursor` per form, shared by the `.frm` and `.frx` writer)

**What:** `BlobCursor` (`crates/deform6/src/vb/frx.rs:246-296`) already exists, is already tested against two real corpus `.frx` files, and is exactly the type Phase 4's writer needs - it was built by Phase 3 (plan 03-15) specifically so Phase 4 would not have to re-derive this fact.

**When to use:** Every time a `.frm` needs to write a `Picture=`, `Icon=`, `Text=` (multi-line), or `Caption=` (`$"..."`) reference. One `BlobCursor::new()` per form (never shared across forms - `frx.rs`'s own test `a_new_form_starts_its_cursor_at_zero_after_a_previous_form_advanced_it` proves this), called once per blob in the exact order the `.frm` writer emits the properties that reference the `.frx`.

**Example:**
```rust
// crates/deform6/src/vb/frx.rs:283-295 (VERIFIED, already shipped and tested)
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
```
`FRX_ITEM_HEADER_LEN` is `4`, verified: `pub const FRX_ITEM_HEADER_LEN: u32 = 4;` [VERIFIED: `crates/deform6/src/vb/frx.rs:244`]. This is the value the roadmap's named risk insists on: *"An earlier text here said `blobLen + 12`... Add 4 to the declared `blobLen`, never 12"* [VERIFIED: `.planning/ROADMAP.md`, Phase 4 named risks, quoted verbatim]. The writer for plan 04-04 must call `BlobCursor::take` for every blob-carrying property, in property-emission order, and write the `u32` it returns as the four-digit-minimum uppercase hex offset in the `.frm` (`.planning/research/FILE-FORMATS.md` §3.11: *"padded with leading zeros to at least four digits"* [CITED: `.planning/research/FILE-FORMATS.md` §3.11]) while separately appending the item's bytes (the length-field-plus-payload the `PropertyValue::Blob` already carries: `declared_len`, `header: [u8; 8]`, `image: Vec<u8>`) to the `.frx` file buffer.

**A subtlety `PropertyValue::Blob` does not carry:** `propstream.rs`'s own doc comment states the blob's raw bytes are deliberately *not* carried forward into the report today (*"A future `.frx` writer re-reads `offset..offset + 4 + declared_len` out of the executable's own bytes"* [VERIFIED: `crates/deform6/src/vb/propstream.rs:119-124`]). This means `PropertyValue::Blob { offset, declared_len, image_len, format, frx_offset, .. }` gives the writer everything needed to *locate* the bytes (the `offset` field is the absolute file offset of the blob's own four-byte length field in the original executable) but not the bytes themselves. **Plan 04-01's `model.rs` must decide whether to add an `image: Vec<u8>` field when building `ProjectModel` from `Report`** (re-reading `offset..offset+4+declared_len` out of the original `data: &[u8]` the writer must therefore also be given, alongside `Report`), or whether `write/frm.rs` itself re-reads the executable bytes at write time. Either way, **the extract subcommand needs the original executable bytes in memory for the whole run**, not just the `Report` - this is a real, previously-invisible input to the write path that plan 04-01 or 04-08 must thread through.

### Pattern 2: The Windows-1252/CRLF emitter is a small hand-rolled primitive, matching the crate's own read convention

**What:** A byte-level string writer that is the exact functional inverse of `char::from(byte)`.

**When to use:** Every text file this phase writes.

**Example:**
```rust
// Recommended shape for write/model.rs, not yet in the codebase.
// Mirrors the read side's own documented convention, e.g.:
// crates/deform6/src/vb/vbstr.rs:225-231 (read side, VERIFIED):
//   "ASCII maps each byte to its own Latin-1 code point... `String::from_utf8_lossy`
//    is never used here: a byte in 0x80 to 0xFF would become the replacement
//    character and the text would be lost."
fn encode_windows_1252(text: &str) -> (Vec<u8>, Vec<char>) {
    let mut bytes = Vec::with_capacity(text.len());
    let mut substituted = Vec::new();
    for ch in text.chars() {
        let code = ch as u32;
        if code <= 0xFF {
            // Inverts char::from(byte) exactly: round-trips with every
            // existing reader in this crate.
            #[allow(clippy::cast_possible_truncation)] // code <= 0xFF checked above
            bytes.push(code as u8);
        } else {
            bytes.push(b'?'); // FILE-FORMATS.md 6.1: "Replace the character,
                               // and record the substitution in the report."
            substituted.push(ch);
        }
    }
    (bytes, substituted)
}
```
CRLF: every line written by any of the five emitters ends `\r\n`, including the last line of the file, and no file begins with a byte order mark (`.planning/research/FILE-FORMATS.md` §6.2-§6.3, corpus-proved on all 121 text files: *"the last two bytes of all 121 files are `0D 0A`"*, *"the first three bytes of all 121 files are `Att`, `Typ`, or `VER`"* [CITED: `.planning/research/FILE-FORMATS.md` §6.2, §6.3]).

### Pattern 3: The `.vbp`/`.frm`/`.cls` layout grammars differ from each other - one writer per format, never a shared line-template function

**What:** Three visually similar but byte-distinct "name = value" grammars exist in this format family, and FILE-FORMATS.md explicitly warns against sharing a writer between them.

| Format | Indent per level | Name field width | Spaces after `=` | Comment lead-in |
|---|---|---|---|---|
| `.frm`/`.ctl` `Begin` block | 3 spaces | 16 | 3 | 2 spaces then `'` (boolean: 2 or 3 spaces depending on value width, see §3.4) |
| `.cls` `BEGIN`/`END` block | 2 spaces | 20 | 1 | 2 spaces then `'` |
| `.vbp` | none (flat `Key=Value`) | none | 0 | n/a |

All corpus-measured: `.planning/research/FILE-FORMATS.md` §2.5 (`.frm`, 5899 lines), §5.2 (`.cls`, 46 files, *"The `.cls` block therefore uses a different layout from the `.frm` block. An emitter that reuses one writer for both produces the wrong bytes"* [CITED: `.planning/research/FILE-FORMATS.md` §5.2]), §1.1 (`.vbp`, *"There is no space around the `=`"* [CITED: `.planning/research/FILE-FORMATS.md` §1.1]).

### Anti-Patterns to Avoid

- **A single generic "key = value" line writer shared across `.vbp`/`.frm`/`.cls`:** produces the wrong bytes for at least two of the three formats (see Pattern 3 above).
- **Deriving a `.frx` offset independently in `write/vbp.rs` or anywhere outside the one `BlobCursor` a form's `.frm`/`.frx` pair shares:** `.planning/research/STRUCTURES.md` §8.8 states this directly: *"any independent computation of an offset drifts."* [CITED: `.planning/research/STRUCTURES.md` §8.8]
- **Iterating a `HashMap` to emit report items or file lines:** `crates/deform6/src/vb/mod.rs:669` and `crates/deform6/src/vb/opcodes.rs:136` both use `HashMap` internally for lookup [VERIFIED: `crates/deform6/src/vb/mod.rs:669`, `crates/deform6/src/vb/opcodes.rs:136`]. Rust's default `HashMap` iteration order is randomized per process (`SipHash` with a random per-process seed), so iterating one directly to produce report or file output would silently break RPT-05's byte-identical-reports requirement on some runs and not others. Every writer and the report builder must only ever iterate the existing ordered `Vec` fields (`Report.objects`, `Report.forms`, `FormReport.controls`, `PropertyStream.properties`, etc.) - never a `HashMap`.
- **A real Windows-1252 codec crate for the write side:** see "Alternatives Considered" above.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON serialization, escaping, number formatting | A hand-written JSON writer | `serde_json` | Escaping and float formatting are exactly the class of "deceptively complex" text problem this project's own `AGENTS.md` prior-art discipline warns against re-deriving; `serde_json` is a 24M-download/week, `serde-rs`-maintained crate this workspace already trusts transitively through `serde`. |
| The `.frx` write cursor | A second offset-tracking implementation in the `.frm` writer | `crate::vb::frx::BlobCursor`, already shipped and tested | This exact struct exists to prevent the drift the roadmap's own named risk describes; re-deriving it is the one mistake the roadmap explicitly calls out. |

**Key insight:** Nearly everything else in this phase (the `.vbp`/`.frm`/`.cls` grammars, the Windows-1252 encoder, the CRLF writer, the identifier-length clamp, the property-alphabetization rule) is *not* a "don't hand-roll" case - it is domain-specific, corpus-measured, project-owned logic with no general-purpose library that could replace it. The two genuine "use a library" cases above are narrow and already identified.

## Common Pitfalls

### Pitfall 1: Treating `PropertyValue::Undecoded` as a rare edge case

**What goes wrong:** A `.frm` writer that assumes most properties decode cleanly and only has a fallback path for the occasional undecoded one will discover, only when run against the corpus, that undecoded properties are the *majority* case.
**Why it happens:** `REQUIREMENTS.md`'s own FRM-03 status: *"Phase 3 recovers 122 named property values against 683 records that report present and not decoded, over 805 property records and five distinct property names"* [VERIFIED: `.planning/REQUIREMENTS.md`, FRM-03, quoted verbatim]. Only ~15% of property records carry a name and value; the other ~85% carry only an opcode, an offset, and a byte count.
**How to avoid:** Design the `.frm` control-block writer around "for each property this control's stream decoded, write a line; for each one it did not, omit the line and add a report item with `confidence: "unrecoverable"` (or similar) naming the opcode, the control, and the byte offset" from the start, not as an afterthought.
**Warning signs:** A `.frm` writer whose test suite only exercises corpus programs with simple forms (few controls, no third-party OCX) will look correct while silently producing near-empty control blocks for anything more complex.

### Pitfall 2: Writing an empty `Begin ... End` block for a form whose control tree walk refused, indistinguishably from a form that is genuinely empty

**What goes wrong:** `corpus/public-domain/LockWorkStation/LockWorkStation.exe` genuinely has a form with zero children (`.planning/research/STRUCTURES.md` §8, controltree.rs test `lock_work_station_gives_one_root_form_with_zero_children_and_tiling_holds` [VERIFIED: `crates/deform6/src/vb/controltree.rs:1178-1184`]) - a legitimate, defect-free empty form. But `corpus/vb6-code/Map-editor-2D/Map Editor.exe`'s form `Main` refuses its control tree walk entirely (`.planning/WINDOWS.md` finding 8: *"the walk expects a scope separator (0xFF) at file offset 0x170e and finds 0x37 instead"* [VERIFIED: `.planning/WINDOWS.md`, finding 8, quoted verbatim]), and `compose_form` (`crates/deform6/src/vb/mod.rs:625-634`) still returns a `FormReport` with the real name (`"Main"`) and an **empty `controls` list**, plus a `StructureUnreadable` defect - producing the *same shape* of `FormReport` (name present, `controls: []`) as the genuinely-empty case, distinguishable only by checking whether `FormReport.defects` is non-empty.
**Why it happens:** `FormReport`'s own doc comment states this design directly: *"A form whose own control tree could not be built... still gets an entry here: `name` may be empty and `controls` is empty, and `defects` names the reason. One damaged form costs one entry, never the whole report"* [VERIFIED: `crates/deform6/src/vb/mod.rs:135-139`].
**How to avoid:** The `.frm` writer must check `FormReport.defects` for a `DefectKind::StructureUnreadable` naming `"ControlTree"` before deciding a form is legitimately childless. A form with such a defect still needs a `.frm` written (roadmap success criterion 1 requires one `.frm` per form and exit 0 across all 44 corpus programs, which includes Map Editor.exe), but the report must mark that form's confidence honestly (e.g. every property/control fact for that form is `"unrecoverable"`, not silently absent) - writing a bare `Begin VB.Form Main \nEnd` with no comment (comments are illegal inside `Begin`, per FILE-FORMATS.md §2.6) is *technically loadable* by the IDE but is a wrong answer that looks like a right one, the exact failure class `AGENTS.md`'s "What a test can hold on to" section names as the worst kind of defect.
**Warning signs:** A structural check (plan 04-09) that only asserts "the written `.frm` parses" will pass on this case; only a check that cross-references `.frm` control count against `FormReport.controls.len()` and separately audits the JSON report's own confidence values for this form will catch a silent empty-form regression.

### Pitfall 3: Confusing "colour" and "enumeration" `Long`/`Byte`/`Integer` properties with plain numeric ones

**What goes wrong:** `PayloadType` (`crates/deform6/src/vb/opcodes.rs:58-82`) has exactly nine variants - `Byte`, `Boolean`, `Integer`, `Long`, `Single`, `Text`, `Picture`, `Font`, `Position` [VERIFIED: `crates/deform6/src/vb/opcodes.rs:58-82`] - and **no `Colour` or `Enum` variant**. `BackColor` is registered as opcode 3, type `Long`, in the built-in table (`crates/deform6/src/vb/opcodes.rs:364`: `(3, "BackColor", PayloadType::Long)`) [VERIFIED: `crates/deform6/src/vb/opcodes.rs:364`]. But FILE-FORMATS.md §3.6 requires `BackColor` to be written as `&H00FFFFFF&` (8 uppercase hex digits, `&H`/`&` bracketing) on an intrinsic control and as a **plain signed decimal** on an OCX control [CITED: `.planning/research/FILE-FORMATS.md` §3.6], while a plain `Long` property (were one to exist) would be written as `.planning/research/FILE-FORMATS.md` §3.2's bare decimal. A writer that formats every `PropertyValue::Long` the same way will corrupt every colour property.
**Why it happens:** The read-side type system was built to decode *bytes*, not to remember *display intent*; that information exists only in FILE-FORMATS.md's own prose (§3.5, §3.6), keyed by property **name**, not by any field on `PropertyValue`.
**How to avoid:** Plan 04-02 needs a name-keyed formatting-hint table separate from `OpcodeTable` - e.g. `{"BackColor", "ForeColor", "FillColor"} -> Colour`, `{"BorderStyle", "ScaleMode", "StartUpPosition", "MousePointer", "Alignment", ...} -> Enum(member name lookup)`, everything else `-> PlainNumber`. This table's membership is itself corpus-bounded: FILE-FORMATS.md §3.5 lists the *only* enumeration member names the corpus has ever proven (`Flat`, `None`, `Solid`, `Transparent`, `Checked`, `Fixed Single`, `Right Justify`, `Center`, `CenterScreen`, `Cross`, `Dropdown List`, `Both`, `Pixel`, `Windows Default`, `Fixed ToolWindow`, `Custom` [CITED: `.planning/research/FILE-FORMATS.md` §3.5]); an enum value outside this list should be written as a bare number with no `'member name` comment (FILE-FORMATS.md §3.5: *"an emitter that does not know the name can omit the whole comment safely"* [CITED: `.planning/research/FILE-FORMATS.md` §3.5]) rather than guessing a name.
**Warning signs:** A `.frm` with `BackColor = 8421504` instead of `BackColor = &H80000005&` - technically a valid VB6 integer literal, so the file still loads, but it is a different value type than the IDE would ever write, and the structural check's "properties in case-insensitive alphabetical order" assertion will not catch a formatting error, only an ordering one.

### Pitfall 4: Assuming `.frm`/`.cls`/`.vbp`/`.bas` line templates can share one "pad name, write =, write value" helper

Covered in "Architecture Patterns" Pattern 3 above; repeated here because it is a common refactoring temptation once three near-identical-looking formats exist side by side, and FILE-FORMATS.md explicitly names it as a real defect class (§5.2: *"An emitter that reuses one writer for both produces the wrong bytes"*).

### Pitfall 5: Missing that the executable's own bytes, not just `Report`, are needed at write time

Covered in Pattern 1 above (blob bytes are not carried in `PropertyValue::Blob`). Plan 04-01 or 04-08 must decide where the original `data: &[u8]` the executable was read from stays alive through the whole `extract` run, since `.frx` writing needs to re-read `offset..offset+4+declared_len` from it.

### Pitfall 6: The 40-character truncation class (named directly in the roadmap)

**What goes wrong:** The IDE silently truncates a control or class name over 40 characters to exactly 40 and still loads the form; code that refers to the untruncated name then fails to compile, with the only diagnostic in a `.log` file the roadmap itself notes *"the user may never open"* [VERIFIED: `.planning/ROADMAP.md`, Phase 4 named risks, quoted verbatim].
**How to avoid:** `model.rs` (plan 04-01) must clamp every control and class name to 40 characters itself, at model-build time (not at the last moment inside the `.frm` writer, since the `.vbp`'s `Class=`/`Form=` file-name-deriving logic and the report's `path` keys must also use the *clamped* name consistently), and record every clamp performed as a report item so RPT-05 ("every defect the run met") is honest about it.

## Code Examples

### The exit-code table this phase's `extract` subcommand must extend, not replace

```rust
// Source: crates/deform6-cli/src/main.rs:6-21 (VERIFIED, already shipped)
// | Code | Meaning |
// |---|---|
// | 0 | The file was read |
// | 1 | Not a PE file |
// | 2 | A PE file, but it holds no Visual Basic runtime |
// | 3 | Visual Basic, but not version 6 |
// | 4 | Visual Basic 6, but damaged |
// | 5 | An internal error, including a usage error |
```
The doc comment on this table states *"the numbering must never move"* [VERIFIED: `crates/deform6-cli/src/main.rs:1-5`]. `extract`'s own new failure modes (`-o` path already exists and `--force` was not given; `--report` path unwritable) must map to exit code 5 (internal/usage error), matching the existing convention that a usage-shaped failure, not a file-content failure, is always 5.

### Reading the `Command` enum this phase's new `Extract` variant is added beside

```rust
// Source: crates/deform6-cli/src/main.rs:50-70 (VERIFIED, already shipped)
#[derive(clap::Subcommand)]
enum Command {
    /// Reads one executable and prints what DeForm6 found in it.
    Inspect {
        input: PathBuf,
        #[arg(long)]
        opcode_table: Option<PathBuf>,
    },
    // Extract { input: PathBuf, #[arg(short, long)] output: PathBuf,
    //           #[arg(long)] report: Option<PathBuf>, #[arg(long)] force: bool, ... }
}
```

### The `.frm` property line template (WRT-03), corpus-measured to the byte

```
// Source: .planning/research/FILE-FORMATS.md §3.1 (CITED, corpus-measured on 5899 lines)
<indent><name padded to 16><=><3 spaces><value><CRLF>

   BackColor       =   &H80000005&
   BorderStyle     =   1  'Fixed Single
   Caption         =   "Image Curves - tannerhelland.com"
```
`indent` is `3 * depth` spaces; `Begin`/`BeginProperty` lines end with exactly one trailing space, `End`/`EndProperty` with none (§2.4, corpus-proved on 487 `Begin` and 235 `BeginProperty` lines).

### WRT-07: an empty procedure body with a correct signature, from `Prototype`

```rust
// deform6::vb::functyp types already carry everything needed
// (crates/deform6/src/vb/functyp.rs:203-352, VERIFIED)
// Prototype { arguments: Vec<Argument>, is_function: bool,
//             return_type: Option<TypeEntry>, property_kind: PropertyKind, .. }
// Argument { name: String, entry: TypeEntry, default: Option<DefaultValue> }
// TypeEntry { vb_type: VbType, optional: bool, array: bool, by_ref: bool }
//
// A writer builds, e.g.:
//   Public Function mySub(Arg1 As Long, V As Variant) As Long
//   End Function
// by joining `arguments` with modifier prefixes (Optional/ByRef/() for Array)
// in declaration order, and appending `As <return type>` when `is_function`.
// The empty body is simply nothing between the signature line and `End Sub`/
// `End Function`/`End Property` - WRT-07 asks for exactly this, no more.
```

### Deterministic JSON output

```rust
// serde_json::to_string_pretty (or to_writer_pretty) on a struct (never a
// HashMap) preserves field declaration order deterministically, because
// serde's derive walks struct fields in source order, not by any runtime
// hash. Existing precedent for "never HashMap, always an ordered
// collection" in this workspace: crates/xtask/src/opcode_table.rs:77 uses
// BTreeMap specifically so `toml::to_string`'s output is reproducible
// (VERIFIED: `crates/xtask/src/opcode_table.rs:26,77`, quoted:
// "use std::collections::BTreeMap;" / "let mut doc: BTreeMap<String,
// BTreeMap<String, SerRow>> = BTreeMap::new();"). Apply the same rule to
// the Phase 4 report: if any map-shaped data is needed at all (unlikely,
// since RPT-02 wants "a flat array of items"), use BTreeMap, never HashMap.
// Floats: serde_json formats f32/f64 via a deterministic shortest-round-trip
// algorithm (ryu); FontBlock::size_points/size_remainder are already u32
// (crates/deform6/src/vb/propstream.rs:486,489), so no float formatting
// concern arises for font size at all - only PropertyValue::Single{value: f32}
// (from a `Single`-typed opcode) would serialize as a JSON number and this
// is already deterministic per-run, per-input by construction.
```

## State of the Art

Not applicable in the usual "library X was superseded by library Y" sense - VB6 itself has had no updates since 2008, and the on-disk formats this phase writes have not changed. The one relevant "state of the art" fact is that `.planning/research/FILE-FORMATS.md` and `.planning/research/STRUCTURES.md` are themselves the most current, most corpus-grounded reference available for this exact problem (built 2026-09-07 through 2026-09-11 by this same project, cross-checking two independent public parsers `vb6parse` and Semi VB Decompiler against 121 real files) - there is no external "VB6 project format" reference more current or more empirically grounded than what already sits in this repository.

**Deprecated/outdated:** The roadmap's own named-risk text records one internally-superseded number: *"An earlier text here said `blobLen + 12`... The two agree once the base is right"* [VERIFIED: `.planning/ROADMAP.md`, Phase 4 named risks] - already corrected in the shipped `frx.rs` code; no plan should reintroduce the `+ 12` reading.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `PropertyValue::Blob`'s bytes should be re-read from the original executable bytes at write time (via `offset..offset+4+declared_len`), rather than carried forward into `Report` by a Phase 4 change to `propstream.rs` | Architecture Patterns, Pattern 1 | If wrong, plan 04-01 must instead widen `PropertyValue::Blob`/`compose_control` to carry the bytes, which touches Phase 3 code (`propstream.rs`) that this phase was scoped to leave alone; either approach works, but the planner must pick one explicitly rather than discover the missing bytes mid-implementation. |
| A2 | The name-keyed colour/enum/plain-number formatting-hint table (Pitfall 3) should be a new, small, hand-written table scoped to the known corpus-proven names, not an extension of `OpcodeTable`/`PayloadType` | Common Pitfalls, Pitfall 3 | If wrong (e.g. the planner instead widens `PayloadType` with `Colour`/`Enum` variants), every existing `opcodes.rs` builtin-table entry and `OpcodeTable::parse` caller needs updating; scoping this as a separate, additive table keeps Phase 3 code untouched, which this phase's own dependency (Phase 3 complete, verified) suggests is the lower-risk path, but it is a design choice, not a corpus-proven fact. |
| A3 | `extract`'s CLI flags are exactly `-o`/`--output`, `--report`, `--force` as the roadmap names them, with no other flags needed for v1 | Phase Requirements table, WRT-01 | Low risk: these three are already locked in `ROADMAP.md`'s plan 04-08 description; only the exact long-form spelling of `-o` (`--output` vs `--out` vs `--out-dir`) is a naming choice this research does not settle, since no prior `deform6-cli` convention exists for a directory-taking flag (only `Inspect`'s `--opcode-table`, which takes a file). |
| A4 | The JSON report file name/location convention (`<dir>/<name>.report.json` beside the project, unless `--report <path>` overrides it) | Phase Requirements table, RPT-01 | Low-medium risk: ROADMAP.md's phase goal says "one JSON report beside it" but does not name the exact filename; the planner should lock this as a CONTEXT.md-equivalent decision during `/gsd-plan-phase`, since the structural check (04-09) and any future Phase 6 documentation both need to agree on it. |

## Open Questions

1. **What does the JSON report's `path` key look like for a report item about the whole run (e.g. a `--opcode-table` note, or a top-level defect not tied to any object), versus a specific item like `/forms/frmMain/controls/cmdOk`?**
   - What we know: the roadmap's own example, `/forms/frmMain/controls/cmdOk`, and RPT-02's requirement that "each item [is] keyed by a path".
   - What's unclear: whether a run-level fact (e.g. "this run used the built-in opcode-table subset, not a user-supplied one") gets a synthetic path like `/` or `/meta`, or is not modeled as an "item" at all and instead lives in a separate top-level field outside the flat array.
   - Recommendation: plan 04-06 should define this explicitly; it does not block any other plan, since 04-06 owns `report.rs` alone in its own wave.

2. **How does the extract subcommand behave on `corpus/vb6-code/Map-editor-2D/Map Editor.exe`, whose `Main` form's control tree refuses (WINDOWS.md finding 8, open)?**
   - What we know: `compose_form` already handles this gracefully at the `inspect` level (empty `controls`, a `StructureUnreadable` defect, form-level continuation, no crash).
   - What's unclear: whether roadmap success criterion 1 ("writes ... one `.frm` per form ... exits 0 ... for all 44 corpus programs") is satisfiable literally for this one form given zero of its real controls are recoverable, or whether the criterion is satisfied by writing a minimal, honestly-flagged `.frm` for `Main` (per Pitfall 2 above) while the report clearly marks every fact about it as unrecoverable.
   - Recommendation: confirm with the human during discuss-phase or plan-phase that "one `.frm` per form, honestly near-empty and flagged, for the one form the control-tree walk cannot fully resolve" satisfies the intent of success criterion 1, since this is a pre-existing, documented, open limitation (not something Phase 4 can close - WINDOWS.md finding 8 needs its own byte-level research, out of this phase's scope per the roadmap's own risk list, which does not mention it).

3. **Should the `.frx` list-record format (FILE-FORMATS.md gap 5, `ComboBox`/`ListBox.List`) ever be reached by this phase, given no corpus control exercises it?**
   - What we know: FILE-FORMATS.md §4.6 documents the layout as `[b]`-tagged (unverified, from `vb6parse` alone) and explicitly warns of a known off-by-one bug vb6parse itself documents for a related record kind.
   - What's unclear: whether any of the 44 corpus programs' `ComboBox`/`ListBox` controls set a `List` property that reaches this code path at all during Phase 4's own writing (as opposed to Phase 3's reading, which already reports `Undecoded` for any opcode it has no safe-provenance table entry for).
   - Recommendation: given the `Undecoded` fallback already exists and this record type is `[b]`-only evidence, do not implement a `List` writer in this phase; let it fall through as an undecoded/omitted property, and record it explicitly in Assumptions/Open Questions of any plan that touches `ComboBox`/`ListBox`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| VB6 IDE (Windows) | Full recompilation verification of the roadmap's own success criteria | ✗ (this is a Linux/macOS CI sandbox; no Windows host) | - | The structural check (plan 04-09) already documented in the roadmap: *"Full recompilation cannot run in this CI. It needs VB6 on Windows. The structural check is what runs"* [VERIFIED: `.planning/ROADMAP.md`, Phase 4 named risks, quoted verbatim]. |
| `cargo` / Rust toolchain | Everything in this phase | ✓ | edition 2024, `rust-version = "1.97.1"` (`Cargo.toml:6-7`) | - |
| `serde_json` crate | RPT-01 through RPT-06 | ✗ (not yet a dependency; verified `OK` to add, see Package Legitimacy Audit) | 1.0.151 on crates.io | - |
| `crates/deform6/tests/support/frm.rs` (independent `.frm` reader) | The structural check (04-09) | ✓ (already exists, already used by `differential.rs`) | - | - |

**Missing dependencies with no fallback:**
- VB6 IDE on Windows: no fallback for *full* recompilation verification; the roadmap already accepts the structural-check fallback as the intended design, not a gap to close.

**Missing dependencies with fallback:**
- `serde_json`: not yet added; fallback is "add it" (trivial, already verified safe), not a workaround.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test --workspace` (Rust's built-in test harness; no external framework) |
| Config file | none - `AGENTS.md`'s own gate: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, run "before every commit... Not a selection" [VERIFIED: `AGENTS.md`, "The gate", quoted verbatim] |
| Quick run command | `cargo test --workspace -- --test extract` (or narrower, once plan 04-09's test file exists) |
| Full suite command | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WRT-01 | `extract` writes a project dir, exits 0, writes nothing outside `out/` | integration | `cargo test --workspace -p deform6-cli` (new CLI test) | ❌ Wave 0 (04-08) |
| WRT-02 | `.vbp` lists every object/control dependency | unit + corpus sweep | `cargo test --workspace -p deform6 write::vbp` | ❌ Wave 0 (04-03) |
| WRT-03 | `.frm` byte-level layout | unit, literal byte assertions | `cargo test --workspace -p deform6 write::frm` | ❌ Wave 0 (04-04) |
| WRT-04 | Alphabetical property order, menus last | unit | `cargo test --workspace -p deform6 write::frm::ordering` | ❌ Wave 0 (04-04) |
| WRT-05 | `.bas`/`.cls` attribute preamble | unit, byte-exact | `cargo test --workspace -p deform6 write::code` | ❌ Wave 0 (04-05) |
| WRT-06 | Windows-1252 + CRLF, no BOM | unit + whole-tree sweep | `cargo test --workspace extract_encoding` | ❌ Wave 0 (04-01, 04-09) |
| WRT-07 | Empty procedure, correct signature | unit | `cargo test --workspace -p deform6 write::code::signatures` | ❌ Wave 0 (04-05) |
| RPT-01..06 | JSON report shape, determinism, evidence | unit + two-run diff | `cargo test --workspace -p deform6 report` | ❌ Wave 0 (04-06) |
| Roadmap SC 3 | `.frx` offsets resolve inside the written `.frx` | integration, reads written `.frm` back through `support/frm.rs` | `cargo test --workspace -p deform6 --test extract_structural` | ❌ Wave 0 (04-09) |
| Roadmap SC 4 | Structural check (file existence, identifier legality, nesting depth, alphabetization, menu order) | integration | `cargo test --workspace -p deform6 --test extract_structural` | ❌ Wave 0 (04-09) |

### Sampling Rate

- **Per task commit:** the narrowest `cargo test --workspace -p deform6 <module>::` slice for the module just written.
- **Per wave merge:** `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` (the whole gate; `AGENTS.md` allows no smaller selection at this point).
- **Phase gate:** full suite green, plus the corpus-wide `extract` run over all 44 programs (mirroring `corpus_sweep.rs`'s existing pattern for `inspect`) before `/gsd-verify-work`.

### Wave 0 Gaps

- [ ] `crates/deform6/src/write/mod.rs`, `model.rs`, `values.rs`, `vbp.rs`, `frm.rs`, `code.rs` - none exist yet; all of Phase 4's own module tree is Wave 0.
- [ ] `crates/deform6/src/report.rs` - does not exist yet.
- [ ] `crates/deform6/tests/extract_structural.rs` - does not exist yet; will reuse `crates/deform6/tests/support/frm.rs` (VERIFIED, already exists) rather than writing a new independent reader.
- [ ] `serde_json` dependency in `Cargo.toml` / `crates/deform6/Cargo.toml` - not yet added.
- [ ] Framework install: none needed beyond `cargo add serde_json` (workspace dependency declaration) - no test framework install required, `cargo test` is already the workspace's own harness.

*(Every test file for this phase is new; this whole phase is Wave 0 by construction, since Phases 1-3 built the reading side only.)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | offline CLI tool, no auth surface |
| V3 Session Management | no | no sessions |
| V4 Access Control | no | single-user local CLI |
| V5 Input Validation | yes | The executable bytes are already validated on the read path (Phase 1-3, `#![forbid(unsafe_code)]`, `Region`'s bounds-checked accessors); Phase 4's *new* input-validation surface is the **output path** (`-o <dir>`) and the recovered strings being written back out as file names. A control/class/module name recovered from a hostile executable becomes a **file name** on disk (`Form=Name.frm`) - this is a path-injection-shaped risk (a recovered name of `../../etc/passwd` or containing a path separator) that Phase 1-3 never had to consider, because `inspect` never wrote a file. |
| V6 Cryptography | no | no cryptographic operation in this phase |
| V12 File and Resources | yes | The identifier-legality check (WRT-03/roadmap success criterion 4: "every control and class name is a legal VB6 identifier of 40 characters or fewer") already gives a natural sanitization boundary for file names *if* it is applied before a recovered name is used to derive a file path, not only before it is used to derive a `.frm` control-block name. |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A recovered object/control name containing `/`, `\`, `..`, or a NUL byte is used verbatim as (part of) a file path under `-o <dir>` | Tampering / Elevation of Privilege (writes outside the intended output directory, violating roadmap success criterion 1's "writes nothing outside `out/`") | The VB6-identifier legality check (already required by WRT-03/roadmap success criterion 4: *"every control and class name is a legal VB6 identifier"*) must run and reject/sanitize **before** any recovered string is used to build a file path, not only before it is written as a `.frm` line. A legal VB6 identifier is alphanumeric-plus-underscore starting with a letter, which is already incompatible with any path separator or `..` sequence, so enforcing this check consistently at the model layer (plan 04-01) closes this class entirely, provided the writers never bypass the model's sanitized names to build a path from a raw `Report` field directly. |
| `--force` overwrites files outside the target `-o <dir>` due to a symlink or relative-path escape in a recovered file name | Tampering | Resolve `-o <dir>` to an absolute, canonicalized path before writing, and verify every file this phase writes is a direct child of that canonicalized directory (no `..` component, no absolute recovered name) before any write occurs; reject the whole run rather than writing partially outside the target. |
| A crafted `--opcode-table` file (already an existing input surface from Phase 3, unchanged by this phase) | Tampering | Out of scope for this phase's *new* work - `OpcodeTable::parse` (Phase 3) already owns this validation; Phase 4 only *consumes* the resulting `OpcodeTable`, unchanged. |

## Sources

### Primary (HIGH confidence)

- `crates/deform6/src/vb/frx.rs` (whole file read this session) - `BlobCursor`, `FRX_ITEM_HEADER_LEN`, `extract_blob`, `Blob`, `ImageFormat`
- `crates/deform6/src/vb/controltree.rs` (read this session, lines 1-1219) - `ControlKind`, `ControlHeader`, `read_scope_run`, `walk`, `ControlNode`, `ControlTree`
- `crates/deform6/src/vb/propstream.rs` (read this session, lines 1-560) - `PropertyValue`, `PropertyStream`, `PositionBlock`, `FontBlock`
- `crates/deform6/src/vb/mod.rs` (read this session, lines 1-400, plus `compose_form` at 585-665) - `Report`, `FormReport`, `ControlReport`, `ObjectReport`, `inspect`
- `crates/deform6/src/error.rs` (whole file read this session) - `Site`, `DefectKind`, `Severity`, `Defect`, `Refusal`, `damaged`
- `crates/deform6/src/vb/vbstr.rs` (whole file read this session) - `VbStr`, `StrEncoding`, the `char::from(byte)` convention
- `crates/deform6/src/vb/ocx.rs`, `controlinfo.rs`, `functyp.rs`, `opcodes.rs` (signatures grepped and key sections read this session)
- `crates/deform6-cli/src/main.rs` (lines 1-140 read this session) - `Command`, `Exit`, the exit-code table
- `crates/deform6/tests/support/frm.rs` (lines 60-300 read this session) - `Form`, `Block`, `parse_blocks`, the independent `.frm` reader
- `.planning/research/STRUCTURES.md` (fully read this session, 1953 lines) - the on-disk binary structure reference, sections 1-15, corpus-verified against 44 executables
- `.planning/research/FILE-FORMATS.md` (fully read this session, 1385 lines) - the on-disk text-format reference, corpus-verified against 121 files across 32 VB6 projects
- `.planning/WINDOWS.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md` (fully read this session)
- `Cargo.toml` (workspace), `crates/deform6/Cargo.toml`, `crates/deform6-cli/Cargo.toml`, `crates/xtask/Cargo.toml` (all read this session) - confirmed no `serde_json`, no `encoding_rs`, no JSON dependency anywhere in the workspace today
- gsd-tools `package-legitimacy check` seam (run this session) - `serde_json` verdict `OK`
- `cargo search serde_json` (run this session against the live crates.io registry) - version `1.0.151`

### Secondary (MEDIUM confidence)

- FILE-FORMATS.md's own `[b]`-tagged rows (documented, not corpus-observed): the `.frx` list-record layout (§4.6), the `.ctl`/`.ctx` grammar (§5.3, §4.8), the MDIForm root class and `MDIChild` property (§9 gap 9), the 31 documented IDE load-error messages (§7.2) - all sourced by FILE-FORMATS.md from Microsoft Learn / `MicrosoftDocs/VBA-Docs` and the independent `vb6parse` Rust crate, cited there with URLs.

### Tertiary (LOW confidence)

- None directly used in this document; every claim above traces to either a file this session opened directly, or a `[VERIFIED: gsd-tools seam]`/`[VERIFIED: crates.io]` tool call this session ran. No `WebSearch`/`WebFetch` calls were made this session (`config.json` reports all external search providers - `brave_search`, `firecrawl`, `exa_search`, `tavily_search`, `ref_search`, `perplexity`, `jina` - disabled, and the domain is already exhaustively documented in-repo at corpus-measured precision, so a LOW-confidence web source would add nothing this document's own primary sources do not already settle at higher confidence).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - `serde_json` verified OK against both the gsd-tools legitimacy seam and the live crates.io registry; no other new dependency needed, and that conclusion follows from directly reading every `Cargo.toml` in the workspace.
- Architecture: HIGH - every type and function this document names as available to the writer was read directly from source this session, with line numbers.
- Pitfalls: HIGH for Pitfalls 1, 2, 5, 6 (each traces to a specific, quoted, already-shipped fact); MEDIUM for Pitfall 3 (the *problem* is corpus-verified, but the *exact shape* of the recommended fix - a new name-keyed table - is this document's own architectural recommendation, logged as Assumption A2).

**Research date:** 2026-09-12
**Valid until:** No expiry driven by external library churn (VB6 is a closed, unchanging target); re-verify only if Phase 3 code (`vb/frx.rs`, `vb/propstream.rs`, `vb/mod.rs`) changes before Phase 4 executes, since this document's HIGH-confidence claims are pinned to exact line numbers in those files as of 2026-09-12.
