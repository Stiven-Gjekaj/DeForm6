# Phase 5: Hostility - Pattern Map

**Mapped:** 2026-09-13
**Files analyzed:** 13 (7 new, 6 modified-in-shape; the ~24-function retrofit is one repeated pattern, counted once)
**Analogs found:** 13 / 13 (every file has a tracked, git-checked analog or a named exception)

All analog paths below were confirmed with `git ls-files` this session. Every
path named is tracked source, not a build artifact or a gitignored mirror.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/deform6/fuzz/fuzz_targets/parse.rs` | test (fuzz harness) | request-response (byte slice in, no return value read) | `crates/deform6/tests/corpus_sweep.rs` (whole-crate entry-point exerciser) | role-match, generated scaffold |
| `crates/deform6/tests/regressions.rs` | test | batch (directory walk + replay) | `crates/deform6/tests/corpus_sweep.rs` | exact (directory-walk-and-assert shape) |
| `crates/deform6/tests/regressions/` (seed file) | fixture data | file-I/O | `corpus/` (vendored fixture tree) | role-match |
| `crates/deform6/tests/fuzz_smoke.rs` | test | request-response | `crates/deform6/tests/corpus_sweep.rs` | role-match |
| `crates/deform6/tests/no_panic_proof.rs` | test | batch (directory walk, no panic assertion) | `crates/deform6/tests/corpus_sweep.rs` | exact |
| `crates/xtask/src/fetch_corpus.rs` | utility (CLI subcommand module) | file-I/O + request-response (HTTP fetch, hash check, write) | `crates/xtask/src/main.rs` (`derive_opcode_table`/`update_ratios` subcommand shape) | role-match |
| `corpus/manifest.toml` | config | CRUD (read-only, keyed table) | `tests/ratios.toml` (pinned, keyed TOML table `xtask` reads/writes) | role-match |
| `.github/workflows/fuzz.yml` | config (CI) | event-driven | `.github/workflows/gate.yml` | exact (only existing workflow) |
| `crates/deform6/src/journal.rs` (caller wiring, not the type) | library/service (policy choke point) | event-driven (each call site is a decision point) | `crates/deform6/src/vb/object.rs::bound_proc_count` (the one site that already half-implements the shape) | exact for the retrofit target shape |
| ~24 `read`/`parse`/`walk` functions across 8 `vb/*.rs` files | service/parser | transform (bytes to typed structure) | `crates/deform6/src/vb/object.rs::bound_proc_count` and `crates/deform6/src/vb/project.rs::DeclareTable::read` | exact |
| `crates/deform6/src/vb/gui.rs::GuiTable::walk` | service/parser | transform | `crates/deform6/src/vb/object.rs::bound_proc_count` | exact (named gap, same file family) |
| `crates/deform6-cli/src/main.rs` (`--salvage` flag) | CLI/controller | request-response | same file, existing `opcode_table`/`output`/`report`/`force` flag declarations | exact |
| `crates/deform6/src/report.rs` (assumption recording) | model | transform | same file, `Confidence`/`ProjectReport.limits`/`build_limits` | exact |

## Pattern Assignments

### `crates/deform6/src/journal.rs` retrofit (the ~24 call sites, 8 files: `object.rs`, `project.rs`, `frx.rs`, `controlinfo.rs`, `vbstr.rs`, `propstream.rs`, `controltree.rs`, `gui.rs`, plus `vb/mod.rs::inspect` and `write/mod.rs::project`)

**Analog:** `crates/deform6/src/vb/object.rs:279-306` (`bound_proc_count`), read in full this session.

**The type every call site must route through, already shipped and untouched by this phase** (`crates/deform6/src/journal.rs:22-72`):
```rust
use crate::error::{Defect, Error, Severity};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Strict,
    Salvage,
}

pub struct Journal {
    mode: Mode,
    defects: Vec<Defect>,
}

impl Journal {
    pub const fn new(mode: Mode) -> Self { /* mode, defects: Vec::new() */ }
    pub fn defects(&self) -> &[Defect] { &self.defects }

    pub fn record<T>(&mut self, defect: Defect, fallback: T) -> Result<T, Error> {
        self.defects.push(defect.clone());
        match (defect.kind.severity(), self.mode) {
            (Severity::Fatal, _) => Err(Error::Refused(defect)),
            (Severity::Recoverable, Mode::Strict) => Err(Error::Refused(defect)),
            (Severity::Recoverable, Mode::Salvage) => Ok(fallback),
        }
    }
}
```
Note the doc comment at `journal.rs:1-20` states the intended shape in prose:
"A parse site does not know which mode the run is in. It builds a `Defect`,
hands it to `Journal::record` with a fallback value, and uses the `?`
operator." Every retrofit call site should end up looking like that sentence.

**Core pattern to generalize, the one function that already does everything
except consult a `Mode`** (`crates/deform6/src/vb/object.rs:279-306`, quoted
in full, VERIFIED):
```rust
fn bound_proc_count(
    pe: &PeImage<'_>,
    element: &Region<'_>,
    lp_proc_names_array: Va,
    raw_proc_count: u32,
) -> (u32, Option<Defect>) {
    let Some(array_region) = pe.region_at_va(lp_proc_names_array) else {
        return (raw_proc_count, None);
    };
    let max_entries = array_region
        .len()
        .checked_div(PROC_NAME_PTR_SIZE)
        .unwrap_or(0);
    if raw_proc_count <= max_entries {
        return (raw_proc_count, None);
    }
    let offset = element.file_offset(Off::new(0x1C)).map_or(0, Off::get);
    let defect = Defect {
        site: Site { offset, rva: None, structure: "Object", field: "ProcCount" },
        kind: DefectKind::ImplausibleCount { offset, count: raw_proc_count, max: max_entries },
    };
    (max_entries, Some(defect))
}
```
This returns `(u32, Option<Defect>)` and the caller always accepts the
clamped value today (there is no `Mode` consulted anywhere near it). The
retrofit's job at this call site, and at every other of the ~24, is to
replace the tuple return with a `journal.record(defect, max_entries)?` call
(or equivalent), so `Mode::Strict` refuses instead of silently clamping.

**Signature surface to change (public entry points that must gain a `Mode`
parameter):**
```rust
// crates/deform6/src/vb/mod.rs:344 (current)
pub fn inspect(data: &[u8], opcode_table: &OpcodeTable) -> Result<Report, Refusal>

