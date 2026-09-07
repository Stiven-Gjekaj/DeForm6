# Phase 1: It reads the file - Research

**Researched:** 2026-09-07
**Domain:** PE parsing, VB6 on-disk structures, Rust workspace and lint design
**Confidence:** HIGH

Every claim below that is tagged `[VERIFIED: local]` was produced by code that
ran on this machine today. The scratch workspace is at `/tmp/df6ws`. It holds
the manifests, the lint wall, `region.rs`, `pe.rs`, `vb.rs`, the CLI, and the
tests described here. `cargo fmt --all --check`, `cargo clippy --all-targets --
-D warnings` and `cargo test --workspace` all pass in it against the real
`corpus/`. Numbers over the corpus come from that run, not from a document.

Toolchain used: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1`, host
`aarch64-apple-darwin`. `[VERIFIED: local, rustc --version]`

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **`inspect` prints text. Machine-readable output waits for Phase 4.**
  `deform6 inspect` writes human-readable text only in this phase. There is no
  `--json` yet. One schema, introduced once, in Phase 4.

  The shape:

  ```
  File      Mandelbrot.exe  (57344 bytes)
  Format    PE32, 4 sections
  Runtime   MSVBVM60.DLL  (Visual Basic 6)
  Header    VB5! at 0x0000141c  build 0x231c
  Project   Mandelbrot_Fractal_Demo
  Mode      native
  Objects   1
  ```

- **The exit code names the reason.**

  | Code | Meaning |
  |---|---|
  | 0 | The file was read |
  | 1 | Not a PE file |
  | 2 | A PE file, but it holds no Visual Basic runtime |
  | 3 | Visual Basic 5, not Visual Basic 6 |
  | 4 | Visual Basic 6, but damaged. `--salvage` may get something |
  | 5 | An internal error in DeForm6 |

  Code 4 is defined here and is not reachable until Phase 5.

- **PE parsing**: `object` 0.40.0, `default-features = false`, features
  `read_core, pe, std`.
- **Binary reading**: hand-written. A bounded `Region` with no infallible
  accessor, plus `Off`, `Rva` and `Va` newtypes that do not implement `Add`.
- **Errors**: a two-level enum with `thiserror` for `Display`. `Defect` carries
  a site, a kind and a severity. `Journal` has one `record` method.
- **CLI**: `clap` 4.6.6 with `default-features = false`.
- **Layout**: `crates/deform6` and `crates/deform6-cli`. `corpus/` stays at the
  repository root.

### Claude's Discretion

Nothing was marked as discretion in CONTEXT.md. This document answers the eight
"how exactly" questions the planner asked and does not re-open a locked
decision.

### Deferred Ideas (OUT OF SCOPE)

- Do not implement an entry stub variant that no corpus file shows
  (`STRUCTURES.md` §1.3, opcodes `0x5A` and `0x11`).
- Do not claim the P-code branch is tested.
- Do not add a lint allowance to get the deny wall to pass in library code.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DET-01 | Read a PE file and resolve its entry point to a file offset through the section table | §5 address map; `PeImage::rva_to_off`; 44 of 44 corpus files resolve |
| DET-02 | Follow the entry stub to the VB header and confirm `VB5!` | §4 `region_at`; §3 `Region::u8`/`va_le`/`take`; 44 of 44 reach `VB5!` |
| DET-03 | Tell VB6 from VB5 by the imported runtime DLL name | §6 runtime discrimination; `PeImage::imported_dlls` |
| DET-04 | Refuse a VB5 file by name, and refuse a non-VB file, each with one sentence | §6 `Refusal` enum; §7 in-test fixtures |
| DET-05 | Report native or P-code from `ProjectInfo.lpNativeCode` | §4 `region_at_va` plus `u32_le(0x20)`; 44 of 44 non-zero |
| DET-06 | `inspect` prints the header, project name and mode, and writes nothing | §2 CLI; §7 exit-code tests; the run opens the file read only |
</phase_requirements>

## Summary

Nothing in this phase is unknown after this session. The workspace manifests,
the lint wall, the `Region` API, the `object` 0.40.0 call set, the address map
rules, the runtime strings, the test shape and the exit-code mechanism were all
built and run, not reasoned about. The five bad shapes that the wall must reject
were compiled and produced eight compile errors. A skeleton library plus CLI
reads all 44 corpus executables, prints the header, the project name and
`native`, and exits 0.

The one open gap this phase had to close is closed. `VBHeader` 0x58 is the EXE
name without its extension, and 0x5C is the project title. The SVBD and SEK
reading is right and the AI, IDC and PVB reading is wrong. The evidence is 44
of 44 corpus binaries diffed against the `Title=`, `ExeName32=`, `HelpFile=` and
`Name=` lines of the matching `.vbp`. Section 8 gives the procedure and the
numbers.

Three findings change what the plan must contain. First, `clap` uses exit code
`2` for a usage error, which collides with the project's "no Visual Basic
runtime" code, so `Cli::try_parse` and a hand-written mapping are required, not
optional. Second, `PeFile32::parse` returns `Ok` on a corpus file truncated to
one eighth of its length, so a successful parse is never evidence that the file
is intact. Third, `VB40016.DLL` is unreachable through `PeFile32`, because a
16-bit VB4 image is an NE file and `object` rejects it before any import is
read.

**Primary recommendation:** Build wave 01-01 from the literal manifests in
section 1 and the literal lint tables in section 2. They compile and the gate is
green with them. Do not invent variants.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Bounded byte access | Library core (`read/region.rs`) | - | No dependency. Every other module reads through it. |
| PE envelope and section table | Library, `object` seam (`read/pe.rs`) | - | The only file that names `object`, so the crate can be replaced behind seven methods. |
| Address translation (VA, RVA, offset) | Library, `object` seam | - | It needs the section table, which lives with the PE parse. |
| VB structure decoding | Library (`vb/*.rs`) | - | Pure reads over `Region`. No I/O and no `object`. |
| Error classification and severity | Library (`error.rs`) | - | One `match` on `DefectKind`. |
| Strict against salvage policy | Library (`journal.rs`) | - | One choke point. No parse site branches on the mode. |
| Argument parsing, exit codes, printing | CLI crate (`main.rs`) | - | The library returns values. The shell turns them into a number and a sentence. |
| File I/O | CLI crate | - | The library takes `&[u8]`. That is what makes the fuzz target the real API. |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `object` | 0.40.0 | PE32 envelope, section table, import directory | Locked in STACK.md. Version confirmed to resolve today. `[VERIFIED: local, cargo tree]` |
| `thiserror` | 2.0.20 | `Display` for `Error`, `Defect`, `DefectKind` | Locked in STACK.md. Resolves to 2.0.20. `[VERIFIED: local]` |
| `serde` | 1.0.229 | `Serialize` on `Defect` and `Site` for the Phase 4 report | Locked in STACK.md. Resolves to 1.0.229. `[VERIFIED: local]` |
| `clap` | 4.6.6 | Subcommands, `--help`, `--version`, `OsString` paths | Locked in STACK.md. Resolves to 4.6.6. `[VERIFIED: local]` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `libfuzzer-sys` | 0.4.13 | The fuzz target body | Phase 5. The crate directory and the exclusion go in now (§1.5). |

`serde_json` and `hmac-sha256` belong to Phase 4 and are not added here.

### Alternatives Considered

STACK.md settled every alternative with a measurement. This session re-opened
none of them. The one open question STACK.md left is whether to hand-roll the
PE envelope. The answer stays "not yet", and section 4 records exactly what
`object` does and does not give, so the swap stays cheap.

**Installation:** No `cargo add` is needed. The manifests in section 1 are
literal and complete.

**Version verification**, run today:

```
$ cargo tree -e normal --prefix none --no-dedupe   # in /tmp/df6ws
object v0.40.0     memchr v2.8.3
thiserror v2.0.20  thiserror-impl v2.0.20
serde v1.0.229     serde_core v1.0.229   serde_derive v1.0.229
clap v4.6.6        clap_builder v4.6.6   clap_derive v4.6.4
clap_lex v1.1.0    anstyle v1.0.14       heck v0.5.0
proc-macro2 v1.0.107  quote v1.0.47  syn v3.0.5  unicode-ident v1.0.24
```

**17 crates total for phase 1.** `[VERIFIED: local, cargo tree | sort -u | wc -l]`
STACK.md predicted 21 for the whole project. The difference is `serde_json`,
`itoa`, `memchr` double counting and `hmac-sha256`, which Phase 4 adds.

## Package Legitimacy Audit

Run with `gsd-tools query package-legitimacy check --ecosystem crates`.
`[VERIFIED: crates.io via package-legitimacy seam]`

| Package | Registry | First published | Weekly downloads | Source repo | Verdict | Disposition |
|---------|----------|-----------------|------------------|-------------|---------|-------------|
| `object` | crates | 2016-08-19 | 8,258,152 | github.com/gimli-rs/object | OK | Approved |
| `thiserror` | crates | 2019-10-09 | 27,154,310 | github.com/dtolnay/thiserror | OK | Approved |
| `serde` | crates | 2014-12-05 | 23,037,273 | github.com/serde-rs/serde | OK | Approved |
| `clap` | crates | 2015-03-01 | 17,555,286 | github.com/clap-rs/clap | OK | Approved |
| `libfuzzer-sys` | crates | (rust-fuzz) | - | github.com/rust-fuzz/libfuzzer | OK | Approved |

**Packages removed due to a SLOP verdict:** none.
**Packages flagged as suspicious:** none.
All five names came from `.planning/research/STACK.md`, which selected them by
measurement, and all five resolved and compiled on this machine today.

---

# 1. The exact `Cargo.toml` set

All three manifests below were written to `/tmp/df6ws`, and the three-command
gate passes with them. `[VERIFIED: local, /tmp/df6ws]`

## 1.1 Root workspace manifest

```toml
[workspace]
resolver = "3"
members = ["crates/deform6", "crates/deform6-cli"]
exclude = ["crates/deform6/fuzz"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.97.1"
license = "MIT"

[workspace.dependencies]
deform6 = { path = "crates/deform6" }
object = { version = "0.40.0", default-features = false, features = ["read_core", "pe", "std"] }
thiserror = "2.0.20"
serde = { version = "1.0.229", features = ["derive"] }
clap = { version = "4.6.6", default-features = false, features = ["std", "derive", "help", "error-context"] }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
indexing_slicing = "deny"
arithmetic_side_effects = "deny"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unreachable = "deny"
cast_possible_truncation = "deny"
cast_sign_loss = "deny"
cast_possible_wrap = "deny"
integer_division = "deny"

[profile.release]
panic = "abort"
overflow-checks = true
```

## 1.2 `crates/deform6/Cargo.toml`

```toml
[package]
name = "deform6"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
object.workspace = true
thiserror.workspace = true
serde.workspace = true

[lints]
workspace = true
```

## 1.3 `crates/deform6-cli/Cargo.toml`

```toml
[package]
name = "deform6-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[[bin]]
name = "deform6"
path = "src/main.rs"

[dependencies]
deform6.workspace = true
clap.workspace = true

[lints]
workspace = true
```

The `[[bin]] name = "deform6"` line is load-bearing. It gives the binary the
name the user types, and it sets the environment variable
`CARGO_BIN_EXE_deform6` that the CLI integration test reads. `[VERIFIED: local]`

## 1.4 The resolver must be declared

**Yes, `resolver = "3"` must be written out.** The root manifest is a virtual
one, so it has no `edition` key of its own, and cargo does not take the resolver
from `[workspace.package] edition`. With the line deleted, cargo 1.97.1 prints:

```
warning: virtual workspace defaulting to `resolver = "1"` despite one or more
workspace members being on edition 2024 which implies `resolver = "3"`
```

`[VERIFIED: local, cargo check --workspace with the line removed]`
`[CITED: https://doc.rust-lang.org/cargo/reference/resolver.html#resolver-versions]`

## 1.5 The fuzz crate exclusion, measured honestly

`exclude = ["crates/deform6/fuzz"]` is in the manifest above, as STACK.md asks.
The measurement does not support the strength of the claim STACK.md makes for
it.

With a real fuzz crate present at `crates/deform6/fuzz` holding a `#![no_main]`
target, `cargo metadata --no-deps` reports members `['deform6', 'deform6-cli']`
in every one of these four combinations:

| `exclude` present | nested `[workspace]` in the fuzz manifest | Result |
|---|---|---|
| yes | yes | fuzz crate outside; gate green |
| yes | no | fuzz crate outside; gate green |
| no | yes | fuzz crate outside; gate green |
| no | no | fuzz crate outside; gate green |

`[VERIFIED: local, cargo metadata and cargo clippy --all-targets in all four combinations]`

The reason is that `members` is an explicit list, not a glob, and
`crates/deform6/fuzz` is two levels below `crates/`, so even `crates/*` would
not reach it. **Keep the `exclude` line.** It costs one line, it documents the
intent, and it protects the gate if somebody later writes a deeper glob. Do not
plan a task that verifies "removing `exclude` breaks the gate", because on
cargo 1.97.1 it does not, and the task would fail.

Keep `--fuzzing-workspace true` on the `cargo fuzz init` call in Phase 5 for the
same reason: it costs nothing and its default is `false`.
`[CITED: .planning/research/STACK.md §5]`

## 1.6 `rust-toolchain.toml`

```toml
[toolchain]
channel = "1.97.1"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

The gate is green with this file present. `[VERIFIED: local]` The pin matters
because `clippy::arithmetic_side_effects` and `clippy::indexing_slicing` are
restriction lints, and a newer clippy can change what they catch. A moving
toolchain would move the wall under the code.

---

# 2. The literal lint wall

## 2.1 What the tables are

The two tables in §1.1 are the whole wall. Copy them exactly. They are the
tables STACK.md verified, expressed as Cargo lint tables rather than as inner
attributes, so that both crates and every target inherit them from one place.

## 2.2 The inner attributes for each crate root

`[workspace.lints.rust] unsafe_code = "forbid"` is passed to rustc as
`-F unsafe-code` on the command line. Cargo says so itself in the error note.
`[VERIFIED: local]`

```
error: usage of an `unsafe` block
  = note: requested on the command line with `-F unsafe-code`
```

So `#![forbid(unsafe_code)]` in a crate root is **not required**. It is still
worth writing, because a reader who opens `lib.rs` should see the promise
without opening the workspace manifest, and because the two together compile
without complaint.

**`crates/deform6/src/lib.rs`:**

```rust
#![forbid(unsafe_code)]
```

**`crates/deform6-cli/src/main.rs`:** nothing. The CLI inherits the same wall
through `[lints] workspace = true`, and no relaxation is needed. See §2.5.

That is the whole set. Do not write a `#![deny(clippy::...)]` block in either
crate root. It would duplicate the Cargo tables and the two would drift.

## 2.3 The five shapes that must fail, and the lint that does the work

Compiled together in one file under the wall. Eight errors, no false negatives,
and the safe forms beside them produced none. `[VERIFIED: local, cargo clippy --all-targets -- -D warnings]`

| Bad shape | Lint that fires | Exact message |
|---|---|---|
| `d[0]` | `clippy::indexing_slicing` | `indexing may panic` |
| `&d[4..8]` | `clippy::indexing_slicing` | `slicing may panic` |
| `a + b` | `clippy::arithmetic_side_effects` | `arithmetic operation that can potentially result in unexpected side-effects` |
| `.unwrap()` | `clippy::unwrap_used` | ``used `unwrap()` on an `Option` value`` |
| `a as u32` where `a: u64` | `clippy::cast_possible_truncation` | ``casting `u64` to `u32` may truncate the value`` |
| `a as u32` where `a: i32` | `clippy::cast_sign_loss` | ``casting `i32` to `u32` may lose the sign of the value`` |
| `a / b` | `clippy::integer_division` and `clippy::arithmetic_side_effects` | `integer division` |
| `unsafe { ... }` | `-F unsafe-code` | ``usage of an `unsafe` block`` |

The full clippy run over that file reported:

```
   2 error: arithmetic operation that can potentially result in unexpected side-effects
   1 error: casting `i32` to `u32` may lose the sign of the value
   1 error: casting `u64` to `u32` may truncate the value
   1 error: indexing may panic
   1 error: integer division
   1 error: slicing may panic
   1 error: used `unwrap()` on an `Option` value
```

## 2.4 One cast the wall does **not** catch

`x as usize` where `x: u32` compiles clean. `clippy::cast_possible_truncation`
does not fire, because `usize` is at least 32 bits on every target clippy
models by default. The reverse, `x as u32` where `x: usize`, does fire:

```
error: casting `usize` to `u32` may truncate the value on targets with 64-bit wide pointers
```

`[VERIFIED: local, both directions compiled]`

This matters, because STACK.md's sketch writes
`pub fn as_usize(self) -> usize { self.0 as usize }`, and that line does
compile. The `Region` in section 3 uses `usize::try_from(...).unwrap_or(...)`
anyway, so that no `as` cast appears in the reading path at all. Either form
passes the wall. Choose the `try_from` form, so a reader never has to work out
which direction is safe.

## 2.5 Where a relaxation is needed, and why it is safe

Measured, not guessed. `cargo test --workspace` was green while
`cargo clippy --all-targets` was red, which is exactly the trap `AGENTS.md`
names: a test helper that `build` accepts and clippy rejects.

**Needed: every file under `tests/`.** An integration test file is its own crate
root, so the relaxation is an inner attribute at the top of the file, not an
attribute on a module.

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

**Needed: a `#[cfg(test)]` module inside `src/`.** The same list, as an outer
attribute on the module:

```rust
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests { /* ... */ }
```

**Use `#[allow]`, not `#[expect]`.** `#[expect]` on a lint the block does not in
fact trigger is itself an error: `this lint expectation is unfulfilled`. A test
module that stops using `.unwrap()` would then break the build for the wrong
reason. `[VERIFIED: local, both attributes tried]`

