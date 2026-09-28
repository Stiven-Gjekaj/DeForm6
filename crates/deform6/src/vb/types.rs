//! The interfaces of the controls, from a file that the user derived.
//!
//! `cargo run -p xtask -- derive-vb-types <VB6.OLB>` writes the file from a
//! copy of `VB6.OLB` that the user owns. This module reads it, and it holds
//! no interface of its own:
//!
//! - `[controls]` maps the GUID that a `ControlInfo` record names, in the
//!   registry form, to the name of the interface of the control.
//! - `[events]` maps the GUID of an events interface to its events, in the
//!   order of the slots of an event table: each with `name`, and with
//!   `parameters` when the tool could declare each of them.
//! - `[iids]` maps the GUID of an interface to its name.
//! - `[imports]` maps the ordinal of an export of the runtime to the name
//!   of its function, with `result_interface` when the result is an
//!   object.
//! - `[interfaces.<name>]` gives `vtable_size`, and under `functions` a
//!   table for each vtable offset in hexadecimal, with `names`, `kinds`,
//!   `arg_bytes`, `result` and `dispid`. `arg_bytes` leaves out the 4 bytes of the object. It is
//!   absent when the tool cannot size a parameter.

use std::collections::BTreeMap;

use crate::read::pe::PeImage;
use crate::read::region::{Off, Va};
use crate::vb::controlinfo::{ControlInfo, ControlInfoTable};
use crate::vb::header::{VbHeader, header_region};
use crate::vb::lift::{CONTROL_ARRAY, Callees};
use crate::vb::object::Object;
use crate::vb::opcodes::{TableError, line_at};

/// The offsets in an entry of the external component table of the GUID of
/// the default interface of the control, of the GUID that the `ControlInfo`
/// record of one control names, and of the GUID that the record of a control
/// array names. `STRUCTURES.md` section 23c gives the measurement.
const EXTERNAL_INTERFACE_AT: u32 = 0x58;
const EXTERNAL_CONTROL_AT: u32 = 0x98;
const EXTERNAL_ARRAY_AT: u32 = 0xA8;

/// The offset in an entry of the external component table of the GUID of
/// the events interface of the control.
const EXTERNAL_EVENTS_AT: u32 = 0x48;

/// Gives, for the GUID that the `ControlInfo` record of a control of an OCX
/// names, the GUID at `at` of its entry of the external component table.
fn external_guid(pe: &PeImage<'_>, guid: &[u8; 16], at: u32) -> Option<[u8; 16]> {
    let header = VbHeader::read(&header_region(pe).ok()?).ok()?;
    let mut entry_at = header.lp_external_table.get();
    for _ in 0..header.w_external_count {
        let entry = pe.region_at_va(Va::new(entry_at))?;
        let read =
            |offset: u32| -> Option<[u8; 16]> { entry.take(Off::new(offset), 16)?.try_into().ok() };
        if [EXTERNAL_CONTROL_AT, EXTERNAL_ARRAY_AT]
            .iter()
            .any(|offset| read(*offset).as_ref() == Some(guid))
        {
            return read(at);
        }
        let length = entry.u32_le(Off::new(0)).filter(|length| *length > 0)?;
        entry_at = entry_at.checked_add(length)?;
    }
    None
}

/// Gives, for the GUID that the `ControlInfo` record of a control of an OCX
/// names, the GUID of the interface of the control, and whether the record
/// is a control array. It reads each entry of the external component table.
fn external_control(pe: &PeImage<'_>, guid: &[u8; 16]) -> Option<([u8; 16], bool)> {
    let header = VbHeader::read(&header_region(pe).ok()?).ok()?;
    let mut at = header.lp_external_table.get();
    for _ in 0..header.w_external_count {
        let entry = pe.region_at_va(Va::new(at))?;
        let read =
            |offset: u32| -> Option<[u8; 16]> { entry.take(Off::new(offset), 16)?.try_into().ok() };
        let array = if read(EXTERNAL_CONTROL_AT).as_ref() == Some(guid) {
            Some(false)
        } else if read(EXTERNAL_ARRAY_AT).as_ref() == Some(guid) {
            Some(true)
        } else {
            None
        };
        if let Some(array) = array {
            return Some((read(EXTERNAL_INTERFACE_AT)?, array));
        }
        let length = entry.u32_le(Off::new(0)).filter(|length| *length > 0)?;
        at = at.checked_add(length)?;
    }
    None
}

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
    /// The interface of an object result, when the file gives it.
    pub result_interface: Option<String>,
    /// The `DISPID` of a late-bound call of the function, when the file
    /// gives it.
    pub dispid: Option<u32>,
}

