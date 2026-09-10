# Roadmap: DeForm6

## Overview

DeForm6 starts as a program that opens a file and ends as a program that writes
a Visual Basic project. Phase 1 makes the file readable and safe to read: the
PE envelope, the VB header, the refusals, and the bounded `Region` type that
every later phase reads bytes through. Phase 2 recovers the names: objects,
procedures, prototypes, and the external `Declare` table, and it builds the
differential harness that measures every phase after it against the original
source. Phase 3 recovers the forms: the control tree, the properties, the
resource blobs, and the event handler names. Phase 4 turns the recovered model
into files on disk and into a JSON report that grades each item. Phase 5 makes
the tool hold under a hostile file. Phase 6 finishes the metadata deliverable
and states its boundary in writing.

The order is fixed. Each phase reads something the phase before it made
reachable.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: It reads the file** - PE parsing, VB6 detection, the VB header, the refusals, and the safety primitives every later phase depends on
- [ ] **Phase 2: The object graph** - Objects, kinds, public procedure names, prototypes, the external table, and the differential harness that measures all of it
- [ ] **Phase 3: Forms** - The control tree, control types and names, property values, the `.frx` blobs, and the event handler names
- [ ] **Phase 4: It writes a project** - `extract` emits `.vbp`, `.frm`, `.frx`, `.bas` and `.cls`, plus the JSON confidence report
- [ ] **Phase 5: Hostility** - `--salvage`, fuzzing in the gate, and the run-time robustness corpus
- [ ] **Phase 6: Version 1.0** - The metadata deliverable is finished, measured, and documented with its limits stated

## Phase Details

### Phase 1: It reads the file

**Goal**: A person runs `deform6 inspect` on a compiled program and learns
whether DeForm6 can read it, what the project is called, and how it was
compiled. A file that is not a VB6 Standard EXE gets one clear sentence and a
non-zero exit code.

**Depends on**: Nothing (first phase)

**Requirements**: DET-01, DET-02, DET-03, DET-04, DET-05, DET-06

**Success Criteria** (what must be TRUE):

  1. `deform6 inspect corpus/vb6-code/Mandelbrot/Mandelbrot.exe` prints the VB
     header fields, the project name and the word `native`, exits 0, and
     changes no file on disk. A directory listing taken before and after the
     run is identical.
  2. A script runs `inspect` over all 44 executables in `corpus/`. All 44
     resolve the entry point to a file offset, reach a `VB5!` signature, name
     `MSVBVM60.DLL` as the imported runtime, and report `native`.
  3. `cargo test --workspace` runs `refusal.rs`. A VB5 executable, a .NET
     executable and an empty file each produce a distinct `Error` variant, not
     a generic one, and each prints one sentence that names the reason.
  4. `cargo clippy --all-targets -- -D warnings` passes with the full deny wall
     in place. A deliberate `d[0]`, a deliberate `a + b` on a file-derived
     value, and a deliberate `.unwrap()` each stop the build.
  5. `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings &&
     cargo test --workspace` passes on a clean clone.

**Named risks**:

  - **The safety primitives cannot be retrofitted.** `#![forbid(unsafe_code)]`,
    the clippy deny wall, the `Region` type with no infallible accessor, and
    the `Off` / `Rva` / `Va` newtypes with no `Add` go in before the first
    parser. A wall added later fires on every line already written, and the
    cheap fix then is to allow the lint.
  - **The overflow trap is a correctness fault, not a crash.** In a release
    build `off + len` wraps silently, the wrapped sum is small, the small sum is
    in bounds, and the parser reports real bytes from the wrong place with
    confidence. `clippy::arithmetic_side_effects` is what makes `checked_add`
    the only shape that compiles.
  - **STRUCTURES gap 1**: the meaning of `VBHeader` 0x58 and 0x5C is disputed
    (§2.3). It blocks the `.vbp` `Title=` and `ExeName32=` keys in Phase 4.
    Resolve it here by diffing the read values against the `.vbp` files in
    `corpus/vb6-code/`.
  - **STRUCTURES gap 18**: the entry-point stub variants `0x5A` and `0x11`
    (§1.3) have no sample. All 44 corpus files use `push imm32` then
    `call rel32`. Do not implement a variant without a file that shows it.
  - **The corpus is 44 native executables.** DET-05 reads
    `ProjectInfo.lpNativeCode` and reports the mode. The P-code value of that
    field is never exercised, because every vendored `.vbp` carries
    `CompilationType=0`. The branch exists and is untested by construction.

