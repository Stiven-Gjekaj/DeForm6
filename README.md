# DeForm6

DeForm6 reads a compiled Visual Basic 6 executable and writes back a Visual
Basic project.
The first milestone recovers the metadata only: the forms, the control trees,
the property values, the names, and the procedure signatures.
It does not recover statements.
Do not add a claim that it does.

## What version 1.0 returns

A run of `extract` writes:

- One `.vbp` project file.
- One `.frm` file for each form.
- One `.frx` file for each form that carries a resource blob.
- One `.bas` file for each standard module.
- One `.cls` file for each class module.
- One JSON report, beside the project, naming what the run recovered and how
  sure it is of each fact.

DeForm6 has two subcommands.

`inspect` reads one executable and prints what DeForm6 found in it. It writes
nothing to disk.

`extract` reads one executable and writes a Visual Basic 6 project directory
that VB6 can open.

## What version 1.0 does not return

DeForm6 does not recover statements. The forms, the control trees, the
property values, the names, and the procedure signatures come back. The code
inside a procedure does not.

DeForm6 does not produce Basic source that a compiler accepts as a proof of
the original program's behaviour. Every recovered item carries a confidence
word, and a confidence word is not a promise of correctness.

DeForm6 does not open the Visual Basic 6 IDE and it does not compile
anything. No sentence in this file states a recovery figure as a share of one
hundred. Every capability sentence in this file traces to a report field, a
confidence word, or a limit the report states in its own words.

## The confidence vocabulary

DeForm6 grades every recovered fact with one of three words: `proven`,
`inferred`, `unrecoverable`. These are the only three words the JSON report's
`confidence` field can hold. There is no fourth tier and no numeric score.

`proven` means the exact byte this run read names the fact directly.

`inferred` means this run chose the fact because the file gives no other
answer, and the item's own `basis` field says so.

`unrecoverable` means this run could not recover the fact at all.

The JSON report holds exactly three top level keys: `items`, `defects`, and
`limits`. `items` lists every recovered fact together with its confidence
word. `defects` lists every place the read did not resolve cleanly. `limits`
lists every boundary this run reached, in plain sentences.

## Three facts to read before you run it

1. **The P-code branch is untested.**
   `ProjectInfo.lpNativeCode` decides whether DeForm6 reports a project as
   native code or P-code, and nothing else does. Every program in the test
   corpus this project measures against carries `CompilationType` zero in its
   own `.vbp` file, which is native, so `lpNativeCode` is non-zero in every
   one of them. The P-code branch of that field has never run against a real
   P-code program. This states what was run, not what is supported:
   DeForm6 reads `lpNativeCode` and branches on it, and only the native
   branch has a real program behind it.

2. **`corpus/vb6-code/Hidden-Markov-model/frmHMM.frx` is damaged upstream and
   is excluded by name.**
   The file is 56 bytes, and its own record header declares 56 bytes of
   payload, but only 55 are present, because the upstream repository sets
   `text=auto` and git's line ending normalisation silently removed one
   carriage return. `HMM.exe` holds the same string as a whole, correct
   record, and is the second, independent source of truth for this fault.
   DeForm6's own test corpus excludes this one file by name, rather than
   papering over the missing byte.

3. **Full recompilation was not tested.**
   Full recompilation did not run. It needs the Visual Basic 6 IDE on
   Windows, and this run had neither. A structural check ran in its place,
   and it never opened this project in the IDE.

## Text, encoding and interruption

DeForm6 measures length and equality in encoded bytes, never in code points
and never in grapheme clusters. This run assumes a Western code page. Every
character above U+00FF was replaced with a question mark and reported, never
guessed at a different code page.

One run is one process over one file. The library never opens a file; the
command line crate owns the file system. `extract` plans every write before
it writes anything, and refuses the whole run before it writes anything when
a planned path would leave the output directory, or when a symbolic link
already sits at a planned path. An interrupted run leaves a part written
directory. DeForm6 claims no atomicity.

## How to run it

Read one executable and print what DeForm6 found in it:

    deform6 inspect <path-to-exe>

Read one executable and write a Visual Basic 6 project directory that VB6 can
open:

    deform6 extract <path-to-exe> -o <output-directory>

DeForm6 gives one of six exit codes.

| Code | Meaning |
|---|---|
| 0 | The file was read. |
| 1 | Not a PE file. |
| 2 | A PE file, but it holds no Visual Basic runtime. |
| 3 | Visual Basic, but not version 6. |
| 4 | Visual Basic 6, but damaged. |
| 5 | An internal error, including a usage error. |

## The numbers

The numbers below are measured, not estimated. Plan 06-07 fills each one in
after the acceptance run over the full corpus.

DeForm6's test corpus holds <!-- measured:corpus-programs --> Visual Basic 6
programs.

Across that corpus, DeForm6 recovers <!-- measured:procedures --> procedure
signatures.

DeForm6 recovers <!-- measured:forms --> forms.

DeForm6 recovers <!-- measured:controls --> controls.

DeForm6 recovers <!-- measured:properties --> property values.

## Known limits still open at release

The list follows.
