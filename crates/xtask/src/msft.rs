//! A reader of the type library format `MSFT`, the format of `VB6.OLB` and
//! of the type libraries that `MSVBVM60.DLL` holds.
//!
//! The file starts with the text `MSFT`. A header of `0x54` bytes gives the
//! number of type infos. Then come an offset for each type info, and a
//! directory of 15 segments. This reader reads only what
//! `derive-vb-types` needs:
//!
//! - segment 0, the table of type info records of `0x64` bytes: the kind,
//!   the offset of the function records, the counts, the GUID, the name and
//!   the size of the vtable;
//! - segment 5, the GUID table, and segment 7, the name table;
//! - segment 9, the table of type descriptions, for the type of a parameter
//!   that is not a simple type;
//! - the function records of each type info: the vtable offset, the kind of
//!   the member, the number of parameters, and the type and the flags of each
//!   parameter.
//!
//! The layout follows the `MSFT` structures that the Wine project documents.
//! The file comes from outside the repository. Each read checks its bounds,
//! and a fault gives an error that names the place.

use crate::pdb2::{u16_at, u32_at};

/// The first 4 bytes of the file.
const MAGIC: &[u8] = b"MSFT";

/// The length of the header.
const HEADER_LEN: usize = 0x54;

/// The flag of `varflags` that adds one field after the header.
const HELP_DLL_FLAG: u32 = 0x100;

/// The number of entries of the segment directory.
const SEGMENTS: usize = 15;

/// The length of one segment directory entry.
const SEGMENT_LEN: usize = 16;

/// The length of one type info record.
const TYPE_INFO_LEN: usize = 0x64;

/// The length of the fixed part of a function record.
const FUNCTION_FIXED_LEN: usize = 0x18;

/// The length of one parameter record.
const PARAMETER_LEN: usize = 12;

/// The segment of the type info records.
const SEGMENT_TYPE_INFOS: usize = 0;

/// The segment of the GUIDs.
const SEGMENT_GUIDS: usize = 5;

/// The segment of the names.
const SEGMENT_NAMES: usize = 7;

/// The segment of the type descriptions.
const SEGMENT_TYPE_DESCS: usize = 9;

/// The kind of a type info.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `TKIND_INTERFACE`: a vtable interface.
    Interface,
    /// `TKIND_DISPATCH`: a dispatch interface.
    Dispatch,
    /// `TKIND_COCLASS`: a class.
    Coclass,
    /// Any other kind.
    Other,
}

/// One parameter of a function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Parameter {
    /// The variant type: the simple type, or the type of the type
    /// description.
    pub vt: u16,
    /// The `PARAMFLAG` bits.
    pub flags: u32,
}

/// The `PARAMFLAG_FRETVAL` bit: the parameter receives the result.
pub(crate) const PARAMFLAG_FRETVAL: u32 = 0x8;

/// One function of a type info.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Function {
    /// The name, when the name table gives one.
    pub name: Option<String>,
    /// The byte offset in the vtable, with the low bit cleared.
    pub vtable_offset: u16,
    /// The `INVOKEKIND`: 1 for a method, 2 for a property get, 4 for a
    /// property let, 8 for a property set.
    pub invoke_kind: u8,
    /// Each parameter, first parameter first.
    pub parameters: Vec<Parameter>,
}

/// One type info.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypeInfo {
    /// The kind.
    pub kind: Kind,
    /// The name, when the name table gives one.
    pub name: Option<String>,
    /// The 16 bytes of the GUID, when the GUID table gives it.
    pub guid: Option<[u8; 16]>,
    /// `cbSizeVft`: the size of the vtable in bytes.
    pub vtable_size: u16,
    /// Each function.
    pub functions: Vec<Function>,
}

/// Reads a little-endian `i32` at `at`.
fn i32_at(bytes: &[u8], at: usize) -> Option<i32> {
    u32_at(bytes, at).map(u32::cast_signed)
}

/// Converts an offset from the file to a `usize`.
fn to_usize(value: i32, what: &str) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| format!("{what} is negative: {value}"))
}

/// Adds two offsets, with an error that names `what` on an overflow.
fn add(a: usize, b: usize, what: &str) -> Result<usize, String> {
    a.checked_add(b)
        .ok_or_else(|| format!("{what} overflows a usize"))
}

/// The start of one segment.
fn segment(bytes: &[u8], directory: usize, index: usize) -> Result<usize, String> {
    let at = add(
        directory,
        index
            .checked_mul(SEGMENT_LEN)
            .ok_or("a segment index overflows")?,
        "a segment entry",
    )?;
    let offset =
        i32_at(bytes, at).ok_or_else(|| format!("the file ends in segment entry {index}"))?;
    to_usize(offset, "a segment offset")
}