**Plans**: 8/8 plans executed

Plans:

- [x] 01-01-PLAN.md - Workspace, toolchain, the lint wall, and the three-command gate in CI
- [x] 01-02-PLAN.md - `read/region.rs` - `Region`, `Off`, `Rva`, `Va`, no dependency, every read returns `Option`
- [x] 01-03-PLAN.md - `error.rs` and `journal.rs` - `Site`, `DefectKind`, `Severity`, `Defect`, `Error`, `Refusal`, `Journal`, `Mode`
- [x] 01-04-PLAN.md - `read/pe.rs` - the PE envelope over `object` 0.40.0, the section table, the one address-to-offset predicate, the import directory
- [x] 01-05-PLAN.md - `vb/header.rs` - the entry stub, the `VB5!` signature, the `VBHeader` fields, and the closed 0x58 / 0x5C gap
- [x] 01-06-PLAN.md - Runtime discrimination and the three refusals - `MSVBVM50.DLL`, `VB40032.DLL`, and no VB runtime at all
- [x] 01-07-PLAN.md - `vb/project.rs` - `ProjectInfo`, the project name, the object count, the compilation mode from `lpNativeCode`, and `inspect`
- [x] 01-08-PLAN.md - The `deform6-cli` shell, the `inspect` subcommand, exit codes, `refusal.rs` and the 44-file corpus sweep

**Waves**: [01-01] then [01-02, 01-03] then [01-04] then [01-05, 01-06] then [01-07] then [01-08]

---

### Phase 2: The object graph

**Goal**: `deform6 inspect` reports every object the program holds, says whether
each one is a form, a module or a class, names every public procedure, prints
each prototype with its argument names, types and modifiers, and prints the
`Declare` statements for external API calls. Nothing is written to disk. A
differential test measures all of it against the original source.

**Depends on**: Phase 1

**Requirements**: OBJ-01, OBJ-02, OBJ-03, OBJ-04, OBJ-05, OBJ-06, VER-01,
VER-02, VER-03, VER-04, VER-05

**Success Criteria** (what must be TRUE):

  1. `cargo test --workspace` runs `differential.rs` over all 44 corpus
     programs. Every object that the program's `.vbp` declares is recovered by
     name and by kind, 44 of 44.
  2. The harness takes its expectation from the file list the `.vbp` declares,
     never from a directory glob. Adding an orphan `.cls` file to a corpus
     project directory does not change any result. Where a directory holds
     several `.vbp` files, the harness selects the one whose `ExeName32` names
     the executable under test, and the two corpus projects that need this
     pass.
  3. `tests/ratios.toml` holds two counts and a rounded ratio per program,
     counted over **procedures**, not objects. Editing one pinned number
     **up** makes the test fail with the word `REGRESSION` and a list of the
     items that went missing, because the tool now recovers less than the pin
     claims. Editing it **down** makes the test fail with the words `MOVED UP`
     and prints the exact TOML block to paste, because the tool now recovers
     more than the pin claims. `cargo run -p xtask -- update-ratios` rewrites
     the file.

     Object recovery is not pinned as a ratio. It is 44 of 44 with no variance,
     so a pin on it could never move and therefore could never fail. It is
     asserted as an equality instead, in both directions.
  4. `inspect` prints a public procedure prototype with argument names, argument
     types, and the ByRef, Array, Optional and ParamArray modifiers. It prints
     a private procedure as `Private` with no name, because the name array
     holds a null there.
  5. `inspect` prints one line per entry in the external import table, giving
     the library name and the export name that the file holds.

     It does **not** print an alias or an argument list, and it says so with a
     marker rather than omitting them silently. `STRUCTURES.md` §7.2 records at
     confidence [C] that neither survives compilation, so the earlier wording
     of this criterion promised something no VB6 binary contains. A reader must
     be able to tell "this file does not hold it" from "DeForm6 did not recover
     it".

