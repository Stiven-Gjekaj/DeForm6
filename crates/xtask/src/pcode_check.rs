//! `check-pcode-table`: decodes each P-code body of `corpus-pcode/` with a
//! table that `derive-pcode-table` wrote, and counts the bodies that decode
//! to their end.
//!
//! The table is not in the repository, so no test can hold it. This command
//! is the check that the user runs on the table that they derive. A body
//! decodes when each opcode is in the table and has a width, and the last
//! opcode ends at the end of the body. It also decodes when an exit opcode
//! (a name that starts with `ExitProc`, or `End`) leaves fewer than four
//! bytes: bodies start on a four-byte boundary, and the bytes after the last
//! exit fill that space.
//!
//! The command exits with 1 when one body does not decode, and it names
//! each such body.

use std::collections::BTreeMap;

use deform6::read::pe::PeImage;
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::object::ObjectTable;
use deform6::vb::procdesc::read_method_table;
use deform6::vb::project::{ObjectTableHead, ProjectInfo};

use crate::build_record;
use crate::pcode_record;
use crate::pcode_table::DEFAULT_OUTPUT_PATH;

/// The first lead byte.
const FIRST_LEAD: u8 = 0xFB;

/// The number of bytes that a body can hold after its last exit.
const PADDING: usize = 4;

/// The width of one slot, as the table gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotWidth {
    /// This many bytes of arguments.
    Fixed(usize),
    /// A 16-bit byte count, and that many bytes.
    Counted,
}

/// One slot of a table that the check reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckSlot {
    /// The width, or `None` when the table gives none.
    pub(crate) width: Option<SlotWidth>,
    /// True when a name of the slot marks an exit.
    pub(crate) exit: bool,
}

/// A table that the check reads: the slot of each (table, opcode).
pub(crate) type CheckTable = BTreeMap<(String, u8), CheckSlot>;

/// Reads a table that `derive-pcode-table` wrote.
///
/// # Errors
///
/// Gives an error when the text is not TOML of that shape.
pub(crate) fn parse_table(text: &str) -> Result<CheckTable, String> {
    let root: toml::Table = text
        .parse()
        .map_err(|err| format!("the table is not TOML: {err}"))?;
    let mut out = BTreeMap::new();
    for (table, rows) in &root {
        let rows = rows
            .as_table()
            .ok_or_else(|| format!("{table} is not a table"))?;
        for (opcode, row) in rows {
            let op = u8::from_str_radix(opcode, 16)
                .map_err(|_| format!("{table}.{opcode} is not an opcode"))?;
            let width = match row.get("width") {
                None => None,
                Some(toml::Value::Integer(bytes)) => Some(SlotWidth::Fixed(
                    usize::try_from(*bytes)
                        .map_err(|_| format!("{table}.{opcode} has a bad width"))?,
                )),
                Some(toml::Value::String(word)) if word == "counted" => Some(SlotWidth::Counted),
                Some(_) => return Err(format!("{table}.{opcode} has a bad width")),
            };
            let exit = row
                .get("names")
                .and_then(toml::Value::as_array)
                .is_some_and(|names| {
                    names
                        .iter()
                        .filter_map(toml::Value::as_str)
                        .any(|name| name.starts_with("ExitProc") || name == "End")
                });
            out.insert((table.clone(), op), CheckSlot { width, exit });
        }
    }
    Ok(out)
}

/// Why a body does not decode, with the offset of the opcode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Fault {
    /// The opcode is not in the table.
    NoSlot {
        at: usize,
        table: String,
        opcode: u8,
    },
    /// The table gives the opcode no width.
    NoWidth {
        at: usize,
        table: String,
        opcode: u8,
    },
    /// The opcode or its arguments run past the end of the body.
    PastEnd { at: usize },
}

/// Decodes one body, and gives the number of opcodes that it holds.
///
/// # Errors
///
/// Gives the first [`Fault`].
pub(crate) fn decode_body(body: &[u8], table: &CheckTable) -> Result<usize, Fault> {
    let mut at = 0_usize;
    let mut count = 0_usize;
    let mut last_exit = false;
    while at < body.len() {
        if last_exit && body.len().saturating_sub(at) < PADDING {
            return Ok(count);
        }
        let start = at;
        let byte = *body.get(at).ok_or(Fault::PastEnd { at: start })?;
        at = at.saturating_add(1);
        let (name, opcode) = if byte >= FIRST_LEAD {
            let second = *body.get(at).ok_or(Fault::PastEnd { at: start })?;
            at = at.saturating_add(1);
            (format!("lead{}", byte.saturating_sub(FIRST_LEAD)), second)
        } else {
            ("primary".to_owned(), byte)
        };
        let slot = table
            .get(&(name.clone(), opcode))
            .ok_or_else(|| Fault::NoSlot {
                at: start,
                table: name.clone(),
                opcode,
            })?;
        let width = match slot.width {
            Some(SlotWidth::Fixed(bytes)) => bytes,
            Some(SlotWidth::Counted) => {
                let count = body
                    .get(at..at.saturating_add(2))
                    .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
                    .ok_or(Fault::PastEnd { at: start })?;
                usize::from(u16::from_le_bytes(count)).saturating_add(2)
            }
            None => {
                return Err(Fault::NoWidth {
                    at: start,
                    table: name,
                    opcode,
                });
            }
        };
        at = at.checked_add(width).ok_or(Fault::PastEnd { at: start })?;
        if at > body.len() {
            return Err(Fault::PastEnd { at: start });
        }
        count = count.saturating_add(1);
        last_exit = slot.exit;
    }
    Ok(count)
}

