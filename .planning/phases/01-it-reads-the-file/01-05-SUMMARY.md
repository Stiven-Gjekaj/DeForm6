---
phase: 01-it-reads-the-file
plan: 05
subsystem: vb
tags: [vbheader, entry-stub, header-strings, off-versus-va, det-02, gap-1]
status: complete

requires:
  - Off, Region and Va from plan 01-02
  - Refusal from plan 01-03
  - PeImage, entry_rva, rva_to_off, va_to_off, region_at and region_at_va from
    plan 01-04
  - the vb/header.rs stub and the pub mod line from plan 01-01
provides:
  - header_region, which walks the entry stub and gives a bounded window on
    the VBHeader
  - VbHeader, with four virtual addresses and four header relative offsets
    that the types keep apart
  - the four header strings, decoded as Latin-1
  - STRUCTURES.md sections 2, 2.3 and 11, with gap 1 closed
affects:
  - 01-07, which reads ProjectInfo from VbHeader::lp_project_data
  - 01-08, whose command line prints the header line
  - Phase 3, which loops over w_form_count entries of the GUI table
  - Phase 4, which writes ExeName32 from exe_name plus the literal ".exe"

tech_stack:
  added: []
  patterns:
    - a window is sized by what it must reach, not by the size of the
      structure that names it
    - a scalar field is proved by a synthetic image whose every field holds a
      different value, because the corpus holds zero in several of them
    - a test walks to its expected offset by a second route, never by the
      function it covers
    - a claim about what a wrong reading would produce is written as a test,
      so it is measured rather than asserted in a comment

key_files:
  created: []
  modified:
    - crates/deform6/src/vb/header.rs
    - .planning/research/STRUCTURES.md

decisions:
  - The header window is 0x16C bytes, not the 0x68 the plan names. The four
    strings live after the structure, at offsets measured from the same base,
    so a 0x68 window cannot reach any of them.
  - The window is clamped to the bytes the section holds rather than refused
    when it is short, which is the choice Region::cstr already makes.
  - The three fields the plan says no test may pin are pinned on a synthetic
    image that the test writes itself. On the corpus two of them are zero and
    nothing can see them.
  - A closed gap keeps its row in the section 11 register and is marked
    CLOSED, against RESEARCH.md section 8.6, which says to move it out.

metrics:
  duration: 1 session
  completed: 2026-09-07

actuals:
  tokens: 47000
  tasks: 3
  commits: 5
plan_head_before: a7e4e24
---

# Phase 01 Plan 05: The VB header Summary

`vb/header.rs` walks the entry stub to the `VB5!` header, reads the
`VBHeader`, and resolves its four strings against the header window itself,
so the one structure that mixes a virtual address with four header relative
offsets cannot have the two read as one another.

## What this plan built

`crates/deform6/src/vb/header.rs`, 755 lines, with 22 tests. The library part
names neither `object` nor any file system type.

| Item | What it gives |
|---|---|
| `header_region` | `Result<Region, Refusal>`, a bounded window whose offset 0 is the `VB5!` magic and whose base is the file offset of that magic |
| `VbHeader` | 18 public fields: 4 magic bytes, 6 scalars, 4 `Va`, 4 `Off` and 4 `String` |
| `VbHeader::read` | `Result<VbHeader, Refusal>`, with a named refusal for every field |

`.planning/research/STRUCTURES.md` now names `oProjectExeName` and
`oProjectTitle` in the section 2 table, reads section 2.3 as a settled
question, and marks gap 1 CLOSED in the section 11 register. Section 13 is
untouched, as the plan requires.

## The corpus facts this plan measured for itself

A script read all 44 corpus executables before any code was written. It walked
each PE header by hand, resolved the entry point through the section table,
followed the pushed operand and read the four string slots. These are its
numbers, not a citation.

| Claim | Result |
|---|---|
| The byte at the entry point is `0x68` | 44 of 44, and no other opcode appears |
| The pushed address lands on `VB5!` | 44 of 44 |
| Bytes `0x68` to `0x77` of the header are sixteen zeros | 44 of 44 |
| `0x58 < 0x5C < 0x60 <= 0x64` | 44 of 44 |
| The largest header relative offset | 171, `0xAB` |
| The furthest byte any string reaches | 196, `0xC4` |
| The two slots hold different strings | **23** of 44 |
| The two slots hold the same string | 21 of 44 |

