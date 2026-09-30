//! `derive-opcode-table`: writes the table [`deform6::vb::opcodes::OpcodeTable::parse`]
//! reads, from a type library a user owns a lawful copy of.
//!
//! It reads `VB6.OLB` with this crate's own reader of a type library,
//! `crate::msft`, so it runs on each host. Semi VB Decompiler binds the
//! same library over COM, and `STRUCTURES.md` section 8.5 gives its rule:
//! the opcode of a property is its member id less `0x10000`. The payload
//! comes from the declared type of the property, and from the widths that
//! the corpus measured where the declared type does not decide it.
//!
//! Per `AGENTS.md`'s "What may enter this repository": this tool is
//! DeForm6's own original code, and it is committed. The table it writes is
//! calculated from a third party file the author does not own, so it is
//! never committed; `.gitignore` excludes its default output path.

use deform6::vb::opcodes::PayloadType;

use crate::msft::{Function, PARAMFLAG_FRETVAL, TypeInfo, parse};
use std::collections::BTreeMap;

/// The tool's default output path, relative to the workspace root.
///
/// `.gitignore` excludes exactly this path. Nested one directory down so
/// the pattern's own extraction (a plan verification step greps for a line
/// starting with a non-`/` character, since `git check-ignore` treats a
/// leading `/` argument as an OS-absolute path rather than a repo-relative
/// one) has a directory segment to anchor on instead.
pub const DEFAULT_OUTPUT_PATH: &str = "derived/opcode-table.toml";

/// One row of a serialized table, matching the shape
/// [`toml::to_string`] writes it in.
#[derive(serde::Serialize)]
struct SerRow {
    name: String,
    payload: &'static str,
}

/// Writes the TOML table format `OpcodeTable::parse` reads: one table per
/// control type, each key an opcode, each value a `{ name, payload }` row.
///
/// Takes the tuples directly, in the order
/// `(control_type, opcode, payload_type, property_name)`, so a test can
/// pass rows built in memory and assert on the result, rather than reading
/// them from a file.
///
/// The `toml` crate owns every quoting and escaping decision a property
/// name needs; this function builds a plain, serializable document and
/// hands it to [`toml::to_string`] rather than formatting text by hand,
/// so a name holding a quote or a backslash is never this function's
/// problem to get right.
#[must_use]
pub fn serialize_table(rows: &[(u8, u8, PayloadType, &str)]) -> String {
    let mut doc: BTreeMap<String, BTreeMap<String, SerRow>> = BTreeMap::new();
    for &(control_type, opcode, payload, name) in rows {
        doc.entry(control_type.to_string()).or_default().insert(
            opcode.to_string(),
            SerRow {
                name: name.to_owned(),
                payload: payload_word(payload),
            },
        );
    }
    // This document is built entirely from Rust `String`s and a fixed set
    // of static payload words this function itself chose; nothing in its
    // shape can trigger `toml::to_string`'s only failure modes (a NaN or
    // infinite float, neither of which this document ever contains). The
    // fallback below is never exercised by a real call; it exists so this
    // function stays infallible rather than reaching for `unwrap`, which
    // this workspace's lint wall denies.
    toml::to_string(&doc).unwrap_or_default()
}

/// Gives the TOML string form of a payload type, matching
/// `deform6::vb::opcodes::PayloadType`'s own `serde::Deserialize` derive,
/// which reads a unit variant back from its bare Rust identifier.
const fn payload_word(payload: PayloadType) -> &'static str {
    match payload {
        PayloadType::Byte => "Byte",
        PayloadType::Boolean => "Boolean",
        PayloadType::Integer => "Integer",
        PayloadType::Long => "Long",
        PayloadType::Single => "Single",
        PayloadType::Text => "Text",
        PayloadType::Picture => "Picture",
        PayloadType::Font => "Font",
        PayloadType::Position => "Position",
    }
}

