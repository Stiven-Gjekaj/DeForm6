---
phase: 06-version-1-0
reviewed: 2026-09-14T19:12:14Z
depth: standard
files_reviewed: 35
files_reviewed_list:
  - .github/workflows/gate.yml
  - CHANGELOG.md
  - Cargo.lock
  - Cargo.toml
  - LICENSES.md
  - README.md
  - crates/deform6-cli/Cargo.toml
  - crates/deform6/Cargo.toml
  - crates/deform6/schema/report.schema.json
  - crates/deform6/src/error.rs
  - crates/deform6/src/report.rs
  - crates/deform6/src/vb/controlinfo.rs
  - crates/deform6/src/vb/controltree.rs
  - crates/deform6/src/vb/frx.rs
  - crates/deform6/src/vb/gui.rs
  - crates/deform6/src/vb/header.rs
  - crates/deform6/src/vb/object.rs
  - crates/deform6/src/vb/ocx.rs
  - crates/deform6/src/vb/opcodes.rs
  - crates/deform6/src/vb/privateobj.rs
  - crates/deform6/src/vb/project.rs
  - crates/deform6/src/vb/propstream.rs
  - crates/deform6/src/write/code.rs
  - crates/deform6/src/write/mod.rs
  - crates/deform6/src/write/model.rs
  - crates/deform6/src/write/vbp.rs
  - crates/deform6/tests/differential.rs
  - crates/deform6/tests/gitattributes.rs
  - crates/deform6/tests/ratios.rs
  - crates/deform6/tests/schema.rs
  - crates/xtask/Cargo.toml
  - crates/xtask/src/licences.rs
  - crates/xtask/src/main.rs
  - scripts/check-claim-surface.sh
  - scripts/check-release-version.sh
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: fixed
resolution: 5 of 6 findings fixed in commits eb0835c, 129e0e7, a91612c, 7550a4e, fd72e10. IN-01 deferred by decision.
---

# Phase 06: Code Review Report

**Reviewed:** 2026-09-14T19:12:14Z
**Depth:** standard
**Files Reviewed:** 35
**Status:** issues_found

## Summary

