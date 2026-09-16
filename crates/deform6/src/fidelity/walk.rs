//! The second walk: reaching every structure again so it can be graded.
//!
//! # Why a second walk and not provenance on the readers
//!
//! Most parsed structures in this crate do not keep the file offset they were
//! read from. Three do: `VbHeader`, `ProjectInfo` and `ControlInfo` each hold
//! a `file_offset`, because a defect about a count that one of them carries
//! must name the byte of that count. The other five structures this module
//! grades keep no offset, and to add one to each would change every reader
//! under `vb/` to serve a measurement.
//!
//! The walk does not use the three offsets that exist. It finds every
//! position itself. If it used them, a reader that read a record from the
//! wrong place and kept that place would be graded against the same wrong
//! bytes, and the record would pass.
//!
//! The readers do keep the addresses, though. `VbHeader` holds
//! `lp_project_data`, `ProjectInfo` holds `lp_object_table`, and so on down.
//! So this module resolves the same chain a second time and arrives at the
//! same places, and `the_header_ledger_lands_on_the_offset_that_inspect_reports`
//! in `tests/byte_fidelity.rs` is what proves it arrives at the same places.
//!
//! # `lpObjectArray` is restated here, not imported
//!
//! `vb::object::object_array_va` is private, so this module reads
//! `lpObjectArray` at `ObjectTable + 0x30` from `docs/STRUCTURES.md` section 4
//! instead. That is the independence rule working rather than a way around a
//! visibility problem: an emitter that shared the reader's constant would
//! agree with the reader about a wrong one.

use crate::error::Refusal;
use crate::fidelity::census::{Array, Count, Evidence, Outcome, Owner, outcome};
use crate::fidelity::ledger::Ledger;
use crate::fidelity::privateobj::PrivateObjRecord;
use crate::fidelity::{Emit, Fault, compare};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Va};
use crate::vb::controlinfo::{ControlInfo, ControlInfoTable, OptionalObjectInfo, read_event_table};
use crate::vb::gui::{GuiTable, GuiTableEntry};
use crate::vb::header::{VbHeader, header_region};
use crate::vb::object::{Object, ObjectTable};
use crate::vb::privateobj::{ObjectInfo, PrivateObj};
use crate::vb::project::{ObjectTableHead, ProjectInfo};

/// `STRUCTURES.md` section 4: `lpObjectArray` sits at `ObjectTable + 0x30`.
const LP_OBJECT_ARRAY: u32 = 0x30;

/// `STRUCTURES.md` section 4: the `ObjectTable` structure is `0x54` bytes.
const OBJECT_TABLE_SIZE: u32 = 0x54;

/// `STRUCTURES.md` section 4: `wTotalObjects` sits at `ObjectTable + 0x2A`.
const W_TOTAL_OBJECTS: u32 = 0x2A;

/// `STRUCTURES.md` section 2: `wFormCount` sits at `VBHeader + 0x44`.
const W_FORM_COUNT: u32 = 0x44;

/// `STRUCTURES.md` section 5.3: the block sits at `lpObjectInfo + 0x38`.
const OPTIONAL_OBJECT_INFO_AT: u32 = 0x38;

/// `STRUCTURES.md` section 5.3: `dwControlCount` sits at block `+ 0x20`.
const DW_CONTROL_COUNT: u32 = 0x20;

/// `STRUCTURES.md` section 8.6: `wEventCount` sits at element `+ 0x02`.
const W_EVENT_COUNT: u32 = 0x02;

/// What stopped a walk.
#[derive(Clone, Debug, thiserror::Error)]
pub enum WalkError {
    /// The file would not read. This is about the file.
    #[error(transparent)]
    Read(#[from] Refusal),
    /// An emitter in this repository misbehaved. This is not about the file.
    #[error(transparent)]
    Emit(#[from] Fault),
}

/// Everything one walk found in one file.
///
/// A value rather than a bare list, so that what the walk measures beside the
/// ledgers has a place to go without changing this function's signature
/// again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Walk {
    /// One ledger per structure graded, in the order the walk reached them.
    pub ledgers: Vec<Ledger>,
    /// One row per structure the walk reached and could not grade.
    pub ungraded: Vec<Ungraded>,
    /// One row per array counted, in the order the walk reached them.
    pub counts: Vec<Count>,
}

/// A structure the walk reached and could not grade.
///
/// A refusal that concerns one object is recorded here and the walk goes on,
/// because the census exists to show where the reader returns less than the
/// file declares, and a walk that stopped at the first such object would hide
/// every object after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ungraded {
    /// The structure that was not graded.
    pub structure: &'static str,
    /// What it belongs to.
    pub owner: Owner,
    /// Why it was not graded.
    pub reason: Reason,
}

