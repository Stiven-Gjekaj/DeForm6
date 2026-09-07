# Technology stack

Decided 2026-09-07. Toolchain `rustc 1.97.1`, `cargo 1.97.1`, edition 2024,
host `aarch64-apple-darwin`.

Every version number below comes from crates.io on 2026-09-07. Every
dependency count below comes from a measurement on this machine, not from a
document.

## How this document counts a dependency

Each candidate went into an empty crate on its own. The count is the number of
unique `name version` pairs in the build graph, with the root crate removed.

```sh
cargo new --lib /tmp/dm && cd /tmp/dm
cargo add <candidate>
cargo tree -e normal --prefix none --no-dedupe \
  | sed 's/ (\*)$//;s/ (proc-macro)$//' \
  | grep -v '^$' | grep -v '^dm ' \
  | awk '{print $1" "$2}' | sort -u | wc -l
```

A proc-macro crate and its private tree build the code but do not enter the
shipped binary. The tables name that difference where it changes a decision.

## The decisions in one table

| Area | Choice | Version | Crates it adds | Rejected |
|---|---|---|---|---|
| PE envelope | `object` | 0.40.0 | 2 | `goblin`, `pelite`, hand-rolled |
| Structure parsing | Hand-written `Region` plus offset newtypes | n/a | 0 | `zerocopy`, `bytemuck`, `nom`, `binrw`, `scroll` |
| Errors | Hand-written two-level enum, `thiserror` for `Display` | 2.0.20 | 6, of which 5 are build time | `anyhow`, plain `impl Display` |
| Report | `serde` plus `serde_json` | 1.0.229 / 1.0.151 | 7 | `miniserde`, hand-written writer |
| Input digest | `hmac-sha256` | 1.1.14 | 1 | `sha2` (9 crates) |
| Fuzzing | `cargo-fuzz` plus `libfuzzer-sys` | 0.13.2 / 0.4.13 | dev only | `afl.rs`, `proptest`, `arbitrary` |
| CLI | `clap`, default features off | 4.6.6 | 6 | `clap` default (17), `argh`, `lexopt`, `pico-args` |
| Differential harness | Plain `#[test]` plus a pinned ratio file | n/a | 0 | `insta`, `expect-test` |

## The measured total

The proposed workspace exists at `/tmp/finalws` and passes `cargo check
--workspace`.

- **`deform6` library: 15 crates.** 9 of them link into the binary. 6 of them
  are the proc-macro chain and build only.
- **`deform6` plus `deform6-cli`: 21 crates.** 13 of them link into the
  binary. 8 of them build only.

The 13 that link in:
`anstyle`, `clap`, `clap_builder`, `clap_lex`, `hmac-sha256`, `itoa`,
`memchr`, `object`, `serde`, `serde_core`, `serde_json`, `thiserror`, `zmij`.

The 8 that build only:
`clap_derive`, `serde_derive`, `thiserror-impl`, `proc-macro2`, `quote`,
`syn`, `unicode-ident`, `heck`.

`syn`, `quote` and `proc-macro2` are shared by all three derive macros. The
second derive macro and the third one are close to free.

---

## 1. PE parsing

### The candidates, measured

| Candidate | Version | Crates | `no_std` | Malformed input | Last release | Recent downloads |
|---|---|---|---|---|---|---|
| `object` | 0.40.0 | **2** | `#![no_std]` at the crate root | Documented: error, not panic | 2026-08-01 | 105.8 M |
| `goblin` | 0.10.7 | 9 | `cfg_attr(not(std), no_std)` | Long history of panic fixes | 2026-05-28 | 16.0 M |
| `pelite` | 0.10.0 | 4 to 6 | Through `no-std-compat` | Zero allocation by design | **2022-11-04** | 0.13 M |
| Hand-rolled | n/a | 0 | Yes | Whatever we write | n/a | n/a |

The `object` count is with `default-features = false` and features
`read_core, pe, std`. That gives `object` plus `memchr`. With default features
it is 10, because `flate2` and `ruzstd` arrive for compressed debug sections
that a VB6 executable does not have.

### The choice: `object` 0.40.0

```toml
object = { version = "0.40.0", default-features = false, features = ["read_core", "pe", "std"] }
```

**Reason 1. It states the contract we need.** The `object` README says:
"The crate aims to be reliable: it provides memory safety (with some use of
`unsafe` for performance), and malformed input is expected to result in an
error rather than a panic or incorrect parsing."

**Reason 2. It holds under a probe.** A test program called `PeFile32::parse`
on four hostile inputs. All four returned an error. None panicked.

```
empty:             clean error, no panic: Invalid DOS header size or alignment
two bytes:         clean error, no panic: Invalid DOS header size or alignment
absurd e_lfanew:   clean error, no panic: Invalid PE headers offset or size
junk:              clean error, no panic: Invalid DOS magic
```

The "absurd e_lfanew" case is a 64 byte file with `e_lfanew` set to
`0xFFFFFFFE`. That is the classic allocation-from-a-length-field trap. It
returns an error.

**Reason 3. It is the smallest tree.** 2 crates, against 9 for `goblin`.

**Reason 4. It exposes exactly what we need.** This compiles today:

```rust
use object::read::pe::{PeFile32, ImageNtHeaders, ImageOptionalHeader};

let pe = PeFile32::parse(data)?;                       // Result, not panic
let nt = pe.nt_headers();
let entry: u32 = nt.optional_header().address_of_entry_point();
let base: u64 = nt.optional_header().image_base();
let at_entry: Option<&[u8]> = pe.section_table().pe_data_at(pe.data(), entry);
for s in pe.section_table().iter() {
    let d: Result<&[u8], object::Error> = s.pe_data(pe.data());
}
let imports = pe.import_table();                       // Result<Option<_>>
```

`pe_data_at` is the critical one. It maps a relative virtual address to bytes
and returns `Option`. The corpus check in `CORPUS.md` proves that every one of
the 44 executables needs that map, because the entry point address is not
constant. `pe_data_at` is the bounds-checked primitive for it.

**Reason 5. Maintenance.** Released 2026-08-01, repository pushed 2026-08-30,
105.8 million downloads in the last 90 days, 598 million in total. It is the
`gimli-rs` crate that the Rust backtrace and debug tooling stands on, so it
runs against real inputs every day.

`object` is `#![no_std]` at the crate root, with `std` as an added feature.
`read_core` needs `alloc`. DeForm6 uses `std`, so this is only a sign of
discipline, not a requirement we exercise.

### The disclaimer, and the answer to it

The same README also says:

> This crate is intended to be used with trusted inputs. For example, it is
> suitable for use as part of a compiler toolchain, but not for malware
> analysis or a service exposed to arbitrary inputs.

That is the exact opposite of the DeForm6 threat model. Do not ignore it.

The answer is a seam. Put `object` behind one internal type in
`crates/deform6/src/pe.rs`:

```rust
/// The only place in the crate that names `object`.
pub(crate) struct PeImage<'a> { /* ... */ }

impl<'a> PeImage<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, Error>;
    pub fn is_i386_pe32(&self) -> bool;
    pub fn image_base(&self) -> u32;
    pub fn entry_rva(&self) -> Rva;
    pub fn imports_dll(&self, name: &str) -> Result<bool, Error>;
    pub fn region_at(&self, rva: Rva) -> Option<Region<'a>>;
    pub fn sections(&self) -> impl Iterator<Item = SectionInfo>;
}
```

Seven methods. Nothing else in the crate says `object::`. That gives three
things.

1. The fuzz target covers `object` as well, because it enters through
   `PeImage::parse`. A panic that libFuzzer finds inside `object` is a bug we
   report upstream and can pin around.
2. If upstream declines a fix, we replace the body of `PeImage` by hand. The
   PE envelope we need is about 250 lines: DOS header, PE signature, COFF
   header, optional header 32, the data directory for imports, and the section
   table. Nothing else in the crate changes.
3. `Cargo.lock` is committed, so the version that the fuzzer cleared is the
   version that ships.

### Rejected: `goblin` 0.10.7

`goblin` is the popular choice and it is the wrong one here.

9 crates against 2. It reaches `scroll`, then `scroll_derive`, then `syn`,
`quote`, `proc-macro2` and `unicode-ident`.

The changelog is the real argument. These are all PE or shared-path entries:

- `pe: Fix potential out-of-bounds read in unwind/POGO info parser` (PR 498)
- `pe: fix load config parser out of bounds` (PR 483)
- `pe: fix base relocation parser panic` (PR 465)
- `pe.tls: tlsdata.parse_with_opts - integer overflow + out of bound` (PR 448)
- `pe.debug: POGOInfo.parse_with_opts - integer overflow + out of bound` (PR 449)
- `coff: fix subtract with overflow in COFF header parser` (PR 453)
- `pe: fix out of bounds access while parsing AttributeCertificate` (PR 368)
- `pe: fix oob access` (PR 330)
- `pe: fix panic when parsing unwind info` (PR 218)

This is a healthy project that fixes its bugs. It is also a record that the PE
path panics on malformed input often enough to fill a changelog. A project
whose first constraint is "no panic on any input, ever" does not start there.

`goblin` also parses much more than we ask for. It reads TLS, load config,
certificates and unwind info by default. Every one of those parsers is attack
surface for a feature DeForm6 does not use, and the list above shows that most
of the panics came from exactly those parsers.

### Rejected: `pelite` 0.10.0

`pelite` is well designed. It is zero allocation, it separates "file image"
from "loaded image", and it holds up on files that other parsers refuse.

It is stale where it counts. The last release on crates.io is **2022-11-04**,
four years ago. The repository saw a push on 2025-08-22, but nothing shipped.
0.13 million recent downloads against 105.8 million for `object`.

A parser for hostile input is a crate that must ship fixes. One that has not
shipped in four years is not that crate. Revisit if it releases again.

### Rejected for now: hand-rolled

This is the honest second choice and it stays documented behind the seam.

For it: zero crates, zero disclaimer, and total control of the panic property.
The surface is small. DeForm6 needs the DOS header, the PE signature, the COFF
header, the 32 bit optional header, one data directory, and the section table.
That is about 250 lines with the `Region` reader from section 2.

Against it: it is 250 lines of new bug surface at the very start of the
project, on the one layer that is already solved well, and it delays the parts
that carry the value. `object` costs 2 crates and buys a parser that 105
million downloads a quarter exercise.

Take the seam now. Take the hand-rolled body only if fuzzing gives a reason.

---

## 2. Safe binary parsing

### The choice: a hand-written `Region` plus offset newtypes plus a lint wall

Zero dependencies. About 150 lines. The design below compiles clean under the
full lint wall on this machine.

```rust
#![forbid(unsafe_code)]
#![deny(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unreachable,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::integer_division
)]

/// A byte offset inside the file image. Never mix it with an address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Off(u32);
/// An address relative to the image base.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Rva(u32);
/// An absolute virtual address, as the VB structures store it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Va(u32);

impl Va {
    pub fn to_rva(self, image_base: u32) -> Option<Rva> {
        self.0.checked_sub(image_base).map(Rva)
    }
}

impl Off {
    pub fn advance(self, n: u32) -> Option<Off> { self.0.checked_add(n).map(Off) }
    pub fn as_usize(self) -> usize { self.0 as usize }
}

/// A bounded window on the file. The only way to read bytes.
pub struct Region<'a> { bytes: &'a [u8], base: Off }

impl<'a> Region<'a> {
    pub fn take(&self, at: Off, len: u32) -> Option<&'a [u8]> {
        let start = at.as_usize();
        let end = at.advance(len)?.as_usize();
        self.bytes.get(start..end)
    }
    pub fn u32_le(&self, at: Off) -> Option<u32> {
        let b: [u8; 4] = self.take(at, 4)?.try_into().ok()?;
        Some(u32::from_le_bytes(b))
    }
}
```

### How this makes "no panic ever" a build failure

The promise becomes a property through four mechanisms. Three of them are
checked by the compiler. This was verified, not assumed: a crate with the lint
block above and five deliberately bad functions produced five compile errors,
and the good forms next to them produced none.

```
error: indexing may panic                                        d[0]
error: slicing may panic                                         &d[4..8]
error: arithmetic operation ... unexpected side-effects          a + b
error: used `unwrap()` on an `Option` value                      .unwrap()
error: casting `u64` to `u32` may truncate the value             a as u32
error: casting `i32` to `u32` may lose the sign of the value     a as u32
```

**1. `#![forbid(unsafe_code)]`.** Nothing in the crate can reach past a bound
without the compiler stopping it. This is why `zerocopy` and `bytemuck` are
not needed: their whole value is a safe wrapper over `unsafe` transmutes, and
we do not want any transmutes.

**2. The lint wall.** `clippy::indexing_slicing` and
`clippy::arithmetic_side_effects` are `restriction` lints. They fire only on
our crate, never on a dependency, which is exactly right. `cargo clippy
--all-targets -- -D warnings` is already in the gate in `AGENTS.md`, so the
wall runs on every commit.

**3. `Region` has no infallible accessor.** It does not implement `Index`. It
does not expose its `&[u8]`. Every read returns `Option`. There is no shape of
call that reads out of bounds, because the type offers none.

**4. `Off`, `Rva` and `Va` do not implement `Add`.** This is the highest value
newtype in the project. The VB6 format stores absolute virtual addresses in
its structures, while the PE stores relative ones, and the file itself is
addressed by offset. Three integer spaces, all `u32`, all easy to confuse.
With no `Add` and no `From`, the only route between them is
`Va::to_rva(image_base)`, which is `checked_sub`. A subtraction that underflows
returns `None` instead of a huge wrapped address that lands inside the file.

Add `panic = "abort"` to `[profile.release]`. A panic then is a hard crash and
never a caught unwind. That keeps the property testable rather than hideable.

### The arithmetic overflow trap, said plainly

`AGENTS.md` already names it. Here is the mechanism, because it decides the
design.

A VB6 structure holds a `u32` offset and a `u32` length. The natural code is
`data.get(off as usize..(off + len) as usize)`. In a debug build,
`off + len` panics on overflow. In a release build, overflow checks are off by
default, so `off + len` **wraps silently**. A wrapped sum is small. A small
sum is in bounds. So the release build reads real bytes from the wrong place
and reports them as recovered data with confidence.

