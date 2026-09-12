# On-disk structures of a VB6 Standard EXE

Researched 2026-09-07. This is the reference the DeForm6 parser is written from.

## 0. How to read this document

### Conventions

- All integers are **little-endian**. The target is 32-bit x86 only.
- `u8`, `u16`, `u32` are unsigned. `i16`, `i32` are signed.
- **VA** means a virtual address, that is `ImageBase + RVA`. Almost every pointer
  in these structures is a VA, not an RVA and not a file offset. To read one, the
  parser must map VA to file offset through the PE section table.
- **NTS** means a null-terminated byte string.
- "Offset" columns are byte offsets from the start of the named structure.
- A field named `null`, `reserved` or `ide` holds no information after
  compilation. Do not read it, but do keep its space in the layout.

### Confidence legend

| Tag | Meaning |
|---|---|
| **[C]** | Confirmed. Three or more independent sources agree, or one primary source plus a working implementation. |
| **[L]** | Likely. Two sources agree, or one source plus a strong structural argument. |
| **[D]** | Disputed. Sources disagree. The disagreement is stated and a recommendation is given. |
| **[G]** | Gap. Not documented anywhere found. Do not invent a layout. Verify against the corpus. |

### Sources

| Key | Source |
|---|---|
| **AI** | Alex Ionescu, *Visual Basic Image Internal Structure Format*, 2004. https://sandsprite.com/vb-reversing/files/Alex_Ionescu_vb_structures.pdf |
| **AG** | AndreaGeddon, *Visual Basic Reversed: a decompiling approach*. https://sandsprite.com/vb-reversing/files/VISUAL%20BASIC%20REVERSED.pdf |
| **GD** | Gen Digital Threat Research, *Recovery of function prototypes in Visual Basic 6 executables*, 2024-06. https://www.gendigital.com/blog/insights/research/recovery-of-function-prototypes-in-visual-basic-6-executables |
| **SVBD** | Semi VB Decompiler source, VBGAMER45. Read for facts only; **no LICENSE file, so no code may be copied**. https://github.com/VBGAMER45/Semi-VB-Decompiler |
| **PVB** | Willi Ballenthin, `python-vb`. https://github.com/williballenthin/python-vb (`vb/__init__.py`) |
| **PVBG** | Ballenthin reference gist. https://gist.github.com/williballenthin/dcbafede053a5a51d99c581acf846e1b |
| **SEK** | SekoiaLab `pe-tools`, `petools/vbparser.py`. https://github.com/SekoiaLab/pe-tools/blob/master/petools/vbparser.py |
| **IDC** | Reginald Wong and Bernard Sapaden, `vb.idc` for IDA. https://www.hex-rays.com/products/ida/support/freefiles/vb.idc |
| **VBD** | DotFix, *Guide to Analyzing Programs Written in Visual Basic 6.0*. https://www.vb-decompiler.org/vb60_reversing.htm (host was unreachable 2026-09-07; read via https://web.archive.org/web/20260220093316/https://www.vb-decompiler.org/vb60_reversing.htm) |

**Independence warning.** IDC and PVB both derive their field names and offsets
from AI. They are *not* independent confirmations of AI. SVBD and SEK are
independent implementations. Where AI disagrees with SVBD *and* SEK, AI is
outvoted by two independent readers, and IDC and PVB add nothing.

**VBD contains no structure layouts.** It is a debugging guide covering
`__vba*` helper semantics and breakpoint technique. It is cited here only so the
next reader does not spend time on it for structure work.

---

## 1. Reaching the VB header

### 1.1 The standard entry point stub

The PE entry point of a VB5/VB6 executable is a two-instruction stub. **[C]**
(AG p.2, PVB `find_vb_header`, SEK `getVbHeaderAddress`, SVBD
`GetVBStartHeader`.)

```
<entry>      68 xx xx xx xx    push    <VA of VBHeader>
<entry+5>    E8 yy yy yy yy    call    ThunRTMain
```

`ThunRTMain` is itself a one-instruction thunk into the import table:

```
ThunRTMain:  FF 25 zz zz zz zz  jmp     dword ptr [__imp_ThunRTMain]
```

So the VBHeader VA is the 32-bit immediate at `entry + 1`. **[C]**

### 1.2 Robust extraction

A parser should not disassemble. It should pattern-match and then validate:

1. Confirm the import directory names `MSVBVM60.DLL` (VB6) or `MSVBVM50.DLL`
   (VB5). PVB refuses the file outright if `MSVBVM60.DLL` is absent. SVBD reads
   the *first* import descriptor's DLL name and branches on
   `VB40032.DLL` / `MSVBVM50.DLL` / `MSVBVM60.DLL`. **[C]**
2. Read the byte at the entry point. If it is `0x68`, read the u32 at
   `entry + 1` as the header VA. **[C]**
3. Convert the VA to a file offset and confirm the four bytes there are
   `"VB5!"` = `56 42 35 21` = u32 `0x21354256`. **[C]** (AI §1, AG p.3.)
4. Optionally confirm the byte at `entry + 5` is `0xE8` and that the call target
   is a `jmp [mem]` whose target is an `MSVBVM60.DLL` import. PVB does this;
   SEK does this and additionally matches the thunk RVA against the import
   descriptor's first-thunk table. This is the strongest check but it needs a
   disassembler. Treat it as an optional confidence upgrade, not a gate.

**Recommendation for DeForm6.** Match on `0x68` at the entry point, take the
immediate, and validate the `VB5!` signature at the target. Report the
`0xE8` check as a separate evidence item in the report rather than as a hard
requirement.

### 1.3 Variations found

| Variation | Detail | Confidence |
|---|---|---|
| Push opcode `0x5A` instead of `0x68` | SVBD `GetVBStartHeader` accepts `0x68` **or** `0x5A` as the first opcode, and `0xE8` **or** `0x11` as the second. No sample is named and no explanation is given. `0x5A` is `pop edx`, which does not fit the stated 5-byte layout, so this may be dead defensive code rather than a real variant. | **[G]** Treat as unverified. Do not implement without a sample. |
| VB5 path | SVBD reads one byte at the entry point and, if it is `104` decimal (`0x68`), reads the following dword and uses it directly as `PushStartAddress` **without** subtracting the image base, on the branch it labels "VB5". This looks like an image-base bookkeeping difference in SVBD, not a file-format difference. | **[G]** |
| ActiveX DLL / OCX | There is no `push`/`call` stub. SVBD walks the PE export directory, finds `DllCanUnloadNow`, then reads a word at a fixed displacement inside the export thunk to recover the header address. This path is out of scope for DeForm6's first milestone (Standard EXE only) but is the reason a naive entry-point-only detector will refuse a VB6 DLL. | **[L]** |
| VB4 (`VB40032.DLL`) | The entry point is used directly as the header offset with a different structure entirely. Out of scope. | **[L]** |
| Packed or protected images | If the entry point has been rewritten by a packer, none of the above holds. The fallback is a raw scan of every executable section for the byte sequence `56 42 35 21` and validation of the candidate header by checking that `lpProjectData` and `lpGuiTable` resolve inside the image. SVBD exposes this as an "advanced decompile" mode where the user supplies the header offset and image base by hand. | **[L]** |

### 1.4 A second, weaker anchor

AG observes that `lpComRegisterData` in the header points to a block that in a
Standard EXE still carries the `stdole2.tlb` reference GUID
`{00020430-0000-0000-C000-000000000046}` and the project name. This is useful as
a sanity check but not as a locator. **[L]**

---

## 2. VBHeader (`EXEPROJECTINFO`)

Size `0x68` = 104 bytes. **[C]** (AI §1; identical in IDC, PVB, SEK, SVBD.)

Every `bSZ*` field at the end is a **byte offset relative to the start of this
structure**, not a VA and not an RVA. All four sources agree on that. **[C]**

| Offset | Size | Name | Type | Meaning | Conf |
|---|---|---|---|---|---|
| 0x00 | 4 | `szVbMagic` | char[4] | `"VB5!"`, u32 `0x21354256`. Present in VB5 **and** VB6. | **[C]** |
| 0x04 | 2 | `wRuntimeBuild` | u16 | Build number of the runtime the file was linked against. AG's sample reads `0x231C`. | **[C]** |
| 0x06 | 14 | `szLangDll` | char[14] | Language extension DLL name, NTS inside a fixed 14-byte field. AG's Italian sample holds `"VB6IT.DLL"`. SVBD notes a lone byte `0x2A` means "default / none". | **[C]** |
| 0x14 | 14 | `szSecLangDll` | char[14] | Backup language DLL. SVBD notes `0x7F` means "default"; AG's sample holds the single byte `0x2A`. Changing it does not affect execution. | **[C]** |
| 0x22 | 2 | `wRuntimeRevision` | u16 | Internal runtime revision. AG's sample: `0x000A`. | **[C]** |
| 0x24 | 4 | `dwLCID` | u32 | LCID of the language DLL. AG's sample: `0x410`, Italian. | **[C]** |
| 0x28 | 4 | `dwSecLCID` | u32 | LCID of the backup language DLL. AG's sample: `0x409`. | **[C]** |
| 0x2C | 4 | `lpSubMain` | VA | VA of `Sub Main`. **Zero means the startup object is a form**, not a module. IDC creates an IDA entry point here when non-zero. | **[C]** |
| 0x30 | 4 | `lpProjectData` | VA | VA of ProjectInfo (§3). The spine of the whole file. | **[C]** |
| 0x34 | 4 | `fMdlIntCtls` | u32 | Bitmask of intrinsic control classes used, IDs 0..31. Table in §2.1. | **[C]** |
| 0x38 | 4 | `fMdlIntCtls2` | u32 | Same for IDs 32..63. | **[C]** |
| 0x3C | 4 | `dwThreadFlags` | u32 | Threading mode. Table in §2.2. | **[C]** |
| 0x40 | 4 | `dwThreadCount` | u32 | Threads to pre-create in the pool. | **[C]** |
| 0x44 | 2 | `wFormCount` | u16 | **Number of entries in the GUI table.** This is the loop bound for §8. | **[C]** |
| 0x46 | 2 | `wExternalCount` | u16 | Number of entries in the external component (OCX) table at 0x50. | **[C]** |
| 0x48 | 4 | `dwThunkCount` | u32 | Number of runtime thunks to create. AG's sample: `0xE9`. | **[C]** |
| 0x4C | 4 | `lpGuiTable` | VA | VA of an array of `wFormCount` GUI table entries, `0x50` bytes each (§8.1). AG calls this `DialogsStruct`. | **[C]** |
| 0x50 | 4 | `lpExternalTable` | VA | VA of the **external component** (OCX/typelib reference) table, `wExternalCount` entries (§7.3). Note the name collision with `ProjectInfo.lpExternalTable`, which is a *different* table holding `Declare` imports. | **[C]** |
| 0x54 | 4 | `lpComRegisterData` | VA | VA of `tagREGDATA`. Present even in a Standard EXE. | **[C]** |
| 0x58 | 4 | `oProjectExeName` | u32 | Header-relative offset to the EXE name **without** its extension, NTS. Equals `.vbp` `ExeName32=` minus `.exe`. | **[C]** |
| 0x5C | 4 | `oProjectTitle` | u32 | Header-relative offset to the project title, NTS. Equals `.vbp` `Title=`, which VB6 omits when it equals `Name=`. | **[C]** |
| 0x60 | 4 | `oHelpFile` | u32 | Offset to the project help file name, NTS. All sources agree. | **[C]** |
| 0x64 | 4 | `oProjectName` | u32 | Offset to the project name, NTS. All sources agree. | **[C]** |

### 2.1 `fMdlIntCtls` / `fMdlIntCtls2` intrinsic control bits **[C]**

Identical in AI, PVB and SVBD.

| ID | Bit (first dword) | Object | | ID | Bit (first dword) | Object |
|---|---|---|---|---|---|---|
| 0x00 | 0x00000001 | PictureBox | | 0x0D | 0x00002000 | Form |
| 0x01 | 0x00000002 | Label | | 0x0E | 0x00004000 | Screen |
| 0x02 | 0x00000004 | TextBox | | 0x0F | 0x00008000 | Clipboard |
| 0x03 | 0x00000008 | Frame | | 0x10 | 0x00010000 | Drive |
| 0x04 | 0x00000010 | CommandButton | | 0x11 | 0x00020000 | Dir |
| 0x05 | 0x00000020 | CheckBox | | 0x12 | 0x00040000 | FileListBox |
| 0x06 | 0x00000040 | OptionButton | | 0x13 | 0x00080000 | Menu |
| 0x07 | 0x00000080 | ComboBox | | 0x14 | 0x00100000 | MDIForm |
| 0x08 | 0x00000100 | ListBox | | 0x15 | 0x00200000 | App |
| 0x09 | 0x00000200 | HScrollBar | | 0x16 | 0x00400000 | Shape |
| 0x0A | 0x00000400 | VScrollBar | | 0x17 | 0x00800000 | Line |
| 0x0B | 0x00000800 | Timer | | 0x18 | 0x01000000 | Image |
| 0x0C | 0x00001000 | Print | | 0x19..0x1F | | unsupported |

Second dword: ID 0x25 = `0x20` DataQuery, 0x26 = `0x40` OLE, 0x28 = `0x100`
UserControl, 0x29 = `0x200` PropertyPage, 0x2A = `0x400` Document. All other
bits unsupported.

The constant `0x30F000` seen in nearly every project means Print, Form, Screen,
Clipboard (`0xF000`) plus Drive and Dir (`0x30000`). AG's sample reads
`0x30F016`, that is the default set plus TextBox, CommandButton and Label.

### 2.2 `dwThreadFlags` **[C]**

| Value | Name | Meaning |
|---|---|---|
| 0x01 | ApartmentModel | Apartment-threaded |
| 0x02 | RequireLicense | Do license validation (OCX only) |
| 0x04 | Unattended | No GUI elements initialised |
| 0x08 | SingleThreaded | Single-threaded image |
| 0x10 | Retained | Keep in memory (Unattended only) |

AG's Standard EXE sample reads `0x08`.

### 2.3 The 0x58 / 0x5C disagreement, settled **[C]**

This question was open. The corpus closed it. §13 holds the method and the
worked example. This section holds the record of who said what, because that
record is why the question was open at all.

