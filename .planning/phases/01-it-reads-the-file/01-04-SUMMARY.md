---
phase: 01-it-reads-the-file
plan: 04
subsystem: read
tags: [pe, object-seam, address-map, section-table, imports, det-01]
status: complete

requires:
  - Off, Rva, Va and Region from plan 01-02
  - Site, DefectKind, Severity, Defect and Refusal from plan 01-03
  - the read/pe.rs stub and the pub mod line from plan 01-01
provides:
  - PeImage, the only place in the crate that names the object crate
  - SectionInfo, an owned section header with no lifetime
  - PeReject and From<PeReject> for Refusal
  - section_for, the one address to file offset predicate
  - rva_to_off, va_to_off, region_at and region_at_va
  - imported_dlls, delay_loaded_dlls and dll_name_sites
  - has_clr_header
affects:
  - 01-05 and 01-06, which read the entry point stub and the runtime name
  - 01-07, which resolves every VB pointer through rva_to_off
  - 01-08, whose refusal fixtures patch a name at the offset dll_name_sites gives
  - Phase 4, whose report cites a name site as evidence
  - Phase 5, whose fuzz target enters through PeImage::parse

tech_stack:
  added: []
  patterns:
    - one predicate answers one question, and every caller routes through it
    - a fixture that a test builds in memory is the only way to cover a case
      the corpus does not hold
    - a test computes its boundary from the raw field, never from the method
      it covers
    - a synthetic image where the address and the file offset differ is the
      only instrument that can tell them apart

key_files:
  created: []
  modified:
    - crates/deform6/src/read/pe.rs

decisions:
  - SectionInfo::mapped_len moved from task 2 to task 1, because the overlap
    check in task 1 is its first caller and the gate refuses an uncalled
    private path.
  - The data field of PeImage is added in task 2 with region_at, its first
    reader, for the same reason plan 01-02 moved Off::index.
  - region_at clamps its window to the end of the byte slice rather than
    refusing, so a section that declares raw data past the end of the file
    gives a short window and the bytes that do exist stay readable.
  - A section whose mapped length is zero claims no byte and therefore
    overlaps nothing. The half open interval test alone would report it.

metrics:
  duration: 1 session
  completed: 2026-09-07

actuals:
  tokens: 15600
  tasks: 3
  commits: 5
plan_head_before: bade676
---

# Phase 01 Plan 04: The PE envelope and the address map Summary

One file that reads a real binary, resolves the entry point of every corpus
executable to a file offset through the section table, and keeps the `object`
crate behind a seam that nothing else in the crate can see.

## What this plan built

`crates/deform6/src/read/pe.rs`, 949 lines, is the only file in the crate that
names `object`. It holds `PeReject`, `SectionInfo`, `PeImage`, the free
functions `section_table_offset`, `section_header_offset` and
`overlap_defects`, and 24 tests.

`PeImage` exposes the thirteen methods RESEARCH.md section 4.9 compiled, plus
`defects`, which the plan's overlap rule needs and section 4.9 does not list.

| Method | What it gives |
|---|---|
| `parse` | `Result<PeImage, PeReject>` |
| `image_base` | `u32` |
| `entry_rva` | `Rva` |
| `sections` | `&[SectionInfo]` |
| `defects` | `&[Defect]`, the overlaps the parse found |
| `section_for` | `Option<&SectionInfo>`, the one predicate |
| `rva_to_off` | `Option<Off>` |
| `va_to_off` | `Option<Off>` |
| `region_at` | `Option<Region>` |
| `region_at_va` | `Option<Region>` |
| `imported_dlls` | `Result<Vec<String>, object::Error>` |
| `delay_loaded_dlls` | `Result<Vec<String>, object::Error>` |
| `dll_name_sites` | `Result<Vec<(Off, u32)>, object::Error>` |
| `has_clr_header` | `bool` |

## The section the zero filled tail test selected

The plan asks for this fact, so a later reader can see the case was exercised
against a real section.

| File | Section | Virtual size | Raw size | Mapped length |
|---|---|---|---|---|
| `corpus/public-domain/PassGen/PassGen.exe` | `.data` | 8,948 | 4,096 | 4,096 |

The test does not name the section or its index. It scans the parsed section
list for the first section whose virtual size exceeds its raw size and calls
`expect` with a message if none is found, so the test cannot degrade into
nothing when the corpus changes.

The tail is 4,852 bytes wide. The address at distance 4,096 into the section
resolves to nothing, and the address at distance 4,095 resolves to a file
offset inside the file.

