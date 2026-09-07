//! `ObjectInfo`, `PrivateObj`, and the procedure name walk.
//!
//! This is OBJ-03 and OBJ-06: recover every public procedure name an object
//! carries, and report a private procedure as private rather than inventing a
//! name for it.
//!
//! # Two null cases, not one
//!
//! `STRUCTURES.md` section 5.1 says a null entry in `Object.lpProcNamesArray`
//! means the procedure at that index is private. That is a fact about one
//! entry. A whole `lpProcNamesArray` **pointer** of `0` is a different fact
//! about the object: measured across the 44 corpus programs, this is exactly
//! the 8 of 105 objects that are standard modules (`.bas`), and their real
//! `ProcCount` (1 to 7) is never reachable through this structure at all. The
//! two cases are modelled as two different values of two different shapes so
//! that a caller cannot mistake "every procedure is private" for "this object
//! carries no procedure names whatsoever". See [`ProcNames`].
//!
//! # A non-null entry is not necessarily a name either
//!
//! `CONTEXT.md`, "The procedure name array does not behave as documented",
//! corrects the worked example this plan inherited. `Mandelbrot.exe`'s
//! `frmFractal` was asserted to hold nine **null** entries. Measured, its
//! nine entries are **not** null: the array was never written by the
//! compiler and still holds a fragment of a build-machine path,
//! `mData\Oracle\Java\`, read back as nine little-endian dwords. Every one of
//! those nine procedures genuinely is private (the source declares all nine
//! `Private`), but the reason is not "the entry is null". Every non-null
//! entry is therefore validated before it is trusted, never merely resolved
//! and printed. See [`ProcedureList::read`] for the exact test this file
//! applies, and why it applies it.
//!
//! # The standard-module cap, stated here before any ratio is pinned
//!
//! GD's own research, which `RESEARCH.md` section 6 carries forward, states
//! the reason plainly: this type data is emitted into `.text` as part of the
//! standard `IDispatch` plumbing every user form, class and user control
//! needs at run time, and the compiler cannot strip it. A standard module is
//! not such an object. Its procedures are declared, `Object.ProcCount`
//! reports them correctly, and yet none of them has a name or a prototype
//! reachable through `Object`, `ObjectInfo` or `PrivateObj`. Measured: 8 of 8
//! corpus module objects carry a null `lpProcNamesArray` pointer, with
//! `ProcCount` from 1 to 7. This is a cap on what OBJ-03 and OBJ-04 can ever
//! promise for a `.bas`, not a per-file anomaly, and it is named here, before
//! plan 02-09 pins the first ratio, per decision D-10.

use crate::error::Refusal;
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Va};

/// The size of the `ObjectInfo` structure.
///
/// `STRUCTURES.md` section 5.2 gives `0x38` = 56 bytes.
const OBJECT_INFO_SIZE: u32 = 0x38;

/// The size of the `PrivateObj` structure.
///
/// `STRUCTURES.md` section 6.1 gives `0x40` = 64 bytes.
const PRIVATE_OBJ_SIZE: u32 = 0x40;

/// The sentinel `ObjectInfo.lpPrivateObject` carries for a standard module.
///
/// `STRUCTURES.md` section 5.2 cites Semi VB Decompiler's note that this
/// field is `-1` for a `.bas`. Measured: it is also plainly `0` in a
/// synthetic value nothing in the corpus needs, and `PrivateObj::read` treats
/// both as the same fact, per the plan's own instruction.
const NO_PRIVATE_OBJECT: u32 = 0xFFFF_FFFF;

/// The head of `ObjectInfo`, reached from `Object.lpObjectInfo`.
///
/// Two fields are read, per `STRUCTURES.md` section 5.2. `wMethodCount` and
/// `lpMethods` point into code in a native build and are Phase 3 and later
/// work; `lpProjectData` is in-memory scratch. None of the three is read
/// here, and this doc comment is the reason a later reader should not extend
/// [`ObjectInfo::read`] to reach for them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectInfo {
    /// This object's own index in the object array.
    pub w_object_index: u16,
    /// The address of this object's `PrivateObj`, or the sentinel that means
    /// there is none.
    ///
    /// This is a raw `u32` and not a [`Va`]. `-1` is a sentinel, not an
    /// address: a `Va` that carried `0xFFFF_FFFF` would invite a caller to
    /// resolve it, and there is nothing at that address to resolve.
    /// [`PrivateObj::read`] takes this raw value and decides the sentinel
    /// question before it builds a [`Va`] from anything.
    pub lp_private_object: u32,
}