That is worse than a panic. A panic is visible. This is a correctness fault
that produces a plausible looking wrong answer.

The fix is not `overflow-checks = true` in release, because that turns the
fault into a panic, and a panic is forbidden. The fix is to never write `+` on
a file-derived value at all. `clippy::arithmetic_side_effects` denies the bare
operator, so the compiler makes you write `checked_add`, and `checked_add`
returns `None`, and `None` becomes a `Defect` with a byte offset in it.

Keep `overflow-checks = true` in the release profile anyway, as a backstop for
our own loop counters. The file-derived paths never reach it.

### Rejected: `zerocopy` 0.8.56 and `bytemuck` 1.25.2

6 crates each, both with a derive macro and `syn`.

Both solve a problem DeForm6 does not have. They make a `&[u8]` into a `&T`
without a copy, safely. DeForm6's structures are small, fixed and few. A
`u32_le(at)` call per field is clearer, costs nothing measurable on a 90 KB
file, and needs no `#[repr(C)]` layout promises.

Neither one helps with the real hazard. `zerocopy` checks that the slice is
long enough for `T`. It does nothing about `off + len` overflowing before you
ask for the slice. The trap survives the crate.

`zerocopy` is also a hard fit for this format. VB6 structures hold absolute
virtual addresses that must be translated through the section table before the
next read. That is pointer chasing, not a flat record array. A zero-copy cast
gives you a struct full of raw `u32` values that you must then validate one at
a time, which is the loop you were trying to avoid.

### Rejected: `nom` 8.0.0

2 crates. Cheap and excellent. Wrong shape.

`nom` is a streaming combinator library. It reads forward through an input.
The VB6 image is random access: read the entry point, follow a pointer to the
VB header, follow another to the project info, follow another to the object
table, follow an array of pointers to each object. `nom`'s core abstraction
gives nothing to that, and you spend the effort fighting it into place.

`nom`'s default error type also names a position by remaining-input length,
not by absolute file offset. The error model in section 3 needs the offset.

### Rejected: `binrw` 0.15.2

10 crates, and it drags `owo-colors` and `either` in through its derive.

`binrw` is a declarative attribute DSL for binary layouts. It is very good
when a format is a clean tree of records. The VB6 image is not. It has
version-dependent fields, pointer indirection through the section table,
optional structures behind a null pointer, and flags that change the meaning of
later fields. Every one of those becomes a `#[br(if(...))]` or a custom parser
attribute. At that density the DSL is harder to read than the code it hides,
and a bug in it is harder to see.

### Rejected: `scroll` 0.13.0

1 crate with no derive. This is the closest call in this document.

`scroll`'s `Pread` returns `Result` on every read and its error carries an
offset. That is genuinely most of what we want, for one crate.

Two reasons against it. First, it gives a fallible read but not a bounded
window, so a caller can still `pread` at an offset computed by an overflowing
`+`. The trap survives. Second, and more important, `Region` is where the
`Defect` gets its byte offset, its expected-field name and its severity. That
is the error model of section 3, and it wants to be our type. Wrapping
`scroll` to add all that is more code than the 150 lines it replaces.

`scroll` stays the right answer for a project that wants a fallible reader and
nothing else.

### Rejected: plain `slice::get()` with no newtypes

This is what most people write, and it is the pattern under `Region` anyway.

On its own it fails the third integer space test. `get(a..b)` cannot tell you
that `a` is a file offset and `b` came from a virtual address. That confusion
is the most likely real bug in this project, and only a newtype catches it.

---

## 3. Error model

### The shape

Two levels, one choke point.

```rust
/// Where a problem is, and what the code wanted to find there.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Site {
    pub offset: u32,          // absolute file offset
    pub rva: Option<u32>,     // when the offset came from an address
    pub structure: &'static str,  // "VbHeader", "ObjectTable", "FormData"
    pub field: &'static str,      // "lpProjectData", "cObjects"
}

#[derive(Clone, Debug, serde::Serialize, thiserror::Error)]
pub enum DefectKind {
    #[error("expected {expected}, found {found:#x}")]
    BadMagic { expected: &'static str, found: u32 },
    #[error("offset {offset:#x} plus length {len:#x} overflows a u32")]
    OffsetOverflow { offset: u32, len: u32 },
    #[error("offset {offset:#x} is past the end of the {} byte file", .file_len)]
    PastEndOfFile { offset: u32, file_len: u64 },
    #[error("count {count} exceeds the {max} that the file can hold")]
    ImplausibleCount { count: u32, max: u32 },
    #[error("address {va:#x} is in no section")]
    UnmappedAddress { va: u32 },
    // ...
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
pub enum Severity {
    /// The file is not what it claims. Nothing downstream is meaningful.
    Fatal,
    /// One item is unreadable. The rest of the graph still stands.
    Recoverable,
}

#[derive(Clone, Debug, serde::Serialize, thiserror::Error)]
#[error("{site:?}: {kind}")]
pub struct Defect {
    pub site: Site,
    pub kind: DefectKind,
}

impl DefectKind {
    /// Severity is a property of the defect, decided in one place.
    pub fn severity(&self) -> Severity { /* match */ }
}
```

### The choke point

The strict-against-salvage split is not a flag that every parse site reads. It
is one method.

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode { Strict, Salvage }

pub struct Journal {
    mode: Mode,
    defects: Vec<Defect>,
}

impl Journal {
    /// Record a defect. Return the fallback, or refuse.
    ///
    /// `Fatal` always refuses, in both modes.
    /// `Recoverable` refuses in `Strict` and returns `fallback` in `Salvage`.
    pub fn record<T>(&mut self, defect: Defect, fallback: T) -> Result<T, Error> {
        let severity = defect.kind.severity();
        self.defects.push(defect.clone());
        match (severity, self.mode) {
            (Severity::Fatal, _) => Err(Error::Refused(defect)),
            (Severity::Recoverable, Mode::Strict) => Err(Error::Refused(defect)),
            (Severity::Recoverable, Mode::Salvage) => Ok(fallback),
        }
    }
}

