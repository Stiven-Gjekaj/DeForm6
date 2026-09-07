# VB6 project source file formats

Written for the part of DeForm6 that **writes** a project, not the part that
reads a binary. Each rule below carries a label.

| Label | Meaning |
|-------|---------|
| **(a)** | Observed in the corpus. The count of files is given. |
| **(b)** | Documented in a cited source. |
| **(c)** | Inferred. The reasoning is given. |

Where the corpus and a document disagree, this file follows the corpus and
says so.

## Evidence base

The corpus is `tannerhelland/vb6-code`, BSD-2-Clause, cloned at the commit that
was current on 2026-09-07.

Files examined:

| Kind | Count | Note |
|------|-------|------|
| `.vbp` | 32 | All 32 read in full. |
| `.frm` | 36 | All 36 read. The header of each was parsed by script. |
| `.frx` | 10 | All 10 read as bytes. All 10 parsed record by record. |
| `.cls` | 46 | All 46 read. The preamble of each was hashed. |
| `.bas` | 7 | All 7 read. |
| `.ctl` | 0 | The corpus holds none. |
| `.exe` | 31 | Three read as bytes to check a `.frx` record. |

Total text files measured: 121.

Three files outside the corpus were read to cover what the corpus lacks. They
are public GitHub files. No byte of them enters this repository.

- `tannerhelland/VBIDEUtils`, `VBDoc/VBDoc.frm`. It gives an `Object` line in a
  `.frm` and two OCX control blocks.
- `tannerhelland/PhotoDemon`, `Controls/pdLabel.ctl`. It gives a `.ctl` header.

Two documented sources are used.

- Microsoft VB6 archive documentation on Microsoft Learn, and the 31 form load
  error pages in the `MicrosoftDocs/VBA-Docs` repository. These are the primary
  document source.
- `vb6parse`, an independent Rust parser for the same formats
  (`scriptandcompile/vb6`). It is used as a second opinion, never as the only
  evidence for a rule.

---

## 0. Notation

`CRLF` is the byte pair `0D 0A`. `SP` is one space, `0x20`.

A "line" in a `.vbp`, `.frm`, `.bas`, `.cls` or `.ctl` file always ends with
CRLF. See section 6.

---

## 1. The `.vbp` grammar

### 1.1 Shape

A `.vbp` file is a flat list of `Key=Value` lines, then an optional list of
`[Section]` blocks. **(a)** 32 of 32.

There is no space around the `=`. **(a)** 32 of 32.

Only one section header appears in the corpus, `[MS Transaction Server]`, in 29
of 32 files, always last, always followed by `AutoRefresh=1`. **(a)**

The three files without the section still load. **(c)** The three files are
`VB_Scanner_Support.vbp`, `Physics.vbp` and `HMM.vbp`, and each has a matching
committed `.exe`, so the IDE built them.

### 1.2 Two kinds of key

The file has two kinds of key, and they behave differently.

**Component lines** name a file that belongs to the project. A key may repeat.
The order is the order in which the developer added the item, and the kinds
interleave freely. **(a)** In `Advanced Histograms.vbp` the order is `Form`,
`Reference`, `Class`, `Form`, `Module`, `Class`.

**Setting lines** carry one value. A key appears once. **(a)**

An emitter must keep component lines in a single ordered list. It must not
group them by kind. **(c)** The IDE rewrites the list in load order on save, so
grouping is not a load failure, but it makes a byte comparison against the
original fail.

### 1.3 Component lines

| Key | Grammar | Seen |
|-----|---------|------|
| `Type` | `Type=Exe` | 32 |
| `Reference` | `Reference=*\G{GUID}#Major.Minor#LCID#Path#Description` | 33 |
| `Object` | `Object={GUID}#Major.Minor#LCID; FILE.OCX` | 1 |
| `Form` | `Form=Name.frm` | 36 |
| `Module` | `Module=ModuleName; File.bas` | 7 |
| `Class` | `Class=ClassName; File.cls` | 44 |

All six are **(a)**.

`Type` takes `Exe`, `Control`, `OleExe` or `OleDll`. **(b)** vb6parse
`CompileTargetType`. The corpus is `Exe` only.

`Reference` has two forms. A reference that starts `*\G{` names a compiled type
library by GUID. A reference that starts `*\A` names a sub-project by path.
**(b)** vb6parse `parse_reference`. Only the `*\G{` form is in the corpus.

The five fields of a `*\G` reference are separated by `#`:

```
Reference=*\G{00020430-0000-0000-C000-000000000046}#2.0#0#..\..\..\Windows\SysWOW64\stdole2.tlb#OLE Automation
```

The path is relative to the `.vbp` file. **(a)** All 33 references in the corpus
are relative, and they climb out of the project directory to reach
`Windows\SysWOW64`.

An emitter should write the reference path that the recovered binary implies,
not a path copied from the developer machine. **(c)** The IDE resolves a type
library by GUID and version first. It uses the path only as a hint.

`Object` names an OCX. The single corpus example is:

```
Object={F9043C88-F6F2-101A-A3C9-08002B2F49FB}#1.2#0; COMDLG32.OCX
```

Note the shape. The GUID is **not** quoted here, the file name is **not**
quoted, and a `; ` separates them. The `.frm` form of the same idea quotes both
parts. See section 2.3. **(a)**

`Module` and `Class` carry two names: the VB name of the component, then the
file name. The separator is `; `. The two often differ:

```
Module=Declaration_Module; Subs.bas
Module=Sub_Module; Declarations.bas
```

That pair is from `Map Editor.vbp`. The module named `Declaration_Module` lives
in `Subs.bas`. An emitter must never assume that the file name matches the VB
name. **(a)**

`Form` carries only the file name. The VB name lives in the `.frm` file, in
`Attribute VB_Name`. **(a)** 36 of 36.

Keys documented but absent from the corpus: `UserControl`, `UserDocument`,
`Designer`, `RelatedDoc`, `PropertyPage`. **(b)** vb6parse handler table.

### 1.4 Setting lines

The corpus shows one fixed block of settings, in one fixed order, in all 32
files. The order below is the corpus order.

| Key | Value form | Seen | Meaning |
|-----|-----------|------|---------|
| `IconForm` | quoted | 32 | The VB name of the form whose icon becomes the EXE icon. |
| `Startup` | quoted | 32 | The VB name of the startup form, or `"Sub Main"`. |
| `HelpFile` | quoted | 32 | Path to a help file. `""` in all 32. |
| `Title` | quoted | 32 | The application title. |
| `ExeName32` | quoted | 32 | The output file name. |
| `Command32` | quoted | 32 | Command line arguments for debugging. `""` in all 32. |
| `Name` | quoted | 32 | The VB project name. It is not the title and not the EXE name. |
| `HelpContextID` | quoted | 32 | `"0"` in all 32. A number inside quotes. |
| `CompatibleMode` | quoted | 32 | `"0"` in all 32. |
| `MajorVer` | bare int | 32 | |
| `MinorVer` | bare int | 32 | |
| `RevisionVer` | bare int | 32 | |
| `AutoIncrementVer` | bare int | 32 | `1` increments `RevisionVer` on each build. |
| `ServerSupportFiles` | bare int | 32 | `0` in all 32. |
| `VersionComments` | quoted | 32 | |
| `VersionCompanyName` | quoted | 28 | |
| `VersionProductName` | quoted | 10 | |
| `VersionLegalCopyright` | quoted | 13 | |
| `VersionFileDescription` | quoted | 4 | |
| `CompilationType` | bare int | 32 | |
| `OptimizationType` | bare int | 32 | |
| `FavorPentiumPro(tm)` | bare int | 32 | |
| `CodeViewDebugInfo` | bare int | 32 | |
| `NoAliasing` | bare int | 32 | |
| `BoundsCheck` | bare int | 32 | |
| `OverflowCheck` | bare int | 32 | |
| `FlPointCheck` | bare int | 32 | |
| `FDIVCheck` | bare int | 32 | |
| `UnroundedFP` | bare int | 32 | |
| `StartMode` | bare int | 32 | |
| `Unattended` | bare int | 32 | |
| `Retained` | bare int | 32 | |
| `ThreadPerObject` | bare int | 32 | |
| `MaxNumberOfThreads` | bare int | 32 | |