| Source | 0x58 | 0x5C |
|---|---|---|
| AI (2004) | `bSZProjectDescription` | `bSZProjectExeName` |
| IDC | copies AI verbatim | copies AI verbatim |
| PVB | copies AI verbatim | copies AI verbatim |
| SVBD | `oProjectExename` (max 0x104) | `oProjectTitle` (max 0x28) |
| SEK | `oProjectExename` | `oProjectTitle` |

0x60 and 0x64 are agreed by everyone, so the dispute was confined to the first
two slots.

**Attempt to resolve from AG's dump.** AG lists the four values as
`0x78, 0x7E, 0x84, 0x85` with a header base of `0x401970`. That gives four
strings at header-relative `0x78` (6 bytes, so 5 characters), `0x7E` (6 bytes,
5 characters), `0x84` (1 byte, empty) and `0x85`. The empty string at the
agreed help-file slot is consistent with both readings. The two 5-character
strings match neither `"Progetto1"` (9) nor a plausible EXE name, so the dump
does **not** settle it.

**The corpus settles it.** 0x58 is `oProjectExeName` and 0x5C is
`oProjectTitle`. The SVBD and SEK reading is correct. The AI, IDC and PVB
reading is wrong.

The evidence is all 44 corpus binaries, each diffed against the `.vbp` that
declares its `ExeName32`. **23** of the 44 hold a different string in the two
slots, so those 23 discriminate between the two candidate readings. All 23
agree with SVBD and SEK. The other 21 hold the same string in both slots,
because the title of those projects equals the executable name, so they are
consistent with either reading and they prove nothing. 21 plus 23 is 44. §13
carries that reconciliation and the reason an earlier draft said 24.

Do not implement the two names as aliases over the same two fields. The
question is answered, and an alias would keep a wrong name alive in the code.
Do not emit `ExeName32` from 0x5C when the value ends in `.exe` either: that
rule reads the wrong field, and no corpus value ends in `.exe`, because the
extension is not in the file. Phase 4 builds `ExeName32` from 0x58 plus the
literal `.exe`.

### 2.4 `tagREGDATA` (COM registration data)

Pointed to by `VBHeader.lpComRegisterData`. All offsets *inside* the sub-blocks
are relative to the start of `tagREGDATA`, not to the sub-block. **[C]**

| Offset | Size | Name | Meaning | Conf |
|---|---|---|---|---|
| 0x00 | 4 | `oRegInfo` | Offset to `tagRegInfo`, or 0 | **[C]** |
| 0x04 | 4 | `oNTSProjectName` | Offset to project / typelib name | **[C]** |
| 0x08 | 4 | `oNTSHelpDirectory` | Offset to help directory | **[C]** |
| 0x0C | 4 | `oNTSProjectDescription` | Offset to project description | **[C]** |
| 0x10 | 16 | `uuidProjectClsId` | CLSID of the project / typelib | **[C]** |
| 0x20 | 4 | `dwTlbLcid` | LCID of the type library | **[C]** |
| 0x24 | 2 | padding / unknown | AI marks "might be something" | **[G]** |
| 0x26 | 2 | `wTlbVerMajor` | Typelib major version | **[C]** |
| 0x28 | 2 | `wTlbVerMinor` | Typelib minor version | **[C]** |

**Size disputed.** AI and PVB say `0x2A`. SVBD says `0x30`, adding
`iPadding2` at 0x2A, and `lPadding3` at 0x2C. **[D]** The extra six bytes are
padding either way, so this only matters if a parser reads `tagRegInfo` by
adding a fixed size instead of by following `oRegInfo`. **Always follow
`oRegInfo`.**

`tagRegInfo`, size `0x44`, is only populated for ActiveX projects and is out of
scope for the first milestone. Layout, all sources agreeing: 0x00 `oNextObject`,
0x04 `oObjectName`, 0x08 `oObjectDescription`, 0x0C `dwInstancing`,
0x10 `dwObjectId`, 0x14 `uuidObject`[16], 0x24 `fIsInterface`,
0x28 `oUuidObjectIFace`, 0x2C `oUuidEventsIFace`, 0x30 `fHasEvents`,
0x34 `dwMiscStatus`, 0x38 `fClassType` (u8), 0x39 `fObjectType` (u8),
0x3A `wToolboxBitmap32`, 0x3C `wDefaultIcon`, 0x3E `fIsDesigner` (u16),
0x40 `oDesignerData`. **[C]**

`fObjectType` values: 0x02 Designer, 0x10 Class Module, 0x20 User Control,
0x80 User Document. **[C]**

**A Standard EXE still has a valid `tagREGDATA` with `oRegInfo == 0`.** Do not
treat a non-null `lpComRegisterData` as evidence of an ActiveX project.

---

## 3. ProjectInfo

Pointed to by `VBHeader.lpProjectData`. Size `0x23C` = 572 bytes. **[C]**
(AI §3, SEK `ProjectInfo._unpack` with format `<LLLLLLLLL528sLL`, SVBD
`tProjectInfo`, PVB `ProjectData`.)

| Offset | Size | Name | Meaning | Conf |
|---|---|---|---|---|
| 0x00 | 4 | `dwVersion` | Template version. `0x1F4` (500 decimal) in AG's VB6 sample; AI describes it as "5.00 in hex". SEK calls it `lTemplateVersion`. **It is not a VB5-vs-VB6 discriminator.** | **[C]** |
| 0x04 | 4 | `lpObjectTable` | VA of the ObjectTable (§4). | **[C]** |
| 0x08 | 4 | `dwNull` | Unused after compilation. | **[C]** |
| 0x0C | 4 | `lpCodeStart` | VA of the start of user code. AG: the region is bracketed by the marker `E9 E9 E9 E9` at the start and `9E 9E 9E 9E` at the end, with `0xCC` padding. | **[C]** |
| 0x10 | 4 | `lpCodeEnd` | VA of the end of user code. GD confirms all user code lies inside `[lpCodeStart, lpCodeEnd)`; the rest of `.text` is compiler stubs and structures. | **[C]** |
| 0x14 | 4 | `dwDataSize` | Size of the VB object structures. AG's sample: `0x1238`. | **[C]** |
| 0x18 | 4 | `lpThreadSpace` | Pointer to a pointer to the thread object. | **[C]** |
| 0x1C | 4 | `lpVbaSeh` | VA of `__vbaExceptHandler`. | **[C]** |
| 0x20 | 4 | `lpNativeCode` | **The native/P-code discriminator.** Non-zero means native, zero means P-code. See §10.2. | **[C]** |
| 0x24 | 528 | `szPathInformation` | Compile-time project path and identifier string. AI, PVB, SEK treat the whole 528 bytes as one blob. SVBD subdivides it: u16 `oProjectLocation` at 0x24, u16 flag at 0x26, u16 flag at 0x28, then the path bytes from 0x2A, then one null byte at 0x233. AG notes the content is often UTF-16. | **[D]** on the subdivision, **[C]** on the extent 0x24..0x233. |
| 0x234 | 4 | `lpExternalTable` | VA of the `Declare` import table (§7.1). | **[C]** |
| 0x238 | 4 | `dwExternalCount` | Number of entries in that table. | **[C]** |

`0x24 + 0x210 = 0x234`, so the three readings of the path region are
arithmetically identical. A parser can safely treat 0x24..0x233 as opaque and
scan it for a printable ASCII or UTF-16 path for the report.

### 3.1 ProjectInfo2

Pointed to by `ObjectTable.lpProjectInfo2` (**not** by ProjectInfo). Size
`0x28` = 40 bytes. **[C]** (AI §4, SVBD `tProjectInfo2`, PVB `ProjectData2`.)

| Offset | Size | Name | Meaning | Conf |
|---|---|---|---|---|
| 0x00 | 4 | `lpHeapLink` | Always 0 after compilation. | **[C]** |
| 0x04 | 4 | `lpObjectTable` | Back-pointer to the ObjectTable. | **[C]** |
| 0x08 | 4 | `dwReserved` | `-1` after compilation. | **[C]** |
| 0x0C | 4 | `dwUnused` | Never written or read. | **[C]** |
| 0x10 | 4 | `lpObjectList` | VA of an array of VAs, each pointing to a PrivateObjectDescriptor (§5.4). Length is `ObjectTable.wCompiledObjects`; entries may be `0` or `0xFFFFFFFF` and must be skipped. | **[L]** (AI; PVB implements it, crediting `openrce.org/repositories/users/Paolo/vbpython.py` for the count.) |
| 0x14 | 4 | `dwUnused2` | Never written or read. | **[C]** |
| 0x18 | 4 | `szProjectDescription` | **VA** (not an offset) of the project description NTS. | **[C]** |
| 0x1C | 4 | `szProjectHelpFile` | VA of the project help file NTS. | **[C]** |
| 0x20 | 4 | `dwReserved2` | `-1` after compilation. | **[C]** |
| 0x24 | 4 | `dwHelpContextId` | Help context ID from project settings. | **[C]** |

There is no "ProjectInfo3". AG's `TreeData` is this same structure; his
`FormList` at `TreeData + 0x10` is `lpObjectList`.

---

## 4. ObjectTable

Pointed to by `ProjectInfo.lpObjectTable`. Size `0x54` = 84 bytes. **[C]**
(AI §5; SEK format `<LLLLLLLLLLHHHHLLLLLLLLL` = 84 bytes; SVBD `tObjectTable`;
PVB `ObjectTable`.)

| Offset | Size | Name | Meaning | Conf |
|---|---|---|---|---|
| 0x00 | 4 | `lpHeapLink` | Always 0 after compilation. | **[C]** |
| 0x04 | 4 | `lpExecProj` | Pointer to the in-memory project exec COM object. Zero on disk. | **[C]** |
| 0x08 | 4 | `lpProjectInfo2` | VA of ProjectInfo2 (§3.1). | **[C]** |
| 0x0C | 4 | `dwReserved` | `-1` after compilation. | **[C]** |
| 0x10 | 4 | `dwNull` | Unused in compiled mode. | **[C]** |
| 0x14 | 4 | `lpProjectObject` | In-memory only. | **[C]** |
| 0x18 | 16 | `uuidObject` | GUID of the object table. AI, PVB, IDC read it as a GUID; SVBD reads the same 16 bytes as four opaque dwords. Same bytes either way. | **[C]** |
| 0x28 | 2 | `fCompileState` | Internal compile flag. | **[C]** |
| 0x2A | 2 | `wTotalObjects` | Total objects in the project. | **[C]** |
| 0x2C | 2 | `wCompiledObjects` | **Not the number of objects.** It is the capacity of the object array, rounded up. It equals `wTotalObjects` in only 29 of the 44 corpus files. Do not loop on it. See §4.1. | **[C]** |
| 0x2E | 2 | `wObjectsInUse` | Usually equal to the above after compile. | **[C]** |
| 0x30 | 4 | `lpObjectArray` | VA of an array of `Object` records, `0x30` bytes each (§5.1). | **[C]** |
| 0x34 | 4 | `fIdeFlag` | IDE only. | **[C]** |
| 0x38 | 4 | `lpIdeData` | IDE only. | **[C]** |
| 0x3C | 4 | `lpIdeData2` | IDE only. | **[C]** |
| 0x40 | 4 | `lpszProjectName` | **VA** of the project name NTS. | **[C]** |
| 0x44 | 4 | `dwLcid` | Project LCID. | **[C]** |
| 0x48 | 4 | `dwLcid2` | Alternate project LCID. | **[C]** |
| 0x4C | 4 | `lpIdeData3` | IDE only. | **[C]** |
| 0x50 | 4 | `dwIdentifier` | Template version of the structure. | **[C]** |

**Trap.** AI's table prints `dwTotalObjects`, `dwCompiledObjects` and
`dwObjectsInUse` with the `dw` prefix at 0x2A, 0x2C, 0x2E. They are two bytes
apart, so they are **words**, and AI's naming is a typo. SEK, SVBD, PVB and IDC
all read them as u16. **[C]**

**Which count to loop on.** SVBD loops on `wTotalObjects` (its
`ObjectCount1` at 0x2A); PVB and GD loop on `wCompiledObjects` (0x2C).

An earlier draft of this section said the two are equal after a clean compile
and recommended `wCompiledObjects` as the count. **The corpus refutes that, and
the recommendation is withdrawn.** See §4.1.

### 4.1 The two counts are not the same quantity, 2026-09-07

A script read both fields from all **44** corpus executables and compared each
against the number of objects the matching `.vbp` declares. The `.vbp` was
selected by its `ExeName32` key, as §12 requires, and an object is a `Form`,
`Module`, `Class`, `UserControl`, `PropertyPage`, `UserDocument`, `Designer`
or `RelatedDoc` key.

| Field | Equals the number of objects the `.vbp` declares |
|---|---|
| `wTotalObjects` at 0x2A | **44 of 44** |
| `wCompiledObjects` at 0x2C | 29 of 44 |

In the other 15 files `wCompiledObjects` is larger, and it is larger by the
amount that rounds the array up: a project that declares 1, 2 or 3 objects
reports 4, and a project that declares 5 reports 8. `wObjectsInUse` at 0x2E
equals `wTotalObjects` in all 44.

Walking the array past `wTotalObjects` in those files reaches a name pointer
that is `0`, or one that resolves to nothing, or one that resolves to an
unrelated string. `corpus/vb6-code/Grayscale-effect/Grayscale.exe` declares
one form and two classes, its array holds `frmGrayscale`, `pdOpenSaveDialog`
and `FastDrawing` and then a null pointer, and its `wCompiledObjects` is 4.

**So `wCompiledObjects` is the capacity of the object array and
`wTotalObjects` is the number of objects.** **[C]**

**Read the count from `wTotalObjects`.** Read `wCompiledObjects` as the
capacity, and use it as the bound the count must not exceed. A capacity above
the count is normal and must not be reported as damage: it is what a third of
this corpus holds. A capacity **below** the count is a real disagreement,
because the array then has no room for the objects the same structure
declares, and that is the damage indicator to report.

§12 records that `wCompiledObjects` "bounds a walkable object array, 44 of
44". That measurement stands. It bounds the array, because it is the size of
the array. It is not the number of objects in it.

---

## 5. Objects: descriptors and info

### 5.1 `Object` (AI: "Public Object Descriptor")

An array of these lives at `ObjectTable.lpObjectArray`. Size `0x30` = 48 bytes.
**[C]** (AI §7, SEK `TObject`, SVBD `tObject`, PVB `PublicObjectDescriptor`,
GD `objArray` figure. All five agree field for field.)

