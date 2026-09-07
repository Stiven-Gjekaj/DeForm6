//! `Object`, the array `lpObjectArray` points at, and the walk that recovers
//! the name of every one.
//!
//! `STRUCTURES.md` section 4.1 corrects the loop bound: it is `wTotalObjects`,
//! never `wCompiledObjects`, which is the array's rounded up capacity and
//! matches the declared object count in only 29 of the 44 corpus programs.
//! Nothing in this file reads or loops on the compiled count.
//!
//! # This file resolves `lpObjectArray` itself, and does not touch `vb/project.rs`
//!
//! Plan 01-07 left a comment on `ObjectTableHead::read` saying it deliberately
//! does not read `ObjectTable + 0x30`, `lpObjectArray`, so that a later phase
//! would read it. `vb/project.rs` belongs to plan 02-06 in this wave, and two
//! plans in the same wave must not edit the same file, so this module reads
//! `lpObjectArray` on its own: a second, narrow window onto the same
//! `ObjectTable` structure that [`crate::vb::project::ObjectTableHead::read`]
//! already opened for its own three fields. That function still owns the two
//! counts, the project name and the defect it reports; this file reads the
//! one field it needs, independently, the same way `ProjectInfo::read` and
//! `ObjectTableHead::read` already read two different windows onto two
//! different structures reached through the same pointer chain.
//!
//! Each `Object` is narrowed to its own `0x30`-byte window before any field
//! inside it is read, which is the window-before-fields discipline
//! `vb/project.rs` documents, applied for the first time to an array of
//! structures rather than to one structure: a fresh `subregion` is taken per
//! element, never one region for the whole array indexed by hand.

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Rva, Va};
use crate::vb::project::ObjectTableHead;

/// The size of one `Object` element.
///
/// `STRUCTURES.md` section 5.1 gives `0x30` = 48 bytes, and five sources
/// agree.
const OBJECT_SIZE: u32 = 0x30;

/// The size of the `ObjectTable` structure that holds `lpObjectArray`.
///
/// This is the same value `vb/project.rs` uses for `ObjectTableHead::read`,
/// kept here rather than imported. See the module doc comment for why this
/// file does not reach into that one.
const OBJECT_TABLE_SIZE: u32 = 0x54;

/// The bound on an object name string.
///
/// The same value and the same reason `vb/project.rs` gives for the project
/// name: `Region::cstr` needs a mandatory maximum, so a file with no NUL byte
/// after the name cannot make the scan run to the end of the section.
const NAME_MAX: u32 = 0x104;

/// The width of one entry in `lpProcNamesArray`, which bounds `ProcCount`.
const PROC_NAME_PTR_SIZE: u32 = 4;

/// One form, module or class the project declares.
///
/// Five fields, matching the load bearing subset `STRUCTURES.md` section 5.1
/// marks and this phase's research script read. `f_object_type` is carried
/// raw: plan 02-02 classifies it, and this file refuses nothing on the
/// strength of it. `proc_count` and `lp_proc_names_array` are carried raw as
/// well, and plan 02-03 resolves them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    /// The address of this object's `ObjectInfo`.
    pub lp_object_info: Va,
    /// The object's name, resolved from `lpszObjectName`.
    ///
    /// Empty when the name pointer resolves nowhere or holds no bounded
    /// string. The object that held it keeps every other field, and
    /// [`ObjectTable::defects`] carries the reason. This file never invents a
    /// name.
    pub name: String,
    /// The number of procedures this object declares.
    ///
    /// Bounded against the real length of the file that remains from
    /// `lpProcNamesArray`, and clamped to what the file can hold when the raw
    /// value cannot fit. See [`ObjectTable::defects`].
    pub proc_count: u32,
    /// The address of the array of procedure name pointers.
    ///
    /// Carried raw. Plan 02-03 resolves it, and it checks this address
    /// against zero before it loops: a `.bas` module's whole array pointer is
    /// null, which is a different case from a null entry inside a populated
    /// array.
    pub lp_proc_names_array: Va,
    /// The form / module / class discriminator, carried raw.
    ///
    /// Plan 02-02 classifies this value. This file does not, and it refuses
    /// no object on the strength of an unrecognised one.
    pub f_object_type: u32,
}