All **(a)**.

The key name `FavorPentiumPro(tm)` contains parentheses and the letters `tm`.
This is not a typing error. **(a)** 32 of 32, and **(b)** vb6parse uses the
same literal.

Three of the four version keys are optional. `VersionComments` appears in all
32 files, but the other four version strings appear only when the developer
filled the field in. **(c)** An empty version field is omitted, not written as
`""`. The corpus shows `HelpFile=""` and `Command32=""`, so an empty value is
sometimes written. The difference is that the version strings live in a
different IDE dialog.

### 1.5 Compilation settings, and what the values mean

`CompilationType=0` selects native code. `CompilationType=-1` selects P-code.
**(b)** vb6parse `CompilationType`, and this agrees with the project's own
`CORPUS.md`, which found `CompilationType=0` in every corpus project and
confirmed each EXE is native.

`OptimizationType` takes `0` for fast code, `1` for small code, `2` for no
optimisation. **(b)** vb6parse `OptimizationType`. The corpus is `0` in all 32.

The remaining compiler flags are VB booleans. `0` is False and `-1` is True.
The sense of each flag is easy to get backwards, so read the table.

| Key | `0` means | `-1` means | Corpus |
|-----|-----------|------------|--------|
| `FavorPentiumPro(tm)` | Do not favour Pentium Pro | Favour Pentium Pro | `-1` in 32 |
| `CodeViewDebugInfo` | Do not create symbolic debug info | Create it | `0` in 32 |
| `NoAliasing` | Assume aliasing | Assume no aliasing | `-1` in 32 |
| `BoundsCheck` | Check array bounds | **Remove** bounds checks | `-1` in 32 |
| `OverflowCheck` | Check integer overflow | **Remove** overflow checks | `-1` in 32 |
| `FlPointCheck` | Check floating point errors | **Remove** those checks | `-1` in 32 |
| `FDIVCheck` | Check the Pentium FDIV bug | **Do not** check it | `-1` in 32 |
| `UnroundedFP` | Do not allow unrounded floating point | Allow it | `-1` in 32 |

Value meanings are **(b)**, from the vb6parse `compilesettings` enums. Corpus
counts are **(a)**.

The `-1` in `BoundsCheck` means the "Remove Array Bounds Checks" box is ticked.
A decompiler cannot recover these flags from a native EXE with certainty.
**(c)** An emitter should write the IDE defaults, which are `0` for every flag
except `FavorPentiumPro(tm)`, and record the choice in the report.

### 1.6 Keys not in the corpus

These are documented but absent. **(b)** vb6parse handler table.

`Path32`, `ResFile32`, `CondComp`, `CompatibleEXE32`, `VersionCompatible32`,
`DllBaseAddress`, `ThreadingModel`, `DebugStartupOption`,
`DebugStartupComponent`, `NoControlUpgrade`, `RemoveUnusedControlInfo`,
`UseExistingBrowser`, `VbIntellisenseFix`, `CompiledReference`,
`SubProjectReference`, `ThirdPartySection`, `VersionLegalTrademarks`.

`Path32` holds the directory of the last compile. **(b)** It is written by the
IDE after a build, not required to load.

`ResFile32` names a `.res` resource file. A VB6 build task is known to fail when
`ResFile32` is present and the file is missing, so an emitter must write the key
only when it writes the file. **(b)** The nant/nantcontrib pull request 27
exists for this reason.

### 1.7 What is mandatory

No document states a minimum key set. The claim below is **(c)**, from the
grammar of the loader.

Mandatory in practice:

- `Type=`. Without it the IDE cannot choose a project kind.
- At least one component line. A project with no form and no module has nothing
  to compile.
- `Startup=`. It must name a form that a `Form=` line brings in, or
  `"Sub Main"`.

Everything else has a default. The IDE fills a missing setting from its own
default and writes the full block back on the next save.

Ordering: no ordering requirement is documented, and the vb6parse parser
dispatches on the key name without caring about order. **(b)** But `Type=` is
the first line in 32 of 32 corpus files **(a)**, and section headers must follow
all top level keys because a key after a `[Section]` line belongs to that
section. **(c)**

**Rule for the emitter.** Write `Type=` first. Write the component lines in
recovered order. Write the setting block in the corpus order given in 1.4.
Write `[MS Transaction Server]` and `AutoRefresh=1` last.

---

## 2. The `.frm` grammar

### 2.1 File layout

A `.frm` file has four parts, in this order:

1. The `VERSION` line.
2. Zero or more `Object` lines.
3. One `Begin ... End` block for the form.
4. Five `Attribute` lines, then the code.

**(a)** for parts 1, 3 and 4 in 36 of 36 files. Part 2 is **(a)** in
`VBIDEUtils/VBDoc.frm` and **(b)** in vb6parse, which holds
`objects: Vec<ObjectReference>` on its `FormFile` type.

### 2.2 The `VERSION` line

```
VERSION 5.00
```

It is the first line, at column 0, with one space, and a two digit minor part.
**(a)** 36 of 36 files carry exactly this text. A `.ctl` file uses the same
line. **(a)** `pdLabel.ctl`.

The class file uses a different line. See section 5.2.

### 2.3 The `Object` lines

When the form holds a control from an OCX, the IDE writes one `Object` line for
each OCX, after `VERSION` and before `Begin`:

```
VERSION 5.00
Object = "{831FDD16-0C5C-11D2-A9FC-0000F8754DA1}#2.1#0"; "MSCOMCTL.OCX"
Object = "{F9043C88-F6F2-101A-A3C9-08002B2F49FB}#1.2#0"; "comdlg32.ocx"
Begin VB.Form frmVBDocumentor 
```

**(a)** `VBIDEUtils/VBDoc.frm`, and **(b)** vb6parse test `object_statement_parsing`.

Compare with the `.vbp` form of the same data in 1.3. In a `.frm` there are
spaces around the `=`, and both the GUID part and the file name are quoted. In a
`.vbp` there are no spaces around the `=` and nothing is quoted. An emitter that
copies one shape into the other file produces a file the IDE cannot use.

The corpus has no example, because the one corpus project that declares an OCX
in its `.vbp` (`Transparency.vbp`, `COMDLG32.OCX`) does not place the control on
a form. **(a)**