/// Why a structure was not graded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The file holds no such record for this owner, and that is not a fault.
    ///
    /// A standard module has no `PrivateObj`, for example.
    Absent,
    /// The reader refused it. This is a fact about the file.
    Refused(Refusal),
}

/// Grades every structure this module knows how to emit in one file, and
/// counts every array the census knows.
///
/// The ledgers come in the order the walk reaches the structures. First come
/// the VB header, `ProjectInfo`, each GUI table entry, and each `Object`
/// element. Then each object adds its `ObjectInfo`, its `PrivateObj`, its
/// `OptionalObjectInfo`, and its `ControlInfo` elements, in that order.
///
/// A structure that the walk reaches and cannot grade gets a row in
/// `ungraded` and no ledger. The walk does not try a `PrivateObj` for an
/// object whose `ObjectInfo` did not read, so that object gets no row for it.
///
/// The counts start with the GUI table and the object array. Then each object
/// whose `OptionalObjectInfo` was read adds one row for its controls, and each
/// graded `ControlInfo` adds one row for its event slots.
///
/// # Errors
///
/// Returns [`WalkError::Read`] when the PE image, the VB header,
/// `ProjectInfo`, the GUI table, the object table, or one element of those
/// two tables does not read. A refusal about the structures under one object
/// does not stop the walk: it becomes a row in `ungraded` or a refused census
/// row. Returns [`WalkError::Emit`] when an emitter in this crate is at
/// fault.
pub fn walk(data: &[u8]) -> Result<Walk, WalkError> {
    let pe = PeImage::parse(data).map_err(Refusal::from)?;

    let hdr = header_region(&pe)?;
    let header = VbHeader::read(&hdr)?;
    let mut ledgers = vec![compare(&header, &hdr)?];

    let info = ProjectInfo::read(&pe, header.lp_project_data)?;
    let project = pe
        .region_at_va(header.lp_project_data)
        .ok_or(Refusal::Damaged("the ProjectInfo pointer is in no section"))?;
    ledgers.push(compare(
        &info,
        &window::<ProjectInfo>(&project, "the file ends inside ProjectInfo")?,
    )?);

    let gui_table = GuiTable::walk(&pe, &header)?;
    let gui_array = pe
        .region_at_va(header.lp_gui_table)
        .ok_or(Refusal::Damaged("the GUI table pointer is in no section"))?;
    for (index, entry) in gui_table.entries.iter().enumerate() {
        let at =
            element::<GuiTableEntry>(&gui_array, index, "the file ends inside a GUI table entry")?;
        ledgers.push(compare(entry, &at)?);
    }

    let head = ObjectTableHead::read(&pe, info.lp_object_table)?;
    let table = ObjectTable::walk(&pe, info.lp_object_table, &head)?;

    let object_table = pe
        .region_at_va(info.lp_object_table)
        .ok_or(Refusal::Damaged(
            "the object table pointer is in no section",
        ))?
        .subregion(Off::new(0), OBJECT_TABLE_SIZE)
        .ok_or(Refusal::Damaged(
            "the file ends inside the object table structure",
        ))?;
    let lp_object_array = object_table
        .va_le(Off::new(LP_OBJECT_ARRAY))
        .ok_or(Refusal::Damaged(
            "the object table holds no address for the object array",
        ))?;
    let array = pe.region_at_va(lp_object_array).ok_or(Refusal::Damaged(
        "the object array pointer is in no section",
    ))?;

    for (index, object) in table.objects.iter().enumerate() {
        let ordinal = u32::try_from(index)
            .map_err(|_ignored| Refusal::Damaged("the object array index leaves a u32"))?;
        let at = ordinal
            .checked_mul(Object::LEN)
            .ok_or(Refusal::Damaged("the object array index overflows a u32"))?;
        let element = array
            .subregion(Off::new(at), Object::LEN)
            .ok_or(Refusal::Damaged("the file ends inside an Object element"))?;
        ledgers.push(compare(object, &element)?);
    }

    let mut found = Walk {
        ledgers,
        ungraded: Vec::new(),
        counts: vec![
            count_gui_table(&hdr, &header, &gui_table)?,
            count_objects(&object_table, &head, &table)?,
        ],
    };

    for (index, object) in table.objects.iter().enumerate() {
        let owner = object_owner(index)?;
        if let Some(info) = grade_object_info(&pe, object, owner, &mut found)? {
            grade_private_obj(&pe, &info, owner, &mut found)?;
        }
        if let Some(optional) = grade_optional_object_info(&pe, object, owner, &mut found)?
            && let Some(table) = count_controls(&pe, object, &optional, owner, &mut found)?
        {
            grade_controls(&pe, &optional, &table, owner, &mut found)?;
        }
    }

    Ok(found)
}

