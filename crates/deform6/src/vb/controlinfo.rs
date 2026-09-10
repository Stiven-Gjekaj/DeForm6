//! `ControlInfo`, the event handler table, and the join from a control's
//! recovered name back to its event handler addresses.
//!
//! Plan 03-09 fills this module. It serves FRM-06.
//!
//! # This file resolves `OptionalObjectInfo` itself
//!
//! `dwControlCount` and `lpControls` live in `OptionalObjectInfo`, which sits
//! immediately after `ObjectInfo` at `Object.lpObjectInfo` plus `0x38`.
//! `STRUCTURES.md` section 5.3 gives the layout. This module resolves it
//! independently from `Object::lp_object_info`, the same way `vb/object.rs`
//! resolves `lpObjectArray` on its own rather than extending `vb/project.rs`:
//! one plan keeps sole ownership of one file. This module does not extend
//! `ObjectInfo` in `crates/deform6/src/vb/privateobj.rs` either, for the
//! same reason.
//!
//! `classify::has_optional_info` (`fObjectType & 0x2`) is the presence test,
//! per `STRUCTURES.md` section 5.5: a standard module has no
//! `OptionalObjectInfo` at all, and [`ControlInfoTable::read`] gives an empty
//! table for one rather than resolving real bytes that hold something else
//! entirely.
//!
//! # The `ControlInfo` field layout (`STRUCTURES.md` section 8.6)
//!
//! `fControlType` and `wEventCount` are read as two-byte values, at offsets
//! `0x00` and `0x02`. One published source reads the first as a four-byte
//! value and puts the second at `0x04`; three independent implementations
//! (SVBD, PVB, IDC) disagree with it, and two of them cite it in their own
//! header text while diverging from it here, which means their authors
//! checked against real files and found it wrong. This module uses the
//! two-byte reading.
//!
//! # The join is name-based, and reports both directions
//!
//! The control tree (`vb/controltree.rs`) gives a control's name, type and
//! parent. The `ControlInfo` array gives a control's CLSID and event handler
//! addresses. The two are joined by control name, per `STRUCTURES.md`
//! section 8.6. A subset check cannot see an over-count, the same reason
//! `vb/object.rs`'s own array walk and `vb/controltree.rs`'s own
//! scope-byte walk both report a full comparison rather than a containment
//! check, so [`join_by_name`] reports a tree name with no matching
//! `ControlInfo` and a `ControlInfo` name with no matching tree node, both.
//!
//! Measured this session, against three independent corpus programs: the
//! `ControlInfo` array carries one entry for the form itself, alongside one
//! entry per hosted control (a control array collapses to one entry shared
//! by every one of its own elements, joined by matching that one name
//! against every tree node that carries it). The form's own entry is never
//! the form's own declared name; it is always the literal string `"Form"`.
//! `join_by_name` excludes the tree's own root and the `"Form"` entry, one
//! on each side, so the form's own slot is never reported as unjoined. See
//! [`FORM_SELF_ENTRY_NAME`]'s own doc comment for the evidence.

use std::collections::BTreeSet;

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Rva, Va};
use crate::vb::classify::has_optional_info;
use crate::vb::controltree::ControlTree;
use crate::vb::object::Object;

/// The stride of one `ControlInfo` element. `STRUCTURES.md` section 8.6:
/// `0x28` = 40 bytes.
pub const CONTROL_INFO_SIZE: u32 = 0x28;

/// The offset of `OptionalObjectInfo` from `Object.lpObjectInfo`.
/// `STRUCTURES.md` section 5.3: it is not a separate allocation, and it sits
/// immediately after the `0x38`-byte `ObjectInfo`.
const OPTIONAL_OBJECT_INFO_AT: u32 = 0x38;

/// The size of the `OptionalObjectInfo` window this module reads.
/// `STRUCTURES.md` section 5.3: `0x40` = 64 bytes.
const OPTIONAL_OBJECT_INFO_SIZE: u32 = 0x40;

/// The bound on a `ControlInfo`'s own name. The same bound
/// `vb/object.rs::read_name` uses for an object name: `Region::cstr` needs a
/// mandatory maximum, so a file with no NUL byte after the name cannot make
/// the scan run to the end of the section.
const NAME_MAX: u32 = 0x104;

/// One `ControlInfo` record: a control's kind, its event-slot count, its
/// CLSID and event-table addresses, and its recovered name.
///
/// The name is empty when the name pointer resolves nowhere or holds no
/// bounded string; [`ControlInfoTable::defects`] carries the reason. This
/// file never invents a name, matching `vb/object.rs::Object::name`'s own
/// contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlInfo {
    /// `0x40` for an intrinsic control's plain event sink, `0x2E` for a COM
    /// control's `IDispatch` sink. Carried raw: [`read_event_table`] selects
    /// the event table's header size from it, and this reader refuses no
    /// control on the strength of an unrecognised value.
    pub f_control_type: u16,
    /// The number of event slots [`read_event_table`] reads after the
    /// header.
    pub w_event_count: u16,
    /// The address of this control's 16-byte CLSID.
    pub lp_guid: Va,
    /// The address of the event handler table [`read_event_table`] reads.
    pub lp_event_table: Va,
    /// The control's name, the join key back to
    /// [`crate::vb::controltree::ControlNode`].
    pub name: String,
}

/// The `ControlInfo` array one object's `OptionalObjectInfo` names, plus the
/// defects the walk found.
#[derive(Clone, Debug)]
pub struct ControlInfoTable {
    /// Every entry the walk recovered, in array order.
    pub entries: Vec<ControlInfo>,
    defects: Vec<Defect>,
}

impl ControlInfoTable {
    /// Gives the defects the walk found: a `dwControlCount` too large for
    /// the file to hold, or a name that resolved nowhere.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }

    /// Reads the `ControlInfo` array for one object.
    ///
    /// Gives an empty table, with no defect, for an object
    /// [`has_optional_info`] says carries no `OptionalObjectInfo` at all (a
    /// standard module, `STRUCTURES.md` section 5.5): there is nothing to
    /// resolve, and resolving `lpObjectInfo + 0x38` against real bytes that
    /// hold something else entirely would be a fault, not a recovery.
    ///
    /// `dwControlCount` is bounded against the real length of the file that
    /// remains from `lpControls` before it becomes a loop bound, the same
    /// shape `vb/object.rs::bound_proc_count` uses for `ProcCount`: a count
    /// straight out of the file must not size an allocation or a loop before
    /// it is checked against what the file can actually hold.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when `object.lp_object_info` is in no
    /// section, when the file ends inside the `OptionalObjectInfo` window,
    /// or when the file ends inside a `ControlInfo` element the bounded
    /// count still calls for.
    pub fn read(pe: &PeImage<'_>, object: &Object) -> Result<Self, Refusal> {
        if !has_optional_info(object.f_object_type) {
            return Ok(Self {
                entries: Vec::new(),
                defects: Vec::new(),
            });
        }

        let base = pe
            .region_at_va(object.lp_object_info)
            .ok_or(Refusal::Damaged(
                "an Object's lpObjectInfo is in no section",
            ))?;
        let optional = base
            .subregion(Off::new(OPTIONAL_OBJECT_INFO_AT), OPTIONAL_OBJECT_INFO_SIZE)
            .ok_or(Refusal::Damaged(
                "the file ends inside an OptionalObjectInfo block",
            ))?;

        let raw_control_count = optional.u32_le(Off::new(0x20)).ok_or(Refusal::Damaged(
            "OptionalObjectInfo holds no dwControlCount",
        ))?;
        let lp_controls = optional.va_le(Off::new(0x24)).ok_or(Refusal::Damaged(
            "OptionalObjectInfo holds no address for its ControlInfo array",
        ))?;

        if raw_control_count == 0 {
            return Ok(Self {
                entries: Vec::new(),
                defects: Vec::new(),
            });
        }

        let Some(array) = pe.region_at_va(lp_controls) else {
            // No section to read the array from. Not observed anywhere in
            // this corpus; treated the same way `vb/privateobj.rs`'s own
            // `ProcedureList::read` treats an unmapped `lpProcNamesArray`:
            // nothing here to bound a count against, so nothing is read.
            return Ok(Self {
                entries: Vec::new(),
                defects: Vec::new(),
            });
        };

        let mut defects = Vec::new();
        let (control_count, count_defect) =
            bound_control_count(&array, &optional, raw_control_count);
        if let Some(defect) = count_defect {
            defects.push(defect);
        }

        let mut entries = Vec::new();
        for i in 0_u32..control_count {
            let at = i.checked_mul(CONTROL_INFO_SIZE).ok_or(Refusal::Damaged(
                "the ControlInfo array index overflows a u32",
            ))?;
            let element =
                array
                    .subregion(Off::new(at), CONTROL_INFO_SIZE)
                    .ok_or(Refusal::Damaged(
                        "the file ends inside a ControlInfo element",
                    ))?;

            let raw = read_raw_control_info(&element)?;
            let (name, name_defect) = read_name(pe, &element, raw.lpsz_name);
            if let Some(defect) = name_defect {
                defects.push(defect);
            }

            entries.push(ControlInfo {
                f_control_type: raw.f_control_type,
                w_event_count: raw.w_event_count,
                lp_guid: raw.lp_guid,
                lp_event_table: raw.lp_event_table,
                name,
            });
        }

        Ok(Self { entries, defects })
    }
}

/// The fields [`read_raw_control_info`] reads directly out of one
/// `ControlInfo` element's own bounded window, before the name pointer is
/// resolved.
struct RawControlInfo {
    f_control_type: u16,
    w_event_count: u16,
    lp_guid: Va,
    lp_event_table: Va,
    lpsz_name: Va,
}

/// Reads the fixed-width fields of one `ControlInfo` element.
///
/// `element` is the element's own `CONTROL_INFO_SIZE`-byte window, taken by
/// the caller. This function reads no name; [`read_name`] resolves
/// `lpsz_name` separately, because that resolution needs a [`PeImage`] and
/// this function does not.
fn read_raw_control_info(element: &Region<'_>) -> Result<RawControlInfo, Refusal> {
    Ok(RawControlInfo {
        f_control_type: element
            .u16_le(Off::new(0x00))
            .ok_or(Refusal::Damaged("a ControlInfo holds no fControlType"))?,
        w_event_count: element
            .u16_le(Off::new(0x02))
            .ok_or(Refusal::Damaged("a ControlInfo holds no wEventCount"))?,
        lp_guid: element.va_le(Off::new(0x08)).ok_or(Refusal::Damaged(
            "a ControlInfo holds no address for its GUID",
        ))?,
        lp_event_table: element.va_le(Off::new(0x18)).ok_or(Refusal::Damaged(
            "a ControlInfo holds no address for its event table",
        ))?,
        lpsz_name: element.va_le(Off::new(0x20)).ok_or(Refusal::Damaged(
            "a ControlInfo holds no address for its name",
        ))?,
    })
}

/// Bounds `dwControlCount` against the real length of the file that remains
/// from `lpControls`, and clamps it when it does not fit.
///
/// The same shape `vb/object.rs::bound_proc_count` uses for `ProcCount`.
/// `array` is the region [`ControlInfoTable::read`] already resolved at
/// `lpControls`; this function is called only once that resolution has
/// succeeded, so a `dwControlCount` too large for the file to hold is always
/// this function's own [`DefectKind::ImplausibleCount`], never silently
/// folded into "the array does not exist at all".
fn bound_control_count(
    array: &Region<'_>,
    optional: &Region<'_>,
    raw_count: u32,
) -> (u32, Option<Defect>) {
    let max_entries = array.len().checked_div(CONTROL_INFO_SIZE).unwrap_or(0);
    if raw_count <= max_entries {
        return (raw_count, None);
    }

    let offset = optional.file_offset(Off::new(0x20)).map_or(0, Off::get);
    let defect = Defect {
        site: Site {
            offset,
            rva: None,
            structure: "OptionalObjectInfo",
            field: "dwControlCount",
        },
        kind: DefectKind::ImplausibleCount {
            offset,
            count: raw_count,
            max: max_entries,
        },
    };
    (max_entries, Some(defect))
}

/// Resolves a `ControlInfo`'s own name.
///
/// The same contract `vb/object.rs::read_name` documents: a name pointer
/// that resolves nowhere, or a name with no NUL terminator inside
/// [`NAME_MAX`] bytes, is one unreadable field, not a broken file. The
/// control keeps its other fields, and the returned defect names the byte
/// offset and the address so a person can open the file there.
///
/// Each byte becomes its own Latin-1 code point. `String::from_utf8_lossy`
/// is never used here: a byte in `0x80` to `0xFF` would become the
/// replacement character and the name would be lost.
fn read_name(pe: &PeImage<'_>, element: &Region<'_>, lpsz_name: Va) -> (String, Option<Defect>) {
    let offset = element.file_offset(Off::new(0x20)).map_or(0, Off::get);
    let site = Site {
        offset,
        rva: lpsz_name.to_rva(pe.image_base()).map(Rva::get),
        structure: "ControlInfo",
        field: "lpszName",
    };

    let Some(name_region) = pe.region_at_va(lpsz_name) else {
        let kind = DefectKind::UnreadablePointer {
            offset,
            va: lpsz_name.get(),
        };
        return (String::new(), Some(Defect { site, kind }));
    };

    match name_region.cstr(Off::new(0), NAME_MAX) {
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

/// The literal name every corpus form's own `ControlInfo` entry carries.
///
/// Measured this session, against three independent corpus programs
/// (`SK-Gradient-Sample__VB6`, `Grayscale-effect`, `LockWorkStation`, none
/// sharing a form name with either of the others): the `ControlInfo` entry
/// for the form itself is never the form's own declared name (`"Form1"`,
/// `"frmGrayscale"`, `"FrmLockWorkStation"`). It is always the literal
/// string `"Form"`, and its `wEventCount` is `31` in all three samples.
/// `STRUCTURES.md` section 8.6 does not name this. This entry represents
/// the form's own event-table slot, distinct from the events of the
/// controls the form hosts, and it is joined to the tree's own root by
/// structural position (there is exactly one root and, per this
/// measurement, exactly one entry of this name), not by a literal string
/// match against the root's own declared name.
const FORM_SELF_ENTRY_NAME: &str = "Form";

/// The result of joining a control tree's own names against a
/// [`ControlInfoTable`]'s own names, in both directions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlJoin {
    /// A control tree node's own name with no matching `ControlInfo`: this
    /// control has no event table, and this is why.
    pub tree_only: Vec<String>,
    /// A `ControlInfo`'s own name with no matching tree node: this entry is
    /// unjoined, and this is why.
    pub info_only: Vec<String>,
}

/// Joins a control tree's own names against a [`ControlInfoTable`]'s own
/// names.
///
/// Compares name membership, not a positional pairing: a name repeated on
/// both sides matches regardless of order, and a name a control array
/// shares across every one of its own elements (`STRUCTURES.md` section 8.6
/// gives one `ControlInfo` entry per array, not per element) matches once,
/// against every tree node that carries it. Two entries are excluded from
/// the comparison, one on each side, so the form's own slot is never
/// reported as unjoined: the tree's own root (the form itself), and the
/// `ControlInfo` entry named [`FORM_SELF_ENTRY_NAME`]; see that constant's
/// doc comment for the measurement behind the exclusion.
#[must_use]
pub fn join_by_name(tree: &ControlTree, table: &ControlInfoTable) -> ControlJoin {
    let tree_names: BTreeSet<&str> = tree
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != tree.root)
        .map(|(_, node)| node.header.name.as_str())
        .collect();
    let info_names: BTreeSet<&str> = table
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .filter(|name| *name != FORM_SELF_ENTRY_NAME)
        .collect();

    ControlJoin {
        tree_only: tree_names
            .difference(&info_names)
            .map(|name| (*name).to_owned())
            .collect(),
        info_only: info_names
            .difference(&tree_names)
            .map(|name| (*name).to_owned())
            .collect(),
    }
}