### 2.4 The `Begin ... End` tree

```
Begin <Class> <Name> 
   <properties>
   <BeginProperty blocks>
   <nested Begin blocks>
End
```

Rules:

1. A `Begin` line ends with **one trailing space** after the control name.
   **(a)** 487 of 487 `Begin` lines in the corpus. Also true in `VBDoc.frm` and
   `pdLabel.ctl`.
2. A `BeginProperty` line also ends with one trailing space. **(a)** 235 of 235.
3. `End` and `EndProperty` carry **no** trailing space. **(a)** 487 and 235.
4. Indentation is exactly `3 * depth` spaces. The form block itself is at depth
   0, so its `Begin` and `End` are at column 0 and its properties are at column
   3. **(a)** Every indent measured in the corpus is 0, 3, 6, 9 or 12.
5. A `BeginProperty` block raises the depth by one, like a `Begin` block.
   **(a)**
6. The class is `VB.<Name>` for an intrinsic control, and
   `<LibraryPrefix>.<Name>` for an OCX control, for example
   `MSComDlg.CommonDialog` or `MSComctlLib.ListView`. **(a)**
7. The top level class is `VB.Form`, `VB.MDIForm` or `VB.UserControl`. **(a)**
   for `VB.Form` (36) and `VB.UserControl` (1, in `pdLabel.ctl`). **(b)**
   vb6parse `FormRoot` is `Form` or `MDIForm`.

Control classes seen in the corpus, with counts: `VB.Label` 125,
`VB.PictureBox` 80, `VB.TextBox` 66, `VB.Menu` 46, `VB.CommandButton` 45,
`VB.Form` 36, `VB.HScrollBar` 22, `VB.Frame` 19, `VB.CheckBox` 15,
`VB.OptionButton` 12, `VB.ComboBox` 10, `VB.Line` 7, `VB.ListBox` 3,
`VB.VScrollBar` 1. **(a)**

### 2.5 Ordering inside a block

Three ordering rules hold, and two of them are load requirements.

**Properties come before child blocks.** In every one of the 487 blocks in the
corpus, every `<name> = <value>` line precedes every nested `Begin` line.
**(a)** 0 violations found by script.

**Menus come last.** Inside a form block, every `VB.Menu` child follows every
non-menu child. **(a)** 0 violations in 36 files. This one is a hard rule:
Microsoft documents the load error `Line 'item1': All controls must precede
menus; can't load control 'item2'.` **(b)**

**Properties are sorted.** Inside a control block or a form block, the property
names are in case insensitive ascending order, and a `BeginProperty` block sorts
by its property name in the same list. **(a)** 487 of 487 blocks pass this test
by script. A `BeginProperty Font` block is the one exception: its seven keys use
a fixed order, `Name`, `Size`, `Charset`, `Weight`, `Underline`, `Italic`,
`Strikethrough`. **(a)** 235 of 235.

Alphabetical order applies to intrinsic VB controls. An OCX control writes its
own persistence order, which is not alphabetical. In `VBDoc.frm` the
`MSComctlLib.ListView` block runs `Height`, `Left`, `TabIndex`, `Top`, `Width`,
`_ExtentX`, `_ExtentY`, `View`, `LabelWrap`, `HideSelection`, `_Version`,
`ForeColor`, `BackColor`, `BorderStyle`, `Appearance`, `NumItems`. **(a)**

Sorting is not a load requirement. **(c)** The loader reads properties in file
order. Sorting matters because a decompiler that sorts produces output the IDE
will not reorder on save, which makes a byte comparison meaningful.

### 2.6 What is legal inside a `Begin` block

**An apostrophe comment is not legal as a line inside a `Begin` block.** The
project's assumption is **correct**.

Evidence:

- **(a)** Every one of the 7379 lines from `VERSION` up to the first
  `Attribute VB_Name`, across all 36 corpus forms, falls into exactly six
  classes: 5899 property lines, 487 `Begin`, 487 `End`, 235 `BeginProperty`,
  235 `EndProperty`, 36 `VERSION`. The counts add to 7379. There is **no** blank
  line and **no** comment line anywhere in the header region of any file.
- **(b)** Microsoft documents the parser as line oriented over a fixed set of
  line shapes, and gives errors for each shape that fails, for example
  `Line 'item1': Syntax error: property 'item2' in 'item3' was missing an equal
  sign (=)` and `Line 'item1': 'item2' has a quoted string where the property
  name should be`. A line starting with an apostrophe matches none of the
  accepted shapes.
- **(b)** Microsoft documents `Line 'item1': The file 'item2' could not be
  loaded.` with the cause "Syntax errors are preventing Visual Basic from
  parsing and loading a file", and the outcome "The form won't be loaded and the
  form name won't be displayed in the Project Explorer".

The trailing apostrophe **comment on a property value line is legal**, because
the IDE writes it itself. See section 3.4. It is a decoration on the value, not
a free comment. An emitter must not use it to carry a message. **(c)**

A blank line inside a `Begin` block is also not written by the IDE. **(a)** 0 of
7379. Treat it as illegal. **(c)**

**Consequence for DeForm6.** Uncertainty markers can go only in the code region,
after the last `Attribute` line. Structural doubt about a control or a property
must go to the report, not to the `.frm`. This confirms the Key Decision already
in `PROJECT.md`.

### 2.7 The `Attribute` block and the code section

Directly after the `End` that closes the form block, with no blank line, come
exactly five `Attribute` lines, in this order:

```
Attribute VB_Name = "frmCurves"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
```

**(a)** 36 of 36 forms have exactly these five keys, with these four fixed
values, and only `VB_Name` varies.

These lines start at column 0 and use `SP=SP` around the `=`, not the padded
form used inside the `Begin` block. **(a)**

The code section starts on the line after `Attribute VB_Exposed`. **(a)** From
there the file is ordinary Basic source, and an apostrophe comment is legal.
**(a)** All 36 corpus forms begin the code region with a comment block.

`VB_Name` must match the value that the `.vbp` `Startup=` and `IconForm=` lines
use. **(a)** For example `Curves.vbp` says `Startup="frmCurves"` and
`Curves.frm` says `Attribute VB_Name = "frmCurves"`.

The IDE decides what kind of component a file is from its content, not from its
extension. A `.frm` without the `Begin VB.Form` block loads as a module. **(b)**
Reported behaviour in the Experts Exchange thread "Forms Loading As Modules".
Treat this as **(c)** on the exact mechanism.

---

## 3. Property serialisation in a `.frm`

### 3.1 The line template

```
<indent><name padded to 16><=><3 spaces><value><CRLF>
```

- `indent` is `3 * depth` spaces.
- `name` is left justified and padded with spaces to a field of **16**
  characters. **(a)** In all 5899 property lines the `=` sits at offset 17 from
  the first non-space character.
- After the `=` come exactly **three** spaces. **(a)** All 5899 lines have the
  four byte run `=SPSPSP`.

The longest property name in the corpus is 15 characters (`StartUpPosition`,
`FontTransparent`), so the padding is never squeezed. **(c)** For a name of 16
characters or more, write one space, because the `Object.Width` case in 3.7
shows the field is a minimum and not a truncation.

Example, taken byte for byte from `Curves-effect/Curves.frm`:

```
   BackColor       =   &H80000005&
   BorderStyle     =   1  'Fixed Single
   Caption         =   "Image Curves - tannerhelland.com"
   ClientHeight    =   8835
```

### 3.2 Integers and longs

A plain signed decimal, no thousands separator, no sign for a positive number.
**(a)** 3061 lines. Examples: `8835`, `120`, `-100`, `-255`, `393216`.

There is no distinct notation for an Integer and a Long. **(a)** An emitter
writes the decimal digits and lets the IDE coerce.

### 3.3 Strings

A double quoted string. A double quote inside the string is doubled. **(a)** 621
string lines, of which 3 contain a doubled quote:

```
Caption         =   "Animate ""Explosion"""
Caption         =   "Sepia / ""Antique"" Effect - www.tannerhelland.com"
```

No backslash escape exists. **(a)** No backslash escape appears in any of the
621 lines, and the corpus contains file paths written with plain backslashes
elsewhere.

There is no way to write a line break inside an inline string. A string that
holds a line break must go to the `.frx`. **(c)** The one corpus example of a
multi-line `Text` property uses the `.frx`, and the `.frm` grammar is line
oriented, so a raw CRLF inside the quotes would end the line.

The longest inline string in the corpus is 97 characters. The one string that
went to the `.frx` is 153 characters. **(a)** The threshold is therefore between
98 and 153 and is **not resolved**. See section 9.

Microsoft documents both failure directions:
`Line 'item1': Property 'item2' in 'item3' must be a quoted string.` and
`Line 'item1': 'item2' has a quoted string where the property name should be.`
**(b)**

### 3.4 Booleans

VB writes `0` or `-1`, then padding, then an apostrophe and the word.

```
MaxButton       =   0   'False
MultiLine       =   -1  'True
```

The value sits in a field of width 2, then two spaces, then the comment. So
`0` is followed by three spaces and `-1` by two. **(a)** 785 lines of
`0␣␣␣'False` and 202 of `-1␣␣'True`. There is no other spelling in the corpus.

### 3.5 Enumerations

VB writes the number, then **two** spaces, then an apostrophe and the name of
the member.

```
BorderStyle     =   1  'Fixed Single
ScaleMode       =   3  'Pixel
StartUpPosition =   2  'CenterScreen
MousePointer    =   99  'Custom
```

The value is not padded. A two digit value keeps the same two spaces. **(a)**
424 lines of `0␣␣'`, 30 of `1␣␣'`, 41 of `2␣␣'`, 119 of `3␣␣'`, 4 of `4␣␣'`,
2 of `99␣␣'`.

All enumeration names seen: `Flat`, `None`, `Solid`, `Transparent`, `Checked`,
`Fixed Single`, `Right Justify`, `Center`, `CenterScreen`, `Cross`,
`Dropdown List`, `Both`, `Pixel`, `Windows Default`, `Fixed ToolWindow`,
`Custom`. **(a)**

The comment is decoration. **(c)** The loader takes the number. An emitter that
knows the member name should write it, because the IDE writes it, and an
emitter that does not know the name can omit the whole comment safely.

### 3.6 Colours

An intrinsic VB control writes a colour as `&H` then exactly **eight upper case
hex digits** then `&`.

```
BackColor       =   &H80000005&
BackColor       =   &H00FFFFFF&
ForeColor       =   &H80000008&
```

**(a)** All 446 colour values have exactly 8 digits, and none uses a lower case
letter. The properties that carry a colour in the corpus are `BackColor` (240),
`ForeColor` (203) and `FillColor` (3).

The layout is `00BBGGRR` for a literal colour and `8000000X` for a system
colour, where `X` selects a `vbWindowBackground` style constant. **(c)** The
corpus values match this reading. `&H00FFFFFF&` is white, `&H000000FF&` is red,
`&H00C0C0C0&` is light grey, and every value starting `&H8` has the shape
`8000000X` with `X` from 5 to 8.

**An OCX control writes a colour as a plain signed decimal, not as `&H`.** In
`VBDoc.frm` the `MSComctlLib.ListView` block has `ForeColor = -2147483640` and
`BackColor = -2147483643`, which are the same values as `&H80000008&` and
`&H80000005&`. **(a)** An emitter must branch on the control class.

### 3.7 The `Object.` prefix on an OCX property

An OCX property that the container forwards to the OLE object is written with an
`Object.` prefix, and the padding is applied to the bare name before the prefix
is added:

```
         Text            =   "Tag Type"
         Object.Width           =   3528
```

`Object.` is 7 characters, then `Width` padded to 16, so the `=` moves right by
7. **(a)** `VBDoc.frm`.

### 3.8 Floating point

Only the font size uses a fraction.

```
Size            =   8.25
Size            =   9.75
```

**(a)** 134 lines, two distinct values. The decimal separator is a full stop.
**(a)** An emitter must write a full stop and must not follow the machine
locale. **(c)**

### 3.9 The `Font` property block

```
      BeginProperty Font 
         Name            =   "Segoe UI"
         Size            =   9.75
         Charset         =   0
         Weight          =   400
         Underline       =   0   'False
         Italic          =   0   'False
         Strikethrough   =   0   'False
      EndProperty
```

The seven keys always appear, always in this order, always all seven. **(a)**
235 of 235 blocks. `Name` is a string, `Size` a float, `Charset` and `Weight`
integers, and the last three booleans.

The block sorts into the parent list under the letter F. **(a)**

An OCX writes a named and indexed property block with a GUID after the name:

```
      BeginProperty ColumnHeader(1) {BDD1F052-858B-11D1-B16A-00C0F0283628} 
         Text            =   "Tag Type"
         Object.Width           =   3528
      EndProperty
```

**(a)** `VBDoc.frm`. The trailing space is still there. Microsoft documents the
matching load error `Line 'item1': The CLSID 'item2' for 'item3' is invalid.`
and states that it "Applies only to objects that are properties, such as the
Font object". **(b)**

### 3.10 Menu shortcuts

The `Shortcut` property is written **unquoted**, as a caret and a letter:

```
         Shortcut        =   ^N
         Shortcut        =   ^O
```

**(a)** 6 lines. Verified at byte level: the value is the two ASCII bytes `5E
4E`, not a control character.

### 3.11 Binary and long string references

Two forms exist, and the difference matters.

```
Picture         =   "Brightness.frx":0000
Icon            =   "frmFire.frx":0000
Text            =   "frmHMM.frx":0000
Caption         =   $"Gradient.frx":0000
```

- The file name is quoted. It may contain a space: `"Main Editor.frx":0000`.
  **(a)**
- The offset follows a colon, has **no** `0x` or `&H`, is upper case hex, and is
  padded with leading zeros to **at least four** digits. **(a)** 21 references
  seen. Widths: `0000`, `0074`, `00E8`, `1E2D`, `B2C5`, `C757`. The value
  `0x74` is written `0074`, so the pad is to four, and a five digit offset would
  simply be five digits. **(c)**
- The `$` prefix marks a **string** record. Without it, the record is the
  control's own binary persistence. See section 4. **(a)**

Microsoft documents the failure:
`Line 'item1': Property 'item2' in 'item3' had an invalid file reference.`
with the cause "a reference to a file that Visual Basic couldn't find in the
specified directory". **(b)**