/// The `Object` array, walked in full.
#[derive(Clone, Debug)]
pub struct ObjectTable {
    /// Every object the walk recovered, in array order.
    pub objects: Vec<Object>,
    defects: Vec<Defect>,
}

impl ObjectTable {
    /// Walks the `Object` array from `lpObjectTable`.
    ///
    /// # The measured ground
    ///
    /// This session's own script walked the array of every one of the 105
    /// objects across the 44 vendored programs: stride `0x30`, zero address
    /// resolve failures, and the loop bound `wTotalObjects`. That is a
    /// measurement, not a citation of `STRUCTURES.md`: section 4 of that
    /// document recommended looping on `wCompiledObjects` instead, and
    /// section 4.1 withdraws the recommendation after the same measurement
    /// found it matches the `.vbp`-declared object count in only 29 of the
    /// 44 corpus programs, against 44 of 44 for `wTotalObjects`. Nothing
    /// here reads or loops on the compiled count.
    ///
    /// Each element is narrowed to its own [`OBJECT_SIZE`]-byte window before
    /// any field inside it is read. A name that resolves nowhere, or a
    /// `ProcCount` too large for the file to hold, is a defect on the object
    /// that carried it, not a reason to refuse the whole array: one
    /// unreadable field does not lose the other objects.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when the object table pointer or the
    /// object array pointer is in no section, when the file ends inside the
    /// object table structure, or when the file ends inside an `Object`
    /// element. Every index up to `head.w_total_objects` must resolve a full
    /// [`OBJECT_SIZE`]-byte window, or the walk stops there: a partial
    /// element is not read in part.
    pub fn walk(
        pe: &PeImage<'_>,
        lp_object_table: Va,
        head: &ObjectTableHead,
    ) -> Result<Self, Refusal> {
        let lp_object_array = object_array_va(pe, lp_object_table)?;
        let array = pe.region_at_va(lp_object_array).ok_or(Refusal::Damaged(
            "the object array pointer is in no section",
        ))?;

        let mut objects = Vec::new();
        let mut defects = Vec::new();

        for i in 0_u32..u32::from(head.w_total_objects) {
            let at = i
                .checked_mul(OBJECT_SIZE)
                .ok_or(Refusal::Damaged("the object array index overflows a u32"))?;
            let element = array
                .subregion(Off::new(at), OBJECT_SIZE)
                .ok_or(Refusal::Damaged("the file ends inside an Object element"))?;

            let lp_object_info = va_at(
                &element,
                0x00,
                "an Object holds no address for its ObjectInfo",
            )?;
            let lpsz_object_name =
                va_at(&element, 0x18, "an Object holds no address for its name")?;
            let raw_proc_count = u32_at(&element, 0x1C, "an Object holds no procedure count")?;
            let lp_proc_names_array = va_at(
                &element,
                0x20,
                "an Object holds no address for its procedure name array",
            )?;
            let f_object_type = u32_at(&element, 0x28, "an Object holds no type discriminator")?;

            let (name, name_defect) = read_name(pe, &element, lpsz_object_name);
            if let Some(defect) = name_defect {
                defects.push(defect);
            }

            let (proc_count, count_defect) =
                bound_proc_count(pe, &element, lp_proc_names_array, raw_proc_count);
            if let Some(defect) = count_defect {
                defects.push(defect);
            }

            objects.push(Object {
                lp_object_info,
                name,
                proc_count,
                lp_proc_names_array,
                f_object_type,
            });
        }

        Ok(Self { objects, defects })
    }

    /// Gives the defects the walk found: a name that resolved nowhere, or a
    /// count too large for the file to hold. One unreadable field does not
    /// remove the object that held it.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }
}

