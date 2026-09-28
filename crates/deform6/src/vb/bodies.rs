//! The lifted bodies of the procedures of a P-code program, for
//! `extract --lift`.
//!
//! [`lift_objects`] lifts each procedure of each object with
//! [`crate::vb::lift::lift_method`], and gives each one the declaration
//! line that `extract` writes above its body. The statements are the lift,
//! not the source: they name a variable by its offset, such as `local_88`,
//! and a branch by a label, such as `L0152`.
//!
//! # The names
//!
//! Each procedure takes the name that `extract` writes for it: the public
//! name that the file gives, the name of the event that it handles, or
//! `UnnamedProcedure` and its index. A call of it takes the same name,
//! through [`callees_of_project_named`].
//!
//! # The declaration
//!
//! A public procedure with a prototype and an event handler take the
//! declaration that `extract` writes for them, and the lift names each of
//! their arguments by its name. The frame slot of an argument follows from
//! the sizes of the arguments before it: 4 bytes for an argument by
//! reference, and the size of the value for an argument by value.
//!
//! Any other procedure takes a declaration that the lift builds from its
//! descriptor. `arg_size` gives the bytes of its arguments, with 4 for `Me`
//! and 4 for the address of the result of a `Function`. An exit opcode whose
//! name starts with `ExitProcCb` marks a `Function`. Each slot of 4 bytes
//! is a `Variant` by reference. The address of the result of a `Function`
//! is the last argument of a procedure of an object, and the first argument
//! of a procedure of a standard module. The value that a `Function` returns
//! is in a frame slot, which takes the name of the `Function`, as Basic
//! writes it.
//!
//! # The variables of an object
//!
//! A field of `Me` that a body names, such as `field_54`, is a variable of
//! the object. Each one gets a `Private` declaration of a `Variant`.
//!
//! A variable of a module takes the name of its address, such as
//! `g_40A1C0`, in each object that names it. The first standard module of
//! the project declares each one `Public`. A project with no standard module
//! declares each one `Private` in each object that names it.

use std::collections::BTreeSet;

use crate::error::Refusal;
use crate::read::pe::PeImage;
use crate::vb::context::callees_of_project_named;
use crate::vb::functyp::{Prototype, TypeEntry, VbType};
use crate::vb::header::{VbHeader, header_region};
use crate::vb::lift::{lift_method, render};
use crate::vb::object::ObjectTable;
use crate::vb::pcode::{PcodeListing, PcodeTable, disassemble};
use crate::vb::procdesc::{MethodEntry, read_method_table};
use crate::vb::project::{ObjectTableHead, ProjectInfo};
use crate::vb::types::VbTypes;
use crate::vb::{ObjectProcedures, ProcedureEntry, Report};

/// One lifted procedure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LiftedProcedure {
    /// The index of the procedure in the method table of its object.
    pub index: u16,
    /// The declaration line, such as `Private Sub cmdOK_Click()`.
    pub declaration: String,
    /// The lines of the body.
    pub lines: Vec<String>,
    /// The closing line, such as `End Sub`.
    pub closing: &'static str,
}

/// The lifted procedures of one object, and the declarations of its
/// variables.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct LiftedObject {
    /// The declaration of each variable of the object, such as
    /// `Private field_54 As Variant`.
    pub declarations: Vec<String>,
    /// Each procedure, by ascending index.
    pub procedures: Vec<LiftedProcedure>,
}

/// The first frame offset of an argument: after the saved `ebp`, the return
/// address and `Me`.
const FIRST_ARGUMENT: u16 = 0x0C;

/// The name that `extract` writes for a procedure that has no name in the
/// file.
fn generated_name(index: u16) -> String {
    format!("UnnamedProcedure{index}")
}

/// Gives the name of the procedure at `index` of `procedures`.
fn procedure_name(procedures: &ObjectProcedures, index: u16) -> String {
    match procedures {
        ObjectProcedures::Slots(slots) => match slots.get(usize::from(index)) {
            Some(ProcedureEntry::Public { name, .. } | ProcedureEntry::Handler { name, .. }) => {
                name.clone()
            }
            Some(ProcedureEntry::Private) | None => generated_name(index),
        },
        ObjectProcedures::NoNameArray { .. } => generated_name(index),
    }
}

