//! `FuncTypDesc`, the type buffer walk, and the default-value walk.
//!
//! This is OBJ-04: a public procedure's argument names, argument types, and
//! its `ByRef`, `Array` and `Optional` modifiers. `PrivateObj.lp_func_type_info`
//! (`vb/privateobj.rs`, plan 02-03) is a pointer array, index-parallel to
//! `Object.lp_proc_names_array` and of the same length, `Object.proc_count`.
//! `STRUCTURES.md` section 6.2 states that a null entry in either array means
//! the procedure at that index is private.
//!
//! # Gap 6, closed on real bytes: the disputed header layout
//!
//! `STRUCTURES.md` section 6.3 records a dispute inside its own source: one
//! figure gives `memberID` at the unaligned offset `0x0A`, with no
//! `optionalVals` field at all; a later, revision-marked figure gives the
//! aligned layout this file reads. A script this planner ran over all 44
//! vendored programs read 193 real `FuncTypDesc` records and found
//! `constFFFF` equal to `0xFFFF` in 193 of 193, and the word at offset `0x06`
//! equal to `0` in 193 of 193. Both numbers are what chose the aligned
//! layout: the other layout would have read `constFFFF` from the low half of
//! `optionalVals`, and would not have produced `0xFFFF` there. [`Prototype`]
//! carries both raw fields (`const_ffff`, `nul1`) so `tests/type_descriptors.rs`
//! (plan 02-04, task 3) can re-assert this over the whole corpus, not just
//! over the three files this module's own tests reach.
//!
//! `constFFFF` is kept as a shipped, `Defect`-producing check, not a debug
//! assertion: a record whose `constFFFF` is not `0xFFFF` is a record whose
//! layout is not the one this file knows, and parsing it anyway would
//! produce a prototype that looks recovered and is wrong.
//!
//! The argument type decode, the modifier bits and the `optionalVals`
//! default-value grammar are plan 02-04's tasks 2 and 3, and land in this
//! file's later commits.
//!
//! # A worked example this plan's own text got wrong
//!
//! The plan text for this phase states that `Grayscale.exe`'s
//! `FastDrawing.GetImageWidth` gives `arg_size` `4`. Measured directly: its
//! `arg_size` is `0x08`. The source is
//! `Public Function GetImageWidth(ByRef srcPictureBox As PictureBox) As Long`,
//! which is a function (so `bFlags` bit 0 is set and the type buffer holds
//! two entries, the one argument plus the return type), and two entries is
//! `2 << 2 = 0x08`, not one entry's `0x04`. This file's own tests assert the
//! measured `0x08`, not the plan's inherited `4`; see the module's SUMMARY
//! for the fuller account.

use crate::error::{Defect, DefectKind, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Rva, Va};
use crate::vb::object::Object;
use crate::vb::privateobj::PrivateObj;

/// The size of the fixed `FuncTypDesc` header, before the variable-length
/// type buffer that follows it. `STRUCTURES.md` section 6.3.
const HEADER_SIZE: u32 = 0x20;

/// The width of one entry in `PrivateObj.lpFuncTypeInfo`.
const PTR_SIZE: u32 = 4;

/// One fully recovered public procedure prototype.
///
/// This task only fills the header-level facts: `member_id`, `v_off`,
/// `const_ffff`, `nul1` and `is_function`. Plan 02-04's task 2 extends this
/// with the decoded argument types and modifiers, and task 3 with the
/// `Optional` default values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prototype {
    /// The DISPID. `STRUCTURES.md` section 6.3 gives the form
    /// `0x6003xxxx` for a plain member; measured, a property reads
    /// `0x6803xxxx` instead, on the four property records this corpus
    /// holds.
    pub member_id: u32,
    /// The vtable offset. `0xFFFF` for an event; not exercised here, since
    /// this file reads `lpFuncTypeInfo`, never `lpEventsTypeInfo`.
    pub v_off: u16,
    /// The header signature word. Always `0xFFFF` by construction: a record
    /// whose `constFFFF` is not `0xFFFF` never reaches a [`Prototype`] at
    /// all. Carried so `tests/type_descriptors.rs` can re-assert the
    /// corpus-wide measurement through the public API, not only trust that
    /// this file's own gate enforced it.
    pub const_ffff: u16,
    /// The word at header offset `0x06`. Always `0`, for the same reason
    /// and the same purpose as `const_ffff`.
    pub nul1: u16,
    /// True when `bFlags` bit 0 is set: the last type buffer entry (task 2)
    /// is a return value, and every other entry is an argument. False when
    /// every entry is an argument.
    pub is_function: bool,
}