impl ObjectInfo {
    /// Reads `ObjectInfo` at the address `Object.lpObjectInfo` holds.
    ///
    /// The window is exactly [`OBJECT_INFO_SIZE`] bytes, taken before any
    /// field is read, matching the window-before-fields discipline
    /// `vb/project.rs` documents.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when the address is in no section, and
    /// when the file holds fewer than 56 bytes there.
    pub fn read(pe: &PeImage<'_>, lp_object_info: Va) -> Result<Self, Refusal> {
        let at = pe
            .region_at_va(lp_object_info)
            .ok_or(Refusal::Damaged("the ObjectInfo pointer is in no section"))?;
        let window = at
            .subregion(Off::new(0), OBJECT_INFO_SIZE)
            .ok_or(Refusal::Damaged(
                "the file ends inside the ObjectInfo structure",
            ))?;
        Ok(Self {
            w_object_index: u16_at(&window, 0x02, "ObjectInfo holds no object index")?,
            lp_private_object: u32_at(
                &window,
                0x0C,
                "ObjectInfo holds no address for its PrivateObj",
            )?,
        })
    }
}

/// `PrivateObj`, reached from `ObjectInfo.lpPrivateObject`.
///
/// # The module sentinel is a case, not an error
///
/// A standard module (`.bas`) is not a COM object, so it carries no
/// `PrivateObj` at all: `STRUCTURES.md` section 5.2 records that
/// `lpPrivateObject` reads `-1` for one, and this file also treats a plain
/// `0` the same way. Refusing here would lose every module in the corpus, so
/// the absence is a state this type carries rather than an error this
/// function returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrivateObj {
    /// The object carries a real `PrivateObj`.
    Present {
        /// The number of entries in the `PubVarDesc` array.
        ///
        /// **Distrust this field.** Per decision D-14, it does not count
        /// source-level `Public variable As Type` declarations. Measured:
        /// `pdOpenSaveDialog.cls`, present in 15 of the 44 corpus programs,
        /// declares zero such lines (only a `Public Enum`), and every corpus
        /// binary reports `cnt_public_vars` of `4` for it regardless. This
        /// field is carried, unexplained, for plan 02-05 to own. Nothing
        /// downstream should be built on the assumption that it means what
        /// its name says.
        cnt_public_vars: u16,
        /// The number of entries in the `EventDesc` array.
        ///
        /// Measured `0` for every object in every one of the 44 corpus
        /// programs. `[VERIFIED: local]` Plan 02-05 ships the address walk
        /// against synthetic fixtures for exactly this reason: the corpus
        /// carries no sample of a non-zero value.
        cnt_events: u16,
        /// The address of the `FuncTypDesc` pointer array.
        ///
        /// Index-parallel to `Object.lpProcNamesArray`, and of the same
        /// length, `Object.proc_count`. `STRUCTURES.md` section 6.1 states
        /// plainly that the count for this array is not carried in
        /// `PrivateObj` at all; it comes from `Object.ProcCount`, which plan
        /// 02-01 already reads and already bounds against the real size of
        /// the file. Plan 02-04 walks this array. Nothing here re-derives a
        /// count for it.
        lp_func_type_info: Va,
        /// The address of the `EventDesc` pointer array.
        lp_events_type_info: Va,
        /// The address of the `PubVarDesc` array.
        ///
        /// Carried for plan 02-05, which owns the record stride and the
        /// explanation of `cnt_public_vars`. This file claims nothing about
        /// either.
        lp_public_vars: Va,
    },
    /// The object carries no `PrivateObj`. This is the standard-module case.
    Absent,
}

