//! The opcode table format and the optional `--opcode-table` run time
//! loader, plus the small safe-provenance subset built from prior art read
//! for facts, per `AGENTS.md`'s Prior Art rule.
//!
//! `03-CONTEXT.md` decision D-01 binds this module: the table itself is
//! never committed, because a table calculated from Microsoft's `VB6.OLB`
//! is a derived fixture `AGENTS.md` bars.
//!
//! # One representation, two sources
//!
//! [`OpcodeTable`] is the one representation. [`OpcodeTable::builtin`]
//! builds it from two kinds of constant: rows transcribed from
//! `STRUCTURES.md` section 8.5.1, which itself cites Semi VB Decompiler's
//! own authored source comments, never `VB6.OLB`; and, since plan 03-13,
//! rows this repository measured directly against its own corpus, per
//! [`CORPUS_MEASURED`]'s own doc comment, D-01's other allowed source when
//! no prior art states a fact. [`OpcodeTable::parse`] builds it from bytes a
//! user supplies at run time, having built their own table on their own
//! machine with `xtask derive-opcode-table`. [`OpcodeTable::lookup`] is the
//! one method both call, and it gives the same answer whichever constructor
//! built the table it is called on.
//!
//! # The table format: TOML, one table per control type, keyed by opcode
//!
//! This plan's own checkpoint chose TOML over a hand-rolled line format and
//! a binary format. `toml` 1.1.5 is already a workspace dependency, and
//! `serde` already carries the `derive` feature this module uses. A table
//! looks like:
//!
//! ```toml
//! [13]
//! 31 = { name = "DrawMode", payload = "Byte" }
//! ```
//!
//! The outer key is the control type, the inner key is the opcode, each
//! read through serde's own key deserializer as a [`u8`]. A control type or
//! an opcode above 255 therefore fails inside that deserializer itself,
//! with the offending key's own position in the file, and this module adds
//! no hand-written bound check that could drift from it.
//!
//! # Never open, copy, or derive from `VB6.OLB`
//!
//! Every fact `OpcodeTable::builtin` carries came from reading Semi VB
//! Decompiler's own authored source comments (`STRUCTURES.md` section 8.5
//! cites `ReturnGuiOpcode`, `ReturnDataType` and `GetControlSize` by name),
//! never from opening the copy of Microsoft's type library that SVBD's own
//! repository commits. `AGENTS.md`'s Prior Art rule permits reading a tool
//! for facts and forbids copying its code; this subset does the former and
//! never the latter.
//!
//! Plan 03-02 fills this module. It serves FRM-03.

use std::collections::{BTreeMap, HashMap};

/// The payload type of one property, per `STRUCTURES.md` section 8.5's
/// payload-width table.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Deserialize)]
pub enum PayloadType {
    /// One byte.
    Byte,
    /// Two bytes, emitted as `-1` or `0` in the `.frm`.
    Boolean,
    /// Two bytes.
    Integer,
    /// Four bytes.
    Long,
    /// Four bytes.
    Single,
    /// `2 + n + 1` bytes: a `u16` length, `n` bytes, a trailing NUL. The
    /// payload's own declared length decides how far it runs.
    Text,
    /// `4 + 8 + m` bytes, or 4 bytes alone when the length field is `-1`.
    /// The payload's own bytes decide how far it runs.
    Picture,
    /// `11 + name` bytes: the `BeginProperty Font ... EndProperty` block,
    /// `STRUCTURES.md` section 8.5.2. The payload's own name-length byte
    /// decides how far it runs.
    Font,
    /// 8 bytes, or 16 bytes when the first `i16` is `-32768`. The payload's
    /// own first two bytes decide which.
    Position,
}

impl PayloadType {
    /// Gives the fixed byte width of this payload type, or `None` when the
    /// payload's own bytes decide the width, which is the case for
    /// [`PayloadType::Text`], [`PayloadType::Picture`], [`PayloadType::Font`]
    /// and [`PayloadType::Position`].
    #[must_use]
    pub const fn fixed_width(self) -> Option<u32> {
        match self {
            Self::Byte => Some(1),
            Self::Boolean | Self::Integer => Some(2),
            Self::Long | Self::Single => Some(4),
            Self::Text | Self::Picture | Self::Font | Self::Position => None,
        }
    }
}

