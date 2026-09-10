//! The control block header: `ControlKind`, `ControlHeader`, the control
//! type, the control name, and the control array `Index`.
//!
//! Plan 03-04 fills this module. It serves FRM-01 and FRM-02. This first
//! commit reads one control block's own header. Reading the tree the scope
//! bytes describe — the walk over every block, and the `Tiling` gate that
//! refuses a mis-nested one — is this same plan's next commit.
//!
//! # The control block header (`STRUCTURES.md` section 8.4)
//!
//! Two layouts, selected by the flags byte at block offset `0x03`. `0x80`
//! selects the array layout; any other value selects the non-array layout.
//! [`read_control_header`] reads either shape into one [`ControlHeader`],
//! following the window-before-fields discipline `vb/object.rs` documents:
//! the caller narrows the block to its own bounded window before any field
//! inside it is read.

use crate::error::{Defect, DefectKind, Site};
use crate::read::region::{Off, Region};

/// The flags byte value, at block offset `0x03`, that selects the array
/// control-block layout.
pub const ARRAY_FLAG: u8 = 0x80;

/// The block offset of the control array `Index` field, in the array
/// layout. `STRUCTURES.md` gap 11, closed this plan: 30 array elements
/// across 2 files, values 0 through 24, zero disagreements against the
/// `.frm` source. See the closure section this plan adds to
/// `STRUCTURES.md`.
pub const INDEX_AT: u32 = 0x05;

/// A recognised control type, or a value `STRUCTURES.md` section 8.4.1's
/// table does not cover.
///
/// Follows `vb/classify.rs`'s "carry raw, never guess" shape: a name for a
/// known value, the raw value for an unknown one, and no refusal either way.
/// Values 12, 14, 15, 21, 25 through 36, and 39 are unassigned in section
/// 8.4.1 and land in [`ControlKind::Unknown`]. This match is never widened
/// beyond what that section lists.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ControlKind {
    /// `cType` 0.
    PictureBox,
    /// `cType` 1.
    Label,
    /// `cType` 2.
    TextBox,
    /// `cType` 3.
    Frame,
    /// `cType` 4.
    CommandButton,
    /// `cType` 5.
    CheckBox,
    /// `cType` 6.
    OptionButton,
    /// `cType` 7.
    ComboBox,
    /// `cType` 8.
    ListBox,
    /// `cType` 9.
    HScrollBar,
    /// `cType` 10.
    VScrollBar,
    /// `cType` 11.
    Timer,
    /// `cType` 13.
    Form,
    /// `cType` 16.
    DriveListBox,
    /// `cType` 17.
    DirListBox,
    /// `cType` 18.
    FileListBox,
    /// `cType` 19.
    Menu,
    /// `cType` 20.
    MdiForm,
    /// `cType` 22.
    Shape,
    /// `cType` 23.
    Line,
    /// `cType` 24.
    Image,
    /// `cType` 37.
    Data,
    /// `cType` 38.
    Ole,
    /// `cType` 40.
    UserControl,
    /// `cType` 41.
    PropertyPage,
    /// `cType` 42.
    UserDocument,
    /// `cType` 255: an external (OCX) control. `vb/ocx.rs`, plan 03-08, owns
    /// reading its class name and CLSID.
    External,
    /// A value the match does not cover, carried raw for the report.
    Unknown(u8),
}

/// Classifies one control's raw `cType`.
#[must_use]
pub const fn classify_control_type(c_type: u8) -> ControlKind {
    match c_type {
        0 => ControlKind::PictureBox,
        1 => ControlKind::Label,
        2 => ControlKind::TextBox,
        3 => ControlKind::Frame,
        4 => ControlKind::CommandButton,
        5 => ControlKind::CheckBox,
        6 => ControlKind::OptionButton,
        7 => ControlKind::ComboBox,
        8 => ControlKind::ListBox,
        9 => ControlKind::HScrollBar,
        10 => ControlKind::VScrollBar,
        11 => ControlKind::Timer,
        13 => ControlKind::Form,
        16 => ControlKind::DriveListBox,
        17 => ControlKind::DirListBox,
        18 => ControlKind::FileListBox,
        19 => ControlKind::Menu,
        20 => ControlKind::MdiForm,
        22 => ControlKind::Shape,
        23 => ControlKind::Line,
        24 => ControlKind::Image,
        37 => ControlKind::Data,
        38 => ControlKind::Ole,
        40 => ControlKind::UserControl,
        41 => ControlKind::PropertyPage,
        42 => ControlKind::UserDocument,
        255 => ControlKind::External,
        other => ControlKind::Unknown(other),
    }
}