21 plus 23 is 44. Every figure the plan and the prompt gave was checked and
every one held.

`Mandelbrot.exe`: image base `0x400000`, entry `0x1274`, header at VA
`0x401760` and file offset `0x1760`, string offsets `0x78`, `0x83`, `0x9B`
and `0x9C`, giving `Mandelbrot`, `Mandelbrot Fractal Demo`, the empty string
and `Mandelbrot_Fractal_Demo`. Those are the four values `Mandelbrot.vbp`
declares.

## The gate

Run on the committed tree at `d8272bc`, with a clean working tree.

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
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running unittests src/main.rs (target/debug/deps/deform6-31e29b1143ef8fee)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   Doc-tests deform6
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

80 library tests, of which 22 are in `vb::header::tests`. The phase held 58
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

The greps the plan and the prompt name were each run.

```
$ grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/           # exit 1

$ grep -rlE '\bobject::|use object' crates/deform6/src/ --include='*.rs'
crates/deform6/src/read/pe.rs

$ grep -nw object crates/deform6/src/vb/header.rs
157:    /// Null means the startup object is a form and not a module.

$ grep -qE 'from_utf8_lossy[[:space:]]*\(' crates/deform6/src/vb/header.rs
                                                                      # exit 1

$ grep -qE '^\| 0x58 \| 4 \| .oProjectExeName' .planning/research/STRUCTURES.md
$ grep -qE '^\| 0x5C \| 4 \| .oProjectTitle'   .planning/research/STRUCTURES.md
$ ! grep -qE '^\| 0x5[8C] \|.*Disputed'        .planning/research/STRUCTURES.md
$ grep -qE '^\| 1 \|.*\| CLOSED'               .planning/research/STRUCTURES.md
$ grep -qE '^\| 18 \|.*Do not implement without a sample' \
      .planning/research/STRUCTURES.md
                                                            # all exit 0
```

The only match for the word `object` in `vb/header.rs` is the English word in
a doc comment. The only match for `from_utf8_lossy` is the doc comment that
says not to use it. See defect 3.

## The deliberate breakages

`AGENTS.md` says to break the thing a test covers and watch it fail. Seven
breakages, nine runs. One was run three times, because the first two runs
found faults in the tests rather than in the code and the tests were repaired
between the runs. Every breakage was reverted and the gate was green before
each commit. No commit holds a broken state.

### 1. The accepted opcode set gains `0x5A`

The plan asks for this one.

```rust
if entry.u8(Off::new(0)) != Some(PUSH_IMM32) && entry.u8(Off::new(0)) != Some(0x5A) {
```

One test failed. 63 passed.

```
---- vb::header::tests::an_entry_opcode_of_0x5a_is_refused_because_no_corpus_file_shows_it ----
panicked at crates/deform6/src/vb/header.rs:189:35:
called `Result::unwrap_err()` on an `Ok` value: Region { bytes: [86, 66, 53, 33,
54, 38, 42, 0, ...], base: Off(5984) }
```

`Off(5984)` is `0x1760` and `[86, 66, 53, 33]` is `VB5!`. With `0x5A` accepted
the rest of the stub is intact, so the file parses to a real header. That is
the whole point of the refusal: no corpus file exercises the path, so nothing
downstream can be trusted to notice when it is taken.

### 2. The executable name is resolved as a virtual address

The plan asks for this one and predicts the value `MZ`. **It does not produce
`MZ`. It cannot.** This is the plan's most substantial defect and it is
recorded as defect 2 below.

`VbHeader::read` was given the `PeImage` and the exe name went through
`pe.region_at_va(Va::new(o_project_exe_name.get()))`.

12 tests failed. 65 passed. The exe name test reported:

```
---- vb::header::tests::the_executable_name_is_the_vbp_exe_name_without_its_extension ----
called `Result::unwrap()` on an `Err` value: Damaged("BREAKAGE: the offset is in no section")
```

**The observed value is a refusal, not a string.** The stored offset is `0x78`.
The image base is `0x400000`. `Va::to_rva` is a checked subtraction, so it
returns nothing before the section table is consulted at all.

Reaching `MZ` needs three faults at once, and this crate has none of them:

| Fault | State here |
|---|---|
| Subtract the image base with a wrapping or saturating operation | `Va::to_rva` is `checked_sub`. Plan 01-02 breakage 2 measured the wrapping form giving `Rva(0xFFC0_1000)`. |
| Resolve an address that is in no section | `section_for` returns nothing for any address below `0x1000`, and `0x78` and `0` are both below it. |
| Fall back to reading an unresolved address as a file offset | Not implemented. This is threat T-01-12, which plan 01-04 refused by design. |

Measured for the record: file offset `0` in `Mandelbrot.exe` holds `MZ`, and
file offset `0x78`, which is where a fallback would have landed, holds `$`.
So even a two fault route reports `$` and only a three fault route that also
clamps the subtraction to zero reports `MZ`.

`the_executable_name_offset_reaches_nothing_when_it_is_read_as_an_address` is
now a permanent test that measures all three of these, so the claim is checked
by the gate rather than asserted in a comment.

### 3. The reads at `0x58` and `0x5C` are swapped

The plan asks for this one and predicts that both string tests fail. They do.
Five tests failed. 72 passed.

```
---- vb::header::tests::the_executable_name_is_the_vbp_exe_name_without_its_extension ----
  left: "Mandelbrot Fractal Demo"
 right: "Mandelbrot"

---- vb::header::tests::the_title_is_the_vbp_title ----
  left: "Mandelbrot"
 right: "Mandelbrot Fractal Demo"
```

`the_four_string_offsets_ascend`, the synthetic string test and the refusal
test failed as well, so five instruments see the swap.

### 4. `w_form_count` and `w_external_count` are read from each other's offsets

Not in the plan. Added because the plan forbids any test from pinning those
two fields, which would leave them with no covering test at all. See defect 4.

**One test failed. 76 passed.**

```
---- vb::header::tests::every_scalar_field_is_read_from_the_offset_the_layout_gives ----
  left: 7
 right: 5
```

The 76 is the number that matters. `Mandelbrot.exe` has one form and zero
external components, so with the two offsets exchanged the corpus reports zero
forms and one external component and **no corpus test in the phase notices**.
The synthetic image, where the two fields hold 5 and 7, is the only instrument
that can see it.

### 5. The header window starts four bytes late

`hdr.subregion(Off::new(0), width)` became `hdr.subregion(Off::new(4), width)`,
which is the shape of a "skip the magic" mistake. Run three times. The first
two runs are findings about the tests, not about the code.

**First run.** 14 failed, 63 passed, but two of the failures and one of the
passes were wrong.

`a_header_that_does_not_begin_with_the_magic_is_refused` failed for the wrong
reason:

```
called `Result::unwrap_err()` on an `Ok` value: Region { bytes: [88, 66, 53, 33, ...],
base: Off(5988) }
```

`[88, 66, 53, 33]` is `XB5!`, at `Off(5988)` = `0x1764`. Its helper asked
`header_region` where the header was and then patched there, so when the
window moved the patch moved with it and landed four bytes late. That is the
fault `AGENTS.md` names under "a test can pass for the wrong reason", and it is
the same shape as plan 01-04's finding 3.

`the_signature_is_carried_as_the_four_bytes_the_file_holds` **passed** through
the breakage for the same reason: it took its comparison offset from
`hdr.file_offset(Off::new(0))`, so both sides moved together.

**Second run, after `header_offset_the_hard_way` was added.** That helper
walks the stub with the plan 01-04 primitives only, so it cannot move when
`header_region` does. 13 failed, 64 passed.

```
---- vb::header::tests::the_header_window_is_based_on_the_file_offset_of_the_magic ----
  left: Some(Off(5988))
 right: Some(Off(5984))
```

`a_header_that_does_not_begin_with_the_magic_is_refused` now **passes** under
this breakage, which is correct: it patches the real magic at `0x1760` and the
magic check refuses the file before the window is ever built.

**Third run, after the signature test was rebased on the same helper.**
14 failed, 63 passed.

```
---- vb::header::tests::the_signature_is_carried_as_the_four_bytes_the_file_holds ----
  left: [54, 38, 42, 0]
 right: [86, 66, 53, 33]
```

`[54, 38, 42, 0]` is `0x2636` little endian followed by two zeros, that is the
runtime build read where the magic should be.

