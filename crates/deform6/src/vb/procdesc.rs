//! The method table of an object, and the P-code procedure descriptors that
//! it names.
//!
//! `STRUCTURES.md` section 5.2 puts `wMethodCount` at `ObjectInfo + 0x20` and
//! `lpMethods` at `ObjectInfo + 0x24`. In a P-code build, `lpMethods` names
//! an array of `wMethodCount` values, and a value that is an address names a
//! `ProcDscInfo` (section 10.3). The P-code body of the procedure is the
//! `ProcSize` bytes immediately before its descriptor.
//!
//! # What the P-code corpus measured
//!
//! The 99 objects of the 42 programs in `corpus-pcode/` hold 871 method table
//! entries. 680 are addresses, and each names a descriptor whose body fits in
//! the section before it. The other 191 map into no section, and each of them
//! comes before the first address in its table. A value that maps into no
//! section is therefore not a fault: [`MethodEntry::NotAnAddress`] keeps it,
//! and no defect is raised.
//!
//! The number of descriptors equals `Object.ProcCount` in only 49 of the 99
//! objects, so this reader takes the entries that are addresses and does not
//! count on `ProcCount`.
//!
//! # A native build is not read
//!
//! In a native build the entries of `lpMethods` are not descriptors. The
//! corpus holds text and small numbers there. A caller reads a method table
//! only for a program whose `ProjectInfo.lpNativeCode` is 0.

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Rva, Va};

/// `STRUCTURES.md` section 5.2: `wMethodCount` sits at `ObjectInfo + 0x20`.
const W_METHOD_COUNT_AT: u32 = 0x20;

/// `STRUCTURES.md` section 5.2: `lpMethods` sits at `ObjectInfo + 0x24`.
const LP_METHODS_AT: u32 = 0x24;

/// The width of one method table entry: a four-byte value.
const METHOD_ENTRY_SIZE: u32 = 4;

/// The bytes of a `ProcDscInfo` that this reader reads: `ProcTable`, the
/// argument size, `FrameSize` and `ProcSize` (`STRUCTURES.md` section 10.3).
pub const PROC_DESC_READ_LEN: u32 = 0x0A;

/// A `ProcDscInfo`, the descriptor of one P-code procedure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProcDescriptor {
    /// The address of the descriptor, as the method table holds it.
    pub va: Va,
    /// `ProcTable` at `+0x00`. In each of the 680 descriptors of the P-code
    /// corpus, it is the address of the `ObjectInfo` of the object whose
    /// method table names the descriptor. `STRUCTURES.md` section 10.3 gives
    /// it as a 56-byte table, and `ObjectInfo` is 56 bytes.
    pub proc_table: Va,
    /// The word at `+0x04`: the bytes of the arguments that a caller pushes.
    /// That is 4 for `Me`, 4 for each argument by reference, the size of the
    /// value for each argument by value (8 for a `Double`, 16 for a
    /// `Variant`), and 4 for the address of the result of a `Function` or a
    /// `Property Get`. Each of the 680 descriptors of the P-code corpus
    /// agrees with the declaration of its procedure in the source.
    ///
    /// A procedure of a standard module has no `Me`, and its word still
    /// counts those 4 bytes: the 17 procedures of the 8 standard modules of
    /// the corpus agree with the same rule.
    pub arg_size: u16,
    /// `FrameSize` at `+0x06`.
    pub frame_size: u16,
    /// `ProcSize` at `+0x08`: the length of the P-code body, which is the
    /// bytes immediately before the descriptor.
    pub proc_size: u16,
}

impl ProcDescriptor {
    /// Reads the descriptor at `va`.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when `va` is in no section, and when the
    /// file holds fewer than [`PROC_DESC_READ_LEN`] bytes there.
    pub fn read(pe: &PeImage<'_>, va: Va) -> Result<Self, Refusal> {
        let at = pe
            .region_at_va(va)
            .ok_or(Refusal::Damaged("a procedure descriptor is in no section"))?;
        let window = at
            .subregion(Off::new(0), PROC_DESC_READ_LEN)
            .ok_or(Refusal::Damaged(
                "the file ends inside a procedure descriptor",
            ))?;
        let field = |at: u32| {
            window
                .u16_le(Off::new(at))
                .ok_or(Refusal::Damaged("a procedure descriptor field is cut"))
        };
        Ok(Self {
            va,
            proc_table: window
                .va_le(Off::new(0x00))
                .ok_or(Refusal::Damaged("a procedure descriptor field is cut"))?,
            arg_size: field(0x04)?,
            frame_size: field(0x06)?,
            proc_size: field(0x08)?,
        })
    }