| Offset | Size | Name | Meaning | Conf |
|---|---|---|---|---|
| 0x00 | 4 | `lpObjectInfo` | VA of the ObjectInfo (§5.2). | **[C]** |
| 0x04 | 4 | `dwReserved` | `-1` after compilation. | **[C]** |
| 0x08 | 4 | `lpPublicBytes` | VA of an array of public-variable sizes. | **[C]** |
| 0x0C | 4 | `lpStaticBytes` | VA of an array of static-variable sizes. | **[C]** |
| 0x10 | 4 | `lpModulePublic` | VA of the public variables in `.data`. | **[C]** |
| 0x14 | 4 | `lpModuleStatic` | VA of the static variables in `.data`. | **[C]** |
| 0x18 | 4 | `lpszObjectName` | **VA of the object name, NTS, ASCII.** This is the form / module / class name. | **[C]** |
| 0x1C | 4 | `ProcCount` | Number of entries in `lpProcNamesArray` **and** in the parallel `FuncTypDesc` array (§6.3). Counts events, subs and functions together. | **[C]** |
| 0x20 | 4 | `lpProcNamesArray` | VA of an array of `ProcCount` VAs, each pointing to a procedure name NTS. **A null entry means the procedure at that index is private.** May itself be 0. | **[C]** |
| 0x24 | 4 | `oStaticVars` | Offset into `lpModuleStatic` where the static variables begin. GD's class sample reads `0xFFFF`. | **[C]** |
| 0x28 | 4 | `fObjectType` | **The form / module / class discriminator.** See §5.5. | **[C]** |
| 0x2C | 4 | `dwNull` | Not valid after compilation. | **[C]** |

### 5.2 `ObjectInfo`

Pointed to by `Object.lpObjectInfo`. Size `0x38` = 56 bytes. **[C]**
(AI §8, SEK `TObjectInfo` format `<HHLLLLLLLHHLHHLLL` = 56 bytes, SVBD
`tObjectInfo`, PVB `ObjectInfo`.)

| Offset | Size | Name | Meaning | Conf |
|---|---|---|---|---|
| 0x00 | 2 | `wRefCount` | Always 1 after compilation. | **[C]** |
| 0x02 | 2 | `wObjectIndex` | Index of this object in the object array. | **[C]** |
| 0x04 | 4 | `lpObjectTable` | Back-pointer to the ObjectTable. | **[C]** |
| 0x08 | 4 | `lpIdeData` | Zero after compilation. | **[C]** |
| 0x0C | 4 | `lpPrivateObject` | VA of the PrivateObject (§6.1). **SVBD notes this is `-1` for a standard module.** | **[C]** |
| 0x10 | 4 | `dwReserved` | `-1` after compilation. | **[C]** |
| 0x14 | 4 | `dwNull` | Unused. | **[C]** |
| 0x18 | 4 | `lpObject` | Back-pointer to the `Object` record (§5.1). | **[C]** |
| 0x1C | 4 | `lpProjectData` | Pointer to the in-memory project object. SVBD's native decompiler notes that a form's pre-declared global instance lives at `dword(ObjectInfo + 0x1C) + 8` in BSS. | **[L]** |
| 0x20 | 2 | `wMethodCount` | Number of entries in `lpMethods`. | **[C]** |
| 0x22 | 2 | `wMethodCount2` | Zeroed after compilation, IDE only. | **[C]** |
| 0x24 | 4 | `lpMethods` | VA of an array of `wMethodCount` VAs to method descriptors. In a **P-code** build each entry points to a `ProcDscInfo` (§10.3). In a **native** build the entries point into code. | **[C]** |
| 0x28 | 2 | `wConstants` | Number of constants in the constant pool. | **[C]** |
| 0x2A | 2 | `wMaxConstants` | Constants to allocate in the pool. | **[C]** |
| 0x2C | 4 | `lpIdeData2` | IDE only. | **[C]** |
| 0x30 | 4 | `lpIdeData3` | IDE only. | **[C]** |
| 0x34 | 4 | `lpConstants` | VA of the constant pool. | **[C]** |

### 5.3 `OptionalObjectInfo`

**It is not a separate allocation.** It sits immediately after `ObjectInfo`, at
`lpObjectInfo + 0x38`. Size `0x40` = 64 bytes. **[C]** (AI §9 describes it as
present "only for COM objects, anything but a module"; SVBD comments
"the rest is optional items"; PVB computes `va = lpObjectInfo + len(ObjectInfo)`.)

**Presence test:** see §5.5. Modules do not have it.

AI's field names and PVB/SVBD's differ in the middle of the structure. The
PVB/SVBD reading is more detailed and mutually consistent, so it is preferred.

| Offset | Size | PVB / SVBD name | AI name | Meaning | Conf |
|---|---|---|---|---|---|
| 0x00 | 4 | `dwObjectGuiGuids` / `fDesigner` | `dwObjectGuids` | Count of object-GUI GUIDs. AI notes `2` means designer. | **[C]** |
| 0x04 | 4 | `lpObjectCLSID` | `lpObjectGuid` | VA of this object's CLSID. | **[C]** |
| 0x08 | 4 | `dwNull` | `dwNull` | Unused. | **[C]** |
| 0x0C | 4 | `lpGuidObjectGUITable` | `lpuuidObjectTypes` | VA of an array of pointers to object-GUI GUIDs. | **[L]** |
| 0x10 | 4 | `dwObjectDefaultIIDCount` | `dwObjectTypeGuids` | Count of default IIDs. | **[L]** |
| 0x14 | 4 | `lpObjectEventsIIDTable` | `lpControls2` | VA of an array of pointers to the events IIDs. AI's "usually the same as lpControls" is wrong. | **[D]** prefer PVB/SVBD |
| 0x18 | 4 | `dwObjectEventsIIDCount` | `dwNull2` | Count of events IIDs. | **[D]** prefer PVB/SVBD |
| 0x1C | 4 | `lpObjectDefaultIIDTable` | `lpObjectGuid2` | VA of an array of pointers to default IIDs. | **[D]** prefer PVB/SVBD |
| 0x20 | 4 | `dwControlCount` | `dwControlCount` | **Number of `ControlInfo` records.** | **[C]** |
| 0x24 | 4 | `lpControls` | `lpControls` | **VA of the `ControlInfo` array (§8.6), `0x28` bytes per entry.** | **[C]** |
| 0x28 | 2 | `wMethodLinkCount` | `wEventCount` | Number of method-link entries. | **[L]** |
| 0x2A | 2 | `wPCodeCount` | `wPCodeCount` | P-codes used by this object. | **[C]** |
| 0x2C | 2 | `bWInitializeEvent` | same | Offset of the `Initialize` event within the method-link table. | **[C]** |
| 0x2E | 2 | `bWTerminateEvent` | same | Offset of the `Terminate` event within the method-link table. | **[C]** |
| 0x30 | 4 | `lpMethodLinkTable` | `lpEvents` | VA of an array of pointers to method links. | **[C]** |
| 0x34 | 4 | `lpBasicClassObject` | same | Pointer to the in-memory class object. | **[C]** |
| 0x38 | 4 | `dwNull3` | same | Unused. | **[C]** |
| 0x3C | 4 | `lpIdeData` | same | IDE only. | **[C]** |

SVBD's copy has a transcription error: it lists both `iPCodeCount` and
`oInitializeEvent` at `0x2C`. The correct sequence is 0x28, 0x2A, 0x2C, 0x2E as
above. **[C]**

### 5.4 `PrivateObjectDescriptor`

Reached through `ProjectInfo2.lpObjectList`, an array of VAs. Size `0x40`.
**AI describes this as compile-time scaffolding that "can be deleted after
compilation".** GD reads the same 64 bytes and finds real, load-bearing data in
the middle. **GD is right and AI is wrong here.** See §6.1, which supersedes
this table.

AI's reading, for reference only: 0x00 `lpHeapLink`, 0x04 `lpObjectInfo`,
0x08 `dwReserved`, 0x0C `dwIdeData[3]`, 0x18 `lpObjectList`, 0x1C `dwIdeData2`,
0x20 `lpObjectList2[3]`, 0x2C `dwIdeData3[3]`, 0x38 `dwObjectType`,
0x3C `dwIdentifier`. **[D]** Do not use this. Use §6.1.

Note also that `ObjectInfo.lpPrivateObject` (0x0C) reaches the same structure
directly, which is the path GD takes and the one DeForm6 should take. There is
no need to walk `ProjectInfo2.lpObjectList` at all.

### 5.5 Telling forms, modules, classes and controls apart

`Object.fObjectType` at 0x28 is a bitfield. SVBD carries an explicit lookup
table of observed values, which is the only enumeration in any source. **[C]**

| Value | Kind |
|---|---|
| `0x18083`, `0x180A3`, `0x180C3` | **Form** |
| `0x18001`, `0x18021`, `0x18041`, `0x18061` | **Standard module** (`.bas`) |
| `0x18023`, `0x18803`, `0x118003`, `0x118803`, `0x138003` | **Class** (`.cls`) |
| `0x1DA003`, `0x1DA023`, `0x1DA803` | **UserControl** |
| `0x158003` | **PropertyPage** |
| `0x158803` | **UserDocument** |

Bit analysis of that table:

| Bit | Mask | Set in | Reading | Conf |
|---|---|---|---|---|
| 0 | 0x000001 | everything | always set | **[C]** |
| 1 | 0x000002 | everything except a standard module | **the object is a COM object, so `OptionalObjectInfo` is present** | **[L]** |
| 7 | 0x000080 | forms only | form | **[L]** |
| 11 | 0x000800 | some classes and usercontrols | unknown | **[G]** |
| 15,16 | 0x018000 | everything | always set | **[C]** |
| 18 | 0x040000 | usercontrol, propertypage, userdocument | ActiveX-hosted object | **[L]** |
| 19 | 0x080000 | usercontrol only | usercontrol | **[L]** |
| 20 | 0x100000 | some classes, usercontrol, propertypage, userdocument | has a public interface | **[L]** |

**The `OptionalObjectInfo` presence test is disputed.** **[D]**

| Source | Test |
|---|---|
| SVBD comment on `tOptionalObjectInfo` | `fObjectType AND 0x80` |
| PVB `OBJECT_HAS_OPTIONAL_INFO` | `fObjectType AND 0x01` |
| SVBD's own bit ladder comment | bit that is 1 for everything but a module |

