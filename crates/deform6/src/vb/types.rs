//! The interfaces of the controls, from a file that the user derived.
//!
//! `cargo run -p xtask -- derive-vb-types <VB6.OLB>` writes the file from a
//! copy of `VB6.OLB` that the user owns. This module reads it, and it holds
//! no interface of its own:
//!
//! - `[controls]` maps the GUID that a `ControlInfo` record names, in the
//!   registry form, to the name of the interface of the control.
//! - `[iids]` maps the GUID of an interface to its name.
//! - `[interfaces.<name>]` gives `vtable_size`, and under `functions` a
//!   table for each vtable offset in hexadecimal, with `names`, `kinds`,
//!   `arg_bytes` and `result`. `arg_bytes` leaves out the 4 bytes of the object. It is
//!   absent when the tool cannot size a parameter.

use std::collections::BTreeMap;

use crate::read::pe::PeImage;
use crate::read::region::Off;
use crate::vb::controlinfo::{ControlInfo, ControlInfoTable};
use crate::vb::lift::Callees;
use crate::vb::object::Object;
use crate::vb::opcodes::{TableError, line_at};

/// The `wIndex` of the `ControlInfo` record of a form itself.
const FORM_RECORD_INDEX: u16 = 0xFFFF;

/// The functions at one vtable offset of an interface.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TypeFunction {
    /// The names of the functions at the offset.
    pub names: Vec<String>,
    /// The kind of each function: `method`, `get`, `let` or `set`.
    pub kinds: Vec<String>,
    /// The bytes of the arguments on the stack, without the object, or
    /// `None` when the file gives none.
    pub arg_bytes: Option<u16>,
    /// Whether the last argument receives the result.
    pub result: bool,
}

/// One interface.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct InterfaceType {
    /// The size of the vtable in bytes.
    pub vtable_size: u16,
    functions: BTreeMap<u16, TypeFunction>,
}

impl InterfaceType {
    /// Gives the functions at `vtable_offset`.
    #[must_use]
    pub fn function(&self, vtable_offset: u16) -> Option<&TypeFunction> {
        self.functions.get(&vtable_offset)
    }
}

/// The interfaces and the controls of the file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VbTypes {
    controls: BTreeMap<String, String>,
    iids: BTreeMap<String, String>,
    interfaces: BTreeMap<String, InterfaceType>,
}

/// The file, before it is folded into [`VbTypes`].
#[derive(serde::Deserialize)]
struct RawTypes {
    #[serde(default)]
    controls: BTreeMap<String, String>,
    #[serde(default)]
    iids: BTreeMap<String, String>,
    #[serde(default)]
    interfaces: BTreeMap<String, RawInterface>,
}

/// One interface of the file.
#[derive(serde::Deserialize)]
struct RawInterface {
    vtable_size: u16,
    #[serde(default)]
    functions: BTreeMap<String, RawFunction>,
}

/// One vtable offset of the file.
#[derive(serde::Deserialize)]
struct RawFunction {
    #[serde(default)]
    names: Vec<String>,
    #[serde(default)]
    kinds: Vec<String>,
    #[serde(default)]
    arg_bytes: Option<u16>,
    #[serde(default)]
    result: bool,
}

/// Formats a GUID in the registry form, such as
/// `{33AD4ED2-6699-11CF-B70C-00AA0060D393}`.
#[must_use]
pub fn guid_text(guid: &[u8; 16]) -> String {
    let [a0, a1, a2, a3, b0, b1, c0, c1, rest @ ..] = *guid;
    let tail: String = rest.iter().map(|byte| format!("{byte:02X}")).collect();
    format!(
        "{{{:08X}-{:04X}-{:04X}-{}-{}}}",
        u32::from_le_bytes([a0, a1, a2, a3]),
        u16::from_le_bytes([b0, b1]),
        u16::from_le_bytes([c0, c1]),
        tail.get(..4).unwrap_or_default(),
        tail.get(4..).unwrap_or_default()
    )
}

impl VbTypes {
    /// Parses a file that `derive-vb-types` wrote.
    ///
    /// # Errors
    ///
    /// Gives a [`TableError`] for text that is not UTF-8 or not TOML, and for
    /// a function key that is not a 16-bit hexadecimal number.
    pub fn parse(bytes: &[u8]) -> Result<Self, TableError> {
        let text = std::str::from_utf8(bytes).map_err(|err| TableError {
            line: line_at(bytes, err.valid_up_to()),
            message: "the types are not valid UTF-8".to_owned(),
        })?;
        let raw: RawTypes = toml::from_str(text).map_err(|err| TableError {
            line: err
                .span()
                .map_or(1, |span| line_at(text.as_bytes(), span.start)),
            message: err.message().to_owned(),
        })?;
        let mut interfaces = BTreeMap::new();
        for (name, interface) in raw.interfaces {
            let mut functions = BTreeMap::new();
            for (key, function) in interface.functions {
                let offset = u16::from_str_radix(&key, 16).map_err(|_| TableError {
                    line: 1,
                    message: format!("{name}.functions.{key} is not a vtable offset"),
                })?;
                functions.insert(
                    offset,
                    TypeFunction {
                        names: function.names,
                        kinds: function.kinds,
                        arg_bytes: function.arg_bytes,
                        result: function.result,
                    },
                );
            }
            interfaces.insert(
                name,
                InterfaceType {
                    vtable_size: interface.vtable_size,
                    functions,
                },
            );
        }
        Ok(Self {
            controls: raw.controls,
            iids: raw.iids,
            interfaces,
        })
    }