The `reason = "..."` key works on stable 1.97.1 for both attributes.
`[VERIFIED: local]`

**Why the relaxation is safe.** A test is the one place where a panic is the
correct outcome. The library's promise is "no panic on any input", where the
input is a file the library did not build. A test builds its own input, so a
`.unwrap()` on it does not read attacker bytes. When the value is wrong, the
panic is the failure report, and it names the file and line. Suppressing it
would turn a broken test helper into a silent pass, which is the fault
`AGENTS.md` warns about under "a test can pass for the wrong reason".

**Not needed anywhere else.** All three of these pass the full wall with no
relaxation. `[VERIFIED: local]`

| Thing that might have needed one | Result |
|---|---|
| `deform6-cli` crate, including `clap::Parser` and `clap::Subcommand` derive output | clean |
| `serde::Serialize` derive on `Site`, `DefectKind` and `Defect` | clean |
| `thiserror::Error` derive on `DefectKind`, `Defect` and `Error` | clean |

So the CLI crate takes the identical `[lints] workspace = true` and nothing
more. There is no reason to give it a different table.

**One trap: `examples/`.** `cargo clippy --all-targets` compiles
`crates/*/examples/*.rs` under the wall too. A scratch example written without
the relaxation broke the gate in this session. If this phase adds an example,
it needs the same inner attribute block, or it must be written to the same
standard as library code. `[VERIFIED: local]`

---

# 3. `Region`, `Off`, `Rva`, `Va` - the actual API

The file below compiles clean under the full wall on 1.97.1.
`[VERIFIED: local, /tmp/df6ws/crates/deform6/src/region.rs]`

## 3.1 The public signature list

```rust
// Off
pub const fn Off::new(raw: u32) -> Off
pub const fn Off::get(self) -> u32
pub const fn Off::checked_add(self, n: u32) -> Option<Off>
pub const fn Off::checked_sub(self, n: u32) -> Option<Off>
// private: fn index(self) -> usize

// Rva
pub const fn Rva::new(raw: u32) -> Rva
pub const fn Rva::get(self) -> u32
pub const fn Rva::checked_add(self, n: u32) -> Option<Rva>

// Va
pub const fn Va::new(raw: u32) -> Va
pub const fn Va::get(self) -> u32
pub const fn Va::is_null(self) -> bool
pub const fn Va::to_rva(self, image_base: u32) -> Option<Rva>

// Region<'a>
pub const fn Region::new(bytes: &'a [u8], base: Off) -> Region<'a>
pub fn Region::len(&self) -> u32
pub const fn Region::is_empty(&self) -> bool
pub fn Region::file_offset(&self, at: Off) -> Option<Off>
pub fn Region::take(&self, at: Off, len: u32) -> Option<&'a [u8]>
pub fn Region::subregion(&self, at: Off, len: u32) -> Option<Region<'a>>
pub fn Region::u8(&self, at: Off) -> Option<u8>
pub fn Region::u16_le(&self, at: Off) -> Option<u16>
pub fn Region::u32_le(&self, at: Off) -> Option<u32>
pub fn Region::i16_le(&self, at: Off) -> Option<i16>
pub fn Region::i32_le(&self, at: Off) -> Option<i32>
pub fn Region::off_le(&self, at: Off) -> Option<Off>
pub fn Region::va_le(&self, at: Off) -> Option<Va>
pub fn Region::cstr(&self, at: Off, max: u32) -> Option<&'a [u8]>
```

Every reader carries `#[must_use]`.

The derive set on all three newtypes:
`#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]`.
No `Add`. No `Sub`. No `From<u32>`. No `Deref`. No `Index` on `Region`.

`Region` holds two private fields and exposes neither:

```rust
pub struct Region<'a> { bytes: &'a [u8], base: Off }
```

There is no `as_bytes`, no `as_slice` and no `Deref<Target = [u8]>`. `take` is
the only route out, it takes a length, and it returns `Option`. That is the
"no infallible accessor" property, and it is a property of the type, not a
convention. `[VERIFIED: local]`

## 3.2 The failure value each returns

Every accessor returns `Option`, never `Result`. The `Region` layer does not
know the name of the structure being read or the name of the field, so it
cannot build a `Site`, and a `Region` that returned a half-populated `Defect`
would be worse than one that returned `None`.

The caller turns the `None` into a `Defect` at the site that has the names:

```rust
let count = hdr.u16_le(Off::new(0x44)).ok_or_else(|| Defect {
    site: Site {
        offset: hdr.file_offset(Off::new(0x44)).map_or(0, Off::get),
        rva: None,
        structure: "VbHeader",
        field: "wFormCount",
    },
    kind: DefectKind::PastEndOfFile { offset: 0x44, file_len },
})?;
```

`Region::file_offset` exists precisely for this. It converts a window-relative
offset into the absolute file offset that `Site` requires, and it does that with
`checked_add`, so it too returns `Option`.

The two-level error type stays exactly as STACK.md specifies. It compiles under
the wall with all three derives. `[VERIFIED: local]`

## 3.3 The common operations

**Read a u8, u16 or u32 little-endian at an offset.**

```rust
let opcode: u8  = region.u8(Off::new(0))?;
let forms:  u16 = region.u16_le(Off::new(0x44))?;
let flags:  u32 = region.u32_le(Off::new(0x34))?;
```

Implementation, for the record:

```rust
pub fn u32_le(&self, at: Off) -> Option<u32> {
    let b: [u8; 4] = self.take(at, 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(b))
}
```

`try_into()` on a `&[u8]` of the wrong length gives `Err`, and `.ok()?` turns
that into `None`. No panic and no `unwrap`.

**Take a subregion.**

```rust
// A 0x68 byte window on the VB header, rebased so offset 0 is the VB5! magic.
let hdr: Region<'_> = image.subregion(Off::new(0x1760), 0x68)?;
let magic = hdr.take(Off::new(0), 4)?;   // b"VB5!"
```

`subregion` recomputes `base`, so `hdr.file_offset(at)` still gives an absolute
file offset for the error report:

```rust
pub fn subregion(&self, at: Off, len: u32) -> Option<Region<'a>> {
    let bytes = self.take(at, len)?;
    Some(Region { bytes, base: self.file_offset(at)? })
}
```

**Read a NUL-terminated ASCII string.**

```rust
let name: &[u8] = hdr.cstr(Off::new(0x78), 0x104)?;
let text: String = name.iter().map(|&c| char::from(c)).collect();
```

`max` is mandatory and it is not a convenience. A file with no NUL byte after
the offset would otherwise scan the whole window. `cstr` returns `None` when no
NUL appears inside `max` bytes, which is a refusal, not a truncation. Use
`0x104` for a path-like field and `0x28` for a title, matching SVBD's own
bounds. `[CITED: .planning/research/STRUCTURES.md §2.3, SVBD column]`

`char::from(u8)` maps a byte to the Latin-1 code point, which is the right
default for a VB6 header string. Do **not** use `String::from_utf8_lossy`,
because a byte in 0x80 to 0xFF would become U+FFFD and the name would be lost.
The full Windows-1252 mapping arrives with `VbStr` in Phase 3.

**Convert an RVA to a file offset.** This is not on `Region`. It needs the
section table, so it lives on `PeImage`. See section 5.

```rust
let off: Off = pe.rva_to_off(Rva::new(0x1760))?;
let region: Region<'_> = pe.region_at(Rva::new(0x1760))?;
let region: Region<'_> = pe.region_at_va(Va::new(0x401760))?;
```

