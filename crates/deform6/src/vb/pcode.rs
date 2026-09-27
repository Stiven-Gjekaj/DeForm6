//! Decoding a P-code body with a table that the user derives.
//!
//! The P-code opcode table is not in this repository. The user derives it
//! from a copy of `MSVBVM60.DLL` that they own, with `cargo run -p xtask --
//! derive-pcode-table`, and hands its bytes to [`PcodeTable::parse`]. This
//! library opens no file, the same split that
//! [`crate::vb::opcodes::OpcodeTable::parse`] follows.
//!
//! # The table
//!
//! One TOML table for each dispatch table: `primary`, and `lead0` to `lead4`
//! for the lead bytes `0xFB` to `0xFF`. Each key is an opcode in two
//! hexadecimal digits. Each row holds the `handler` address, the `width` of
//! the arguments (a number of bytes, or `"counted"` for a 16-bit byte count
//! and that many bytes), and the `names` of the handler.
//!
//! # The decode
//!
//! [`disassemble`] reads a body from its first byte. Each opcode is one
//! byte, or a lead byte and one more. Its arguments follow it. The decode
//! ends at the end of the body, or after an exit (a name that starts with
//! `ExitProc`, or `End`) that leaves fewer than four bytes: bodies start on a
//! four-byte boundary (`STRUCTURES.md` section 23), and those bytes fill the
//! space. It stops at the first opcode that the table does not hold, that
//! has no width, or whose arguments run past the end of the body.
//!
//! With the table of the runtime of the XP host, each of the 680 bodies of
//! `corpus-pcode/` decodes to its end.

use std::collections::BTreeMap;

use crate::read::region::{Off, Region};
use crate::vb::opcodes::{TableError, line_at};

/// The first lead byte.
const FIRST_LEAD: u8 = 0xFB;

/// The number of bytes that a body can hold after its last exit.
const PADDING: usize = 4;

/// The argument width of one slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcodeWidth {
    /// This many bytes of arguments.
    Fixed(u16),
    /// A 16-bit byte count, and that many bytes.
    Counted,
}

/// One slot of the table.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PcodeSlot {
    /// Each name of the handler, as the table gives them.
    pub names: Vec<String>,
    /// The argument width, or `None` when the table gives none.
    pub width: Option<PcodeWidth>,
}

impl PcodeSlot {
    /// Tells whether a name of the slot marks an exit.
    #[must_use]
    pub fn is_exit(&self) -> bool {
        self.names
            .iter()
            .any(|name| name.starts_with("ExitProc") || name == "End")
    }
}

/// A P-code opcode table that the user derived.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PcodeTable {
    slots: BTreeMap<(Option<u8>, u8), PcodeSlot>,
}

/// One row of the table, before it is folded into a [`PcodeSlot`].
#[derive(serde::Deserialize)]
struct RawRow {
    #[serde(default)]
    width: Option<RawWidth>,
    #[serde(default)]
    names: Vec<String>,
}

/// The width of a row: a number, or a word.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RawWidth {
    Fixed(u16),
    Word(String),
}

