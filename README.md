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

A gap that is still open at release is a documented limit, not a silent one.
The rows below come from two surveys: `.planning/research/STRUCTURES.md`
section 11, the structure gap register, and `.planning/research/FILE-FORMATS.md`
section 9, the file format gap list.

### From the structure survey

| Limit | What is not known | What the tool does instead |
|---|---|---|
| S-02 | Which bit of `fObjectType` reliably marks that the optional half of `ObjectInfo` is present. Two published tests disagree with the corpus. | The tool tests bit `0x2`, the only bit that separates a standard module from every other kind across all seventeen tabulated values. |
| S-03 | No source names the raw `fObjectType` value an MDIForm carries. | The tool classifies any value outside the three the corpus proves as `Unknown`, and carries the raw number forward in the report instead of guessing a name. |
| S-04 | How a `ParamArray` argument modifier is encoded. No corpus source declares one. | The tool never emits the `ParamArray` modifier. The argument decodes under its ordinary type code, because the documented modifier bits are indistinguishable from a plain `ByRef Variant` argument. |
| S-05 | What Visual Basic type ten of the documented type codes name. None occurs in the corpus. | The tool reports the type as `Unknown` and carries the raw byte forward, rather than guessing a name. |
| S-06 | Which of two disputed `FuncTypDesc` header layouts is correct. | The tool checks a fixed marker word at this position on every record, and reports a defect instead of building a prototype when the marker does not match, rather than parsing the disputed alternate layout. |
| S-07 | Where the `optionalVals` pointer's value comes from, and what governs it at the source level. | The tool walks a length prefixed run of default value records behind the pointer, and stops the walk and reports that record's own defaults as unrecoverable when it meets a value tag it does not recognise, rather than guessing a width. |
| S-08 | How wide one entry of the `PubVarDesc` array is, so the array can be walked. No stride hypothesis converges against the corpus. | The tool carries the count field forward as an unexplained open question in the report, and does not walk the array at all. |
| S-09 | How to recover the source level name of an event slot. No corpus program carries a single `EventDesc` record. | The tool walks the event pointer array and reports the address of each entry, and decodes no name from any of them. |
| S-10 | How a source level `Alias "#123"` ordinal import is encoded in the compiled file. | When the Visual Basic level name is not present in the file, the tool reports the export as an inferred ordinal number, rather than guessing a name. |
| S-12 | Whether the byte at control block offset `0x02`, named `uni` by one prior tool and never read by it, is a Unicode flag for the name that follows. | The tool does not read this byte into any field. It is skipped as part of the fixed header the tool does not interpret. |
| S-13 | The exact character range and code page a written string obeys. | This run assumes a Western code page. Every character above U+00FF was replaced with a question mark and reported, never guessed at a different code page. |
| S-14 | Whether a menu that is itself a sibling within an already open menu group, opening its own child, reads the same as a confirmed sibling case; and one separate case where an expected separator byte is not present at all. | The tool decodes the two level deep menu close transitions measured across five transitions in two programs, and reports a defect naming the byte offset when the walk meets a byte that is not the expected separator, rather than guessing an ambiguous depth. |
| S-15 | The property name behind most control type and opcode pairs. | The tool ships a small, safe provenance "Opcode table" built into the binary, states its own size in the report's limits array, and reports a property it cannot name as present and not decoded, rather than guessing a name. A user can supply a larger table built on their own lawful Visual Basic 6 install. |
| S-16 | What the nine unknown dwords in `GUIObjectInfo` hold. | The tool does not read these bytes into any field, and reports nothing from them. |
| S-17 | What the five unknown dwords in the external component entry hold. | The tool decodes only the two fields it uses, `oUuid` and the textual GUID offset and length. The other fields carry no struct field, and the tool reports nothing from them. |
| S-18 | Whether an entry point opcode other than the one all 44 corpus executables share is a valid, undocumented variant. | The tool accepts only the one push immediate opcode the whole corpus shares, and refuses any other opcode with an error naming what it expected at the entry point. |

### From the file format survey

| Limit | What is not known | What the tool does instead |
|---|---|---|
| F-01 | The exact character count where the IDE moves a string property from the `.frm` into the `.frx`. The corpus bounds it between 99 and 153 characters but does not fix it. | The tool always writes the string inline when it is 97 characters or shorter and holds no line break, the proven floor of the open range. |
| F-02 | Whether a second, healthy multi line `TextBox` sample would confirm the reconstructed `u8` count, or point to a different width. | The tool writes the `$` form with a `u32` count, proved exactly by one corpus file, for every string that must go to the `.frx`. |
| F-03 | Whether the loader tolerates a missing `Attribute` line. No corpus file omits one, and no document states a requirement. | The tool writes all five `Attribute` lines, always. |
| F-04 | Whether `.vbp` key order matters. Not documented, and the one independent parser the survey checked does not care. | The tool matches the order the IDE itself writes. |
| F-05 | The `List` record layout for `ComboBox` and `ListBox`. No `ComboBox` or `ListBox` in the corpus stores its `List` property in a `.frx`, so the record layout one prior tool documents is unverified. | The tool reports a `List` property as present and not decoded, the same generic fallback it uses for any property with no proven table entry, rather than writing the unverified layout. |
| F-06 | A second record shape one prior tool documents for small to medium text, including an off by one bug that tool warns about. Unverified against this corpus. | The tool never writes this record shape. Every string routed to the `.frx` uses the one shape the corpus proves, the `$` form with a `u32` count. |
| F-07 | How DeForm6 should choose a code page for a Japanese or Cyrillic Visual Basic 6 project. Every corpus file is Western. | The tool assumes a Western code page on every run, and replaces every character above U+00FF with a question mark, reporting that it did so, rather than guessing a different code page. |
| F-08 | The `.ctx` record layout, assumed identical to `.frx` and not checked. This matters only when ActiveX control projects come into scope. | The tool does not read or write `.ctl` or `.ctx` files. ActiveX control projects are out of scope for this milestone, per `PROJECT.md`. |
| F-09 | Whether an MDIForm project, root class `VB.MDIForm` and a child form carrying `MDIChild = -1`, reads the way the one cited source states. No corpus program is an MDIForm project. | The tool classifies an MDIForm's `fObjectType` value as `Unknown`, the same fallback S-03 describes, and writes whatever property values it reads with no MDIForm-specific handling. |
| F-10 | Whether the VB6 IDE ever wrote a comma decimal separator, such as `Size = 8,25`, on a non-English machine. The corpus is English only. | The tool always writes a period as the decimal separator, using Rust's own locale independent number formatting, regardless of the host machine's locale. |

Sixteen rows come from the structure survey and ten rows come from the file
format survey: twenty six limits, still open at release, each with the
default the tool chose in their place.