## 3.4 What `checked_add` looks like with no `Add`

The newtype does not implement `std::ops::Add`, so there is no `+`. The
inherent method has the same name as the primitive one, so the shape a reader
already knows is the shape that compiles:

```rust
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

Two details.

`Option::map` is not a `const fn` on 1.97.1, so a `const fn` wrapper must use
`match`. `[VERIFIED: local]` If the `const` is not wanted, `.map(Self)` is
shorter. Keep the `const`: it lets a table of fixed offsets be built at compile
time later.

The right-hand side is a bare `u32`, not another `Off`. Adding two offsets is
meaningless, and the signature refuses it. A length is a `u32`. That is the
whole point of the newtype: `off.checked_add(len)` compiles, `off + other_off`
does not exist, and `off.get() + len.get()` is stopped by
`clippy::arithmetic_side_effects`.

`Va::to_rva` is the only bridge between the three spaces, and it is a
`checked_sub`:

```rust
pub const fn to_rva(self, image_base: u32) -> Option<Rva> {
    match self.0.checked_sub(image_base) {
        Some(v) => Some(Rva(v)),
        None => None,
    }
}
```

A VA below the image base returns `None` instead of a huge wrapped address that
lands inside the file.

## 3.5 The two casts inside `Region`, and why they are safe

```rust
fn index(self) -> usize { usize::try_from(self.0).unwrap_or(usize::MAX) }
pub fn len(&self) -> u32 { u32::try_from(self.bytes.len()).unwrap_or(u32::MAX) }
```

Neither uses `as`. `unwrap_or` is not `unwrap`, so `clippy::unwrap_used` does
not fire. `[VERIFIED: local]` The saturating fallbacks are correct in both
directions: a `usize::MAX` index fails the following `get`, and a `u32::MAX`
length over-reports a file larger than 4 GiB, which then fails every bounds
check that uses it. Both failures are refusals, not reads.

---

# 4. `object` 0.40.0, concretely

All type names, method names and error strings below were read from
`~/.cargo/registry/src/index.crates.io-*/object-0.40.0/src/read/pe/` and then
compiled and run. `[VERIFIED: local]`

## 4.1 The imports

```rust
use object::LittleEndian as LE;
use object::read::pe::{ImageNtHeaders as _, ImageOptionalHeader as _, PeFile32};
```

`ImageNtHeaders` and `ImageOptionalHeader` are traits. Without them in scope,
`nt.file_header()`, `nt.optional_header()`, `.magic()`,
`.address_of_entry_point()` and `.image_base()` do not resolve. Import them as
`_`, because nothing names them directly.

## 4.2 Open a PE32 from a byte slice

```rust
let file: PeFile32<'a> = PeFile32::parse(data)?;   // data: &'a [u8]
```

`PeFile32<'data, R = &'data [u8]>` is
`PeFile<'data, pe::ImageNtHeaders32, &'data [u8]>`. Passing a `&[u8]` needs no
extra type annotation.

**`parse` validates headers only.** It does not check that the section raw
data exists. See §4.7.

## 4.3 Machine, magic, entry point and image base

```rust
let nt = file.nt_headers();

// i386 only. Refuse anything else.
if nt.file_header().machine.get(LE) != object::pe::IMAGE_FILE_MACHINE_I386 { /* refuse */ }

// PE32, not PE32+.
if nt.optional_header().magic() != object::pe::IMAGE_NT_OPTIONAL_HDR32_MAGIC { /* refuse */ }

let entry: u32 = nt.optional_header().address_of_entry_point();  // an RVA
let base:  u64 = nt.optional_header().image_base();              // widened to u64
```

Two shapes to note.

`machine` is a field, not a method, and it is an endian-wrapped `U16<LE>`, so it
needs `.get(LE)`. `magic()` is a trait method and returns a plain `u16`.

`image_base()` returns `u64` even on `ImageOptionalHeader32`, because the trait
is shared with PE32+. Narrow it with `u32::try_from(...).unwrap_or(u32::MAX)`,
never with `as u32`, which the wall rejects.

`address_of_entry_point()` is an **RVA**, not a VA and not an offset.

## 4.4 Enumerate the sections

```rust
let st = file.section_table();          // SectionTable<'data>
let count: usize = st.len();
for s in st.iter() {                    // s: &'data pe::ImageSectionHeader
    let name: [u8; 8] = s.name;                          // field, not endian wrapped
    let va:    u32    = s.virtual_address.get(LE);       // an RVA
    let vsize: u32    = s.virtual_size.get(LE);
    let praw:  u32    = s.pointer_to_raw_data.get(LE);   // a file offset
    let rsize: u32    = s.size_of_raw_data.get(LE);
    let chars: u32    = s.characteristics.get(LE);
}
```

`ImageSectionHeader` also offers `pe_file_range()`, `pe_file_range_at(va)`,
`pe_address_range()`, `pe_data(data)`, `pe_data_at(data, va)`,
`contains_rva(va)` and `pe_data_containing(data, va)`. Section 5 explains why
DeForm6 reimplements the map instead of calling them.

## 4.5 Enumerate imported DLL names

```rust
pub fn imported_dlls(&self) -> Result<Vec<String>, object::Error> {
    let mut out = Vec::new();
    let Some(it) = self.file.import_table()? else { return Ok(out) };
    let mut d = it.descriptors()?;
    while let Some(desc) = d.next()? {
        let name: &[u8] = it.name(desc.name.get(LE))?;
        out.push(String::from_utf8_lossy(name).to_ascii_uppercase());
    }
    Ok(out)
}
```

Three shapes to note.

`import_table()` returns `Result<Option<ImportTable<'data>>>`. `Ok(None)` means
the image has no import data directory. That is not an error, and for DeForm6
it becomes "no Visual Basic runtime", not "damaged".

`ImportDescriptorIterator::next` is a **fallible** iterator with an inherent
`next` that returns `Result<Option<_>>`. It does not implement
`Iterator`, so `for` does not work and `?` does. The `while let Some(x) =
d.next()?` shape is the one that compiles.

`desc.name` is an RVA into the section that holds the import directory.
`it.name(rva)` resolves it against the section data the `ImportTable` was built
from, and returns the bytes without the NUL.

The delay-load path has the same shape:

```rust
let Some(dt) = self.file.delay_load_import_table()? else { return Ok(out) };
let mut d = dt.descriptors()?;
while let Some(desc) = d.next()? {
    let name = dt.name(desc.dll_name_rva.get(LE))?;
}
```

## 4.6 What `object` returns for a malformed file

`object::Error` is `pub struct Error(pub(crate) &'static str)` with
`#[derive(Debug, Clone, Copy, PartialEq, Eq)]`.
`[VERIFIED: local, object-0.40.0/src/read/mod.rs:117-127]`

**The field is private.** The only way to reach the text is `Display`, that is
`e.to_string()`, which allocates. Two consequences for `error.rs`:

- `object::Error` is `Copy` and `Eq`, so `PeReject::NotPe(object::Error)`
  carries it with no allocation and stays `Copy`. That is what the seam does.
- `object::Error` does **not** implement `serde::Serialize`. If a `Defect` must
  carry the text into the Phase 4 report, convert with `.to_string()` at that
  boundary and store a `String`. Do not try to borrow the `&'static str`.

Observed messages, produced today: `[VERIFIED: local]`

| Input | `PeFile32::parse` result |
|---|---|
| empty slice | `Err("Invalid DOS header size or alignment")` |
| `b"MZ"` | `Err("Invalid DOS header size or alignment")` |
| `b"this is not a program\n"` | `Err("Invalid DOS header size or alignment")` |
| 64 bytes, `e_lfanew = 0xFFFFFFFE` | `Err("Invalid PE headers offset or size")` |
| MZ with `e_lfanew` valid, no `PE\0\0` | `Err("Invalid PE magic")` |
| NE image (`NE` at `e_lfanew`) | `Err("Invalid PE magic")` |
| `PE\0\0` with a zero COFF header | `Err("Invalid PE optional header magic")` |
| PE32+ image parsed as `PeFile32` | `Err("Invalid PE optional header magic")` |
| a corpus file cut to 1/64 | `Err("Invalid COFF/PE section headers")` |

None panicked.

**The mapping into our `Error`.** All of these become one variant. There is no
value in surfacing nine different English strings from a third-party crate to a
user who typed a filename.

```rust
PeFile32::parse(data).map_err(PeReject::NotPe)?          // exit code 1
```

Keep the `object::Error` inside `PeReject::NotPe` so the report can print it as
an evidence field, and print one sentence of our own to the user:
`"<path> is not a PE executable."` The two failures after the parse get their
own variants, because they are ours, not `object`'s:

```rust
PeReject::NotI386     // machine is not 0x014C
PeReject::NotPe32     // optional header magic is not 0x010B
```

Both fold into exit code 1 as well: a 64-bit or ARM image is not a thing this
tool reads, and the sentence should say so.

## 4.7 A successful parse is **not** evidence of an intact file

`PeFile32::parse` returned `Ok` for the Mandelbrot corpus file truncated to
1/2, 1/4 and **1/8** of its 28,672 bytes. It also returned `Ok` after one byte
in the middle of the file was flipped. `[VERIFIED: local]`

Header validation is all it does. Every read after the parse must go through
the bounded `Region`, which checks against the real slice length. The
end-to-end behaviour of the skeleton, measured:

```
1/1  (28672 bytes) -> Ok("Mandelbrot_Fractal_Demo")
1/2  (14336 bytes) -> Err(Damaged("the import directory is unreadable"))
1/4  ( 7168 bytes) -> Err(Damaged("the import directory is unreadable"))
1/8  ( 3584 bytes) -> Err(Damaged("the import directory is unreadable"))
1/16 ( 1792 bytes) -> Err(Damaged("the import directory is unreadable"))
1/64 (  448 bytes) -> Err(NotPe)
```

A truncated VB6 file therefore reaches exit code 4, not exit code 1. That is
the right answer and it is what the exit-code table intends.

## 4.8 What `object` does not expose, and must be read by hand

| Thing | Why `object` cannot give it | What DeForm6 does |
|---|---|---|
| The entry-point stub bytes as a bounded window | `pe_data_at` returns a bare `&[u8]` with no offset provenance, so an error cannot name a file offset | `PeImage::region_at(entry_rva)` returns a `Region` whose `base` is the file offset |
| Everything from `VBHeader` onward | It is not a PE structure | `region.rs` reads it |
| A stable RVA-to-offset rule for the zero-filled tail | `pe_file_range_at` and `contains_rva` disagree by design, see §5.3 | one rule, in `PeImage::section_for` |
| The file offset of an imported DLL name | `ImportTable::name` gives bytes, never a position | `PeImage::dll_name_sites()` joins `desc.name` through `rva_to_off` |
| The error text of `object::Error` as data | the field is private | `.to_string()` at the report boundary only |
| Whether the delay-load descriptor uses RVAs or VAs | `DelayLoadImportTable::name` treats the value as an RVA unconditionally and never reads `attributes` | see §6.4 |

`PeImage::dll_name_sites()` is worth its ten lines twice over. The report needs
a byte offset as evidence for "the runtime is MSVBVM60.DLL", and the refusal
tests need it to patch a name in place (see §7.3).

```rust
pub fn dll_name_sites(&self) -> Result<Vec<(Off, u32)>, object::Error> {
    let mut out = Vec::new();
    let Some(it) = self.file.import_table()? else { return Ok(out) };
    let mut d = it.descriptors()?;
    while let Some(desc) = d.next()? {
        let rva = Rva::new(desc.name.get(LE));
        let name = it.name(desc.name.get(LE))?;
        if let Some(off) = self.rva_to_off(rva) {
            out.push((off, u32::try_from(name.len()).unwrap_or(u32::MAX)));
        }
    }
    Ok(out)
}
```

## 4.9 The seam, as it now stands

Ten methods, all compiled. `[VERIFIED: local, /tmp/df6ws/crates/deform6/src/pe.rs]`

```rust
impl<'a> PeImage<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, PeReject>;
    pub const fn image_base(&self) -> u32;
    pub fn entry_rva(&self) -> Rva;
    pub fn sections(&self) -> &[SectionInfo];
    pub fn rva_to_off(&self, rva: Rva) -> Option<Off>;
    pub fn va_to_off(&self, va: Va) -> Option<Off>;
    pub fn section_for(&self, rva: Rva) -> Option<&SectionInfo>;
    pub fn region_at(&self, rva: Rva) -> Option<Region<'a>>;
    pub fn region_at_va(&self, va: Va) -> Option<Region<'a>>;
    pub fn imported_dlls(&self) -> Result<Vec<String>, object::Error>;
    pub fn delay_loaded_dlls(&self) -> Result<Vec<String>, object::Error>;
    pub fn dll_name_sites(&self) -> Result<Vec<(Off, u32)>, object::Error>;
    pub fn has_clr_header(&self) -> bool;
}
```

