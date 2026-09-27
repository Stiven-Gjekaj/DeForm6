//! `derive-vb-types`: writes the interfaces of the Visual Basic 6 controls,
//! with the vtable offset and the argument bytes of each function, from a
//! copy of `VB6.OLB` that the user owns.
//!
//! A P-code call of a method of another object, such as `VCallHresult`,
//! names the method by its vtable offset, and the called method removes its
//! own arguments. So the lift must know the interface of the object and the
//! argument bytes of the method. `VB6.OLB` is the type library of the
//! intrinsic controls and of the form.
//!
//! # What the file holds
//!
//! - `[controls]` maps the GUID that a `ControlInfo` record names to the
//!   interface of the control. The record names the events interface of the
//!   control, such as `PictureBoxEvents`, and the interface of the control
//!   is `_PictureBox`.
//! - `[iids]` maps the GUID of each interface to its name. A class
//!   reference of a constant table, such as the one of the global object of
//!   the runtime, names an interface by its GUID.
//! - `[interfaces.<name>]` gives `vtable_size`, and for each vtable offset
//!   the names of the functions there, the kinds, `arg_bytes` and `result`.
//!   `arg_bytes` is the sum of the sizes of the parameters on the stack: 4
//!   for a pointer and for each type of 4 bytes or less, 8 for a `Double`,
//!   a `Currency` and a `Date`, and 16 for a `Variant`. It leaves out the
//!   4 bytes of the object itself. `result` tells whether the last
//!   parameter receives the result, and `result_interface` gives the
//!   interface of an object result when the library holds it.
//!
//! Two functions can share a vtable offset: a property let and a property
//! set of the same name. The tool writes the offset one time, and it stops
//! with an error when the two give different argument bytes.
//!
//! # What may enter the repository
//!
//! This tool is committed. `VB6.OLB` and the file that this tool writes are
//! not: `.gitignore` excludes [`DEFAULT_OUTPUT_PATH`].

use std::collections::BTreeMap;

use crate::msft::{Function, Kind, PARAMFLAG_FRETVAL, TypeInfo, guid_text, parse};

/// The default output path, relative to the workspace root.
pub(crate) const DEFAULT_OUTPUT_PATH: &str = "derived/vb-types.toml";

/// The suffix of the name of an events interface.
const EVENTS_SUFFIX: &str = "Events";

/// The bytes of one parameter of the variant type `vt` on the stack, or
/// `None` for a type that this tool does not size.
fn parameter_bytes(vt: u16) -> Option<u16> {
    match vt & 0x0FFF {
        2 | 3 | 4 | 8 | 9 | 10 | 11 | 13 | 16 | 17 | 18 | 19 | 22 | 23 | 25 | 26 | 27 | 30 | 31 => {
            Some(4)
        }
        5..=7 | 20 | 21 => Some(8),
        12 | 14 => Some(16),
        _ => None,
    }
}

/// The name of an `INVOKEKIND`.
fn kind_name(invoke_kind: u8) -> &'static str {
    match invoke_kind {
        1 => "method",
        2 => "get",
        4 => "let",
        8 => "set",
        _ => "other",
    }
}

/// The functions at one vtable offset.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct Slot {
    names: Vec<String>,
    kinds: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arg_bytes: Option<u16>,
    result: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result_interface: Option<String>,
}

/// One interface.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct Interface {
    vtable_size: u16,
    functions: BTreeMap<String, Slot>,
}

/// The whole file.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct Types {
    controls: BTreeMap<String, String>,
    iids: BTreeMap<String, String>,
    interfaces: BTreeMap<String, Interface>,
}

/// The name of the interface of the type info at `index`: the interface of
/// the same name with `_` before it when the library holds one, as for a
/// class, else the type info itself when it is an interface.
fn interface_name(infos: &[TypeInfo], index: usize) -> Option<String> {
    let info = infos.get(index)?;
    let name = info.name.as_ref()?;
    let underscored = format!("_{name}");
    let is_interface = |name: &str| {
        infos
            .iter()
            .any(|info| info.kind == Kind::Interface && info.name.as_deref() == Some(name))
    };
    if is_interface(&underscored) {
        Some(underscored)
    } else {
        is_interface(name).then(|| name.clone())
    }
}