/// The only error the library returns to a caller.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("refused: {0}")]
    Refused(Defect),
    #[error("not a VB6 executable: {0}")]
    NotVb6(&'static str),
    #[error("this is a VB5 executable, which DeForm6 does not read")]
    IsVb5,
    #[error("reading {path}: {source}")]
    Io { path: std::path::PathBuf, #[source] source: std::io::Error },
}
```

Four properties fall out of this.

1. **A defect is always recorded, in both modes.** A strict run that refuses
   still knows what it saw and can say so. The failure message and the report
   come from the same value.
2. **The mode is applied once.** No parse site branches on `Strict`. A parse
   site calls `journal.record(defect, fallback)?` and gets on with it.
3. **The classification lives on `DefectKind::severity`.** One `match`. It is
   the place to argue about whether a thing is fatal.
4. **`Defect` derives `Serialize`.** The same value that is the error is the
   evidence in the JSON report. No second type, no conversion, no drift.

### Rejected: `anyhow` 1.0.104 in the library

1 crate with no transitive deps, so the cost is not the problem.

`anyhow::Error` erases the type. The library's whole job is to distinguish
"this file is not a VB6 executable" from "this one form is unreadable" from
"the disk is full", and to serialise the second one into a report. An erased
error can do none of that.

### Rejected: `anyhow` in the CLI as well

This is the usual advice and it does not earn its place here. `deform6-cli` has
exactly two error sources: `deform6::Error` and `std::io::Error`. A twelve line
enum covers both. `anyhow` is worth taking when there are twenty sources and no
caller ever matches on them.

Revisit if the CLI grows a config file, a network fetch and a manifest parser.
Three more sources is where the balance changes.

### Rejected: `impl Display` by hand, no `thiserror`

`thiserror` 2.0.20 costs 6 crates, of which 5 are the shared proc-macro chain
that `serde` derive brings anyway. The real marginal cost is `thiserror` plus
`thiserror-impl`. Two crates.

Two crates to keep the message next to the variant is worth it. A hand-written
`Display` puts the text in a second `match` in a second place, and the two
drift. The error text is a product surface here: `AGENTS.md` requires that "an
error names the byte offset and what the code expected to find there". Keep it
attached to the variant.

---

## 4. The confidence report

### Serialisation: `serde` 1.0.229 plus `serde_json` 1.0.151

7 crates together: `serde`, `serde_core`, `serde_derive`, `serde_json`,
`itoa`, `memchr`, `zmij`. `memchr` is already there from `object`, so the true
marginal cost is 6. `zmij` is dtolnay's float formatter, new in this line, and
it replaces `ryu`.

There is no serious alternative. `miniserde` drops the derive weight and also
drops the control over field naming and enum representation that a documented
schema needs. A hand-written JSON writer means hand-written escaping, and
hand-written escaping is a bug.

### Input digest: `hmac-sha256` 1.1.14, not `sha2`

The report names the bytes it read by SHA-256. The corpus manifest for the
fetched robustness set needs the same digest.

`sha2` 0.11.0 costs **9 crates**: `cfg-if`, `cpufeatures`, `libc`, `digest`,
`block-buffer`, `hybrid-array`, `typenum`, `crypto-common`, `const-oid`.
`sha2` 0.10 costs 9 as well. That is a large tree for one hash.

`hmac-sha256` 1.1.14 costs **1 crate** with no transitive deps. It is
`jedisct1`'s, pure Rust, `no_std`, 5.9 million downloads in the last 90 days,
updated 2026-02-13. A test on this machine confirmed it against the NIST
vector for `"abc"`:

```
ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
```

Take the 1 crate.

### Prior art for the schema

**SARIF 2.1.0 (OASIS)** is the closest fit and the most useful to copy from.
It is a JSON format for static analysis results and it has an explicit
`address` object for binary analysis tools, with `absoluteAddress`,
`relativeAddress`, `offsetFromParent`, `parentIndex`, `kind`, `name` and
`length`. That maps onto DeForm6's three integer spaces exactly:
`absoluteAddress` is a `Va`, `relativeAddress` is an `Rva`, and
`offsetFromParent` with `parentIndex` is an `Off` inside a named section.
Reuse the field names. Do not emit SARIF itself, because SARIF's frame is
"findings against source", not "recovered artifacts with grades".

**capa (Mandiant)** is prior art on grading an item with its evidence. Its
result document is `meta` plus `rules`, where each rule match carries a nested
node tree with a `success` flag and a `locations` array of addresses per node.
Two lessons come from it.

The good lesson: split a `meta` header from the findings. capa's `meta` holds
the sample hash and path, the tool version, the `argv`, and the analysis
parameters. That is what makes a report reproducible. Copy it.

The warning: capa's evidence is a recursive match tree, and Mandiant had to
build and ship **capa Explorer Web**, a browser application, so that people
could read capa's own JSON. Take that as a caution. A deeply nested evidence
tree is unreadable without a viewer, and DeForm6 is not going to ship a viewer.

**in-toto and SLSA provenance** contribute one idea: separate the thing being
described from the claim about it. `subject` plus `predicate`. In our terms:
the recovered project is one artifact on disk, and the report is a separate
document that points into it. Do not mix the grades into the `.frm` files. The
`AGENTS.md` rule about uncertainty markers in code regions only already says
the same thing from the other direction.

### The recommended schema shape

Flat. Keyed by a path. Evidence as a short array, never a tree.

```json
{
  "schema_version": 1,
  "meta": {
    "tool": "deform6",
    "tool_version": "0.1.0",
    "argv": ["deform6", "extract", "Mandelbrot.exe", "-o", "out/"],
    "mode": "strict",
    "input": {
      "path": "Mandelbrot.exe",
      "size": 73728,
      "sha256": "e3b0c442..."
    },
    "image": {
      "format": "pe32-i386",
      "image_base": 4194304,
      "entry_rva": 5156,
      "runtime": "MSVBVM60.DLL",
      "compilation": "native"
    }
  },
  "items": [
    {
      "path": "/forms/frmMain",
      "kind": "form",
      "confidence": "proved",
      "basis": "read-from-structure",
      "value": "frmMain",
      "evidence": [
        {
          "what": "ObjectInfo.lpObjectName",
          "kind": "string",
          "absolute_address": 4304912,
          "relative_address": 110608,
          "section": ".text",
          "offset_from_parent": 44560,
          "length": 8
        }
      ]
    },
    {
      "path": "/forms/frmMain/controls/cmdRender/properties/Caption",
      "kind": "property",
      "confidence": "proved",
      "basis": "read-from-structure",
      "value": "Render",
      "evidence": [ { "what": "FormData.property", "offset_from_parent": 51200, "section": ".text", "length": 6 } ]
    },
    {
      "path": "/modules/modFilters/procedures/ApplyGamma/args/1/type",
      "kind": "argument-type",
      "confidence": "inferred",
      "basis": "type-descriptor",
      "value": "Double",
      "evidence": [ { "what": "FuncTypDesc.vt", "offset_from_parent": 60112, "section": ".text", "length": 2 } ],
      "note": "The descriptor names vt 5. The ByRef bit is set."
    }
  ],
  "defects": [
    {
      "severity": "recoverable",
      "site": { "offset": 65540, "structure": "FormData", "field": "cControls" },
      "kind": "implausible_count",
      "message": "count 65535 exceeds the 2048 that the file can hold"
    }
  ],
  "recovery": {
    "forms": { "recovered": 3, "expected": 3 },
    "controls": { "recovered": 41, "expected": 41 },
    "procedures": { "recovered": 12, "expected": 17 },
    "ratio": 0.9298
  }
}
```

### The rules that keep it usable

1. **`items` is a flat array, not a tree.** The hierarchy lives in the `path`
   string. This is the single decision that keeps the file readable. It stays
   greppable, it diffs line by line in git, and one `jq` expression answers a
   question:
   `jq '.items[] | select(.confidence=="inferred") | .path' report.json`
   A nested tree needs a viewer. See capa.

2. **`path` is the address of the item in the recovered project**, and it is
   stable across runs. `/forms/<name>/controls/<name>/properties/<name>`. This
   is what a test pins and what a diff between two versions compares.

3. **`confidence` is a small closed enum, not a number.** Three values:
   `proved`, `inferred`, `guessed`. A float invites the "85% recovery" habit
   that `AGENTS.md` forbids. A word forces a definition. Suggested
   definitions: `proved` means the value is read from a structure field whose
   meaning is documented; `inferred` means it is derived from a descriptor
   through a rule; `guessed` means a heuristic chose it and it may be wrong.

4. **`basis` names the rule, `evidence` names the bytes.** They answer
   different questions. `basis` is why we believe it. `evidence` is where to
   look. Both are needed for a report that is used as evidence.

5. **`evidence` is an array of flat address records, capped at a few entries.**
   Field names come from SARIF. No nesting, ever.

6. **Serialise every map as a `BTreeMap` and every list as a `Vec`.** Then
   `serde_json::to_string_pretty` is byte-for-byte deterministic across runs
   and across machines. That is what makes the report itself pinnable in a
   test.

7. **No floats except `recovery.ratio`, and round that to four places before
   serialising.** A float that differs in the last bit between two machines
   breaks a pinned test for no reason.

8. **`schema_version` is an integer and it goes first.** Bump it when a
   consumer would break.

9. **`defects` is a separate array from `items`.** An item that was recovered
   with a defect nearby carries a `note`, and the defect carries the site. Do
   not force a defect to attach to an item, because the interesting ones happen
   where no item exists.

---

## 5. Fuzzing

### The choice: `cargo-fuzz` 0.13.2 with `libfuzzer-sys` 0.4.13

The target is "hand this function a byte slice, it must never panic". That is
the exact case libFuzzer is built for, and coverage-guided mutation is what
finds the deep paths in a pointer-chasing parser.

```rust
// fuzz/fuzz_targets/parse.rs
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    // The real API. Not a special path.
    let _ = deform6::inspect(data, deform6::Mode::Strict);
    let _ = deform6::inspect(data, deform6::Mode::Salvage);
});
```

Fuzz both modes. `Salvage` reaches code that `Strict` refuses before it gets
there, and that code is the least exercised in the crate.

### The nightly question, and why it is acceptable

`cargo-fuzz` needs a nightly toolchain, because it passes `-Z` flags for
sanitizer instrumentation. That is not going away.

It is acceptable, because nightly is confined to one place and never blocks
anything. Three facts make this true.

**Fact 1. The fuzz crate is cut out of the workspace, but you must ask for
it.** `cargo fuzz init` takes a `--fuzzing-workspace` flag, and its default
value in current `cargo-fuzz` is **`false`**. The default therefore leaves the
fuzz crate inside the parent workspace, where `cargo test --workspace` and
`cargo clippy --all-targets` on stable both try to build a `#![no_main]` crate
that needs `-Z sanitizer`. That breaks the gate in `AGENTS.md`.