`has_clr_header` reads data directory 14,
`object::pe::IMAGE_DIRECTORY_ENTRY_COM_DESCRIPTOR`, which is a `usize` constant.
`[VERIFIED: local, object-0.40.0/src/pe.rs:686]` It is one line and it lets the
tool say "this is a .NET assembly" instead of "this holds no Visual Basic
runtime". It is optional for this phase. See §7.4.

`SectionInfo` is our own owned copy, so nothing outside `pe.rs` holds an
`object` type:

```rust
pub struct SectionInfo {
    pub name: [u8; 8],
    pub virtual_address: Rva,
    pub virtual_size: u32,
    pub pointer_to_raw_data: Off,
    pub size_of_raw_data: u32,
}
```

---

# 5. Where the address-to-offset map belongs, and the four awkward cases

## 5.1 Where it belongs

**In `read/pe.rs`, on `PeImage`, over the owned `Vec<SectionInfo>`.** Not in
`region.rs`, and not by calling `object`'s helpers.

Three reasons.

The map needs the section table, and the section table arrives with the PE
parse. Putting it in `region.rs` would force `region.rs` to know about
sections, and `region.rs` is the one file in the crate with zero dependencies
and zero domain knowledge.

`object` already has `SectionTable::pe_file_range_at`, and DeForm6 does not
call it, because it returns `(u32, u32)` with no provenance. DeForm6 needs the
answer as an `Off` and a `Region` whose `base` is that `Off`, so that a defect
three levels down still names a file offset.

The rules in §5.3 must not change when `object` changes. Behind the seam, they
are ours.

## 5.2 What the PE format actually guarantees

Quoted from the Microsoft PE format specification.
`[CITED: https://learn.microsoft.com/en-us/windows/win32/debug/pe-format]`

> **VirtualSize**: "The total size of the section when loaded into memory. If
> this value is greater than SizeOfRawData, the section is zero-padded."

> **SizeOfRawData**: "The size of the section (for object files) or the size of
> the initialized data on disk (for image files). For executable images, this
> must be a multiple of FileAlignment from the optional header. If this is less
> than VirtualSize, the remainder of the section is zero-filled. Because the
> SizeOfRawData field is rounded but the VirtualSize field is not, it is
> possible for SizeOfRawData to be greater than VirtualSize as well."

> "In an image file, the VAs for sections must be assigned by the linker so
> that they are in ascending order and adjacent, and they must be a multiple of
> the SectionAlignment value in the optional header."

> "Section data must appear in order of the RVA values for the corresponding
> sections (as do the individual section headers in the section table)."

So the format **requires** a linker to produce sections that ascend and do not
overlap. It does not give a parser any way to rely on that, because a hostile
file is not the output of a linker. DeForm6 must define behaviour for the
violation anyway, and it must not assume sorted input.

## 5.3 The four rules

Let `d = rva - section.virtual_address`, computed with `checked_sub`.
Let `mapped_len = min(virtual_size, size_of_raw_data)`.

### Case A: virtual size exceeds raw size

**Rule: `mapped_len` is `min(virtual_size, size_of_raw_data)`, always.**

This handles both directions the spec allows. When `virtual_size` is larger,
the difference is the BSS tail, and the file holds no byte for it. When
`size_of_raw_data` is larger, the difference is `FileAlignment` padding, which
the loader does not map, so reading it would return bytes at an address the
program never sees.

```rust
impl SectionInfo {
    pub fn mapped_len(&self) -> u32 {
        self.virtual_size.min(self.size_of_raw_data)
    }
}
```

`object` uses the same `min` in `ImageSectionHeader::pe_file_range`.
`[VERIFIED: local, object-0.40.0/src/read/pe/section.rs:388-393]`

Measured on the corpus: **7 of 132 sections have `virtual_size >
size_of_raw_data`**, all of them `.data`. The other 125 have `size_of_raw_data >
virtual_size`. **No section has `virtual_size == 0`**, and **no section declares
raw data past the end of its file**. `[VERIFIED: local, sweep over all 44]`

The `virtual_size == 0` case matters even though the corpus does not show it: an
old linker writes 0 there for an object file, and `min` would then make every
RVA in that section unmapped. Do not add a fallback that swaps in
`size_of_raw_data` when `virtual_size` is 0. That would let a hand-made file
turn any padding into readable data. Refuse, and name the section.

### Case B: an RVA inside a section's zero-filled tail

**Rule: `None`. Report `DefectKind::UnmappedAddress`. Do not synthesise zeros.**

The tail exists at run time and does not exist in the file. A parser that
returned a zero would let a VB pointer chain "succeed" into a run of zeros and
report a project with zero objects rather than a refusal.

This is where `object`'s two helpers disagree, and it is why DeForm6 does not
mix them:

- `ImageSectionHeader::contains_rva(va)` compares `d < virtual_size`, so it
  answers **true** for an address in the tail.
- `ImageSectionHeader::pe_file_range_at(va)` compares `d < min(vsize, rawsize)`,
  so it returns **None** for the same address.

`[VERIFIED: local, object-0.40.0/src/read/pe/section.rs:395-445]`

`SectionTable::section_containing` is built on `contains_rva`, so it and
`pe_file_range_at` can give opposite answers for one address. DeForm6 uses one
predicate everywhere:

```rust
pub fn section_for(&self, rva: Rva) -> Option<&SectionInfo> {
    self.sections.iter().find(|s| {
        match rva.get().checked_sub(s.virtual_address.get()) {
            Some(d) => d < s.mapped_len(),
            None => false,
        }
    })
}
```

`section_for`, `rva_to_off` and `region_at` all call it, so they cannot
disagree.

Measured: with this rule, all 44 corpus files resolve the entry point, the VB
header, `ProjectInfo` and the four header strings. No VB pointer in the corpus
lands in a BSS tail. `[VERIFIED: local, 44 of 44]`

### Case C: an RVA in no section

**Rule: `None`. Report `DefectKind::UnmappedAddress { va }`. Fatal severity for
the four spine pointers, recoverable for a leaf.**

Do not fall back to treating the RVA as a file offset. That fallback is the
classic way a decompiler reports real bytes from the wrong place: on a
`FileAlignment == SectionAlignment` image the two often agree, so the bug hides
until one file where they do not.

Do not fall back to the PE headers region either. An RVA below the first
section's `virtual_address` is inside the headers at run time, but nothing VB
writes points there, and accepting it widens the attack surface for no gain.

### Case D: overlapping sections

**Rule: the first section in section-table order that satisfies the predicate
wins. Record a `Defect` at severity `Recoverable` when any two sections
overlap, and check that once at parse time, not per lookup.**

`find` gives first-match, which is deterministic and matches what `object` does
with `find_map`. `[VERIFIED: local, object-0.40.0/src/read/pe/section.rs:350-361]`
It also matches the Windows loader, which maps sections in table order, so a
later section that overlaps an earlier one wins in memory. The two disagree.
That disagreement is exactly why the overlap must be reported rather than
silently resolved: a file with overlapping sections is either damaged or built
to make two parsers see different bytes.

The check, once, after the section list is built:

```rust
// O(n^2) over at most 96 sections. Cheap and obvious.
for (i, a) in sections.iter().enumerate() {
    for b in sections.iter().skip(i + 1) {
        // ranges [va, va + mapped_len) must not intersect
    }
}
```

Compute both ends with `checked_add` and treat an overflowing end as an overlap
report, not as an end of `u32::MAX`.

The corpus has no overlap: all 44 files have exactly 3 sections, `.text`,
`.data`, `.rsrc`, ascending. `[VERIFIED: local]` So this rule is untested by
construction until a fuzz input or a hand-made regression file exercises it.
Say that in the test, in the same way the P-code branch is called out.

## 5.4 Corpus facts the map rests on

`[VERIFIED: local, sweep over all 44 executables]`

| Fact | Result |
|---|---|
| Files that `PeFile32::parse` accepts | 44 of 44 |
| `machine` | `0x014C` in 44 of 44 |
| Optional header `magic` | `0x010B` in 44 of 44 |
| `ImageBase` | `0x00400000` in 44 of 44 |
| Section count | 3 in 44 of 44 (`.text`, `.data`, `.rsrc`) |
| Distinct `AddressOfEntryPoint` values | 38 of 44 |
| Sections declaring raw data past EOF | 0 of 132 |

The 38 distinct entry-point values confirm what `CORPUS.md` says: the address
map is required, not a convenience.

---

# 6. Runtime discrimination

## 6.1 The exact strings

```rust
pub const VB6_DLL:    &str = "MSVBVM60.DLL";
pub const VB5_DLL:    &str = "MSVBVM50.DLL";
pub const VB4_32_DLL: &str = "VB40032.DLL";
pub const VB4_16_DLL: &str = "VB40016.DLL";
```

`[CITED: .planning/research/STRUCTURES.md §1.2, SVBD branch]`

## 6.2 Case sensitivity

**The PE import directory stores the name as raw bytes with no case rule.**
Nothing in the PE specification normalises it. The Windows loader resolves a
module name case-insensitively, so a linker is free to write any case, and
different linkers do.

**Compare case-insensitively. Match on an upper-cased copy.**

```rust
out.push(String::from_utf8_lossy(name).to_ascii_uppercase());
```

`to_ascii_uppercase` and not `to_uppercase`. A DLL name is ASCII, and the
Unicode uppercase mapping of a stray high byte could produce a longer string
and a wrong comparison.

Measured on the corpus: all 44 executables import exactly one DLL, and its name
is the literal ASCII `MSVBVM60.DLL`, already upper case.
`[VERIFIED: local, 44 of 44]` So case folding is never exercised by the corpus.
Keep it anyway. It costs one method call, and the counter-example is a file
nobody has yet.

## 6.3 A packed import table

If a packer rewrote the entry point and the import directory, none of this
holds. Three sub-cases, in order of likelihood:

| What the file looks like | What DeForm6 does |
|---|---|
| `import_table()` is `Ok(None)`, that is, no import data directory | `Refusal::NoVbRuntime`, exit 2 |
| `import_table()` is `Err(...)`, that is, the directory RVA is unmapped or truncated | `Refusal::Damaged`, exit 4 |
| The directory parses and names only the packer's stub imports | `Refusal::NoVbRuntime`, exit 2 |

**Do not add a fallback scan for `MSVBVM60.DLL` in this phase.** `STRUCTURES.md`
§1.3 offers a raw scan of executable sections for the bytes `56 42 35 21`
(`VB5!`) as the packed-file fallback, and it belongs in Phase 5 with
`--salvage`, behind an explicit flag, not in the default path. A default that
scans for a four-byte magic will find one in the `.rsrc` of an unrelated
program and report a VB6 project that does not exist. That is a wrong answer
stated confidently, which the project constitution forbids.

## 6.4 A delay-loaded import

**A delay-loaded VB runtime does not occur and must not be trusted.** Three
facts.

The corpus has no delay-load directory at all: `delay_load_import_table()`
returned `Ok(None)` for 44 of 44. `[VERIFIED: local]`

The VB6 linker does not emit one for `MSVBVM60.DLL`. The runtime is resolved
at load time, because `ThunRTMain` is called from the entry stub itself, before
any user code runs. `[CITED: .planning/research/STRUCTURES.md §1.1]`

`object`'s `DelayLoadImportTable::name(address)` computes
`address.wrapping_sub(section_address)` and never reads the descriptor's
`attributes` field. `[VERIFIED: local, object-0.40.0/src/read/pe/import.rs:340-346]`
The old Visual C++ 6 delay-load format stores VAs there rather than RVAs, and
the `attributes` bit distinguishes them. The Microsoft PE specification does not
document that bit. `[VERIFIED: absent from https://learn.microsoft.com/en-us/windows/win32/debug/pe-format]`
Its name in `delayimp.h` is `dlattrRva = 0x1`, which is training knowledge, not
something confirmed this session. `[ASSUMED]`

**Rule.** Read the delay-load table with `PeImage::delay_loaded_dlls()`, print
what it holds as an informational line, and **never** let it decide the runtime.
A file whose only VB runtime reference is delay-loaded is refused with
`NoVbRuntime` and the message says the delay-load table was seen. A `wrapping_sub`
on a garbage address cannot panic, so the worst case is an `Err` from a bounds
check, which is a `Damaged` report.

## 6.5 `VB40016.DLL` is unreachable in this phase