/// One property an opcode names: its name, its payload type, and where the
/// fact came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpcodeEntry {
    /// The property this opcode names, such as `"DrawMode"`.
    pub name: String,
    /// How many bytes the payload occupies, and how to read it.
    pub payload: PayloadType,
    /// Where this fact came from.
    ///
    /// A builtin entry names the exact SVBD function `STRUCTURES.md`
    /// section 8.5 cites for it. A parsed entry names only that it came
    /// from a user supplied table, per [`PARSED_TABLE_SOURCE`]: a table a
    /// user built on their own machine names no SVBD function, because it
    /// did not come from one.
    pub source: &'static str,
}

/// A table a user supplied is malformed, and where.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {message}")]
pub struct TableError {
    /// The 1-indexed line the parser was reading when it refused.
    pub line: usize,
    /// What the parser expected there.
    pub message: String,
}

/// A map from `(control type, opcode)` to the property that opcode names.
///
/// Built either by [`OpcodeTable::builtin`], from the small safe-provenance
/// subset compiled into this crate, or by [`OpcodeTable::parse`], from
/// bytes a user supplies. Both give a value of this one type, and
/// [`OpcodeTable::lookup`] behaves the same over either one.
#[derive(Clone, Debug, Default)]
pub struct OpcodeTable {
    entries: HashMap<(u8, u8), OpcodeEntry>,
}

impl OpcodeTable {
    /// Gives the entry the `(control_type, opcode)` pair names, or `None`
    /// when the table holds no entry for that pair.
    ///
    /// `lookup` never gives a neighbouring entry and never gives a
    /// default. A `None` here is the caller's signal to report the
    /// property present, at its byte offset, undecoded, per the honest-gap
    /// pattern FRM-04 already established for an OCX property blob.
    #[must_use]
    pub fn lookup(&self, control_type: u8, opcode: u8) -> Option<&OpcodeEntry> {
        self.entries.get(&(control_type, opcode))
    }

    /// Gives the number of entries this table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Tells whether this table holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Builds the table from the small safe-provenance subset: two kinds of
    /// row, each naming its own source. `STRUCTURES.md` section 8.5.1's
    /// Form, MDIForm, CommandButton, Label and ListBox rows are transcribed
    /// from Semi VB Decompiler's own authored source comments and nothing
    /// else. [`FORM_CORPUS_ROWS`] carries three further Form/MDIForm rows
    /// plan 03-13 measured directly against this repository's own corpus
    /// instead, per [`CORPUS_MEASURED`]'s own doc comment: `STRUCTURES.md`
    /// names no payload width for these three, so there was nothing to
    /// transcribe, and a corpus measurement is D-01's other allowed source.
    ///
    /// A handful of section 8.5.1's own rows are left out on purpose: the
    /// `ScaleMode` opcode (25) and the `ClientLeft/Top/Width/Height` opcode
    /// (53) on Form are conditional, multi-shape payloads that fit none of
    /// [`PayloadType`]'s nine variants, `ListBox`'s `List` opcode (20) is a
    /// variable count of length-prefixed strings that fits none of them
    /// either, and the three opcodes SVBD's own comment marks "consume 1
    /// byte, no output" (0, 98, 99 on Form) name no property at all. A
    /// wrong width transcribed here would silently misalign every property
    /// that follows it in the stream, so this subset carries only the rows
    /// whose payload is a single, unconditional [`PayloadType`], and reports
    /// every opcode it leaves out as absent, exactly like any opcode a user
    /// supplied table does not name.
    #[must_use]
    pub fn builtin() -> Self {
        let mut entries = HashMap::new();
        for &control_type in &[CT_FORM, CT_MDIFORM] {
            insert_builtin_rows(&mut entries, control_type, FORM_ROWS, SVBD_OPCODE_AND_TYPE);
            insert_builtin_rows(
                &mut entries,
                control_type,
                FORM_CORPUS_ROWS,
                CORPUS_MEASURED,
            );
        }
        insert_builtin_rows(
            &mut entries,
            CT_COMMAND_BUTTON,
            COMMAND_BUTTON_ROWS,
            SVBD_OPCODE_AND_TYPE,
        );
        insert_builtin_rows(&mut entries, CT_LABEL, LABEL_ROWS, SVBD_OPCODE_AND_TYPE);
        insert_builtin_rows(
            &mut entries,
            CT_LISTBOX,
            LIST_BOX_ROWS,
            SVBD_OPCODE_AND_TYPE,
        );
        Self { entries }
    }