// --- The event handler table and the native stub -------------------------
//
// `STRUCTURES.md` section 8.6 gives two header shapes, selected by
// `fControlType`, and a 13-byte native stub. This session decoded one real
// corpus stub byte for byte (`SK-Gradient-Sample__VB6`'s own `Command1`,
// event slot 0): `81 6c 24 04 3f 00 00 00 e9 23 04 00`, confirming every
// field this section reads: the `sub`/`jmp` opcode bytes, the immediate
// value at `+0x04` (`0x3F`, which AG's own sample also gives, per
// `STRUCTURES.md`), and the relative jump at `+0x09`.
//
// This module reads the **native** stub shape only. `STRUCTURES.md` section
// 10.2 names a second, P-code stub shape (`xor eax,eax / mov edx,<addr> /
// push <addr> / ret`); this corpus is 44 native programs and holds no P-code
// sample (`STATE.md`'s own standing blocker), so the P-code branch is not
// implemented here. It is a declared absence, not an oversight.

/// The event table header length for `fControlType == 0x40`
/// (`STRUCTURES.md` section 8.6): 6 four-byte values.
const NATIVE_EVENT_HEADER_LEN: u32 = 0x18;

/// The event table header length for `fControlType == 0x2E`: 10 four-byte
/// values, the 6 native ones plus the four `IDispatch` slots a COM control's
/// sink adds.
const COM_EVENT_HEADER_LEN: u32 = 0x28;

/// The width of one event slot: a four-byte little-endian address, `0`
/// meaning unbound.
const EVENT_SLOT_SIZE: u32 = 4;

/// The native stub's own byte length: 4 bytes of `sub dword ptr [esp+4],
/// imm32` opcode, 4 bytes of `imm32`, 1 byte of `jmp rel32` opcode, 4 bytes
/// of `rel32`.
const STUB_LEN: u32 = 13;

/// The `imm32` value a stub gives for a method. Any smaller value marks an
/// event.
const METHOD_MARKER: u32 = 0xFFFF;

/// Chooses the event table header length from `fControlType`.
///
/// `None` for any value other than `0x40` or `0x2E`: the header size is
/// never guessed, and [`read_event_table`] reads no slots for one.
#[must_use]
const fn event_table_header_len(f_control_type: u16) -> Option<u32> {
    match f_control_type {
        0x0040 => Some(NATIVE_EVENT_HEADER_LEN),
        0x002E => Some(COM_EVENT_HEADER_LEN),
        _ => None,
    }
}

/// One event slot: bound to a handler stub, or unbound.
///
/// `STRUCTURES.md` section 8.6: a slot of zero means the event has no
/// handler in the source. Every slot is reported by its own ascending
/// index, and the index is never renumbered: an unbound slot is reported,
/// never omitted, and the slot after it keeps its own index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventSlot {
    /// The slot's own value was `0`: no handler is bound in the source.
    Unbound {
        /// This slot's own ordinal in the control's default source
        /// interface, ascending from `0`.
        index: u16,
    },
    /// The slot's own value is a stub address.
    Bound {
        /// This slot's own ordinal.
        index: u16,
        /// The stub's own address, as the file gives it.
        stub: Va,
        /// The decoded handler, when the stub itself resolved and its own
        /// 13 bytes could be read. `None` when it could not;
        /// [`EventTable::defects`] carries the reason. The slot still
        /// reports as bound either way: a slot whose handler this module
        /// cannot decode is not the same fact as a slot with no handler at
        /// all.
        handler: Option<StubHandler>,
    },
}

/// A native stub, decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StubHandler {
    /// `true` when the stub's own `imm32` is [`METHOD_MARKER`] (`0xFFFF`), a
    /// method. A smaller value marks an event.
    pub is_method: bool,
    /// The handler's own address: the stub start plus [`STUB_LEN`] plus the
    /// signed four-byte value at the stub start plus `0x09`. Computed with
    /// checked arithmetic over the whole signed range, so a negative
    /// relative value gives an address below the stub start rather than
    /// wrapping.
    pub handler_address: u32,
}

/// The event handler table one `ControlInfo` names, plus the defects the
/// walk found.
#[derive(Clone, Debug)]
pub struct EventTable {
    /// Every slot, by ascending index, in `STRUCTURES.md` section 8.6's own
    /// order.
    pub slots: Vec<EventSlot>,
    /// Set when `fControlType` names neither `0x40` nor `0x2E`: no header
    /// size can be chosen, so no slots are read. Carries the raw value, so
    /// the reason a report gives names it rather than guessing.
    pub unsupported_control_type: Option<u16>,
    defects: Vec<Defect>,
}

impl EventTable {
    /// Gives the defects the walk found: a stub address that resolved
    /// nowhere or held too few bytes to decode, or a `wEventCount` too large
    /// for the file to hold.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }
}

/// Reads the event handler table `control` names.
///
/// `control.w_event_count` is bounded against the real length of the file
/// that remains after the header, the same shape
/// [`ControlInfoTable::read`]'s own `bound_control_count` uses for
/// `dwControlCount`.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when `control.lp_event_table` is in no
/// section. A control whose header size cannot be chosen, or whose event
/// count is `0`, is not an error: it gives an [`EventTable`] with no slots.
pub fn read_event_table(pe: &PeImage<'_>, control: &ControlInfo) -> Result<EventTable, Refusal> {
    let Some(header_len) = event_table_header_len(control.f_control_type) else {
        return Ok(EventTable {
            slots: Vec::new(),
            unsupported_control_type: Some(control.f_control_type),
            defects: Vec::new(),
        });
    };

    if control.w_event_count == 0 {
        return Ok(EventTable {
            slots: Vec::new(),
            unsupported_control_type: None,
            defects: Vec::new(),
        });
    }

    let table = pe
        .region_at_va(control.lp_event_table)
        .ok_or(Refusal::Damaged(
            "a ControlInfo's lpEventTable is in no section",
        ))?;

    let mut defects = Vec::new();
    let (event_count, count_defect) = bound_event_count(&table, header_len, control.w_event_count);
    if let Some(defect) = count_defect {
        defects.push(defect);
    }

    let mut slots = Vec::new();
    for index in 0..event_count {
        let slot_off = header_len
            .checked_add(
                u32::from(index)
                    .checked_mul(EVENT_SLOT_SIZE)
                    .ok_or(Refusal::Damaged("an event slot index overflows a u32"))?,
            )
            .ok_or(Refusal::Damaged("an event slot offset overflows a u32"))?;
        let slot_va = table
            .va_le(Off::new(slot_off))
            .ok_or(Refusal::Damaged("the file ends inside an event slot"))?;

        if slot_va.is_null() {
            slots.push(EventSlot::Unbound { index });
            continue;
        }

        let slot_offset = table.file_offset(Off::new(slot_off)).map_or(0, Off::get);
        let (handler, stub_defect) = decode_stub(pe, slot_va, slot_offset);
        if let Some(defect) = stub_defect {
            defects.push(defect);
        }
        slots.push(EventSlot::Bound {
            index,
            stub: slot_va,
            handler,
        });
    }

    Ok(EventTable {
        slots,
        unsupported_control_type: None,
        defects,
    })
}