/// The slot of one function.
fn slot(function: &Function, infos: &[TypeInfo]) -> Slot {
    let arg_bytes = function
        .parameters
        .iter()
        .try_fold(0_u16, |sum, parameter| {
            sum.checked_add(parameter_bytes(parameter.vt)?)
        });
    Slot {
        names: function.name.iter().cloned().collect(),
        kinds: vec![kind_name(function.invoke_kind)],
        arg_bytes,
        result: function
            .parameters
            .last()
            .is_some_and(|parameter| parameter.flags & PARAMFLAG_FRETVAL != 0),
        result_interface: function
            .parameters
            .last()
            .filter(|parameter| parameter.flags & PARAMFLAG_FRETVAL != 0)
            .and_then(|parameter| interface_name(infos, parameter.user_type?)),
    }
}

/// Derives the file from the type infos of a library.
///
/// # Errors
///
/// Gives an error when two functions at one vtable offset of one interface
/// give different argument bytes or results.
pub(crate) fn derive(infos: &[TypeInfo]) -> Result<Types, String> {
    let mut types = Types::default();
    for info in infos {
        let (Kind::Interface, Some(name)) = (info.kind, &info.name) else {
            continue;
        };
        if name.ends_with(EVENTS_SUFFIX) {
            continue;
        }
        let mut functions: BTreeMap<String, Slot> = BTreeMap::new();
        for function in &info.functions {
            let key = format!("{:04X}", function.vtable_offset);
            let new = slot(function, infos);
            match functions.get_mut(&key) {
                None => {
                    functions.insert(key, new);
                }
                Some(old) if old.arg_bytes == new.arg_bytes && old.result == new.result => {
                    old.names.extend(new.names);
                    old.kinds.extend(new.kinds);
                }
                Some(_) => {
                    return Err(format!(
                        "{name} holds two functions at {key} with different arguments"
                    ));
                }
            }
        }
        if let Some(guid) = &info.guid {
            types.iids.insert(guid_text(guid), name.clone());
        }
        types.interfaces.insert(
            name.clone(),
            Interface {
                vtable_size: info.vtable_size,
                functions,
            },
        );
    }
    for info in infos {
        let (Some(name), Some(guid)) = (&info.name, &info.guid) else {
            continue;
        };
        let Some(class) = name.strip_suffix(EVENTS_SUFFIX) else {
            continue;
        };
        let interface = format!("_{class}");
        if types.interfaces.contains_key(&interface) {
            types.controls.insert(guid_text(guid), interface);
        }
    }
    Ok(types)
}

/// Renders the file as TOML.
pub(crate) fn render(types: &Types) -> Result<String, String> {
    let body =
        toml::to_string(types).map_err(|err| format!("the types cannot be written: {err}"))?;
    Ok(format!(
        "# The interfaces of the Visual Basic 6 controls, written by\n# `cargo run -p xtask -- \
         derive-vb-types`. Do not commit this file.\n#\n# `controls` maps the GUID of the events \
         interface that a ControlInfo record\n# names to the interface of the control. `iids` maps the GUID of each\n# interface to its name. Each \
         function is keyed by its vtable\n# offset in hexadecimal. `arg_bytes` leaves out the 4 \
         bytes of the object.\n\n{body}"
    ))
}

/// Runs `derive-vb-types <olb> [--out <path>]`.
pub(crate) fn run(args: &[String]) -> i32 {
    let (olb, out) = match args {
        [olb] => (olb, DEFAULT_OUTPUT_PATH),
        [olb, flag, out] if flag == "--out" => (olb, out.as_str()),
        _ => {
            eprintln!("usage: cargo run -p xtask -- derive-vb-types <olb> [--out <path>]");
            return 1;
        }
    };
    match derive_file(olb, out) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(err) => {
            eprintln!("derive-vb-types: {err}");
            1
        }
    }
}