/// Reads the name at `offset` in the name table, or `None` for -1.
fn name(bytes: &[u8], names: usize, offset: i32) -> Result<Option<String>, String> {
    if offset < 0 {
        return Ok(None);
    }
    let at = add(names, to_usize(offset, "a name offset")?, "a name")?;
    let len = usize::from(
        *bytes
            .get(add(at, 8, "a name length")?)
            .ok_or_else(|| format!("the file ends in the name at {at:#x}"))?,
    );
    let start = add(at, 12, "a name")?;
    let text = bytes
        .get(start..add(start, len, "a name")?)
        .ok_or_else(|| format!("the name at {at:#x} is cut"))?;
    Ok(Some(text.iter().copied().map(char::from).collect()))
}

/// Reads the type of a parameter: a simple type when `data_type` is
/// negative, or else the type of the type description at that offset.
fn parameter_vt(bytes: &[u8], descs: usize, data_type: i32) -> Result<u16, String> {
    if data_type < 0 {
        return Ok(u16::try_from(data_type.cast_unsigned() & 0xFFFF).unwrap_or(0));
    }
    let at = add(descs, to_usize(data_type, "a type description")?, "a type")?;
    u16_at(bytes, at).ok_or_else(|| format!("the file ends in the type description at {at:#x}"))
}

/// Reads the function records of one type info.
fn functions(
    bytes: &[u8],
    names: usize,
    descs: usize,
    records: usize,
    functions: usize,
    variables: usize,
) -> Result<Vec<Function>, String> {
    let total = u32_at(bytes, records)
        .ok_or_else(|| format!("the file ends in the records at {records:#x}"))?;
    let first = add(records, 4, "the records")?;
    let after = add(
        first,
        usize::try_from(total).map_err(|_| "a record size does not fit")?,
        "the records",
    )?;
    let name_offsets = add(
        after,
        add(functions, variables, "the member count")?
            .checked_mul(4)
            .ok_or("the member count overflows")?,
        "the names of the members",
    )?;
    let mut out = Vec::new();
    let mut at = first;
    for index in 0..functions {
        let size = usize::from(
            u16_at(bytes, at).ok_or_else(|| format!("the file ends in the record at {at:#x}"))?,
        );
        if size < FUNCTION_FIXED_LEN || add(at, size, "a record")? > after {
            return Err(format!("the record at {at:#x} has the size {size:#x}"));
        }
        let field = |offset: usize| {
            u32_at(bytes, at.saturating_add(offset))
                .ok_or_else(|| format!("the record at {at:#x} is cut"))
        };
        let vtable_offset = u16::try_from(field(0x0C)? & 0xFFFE).unwrap_or(0);
        let invoke_kind = u8::try_from((field(0x10)? >> 3) & 0xF).unwrap_or(0);
        let count = usize::from(u16::try_from(field(0x14)? & 0xFFFF).unwrap_or(0));
        let parameter_bytes = count
            .checked_mul(PARAMETER_LEN)
            .ok_or("a parameter count overflows")?;
        if add(FUNCTION_FIXED_LEN, parameter_bytes, "a record")? > size {
            return Err(format!(
                "the record at {at:#x} is too short for {count} parameters"
            ));
        }
        let mut parameters = Vec::new();
        let start = add(
            at,
            size.checked_sub(parameter_bytes)
                .ok_or("a record is shorter than its parameters")?,
            "the parameters",
        )?;
        for k in 0..count {
            let p = add(
                start,
                k.checked_mul(PARAMETER_LEN)
                    .ok_or("a parameter index overflows")?,
                "a parameter",
            )?;
            let data_type =
                i32_at(bytes, p).ok_or_else(|| format!("the parameter at {p:#x} is cut"))?;
            let flags = u32_at(bytes, add(p, 8, "a parameter")?)
                .ok_or_else(|| format!("the parameter at {p:#x} is cut"))?;
            parameters.push(Parameter {
                vt: parameter_vt(bytes, descs, data_type)?,
                flags,
            });
        }
        let name_at = add(
            name_offsets,
            index.checked_mul(4).ok_or("a member index overflows")?,
            "a member name",
        )?;
        let name_offset = i32_at(bytes, name_at)
            .ok_or_else(|| format!("the file ends in the member names at {name_at:#x}"))?;
        out.push(Function {
            name: name(bytes, names, name_offset)?,
            vtable_offset,
            invoke_kind,
            parameters,
        });
        at = add(at, size, "a record")?;
    }
    Ok(out)
}

