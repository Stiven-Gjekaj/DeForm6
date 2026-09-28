//! The constant table of an object.
//!
//! `STRUCTURES.md` section 5.2 puts `lpConstants` at `ObjectInfo + 0x34`.
//! The P-code engine loads it for each procedure of the object, and the
//! opcodes `LitStr`, `ImpAdCall`, `ImpAdLd`, `New` and `NewIfNullPr` name an
//! entry of it by its index. An entry is an address. What it points to
//! depends on the opcode: `LitStr` names the characters of a string, whose
//! length in bytes is the 4 bytes before them.
//!
//! The table holds entries of many kinds, and this module does not tell them
//! apart. A caller reads a string only at an index that a `LitStr` names.

use crate::read::pe::PeImage;
use crate::read::region::{Off, Va};

/// `STRUCTURES.md` section 5.2: `lpConstants` sits at `ObjectInfo + 0x34`.
const LP_CONSTANTS_AT: u32 = 0x34;

/// The longest string that this module reads, in bytes.
const MAX_STRING_BYTES: u32 = 0x1_0000;

/// Gives the address at `index` of the constant table of the object whose
/// `ObjectInfo` is at `lp_object_info`.
#[must_use]
pub fn constant(pe: &PeImage<'_>, lp_object_info: Va, index: u16) -> Option<Va> {
    let table = pe
        .region_at_va(lp_object_info)?
        .va_le(Off::new(LP_CONSTANTS_AT))?;
    pe.region_at_va(table)?
        .va_le(Off::new(u32::from(index).checked_mul(4)?))
}

/// Gives the 16 bytes of the GUID at the address of the entry at `index` of
/// the constant table. The second argument of `VCallHresult` names such an
/// entry: the GUID of the interface of the call.
#[must_use]
pub fn constant_guid(pe: &PeImage<'_>, lp_object_info: Va, index: u16) -> Option<[u8; 16]> {
    pe.region_at_va(constant(pe, lp_object_info, index)?)?
        .take(Off::new(0), 16)?
        .try_into()
        .ok()
}

/// The bytes of `jmp dword ptr [address]`, before the address.
const JUMP_THROUGH_ADDRESS: [u8; 2] = [0xFF, 0x25];

/// Gives the ordinal of the function of the runtime that the entry at
/// `index` of the constant table calls. `ImpAdCall` names such an entry:
/// the address of a jump through a slot of the import address table.
#[must_use]
pub fn constant_runtime_ordinal(pe: &PeImage<'_>, lp_object_info: Va, index: u16) -> Option<u16> {
    let thunk = pe.region_at_va(constant(pe, lp_object_info, index)?)?;
    if thunk.take(Off::new(0), 2)? != JUMP_THROUGH_ADDRESS {
        return None;
    }
    pe.runtime_ordinal(thunk.va_le(Off::new(2))?)
}

/// The first word of a class reference: the number of GUIDs that follow.
const CLASS_REFERENCE_GUIDS: u32 = 2;

/// Gives the GUID of the interface of the class reference at `index` of the
/// constant table. A class reference holds 2, the address of the GUID of
/// the class, and the address of the GUID of its interface. `NewIfNullPr`
/// names one for a class of the runtime, such as its global object.
#[must_use]
pub fn class_reference_iid(pe: &PeImage<'_>, lp_object_info: Va, index: u16) -> Option<[u8; 16]> {
    let reference = pe.region_at_va(constant(pe, lp_object_info, index)?)?;
    if reference.u32_le(Off::new(0))? != CLASS_REFERENCE_GUIDS {
        return None;
    }
    let iid = reference.va_le(Off::new(8))?;
    pe.region_at_va(iid)?.take(Off::new(0), 16)?.try_into().ok()
}

/// The longest member name that this module reads, in UTF-16 units.
const MAX_NAME_UNITS: u32 = 0x100;

/// Gives the member name at `index` of the constant table: UTF-16
/// characters up to a zero unit, with no length before them. `LateMemCall`
/// names one.
///
/// Gives `None` when no zero unit comes in the first 256 units, when the
/// name is empty, and when a character is not a letter, a digit or `_`.
#[must_use]
pub fn constant_name(pe: &PeImage<'_>, lp_object_info: Va, index: u16) -> Option<String> {
    let region = pe.region_at_va(constant(pe, lp_object_info, index)?)?;
    let mut name = String::new();
    for unit in 0..MAX_NAME_UNITS {
        let code = region.u16_le(Off::new(unit.checked_mul(2)?))?;
        if code == 0 {
            return (!name.is_empty()).then_some(name);
        }
        let character = char::from_u32(u32::from(code))
            .filter(|character| character.is_ascii_alphanumeric() || *character == '_')?;
        name.push(character);
    }
    None
}

