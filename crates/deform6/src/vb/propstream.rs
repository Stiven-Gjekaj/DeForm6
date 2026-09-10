//! Typed property payloads: the property loop, the position block escape,
//! the `Font` block, and the other special opcodes `03-RESEARCH.md` section
//! 8.5 names.
//!
//! Plan 03-06 fills this module. It serves FRM-03. Plan 03-15 wires the
//! resource blob arm to [`crate::vb::frx::extract_blob`]; it serves FRM-05.
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

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::region::{Off, Region};
use crate::vb::controltree::{ControlHeader, classify_control_type};
use crate::vb::frx::{self, BlobCursor};
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
    /// A `Font` payload, read through [`read_font_block`].
    Font {
        /// The property this opcode names.
        name: String,
        /// The font's own charset, style, weight, size and name.
        value: FontBlock,
    },
    /// A recovered resource blob, `Picture`'s own payload shape, read
    /// through [`frx::extract_blob`].
    ///
    /// Carries the facts a reader and a phase 4 `.frx` writer need, never
    /// the blob's own bytes: `extract_blob` already holds them in memory
    /// for exactly as long as this call needs them, and a second copy
    /// inside the report would sit in every [`crate::vb::ControlReport`] a
    /// form's blobs pass through for no consumer this plan has. A future
    /// `.frx` writer re-reads `offset..offset + 4 + declared_len` out of
    /// the executable's own bytes, the same range `extract_blob` read them
    /// from, whenever it needs the bytes themselves; this plan's own end to
    /// end test (`tests/blobs.rs`) does the same over the corpus executable
    /// it already holds in memory, and compares the result against the
    /// committed `.frx` beside it.
    Blob {
        /// The property this opcode names (`Icon`, not `Picture`: plan
        /// 03-13's own corpus-measured row already gives the `.frm`'s own
        /// name).
        name: String,
        /// The absolute file offset of the blob's own four byte length
        /// field.
        offset: u32,
        /// The blob's own declared length (`blobLen`): the eight byte
        /// inline picture header plus the image bytes.
        declared_len: u32,
        /// The image byte count: `declared_len - 8`.
        image_len: u32,
        /// The container format [`frx::sniff_format`] detected from the
        /// image's own first bytes.
        format: frx::ImageFormat,
        /// The `.frx` offset [`BlobCursor::take`] gave this blob: where a
        /// phase 4 writer would place it in the generated `.frx` file.
        frx_offset: u32,
    },
    /// A resource blob this repository could not read: present in the
    /// file, but its own bound check refused.
    ///
    /// Distinct from [`PropertyValue::Undecoded`] (an opcode this
    /// repository names no decoder for) and from a plain absence (the file
    /// itself marks the property `0xFFFFFFFF`, which produces no property
    /// value at all): this state means the opcode names a resource blob,
    /// the file marks one present, and its own length field or its own
    /// bytes refused a bound check. The defect [`frx::extract_blob`]
    /// returned is carried into [`walk_properties`]'s own defect list, and
    /// this value stays in the property list rather than being dropped, so
    /// a reader never mistakes an unreadable blob for one the file simply
    /// does not set.
    BlobUnreadable {
        /// The property this opcode names.
        name: String,
        /// The absolute file offset of the blob's own four byte length
        /// field, or of whichever earlier field this blob's own read
        /// refused at.
        offset: u32,
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
            | Self::Position { .. }
            | Self::Font { .. }
            | Self::Blob { .. }
            | Self::BlobUnreadable { .. } => None,
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

/// Builds the [`Defect`] for a `.frx` offset cursor that refused to
/// advance: [`BlobCursor::take`]'s own `u32` overflow refusal, converted
/// from a [`Refusal`] into a per-property defect. This is the one place
/// `walk_properties` reaches a `Refusal` rather than an `Option`, because
/// `BlobCursor::take` is the one function in this crate that must name a
/// value (the running `.frx` offset) no `&'static str` can carry.
///
/// No corpus program reaches this: it needs a form whose blobs sum past a
/// `u32`, which requires billions of bytes of resource data in one file.
/// It exists for the hostile file the corpus does not contain, per
/// `AGENTS.md`'s "no panic on any input, ever".
fn blob_cursor_defect(offset: u32, refusal: &Refusal) -> Defect {
    Defect {
        site: Site {
            offset,
            rva: None,
            structure: "BlobCursor",
            field: "take",
        },
        kind: DefectKind::StructureUnreadable {
            offset,
            reason: refusal.to_string(),
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

/// The `Font` payload: `BeginProperty Font ... EndProperty`, `STRUCTURES.md`
/// section 8.5.2.
///
/// The size is stored in tenths of a thousandth of a point. `AGENTS.md`
/// says to give the number that can be proved and not a number calculated
/// from a part, so this carries both: the raw stored value, and the value
/// in points with its own remainder, rather than only the divided number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontBlock {
    /// The charset byte at the block's own offset `0x01`.
    pub charset: u8,
    /// Style bit `0x02`.
    pub italic: bool,
    /// Style bit `0x04`.
    pub underline: bool,
    /// Style bit `0x08`.
    pub strikethrough: bool,
    /// The weight, at offset `0x04`, two bytes.
    pub weight: u16,
    /// The size exactly as the file stores it: tenths of a thousandth of a
    /// point.
    pub size_raw: u32,
    /// [`Self::size_raw`] divided by 10000, in points.
    pub size_points: u32,
    /// The remainder [`Self::size_points`] drops, so a size that does not
    /// divide evenly is never silently lost.
    pub size_remainder: u32,
    /// The font name, read as length-prefixed ASCII bytes, each its own
    /// Latin-1 code point.
    pub name: String,
}

/// Reads a [`FontBlock`] at `at`, bounded by `block_end`.
///
/// The block consumes `11 + n` bytes, `n` being the declared name length at
/// offset `0x0A`; the name length is checked against the remaining block
/// space before any allocation is sized from it.
///
/// # Errors
///
/// Returns a [`Defect`] naming the byte offset when the fixed 11 byte header
/// or the name would run past `block_end`.
fn read_font_block(
    block: &Region<'_>,
    at: u32,
    block_end: u32,
) -> Result<(FontBlock, u32), Defect> {
    let offset = block.file_offset(Off::new(at)).map_or(0, Off::get);
    let block_end_offset = block.file_offset(Off::new(block_end)).map_or(0, Off::get);

    if ends_within(at, 11, block_end).is_none() {
        let fixed_end_offset = block
            .file_offset(Off::new(at.saturating_add(11)))
            .map_or(0, Off::get);
        return Err(overrun_defect(offset, fixed_end_offset, block_end_offset));
    }

    let charset = at
        .checked_add(1)
        .and_then(|o| block.u8(Off::new(o)))
        .unwrap_or(0);
    let style = at
        .checked_add(3)
        .and_then(|o| block.u8(Off::new(o)))
        .unwrap_or(0);
    let weight = at
        .checked_add(4)
        .and_then(|o| block.u16_le(Off::new(o)))
        .unwrap_or(0);
    let size_raw = at
        .checked_add(6)
        .and_then(|o| block.u32_le(Off::new(o)))
        .unwrap_or(0);
    let name_len = at
        .checked_add(0x0A)
        .and_then(|o| block.u8(Off::new(o)))
        .unwrap_or(0);

    let total_width = 11_u32.saturating_add(u32::from(name_len));
    let Some(font_end) = ends_within(at, total_width, block_end) else {
        let name_end_offset = block
            .file_offset(Off::new(at.saturating_add(total_width)))
            .map_or(0, Off::get);
        return Err(overrun_defect(offset, name_end_offset, block_end_offset));
    };

    let name = at
        .checked_add(0x0B)
        .and_then(|name_start| block.take(Off::new(name_start), u32::from(name_len)))
        .map_or_else(String::new, |bytes| {
            bytes.iter().copied().map(char::from).collect()
        });

    let size_points = size_raw.div_euclid(10_000);
    let size_remainder = size_raw.rem_euclid(10_000);
    let consumed = font_end.checked_sub(at).unwrap_or(total_width);

    Ok((
        FontBlock {
            charset,
            italic: style & 0x02 != 0,
            underline: style & 0x04 != 0,
            strikethrough: style & 0x08 != 0,
            weight,
            size_raw,
            size_points,
            size_remainder,
            name,
        },
        consumed,
    ))
}

/// The `cType` `STRUCTURES.md` section 8.4.1 gives for `Form`.
const CT_FORM: u8 = 13;

/// The `cType` `STRUCTURES.md` section 8.4.1 gives for `MDIForm`.
const CT_MDIFORM: u8 = 20;

/// The opcode `STRUCTURES.md` section 8.5.1 names `ScaleMode` on Form and
/// MDIForm, with its own conditional skip.
const SCALE_MODE_OPCODE: u8 = 25;

/// The three opcodes `STRUCTURES.md` section 8.5.1 marks "consume 1 byte,
/// no output" on Form and MDIForm.
const NO_OUTPUT_OPCODES: [u8; 3] = [0, 98, 99];

/// Handles a Form/MDIForm special opcode before the generic typed path is
/// tried. Covers only the four safe-provenance control types this plan's
/// own opcode table subset names (Form, MDIForm, CommandButton, Label,
/// ListBox); of those, only Form and MDIForm carry a special case at all.
///
/// `None` means `opcode` is not a special case for `control_type`, and the
/// caller falls through to the generic [`OpcodeTable::lookup`] path.
/// `Some(Ok(new_cursor))` means the special case consumed bytes up to
/// `new_cursor` and produced no property, per `STRUCTURES.md` section
/// 8.5.1's own words for these opcodes. `Some(Err(_))` means the special
/// case would run past the block's own end.
fn read_special_opcode(
    control_type: u8,
    opcode: u8,
    block: &Region<'_>,
    payload_start: u32,
    block_end: u32,
) -> Option<Result<u32, Defect>> {
    if control_type != CT_FORM && control_type != CT_MDIFORM {
        return None;
    }
    if NO_OUTPUT_OPCODES.contains(&opcode) {
        // The opcode byte itself is already consumed by the caller before
        // `payload_start`; this opcode carries no payload the format
        // exposes, so producing nothing here is correct, not a gap.
        return Some(Ok(payload_start));
    }
    if opcode == SCALE_MODE_OPCODE {
        return Some(read_scale_mode(block, payload_start, block_end));
    }
    None
}

/// Reads the Form `ScaleMode` special opcode: one byte for the scale mode,
/// then, only when that byte is `0`, 16 more skipped bytes, then one flags
/// byte (`0x20` = `AutoRedraw`, `0x02` = `FontTransparent`), then one more
/// byte. Gives the cursor position after all of that; produces no property,
/// per `STRUCTURES.md` section 8.5.1.
fn read_scale_mode(block: &Region<'_>, payload_start: u32, block_end: u32) -> Result<u32, Defect> {
    let offset = block
        .file_offset(Off::new(payload_start))
        .map_or(0, Off::get);
    let block_end_offset = block.file_offset(Off::new(block_end)).map_or(0, Off::get);

    let Some(after_mode) = ends_within(payload_start, 1, block_end) else {
        let end_offset = block
            .file_offset(Off::new(payload_start.saturating_add(1)))
            .map_or(0, Off::get);
        return Err(overrun_defect(offset, end_offset, block_end_offset));
    };
    let scale_mode = block.u8(Off::new(payload_start)).unwrap_or(0);

    let after_skip = if scale_mode == 0 {
        let Some(skipped) = ends_within(after_mode, 16, block_end) else {
            let end_offset = block
                .file_offset(Off::new(after_mode.saturating_add(16)))
                .map_or(0, Off::get);
            return Err(overrun_defect(offset, end_offset, block_end_offset));
        };
        skipped
    } else {
        after_mode
    };

    let Some(after_flags) = ends_within(after_skip, 1, block_end) else {
        let end_offset = block
            .file_offset(Off::new(after_skip.saturating_add(1)))
            .map_or(0, Off::get);
        return Err(overrun_defect(offset, end_offset, block_end_offset));
    };

    // One more byte after the flags byte, per STRUCTURES.md section 8.5.1.
    let Some(final_end) = ends_within(after_flags, 1, block_end) else {
        let end_offset = block
            .file_offset(Off::new(after_flags.saturating_add(1)))
            .map_or(0, Off::get);
        return Err(overrun_defect(offset, end_offset, block_end_offset));
    };

    Ok(final_end)
}

/// Walks one control block's own property stream.
///
/// `block` is the control block's own bounded window, the same
/// `Length + 2`-byte region [`crate::vb::controltree::read_control_header`]
/// reads its header from. `header` is that block's own [`ControlHeader`],
/// which gives [`ControlHeader::header_len`], the byte offset the property
/// stream begins at. `table` resolves each opcode to a name and a payload
/// shape. On a Form or MDIForm control, [`read_special_opcode`] is tried
/// first, per `STRUCTURES.md` section 8.5.1; a resolved
/// [`PayloadType::Picture`] entry calls [`frx::extract_blob`] and advances
/// the cursor by the count that call returns, never by a count this
/// function computes.
///
/// `blob_cursor` is one form's own [`BlobCursor`]: the caller
/// (`vb/mod.rs::compose_form`) owns one per form and threads it by mutable
/// reference through every control that form's own tree holds, in control
/// tree order, so a blob's `.frx` offset is assigned in the order the
/// property stream holds and a second form starts its own cursor at 0.
///
/// Gives `(PropertyStream, Vec<Defect>)`. A payload that would end past the
/// block's own end is never truncated to fit: it gives a [`Defect`] and
/// stops the loop, and no value is reported for it. A resource blob the
/// file marks absent gives neither a value nor a defect, and does not stop
/// the loop; a resource blob the file marks present but that this reader
/// could not read gives a [`PropertyValue::BlobUnreadable`], carries its
/// own defect into the returned list, and does stop the loop, the same
/// honest treatment every other unreadable payload in this function gets.
#[must_use]
pub fn walk_properties(
    block: &Region<'_>,
    header: &ControlHeader,
    table: &OpcodeTable,
    blob_cursor: &mut BlobCursor,
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

        if let Some(special) =
            read_special_opcode(header.c_type, opcode, block, payload_start, block_end)
        {
            match special {
                Ok(new_cursor) => {
                    cursor = new_cursor;
                    continue;
                }
                Err(defect) => {
                    defects.push(defect);
                    cursor = block_end;
                    break;
                }
            }
        }

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
                PayloadType::Font => match read_font_block(block, payload_start, block_end) {
                    Ok((value, consumed)) => {
                        let Some(new_cursor) = payload_start.checked_add(consumed) else {
                            break;
                        };
                        properties.push(PropertyValue::Font {
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
                PayloadType::Picture => {
                    // `frx::extract_blob` owns every bound check on the
                    // declared length and the block's own remaining bytes;
                    // this arm computes no width of its own and never
                    // repeats a check `extract_blob` already made.
                    let (blob, consumed, defect) =
                        frx::extract_blob(block, Off::new(payload_start));
                    match blob {
                        Some(blob) => match blob_cursor.take(&blob) {
                            Ok(frx_offset) => {
                                let image_len = u32::try_from(blob.image.len()).unwrap_or(u32::MAX);
                                let format = frx::sniff_format(&blob.image);
                                properties.push(PropertyValue::Blob {
                                    name: entry.name.clone(),
                                    offset: blob.offset,
                                    declared_len: blob.declared_len,
                                    image_len,
                                    format,
                                    frx_offset,
                                });
                                let Some(new_cursor) = payload_start.checked_add(consumed) else {
                                    break;
                                };
                                cursor = new_cursor;
                            }
                            Err(refusal) => {
                                defects.push(blob_cursor_defect(blob.offset, &refusal));
                                properties.push(PropertyValue::BlobUnreadable {
                                    name: entry.name.clone(),
                                    offset: blob.offset,
                                });
                                cursor = block_end;
                                break;
                            }
                        },
                        None => match defect {
                            Some(d) => {
                                let unreadable_offset = d.site.offset;
                                defects.push(d);
                                properties.push(PropertyValue::BlobUnreadable {
                                    name: entry.name.clone(),
                                    offset: unreadable_offset,
                                });
                                cursor = block_end;
                                break;
                            }
                            None => {
                                // Absent (`0xFFFFFFFF`): no property, no
                                // defect, and the loop continues.
                                let Some(new_cursor) = payload_start.checked_add(consumed) else {
                                    break;
                                };
                                cursor = new_cursor;
                            }
                        },
                    }
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
    use super::{
        BlobCursor, PositionBlock, PropertyValue, read_font_block, read_position_block,
        walk_properties,
    };
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
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
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
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
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
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
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
        // 200 is deliberately outside both FORM_ROWS and the special
        // no-output/ScaleMode opcodes (0, 25, 98, 99), so this exercises a
        // genuine lookup miss, not a special case.
        let bytes = control_block("Frm1", 13, &[200, 1, 2, 3, 4, 5]);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let table = OpcodeTable::builtin(); // holds no entry for Form opcode 200 here
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
        assert_eq!(stream.properties.len(), 1);
        match &stream.properties[0] {
            PropertyValue::Undecoded {
                opcode,
                offset,
                control_type,
                bytes_not_read,
            } => {
                assert_eq!(*opcode, 200);
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
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
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
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
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
        let (stream, _) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Boolean { value: -1, .. }
        ));

        let bytes = control_block("Cmd", 4, &[1, 0x00, 0x00]);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, _) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
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
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
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
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
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
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
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

    // --- Task 3: the Font block and the special opcodes -------------------

    #[test]
    fn a_font_block_with_a_name_of_13_characters_consumes_24_bytes() {
        let name = "ComicSansMS12"; // 13 ASCII characters
        assert_eq!(name.len(), 13);
        let mut bytes = vec![0u8]; // +0x00 unknown
        bytes.push(7); // +0x01 charset
        bytes.push(0); // +0x02 unknown
        bytes.push(0x0E); // +0x03 style: italic | underline | strikethrough
        bytes.extend_from_slice(&400_u16.to_le_bytes()); // +0x04 weight
        bytes.extend_from_slice(&82_500_u32.to_le_bytes()); // +0x06 size
        bytes.push(13); // +0x0A name length
        bytes.extend_from_slice(name.as_bytes()); // +0x0B name
        let block_end = u32::try_from(bytes.len()).unwrap();
        let region = Region::new(&bytes, Off::new(0));
        let (value, consumed) = read_font_block(&region, 0, block_end).unwrap();
        assert_eq!(consumed, 24, "11 fixed bytes plus 13 name bytes");
        assert_eq!(value.charset, 7);
        assert!(value.italic && value.underline && value.strikethrough);
        assert_eq!(value.weight, 400);
        assert_eq!(value.size_raw, 82_500);
        assert_eq!(value.size_points, 8);
        assert_eq!(value.size_remainder, 2_500);
        assert_eq!(value.name, name);
    }

    #[test]
    fn a_style_byte_of_0x00_gives_all_three_style_flags_clear() {
        let mut bytes = vec![0u8, 0, 0, 0x00];
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.push(0); // name length = 0
        let block_end = u32::try_from(bytes.len()).unwrap();
        let region = Region::new(&bytes, Off::new(0));
        let (value, consumed) = read_font_block(&region, 0, block_end).unwrap();
        assert_eq!(consumed, 11);
        assert!(!value.italic && !value.underline && !value.strikethrough);
        assert_eq!(value.name, "");
    }

    #[test]
    fn a_stored_size_that_divides_evenly_gives_a_zero_remainder() {
        let mut bytes = vec![0u8, 0, 0, 0];
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&120_000_u32.to_le_bytes()); // 12 points exactly
        bytes.push(0);
        let block_end = u32::try_from(bytes.len()).unwrap();
        let region = Region::new(&bytes, Off::new(0));
        let (value, _) = read_font_block(&region, 0, block_end).unwrap();
        assert_eq!(value.size_points, 12);
        assert_eq!(value.size_remainder, 0);
    }

    #[test]
    fn a_font_name_length_larger_than_the_block_gives_a_defect_and_sizes_no_allocation() {
        let mut bytes = vec![0u8, 0, 0, 0];
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.push(200); // declares 200 name bytes; none of them are present
        let block_end = u32::try_from(bytes.len()).unwrap();
        let region = Region::new(&bytes, Off::new(0x4000));
        let err = read_font_block(&region, 0, block_end).unwrap_err();
        let message = format!("{}", err.kind);
        assert!(message.contains("0x4000"), "{message}");
    }

    #[test]
    fn a_font_property_wired_through_walk_properties_advances_by_its_own_reported_count() {
        let table = OpcodeTable::builtin();
        let mut body = vec![64u8]; // opcode 64 = Font on Form
        body.push(0); // unknown
        body.push(0); // charset
        body.push(0); // unknown
        body.push(0); // style
        body.extend_from_slice(&0_u16.to_le_bytes()); // weight
        body.extend_from_slice(&0_u32.to_le_bytes()); // size
        body.push(0); // name length = 0
        body.push(10); // opcode 10 = WindowState (Byte), right after
        body.push(2);
        let bytes = control_block("Frm1", 13, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
        assert_eq!(stream.properties.len(), 2, "{:?}", stream.properties);
        assert!(matches!(&stream.properties[0], PropertyValue::Font { .. }));
        assert!(matches!(
            &stream.properties[1],
            PropertyValue::Byte { value: 2, .. }
        ));
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn a_form_scale_mode_byte_of_0_consumes_sixteen_more_bytes_before_the_flags_byte() {
        let table = OpcodeTable::builtin();
        let mut body = vec![25u8, 0]; // opcode 25 = ScaleMode, mode = 0
        body.extend(std::iter::repeat_n(0xAA_u8, 16)); // the 16 skipped bytes
        body.push(0x20); // flags byte
        body.push(0x00); // one more byte
        body.push(10); // opcode 10 = WindowState, right after
        body.push(3);
        let bytes = control_block("Frm1", 13, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
        assert_eq!(stream.properties.len(), 1, "{:?}", stream.properties);
        assert!(
            matches!(&stream.properties[0], PropertyValue::Byte { value: 3, .. }),
            "{:?}",
            stream.properties[0]
        );
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn a_non_zero_form_scale_mode_does_not_skip_sixteen_bytes() {
        let table = OpcodeTable::builtin();
        let mut body = vec![25u8, 3]; // opcode 25 = ScaleMode, mode = 3
        body.push(0x20); // flags byte, right after the mode (no skip)
        body.push(0x00); // one more byte
        body.push(10); // opcode 10 = WindowState, right after
        body.push(7);
        let bytes = control_block("Frm1", 13, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
        assert_eq!(stream.properties.len(), 1, "{:?}", stream.properties);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Byte { value: 7, .. }
        ));
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn the_no_output_opcodes_zero_ninety_eight_and_ninety_nine_consume_only_their_own_byte() {
        let table = OpcodeTable::builtin();
        for opcode in [0u8, 98, 99] {
            let body = vec![opcode, 10, 4]; // the special opcode, then WindowState = 4
            let bytes = control_block("Frm1", 13, &body);
            let region = Region::new(&bytes, Off::new(0));
            let (header, _) = read_control_header(&region);
            let (stream, defects) =
                walk_properties(&region, &header, &table, &mut BlobCursor::new());
            assert_eq!(
                stream.properties.len(),
                1,
                "opcode {opcode}: {:?}",
                stream.properties
            );
            assert!(
                matches!(&stream.properties[0], PropertyValue::Byte { value: 4, .. }),
                "opcode {opcode}: {:?}",
                stream.properties[0]
            );
            assert!(defects.is_empty(), "opcode {opcode}: {defects:?}");
        }
    }

    #[test]
    fn special_opcode_handling_does_not_apply_to_commandbutton() {
        // Opcode 25 on CommandButton (cType 4) is neither in the builtin
        // subset nor one of Form's own special cases: it is a genuine
        // lookup miss, not the ScaleMode special case.
        let table = OpcodeTable::builtin();
        let body = vec![25u8, 0, 1, 2, 3];
        let bytes = control_block("Cmd", 4, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
        assert_eq!(stream.properties.len(), 1);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Undecoded { opcode: 25, .. }
        ));
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn mdiform_shares_the_forms_special_opcode_handling() {
        let table = OpcodeTable::builtin();
        let body = vec![98u8, 10, 9]; // a no-output opcode, then WindowState = 9
        let bytes = control_block("Mdi1", 20, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());
        assert_eq!(stream.properties.len(), 1);
        assert!(matches!(
            &stream.properties[0],
            PropertyValue::Byte { value: 9, .. }
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

        let (control_header, _) = read_control_header(&region);
        assert_eq!(control_header.c_type, 13, "the form's own cType");

        let table = OpcodeTable::builtin();
        let (props, _defects) =
            walk_properties(&region, &control_header, &table, &mut BlobCursor::new());

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

    // --- Plan 03-15, Task 2: the resource blob arm calls extract_blob -----

    /// A hand-written table naming opcode 1 `Icon` (`Picture`) and opcode 2
    /// `After` (`Byte`), on control type 4, so a test can prove the blob
    /// arm advances the cursor by the count `extract_blob` returned and the
    /// loop reaches the property right after it.
    fn picture_table() -> OpcodeTable {
        let text = concat!(
            "[4]\n",
            "1 = { name = \"Icon\", payload = \"Picture\" }\n",
            "2 = { name = \"After\", payload = \"Byte\" }\n",
        );
        OpcodeTable::parse(text.as_bytes()).unwrap()
    }

    /// Gives the `.frx` offset of the property list's own first
    /// [`PropertyValue::Blob`], or `None` when it holds none.
    fn first_blob_frx_offset(properties: &[PropertyValue]) -> Option<u32> {
        properties.iter().find_map(|p| match p {
            PropertyValue::Blob { frx_offset, .. } => Some(*frx_offset),
            _ => None,
        })
    }

    #[test]
    fn the_resource_blob_arm_advances_by_the_count_extract_blob_returned() {
        let table = picture_table();
        let mut body = vec![1u8]; // opcode 1 = Icon (Picture)
        body.extend_from_slice(&12_u32.to_le_bytes()); // declared_len = 12
        body.extend_from_slice(&[0xAA; 8]); // the 8 byte inline header
        body.extend_from_slice(&[0xBB; 4]); // 12 - 8 = 4 image bytes
        body.push(2); // opcode 2 = After (Byte), right after the blob
        body.push(7);
        let bytes = control_block("Cmd", 4, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());

        assert_eq!(stream.properties.len(), 2, "{:?}", stream.properties);
        match &stream.properties[0] {
            PropertyValue::Blob {
                name,
                declared_len,
                image_len,
                frx_offset,
                ..
            } => {
                assert_eq!(name, "Icon");
                assert_eq!(*declared_len, 12);
                assert_eq!(*image_len, 4);
                assert_eq!(
                    *frx_offset, 0,
                    "the first blob a fresh cursor gives must start at 0"
                );
            }
            other => panic!("expected Blob, got {other:?}"),
        }
        assert!(
            matches!(&stream.properties[1], PropertyValue::Byte { name, value: 7 } if name == "After"),
            "the arm must never compute a width of its own: the property right after the blob \
             must be read at the position extract_blob's own consumed count landed on, {:?}",
            stream.properties[1]
        );
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn an_absent_resource_property_gives_no_blob_no_defect_and_does_not_stop_the_loop() {
        let table = picture_table();
        let mut body = vec![1u8]; // opcode 1 = Icon (Picture)
        body.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes()); // absent
        body.push(2); // opcode 2 = After (Byte), right after
        body.push(9);
        let bytes = control_block("Cmd", 4, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());

        assert_eq!(
            stream.properties.len(),
            1,
            "an absent blob reports no property at all for its own opcode: {:?}",
            stream.properties
        );
        assert!(
            matches!(&stream.properties[0], PropertyValue::Byte { name, value: 9 } if name == "After"),
            "the loop must continue past the absent marker: {:?}",
            stream.properties[0]
        );
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn an_unreadable_blob_is_reported_present_and_unreadable_with_its_byte_offset() {
        // A declared length of 3 cannot hold its own 8 byte header (the
        // same shape frx.rs's own
        // `a_length_of_three_gives_a_defect_naming_the_value_and_the_offset`
        // test proves at the `extract_blob` layer); three padding bytes
        // keep the length field itself inside the block, so this exercises
        // the "too small for its own header" refusal, not "runs past the
        // block's own end".
        let table = picture_table();
        let mut body = vec![1u8]; // opcode 1 = Icon (Picture)
        body.extend_from_slice(&3_u32.to_le_bytes());
        body.extend_from_slice(&[0, 0, 0]);
        let bytes = control_block("Cmd", 4, &body);
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        let (stream, defects) = walk_properties(&region, &header, &table, &mut BlobCursor::new());

        assert_eq!(stream.properties.len(), 1, "{:?}", stream.properties);
        match &stream.properties[0] {
            PropertyValue::BlobUnreadable { name, offset } => {
                assert_eq!(name, "Icon");
                assert!(*offset > 0, "the byte offset must be carried, not zero");
            }
            other => panic!(
                "expected BlobUnreadable, distinguishable from the absent case's empty \
                 property list, got {other:?}"
            ),
        }
        assert_eq!(defects.len(), 1, "{defects:?}");
        assert!(matches!(
            defects[0].kind,
            DefectKind::BlobLenTooSmall { .. }
        ));
    }

    #[test]
    fn two_forms_in_one_program_each_start_their_own_blob_cursor_at_zero() {
        let table = picture_table();
        let mut body = vec![1u8]; // opcode 1 = Icon (Picture)
        body.extend_from_slice(&12_u32.to_le_bytes());
        body.extend_from_slice(&[0xAA; 8]);
        body.extend_from_slice(&[0xBB; 4]);

        // Two entirely separate control blocks, each read through its own
        // fresh `BlobCursor`, the same shape `compose_form` gives every
        // form: a local `BlobCursor::new()`, never carried over from a
        // form composed before it.
        let bytes_a = control_block("FrmA", 4, &body);
        let region_a = Region::new(&bytes_a, Off::new(0));
        let (header_a, _) = read_control_header(&region_a);
        let (stream_a, _) = walk_properties(&region_a, &header_a, &table, &mut BlobCursor::new());

        let bytes_b = control_block("FrmB", 4, &body);
        let region_b = Region::new(&bytes_b, Off::new(0));
        let (header_b, _) = read_control_header(&region_b);
        let (stream_b, _) = walk_properties(&region_b, &header_b, &table, &mut BlobCursor::new());

        assert_eq!(first_blob_frx_offset(&stream_a.properties), Some(0));
        assert_eq!(
            first_blob_frx_offset(&stream_b.properties),
            Some(0),
            "a second form's own cursor must not carry over the first form's"
        );
    }
}
