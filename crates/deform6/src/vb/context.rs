//! What the lift knows about each object of a project, from the image.
//!
//! [`callees_of_project`] builds the [`Callees`] of each object: the
//! methods and the accessors of its link table, the control accessors of a
//! form when the user derived the control types, the strings of its
//! constant table that its P-code names, and the profile of each object of
//! the project that its P-code names with `NewIfNullPr`, or the interface
//! of a class of the runtime that it names so.
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
use crate::vb::constants::{class_reference_iid, constant, constant_string};
use crate::vb::functyp::{FuncTypeWalk, ProcedureSignature, PrototypeList, TypeEntry, VbType};
use std::collections::BTreeMap;

use crate::vb::lift::{Callees, class_indexes, method_calls, string_indexes};
use crate::vb::links::read_method_links;
use crate::vb::object::Object;
use crate::vb::pcode::{PcodeListing, PcodeTable, disassemble};
use crate::vb::privateobj::{ObjectInfo, PrivateObj};
use crate::vb::procdesc::{MethodEntry, read_method_table};
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

/// Gives the vtable of `object`: its methods and accessors, and its control
/// accessors when `types` is given.
fn profile(pe: &PeImage<'_>, object: &Object, types: Option<&VbTypes>) -> Callees {
    let methods = read_method_table(pe, object.lp_object_info).unwrap_or_default();
    let callees = read_method_links(pe, object)
        .map(|links| links.callees(&methods))
        .unwrap_or_default();
    match types {
        Some(types) => types.with_controls(callees, pe, object),
        None => callees,
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
    let profiles: Vec<Callees> = objects
        .iter()
        .zip(0_u16..)
        .map(|(object, owner)| profile(pe, object, types).with_owner(owner))
        .collect();
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