### 6. The executable name offset is read from `0x30` instead of `0x58`

Run to prove that
`the_executable_name_offset_reaches_nothing_when_it_is_read_as_an_address` can
fail, without editing a file this plan does not own. 11 failed, 67 passed.

```
---- vb::header::tests::the_executable_name_offset_reaches_nothing_when_it_is_read_as_an_address ----
called `Result::unwrap()` on an `Err` value:
Damaged("the VB header executable name is not a bounded string")
```

`0x30` holds `lpProjectData`, which is `0x401814`. Read as a header relative
offset it leaves the 364 byte window and refuses. That is the mirror image of
breakage 2: an address read as an offset refuses, and an offset read as an
address refuses. Neither one reports a string.

### 7. The two entry stub refusal sentences are exchanged

Not in the plan. Run to prove that the two refusals no corpus file reaches
each have an instrument, and that each instrument reaches its own branch and
not the other.

Two tests failed, and only those two. 78 passed.

```
---- vb::header::tests::an_entry_point_in_no_section_is_refused ----
  left: Damaged("the push operand runs past the end of the section")
 right: Damaged("the entry point is in no section")

---- vb::header::tests::a_push_operand_that_runs_past_the_end_of_the_section_is_refused ----
  left: Damaged("the entry point is in no section")
 right: Damaged("the push operand runs past the end of the section")
```

Both corpus files have an entry point that resolves and a stub that fits
inside its section, so before commit `d8272bc` neither branch had a test. The
synthetic image gives both: the entry point is moved above every section, and
then to the last byte of the section, so the four byte operand lies outside
the window that the address resolves to.

## Defects found in the plan

Five. Four of them change the code, and one of them makes the plan as written
unable to pass its own tests.

### 1. The `0x68` byte window cannot reach any of the four strings

**Found during:** Task 1. **Rule 3, a blocking issue. This is the serious one.**

Task 1 says: "Return `subregion(Off::new(0), 0x68)`, a window whose offset 0 is
the `VB5!` magic". Task 2 says: "Resolve each string with `hdr.cstr(offset,
max)` against the same header window that task 1 returned".

The two cannot both hold. The `VBHeader` structure is `0x68` = 104 bytes, and
the four strings are **not inside it**. They sit after it, at offsets measured
from the same base. In `Mandelbrot.exe` those offsets are `0x78`, `0x83`,
`0x9B` and `0x9C`, that is 120, 131, 155 and 156, and every one of them is past
104. `Region::cstr` at offset 120 on a 104 byte window returns nothing, so
**all four strings would have been refusals** and none of task 2's eight
behaviours could pass.

The plan's own threat register repeats the error. T-01-17 says "The window
itself is 0x68 bytes, so a run cannot leave the header." The scan does not need
to leave the header: the strings were never in it.

RESEARCH.md section 8.5 states the fact the plan needed, and the plan cites the
section for a different sentence: the sixteen bytes at `0x68` to `0x77` are "a
sixteen-byte gap between the last field and the string pool". A gap after the
last field is a pool outside the structure.

**Fix.** `HEADER_WINDOW` is `HEADER_SIZE + STRING_MAX` = `0x68 + 0x104` = 364
bytes. Measured against the corpus, the furthest byte any string reaches is
196, so 364 leaves margin and still refuses an offset that points into the rest
of the section. Each `cstr` remains bounded by `0x104` as well, so T-01-17 is
mitigated by the two bounds together rather than by the structure size.

The window is clamped to `min(HEADER_WINDOW, hdr.len())` rather than refused
when the section is short. Refusing would throw away readable bytes on the
strength of a field in the file, and `Region::cstr` already made this choice
for the same reason in plan 01-02.

Two tests cover the bound:
`a_string_offset_that_leaves_the_header_window_is_refused` and
`a_string_with_no_terminator_inside_the_bound_is_refused`.

### 2. The `MZ` breakage cannot produce `MZ` in this crate

**Found during:** Task 2, breakage 2.

The plan says: "confirm the `exe_name` test fails and that the value it reports
is `MZ`", and the `<output>` block asks for that value to be recorded because
it "is the evidence that the two pointer kinds are genuinely distinguished".

