---
phase: 01-it-reads-the-file
plan: 07
subsystem: vb
tags: [projectinfo, objecttable, compile-mode, object-count, inspect, report, det-05, det-06]
status: complete

requires:
  - Off, Region, Rva and Va from plan 01-02
  - Refusal, Defect, DefectKind, Site and Severity from plan 01-03
  - PeImage, region_at_va, image_base, sections, dll_name_sites from plan 01-04
  - header_region and VbHeader, with lp_project_data as a Va, from plan 01-05
  - runtime_of, which returns the matched runtime name, from plan 01-06
provides:
  - ProjectInfo and ProjectInfo::read, with a 0x23C byte window
  - CompileMode, decided by lpNativeCode and by nothing else
  - ObjectTableHead and ObjectTableHead::read, with an 0x54 byte window
  - the project name, read from the object table as a virtual address
  - the object count, read from wTotalObjects
  - Report and inspect, re-exported from the crate root
affects:
  - 01-08, whose command line prints the Report and whose sweep reads
    native, project_name, runtime_dll and signature
  - Phase 2, which walks the object array and must loop on the count and
    bound itself by the capacity
  - Phase 4, which writes ExeName32 from exe_name plus the literal ".exe"
  - Phase 5, whose fuzz target enters through inspect

tech_stack:
  added: []
  patterns:
    - a structure window is taken before any field inside it is read, so a
      truncated file is refused at the window and not after three plausible
      reads
    - a fixture resolves its target through the parser's own path and asserts
      that the write changes something
    - a recommendation carried at low confidence from prior-art code is
      measured against the original source before it is implemented
    - a branch that no sample exercises names its own limit in the test name,
      so the limit is visible in the test list without running anything

key_files:
  created: []
  modified:
    - crates/deform6/src/vb/project.rs
    - crates/deform6/src/vb/mod.rs
    - crates/deform6/src/lib.rs
    - .planning/research/STRUCTURES.md

decisions:
  - The object count comes from wTotalObjects at 0x2A and not from
    wCompiledObjects at 0x2C. Measured against the .vbp of all 44 corpus
    programs, wTotalObjects is the declared object count in 44 of 44 and
    wCompiledObjects in 29 of 44. This contradicts the plan, D-05 and
    STRUCTURES.md section 4, and the measurement is in section 4.1.
  - A count disagreement is reported only when the capacity is below the
    count. Reporting inequality would mark 15 of the 44 corpus files damaged.
  - ObjectTableHead carries its defects and inspect drops them, which is the
    shape PeImage::defects already has. The Journal is not used, because its
    strict policy refuses on a recoverable defect.
  - The truncated ProjectInfo test cuts the file after the third field and
    before the last two, so the window is the thing the test measures.

metrics:
  duration: 1 session
  completed: 2026-09-07

actuals:
  tokens: 61000
  tasks: 3
  commits: 5
plan_head_before: c8db8a06ee1a935f32e8495ddf04248c2b652146
---

# Phase 01 Plan 07: ProjectInfo, the object table and inspect Summary

`vb/project.rs` reads `ProjectInfo` and the head of the object table through
two windows that are taken before any field inside them, and `inspect` in
`vb/mod.rs` turns the six files this phase built into one function that takes
a byte slice and returns a `Report`.

**The P-code branch was exercised by zeroing a field in a copy of a native
program held in memory, and no P-code program exists in this repository.**
All 44 vendored projects carry `CompilationType=0`, which is native, and
`lpNativeCode` is non-zero in all 44 executables, so nothing here shows that
DeForm6 reads a real P-code binary.

## What this plan built