/// One control block's header: its type, its name, and, when it is an array
/// element, its index.
///
/// `c_id` is `0` for an array-layout control. `STRUCTURES.md`'s own
/// array-header table places `cId` at offset `0x05` there, but this plan's
/// own measurement (see the module doc comment, and the `STRUCTURES.md`
/// closure section this plan adds) shows that offset holds the array
/// `Index`, not `cId`. Section 8.4 names no other location for `cId` in the
/// array layout, so this reader does not invent one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlHeader {
    /// The control's ID, used to link it to its event handlers.
    /// `0` for an array-layout control; see the struct doc comment.
    pub c_id: u8,
    /// The raw control type code. Carried raw: [`classify_control_type`]
    /// names it, and this reader refuses no control on the strength of an
    /// unrecognised value.
    pub c_type: u8,
    /// The control's name, read as length-prefixed bytes. Each byte becomes
    /// its own Latin-1 code point; `String::from_utf8_lossy` is never used,
    /// because a byte in `0x80` to `0xFF` would become the replacement
    /// character and the name would be lost.
    pub name: String,
    /// The control array `Index`, when this block uses the array layout.
    /// `None` for a non-array control.
    pub array_index: Option<u16>,
    header_len: u32,
}

impl ControlHeader {
    /// The header's own byte length, from the block's own offset `0x00`
    /// through the `cType` byte inclusive. The property stream that follows
    /// a control's header starts here.
    #[must_use]
    pub const fn header_len(&self) -> u32 {
        self.header_len
    }
}

/// Reads the control array `Index` field: the two-byte little-endian value
/// at [`INDEX_AT`].
///
/// Read defensively as two bytes, per `03-RESEARCH.md` assumption A4: the
/// corpus cannot tell a one-byte field at `0x05` from the low byte of a
/// two-byte field spanning `0x05` and `0x06`, because no corpus index
/// exceeds 24. [`read_control_header`] gives a [`Defect`] when the high byte
/// is non-zero; this function only reads the raw value.
#[must_use]
pub fn read_array_index(block: &Region<'_>) -> Option<u16> {
    block.u16_le(Off::new(INDEX_AT))
}

/// Reads one control block's header from its own bounded window.
///
/// `block` is the control block's own window, starting at its `Length`
/// field (offset `0x00`) and bounded to at least the block's own declared
/// span. The caller (this module's [`walk`]) is responsible for reading
/// `Length` itself and building that window; this function reads only the
/// fields the header itself carries.
///
/// Gives `(ControlHeader, Vec<Defect>)`. A [`Defect`] never loses the
/// control: a name that cannot be read gives an empty name and the header's
/// other fields still stand, following `vb/object.rs::read_name`'s own
/// contract.
#[must_use]
pub fn read_control_header(block: &Region<'_>) -> (ControlHeader, Vec<Defect>) {
    let offset = block.file_offset(Off::new(0)).map_or(0, Off::get);
    let mut defects = Vec::new();

    let flags = block.u8(Off::new(0x03));
    if flags == Some(ARRAY_FLAG) {
        let array_index = read_array_index(block);
        if let Some(index) = array_index
            && index > 0xFF
        {
            let high = u8::try_from(index >> 8).unwrap_or(0);
            defects.push(Defect {
                site: Site {
                    offset,
                    rva: None,
                    structure: "ControlHeader",
                    field: "Index",
                },
                kind: DefectKind::IndexHighByteSet { offset, high },
            });
        }

        let name_len = block.u16_le(Off::new(0x07)).unwrap_or(0);
        let (name, name_defect) = read_name(block, offset, 0x09, name_len);
        if let Some(defect) = name_defect {
            defects.push(defect);
        }

        let c_type_offset = 0x0A_u32.saturating_add(u32::from(name_len));
        let c_type = block.u8(Off::new(c_type_offset)).unwrap_or(0);
        let header_len = c_type_offset.saturating_add(1);

        (
            ControlHeader {
                c_id: 0,
                c_type,
                name,
                array_index,
                header_len,
            },
            defects,
        )
    } else {
        let c_id = block.u8(Off::new(0x04)).unwrap_or(0);
        let name_len = block.u16_le(Off::new(0x05)).unwrap_or(0);
        let (name, name_defect) = read_name(block, offset, 0x07, name_len);
        if let Some(defect) = name_defect {
            defects.push(defect);
        }

        let c_type_offset = 0x08_u32.saturating_add(u32::from(name_len));
        let c_type = block.u8(Off::new(c_type_offset)).unwrap_or(0);
        let header_len = c_type_offset.saturating_add(1);

        (
            ControlHeader {
                c_id,
                c_type,
                name,
                array_index: None,
                header_len,
            },
            defects,
        )
    }
}