/// Tells whether `character` can be part of a Basic name.
const fn is_name(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// Gives `line` with each whole word `from` replaced with `to`. A word is
/// whole when no name character and no `.` comes before it, and no name
/// character comes after it.
fn replace_word(line: &str, from: &str, to: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    let mut before: Option<char> = None;
    while !rest.is_empty() {
        let after = rest.get(from.len()..).and_then(|tail| tail.chars().next());
        if rest.starts_with(from)
            && !before.is_some_and(|before| is_name(before) || before == '.')
            && !after.is_some_and(is_name)
        {
            out.push_str(to);
            rest = rest.get(from.len()..).unwrap_or_default();
            before = to.chars().last();
            continue;
        }
        let Some(character) = rest.chars().next() else {
            break;
        };
        out.push(character);
        before = Some(character);
        rest = rest.get(character.len_utf8()..).unwrap_or_default();
    }
    out
}

/// Gives each word of `lines` that starts with `prefix` and goes on with
/// hexadecimal digits, and that no name character and no `.` comes before.
fn words_with(lines: &[String], prefix: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in lines {
        let characters: Vec<char> = line.chars().collect();
        let mut at = 0_usize;
        while at < characters.len() {
            let before = at
                .checked_sub(1)
                .and_then(|before| characters.get(before))
                .copied();
            let tail: String = characters.get(at..).unwrap_or_default().iter().collect();
            if tail.starts_with(prefix) && !before.is_some_and(|c| is_name(c) || c == '.') {
                let digits: String = tail
                    .chars()
                    .skip(prefix.len())
                    .take_while(char::is_ascii_hexdigit)
                    .collect();
                let next = tail.chars().nth(prefix.len().saturating_add(digits.len()));
                if !digits.is_empty() && !next.is_some_and(is_name) {
                    out.insert(format!("{prefix}{digits}"));
                }
                at = at.saturating_add(prefix.len().saturating_add(digits.len()).max(1));
                continue;
            }
            at = at.saturating_add(1);
        }
    }
    out
}

/// The bytes that an argument of `entry` takes on the stack.
const fn entry_bytes(entry: &TypeEntry) -> u16 {
    if entry.by_ref || entry.array {
        return 4;
    }
    match entry.vb_type {
        VbType::Variant => 16,
        VbType::Double | VbType::Currency | VbType::Date => 8,
        _ => 4,
    }
}

/// The bytes that an argument of the declaration `declared`, such as
/// `ByVal X As Single`, takes on the stack, and its name.
fn declared_bytes(declared: &str) -> Option<(u16, String)> {
    let (by_value, rest) = match declared.strip_prefix("ByVal ") {
        Some(rest) => (true, rest),
        None => (false, declared),
    };
    let (name, basic) = rest.split_once(" As ")?;
    let bytes = match (by_value, basic) {
        (false, _) => 4,
        (true, "Variant") => 16,
        (true, "Double" | "Currency" | "Date") => 8,
        (true, _) => 4,
    };
    Some((bytes, name.to_owned()))
}

/// Gives the slot of each argument of sizes `bytes`, from the first slot.
fn slots(bytes: &[u16]) -> Vec<u16> {
    let mut at = FIRST_ARGUMENT;
    let mut out = Vec::new();
    for size in bytes {
        out.push(at);
        at = at.saturating_add(*size);
    }
    out
}

/// The size of the value that a `Function` returns when its exit opcode is
/// `ExitProcCb`, which gives no size: a `Variant`. Both such procedures of
/// the corpus return a `Variant`.
const VARIANT_BYTES: u16 = 16;

/// Gives the bytes of the value that `listing` returns, or `None` for a
/// procedure that returns none. The exit opcode `ExitProcCbHresult` gives
/// them in its second word.
fn result_bytes(listing: &PcodeListing, table: &PcodeTable) -> Option<u16> {
    listing.instructions.iter().find_map(|instruction| {
        let names = &table.slot(instruction.lead, instruction.opcode)?.names;
        if names.iter().any(|name| name == "ExitProcCbHresult") {
            let low = *instruction.arguments.get(2)?;
            let high = *instruction.arguments.get(3)?;
            Some(u16::from_le_bytes([low, high]))
        } else if names.iter().any(|name| name.starts_with("ExitProcCb")) {
            Some(VARIANT_BYTES)
        } else {
            None
        }
    })
}

/// The frame offset below which a `Function` keeps the value that it
/// returns: the value of `n` bytes is at `local_` 0x84 plus `n`, and a value
/// of 1 byte takes 2.
const RESULT_BASE: u16 = 0x84;

/// Builds the parameters of a procedure with no prototype: one argument of
/// 4 bytes in each slot from `first` to `end`, by reference. An argument of
/// 8 or 16 bytes by value is not told apart from two or four arguments of 4
/// bytes, so the lift takes the arguments of 4 bytes.
fn built_parameters(first: u16, end: u16) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = first;
    while at < end {
        out.push(format!("arg_{at:X} As Variant"));
        at = at.saturating_add(4);
    }
    out
}

