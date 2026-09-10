//! Typed property payloads: the property loop, the position block escape,
//! the `Font` block, and the other special opcodes `03-RESEARCH.md` section
//! 8.5 names.
//!
//! Plan 03-06 fills this module. It serves FRM-03.
//!
//! # The loop bound
//!
//! `03-RESEARCH.md` section 8.3 (SVBD's own account) says the property loop
//! runs "while the cursor is below `blockStart + Length - 2`". Plan 03-04
//! measured the real bound directly against `Grayscale.exe` and found this
//! formula does not reconcile: the scope separator that follows a control's
//! own properties starts at `blockStart + Length - 1`, not `Length - 2`
//! (`vb/controltree.rs`'s own `content = length.checked_sub(1)`, cited in
//! its module doc comment). The bytes a control's own header and properties
//! occupy are exactly what precedes that separator, so this module uses the
//! same, corpus-measured bound: the loop runs while the cursor is below
//! `Length - 1`, not `Length - 2`. Using the stale `Length - 2` bound here
//! would leave the last property byte unread and misalign against
//! `controltree.rs`'s own tiling accounting, which already treats `Length -
//! 1` bytes as the block's own content.
//!
//! # Never guess a width
//!
//! An opcode with no table entry stops the loop. It never advances the
//! cursor by a guessed width: a wrong guess shifts every property after it
//! in the same block, and nothing would report it. [`PropertyValue::Undecoded`]
//! carries the opcode, the byte offset, and the control type, and names
//! `--opcode-table` as the way to supply a table that decodes it.

use crate::error::{Defect, DefectKind, Site};
use crate::read::region::{Off, Region};
use crate::vb::controltree::{ControlHeader, classify_control_type};
use crate::vb::opcodes::{OpcodeTable, PayloadType};
use crate::vb::vbstr::{StrEncoding, VbStr};

/// One property's value, as [`walk_properties`] decoded it.
///
/// One variant per payload shape [`OpcodeTable`] can name, plus one
/// [`PropertyValue::Undecoded`] variant for an opcode this repository
/// cannot decode: either because no table entry names it, or because the
/// table names a payload shape this module has no reader for (a `Picture`
/// blob; `frx.rs`, plan 03-07, owns that).
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    /// A `Byte` payload, 1 byte.
    Byte {
        /// The property this opcode names.
        name: String,
        /// The raw byte.
        value: u8,
    },
    /// A `Boolean` payload, 2 bytes, read as a signed 16 bit value so the
    /// two values a `.frm` file writes, `-1` and `0`, come through exactly.
    Boolean {
        /// The property this opcode names.
        name: String,
        /// `-1` or `0`, the form a `.frm` file writes.
        value: i16,
    },
    /// An `Integer` payload, 2 bytes.
    Integer {
        /// The property this opcode names.
        name: String,
        /// The signed 16 bit value.
        value: i16,
    },
    /// A `Long` payload, 4 bytes.
    Long {
        /// The property this opcode names.
        name: String,
        /// The signed 32 bit value.
        value: i32,
    },
    /// A `Single` payload, 4 bytes.
    Single {
        /// The property this opcode names.
        name: String,
        /// The IEEE-754 single precision value, read bit for bit from the
        /// four bytes with no rounding.
        value: f32,
    },
    /// A `String` payload, read through [`VbStr`]. The cursor advances by
    /// [`VbStr::declared_end`], never by a length this module computes
    /// itself.
    Text {
        /// The property this opcode names.
        name: String,
        /// The decoded text. Empty when [`VbStr::read`] could not land it
        /// under either encoding; the [`Defect`] it gave is still in the
        /// list [`walk_properties`] returns.
        value: String,
    },
    /// A `Position` payload: four coordinates, read through
    /// [`read_position_block`].
    Position {
        /// The property this opcode names.
        name: String,
        /// The four coordinates, in the short or the long form.
        value: PositionBlock,
    },
    /// An opcode this repository does not decode.
    ///
    /// Two distinct causes share this one variant: no table entry names the
    /// opcode at all, or the table names a payload shape this module has no
    /// reader for (`Picture`, a resource blob; see the module doc comment).
    /// Either way the loop stops here rather than guessing a width. The
    /// message names the opcode, the byte offset, the control type, and
    /// `--opcode-table` as the way to supply a table.
    Undecoded {
        /// The opcode byte this repository could not name a decoder for.
        opcode: u8,
        /// The absolute file offset of the opcode byte.
        offset: u32,
        /// The control type this opcode belongs to, from
        /// [`classify_control_type`]'s own `Debug` rendering.
        control_type: String,
        /// How many bytes of the block, from this opcode's own byte
        /// onward, were left unread.
        bytes_not_read: u32,
    },
}