impl PcodeTable {
    /// Parses a table that `derive-pcode-table` wrote.
    ///
    /// # Errors
    ///
    /// Gives a [`TableError`] for text that is not UTF-8 or not TOML, for a
    /// table name other than `primary` and `lead0` to `lead4`, for a key
    /// that is not an opcode, and for a width word other than `counted`.
    pub fn parse(bytes: &[u8]) -> Result<Self, TableError> {
        let text = std::str::from_utf8(bytes).map_err(|err| TableError {
            line: line_at(bytes, err.valid_up_to()),
            message: "the table is not valid UTF-8".to_owned(),
        })?;
        let raw: BTreeMap<String, BTreeMap<String, RawRow>> =
            toml::from_str(text).map_err(|err| TableError {
                line: err
                    .span()
                    .map_or(1, |span| line_at(text.as_bytes(), span.start)),
                message: err.message().to_owned(),
            })?;
        let refuse = |message: String| TableError { line: 1, message };
        let mut slots = BTreeMap::new();
        for (table, rows) in raw {
            let lead = match table.as_str() {
                "primary" => None,
                "lead0" => Some(0),
                "lead1" => Some(1),
                "lead2" => Some(2),
                "lead3" => Some(3),
                "lead4" => Some(4),
                other => return Err(refuse(format!("{other:?} is not a dispatch table"))),
            };
            for (key, row) in rows {
                let opcode = u8::from_str_radix(&key, 16)
                    .ok()
                    .filter(|_| key.len() == 2)
                    .ok_or_else(|| refuse(format!("{table}.{key} is not an opcode")))?;
                let width = match row.width {
                    None => None,
                    Some(RawWidth::Fixed(bytes)) => Some(PcodeWidth::Fixed(bytes)),
                    Some(RawWidth::Word(word)) if word == "counted" => Some(PcodeWidth::Counted),
                    Some(RawWidth::Word(word)) => {
                        return Err(refuse(format!("{table}.{key} has the width {word:?}")));
                    }
                };
                slots.insert(
                    (lead, opcode),
                    PcodeSlot {
                        names: row.names,
                        width,
                    },
                );
            }
        }
        Ok(Self { slots })
    }

    /// Gives the slot of `opcode` after the lead byte `lead` (0 for `0xFB`),
    /// or in the primary table when `lead` is `None`.
    #[must_use]
    pub fn slot(&self, lead: Option<u8>, opcode: u8) -> Option<&PcodeSlot> {
        self.slots.get(&(lead, opcode))
    }

    /// Gives the number of slots.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Tells whether the table holds no slot.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// One decoded opcode.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PcodeInstruction {
    /// The offset of its first byte in the body.
    pub offset: u32,
    /// The lead byte, as 0 for `0xFB` to 4 for `0xFF`, or `None`.
    pub lead: Option<u8>,
    /// The opcode byte.
    pub opcode: u8,
    /// The argument bytes.
    pub arguments: Vec<u8>,
}

/// How a decode ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcodeEnd {
    /// The last opcode ends at the end of the body.
    Complete,
    /// An exit leaves this many bytes, fewer than four.
    Padding(u32),
    /// The table holds no slot for the opcode at this offset.
    NoSlot(u32),
    /// The table gives no width for the opcode at this offset.
    NoWidth(u32),
    /// The opcode at this offset, or its arguments, run past the end of the
    /// body.
    PastEnd(u32),
}

impl PcodeEnd {
    /// Tells whether the body decoded to its end.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Complete | Self::Padding(_))
    }
}

/// A decoded body.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PcodeListing {
    /// Each opcode that was decoded, in order.
    pub instructions: Vec<PcodeInstruction>,
    /// How the decode ended.
    pub end: PcodeEnd,
}

/// Converts an offset in a body to a `u32`.
fn offset(at: usize) -> u32 {
    u32::try_from(at).unwrap_or(u32::MAX)
}

