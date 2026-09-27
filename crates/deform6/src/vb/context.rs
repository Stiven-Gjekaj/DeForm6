//! What the lift knows about each object of a project, from the image.
//!
//! [`callees_of_project`] builds the [`Callees`] of each object: the
//! methods and the accessors of its link table, the control accessors of a
//! form when the user derived the control types, the strings of its
//! constant table that its P-code names, and the profile of each object of
//! the project that its P-code names with `NewIfNullPr`, or the interface
//! of a class of the runtime that it names so.
//!
//! It also gives the interface of an object argument of a method, when each
//! call of the method in its object gives that argument one interface. The
//! binary does not hold the class of an argument, so this is the only
//! source. A call that the lift cannot follow gives no evidence, and an
//! argument that one call gives no interface gets none. Two rounds let the
//! interface of an argument pass to the calls that the method makes.

use crate::read::pe::PeImage;
use crate::vb::constants::{class_reference_iid, constant, constant_string};
use std::collections::BTreeMap;

use crate::vb::lift::{Callees, class_indexes, method_calls, string_indexes};
use crate::vb::links::read_method_links;
use crate::vb::object::Object;
use crate::vb::pcode::{PcodeListing, PcodeTable, disassemble};
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

/// Adds to `callees` the interface of each argument of a method of the
/// object that each call of it in `listings` gives one interface.
fn with_arguments(
    callees: Callees,
    listings: &[(u16, PcodeListing)],
    table: &PcodeTable,
    types: Option<&VbTypes>,
) -> Callees {
    let mut callees = callees;
    for _ in 0..ARGUMENT_ROUNDS {
        let mut seen: BTreeMap<(u16, i16), Vec<Option<String>>> = BTreeMap::new();
        for (index, listing) in listings {
            for (method, args) in method_calls(listing, table, &callees, types, *index) {
                let mut slot = FIRST_ARGUMENT;
                for (bytes, class) in args {
                    seen.entry((method, slot)).or_default().push(class);
                    slot = slot.saturating_add(i16::from(bytes));
                }
            }
        }
        let mut next = callees.clone();
        for ((method, slot), classes) in seen {
            let Some(Some(first)) = classes.first() else {
                continue;
            };
            if classes.iter().all(|class| class.as_ref() == Some(first))
                && !callees
                    .arguments_of(method)
                    .iter()
                    .any(|(known, _)| *known == slot)
            {
                next = next.with_argument(method, slot, first);
            }
        }
        callees = next;
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
        .map(|object| profile(pe, object, types))
        .collect();
    let mut out = Vec::new();
    for (object, own) in objects.iter().zip(&profiles) {
        let mut callees = own.clone();
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
        out.push(with_arguments(callees, &listings, table, types));
    }
    out
}