/// Gives the string at `index` of the constant table: the UTF-16 characters
/// at the address of the entry, with their length in bytes in the 4 bytes
/// before them.
///
/// Gives `None` when the length is odd or longer than 64 KiB, when the file
/// does not hold all the bytes, and when the bytes are not valid UTF-16.
#[must_use]
pub fn constant_string(pe: &PeImage<'_>, lp_object_info: Va, index: u16) -> Option<String> {
    let start = constant(pe, lp_object_info, index)?;
    let length_at = Va::new(start.get().checked_sub(4)?);
    let region = pe.region_at_va(length_at)?;
    let bytes = region.u32_le(Off::new(0))?;
    if bytes.checked_rem(2) != Some(0) || bytes > MAX_STRING_BYTES {
        return None;
    }
    let text = region.take(Off::new(4), bytes)?;
    let units: Vec<u16> = text
        .chunks_exact(2)
        .map(|pair| Some(u16::from_le_bytes([*pair.first()?, *pair.get(1)?])))
        .collect::<Option<Vec<u16>>>()?;
    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .ok()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{
        class_reference_iid, constant, constant_guid, constant_name, constant_runtime_ordinal,
        constant_string,
    };
    use crate::read::pe::PeImage;
    use crate::read::region::Va;

    /// A one-section image: the section starts at RVA `0x1000` and file
    /// offset `0x400`, and holds `extra`. A local copy of the helper in
    /// `vb/procdesc.rs`, so the tests of the two files fail on their own.
    fn synthetic_image(extra: &[u8]) -> Vec<u8> {
        const LFANEW: usize = 0x40;
        const OPTIONAL: usize = LFANEW + 24;
        const SECTION: usize = OPTIONAL + 224;
        const SECTION_START: usize = 0x400;

        let mapped_len = u32::try_from(extra.len().max(0x10)).unwrap();
        let file_len = SECTION_START + usize::try_from(mapped_len).unwrap() + 0x10;
        let mut out = vec![0_u8; file_len];
        out[0] = b'M';
        out[1] = b'Z';
        out[0x3c..0x40].copy_from_slice(&u32::try_from(LFANEW).unwrap().to_le_bytes());
        out[LFANEW..LFANEW + 4].copy_from_slice(b"PE\0\0");
        out[LFANEW + 4..LFANEW + 6].copy_from_slice(&0x014c_u16.to_le_bytes());
        out[LFANEW + 6..LFANEW + 8].copy_from_slice(&1_u16.to_le_bytes());
        out[LFANEW + 20..LFANEW + 22].copy_from_slice(&224_u16.to_le_bytes());
        out[LFANEW + 22..LFANEW + 24].copy_from_slice(&0x0102_u16.to_le_bytes());
        out[OPTIONAL..OPTIONAL + 2].copy_from_slice(&0x010b_u16.to_le_bytes());
        out[OPTIONAL + 0x1c..OPTIONAL + 0x20].copy_from_slice(&0x0040_0000_u32.to_le_bytes());
        out[SECTION..SECTION + 8].copy_from_slice(b".text\0\0\0");
        out[SECTION + 8..SECTION + 12].copy_from_slice(&mapped_len.to_le_bytes());
        out[SECTION + 12..SECTION + 16].copy_from_slice(&0x1000_u32.to_le_bytes());
        out[SECTION + 16..SECTION + 20].copy_from_slice(&mapped_len.to_le_bytes());
        out[SECTION + 20..SECTION + 24]
            .copy_from_slice(&u32::try_from(SECTION_START).unwrap().to_le_bytes());
        out[SECTION + 36..SECTION + 40].copy_from_slice(&0x6000_0020_u32.to_le_bytes());
        out[SECTION_START..SECTION_START + extra.len()].copy_from_slice(extra);
        out
    }

    fn put_u32(extra: &mut [u8], at: usize, value: u32) {
        extra[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// `ObjectInfo` at `0x00` names the constant table at `0x40`. Entry 0
    /// names the string " px" at `0x84`, entry 1 an odd length at `0xA4`,
    /// and entry 2 a lone surrogate at `0xC4`.
    fn table() -> Vec<u8> {
        let mut extra = vec![0_u8; 0x100];
        put_u32(&mut extra, 0x34, 0x0040_1040);
        put_u32(&mut extra, 0x40, 0x0040_1084);
        put_u32(&mut extra, 0x44, 0x0040_10A4);
        put_u32(&mut extra, 0x48, 0x0040_10C4);
        put_u32(&mut extra, 0x80, 6);
        for (at, unit) in [(0x84, b' '), (0x86, b'p'), (0x88, b'x')] {
            extra[at] = unit;
        }
        put_u32(&mut extra, 0xA0, 3);
        put_u32(&mut extra, 0xC0, 2);
        extra[0xC4..0xC6].copy_from_slice(&0xD800_u16.to_le_bytes());
        extra
    }

    #[test]
    fn an_entry_names_the_characters_of_a_string_after_its_length() {
        let bytes = synthetic_image(&table());
        let pe = PeImage::parse(&bytes).unwrap();
        let info = Va::new(0x0040_1000);
        assert_eq!(constant(&pe, info, 0), Some(Va::new(0x0040_1084)));
        assert_eq!(constant_string(&pe, info, 0).as_deref(), Some(" px"));
    }

    const DIFFUSE_P_CODE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus-pcode/vb6-code/Diffuse-effect/Diffuse.exe"
    ));

    /// `frmDiffuse` in the P-code `Diffuse.exe`: its `ObjectInfo` is at
    /// `0x401b48`, and entry `0xE` of its constant table is the string of
    /// `lblX.Caption = hScrollX.Value & " px"` in `Diffuse.frm`.
    #[test]
    fn diffuse_gives_the_string_of_its_source() {
        let pe = PeImage::parse(DIFFUSE_P_CODE).unwrap();
        assert_eq!(
            constant_string(&pe, Va::new(0x0040_1B48), 0xE).as_deref(),
            Some(" px")
        );
    }

    /// `frmDiffuse` in the P-code `Diffuse.exe`: entry 2 of its constant
    /// table is the class reference of the global object of the runtime,
    /// whose interface `VBGlobal` has the GUID
    /// `{FCFB3D22-A0FA-1068-A738-08002B3371B5}`.
    #[test]
    fn diffuse_gives_the_interface_of_the_global_object() {
        let pe = PeImage::parse(DIFFUSE_P_CODE).unwrap();
        assert_eq!(
            class_reference_iid(&pe, Va::new(0x0040_1B48), 2),
            Some([
                0x22, 0x3D, 0xFB, 0xFC, 0xFA, 0xA0, 0x68, 0x10, 0xA7, 0x38, 0x08, 0x00, 0x2B, 0x33,
                0x71, 0xB5
            ])
        );
        assert_eq!(class_reference_iid(&pe, Va::new(0x0040_1B48), 0xE), None);
    }

    const PASSGEN_P_CODE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus-pcode/public-domain/PassGen/PassGen.exe"
    ));

    /// `frmPassGen` in the P-code `PassGen.exe`: its `ObjectInfo` is at
    /// `0x40599c`, and entry `0x43` of its constant table is the name of
    /// `qS.RegWrite ...` in `frmPassGen.frm`. Entry `0x42` is the string
    /// "REG_DWORD", whose length comes before it and is not a name.
    #[test]
    fn passgen_gives_the_name_of_a_late_member() {
        let pe = PeImage::parse(PASSGEN_P_CODE).unwrap();
        let info = Va::new(0x0040_599C);
        assert_eq!(constant_name(&pe, info, 0x43).as_deref(), Some("RegWrite"));
        assert_eq!(
            constant_string(&pe, info, 0x42).as_deref(),
            Some("REG_DWORD")
        );
    }

    /// `frmPassGen` in the P-code `PassGen.exe`: entry `0x3D` of its
    /// constant table is a jump through the slot of the ordinal 685 of the
    /// runtime, `rtcErrObj`. Entry `0x43` is a name, not a jump.
    #[test]
    fn passgen_gives_the_ordinal_of_an_import_call() {
        let pe = PeImage::parse(PASSGEN_P_CODE).unwrap();
        let info = Va::new(0x0040_599C);
        assert_eq!(constant_runtime_ordinal(&pe, info, 0x3D), Some(685));
        assert_eq!(constant_runtime_ordinal(&pe, info, 0x43), None);
    }

    #[test]
    fn a_name_needs_a_zero_unit_and_only_the_characters_of_a_name() {
        let bytes = synthetic_image(&table());
        let pe = PeImage::parse(&bytes).unwrap();
        let info = Va::new(0x0040_1000);
        assert_eq!(constant_name(&pe, info, 0), None);
        assert_eq!(constant_name(&pe, info, 2), None);
        let mut extra = table();
        extra[0x84..0x8A].copy_from_slice(&[b'R', 0, b'u', 0, b'n', 0]);
        let bytes = synthetic_image(&extra);
        let pe = PeImage::parse(&bytes).unwrap();
        assert_eq!(constant_name(&pe, info, 0).as_deref(), Some("Run"));
    }

    /// `frmDiffuse` in the P-code `Diffuse.exe`: `VCallHresult` at `0x0014`
    /// of its first body names entry 4, the GUID
    /// `{33AD4F79-6699-11CF-B70C-00AA0060D393}` of the interface of `App`.
    #[test]
    fn diffuse_gives_the_interface_of_a_call() {
        let pe = PeImage::parse(DIFFUSE_P_CODE).unwrap();
        assert_eq!(
            constant_guid(&pe, Va::new(0x0040_1B48), 4),
            Some([
                0x79, 0x4F, 0xAD, 0x33, 0x99, 0x66, 0xCF, 0x11, 0xB7, 0x0C, 0x00, 0xAA, 0x00, 0x60,
                0xD3, 0x93
            ])
        );
        assert_eq!(constant_guid(&pe, Va::new(0x0090_0000), 4), None);
    }

    #[test]
    fn an_odd_length_a_bad_character_or_a_missing_entry_gives_no_string() {
        let bytes = synthetic_image(&table());
        let pe = PeImage::parse(&bytes).unwrap();
        let info = Va::new(0x0040_1000);
        assert_eq!(constant_string(&pe, info, 1), None);
        assert_eq!(constant_string(&pe, info, 2), None);
        assert_eq!(constant_string(&pe, info, 0x100), None);
        assert_eq!(constant_string(&pe, Va::new(0x0090_0000), 0), None);
    }
}