### 3.12 Control arrays

A member of a control array carries an `Index` property, and several `Begin`
blocks share the same name.

```
            Index           =   0
            Index           =   1
```

**(a)** 20 `Index` lines in the corpus, values 0 to 7.

Microsoft documents the failure:
`Line 'item1': Did not find an index property, and control 'item2' already
exists.` **(b)** So two blocks with the same name and no `Index` is a load
error. An emitter must add `Index` whenever a name repeats.

---

## 4. The `.frx` format

### 4.1 File shape

A `.frx` file has **no header**. It is a bare sequence of variable length
records. Each record starts at the byte offset that the `.frm` gives. Records
sit end to end with no padding and no alignment. **(a)** and **(b)**.

Proof of the packing, from `Game-physics-basic/FormPhysics.frx`, 58938 bytes.
The `.frm` references offsets `0000, 0074, 00E8, 1E2D, 3D10, 573E, 7373, 8E05,
AA12, C757`. For each record, the first 32 bit value plus 4 gives exactly the
next referenced offset, and the last record ends exactly at the end of file.
**(a)**

```
off 0x0000: u32=112   next=0x0074
off 0x0074: u32=112   next=0x00E8
off 0x00E8: u32=7489  next=0x1E2D
off 0x1E2D: u32=7903  next=0x3D10
off 0x3D10: u32=6698  next=0x573E
off 0x573E: u32=7217  next=0x7373
off 0x7373: u32=6798  next=0x8E05
off 0x8E05: u32=7177  next=0xAA12
off 0xAA12: u32=7489  next=0xC757
off 0xC757: u32=7903  next=0xE63A = file size
```

All integers are little endian. **(a)**

### 4.2 Picture and icon records, and the `lt\0\0` marker

The marker is present. The layout is:

| Offset | Size | Content |
|--------|------|---------|
| 0 | 4 | `u32` size of everything that follows this field |
| 4 | 4 | The four bytes `6C 74 00 00`, that is `l`, `t`, `NUL`, `NUL` |
| 8 | 4 | `u32` size of the payload, equal to the first field minus 8 |
| 12 | n | The payload, an ordinary image file |

**(a)** 19 records across 8 `.frx` files. The relation `field0 - 8 == field2`
holds in all 19. One of the 19 is the empty case of section 4.3.

Worked example, `Fire-effect/frmFire.frx`, whole file 1418 bytes, referenced by
`Icon = "frmFire.frx":0000`:

```
00000000: 8605 0000 6c74 0000 7e05 0000 0000 0100
00000010: 0100 1010 0000 0000 0000 6805 0000 1600
00000020: 0000 2800 0000 1000 0000 2000 0000 0100
```

- `0x00000586` = 1414. 1418 = 4 + 1414.
- `6c74 0000` is the marker.
- `0x0000057E` = 1406 = 1414 - 8.
- The payload starts `00 00 01 00 01 00`, which is an `ICONDIR`: reserved 0,
  type 1, count 1. The `ICONDIRENTRY` that follows says 16 by 16, 1384 bytes at
  offset 22, and 1384 + 22 = 1406. The payload is a complete `.ico` file.

**(a)** The payload is stored in its original file format. `gfxfromfrx` states
the same, listing BMP, GIF, JPEG, WMF, EMF, CUR and ICO. **(b)**

All 1418 bytes of this record, including the leading `u32`, appear byte for
byte inside `Fast_Flames.exe`, at file offset `0x13D5`. **(a)** So the record
format in the `.frx` and in the compiled form data is the same for pictures.
This is the single most useful fact in this file: DeForm6 can copy a picture
record out of the EXE and into the `.frx` without reserialising anything.

### 4.3 The empty picture record

`Transparency-2D/frmTransparency.frx` offset 0 holds:

```
08 00 00 00 6C 74 00 00 00 00 00 00
```

That is a 12 byte record with a payload of zero bytes. The `.frm` references it
as `Icon = "frmTransparency.frx":0000`. **(a)**

vb6parse names this case and gives the cause: the developer added an icon to the
form and then removed it, and the IDE left the empty header behind. **(b)**

An emitter should write this exact 12 byte record when a picture property exists
but the blob is absent. **(c)**

### 4.4 Long string records, the `$` form

The record is a `u32` little endian byte count, then the text.

`Gradient-2D/Gradient.frx`, whole file 157 bytes, referenced by
`Caption = $"Gradient.frx":0000`:

```
00000000: 9900 0000 436c 6963 6b20 6f6e 2061 2070  ....Click on a p
```

`0x99` = 153, and 4 + 153 = 157. The 153 bytes are the caption text with no
terminator. **(a)**

The count **excludes** the four header bytes. vb6parse documents its
`Record4ByteHeader` as "Size of data including header". **The corpus disagrees,
and this file follows the corpus.** The arithmetic is exact and leaves no room
for another reading.

The text is Windows-1252, not UTF-8, and carries no BOM. **(a)** and **(b)**
vb6parse decodes `.frx` text with `encoding_rs::WINDOWS_1252`.

### 4.5 Short string records, the plain form

`Hidden-Markov-model/frmHMM.frx` is referenced by
`Text = "frmHMM.frx":0000` on a multi-line `VB.TextBox`.

**The committed file is damaged. Do not use it as a fixture.** Its 56 bytes are:

```
00000000: 3820 596f 7572 2048 4d4d 2061 6e61 6c79
...
00000030: 6578 7420 626f 780a
```

The repository `.gitattributes` sets `* text=auto`. This file holds no NUL byte,
so git classified it as text and normalised its line endings on commit or
checkout. One `CR` was removed. **(c)**

The original record is recoverable from the compiled EXE. `HMM.exe` holds:

```
... 0B 38 00 20 'Your HMM analyses output will appear in this text box' 0D 0A 00 ...
```

The 56 byte payload is `SP` + 53 characters + CRLF. **(a)**

The `.frx` record is therefore:

| Offset | Size | Content |
|--------|------|---------|
| 0 | 1 | `u8` byte count, here `0x38` = 56 |
| 1 | n | The text, Windows-1252, CRLF inside |

1 + 56 = 57, and git removing one `CR` gives the 56 bytes on disk. **(c)** The
arithmetic closes exactly, and vb6parse documents a matching
`Record1ByteHeader` for "Small data (< 256 bytes)". **(b)**

Note that the compiled form data uses a `u16` count (`38 00`) for the same
string, while the `.frx` uses a `u8` count. The two serialisations differ for
strings even though they agree for pictures. **(a)**

### 4.6 Record kinds not seen in the corpus

These are **(b)** only, from vb6parse `ResourceEntry`. Treat them as unverified.

**List items**, used by `ComboBox.List` and `ListBox.List`:

| Offset | Size | Content |
|--------|------|---------|
| 0 | 2 | `u16` number of items |
| 2 | 2 | Magic, `03 00` or `07 00` |
| 4 | .. | For each item: `u16` length, then the bytes, no terminator |

**A three byte header record**, used for small to medium text:

| Offset | Size | Content |
|--------|------|---------|
| 0 | 1 | `0xFF` marker |
| 1 | 2 | `u16` size |
| 3 | n | The data |