/// The interface of each intrinsic control in `VB6.OLB`, and the `cType`
/// of the control in the form stream.
const CONTROL_TYPES: &[(&str, u8)] = &[
    ("_PictureBox", 0),
    ("_Label", 1),
    ("_TextBox", 2),
    ("_Frame", 3),
    ("_CommandButton", 4),
    ("_CheckBox", 5),
    ("_OptionButton", 6),
    ("_ComboBox", 7),
    ("_ListBox", 8),
    ("_HScrollBar", 9),
    ("_VScrollBar", 10),
    ("_Timer", 11),
    ("_Form", 13),
    ("_DriveListBox", 16),
    ("_DirListBox", 17),
    ("_FileListBox", 18),
    ("_Menu", 19),
    ("_MDIForm", 20),
    ("_Shape", 22),
    ("_Line", 23),
    ("_Image", 24),
    ("_Data", 37),
    ("_OLE", 38),
];

/// The member id of the first property of a control interface. The opcode
/// of a property in the form stream is its member id less this value.
const FIRST_PROPERTY_ID: u32 = 0x1_0000;

/// The `INVOKE_PROPERTYGET` kind of a function.
const PROPERTY_GET: u8 = 2;

/// The variant types that a property of a control declares.
const VT_I2: u16 = 2;
const VT_I4: u16 = 3;
const VT_R4: u16 = 4;
const VT_BSTR: u16 = 8;
const VT_BOOL: u16 = 11;

/// The `cType` of `Form`, `MDIForm` and `Line`.
const CT_FORM: u8 = 13;
const CT_MDIFORM: u8 = 20;
const CT_LINE: u8 = 23;

/// The `cType` of the two scroll bars.
const CT_HSCROLLBAR: u8 = 9;
const CT_VSCROLLBAR: u8 = 10;

/// The `cType` of `CheckBox`.
const CT_CHECKBOX: u8 = 5;

/// The properties of the type `Integer` that the form stream holds in one
/// byte: each is an enumeration. `VB6.OLB` declares each as a `short`, as it
/// declares a number, so the type library cannot tell the two apart. The
/// corpus measured the first eleven: with one byte, the value and each
/// property after it agree with the source `.frm`. `STRUCTURES.md` section
/// 8.5.1 gives the other eight as one byte, from Semi VB Decompiler.
const BYTE_PROPERTIES: &[&str] = &[
    "Alignment",
    "Appearance",
    "BackStyle",
    "BorderStyle",
    "FillStyle",
    "MousePointer",
    "MultiSelect",
    "ScaleMode",
    "ScrollBars",
    "Style",
    "WindowState",
    "DragMode",
    "DrawMode",
    "DrawStyle",
    "LinkMode",
    "OLEDragMode",
    "OLEDropMode",
    "PaletteMode",
    "StartUpPosition",
];

/// The properties of the type `Integer` that the form stream holds in two
/// bytes: each is a number. The corpus measured each one.
const INTEGER_PROPERTIES: &[&str] = &[
    "DrawWidth",
    "Index",
    "LargeChange",
    "Max",
    "Min",
    "TabIndex",
];

/// The properties that hold a picture.
const PICTURE_PROPERTIES: &[&str] = &[
    "DisabledPicture",
    "DownPicture",
    "DragIcon",
    "Icon",
    "MouseIcon",
    "Picture",
];

/// The properties that the position block of a control holds after `Left`.
const POSITION_REST: &[&str] = &["Top", "Width", "Height"];

/// The rows that the form stream holds with no member in `VB6.OLB`, by the
/// `cType` of their control. A `Timer` holds its place on the form, `Left`
/// and `Top`, as opcodes 7 and 8 with four bytes each; `_Timer` has no such
/// members. The corpus measured it on each of its six timers.
const EXTRA_ROWS: &[(u8, u8, PayloadType, &str)] = &[
    (11, 7, PayloadType::Long, "Left"),
    (11, 8, PayloadType::Long, "Top"),
];

