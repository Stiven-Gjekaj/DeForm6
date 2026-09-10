---
phase: 03-forms
plan: 02
subsystem: forms
tags: [vb6, opcode-table, toml, cli, xtask, derived-data]

# Dependency graph
requires:
  - phase: 03-forms
    provides: "plan 03-01's phase 3 module scaffolding (vb/opcodes.rs stub); this plan fills it"
provides:
  - "OpcodeTable, OpcodeEntry, PayloadType, TableError: the one representation for naming a property opcode, built either from OpcodeTable::builtin (the safe-provenance subset) or OpcodeTable::parse (a user supplied TOML table), sharing one lookup method"
  - "xtask derive-opcode-table: serialize_table (plain Rust, unit tested) and the #[cfg(windows)] COM walk stub, refusing by name on every other host"
  - "deform6 inspect --opcode-table <path>: the run time loader, mapping a missing or malformed table to exit 5, and a report line naming which table produced this run's property names"
affects: [03-06-propstream, 03-10-differential-gate]

actuals:
  tokens: 11047
  tasks: 3
  commits: 3
plan_head_before: 3e90ad0
commits: 3

tech-stack:
  added: ["toml (deform6 library dependency, already a pinned workspace dependency used by xtask)"]
  patterns:
    - "one representation, two sources: OpcodeTable::builtin and OpcodeTable::parse both return the same OpcodeTable and share one lookup method, so a caller never has two code paths to keep in sync"
    - "a control type or an opcode above 255 refuses inside serde's own u8 map-key deserializer, carrying that key's own span; no hand-written bound check duplicates or could drift from it"
    - "#[allow(dead_code, reason = \"...\")] for code whose only real caller is a #[cfg(windows)] branch this host never compiles, matching the exact precedent crates/xtask/src/main.rs's own embedded ratios module already set"

key-files:
  created:
    - crates/xtask/src/opcode_table.rs
  modified:
    - crates/deform6/src/vb/opcodes.rs
    - crates/deform6/Cargo.toml
    - crates/xtask/src/main.rs
    - crates/xtask/Cargo.toml
    - crates/deform6-cli/src/main.rs
    - crates/deform6-cli/tests/cli.rs
    - .gitignore

key-decisions:
  - "Checkpoint (Task 1) auto-resolved to the first option, TOML, per this run's auto-mode checkpoint protocol (gate=\"blocking\"): one table per control type, keyed by opcode."
  - "The safe-provenance subset transcribes only the STRUCTURES.md section 8.5.1 rows whose payload is a single, unconditional PayloadType. ScaleMode (opcode 25) and ClientLeft/Top/Width/Height (opcode 53) on Form, ListBox's List (opcode 20), and Label/ListBox's DataSource/DataFormat are left out: their payload is conditional, multi-shape, or has no stated width in the source, and a wrong width transcribed here would silently misalign every property that follows it in the stream."
  - "Every commit in this plan landed on main, per this session's explicit sequential-executor instructions (\"work on the main working tree, use normal git commits\") and this project's own git.branching_strategy: none. gsd-tools query git.base-branch --is-protected main returns true and no git.allow_default_branch_commits override is set in .planning/config.json, but plans 03-01 and 03-03 in this same phase already committed directly to main under this identical configuration; this plan follows that established, explicit project convention rather than the generic multi-agent worktree safety net."
  - "Added serde as an xtask dependency (Rule 3 - blocking): SerRow's #[derive(serde::Serialize)] needs it, and it was not in Task 3's own <files> list. serde is already a pinned workspace dependency used by deform6, so this is wiring an already-vetted dependency into another local crate, not a new package install; Rule 3's package-legitimacy exclusion does not apply."
  - "The derivation tool's default output path is derived/opcode-table.toml, not a bare top-level opcode-table.toml: a leading '/' in the .gitignore line makes git check-ignore treat the extracted path as an OS-absolute path outside the repository (confirmed by running it), which breaks the plan's own literal verify script. Nesting one directory down gives the grep pattern a non-slash character to anchor on and keeps git check-ignore working from a relative path."

patterns-established:
  - "A hostile-input table format built on toml::from_str into a typed BTreeMap<u8, BTreeMap<u8, Row>> gets its own T-03-15 row-count bound for free: TOML has no length-prefix field a collection could be sized from before validation, so the attack this repository's usual checked_add discipline exists to stop does not have a foothold here in the first place."

requirements-completed: [FRM-03]