    /// Parses a table a user built on their own machine, in the TOML shape
    /// this module's doc comment gives.
    ///
    /// Takes a byte slice, not a path: this library opens no file. The
    /// command line crate reads the file and hands over the bytes, the
    /// same split `lib.rs` documents for `inspect`.
    ///
    /// `TableError` names the 1-indexed line the parser was reading and
    /// what it expected there, for a malformed row and for a control type
    /// or an opcode above 255, which is a range error inside serde's own
    /// key deserializer: `cType` and the opcode are each one byte, and
    /// `[u8]` as the map key type is what enforces that, not a hand-written
    /// check that could drift from it.
    pub fn parse(bytes: &[u8]) -> Result<Self, TableError> {
        let text = std::str::from_utf8(bytes).map_err(|err| TableError {
            line: line_at(bytes, err.valid_up_to()),
            message: "the table is not valid UTF-8".to_owned(),
        })?;

        let raw: RawTable = toml::from_str(text).map_err(|err| TableError {
            line: err
                .span()
                .map_or(1, |span| line_at(text.as_bytes(), span.start)),
            message: err.message().to_owned(),
        })?;

        let mut entries = HashMap::new();
        for (control_type, opcodes) in raw {
            for (opcode, row) in opcodes {
                entries.insert(
                    (control_type, opcode),
                    OpcodeEntry {
                        name: row.name,
                        payload: row.payload,
                        source: PARSED_TABLE_SOURCE,
                    },
                );
            }
        }
        Ok(Self { entries })
    }
}

/// One row of a user supplied table, before it is folded into an
/// [`OpcodeEntry`]. `payload` deserializes straight into [`PayloadType`]:
/// serde's own string-to-unit-variant matching handles `payload = "Byte"`
/// with no custom code here.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    name: String,
    payload: PayloadType,
}

/// The shape [`toml::from_str`] reads directly: one table per control
/// type, each keyed by its opcode. Both keys deserialize through [`u8`]
/// itself, so a value above 255 fails inside serde's own key deserializer,
/// carrying that key's own position, before [`OpcodeTable::parse`] ever
/// sees the value.
type RawTable = BTreeMap<u8, BTreeMap<u8, RawEntry>>;

/// [`OpcodeEntry::source`] for every row [`OpcodeTable::parse`] reads. A
/// user supplied table names no SVBD function, because it did not come
/// from one.
const PARSED_TABLE_SOURCE: &str = "a user supplied table";

/// Counts to the 1-indexed line number byte offset `at` falls on, within
/// `bytes`. `AGENTS.md` asks an error to name the byte offset and what the
/// code expected there; for text input read a line at a time, the line
/// number is the more useful "where", and both this module's own error
/// paths (invalid UTF-8, a `toml` parse or key error) give a byte offset
/// this turns into one.
fn line_at(bytes: &[u8], at: usize) -> usize {
    let end = at.min(bytes.len());
    let prefix = bytes.get(..end).unwrap_or(&[]);
    prefix
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        .saturating_add(1)
}

/// `cType` values `STRUCTURES.md` section 8.4.1 gives, for the four
/// control types (five, counting MDIForm) this subset covers.
const CT_LABEL: u8 = 1;
const CT_COMMAND_BUTTON: u8 = 4;
const CT_LISTBOX: u8 = 8;
const CT_FORM: u8 = 13;
const CT_MDIFORM: u8 = 20;

/// The exact SVBD function names `STRUCTURES.md` section 8.5 cites for the
/// opcode-to-name and opcode-to-type facts, and the one it cites for the
/// position block shape specifically (`GetControlSize`). Prior art read for
/// facts, per `AGENTS.md`; the type library itself is never opened.
const SVBD_OPCODE_AND_TYPE: &str = "SVBD ReturnGuiOpcode, ReturnDataType";
const SVBD_POSITION_BLOCK: &str = "SVBD GetControlSize";

/// This repository's own corpus measurement, `03-CONTEXT.md` decision D-01's
/// other allowed source: a fact this session read directly from a corpus
/// executable's own bytes, compared against the `.frm` source that
/// executable was built from, never transcribed from Semi VB Decompiler and
/// never from Microsoft's `VB6.OLB`. Plan 03-13 is the first to cite this
/// constant, closing the gap the phase 3 verification found: the property
/// loop stopped four opcodes before the resource blob opcode on every
/// corpus form, because none of the three opcodes in between carried a row.
///
/// Measured against, opcode by opcode:
/// - Opcode 1, `Caption` (`Text`): `corpus/vb6-code/Fire-effect/Fast_Flames.exe`
///   (offset `0x138d`) and
///   `corpus/public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe`
///   (offset `0x12e5`).
/// - Opcode 3, `BackColor` (`Long`):
///   `corpus/vb6-code/Fire-effect/Fast_Flames.exe` (offset `0x13ca`, a
///   system colour) and
///   `corpus/vb6-code/Brightness-effect/Part 1 - Pure VB6/vbBrightness.exe`
///   (offset `0x1303`, a literal, non-system colour).
/// - Opcode 35, the resource blob (`Picture`):
///   `corpus/vb6-code/Fire-effect/Fast_Flames.exe` (offset `0x13d4`) and
///   `corpus/public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe`
///   (offset `0x1301`).
const CORPUS_MEASURED: &str = "this repository's own corpus measurement (plan 03-13)";