Both stated tests are wrong against SVBD's own value table. `0x80` is set only
for forms, yet classes clearly carry controls and event IIDs. `0x01` is set for
modules too. **Use `fObjectType & 0x2`.** It is the only bit that separates
modules from everything else across all seventeen observed values. Cross-check
it against `ObjectInfo.lpPrivateObject != -1` (SVBD's module marker) and report
a disagreement. **[L]**

**MDIForm is missing from SVBD's table.** **[G]** An MDI parent form's
`fObjectType` value is not recorded in any source found. A parser must not
refuse an object whose type value is unknown; classify it as `Unknown`, fall
back to the control-tree type byte (§8.4, `cType == 20` for MDIForm), and flag
it in the report.

---

## 6. Procedure and type descriptors

This is GD's contribution and it is the newest work in the field. GD states the
key property directly: **this type data is emitted into `.text` as part of the
standard `IDispatch` plumbing for every user form, class and user control, it
is required at run time, and the compiler cannot strip it.** It does **not**
exist for procedures in `.bas` code modules, because those are not COM objects.

All of §6 is reconstructed from the figures in the GD article
(`privateObj.png`, `aFuncTyp.png`, `pubfunc_v2.png`, `func2Type.png`,
`publicVar.png`, `eventDesc.png`, `objArray.png`, `procNamesArray.png`,
`image_10.png`). GD says plainly "I could not find public documentation on this
structure or those below it".

### 6.1 `PrivateObj`

Reached from `ObjectInfo.lpPrivateObject` (§5.2, offset 0x0C). Occupies the
same 64 bytes AI calls the PrivateObjectDescriptor. **[L]**

| Offset | Size | Name | Meaning |
|---|---|---|---|
| 0x00 | 4 | `nul1` | 0 |
| 0x04 | 4 | `lpParentLink` | VA of the owning `ObjInfo` |
| 0x08 | 4 | `unk1` | `0xFFFFFFFF` |
| 0x0C | 4 | `nul2` | 0 |
| 0x10 | 2 | `cntPublicVars` | **Number of entries in the `PubVarDesc` array** |
| 0x12 | 2 | `cntEvents` | **Number of entries in the `EventDesc` array** |
| 0x14 | 4 | `nul3` | 0 |
| 0x18 | 4 | `lpFuncTypeInfo` | **VA of the `FuncTypDesc` pointer array** |
| 0x1C | 4 | `nul4` | 0 |
| 0x20 | 4 | `lpPublicVars` | **VA of the `PubVarDesc` array** |
| 0x24 | 4 | `lpEventsTypeInfo` | **VA of the `EventDesc` pointer array** |
| 0x28 | 4 | `lpNull` | 0 |
| 0x2C | 4 | `nul5` | 0 |
| 0x30 | 4 | `nul6` | 0 |
| 0x34 | 4 | `nul7` | 0 |
| 0x38 | 4 | `unk3` | `0x4C` in GD's sample |
| 0x3C | 4 | `unk4` | `0x104` in GD's sample |

**The count for `lpFuncTypeInfo` is not in this structure.** GD: "To get the
count for the public functions, we have to reference the top-level
`Object->ProcCount` field." That is §5.1 offset 0x1C. **[C]**

### 6.2 How to walk the whole thing

```
Object                                  (ObjectTable.lpObjectArray[i], 0x30 bytes)
 ├─ lpszObjectName   0x18 → "MyClass"           (ASCII NTS)
 ├─ ProcCount        0x1C → N
 ├─ lpProcNamesArray 0x20 → VA[N]               (VA per proc, NULL = private)
 └─ lpObjectInfo     0x00 → ObjectInfo          (0x38 bytes)
      ├─ lpPrivateObject 0x0C → PrivateObj      (0x40 bytes)
      │    ├─ lpFuncTypeInfo   0x18 → VA[N]     (parallel to lpProcNamesArray)
      │    ├─ cntPublicVars    0x10 → V
      │    ├─ lpPublicVars     0x20 → PubVarDesc[V]   (contiguous records)
      │    ├─ cntEvents        0x12 → E
      │    └─ lpEventsTypeInfo 0x24 → VA[E]     (pointers to EventDesc)
      └─ (ObjectInfo + 0x38) → OptionalObjectInfo, if fObjectType & 2
```

**The `FuncTypDesc` pointer array and the `ProcNames` array are index-parallel
and both have length `ProcCount`.** A `NULL` at index *i* in either means the
procedure at that index is private and has no recoverable name or prototype.
GD's figures show the same `NULL` at the same index in both arrays. **[C]**

Note the asymmetry, which is real and which GD's figures make explicit:
`lpFuncTypeInfo` and `lpEventsTypeInfo` are arrays of **pointers**;
`lpPublicVars` is an array of **inline records**.

### 6.3 `FuncTypDesc`

GD's `pubfunc_v2` figure, for the prototype it names `mySub`. Header is
`0x20` bytes, followed by a variable-length type buffer. **[L]**

| Offset | Size | Name | Meaning |
|---|---|---|---|
| 0x00 | 1 | `argSize` | See §6.5. Type-entry count is `argSize >> 2`. The low three bits are the property-kind field. |
| 0x01 | 1 | `bFlags` | Bit 0 set ⇒ the member is a **Function** (the last type entry is the return value) rather than a Sub. GD calls this field `isFunc` in one figure and `bFlags` in the other. |
| 0x02 | 2 | `vOff` | **vtable offset of the member.** Clear the low bit; the adjusted value is always 4-byte aligned. `0xFFFF` for an event. |
| 0x04 | 2 | `constFFFF` | Always `0xFFFF` in every observed sample. |
| 0x06 | 2 | `nul1` | 0 |
| 0x08 | 4 | `optionalVals` | Non-zero when the method has optional parameters **with default values**. GD does not describe what it points to. **[G]** |
| 0x0C | 4 | `memberID` | DISPID. `0x60030002` in GD's class samples. |
| 0x10 | 4 | `lpAryArgNames` | **VA of an array of VAs to argument-name strings. The array is NOT null-terminated.** Compute the argument count first, then walk exactly that many entries. |
| 0x14 | 4 | `lpFuncDesc` | Pointer, use unknown. **[G]** |
| 0x18 | 4 | `nul3` | 0 |
| 0x1C | 4 | `helpID` | Help context ID, 0 in samples. |
| 0x20 | .. | type buffer | See §6.5. |

**Offset dispute inside GD's own article.** **[D]** The `func2Type` figure
annotates 0x06 and 0x08 as two separate words (`nul1`, `nul2`) with no
`optionalVals`, which would place `memberID` at the unaligned offset `0x0A` and
shift the whole tail down by two bytes. The `pubfunc_v2` figure, which the
filename marks as a revision, gives the aligned layout tabulated above. **Use
the `pubfunc_v2` layout** and confirm it on the first real binary by checking
that `constFFFF` reads `0xFFFF` and `memberID` has the form `0x6003xxxx`.

### 6.4 `EventDesc` and `PubVarDesc`

**`EventDesc`** has the same layout as `FuncTypDesc`. GD: "This structure
closely follows the layout of the `PubFuncDesc` type." In GD's `eventDesc`
figure `vOff` and `constFFFF` together read `0xFFFFFFFF` (an event has no
vtable slot), `memberID` is a small ordinal (`1`), and the byte at 0x20 is `0`
rather than `0x1E`. **[L]**

**Event names are a gap.** **[G]** GD: "I have not currently been able to locate
a link to the Event name strings embedded within the compiled binaries. In
practice, they are embedded after the strings of the `ProcNamesArray`." So the
strings are present but there is no pointer to them. Recovering an event's name
requires scanning the NTS run that follows the last `ProcNamesArray` string and
matching by position, which is a heuristic. DeForm6 should recover event
*signatures* (argument names and types are reachable) and mark event *names* as
inferred, or omit them.

**`PubVarDesc`**, from GD's `publicVar` figure. Inline records, `cntPublicVars`
of them starting at `PrivateObj.lpPublicVars`. **[L]**

| Offset | Size | Name | Meaning |
|---|---|---|---|
| 0x00 | 4 | `lpName` | VA of the variable name NTS |
| 0x04 | 4 | `nul1` | 0 |
| 0x08 | 4 | `nul2` | 0 |
| 0x0C | 2 | `index` | 0 in sample |
| 0x0E | 1 | `unk1` | 3 in sample |
| 0x0F | 1 | `unk2` | 0x40 in sample |
| 0x10 | 2 | `unk3` | 2 in sample |
| 0x12 | 2 | `unk4` | 0x1C in sample |
| 0x14 | 4 | `varOffset` | **Byte offset of the field inside the instance.** GD: `ObjPtr(obj) + varOffset` is where the data lives; `ObjPtr` + 0 is the vtable pointer. |
| 0x18 | 4 | type code | The §6.5 type byte, zero-extended / padded to 4 bytes |
| 0x1C | 4 | extra | Present only for `comobj` (0x1D) and `internal` (0x13). See §6.6. |

**Record stride is not stated by GD.** **[G]** The sample shows 0x20 bytes for a
`comobj` variable. A variable of a plain scalar type presumably occupies 0x1C.
Do not assume a fixed stride. Verify against a real binary before implementing;
if the stride turns out to be variable, the array must be walked record by
record with the type code deciding whether the trailing pointer is present.

**Only public variables survive.** SVBD's native decompiler notes that a class's
private backing variables are null-stripped from the type info and can only be
inferred from code. Do not promise private field names.

### 6.5 The VB type code enumeration

GD transcribed this by compiling variations and diffing the output. It is
**not** the OLE `VARENUM` enumeration and the values do not line up with it.
GD states this explicitly. **[L]** (source: GD `image_10.png`, an
`Enum eVBInternal_VarTypes` listing.)

| Value | Name | VB type |
|---|---|---|
| 0x03 | `epvT_bool` | `Boolean` |
| 0x05 | `epvT_byte` | `Byte` |
| 0x06 | `epvT_int` | `Integer` |
| 0x08 | `epvT_long` | `Long` |
| 0x0A | `epvT_single` | `Single` |
| 0x0B | `epvT_double` | `Double` |
| 0x0C | `epvT_date` | `Date` |
| 0x0D | `epvT_currency` | `Currency` |
| 0x0F | `epvT_variant` | `Variant` |
| 0x10 | `epvT_String` | `String` |
| 0x13 | `epvT_internal` | a class defined in this project, **adds a 32-bit pointer** |
| 0x1B | `epvT_object` | `Object` |
| 0x1C | `epvT_comIFace` | external COM interface, **adds a 32-bit pointer** |
| 0x1D | `epvT_comobj` | external COM object, **adds a 32-bit pointer** |
| 0x1E | `epvT_hresult` | `HRESULT` |

**Values 0x00, 0x01, 0x02, 0x04, 0x07, 0x09, 0x0E, 0x11, 0x12, 0x14..0x1A are
not documented.** **[G]** Notably `Decimal` and `LongLong` have no entry; VB6
has no `LongLong`, and `Decimal` exists only inside a `Variant`, so the gaps may
simply be unused. Do not guess. Emit an `UnknownType(0xNN)` marker and record it
in the report.

Modifier bits, OR-ed onto the base type: **[C]** (GD gives both the prose and
the `initFromRawVal` decoder, which strips them in the order 0x80, 0x40, 0x20.)

| Bit | Meaning |
|---|---|
| 0x20 | **ByRef** |
| 0x40 | **Array** |
| 0x80 | **Optional** |

GD's worked examples:

```
0x08 = 0000 1000   Long
0x28 = 0010 1000   ByRef Long
0x68 = 0110 1000   ByRef Long array   ( ... As Long() )
0xA8 = 1010 1000   Optional ByRef Long
0x2F = 0010 1111   ByRef Variant
0x25 = 0010 0101   ByRef Byte
0x33 = 0011 0011   ByRef <internal class>
```

**`ParamArray` has no observed encoding.** **[G]** A VB6 `ParamArray` argument is
always `Optional`-less, always the last parameter, always
`ByRef ... () As Variant`, so the natural guess is that it appears as
`0x20 | 0x40 | 0x0F = 0x6F` and is indistinguishable from a plain
`ByRef Variant()`. **Do not assert this.** Compile a `ParamArray` sample and
diff before emitting `ParamArray` in recovered source. Emitting
`ByRef v() As Variant` instead is wrong but compiles; emitting `ParamArray`
wrongly does not.

### 6.6 Walking the type buffer

The type buffer begins at `FuncTypDesc + 0x20`. **[L]**

1. The **first byte** is `0x1E` (`epvT_hresult`) for a Sub or Function, and
   `0x00` for an event. GD labels it `unk` / `unk1` and does not identify it.
   The `0x1E` value being exactly `epvT_hresult` and the count arithmetic below
   both working out is strong circumstantial evidence that it is the vtable
   slot's own `HRESULT` return, that is, a leading entry outside the user
   argument list. **[L]** Treat it as a leading byte to skip, and validate that
   it is `0x1E` or `0x00`.
2. Then follow **`argSize >> 2`** type entries.
3. Each entry is one **type byte**. If the base type (after masking off
   0x20/0x40/0x80) is `0x13`, `0x1C` or `0x1D`, the entry is followed by a
   **32-bit value**, aligned up to a 4-byte boundary.
4. If `bFlags` bit 0 is set, the **last** entry is the return type and the rest
   are the arguments. If it is clear, every entry is an argument.
5. `lpAryArgNames` then holds exactly as many name pointers as there are
   argument entries.

**Padding is real and must be tolerated.** GD: "these offsets appear to always
be 32-bit aligned. This can introduce null padding between the type byte and the
data offset. Null padding has also been observed between individual type bytes
as well. Parsers will have to be tolerant of these variations." A parser must
skip zero bytes when looking for the next type entry, and must bound that skip
so a corrupt buffer cannot make it run away.

**The property-kind field.** The low three bits of `argSize` are only set for
properties. GD: **`111` = Property Set, `010` = Property Let, `001` = Property
Get** (and for a Get, `bFlags` bit 0 is additionally set). Note the values are
given as three bits, so mask with `0x07`. **[L]** GD also notes
`argSize == 0` is legal for a Sub with no arguments and no return value, and
that VB6 allows at most **59** user arguments, which is a useful sanity bound.

**Worked check, from GD's own figures.** `mySub`: `argSize = 8`, `bFlags = 0`,
type buffer `1E 08 2F 00`, arg names `Arg1`, `V`. Leading `0x1E`, then
`8 >> 2 = 2` entries: `0x08` = `Long`, `0x2F` = `ByRef Variant`. Two entries,
two names, `bFlags` clear so both are arguments. That reconstructs
`Public Sub mySub(Arg1 As Long, V As Variant)`. The arithmetic closes.

### 6.7 The external-COM-object side structure

When a type entry is `epvT_comobj` (0x1D), the trailing 32-bit value points to a
structure describing the foreign object. From GD's `publicVar` figure: **[L]**

```
structCOMObj:
  0x00  VA → a pointer-to-library-GUID block
  0x04  VA → the object's CLSID (16 bytes)

that block:
  0x00  VA → the library GUID (16 bytes)
  0x04  0
  0x08  1
  0x0C  0
  0x10  VA → the DLL path NTS   e.g. "C:\Windows\SysWOW64\scrrun.dll"
  0x14  VA → the library name NTS  e.g. "Scripting"
```

Field names and the meaning of the `0/1/0` triple are not given. **[G]** The two
strings and the two GUIDs are what a `.vbp` `Reference=` line needs, so this is
worth implementing even with the unknown middle.

When a type entry is `epvT_internal` (0x13), the trailing 32-bit value is a VA
of the target class's **`ObjInfo`** structure, which resolves to a name through
`ObjectInfo.lpObject → Object.lpszObjectName`. **[C]** (GD `func2Type` figure,
annotated "pointer to internal class ObjInfo".)

---

## 7. External tables

There are **two** unrelated tables both called "external". Do not confuse them.

### 7.1 The `Declare` import table

Located by `ProjectInfo.lpExternalTable` (0x234) with
`ProjectInfo.dwExternalCount` (0x238) entries of 8 bytes each. **[C]**
(AI implied, AG p.4, PVB `ExternalTableEntry`, SVBD.)

| Offset | Size | Name | Meaning |
|---|---|---|---|
| 0x00 | 4 | `dwEntryType` | `6` = internal (resolved inside the runtime), `7` = external (a real DLL import). |
| 0x04 | 4 | `lpImportDescriptor` | VA of the descriptor below. |

**Entry type semantics.** AG states plainly: "The flag indicates the type of
import (6 = inside, 7 = outside)". SVBD processes an entry when
`Flag <> 6`. PVB processes an entry when `dwEntryType == 0x7` and logs a warning
otherwise. **[C]**

For `dwEntryType == 7`, `lpImportDescriptor` points at: **[C]**

| Offset | Size | Name | Meaning |
|---|---|---|---|
| 0x00 | 4 | `lpDllName` | VA of the library name NTS, e.g. `"shell32"` |
| 0x04 | 4 | `lpApiName` | VA of the export name NTS, e.g. `"ShellExecuteA"` |

AG's dump additionally shows an `align 8` gap and a third dword pointing at
thunking data (module handle plus resolved address), used by VB's own
`DLL_Import` primitive. That trailing data is runtime scratch. **[L]**

For `dwEntryType == 6`, `lpImportDescriptor` points at a pair
`{ VA descriptor, VA thunk }` where the descriptor is four dwords that AG
reports are identical across all VB applications. Skip these entries. **[L]**

### 7.2 Reconstructing the `Declare` statement

What survives: the library name and the **export** name. **[C]**

What does **not** survive:

- **The VB-level procedure name.** The source may have said
  `Declare Function FindWindow Lib "user32" Alias "FindWindowA" (...)`. Only
  `FindWindowA` is in the file. SVBD works around this by looking the export
  name up in a bundled `winapi.dat` of ~800 KB of known API declarations, then
  retrying with a trailing `A` or `W` stripped. That is a database, not
  recovery. **[C]**
- **The argument names and types of the `Declare`.** Not stored anywhere.
  **[C]** DeForm6 must either ship a curated Win32 declaration table or emit
  `Declare Function FindWindowA Lib "user32" (...) As Long` with a comment
  marking the signature as unknown.
- **`Private` vs `Public` on the `Declare`, and which module owned it.** SVBD's
  own comment: "the EXE does not record which module owned each Declare", so it
  emits the whole block once, as `Public`, in the first standard module. **[C]**

**Ordinal imports are a gap.** **[G]** A source-level
`Alias "#123"` should leave the literal string `"#123"` in `lpApiName`, since VB
stores the alias verbatim, but no source confirms this and no sample was
inspected. Implement it as: if the name string begins with `#` and the rest
parses as a decimal integer, emit `Alias "#N"`; otherwise emit the name. Flag
the ordinal path as inferred in the report.

### 7.3 The external component (OCX / typelib reference) table

Located by `VBHeader.lpExternalTable` (0x50) with `VBHeader.wExternalCount`
(0x46) entries. **Entries are variable-length and self-describing.** All
sub-offsets are relative to the **start of the entry**. **[L]** (SVBD
`tComponent` plus its use site in `frmMain`; no other source documents this.)

| Offset | Size | Name | Meaning |
|---|---|---|---|
| 0x00 | 4 | `StructLength` | Length of this entry. Next entry starts at `entry + StructLength`. |
| 0x04 | 4 | `oUuid` | Offset to a 16-byte binary GUID |
| 0x08 | 4 | | unknown |
| 0x0C | 4 | | unknown |
| 0x10 | 4 | | unknown |
| 0x14 | 4 | | unknown |
| 0x18 | 4 | | unknown |
| 0x1C | 4 | `GUIDoffset` | Offset to a **textual** GUID |
| 0x20 | 4 | `GUIDlength` | `-1` ⇒ no textual GUID at `GUIDoffset`. `72` ⇒ the textual GUID is 36 UTF-16 characters. Plan 03-16 corrects an earlier draft of this row, which named `oUuid` here by mistake: `GUIDlength` governs `GUIDoffset` alone, and `oUuid` (0x04) is a separate field. |
| 0x24 | 4 | | unknown |
| 0x28 | 4 | `FileNameOffset` | Offset to the OCX file name NTS |
| 0x2C | 4 | `SourceOffset` | Offset to the library name NTS, e.g. `"TabDlg"` |
| 0x30 | 4 | `NameOffset` | Offset to the component name NTS |

`SourceOffset`'s string is the key that links a form's external control (§8.7)
to its CLSID. This table is what produces the `Object=` lines of the `.vbp`.

**Five of thirteen fields are unknown.** **[G]** They are not needed for
`.vbp` reconstruction. Gap 17 (§11) tracks them; plan 03-16 did not measure
any of them, and none is closed.

### 7.3.1 Neither `oUuid` nor `GUIDoffset` holds the CLSID a project file declares, 2026-09-11

Plan 03-08 read `GUIDoffset`/`GUIDlength` as a third-party control's CLSID.
Phase 3's own verification found the value it decodes for
`MSWinsockLib.Winsock` does not match the identifier the corpus `.vbp` files
declare, and flagged the discrepancy as unresolved. Plan 03-16 measured both
candidate fields directly, against all three corpus executables that declare
a `MSWinsockLib.Winsock` component, to settle which field, if either, holds
the declared identifier.

**Ground truth.** All three `.vbp` files that reference this control declare
the same line:

    Object={248DD890-BB45-11CF-9ABC-0080C7E7B78D}#1.0#0; MSWINSCK.OCX

**The eighteen searches.** For each of the three executables, the whole
declared length of the component entry (`StructLength`, 368 bytes in every
one) was searched for the declared identifier
`248DD890-BB45-11CF-9ABC-0080C7E7B78D`, in six forms: the sixteen byte binary
layout with the first field in file order (the standard Microsoft binary GUID
encoding: the first three fields little-endian, the fourth raw), the same
layout with the first field reversed (the sixteen bytes in the order the hex
string itself reads, with no byte-swap), plain text upper case, plain text
lower case, sixteen bit (UTF-16LE) upper case, and sixteen bit lower case.
**None of the eighteen searches found the declared identifier anywhere in the
entry, in any of the three files.** A whole-file search (not bounded to the
entry) gave the same result: not found, in any form, in any file.

**What each identifier-shaped field decodes to.** Both fields decode
identically across all three corpus executables:

| Field | Entry offset | Decoded value | Matches the declared identifier? |
|---|---|---|---|
| `oUuid` | `0x04` (an offset field; the sixteen raw bytes it points to sit at entry offset `0x38` in all three files) | `248DD896-BB45-11CF-9ABC-0080C7E7B78D` | No. Differs by one byte: the low byte of `Data1` (`0x96` vs the declared `0x90`). |
| `GUIDoffset`/`GUIDlength` | `0x1C`/`0x20` (the text sits at entry offset `0xF8` in all three files) | `2c49f800-c2dd-11cf-9ad6-0080c7e7b78d` | No. Shares no digit pattern with the declared identifier. |

The absolute file offsets the sixteen raw `oUuid` bytes sit at are
`0x2180` (`TFTPClient.exe`), `0x1738` (`Server.exe`) and `0x1f18`
(`SubReality_WinsockSample.exe`); the component entries themselves start at
`0x2148`, `0x1700` and `0x1ee0` respectively.

**The selection.** Since neither field holds the declared identifier, this
repository selects `oUuid`: its own shape, a fixed sixteen byte binary
identifier, is the shape a CLSID takes, and its decoded value is the closer
of the two candidates to the declared one. `vb/ocx.rs::join_component` (plan
03-16) reports it, and a successful join always carries a caveat naming the
byte offset the value was read from and stating plainly that the value is
not confirmed against the control's own project file. A one byte difference
is never treated as a match: `Clsid`'s own equality is exact.

**What this measurement does not prove.** The corpus holds exactly one
third-party control, `MSWinsockLib.Winsock`, across three files that all
reference the same OCX. This is one control's own evidence, not a general
proof about the external component table entry for any other control. §11
gap 17's own five (six, counting `0x24`) truly unknown dwords remain
untouched by this measurement and stay open.

**The open question this raises.** FRM-04's own requirement text promises
the CLSID of each third-party OCX control; if no field of this entry, in the
one control this corpus can test, holds a value a registry lookup would
recognise, that wording may promise more than the format can deliver for
every control. This is a question for the human, not a decision this
document makes: see `03-16-SUMMARY.md`.

---

## 8. The GUI table and the form data stream

This is the most important section for DeForm6 and the least well documented in
the canonical sources. **AI does not document it at all**: his structure
diagram shows a "GUI Table" box and the document ends with "TBD LATER". AG
reaches it, names it `DialogsStruct`, and correctly identifies its 0x50-byte
stride and the pointer at 0x48, but does not decode the stream. **Everything
below the GUI table entry comes from SVBD, which is the only public
implementation that reconstructs `.frm` files.** Treat this whole section as
**[L]** at best unless a specific line says otherwise, and validate it against
the corpus early.

**Compilation mode is irrelevant here.** The form stream is design-time data
serialised by the same mechanism in native and P-code builds. **[C]**

### 8.1 The GUI table entry (`tGuiTable`)

An array of `VBHeader.wFormCount` entries at `VBHeader.lpGuiTable`, stride
`0x50` = 80 bytes. **[C]** on the stride and the array (AG's dump shows two
consecutive entries at `0x401A00` and `0x401A50`, each opening with `dd 50h`;
SVBD reads the array with the same stride).

| Offset | Size | Name | Meaning | Conf |
|---|---|---|---|---|
| 0x00 | 4 | `lStructSize` | `0x50`. AG confirms; a useful validation gate. | **[C]** |
| 0x04 | 16 | `uuidObjectGUI` | GUID of the object GUI | **[L]** |
| 0x14 | 16 | unknown (4 dwords) | | **[G]** |
| 0x24 | 4 | `lObjectID` | Object ID within the project | **[L]** |
| 0x28 | 4 | unknown | | **[G]** |
| 0x2C | 4 | `fOLEMisc` | OLEMISC flags | **[L]** |
| 0x30 | 16 | `uuidObject` | GUID of the object | **[L]** |
| 0x40 | 8 | unknown (2 dwords) | AG's first sample reads `0x596` here | **[G]** |
| 0x48 | 4 | `aFormPointer` | **VA of the `GUIObjectInfo` block (§8.2).** AG independently identifies "the 19th field" as the pointer to the form resource data. | **[C]** |
| 0x4C | 4 | unknown | AG's samples read `0x4C` and `0x9C` | **[G]** |

**The GUI table has one entry per form, not per object.** It does not include
modules or classes. Its length is `wFormCount`.

**There is no name in this entry.** The form's name comes from the stream
itself (§8.4).

### 8.2 `GUIObjectInfo`

At `tGuiTable.aFormPointer`. Size `0x5D` = 93 bytes. **[L]** (SVBD
`GUIObjectInfo`; corroborated by SVBD's own `Seek F, aFormPointer + 94`. VB's
`Seek` is 1-based, so `+94` is byte offset `0x5D`.)

| Offset | Size | Name | Meaning |
|---|---|---|---|
| 0x00 | 4 | `lUnknown1` | |
| 0x04 | 1 | `bUnknown2` | |
| 0x05 | 16 | `guidObjectGUI` | GUID of this object GUI |
| 0x15 | 16 | `uuidUnknown1` | |
| 0x25 | 16 | `guidCOMEventsIID` | Events IID of this object |
| 0x35 | 36 | nine unknown dwords | |
| 0x59 | 4 | `lPropertiesLength` | **Total byte length of the property stream that follows.** |
| 0x5D | | | **The control tree stream begins here.** |

Note the structure is **not** 4-byte aligned internally: a single byte at 0x04
throws every following field onto an odd offset. That is unusual enough to be
worth double-checking on the first real sample; if the read produces garbage,
the most likely error is a missing or extra byte around 0x04.

For VB4 files SVBD seeks to `aFormPointer + 9` instead. Out of scope. **[L]**

### 8.3 The stream in outline

The stream at `aFormPointer + 0x5D` is a depth-first serialisation of the
control tree. It is not a table; it is a walk. **[L]**

```
<control block for the form itself>
0xFF <scope byte(s)>
<control block for the first child>
0xFF <scope byte(s)>
<control block for the next control>
...
0xFF 0x04            end of form
```

Each control block is:

```
+0x00  u16   Length          length of the block, measured from block start + 2
+0x02  u8    unknown         SVBD names the field "uni" but never reads it
+0x03  u8    flags           0x80 ⇒ this control is a control ARRAY
...    header (§8.4)
...    property stream (§8.5), until block start + Length - 2
```

`Length` is used two ways in SVBD, both consistent with the above: the property
loop runs while the cursor is below `blockStart + Length - 2`, and the next
sibling block starts at `blockStart + Length + 2`.

**The byte at +0x02 is a lead worth chasing.** **[G]** SVBD names it `uni` in
its `ArrayTestType` and never uses it. If it is a Unicode flag for the name that
follows, it would settle a large part of §9. Test it early.

### 8.4 The control block header

Two layouts, selected by the flag byte at +0x03. **[L]**

**Non-array control** (`flags != 0x80`):

| Offset | Size | Meaning |
|---|---|---|
| 0x00 | 2 | `Length` |
| 0x02 | 1 | unknown |
| 0x03 | 1 | flags |
| 0x04 | 1 | `cId`, the control's ID, used to link it to its event handlers |
| 0x05 | 2 | name length, `n` |
| 0x07 | n | control name, ASCII |
| 0x07+n | 1 | unknown |
| 0x08+n | 1 | `cType`, the control type code, §8.4.1 |
| 0x09+n | | property stream begins |

**Control array** (`flags == 0x80`):

| Offset | Size | Meaning |
|---|---|---|
| 0x00 | 2 | `Length` |
| 0x02 | 1 | unknown |
| 0x03 | 2 | array flag / index |
| 0x05 | 1 | `cId` |
| 0x06 | 1 | unknown |
| 0x07 | 2 | name length, `n` |
| 0x09 | n | control name, ASCII |
| 0x09+n | 1 | unknown |
| 0x0A+n | 1 | `cType` |

The array form inserts two extra bytes ahead of `cId`. **The array index itself
is not clearly located.** **[G]** SVBD reads a 16-bit `arrayflag` at 0x03 and
discards it. A control array needs an `Index = N` property per element in the
emitted `.frm`, so this must be resolved before control arrays can be written
correctly.

**Gap 11 is closed.** The field this table names `cId` at 0x05 holds the
array `Index`, not `cId`. See section 14, "Gap 11 closed," for the offset,
the evidence, and what the measurement did not settle.

AG's dump independently corroborates the name encoding: he shows
`db 5,0,'Form1',0` and `db 0Bh,0,'Leimcrackme',0`, a 16-bit little-endian
length followed by ASCII bytes. **[C]**

#### 8.4.1 `cType` values **[L]** (SVBD `ControlType` enum)

| Value | Control | | Value | Control |
|---|---|---|---|---|
| 0 | PictureBox | | 17 | DirListBox |
| 1 | Label | | 18 | FileListBox |
| 2 | TextBox | | 19 | Menu |
| 3 | Frame | | 20 | MDIForm |
| 4 | CommandButton | | 22 | Shape |
| 5 | CheckBox | | 23 | Line |
| 6 | OptionButton | | 24 | Image |
| 7 | ComboBox | | 37 | Data |
| 8 | ListBox | | 38 | OLE |
| 9 | HScrollBar | | 40 | UserControl |
| 10 | VScrollBar | | 41 | PropertyPage |
| 11 | Timer | | 42 | UserDocument |
| 13 | Form | | 255 | **external (OCX) control**, §8.7 |
| 16 | DriveListBox | | | |

Values 12, 14, 15, 21, 25..36, 39 are unassigned in SVBD's enum. **[G]**
They line up with the `fMdlIntCtls` control IDs in §2.1 for 0..24, which is
strong evidence the two enumerations are the same numbering. 12 = Print,
14 = Screen, 15 = Clipboard and 21 = App are non-visual and would never appear
in a form tree, which explains their absence.

### 8.5 The property stream

Inside a control block, after the header and until `blockStart + Length - 2`,
properties are encoded as: **[L]**

```
u8 opcode
<payload, width determined by the property's declared type>
```

**The opcode is per-control-type.** Opcode `31` means `Appearance` on a
CommandButton, `BackStyle` on a Label and `DrawMode` on a Form. There is no
global opcode table.

**How SVBD resolves an opcode to a name.** It binds to `VB6.OLB` over COM,
enumerates the members of the control's coclass, and matches the opcode against
the member's index. It then reads the payload using the member's **declared
return type** from the same type library. This is why SVBD needs VB6 installed,
and why hardcoded property tables in other tools are incomplete. **[C]**
(SVBD `ProccessControls`, `ReturnGuiOpcode`, `ReturnDataType`.)

**DeForm6 cannot use that technique** (no VB6, no `VB6.OLB` redistribution
right), so it must ship its own opcode-to-property table, built once from a
type-library dump and committed as derived data. The table needs, per control
type, a map from opcode to `(property name, payload type)`.

Payload widths by declared type: **[L]**

| Declared type | Bytes consumed | Notes |
|---|---|---|
| `Byte` | 1 | |
| `Boolean` | 2 | Emitted as `-1` / `0` in the `.frm` |
| `Integer` | 2 | |
| `Long` | 4 | |
| `Single` | 4 | |
| `String` | `2 + n + 1` | u16 length, `n` bytes, trailing NUL. See §9. |
| `stdole.Picture` | `4 + 8 + m` | u32 `blobLen`; if it is `-1` the property is absent and only the 4 bytes are consumed. Otherwise an 8-byte picture header then `m = blobLen - 8` image bytes. See §8.8. |
| `Font` | 11 + name | `BeginProperty Font` block, §8.5.2 |

Some opcodes are not simple typed properties and must be special-cased per
control type before the generic path is tried. SVBD does exactly this.

#### 8.5.1 Special opcodes seen in SVBD

**Form / MDIForm (`cType` 13, 20)**

| Opcode | Property | Payload |
|---|---|---|
| 10 | `WindowState` | 1 byte |
| 11 | `MousePointer` | 1 byte |
| 25 | `ScaleMode` + flags | 1 byte scale mode; **if scale mode is 0 (User) skip 16 more bytes**; then a flags byte where `0x20` = `AutoRedraw`, `0x02` = `FontTransparent`; then one more byte |
| 27 | `DrawStyle` | 1 byte |
| 29 | `FillStyle` | 1 byte |
| 31 | `DrawMode` | 1 byte |
| 34 | `BorderStyle` | 1 byte |
| 37 | `LinkMode` | 1 byte |
| 53 | `ClientLeft/Top/Width/Height` | 16 bytes, four `(i16 value, i16 pad)` pairs |
| 61 | `LockControls` | 1 byte, `0xFF` ⇒ `-1` |
| 62 | `NegotiateMenus` | 1 byte, `0xFF` ⇒ `-1` |
| 64 | `Font` | font block |
| 65 | `Appearance` | 1 byte |
| 70 | `StartUpPosition` | 1 byte |
| 71 | `OLEDropMode` | 1 byte |
| 73 | `PaletteMode` | 1 byte |
| 0, 98, 99 | consume 1 byte, no output | |

**CommandButton (`cType` 4)**: 4 = position block (8 bytes), 10 = `MousePointer`,
22 = `DragMode`, 29 = `Font`, 31 = `Appearance`, 38 = `OLEDropMode`,
41 = `Style`.

**Label (`cType` 1)**: 5 = position block, 11 = `MousePointer`,
19 = `BorderStyle`, 20 = `Alignment`, 26 = `DragMode`, 31 = `BackStyle`,
32 = `DataSource`, 37 = `Font`, 39 = `Appearance`, 43 = `OLEDropMode`,
45 = `DataFormat`.

**ListBox (`cType` 8)**: 4 = position block, 10 = `MousePointer`,
20 = `List` (an item count then length-prefixed strings), 24 = `DragMode`,
29 = `MultiSelect`, 39 = `Font`, 40 = `DataSource`, 44 = `Appearance`,
49 = `OLEDragMode`, 50 = `OLEDropMode`, 51 = `Style`.

Note that the position opcode is **4** for CommandButton and ListBox, **5** for
Label and CheckBox, **2** for the Data control and **53** for Form. Confirming
this is a stark reminder that the opcode space is per-control.

The **position block** is 8 bytes read as four `i16` in the order
Left, Top, Width, Height. **If the first `i16` is `-32768` the block is instead
four `i32`** starting two bytes later, that is a 16-byte block. **[L]**
(SVBD `GetControlSize`.) This is a real escape hatch for coordinates outside
`i16` range and a parser that misses it will silently produce garbage
geometry.

#### 8.5.2 The `Font` block **[L]**

```
+0x00  u8   unknown
+0x01  u8   charset
+0x02  u8   unknown
+0x03  u8   style bits:  0x02 Italic, 0x04 Underline, 0x08 Strikethrough
+0x04  u16  weight
+0x06  u32  size in tenths of a thousandth of a point (divide by 10000)
+0x0A  u8   name length n
+0x0B  n    font name, ASCII
```

It renders as `BeginProperty Font ... EndProperty`.

### 8.6 Linking a control to its event handlers

The form stream gives the control **tree and properties**. The `ControlInfo`
array reached through `OptionalObjectInfo.lpControls` (§5.3) gives the control's
**GUID and event handler addresses**. The two are joined by control name.

`ControlInfo`, stride `0x28` = 40 bytes. **[D]** AI's layout is wrong at the
front; three independent implementations agree against him.

| Offset | Size | Name | Meaning | Conf |
|---|---|---|---|---|
| 0x00 | 2 | `fControlType` | Control kind. `0x40` = an intrinsic control with a plain event sink; `0x2E` = a COM control with an `IDispatch` sink. AI reads this as a **dword**; SVBD, PVB and IDC all read it as a **word**. Prefer word. | **[D]** |
| 0x02 | 2 | `wEventCount` | **Number of event slots.** AI puts this at 0x04. | **[D]** |
| 0x04 | 2 | unknown | IDC: `wFlagIndexRef` | **[G]** |
| 0x06 | 2 | `bWEventsOffset` | Offset into the memory struct to copy events | **[C]** |
| 0x08 | 4 | `lpGuid` | **VA of this control's 16-byte CLSID.** | **[C]** |
| 0x0C | 2 | `wIndex` | Control index. AI reads a dword at 0x0C. | **[D]** |
| 0x0E | 2 | unknown | | **[G]** |
| 0x10 | 2 | unknown | PVB: `wUnnamedEvents` | **[G]** |
| 0x12 | 2 | unknown | PVB: `wFlags`, `0x4` if unnamed events exist | **[G]** |
| 0x14 | 4 | `dwNull2` | Unused | **[C]** |
| 0x18 | 4 | `lpEventTable` | **VA of the event handler table.** | **[C]** |
| 0x1C | 4 | `lpIdeData` | IDE only | **[C]** |
| 0x20 | 4 | `lpszName` | **VA of the control name NTS, ASCII.** The join key to §8.4. | **[C]** |
| 0x24 | 4 | `dwIndexCopy` | Secondary index | **[C]** |

**Recommendation.** Use the word-based layout (SVBD / PVB / IDC). AI's is the
odd one out and the two sources that copy him (IDC's own header text credits
him, PVB's comments quote him) diverged from him precisely here, which means
their authors checked against real files and found him wrong.

**Event handler table.** At `lpEventTable`:

- Header, 6 dwords = `0x18` bytes: `{ 0, VA→ControlInfo, VA→ObjectInfo,
  VA→EVENT_SINK_QueryInterface, VA→EVENT_SINK_AddRef, VA→EVENT_SINK_Release }`.
  **[C]** (AG's `LocalDispatcher` dump, IDC `FixEventHandlerTable`, PVB
  `EventHandlerInfo`.)
- For a COM control (`fControlType == 0x2E`) the header is 10 dwords =
  `0x28` bytes, adding the four `IDispatch` slots
  (`GetTypeInfoCount`, `GetTypeInfo`, `GetIDsOfNames`, `Invoke`). **[L]** (PVB
  `ExtendedEventHandlerInfo`.)
- Then `wEventCount` dwords. A zero means the event has no handler in the
  source. A non-zero one points at a small stub.

**The slot index is the event's ordinal in the control's default source
interface.** There is no name in the file. PVB carries a hardcoded per-control
event-name list (crediting `vic4key/VB-Exe-Parser`); SVBD carries a per-control
index-remapping table; IDC hardcodes them per control GUID. **[C]** DeForm6
needs the same table, built as derived data.

The stub itself, in a **native** build: **[C]**

```
81 6C 24 04 <imm32>     sub  dword ptr [esp+4], imm32
E9 <rel32>              jmp  <handler>
```

`imm32` is `0xFFFF` for a method and a smaller value for an event (AG's sample
shows `0x3F`). PVB reads it as `wProcType` and treats `0xFFFF` as "method". The
handler address is at stub + 0x8 as a `jmp rel32`. IDC computes it as
`stub + 0x0D + dword(stub + 0x09)`.

In a **P-code** build the stub is a 13-byte sequence
`xor eax,eax / mov edx,<addr> / push <addr> / ret` (SVBD `MethodLinkPCode`), and
there is also a 5-byte `jmp rel32` variant (SVBD `MethodLinkNative`). **[L]**

### 8.7 External (OCX) controls in the form stream

`cType == 255`. **[L]** (SVBD.)

Immediately after the header, before any property opcode, comes a
length-prefixed string holding the control's **programmatic class name**, for
example `"TabDlg.SSTab"` or `"MSComDlg.CommonDialog"`. SVBD reads it with the
same string reader used for string properties and emits
`Begin TabDlg.SSTab <name>`.

**The CLSID is not in the form stream.** It is recovered by matching the library
part of that class name against `SourceOffset` in the external component table
(§7.3), which carries the GUID and the OCX file name. **[L]**

**The property blob of an OCX control is opaque.** It is the control's own
`IPersistStream` serialisation and interpreting it requires the OCX type
library. **[C]** (Confirmed as a limitation by SVBD's own comments and by the
prior-art survey.)

One control-agnostic fragment *is* readable. SVBD documents a fixed header that
VB writes around every external control's stream, and says it verified it
byte-for-byte against `richtx32.ocx` and `MSWINSCK.OCX`: **[L]**

```
signature   0x12344321   (bytes 21 43 34 12)
 +0x04      reserved, always 8
 +0x08      _ExtentX   (Long, HiMetric)
 +0x0C      _ExtentY   (Long, HiMetric)
 +0x10      reserved
 +0x14      _Version   (Long)
```

So `_ExtentX`, `_ExtentY` and `_Version` are recoverable for every OCX control
without instantiating it, by scanning the control's block for the signature.
Everything else in the blob should be emitted as an opaque
`OleObjectBlob = "<form>.frx":OFFSET` reference, which is what the commercial
tools do.

### 8.8 `.frx` blob offsets

**The `.frx` offset is not stored in the executable.** **[C]** This is the
single most important fact in this section and it is easy to get wrong.

In the `.frm` source, a bulk property reads
`Picture = "MyForm.frx":27A2`. In the compiled EXE, the same property is stored
**inline in the property stream**:

```
u32   blobLen        ( = imageLen + 8 ), or 0xFFFFFFFF meaning "no value"
u8[8] picture header
u8[imageLen] raw image bytes
```

The `.frx` offset is **synthesised by the decompiler** as a running cursor that
starts at 0 for each form and advances by `blobLen + 4` after each blob it
emits, where `blobLen` is the length the executable declares inline. **[M]**
(Measured in phase 3, plan 03-15: eleven consecutive gaps across
`FormPhysics.frx` and `frmTransparency.frx`, each file ending exactly at its
last item. The shipped constant is `FRX_ITEM_HEADER_LEN = 4`.)

This corrects the figure this section carried before, which read `blobLen + 12`
after the prior art. **[L]** (SVBD: `FRXAddress = FRXAddress + varHold + 12`,
where `varHold` is the inline `blobLen`.) The two figures agree once the base is
right. The executable's inline `blobLen` already equals `imageLen + 8`, so
`blobLen + 4` covers the same distance as `imageLen + 12`. The 12 byte item
header below is not in doubt. What was wrong was adding 12 to a `blobLen` that
already held 8 of those bytes, which over-advanced the cursor by 8 bytes per
blob. Add 4 to the declared `blobLen`, never 12.

The `+12` is the size of the `.frx` item header that the decompiler writes but
the EXE does not contain: **[C]** (SVBD `modFrx`, which credits Brad Martinez.)

```
FRXITEMHDR (12 bytes)
  +0x00  u32  dwSizeImageEx  = dwSizeImage + 8   ← equals the EXE's inline blobLen
  +0x04  u32  dwKey          = 0x0000746C  ("lt")
  +0x08  u32  dwSizeImage    = dwSizeImageEx - 8
```

So the EXE's inline `blobLen` **is** `dwSizeImageEx`, and the 8-byte inline
"picture header" is the 8 bytes that `dwSizeImageEx` counts beyond the image.
A deleted `Form.Icon` appears in a `.frx` as the bare header with no data:
`08 00 00 00 6C 74 00 00 00 00 00 00`.

Other `.frx` item header shapes: **[C]**

| Header | Used by | Layout |
|---|---|---|
| `FRXITEMHDRW` | `TextBox.Text` when `MultiLine = True` | u16 text size, then text |
| `FRXITEMHDRDW` | `Label.Caption`, VB3 `.frx` | u32 size, then data |
| `FRXITEMHDR` | intrinsic-control `StdPicture` | as above, 12 bytes |
| `FRXITEMHDREX` | `Comctl32.ocx` / `Mscomctl.ocx` `StdPicture` | u32 `dwSizeImage + 24`, 16-byte CLSID, u32 `0x746C`, u32 `dwSizeImage`; 28 bytes total |

**Consequence for DeForm6.** The `.frm` writer and the `.frx` writer are one
component, not two. They must emit blobs in exactly the order they are
encountered in the property stream, and the offsets in the `.frm` must be
produced by the same cursor that drives the `.frx` writer. Any independent
computation of an offset will drift.

Image format is detected from the first bytes of the blob, since the EXE does
not record it: `42 4D` BMP, `47 49 46` GIF, `FF D8` JPEG, `D7 CD` WMF (Aldus
placeable, full key `0x9AC6CDD7`), `20 45 4D 46` at offset 40 for EMF, and
`00 00 01 00` / `00 00 02 00` for ICO / CUR. **[C]** (SVBD `modFrx` and
`GetStdPicture`.)

### 8.9 Scope separators and the tree shape

After each control block comes a separator. **This is the least certain part of
the entire format.** **[G]**

SVBD reads a 16-bit little-endian value and compares it against:

| u16 value | Bytes | SVBD name |
|---|---|---|
| 511 (0x01FF) | `FF 01` | `vbFormNewChildControl` |
| 767 (0x02FF) | `FF 02` | `vbFormExistingChildControl` |
| 1023 (0x03FF) | `FF 03` | `vbFormChildControl` |
| 1279 (0x04FF) | `FF 04` | `vbFormEnd` |
| 1535 (0x05FF) | `FF 05` | `vbFormMenu` |

so the wire form is the byte `0xFF` followed by a scope byte. After a `02` or
`03` scope byte, SVBD then reads **further single bytes** in a loop, closing one
container level for each `02` or `03` it sees, and stopping when it reads a byte
greater than 3 or equal to 0. A trailing `04` means the form is finished.

The honest description is therefore: **the separator is `0xFF` followed by a run
of one or more scope bytes**, where `01` opens a sibling, `02` and `03` each
close one container level, `04` ends the form and `05` introduces a menu. SVBD's
handling of this is visibly heuristic, it contains a separate special case for
menus with its own counter and a `IdentNextMenu` flag, and it is the most
likely place for a decompiler to produce a mis-nested `.frm`.

**Recommendation for DeForm6.** Do not model this as a stack machine driven by
guesses. Model it as: read `0xFF`, then read scope bytes until one is `> 3` or
`0`, counting `02`/`03` as pops. Cross-validate the resulting nesting against an
independent signal: the sum of the control blocks' `Length` fields must exactly
tile `GUIObjectInfo.lPropertiesLength`. If it does not, refuse rather than emit
a wrong tree. Menus in particular should be validated against the
`fMdlIntCtls` Menu bit and against a `cType == 19` count.

---

## 9. String encoding

The prior-art note that encoding is inconsistent is correct, and the underlying
reason is worse than "inconsistent": **in the form property stream there is no
encoding flag that any public tool reads.**

### 9.1 What is definitely ASCII **[C]**

Everything reached through a pointer in the structure layer is a
null-terminated **ASCII** byte string:

| Field | Structure |
|---|---|
| `lpszObjectName` | `Object` 0x18 |
| entries of `lpProcNamesArray` | `Object` 0x20 |
| `lpszProjectName` | `ObjectTable` 0x40 |
| `szProjectDescription`, `szProjectHelpFile` | `ProjectInfo2` 0x18, 0x1C |
| `lpDllName`, `lpApiName` | `Declare` import descriptor |
| `lpszName` | `ControlInfo` 0x20 |
| `lpName` | `PubVarDesc` 0x00 |
| entries of `lpAryArgNames` | `FuncTypDesc` 0x10 |
| the four `VBHeader` offset strings | 0x58..0x64 |
| `FileNameOffset`, `SourceOffset`, `NameOffset` strings | external component table |

Every implementation (SVBD `GetUntilNull`, PVB `read_string(...).decode('ascii')`,
SEK, IDC) reads all of these as single-byte NTS. There is no dispute.

The **control name** in the form stream header is also single-byte: AG's
disassembly literally shows `db 5,0,'Form1',0`. **[C]**

### 9.2 What is definitely UTF-16 **[C]**

| Field | Source |
|---|---|
| The textual GUID in the external component table, when `GUIDlength == 72` | SVBD `GetGuidString` reads 36 wide characters |
| The `DataFormat` sub-properties on data-bound controls: format string, True/False/Null value strings | SVBD `GetUnicodeStringWLen` |
| `ProjectInfo.szPathInformation` content | AG: "usually you see here some unicode strings" |
| `DesignerData` `bstr*` fields | AI describes them as BSTRs, which are UTF-16 with a 32-bit byte-length prefix |

### 9.3 The form property stream: the actual rule **[D]** / **[G]**

A `String`-typed property in the form stream is:

```
u16   length
      <length characters, ASCII or UTF-16>
u8    0x00
```

SVBD's `GetAllString` does **not** read a flag. It:

1. reads the u16 `length`,
2. reads bytes until a NUL and measures the result,
3. if the measured length is **less than** `length`, and `length < 100`, rewinds
   two bytes and re-reads the field as UTF-16 of `length` characters.

That is a heuristic, and it is a bad one. It fails on an ASCII string whose
length is `>= 100`, and it fails on a UTF-16 string whose first character
happens to be at an even byte boundary with a high byte that is not zero (any
non-Latin-1 text). SVBD's own byte accounting confirms it is unsound: it
subtracts `Len(text) + 3` from the remaining block length regardless of which
branch it took, which is wrong by `length` bytes for every UTF-16 string.

**So the true rule is not known.** **[G]** The claim in `PRIOR-ART.md` that
`Caption` and `Name` are ASCII while `Tag` and `Connect` are Unicode is
consistent with everything above but no source states it as a rule, and no
source explains *why*.

**Three testable hypotheses, in order of likelihood:**

1. **The encoding is a property of the property, fixed per opcode.** The
   `VB6.OLB` member returns `String` either way, so the encoding would be a
   VB-internal convention baked into the persistence code. This matches the
   observed pattern (`Tag` and `Connect` are runtime-only, late-bound
   properties; `Caption` and `Name` are design-time ones) and it is the only
   hypothesis that gives a parser a deterministic rule. **Test:** compile one
   form with a `Caption`, a `Tag` and a `Connect` all set to the same ASCII
   text and diff the three encodings in the EXE.
2. **The unknown byte at control-block offset 0x02, which SVBD names `uni`,
   carries the flag for the block.** The name is suggestive. **Test:** compile
   one form with an all-ASCII caption and one with a caption containing a
   non-Latin-1 character and diff that byte.
3. **The `length` field is a byte count in one case and a character count in the
   other**, so that a UTF-16 field is self-identifying by parity or by the
   trailing NUL count. **Test:** same samples as above.

**Recommendation for DeForm6.** Implement a `VbStr` reader that takes an
explicit `Encoding` parameter supplied by the per-opcode property table, defaults
to ASCII, and **validates**: after reading, the cursor must land exactly on the
declared field end. If it does not, retry as the other encoding, and if that also
fails, refuse the property and record it in the report as unrecoverable rather
than guessing. Never let a string read determine how far the cursor advances;
always advance by the declared length. That is the difference between a
mis-decoded caption and a whole form that decodes into garbage.

---

## 10. Version and mode differences

### 10.1 VB5 versus VB6

| Item | VB5 | VB6 | Conf |
|---|---|---|---|
| Imported runtime | `MSVBVM50.DLL` | `MSVBVM60.DLL` | **[C]** |
| `VBHeader.szVbMagic` | `"VB5!"` | `"VB5!"`, **unchanged** | **[C]** |
| `ProjectInfo.dwVersion` | `0x1F4` | `0x1F4` | **[C]** |
| VBHeader layout | identical | identical | **[L]** |
| `Object.fObjectType` | SVBD's bit ladder labels one bit "vb5", so there may be a VB5 marker inside the object type word. Which bit is not resolvable from the value table. | | **[G]** |
| Property opcodes | resolved against `VB32.OLB` | resolved against `VB6.OLB` | **[C]** |

**The magic is not a version discriminator.** AG notes this with some
irritation: "the program has been written with VB 6.0, but the signature is
'VB5!'". **The only reliable discriminator is the imported runtime DLL name.**
This is exactly what DeForm6's "detect and refuse VB5 by name" requirement needs:
read the PE import directory, and refuse if the VB runtime import is
`MSVBVM50.DLL`. **[C]**

SVBD ships two API name databases (`vb5api.txt`, `vb6api.txt`) and two type
libraries (`VB32.OLB`, `VB6.OLB`), which implies the runtime helper set and the
control property opcode numbering both differ between VB5 and VB6. **[L]** That
is another reason not to attempt VB5 in this milestone.

### 10.2 Native versus P-code

**The discriminator is `ProjectInfo.lpNativeCode` at offset 0x20.**
Non-zero ⇒ native. Zero ⇒ P-code. **[C]** (SVBD branches on this in six places
and sets `AppData.CompileType` from it.)

| Item | Native | P-code | Conf |
|---|---|---|---|
| `ProjectInfo.lpNativeCode` | VA of the native code / `.data` | `0` | **[C]** |
| `ObjectInfo.lpMethods` entries | point into code | point at a `ProcDscInfo` (§10.3) | **[C]** |
| Event handler stub | `sub [esp+4], imm32` + `jmp rel32`, 13 bytes | `xor eax,eax / mov edx,addr / push addr / ret`, 13 bytes | **[L]** |
| Form data, GUI table, control tree, properties, `.frx` blobs | **identical** | **identical** | **[C]** |
| `FuncTypDesc` / `PubVarDesc` / `EventDesc` | present | present | **[C]** |

GD is explicit that the type-descriptor layer is part of the `IDispatch`
plumbing and "can not be disabled", so §6 works in both modes. AG and the
prior-art survey both confirm the form layer is mode-independent.

**Everything DeForm6 promises in this milestone is mode-independent.** The
`lpNativeCode` branch matters only for reporting and for the later code-recovery
milestone.

### 10.3 `ProcDscInfo` (P-code only) **[L]**

SEK is the only source that documents it, at
`ObjectInfo.lpMethods[i]`:

| Offset | Size | Name |
|---|---|---|
| 0x00 | 4 | `ProcTable`, VA of a 56-byte table whose last dword is a data constant |
| 0x04 | 2 | unknown |
| 0x06 | 2 | `FrameSize` |
| 0x08 | 2 | `ProcSize` |

The P-code body occupies the `ProcSize` bytes **immediately before** the
descriptor, that is `[descriptor - ProcSize, descriptor)`. Out of scope for this
milestone but recorded here because it is not documented anywhere else found.

---

## 11. Gap register

Collected so the roadmap can plan around them. Each one is a place where a wrong
answer stated confidently would be worse than no answer.

| # | Gap | Blocks | Resolution path |
|---|---|---|---|
| 1 | `VBHeader` 0x58 / 0x5C meaning (§2.3) | `.vbp` `Title=` / `ExeName32=` | CLOSED 2026-09-07, 44 of 44 corpus binaries. 0x58 is the EXE name, 0x5C is the title. Method and worked example in §13. |
| 2 | `OptionalObjectInfo` presence test (§5.5) | reading controls off a class | Use `fObjectType & 2`, validate on corpus |
| 3 | MDIForm `fObjectType` value (§5.5) | classifying MDI parents | Corpus scan for `cType == 20` forms |
| 4 | `ParamArray` type encoding (§6.5) | correct `.bas`/`.cls` signatures | Compile a `ParamArray` sample, diff |
| 5 | Type codes 0x00-0x02, 0x04, 0x07, 0x09, 0x0E, 0x11, 0x12, 0x14-0x1A (§6.5) | full prototype coverage | Compile variations, diff |
| 6 | `FuncTypDesc` offsets 0x06-0x0B (§6.3) | every prototype | Validate `constFFFF == 0xFFFF` on a real file |
| 7 | `optionalVals` target (§6.3) | `Optional x As Long = 5` default values | Debug the runtime, or diff compiled defaults |
| 8 | `PubVarDesc` record stride (§6.4) | walking public variables | Diff two adjacent records in a real file |
| 9 | Event name strings (§6.4) | naming recovered events | Positional heuristic after `ProcNamesArray`; mark inferred |
| 10 | Ordinal `Declare` encoding (§7.2) | `Alias "#123"` | Compile a sample |
| 11 | Control array index location (§8.4) | `Index = N` in `.frm` | CLOSED 2026-09-10, 30 array elements across 2 files. The array `Index` is the two byte value at control block offset 0x05. Method and worked example in §14. |
| 12 | Byte at control-block +0x02 ("uni") (§8.3) | possibly the string encoding flag | Compile ASCII vs non-Latin-1 caption, diff |
| 13 | String encoding rule (§9.3) | every string property | Three named experiments in §9.3 |
| 14 | Scope separator grammar (§8.9) | correct control nesting | PARTIALLY CLOSED 2026-09-11, 5 transitions across 2 programs. The two-level-deep menu close (a top-level menu with a child, then a second top-level menu with children) is measured; method and worked example in §15. What remains open, narrowed from this row's own prior text: (1) a menu that is itself a sibling within an already-open menu group, opening its own child, is byte-identical to a confirmed sibling case and unresolved (`corpus/public-domain/PassGen/PassGen.exe`, `menuAbout`, offset `0x21d0`); (2) a separate, unexplained failure where an expected scope separator (`0xFF`) is not there at all (`corpus/vb6-code/Map-editor-2D/Map Editor.exe`, `Main`, offset `0x170e`, byte `0x37`). Both tracked in `WINDOWS.md`. |
| 15 | Full opcode-to-property tables per control type (§8.5) | every property | Build from a type-library dump; ship as derived data |
| 16 | Nine unknown dwords in `GUIObjectInfo` 0x35-0x58 (§8.2) | nothing known | Leave opaque |
| 17 | Five unknown dwords in the external component entry (§7.3) | nothing known | Leave opaque. Not touched by plan 03-16 (§7.3.1): that plan measured the two already-named fields, `oUuid` and `GUIDoffset`/`GUIDlength`, not these unknown ones. Still open. |
| 18 | Entry-point stub variants `0x5A` / `0x11` (§1.3) | exotic inputs | Do not implement without a sample |
| 19 | Which object table count is the number of objects (§4) | the object count DeForm6 reports, and the Phase 2 object walk | CLOSED 2026-09-07, 44 of 44 corpus binaries against their `.vbp`. `wTotalObjects` is the count and `wCompiledObjects` is the array capacity. Method and numbers in §4.1. |

A closed row stays in this register and it is marked closed. A register that
drops a row loses the record of what was once uncertain, and Phase 6 has to
list every gap that is still open at release. That is easier to check against
a register that shows all nineteen rows with their state.

**The method that closed gap 1.** Read the four `u32` values at `VBHeader`
+0x58, +0x5C, +0x60 and +0x64 as offsets from the start of the header, read
the string at each one, and compare the four against the `.vbp` that declares
the executable under test. Select that `.vbp` by its `ExeName32` value and not
by the directory it sits in (§12).

The checking method has to handle two exceptions, and both cost a correct
answer on the first run:

1. A `.vbp` value can contain a double quote.
   `corpus/vb6-code/Sepia-effect/Sepia.vbp` holds
   `Title="Sepia / "Antique" Image Filter"`. Strip one leading and one
   trailing quote, and nothing more.
2. One corpus project has no `Title=` key at all, because VB6 omits the key
   when the title equals the project name. An absent key is not an empty
   title. Fall back to `Name=`.

---

## 12. Practical parse order for DeForm6

1. Parse the PE. Reject anything that is not 32-bit x86. Record `ImageBase` and
   build the RVA-to-file-offset map from the section table.
2. Read the import directory. Require a VB runtime import. Refuse
   `MSVBVM50.DLL` by name (§10.1). Refuse `VB40032.DLL` by name.
3. Locate the VB header (§1). Validate `"VB5!"`.
4. Read `VBHeader` (§2). Everything downstream is bounded by `wFormCount`,
   `wExternalCount`, and the two pointers `lpProjectData` and `lpGuiTable`.
5. Read `ProjectInfo` (§3). Record `lpNativeCode` for the mode report. Read the
   `Declare` table (§7.1).
6. Read `ObjectTable` (§4), then the `Object` array (§5.1). This yields the
   project's object names and kinds (§5.5).
7. For each object: `ObjectInfo` (§5.2), then `PrivateObj` (§6.1) for
   prototypes, then `OptionalObjectInfo` (§5.3) for controls if
   `fObjectType & 2`.
8. Read the external component table (§7.3) so OCX class names can be resolved
   to CLSIDs later.
9. Walk the GUI table (§8.1) and, for each form, the form stream (§8.2-§8.9),
   emitting `.frm` and `.frx` from one cursor.
10. Join controls to event handlers by name (§8.6).

**Two invariants to check at every step, because DeForm6 must not panic:**

- Every VA must resolve inside a mapped section before it is dereferenced.
- Every count (`wFormCount`, `wCompiledObjects`, `ProcCount`, `dwControlCount`,
  `wEventCount`, `dwExternalCount`, every `Length`) must be bounds-checked
  against the real file size **before** any allocation is sized from it. Several
  of these are `u32` fields read straight from attacker-controlled bytes.

---

## 12. What this document's spine was verified against, 2026-09-07

A script walked the full pointer chain on **all 44 corpus executables**. It is
not a citation. It ran, and these are its numbers.

The chain: PE `AddressOfEntryPoint`, resolved to a file offset through the
section table, then the `push imm32` operand as a virtual address, then
`VBHeader + 0x30` (`lpProjectData`), then `ProjectInfo + 0x04`
(`lpObjectTable`), then `ObjectTable + 0x30` (`lpObjectArray`), then each
`Object` record of `0x30` bytes at `+0x18` (`lpszObjectName`).

| Claim | Result |
|---|---|
| Entry stub is `push imm32` then `call rel32` | 44 of 44 |
| The pushed pointer lands on `VB5!` | 44 of 44 |
| `VBHeader + 0x30` reaches a usable ProjectInfo | 44 of 44 |
| `ProjectInfo + 0x20` (`lpNativeCode`) is non-zero | 44 of 44, all native |
| `ObjectTable + 0x40` gives a readable project name | 44 of 44 |
| `wCompiledObjects` bounds a walkable object array | 44 of 44 |
| Every object the project declares is recovered by name | **44 of 44** |

`lpNativeCode` being non-zero on all 44 agrees with the source: every vendored
`.vbp` carries `CompilationType=0`, which is native. The corpus therefore does
**not** exercise the P-code branch of this field. A P-code binary is needed
before that branch can be called tested.
`TimoKunze/ExplorerTreeView-VB6` is the known source of one.

### A trap the harness fell into first

The first run reported 42 of 44, not 44 of 44. Two projects appeared to be
missing an object named `cCommonDialog`.

The parser was right and the check was wrong. `cCommonDialog.cls` sits in those
two project directories but **is not listed in the `.vbp`**, so the compiler
never put it in the executable. The check had built its expectation from a
directory glob rather than from the file list the project declares.

**The differential gate must take its expectation from the `.vbp` file list,
never from a directory glob.** A project directory holds orphan source that was
never compiled, and a glob counts it as a recovery failure. This is the same
shape as the rule in `AGENTS.md`: build the state a test needs from the thing
that defines it, not from what happens to sit next to it.

One more detail the harness needs: a directory can hold several `.vbp` files
for one program. Select the project whose `ExeName32` matches the executable
under test. Two corpus projects require this.

---

## 13. Gap 1 closed: `VBHeader` 0x58 and 0x5C, 2026-09-07

Section 2.3 recorded a dispute. Alex Ionescu, the IDA script and `python-vb`
give one reading of these two fields. Semi VB Decompiler and the Sekoia parser
give another. AndreaGeddon's dump does not settle it.

**The corpus settles it. The fields are the executable name and the project
title, as SVBD and SEK say. AI, IDC and PVB are wrong.**

| Field | Meaning | Agreement |
|---|---|---|
| `0x58` | The executable name with the extension removed | 44 of 44 |
| `0x5C` | The project title | 44 of 44 |
| `0x60` | (control) | 44 of 44 |
| `0x64` | (control) | 44 of 44 |

23 of the 44 corpus programs carry an executable name that differs from the
project title, so those 23 discriminate between the two candidate readings
rather than merely being consistent with both. The other 21 hold the same
string in both fields and are consistent with either reading.

An earlier draft of this section said 24. That figure was wrong. It came from
comparing the binary against values parsed out of the `.vbp` files, and the
truncated `Title` described below inflated it by one. The figure above compares
the two binary fields against each other, which needs no `.vbp` parsing at all,
and it reconciles: 21 + 23 = 44.

### These four fields are offsets from the VB header, not virtual addresses

This is the detail that makes the field look wrong when it is read the obvious
way. `lpProjectData` at `0x30` is a **virtual address**. The fields at `0x58`,
`0x5C`, `0x60` and `0x64` are **byte offsets from the start of the VBHeader**.

A first attempt at this check treated `0x58` as a virtual address, resolved it
through the section table, and read the string `"MZ"` out of the DOS stub,
because the stored value happened to be small enough to look like an image
base offset. The value is `0x78`, and the VBHeader in that file sits at file
offset `0x1760`. `0x1760 + 0x78` is `0x17D8`, which is exactly where the name
`Mandelbrot` is stored.

**One structure therefore mixes two kinds of pointer.** This is the reason the
`Off`, `Rva` and `Va` newtypes exist and do not convert into one another
implicitly. A parser that has only `u32` will make this mistake and will report
a real string from the wrong place with confidence.

### A defect this check found in the checking harness

The first run reported 42 of 44 for `0x5C`. Both failures were faults in the
`.vbp` reader used to build the expectation, not in the binary.

1. One project has no `Title=` key at all. The compiler wrote the project name
   into the field instead. An absent key is not an empty title.
2. `corpus/vb6-code/Sepia-effect` has the title `Sepia / "Antique" Image
   Filter`. **A `.vbp` value can contain a double quote.** A regular expression
   of the form `"([^"]*)"` stops at the inner quote and silently returns a
   truncated value.

Both matter beyond this check. The Phase 4 `.vbp` writer must be able to write
a value containing a double quote, and the differential harness must be able to
read one back.

### 4.1.1 Confirmed a second time, independently

The measurement in §4.1 was re-run by the orchestrator against all 44 corpus
executables, selecting each program's `.vbp` by its `ExeName32` and counting
the `Form=`, `Module=`, `Class=`, `UserControl=` and `PropertyPage=` lines.

| Field | Equals the declared count |
|---|---|
| `wTotalObjects` at 0x2A | **44 of 44** |
| `wCompiledObjects` at 0x2C | **29 of 44** |
| `wObjectsInUse` at 0x2E | **44 of 44** |

The 15 disagreements are all the same shape. `wCompiledObjects` is the capacity
of the array rounded up, so a program that declares 1, 2 or 3 objects reports
4. `LockWorkStation` declares 1 and reports 4. `Grayscale` declares 3 and
reports 4.

**This corrects a recommendation written in this document earlier the same
day**, which said to loop on `wCompiledObjects` and to report a disagreement
with `wTotalObjects` as a damage indicator. Following it would have printed the
wrong object count for a third of the corpus and called 15 healthy files
damaged.

It also corrects the two open references that this document leaned on:
`python-vb` and the Gen Digital article both loop on `wCompiledObjects`.
Semi VB Decompiler loops on `wTotalObjects` and is right.

**An earlier check in this repository missed this**, and the reason is worth
recording. The orchestrator's first corpus walk looped on `wCompiledObjects`
and reported that every declared object was recovered in 44 of 44. It asserted
that the expected names were a **subset** of the recovered names, so the extra
slots that the capacity introduced added unexpected names without failing
anything. A subset assertion cannot see an over-count. The differential gate in
Phase 2 must compare both directions.

---

## 14. Gap 11 closed: the control array `Index` field, 2026-09-10

Section 8.4's array header table names the byte at control block offset 0x05
`cId`. It is not `cId`. It is the control array `Index`.

**The offset.** The control array `Index` is the two byte, little endian
value at control block offset 0x05, in the array header layout (`flags`
byte at offset 0x03 equal to `0x80`). Offset 0x03 and 0x04 together are a
flags-and-selector pair that stays constant per array group and does not
vary with `Index`; reading it as the index gives the same wrong number for
every element of one array.

**The evidence.** 30 array elements, across 2 files and 2 control types,
with array sizes of 2, 3 and 25:

- `corpus/vb6-code/Custom-image-filters/Custom_Filters.exe` (the program's
  own `ExeName32`, per its `.vbp`; earlier phase 3 documents cite
  `CustomFilters.exe`, which this corpus does not hold). The `TxtF` array,
  `VB.TextBox`, 25 elements, `Index` 0 through 24.
- `corpus/vb6-code/Grayscale-effect/Grayscale.exe`. The `optDecompose`
  array, `VB.OptionButton`, 2 elements, `Index` 1 and 0. The `optChannel`
  array, `VB.OptionButton`, 3 elements, `Index` 2, 1 and 0.

Every one of the 30 values read at offset 0x05 matches the element's own
`Index =` line in the committed `.frm` source. Zero disagreements.

**What the measurement did not settle.** The corpus cannot tell a one byte
field at 0x05 from the low byte of a two byte field spanning 0x05 and 0x06,
because no corpus array index exceeds 24. DeForm6 reads two bytes
defensively and gives a `Defect` when the high byte is non-zero, which
surfaces the case rather than deciding it. A program with a control array
index above 255 would close the question.

---

## 15. Gap 14, partially closed: the two-level-deep menu close, 2026-09-11

Section 8.9 recommends: "read `0xFF`, then read scope bytes until one is
`> 3` or `0`, counting `02`/`03` as pops." That reading, alone, covers the
single-level menu case plan 03-04 measured (§14's own sibling companion,
`Grayscale.exe`'s `mnuFile`/`mnuOpenImage`), but does not cover closing a
menu nested two levels deep back to a sibling menu at the form's own top
level: a top-level menu with a child, then a second top-level menu that
also has children. This session measured that transition directly.

**The rule.** The decision depends on which control the walk's own parent
stack currently has at its top, not only on which control was just read:

- When the stack top is **not** a menu, a bare `0xFF 0x02` (a run of one
  byte after the leading `0xFF`) after a menu control means that menu opens
  its own first child, matching plan 03-04's own finding, unchanged.
- When the stack top **is** a menu, `0x03` behaves the way `0x02` behaves
  for every other control: it adds a pop and the run continues. `0x02`
  becomes the run's own sibling terminal, at whatever pop count the run has
  accumulated (zero, if it is the run's own first byte).

**The evidence.** Five independent real transitions, across two programs,
all verified against each program's own committed `.frm` source, by name:

| Program | Transition | Bytes | Stack top | Role |
|---|---|---|---|---|
| `corpus/public-domain/HexScroll/Hex Scroll.exe`, offset `0x16fd` | `menuExit` to `menuAbout` (sibling of `menuFile`) | `FF 03 02` | `menuFile` | one pop, then sibling |
| `corpus/public-domain/UUID2/VB6/UUID2.exe`, offset `0x1918` | `menuExit` to `menuSettings` (sibling of `menuFile`) | `FF 03 02` | `menuFile` | one pop, then sibling |
| `corpus/public-domain/UUID2/VB6/UUID2.exe`, offset `0x1986` | `menuSave` to `menuAbout` (sibling of `menuSettings`) | `FF 03 02` | `menuSettings` | one pop, then sibling |
| `corpus/public-domain/UUID2/VB6/UUID2.exe`, offset `0x19cc` | `menuLicense` to `menuSep` (sibling, both children of `menuAbout`) | `FF 02` | `menuAbout` | zero pops, sibling |
| `corpus/public-domain/HexScroll/Hex Scroll.exe`, offset `0x1743` | `menuLicense` to `menuSep` (sibling, both children of `menuAbout`) | `FF 02` | `menuAbout` | zero pops, sibling |

The first three settle the transition this gap named: reading `0x03` as an
immediate terminal (plan 03-04's own rule) stopped the run two bytes too
early and left the real `0x02` byte to be misread as the start of a bogus
next control block, which is the exact and only cause of the
`MAX_UNEXPLAINED_TAIL` refusal `WINDOWS.md` finding 7 recorded. The last
two, one measurement in each of two independent programs, settle a second
case this gap's own text did not separately name: a bare `0x02` after a
menu that is itself already a sibling within an open menu group.
`FrmHex.frm` and `frmUUID2.frm` both now recover in full, matching the
committed `.frm` parent for parent, by name.

**What the measurement did not settle.** `corpus/public-domain/PassGen/PassGen.exe`
holds a third shape: `menuAbout`, itself a sibling within an already-open
menu (`menuHelp`), that genuinely opens its own child (`menuAboutForm`).
Its own trailing separator, at file offset `0x21d0`, is a bare `0xFF 0x02`
with the stack top a menu and zero pops, byte for byte identical to the
two confirmed sibling transitions above, which need the opposite role. No
byte in the control header or the property stream up to the separator
distinguishes the two; this session found none. `frmPassGen` still builds
a tree (the byte count tiles exactly, no control is lost), but
`menuAboutForm`, `menuSeparatorC` and `menuWebsite` recover with `menuHelp`
as their parent rather than `menuAbout`. Tracked as `WINDOWS.md`'s own
finding for this program, open. `corpus/vb6-code/Map-editor-2D/Map Editor.exe`'s
`Main` form fails for an unrelated reason (an expected scope separator
that is not there, at file offset `0x170e`) and is tracked separately too;
this session's own measurement did not explain it.