coverage:
  - id: D1
    description: "OpcodeTable::builtin() holds the Form, MDIForm, CommandButton, Label and ListBox rows transcribed from STRUCTURES.md section 8.5.1; opcode 31 names three different properties on three different control types, and a PictureBox opcode gives None"
    requirement: "FRM-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::opcode_31_names_three_different_properties_on_three_control_types"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::a_picture_box_opcode_gives_none_because_the_subset_holds_no_picture_box_entries"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::every_builtin_entry_carries_a_non_empty_source"
        status: pass
    human_judgment: false
  - id: D2
    description: "OpcodeTable::parse reads the TOML format a user supplies, refuses a control type above 255 and a malformed row with the line number, and gives zero rows with no error on empty input; no table file is ever committed"
    requirement: "FRM-03"
    verification:
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::parse_refuses_a_control_type_above_255_and_names_the_line"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::parse_refuses_a_malformed_row_and_names_the_line_and_what_it_expected"
        status: pass
      - kind: unit
        ref: "crates/deform6/src/vb/opcodes.rs#tests::parse_on_empty_input_gives_a_table_with_zero_rows_and_no_error"
        status: pass
      - kind: other
        ref: "TRACKED=$(git ls-files) && test 0 -eq \"$(printf '%s\\n' \"$TRACKED\" | grep -ci 'opcode.*table\\.\\(toml\\|txt\\|bin\\|csv\\)')\""
        status: pass
    human_judgment: false
  - id: D3
    description: "xtask derive-opcode-table: serialize_table round trips through OpcodeTable::parse including a name holding a quote and a backslash; the COM walk refuses by name on this non-Windows host and reads no file; .gitignore excludes the tool's default output path and git check-ignore confirms it"
    requirement: "FRM-03"
    verification:
      - kind: unit
        ref: "crates/xtask/src/opcode_table.rs#tests::a_name_holding_a_quote_and_a_backslash_round_trips_intact"
        status: pass
      - kind: unit
        ref: "crates/xtask/src/opcode_table.rs#tests::a_multi_row_multi_control_type_table_round_trips_through_both_functions"
        status: pass
      - kind: other
        ref: "cargo run -p xtask -- derive-opcode-table | grep -qi windows (exit 1)"
        status: pass
      - kind: other
        ref: "git check-ignore -q derived/opcode-table.toml"
        status: pass
    human_judgment: false
  - id: D4
    description: "deform6 inspect --opcode-table <path> reads a user supplied table; a missing file or a malformed table exits 5 and names the line; absent the flag the report names the builtin subset and its entry count"
    requirement: "FRM-03"
    verification:
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#inspect_with_no_opcode_table_flag_exits_zero_and_prints_the_builtin_line"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#inspect_with_an_opcode_table_flag_pointing_at_a_missing_file_exits_five"
        status: pass
      - kind: e2e
        ref: "crates/deform6-cli/tests/cli.rs#inspect_with_a_malformed_opcode_table_exits_five_and_prints_a_line_number"
        status: pass
    human_judgment: false

duration: 46min
completed: 2026-09-10
status: complete
---

# Phase 3 Plan 2: The Opcode Table Summary

**OpcodeTable with one lookup shared by the builtin safe-provenance subset and a user-supplied TOML table, `xtask derive-opcode-table` committing the tool and never the table, and `deform6 inspect --opcode-table` naming which table produced each run's property names.**

## Performance

- **Duration:** 46 min (approximate; this session did not capture a literal start epoch, so this is measured from plan 03-03's completion commit to this plan's final code commit)
- **Started:** 2026-09-10T09:34:00Z (approximate)
- **Completed:** 2026-09-10T10:19:14Z
- **Tasks:** 4 (1 checkpoint, 3 code tasks)
- **Files modified:** 9 (1 created, 8 modified)

## Accomplishments

- `OpcodeTable`, `OpcodeEntry`, `PayloadType` and `TableError` in `crates/deform6/src/vb/opcodes.rs`. `OpcodeTable::builtin()` transcribes the Form, MDIForm, CommandButton, Label and ListBox rows `STRUCTURES.md` section 8.5.1 cites from Semi VB Decompiler's own authored source comments, never from Microsoft's `VB6.OLB`. Opcode 31 gives `DrawMode` on a Form, `Appearance` on a CommandButton and `BackStyle` on a Label, proving the opcode space is per control type; a PictureBox opcode gives `None`, since the subset holds no PictureBox entries.
- `OpcodeTable::parse` reads a TOML table a user builds on their own machine: one table per control type, keyed by opcode. A control type or an opcode above 255 refuses inside serde's own `u8` key deserializer, with that key's own line number; a malformed row (a missing field, a bad payload word) refuses the same way. `git ls-files` names no opcode table anywhere in this repository.
- `crates/xtask/src/opcode_table.rs`: `serialize_table` writes the same TOML format, tested against tables built in memory including a property name holding a quote and a backslash. The COM walk that would read a real type library is `#[cfg(windows)]` and stubbed, documented as untested by construction on this repository's own gate; on this macOS host `xtask derive-opcode-table` refuses by name and reads no file. `.gitignore` excludes the tool's default output path, `derived/opcode-table.toml`.
- `deform6 inspect --opcode-table <path>` on `crates/deform6-cli`: absent the flag, `OpcodeTable::builtin()` names properties; present, the CLI reads the file and calls `OpcodeTable::parse` on the bytes, keeping the library's "no file access" rule intact. A missing file or a malformed table exits 5, never 4: the argument is unusable, not the executable under inspection. The report's last line names which table produced the run's property names and how many entries it holds.