// crates/deform6/src/write/mod.rs:99 (current)
pub fn project(report: &Report, data: &[u8]) -> Result<WrittenProject, Refusal>
```
Both currently return `Refusal`, not `Error`. `Journal::record` returns
`Result<T, crate::error::Error>` (`Error::Refused(Defect)`), a distinct type
from `Refusal`. A grep this session for `impl From<Error>` in `error.rs`
found none: the retrofit must add one, or an equivalent explicit mapping
function, at the point `inspect`/`project` propagate a `Journal::record`
failure outward. `Refusal::Damaged(&'static str)` (`error.rs:425` onward)
takes a static string, not a `Defect`, so a straight `From` impl needs a new
`Refusal` variant or a `.to_string()`-shaped conversion; this is an open
design point the plan must resolve (see RESEARCH.md Assumption A1/A3), not
something this pattern map can pick for it.

**Severity table already shipped** (`error.rs:290-, 304-, 319`):
```rust
pub enum Severity {
    Fatal,
    Recoverable,
}
// DefectKind::severity(&self) -> Severity, per-variant match, e.g.:
Self::ImplausibleCount { .. } => Severity::Recoverable,
```

---

### `crates/deform6/src/vb/gui.rs::GuiTable::walk` (the named SAF-04 gap)

**Analog:** `crates/deform6/src/vb/object.rs::bound_proc_count` (same file family, twelve files away, already correct).

**Current shape, with the gap** (`crates/deform6/src/vb/gui.rs:79-111`, VERIFIED):
```rust
pub fn walk(pe: &PeImage<'_>, header: &VbHeader) -> Result<Self, Refusal> {
    let array = pe
        .region_at_va(header.lp_gui_table)
        .ok_or(Refusal::Damaged("the GUI table pointer is in no section"))?;

    let mut entries = Vec::new();
    for i in 0_u32..u32::from(header.w_form_count) {
        let at = i
            .checked_mul(GUI_ENTRY_SIZE)
            .ok_or(Refusal::Damaged("the GUI table index overflows a u32"))?;
        let entry = array
            .subregion(Off::new(at), GUI_ENTRY_SIZE)
            .ok_or(Refusal::Damaged("the file ends inside a GUI table entry"))?;
        let l_struct_size = entry
            .u32_le(Off::new(0x00))
            .ok_or(Refusal::Damaged("a GUI table entry holds no lStructSize"))?;
        if l_struct_size != GUI_ENTRY_SIZE {
            let offset = entry.file_offset(Off::new(0x00)).map_or(0, Off::get);
            return Err(damaged(format!(
                "the GUI table entry at file offset {offset:#x} holds lStructSize \
                 {l_struct_size:#x}, not {GUI_ENTRY_SIZE:#x}"
            )));
        }
        let a_form_pointer = entry.va_le(Off::new(0x48)).ok_or(Refusal::Damaged(
            "a GUI table entry holds no address for its form",
        ))?;
        entries.push(GuiTableEntry { a_form_pointer });
    }
    Ok(Self { entries })
}
```
`header.w_form_count` (a `u16`) is never compared against
`array.len() / GUI_ENTRY_SIZE` before the loop starts. The fix: compute
`array.len().checked_div(GUI_ENTRY_SIZE)` once, before the loop, compare
against `u32::from(header.w_form_count)`, and if it exceeds, build a
`Defect { kind: DefectKind::ImplausibleCount { offset, count, max }, .. }`
the same shape `bound_proc_count` builds, and route it through
`Journal::record`, exactly like every other of the ~24 call sites this
phase retrofits (Strict refuses, Salvage clamps the loop bound to
`max`). Do not invent a new bound-check helper; copy `bound_proc_count`'s
four lines (null-check the region, divide, compare, build-and-record) into
this function's own shape.

---

### `crates/deform6/tests/regressions.rs` and `crates/deform6/tests/no_panic_proof.rs` (directory-walk-and-assert harnesses)

**Analog:** `crates/deform6/tests/corpus_sweep.rs` (whole file read this session, 239 lines). This is the one directory-walking test harness already in the tree; reuse its walk shape rather than writing a second one.

**Directory-walk helper to copy** (`crates/deform6/tests/corpus_sweep.rs:37-74`, VERIFIED):
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

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("reading an entry of {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push(path);
        }
    }
}
```
`regressions.rs` and `no_panic_proof.rs` should each call the same shape
(root swapped to `tests/regressions/` or `corpus/fetched/`, extension filter
dropped or widened since regression files carry no fixed extension), rather
than writing a second, independent recursive walk. The count assertion this
pattern implies (`assert_eq!(count, 44, ...)` at `corpus_sweep.rs:80`) is the
direct analog for SC2's "emptying the directory makes the test fail":
`regressions.rs` needs the same shape asserting `count > 0` (or a named
minimum), not `== 44`.

**Top-of-file `#![allow(...)]` convention for a test that builds its own
fixtures and must fail loudly** (`corpus_sweep.rs:1-9`, copy verbatim,
adjusting the reason if it no longer fits):
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

