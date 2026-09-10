//! Third party (OCX) controls, `cType` 255: the class name, the CLSID join
//! against the external component table, and the fixed OCX header
//! (`_ExtentX`, `_ExtentY`, `_Version`).
//!
//! Plan 03-08 fills this module. It serves FRM-04.
//!
//! # The class name (`STRUCTURES.md` section 8.7)
//!
//! A control block whose `cType` is `255` carries no ordinary property
//! stream at its own header's end. Instead, a length prefixed string holds
//! the control's programmatic class name, for example
//! `"MSWinsockLib.Winsock"`. [`read_external_control`] reads it with the
//! same declared length cursor discipline `vb/vbstr.rs` established: the
//! cursor always advances by the declared end, never by what the decode
//! found.
//!
//! The class name splits on its first dot: the library part is everything
//! before it, and the component part is everything after. The library part
//! is the join key against the external component table
//! (`vb/project.rs::Component::library`), which [`join_component`] uses to
//! recover the control's CLSID.

use std::fmt;

use crate::error::{Defect, DefectKind, Site};
use crate::read::region::{Off, Region};
use crate::vb::controltree::ControlHeader;
use crate::vb::project::ComponentTable;
use crate::vb::vbstr::{StrEncoding, VbStr};

/// A third party control's programmatic class name, split into its library
/// and component parts, with the CLSID [`join_component`] recovers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalControl {
    /// The class name, exactly as the file holds it, for example
    /// `"MSWinsockLib.Winsock"`.
    pub class_name: String,
    /// Everything before the class name's first dot, for example
    /// `"MSWinsockLib"`. The whole class name when it holds no dot.
    pub library: String,
    /// Everything after the class name's first dot, for example
    /// `"Winsock"`. Empty when the class name holds no dot.
    pub component: String,
    /// The absolute file offset of the class name's own length field.
    pub offset: u32,
    /// The control's CLSID, once [`join_component`] recovers one.
    pub clsid: Option<Clsid>,
}

/// Reads an external control's own class name.
///
/// `block` is the control's own bounded block region, the same shape
/// [`crate::vb::controltree::read_control_header`] reads its header from.
/// `header` is that block's own [`ControlHeader`]. The caller takes this
/// path only when `header.c_type` is `255`; this reader does not check it
/// again.
///
/// Gives `(ExternalControl, consumed, Vec<Defect>)`. `consumed` is the byte
/// count the class name occupies, from [`ControlHeader::header_len`]
/// onward: the cursor a caller advances by, always the class name's own
/// declared end, never however far the split or the decode looked.
///
/// # Defects
///
/// A declared length of `0` gives an empty class name and a [`Defect`]: an
/// external control with no class name cannot be joined. A declared length
/// that runs past the block's own end gives a [`Defect`] naming the byte
/// offset, through [`VbStr::read`]'s own bound check, before any allocation
/// is sized from it. A class name with no dot gives the whole string as the
/// library part, an empty component part, and a [`Defect`] naming the byte
/// offset: `STRUCTURES.md` section 8.7's own examples always hold one.
#[must_use]
pub fn read_external_control(
    block: &Region<'_>,
    header: &ControlHeader,
) -> (ExternalControl, u32, Vec<Defect>) {
    let mut defects = Vec::new();
    let name_start = header.header_len();
    let offset = block.file_offset(Off::new(name_start)).map_or(0, Off::get);

    let declared_len = block.u16_le(Off::new(name_start)).unwrap_or(0);
    if declared_len == 0 {
        defects.push(Defect {
            site: Site {
                offset,
                rva: None,
                structure: "ExternalControl",
                field: "class_name",
            },
            kind: DefectKind::EmptyName { offset },
        });
    }

    let (vb_str, str_defect) = VbStr::read(block, Off::new(name_start), StrEncoding::Ascii);
    if let Some(defect) = str_defect {
        defects.push(defect);
    }

    let class_name = vb_str.text().to_owned();
    let consumed = vb_str.declared_end().get().saturating_sub(name_start);

    let (library, component) = split_class_name(&class_name, offset, &mut defects);

    (
        ExternalControl {
            class_name,
            library,
            component,
            offset,
            clsid: None,
        },
        consumed,
        defects,
    )
}

/// Splits a class name on its first dot.
///
/// An empty class name (already flagged by [`read_external_control`]'s own
/// zero-length check) gives two empty parts and no second defect: a class
/// name with no text has no dot to be missing. A non-empty class name with
/// no dot gives the whole string as the library part, an empty component
/// part, and a [`Defect`] naming `offset`.
fn split_class_name(class_name: &str, offset: u32, defects: &mut Vec<Defect>) -> (String, String) {
    if class_name.is_empty() {
        return (String::new(), String::new());
    }
    match class_name.split_once('.') {
        Some((library, component)) => (library.to_owned(), component.to_owned()),
        None => {
            defects.push(Defect {
                site: Site {
                    offset,
                    rva: None,
                    structure: "ExternalControl",
                    field: "class_name",
                },
                kind: DefectKind::ClassNameNoDot { offset },
            });
            (class_name.to_owned(), String::new())
        }
    }
}

