---
phase: 05
phase_name: "hostility"
project: "DeForm6"
generated: "2026-09-13"
counts:
  decisions: 10
  lessons: 7
  patterns: 6
  surprises: 7
missing_artifacts:
  - "05-VERIFICATION.md"
---

# Phase 05 Learnings: hostility

## Decisions

### Severity holds three values, not two
`Tolerated` joins `Fatal` and `Recoverable`. `Tolerated` names a defect that costs
one item and assumes nothing in its place. `Recoverable` keeps its old meaning, a
defect the reader continued past by assuming a value the file does not state.

**Rationale:** 36 of the 44 vendored corpus programs raise at least one defect
today. A strict mode that refused on any `Recoverable` defect would refuse 36
undamaged programs and break the phase 2, 3 and 4 gates. The third rung separates
"the reader recovered less" from "the reader invented a value".
**Source:** 05-01-SUMMARY.md

### The mode never enters a read function
`inspect()` builds a `Journal` from the mode only after every read has finished and
the local defect list holds every entry. It then feeds the whole list through
`Journal::record`.

**Rationale:** Success criterion 3 asks for the same defect list in both modes,
item for item. A per site `?` gives strict a prefix of salvage's list, not the same
list. One choke point at the end is the only shape that satisfies it.
**Source:** 05-01-SUMMARY.md

### A mechanical signature change lands before the behaviour change
Adding the `Mode` parameter to `inspect`, and later to `write::project`, landed as
its own commit with every call site passing `Mode::Strict`. The commit that wires
the policy came after.

**Rationale:** The signature change touches 56 call sites in 17 files and changes
nothing. The policy commit changes behaviour and touches few lines. Split this way,
a reader examines one step at a time and a bisect lands on the small commit.
**Source:** 05-01-SUMMARY.md

### Assumptions reuse the existing limits array
One line per `Recoverable` defect goes into the free text `ProjectReport.limits`
array through a new `report::assumption_lines` helper. `ProjectReport` gains no
field.

**Rationale:** The reversible option. Every phase 4 consumer of the report keeps
working. A typed field would have been the one way door, and the plan rated it as
such before taking the other path.
**Source:** 05-01-SUMMARY.md

### The capacity wall matches by text, not by line number
An audited row is matched against the tree by exact file plus trimmed source line.

**Rationale:** A line number drifts every time an unrelated edit lands above it, so
a wall keyed on line numbers reports false failures and gets relaxed. A wall that
gets relaxed is not a wall.
**Source:** 05-02-SUMMARY.md

### CRON_RUNS is set by arithmetic, not by preference
`CRON_RUNS` is 500,000, chosen so the `Box::leak` accumulation alone stays at least
ten times below the fuzz run's own resident set limit.

**Rationale:** The worst case leaked message was measured at 237 bytes across every
`damaged()` call site, not assumed. At a 2048 MiB limit the leak alone would reach
the limit at about nine million iterations, so 500,000 leaves the margin stated
rather than hoped for.
**Source:** 05-03-SUMMARY.md

### The two fuzz campaigns take different bounds
The pull request job uses `-max_total_time`. The scheduled job uses `-runs=N`.

**Rationale:** Roadmap success criterion 1 fixes the pull request command
verbatim, so that bound is not free to change. A wall clock bound is flaky on a
varying CI machine, so the longer scheduled run takes the deterministic bound
instead.
**Source:** 05-03-SUMMARY.md, 05-04-SUMMARY.md

### The first committed binary fixture was a human decision
The plan stopped at a `checkpoint:decision` before committing the seed. The human
chose `seed-with-an-owned-binary` over a text seed and over an empty directory with
a red gate.

**Rationale:** A blob does not leave git history without a rewrite, so the plan
rated the task `one-way` and earned the gate. The alternatives were worse: a text
seed refuses even earlier and defers the same door, and an empty directory
contradicts the rule that the full gate passes before every commit.
**Source:** 05-05-SUMMARY.md, 05-UAT.md

### pin-corpus regenerates the manifest rather than appending to it
The command parses the whole file into a `BTreeMap`, adds the entry, and writes the
file back, preserving the text above the first table byte for byte.

**Rationale:** Appending text produces a file whose order depends on the order a
human ran the commands. Regenerating from a sorted map gives the same file whatever
that order was.
**Source:** 05-07-SUMMARY.md

### Every new dependency is pinned to an exact version
`ureq = "=3.4.1"`, `sha2 = "=0.11.0"`, and `cargo-fuzz` installed as `0.13.2`.

**Rationale:** These are the exact versions the research audited. A caret range
lets a future release enter the tree without an audit, and the CI install step
would silently pick it up.
**Source:** 05-07-SUMMARY.md, 05-04-SUMMARY.md

---

## Lessons