**Both-modes replay pattern** (from RESEARCH.md's recommended shape, not yet
written, but directly implied by `journal.rs`'s existing `Mode` enum and
`corpus_sweep.rs`'s existing `inspect(&bytes, &table)` call at
`crates/deform6/tests/ratios.rs`'s embedded `differential` module, see
`crates/xtask/src/main.rs`'s `measure_all`, which calls
`deform6::inspect(&bytes, &table)` today, no `Mode` argument): once `inspect`
gains a `Mode` parameter, `regressions.rs` calls it twice per file, once with
`Mode::Strict` and once with `Mode::Salvage`, asserting neither panics
(the assertion is "does not panic," proven by the process completing, not by
the `Result` variant, a `Result::Err` from `Mode::Strict` refusing a known-bad
regression file is an expected, passing outcome).

---

### `crates/deform6/fuzz/fuzz_targets/parse.rs`

**Analog:** no in-tree analog for the harness shape itself (this is genuinely
new infrastructure, per RESEARCH.md); nearest in-tree shape is
`crates/xtask/src/main.rs::measure_all`'s own `inspect(&bytes, &table)` call,
for how this crate already calls its own library entry point from outside
`crates/deform6/src`.

**Recommended shape** (from RESEARCH.md, not yet written; matches the
already-existing `OpcodeTable::builtin()` call `crates/xtask/src/main.rs`
uses):
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let table = deform6::vb::opcodes::OpcodeTable::builtin();
    let _ = deform6::inspect(data, &table, deform6::journal::Mode::Strict);
    if let Ok(report) = deform6::inspect(data, &table, deform6::journal::Mode::Salvage) {
        let _ = deform6::write::project(&report, data, deform6::journal::Mode::Salvage);
    }
});
```
Do not hand-write `crates/deform6/fuzz/Cargo.toml`; it is generated by
`cargo +nightly fuzz init --fuzzing-workspace true --fuzz-dir crates/deform6/fuzz`
and only `fuzz_targets/parse.rs` is hand-edited afterward.

---

### `crates/xtask/src/fetch_corpus.rs` and its `xtask` subcommand wiring

**Analog:** `crates/xtask/src/main.rs`, whole file read this session, the
existing `update-ratios`/`derive-opcode-table` subcommand dispatch.

**Subcommand dispatch shape to copy** (`crates/xtask/src/main.rs:73-88`,
VERIFIED):
```rust
fn run(args: Vec<String>) -> i32 {
    match args.first().map(String::as_str) {
        Some("update-ratios") => update_ratios(),
        Some("derive-opcode-table") => {
            opcode_table::derive_opcode_table(args.get(1..).unwrap_or(&[]))
        }
        Some("--help" | "-h") => {
            println!("{USAGE}");
            0
        }
        Some(other) => {
            eprintln!("xtask: unknown subcommand {other:?}\n{USAGE}");
            1
        }
        None => {
            eprintln!("xtask: missing subcommand\n{USAGE}");
            1
        }
    }
}