Phase 6 ships the release mechanics for v1.0.0: a JSON Schema for `report.json`
with a doctored-report refusal test, a `.gitattributes` audit test, an
`xtask licences` subcommand, a claim-surface scanner, and a release-tag
checker. The JSON Schema, the schema/gitattributes tests, `licences.rs`, and
`check-release-version.sh` all hold up under adversarial reading: the schema
genuinely constrains (all 17 `DefectKind` variants present, each with
`additionalProperties: false` and correctly-typed/bounded fields matching
`error.rs`'s real Rust types), the doctored-report test builds every negative
case from a real, passing report rather than a hand-built value, and
`licences.rs` never panics on a malformed `cargo metadata` shape and renders
deterministically (sorted `BTreeMap` licence counts, explicit `(name, version)`
sort on the package list).

The one real defect is in `scripts/check-claim-surface.sh`, the phase's
flagship "does the release overclaim" gate. Its detection is line-based
(`grep` without multi-line mode), but the exact prose style this repository's
own `README.md` uses: hard-wrapped sentences that routinely split a clause
across two physical lines (confirmed by reading the file directly, e.g. line
3-4, line 19-20): defeats all six forbidden-shape regexes whenever a future
violation happens to wrap. This was proven experimentally, not asserted: a
two-line sentence containing both "recover" and "statement" is invisible to
the `stmt-fwd` shape that is supposed to catch exactly that claim. Two
narrower detection gaps in the same script (the "compilable" shape misses
plain "compiles"/"compiled"; the percentage-word shape requires a digit and
misses a spelled-out number) round out the findings.

## Critical Issues

### CR-01: The claim-surface scanner cannot see a claim that wraps across two lines, and this repository's own prose style wraps routinely

**File:** `scripts/check-claim-surface.sh:47-52` (the `SHAPES` table, scanned via `grep -inE` at line 157)
**Issue:** Every one of the six forbidden-shape regexes is matched with `grep -inE`, which operates strictly within a single physical line: it never joins text across a newline. `README.md`, the primary scanned source, is not written as one sentence per line; it is hard-wrapped prose where a single sentence routinely spans two physical lines (confirmed directly, e.g. `README.md:3-4`: `"DeForm6 reads a compiled Visual Basic 6 executable and writes back a Visual\nBasic project."`, and `README.md:19-20`: `"...naming what the run recovered and how\n  sure it is of each fact."`). A future edit that phrases an overclaim the same way the rest of this file is already written: wrapped across a line boundary: passes the scanner silently. This was reproduced directly:

```
$ printf 'DeForm6 will, after more work in a future release, recover every\nstatement from the compiled executable directly and precisely.\n' > /tmp/t.txt
$ grep -inE 'recover[a-z]*[^.]{0,40}statement' /tmp/t.txt
(no output: the line "recover every statement..." is never assembled because "recover" is on line 1 and "statement" is on line 2)
```

The same gap applies to all six shapes (`pct-sign`, `pct-word`, `stmt-fwd`, `stmt-rev`, `compilable`, `decompile-src`) and to all three scanned sources, since none of them are guaranteed to keep a sentence on one line. This defeats the script's own stated purpose ("the claim surface holds no claim the measurement does not support") in exactly the document style it is built to police, and the self-test in Stage 4 does not exercise this failure mode at all: every planted probe line is a single unwrapped sentence appended as one `printf` line, so the self-test's 18/18 pass rate proves nothing about the wrapped case.
**Fix:** Before scanning, collapse each source to a per-sentence (or per-paragraph) text with hard line breaks removed, e.g. join lines with a single space before running the shape regexes:

```sh
# Join wrapped prose into one line per blank-line-delimited paragraph before
# scanning, so a sentence split across a hard line wrap is still visible to
# a single-line regex.
awk 'BEGIN{buf=""} /^$/{if (buf!="") print buf; buf=""; print ""; next} {buf = (buf=="" ? $0 : buf " " $0)} END{if (buf!="") print buf}' "$sfile" >"$WORK/joined.txt"
grep -inE "$shape_re" "$WORK/joined.txt"
```

Add a Stage 4 probe that plants a violation split across two lines (matching this repository's real wrap style) and requires it to fire, so the self-test actually proves the fix.

## Warnings

### WR-01: The "compilable" shape misses the plainer, more natural phrasing "compiles" / "compiled"

**File:** `scripts/check-claim-surface.sh:51`
**Issue:** The shape regex is `compilable|recompilable|compile.ready`. It requires the `-able` suffix or the literal "ready" nearby. The most natural way to write this exact forbidden claim: "the recovered project compiles in the Visual Basic 6 IDE without changes": contains neither "compilable" nor "compile ready" and is not caught:
```
$ printf 'The recovered project compiles in the Visual Basic 6 IDE without changes.\n' | grep -inE 'compilable|recompilable|compile.ready'
(no output)
```
**Fix:** Broaden the shape to catch the verb form directly, e.g. `compil(e|es|ed|able)|recompil(e|es|ed|able)`, and add a Stage 4 probe using the plain "compiles" phrasing so the self-test actually exercises it.

### WR-02: The percentage-word shape requires a leading digit and misses a spelled-out number

**File:** `scripts/check-claim-surface.sh:48`
**Issue:** `pct-word`'s regex is `[0-9]+[[:space:]]*(per cent|percent)`, so it only fires when a digit precedes the word. "DeForm6 recovers ninety percent of every project it reads." holds none of the six shapes and passes silently:
```
$ printf 'DeForm6 recovers ninety percent of every project it reads.\n' | grep -inE '[0-9]+[[:space:]]*(per cent|percent)'
(no output)
```
**Fix:** Drop the leading `[0-9]+` requirement, or add a companion shape that matches `\b(percent|per cent)\b` on its own (case-insensitive), and add a Stage 4 probe for the spelled-out case.

### WR-03: The 40-character window in `stmt-fwd` / `stmt-rev` / `decompile-src` is narrow enough that a modestly elaborated sentence escapes detection

**File:** `scripts/check-claim-surface.sh:49-50, 52`
**Issue:** These three shapes bound the gap between the trigger word and the target word to `[^.]{0,40}`. A realistic, only slightly more verbose rephrasing of the exact claim the shape is meant to catch exceeds 40 characters and is missed, e.g.: "DeForm6 can, based on the byte patterns already proven for this format, recover the original statement." (the span between "recover" and "statement" here is well over 40 characters). This is a narrower version of CR-01 (same root cause: a single-line, fixed-width regex against free prose) but reproducible even without a line wrap.
**Fix:** Loosen the window (e.g. to 120 characters) or switch to a two-pass check: first find the trigger word, then search the surrounding paragraph (not just the next 40 characters) for the target word. Pair with CR-01's paragraph-join fix so both problems are addressed by the same preprocessing step.

## Info

### IN-01: Only one corpus program's report is scanned for the "report vocabulary" claim surface

**File:** `scripts/check-claim-surface.sh:134-142`
**Issue:** The `report` source is built from a single `extract` run over `corpus/public-domain/PassGen/PassGen.exe`, plus the three literal confidence words appended by `printf`. This is a reasonable proxy since `Defect`/`Evidence` message text is built from `&'static str` literals in `error.rs`/`report.rs` rather than free-form program-specific text, so the risk of a claim-shaped phrase appearing only in a defect variant PassGen.exe never triggers is low, but it is not zero (e.g. a `note: Option<String>` field on `Evidence`, which does carry ad hoc text in some call sites).
**Fix:** No action required for this release; if `Evidence.note` or a similar free-text field ever becomes more expressive, consider scanning it across all 44 corpus reports the way `tests/schema.rs` already does, rather than one representative program.

### IN-02: The `compile.ready` alternative in the "compilable" shape uses an unescaped `.` metacharacter

**File:** `scripts/check-claim-surface.sh:51`
**Issue:** `compile.ready` is an unescaped regex, so the `.` matches any single character, not just a space or hyphen. This is over-permissive rather than under-permissive (it would also fire on "compileXready"), so it does not weaken detection, but it is inconsistent with the otherwise careful, explicit shapes used elsewhere in the same table (e.g. `[[:space:]]*` is spelled out explicitly for whitespace elsewhere in the same table).
**Fix:** Replace with `compile[- ]ready` if the intent is literally "compile" + separator + "ready".

---

_Reviewed: 2026-09-14T19:12:14Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

---

## Resolution

Applied on 2026-09-14, after the review, by `gsd-code-fixer`. Every fix landed in
`scripts/check-claim-surface.sh` as its own commit.

| Finding | Severity | Outcome | Commit |
|---------|----------|---------|--------|
| CR-01 wrapped claim escapes the line-based scan | Critical | Fixed | `129e0e7` |
| WR-01 `compilable` misses plain compile/compiles/compiled | Warning | Fixed | `a91612c` |
| WR-02 `pct-word` needs a leading digit | Warning | Fixed | `7550a4e` |
| WR-03 the forty character gap window is too narrow | Warning | Fixed | `fd72e10` |
| IN-02 unescaped `.` in `compile.ready` | Info | Fixed | `eb0835c` |
| IN-01 only one corpus report is scanned | Info | Deferred | none |

IN-01 is deferred on purpose. Scanning all 44 corpus reports changes the runtime profile of
the script. That is a scope decision for the human, not a defect to fix inside a review pass.

### The fix for CR-01, measured

A claim that wraps across two lines is the normal way a claim enters `README.md`, because the
file hard wraps its prose throughout. The scanner read one line at a time, so it could not see
such a claim. The Stage 4 self test hid this: it planted only single line probes, so it
reported 18 of 18 and looked strong.

`join_source()` now joins hard wrapped lines into one segment before the shapes run. It keeps
markdown headings, table rows and JSON structural lines as their own segment, so a table or a
report is never flattened into one blob. A FAIL now reports a line range.

Measured on the same planted sentence, before and after the fix:

```
BEFORE  exit 0, no FAIL line
AFTER   exit 1
        FAIL  readme  line 200-201  stmt-fwd    DeForm6 can recover every statement from the compiled executable.
```

The self test now plants wrapped probes as well as single line probes, so the same false
confidence cannot return.

### Two false positives the review did not anticipate

The fixer found these by running the widened shapes against the real tree:
- The noun "compiler" appears about ten times in the report vocabulary, describing the input
  file compiler flags. A bare `compile` shape fires on it. The fix requires a trailing non
  letter.
- The corpus holds a control named `chkPercent`. Dropping the leading digit rule from
  `pct-word` fires on it. The fix adds word boundaries.

Both are recorded in the script comments and in their commit messages.