/// Reads the library, derives the types and writes them to `out`.
fn derive_file(olb: &str, out: &str) -> Result<String, String> {
    let bytes = std::fs::read(olb).map_err(|err| format!("reading {olb}: {err}"))?;
    let types = derive(&parse(&bytes)?)?;
    if let Some(parent) = std::path::Path::new(out).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("creating {}: {err}", parent.display()))?;
    }
    std::fs::write(out, render(&types)?).map_err(|err| format!("writing {out}: {err}"))?;
    let functions: usize = types
        .interfaces
        .values()
        .map(|interface| interface.functions.len())
        .sum();
    Ok(format!(
        "wrote {out}: {} interfaces, {functions} vtable offsets, {} controls",
        types.interfaces.len(),
        types.controls.len()
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{derive, render};
    use crate::msft::tests::library;
    use crate::msft::{Function, Kind, Parameter, TypeInfo, parse};

    #[test]
    fn each_offset_gives_its_argument_bytes_and_each_events_guid_its_interface() {
        let types = derive(&parse(&library()).unwrap()).unwrap();
        let text = render(&types).unwrap();
        assert!(
            text.contains("\"{33AD4ED2-6699-11CF-B70C-00AA0060D393}\" = \"_Box\""),
            "{text}"
        );
        assert!(text.contains("[interfaces._Box.functions.0024]"), "{text}");
        assert!(text.contains("[interfaces._Box.functions.0028]"), "{text}");
        let interface = &types.interfaces["_Box"];
        assert_eq!(interface.vtable_size, 0x30);
        assert_eq!(interface.functions["0024"].arg_bytes, Some(4));
        assert!(interface.functions["0024"].result);
        assert_eq!(interface.functions["0028"].arg_bytes, Some(20));
        assert!(!interface.functions["0028"].result);
        assert!(!types.interfaces.contains_key("BoxEvents"));
        // The result of Text points to BoxEvents, which is not an interface
        // that the file keeps a name for: no _BoxEvents, and BoxEvents is an
        // interface of the library.
        assert_eq!(
            interface.functions["0024"].result_interface.as_deref(),
            Some("BoxEvents")
        );
        assert_eq!(interface.functions["0028"].result_interface, None);
    }

    fn function(name: &str, invoke_kind: u8, vt: u16) -> Function {
        Function {
            name: Some(name.to_owned()),
            vtable_offset: 0x40,
            invoke_kind,
            parameters: vec![Parameter {
                vt,
                flags: 1,
                user_type: None,
            }],
        }
    }

    fn interface(functions: Vec<Function>) -> TypeInfo {
        TypeInfo {
            kind: Kind::Interface,
            name: Some("_Box".to_owned()),
            guid: None,
            vtable_size: 0x44,
            functions,
        }
    }

    #[test]
    fn a_let_and_a_set_at_one_offset_give_one_slot_or_an_error() {
        let same = derive(&[interface(vec![
            function("Picture", 4, 9),
            function("Picture", 8, 9),
        ])])
        .unwrap();
        let slot = &same.interfaces["_Box"].functions["0040"];
        assert_eq!(slot.kinds, ["let", "set"]);
        assert_eq!(slot.arg_bytes, Some(4));
        assert!(
            derive(&[interface(vec![
                function("Picture", 4, 9),
                function("Picture", 8, 12),
            ])])
            .is_err()
        );
    }

    #[test]
    fn the_guid_of_an_interface_names_it() {
        let mut info = interface(vec![function("Picture", 4, 9)]);
        info.guid = Some([
            0x22, 0x3D, 0xFB, 0xFC, 0xFA, 0xA0, 0x68, 0x10, 0xA7, 0x38, 0x08, 0x00, 0x2B, 0x33,
            0x71, 0xB5,
        ]);
        let types = derive(&[info]).unwrap();
        assert_eq!(types.iids["{FCFB3D22-A0FA-1068-A738-08002B3371B5}"], "_Box");
        let text = render(&types).unwrap();
        assert!(
            text.contains("\"{FCFB3D22-A0FA-1068-A738-08002B3371B5}\" = \"_Box\""),
            "{text}"
        );
    }

    #[test]
    fn a_type_that_the_tool_does_not_size_gives_no_argument_bytes() {
        let types = derive(&[interface(vec![function("Odd", 1, 29)])]).unwrap();
        assert_eq!(types.interfaces["_Box"].functions["0040"].arg_bytes, None);
    }
}