Measured for the record, before any code was written, by a script that read
the two headers directly:

```
Mandelbrot.exe len 28672 mach 0x14c nsec 3 magic 0x10b entry 0x1274 base 0x400000
  dir14 (0, 0)
  .text  vsize=14224 va=0x1000 rsize=16384 praw=0x1000 mapped=14224
  .data  vsize=2588  va=0x5000 rsize=4096  praw=0x5000 mapped=2588
  .rsrc  vsize=2664  va=0x6000 rsize=4096  praw=0x6000 mapped=2664
  entry file offset 0x1274 byte 0x68
PassGen.exe len 135168 mach 0x14c nsec 3 magic 0x10b entry 0x195c base 0x400000
  dir14 (0, 0)
  .text  vsize=122108 va=0x1000  rsize=122880 praw=0x1000  mapped=122108
  .data  vsize=8948   va=0x1f000 rsize=4096   praw=0x1f000 mapped=4096
  .rsrc  vsize=2444   va=0x22000 rsize=4096   praw=0x20000 mapped=2444
  entry file offset 0x195c byte 0x68
```

Every fact the prompt gave was checked and every one held. `Mandelbrot.exe`
has no tail. `PassGen.exe` `.data` declares 8,948 against 4,096. Data
directory 14 is zero in both. The entry byte is `0x68` in both.

## The gate

Run on the committed tree at `5ba4e33`, with a clean working tree.

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
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running unittests src/main.rs (target/debug/deps/deform6-31e29b1143ef8fee)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   Doc-tests deform6
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

58 library tests, of which 24 are in `read::pe::tests`. The phase held 34
before this plan.

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

The four greps the plan's success criteria name were each run.

```
$ grep -rlE '\bobject::|use object' crates/deform6/src/ --include='*.rs'
crates/deform6/src/read/pe.rs

$ grep -rnE '\.(contains_rva|pe_file_range_at)[[:space:]]*\(' crates/     # exit 1

$ grep -rn 'contains_rva\|pe_file_range_at' crates/                      # exit 1

$ grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/              # exit 1

$ git status --porcelain corpus/                                         # empty
```

The third grep is stronger than the plan asks for. Neither forbidden name
appears anywhere under `crates/`, in a call, in a comment or in a string.

## The deliberate breakages

`AGENTS.md` says to break the thing a test covers and watch it fail. Six
breakages, nine runs. Three of them were run twice, because the first run
found a fault in the test rather than in the code and the test was repaired
between the two runs. Every breakage was reverted and the gate was green
before each commit. No commit holds a broken state.

### 1. The optional header magic becomes the PE32+ value

`IMAGE_NT_OPTIONAL_HDR32_MAGIC` replaced with `IMAGE_NT_OPTIONAL_HDR64_MAGIC`.

Five tests failed, each with the same error value. 38 passed.

```
---- read::pe::tests::the_corpus_file_parses_and_names_its_three_sections stdout ----
panicked at crates/deform6/src/read/pe.rs:337:48:
called `Result::unwrap()` on an `Err` value: NotPe32
```

The other four were `the_corpus_file_holds_no_clr_header`,
`the_entry_point_lies_inside_the_text_section`,
`the_section_list_outlives_the_bytes_it_was_built_from` and
`a_patched_data_directory_fourteen_makes_the_image_a_dot_net_assembly`.

### 2. `has_clr_header` returns a constant `false`

The body replaced with `false`.

One test failed. **42 passed.**

```
---- read::pe::tests::a_patched_data_directory_fourteen_makes_the_image_a_dot_net_assembly stdout ----
panicked at crates/deform6/src/read/pe.rs:424:9:
assertion failed: image.has_clr_header()
```

This is the whole reason the positive fixture exists, and the number that
proves it is the 42. Every other test in the phase passed while the method
returned a constant. Data directory 14 is zero in all 44 corpus files, so no
corpus test can tell a working `has_clr_header` from a broken one.

### 3. `mapped_len` returns `virtual_size`

Run twice. The first run is a finding about the test, not about the code.

**First run, against the test as the plan describes it.** Two tests failed,
but `an_address_in_a_zero_filled_tail_resolves_to_nothing` failed on its own
guard clause and never reached the assertion it exists for:

```
panicked at crates/deform6/src/read/pe.rs:578:9:
assertion failed: first_unmapped < section.virtual_address.get() + section.virtual_size
```