    /// Gives the name of the interface of the control whose `ControlInfo`
    /// record names `guid`.
    #[must_use]
    pub fn control_interface(&self, guid: &[u8; 16]) -> Option<&str> {
        self.controls.get(&guid_text(guid)).map(String::as_str)
    }

    /// Gives the 16 bytes of the GUID that `control` names, when the image
    /// holds them.
    fn guid(pe: &PeImage<'_>, control: &ControlInfo) -> Option<[u8; 16]> {
        pe.region_at_va(control.lp_guid)?
            .take(Off::new(0), 16)?
            .try_into()
            .ok()
    }

    /// Adds to `callees` the control accessors of `object`, when it is a
    /// form.
    ///
    /// The record of the form itself holds `wIndex` `0xFFFF`, and its GUID
    /// names the interface of the form. The accessor of a control is at the
    /// vtable size of that interface, plus 4 times the `wIndex` of the
    /// control. A control whose GUID names no interface of the file is left
    /// out, and so is each control of an object that is not a form.
    #[must_use]
    pub fn with_controls(&self, callees: Callees, pe: &PeImage<'_>, object: &Object) -> Callees {
        let Ok(controls) = ControlInfoTable::read(pe, object) else {
            return callees;
        };
        let base = controls
            .entries
            .iter()
            .find(|control| control.w_index == FORM_RECORD_INDEX)
            .and_then(|form| self.control_interface(&Self::guid(pe, form)?))
            .and_then(|interface| self.interface(interface))
            .map(|interface| u32::from(interface.vtable_size));
        let Some(base) = base else {
            return callees;
        };
        let mut callees = callees;
        for control in &controls.entries {
            if control.w_index == FORM_RECORD_INDEX {
                continue;
            }
            let offset = u32::from(control.w_index)
                .checked_mul(4)
                .and_then(|bytes| bytes.checked_add(base))
                .and_then(|offset| u16::try_from(offset).ok());
            let interface = Self::guid(pe, control)
                .and_then(|guid| self.control_interface(&guid).map(str::to_owned));
            if let (Some(offset), Some(interface)) = (offset, interface) {
                callees = callees.with_control(offset, &control.name, &interface);
            }
        }
        callees
    }

    /// Gives the name of the interface whose GUID is `guid`.
    #[must_use]
    pub fn interface_of_iid(&self, guid: &[u8; 16]) -> Option<&str> {
        self.iids.get(&guid_text(guid)).map(String::as_str)
    }

    /// Gives the interface of the name `name`.
    #[must_use]
    pub fn interface(&self, name: &str) -> Option<&InterfaceType> {
        self.interfaces.get(name)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{VbTypes, guid_text};

    /// A file built here, in the form that `derive-vb-types` writes. The
    /// interface and its functions are placeholders.
    const TYPES: &str = r#"
[controls]
"{33AD4ED2-6699-11CF-B70C-00AA0060D393}" = "_Box"

[iids]
"{33AD4ED1-6699-11CF-B70C-00AA0060D393}" = "_Box"

[interfaces._Box]
vtable_size = 48

[interfaces._Box.functions.0024]
names = ["Text"]
kinds = ["get"]
arg_bytes = 4
result = true

[interfaces._Box.functions.0028]
names = ["Odd"]
kinds = ["method"]
result = false
"#;

    const GUID: [u8; 16] = [
        0xD2, 0x4E, 0xAD, 0x33, 0x99, 0x66, 0xCF, 0x11, 0xB7, 0x0C, 0x00, 0xAA, 0x00, 0x60, 0xD3,
        0x93,
    ];

    #[test]
    fn a_guid_names_its_interface_and_an_offset_its_functions() {
        let types = VbTypes::parse(TYPES.as_bytes()).unwrap();
        assert_eq!(types.control_interface(&GUID), Some("_Box"));
        assert_eq!(types.control_interface(&[0; 16]), None);
        let mut iid = GUID;
        iid[0] = 0xD1;
        assert_eq!(types.interface_of_iid(&iid), Some("_Box"));
        assert_eq!(types.interface_of_iid(&GUID), None);
        let interface = types.interface("_Box").unwrap();
        assert_eq!(interface.vtable_size, 48);
        let text = interface.function(0x24).unwrap();
        assert_eq!(text.names, ["Text"]);
        assert_eq!(text.kinds, ["get"]);
        assert_eq!(text.arg_bytes, Some(4));
        assert!(text.result);
        assert_eq!(interface.function(0x28).unwrap().arg_bytes, None);
        assert!(interface.function(0x2C).is_none());
        assert!(types.interface("_Other").is_none());
    }

    #[test]
    fn a_damaged_file_gives_an_error_with_its_line() {
        let bad_key = TYPES.replace("functions.0024", "functions.x024");
        assert!(VbTypes::parse(bad_key.as_bytes()).is_err());
        let error = VbTypes::parse(b"[interfaces._Box]\nvtable_size = \"big\"\n").unwrap_err();
        assert_eq!(error.line, 2);
        assert!(VbTypes::parse(&[0xFF, 0xFE]).is_err());
    }

    #[test]
    fn a_guid_has_the_registry_form() {
        assert_eq!(guid_text(&GUID), "{33AD4ED2-6699-11CF-B70C-00AA0060D393}");
    }
}
