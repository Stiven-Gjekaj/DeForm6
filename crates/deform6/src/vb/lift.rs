//! The lift of a decoded P-code body to statements.
//!
//! P-code is a stack machine. [`lift`] follows a [`PcodeListing`] opcode by
//! opcode, keeps a stack of expressions, and gives a statement at each
//! opcode that stores a value, branches or exits. The kind of each opcode
//! comes from the names of its handler in the table that the user derived:
//! this module knows the families of names, not the opcode numbers, and it
//! holds no table.
//!
//! # The families
//!
//! | Names | What the opcode does |
//! |---|---|
//! | `LitI4`, `LitR4`, `LitI2`, `LitI2_Byte` | Push a constant |
//! | `FLd` or `ILd` + a type, `FLdRf`, `ILdRf` | Push the frame slot at a signed 16-bit offset; the lift does not yet tell a slot from what it points to, and one handler serves names of both kinds |
//! | `ILdRfDarg` | Push the argument at a 16-bit offset |
//! | `FSt` + a type, `FStVarCopy` | Pop into the frame slot |
//! | `ISt` + a type | Pop into what the frame slot points to |
//! | `FLdPr` | Set the object register to the frame slot |
//! | `MemSt` + a type | Pop into a field of the object register |
//! | `ImpAdSt` + a type | Pop into a global at a 16-bit index |
//! | an operator and two types, such as `LtI4` | Pop two values, push one |
//! | `C` and two types, such as `CI4I2`, or `FnC`, a Basic word and a type, such as `FnCSngI2` | Convert the top value |
//! | `BranchF` | Pop a condition, and branch when it is false |
//! | `Branch` | Branch |
//! | `End`, and a name that starts with `ExitProc` | End the program, or exit |
//!
//! All the names of one slot must give one family. A conversion whose names
//! give both `CSng` and `CDbl` gives no conversion: one handler serves both,
//! and the store that follows decides the type.
//!
//! # What the lift checks
//!
//! The stack must hold the values that each opcode pops, and it must be
//! empty after each statement. A body that breaks either rule, or that holds
//! an opcode of no known family, gives a [`LiftFault`] and no statements.
//!
//! # What the output is
//!
//! The statements name a frame slot by its offset, such as `local_94` or
//! `arg_C`, and a field by its offset, such as `Me.field_54`. A branch is a
//! `GoTo` to a label at the offset of a statement. This is not the Basic of
//! the source, and no claim is made that it compiles.