/// Gives the declaration and the closing line of the procedure at `index`,
/// and renames the arguments and the result in `lines`.
fn declare(
    procedures: &ObjectProcedures,
    index: u16,
    arg_size: u16,
    result: Option<u16>,
    lines: &mut Vec<String>,
) -> (String, &'static str) {
    let name = procedure_name(procedures, index);
    let (entry, module) = match procedures {
        ObjectProcedures::Slots(slots) => (slots.get(usize::from(index)), false),
        ObjectProcedures::NoNameArray { .. } => (None, true),
    };
    let rename = |lines: &mut Vec<String>, from: &str, to: &str| {
        for line in lines.iter_mut() {
            *line = replace_word(line, from, to);
        }
    };
    if let Some(bytes) = result {
        let slot = RESULT_BASE.saturating_add(bytes.max(2));
        rename(lines, &format!("local_{slot:X}"), &name);
    }
    let function = result.is_some();
    match entry {
        Some(ProcedureEntry::Public {
            prototype: Some(prototype),
            ..
        }) => {
            let signature = crate::write::code::format_signature("Public", &name, Some(prototype));
            rename_prototype(prototype, lines, &rename);
            (signature.declaration, signature.closing)
        }
        Some(ProcedureEntry::Handler { parameters, .. }) => {
            let declared: Vec<(u16, String)> = parameters
                .iter()
                .filter_map(|p| declared_bytes(p))
                .collect();
            let sizes: Vec<u16> = declared.iter().map(|(bytes, _)| *bytes).collect();
            for (slot, (_, own)) in slots(&sizes).iter().zip(&declared) {
                rename(lines, &format!("arg_{slot:X}"), own);
            }
            (
                format!("Private Sub {name}({})", parameters.join(", ")),
                "End Sub",
            )
        }
        _ => {
            let scope = match entry {
                Some(ProcedureEntry::Private) => "Private",
                _ => "Public",
            };
            let end = FIRST_ARGUMENT.saturating_add(arg_size.saturating_sub(4));
            let (first, end) = match (function, module) {
                (true, true) => (FIRST_ARGUMENT.saturating_add(4), end),
                (true, false) => (FIRST_ARGUMENT, end.saturating_sub(4)),
                (false, _) => (FIRST_ARGUMENT, end),
            };
            let list = built_parameters(first, end);
            if function {
                (
                    format!("{scope} Function {name}({}) As Variant", list.join(", ")),
                    "End Function",
                )
            } else {
                (
                    format!("{scope} Sub {name}({})", list.join(", ")),
                    "End Sub",
                )
            }
        }
    }
}

/// Renames the arguments of `prototype` in `lines`.
fn rename_prototype(
    prototype: &Prototype,
    lines: &mut Vec<String>,
    rename: &dyn Fn(&mut Vec<String>, &str, &str),
) {
    let sizes: Vec<u16> = prototype
        .arguments
        .iter()
        .map(|argument| entry_bytes(&argument.entry))
        .collect();
    for (slot, argument) in slots(&sizes).iter().zip(&prototype.arguments) {
        rename(lines, &format!("arg_{slot:X}"), &argument.name);
    }
}

