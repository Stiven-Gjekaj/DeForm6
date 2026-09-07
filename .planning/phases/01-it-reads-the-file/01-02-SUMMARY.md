---
phase: 01-it-reads-the-file
plan: 02
subsystem: read
tags: [region, newtypes, bounds-checking, overflow, compile-fail-proof]
status: complete

requires:
  - the cargo workspace and the lint wall from plan 01-01
  - the read/region.rs stub and the pub mod line from plan 01-01
provides:
  - Off, Rva and Va, three offset newtypes with no Add and one checked bridge
  - Region, a bounded window with no infallible accessor
  - Region::file_offset, which plan 01-03's Site reads
  - scripts/prove-region-wall.sh
affects:
  - every later parser in the project reads bytes through Region

tech_stack:
  added: []
  patterns:
    - every read returns Option, never Result, because this layer holds no names
    - every conversion between u32 and usize is a try_from with a saturating
      fallback, never an as cast
    - a shape that the type refuses is proved by compiling it and reading the
      rustc error code, because no lint and no test can see it

key_files:
  created:
    - scripts/prove-region-wall.sh
  modified:
    - crates/deform6/src/read/region.rs
    - .github/workflows/gate.yml

decisions:
  - Off::index is committed with its first caller in task 2, not in task 1. A
    private method with no caller stops the gate on dead_code, and D-08 forbids
    a lint allowance in library code.
  - Region::cstr clamps its scan to the smaller of max and the bytes that
    remain, rather than refusing when max runs past the end of the window. A
    short file whose last field does hold a NUL still reads, and the scan is
    still bounded by max.
  - Region derives Clone, Copy and Debug. It does not derive PartialEq, so a
    test asserts is_none rather than comparing an Option<Region>.

metrics:
  duration: 1 session
  completed: 2026-09-07

actuals:
  tokens: 5700
  tasks: 3
  commits: 4
plan_head_before: 722c7e05fa402e0ba449408a097505b79837b14b
---

# Phase 01 Plan 02: Region, Off, Rva and Va Summary

A bounded window on the bytes of a hostile file, three offset newtypes that
cannot be added to one another, and a committed script that shows the compiler
refusing the three shapes a caller must not be able to write.

## What this plan built

`crates/deform6/src/read/region.rs` holds four public types and names nothing
outside the standard library. The file holds exactly one `use` item, and it is
`use super::{Off, Region, Rva, Va};` inside the test module. `grep -c '^use '`
on the file is 0, because that one item is indented. `grep -n 'use '` matches
two lines, and the other is a doc comment that holds the English word.

`Off`, `Rva` and `Va` carry the derive set the plan names and nothing else. No
`Add`, no `Sub`, no `From<u32>`, no `Deref`. Every `checked_add` takes a bare
`u32` length on the right-hand side, so adding two offsets has no signature
that accepts it. Each `checked_add` and `checked_sub` is a `const fn` written
with a `match`, because `Option::map` is not a `const fn` on 1.97.1.

`Va::to_rva` is the only bridge between the three integer spaces and it is a
`checked_sub`. This is the reason the three types exist. `STRUCTURES.md`
section 13 records a survey that read `VBHeader + 0x58` as a virtual address,
resolved it through the section table, and got the string `MZ` out of the DOS
stub with no error. The field is a byte offset from the header base, not a
virtual address. A parser that carries only `u32` cannot tell the two apart.

`Region` holds `bytes: &'a [u8]` and `base: Off` and exposes neither. `take` is
the only route out, it takes a length, and it returns `Option`. `file_offset`
converts a window-relative offset into an absolute file offset with a
`checked_add`, and `subregion` recomputes the base from it, so a defect three
levels down still names a byte offset in the file.

`scripts/prove-region-wall.sh` writes one probe at a time into
`crates/deform6/examples/region_probe.rs`, compiles it, requires the compiler
to refuse it with a named error code on the probe file, removes the probe, and
then runs the three gate commands. `.github/workflows/gate.yml` runs it as a
fourth step.

## The three rustc error codes the script observed

Read from this toolchain, `rustc 1.97.1 (8bab26f4f 2026-07-14)`. All three are
the codes the plan predicted. Nothing had to be re-pinned.

| Shape | Code | What rustc said |
|---|---|---|
| reaching the bytes of a `Region` directly | `E0616` | ``field `bytes` of struct `Region` is private: private field`` |
| adding two `Off` values with the plus operator | `E0369` | ``cannot add `deform6::read::region::Off` to `deform6::read::region::Off` `` |
| indexing a `Region` with square brackets | `E0608` | ``cannot index into a value of type `Region<'_>` `` |