/// The Form and MDIForm rows `STRUCTURES.md` section 8.5.1 already
/// transcribes from Semi VB Decompiler's own authored source comments.
/// `STRUCTURES.md` groups them under one "Form / MDIForm (`cType` 13, 20)"
/// heading, because they share one property set.
const FORM_ROWS: &[(u8, &str, PayloadType)] = &[
    (10, "WindowState", PayloadType::Byte),
    (11, "MousePointer", PayloadType::Byte),
    (27, "DrawStyle", PayloadType::Byte),
    (29, "FillStyle", PayloadType::Byte),
    (31, "DrawMode", PayloadType::Byte),
    (34, "BorderStyle", PayloadType::Byte),
    (37, "LinkMode", PayloadType::Byte),
    (61, "LockControls", PayloadType::Byte),
    (62, "NegotiateMenus", PayloadType::Byte),
    (64, "Font", PayloadType::Font),
    (65, "Appearance", PayloadType::Byte),
    (70, "StartUpPosition", PayloadType::Byte),
    (71, "OLEDropMode", PayloadType::Byte),
    (73, "PaletteMode", PayloadType::Byte),
];

/// The Form and MDIForm rows this repository measured directly against its
/// own corpus, per [`CORPUS_MEASURED`], never transcribed from
/// `STRUCTURES.md` section 8.5.1 or from any other prior art. Plan 03-13
/// adds these three so the property loop reaches the resource blob opcode
/// (35) instead of stopping at the first one, opcode 1, with no row at all.
const FORM_CORPUS_ROWS: &[(u8, &str, PayloadType)] = &[
    (1, "Caption", PayloadType::Text),
    (3, "BackColor", PayloadType::Long),
    (35, "Icon", PayloadType::Picture),
];

/// The CommandButton rows, `cType` 4.
const COMMAND_BUTTON_ROWS: &[(u8, &str, PayloadType)] = &[
    (4, "Position", PayloadType::Position),
    (10, "MousePointer", PayloadType::Byte),
    (22, "DragMode", PayloadType::Byte),
    (29, "Font", PayloadType::Font),
    (31, "Appearance", PayloadType::Byte),
    (38, "OLEDropMode", PayloadType::Byte),
    (41, "Style", PayloadType::Byte),
];

/// The Label rows, `cType` 1. `DataSource` (opcode 32) and `DataFormat`
/// (opcode 45) are left out: section 8.5.1 names them with no payload
/// width, and this subset transcribes only a row whose width it can cite.
const LABEL_ROWS: &[(u8, &str, PayloadType)] = &[
    (5, "Position", PayloadType::Position),
    (11, "MousePointer", PayloadType::Byte),
    (19, "BorderStyle", PayloadType::Byte),
    (20, "Alignment", PayloadType::Byte),
    (26, "DragMode", PayloadType::Byte),
    (31, "BackStyle", PayloadType::Byte),
    (37, "Font", PayloadType::Font),
    (39, "Appearance", PayloadType::Byte),
    (43, "OLEDropMode", PayloadType::Byte),
];

/// The ListBox rows, `cType` 8. `List` (opcode 20, a count then
/// length-prefixed strings) and `DataSource` (opcode 40, no payload width
/// given) are left out for the same reason `LABEL_ROWS` leaves out its two.
const LIST_BOX_ROWS: &[(u8, &str, PayloadType)] = &[
    (4, "Position", PayloadType::Position),
    (10, "MousePointer", PayloadType::Byte),
    (24, "DragMode", PayloadType::Byte),
    (29, "MultiSelect", PayloadType::Byte),
    (39, "Font", PayloadType::Font),
    (44, "Appearance", PayloadType::Byte),
    (49, "OLEDragMode", PayloadType::Byte),
    (50, "OLEDropMode", PayloadType::Byte),
    (51, "Style", PayloadType::Byte),
];

