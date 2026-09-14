---
phase: 06-version-1-0
plan: 02
subsystem: documentation
tags: [readme, honesty-surface, gap-register, release-docs]

requires:
  - phase: 06-version-1-0
    provides: "06-01: the report schema and the confirmation that report.json's three top level keys are items, defects, limits"
provides:
  - "README.md: the released claim surface, stating what DeForm6 returns, what it does not return, the three-word confidence vocabulary, the three facts a reader must have before running the tool, and every gap that is still open at release"
affects: [06-04, 06-06, 06-07]

actuals:
  tokens: 3435
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "A README limit row keyed to its source register row number (S-NN, F-NN), so the count can be re-derived and checked against the register mechanically rather than trusted by eye."
    - "Deferred numeric claims as named HTML comment markers (<!-- measured:x -->) rather than a number written before the number is measured."

key-files:
  created:
    - README.md
  modified: []

key-decisions:
  - "README.md's lead paragraph reuses all four sentences of AGENTS.md's 'What this project is' verbatim, including the closing instruction 'Do not add a claim that it does', per the plan's own read_first instruction to not weaken them."
  - "For structure-register rows 6 and 7 (FuncTypDesc header layout, optionalVals grammar), the code's own doc comments state both are 'closed on real bytes' by measurement, but STRUCTURES.md section 11's own Resolution-path column does not say CLOSED for either row. The README lists both as still-open limits, matching the register the acceptance check re-derives its count from, while describing what the code actually does today (a constFFFF marker check that refuses a mismatched record; a value-tag walk that reports unrecoverable defaults on an unrecognised tag)."
  - "Every 'What the tool does instead' cell was written from a direct code read (classify.rs, functyp.rs, privateobj.rs, controltree.rs, header.rs, ocx.rs, project.rs, values.rs), never from the register's own Resolution-path column, which names how a gap could be closed later, not what the shipped code does today."

patterns-established:
  - "The claim-surface discipline for any future documentation edit: a capability sentence must trace to a Confidence variant name, a report field, or a limits string the report emits verbatim, never to unsourced prose."

requirements-completed: [DET-05, FRM-01, FRM-02, FRM-03, FRM-04, VER-06, WRT-03]

coverage:
  - id: D1
    description: "README.md states what DeForm6 returns, what it does not return, the three confidence words with no fourth tier and no numeric score, and the three facts a reader must have before running the tool (the P-code branch is untested, frmHMM.frx is excluded by name, full recompilation was not tested, reusing the report's own verbatim sentences)."
    requirement: "DET-05"
    verification:
      - kind: other
        ref: "shell: grep -c 'measured:' README.md (=5); grep -q lpNativeCode/frmHMM.frx/'needs the Visual Basic 6 IDE on'; grep -oE confidence-words | sort -u | wc -l (=3); grep -q items/defects/limits/inspect/extract"
        status: pass
      - kind: other
        ref: "shell: LC_ALL=C tr -d printable-ascii < README.md | wc -c (=0), the byte-safety check from the plan's own <verify>"
        status: pass
    human_judgment: false
  - id: D2
    description: "README.md lists every still-open row of STRUCTURES.md section 11 (16 rows, keyed S-02 through S-18, excluding the three closed rows) and every item of FILE-FORMATS.md section 9 (10 items, keyed F-01 through F-10), each with the safe default the tool actually chose, re-derivable and checked against both registers by count."
    requirement: "FRM-01"
    verification:
      - kind: other
        ref: "shell: grep -c '^| S-' README.md (=16) equals a live re-derivation of STRUCTURES.md section 11's still-open rows; grep -c '^| F-' README.md (=10) equals a live re-count of FILE-FORMATS.md section 9's items; grep -c 'S-01\\|S-11\\|S-19' README.md (=0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Each of the 26 'What the tool does instead' cells accurately describes the code's real behaviour, not a paraphrase of the register's own Resolution-path column."
    verification: []
    human_judgment: true
    rationale: "Correctness of a prose description against source code is a judgment call a grep cannot make; each cell was written from a direct read of the naming code site (classify.rs, functyp.rs, privateobj.rs, controltree.rs, header.rs, ocx.rs, project.rs, values.rs) during this plan's own execution, but no automated test asserts the prose matches the code, so a human review against the cited files closes this deliverable."

duration: 40min
completed: 2026-09-14
status: complete
---

# Phase 6 Plan 2: README Claim Surface Summary

**A new `README.md`, 188 lines, stating what DeForm6 returns and does not return, the three-word confidence vocabulary, the three pre-run facts, and 26 still-open limits (16 from the structure survey, 10 from the file format survey), each with the code's real default and no corpus-measured number written before plan 06-07 measures it.**

## Performance

- **Duration:** ~40 min
- **Completed:** 2026-09-14
- **Tasks:** 2
- **Files modified:** 1 (created)

