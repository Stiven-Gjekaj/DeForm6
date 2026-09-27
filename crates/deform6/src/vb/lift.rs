//! The lift of a decoded P-code body to statements.
//!
//! P-code is a stack machine. [`lift`] follows a [`PcodeListing`] opcode by
//! opcode, keeps a stack of expressions, and gives a statement at each
//! opcode that stores a value, branches, exits, or calls a procedure whose
//! result it does not use. The kind of each opcode
//! comes from the names of its handler in the table that the user derived:
//! this module knows the families of names, not the opcode numbers, and it
//! holds no table.
//!
//! # The families
//!
//! | Names | What the opcode does |
//! |---|---|
//! | `LitI4`, `LitR4`, `LitI2`, `LitI2_Byte` | Push a constant |
//! | `FLd` or `ILd` + a type, `FLdRf`, `FLdRfVar`, `ILdRf` | Push the frame slot at a signed 16-bit offset; the lift does not yet tell a slot from what it points to, and one handler serves names of both kinds |
//! | `ILdRfDarg` | Push the argument at a 16-bit offset |
//! | `FSt` + a type, `FStVarCopy` | Pop into the frame slot |
//! | `ISt` + a type | Pop into what the frame slot points to |
//! | `FLdPr`, `ILdPr` | Set the object register to the frame slot, or to what it points to |
//! | `FLdPrThis` | Set the object register to `Me` |
//! | `MemSt` + a type | Pop into a field of the object register |
//! | `ImpAdSt` + a type | Pop into a global at a 16-bit index |
//! | an operator and two types, such as `LtI4` | Pop two values, push one |
//! | `C` and two types, such as `CI4I2`, or `FnC`, a Basic word and a type, such as `FnCSngI2` | Convert the top value |
//! | `BranchF` | Pop a condition, and branch when it is false |
//! | `Branch` | Branch |
//! | `End`, and a name that starts with `ExitProc` | End the program, or exit |
//! | `ThisVCallHresult` | Call a method of `Me` at a vtable offset |
//! | `ImpAdCall`, `ImpAdCallHresult`, `ImpAdCallFPR4`, `ImpAdCallFPR8` | Call the procedure at an index of the constant table |
//! | `ImpAdCall` + `Ad`, `I2`, `I4`, `Str` or `UI1` | The same, and push the result |
//! | `FFree1Ad`, `FFree1Str`, `FFree1Var`, `FFreeAd`, `FFreeStr`, `FFreeVar` | Free temporary frame slots; the stack does not change, and the lift gives no statement |
//! | `FStAdFunc` | Pop an object into the frame slot |
//! | `VCallAd`, `VCallI2`, `VCallI4`, `VCallStr` | Call through the object register, and push the result: the lift knows only the control accessors of a form |
//! | `VCallHresult` | Call a function of the interface of the object in the object register |
//! | `ImpAdLd` + a type, `ImpAdLdRf`, `ImpAdLdRfVar` | Push a global, or its address, at an index of the constant table |
//! | `ImpAdLdPr` | Set the object register to a global |
//! | `MemLd` + a type, `MemLdRf`, `MemLdRfVar` | Push a field of the object register, or its address |
//! | `MemLdPr` | Set the object register to a field of the object register |
//! | `FMemLd` + a type, `FMemLdRf`, `FMemLdRfVar` | Push a field of the object in a frame slot |
//! | `FMemLdPr` | Set the object register to a field of the object in a frame slot |
//! | `FMemSt` + a type | Pop into a field of the object in a frame slot |
//! | `FStStrCopy` | Pop a string into the frame slot |
//! | `FLdZeroAd`, `FLdZeroStr` | Push the frame slot, and clear it |
//! | `FStStrNoPop`, `FStAdNoPop` | Store the top value into the frame slot, and keep it on the stack |
//! | `PopTmpLdAd` + a size or `Str` | Pop a value into a temporary frame slot, and push its address |
//! | `LitStr` | Push the string at an index of the constant table |
//! | `LitVar_` + `Missing`, `Empty`, `Null`, `TRUE` or `FALSE`, and `LitVar` + `I2`, `I4`, `UI1`, `R4` or `Str` | Write a `Variant` constant into a temporary frame slot, and push its address |
//! | `LitDate`, `LitR8FP` | Push an 8-byte constant on the floating point unit |
//! | `New` | Push a new object of a class at an index of the constant table |
//! | `NewIfNullPr` | Pop the address of an object variable, create the object when the variable is empty, and set the object register to it |
//! | `Bos`, `LargeBos`, `ZeroRetVal`, `ZeroRetValVar`, `SetLastSystemError`, `HardType` | Change no stack and give no statement |
//! | `ConcatStr` | Pop two strings, push the `&` of them |
//! | `FnLenStr`, `FnLenBStr`, `NotI2`, `NotI4`, `CBoolI2`, `CBoolUI1`, `CBoolI4` | Replace the top value with `Len`, `LenB`, `Not` or `CBool` of it |
//! | `FMemStStrCopy` | Pop a string into a field of the object in a frame slot |
//! | `FStAdFuncNoPop` | Store the top object into the frame slot, and keep it on the stack |
//! | `PopAdLdVar` | Pop the address of a `Variant`, and push the 16 bytes of the `Variant` |
//! | `PopFPR4`, `PopFPR8` | Move the value of the floating point unit to the stack, as 4 or 8 bytes |
//! | `CStr2Ansi`, `CStr2Uni` | Pop the address of a frame slot and a string, and write a copy of the string into the slot |
//! | `ForI4`, `ForStepI4`, `NextI4` | A `For` loop over a `Long` counter |
//! | `LateIdLdVar` | Get a property of the object register by its `DISPID`, late bound, into a `Variant` in the frame slot, and push its address |
//! | `PopAd` | Pop a value that nothing uses; a call gives a statement |
//! | `Ary1Ld` + a type, `Ary1St` + a type | Pop a one-dimensional array and an index, and push the element or pop a value into it |
//! | `AryLdPr` | Pop an array and its indexes, and set the object register to the element |
//! | `FnInStr4`, `FnUBound` | `InStr` of four values, `UBound` of two, in the order of the push |
//! | `FStVarCopyObj` | Pop into the `Variant` in the frame slot |
//! | `ForI2`, `NextI2`, `NextStepI4` | The `For` loops of an `Integer` counter, and the end of a loop with a step |
//! | `FnAbsI2`, `FnAbsI4`, `FnAbsR4`, `FnAbsR8`, `FnIntR4`, `FnIntR8`, `CBoolR4`, `CBoolR8` | `Abs`, `Int` or `CBool` of the top value |
//! | `PwrR8R8` | Pop two values, push the `^` of them |
//! | `AryLock`, `AryUnlock` | Lock or unlock an array in a frame slot; the stack does not change |
//! | `Erase` | Pop an array, and `Erase` it |
//! | `CVarRef`, `CVarR4` | Put the top value into a `Variant` in the frame slot, and push its address |
//! | `CDargRef` | Pop an address, and push a `Variant` of 16 bytes that refers to it |
//! | `CopyBytes` | Pop an address and a source, and copy the source there: an assignment of a `Type` |
//! | `Ary1LdPr` | Pop a one-dimensional array and an index, and set the object register to the element |
//! | `NewIfNullAd` | As `NewIfNullPr`, and push the object in the place of setting the object register |
//! | `MemStStrCopy` | Pop a string into a field of the object register |
//! | `Redim` | Pop an array, then a lower and an upper bound for each dimension, and `ReDim` it |
//! | `AryLdRf`, `Ary1LdRf` | Pop an array and its indexes, and push the address of the element |
//! | an operator and `VarBool`, such as `EqVarBool` | Pop two `Variant` values, push the Boolean of the comparison |
//! | `UMiI2`, `UMiI4`, `UMiR4`, `UMiR8` | The negative of the top value |
//! | `CastAd` | Give the top object the class at an index of the constant table |
//! | `ImpAdCallNonVirt` | As `ImpAdCallHresult` |
//! | `IStStrCopy` | Pop a string into what the frame slot points to |
//! | `LitR4FP` | Push a 4-byte floating point constant on the floating point unit |
//! | `PopTmpLdAdFPR4`, `CVarBoolI2` | Pop a value into a temporary slot, and push its address |
//! | `OnErrorGoto` | `On Error GoTo` a label, `On Error Resume Next` for `0xFFFF`, or `On Error GoTo 0` for `0xFFFE` |
//!
//! All the names of one slot must give one family. A conversion whose names
//! give both `CSng` and `CDbl` gives no conversion: one handler serves both,
//! and the store that follows decides the type.
//!
//! # The arguments of a call
//!
//! A called procedure removes its own arguments from the stack, so the
//! opcode of a call does not show how many there are. Two sources give the
//! number of bytes:
//!
//! - An `ImpAdCall` opcode holds it as its second 16-bit argument. The
//!   handler adds it to `esp` before the call, and it stops with an error
//!   when `esp` has another value after the call.
//! - A `ThisVCallHresult` opcode holds a vtable offset. [`Callees`] gives
//!   the method at that offset and the argument size of its descriptor. The
//!   handler pushes `Me` itself, so the opcodes before it push 4 bytes less.
//!
//! - A `VCallHresult` opcode holds a vtable offset in the interface of the
//!   object in the object register. The file that `derive-vb-types` writes
//!   ([`VbTypes`]) gives the argument bytes of the function there. The
//!   handler pushes the object itself.
//!
//! The lift pops values until their sizes add up to the bytes of the call.
//! It refuses the call when a popped value has no known size on the stack,
//! such as a value of the floating point unit, or when the sizes do not add
//! up to the bytes. A value of 4 bytes is a value of a type of 4 bytes or
//! less, or an address; `PopAdLdVar` gives a `Variant` of 16 bytes. An
//! `ImpAdCall` handler that serves `ImpAdCallFPR4` and `ImpAdCallFPR8` too
//! can leave a result on the floating point unit, which is not the stack.
//! The lift gives it no result, so an opcode that uses the result finds an
//! empty stack and the lift stops there.
//!
//! # Objects
//!
//! The lift keeps the class of a value when it knows it: the interface of
//! a control. `VCallAd` on `Me` at a control accessor of a form gives the
//! control, with the interface that [`Callees`] names for it. `FStAdFunc`
//! stores an object in a frame slot, and the lift binds the slot to the
//! object: a later load of the slot gives the object. When values stay on
//! the stack, the slot is a temporary one of the expression, and the lift
//! gives no statement. When the stack is empty, the slot is a temporary one
//! when an `FFree` frees it later, as the compiler does after a call of a
//! control. Else the lift gives `Set` at the place of the store. A property
//! get writes
//! its result through its last argument, the address of a frame slot, and
//! the lift binds that slot to the call in the same way.
//!
//! A store into a frame slot while other values stay on the stack is a
//! store into a temporary slot of an expression, such as an argument that
//! the compiler computes before the others. The lift binds the slot and
//! gives the assignment only when no `FFree` frees the slot, as for an
//! object.
//!
//! `NewIfNullPr` names a class of the constant table. When [`Callees`]
//! gives the profile of that class, an object of the project, a call on the
//! object finds a control accessor, a method, or the get or the let of a
//! public variable there. The lift names a variable by its field, such as
//! `global_7.field_34`, as it names a field of `Me`. When the class is a
//! class of the runtime, such as its global object, [`Callees`] can give
//! its interface, and a call finds its function in [`VbTypes`].
//!
//! The class of an object argument of a procedure is not in the binary.
//! [`method_calls`] gives the classes of the arguments of each call of a
//! method of `Me`. When every call gives an argument one class,
//! `vb::context` gives it to [`Callees`], and [`lift_method`] lifts the
//! procedure with it.
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

use std::collections::{BTreeMap, BTreeSet};

use crate::vb::pcode::{PcodeEnd, PcodeInstruction, PcodeListing, PcodeTable};
use crate::vb::types::{TypeFunction, VbTypes};

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
    /// `&`
    Concat,
    /// `^`
    Pow,
    /// `To`, between the bounds of a dimension of `ReDim`.
    To,
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
            Self::Concat => "&",
            Self::Pow => "^",
            Self::To => "To",
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
    /// A call and its arguments, first argument first.
    Call(Callee, Vec<Expr>),
    /// A named member of an object, such as a control of a form or a
    /// property.
    Member(Box<Expr>, String),
    /// A Basic word: `Missing`, `Empty` or `Null`.
    Word(&'static str),
    /// An 8-byte floating point constant, by its bits.
    Real(u64),
    /// A string of the constant table.
    Str(String),
    /// An entry of the constant table whose text the lift does not know.
    Constant(u16),
    /// A new object of the class at an index of the constant table.
    New(u16),
    /// A member of an object, late bound, by its `DISPID`.
    Late(Box<Expr>, u32),
    /// An element of an array.
    Index(Box<Expr>, Vec<Expr>),
    /// The value that a store put into a frame slot, with the number of
    /// the store and the slot. The end of the lift replaces it: with the
    /// slot when the statement of the store stays, and with the value when
    /// an `FFree` drops that statement.
    Bound(u32, i16, Box<Expr>),
}

