# Roadmap

## The finished product

A person has a compiled program and no source. They run DeForm6. They get a
Visual Basic project back. They open that project in a compiler they already
own, they edit it, and they build it.

DeForm6 is the decompiler and the proof that its output builds. It is not the
compiler and it is not the editor.

Two compilers exist and both accept the output shape: the Visual Basic 6 IDE,
and twinBASIC. DeForm6 ships neither. It finds what the user owns and drives
it, the same way `--opcode-table` already reads a table the user builds on
their own install.

## Two finished products, not one

The phases below deliver two different things, and the first one is close.

| | What the user gets | Distance |
|---|---|---|
| **1.1** | The shell of the program: forms, controls, properties, names, signatures, API declarations. Verified to build. The user writes the logic. | About 3 months |
| **2.0** | The logic as well, for a P-code program. | About 2 to 3 years after 1.1 |

Version 1.1 is worth shipping on its own. A person who lost a program still
has to rebuild the user interface by hand today, and that is most of the
typing.

## A note on the numbers below

Every effort figure in this file is an estimate. It is not a measurement.
The measured numbers in this project live in `tests/ratios.toml` and in the
README, and each one names the test that asserts it. Nothing here does.

## Phases

### Phase 7: Byte fidelity

**Goal.** Measure what the reader does not understand, against the bytes
Microsoft's own compiler wrote.

For every structure the reader parses at offset `O` with length `L`, add one
method that writes exactly `L` bytes back. Re-emit each structure over its own
location in a copy of the original file, then compare against the original. No
addresses are computed, because the original's own addresses are reused.

Every byte that differs is a field the reader misreads or does not model.

**Why first.** It needs no compiler, no Windows host, and no purchase. It can
start today. It also de-risks every phase after it: if the reader misunderstands
a structure, statement recovery builds on sand.

**Exit.** A committed fidelity map over all 44 corpus programs. Every difference
is either a closed gap or a named limit in the register.

**Estimate.** 2 to 4 weeks.

### Phase 8: The build gate

**Goal.** DeForm6 proves its own output compiles, on the user's machine, with
the user's compiler.

Add `--verify-build`. DeForm6 locates the Visual Basic 6 IDE or the twinBASIC
command line tool, runs it over the project it just wrote, captures the exit
status and the compiler's own diagnostics, and records the result in the JSON
report. It redistributes neither compiler.

**What this replaces.** The README states today that full recompilation did not
run, and that a structural check stood in for it. That sentence becomes a
measured field instead.

**A weakness to design around.** The build gate is necessary and it is weak. An
unknown type code becomes `Variant` at `write/code.rs:295`, and `Variant`
compiles. Four distinct COM types become `Object` at line 291. The project will
build while the signatures are wrong. Keep the differential harness. The build
gate does not replace it.

**Expect a tail of format faults.** The `Object=` lines for OCX references, the
five `Attribute` lines, the `.frx` byte offsets, and the code page assumption
are the likely first failures.

**Exit.** All 44 corpus programs extract and build, or each failure is a named
limit with the compiler's own error beside it.

**Estimate.** 4 to 8 weeks. Needs one Windows host and one compiler licence.

### Phase 9: The P-code corpus

**Goal.** 44 P-code binaries whose exact source is already held.

Every corpus program is native. 38 project files declare `CompilationType=0`
and none declares P-code. Statement recovery cannot be measured without P-code
inputs, and scavenging them gives binaries with no matching source.

Rebuild the existing corpus projects with `CompilationType=1` on the Phase 8
host. The ground truth is already in the repository.

This is also the first real exercise of the `lpNativeCode` P-code branch, which
the README records as never having run against a real P-code program.

**Estimate.** Days to 2 weeks, once the Phase 8 host exists.

### Phase 10: The P-code opcode table

**Goal.** A table that names each opcode and gives its argument width.

Derive it from the dispatch array in the runtime, together with the Microsoft
build that ships debug symbols naming the handlers. Public counts put this near
822 real handlers against 1531 slots.

Reuse the `xtask derive-opcode-table` pattern exactly: the tool is committed,
the table it produces is not, and the user builds it on a copy they lawfully
own.

**The hard part is argument width, not opcode names.** Every existing tool is
weakest there. The Phase 9 matched pairs let a width be verified against known
source rather than guessed.

**Estimate.** 4 to 8 months.

### Phase 11: P-code statement recovery

**Goal.** Procedure bodies come back as Visual Basic.

Retire one risk in week one: `STRUCTURES.md` section 10.3 has a single source at
confidence `[L]`, and the claim that a body occupies the `ProcSize` bytes
immediately before its descriptor is unverified. Check it against a real file
before anything is built on it.

Then: disassemble, lift to an intermediate form, recover control flow, and emit
Basic.

**The gate.** The recovered code compiles, which Phase 8 already measures, and
the rebuilt program behaves the same as the original.

**Estimate.** 12 to 24 months. This is the long pole.

### Phase 12: Native code, at its honest ceiling

**Goal.** Annotated disassembly for native builds, with the runtime helper calls
resolved into Visual Basic idioms.

Not compilable Basic. No public tool recovers compilable Basic from a native
build, and this one will not claim to either. State that limit in the README
before the first line is written, the way the current three limits are stated.

This phase is independent of 10 and 11 and can run beside them.

**Estimate.** 6 to 12 months.

### Phase 13: The workflow

**Goal.** One documented path from a compiled program to a rebuilt one.

Document the edit and rebuild route for both compilers. Make the failure
messages name the next action.

**Estimate.** 2 to 4 weeks.

## What blocks what

```
Phase 7  (no dependency, starts today)
Phase 8  (needs a Windows host and a compiler licence)
   |
   +-- Phase 9  (needs the Phase 8 host)
          |
          +-- Phase 10 (needs the Phase 9 corpus to verify widths)
                 |
                 +-- Phase 11 (needs the Phase 10 table)

Phase 12 runs beside 10 and 11.
Phase 13 closes the milestone.
```

Version 1.1 ships after Phase 8. Version 2.0 ships after Phase 11 and 13.

## Procurement

Three phases are blocked on hardware and a licence, so start this before the
code.

- A Windows host that runs 32-bit programs.
- One compiler. The Visual Basic 6 IDE, if a lawful copy is available, or
  twinBASIC.
- For automated builds in a pipeline, twinBASIC requires its Professional
  Edition. The free edition permits commercial use but not unattended command
  line builds.

## Open questions to close before planning around them

- Does a hosted Windows runner still carry the Visual Basic 6 runtime? If it
  does not, the build gate needs a different host.
- Does the twinBASIC command line accept a Visual Basic project file directly,
  or does it need a conversion step first?