impl PropertyValue {
    /// Renders the honest-gap message `03-RESEARCH.md` Pattern 4 states,
    /// for an [`PropertyValue::Undecoded`] value. `None` for every other
    /// variant, which already carries its own name and value.
    #[must_use]
    pub fn undecoded_message(&self) -> Option<String> {
        match self {
            Self::Undecoded {
                opcode,
                offset,
                control_type,
                ..
            } => Some(format!(
                "Property opcode {opcode} at offset {offset:#x}: value not decoded for \
                 control type {control_type}. Run with --opcode-table to supply a table \
                 naming this opcode."
            )),
            Self::Byte { .. }
            | Self::Boolean { .. }
            | Self::Integer { .. }
            | Self::Long { .. }
            | Self::Single { .. }
            | Self::Text { .. }
            | Self::Position { .. } => None,
        }
    }
}

/// The property stream one control block's own bytes decode into.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyStream {
    /// Every property this walk recovered, in stream order. Two runs over
    /// the same bytes give the same order.
    pub properties: Vec<PropertyValue>,
    /// The absolute file offset the loop stopped at: the block's own end on
    /// a clean finish, or the byte offset a [`PropertyValue::Undecoded`] or
    /// a payload overrun stopped it at.
    pub stopped_at: u32,
}

/// Computes the end of a payload of `width` bytes starting at
/// `payload_start`, refusing when it would run past `block_end`.
///
/// `block_end` is the block's own bound (`Length - 1`, per the module doc
/// comment), not the enclosing region's own length: a payload that fits
/// inside the region but runs into the scope separator that follows the
/// block is still refused.
fn ends_within(payload_start: u32, width: u32, block_end: u32) -> Option<u32> {
    let end = payload_start.checked_add(width)?;
    if end > block_end { None } else { Some(end) }
}

/// Builds the [`Defect`] for a payload that would end past the block's own
/// end. Names both positions, per this plan's own acceptance criteria: where
/// the payload would have ended, and where the block itself ends.
fn overrun_defect(offset: u32, payload_end: u32, block_end: u32) -> Defect {
    Defect {
        site: Site {
            offset,
            rva: None,
            structure: "PropertyStream",
            field: "payload",
        },
        kind: DefectKind::ImplausibleCount {
            offset,
            count: payload_end,
            max: block_end,
        },
    }
}

/// Reads a fixed-width payload at `payload_start`. Only called for a
/// [`PayloadType`] whose [`PayloadType::fixed_width`] gives `Some`.
fn read_fixed(
    block: &Region<'_>,
    payload_start: u32,
    payload: PayloadType,
    name: String,
) -> PropertyValue {
    match payload {
        PayloadType::Byte => PropertyValue::Byte {
            name,
            value: block.u8(Off::new(payload_start)).unwrap_or(0),
        },
        PayloadType::Boolean => PropertyValue::Boolean {
            name,
            value: block.i16_le(Off::new(payload_start)).unwrap_or(0),
        },
        PayloadType::Integer => PropertyValue::Integer {
            name,
            value: block.i16_le(Off::new(payload_start)).unwrap_or(0),
        },
        PayloadType::Long => PropertyValue::Long {
            name,
            value: block.i32_le(Off::new(payload_start)).unwrap_or(0),
        },
        PayloadType::Single
        | PayloadType::Text
        | PayloadType::Picture
        | PayloadType::Font
        | PayloadType::Position => {
            // `Single` is the only remaining fixed-width shape; the other
            // four are variable-width and never reach this function.
            let raw = block.u32_le(Off::new(payload_start)).unwrap_or(0);
            PropertyValue::Single {
                name,
                value: f32::from_bits(raw),
            }
        }
    }
}