/// The procedure that a call names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Callee {
    /// A method of `Me`, by its index in the method table of the object.
    Method(u16),
    /// A procedure, by its index in the constant table of the object.
    Import(u16),
    /// A named function of an object.
    Member(Box<Expr>, String),
}

impl Callee {
    /// The text of the callee.
    fn text(&self) -> String {
        match self {
            Self::Method(index) => format!("Me.method_{index}"),
            Self::Import(index) => format!("import_{index:X}"),
            Self::Member(object, name) => format!("{}.{name}", object.text()),
        }
    }

    /// Replaces each [`Expr::Bound`] of the callee, as [`Expr::resolve`].
    fn resolve(self, kept: &BTreeSet<u32>) -> Self {
        match self {
            Self::Member(object, name) => Self::Member(Box::new(object.resolve(kept)), name),
            other => other,
        }
    }
}

/// The methods that a `ThisVCallHresult` of one object can call, by vtable
/// offset. `vb::links::MethodLinks::callees` builds it from the image.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Callees {
    methods: Vec<(u16, CalledMethod)>,
    controls: Vec<(u16, String, String)>,
    strings: Vec<(u16, String)>,
    variables: Vec<(u16, u32, bool)>,
    classes: Vec<(u16, Callees)>,
    class_interfaces: Vec<(u16, String)>,
    arguments: Vec<(u16, i16, String)>,
    base: Option<String>,
    owner: Option<u16>,
}

/// A method that [`Callees`] gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CalledMethod {
    /// The index of the method in the method table of the object.
    pub index: u16,
    /// The argument size of its descriptor, with the 4 bytes of `Me`.
    pub arg_size: u16,
}

impl Callees {
    /// Adds the method `index` at `vtable_offset`, with the argument size
    /// `arg_size` of its descriptor.
    #[must_use]
    pub fn with_method(mut self, vtable_offset: u16, index: u16, arg_size: u16) -> Self {
        self.methods
            .push((vtable_offset, CalledMethod { index, arg_size }));
        self
    }

    /// Adds the control accessor of a form at `vtable_offset`: the control
    /// `name`, of the interface `interface`.
    #[must_use]
    pub fn with_control(mut self, vtable_offset: u16, name: &str, interface: &str) -> Self {
        self.controls
            .push((vtable_offset, name.to_owned(), interface.to_owned()));
        self
    }

    /// Adds the accessor of a public variable at `vtable_offset`: the get
    /// when `get` is true, else the let, of the field at `field`.
    #[must_use]
    pub fn with_variable(mut self, vtable_offset: u16, field: u32, get: bool) -> Self {
        self.variables.push((vtable_offset, field, get));
        self
    }

    /// Gives the field and whether it is the get of the accessor at
    /// `vtable_offset`.
    #[must_use]
    pub fn variable(&self, vtable_offset: u16) -> Option<(u32, bool)> {
        self.variables
            .iter()
            .find(|(offset, _, _)| *offset == vtable_offset)
            .map(|(_, field, get)| (*field, *get))
    }

    /// Adds the class at `index` of the constant table: an object of the
    /// project whose vtable `profile` gives.
    #[must_use]
    pub fn with_class(mut self, index: u16, profile: Self) -> Self {
        self.classes.push((index, profile));
        self
    }

    /// Sets the owner of the profile: the index of its object in the
    /// project. A call of a method records it.
    #[must_use]
    pub const fn with_owner(mut self, owner: u16) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Sets the base interface of the object, such as `_Form`: its vtable
    /// comes before the control accessors and the link table.
    #[must_use]
    pub fn with_base(mut self, interface: &str) -> Self {
        self.base = Some(interface.to_owned());
        self
    }

    /// Gives the base interface of the object.
    #[must_use]
    pub fn base(&self) -> Option<&str> {
        self.base.as_deref()
    }

    /// Adds the interface `interface` of the argument at the frame offset
    /// `slot` of the method `method`.
    #[must_use]
    pub fn with_argument(mut self, method: u16, slot: i16, interface: &str) -> Self {
        self.arguments.push((method, slot, interface.to_owned()));
        self
    }

    /// Gives the frame offset and the interface of each argument of the
    /// method `method` whose interface [`Callees`] gives.
    #[must_use]
    pub fn arguments_of(&self, method: u16) -> Vec<(i16, String)> {
        self.arguments
            .iter()
            .filter(|(at, _, _)| *at == method)
            .map(|(_, slot, interface)| (*slot, interface.clone()))
            .collect()
    }

    /// Adds the class at `index` of the constant table that is a class of
    /// the runtime, with the interface `interface`.
    #[must_use]
    pub fn with_class_interface(mut self, index: u16, interface: &str) -> Self {
        self.class_interfaces.push((index, interface.to_owned()));
        self
    }

    /// Gives the interface of the class of the runtime at `index` of the
    /// constant table.
    fn class_interface(&self, index: u16) -> Option<&str> {
        self.class_interfaces
            .iter()
            .find(|(at, _)| *at == index)
            .map(|(_, interface)| interface.as_str())
    }

    /// Gives the profile of the class at `index` of the constant table.
    #[must_use]
    pub fn class(&self, index: u16) -> Option<&Self> {
        self.classes
            .iter()
            .find(|(at, _)| *at == index)
            .map(|(_, profile)| profile)
    }

    /// Adds the string `text` at `index` of the constant table.
    #[must_use]
    pub fn with_string(mut self, index: u16, text: &str) -> Self {
        self.strings.push((index, text.to_owned()));
        self
    }

    /// Gives the expression of the string at `index` of the constant table.
    fn string(&self, index: u16) -> Expr {
        self.strings
            .iter()
            .find(|(at, _)| *at == index)
            .map_or(Expr::Constant(index), |(_, text)| Expr::Str(text.clone()))
    }

    /// Gives the name and the interface of the control at `vtable_offset`.
    #[must_use]
    pub fn control(&self, vtable_offset: u16) -> Option<(&str, &str)> {
        self.controls
            .iter()
            .find(|(offset, _, _)| *offset == vtable_offset)
            .map(|(_, name, interface)| (name.as_str(), interface.as_str()))
    }

    /// Gives the method at `vtable_offset`.
    #[must_use]
    pub fn method(&self, vtable_offset: u16) -> Option<CalledMethod> {
        self.methods
            .iter()
            .find(|(offset, _)| *offset == vtable_offset)
            .map(|(_, method)| *method)
    }
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
            Self::Call(callee, args) => format!("{}({})", callee.text(), arguments_text(args)),
            Self::Member(object, name) => format!("{}.{name}", object.text()),
            Self::Word(word) => (*word).to_owned(),
            Self::Real(bits) => format!("{:?}", f64::from_bits(*bits)),
            Self::Str(text) => format!("\"{}\"", text.replace('"', "\"\"")),
            Self::Constant(index) => format!("const_{index:X}"),
            Self::New(index) => format!("New class_{index:X}"),
            Self::Late(object, dispid) => format!("{}.[DISPID {dispid:#X}]", object.text()),
            Self::Index(array, indexes) => format!("{}({})", array.text(), arguments_text(indexes)),
            Self::Bound(_, _, value) => value.text(),
        }
    }

    /// Replaces each [`Expr::Bound`]: with its slot when `kept` holds the
    /// number of its store, and else with its value.
    fn resolve(self, kept: &BTreeSet<u32>) -> Self {
        let each = |exprs: Vec<Self>| exprs.into_iter().map(|expr| expr.resolve(kept)).collect();
        match self {
            Self::Bound(store, slot, _) if kept.contains(&store) => Self::frame(slot),
            Self::Bound(_, _, value) => value.resolve(kept),
            Self::Field(object, offset) => Self::Field(Box::new(object.resolve(kept)), offset),
            Self::Binary(op, left, right) => Self::Binary(
                op,
                Box::new(left.resolve(kept)),
                Box::new(right.resolve(kept)),
            ),
            Self::Convert(function, value) => {
                Self::Convert(function, Box::new(value.resolve(kept)))
            }
            Self::Call(callee, args) => Self::Call(callee.resolve(kept), each(args)),
            Self::Member(object, name) => Self::Member(Box::new(object.resolve(kept)), name),
            Self::Late(object, dispid) => Self::Late(Box::new(object.resolve(kept)), dispid),
            Self::Index(array, indexes) => {
                Self::Index(Box::new(array.resolve(kept)), each(indexes))
            }
            other => other,
        }
    }

    /// The expression without the [`Expr::Bound`] around it.
    fn unbound(self) -> Self {
        match self {
            Self::Bound(_, _, value) => value.unbound(),
            other => other,
        }
    }
}

/// The text of a list of arguments.
fn arguments_text(args: &[Expr]) -> String {
    args.iter().map(Expr::text).collect::<Vec<_>>().join(", ")
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
    /// A call whose result is not used.
    Call(Callee, Vec<Expr>),
    /// The start of a `For` loop. When the counter is past the end, the
    /// loop goes to `exit`.
    For {
        /// The counter.
        counter: Expr,
        /// The first value.
        start: Expr,
        /// The last value.
        end: Expr,
        /// The step, when it is not 1.
        step: Option<Expr>,
        /// The offset after the loop.
        exit: u16,
    },
    /// The end of a `For` loop: the next value of the counter, and a branch
    /// back to `body` while the counter is not past the end.
    Next {
        /// The counter.
        counter: Expr,
        /// The offset of the first statement of the loop.
        body: u16,
    },
    /// `On Error`: `Some` target offset, or `None` for `Resume Next`, or
    /// `Some(0)` for `GoTo 0`.
    OnError(Option<u16>),
    /// A store of an object.
    Set {
        /// Where the object goes.
        target: Expr,
        /// The object.
        value: Expr,
    },
}