/// The outcome of resolving one non-null entry of `PrivateObj.lpFuncTypeInfo`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcedureSignature {
    /// The entry at this index is null. Paired with a private procedure at
    /// the same index in `Object.lpProcNamesArray`, per `STRUCTURES.md`
    /// section 6.2.
    NoDescriptor,
    /// The entry is non-null, and the record could not be read: an
    /// unmapped address, a truncated header, or a `constFFFF` mismatch.
    /// See [`FuncTypeWalk::defects`].
    Unrecoverable,
    /// The entry is non-null, and the record was read.
    Prototype(Prototype),
}

/// The procedure signatures one object carries, or the fact that it carries
/// none at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrototypeList {
    /// `object.proc_count` slots, one outcome per index, in array order.
    Slots(Vec<ProcedureSignature>),
    /// The object carries no `PrivateObj` at all: the standard-module cap,
    /// D-10. There is no `lpFuncTypeInfo` array to read. `proc_count` is
    /// carried through unread, matching `vb/privateobj.rs`'s own
    /// `ProcNames::NoNameArray`.
    NoPrivateObject {
        /// The number of procedures the object declares.
        proc_count: u32,
    },
    /// The object carries a `PrivateObj`, and its own `lpFuncTypeInfo`
    /// pointer is null or resolves to no section. Not observed anywhere in
    /// this corpus: every `PrivateObj::Present` object's `lp_func_type_info`
    /// is non-null and resolves. Kept distinct from `NoPrivateObject`
    /// because the two are different facts about the file, matching the
    /// same reasoning `vb/privateobj.rs`'s `ProcNames::NoNameArray` gives
    /// for its sibling array's own null-or-unmapped case.
    NoFuncTypeArray {
        /// The number of procedures the object declares.
        proc_count: u32,
    },
}

/// `PrivateObj.lpFuncTypeInfo`, walked in full, plus the defects the walk
/// found.
pub struct FuncTypeWalk {
    /// The recovered signatures, or the fact that there is no array to
    /// recover them from.
    pub signatures: PrototypeList,
    defects: Vec<Defect>,
}

impl FuncTypeWalk {
    /// Walks `private.lp_func_type_info`, bounded by `object.proc_count`,
    /// giving one [`ProcedureSignature`] per index.
    ///
    /// `object` and `private` come from the same object: `object.proc_count`
    /// is the length both `Object.lpProcNamesArray` and
    /// `PrivateObj.lpFuncTypeInfo` share, per `STRUCTURES.md` section 6.2,
    /// so this file bounds its own array by the one count plan 02-01 already
    /// reads and already bounds against the real size of the file, rather
    /// than re-deriving a second count that a later change could knock out
    /// of step with the name array's.
    #[must_use]
    pub fn read(pe: &PeImage<'_>, object: &Object, private: &PrivateObj) -> Self {
        let no_private_object = || Self {
            signatures: PrototypeList::NoPrivateObject {
                proc_count: object.proc_count,
            },
            defects: Vec::new(),
        };
        let no_func_type_array = || Self {
            signatures: PrototypeList::NoFuncTypeArray {
                proc_count: object.proc_count,
            },
            defects: Vec::new(),
        };

        let PrivateObj::Present {
            lp_func_type_info, ..
        } = private
        else {
            return no_private_object();
        };

        if lp_func_type_info.is_null() {
            return no_func_type_array();
        }
        let Some(array) = pe.region_at_va(*lp_func_type_info) else {
            return no_func_type_array();
        };
        let Some(window_size) = object.proc_count.checked_mul(PTR_SIZE) else {
            return no_func_type_array();
        };
        let Some(window) = array.subregion(Off::new(0), window_size) else {
            return no_func_type_array();
        };

        let mut signatures = Vec::with_capacity(usize::try_from(object.proc_count).unwrap_or(0));
        let mut defects = Vec::new();

        for index in 0..object.proc_count {
            let Some(entry_off) = index.checked_mul(PTR_SIZE) else {
                signatures.push(ProcedureSignature::NoDescriptor);
                continue;
            };
            let Some(va) = window.va_le(Off::new(entry_off)) else {
                signatures.push(ProcedureSignature::NoDescriptor);
                continue;
            };
            if va.is_null() {
                signatures.push(ProcedureSignature::NoDescriptor);
                continue;
            }

            let offset = window.file_offset(Off::new(entry_off)).map_or(0, Off::get);
            let site = Site {
                offset,
                rva: va.to_rva(pe.image_base()).map(Rva::get),
                structure: "PrivateObj",
                field: "lpFuncTypeInfo",
            };

            let Some(base) = pe.region_at_va(va) else {
                defects.push(Defect {
                    site,
                    kind: DefectKind::UnreadablePointer {
                        offset,
                        va: va.get(),
                    },
                });
                signatures.push(ProcedureSignature::Unrecoverable);
                continue;
            };

            let (prototype, mut record_defects) = read_one(pe, va, base);
            defects.append(&mut record_defects);
            match prototype {
                Some(prototype) => signatures.push(ProcedureSignature::Prototype(prototype)),
                None => signatures.push(ProcedureSignature::Unrecoverable),
            }
        }

        Self {
            signatures: PrototypeList::Slots(signatures),
            defects,
        }
    }