/// A control's CLSID: sixteen raw bytes, parsed from a decoded textual GUID.
///
/// The textual GUID `vb/project.rs::Component::guid_text` carries is 32 hex
/// digits in the eight-four-four-four-twelve grouping, with three internal
/// hyphens and no braces (`STRUCTURES.md` section 7.3). [`Clsid::parse`]
/// removes the hyphens and reads the 32 digits as bytes; [`Display`] renders
/// them back in the same fixed grouping, with braces added. GUID formatting
/// is written by hand: the output shape never varies, and this workspace
/// carries no `uuid` crate for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clsid([u8; 16]);

impl Clsid {
    /// Parses a decoded textual GUID into its sixteen raw bytes.
    ///
    /// Gives `None` when the text, once its hyphens are removed, is not
    /// exactly 32 hexadecimal digits. Case-insensitive: a lower case or an
    /// upper case textual GUID both parse the same sixteen bytes.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let hex: String = text.chars().filter(|&c| c != '-').collect();
        if hex.len() != 32 {
            return None;
        }
        let mut bytes = [0_u8; 16];
        for (i, slot) in bytes.iter_mut().enumerate() {
            let start = i.checked_mul(2)?;
            let end = start.checked_add(2)?;
            let digits = hex.get(start..end)?;
            *slot = u8::from_str_radix(digits, 16).ok()?;
        }
        Some(Self(bytes))
    }
}

impl fmt::Display for Clsid {
    /// Renders the eight-four-four-four-twelve hex groups with braces
    /// around them, upper case, matching the shape a `.vbp`'s own `Object=`
    /// line uses.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [
            b0,
            b1,
            b2,
            b3,
            b4,
            b5,
            b6,
            b7,
            b8,
            b9,
            b10,
            b11,
            b12,
            b13,
            b14,
            b15,
        ] = self.0;
        write!(
            f,
            "{{{b0:02X}{b1:02X}{b2:02X}{b3:02X}-{b4:02X}{b5:02X}-{b6:02X}{b7:02X}-{b8:02X}{b9:02X}-{b10:02X}{b11:02X}{b12:02X}{b13:02X}{b14:02X}{b15:02X}}}"
        )
    }
}

/// Joins `control`'s own class name against `table`, by an exact,
/// case-insensitive match against
/// [`Component::library`](crate::vb::project::Component::library). Never a
/// near match: a prefix match or a substring match could join a control to
/// the wrong CLSID, and a wrong GUID is worse than no GUID, because a reader
/// cannot tell it is wrong.
///
/// # The join key is the whole class name, not its own library part
///
/// `STRUCTURES.md` section 8.7 describes the join as matching "the library
/// part of that class name" against `SourceOffset`. This session measured
/// what `SourceOffset` actually holds, in both `Server.exe` and
/// `SubReality_WinsockSample.exe`: the full dotted string
/// `"MSWinsockLib.Winsock"`, identical to the external control's own whole
/// class name, not the bare `"MSWinsockLib"` prefix
/// [`read_external_control`]'s own split gives as `control.library`.
/// `Component::library` carries that same full string verbatim (see its own
/// doc comment and the corpus test proving it). Joining `control.library`
/// (the split prefix) against `component.library` (the whole name) would
/// therefore never match a real component; this function joins
/// `control.class_name` against it instead.
///
/// On a match whose own textual GUID decodes, fills `control.clsid` and
/// gives `None`. On no match, or on a match whose component declares no
/// binary GUID (`guid_text` is `None`, or does not parse), `control.clsid`
/// stays `None`, and this gives `Some` with a stated reason in plain words,
/// naming the class name: the CLSID is not recoverable from this file.
pub fn join_component(control: &mut ExternalControl, table: &ComponentTable) -> Option<String> {
    let Some(component) = table
        .components
        .iter()
        .find(|component| component.library.eq_ignore_ascii_case(&control.class_name))
    else {
        return Some(format!(
            "the control declares the class name {}, and no component entry in this program \
             declares that library, so the CLSID is not recoverable from this file",
            control.class_name
        ));
    };

    match component.guid_text.as_deref().and_then(Clsid::parse) {
        Some(clsid) => {
            control.clsid = Some(clsid);
            None
        }
        None => Some(format!(
            "the control declares the class name {}, and the component entry for {} declares \
             no binary GUID, so the CLSID is not recoverable from this file",
            control.class_name, component.library
        )),
    }
}

/// The four byte little endian signature VB writes at the start of the
/// fixed header every external control's own property blob carries,
/// `STRUCTURES.md` section 8.7. In the file the bytes appear in the order
/// `0x21 0x43 0x34 0x12`.
pub const OCX_SIGNATURE: u32 = 0x1234_4321;

/// The reserved field the fixed header carries at offset `0x04` from the
/// signature. Every sample `STRUCTURES.md` section 8.7 names holds `8`.
const OCX_RESERVED: u32 = 8;

/// The fixed header's own total length, signature through `_Version`
/// inclusive: `STRUCTURES.md` section 8.7 gives six fields at offsets
/// `0x00` through `0x14`, the last one four bytes wide.
const OCX_HEADER_LEN: u32 = 0x18;

/// The one control-agnostic fragment of an external control's own property
/// blob this repository can read without its type library:
/// `_ExtentX`/`_ExtentY`, in HiMetric units and carried raw (never converted
/// here; that belongs with the writer), and `_Version`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OcxHeader {
    /// The control's width, in HiMetric units, as the file stores it.
    pub extent_x: i32,
    /// The control's height, in HiMetric units, as the file stores it.
    pub extent_y: i32,
    /// The control's version.
    pub version: i32,
}