## Accomplishments

- `README.md` did not exist anywhere in this repository; this plan wrote the whole file, 188 lines, the repository's first release-facing document.
- The three facts required before a reader runs the tool are present as literal, reused sentences: `lpNativeCode` decides native-vs-P-code and every corpus program is native; `corpus/vb6-code/Hidden-Markov-model/frmHMM.frx` is damaged upstream, reusing the exclusion reason from `tests/support/frm.rs` verbatim; full recompilation was not tested, reusing `report.rs`'s own `build_limits` sentence word for word.
- The confidence vocabulary section names exactly `proven`, `inferred`, `unrecoverable`, no fourth tier, no numeric score, and names the three JSON report keys `items`, `defects`, `limits`.
- Two tables list all 26 gaps still open at release: 16 rows keyed `S-02` through `S-18` (STRUCTURES.md section 11, excluding the three closed rows), and 10 rows keyed `F-01` through `F-10` (FILE-FORMATS.md section 9). Each row states what the tool actually does today, read directly from the naming code site, not from the register's own resolution-path text.
- Five deferred number markers (`<!-- measured:corpus-programs -->` and four others) hold the place for plan 06-07's acceptance-run numbers; no corpus-measured figure is written in this wave.
- The whole file holds no byte outside tab, newline and printable ASCII: no em dash, no emoji, no percentage sign anywhere.

## Task Commits

1. **Task 1: The README spine, the vocabulary, and the three facts** - `28bf489` (feat)
2. **Task 2: Every limit that is still open at release, with the default the tool chose** - `57a01dd` (feat)

**Plan metadata:** committed separately, see below.

## Files Created/Modified

- `README.md` - the repository's first release-facing document: lead paragraph, what version 1.0 returns and does not return, the confidence vocabulary, the three pre-run facts, text/encoding/interruption rules, how to run it, five deferred number markers, and two tables of 26 still-open limits with the tool's chosen default for each.

## Decisions Made

- README.md's lead paragraph reuses AGENTS.md's four "What this project is" sentences verbatim, including "Do not add a claim that it does", per the plan's own instruction not to weaken them.
- STRUCTURES.md section 11 rows 6 and 7 (`FuncTypDesc` header layout, `optionalVals` grammar) are listed as still-open limits, matching the register's own Resolution-path column (neither says CLOSED), even though the naming code's own doc comments call both "closed on real bytes" by measurement. The README's "what the tool does instead" cells for S-06 and S-07 describe the actual, already-measured code behaviour (a `constFFFF` marker check that refuses a mismatched record; a value-tag walk that reports unrecoverable defaults on an unrecognised tag), so the row is honest about both the register's open status and the code's real behaviour at once.
- Every "what the tool does instead" cell traces to a direct code read this session (`classify.rs`, `functyp.rs`, `privateobj.rs`, `controltree.rs`, `header.rs`, `ocx.rs`, `project.rs`, `values.rs`), not to the register's own Resolution-path column, which names a future closing method, not today's shipped behaviour.

## Deviations from Plan

None - plan executed exactly as written. Both tasks completed with all automated `<verify>` commands passing on the first attempt; no fix-up cycle was needed.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- README.md is ready for plan 06-06's `scripts/check-claim-surface.sh` to grep against, and for plan 06-04's honesty audit.
- The five deferred number markers are ready for plan 06-07 to fill in from the acceptance run: `<!-- measured:corpus-programs -->`, `<!-- measured:procedures -->`, `<!-- measured:forms -->`, `<!-- measured:controls -->`, `<!-- measured:properties -->`.
- No limit row's default could not be found in the code; every one of the 26 rows traces to a named source file this session read directly. Nothing is deferred to plan 06-07's honesty audit as an unresolved default.
- The own-count re-derivation this plan's `<verify>` commands run (`awk`/`grep` over the two registers) will re-fail loudly if either register gains or loses a row before plan 06-07 runs; that is by design, not a fragility to fix.

---
*Phase: 06-version-1-0*
*Completed: 2026-09-14*

## Self-Check: PASSED

- `README.md` found on disk, 188 lines, tracked by git (`git ls-files README.md`).
- Commits `28bf489` and `57a01dd` found in `git log --oneline --all`.
- Re-ran the plan's full acceptance-criteria set for both tasks: all pass (5-of-5 Task 1 automated checks, 6-of-6 Task 2 automated checks, plus the two source-literal assertions for S-13 and S-15).
- `commits: 2` measured via `git rev-list --count` against the ledger recorded before Task 1's commit; matches the two task commits above.
- `cargo fmt --all --check` and `cargo test --workspace` re-run clean after both commits (89 tests passed in the schema/ratios group shown; full workspace suite unaffected by a documentation-only change).