The value is not `MZ`. It is a refusal. The full measurement is in breakage 2
above. The short form: `0x78` is below the image base, `Va::to_rva` is a
checked subtraction, and `PeImage` has no fallback that reads an unresolved
address as a file offset.

**This is a stronger result than the plan asks for, and it is the plan's own
premise turned around.** The `MZ` outcome is what a parser that carries only
`u32` produces. The three newtypes exist precisely so that outcome becomes
unreachable, and the measurement shows that it is. A test that could still
report `MZ` would mean the types had failed.

The plan's breakage is also not applicable as written: it says to route the
resolution through `pe.region_at_va`, but the signature the same task
specifies, `VbHeader::read(hdr: &Region)`, has no `PeImage` in scope. The
breakage was run by temporarily widening that signature.

**Fix.** `the_executable_name_offset_reaches_nothing_when_it_is_read_as_an_address`
is a permanent test that measures each of the three faults the `MZ` outcome
would need. The claim is now checked by the gate.

### 3. Task 2's `from_utf8_lossy` grep forbids the comment that forbids the call

**Found during:** Task 2, running the plan's `<verify>`.

The plan's automated verification ends with
`! grep -q 'from_utf8_lossy' crates/deform6/src/vb/header.rs`. The plan also
says, three paragraphs earlier, to write down why `String::from_utf8_lossy` is
wrong for a VB6 header string. A file that carries that reason fails that grep.
`crates/deform6/src/read/region.rs` already carries the same sentence, put
there by plan 01-02.

A grep for a bare name cannot tell a call from a warning about a call.

**Fix.** The doc comment stays, because it carries the reason and the reason is
what stops the next reader reaching for the lossy decoder. The check used is
call shaped:

```sh
! grep -qE 'from_utf8_lossy[[:space:]]*\(' crates/deform6/src/vb/header.rs
```

It exits 1 on this file. Recorded here so a later reader does not "repair" the
file to satisfy the weaker grep.

### 4. Three fields are left with no test that can fail

**Found during:** Task 2. **Rule 2, missing critical coverage.**

Task 2's behaviour list says `runtime_build`, `w_form_count` and
`w_external_count` "are read and returned, and no test compares any of them
against a literal". Under that rule those three fields have no covering test at
all, and `AGENTS.md` says a test that cannot fail is worse than no test.

The rule itself is right about the corpus. RESEARCH.md Pitfall 6 says not to
pin a build number or a count read out of one binary, and the prompt repeats
it. But the corpus cannot substitute: `Mandelbrot.exe` has `wExternalCount`
zero, `lpSubMain` zero and `fMdlIntCtls2` `0xFFFFFF00`, so several fields are
indistinguishable from a wrong read.

**Fix.** `a_synthetic_vb_image` builds a complete PE in memory with a `VBHeader`
whose every scalar holds a different value. `AGENTS.md` asks for exactly this:
"Build the state that a test needs inside the test." No corpus number is
pinned anywhere in this file except the four `.vbp` strings, which the plan
pins on purpose.

Breakage 4 measured what this is worth: with `w_form_count` and
`w_external_count` exchanged, 76 tests pass and only the synthetic one fails.

### 5. The identity trap the prompt warns about does not reach the header strings, and does reach the window base

**Found during:** Task 2.

The prompt warns that in both corpus files the relevant section has
`virtual_address == pointer_to_raw_data == 0x1000`, so an address and a file
offset are the same number, and asks for a synthetic image whose section is at
address `0x1000` and file offset `0x400`.

The synthetic image was built. Two things were measured while building it.

**The trap cannot hide a header string bug.** Resolving a header relative
offset by adding it to the header's address and then converting, and resolving
it by adding it to the header's file offset, give the same answer for any
section, not only for one where the two coincide, because the shift from an
address to a file offset is constant inside a section. The two routes differ
only at a section boundary. So a synthetic image proves nothing extra about the
four strings.

**The trap does hide a window base bug.** In `Mandelbrot.exe` the header is at
RVA `0x1760` and at file offset `0x1760`. An implementation of `header_region`
that based its window on the address instead of the file offset would pass
every corpus assertion in this file. In the synthetic image the header is at
RVA `0x1100` and at file offset `0x500`, which differ by `0xC00`, so
`the_header_window_is_based_on_a_file_offset_and_not_on_an_address` has
content. It asserts the base, asserts that the base differs from the RVA, and
asserts that `rva_to_off` of the RVA gives the base.