vb6parse adds a warning on this kind: "VB6 IDE has an off-by-one bug where some
records are marked as N bytes but are actually N-1 bytes." **(b)** A reader must
clamp to the end of file.

### 4.7 What a reader must guard

The record kind is not self describing. Only the property that references the
offset tells you which layout to expect. **(c)** An emitter has it easier: it
chooses the property and the record together.

A safe reader:

1. Never trusts a length field. Clamp every length to the remaining file size.
2. Sniffs `6C 74 00 00` at offset+4 before assuming a picture.
3. Treats an offset beyond the end of file as a recoverable error, not a panic.

### 4.8 The `.ctx` file

A `.ctl` file uses a `.ctx` resource file instead of a `.frx`, referenced the
same way: `ToolboxBitmap = "pdLabel.ctx":0000`. **(a)** `pdLabel.ctl`. The
record grammar is assumed identical. **(c)** Not verified.

---

## 5. The `.bas`, `.cls` and `.ctl` grammars

### 5.1 The `.bas` file

The whole header is one line:

```
Attribute VB_Name = "Logic_Module"
```

Then the code starts on the next line. **(a)** 7 of 7 files. There is no
`VERSION` line and no `BEGIN` block.

The value is the VB name of the module. It must match the first field of the
`Module=` line in the `.vbp`. **(a)** For example `Physics.vbp` says
`Module=Logic_Module; Physics_Logic.bas`.

The line starts at column 0 and uses `SP=SP`. **(a)**

### 5.2 The `.cls` file

The preamble is thirteen lines and is byte identical in all 46 corpus classes
apart from the name. **(a)** A single MD5 over the normalised preamble of all 46
files gives one value.

```
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
  Persistable = 0  'NotPersistable
  DataBindingBehavior = 0  'vbNone
  DataSourceBehavior  = 0  'vbNone
  MTSTransactionMode  = 0  'NotAnMTSObject
END
Attribute VB_Name = "FastDrawing"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = True
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
```

Layout notes, all **(a)**:

- The version line is `VERSION 1.0 CLASS`, with a one digit minor part. It is
  **not** `VERSION 5.00`.
- `BEGIN` and `END` are upper case and alone on their line, at column 0.
  Microsoft's grammar is enforced by vb6parse with the errors "The 'BEGIN'
  keyword should stand alone on its own line" and the same for `END`. **(b)**
- The five property lines are indented by **two** spaces, not three.
- The name field is padded to a width of **20**, not 16. `MultiUse` is 8
  characters and gets 1 space, `DataBindingBehavior` is 19 and gets 1,
  `DataSourceBehavior` is 18 and gets 2, `MTSTransactionMode` is 18 and gets 2.
- After the `=` comes **one** space, not three.
- The boolean and enumeration comment uses two spaces before the apostrophe,
  the same as an enumeration in a `.frm`.

The `.cls` block therefore uses a different layout from the `.frm` block. An
emitter that reuses one writer for both produces the wrong bytes. **(c)**

Meaning of the five properties, all **(b)** from vb6parse
`files/class/properties.rs`:

| Property | Values | Meaning |
|----------|--------|---------|
| `MultiUse` | `0` SingleUse, `-1` MultiUse | COM instancing. Meaningful only in an ActiveX DLL project that is public and creatable. |
| `Persistable` | `0` NotPersistable, `-1` Persistable | Whether the class can save itself to a property bag. Adds `InitProperties`, `ReadProperties`, `WriteProperties` and `PropertyChanged`. |
| `DataBindingBehavior` | `0` vbNone, and others | Default data binding behaviour. |
| `DataSourceBehavior` | `0` vbNone, `1` DataSource | Whether the class can act as a data source. |
| `MTSTransactionMode` | `0` NotAnMTSObject, `1` NoTransactions, `2` RequiresTransaction, `3` UsesTransaction, `4` RequiresNewTransaction | MTS component mode. |

For a Standard EXE project, all five are inert and take the value `0`, except
`MultiUse`, which the IDE writes as `-1`. **(a)** 46 of 46.

Meaning of the five attributes:

| Attribute | `.cls` value | `.frm` value | Meaning |
|-----------|--------------|--------------|---------|
| `VB_Name` | the class name | the form name | The name the code uses. |
| `VB_GlobalNameSpace` | `False` | `False` | Whether members are reachable without qualifying them. |
| `VB_Creatable` | `True` | `False` | Whether outside code may call `New` on it. A form is created by the IDE, so `False`. |
| `VB_PredeclaredId` | `False` | `True` | Whether a global instance with the same name exists. This is why `frmMain.Show` works without `New`. |
| `VB_Exposed` | `False` | `False` | Whether the class is exposed to COM. |

Values are **(a)**, 46 of 46 for classes and 36 of 36 for forms. The meanings
are **(c)**, from VB6 working knowledge, and are consistent with the corpus.

All five attributes are mandatory in the sense that the IDE writes all five
every time. **(a)** No corpus file omits one. Whether the loader tolerates a
missing attribute is **not verified**. See section 9.

### 5.3 The `.ctl` file

Not in the corpus. From `pdLabel.ctl`, **(a)**:

```
VERSION 5.00
Begin VB.UserControl pdLabel 
   Appearance      =   0  'Flat
   ...
   ToolboxBitmap   =   "pdLabel.ctx":0000
End
Attribute VB_Name = "pdLabel"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = True
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
```

It uses the `.frm` layout, with `VB.UserControl` as the root class and `True`
and `False` swapped on `VB_Creatable` and `VB_PredeclaredId`. Its resource file
is `.ctx`.

---

## 6. Text encoding and line endings

### 6.1 Encoding

VB6 writes and reads a single byte ANSI file in the code page of the machine
that saved it. On a Western machine that is Windows-1252.

**(a)** 121 of 121 corpus text files parse as single byte text. Two distinct
non-ASCII bytes appear: `0xA9` in 7 `.vbp` files (the copyright sign in
`VersionComments`) and `0xAE` in 20 `.cls` files (the registered sign). Both are
correct Windows-1252 and are not valid UTF-8 lead bytes on their own.

**(b)** vb6parse states "The library assumes that VB6 source files are encoded
in Windows-1252" and decodes with `encoding_rs::WINDOWS_1252`.

There is no encoding declaration anywhere in any of these formats. **(a)** A
reader must be told the code page, or guess it, or use Windows-1252.

**Rule for the emitter.** Write Windows-1252. When a recovered string holds a
character that Windows-1252 cannot represent, do not fall back to UTF-8. Replace
the character, and record the substitution in the report. **(c)** A UTF-8 byte
pair inside a `.frm` string would show as two wrong glyphs in the IDE, which is
worse than one replacement character because it hides the loss.

### 6.2 Line endings

Every line ends with CRLF. Every file ends with CRLF.

**(a)** Measured over all 121 text files: the count of `LF` equals the count of
`CRLF` in every file, so there is no bare `LF` anywhere. The last two bytes of
all 121 files are `0D 0A`.

The corpus repository sets `eol=crlf` in `.gitattributes` for `.bas`, `.cls`,
`.ctl`, `.frm`, `.txt` and `.vbp`, which is an author statement that CRLF is
required. **(a)**

**Rule for the emitter.** Write CRLF and a final CRLF. Never write a bare LF.
**(c)** A bare LF is not proved to break the loader, but no IDE written file
contains one, so it is untested ground.