### A planning time number is a hypothesis, not a measurement
The plan and the research said the corpus raises 430 defects, 428 of one kind and 2
of another. The real figure is 429, with 1 of the second kind. The planning walk
counted `Report::defects` and every `FormReport::defects`, but `compose_form`
already folds each form defect into the shared list, so one defect was counted
twice.

**Context:** The executor traced the double count to its source and reported it
rather than editing the test to match the plan. The plan text and the roadmap were
then corrected to the measured figure.
**Source:** 05-01-SUMMARY.md

### An audit finds a different number than a survey predicts
The research named 6 capacity sized allocations. The planning session named 13
across 8 files. The audit found 14 across 9 files, five of them taking a file
derived value, and zero of them unbounded.

**Context:** Each number was produced by a different method. Only the last one
walked every file in the tree with the test modules dropped. The wall script now
pins the result so the next count cannot drift silently.
**Source:** 05-02-SUMMARY.md

### A test can pass for the wrong reason, and only a second instrument shows it
The census test's first result agreed exactly with the plan's stated figure. It
agreed because both used the same wrong method. A standalone probe built by a
different route gave a different answer.

**Context:** Agreement between two things derived the same way is not evidence.
The test now asserts the containment invariant directly, so the double count cannot
come back unnoticed.
**Source:** 05-01-SUMMARY.md

### A plan's prose can overstate what its own byte recipe produces
Plan 05-05 describes the seed as an image whose declared form count is `0xFFFF`,
which implies it exercises the GUI table bound check. The image carries no import
directory, so `inspect` refuses it at `runtime_of` with `NoVbRuntime` and never
reaches `VbHeader::read` or `GuiTable::walk`.

**Context:** The executor followed the plan's literal byte steps and its explicit
instruction to mirror plan 05-02's fixture, which also embedded no header. The
gap is in the prose, not the recipe. `regressions.rs` now records the measured
scope in its own doc comment and names the test that really covers `0xFFFF`.
**Source:** 05-05-SUMMARY.md

### cargo test cannot prove panic equals abort
Cargo compiles every `--test` target with `panic=unwind` whatever
`[profile.release]` says, because the built in harness needs `catch_unwind` to keep
running after one test panics. The `rustc` invocation carries no `-C panic=abort`.

**Context:** Roadmap success criterion 5 asks for the proof run under
`panic = "abort"`. What the run proves is narrower and still true: no panic occurs
on any of the 48 inputs, in either mode, in both profiles. Proving the abort path
needs a test that starts the release binary as a separate process and reads its
signal. No plan in this phase does that.
**Source:** 05-08-SUMMARY.md, 05-UAT.md

### An invariant that lives only in a plan file does not hold
Phase 4 asserted that `report.rs` and `write/model.rs` hold no `HashMap`, because a
hash keyed container makes a byte identical report stop being one. A plan file runs
its assertions once, on the day that plan runs.

**Context:** Plan 05-06 fixed a real quadratic search, reached for `HashSet` and
`HashMap`, and both assertions were broken for four commits before a human read
them again. `scripts/prove-ordered-output-wall.sh` now runs the rule on every gate.
**Source:** 05-06-SUMMARY.md, commit dec19f9, commit "build: add the ordered output wall to the gate"

### A shared requirement ID completes only when every plan declaring it is done
SAF-01 is declared by four plans and SAF-05 by four plans. A plan that names an ID
has not earned it alone.

**Context:** One executor marked both complete while `.github/workflows/fuzz.yml`
and `crates/deform6/tests/regressions.rs` did not yet exist. Both were reverted and
ticked later, each after the files proving them were confirmed on disk.
`requirements.ready-ids` answers this correctly and should be consulted rather than
the plan's own frontmatter.
**Source:** 05-07-SUMMARY.md, .planning/REQUIREMENTS.md history

---

## Patterns

### The choke point
Collect every defect during the read. Apply the policy once, at the end, in the one
function that owns the whole read.

**When to use:** Whenever two modes must report the same findings and differ only
in what they do about them. A per site branch gives one mode a prefix of the
other's list and cannot satisfy an "identical list" requirement.
**Source:** 05-01-SUMMARY.md

### The two directional wall script
Hold an audited list. Fail when the source holds a site the list does not, and fail
when the list holds a row the source no longer has.

**When to use:** For a rule no lint and no type can express, where the danger is
both a new violation and a stale claim. Three walls in this repository now share
the shape, and the fourth was written by copying it.
**Source:** 05-02-SUMMARY.md

### A fuzz target that calls both modes and asserts nothing
Run the strict read and the salvage read over the same bytes. Assert nothing about
either result. Let the panic be the only failure.

**When to use:** Any fuzz target over a parser that can legitimately refuse. An
assertion on the result variant reports a correct refusal as a crash. The salvage
path also reaches code the strict path refuses before it gets there, so a target
that calls one mode leaves the least exercised code unfuzzed.
**Source:** 05-03-SUMMARY.md