| Item | What it gives |
|---|---|
| `ProjectInfo` | five fields: `dw_version`, `lp_object_table`, `lp_native_code`, `lp_external_table`, `dw_external_count` |
| `ProjectInfo::read` | `Result<ProjectInfo, Refusal>`, through a `0x23C` byte window |
| `ProjectInfo::mode` | `CompileMode::Native` or `CompileMode::PCode` |
| `ObjectTableHead` | `w_total_objects`, `w_compiled_objects`, `lpsz_project_name`, `project_name`, and its defects |
| `ObjectTableHead::read` | `Result<ObjectTableHead, Refusal>`, through an `0x54` byte window |
| `ObjectTableHead::object_count` | the number of objects the project declares |
| `Report` | 13 fields, deriving `Clone, Debug, PartialEq, Eq` |
| `inspect` | `pub fn inspect(data: &[u8]) -> Result<Report, Refusal>` |

`crates/deform6/src/vb/project.rs` is 673 lines with 12 tests.
`crates/deform6/src/vb/mod.rs` is 320 lines with 6 tests. `lib.rs` gained one
`pub use` line. Neither file names a file system type and neither names the
`object` crate.

## The corpus facts this plan measured for itself

A script walked the whole pointer chain on all 44 corpus executables before
any code was written, and then compared the result against the `.vbp` that
declares each executable, selected by its `ExeName32` key. These are its
numbers.

| Claim | Result |
|---|---|
| `ProjectInfo + 0x20` is non-zero | 44 of 44, all native |
| The `ProjectInfo` window of `0x23C` bytes fits inside its section | 44 of 44, smallest margin 1612 bytes |
| The object table window of `0x54` bytes fits inside its section | 44 of 44, smallest margin 2160 bytes |
| `ObjectTable + 0x40` gives a readable project name | 44 of 44 |
| That name equals the string at `VBHeader + 0x64` | 44 of 44 |
| `wTotalObjects` equals the object count the `.vbp` declares | **44 of 44** |
| `wCompiledObjects` equals the object count the `.vbp` declares | **29 of 44** |
| `wObjectsInUse` equals `wTotalObjects` | 44 of 44 |

Every figure the prompt gave held. The last three are new, and the second to
last is the reason this plan does not do what its own task 2 says. See defect 1.

`Mandelbrot.exe`: `lpProjectData` `0x401814` at file offset `0x1814`,
`lpObjectTable` `0x401A50` at `0x1A50`, `lpNativeCode` `0x405000`,
`lpszProjectName` `0x401B04` at `0x1B04` giving `Mandelbrot_Fractal_Demo`, and
one object.

`inspect` was then run over all 44 executables in a scratch integration test,
which was deleted before the first commit because `crates/deform6/tests/`
belongs to plan 01-08. All 44 returned `Ok`, all 44 reported `native`, all 44
named `MSVBVM60.DLL`, and every one of the 44 object counts equals the number
its `.vbp` declares. The empty `tests/` directory that the scratch file needed
was removed as well, so the tree this plan leaves is the tree it found.

## The gate

Run on the committed tree at `5fc9144`, with a clean working tree.

| Command | Exit |
|---|---|
| `cargo fmt --all --check` | 0, no output |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0 |
| `sh scripts/prove-lint-wall.sh` | 0 |
| `sh scripts/prove-region-wall.sh` | 0 |

```
$ cargo test --workspace
     Running unittests src/lib.rs (target/debug/deps/deform6-ef936274b9c3bf51)
test result: ok. 115 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running unittests src/main.rs (target/debug/deps/deform6-31e29b1143ef8fee)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   Doc-tests deform6
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

115 library tests. The phase held 97 before this plan, so all 18 are new and
none was displaced.

```
$ sh scripts/prove-lint-wall.sh          # exit 0
The wall stops every bad shape, and the tree it leaves behind is clean.