/// A function of the runtime that an executable imports by its ordinal.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[non_exhaustive]
pub struct ImportType {
    /// The name of the function in its module, such as `Err`.
    pub name: String,
    /// The interface of an object result, when the file gives it.
    #[serde(default)]
    pub result_interface: Option<String>,
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

    /// Gives the functions whose `DISPID` is `dispid`.
    #[must_use]
    pub fn function_of_dispid(&self, dispid: u32) -> Option<&TypeFunction> {
        self.functions
            .values()
            .find(|function| function.dispid == Some(dispid))
    }
}

/// Gives the GUID of the events of the control of a control array whose
/// `ControlInfo` record names `guid`: `guid` less 1 in its first 32 bits.
fn array_element(guid: &[u8; 16]) -> Option<[u8; 16]> {
    let (first, _) = guid.split_first_chunk::<4>()?;
    let events = u32::from_le_bytes(*first).checked_sub(1)?;
    let mut single = *guid;
    let (head, _) = single.split_first_chunk_mut::<4>()?;
    *head = events.to_le_bytes();
    Some(single)
}

/// One event of an events interface.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[non_exhaustive]
pub struct EventType {
    /// The name, such as `Click`.
    pub name: String,
    /// The declaration of each parameter in Basic, such as
    /// `KeyAscii As Integer`, when the file gives each of them.
    #[serde(default)]
    pub parameters: Option<Vec<String>>,
}

/// The interfaces and the controls of the file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VbTypes {
    controls: BTreeMap<String, String>,
    events: BTreeMap<String, Vec<EventType>>,
    iids: BTreeMap<String, String>,
    imports: BTreeMap<u16, ImportType>,
    interfaces: BTreeMap<String, InterfaceType>,
}