**Named risks**:

  - **A `.bas` code module carries no type descriptors.** The type data is part
    of the `IDispatch` plumbing, and a standard module is not a COM object.
    OBJ-04 is therefore reachable for forms, classes and user controls and is
    not reachable for a procedure in a `.bas`. This caps the procedure ratio.
    State the cap in the report; do not discover it when a ratio looks low.
  - **STRUCTURES gap 2**: the `OptionalObjectInfo` presence test is disputed
    (§5.5). Both stated tests are wrong against the only value table that
    exists. Use `fObjectType & 0x2`, cross-check it against
    `ObjectInfo.lpPrivateObject != -1`, and report a disagreement rather than
    picking one.
  - **STRUCTURES gap 3**: the MDIForm `fObjectType` value is in no source. An
    object with an unknown type value must be classified `Unknown` and flagged,
    never refused.
  - **STRUCTURES gap 4**: the `ParamArray` type encoding is unknown (§6.5), and
    OBJ-04 names ParamArray directly.
  - **STRUCTURES gap 5**: fifteen type codes are unassigned (§6.5). Full
    prototype coverage is not reachable until they are settled. An unknown code
    becomes a reported gap, not a guessed type name.
  - **STRUCTURES gaps 6, 7, 8**: the `FuncTypDesc` bytes at 0x06 to 0x0B, the
    `optionalVals` target for a default value, and the `PubVarDesc` record
    stride are all unresolved. Validate `constFFFF == 0xFFFF` against a real
    file before trusting the layout.
  - **STRUCTURES gap 9**: event name strings come from a positional heuristic
    after `ProcNamesArray`. Mark every one `inferred`.
  - **STRUCTURES gap 10**: the ordinal `Declare` encoding for `Alias "#123"` is
    unproved.
  - **The harness must not share code with the thing it tests.**
    `support/vbp.rs` is a second, independent reader. It must not call anything
    in `src/`. A harness that shares a reader agrees with a bug in that reader.

**Plans**: 4/10 plans executed

Plans:

- [x] 02-01-PLAN.md - `ObjectTable` and the `Object` array - every object recovered by name
- [x] 02-02-PLAN.md - `fObjectType` classification - form, module, class, and `Unknown` with a fallback
- [x] 02-03-PLAN.md - `ObjectInfo` and `PrivateObj` - public procedure names, and a null name reported as private
- [x] 02-04-PLAN.md - `FuncTypDesc` and the type code table - prototypes with argument names, types and modifiers
- [x] 02-05-PLAN.md - `PubVarDesc` and `EventDesc` - the two counts explained and the event descriptor addresses
- [x] 02-06-PLAN.md - The `Declare` import table and the external component table
- [x] 02-07-PLAN.md - Harness support - `support/vbp.rs`, an independent reader, and `support/rules.rs`, the exclusion rules
- [x] 02-08-PLAN.md - `differential.rs` - the gate over all 44 corpus programs
- [x] 02-09-PLAN.md - `ratios.toml`, the exact pin, the two failure messages, and `xtask update-ratios`
- [x] 02-10-PLAN.md - `inspect` reports the object graph

**Waves**: [02-01, 02-06, 02-07] then [02-02, 02-03] then [02-04, 02-05] then [02-08] then [02-09, 02-10]

---

### Phase 3: Forms

