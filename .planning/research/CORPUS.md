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