/// The `Position` payload: four coordinates, `Left`, `Top`, `Width` and
/// `Height`, in an 8 byte short form or a 16 byte long form.
///
/// `STRUCTURES.md` section 8.5.1: the short form is four signed 16 bit
/// values, in that order. When the first of them is `-32768`, the format
/// escapes to four signed 32 bit values instead, for coordinates outside the
/// signed 16 bit range. `PayloadType::Position`'s own doc comment (plan
/// 03-02, already committed) already states the total payload width as "8
/// bytes, or 16 bytes when the first `i16` is `-32768`": 16 bytes total, not
/// 18. `03-RESEARCH.md`'s own illustrative code reads the four `i32` values
/// starting two bytes after the escape marker (18 bytes total: 2 for the
/// marker, plus 16 for four `i32` values) while also returning `16` as its
/// own consumed count, which does not reconcile against its own read. No
/// corpus file exercises this escape (`03-RESEARCH.md`'s own "Flagged
/// assumption", carried into this plan), so there is no corpus evidence to
/// arbitrate between the two readings. This reader keeps the total the
/// already-shipped `PayloadType::fixed_width` doc comment commits to: on the
/// escape, it re-reads the same 16 byte span the short form's own four `i16`
/// values would have occupied, as four `i32` values instead, 16 bytes total,
/// self-consistent and untested by construction against the corpus, the
/// same treatment phase 1 gave the P-code branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionBlock {
    /// Four signed 16 bit values, 8 bytes total.
    Short {
        /// The left coordinate.
        left: i16,
        /// The top coordinate.
        top: i16,
        /// The width.
        width: i16,
        /// The height.
        height: i16,
    },
    /// Four signed 32 bit values, 16 bytes total, used when a coordinate
    /// does not fit in a signed 16 bit value.
    Long {
        /// The left coordinate.
        left: i32,
        /// The top coordinate.
        top: i32,
        /// The width.
        width: i32,
        /// The height.
        height: i32,
    },
}

/// Reads a [`PositionBlock`] at `at`, bounded by `block_end`.
///
/// Gives `(PositionBlock, consumed)`, where `consumed` is `8` or `16`, the
/// caller's own cursor advance: never a constant, always the width this
/// reader itself decided on from the payload's own first two bytes.
///
/// # Errors
///
/// Returns a [`Defect`] naming both positions when the chosen form (8 or 16
/// bytes) would run past `block_end`.
fn read_position_block(
    block: &Region<'_>,
    at: u32,
    block_end: u32,
) -> Result<(PositionBlock, u32), Defect> {
    let offset = block.file_offset(Off::new(at)).map_or(0, Off::get);
    let block_end_offset = block.file_offset(Off::new(block_end)).map_or(0, Off::get);

    // Peek the first signed 16 bit value to decide the form. Bound the peek
    // itself first: a crafted file can put the opcode one or two bytes
    // before the block's own end.
    if ends_within(at, 2, block_end).is_none() {
        let peek_end_offset = block
            .file_offset(Off::new(at.saturating_add(2)))
            .map_or(0, Off::get);
        return Err(overrun_defect(offset, peek_end_offset, block_end_offset));
    }
    let first = block.i16_le(Off::new(at)).unwrap_or(0);
    let escaped = first == i16::MIN;
    let width: u32 = if escaped { 16 } else { 8 };

    if ends_within(at, width, block_end).is_none() {
        let payload_end_offset = block
            .file_offset(Off::new(at.saturating_add(width)))
            .map_or(0, Off::get);
        return Err(overrun_defect(offset, payload_end_offset, block_end_offset));
    }

    if escaped {
        let left = block.i32_le(Off::new(at)).unwrap_or(0);
        let top = at
            .checked_add(4)
            .and_then(|o| block.i32_le(Off::new(o)))
            .unwrap_or(0);
        let width_value = at
            .checked_add(8)
            .and_then(|o| block.i32_le(Off::new(o)))
            .unwrap_or(0);
        let height = at
            .checked_add(12)
            .and_then(|o| block.i32_le(Off::new(o)))
            .unwrap_or(0);
        Ok((
            PositionBlock::Long {
                left,
                top,
                width: width_value,
                height,
            },
            16,
        ))
    } else {
        let top = at
            .checked_add(2)
            .and_then(|o| block.i16_le(Off::new(o)))
            .unwrap_or(0);
        let width_value = at
            .checked_add(4)
            .and_then(|o| block.i16_le(Off::new(o)))
            .unwrap_or(0);
        let height = at
            .checked_add(6)
            .and_then(|o| block.i16_le(Off::new(o)))
            .unwrap_or(0);
        Ok((
            PositionBlock::Short {
                left: first,
                top,
                width: width_value,
                height,
            },
            8,
        ))
    }
}