### Break the test on purpose before you commit it
Change the thing the test covers, watch the test fail, read the message, revert.

**When to use:** Every new test. Every executor in this phase did it, and it found
a wrong severity arm, an unbounded walk that read ten million entries into
unrelated bytes, a stale wall row, an un-audited site, and an assumption filter
that reported 15 assumptions where it should report 1.
**Source:** 05-01-SUMMARY.md, 05-02-SUMMARY.md

### The crash to test procedure as the module doc comment
Write the steps for turning a fuzzer artifact into a committed regression test at
the top of the replay file, as numbered steps.

**When to use:** For a procedure a person performs rarely and under pressure. The
next person looks at the file the procedure is about, not at a separate document.
**Source:** 05-05-SUMMARY.md

### Seed the directory in the same commit as the count assertion
A directory walking test that passes on an empty directory proves nothing. Land the
first input and the assertion together.

**When to use:** Any test that walks a directory and asserts over what it finds.
Emptying `tests/regressions/` now fails two tests, the count and the provenance
check.
**Source:** 05-05-SUMMARY.md, 05-UAT.md

---

## Surprises

### A hostile input smoke test found a production denial of service on its first run
`PathIssuer::issue` and `SafeNameIssuer::issue` each scanned a growing `Vec` and
restarted the collision suffix search from the first candidate on every call. One
three byte mutation of `Curves.exe` drove many items to one name and turned a sub
second test into a run of more than 130 seconds.

**Impact:** The bug is quadratic in the number of colliding items and reachable
from a file. It was fixed with a set for membership and a per key next suffix
cache, with a 1000 item regression test in the same commit. This is the defect the
phase exists to find, and the cheapest test in the phase found it.
**Source:** 05-06-SUMMARY.md

### The fix for that bug broke a phase 4 invariant
The fix reached for `HashSet` and `HashMap`. Phase 4 forbids a `HashMap` in
`report.rs` and `write/model.rs`, because the written report must be byte
identical.

**Impact:** Both assertions failed for four commits. Nothing in the three gate
commands sees it, and the ten determinism tests pass until the day the iteration
order differs. The containers are now `BTreeSet` and `BTreeMap`, and a fourth wall
runs the rule on every gate.
**Source:** commit dec19f9, commit "build: add the ordered output wall to the gate"

### Two requirements were marked complete before they were earned
SAF-01 and SAF-05 were ticked while `.github/workflows/fuzz.yml` and
`crates/deform6/tests/regressions.rs` did not exist.

**Impact:** Caught by reading REQUIREMENTS.md against the filesystem rather than
against the summary. Both were reverted and ticked later, correctly. A tick that
nobody checks is worse than no tick, because it stops the next reader looking.
**Source:** .planning/REQUIREMENTS.md history, 05-07-SUMMARY.md

### The regression seed guards a shallower path than its own name says
`gui-table-overcount-4k.bin` refuses at `NoVbRuntime`, the first Visual Basic
specific check, not at the GUI table bound check.

**Impact:** The trap still closes and the count assertion is meaningful. What it
traps is the earliest refusal path. The real `wFormCount = 0xFFFF` coverage lives
in a unit test in `gui.rs`, and the replay file now points at it by name.
**Source:** 05-05-SUMMARY.md, 05-UAT.md

### The state tool resets the progress fields on every call
`gsd-tools`'s `state.*` verbs write `progress.percent` and
`progress.completed_phases` to zero while measuring `completed_plans` correctly.

**Impact:** Hit independently by every executor in the phase. It once left
`STATE.md` saying "0% of the 50 plans in the roadmap, which is 18 of 50" in one
sentence. Each executor now reads the file after the call and repairs both fields
by hand. A fix belongs in the tool.
**Source:** 05-07-SUMMARY.md, 05-08-SUMMARY.md

### Subagents resolved every path against a deleted worktree
The session began in a git worktree that was deleted mid session. Every subagent
spawned afterwards still resolved repository paths against it. `Write` and `Edit`
refused real paths, and one agent recreated the dead directory and wrote a file
into it.

**Impact:** Every executor worked around it with a `cat` heredoc or a small
`python3` script through Bash, which were never blocked. One misplaced file was
moved back and the stale tree removed. Every commit is on `main` in the real
repository.
**Source:** 05-03-SUMMARY.md

### A verify command in a plan carried a pre-existing false positive
Plan 05-03's acceptance check for `.planning/WINDOWS.md` searches for the first `[`
and the last `]` to extract a JSON block. An earlier finding's own description
holds a literal `[]`, so the search starts in the wrong place.

**Impact:** The check fails identically against the file as it stood before the
plan touched anything. The JSON block itself parses, with 14 entries. The defect is
in the verify command, not in the file, and it was reported rather than worked
around quietly.
**Source:** 05-03-SUMMARY.md
