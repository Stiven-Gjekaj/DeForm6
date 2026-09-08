# Test corpus

Surveyed 2026-09-07. A sweep of 366 VB6 repositories on GitHub.

## The measurement

- 366 candidate repositories examined by fetching each recursive git tree.
- **120 (33%)** hold both a `.vbp` and an `.exe`.
- Of those 120, **52** carry a licence that permits redistribution:
  43 MIT, 7 Unlicense, 1 MIT-0, 1 BSD-2-Clause.
- After discarding repositories where the executable is an installer, a .NET
  build, or otherwise not the output of the VB6 project, roughly **35 to 40**
  survive as genuine pairs.

## What goes in the repository

### Tier 1, and it carries most of the value

**tannerhelland/vb6-code**, BSD-2-Clause, about 1.4 MB in source terms.
32 `.vbp` and 31 `.exe`. The README states that prebuilt executables are
provided for all projects, which is an author statement that the binaries are
the build output of the committed source.

Verified locally: 31 executables, all import `MSVBVM60`. All checked `.vbp` are
`Type=Exe` with `CompilationType=0`, meaning native code, and `ExeName32=`
matches the committed executable name.

Content is 31 independent programs of 20 to 90 KB: image filters, gradients,
artificial life, a Hidden Markov model, game physics, histograms, Mandelbrot,
a 2D map editor, TWAIN scanning, screen capture.

### Tier 2, public domain

Four `RizkyKhapidsyah/SK-*__VB6` repositories and six `Jigsy1/*` repositories,
all Unlicense, giving about **13 more programs**. All tiny, all Standard EXE,
all native.

### Tier 3, MIT, optional extras

`SweetIceLolly` (several, good OCX dependency variety), `Gagniuc` (about 10
small scientific programs with a clean `src/` and `bin/` split),
`DavoDC/Yr10_Programming` (about 18 trivial programs, excellent for simple
cases), `morphx666/dmb` (a large application), `liuzikai/iCode` (30 `.vbp`).

**`TimoKunze/ExplorerTreeView-VB6`**, MIT, is the only P-code source found:
7 sample applications with `CompilationType=-1`. Disproportionately valuable
when P-code work begins, because everything else in the corpus is native.

Vendoring tier 1 and tier 2 gives about **45 ground-truth programs in under
5 MB**. Adding tier 3 reaches about 90.

## What must not go in the repository

- **Microsoft's VB98 samples.** The VS6 licence grants no redistribution right,
  and they shipped as source with no binaries, so they carry no ground truth
  anyway. Excluded on both counts.
- **The 48 unlicensed pair repositories.** They are the biggest piles
  (`ChuckBolin/VB6` at 228 `.vbp`, `OtacomTec/vb6` at 140) and no licence means
  all rights reserved.
- **Archive.org freeware.** Tucows (about 40,000 titles), the Classic Windows
  Software Pack, Windows 98apps, the Planet Source Code Jumbo Resource CD.
  Archive.org's hosting is a preservation posture, not a grant to downstream
  mirrors, and "freeware" means gratis, not redistributable.

These are still useful as robustness inputs. The harness must fetch them at run
time from a manifest of pinned identifiers and SHA-256 hashes. Commit the
manifest, never the bytes.

## A negative result worth recording

The **Planet Source Code GitHub mirror** (16,662 repositories) looks like the
jackpot and is not. 70 repositories sampled across two widely separated pages
contained **zero** executables. The mirror is source only. Its terms also
disclaim ownership and defer to each original author, so it is unlicensed in
practice even for the source.

No VB6 source-and-binary pair under a permissive licence was found anywhere
outside GitHub.

## Identifying a VB6 executable from outside

- `MSVBVM60.DLL` in the PE import table. VB5 imports `MSVBVM50.DLL`.
- The entry point is a `push <ptr>` followed by a non-returning call to
  `ThunRTMain`. The pushed pointer is the VB header, whose first dword is the
  ASCII signature `VB5!`, used by VB5 and VB6 alike.