## Task Commits

Each task was committed atomically:

1. **Task 1: Choose the on-disk format of the opcode table** — checkpoint:decision, auto-resolved to `toml` per auto-mode; no code commit.
2. **Task 2: OpcodeTable, its parser, and the safe provenance subset** — `b2ac045` (feat, tdd="true", code and tests together per `AGENTS.md`)
3. **Task 3: The derivation tool, committed, and its output path, excluded** — `e3e3aa3` (feat)
4. **Task 4: The --opcode-table flag on inspect** — `b29ece6` (feat)

**Plan metadata:** commit follows this SUMMARY.

_Note: Task 2 carries `tdd="true"`. Per `AGENTS.md`'s "code and its tests in the same commit" rule (binding over the plan's generic RED-then-GREEN split, matching 03-01's and 03-03's own precedent), the RED phase was run once against a deliberately broken `lookup` (see Break-on-purpose evidence below), then reverted and committed together with the correct implementation in one commit._

## Files Created/Modified

- `crates/deform6/src/vb/opcodes.rs` — `OpcodeTable`, `OpcodeEntry`, `PayloadType`, `TableError`, `builtin`, `parse`, `lookup`, and 14 unit tests
- `crates/deform6/Cargo.toml` — adds `toml.workspace = true`
- `crates/xtask/src/opcode_table.rs` — `serialize_table`, `payload_word`, `derive_opcode_table`, the `#[cfg(windows)]` COM walk stub, and 5 unit tests
- `crates/xtask/src/main.rs` — declares `mod opcode_table;`, wires the `derive-opcode-table` subcommand, extends the usage line and module doc comment
- `crates/xtask/Cargo.toml` — adds `serde.workspace = true`
- `crates/deform6-cli/src/main.rs` — the `--opcode-table` flag, `load_opcode_table`, the report's table-summary line
- `crates/deform6-cli/tests/cli.rs` — 3 new tests for the flag's three outcomes
- `.gitignore` — excludes `derived/opcode-table.toml`, with the reason recorded

## Decisions Made

- Checkpoint (Task 1) auto-resolved to the first option, TOML, per this run's auto-mode checkpoint protocol.
- The safe-provenance subset leaves out `STRUCTURES.md` section 8.5.1's own conditional or unwidth-stated rows (`ScaleMode`, the `ClientLeft/Top/Width/Height` block, `List`, `DataSource`, `DataFormat`) rather than guess a payload width for them. See key-decisions in the frontmatter for the full list and reasoning.
- Every commit in this plan landed on `main`, per this session's explicit sequential-executor instructions and this project's `git.branching_strategy: none`, matching plans 03-01 and 03-03's own precedent in this same phase.
- The derivation tool's default output path is `derived/opcode-table.toml`, not a bare top-level `opcode-table.toml`: a leading `/` in the `.gitignore` line makes `git check-ignore` treat the extracted path as an OS-absolute path outside the repository, breaking the plan's own literal verify script. Confirmed by running it before choosing the final path.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `serde` as an `xtask` dependency**
- **Found during:** Task 3, first `cargo test -p xtask` run
- **Issue:** `opcode_table.rs`'s `SerRow` needs `#[derive(serde::Serialize)]`, and `xtask`'s `Cargo.toml` (not in Task 3's `<files>` list) did not depend on `serde` directly, only through `deform6`'s re-exports, which are not visible as `serde::Serialize` in a downstream crate.
- **Fix:** Added `serde.workspace = true` to `crates/xtask/Cargo.toml`. `serde` is already a pinned workspace dependency used by `deform6`; this wires an already-vetted dependency into another local crate, not a new package install.
- **Files modified:** `crates/xtask/Cargo.toml`
- **Verification:** `cargo test -p xtask` compiles and the new tests pass.
- **Committed in:** `e3e3aa3` (Task 3 commit)

**2. [Rule 3 - Blocking] `#[allow(dead_code)]` on `SerRow` and `serialize_table`**
- **Found during:** Task 3, `cargo clippy --all-targets -- -D warnings`
- **Issue:** `serialize_table`'s only real caller is the `#[cfg(windows)]` COM walk, which never compiles on this host, and its only other caller is this file's own `#[cfg(test)]` module. The plain `bin` target `cargo clippy --all-targets` also builds (separately from the `test` target) sees neither, so `-D dead-code` fails the gate.
- **Fix:** Added a documented `#[allow(dead_code, reason = "...")]` to `SerRow` and `serialize_table`, matching the exact precedent `crates/xtask/src/main.rs`'s own embedded `ratios` module already set for the same class of "used differently across build targets" issue.
- **Files modified:** `crates/xtask/src/opcode_table.rs`
- **Verification:** `cargo clippy --all-targets -- -D warnings` passes with zero warnings.
- **Committed in:** `e3e3aa3` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3, blocking).
**Impact on plan:** Both were necessary for the gate to pass. `serde` is an already-vetted, already-pinned workspace dependency, so no new package legitimacy question is raised. No scope creep.