This finding also explains why breakage 5 was worth three runs. The window base
is what two other tests were silently anchored to.

## Divergences from the plan that are choices, not defects

### Gap 1 keeps its row in the register

RESEARCH.md section 8.6 says to move gap 1 out of the section 11 register. The
plan says to keep the row and mark it `CLOSED`, and gives the reason. The row
is kept. The register shows all eighteen rows with their state, gap 18 is
untouched and still says "Do not implement without a sample", and a paragraph
under the table records the method and its two exceptions.

### Four extra tests

The plan names fifteen behaviours. Twenty two tests are written. The seven
extra ones are: the synthetic scalar test and the synthetic string test
(defect 4 and defect 5), the two window bound refusal tests (defect 1), the
address reading test (defect 2), and the two entry stub refusal tests
(breakage 7). Each one exists because a breakage or a branch review showed
that something had no instrument.

### Two extra commits

The plan names three commits. Five were made. The fourth carries the address
reading test and the fifth carries the two entry stub refusal tests, each on
its own, because each is a separate step from reading the fields and
`AGENTS.md` asks for one change per commit.

## Threat mitigations

| Threat | State |
|---|---|
| T-01-16, the four string offsets spoofed as addresses | Mitigated, and now measured. The offsets are `Off`, they resolve only through `Region::cstr` on the header window, and `Va` and `Off` do not convert into one another. Breakage 2 and breakage 6 show both directions of the confusion producing a refusal rather than a string. The plan's prediction that the failure reports `MZ` is wrong, and the reason is defect 2. |
| T-01-17, an unbounded scan in the header | Mitigated by two bounds, not by one. `max` is `0x104` and mandatory, and the window is 364 bytes. The plan's stated reason, that the window is `0x68` bytes, is wrong, and the correction is defect 1. `a_string_with_no_terminator_inside_the_bound_is_refused` covers it. |
| T-01-18, an unsampled entry stub variant | Mitigated. Only `0x68` is accepted. `an_entry_opcode_of_0x5a_is_refused_because_no_corpus_file_shows_it` and the `0x11` test name the reason in the test name. Breakage 1 made the first one fail. |
| T-01-19, the pushed virtual address spoofed | Mitigated. `region_at_va` is the only route, and it is `Va::to_rva` followed by the one section predicate. `a_pushed_address_below_the_image_base_is_refused_and_the_dos_stub_is_not_read` covers it and also asserts that the file does begin with `MZ`, so the test states what a fallback would have read. |

## Known Stubs

None. Every item the plan names is implemented and covered.

One line of this file has no test and can have none. The window width is
`min(HEADER_WINDOW, hdr.len())`, so `subregion` cannot fail and the `ok_or`
after it is unreachable. It stays because `subregion` returns an `Option` and
this function must not unwrap one. The doc comment in the source says so.

The two refusals that no corpus file can reach do have tests, added in commit
`d8272bc`. See breakage 7.

## Deferred Issues

None.

## Threat Flags

None. This plan adds no network endpoint, no auth path and no file access
path. `grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/` finds
nothing, and the corpus file is reached with `include_bytes!`.

No package was added. `git diff a7e4e24 HEAD -- Cargo.toml Cargo.lock` is
empty.

## Commits

Four commits carry code and its tests, one carries the documentation, as
`AGENTS.md` requires.

| Commit | Subject |
|---|---|
| `521769b` | Reach the VB header from the entry point stub |
| `377bec8` | Read the VBHeader fields and its four header relative strings |
| `52e41f4` | Record the closed 0x58 and 0x5C gap in the structures document |
| `b3d243d` | Prove the executable name offset reaches nothing when it is read as an address |
| `d8272bc` | Cover the two entry stub refusals that no corpus file reaches |

`git rev-list --count a7e4e24..HEAD` is 5.

## Self-Check: PASSED

`crates/deform6/src/vb/header.rs` exists and is 755 lines.
`.planning/research/STRUCTURES.md` holds both new field names. All five commit
hashes resolve in the history of this branch. Every command in the gate table
was run and its exit status is recorded. `git status --porcelain corpus/` is
empty, so no fixture wrote to disk.