/// Names one object as the owner of what the walk finds under it.
fn object_owner(index: usize) -> Result<Owner, Refusal> {
    let object = u32::try_from(index)
        .map_err(|_ignored| Refusal::Damaged("the object array index leaves a u32"))?;
    Ok(Owner::Object { object })
}

/// Resolves `va` and cuts the window to exactly the length of `T`.
fn located<'a, T: Emit>(
    pe: &PeImage<'a>,
    va: Va,
    what: &'static str,
) -> Result<Region<'a>, Refusal> {
    let region = pe.region_at_va(va).ok_or(Refusal::Damaged(what))?;
    window::<T>(&region, what)
}

/// Grades one object's `ObjectInfo`, or records why it could not.
///
/// A refusal here concerns this one object, so it is recorded and the walk
/// goes on. An emitter fault still stops the walk: a walk that carried on
/// past a fault in this repository could report a result that looks clean.
/// Gives back the `ObjectInfo` it read, because the structures below it are
/// reached through it.
fn grade_object_info(
    pe: &PeImage<'_>,
    object: &Object,
    owner: Owner,
    found: &mut Walk,
) -> Result<Option<ObjectInfo>, WalkError> {
    let read = ObjectInfo::read(pe, object.lp_object_info).and_then(|info| {
        let record = located::<ObjectInfo>(
            pe,
            object.lp_object_info,
            "the file ends inside an ObjectInfo",
        )?;
        Ok((info, record))
    });
    match read {
        Ok((info, record)) => {
            found.ledgers.push(compare(&info, &record)?);
            Ok(Some(info))
        }
        Err(reason) => {
            found.ungraded.push(Ungraded {
                structure: ObjectInfo::STRUCTURE,
                owner,
                reason: Reason::Refused(reason),
            });
            Ok(None)
        }
    }
}

/// Grades one object's `OptionalObjectInfo` block, or records why it could
/// not.
///
/// Tried for every object on its own, not only when `ObjectInfo` was read:
/// the block's declared control count is what the census compares against,
/// and it must survive whatever happened to the record before it.
///
/// Gives back the block whenever it was read, even when its window could not
/// be graded, because the count is still the file's own number.
fn grade_optional_object_info(
    pe: &PeImage<'_>,
    object: &Object,
    owner: Owner,
    found: &mut Walk,
) -> Result<Option<OptionalObjectInfo>, WalkError> {
    let skip = |reason: Reason| Ungraded {
        structure: OptionalObjectInfo::STRUCTURE,
        owner,
        reason,
    };
    let info = match OptionalObjectInfo::read(pe, object) {
        Ok(Some(info)) => info,
        Ok(None) => {
            found.ungraded.push(skip(Reason::Absent));
            return Ok(None);
        }
        Err(reason) => {
            found.ungraded.push(skip(Reason::Refused(reason)));
            return Ok(None);
        }
    };
    let block = pe
        .region_at_va(object.lp_object_info)
        .ok_or(Refusal::Damaged(
            "an Object's lpObjectInfo is in no section",
        ))
        .and_then(|base| {
            base.subregion(Off::new(OPTIONAL_OBJECT_INFO_AT), OptionalObjectInfo::LEN)
                .ok_or(Refusal::Damaged(
                    "the file ends inside an OptionalObjectInfo block",
                ))
        });
    match block {
        Ok(block) => found.ledgers.push(compare(&info, &block)?),
        Err(reason) => found.ungraded.push(skip(Reason::Refused(reason))),
    }
    Ok(Some(info))
}

