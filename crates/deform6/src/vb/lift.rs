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
//! | `RedimPreserve` | As `Redim`, and keep the elements |
//! | `Open` | Pop the record length, the file number and the file name, and open the file in the mode of a 16-bit argument |
//! | `CRec2Ansi`, `CRec2Uni` | As `CStr2Ansi`, for a record |
//! | `GetRec4`, `GetRec3`, `PutRec4`, `PutRec3` | Pop the byte size, the variable, the record number for the `4` form, and the file number, and `Get` or `Put` the variable |
//! | `GetRecOwn3`, `PutRecOwn3` | As `GetRec3` and `PutRec3`, with a descriptor of the type in place of the byte size |
//! | `Close` | Pop the file number, and close the file |
//! | `ForVar` | As `ForI4`, for a `Variant` counter |
//! | `NextVar`, `NextStepVar` | As `NextI4`, for a `Variant` counter |
//! | `AddVar`, `SubVar`, `MulVar` | As `ConcatVar`, for `+`, `-` and `*` |
//! | `LitNothing` | Push the 4 bytes of `Nothing` |
//! | `DestructAnsiOFrame` | Nothing: it frees the ANSI copy of a record |
//! | `PrintFile` | Pop the file number and one item, whose bytes with the 4 bytes of a descriptor are a 16-bit argument, and `Print` the item. The corpus holds no `Print` of more items |
//! | `IStDarg` | As `IStStrCopy` |
//! | `AryLdRf`, `Ary1LdRf` | Pop an array and its indexes, and push the address of the element |
//! | an operator and `VarBool`, such as `EqVarBool` | Pop two `Variant` values, push the Boolean of the comparison |
//! | `UMiI2`, `UMiI4`, `UMiR4`, `UMiR8` | The negative of the top value |
//! | `CastAd` | Give the top object the class at an index of the constant table |
//! | `ImpAdCallNonVirt` | As `ImpAdCallHresult` |
//! | `IStStrCopy` | Pop a string into what the frame slot points to |
//! | `LitR4FP` | Push a 4-byte floating point constant on the floating point unit |
//! | `PopTmpLdAdFPR4`, `CVarBoolI2` | Pop a value into a temporary slot, and push its address |
//! | `LateIdCall`, `LateIdCallLdVar` | As `LateMemCall` and `LateMemCallLdVar`, with a 32-bit `DISPID` in place of the index of a name |
//! | `LateIdSt` | Pop a `Variant`, and set the member of the object register of a 32-bit `DISPID` to it |
//! | `FLdVar` | Push the 16 bytes of the `Variant` of a frame slot |
//! | `ImpAdCall` of a function of the runtime | As `ImpAdCall`, with the name of the function and the class of its result when [`Callees`] gives them |
//! | `LateMemCall` | Pop a count of `Variant` values, and call the member of the object register whose name is at an index of the constant table |
//! | `LateMemCallLdVar`, `LateMemLdVar` | As `LateMemCall`, or with no arguments, into a frame slot, and push the address of the slot |
//! | `ImpAdStAdFunc` | Pop an object, and `Set` a global at a 16-bit index to it |
//! | `LdPrVar` | Pop the address of a `Variant`, and load its object into the object register |
//! | `CStrVarVal`, `CStrVarTmp`, `CBoolVarNull`, `FnLenVar` | Convert the `Variant` at the popped address, or give its `Len` |
//! | `NextStepI2` | As `NextI2` |
//! | `ConcatVar` | Pop the addresses of two `Variant` values, push the address of their `&` |
//! | `FDupVar` | Copy the `Variant` of the first frame slot into the second |
//! | `ForStepI2` | As `ForStepI4` |
//! | `FnFixR4`, `FnFixR8` | `Fix` of a value of the floating point unit |
//! | `VCall` | As `VCallHresult`, with no argument that names the interface |
//! | `BranchT` | Pop a condition, and branch when it is true |
//! | `Resume` | `Resume Next` for `0xFFFF`, `Resume` for `0xFFFE`, else `Resume` to the label at the offset |
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
//! It steps over the result of an import call on the floating point unit,
//! which is not on the stack, and so does each opcode that takes no value
//! of the floating point unit. It refuses the call when a popped value has no known size on the stack,
//! such as a value of the floating point unit, or when the sizes do not add
//! up to the bytes. A value of 4 bytes is a value of a type of 4 bytes or
//! less, or an address; `PopAdLdVar` gives a `Variant` of 16 bytes. An
//! `ImpAdCall` handler that serves `ImpAdCallFPR4` and `ImpAdCallFPR8` too
//! can leave a result on the floating point unit, which is not the stack.
//! The lift gives it a result on the floating point unit. When no opcode,
//! such as `MulR4`, takes that result before the end of the statement, the
//! call becomes a call statement of its own, at its own offset.
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
//! `arg_C`, and a field by its offset, such as `local_88.field_C`. A field
//! of `Me` has no `Me.`, such as `field_54`, because Basic does not reach a
//! private variable of a module through `Me`. A branch is a
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
    /// A global, at an index of the constant table.
    Global(u16),
    /// A variable of a module, by the address that the constant table
    /// gives for it. Each object that names the variable gives the same
    /// address, at an index of its own.
    Variable(u32),
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
    /// A name of the project, such as the form of a global that holds the
    /// instance of the form.
    Name(String),
    /// The global object of the runtime, whose members Basic writes with no
    /// object, such as `Screen` and `App`.
    Implicit,
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
    /// A procedure, by its index in the constant table of the object.
    Import(u16),
    /// A procedure of a DLL that a `Declare` names, by its export name.
    Declare(String),
    /// A procedure of a module of the project: the name of the module and
    /// the name of the procedure.
    Module(String, String),
    /// A named function of an object.
    Member(Box<Expr>, String),
}

impl Callee {
    /// The text of the callee.
    fn text(&self) -> String {
        match self {
            Self::Import(index) => format!("import_{index:X}"),
            Self::Declare(name) => name.clone(),
            Self::Module(module, name) => format!("{module}.{name}"),
            Self::Member(object, name) => member_text(object, name),
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
    controls: Vec<(u16, String, Option<String>)>,
    strings: Vec<(u16, String)>,
    names: Vec<(u16, String)>,
    variables: Vec<(u16, u32, bool)>,
    classes: Vec<(u16, Callees)>,
    class_interfaces: Vec<(u16, String)>,
    imports: Vec<(u16, String, Option<String>)>,
    declares: Vec<(u16, String)>,
    variant_results: Vec<u16>,
    functions: Vec<u16>,
    function_stubs: Vec<u16>,
    stubs: Vec<(u16, ProjectCall)>,
    arguments: Vec<(u16, i16, String)>,
    argument_sizes: Vec<(u16, Vec<u8>)>,
    procedures: Vec<(u16, String)>,
    form_name: Option<String>,
    object_name: Option<String>,
    globals: Vec<(u16, u32)>,
    base: Option<String>,
    owner: Option<u16>,
}

/// A procedure of the project that an `ImpAdCall` goes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectCall {
    /// A procedure of a module: the name of the module and of the
    /// procedure.
    Module(String, String),
    /// A procedure of an object, by its name. The first argument of the
    /// call is the object.
    Object(String),
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
            .push((vtable_offset, name.to_owned(), Some(interface.to_owned())));
        self
    }

    /// Adds the accessor of the control `name` at `vtable_offset`, whose
    /// interface the lift does not know, such as a control of an OCX.
    #[must_use]
    pub fn with_untyped_control(mut self, vtable_offset: u16, name: &str) -> Self {
        self.controls.push((vtable_offset, name.to_owned(), None));
        self
    }

    /// Adds the accessor of a public variable at `vtable_offset`: the get
    /// when `get` is true, else the let, of the field at `field`.
    #[must_use]
    pub fn with_variable(mut self, vtable_offset: u16, field: u32, get: bool) -> Self {
        self.variables.push((vtable_offset, field, get));
        self
    }