/// Everything in an external control's own property blob this repository
/// does not decode: reading it needs the control's own type library, which
/// this repository does not hold and may not redistribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpaqueBlob {
    /// The absolute file offset the opaque span starts at.
    pub offset: u32,
    /// The opaque span's own length in bytes. `0` when the blob holds no
    /// bytes at all past the class name.
    pub length: u32,
}

impl OpaqueBlob {
    /// The message `03-RESEARCH.md` Pattern 4 states, in the same words
    /// `vb/propstream.rs::PropertyValue::undecoded_message` uses for an
    /// unnamed property opcode: the two gaps are the same kind of fact.
    #[must_use]
    pub fn message(&self) -> String {
        format!(
            "External control property blob at offset {:#x}, length {} bytes: value not \
             decoded. Reading it needs the control's own type library, which this repository \
             does not hold and may not redistribute.",
            self.offset, self.length
        )
    }
}

/// Scans an external control's own property blob for the fixed OCX header,
/// bounded by `block_end`, and reports the rest of the blob as opaque.
///
/// `block` is the control's own block region, the same shape
/// [`read_external_control`]'s own `block` parameter is. `blob_start` is
/// where the property blob begins, in `block`-relative offset terms: after
/// the class name [`read_external_control`] read. `block_end` is the
/// block's own end (`Length - 1`, matching `vb/propstream.rs`'s own bound,
/// itself corrected from `03-RESEARCH.md`'s stale `Length - 2` per plan
/// 03-04's own measurement).
///
/// The scan reads no byte past `block_end`: every candidate position is
/// checked against it, both for the four byte signature word and for the
/// full 24 byte header, before either is read. A signature sitting too
/// close to the block's own end for the full header to fit is therefore
/// never treated as a complete header, and no byte belonging to whatever
/// follows the block (the next control, or the scope separator run) is
/// ever read.
///
/// Gives `(Option<OcxHeader>, OpaqueBlob, Vec<Defect>)`. The [`OpaqueBlob`]
/// is always given, at `blob_start`, with a length covering the whole
/// scanned span (`block_end` minus `blob_start`) minus the fixed header's
/// own 24 bytes when one is found: everything in the blob other than the
/// fixed header is opaque, wherever inside the span the header sits. A blob
/// with no signature gives no header and no [`Defect`]: that absence is a normal
/// state, not a fault.
#[must_use]
pub fn read_ocx_blob(
    block: &Region<'_>,
    blob_start: u32,
    block_end: u32,
) -> (Option<OcxHeader>, OpaqueBlob, Vec<Defect>) {
    let mut defects = Vec::new();
    let mut header = None;

    let mut sig_at = blob_start;
    while sig_at < block_end {
        if let Some(sig_end) = sig_at.checked_add(4)
            && sig_end <= block_end
            && let Some(sig) = block.u32_le(Off::new(sig_at))
            && sig == OCX_SIGNATURE
            && let Some(header_end) = sig_at.checked_add(OCX_HEADER_LEN)
            && header_end <= block_end
        {
            let (found, mut header_defects) = read_ocx_header_at(block, sig_at);
            defects.append(&mut header_defects);
            header = Some(found);
            break;
        }
        sig_at = sig_at.saturating_add(1);
    }

    let span = block_end.saturating_sub(blob_start);
    let opaque = OpaqueBlob {
        offset: block.file_offset(Off::new(blob_start)).map_or(0, Off::get),
        // Everything in the blob other than the fixed header is opaque: the
        // header's own OCX_HEADER_LEN bytes are subtracted from the whole
        // scanned span when it is found, wherever inside the span it sits.
        length: if header.is_some() {
            span.saturating_sub(OCX_HEADER_LEN)
        } else {
            span
        },
    };

    (header, opaque, defects)
}