/// Counts the `ControlInfo` entries one object's block declares against the
/// entries the reader returned.
///
/// This is the census row for `controlinfo::bound_control_count`, the clamp
/// that bounds a loop and changes no byte. The walk decides for itself whether
/// the array's address maps anywhere, so an unmapped array is told apart from
/// a clamp without asking the reader.
///
/// Gives back the table the reader returned, for the structures below it.
fn count_controls(
    pe: &PeImage<'_>,
    object: &Object,
    optional: &OptionalObjectInfo,
    owner: Owner,
    found: &mut Walk,
) -> Result<Option<ControlInfoTable>, WalkError> {
    let Some(declared_at) = pe
        .region_at_va(object.lp_object_info)
        .and_then(|base| base.file_offset(Off::new(OPTIONAL_OBJECT_INFO_AT)))
        .and_then(|block| block.checked_add(DW_CONTROL_COUNT))
    else {
        // The count field cannot be located, so there is no row to state.
        return Ok(None);
    };
    let declared = optional.dw_control_count;

    let (recovered, decided, table) = match ControlInfoTable::read(pe, object) {
        Ok(table) => {
            let recovered = u32::try_from(table.entries.len())
                .map_err(|_ignored| Refusal::Damaged("the control entry count leaves a u32"))?;
            let decided = outcome(&Evidence {
                array: Array::Controls,
                declared,
                recovered,
                defects: table.defects(),
                unmapped: pe.region_at_va(optional.lp_controls).is_none(),
                unsupported_control_type: None,
            });
            (recovered, decided, Some(table))
        }
        Err(reason) => (0, Outcome::Refused(reason), None),
    };

    found.counts.push(Count {
        array: Array::Controls,
        owner,
        declared_at,
        declared,
        recovered,
        outcome: decided,
    });
    Ok(table)
}

/// Grades every `ControlInfo` entry the reader returned for one object, or
/// records why one could not be graded.
///
/// An array whose address maps nowhere has no entries to grade: the reader
/// already returned none, and the census row says why.
fn grade_controls(
    pe: &PeImage<'_>,
    optional: &OptionalObjectInfo,
    table: &ControlInfoTable,
    owner: Owner,
    found: &mut Walk,
) -> Result<(), WalkError> {
    let Owner::Object { object } = owner else {
        return Ok(());
    };
    let Some(array) = pe.region_at_va(optional.lp_controls) else {
        return Ok(());
    };
    for (index, control) in table.entries.iter().enumerate() {
        let control_owner = Owner::Control {
            object,
            control: u32::try_from(index)
                .map_err(|_ignored| Refusal::Damaged("the control index leaves a u32"))?,
        };
        match element::<ControlInfo>(&array, index, "the file ends inside a ControlInfo element") {
            Ok(window) => {
                found.ledgers.push(compare(control, &window)?);
                count_event_slots(pe, control, &window, control_owner, found)?;
            }
            Err(reason) => found.ungraded.push(Ungraded {
                structure: ControlInfo::STRUCTURE,
                owner: control_owner,
                reason: Reason::Refused(reason),
            }),
        }
    }
    Ok(())
}

/// Counts the event slots one control declares against the slots the reader
/// returned.
///
/// This is the census row for `controlinfo::bound_event_count`. The reader
/// returns no slots, and says so, for a control type it knows no header
/// layout for, and it refuses a table whose address maps nowhere; the row
/// keeps those two apart from a clamp.
fn count_event_slots(
    pe: &PeImage<'_>,
    control: &ControlInfo,
    element: &Region<'_>,
    owner: Owner,
    found: &mut Walk,
) -> Result<(), WalkError> {
    let Some(declared_at) = element.file_offset(Off::new(W_EVENT_COUNT)) else {
        // The count field cannot be located, so there is no row to state.
        return Ok(());
    };
    let declared = u32::from(control.w_event_count);

    let (recovered, decided) = match read_event_table(pe, control) {
        Ok(table) => {
            let recovered = u32::try_from(table.slots.len())
                .map_err(|_ignored| Refusal::Damaged("the event slot count leaves a u32"))?;
            let decided = outcome(&Evidence {
                array: Array::EventSlots,
                declared,
                recovered,
                defects: table.defects(),
                unmapped: false,
                unsupported_control_type: table.unsupported_control_type,
            });
            (recovered, decided)
        }
        Err(reason) => (0, Outcome::Refused(reason)),
    };

    found.counts.push(Count {
        array: Array::EventSlots,
        owner,
        declared_at,
        declared,
        recovered,
        outcome: decided,
    });
    Ok(())
}