$ sh scripts/prove-region-wall.sh        # exit 0
PASS  E0616  reaching the bytes of a Region directly
PASS  E0369  adding two Off values with the plus operator
PASS  E0608  indexing a Region with square brackets
Checked 3 shapes.
The type refuses every shape, and the tree it leaves behind is clean.
```

The acceptance commands the plan names were each run.

```
$ grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/            # exit 1
$ grep -rnE 'lp_object_array|lpObjectArray' crates/deform6/src/        # exit 1
$ cargo test -p deform6 --lib -- --list | grep -c 'no_corpus_program_is_p_code'
1
$ grep -rlE '\bobject::|use object' crates/deform6/src/ --include='*.rs'
crates/deform6/src/read/pe.rs
$ git status --porcelain corpus/                                       # empty
```

The last one matters: seven fixtures in this plan patch or truncate the corpus
bytes, and every one of them works on a `Vec<u8>` copy. Nothing was written to
disk.

## The `Report` this phase produces

Printed from the scratch run, before that file was deleted. Plan 01-08 pins
the shape of the printed output and not these values, per RESEARCH.md
pitfall 6.

```
Report {
    file_len: 28672,
    section_count: 3,
    runtime: Vb6,
    runtime_dll: "MSVBVM60.DLL",
    signature: [86, 66, 53, 33],
    header_offset: Off(5984),
    runtime_build: 9782,
    project_name: "Mandelbrot_Fractal_Demo",
    title: "Mandelbrot Fractal Demo",
    exe_name: "Mandelbrot",
    help_file: "",
    native: true,
    object_count: 1,
}
```

`5984` is `0x1760` and `9782` is `0x2636`, which are the two values CONTEXT.md
records for this file. `[86, 66, 53, 33]` is `VB5!`.

## The deliberate breakages

`AGENTS.md` says to break the thing a test covers and watch it fail. Seven
breakages, seven runs. Every one was reverted and the gate was green before
each commit. No commit holds a broken state.

The plan asks for six. The seventh is mine, and it is the instrument for the
correction in defect 1.

### 1. The compilation mode is inverted

The plan asks for this one. `mode()` returned `PCode` for a non-zero
`lpNativeCode`.

**Two tests failed. 100 passed.**

```
---- vb::project::tests::the_corpus_file_is_native_because_its_native_code_address_is_not_zero ----
  left: PCode
 right: Native

---- vb::project::tests::zeroing_lp_native_code_reports_p_code_but_no_corpus_program_is_p_code ----
  left: Native
 right: PCode
```

Both directions of the branch have an instrument, which is the point of the
patched fixture. Without it only the first test exists, and a build that
reported every file as P-code would fail one test instead of two.

### 2. The `ProjectInfo` window is removed

The plan asks for this one. `subregion(Off::new(0), PROJECT_INFO_SIZE)` was
replaced by the section-length region.

**One test failed. 101 passed.**

```
---- vb::project::tests::a_file_that_ends_inside_project_info_is_damaged_and_is_not_read_in_part ----
  left: Damaged("ProjectInfo holds no address for the import table")
 right: Damaged("the file ends inside the ProjectInfo structure")
```

The failure text is the whole argument for the window. Without it the parser
reports a template version, an object table address and a native code address
out of a file that ends 316 bytes into a 572 byte structure, and only fails at
`0x234`. That is the partial read the window exists to stop.

### 3. The project name pointer is read from `0x3C`

The plan asks for this one.

**Five tests failed. 104 passed.**

```
---- vb::project::tests::the_project_name_comes_from_the_object_table ----
called `Result::unwrap()` on an `Err` value:
Damaged("the project name pointer is in no section")
```

`0x3C` is `lpIdeData2` and it holds `0` in this file, so the read refuses
rather than reporting a plausible string from the wrong place. That is the
same result plan 01-05 measured for the header offsets: in this crate a
pointer read from the wrong slot produces a refusal, not a wrong answer.
`Va::to_rva` is a checked subtraction, so the null never reaches the section
table.

### 4. A count disagreement refuses the file

The plan asks for this one.

**One test failed. 108 passed.**

```
---- vb::project::tests::a_capacity_below_the_object_count_is_a_recoverable_defect_and_not_a_refusal ----
a count disagreement must not refuse the file:
Damaged("BREAKAGE: the two object counts disagree")
```

### 5. The object count is taken from `wCompiledObjects`

**Not in the plan. This is the plan's own instruction, run as a breakage.**
`object_count()` returned `w_compiled_objects`.

**One test failed. 108 passed.**

```
---- vb::project::tests::a_capacity_above_the_object_count_is_normal_and_is_not_a_defect ----
assertion `left == right` failed: the reported count must be the number of
objects the project declares, and not the capacity the compiler rounded the
array up to
  left: 4
 right: 1