**Goal**: `deform6 inspect` reports the control tree of every form with the
correct parent for each control, the type and the name of each control, the
property values of each control and of the form, the CLSID of each third-party
OCX control, the resource blobs, and the event handler names bound to each
control.

**Depends on**: Phase 2

**Requirements**: FRM-01, FRM-02, FRM-03, FRM-04, FRM-05, FRM-06, VER-06

**Success Criteria** (what must be TRUE):

  1. `inspect` prints the control tree of every corpus form with the parent of
     each control. On all 44 programs the sum of the control block `Length`
     fields tiles `GUIObjectInfo.lPropertiesLength` exactly. Where the tiling
     check fails, the run refuses and names the byte offset. It never prints a
     tree it cannot prove.
  2. `cargo test --workspace` compares the recovered control tree, control
     types, control names and property values against the committed `.frm`
     files through `support/frm.rs`, a second independent reader. The pinned
     form ratio and the pinned control ratio hold.
  3. A form that uses a third-party OCX reports the control's CLSID, resolved
     by matching the class name against the external component table, and
     states in plain words that the property blob is not interpretable without
     that control's own type library.
  4. Every string property read lands the cursor exactly on the declared field
     end. A property whose read does not land there is reported as
     unrecoverable with its byte offset. No string read decides how far the
     cursor advances.
  5. `corpus/vb6-code/Hidden-Markov-model/frmHMM.frx` is excluded by name and
     the reason is recorded next to the exclusion. Removing the exclusion makes
     the gate fail with a named message, not pass silently.

**Named risks**:

  - **STRUCTURES gap 15**: no public opcode-to-property table exists. The
    opcode space is per control type, and the same opcode 31 means `Appearance`
    on a CommandButton, `BackStyle` on a Label and `DrawMode` on a Form. Semi VB
    Decompiler reads `VB6.OLB` over COM at run time, which DeForm6 cannot do
    and has no right to redistribute. The table must be built once from a type
    library dump and committed as derived data. This is the largest single
    unknown in the phase, and it is a data build job, not a parsing job.
  - **STRUCTURES gap 14**: the scope separator grammar (§8.9) is the least
    certain part of the whole format. It is `0xFF` followed by a run of scope
    bytes. Do not model it as a stack machine driven by guesses. Use the
    `lPropertiesLength` tiling check as a hard gate and refuse rather than emit
    a mis-nested tree. Validate menus against the `fMdlIntCtls` Menu bit and a
    `cType == 19` count.
  - **STRUCTURES gap 13**: the string encoding rule in the form property stream
    (§9.3) is not known. The one public heuristic is unsound and its own byte
    accounting proves it. `VbStr` takes an explicit encoding from the property
    table, defaults to ASCII, validates the landing point, retries once as the
    other encoding, and then refuses. It never guesses.
  - **STRUCTURES gap 12**: the byte at control block offset 0x02, named `uni`,
    may carry the encoding flag for the block. Test it before relying on it.
  - **STRUCTURES gap 11**: the control array index location is unproved, and
    FRM-02 needs it to write `Index = N`.
  - **The position block escape.** If the first `i16` of a position block is
    `-32768`, the block is instead four `i32` starting two bytes later, that is
    16 bytes and not 8. A parser that misses this produces silently wrong
    geometry for every control after it.
  - **The event name table.** The event slot index is the event's ordinal in
    the control's default source interface, and there is no name in the file.
    This is a second derived data table, per control type.
  - **STRUCTURES gap 16**: nine dwords in `GUIObjectInfo` at 0x35 to 0x58 have
    no known meaning. Leave them opaque; do not invent a reading.

**Plans**: 3/10 plans executed

Plans:
**Wave 1**