    /// Gives the field of each accessor of a public variable, once each, in
    /// the order of the fields.
    #[must_use]
    pub fn variable_fields(&self) -> BTreeSet<u32> {
        self.variables.iter().map(|(_, field, _)| *field).collect()
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

    /// Adds the name `name` of the method `index` of the method table: the
    /// name of a public procedure. It replaces a name that the method had.
    #[must_use]
    pub fn with_procedure(mut self, index: u16, name: &str) -> Self {
        self.procedures.retain(|(at, _)| *at != index);
        self.procedures.push((index, name.to_owned()));
        self
    }

    /// Gives the name of the method `index` of the method table, or
    /// `method_` and the index when the procedure is private.
    #[must_use]
    pub fn procedure(&self, index: u16) -> String {
        self.procedures
            .iter()
            .find(|(at, _)| *at == index)
            .map_or_else(|| format!("method_{index}"), |(_, name)| name.clone())
    }

    /// Adds the address `address` of the variable at `index` of the constant
    /// table.
    #[must_use]
    pub fn with_global(mut self, index: u16, address: u32) -> Self {
        self.globals.push((index, address));
        self
    }

    /// Sets the name of the form whose vtable the profile gives. A global of
    /// the class of a form holds the instance of the form, which Basic
    /// names by the name of the form.
    #[must_use]
    pub fn with_form_name(mut self, name: &str) -> Self {
        self.form_name = Some(name.to_owned());
        self
    }

    /// Sets the name of the object whose vtable the profile gives. `New`
    /// of the class writes this name.
    #[must_use]
    pub fn with_object_name(mut self, name: &str) -> Self {
        self.object_name = Some(name.to_owned());
        self
    }

    /// Gives the name of the object whose vtable the profile gives.
    #[must_use]
    pub fn object_name(&self) -> Option<&str> {
        self.object_name.as_deref()
    }

    /// Gives the name of the form whose vtable the profile gives.
    #[must_use]
    pub fn form_name(&self) -> Option<&str> {
        self.form_name.as_deref()
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

    /// Adds the bytes of each argument of the method `method`, first
    /// argument first, that each call of it in the project gives.
    #[must_use]
    pub fn with_argument_sizes(mut self, method: u16, sizes: &[u8]) -> Self {
        self.argument_sizes.retain(|(at, _)| *at != method);
        self.argument_sizes.push((method, sizes.to_vec()));
        self
    }

    /// Gives the bytes of each argument of the method `method`, when each
    /// call of it gives the same.
    #[must_use]
    pub fn argument_sizes(&self, method: u16) -> Option<&[u8]> {
        self.argument_sizes
            .iter()
            .find(|(at, _)| *at == method)
            .map(|(_, sizes)| sizes.as_slice())
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

    /// Adds the function of the runtime `name` that the entry at `index` of
    /// the constant table calls, with the interface of its object result.
    #[must_use]
    pub fn with_import(mut self, index: u16, name: &str, result: Option<&str>) -> Self {
        self.imports
            .push((index, name.to_owned(), result.map(str::to_owned)));
        self
    }

    /// Adds the procedure of the project `call` that the stub at `index` of
    /// the constant table goes to.
    #[must_use]
    pub fn with_project_call(mut self, index: u16, call: ProjectCall) -> Self {
        self.stubs.push((index, call));
        self
    }

    /// Gives the procedure of the project that the stub at `index` of the
    /// constant table goes to.
    fn project_call(&self, index: u16) -> Option<&ProjectCall> {
        self.stubs
            .iter()
            .find(|(at, _)| *at == index)
            .map(|(_, call)| call)
    }

    /// Adds the export name `name` of the procedure of a DLL at `index` of
    /// the constant table.
    #[must_use]
    pub fn with_declare(mut self, index: u16, name: &str) -> Self {
        self.declares.push((index, name.to_owned()));
        self
    }

    /// Gives the export name of the procedure of a DLL at `index` of the
    /// constant table.
    fn declare(&self, index: u16) -> Option<&str> {
        self.declares
            .iter()
            .find(|(at, _)| *at == index)
            .map(|(_, name)| name.as_str())
    }

    /// Gives the name of the function of the runtime at `index` of the
    /// constant table, and the interface of its result.
    fn import(&self, index: u16) -> Option<(&str, Option<&str>)> {
        self.imports
            .iter()
            .find(|(at, _, _)| *at == index)
            .map(|(_, name, result)| (name.as_str(), result.as_deref()))
    }

    /// Marks the method `index` of the method table as a `Function`: a call
    /// of it passes the address of its result last.
    #[must_use]
    pub fn with_function(mut self, index: u16) -> Self {
        self.functions.push(index);
        self
    }

    /// Tells whether the method `index` of the method table is a `Function`.
    #[must_use]
    pub fn is_function(&self, index: u16) -> bool {
        self.functions.contains(&index)
    }

    /// Marks the procedure of the project that the stub at `index` of the
    /// constant table goes to as a `Function`. A call of a `Function` of a
    /// module passes the address of its result first, and a call of one of
    /// an object passes it last.
    #[must_use]
    pub fn with_function_stub(mut self, index: u16) -> Self {
        self.function_stubs.push(index);
        self
    }

    /// Marks the function of the runtime at `index` of the constant table as
    /// one that returns a `Variant`: a call of it passes the address of the
    /// result first.
    #[must_use]
    pub fn with_variant_result(mut self, index: u16) -> Self {
        self.variant_results.push(index);
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

    /// Adds the member name `name` at `index` of the constant table.
    #[must_use]
    pub fn with_name(mut self, index: u16, name: &str) -> Self {
        self.names.push((index, name.to_owned()));
        self
    }

    /// Gives the member name at `index` of the constant table.
    fn name(&self, index: u16) -> Option<&str> {
        self.names
            .iter()
            .find(|(at, _)| *at == index)
            .map(|(_, name)| name.as_str())
    }

    /// Gives the expression of the string at `index` of the constant table.
    fn string(&self, index: u16) -> Expr {
        self.strings
            .iter()
            .find(|(at, _)| *at == index)
            .map_or(Expr::Constant(index), |(_, text)| Expr::Str(text.clone()))
    }

    /// Gives the name of the control at `vtable_offset`, and its interface
    /// when the lift knows it.
    #[must_use]
    pub fn control(&self, vtable_offset: u16) -> Option<(&str, Option<&str>)> {
        self.controls
            .iter()
            .find(|(offset, _, _)| *offset == vtable_offset)
            .map(|(_, name, interface)| (name.as_str(), interface.as_deref()))
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
            Self::Field(object, offset) if **object == Self::Arg(8) => format!("field_{offset:X}"),
            // The field at 0 of an element of an array, which `AryLdPr`
            // gives the address of, is the element itself.
            Self::Field(object, 0) if matches!(**object, Self::Index(..)) => object.text(),
            Self::Field(object, offset) => format!("{}.field_{offset:X}", object.text()),
            Self::Global(index) => format!("global_{index:X}"),
            Self::Variable(address) => format!("g_{address:X}"),
            Self::Binary(op, left, right) => {
                format!("({} {} {})", left.text(), op.text(), right.text())
            }
            Self::Convert(function, value) => format!("{function}({})", value.text()),
            Self::Call(callee, args) => format!("{}({})", callee.text(), arguments_text(args)),
            Self::Member(object, name) => member_text(object, name),
            Self::Word(word) => (*word).to_owned(),
            Self::Name(name) => name.clone(),
            Self::Implicit => String::new(),
            Self::Real(bits) => format!("{:?}", f64::from_bits(*bits)),
            Self::Str(text) => string_text(text),
            Self::Constant(index) => format!("const_{index:X}"),
            Self::New(index) => format!("New class_{index:X}"),
            Self::Late(object, 0) => object.text(),
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

/// Gives the expression of the variable at `index` of the constant table:
/// by its address when `callees` gives it.
fn global_at(callees: &Callees, index: u16) -> Expr {
    callees
        .globals
        .iter()
        .find(|(at, _)| *at == index)
        .map_or(Expr::Global(index), |(_, address)| Expr::Variable(*address))
}

/// The text of the string `text` as Basic writes it. A literal of Basic
/// holds no control character, so each one is a constant, such as
/// `vbCrLf`, or `Chr$` and its code, joined to the literals with `&`.
fn string_text(text: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut literal = String::new();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        let constant = match character {
            '\r' if characters.peek() == Some(&'\n') => {
                characters.next();
                "vbCrLf".to_owned()
            }
            '\r' => "vbCr".to_owned(),
            '\n' => "vbLf".to_owned(),
            '\t' => "vbTab".to_owned(),
            '\0' => "vbNullChar".to_owned(),
            control if control.is_control() => format!("Chr$({})", u32::from(control)),
            '"' => {
                literal.push_str("\"\"");
                continue;
            }
            other => {
                literal.push(other);
                continue;
            }
        };
        if !literal.is_empty() {
            parts.push(format!("\"{literal}\""));
            literal.clear();
        }
        parts.push(constant);
    }
    if !literal.is_empty() || parts.is_empty() {
        parts.push(format!("\"{literal}\""));
    }
    parts.join(" & ")
}

/// Gives the name that Basic writes for the function of the runtime `name`:
/// `Left$` for `_B_str_Left`, which returns a `String`, and `Left` for
/// `_B_var_Left`, which returns a `Variant`.
fn basic_name(name: &str) -> String {
    if let Some(bare) = name.strip_prefix("_B_str_") {
        format!("{bare}$")
    } else if let Some(bare) = name.strip_prefix("_B_var_") {
        bare.to_owned()
    } else {
        name.to_owned()
    }
}

/// The text of the member `name` of `object`: with no object for a member
/// of the global object of the runtime.
fn member_text(object: &Expr, name: &str) -> String {
    match object {
        Expr::Implicit => name.to_owned(),
        _ => format!("{}.{name}", object.text()),
    }
}

/// The text of a list of arguments. An optional argument that the call
/// leaves out is `Missing`: Basic writes it as nothing, and drops it at the
/// end of the list.
fn arguments_text(args: &[Expr]) -> String {
    let missing = |expr: &Expr| *expr == Expr::Word("Missing");
    let given = args
        .iter()
        .rposition(|expr| !missing(expr))
        .map_or(0, |last| last.saturating_add(1));
    args.iter()
        .take(given)
        .map(|expr| {
            if missing(expr) {
                String::new()
            } else {
                expr.text()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A statement of Basic with a form of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyword {
    /// `ReDim` or `ReDim Preserve`: the array, then the lower and the upper
    /// bound of each dimension.
    ReDim {
        /// Whether the statement keeps the values.
        preserve: bool,
    },
    /// `Erase`: the array.
    Erase,
    /// `Open`: the path, the file number and the record length, which is
    /// `-1` when the statement gives none.
    Open {
        /// The mode, such as `Binary`, or `None` for a mode that the lift
        /// does not know.
        mode: Option<&'static str>,
    },
    /// `Close`: the file number.
    Close,
    /// `Print #`: the file number and the value.
    Print,
    /// `Get` or `Put`: the file number, the record or an empty word, and the
    /// variable.
    Record {
        /// `Get` or `Put`.
        name: &'static str,
    },
    /// The `Line` method: the object, the two points, and the color when
    /// the statement gives one.
    Line {
        /// Whether the statement ends with `BF`.
        filled: bool,
    },
    /// The `Circle` method: the object, the center, the radius and the
    /// color.
    Circle,
    /// The `PSet` method: the object, the point and the color.
    PSet,
}

impl Keyword {
    /// The text of the statement with the expressions `args`.
    fn text(self, args: &[Expr]) -> String {
        let all = |from: usize| {
            args.get(from..)
                .unwrap_or_default()
                .iter()
                .map(Expr::text)
                .collect::<Vec<_>>()
        };
        let at = |index: usize| args.get(index).map(Expr::text).unwrap_or_default();
        match self {
            Self::ReDim { preserve } => {
                let ranges: Vec<String> = args
                    .get(1..)
                    .unwrap_or_default()
                    .chunks(2)
                    .map(|pair| {
                        let texts: Vec<String> = pair.iter().map(Expr::text).collect();
                        texts.join(" To ")
                    })
                    .collect();
                let word = if preserve { "ReDim Preserve" } else { "ReDim" };
                format!("{word} {}({})", at(0), ranges.join(", "))
            }
            Self::Erase => format!("Erase {}", all(0).join(", ")),
            Self::Open { mode } => {
                let mode = mode.map_or_else(String::new, |mode| format!(" For {mode}"));
                let length = match args.get(2) {
                    Some(Expr::Const(-1)) | None => String::new(),
                    Some(length) => format!(" Len = {}", length.text()),
                };
                format!("Open {}{mode} As #{}{length}", at(0), at(1))
            }
            Self::Close => format!("Close #{}", at(0)),
            Self::Print => format!("Print #{}, {}", at(0), all(1).join("; ")),
            Self::Record { name } => format!("{name} #{}, {}, {}", at(0), at(1), at(2)),
            Self::Line { filled } => {
                let mut text = format!(
                    "{} ({}, {})-({}, {})",
                    member_text(args.first().unwrap_or(&Expr::Implicit), "Line"),
                    at(1),
                    at(2),
                    at(3),
                    at(4)
                );
                if args.len() > 5 {
                    text.push_str(&format!(", {}", at(5)));
                }
                if filled {
                    text.push_str(", BF");
                }
                text
            }
            Self::Circle => format!(
                "{} ({}, {}), {}, {}",
                member_text(args.first().unwrap_or(&Expr::Implicit), "Circle"),
                at(1),
                at(2),
                at(3),
                at(4)
            ),
            Self::PSet => format!(
                "{} ({}, {}), {}",
                member_text(args.first().unwrap_or(&Expr::Implicit), "PSet"),
                at(1),
                at(2),
                at(3)
            ),
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
    /// A branch to `target` when `condition` is true.
    IfGoTo {
        /// The condition.
        condition: Expr,
        /// The offset of the target in the body.
        target: u16,
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
    Exit {
        /// Whether the procedure is a `Function`: its exit opcode is one of
        /// `ExitProcCb`, which returns a value.
        function: bool,
    },
    /// A call whose result is not used.
    Call(Callee, Vec<Expr>),
    /// A statement of Basic that is not a call, with its expressions in the
    /// order that [`Keyword`] gives.
    Keyword(Keyword, Vec<Expr>),
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
    /// `Resume`: `None` for `Resume Next`, `Some(0)` for `Resume` of the
    /// statement of the error, or `Some` target offset.
    Resume(Option<u16>),
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
            Self::IfGoTo { condition, target } => Self::IfGoTo {
                condition: condition.resolve(kept),
                target,
            },
            Self::IfNotGoTo { condition, target } => Self::IfNotGoTo {
                condition: condition.resolve(kept),
                target,
            },
            Self::Call(callee, args) => Self::Call(
                callee.resolve(kept),
                args.into_iter().map(|arg| arg.resolve(kept)).collect(),
            ),
            Self::Keyword(keyword, args) => Self::Keyword(
                keyword,
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
            Self::IfGoTo { condition, target } => {
                format!("If {} Then GoTo L{target:04X}", condition.text())
            }
            Self::IfNotGoTo { condition, target } => {
                format!("If Not {} Then GoTo L{target:04X}", condition.text())
            }
            Self::Resume(None) => "Resume Next".to_owned(),
            Self::Resume(Some(0)) => "Resume".to_owned(),
            Self::Resume(Some(target)) => format!("Resume L{target:04X}"),
            Self::GoTo(target) => format!("GoTo L{target:04X}"),
            Self::End => "End".to_owned(),
            Self::Exit { function: false } => "Exit Sub".to_owned(),
            Self::Exit { function: true } => "Exit Function".to_owned(),
            Self::Call(callee, args) => format!("Call {}({})", callee.text(), arguments_text(args)),
            Self::Keyword(keyword, args) => keyword.text(args),
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
    /// The offsets of the opcodes with no effect before the first opcode,
    /// such as `Bos` and `FFree`, which a branch can name.
    pub also_at: Vec<u32>,
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
    /// A branch names an offset inside the body where no statement starts.
    /// A label there would change where the branch goes.
    BranchTarget(u32),
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
    BranchTrue,
    Resume,
    Branch,
    End,
    Exit {
        /// Whether the opcode returns a value.
        function: bool,
    },
    ThisCall,
    ImportCall {
        result: bool,
    },
    Free,
    ObjectStore,
    ObjectCall {
        pushes: bool,
    },
    GlobalLoad,
    GlobalObjectRegister,
    FieldLoad,
    FieldObjectRegister,
    FrameFieldLoad,
    FrameFieldObjectRegister,
    FrameFieldStore,
    StoreKeep {
        object: bool,
    },
    PopTemp,
    LitString,
    LitNothing,
    LitVariant(VariantKind),
    LitReal,
    NewObject,
    NewIfNull,
    Nop,
    Function(&'static str),
    FunctionOf(&'static str, u8),
    FloatFunction(&'static str),
    ArrayErase,
    GlobalObjectStore,
    FrameLoadVariant,
    LateCall {
        result: bool,
        arguments: bool,
        by_id: bool,
    },
    LateStore,
    VariantObjectRegister,
    VariantBinary(BinaryOp),
    VariantCopy,
    Redim {
        preserve: bool,
    },
    Open,
    Close,
    FileRecord {
        name: &'static str,
        record: bool,
        sized: bool,
    },
    PrintFile,
    ArrayReference {
        dimensions_argument: bool,
    },
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
    For {
        step: bool,
    },
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
        "BranchT" => Family::BranchTrue,
        "Resume" => Family::Resume,
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
        "LitNothing" => Family::LitNothing,
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
        "Redim" => Family::Redim { preserve: false },
        "RedimPreserve" => Family::Redim { preserve: true },
        "Open" => Family::Open,
        "CRec2Ansi" | "CRec2Uni" => Family::StringCopy,
        "GetRec4" => Family::FileRecord {
            name: "Get",
            record: true,
            sized: true,
        },
        "GetRec3" => Family::FileRecord {
            name: "Get",
            record: false,
            sized: true,
        },
        "PutRec4" => Family::FileRecord {
            name: "Put",
            record: true,
            sized: true,
        },
        "PutRec3" => Family::FileRecord {
            name: "Put",
            record: false,
            sized: true,
        },
        "GetRecOwn3" => Family::FileRecord {
            name: "Get",
            record: false,
            sized: false,
        },
        "PutRecOwn3" => Family::FileRecord {
            name: "Put",
            record: false,
            sized: false,
        },
        "DestructAnsiOFrame" => Family::Nop,
        "PrintFile" => Family::PrintFile,
        "Close" => Family::Close,
        "ForVar" => Family::For { step: false },
        "NextVar" | "NextStepVar" => Family::Next,
        "AddVar" => Family::VariantBinary(BinaryOp::Add),
        "SubVar" => Family::VariantBinary(BinaryOp::Sub),
        "MulVar" => Family::VariantBinary(BinaryOp::Mul),
        "IStDarg" => Family::IndirectStore,
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
        "ImpAdStAdFunc" => Family::GlobalObjectStore,
        "FLdVar" => Family::FrameLoadVariant,
        "LateMemCall" => Family::LateCall {
            result: false,
            arguments: true,
            by_id: false,
        },
        "LateMemCallLdVar" => Family::LateCall {
            result: true,
            arguments: true,
            by_id: false,
        },
        "LateMemLdVar" => Family::LateCall {
            result: true,
            arguments: false,
            by_id: false,
        },
        "LateIdCall" => Family::LateCall {
            result: false,
            arguments: true,
            by_id: true,
        },
        "LateIdCallLdVar" => Family::LateCall {
            result: true,
            arguments: true,
            by_id: true,
        },
        "LateIdSt" => Family::LateStore,
        "LdPrVar" => Family::VariantObjectRegister,
        "CStrVarVal" | "CStrVarTmp" => Family::Function("CStr"),
        "FnLenVar" => Family::Function("Len"),
        "NextStepI2" => Family::Next,
        "CBoolVarNull" => Family::Function("CBool"),
        "ConcatVar" => Family::VariantBinary(BinaryOp::Concat),
        "FDupVar" => Family::VariantCopy,
        "ForStepI2" => Family::For { step: true },
        "FnFixR4" | "FnFixR8" => Family::FloatFunction("Fix"),
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
        "VCallHresult" | "VCall" | "VCallFPR4" | "VCallFPR8" | "VCallHidden" => {
            Family::ObjectCall { pushes: false }
        }
        "FFree1Ad" | "FFree1Str" | "FFree1Var" | "FFreeAd" | "FFreeStr" | "FFreeVar" => {
            Family::Free
        }
        _ if name.starts_with("ExitProc") => Family::Exit {
            function: name.starts_with("ExitProcCb"),
        },
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
/// point unit: a name that ends in a floating point type, such as `MulR4`,
/// `CI4R4` or `FStFPR8`. A load, a literal and a call end in the type of
/// the value that they give, and take none.
fn takes_a_float(name: &str) -> bool {
    const GIVE_A_FLOAT: [&str; 7] = [
        "FLd",
        "ILd",
        "MemLd",
        "ImpAdLd",
        "Lit",
        "ImpAdCall",
        "VCall",
    ];
    ["R4", "R8"].iter().any(|kind| name.ends_with(kind))
        && !GIVE_A_FLOAT.iter().any(|prefix| name.starts_with(prefix))
}

/// The bytes on the stack of the value that a load of the names `names`
/// pushes: 4 for a word, 8 for a `Double` or a `Currency` that the load
/// pushes as two words, such as `FLdR8`, and else 0.
fn load_bytes(names: &[String]) -> u8 {
    if is_word(names) {
        4
    } else if names
        .iter()
        .all(|name| (name.ends_with("R8") || name.ends_with("Cy")) && !name.contains("FPR"))
    {
        8
    } else {
        0
    }
}

/// The name that the lift gives a member of a late-bound call by its
/// `DISPID`.
fn dispid_name(dispid: u32) -> String {
    format!("[DISPID {dispid:#X}]")
}

/// Gives the name of the member `dispid` of the interface `class`, when the
/// types file gives it.
fn named_dispid(types: Option<&VbTypes>, class: Option<&str>, dispid: u32) -> Option<String> {
    let function = types?.interface(class?)?.function_of_dispid(dispid)?;
    Some(member_name(function))
}

/// Gives the name of the member `dispid` of the interface `class`, or else
/// the `DISPID` itself.
fn dispid_member(types: Option<&VbTypes>, class: Option<&str>, dispid: u32) -> String {
    named_dispid(types, class, dispid).unwrap_or_else(|| dispid_name(dispid))
}

/// Reads an unsigned 32-bit argument.
fn u32_at(arguments: &[u8], at: usize) -> Option<u32> {
    let bytes = arguments.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
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

/// The Basic conversions whose result is 4 bytes on the stack. `CVar`
/// writes a `Variant` into its frame slot and pushes the address of it.
const WORD_CONVERSIONS: &[&str] = &["CByte", "CInt", "CLng", "CStr", "CVar"];

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
    /// Tells whether the value is the result of an import call on the
    /// floating point unit: a call with 0 bytes on the stack. When no opcode
    /// takes it before the end of the statement, the call is a statement of
    /// its own.
    const fn is_float_call(&self) -> bool {
        self.bytes == 0 && matches!(self.expr, Expr::Call(..))
    }

    /// A value of `bytes` bytes, with no slot and no class.
    const fn sized(expr: Expr, bytes: u8) -> Self {
        Self {
            expr,
            bytes,
            slot: None,
            class: None,
        }
    }

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
    let mut floats = Vec::new();
    while left > 0 {
        let value = stack.pop().ok_or(LiftFault::StackShort(at))?;
        if value.is_float_call() {
            floats.push(value);
            continue;
        }
        left = left
            .checked_sub(u16::from(value.bytes))
            .filter(|_| value.bytes > 0)
            .ok_or(LiftFault::CallArguments(at))?;
        args.push(value);
    }
    stack.extend(floats.into_iter().rev());
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
    /// The offsets of the opcodes with no effect before the next statement.
    leading: Vec<u32>,
    /// The offsets of the opcodes with no effect after a call that stays on
    /// the floating point unit, before the statement after it.
    resumed: Vec<u32>,
    object: Option<Object>,
    bindings: BTreeMap<i16, Object>,
    pending: BTreeMap<i16, PendingSet>,
    /// The pending statements that a later store to the same slot replaced
    /// before an `FFree`: they stay.
    kept: Vec<PendingSet>,
    stores: u32,
    /// The first offset of each import call whose result is on the
    /// floating point unit, in the order of the stack.
    floats: Vec<u32>,
    /// The offset of the opcode after the last of those calls.
    resume: Option<u32>,
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
    /// The number of values on the stack. A result of an import call on the
    /// floating point unit is not on the stack.
    fn depth(&self) -> usize {
        self.stack
            .iter()
            .filter(|value| !value.is_float_call())
            .count()
    }

    /// Binds `slot` to `value`, and keeps `stmt` until an `FFree` of the slot
    /// drops it. A statement that is still pending for the slot stays. The
    /// statement goes after the import calls on the floating point unit
    /// that the end of the statement gives as statements of their own.
    fn bind_pending(&mut self, slot: i16, value: Value, index: usize, stmt: LiftedStmt) {
        if let Some(old) = self.pending.remove(&slot) {
            self.kept.push(old);
        }
        let floats = self.stack.len().saturating_sub(self.depth());
        let index = index.saturating_add(floats);
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
/// whose controls have the interface `element`, or an interface that the
/// lift does not know when `element` is empty.
///
/// The runtime calls this object `tagCARR`. Its vtable for `IVBControl` in
/// `MSVBVM60.DLL` 6.0.98.2 holds `Item(Integer, CTL**)` at `0x40`, then
/// `LBound`, `UBound` and `Count` at `0x44`, `0x48` and `0x4C`. Each of the
/// last three writes an `Integer`. No type library gives them.
fn control_array_function(element: &str, vtable_offset: u16) -> Option<TypeFunction> {
    let (name, arg_bytes, result_interface) = match vtable_offset {
        0x40 => ("Item", 8, (!element.is_empty()).then(|| element.to_owned())),
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
        dispid: None,
    })
}

/// The prefix of the class of an object of the project, before the index of
/// its class in the constant table.
const PROJECT_CLASS: char = '@';

/// The interface of the global object of the runtime, such as `Screen` and
/// `App`, in `VB6.OLB`.
const GLOBAL_INTERFACE: &str = "VBGlobal";

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
        ([_, _, ..], true, _) | ([_, _, ..], false, true) => {
            let mut indexes = expressions(args);
            let value = indexes.pop().ok_or(LiftFault::CallArguments(at))?;
            let target = Expr::Call(Callee::Member(Box::new(object), name), indexes);
            if has("let") {
                Stmt::Assign { target, value }
            } else {
                Stmt::Set { target, value }
            }
        }
        _ => {
            let args = expressions(args);
            drawing(&name, &object, &args)
                .unwrap_or_else(|| Stmt::Call(Callee::Member(Box::new(object), name), args))
        }
    }))
}

/// Gives the statement of a call of `Line`, `Circle` or `PSet` on `object`.
/// The first argument holds flags that tell which parts the statement
/// gives. The corpus shows these values only: `Line` 4 with two points, 6
/// with a color too, and 38 with `BF` too; `Circle` 2 and `PSet` 2 with a
/// color. Another value gives `None`, and the call stays a call.
fn drawing(name: &str, object: &Expr, args: &[Expr]) -> Option<Stmt> {
    let with = |keyword: Keyword, parts: &[Expr]| {
        let mut all = vec![object.clone()];
        all.extend_from_slice(parts);
        Some(Stmt::Keyword(keyword, all))
    };
    match (name, args) {
        ("Line", [Expr::Const(4), points @ .., _]) if points.len() == 4 => {
            with(Keyword::Line { filled: false }, points)
        }
        ("Line", [Expr::Const(6), parts @ ..]) if parts.len() == 5 => {
            with(Keyword::Line { filled: false }, parts)
        }
        ("Line", [Expr::Const(38), parts @ ..]) if parts.len() == 5 => {
            with(Keyword::Line { filled: true }, parts)
        }
        (
            "Circle",
            [
                Expr::Const(2),
                x,
                y,
                radius,
                color,
                Expr::Const(0),
                Expr::Const(0),
                Expr::Const(0),
            ],
        ) => with(
            Keyword::Circle,
            &[x.clone(), y.clone(), radius.clone(), color.clone()],
        ),
        ("PSet", [Expr::Const(2), parts @ ..]) if parts.len() == 3 => with(Keyword::PSet, parts),
        _ => None,
    }
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
            class: interface.map(str::to_owned),
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
        let callee = Callee::Member(Box::new(object), profile.procedure(method.index));
        let function = profile.is_function(method.index);
        return Ok(call_or_function(state, callee, args, function));
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

/// Gives the class that the entry at `index` of the constant table names:
/// an object of the project, or an interface of the runtime.
fn class_at(callees: &Callees, index: u16) -> Option<String> {
    if callees.class(index).is_some() {
        Some(format!("{PROJECT_CLASS}{index}"))
    } else {
        callees.class_interface(index).map(str::to_owned)
    }
}

/// Gives the call of a method of the project with `args`, first argument
/// first. A call of a `Function`, or a call with values under its arguments,
/// which is part of an expression, passes the local that takes the result
/// last. Inside an expression the lift binds that local to the call, and
/// else the call is an assignment to the local. Any other call is a
/// statement.
fn call_or_function(
    state: &mut State,
    callee: Callee,
    mut args: Vec<Value>,
    function: bool,
) -> Option<Stmt> {
    if let Some(slot) = args
        .last()
        .and_then(|value| value.slot)
        .filter(|slot| *slot < 0 && (function || state.depth() > 0))
    {
        args.pop();
        result_of(state, slot, Expr::Call(callee, expressions(args)))
    } else {
        Some(Stmt::Call(callee, expressions(args)))
    }
}

/// Gives the call `call`, whose result goes to the local at `slot`: bound
/// to the local inside an expression, and else an assignment to it.
fn result_of(state: &mut State, slot: i16, call: Expr) -> Option<Stmt> {
    if state.depth() > 0 {
        state.bindings.insert(slot, (call, None));
        None
    } else {
        Some(Stmt::Assign {
            target: Expr::frame(slot),
            value: call,
        })
    }
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
        let next_offset = listing
            .instructions
            .get(position.saturating_add(1))
            .map(|next| next.offset);
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
        let float = names.iter().any(|name| takes_a_float(name));
        let pop_value = |state: &mut State| {
            let at_top = if float {
                state.stack.len().checked_sub(1)
            } else {
                state.stack.iter().rposition(|value| !value.is_float_call())
            };
            at_top
                .map(|index| state.stack.remove(index))
                .ok_or(LiftFault::StackShort(at))
        };
        let pop = |state: &mut State| pop_value(state).map(|value| value.expr);
        let stmt = match family {
            Family::Lit(len) => {
                let value = constant(arguments, len).ok_or_else(short)?;
                state.stack.push(Value::plain(Expr::Const(value), true));
                None
            }
            Family::FrameLoadVariant => {
                let value = state.load(offset16()?, false);
                state.stack.push(Value { bytes: 16, ..value });
                None
            }
            Family::FrameLoad | Family::ArgRef => {
                let value = state.load(offset16()?, is_word(names));
                state.stack.push(Value {
                    bytes: load_bytes(names),
                    ..value
                });
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
            Family::FrameStore if state.depth() > 1 => {
                let value = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                let slot = offset16()?;
                let assign = LiftedStmt {
                    offset: first,
                    also_at: state.leading.clone(),
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
            Family::LateCall {
                result,
                arguments: with_arguments,
                by_id,
            } => {
                let (object, class) = state.object.clone().ok_or(LiftFault::NoObject(at))?;
                let skip = if result { 2 } else { 0 };
                let (name, count_at) = if by_id {
                    let dispid = u32_at(arguments, skip).ok_or_else(short)?;
                    (dispid_member(types, class.as_deref(), dispid), 4)
                } else {
                    let name = callees
                        .name(u16_at(arguments, skip).ok_or_else(short)?)
                        .ok_or(LiftFault::NoCallee(at))?
                        .to_owned();
                    (name, 2)
                };
                let count = if with_arguments {
                    u16_at(arguments, skip.saturating_add(count_at)).ok_or_else(short)?
                } else {
                    0
                };
                let bytes = count.checked_mul(16).ok_or(LiftFault::CallArguments(at))?;
                let mut args = expressions(call_arguments(&mut state.stack, bytes, at)?);
                args.reverse();
                let callee = Callee::Member(Box::new(object), name);
                if result {
                    let slot = offset16()?;
                    let expr = if with_arguments {
                        Expr::Call(callee, args)
                    } else {
                        let Callee::Member(object, name) = callee else {
                            return Err(LiftFault::NoCallee(at));
                        };
                        Expr::Member(object, name)
                    };
                    state.bindings.insert(slot, (expr.clone(), None));
                    state.stack.push(Value {
                        expr,
                        bytes: 4,
                        slot: Some(slot),
                        class: None,
                    });
                    None
                } else {
                    let Callee::Member(object, name) = callee else {
                        return Err(LiftFault::NoCallee(at));
                    };
                    Some(
                        drawing(&name, &object, &args)
                            .unwrap_or(Stmt::Call(Callee::Member(object, name), args)),
                    )
                }
            }
            Family::LateStore => {
                let (object, class) = state.object.clone().ok_or(LiftFault::NoObject(at))?;
                let dispid = u32_at(arguments, 0).ok_or_else(short)?;
                let name = dispid_member(types, class.as_deref(), dispid);
                let value = expressions(call_arguments(&mut state.stack, 16, at)?);
                Some(Stmt::Assign {
                    target: Expr::Member(Box::new(object), name),
                    value: value
                        .into_iter()
                        .next()
                        .ok_or(LiftFault::CallArguments(at))?,
                })
            }
            Family::GlobalObjectStore => {
                let value = pop(&mut state)?;
                Some(Stmt::Set {
                    target: global_at(callees, word16(0)?),
                    value,
                })
            }
            Family::VariantObjectRegister => {
                let value = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                state.object = Some((value.expr, value.class));
                None
            }
            Family::VariantBinary(op) => {
                let right = pop(&mut state)?;
                let left = pop(&mut state)?;
                state.stack.push(Value::plain(
                    Expr::Binary(op, Box::new(left), Box::new(right)),
                    true,
                ));
                None
            }
            Family::VariantCopy => {
                let source = state.load(offset16()?, true);
                let slot = i16_at(arguments, 2).ok_or_else(short)?;
                let copy = LiftedStmt {
                    offset: first,
                    also_at: state.leading.clone(),
                    stmt: Stmt::Assign {
                        target: Expr::frame(slot),
                        value: source.expr.clone(),
                    },
                };
                state.bind_pending(slot, source, out.len(), copy);
                None
            }
            Family::GlobalStore => {
                let value = pop(&mut state)?;
                Some(Stmt::Assign {
                    target: global_at(callees, word16(0)?),
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
            Family::BranchTrue => {
                let condition = pop(&mut state)?;
                Some(Stmt::IfGoTo {
                    condition,
                    target: offset16()?.cast_unsigned(),
                })
            }
            Family::Resume => Some(Stmt::Resume(match word16(0)? {
                0xFFFF => None,
                0xFFFE => Some(0),
                target => Some(target),
            })),
            Family::Branch => Some(Stmt::GoTo(offset16()?.cast_unsigned())),
            Family::End => Some(Stmt::End),
            Family::Exit { function } => Some(Stmt::Exit { function }),
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
                let callee =
                    Callee::Member(Box::new(Expr::Implicit), callees.procedure(method.index));
                let function = callees.is_function(method.index);
                call_or_function(&mut state, callee, args, function)
            }
            Family::ImportCall { result } => {
                let index = word16(0)?;
                let (callee, class) = match callees.import(index) {
                    Some((name, class)) => (
                        Callee::Member(Box::new(Expr::Word("VBA")), basic_name(name)),
                        class.map(str::to_owned),
                    ),
                    None => match callees.declare(index) {
                        Some(name) => (Callee::Declare(name.to_owned()), None),
                        None => (Callee::Import(index), None),
                    },
                };
                let mut values = call_arguments(&mut state.stack, word16(2)?, at)?;
                let result_slot = values
                    .first()
                    .and_then(|value| value.slot)
                    .filter(|slot| *slot < 0);
                if !result
                    && callees.variant_results.contains(&index)
                    && let Some(slot) = result_slot
                {
                    values.remove(0);
                    result_of(&mut state, slot, Expr::Call(callee, expressions(values)))
                } else if !result && callees.function_stubs.contains(&index) {
                    match callees.project_call(index) {
                        Some(ProjectCall::Module(module, name)) => {
                            let callee = Callee::Module(module.clone(), name.clone());
                            match result_slot {
                                Some(slot) => {
                                    values.remove(0);
                                    result_of(
                                        &mut state,
                                        slot,
                                        Expr::Call(callee, expressions(values)),
                                    )
                                }
                                None => Some(Stmt::Call(callee, expressions(values))),
                            }
                        }
                        Some(ProjectCall::Object(name)) => {
                            if values.is_empty() {
                                return Err(LiftFault::CallArguments(at));
                            }
                            let object = values.remove(0).expr;
                            let callee = Callee::Member(Box::new(object), name.clone());
                            call_or_function(&mut state, callee, values, true)
                        }
                        None => Some(Stmt::Call(callee, expressions(values))),
                    }
                } else {
                    let mut args = expressions(values);
                    let callee = match callees.project_call(index) {
                        Some(ProjectCall::Module(module, name)) => {
                            Callee::Module(module.clone(), name.clone())
                        }
                        Some(ProjectCall::Object(name)) => {
                            if args.is_empty() {
                                return Err(LiftFault::CallArguments(at));
                            }
                            Callee::Member(Box::new(args.remove(0)), name.clone())
                        }
                        None => callee,
                    };
                    let float = names
                        .iter()
                        .any(|name| name.ends_with("FPR4") || name.ends_with("FPR8"));
                    if result {
                        let mut value = Value::plain(Expr::Call(callee, args), true);
                        value.class = class;
                        state.stack.push(value);
                        None
                    } else if float {
                        let begin = if !state.stack.is_empty()
                            && state.stack.iter().all(Value::is_float_call)
                        {
                            state.resume.unwrap_or(first)
                        } else {
                            first
                        };
                        let below = state
                            .stack
                            .iter()
                            .filter(|value| value.is_float_call())
                            .count();
                        state.floats.truncate(below);
                        state.floats.push(begin);
                        state.resume = next_offset;
                        state
                            .stack
                            .push(Value::plain(Expr::Call(callee, args), false));
                        None
                    } else {
                        Some(Stmt::Call(callee, args))
                    }
                }
            }
            Family::ObjectStore => {
                let value = state.stack.pop().ok_or(LiftFault::StackShort(at))?;
                let slot = offset16()?;
                if state.depth() == 0 {
                    let set = LiftedStmt {
                        offset: first,
                        also_at: state.leading.clone(),
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
                if !pushes
                    && let Some(index) = u16_at(arguments, 2)
                    && let Some((object, class @ None)) = state.object.as_mut()
                    && *object != Expr::Arg(8)
                {
                    *class = class_at(callees, index);
                }
                object_call(&mut state, callees, types, word16(0)?, pushes, at, calls)?
            }
            Family::GlobalLoad => {
                state.stack.push(Value::sized(
                    global_at(callees, word16(0)?),
                    load_bytes(names),
                ));
                None
            }
            Family::GlobalObjectRegister => {
                state.object = Some((global_at(callees, word16(0)?), None));
                None
            }
            Family::FieldLoad | Family::FieldObjectRegister => {
                let (base, _) = state.object.clone().ok_or(LiftFault::NoObject(at))?;
                let field = Expr::Field(Box::new(base), word16(0)?);
                if family == Family::FieldLoad {
                    state.stack.push(Value::sized(field, load_bytes(names)));
                } else {
                    state.object = Some((field, None));
                }
                None
            }
            Family::FrameFieldLoad | Family::FrameFieldObjectRegister => {
                let base = state.load(offset16()?, true).expr;
                let field = Expr::Field(Box::new(base), word16(2)?);
                if family == Family::FrameFieldLoad {
                    state.stack.push(Value::sized(field, load_bytes(names)));
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
                    also_at: state.leading.clone(),
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
            Family::LitNothing => {
                state.stack.push(Value::plain(Expr::Word("Nothing"), true));
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
                let expr = callees
                    .class(index)
                    .and_then(Callees::object_name)
                    .map_or(Expr::New(index), |name| Expr::Name(format!("New {name}")));
                let mut value = Value::plain(expr, true);
                value.class = class_at(callees, index);
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
                let object = match (&reference.expr, callees.class(index)) {
                    (Expr::Global(_) | Expr::Variable(_), Some(profile)) => profile
                        .form_name()
                        .map_or(reference.expr, |form| Expr::Name(form.to_owned())),
                    (Expr::Global(_) | Expr::Variable(_), None)
                        if class.as_deref() == Some(GLOBAL_INTERFACE) =>
                    {
                        Expr::Implicit
                    }
                    _ => reference.expr,
                };
                state.object = Some((object, class));
                None
            }
            Family::Nop => None,
            Family::FunctionOf(function, count) => {
                let mut args = Vec::new();
                for _ in 0..count {
                    args.push(pop(&mut state)?);
                }
                args.reverse();
                // `UBound` is a word of Basic, not a member of `VBA`.
                let owner = if function == "UBound" {
                    Expr::Implicit
                } else {
                    Expr::Word("VBA")
                };
                state.stack.push(Value::plain(
                    Expr::Call(Callee::Member(Box::new(owner), function.to_owned()), args),
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
            Family::FileRecord {
                name,
                record,
                sized,
            } => {
                if sized {
                    pop(&mut state)?;
                }
                let variable = pop(&mut state)?;
                let number = if record {
                    pop(&mut state)?
                } else {
                    Expr::Word("")
                };
                let file = pop(&mut state)?;
                Some(Stmt::Keyword(
                    Keyword::Record { name },
                    vec![file, number, variable],
                ))
            }
            Family::PrintFile => {
                let bytes = word16(2)?
                    .checked_sub(4)
                    .ok_or(LiftFault::CallArguments(at))?;
                let args = expressions(call_arguments(&mut state.stack, bytes, at)?);
                if args.len() != 2 {
                    return Err(LiftFault::CallArguments(at));
                }
                Some(Stmt::Keyword(Keyword::Print, args))
            }
            Family::Close => Some(Stmt::Keyword(Keyword::Close, vec![pop(&mut state)?])),
            Family::Open => {
                let length = pop(&mut state)?;
                let number = pop(&mut state)?;
                let file = pop(&mut state)?;
                let mode = match word16(0)? & 0xFF {
                    0x01 => Some("Input"),
                    0x02 => Some("Output"),
                    0x04 => Some("Random"),
                    0x08 => Some("Append"),
                    0x20 => Some("Binary"),
                    _ => None,
                };
                Some(Stmt::Keyword(
                    Keyword::Open { mode },
                    vec![file, number, length],
                ))
            }
            Family::Redim { preserve } => {
                let array = pop(&mut state)?;
                let mut bounds = Vec::new();
                for _ in 0..word16(0)?.saturating_mul(2) {
                    bounds.push(pop(&mut state)?);
                }
                bounds.reverse();
                let mut args = vec![array];
                args.extend(bounds);
                Some(Stmt::Keyword(Keyword::ReDim { preserve }, args))
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
            Family::ArrayErase => Some(Stmt::Keyword(Keyword::Erase, vec![pop(&mut state)?])),
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
                let (object, class) = state.object.clone().ok_or(LiftFault::NoObject(at))?;
                let slot = offset16()?;
                let dispid = arguments
                    .get(2..6)
                    .and_then(|bytes| bytes.try_into().ok())
                    .map(u32::from_le_bytes)
                    .ok_or_else(short)?;
                let expr = match named_dispid(types, class.as_deref(), dispid) {
                    Some(name) => Expr::Member(Box::new(object), name),
                    None => Expr::Late(Box::new(object), dispid),
                };
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
                let value = pop_value(&mut state)?;
                let target = Expr::Index(Box::new(array), vec![index]);
                // A value of a known class is an object, which Basic stores
                // with Set.
                Some(if value.class.is_some() {
                    Stmt::Set {
                        target,
                        value: value.expr,
                    }
                } else {
                    Stmt::Assign {
                        target,
                        value: value.expr,
                    }
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
        // A free or an opcode with no effect after a statement, or after a
        // call that stays on the floating point unit, does not start the
        // next statement. The next statement keeps its offset, which a
        // branch can name.
        if matches!(family, Family::Free | Family::Nop) && stmt.is_none() {
            if state.stack.is_empty() {
                state.leading.push(at);
                start = None;
            } else if state.resume == Some(at) {
                state.resumed.push(at);
                state.resume = next_offset;
            }
        }
        if let Some(stmt) = stmt {
            if !state.stack.iter().all(Value::is_float_call) {
                return Err(LiftFault::StackLeft(at));
            }
            let mut offset = first;
            let mut leading = std::mem::take(&mut state.leading);
            for (count, value) in std::mem::take(&mut state.stack).into_iter().enumerate() {
                if let Expr::Call(callee, args) = value.expr {
                    let begin = state.floats.get(count).copied().unwrap_or(first);
                    out.push(LiftedStmt {
                        offset: begin,
                        also_at: std::mem::take(&mut leading),
                        stmt: Stmt::Call(callee, args),
                    });
                    offset = state.resume.unwrap_or(first);
                }
            }
            state.floats.clear();
            state.resume = None;
            leading.append(&mut state.resumed);
            out.push(LiftedStmt {
                offset,
                also_at: leading,
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
    let last = listing.instructions.last().map_or(0, |last| last.offset);
    for target in branch_targets(&out) {
        let target = u32::from(target);
        let starts = out
            .iter()
            .any(|lifted| lifted.offset == target || lifted.also_at.contains(&target));
        if !starts && target < last {
            return Err(LiftFault::BranchTarget(target));
        }
    }
    Ok(out
        .into_iter()
        .map(|lifted| LiftedStmt {
            offset: lifted.offset,
            also_at: lifted.also_at,
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

/// Gives the index of the entry of the constant table that each
/// `VCallHresult` of `listing` names: the GUID of the interface of the
/// call. A caller gives [`Callees`] the class of each one, and the lift
/// uses it for an object whose class it does not know.
#[must_use]
pub fn interface_indexes(listing: &PcodeListing, table: &PcodeTable) -> Vec<u16> {
    let mut out = Vec::new();
    for instruction in &listing.instructions {
        let names = table
            .slot(instruction.lead, instruction.opcode)
            .map(|slot| slot.names.as_slice())
            .unwrap_or_default();
        if family(names) == Some(Family::ObjectCall { pushes: false })
            && let Some(index) = u16_at(&instruction.arguments, 2)
            && !out.contains(&index)
        {
            out.push(index);
        }
    }
    out
}

/// Gives the index of each entry of the constant table that an import call
/// of `listing` names. A caller gives [`Callees`] the function of the
/// runtime of each one.
#[must_use]
pub fn import_indexes(listing: &PcodeListing, table: &PcodeTable) -> Vec<u16> {
    let mut out = Vec::new();
    for instruction in &listing.instructions {
        let names = table
            .slot(instruction.lead, instruction.opcode)
            .map(|slot| slot.names.as_slice())
            .unwrap_or_default();
        if matches!(family(names), Some(Family::ImportCall { .. }))
            && let Some(index) = u16_at(&instruction.arguments, 0)
            && !out.contains(&index)
        {
            out.push(index);
        }
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
#[must_use]
pub fn result_bytes(listing: &PcodeListing, table: &PcodeTable) -> Option<u16> {
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

/// Gives the index of each variable of the constant table that `listing`
/// loads or stores. A caller gives [`Callees`] the address of each one.
#[must_use]
pub fn global_indexes(listing: &PcodeListing, table: &PcodeTable) -> Vec<u16> {
    let mut out = Vec::new();
    for instruction in &listing.instructions {
        let names = table
            .slot(instruction.lead, instruction.opcode)
            .map(|slot| slot.names.as_slice())
            .unwrap_or_default();
        if matches!(
            family(names),
            Some(
                Family::GlobalLoad
                    | Family::GlobalStore
                    | Family::GlobalObjectStore
                    | Family::GlobalObjectRegister
            )
        ) && let Some(index) = u16_at(&instruction.arguments, 0)
            && !out.contains(&index)
        {
            out.push(index);
        }
    }
    out
}

/// Gives the index of each member name of the constant table that a
/// late-bound call of `listing` names. A caller gives [`Callees`] each one.
#[must_use]
pub fn name_indexes(listing: &PcodeListing, table: &PcodeTable) -> Vec<u16> {
    let mut out = Vec::new();
    for instruction in &listing.instructions {
        let names = table
            .slot(instruction.lead, instruction.opcode)
            .map(|slot| slot.names.as_slice())
            .unwrap_or_default();
        let Some(Family::LateCall {
            result,
            by_id: false,
            ..
        }) = family(names)
        else {
            continue;
        };
        if let Some(index) = u16_at(&instruction.arguments, if result { 2 } else { 0 })
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
    let targets = branch_targets(stmts);
    let mut lines = Vec::new();
    let mut labelled = Vec::new();
    for lifted in stmts {
        for also in &lifted.also_at {
            if let Ok(offset) = u16::try_from(*also)
                && targets.contains(&offset)
                && !labelled.contains(&offset)
            {
                labelled.push(offset);
                lines.push(format!("L{offset:04X}:"));
            }
        }
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
        if !labelled.contains(&target) {
            lines.push(format!("L{target:04X}:"));
        }
    }
    lines
}

/// Gives the offset that each branch of `stmts` names, once each, in order.
fn branch_targets(stmts: &[LiftedStmt]) -> Vec<u16> {
    let mut targets: Vec<u16> = stmts
        .iter()
        .filter_map(|lifted| match lifted.stmt {
            Stmt::IfNotGoTo { target, .. }
            | Stmt::IfGoTo { target, .. }
            | Stmt::GoTo(target)
            | Stmt::For { exit: target, .. }
            | Stmt::Next { body: target, .. } => Some(target),
            Stmt::OnError(target) | Stmt::Resume(target) => target.filter(|target| *target != 0),
            Stmt::Assign { .. }
            | Stmt::Set { .. }
            | Stmt::End
            | Stmt::Exit { .. }
            | Stmt::Call(..)
            | Stmt::Keyword(..) => None,
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{
        Callees, Expr, Keyword, LiftFault, ProjectCall, Stmt, arguments_text, class_indexes,
        drawing, global_indexes, import_indexes, interface_indexes, lift, lift_method,
        method_calls, name_indexes, render, string_indexes,
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
[primary.3D]
width = 2
names = ["ImpAdStAdFunc"]
[primary.3E]
width = 0
names = ["LdPrVar"]
[primary.3F]
width = 2
names = ["ConcatVar"]
[primary.40]
width = 4
names = ["FDupVar"]
[primary.41]
width = 4
names = ["ForStepI2"]
[primary.42]
width = 2
names = ["CVarStr"]
[primary.43]
width = 2
names = ["FLdVar"]
[primary.46]
width = 0
names = ["CStrVarTmp"]
[primary.47]
width = 2
names = ["FnLenVar"]
[primary.48]
width = 0
names = ["CI4R4"]
[primary.49]
width = 2
names = ["FLdCy", "FLdR8"]
[primary.4A]
width = 2
names = ["Open"]
[primary.4B]
width = 8
names = ["RedimPreserve"]
[primary.4C]
width = 2
names = ["IStDarg"]
[primary.4D]
width = 2
names = ["CRec2Ansi"]
[primary.4E]
width = 0
names = ["GetRec4"]
[primary.4F]
width = 0
names = ["GetRec3"]
[primary.50]
width = 4
names = ["PrintFile"]
[primary.51]
width = 2
names = ["GetRecOwn3"]
[primary.52]
width = 4
names = ["DestructAnsiOFrame"]
[primary.53]
width = 4
names = ["ForVar"]
[primary.54]
width = 4
names = ["NextVar"]
[primary.55]
width = 0
names = ["Close"]
[primary.56]
width = 2
names = ["AddVar"]
[primary.57]
width = 0
names = ["LitNothing"]
[primary.58]
width = 2
names = ["VCall"]
[primary.59]
width = 6
names = ["LateIdCall"]
[primary.5A]
width = 4
names = ["LateIdSt"]
[primary.5B]
width = 2
names = ["BranchT"]
[primary.5C]
width = 2
names = ["Resume"]
[primary.5D]
width = 4
names = ["ExitProcCbHresult"]
[primary.44]
width = 4
names = ["LateMemCall"]
[primary.45]
width = 6
names = ["LateMemCallLdVar"]
[primary.5E]
width = 0
names = ["FnUBound"]
[primary.5F]
width = 4
names = ["LitVarI2"]
[primary.60]
width = 0
names = ["PopAdLdVar"]
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
[interfaces._Box.functions.00B8]
names = ["Selected"]
kinds = ["let"]
arg_bytes = 8
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
dispid = 67
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
            ["       local_88 = Me.box1.Text", "       Exit Sub"]
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
                "       Exit Sub"
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
            ["       local_88 = Me.box1.Item(0).Text", "       Exit Sub"]
        );
        // An array of controls whose interface is not known: Item gives a
        // control with no class, and the call on it names entry 5, the
        // interface _Box.
        let untyped = [
            0x15, 0x9C, 0xFF, 0x02, 0x00, 0x16, 0x1A, 0x2C, 0x03, 0x1B, 0x98, 0xFF, 0x06, 0x98,
            0xFF, 0x1C, 0x40, 0x00, 0x00, 0x00, 0x06, 0x9C, 0xFF, 0x1C, 0xB0, 0x00, 0x05, 0x00,
            0x18, 0x98, 0xFF, 0x0C,
        ];
        let listing_untyped = disassemble(&Region::new(&untyped, Off::new(0)), &table);
        let array = Callees::default()
            .with_control(0x32C, "wsk", "[]")
            .with_class_interface(5, "_Box");
        assert_eq!(
            render(&lift(&listing_untyped, &table, &array, Some(&types)).unwrap()),
            ["       Call Me.wsk.Item(0).Cls()", "       Exit Sub"]
        );
    }

    #[test]
    fn a_named_import_gives_its_name_and_the_class_of_its_result() {
        // Set local_1C = Err(): Err.Cls through VCall, which names no
        // interface.
        let body = [
            0x13, 0x02, 0x00, 0x00, 0x00, 0x1B, 0xE4, 0xFF, 0x06, 0xE4, 0xFF, 0x58, 0xB0, 0x00,
            0x0C,
        ];
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let types = VbTypes::parse(TYPES.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        assert_eq!(import_indexes(&listing, &table), [2]);
        let named = Callees::default().with_import(2, "Err", Some("_Box"));
        assert_eq!(
            render(&lift(&listing, &table, &named, Some(&types)).unwrap()),
            [
                "       Set local_1C = VBA.Err()",
                "       Call local_1C.Cls()",
                "       Exit Sub"
            ]
        );
        assert_eq!(
            lift(&listing, &table, &Callees::default(), Some(&types)),
            Err(LiftFault::NoFunction(11))
        );
        // Beep, a named call whose result nothing takes, then a store.
        let beep = [
            0x12, 0x02, 0x00, 0x00, 0x00, 0x0F, 0x9C, 0xFF, 0x05, 0x78, 0xFF, 0x0C,
        ];
        let listing_beep = disassemble(&Region::new(&beep, Off::new(0)), &table);
        let named_beep = Callees::default().with_import(2, "Beep", None);
        assert_eq!(
            render(&lift(&listing_beep, &table, &named_beep, None).unwrap()),
            [
                "       Call VBA.Beep()",
                "       local_88 = local_64",
                "       Exit Sub"
            ]
        );
        // A call of a procedure of an object needs the object.
        let object =
            Callees::default().with_project_call(2, ProjectCall::Object("Save".to_owned()));
        assert_eq!(
            lift(&listing_beep, &table, &object, None),
            Err(LiftFault::CallArguments(0))
        );
        // The same call of a procedure of a DLL gives its export name.
        let declared = Callees::default()
            .with_declare(1, "GetObjectA")
            .with_declare(2, "Beep");
        assert_eq!(
            render(&lift(&listing_beep, &table, &declared, None).unwrap()),
            [
                "       Call Beep()",
                "       local_88 = local_64",
                "       Exit Sub"
            ]
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
                "       Exit Sub"
            ]
        );
    }

    #[test]
    fn the_accessor_of_an_untyped_control_gives_the_control_with_no_class() {
        // Set local_68 = Me.wsk1; the call on it names no interface that
        // the lift knows.
        let body = [0x16, 0x1A, 0x2C, 0x03, 0x1B, 0x98, 0xFF, 0x0C];
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        let callees = Callees::default().with_untyped_control(0x32C, "wsk1");
        assert_eq!(callees.control(0x32C), Some(("wsk1", None)));
        assert_eq!(
            render(&lift(&listing, &table, &callees, None).unwrap()),
            ["       Set local_68 = Me.wsk1", "       Exit Sub"]
        );
    }

    #[test]
    fn a_vcall_calls_through_the_object_register_with_no_interface_argument() {
        let mut body = BOX1_IN_LOCAL_68.to_vec();
        body.extend_from_slice(&[0xFF, 0x58, 0xB0, 0x00, 0x18, 0x98, 0xFF, 0x0C]);
        assert_eq!(
            object_lines(&body).unwrap(),
            ["       Call Me.box1.Cls()", "       Exit Sub"]
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
                "       Exit Sub"
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
        // Call method_5(Me.box1, 1): the control is passed by the address
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
        // The let of Selected with an index: Basic pushes the value 7
        // first and the index 3 last, so the index is the first argument.
        let indexed = [
            0x02, 0x07, 0x02, 0x03, 0x06, 0x08, 0x00, 0x1C, 0xB8, 0x00, 0x00, 0x00, 0x0C,
        ];
        let listing = disassemble(&Region::new(&indexed, Off::new(0)), &table);
        assert_eq!(
            render(&lift(&listing, &table, &callees, Some(&types)).unwrap())[0],
            "       Me.Selected(3) = 7"
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
            ["       Call method_5(arg_C, local_88)", "       Exit Sub"]
        );
        // A public procedure gives its name.
        let named = Callees::default()
            .with_method(0x6F8, 5, 12)
            .with_procedure(4, "Other")
            .with_procedure(5, "SaveFile");
        assert_eq!(
            lines_with(&body, &named).unwrap(),
            ["       Call SaveFile(arg_C, local_88)", "       Exit Sub"]
        );
        let fewer = Callees::default().with_method(0x6F8, 5, 8);
        assert_eq!(lines_with(&body, &fewer), Err(LiftFault::StackLeft(6)));
        // The same call of a Function assigns its result to local_88.
        let function = Callees::default()
            .with_method(0x6F8, 5, 12)
            .with_function(5);
        assert_eq!(
            lines_with(&body, &function).unwrap(),
            ["       local_88 = method_5(arg_C)", "       Exit Sub"]
        );
    }

    #[test]
    fn a_call_of_a_function_of_another_module_or_object_assigns_its_result() {
        // Push the address of local_88, then arg_C, then call stub 1.
        let body = [
            0x15, 0x78, 0xFF, 0x03, 0x0C, 0x00, 0x12, 0x01, 0x00, 0x08, 0x00, 0x0C,
        ];
        let module = ProjectCall::Module("Module1".to_owned(), "Draw".to_owned());
        let sub = Callees::default().with_project_call(1, module.clone());
        assert_eq!(
            lines_with(&body, &sub).unwrap()[0],
            "       Call Module1.Draw(arg_C, local_88)"
        );
        // A Function of a module takes the result first: here arg_C, which
        // is not a local, so the call stays a call.
        let function = Callees::default()
            .with_project_call(1, module)
            .with_function_stub(1);
        assert_eq!(
            lines_with(&body, &function).unwrap()[0],
            "       Call Module1.Draw(arg_C, local_88)"
        );
        // A Function of an object takes the object first and the result last.
        let object = Callees::default()
            .with_project_call(1, ProjectCall::Object("Save".to_owned()))
            .with_function_stub(1);
        assert_eq!(
            lines_with(&body, &object).unwrap()[0],
            "       local_88 = arg_C.Save()"
        );
        // Push arg_C, then the address of local_88: the result is first.
        let first = [
            0x03, 0x0C, 0x00, 0x15, 0x78, 0xFF, 0x12, 0x01, 0x00, 0x08, 0x00, 0x0C,
        ];
        let module = Callees::default()
            .with_project_call(
                1,
                ProjectCall::Module("Module1".to_owned(), "Draw".to_owned()),
            )
            .with_function_stub(1);
        assert_eq!(
            lines_with(&first, &module).unwrap()[0],
            "       local_88 = Module1.Draw(arg_C)"
        );
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
                "       Exit Sub"
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
                "L000F: Exit Sub",
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
                "       field_54 = 0",
                "       field_40 = -1",
                "       Exit Sub"
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
                "       field_54 = 0",
                "       arg_C.field_40 = 1",
                "       Exit Sub"
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
            ["       local_88 = 1", "       Exit Sub"]
        );
    }

    #[test]
    fn globals_fields_and_strings_give_their_expressions() {
        // local_88 = field_54; local_68.field_10 = local_88.field_C;
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
                "       local_88 = field_54",
                "       local_68.field_10 = local_88.field_C",
                "       local_88 = \"x\"",
                "       Exit Sub"
            ]
        );
    }

    #[test]
    fn a_function_that_returns_a_variant_takes_its_result_first() {
        // Push 7, then the address of local_88, then call import 1: the
        // value pushed last is the first argument.
        let body = [
            0x02, 0x07, 0x15, 0x78, 0xFF, 0x12, 0x01, 0x00, 0x08, 0x00, 0x0C,
        ];
        let plain = Callees::default().with_import(1, "_B_var_Left", None);
        assert_eq!(
            lines_with(&body, &plain).unwrap()[0],
            "       Call VBA.Left(local_88, 7)"
        );
        let variant = plain.with_variant_result(1);
        assert_eq!(
            lines_with(&body, &variant).unwrap()[0],
            "       local_88 = VBA.Left(7)"
        );
        let string = Callees::default().with_import(1, "_B_str_Left", None);
        assert_eq!(
            lines_with(&body, &string).unwrap()[0],
            "       Call VBA.Left$(local_88, 7)"
        );
    }

    #[test]
    fn a_branch_names_the_statement_after_the_opcodes_with_no_effect() {
        // If Not (local_88 = 7) Then GoTo <target>; local_8C = 1; then an
        // opcode with no effect at 0x14; local_90 = 2 at 0x16; Exit Sub.
        let body = |target: u8, between: &[u8]| {
            let mut out = vec![
                0x03, 0x78, 0xFF, 0x01, 7, 0, 0, 0, 0x09, 0x0A, target, 0x00, 0x01, 1, 0, 0, 0,
                0x05, 0x74, 0xFF,
            ];
            out.extend_from_slice(between);
            out.extend_from_slice(&[0x01, 2, 0, 0, 0, 0x05, 0x70, 0xFF, 0x0C]);
            out
        };
        // Bos at 0x14 does not start the statement at 0x16.
        let bos = [0x29, 0x00];
        assert_eq!(
            lines(&body(0x16, &bos)).unwrap(),
            [
                "       If Not (local_88 = 7) Then GoTo L0016",
                "       local_8C = 1",
                "L0016: local_90 = 2",
                "       Exit Sub"
            ]
        );
        // A branch to the Bos names the same statement, with a label of its
        // own.
        assert_eq!(
            lines(&body(0x14, &bos)).unwrap(),
            [
                "       If Not (local_88 = 7) Then GoTo L0014",
                "       local_8C = 1",
                "L0014:",
                "       local_90 = 2",
                "       Exit Sub"
            ]
        );
        // A free at 0x14 does not start it either.
        let free = [0x18, 0x6C, 0xFF];
        assert_eq!(lines(&body(0x17, &free)).unwrap()[2], "L0017: local_90 = 2");
        // A branch into a statement, where none starts, is a fault: a label
        // there would move the branch.
        assert_eq!(lines(&body(0x0E, &bos)), Err(LiftFault::BranchTarget(0x0E)));
    }

    #[test]
    fn the_field_at_zero_of_an_array_element_is_the_element() {
        let element = Expr::Index(
            Box::new(Expr::Local(0x88)),
            vec![Expr::Const(1), Expr::Const(2)],
        );
        assert_eq!(
            Expr::Field(Box::new(element.clone()), 0).text(),
            "local_88(1, 2)"
        );
        assert_eq!(
            Expr::Field(Box::new(element), 4).text(),
            "local_88(1, 2).field_4"
        );
        assert_eq!(
            Expr::Field(Box::new(Expr::Local(0x88)), 0).text(),
            "local_88.field_0"
        );
    }

    #[test]
    fn a_keyword_statement_has_the_form_of_basic() {
        let one = Expr::Const(1);
        let path = Expr::Str("a.map".to_owned());
        let text = |keyword: Keyword, args: &[Expr]| Stmt::Keyword(keyword, args.to_vec()).text();
        assert_eq!(
            text(
                Keyword::Open {
                    mode: Some("Binary")
                },
                &[path.clone(), one.clone(), Expr::Const(-1)]
            ),
            "Open \"a.map\" For Binary As #1"
        );
        assert_eq!(
            text(
                Keyword::Open { mode: None },
                &[path, one.clone(), Expr::Const(64)]
            ),
            "Open \"a.map\" As #1 Len = 64"
        );
        assert_eq!(text(Keyword::Close, std::slice::from_ref(&one)), "Close #1");
        assert_eq!(
            text(Keyword::Print, &[one.clone(), Expr::Local(0xC0)]),
            "Print #1, local_C0"
        );
        assert_eq!(
            text(
                Keyword::Record { name: "Get" },
                &[one.clone(), Expr::Word(""), Expr::Local(0x88)]
            ),
            "Get #1, , local_88"
        );
        assert_eq!(
            text(
                Keyword::Record { name: "Put" },
                &[one.clone(), one.clone(), Expr::Local(0x88)]
            ),
            "Put #1, 1, local_88"
        );
        assert_eq!(
            text(
                Keyword::ReDim { preserve: true },
                &[Expr::Local(0x88), Expr::Const(0), one]
            ),
            "ReDim Preserve local_88(0 To 1)"
        );
        assert_eq!(text(Keyword::Erase, &[Expr::Arg(0xC)]), "Erase arg_C");
    }

    #[test]
    fn a_drawing_call_has_the_form_of_basic() {
        let object = Expr::Name("pic".to_owned());
        let n = |value: i64| Expr::Const(value);
        let text = |name: &str, args: &[Expr]| drawing(name, &object, args).map(|stmt| stmt.text());
        assert_eq!(
            text("Line", &[n(4), n(1), n(2), n(3), n(5), n(0)]).unwrap(),
            "pic.Line (1, 2)-(3, 5)"
        );
        assert_eq!(
            text("Line", &[n(6), n(1), n(2), n(3), n(5), n(9)]).unwrap(),
            "pic.Line (1, 2)-(3, 5), 9"
        );
        assert_eq!(
            text("Line", &[n(38), n(1), n(2), n(3), n(5), n(9)]).unwrap(),
            "pic.Line (1, 2)-(3, 5), 9, BF"
        );
        assert_eq!(
            text("Circle", &[n(2), n(1), n(2), n(4), n(9), n(0), n(0), n(0)]).unwrap(),
            "pic.Circle (1, 2), 4, 9"
        );
        assert_eq!(
            text("PSet", &[n(2), n(1), n(2), n(9)]).unwrap(),
            "pic.PSet (1, 2), 9"
        );
        // A flag that the corpus does not show stays a call.
        assert_eq!(text("Line", &[n(5), n(1), n(2), n(3), n(5), n(0)]), None);
        assert_eq!(
            text("Circle", &[n(2), n(1), n(2), n(4), n(9), n(1), n(0), n(0)]),
            None
        );
    }

    #[test]
    fn a_second_name_of_a_procedure_replaces_the_first() {
        let callees = Callees::default()
            .with_procedure(2, "First")
            .with_procedure(3, "Other")
            .with_procedure(2, "Second");
        assert_eq!(callees.procedure(2), "Second");
        assert_eq!(callees.procedure(3), "Other");
        assert_eq!(callees.procedure(4), "method_4");
    }

    #[test]
    fn an_exit_that_returns_a_value_exits_a_function() {
        assert_eq!(lines(&[0x0C]).unwrap(), ["       Exit Sub"]);
        assert_eq!(
            lines(&[0x5D, 0x00, 0x00, 0x00, 0x00]).unwrap(),
            ["       Exit Function"]
        );
    }

    #[test]
    fn a_control_character_of_a_string_is_a_constant() {
        let text = |value: &str| Expr::Str(value.to_owned()).text();
        assert_eq!(text("a\r\nb"), "\"a\" & vbCrLf & \"b\"");
        assert_eq!(text("\r\n"), "vbCrLf");
        assert_eq!(text("x\ty\n"), "\"x\" & vbTab & \"y\" & vbLf");
        assert_eq!(text("\r"), "vbCr");
        assert_eq!(text("\0"), "vbNullChar");
        assert_eq!(text("\u{1}q"), "Chr$(1) & \"q\"");
        assert_eq!(text("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(text(""), "\"\"");
    }

    #[test]
    fn a_missing_argument_is_empty_and_is_dropped_at_the_end() {
        let missing = Expr::Word("Missing");
        assert_eq!(
            arguments_text(&[
                missing.clone(),
                Expr::Const(1),
                missing.clone(),
                missing.clone()
            ]),
            ", 1"
        );
        assert_eq!(arguments_text(&[missing.clone(), missing]), "");
        assert_eq!(arguments_text(&[Expr::Const(1), Expr::Const(2)]), "1, 2");
    }

    #[test]
    fn literals_give_their_constants() {
        // local_A0 = 2.5, then Call import_1(, 7, New class_4), with a
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
                "       Call import_1(, 7, New class_4)",
                "       Exit Sub"
            ]
        );
        // The same call of a procedure of a module, and of an object, whose
        // first argument is the object.
        let module = Callees::default()
            .with_project_call(0, ProjectCall::Object("Other".to_owned()))
            .with_project_call(
                1,
                ProjectCall::Module("Module1".to_owned(), "Draw".to_owned()),
            );
        assert_eq!(
            lines_with(&body, &module).unwrap()[1],
            "       Call Module1.Draw(, 7, New class_4)"
        );
        let object =
            Callees::default().with_project_call(1, ProjectCall::Object("Save".to_owned()));
        assert_eq!(
            lines_with(&body, &object).unwrap()[1],
            "       Call Missing.Save(7, New class_4)"
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
                "       Exit Sub"
            ]
        );
        let freed = [
            0x03, 0x0C, 0x00, 0x25, 0x78, 0xFF, 0x12, 0x02, 0x00, 0x04, 0x00, 0x18, 0x78, 0xFF,
            0x0C,
        ];
        assert_eq!(
            lines(&freed).unwrap(),
            ["       Call import_2(arg_C)", "       Exit Sub"]
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
            ["       global_3.field_40 = 1", "       Exit Sub"]
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
                "L0022: Exit Sub",
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
                "       Exit Sub"
            ]
        );
        assert_eq!(class_indexes(&listing, &table), [9]);
        let named = Callees::default().with_class(
            9,
            Callees::default()
                .with_method(0x6F8, 2, 8)
                .with_procedure(2, "GetImageWidth")
                .with_variable(0x700, 0x34, true)
                .with_variable(0x704, 0x34, false),
        );
        assert_eq!(
            render(&lift(&listing, &table, &named, None).unwrap()),
            [
                "       global_3.field_34 = 5",
                "       Call global_3.GetImageWidth(global_3.field_34)",
                "       Exit Sub"
            ]
        );
        // A global of the class of a form holds the instance of the form.
        let form = Callees::default().with_class(
            9,
            Callees::default()
                .with_form_name("frmMain")
                .with_method(0x6F8, 2, 8)
                .with_variable(0x700, 0x34, true)
                .with_variable(0x704, 0x34, false),
        );
        assert_eq!(
            render(&lift(&listing, &table, &form, None).unwrap()),
            [
                "       frmMain.field_34 = 5",
                "       Call frmMain.method_2(frmMain.field_34)",
                "       Exit Sub"
            ]
        );
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
        // If Not (0 = New class_9.method_2()): a method of the project with
        // a value under its arguments is a Function, whose last argument
        // takes the result.
        let function = [
            0x01, 0x00, 0x00, 0x00, 0x00, 0x15, 0xB4, 0xFF, 0x27, 0x09, 0x00, 0x1B, 0xE4, 0xFF,
            0x06, 0xE4, 0xFF, 0x1C, 0xF8, 0x06, 0x00, 0x00, 0x03, 0xB4, 0xFF, 0x09, 0x0A, 0x1E,
            0x00, 0x0C,
        ];
        let listing_function = disassemble(&Region::new(&function, Off::new(0)), &table);
        assert_eq!(
            render(&lift(&listing_function, &table, &owned, None).unwrap()),
            [
                "       If Not (0 = New class_9.method_2()) Then GoTo L001E",
                "       Exit Sub",
                "L001E:"
            ]
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
        // The same get on the global object of the runtime has no object.
        let global = VbTypes::parse(
            TYPES
                .replace("[interfaces._Box", "[interfaces.VBGlobal")
                .as_bytes(),
        )
        .unwrap();
        let implicit = Callees::default().with_class_interface(9, "VBGlobal");
        assert_eq!(
            render(&lift(&listing_get, &table, &implicit, Some(&global)).unwrap())[0],
            "       local_88 = Text"
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
                "       Exit Sub"
            ]
        );
        // local_78 has no known class, and the call names entry 5, the
        // interface _Box.
        let named = [
            0x15, 0x9C, 0xFF, 0x06, 0x88, 0xFF, 0x1C, 0xA8, 0x00, 0x05, 0x00, 0x03, 0x9C, 0xFF,
            0x05, 0x78, 0xFF, 0x0C,
        ];
        let listing_named = disassemble(&Region::new(&named, Off::new(0)), &table);
        assert_eq!(interface_indexes(&listing_named, &table), [5]);
        let by_call = Callees::default().with_class_interface(5, "_Box");
        assert_eq!(
            render(&lift(&listing_named, &table, &by_call, Some(&types)).unwrap())[0],
            "       local_88 = local_78.Text"
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
        // writes local_4C and pushes its address, which PopAd drops. DISPID 0
        // is the default member, which Basic does not name.
        let body = [
            0x16, 0x33, 0xB4, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x34, 0x01, 1, 0, 0, 0, 0x03, 0xB4,
            0xFF, 0x02, 0x02, 0x15, 0x0C, 0x00, 0x35, 0x01, 0, 0, 0, 0, 0x37, 0x05, 0x78, 0xFF,
            0x0C,
        ];
        assert_eq!(
            lines(&body).unwrap()[0],
            "       local_88 = VBA.InStr(1, Me, arg_C(2), 0)"
        );
        // local_88 = UBound(arg_C, 1): Basic has no VBA.UBound.
        let bound = [0x03, 0x0C, 0x00, 0x02, 0x01, 0x5E, 0x05, 0x78, 0xFF, 0x0C];
        assert_eq!(
            lines(&bound).unwrap()[0],
            "       local_88 = UBound(arg_C, 1)"
        );
        // arg_C(1) = 7
        let store = [0x02, 0x07, 0x02, 0x01, 0x15, 0x0C, 0x00, 0x36, 0x0C];
        assert_eq!(lines(&store).unwrap()[0], "       arg_C(1) = 7");
        // Set arg_C(1) = New class_9: an object of a known class.
        let object = [0x27, 0x09, 0x00, 0x02, 0x01, 0x15, 0x0C, 0x00, 0x36, 0x0C];
        let class = Callees::default().with_class(9, Callees::default());
        assert_eq!(
            lines_with(&object, &class).unwrap()[0],
            "       Set arg_C(1) = New class_9"
        );
        assert_eq!(lines(&object).unwrap()[0], "       arg_C(1) = New class_9");
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
            ["       Call import_2(arg_C, 1)", "       Exit Sub"]
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
                "       Exit Sub"
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
                "       Erase arg_C",
                "       local_88 = arg_10",
                "       local_90 = Abs(arg_14)",
                "       Exit Sub"
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
            "       ReDim arg_C(0 To 7, 1 To 3)"
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
        // Two calls whose results nothing takes, and a GoTo to the
        // second: each call is a statement at its own offset.
        let calls = [
            0x12, 0x02, 0x00, 0x00, 0x00, 0x12, 0x03, 0x00, 0x00, 0x00, 0x0B, 0x05, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&calls).unwrap(),
            [
                "       Call import_2()",
                "L0005: Call import_3()",
                "       GoTo L0005",
                "       Exit Sub"
            ]
        );
        // The argument of the second call comes before the first call, and
        // the pop steps over the result of the first, which is not on the
        // stack.
        let over = [
            0x0F, 0x9C, 0xFF, 0x12, 0x02, 0x00, 0x00, 0x00, 0x12, 0x03, 0x00, 0x04, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&over).unwrap(),
            [
                "       Call import_2()",
                "       Call import_3(local_64)",
                "       Exit Sub"
            ]
        );
        // An operator that takes no float steps over such a call:
        // import_2(local_64), whose result goes into its argument, then
        // local_64 & local_60.
        let concat = [
            0x15, 0xA0, 0xFF, 0x15, 0x9C, 0xFF, 0x12, 0x02, 0x00, 0x04, 0x00, 0x15, 0x9C, 0xFF,
            0x3F, 0x90, 0xFF, 0x05, 0x78, 0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&concat).unwrap(),
            [
                "       Call import_2(local_64)",
                "       local_88 = (local_60 & local_64)",
                "       Exit Sub"
            ]
        );
        // CI4R4 takes a float, so it takes the result of the call.
        let convert = [0x12, 0x02, 0x00, 0x00, 0x00, 0x48, 0x05, 0x78, 0xFF, 0x0C];
        assert_eq!(
            lines(&convert).unwrap()[0],
            "       local_88 = CLng(import_2())"
        );
        // A store after such a call comes after it.
        let store = [
            0x12, 0x02, 0x00, 0x00, 0x00, 0x0F, 0x9C, 0xFF, 0x05, 0x78, 0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&store).unwrap(),
            [
                "       Call import_2()",
                "       local_88 = local_64",
                "       Exit Sub"
            ]
        );
        // A Set after such a call comes after it too.
        let set = [
            0x12, 0x02, 0x00, 0x00, 0x00, 0x27, 0x09, 0x00, 0x1B, 0xE4, 0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&set).unwrap(),
            [
                "       Call import_2()",
                "       Set local_1C = New class_9",
                "       Exit Sub"
            ]
        );
        // The statement after such a call keeps its own offset.
        let after = [0x12, 0x02, 0x00, 0x00, 0x00, 0x0B, 0x05, 0x00, 0x0C];
        assert_eq!(
            lines(&after).unwrap(),
            [
                "       Call import_2()",
                "L0005: GoTo L0005",
                "       Exit Sub"
            ]
        );
    }

    #[test]
    fn variants_a_global_object_and_a_step_of_integers_give_statements() {
        // Set global_5 = New class_9
        let set = [0x27, 0x09, 0x00, 0x3D, 0x05, 0x00, 0x0C];
        assert_eq!(lines(&set).unwrap()[0], "       Set global_5 = New class_9");
        // The same store to a variable whose address the constant table
        // gives.
        assert_eq!(
            lines_with(&set, &Callees::default().with_global(5, 0x0040_A1C0)).unwrap()[0],
            "       Set g_40A1C0 = New class_9"
        );
        // New of a class of the project writes the name of its object.
        let named = Callees::default().with_class(9, Callees::default().with_object_name("cImage"));
        assert_eq!(
            lines_with(&set, &named).unwrap()[0],
            "       Set global_5 = New cImage"
        );
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(&set, Off::new(0)), &table);
        assert_eq!(global_indexes(&listing, &table), [5]);
        // local_68 = local_64; local_88 = local_68 & local_60;
        // local_88 = local_78.field_34
        let variants = [
            0x40, 0x9C, 0xFF, 0x98, 0xFF, 0x15, 0x98, 0xFF, 0x15, 0xA0, 0xFF, 0x3F, 0x90, 0xFF,
            0x05, 0x78, 0xFF, 0x15, 0x88, 0xFF, 0x3E, 0x1E, 0x34, 0x00, 0x05, 0x78, 0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&variants).unwrap(),
            [
                "       local_68 = local_64",
                "       local_88 = (local_68 & local_60)",
                "       local_88 = local_78.field_34",
                "       Exit Sub"
            ]
        );
        // import_2(CVar(local_64)): CVarStr pushes the address of its slot.
        let variant = [
            0x0F, 0x9C, 0xFF, 0x42, 0x90, 0xFF, 0x12, 0x02, 0x00, 0x04, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&variant).unwrap()[0],
            "       Call import_2(CVar(local_64))"
        );
        // import_2(local_64): FLdVar pushes the 16 bytes of the Variant.
        let copy = [0x43, 0x9C, 0xFF, 0x12, 0x02, 0x00, 0x10, 0x00, 0x0C];
        assert_eq!(lines(&copy).unwrap()[0], "       Call import_2(local_64)");
        // import_2(CStr(local_64), Len(local_60))
        let text = [
            0x15, 0xA0, 0xFF, 0x47, 0x90, 0xFF, 0x15, 0x9C, 0xFF, 0x46, 0x12, 0x02, 0x00, 0x08,
            0x00, 0x0C,
        ];
        assert_eq!(
            lines(&text).unwrap()[0],
            "       Call import_2(CStr(local_64), Len(local_60))"
        );
        // import_2(local_64): FLdR8 pushes the 8 bytes of a Double.
        let double = [0x49, 0x9C, 0xFF, 0x12, 0x02, 0x00, 0x08, 0x00, 0x0C];
        assert_eq!(lines(&double).unwrap()[0], "       Call import_2(local_64)");
        // ReDim arg_10(0 To method_2(arg_C)): a call with a value under
        // its arguments is a Function, whose last argument is the local that
        // takes the result.
        let function = [
            0x01, 0x00, 0x00, 0x00, 0x00, 0x15, 0xB4, 0xFF, 0x0F, 0x0C, 0x00, 0x11, 0x24, 0x00,
            0x06, 0x00, 0x0F, 0xB4, 0xFF, 0x15, 0x10, 0x00, 0x3B, 0x01, 0x00, 0x11, 0x00, 0x01,
            0x00, 0x80, 0x00, 0x0C,
        ];
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(&function, Off::new(0)), &table);
        let callees = Callees::default().with_method(0x24, 2, 12);
        assert_eq!(
            render(&lift(&listing, &table, &callees, None).unwrap())[0],
            "       ReDim arg_10(0 To method_2(arg_C))"
        );
        // Open "a" For Binary As #1, with the length -1 of no Len clause.
        let open = [
            0x21, 0x04, 0x00, 0x02, 0x01, 0x02, 0xFF, 0x4A, 0x20, 0x00, 0x0C,
        ];
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let listing = disassemble(&Region::new(&open, Off::new(0)), &table);
        let callees = Callees::default().with_string(4, "a");
        assert_eq!(
            render(&lift(&listing, &table, &callees, None).unwrap())[0],
            "       Open \"a\" For Binary As #1"
        );
        // ReDim Preserve arg_C(0 To 7)
        let preserve = [
            0x02, 0x00, 0x02, 0x07, 0x15, 0x0C, 0x00, 0x4B, 0x01, 0x00, 0x11, 0x00, 0x01, 0x00,
            0x80, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&preserve).unwrap()[0],
            "       ReDim Preserve arg_C(0 To 7)"
        );
        // arg_C = 5; then import_2(local_54) through an ANSI copy of the
        // record into local_88.
        let darg = [
            0x02, 0x05, 0x4C, 0x0C, 0x00, 0x15, 0xAC, 0xFF, 0x15, 0x78, 0xFF, 0x4D, 0x03, 0x00,
            0x15, 0x78, 0xFF, 0x12, 0x02, 0x00, 0x04, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&darg).unwrap(),
            [
                "       arg_C = 5",
                "       Call import_2(local_54)",
                "       Exit Sub"
            ]
        );
        // Get #1, 1, local_64; Get #1, , local_60; Print #1, local_5C
        let file = [
            0x02, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x15, 0x9C, 0xFF, 0x01, 0x02, 0x00, 0x00,
            0x00, 0x4E, 0x02, 0x01, 0x15, 0xA0, 0xFF, 0x01, 0x02, 0x00, 0x00, 0x00, 0x4F, 0x0F,
            0xA4, 0xFF, 0x02, 0x01, 0x50, 0x29, 0x00, 0x0C, 0x00, 0x0C,
        ];
        assert_eq!(
            lines(&file).unwrap(),
            [
                "       Get #1, 1, local_64",
                "       Get #1, , local_60",
                "       Print #1, local_5C",
                "       Exit Sub"
            ]
        );
        // Get #1, , local_64 of a record or an array.
        let own = [0x02, 0x01, 0x15, 0x9C, 0xFF, 0x51, 0x0D, 0x00, 0x0C];
        assert_eq!(lines(&own).unwrap()[0], "       Get #1, , local_64");
        // For local_64 = 1 To 3: local_88 = local_60 + local_64: Next;
        // Close #1
        let loop_of_variants = [
            0x02, 0x01, 0x15, 0x9C, 0xFF, 0x02, 0x03, 0x53, 0xEC, 0xFE, 0x20, 0x00, 0x15, 0xA0,
            0xFF, 0x15, 0x9C, 0xFF, 0x56, 0x90, 0xFF, 0x05, 0x78, 0xFF, 0x15, 0x9C, 0xFF, 0x54,
            0xEC, 0xFE, 0x0C, 0x00, 0x02, 0x01, 0x55, 0x0C,
        ];
        assert_eq!(
            lines(&loop_of_variants).unwrap(),
            [
                "       For local_64 = 1 To 3  ' past the end: GoTo L0020",
                "L000C: local_88 = (local_60 + local_64)",
                "       Next local_64  ' loop: GoTo L000C",
                "L0020: Close #1",
                "       Exit Sub"
            ]
        );
        // import_2(Nothing)
        let nothing = [0x57, 0x12, 0x02, 0x00, 0x04, 0x00, 0x0C];
        assert_eq!(lines(&nothing).unwrap()[0], "       Call import_2(Nothing)");
        // If local_64 Then GoTo L000C; Resume Next; Resume L0006; Resume
        let resume = [
            0x0F, 0x9C, 0xFF, 0x5B, 0x0C, 0x00, 0x5C, 0xFF, 0xFF, 0x5C, 0x06, 0x00, 0x5C, 0xFE,
            0xFF, 0x0C,
        ];
        assert_eq!(
            lines(&resume).unwrap(),
            [
                "       If local_64 Then GoTo L000C",
                "L0006: Resume Next",
                "       Resume L0006",
                "L000C: Resume",
                "       Exit Sub"
            ]
        );
        let destruct = [0x52, 0x88, 0xFE, 0x03, 0x00, 0x0C];
        assert_eq!(lines(&destruct).unwrap(), ["       Exit Sub"]);
        let two = [
            0x0F, 0xA4, 0xFF, 0x0F, 0xA4, 0xFF, 0x02, 0x01, 0x50, 0x29, 0x00, 0x10, 0x00, 0x0C,
        ];
        assert_eq!(lines(&two), Err(LiftFault::CallArguments(8)));
        // For local_64 = 1 To 9 Step 2
        let step = [
            0x02, 0x01, 0x15, 0x9C, 0xFF, 0x02, 0x09, 0x02, 0x02, 0x41, 0x9C, 0xFF, 0x0E, 0x00,
            0x0C,
        ];
        assert_eq!(
            lines(&step).unwrap()[0],
            "       For local_64 = 1 To 9 Step 2  ' past the end: GoTo L000E"
        );
    }

    #[test]
    fn a_late_call_takes_its_variants_in_the_order_of_the_source() {
        let table = PcodeTable::parse(TABLE.as_bytes()).unwrap();
        let callees = Callees::default()
            .with_name(7, "Run")
            .with_name(8, "RegRead");
        // local_1C.Run local_64, local_60; local_88 = local_1C.RegRead(local_64)
        let body = [
            0x06, 0xE4, 0xFF, 0x43, 0x9C, 0xFF, 0x43, 0xA0, 0xFF, 0x44, 0x07, 0x00, 0x02, 0x00,
            0x43, 0x9C, 0xFF, 0x06, 0xE4, 0xFF, 0x45, 0x90, 0xFF, 0x08, 0x00, 0x01, 0x00, 0x05,
            0x78, 0xFF, 0x0C,
        ];
        let listing = disassemble(&Region::new(&body, Off::new(0)), &table);
        assert_eq!(
            render(&lift(&listing, &table, &callees, None).unwrap()),
            [
                "       Call local_1C.Run(local_64, local_60)",
                "       local_88 = local_1C.RegRead(local_64)",
                "       Exit Sub"
            ]
        );
        assert_eq!(name_indexes(&listing, &table), [7, 8]);
        // local_1C.Line (1, 2)-(3, 4), 9, BF: a late Line with the flags 38.
        let mut line = Vec::new();
        for (slot, value) in [
            (0x80_u8, 38_u8),
            (0x70, 1),
            (0x60, 2),
            (0x50, 3),
            (0x40, 4),
            (0x30, 9),
        ] {
            line.extend_from_slice(&[0x5F, slot, 0xFF, value, 0x00, 0x60]);
        }
        line.extend_from_slice(&[0x06, 0xE4, 0xFF, 0x44, 0x07, 0x00, 0x06, 0x00, 0x0C]);
        let listing_line = disassemble(&Region::new(&line, Off::new(0)), &table);
        assert_eq!(
            render(
                &lift(
                    &listing_line,
                    &table,
                    &Callees::default().with_name(7, "Line"),
                    None
                )
                .unwrap()
            )[0],
            "       local_1C.Line (1, 2)-(3, 4), 9, BF"
        );
        // local_1C.[DISPID 0x43] local_64; local_1C.[DISPID 0x3] = local_60
        let by_id = [
            0x06, 0xE4, 0xFF, 0x43, 0x9C, 0xFF, 0x59, 0x43, 0x00, 0x00, 0x00, 0x01, 0x00, 0x43,
            0xA0, 0xFF, 0x06, 0xE4, 0xFF, 0x5A, 0x03, 0x00, 0x00, 0x00, 0x0C,
        ];
        let listing_id = disassemble(&Region::new(&by_id, Off::new(0)), &table);
        assert!(name_indexes(&listing_id, &table).is_empty());
        // The same calls on Me.box1, of the interface _Box, whose Cls has
        // the DISPID 0x43.
        let types = VbTypes::parse(TYPES.as_bytes()).unwrap();
        let mut on_box = BOX1_IN_LOCAL_68.to_vec();
        on_box.extend_from_slice(&[
            0xFF, 0x59, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x43, 0xA0, 0xFF, 0x06, 0x98, 0xFF,
            0x5A, 0x43, 0x00, 0x00, 0x00, 0x18, 0x98, 0xFF, 0x0C,
        ]);
        let listing_box = disassemble(&Region::new(&on_box, Off::new(0)), &table);
        let boxed = Callees::default().with_control(0x32C, "box1", "_Box");
        assert_eq!(
            render(&lift(&listing_box, &table, &boxed, Some(&types)).unwrap()),
            [
                "       Call Me.box1.Cls()",
                "       Me.box1.Cls = local_60",
                "       Exit Sub"
            ]
        );
        // local_88 = Me.box1.Cls through LateIdLdVar.
        let mut get = BOX1_IN_LOCAL_68.to_vec();
        get.extend_from_slice(&[
            0xFF, 0x33, 0xB4, 0xFF, 0x43, 0x00, 0x00, 0x00, 0x05, 0x78, 0xFF, 0x18, 0x98, 0xFF,
            0x0C,
        ]);
        let listing_get = disassemble(&Region::new(&get, Off::new(0)), &table);
        assert_eq!(
            render(&lift(&listing_get, &table, &boxed, Some(&types)).unwrap())[0],
            "       local_88 = Me.box1.Cls"
        );
        assert_eq!(
            render(&lift(&listing_id, &table, &Callees::default(), None).unwrap()),
            [
                "       Call local_1C.[DISPID 0x43](local_64)",
                "       local_1C.[DISPID 0x3] = local_60",
                "       Exit Sub"
            ]
        );
        assert_eq!(
            lift(&listing, &table, &Callees::default(), None),
            Err(LiftFault::NoCallee(9))
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
                "L0015: Exit Sub",
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
