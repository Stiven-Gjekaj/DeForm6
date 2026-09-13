//! Property value serialisation: turns a decoded
//! `crate::vb::propstream::PropertyValue` into the exact bytes a `.frm`
//! line, a colour, or an enumeration comment carries.
//!
//! [`format_value`] is the whole public entry point. It never returns a
//! placeholder: a property either becomes a real value, becomes several
//! real named lines (a `Position` payload becomes `Left`, `Top`, `Width`
//! and `Height`; a `Font` payload becomes its own seven keys), asks the
//! caller for a resource reference, or is omitted with a report item
//! naming why. `FILE-FORMATS.md` section 2.6 proves that a `Begin` block
//! accepts no comment and no blank line, so an undecoded property has no
//! legal way to say "unknown" inline: the report is the only place that
//! doubt can live.
//!
//! The match in [`format_value`] has one arm for each variant of
//! [`PropertyValue`] and no wildcard arm, the same discipline
//! `crate::error::DefectKind::severity` already uses: a variant this crate
//! adds later fails to compile here until somebody decides how to write
//! it.
//!
//! Plan 04-02 fills this module.

use crate::report::{Confidence, Evidence, ReportItem};
use crate::vb::propstream::{FontBlock, PositionBlock, PropertyValue};
use crate::write::model::{MAX_INLINE_STRING_LEN, encode_windows_1252};

/// The three property names this corpus proves carry a colour.
/// `FILE-FORMATS.md` section 3.6: `BackColor` (240 lines), `ForeColor`
/// (203 lines) and `FillColor` (3 lines), out of 446 colour values total.
/// The shape lookup is by this exact name, never by payload type: the
/// reader's own `PayloadType` (`crate::vb::opcodes`) has no `Colour`
/// variant, because it was built to decode bytes, not to remember display
/// intent.
pub const COLOUR_PROPERTIES: [&str; 3] = ["BackColor", "ForeColor", "FillColor"];

/// The sixteen enumeration member names `FILE-FORMATS.md` section 3.5
/// proves, and only these: `(property name, value, member name)`. Taken
/// from section 3.5's own list and from nowhere else, then matched to the
/// property that carries each one by grepping every corpus `.frm` file
/// this session for a line of the exact shape `Name = Value  'Member`. A
/// value this table does not name still writes a bare number, with no
/// comment at all: section 3.5 states the comment is decoration the
/// loader ignores, so leaving one off for a value outside this proven set
/// is safe.
pub const ENUM_MEMBERS: &[(&str, i64, &str)] = &[
    ("Appearance", 0, "Flat"),
    ("BorderStyle", 0, "None"),
    ("FillStyle", 0, "Solid"),
    ("BackStyle", 0, "Transparent"),
    ("Value", 1, "Checked"),
    ("BorderStyle", 1, "Fixed Single"),
    ("Alignment", 1, "Right Justify"),
    ("Alignment", 2, "Center"),
    ("StartUpPosition", 2, "CenterScreen"),
    ("MousePointer", 2, "Cross"),
    ("Style", 2, "Dropdown List"),
    ("ScrollBars", 3, "Both"),
    ("ScaleMode", 3, "Pixel"),
    ("StartUpPosition", 3, "Windows Default"),
    ("BorderStyle", 4, "Fixed ToolWindow"),
    ("MousePointer", 99, "Custom"),
];

/// Which written shape a name-keyed numeric property takes. Never decided
/// from the payload type: the reader's own payload type carries no colour
/// variant and no enumeration variant, because it was built to decode
/// bytes, not to remember display intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueShape {
    /// `BackColor`, `ForeColor` or `FillColor`: [`format_colour`] decides
    /// the rest, branching on the control rather than on the value.
    Colour,
    /// Every other numeric property: a plain decimal, with a known
    /// enumeration comment when [`enum_member_name`] finds one for it.
    PlainNumber,
}