## Break-on-purpose evidence (AGENTS.md requirement, and this plan's own acceptance criteria)

**Task 2's `lookup`** (tdd="true", the RED phase): changed to ignore `opcode` and give the first entry the control type holds. `cargo test -p deform6 --lib vb::opcodes` run once:

```
thread 'vb::opcodes::tests::lookup_gives_the_entry_the_pair_names' panicked:
  left: "LinkMode"
 right: "DrawMode"
thread 'vb::opcodes::tests::opcode_31_names_three_different_properties_on_three_control_types' panicked:
  left: "Font"
 right: "Appearance"
thread 'vb::opcodes::tests::mdiform_shares_the_forms_property_set' panicked:
  left: "OLEDropMode"
 right: "DrawMode"
thread 'vb::opcodes::tests::a_hand_written_table_parses_and_lookup_finds_it_through_the_one_shared_method' panicked:
  left: "Appearance"
 right: "Style"
thread 'vb::opcodes::tests::a_hand_written_table_round_trips_through_parse_alone' panicked:
  left: "BackStyle"
 right: "DragMode"

test result: FAILED. 9 passed; 5 failed; 0 ignored; 0 measured; 219 filtered out
```

Reverted to the correct `lookup` before committing `b2ac045`.

**Task 3's acceptance criteria**: removed the `#[cfg(not(windows))]` refusal branch entirely (`derive_opcode_table` returned `0` unconditionally). `cargo run -p xtask -- derive-opcode-table` printed nothing and exited 0; the plan's own `grep -qi 'windows'` check genuinely failed. Reverted before committing `e3e3aa3`.

**Task 4's acceptance criteria**: changed the missing-opcode-table-file branch to `Exit::Damaged` instead of `Exit::Internal`. `cargo test -p deform6-cli inspect_with_an_opcode_table_flag_pointing_at_a_missing_file_exits_five` run once:

```
thread 'inspect_with_an_opcode_table_flag_pointing_at_a_missing_file_exits_five' panicked:
assertion `left == right` failed: stderr was: could not read .../deform6-cli-test-a-missing-opcode-table-that-does-not-exist.toml: No such file or directory (os error 2)
  left: 4
 right: 5
```

Reverted to `Exit::Internal` before committing `b29ece6`.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None. A human who owns a lawful Visual Basic 6 install can run `xtask derive-opcode-table` on a Windows host once the COM walk is implemented (see Known Stubs); this plan builds no interactive setup step.

## Known Stubs

- **`crates/xtask/src/opcode_table.rs`, `windows_walk::run`** (`#[cfg(windows)]`): the COM walk over a real type library is a documented stub that prints "not yet implemented" and exits 1. It is intentional and out of scope for this plan: `03-CONTEXT.md` D-01 and `03-RESEARCH.md` both scope the real COM walk as follow-up work needing a human at a Windows host with a lawful Visual Basic 6 install, which this session's environment cannot provide or test. No future GSD plan in this roadmap currently owns implementing it; it is a manual, external prerequisite for a human who wants to build their own opcode table beyond the safe-provenance subset this plan ships.

## Threat Flags

None. Every new trust boundary this plan introduces (the `--opcode-table` file, a declared row count inside it, an opcode with no table entry, the fact source a table's entry carries) is already named in this plan's own `<threat_model>` (T-03-05, T-03-15, T-03-04, T-03-17), and this plan's implementation follows each mitigation as written.

## Next Phase Readiness

- `OpcodeTable::lookup`, `PayloadType::fixed_width` and the honest-gap pattern (`None` → present, undecoded, at its byte offset) are ready for plan 03-06 (`propstream.rs`) to build the real property-value walk against.
- The 158 pairs outside this plan's safe-provenance subset stay an explicitly tracked, honestly reported gap, per `03-RESEARCH.md`'s own scoping; closing them is human follow-up work behind a Windows host, not this roadmap's remaining phase 3 plans.
- No blockers for plan 03-04, which does not depend on this plan's output.

---
*Phase: 03-forms*
*Completed: 2026-09-10*

## Self-Check: PASSED

All 6 created/modified files central to this plan found on disk. All 3
task commits (`b2ac045`, `e3e3aa3`, `b29ece6`) found in `git log`.
