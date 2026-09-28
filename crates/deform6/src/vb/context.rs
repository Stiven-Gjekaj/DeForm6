//! What the lift knows about each object of a project, from the image.
//!
//! [`callees_of_project`] builds the [`Callees`] of each object: the
//! methods and the accessors of its link table, the control accessors of a
//! form when the user derived the control types, the strings of its
//! constant table that its P-code names, and the profile of each object of
//! the project that its P-code names with `NewIfNullPr`, or the interface
//! of a class of the runtime that it names so. For the GUID that a
//! `VCallHresult` names, it gives the object of the project whose default
//! interface has that GUID, or else the interface that [`VbTypes`] names.
//! The null GUID names the object of a control array: each of the 65 calls
//! of the corpus that name it calls `Item` or `Count` after a control
//! accessor. For an import call of a function of the runtime, it gives the name and
//! the result interface that [`VbTypes`] gives for its ordinal.
//!
//! A public method has a `FuncTypDesc` record (`STRUCTURES.md` section
//! 6.3). An argument of an external class in it names a side structure,
//! whose second word is the address of the GUID of its interface. So the
//! record gives the interface of such an argument, and [`VbTypes`] gives its
//! name.
//!
//! It also gives the interface of an object argument of a method, when each
//! call of the method in the project gives that argument one interface. The
//! binary does not hold the class of an argument, so this is the only
//! source. A call that the lift cannot follow gives no evidence, and an
//! argument that one call gives no interface gets none. Two rounds let the
//! interface of an argument pass to the calls that the method makes.

use crate::read::pe::PeImage;
use crate::read::region::{Off, Va};
use crate::vb::classify::{ObjectKind, classify, has_optional_info};
use crate::vb::constants::{
    ProcedureStub, class_reference_iid, constant, constant_declare, constant_guid, constant_name,
    constant_procedure, constant_runtime_ordinal, constant_string,
};
use crate::vb::functyp::{FuncTypeWalk, ProcedureSignature, PrototypeList, TypeEntry, VbType};
use std::collections::BTreeMap;

use crate::vb::header::{VbHeader, header_region};
use crate::vb::lift::{
    CONTROL_ARRAY, Callees, ProjectCall, class_indexes, global_indexes, import_indexes,
    interface_indexes, method_calls, name_indexes, string_indexes,
};
use crate::vb::links::read_method_links;
use crate::vb::object::Object;
use crate::vb::pcode::{PcodeListing, PcodeTable, disassemble};
use crate::vb::privateobj::{ObjectInfo, PrivateObj, ProcNames, Procedure, ProcedureList};
use crate::vb::procdesc::{MethodEntry, read_method_table};
use crate::vb::project::{DeclareTable, ProjectInfo};
use crate::vb::types::VbTypes;