/// Grades one object's `PrivateObj`, or records why it could not.
///
/// A standard module has no private object. That is recorded as `Absent`,
/// which is a fact about the program and not a fault in anything.
fn grade_private_obj(
    pe: &PeImage<'_>,
    info: &ObjectInfo,
    owner: Owner,
    found: &mut Walk,
) -> Result<(), WalkError> {
    let skip = |reason: Reason| Ungraded {
        structure: PrivateObjRecord::STRUCTURE,
        owner,
        reason,
    };
    let value = match PrivateObj::read(pe, info.lp_private_object) {
        Ok(value) => value,
        Err(reason) => {
            found.ungraded.push(skip(Reason::Refused(reason)));
            return Ok(());
        }
    };
    let Some(record) = PrivateObjRecord::of(&value) else {
        found.ungraded.push(skip(Reason::Absent));
        return Ok(());
    };
    match located::<PrivateObjRecord>(
        pe,
        Va::new(info.lp_private_object),
        "the file ends inside a PrivateObj",
    ) {
        Ok(window) => found.ledgers.push(compare(&record, &window)?),
        Err(reason) => found.ungraded.push(skip(Reason::Refused(reason))),
    }
    Ok(())
}

/// Cuts `region` to exactly the length of `T`.
///
/// A window shorter than the structure is a fact about the file, so it is a
/// refusal here. Passing a short window to `compare` would report it as
/// `Fault::Short`, which says this repository's emitter was wrong, and it was
/// not.
fn window<'a, T: Emit>(region: &Region<'a>, what: &'static str) -> Result<Region<'a>, Refusal> {
    region
        .subregion(Off::new(0), T::LEN)
        .ok_or(Refusal::Damaged(what))
}

/// Cuts element `index` out of an array of `T`, at a stride of `T::LEN`.
///
/// The stride is the emitter's own length, restated from the format
/// document, and never the reader's constant.
fn element<'a, T: Emit>(
    array: &Region<'a>,
    index: usize,
    what: &'static str,
) -> Result<Region<'a>, Refusal> {
    let ordinal = u32::try_from(index).map_err(|_ignored| Refusal::Damaged(what))?;
    let at = ordinal.checked_mul(T::LEN).ok_or(Refusal::Damaged(what))?;
    array
        .subregion(Off::new(at), T::LEN)
        .ok_or(Refusal::Damaged(what))
}

/// Counts the GUI table entries the header declares against the entries the
/// reader returned.
///
/// The GUI table walk clamps `wFormCount` to the entries the file can hold and
/// raises a defect when it does, so this is the row where that clamp would
/// show. It fires only when the GUI table ends its section. Anywhere else,
/// raising `wFormCount` makes the walk read past the real entries and refuse
/// the whole table on the next `lStructSize` before any clamp could matter.
fn count_gui_table(
    header_window: &Region<'_>,
    header: &VbHeader,
    table: &GuiTable,
) -> Result<Count, WalkError> {
    let declared_at = header_window
        .file_offset(Off::new(W_FORM_COUNT))
        .ok_or(Refusal::Damaged("the form count has no file offset"))?;
    let declared = u32::from(header.w_form_count);
    let recovered = u32::try_from(table.entries.len())
        .map_err(|_ignored| Refusal::Damaged("the GUI table entry count leaves a u32"))?;
    Ok(Count {
        array: Array::GuiTable,
        owner: Owner::Program,
        declared_at,
        declared,
        recovered,
        outcome: outcome(&Evidence {
            array: Array::GuiTable,
            declared,
            recovered,
            defects: table.defects(),
            unmapped: false,
            unsupported_control_type: None,
        }),
    })
}

/// Counts the objects the object table declares against the objects the
/// reader returned.
///
/// `ObjectTable::walk` refuses rather than clamps when an element will not
/// read, so this row can only be whole or the walk has already stopped. It is
/// still counted: the census states every array it can reach, including the
/// ones that cannot come up short, so that a later reader that did start to
/// clamp here would show up.
fn count_objects(
    object_table: &Region<'_>,
    head: &ObjectTableHead,
    table: &ObjectTable,
) -> Result<Count, WalkError> {
    let declared_at =
        object_table
            .file_offset(Off::new(W_TOTAL_OBJECTS))
            .ok_or(Refusal::Damaged(
                "the object table count has no file offset",
            ))?;
    let declared = u32::from(head.w_total_objects);
    let recovered = u32::try_from(table.objects.len())
        .map_err(|_ignored| Refusal::Damaged("the object count leaves a u32"))?;
    Ok(Count {
        array: Array::Objects,
        owner: Owner::Program,
        declared_at,
        declared,
        recovered,
        outcome: outcome(&Evidence {
            array: Array::Objects,
            declared,
            recovered,
            defects: table.defects(),
            unmapped: false,
            unsupported_control_type: None,
        }),
    })
}