    /// Gives the defects the walk found: an unmapped `lpFuncTypeInfo` entry,
    /// or a `FuncTypDesc` whose `constFFFF` did not validate.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }
}

/// The raw header fields of one `FuncTypDesc`, before any interpretation.
struct RawHeader {
    #[allow(dead_code, reason = "read here; decoded by task 2, in a later commit")]
    arg_size: u8,
    b_flags: u8,
    v_off: u16,
    const_ffff: u16,
    nul1: u16,
    #[allow(dead_code, reason = "read here; decoded by task 3, in a later commit")]
    optional_vals: Va,
    member_id: u32,
    #[allow(dead_code, reason = "read here; decoded by task 2, in a later commit")]
    lp_ary_arg_names: Va,
}

/// Reads the fixed `0x20`-byte header. `header` is exactly [`HEADER_SIZE`]
/// bytes, taken before any field inside it is read, so every access here is
/// bounds-checked against real bytes already known to be present.
fn read_raw_header(header: &Region<'_>) -> Option<RawHeader> {
    Some(RawHeader {
        arg_size: header.u8(Off::new(0x00))?,
        b_flags: header.u8(Off::new(0x01))?,
        v_off: header.u16_le(Off::new(0x02))?,
        const_ffff: header.u16_le(Off::new(0x04))?,
        nul1: header.u16_le(Off::new(0x06))?,
        optional_vals: header.va_le(Off::new(0x08))?,
        member_id: header.u32_le(Off::new(0x0C))?,
        lp_ary_arg_names: header.va_le(Off::new(0x10))?,
    })
}