/// Lifts each procedure of each object of the P-code program `data`, whose
/// report is `report`. Gives one entry for each object of `report.objects`,
/// in the same order: `None` for an object with no method table.
///
/// A procedure whose lift stops gives a comment line that names the fault,
/// and no statement.
///
/// # Errors
///
/// Returns a [`Refusal`] when the structures that [`crate::inspect`]
/// already read do not read again.
pub fn lift_objects(
    data: &[u8],
    report: &Report,
    table: &PcodeTable,
    types: Option<&VbTypes>,
) -> Result<Vec<Option<LiftedObject>>, Refusal> {
    let pe = PeImage::parse(data)?;
    let header = VbHeader::read(&header_region(&pe)?)?;
    let info = ProjectInfo::read(&pe, header.lp_project_data)?;
    let head = ObjectTableHead::read(&pe, info.lp_object_table)?;
    let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)?.objects;
    let procedures: Vec<Option<&ObjectProcedures>> = objects
        .iter()
        .map(|object| {
            report
                .objects
                .iter()
                .find(|own| own.name == object.name)
                .map(|own| &own.procedures)
        })
        .collect();
    let names: Vec<Vec<(u16, String)>> = objects
        .iter()
        .zip(&procedures)
        .map(|(object, own)| {
            let Ok(methods) = read_method_table(&pe, object.lp_object_info) else {
                return Vec::new();
            };
            methods
                .entries
                .iter()
                .filter_map(|entry| match (entry, own) {
                    (MethodEntry::Descriptor { index, .. }, Some(own)) => {
                        Some((*index, procedure_name(own, *index)))
                    }
                    _ => None,
                })
                .collect()
        })
        .collect();
    let callees = callees_of_project_named(&pe, &objects, table, types, &names);
    let mut out: Vec<Option<LiftedObject>> = Vec::new();
    let mut variables: Vec<BTreeSet<String>> = Vec::new();
    for own in &report.objects {
        let Some(at) = objects.iter().position(|object| object.name == own.name) else {
            out.push(None);
            variables.push(BTreeSet::new());
            continue;
        };
        let (Some(object), Some(callees)) = (objects.get(at), callees.get(at)) else {
            out.push(None);
            variables.push(BTreeSet::new());
            continue;
        };
        let Ok(methods) = read_method_table(&pe, object.lp_object_info) else {
            out.push(None);
            variables.push(BTreeSet::new());
            continue;
        };
        let mut lifted = LiftedObject::default();
        let mut globals = BTreeSet::new();
        let mut fields = BTreeSet::new();
        for entry in &methods.entries {
            let MethodEntry::Descriptor { index, descriptor } = entry else {
                continue;
            };
            let Some(body) = descriptor.body(&pe) else {
                continue;
            };
            let listing = disassemble(&body, table);
            let mut lines = match lift_method(&listing, table, callees, types, *index) {
                Ok(stmts) => render(&stmts),
                Err(fault) => vec![format!("    ' The lift stopped: {fault:?}")],
            };
            fields.extend(words_with(&lines, "field_"));
            globals.extend(words_with(&lines, "g_"));
            let result = result_bytes(&listing, table);
            let (declaration, closing) = declare(
                &own.procedures,
                *index,
                descriptor.arg_size,
                result,
                &mut lines,
            );
            lifted.procedures.push(LiftedProcedure {
                index: *index,
                declaration,
                lines,
                closing,
            });
        }
        lifted.declarations = fields
            .iter()
            .map(|field| format!("Private {field} As Variant"))
            .collect();
        out.push(Some(lifted));
        variables.push(globals);
    }
    let home = report
        .objects
        .iter()
        .position(|object| matches!(object.procedures, ObjectProcedures::NoNameArray { .. }));
    let all: BTreeSet<String> = variables.iter().flatten().cloned().collect();
    for (at, (lifted, own)) in out.iter_mut().zip(&variables).enumerate() {
        let Some(lifted) = lifted else {
            continue;
        };
        match home {
            Some(home) if home == at => lifted
                .declarations
                .extend(all.iter().map(|name| format!("Public {name} As Variant"))),
            Some(_) => {}
            None => lifted
                .declarations
                .extend(own.iter().map(|name| format!("Private {name} As Variant"))),
        }
    }
    Ok(out)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{
        built_parameters, declare, declared_bytes, procedure_name, replace_word, result_bytes,
        words_with,
    };
    use crate::read::region::{Off, Region};
    use crate::vb::pcode::{PcodeTable, disassemble};
    use crate::vb::{ObjectProcedures, ProcedureEntry};

    #[test]
    fn a_word_is_replaced_only_where_it_is_whole() {
        assert_eq!(
            replace_word("arg_C = arg_C0 + x.arg_C + arg_C", "arg_C", "Index"),
            "Index = arg_C0 + x.arg_C + Index"
        );
        assert_eq!(replace_word("(arg_10)", "arg_10", "X"), "(X)");
    }

    #[test]
    fn the_words_of_a_prefix_are_the_whole_hexadecimal_ones() {
        let lines = vec![
            "field_54 = local_88.field_C + g_40A1C0".to_owned(),
            "field_6E = field_54 + field_ + xfield_1".to_owned(),
        ];
        assert_eq!(
            words_with(&lines, "field_").into_iter().collect::<Vec<_>>(),
            ["field_54", "field_6E"]
        );
        assert_eq!(
            words_with(&lines, "g_").into_iter().collect::<Vec<_>>(),
            ["g_40A1C0"]
        );
    }

    #[test]
    fn a_declared_argument_takes_the_bytes_of_how_it_is_passed() {
        assert_eq!(
            declared_bytes("KeyAscii As Integer"),
            Some((4, "KeyAscii".to_owned()))
        );
        assert_eq!(
            declared_bytes("ByVal X As Single"),
            Some((4, "X".to_owned()))
        );
        assert_eq!(
            declared_bytes("ByVal V As Variant"),
            Some((16, "V".to_owned()))
        );
        assert_eq!(
            declared_bytes("ByVal D As Double"),
            Some((8, "D".to_owned()))
        );
        assert_eq!(declared_bytes("Odd"), None);
    }

    #[test]
    fn each_slot_of_4_bytes_is_a_parameter() {
        assert_eq!(
            built_parameters(0x10, 0x1C),
            [
                "arg_10 As Variant",
                "arg_14 As Variant",
                "arg_18 As Variant"
            ]
        );
        assert!(built_parameters(0x0C, 0x0C).is_empty());
    }

    /// A table with `ExitProcHresult`, `ExitProcCbHresult` with 4 argument
    /// bytes, and `ExitProcCb` with 2.
    const TABLE: &str = r#"