    /// Gives the P-code body: the [`ProcDescriptor::proc_size`] bytes
    /// immediately before the descriptor.
    ///
    /// Gives `None` when the body would start below address 0, when its
    /// start is in no section, and when the body and the descriptor are not
    /// adjacent in the file, for example because the body crosses the start
    /// of the section of the descriptor.
    #[must_use]
    pub fn body<'a>(&self, pe: &PeImage<'a>) -> Option<Region<'a>> {
        let size = u32::from(self.proc_size);
        let start = self.va.get().checked_sub(size)?;
        let body = pe
            .region_at_va(Va::new(start))?
            .subregion(Off::new(0), size)?;
        let descriptor = pe.region_at_va(self.va)?;
        (body.file_offset(Off::new(size))? == descriptor.file_offset(Off::new(0))?).then_some(body)
    }
}

/// A fixed-size local array of a procedure, which its descriptor holds:
/// `Dim bTable(0 To 255) As Long`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixedArray {
    /// The offset of the frame slot of the array below the frame, such as
    /// `0xB0` for `-0xB0`.
    pub slot: u16,
    /// The `VARTYPE` of the elements, such as 3 for `Long`.
    pub vartype: u16,
    /// The number of elements and the lower bound of each dimension, first
    /// dimension first.
    pub bounds: Vec<(u32, i32)>,
    /// The bytes of one element, `cbElements` of the template.
    pub element_bytes: u32,
}

impl FixedArray {
    /// Builds a fixed array from its slot, its `VARTYPE` and its bounds.
    #[must_use]
    pub const fn new(slot: u16, vartype: u16, bounds: Vec<(u32, i32)>) -> Self {
        Self {
            slot,
            vartype,
            bounds,
            element_bytes: 0,
        }
    }

    /// Gives the array with `bytes` as the bytes of one element.
    #[must_use]
    pub fn with_element_bytes(mut self, bytes: u32) -> Self {
        self.element_bytes = bytes;
        self
    }
}

/// The place of a table of fixed-size arrays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    /// The frame of a procedure: an entry names a slot below the frame, as a
    /// negative offset, and the elements have a `VARTYPE`.
    Frame,
    /// The public variables of an object: an entry names the offset of the
    /// variable in the data of the object, and the elements can be records
    /// of a user type, which have no `VARTYPE`.
    Module,
}

/// The `SAFEARRAY` feature that marks a valid `VARTYPE` after the template:
/// `FADF_HAVEVARTYPE`.
const FADF_HAVEVARTYPE: u16 = 0x80;

/// The largest number of bytes of the table of the public variables of an
/// object that [`module_fixed_arrays`] reads. The first word of the table
/// gives its bytes: `0x5C` for the two entries of the module of
/// `Physics_Demo.exe`, and `0x0C`, the header alone, for its form.
const MODULE_TABLE_MAX: u32 = 0x400;

/// The offset of the first entry of the table of the public variables of an
/// object, after its header of twelve bytes.
const MODULE_TABLE_ENTRIES: u32 = 0x0C;

/// The largest element of a record that [`module_fixed_arrays`] accepts.
const MAX_RECORD_BYTES: u32 = 0x1_0000;