The observed output of `sh scripts/prove-region-wall.sh`, exit status 0, with
the 18 passing test lines of the gate removed for length:

```
The probe goes to crates/deform6/examples/region_probe.rs, one shape at a time.

PASS  E0616  reaching the bytes of a Region directly
PASS  E0369  adding two Off values with the plus operator
PASS  E0608  indexing a Region with square brackets

Checked 3 shapes.
Each one is refused by the type, not by a lint, so none of the three gate
commands can catch a change that lets one in. That is why this script
runs in the gate as a fourth step.

Removing the probe and running the gate.
...
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
...
The type refuses every shape, and the tree it leaves behind is clean.
```

The script matches the error code against a line that also names
`region_probe.rs`. That proves the probe line produced the error, and not some
other part of the workspace.

## The five deliberate breakages

`AGENTS.md` says to break the thing a test covers and watch it fail. The plan
asks for four. A fifth was added, because the proof script is itself a test and
nothing else checks that it can fail. None of the five was committed.

### 1. `Off::checked_add` becomes a `wrapping_add` that always returns `Some`

Body replaced with `Some(Self(self.0.wrapping_add(n)))`.

Failed: `read::region::tests::off_checked_add_refuses_a_sum_that_leaves_a_u32`.

```
assertion `left == right` failed
  left: Some(Off(0))
 right: None
```

`Some(Off(0))` is exactly the fault the type exists to refuse. `u32::MAX` plus
1 wraps to 0, and 0 is inside every window, so a naive bounds check would pass
and the parser would read real bytes from offset 0. The other six tests still
passed, so the failure was local to the method that was broken.

### 2. `Va::to_rva` becomes a `wrapping_sub` that always returns `Some`

Body replaced with `Some(Rva(self.0.wrapping_sub(image_base)))`.

Failed:
`read::region::tests::va_to_rva_refuses_an_address_below_the_image_base`.

```
assertion `left == right` failed
  left: Some(Rva(4290777088))
 right: None
```

`4290777088` is `0xFFC0_1000`. A virtual address of `0x1000` under an image
base of `0x0040_0000` wraps into a huge RVA. The point is that it is not
obviously wrong: it is a number the section table would try to resolve.

### 3. `Region::take` checks only the start offset, never the end

Body replaced with `self.bytes.get(at.index()..)`, so the length is ignored.

Five tests failed. The one the plan names is
`read::region::tests::take_refuses_a_length_that_runs_one_byte_past_the_end`:

```
assertion `left == right` failed
  left: Some([255, 255, 254, 255])
 right: None
```

The read asked for 5 bytes at offset 4 of an 8-byte window and got 4 real bytes
back. That is a short read dressed as a success, which is worse than a refusal.

The four other failures were
`take_refuses_a_length_that_wraps_a_u32_and_does_not_read_short`
(`left: Some([])`), `an_empty_region_has_length_zero` (`left: Some([])`),
`every_fixed_width_reader_agrees_with_the_buffer` (`left: None`,
`right: Some(513)`) and `off_le_and_va_le_give_the_typed_value` (`left: None`,
`right: Some(Off(67305985))`). The last two failed because the fixed-width
readers use `try_into` on a fixed-size array, and a slice of the wrong length
gives `Err`, which `.ok()?` turns into `None`. That is the second bound under
the first one.

### 4. `Region::subregion` keeps the parent base instead of recomputing it

`base: self.file_offset(at)?` replaced with `base: self.base`.

Failed: `read::region::tests::subregion_rebases_so_file_offset_stays_absolute`.

```
assertion `left == right` failed
  left: Some(Off(5984))
 right: Some(Off(6000))
```

`5984` is `0x1760` and `6000` is `0x1770`. The subregion was taken at offset
`0x10` of a window based at `0x1760`, so every defect inside it would have
named a byte offset 16 bytes too early. A defect that names the wrong offset is
worse than no offset, because a reader trusts it.

`file_offset_refuses_a_sum_that_leaves_a_u32` also failed, because the broken
`subregion` no longer propagates the `None` from `file_offset`.

### 5. `Region::bytes` is made public, and the proof script must notice

`bytes: &'a [u8]` changed to `pub bytes: &'a [u8]`.

`sh scripts/prove-region-wall.sh` exited 1 and printed:

```
FAIL  E0616  reaching the bytes of a Region directly
      The compiler did not refuse this shape with E0616.
      A shape that compiles is a hole in the type. Repair the
      type in crates/deform6/src/read/region.rs. Do not relax
      this script.
      What the compiler said:
         Compiling deform6 v0.1.0 (...)
          Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
PASS  E0369  adding two Off values with the plus operator
PASS  E0608  indexing a Region with square brackets
```