impl PrivateObj {
    /// Reads `PrivateObj` at the address `ObjectInfo.lpPrivateObject` holds,
    /// or reports its absence.
    ///
    /// `lp_private_object` is the raw value [`ObjectInfo::lp_private_object`]
    /// carries. A plain `0` and the sentinel `0xFFFF_FFFF` both mean the
    /// object has no private object, per the doc comment on [`PrivateObj`],
    /// and neither is dereferenced: the sentinel check happens before this
    /// function builds a [`Va`] from anything.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when the address is a real address but it
    /// is in no section, and when the file holds fewer than 64 bytes there.
    pub fn read(pe: &PeImage<'_>, lp_private_object: u32) -> Result<Self, Refusal> {
        if lp_private_object == 0 || lp_private_object == NO_PRIVATE_OBJECT {
            return Ok(Self::Absent);
        }

        let va = Va::new(lp_private_object);
        let at = pe
            .region_at_va(va)
            .ok_or(Refusal::Damaged("the PrivateObj pointer is in no section"))?;
        let window = at
            .subregion(Off::new(0), PRIVATE_OBJ_SIZE)
            .ok_or(Refusal::Damaged(
                "the file ends inside the PrivateObj structure",
            ))?;

        Ok(Self::Present {
            cnt_public_vars: u16_at(&window, 0x10, "PrivateObj holds no public variable count")?,
            cnt_events: u16_at(&window, 0x12, "PrivateObj holds no event count")?,
            lp_func_type_info: va_at(
                &window,
                0x18,
                "PrivateObj holds no address for its FuncTypDesc array",
            )?,
            lp_public_vars: va_at(
                &window,
                0x20,
                "PrivateObj holds no address for its public variables",
            )?,
            lp_events_type_info: va_at(
                &window,
                0x24,
                "PrivateObj holds no address for its EventDesc array",
            )?,
        })
    }
}