A 16-bit VB4 program is an NE executable, not a PE one. `PeFile32::parse` on an
image whose `e_lfanew` points at `NE` returns `Err("Invalid PE magic")`.
`[VERIFIED: local]` The refusal is therefore `NotPe`, exit code 1, and the
`VB40016.DLL` string is never compared.

Keep the constant, because it documents the intent and costs nothing, but do
not plan a test that asserts a VB4 16-bit file reaches `Refusal::IsVb4`. Write
the test that asserts an NE image reaches `NotPe`, which is the true behaviour.
`VB40032.DLL` is reachable, because VB4 32-bit produces a PE.

## 6.6 The exit code for VB4 is undefined, and this is the safe default

The CONTEXT.md table defines code 3 as "Visual Basic 5, not Visual Basic 6". It
defines no code for VB4, and ROADMAP plan 01-06 names `VB40032.DLL` as one of
the three refusals. This is a real gap in the locked decision.

**Safe default: map VB4-32 to exit code 3 and widen the meaning of code 3 to "a
Visual Basic runtime that DeForm6 does not read".** The sentence still names
the version, so the human loses nothing, and a script that sorts a directory by
exit code gets the right bucket: this file is Visual Basic and this tool cannot
read it.

The alternative, a sixth code, moves no existing number and stays open. Raise
it with the user; do not decide it in a plan.

## 6.7 Confidence, and the fallback when the import table is gone

**Confidence in the discrimination itself is HIGH.** The runtime DLL name is
the discriminator the format itself uses, four independent implementations use
it, and 44 of 44 corpus files agree. `[VERIFIED: local]`
`[CITED: .planning/research/STRUCTURES.md §1.2]`

**Confidence that the import table is present is not the same thing.** For a
file with no readable import directory, DeForm6 has no second opinion in this
phase, and it should not pretend to. The `VB5!` signature does not help,
because `STRUCTURES.md` §2 records that VB5 writes the same four bytes.

**The fallback is to refuse and to say which check failed.** One sentence, one
exit code, and a `Defect` naming the byte offset of the import data directory.
Phase 5 adds `--salvage`, and only then does the `VB5!` scan become reachable,
behind that flag, with every result marked inferred.

## 6.8 Discrimination corpus facts

`[VERIFIED: local, 44 of 44]`

| Fact | Result |
|---|---|
| Files importing exactly one DLL | 44 of 44 |
| That DLL is `MSVBVM60.DLL` | 44 of 44 |
| Files with a delay-load directory | 0 of 44 |
| Entry byte is `0x68` | 44 of 44 |
| The pushed VA lands on `VB5!` | 44 of 44 |
| `ProjectInfo + 0x20` (`lpNativeCode`) non-zero | 44 of 44 |
| `VBHeader + 0x2C` (`lpSubMain`) is zero | 44 of 44 |
| `wFormCount` | 1 in 37 files, 2 in 6, 4 in 1 |
| `wRuntimeBuild` | `9782` in 34 files, `8176` in 10 |

`lpSubMain == 0` in every file means the startup object is a form in all 44.
`STRUCTURES.md` §2 says so and the corpus agrees. The `Sub Main` branch is
untested by construction, in the same way the P-code branch is. Say so.

`wRuntimeBuild` 9782 is `0x2636` and 8176 is `0x1FF0`. Neither equals AG's
sample value `0x231C`. Read the field, print it, and do not compare it against a
literal in a test.

---

# 7. Testing shape for this phase

## 7.1 Where each file goes

```
crates/deform6/tests/
├── refusal.rs        # every refusal, from fixtures built inside the test
└── corpus_sweep.rs   # success criterion 2, over all 44 executables

crates/deform6-cli/tests/
└── cli.rs            # exit codes only
```

`differential.rs`, `ratios.toml`, `regressions.rs` and `support/` arrive in
Phase 2 and Phase 5. Do not create empty versions of them now.

Every file under `tests/` opens with the inner attribute block from §2.5.

## 7.2 How `refusal.rs` is organised

One `fn` per refusal, named for the behaviour rather than for the variant, plus
two helpers at the top. No module nesting: the file is small and the file-level
`#![allow]` covers it.

```rust
#![allow(
    clippy::unwrap_used, clippy::expect_used, clippy::panic,
    clippy::indexing_slicing, clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

use deform6::pe::PeImage;
use deform6::vb::{Refusal, inspect};

fn corpus_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

fn a_vb6_exe() -> Vec<u8> { /* read Mandelbrot.exe */ }

fn patch_runtime_name(data: Vec<u8>, new_name: &[u8]) -> Vec<u8> { /* §7.3 */ }

#[test] fn an_empty_file_is_not_a_pe() { ... }
#[test] fn a_text_file_is_not_a_pe() { ... }
#[test] fn a_pe_with_no_vb_runtime_is_refused_by_that_name() { ... }
#[test] fn a_dot_net_assembly_is_refused_as_holding_no_vb_runtime() { ... }
#[test] fn a_vb5_executable_is_refused_by_that_name() { ... }
#[test] fn a_vb4_executable_is_refused_by_that_name() { ... }
#[test] fn a_truncated_vb6_executable_is_damaged_not_refused_as_a_non_pe() { ... }
#[test] fn the_unmodified_corpus_file_is_accepted() { ... }
```

Eight tests, all passing in the scratch workspace. `[VERIFIED: local]`

The last one is the control. Without it, a bug that made `inspect` refuse
everything would leave all seven refusal tests green. `AGENTS.md` warns about
exactly that failure shape.

Assert on the variant, never on the sentence:

```rust
assert_eq!(inspect(&data).unwrap_err(), Refusal::IsVb5);
```

`Refusal` derives `PartialEq, Eq, Debug`. `Report` must derive them too, or
`assert_eq!` on a `Result<Report, Refusal>` does not compile. That is a small
trap and it cost a compile cycle in this session. `[VERIFIED: local]`

## 7.3 How the fixtures are built, with no vendored binary

**The construction: patch an imported DLL name in memory, at the offset the PE
itself names.** The bytes never touch the disk and never enter the repository.

```rust
/// `new_name` must be exactly as long as the name it replaces, so no offset moves.
fn patch_runtime_name(mut data: Vec<u8>, new_name: &[u8]) -> Vec<u8> {
    let sites = {
        let pe = PeImage::parse(&data).expect("the corpus file is a PE32");
        pe.dll_name_sites().expect("the import directory is readable")
    };
    let (off, len) = *sites.first().expect("the corpus file imports at least one DLL");
    assert_eq!(len as usize, new_name.len(),
        "the replacement name must be the same length as the original");
    let start = off.get() as usize;
    let end = start + new_name.len();
    data.splice(start..end, new_name.iter().copied());
    data
}
```

The offset comes from `dll_name_sites()`, which reads the import descriptor and
maps its `name` RVA through the section table. It is not a byte search. A byte
search for `MSVBVM60.DLL` could hit the same bytes in `.rsrc` or in a string
table, and `AGENTS.md` names that class of mistake directly.

The three fixtures:

| Fixture | Call | Result | Exit |
|---|---|---|---|
| Not a PE | `inspect(&[])` and `inspect(b"this is not a program\n")` | `Refusal::NotPe` | 1 |
| PE, no VB | `patch_runtime_name(exe, b"KERNEL32.DLL")` | `Refusal::NoVbRuntime` | 2 |
| PE, .NET | `patch_runtime_name(exe, b"mscoree.dll\0")` | `Refusal::NoVbRuntime` | 2 |
| VB5, not VB6 | `patch_runtime_name(exe, b"MSVBVM50.DLL")` | `Refusal::IsVb5` | 3 |
| VB4-32 | `patch_runtime_name(exe, b"VB40032.DLL\0")` | `Refusal::IsVb4` | 3 |
| VB6, truncated | `exe[..exe.len() / 2]` | `Refusal::Damaged(..)` | 4 |

All six run and pass. `[VERIFIED: local]`

`MSVBVM60.DLL` is 12 bytes. `VB40032.DLL` and `mscoree.dll` are 11, so the
replacement carries its own trailing NUL and the original NUL becomes a second,
harmless one. The equal-length assertion inside the helper makes a wrong-length
replacement a loud failure rather than a corrupted image.

**Why this is permitted.** `AGENTS.md` forbids "a binary, a source file, or a
derived fixture from a third party system that the author does not own" from
entering the repository, "including a fixture that was calculated from such a
file". Nothing enters. The bytes exist for the duration of one test function, in
a `Vec<u8>`, and the source they come from is a `corpus/` program that is
already committed under BSD-2 or the Unlicense with its origin recorded in
`corpus/NOTICES`. The same reasoning that STACK.md §5 gives for committed fuzz
crash inputs applies with more room, because these are not committed at all.

**What this does not prove, said plainly.** A patched import name proves the
discrimination logic. It does not prove that DeForm6 handles a genuine VB5
binary, whose `VBHeader` layout differs after 0x30 (`STRUCTURES.md` §10.1), nor
a genuine .NET assembly, which carries a CLR header the patched file does not.
DeForm6 refuses both before it reads any of that, so the test covers the
behaviour that phase 1 promises. When `corpus/manifest.toml` gains a real VB5
sample in Phase 5, add a second test beside this one. Do not delete this one:
it runs offline and it runs on a clean clone.

## 7.4 An optional stronger .NET refusal

`PeImage::has_clr_header()` reads data directory 14 and is one line (§4.9). With
it, a real .NET assembly gets `Refusal::IsDotNet` and a sentence that names the
right thing, instead of "holds no Visual Basic runtime".

**Recommendation: add the method, do not add the variant in this phase.** The
method is free and the report can use it as evidence. The variant needs a sixth
exit code or an overload of code 2, and the exit-code table is a locked
decision. The success criterion asks for three distinct variants from a VB5
file, a .NET file and an empty file, and `IsVb5`, `NoVbRuntime` and `NotPe` are
three distinct variants. The criterion is met without it.

## 7.5 The 44-executable sweep, as a test

Success criterion 2 says "a script runs `inspect` over all 44 executables". Write
it as a `#[test]`, so `cargo test --workspace` is the whole gate and nothing has
to be remembered.

```rust
#[test]
fn every_corpus_executable_is_read() {
    let files = executables();                 // recursive walk, extension eq_ignore_ascii_case("exe")
    assert_eq!(files.len(), 44,
        "the corpus holds 44 executables; found {}", files.len());

    let mut failed = Vec::new();
    for p in &files {
        let data = std::fs::read(p).unwrap();
        match inspect(&data) {
            Ok(r) => if !r.native || r.project_name.is_empty() {
                failed.push(format!("{}: {r:?}", p.display()));
            },
            Err(e) => failed.push(format!("{}: {e:?}", p.display())),
        }
    }
    assert!(failed.is_empty(),
        "{} of {} corpus executables did not read:\n{}",
        failed.len(), files.len(), failed.join("\n"));
}
```

It passes. `[VERIFIED: local, 44 of 44 in 0.01 s]`

Four properties the plan must keep.

**The count is asserted first.** A sweep over an empty list passes and proves
nothing. This is the same guard STACK.md puts on the regression replay.

**The count is a literal, `44`.** It is the number `CORPUS.md` records and
`ROADMAP.md` repeats. When somebody adds a corpus program, this test fails and
forces the pinned numbers elsewhere to be looked at. That failure is the point.

**Failures are collected, not returned early.** A run that names all the files
that broke is worth ten runs that each name one.

**The walk is case-insensitive on the extension.** The corpus already holds
`MCI.VBP` in upper case, so a case-sensitive glob would silently miss a file. A
`*.exe` glob in the shell would too. `[VERIFIED: local, find corpus -name "*.vbp" gives 44 and -iname gives 45]`

**Do not use `std::fs::read_dir` with `?` in a helper that returns
`Vec<PathBuf>`.** Use `unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))`
so an unreadable directory names itself. The file-level `#![allow]` permits it.

The sweep takes 0.01 s. There is no reason to gate it behind a feature or an
`--ignored` flag.

## 7.6 How a test asserts an exit code

```rust
fn run(args: &[&std::ffi::OsStr]) -> (i32, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_deform6"))
        .args(args)
        .output()
        .unwrap();
    let code = out.status.code().expect("the process was not signalled");
    (code, String::from_utf8_lossy(&out.stderr).into_owned())
}
```

`CARGO_BIN_EXE_<name>` is set by cargo for an integration test in the same
package as the binary. `<name>` is the **bin target name**, so it is
`CARGO_BIN_EXE_deform6`, from the `[[bin]] name = "deform6"` line, not
`CARGO_BIN_EXE_deform6_cli`. It needs no dev-dependency and no path guessing.
`[VERIFIED: local]`

