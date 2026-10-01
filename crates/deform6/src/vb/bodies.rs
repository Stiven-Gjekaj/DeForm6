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
//! their arguments by its name. Each argument by reference and each array
//! of a prototype is a `Variant` by reference, because a lifted caller passes
//! a `Variant` local. The frame slot of an argument follows from
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
//! Each procedure of a DLL that a body calls gets a `Declare` statement in
//! its object, by its export name and its library. The file holds no type
//! of an argument, so each one is `ByRef As Any`, and the result is a
//! `Long`.
//!
//! A field of `Me` that a body names, such as `field_54`, is a variable of
//! the object. Each one gets a `Private` declaration of a `Variant`.
//!
//! A variable of a module takes the name of its address, such as
//! `g_40A1C0`, in each object that names it. The first standard module of
//! the project declares each one `Public`. A project with no standard module
//! declares each one `Private` in each object that names it.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::Refusal;
use crate::read::pe::PeImage;
use crate::vb::context::callees_of_project_named;
use crate::vb::functyp::{Prototype, TypeEntry, VbType};
use crate::vb::header::{VbHeader, header_region};
use crate::vb::lift::{Callees, lift_method, render, result_bytes};
use crate::vb::object::ObjectTable;
use crate::vb::pcode::{PcodeListing, PcodeTable, disassemble};
use crate::vb::procdesc::{FixedArray, MethodEntry, module_fixed_arrays, read_method_table};
use crate::vb::project::{Declaration, DeclareTable, ExportName, ObjectTableHead, ProjectInfo};
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
    words_before(lines, prefix, |next| !next.is_some_and(is_name))
}

/// Gives each word of [`words_with`] that an index follows: the arrays of
/// `lines`.
fn indexed_words(lines: &[String], prefix: &str) -> BTreeSet<String> {
    words_before(lines, prefix, |next| next == Some('('))
}

/// Gives the declaration of each field: `Public` for a field that has an
/// accessor of a public variable in `public`, which another object can
/// use, and `Private` for each other field in `used`. A field in `fixed` is
/// a fixed-size array with that shape after its name.
fn field_declarations(
    used: &BTreeSet<String>,
    public: &BTreeSet<u32>,
    arrays: &BTreeMap<String, &'static str>,
    fixed: &BTreeMap<String, String>,
) -> Vec<String> {
    let declaration = |scope: &str, field: &str| match fixed.get(field) {
        Some(shape) => format!("{scope} {field}{shape}"),
        None => variable_declaration(scope, field, arrays),
    };
    let public: BTreeSet<String> = public
        .iter()
        .map(|field| format!("field_{field:X}"))
        .collect();
    let mut out: Vec<String> = public
        .iter()
        .map(|field| declaration("Public", field))
        .collect();
    out.extend(
        used.iter()
            .filter(|field| !public.contains(*field))
            .map(|field| declaration("Private", field)),
    );
    out
}

/// Gives the declaration of the variable `name` with the scope `scope`: an
/// array of the type that `arrays` gives it, or else a `Variant`.
fn variable_declaration(
    scope: &str,
    name: &str,
    arrays: &BTreeMap<String, &'static str>,
) -> String {
    match arrays.get(name) {
        Some(element) => format!("{scope} {name}() As {element}"),
        None => format!("{scope} {name} As Variant"),
    }
}

/// The Basic type of the elements of an array that `Redim` sizes, by the
/// `VARTYPE` of its arguments.
const fn redim_element(vartype: u16) -> Option<&'static str> {
    match vartype {
        2 => Some("Integer"),
        3 => Some("Long"),
        4 => Some("Single"),
        5 => Some("Double"),
        6 => Some("Currency"),
        7 => Some("Date"),
        8 => Some("String"),
        11 => Some("Boolean"),
        12 => Some("Variant"),
        17 => Some("Byte"),
        _ => None,
    }
}

/// The flag of the `SAFEARRAY` flags of a `Redim` that tells that its
/// `VARTYPE` is valid: `FADF_HAVEVARTYPE`.
const HAVE_VARTYPE: u16 = 0x80;

/// Gives the element type of each array that a `ReDim` of `lines` sizes.
/// The arguments of each `Redim` or `RedimPreserve` of `listing` are the
/// number of dimensions, the `VARTYPE` of the elements, their bytes, and the
/// flags of the `SAFEARRAY`. The opcodes and the `ReDim` lines have the same
/// order; when their counts differ, this gives nothing.
fn redim_arrays(
    listing: &PcodeListing,
    table: &PcodeTable,
    lines: &[String],
) -> BTreeMap<String, &'static str> {
    let elements: Vec<Option<&'static str>> = listing
        .instructions
        .iter()
        .filter(|instruction| {
            table
                .slot(instruction.lead, instruction.opcode)
                .is_some_and(|slot| {
                    slot.names
                        .iter()
                        .any(|name| name == "Redim" || name == "RedimPreserve")
                })
        })
        .map(|instruction| {
            let word = |at: usize| {
                instruction
                    .arguments
                    .get(at..at.checked_add(2)?)
                    .and_then(|bytes| bytes.try_into().ok())
                    .map(u16::from_le_bytes)
            };
            let flags = word(6)?;
            (flags & HAVE_VARTYPE != 0)
                .then_some(())
                .and_then(|()| redim_element(word(2)?))
        })
        .collect();
    let targets: Vec<String> = lines
        .iter()
        .filter_map(|line| {
            let text = line.trim_start();
            let text = match text.split_once(": ") {
                Some((label, rest)) if label.starts_with('L') && label.len() == 5 => rest,
                _ => text,
            };
            let rest = text
                .strip_prefix("ReDim Preserve ")
                .or_else(|| text.strip_prefix("ReDim "))?;
            rest.split_once('(').map(|(name, _)| name.to_owned())
        })
        .collect();
    let mut out = BTreeMap::new();
    if targets.len() == elements.len() {
        for (target, element) in targets.into_iter().zip(elements) {
            if let Some(element) = element {
                out.entry(target).or_insert(element);
            }
        }
    }
    out
}

/// How a body uses one frame slot, from the names of the handlers of the
/// opcodes that name it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SlotUse {
    /// An opcode takes the address of the slot.
    address: bool,
    /// The bytes and the Basic type of each load and store of a number in
    /// the slot.
    number: Option<(u8, &'static str)>,
    /// An opcode uses the slot in another way, or as another number.
    other: bool,
}

/// The loads and stores of a number, in a frame slot or in a field through
/// a frame slot, with their bytes and their Basic type. The handler of an
/// opcode can serve more than one name: `FLdAd`, `FLdI4`, `FLdR4` and
/// `FLdStr` share one, which copies 4 bytes.
const NUMBER_ACCESS: &[(&str, u8, &str)] = &[
    ("FLdAd", 4, "Long"),
    ("FLdI4", 4, "Long"),
    ("FLdR4", 4, "Long"),
    ("FLdStr", 4, "Long"),
    ("ILdRf", 4, "Long"),
    ("FStI4", 4, "Long"),
    ("FStR4", 4, "Long"),
    ("FLdI2", 2, "Integer"),
    ("FStI2", 2, "Integer"),
    ("FLdUI1", 1, "Byte"),
    ("FStUI1", 1, "Byte"),
    ("FLdCy", 8, "Double"),
    ("FLdR8", 8, "Double"),
    ("FStCy", 8, "Double"),
    ("FStR8", 8, "Double"),
    ("FLdFPR4", 4, "Single"),
    ("FStFPR4", 4, "Single"),
    ("FLdFPR8", 8, "Double"),
    ("FStFPR8", 8, "Double"),
    ("FMemLdAd", 4, "Long"),
    ("FMemLdI4", 4, "Long"),
    ("FMemLdR4", 4, "Long"),
    ("FMemLdStr", 4, "Long"),
    ("FMemStI4", 4, "Long"),
    ("FMemStR4", 4, "Long"),
    ("FMemLdI2", 2, "Integer"),
    ("FMemStI2", 2, "Integer"),
    ("FMemLdUI1", 1, "Byte"),
    ("FMemStUI1", 1, "Byte"),
    ("FMemLdCy", 8, "Double"),
    ("FMemLdR8", 8, "Double"),
    ("FMemStCy", 8, "Double"),
    ("FMemStR8", 8, "Double"),
    ("FMemLdFPR4", 4, "Single"),
    ("FMemStFPR4", 4, "Single"),
    ("FMemLdFPR8", 8, "Double"),
    ("FMemStFPR8", 8, "Double"),
    ("MemLdAd", 4, "Long"),
    ("MemLdI4", 4, "Long"),
    ("MemLdR4", 4, "Long"),
    ("MemLdStr", 4, "Long"),
    ("MemStI4", 4, "Long"),
    ("MemStR4", 4, "Long"),
    ("MemLdI2", 2, "Integer"),
    ("MemStI2", 2, "Integer"),
    ("MemLdUI1", 1, "Byte"),
    ("MemStUI1", 1, "Byte"),
    ("MemLdCy", 8, "Double"),
    ("MemLdR8", 8, "Double"),
    ("MemStCy", 8, "Double"),
    ("MemStR8", 8, "Double"),
    ("MemLdFPR4", 4, "Single"),
    ("MemStFPR4", 4, "Single"),
    ("MemLdFPR8", 8, "Double"),
    ("MemStFPR8", 8, "Double"),
];