/// Reads each type info of a type library.
///
/// # Errors
///
/// Gives an error that names the place when the file is not an `MSFT` type
/// library, or when a read goes past its end.
pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<TypeInfo>, String> {
    if bytes.get(..MAGIC.len()) != Some(MAGIC) {
        return Err("the file does not start with MSFT".to_owned());
    }
    let header = |at: usize| u32_at(bytes, at).ok_or("the file ends in the header");
    let varflags = header(0x14)?;
    let count = usize::try_from(header(0x20)?).map_err(|_| "the type info count does not fit")?;
    let offsets = if varflags & HELP_DLL_FLAG == 0 {
        HEADER_LEN
    } else {
        HEADER_LEN + 4
    };
    let directory = add(
        offsets,
        count
            .checked_mul(4)
            .ok_or("the type info count overflows")?,
        "the segment directory",
    )?;
    if add(directory, SEGMENTS * SEGMENT_LEN, "the segment directory")? > bytes.len() {
        return Err("the file ends in the segment directory".to_owned());
    }
    let infos = segment(bytes, directory, SEGMENT_TYPE_INFOS)?;
    let guids = segment(bytes, directory, SEGMENT_GUIDS)?;
    let names = segment(bytes, directory, SEGMENT_NAMES)?;
    let descs = segment(bytes, directory, SEGMENT_TYPE_DESCS)?;

    let mut out = Vec::new();
    for index in 0..count {
        let at = add(
            infos,
            index
                .checked_mul(TYPE_INFO_LEN)
                .ok_or("a type info index overflows")?,
            "a type info",
        )?;
        let field =
            |offset: usize| i32_at(bytes, at.saturating_add(offset)).ok_or("a type info is cut");
        let kind = match field(0x00)? & 0xF {
            3 => Kind::Interface,
            4 => Kind::Dispatch,
            5 => Kind::Coclass,
            _ => Kind::Other,
        };
        let elements = field(0x18)?.cast_unsigned();
        let function_count = usize::from(u16::try_from(elements & 0xFFFF).unwrap_or(0));
        let variable_count = usize::from(u16::try_from(elements >> 16).unwrap_or(0));
        let guid_offset = field(0x2C)?;
        let guid = if guid_offset < 0 {
            None
        } else {
            let start = add(guids, to_usize(guid_offset, "a GUID offset")?, "a GUID")?;
            let raw = bytes
                .get(start..add(start, 16, "a GUID")?)
                .ok_or_else(|| format!("the GUID at {start:#x} is cut"))?;
            Some(<[u8; 16]>::try_from(raw).map_err(|_| "a GUID is not 16 bytes")?)
        };
        let vtable_size =
            u16_at(bytes, add(at, 0x4E, "a type info")?).ok_or("a type info is cut")?;
        let records = field(0x04)?;
        let functions = if function_count == 0 || records < 0 {
            Vec::new()
        } else {
            functions(
                bytes,
                names,
                descs,
                to_usize(records, "a record offset")?,
                function_count,
                variable_count,
            )?
        };
        out.push(TypeInfo {
            kind,
            name: name(bytes, names, field(0x34)?)?,
            guid,
            vtable_size,
            functions,
        });
    }
    Ok(out)
}