`status.code()` returns `Option<i32>`, and it is `None` when the process was
killed by a signal. Use `.expect(...)`, so a segfault reports as "the process
was not signalled" rather than as a wrong number.

Pass the failing stderr into the assertion message. When exit code 1 arrives
instead of 0, the sentence the tool printed is the whole diagnosis:

```rust
assert_eq!(code, 0, "stderr was: {msg}");
```

Four tests, all passing: `[VERIFIED: local]`

| Test | Input | Code |
|---|---|---|
| `a_vb6_executable_exits_zero` | `inspect corpus/vb6-code/Mandelbrot/Mandelbrot.exe` | 0 |
| `a_non_pe_exits_one` | a text file the test writes into `std::env::temp_dir()` | 1 |
| `a_missing_file_is_an_internal_error` | `inspect /nonexistent/x.exe` | 5 |
| `a_bad_subcommand_does_not_collide_with_code_two` | `frobnicate` | 5 |

## 7.7 `clap` uses exit code 2, and that is a collision

**`clap` exits with 2 on a usage error.** Code 2 in the locked table means "a PE
file, but it holds no Visual Basic runtime". A shell loop that sorts a directory
by exit code would put a typo in the arguments into the "not Visual Basic"
bucket. `[VERIFIED: local, the collision was observed before it was fixed]`

`Cli::parse()` calls `std::process::exit` itself, so the mapping cannot be done
after it. Use `try_parse`:

```rust
fn main() -> ExitCode {
    use clap::Parser as _;
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            return match e.kind() {
                clap::error::ErrorKind::DisplayHelp
                | clap::error::ErrorKind::DisplayVersion => Exit::Ok.into(),
                _ => Exit::Internal.into(),
            };
        }
    };
    match cli.command {
        Command::Inspect { input } => run_inspect(&input).into(),
    }
}
```

`e.print()` writes help and version to stdout and a usage error to stderr, which
is what `clap` would have done. Measured after the change: `--help` exits 0,
`--version` exits 0, no arguments exits 5, and a bad subcommand exits 5.
`[VERIFIED: local]`

`clap::error::ErrorKind` is available with the `error-context` feature that
STACK.md already selects.

## 7.8 The exit code enum

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Exit {
    Ok = 0, NotPe = 1, NoVbRuntime = 2, NotVb6 = 3, Damaged = 4, Internal = 5,
}

impl From<Exit> for ExitCode {
    fn from(e: Exit) -> Self { Self::from(e as u8) }
}
```

`e as u8` on a `#[repr(u8)]` fieldless enum passes the lint wall.
`clippy::cast_possible_truncation` does not fire on an enum-to-integer cast.
`[VERIFIED: local]`

`Exit::Damaged` is unreachable from the CLI in this phase only because
`--salvage` does not exist. It is reachable already, because a truncated file
produces `Refusal::Damaged` (§4.7). Define all six now, as CONTEXT.md asks.

**Return `ExitCode` from `main`, do not call `std::process::exit`.**
`std::process::exit` does not run destructors, and it would make a future
buffered-writer flush silently disappear.

---

# 8. The `VBHeader` 0x58 / 0x5C gap: closed

## 8.1 The answer

**0x58 is `oProjectExeName`. It holds the EXE name with the `.exe` extension
removed.**
**0x5C is `oProjectTitle`. It holds the `.vbp` `Title=` value.**

The SVBD and SEK reading is correct. The AI, IDC and PVB reading, which calls
0x58 the project description and 0x5C the EXE name, is **wrong** and must be
removed from `STRUCTURES.md` §2.3 rather than kept as an alias.

`[VERIFIED: local, all 44 corpus executables diffed against the matching .vbp]`

| Field | Claim | Result |
|---|---|---|
| `0x58` | equals `ExeName32=` with the extension removed | **44 of 44** |
| `0x5C` | equals `Title=` | **44 of 44** |
| `0x60` | equals `HelpFile=` | **44 of 44** |
| `0x64` | equals `Name=` | **44 of 44** |

The 0x60 and 0x64 columns were already agreed by every source. They are in the
table as the control: a procedure that got them wrong would not be trusted on
0x58 and 0x5C.

## 8.2 The concrete procedure

**Which offsets to read.** From the start of the `VBHeader` structure, four
`u32` values at `+0x58`, `+0x5C`, `+0x60` and `+0x64`.

**How to resolve them.** Each value is a byte offset **relative to the start of
the `VBHeader`**, not a VA and not an RVA. `STRUCTURES.md` §2 says all four
sources agree on that, and the measurement confirms it. So:

1. `PeFile32::parse(data)`, take `AddressOfEntryPoint` and `ImageBase`.
2. `region_at(entry_rva)`, confirm byte 0 is `0x68`, read the `u32` at `+1` as a
   VA.
3. `region_at_va(that_va)` gives the header window. Confirm bytes 0 to 3 are
   `VB5!`.
4. Read the four `u32` values from that window.
5. For each, read a NUL-terminated string from the **same window**, at the offset
   the value gives. Do not resolve it against the file or the image base.
6. Decode as Latin-1, that is `char::from(byte)`.

**Which corpus files to use: all 44.** Not a sample. The two readings differ in
which slot holds which string, and a project whose title equals its exe name
would satisfy both. 21 of the 44 are exactly that shape, so a small sample
could have proved nothing.

**How to select the matching `.vbp`.** Do not glob. Search the executable's
directory and then up to three parent directories for any file whose name ends
in `.vbp`, **case-insensitively**, and select the one whose `ExeName32` equals
the executable's file name, compared case-insensitively. This is the rule
`STRUCTURES.md` §12 already sets for the Phase 2 harness, and it is needed here
for the same reason.

Two corpus shapes force it:

- `corpus/public-domain/SK-MCI-Sample__VB6/` holds `MCI.VBP` in **upper case**,
  and the executable is in a `demo/` subdirectory. A `*.vbp` glob misses the
  file and a same-directory search misses it too.
- `corpus/public-domain/SK-TFTP-Sample__VB6/` holds `Client/` and `Server/`,
  each with its own `.vbp` and its own `demo/` subdirectory.

With this rule, 44 of 44 executables matched a project. With a same-directory
case-sensitive glob, 5 did not. `[VERIFIED: local]`

**How to read the `.vbp`.** Decode as Windows-1252 and split on CRLF. Strip
**one** leading and **one** trailing double quote from a value, and nothing
more. VB6 does not escape an embedded quote:
`corpus/vb6-code/Sepia-effect/Sepia.vbp` holds

```
Title="Sepia / "Antique" Image Filter"
```

A parser that strips every quote, or that splits on quotes, produces
`Sepia / Antique Image Filter` and reports a false mismatch. It did, in the
first run of this procedure. `[VERIFIED: local]`

**The `Title=` default.** `corpus/public-domain/SK-Gradient-Sample__VB6/Project1.vbp`
has **no `Title=` line at all**. VB6 omits the key when the title equals the
project name. The comparison must fall back to `Name=` when `Title=` is absent,
or it reports a false mismatch on that one file. `[VERIFIED: local, 1 of 44]`

## 8.3 What each result would have confirmed

This is stated so the next reader can see the test was capable of failing.

| Observation | Reading it confirms |
|---|---|
| The string at 0x58 matches `ExeName32=`, and the string at 0x5C matches `Title=` | SVBD and SEK. **This is what happened, 44 of 44.** |
| The string at 0x58 matches a project description, and the string at 0x5C matches `ExeName32=` | AI, IDC and PVB |
| Neither, or a mixture across files | Both readings wrong; the fields are something else and the gap stays open |
| A file where the two slots hold the same string | Nothing. Discard those files and count only the rest. |

The third and fourth rows were real risks. **21 of the 44 files hold the same
string in both slots**, because their title equals their exe stem. The remaining
**23 files discriminate**, and all 23 agree with SVBD and SEK.
`[VERIFIED: local]`

## 8.4 The worked example

`corpus/vb6-code/Mandelbrot/Mandelbrot.exe`, `ImageBase = 0x00400000`,
`AddressOfEntryPoint = 0x1274`, `VBHeader` at VA `0x00401760`, RVA `0x1760`,
file offset `0x1760`. `[VERIFIED: local]`

| Header offset | Raw `u32` | String at that header-relative offset | `.vbp` line |
|---|---|---|---|
| `0x58` | `0x78` | `Mandelbrot` | `ExeName32="Mandelbrot.exe"` |
| `0x5C` | `0x83` | `Mandelbrot Fractal Demo` | `Title="Mandelbrot Fractal Demo"` |
| `0x60` | `0x9B` | `` (empty) | `HelpFile=""` |
| `0x64` | `0x9C` | `Mandelbrot_Fractal_Demo` | `Name="Mandelbrot_Fractal_Demo"` |

The offsets are `0x78`, `0x78 + 11`, then `+ 24`, then `+ 1`. The strings are
packed end to end with one NUL between them.

## 8.5 Three further facts the sweep produced

`[VERIFIED: local, 44 of 44]`

**The value at `0x58` is `0x78` in every file.** The four strings always start
immediately at header-relative `0x78`.

**Bytes `0x68` to `0x77` of the `VBHeader` are sixteen zero bytes in every
file.** The header is `0x68` bytes long, so this is a sixteen-byte gap between
the last field and the string pool. `STRUCTURES.md` §2 does not mention it. Do
not read it, do not name it, and do not assume the pool always starts at
`0x78`: follow the offset in the field, as the format requires.

**The ordering `0x58 < 0x5C < 0x60 <= 0x64` holds in every file**, and the
largest header-relative offset seen is `171` (`0xAB`). This is a cheap sanity
check for the report, not a parse rule. Never derive a length from the next
offset. Read to the NUL, with a bound.

## 8.6 What to write back into `STRUCTURES.md`

Replace the §2.3 disputed rows with:

| Offset | Size | Name | Type | Meaning | Conf |
|---|---|---|---|---|---|
| 0x58 | 4 | `oProjectExeName` | u32 | Header-relative offset to the EXE name **without** its extension, NTS. Equals `.vbp` `ExeName32=` minus `.exe`. | **[C]** |
| 0x5C | 4 | `oProjectTitle` | u32 | Header-relative offset to the project title, NTS. Equals `.vbp` `Title=`, which VB6 omits when it equals `Name=`. | **[C]** |

Move gap 1 out of the §11 register and record the method, the date, the sample
size and the two exceptions that the method must handle (the embedded quotes and
the absent `Title=`).

## 8.7 What Phase 4 must now do

`Title=` comes straight from 0x5C, and it is `confidence: recovered`, not
`inferred`.

`ExeName32=` is 0x58 **plus the literal `.exe`**, because the extension is not in
the file. Record that as the `basis` for the item. The STACK.md fallback rule,
"emit `ExeName32` from 0x5C only if it ends in `.exe`", is now wrong twice over:
it reads the wrong field and no corpus file's value ends in `.exe`. Delete it.

When `Title=` equals `Name=`, VB6 itself omits the key. Emitting it anyway is
harmless and the IDE accepts it, but omitting it makes a byte-for-byte diff
against the original `.vbp` easier. That is a Phase 4 decision, not a Phase 1
one.

---

## Architecture Patterns

### System architecture

```
  a file path                     bytes                        a value
      |                             |                             |
      v                             v                             v
+-------------+   read()   +-----------------+           +-----------------+
| deform6-cli | ---------> | deform6::inspect| --------> | Report | Refusal |
|  clap parse |            |   (&[u8]) ->    |           +-----------------+
|  ExitCode   | <--------- |     Result      |                    |
+-------------+            +--------+--------+                    v
                                    |                       one sentence
             +----------------------+----------------------+  + exit code
             |                      |                      |
             v                      v                      v
     +---------------+    +------------------+    +----------------+
     | read/pe.rs    |    | vb/header.rs     |    | error.rs       |
     | PeImage       |    | entry stub       |    | Site           |
     | (only file    |    | VB5! magic       |    | DefectKind     |
     |  naming       |    | VBHeader fields  |    | Severity       |
     |  `object`)    |    +--------+---------+    | Defect / Error |
     | section table |             |              +-------+--------+
     | import dir    |             v                      ^
     | rva_to_off    |    +------------------+            |
     +-------+-------+    | vb/project.rs    |            |
             |            | ProjectInfo      |            |
             |            | lpNativeCode     |            |
             |            | project name     |            |
             |            +--------+---------+            |
             |                     |                      |
             +----------+----------+                      |
                        v                                 |
              +-------------------+                       |
              | read/region.rs    |                       |
              | Region, Off,      |--- None -> a Defect ->-+
              | Rva, Va           |
              | zero dependencies |
              +-------------------+
                        ^
                        |
              +-------------------+
              | journal.rs        |
              | Mode, Journal     |
              | one record() call |
              | applies strict or |
              | salvage, once     |
              +-------------------+
```

