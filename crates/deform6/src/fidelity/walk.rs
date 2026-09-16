//! The second walk: reaching every structure again so it can be graded.
//!
//! # Why a second walk and not provenance on the readers
//!
//! Almost no parsed structure in this crate keeps the file offset it was read
//! from. Threading one through every reader would touch every file under
//! `vb/` to serve a measurement, and it would put a field on a public struct
//! that nothing outside this module reads.
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
use crate::fidelity::census::{Array, Count, Evidence, Owner, outcome};
use crate::fidelity::ledger::Ledger;
use crate::fidelity::{Emit, Fault, compare};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region};
use crate::vb::header::{VbHeader, header_region};
use crate::vb::object::{Object, ObjectTable};
use crate::vb::project::{ObjectTableHead, ProjectInfo};

/// `STRUCTURES.md` section 4: `lpObjectArray` sits at `ObjectTable + 0x30`.
const LP_OBJECT_ARRAY: u32 = 0x30;

/// `STRUCTURES.md` section 4: the `ObjectTable` structure is `0x54` bytes.
const OBJECT_TABLE_SIZE: u32 = 0x54;

/// `STRUCTURES.md` section 4: `wTotalObjects` sits at `ObjectTable + 0x2A`.
const W_TOTAL_OBJECTS: u32 = 0x2A;

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
    /// One row per array counted, in the order the walk reached them.
    pub counts: Vec<Count>,
}

/// Grades every structure this module knows how to emit, in one file.
///
/// The ledgers hold one entry for the VB header, then one per object in array
/// order.
///
/// # Errors
///
/// Returns [`WalkError::Read`] when the file does not resolve, and
/// [`WalkError::Emit`] when an emitter in this crate is at fault.
pub fn walk(data: &[u8]) -> Result<Walk, WalkError> {
    let pe = PeImage::parse(data).map_err(Refusal::from)?;

    let hdr = header_region(&pe)?;
    let header = VbHeader::read(&hdr)?;
    let mut ledgers = vec![compare(&header, &hdr)?];

    let info = ProjectInfo::read(&pe, header.lp_project_data)?;
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

    let counts = vec![count_objects(&object_table, &head, &table)?];

    Ok(Walk { ledgers, counts })
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