/// Reads the fixed-size arrays of the table of the public variables of an
/// object at `va`, its `lpPublicBytes`. Each entry is the offset of the
/// variable and the kind 5, and a `SAFEARRAY` template follows 16 bytes
/// after it, as in the frame of a procedure. The elements of
/// `StarArray(0 To 500) As Star` of `Physics_Demo.exe` are records of 16
/// bytes, so the template has no `VARTYPE`, and the array gives 0 there.
/// The scan stops at the bytes that the first word of the table gives. An
/// address with no table gives nothing.
#[must_use]
pub fn module_fixed_arrays(pe: &PeImage<'_>, va: Va) -> Vec<FixedArray> {
    if va.get() == 0 {
        return Vec::new();
    }
    let Some(head) = pe.region_at_va(va) else {
        return Vec::new();
    };
    let Some(stated) = head.u16_le(Off::new(0)) else {
        return Vec::new();
    };
    let length = head.len().min(MODULE_TABLE_MAX).min(u32::from(stated));
    let Some(table) = head.subregion(Off::new(0), length) else {
        return Vec::new();
    };
    let mut out: Vec<FixedArray> = Vec::new();
    let mut at = MODULE_TABLE_ENTRIES;
    while at < length {
        if let Some(array) = fixed_array_at(&table, at, Place::Module) {
            out.push(array);
        }
        at = at.saturating_add(2);
    }
    out
}

/// The `SAFEARRAY` feature that marks a fixed-size array: `FADF_FIXEDSIZE`.
const FADF_FIXEDSIZE: u16 = 0x10;

/// The low byte of the kind of an entry of the frame table of a descriptor
/// that names an array. The high byte holds flags: the corpus holds 0x0005
/// and 0x2005 for a fixed-size array. The template decides.
const ARRAY_KIND: u16 = 5;

/// The bytes between a fixed-array entry and its `SAFEARRAY` template.
const TEMPLATE_GAP: u32 = 16;

/// The most dimensions that the reader takes for one template.
const MAX_DIMENSIONS: u16 = 8;

impl ProcDescriptor {
    /// Gives the fixed-size local arrays of the procedure.
    ///
    /// The word at `+0x0A` is the length of the descriptor. After `+0x0C`,
    /// the descriptor holds a table of frame slots. An entry of an array is
    /// the slot as a negative 16-bit offset, then a kind whose low byte is 5.
    /// Sixteen bytes after its start, a `SAFEARRAY` template follows: the number of
    /// dimensions, the features, the bytes of an element, the locks and the
    /// data, both 0, and the number of elements and the lower bound of each
    /// dimension. The `VARTYPE` of the elements follows the template.
    /// `STRUCTURES.md` section 23d gives the measure. An entry whose template
    /// does not have this shape, or does not fit in the descriptor, gives
    /// nothing.
    #[must_use]
    pub fn fixed_arrays(&self, pe: &PeImage<'_>) -> Vec<FixedArray> {
        let Some(head) = pe.region_at_va(self.va) else {
            return Vec::new();
        };
        let Some(length) = head.u16_le(Off::new(PROC_DESC_READ_LEN)) else {
            return Vec::new();
        };
        let Some(descriptor) = head.subregion(Off::new(0), u32::from(length)) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut at = PROC_DESC_READ_LEN.saturating_add(2);
        while at < u32::from(length) {
            if let Some(array) = fixed_array_at(&descriptor, at, Place::Frame) {
                out.push(array);
            }
            at = at.saturating_add(2);
        }
        out
    }
}

/// Reads a fixed-array entry and its template at `at` in `descriptor`.
fn fixed_array_at(descriptor: &Region<'_>, at: u32, place: Place) -> Option<FixedArray> {
    let slot = i16::from_le_bytes(descriptor.u16_le(Off::new(at))?.to_le_bytes());
    let named = match place {
        Place::Frame => slot < 0,
        Place::Module => slot > 0,
    };
    if !named || descriptor.u16_le(Off::new(at.checked_add(2)?))? & 0xFF != ARRAY_KIND {
        return None;
    }
    let template = at.checked_add(TEMPLATE_GAP)?;
    let word = |offset: u32| descriptor.u16_le(Off::new(template.checked_add(offset)?));
    let long = |offset: u32| descriptor.u32_le(Off::new(template.checked_add(offset)?));
    let dimensions = word(0)?;
    let features = word(2)?;
    let element = long(4)?;
    let record = place == Place::Module && features & FADF_HAVEVARTYPE == 0;
    let element_fits = if record {
        (1..=MAX_RECORD_BYTES).contains(&element)
    } else {
        matches!(element, 1 | 2 | 4 | 8 | 16)
    };
    if !(1..=MAX_DIMENSIONS).contains(&dimensions)
        || features & FADF_FIXEDSIZE == 0
        || !element_fits
        || long(8)? != 0
        || long(12)? != 0
    {
        return None;
    }
    let mut bounds = Vec::new();
    for dimension in 0..u32::from(dimensions) {
        let base = dimension.checked_mul(8)?.checked_add(16)?;
        let count = long(base)?;
        let lower = descriptor.i32_le(Off::new(template.checked_add(base)?.checked_add(4)?))?;
        if count == 0 {
            return None;
        }
        bounds.push((count, lower));
    }
    let end = u32::from(dimensions).checked_mul(8)?.checked_add(16)?;
    let vartype = if record { 0 } else { word(end)? };
    Some(FixedArray {
        slot: slot.unsigned_abs(),
        vartype,
        bounds,
        element_bytes: element,
    })
}