- **Native against P-code**: a field in the VB header, the native code
  descriptor pointer, is zero for P-code and non-zero for native. A
  corroborating heuristic is that native builds carry `0xE9E9E9E9` markers at
  real function starts, while P-code builds show `0xE9E9E9E9` followed by
  `0x9E9E9E9E` with no x86 after it.
- In a `.vbp`, `CompilationType=0` is native and `-1` is P-code. Useful for
  labelling our own corpus. Native is the VB6 default and dominates shipped
  software.

No public dataset is tagged "VB6 compiled" as its organising principle. The
nearest, MalSource, is malware under a research-only licence.

## What the corpus proves, measured rather than cited

Vendored on 2026-09-07: **44 executables and 45 project files** in 4.6 MB.
`corpus/vb6-code` holds 31 executables, `corpus/public-domain` holds 13.

A check ran over all 44 executables. For each one it read the PE header,
resolved `AddressOfEntryPoint` to a file offset through the section table, and
looked at the bytes there.

**All 44 match the documented pattern.** In every case the entry point begins
with opcode `0x68`, which is `push imm32`, and is followed by `0xE8`, which is
`call rel32`. In every case the pushed pointer, converted from a virtual
address to a file offset, lands on the four bytes `VB5!`.

This is the first link in the chain the parser depends on, and it is now
measured on this corpus rather than taken from a document. The entry point
addresses vary (`0x1424`, `0x116C`, `0x195C` and so on), so the offset is not
constant and must be resolved through the section table each time.

## What the corpus holds for phase 3, measured 2026-09-08

Counted by reading every `.frm` in `corpus/`, not by assumption.

### Control instances

| Control | Instances |
|---|---|
| `VB.Label` | 143 |
| `VB.CommandButton` | 90 |
| `VB.TextBox` | 88 |
| `VB.PictureBox` | 87 |
| **`VB.Menu`** | **76** |
| `VB.CheckBox` | 71 |
| `VB.Form` | 54 |
| `VB.Frame` | 31 |
| `VB.HScrollBar` | 25 |
| `VB.ComboBox` | 13 |
| `VB.OptionButton` | 12 |
| `VB.Timer` | 7 |
| `VB.Line` | 7 |
| `VB.ListBox` | 4 |
| **`MSWinsockLib.Winsock`** | **3** |
| `VB.VScrollBar`, `VB.FileListBox`, `VB.DriveListBox` | 1 each |

### What this means for phase 3

**Well covered.** The intrinsic control set is exercised heavily, so FRM-01,
FRM-02 and FRM-03 have real ground truth. Nesting is real too: 31 `Frame`
controls and 87 `PictureBox` controls are both container types.

**Menus are well covered**, which matters more than the raw count suggests.
76 menu entries across 22 form files. `FILE-FORMATS.md` records that the IDE
refuses a form whose menus are not written last, so WRT-04 in phase 4 has a
real corpus to prove itself against rather than one contrived case.

**Control arrays are covered.** 48 controls carry an `Index` property.
`STRUCTURES.md` gap 5 records that the control array index location is
unresolved, and the corpus can settle it.

**Third party controls are barely covered, and this is the real hole.**
The whole corpus declares **two** `Object=` lines:

    {248DD890-BB45-11CF-9ABC-0080C7E7B78D}#1.0#0; MSWINSCK.OCX
    {F9043C88-F6F2-101A-A3C9-08002B2F49FB}#1.2#0; COMDLG32.OCX

and holds **3** third party control instances, all `MSWinsockLib.Winsock`.
FRM-04 requires recovering an OCX CLSID and saying plainly that the control's
property blob cannot be interpreted without its type library. Three instances
of one control is thin. Phase 3 needs a synthetic fixture here, built the way
`has_clr_header` and the event descriptor walk were, and it must not present a
synthetic result as a corpus result.

**Three object kinds are absent entirely: 0 MDIForm, 0 UserControl, 0
PropertyPage.** `STRUCTURES.md` gap 3 records that the MDIForm `fObjectType`
value appears in no public source. The corpus cannot close it. An object of an
unknown kind is classified `Unknown` and flagged, which plan 02-02 built and
proved with a synthetic value, so an MDIForm will be reported rather than
refused.
