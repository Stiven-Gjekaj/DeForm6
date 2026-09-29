//! The method link table of an object: the part of its vtable that the
//! source declares.
//!
//! `STRUCTURES.md` section 5.3 puts three fields in `OptionalObjectInfo`:
//! a count at `+0x28`, a count at `+0x2A` and `lpMethodLinkTable` at
//! `+0x30`. A P-code call to a method of `Me`, such as `ThisVCallHresult`,
//! names the method by its offset in the vtable of the object. This module
//! turns that offset into the descriptor of the method.
//!
//! # What the P-code corpus measured
//!
//! - The vtable starts with the 7 methods of `IDispatch`. Then come the
//!   methods of the base interface: the count at `+0x2A` gives them, 439 for
//!   a form and 0 for a class. Then come the slots of the link table: the
//!   count at `+0x28` gives them.
//! - An entry of the link table is an address. For a method, the address is
//!   7 bytes into the P-code stub of the method, where the stub reads
//!   `33 C0 BA <descriptor> 68 <engine> C3` (`STRUCTURES.md` section 22).
//! - In each of the 91 objects of the corpus that are not standard modules,
//!   the other slots number two for each public variable of the source, and
//!   they come first. The method slots follow: the public
//!   procedures of the source, then the private ones, each group in the
//!   order of the file.
//! - Each of the 465 `ThisVCallHresult` offsets of the corpus names a
//!   method slot of its own object.
//!
//! A standard module has no `OptionalObjectInfo`, and no link table.

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Rva, Va};
use crate::vb::classify::has_optional_info;
use crate::vb::lift::Callees;
use crate::vb::object::Object;
use crate::vb::procdesc::{MethodEntry, MethodTable};

/// `STRUCTURES.md` section 5.3: `OptionalObjectInfo` sits at
/// `Object.lpObjectInfo + 0x38`.
const OPTIONAL_OBJECT_INFO_AT: u32 = 0x38;

/// The count of the link table, at `OptionalObjectInfo + 0x28`.
const LINK_COUNT_AT: u32 = 0x28;

/// The count of the methods of the base interface, at
/// `OptionalObjectInfo + 0x2A`.
const BASE_COUNT_AT: u32 = 0x2A;

/// `lpMethodLinkTable`, at `OptionalObjectInfo + 0x30`.
const LINK_TABLE_AT: u32 = 0x30;

/// The methods of `IDispatch`, which come before the base interface:
/// `QueryInterface`, `AddRef`, `Release`, `GetTypeInfoCount`, `GetTypeInfo`,
/// `GetIDsOfNames` and `Invoke`.
const IDISPATCH_METHODS: u32 = 7;

/// The width of one vtable slot and of one link table entry.
const SLOT_SIZE: u32 = 4;

/// The bytes of the tail of a P-code stub that a method slot names:
/// `xor eax, eax`, `mov edx, <descriptor>`, `push <engine>`, `ret`.
const STUB_TAIL_LEN: u32 = 13;

/// One slot of the link table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkSlot {
    /// The slot names a P-code stub, and the stub names this descriptor.
    Method(Va),
    /// The slot names the accessor of a public variable:
    /// `add dword ptr [esp+4], <field>`, `mov ecx, <thunk>`, `jmp ecx`. The
    /// accessor adds the offset of the field of the variable to the object,
    /// and goes to a function of the runtime that gets or lets the field.
    Variable {
        /// The offset of the field in the object.
        field: u32,
    },
    /// The slot names something else.
    Other,
}

/// The method link table of one object.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MethodLinks {
    /// The number of vtable slots before the link table: the methods of
    /// `IDispatch` and of the base interface.
    pub first_slot: u32,
    /// Each slot, by ascending index.
    pub slots: Vec<LinkSlot>,
    defects: Vec<Defect>,
}

impl MethodLinks {
    /// Gives the defects that the read found: a count too large for the
    /// file to hold.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }

    /// Gives the descriptor of the method at `vtable_offset`, a byte offset
    /// in the vtable of the object. Gives `None` for an offset that is not
    /// a multiple of 4, that is before the link table or after it, and for a
    /// slot that names no method.
    #[must_use]
    pub fn method_at(&self, vtable_offset: u16) -> Option<Va> {
        let offset = u32::from(vtable_offset);
        if offset.checked_rem(SLOT_SIZE)? != 0 {
            return None;
        }
        let index = offset
            .checked_div(SLOT_SIZE)?
            .checked_sub(self.first_slot)?;
        match self.slots.get(usize::try_from(index).ok()?)? {
            LinkSlot::Method(descriptor) => Some(*descriptor),
            LinkSlot::Variable { .. } | LinkSlot::Other => None,
        }
    }
}

impl MethodLinks {
    /// Gives the methods of `methods` that a `ThisVCallHresult` can call:
    /// each method slot whose descriptor `methods` holds, by its vtable
    /// offset, with the index and the argument size of the descriptor. A
    /// slot whose offset does not fit in 16 bits is left out.
    ///
    /// It also gives the accessors of the public variables. They come in
    /// pairs, the get and then the let of one variable, and the two of a
    /// pair name the same field.
    #[must_use]
    pub fn callees(&self, methods: &MethodTable) -> Callees {
        let mut callees = Callees::default();
        let mut variables = 0_u32;
        for (slot, link) in (self.first_slot..).zip(&self.slots) {
            let Some(offset) = slot
                .checked_mul(SLOT_SIZE)
                .and_then(|offset| u16::try_from(offset).ok())
            else {
                continue;
            };
            let va = match link {
                LinkSlot::Method(va) => va,
                LinkSlot::Variable { field } => {
                    callees = callees.with_variable(offset, *field, variables.is_multiple_of(2));
                    variables = variables.saturating_add(1);
                    continue;
                }
                LinkSlot::Other => continue,
            };
            let found = methods.entries.iter().find_map(|entry| match entry {
                MethodEntry::Descriptor { index, descriptor } if descriptor.va == *va => {
                    Some((*index, descriptor.arg_size))
                }
                MethodEntry::Descriptor { .. }
                | MethodEntry::NotAnAddress { .. }
                | MethodEntry::Unreadable { .. } => None,
            });
            if let Some((index, arg_size)) = found {
                callees = callees.with_method(offset, index, arg_size);
            }
        }
        callees
    }
}

/// The bytes of the accessor of a public variable.
const ACCESSOR_LEN: u32 = 15;

/// Reads the slot that `entry` names.
fn slot(pe: &PeImage<'_>, entry: Va) -> LinkSlot {
    let Some(region) = pe.region_at_va(entry) else {
        return LinkSlot::Other;
    };
    if let Some([0x33, 0xC0, 0xBA, d0, d1, d2, d3, 0x68, _, _, _, _, 0xC3]) =
        region.take(Off::new(0), STUB_TAIL_LEN)
    {
        return LinkSlot::Method(Va::new(u32::from_le_bytes([*d0, *d1, *d2, *d3])));
    }
    if let Some(
        [
            0x81,
            0x44,
            0x24,
            0x04,
            f0,
            f1,
            f2,
            f3,
            0xB9,
            _,
            _,
            _,
            _,
            0xFF,
            0xE1,
        ],
    ) = region.take(Off::new(0), ACCESSOR_LEN)
    {
        return LinkSlot::Variable {
            field: u32::from_le_bytes([*f0, *f1, *f2, *f3]),
        };
    }
    LinkSlot::Other
}