const USAGE: &str = "usage: cargo run -p xtask -- update-ratios | derive-opcode-table";
```
`fetch-corpus` (and any `fuzz-ci` subcommand plan 05-04 needs) is a new
`Some("fetch-corpus") => fetch_corpus::run(args.get(1..).unwrap_or(&[]))` arm
in this same `match`, plus an update to `USAGE`'s literal string. The module
itself (`fetch_corpus.rs`) follows `opcode_table.rs`'s file-per-subcommand
convention (a `mod opcode_table;` declaration at `main.rs:60` next to a
sibling `opcode_table.rs` file), add `mod fetch_corpus;` the same way.

**Error-as-`String`, process-exit-code convention** (`main.rs`'s
`update_ratios`/`update_ratios_inner` pair and the `Result<_, String>` return
shape used throughout): every fallible xtask step returns `Result<T, String>`
and the top-level subcommand function prints the string to stderr and
returns `1`; `fetch_corpus::run` should return `i32` the same way
`derive_opcode_table` and `update_ratios` do, not `Result` directly, since
`run`'s own `match` needs a plain `i32` from every arm.

**Loud-failure convention directly relevant to Pitfall 5 (never silently
skip a fetch failure)**, `check_minimum_program_count` (VERIFIED) is the
closest existing analog for "refuse the whole command if a count comes up
short":
```rust
fn check_minimum_program_count(found: usize) -> Result<(), String> {
    if found < MINIMUM_PROGRAM_COUNT {
        return Err(format!(
            "found {found} corpus programs, refusing to write fewer than {MINIMUM_PROGRAM_COUNT}"
        ));
    }
    Ok(())
}
```
`fetch_corpus.rs` needs the same shape: compare the number of files
successfully fetched-and-verified against `corpus/manifest.toml`'s own
declared entry count, and return `Err` (not silently continue) on any
mismatch, per RESEARCH.md Pitfall 5.

**`xtask`'s own `Cargo.toml`, dependency block to extend**
(`crates/xtask/Cargo.toml`, whole file read this session):
```toml
[package]
name = "xtask"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
toml.workspace = true
serde.workspace = true
deform6.workspace = true