/// Decodes a body with `table`.
#[must_use]
pub fn disassemble(body: &Region<'_>, table: &PcodeTable) -> PcodeListing {
    let bytes = body.take(Off::new(0), body.len()).unwrap_or_default();
    let mut instructions = Vec::new();
    let mut at = 0_usize;
    let mut after_exit = false;
    let end = loop {
        let left = bytes.len().saturating_sub(at);
        if left == 0 {
            break PcodeEnd::Complete;
        }
        if after_exit && left < PADDING {
            break PcodeEnd::Padding(offset(left));
        }
        let start = at;
        let Some(&first) = bytes.get(at) else {
            break PcodeEnd::PastEnd(offset(start));
        };
        at = at.saturating_add(1);
        let (lead, opcode) = if first >= FIRST_LEAD {
            let Some(&second) = bytes.get(at) else {
                break PcodeEnd::PastEnd(offset(start));
            };
            at = at.saturating_add(1);
            (Some(first.saturating_sub(FIRST_LEAD)), second)
        } else {
            (None, first)
        };
        let Some(slot) = table.slot(lead, opcode) else {
            break PcodeEnd::NoSlot(offset(start));
        };
        let width = match slot.width {
            Some(PcodeWidth::Fixed(width)) => usize::from(width),
            Some(PcodeWidth::Counted) => {
                let Some(count) = at
                    .checked_add(2)
                    .and_then(|end| bytes.get(at..end))
                    .and_then(|count| <[u8; 2]>::try_from(count).ok())
                else {
                    break PcodeEnd::PastEnd(offset(start));
                };
                usize::from(u16::from_le_bytes(count)).saturating_add(2)
            }
            None => break PcodeEnd::NoWidth(offset(start)),
        };
        let Some(arguments) = at.checked_add(width).and_then(|end| bytes.get(at..end)) else {
            break PcodeEnd::PastEnd(offset(start));
        };
        at = at.saturating_add(width);
        instructions.push(PcodeInstruction {
            offset: offset(start),
            lead,
            opcode,
            arguments: arguments.to_vec(),
        });
        after_exit = slot.is_exit();
    };
    PcodeListing { instructions, end }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{PcodeEnd, PcodeInstruction, PcodeTable, PcodeWidth, disassemble};
    use crate::read::region::{Off, Region};

    /// A table of seven slots, built here. The names are placeholders: a
    /// table that a runtime gives must not enter this repository. Only the
    /// rule of the exit is real: `ExitProc` at the start of a name, or `End`.
    /// The widths of `F4`, `08`, `8E` and `13` are the widths under which the
    /// measured body below decodes.
    const TABLE: &str = r#"
[primary.F4]
handler = "0x000fd21f"
width = 1
names = ["OpA"]
[primary.08]
handler = "0x1"
width = 2
names = ["OpB"]
[primary.8E]
handler = "0x2"
width = 2
names = ["OpC"]
[primary.13]
handler = "0x3"
width = 0
names = ["ExitProcTest"]
[primary.32]
handler = "0x4"
width = "counted"
names = ["OpCounted"]
[lead1.C8]
handler = "0x5"
width = 0
names = ["End"]
[primary.05]
handler = "0x6"
names = ["OpNoWidth"]
"#;

    /// One decoded opcode as a tuple: offset, lead, opcode and arguments.
    type Row = (u32, Option<u8>, u8, Vec<u8>);

    fn run(bytes: &[u8]) -> (Vec<Row>, PcodeEnd) {
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(bytes, Off::new(0)), &table);
        let list = listing
            .instructions
            .into_iter()
            .map(
                |PcodeInstruction {
                     offset,
                     lead,
                     opcode,
                     arguments,
                 }| { (offset, lead, opcode, arguments) },
            )
            .collect();
        (list, listing.end)
    }

    #[test]
    fn the_table_gives_each_slot_by_its_lead_and_its_opcode() {
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        assert_eq!(table.len(), 7);
        assert_eq!(
            table.slot(None, 0x32).unwrap().width,
            Some(PcodeWidth::Counted)
        );
        assert!(table.slot(Some(1), 0xC8).unwrap().is_exit());
        assert!(table.slot(None, 0xC8).is_none());
        assert_eq!(table.slot(None, 0x05).unwrap().width, None);
    }

    #[test]
    fn a_table_of_another_shape_is_refused_with_its_reason() {
        for bad in [
            "[lead5.00]\nwidth = 1\n",
            "[primary.100]\nwidth = 1\n",
            "[primary.0]\nwidth = 1\n",
            "[primary.00]\nwidth = \"many\"\n",
            "[primary.00]\nwidth = -1\n",
            "not toml",
        ] {
            assert!(PcodeTable::parse(bad.as_bytes()).is_err(), "{bad}");
        }
        assert!(PcodeTable::parse(&[0xFF]).is_err());
    }

    /// The body of `cmdStop_Click` in the P-code `Fast_Flames.exe`, byte for
    /// byte: four opcodes with 1, 2, 2 and 0 argument bytes, the last an exit,
    /// and three bytes of padding.
    #[test]
    fn the_measured_body_of_cmd_stop_click_decodes_to_its_end() {
        let body = [
            0xF4, 0x00, 0x08, 0x08, 0x00, 0x8E, 0x4C, 0x00, 0x13, 0x00, 0x00, 0x00,
        ];
        let (list, end) = run(&body);
        assert_eq!(
            list,
            [
                (0, None, 0xF4, vec![0x00]),
                (2, None, 0x08, vec![0x08, 0x00]),
                (5, None, 0x8E, vec![0x4C, 0x00]),
                (8, None, 0x13, vec![]),
            ]
        );
        assert_eq!(end, PcodeEnd::Padding(3));
        assert!(end.is_complete());
    }

    const FAST_FLAMES_P_CODE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus-pcode/vb6-code/Fire-effect/Fast_Flames.exe"
    ));

    /// The body above is the body that the descriptor of `cmdStop_Click`
    /// names in the file.
    #[test]
    fn the_body_of_cmd_stop_click_is_read_from_the_file() {
        use crate::read::pe::PeImage;
        use crate::read::region::Va;
        use crate::vb::procdesc::ProcDescriptor;

        let pe = PeImage::parse(FAST_FLAMES_P_CODE).unwrap();
        let descriptor = ProcDescriptor::read(&pe, Va::new(0x0040_3438)).unwrap();
        let body = descriptor.body(&pe).unwrap();
        assert_eq!(
            body.take(Off::new(0), body.len()),
            Some(
                &[
                    0xF4, 0x00, 0x08, 0x08, 0x00, 0x8E, 0x4C, 0x00, 0x13, 0x00, 0x00, 0x00
                ][..]
            )
        );
    }

    /// Each body of two or three bytes decodes to an end, and none panics.
    #[test]
    fn each_short_body_gives_an_end() {
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        for a in 0..=255_u8 {
            for b in 0..=255_u8 {
                for body in [[a, b].as_slice(), [a, b, a ^ b].as_slice()] {
                    let listing = disassemble(&Region::new(body, Off::new(0)), &table);
                    let decoded: usize = listing
                        .instructions
                        .iter()
                        .map(|i| 1 + usize::from(i.lead.is_some()) + i.arguments.len())
                        .sum();
                    assert!(decoded <= body.len());
                }
            }
        }
    }

    #[test]
    fn a_lead_byte_and_a_counted_argument_are_decoded() {
        let (list, end) = run(&[0x32, 0x04, 0x00, 1, 2, 3, 4, 0xFC, 0xC8]);
        assert_eq!(
            list,
            [
                (0, None, 0x32, vec![0x04, 0x00, 1, 2, 3, 4]),
                (7, Some(1), 0xC8, vec![]),
            ]
        );
        assert_eq!(end, PcodeEnd::Complete);
    }

    #[test]
    fn a_decode_stops_at_the_first_opcode_that_it_cannot_decode() {
        assert_eq!(run(&[0xF4, 0x00, 0x77]).1, PcodeEnd::NoSlot(2));
        assert_eq!(run(&[0x05, 0x00, 0x00]).1, PcodeEnd::NoWidth(0));
        assert_eq!(run(&[0xF4]).1, PcodeEnd::PastEnd(0));
        assert_eq!(run(&[0x32, 0x09, 0x00, 1]).1, PcodeEnd::PastEnd(0));
        assert_eq!(run(&[0xFC]).1, PcodeEnd::PastEnd(0));
        // Four bytes after an exit are not padding.
        assert_eq!(run(&[0x13, 0x00, 0x00, 0x00, 0x00]).1, PcodeEnd::NoSlot(1));
        assert!(!PcodeEnd::NoSlot(1).is_complete());
    }
}