/// The prefixes of the names of the opcodes whose first argument is a frame
/// slot.
const FRAME_ACCESS: &[&str] = &["FLd", "FSt", "FFree1", "FMem", "FDup", "FCopy"];

/// Gives the number that the opcode with the handler names `names` loads or
/// stores: its bytes and its Basic type, when each name gives the same bytes.
/// The type is the type of the first name that the bytes do not decide.
fn number_access(names: &[String]) -> Option<(u8, &'static str)> {
    let found: Vec<(u8, &'static str)> = names
        .iter()
        .map(|name| {
            NUMBER_ACCESS
                .iter()
                .find(|(known, _, _)| known == name)
                .map(|(_, width, basic)| (*width, *basic))
        })
        .collect::<Option<_>>()?;
    let (width, basic) = *found.first()?;
    found
        .iter()
        .all(|(other, _)| *other == width)
        .then_some((width, basic))
}

/// The first argument of `arguments` as a frame slot of a local: its offset
/// below the frame.
fn local_slot(arguments: &[u8]) -> Option<u16> {
    let offset = i16::from_le_bytes(arguments.get(..2)?.try_into().ok()?);
    (offset < 0).then(|| offset.unsigned_abs())
}

/// One opcode of a body: the names of its handler and its arguments.
type Op<'a> = (&'a [String], &'a [u8]);

/// Gives how the opcodes `ops` use each frame slot of a local, by its offset
/// below the frame.
fn slot_uses(ops: &[Op<'_>]) -> BTreeMap<u16, SlotUse> {
    let mut out: BTreeMap<u16, SlotUse> = BTreeMap::new();
    for (names, arguments) in ops {
        if !names
            .iter()
            .any(|name| FRAME_ACCESS.iter().any(|prefix| name.starts_with(prefix)))
        {
            continue;
        }
        let Some(slot) = local_slot(arguments) else {
            continue;
        };
        let entry = out.entry(slot).or_default();
        if names.iter().any(|name| name == "FLdRf") {
            entry.address = true;
            continue;
        }
        let is_field = names.iter().any(|name| name.starts_with("FMem"));
        match number_access(names) {
            Some(number) if !is_field && entry.number.is_none_or(|known| known.0 == number.0) => {
                entry.number.get_or_insert(number);
            }
            _ => entry.other = true,
        }
    }
    out
}

/// Gives each frame slot of `ops` that holds the address of another, with
/// that other slot: an `FLdRf` of the struct, then an `FStI4` of the slot.
/// Basic does this for a `With` block on a struct.
fn struct_pointers(ops: &[Op<'_>]) -> BTreeMap<u16, u16> {
    let mut out = BTreeMap::new();
    for pair in ops.windows(2) {
        let [(first, from), (second, to)] = pair else {
            continue;
        };
        if first.iter().any(|name| name == "FLdRf")
            && second.iter().any(|name| name == "FStI4")
            && let (Some(target), Some(pointer)) = (local_slot(from), local_slot(to))
        {
            out.insert(pointer, target);
        }
    }
    out
}

/// Gives the fields that `ops` read and write through each pointer of
/// `pointers`, by the slot of the struct: the offset in the struct, the
/// bytes and the Basic type. A struct with a field access that is not a
/// number is in the second set.
#[allow(
    clippy::type_complexity,
    reason = "a map of fields and a set, built in one pass"
)]
fn pointer_fields(
    ops: &[Op<'_>],
    pointers: &BTreeMap<u16, u16>,
) -> (BTreeMap<u16, Vec<(u16, u8, &'static str)>>, BTreeSet<u16>) {
    let mut fields: BTreeMap<u16, Vec<(u16, u8, &'static str)>> = BTreeMap::new();
    let mut bad = BTreeSet::new();
    for (names, arguments) in ops {
        if !names.iter().any(|name| name.starts_with("FMem")) {
            continue;
        }
        let Some(target) = local_slot(arguments).and_then(|slot| pointers.get(&slot)) else {
            continue;
        };
        let offset = arguments
            .get(2..4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes);
        match (number_access(names), offset) {
            (Some((width, basic)), Some(offset)) => {
                fields
                    .entry(*target)
                    .or_default()
                    .push((offset, width, basic));
            }
            _ => {
                bad.insert(*target);
            }
        }
    }
    (fields, bad)
}

/// A struct of the frame: a local whose address a call of a DLL takes, and
/// the fields after it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FrameStruct {
    /// The offset of its first byte below the frame.
    start: u16,
    /// Its bytes.
    size: u16,
    /// The offset in the struct, the bytes and the Basic type of each field,
    /// in the order of the offsets.
    fields: Vec<(u16, u8, &'static str)>,
}

/// Finds the structs of a frame. A local of `uses` whose address a call of
/// a DLL takes (`passed`), and that the body does not load or store as a
/// value, starts a struct. The struct holds each slot after it that the body
/// loads and stores only as a number, up to the next slot that it uses in
/// another way, the slot of the result `result`, or the end of the locals.
/// A field that a pointer of a `With` block names, in `through`, is a field
/// too. Basic lays out a struct of the source this way: the call writes its
/// fields, and the body reads them as frame slots.
fn frame_structs(
    uses: &BTreeMap<u16, SlotUse>,
    passed: &BTreeSet<u16>,
    result: Option<u16>,
    through: &BTreeMap<u16, Vec<(u16, u8, &'static str)>>,
    bad: &BTreeSet<u16>,
) -> Vec<FrameStruct> {
    let mut out = Vec::new();
    for (&start, used) in uses {
        if !used.address
            || used.number.is_some()
            || used.other
            || !passed.contains(&start)
            || bad.contains(&start)
        {
            continue;
        }
        let end = uses
            .iter()
            .filter(|(slot, _)| **slot < start)
            .filter(|(slot, used)| {
                used.other || used.address || used.number.is_none() || Some(**slot) == result
            })
            .map(|(slot, _)| *slot)
            .max()
            .unwrap_or(RESULT_BASE)
            .max(RESULT_BASE);
        let size = start.saturating_sub(end);
        let mut fields: Vec<(u16, u8, &'static str)> = uses
            .iter()
            .filter(|(slot, _)| **slot < start && **slot > end)
            .filter_map(|(slot, used)| {
                used.number
                    .map(|(width, basic)| (start.saturating_sub(*slot), width, basic))
            })
            .collect();
        fields.extend(through.get(&start).into_iter().flatten().copied());
        fields.sort_unstable_by_key(|(offset, width, _)| (*offset, *width));
        fields.dedup_by_key(|(offset, width, _)| (*offset, *width));
        let fits = fields.iter().enumerate().all(|(at, (offset, width, _))| {
            let after = offset.checked_add(u16::from(*width));
            let next = fields.get(at.saturating_add(1)).map_or(size, |next| next.0);
            after.is_some_and(|after| after <= next)
        });
        if fields.is_empty() || !fits {
            continue;
        }
        out.push(FrameStruct {
            start,
            size,
            fields,
        });
    }
    out
}

/// Tells whether `line` is the statement `statement`, with or without a
/// label.
fn is_statement(line: &str, statement: &str) -> bool {
    let text = line.trim_start();
    let text = match text.split_once(": ") {
        Some((label, rest)) if label.starts_with('L') && label.len() == 5 => rest,
        _ => text,
    };
    text == statement
}

/// Writes each struct of `structs` into `lines`: a `Dim` of the local of
/// its start as the type `T<tag>_<start>`, each field slot as a member of
/// that local, and each field through a pointer of `pointers` as the same
/// member. The statements that set a pointer go, and their labels stay.
/// Gives the declaration of each type, for the object.
fn apply_frame_structs(
    lines: &mut Vec<String>,
    structs: &[FrameStruct],
    pointers: &BTreeMap<u16, u16>,
    tag: u16,
) -> Vec<String> {
    let mut types = Vec::new();
    let mut dims = Vec::new();
    for frame in structs {
        let name = format!("T{tag}_{:X}", frame.start);
        let local = format!("local_{:X}", frame.start);
        types.push(format!("Private Type {name}"));
        let mut at = 0_u16;
        let mut pad = 0_u32;
        let mut members = Vec::new();
        for (offset, width, basic) in &frame.fields {
            if *offset > at {
                types.push(format!(
                    "    pad{pad}(0 To {}) As Byte",
                    offset.saturating_sub(at).saturating_sub(1)
                ));
                pad = pad.saturating_add(1);
            }
            types.push(format!("    f{offset:X} As {basic}"));
            let member = format!("{local}.f{offset:X}");
            if let Some(slot) = frame.start.checked_sub(*offset)
                && slot != frame.start
            {
                members.push((format!("local_{slot:X}"), member.clone()));
            }
            for (pointer, _) in pointers
                .iter()
                .filter(|(_, target)| **target == frame.start)
            {
                members.push((
                    format!("local_{pointer:X}.field_{offset:X}"),
                    member.clone(),
                ));
            }
            at = offset.saturating_add(u16::from(*width));
        }
        if frame.size > at {
            types.push(format!(
                "    pad{pad}(0 To {}) As Byte",
                frame.size.saturating_sub(at).saturating_sub(1)
            ));
        }
        types.push("End Type".to_owned());
        for (pointer, _) in pointers
            .iter()
            .filter(|(_, target)| **target == frame.start)
        {
            let set = format!("local_{pointer:X} = {local}");
            let clear = format!("local_{pointer:X} = 0");
            for line in lines.iter_mut() {
                if is_statement(line, &set) || is_statement(line, &clear) {
                    *line = match line.trim_start().split_once(": ") {
                        Some((label, _)) if label.starts_with('L') => format!("{label}:"),
                        _ => String::new(),
                    };
                }
            }
        }
        lines.retain(|line| !line.is_empty());
        for line in lines.iter_mut() {
            for (from, to) in &members {
                *line = replace_word(line, from, to);
            }
        }
        dims.push(format!("       Dim {local} As {name}"));
    }
    lines.splice(0..0, dims);
    types
}

/// Gives the frame slots of locals that a call of a DLL takes in `lines`:
/// each `local_` word of a line that calls one of `calls`.
fn passed_to_calls(lines: &[String], calls: &[String]) -> BTreeSet<u16> {
    lines
        .iter()
        .filter(|line| calls.iter().any(|call| line.contains(&format!("{call}("))))
        .flat_map(|line| words_with(std::slice::from_ref(line), "local_"))
        .filter_map(|word| {
            word.strip_prefix("local_")
                .and_then(|digits| u16::from_str_radix(digits, 16).ok())
        })
        .collect()
}

/// Gives the structs of the frame of the body `listing`, whose lines are
/// `lines`, and writes them into `lines` with [`apply_frame_structs`].
fn declare_frame_structs(
    listing: &PcodeListing,
    table: &PcodeTable,
    lines: &mut Vec<String>,
    calls: &[String],
    result: Option<u16>,
    tag: u16,
) -> Vec<String> {
    let ops: Vec<Op<'_>> = listing
        .instructions
        .iter()
        .map(|instruction| {
            let names = table
                .slot(instruction.lead, instruction.opcode)
                .map(|slot| slot.names.as_slice())
                .unwrap_or_default();
            (names, instruction.arguments.as_slice())
        })
        .collect();
    let uses = slot_uses(&ops);
    let pointers = struct_pointers(&ops);
    let (through, bad) = pointer_fields(&ops, &pointers);
    let passed = passed_to_calls(lines, calls);
    let result = result.map(|bytes| RESULT_BASE.saturating_add(bytes.max(2)));
    let structs = frame_structs(&uses, &passed, result, &through, &bad);
    apply_frame_structs(lines, &structs, &pointers, tag)
}

/// The arrays of the prototypes of the project that keep their type, by the
/// name of the procedure: the position of each argument and the Basic type
/// of its elements.
type TypedArrays = BTreeMap<String, Vec<(usize, &'static str)>>;

/// The Basic type of the elements of an array argument that a lifted caller
/// can declare: a type that is not an object.
const fn element_type(vb_type: &VbType) -> Option<&'static str> {
    match vb_type {
        VbType::Boolean => Some("Boolean"),
        VbType::Byte => Some("Byte"),
        VbType::Integer => Some("Integer"),
        VbType::Long => Some("Long"),
        VbType::Single => Some("Single"),
        VbType::Double => Some("Double"),
        VbType::Date => Some("Date"),
        VbType::Currency => Some("Currency"),
        VbType::Variant => Some("Variant"),
        VbType::Str => Some("String"),
        _ => None,
    }
}

/// Gives the text of each argument of each call of `name` in `lines`, apart
/// by the commas that are outside a string and outside inner parentheses.
fn call_arguments(lines: &[String], name: &str) -> Vec<Vec<String>> {
    let open = format!("{name}(");
    let mut out = Vec::new();
    for line in lines {
        let characters: Vec<char> = line.chars().collect();
        for at in 0..characters.len() {
            let tail: String = characters.get(at..).unwrap_or_default().iter().collect();
            let before = at
                .checked_sub(1)
                .and_then(|before| characters.get(before))
                .copied();
            if !tail.starts_with(&open) || before.is_some_and(is_name) {
                continue;
            }
            let mut depth = 0_usize;
            let mut quoted = false;
            let mut current = String::new();
            let mut arguments = Vec::new();
            for character in tail.chars().skip(open.len()) {
                match character {
                    '"' => quoted = !quoted,
                    _ if quoted => {}
                    '(' => depth = depth.saturating_add(1),
                    ')' if depth == 0 => {
                        arguments.push(current.trim().to_owned());
                        break;
                    }
                    ')' => depth = depth.saturating_sub(1),
                    ',' if depth == 0 => {
                        arguments.push(std::mem::take(&mut current).trim().to_owned());
                        continue;
                    }
                    _ => {}
                }
                current.push(character);
            }
            if arguments.len() == 1 && arguments.first().is_some_and(String::is_empty) {
                arguments.clear();
            }
            out.push(arguments);
        }
    }
    out
}

/// Tells whether `text` is a whole `local_` word.
fn is_local(text: &str) -> bool {
    text.strip_prefix("local_")
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Gives the array arguments of the prototypes of `procedures` that keep
/// their type: an array of a type that is not an object, which each call of
/// the procedure in `lines` fills with a local. A procedure name that two
/// prototypes give with other array types keeps no type.
fn typed_arrays<'a>(
    procedures: impl Iterator<Item = &'a ObjectProcedures>,
    lines: &[String],
) -> TypedArrays {
    let mut found: BTreeMap<String, Option<Vec<(usize, &'static str)>>> = BTreeMap::new();
    for own in procedures {
        let ObjectProcedures::Slots(slots) = own else {
            continue;
        };
        for slot in slots {
            let ProcedureEntry::Public {
                name,
                prototype: Some(prototype),
            } = slot
            else {
                continue;
            };
            let arrays: Vec<(usize, &'static str)> = prototype
                .arguments
                .iter()
                .enumerate()
                .filter(|(_, argument)| argument.entry.array)
                .filter_map(|(at, argument)| element_type(&argument.entry.vb_type).map(|t| (at, t)))
                .collect();
            if arrays.is_empty() {
                continue;
            }
            let entry = found
                .entry(name.clone())
                .or_insert_with(|| Some(arrays.clone()));
            if entry.as_ref() != Some(&arrays) {
                *entry = None;
            }
        }
    }
    let mut out = TypedArrays::new();
    for (name, arrays) in found {
        let Some(arrays) = arrays else {
            continue;
        };
        let calls = call_arguments(lines, &name);
        let kept: Vec<(usize, &'static str)> = arrays
            .into_iter()
            .filter(|(at, _)| {
                calls.iter().all(|arguments| {
                    arguments
                        .get(*at)
                        .is_some_and(|argument| is_local(argument))
                })
            })
            .collect();
        if !kept.is_empty() {
            out.insert(name, kept);
        }
    }
    out
}

/// Gives the element type of each local of `lines` that a call passes as a
/// typed array of `typed`.
fn typed_locals(lines: &[String], typed: &TypedArrays) -> BTreeMap<String, &'static str> {
    let mut out = BTreeMap::new();
    for (name, arrays) in typed {
        for arguments in call_arguments(lines, name) {
            for (at, element) in arrays {
                if let Some(argument) = arguments.get(*at)
                    && is_local(argument)
                {
                    out.entry(argument.clone()).or_insert(*element);
                }
            }
        }
    }
    out
}

/// Puts a `Dim` before `lines` for each local that `lines` index, for each
/// local of `typed`, which a call passes as an array of that type, and for
/// each fixed-size array of `fixed`, with its bounds.
/// Basic reads an index of a name that is not declared as a call.
fn declare_arrays(
    lines: &mut Vec<String>,
    typed: &BTreeMap<String, &'static str>,
    fixed: &BTreeMap<String, String>,
) {
    let mut arrays: BTreeMap<String, String> = indexed_words(lines, "local_")
        .into_iter()
        .map(|array| (array, "() As Variant".to_owned()))
        .collect();
    arrays.extend(
        typed
            .iter()
            .map(|(local, element)| (local.clone(), format!("() As {element}"))),
    );
    arrays.extend(
        fixed
            .iter()
            .map(|(local, shape)| (local.clone(), shape.clone())),
    );
    lines.splice(
        0..0,
        arrays
            .iter()
            .map(|(array, shape)| format!("       Dim {array}{shape}")),
    );
}

/// Gives the class of each local of the body `listing` that `Dim ... As New`
/// declares, by the name of the local. Basic creates such an object at its
/// first use: `FLdRf` gives the address of the local, and `NewIfNullPr`
/// creates the object of its class when the local is empty. A local with no
/// `New` in its declaration stays `Nothing`, and a call on it fails with
/// error 424. `callees` gives the name of each class of the project. A
/// class of a form is left out: the lift names such an object by the name
/// of the form.
fn new_locals(
    listing: &PcodeListing,
    table: &PcodeTable,
    callees: &Callees,
) -> BTreeMap<String, String> {
    let names = |at: usize| {
        listing
            .instructions
            .get(at)
            .and_then(|instruction| table.slot(instruction.lead, instruction.opcode))
            .map(|slot| slot.names.as_slice())
            .unwrap_or_default()
    };
    let word = |at: usize| {
        listing
            .instructions
            .get(at)
            .and_then(|instruction| instruction.arguments.get(..2))
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes)
    };
    let mut out = BTreeMap::new();
    for at in 1..listing.instructions.len() {
        let Some(before) = at.checked_sub(1) else {
            continue;
        };
        if !names(at).iter().any(|name| name == "NewIfNullPr")
            || !names(before).iter().any(|name| name == "FLdRf")
        {
            continue;
        }
        let (Some(slot), Some(index)) = (word(before), word(at)) else {
            continue;
        };
        let offset = i16::from_le_bytes(slot.to_le_bytes());
        let Some(class) = callees.class(index) else {
            continue;
        };
        if offset >= 0 || class.form_name().is_some() {
            continue;
        }
        if let Some(name) = class.object_name() {
            out.insert(
                format!("local_{:X}", offset.unsigned_abs()),
                name.to_owned(),
            );
        }
    }
    out
}

/// Puts a `Dim ... As New` before `lines` for each local of `locals` that
/// `lines` use.
fn declare_new_locals(lines: &mut Vec<String>, locals: &BTreeMap<String, String>) {
    let used = words_with(lines, "local_");
    lines.splice(
        0..0,
        locals
            .iter()
            .filter(|(local, _)| used.contains(*local))
            .map(|(local, class)| format!("       Dim {local} As New {class}")),
    );
}

/// Gives the bounds of `array` as Basic writes them, such as
/// `0 To 255, -8 To 7`.
fn array_ranges(array: &FixedArray) -> String {
    let ranges: Vec<String> = array
        .bounds
        .iter()
        .map(|(count, lower)| {
            let upper = i64::from(*lower)
                .saturating_add(i64::from(*count))
                .saturating_sub(1);
            format!("{lower} To {upper}")
        })
        .collect();
    ranges.join(", ")
}

/// Gives the declaration of each fixed-size local array of `arrays` after
/// its name, such as `(0 To 255) As Long` for `local_B0`.
fn fixed_shapes(arrays: &[FixedArray]) -> BTreeMap<String, String> {
    arrays
        .iter()
        .map(|array| {
            let element = redim_element(array.vartype).unwrap_or("Variant");
            (
                format!("local_{:X}", array.slot),
                format!("({}) As {element}", array_ranges(array)),
            )
        })
        .collect()
}

/// The fields of the records of each fixed-size array of an object, by the
/// offset of the array in the data of the object: the offset of each field
/// in the record, its bytes and its Basic type.
type RecordFields = BTreeMap<u16, Vec<(u16, u8, &'static str)>>;

/// Gives the fields that `ops` read and write in an element of an array of
/// the object whose offset is in `records`. Basic reads a field of
/// `StarArray(X).Y` as the address of the array in `Me` (`FMemLdRf` with
/// `Me` at 8), the address of the element (`Ary1LdPr` or `AryLdPr`), and a
/// load or a store at the offset of the field (`MemLdFPR4 4`). A record with
/// a field access that is not a number is in the second set.
fn record_fields(ops: &[Op<'_>], records: &BTreeSet<u16>) -> (RecordFields, BTreeSet<u16>) {
    let mut fields: RecordFields = BTreeMap::new();
    let mut bad = BTreeSet::new();
    for window in ops.windows(3) {
        let [(array, at), (element, _), (access, offset)] = window else {
            continue;
        };
        let word = |bytes: &[u8], at: usize| {
            bytes
                .get(at..at.checked_add(2)?)
                .and_then(|bytes| bytes.try_into().ok())
                .map(u16::from_le_bytes)
        };
        if !array.iter().any(|name| name == "FMemLdRf") || word(at, 0) != Some(8) {
            continue;
        }
        let Some(slot) = word(at, 2).filter(|slot| records.contains(slot)) else {
            continue;
        };
        if !element
            .iter()
            .any(|name| name == "Ary1LdPr" || name == "AryLdPr")
        {
            continue;
        }
        if !access
            .iter()
            .any(|name| name.starts_with("MemLd") || name.starts_with("MemSt"))
        {
            continue;
        }
        match (number_access(access), word(offset, 0)) {
            (Some((width, basic)), Some(offset)) => {
                fields.entry(slot).or_default().push((offset, width, basic));
            }
            _ => {
                bad.insert(slot);
            }
        }
    }
    (fields, bad)
}

/// A fixed-size array of records of an object, as its declarations give it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct RecordArray {
    /// The `Type` statement of its record, line by line.
    record: Vec<String>,
    /// Its shape after its name, such as `(0 To 500) As TField48`.
    shape: String,
}

/// Gives the type of the records of `array`, whose elements are
/// `array.element_bytes` long, from the fields `fields` that the bodies use.
/// Each field is the member `field_<offset>`, which is the text that the
/// lift writes, and a `Byte` array fills each gap. `public` gives a
/// `Public Type`, which a `Public` variable of a standard module needs.
/// Gives nothing when two fields overlap or a field passes the end.
fn record_array(
    array: &FixedArray,
    fields: &[(u16, u8, &'static str)],
    public: bool,
) -> Option<RecordArray> {
    let mut fields = fields.to_vec();
    fields.sort_unstable_by_key(|(offset, width, _)| (*offset, *width));
    fields.dedup();
    let size = array.element_bytes;
    let name = format!("TField{:X}", array.slot);
    let scope = if public { "Public" } else { "Private" };
    let mut record = vec![format!("{scope} Type {name}")];
    let mut at = 0_u32;
    let mut pad = 0_u32;
    for (offset, width, basic) in &fields {
        let offset = u32::from(*offset);
        if offset < at {
            return None;
        }
        if offset > at {
            record.push(format!(
                "    pad{pad}(0 To {}) As Byte",
                offset.saturating_sub(at).saturating_sub(1)
            ));
            pad = pad.saturating_add(1);
        }
        record.push(format!("    field_{offset:X} As {basic}"));
        at = offset.checked_add(u32::from(*width))?;
    }
    if fields.is_empty() || at > size {
        return None;
    }
    if size > at {
        record.push(format!(
            "    pad{pad}(0 To {}) As Byte",
            size.saturating_sub(at).saturating_sub(1)
        ));
    }
    record.push("End Type".to_owned());
    Some(RecordArray {
        record,
        shape: format!("({}) As {name}", array_ranges(array)),
    })
}

/// Gives `line` with `.field_0` after each element of the array `name`
/// that no member follows. The lift writes the field at 0 of an element as
/// the element, which is correct for an array of numbers and not for an
/// array of records.
fn member_zero(line: &str, name: &str) -> String {
    let open = format!("{name}(");
    let characters: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut at = 0_usize;
    while let Some(&character) = characters.get(at) {
        let tail: String = characters.get(at..).unwrap_or_default().iter().collect();
        let before = at
            .checked_sub(1)
            .and_then(|before| characters.get(before))
            .copied();
        if !tail.starts_with(&open) || before.is_some_and(|c| is_name(c) || c == '.') {
            out.push(character);
            at = at.saturating_add(1);
            continue;
        }
        let mut depth = 0_usize;
        let mut quoted = false;
        let mut end = None;
        for (step, inner) in tail.chars().enumerate().skip(open.len()) {
            match inner {
                '"' => quoted = !quoted,
                _ if quoted => {}
                '(' => depth = depth.saturating_add(1),
                ')' if depth == 0 => {
                    end = Some(step);
                    break;
                }
                ')' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        let Some(end) = end else {
            out.push(character);
            at = at.saturating_add(1);
            continue;
        };
        let element: String = tail.chars().take(end.saturating_add(1)).collect();
        out.push_str(&element);
        at = at.saturating_add(end).saturating_add(1);
        if characters.get(at) != Some(&'.') {
            out.push_str(".field_0");
        }
    }
    out
}

/// Gives each word of `lines` that starts with `prefix` and goes on with
/// hexadecimal digits, that no name character and no `.` comes before, and
/// whose next character `next` accepts.
fn words_before(
    lines: &[String],
    prefix: &str,
    next: impl Fn(Option<char>) -> bool,
) -> BTreeSet<String> {
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
                let after = tail.chars().nth(prefix.len().saturating_add(digits.len()));
                if !digits.is_empty() && next(after) {
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

/// The frame offset below which a `Function` keeps the value that it
/// returns: the value of `n` bytes is at `local_` 0x84 plus `n`, and a value
/// of 1 byte takes 2.
const RESULT_BASE: u16 = 0x84;

/// Builds the parameters of a procedure with no prototype, in the slots
/// from `first` to `end`. When `sizes` gives the bytes of each argument, as
/// each call of the procedure gives them, and they fill the slots, each
/// argument takes its size: an argument of more than 4 bytes is a value,
/// `ByVal`. Else each slot of 4 bytes is one argument by reference, because
/// an argument of 8 or 16 bytes by value is not told apart from two or
/// four arguments of 4 bytes.
fn built_parameters(first: u16, end: u16, sizes: Option<&[u8]>) -> Vec<String> {
    let fits = sizes.is_some_and(|sizes| {
        sizes
            .iter()
            .try_fold(first, |at, size| at.checked_add(u16::from(*size)))
            == Some(end)
    });
    let mut out = Vec::new();
    let mut at = first;
    match sizes {
        Some(sizes) if fits => {
            for size in sizes {
                if *size > 4 {
                    out.push(format!("ByVal arg_{at:X} As Variant"));
                } else {
                    out.push(format!("arg_{at:X} As Variant"));
                }
                at = at.saturating_add(u16::from(*size));
            }
        }
        _ => {
            while at < end {
                out.push(format!("arg_{at:X} As Variant"));
                at = at.saturating_add(4);
            }
        }
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
    sizes: Option<&[u8]>,
    typed: &TypedArrays,
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
            let signature = crate::write::code::format_signature(
                "Public",
                &name,
                Some(&by_reference_variants(
                    prototype,
                    typed.get(&name).map_or(&[], Vec::as_slice),
                )),
            );
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
            // A call passes the address of the result as an argument too:
            // first for a module, last for an object.
            let sizes = match (function, module) {
                (true, true) => sizes.and_then(|sizes| sizes.get(1..)),
                (true, false) => sizes.and_then(|sizes| sizes.split_last().map(|(_, rest)| rest)),
                (false, _) => sizes,
            };
            let list = built_parameters(first, end, sizes);
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

/// Gives `prototype` with each argument by reference, and each array, as a
/// `Variant` by reference. A lifted caller passes a `Variant` local, which
/// Basic refuses for an argument by reference of another type. A `Variant`
/// by reference still passes the variable, and it can hold an array. An
/// array at a position of `typed` keeps its type: each caller declares the
/// local that it passes there as an array of that type.
fn by_reference_variants(prototype: &Prototype, typed: &[(usize, &'static str)]) -> Prototype {
    let mut out = prototype.clone();
    for (position, argument) in out.arguments.iter_mut().enumerate() {
        if argument.entry.array && typed.iter().any(|(at, _)| *at == position) {
            continue;
        }
        if argument.entry.by_ref || argument.entry.array {
            argument.entry.vb_type = VbType::Variant;
            argument.entry.array = false;
            argument.entry.by_ref = true;
        }
    }
    out
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

/// Gives the number of arguments of the first call of `name` in `lines`:
/// the arguments between the parentheses after `name`, apart by the commas
/// that are outside a string and outside inner parentheses.
fn call_arity(lines: &[String], name: &str) -> Option<usize> {
    let open = format!("{name}(");
    for line in lines {
        let characters: Vec<char> = line.chars().collect();
        let mut at = 0_usize;
        while at < characters.len() {
            let tail: String = characters.get(at..).unwrap_or_default().iter().collect();
            let before = at
                .checked_sub(1)
                .and_then(|before| characters.get(before))
                .copied();
            if tail.starts_with(&open) && !before.is_some_and(|c| is_name(c) || c == '.') {
                let mut depth = 0_usize;
                let mut commas = 0_usize;
                let mut empty = true;
                let mut quoted = false;
                for character in tail.chars().skip(open.len()) {
                    match character {
                        '"' => quoted = !quoted,
                        _ if quoted => {}
                        '(' => depth = depth.saturating_add(1),
                        ')' if depth == 0 => {
                            return Some(if empty { 0 } else { commas.saturating_add(1) });
                        }
                        ')' => depth = depth.saturating_sub(1),
                        ',' if depth == 0 => commas = commas.saturating_add(1),
                        _ => {}
                    }
                    if !character.is_whitespace() {
                        empty = false;
                    }
                }
                return None;
            }
            at = at.saturating_add(1);
        }
    }
    None
}

/// Gives the `Declare` statement of each procedure of `declarations` that
/// `lines` calls. The file holds no type of an argument, so each one is
/// `ByRef As Any`, which takes any argument, and the result is a `Long`.
fn declare_statements(declarations: &[Declaration], lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for declaration in declarations {
        let ExportName::Name(name) = &declaration.export else {
            continue;
        };
        if !crate::vb::privateobj::is_plausible_identifier(name.as_bytes()) {
            continue;
        }
        let Some(arity) = call_arity(lines, name) else {
            continue;
        };
        let parameters: Vec<String> = (1..=arity)
            .map(|position| format!("ByRef a{position} As Any"))
            .collect();
        let statement = format!(
            "Private Declare Function {name} Lib \"{}\" ({}) As Long",
            declaration.library,
            parameters.join(", ")
        );
        if !out.contains(&statement) {
            out.push(statement);
        }
    }
    out
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
    let declarations = DeclareTable::read(&pe, &info).declarations;
    let calls: Vec<String> = declarations
        .iter()
        .filter_map(|declaration| match &declaration.export {
            ExportName::Name(name) => Some(name.clone()),
            _ => None,
        })
        .collect();
    let mut raw: Vec<String> = Vec::new();
    for (object, callees) in objects.iter().zip(&callees) {
        let Ok(methods) = read_method_table(&pe, object.lp_object_info) else {
            continue;
        };
        for entry in &methods.entries {
            if let MethodEntry::Descriptor { index, descriptor } = entry
                && let Some(body) = descriptor.body(&pe)
                && let Ok(stmts) =
                    lift_method(&disassemble(&body, table), table, callees, types, *index)
            {
                raw.extend(render(&stmts));
            }
        }
    }
    let typed = typed_arrays(report.objects.iter().map(|own| &own.procedures), &raw);
    let mut arrays: BTreeMap<String, &'static str> = BTreeMap::new();
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
        let mut all_lines: Vec<String> = Vec::new();
        let mut fields = BTreeSet::new();
        let mut frame_types = Vec::new();
        let mut object_arrays: BTreeMap<String, &'static str> = BTreeMap::new();
        let module_arrays = module_fixed_arrays(&pe, object.lp_public_bytes);
        let records: BTreeSet<u16> = module_arrays
            .iter()
            .filter(|array| array.vartype == 0)
            .map(|array| array.slot)
            .collect();
        let mut members: RecordFields = BTreeMap::new();
        let mut bad_records = BTreeSet::new();
        for entry in &methods.entries {
            let MethodEntry::Descriptor { index, descriptor } = entry else {
                continue;
            };
            let Some(body) = descriptor.body(&pe) else {
                continue;
            };
            let listing = disassemble(&body, table);
            let ops: Vec<Op<'_>> = listing
                .instructions
                .iter()
                .map(|instruction| {
                    let names = table
                        .slot(instruction.lead, instruction.opcode)
                        .map(|slot| slot.names.as_slice())
                        .unwrap_or_default();
                    (names, instruction.arguments.as_slice())
                })
                .collect();
            let (found, bad) = record_fields(&ops, &records);
            for (slot, fields) in found {
                members.entry(slot).or_default().extend(fields);
            }
            bad_records.extend(bad);
            let mut lines = match lift_method(&listing, table, callees, types, *index) {
                Ok(stmts) => render(&stmts),
                Err(fault) => vec![format!("    ' The lift stopped: {fault:?}")],
            };
            fields.extend(words_with(&lines, "field_"));
            all_lines.extend(lines.iter().cloned());
            globals.extend(words_with(&lines, "g_"));
            let result = result_bytes(&listing, table);
            let (declaration, closing) = declare(
                &own.procedures,
                *index,
                descriptor.arg_size,
                result,
                callees.argument_sizes(*index),
                &typed,
                &mut lines,
            );
            frame_types.extend(declare_frame_structs(
                &listing, table, &mut lines, &calls, result, *index,
            ));
            let mut locals = typed_locals(&lines, &typed);
            for (array, element) in redim_arrays(&listing, table, &lines) {
                if array.starts_with("local_") {
                    locals.entry(array).or_insert(element);
                } else if array.starts_with("g_") {
                    arrays.entry(array).or_insert(element);
                } else {
                    object_arrays.entry(array).or_insert(element);
                }
            }
            declare_arrays(
                &mut lines,
                &locals,
                &fixed_shapes(&descriptor.fixed_arrays(&pe)),
            );
            declare_new_locals(&mut lines, &new_locals(&listing, table, callees));
            lifted.procedures.push(LiftedProcedure {
                index: *index,
                declaration,
                lines,
                closing,
            });
        }
        let public = callees.variable_fields();
        let mut fixed: BTreeMap<String, String> = BTreeMap::new();
        for array in &module_arrays {
            let name = format!("field_{:X}", array.slot);
            if array.vartype != 0 {
                if let Some(element) = redim_element(array.vartype) {
                    fixed.insert(name, format!("({}) As {element}", array_ranges(array)));
                }
                continue;
            }
            if bad_records.contains(&array.slot) {
                continue;
            }
            let found = members
                .get(&array.slot)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let Some(record) = record_array(array, found, public.contains(&u32::from(array.slot)))
            else {
                continue;
            };
            if found.iter().any(|(offset, _, _)| *offset == 0) {
                for procedure in &mut lifted.procedures {
                    for line in &mut procedure.lines {
                        *line = member_zero(line, &name);
                    }
                }
            }
            frame_types.extend(record.record);
            fixed.insert(name, record.shape);
        }
        lifted.declarations = frame_types;
        lifted
            .declarations
            .extend(declare_statements(&declarations, &all_lines));
        lifted
            .declarations
            .extend(field_declarations(&fields, &public, &object_arrays, &fixed));
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
            Some(home) if home == at => lifted.declarations.extend(
                all.iter()
                    .map(|name| variable_declaration("Public", name, &arrays)),
            ),
            Some(_) => {}
            None => lifted.declarations.extend(
                own.iter()
                    .map(|name| variable_declaration("Private", name, &arrays)),
            ),
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
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        FixedArray, Op, TypedArrays, apply_frame_structs, built_parameters, call_arguments,
        call_arity, declare, declare_arrays, declare_new_locals, declare_statements,
        declared_bytes, field_declarations, fixed_shapes, frame_structs, indexed_words,
        member_zero, new_locals, pointer_fields, procedure_name, record_array, record_fields,
        redim_arrays, replace_word, slot_uses, struct_pointers, typed_arrays, typed_locals,
        variable_declaration, words_with,
    };
    use crate::read::region::{Off, Region};
    use crate::vb::lift::result_bytes;
    use crate::vb::pcode::{PcodeTable, disassemble};
    use crate::vb::project::{Declaration, ExportName};
    use crate::vb::{ObjectProcedures, ProcedureEntry};

    #[test]
    fn a_call_gives_the_number_of_its_arguments() {
        let lines = vec![
            "       x = Other(1)".to_owned(),
            "       Call GetObjectA(CLng(a.Image), 24, \"(,)\")".to_owned(),
        ];
        assert_eq!(call_arity(&lines, "GetObjectA"), Some(3));
        assert_eq!(call_arity(&lines, "Other"), Some(1));
        assert_eq!(call_arity(&["Call Beep()".to_owned()], "Beep"), Some(0));
        assert_eq!(
            call_arity(&["Call F(G(1, 2), \"a,b\")".to_owned()], "F"),
            Some(2)
        );
        assert_eq!(call_arity(&["Call x.Beep(1)".to_owned()], "Beep"), None);
        assert_eq!(call_arity(&lines, "Missing"), None);
    }

    #[test]
    fn a_called_procedure_of_a_dll_gets_a_declare_statement() {
        let declarations = vec![
            Declaration {
                library: "gdi32".to_owned(),
                export: ExportName::Name("GetObjectA".to_owned()),
            },
            Declaration {
                library: "user32".to_owned(),
                export: ExportName::Name("Unused".to_owned()),
            },
            Declaration {
                library: "user32".to_owned(),
                export: ExportName::OrdinalInferred(12),
            },
        ];
        let lines = vec!["       Call GetObjectA(h, 24, local_A0)".to_owned()];
        assert_eq!(
            declare_statements(&declarations, &lines),
            [
                "Private Declare Function GetObjectA Lib \"gdi32\" (ByRef a1 As Any, ByRef a2 As Any, ByRef a3 As Any) As Long"
            ]
        );
    }

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
        let indexed = vec![
            "local_B0(local_88) = local_C4".to_owned(),
            "local_D4(1, 2).field_0 = x.local_E0(1) + local_F0 (2)".to_owned(),
        ];
        assert_eq!(
            indexed_words(&indexed, "local_")
                .into_iter()
                .collect::<Vec<_>>(),
            ["local_B0", "local_D4"]
        );
    }

    #[test]
    fn a_field_with_an_accessor_is_public() {
        let used: BTreeSet<String> = ["field_34".to_owned(), "field_42".to_owned()].into();
        let public: BTreeSet<u32> = [0x42, 0x80].into();
        assert_eq!(
            field_declarations(&used, &public, &BTreeMap::new(), &BTreeMap::new()),
            [
                "Public field_42 As Variant",
                "Public field_80 As Variant",
                "Private field_34 As Variant"
            ]
        );
    }

    /// The names of one opcode, and its arguments.
    fn op(names: &[&str], arguments: &[u8]) -> (Vec<String>, Vec<u8>) {
        (
            names.iter().map(|name| (*name).to_owned()).collect(),
            arguments.to_vec(),
        )
    }

    /// Runs the struct analysis over `ops`, with `passed` as the slots that
    /// a call of a DLL takes, and writes the structs into `lines`.
    fn structs_of(
        ops: &[(Vec<String>, Vec<u8>)],
        passed: &[u16],
        result: Option<u16>,
        lines: &mut Vec<String>,
    ) -> Vec<String> {
        let ops: Vec<(&[String], &[u8])> = ops
            .iter()
            .map(|(names, arguments)| (names.as_slice(), arguments.as_slice()))
            .collect();
        let uses = slot_uses(&ops);
        let pointers = struct_pointers(&ops);
        let (through, bad) = pointer_fields(&ops, &pointers);
        let passed: BTreeSet<u16> = passed.iter().copied().collect();
        let structs = frame_structs(&uses, &passed, result, &through, &bad);
        apply_frame_structs(lines, &structs, &pointers, 4)
    }

    #[test]
    fn a_local_that_a_dll_fills_is_a_struct_with_the_slots_after_it() {
        // GetObject srcPictureBox.Image, Len(bm), bm, then a read of
        // bm.bmWidth at 4, and the result of the Function at 0x88.
        let load = ["FLdAd", "FLdI4", "FLdR4", "FLdStr", "ILdRf"];
        let ops = [
            op(&["FLdRf", "FLdRfVar"], &[0x60, 0xFF]),
            op(&load, &[0x64, 0xFF]),
            op(&["FStI4", "FStR4"], &[0x78, 0xFF]),
        ];
        let mut lines = vec![
            "       Call GetObjectA(local_4C, 24, local_A0)".to_owned(),
            "       GetImageWidth = local_9C".to_owned(),
        ];
        let types = structs_of(&ops, &[0xA0, 0x4C], Some(0x88), &mut lines);
        assert_eq!(
            types,
            [
                "Private Type T4_A0",
                "    pad0(0 To 3) As Byte",
                "    f4 As Long",
                "    pad1(0 To 15) As Byte",
                "End Type"
            ]
        );
        assert_eq!(
            lines,
            [
                "       Dim local_A0 As T4_A0",
                "       Call GetObjectA(local_4C, 24, local_A0)",
                "       GetImageWidth = local_A0.f4"
            ]
        );
        // A local that the body also reads as a value is no struct.
        let mut read = vec!["       Call GetObjectA(local_A0)".to_owned()];
        let value = [
            op(&["FLdRf"], &[0x60, 0xFF]),
            op(&load, &[0x60, 0xFF]),
            op(&load, &[0x64, 0xFF]),
        ];
        assert!(structs_of(&value, &[0xA0], None, &mut read).is_empty());
        // A local that no call of a DLL takes is no struct.
        let mut other = vec!["       Call Me.Fill(local_A0)".to_owned()];
        assert!(structs_of(&ops, &[], Some(0x88), &mut other).is_empty());
    }

    #[test]
    fn a_with_block_on_a_struct_writes_its_fields_through_the_struct() {
        // With bmi.bmHeader: FLdRf of bmi at 0x4DC, FStI4 of the pointer at
        // 0x4E0, then .bmPlanes = 1 at 12 and .bmSize = 40 at 0 through it.
        let ops = [
            op(&["FLdRf", "FLdRfVar"], &[0x24, 0xFB]),
            op(&["FStI4", "FStR4"], &[0x20, 0xFB]),
            op(&["FMemStI2"], &[0x20, 0xFB, 0x0C, 0x00]),
            op(&["FMemStI4", "FMemStR4"], &[0x20, 0xFB, 0x00, 0x00]),
            op(&["FStI4", "FStR4"], &[0x20, 0xFB]),
            op(&["FStI4", "FStR4"], &[0x70, 0xFF]),
        ];
        let mut lines = vec![
            "       local_4E0 = local_4DC".to_owned(),
            "       local_4E0.field_C = 1".to_owned(),
            "       local_4E0.field_0 = 40".to_owned(),
            "L0020: local_4E0 = 0".to_owned(),
            "       Call GetDIBits(local_4DC, 0)".to_owned(),
        ];
        // The slot of the result, at 0x90, ends the struct.
        let types = structs_of(&ops, &[0x4DC], Some(0x90), &mut lines);
        assert_eq!(
            types,
            [
                "Private Type T4_4DC",
                "    f0 As Long",
                "    pad0(0 To 7) As Byte",
                "    fC As Integer",
                "    pad1(0 To 1085) As Byte",
                "End Type"
            ]
        );
        assert_eq!(
            lines,
            [
                "       Dim local_4DC As T4_4DC",
                "       local_4DC.fC = 1",
                "       local_4DC.f0 = 40",
                "L0020:",
                "       Call GetDIBits(local_4DC, 0)"
            ]
        );
    }

    #[test]
    fn an_indexed_local_is_declared_as_an_array() {
        let mut lines = vec![
            "       local_B0(local_88) = local_C4".to_owned(),
            "       Exit Sub".to_owned(),
        ];
        declare_arrays(&mut lines, &BTreeMap::new(), &BTreeMap::new());
        assert_eq!(
            lines,
            [
                "       Dim local_B0() As Variant",
                "       local_B0(local_88) = local_C4",
                "       Exit Sub"
            ]
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
            built_parameters(0x10, 0x1C, None),
            [
                "arg_10 As Variant",
                "arg_14 As Variant",
                "arg_18 As Variant"
            ]
        );
        assert!(built_parameters(0x0C, 0x0C, None).is_empty());
        // Four values of 8 bytes and one of 4, as each call gives them.
        let sizes = [8, 8, 8, 8, 4];
        assert_eq!(
            built_parameters(0x0C, 0x30, Some(&sizes)),
            [
                "ByVal arg_C As Variant",
                "ByVal arg_14 As Variant",
                "ByVal arg_1C As Variant",
                "ByVal arg_24 As Variant",
                "arg_2C As Variant"
            ]
        );
        // Sizes that do not fill the slots are not taken.
        assert_eq!(built_parameters(0x0C, 0x14, Some(&sizes)).len(), 2);
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
    fn an_argument_by_reference_of_a_prototype_becomes_a_variant() {
        use crate::vb::functyp::{
            Argument, OptionalDefaultsOutcome, PropertyKind, Prototype, TypeEntry, VbType,
        };
        let entry = |vb_type, by_ref, array| TypeEntry {
            vb_type,
            optional: false,
            array,
            by_ref,
        };
        let argument = |name: &str, entry| Argument {
            name: name.to_owned(),
            entry,
            default: None,
        };
        let prototype = Prototype {
            member_id: 0,
            v_off: 0,
            const_ffff: 0xFFFF,
            nul1: 0,
            property_kind: PropertyKind::None,
            is_function: true,
            arguments: vec![
                argument("Box", entry(VbType::Object, true, false)),
                argument("Pixels", entry(VbType::Byte, true, true)),
                argument("Size", entry(VbType::Long, false, false)),
            ],
            return_type: Some(entry(VbType::Long, false, false)),
            optional_defaults: OptionalDefaultsOutcome::NoOptionalVals,
        };
        let slots = ObjectProcedures::Slots(vec![ProcedureEntry::Public {
            name: "Draw".to_owned(),
            prototype: Some(prototype),
        }]);
        let mut lines = vec!["local_88 = arg_C + arg_10 + arg_14".to_owned()];
        let (declaration, closing) = declare(
            &slots,
            0,
            20,
            Some(4),
            None,
            &TypedArrays::new(),
            &mut lines,
        );
        assert_eq!(
            declaration,
            "Public Function Draw(ByRef Box As Variant, ByRef Pixels As Variant, ByVal Size As Long) As Long"
        );
        assert_eq!(closing, "End Function");
        assert_eq!(lines, ["Draw = Box + Pixels + Size"]);
    }

    /// The opcodes of `StarArray(X).Y = StarArray(X).Y + StarArray(X).Speed`
    /// of `Physics_Demo.exe`, with the array at 0x48 of `Me`.
    #[test]
    fn a_record_field_is_the_access_after_the_element_of_an_array_of_me() {
        let names =
            |list: &[&str]| -> Vec<String> { list.iter().map(|n| (*n).to_owned()).collect() };
        let array = names(&["FMemLdRf", "FMemLdRfVar"]);
        let element = names(&["Ary1LdPr"]);
        let single = names(&["MemLdFPR4"]);
        let byte = names(&["MemStUI1"]);
        let add = names(&["AddR4", "AddR8"]);
        let me_48 = [8, 0, 0x48, 0];
        let ops: Vec<Op<'_>> = vec![
            (&array, &me_48),
            (&element, &[]),
            (&single, &[4, 0]),
            (&array, &me_48),
            (&element, &[]),
            (&single, &[0x0C, 0]),
            (&add, &[]),
            (&array, &me_48),
            (&element, &[]),
            (&byte, &[8, 0]),
            // An array at another offset, and an array of a local.
            (&array, &[8, 0, 0x2C, 0]),
            (&element, &[]),
            (&single, &[0, 0]),
            (&array, &[0x70, 0xFF, 0x48, 0]),
            (&element, &[]),
            (&single, &[0, 0]),
        ];
        let (fields, bad) = record_fields(&ops, &BTreeSet::from([0x48]));
        assert_eq!(
            fields,
            BTreeMap::from([(
                0x48,
                vec![(4, 4, "Single"), (0x0C, 4, "Single"), (8, 1, "Byte")]
            )])
        );
        assert!(bad.is_empty());
        let other = names(&["MemLdVar"]);
        let ops: Vec<Op<'_>> = vec![(&array, &me_48), (&element, &[]), (&other, &[0, 0])];
        assert_eq!(
            record_fields(&ops, &BTreeSet::from([0x48])).1,
            BTreeSet::from([0x48])
        );
    }

    #[test]
    fn a_record_array_gives_its_type_with_a_pad_in_each_gap() {
        let array = FixedArray::new(0x48, 0, vec![(501, 0)]).with_element_bytes(16);
        let fields = [
            (0x0C, 4, "Single"),
            (4, 4, "Single"),
            (8, 1, "Byte"),
            (4, 4, "Single"),
        ];
        let record = record_array(&array, &fields, false).unwrap();
        assert_eq!(
            record.record,
            [
                "Private Type TField48",
                "    pad0(0 To 3) As Byte",
                "    field_4 As Single",
                "    field_8 As Byte",
                "    pad1(0 To 2) As Byte",
                "    field_C As Single",
                "End Type"
            ]
        );
        assert_eq!(record.shape, "(0 To 500) As TField48");
        let small = FixedArray::new(0x2C, 0, vec![(76, 0)]).with_element_bytes(16);
        let record = record_array(&small, &[(0, 4, "Long")], true).unwrap();
        assert_eq!(
            record.record,
            [
                "Public Type TField2C",
                "    field_0 As Long",
                "    pad0(0 To 11) As Byte",
                "End Type"
            ]
        );
        // Two fields that overlap, a field past the end, and no field.
        assert!(record_array(&array, &[(0, 4, "Long"), (2, 2, "Integer")], false).is_none());
        assert!(record_array(&array, &[(0x0E, 4, "Long")], false).is_none());
        assert!(record_array(&array, &[], false).is_none());
    }

    #[test]
    fn the_field_at_zero_of_a_record_element_gets_its_member() {
        assert_eq!(
            member_zero(
                "Call F(field_48(g(1, 2)), field_48(i).field_4, xfield_48(1))",
                "field_48"
            ),
            "Call F(field_48(g(1, 2)).field_0, field_48(i).field_4, xfield_48(1))"
        );
        assert_eq!(
            member_zero("x = field_48(\")\")", "field_48"),
            "x = field_48(\")\").field_0"
        );
        assert_eq!(member_zero("x = field_48(1", "field_48"), "x = field_48(1");
    }

    #[test]
    fn a_fixed_array_is_declared_with_its_bounds_and_its_type() {
        let shapes = fixed_shapes(&[
            FixedArray::new(0xB0, 3, vec![(256, 0)]),
            FixedArray::new(0x2C, 0x11, vec![(256, 0), (16, -8)]),
        ]);
        let mut lines = vec!["       local_B0(1) = local_2C(2, 3)".to_owned()];
        declare_arrays(&mut lines, &BTreeMap::new(), &shapes);
        assert_eq!(
            lines,
            [
                "       Dim local_2C(0 To 255, -8 To 7) As Byte",
                "       Dim local_B0(0 To 255) As Long",
                "       local_B0(1) = local_2C(2, 3)"
            ]
        );
    }

    #[test]
    fn a_local_that_new_if_null_creates_is_declared_as_new() {
        use crate::vb::lift::Callees;
        let table = PcodeTable::parse(
            b"[primary.04]\nwidth = 2\nnames = [\"FLdRf\"]\n\
              [primary.24]\nwidth = 2\nnames = [\"NewIfNullPr\"]\n",
        )
        .unwrap();
        // FLdRf -0xA0, NewIfNullPr 0x18; FLdRf -0x98, NewIfNullPr 0x1C;
        // FLdRf +0x0C, NewIfNullPr 0x18.
        let body = [
            0x04, 0x60, 0xFF, 0x24, 0x18, 0, 0x04, 0x68, 0xFF, 0x24, 0x1C, 0, 0x04, 0x0C, 0, 0x24,
            0x18, 0,
        ];
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        let callees = Callees::default()
            .with_class(0x18, Callees::default().with_object_name("FastDrawing"))
            .with_class(
                0x1C,
                Callees::default()
                    .with_object_name("Form2")
                    .with_form_name("Form2"),
            );
        let locals = new_locals(&listing, &table, &callees);
        assert_eq!(
            locals.into_iter().collect::<Vec<_>>(),
            vec![("local_A0".to_owned(), "FastDrawing".to_owned())]
        );
        let mut lines = vec!["       local_A4 = local_A0.GetImageWidth(arg_C)".to_owned()];
        declare_new_locals(
            &mut lines,
            &BTreeMap::from([
                ("local_A0".to_owned(), "FastDrawing".to_owned()),
                ("local_B0".to_owned(), "FastDrawing".to_owned()),
            ]),
        );
        assert_eq!(
            lines.first().map(String::as_str),
            Some("       Dim local_A0 As New FastDrawing")
        );
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn a_redim_gives_the_type_of_the_elements_of_its_array() {
        let table = PcodeTable::parse(b"[primary.3B]\nwidth = 8\nnames = [\"Redim\"]\n").unwrap();
        // Redim of 2 dimensions of VARTYPE 0x11, 1 byte each, flags 0x80;
        // then one of VARTYPE 3 with no FADF_HAVEVARTYPE.
        let body = [
            0x3B, 2, 0, 0x11, 0, 1, 0, 0x80, 0, 0x3B, 1, 0, 3, 0, 4, 0, 0, 0,
        ];
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        let lines = vec![
            "       ReDim local_88(0 To 1, 0 To 2)".to_owned(),
            "L0009: ReDim field_34(0 To 4)".to_owned(),
        ];
        let arrays = redim_arrays(&listing, &table, &lines);
        assert_eq!(arrays.get("local_88"), Some(&"Byte"));
        assert_eq!(arrays.get("field_34"), None);
        // A count of ReDim lines that is not the count of opcodes gives
        // nothing.
        assert!(redim_arrays(&listing, &table, lines.get(..1).unwrap()).is_empty());
        assert_eq!(
            variable_declaration("Private", "local_88", &arrays),
            "Private local_88() As Byte"
        );
    }

    #[test]
    fn an_array_that_each_call_fills_with_a_local_keeps_its_type() {
        use crate::vb::functyp::{
            Argument, OptionalDefaultsOutcome, PropertyKind, Prototype, TypeEntry, VbType,
        };
        let entry = |vb_type, array| TypeEntry {
            vb_type,
            optional: false,
            array,
            by_ref: true,
        };
        let prototype = Prototype {
            member_id: 0,
            v_off: 0,
            const_ffff: 0xFFFF,
            nul1: 0,
            property_kind: PropertyKind::None,
            is_function: false,
            arguments: vec![
                Argument {
                    name: "Box".to_owned(),
                    entry: entry(VbType::Object, false),
                    default: None,
                },
                Argument {
                    name: "Pixels".to_owned(),
                    entry: entry(VbType::Byte, true),
                    default: None,
                },
            ],
            return_type: None,
            optional_defaults: OptionalDefaultsOutcome::NoOptionalVals,
        };
        let slots = ObjectProcedures::Slots(vec![ProcedureEntry::Public {
            name: "Fill".to_owned(),
            prototype: Some(prototype),
        }]);
        let calls = vec![
            "       Call local_E8.Fill(Me.pic, local_EC)".to_owned(),
            "       Call Fill(arg_C, local_88)".to_owned(),
        ];
        let typed = typed_arrays(std::iter::once(&slots), &calls);
        assert_eq!(typed.get("Fill"), Some(&vec![(1, "Byte")]));
        let locals = typed_locals(&calls, &typed);
        assert_eq!(locals.get("local_EC"), Some(&"Byte"));
        let mut lines = calls.clone();
        declare_arrays(&mut lines, &locals, &BTreeMap::new());
        assert_eq!(
            lines.get(..2),
            Some(
                &[
                    "       Dim local_88() As Byte".to_owned(),
                    "       Dim local_EC() As Byte".to_owned()
                ][..]
            )
        );
        let (declaration, _) = declare(&slots, 0, 12, None, None, &typed, &mut Vec::new());
        assert_eq!(
            declaration,
            "Public Sub Fill(ByRef Box As Variant, ByRef Pixels() As Byte)"
        );
        // One call that passes no bare local keeps the array a Variant.
        let mixed = vec![
            "       Call Fill(arg_C, local_88)".to_owned(),
            "       Call Fill(arg_C, field_34)".to_owned(),
        ];
        assert!(typed_arrays(std::iter::once(&slots), &mixed).is_empty());
        assert_eq!(
            call_arguments(&mixed, "Fill"),
            [vec!["arg_C", "local_88"], vec!["arg_C", "field_34"]]
        );
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
            declare(
                &slots,
                0,
                12,
                Some(4),
                None,
                &TypedArrays::new(),
                &mut lines
            ),
            (
                "Private Function UnnamedProcedure0(arg_C As Variant) As Variant".to_owned(),
                "End Function"
            )
        );
        assert_eq!(lines, ["UnnamedProcedure0 = arg_C"]);
        // The calls give a Double, then the address of the result.
        assert_eq!(
            declare(
                &slots,
                0,
                16,
                Some(4),
                Some(&[8, 4]),
                &TypedArrays::new(),
                &mut Vec::new()
            )
            .0,
            "Private Function UnnamedProcedure0(ByVal arg_C As Variant) As Variant"
        );
        let mut byte = vec!["local_86 = 1".to_owned(), "local_85 = 2".to_owned()];
        let _ = declare(&slots, 0, 8, Some(1), None, &TypedArrays::new(), &mut byte);
        assert_eq!(byte, ["UnnamedProcedure0 = 1", "local_85 = 2"]);
        let mut handler = vec!["local_88 = arg_C".to_owned()];
        assert_eq!(
            declare(&slots, 1, 8, None, None, &TypedArrays::new(), &mut handler),
            (
                "Private Sub Form_KeyPress(KeyAscii As Integer)".to_owned(),
                "End Sub"
            )
        );
        assert_eq!(handler, ["local_88 = KeyAscii"]);
        let module = ObjectProcedures::NoNameArray { proc_count: 1 };
        let mut body = vec!["local_94 = arg_10".to_owned()];
        assert_eq!(
            declare(&module, 0, 16, Some(16), None, &TypedArrays::new(), &mut body),
            (
                "Public Function UnnamedProcedure0(arg_10 As Variant, arg_14 As Variant) As Variant"
                    .to_owned(),
                "End Function"
            )
        );
        assert_eq!(body, ["UnnamedProcedure0 = arg_10"]);
        let mut sub = Vec::new();
        assert_eq!(
            declare(&module, 0, 12, None, None, &TypedArrays::new(), &mut sub).0,
            "Public Sub UnnamedProcedure0(arg_C As Variant, arg_10 As Variant)"
        );
        // In a module the address of the result comes first.
        assert_eq!(
            declare(
                &module,
                0,
                16,
                Some(4),
                Some(&[4, 8]),
                &TypedArrays::new(),
                &mut Vec::new()
            )
            .0,
            "Public Function UnnamedProcedure0(ByVal arg_10 As Variant) As Variant"
        );
    }
}