/// Reads the method link table of `object`.
///
/// Gives an empty table for an object with no `OptionalObjectInfo`, and for
/// a table whose count is 0 or whose `lpMethodLinkTable` is 0. The count is
/// bounded by the bytes that the section of the table holds, before it
/// bounds a loop.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when `OptionalObjectInfo` is in no section
/// or is cut before `lpMethodLinkTable`, and when a non-empty table is in no
/// section.
pub fn read_method_links(pe: &PeImage<'_>, object: &Object) -> Result<MethodLinks, Refusal> {
    if !has_optional_info(object.f_object_type) {
        return Ok(MethodLinks::default());
    }
    let at = object
        .lp_object_info
        .get()
        .checked_add(OPTIONAL_OBJECT_INFO_AT)
        .ok_or(Refusal::Damaged(
            "OptionalObjectInfo is past the end of memory",
        ))?;
    let optional = pe
        .region_at_va(Va::new(at))
        .ok_or(Refusal::Damaged("OptionalObjectInfo is in no section"))?;
    let cut = Refusal::Damaged("OptionalObjectInfo is cut before lpMethodLinkTable");
    let count = optional.u16_le(Off::new(LINK_COUNT_AT)).ok_or(cut)?;
    let base = optional.u16_le(Off::new(BASE_COUNT_AT)).ok_or(cut)?;
    let table_va = optional.va_le(Off::new(LINK_TABLE_AT)).ok_or(cut)?;
    let first_slot = IDISPATCH_METHODS
        .checked_add(u32::from(base))
        .ok_or(Refusal::Damaged("the base interface count overflows a u32"))?;
    if count == 0 || table_va.is_null() {
        return Ok(MethodLinks {
            first_slot,
            ..MethodLinks::default()
        });
    }
    let table = pe.region_at_va(table_va).ok_or(Refusal::Damaged(
        "the lpMethodLinkTable pointer is in no section",
    ))?;

    let mut defects = Vec::new();
    let max = table.len().checked_div(SLOT_SIZE).unwrap_or(0);
    let count = if u32::from(count) <= max {
        u32::from(count)
    } else {
        let offset = optional
            .file_offset(Off::new(LINK_COUNT_AT))
            .map_or(0, Off::get);
        defects.push(Defect {
            site: Site {
                offset,
                rva: optional.rva(Off::new(LINK_COUNT_AT)).map(Rva::get),
                structure: "OptionalObjectInfo",
                field: "wMethodLinkCount",
            },
            kind: DefectKind::ImplausibleCount {
                offset,
                count: u32::from(count),
                max,
            },
        });
        max
    };

    let mut slots = Vec::new();
    for index in 0..count {
        let at = index
            .checked_mul(SLOT_SIZE)
            .ok_or(Refusal::Damaged("a link table index overflows a u32"))?;
        let entry = table
            .va_le(Off::new(at))
            .ok_or(Refusal::Damaged("the file ends inside the link table"))?;
        slots.push(slot(pe, entry));
    }
    Ok(MethodLinks {
        first_slot,
        slots,
        defects,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{LinkSlot, MethodLinks, read_method_links};
    use crate::error::DefectKind;
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Va};
    use crate::vb::object::Object;
    use crate::vb::procdesc::MethodTable;

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

    fn put_u16(extra: &mut [u8], at: usize, value: u16) {
        extra[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(extra: &mut [u8], at: usize, value: u32) {
        extra[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// An object whose `ObjectInfo` is at `0x401000`, with the type bits of
    /// a class, so that it has an `OptionalObjectInfo`.
    fn object() -> Object {
        with_type(0x0001_8003)
    }

    /// The object of [`object`], with the type bits `f_object_type`.
    fn with_type(f_object_type: u32) -> Object {
        Object {
            file_offset: Off::new(0),
            rva: None,
            lp_object_info: Va::new(0x0040_1000),
            lpsz_object_name: Va::new(0),
            name: String::new(),
            proc_count: 0,
            lp_proc_names_array: Va::new(0),
            f_object_type,
        }
    }

    /// `OptionalObjectInfo` at `0x38` gives three slots and a base
    /// interface of 2 methods, and names the link table at `0x80`. The
    /// table names an accessor at `0xA0` and two stub tails at `0xB0` and
    /// `0xC0`, whose descriptors are `0x401200` and `0x401300`.
    fn three_slots() -> Vec<u8> {
        let mut extra = vec![0_u8; 0x100];
        put_u16(&mut extra, 0x38 + 0x28, 3);
        put_u16(&mut extra, 0x38 + 0x2A, 2);
        put_u32(&mut extra, 0x38 + 0x30, 0x0040_1080);
        put_u32(&mut extra, 0x80, 0x0040_10A0);
        put_u32(&mut extra, 0x84, 0x0040_10B0);
        put_u32(&mut extra, 0x88, 0x0040_10C0);
        extra[0xA0..0xAF].copy_from_slice(&[
            0x81, 0x44, 0x24, 0x04, 0x34, 0, 0, 0, 0xB9, 0, 0x11, 0x40, 0, 0xFF, 0xE1,
        ]);
        for (at, descriptor) in [(0xB0, 0x0040_1200_u32), (0xC0, 0x0040_1300)] {
            extra[at..at + 3].copy_from_slice(&[0x33, 0xC0, 0xBA]);
            put_u32(&mut extra, at + 3, descriptor);
            extra[at + 7] = 0x68;
            put_u32(&mut extra, at + 8, 0x0040_1100);
            extra[at + 12] = 0xC3;
        }
        extra
    }

    #[test]
    fn each_slot_is_a_method_stub_or_another_entry() {
        let bytes = synthetic_image(&three_slots());
        let pe = PeImage::parse(&bytes).unwrap();
        let links = read_method_links(&pe, &object()).unwrap();
        assert_eq!(
            links,
            MethodLinks {
                first_slot: 9,
                slots: vec![
                    LinkSlot::Variable { field: 0x34 },
                    LinkSlot::Method(Va::new(0x0040_1200)),
                    LinkSlot::Method(Va::new(0x0040_1300)),
                ],
                defects: Vec::new(),
            }
        );
    }

    #[test]
    fn a_vtable_offset_names_the_method_of_its_slot() {
        let bytes = synthetic_image(&three_slots());
        let pe = PeImage::parse(&bytes).unwrap();
        let links = read_method_links(&pe, &object()).unwrap();
        // Slot 9 is the first of the table: 9 * 4 = 0x24.
        assert_eq!(links.method_at(0x24), None, "an accessor");
        assert_eq!(links.method_at(0x28), Some(Va::new(0x0040_1200)));
        assert_eq!(links.method_at(0x2C), Some(Va::new(0x0040_1300)));
        assert_eq!(links.method_at(0x20), None, "before the table");
        assert_eq!(links.method_at(0x30), None, "after the table");
        assert_eq!(links.method_at(0x29), None, "not a multiple of 4");
        assert_eq!(links.method_at(u16::MAX), None);
    }

    #[test]
    fn the_accessors_of_a_variable_give_a_get_then_a_let() {
        let mut extra = three_slots();
        // The second slot is an accessor too, of the same field.
        put_u32(&mut extra, 0x84, 0x0040_10A0);
        let bytes = synthetic_image(&extra);
        let pe = PeImage::parse(&bytes).unwrap();
        let callees = read_method_links(&pe, &object())
            .unwrap()
            .callees(&MethodTable::default());
        assert_eq!(callees.variable(0x24), Some((0x34, true)));
        assert_eq!(callees.variable(0x28), Some((0x34, false)));
        assert_eq!(callees.variable(0x2C), None);
    }

    #[test]
    fn a_standard_module_has_no_link_table() {
        let bytes = synthetic_image(&three_slots());
        let pe = PeImage::parse(&bytes).unwrap();
        let module = with_type(0x0001_8001);
        assert_eq!(
            read_method_links(&pe, &module).unwrap(),
            MethodLinks::default()
        );
    }

    #[test]
    fn a_count_too_large_for_the_section_is_clamped_with_a_defect() {
        let mut extra = three_slots();
        put_u16(&mut extra, 0x38 + 0x28, 0xFFFF);
        let bytes = synthetic_image(&extra);
        let pe = PeImage::parse(&bytes).unwrap();
        let links = read_method_links(&pe, &object()).unwrap();
        // 0x80 bytes remain from 0x80 to the end of the section: 32 slots.
        assert_eq!(links.slots.len(), 32);
        assert_eq!(
            links.defects()[0].kind,
            DefectKind::ImplausibleCount {
                offset: 0x460,
                count: 0xFFFF,
                max: 32,
            }
        );
    }

    #[test]
    fn a_table_in_no_section_is_refused() {
        let mut extra = three_slots();
        put_u32(&mut extra, 0x38 + 0x30, 0x0090_0000);
        let bytes = synthetic_image(&extra);
        let pe = PeImage::parse(&bytes).unwrap();
        assert!(read_method_links(&pe, &object()).is_err());
    }
}
