//! What the lift knows about each object of a project, from the image.
//!
//! [`callees_of_project`] builds the [`Callees`] of each object: the
//! methods and the accessors of its link table, the control accessors of a
//! form when the user derived the control types, the strings of its
//! constant table that its P-code names, and the profile of each object of
//! the project that its P-code names with `NewIfNullPr`, or the interface
//! of a class of the runtime that it names so.

use crate::read::pe::PeImage;
use crate::vb::constants::{class_reference_iid, constant, constant_string};
use crate::vb::lift::{Callees, class_indexes, string_indexes};
use crate::vb::links::read_method_links;
use crate::vb::object::Object;
use crate::vb::pcode::{PcodeListing, PcodeTable, disassemble};
use crate::vb::procdesc::read_method_table;
use crate::vb::types::VbTypes;

/// Gives the listing of each body of the method table of `object`.
fn listings(pe: &PeImage<'_>, object: &Object, table: &PcodeTable) -> Vec<PcodeListing> {
    read_method_table(pe, object.lp_object_info)
        .map(|methods| {
            methods
                .descriptors()
                .filter_map(|descriptor| descriptor.body(pe))
                .map(|body| disassemble(&body, table))
                .collect()
        })
        .unwrap_or_default()
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
        for listing in listings(pe, object, table) {
            for index in string_indexes(&listing, table) {
                if let Some(text) = constant_string(pe, object.lp_object_info, index) {
                    callees = callees.with_string(index, &text);
                }
            }
            for index in class_indexes(&listing, table) {
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
        out.push(callees);
    }
    out
}