/// Walks one control block's own property stream.
///
/// `block` is the control block's own bounded window, the same
/// `Length + 2`-byte region [`crate::vb::controltree::read_control_header`]
/// reads its header from. `header` is that block's own [`ControlHeader`],
/// which gives [`ControlHeader::header_len`], the byte offset the property
/// stream begins at. `table` resolves each opcode to a name and a payload
/// shape; a resolved [`PayloadType::Position`] or [`PayloadType::Font`] or
/// [`PayloadType::Picture`] entry is not yet decoded by this reader (see the
/// module doc comment): it becomes a [`PropertyValue::Undecoded`] and stops
/// the loop, the same honest treatment a genuinely unnamed opcode gets.
///
/// Gives `(PropertyStream, Vec<Defect>)`. A payload that would end past the
/// block's own end is never truncated to fit: it gives a [`Defect`] and
/// stops the loop, and no value is reported for it.
#[must_use]
pub fn walk_properties(
    block: &Region<'_>,
    header: &ControlHeader,
    table: &OpcodeTable,
) -> (PropertyStream, Vec<Defect>) {
    let mut properties = Vec::new();
    let mut defects = Vec::new();

    let length = block.u16_le(Off::new(0)).unwrap_or(0);
    let block_end = u32::from(length).saturating_sub(1);
    let control_type_name = format!("{:?}", classify_control_type(header.c_type));

    let mut cursor = header.header_len();

    while cursor < block_end {
        let opcode_offset = block.file_offset(Off::new(cursor)).map_or(0, Off::get);
        let Some(opcode) = block.u8(Off::new(cursor)) else {
            break;
        };
        let Some(payload_start) = cursor.checked_add(1) else {
            break;
        };

        let Some(entry) = table.lookup(header.c_type, opcode) else {
            let bytes_not_read = block_end.saturating_sub(cursor);
            properties.push(PropertyValue::Undecoded {
                opcode,
                offset: opcode_offset,
                control_type: control_type_name.clone(),
                bytes_not_read,
            });
            cursor = block_end;
            break;
        };

        match entry.payload.fixed_width() {
            Some(width) => {
                let Some(payload_end) = ends_within(payload_start, width, block_end) else {
                    let payload_end_offset = block
                        .file_offset(Off::new(payload_start.saturating_add(width)))
                        .map_or(0, Off::get);
                    let block_end_offset =
                        block.file_offset(Off::new(block_end)).map_or(0, Off::get);
                    defects.push(overrun_defect(
                        opcode_offset,
                        payload_end_offset,
                        block_end_offset,
                    ));
                    cursor = block_end;
                    break;
                };
                properties.push(read_fixed(
                    block,
                    payload_start,
                    entry.payload,
                    entry.name.clone(),
                ));
                cursor = payload_end;
            }
            None => match entry.payload {
                PayloadType::Text => {
                    let (s, defect) =
                        VbStr::read(block, Off::new(payload_start), StrEncoding::Ascii);
                    if let Some(d) = defect {
                        defects.push(d);
                    }
                    let declared_end = s.declared_end().get();
                    if declared_end > block_end {
                        let payload_end_offset = block
                            .file_offset(Off::new(declared_end))
                            .map_or(0, Off::get);
                        let block_end_offset =
                            block.file_offset(Off::new(block_end)).map_or(0, Off::get);
                        defects.push(overrun_defect(
                            opcode_offset,
                            payload_end_offset,
                            block_end_offset,
                        ));
                        cursor = block_end;
                        break;
                    }
                    properties.push(PropertyValue::Text {
                        name: entry.name.clone(),
                        value: s.text().to_owned(),
                    });
                    cursor = declared_end;
                }
                PayloadType::Position => match read_position_block(block, payload_start, block_end)
                {
                    Ok((value, consumed)) => {
                        let Some(new_cursor) = payload_start.checked_add(consumed) else {
                            break;
                        };
                        properties.push(PropertyValue::Position {
                            name: entry.name.clone(),
                            value,
                        });
                        cursor = new_cursor;
                    }
                    Err(defect) => {
                        defects.push(defect);
                        cursor = block_end;
                        break;
                    }
                },
                PayloadType::Font | PayloadType::Picture => {
                    // Not yet decoded by this reader. Task 3 fills in
                    // `PayloadType::Font`. `Picture` (a resource blob)
                    // stays undecoded through this whole plan; `frx.rs`,
                    // plan 03-07, owns blob extraction.
                    let bytes_not_read = block_end.saturating_sub(cursor);
                    properties.push(PropertyValue::Undecoded {
                        opcode,
                        offset: opcode_offset,
                        control_type: control_type_name.clone(),
                        bytes_not_read,
                    });
                    cursor = block_end;
                    break;
                }
                PayloadType::Byte
                | PayloadType::Boolean
                | PayloadType::Integer
                | PayloadType::Long
                | PayloadType::Single => {
                    // `fixed_width` gives `Some` for all five of these;
                    // this arm is unreachable, and it names no behaviour.
                    let bytes_not_read = block_end.saturating_sub(cursor);
                    properties.push(PropertyValue::Undecoded {
                        opcode,
                        offset: opcode_offset,
                        control_type: control_type_name.clone(),
                        bytes_not_read,
                    });
                    cursor = block_end;
                    break;
                }
            },
        }
    }

    let stopped_at = block.file_offset(Off::new(cursor)).map_or(0, Off::get);
    (
        PropertyStream {
            properties,
            stopped_at,
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
    use super::{PositionBlock, PropertyValue, read_position_block, walk_properties};
    use crate::error::DefectKind;
    use crate::read::region::{Off, Region};
    use crate::vb::controltree::read_control_header;
    use crate::vb::opcodes::OpcodeTable;

    /// Builds a synthetic non-array control block: `Length`(2) `unknown`(1)
    /// `flags=0`(1) `cId`(1) `name_len`(2) `name`(n) `unknown`(1) `cType`(1),
    /// followed by `body`, the raw property bytes. `Length` is computed so
    /// the loop bound (`Length - 1`) lands exactly at the end of `body`,
    /// matching this module's own corrected bound (see the module doc
    /// comment).
    fn control_block(name: &str, c_type: u8, body: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0u8, 0u8]; // Length placeholder
        bytes.push(0); // unknown
        bytes.push(0); // flags, non-array
        bytes.push(0); // cId
        let name_len = u16::try_from(name.len()).unwrap();
        bytes.extend_from_slice(&name_len.to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0); // unknown, the "0x07+n unknown" byte
        bytes.push(c_type);
        bytes.extend_from_slice(body);
        // This fixture carries no bytes past its own body: `bytes`' own
        // byte count is exactly the header plus every property byte, with
        // no trailing scope-separator span the way a real corpus block has.
        // The loop bound this module uses is `Length - 1`, and it must land
        // exactly at that byte count, so `Length` is one more than it.
        let total = u16::try_from(bytes.len()).unwrap();
        let length = total + 1;
        bytes[0..2].copy_from_slice(&length.to_le_bytes());
        bytes
    }

    /// A hand-written table naming one Form opcode as `Byte`.
    fn form_byte_table() -> OpcodeTable {
        let text = b"[13]\n10 = { name = \"WindowState\", payload = \"Byte\" }\n";
        OpcodeTable::parse(text).unwrap()
    }

    #[test]
    fn a_control_block_with_zero_properties_gives_an_empty_list_and_no_defect() {
        let bytes = control_block("Cmd", 4, &[]);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let table = OpcodeTable::builtin();
        let (stream, defects) = walk_properties(&region, &header, &table);
        assert!(stream.properties.is_empty());
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn a_byte_payload_ending_exactly_on_the_block_end_is_accepted_and_the_loop_stops_there() {
        // opcode 10 (WindowState, Byte), value 1. This is the only
        // property, so its own end is the block's own end.
        let bytes = control_block("Frm1", 13, &[10, 1]);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let table = form_byte_table();
        let (stream, defects) = walk_properties(&region, &header, &table);
        assert_eq!(stream.properties.len(), 1);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Byte { name, value: 1 } if name == "WindowState"
        ));
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn a_payload_that_would_end_one_byte_past_the_block_end_gives_a_defect_and_no_value() {
        // The block declares only 1 byte of value after the opcode, but a
        // Byte payload needs exactly 1 byte, so shrink the declared Length
        // by 1 to make the payload end one byte past the block's own end.
        let mut bytes = control_block("Frm1", 13, &[10, 1]);
        let shrunk = u16::from_le_bytes([bytes[0], bytes[1]]) - 1;
        bytes[0..2].copy_from_slice(&shrunk.to_le_bytes());
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let table = form_byte_table();
        let (stream, defects) = walk_properties(&region, &header, &table);
        assert!(stream.properties.is_empty(), "{:?}", stream.properties);
        assert_eq!(defects.len(), 1);
        let message = format!("{}", defects[0].kind);
        assert!(matches!(
            defects[0].kind,
            DefectKind::ImplausibleCount { .. }
        ));
        // Both positions: the offset the payload would end at (past the
        // block), and the block's own end.
        assert!(message.contains("0x"), "{message}");
    }

    #[test]
    fn an_opcode_with_no_table_entry_gives_undecoded_and_stops_the_loop_naming_offset_and_bytes_not_read()
     {
        let bytes = control_block("Frm1", 13, &[99, 1, 2, 3, 4, 5]);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let table = OpcodeTable::builtin(); // holds no entry for Form opcode 99 here
        let (stream, defects) = walk_properties(&region, &header, &table);
        assert_eq!(stream.properties.len(), 1);
        match &stream.properties[0] {
            PropertyValue::Undecoded {
                opcode,
                offset,
                control_type,
                bytes_not_read,
            } => {
                assert_eq!(*opcode, 99);
                // "Frm1" is 4 bytes, so the header is 13 bytes (9 + 4), and
                // the opcode sits at that same byte offset in this region,
                // whose base is 0.
                assert_eq!(*offset, 13);
                assert_eq!(control_type, "Form");
                assert!(*bytes_not_read > 0);
            }
            other => panic!("expected Undecoded, got {other:?}"),
        }
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn an_integer_a_long_and_a_single_payload_each_read_their_own_declared_width() {
        let text = concat!(
            "[4]\n",
            "1 = { name = \"AnInt\", payload = \"Integer\" }\n",
            "2 = { name = \"ALong\", payload = \"Long\" }\n",
            "3 = { name = \"ASingle\", payload = \"Single\" }\n",
        );
        let table = OpcodeTable::parse(text.as_bytes()).unwrap();

        let mut body = vec![1u8];
        body.extend_from_slice(&(-1000_i16).to_le_bytes());
        body.push(2);
        body.extend_from_slice(&(-70_000_i32).to_le_bytes());
        body.push(3);
        body.extend_from_slice(&1.5_f32.to_le_bytes());

        let bytes = control_block("Cmd", 4, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table);
        assert_eq!(stream.properties.len(), 3, "{:?}", stream.properties);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Integer { value: -1000, .. }
        ));
        assert!(matches!(
            &stream.properties[1],
            PropertyValue::Long { value: -70_000, .. }
        ));
        assert!(matches!(
            &stream.properties[2],
            PropertyValue::Single { value, .. } if (*value - 1.5).abs() < f32::EPSILON
        ));
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn a_position_typed_entry_now_decodes_through_read_position_block() {
        // CommandButton opcode 4 is `Position` in the builtin subset.
        // Task 1 had no reader for it (see this test's own former name and
        // body, updated once Task 2 added `read_position_block`); this is
        // the same evolution 03-04's own SUMMARY documents for a test that
        // depended on later-task functionality.
        let bytes = control_block("Cmd", 4, &[4, 0, 0, 0, 0, 0, 0, 0, 0]);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let table = OpcodeTable::builtin();
        let (stream, defects) = walk_properties(&region, &header, &table);
        assert_eq!(stream.properties.len(), 1);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Position {
                value: PositionBlock::Short {
                    left: 0,
                    top: 0,
                    width: 0,
                    height: 0
                },
                ..
            }
        ));
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn the_undecoded_message_names_the_opcode_table_flag() {
        let value = PropertyValue::Undecoded {
            opcode: 99,
            offset: 0x10,
            control_type: "Form".to_owned(),
            bytes_not_read: 4,
        };
        let message = value.undecoded_message().unwrap();
        assert!(message.contains("--opcode-table"), "{message}");
        assert!(message.contains("99"), "{message}");
        assert!(message.contains("0x10"), "{message}");
        assert!(message.contains("Form"), "{message}");
    }

    #[test]
    fn a_resolved_property_gives_no_undecoded_message() {
        let value = PropertyValue::Byte {
            name: "WindowState".to_owned(),
            value: 1,
        };
        assert_eq!(value.undecoded_message(), None);
    }

    #[test]
    fn a_boolean_payload_of_0xffff_reports_as_minus_one_and_0x0000_reports_as_zero() {
        let text = b"[4]\n1 = { name = \"Flag\", payload = \"Boolean\" }\n";
        let table = OpcodeTable::parse(text).unwrap();

        let bytes = control_block("Cmd", 4, &[1, 0xFF, 0xFF]);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, _) = walk_properties(&region, &header, &table);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Boolean { value: -1, .. }
        ));

        let bytes = control_block("Cmd", 4, &[1, 0x00, 0x00]);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, _) = walk_properties(&region, &header, &table);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Boolean { value: 0, .. }
        ));
    }

    #[test]
    fn a_string_property_advances_by_vb_strs_own_declared_end() {
        let text = b"[4]\n1 = { name = \"Caption\", payload = \"Text\" }\n";
        let table = OpcodeTable::parse(text).unwrap();
        // opcode(1) len(2)=3 "Tag" nul(1)
        let mut body = vec![1u8, 0x03, 0x00];
        body.extend_from_slice(b"Tag");
        body.push(0x00);
        let bytes = control_block("Cmd", 4, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table);
        assert_eq!(stream.properties.len(), 1);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Text { value, .. } if value == "Tag"
        ));
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn two_properties_in_a_row_are_read_in_stream_order() {
        let text =
            b"[4]\n10 = { name = \"First\", payload = \"Byte\" }\n11 = { name = \"Second\", payload = \"Byte\" }\n";
        let table = OpcodeTable::parse(text).unwrap();
        let bytes = control_block("Cmd", 4, &[10, 1, 11, 2]);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table);
        assert_eq!(stream.properties.len(), 2);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Byte { name, value: 1 } if name == "First"
        ));
        assert!(matches!(
            &stream.properties[1],
            PropertyValue::Byte { name, value: 2 } if name == "Second"
        ));
        assert!(defects.is_empty(), "{defects:?}");
    }

    // --- Task 2: the position block and its escape at -32768 -------------

    #[test]
    fn read_position_block_with_a_first_value_of_100_consumes_8_bytes() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&100_i16.to_le_bytes());
        bytes.extend_from_slice(&200_i16.to_le_bytes());
        bytes.extend_from_slice(&300_i16.to_le_bytes());
        bytes.extend_from_slice(&400_i16.to_le_bytes());
        let region = Region::new(&bytes, Off::new(0));
        let (value, consumed) = read_position_block(&region, 0, 8).unwrap();
        assert_eq!(consumed, 8);
        assert!(matches!(
            value,
            PositionBlock::Short {
                left: 100,
                top: 200,
                width: 300,
                height: 400
            }
        ));
    }

    #[test]
    fn read_position_block_at_the_threshold_minus_32767_consumes_8_bytes_one_step_above_the_escape()
    {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(-32_767_i16).to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        let region = Region::new(&bytes, Off::new(0));
        let (value, consumed) = read_position_block(&region, 0, 8).unwrap();
        assert_eq!(consumed, 8);
        assert!(matches!(value, PositionBlock::Short { left: -32_767, .. }));
    }

    #[test]
    fn read_position_block_at_minus_32768_consumes_16_bytes_and_reads_four_i32_values() {
        // Synthetic fixture: no corpus file in this repository exercises
        // this escape (03-RESEARCH.md's own "Flagged assumption"). `left`'s
        // own first two bytes are the -32768 escape marker; the full i32
        // value they are part of is 0x0000_8000 = 32768.
        let mut body = Vec::new();
        body.extend_from_slice(&[0x00, 0x80, 0x00, 0x00]); // left
        body.extend_from_slice(&(-70_000_i32).to_le_bytes()); // top
        body.extend_from_slice(&123_456_i32.to_le_bytes()); // width
        body.extend_from_slice(&(-1_i32).to_le_bytes()); // height
        let region = Region::new(&body, Off::new(0));
        let (value, consumed) = read_position_block(&region, 0, 16).unwrap();
        assert_eq!(
            consumed, 16,
            "synthetic fixture: the -32768 escape must consume 16 bytes total"
        );
        assert!(matches!(
            value,
            PositionBlock::Long {
                left: 32_768,
                top: -70_000,
                width: 123_456,
                height: -1
            }
        ));
    }

    #[test]
    fn a_left_of_minus_one_reads_back_as_minus_one_not_65535() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(-1_i16).to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        let region = Region::new(&bytes, Off::new(0));
        let (value, _) = read_position_block(&region, 0, 8).unwrap();
        assert!(matches!(value, PositionBlock::Short { left: -1, .. }));
    }

    #[test]
    fn a_16_byte_form_that_runs_past_the_block_end_gives_a_defect_naming_the_byte_offset() {
        // Synthetic fixture: the escape marker sits one byte before the
        // declared block end, which is not enough room for the 16 byte
        // long form.
        let mut body = Vec::new();
        body.extend_from_slice(&[0x00, 0x80, 0x00, 0x00]);
        body.extend_from_slice(&0_i32.to_le_bytes());
        body.extend_from_slice(&0_i32.to_le_bytes());
        body.extend_from_slice(&0_i32.to_le_bytes());
        let region = Region::new(&body, Off::new(0x3000));
        let err = read_position_block(&region, 0, 15).unwrap_err();
        let message = format!("{}", err.kind);
        assert!(message.contains("0x3000"), "{message}");
    }

    #[test]
    fn read_position_block_refuses_rather_than_overflowing_near_u32_max() {
        let bytes = [0u8; 4];
        let region = Region::new(&bytes, Off::new(0));
        let err = read_position_block(&region, u32::MAX - 1, u32::MAX).unwrap_err();
        assert!(!format!("{}", err.kind).is_empty());
    }

    #[test]
    fn a_position_property_wired_through_walk_properties_advances_by_its_own_reported_count() {
        let table = OpcodeTable::builtin();
        let mut body = vec![4u8]; // opcode 4 = Position on CommandButton
        body.extend_from_slice(&10_i16.to_le_bytes());
        body.extend_from_slice(&20_i16.to_le_bytes());
        body.extend_from_slice(&30_i16.to_le_bytes());
        body.extend_from_slice(&40_i16.to_le_bytes());
        body.push(10); // opcode 10 = MousePointer (Byte), right after
        body.push(5);
        let bytes = control_block("Cmd", 4, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table);
        assert_eq!(stream.properties.len(), 2, "{:?}", stream.properties);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Position {
                value: PositionBlock::Short {
                    left: 10,
                    top: 20,
                    width: 30,
                    height: 40
                },
                ..
            }
        ));
        assert!(matches!(
            &stream.properties[1],
            PropertyValue::Byte { value: 5, .. }
        ));
        assert!(defects.is_empty(), "{defects:?}");
    }

    // --- Corpus test: LockWorkStation, the form's own properties ----------

    const LOCK_WORK_STATION: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/LockWorkStation/LockWorkStation.exe"
    ));

    #[test]
    fn lock_work_stations_form_properties_read_in_stream_order_and_stop_at_the_block_end() {
        use crate::read::pe::PeImage;
        use crate::vb::gui::{GuiObjectInfo, GuiTable};
        use crate::vb::header::{VbHeader, header_region};

        let image = PeImage::parse(LOCK_WORK_STATION).unwrap();
        let hdr = header_region(&image).unwrap();
        let header = VbHeader::read(&hdr).unwrap();
        let gui_table = GuiTable::walk(&image, &header).unwrap();
        let entry = gui_table.entries[0];
        let info = GuiObjectInfo::read(&image, entry.a_form_pointer).unwrap();
        let stream = info.form_stream().unwrap();
        let region = stream.region();

        let (control_header, _) = read_control_header(region);
        assert_eq!(control_header.c_type, 13, "the form's own cType");

        let table = OpcodeTable::builtin();
        let (props, _defects) = walk_properties(region, &control_header, &table);

        // The form's own first property, WindowState (opcode 10, Byte), is
        // resolved from the safe-provenance subset.
        assert!(
            !props.properties.is_empty(),
            "expected at least one resolved property before the walk stopped"
        );
        assert!(
            matches!(&props.properties[0], PropertyValue::Byte { name, .. } if name == "WindowState")
        );

        // The loop stops somewhere at or before the block's own end
        // (Length - 1 past the block start); it never reports a
        // stopped_at position past that bound.
        let length = region.u16_le(Off::new(0)).unwrap();
        let block_end_offset = region
            .file_offset(Off::new(u32::from(length).saturating_sub(1)))
            .unwrap();
        assert!(
            props.stopped_at <= block_end_offset.get(),
            "stopped_at {:#x} must not run past the block's own end {:#x}",
            props.stopped_at,
            block_end_offset.get()
        );
    }
}