Data flows one way. Bytes enter as `&[u8]` at `inspect`. Every read goes through
`Region`. A `None` from `Region` becomes a `Defect` at the site that knows the
structure and field names. `Journal::record` is the single place the strict or
salvage policy applies. The CLI turns the final value into a sentence and a
number and does nothing else.

### Recommended project structure for this phase only

```
DeForm6/
├── Cargo.toml               # §1.1
├── Cargo.lock               # committed
├── rust-toolchain.toml      # §1.6
├── crates/
│   ├── deform6/
│   │   ├── Cargo.toml       # §1.2
│   │   ├── src/
│   │   │   ├── lib.rs       # #![forbid(unsafe_code)]; pub fn inspect
│   │   │   ├── read/mod.rs
│   │   │   ├── read/region.rs   # 01-02
│   │   │   ├── read/pe.rs       # 01-04
│   │   │   ├── vb/mod.rs
│   │   │   ├── vb/header.rs     # 01-05
│   │   │   ├── vb/project.rs    # 01-07
│   │   │   ├── error.rs         # 01-03
│   │   │   └── journal.rs       # 01-03
│   │   ├── tests/refusal.rs      # 01-08
│   │   ├── tests/corpus_sweep.rs # 01-08
│   │   └── fuzz/                 # directory and exclusion only; Phase 5 fills it
│   └── deform6-cli/
│       ├── Cargo.toml       # §1.3
│       ├── src/main.rs      # 01-08
│       └── tests/cli.rs     # 01-08
└── corpus/                  # unchanged
```

`model.rs`, `write/`, `report.rs` and `xtask/` are later phases. Do not create
empty files for them.

### Pattern: the fallible newtype bridge

**What:** three integer spaces, one crossing, and it is checked.
**When:** every time a VB structure yields a pointer.

```rust
let va: Va = hdr.va_le(Off::new(0x30))?;          // typed at the read
let region = pe.region_at_va(va).ok_or(defect)?;  // Va -> Rva -> Off, both checked
```

`region_at_va` is `va.to_rva(image_base)?` then `region_at`. There is no other
route, because `Va` has no `From<u32>` for an `Off` and no `Add`.

### Pattern: the bounded window, rebased

**What:** every structure gets its own `Region`, whose offset 0 is the
structure's start and whose `base` is the structure's file offset.
**When:** the moment a pointer resolves.

This is what lets a defect three levels down report an absolute file offset
without any caller threading one through.

### Anti-patterns to avoid

- **Reading past `mapped_len` because `size_of_raw_data` is larger.** Those
  bytes are `FileAlignment` padding that the loader never maps. 125 of 132
  corpus sections have that padding.
- **Falling back to "treat the RVA as a file offset" when a section lookup
  fails.** It often works and it is often wrong.
- **`String::from_utf8_lossy` on a VB header string.** A high byte becomes
  U+FFFD and the name is lost. Use `char::from(byte)` here, and the real
  Windows-1252 table in Phase 3.
- **Globbing `corpus/**/*.vbp` case-sensitively.** It misses `MCI.VBP`.
- **`Cli::parse()`.** It exits 2 on a usage error, which collides with a locked
  exit code.
- **`#[expect]` instead of `#[allow]` in a test module.** An unfulfilled
  expectation is itself a build error.
- **Asserting on the text of a refusal sentence.** Assert on the variant and on
  the exit code. CONTEXT.md gives that as a reason for the exit-code table.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| DOS header, PE signature, COFF header, optional header 32 | a 250-line parser | `object` 0.40.0 behind `PeImage` | measured against four hostile inputs and 105 M downloads a quarter; the seam keeps the escape open |
| The import directory walk | a descriptor loop with a null test | `ImportTable::descriptors()` | the fallible iterator already bounds every read |
| Argument parsing, `--help`, `--version`, `OsString` paths | a hand-written token loop | `clap` 4.6.6, features off | six marginal crates for help text that cannot drift |
| `Display` for a two-level error enum | `impl Display` by hand | `thiserror` 2.0.20 | the message lives next to the variant |
| An offset-carrying byte reader | `scroll`, `zerocopy`, `nom`, `binrw` | `Region` | none of them stops `off + len` from wrapping, which is the actual hazard |
| RVA to file offset | a call to `SectionTable::pe_file_range_at` | `PeImage::rva_to_off` | `object`'s two helpers disagree about the zero-filled tail (§5.3), and neither returns provenance |

**Key insight:** the thing worth hand-writing in this domain is not the parser.
It is the *bounds discipline*. Every crate rejected in STACK.md was rejected
because it gives a fallible read without giving a bounded window, and the fault
this project must prevent is an in-bounds read at a wrapped offset.

## Common Pitfalls

### Pitfall 1: a successful `PeFile32::parse` read as "the file is intact"

**What goes wrong:** a truncated file parses, and the next read returns bytes
from an unrelated part of the image or fails far from the real cause.
**Why:** `parse` validates headers only. Measured: a corpus file cut to one
eighth still returns `Ok`.
**How to avoid:** every read after the parse goes through `Region`, whose bounds
come from the real slice length.
**Warning sign:** any code that treats `PeImage::parse` as a validity gate.

### Pitfall 2: the `clap` exit code 2 collision

**What goes wrong:** `deform6 inspec file.exe` exits 2, and a sorting script
files it under "not a Visual Basic program".
**Why:** `clap` uses 2 for a usage error, and `Cli::parse()` exits before any
code of ours runs.
**How to avoid:** `try_parse` plus the mapping in §7.7.
**Warning sign:** the CLI test that asserts a bad subcommand does not produce 2.

### Pitfall 3: the lint wall passing `cargo test` and failing `cargo clippy`

**What goes wrong:** `cargo test --workspace` is green and the commit fails the
gate on `cargo clippy --all-targets`.
**Why:** the workspace lints apply to test targets and example targets too, and
a test helper naturally uses `unwrap`, `expect`, `panic!` and indexing.
**How to avoid:** the inner attribute block from §2.5 at the top of every file
under `tests/`. Run all three gate commands, in the order `AGENTS.md` gives.
**Warning sign:** a green `cargo test` and a red `cargo clippy` in the same
session. This happened here.

### Pitfall 4: quote handling in a `.vbp`

**What goes wrong:** the Phase 1 gap check reports a false mismatch, or a Phase 4
round trip loses part of a title.
**Why:** VB6 does not escape an embedded double quote:
`Title="Sepia / "Antique" Image Filter"`.
**How to avoid:** strip exactly one leading and one trailing quote. Never split
on quotes.
**Warning sign:** exactly one corpus project failing a string comparison.

### Pitfall 5: choosing a `.vbp` by directory rather than by `ExeName32`

**What goes wrong:** 5 of 44 executables match no project, or match the wrong
one.
**Why:** the executable often sits in a `demo/` subdirectory, one project file
is named in upper case, and one repository holds a `Client/` and a `Server/`
project side by side.
**How to avoid:** search up to three parent directories, match `.vbp`
case-insensitively, and select by `ExeName32`.
**Warning sign:** an unmatched-file count that is not zero.

### Pitfall 6: pinning the numbers from the CONTEXT.md example output

**What goes wrong:** a test asserts `57344 bytes` and `4 sections` for
`Mandelbrot.exe` and fails on the real file.
**Why:** the example in CONTEXT.md is illustrative. The real file is **28,672
bytes with 3 sections**, its `VB5!` is at file offset **`0x1760`**, and its
`wRuntimeBuild` is **`0x2636`**. `[VERIFIED: local]`
**How to avoid:** assert on the project name, the mode and the exit code, which
the `.vbp` proves. Do not assert on a size, a section count or a build number.
**Warning sign:** any literal from that code block appearing in a test.

## Code Examples

### Resolve the entry point and reach the VB header

```rust
// Source: verified against all 44 corpus executables, 2026-09-07
let pe = PeImage::parse(data)?;                             // exit 1 on Err
let entry = pe.region_at(pe.entry_rva())
    .ok_or(Refusal::Damaged("the entry point is in no section"))?;
if entry.u8(Off::new(0)) != Some(0x68) {
    return Err(Refusal::Damaged("the entry point is not a push imm32"));
}
let va = entry.va_le(Off::new(1))
    .ok_or(Refusal::Damaged("the push operand is past the end of the section"))?;
let hdr = pe.region_at_va(va)
    .ok_or(Refusal::Damaged("the pushed address is in no section"))?;
if hdr.take(Off::new(0), 4).ok_or(Refusal::Damaged("short header"))? != b"VB5!" {
    return Err(Refusal::Damaged("the header does not begin with VB5!"));
}
```

### Read the four `VBHeader` string offsets and resolve them

```rust
// Source: verified against all 44 corpus executables, 2026-09-07
let o_exe_name     = hdr.u32_le(Off::new(0x58)).ok_or(d)?;  // ExeName32 minus ".exe"
let o_title        = hdr.u32_le(Off::new(0x5C)).ok_or(d)?;  // Title
let o_help_file    = hdr.u32_le(Off::new(0x60)).ok_or(d)?;  // HelpFile
let o_project_name = hdr.u32_le(Off::new(0x64)).ok_or(d)?;  // Name

fn header_string(hdr: &Region<'_>, o: u32) -> Option<String> {
    let b = hdr.cstr(Off::new(o), 0x104)?;      // offsets are header-relative
    Some(b.iter().map(|&c| char::from(c)).collect())
}
```

### Report the compilation mode

```rust
// Source: verified against all 44 corpus executables, 2026-09-07 (non-zero in 44 of 44)
let pi = pe.region_at_va(h.lp_project_data)
    .ok_or(Refusal::Damaged("lpProjectData is in no section"))?;
let native = pi.u32_le(Off::new(0x20))
    .ok_or(Refusal::Damaged("ProjectInfo is truncated"))? != 0;
```

The `false` branch is unreachable with the current corpus. Say so in the test
rather than papering over it, as CONTEXT.md requires.

### The whole thing, running

```
$ deform6 inspect corpus/vb6-code/Mandelbrot/Mandelbrot.exe ; echo "exit=$?"
File      Mandelbrot.exe  (28672 bytes)
Format    PE32, 3 sections
Runtime   MSVBVM60.DLL  (Visual Basic 6)
Header    VB5! at 0x00001760  build 0x2636
Project   Mandelbrot_Fractal_Demo
Title     Mandelbrot Fractal Demo
ExeName   Mandelbrot.exe
Mode      native
Forms     1
exit=0
```

`[VERIFIED: local]` The locked shape says `Objects   1`. The skeleton prints
`Forms 1`, because the object table is Phase 2. Print `Forms` here and rename
the line to `Objects` in Phase 2, or print both. That is a wording choice for
the plan, not a research question.

## State of the Art

| Old approach | Current approach | When changed | Impact |
|---|---|---|---|
| `#![deny(clippy::...)]` in each crate root | `[workspace.lints.clippy]` plus `[lints] workspace = true` | Cargo 1.74 | one table, both crates, every target, no drift |
| `#[allow(..)]` with a comment | `#[allow(.., reason = "..")]` | Rust 1.81 | the reason travels with the attribute |
| `resolver = "2"` | `resolver = "3"` | edition 2024 | a virtual root must still declare it |
| `fn main()` plus `std::process::exit` | `fn main() -> ExitCode` | Rust 1.61 | destructors run |
| `object` 0.36 `PeFile32::parse` | unchanged API in 0.40.0 | - | STACK.md's sketch still compiles as written |

**Deprecated or outdated in the existing research:**

- `STRUCTURES.md` §2.3, the AI, IDC and PVB reading of 0x58 and 0x5C: refuted by
  44 of 44. Replace, do not alias.
- `STACK.md` §2's `pub fn as_usize(self) -> usize { self.0 as usize }`: it
  compiles, but `usize::try_from(..).unwrap_or(..)` is the shape to write, so no
  reader has to reason about which cast direction the wall catches.
- `STACK.md` §5's claim that `exclude` is needed to stop the gate breaking: not
  reproducible on cargo 1.97.1 (§1.5). Keep the line; drop the claim.
- `STACK.md` §4's `ExeName32` fallback rule ("emit from 0x5C only if it ends in
  `.exe`"): wrong field and no corpus value ends in `.exe`. Delete it (§8.7).

## Runtime State Inventory

Not applicable. This phase is greenfield: the repository holds no Rust code at
all. There is no stored data, no live service configuration, no OS-registered
state, no secret or environment variable, and no build artifact that carries a
name this phase changes. `git ls-files` lists six top-level entries and none of
them is code: `.gitattributes`, `.gitignore`, `.planning/`, `AGENTS.md`,
`LICENSE` and `corpus/`. An untracked `Notes/` directory sits beside them.
`[VERIFIED: local, git ls-files]`