/// Gives the payload of the property `name` of the type `vt` of the control
/// `control_type`, or `None` when the form stream does not hold the
/// property as its own opcode, or when its width is not known.
///
/// A width that is not known gives no row. The walk of the form stream then
/// stops at the opcode and reports it, and it never guesses a width: a
/// wrong width moves each property after it.
fn payload(control_type: u8, name: &str, vt: u16) -> Option<PayloadType> {
    let form = control_type == CT_FORM || control_type == CT_MDIFORM;
    if name == "Name" {
        // The header of the control block holds the name.
        return None;
    }
    if name == "Left" || POSITION_REST.contains(&name) {
        // The position block of a control starts at the opcode of `Left`,
        // and it holds all four. A form holds its client area instead.
        return (name == "Left" && !form).then_some(PayloadType::Position);
    }
    if name == "Font" {
        return Some(PayloadType::Font);
    }
    if PICTURE_PROPERTIES.contains(&name) {
        return Some(PayloadType::Picture);
    }
    match vt {
        VT_BSTR => Some(PayloadType::Text),
        VT_I4 => Some(PayloadType::Long),
        // `Line` holds `X1`, `Y1`, `X2` and `Y2` as whole twips in four
        // bytes, although `VB6.OLB` declares each as a `Single`. The corpus
        // measured it: 120 reads as 1.68e-43 when it is read as a `Single`.
        VT_R4 if control_type == CT_LINE => Some(PayloadType::Long),
        VT_R4 => Some(PayloadType::Single),
        VT_BOOL => Some(PayloadType::Boolean),
        VT_I2 if name == "Value" => match control_type {
            CT_HSCROLLBAR | CT_VSCROLLBAR => Some(PayloadType::Integer),
            CT_CHECKBOX => Some(PayloadType::Byte),
            _ => None,
        },
        VT_I2 if BYTE_PROPERTIES.contains(&name) => Some(PayloadType::Byte),
        VT_I2 if INTEGER_PROPERTIES.contains(&name) => Some(PayloadType::Integer),
        _ => None,
    }
}

/// Gives the variant type of the property that the property get `function`
/// reads: the type that its result parameter points to, or its return type.
fn property_type(function: &Function) -> u16 {
    function
        .parameters
        .iter()
        .find(|parameter| parameter.flags & PARAMFLAG_FRETVAL != 0)
        .map_or(function.return_vt, |parameter| {
            parameter.pointee.unwrap_or(parameter.vt)
        })
}

/// Gives a row `(control_type, opcode, payload, name)` for each property of
/// each control interface of `infos` that [`payload`] gives a payload for.
fn table_rows(infos: &[TypeInfo]) -> Vec<(u8, u8, PayloadType, String)> {
    let mut rows = Vec::new();
    for info in infos {
        let Some(control_type) = CONTROL_TYPES
            .iter()
            .find(|(name, _)| info.name.as_deref() == Some(*name))
            .map(|(_, control_type)| *control_type)
        else {
            continue;
        };
        for function in &info.functions {
            if function.invoke_kind != PROPERTY_GET {
                continue;
            }
            let Some(opcode) = function
                .member_id
                .checked_sub(FIRST_PROPERTY_ID)
                .and_then(|opcode| u8::try_from(opcode).ok())
            else {
                continue;
            };
            let Some(name) = &function.name else {
                continue;
            };
            if let Some(payload) = payload(control_type, name, property_type(function)) {
                rows.push((control_type, opcode, payload, name.clone()));
            }
        }
        rows.extend(EXTRA_ROWS.iter().filter(|row| row.0 == control_type).map(
            |&(control_type, opcode, payload, name)| {
                (control_type, opcode, payload, name.to_owned())
            },
        ));
    }
    rows
}

/// Runs `derive-opcode-table <olb> [--out <path>]`. Gives the process exit
/// code, matching `xtask::run`'s own convention.
pub fn derive_opcode_table(args: &[String]) -> i32 {
    let (input, out) = match args {
        [input] => (input.as_str(), DEFAULT_OUTPUT_PATH),
        [input, flag, out] if flag == "--out" => (input.as_str(), out.as_str()),
        _ => {
            eprintln!("usage: cargo run -p xtask -- derive-opcode-table <olb> [--out <path>]");
            return 1;
        }
    };
    match derive_file(input, out) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(err) => {
            eprintln!("derive-opcode-table: {err}");
            1
        }
    }
}