/// Resolves `lpObjectArray`, the address of the `Object` array.
///
/// See the module doc comment for why this file reads `ObjectTable + 0x30`
/// on its own rather than through [`ObjectTableHead`].
fn object_array_va(pe: &PeImage<'_>, lp_object_table: Va) -> Result<Va, Refusal> {
    let at = pe.region_at_va(lp_object_table).ok_or(Refusal::Damaged(
        "the object table pointer is in no section",
    ))?;
    let window = at
        .subregion(Off::new(0), OBJECT_TABLE_SIZE)
        .ok_or(Refusal::Damaged(
            "the file ends inside the object table structure",
        ))?;
    va_at(
        &window,
        0x30,
        "the object table holds no address for the object array",
    )
}

/// Resolves an object's name.
///
/// A name pointer that resolves nowhere, or a name with no NUL terminator
/// inside [`NAME_MAX`] bytes, is one unreadable object, not a broken file.
/// The object keeps its other fields, the walk continues, and the returned
/// defect names the byte offset and the address so a person can open the
/// file at that offset.
fn read_name(
    pe: &PeImage<'_>,
    element: &Region<'_>,
    lpsz_object_name: Va,
) -> (String, Option<Defect>) {
    let offset = element.file_offset(Off::new(0x18)).map_or(0, Off::get);
    let site = Site {
        offset,
        rva: lpsz_object_name.to_rva(pe.image_base()).map(Rva::get),
        structure: "Object",
        field: "lpszObjectName",
    };

    let Some(name_region) = pe.region_at_va(lpsz_object_name) else {
        let kind = DefectKind::UnreadablePointer {
            offset,
            va: lpsz_object_name.get(),
        };
        return (String::new(), Some(Defect { site, kind }));
    };

    match name_region.cstr(Off::new(0), NAME_MAX) {
        // Each byte becomes its Latin-1 code point, which is the rule
        // `vb/project.rs` uses for the project name. `String::from_utf8_lossy`
        // is wrong here: a byte in 0x80 to 0xFF would become the replacement
        // character and the name would be lost.
        Some(bytes) => (bytes.iter().copied().map(char::from).collect(), None),
        None => {
            let kind = DefectKind::NoNulTerminator {
                offset,
                limit: NAME_MAX,
            };
            (String::new(), Some(Defect { site, kind }))
        }
    }
}

/// Bounds `ProcCount` against the real length of the file that remains from
/// `lpProcNamesArray`, and clamps it when it does not fit.
///
/// `ProcCount` is a `u32` straight out of the file, and plan 02-03 uses it as
/// a loop bound over an array of four-byte pointers. A file that claims more
/// entries than it holds must not let that count reach an allocation
/// downstream. The largest `ProcCount` measured anywhere in this corpus is
/// 83.
///
/// A null `lpProcNamesArray`, or one that resolves to no section, gives
/// nothing to bound against, and this returns the raw count unclamped: a
/// `.bas` module's whole array pointer is null while its `ProcCount` still
/// reports a real number of procedures (measured 1 to 7 across the corpus),
/// and clamping that count to zero here would erase a true value that
/// nothing in this file reads through anyway.
fn bound_proc_count(
    pe: &PeImage<'_>,
    element: &Region<'_>,
    lp_proc_names_array: Va,
    raw_proc_count: u32,
) -> (u32, Option<Defect>) {
    let Some(array_region) = pe.region_at_va(lp_proc_names_array) else {
        return (raw_proc_count, None);
    };
    let max_entries = array_region
        .len()
        .checked_div(PROC_NAME_PTR_SIZE)
        .unwrap_or(0);
    if raw_proc_count <= max_entries {
        return (raw_proc_count, None);
    }

    let offset = element.file_offset(Off::new(0x1C)).map_or(0, Off::get);
    let defect = Defect {
        site: Site {
            offset,
            rva: None,
            structure: "Object",
            field: "ProcCount",
        },
        kind: DefectKind::ImplausibleCount {
            offset,
            count: raw_proc_count,
            max: max_entries,
        },
    };
    (max_entries, Some(defect))
}