## Environment Availability

| Dependency | Required by | Available | Version | Fallback |
|---|---|---|---|---|
| `rustc` | everything | yes | 1.97.1 (8bab26f4f 2026-07-14) | - |
| `cargo` | everything | yes | 1.97.1 (c980f4866 2026-06-30) | - |
| `rustfmt` | gate command 1 | yes | installed component | - |
| `clippy` | gate command 2 | yes | installed component | - |
| rustup toolchain `1.97.1` | `rust-toolchain.toml` | yes | `1.97.1-aarch64-apple-darwin` | - |
| crates.io network access | first build only | yes | resolved 17 crates today | vendor with `cargo vendor` |
| `corpus/` | the sweep and the refusal fixtures | yes | 44 executables, 45 project files | none needed |
| nightly toolchain, `cargo-fuzz` | Phase 5 only | not installed | - | not needed in Phase 1 |
| VB6 IDE on Windows | Phase 4 recompilation check | no | - | the structural check, as ROADMAP already states |

**Missing dependencies with no fallback:** none for this phase.
**Missing dependencies with a fallback:** none for this phase.

## Validation Architecture

`workflow.nyquist_validation` is `true` in `.planning/config.json`.
`[VERIFIED: local]`

### Test framework

| Property | Value |
|---|---|
| Framework | built-in `libtest`, `#[test]`, no crate |
| Config file | none; targets are discovered from `tests/` |
| Quick run command | `cargo test -p deform6` |
| Full suite command | `cargo test --workspace` |

Measured: the whole phase-1 suite runs in **0.34 s**, dominated by the four CLI
tests that spawn a process. `[VERIFIED: local]`

### Phase requirements mapped to tests

| Req | Behaviour | Type | Automated command | File exists? |
|---|---|---|---|---|
| DET-01 | entry RVA resolves to a file offset on every corpus file | integration | `cargo test -p deform6 --test corpus_sweep` | Wave 01-08 |
| DET-02 | the stub reaches `VB5!` on every corpus file | integration | `cargo test -p deform6 --test corpus_sweep` | Wave 01-08 |
| DET-03 | `MSVBVM50.DLL` gives `IsVb5`, `MSVBVM60.DLL` gives a report | integration | `cargo test -p deform6 --test refusal` | Wave 01-08 |
| DET-04 | each refusal gives a distinct variant and one sentence | integration | `cargo test -p deform6 --test refusal` | Wave 01-08 |
| DET-05 | `lpNativeCode` non-zero reports `native` on every corpus file | integration | `cargo test -p deform6 --test corpus_sweep` | Wave 01-08 |
| DET-06 | `inspect` exits 0 and prints the project name | integration | `cargo test -p deform6-cli --test cli` | Wave 01-08 |
| Risk: lint wall | `d[0]`, `a + b` and `.unwrap()` each stop the build | build | `cargo clippy --all-targets -- -D warnings` | Wave 01-01 |
| Risk: no write | `inspect` changes no file on disk | integration | see below | Wave 01-08 |

**The "changes no file" check.** ROADMAP success criterion 1 says a directory
listing before and after the run is identical. Write it as a test that snapshots
`(path, len, mtime)` for every entry under the executable's directory, runs
`inspect`, and compares. Do not shell out to `ls`. The library only ever takes a
`&[u8]`, and the CLI only ever calls `std::fs::read`, so this test is a guard
against a future regression rather than a discovery.

### Sampling rate

- **Per task commit:** `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`. That is the `AGENTS.md` gate, unchanged, and it takes about one second on this machine after the first build.
- **Per wave merge:** the same. There is nothing slower to defer.
- **Phase gate:** the same, on a clean clone, with `cargo build` never
  substituted for `cargo test`.

### Wave 0 gaps

- [ ] `crates/deform6/tests/refusal.rs` covers DET-03 and DET-04
- [ ] `crates/deform6/tests/corpus_sweep.rs` covers DET-01, DET-02, DET-05
- [ ] `crates/deform6-cli/tests/cli.rs` covers DET-06 and the exit-code table
- [ ] No framework install is needed. `libtest` ships with the toolchain.
- [ ] No `conftest`-style shared fixture file is needed. `corpus_root()` is four
      lines and it is duplicated in each of the three files on purpose: a shared
      `support/` module would be a dependency between three tests that must be
      able to fail independently.

## Security Domain

`workflow.security_enforcement` is `true` and `security_asvs_level` is `1`.
`[VERIFIED: local, .planning/config.json]`

DeForm6 is an offline command-line file parser. It has no network, no
authentication, no session, no user, no database and no cryptography. Most ASVS
categories do not apply, and saying so is more useful than inventing a control.

### Applicable ASVS categories

| ASVS category | Applies | Standard control |
|---|---|---|
| V2 Authentication | no | there is no principal |
| V3 Session management | no | the process is one run |
| V4 Access control | no | the tool has the user's own rights |
| V5 Input validation | **yes** | this is the whole phase: `Region`, the newtypes, the lint wall, `checked_add`, and a bound on every count before an allocation |
| V6 Cryptography | no | none in this phase; `hmac-sha256` in Phase 4 is a content digest for the report, not a security control |
| V7 Error handling and logging | **yes** | one sentence to stderr, an exit code, a `Defect` with a byte offset; no path or byte dump beyond what the user supplied |
| V12 Files and resources | **yes** | `inspect` opens one file read only and writes nothing; `extract` in Phase 4 is where path handling starts to matter |

### Known threat patterns for this stack

| Pattern | STRIDE | Standard mitigation |
|---|---|---|
| Integer overflow on `off + len`, making an out-of-range read land in bounds | Tampering | `clippy::arithmetic_side_effects` deny plus `checked_add` on the newtype; `overflow-checks = true` in release as a backstop |
| Slice index panic on a truncated or hostile file | Denial of service | `clippy::indexing_slicing` deny; `Region` has no infallible accessor; `panic = "abort"` in release makes a panic visible rather than catchable |
| Allocation sized from a length field in the file | Denial of service | check every count against the real file length before it sizes anything; `ImplausibleCount` refuses; `-rss_limit_mb` in Phase 5 fuzzing is the test for it |
| Unbounded scan for a NUL byte | Denial of service | `Region::cstr` takes a mandatory `max` |
| A parser differential between DeForm6 and the Windows loader on overlapping sections | Spoofing | §5.3 case D: first-match, and report the overlap as a `Defect` rather than resolving it silently |
| Memory unsafety inside `object` | Elevation of privilege | `object` uses `unsafe` for performance and disclaims hostile input; the mitigation is the `PeImage` seam plus a Phase 5 fuzz target that enters through it, so a crash inside `object` is found by our fuzzer and reported upstream |
| Path handling on the input argument | Tampering | `clap` gives an `OsString`; `PathBuf` never assumes UTF-8; nothing is written in this phase |

`#![forbid(unsafe_code)]` in the library means every unsafe line in the process
belongs to `object` or to the standard library. That is the whole point of the
seam, and it is why the fuzz target must call the public API rather than an
internal one.

## Assumptions Log

| # | Claim | Section | Risk if wrong |
|---|---|---|---|
| A1 | The delay-load descriptor `Attributes` bit 1 is `dlattrRva` and distinguishes RVA-based from VA-based descriptors. The Microsoft PE specification does not document it. | §6.4 | Low. DeForm6 never lets the delay-load table decide the runtime, so a wrong reading changes an informational line only. |
| A2 | Mapping VB4-32 to exit code 3 and widening code 3 to "a Visual Basic runtime that DeForm6 does not read" is what the user wants. CONTEXT.md defines no code for VB4. | §6.6 | Medium. It changes a locked table. Confirm with the user before a plan pins it. |
| A3 | `wRuntimeBuild` 9782 is VB6 SP6 and 8176 is an earlier service pack. The mapping from build number to service pack is training knowledge. | §6.8 | Very low. The field is printed as a number and nothing branches on it. |
| A4 | Printing `Forms N` instead of `Objects N` in this phase is acceptable, because the object table arrives in Phase 2. | Code examples | Very low, but it is a visible difference from the locked output shape. Confirm the wording. |

## Open Questions

1. **The exit code for a VB4-32 executable.**
   - What we know: `VB40032.DLL` is a named refusal in ROADMAP plan 01-06, and it
     is reachable, because VB4-32 produces a PE.
   - What is unclear: CONTEXT.md's table defines codes 0 to 5 and gives none to
     VB4.
   - Recommendation: exit 3, with the meaning of code 3 widened to "a Visual
     Basic runtime that DeForm6 does not read", and a sentence that names the
     version. Ask the user before the plan locks it. A sixth code is the other
     option and it moves no existing number.

2. **`Objects` against `Forms` in the `inspect` output.**
   - What we know: CONTEXT.md's locked shape ends with `Objects   1`. The object
     table is Phase 2.
   - What is unclear: whether to print `Forms` from `wFormCount` now and rename
     later, print both, or omit the line until Phase 2.
   - Recommendation: print `Forms`, from `wFormCount`, which this phase can
     prove, and add `Objects` in Phase 2. Do not print a line named `Objects`
     that is filled from a form count.

3. **Whether `Refusal::IsDotNet` should exist in this phase.**
   - What we know: `has_clr_header()` is one line and the data directory is
     already parsed.
   - What is unclear: whether a distinct sentence is worth touching the exit
     code table.
   - Recommendation: add the method, use it in the message text under exit code
     2, and do not add a variant or a code. §7.4.

4. **Whether the corpus should gain a real VB5 sample now.**
   - What we know: the patched fixture proves the discrimination and nothing
     more. `corpus/manifest.toml` is Phase 5's mechanism for a fetched sample.
   - What is unclear: whether a redistributable VB5 Standard EXE exists that
     `AGENTS.md` would let into `corpus/`.
   - Recommendation: leave it. The patched fixture is honest about its own
     limits and it runs offline.

## Sources

### Primary (HIGH confidence)

- **This machine, 2026-09-07.** `/tmp/df6ws` holds a workspace with the three
  manifests, the lint wall, `region.rs`, `pe.rs`, `vb.rs`, the CLI and thirteen
  tests. `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`
  and `cargo test --workspace` all pass. The sweep reads 44 of 44 corpus
  executables.
- **This machine, 2026-09-07.** `/tmp/df6probe` holds four probe binaries:
  section and import dump, `VBHeader` string extraction, hostile-input
  behaviour, and NE and PE32+ rejection.
- **`object` 0.40.0 source**,
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/object-0.40.0/`.
  `src/read/pe/file.rs`, `src/read/pe/section.rs`, `src/read/pe/import.rs`,
  `src/read/mod.rs:117-127`, `src/pe.rs:650-686`.
- **`corpus/`**, 44 executables and 45 project files, already in the repository
  with `NOTICES`.

### Secondary (MEDIUM confidence)

- Microsoft, *PE Format*.
  https://learn.microsoft.com/en-us/windows/win32/debug/pe-format
- The Cargo Book, *Resolver versions*.
  https://doc.rust-lang.org/cargo/reference/resolver.html#resolver-versions
- `.planning/research/STACK.md`, `.planning/research/STRUCTURES.md`,
  `.planning/research/CORPUS.md`, all dated 2026-09-07.
- crates.io metadata through the package-legitimacy seam, for the five package
  verdicts.

### Tertiary (LOW confidence)

- The `delayimp.h` `dlattrRva` constant. Training knowledge, not confirmed in
  this session, and not needed by any code path. See assumption A1.
- The mapping from `wRuntimeBuild` to a VB6 service pack. Training knowledge.
  See assumption A3.

## Metadata

**Confidence breakdown:**

- Manifests and lint wall: HIGH. Written, compiled, and the gate is green.
- `Region` API: HIGH. Written and compiled under the full wall.
- `object` call set: HIGH. Read from the crate source and then run.
- Address map rules: HIGH for cases A, B and C, which the corpus exercises.
  MEDIUM for case D, overlapping sections, which no corpus file shows. The rule
  is stated and it is untested by construction.
- Runtime discrimination: HIGH for `MSVBVM60.DLL` and `MSVBVM50.DLL`. MEDIUM for
  `VB40032.DLL`, which no sample exercises. The `VB40016.DLL` path is proved
  unreachable.
- Test shape: HIGH. Thirteen tests written and passing.
- The 0x58 and 0x5C gap: HIGH. 44 of 44, with 23 of those discriminating between
  the two candidate readings.

**Research date:** 2026-09-07
**Valid until:** 2026-10-07 for the crate versions. The corpus measurements do
not expire, because the corpus is committed.