/// One entry of a method table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MethodEntry {
    /// The value is an address, and it names a descriptor whose body fits
    /// before it.
    Descriptor {
        /// The index of the entry in the table.
        index: u16,
        /// The descriptor.
        descriptor: ProcDescriptor,
    },
    /// The value maps into no section. The corpus holds such values before
    /// the first descriptor of a table, and they are not a fault.
    NotAnAddress {
        /// The index of the entry in the table.
        index: u16,
        /// The value, as the file holds it.
        value: u32,
    },
    /// The value maps into a section, and the descriptor or its body cannot
    /// be read there. [`MethodTable::defects`] gives the reason.
    Unreadable {
        /// The index of the entry in the table.
        index: u16,
        /// The address that the entry holds.
        va: Va,
    },
}

/// The method table of one object.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MethodTable {
    /// Each entry, by ascending index.
    pub entries: Vec<MethodEntry>,
    defects: Vec<Defect>,
}

impl MethodTable {
    /// Gives the defects that the read found: a `wMethodCount` too large for
    /// the file to hold, and each entry whose descriptor or body cannot be
    /// read.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }

    /// Gives each descriptor, by ascending index.
    pub fn descriptors(&self) -> impl Iterator<Item = &ProcDescriptor> {
        self.entries.iter().filter_map(|entry| match entry {
            MethodEntry::Descriptor { descriptor, .. } => Some(descriptor),
            MethodEntry::NotAnAddress { .. } | MethodEntry::Unreadable { .. } => None,
        })
    }
}