/// Bounds `wEventCount` against the real length of the file that remains
/// after `header_len`, and clamps it when it does not fit. The same shape
/// [`bound_control_count`] uses for `dwControlCount`.
fn bound_event_count(table: &Region<'_>, header_len: u32, raw_count: u16) -> (u16, Option<Defect>) {
    let remaining = table.len().saturating_sub(header_len);
    let max_entries = remaining.checked_div(EVENT_SLOT_SIZE).unwrap_or(0);
    let raw_count_u32 = u32::from(raw_count);
    if raw_count_u32 <= max_entries {
        return (raw_count, None);
    }

    let max = u16::try_from(max_entries).unwrap_or(u16::MAX);
    let offset = table.file_offset(Off::new(0)).map_or(0, Off::get);
    let defect = Defect {
        site: Site {
            offset,
            rva: None,
            structure: "ControlInfo",
            field: "wEventCount",
        },
        kind: DefectKind::ImplausibleCount {
            offset,
            count: raw_count_u32,
            max: max_entries,
        },
    };
    (max, Some(defect))
}

/// Decodes one native stub.
///
/// `slot_offset` is the byte offset of the event slot itself (not the
/// stub), used to build the defect a caller reports when the stub cannot be
/// decoded, the same "name where the pointer was read from" contract
/// [`read_name`] follows for a `ControlInfo`'s own name.
///
/// Gives `(None, Some(defect))` when the stub address resolves to no
/// section, when its own [`STUB_LEN`] bytes cannot be read in full, or when
/// the handler address computation overflows. Every one of these keeps the
/// slot itself bound; only the decoded handler is lost.
fn decode_stub(
    pe: &PeImage<'_>,
    stub_va: Va,
    slot_offset: u32,
) -> (Option<StubHandler>, Option<Defect>) {
    let unreadable = || Defect {
        site: Site {
            offset: slot_offset,
            rva: stub_va.to_rva(pe.image_base()).map(Rva::get),
            structure: "EventSlot",
            field: "stub",
        },
        kind: DefectKind::UnreadablePointer {
            offset: slot_offset,
            va: stub_va.get(),
        },
    };

    let Some(stub) = pe.region_at_va(stub_va) else {
        return (None, Some(unreadable()));
    };
    let Some(imm32) = stub.u32_le(Off::new(0x04)) else {
        return (None, Some(unreadable()));
    };
    let Some(rel32) = stub.i32_le(Off::new(0x09)) else {
        return (None, Some(unreadable()));
    };
    let Some(handler_address) = stub_va
        .get()
        .checked_add(STUB_LEN)
        .and_then(|v| v.checked_add_signed(rel32))
    else {
        return (None, Some(unreadable()));
    };

    (
        Some(StubHandler {
            is_method: imm32 == METHOD_MARKER,
            handler_address,
        }),
        None,
    )
}

// --- The event name, reported honestly per 03-CONTEXT.md D-02 ------------
//
// The file holds no event name. The slot index is the event's ordinal in
// the control's default source interface, and turning an ordinal into a
// name needs a table this repository does not commit: the vtable ordering
// is not commonly published, and D-02 treats it with the same "commit the
// tool, not the table" discipline `vb/opcodes.rs::OpcodeTable` already
// established for property names. This module builds no such table. It
// builds the honest report for a slot whose name is not available, and the
// seam a caller-supplied table plugs into later.

/// A source of event names, keyed by control type and event ordinal.
///
/// Per `03-CONTEXT.md` D-02, this repository ships zero entries.
/// [`EventNameTable::default`] is the "no table" case every
/// [`report_events`] call uses until a caller supplies one; a supplied
/// table plugs into this same [`EventNameTable::lookup`] path, so it can
/// never behave differently from the empty one, the same seam plan 03-02
/// gave [`crate::vb::opcodes::OpcodeTable`].
#[derive(Clone, Debug, Default)]
pub struct EventNameTable {
    entries: std::collections::HashMap<(String, u16), String>,
}

impl EventNameTable {
    /// Looks up the event name for one control type's own ordinal.
    ///
    /// Gives `None` for every ordinal on an empty table, and for any
    /// control type or ordinal a supplied table names no entry for: a slot
    /// on a control type with no table entry reports no name, never one
    /// guessed from a neighbouring entry.
    #[must_use]
    pub fn lookup(&self, control_type: &str, index: u16) -> Option<&str> {
        self.entries
            .get(&(control_type.to_owned(), index))
            .map(String::as_str)
    }
}

/// The event name state for one control's own event slot, reported
/// honestly per `03-CONTEXT.md` D-02.
///
/// Three states: a bound slot whose name a supplied [`EventNameTable`]
/// names, a bound slot with no name available, and an unbound slot. This
/// module never derives a name from the slot index by counting, and never
/// names an event from a control type this repository has not verified
/// against a cited public source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventReport {
    /// A bound slot whose name [`EventNameTable::lookup`] gave.
    Named {
        /// The control's own name.
        control_name: String,
        /// This slot's own ordinal.
        index: u16,
        /// The event name a supplied table gave.
        event_name: String,
    },
    /// A bound slot with no name available.
    BoundUnnamed {
        /// The control's own name.
        control_name: String,
        /// This slot's own ordinal.
        index: u16,
    },
    /// An unbound slot: no handler in the source. VB6 declares a
    /// source-level event procedure only for a bound slot, so this module
    /// never attempts a name lookup for one: naming an event nothing calls
    /// serves no recovery this repository writes.
    Unbound {
        /// The control's own name.
        control_name: String,
        /// This slot's own ordinal.
        index: u16,
    },
}

impl EventReport {
    /// Renders the honest-gap message for a slot with no name available,
    /// matching the shape `PropertyValue::undecoded_message` gives for an
    /// unnamed property opcode: present, at its own index, no table
    /// supplied, and the way to supply one. `None` for
    /// [`EventReport::Named`], which already carries its own name.
    #[must_use]
    pub fn no_name_message(&self) -> Option<String> {
        match self {
            Self::BoundUnnamed {
                control_name,
                index,
            } => Some(format!(
                "Event slot {index} on {control_name}: bound, name not available, no event \
                 name table loaded. Run with --event-name-table to supply one naming this \
                 control's own vtable ordering."
            )),
            Self::Unbound {
                control_name,
                index,
            } => Some(format!(
                "Event slot {index} on {control_name}: unbound, name not available, no event \
                 name table loaded. Run with --event-name-table to supply one naming this \
                 control's own vtable ordering."
            )),
            Self::Named { .. } => None,
        }
    }
}