- [x] 03-01-PLAN.md - The GUI table, `GUIObjectInfo`, the `lPropertiesLength` tiling invariant, the end to end tracer, and the module set
- [x] 03-03-PLAN.md - `support/frm.rs`, the independent `.frm` reader, and the `frmHMM.frx` exclusion by name

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03-02-PLAN.md - `OpcodeTable`, the never-committed derived table, the `--opcode-table` flag, and the safe-provenance subset
- [ ] 03-04-PLAN.md - Scope separators, the control tree, control types, control names, and the control array index at offset `0x05`
- [ ] 03-05-PLAN.md - `VbStr` - the encoding parameter, the landing point validation, and the refusal

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 03-06-PLAN.md - The property stream reader - typed payloads, the position block escape, the `Font` block, the special opcodes
- [ ] 03-08-PLAN.md - External OCX controls - `cType 255`, the class name, the CLSID join, `_ExtentX` and `_ExtentY`, the opaque blob
- [ ] 03-09-PLAN.md - `ControlInfo` and the event handler table - event handler names and the honest report for a slot with no name

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 03-07-PLAN.md - `.frx` blob extraction from the inline property stream, with image format detection

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 03-10-PLAN.md - `inspect` reports the form tree, and the differential gate extends to forms and controls

**Waves**: [03-01, 03-03] then [03-02, 03-04, 03-05] then [03-06, 03-08, 03-09] then [03-07] then [03-10]

Plan 03-02 moves from wave 1 to wave 2. Plan 03-01 owns
`crates/deform6/src/vb/mod.rs` for the whole phase and creates all eight new
module files, which is the pattern `crates/deform6/src/lib.rs` states in its
own doc comment: the modules are declared as a set, so that two plans in one
wave never edit this file. 03-02 edits `vb/opcodes.rs`, a file 03-01 creates,
so it cannot run in the same wave.

Plan 03-02 also withdraws the "committed as derived data" instruction in the
gap 15 named risk below, per `03-CONTEXT.md` D-01. `AGENTS.md` bars a fixture
calculated from a third party file, so the tool is committed and the table
never is.

---

### Phase 4: It writes a project

**Goal**: `deform6 extract <exe> -o <dir>` writes a Visual Basic project
directory that the VB6 IDE can open, and one JSON report beside it that grades
each recovered item by confidence and names the bytes it came from.

**Depends on**: Phase 3

**Requirements**: WRT-01, WRT-02, WRT-03, WRT-04, WRT-05, WRT-06, WRT-07,
RPT-01, RPT-02, RPT-03, RPT-04, RPT-05, RPT-06

**Success Criteria** (what must be TRUE):

  1. `deform6 extract <exe> -o out/` on each of the 44 corpus programs writes
     one `.vbp`, one `.frm` per form, one `.frx` per form that holds a blob, one
     `.bas` or `.cls` per module and class, and one JSON report. The run exits
     0 and writes nothing outside `out/`.
  2. A check over the written tree asserts that every text file holds
     Windows-1252 bytes with CRLF on every line, including the last, and no
     byte order mark.
  3. Every `.frx` offset written into a `.frm` resolves inside the `.frx` that
     was actually written. The check reads each written `.frm` back through
     `support/frm.rs`, seeks each offset in the matching `.frx`, and lands on a
     valid record header. This passes for all 44 programs.
  4. The structural check passes: every `Form=`, `Module=` and `Class=` line
     names a file that exists, `Startup=` names a form a `Form=` line brings
     in, every control and class name is a legal VB6 identifier of 40
     characters or fewer, nesting depth is 7 or less, properties inside a block
     are in case-insensitive alphabetical order, and every menu comes after
     every other control.
  5. `jq '.items[] | select(.confidence=="inferred") | .path' report.json`
     returns paths. Every item carries a `basis` and at least one `evidence`
     record with a byte offset. `confidence` is one of exactly three words and
     never a number. Two runs on the same input produce byte-identical reports.