/// Reads the method table of the object whose `ObjectInfo` is at
/// `lp_object_info`.
///
/// `wMethodCount` is bounded by the bytes that the section of `lpMethods`
/// holds, before it bounds a loop. A table whose count is 0, or whose
/// `lpMethods` is 0, is empty and is not an error.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when `ObjectInfo` is in no section or is cut
/// before `lpMethods`, and when a non-empty table has its `lpMethods` in no
/// section.
pub fn read_method_table(pe: &PeImage<'_>, lp_object_info: Va) -> Result<MethodTable, Refusal> {
    let info = pe
        .region_at_va(lp_object_info)
        .ok_or(Refusal::Damaged("the ObjectInfo pointer is in no section"))?;
    let count = info
        .u16_le(Off::new(W_METHOD_COUNT_AT))
        .ok_or(Refusal::Damaged("ObjectInfo holds no wMethodCount"))?;
    let lp_methods = info
        .va_le(Off::new(LP_METHODS_AT))
        .ok_or(Refusal::Damaged("ObjectInfo holds no lpMethods"))?;
    if count == 0 || lp_methods.is_null() {
        return Ok(MethodTable::default());
    }
    let table = pe
        .region_at_va(lp_methods)
        .ok_or(Refusal::Damaged("the lpMethods pointer is in no section"))?;

    let mut defects = Vec::new();
    let max = table.len().checked_div(METHOD_ENTRY_SIZE).unwrap_or(0);
    let count = if u32::from(count) <= max {
        count
    } else {
        let offset = info
            .file_offset(Off::new(W_METHOD_COUNT_AT))
            .map_or(0, Off::get);
        defects.push(Defect {
            site: Site {
                offset,
                rva: info.rva(Off::new(W_METHOD_COUNT_AT)).map(Rva::get),
                structure: "ObjectInfo",
                field: "wMethodCount",
            },
            kind: DefectKind::ImplausibleCount {
                offset,
                count: u32::from(count),
                max,
            },
        });
        u16::try_from(max).unwrap_or(u16::MAX)
    };

    let mut entries = Vec::new();
    for index in 0..count {
        let at = Off::new(
            u32::from(index)
                .checked_mul(METHOD_ENTRY_SIZE)
                .ok_or(Refusal::Damaged("a method table index overflows a u32"))?,
        );
        let value = table
            .u32_le(at)
            .ok_or(Refusal::Damaged("the file ends inside the method table"))?;
        let va = Va::new(value);
        if pe.region_at_va(va).is_none() {
            entries.push(MethodEntry::NotAnAddress { index, value });
            continue;
        }
        let slot = || Site {
            offset: table.file_offset(at).map_or(0, Off::get),
            rva: table.rva(at).map(Rva::get),
            structure: "MethodTable",
            field: "entry",
        };
        let read = ProcDescriptor::read(pe, va);
        match read {
            Ok(descriptor) if descriptor.body(pe).is_some() => {
                entries.push(MethodEntry::Descriptor { index, descriptor });
            }
            Ok(descriptor) => {
                defects.push(Defect {
                    site: slot(),
                    kind: DefectKind::ItemCutShort {
                        offset: slot().offset,
                        va: value.saturating_sub(u32::from(descriptor.proc_size)),
                        len: u32::from(descriptor.proc_size),
                    },
                });
                entries.push(MethodEntry::Unreadable { index, va });
            }
            Err(_) => {
                defects.push(Defect {
                    site: slot(),
                    kind: DefectKind::ItemCutShort {
                        offset: slot().offset,
                        va: value,
                        len: PROC_DESC_READ_LEN,
                    },
                });
                entries.push(MethodEntry::Unreadable { index, va });
            }
        }
    }
    Ok(MethodTable { entries, defects })
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
    use super::{MethodEntry, Place, ProcDescriptor, fixed_array_at, read_method_table};
    use crate::error::{Defect, DefectKind, Site};
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Region, Va};

    /// A fixed-array entry for the slot `-0xB0` with the kind `kind`, and its
    /// template of 256 elements of 4 bytes from 0, with the features
    /// `features` and the `VARTYPE` 3.
    fn array_entry(kind: u16, features: u16) -> Vec<u8> {
        let mut bytes = vec![0x50, 0xFF];
        bytes.extend_from_slice(&kind.to_le_bytes());
        bytes.extend_from_slice(&[0; 12]);
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&features.to_le_bytes());
        bytes.extend_from_slice(&4_u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&256_u32.to_le_bytes());
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes.extend_from_slice(&3_u16.to_le_bytes());
        bytes
    }

    #[test]
    fn a_fixed_array_entry_needs_its_kind_and_a_fixed_size_template() {
        let read = |bytes: &[u8]| fixed_array_at(&Region::new(bytes, Off::new(0)), 0, Place::Frame);
        let array = read(&array_entry(0x2005, 0x92)).unwrap();
        assert_eq!((array.slot, array.vartype), (0xB0, 3));
        assert_eq!(array.bounds, [(256, 0)]);
        // No FADF_FIXEDSIZE: a dynamic array.
        assert!(read(&array_entry(5, 0x82)).is_none());
        // Another kind.
        assert!(read(&array_entry(3, 0x92)).is_none());
        // A template cut by the end of the descriptor.
        let entry = array_entry(5, 0x92);
        assert!(read(entry.get(..entry.len() - 2).unwrap()).is_none());
    }

    /// The entry of `StarArray(0 To 500) As Star` in the table of the public
    /// variables of the module of `Physics_Demo.exe`: the offset 0x48, the
    /// kind 5, and a template of 501 records of 16 bytes with the features
    /// 0x12, so no `VARTYPE`.
    fn module_entry(slot: u16, features: u16) -> Vec<u8> {
        let mut bytes = slot.to_le_bytes().to_vec();
        bytes.extend_from_slice(&5_u16.to_le_bytes());
        bytes.extend_from_slice(&[0xFF; 12]);
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&features.to_le_bytes());
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&501_u32.to_le_bytes());
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes
    }

    #[test]
    fn a_module_entry_names_a_positive_offset_and_can_hold_records() {
        let read = |bytes: &[u8], place| fixed_array_at(&Region::new(bytes, Off::new(0)), 0, place);
        let array = read(&module_entry(0x48, 0x12), Place::Module).unwrap();
        assert_eq!(
            (array.slot, array.vartype, array.element_bytes),
            (0x48, 0, 16)
        );
        assert_eq!(array.bounds, [(501, 0)]);
        // The frame of a procedure names a negative slot.
        assert!(read(&module_entry(0x48, 0x12), Place::Frame).is_none());
        // A module names a positive offset.
        assert!(read(&module_entry(0xFFB8, 0x12), Place::Module).is_none());
        // With FADF_HAVEVARTYPE, an element of 16 bytes is a Variant, and the
        // VARTYPE must follow the bounds.
        assert!(read(&module_entry(0x48, 0x92), Place::Module).is_none());
    }

    /// A one-section image: the section starts at RVA `0x1000` and file
    /// offset `0x400`, and holds `extra`. A local copy of the helper in
    /// `vb/controlinfo.rs`, so the tests of the two files fail on their own.
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

    fn put_u16(extra: &mut [u8], at: usize, value: u16) {
        extra[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(extra: &mut [u8], at: usize, value: u32) {
        extra[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// `ObjectInfo` at `0x00` names a table of three entries at `0x40`: a
    /// value that is not an address, a descriptor at `0x80` with a body of
    /// 16 bytes at `0x70`, and a descriptor at `0x100` whose body of `0x200`
    /// bytes would start before the section.
    fn three_entries() -> Vec<u8> {
        let mut extra = vec![0_u8; 0x120];
        put_u16(&mut extra, 0x20, 3);
        put_u32(&mut extra, 0x24, 0x0040_1040);
        put_u32(&mut extra, 0x40, 0xD8);
        put_u32(&mut extra, 0x44, 0x0040_1080);
        put_u32(&mut extra, 0x48, 0x0040_1100);
        for (at, byte) in (0x70..0x80).zip(1_u8..) {
            extra[at] = byte;
        }
        put_u32(&mut extra, 0x80, 0x0040_10C0);
        put_u16(&mut extra, 0x84, 4);
        put_u16(&mut extra, 0x86, 0x80);
        put_u16(&mut extra, 0x88, 0x10);
        put_u32(&mut extra, 0x100, 0x0040_10C0);
        put_u16(&mut extra, 0x108, 0x200);
        extra
    }

    #[test]
    fn each_entry_is_a_descriptor_a_value_that_is_not_an_address_or_unreadable() {
        let bytes = synthetic_image(&three_entries());
        let pe = PeImage::parse(&bytes).unwrap();
        let table = read_method_table(&pe, Va::new(0x0040_1000)).unwrap();
        assert_eq!(
            table.entries,
            vec![
                MethodEntry::NotAnAddress {
                    index: 0,
                    value: 0xD8
                },
                MethodEntry::Descriptor {
                    index: 1,
                    descriptor: ProcDescriptor {
                        va: Va::new(0x0040_1080),
                        proc_table: Va::new(0x0040_10C0),
                        arg_size: 4,
                        frame_size: 0x80,
                        proc_size: 0x10,
                    },
                },
                MethodEntry::Unreadable {
                    index: 2,
                    va: Va::new(0x0040_1100),
                },
            ]
        );
        assert_eq!(
            table.defects(),
            [Defect {
                site: Site {
                    offset: 0x448,
                    rva: Some(0x1048),
                    structure: "MethodTable",
                    field: "entry",
                },
                kind: DefectKind::ItemCutShort {
                    offset: 0x448,
                    va: 0x0040_0F00,
                    len: 0x200,
                },
            }]
        );
    }

    #[test]
    fn the_body_is_the_proc_size_bytes_immediately_before_the_descriptor() {
        let extra = three_entries();
        let bytes = synthetic_image(&extra);
        let pe = PeImage::parse(&bytes).unwrap();
        let table = read_method_table(&pe, Va::new(0x0040_1000)).unwrap();
        let descriptor = table.descriptors().next().unwrap();
        let body = descriptor.body(&pe).unwrap();
        assert_eq!(body.len(), 0x10);
        assert_eq!(body.take(Off::new(0), 0x10), Some(&extra[0x70..0x80]));
        assert_eq!(body.file_offset(Off::new(0)), Some(Off::new(0x470)));
    }

    #[test]
    fn a_count_too_large_for_the_section_is_clamped_with_a_defect() {
        let mut extra = three_entries();
        put_u16(&mut extra, 0x20, 0xFFFF);
        let bytes = synthetic_image(&extra);
        let pe = PeImage::parse(&bytes).unwrap();
        let table = read_method_table(&pe, Va::new(0x0040_1000)).unwrap();
        // 0xE0 bytes remain from 0x40 to the end of the section: 56 entries.
        assert_eq!(table.entries.len(), 56);
        assert_eq!(
            table.defects()[0].kind,
            DefectKind::ImplausibleCount {
                offset: 0x420,
                count: 0xFFFF,
                max: 56,
            }
        );
    }

    #[test]
    fn a_count_of_zero_or_a_null_table_gives_an_empty_table_and_no_defect() {
        for (count, lp_methods) in [(0, 0x0040_1040), (3, 0)] {
            let mut extra = three_entries();
            put_u16(&mut extra, 0x20, count);
            put_u32(&mut extra, 0x24, lp_methods);
            let bytes = synthetic_image(&extra);
            let pe = PeImage::parse(&bytes).unwrap();
            let table = read_method_table(&pe, Va::new(0x0040_1000)).unwrap();
            assert!(table.entries.is_empty());
            assert!(table.defects().is_empty());
        }
    }

    #[test]
    fn a_descriptor_that_the_section_cuts_is_unreadable() {
        let mut extra = three_entries();
        // The last entry names the last 4 bytes of the section.
        put_u32(&mut extra, 0x48, 0x0040_111C);
        let bytes = synthetic_image(&extra);
        let pe = PeImage::parse(&bytes).unwrap();
        let table = read_method_table(&pe, Va::new(0x0040_1000)).unwrap();
        assert_eq!(
            table.entries[2],
            MethodEntry::Unreadable {
                index: 2,
                va: Va::new(0x0040_111C),
            }
        );
        assert_eq!(
            table.defects()[0].kind,
            DefectKind::ItemCutShort {
                offset: 0x448,
                va: 0x0040_111C,
                len: 0x0A,
            }
        );
    }

    const FAST_FLAMES_P_CODE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus-pcode/vb6-code/Fire-effect/Fast_Flames.exe"
    ));

    /// `frmFire` in the P-code `Fast_Flames.exe`: its `ObjectInfo` is at
    /// `0x402788`, and its method table holds five values that are not
    /// addresses, then the five descriptors of its five source procedures.
    #[test]
    fn fast_flames_frm_fire_gives_five_values_then_five_descriptors() {
        let pe = PeImage::parse(FAST_FLAMES_P_CODE).unwrap();
        let table = read_method_table(&pe, Va::new(0x0040_2788)).unwrap();
        assert!(table.defects().is_empty());
        let values: Vec<bool> = table
            .entries
            .iter()
            .map(|entry| matches!(entry, MethodEntry::NotAnAddress { .. }))
            .collect();
        assert_eq!(
            values,
            [
                true, true, true, true, true, false, false, false, false, false
            ]
        );
        let cmd_start = table.descriptors().next().unwrap();
        assert_eq!(
            *cmd_start,
            ProcDescriptor {
                va: Va::new(0x0040_3B90),
                proc_table: Va::new(0x0040_2788),
                arg_size: 4,
                frame_size: 128,
                proc_size: 400,
            }
        );
        let body = cmd_start.body(&pe).unwrap();
        assert_eq!(body.file_offset(Off::new(0)), Some(Off::new(0x3A00)));
        assert_eq!(body.take(Off::new(398), 2), Some(&[0x13, 0x00][..]));
    }
}