Two facts this measured. First, the failure is local: the other two probes
still passed. Second, the whole gate stayed green while the hole was open.
`cargo fmt`, `cargo clippy --all-targets -- -D warnings` and
`cargo test --workspace` all pass with `bytes` public, because a public field
is not a bad shape until somebody writes it, and nothing in the workspace
writes it. That is the reason this script exists and the reason it is now a
step in the gate.

## The gate

Every command run on the committed tree at `8209066`, with a clean working
tree.

| Command | Exit |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0 |
| `sh scripts/prove-region-wall.sh` | 0 |
| `sh scripts/prove-lint-wall.sh` | 0 |

`cargo test --workspace` reports `18 passed; 0 failed` for the library, and
`0 passed; 0 failed` for the CLI binary and for the doc tests.

The plan's `<verification>` and `<success_criteria>` also ask for these, and
each was run:

- `grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/` finds nothing.
- `grep -n 'use ' crates/deform6/src/read/region.rs` matches two lines. One is
  `use super::{Off, Region, Rva, Va};` inside the test module. The other is a
  doc comment on `Off::index` that holds the English word "use". There is no
  other `use` item, and no external crate is named.
- `test ! -e crates/deform6/examples/region_probe.rs` passes after the script.
- `grep -q 'prove-region-wall.sh' .github/workflows/gate.yml` passes.

## Deviations from Plan

### 1. `Off::index` moved from task 1 to task 2, because task 1 could not pass the gate

**Found during:** Task 1.
**Rule:** 3, a blocking issue.

**Issue:** The plan puts `Off::index` in task 1 and `Region` in task 2. `index`
is private and `Region` is its only caller, so at the end of task 1 it has no
caller. Measured:

```
error: method `index` is never used
  --> crates/deform6/src/read/region.rs:75:8
   = note: `-D dead-code` implied by `-D warnings`
```

`cargo clippy --all-targets -- -D warnings` exits non-zero. The gate must pass
before every commit, so task 1 as written cannot be committed.

**Fix:** `Off::index` is committed in task 2, with its first caller. The two
alternatives were both rejected. `#[allow(dead_code)]` is a lint allowance in
library code, which D-08 forbids. Making `index` public widens a signature the
plan says not to widen, and it hands a caller a raw index, which is the whole
thing `Region` exists to prevent.

**Cost:** None to the shape of the code. The final file is identical to what
the plan describes. Only the commit boundary moved, and it moved toward the
rule in `AGENTS.md` that code goes in the same commit as the thing that uses
it.

### 2. `Region::cstr` clamps its scan instead of refusing a `max` that runs past the end

**Found during:** Task 2.
**Rule:** 2, missing critical functionality.

**Issue:** The obvious body is `let window = self.take(at, max)?;` followed by
the NUL scan. That refuses every string field whose `max` runs past the end of
the window. `RESEARCH.md` section 3.3 says to pass `0x104` for a path-like
field. A file of any realistic size has header string fields within `0x104`
bytes of its end, and a truncated file certainly does. The strict form returns
`None` for a field that holds a perfectly good NUL-terminated name.

**Fix:** The scan window is `min(max, the bytes that remain)`. The scan is
still bounded by `max`, so threat T-01-06 is still mitigated: a file with no
NUL cannot make the scan run to the end of the window. A short file whose last
field does hold a NUL now reads.

**Verification:** `cstr_refuses_when_no_nul_appears_inside_max` covers both
halves. A 4-byte buffer with no NUL returns `None`. A buffer `ABC\0` returns
`None` at `max = 3`, because the NUL is one byte outside the bound, and returns
`ABC` at `max = 4`.

**Files modified:** `crates/deform6/src/read/region.rs`.

### 3. The test module uses byte string literals, because clippy demands it

**Found during:** Task 2.
**Rule:** 3, a blocking issue.

**Issue:** `let buf = [b'A', b'B', b'C', b'D'];` fires
`clippy::byte_char_slices`, "can be more succinctly written as a byte str".
That lint is in the default `style` group, and the wall runs `-D warnings`, so
it is an error. It is not in the `[workspace.lints]` table, so it is a lint the
plan did not anticipate.

**Fix:** The three test buffers are written as `*b"ABCD"`, `*b"ABC\0"` and
`*b"Main\0XYZ"`. No allowance was added.

**Files modified:** `crates/deform6/src/read/region.rs`.

### 4. `Region` derives `Clone`, `Copy` and `Debug`, and a test uses `is_none`

**Found during:** Task 2.

**Issue:** `RESEARCH.md` section 3.1 lists the methods of `Region` and does not
give its derive set. A first draft of
`file_offset_refuses_a_sum_that_leaves_a_u32` wrote
`assert_eq!(r.subregion(Off::new(3), 1), None);` and did not compile:

```
error[E0369]: binary operation `==` cannot be applied to type `Option<Region<'_>>`
```

**Fix:** The test asserts `r.subregion(Off::new(3), 1).is_none()`. `PartialEq`
was deliberately not derived. A `Region` is a window on a file, and two windows
that hold equal bytes at different offsets are not the same window, so the
derived comparison would be wrong more often than it would be useful. `Clone`,
`Copy` and `Debug` are derived, because a two-field `Copy` struct that later
parsers pass by value needs them.

**Files modified:** `crates/deform6/src/read/region.rs`.

### 5. The proof script prints its conclusion only when it is true

**Found during:** Task 3, breakage 5.

**Issue:** The first draft printed "Checked 3 shapes. Each one is refused by
the type" before it checked the failure flag. Breakage 5 made it print that
sentence in a run where one shape was **not** refused. `AGENTS.md` says to
measure the thing you tell the human.

**Fix:** "Checked 3 shapes." always prints. The claim that each one is refused
prints only after the failure flag is found to be zero.

**Files modified:** `scripts/prove-region-wall.sh`.

### 6. The comment on the lint wall step in the workflow was made false

**Found during:** Task 3.

**Issue:** `.github/workflows/gate.yml` says the lint wall step "comes last".
Adding the Region wall step after it makes that sentence untrue.

**Fix:** The comment now says the proof steps come after the three gate
commands, and it names the reason, which is that each script writes a probe
into the source tree. The wording change is in the same commit as the step that
made it necessary.

**Files modified:** `.github/workflows/gate.yml`.

## Findings that correct the sources

### `cargo build` appears in this plan, and `AGENTS.md` forbids it

The plan tells the script to run `cargo build --examples`. `AGENTS.md` says
"`cargo test --workspace` is the build check, never `cargo build`". The two do
not conflict, and the script carries a comment saying so. The reason
`AGENTS.md` forbids `cargo build` is that it does not compile a `#[cfg(test)]`
module, so it is a weaker gate. The script is not the gate. The question it
asks each probe is "does this shape reach the compiler", and `build` answers
that. The three gate commands run at the end of the same script.

### The lint wall and the type wall catch disjoint sets, and breakage 5 measured it

`scripts/prove-lint-wall.sh` and `scripts/prove-region-wall.sh` are not two
ways of saying the same thing. With `Region::bytes` public, all three gate
commands and the lint wall proof stayed green. Only the Region wall proof went
red. A shape that does not compile never reaches clippy, so no lint can guard
it, and no test can call it. The compile-fail probe is the only instrument that
sees this class of change.

## Threat Flags

None. This plan adds no network endpoint, no auth path and no file access path.
`grep -rnE 'std::fs|std::path|PathBuf' crates/deform6/src/` finds nothing.

The four mitigations the plan's threat register names are in place and each is
covered:

- **T-01-04**, a wrapped offset plus length. `Off::checked_add` returns `None`
  at `u32::MAX`, `Region::take` refuses the wrapping length, there is no `Add`
  to write the wrapping form with, and probe two proves the absence.
  Breakage 1 and breakage 3 both made the covering test fail.
- **T-01-05**, a read past the end. Every read bounds-checks the real slice
  before it touches a byte, and probes one and three prove that no accessor
  bypasses it. Breakage 3 and breakage 5 both made a covering check fail.
- **T-01-06**, an unbounded string scan. `max` is a mandatory argument and the
  scan is bounded by it. `cstr_refuses_when_no_nul_appears_inside_max` covers
  it. See deviation 2 for the clamping the mitigation still permits.
- **T-01-07**, a virtual address below the image base. `Va::to_rva` is a
  `checked_sub`. Breakage 2 made the covering test fail with `Rva(0xFFC0_1000)`.

## Known Stubs

None in this plan's files. `crates/deform6/src/read/region.rs` is complete and
holds no `TODO`, no `FIXME` and no placeholder. Every method in
`RESEARCH.md` section 3.1 is implemented and covered by a test.

The stubs that plan 01-01 recorded for other files are untouched. This plan
edited no file outside its three.

## Commits

| Commit | Subject |
|---|---|
| `d8c3356` | Add the Off, Rva and Va offset newtypes |
| `ac7dd72` | Add the Region bounded window with no infallible accessor |
| `ab9867d` | Add a script that proves the Region and the newtypes refuse three shapes |
| `8209066` | Run the Region wall proof in the continuous integration gate |

`git rev-list --count 722c7e0..HEAD` is 4.

## Self-Check: PASSED

All four files exist on disk. All four commit hashes are in the history of
this branch. The five commands in the gate table were each run and each exit
status is recorded above.