/// Formats a GUID in the registry form, such as
/// `{33AD4ED2-6699-11CF-B70C-00AA0060D393}`.
pub(crate) fn guid_text(guid: &[u8; 16]) -> String {
    let [a0, a1, a2, a3, b0, b1, c0, c1, rest @ ..] = *guid;
    let tail: String = rest.iter().map(|byte| format!("{byte:02X}")).collect();
    format!(
        "{{{:08X}-{:04X}-{:04X}-{}-{}}}",
        u32::from_le_bytes([a0, a1, a2, a3]),
        u16::from_le_bytes([b0, b1]),
        u16::from_le_bytes([c0, c1]),
        tail.get(..4).unwrap_or_default(),
        tail.get(4..).unwrap_or_default()
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
pub(crate) mod tests {
    use super::{Function, Kind, Parameter, guid_text, parse};

    fn put_u16(out: &mut [u8], at: usize, value: u16) {
        out[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(out: &mut [u8], at: usize, value: u32) {
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// The GUID of the events interface of [`library`].
    pub(crate) const GUID: [u8; 16] = [
        0xD2, 0x4E, 0xAD, 0x33, 0x99, 0x66, 0xCF, 0x11, 0xB7, 0x0C, 0x00, 0xAA, 0x00, 0x60, 0xD3,
        0x93,
    ];

    /// A type library built here, with two type infos. The first is the
    /// interface `_Box` with a vtable of 0x30 bytes, no GUID, and two
    /// functions: `Text` at 0x24, a property get with one parameter, a
    /// pointer that receives the result; and `Move` at 0x29 with the low
    /// bit set, a method with a `Single` and a `Variant`. The second is the
    /// interface `BoxEvents` with [`GUID`] and no function.
    ///
    /// Layout: the header at 0, the 2 type info offsets at 0x54, the
    /// segment directory at 0x5C, the type infos at 0x200, the GUIDs at
    /// 0x300, the names at 0x400, the type descriptions at 0x500 and the
    /// function records at 0x600.
    pub(crate) fn library() -> Vec<u8> {
        let mut out = vec![0_u8; 0x800];
        out[..4].copy_from_slice(b"MSFT");
        put_u32(&mut out, 0x20, 2);
        let directory = 0x5C;
        for (index, offset) in [(0, 0x200), (5, 0x300), (7, 0x400), (9, 0x500)] {
            put_u32(&mut out, directory + 16 * index, offset);
        }
        // _Box.
        put_u32(&mut out, 0x200, 3);
        put_u32(&mut out, 0x204, 0x600);
        put_u32(&mut out, 0x218, 2);
        put_u32(&mut out, 0x22C, u32::MAX);
        put_u32(&mut out, 0x234, 0);
        put_u16(&mut out, 0x24E, 0x30);
        // BoxEvents.
        put_u32(&mut out, 0x264, 3);
        put_u32(&mut out, 0x268, u32::MAX);
        put_u32(&mut out, 0x290, 0);
        put_u32(&mut out, 0x298, 0x20);
        out[0x300..0x310].copy_from_slice(&GUID);
        for (at, text) in [
            (0x400, "_Box"),
            (0x420, "BoxEvents"),
            (0x440, "Text"),
            (0x460, "Move"),
        ] {
            out[at + 8] = u8::try_from(text.len()).unwrap();
            out[at + 12..at + 12 + text.len()].copy_from_slice(text.as_bytes());
        }
        // A type description of VT_PTR.
        put_u16(&mut out, 0x500, 26);
        // Text: 0x18 + 12 bytes.
        let text = 0x604;
        put_u16(&mut out, text, 0x24);
        put_u32(&mut out, text + 0x0C, 0x24);
        put_u32(&mut out, text + 0x10, 2 << 3);
        put_u32(&mut out, text + 0x14, 1);
        put_u32(&mut out, text + 0x18, 0);
        put_u32(&mut out, text + 0x20, 0xA);
        // Move: 0x18 + 24 bytes.
        let moving = text + 0x24;
        put_u16(&mut out, moving, 0x30);
        put_u32(&mut out, moving + 0x0C, 0x29);
        put_u32(&mut out, moving + 0x10, 1 << 3);
        put_u32(&mut out, moving + 0x14, 2);
        put_u32(&mut out, moving + 0x18, 0x8000_0004);
        put_u32(&mut out, moving + 0x20, 1);
        put_u32(&mut out, moving + 0x24, 0x8000_000C);
        put_u32(&mut out, moving + 0x2C, 1);
        put_u32(&mut out, 0x600, 0x24 + 0x30);
        // The member ids, then the names.
        let names = 0x604 + 0x24 + 0x30 + 8;
        put_u32(&mut out, names, 0x40);
        put_u32(&mut out, names + 4, 0x60);
        out
    }

    #[test]
    fn a_library_gives_its_interfaces_and_their_functions() {
        let infos = parse(&library()).unwrap();
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].kind, Kind::Interface);
        assert_eq!(infos[0].name.as_deref(), Some("_Box"));
        assert_eq!(infos[0].guid, None);
        assert_eq!(infos[0].vtable_size, 0x30);
        assert_eq!(
            infos[0].functions,
            [
                Function {
                    name: Some("Text".to_owned()),
                    vtable_offset: 0x24,
                    invoke_kind: 2,
                    parameters: vec![Parameter { vt: 26, flags: 0xA }],
                },
                Function {
                    name: Some("Move".to_owned()),
                    vtable_offset: 0x28,
                    invoke_kind: 1,
                    parameters: vec![
                        Parameter { vt: 4, flags: 1 },
                        Parameter { vt: 12, flags: 1 }
                    ],
                },
            ]
        );
        assert_eq!(infos[1].name.as_deref(), Some("BoxEvents"));
        assert_eq!(infos[1].guid, Some(GUID));
        assert!(infos[1].functions.is_empty());
    }

    #[test]
    fn a_damaged_library_gives_an_error_and_no_panic() {
        let whole = library();
        assert!(parse(b"MSFX").is_err());
        for len in 0..whole.len() {
            let _ = parse(&whole[..len]);
        }
        let mut large = library();
        put_u32(&mut large, 0x20, u32::MAX);
        assert!(parse(&large).is_err());
        let mut record = library();
        put_u16(&mut record, 0x604, 0xFFFF);
        assert!(parse(&record).is_err());
    }

    #[test]
    fn a_guid_has_the_registry_form() {
        assert_eq!(guid_text(&GUID), "{33AD4ED2-6699-11CF-B70C-00AA0060D393}");
    }
}