/// Inserts one control type's rows into `entries`, citing
/// [`SVBD_POSITION_BLOCK`] for a [`PayloadType::Position`] row and
/// `default_source` for every other one.
///
/// `default_source` is the caller's own provenance string, not a value this
/// function chooses from the payload shape: every call site that inserts
/// `STRUCTURES.md`-transcribed rows passes [`SVBD_OPCODE_AND_TYPE`], and
/// [`OpcodeTable::builtin`]'s own corpus-measured call sites pass
/// [`CORPUS_MEASURED`] instead. A [`PayloadType::Position`] row still always
/// cites [`SVBD_POSITION_BLOCK`], because every position row this table
/// carries today came from that one fact, `STRUCTURES.md` section 8.5's own
/// `GetControlSize` citation, regardless of which rows array it sits in.
fn insert_builtin_rows(
    entries: &mut HashMap<(u8, u8), OpcodeEntry>,
    control_type: u8,
    rows: &[(u8, &str, PayloadType)],
    default_source: &'static str,
) {
    for &(opcode, name, payload) in rows {
        let source = if matches!(payload, PayloadType::Position) {
            SVBD_POSITION_BLOCK
        } else {
            default_source
        };
        entries.insert(
            (control_type, opcode),
            OpcodeEntry {
                name: name.to_owned(),
                payload,
                source,
            },
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{OpcodeTable, PayloadType};

    #[test]
    fn lookup_gives_the_entry_the_pair_names() {
        let table = OpcodeTable::builtin();
        let entry = table.lookup(13, 31).unwrap();
        assert_eq!(entry.name, "DrawMode");
    }

    #[test]
    fn lookup_gives_none_for_a_pair_the_table_holds_no_entry_for() {
        // PictureBox, cType 0, is not in the safe-provenance subset.
        assert_eq!(OpcodeTable::builtin().lookup(0, 31), None);
    }

    /// The concrete proof that the opcode space is per control type:
    /// opcode 31 names three different properties on three different
    /// control types.
    #[test]
    fn opcode_31_names_three_different_properties_on_three_control_types() {
        let table = OpcodeTable::builtin();
        assert_eq!(table.lookup(13, 31).unwrap().name, "DrawMode");
        assert_eq!(table.lookup(4, 31).unwrap().name, "Appearance");
        assert_eq!(table.lookup(1, 31).unwrap().name, "BackStyle");
    }

    #[test]
    fn a_picture_box_opcode_gives_none_because_the_subset_holds_no_picture_box_entries() {
        assert_eq!(OpcodeTable::builtin().lookup(0, 31), None);
    }

    #[test]
    fn mdiform_shares_the_forms_property_set() {
        assert_eq!(
            OpcodeTable::builtin().lookup(20, 31).unwrap().name,
            "DrawMode"
        );
    }

    #[test]
    fn every_builtin_entry_carries_a_non_empty_source() {
        let table = OpcodeTable::builtin();
        assert!(!table.is_empty());
        for control_type in [1_u8, 4, 8, 13, 20] {
            for opcode in 0_u8..=255 {
                if let Some(entry) = table.lookup(control_type, opcode) {
                    assert!(
                        !entry.source.is_empty(),
                        "{:?} on control type {control_type} has an empty source",
                        entry.name
                    );
                }
            }
        }
    }

    #[test]
    fn parse_on_empty_input_gives_a_table_with_zero_rows_and_no_error() {
        let table = OpcodeTable::parse(b"").unwrap();
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
    }

    #[test]
    fn a_hand_written_table_parses_and_lookup_finds_it_through_the_one_shared_method() {
        let text = b"[4]\n31 = { name = \"Appearance\", payload = \"Byte\" }\n41 = { name = \"Style\", payload = \"Byte\" }\n";
        let table = OpcodeTable::parse(text).unwrap();
        assert_eq!(table.len(), 2);
        let entry = table.lookup(4, 31).unwrap();
        assert_eq!(entry.name, "Appearance");
        assert_eq!(entry.payload, PayloadType::Byte);
        assert_eq!(entry.source, "a user supplied table");
        assert_eq!(table.lookup(4, 41).unwrap().name, "Style");
    }

    #[test]
    fn parse_refuses_a_control_type_above_255_and_names_the_line() {
        let text = b"[256]\n31 = { name = \"X\", payload = \"Byte\" }\n";
        let err = OpcodeTable::parse(text).unwrap_err();
        assert_eq!(err.line, 1);
        assert!(!err.message.is_empty());
    }

    #[test]
    fn parse_refuses_a_malformed_row_and_names_the_line_and_what_it_expected() {
        // The row is missing its required "payload" field.
        let text = b"[13]\n31 = { name = \"DrawMode\" }\n";
        let err = OpcodeTable::parse(text).unwrap_err();
        assert_eq!(err.line, 2);
        assert!(!err.message.is_empty());
    }

    #[test]
    fn parse_refuses_bytes_that_are_not_valid_utf8() {
        let bytes = [0xFF, 0xFE, 0x00];
        let err = OpcodeTable::parse(&bytes).unwrap_err();
        assert!(!err.message.is_empty());
    }

    #[test]
    fn payload_type_fixed_width_gives_the_declared_byte_count_for_the_fixed_width_kinds() {
        assert_eq!(PayloadType::Byte.fixed_width(), Some(1));
        assert_eq!(PayloadType::Boolean.fixed_width(), Some(2));
        assert_eq!(PayloadType::Integer.fixed_width(), Some(2));
        assert_eq!(PayloadType::Long.fixed_width(), Some(4));
        assert_eq!(PayloadType::Single.fixed_width(), Some(4));
    }

    #[test]
    fn payload_type_fixed_width_gives_none_for_the_variable_width_kinds() {
        assert_eq!(PayloadType::Text.fixed_width(), None);
        assert_eq!(PayloadType::Picture.fixed_width(), None);
        assert_eq!(PayloadType::Font.fixed_width(), None);
        assert_eq!(PayloadType::Position.fixed_width(), None);
    }

    /// A round trip through this module's own `parse`, over a literal this
    /// test writes by hand, is self agreement, not verification: it proves
    /// only that `parse` reads the shape this test itself wrote. The real
    /// check on the builtin subset is plan 03-10's differential gate,
    /// against the committed `.frm` source, per `AGENTS.md`. `source`
    /// differs on purpose between the two tables (a parsed row never cites
    /// an SVBD function), so only `name` and `payload` are compared.
    #[test]
    fn a_hand_written_table_round_trips_through_parse_alone() {
        let text = b"[1]\n31 = { name = \"BackStyle\", payload = \"Byte\" }\n";
        let parsed = OpcodeTable::parse(text).unwrap();
        let parsed_entry = parsed.lookup(1, 31).unwrap();
        let builtin = OpcodeTable::builtin();
        let builtin_entry = builtin.lookup(1, 31).unwrap();
        assert_eq!(parsed_entry.name, builtin_entry.name);
        assert_eq!(parsed_entry.payload, builtin_entry.payload);
    }

    // --- Plan 03-13, Task 1: the provenance seam and the Caption row -----

    /// `insert_builtin_rows` taking its default source from the caller adds
    /// no row of its own: the table's shape (which pairs resolve, and to
    /// what) is unchanged by the seam alone. This is `lookup_gives_the_entry_
    /// the_pair_names` and `opcode_31_names_three_different_properties_on_
    /// three_control_types`, both still passing after the seam change and
    /// before `FORM_CORPUS_ROWS` existed, run during this task's own
    /// development; they are not repeated here as a separate assertion
    /// because a passing `cargo test -p deform6 --lib vb::opcodes` on the
    /// seam commit alone is what that step proved.
    #[test]
    fn form_opcode_1_gives_caption_with_a_text_payload_and_the_corpus_measured_source() {
        let table = OpcodeTable::builtin();
        let entry = table.lookup(super::CT_FORM, 1).unwrap();
        assert_eq!(entry.name, "Caption");
        assert_eq!(entry.payload, PayloadType::Text);
        assert_eq!(entry.source, super::CORPUS_MEASURED);
    }

    #[test]
    fn mdiform_shares_the_forms_corpus_measured_caption_row() {
        let table = OpcodeTable::builtin();
        let entry = table.lookup(super::CT_MDIFORM, 1).unwrap();
        assert_eq!(entry.name, "Caption");
        assert_eq!(entry.source, super::CORPUS_MEASURED);
    }

    /// `Fast_Flames.exe`'s own form, `frmFire`, gives the caption
    /// `frmFire.frm` line 5 declares, read through the production `inspect`
    /// path, at the offset this session measured by hand (`0x138d`): opcode
    /// `01`, a declared length of `0x0039` (57), 57 characters, and a
    /// terminating zero at `0x13c9`.
    #[test]
    fn fast_flames_form_recovers_the_real_caption_from_the_committed_frm() {
        let data: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus/vb6-code/Fire-effect/Fast_Flames.exe"
        ));
        let report = crate::vb::inspect(data, &OpcodeTable::builtin()).unwrap();
        let form = report
            .forms
            .iter()
            .find(|f| f.name == "frmFire")
            .expect("Fast_Flames.exe declares a form named frmFire");
        let root = form
            .controls
            .first()
            .expect("frmFire's own tree must resolve for this corpus measurement to stand");
        let caption = root
            .properties
            .iter()
            .find_map(|p| match p {
                crate::vb::propstream::PropertyValue::Text { name, value } if name == "Caption" => {
                    Some(value.as_str())
                }
                _ => None,
            })
            .expect("frmFire's own Caption must now resolve, not stop the loop at opcode 1");
        assert_eq!(
            caption,
            "Even Faster Real-Time Fire Effect - www.tannerhelland.com"
        );
    }

    /// A second, independent corpus program: `SubReality_WinsockSample.exe`'s
    /// own form, `frmMain`, gives the caption `frmMain.frm` line 5 declares.
    /// One file alone cannot carry this row's proof.
    #[test]
    fn winsock_sample_form_recovers_the_real_caption_from_the_committed_frm() {
        let data: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus/public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe"
        ));
        let report = crate::vb::inspect(data, &OpcodeTable::builtin()).unwrap();
        let form = report
            .forms
            .iter()
            .find(|f| f.name == "frmMain")
            .expect("SubReality_WinsockSample.exe declares a form named frmMain");
        let root = form
            .controls
            .first()
            .expect("frmMain's own tree must resolve for this corpus measurement to stand");
        let caption = root
            .properties
            .iter()
            .find_map(|p| match p {
                crate::vb::propstream::PropertyValue::Text { name, value } if name == "Caption" => {
                    Some(value.as_str())
                }
                _ => None,
            })
            .expect("frmMain's own Caption must now resolve, not stop the loop at opcode 1");
        assert_eq!(caption, "Connect to server");
    }

    // --- Plan 03-13, Task 2: the Form colour row -------------------------

    #[test]
    fn form_opcode_3_gives_back_color_with_a_long_payload_and_the_corpus_measured_source() {
        let table = OpcodeTable::builtin();
        let entry = table.lookup(super::CT_FORM, 3).unwrap();
        assert_eq!(entry.name, "BackColor");
        assert_eq!(entry.payload, PayloadType::Long);
        assert_eq!(entry.source, super::CORPUS_MEASURED);
    }

    /// `Fast_Flames.exe`'s own form, `frmFire`, gives the colour
    /// `frmFire.frm` line 4 declares, `BackColor = &H80000005&`, a system
    /// colour: read through the production `inspect` path, at the offset
    /// this session measured by hand (`0x13ca`), the four bytes `05 00 00
    /// 80` reassemble to `0x8000_0005`, matching the declared hex literal
    /// bit for bit.
    #[test]
    fn fast_flames_form_recovers_the_real_system_back_color_from_the_committed_frm() {
        let data: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus/vb6-code/Fire-effect/Fast_Flames.exe"
        ));
        let report = crate::vb::inspect(data, &OpcodeTable::builtin()).unwrap();
        let form = report
            .forms
            .iter()
            .find(|f| f.name == "frmFire")
            .expect("Fast_Flames.exe declares a form named frmFire");
        let root = form
            .controls
            .first()
            .expect("frmFire's own tree must resolve for this corpus measurement to stand");
        let back_color = root
            .properties
            .iter()
            .find_map(|p| match p {
                crate::vb::propstream::PropertyValue::Long { name, value }
                    if name == "BackColor" =>
                {
                    Some(*value)
                }
                _ => None,
            })
            .expect("frmFire's own BackColor must now resolve");
        assert_eq!(back_color.cast_unsigned(), 0x8000_0005);
    }

    /// A second, independent corpus program, and a literal colour rather
    /// than a system one: `vbBrightness.exe`'s own form, `frmBrightness`,
    /// gives the colour `Brightness.frm` line 4 declares, `BackColor =
    /// &H00C0C0C0&`. Found by grepping the corpus for a `BackColor` line
    /// that is not the system colour every other sample so far has carried.
    #[test]
    fn brightness_form_recovers_a_real_literal_back_color_from_the_committed_frm() {
        let data: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus/vb6-code/Brightness-effect/Part 1 - Pure VB6/vbBrightness.exe"
        ));
        let report = crate::vb::inspect(data, &OpcodeTable::builtin()).unwrap();
        let form = report
            .forms
            .iter()
            .find(|f| f.name == "frmBrightness")
            .expect("vbBrightness.exe declares a form named frmBrightness");
        let root = form
            .controls
            .first()
            .expect("frmBrightness's own tree must resolve for this corpus measurement to stand");
        let back_color = root
            .properties
            .iter()
            .find_map(|p| match p {
                crate::vb::propstream::PropertyValue::Long { name, value }
                    if name == "BackColor" =>
                {
                    Some(*value)
                }
                _ => None,
            })
            .expect("frmBrightness's own BackColor must now resolve");
        assert_eq!(back_color.cast_unsigned(), 0x00C0_C0C0);
    }

    // --- Plan 03-13, Task 3: the Form resource blob row -------------------

    #[test]
    fn form_opcode_35_gives_icon_with_a_picture_payload_and_the_corpus_measured_source() {
        let table = OpcodeTable::builtin();
        let entry = table.lookup(super::CT_FORM, 35).unwrap();
        assert_eq!(entry.name, "Icon");
        assert_eq!(entry.payload, PayloadType::Picture);
        assert_eq!(entry.source, super::CORPUS_MEASURED);
    }

    /// Gives the byte offset of the form's own root control block's first
    /// [`crate::vb::propstream::PropertyValue::Undecoded`] entry with the
    /// given `opcode`, or `None` when none of its properties is that
    /// opcode. `Picture` is not yet a reader `walk_properties` has (plan
    /// 03-15 owns wiring `frx::extract_blob`), so opcode 35 still surfaces
    /// as `Undecoded`, carrying its own real byte offset: reachability, not
    /// decoding, is what this task proves.
    fn undecoded_offset(
        properties: &[crate::vb::propstream::PropertyValue],
        opcode: u8,
    ) -> Option<u32> {
        properties.iter().find_map(|p| match p {
            crate::vb::propstream::PropertyValue::Undecoded {
                opcode: found,
                offset,
                ..
            } if *found == opcode => Some(*offset),
            _ => None,
        })
    }

    /// `Fast_Flames.exe`'s own form, `frmFire`, reaches opcode 35 at the
    /// offset this session measured by hand (`0x13d4`): the property loop
    /// now advances past opcode 1 (Caption), opcode 25 (ScaleMode, already
    /// special-cased), opcode 3 (BackColor) and opcode 0 (a no-output
    /// opcode, already special-cased) to arrive there, instead of stopping
    /// at the very first opcode as it did before this plan.
    #[test]
    fn fast_flames_form_reaches_opcode_35_at_the_measured_offset() {
        let data: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus/vb6-code/Fire-effect/Fast_Flames.exe"
        ));
        let report = crate::vb::inspect(data, &OpcodeTable::builtin()).unwrap();
        let form = report
            .forms
            .iter()
            .find(|f| f.name == "frmFire")
            .expect("Fast_Flames.exe declares a form named frmFire");
        let root = form
            .controls
            .first()
            .expect("frmFire's own tree must resolve for this corpus measurement to stand");
        let offset = undecoded_offset(&root.properties, 35)
            .expect("frmFire's own property loop must reach opcode 35, not stop earlier");
        assert_eq!(offset, 0x13d4);
    }

    /// A second, independent corpus program:
    /// `SubReality_WinsockSample.exe`'s own form, `frmMain`, reaches opcode
    /// 35 at the offset this session measured by hand (`0x1301`). Both
    /// programs give the same opcode number for the resource blob, which is
    /// what this row's own proof requires.
    #[test]
    fn winsock_sample_form_reaches_opcode_35_at_the_measured_offset() {
        let data: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus/public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe"
        ));
        let report = crate::vb::inspect(data, &OpcodeTable::builtin()).unwrap();
        let form = report
            .forms
            .iter()
            .find(|f| f.name == "frmMain")
            .expect("SubReality_WinsockSample.exe declares a form named frmMain");
        let root = form
            .controls
            .first()
            .expect("frmMain's own tree must resolve for this corpus measurement to stand");
        let offset = undecoded_offset(&root.properties, 35)
            .expect("frmMain's own property loop must reach opcode 35, not stop earlier");
        assert_eq!(offset, 0x1301);
    }

    /// Adding `FORM_CORPUS_ROWS` changes no other control type: CommandButton
    /// opcode 31 still resolves `Appearance`, the same row it resolved
    /// before this plan, since `FORM_CORPUS_ROWS` is only ever inserted
    /// under `CT_FORM` and `CT_MDIFORM`.
    #[test]
    fn a_control_type_outside_the_form_group_resolves_the_same_rows_it_resolved_before() {
        let table = OpcodeTable::builtin();
        let entry = table.lookup(super::CT_COMMAND_BUTTON, 31).unwrap();
        assert_eq!(entry.name, "Appearance");
        assert_eq!(entry.source, super::SVBD_OPCODE_AND_TYPE);
        // CommandButton has no opcode 35 row: FORM_CORPUS_ROWS never reaches it.
        assert_eq!(table.lookup(super::CT_COMMAND_BUTTON, 35), None);
    }
}