impl Stmt {
    /// Replaces each [`Expr::Bound`] of the statement, as [`Expr::resolve`].
    fn resolve(self, kept: &BTreeSet<u32>) -> Self {
        match self {
            Self::Assign { target, value } => Self::Assign {
                target: target.resolve(kept),
                value: value.resolve(kept),
            },
            Self::Set { target, value } => Self::Set {
                target: target.resolve(kept),
                value: value.resolve(kept),
            },
            Self::IfNotGoTo { condition, target } => Self::IfNotGoTo {
                condition: condition.resolve(kept),
                target,
            },
            Self::Call(callee, args) => Self::Call(
                callee.resolve(kept),
                args.into_iter().map(|arg| arg.resolve(kept)).collect(),
            ),
            Self::For {
                counter,
                start,
                end,
                step,
                exit,
            } => Self::For {
                counter: counter.resolve(kept),
                start: start.resolve(kept),
                end: end.resolve(kept),
                step: step.map(|step| step.resolve(kept)),
                exit,
            },
            Self::Next { counter, body } => Self::Next {
                counter: counter.resolve(kept),
                body,
            },
            other => other,
        }
    }

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
            Self::Call(callee, args) => format!("Call {}({})", callee.text(), arguments_text(args)),
            Self::Set { target, value } => format!("Set {} = {}", target.text(), value.text()),
            Self::For {
                counter,
                start,
                end,
                step,
                exit,
            } => {
                let step = step
                    .as_ref()
                    .map(|step| format!(" Step {}", step.text()))
                    .unwrap_or_default();
                format!(
                    "For {} = {} To {}{step}  ' past the end: GoTo L{exit:04X}",
                    counter.text(),
                    start.text(),
                    end.text()
                )
            }
            Self::Next { counter, body } => {
                format!("Next {}  ' loop: GoTo L{body:04X}", counter.text())
            }
            Self::OnError(None) => "On Error Resume Next".to_owned(),
            Self::OnError(Some(0)) => "On Error GoTo 0".to_owned(),
            Self::OnError(Some(target)) => format!("On Error GoTo L{target:04X}"),
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
    /// A call names a vtable offset that [`Callees`] does not give.
    NoCallee(u32),
    /// The argument bytes of a call are not a whole number of values that
    /// are 4 bytes on the stack.
    CallArguments(u32),
    /// A call through the object register names a function that the lift
    /// cannot find: the class of the object is not known, or its interface
    /// holds no function at the offset.
    NoFunction(u32),
    /// A property get gives its result through an argument that is not the
    /// address of a frame slot.
    NoResultSlot(u32),
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
    ObjectRegisterThis,
    FieldStore,
    GlobalStore,
    Binary(BinaryOp),
    Convert(Option<&'static str>),
    BranchFalse,
    Branch,
    End,
    Exit,
    ThisCall,
    ImportCall { result: bool },
    Free,
    ObjectStore,
    ObjectCall { pushes: bool },
    GlobalLoad,
    GlobalObjectRegister,
    FieldLoad,
    FieldObjectRegister,
    FrameFieldLoad,
    FrameFieldObjectRegister,
    FrameFieldStore,
    StoreKeep { object: bool },
    PopTemp,
    LitString,
    LitVariant(VariantKind),
    LitReal,
    NewObject,
    NewIfNull,
    Nop,
    Function(&'static str),
    FunctionOf(&'static str, u8),
    FloatFunction(&'static str),
    ArrayErase,
    Redim,
    ArrayReference { dimensions_argument: bool },
    LitSingle,
    CopyBytes,
    ArrayElementRegister,
    NewIfNullPush,
    LateGet,
    Discard,
    ArrayLoad,
    ArrayStore,
    ArrayObjectRegister,
    Sized(u8),
    StringCopy,
    For { step: bool },
    Next,
    OnError,
}

/// The constant of a `Variant` literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VariantKind {
    Word(&'static str),
    Byte,
    Int(u8),
    Single,
    Str,
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
        "FLdRf" | "FLdRfVar" | "ILdRf" => Family::FrameLoad,
        "FStVarCopy" => Family::FrameStore,
        "FLdPr" | "ILdPr" => Family::ObjectRegister,
        "FLdPrThis" => Family::ObjectRegisterThis,
        "BranchF" => Family::BranchFalse,
        "Branch" => Family::Branch,
        "End" => Family::End,
        "ThisVCallHresult" => Family::ThisCall,
        "ImpAdCall" | "ImpAdCallHresult" | "ImpAdCallFPR4" | "ImpAdCallFPR8" => {
            Family::ImportCall { result: false }
        }
        "ImpAdCallAd" | "ImpAdCallI2" | "ImpAdCallI4" | "ImpAdCallStr" | "ImpAdCallUI1" => {
            Family::ImportCall { result: true }
        }
        "FStAdFunc" => Family::ObjectStore,
        "ImpAdLdRf" | "ImpAdLdRfVar" => Family::GlobalLoad,
        "ImpAdLdPr" => Family::GlobalObjectRegister,
        "MemLdRf" | "MemLdRfVar" => Family::FieldLoad,
        "MemLdPr" => Family::FieldObjectRegister,
        "FMemLdRf" | "FMemLdRfVar" => Family::FrameFieldLoad,
        "FMemLdPr" => Family::FrameFieldObjectRegister,
        "FStStrCopy" => Family::FrameStore,
        "FLdZeroAd" | "FLdZeroStr" => Family::FrameLoad,
        "FStStrNoPop" => Family::StoreKeep { object: false },
        "FStAdNoPop" => Family::StoreKeep { object: true },
        "PopTmpLdAd1" | "PopTmpLdAd2" | "PopTmpLdAd4" | "PopTmpLdAdStr" => Family::PopTemp,
        "LitStr" => Family::LitString,
        "LitVar_Missing" => Family::LitVariant(VariantKind::Word("Missing")),
        "LitVar_Empty" => Family::LitVariant(VariantKind::Word("Empty")),
        "LitVar_Null" => Family::LitVariant(VariantKind::Word("Null")),
        "LitVar_TRUE" => Family::LitVariant(VariantKind::Word("True")),
        "LitVar_FALSE" => Family::LitVariant(VariantKind::Word("False")),
        "LitVarUI1" => Family::LitVariant(VariantKind::Byte),
        "LitVarI2" => Family::LitVariant(VariantKind::Int(2)),
        "LitVarI4" => Family::LitVariant(VariantKind::Int(4)),
        "LitVarR4" => Family::LitVariant(VariantKind::Single),
        "LitVarStr" => Family::LitVariant(VariantKind::Str),
        "LitDate" | "LitR8FP" => Family::LitReal,
        "New" => Family::NewObject,
        "NewIfNullPr" => Family::NewIfNull,
        "Bos" | "LargeBos" | "ZeroRetVal" | "ZeroRetValVar" | "SetLastSystemError" | "HardType" => {
            Family::Nop
        }
        "ConcatStr" => Family::Binary(BinaryOp::Concat),
        "LateIdLdVar" => Family::LateGet,
        "Redim" => Family::Redim,
        "AryLdRf" => Family::ArrayReference {
            dimensions_argument: true,
        },
        "Ary1LdRf" => Family::ArrayReference {
            dimensions_argument: false,
        },
        "EqVarBool" => Family::Binary(BinaryOp::Eq),
        "NeVarBool" => Family::Binary(BinaryOp::Ne),
        "LtVarBool" => Family::Binary(BinaryOp::Lt),
        "GtVarBool" => Family::Binary(BinaryOp::Gt),
        "LeVarBool" => Family::Binary(BinaryOp::Le),
        "GeVarBool" => Family::Binary(BinaryOp::Ge),
        "UMiI2" | "UMiI4" => Family::Function("-"),
        "UMiR4" | "UMiR8" => Family::FloatFunction("-"),
        "CastAd" => Family::NewIfNullPush,
        "ImpAdCallNonVirt" => Family::ImportCall { result: false },
        "IStStrCopy" => Family::IndirectStore,
        "LitR4FP" => Family::LitSingle,
        "PopTmpLdAdFPR4" | "CVarBoolI2" => Family::PopTemp,
        "FnAbsI2" | "FnAbsI4" => Family::Function("Abs"),
        "FnAbsR4" | "FnAbsR8" => Family::FloatFunction("Abs"),
        "FnIntR4" | "FnIntR8" => Family::FloatFunction("Int"),
        "CBoolR4" | "CBoolR8" => Family::Function("CBool"),
        "PwrR8R8" => Family::Binary(BinaryOp::Pow),
        "AryLock" | "AryUnlock" => Family::Nop,
        "Erase" => Family::ArrayErase,
        "CVarRef" | "CVarR4" => Family::PopTemp,
        "CDargRef" => Family::Sized(16),
        "CopyBytes" => Family::CopyBytes,
        "Ary1LdPr" => Family::ArrayElementRegister,
        "NewIfNullAd" => Family::NewIfNullPush,
        "MemStStrCopy" => Family::FieldStore,
        "PopAd" => Family::Discard,
        "AryLdPr" => Family::ArrayObjectRegister,
        "FnInStr4" => Family::FunctionOf("InStr", 4),
        "FnUBound" => Family::FunctionOf("UBound", 2),
        "FStVarCopyObj" => Family::FrameStore,
        "ForI2" => Family::For { step: false },
        "NextI2" | "NextStepI4" => Family::Next,
        _ if typed("Ary1Ld") => Family::ArrayLoad,
        _ if typed("Ary1St") => Family::ArrayStore,
        "FnLenStr" => Family::Function("Len"),
        "FnLenBStr" => Family::Function("LenB"),
        "NotI2" | "NotI4" => Family::Function("Not"),
        "CBoolI2" | "CBoolUI1" | "CBoolI4" => Family::Function("CBool"),
        "FMemStStrCopy" => Family::FrameFieldStore,
        "FStAdFuncNoPop" => Family::StoreKeep { object: true },
        "PopAdLdVar" => Family::Sized(16),
        "PopFPR4" => Family::Sized(4),
        "PopFPR8" => Family::Sized(8),
        "CStr2Ansi" | "CStr2Uni" => Family::StringCopy,
        "ForI4" => Family::For { step: false },
        "ForStepI4" => Family::For { step: true },
        "NextI4" => Family::Next,
        "OnErrorGoto" => Family::OnError,
        _ if typed("ImpAdLd") => Family::GlobalLoad,
        _ if typed("MemLd") => Family::FieldLoad,
        _ if typed("FMemLd") => Family::FrameFieldLoad,
        _ if typed("FMemSt") => Family::FrameFieldStore,
        "VCallAd" | "VCallI2" | "VCallI4" | "VCallStr" => Family::ObjectCall { pushes: true },
        "VCallHresult" => Family::ObjectCall { pushes: false },
        "FFree1Ad" | "FFree1Str" | "FFree1Var" | "FFreeAd" | "FFreeStr" | "FFreeVar" => {
            Family::Free
        }
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

/// The types of a value that is 4 bytes on the stack.
const WORD_TYPES: &[&str] = &["UI1", "I2", "I4", "Ad", "Str", "Bool"];

/// The types of a value that is not 4 bytes on the stack, or that is on
/// the floating point unit.
const WIDE_TYPES: &[&str] = &["FPR4", "FPR8", "R8", "Cy", "Var"];

/// Tells whether the names of a slot that loads or computes a value give a
/// value of 4 bytes on the stack. One name must end in a type of 4 bytes,
/// or name a reference (`Rf`), and no name may end in a wide type. A name
/// that ends in `R4` is neither: `LitR4` and `FLdR4` share the handler of
/// `LitI4` and `FLdI4`, but an operator on `R4` uses the floating point
/// unit.
fn is_word(names: &[String]) -> bool {
    let reference = |name: &String| name.contains("Rf");
    names
        .iter()
        .any(|name| reference(name) || WORD_TYPES.iter().any(|kind| name.ends_with(kind)))
        && !names
            .iter()
            .any(|name| !reference(name) && WIDE_TYPES.iter().any(|kind| name.ends_with(kind)))
}

/// Tells whether an opcode of the name `name` takes a value of the floating
/// point unit: a name that ends in a floating point type, or `CVarR4`,
/// `CVarR8` and the conversions from them.
fn takes_a_float(name: &str) -> bool {
    ["R4", "R8", "FPR4", "FPR8"]
        .iter()
        .any(|kind| name.ends_with(kind))
}

/// Reads an unsigned 16-bit argument.
fn u16_at(arguments: &[u8], at: usize) -> Option<u16> {
    let bytes = arguments.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
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

/// The Basic conversions whose result is 4 bytes on the stack.
const WORD_CONVERSIONS: &[&str] = &["CByte", "CInt", "CLng", "CStr"];

/// The name of a default member, which the lift names only when the
/// interface gives no other name at the offset.
const DEFAULT_MEMBER: &str = "_Default";

/// A value on the stack.
#[derive(Clone, Debug)]
struct Value {
    /// The expression.
    expr: Expr,
    /// The bytes of the value on the stack, or 0 when the lift does not know
    /// them: a value on the floating point unit has 0 bytes on the stack.
    bytes: u8,
    /// The frame slot that the opcode addressed, for a load of a slot.
    slot: Option<i16>,
    /// The interface of the object, when the value is an object whose class
    /// the lift knows.
    class: Option<String>,
}

impl Value {
    /// A value with no slot and no class.
    const fn plain(expr: Expr, word: bool) -> Self {
        Self {
            expr,
            bytes: if word { 4 } else { 0 },
            slot: None,
            class: None,
        }
    }
}

/// An object and its interface, when the lift knows it.
type Object = (Expr, Option<String>);

/// Pops the arguments of a call of `bytes` bytes, first argument first.
/// Each popped value must have a known size, and the sizes must add up to
/// `bytes`.
fn call_arguments(stack: &mut Vec<Value>, bytes: u16, at: u32) -> Result<Vec<Value>, LiftFault> {
    if bytes.checked_rem(4) != Some(0) {
        return Err(LiftFault::CallArguments(at));
    }
    let mut left = bytes;
    let mut args = Vec::new();
    while left > 0 {
        let value = stack.pop().ok_or(LiftFault::StackShort(at))?;
        left = left
            .checked_sub(u16::from(value.bytes))
            .filter(|_| value.bytes > 0)
            .ok_or(LiftFault::CallArguments(at))?;
        args.push(value);
    }
    Ok(args)
}

/// The expressions of the arguments.
fn expressions(args: Vec<Value>) -> Vec<Expr> {
    args.into_iter().map(|value| value.expr).collect()
}

/// The name that the lift gives the functions at one offset: the first name
/// that is not the default member, or else the first name.
fn member_name(function: &TypeFunction) -> String {
    function
        .names
        .iter()
        .find(|name| *name != DEFAULT_MEMBER)
        .or_else(|| function.names.first())
        .cloned()
        .unwrap_or_default()
}

/// A `Set` that the lift gives only when no `FFree` frees its slot: the
/// number of its store, the index of the statement that it goes before, and
/// the statement.
type PendingSet = (u32, usize, LiftedStmt);

/// The things that the lift keeps between opcodes.
#[derive(Default)]
struct State {
    stack: Vec<Value>,
    object: Option<Object>,
    bindings: BTreeMap<i16, Object>,
    pending: BTreeMap<i16, PendingSet>,
    /// The pending statements that a later store to the same slot replaced
    /// before an `FFree`: they stay.
    kept: Vec<PendingSet>,
    stores: u32,
}

/// Reads the frame slots that an `FFree` opcode frees: one slot, or a 16-bit
/// byte count and that many bytes of slots.
fn freed_slots(names: &[String], arguments: &[u8]) -> Option<Vec<i16>> {
    if names.iter().any(|name| name.starts_with("FFree1")) {
        return Some(vec![i16_at(arguments, 0)?]);
    }
    let bytes = usize::from(u16_at(arguments, 0)?);
    let slots = arguments.get(2..bytes.checked_add(2)?)?;
    slots
        .chunks(2)
        .map(|pair| Some(i16::from_le_bytes(pair.try_into().ok()?)))
        .collect()
}

impl State {
    /// Binds `slot` to `value`, and keeps `stmt` until an `FFree` of the slot
    /// drops it. A statement that is still pending for the slot stays.
    fn bind_pending(&mut self, slot: i16, value: Value, index: usize, stmt: LiftedStmt) {
        if let Some(old) = self.pending.remove(&slot) {
            self.kept.push(old);
        }
        let store = self.stores;
        self.stores = store.saturating_add(1);
        self.pending.insert(slot, (store, index, stmt));
        self.bindings.insert(
            slot,
            (Expr::Bound(store, slot, Box::new(value.expr)), value.class),
        );
    }

    /// The value of a load of the frame slot at `offset`: the object that a
    /// temporary slot is bound to, or the slot.
    fn load(&self, offset: i16, word: bool) -> Value {
        let (expr, class) = self
            .bindings
            .get(&offset)
            .cloned()
            .unwrap_or_else(|| (Expr::frame(offset), None));
        Value {
            expr,
            bytes: if word { 4 } else { 0 },
            slot: Some(offset),
            class,
        }
    }
}

/// The prefix of the class of the object of a control array, before the
/// interface of its controls.
pub(crate) const CONTROL_ARRAY: &str = "[]";

/// Gives the function at `vtable_offset` of the object of a control array
/// whose controls have the interface `element`.
///
/// The runtime calls this object `tagCARR`. Its vtable for `IVBControl` in
/// `MSVBVM60.DLL` 6.0.98.2 holds `Item(Integer, CTL**)` at `0x40`, then
/// `LBound`, `UBound` and `Count` at `0x44`, `0x48` and `0x4C`. Each of the
/// last three writes an `Integer`. No type library gives them.
fn control_array_function(element: &str, vtable_offset: u16) -> Option<TypeFunction> {
    let (name, arg_bytes, result_interface) = match vtable_offset {
        0x40 => ("Item", 8, Some(element.to_owned())),
        0x44 => ("LBound", 4, None),
        0x48 => ("UBound", 4, None),
        0x4C => ("Count", 4, None),
        _ => return None,
    };
    Some(TypeFunction {
        names: vec![name.to_owned()],
        kinds: vec!["get".to_owned()],
        arg_bytes: Some(arg_bytes),
        result: true,
        result_interface,
    })
}

/// The prefix of the class of an object of the project, before the index of
/// its class in the constant table.
const PROJECT_CLASS: char = '@';

/// Lifts a call of the function at `vtable_offset` of the interface
/// `interface` on `object`.
fn interface_call(
    state: &mut State,
    types: Option<&VbTypes>,
    interface: &str,
    object: Expr,
    vtable_offset: u16,
    at: u32,
) -> Result<Option<Stmt>, LiftFault> {
    let function = match interface.strip_prefix(CONTROL_ARRAY) {
        Some(element) => control_array_function(element, vtable_offset),
        None => types
            .and_then(|types| types.interface(interface)?.function(vtable_offset))
            .cloned(),
    }
    .ok_or(LiftFault::NoFunction(at))?;
    let function = &function;
    let bytes = function.arg_bytes.ok_or(LiftFault::CallArguments(at))?;
    let mut args = call_arguments(&mut state.stack, bytes, at)?;
    let name = member_name(function);
    let has = |kind: &str| function.kinds.iter().any(|one| one == kind);
    if function.result {
        let slot = args
            .pop()
            .and_then(|value| value.slot)
            .ok_or(LiftFault::NoResultSlot(at))?;
        let value = if args.is_empty() && has("get") {
            Expr::Member(Box::new(object), name)
        } else {
            Expr::Call(Callee::Member(Box::new(object), name), expressions(args))
        };
        state
            .bindings
            .insert(slot, (value, function.result_interface.clone()));
        return Ok(None);
    }
    let target = Expr::Member(Box::new(object.clone()), name.clone());
    Ok(Some(match (args.as_slice(), has("let"), has("set")) {
        ([_], true, _) => Stmt::Assign {
            target,
            value: expressions(args).remove(0),
        },
        ([_], false, true) => Stmt::Set {
            target,
            value: expressions(args).remove(0),
        },
        _ => Stmt::Call(Callee::Member(Box::new(object), name), expressions(args)),
    }))
}

/// Lifts a call on an object of the project whose vtable `profile` gives:
/// `Me`, or an object of a class of the constant table. An offset in the
/// base interface of the object, such as `_Form`, is a call of that
/// interface.
#[allow(
    clippy::too_many_arguments,
    reason = "the state of the lift, the profile, the call and the record of it"
)]
fn project_call(
    state: &mut State,
    profile: &Callees,
    types: Option<&VbTypes>,
    object: Expr,
    vtable_offset: u16,
    pushes: bool,
    at: u32,
    calls: &mut Vec<MethodCall>,
) -> Result<Option<Stmt>, LiftFault> {
    if pushes {
        let (name, interface) = profile
            .control(vtable_offset)
            .ok_or(LiftFault::NoFunction(at))?;
        state.stack.push(Value {
            expr: Expr::Member(Box::new(object), name.to_owned()),
            bytes: 4,
            slot: None,
            class: Some(interface.to_owned()),
        });
        return Ok(None);
    }
    if let Some(base) = profile.base()
        && types
            .and_then(|types| types.interface(base))
            .is_some_and(|interface| vtable_offset < interface.vtable_size)
    {
        return interface_call(state, types, base, object, vtable_offset, at);
    }
    if let Some(method) = profile.method(vtable_offset) {
        let bytes = method
            .arg_size
            .checked_sub(4)
            .ok_or(LiftFault::CallArguments(at))?;
        let args = call_arguments(&mut state.stack, bytes, at)?;
        calls.push((
            profile.owner,
            method.index,
            args.iter()
                .map(|value| (value.bytes, value.class.clone()))
                .collect(),
        ));
        let callee = Callee::Member(Box::new(object), format!("method_{}", method.index));
        return Ok(Some(Stmt::Call(callee, expressions(args))));
    }
    let (field, get) = profile
        .variable(vtable_offset)
        .ok_or(LiftFault::NoFunction(at))?;
    let field = u16::try_from(field).map_err(|_| LiftFault::NoFunction(at))?;
    let target = Expr::Field(Box::new(object), field);
    let value = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
    if get {
        let slot = value.slot.ok_or(LiftFault::NoResultSlot(at))?;
        state.bindings.insert(slot, (target, None));
        return Ok(None);
    }
    if value.bytes == 0 {
        return Err(LiftFault::CallArguments(at));
    }
    Ok(Some(Stmt::Assign {
        target,
        value: value.expr,
    }))
}

/// Lifts a call through the object register at `vtable_offset`.
fn object_call(
    state: &mut State,
    callees: &Callees,
    types: Option<&VbTypes>,
    vtable_offset: u16,
    pushes: bool,
    at: u32,
    calls: &mut Vec<MethodCall>,
) -> Result<Option<Stmt>, LiftFault> {
    let (object, class) = state.object.clone().ok_or(LiftFault::NoObject(at))?;
    let profile = match class.as_deref() {
        None if object == Expr::Arg(8) => Some(callees),
        Some(class) => class
            .strip_prefix(PROJECT_CLASS)
            .and_then(|index| index.parse::<u16>().ok())
            .and_then(|index| callees.class(index)),
        None => None,
    };
    if let Some(profile) = profile {
        return project_call(
            state,
            profile,
            types,
            object,
            vtable_offset,
            pushes,
            at,
            calls,
        );
    }
    let interface = class.ok_or(LiftFault::NoFunction(at))?;
    if pushes {
        return Err(LiftFault::NoFunction(at));
    }
    interface_call(state, types, &interface, object, vtable_offset, at)
}

/// The classes of the arguments of one call of a method of an object of the
/// project: the owner of the profile of the object, the index of the
/// method, and for each argument, first argument first, its bytes on the
/// stack and its interface when the lift knows it.
pub type MethodCall = (Option<u16>, u16, Vec<(u8, Option<String>)>);

/// Lifts a listing to statements. `callees` gives the methods that a
/// `ThisVCallHresult` can call and the control accessors of the form.
/// `types` gives the interfaces of the controls, when the user derived
/// them.
///
/// # Errors
///
/// Gives the first [`LiftFault`].
pub fn lift(
    listing: &PcodeListing,
    table: &PcodeTable,
    callees: &Callees,
    types: Option<&VbTypes>,
) -> Result<Vec<LiftedStmt>, LiftFault> {
    run(listing, table, callees, types, &[], &mut Vec::new())
}

/// Lifts the listing of the method `method` of the object, as [`lift`]
/// does. The arguments of the method that [`Callees`] gives an interface
/// have that interface.
///
/// # Errors
///
/// Gives the first [`LiftFault`].
pub fn lift_method(
    listing: &PcodeListing,
    table: &PcodeTable,
    callees: &Callees,
    types: Option<&VbTypes>,
    method: u16,
) -> Result<Vec<LiftedStmt>, LiftFault> {
    let arguments = callees.arguments_of(method);
    run(listing, table, callees, types, &arguments, &mut Vec::new())
}

/// Gives the calls of methods of `Me` that the lift of the listing of the
/// method `method` meets before its first fault, with the classes of their
/// arguments.
#[must_use]
pub fn method_calls(
    listing: &PcodeListing,
    table: &PcodeTable,
    callees: &Callees,
    types: Option<&VbTypes>,
    method: u16,
) -> Vec<MethodCall> {
    let arguments = callees.arguments_of(method);
    let mut calls = Vec::new();
    let _ = run(listing, table, callees, types, &arguments, &mut calls);
    calls
}

/// The lift of [`lift`], with the interfaces of some argument slots, and
/// each call of a method of `Me` recorded in `calls`.
fn run(
    listing: &PcodeListing,
    table: &PcodeTable,
    callees: &Callees,
    types: Option<&VbTypes>,
    arguments: &[(i16, String)],
    calls: &mut Vec<MethodCall>,
) -> Result<Vec<LiftedStmt>, LiftFault> {
    if !listing.end.is_complete() {
        return Err(LiftFault::NotDecoded(listing.end));
    }
    let mut state = State::default();
    for (slot, interface) in arguments {
        state
            .bindings
            .insert(*slot, (Expr::frame(*slot), Some(interface.clone())));
    }
    let mut out = Vec::new();
    let mut start: Option<u32> = None;
    for (position, instruction) in listing.instructions.iter().enumerate() {
        let next_takes_a_float = listing
            .instructions
            .get(position.saturating_add(1))
            .and_then(|next| table.slot(next.lead, next.opcode))
            .is_some_and(|slot| slot.names.iter().any(|name| takes_a_float(name)));
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
        let word16 = |from: usize| u16_at(arguments, from).ok_or_else(short);
        let pop = |state: &mut State| {
            state
                .stack
                .pop()
                .map(|value| value.expr)
                .ok_or(LiftFault::StackShort(at))
        };
        let stmt = match family {
            Family::Lit(len) => {
                let value = constant(arguments, len).ok_or_else(short)?;
                state.stack.push(Value::plain(Expr::Const(value), true));
                None
            }
            Family::FrameLoad | Family::ArgRef => {
                let value = state.load(offset16()?, is_word(names));
                state.stack.push(value);
                None
            }
            Family::ObjectRegister => {
                let value = state.load(offset16()?, true);
                state.object = Some((value.expr, value.class));
                None
            }
            Family::ObjectRegisterThis => {
                state.object = Some((Expr::Arg(8), None));
                None
            }
            Family::Binary(op) => {
                let right = pop(&mut state)?;
                let left = pop(&mut state)?;
                state.stack.push(Value::plain(
                    Expr::Binary(op, Box::new(left), Box::new(right)),
                    is_word(names),
                ));
                None
            }
            Family::Convert(function) => {
                let value = pop(&mut state)?;
                state.stack.push(match function {
                    Some(function) => Value::plain(
                        Expr::Convert(function, Box::new(value)),
                        WORD_CONVERSIONS.contains(&function),
                    ),
                    None => Value::plain(value, false),
                });
                None
            }
            Family::FrameStore if state.stack.len() > 1 => {
                let value = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                let slot = offset16()?;
                let assign = LiftedStmt {
                    offset: first,
                    stmt: Stmt::Assign {
                        target: Expr::frame(slot),
                        value: value.expr.clone(),
                    },
                };
                state.bind_pending(slot, value, out.len(), assign);
                None
            }
            Family::FrameStore | Family::IndirectStore => {
                let value = pop(&mut state)?;
                let slot = offset16()?;
                state.bindings.remove(&slot);
                Some(Stmt::Assign {
                    target: Expr::frame(slot),
                    value,
                })
            }
            Family::FieldStore => {
                let value = pop(&mut state)?;
                let (base, _) = state.object.clone().ok_or(LiftFault::NoObject(at))?;
                Some(Stmt::Assign {
                    target: Expr::Field(Box::new(base), word16(0)?),
                    value,
                })
            }
            Family::GlobalStore => {
                let value = pop(&mut state)?;
                Some(Stmt::Assign {
                    target: Expr::Global(word16(0)?),
                    value,
                })
            }
            Family::BranchFalse => {
                let condition = pop(&mut state)?;
                let target = offset16()?;
                Some(Stmt::IfNotGoTo {
                    condition,
                    target: target.cast_unsigned(),
                })
            }
            Family::Branch => Some(Stmt::GoTo(offset16()?.cast_unsigned())),
            Family::End => Some(Stmt::End),
            Family::Exit => Some(Stmt::Exit),
            Family::Free => {
                for slot in freed_slots(names, arguments).ok_or_else(short)? {
                    state.pending.remove(&slot);
                    state.bindings.remove(&slot);
                }
                None
            }
            Family::ThisCall => {
                let method = callees.method(word16(0)?).ok_or(LiftFault::NoCallee(at))?;
                let bytes = method
                    .arg_size
                    .checked_sub(4)
                    .ok_or(LiftFault::CallArguments(at))?;
                let args = call_arguments(&mut state.stack, bytes, at)?;
                calls.push((
                    callees.owner,
                    method.index,
                    args.iter()
                        .map(|value| (value.bytes, value.class.clone()))
                        .collect(),
                ));
                Some(Stmt::Call(Callee::Method(method.index), expressions(args)))
            }
            Family::ImportCall { result } => {
                let callee = Callee::Import(word16(0)?);
                let args = expressions(call_arguments(&mut state.stack, word16(2)?, at)?);
                let float = names
                    .iter()
                    .any(|name| name.ends_with("FPR4") || name.ends_with("FPR8"));
                if result {
                    state
                        .stack
                        .push(Value::plain(Expr::Call(callee, args), true));
                    None
                } else if float && next_takes_a_float {
                    state
                        .stack
                        .push(Value::plain(Expr::Call(callee, args), false));
                    None
                } else {
                    Some(Stmt::Call(callee, args))
                }
            }
            Family::ObjectStore => {
                let value = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                let slot = offset16()?;
                if state.stack.is_empty() {
                    let set = LiftedStmt {
                        offset: first,
                        stmt: Stmt::Set {
                            target: Expr::frame(slot),
                            value: value.expr.clone(),
                        },
                    };
                    state.bind_pending(slot, value, out.len(), set);
                } else {
                    state.bindings.insert(slot, (value.expr, value.class));
                }
                None
            }
            Family::ObjectCall { pushes } => {
                object_call(&mut state, callees, types, word16(0)?, pushes, at, calls)?
            }
            Family::GlobalLoad => {
                state
                    .stack
                    .push(Value::plain(Expr::Global(word16(0)?), is_word(names)));
                None
            }
            Family::GlobalObjectRegister => {
                state.object = Some((Expr::Global(word16(0)?), None));
                None
            }
            Family::FieldLoad | Family::FieldObjectRegister => {
                let (base, _) = state.object.clone().ok_or(LiftFault::NoObject(at))?;
                let field = Expr::Field(Box::new(base), word16(0)?);
                if family == Family::FieldLoad {
                    state.stack.push(Value::plain(field, is_word(names)));
                } else {
                    state.object = Some((field, None));
                }
                None
            }
            Family::FrameFieldLoad | Family::FrameFieldObjectRegister => {
                let base = state.load(offset16()?, true).expr;
                let field = Expr::Field(Box::new(base), word16(2)?);
                if family == Family::FrameFieldLoad {
                    state.stack.push(Value::plain(field, is_word(names)));
                } else {
                    state.object = Some((field, None));
                }
                None
            }
            Family::FrameFieldStore => {
                let value = pop(&mut state)?;
                let base = state.load(offset16()?, true).expr;
                Some(Stmt::Assign {
                    target: Expr::Field(Box::new(base), word16(2)?),
                    value,
                })
            }
            Family::StoreKeep { object } => {
                let value = state
                    .stack
                    .last()
                    .cloned()
                    .ok_or(LiftFault::StackShort(at))?;
                let slot = offset16()?;
                let target = Expr::frame(slot);
                let stmt = if object {
                    Stmt::Set {
                        target,
                        value: value.expr.clone(),
                    }
                } else {
                    Stmt::Assign {
                        target,
                        value: value.expr.clone(),
                    }
                };
                let stmt = LiftedStmt {
                    offset: first,
                    stmt,
                };
                state.bind_pending(slot, value, out.len(), stmt);
                None
            }
            Family::PopTemp => {
                let value = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                state.stack.push(Value {
                    bytes: 4,
                    slot: Some(offset16()?),
                    ..value
                });
                None
            }
            Family::LitString => {
                state
                    .stack
                    .push(Value::plain(callees.string(word16(0)?), true));
                None
            }
            Family::LitVariant(kind) => {
                let slot = offset16()?;
                let rest = arguments.get(2..).unwrap_or_default();
                let expr = match kind {
                    VariantKind::Word("True") => Expr::Const(-1),
                    VariantKind::Word("False") => Expr::Const(0),
                    VariantKind::Word(word) => Expr::Word(word),
                    VariantKind::Byte => Expr::Const(i64::from(*rest.first().ok_or_else(short)?)),
                    VariantKind::Int(len) => Expr::Const(constant(rest, len).ok_or_else(short)?),
                    VariantKind::Single => {
                        let bytes: [u8; 4] = rest
                            .get(..4)
                            .and_then(|bytes| bytes.try_into().ok())
                            .ok_or_else(short)?;
                        Expr::Real(f64::from(f32::from_le_bytes(bytes)).to_bits())
                    }
                    VariantKind::Str => callees.string(u16_at(rest, 0).ok_or_else(short)?),
                };
                state.stack.push(Value {
                    expr,
                    bytes: 4,
                    slot: Some(slot),
                    class: None,
                });
                None
            }
            Family::LitReal => {
                let bytes: [u8; 8] = arguments
                    .get(..8)
                    .and_then(|bytes| bytes.try_into().ok())
                    .ok_or_else(short)?;
                state
                    .stack
                    .push(Value::plain(Expr::Real(u64::from_le_bytes(bytes)), false));
                None
            }
            Family::NewObject => {
                let index = word16(0)?;
                let mut value = Value::plain(Expr::New(index), true);
                value.class = if callees.class(index).is_some() {
                    Some(format!("{PROJECT_CLASS}{index}"))
                } else {
                    callees.class_interface(index).map(str::to_owned)
                };
                state.stack.push(value);
                None
            }
            Family::NewIfNull => {
                let reference = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                let index = word16(0)?;
                let class = if callees.class(index).is_some() {
                    Some(format!("{PROJECT_CLASS}{index}"))
                } else if let Some(interface) = callees.class_interface(index) {
                    Some(interface.to_owned())
                } else {
                    reference.class
                };
                state.object = Some((reference.expr, class));
                None
            }
            Family::Nop => None,
            Family::FunctionOf(function, count) => {
                let mut args = Vec::new();
                for _ in 0..count {
                    args.push(pop(&mut state)?);
                }
                args.reverse();
                state.stack.push(Value::plain(
                    Expr::Call(
                        Callee::Member(Box::new(Expr::Word("VBA")), function.to_owned()),
                        args,
                    ),
                    true,
                ));
                None
            }
            Family::FloatFunction(function) => {
                let value = pop(&mut state)?;
                state.stack.push(Value::plain(
                    Expr::Convert(function, Box::new(value)),
                    false,
                ));
                None
            }
            Family::Redim => {
                let array = pop(&mut state)?;
                let mut bounds = Vec::new();
                for _ in 0..word16(0)?.saturating_mul(2) {
                    bounds.push(pop(&mut state)?);
                }
                bounds.reverse();
                let ranges = bounds
                    .chunks(2)
                    .map(|pair| match pair {
                        [lower, upper] => Expr::Binary(
                            BinaryOp::To,
                            Box::new(lower.clone()),
                            Box::new(upper.clone()),
                        ),
                        _ => Expr::Word("?"),
                    })
                    .collect();
                Some(Stmt::Call(
                    Callee::Member(Box::new(Expr::Word("VBA")), "ReDim".to_owned()),
                    vec![Expr::Index(Box::new(array), ranges)],
                ))
            }
            Family::ArrayReference {
                dimensions_argument,
            } => {
                let array = pop(&mut state)?;
                let count = if dimensions_argument { word16(0)? } else { 1 };
                let mut indexes = Vec::new();
                for _ in 0..count {
                    indexes.push(pop(&mut state)?);
                }
                indexes.reverse();
                state
                    .stack
                    .push(Value::plain(Expr::Index(Box::new(array), indexes), true));
                None
            }
            Family::LitSingle => {
                let bytes: [u8; 4] = arguments
                    .get(..4)
                    .and_then(|bytes| bytes.try_into().ok())
                    .ok_or_else(short)?;
                state.stack.push(Value::plain(
                    Expr::Real(f64::from(f32::from_le_bytes(bytes)).to_bits()),
                    false,
                ));
                None
            }
            Family::ArrayErase => Some(Stmt::Call(
                Callee::Member(Box::new(Expr::Word("VBA")), "Erase".to_owned()),
                vec![pop(&mut state)?],
            )),
            Family::CopyBytes => {
                let target = pop(&mut state)?;
                let value = pop(&mut state)?;
                Some(Stmt::Assign { target, value })
            }
            Family::ArrayElementRegister => {
                let array = pop(&mut state)?;
                let index = pop(&mut state)?;
                state.object = Some((Expr::Index(Box::new(array), vec![index]), None));
                None
            }
            Family::NewIfNullPush => {
                let reference = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                let index = word16(0)?;
                let class = if callees.class(index).is_some() {
                    Some(format!("{PROJECT_CLASS}{index}"))
                } else if let Some(interface) = callees.class_interface(index) {
                    Some(interface.to_owned())
                } else {
                    reference.class
                };
                state.stack.push(Value {
                    expr: reference.expr,
                    bytes: 4,
                    slot: None,
                    class,
                });
                None
            }
            Family::LateGet => {
                let (object, _) = state.object.clone().ok_or(LiftFault::NoObject(at))?;
                let slot = offset16()?;
                let dispid = arguments
                    .get(2..6)
                    .and_then(|bytes| bytes.try_into().ok())
                    .map(u32::from_le_bytes)
                    .ok_or_else(short)?;
                let expr = Expr::Late(Box::new(object), dispid);
                state.bindings.insert(slot, (expr.clone(), None));
                state.stack.push(Value {
                    expr,
                    bytes: 4,
                    slot: Some(slot),
                    class: None,
                });
                None
            }
            Family::Discard => {
                let value = pop(&mut state)?;
                match value.unbound() {
                    Expr::Call(callee, args) => Some(Stmt::Call(callee, args)),
                    _ => None,
                }
            }
            Family::ArrayLoad => {
                let array = pop(&mut state)?;
                let index = pop(&mut state)?;
                state.stack.push(Value::plain(
                    Expr::Index(Box::new(array), vec![index]),
                    is_word(names),
                ));
                None
            }
            Family::ArrayStore => {
                let array = pop(&mut state)?;
                let index = pop(&mut state)?;
                let value = pop(&mut state)?;
                Some(Stmt::Assign {
                    target: Expr::Index(Box::new(array), vec![index]),
                    value,
                })
            }
            Family::ArrayObjectRegister => {
                let array = pop(&mut state)?;
                let mut indexes = Vec::new();
                for _ in 0..word16(0)? {
                    indexes.push(pop(&mut state)?);
                }
                indexes.reverse();
                state.object = Some((Expr::Index(Box::new(array), indexes), None));
                None
            }
            Family::Function(function) => {
                let value = pop(&mut state)?;
                state
                    .stack
                    .push(Value::plain(Expr::Convert(function, Box::new(value)), true));
                None
            }
            Family::Sized(bytes) => {
                let value = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                state.stack.push(Value {
                    bytes,
                    slot: None,
                    ..value
                });
                None
            }
            Family::StringCopy => {
                let target = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                let value = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                let slot = target.slot.ok_or(LiftFault::NoResultSlot(at))?;
                state.bindings.insert(slot, (value.expr, None));
                None
            }
            Family::For { step } => {
                let step = if step { Some(pop(&mut state)?) } else { None };
                let end = pop(&mut state)?;
                let counter = pop(&mut state)?;
                let start = pop(&mut state)?;
                Some(Stmt::For {
                    counter,
                    start,
                    end,
                    step,
                    exit: word16(2)?,
                })
            }
            Family::Next => Some(Stmt::Next {
                counter: pop(&mut state)?,
                body: word16(2)?,
            }),
            Family::OnError => Some(Stmt::OnError(match word16(0)? {
                0xFFFF => None,
                0xFFFE => Some(0),
                target => Some(target),
            })),
        };
        if let Some(stmt) = stmt {
            if !state.stack.is_empty() {
                return Err(LiftFault::StackLeft(at));
            }
            out.push(LiftedStmt {
                offset: first,
                stmt,
            });
            start = None;
        }
    }
    let mut pending: Vec<PendingSet> = state.pending.into_values().collect();
    pending.extend(state.kept);
    let kept: BTreeSet<u32> = pending.iter().map(|(store, _, _)| *store).collect();
    pending.sort_by_key(|(store, index, _)| std::cmp::Reverse((*index, *store)));
    for (_, index, set) in pending {
        out.insert(index.min(out.len()), set);
    }
    Ok(out
        .into_iter()
        .map(|lifted| LiftedStmt {
            offset: lifted.offset,
            stmt: lifted.stmt.resolve(&kept),
        })
        .collect())
}

/// Gives the index of each class of the constant table that `listing`
/// names with `NewIfNullPr` or `New`. A caller gives [`Callees`] the profile of each
/// one that is an object of the project.
#[must_use]
pub fn class_indexes(listing: &PcodeListing, table: &PcodeTable) -> Vec<u16> {
    let mut out = Vec::new();
    for instruction in &listing.instructions {
        let names = table
            .slot(instruction.lead, instruction.opcode)
            .map(|slot| slot.names.as_slice())
            .unwrap_or_default();
        if matches!(family(names), Some(Family::NewIfNull | Family::NewObject))
            && let Some(index) = u16_at(&instruction.arguments, 0)
            && !out.contains(&index)
        {
            out.push(index);
        }
    }
    out
}

/// Gives the index of each string of the constant table that `listing`
/// names: the argument of each `LitStr`, and the second argument of each
/// `LitVarStr`. A caller reads those strings with
/// `vb::constants::constant_string` and gives them to [`Callees`].
#[must_use]
pub fn string_indexes(listing: &PcodeListing, table: &PcodeTable) -> Vec<u16> {
    let mut out = Vec::new();
    for instruction in &listing.instructions {
        let names = table
            .slot(instruction.lead, instruction.opcode)
            .map(|slot| slot.names.as_slice())
            .unwrap_or_default();
        let at = match family(names) {
            Some(Family::LitString) => 0,
            Some(Family::LitVariant(VariantKind::Str)) => 2,
            _ => continue,
        };
        if let Some(index) = u16_at(&instruction.arguments, at)
            && !out.contains(&index)
        {
            out.push(index);
        }
    }
    out
}

/// Renders statements as lines, with a label before each statement that a
/// branch names. A label whose offset starts no statement is given on a
/// line of its own at the end.
#[must_use]
pub fn render(stmts: &[LiftedStmt]) -> Vec<String> {
    let mut targets: Vec<u16> = stmts
        .iter()
        .filter_map(|lifted| match lifted.stmt {
            Stmt::IfNotGoTo { target, .. }
            | Stmt::GoTo(target)
            | Stmt::For { exit: target, .. }
            | Stmt::Next { body: target, .. } => Some(target),
            Stmt::OnError(target) => target.filter(|target| *target != 0),
            Stmt::Assign { .. } | Stmt::Set { .. } | Stmt::End | Stmt::Exit | Stmt::Call(..) => {
                None
            }
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    let mut lines = Vec::new();
    let mut labelled = Vec::new();
    for lifted in stmts {
        let label = u16::try_from(lifted.offset)
            .ok()
            .filter(|offset| targets.contains(offset) && !labelled.contains(offset));
        labelled.extend(label);
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
    use super::{
        Callees, LiftFault, class_indexes, lift, lift_method, method_calls, render, string_indexes,
    };
    use crate::read::region::{Off, Region};
    use crate::vb::pcode::{PcodeTable, disassemble};
    use crate::vb::types::VbTypes;

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
[primary.11]
width = 4
names = ["ThisVCallHresult"]
[primary.12]
width = 4
names = ["ImpAdCall", "ImpAdCallFPR4", "ImpAdCallFPR8"]
[primary.13]
width = 4
names = ["ImpAdCallAd", "ImpAdCallI2", "ImpAdCallI4", "ImpAdCallStr"]
[primary.14]
width = 2
names = ["FLdFPR8"]
[primary.15]
width = 2
names = ["FLdRf", "FLdRfVar"]
[primary.16]
width = 0
names = ["FLdPrThis"]
[primary.17]
width = 2
names = ["ILdPr"]
[primary.18]
width = 2
names = ["FFree1Ad"]
[primary.19]
width = "counted"
names = ["FFreeStr"]
[primary.1A]
width = 2
names = ["VCallAd", "VCallI2", "VCallI4", "VCallStr"]
[primary.1B]
width = 2
names = ["FStAdFunc"]
[primary.1C]
width = 4
names = ["VCallHresult"]
[primary.1D]
width = 2
names = ["ImpAdLdRf", "ImpAdLdRfVar"]
[primary.1E]
width = 2
names = ["MemLdAd", "MemLdI4", "MemLdR4", "MemLdStr"]
[primary.1F]
width = 4
names = ["FMemStI4", "FMemStR4"]
[primary.20]
width = 4
names = ["FMemLdI2"]
[primary.21]
width = 2
names = ["LitStr"]
[primary.22]
width = 2
names = ["LitVar_Missing"]
[primary.23]
width = 4
names = ["LitVarI2"]
[primary.24]
width = 8
names = ["LitDate", "LitR8FP"]
[primary.25]
width = 2
names = ["FStStrNoPop"]
[primary.26]
width = 2
names = ["PopTmpLdAd2"]
[primary.27]
width = 2
names = ["New"]
[primary.28]
width = 2
names = ["NewIfNullPr"]
[primary.29]
width = 1
names = ["Bos", "LargeBos"]
[primary.2A]
width = 2
names = ["FStStrCopy"]
[primary.2B]
width = 2
names = ["FStR8"]
[primary.2C]
width = 0
names = ["ConcatStr"]
[primary.2D]
width = 0
names = ["NotI2", "NotI4"]
[primary.2E]
width = 0
names = ["PopAdLdVar"]
[primary.2F]
width = 0
names = ["CStr2Ansi"]
[primary.30]
width = 4
names = ["ForI4"]
[primary.31]
width = 4
names = ["NextI4"]
[primary.32]
width = 2
names = ["OnErrorGoto"]
[primary.33]
width = 6
names = ["LateIdLdVar"]
[primary.34]
width = 0
names = ["PopAd"]
[primary.35]
width = 0
names = ["Ary1LdAd", "Ary1LdI4", "Ary1LdR4", "Ary1LdStr"]
[primary.36]
width = 0
names = ["Ary1StI4", "Ary1StR4"]
[primary.37]
width = 0
names = ["FnInStr4"]
[primary.38]
width = 0
names = ["Erase"]
[primary.39]
width = 2
names = ["CopyBytes"]
[primary.3A]
width = 0
names = ["FnAbsI2", "FnAbsI4"]
[primary.3B]
width = 8
names = ["Redim"]
[primary.3C]
width = 0
names = ["MulR4", "MulR8"]
[lead1.C8]
width = 0
names = ["End"]
"#;

    fn lines(body: &[u8]) -> Result<Vec<String>, LiftFault> {
        lines_with(body, &Callees::default())
    }

    fn lines_with(body: &[u8], callees: &Callees) -> Result<Vec<String>, LiftFault> {
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(body, Off::new(0)), &table);
        lift(&listing, &table, callees, None).map(|stmts| render(&stmts))
    }

    /// Interfaces built here, in the form that `derive-vb-types` writes. The
    /// interface `_Box` and its offsets are placeholders.
    const TYPES: &str = r#"
[interfaces._Box]
vtable_size = 256
[interfaces._Box.functions.00A8]
names = ["_Default", "Text"]
kinds = ["get", "get"]
arg_bytes = 4
result = true
[interfaces._Box.functions.00AC]
names = ["_Default", "Text"]
kinds = ["let", "let"]
arg_bytes = 4
result = false
[interfaces._Box.functions.00B4]
names = ["Container"]
kinds = ["get"]
arg_bytes = 4
result = true
result_interface = "_Box"
[interfaces._Box.functions.00B0]
names = ["Cls"]
kinds = ["method"]
arg_bytes = 0
result = false
"#;

    /// Lifts `body` with the control `box1` at the accessor offset `0x32C`
    /// of `Me`, of the interface `_Box`.
    fn object_lines(body: &[u8]) -> Result<Vec<String>, LiftFault> {
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let types = VbTypes::parse(TYPES.as_bytes()).unwrap();
        let callees = Callees::default().with_control(0x32C, "box1", "_Box");
        let listing = disassemble(&Region::new(body, Off::new(0)), &table);
        lift(&listing, &table, &callees, Some(&types)).map(|stmts| render(&stmts))
    }

    /// `FLdPrThis`, then `VCallAd` of the accessor `0x32C`, then `FStAdFunc`
    /// into `local_68`, then `FLdPr` of `local_68`.
    const BOX1_IN_LOCAL_68: [u8; 9] = [0x16, 0x1A, 0x2C, 0x03, 0x1B, 0x98, 0xFF, 0x06, 0x98];

    #[test]
    fn a_property_get_of_a_control_binds_its_result_slot() {
        // local_88 = box1.Text: push the address of local_64 for the result,
        // get the control into a temporary slot, call the get, then load
        // local_64 and store it.
        let mut body = vec![0x15, 0x9C, 0xFF];
        body.extend_from_slice(&BOX1_IN_LOCAL_68);
        body.extend_from_slice(&[
            0xFF, 0x1C, 0xA8, 0x00, 0x00, 0x00, 0x03, 0x9C, 0xFF, 0x05, 0x78, 0xFF, 0x18, 0x98,
            0xFF, 0x0C,
        ]);
        assert_eq!(
            object_lines(&body).unwrap(),
            ["       local_88 = Me.box1.Text", "       Exit"]
        );
    }

    #[test]
    fn a_let_and_a_method_of_a_control_give_statements_and_a_freed_slot_no_set() {
        let mut body = vec![0x02, 0x05];
        body.extend_from_slice(&BOX1_IN_LOCAL_68);
        body.extend_from_slice(&[0xFF, 0x1C, 0xAC, 0x00, 0x00, 0x00, 0x18, 0x98, 0xFF]);
        body.extend_from_slice(&BOX1_IN_LOCAL_68);
        body.extend_from_slice(&[0xFF, 0x1C, 0xB0, 0x00, 0x00, 0x00, 0x18, 0x98, 0xFF, 0x0C]);
        assert_eq!(
            object_lines(&body).unwrap(),
            [
                "       Me.box1.Text = 5",
                "       Call Me.box1.Cls()",
                "       Exit"
            ]
        );
    }

    #[test]
    fn a_control_array_gives_its_control_by_item() {
        // local_88 = Me.box1.Item(0).Text: the accessor gives the object of
        // the array into local_68, whose Item writes the control into
        // local_64, whose Text writes local_60.
        let body = [
            0x15, 0xA0, 0xFF, 0x15, 0x9C, 0xFF, 0x02, 0x00, 0x16, 0x1A, 0x2C, 0x03, 0x1B, 0x98,
            0xFF, 0x06, 0x98, 0xFF, 0x1C, 0x40, 0x00, 0x00, 0x00, 0x06, 0x9C, 0xFF, 0x1C, 0xA8,
            0x00, 0x00, 0x00, 0x03, 0xA0, 0xFF, 0x05, 0x78, 0xFF, 0x18, 0x98, 0xFF, 0x18, 0x9C,
            0xFF, 0x0C,
        ];
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let types = VbTypes::parse(TYPES.as_bytes()).unwrap();
        let callees = Callees::default().with_control(0x32C, "box1", "[]_Box");
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        assert_eq!(
            render(&lift(&listing, &table, &callees, Some(&types)).unwrap()),
            ["       local_88 = Me.box1.Item(0).Text", "       Exit"]
        );
    }

    #[test]
    fn a_second_store_to_a_slot_keeps_the_first_set() {
        let mut body = BOX1_IN_LOCAL_68.to_vec();
        body.push(0xFF);
        body.extend_from_slice(&BOX1_IN_LOCAL_68);
        body.extend_from_slice(&[0xFF, 0x1C, 0xB0, 0x00, 0x00, 0x00, 0x0C]);
        assert_eq!(
            object_lines(&body).unwrap(),
            [
                "       Set local_68 = Me.box1",
                "       Set local_68 = Me.box1",
                "       Call local_68.Cls()",
                "       Exit"
            ]
        );
    }

    #[test]
    fn an_object_slot_that_no_free_frees_gives_a_set() {
        let mut body = BOX1_IN_LOCAL_68.to_vec();
        body.extend_from_slice(&[0xFF, 0x1C, 0xB0, 0x00, 0x00, 0x00, 0x0C]);
        assert_eq!(
            object_lines(&body).unwrap(),
            [
                "       Set local_68 = Me.box1",
                "       Call local_68.Cls()",
                "       Exit"
            ]
        );
    }

    #[test]
    fn the_result_of_a_get_keeps_its_interface() {
        // Me.box1.Container.Cls: the get of Container writes local_64, whose
        // interface is _Box, and FLdPr of local_64 calls Cls on it.
        let mut body = vec![0x15, 0x9C, 0xFF];
        body.extend_from_slice(&BOX1_IN_LOCAL_68);
        body.extend_from_slice(&[
            0xFF, 0x1C, 0xB4, 0x00, 0x00, 0x00, 0x06, 0x9C, 0xFF, 0x1C, 0xB0, 0x00, 0x00, 0x00,
            0x0C,
        ]);
        assert_eq!(
            object_lines(&body).unwrap()[0],
            "       Call Me.box1.Container.Cls()"
        );
    }

    #[test]
    fn an_argument_of_a_known_class_is_an_object_of_that_class() {
        // ILdPr of arg_C, then Cls: with the interface _Box for arg_C of
        // method 3, the call finds Cls.
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let types = VbTypes::parse(TYPES.as_bytes()).unwrap();
        let body = [0x17, 0x0C, 0x00, 0x1C, 0xB0, 0x00, 0x00, 0x00, 0x0C];
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        let callees = Callees::default().with_argument(3, 0x0C, "_Box");
        assert_eq!(
            render(&lift_method(&listing, &table, &callees, Some(&types), 3).unwrap())[0],
            "       Call arg_C.Cls()"
        );
        assert_eq!(
            lift_method(&listing, &table, &callees, Some(&types), 4),
            Err(LiftFault::NoFunction(3))
        );
    }

    #[test]
    fn a_call_of_a_method_of_me_records_the_classes_of_its_arguments() {
        // Call Me.method_5(Me.box1, 1): the control is passed by the address
        // of its temporary slot.
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let types = VbTypes::parse(TYPES.as_bytes()).unwrap();
        let body = [
            0x02, 0x01, 0x16, 0x1A, 0x2C, 0x03, 0x1B, 0x98, 0xFF, 0x15, 0x98, 0xFF, 0x11, 0xF8,
            0x06, 0x00, 0x00, 0x0C,
        ];
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        let callees = Callees::default()
            .with_control(0x32C, "box1", "_Box")
            .with_method(0x6F8, 5, 12);
        assert_eq!(
            method_calls(&listing, &table, &callees, Some(&types), 0),
            [(None, 5, vec![(4, Some("_Box".to_owned())), (4, None)])]
        );
    }

    #[test]
    fn a_call_on_me_below_the_accessors_is_a_call_of_the_base_interface() {
        // FLdPr of the slot of Me, then the let of Text at 0xAC, with the
        // base interface _Box.
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let types = VbTypes::parse(TYPES.as_bytes()).unwrap();
        let body = [
            0x02, 0x07, 0x06, 0x08, 0x00, 0x1C, 0xAC, 0x00, 0x00, 0x00, 0x0C,
        ];
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        let callees = Callees::default().with_base("_Box");
        assert_eq!(
            render(&lift(&listing, &table, &callees, Some(&types)).unwrap())[0],
            "       Me.Text = 7"
        );
        assert_eq!(
            lift(&listing, &table, &Callees::default(), Some(&types)),
            Err(LiftFault::NoFunction(5))
        );
    }

    #[test]
    fn a_call_of_an_object_that_the_lift_cannot_type_gives_its_fault() {
        // FLdPr of local_88, which holds no known object, then a call.
        let unknown = [0x06, 0x78, 0xFF, 0x1C, 0xB0, 0x00, 0x00, 0x00, 0x0C];
        assert_eq!(object_lines(&unknown), Err(LiftFault::NoFunction(3)));
        // The accessor of an offset that no control has.
        let accessor = [0x16, 0x1A, 0x30, 0x03, 0x0C];
        assert_eq!(object_lines(&accessor), Err(LiftFault::NoFunction(1)));
        // A get whose result pointer is a constant.
        let mut constant = vec![0x02, 0x00];
        constant.extend_from_slice(&BOX1_IN_LOCAL_68);
        constant.extend_from_slice(&[0xFF, 0x1C, 0xA8, 0x00, 0x00, 0x00, 0x0C]);
        assert_eq!(object_lines(&constant), Err(LiftFault::NoResultSlot(12)));
    }

    #[test]
    fn a_method_call_pops_its_argument_size_less_the_four_bytes_of_me() {
        // Push the address of local_88, then arg_C, then call the method at
        // 0x6F8, whose descriptor gives 12 bytes: Me and two arguments.
        let body = [
            0x15, 0x78, 0xFF, 0x03, 0x0C, 0x00, 0x11, 0xF8, 0x06, 0x00, 0x00, 0x0C,
        ];
        let callees = Callees::default().with_method(0x6F8, 5, 12);
        assert_eq!(
            lines_with(&body, &callees).unwrap(),
            ["       Call Me.method_5(arg_C, local_88)", "       Exit"]
        );
        let fewer = Callees::default().with_method(0x6F8, 5, 8);
        assert_eq!(lines_with(&body, &fewer), Err(LiftFault::StackLeft(6)));
    }

    #[test]
    fn an_import_call_pops_the_bytes_of_its_second_argument() {
        // import_3(1) gives a value, and import_2 takes no argument.
        let body = [
            0x01, 1, 0, 0, 0, 0x13, 0x03, 0x00, 0x04, 0x00, 0x05, 0x78, 0xFF, 0x12, 0x02, 0x00,
            0x00, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap(),
            [
                "       local_88 = import_3(1)",
                "       Call import_2()",
                "       Exit"
            ]
        );
    }

    #[test]
    fn a_call_that_the_lift_cannot_count_gives_its_fault() {
        let method = [0x11, 0xF8, 0x06, 0x00, 0x00, 0x0C];
        assert_eq!(lines(&method), Err(LiftFault::NoCallee(0)));
        let odd = [0x12, 0x02, 0x00, 0x02, 0x00, 0x0C];
        assert_eq!(lines(&odd), Err(LiftFault::CallArguments(0)));
        let wide = [0x14, 0x78, 0xFF, 0x12, 0x02, 0x00, 0x04, 0x00, 0x0C];
        assert_eq!(lines(&wide), Err(LiftFault::CallArguments(3)));
        let empty = [0x12, 0x02, 0x00, 0x04, 0x00, 0x0C];
        assert_eq!(lines(&empty), Err(LiftFault::StackShort(0)));
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
    fn the_object_register_is_me_or_a_frame_slot() {
        // `FLdPrThis`, then a store to field 0x54; `ILdPr` of arg_C, then a
        // store to field 0x40.
        let body = [
            0x16, 0x02, 0x00, 0x07, 0x54, 0x00, 0x17, 0x0C, 0x00, 0x02, 0x01, 0x07, 0x40, 0x00,
            0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap(),
            [
                "       Me.field_54 = 0",
                "       arg_C.field_40 = 1",
                "       Exit"
            ]
        );
    }

    #[test]
    fn a_free_of_temporary_slots_changes_no_stack_and_gives_no_statement() {
        // Push 1, free one slot and two slots, store the 1.
        let body = [
            0x02, 0x01, 0x18, 0x78, 0xFF, 0x19, 0x04, 0x00, 0x70, 0xFF, 0x6C, 0xFF, 0x05, 0x78,
            0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap(),
            ["       local_88 = 1", "       Exit"]
        );
    }

    #[test]
    fn globals_fields_and_strings_give_their_expressions() {
        // local_88 = Me.field_54; local_68.field_10 = local_88.field_C;
        // local_88 = "x", with the string at index 2.
        let body = [
            0x16, 0x1E, 0x54, 0x00, 0x05, 0x78, 0xFF, 0x20, 0x78, 0xFF, 0x0C, 0x00, 0x1F, 0x98,
            0xFF, 0x10, 0x00, 0x21, 0x02, 0x00, 0x2A, 0x78, 0xFF, 0x0C,
        ];
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        let callees = Callees::default().with_string(2, "x");
        let lines = render(&lift(&listing, &table, &callees, None).unwrap());
        assert_eq!(
            lines,
            [
                "       local_88 = Me.field_54",
                "       local_68.field_10 = local_88.field_C",
                "       local_88 = \"x\"",
                "       Exit"
            ]
        );
    }

    #[test]
    fn literals_give_their_constants() {
        // local_A0 = 2.5, then Call import_1(Missing, 7, New class_4), with a
        // Bos before it.
        let mut body = vec![0x24];
        body.extend_from_slice(&2.5_f64.to_le_bytes());
        body.extend_from_slice(&[0x2B, 0x60, 0xFF, 0x29, 0x00, 0x27, 0x04, 0x00]);
        body.extend_from_slice(&[
            0x23, 0x70, 0xFF, 0x07, 0x00, 0x22, 0x68, 0xFF, 0x12, 0x01, 0x00, 0x0C, 0x00, 0x0C,
        ]);
        assert_eq!(
            lines(&body).unwrap(),
            [
                "       local_A0 = 2.5",
                "       Call import_1(Missing, 7, New class_4)",
                "       Exit"
            ]
        );
        assert_eq!(
            lines(&[0x21, 0x05, 0x00, 0x2A, 0x78, 0xFF, 0x0C]).unwrap()[0],
            "       local_88 = const_5"
        );
    }

    #[test]
    fn a_store_that_keeps_its_value_gives_an_assignment_unless_freed() {
        // import_2(local_88 = arg_C): the store keeps arg_C, and PopTmpLdAd2
        // passes it by reference.
        let kept = [
            0x03, 0x0C, 0x00, 0x25, 0x78, 0xFF, 0x26, 0x70, 0xFF, 0x12, 0x02, 0x00, 0x04, 0x00,
            0x0C,
        ];
        assert_eq!(
            lines(&kept).unwrap(),
            [
                "       local_88 = arg_C",
                "       Call import_2(arg_C)",
                "       Exit"
            ]
        );
        let freed = [
            0x03, 0x0C, 0x00, 0x25, 0x78, 0xFF, 0x12, 0x02, 0x00, 0x04, 0x00, 0x18, 0x78, 0xFF,
            0x0C,
        ];
        assert_eq!(
            lines(&freed).unwrap(),
            ["       Call import_2(arg_C)", "       Exit"]
        );
    }

    #[test]
    fn a_value_passed_through_a_temporary_slot_is_an_argument_of_4_bytes() {
        // A float conversion gives a value that the lift does not size;
        // PopTmpLdAd2 passes it by the address of a temporary slot.
        let body = [
            0x03, 0x0C, 0x00, 0x0D, 0x26, 0x70, 0xFF, 0x12, 0x02, 0x00, 0x04, 0x00, 0x0C,
        ];
        assert_eq!(lines(&body).unwrap()[0], "       Call import_2(arg_C)");
        assert_eq!(
            lines(&[0x03, 0x0C, 0x00, 0x0D, 0x12, 0x02, 0x00, 0x04, 0x00, 0x0C]),
            Err(LiftFault::CallArguments(4))
        );
    }

    #[test]
    fn new_if_null_sets_the_object_register_to_the_variable() {
        // NewIfNullPr of global_3, then a field store through the register.
        let body = [
            0x1D, 0x03, 0x00, 0x28, 0x01, 0x00, 0x02, 0x01, 0x07, 0x40, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap(),
            ["       global_3.field_40 = 1", "       Exit"]
        );
    }

    #[test]
    fn a_concatenation_and_a_function_give_their_expressions() {
        // local_88 = Not (arg_C & arg_10)
        let body = [
            0x03, 0x0C, 0x00, 0x03, 0x10, 0x00, 0x2C, 0x2D, 0x05, 0x78, 0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap()[0],
            "       local_88 = Not((arg_C & arg_10))"
        );
    }

    #[test]
    fn a_variant_by_value_is_16_bytes_and_an_ansi_copy_binds_its_slot() {
        // Call import_1 with the Variant at local_70, which is 16 bytes.
        let variant = [0x15, 0x90, 0xFF, 0x2E, 0x12, 0x01, 0x00, 0x10, 0x00, 0x0C];
        assert_eq!(
            lines(&variant).unwrap()[0],
            "       Call import_1(local_70)"
        );
        let short = [0x15, 0x90, 0xFF, 0x2E, 0x12, 0x01, 0x00, 0x04, 0x00, 0x0C];
        assert_eq!(lines(&short), Err(LiftFault::CallArguments(4)));
        // Copy arg_C into local_64 as ANSI, then pass local_64.
        let ansi = [
            0x03, 0x0C, 0x00, 0x15, 0x9C, 0xFF, 0x2F, 0x03, 0x9C, 0xFF, 0x12, 0x02, 0x00, 0x04,
            0x00, 0x0C,
        ];
        assert_eq!(lines(&ansi).unwrap()[0], "       Call import_2(arg_C)");
    }

    #[test]
    fn a_for_loop_and_on_error_give_statements_and_labels() {
        // On Error Resume Next; For local_88 = 1 To 3: local_90 = 0: Next;
        // On Error GoTo 0; On Error GoTo the exit.
        let body = [
            0x32, 0xFF, 0xFF, 0x02, 0x01, 0x15, 0x78, 0xFF, 0x02, 0x03, 0x30, 0x60, 0xFF, 0x1C,
            0x00, 0x02, 0x00, 0x05, 0x70, 0xFF, 0x15, 0x78, 0xFF, 0x31, 0x60, 0xFF, 0x0F, 0x00,
            0x32, 0xFE, 0xFF, 0x32, 0x22, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap(),
            [
                "       On Error Resume Next",
                "       For local_88 = 1 To 3  ' past the end: GoTo L001C",
                "L000F: local_90 = 0",
                "       Next local_88  ' loop: GoTo L000F",
                "L001C: On Error GoTo 0",
                "       On Error GoTo L0022",
                "L0022: Exit",
            ]
        );
    }

    #[test]
    fn the_string_indexes_are_the_arguments_of_lit_str() {
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let body = [0x21, 0x05, 0x00, 0x21, 0x02, 0x00, 0x21, 0x05, 0x00, 0x0C];
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        assert_eq!(string_indexes(&listing, &table), [5, 2]);
    }

    #[test]
    fn a_call_on_an_object_of_the_project_uses_the_profile_of_its_class() {
        // NewIfNullPr of global_3, whose class at index 9 of the constant
        // table has the control box1 at 0x32C, the method 2 at 0x6F8 with
        // one argument, and the get and the let of field 0x34 at 0x700 and
        // 0x704.
        let profile = Callees::default()
            .with_control(0x32C, "box1", "_Box")
            .with_method(0x6F8, 2, 8)
            .with_variable(0x700, 0x34, true)
            .with_variable(0x704, 0x34, false);
        let callees = Callees::default().with_class(9, profile);
        let object = [0x1D, 0x03, 0x00, 0x28, 0x09, 0x00];
        let mut body = vec![0x02, 0x05];
        body.extend_from_slice(&object);
        body.extend_from_slice(&[0x1C, 0x04, 0x07, 0x00, 0x00]);
        body.extend_from_slice(&[0x15, 0x9C, 0xFF]);
        body.extend_from_slice(&object);
        body.extend_from_slice(&[0x1C, 0x00, 0x07, 0x00, 0x00, 0x03, 0x9C, 0xFF]);
        body.extend_from_slice(&object);
        body.extend_from_slice(&[0x1C, 0xF8, 0x06, 0x00, 0x00, 0x0C]);
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        assert_eq!(
            render(&lift(&listing, &table, &callees, None).unwrap()),
            [
                "       global_3.field_34 = 5",
                "       Call global_3.method_2(global_3.field_34)",
                "       Exit"
            ]
        );
        assert_eq!(class_indexes(&listing, &table), [9]);
        let owned = Callees::default().with_class(
            9,
            Callees::default()
                .with_owner(4)
                .with_method(0x6F8, 2, 8)
                .with_variable(0x700, 0x34, true)
                .with_variable(0x704, 0x34, false),
        );
        assert_eq!(
            method_calls(&listing, &table, &owned, None, 0),
            [(Some(4), 2, vec![(4, None)])]
        );
        // The same class as a class of the runtime with the interface _Box:
        // the get of 0x00A8 of _Box gives Text.
        let types = VbTypes::parse(TYPES.as_bytes()).unwrap();
        let runtime = Callees::default().with_class_interface(9, "_Box");
        let mut get = vec![0x15, 0x9C, 0xFF];
        get.extend_from_slice(&object);
        get.extend_from_slice(&[
            0x1C, 0xA8, 0x00, 0x00, 0x00, 0x03, 0x9C, 0xFF, 0x05, 0x78, 0xFF,
        ]);
        get.push(0x0C);
        let listing_get = disassemble(&Region::new(&get, Off::new(0)), &table);
        assert_eq!(
            render(&lift(&listing_get, &table, &runtime, Some(&types)).unwrap())[0],
            "       local_88 = global_3.Text"
        );
        // Set local_1C = New class 9, then the get of 0x00A8 of _Box on
        // local_1C: New gives the class of its index.
        let new = [
            0x27, 0x09, 0x00, 0x1B, 0xE4, 0xFF, 0x15, 0x9C, 0xFF, 0x06, 0xE4, 0xFF, 0x1C, 0xA8,
            0x00, 0x00, 0x00, 0x03, 0x9C, 0xFF, 0x05, 0x78, 0xFF, 0x0C,
        ];
        let listing_new = disassemble(&Region::new(&new, Off::new(0)), &table);
        assert_eq!(class_indexes(&listing_new, &table), [9]);
        assert_eq!(
            render(&lift(&listing_new, &table, &runtime, Some(&types)).unwrap()),
            [
                "       Set local_1C = New class_9",
                "       local_88 = local_1C.Text",
                "       Exit"
            ]
        );
        let unknown = Callees::default();
        assert_eq!(
            lift(&listing, &table, &unknown, None),
            Err(LiftFault::NoFunction(8))
        );
    }

    #[test]
    fn a_late_get_an_array_and_a_function_give_their_expressions() {
        // local_88 = InStr(1, Me.[DISPID 0], arg_C(2), 0): the late get
        // writes local_4C and pushes its address, which PopAd drops.
        let body = [
            0x16, 0x33, 0xB4, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x34, 0x01, 1, 0, 0, 0, 0x03, 0xB4,
            0xFF, 0x02, 0x02, 0x15, 0x0C, 0x00, 0x35, 0x01, 0, 0, 0, 0, 0x37, 0x05, 0x78, 0xFF,
            0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap()[0],
            "       local_88 = VBA.InStr(1, Me.[DISPID 0x0], arg_C(2), 0)"
        );
        // arg_C(1) = 7
        let store = [0x02, 0x07, 0x02, 0x01, 0x15, 0x0C, 0x00, 0x36, 0x0C];
        assert_eq!(lines(&store).unwrap()[0], "       arg_C(1) = 7");
    }

    #[test]
    fn a_store_between_the_arguments_of_a_call_binds_a_temporary_slot() {
        // import_2(1, arg_C): arg_C goes into local_88 while 1 stays on the
        // stack, and FFree1Ad frees local_88 after the call.
        let freed = [
            0x02, 0x01, 0x03, 0x0C, 0x00, 0x05, 0x78, 0xFF, 0x03, 0x78, 0xFF, 0x12, 0x02, 0x00,
            0x08, 0x00, 0x18, 0x78, 0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&freed).unwrap(),
            ["       Call import_2(arg_C, 1)", "       Exit"]
        );
        // Without the free, local_88 is a variable: its assignment comes
        // before the call, and the call names it.
        let kept = [
            0x02, 0x01, 0x03, 0x0C, 0x00, 0x05, 0x78, 0xFF, 0x03, 0x78, 0xFF, 0x12, 0x02, 0x00,
            0x08, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&kept).unwrap(),
            [
                "       local_88 = arg_C",
                "       Call import_2(local_88, 1)",
                "       Exit"
            ]
        );
    }

    #[test]
    fn erase_a_copy_of_bytes_and_abs_give_statements() {
        // Erase arg_C; local_88 = arg_10 by 8 bytes; local_90 = Abs(arg_14)
        let body = [
            0x15, 0x0C, 0x00, 0x38, 0x15, 0x10, 0x00, 0x15, 0x78, 0xFF, 0x39, 0x08, 0x00, 0x03,
            0x14, 0x00, 0x3A, 0x05, 0x70, 0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap(),
            [
                "       Call VBA.Erase(arg_C)",
                "       local_88 = arg_10",
                "       local_90 = Abs(arg_14)",
                "       Exit"
            ]
        );
    }

    #[test]
    fn redim_gives_its_bounds_and_a_float_call_gives_a_float_result() {
        // ReDim arg_C(0 To 7, 1 To 3)
        let redim = [
            0x02, 0x00, 0x02, 0x07, 0x02, 0x01, 0x02, 0x03, 0x15, 0x0C, 0x00, 0x3B, 0x02, 0x00,
            0x11, 0x00, 0x01, 0x00, 0x80, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&redim).unwrap()[0],
            "       Call VBA.ReDim(arg_C((0 To 7), (1 To 3)))"
        );
        // import_2() * import_3(), both on the floating point unit, into
        // local_A0 as a Double.
        let float = [
            0x12, 0x02, 0x00, 0x00, 0x00, 0x12, 0x03, 0x00, 0x00, 0x00, 0x3C, 0x2B, 0x60, 0xFF,
            0x0C,
        ];
        assert_eq!(
            lines(&float).unwrap()[0],
            "       local_A0 = (import_2() * import_3())"
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
        // The store of 2 binds a temporary slot, and the 1 is still on the
        // stack at the exit.
        assert_eq!(
            lines(&[0x02, 0x01, 0x02, 0x02, 0x05, 0x78, 0xFF, 0x0C]),
            Err(LiftFault::StackLeft(7))
        );
        assert_eq!(
            lines(&[0x02, 0x01, 0x07, 0x54, 0x00]),
            Err(LiftFault::NoObject(2))
        );
        assert_eq!(lines(&[0x0E, 0x0C]), Err(LiftFault::NoFamily(0)));
        assert!(matches!(lines(&[0x77]), Err(LiftFault::NotDecoded(_))));
    }
}