Do two things, not one.

```sh
cargo fuzz init --fuzzing-workspace true --fuzz-dir crates/deform6/fuzz -t parse
```

and add a belt to the braces in the root `Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["crates/deform6", "crates/deform6-cli"]
exclude = ["crates/deform6/fuzz"]
```

The `exclude` line stops cargo from complaining about a nested package that is
neither a member nor excluded, and it holds even if a later `cargo-fuzz`
changes the flag default again.

The fuzz directory goes at `crates/deform6/fuzz`, not at the repository root.
The generated `fuzz/Cargo.toml` names the crate under test as
`[dependencies.deform6] path = ".."`, so the fuzz directory must be a sibling
of the library's `Cargo.toml`.

**Fact 2. Nightly only finds crashes. It never prevents regressions.**
The fuzz target body is one call to the ordinary public API. A crash that
libFuzzer finds becomes a file in `crates/deform6/tests/regressions/`, and a
plain `#[test]` in the library replays every file in that directory through
the same function on stable. See below. Once a crash is committed, nightly is
not needed to keep it fixed.

**Fact 3. It runs on this machine.** libFuzzer needs LLVM sanitizer support.
The Rust unstable book lists `aarch64-apple-darwin` and `x86_64-apple-darwin`
as AddressSanitizer targets, and the Rust Fuzz Book states that Apple Silicon
macOS is supported. The author's host is `aarch64-apple-darwin`.

If nightly ever does become a blocker, the escape is `afl.rs` on the same
target function, not a rewrite.

### How it runs in CI without unbounded time

libFuzzer runs forever by default. Bound it two ways, and use both.

```yaml
# On every pull request. Stable. No nightly. Seconds.
- run: cargo test --workspace          # replays the committed regressions

# On every pull request. Nightly. Bounded. About two minutes.
- run: cargo install cargo-fuzz
- run: |
    cargo +nightly fuzz run --fuzz-dir crates/deform6/fuzz parse -- \
      -max_total_time=60 -rss_limit_mb=2048

# Nightly cron. Longer. Uploads any new crash as an artifact.
- run: |
    cargo +nightly fuzz run --fuzz-dir crates/deform6/fuzz parse -- \
      -max_total_time=1800 -rss_limit_mb=2048
```

`--fuzz-dir` is needed on every `cargo fuzz` call, because the fuzz crate is
not at the repository root. Put it in a shell alias or an `xtask` command so
nobody forgets it and creates a second fuzz directory by mistake.

Notes that matter.

- `-max_total_time=60` is a wall clock bound, so the job cannot hang.
- `-runs=N` is the deterministic alternative when a flaky time bound is worse
  than a flaky iteration count. Use `-runs` if CI machines vary a lot.
- `-rss_limit_mb` catches an allocation sized from a length field. That is a
  named project constraint, so set it explicitly rather than take the default.
- Seed the corpus from `corpus/` before the first run. 44 real VB6 executables
  is an excellent seed set, and it is already in the repository. Copy them into
  `fuzz/corpus/parse/` in the CI step, or point libFuzzer at both directories.
  A fuzzer that starts from a real VB6 file finds a VB6 parser bug far sooner
  than one that starts from nothing.
- `cargo-fuzz` builds with debug assertions on by default. That is how a
  wrapping `+` gets caught, if one ever slips past the lint wall.

### How a found crash becomes a regression test

This is the part to get right, because it is what makes the fuzzing permanent.

`cargo fuzz` writes a crash to
`crates/deform6/fuzz/artifacts/parse/crash-<sha1>`. Do not leave it there.
The `artifacts/` directory is scratch and it is gitignored.

1. Copy the file from `crates/deform6/fuzz/artifacts/parse/crash-<sha1>` to
   `crates/deform6/tests/regressions/<sha1-prefix>.bin`.
2. Write `crates/deform6/tests/regressions/<sha1-prefix>.md` with one
   paragraph: what the input does, which parse site broke, and the commit that
   fixed it.
3. Commit the input and the fix in the same commit. `AGENTS.md` requires the
   code and its tests in one commit.

The replay test lives in the library and runs on stable:

```rust
// crates/deform6/tests/regressions.rs
#[test]
fn no_committed_crash_input_panics() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/regressions");
    let mut count = 0usize;
    for entry in std::fs::read_dir(&dir).expect("the regressions directory exists") {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("bin") { continue; }
        let data = std::fs::read(&path).expect("a readable regression input");
        // If this panics, the test fails and names the file.
        let _ = deform6::inspect(&data, deform6::Mode::Strict);
        let _ = deform6::inspect(&data, deform6::Mode::Salvage);
        count += 1;
    }
    assert!(count > 0, "the regressions directory is empty; the test proves nothing");
}
```

The `assert!(count > 0)` matters. `AGENTS.md` warns about a test that passes
for the wrong reason. A loop over an empty directory passes and tests nothing.

These inputs are our own bytes, produced by our own fuzzer from our own corpus.
They are not a derived fixture from a third party system, so the `AGENTS.md`
evidence rule permits them. A crash found by mutating a `corpus/` file is a
mutation of a BSD-2 or Unlicense file, which both permit it.

### Rejected: `afl.rs` 0.18.2

5 crates and it works on stable, which is its one real advantage.

Against it: `cargo afl` builds AFL++ from source at install time, so CI needs a
C toolchain and a slow install step. It needs its own instrumented build with
`cargo afl build`, so the workflow is two commands instead of one. Its output
directory layout is more work to turn into committed regression files. And its
input corpus and dictionary handling is heavier than a project of this size
needs.

Keep it as the named fallback if the nightly requirement ever bites.

### Rejected: `proptest` 1.11.0

**26 crates.** That alone settles it for a project that counts them.

The technical reason is better. `proptest` generates structured values from a
strategy and shrinks failures well. It is not coverage guided. A parser bug
lives behind a specific byte sequence that gets past four earlier checks, and
random structured generation does not find that. Mutation guided by coverage
does.

### Rejected: `arbitrary` 1.4.2 as a direct dependency

Not rejected as a concept. It is simply not needed. `libfuzzer-sys` already
depends on it, and our target takes `&[u8]` directly, so we never write an
`Arbitrary` impl and never take the derive.

Take `arbitrary` with `derive` only if a second fuzz target ever needs a
structured input, such as a generated `.frm` for the writer. That is a later
milestone.

### A cheap fuzz smoke test on stable, in the normal gate

Add one stable test that mutates real corpus files with a fixed-seed PRNG and
feeds the results through `inspect`. Twenty lines, a hand-written xorshift, no
dependency, a few thousand cases in under a second. It will not find what
libFuzzer finds. It does catch a gross regression on every `cargo test` run,
including on a machine with no nightly toolchain and no `cargo-fuzz`.

Use a fixed seed. A test that fails differently each run is not a test.

---

## 6. Command line

### The choice: `clap` 4.6.6 with default features off

```toml
clap = { version = "4.6.6", default-features = false, features = ["std", "derive", "help", "error-context"] }
```

**10 crates measured on its own. 6 marginal in this workspace**, because
`proc-macro2`, `quote`, `syn` and `unicode-ident` already arrive with `serde`
derive. The 6 are `clap`, `clap_builder`, `anstyle`, `clap_lex`, `clap_derive`
and `heck`. Four of those link in. Two are build time.

`clap` with default features is 17 crates. Turning them off drops the
`anstream` colour stack, `anstyle-parse`, `anstyle-query`, `colorchoice`,
`utf8parse`, `is_terminal_polyfill` and `strsim`. Colour in help output and
"did you mean" suggestions are not worth 7 crates.

Keep `help` and `error-context`. Without them the binary gives no `--help`
text and no message that says which argument was wrong. That is the whole
value of taking an argument parser.

The derive fits the shape of the CLI:

```rust
#[derive(clap::Parser)]
#[command(name = "deform6", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Read a file and write a report. Change nothing on disk.
    Inspect {
        input: std::path::PathBuf,
        /// Write the JSON report here instead of standard output.
        #[arg(long)]
        report: Option<std::path::PathBuf>,
        /// Recover what is readable from a damaged file.
        #[arg(long)]
        salvage: bool,
    },
    /// Write a Visual Basic project directory.
    Extract {
        input: std::path::PathBuf,
        #[arg(short, long)]
        out: std::path::PathBuf,
        #[arg(long)]
        report: Option<std::path::PathBuf>,
        #[arg(long)]
        salvage: bool,
        /// Refuse to write into a directory that is not empty.
        #[arg(long)]
        force: bool,
    },
}
```

`PathBuf` matters. A user's file path is not guaranteed to be UTF-8. `clap`
handles `OsString` correctly.

### Rejected: `argh` 0.1.19

11 crates, which is more than the trimmed `clap`. This is a change from what
`argh` used to cost: `argh_shared` 0.1.19 now depends on `serde`.

`argh` also follows the Fuchsia argument style, where a value is written
`--out value` and never `--out=value`. That surprises people. And it has no
built-in `--version`.

### Rejected: `lexopt` 0.3.2 and `pico-args` 0.5.0

1 crate each, no transitive deps, and both handle `OsString` correctly.
`lexopt` in particular is well built.

They give you a token stream and nothing else. Two subcommands, nine flags,
`--help` text for each subcommand, `--version`, and a correct error message
for a bad argument all become hand-written code. That is a hundred lines that
does not decompile anything and that drifts from the flags as they change.

Six marginal crates for generated help that cannot drift is the better trade
here. Revisit if the CLI ever shrinks to one command and two flags.

---

## 7. Workspace layout

`corpus/` already exists at the repository root with 4.6 MB, 44 executables,
45 project files and a `NOTICES` file. Keep it there. It is a repository
asset, not a fixture of one crate, and both crates and the fuzzer read it.