The test computed the tail boundary as `virtual_address + mapped_len()`, so it
asked the method it covers where the boundary is. When that method changed,
the boundary moved with it and the test failed for the wrong reason. That is
the fault `AGENTS.md` names under "a test can pass for the wrong reason",
seen from the other side. See deviation 3.

**Second run, after the test was rebased on `size_of_raw_data`.** One test
failed, on the assertion it exists for. 50 passed.

```
---- read::pe::tests::an_address_in_a_zero_filled_tail_resolves_to_nothing ----
panicked at crates/deform6/src/read/pe.rs:580:9:
assertion `left == right` failed
  left: Some(Off(131072))
 right: None
```

`131072` is `0x20000`, and `PassGen.exe` is 135,168 bytes long, so the broken
rule hands back a real byte inside the file at an address the program sees as
zero. That is exactly the fault the rule exists to refuse.

### 4. `mapped_len` returns `size_of_raw_data`

Run twice.

**First run: nothing failed. 50 passed, 0 failed.** The plan predicts that
"the same test fails" here, and it does not, and it cannot. For a section with
a zero filled tail the raw size **is** the smaller of the two numbers, so
`min(virtual_size, size_of_raw_data)` and `size_of_raw_data` give the same
answer on `PassGen.exe` `.data`. The tail test is structurally blind to this
half of the rule. See deviation 4.

**Second run, after `an_address_in_the_file_alignment_padding_resolves_to_nothing`
was added.** One test failed. 50 passed.

```
---- read::pe::tests::an_address_in_the_file_alignment_padding_resolves_to_nothing ----
panicked at crates/deform6/src/read/pe.rs:616:9:
assertion `left == right` failed
  left: Some(Off(18320))
 right: None
```

`18320` is `0x4790`, inside the 28,672 bytes of `Mandelbrot.exe`, in the file
alignment padding of `.text`. The loader never maps that byte. A rule that
returned it would read padding as code.

### 5. The descriptor loop returns after the first descriptor

`return Ok(out);` added at the end of the `while let` body of
`imported_dlls`. Run twice.

**First run: nothing failed. 57 passed, 0 failed.** The plan predicts this
outcome and asks for it to be recorded, and it is correct.
`Mandelbrot.exe` imports exactly one DLL, `MSVBVM60.DLL`, so a loop that stops
after the first name returns the same list as a loop that reads them all.
`PassGen.exe` imports one as well.

**Second run, after the synthetic image was given two import descriptors.**
One test failed. 57 passed.

```
---- read::pe::tests::every_descriptor_is_read_and_every_name_is_folded_to_upper_case ----
panicked at crates/deform6/src/read/pe.rs:914:9:
  left: ["SOMELIB.DLL"]
 right: ["SOMELIB.DLL", "OTHERLIB.DLL"]
```

See deviation 5.

### 6. `dll_name_sites` pushes the raw address as if it were an offset

`out.push((offset, len))` replaced with `out.push((Off::new(rva.get()), len))`
and the `rva_to_off` call removed.

One test failed. 57 passed.

```
---- read::pe::tests::a_name_site_is_a_file_offset_and_not_an_address ----
panicked at crates/deform6/src/read/pe.rs:897:13:
assertion `left == right` failed
  left: Off(4160)
 right: Off(1088)
```

`4160` is `0x1040`, the address. `1088` is `0x440`, the file offset. The
corpus test `a_name_site_names_the_bytes_that_imported_dlls_returned` passed
through this breakage untouched. See deviation 6, which is the reason.

## Findings that correct the plan

The plan is right about the corpus, right about the `object` call sequence and
right about every constant it names. Six of its statements did not survive
contact, and four of those are places where a test the plan describes cannot
fail.

### 1. `SectionInfo::mapped_len` cannot wait for task 2

**Found during:** Task 1. **Rule 3, a blocking issue.**

The plan puts `mapped_len` in task 2 and the overlap check in task 1. The
overlap check compares the ranges `[virtual_address, virtual_address +
mapped_len)`, which RESEARCH.md section 5.3 case D states directly, so task 1
cannot be written without the method.

`mapped_len` is defined in task 1, with its full doc comment, and task 2 adds
its other callers. This is the mirror of plan 01-02's deviation 1: code goes
in the same commit as the thing that uses it.

### 2. The `data` field of `PeImage` cannot be added in task 1

**Found during:** Task 1. **Rule 3, a blocking issue.**

The first draft stored `data: &'a [u8]` in task 1, because `region_at` needs
it. `region_at` is a task 2 method, so at the end of task 1 the field has no
reader and the gate refuses the commit:

```
error: field `data` is never read
   --> crates/deform6/src/read/pe.rs:120:5
    = note: `-D dead-code` implied by `-D warnings`
```

The field is added in task 2, with `region_at`. Same precedent, same reason.
The final struct is what the plan describes.

### 3. A test must not ask the method it covers where its boundary is

**Found during:** Task 2, breakage 3, first run.

The plan says to select the tail section by scanning, which the test does. It
does not say where the tail boundary comes from, and the obvious reading is
`virtual_address + mapped_len()`. That reading makes the test move its own
goal posts. When `mapped_len` was broken to return `virtual_size`, the
boundary moved with it and the test failed on a guard clause instead of on the
address that now resolves.

Both tail tests now compute the boundary as `virtual_address +
size_of_raw_data`, which is the raw field. `size_of_raw_data` is the correct
boundary for a section with a tail by definition, and the test states that in
a comment.

**This is the difference between a test that fails and a test that fails for
the right reason.** Both runs are recorded above so the difference is visible.

### 4. The zero filled tail cannot catch `mapped_len = size_of_raw_data`

**Found during:** Task 2, breakage 4, first run. **Rule 2, missing critical
coverage.**

The plan says: "Change it to return `size_of_raw_data` and confirm the same
test fails." It does not fail. 50 tests passed with the rule half broken.

The reason is arithmetic. `mapped_len` is `min(virtual_size,
size_of_raw_data)`. A section with a zero filled tail has `virtual_size >
size_of_raw_data`, so the minimum **is** `size_of_raw_data` and the two
expressions agree on every such section. The tail test cannot see this half of
the rule, and no test in the plan covers the other direction, so
`min(a, b) -> b` would have shipped.

`an_address_in_the_file_alignment_padding_resolves_to_nothing` was added. It
scans `Mandelbrot.exe` for the first section whose raw size exceeds its
virtual size, checks that the first address above the virtual size resolves to
nothing and that the address one below it resolves. 125 of the 132 corpus
sections have that padding, so the case is the common one, and the plan's
behaviour list omits it.

The two tests are now one pair: each catches exactly one direction of the
`min`, and neither catches the other.

### 5. Neither corpus file can see a truncated descriptor loop

**Found during:** Task 3, breakage 5, first run. **Rule 2, missing critical
coverage.**

The plan predicts this and asks for it to be recorded, which is right, but it
stops there. A loop with no second iteration is not covered by anything in the
plan, and `AGENTS.md` says a test that cannot fail is worse than no test.

The synthetic image the plan asks for, for the "no import data directory"
case, was given **two** import descriptors instead of one, and
`every_descriptor_is_read_and_every_name_is_folded_to_upper_case` asserts both
names come back in order. The truncated loop now fails.

The first name in that image is written in the file as `somelib.dll`, in lower
case. The plan notes that all 44 corpus files hold the upper case name, so the
`to_ascii_uppercase` fold is never exercised by the corpus either. It is
exercised now, by the same test, at no extra cost.

### 6. Neither corpus file can tell a file offset from an address

**Found during:** Task 3. **Rule 2, missing critical coverage.**

The plan says to compare the site offset against `rva_to_off` of the
descriptor's name address, then break `dll_name_sites` to push the raw address
and confirm the test fails. Against either corpus file that test cannot fail.
Measured:

```
Mandelbrot.exe  import name rva 0x44e8  -> file offset 0x44e8   equal
PassGen.exe     import name rva 0x1e808 -> file offset 0x1e808  equal
```

Both files put the import directory in `.text`, and in both files `.text` has
`virtual_address == pointer_to_raw_data == 0x1000`, so `rva_to_off` is the
identity function over the whole section. This is precisely the trap
RESEARCH.md section 5.3 case C describes: "on an image where file alignment
equals section alignment the two often agree, so the bug hides until one file
where they do not". The plan quotes that sentence and then writes a test that
walks into it.

The synthetic image is built so that they differ. Its one section is at
address `0x1000` and at file offset `0x400`. The name at address `0x1040` is
at file offset `0x440`. `a_name_site_is_a_file_offset_and_not_an_address`
asserts the site equals `rva_to_off` of the address **and** asserts that the
two numbers differ, so the test fails loudly if a future fixture makes them
equal again and quietly stops proving anything.

Breakage 6 shows it works, and shows that the corpus test
`a_name_site_names_the_bytes_that_imported_dlls_returned` passed through the
same breakage without noticing.

## Two smaller decisions the plan did not settle