```

This breakage exists because defect 1 changes what the code does, and a
correction with no instrument is not a correction. `Mandelbrot.exe` alone
cannot see it, because both of its count fields hold 1. The fixture that
raises the capacity to 4 is the only test in the module that can.

### 6. The runtime decision moves after the header read

The plan asks for this one, and predicts the failure exactly.

**One test failed. 114 passed.**

```
---- vb::tests::a_visual_basic_5_file_with_no_signature_is_refused_by_name_and_not_as_damaged ----
  left: Err(Damaged("the header does not begin with VB5!"))
 right: Err(IsVb5)
```

A person holding a Visual Basic 5 file would be told their file is damaged.
It is not damaged. It is the wrong version, and DET-04 asks for it to be
refused by name.

### 7. `runtime_dll` is filled from the constant instead of from the file

**Not in the plan for this task. Plan 01-08 task 3 predicts this outcome and
asks for it to be recorded, so it was run here, where the field is built.**

`runtime_dll` was filled with `runtime::VB6_DLL.to_owned()`.

**No test failed. 115 passed.** The gate still caught it, and not through a
test:

```
$ cargo clippy --all-targets -- -D warnings
error: unused variable: `runtime_dll`
error: could not compile `deform6` (lib) due to 1 previous error
```

`AGENTS.md` says a breakage that produces no failure means the covering test
does not exist, so the missing test was looked for. **It cannot be written,
and the reason is worth recording rather than papering over.**
`PeImage::imported_dlls` folds every name with `to_ascii_uppercase` before
`classify` sees it, and the only names `classify` accepts as Visual Basic 6
are case variants of `MSVBVM60.DLL`. The upper case fold of every one of them
is exactly `VB6_DLL`. So for any file that reaches a `Report` at all, the
matched name is equal to the constant, and no fixture can make the two differ.

What does hold the property, in three places rather than one:

- `vb/runtime.rs` proves that `classify` returns the matched entry rather than
  the constant, with a lower case literal list that never goes through the
  fold. Plan 01-06 breakage 1 made that test fail.
- The lint wall, above. The naive substitution leaves the binding unused and
  the gate denies warnings.
- Plan 01-08 task 1 greps `crates/deform6-cli/src/` for the bare text of the
  runtime name and the signature, which is where a literal would actually
  mislead a user.

`the_runtime_name_and_the_signature_are_read_out_of_the_file` still earns its
place: it compares both fields against bytes read at the offsets the file
itself names, through `dll_name_sites` and through a second walk of the entry
stub. It catches a wrong value. It cannot catch a right value that came from
the wrong place, and the test's own doc comment does not claim it can.

## The seventeen behaviours and the tests that hold them

| Behaviour | Test |
|---|---|
| `ProjectInfo` resolves and its object table address resolves | `the_project_data_address_reaches_an_object_table_that_resolves` |
| `lp_native_code` is non-zero and the mode is native | `the_corpus_file_is_native_because_its_native_code_address_is_not_zero` |
| A zeroed `lpNativeCode` reads `PCode` | `zeroing_lp_native_code_reports_p_code_but_no_corpus_program_is_p_code` |
| A project data pointer in no section is damaged | `a_project_data_pointer_in_no_section_is_damaged` |
| A truncated `ProjectInfo` window is damaged, and is not read in part | `a_file_that_ends_inside_project_info_is_damaged_and_is_not_read_in_part` |
| The project name is `Mandelbrot_Fractal_Demo` | `the_project_name_comes_from_the_object_table` |
| The two name sources agree | `the_object_table_and_the_header_agree_on_the_project_name` |
| The corpus file declares one object | `the_corpus_file_declares_one_object_and_its_array_holds_one` |
| A capacity above the count is normal, and the count is not the capacity | `a_capacity_above_the_object_count_is_normal_and_is_not_a_defect` |
| A capacity below the count is a recoverable defect and not a refusal | `a_capacity_below_the_object_count_is_a_recoverable_defect_and_not_a_refusal` |
| An object table pointer in no section is damaged | `an_object_table_pointer_in_no_section_is_damaged` |
| A project name pointer in no section is damaged | `a_project_name_pointer_in_no_section_is_damaged` |
| `inspect` reports what the `.vbp` declares | `the_corpus_file_reports_what_its_project_file_declares` |
| The runtime name and the signature come from the file | `the_runtime_name_and_the_signature_are_read_out_of_the_file` |
| An empty slice is `Refusal::NotPe` | `an_empty_slice_is_not_a_portable_executable` |
| A Visual Basic 5 file with no signature is `Refusal::IsVb5` | `a_visual_basic_5_file_with_no_signature_is_refused_by_name_and_not_as_damaged` |
| `assert_eq!` compiles on a `Result<Report, Refusal>` | `a_result_of_a_report_compares_and_prints` |

Eighteen tests hold seventeen behaviours. The extra one is
`the_report_measures_the_slice_it_was_given`, which covers `file_len` and
`section_count`. The plan's behaviour list names neither, and both are fields
the command line prints on its File and Format lines.

The behaviour "`inspect` takes only a byte slice" is a grep, as the plan says,
and not a Rust test:
`grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/` exits 1.

## Defects found in the plan

Five. One of them changes what the code reports, and it changes a decision the
plan inherited from `CONTEXT.md` and from `STRUCTURES.md`.

### 1. The object count must not come from `wCompiledObjects`

**Found during:** Task 2, before any code was written. **Rule 1, a bug: the
plan as written prints a number that disagrees with the source in 15 of the
44 corpus programs.**

Task 2 says "Take `w_compiled_objects` as the count". D-05 in this plan and in
plan 01-08 says the same, `CONTEXT.md` says the same, and `STRUCTURES.md`
section 4 recommends it, at confidence `[L]`, on the strength of what two
prior tools do.

**The corpus refutes it.** Both fields were read from all 44 executables and
compared against the number of objects the matching `.vbp` declares.

| Field | Equals the object count the `.vbp` declares |
|---|---|
| `wTotalObjects` at `0x2A` | **44 of 44** |
| `wCompiledObjects` at `0x2C` | **29 of 44** |

In the other 15 files `wCompiledObjects` is larger, and it is larger by the
amount that rounds the array up. A project that declares 1, 2 or 3 objects
reports 4. A project that declares 5 reports 8.

Walking the object array confirms which number is real.
`corpus/vb6-code/Grayscale-effect/Grayscale.exe` declares one form and two
classes in its `.vbp`. Its `wTotalObjects` is 3, its `wObjectsInUse` is 3 and
its `wCompiledObjects` is 4. Its array holds `frmGrayscale`,
`pdOpenSaveDialog`, `FastDrawing`, and then a null pointer.
`corpus/public-domain/LockWorkStation/LockWorkStation.exe` declares one form,
holds `FrmLockWorkStation`, and then three pointers that resolve to nothing.

So `wCompiledObjects` is the **capacity** of the object array and
`wTotalObjects` is the **number of objects**. `STRUCTURES.md` section 12's
measurement that `wCompiledObjects` "bounds a walkable object array, 44 of
44" is true and is not in conflict: it bounds the array because it is the size
of the array.

**Fix.** `ObjectTableHead::object_count` gives `w_total_objects`. Both fields
are kept and both are public, because Phase 2 needs both: the count says how
many objects to read and the capacity is the bound the count must not exceed.
The measurement, the method and the worked example are written into
`STRUCTURES.md` as a new section 4.1, the withdrawn recommendation is marked
withdrawn, and the register gained gap 19 marked CLOSED.

**Why this was not raised as a question instead.** `AGENTS.md` says to measure
the thing you tell the human and to give the number you can prove, and it says
to compare a recovered answer against the original source that is committed
next to the binary. The `.vbp` is that source and it is unambiguous, 44 files
to nothing. Following the plan would have printed `Objects 4` for a program
with one form, for a third of the corpus, with no warning to the reader.

**What this costs.** Plan 01-08's `Objects` line still prints, and its test
pins the label rather than the value, so nothing downstream breaks. D-05's
substance, that the count is free and honest here because the object table is
already reached, is unchanged. Only the byte it is read from moves, by two.

### 2. Reporting any count inequality as damage would mark a third of the corpus damaged

**Found during:** Task 2. **Rule 1, and it follows from defect 1.**

Task 2 says "When `w_compiled_objects` and `w_total_objects` are patched to
disagree ... a `CountMismatch` defect at `Severity::Recoverable` is produced",
and `STRUCTURES.md` section 4 says to "report a mismatch as a damage
indicator". Under defect 1 the two fields are different quantities, and they
disagree in **15 of the 44** corpus files as a normal result of a clean
compile. An implementation that reported inequality would attach a defect to
34% of the corpus, and a warning that fires on a third of all healthy input is
not a warning.

**Fix.** The defect is produced when the capacity is **below** the count. That
direction is a genuine disagreement, because the array then has no room for
the objects the same structure declares, so one of the two numbers is wrong.
The other direction is silent. Two tests hold both directions, and breakage 4
and breakage 5 make each of them fail.

### 3. A recoverable defect cannot be recorded through the `Journal`

**Found during:** Task 2. **Rule 3, a blocking issue.**

The plan requires a count disagreement to produce a `Severity::Recoverable`
defect and to keep going. `Journal::record` is the crate's one policy point,
and in `Mode::Strict`, which is the only mode Phase 1 has, it returns
`Err(Error::Refused(defect))` for a recoverable defect. Recording through the
journal would therefore refuse the file, which is exactly what the plan
forbids two sentences later.

**Fix.** `ObjectTableHead` carries a defect list and exposes it through
`defects()`, which is the shape `PeImage::defects` already has for the section
overlap rule. The journal is untouched. Its strict policy is not wrong: it is
the policy for a run that has asked for no damage at all, and Phase 5's
`--salvage` is what selects the other one.

### 4. `Report` cannot carry the defect it produces, so `inspect` drops it

**Found during:** Task 3. **A gap in the plan, recorded rather than fixed.**

`Report` derives `PartialEq` and `Eq`, which the plan requires and which
RESEARCH.md section 7.2 explains. `Defect` derives `Clone, Debug, Serialize`
and `Error`, and not `PartialEq`. So the locked `Report` shape cannot hold a
defect list, and `inspect` discards both the object count defect and
`PeImage::defects`.

Nothing in this phase reads either, so nothing is lost yet, and `PeImage` has
had the same gap since plan 01-04. It is recorded here because Phase 4's
confidence report is the thing that needs them, and because a reader of
`inspect` should not have to discover it. It is in `.planning/WINDOWS.md` as
an unmet truth.

### 5. Task 2 says to read three fields and no more, and the third field it
names is the one to stop reading

**Found during:** Task 2. A small one.

Task 2 says to read `w_total_objects`, `w_compiled_objects` and
`lpsz_project_name` and no more. `wObjectsInUse` at `0x2E` is not read, which
is right. It is worth recording that it was measured: it equals
`wTotalObjects` in 44 of 44, so it is a third witness for defect 1 and it is
not needed as a fourth field.

## Divergences from the plan that are choices, not defects

### The project data pointer in no section is passed, not patched

Task 1's fourth behaviour says "With `lp_project_data` patched to an address
in no section". `ProjectInfo::read` takes that address as a parameter, so the
test passes the address directly and asserts first that `region_at_va` gives
nothing for it. Patching the header field would have exercised `vb/header.rs`
as well, and task 3's `inspect` tests already cover the chain end to end.

### One extra test

`the_report_measures_the_slice_it_was_given`. `file_len` and `section_count`
are in the `Report` field list the plan gives and in no behaviour, so they
would have shipped with no covering test.

### A fourth commit, for the documentation

`AGENTS.md` requires documentation to sit in its own commit.
`.planning/research/STRUCTURES.md` is not in this plan's `files_modified`. It
is edited anyway, because leaving section 4 recommending the withdrawn field
would make Phase 2 repeat the mistake this plan found, and section 4 is what
Phase 2 will read.

## Threat mitigations

| Threat | State |
|---|---|
| T-01-24, a count as a denial of service | Mitigated so far as this phase can. Nothing is allocated from either count, because the object array is not walked and the acceptance grep proves the name of its pointer appears nowhere. The defect this plan records is now the correct half of that check, and defect 1 changes which field Phase 2 must loop on: the count, bounded by the capacity. |
| T-01-25, `ObjectTable + 0x40` read as a header relative offset | Mitigated. The field is typed `Va` at the read and reaches bytes only through `region_at_va`. Breakage 3 shows what a wrong slot produces: a refusal, not a string. `the_object_table_and_the_header_agree_on_the_project_name` is the corpus assertion that would catch a confusion between the two pointer kinds. |
| T-01-26, the two structure windows | Mitigated and measured. Each window is taken before any field is read. Breakage 2 shows the partial read that happens without it. The margins were measured on all 44 files: the smallest is 1612 bytes for `ProjectInfo` and 2160 for the object table, so no corpus file is near either bound. |
| T-01-27, the order of the runtime decision | Mitigated. Breakage 6 shows the misreport. The test destroys the signature as well as the name, so it cannot pass by accident. |
| T-01-28, the untested P-code branch | Accepted and stated, in three places: the test name, the doc comment on `CompileMode`, and the first paragraph of this summary. |

## Known Stubs

None. Every item the plan names is implemented, and every behaviour has a test.

Three limits are stated in the source rather than papered over.

- **No program in this repository is P-code.** The branch is reachable and
  correct on the field it reads. Nothing here shows a real P-code binary being
  read.
- **`inspect` drops the defects it collects.** See defect 4.
- **`runtime_dll` cannot be distinguished from the constant by any fixture on
  this corpus.** See breakage 7 for the proof and for the three places that do
  hold the property.

The five field refusals inside `ProjectInfo::read` and the three inside
`ObjectTableHead::read` that sit below the window check cannot be reached,
because each window is the exact size of its structure. They stay because
`Region` has no infallible accessor and neither function may unwrap an
`Option`. The doc comment on each says so. This is the same shape as the one
unreachable `ok_or` plan 01-05 recorded in `header_region`.

## Deferred Issues

None.

## Threat Flags

None. This plan adds no network endpoint, no auth path and no file access
path. `grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/` finds
nothing, and the corpus file is reached with `include_bytes!`.

No package was added. `git diff c8db8a0 HEAD -- Cargo.toml Cargo.lock` is
empty.

## Commits

| Commit | Subject |
|---|---|
| `b461b34` | Read ProjectInfo and report the compilation mode |
| `ce8d0bb` | Read the project name and the object count from the object table |
| `09869e8` | Compose inspect and return the report |
| `5fc9144` | Record which object table count is the number of objects |
| `9c370f6` | Record what plan 01-07 built and the object count it corrected |

`git rev-list --count c8db8a0..HEAD` is 5, measured after the last of them.
Three commits carry code with its tests. The fourth carries the correction to
`STRUCTURES.md` and the fifth carries this summary and the state files, as
`AGENTS.md` requires of documentation.

## Self-Check: PASSED

`crates/deform6/src/vb/project.rs` exists and is 673 lines.
`crates/deform6/src/vb/mod.rs` exists and is 320 lines.
`.planning/research/STRUCTURES.md` holds section 4.1 and gap 19. All four
commit hashes resolve in the history of this branch.
`git diff --diff-filter=D --name-only c8db8a0..HEAD` is empty, so no commit
deleted a tracked file. Every command in the gate table was run and its exit
status is recorded above. `git status --porcelain` is empty and
`git status --porcelain corpus/` is empty, so no fixture wrote to disk.