```
DeForm6/
├── Cargo.toml                  # [workspace], resolver "3", members, exclude, [profile]
├── Cargo.lock                  # committed; a binary crate pins its inputs
├── rust-toolchain.toml         # pins the stable channel and the components
├── AGENTS.md
├── LICENSE                     # MIT
├── README.md
├── .gitignore                  # /target/, /corpus/fetched/, .DS_Store
│
├── crates/
│   ├── deform6/                # the product
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # the lint wall lives here; pub API: inspect, extract
│   │   │   ├── read/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── region.rs   # Region, Off, Rva, Va. No dependency.
│   │   │   │   └── pe.rs       # PeImage. The only file that names `object`.
│   │   │   ├── vb/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── header.rs   # VB5! signature, VBHeader
│   │   │   │   ├── project.rs  # ProjectInfo, ExternalTable
│   │   │   │   ├── objects.rs  # ObjectTable, ObjectInfo
│   │   │   │   ├── form.rs     # GUI form data, control tree, properties
│   │   │   │   └── proto.rs    # FuncTypDesc, PubVarDesc, EventDesc
│   │   │   ├── model.rs        # the recovered object graph; serde types
│   │   │   ├── write/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── vbp.rs      # the project file
│   │   │   │   ├── frm.rs      # the form file and its .frx
│   │   │   │   └── code.rs     # .bas and .cls skeletons
│   │   │   ├── report.rs       # the schema from section 4
│   │   │   ├── error.rs        # Error, Defect, DefectKind, Site, Severity
│   │   │   └── journal.rs      # Journal, Mode, the strict/salvage choke point
│   │   ├── tests/
│   │   │   ├── differential.rs # the gate; see section 8
│   │   │   ├── ratios.toml     # the pinned recovery ratio per program
│   │   │   ├── regressions.rs  # replays every committed crash input
│   │   │   ├── regressions/    # the crash inputs and one .md note each
│   │   │   │   ├── 0a3f91c2.bin
│   │   │   │   └── 0a3f91c2.md
│   │   │   ├── refusal.rs      # a VB5 file, a .NET file, an empty file
│   │   │   └── support/        # a second, independent reader for .vbp and .frm
│   │   └── fuzz/               # its own crate; excluded from the workspace
│   │       ├── Cargo.toml      # [workspace] members = ["."]; deform6 path = ".."
│   │       ├── fuzz_targets/
│   │       │   └── parse.rs
│   │       ├── corpus/parse/   # gitignored; CI seeds it from corpus/
│   │       └── artifacts/      # gitignored; a crash gets promoted, not left
│   │
│   └── deform6-cli/            # a thin shell
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs         # clap derive, ExitCode, nothing else
│       │   └── error.rs        # the twelve line CLI enum
│       └── tests/
│           └── cli.rs          # argument handling and exit codes only
│
├── corpus/                     # committed; redistributable licences only
│   ├── NOTICES                 # origin, licence and upstream commit per program
│   ├── vb6-code/               # BSD-2, 31 executables
│   ├── public-domain/          # Unlicense, 13 executables
│   ├── manifest.toml           # the fetched set: URL plus SHA-256, bytes never committed
│   └── fetched/                # gitignored; the fetch tool writes here
│
├── xtask/                      # optional; a cargo-run helper, no dependency on the lib
│   └── src/main.rs             # fetch-corpus, seed-fuzz-corpus, update-ratios
│
└── .planning/
```

Three points on this tree.

**The fuzz crate sits at `crates/deform6/fuzz`, not at the repository root.**
The `cargo fuzz` template writes `[dependencies.deform6] path = ".."`, so the
fuzz directory must be a sibling of the library's `Cargo.toml`.

**The fuzz crate is outside the workspace, and you must say so twice.** Pass
`--fuzzing-workspace true` to `cargo fuzz init`, because the flag defaults to
`false`. Also put `exclude = ["crates/deform6/fuzz"]` in the root
`Cargo.toml`. Together they keep the nightly crate out of `cargo test
--workspace` and `cargo clippy --all-targets`, which is what lets the gate in
`AGENTS.md` run on stable.

**`crates/deform6/tests/` holds no binary fixture of its own.** Every input is
either a `corpus/` program or a crash input that our own fuzzer produced. That
satisfies the `AGENTS.md` rule that no derived fixture from a third-party
system enters the repository.

---

## 8. The differential test harness

This is the verification gate, so it gets the most design attention.

### The question the harness asks

Parse a real executable from `corpus/`. Extract the object graph. Compare it
against the `.vbp` and `.frm` files that are committed next to it and that the
executable was built from.

Not against our own writer's output. `AGENTS.md` is explicit: a round trip
through our own writer and our own parser proves only that the code agrees with
itself.

### Test organisation

One integration test file, data driven over the corpus.

```
crates/deform6/tests/
├── differential.rs      # the harness
├── ratios.toml          # the pinned number per program
└── support/
    ├── mod.rs
    ├── vbp.rs           # reads a .vbp: Form=, Module=, Class=, ExeName32=
    ├── frm.rs           # reads a .frm: the Begin VB.Form tree and the properties
    └── rules.rs         # the exclusion rules, in one place
```

`support/vbp.rs` and `support/frm.rs` are a **second, independent reader** for
the text formats. They must not call anything in `src/write/`. A harness that
shares code with the thing it tests can agree with a bug in that code.

### The exclusion rules

Some things are absent from the binary by design. Exclude them by rule, in
`support/rules.rs`, never by a per-program list of exceptions. A per-program
exception list is where a real regression hides.

The rules:

1. Drop every line whose first non-space character is an apostrophe. Comments
   are not compiled.
2. Drop every local variable. A `Dim` inside a `Sub` or `Function` body has no
   name in the binary.
3. Drop every `Private Sub` and `Private Function` name. Only public names
   reach the object table.
4. Drop all whitespace and line ending differences.
5. Drop the ordering of properties inside one control. VB6 writes them in a
   fixed order that the binary does not preserve.
6. Drop design-time-only properties. `ClientHeight`, `ClientLeft`,
   `ClientTop`, `ClientWidth`, `_ExtentX`, `_ExtentY` and the `Attribute`
   lines are IDE bookkeeping.
7. Drop the `Object=` GUID version suffix in a `.vbp`. The binary records the
   type library, not the `.vbp` text.

Each rule gets a one-line comment that says **why** the compiler drops the
thing. A rule with no reason is a rule that hides a bug.

### How expectations are stored

Two kinds, and they are stored differently on purpose.

**Kind 1: the ground truth. Stored as the original source, unchanged.** The
`.vbp` and `.frm` files in `corpus/` are the expectation. Nothing is
pre-computed from them and committed. `AGENTS.md` says to build the state a
test needs inside the test, and this is that: the harness reads the `.frm` at
test time and derives the expected control tree in memory.

The reason is the failure mode. A committed `expected.json` derived from a
`.frm` drifts silently when the exclusion rules change, and then the test
passes against a stale expectation. Deriving it at test time makes that
impossible.

**Kind 2: the recovery ratio. Stored as one committed TOML file.** This one
cannot be derived, because it is a measurement of us, not of the corpus.

```toml
# crates/deform6/tests/ratios.toml
#
# The recovery ratio for each corpus program.
# recovered / expected, measured against the original source.
# Run `cargo run -p xtask -- update-ratios` to rewrite this file.
# A number that goes down is a regression. Find out why before you accept it.

[vb6-code.Mandelbrot]
forms = "3/3"
controls = "41/41"
procedures = "12/17"
ratio = 0.9298

[vb6-code.Artificial-life]
forms = "1/1"
controls = "18/18"
procedures = "9/9"
ratio = 1.0
```

Store the two counts, not only the quotient. `12/17` says what moved.
`0.9298` alone does not. When a ratio changes you need to know whether the
numerator went up or the denominator went down, and a denominator that moved
means the corpus or the counting rule changed, which is a different problem
from a parser regression.

### How to pin the ratio so the message gives the new number

This is the pattern the author's other project already uses, and it is worth
stating exactly.

Pin the **exact** value, not a floor. A floor lets an improvement pass
unrecorded, and an unrecorded improvement means nobody checked whether
"recovered" still means the same thing.

Then split the failure into two messages, because they are two different
events with two different correct responses.