### `region_at` clamps rather than refuses

RESEARCH.md says 0 of 132 corpus sections declare raw data past the end of
their file, and a hostile file can. `region_at` takes the mapped bytes that
the byte slice actually holds, so such a section gives a short window rather
than nothing. The bytes that do exist are real, and every read inside the
window is still bounded by the real length of the slice, so nothing is
synthesised. Refusing would throw away readable bytes on the strength of a
field in the file.

### A section with a zero mapped length overlaps nothing

The half open interval test `a.start < b.end && b.start < a.end` reports an
overlap between an empty range and a range that contains its start point. An
empty section claims no byte, so `overlap_defects` skips a pair when either
mapped length is zero. No corpus section has a zero virtual size, so this
never fires today.

## The overlap rule is untested by construction

All 44 corpus executables have exactly 3 sections, `.text`, `.data` and
`.rsrc`, ascending, so no corpus file reaches `overlap_defects`. The doc
comment on that function says so in the same way the P-code branch is called
out, and it says why the rule exists anyway: the Windows loader maps sections
in table order and gives the later one, `section_for` takes the first match,
and a file with overlapping sections is either damaged or built to make two
parsers see different bytes.

A fuzz input in Phase 5 or a hand made regression file has to exercise it.
This is recorded in `.planning/WINDOWS.md` as an unrun verification.

## Threat mitigations

| Threat | State |
|---|---|
| T-01-11, overlapping sections | First match in section table order, in one predicate, with the overlap reported once at parse time as a `Recoverable` defect. Untested by construction, and said so in the source. |
| T-01-12, the address as a file offset fallback | Not implemented. `an_address_above_every_section_resolves_to_nothing` picks the address one past the last mapped byte, which is also a valid file offset in this file, so a fallback would answer there. It returns `None`. |
| T-01-13, memory unsafety inside `object` | Transferred. `object` is named in one file, the library forbids `unsafe`, and the module doc says the Phase 5 fuzz target enters through this API. |
| T-01-14, the import walk | Every read goes through the fallible descriptor iterator, no allocation is sized from a field in the file, and a name whose address does not resolve is dropped rather than read. `a_truncated_file_cannot_read_its_import_directory` covers the error path. |
| T-01-15, `parse` read as a validity gate | The doc comment on `parse` states that it validates headers only, names the one eighth truncation measurement, and says that any code that treats it as a validity gate is a bug. `a_truncated_file_cannot_read_its_import_directory` parses a half file successfully and then fails to read it, which is the same fact as a test. |

## Known Stubs

None. Every method the plan names is implemented and covered.

Two behaviours have no test and cannot have a useful one, and both are stated
in the source rather than papered over.

- **The overlap rule.** No corpus file overlaps. See above.
- **`delay_loaded_dlls` against a real delay load table.** No corpus file has
  a delay load directory, in 44 of 44. The empty case is covered. The doc
  comment says the list is informational only and must never decide the
  runtime, and gives the reason: `object` computes the name address with a
  wrapping subtraction and never reads the `attributes` field, and the old
  Visual C++ 6 format stores virtual addresses there.

## Deferred Issues

None.

## Threat Flags

None. This plan adds no network endpoint, no auth path and no file access
path. `grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/` finds
nothing, and both corpus files are reached with `include_bytes!`.

No package was added. `git diff bade676 HEAD -- Cargo.toml Cargo.lock` is
empty.

## Commits

Three commits carry the code and its tests, one per task, and two carry the
documentation, as `AGENTS.md` requires.

| Commit | Subject |
|---|---|
| `ffc6504` | Add the PE envelope and the owned section table |
| `34c495f` | Add the single address to file offset predicate |
| `5ba4e33` | Add the import directory walk and the name sites |
| `89da054` | Ignore the generated planning state cache |
| (this file) | Record what plan 01-04 built and the four tests in its plan that could not fail |

`git rev-list --count bade676..HEAD` is 5.

`gsd-tools` wrote `.planning/state.json` when this plan ran its state queries.
The file has never been in the history of this repository, it is derived from
`ROADMAP.md` and `STATE.md`, and its phase status disagreed with `STATE.md` as
soon as it was written. It is ignored rather than committed.

## Self-Check: PASSED

`crates/deform6/src/read/pe.rs` exists and is 949 lines. All three commit
hashes resolve in the history of this branch. Every command in the gate table
was run and its exit status is recorded. `git status --porcelain corpus/` is
empty, so the `has_clr_header` fixture and the truncation fixture wrote
nothing to disk.