/// Reads a control's length-prefixed name.
///
/// A declared length of `0` gives an empty name and a [`Defect`] naming the
/// block's own byte offset; the header still gives its `cType`. A declared
/// length larger than the remaining block also gives an empty name and a
/// [`Defect`]; no allocation is sized from the declared length before this
/// check, because [`Region::take`] itself refuses the read rather than
/// allocating first.
fn read_name(
    block: &Region<'_>,
    block_offset: u32,
    name_start: u32,
    name_len: u16,
) -> (String, Option<Defect>) {
    if name_len == 0 {
        let defect = Defect {
            site: Site {
                offset: block_offset,
                rva: None,
                structure: "ControlHeader",
                field: "name",
            },
            kind: DefectKind::EmptyName {
                offset: block_offset,
            },
        };
        return (String::new(), Some(defect));
    }

    match block.take(Off::new(name_start), u32::from(name_len)) {
        // Each byte becomes its own Latin-1 code point, the rule
        // `vb/object.rs::read_name` and `vb/gui.rs::FormStream::name` both
        // use. `String::from_utf8_lossy` is never used here.
        Some(bytes) => (bytes.iter().copied().map(char::from).collect(), None),
        None => {
            let max = block.len().saturating_sub(name_start);
            let defect = Defect {
                site: Site {
                    offset: block_offset,
                    rva: None,
                    structure: "ControlHeader",
                    field: "name",
                },
                kind: DefectKind::ImplausibleCount {
                    offset: block_offset,
                    count: u32::from(name_len),
                    max,
                },
            };
            (String::new(), Some(defect))
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
    use super::{
        ARRAY_FLAG, ControlKind, classify_control_type, read_array_index, read_control_header,
    };
    use crate::error::DefectKind;
    use crate::read::region::{Off, Region};

    // --- Task 1: the control block header --------------------------------

    /// Builds a synthetic array-layout control block matching the exact hex
    /// this session measured for the `TxtF` array's first three elements in
    /// `corpus/vb6-code/Custom-image-filters/Custom_Filters.exe` (the
    /// corpus's real `ExeName32`; the plan's own text names
    /// `CustomFilters.exe`, which this corpus does not hold — the `.vbp`
    /// declares `ExeName32="Custom_Filters.exe"`). This is a literal the
    /// test builds in memory, per `AGENTS.md`: a test builds the state it
    /// needs and does not read it out of a file the author edits.
    ///
    /// Layout: `Length`(2) `unknown`(1) `flags=0x80`(1) `group-const=0x02`(1)
    /// `Index`(2) `name_len=4`(2) `"TxtF"`(4) `unknown`(1) `cType`(1).
    fn txt_f_element(index_low: u8, c_type: u8) -> Vec<u8> {
        vec![
            0x0f, 0x00, // Length = 15 (content is small; this test does not
            // exercise the tiling walk, only the header reader)
            0x00,       // unknown
            ARRAY_FLAG, // flags
            0x02,       // the unexplained group constant
            index_low, 0x00, // Index, low byte then high byte
            0x04, 0x00, // name length = 4
            b'T', b'x', b't', b'F', // name
            0x00, // unknown, the "0x09+n unknown" byte STRUCTURES.md section 8.4 names
            c_type,
        ]
    }

    #[test]
    fn the_three_txt_f_elements_give_index_zero_one_and_two() {
        for (i, expected_index) in [(0u8, 0u16), (1, 1), (2, 2)] {
            let bytes = txt_f_element(i, 2);
            let region = Region::new(&bytes, Off::new(0));
            let (header, defects) = read_control_header(&region);
            assert_eq!(header.array_index, Some(expected_index));
            assert_eq!(header.name, "TxtF");
            assert_eq!(header.c_type, 2);
            assert!(defects.is_empty(), "{defects:?}");
        }
    }

    #[test]
    fn read_array_index_gives_none_when_the_flags_byte_is_not_0x80() {
        let mut bytes = txt_f_element(0, 2);
        bytes[3] = 0x00; // flags, not 0x80
        let region = Region::new(&bytes, Off::new(0));
        assert_eq!(read_array_index(&region), Some(0));
        // The header reader itself takes the non-array branch, and gives no
        // array_index, regardless of what bytes happen to sit at INDEX_AT.
        let (header, _) = read_control_header(&region);
        assert_eq!(header.array_index, None);
    }

    #[test]
    fn a_two_byte_array_index_with_a_non_zero_high_byte_gives_a_defect() {
        let mut bytes = txt_f_element(0, 2);
        bytes[5] = 0x01; // Index low byte
        bytes[6] = 0x01; // Index high byte, non-zero
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.array_index, Some(0x0101));
        assert_eq!(defects.len(), 1);
        assert!(matches!(
            defects[0].kind,
            DefectKind::IndexHighByteSet { high: 0x01, .. }
        ));
    }

    #[test]
    fn a_non_array_header_reads_c_id_name_and_c_type() {
        // Length(2) unknown(1) flags=0(1) cId(1) name_len(2) name(n) unknown(1) cType(1)
        let mut bytes = vec![0x00, 0x00, 0x00, 0x00, 0x07, 0x03, 0x00];
        bytes.extend_from_slice(b"Cmd");
        bytes.push(0x00); // unknown, per STRUCTURES.md "0x07+n unknown"
        bytes.push(4); // cType 4 = CommandButton
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.c_id, 0x07);
        assert_eq!(header.name, "Cmd");
        assert_eq!(header.c_type, 4);
        assert_eq!(header.array_index, None);
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn a_declared_name_length_of_zero_gives_an_empty_name_and_a_defect_and_still_gives_c_type() {
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 13];
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.name, "");
        assert_eq!(header.c_type, 13);
        assert_eq!(defects.len(), 1);
        assert!(matches!(defects[0].kind, DefectKind::EmptyName { .. }));
    }

    #[test]
    fn a_declared_name_length_larger_than_the_remaining_block_gives_an_empty_name_and_a_defect() {
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x07, 0xFF, 0xFF];
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.name, "");
        assert_eq!(defects.len(), 1);
        assert!(matches!(
            defects[0].kind,
            DefectKind::ImplausibleCount { count: 0xFFFF, .. }
        ));
    }

    #[test]
    fn a_name_byte_of_0xa9_becomes_its_own_latin1_code_point() {
        // Length(2) unknown(1) flags=0(1) cId(1) name_len=1(2) name(1) unknown(1) cType(1)
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0xA9, 0x00, 1];
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        assert_eq!(header.name.chars().next(), Some('\u{A9}'));
    }

    #[test]
    fn c_type_39_is_carried_raw_and_refuses_no_control() {
        // Length(2) unknown(1) flags=0(1) cId(1) name_len=1(2) name(1) unknown(1) cType(1)
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, b'X', 0x00, 39];
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.c_type, 39);
        assert!(defects.is_empty(), "{defects:?}");
        assert_eq!(classify_control_type(39), ControlKind::Unknown(39));
    }

    #[test]
    fn every_named_c_type_classifies_by_name() {
        assert_eq!(classify_control_type(13), ControlKind::Form);
        assert_eq!(classify_control_type(4), ControlKind::CommandButton);
        assert_eq!(classify_control_type(1), ControlKind::Label);
        assert_eq!(classify_control_type(19), ControlKind::Menu);
        assert_eq!(classify_control_type(255), ControlKind::External);
    }

    #[test]
    fn a_repeated_index_in_a_synthetic_two_element_array_is_reported_twice_not_deduplicated() {
        // Synthetic fixture: two array elements that both declare Index=0.
        // The corpus holds no such case (per the plan's own acceptance
        // criteria); this fixture is built here, and its own assertion
        // message names it as synthetic.
        let elem_a = txt_f_element(0, 2);
        let elem_b = txt_f_element(0, 2);
        let region_a = Region::new(&elem_a, Off::new(0));
        let region_b = Region::new(&elem_b, Off::new(0));
        let (header_a, _) = read_control_header(&region_a);
        let (header_b, _) = read_control_header(&region_b);
        assert_eq!(
            header_a.array_index,
            Some(0),
            "synthetic fixture: element A"
        );
        assert_eq!(
            header_b.array_index,
            Some(0),
            "synthetic fixture: element B repeats Index=0, and the header reader \
             gives it back as a second, distinct control rather than merging it"
        );
    }

    #[test]
    fn the_array_index_offset_is_a_named_constant_set_to_0x05() {
        assert_eq!(super::INDEX_AT, 0x05);
    }
}