/// Gives the index and the listing of each body of the method table of
/// `object`.
fn listings(pe: &PeImage<'_>, object: &Object, table: &PcodeTable) -> Vec<(u16, PcodeListing)> {
    read_method_table(pe, object.lp_object_info)
        .map(|methods| {
            methods
                .entries
                .iter()
                .filter_map(|entry| match entry {
                    MethodEntry::Descriptor { index, descriptor } => {
                        Some((*index, disassemble(&descriptor.body(pe)?, table)))
                    }
                    MethodEntry::NotAnAddress { .. } | MethodEntry::Unreadable { .. } => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The number of rounds of [`with_arguments`].
const ARGUMENT_ROUNDS: usize = 2;

/// The first frame offset of an argument: after the saved `ebp`, the
/// return address and `Me`.
const FIRST_ARGUMENT: i16 = 0x0C;

/// The bytes of an argument of the type `entry` on the stack.
const fn argument_bytes(entry: &TypeEntry) -> i16 {
    if entry.by_ref || entry.array {
        return 4;
    }
    match entry.vb_type {
        VbType::Double | VbType::Date | VbType::Currency => 8,
        VbType::Variant => 16,
        _ => 4,
    }
}

/// Gives the interface of an argument of an external class: the GUID at
/// the address in the second word of its side structure.
fn argument_interface<'t>(pe: &PeImage<'_>, types: &'t VbTypes, side: Va) -> Option<&'t str> {
    let iid = pe.region_at_va(side)?.va_le(Off::new(4))?;
    let guid: [u8; 16] = pe
        .region_at_va(iid)?
        .take(Off::new(0), 16)?
        .try_into()
        .ok()?;
    types.interface_of_iid(&guid)
}

/// Adds to `callees` the interface of each argument of an external class of
/// each public method of `object` that its `FuncTypDesc` record names.
fn with_declared_arguments(
    callees: Callees,
    pe: &PeImage<'_>,
    object: &Object,
    types: &VbTypes,
) -> Callees {
    let Ok(private) = ObjectInfo::read(pe, object.lp_object_info)
        .and_then(|info| PrivateObj::read(pe, info.lp_private_object))
    else {
        return callees;
    };
    let PrototypeList::Slots(slots) = FuncTypeWalk::read(pe, object, &private).signatures else {
        return callees;
    };
    let mut callees = callees;
    for slot in slots {
        let ProcedureSignature::Prototype(prototype) = slot else {
            continue;
        };
        let Some(method) = callees
            .method(prototype.v_off & !1)
            .map(|method| method.index)
        else {
            continue;
        };
        let mut frame = FIRST_ARGUMENT;
        for argument in &prototype.arguments {
            if let VbType::ComObj(side) | VbType::ComIFace(side) = argument.entry.vb_type
                && let Some(interface) = argument_interface(pe, types, side)
            {
                callees = callees.with_argument(method, frame, interface);
            }
            frame = frame.saturating_add(argument_bytes(&argument.entry));
        }
    }
    callees
}

/// `STRUCTURES.md` section 5.3: `OptionalObjectInfo` sits at `ObjectInfo +
/// 0x38`. It holds the count of default IIDs at `0x10`, and at `0x1C` the
/// address of an array of addresses of them.
const OPTIONAL_OBJECT_INFO_AT: u32 = 0x38;
const DEFAULT_IID_COUNT_AT: u32 = 0x10;
const DEFAULT_IID_TABLE_AT: u32 = 0x1C;

/// Gives the GUID of the first default interface of `object`, when it has
/// an `OptionalObjectInfo` that names one.
fn default_iid(pe: &PeImage<'_>, object: &Object) -> Option<[u8; 16]> {
    if !has_optional_info(object.f_object_type) {
        return None;
    }
    let at = object
        .lp_object_info
        .get()
        .checked_add(OPTIONAL_OBJECT_INFO_AT)?;
    let optional = pe.region_at_va(Va::new(at))?;
    if optional.u32_le(Off::new(DEFAULT_IID_COUNT_AT))? == 0 {
        return None;
    }
    let table = optional.va_le(Off::new(DEFAULT_IID_TABLE_AT))?;
    let iid = pe.region_at_va(table)?.va_le(Off::new(0))?;
    pe.region_at_va(iid)?.take(Off::new(0), 16)?.try_into().ok()
}

/// The class that the interface GUID of a `VCallHresult` names.
#[derive(Debug, PartialEq, Eq)]
enum CallClass<'a> {
    /// The object of a control array: the null GUID.
    Array,
    /// The object of the project whose default interface has the GUID.
    Project(&'a Callees),
    /// The interface of the types file that has the GUID.
    Runtime(&'a str),
}

/// Gives the class that `iid` names. `defaults` gives the default interface
/// of each object of the project, with its profile. The null GUID names the
/// object of a control array: each of the 65 calls of the corpus that name
/// it calls `Item` or `Count` right after a control accessor.
fn call_class<'a>(
    iid: &[u8; 16],
    defaults: &[(Option<[u8; 16]>, &'a Callees)],
    types: Option<&'a VbTypes>,
) -> Option<CallClass<'a>> {
    if *iid == [0; 16] {
        return Some(CallClass::Array);
    }
    if let Some((_, profile)) = defaults
        .iter()
        .find(|(default, _)| default.as_ref() == Some(iid))
    {
        return Some(CallClass::Project(profile));
    }
    types?.interface_of_iid(iid).map(CallClass::Runtime)
}

/// Gives the vtable of `object`: its methods and accessors, and its control
/// accessors when `types` is given.
fn profile(pe: &PeImage<'_>, object: &Object, types: Option<&VbTypes>) -> Callees {
    let methods = read_method_table(pe, object.lp_object_info).unwrap_or_default();
    let mut callees = read_method_links(pe, object)
        .map(|links| links.callees(&methods))
        .unwrap_or_default();
    if classify(object.f_object_type) == ObjectKind::Form {
        callees = callees.with_form_name(&object.name);
    }
    if let ProcNames::Slots(slots) = ProcedureList::read(pe, object).procs {
        for (slot, index) in slots.iter().zip(0_u16..) {
            if let Procedure::Public(name) = slot {
                callees = callees.with_procedure(index, name);
            }
        }
    }
    match types {
        Some(types) => types.with_controls(callees, pe, object),
        None => callees,
    }
}

/// Gives, for each descriptor of the method tables of `objects`, the index
/// of its object and its index in the method table.
fn descriptor_owners(pe: &PeImage<'_>, objects: &[Object]) -> BTreeMap<u32, (usize, u16)> {
    let mut out = BTreeMap::new();
    for (owner, object) in objects.iter().enumerate() {
        let Ok(methods) = read_method_table(pe, object.lp_object_info) else {
            continue;
        };
        for entry in &methods.entries {
            if let MethodEntry::Descriptor { index, descriptor } = entry {
                out.insert(descriptor.va.get(), (owner, *index));
            }
        }
    }
    out
}

/// Gives the procedure of the project that `stub` goes to: the name of its
/// module and its name, or its name when it is of an object.
fn project_call(
    stub: ProcedureStub,
    owners: &BTreeMap<u32, (usize, u16)>,
    objects: &[Object],
    profiles: &[Callees],
) -> Option<ProjectCall> {
    let (owner, index) = *owners.get(&stub.descriptor.get())?;
    let name = profiles.get(owner)?.procedure(index);
    if stub.of_object {
        Some(ProjectCall::Object(name))
    } else {
        Some(ProjectCall::Module(objects.get(owner)?.name.clone(), name))
    }
}

/// Gives the [`Callees`] of each object of `objects`, in the same order.
///
/// A table or a record that cannot be read gives nothing, and the lift of
/// a call that needs it stops with a fault.
#[must_use]
pub fn callees_of_project(
    pe: &PeImage<'_>,
    objects: &[Object],
    table: &PcodeTable,
    types: Option<&VbTypes>,
) -> Vec<Callees> {
    callees_of_project_named(pe, objects, table, types, &[])
}

/// Gives the [`Callees`] of each object of `objects`, as
/// [`callees_of_project`] does, with the names of `names`: for each object,
/// in the same order, the name of each procedure by its index in the method
/// table. A name of `names` replaces the name that the file gives.
#[must_use]
pub fn callees_of_project_named(
    pe: &PeImage<'_>,
    objects: &[Object],
    table: &PcodeTable,
    types: Option<&VbTypes>,
    names: &[Vec<(u16, String)>],
) -> Vec<Callees> {
    let profiles: Vec<Callees> = objects
        .iter()
        .zip(0_u16..)
        .enumerate()
        .map(|(at, (object, owner))| {
            let mut profile = profile(pe, object, types).with_owner(owner);
            for (index, name) in names.get(at).map(Vec::as_slice).unwrap_or_default() {
                profile = profile.with_procedure(*index, name);
            }
            profile
        })
        .collect();
    let declares = header_region(pe)
        .and_then(|region| VbHeader::read(&region))
        .and_then(|header| ProjectInfo::read(pe, header.lp_project_data))
        .ok()
        .map(|info| DeclareTable::read(pe, &info));
    let owners = descriptor_owners(pe, objects);
    let mut out = Vec::new();
    for (object, own) in objects.iter().zip(&profiles) {
        let mut callees = match types {
            Some(types) => with_declared_arguments(own.clone(), pe, object, types),
            None => own.clone(),
        };
        let listings = listings(pe, object, table);
        for (_, listing) in &listings {
            for index in string_indexes(listing, table) {
                if let Some(text) = constant_string(pe, object.lp_object_info, index) {
                    callees = callees.with_string(index, &text);
                }
            }
            for index in name_indexes(listing, table) {
                if let Some(name) = constant_name(pe, object.lp_object_info, index) {
                    callees = callees.with_name(index, &name);
                }
            }
            for index in global_indexes(listing, table) {
                if let Some(address) = constant(pe, object.lp_object_info, index) {
                    callees = callees.with_global(index, address.get());
                }
            }
            for index in import_indexes(listing, table) {
                if let Some(name) = constant_declare(pe, object.lp_object_info, index)
                    .and_then(|descriptor| declares.as_ref()?.export_at(pe, descriptor))
                {
                    callees = callees.with_declare(index, &name);
                }
                if let Some(call) = constant_procedure(pe, object.lp_object_info, index)
                    .and_then(|stub| project_call(stub, &owners, objects, &profiles))
                {
                    callees = callees.with_project_call(index, call);
                }
                if let Some(import) = constant_runtime_ordinal(pe, object.lp_object_info, index)
                    .and_then(|ordinal| types?.import(ordinal))
                {
                    callees = callees.with_import(
                        index,
                        &import.name,
                        import.result_interface.as_deref(),
                    );
                }
            }
            for index in interface_indexes(listing, table) {
                let Some(iid) = constant_guid(pe, object.lp_object_info, index) else {
                    continue;
                };
                let defaults: Vec<(Option<[u8; 16]>, &Callees)> = objects
                    .iter()
                    .map(|other| default_iid(pe, other))
                    .zip(&profiles)
                    .collect();
                callees = match call_class(&iid, &defaults, types) {
                    Some(CallClass::Array) => callees.with_class_interface(index, CONTROL_ARRAY),
                    Some(CallClass::Project(profile)) => callees.with_class(index, profile.clone()),
                    Some(CallClass::Runtime(interface)) => {
                        callees.with_class_interface(index, interface)
                    }
                    None => callees,
                };
            }
            for index in class_indexes(listing, table) {
                let Some(target) = constant(pe, object.lp_object_info, index) else {
                    continue;
                };
                if let Some((_, profile)) = objects
                    .iter()
                    .zip(&profiles)
                    .find(|(other, _)| other.lp_object_info == target)
                {
                    callees = callees.with_class(index, profile.clone());
                } else if let Some(interface) = types.and_then(|types| {
                    types.interface_of_iid(&class_reference_iid(pe, object.lp_object_info, index)?)
                }) {
                    callees = callees.with_class_interface(index, interface);
                }
            }
        }
        out.push((callees, listings));
    }
    with_arguments(out, table, types)
}

/// Adds to the callees of each object the interface of each argument of a
/// method of the object that each call of it in the project gives one
/// interface.
fn with_arguments(
    objects: Vec<(Callees, Vec<(u16, PcodeListing)>)>,
    table: &PcodeTable,
    types: Option<&VbTypes>,
) -> Vec<Callees> {
    let (mut all, listings): (Vec<Callees>, Vec<Vec<(u16, PcodeListing)>>) =
        objects.into_iter().unzip();
    for _ in 0..ARGUMENT_ROUNDS {
        let mut seen: BTreeMap<(u16, u16, i16), Vec<Option<String>>> = BTreeMap::new();
        for (callees, own) in all.iter().zip(&listings) {
            for (index, listing) in own {
                for (owner, method, args) in method_calls(listing, table, callees, types, *index) {
                    let Some(owner) = owner else {
                        continue;
                    };
                    let mut slot = FIRST_ARGUMENT;
                    for (bytes, class) in args {
                        seen.entry((owner, method, slot)).or_default().push(class);
                        slot = slot.saturating_add(i16::from(bytes));
                    }
                }
            }
        }
        for ((owner, method, slot), classes) in seen {
            let Some(Some(first)) = classes.first() else {
                continue;
            };
            let Some(callees) = all.get_mut(usize::from(owner)) else {
                continue;
            };
            if classes.iter().all(|class| class.as_ref() == Some(first))
                && !callees
                    .arguments_of(method)
                    .iter()
                    .any(|(known, _)| *known == slot)
            {
                *callees = callees.clone().with_argument(method, slot, first);
            }
        }
    }
    all
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test reads a file of the corpus and must fail loudly when it cannot"
)]
mod tests {
    use super::{CallClass, call_class, default_iid};
    use crate::read::pe::PeImage;
    use crate::vb::constants::constant_guid;
    use crate::vb::header::{VbHeader, header_region};
    use crate::vb::lift::Callees;
    use crate::vb::object::ObjectTable;
    use crate::vb::project::{ObjectTableHead, ProjectInfo};
    use crate::vb::types::VbTypes;

    const DIFFUSE_P_CODE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus-pcode/vb6-code/Diffuse-effect/Diffuse.exe"
    ));

    /// The null GUID names a control array; the default interface of an
    /// object of the project comes before an interface of the types file;
    /// and a GUID that neither gives names nothing.
    #[test]
    fn the_guid_of_a_call_names_an_array_an_object_or_an_interface() {
        let project = [7_u8; 16];
        let runtime = [
            0x22, 0x3D, 0xFB, 0xFC, 0xFA, 0xA0, 0x68, 0x10, 0xA7, 0x38, 0x08, 0x00, 0x2B, 0x33,
            0x71, 0xB5,
        ];
        let profile = Callees::default().with_owner(3);
        let defaults = [(None, &profile), (Some(project), &profile)];
        let types = VbTypes::parse(
            b"[iids]\n\"{FCFB3D22-A0FA-1068-A738-08002B3371B5}\" = \"VBGlobal\"\n\
              \"{07070707-0707-0707-0707-070707070707}\" = \"_Other\"\n",
        )
        .unwrap();
        assert_eq!(
            call_class(&[0; 16], &defaults, Some(&types)),
            Some(CallClass::Array)
        );
        assert_eq!(
            call_class(&project, &defaults, Some(&types)),
            Some(CallClass::Project(&profile))
        );
        assert_eq!(
            call_class(&runtime, &defaults, Some(&types)),
            Some(CallClass::Runtime("VBGlobal"))
        );
        assert_eq!(call_class(&runtime, &defaults, None), None);
        assert_eq!(call_class(&[9; 16], &defaults, Some(&types)), None);
    }

    /// `frmDiffuse` in the P-code `Diffuse.exe` calls the methods of its
    /// `FastDrawing` object with `VCallHresult`, whose second argument names
    /// entry 9 of its constant table. That entry holds the GUID of the
    /// default interface of `FastDrawing`, and of no other object.
    #[test]
    fn a_call_names_the_default_interface_of_an_object_of_the_project() {
        let pe = PeImage::parse(DIFFUSE_P_CODE).unwrap();
        let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
        let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
        let head = ObjectTableHead::read(&pe, info.lp_object_table).unwrap();
        let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
            .unwrap()
            .objects;
        let form = objects.iter().find(|o| o.name == "frmDiffuse").unwrap();
        let called = constant_guid(&pe, form.lp_object_info, 9);
        assert!(called.is_some());
        let named: Vec<&str> = objects
            .iter()
            .filter(|object| default_iid(&pe, object) == called)
            .map(|object| object.name.as_str())
            .collect();
        assert_eq!(named, ["FastDrawing"]);
    }
}