**Named risks**:

  - **The `.frx` offset is not stored in the executable.** It is a running
    cursor that starts at 0 for each form and advances by `blobLen + 12` after
    each blob. The `.frm` writer and the `.frx` writer are therefore one
    component. They cannot be split across two plans or two phases, and any
    offset computed independently of that cursor drifts. Plan 04-04 owns both.
  - **The silent truncation class.** The IDE cuts a control or class name
    longer than 40 characters to 40 and still loads the form. The code that
    names the full identifier then fails to compile, and the cause sits in a
    `.log` file the user may never open. The emitter clamps every name itself
    and records the clamp in the report.
  - **Full recompilation cannot run in this CI.** It needs VB6 on Windows. The
    structural check is what runs. Say that in the report and in the README.
    Do not imply that the IDE opened the project.
  - **FILE-FORMATS gap 1**: the inline string threshold is bounded between 99
    and 153 characters and is not fixed. Safe default: write a string inline
    only at 97 characters or fewer with no line break, which is inside the
    proved range.
  - **FILE-FORMATS gap 2**: the short string record header width is
    reconstructed arithmetic, not a second observation. Safe default: always
    write the `$` form with a u32 count, which one corpus file proves exactly.
  - **FILE-FORMATS gap 5**: no corpus `ComboBox` or `ListBox` stores its `List`
    in a `.frx`, so the list record layout is unverified.
  - **FILE-FORMATS gap 7**: every corpus file is Western. A Japanese or Cyrillic
    project uses a different code page and nothing in the file says which. How
    DeForm6 chooses is an open design question, and the choice goes in the
    report.
  - **FILE-FORMATS gap 9**: MDIForm is not in the corpus. `VB.MDIForm` and
    `MDIChild = -1  'True` are unverified.
  - **STRUCTURES gap 1 lands here.** The `.vbp` `Title=` and `ExeName32=` keys
    depend on the disputed `VBHeader` 0x58 and 0x5C reading from Phase 1.

**Plans**: 9 plans

Plans:

- [ ] 04-01: `model.rs`, the recovered project model, the Windows-1252 and CRLF emitter primitives, the 40-character clamp, the identifier check
- [ ] 04-02: Property value serialisation - integers, strings, booleans, enumerations, colours, floats, the `Font` block, blob references
- [ ] 04-03: `write/vbp.rs` - component lines, the setting block order, `Startup=`, no stray `ResFile32=`
- [ ] 04-04: `write/frm.rs` - the `.frm` and the `.frx` as one component driven by one cursor. Not splittable
- [ ] 04-05: `write/code.rs` - `.bas` and `.cls` with the attribute preamble, and an empty procedure body with the correct signature
- [ ] 04-06: `report.rs` - the flat item array, the path key, the three confidence words, `basis` and `evidence`, the defect array, deterministic output
- [ ] 04-07: The uncertainty comment emitter, in code regions only, never in a `Begin` block and never in the `.vbp`
- [ ] 04-08: The `extract` subcommand - `-o`, `--report`, `--force`, exit codes
- [ ] 04-09: The structural recompilation check in the harness, and the ratios re-pinned

**Waves**: [04-01, 04-06] then [04-02, 04-03, 04-05] then [04-04, 04-07] then [04-08] then [04-09]

---

### Phase 5: Hostility

**Goal**: DeForm6 holds under a file that is damaged or hand-made to break a
parser. It refuses by default and names the byte offset. It recovers what it
can under `--salvage` and marks every assumption. A fuzzer runs in the gate,
and every crash it finds becomes a committed test that replays on stable Rust.

**Depends on**: Phase 4

**Requirements**: SAF-01, SAF-02, SAF-03, SAF-04, SAF-05