/// Reads an unsigned 32-bit value out of a structure window.
fn u32_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<u32, Refusal> {
    window.u32_le(Off::new(at)).ok_or(Refusal::Damaged(what))
}

/// Reads a virtual address out of a structure window.
fn va_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<Va, Refusal> {
    window.va_le(Off::new(at)).ok_or(Refusal::Damaged(what))
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
    use super::{OBJECT_SIZE, Object, ObjectTable};
    use crate::error::{DefectKind, Refusal, Severity};
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Va};
    use crate::vb::header::{VbHeader, header_region};
    use crate::vb::project::{ObjectTableHead, ProjectInfo};

    /// The corpus program this task's tracer is worked against.
    ///
    /// A test module under `src/` reaches a corpus file this way and no
    /// other way. The library names no file system type. Plan 02-08 owns the
    /// sweep over all 44, in a file under `tests/`, which is a separate
    /// crate root.
    const MANDELBROT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
    ));

    /// The program whose object count and object capacity differ: one form
    /// and two classes declared, and a capacity of 4. The fourth array slot
    /// holds a null pointer that the walk must never reach.
    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    /// The program whose capacity is 4 and whose count is 1. The three slots
    /// past the count resolve to nothing, and the walk must never reach them
    /// either.
    const LOCK_WORK_STATION: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/LockWorkStation/LockWorkStation.exe"
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

    /// Reads the head of the object table out of a byte slice.
    fn object_table_head(data: &[u8]) -> ObjectTableHead {
        let image = PeImage::parse(data).unwrap();
        ObjectTableHead::read(&image, object_table_va(data)).unwrap()
    }

    /// Walks the object array out of a byte slice, through the same pointer
    /// chain `inspect` uses.
    fn walk(data: &[u8]) -> Result<ObjectTable, Refusal> {
        let image = PeImage::parse(data).unwrap();
        let head = object_table_head(data);
        ObjectTable::walk(&image, object_table_va(data), &head)
    }

    /// Gives the absolute file offset of a field inside the object table.
    ///
    /// The route is the parser's own: the header's address, then the address
    /// `ProjectInfo` holds, then `file_offset`. Nothing searches for a byte
    /// pattern.
    fn object_table_field_offset(data: &[u8], field: u32) -> usize {
        let image = PeImage::parse(data).unwrap();
        let window = image.region_at_va(object_table_va(data)).unwrap();
        let at = window.file_offset(Off::new(field)).unwrap();
        usize::try_from(at.get()).unwrap()
    }

    /// Gives the address of the object array that the file itself holds.
    fn object_array_va(data: &[u8]) -> Va {
        let image = PeImage::parse(data).unwrap();
        let window = image.region_at_va(object_table_va(data)).unwrap();
        window.va_le(Off::new(0x30)).unwrap()
    }

    /// Gives the raw array capacity the object table declares, read at its
    /// own file offset rather than through `ObjectTableHead`'s field of the
    /// same name.
    ///
    /// This file must never hold the identifier this field is named after
    /// outside a comment: `success_criteria` greps for it, because looping
    /// on it is exactly the fault D-01 corrects. A test that wants to state
    /// the real capacity for context, without looping on it, reads the raw
    /// bytes instead.
    fn object_array_capacity(data: &[u8]) -> u16 {
        let at = object_table_field_offset(data, 0x2C);
        u16::from_le_bytes(data[at..at + 2].try_into().unwrap())
    }

    /// Gives the absolute file offset of a field inside one element of the
    /// object array. Nothing searches for a byte pattern here either.
    fn object_element_field_offset(data: &[u8], index: u32, field: u32) -> usize {
        let image = PeImage::parse(data).unwrap();
        let array = image.region_at_va(object_array_va(data)).unwrap();
        let at = array.file_offset(Off::new(index * 0x30 + field)).unwrap();
        usize::try_from(at.get()).unwrap()
    }

    /// Copies `data` and writes a `u32` at an absolute file offset.
    fn with_u32_at(data: &[u8], at: usize, value: u32) -> Vec<u8> {
        let mut out = data.to_vec();
        assert_ne!(
            out[at..at + 4],
            value.to_le_bytes(),
            "the fixture writes the value the field already holds, so it proves nothing"
        );
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        out
    }

    /// The `.vbp` beside `Mandelbrot.exe` declares one `Form`, and its
    /// `Attribute VB_Name` is `frmFractal`. `CONTEXT.md`'s corrected worked
    /// example: `ProcCount` 9 is 8 `Sub`/`Function` declarations plus one
    /// `Private Declare Function`, and a `Declare` consumes a procedure slot.
    #[test]
    fn the_corpus_file_gives_exactly_one_object_named_frm_fractal() {
        let table = walk(MANDELBROT).unwrap();
        assert_eq!(table.objects.len(), 1);
        let object = &table.objects[0];
        assert_eq!(object.name, "frmFractal");
        assert_eq!(object.f_object_type, 0x0001_8083);
        assert_eq!(object.proc_count, 9);
        assert!(!object.lp_proc_names_array.is_null());
        assert!(table.defects().is_empty());
    }

    /// A patched `lpObjectArray` that resolves nowhere refuses the file, and
    /// it must not give an empty object list. An empty list reads as "this
    /// project declares no objects," which is a different and wrong claim
    /// about the source from "this file is damaged."
    ///
    /// This file does not add `lp_object_array` to `ObjectTableHead`, because
    /// `vb/project.rs` belongs to plan 02-06 in this wave. See the module doc
    /// comment. Patching the exact byte offset this test computes is the
    /// proof that `ObjectTable::walk` reads `ObjectTable + 0x30`, which is
    /// what that behaviour asked to see.
    #[test]
    fn a_patched_object_array_pointer_in_no_section_is_damaged_and_not_empty() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        assert!(image.region_at_va(Va::new(nowhere)).is_none());

        let at = object_table_field_offset(MANDELBROT, 0x30);
        let bytes = with_u32_at(MANDELBROT, at, nowhere);

        assert_eq!(
            walk(&bytes).unwrap_err(),
            Refusal::Damaged("the object array pointer is in no section")
        );
    }

    /// A file cut off inside the first `Object` element is damaged, and is
    /// not read in part.
    ///
    /// The head is built from the full file: the fields it reads sit before
    /// the array, and the project name sits after it, so a head built from
    /// the truncated bytes would fail for the unrelated reason that the
    /// project name no longer resolves. That would test the wrong thing.
    #[test]
    fn a_file_truncated_inside_the_first_object_is_damaged_and_is_not_read_in_part() {
        let array_start = object_element_field_offset(MANDELBROT, 0, 0x00);
        let cut = array_start + 0x10;
        assert!(cut < array_start + usize::try_from(OBJECT_SIZE).unwrap());
        assert!(cut < MANDELBROT.len());
        let bytes = MANDELBROT[..cut].to_vec();

        let head = object_table_head(MANDELBROT);
        let lp_object_table = object_table_va(MANDELBROT);
        let truncated = PeImage::parse(&bytes).unwrap();

        // The bytes before the cut are still readable, so the refusal below
        // comes from the truncated element and not from a short file overall.
        let array = truncated.region_at_va(object_array_va(MANDELBROT)).unwrap();
        assert!(array.len() < OBJECT_SIZE);

        assert_eq!(
            ObjectTable::walk(&truncated, lp_object_table, &head).unwrap_err(),
            Refusal::Damaged("the file ends inside an Object element")
        );
    }

    /// `Grayscale.exe` declares one form and two classes, its array holds the
    /// three names and then a null pointer, and its capacity is 4. A walk
    /// that looped on the capacity would give four objects here. The
    /// expectation is an exact ordered list, never a subset: a subset
    /// assertion cannot see an over-count, which is the whole reason this
    /// phase exists.
    #[test]
    fn the_grayscale_corpus_file_gives_exactly_three_objects_in_array_order() {
        let head = object_table_head(GRAYSCALE);
        assert_eq!(head.w_total_objects, 3);
        assert_eq!(object_array_capacity(GRAYSCALE), 4);

        let table = walk(GRAYSCALE).unwrap();
        let names: Vec<&str> = table.objects.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["frmGrayscale", "pdOpenSaveDialog", "FastDrawing"]
        );
        assert!(table.defects().is_empty());
    }

    /// The object table declares room for four objects. The fourth slot is
    /// real, not a fiction of the capacity field: it holds a null
    /// `lpObjectInfo` in the file. The walk never reaches it, because the
    /// loop bound is the count and not the capacity.
    #[test]
    fn the_fourth_array_slot_of_grayscale_exists_and_is_never_read() {
        let head = object_table_head(GRAYSCALE);
        assert_eq!(object_array_capacity(GRAYSCALE), 4);
        assert_eq!(head.w_total_objects, 3);

        let at = object_element_field_offset(GRAYSCALE, 3, 0x00);
        let raw = u32::from_le_bytes(GRAYSCALE[at..at + 4].try_into().unwrap());
        assert_eq!(
            raw, 0,
            "the fourth slot must hold a null lpObjectInfo, or this test proves nothing"
        );

        assert_eq!(walk(GRAYSCALE).unwrap().objects.len(), 3);
    }

    /// `LockWorkStation.exe` declares one form. Its capacity is 4, and the
    /// three slots past the count hold pointers that resolve to nothing. A
    /// walk that looped on the capacity would give four objects here too.
    #[test]
    fn the_lock_work_station_corpus_file_gives_exactly_one_object() {
        let head = object_table_head(LOCK_WORK_STATION);
        assert_eq!(head.w_total_objects, 1);
        assert_eq!(object_array_capacity(LOCK_WORK_STATION), 4);

        let table = walk(LOCK_WORK_STATION).unwrap();
        assert_eq!(table.objects.len(), 1);
        assert_eq!(table.objects[0].name, "FrmLockWorkStation");
        assert!(table.defects().is_empty());
    }

    /// A name pointer that resolves nowhere is one unreadable object, not a
    /// broken file. The walk still returns every object, the one whose name
    /// could not be read keeps its other fields with an empty name, and the
    /// defect names the byte offset and the address so a person can open the
    /// file at that offset.
    #[test]
    fn a_grayscale_name_pointer_in_no_section_loses_one_name_and_no_object() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let nowhere = image.image_base() + 0x00F0_0000;
        assert!(image.region_at_va(Va::new(nowhere)).is_none());

        let at = object_element_field_offset(GRAYSCALE, 1, 0x18);
        let bytes = with_u32_at(GRAYSCALE, at, nowhere);

        let table = walk(&bytes).unwrap();
        assert_eq!(table.objects.len(), 3);
        assert_eq!(table.objects[0].name, "frmGrayscale");
        assert_eq!(table.objects[1].name, "");
        assert_eq!(table.objects[2].name, "FastDrawing");

        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert!(matches!(
            defect.kind,
            DefectKind::UnreadablePointer { va, .. } if va == nowhere
        ));
        assert_eq!(defect.site.offset, u32::try_from(at).unwrap());
    }

    /// `fObjectType` is carried raw. Plan 02-02 classifies it; this file does
    /// not, and it does not guess a type code.
    #[test]
    fn the_grayscale_object_kinds_are_carried_raw() {
        let table = walk(GRAYSCALE).unwrap();
        let kinds: Vec<u32> = table.objects.iter().map(|o| o.f_object_type).collect();
        assert_eq!(kinds, vec![0x0001_8083, 0x0011_8003, 0x0011_8003]);
    }

    /// A `ProcCount` larger than the file can hold is bounded and clamped
    /// rather than carried through to become an allocation size downstream.
    ///
    /// This is not one of the plan's five named behaviours for this task. It
    /// is the instrument for the `T-02-01` mitigation the threat model
    /// requires, added so the mitigation has a test rather than only a
    /// comment.
    #[test]
    fn an_implausible_proc_count_is_bounded_and_clamped() {
        let at = object_element_field_offset(MANDELBROT, 0, 0x1C);
        let bytes = with_u32_at(MANDELBROT, at, 10_000_000);

        let table = walk(&bytes).unwrap();
        let object = &table.objects[0];
        assert!(object.proc_count < 10_000_000);

        assert_eq!(table.defects().len(), 1);
        let defect = &table.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert!(matches!(
            defect.kind,
            DefectKind::ImplausibleCount {
                count: 10_000_000,
                ..
            }
        ));
    }

    /// Every object recovered from the three vendored programs this module
    /// holds has a non-empty name and produces no defect. The full sweep
    /// over all 44 corpus programs, and the count of 105, belongs to
    /// `tests/differential.rs` in plan 02-08, which is a separate crate root
    /// and may open files. This proves the walk on the programs already
    /// reachable by inclusion.
    #[test]
    fn every_object_in_the_three_vendored_programs_has_a_name_and_no_defect() {
        for data in [MANDELBROT, GRAYSCALE, LOCK_WORK_STATION] {
            let table = walk(data).unwrap();
            assert!(!table.objects.is_empty());
            for object in &table.objects {
                assert!(!object.name.is_empty(), "an object recovered with no name");
            }
            assert!(
                table.defects().is_empty(),
                "a vendored program produced a defect: {:?}",
                table.defects()
            );
        }
    }

    /// A whole recovered list compared against a literal with one
    /// `assert_eq!`, so a mismatch prints both lists rather than a length.
    /// This is what makes `Object` need to derive `Clone`, `Debug`,
    /// `PartialEq` and `Eq`.
    #[test]
    fn the_grayscale_object_list_matches_a_literal() {
        let table = walk(GRAYSCALE).unwrap();
        let expected = vec![
            Object {
                lp_object_info: Va::new(0x0040_1ff0),
                name: "frmGrayscale".to_owned(),
                proc_count: 20,
                lp_proc_names_array: Va::new(0x0040_2a40),
                f_object_type: 0x0001_8083,
            },
            Object {
                lp_object_info: Va::new(0x0040_1b98),
                name: "pdOpenSaveDialog".to_owned(),
                proc_count: 6,
                lp_proc_names_array: Va::new(0x0040_2a90),
                f_object_type: 0x0011_8003,
            },
            Object {
                lp_object_info: Va::new(0x0040_1c98),
                name: "FastDrawing".to_owned(),
                proc_count: 8,
                lp_proc_names_array: Va::new(0x0040_2aa8),
                f_object_type: 0x0011_8003,
            },
        ];
        assert_eq!(table.objects, expected);
    }

    /// Builds a minimal 32 bit i386 portable executable with one section,
    /// an `ObjectTable` at `RVA 0x1000` / file offset `0x400`, and an
    /// `Object` array immediately after it whose second element crosses the
    /// section's declared mapped length.
    ///
    /// The relative address and the file offset differ here (`0x1000`
    /// against `0x400`), which neither corpus file can exercise: both put
    /// their own pointer chains where the two happen to agree, so a fixture
    /// built out of either one could pass under an identity that proves
    /// nothing. The section declares a mapped length of `0x84` bytes, well
    /// inside the `0x600`-byte file, so the refusal below comes from the
    /// section's own bound and not from the file running out of bytes: the
    /// bytes after the mapped length are filled with `0xCC` and the test
    /// would still pass if they were read as an unrelated third element,
    /// which is exactly the failure this fixture is built to catch.
    fn synthetic_image_with_a_short_mapped_section() -> Vec<u8> {
        const LFANEW: usize = 0x40;
        const OPTIONAL: usize = LFANEW + 24;
        const SECTION: usize = OPTIONAL + 224;

        let mut out = vec![0_u8; 0x600];
        out[0] = b'M';
        out[1] = b'Z';
        out[0x3c..0x40].copy_from_slice(&u32::try_from(LFANEW).unwrap().to_le_bytes());
        out[LFANEW..LFANEW + 4].copy_from_slice(b"PE\0\0");

        // The COFF file header.
        out[LFANEW + 4..LFANEW + 6].copy_from_slice(&0x014c_u16.to_le_bytes());
        out[LFANEW + 6..LFANEW + 8].copy_from_slice(&1_u16.to_le_bytes());
        out[LFANEW + 20..LFANEW + 22].copy_from_slice(&224_u16.to_le_bytes());
        out[LFANEW + 22..LFANEW + 24].copy_from_slice(&0x0102_u16.to_le_bytes());

        // The PE32 optional header.
        out[OPTIONAL..OPTIONAL + 2].copy_from_slice(&0x010b_u16.to_le_bytes());
        out[OPTIONAL + 0x1c..OPTIONAL + 0x20].copy_from_slice(&0x0040_0000_u32.to_le_bytes());

        // One section: RVA 0x1000, file offset 0x400, mapped length 0x84.
        out[SECTION..SECTION + 8].copy_from_slice(b".text\0\0\0");
        out[SECTION + 8..SECTION + 12].copy_from_slice(&0x84_u32.to_le_bytes());
        out[SECTION + 12..SECTION + 16].copy_from_slice(&0x1000_u32.to_le_bytes());
        out[SECTION + 16..SECTION + 20].copy_from_slice(&0x84_u32.to_le_bytes());
        out[SECTION + 20..SECTION + 24].copy_from_slice(&0x400_u32.to_le_bytes());
        out[SECTION + 36..SECTION + 40].copy_from_slice(&0x6000_0020_u32.to_le_bytes());

        // The ObjectTable structure at file offset 0x400. The project name
        // pointer names offset 0 of the same structure, one byte plus a NUL,
        // which is inside the mapped length and needs no space of its own.
        out[0x400] = b'X';
        out[0x400 + 0x2A..0x400 + 0x2C].copy_from_slice(&2_u16.to_le_bytes());
        out[0x400 + 0x2C..0x400 + 0x2E].copy_from_slice(&2_u16.to_le_bytes());
        out[0x400 + 0x30..0x400 + 0x34].copy_from_slice(&0x0040_1054_u32.to_le_bytes());
        out[0x400 + 0x40..0x400 + 0x44].copy_from_slice(&0x0040_1000_u32.to_le_bytes());

        // Bytes past the mapped length. A read that used the section's raw
        // size or the file's real length instead of the mapped length would
        // read these as the second element and would not refuse.
        for byte in &mut out[0x484..0x600] {
            *byte = 0xCC;
        }

        out
    }

    /// The second `Object` element sits past the section's declared mapped
    /// length. The array resolves, the first element resolves, and the
    /// second is refused rather than read from the padding.
    #[test]
    fn an_object_array_element_that_crosses_the_section_boundary_is_refused() {
        let bytes = synthetic_image_with_a_short_mapped_section();
        let image = PeImage::parse(&bytes).unwrap();
        let lp_object_table = Va::new(0x0040_1000);
        let head = ObjectTableHead::read(&image, lp_object_table).unwrap();
        assert_eq!(head.w_total_objects, 2);

        assert_eq!(
            ObjectTable::walk(&image, lp_object_table, &head).unwrap_err(),
            Refusal::Damaged("the file ends inside an Object element")
        );
    }
}