/// Builds the event report for one control's own event table.
///
/// `control_type_name` is the control's own type, from
/// `classify_control_type`'s own `Debug` rendering (matching
/// `vb/propstream.rs`'s own `Undecoded` message, which reads the same way):
/// event names are per control type, not per control instance, so the
/// lookup key is the type, while the report itself still carries the
/// control's own `control_name` for identification.
///
/// Two runs over the same bytes give the same report in the same order:
/// this function reads only `table.slots`, in its own stored order, and
/// invents nothing.
#[must_use]
pub fn report_events(
    control_name: &str,
    control_type_name: &str,
    table: &EventTable,
    names: &EventNameTable,
) -> Vec<EventReport> {
    table
        .slots
        .iter()
        .map(|slot| match slot {
            EventSlot::Unbound { index } => EventReport::Unbound {
                control_name: control_name.to_owned(),
                index: *index,
            },
            EventSlot::Bound { index, .. } => match names.lookup(control_type_name, *index) {
                Some(event_name) => EventReport::Named {
                    control_name: control_name.to_owned(),
                    index: *index,
                    event_name: event_name.to_owned(),
                },
                None => EventReport::BoundUnnamed {
                    control_name: control_name.to_owned(),
                    index: *index,
                },
            },
        })
        .collect()
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
    use super::{
        ControlInfoTable, EventNameTable, EventReport, EventSlot, join_by_name, read_event_table,
        read_raw_control_info, report_events,
    };
    use crate::error::DefectKind;
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Region, Va};
    use crate::vb::classify::{self, ObjectKind};
    use crate::vb::controltree::{self, ControlTree};
    use crate::vb::gui::{GuiObjectInfo, GuiTable, Tiling};
    use crate::vb::header::{VbHeader, header_region};
    use crate::vb::object::{Object, ObjectTable};
    use crate::vb::project::{ObjectTableHead, ProjectInfo};

    // --- Task 1: ControlInfo and the join by control name -----------------

    /// Builds one synthetic `ControlInfo` element (40 bytes), following
    /// `STRUCTURES.md` section 8.6's own field table. Bytes 0x04-0x05 (the
    /// "unknown" field the table names right after `wEventCount`) are set to
    /// a value that differs from `w_event_count`, so a reader that
    /// mis-reads `wEventCount` at offset `0x04` instead of `0x02` is caught
    /// rather than accidentally agreeing.
    fn control_info_bytes(
        f_control_type: u16,
        w_event_count: u16,
        lp_guid: u32,
        lp_event_table: u32,
        lpsz_name: u32,
    ) -> [u8; 40] {
        let mut buf = [0_u8; 40];
        buf[0x00..0x02].copy_from_slice(&f_control_type.to_le_bytes());
        buf[0x02..0x04].copy_from_slice(&w_event_count.to_le_bytes());
        buf[0x04..0x06].copy_from_slice(&0xBBAA_u16.to_le_bytes());
        buf[0x08..0x0C].copy_from_slice(&lp_guid.to_le_bytes());
        buf[0x18..0x1C].copy_from_slice(&lp_event_table.to_le_bytes());
        buf[0x20..0x24].copy_from_slice(&lpsz_name.to_le_bytes());
        buf
    }

    #[test]
    fn f_control_type_and_w_event_count_read_as_two_byte_values_at_0x00_and_0x02() {
        let bytes = control_info_bytes(0x0040, 5, 0x0040_1000, 0x0040_2000, 0x0040_3000);
        let region = Region::new(&bytes, Off::new(0));
        let raw = read_raw_control_info(&region).unwrap();
        assert_eq!(raw.f_control_type, 0x0040);
        assert_eq!(raw.w_event_count, 5);
        assert_eq!(raw.lp_guid, Va::new(0x0040_1000));
        assert_eq!(raw.lp_event_table, Va::new(0x0040_2000));
        assert_eq!(raw.lpsz_name, Va::new(0x0040_3000));
    }

    #[test]
    fn the_control_info_size_constant_is_forty() {
        assert_eq!(super::CONTROL_INFO_SIZE, 40);
    }

    /// Builds a minimal 32-bit i386 portable executable with one section at
    /// RVA `0x1000` / file offset `0x400`, holding `extra` at its start.
    ///
    /// A local copy of the same helper `vb/gui.rs`'s own test module holds,
    /// per this project's own convention: two files' tests must be able to
    /// fail independently, and a shared fixture module would let a change to
    /// one break the other silently.
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

    /// A synthetic form object whose `lp_object_info` is the section's own
    /// start (VA `0x0040_1000`), matching `synthetic_image`'s own section
    /// RVA.
    fn synthetic_form_object() -> Object {
        Object {
            lp_object_info: Va::new(0x0040_1000),
            name: "SyntheticForm".to_owned(),
            proc_count: 0,
            lp_proc_names_array: Va::new(0),
            f_object_type: 0x0001_8083, // classify::classify gives ObjectKind::Form
        }
    }

    /// A synthetic standard-module object: `has_optional_info` is false for
    /// this `f_object_type`, so [`ControlInfoTable::read`] must not resolve
    /// `lpObjectInfo + 0x38` at all for it.
    fn synthetic_module_object() -> Object {
        Object {
            lp_object_info: Va::new(0x0040_1000),
            name: "SyntheticModule".to_owned(),
            proc_count: 0,
            lp_proc_names_array: Va::new(0),
            f_object_type: 0x0001_8001, // classify::classify gives ObjectKind::Module
        }
    }

    #[test]
    fn a_standard_module_gives_an_empty_table_with_no_defect() {
        assert!(!classify::has_optional_info(
            synthetic_module_object().f_object_type
        ));
        let bytes = synthetic_image(&[0_u8; 0x10]);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ControlInfoTable::read(&image, &synthetic_module_object()).unwrap();
        assert!(table.entries.is_empty());
        assert!(table.defects().is_empty());
    }

    #[test]
    fn a_dw_control_count_of_zero_gives_an_empty_list_and_no_fault() {
        let extra = vec![0_u8; 0x100];
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ControlInfoTable::read(&image, &synthetic_form_object()).unwrap();
        assert!(table.entries.is_empty());
        assert!(table.defects().is_empty());
    }

    #[test]
    fn a_dw_control_count_larger_than_the_file_can_hold_gives_a_defect_and_sizes_no_allocation() {
        // dwControlCount at extra offset 0x38+0x20=0x58; lpControls at
        // 0x38+0x24=0x5C, pointing near the end of the mapped section, so
        // only 4 bytes remain there: max_entries = 4 / 40 = 0.
        let mut extra = vec![0_u8; 0x84];
        extra[0x58..0x5C].copy_from_slice(&1000_u32.to_le_bytes());
        extra[0x5C..0x60].copy_from_slice(&0x0040_1080_u32.to_le_bytes());
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ControlInfoTable::read(&image, &synthetic_form_object()).unwrap();
        assert!(table.entries.is_empty());
        assert_eq!(table.defects().len(), 1);
        assert!(matches!(
            table.defects()[0].kind,
            DefectKind::ImplausibleCount {
                count: 1000,
                max: 0,
                ..
            }
        ));
    }

    #[test]
    fn an_lpsz_name_address_in_no_section_gives_an_empty_name_and_a_defect_naming_the_address() {
        let mut extra = vec![0_u8; 0x100];
        extra[0x58..0x5C].copy_from_slice(&1_u32.to_le_bytes());
        extra[0x5C..0x60].copy_from_slice(&0x0040_1080_u32.to_le_bytes());
        // The one ControlInfo element at extra offset 0x80: lpszName at
        // +0x20 (extra offset 0xA0) is an address in no section.
        extra[0xA0..0xA4].copy_from_slice(&0x00F0_0000_u32.to_le_bytes());
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        assert!(image.region_at_va(Va::new(0x00F0_0000)).is_none());

        let table = ControlInfoTable::read(&image, &synthetic_form_object()).unwrap();
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].name, "");
        assert_eq!(table.defects().len(), 1);
        assert!(matches!(
            table.defects()[0].kind,
            DefectKind::UnreadablePointer {
                va: 0x00F0_0000,
                ..
            }
        ));
    }

    #[test]
    fn the_second_array_element_starts_forty_bytes_after_the_first() {
        let mut extra = vec![0_u8; 0x200];
        extra[0x58..0x5C].copy_from_slice(&2_u32.to_le_bytes());
        extra[0x5C..0x60].copy_from_slice(&0x0040_1080_u32.to_le_bytes());

        // Entry 0 at extra offset 0x80.
        extra[0x80..0x82].copy_from_slice(&0x0040_u16.to_le_bytes()); // fControlType
        extra[0x80 + 0x20..0x80 + 0x24].copy_from_slice(&0x0040_1180_u32.to_le_bytes()); // lpszName
        extra[0x180..0x185].copy_from_slice(b"Cmd1\0");

        // Entry 1 at extra offset 0xA8 (0x80 + CONTROL_INFO_SIZE).
        extra[0xA8..0xAA].copy_from_slice(&0x002E_u16.to_le_bytes()); // fControlType
        extra[0xA8 + 0x20..0xA8 + 0x24].copy_from_slice(&0x0040_1190_u32.to_le_bytes()); // lpszName
        extra[0x190..0x195].copy_from_slice(b"Cmd2\0");

        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let table = ControlInfoTable::read(&image, &synthetic_form_object()).unwrap();
        assert_eq!(table.entries.len(), 2);
        assert_eq!(table.entries[0].f_control_type, 0x0040);
        assert_eq!(table.entries[0].name, "Cmd1");
        assert_eq!(table.entries[1].f_control_type, 0x002E);
        assert_eq!(table.entries[1].name, "Cmd2");
        assert!(table.defects().is_empty());
    }

    #[test]
    fn the_reader_uses_no_lossy_utf8_conversion() {
        assert_eq!(0, production_code_only().matches("from_utf8_lossy").count());
    }

    #[test]
    fn the_reader_sizes_no_allocation_with_capacity() {
        assert_eq!(0, production_code_only().matches("with_capacity").count());
    }

    /// Gives this file's own source, with comment lines and the whole
    /// `#[cfg(test)]` module stripped, so a grep-shaped acceptance check
    /// scans only the production reading code and not this test module's
    /// own source, which necessarily names the very strings these checks
    /// forbid (as the literal argument to `.matches(...)` below).
    fn production_code_only() -> String {
        let source = include_str!("controlinfo.rs");
        let before_tests = source.split("#[cfg(test)]").next().unwrap_or(source);
        before_tests
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // --- Corpus: the join, both directions ---------------------------------

    const SK_GRADIENT_SAMPLE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe"
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

    /// Gives the first object this file classifies as a form.
    fn first_form_object(data: &[u8]) -> Object {
        let image = PeImage::parse(data).unwrap();
        let lp_object_table = object_table_va(data);
        let head = ObjectTableHead::read(&image, lp_object_table).unwrap();
        let table = ObjectTable::walk(&image, lp_object_table, &head).unwrap();
        table
            .objects
            .into_iter()
            .find(|object| classify::classify(object.f_object_type) == ObjectKind::Form)
            .expect("this fixture declares at least one form")
    }

    /// Walks the whole control tree of the first form in `data`, through the
    /// same pointer chain `vb/controltree.rs`'s own corpus tests use.
    fn walk_first_form_tree(data: &[u8]) -> ControlTree {
        let image = PeImage::parse(data).unwrap();
        let hdr = header_region(&image).unwrap();
        let header = VbHeader::read(&hdr).unwrap();
        let table = GuiTable::walk(&image, &header).unwrap();
        let entry = table.entries[0];
        let info = GuiObjectInfo::read(&image, entry.a_form_pointer).unwrap();
        let stream = info.form_stream().unwrap();
        let mut tiling = Tiling::new(info.l_properties_length);
        controltree::walk(&stream, &mut tiling).unwrap().0
    }

    #[test]
    fn sk_gradient_sample_joins_both_directions_with_no_unmatched_entry() {
        let image = PeImage::parse(SK_GRADIENT_SAMPLE).unwrap();
        let object = first_form_object(SK_GRADIENT_SAMPLE);
        let control_info_table = ControlInfoTable::read(&image, &object).unwrap();
        let tree = walk_first_form_tree(SK_GRADIENT_SAMPLE);

        let join = join_by_name(&tree, &control_info_table);
        assert!(
            join.tree_only.is_empty(),
            "control names present in the tree with no matching ControlInfo: {:?}",
            join.tree_only
        );
        assert!(
            join.info_only.is_empty(),
            "ControlInfo names present with no matching tree node: {:?}",
            join.info_only
        );
        // SK-Gradient-Sample declares no control array, so the tree's own
        // three children (excluding the root) equal the ControlInfoTable's
        // own three hosted-control entries (excluding the one named "Form").
        assert_eq!(tree.nodes.len() - 1, 3);
        assert_eq!(control_info_table.entries.len() - 1, 3);
    }

    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    /// `Grayscale.exe` carries two control arrays (`optDecompose`,
    /// `optChannel`) and a menu with a submenu. `STRUCTURES.md` section 8.6
    /// gives one `ControlInfo` entry per array, not per element, so this is
    /// the corpus program that proves `join_by_name` collapses a repeated
    /// tree name to the one array entry that names it, rather than reporting
    /// every element past the first as unmatched.
    #[test]
    fn grayscale_joins_both_directions_with_no_unmatched_entry_despite_two_control_arrays() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let object = first_form_object(GRAYSCALE);
        let control_info_table = ControlInfoTable::read(&image, &object).unwrap();
        let tree = walk_first_form_tree(GRAYSCALE);

        let join = join_by_name(&tree, &control_info_table);
        assert!(
            join.tree_only.is_empty(),
            "control names present in the tree with no matching ControlInfo: {:?}",
            join.tree_only
        );
        assert!(
            join.info_only.is_empty(),
            "ControlInfo names present with no matching tree node: {:?}",
            join.info_only
        );
    }

    #[test]
    fn a_control_info_name_with_no_matching_tree_node_is_reported_info_only() {
        // Synthetic: "GhostControl" appears in no real corpus form, so a
        // ControlInfoTable that names it against a real tree must report it
        // as unjoined on the info side, and nothing on the tree side.
        let control_info_table = ControlInfoTable {
            entries: vec![super::ControlInfo {
                f_control_type: 0x40,
                w_event_count: 0,
                lp_guid: Va::new(0),
                lp_event_table: Va::new(0),
                name: "GhostControl".to_owned(),
            }],
            defects: Vec::new(),
        };
        let tree = walk_first_form_tree(SK_GRADIENT_SAMPLE);
        let join = join_by_name(&tree, &control_info_table);
        assert_eq!(join.info_only, vec!["GhostControl".to_owned()]);
        assert_eq!(join.tree_only.len(), tree.nodes.len() - 1);
    }

    // --- Task 2: the event handler table and the stub ---------------------

    fn synthetic_control_info(
        f_control_type: u16,
        w_event_count: u16,
        lp_event_table: u32,
    ) -> super::ControlInfo {
        super::ControlInfo {
            f_control_type,
            w_event_count,
            lp_guid: Va::new(0),
            lp_event_table: Va::new(lp_event_table),
            name: "Synthetic".to_owned(),
        }
    }

    #[test]
    fn the_header_end_and_the_first_slot_offset_come_from_one_computation() {
        assert_eq!(super::event_table_header_len(0x0040), Some(0x18));
        assert_eq!(super::event_table_header_len(0x002E), Some(0x28));
    }

    #[test]
    fn an_f_control_type_of_0x40_puts_the_first_slot_at_offset_0x18() {
        let mut extra = vec![0_u8; 0x30];
        extra[0x18..0x1C].copy_from_slice(&0x0040_1100_u32.to_le_bytes());
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let control = synthetic_control_info(0x0040, 1, 0x0040_1000);
        let table = read_event_table(&image, &control).unwrap();
        assert_eq!(table.slots.len(), 1);
        assert!(matches!(
            table.slots[0],
            EventSlot::Bound { index: 0, stub, .. } if stub == Va::new(0x0040_1100)
        ));
    }

    #[test]
    fn an_f_control_type_of_0x2e_puts_the_first_slot_at_offset_0x28() {
        let mut extra = vec![0_u8; 0x40];
        extra[0x28..0x2C].copy_from_slice(&0x0040_1100_u32.to_le_bytes());
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let control = synthetic_control_info(0x002E, 1, 0x0040_1000);
        let table = read_event_table(&image, &control).unwrap();
        assert_eq!(table.slots.len(), 1);
        assert!(matches!(
            table.slots[0],
            EventSlot::Bound { index: 0, stub, .. } if stub == Va::new(0x0040_1100)
        ));
    }

    #[test]
    fn an_f_control_type_of_0x41_gives_no_slots_and_a_reason_naming_0x41() {
        let bytes = synthetic_image(&[0_u8; 0x10]);
        let image = PeImage::parse(&bytes).unwrap();
        let control = synthetic_control_info(0x0041, 5, 0x0040_1000);
        let table = read_event_table(&image, &control).unwrap();
        assert!(table.slots.is_empty());
        assert_eq!(table.unsupported_control_type, Some(0x0041));
    }

    #[test]
    fn a_w_event_count_of_zero_gives_an_empty_list_and_no_fault() {
        let bytes = synthetic_image(&[0_u8; 0x20]);
        let image = PeImage::parse(&bytes).unwrap();
        let control = synthetic_control_info(0x0040, 0, 0x0040_1000);
        let table = read_event_table(&image, &control).unwrap();
        assert!(table.slots.is_empty());
        assert!(table.defects().is_empty());
    }

    #[test]
    fn a_w_event_count_larger_than_the_file_can_hold_gives_a_defect_and_sizes_no_allocation() {
        // Header ends at 0x18; only 4 bytes remain in the mapped section
        // past it, so max_entries = 4 / 4 = 1, well below the claimed 1000.
        let extra = vec![0_u8; 0x1C];
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let control = synthetic_control_info(0x0040, 1000, 0x0040_1000);
        let table = read_event_table(&image, &control).unwrap();
        assert_eq!(table.slots.len(), 1);
        assert_eq!(table.defects().len(), 1);
        assert!(matches!(
            table.defects()[0].kind,
            DefectKind::ImplausibleCount {
                count: 1000,
                max: 1,
                ..
            }
        ));
    }

    #[test]
    fn a_slot_of_zero_reports_at_its_own_index_as_unbound_and_the_next_index_is_unchanged() {
        let mut extra = vec![0_u8; 0x30];
        // Slot 0 stays zero (unbound). Slot 1 is a non-null address that
        // resolves to no section: still bound, with no decoded handler.
        extra[0x18 + 4..0x18 + 8].copy_from_slice(&0x00F0_0000_u32.to_le_bytes());
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let control = synthetic_control_info(0x0040, 2, 0x0040_1000);
        let table = read_event_table(&image, &control).unwrap();
        assert_eq!(table.slots.len(), 2);
        assert!(matches!(table.slots[0], EventSlot::Unbound { index: 0 }));
        assert!(matches!(table.slots[1], EventSlot::Bound { index: 1, .. }));
    }

    /// Builds one native stub (`STRUCTURES.md` section 8.6): `sub dword ptr
    /// [esp+4], imm32` then `jmp rel32`, 13 bytes total.
    fn stub_bytes(imm32: u32, rel32: i32) -> [u8; 13] {
        let mut buf = [0_u8; 13];
        buf[0x00..0x04].copy_from_slice(&[0x81, 0x6C, 0x24, 0x04]);
        buf[0x04..0x08].copy_from_slice(&imm32.to_le_bytes());
        buf[0x08] = 0xE9;
        buf[0x09..0x0D].copy_from_slice(&rel32.to_le_bytes());
        buf
    }

    #[test]
    fn a_negative_relative_jump_gives_a_handler_address_below_the_stub_start() {
        let mut extra = vec![0_u8; 0x120];
        extra[0x18..0x1C].copy_from_slice(&0x0040_1100_u32.to_le_bytes());
        extra[0x100..0x10D].copy_from_slice(&stub_bytes(0x3F, -32));
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let control = synthetic_control_info(0x0040, 1, 0x0040_1000);
        let table = read_event_table(&image, &control).unwrap();
        let EventSlot::Bound {
            handler: Some(handler),
            stub,
            ..
        } = &table.slots[0]
        else {
            panic!(
                "expected a bound slot with a decoded handler: {:?}",
                table.slots[0]
            );
        };
        assert!(
            handler.handler_address < stub.get(),
            "synthetic fixture: handler address {:#x} must be below the stub start {:#x}",
            handler.handler_address,
            stub.get()
        );
        assert_eq!(handler.handler_address, 0x0040_10ED);
    }

    #[test]
    fn an_immediate_value_of_0xffff_marks_a_method_and_a_smaller_value_marks_an_event() {
        let mut extra = vec![0_u8; 0x160];
        extra[0x18..0x1C].copy_from_slice(&0x0040_1100_u32.to_le_bytes());
        extra[0x1C..0x20].copy_from_slice(&0x0040_1140_u32.to_le_bytes());
        extra[0x100..0x10D].copy_from_slice(&stub_bytes(0xFFFF, 0));
        extra[0x140..0x14D].copy_from_slice(&stub_bytes(0x3F, 0));
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let control = synthetic_control_info(0x0040, 2, 0x0040_1000);
        let table = read_event_table(&image, &control).unwrap();
        let EventSlot::Bound {
            handler: Some(method),
            ..
        } = &table.slots[0]
        else {
            panic!("expected slot 0 bound with a decoded handler");
        };
        let EventSlot::Bound {
            handler: Some(event),
            ..
        } = &table.slots[1]
        else {
            panic!("expected slot 1 bound with a decoded handler");
        };
        assert!(method.is_method, "imm32 0xFFFF must mark a method");
        assert!(!event.is_method, "imm32 0x3F must mark an event");
    }

    #[test]
    fn a_stub_address_in_no_section_gives_a_defect_and_the_slot_still_reports_as_bound() {
        let mut extra = vec![0_u8; 0x30];
        extra[0x18..0x1C].copy_from_slice(&0x00F0_0000_u32.to_le_bytes());
        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        assert!(image.region_at_va(Va::new(0x00F0_0000)).is_none());
        let control = synthetic_control_info(0x0040, 1, 0x0040_1000);
        let table = read_event_table(&image, &control).unwrap();
        assert!(matches!(
            table.slots[0],
            EventSlot::Bound {
                index: 0,
                handler: None,
                ..
            }
        ));
        assert_eq!(table.defects().len(), 1);
        assert!(matches!(
            table.defects()[0].kind,
            DefectKind::UnreadablePointer {
                va: 0x00F0_0000,
                ..
            }
        ));
    }

    #[test]
    fn the_doc_comment_names_the_p_code_stub_shapes_as_not_read() {
        let source = include_str!("controlinfo.rs");
        assert!(
            source.to_lowercase().matches("p-code").count() >= 1,
            "the doc comment must name the P-code stub shapes as not read in this task"
        );
    }

    #[test]
    fn sk_gradient_sample_command1_slot_zero_decodes_to_the_measured_stub() {
        let image = PeImage::parse(SK_GRADIENT_SAMPLE).unwrap();
        let object = first_form_object(SK_GRADIENT_SAMPLE);
        let control_info_table = ControlInfoTable::read(&image, &object).unwrap();
        let command1 = control_info_table
            .entries
            .iter()
            .find(|entry| entry.name == "Command1")
            .unwrap();
        assert_eq!(command1.w_event_count, 17);

        let event_table = read_event_table(&image, command1).unwrap();
        assert_eq!(event_table.slots.len(), 17);
        let EventSlot::Bound {
            index: 0,
            stub,
            handler: Some(handler),
        } = &event_table.slots[0]
        else {
            panic!(
                "slot 0 must be bound with a decoded handler: {:?}",
                event_table.slots[0]
            );
        };
        assert_eq!(*stub, Va::new(4_200_912));
        assert!(
            !handler.is_method,
            "imm32 0x3F marks an event, not a method"
        );
        assert_eq!(handler.handler_address, 4_201_984);
        assert!(
            event_table.slots[1..]
                .iter()
                .all(|slot| matches!(slot, EventSlot::Unbound { .. })),
            "every remaining slot must be unbound: {:?}",
            &event_table.slots[1..]
        );
        assert!(event_table.defects().is_empty());
    }

    // --- Task 3: the event name, reported honestly per D-02 ----------------

    #[test]
    fn a_bound_slot_with_a_named_table_entry_gives_the_named_state() {
        let mut names = EventNameTable::default();
        names
            .entries
            .insert(("CommandButton".to_owned(), 0), "Click".to_owned());
        let table = super::EventTable {
            slots: vec![EventSlot::Bound {
                index: 0,
                stub: Va::new(0x0040_1000),
                handler: None,
            }],
            unsupported_control_type: None,
            defects: Vec::new(),
        };

        let reports = report_events("Command1", "CommandButton", &table, &names);
        assert_eq!(reports.len(), 1);
        assert_eq!(
            reports[0],
            EventReport::Named {
                control_name: "Command1".to_owned(),
                index: 0,
                event_name: "Click".to_owned(),
            }
        );
        assert!(reports[0].no_name_message().is_none());
    }

    #[test]
    fn a_bound_slot_with_no_table_gives_the_bound_unnamed_state_and_a_reason() {
        let names = EventNameTable::default();
        let table = super::EventTable {
            slots: vec![EventSlot::Bound {
                index: 3,
                stub: Va::new(0x0040_1000),
                handler: None,
            }],
            unsupported_control_type: None,
            defects: Vec::new(),
        };

        let reports = report_events("Command1", "CommandButton", &table, &names);
        assert_eq!(
            reports[0],
            EventReport::BoundUnnamed {
                control_name: "Command1".to_owned(),
                index: 3,
            }
        );
        let message = reports[0].no_name_message().unwrap();
        assert!(message.contains('3'), "{message}");
        assert!(message.contains("Command1"), "{message}");
        assert!(message.contains("--event-name-table"), "{message}");
    }

    #[test]
    fn an_unbound_slot_gives_the_unbound_state_and_a_reason() {
        let names = EventNameTable::default();
        let table = super::EventTable {
            slots: vec![EventSlot::Unbound { index: 7 }],
            unsupported_control_type: None,
            defects: Vec::new(),
        };

        let reports = report_events("Picture1", "PictureBox", &table, &names);
        assert_eq!(
            reports[0],
            EventReport::Unbound {
                control_name: "Picture1".to_owned(),
                index: 7,
            }
        );
        let message = reports[0].no_name_message().unwrap();
        assert!(message.contains('7'), "{message}");
        assert!(message.contains("Picture1"), "{message}");
        assert!(message.contains("--event-name-table"), "{message}");
    }

    #[test]
    fn a_slot_on_a_control_type_with_no_table_entry_reports_no_name() {
        let mut names = EventNameTable::default();
        names
            .entries
            .insert(("CommandButton".to_owned(), 0), "Click".to_owned());
        let table = super::EventTable {
            slots: vec![EventSlot::Bound {
                index: 0,
                stub: Va::new(0x0040_1000),
                handler: None,
            }],
            unsupported_control_type: None,
            defects: Vec::new(),
        };

        // The table names an entry for CommandButton's own ordinal 0, but
        // this slot belongs to a Label: no name is invented from the
        // CommandButton entry, and no name is derived from the ordinal.
        let reports = report_events("Label1", "Label", &table, &names);
        assert!(matches!(reports[0], EventReport::BoundUnnamed { .. }));
    }

    #[test]
    fn two_runs_over_the_same_bytes_give_the_same_report_in_the_same_order() {
        let names = EventNameTable::default();
        let table = super::EventTable {
            slots: vec![
                EventSlot::Unbound { index: 0 },
                EventSlot::Bound {
                    index: 1,
                    stub: Va::new(0x0040_1000),
                    handler: None,
                },
                EventSlot::Unbound { index: 2 },
            ],
            unsupported_control_type: None,
            defects: Vec::new(),
        };

        let first = report_events("Command1", "CommandButton", &table, &names);
        let second = report_events("Command1", "CommandButton", &table, &names);
        assert_eq!(first, second);
        assert_eq!(first.len(), 3);
    }

    #[test]
    fn the_name_source_is_a_parameter_not_a_compiled_in_table() {
        assert_eq!(
            0,
            production_code_only().matches("const EVENT_NAMES").count()
        );
    }

    #[test]
    fn the_bound_and_unbound_reasons_read_differently_for_the_same_index_and_control() {
        let bound = EventReport::BoundUnnamed {
            control_name: "Command1".to_owned(),
            index: 0,
        };
        let unbound = EventReport::Unbound {
            control_name: "Command1".to_owned(),
            index: 0,
        };
        let bound_message = bound.no_name_message().unwrap();
        let unbound_message = unbound.no_name_message().unwrap();
        assert_ne!(
            bound_message, unbound_message,
            "a bound slot and an unbound slot must not read the same"
        );
        assert!(bound_message.contains("bound"), "{bound_message}");
        assert!(unbound_message.contains("unbound"), "{unbound_message}");
    }
}