[lints]
workspace = true
```
Add `ureq = "3"` and `sha2 = "0.11"` as new, non-workspace `[dependencies]`
entries (RESEARCH.md's own "Installation" section), since neither is a
`[workspace.dependencies]` entry today (root `Cargo.toml` read this session
lists only `deform6`, `object`, `thiserror`, `serde`, `serde_json`, `clap`,
`toml`). The `[lints] workspace = true` block must stay; it is the reason
`xtask` carries the full lint wall.

---

### `corpus/manifest.toml`

**Analog:** `tests/ratios.toml` (the format `ratios::parse_ratios_toml`/
`ratios::format_entry` in `crates/deform6/tests/ratios.rs`, embedded into
`xtask` via `#[path]`, read/writes), the only other pinned, keyed TOML table
this workspace already maintains by a similar xtask-driven process.

**Shape implied by RESEARCH.md's own recommendation** (Open Question 3): a
table keyed by program name, each entry giving a URL and a SHA-256 hex
string, e.g.:
```toml
["some-program-name"]
url = "https://example.invalid/some-program.exe"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
```
`validate_toml` in `crates/xtask/src/main.rs`
(`rendered.parse::<toml::Table>()`) is the existing pattern for proving a
rendered/read TOML file is well-formed before trusting it; `fetch_corpus.rs`
should parse `corpus/manifest.toml` the same way, via the `toml` crate
already a workspace dependency.

---

### `.github/workflows/fuzz.yml`

**Analog:** `.github/workflows/gate.yml` (whole file read this session, the
only existing workflow).

**Shape to copy** (`gate.yml`, VERIFIED, full file):
```yaml
name: gate

on:
  push:
  pull_request:

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always

jobs:
  gate:
    name: fmt, clippy and test
    runs-on: ubuntu-latest

    steps:
      - name: Check out the repository
        uses: actions/checkout@v4

      - name: Install the toolchain that rust-toolchain.toml names
        run: rustup show

      - name: Report the versions
        run: |
          rustc --version
          cargo --version
          cargo fmt --version
          cargo clippy --version

      - name: Cache the cargo registry and the target directory
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry/index
            ~/.cargo/registry/cache
            ~/.cargo/git/db
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('rust-toolchain.toml', 'Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-

      - name: cargo fmt
        run: cargo fmt --all --check

      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: cargo test
        run: cargo test --workspace

      - name: Prove the lint wall
        run: sh scripts/prove-lint-wall.sh

      - name: Prove the Region wall
        run: sh scripts/prove-region-wall.sh
```
Runner: `ubuntu-latest`. Toolchain step: `rustup show`, reading the pin from
`rust-toolchain.toml` (`channel = "1.97.1"`, verified this session), never
edited by `fuzz.yml`. `fuzz.yml` must install nightly as a **second,
additional** toolchain (`rustup toolchain install nightly`), then invoke
every `cargo fuzz` command as `cargo +nightly fuzz ...`, exactly as
RESEARCH.md's Pattern 2 and Anti-Patterns section require. Copy the
`actions/checkout@v4` and `actions/cache@v4` steps verbatim (same action
versions, same cache key shape, with `crates/deform6/fuzz` corpus/artifact
directories added to the cached `path:` list if the plan wants fuzz-run
cache reuse). Two jobs are needed (PR-bounded, success-criterion-1's literal
command; and a separate `cron`-triggered longer run, per RESEARCH.md's
Pitfall 3), `gate.yml` has only one job, so the second job is new shape, not
copied, but should follow the same `runs-on`/`steps` skeleton.