use crate::vb::pcode::{PcodeEnd, PcodeInstruction, PcodeListing, PcodeTable};

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `=`
    Eq,
    /// `<>`
    Ne,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `And`
    And,
    /// `Or`
    Or,
    /// `Xor`
    Xor,
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `\`
    IntDiv,
    /// `Mod`
    Mod,
}

impl BinaryOp {
    /// The Basic text of the operator.
    const fn text(self) -> &'static str {
        match self {
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::And => "And",
            Self::Or => "Or",
            Self::Xor => "Xor",
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::IntDiv => "\\",
            Self::Mod => "Mod",
        }
    }
}

/// An expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// A constant.
    Const(i64),
    /// The frame slot at a negative offset: a local.
    Local(u16),
    /// The frame slot at a positive offset: an argument, or `Me` at 8.
    Arg(u16),
    /// A field of an object, at an offset.
    Field(Box<Expr>, u16),
    /// A global, at an index.
    Global(u16),
    /// Two values and an operator.
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    /// A conversion, by the name of the Basic function.
    Convert(&'static str, Box<Expr>),
}

impl Expr {
    /// The expression of the frame slot at `offset`.
    fn frame(offset: i16) -> Self {
        if offset < 0 {
            Self::Local(offset.unsigned_abs())
        } else {
            Self::Arg(offset.unsigned_abs())
        }
    }

    /// The text of the expression.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Const(value) => value.to_string(),
            Self::Local(offset) => format!("local_{offset:X}"),
            Self::Arg(8) => "Me".to_owned(),
            Self::Arg(offset) => format!("arg_{offset:X}"),
            Self::Field(object, offset) => format!("{}.field_{offset:X}", object.text()),
            Self::Global(index) => format!("global_{index:X}"),
            Self::Binary(op, left, right) => {
                format!("({} {} {})", left.text(), op.text(), right.text())
            }
            Self::Convert(function, value) => format!("{function}({})", value.text()),
        }
    }
}

/// A statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stmt {
    /// A store.
    Assign {
        /// Where the value goes.
        target: Expr,
        /// The value.
        value: Expr,
    },
    /// A branch to `target` when `condition` is false.
    IfNotGoTo {
        /// The condition.
        condition: Expr,
        /// The offset of the target in the body.
        target: u16,
    },
    /// A branch to `target`.
    GoTo(u16),
    /// `End`.
    End,
    /// An exit from the procedure.
    Exit,
}

impl Stmt {
    /// The text of the statement.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Assign { target, value } => format!("{} = {}", target.text(), value.text()),
            Self::IfNotGoTo { condition, target } => {
                format!("If Not {} Then GoTo L{target:04X}", condition.text())
            }
            Self::GoTo(target) => format!("GoTo L{target:04X}"),
            Self::End => "End".to_owned(),
            Self::Exit => "Exit".to_owned(),
        }
    }
}

/// A statement and the offset of its first opcode.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LiftedStmt {
    /// The offset of the first opcode of the statement.
    pub offset: u32,
    /// The statement.
    pub stmt: Stmt,
}

/// Why a body gives no statements, with the offset of the opcode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiftFault {
    /// The listing did not decode to its end.
    NotDecoded(PcodeEnd),
    /// The names of the slot give no family, or more than one.
    NoFamily(u32),
    /// The opcode pops more values than the stack holds.
    StackShort(u32),
    /// Values are left on the stack after the statement.
    StackLeft(u32),
    /// A field is stored with no object register set.
    NoObject(u32),
    /// The arguments are shorter than the family reads.
    ShortArguments(u32),
}

/// What an opcode does, from the names of its handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Lit(u8),
    FrameLoad,
    ArgRef,
    FrameStore,
    IndirectStore,
    ObjectRegister,
    FieldStore,
    GlobalStore,
    Binary(BinaryOp),
    Convert(Option<&'static str>),
    BranchFalse,
    Branch,
    End,
    Exit,
}

/// The type suffixes of the names, and the Basic conversion to each.
const TYPES: &[(&str, &str)] = &[
    ("UI1", "CByte"),
    ("I2", "CInt"),
    ("I4", "CLng"),
    ("R4", "CSng"),
    ("R8", "CDbl"),
    ("Cy", "CCur"),
    ("Str", "CStr"),
    ("Var", "CVar"),
    ("Ad", "CLng"),
    ("FPR4", "CSng"),
    ("FPR8", "CDbl"),
];

/// The operators of the names, longest first so that `IDv` is not read as
/// `Div`.
const OPERATORS: &[(&str, BinaryOp)] = &[
    ("IDv", BinaryOp::IntDiv),
    ("Mod", BinaryOp::Mod),
    ("And", BinaryOp::And),
    ("Xor", BinaryOp::Xor),
    ("Add", BinaryOp::Add),
    ("Sub", BinaryOp::Sub),
    ("Mul", BinaryOp::Mul),
    ("Div", BinaryOp::Div),
    ("Lt", BinaryOp::Lt),
    ("Gt", BinaryOp::Gt),
    ("Eq", BinaryOp::Eq),
    ("Ne", BinaryOp::Ne),
    ("Le", BinaryOp::Le),
    ("Ge", BinaryOp::Ge),
    ("Or", BinaryOp::Or),
];

/// Tells whether `text` is one type suffix.
fn is_type(text: &str) -> bool {
    TYPES.iter().any(|(suffix, _)| *suffix == text)
}

/// The Basic words of the `FnC` names, such as `FnCSngI2`, and their
/// functions.
const FUNCTIONS: &[(&str, &str)] = &[
    ("Byte", "CByte"),
    ("Int", "CInt"),
    ("Lng", "CLng"),
    ("Sng", "CSng"),
    ("Dbl", "CDbl"),
    ("Cur", "CCur"),
    ("Str", "CStr"),
    ("Var", "CVar"),
];

/// Gives the conversion of a name such as `CI4I2` or `FnCSngI2`: the Basic
/// function of the first type, when the rest is a type too.
fn conversion(name: &str) -> Option<&'static str> {
    if let Some(rest) = name.strip_prefix("FnC") {
        return FUNCTIONS.iter().find_map(|(word, function)| {
            rest.strip_prefix(word)
                .filter(|from| is_type(from))
                .map(|_| *function)
        });
    }
    let rest = name.strip_prefix('C')?;
    TYPES.iter().find_map(|(suffix, function)| {
        rest.strip_prefix(suffix)
            .filter(|from| is_type(from))
            .map(|_| *function)
    })
}

/// Gives the family of one name.
fn family_of(name: &str) -> Option<Family> {
    let typed = |prefix: &str| name.strip_prefix(prefix).is_some_and(is_type);
    let family = match name {
        "LitI4" | "LitR4" => Family::Lit(4),
        "LitI2" => Family::Lit(2),
        "LitI2_Byte" => Family::Lit(1),
        "ILdRfDarg" => Family::ArgRef,
        "FLdRf" | "ILdRf" => Family::FrameLoad,
        "FStVarCopy" => Family::FrameStore,
        "FLdPr" => Family::ObjectRegister,
        "BranchF" => Family::BranchFalse,
        "Branch" => Family::Branch,
        "End" => Family::End,
        _ if name.starts_with("ExitProc") => Family::Exit,
        _ if typed("FLd") => Family::FrameLoad,
        _ if typed("ILd") => Family::FrameLoad,
        _ if typed("FSt") => Family::FrameStore,
        _ if typed("ISt") => Family::IndirectStore,
        _ if typed("MemSt") => Family::FieldStore,
        _ if typed("ImpAdSt") => Family::GlobalStore,
        _ => {
            if let Some(op) = OPERATORS.iter().find_map(|(prefix, op)| {
                name.strip_prefix(prefix)
                    .filter(|rest| is_type(rest))
                    .map(|_| *op)
            }) {
                Family::Binary(op)
            } else {
                Family::Convert(Some(conversion(name)?))
            }
        }
    };
    Some(family)
}

/// Gives the one family of the names of a slot.
fn family(names: &[String]) -> Option<Family> {
    let mut found: Vec<Family> = Vec::new();
    for name in names {
        let one = family_of(name)?;
        if !found.contains(&one) {
            found.push(one);
        }
    }
    match found.as_slice() {
        [one] => Some(*one),
        many if many.iter().all(|family| {
            matches!(family, Family::Convert(Some(function)) if *function == "CSng" || *function == "CDbl")
        }) && !many.is_empty() =>
        {
            Some(Family::Convert(None))
        }
        _ => None,
    }
}

/// Reads a signed 16-bit argument.
fn i16_at(arguments: &[u8], at: usize) -> Option<i16> {
    let bytes = arguments.get(at..at.checked_add(2)?)?;
    Some(i16::from_le_bytes(bytes.try_into().ok()?))
}

/// Reads a constant of `len` bytes, sign-extended.
fn constant(arguments: &[u8], len: u8) -> Option<i64> {
    Some(match len {
        1 => i64::from(i8::from_le_bytes([*arguments.first()?])),
        2 => i64::from(i16_at(arguments, 0)?),
        4 => i64::from(i32::from_le_bytes(arguments.get(..4)?.try_into().ok()?)),
        _ => return None,
    })
}

/// Lifts a listing to statements.
///
/// # Errors
///
/// Gives the first [`LiftFault`].
pub fn lift(listing: &PcodeListing, table: &PcodeTable) -> Result<Vec<LiftedStmt>, LiftFault> {
    if !listing.end.is_complete() {
        return Err(LiftFault::NotDecoded(listing.end));
    }
    let mut stack: Vec<Expr> = Vec::new();
    let mut object: Option<Expr> = None;
    let mut out = Vec::new();
    let mut start: Option<u32> = None;
    for instruction in &listing.instructions {
        let PcodeInstruction {
            offset,
            lead,
            opcode,
            arguments,
            ..
        } = instruction;
        let at = *offset;
        let first = *start.get_or_insert(at);
        let names = table
            .slot(*lead, *opcode)
            .map(|slot| slot.names.as_slice())
            .unwrap_or_default();
        let family = family(names).ok_or(LiftFault::NoFamily(at))?;
        let short = || LiftFault::ShortArguments(at);
        let offset16 = || i16_at(arguments, 0).ok_or_else(short);
        let mut pop = || stack.pop().ok_or(LiftFault::StackShort(at));
        let stmt = match family {
            Family::Lit(len) => {
                let value = constant(arguments, len).ok_or_else(short)?;
                stack.push(Expr::Const(value));
                None
            }
            Family::FrameLoad => {
                let slot = Expr::frame(offset16()?);
                stack.push(slot);
                None
            }
            Family::ArgRef => {
                let slot = Expr::frame(offset16()?);
                stack.push(slot);
                None
            }
            Family::ObjectRegister => {
                object = Some(Expr::frame(offset16()?));
                None
            }
            Family::Binary(op) => {
                let right = pop()?;
                let left = pop()?;
                stack.push(Expr::Binary(op, Box::new(left), Box::new(right)));
                None
            }
            Family::Convert(function) => {
                let value = pop()?;
                stack.push(match function {
                    Some(function) => Expr::Convert(function, Box::new(value)),
                    None => value,
                });
                None
            }
            Family::FrameStore | Family::IndirectStore => {
                let value = pop()?;
                Some(Stmt::Assign {
                    target: Expr::frame(offset16()?),
                    value,
                })
            }
            Family::FieldStore => {
                let value = pop()?;
                let base = object.clone().ok_or(LiftFault::NoObject(at))?;
                let field = u16::from_le_bytes(
                    arguments
                        .get(..2)
                        .and_then(|bytes| bytes.try_into().ok())
                        .ok_or_else(short)?,
                );
                Some(Stmt::Assign {
                    target: Expr::Field(Box::new(base), field),
                    value,
                })
            }
            Family::GlobalStore => {
                let value = pop()?;
                let index = u16::from_le_bytes(
                    arguments
                        .get(..2)
                        .and_then(|bytes| bytes.try_into().ok())
                        .ok_or_else(short)?,
                );
                Some(Stmt::Assign {
                    target: Expr::Global(index),
                    value,
                })
            }
            Family::BranchFalse => {
                let condition = pop()?;
                let target = offset16()?;
                Some(Stmt::IfNotGoTo {
                    condition,
                    target: target.cast_unsigned(),
                })
            }
            Family::Branch => Some(Stmt::GoTo(offset16()?.cast_unsigned())),
            Family::End => Some(Stmt::End),
            Family::Exit => Some(Stmt::Exit),
        };
        if let Some(stmt) = stmt {
            if !stack.is_empty() {
                return Err(LiftFault::StackLeft(at));
            }
            out.push(LiftedStmt {
                offset: first,
                stmt,
            });
            start = None;
        }
    }
    Ok(out)
}

/// Renders statements as lines, with a label before each statement that a
/// branch names. A label whose offset starts no statement is given on a
/// line of its own at the end.
#[must_use]
pub fn render(stmts: &[LiftedStmt]) -> Vec<String> {
    let mut targets: Vec<u16> = stmts
        .iter()
        .filter_map(|lifted| match lifted.stmt {
            Stmt::IfNotGoTo { target, .. } | Stmt::GoTo(target) => Some(target),
            Stmt::Assign { .. } | Stmt::End | Stmt::Exit => None,
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    let mut lines = Vec::new();
    for lifted in stmts {
        let label = u16::try_from(lifted.offset)
            .ok()
            .filter(|offset| targets.contains(offset));
        match label {
            Some(offset) => lines.push(format!("L{offset:04X}: {}", lifted.stmt.text())),
            None => lines.push(format!("       {}", lifted.stmt.text())),
        }
    }
    for target in targets {
        if !stmts
            .iter()
            .any(|lifted| u16::try_from(lifted.offset).ok() == Some(target))
        {
            lines.push(format!("L{target:04X}:"));
        }
    }
    lines
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{LiftFault, lift, render};
    use crate::read::region::{Off, Region};
    use crate::vb::pcode::{PcodeTable, disassemble};

    /// A table built here. The opcode numbers are placeholders, chosen for
    /// this test: a table that a runtime gives must not enter this
    /// repository. The names are the names of the families that this module
    /// reads.
    const TABLE: &str = r#"
[primary.01]
width = 4
names = ["LitI4", "LitR4"]
[primary.02]
width = 1
names = ["LitI2_Byte"]
[primary.03]
width = 2
names = ["FLdI4", "FLdR4"]
[primary.04]
width = 2
names = ["ILdI2"]
[primary.05]
width = 2
names = ["FStI4"]
[primary.06]
width = 2
names = ["FLdPr"]
[primary.07]
width = 2
names = ["MemStI2"]
[primary.08]
width = 0
names = ["CI4I2"]
[primary.09]
width = 0
names = ["EqI4"]
[primary.0A]
width = 2
names = ["BranchF"]
[primary.0B]
width = 2
names = ["Branch"]
[primary.0C]
width = 0
names = ["ExitProcHresult"]
[primary.0D]
width = 0
names = ["CR4I2", "CR8I2"]
[primary.0E]
width = 0
names = ["LtI4", "FLdI2"]
[primary.0F]
width = 2
names = ["FLdI4", "FLdStr", "ILdRf"]
[primary.10]
width = 0
names = ["CR4I2", "FnCSngI2"]
[lead1.C8]
width = 0
names = ["End"]
"#;

    fn lines(body: &[u8]) -> Result<Vec<String>, LiftFault> {
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(body, Off::new(0)), &table);
        lift(&listing, &table).map(|stmts| render(&stmts))
    }

    #[test]
    fn a_comparison_and_a_branch_give_an_if_and_a_label() {
        // The shape of `If KeyAscii = vbKeyEscape Then End`: load the
        // argument at 0x0C through its pointer, convert, compare with 27,
        // branch when false past `End`.
        let body = [
            0x04, 0x0C, 0x00, 0x08, 0x01, 27, 0, 0, 0, 0x09, 0x0A, 0x0F, 0x00, 0xFC, 0xC8, 0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap(),
            [
                "       If Not (CLng(arg_C) = 27) Then GoTo L000F",
                "       End",
                "L000F: Exit",
            ]
        );
    }

    #[test]
    fn a_store_to_a_field_uses_the_object_register() {
        // The shape of `Selected = False` and `isAlive = True` in a class.
        let body = [
            0x02, 0x00, 0x06, 0x08, 0x00, 0x07, 0x54, 0x00, 0x02, 0xFF, 0x06, 0x08, 0x00, 0x07,
            0x40, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap(),
            [
                "       Me.field_54 = 0",
                "       Me.field_40 = -1",
                "       Exit"
            ]
        );
    }

    #[test]
    fn a_label_goes_on_the_first_opcode_of_its_statement() {
        // The shape of `If c Then x = a Else x = b`.
        let body = [
            0x03, 0x0C, 0x00, 0x0A, 0x0F, 0x00, 0x03, 0x10, 0x00, 0x05, 0x78, 0xFF, 0x0B, 0x15,
            0x00, 0x03, 0x14, 0x00, 0x05, 0x78, 0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap(),
            [
                "       If Not arg_C Then GoTo L000F",
                "       local_88 = arg_10",
                "       GoTo L0015",
                "L000F: local_88 = arg_14",
                "L0015: Exit",
            ]
        );
    }

    #[test]
    fn the_aliases_of_one_handler_give_one_family() {
        let body = [0x0F, 0x0C, 0x00, 0x10, 0x05, 0x78, 0xFF, 0x0C];
        assert_eq!(lines(&body).unwrap()[0], "       local_88 = CSng(arg_C)");
    }

    #[test]
    fn a_float_conversion_of_two_types_gives_no_conversion() {
        let body = [0x03, 0x0C, 0x00, 0x0D, 0x05, 0x78, 0xFF, 0x0C];
        assert_eq!(lines(&body).unwrap()[0], "       local_88 = arg_C");
    }

    #[test]
    fn each_broken_rule_gives_its_fault() {
        assert_eq!(lines(&[0x09, 0x0C]), Err(LiftFault::StackShort(0)));
        assert_eq!(
            lines(&[0x02, 0x01, 0x02, 0x02, 0x05, 0x78, 0xFF, 0x0C]),
            Err(LiftFault::StackLeft(4))
        );
        assert_eq!(
            lines(&[0x02, 0x01, 0x07, 0x54, 0x00]),
            Err(LiftFault::NoObject(2))
        );
        assert_eq!(lines(&[0x0E, 0x0C]), Err(LiftFault::NoFamily(0)));
        assert!(matches!(lines(&[0x77]), Err(LiftFault::NotDecoded(_))));
    }
}
