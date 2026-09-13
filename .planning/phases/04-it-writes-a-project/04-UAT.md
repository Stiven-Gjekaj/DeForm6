---
status: complete
phase: 04-it-writes-a-project
source: [04-VERIFICATION.md]
started: 2026-09-13
updated: 2026-09-13
---

## Current Test

[testing complete]

## Tests

### 1. Open a written project in the real VB6 IDE

expected: The IDE opens the written `.vbp` with no fatal load error, or it writes a `.log` file beside the form that names the line and the message.
result: skipped
reason: "Waived by the human on 2026-09-13. No Windows host with Visual Basic 6 is available. Nobody has opened a written project in the VB6 IDE. The human chose to close the phase without this evidence. This is a waiver, not a pass: no part of this repository may state that the IDE opened a written project."

Steps:

1. On this machine, write one project:

       cargo run -p deform6-cli -- extract corpus/vb6-code/Fire-effect/Fast_Flames.exe -o out/

2. Copy `out/` to a Windows host that has Visual Basic 6 installed.
3. Open `out/VBFire2.vbp` in the VB6 IDE.
4. Record what happens. If the IDE writes a `.log` file, copy its text here.

Why a human runs this: this CI has no Windows host and no VB6 install. The
roadmap accepts that limit and says so. The structural check proves the files
have the correct shape. It cannot prove the IDE opens them. Nothing in the
repository may claim this step ran until someone runs it.

## Summary

total: 1
passed: 0
issues: 0
pending: 0
skipped: 1
blocked: 0

## Gaps