**Success Criteria** (what must be TRUE):

  1. `cargo +nightly fuzz run --fuzz-dir crates/deform6/fuzz parse --
     -max_total_time=60 -rss_limit_mb=2048` finds no crash, and the job runs on
     every pull request. The fuzz crate builds only on nightly and stays out of
     `cargo test --workspace` and `cargo clippy --all-targets`.
  2. `cargo test --workspace` replays every file in `tests/regressions/`
     through both `Strict` and `Salvage` on stable and fails with the file name
     if one panics. Emptying the directory makes the test fail, because the
     count assertion refuses a loop that proves nothing.
  3. A truncated corpus file refused in strict mode prints the byte offset and
     what the parser expected to find there. The same file under `--salvage`
     produces output, and the report lists every assumption the run had to
     make. The defect list is the same in both modes.
  4. Every count and every length field named in the parse order is checked
     against the real file size before any allocation is sized from it. A
     hand-made file that declares `wFormCount = 0xFFFF` in a 4 KB image is
     refused with `ImplausibleCount` and allocates nothing.
  5. The whole vendored corpus, the fetched run-time set and every regression
     input run through both modes with `panic = "abort"` set in the release
     profile, and no process aborts. The run prints the number of inputs it
     read, and that number equals the number of files that exist.

**Named risks**:

  - **`cargo fuzz` needs nightly, and the isolation needs two statements, not
    one.** Pass `--fuzzing-workspace true`, because the flag defaults to
    `false`, and also put `exclude = ["crates/deform6/fuzz"]` in the root
    `Cargo.toml`. Either alone lets a stable `cargo clippy --all-targets` try
    to build a `#![no_main]` crate and break the gate.
  - **`--fuzz-dir` is needed on every `cargo fuzz` call**, because the fuzz
    crate is not at the repository root. Forgetting it creates a second fuzz
    directory. Put the call in an `xtask` command so nobody types it by hand.
  - **A wall clock bound is a flaky gate on a varying CI machine.** `-runs=N` is
    the deterministic alternative. Choose one deliberately.
  - **`-rss_limit_mb` is the check for an allocation sized from a length
    field.** That is a named project constraint, so set the value explicitly
    rather than take the default.
  - **The run-time corpus can disappear.** The manifest pins a SHA-256 for each
    program but cannot pin availability. A fetch that fails must fail loudly.
    A silent skip turns the robustness set into a set of zero files that
    passes.
  - **Salvage reaches code that strict refuses before it gets there.** That
    code is the least exercised in the crate, so the fuzz target must call both
    modes, not one.

**Plans**: 8 plans

Plans:

- [ ] 05-01: The strict and salvage split wired end to end, `--salvage` on both subcommands, every assumption recorded in the report
- [ ] 05-02: The bound check audit - every count and length field from the parse order, checked against the real file size
- [ ] 05-03: The fuzz crate - `cargo fuzz init` with the workspace flag, the `exclude` line, and a target that calls both modes
- [ ] 05-04: The CI fuzz jobs - the bounded pull request run, the longer cron run, and corpus seeding from `corpus/`
- [ ] 05-05: `regressions.rs`, the stable replay, the count assertion, and the written crash-to-test procedure
- [ ] 05-06: The stable fuzz smoke test - a fixed seed mutation of corpus files, in the normal gate, no dependency
- [ ] 05-07: `corpus/manifest.toml` with pinned hashes, `xtask fetch-corpus`, and `corpus/fetched/` kept out of the repository
- [ ] 05-08: The no-panic proof run over the vendored corpus, the fetched set, and every regression input

**Waves**: [05-01, 05-02, 05-03, 05-07] then [05-04, 05-05, 05-06] then [05-08]

---

### Phase 6: Version 1.0

**Goal**: The metadata deliverable is finished, measured and documented. A
person who reads the README learns exactly what DeForm6 returns and exactly
what it does not, before they run it. There is no code recovery in version 1.0
and no claim of it anywhere in the program, the help text or the documentation.

**Depends on**: Phase 5

**Requirements**: None new. This phase re-verifies DET, OBJ, FRM, WRT, RPT, SAF
and VER end to end and turns the measured result into the released
documentation.