/// Reads the type library `input`, derives the table and writes it to `out`.
fn derive_file(input: &str, out: &str) -> Result<String, String> {
    let bytes = std::fs::read(input).map_err(|err| format!("reading {input}: {err}"))?;
    let infos = parse(&bytes).map_err(|err| format!("{input}: {err}"))?;
    let rows = table_rows(&infos);
    if rows.is_empty() {
        return Err(format!(
            "{input} holds no control interface of Visual Basic 6"
        ));
    }
    let borrowed: Vec<(u8, u8, PayloadType, &str)> = rows
        .iter()
        .map(|(control_type, opcode, payload, name)| {
            (*control_type, *opcode, *payload, name.as_str())
        })
        .collect();
    if let Some(parent) = std::path::Path::new(out).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("creating {}: {err}", parent.display()))?;
    }
    std::fs::write(out, serialize_table(&borrowed))
        .map_err(|err| format!("writing {out}: {err}"))?;
    Ok(format!("wrote {out}: {} properties", rows.len()))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{derive_opcode_table, payload_word, serialize_table, table_rows};
    use crate::msft::{Function, Kind, PARAMFLAG_FRETVAL, Parameter, TypeInfo};
    use deform6::vb::opcodes::{OpcodeTable, PayloadType};

    /// A property get of the member id `0x10000 + opcode` whose result
    /// parameter points to the type `vt`.
    fn get(name: &str, opcode: u32, vt: u16) -> Function {
        Function {
            name: Some(name.to_owned()),
            vtable_offset: 0x40,
            invoke_kind: 2,
            parameters: vec![Parameter {
                name: None,
                vt: 26,
                pointee: Some(vt),
                flags: PARAMFLAG_FRETVAL,
                user_type: None,
            }],
            ordinal: None,
            result_type: None,
            member_id: opcode.saturating_add(0x1_0000),
            return_vt: 25,
        }
    }

    /// The interface `name` with the functions `functions`.
    fn interface(name: &str, functions: Vec<Function>) -> TypeInfo {
        TypeInfo {
            kind: Kind::Interface,
            name: Some(name.to_owned()),
            guid: None,
            vtable_size: 0x44,
            functions,
        }
    }

    /// The payload of the row of `rows` for `control_type` and `opcode`.
    fn row(
        rows: &[(u8, u8, PayloadType, String)],
        control_type: u8,
        opcode: u8,
    ) -> Option<PayloadType> {
        rows.iter()
            .find(|row| row.0 == control_type && row.1 == opcode)
            .map(|row| row.2)
    }

    #[test]
    fn a_property_get_gives_its_member_id_less_0x10000_as_its_opcode_and_its_type_as_its_payload() {
        let mut set = get("Caption", 1, 3);
        set.invoke_kind = 4;
        let rows = table_rows(&[
            interface(
                "_CommandButton",
                vec![
                    get("Caption", 1, 8),
                    get("BackColor", 3, 3),
                    get("Left", 4, 4),
                    get("Top", 5, 4),
                    get("Visible", 9, 11),
                    get("MousePointer", 10, 2),
                    get("TabIndex", 17, 2),
                    get("Frobnicate", 18, 2),
                    get("Font", 29, 26),
                    get("Picture", 34, 26),
                    set,
                ],
            ),
            interface("_Unknown", vec![get("Caption", 1, 8)]),
        ]);
        assert_eq!(row(&rows, 4, 1), Some(PayloadType::Text));
        assert_eq!(row(&rows, 4, 3), Some(PayloadType::Long));
        assert_eq!(row(&rows, 4, 4), Some(PayloadType::Position));
        assert_eq!(row(&rows, 4, 5), None, "the position block holds Top");
        assert_eq!(row(&rows, 4, 9), Some(PayloadType::Boolean));
        assert_eq!(row(&rows, 4, 10), Some(PayloadType::Byte));
        assert_eq!(row(&rows, 4, 17), Some(PayloadType::Integer));
        assert_eq!(
            row(&rows, 4, 18),
            None,
            "no width is known for this Integer"
        );
        assert_eq!(row(&rows, 4, 29), Some(PayloadType::Font));
        assert_eq!(row(&rows, 4, 34), Some(PayloadType::Picture));
        assert_eq!(
            rows.len(),
            8,
            "a property let and an unknown interface give no row"
        );
    }

    #[test]
    fn a_line_holds_its_coordinates_as_long_and_a_form_holds_no_position_block() {
        let rows = table_rows(&[
            interface("_Line", vec![get("X1", 5, 4)]),
            interface("_Form", vec![get("Left", 4, 4), get("ScaleHeight", 20, 4)]),
            interface("_HScrollBar", vec![get("Value", 7, 2)]),
            interface("_CheckBox", vec![get("Value", 7, 2)]),
        ]);
        assert_eq!(row(&rows, 23, 5), Some(PayloadType::Long));
        assert_eq!(row(&rows, 13, 4), None);
        assert_eq!(row(&rows, 13, 20), Some(PayloadType::Single));
        assert_eq!(row(&rows, 9, 7), Some(PayloadType::Integer));
        assert_eq!(row(&rows, 5, 7), Some(PayloadType::Byte));
    }

    #[test]
    fn a_timer_gives_its_place_as_two_long_rows_that_vb6_olb_does_not_name() {
        assert_eq!(table_rows(&[]).len(), 0);
        let rows = table_rows(&[interface("_Timer", vec![get("Interval", 3, 3)])]);
        assert_eq!(row(&rows, 11, 3), Some(PayloadType::Long));
        assert_eq!(row(&rows, 11, 7), Some(PayloadType::Long));
        assert_eq!(row(&rows, 11, 8), Some(PayloadType::Long));
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn derive_opcode_table_with_no_type_library_gives_the_usage_and_exit_code_1() {
        assert_eq!(derive_opcode_table(&[]), 1);
        assert_eq!(
            derive_opcode_table(&["/no/such/VB6.OLB".to_owned()]),
            1,
            "a file that cannot be read"
        );
    }

    #[test]
    fn an_empty_list_serializes_to_a_table_that_parses_to_zero_rows() {
        let rendered = serialize_table(&[]);
        let table = OpcodeTable::parse(rendered.as_bytes()).unwrap();
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn one_row_serializes_and_parses_back_to_the_same_entry() {
        let rows = [(13_u8, 31_u8, PayloadType::Byte, "DrawMode")];
        let rendered = serialize_table(&rows);
        let table = OpcodeTable::parse(rendered.as_bytes()).unwrap();
        let entry = table.lookup(13, 31).unwrap();
        assert_eq!(entry.name, "DrawMode");
        assert_eq!(entry.payload, PayloadType::Byte);
    }

    /// A property name that holds a character the format must quote or
    /// escape: a double quote and a backslash, either of which breaks a
    /// hand-written `"{name}"` format string that does not escape them.
    #[test]
    fn a_name_holding_a_quote_and_a_backslash_round_trips_intact() {
        let name = "Weird\"Name\\Here";
        let rows = [(1_u8, 5_u8, PayloadType::Text, name)];
        let rendered = serialize_table(&rows);
        let table = OpcodeTable::parse(rendered.as_bytes()).unwrap();
        assert_eq!(table.lookup(1, 5).unwrap().name, name);
    }

    /// A round trip through this crate's own `serialize_table` and
    /// `deform6`'s own `OpcodeTable::parse` is self agreement, not
    /// verification: it proves only that the two agree with each other.
    /// The real check on the safe-provenance subset is plan 03-10's
    /// differential gate, against the committed `.frm` source.
    #[test]
    fn a_multi_row_multi_control_type_table_round_trips_through_both_functions() {
        let rows = [
            (13_u8, 31_u8, PayloadType::Byte, "DrawMode"),
            (4_u8, 31_u8, PayloadType::Byte, "Appearance"),
            (1_u8, 31_u8, PayloadType::Byte, "BackStyle"),
            (4_u8, 4_u8, PayloadType::Position, "Position"),
        ];
        let rendered = serialize_table(&rows);
        let table = OpcodeTable::parse(rendered.as_bytes()).unwrap();
        assert_eq!(table.len(), rows.len());
        for &(control_type, opcode, payload, name) in &rows {
            let entry = table.lookup(control_type, opcode).unwrap();
            assert_eq!(entry.name, name);
            assert_eq!(entry.payload, payload);
        }
    }

    #[test]
    fn payload_word_gives_the_exact_rust_identifier_for_every_variant() {
        assert_eq!(payload_word(PayloadType::Byte), "Byte");
        assert_eq!(payload_word(PayloadType::Boolean), "Boolean");
        assert_eq!(payload_word(PayloadType::Integer), "Integer");
        assert_eq!(payload_word(PayloadType::Long), "Long");
        assert_eq!(payload_word(PayloadType::Single), "Single");
        assert_eq!(payload_word(PayloadType::Text), "Text");
        assert_eq!(payload_word(PayloadType::Picture), "Picture");
        assert_eq!(payload_word(PayloadType::Font), "Font");
        assert_eq!(payload_word(PayloadType::Position), "Position");
    }
}