/// Runs `check-pcode-table [<table>]`.
pub(crate) fn run(args: &[String]) -> i32 {
    let path = match args {
        [] => DEFAULT_OUTPUT_PATH,
        [path] => path.as_str(),
        _ => {
            eprintln!("usage: cargo run -p xtask -- check-pcode-table [<table>]");
            return 1;
        }
    };
    match check(path) {
        Ok((bodies, failures)) => {
            for failure in &failures {
                println!("{failure}");
            }
            let decoded = bodies.saturating_sub(failures.len());
            println!("{decoded} of {bodies} P-code bodies decode to their end");
            i32::from(!failures.is_empty())
        }
        Err(err) => {
            eprintln!("check-pcode-table: {err}");
            1
        }
    }
}

/// Decodes each body of each program in `corpus-pcode/`, and gives the
/// number of bodies and a line for each body that does not decode.
fn check(path: &str) -> Result<(usize, Vec<String>), String> {
    let text = std::fs::read_to_string(path).map_err(|err| format!("reading {path}: {err}"))?;
    let table = parse_table(&text)?;
    let root = pcode_record::pcode_root();
    let mut bodies = 0_usize;
    let mut failures = Vec::new();
    for exe in build_record::executables(&root)? {
        let key = build_record::program_key(&exe, &root)?;
        let bytes = std::fs::read(&exe).map_err(|err| format!("reading {key}: {err}"))?;
        let pe = PeImage::parse(&bytes).map_err(|err| format!("{key}: {err}"))?;
        let header = VbHeader::read(&header_region(&pe).map_err(|err| format!("{key}: {err}"))?)
            .map_err(|err| format!("{key}: {err}"))?;
        let info = ProjectInfo::read(&pe, header.lp_project_data)
            .map_err(|err| format!("{key}: {err}"))?;
        let head = ObjectTableHead::read(&pe, info.lp_object_table)
            .map_err(|err| format!("{key}: {err}"))?;
        let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
            .map_err(|err| format!("{key}: {err}"))?;
        for object in &objects.objects {
            let methods = read_method_table(&pe, object.lp_object_info)
                .map_err(|err| format!("{key}: {}: {err}", object.name))?;
            for descriptor in methods.descriptors() {
                bodies = bodies.saturating_add(1);
                let body = descriptor
                    .body(&pe)
                    .and_then(|body| body.take(deform6::read::region::Off::new(0), body.len()))
                    .ok_or_else(|| format!("{key}: a body cannot be read"))?;
                if let Err(fault) = decode_body(body, &table) {
                    failures.push(format!(
                        "{key}: {} descriptor {:#x}: {fault:?}",
                        object.name,
                        descriptor.va.get()
                    ));
                }
            }
        }
    }
    Ok((bodies, failures))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{CheckSlot, CheckTable, Fault, SlotWidth, decode_body, parse_table};

    /// A table: `F4` has one argument byte, `13` is an exit with none, `FB
    /// 01` has two, and `32` is counted. `05` has no width.
    fn table() -> CheckTable {
        parse_table(
            r#"
[primary.F4]
handler = "0x1"
width = 1
names = ["LitI2_Byte"]
[primary.13]
handler = "0x2"
width = 0
names = ["ExitProcHresult"]
[primary.32]
handler = "0x3"
width = "counted"
names = ["FFreeStr"]
[primary.05]
handler = "0x4"
names = ["ImpAdLdRf"]
[lead0.01]
handler = "0x5"
width = 2
names = ["Other"]
"#,
        )
        .unwrap()
    }

    #[test]
    fn the_table_gives_each_width_and_each_exit() {
        let table = table();
        assert_eq!(
            table[&("primary".to_owned(), 0x13)],
            CheckSlot {
                width: Some(SlotWidth::Fixed(0)),
                exit: true,
            }
        );
        assert_eq!(
            table[&("primary".to_owned(), 0x32)].width,
            Some(SlotWidth::Counted)
        );
        assert_eq!(table[&("primary".to_owned(), 0x05)].width, None);
        assert!(parse_table("[primary.F4]\nwidth = \"many\"\n").is_err());
    }

    #[test]
    fn a_body_decodes_to_its_end_or_to_an_exit_and_padding() {
        let table = table();
        assert_eq!(
            decode_body(&[0xF4, 0x00, 0xFB, 0x01, 0xAA, 0xBB, 0x13], &table),
            Ok(3)
        );
        assert_eq!(
            decode_body(
                &[0x32, 0x04, 0x00, 1, 2, 3, 4, 0x13, 0x00, 0xFF, 0x7],
                &table
            ),
            Ok(2)
        );
        // Four bytes after the exit are not padding.
        assert_eq!(
            decode_body(&[0x13, 0x00, 0x00, 0x00, 0x00], &table),
            Err(Fault::NoSlot {
                at: 1,
                table: "primary".to_owned(),
                opcode: 0,
            })
        );
    }

    #[test]
    fn a_body_that_runs_past_its_end_or_holds_an_unknown_opcode_fails() {
        let table = table();
        assert_eq!(
            decode_body(&[0xFB, 0x01, 0xAA], &table),
            Err(Fault::PastEnd { at: 0 })
        );
        assert_eq!(
            decode_body(&[0x32, 0x09, 0x00, 1], &table),
            Err(Fault::PastEnd { at: 0 })
        );
        assert_eq!(
            decode_body(&[0x05, 0x00, 0x00], &table),
            Err(Fault::NoWidth {
                at: 0,
                table: "primary".to_owned(),
                opcode: 5,
            })
        );
        assert!(matches!(
            decode_body(&[0x77], &table),
            Err(Fault::NoSlot { .. })
        ));
    }
}
