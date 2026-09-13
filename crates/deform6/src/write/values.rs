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

/// Gives the seven `Font` block lines in the fixed order the IDE always
/// writes them: `Name`, `Size`, `Charset`, `Weight`, `Underline`,
/// `Italic`, `Strikethrough`. `FILE-FORMATS.md` section 3.9: 235 of 235
/// blocks in the corpus hold exactly these seven keys, in exactly this
/// order.
///
/// `Size` is written from [`FontBlock::size_points`] alone here: the exact
/// fraction [`FontBlock::size_remainder`] carries is plan 04-02 task 2's
/// own job (`format_font_size`), one of the six measured value grammars
/// this task's own scope does not yet cover. The order and the other six
/// keys are already final.
fn font_lines(value: &FontBlock) -> [(&'static str, String); 7] {
    [
        ("Name", escape_inline_string(&value.name)),
        ("Size", value.size_points.to_string()),
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
/// The `path` field of a returned [`ReportItem`] is always empty: this
/// function knows only the property, never the form or the control it
/// belongs to. The caller fills in the real path before the item enters
/// the project report.
///
/// The match below has one arm for each variant of [`PropertyValue`] and
/// no wildcard arm: a variant this crate adds later fails to compile here
/// until somebody decides how to write it. Most of the arms below give a
/// plain, rough shape for now: the exact colour, enumeration, string and
/// font grammars are this plan's own task 2 and task 3. The undecoded and
/// unreadable-blob arms are this task's own job, and are already final.
#[must_use]
pub fn format_value(value: &PropertyValue) -> (FormattedValue, Option<ReportItem>) {
    match value {
        PropertyValue::Byte { value, .. } => (FormattedValue::Line(value.to_string()), None),
        PropertyValue::Boolean { value, .. } => {
            (FormattedValue::Line(format_boolean(*value != 0)), None)
        }
        PropertyValue::Integer { value, .. } => (FormattedValue::Line(value.to_string()), None),
        PropertyValue::Long { value, .. } => (FormattedValue::Line(value.to_string()), None),
        PropertyValue::Single { value, .. } => (FormattedValue::Line(value.to_string()), None),
        PropertyValue::Text { value, .. } => {
            (FormattedValue::Line(escape_inline_string(value)), None)
        }
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
    use super::{FormattedValue, font_lines, format_value, position_coordinates};
    use crate::report::Confidence;
    use crate::vb::propstream::{FontBlock, PositionBlock, PropertyValue};

    fn assert_line(value: &PropertyValue) -> String {
        match format_value(value).0 {
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
        match format_value(&value).0 {
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
        match format_value(&value).0 {
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
        assert_eq!(format_value(&value).0, FormattedValue::Resource);
    }

    #[test]
    fn an_undecoded_property_gives_omit_and_a_report_item_naming_the_opcode_and_offset() {
        let value = PropertyValue::Undecoded {
            opcode: 200,
            offset: 0x13,
            control_type: "Form".to_owned(),
            bytes_not_read: 6,
        };
        let (formatted, item) = format_value(&value);
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
        let (undecoded_formatted, undecoded_item) = format_value(&undecoded);
        let (unreadable_formatted, unreadable_item) = format_value(&unreadable);
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
            if let FormattedValue::Line(text) = format_value(value).0 {
                assert!(!text.is_empty(), "{value:?} gave an empty line");
            }
        }
    }
}