**Warning about the `.frx`.** The corpus `.gitattributes` applies `* text=auto`
and `*.frx -diff`. The `-diff` setting stops a diff. It does **not** stop line
ending normalisation. A `.frx` that holds no NUL byte is treated as text and is
corrupted. This is exactly what happened to `frmHMM.frx`. See section 4.5.
**(a)** DeForm6 must set `-text` or `binary` on `.frx` and `.ctx` in its own
`.gitattributes` before it commits any fixture.

### 6.3 Byte order marks

No corpus file has a BOM. The first three bytes of all 121 files are `Att`,
`Typ` or `VER`. **(a)**

A UTF-8 BOM at the start of a `.frm` places three bytes before the word
`VERSION`. The parser reads the first line and finds no keyword it knows.
**(c)** Expect a load failure. vb6parse has no BOM handling code, which supports
the view that a real file never has one. **(b)**

**Rule for the emitter.** Never write a BOM.

---

## 7. What makes the IDE refuse a file

### 7.1 The mechanism

The IDE parses the ASCII form line by line, converts it to a binary form in
memory, and writes the ASCII form back on save. When it hits an error it writes
a message to a log file beside the form, with the same base name and a `.log`
extension. It replaces the log on the next failed load. **(b)** Microsoft, "Form
File Loading Errors".

The log line format is `Line <n>: <message>`, for example
`Line 59: Property Picture in cmdSelect could not be set.` **(b)**

**This is a gift for DeForm6.** The acceptance test for "the IDE can open the
project" is: open the project, and assert that no `.log` file appeared next to
any `.frm`. That is machine checkable.

### 7.2 The documented failure list

Microsoft ships 31 `Line 'item1':` messages. All are **(b)**, from the
`MicrosoftDocs/VBA-Docs` repository, path
`Language/Reference/User-Interface-Help/`. The list below groups them by what
the emitter must do.

**Fatal. The form does not load at all.**

| Message | Cause |
|---------|-------|
| The file 'item2' could not be loaded. | Syntax errors stop the parse, or a form name conflicts with another form. The form does not appear in the Project Explorer. |
| The Form or MDIForm name 'item2' is not valid; can't load this form. | The name is not a valid VB identifier. |
| The Form or MDIForm name 'item2' is already in use; can't load this form. | Two forms in the project share a name. |

**The control is dropped, and its code becomes orphaned.**

| Message | Cause |
|---------|-------|
| All controls must precede menus; can't load control 'item2'. | A non-menu control appears after a menu inside the form block. |
| Class 'item2' of control 'item3' was not a loaded control class. | The class is unknown. The `.vbp` is missing the matching `Object=` line, or the OCX is not registered. |
| Missing or invalid control class in file 'item2'. | The class name is unknown or is not a valid VB string. |
| Missing or invalid control name in file 'item2'. | No name follows the class on the `Begin` line. |
| The control name 'item2' is invalid. | The name is not a valid VB identifier. |
| Did not find an index property, and control 'item2' already exists. | A repeated control name with no `Index` property. |
| Can't load control 'item2'; name already in use. | The same, at a different stage. |
| Can't load control 'item2'; containing control not a valid container. | A `Begin` block is nested inside a control that is not a container. |
| Can't load control 'item2'; license not found. | A licensed OCX with no design time license on the machine. |
| Maximum nesting level for controls exceeded with 'item2'. | Controls nested more than **7** levels deep. |
| Can't create embedded object in 'item2'. | An OLE container failed. |

**The property is dropped or defaulted. The form still loads.**

| Message | Cause |
|---------|-------|
| Property 'item2' in 'item3' could not be loaded. | The property name is unknown. The property is skipped. |
| Property 'item2' in 'item3' could not be set. | The property is known but the set failed. A missing or mismatched `.frx` is the common cause. |
| Property 'item2' in 'item3' had an invalid value. | The value is out of range for the control. **The property is set to its default.** |
| Property 'item2' in 'item3' had an invalid file reference. | The `.frx` named by the property is not in the directory. |
| Property 'item2' in 'item3' must be a quoted string. | The quotation marks are missing. The line is ignored. |
| 'item2' has a quoted string where the property name should be. | A quoted string where a name was expected. |
| The property name 'item2' in 'item3' is invalid. | The name is not a property of that control. |
| Property 'item2' in control 'item3' had an invalid property index. | An index above **255**. |
| Syntax error: property 'item2' in 'item3' was missing an equal sign (=). | No `=` between the name and the value. |
| The CLSID 'item2' for 'item3' is invalid. | A bad GUID on a `BeginProperty` block. |
| Could not create reference: 'item2'. | A `Reference` could not be resolved. |

**Silently changed. This is the dangerous class.**

| Message | Effect |
|---------|--------|
| Class name too long; truncated to 'item2'. | A class name longer than **40** characters is cut to 40. |
| Control name too long; truncated to 'item2'. | A control name longer than **40** characters is cut to 40. |

A truncated name still loads. The code that refers to the full name then fails
to compile, and the cause is in a log file the user may never open. An emitter
must clamp every control and class name to 40 characters itself, and record the
clamp in the report. **(c)**

**Menu specific.**

| Message | Cause |
|---------|-------|
| Parent menu 'item2' can't be loaded as a separator. | A menu that has children was given a separator caption. |
| Can't set Checked property in menu 'item2'; parent menu can't be checked. | `Checked` on a menu that has children. |
| Can't set Shortcut property in menu 'item2'; parent menu cannot have a shortcut. | `Shortcut` on a menu that has children. |

### 7.3 Limits to respect

| Limit | Value | Source |
|-------|-------|--------|
| Control nesting depth | 7 | **(b)** Corpus maximum is 4. **(a)** |
| Control name length | 40 | **(b)** Corpus maximum is 21. **(a)** |
| Class name length | 40 | **(b)** |
| Property index | 255 | **(b)** |

### 7.4 Ways to fail that the documents do not cover

These are **(c)**, from the grammar and from the corpus.

1. A comment or a blank line inside a `Begin` block. See 2.6.
2. A missing `Attribute VB_Name` in a `.frm`. The file may load as a module.
3. A `Startup=` value in the `.vbp` that names a form no `Form=` line brings in.
4. A `.frm` that names a `.frx` the emitter did not write.
5. An offset in a `.frm` that points past the end of the `.frx`.
6. An `Object=` line in a `.frm` with no matching `Object=` line in the `.vbp`.
   The control class then fails to resolve.
7. A file written in UTF-8, or with a BOM, or with bare LF endings.
8. A `Reference=` path that points at a developer machine that no longer exists.
   The GUID usually saves this, but not always.
9. A `ResFile32=` key naming a `.res` file that is not there.

---

## 8. Emitter checklist

Use this as the acceptance list for the writer.

**Every text file**

- [ ] Windows-1252 bytes. No UTF-8. No BOM.
- [ ] CRLF on every line, including the last.

**`.vbp`**

- [ ] `Type=Exe` first.
- [ ] Component lines in one ordered list, kinds interleaved as recovered.
- [ ] `Module=` and `Class=` carry `VBName; FileName`, with `; ` between them.
- [ ] `Object=` unquoted with `; ` before the OCX file name.
- [ ] Setting block in the order of section 1.4.
- [ ] `Startup=` names a form that a `Form=` line brings in.
- [ ] `[MS Transaction Server]` and `AutoRefresh=1` last.
- [ ] No `ResFile32=` unless the `.res` file is written.