/// Reads the fixed header's own three recoverable fields at `sig_at`, the
/// absolute position of the signature the caller already confirmed the
/// full header fits at.
fn read_ocx_header_at(block: &Region<'_>, sig_at: u32) -> (OcxHeader, Vec<Defect>) {
    let mut defects = Vec::new();

    let reserved_at = sig_at.checked_add(4);
    let reserved = reserved_at
        .and_then(|at| block.u32_le(Off::new(at)))
        .unwrap_or(0);
    if reserved != OCX_RESERVED {
        let offset = reserved_at
            .and_then(|at| block.file_offset(Off::new(at)))
            .map_or(0, Off::get);
        defects.push(Defect {
            site: Site {
                offset,
                rva: None,
                structure: "OcxHeader",
                field: "reserved",
            },
            kind: DefectKind::OcxReservedFieldUnexpected {
                offset,
                value: reserved,
            },
        });
    }

    let extent_x = sig_at
        .checked_add(8)
        .and_then(|at| block.i32_le(Off::new(at)))
        .unwrap_or(0);
    let extent_y = sig_at
        .checked_add(12)
        .and_then(|at| block.i32_le(Off::new(at)))
        .unwrap_or(0);
    let version = sig_at
        .checked_add(20)
        .and_then(|at| block.i32_le(Off::new(at)))
        .unwrap_or(0);

    (
        OcxHeader {
            extent_x,
            extent_y,
            version,
        },
        defects,
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{
        Clsid, ExternalControl, OCX_SIGNATURE, join_component, read_external_control, read_ocx_blob,
    };
    use crate::error::DefectKind;
    use crate::read::region::{Off, Region};
    use crate::vb::controltree::{ControlKind, classify_control_type, read_control_header};
    use crate::vb::project::{Component, ComponentTable};

    /// Builds a synthetic non-array control block: `Length`(2) `unknown`(1)
    /// `flags=0`(1) `cId`(1) `name_len`(2) `name`(n) `unknown`(1)
    /// `cType`(1)=255, followed by `tail`, the raw bytes that would follow
    /// the header for an external control (the class name, and whatever
    /// comes after it). This fixture is built here, per `AGENTS.md`: a test
    /// builds the state it needs and does not read it out of a file the
    /// author edits.
    fn external_control_block(name: &str, tail: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0u8, 0u8]; // Length placeholder
        bytes.push(0); // unknown
        bytes.push(0); // flags, non-array
        bytes.push(0); // cId
        let name_len = u16::try_from(name.len()).unwrap();
        bytes.extend_from_slice(&name_len.to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0); // unknown, the "0x07+n unknown" byte
        bytes.push(255); // cType = external control
        bytes.extend_from_slice(tail);
        let total = u16::try_from(bytes.len()).unwrap();
        bytes[0..2].copy_from_slice(&total.to_le_bytes());
        bytes
    }

    /// A length prefixed ASCII string, the shape `VbStr` reads: `u16` length,
    /// the text, then a trailing null.
    fn length_prefixed(text: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let len = u16::try_from(text.len()).unwrap();
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(text.as_bytes());
        out.push(0);
        out
    }

    #[test]
    fn a_class_name_with_a_dot_splits_into_library_and_component() {
        let tail = length_prefixed("MSWinsockLib.Winsock");
        let bytes = external_control_block("wsPop", &tail);
        let region = Region::new(&bytes, Off::new(0));
        let (header, header_defects) = read_control_header(&region);
        assert!(header_defects.is_empty(), "{header_defects:?}");
        assert_eq!(header.c_type, 255);

        let (control, consumed, defects) = read_external_control(&region, &header);
        assert_eq!(control.class_name, "MSWinsockLib.Winsock");
        assert_eq!(control.library, "MSWinsockLib");
        assert_eq!(control.component, "Winsock");
        assert_eq!(consumed, u32::try_from(tail.len()).unwrap());
        assert!(defects.is_empty(), "{defects:?}");
    }

    /// Synthetic fixture: no corpus program's own class name lacks a dot, so
    /// this case is proved here rather than against real bytes.
    #[test]
    fn a_class_name_with_no_dot_gives_the_whole_string_as_the_library_part_and_a_defect() {
        let tail = length_prefixed("Foo");
        let bytes = external_control_block("Foo1", &tail);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);

        let (control, _consumed, defects) = read_external_control(&region, &header);
        assert_eq!(
            control.library, "Foo",
            "synthetic fixture: a class name with no dot gives the whole string as the \
             library part"
        );
        assert_eq!(control.component, "");
        assert_eq!(defects.len(), 1);
        assert!(matches!(defects[0].kind, DefectKind::ClassNameNoDot { .. }));
        let message = format!("{}", defects[0].kind);
        let wanted = format!("{:#x}", control.offset);
        assert!(message.find(&wanted).is_some(), "{message}");
    }

    #[test]
    fn a_declared_length_of_zero_gives_an_empty_class_name_and_a_defect() {
        let tail = vec![0x00, 0x00, 0x00]; // declared length 0, then a trailing null
        let bytes = external_control_block("X", &tail);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);

        let (control, _consumed, defects) = read_external_control(&region, &header);
        assert_eq!(control.class_name, "");
        assert_eq!(control.library, "");
        assert_eq!(control.component, "");
        assert_eq!(defects.len(), 1);
        assert!(matches!(defects[0].kind, DefectKind::EmptyName { .. }));
    }

    /// A declared length far larger than the bytes this fixture holds gives
    /// a `Defect` through `VbStr::read`'s own bound check, and no allocation
    /// is sized from it: `Region::take` refuses the read rather than
    /// allocating first.
    #[test]
    fn a_declared_length_past_the_block_end_gives_a_defect_and_sizes_no_allocation() {
        let mut tail = vec![0x64, 0x00]; // declared length 100
        tail.extend_from_slice(&[0xAA, 0xAA]); // far fewer bytes actually present
        let bytes = external_control_block("X", &tail);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);

        let (control, _consumed, defects) = read_external_control(&region, &header);
        assert_eq!(control.class_name, "");
        assert_eq!(defects.len(), 1);
        let message = format!("{}", defects[0].kind);
        assert!(message.find("100").is_some(), "{message}");
    }

    #[test]
    fn a_c_type_of_254_does_not_classify_as_external() {
        assert_eq!(classify_control_type(254), ControlKind::Unknown(254));
        assert_eq!(classify_control_type(255), ControlKind::External);
    }

    /// Every fixture this module's own tests build is a synthetic byte
    /// array, named as such in this comment and in the assertion messages
    /// above that call it out directly.
    #[test]
    fn the_external_control_struct_carries_no_clsid_before_the_join() {
        let control = ExternalControl {
            class_name: "Foo.Bar".to_owned(),
            library: "Foo".to_owned(),
            component: "Bar".to_owned(),
            offset: 0,
            clsid: None,
        };
        assert!(control.clsid.is_none());
    }

    /// A class name with more than one dot splits on the first: everything
    /// after it, dots included, is the component part.
    #[test]
    fn a_class_name_with_two_dots_splits_on_the_first() {
        let tail = length_prefixed("A.B.C");
        let bytes = external_control_block("X", &tail);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (control, consumed, defects) = read_external_control(&region, &header);
        assert_eq!(control.library, "A");
        assert_eq!(control.component, "B.C");
        assert_eq!(consumed, u32::try_from(tail.len()).unwrap());
        assert!(defects.is_empty(), "{defects:?}");
    }

    /// The cursor advance is the class name's own declared end, never
    /// however far the split looked: a declared length of `0` still
    /// consumes exactly 3 bytes (the length field plus the trailing null),
    /// the same shape `VbStr::declared_end` gives for any other empty
    /// string.
    #[test]
    fn a_declared_length_of_zero_still_consumes_three_bytes() {
        let tail = vec![0x00, 0x00, 0x00];
        let bytes = external_control_block("X", &tail);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (_control, consumed, _defects) = read_external_control(&region, &header);
        assert_eq!(consumed, 3);
    }

    /// `SK-Winsock-Sample__VB6`'s own `frmMain.frm` declares
    /// `Begin MSWinsockLib.Winsock wsPop` nested two levels deep, inside
    /// `Frame1`. The control block's own file offset (`0x1d68`) was
    /// measured this session by searching the executable for the length
    /// prefixed class name string and stepping back by the non-array
    /// header's own fixed layout (`0x09 + len("wsPop")` = 14 bytes),
    /// `STRUCTURES.md` section 8.4. `[VERIFIED: local]`
    const WINSOCK_SAMPLE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/SK-Winsock-Sample__VB6/demo/\
         SubReality_WinsockSample.exe"
    ));

    /// The measured file offset of `wsPop`'s own control block: the
    /// `Length` field, at `WINSOCK_SAMPLE[0x1d68..0x1d6a]`, reads `0x0061`
    /// (97), and the bytes from there read `cId=9, name="wsPop",
    /// cType=0xFF` (255, external), matching `STRUCTURES.md` section 8.4's
    /// non-array layout exactly.
    const WSPOP_BLOCK_START: u32 = 0x1d68;

    /// Gives `wsPop`'s own control block, bounded by its own declared
    /// `Length + 2` span, read live from [`WSPOP_BLOCK_START`] rather than
    /// from a second hardcoded length: a wrong offset would give an
    /// implausible `Length` and this helper would panic, which is the
    /// point.
    fn wspop_block() -> Region<'static> {
        let file = Region::new(WINSOCK_SAMPLE, Off::new(0));
        let length = file
            .u16_le(Off::new(WSPOP_BLOCK_START))
            .expect("the measured offset must hold a readable Length field");
        assert_eq!(
            length, 0x61,
            "the measured offset no longer points at wsPop's own block"
        );
        file.subregion(Off::new(WSPOP_BLOCK_START), u32::from(length) + 2)
            .expect("the block's own declared span must fit inside the file")
    }

    #[test]
    fn the_winsock_sample_external_control_class_name_reads_back_whole() {
        let block = wspop_block();
        let (header, header_defects) = read_control_header(&block);
        assert!(header_defects.is_empty(), "{header_defects:?}");
        assert_eq!(header.name, "wsPop");
        assert_eq!(header.c_type, 255);

        let (control, _consumed, defects) = read_external_control(&block, &header);
        assert_eq!(control.class_name, "MSWinsockLib.Winsock");
        assert_eq!(control.library, "MSWinsockLib");
        assert_eq!(control.component, "Winsock");
        assert!(defects.is_empty(), "{defects:?}");
    }

    // --- Task 2: the CLSID, decoded and joined ----------------------------

    /// Walks the external component table out of a byte slice, the same
    /// route `vb/project.rs`'s own test module uses: reached from the
    /// header, never from `ProjectInfo`.
    fn component_table(data: &[u8]) -> ComponentTable {
        use crate::read::pe::PeImage;
        use crate::vb::header::{VbHeader, header_region};

        let image = PeImage::parse(data).unwrap();
        let hdr = header_region(&image).unwrap();
        let header = VbHeader::read(&hdr).unwrap();
        ComponentTable::read(&image, header.lp_external_table, header.w_external_count)
    }

    /// The end to end join: `wsPop`'s own class name, read from the real
    /// executable in [`the_winsock_sample_external_control_class_name_reads_back_whole`]'s
    /// own way, joined against the SAME file's own external component
    /// table. This session measured the textual GUID at `GUIDoffset` for
    /// `MSWinsockLib.Winsock` as `2c49f800-c2dd-11cf-9ad6-0080c7e7b78d`, not
    /// the `248DD890-BB45-11CF-9ABC-0080C7E7B78D` the `.vbp`'s own `Object=`
    /// line names: see `vb/project.rs`'s own
    /// `the_one_component_server_exe_declares_resolves_all_three_strings`
    /// test for the full measurement and why the two differ. That measured
    /// value, confirmed identically in this file and in `Server.exe`, is
    /// what this test proves the join recovers.
    #[test]
    fn the_winsock_sample_class_name_joins_to_its_real_clsid() {
        let block = wspop_block();
        let (header, _) = read_control_header(&block);
        let (mut control, _consumed, defects) = read_external_control(&block, &header);
        assert!(defects.is_empty(), "{defects:?}");

        let table = component_table(WINSOCK_SAMPLE);
        let reason = join_component(&mut control, &table);
        assert_eq!(reason, None, "{reason:?}");
        let clsid = control.clsid.expect("the join must recover a CLSID");
        assert_eq!(clsid.to_string(), "{2C49F800-C2DD-11CF-9AD6-0080C7E7B78D}");
    }

    /// A synthetic fixture: no corpus program's own class name differs from
    /// its component's own library only by case, so this is proved here.
    #[test]
    fn a_class_name_differing_only_by_case_still_joins() {
        let mut control = ExternalControl {
            class_name: "mswinsocklib.winsock".to_owned(),
            library: "mswinsocklib".to_owned(),
            component: "winsock".to_owned(),
            offset: 0,
            clsid: None,
        };
        let table = ComponentTable::synthetic(vec![Component {
            file_name: "MSWINSCK.OCX".to_owned(),
            library: "MSWinsockLib.Winsock".to_owned(),
            name: "Winsock".to_owned(),
            guid_offset: Off::new(0),
            guid_length: 72,
            guid_text: Some("2c49f800-c2dd-11cf-9ad6-0080c7e7b78d".to_owned()),
        }]);

        let reason = join_component(&mut control, &table);
        assert_eq!(
            reason, None,
            "synthetic fixture: a class name differing only by case must still join"
        );
        assert_eq!(
            control.clsid,
            Clsid::parse("2c49f800-c2dd-11cf-9ad6-0080c7e7b78d")
        );
    }

    /// A synthetic fixture proving `join_component` takes no near match: a
    /// class name differing from the component's own library by one
    /// character gives no CLSID.
    #[test]
    fn a_class_name_differing_by_one_character_gives_no_clsid_and_a_reason() {
        let mut control = ExternalControl {
            class_name: "MSWinsockLiC.Winsock".to_owned(),
            library: "MSWinsockLiC".to_owned(),
            component: "Winsock".to_owned(),
            offset: 0,
            clsid: None,
        };
        let table = ComponentTable::synthetic(vec![Component {
            file_name: "MSWINSCK.OCX".to_owned(),
            library: "MSWinsockLib.Winsock".to_owned(),
            name: "Winsock".to_owned(),
            guid_offset: Off::new(0),
            guid_length: 72,
            guid_text: Some("2c49f800-c2dd-11cf-9ad6-0080c7e7b78d".to_owned()),
        }]);

        let reason = join_component(&mut control, &table);
        assert!(control.clsid.is_none());
        let reason = reason.expect("synthetic fixture: a one character difference must refuse");
        assert!(reason.find(&control.class_name).is_some(), "{reason}");
    }

    /// A program with zero components gives no CLSID for an external
    /// control, with the same shape of reason a real no-match gives.
    #[test]
    fn a_program_with_zero_components_gives_no_clsid_with_the_same_reason() {
        let mut control = ExternalControl {
            class_name: "Foo.Bar".to_owned(),
            library: "Foo".to_owned(),
            component: "Bar".to_owned(),
            offset: 0,
            clsid: None,
        };
        let table = ComponentTable::synthetic(Vec::new());

        let reason = join_component(&mut control, &table);
        assert!(control.clsid.is_none());
        let reason = reason.expect("synthetic fixture: zero components must refuse");
        assert!(reason.find(&control.class_name).is_some(), "{reason}");
    }

    #[test]
    fn a_clsid_renders_the_fixed_hex_groups_with_braces() {
        let clsid = Clsid::parse("2c49f800-c2dd-11cf-9ad6-0080c7e7b78d").unwrap();
        assert_eq!(clsid.to_string(), "{2C49F800-C2DD-11CF-9AD6-0080C7E7B78D}");
    }

    #[test]
    fn a_clsid_text_that_is_not_thirty_two_hex_digits_gives_none() {
        assert_eq!(Clsid::parse("not a guid"), None);
        assert_eq!(Clsid::parse("2c49f800-c2dd-11cf-9ad6-0080c7e7b78"), None);
    }

    #[test]
    fn clsid_parse_is_case_insensitive() {
        let lower = Clsid::parse("2c49f800-c2dd-11cf-9ad6-0080c7e7b78d");
        let upper = Clsid::parse("2C49F800-C2DD-11CF-9AD6-0080C7E7B78D");
        assert_eq!(lower, upper);
    }

    /// A synthetic fixture: a matched component whose own `guid_text` is
    /// `None` (the `guid_length == -1` case) gives no CLSID and a reason,
    /// distinct from the "no matching component" reason.
    #[test]
    fn a_matched_component_with_no_guid_text_gives_no_clsid_and_a_reason() {
        let mut control = ExternalControl {
            class_name: "MSWinsockLib.Winsock".to_owned(),
            library: "MSWinsockLib".to_owned(),
            component: "Winsock".to_owned(),
            offset: 0,
            clsid: None,
        };
        let table = ComponentTable::synthetic(vec![Component {
            file_name: "MSWINSCK.OCX".to_owned(),
            library: "MSWinsockLib.Winsock".to_owned(),
            name: "Winsock".to_owned(),
            guid_offset: Off::new(0),
            guid_length: -1,
            guid_text: None,
        }]);

        let reason = join_component(&mut control, &table);
        assert!(control.clsid.is_none());
        let reason = reason.expect("synthetic fixture: no binary GUID must refuse");
        assert!(reason.find(&control.class_name).is_some(), "{reason}");
    }

    /// A synthetic fixture: a matched component whose `guid_text` does not
    /// parse as 32 hex digits gives no CLSID and a reason, the same
    /// treatment as a component with no `guid_text` at all.
    #[test]
    fn a_matched_component_whose_guid_text_does_not_parse_gives_no_clsid_and_a_reason() {
        let mut control = ExternalControl {
            class_name: "MSWinsockLib.Winsock".to_owned(),
            library: "MSWinsockLib".to_owned(),
            component: "Winsock".to_owned(),
            offset: 0,
            clsid: None,
        };
        let table = ComponentTable::synthetic(vec![Component {
            file_name: "MSWINSCK.OCX".to_owned(),
            library: "MSWinsockLib.Winsock".to_owned(),
            name: "Winsock".to_owned(),
            guid_offset: Off::new(0),
            guid_length: 72,
            guid_text: Some("not a real guid at all, thirty six characters".to_owned()),
        }]);

        let reason = join_component(&mut control, &table);
        assert!(control.clsid.is_none());
        assert!(reason.is_some());
    }

    /// A synthetic fixture proving `join_component` takes no near match at
    /// the prefix level: a component whose library is a strict prefix of
    /// the control's own class name (or the reverse) does not join.
    #[test]
    fn join_component_takes_no_prefix_near_match() {
        let mut control = ExternalControl {
            class_name: "MSWinsockLib.Winsock".to_owned(),
            library: "MSWinsockLib".to_owned(),
            component: "Winsock".to_owned(),
            offset: 0,
            clsid: None,
        };
        // The component's own library is a strict prefix of the control's
        // class name: a `starts_with` join would wrongly match this.
        let table = ComponentTable::synthetic(vec![Component {
            file_name: "MSWINSCK.OCX".to_owned(),
            library: "MSWinsockLib".to_owned(),
            name: "Winsock".to_owned(),
            guid_offset: Off::new(0),
            guid_length: 72,
            guid_text: Some("2c49f800-c2dd-11cf-9ad6-0080c7e7b78d".to_owned()),
        }]);

        let reason = join_component(&mut control, &table);
        assert!(
            control.clsid.is_none(),
            "synthetic fixture: a prefix match must not join"
        );
        assert!(reason.is_some());
    }

    /// An `ExternalControl` with an empty class name (`read_external_control`
    /// already flags this with `DefectKind::EmptyName`) never matches any
    /// component, since no component's own library is the empty string.
    #[test]
    fn an_empty_class_name_never_joins() {
        let mut control = ExternalControl {
            class_name: String::new(),
            library: String::new(),
            component: String::new(),
            offset: 0,
            clsid: None,
        };
        let table = ComponentTable::synthetic(vec![Component {
            file_name: "MSWINSCK.OCX".to_owned(),
            library: "MSWinsockLib.Winsock".to_owned(),
            name: "Winsock".to_owned(),
            guid_offset: Off::new(0),
            guid_length: 72,
            guid_text: Some("2c49f800-c2dd-11cf-9ad6-0080c7e7b78d".to_owned()),
        }]);

        let reason = join_component(&mut control, &table);
        assert!(control.clsid.is_none());
        assert!(reason.is_some());
    }

    // --- Task 3: the fixed OCX header and the opaque blob statement -------

    /// The four bytes VB writes in the file, little endian, for
    /// [`OCX_SIGNATURE`]. `STRUCTURES.md` section 8.7's own doc comment
    /// names the byte order; this test proves the constant matches it.
    #[test]
    fn ocx_signature_is_0x12344321_stored_little_endian_as_21_43_34_12() {
        assert_eq!(OCX_SIGNATURE, 0x1234_4321);
        assert_eq!(OCX_SIGNATURE.to_le_bytes(), [0x21, 0x43, 0x34, 0x12]);
    }

    /// Builds a synthetic fixed header: the four byte signature, the
    /// reserved field, `_ExtentX`, `_ExtentY`, a second reserved field, and
    /// `_Version`, the exact 24 byte layout `STRUCTURES.md` section 8.7
    /// gives.
    fn ocx_header_bytes(reserved: u32, extent_x: i32, extent_y: i32, version: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&OCX_SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&reserved.to_le_bytes());
        bytes.extend_from_slice(&extent_x.to_le_bytes());
        bytes.extend_from_slice(&extent_y.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes()); // the second reserved field
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes
    }

    #[test]
    fn a_blob_holding_the_signature_gives_a_header_with_the_three_recoverable_fields() {
        let bytes = ocx_header_bytes(8, 741, 741, 393_216);
        let region = Region::new(&bytes, Off::new(0));
        let block_end = u32::try_from(bytes.len()).unwrap();
        let (header, opaque, defects) = read_ocx_blob(&region, 0, block_end);
        let header = header.expect("synthetic fixture: the signature must be found");
        assert_eq!(header.extent_x, 741);
        assert_eq!(header.extent_y, 741);
        assert_eq!(header.version, 393_216);
        assert!(defects.is_empty(), "{defects:?}");
        assert_eq!(
            opaque.length, 0,
            "the fixture holds nothing past the header"
        );
    }

    #[test]
    fn a_blob_with_no_signature_gives_no_header_and_no_defect() {
        let bytes = vec![0xAA_u8; 40];
        let region = Region::new(&bytes, Off::new(0));
        let (header, opaque, defects) = read_ocx_blob(&region, 0, 40);
        assert!(header.is_none());
        assert!(defects.is_empty(), "{defects:?}");
        assert_eq!(opaque.length, 40);
    }

    #[test]
    fn a_reserved_field_other_than_eight_gives_a_defect_and_the_other_three_fields_still_read() {
        let bytes = ocx_header_bytes(99, 100, 200, 300);
        let region = Region::new(&bytes, Off::new(0));
        let block_end = u32::try_from(bytes.len()).unwrap();
        let (header, _opaque, defects) = read_ocx_blob(&region, 0, block_end);
        let header = header.expect("the header still reads past an unexpected reserved value");
        assert_eq!(header.extent_x, 100);
        assert_eq!(header.extent_y, 200);
        assert_eq!(header.version, 300);
        assert_eq!(defects.len(), 1);
        assert!(matches!(
            defects[0].kind,
            DefectKind::OcxReservedFieldUnexpected { value: 99, .. }
        ));
    }

    /// Synthetic fixture: the signature sits three bytes before the block's
    /// own end, too close for the full 24 byte header to fit. The scan must
    /// not read past `block_end`, so this gives no header, not a header read
    /// from bytes belonging to whatever follows the block.
    #[test]
    fn a_signature_three_bytes_before_the_block_end_reads_no_byte_past_the_bound() {
        // The region's own physical buffer is 50 bytes, long enough to hold
        // a complete, real signature and header at `sig_at`. `block_end` is
        // 20, three bytes past `sig_at`: a caller that trusted the region's
        // own physical length instead of `block_end` would read a
        // plausible-looking header from bytes that, logically, belong to
        // whatever follows this block.
        let mut bytes = vec![0xAA_u8; 50];
        let sig_at = 17;
        let header_bytes = ocx_header_bytes(8, 1, 2, 3);
        bytes[sig_at..sig_at + header_bytes.len()].copy_from_slice(&header_bytes);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _opaque, defects) = read_ocx_blob(&region, 0, 20);
        assert!(
            header.is_none(),
            "synthetic fixture: too little room for the full header must not read past the bound"
        );
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn a_blob_of_zero_bytes_gives_no_header_and_one_opaque_report_of_length_zero() {
        let bytes: Vec<u8> = Vec::new();
        let region = Region::new(&bytes, Off::new(0x5000));
        let (header, opaque, defects) = read_ocx_blob(&region, 0, 0);
        assert!(header.is_none());
        assert!(defects.is_empty(), "{defects:?}");
        assert_eq!(opaque.length, 0);
        assert_eq!(opaque.offset, 0x5000);
    }

    #[test]
    fn the_opaque_message_names_the_type_library_and_the_repositorys_inability_to_hold_it() {
        let opaque = super::OpaqueBlob {
            offset: 0x10,
            length: 4,
        };
        let message = opaque.message();
        assert!(message.to_ascii_lowercase().find("type library").is_some());
        assert!(message.find("0x10").is_some(), "{message}");
    }

    /// A gap of unreadable bytes before the signature (this session
    /// measured exactly this shape in the real corpus: nine bytes between
    /// `wsPop`'s own class name and its fixed header) is scanned over, not
    /// skipped by assuming the header sits at `blob_start`.
    #[test]
    fn the_scan_finds_a_signature_that_does_not_sit_at_blob_start() {
        let mut bytes = vec![0xBB_u8; 9]; // an unrelated gap, matching the corpus shape
        bytes.extend_from_slice(&ocx_header_bytes(8, 1, 2, 3));
        let region = Region::new(&bytes, Off::new(0));
        let block_end = u32::try_from(bytes.len()).unwrap();
        let (header, opaque, defects) = read_ocx_blob(&region, 0, block_end);
        let header = header.expect("the scan must find a signature past the gap");
        assert_eq!(header.extent_x, 1);
        assert_eq!(header.extent_y, 2);
        assert_eq!(header.version, 3);
        assert!(defects.is_empty(), "{defects:?}");
        assert_eq!(
            opaque.length, 9,
            "the 9 byte gap before the header is still reported opaque; the header's own 24 \
             bytes are not"
        );
    }

    /// `SK-Winsock-Sample__VB6`'s own `.frm` declares `_ExtentX = 741`,
    /// `_ExtentY = 741` and `_Version = 393216` on `wsPop`. This session
    /// measured the fixed header's own real position: 9 bytes after the
    /// class name's own declared end, at file offset `0x1d96`.
    /// `[VERIFIED: local]`
    #[test]
    fn the_winsock_sample_gives_its_real_ocx_header_with_non_zero_extents() {
        let block = wspop_block();
        let (header, _) = read_control_header(&block);
        let (control, consumed, defects) = read_external_control(&block, &header);
        assert!(defects.is_empty(), "{defects:?}");

        let blob_start = header.header_len().checked_add(consumed).unwrap();
        let length = block.u16_le(Off::new(0)).unwrap();
        let block_end = u32::from(length) - 1;

        let (ocx_header, _opaque, defects) = read_ocx_blob(&block, blob_start, block_end);
        assert!(defects.is_empty(), "{defects:?}");
        let ocx_header = ocx_header.expect("the real Winsock control carries the fixed header");
        assert_eq!(ocx_header.extent_x, 741);
        assert_eq!(ocx_header.extent_y, 741);
        assert_eq!(ocx_header.version, 393_216);
        assert_ne!(ocx_header.extent_x, 0);
        assert_ne!(ocx_header.extent_y, 0);
        // control.class_name still available for readers of this test.
        assert_eq!(control.class_name, "MSWinsockLib.Winsock");
    }
}