---

### `crates/deform6-cli/src/main.rs` (`--salvage` flag)

**Analog:** same file, existing flag declarations on `Command::Inspect` and
`Command::Extract`.

**Existing per-variant flag convention** (`main.rs:57-118`, VERIFIED, full
`Command` enum):
```rust
#[derive(clap::Subcommand)]
enum Command {
    Inspect {
        input: PathBuf,
        #[arg(long)]
        opcode_table: Option<PathBuf>,
    },
    Extract {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        report: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
}
```
No `#[command(flatten)]` shared-args struct exists anywhere in this
codebase (confirmed by reading the whole file this session). Per
RESEARCH.md Open Question 2's recommendation, add
`#[arg(long)] salvage: bool` independently to both `Inspect` and `Extract`,
matching this established convention, rather than introducing
`#[command(flatten)]` as a new clap idiom for this one flag.

**Exit code table this flag must not disturb** (`main.rs:1-29`, doc comment,
VERIFIED): six exit codes, 0 through 5, `Damaged` = 4, already reserved for
a future `--salvage` route per the doc comment ("`Damaged` has no
`--salvage` route in this phase. `Damaged` is in fact already reachable").
`--salvage` changes which inputs reach `Damaged` (fewer, since Salvage
clamps more `Recoverable` defects) but must not renumber or repurpose any of
the six codes.

---

### `crates/deform6/src/report.rs` (recording a salvage-only assumption)

**Analog:** same file, existing `Confidence`/`ProjectReport.limits` machinery.

**Existing extension points, read in full this session**
(`report.rs:25-90`, `report.rs:330-372`):
```rust
pub struct ProjectReport {
    // ...
    pub limits: Vec<String>,
}

pub struct ReportItem {
    // ...
    pub confidence: Confidence,
}

pub enum Confidence {
    Proven,
    Inferred,
    Unrecoverable,
}

fn build_limits(opcode_table_summary: &str) -> Vec<String> {
    // returns Vec<String>, free-text lines describing what this run did not do
}
```
Three viable extension points already exist for SAF-03 ("marks every
assumption"): append a new free-text line to `limits` inside
`build_limits` (or a new sibling function called the same way), mark the
affected `ReportItem.confidence` as `Confidence::Inferred`, or add a new
typed field. RESEARCH.md's Assumption A2 recommends reusing `limits` or
`Confidence::Inferred` rather than adding a new typed field, to avoid
changing the JSON report schema Phase 4's consumers already depend on. This
pattern map does not adjudicate the choice; the plan must pick one
explicitly and can wire it directly into `build_limits`'s existing call
site (`limits: build_limits(opcode_table_summary)`, `report.rs:415`).

## Shared Patterns

### The `Journal`/`Mode` choke point
**Source:** `crates/deform6/src/journal.rs:1-72` (whole file, unchanged by
this phase, only its callers change).
**Apply to:** every one of the ~24 read/parse/walk functions in `vb/*.rs`,
plus `vb/mod.rs::inspect` and `write/mod.rs::project`.
```rust
pub fn record<T>(&mut self, defect: Defect, fallback: T) -> Result<T, Error> {
    self.defects.push(defect.clone());
    match (defect.kind.severity(), self.mode) {
        (Severity::Fatal, _) => Err(Error::Refused(defect)),
        (Severity::Recoverable, Mode::Strict) => Err(Error::Refused(defect)),
        (Severity::Recoverable, Mode::Salvage) => Ok(fallback),
    }
}
```
Do not re-implement `match mode { ... }` at any call site; every call site
hands its `Defect` and fallback to this one function and propagates the
`Result` with `?`.

### The bound-check-before-allocate pattern
**Source:** `crates/deform6/src/vb/object.rs::bound_proc_count`
(`object.rs:279-306`) and `crates/deform6/src/vb/project.rs::DeclareTable::read`
(cited in RESEARCH.md as a second correct instance).
**Apply to:** every count/length field STRUCTURES.md catalogs that sizes a
`Vec::with_capacity` or a loop bound before this phase, starting with
`gui.rs::GuiTable::walk`'s `header.w_form_count`, and any other site the
05-02 audit finds in the same shape (object counts, external table, property
stream lengths, `.frx` blob lengths, control counts). Do not invent a new
generic `checked_count(len, size) -> Result<u32, Defect>` utility; copy this
function's four-step shape (null/no-region early return, `checked_div`,
compare, build-and-record) at each site.

### CLI per-subcommand flag declaration
**Source:** `crates/deform6-cli/src/main.rs:57-118` (`Command::Inspect`,
`Command::Extract`).
**Apply to:** the new `--salvage` flag, declared independently on both
variants, matching every existing flag's own pattern.

### `xtask` subcommand dispatch
**Source:** `crates/xtask/src/main.rs:73-88` (`run`'s own `match`).
**Apply to:** `fetch-corpus` (and any `fuzz-ci`-named subcommand plan 05-04
adds), as new arms in the same `match`, with a new sibling module file
(`fetch_corpus.rs`) declared with `mod fetch_corpus;` next to the existing
`mod opcode_table;`.

### CI workflow shape
**Source:** `.github/workflows/gate.yml` (whole file).
**Apply to:** `.github/workflows/fuzz.yml`'s `runs-on: ubuntu-latest`,
`actions/checkout@v4`, `actions/cache@v4` steps, and the "one named step per
command" convention (never chaining unrelated commands with `&&` inside one
`run:` block).

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/deform6/fuzz/fuzz_targets/parse.rs` | test (fuzz harness) | request-response | No `#![no_main]`/libFuzzer-shaped file exists anywhere in this tree today; RESEARCH.md's own recommended shape (quoted above, under Pattern Assignments) is the only reference, itself built from calling this crate's own already-shipped `inspect`/`write::project` entry points. |
| `crates/deform6/fuzz/Cargo.toml` | config | n/a | Not hand-written at all; generated by `cargo fuzz init`. No analog needed or wanted. |
| `.github/workflows/fuzz.yml`'s cron-triggered second job | config (CI) | event-driven | `gate.yml` has exactly one job, triggered by `push`/`pull_request` only; no `schedule:`-triggered job exists anywhere in this repository's CI today. The job skeleton (`runs-on`, `steps`) is copied from `gate.yml`'s one job, but the trigger and the `-runs=N` flag choice (RESEARCH.md Pitfall 3) are new. |

## Metadata

**Analog search scope:** `crates/deform6/src/{journal.rs,error.rs,report.rs,vb/*.rs,write/*.rs}`, `crates/deform6/tests/*.rs`, `crates/deform6-cli/src/main.rs`, `crates/xtask/src/*.rs`, `.github/workflows/*.yml`, root and `crates/xtask/Cargo.toml`.
**Files scanned:** 13 read directly this session (with line numbers), plus `git ls-files` run against every analog path to confirm tracked-source status (all confirmed tracked, none a gitignored mirror).
**Pattern extraction date:** 2026-09-13