/// Gives the [`ValueShape`] property `name` takes.
#[must_use]
pub fn shape_for(name: &str) -> ValueShape {
    if COLOUR_PROPERTIES.contains(&name) {
        ValueShape::Colour
    } else {
        ValueShape::PlainNumber
    }
}

/// Gives the enumeration member name `(name, value)` names, or `None` when
/// [`ENUM_MEMBERS`] holds no such pair. Never a guess: a value outside the
/// table takes no comment, per `FILE-FORMATS.md` section 3.5.
#[must_use]
pub fn enum_member_name(name: &str, value: i64) -> Option<&'static str> {
    ENUM_MEMBERS
        .iter()
        .find(|(row_name, row_value, _)| *row_name == name && *row_value == value)
        .map(|(_, _, member)| *member)
}

/// Formats a colour value: eight upper case hex digits between `&H` and
/// `&` on an intrinsic control, or the same signed value as a plain
/// decimal on an external (OCX) control. `FILE-FORMATS.md` section 3.6:
/// the two forms carry the same bit pattern (the one corpus file that
/// proves the OCX form writes `-2147483643` where an intrinsic control
/// writes `&H80000005&` for the same value), so both are built from the
/// one signed value the reader gave, never from two different readings.
#[must_use]
pub fn format_colour(value: i32, is_external: bool) -> String {
    if is_external {
        value.to_string()
    } else {
        format!("&H{:08X}&", value.cast_unsigned())
    }
}

/// Formats a plain number, adding the known enumeration comment when
/// [`enum_member_name`] finds one for `name` and `value`, and writing a
/// bare number, with no comment at all, otherwise.
#[must_use]
fn format_number_with_enum_comment(name: &str, value: i64) -> String {
    match enum_member_name(name, value) {
        Some(member) => format!("{value}  '{member}"),
        None => value.to_string(),
    }
}

/// Formats a boolean the exact way the IDE writes one: `0` then three
/// spaces then `'False`, or `-1` then two spaces then `'True`.
/// `FILE-FORMATS.md` section 3.4: there is no other spelling. Any value
/// that is not exactly zero is written as `True`, matching VB6's own
/// truthiness rather than the two literal bit patterns alone.
#[must_use]
fn format_boolean(flag: bool) -> String {
    if flag {
        "-1  'True".to_owned()
    } else {
        "0   'False".to_owned()
    }
}

/// Escapes `text` for an inline `.frm` string: wraps it in double quotes
/// and doubles every inner double quote. `FILE-FORMATS.md` section 3.3:
/// this is the only escape the format has. No backslash escape is ever
/// written, even when the recovered text holds one.
#[must_use]
pub fn escape_inline_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_add(2));
    out.push('"');
    for ch in text.chars() {
        if ch == '"' {
            out.push('"');
        }
        out.push(ch);
    }
    out.push('"');
    out
}

/// What a recovered string's own length and content decide: whether it
/// goes inline, or needs a resource reference instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineDecision {
    /// The string fits inline: the exact quoted, escaped text, ready to
    /// follow the `= ` on a property line.
    Inline(String),
    /// The string does not fit inline: the caller must write a resource
    /// reference instead, using the file name and the offset it supplies.
    Resource,
}

/// Decides whether `text` goes inline or needs a resource reference.
///
/// `FILE-FORMATS.md` gap 1: the real threshold is between 98 and 153
/// encoded bytes, and the corpus does not fix it. [`MAX_INLINE_STRING_LEN`]
/// (97) is the safe default: the longest inline string the corpus proves,
/// so every string this tool writes inline sits inside the proved range,
/// never inside the unresolved gap.
///
/// A string holding a line break never goes inline, whatever its length.
/// `FILE-FORMATS.md` section 3.3: a raw line break inside the quotes would
/// end the `.frm` line, and the next recovered byte would be parsed as a
/// property name (threat T-4-05).
///
/// Counts encoded bytes, never Unicode scalar values: the encoder maps
/// each character below `0x100` to one byte, the same convention
/// [`crate::write::model::SafeName`] already counts a name's own length
/// by.
#[must_use]
pub fn inline_decision(text: &str) -> InlineDecision {
    if text.contains('\r') || text.contains('\n') {
        return InlineDecision::Resource;
    }
    let (encoded, _substituted) = encode_windows_1252(text);
    if encoded.len() > MAX_INLINE_STRING_LEN {
        InlineDecision::Resource
    } else {
        InlineDecision::Inline(escape_inline_string(text))
    }
}

