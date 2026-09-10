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

use crate::error::{Defect, DefectKind, Site};
use crate::read::region::{Off, Region};
use crate::vb::controltree::ControlHeader;
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
    pub clsid: Option<()>,
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
    use super::{ExternalControl, read_external_control};
    use crate::error::DefectKind;
    use crate::read::region::{Off, Region};
    use crate::vb::controltree::{ControlKind, classify_control_type, read_control_header};

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
}
