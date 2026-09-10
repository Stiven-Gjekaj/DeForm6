---
phase: 03-forms
reviewed: 2026-09-10T14:47:43Z
depth: standard
files_reviewed: 25
files_reviewed_list:
  - crates/deform6/src/vb/gui.rs
  - crates/deform6/src/vb/controltree.rs
  - crates/deform6/src/vb/vbstr.rs
  - crates/deform6/src/vb/propstream.rs
  - crates/deform6/src/vb/opcodes.rs
  - crates/deform6/src/vb/frx.rs
  - crates/deform6/src/vb/ocx.rs
  - crates/deform6/src/vb/controlinfo.rs
  - crates/deform6/src/vb/project.rs
  - crates/deform6/src/vb/mod.rs
  - crates/deform6/src/error.rs
  - crates/deform6-cli/src/main.rs
  - crates/deform6-cli/tests/cli.rs
  - crates/deform6/tests/form_tracer.rs
  - crates/deform6/tests/differential.rs
  - crates/deform6/tests/ratios.rs
  - crates/deform6/tests/corpus_sweep.rs
  - crates/deform6/tests/refusal.rs
  - crates/deform6/tests/support/frm.rs
  - crates/deform6/tests/support/mod.rs
  - crates/deform6/tests/support_selftest.rs
  - crates/xtask/src/main.rs
  - crates/xtask/src/opcode_table.rs
  - crates/deform6/Cargo.toml
  - crates/xtask/Cargo.toml
  - tests/ratios.toml
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 03-forms: Code Review Report

**Reviewed:** 2026-09-10T14:47:43Z
**Depth:** standard
**Files Reviewed:** 25 (plus `tests/ratios.toml`)
**Status:** issues_found

## Summary

This phase adds the form and control tree recovery path: the GUI table, the
control tree scope-byte walk, string and property decoding, `.frx` blob
extraction, OCX/CLSID join, `ControlInfo`/event table recovery, and the
composed `Report.forms` surface, plus the CLI printer and the differential
and pinned-ratio test gates.

I traced every arithmetic operation on a file-derived offset or length
across all ten `src/` files in scope and found the hostile-file discipline
AGENTS.md demands (no panic, no `unwrap`/`expect` on file data, no slice
indexing on file-derived indices, `checked_add`/`checked_mul` before any
offset sum, allocation size checked against the real file length before it
is used) applied consistently. I found no path where a crafted file reaches
an unchecked index, an unchecked allocation, or an arithmetic overflow. I
verified this by hand and by grep: zero `.unwrap()`/`.expect()`/`panic!`
and zero raw `[index]` or bare `+` calls on offset-shaped variables in any
production code path of the ten reviewed `src/` files. The control tree
cannot cycle by construction (`ControlNode.parent` is always an index
strictly less than the node's own index, since `idx = nodes.len()` before
each push), and every loop that walks file-declared structures is bounded
either by a `MAX_*` constant this repository chooses, or by a count that is
checked against the region's own real length before it is used as a loop
bound.

I also traced the measurement-honesty rule across `differential.rs`,
`ratios.rs`, and `support/`: the "declared" side of every corpus-wide
comparison comes from an independent second reader (`support::vbp`,
`support::frm`, `support::source`), never from a second call into
`deform6`, and the `ratios.toml` `declared` column is explicitly and
correctly labelled as the binary's own declared count, not the source's,
so it is not misrepresented as ground truth. No test in the reviewed set
asserts a tautology or measures the tool against its own output as if it
were verification; the one true self-agreement round trip
(`xtask/opcode_table.rs`'s `serialize_table`/`OpcodeTable::parse` round
trip) is explicitly labelled as such in its own doc comment and is not
counted toward any recovery claim.

The findings below are all Warning or Info tier: two maintainability/
design-smell issues, one silent-failure-mode gap in a developer-tool
arithmetic path, one text-style violation, and one acknowledged dead-code
arm.

## Warnings

### WR-01: The `Box::leak`-based `damaged()` helper is triplicated, and each copy leaks unboundedly under repeated hostile input

**File:** `crates/deform6/src/vb/gui.rs:362-365`, `crates/deform6/src/vb/controltree.rs:394-397`, `crates/deform6/src/vb/frx.rs:275-278`

**Issue:** All three modules carry a byte-for-byte identical private helper:

```rust
fn damaged(message: String) -> Refusal {
    let leaked: &'static str = Box::leak(message.into_boxed_str());
    Refusal::Damaged(leaked)
}
```

This is a deliberate, well-documented escape hatch (each module's doc
comment explains the `&'static str` constraint on `Refusal::Damaged`), and
for a single-shot CLI run leaking one short string before the process exits
is harmless. But `AGENTS.md` requires fuzzing to be part of the gate, and a
fuzz target (or any long-running host that embeds this library, for
example a batch scanner) calls `inspect()` in a tight loop over many
malformed inputs. Every hostile-input refusal that reaches `damaged()` — a
bad `lStructSize`, a tiling mismatch, a scope run with no terminator, a
`.frx` cursor overflow — permanently leaks one heap-allocated string that
is never reclaimed until the process exits. Over a long fuzzing session
this is unbounded memory growth, which is exactly the class of "the file
is hostile" failure mode AGENTS.md's hostile-input section exists to catch
in a different guise (a resource-exhaustion path rather than a crash path).

**Fix:** Extract one shared, crate-visible helper (for example
`pub(crate) fn damaged(message: String) -> Refusal` in `error.rs` itself,
next to `Refusal`) instead of three private copies, so a future decision to
widen `Refusal::Damaged` to carry an owned `String` (removing the leak
entirely) touches one call site instead of three:

```rust
// error.rs
pub(crate) fn damaged(message: String) -> Refusal {
    let leaked: &'static str = Box::leak(message.into_boxed_str());
    Refusal::Damaged(leaked)
}
```

### WR-02: `close_walk`'s pop-count bounding is dead code that reads as a safety check it does not perform

**File:** `crates/deform6/src/vb/controltree.rs:631-658`

**Issue:** `close_walk` is called only immediately before the walk's own
`break` (from the `EndForm` arm and from the `!block_fits` arm), and its
`stack: &mut Vec<usize>` parameter is never read again after it returns —
the function's return value carries nothing derived from `stack`, and the
caller (`walk`) does not inspect `stack` after the loop ends. This means
the entire `bounded_pops`/`stack.pop()` block has no observable effect on
the function's result:

```rust
let bounded_pops = u8::try_from(stack.len().saturating_sub(1))
    .unwrap_or(pops)
    .min(pops);
for _ in 0..bounded_pops {
    stack.pop();
}
```

This differs materially from `apply_pops`, used earlier in the same
`walk()` loop, which *does* refuse (`Refusal::Damaged`) when a pop count
would empty the stack. A reader who sees `close_walk` bounding `pops`
against `stack.len()` reasonably infers the same validation discipline
applies at the terminal transition; it does not; an over-large pop count
on the terminal `EndForm`/no-more-controls run is silently accepted with
no `Defect` recorded, and the bounding computation that looks like it
guards against that is inert. The trailing-tail check (`MAX_UNEXPLAINED_TAIL`)
is the only thing `close_walk` actually enforces.

**Fix:** Either drop the `stack` parameter and the dead pop loop entirely
(since it has no effect), or, if the intent was genuinely to detect and
report an over-large terminal pop count, push a `Defect` when
`pops as usize > stack.len().saturating_sub(1)` the same way `apply_pops`
does, instead of silently truncating.

### WR-03: `format_ratio` divides by `declared` with no zero guard, so a future zero-procedure program would silently write `ratio = NaN` into the committed pin file

**File:** `crates/deform6/tests/ratios.rs:362-365`

**Issue:**

```rust
pub(crate) fn format_ratio(recovered: u32, declared: u32) -> String {
    let ratio = f64::from(recovered) / f64::from(declared);
    format!("{ratio:.2}")
}
```

No corpus program currently has `declared == 0` (verified: `grep -B1
'declared = 0$' tests/ratios.toml` finds nothing), so this is not reachable
today. But `f64` division by zero does not panic in Rust; it produces `NaN`
or `inf`, and `format!("{:.2}", f64::NAN)` renders the literal text `NaN`.
If a future corpus program is added whose kept objects declare zero
procedure slots (a legitimate outcome — see the module doc comment's own
note that 13 of the 44 current programs already recover zero procedures,
just not zero *declared* slots), `xtask update-ratios` would silently
write `ratio = NaN` into the committed `tests/ratios.toml` rather than
failing loudly, and `the_gate_passes_on_the_committed_file` would then
compare that text against itself and pass, hiding the degenerate case
rather than surfacing it. `AGENTS.md`'s "give the number that can be
proved" spirit argues for an explicit guard here rather than relying on the
current corpus's shape to keep the denominator non-zero.

**Fix:** Guard the zero-declared case explicitly, for example returning
`"n/a"` or refusing the write with a named reason, rather than trusting
`f64` division to produce a printable-looking-but-meaningless string.

## Info

### IN-01: Ten em-dash uses in `controltree.rs` violate the repository's own text-style rule

**File:** `crates/deform6/src/vb/controltree.rs:18, 21, 412-413, 528, 583, 666, 803, 1088-1089`

**Issue:** `AGENTS.md` states plainly, twice (once in "How to write", once
repeated in the reviewing agent's own project rules): "Do not use an
em-dash." `controltree.rs` is the one file in this phase's scope that uses
one, ten times, for example:

- Line 18: `"[walk] implements the recommendation literally — read 0xFF,"`
- Line 583: `` "not enough room left" — it is `` ``
- Line 803: `` "does not hold — the .vbp" ``

Every other file in this review's scope is clean (verified by grepping the
UTF-8 em-dash byte sequence across all 22 `src`/`tests` files in scope; only
`controltree.rs` matches, with a count of exactly 10).

**Fix:** Replace each em-dash with a period, a comma, or a parenthetical,
matching the plain, short-sentence style the rest of the module already
uses.

### IN-02: An acknowledged-unreachable match arm in `walk_properties` could be designed out instead of documented around

**File:** `crates/deform6/src/vb/propstream.rs:756-772`

**Issue:** The `None` branch of `entry.payload.fixed_width()` matches on
`entry.payload`, and one arm is:

```rust
PayloadType::Byte
| PayloadType::Boolean
| PayloadType::Integer
| PayloadType::Long
| PayloadType::Single => {
    // `fixed_width` gives `Some` for all five of these;
    // this arm is unreachable, and it names no behaviour.
    ...
}
```

The comment is honest about the arm being unreachable (a live invariant:
`PayloadType::fixed_width` gives `Some` for exactly these five variants),
so this is not a correctness bug, and it costs nothing at runtime beyond a
few bytes of unreachable object code. It is dead code kept alive only to
satisfy match exhaustiveness against a `None` arm that structurally can
never hold one of these five variants.

**Fix (optional, low priority):** Consider having `PayloadType` expose the
"fixed vs. variable" distinction as its own two-variant type (or an
associated function returning an enum with only the four variable shapes),
so the compiler enforces the invariant the comment currently asserts by
hand, rather than relying on a human to keep the comment in sync with
`fixed_width`'s own match arms.

---

_Reviewed: 2026-09-10T14:47:43Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