/// Reads one `FuncTypDesc` record, already resolved to `base`, the region
/// starting at its own address and running to the end of its section's
/// mapped bytes.
///
/// This task validates the header only: `constFFFF` must read `0xFFFF`, or
/// the record is reported unrecoverable rather than parsed. The type buffer
/// that follows the header (task 2) and the `optionalVals` default-value
/// walk (task 3) are not read yet; a later commit in this same file extends
/// this function.
fn read_one(
    pe: &PeImage<'_>,
    functype_va: Va,
    base: Region<'_>,
) -> (Option<Prototype>, Vec<Defect>) {
    let rva = functype_va.to_rva(pe.image_base()).map(Rva::get);
    let self_offset = base.file_offset(Off::new(0)).map_or(0, Off::get);
    let unrecoverable_here = |offset: u32, field: &'static str| {
        let site = Site {
            offset,
            rva,
            structure: "FuncTypDesc",
            field,
        };
        vec![Defect {
            site,
            kind: DefectKind::UnreadablePointer {
                offset,
                va: functype_va.get(),
            },
        }]
    };

    let Some(header) = base.subregion(Off::new(0), HEADER_SIZE) else {
        return (None, unrecoverable_here(self_offset, "argSize"));
    };

    // `header` is exactly `HEADER_SIZE` bytes, and every field
    // `read_raw_header` reads is inside that window. `Region` has no
    // infallible accessor, so this check stays; no test covers it and none
    // can.
    let Some(raw) = read_raw_header(&header) else {
        return (None, unrecoverable_here(self_offset, "argSize"));
    };

    if raw.const_ffff != 0xFFFF {
        let offset = header
            .file_offset(Off::new(0x04))
            .map_or(self_offset, Off::get);
        return (None, unrecoverable_here(offset, "constFFFF"));
    }

    let prototype = Prototype {
        member_id: raw.member_id,
        v_off: raw.v_off,
        const_ffff: raw.const_ffff,
        nul1: raw.nul1,
        is_function: raw.b_flags & 1 != 0,
    };

    (Some(prototype), Vec::new())
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
    use super::{FuncTypeWalk, ProcedureSignature, Prototype, PrototypeList, read_raw_header};
    use crate::error::Severity;
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Va};
    use crate::vb::header::{VbHeader, header_region};
    use crate::vb::object::{Object, ObjectTable};
    use crate::vb::privateobj::{ObjectInfo, PrivateObj, ProcNames, Procedure, ProcedureList};
    use crate::vb::project::{ObjectTableHead, ProjectInfo};

    /// `Grayscale.exe`, the tracer for task 1: three objects, and
    /// `FastDrawing`'s eight slots, four private (`Declare`) and four
    /// public.
    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    /// Gives the address of `ProjectInfo` that the file itself holds.
    fn project_data_va(data: &[u8]) -> Va {
        let image = PeImage::parse(data).unwrap();
        let hdr = header_region(&image).unwrap();
        VbHeader::read(&hdr).unwrap().lp_project_data
    }

    /// Gives the address of the object table that the file itself holds.
    fn object_table_va(data: &[u8]) -> Va {
        let image = PeImage::parse(data).unwrap();
        ProjectInfo::read(&image, project_data_va(data))
            .unwrap()
            .lp_object_table
    }

    /// Walks the object array out of a byte slice, through the same pointer
    /// chain `inspect` uses. Duplicated per-module on purpose: this module's
    /// tests must be able to fail independently of `vb/object.rs`'s and
    /// `vb/privateobj.rs`'s own.
    fn objects(data: &[u8]) -> Vec<Object> {
        let image = PeImage::parse(data).unwrap();
        let lp_object_table = object_table_va(data);
        let head = ObjectTableHead::read(&image, lp_object_table).unwrap();
        ObjectTable::walk(&image, lp_object_table, &head)
            .unwrap()
            .objects
    }

    /// Finds the one object of `data` named `name`, or panics: every corpus
    /// fixture this module reads is asserted to hold the object it is used
    /// for, so a missing name is this test's own fixture being wrong.
    fn find_object(data: &[u8], name: &str) -> Object {
        objects(data)
            .into_iter()
            .find(|o| o.name == name)
            .unwrap_or_else(|| panic!("{name} not found"))
    }

    /// Reads `PrivateObj` for one object.
    fn private_obj_of(pe: &PeImage<'_>, object: &Object) -> PrivateObj {
        let info = ObjectInfo::read(pe, object.lp_object_info).unwrap();
        PrivateObj::read(pe, info.lp_private_object).unwrap()
    }

    /// Finds the recovered [`Prototype`] named `proc_name` on the object
    /// named `object_name`, through the same index-parallel walk
    /// `FuncTypeWalk::read` and `vb/privateobj.rs`'s own `ProcedureList`
    /// perform, cross-checking the two arrays agree at that index.
    fn find_prototype(data: &[u8], object_name: &str, proc_name: &str) -> Prototype {
        let image = PeImage::parse(data).unwrap();
        let object = find_object(data, object_name);
        let private = private_obj_of(&image, &object);
        let names = ProcedureList::read(&image, &object);
        let ProcNames::Slots(slots) = &names.procs else {
            panic!("{object_name} carries no name array");
        };
        let index = slots
            .iter()
            .position(|p| matches!(p, Procedure::Public(n) if n == proc_name))
            .unwrap_or_else(|| panic!("{proc_name} not found on {object_name}"));

        let walk = FuncTypeWalk::read(&image, &object, &private);
        let PrototypeList::Slots(signatures) = &walk.signatures else {
            panic!("{object_name} carries no FuncTypDesc array");
        };
        match &signatures[index] {
            ProcedureSignature::Prototype(p) => p.clone(),
            other => panic!("{proc_name} on {object_name} did not resolve: {other:?}"),
        }
    }

    /// Reads the raw header bytes of the `FuncTypDesc` at index `index` of
    /// `object`, through the parser's own pointer chain. Used for the
    /// fields task 1 reads but task 2 has not decoded yet
    /// (`lp_ary_arg_names`, `arg_size`).
    fn raw_header_at(data: &[u8], object_name: &str, index: u32) -> super::RawHeader {
        let image = PeImage::parse(data).unwrap();
        let object = find_object(data, object_name);
        let private = private_obj_of(&image, &object);
        let PrivateObj::Present {
            lp_func_type_info, ..
        } = private
        else {
            panic!("{object_name} is a module, not an object with private data");
        };
        let array_region = image.region_at_va(lp_func_type_info).unwrap();
        let entry_va = array_region.va_le(Off::new(index * 4)).unwrap();
        let header_region = image.region_at_va(entry_va).unwrap();
        let window = header_region
            .subregion(Off::new(0), super::HEADER_SIZE)
            .unwrap();
        read_raw_header(&window).unwrap()
    }

    // -- Task 1: the header, and the two disputed offsets --------------

    /// The type descriptor array for `FastDrawing` holds a null pointer at
    /// every index whose name slot is `Private`, and a real pointer at
    /// every index whose name slot is `Public`. Behaviour 1.
    #[test]
    fn grayscale_fast_drawing_type_array_agrees_with_the_name_array_at_every_index() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let object = find_object(GRAYSCALE, "FastDrawing");
        let private = private_obj_of(&image, &object);

        let names = ProcedureList::read(&image, &object);
        let ProcNames::Slots(name_slots) = &names.procs else {
            panic!("FastDrawing carries no name array");
        };

        let walk = FuncTypeWalk::read(&image, &object, &private);
        let PrototypeList::Slots(type_slots) = &walk.signatures else {
            panic!("FastDrawing carries no FuncTypDesc array");
        };

        assert_eq!(name_slots.len(), type_slots.len());
        for (name_slot, type_slot) in name_slots.iter().zip(type_slots.iter()) {
            match name_slot {
                Procedure::Private => {
                    assert_eq!(*type_slot, ProcedureSignature::NoDescriptor);
                }
                Procedure::Public(_) => {
                    assert!(matches!(type_slot, ProcedureSignature::Prototype(_)));
                }
            }
        }
    }

    /// `GetImageWidth`'s descriptor gives the measured `arg_size` (`0x08`,
    /// not the plan's inherited `4`: see the module doc comment), `b_flags`
    /// bit 0 set, `const_ffff` `0xFFFF`, `nul1` `0`, and a non-null
    /// `lp_ary_arg_names`. `GetImageWidth` is index 4 of `FastDrawing`'s
    /// eight slots (see `vb/privateobj.rs`'s own test of the same object).
    /// Behaviour 2, corrected.
    #[test]
    fn grayscale_get_image_width_header_fields_match_the_measured_bytes() {
        let raw = raw_header_at(GRAYSCALE, "FastDrawing", 4);
        assert_eq!(raw.arg_size, 0x08);
        assert_eq!(raw.b_flags & 1, 1);
        assert_eq!(raw.const_ffff, 0xFFFF);
        assert_eq!(raw.nul1, 0);
        assert!(!raw.lp_ary_arg_names.is_null());

        let prototype = find_prototype(GRAYSCALE, "FastDrawing", "GetImageWidth");
        assert_eq!(prototype.const_ffff, 0xFFFF);
        assert_eq!(prototype.nul1, 0);
        assert!(prototype.is_function);
    }

    /// `member_id`'s top sixteen bits are `0x6003` on `GetImageWidth`.
    /// Behaviour 3.
    #[test]
    fn grayscale_get_image_width_member_id_top_bits_are_0x6003() {
        let prototype = find_prototype(GRAYSCALE, "FastDrawing", "GetImageWidth");
        assert_eq!((prototype.member_id >> 16) & 0xFFFF, 0x6003);
    }

    /// A record whose `const_ffff` is not `0xFFFF` produces a recoverable
    /// defect naming the offset, and the record is reported unrecoverable
    /// rather than parsed. Behaviour 4.
    #[test]
    fn a_record_whose_const_ffff_is_wrong_is_unrecoverable_with_a_recoverable_defect() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let object = find_object(GRAYSCALE, "FastDrawing");
        let private = private_obj_of(&image, &object);
        let PrivateObj::Present {
            lp_func_type_info, ..
        } = private
        else {
            panic!("FastDrawing is a class, not a module");
        };

        let entry_region = image.region_at_va(lp_func_type_info).unwrap();
        let entry_va = entry_region.va_le(Off::new(4 * 4)).unwrap();
        let header_region = image.region_at_va(entry_va).unwrap();
        let at = usize::try_from(header_region.file_offset(Off::new(0x04)).unwrap().get()).unwrap();

        let mut bytes = GRAYSCALE.to_vec();
        assert_eq!(&bytes[at..at + 2], &0xFFFFu16.to_le_bytes());
        bytes[at..at + 2].copy_from_slice(&0x1234u16.to_le_bytes());

        let patched = PeImage::parse(&bytes).unwrap();
        let object = find_object(&bytes, "FastDrawing");
        let private = private_obj_of(&patched, &object);
        let walk = FuncTypeWalk::read(&patched, &object, &private);
        let PrototypeList::Slots(slots) = &walk.signatures else {
            panic!("FastDrawing carries no FuncTypDesc array");
        };
        assert_eq!(slots[4], ProcedureSignature::Unrecoverable);

        assert_eq!(walk.defects().len(), 1);
        let defect = &walk.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert_eq!(defect.site.offset, u32::try_from(at).unwrap());
    }

    /// With the descriptor pointer patched to an address in no section, the
    /// procedure keeps its name and reports no prototype, and a recoverable
    /// defect names the address. Behaviour 5.
    #[test]
    fn a_descriptor_pointer_in_no_section_keeps_the_name_and_reports_no_prototype() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let object = find_object(GRAYSCALE, "FastDrawing");
        let private = private_obj_of(&image, &object);
        let PrivateObj::Present {
            lp_func_type_info, ..
        } = private
        else {
            panic!("FastDrawing is a class, not a module");
        };

        let nowhere = image.image_base() + 0x00F0_0000;
        assert!(image.region_at_va(Va::new(nowhere)).is_none());

        let array_region = image.region_at_va(lp_func_type_info).unwrap();
        let at = usize::try_from(array_region.file_offset(Off::new(4 * 4)).unwrap().get()).unwrap();
        let mut bytes = GRAYSCALE.to_vec();
        bytes[at..at + 4].copy_from_slice(&nowhere.to_le_bytes());

        let patched_image = PeImage::parse(&bytes).unwrap();
        let object = find_object(&bytes, "FastDrawing");

        // The procedure name is still recovered, through the unrelated
        // `lpProcNamesArray`, which this patch does not touch.
        let names = ProcedureList::read(&patched_image, &object);
        let ProcNames::Slots(name_slots) = &names.procs else {
            panic!("FastDrawing carries no name array");
        };
        assert_eq!(name_slots[4], Procedure::Public("GetImageWidth".to_owned()));

        let private = private_obj_of(&patched_image, &object);
        let walk = FuncTypeWalk::read(&patched_image, &object, &private);
        let PrototypeList::Slots(type_slots) = &walk.signatures else {
            panic!("FastDrawing carries no FuncTypDesc array");
        };
        assert_eq!(type_slots[4], ProcedureSignature::Unrecoverable);

        assert_eq!(walk.defects().len(), 1);
        let defect = &walk.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert!(matches!(
            defect.kind,
            crate::error::DefectKind::UnreadablePointer { va, .. } if va == nowhere
        ));
    }
}