/// The file, before it is folded into [`VbTypes`].
#[derive(serde::Deserialize)]
struct RawTypes {
    #[serde(default)]
    controls: BTreeMap<String, String>,
    #[serde(default)]
    events: BTreeMap<String, Vec<EventType>>,
    #[serde(default)]
    iids: BTreeMap<String, String>,
    #[serde(default)]
    imports: BTreeMap<String, ImportType>,
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
    #[serde(default)]
    result_interface: Option<String>,
    #[serde(default)]
    dispid: Option<u32>,
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
                        result_interface: function.result_interface,
                        dispid: function.dispid,
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
        let mut imports = BTreeMap::new();
        for (key, import) in raw.imports {
            let ordinal = key.parse::<u16>().map_err(|_| TableError {
                line: 1,
                message: format!("imports.{key} is not an ordinal"),
            })?;
            imports.insert(ordinal, import);
        }
        Ok(Self {
            controls: raw.controls,
            events: raw.events,
            iids: raw.iids,
            imports,
            interfaces,
        })
    }

    /// Gives the event at `slot` of an event table of `slots` slots, of the
    /// control whose `ControlInfo` record names `guid`.
    ///
    /// The events of the interface fill the last slots of the table, in
    /// their order. The table of an intrinsic control has no other slot,
    /// and the table of a control of an OCX has the slots of the extender
    /// before them. The events interface is the one that `guid` names, or
    /// for a control array the one of its control (see
    /// [`VbTypes::control_array_interface`]), or for a control of an OCX the
    /// one that its entry of the external component table names.
    #[must_use]
    pub fn event(
        &self,
        pe: &PeImage<'_>,
        guid: &[u8; 16],
        slot: u16,
        slots: u16,
    ) -> Option<&EventType> {
        let events = self
            .events
            .get(&guid_text(guid))
            .or_else(|| self.events.get(&guid_text(&array_element(guid)?)))
            .or_else(|| {
                self.events
                    .get(&guid_text(&external_guid(pe, guid, EXTERNAL_EVENTS_AT)?))
            })?;
        let first = slots.checked_sub(u16::try_from(events.len()).ok()?)?;
        events.get(usize::from(slot.checked_sub(first)?))
    }

    /// Gives the name of the interface of the control whose `ControlInfo`
    /// record names `guid`.
    #[must_use]
    pub fn control_interface(&self, guid: &[u8; 16]) -> Option<&str> {
        self.controls.get(&guid_text(guid)).map(String::as_str)
    }

    /// Gives the class of the object of a control array whose `ControlInfo`
    /// record names `guid`.
    ///
    /// The record of a control array names the GUID of the events of its
    /// control plus 1 in its first 32 bits, such as `{33AD4F03-...}` for an
    /// array of `OptionButton`, whose events have `{33AD4F02-...}`. This
    /// holds for each control array of the P-code corpus.
    #[must_use]
    pub fn control_array_interface(&self, guid: &[u8; 16]) -> Option<String> {
        self.control_interface(&array_element(guid)?)
            .map(|interface| format!("{CONTROL_ARRAY}{interface}"))
    }

    /// Gives the class of a control of an OCX whose `ControlInfo` record names
    /// `guid`: the interface that the external component table gives, as a
    /// control array when the record names the GUID of an array.
    fn external_interface(&self, pe: &PeImage<'_>, guid: &[u8; 16]) -> Option<String> {
        let (iid, array) = external_control(pe, guid)?;
        let interface = self.interface_of_iid(&iid)?;
        Some(if array {
            format!("{CONTROL_ARRAY}{interface}")
        } else {
            interface.to_owned()
        })
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
    /// control. A control array gets the class that
    /// [`VbTypes::control_array_interface`] gives. A control whose GUID names
    /// no interface of the file gets the interface that the external
    /// component table gives for it, when it is a control of an OCX, or else
    /// its accessor with no interface. Each control of an object that is not a
    /// form is left out. The
    /// interface of the form becomes the base interface of `callees`.
    #[must_use]
    pub fn with_controls(&self, callees: Callees, pe: &PeImage<'_>, object: &Object) -> Callees {
        let Ok(controls) = ControlInfoTable::read(pe, object) else {
            return callees;
        };
        let form = controls
            .entries
            .iter()
            .find(|control| control.w_index == FORM_RECORD_INDEX)
            .and_then(|form| self.control_interface(&Self::guid(pe, form)?));
        let Some((name, base)) = form.and_then(|name| {
            self.interface(name)
                .map(|interface| (name, u32::from(interface.vtable_size)))
        }) else {
            return callees;
        };
        let mut callees = callees.with_base(name);
        for control in &controls.entries {
            if control.w_index == FORM_RECORD_INDEX {
                continue;
            }
            let offset = u32::from(control.w_index)
                .checked_mul(4)
                .and_then(|bytes| bytes.checked_add(base))
                .and_then(|offset| u16::try_from(offset).ok());
            let interface = Self::guid(pe, control).and_then(|guid| {
                self.control_interface(&guid)
                    .map(str::to_owned)
                    .or_else(|| self.control_array_interface(&guid))
                    .or_else(|| self.external_interface(pe, &guid))
            });
            match (offset, interface) {
                (Some(offset), Some(interface)) => {
                    callees = callees.with_control(offset, &control.name, &interface);
                }
                (Some(offset), None) => {
                    callees = callees.with_untyped_control(offset, &control.name);
                }
                (None, _) => {}
            }
        }
        callees
    }

    /// Gives the name of the interface whose GUID is `guid`.
    #[must_use]
    pub fn interface_of_iid(&self, guid: &[u8; 16]) -> Option<&str> {
        self.iids.get(&guid_text(guid)).map(String::as_str)
    }

    /// Gives the function of the runtime whose export has the ordinal
    /// `ordinal`.
    #[must_use]
    pub fn import(&self, ordinal: u16) -> Option<&ImportType> {
        self.imports.get(&ordinal)
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

[imports.685]
name = "Err"
result_interface = "_Box"

[imports.595]
name = "MsgBox"

[interfaces._Box]
vtable_size = 48

[interfaces._Box.functions.0024]
names = ["Text"]
kinds = ["get"]
arg_bytes = 4
result = true
result_interface = "_Box"
dispid = 67

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
        let mut array = GUID;
        array[0] = 0xD3;
        assert_eq!(
            types.control_array_interface(&array).as_deref(),
            Some("[]_Box")
        );
        assert_eq!(types.control_array_interface(&GUID), None);
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
        assert_eq!(text.result_interface.as_deref(), Some("_Box"));
        assert_eq!(interface.function(0x28).unwrap().arg_bytes, None);
        assert_eq!(text.dispid, Some(0x43));
        assert_eq!(interface.function_of_dispid(0x43).unwrap().names, ["Text"]);
        assert!(interface.function_of_dispid(0x44).is_none());
        assert!(interface.function(0x2C).is_none());
        assert!(types.interface("_Other").is_none());
        let err = types.import(685).unwrap();
        assert_eq!(err.name, "Err");
        assert_eq!(err.result_interface.as_deref(), Some("_Box"));
        assert_eq!(types.import(595).unwrap().result_interface, None);
        assert!(types.import(1).is_none());
    }

    #[test]
    fn a_damaged_file_gives_an_error_with_its_line() {
        let bad_key = TYPES.replace("functions.0024", "functions.x024");
        assert!(VbTypes::parse(bad_key.as_bytes()).is_err());
        let bad_ordinal = TYPES.replace("imports.685", "imports.x685");
        assert!(VbTypes::parse(bad_ordinal.as_bytes()).is_err());
        let error = VbTypes::parse(b"[interfaces._Box]\nvtable_size = \"big\"\n").unwrap_err();
        assert_eq!(error.line, 2);
        assert!(VbTypes::parse(&[0xFF, 0xFE]).is_err());
    }

    /// A types file with the form and `IMSWinsockControl` by their GUIDs.
    const WINSOCK_TYPES: &str = r#"
[controls]
"{33AD4F3A-6699-11CF-B70C-00AA0060D393}" = "_Form0"

[iids]
"{248DD892-BB45-11CF-9ABC-0080C7E7B78D}" = "IMSWinsockControl"

[interfaces._Form0]
vtable_size = 760

[interfaces.IMSWinsockControl]
vtable_size = 128

[[events."{248DD893-BB45-11CF-9ABC-0080C7E7B78D}"]]
name = "Error"

[[events."{248DD893-BB45-11CF-9ABC-0080C7E7B78D}"]]
name = "DataArrival"
parameters = ["ByVal bytesTotal As Long"]

[[events."{33AD4ED2-6699-11CF-B70C-00AA0060D393}"]]
name = "Change"
parameters = []

[[events."{33AD4ED2-6699-11CF-B70C-00AA0060D393}"]]
name = "KeyPress"
parameters = ["KeyAscii As Integer"]
"#;

    /// Gives the callees of `Form1` of the P-code program at `path`.
    fn form1(bytes: &[u8]) -> crate::vb::lift::Callees {
        use crate::read::pe::PeImage;
        use crate::vb::header::{VbHeader, header_region};
        use crate::vb::object::ObjectTable;
        use crate::vb::project::{ObjectTableHead, ProjectInfo};
        let pe = PeImage::parse(bytes).unwrap();
        let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
        let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
        let head = ObjectTableHead::read(&pe, info.lp_object_table).unwrap();
        let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
            .unwrap()
            .objects;
        let form = objects.iter().find(|o| o.name == "Form1").unwrap();
        let types = VbTypes::parse(WINSOCK_TYPES.as_bytes()).unwrap();
        types.with_controls(crate::vb::lift::Callees::default(), &pe, form)
    }

    /// The Winsock control of `TFTPClient.exe` and the array of Winsock
    /// controls of `Server.exe` get `IMSWinsockControl`, through the GUIDs of
    /// the external component table.
    #[test]
    fn a_control_of_an_ocx_gets_the_interface_of_its_component() {
        let client = form1(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus-pcode/public-domain/SK-TFTP-Sample__VB6/Client/demo/TFTPClient.exe"
        )));
        assert_eq!(
            client.control(0x328),
            Some(("WskClient", Some("IMSWinsockControl")))
        );
        let server = form1(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus-pcode/public-domain/SK-TFTP-Sample__VB6/Server/demo/Server.exe"
        )));
        assert_eq!(
            server.control(0x308),
            Some(("WskServer", Some("[]IMSWinsockControl")))
        );
    }

    /// A slot names the event at its place among the last slots of the
    /// table: for a control, for a control array and for a control of an
    /// OCX, whose table has the slots of the extender first.
    #[test]
    fn a_slot_of_an_event_table_names_its_event() {
        use crate::read::pe::PeImage;
        use crate::vb::controlinfo::ControlInfoTable;
        use crate::vb::header::{VbHeader, header_region};
        use crate::vb::object::ObjectTable;
        use crate::vb::project::{ObjectTableHead, ProjectInfo};
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus-pcode/public-domain/SK-TFTP-Sample__VB6/Client/demo/TFTPClient.exe"
        ));
        let pe = PeImage::parse(bytes).unwrap();
        let types = VbTypes::parse(WINSOCK_TYPES.as_bytes()).unwrap();
        let name = |guid: &[u8; 16], slot: u16, slots: u16| {
            types
                .event(&pe, guid, slot, slots)
                .map(|event| event.name.as_str())
        };
        assert_eq!(name(&GUID, 0, 2), Some("Change"));
        assert_eq!(name(&GUID, 1, 2), Some("KeyPress"));
        assert_eq!(name(&GUID, 2, 2), None);
        assert_eq!(name(&GUID, 0, 3), None, "a slot before the events");
        assert_eq!(name(&GUID, 2, 3), Some("KeyPress"));
        assert_eq!(name(&GUID, 0, 1), None, "fewer slots than events");
        let mut array = GUID;
        array[0] = 0xD3;
        assert_eq!(name(&array, 1, 2), Some("KeyPress"));
        let key_press = types.event(&pe, &GUID, 1, 2).unwrap();
        assert_eq!(
            key_press.parameters.as_deref(),
            Some(&["KeyAscii As Integer".to_owned()][..])
        );
        assert_eq!(
            types.event(&pe, &GUID, 0, 2).unwrap().parameters,
            Some(Vec::new())
        );
        let header = VbHeader::read(&header_region(&pe).unwrap()).unwrap();
        let info = ProjectInfo::read(&pe, header.lp_project_data).unwrap();
        let head = ObjectTableHead::read(&pe, info.lp_object_table).unwrap();
        let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
            .unwrap()
            .objects;
        let form = objects.iter().find(|o| o.name == "Form1").unwrap();
        let controls = ControlInfoTable::read(&pe, form).unwrap();
        let winsock = controls
            .entries
            .iter()
            .find(|control| control.name == "WskClient")
            .unwrap();
        let guid = VbTypes::guid(&pe, winsock).unwrap();
        assert_eq!(name(&guid, 10, 11), Some("DataArrival"));
        assert_eq!(types.event(&pe, &guid, 9, 11).unwrap().parameters, None);
        assert_eq!(name(&guid, 8, 11), None, "a slot of the extender");
    }

    #[test]
    fn a_guid_has_the_registry_form() {
        assert_eq!(guid_text(&GUID), "{33AD4ED2-6699-11CF-B70C-00AA0060D393}");
    }
}