**Success Criteria** (what must be TRUE):

  1. `deform6 extract` runs to completion on all 44 corpus programs, the
     structural check passes for all 44, the JSON report validates against the
     schema for all 44, and every pinned ratio in `tests/ratios.toml` holds
     unchanged.
  2. A grep over `README.md`, the `--help` output and the report vocabulary
     finds no claim of statement recovery, no claim of compilable Basic from
     native code, and no percentage figure outside the measured
     `recovery.ratio`.
  3. The README states three facts in plain words: the P-code branch of
     `lpNativeCode` is untested because every corpus program is native; the
     `frmHMM.frx` file is damaged upstream and is excluded by name; and full
     recompilation was not tested, because it needs VB6 on Windows.
  4. The README lists every gap from STRUCTURES section 11 and FILE-FORMATS
     section 9 that is still open at release, with the safe default the tool
     chose for each one.
  5. `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings &&
     cargo test --workspace` passes on a clean clone, `cargo doc --no-deps`
     produces no warning, and the tag matches the version in `Cargo.toml`.

**Named risks**:

  - **The temptation to state a percentage.** A commercial tool in this field
    advertises "85% recovery" with no evidence behind it. DeForm6 has three
    named confidence words and one ratio measured against the original source.
    Do not copy the habit that the project was written to replace.
  - **The P-code branch is untested by construction.** Every one of the 44
    vendored programs carries `CompilationType=0`. Saying "supports native and
    P-code" would be a claim with no measurement behind it. Say what was run.
  - **A gap that is still open at release is a documented limit, not a silent
    one.** An open gap that nobody wrote down becomes a wrong answer stated
    confidently, which is worse than no answer.
  - **The README is the last place a claim can drift.** It is written after the
    measurement, from the measurement, and not from the plan.

**Plans**: 5 plans

Plans:

- [ ] 06-01: The README - what it returns, what it does not, and the confidence vocabulary defined
- [ ] 06-02: The public library API review, the doc comments, and a clean `cargo doc --no-deps`
- [ ] 06-03: The end-to-end acceptance run over all 44 corpus programs, with the numbers recorded
- [ ] 06-04: The honesty audit - every claim checked against a measured result, and every open gap listed as a known limit
- [ ] 06-05: Release mechanics - the version, the changelog, the licence check, `.gitattributes` for `*.frx` and `*.ctx`, the tag

**Waves**: [06-01, 06-02, 06-05] then [06-03] then [06-04]

---

## Requirement coverage

Every v1 requirement maps to exactly one phase. There are 42 requirement IDs in
7 categories.

| Category | IDs | Count | Phase |
|----------|-----|-------|-------|
| Identification | DET-01 to DET-06 | 6 | 1 |
| Object graph | OBJ-01 to OBJ-06 | 6 | 2 |
| Verification | VER-01 to VER-05 | 5 | 2 |
| Forms | FRM-01 to FRM-06 | 6 | 3 |
| Verification | VER-06 | 1 | 3 |
| Written output | WRT-01 to WRT-07 | 7 | 4 |
| The report | RPT-01 to RPT-06 | 6 | 4 |
| Hostile input | SAF-01 to SAF-05 | 5 | 5 |

Mapped: 42 of 42. Orphans: none. Duplicates: none.

**Two notes on the mapping.**

VER-05 pins the recovery ratio. Phase 2 introduces the file and the two failure
messages. Phase 3 and Phase 4 each raise the pinned numbers, which is a change
to an existing requirement's data and not a second phase owning the
requirement.

SAF-01 and SAF-04 are proved in Phase 5, and the mechanism that makes them
possible is built in Phase 1. The lint wall, `#![forbid(unsafe_code)]` and the
`Region` type go in before the first parser. The fuzzer that proves the result
runs in Phase 5. One requirement, one phase, two places where the work shows.

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. It reads the file | 7/8 | In Progress|  |
| 2. The object graph | 4/10 | In Progress|  |
| 3. Forms | 3/10 | In Progress|  |
| 4. It writes a project | 0/9 | Not started | - |
| 5. Hostility | 0/8 | Not started | - |
| 6. Version 1.0 | 0/5 | Not started | - |