/// Builds a resource reference: the quoted file name, a colon, and the
/// offset in upper case hex, padded to at least four digits. `long`
/// selects the leading dollar sign the long string form takes.
///
/// `FILE-FORMATS.md` gap 2: this tool always writes the long string form
/// with a four byte count for a string that must leave the form file, the
/// one shape a corpus resource file proves exactly (`Gradient.frx`); it
/// never writes the one byte count form section 4.5 reconstructs by
/// arithmetic from a damaged file, since that reconstruction is not a
/// second observation.
///
/// This function takes `offset` as a parameter and computes no offset of
/// its own: the one place an offset is computed in this repository is
/// [`crate::vb::frx::BlobCursor`].
#[must_use]
pub fn format_resource_reference(file_name: &str, offset: u32, long: bool) -> String {
    let prefix = if long { "$" } else { "" };
    format!("{prefix}\"{file_name}\":{offset:04X}")
}

/// Gives the four `Left`, `Top`, `Width` and `Height` lines a `Position`
/// payload decomposes into.
///
/// There is no single `Position` line in this format: `FILE-FORMATS.md`
/// names no such key anywhere in section 3, because the reader's own
/// compact coordinate block is a phase 3 reading convenience, not a
/// written property. The corpus writes four ordinary plain-decimal
/// properties instead. Plan 04-04's own alphabetical ordering rule sorts
/// these four alongside every other property in the block; this function
/// only names them and formats their values.
fn position_coordinates(value: &PositionBlock) -> [(&'static str, String); 4] {
    let (left, top, width, height) = match value {
        PositionBlock::Short {
            left,
            top,
            width,
            height,
        } => (
            i64::from(*left),
            i64::from(*top),
            i64::from(*width),
            i64::from(*height),
        ),
        PositionBlock::Long {
            left,
            top,
            width,
            height,
        } => (
            i64::from(*left),
            i64::from(*top),
            i64::from(*width),
            i64::from(*height),
        ),
    };
    [
        ("Left", left.to_string()),
        ("Top", top.to_string()),
        ("Width", width.to_string()),
        ("Height", height.to_string()),
    ]
}

/// Formats a font size from its own points and its own ten-thousandths
/// remainder, both already computed by the reader
/// ([`FontBlock::size_points`], [`FontBlock::size_remainder`]). Builds the
/// fraction from the remainder's own decimal digits, trimming trailing
/// zeros, rather than reconstructing a floating point value: `AGENTS.md`
/// asks for the number that can be proved, and a digit trim never rounds.
#[must_use]
fn format_font_size(size_points: u32, size_remainder: u32) -> String {
    if size_remainder == 0 {
        return size_points.to_string();
    }
    let digits = format!("{size_remainder:04}");
    let trimmed = digits.trim_end_matches('0');
    format!("{size_points}.{trimmed}")
}

/// Gives the seven `Font` block lines in the fixed order the IDE always
/// writes them: `Name`, `Size`, `Charset`, `Weight`, `Underline`,
/// `Italic`, `Strikethrough`. `FILE-FORMATS.md` section 3.9: 235 of 235
/// blocks in the corpus hold exactly these seven keys, in exactly this
/// order.
fn font_lines(value: &FontBlock) -> [(&'static str, String); 7] {
    [
        ("Name", escape_inline_string(&value.name)),
        (
            "Size",
            format_font_size(value.size_points, value.size_remainder),
        ),
        ("Charset", i64::from(value.charset).to_string()),
        ("Weight", i64::from(value.weight).to_string()),
        ("Underline", format_boolean(value.underline)),
        ("Italic", format_boolean(value.italic)),
        ("Strikethrough", format_boolean(value.strikethrough)),
    ]
}

/// What one call to [`format_value`] decided for a single property.
///
/// Never a placeholder: a property either becomes a real value
/// ([`Self::Line`]), becomes several real named lines ([`Self::Multi`],
/// for a `Position` or a `Font` payload), asks the caller for a resource
/// reference ([`Self::Resource`]), or produces no line at all
/// ([`Self::Omit`]), with the accompanying [`ReportItem`] naming why.
#[derive(Clone, Debug, PartialEq)]
pub enum FormattedValue {
    /// The value's own formatted text, ready to follow the `= ` on a
    /// property line.
    Line(String),
    /// This one property expands into several separate named lines, in
    /// the order given: `(property name, formatted value)` pairs.
    Multi(Vec<(&'static str, String)>),
    /// The value does not fit inline: the caller must write a resource
    /// reference, supplying the file name and the offset.
    Resource,
    /// This property produces no line at all.
    Omit,
}

/// Turns one decoded (or undecoded) property into a [`FormattedValue`],
/// plus a [`ReportItem`] when the property is omitted.
///
/// `is_external` decides the colour branch ([`format_colour`]): an
/// intrinsic control writes `&H`-bracketed hex, an external (OCX) control
/// writes a plain signed decimal, for the same value. Every other shape
/// ignores it.
///
/// The `path` field of a returned [`ReportItem`] is always empty: this
/// function knows only the property, never the form or the control it
/// belongs to. The caller fills in the real path before the item enters
/// the project report.
///
/// The match below has one arm for each variant of [`PropertyValue`] and
/// no wildcard arm: a variant this crate adds later fails to compile here
/// until somebody decides how to write it.
#[must_use]
pub fn format_value(
    value: &PropertyValue,
    is_external: bool,
) -> (FormattedValue, Option<ReportItem>) {
    match value {
        PropertyValue::Byte { name, value } => (
            FormattedValue::Line(format_number_with_enum_comment(name, i64::from(*value))),
            None,
        ),
        PropertyValue::Boolean { value, .. } => {
            (FormattedValue::Line(format_boolean(*value != 0)), None)
        }
        PropertyValue::Integer { name, value } => (
            FormattedValue::Line(format_number_with_enum_comment(name, i64::from(*value))),
            None,
        ),
        PropertyValue::Long { name, value } => {
            let text = match shape_for(name) {
                ValueShape::Colour => format_colour(*value, is_external),
                ValueShape::PlainNumber => format_number_with_enum_comment(name, i64::from(*value)),
            };
            (FormattedValue::Line(text), None)
        }
        PropertyValue::Single { value, .. } => (FormattedValue::Line(value.to_string()), None),
        PropertyValue::Text { value, .. } => match inline_decision(value) {
            InlineDecision::Inline(text) => (FormattedValue::Line(text), None),
            InlineDecision::Resource => (FormattedValue::Resource, None),
        },
        PropertyValue::Position { value, .. } => (
            FormattedValue::Multi(position_coordinates(value).into_iter().collect()),
            None,
        ),
        PropertyValue::Font { value, .. } => (
            FormattedValue::Multi(font_lines(value).into_iter().collect()),
            None,
        ),
        PropertyValue::Blob { .. } => (FormattedValue::Resource, None),
        PropertyValue::BlobUnreadable { offset, .. } => {
            let item = ReportItem {
                path: String::new(),
                confidence: Confidence::Unrecoverable,
                basis: "the file marks a resource blob present, but its own bound check \
                        refused"
                    .to_owned(),
                evidence: vec![Evidence {
                    offset: *offset,
                    structure: "PropertyValue",
                    field: "BlobUnreadable",
                    note: None,
                }],
            };
            (FormattedValue::Omit, Some(item))
        }
        PropertyValue::Undecoded {
            opcode,
            offset,
            control_type,
            ..
        } => {
            let item = ReportItem {
                path: String::new(),
                confidence: Confidence::Unrecoverable,
                basis: format!(
                    "no table entry names a decoder for opcode {opcode} on control type \
                     {control_type}; supply one with --opcode-table"
                ),
                evidence: vec![Evidence {
                    offset: *offset,
                    structure: "PropertyValue",
                    field: "Undecoded",
                    note: None,
                }],
            };
            (FormattedValue::Omit, Some(item))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{
        COLOUR_PROPERTIES, FormattedValue, InlineDecision, font_lines, format_colour,
        format_resource_reference, format_value, inline_decision, position_coordinates,
    };
    use crate::report::Confidence;
    use crate::vb::propstream::{FontBlock, PositionBlock, PropertyValue};

    fn assert_line(value: &PropertyValue) -> String {
        match format_value(value, false).0 {
            FormattedValue::Line(text) => text,
            other => panic!("expected a Line, got {other:?}"),
        }
    }

    #[test]
    fn a_byte_value_gives_a_non_empty_line() {
        let value = PropertyValue::Byte {
            name: "WindowState".to_owned(),
            value: 1,
        };
        assert_eq!(assert_line(&value), "1");
    }

    #[test]
    fn a_false_boolean_gives_the_false_spelling() {
        let value = PropertyValue::Boolean {
            name: "MaxButton".to_owned(),
            value: 0,
        };
        assert_eq!(assert_line(&value), "0   'False");
    }

    #[test]
    fn a_true_boolean_gives_the_true_spelling() {
        let value = PropertyValue::Boolean {
            name: "MultiLine".to_owned(),
            value: -1,
        };
        assert_eq!(assert_line(&value), "-1  'True");
    }

    #[test]
    fn an_integer_value_gives_a_plain_signed_decimal() {
        let value = PropertyValue::Integer {
            name: "AnInt".to_owned(),
            value: -1000,
        };
        assert_eq!(assert_line(&value), "-1000");
    }

    #[test]
    fn a_long_value_gives_a_plain_signed_decimal() {
        let value = PropertyValue::Long {
            name: "ALong".to_owned(),
            value: -70_000,
        };
        assert_eq!(assert_line(&value), "-70000");
    }

    #[test]
    fn a_single_value_gives_a_full_stop_separated_string() {
        let value = PropertyValue::Single {
            name: "ASingle".to_owned(),
            value: 1.5,
        };
        assert_eq!(assert_line(&value), "1.5");
    }

    #[test]
    fn a_text_value_gives_a_double_quoted_line() {
        let value = PropertyValue::Text {
            name: "Caption".to_owned(),
            value: "Tag".to_owned(),
        };
        assert_eq!(assert_line(&value), "\"Tag\"");
    }

    #[test]
    fn a_position_value_gives_left_top_width_height_in_that_order() {
        let value = PositionBlock::Short {
            left: 10,
            top: 20,
            width: 30,
            height: 40,
        };
        let lines = position_coordinates(&value);
        assert_eq!(
            lines,
            [
                ("Left", "10".to_owned()),
                ("Top", "20".to_owned()),
                ("Width", "30".to_owned()),
                ("Height", "40".to_owned()),
            ]
        );
    }

    #[test]
    fn a_position_value_through_format_value_gives_a_multi_of_four() {
        let value = PropertyValue::Position {
            name: "Position".to_owned(),
            value: PositionBlock::Long {
                left: 1,
                top: 2,
                width: 3,
                height: 4,
            },
        };
        match format_value(&value, false).0 {
            FormattedValue::Multi(lines) => assert_eq!(lines.len(), 4),
            other => panic!("expected a Multi, got {other:?}"),
        }
    }

    #[test]
    fn a_font_value_gives_seven_lines_in_the_fixed_order() {
        let font = FontBlock {
            charset: 0,
            italic: false,
            underline: false,
            strikethrough: false,
            weight: 400,
            size_raw: 82_500,
            size_points: 8,
            size_remainder: 2_500,
            name: "Arial".to_owned(),
        };
        let lines = font_lines(&font);
        let names: Vec<&str> = lines.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            [
                "Name",
                "Size",
                "Charset",
                "Weight",
                "Underline",
                "Italic",
                "Strikethrough"
            ]
        );
        assert_eq!(lines[0], ("Name", "\"Arial\"".to_owned()));
        assert_eq!(lines[1], ("Size", "8.25".to_owned()));
        assert_eq!(lines[2], ("Charset", "0".to_owned()));
        assert_eq!(lines[3], ("Weight", "400".to_owned()));
        assert_eq!(lines[4], ("Underline", "0   'False".to_owned()));
        assert_eq!(lines[5], ("Italic", "0   'False".to_owned()));
        assert_eq!(lines[6], ("Strikethrough", "0   'False".to_owned()));
    }

    #[test]
    fn a_font_value_through_format_value_gives_a_multi_of_seven() {
        let value = PropertyValue::Font {
            name: "Font".to_owned(),
            value: FontBlock {
                charset: 0,
                italic: false,
                underline: false,
                strikethrough: false,
                weight: 400,
                size_raw: 97_500,
                size_points: 9,
                size_remainder: 7_500,
                name: "Segoe UI".to_owned(),
            },
        };
        match format_value(&value, false).0 {
            FormattedValue::Multi(lines) => assert_eq!(lines.len(), 7),
            other => panic!("expected a Multi, got {other:?}"),
        }
    }

    #[test]
    fn a_blob_value_gives_the_resource_decision() {
        let value = PropertyValue::Blob {
            name: "Icon".to_owned(),
            offset: 0x100,
            declared_len: 20,
            image_len: 12,
            format: crate::vb::frx::ImageFormat::Unknown(Vec::new()),
            frx_offset: 0,
        };
        assert_eq!(format_value(&value, false).0, FormattedValue::Resource);
    }

    #[test]
    fn an_undecoded_property_gives_omit_and_a_report_item_naming_the_opcode_and_offset() {
        let value = PropertyValue::Undecoded {
            opcode: 200,
            offset: 0x13,
            control_type: "Form".to_owned(),
            bytes_not_read: 6,
        };
        let (formatted, item) = format_value(&value, false);
        assert_eq!(formatted, FormattedValue::Omit);
        let item = item.expect("an omitted undecoded property must carry a report item");
        assert_eq!(item.confidence, Confidence::Unrecoverable);
        assert!(item.basis.contains("200"), "{}", item.basis);
        assert_eq!(item.evidence.len(), 1);
        assert_eq!(item.evidence[0].offset, 0x13);
    }

    #[test]
    fn an_unreadable_blob_gives_omit_and_a_report_item_distinct_from_undecoded() {
        let undecoded = PropertyValue::Undecoded {
            opcode: 1,
            offset: 0x10,
            control_type: "Form".to_owned(),
            bytes_not_read: 1,
        };
        let unreadable = PropertyValue::BlobUnreadable {
            name: "Icon".to_owned(),
            offset: 0x20,
        };
        let (undecoded_formatted, undecoded_item) = format_value(&undecoded, false);
        let (unreadable_formatted, unreadable_item) = format_value(&unreadable, false);
        assert_eq!(undecoded_formatted, FormattedValue::Omit);
        assert_eq!(unreadable_formatted, FormattedValue::Omit);
        let undecoded_item = undecoded_item.expect("undecoded must carry a report item");
        let unreadable_item = unreadable_item.expect("unreadable blob must carry a report item");
        assert_ne!(
            undecoded_item.evidence[0].field, unreadable_item.evidence[0].field,
            "the two omission causes must be distinguishable by field"
        );
        assert_eq!(unreadable_item.evidence[0].offset, 0x20);
    }

    #[test]
    fn no_line_value_from_format_value_is_ever_empty() {
        let values = [
            PropertyValue::Byte {
                name: "B".to_owned(),
                value: 0,
            },
            PropertyValue::Boolean {
                name: "Bo".to_owned(),
                value: 0,
            },
            PropertyValue::Integer {
                name: "I".to_owned(),
                value: 0,
            },
            PropertyValue::Long {
                name: "L".to_owned(),
                value: 0,
            },
            PropertyValue::Single {
                name: "S".to_owned(),
                value: 0.0,
            },
            PropertyValue::Text {
                name: "T".to_owned(),
                value: String::new(),
            },
        ];
        for value in &values {
            if let FormattedValue::Line(text) = format_value(value, false).0 {
                assert!(!text.is_empty(), "{value:?} gave an empty line");
            }
        }
    }

    // --- Task 2: the six measured value grammars --------------------------

    #[test]
    fn colour_properties_holds_exactly_the_three_corpus_proven_names() {
        assert_eq!(COLOUR_PROPERTIES, ["BackColor", "ForeColor", "FillColor"]);
    }

    #[test]
    fn a_colour_on_an_intrinsic_control_writes_eight_upper_case_hex_digits_bracketed() {
        let text = format_colour(-2_147_483_643, false);
        // "&H" (2) + eight hex digits (8) + "&" (1) = 11, not the plan
        // text's own stated twelve: counted directly against
        // "&H80000005&", the literal example FILE-FORMATS.md section 3.6
        // and the corpus both give.
        assert_eq!(text.len(), 11, "{text}");
        assert!(text.starts_with("&H"), "{text}");
        assert!(text.ends_with('&'), "{text}");
        let digits = text
            .strip_prefix("&H")
            .and_then(|rest| rest.strip_suffix('&'))
            .expect("checked starts_with/ends_with above");
        assert_eq!(digits.len(), 8, "{text}");
        assert!(
            digits
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()),
            "{text}"
        );
        assert_eq!(text, "&H80000005&");
    }

    #[test]
    fn the_same_signed_colour_value_on_an_external_control_writes_a_plain_decimal() {
        let text = format_colour(-2_147_483_643, true);
        assert_eq!(text, "-2147483643");
        assert!(!text.contains('&'), "{text}");
    }

    #[test]
    fn a_long_back_color_through_format_value_branches_on_is_external() {
        let value = PropertyValue::Long {
            name: "BackColor".to_owned(),
            value: -2_147_483_643,
        };
        assert_eq!(
            format_value(&value, false).0,
            FormattedValue::Line("&H80000005&".to_owned())
        );
        assert_eq!(
            format_value(&value, true).0,
            FormattedValue::Line("-2147483643".to_owned())
        );
    }

    #[test]
    fn an_enumeration_value_the_table_does_not_name_writes_a_bare_number() {
        let value = PropertyValue::Byte {
            name: "SomeUnknownEnumProperty".to_owned(),
            value: 5,
        };
        let text = assert_line(&value);
        assert_eq!(text, "5");
        assert!(!text.contains('\''), "{text}");
    }

    #[test]
    fn an_enumeration_value_the_table_names_writes_the_number_two_spaces_and_the_name() {
        let value = PropertyValue::Byte {
            name: "BorderStyle".to_owned(),
            value: 1,
        };
        assert_eq!(assert_line(&value), "1  'Fixed Single");
    }

    #[test]
    fn a_string_holding_one_double_quote_is_escaped_by_doubling_it_and_never_a_backslash() {
        let text = super::escape_inline_string("Animate \"Explosion\"");
        assert_eq!(text, "\"Animate \"\"Explosion\"\"\"");
        assert!(!text.contains('\\'), "{text}");
    }

    #[test]
    fn a_float_formats_with_a_full_stop_separator() {
        let value = PropertyValue::Single {
            name: "ASingle".to_owned(),
            value: 9.75,
        };
        let text = assert_line(&value);
        assert!(text.contains('.'), "{text}");
        assert!(!text.contains(','), "{text}");
        assert_eq!(text, "9.75");
    }

    #[test]
    fn the_font_size_reconstructs_the_exact_fraction_from_points_and_remainder() {
        assert_eq!(super::format_font_size(8, 2_500), "8.25");
        assert_eq!(super::format_font_size(9, 7_500), "9.75");
        assert_eq!(super::format_font_size(10, 0), "10");
    }

    /// `corpus/vb6-code/Fire-effect/frmFire.frm` line 4:
    /// `BackColor       =   &H80000005&`. Read through the production
    /// `inspect` path (`vb::opcodes` plan 03-13), `frmFire`'s own
    /// `BackColor` decodes to the signed value `-2_147_483_643`.
    #[test]
    fn the_fast_flames_back_color_line_matches_the_committed_frm_byte_for_byte() {
        let value = PropertyValue::Long {
            name: "BackColor".to_owned(),
            value: -2_147_483_643,
        };
        assert_eq!(assert_line(&value), "&H80000005&");
    }

    /// `corpus/vb6-code/Curves-effect/Curves.frm` line 3:
    /// `BorderStyle     =   1  'Fixed Single`.
    #[test]
    fn the_curves_border_style_line_matches_the_committed_frm_byte_for_byte() {
        let value = PropertyValue::Byte {
            name: "BorderStyle".to_owned(),
            value: 1,
        };
        assert_eq!(assert_line(&value), "1  'Fixed Single");
    }

    // --- Task 3: the inline string rule and the resource reference forms --

    #[test]
    fn a_97_byte_string_goes_inline_and_a_98_byte_string_does_not() {
        let ninety_seven = "a".repeat(97);
        let ninety_eight = "a".repeat(98);
        assert!(matches!(
            inline_decision(&ninety_seven),
            InlineDecision::Inline(_)
        ));
        assert!(matches!(
            inline_decision(&ninety_eight),
            InlineDecision::Resource
        ));
    }

    #[test]
    fn a_string_holding_a_carriage_return_and_line_feed_never_goes_inline() {
        assert_eq!(inline_decision("a\r\nb"), InlineDecision::Resource);
        assert_eq!(inline_decision("short\n"), InlineDecision::Resource);
        assert_eq!(inline_decision("short\r"), InlineDecision::Resource);
    }

    #[test]
    fn a_text_property_over_the_threshold_gives_the_resource_decision_through_format_value() {
        let value = PropertyValue::Text {
            name: "Caption".to_owned(),
            value: "a".repeat(98),
        };
        assert_eq!(format_value(&value, false).0, FormattedValue::Resource);
    }

    #[test]
    fn an_offset_of_116_formats_to_four_upper_case_hex_digits_with_a_leading_zero() {
        assert_eq!(
            format_resource_reference("frmFire.frx", 116, false),
            "\"frmFire.frx\":0074"
        );
    }

    #[test]
    fn an_offset_needing_five_digits_formats_to_five() {
        assert_eq!(
            format_resource_reference("Big.frx", 0x1_2345, false),
            "\"Big.frx\":12345"
        );
    }

    #[test]
    fn the_long_string_reference_form_differs_from_the_binary_form_by_one_leading_character() {
        let binary = format_resource_reference("Gradient.frx", 0, false);
        let long = format_resource_reference("Gradient.frx", 0, true);
        assert_eq!(binary, "\"Gradient.frx\":0000");
        assert_eq!(long, "$\"Gradient.frx\":0000");
        assert_eq!(long.len(), binary.len().saturating_add(1));
        assert_eq!(format!("${binary}"), long);
    }

    #[test]
    fn a_resource_file_name_holding_a_space_is_still_quoted() {
        let text = format_resource_reference("Main Editor.frx", 0, false);
        assert_eq!(text, "\"Main Editor.frx\":0000");
    }
}