/// Reads an unsigned 16-bit value out of a structure window.
fn u16_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<u16, Refusal> {
    window.u16_le(Off::new(at)).ok_or(Refusal::Damaged(what))
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
    use super::{OBJECT_INFO_SIZE, ObjectInfo, PrivateObj};
    use crate::error::Refusal;
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Va};
    use crate::vb::header::{VbHeader, header_region};
    use crate::vb::object::{Object, ObjectTable};
    use crate::vb::project::{ObjectTableHead, ProjectInfo};

    /// The program this task's tracer is worked against: three objects, none
    /// a module, none sharing the same `PrivateObj` shape as another.
    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    /// The program whose object count and object capacity differ, and whose
    /// two standard modules give the module sentinel. `[VERIFIED: local]`
    /// `Attribute VB_Name` inside each `.bas` names them the opposite of
    /// their file names: `Subs.bas` is `Attribute VB_Name = "Declaration_Module"`
    /// and `Declarations.bas` is `Attribute VB_Name = "Sub_Module"`.
    const MAP_EDITOR: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Map-editor-2D/Map Editor.exe"
    ));

    /// The program whose worked example this plan corrects: `frmFractal`'s
    /// nine `lpProcNamesArray` entries are not null. See the module doc
    /// comment.
    const MANDELBROT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
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
    /// chain `inspect` uses, and gives every recovered `Object`.
    ///
    /// This duplicates the walk `vb/object.rs`'s own tests carry, on
    /// purpose: the two modules must be able to fail independently, and a
    /// shared test helper would make one change break both.
    fn objects(data: &[u8]) -> Vec<Object> {
        let image = PeImage::parse(data).unwrap();
        let lp_object_table = object_table_va(data);
        let head = ObjectTableHead::read(&image, lp_object_table).unwrap();
        ObjectTable::walk(&image, lp_object_table, &head)
            .unwrap()
            .objects
    }

    /// `Grayscale.exe` declares one form and two classes. None is a module,
    /// so `ObjectInfo::read` resolves for all three, and each one's
    /// `lp_private_object` is a real address: neither `0` nor the module
    /// sentinel `0xFFFF_FFFF`.
    #[test]
    fn grayscale_object_info_resolves_for_all_three_objects_with_a_real_private_object() {
        let objs = objects(GRAYSCALE);
        assert_eq!(objs.len(), 3);
        let image = PeImage::parse(GRAYSCALE).unwrap();
        for (index, object) in objs.iter().enumerate() {
            let info = ObjectInfo::read(&image, object.lp_object_info).unwrap();
            assert_eq!(usize::from(info.w_object_index), index);
            assert_ne!(info.lp_private_object, 0);
            assert_ne!(info.lp_private_object, 0xFFFF_FFFF);
        }
    }

    /// `Map Editor.exe` declares two standard modules among its five objects.
    /// Each one's `ObjectInfo.lpPrivateObject` is the module sentinel, and
    /// reading it gives [`PrivateObj::Absent`] rather than a refusal: a
    /// refusal here would lose every module in the corpus.
    #[test]
    fn map_editor_module_objects_give_private_obj_absent_and_object_info_still_reads() {
        let objs = objects(MAP_EDITOR);
        assert_eq!(objs.len(), 5);
        let image = PeImage::parse(MAP_EDITOR).unwrap();

        let declaration_module = &objs[1];
        let sub_module = &objs[2];
        assert_eq!(declaration_module.name, "Declaration_Module");
        assert_eq!(declaration_module.proc_count, 1);
        assert_eq!(sub_module.name, "Sub_Module");
        assert_eq!(sub_module.proc_count, 7);

        for module in [declaration_module, sub_module] {
            let info = ObjectInfo::read(&image, module.lp_object_info).unwrap();
            assert_eq!(info.lp_private_object, 0xFFFF_FFFF);
            assert_eq!(
                PrivateObj::read(&image, info.lp_private_object).unwrap(),
                PrivateObj::Absent
            );
        }
    }

    /// A plain `0`, not only the sentinel `0xFFFF_FFFF`, also means the
    /// object carries no `PrivateObj`. No corpus program in this module
    /// exercises the plain-zero case, so this is a direct, synthetic check
    /// on the raw value alone: it needs no `PeImage` at all, because both
    /// sentinel checks happen before this function resolves an address.
    #[test]
    fn a_plain_zero_private_object_address_is_also_absent() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        assert_eq!(PrivateObj::read(&image, 0).unwrap(), PrivateObj::Absent);
    }

    /// `FastDrawing`, `Grayscale.exe`'s third object, gives a non-null
    /// `lp_func_type_info` and carries `cnt_public_vars` and `cnt_events`.
    #[test]
    fn grayscale_fast_drawing_private_obj_gives_non_null_func_type_info_and_carries_counts() {
        let objs = objects(GRAYSCALE);
        let fast_drawing = &objs[2];
        assert_eq!(fast_drawing.name, "FastDrawing");

        let image = PeImage::parse(GRAYSCALE).unwrap();
        let info = ObjectInfo::read(&image, fast_drawing.lp_object_info).unwrap();
        let private = PrivateObj::read(&image, info.lp_private_object).unwrap();
        match private {
            PrivateObj::Present {
                lp_func_type_info,
                cnt_public_vars,
                cnt_events,
                ..
            } => {
                assert!(!lp_func_type_info.is_null());
                // `cnt_public_vars` is carried and is not asserted against a
                // meaning: see the doc comment on `PrivateObj::Present`.
                let _ = cnt_public_vars;
                assert_eq!(cnt_events, 0);
            }
            PrivateObj::Absent => panic!("FastDrawing is a class, not a module"),
        }
    }

    /// `cnt_events` is `0` for every object in all three vendored programs
    /// this module reads. `RESEARCH.md` records the same finding across the
    /// whole 44-file corpus: no sample of a non-zero value exists anywhere.
    #[test]
    fn cnt_events_is_zero_for_every_object_in_the_three_vendored_programs() {
        for data in [GRAYSCALE, MAP_EDITOR, MANDELBROT] {
            let image = PeImage::parse(data).unwrap();
            for object in objects(data) {
                let info = ObjectInfo::read(&image, object.lp_object_info).unwrap();
                match PrivateObj::read(&image, info.lp_private_object).unwrap() {
                    PrivateObj::Present { cnt_events, .. } => assert_eq!(cnt_events, 0),
                    PrivateObj::Absent => {}
                }
            }
        }
    }

    /// A patched `lpObjectInfo` that resolves nowhere refuses the read.
    #[test]
    fn an_object_info_pointer_in_no_section_is_damaged() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let nowhere = Va::new(image.image_base() + 0x00F0_0000);
        assert!(image.region_at_va(nowhere).is_none());
        assert_eq!(
            ObjectInfo::read(&image, nowhere).unwrap_err(),
            Refusal::Damaged("the ObjectInfo pointer is in no section")
        );
    }

    /// A file cut off inside `ObjectInfo` is refused at the window, and is
    /// not read in part.
    #[test]
    fn a_file_truncated_inside_object_info_is_damaged_and_is_not_read_in_part() {
        let objs = objects(GRAYSCALE);
        let object = &objs[0];

        let image = PeImage::parse(GRAYSCALE).unwrap();
        let region = image.region_at_va(object.lp_object_info).unwrap();
        let start = usize::try_from(region.file_offset(Off::new(0)).unwrap().get()).unwrap();
        let cut = start + 0x10;
        assert!(cut < start + usize::try_from(OBJECT_INFO_SIZE).unwrap());
        assert!(cut < GRAYSCALE.len());

        let bytes = GRAYSCALE[..cut].to_vec();
        let truncated = PeImage::parse(&bytes).unwrap();
        assert_eq!(
            ObjectInfo::read(&truncated, object.lp_object_info).unwrap_err(),
            Refusal::Damaged("the file ends inside the ObjectInfo structure")
        );
    }
}