```rust
fn check_ratio(program: &str, pinned: &Pin, measured: &Measured) {
    if measured == pinned { return; }

    if measured.ratio < pinned.ratio {
        panic!(
"\n\
REGRESSION in {program}.\n\
\n\
  pinned    {pinned_recovered}/{pinned_expected}  ratio {pinned_ratio:.4}\n\
  measured  {new_recovered}/{new_expected}  ratio {new_ratio:.4}\n\
\n\
The recovery ratio went DOWN. Find out why before you accept it.\n\
Items that the pin expects and this run did not recover:\n\
{missing}\n"
        );
    }

    panic!(
"\n\
The recovery ratio for {program} MOVED UP.\n\
\n\
  pinned    {pinned_recovered}/{pinned_expected}  ratio {pinned_ratio:.4}\n\
  measured  {new_recovered}/{new_expected}  ratio {new_ratio:.4}\n\
\n\
If that is what you intended, put this in crates/deform6/tests/ratios.toml:\n\
\n\
[{toml_key}]\n\
forms = \"{f_rec}/{f_exp}\"\n\
controls = \"{c_rec}/{c_exp}\"\n\
procedures = \"{p_rec}/{p_exp}\"\n\
ratio = {new_ratio:.4}\n\
\n\
Or run:  cargo run -p xtask -- update-ratios\n"
    );
}
```

Five properties make this message do its job.

1. **It prints the exact TOML block to paste.** No arithmetic for the author,
   no chance of a typo, no rounding argument.
2. **It names the file to edit** with the full path from the repository root.
3. **It gives the one command that rewrites the file.** `update-ratios` writes
   `ratios.toml` from a full run. Never make this automatic and never read an
   environment variable inside the test, because a variable set once in a
   shell silently disables the gate for the rest of the session.
4. **A drop and a rise print different words.** `REGRESSION` for a drop.
   `MOVED UP` for a rise. The same event with the same number means two
   different things, and the message must not make the author work that out.
5. **A drop lists which items went missing.** The number says that something
   broke. The list says what. That is the difference between a five minute fix
   and an afternoon.

Round the ratio to four places before comparing. `assert_eq!` on an unrounded
`f64` fails between two machines for no reason.

### Two more assertions that belong in the same harness

**Recompilation is the stated bar, so test the shape of the output.** Full
recompilation needs VB6 on Windows and cannot run in this CI. What can run is a
structural check: every `Form=` line in the written `.vbp` names a file that
exists, every `.frm` parses back through `support/frm.rs`, and every control
name is a legal VB6 identifier. That is a real check and it costs nothing.

**Test the refusals.** A separate `refusal.rs` asserts that a VB5 executable,
a .NET executable and an empty file each fail with the specific `Error`
variant and not a generic one. The requirement in `PROJECT.md` is that the
tool "refuses everything else with a clear sentence", so the sentence is part
of the test.

And follow the `AGENTS.md` rule when adding either: break the thing on purpose
and watch the test fail. A test that cannot fail is worse than no test.

### Rejected: `insta` 1.48.0

12 crates, and it is a dev dependency so the cost is contained.

It is the wrong tool for the main assertion. `insta` compares a rendered
snapshot against a committed file. The primary expectation here is not a
snapshot of our output. It is the original `.vbp` and `.frm` source, which is
already in the repository and must be read at test time. Wrapping that in a
snapshot adds a second copy that can go stale.

The ratio pin is a snapshot, and `insta` could hold it. But `insta`'s failure
message shows a diff. What the author wants is a printed TOML block and a
named command, and the fifty lines above give exactly that with no dependency.

### Rejected: `expect-test` 1.5.1

3 crates, which is genuinely cheap, and its inline `expect![[...]]` blocks
with `UPDATE_EXPECT=1` are close to the pattern above.

Rejected for the same reason plus one. `expect-test` updates through an
environment variable. A variable that is set once in a shell stays set, and
then every later run rewrites the expectation instead of failing. For a gate
that exists to catch a silent regression, that is the wrong default. An
explicit `xtask` command cannot be left on by accident.

Reasonable for a smaller assertion elsewhere. Not for the gate.

---

## Sources

Measured on this machine, 2026-09-07:

- Dependency counts, `cargo tree -e normal --prefix none --no-dedupe` into a
  fresh crate per candidate.
- Malformed input probe against `object::read::pe::PeFile32::parse`, four
  inputs, four clean errors.
- Lint wall proof, `cargo clippy` on a crate with the deny block from section
  2, five bad functions and five errors, four good functions and no error.
- `hmac-sha256` against the NIST SHA-256 vector for `"abc"`.
- The proposed workspace at `/tmp/finalws`, `cargo check --workspace`, exit 0,
  23 packages in `Cargo.lock`.
- `goblin` 0.10.7 `CHANGELOG.md` from the local registry, grepped for panic,
  overflow and out-of-bounds entries.
- `object` 0.40.0 `README.md` from the local registry, Security section.

Crate metadata from the crates.io API, 2026-09-07:

- <https://crates.io/api/v1/crates/object>, 0.40.0, updated 2026-08-01
- <https://crates.io/api/v1/crates/goblin>, 0.10.7, updated 2026-05-28
- <https://crates.io/api/v1/crates/pelite>, 0.10.0, updated 2022-11-04
- <https://crates.io/api/v1/crates/clap>, 4.6.6
- <https://crates.io/api/v1/crates/thiserror>, 2.0.20
- <https://crates.io/api/v1/crates/serde_json>, 1.0.151
- <https://crates.io/api/v1/crates/hmac-sha256>, 1.1.14
- <https://crates.io/api/v1/crates/cargo-fuzz>, 0.13.2
- <https://crates.io/api/v1/crates/libfuzzer-sys>, 0.4.13

Repository activity from the GitHub API, 2026-09-07:

- `gimli-rs/object`, pushed 2026-08-30, 844 stars, 29 open issues
- `m4b/goblin`, pushed 2026-08-30, 1541 stars, 85 open issues
- `CasualX/pelite`, pushed 2025-08-22, 344 stars, 24 open issues, no release
  since 2022

Documents:

- `object` README, Security section:
  <https://github.com/gimli-rs/object/blob/master/README.md>
- `goblin` CHANGELOG, PE panic and overflow fixes:
  <https://github.com/m4b/goblin/blob/master/CHANGELOG.md>
- Rust unstable book, sanitizer target support, AddressSanitizer on
  `aarch64-apple-darwin`:
  <https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html>
- Rust Fuzz Book, `cargo-fuzz` setup and the nightly requirement:
  <https://rust-fuzz.github.io/book/cargo-fuzz/setup.html>
- `cargo-fuzz`, `--sanitizer none` and CI usage:
  <https://github.com/rust-fuzz/cargo-fuzz>
- Trail of Bits Testing Handbook, `cargo-fuzz` chapter:
  <https://appsec.guide/docs/fuzzing/rust/cargo-fuzz/>
- SARIF 2.1.0 (OASIS), the `address` object with `absoluteAddress`,
  `relativeAddress`, `offsetFromParent` and `parentIndex`:
  <https://docs.oasis-open.org/sarif/sarif/v2.1.0/csprd01/sarif-v2.1.0-csprd01.html>
- Microsoft SARIF tutorials, locations and addresses for binary analysis:
  <https://github.com/microsoft/sarif-tutorials/blob/main/docs/2-Basics.md>
- capa result document model, `meta` plus graded matches with `locations`:
  <https://github.com/mandiant/capa/blob/master/capa/render/result_document.py>
- capa Explorer Web, the viewer that capa's nested JSON needs:
  <https://github.com/mandiant/capa/blob/master/web/explorer/README.md>