**`.frm`**

- [ ] `VERSION 5.00` on line 1.
- [ ] `Object = "{GUID}#m.n#lcid"; "FILE.OCX"` lines next, one per OCX used.
- [ ] `Begin VB.Form <Name> ` with one trailing space.
- [ ] Indent `3 * depth`.
- [ ] Property name padded to 16, then `=`, then three spaces.
- [ ] Properties sorted case insensitively, `BeginProperty` blocks sorted in.
- [ ] Properties before child blocks.
- [ ] Menus after all other controls.
- [ ] Nesting depth 7 or less.
- [ ] Every name 40 characters or less.
- [ ] Repeated control names carry `Index`.
- [ ] `End` with no trailing space.
- [ ] Five `Attribute` lines, `VB_Creatable = False`, `VB_PredeclaredId = True`.
- [ ] No comment and no blank line above the first `Attribute` line.

**Property values**

- [ ] Integer: bare decimal.
- [ ] String: double quotes, inner quote doubled, no line break.
- [ ] Boolean: `0` plus three spaces plus `'False`, or `-1` plus two spaces plus
      `'True`.
- [ ] Enumeration: number plus two spaces plus `'Name`.
- [ ] Colour on an intrinsic control: `&H` plus 8 upper case hex digits plus `&`.
- [ ] Colour on an OCX: signed decimal.
- [ ] Float: full stop separator.
- [ ] Font: `BeginProperty Font ` with trailing space, seven keys in fixed order.
- [ ] Blob: `"<file>.frx":<HEX>`, at least 4 hex digits, upper case.
- [ ] Long string: `$"<file>.frx":<HEX>`.

**`.frx`**

- [ ] Records packed end to end in the order the `.frm` references them.
- [ ] Picture: `u32 (n+8)`, `6C 74 00 00`, `u32 n`, then the image file bytes.
- [ ] Empty picture: `08 00 00 00 6C 74 00 00 00 00 00 00`.
- [ ] `$` string: `u32 n`, then n bytes of Windows-1252.
- [ ] Short string: `u8 n`, then n bytes.
- [ ] Offsets written into the `.frm` match the byte offsets actually used.

**`.bas`**

- [ ] One line, `Attribute VB_Name = "<Name>"`, then the code.

**`.cls`**

- [ ] `VERSION 1.0 CLASS`, `BEGIN`, the five properties at indent 2 with the
      name padded to 20 and one space after the `=`, `END`.
- [ ] Five `Attribute` lines, `VB_Creatable = True`,
      `VB_PredeclaredId = False`.

**Repository hygiene**

- [ ] `.gitattributes` marks `*.frx` and `*.ctx` as `binary`, not just `-diff`.

---

## 9. Gaps

These questions are open. Each one is a place where the emitter must choose a
safe default and the report must say so.

1. **The inline string threshold.** The point at which the IDE moves a string
   property from the `.frm` into the `.frx` is between 99 and 153 characters.
   The corpus bounds it but does not fix it. **Safe default:** always write the
   string inline when it is 97 characters or shorter and holds no line break.
   That is inside the proved inline range.

2. **The short string record header width.** Section 4.5 reconstructs a `u8`
   count from a damaged file plus the EXE. It is arithmetic, not a second
   observation. A second corpus with a healthy multi-line `TextBox` would settle
   it. **Safe default:** write the `$` form with a `u32` count, which is proved
   exactly by `Gradient.frx`, for every string that must go to the `.frx`.

3. **Whether the loader tolerates a missing `Attribute` line.** No corpus file
   omits one and no document states a requirement. **Safe default:** write all
   five, always.

4. **Whether `.vbp` key order matters.** Not documented, and the one independent
   parser does not care. **Safe default:** match the IDE order.

5. **List records.** No `ComboBox` or `ListBox` in the corpus stores its `List`
   in a `.frx`, so the list record layout in 4.6 is unverified.

6. **The three byte header record.** Unverified, including the off-by-one
   warning that vb6parse attaches to it.

7. **Non-Latin code pages.** All corpus files are Western. A Japanese or Cyrillic
   VB6 project uses a different code page, and nothing in the file says which.
   How DeForm6 chooses is an open design question.

8. **`.ctl` and `.ctx`.** One `.ctl` file was read. The `.ctx` record layout is
   assumed, not checked. This matters only when ActiveX control projects come
   into scope, which `PROJECT.md` places in a later milestone.

9. **`MDIForm`.** Not in the corpus. The root class is `VB.MDIForm` and a child
   form carries `MDIChild = -1  'True`. Unverified.

10. **The float decimal separator on a non-English machine.** The corpus is
    English only. Whether the VB6 IDE ever wrote `Size = 8,25` is not
    established.

---

## Sources

Corpus, read directly:

- `tannerhelland/vb6-code`, BSD-2-Clause. 32 projects. Every `.vbp`, `.frm`,
  `.frx`, `.bas` and `.cls` in it, plus three `.exe` files.

Public files read to cover what the corpus lacks:

- [tannerhelland/VBIDEUtils, VBDoc/VBDoc.frm](https://github.com/tannerhelland/VBIDEUtils)
- [tannerhelland/PhotoDemon, Controls/pdLabel.ctl](https://github.com/tannerhelland/PhotoDemon)

Documented sources:

- [Form File Loading Errors, Microsoft Learn](https://learn.microsoft.com/en-us/previous-versions/visualstudio/visual-basic-6/aa242153(v=vs.60))
- [The 31 `Line 'item1':` load error pages, MicrosoftDocs/VBA-Docs](https://github.com/MicrosoftDocs/VBA-Docs/tree/live/Language/Reference/User-Interface-Help)
- [Line 'item1': The CLSID 'item2' for 'item3' is invalid](https://learn.microsoft.com/en-us/office/vba/Language/reference/user-interface-help/line-item1the-clsid-item2-for-item3-is-invalid)
- [scriptandcompile/vb6, the vb6parse crate](https://github.com/scriptandcompile/vb6/tree/master/projects/vb6parse),
  read at `src/files/project/mod.rs`, `src/files/project/compilesettings.rs`,
  `src/files/project/properties.rs`, `src/files/class/mod.rs`,
  `src/files/class/properties.rs`, `src/files/form/mod.rs`,
  `src/files/resource/mod.rs`, `src/io/source_file.rs`, `src/errors/`
- [vb6parse on docs.rs](https://docs.rs/vb6parse/latest/vb6parse/)
- [countingpine/gfxfromfrx, by Brad Martinez](https://github.com/countingpine/gfxfromfrx)
- [VB6 Task failing when "ResFile32" present in .vbp, nant/nantcontrib PR 27](https://github.com/nant/nantcontrib/pull/27)

Confidence of the web providers used, from the project's own
`classify-confidence` seam: `websearch` and `webfetch` both return **LOW**. Every
rule in this file that rests on a web source alone is marked **(b)** and is
never the only basis for an emitter rule. Rules marked **(a)** rest on bytes read
from disk and are the strongest evidence here.

---
*Researched 2026-09-07.*