[primary.0C]
width = 0
names = ["ExitProcHresult"]
[primary.0D]
width = 4
names = ["ExitProcCbHresult"]
[primary.0E]
width = 2
names = ["ExitProcCb"]
"#;

    #[test]
    fn the_exit_opcode_gives_the_bytes_of_the_result() {
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let bytes = |body: &[u8]| {
            result_bytes(
                &disassemble(&Region::new(body, Off::new(0)), &table),
                &table,
            )
        };
        assert_eq!(bytes(&[0x0C]), None);
        assert_eq!(bytes(&[0x0D, 0x10, 0x00, 0x04, 0x00]), Some(4));
        assert_eq!(bytes(&[0x0E, 0x10, 0x00]), Some(16));
    }

    #[test]
    fn a_procedure_takes_the_name_that_extract_writes() {
        let slots = ObjectProcedures::Slots(vec![
            ProcedureEntry::Private,
            ProcedureEntry::Handler {
                name: "Form_Load".to_owned(),
                parameters: Vec::new(),
            },
        ]);
        assert_eq!(procedure_name(&slots, 0), "UnnamedProcedure0");
        assert_eq!(procedure_name(&slots, 1), "Form_Load");
        let module = ObjectProcedures::NoNameArray { proc_count: 2 };
        assert_eq!(procedure_name(&module, 1), "UnnamedProcedure1");
    }

    #[test]
    fn a_declaration_follows_the_kind_and_the_result_of_the_procedure() {
        let slots = ObjectProcedures::Slots(vec![
            ProcedureEntry::Private,
            ProcedureEntry::Handler {
                name: "Form_KeyPress".to_owned(),
                parameters: vec!["KeyAscii As Integer".to_owned()],
            },
        ]);
        let mut lines = vec!["local_88 = arg_C".to_owned()];
        assert_eq!(
            declare(&slots, 0, 12, Some(4), &mut lines),
            (
                "Private Function UnnamedProcedure0(arg_C As Variant) As Variant".to_owned(),
                "End Function"
            )
        );
        assert_eq!(lines, ["UnnamedProcedure0 = arg_C"]);
        let mut byte = vec!["local_86 = 1".to_owned(), "local_85 = 2".to_owned()];
        let _ = declare(&slots, 0, 8, Some(1), &mut byte);
        assert_eq!(byte, ["UnnamedProcedure0 = 1", "local_85 = 2"]);
        let mut handler = vec!["local_88 = arg_C".to_owned()];
        assert_eq!(
            declare(&slots, 1, 8, None, &mut handler),
            (
                "Private Sub Form_KeyPress(KeyAscii As Integer)".to_owned(),
                "End Sub"
            )
        );
        assert_eq!(handler, ["local_88 = KeyAscii"]);
        let module = ObjectProcedures::NoNameArray { proc_count: 1 };
        let mut body = vec!["local_94 = arg_10".to_owned()];
        assert_eq!(
            declare(&module, 0, 16, Some(16), &mut body),
            (
                "Public Function UnnamedProcedure0(arg_10 As Variant, arg_14 As Variant) As Variant"
                    .to_owned(),
                "End Function"
            )
        );
        assert_eq!(body, ["UnnamedProcedure0 = arg_10"]);
        let mut sub = Vec::new();
        assert_eq!(
            declare(&module, 0, 12, None, &mut sub).0,
            "Public Sub UnnamedProcedure0(arg_C As Variant, arg_10 As Variant)"
        );
    }
}
