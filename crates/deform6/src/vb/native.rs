//! The annotated disassembly of the procedures of a native program.
//!
//! This module gives x86 instructions with notes. It does not give Basic
//! statements, and no public tool gives them from a native build.
//!
//! # The entries
//!
//! The walk starts at each 13-byte native stub in the image: `sub dword ptr
//! [esp+4], imm32`, then `jmp rel32`. The target of the jump is a procedure
//! of a form or of a class. A direct call to an address that holds code,
//! and that is not an import thunk or a `Declare` stub, is a procedure too.
//! A procedure of a standard module has no stub, so the walk finds it only
//! through a call. A module procedure that no walked code calls is not
//! found through a call. A frame prologue that installs
//! `__vbaExceptHandler` is an entry too, and it finds most of the rest.
//!
//! # The names
//!
//! The link table of an object gives the address of each procedure of the
//! object, and its place in the source. The file holds the name of a public
//! procedure only. The name of an event handler comes from its control and
//! from the types file. A private procedure takes its place, such as
//! `Form1.proc3`. A procedure of a standard module takes its place, when the
//! objects beside the module show where its code is.
//!
//! # The object calls
//!
//! A call through a vtable takes a note in one of three ways. A check block
//! after the call names the interface and the offset. A register that holds
//! the first argument of a procedure of an object gives the object. A global
//! variable that `__vbaNew2` fills gives the default instance of a form.
//!
//! # The bounds
//!
//! The walk follows the control flow from each entry. It never decodes
//! bytes in a straight line past a return, because the compiler puts data
//! between procedures. Each instruction count is limited by
//! [`MAX_INSTRUCTIONS`], so a crafted file cannot make the walk allocate
//! without end.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;

use iced_x86::{
    Code, Decoder, DecoderOptions, FlowControl, Formatter, Instruction, InstructionInfoFactory,
    IntelFormatter, MemorySize, Mnemonic, OpAccess, OpKind, Register,
};

use crate::read::pe::{ImportName, ImportedFunction, PeImage};
use crate::read::region::{Off, Region, Va};
use crate::vb::classify::{ObjectKind, classify};
use crate::vb::context::{default_iid, profile};
use crate::vb::controlinfo::EventReport;
use crate::vb::functyp::VbType;
use crate::vb::header::{VbHeader, header_region};
use crate::vb::helpers;
use crate::vb::lift::{Callees, control_array_function, member_name};
use crate::vb::links::{LinkSlot, read_method_links};
use crate::vb::object::{Object, ObjectTable};
use crate::vb::privateobj::{ProcNames, Procedure as ProcedureName, ProcedureList};
use crate::vb::project::{DeclareTable, ObjectTableHead, ProjectInfo};
use crate::vb::types::{TypeFunction, VbTypes, guid_text};
use crate::vb::{ObjectProcedures, ProcedureEntry, Report};

/// The most instructions that one listing holds.
///
/// All 44 native corpus programs together hold fewer than 200000.
pub const MAX_INSTRUCTIONS: usize = 4_000_000;

/// The longest string, in bytes, that a note gives.
const MAX_STRING_BYTES: u32 = 0x1_0000;

/// The name of the helper that raises the error of an overflow.
const RAISE_OVERFLOW: &str = "__vbaErrorOverflow";

/// The most entries that one jump table holds, less one.
const MAX_TABLE_ENTRIES: u32 = 0x1000;

/// The most jump table entries that one listing reads.
const MAX_TABLE_READS: u32 = 1_000_000;

/// The length of a native stub.
const STUB_LEN: u32 = 13;

/// The opcode of `jmp dword ptr [address]`, the shape of an import thunk.
const JUMP_THROUGH_ADDRESS: [u8; 2] = [0xFF, 0x25];

/// Where the stub of a `Declare` call holds the address that it pushes.
const DECLARE_PUSH_AT: u32 = 12;

/// The bytes of the stub of a `Declare` call, after the address of its
/// cache: `or eax, eax`, `je +2`, `jmp eax`, `push`.
const DECLARE_TAIL: [u8; 7] = [0x0B, 0xC0, 0x74, 0x02, 0xFF, 0xE0, 0x68];

/// The name of the helper that raises the error of a failed object call.
const CHECK_OBJECT: &str = "__vbaHresultCheckObj";

/// The name of the helper that makes the default instance of a form.
const NEW_INSTANCE: &str = "__vbaNew2";

/// The name of the helper of a late-bound call with no result.
const LATE_CALL: &str = "__vbaLateMemCall";

/// The name of the helper of a late-bound call with a result.
const LATE_CALL_LOAD: &str = "__vbaLateMemCallLd";

/// The name of the helper that makes a new object.
const NEW_OBJECT: &str = "__vbaNew";

/// How many pushes before a late-bound call can hold its name.
const MAX_LATE_PUSHES: usize = 4;

/// The longest member name of a late-bound call, in UTF-16 units.
const MAX_IDENTIFIER_UNITS: u32 = 0x100;

/// How many instructions after a call can jump to its check block.
const MAX_JUMPS_TO_CHECK: usize = 6;

/// How many instructions before a check block the search goes.
const MAX_CHECK_REACH: usize = 64;

/// Where `OptionalObjectInfo` is, after `ObjectInfo`.
const OPTIONAL_OBJECT_INFO_AT: u32 = 0x38;

/// Where `OptionalObjectInfo` holds the address of the GUID of the base
/// interface.
const BASE_IID_AT: u32 = 4;

/// The length of a GUID.
const GUID_LEN: u32 = 16;

/// Where a procedure of an object finds its first argument in its frame.
const FIRST_ARGUMENT_AT: u32 = 0x0C;

/// The end of the frame offsets that can be an argument.
const MAX_ARGUMENT_AT: u32 = 0x1_0000;

/// Where a procedure with no `ebp` frame finds the object, above `esp` at
/// its entry.
const FIRST_STACK_ARGUMENT: u32 = 4;

/// Where a procedure of an object finds the object in its frame.
const ME_ARGUMENT_AT: u32 = 8;

/// The name of the exception handler of the runtime.
const EXCEPTION_HANDLER: &str = "__vbaExceptHandler";

/// What a note tells about one instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Note {
    /// The instruction calls or jumps to a function that the file imports.
    Import(ImportedFunction),
    /// The instruction calls another procedure of the program.
    Procedure(Va),
    /// The instruction calls the stub of a `Declare` statement.
    Declare(Va),
    /// An operand is the address of this string constant.
    Text(String),
    /// The instruction calls a member of an object, and the check block
    /// after the call names the interface and the vtable offset.
    Interface {
        /// The GUID of the interface.
        iid: [u8; 16],
        /// The vtable offset of the member.
        offset: u16,
    },
    /// The instruction calls a member of the object of the procedure, at
    /// this vtable offset.
    Me(u16),
    /// An operand is the address of this floating point constant, as text.
    Number(String),
    /// The instruction makes a late-bound call of the member of this name.
    LateMember(String),
    /// The instruction calls `__vbaNew` for the class at this `ObjectInfo`.
    NewObject(Va),
    /// An operand is in the frame of the procedure, at this positive offset
    /// from `ebp`.
    Argument(u32),
    /// The instruction calls a member of the default instance of a form.
    Instance {
        /// The address of the `ObjectInfo` of the form.
        object: Va,
        /// The vtable offset of the member.
        offset: u16,
    },
}

/// One decoded instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Line {
    /// The address of the instruction.
    pub va: Va,
    /// The number of bytes of the instruction.
    pub len: u32,
    /// The instruction, in Intel syntax.
    pub text: String,
    /// The notes of the instruction.
    pub notes: Vec<Note>,
}

/// One procedure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Procedure {
    /// The address of the first instruction.
    pub entry: Va,
    /// Each instruction that the walk reached, by ascending address.
    pub lines: Vec<Line>,
    /// The address of each jump whose target the walk cannot follow.
    pub open_jumps: Vec<Va>,
    /// The address where the walk found bytes that are not an instruction.
    pub undecoded: Vec<Va>,
}

/// The annotated disassembly of one program.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Listing {
    /// Each procedure, by ascending entry.
    pub procedures: Vec<Procedure>,
    /// `true` when the walk stopped at [`MAX_INSTRUCTIONS`].
    pub truncated: bool,
}

/// Gives the whole of the bytes that `region` holds.
fn all_bytes<'a>(region: &Region<'a>) -> Option<&'a [u8]> {
    region.take(Off::new(0), region.len())
}

/// Gives the target of the native stub at `stub`, when `window` is the 13
/// bytes of a native stub.
pub(crate) fn stub_target(window: &[u8], stub: u32) -> Option<u32> {
    let [0x81, 0x6C, 0x24, 0x04, _, _, _, _, 0xE9, a, b, c, d] = *window else {
        return None;
    };
    stub.checked_add(STUB_LEN)?
        .checked_add_signed(i32::from_le_bytes([a, b, c, d]))
}

/// Gives the address of a procedure when `window` is a frame prologue
/// that installs the exception handler of the runtime: `push ebp`, `mov
/// ebp, esp`, `sub esp, 8` or `sub esp, 12`, then `push <handler>`.
fn prologue_handler(window: &[u8]) -> Option<u32> {
    let [0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x08 | 0x0C, 0x68, a, b, c, d] = *window else {
        return None;
    };
    Some(u32::from_le_bytes([a, b, c, d]))
}

/// Gives each known procedure entry of `pe`.
///
/// An entry is the target of a native stub, or a frame prologue that
/// installs `__vbaExceptHandler`. The second shape finds a procedure of a
/// standard module that no walked code calls.
#[must_use]
pub fn known_entries(pe: &PeImage<'_>) -> BTreeSet<u32> {
    let imports = pe.import_slots();
    let mut targets = BTreeSet::new();
    let is_handler = |thunk: u32| {
        let Some(region) = pe.region_at_va(Va::new(thunk)) else {
            return false;
        };
        region.take(Off::new(0), 2) == Some(&JUMP_THROUGH_ADDRESS[..])
            && region
                .u32_le(Off::new(2))
                .and_then(|slot| imports.get(&slot))
                .is_some_and(|function| {
                    function.name == ImportName::Name(EXCEPTION_HANDLER.to_owned())
                })
    };
    for section in pe.sections() {
        let Some(bytes) = pe
            .region_at(section.virtual_address)
            .as_ref()
            .and_then(all_bytes)
        else {
            continue;
        };
        let Some(start) = pe.image_base().checked_add(section.virtual_address.get()) else {
            continue;
        };
        let mut at = start;
        for (stub, prologue) in bytes
            .windows(13)
            .map(|window| (window, window.get(..11).unwrap_or_default()))
        {
            if let Some(target) = stub_target(stub, at)
                && pe.region_at_va(Va::new(target)).is_some()
            {
                targets.insert(target);
            }
            if prologue_handler(prologue).is_some_and(is_handler) {
                targets.insert(at);
            }
            let Some(next) = at.checked_add(1) else {
                break;
            };
            at = next;
        }
    }
    targets
}

/// Gives the string constant at `va`: a length in bytes, then UTF-16.
///
/// Gives `None` when the length is 0, odd or too large, when the text does
/// not end with a zero unit, and when a unit is a control character other
/// than a tab, a line feed or a carriage return.
fn string_at(pe: &PeImage<'_>, va: u32) -> Option<String> {
    let region = pe.region_at_va(Va::new(va.checked_sub(4)?))?;
    let bytes = region.u32_le(Off::new(0))?;
    if bytes == 0 || bytes.checked_rem(2) != Some(0) || bytes > MAX_STRING_BYTES {
        return None;
    }
    let text = region.take(Off::new(4), bytes)?;
    let terminator = region.u16_le(Off::new(bytes.checked_add(4)?))?;
    if terminator != 0 {
        return None;
    }
    let units: Vec<u16> = text
        .chunks_exact(2)
        .map(|pair| match *pair {
            [low, high] => u16::from_le_bytes([low, high]),
            _ => 0,
        })
        .collect();
    let decoded: String = char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .ok()?;
    decoded
        .chars()
        .all(|unit| !unit.is_control() || matches!(unit, '\t' | '\n' | '\r'))
        .then_some(decoded)
}

/// What a direct call or jump reaches.
enum Target {
    /// An import thunk.
    Import(ImportedFunction),
    /// The stub of a `Declare` statement.
    Declare,
    /// Code of the program.
    Code,
    /// An address that no section holds.
    Nothing,
}

/// The walk of one program.
struct Walk<'p, 'a> {
    /// The image.
    pe: &'p PeImage<'a>,
    /// The import map.
    imports: BTreeMap<u32, ImportedFunction>,
    /// Each known entry.
    entries: BTreeSet<u32>,
    /// The number of instructions that the walk decoded.
    decoded: usize,
    /// `true` when the walk reached [`MAX_INSTRUCTIONS`].
    truncated: bool,
    /// The number of jump table entries that the walk may still read.
    table_budget: u32,
    /// Each entry that is a procedure of an object, so that its first
    /// argument is the object.
    methods: BTreeSet<u32>,
}

impl Walk<'_, '_> {
    /// Tells what the direct call or jump to `target` reaches.
    fn classify(&self, target: u32) -> Target {
        let Some(region) = self.pe.region_at_va(Va::new(target)) else {
            return Target::Nothing;
        };
        if region.take(Off::new(0), 2) == Some(&JUMP_THROUGH_ADDRESS[..])
            && let Some(function) = region
                .u32_le(Off::new(2))
                .and_then(|slot| self.imports.get(&slot))
        {
            return Target::Import(function.clone());
        }
        if region.u8(Off::new(0)) == Some(0xA1)
            && region.take(Off::new(5), 7) == Some(&DECLARE_TAIL[..])
        {
            return Target::Declare;
        }
        Target::Code
    }

    /// Gives the notes of `instruction`.
    fn notes(&self, instruction: &Instruction) -> Vec<Note> {
        let mut notes = Vec::new();
        if matches!(
            instruction.flow_control(),
            FlowControl::Call | FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch
        ) && instruction.op0_kind() == OpKind::NearBranch32
        {
            let target = instruction.near_branch32();
            match self.classify(target) {
                Target::Import(function) => notes.push(Note::Import(function)),
                Target::Declare => notes.push(Note::Declare(Va::new(target))),
                Target::Code if instruction.flow_control() == FlowControl::Call => {
                    notes.push(Note::Procedure(Va::new(target)));
                }
                _ => {}
            }
            return notes;
        }
        for operand in 0..instruction.op_count() {
            match instruction.op_kind(operand) {
                OpKind::Memory
                    if instruction.memory_base() == Register::EBP
                        && instruction.memory_index() == Register::None
                        && instruction.memory_displacement32() >= FIRST_ARGUMENT_AT
                        && instruction.memory_displacement32() < MAX_ARGUMENT_AT =>
                {
                    notes.push(Note::Argument(instruction.memory_displacement32()));
                }
                OpKind::Immediate32 => {
                    if let Some(text) = string_at(self.pe, instruction.immediate32()) {
                        notes.push(Note::Text(text));
                    }
                }
                OpKind::Memory
                    if instruction.memory_base() == Register::None
                        && instruction.memory_index() == Register::None =>
                {
                    if let Some(function) = self.imports.get(&instruction.memory_displacement32()) {
                        notes.push(Note::Import(function.clone()));
                    } else if let Some(number) = self.number_at(instruction) {
                        notes.push(Note::Number(number));
                    }
                }
                _ => {}
            }
        }
        notes
    }

    /// Gives the text of the floating point constant that `instruction`
    /// reads from a fixed address. A value of 4 bytes prints in the fewest
    /// digits that give the same `Single`.
    fn number_at(&self, instruction: &Instruction) -> Option<String> {
        let region = self
            .pe
            .region_at_va(Va::new(instruction.memory_displacement32()))?;
        match instruction.memory_size() {
            MemorySize::Float32 => {
                let bits = region.u32_le(Off::new(0))?;
                let value = f32::from_bits(bits);
                value.is_finite().then(|| format!("{value}"))
            }
            MemorySize::Float64 => {
                let bytes: [u8; 8] = region.take(Off::new(0), 8)?.try_into().ok()?;
                let value = f64::from_le_bytes(bytes);
                value.is_finite().then(|| format!("{value}"))
            }
            _ => None,
        }
    }

    /// Gives the address that a `push imm32` hands to a later `ret`, when
    /// it is a place in the procedure that starts at `entry`.
    ///
    /// The compiler uses this shape for the cleanup block of a procedure.
    /// The address is after the instruction and before the next entry, and
    /// it is not a string.
    fn return_label(&self, instruction: &Instruction, entry: u32) -> Option<u32> {
        if instruction.code() != Code::Pushd_imm32 {
            return None;
        }
        let label = instruction.immediate32();
        let after = u32::try_from(instruction.next_ip()).ok()?;
        if label < after || string_at(self.pe, label).is_some() {
            return None;
        }
        let limit = self.entries.range(entry.checked_add(1)?..).next();
        (limit.is_none_or(|next| label < *next) && self.pe.region_at_va(Va::new(label)).is_some())
            .then_some(label)
    }

    /// Tells whether `instruction` calls a helper that never returns.
    ///
    /// The compiler puts the next data right after such a call, so the walk
    /// must stop there. The corpus shows this for `__vbaErrorOverflow`.
    fn never_returns(&self, instruction: &Instruction) -> bool {
        instruction.op0_kind() == OpKind::Memory
            && self
                .imports
                .get(&instruction.memory_displacement32())
                .is_some_and(|function| {
                    function.name == ImportName::Name(RAISE_OVERFLOW.to_owned())
                })
    }

    /// Gives the targets of the jump table of a `Select Case`: `cmp reg,
    /// limit`, `ja default`, `jmp dword ptr [reg*4+table]`.
    ///
    /// Gives `None` for another shape, when no compare of the register
    /// comes before it in the same run, and when an entry of the table
    /// maps into no section.
    fn jump_table(
        &mut self,
        instruction: &Instruction,
        last_compare: Option<(Register, u32)>,
    ) -> Option<Vec<u32>> {
        if instruction.op0_kind() != OpKind::Memory
            || instruction.memory_base() != Register::None
            || instruction.memory_index_scale() != 4
        {
            return None;
        }
        let (register, limit) = last_compare?;
        if register != instruction.memory_index() || limit > MAX_TABLE_ENTRIES {
            return None;
        }
        let table = self
            .pe
            .region_at_va(Va::new(instruction.memory_displacement32()))?;
        self.table_budget = self.table_budget.checked_sub(limit.checked_add(1)?)?;
        let mut targets = Vec::new();
        for index in 0..=limit {
            let target = table.u32_le(Off::new(index.checked_mul(4)?))?;
            self.pe.region_at_va(Va::new(target))?;
            targets.push(target);
        }
        Some(targets)
    }

    /// Decodes each hole between the instructions in `seen`.
    ///
    /// The compiler puts the error cleanup blocks of a procedure inside its
    /// body, and only the exception handler of the runtime reaches them. A
    /// hole is taken only when each byte of it decodes, the last instruction
    /// ends at the end of the hole, and no jump table overlaps it.
    fn fill_holes(
        &mut self,
        seen: &mut BTreeMap<u32, Instruction>,
        tables: &BTreeMap<u32, u32>,
        found: &mut Vec<u32>,
    ) {
        let mut holes = Vec::new();
        let mut previous_end = None;
        for (at, instruction) in seen.iter() {
            if let Some(end) = previous_end
                && end < *at
            {
                holes.push((end, *at));
            }
            previous_end = u32::try_from(instruction.next_ip()).ok();
        }
        for (start, end) in holes {
            if tables
                .range(..end)
                .next_back()
                .is_some_and(|(_, to)| start < *to)
            {
                continue;
            }
            let Some(bytes) = self
                .pe
                .region_at_va(Va::new(start))
                .and_then(|region| region.take(Off::new(0), end.checked_sub(start)?))
            else {
                continue;
            };
            let Ok(mut decoder) =
                Decoder::try_with_ip(32, bytes, u64::from(start), DecoderOptions::NONE)
            else {
                continue;
            };
            let mut block = Vec::new();
            let mut whole = true;
            while decoder.can_decode() {
                let instruction = decoder.decode();
                if instruction.is_invalid() {
                    whole = false;
                    break;
                }
                block.push(instruction);
            }
            let last_end = block.last().map(Instruction::next_ip);
            if !whole
                || last_end != Some(u64::from(end))
                || self.decoded.saturating_add(block.len()) > MAX_INSTRUCTIONS
            {
                continue;
            }
            self.decoded = self.decoded.saturating_add(block.len());
            for instruction in block {
                if instruction.flow_control() == FlowControl::Call
                    && instruction.op0_kind() == OpKind::NearBranch32
                {
                    let target = instruction.near_branch32();
                    if matches!(self.classify(target), Target::Code) && self.entries.insert(target)
                    {
                        found.push(target);
                    }
                }
                if let Ok(at) = u32::try_from(instruction.ip()) {
                    seen.insert(at, instruction);
                }
            }
        }
    }

    /// Gives the lines of the instructions in `seen`.
    ///
    /// The compiler loads a helper into a register once, and calls the
    /// register after that. A `call` of a register takes the note of the
    /// last `mov` that loaded the helper, while no other instruction writes
    /// that register.
    ///
    /// A call through a register that holds the vtable of the object of the
    /// procedure takes the note of that slot. The tracking reads the
    /// instructions in the order of their addresses.
    fn render(&self, seen: &BTreeMap<u32, Instruction>, entry: u32) -> Vec<Line> {
        let mut formatter = IntelFormatter::new();
        let mut info = InstructionInfoFactory::new();
        let mut loaded: BTreeMap<Register, ImportedFunction> = BTreeMap::new();
        let mut lines = Vec::with_capacity(seen.len());
        let mut frame = Frame::default();
        frame.me.pushed = Some(0);
        let globals = self.default_instances(seen);
        let mut pending: BTreeMap<u32, Frame> = BTreeMap::new();
        let mut fell_through = true;
        let in_method = self.methods.contains(&entry);
        for (at, instruction) in seen {
            match (pending.remove(at), fell_through) {
                (Some(saved), true) => frame.meet(&saved),
                (Some(saved), false) => frame = saved,
                (None, true) => {}
                (None, false) => frame.clear_registers(),
            }
            let mut text = String::new();
            formatter.format(instruction, &mut text);
            let mut notes = self.notes(instruction);
            if instruction.flow_control() == FlowControl::IndirectCall
                && instruction.op0_kind() == OpKind::Register
                && let Some(function) = loaded.get(&instruction.op0_register())
            {
                notes.push(Note::Import(function.clone()));
            }
            if in_method && let Some(offset) = frame.me.slot_of(instruction) {
                notes.push(Note::Me(offset));
            }
            if let Some((object, offset)) = frame.instances.slot_of(instruction) {
                notes.push(Note::Instance {
                    object: Va::new(object),
                    offset,
                });
            }
            let copied_object = frame.me.copy_target(instruction);
            for used in info.info(instruction).used_registers() {
                if matches!(
                    used.access(),
                    OpAccess::Write
                        | OpAccess::CondWrite
                        | OpAccess::ReadWrite
                        | OpAccess::ReadCondWrite
                ) {
                    loaded.remove(&used.register().full_register32());
                    frame.instances.forget(used.register().full_register32());
                    if !clears_the_low_bit(instruction) {
                        frame.me.forget(used.register().full_register32());
                    }
                }
            }
            frame.me.learn(instruction);
            frame.instances.learn(instruction, &globals);
            if let Some(register) = copied_object {
                frame.me.objects.insert(register);
            }
            let flow = instruction.flow_control();
            if matches!(
                flow,
                FlowControl::ConditionalBranch | FlowControl::UnconditionalBranch
            ) && instruction.op0_kind() == OpKind::NearBranch32
                && instruction.near_branch32() > *at
            {
                pending
                    .entry(instruction.near_branch32())
                    .and_modify(|saved| saved.meet(&frame))
                    .or_insert_with(|| frame.clone());
            }
            fell_through = !matches!(
                flow,
                FlowControl::UnconditionalBranch
                    | FlowControl::IndirectBranch
                    | FlowControl::Return
                    | FlowControl::Interrupt
                    | FlowControl::Exception
            );
            if instruction.mnemonic() == Mnemonic::Mov
                && instruction.op0_kind() == OpKind::Register
                && instruction.op1_kind() == OpKind::Memory
                && instruction.memory_base() == Register::None
                && instruction.memory_index() == Register::None
                && let Some(function) = self.imports.get(&instruction.memory_displacement32())
            {
                loaded.insert(instruction.op0_register(), function.clone());
            }
            lines.push(Line {
                va: Va::new(*at),
                len: u32::try_from(instruction.len()).unwrap_or(0),
                text,
                notes,
            });
        }
        let instructions: Vec<Instruction> = seen.values().copied().collect();
        self.note_checked_calls(&instructions, &mut lines);
        self.note_helper_arguments(&instructions, &mut lines);
        lines
    }

    /// Adds the note of what a late-bound call or a `New` names in its
    /// arguments.
    fn note_helper_arguments(&self, instructions: &[Instruction], lines: &mut [Line]) {
        for at in 0..lines.len() {
            let Some(name) = lines.get(at).and_then(|line| {
                line.notes.iter().find_map(|note| match note {
                    Note::Import(ImportedFunction {
                        name: ImportName::Name(name),
                        ..
                    }) if name == LATE_CALL || name == LATE_CALL_LOAD || name == NEW_OBJECT => {
                        Some(name.clone())
                    }
                    _ => None,
                })
            }) else {
                continue;
            };
            if !instructions
                .get(at)
                .is_some_and(|one| one.flow_control() == FlowControl::IndirectCall)
            {
                continue;
            }
            let mut values = instructions
                .get(..at)
                .unwrap_or_default()
                .iter()
                .rev()
                .take(MAX_CHECK_REACH)
                .filter(|earlier| earlier.mnemonic() == Mnemonic::Push)
                .take(MAX_LATE_PUSHES)
                .filter_map(pushed_value);
            let note = if name == NEW_OBJECT {
                values.next().map(|object| Note::NewObject(Va::new(object)))
            } else {
                values
                    .find_map(|value| self.identifier_at(value))
                    .map(Note::LateMember)
            };
            if let (Some(note), Some(line)) = (note, lines.get_mut(at)) {
                line.notes.push(note);
            }
        }
    }

    /// Gives the name at `va`: UTF-16 units up to a zero unit, each a
    /// letter, a digit or `_`.
    fn identifier_at(&self, va: u32) -> Option<String> {
        let region = self.pe.region_at_va(Va::new(va))?;
        let mut units = Vec::new();
        for index in 0..MAX_IDENTIFIER_UNITS {
            let unit = region.u16_le(Off::new(index.checked_mul(2)?))?;
            if unit == 0 {
                break;
            }
            units.push(unit);
        }
        let name = String::from_utf16(&units).ok()?;
        (!name.is_empty()
            && units.len() < MAX_IDENTIFIER_UNITS as usize
            && name
                .chars()
                .all(|one| one.is_ascii_alphanumeric() || one == '_'))
        .then_some(name)
    }

    /// Gives the `ObjectInfo` of the default instance that each global
    /// variable holds, from each call of `__vbaNew2` in `seen`.
    fn default_instances(&self, seen: &BTreeMap<u32, Instruction>) -> BTreeMap<u32, u32> {
        let instructions: Vec<&Instruction> = seen.values().collect();
        let mut globals = BTreeMap::new();
        for (at, instruction) in instructions.iter().enumerate() {
            let creates = instruction.flow_control() == FlowControl::IndirectCall
                && instruction.op0_kind() == OpKind::Memory
                && instruction.memory_base() == Register::None
                && self
                    .imports
                    .get(&instruction.memory_displacement32())
                    .is_some_and(|function| {
                        function.name == ImportName::Name(NEW_INSTANCE.to_owned())
                    });
            if !creates {
                continue;
            }
            let mut pushes = instructions
                .get(..at)
                .unwrap_or_default()
                .iter()
                .rev()
                .take(MAX_CHECK_REACH)
                .filter(|earlier| earlier.mnemonic() == Mnemonic::Push);
            if let (Some(object), Some(global)) = (
                pushes.next().and_then(|push| pushed_value(push)),
                pushes.next().and_then(|push| pushed_value(push)),
            ) {
                globals.insert(global, object);
            }
        }
        globals
    }

    /// Adds the note of each object call that a check block names.
    fn note_checked_calls(&self, instructions: &[Instruction], lines: &mut [Line]) {
        let checks: Vec<usize> = lines
            .iter()
            .zip(instructions)
            .enumerate()
            .filter(|(_, (line, instruction))| {
                instruction.flow_control() == FlowControl::IndirectCall
                    && line.notes.iter().any(is_object_check)
            })
            .map(|(at, _)| at)
            .collect();
        for check in checks {
            if let Some((at, note)) = self.checked_call(instructions, check)
                && let Some(line) = lines.get_mut(at)
            {
                line.notes
                    .retain(|old| !matches!(old, Note::Me(_) | Note::Instance { .. }));
                line.notes.push(note);
            }
        }
        for at in 0..lines.len() {
            let unnoted = lines.get(at).is_some_and(|line| line.notes.is_empty())
                && instructions
                    .get(at)
                    .is_some_and(|one| calls_vtable_slot(one, one.memory_displacement32()));
            if unnoted
                && let Some(note) = self.shared_check(instructions, lines, at)
                && let Some(line) = lines.get_mut(at)
            {
                line.notes.push(note);
            }
        }
    }

    /// Gives the four values of the check at index `check`: the index of
    /// the first `push`, the GUID and the vtable offset.
    ///
    /// The compiler gives the helper four values: the offset, the address
    /// of the GUID, the object and the result.
    fn check_values(
        &self,
        instructions: &[Instruction],
        check: usize,
    ) -> Option<(usize, [u8; 16], u32)> {
        let before = instructions.get(..check)?;
        let mut pushes = before
            .iter()
            .enumerate()
            .rev()
            .take(MAX_CHECK_REACH)
            .filter(|(_, instruction)| instruction.mnemonic() == Mnemonic::Push);
        pushes.next()?;
        pushes.next()?;
        let (_, iid) = pushes.next()?;
        let (first, offset) = pushes.next()?;
        let iid = pushed_value(iid)?;
        let offset = pushed_value(offset)?;
        let iid: [u8; 16] = self
            .pe
            .region_at_va(Va::new(iid))?
            .take(Off::new(0), 16)?
            .try_into()
            .ok()?;
        Some((first, iid, offset))
    }

    /// Gives the call that the check at index `check` belongs to, and its
    /// note. The call is the last one before the values of the check that
    /// goes through that offset of a vtable.
    fn checked_call(&self, instructions: &[Instruction], check: usize) -> Option<(usize, Note)> {
        let (first, iid, offset) = self.check_values(instructions, check)?;
        let (at, _) = instructions
            .get(..first)?
            .iter()
            .enumerate()
            .rev()
            .take(MAX_CHECK_REACH)
            .find(|(_, instruction)| calls_vtable_slot(instruction, offset))?;
        let offset = u16::try_from(offset).ok()?;
        Some((at, Note::Interface { iid, offset }))
    }

    /// Gives the note of the call at index `at` when the code after it
    /// reaches a check of the same offset. The compiler merges the check
    /// blocks of two calls that have the same values, in whole or in part,
    /// so the path can go through a jump.
    fn shared_check(
        &self,
        instructions: &[Instruction],
        lines: &[Line],
        at: usize,
    ) -> Option<Note> {
        let call = instructions.get(at)?;
        let offset = call.memory_displacement32();
        let next = at.checked_add(1)?;
        let after = instructions.get(next..)?;
        let branches = after
            .iter()
            .take(MAX_JUMPS_TO_CHECK)
            .filter(|jump| jump.flow_control() == FlowControl::ConditionalBranch)
            .filter_map(|jump| {
                (jump.op0_kind() == OpKind::NearBranch32)
                    .then(|| index_of(instructions, jump.near_branch32()))
                    .flatten()
            });
        std::iter::once(next)
            .chain(branches)
            .find_map(|start| self.check_on_path(instructions, lines, start, offset))
    }

    /// Follows the code from index `start` to a check, and gives its note
    /// when the check has the vtable offset `offset`.
    fn check_on_path(
        &self,
        instructions: &[Instruction],
        lines: &[Line],
        start: usize,
        offset: u32,
    ) -> Option<Note> {
        let mut at = start;
        let mut pushes = Vec::new();
        for _ in 0..MAX_CHECK_REACH {
            let instruction = instructions.get(at)?;
            let is_check = lines
                .get(at)
                .is_some_and(|line| line.notes.iter().any(is_object_check));
            match instruction.flow_control() {
                FlowControl::IndirectCall if is_check => {
                    let [first, iid, _, _] = *pushes.last_chunk::<4>()?;
                    if pushed_value(first)? != offset {
                        return None;
                    }
                    let iid: [u8; 16] = self
                        .pe
                        .region_at_va(Va::new(pushed_value(iid)?))?
                        .take(Off::new(0), 16)?
                        .try_into()
                        .ok()?;
                    return Some(Note::Interface {
                        iid,
                        offset: u16::try_from(offset).ok()?,
                    });
                }
                FlowControl::Call
                | FlowControl::IndirectCall
                | FlowControl::Return
                | FlowControl::IndirectBranch => return None,
                FlowControl::UnconditionalBranch => {
                    if instruction.op0_kind() != OpKind::NearBranch32 {
                        return None;
                    }
                    at = index_of(instructions, instruction.near_branch32())?;
                    continue;
                }
                _ => {}
            }
            if instruction.mnemonic() == Mnemonic::Push {
                pushes.push(instruction);
            }
            at = at.checked_add(1)?;
        }
        None
    }

    /// Walks the procedure at `entry`, and gives each new entry that its
    /// calls name.
    fn procedure(&mut self, entry: u32) -> (Procedure, Vec<u32>) {
        let mut seen: BTreeMap<u32, Instruction> = BTreeMap::new();
        let mut work = vec![entry];
        let mut found = Vec::new();
        let mut open_jumps = BTreeSet::new();
        let mut undecoded = BTreeSet::new();
        let mut tables = BTreeMap::new();
        while let Some(start) = work.pop() {
            let Some(bytes) = self
                .pe
                .region_at_va(Va::new(start))
                .as_ref()
                .and_then(all_bytes)
            else {
                undecoded.insert(start);
                continue;
            };
            let Ok(mut decoder) =
                Decoder::try_with_ip(32, bytes, u64::from(start), DecoderOptions::NONE)
            else {
                continue;
            };
            let mut last_compare = None;
            while decoder.can_decode() {
                if self.decoded >= MAX_INSTRUCTIONS {
                    self.truncated = true;
                    work.clear();
                    break;
                }
                let instruction = decoder.decode();
                let Ok(at) = u32::try_from(instruction.ip()) else {
                    break;
                };
                if seen.contains_key(&at) {
                    break;
                }
                if instruction.is_invalid() {
                    undecoded.insert(at);
                    break;
                }
                seen.insert(at, instruction);
                self.decoded = self.decoded.saturating_add(1);
                if instruction.mnemonic() == Mnemonic::Cmp
                    && instruction.op0_kind() == OpKind::Register
                    && let Some(limit) = instruction.try_immediate(1).ok().and_then(as_limit)
                {
                    last_compare = Some((instruction.op0_register(), limit));
                }
                match instruction.flow_control() {
                    FlowControl::Next => {
                        if let Some(label) = self.return_label(&instruction, entry) {
                            work.push(label);
                        }
                    }
                    FlowControl::ConditionalBranch => {
                        if instruction.op0_kind() == OpKind::NearBranch32 {
                            work.push(instruction.near_branch32());
                        }
                    }
                    FlowControl::UnconditionalBranch => {
                        if instruction.op0_kind() == OpKind::NearBranch32 {
                            let target = instruction.near_branch32();
                            if !self.entries.contains(&target)
                                && matches!(self.classify(target), Target::Code)
                            {
                                work.push(target);
                            }
                        }
                        break;
                    }
                    FlowControl::Call => {
                        if instruction.op0_kind() == OpKind::NearBranch32 {
                            let target = instruction.near_branch32();
                            if matches!(self.classify(target), Target::Code)
                                && self.entries.insert(target)
                            {
                                found.push(target);
                            }
                        }
                    }
                    FlowControl::IndirectCall if self.never_returns(&instruction) => break,
                    FlowControl::IndirectBranch => {
                        match self.jump_table(&instruction, last_compare) {
                            Some(targets) => {
                                let table = instruction.memory_displacement32();
                                let bytes = u32::try_from(targets.len().saturating_mul(4))
                                    .unwrap_or(u32::MAX);
                                tables.insert(table, table.saturating_add(bytes));
                                work.extend(targets);
                            }
                            None => {
                                open_jumps.insert(at);
                            }
                        }
                        break;
                    }
                    FlowControl::Return | FlowControl::Interrupt | FlowControl::Exception => break,
                    _ => {}
                }
            }
        }
        self.fill_holes(&mut seen, &tables, &mut found);
        let lines = self.render(&seen, entry);
        (
            Procedure {
                entry: Va::new(entry),
                lines,
                open_jumps: open_jumps.into_iter().map(Va::new).collect(),
                undecoded: undecoded.into_iter().map(Va::new).collect(),
            },
            found,
        )
    }
}

/// What the listing knows at one instruction.
#[derive(Clone, Default)]
struct Frame {
    /// The registers that hold the object of the procedure.
    me: MeTracker,
    /// The registers that hold a default instance.
    instances: InstanceTracker,
}

impl Frame {
    /// Keeps only what `other` knows too.
    fn meet(&mut self, other: &Self) {
        self.me.objects.retain(|one| other.me.objects.contains(one));
        self.me.vtables.retain(|one| other.me.vtables.contains(one));
        if self.me.pushed != other.me.pushed {
            self.me.pushed = None;
        }
        self.instances
            .objects
            .retain(|key, value| other.instances.objects.get(key) == Some(value));
        self.instances
            .vtables
            .retain(|key, value| other.instances.vtables.get(key) == Some(value));
    }

    /// Forgets what the next block cannot use. A block that no forward
    /// jump reaches keeps the registers that a call does not change, because
    /// the compiler keeps the object in one of them for the whole procedure.
    fn clear_registers(&mut self) {
        let lasts =
            |register: &Register| matches!(register, Register::EBX | Register::ESI | Register::EDI);
        self.me.objects.retain(lasts);
        self.me.vtables.clear();
        self.instances.objects.clear();
        self.instances.vtables.clear();
    }
}

/// The registers that hold the object of a procedure, or its vtable.
#[derive(Clone, Default)]
struct MeTracker {
    /// Each register that holds the object.
    objects: BTreeSet<Register>,
    /// Each register that holds the vtable of the object.
    vtables: BTreeSet<Register>,
    /// How far `esp` moved since the entry, until the first call. `None`
    /// before the first instruction shows it, and after the first call.
    pushed: Option<u32>,
}

impl MeTracker {
    /// Gives the vtable offset when `instruction` calls through the vtable
    /// of the object.
    fn slot_of(&self, instruction: &Instruction) -> Option<u16> {
        (instruction.flow_control() == FlowControl::IndirectCall
            && instruction.op0_kind() == OpKind::Memory
            && instruction.memory_index() == Register::None
            && self.vtables.contains(&instruction.memory_base()))
        .then(|| u16::try_from(instruction.memory_displacement32()).ok())
        .flatten()
    }

    /// Gives the target when `instruction` copies the object to a register.
    fn copy_target(&self, instruction: &Instruction) -> Option<Register> {
        (instruction.mnemonic() == Mnemonic::Mov
            && instruction.op0_kind() == OpKind::Register
            && instruction.op1_kind() == OpKind::Register
            && self.objects.contains(&instruction.op1_register()))
        .then(|| instruction.op0_register())
    }

    /// Forgets what `register` held.
    fn forget(&mut self, register: Register) {
        self.objects.remove(&register);
        self.vtables.remove(&register);
    }

    /// Records what `instruction` puts in a register. A call leaves no
    /// value in the three registers that a callee can change.
    fn learn(&mut self, instruction: &Instruction) {
        if matches!(
            instruction.flow_control(),
            FlowControl::Call | FlowControl::IndirectCall
        ) {
            self.pushed = None;
            for register in [Register::EAX, Register::ECX, Register::EDX] {
                self.forget(register);
            }
            return;
        }
        self.pushed = self.pushed.and_then(|pushed| match instruction.mnemonic() {
            Mnemonic::Push => pushed.checked_add(4),
            Mnemonic::Sub
                if instruction.op0_kind() == OpKind::Register
                    && instruction.op0_register() == Register::ESP =>
            {
                instruction
                    .try_immediate(1)
                    .ok()
                    .and_then(|bytes| u32::try_from(bytes).ok())
                    .and_then(|bytes| pushed.checked_add(bytes))
            }
            _ if instruction.op0_kind() == OpKind::Register
                && instruction.op0_register() == Register::ESP =>
            {
                None
            }
            _ => Some(pushed),
        });
        if instruction.code() != Code::Mov_r32_rm32 || instruction.op1_kind() != OpKind::Memory {
            return;
        }
        let base = instruction.memory_base();
        let offset = instruction.memory_displacement32();
        if instruction.memory_index() != Register::None {
            return;
        }
        let at_entry = self
            .pushed
            .and_then(|pushed| pushed.checked_add(FIRST_STACK_ARGUMENT));
        if (base == Register::EBP && offset == ME_ARGUMENT_AT)
            || (base == Register::ESP && Some(offset) == at_entry)
        {
            self.objects.insert(instruction.op0_register());
        } else if offset == 0 && self.objects.contains(&base) {
            self.vtables.insert(instruction.op0_register());
        }
    }
}

/// Tells whether `instruction` clears only the low bit of a register.
///
/// The first argument of a procedure of an object holds a flag in its low
/// bit, and the compiler clears the bit to get the object.
fn clears_the_low_bit(instruction: &Instruction) -> bool {
    instruction.mnemonic() == Mnemonic::And
        && instruction.op0_kind() == OpKind::Register
        && match (
            instruction.op0_register().size(),
            instruction.try_immediate(1),
        ) {
            (1, Ok(value)) => value == 0xFE,
            (4, Ok(value)) => value & 0xFFFF_FFFF == 0xFFFF_FFFE,
            _ => false,
        }
}

/// The default instances that a procedure loads, and the registers that
/// hold one of them or its vtable.
#[derive(Clone, Default)]
struct InstanceTracker {
    /// The `ObjectInfo` of the instance that each register holds.
    objects: BTreeMap<Register, u32>,
    /// The `ObjectInfo` of the instance whose vtable each register holds.
    vtables: BTreeMap<Register, u32>,
}

impl InstanceTracker {
    /// Gives the `ObjectInfo` and the vtable offset when `instruction`
    /// calls through the vtable of a default instance.
    fn slot_of(&self, instruction: &Instruction) -> Option<(u32, u16)> {
        if instruction.flow_control() != FlowControl::IndirectCall
            || instruction.op0_kind() != OpKind::Memory
            || instruction.memory_index() != Register::None
        {
            return None;
        }
        let object = *self.vtables.get(&instruction.memory_base())?;
        Some((
            object,
            u16::try_from(instruction.memory_displacement32()).ok()?,
        ))
    }

    /// Forgets what `register` held.
    fn forget(&mut self, register: Register) {
        self.objects.remove(&register);
        self.vtables.remove(&register);
    }

    /// Records what `instruction` puts in a register. `globals` gives the
    /// `ObjectInfo` of the form that each global variable holds.
    fn learn(&mut self, instruction: &Instruction, globals: &BTreeMap<u32, u32>) {
        if matches!(
            instruction.flow_control(),
            FlowControl::Call | FlowControl::IndirectCall
        ) {
            for register in [Register::EAX, Register::ECX, Register::EDX] {
                self.forget(register);
            }
            return;
        }
        if instruction.mnemonic() != Mnemonic::Mov
            || instruction.op0_kind() != OpKind::Register
            || instruction.op0_register().size() != 4
            || instruction.op1_kind() != OpKind::Memory
            || instruction.memory_index() != Register::None
        {
            return;
        }
        let target = instruction.op0_register();
        if instruction.memory_base() == Register::None {
            if let Some(object) = globals.get(&instruction.memory_displacement32()) {
                self.objects.insert(target, *object);
            }
        } else if instruction.memory_displacement32() == 0
            && let Some(object) = self.objects.get(&instruction.memory_base()).copied()
        {
            self.vtables.insert(target, object);
        }
    }
}

/// Gives the index in `instructions` of the instruction at `address`.
fn index_of(instructions: &[Instruction], address: u32) -> Option<usize> {
    instructions
        .binary_search_by_key(&u64::from(address), Instruction::ip)
        .ok()
}

/// Tells whether `note` names the helper that checks an object call.
fn is_object_check(note: &Note) -> bool {
    matches!(note, Note::Import(function)
        if function.name == ImportName::Name(CHECK_OBJECT.to_owned()))
}

/// Gives the value of a `push` of a constant.
fn pushed_value(instruction: &Instruction) -> Option<u32> {
    matches!(instruction.code(), Code::Pushd_imm32 | Code::Pushd_imm8)
        .then(|| instruction.try_immediate(0).ok())
        .flatten()
        .and_then(|value| u32::try_from(value).ok())
}

/// Tells whether `instruction` calls through the vtable slot at `offset`:
/// `call dword ptr [reg+offset]`.
fn calls_vtable_slot(instruction: &Instruction, offset: u32) -> bool {
    instruction.flow_control() == FlowControl::IndirectCall
        && instruction.op0_kind() == OpKind::Memory
        && instruction.memory_base() != Register::None
        && instruction.memory_index() == Register::None
        && instruction.memory_displacement32() == offset
}

/// Gives the limit of a compare when it is a small value that is not
/// negative.
fn as_limit(value: u64) -> Option<u32> {
    u32::try_from(value)
        .ok()
        .filter(|limit| *limit <= MAX_TABLE_ENTRIES)
}

/// Gives the annotated disassembly of `pe`.
///
/// No value of the file stops this function. A place that cannot be decoded
/// is named in [`Procedure::undecoded`].
#[must_use]
pub fn disassemble(pe: &PeImage<'_>) -> Listing {
    let entries = known_entries(pe);
    let mut work: Vec<u32> = entries.iter().rev().copied().collect();
    let mut walk = Walk {
        pe,
        imports: pe.import_slots(),
        entries,
        decoded: 0,
        truncated: false,
        table_budget: MAX_TABLE_READS,
        methods: object_methods(pe).into_keys().collect(),
    };
    let mut procedures = Vec::new();
    while let Some(entry) = work.pop() {
        let (procedure, found) = walk.procedure(entry);
        procedures.push(procedure);
        work.extend(found);
        if walk.truncated {
            break;
        }
    }
    procedures.sort_by_key(|procedure: &Procedure| procedure.entry.get());
    Listing {
        procedures,
        truncated: walk.truncated,
    }
}

/// Gives the name of each event handler of a form in `report`, by the
/// address of the machine code of the handler.
///
/// A handler takes the name that Basic gives it, such as `Form1.cmdOK_Click`.
/// With no name for its event, it takes the index of the slot, such as
/// `Form1.cmdOK_event3`.
#[must_use]
pub fn handler_names(report: &Report) -> BTreeMap<u32, String> {
    let mut names = BTreeMap::new();
    for form in &report.forms {
        for control in &form.controls {
            let owner = if control.parent.is_some() {
                control.name.as_str()
            } else {
                "Form"
            };
            for event in &control.events {
                let (address, label) = match event {
                    EventReport::Named {
                        event_name,
                        handler_address: Some(address),
                        ..
                    } => (*address, event_name.clone()),
                    EventReport::BoundUnnamed {
                        index,
                        handler_address: Some(address),
                        ..
                    } => (*address, format!("event{index}")),
                    _ => continue,
                };
                names.insert(address, format!("{}.{owner}_{label}", form.name));
            }
        }
    }
    names
}

/// Gives the objects of `pe`, or nothing when the object table cannot be
/// read.
fn objects_of(pe: &PeImage<'_>) -> Vec<Object> {
    (|| {
        let header = VbHeader::read(&header_region(pe).ok()?).ok()?;
        let info = ProjectInfo::read(pe, header.lp_project_data).ok()?;
        let head = ObjectTableHead::read(pe, info.lp_object_table).ok()?;
        Some(
            ObjectTable::walk(pe, info.lp_object_table, &head)
                .ok()?
                .objects,
        )
    })()
    .unwrap_or_default()
}

/// One procedure that the link table of an object names.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ObjectMethod {
    /// The name of the object.
    pub object: String,
    /// The place of the procedure among the procedures of the object, in
    /// the order of the source.
    pub index: u16,
    /// The place of the procedure in the name array of the object.
    pub slot: usize,
    /// The name of the procedure, when it is public.
    pub name: Option<String>,
}

impl ObjectMethod {
    /// Gives the text that names the procedure.
    #[must_use]
    pub fn text(&self) -> String {
        match &self.name {
            Some(name) => format!("{}.{name}", self.object),
            None => format!("{}.proc{}", self.object, self.index),
        }
    }
}

/// Gives each procedure that the link table of `object` names, by the
/// address of its code.
///
/// The name array of the object holds each `Declare` statement first, and
/// then each procedure in the order of the source. The link table holds the
/// public procedures in that order, and then the private ones in that
/// order. So the slots of the link table give the place of each procedure
/// in the source.
fn methods_of(pe: &PeImage<'_>, object: &Object) -> BTreeMap<u32, ObjectMethod> {
    let Ok(links) = read_method_links(pe, object) else {
        return BTreeMap::new();
    };
    let targets: Vec<u32> = links
        .slots
        .iter()
        .filter_map(|slot| match slot {
            LinkSlot::Native(target) => Some(target.get()),
            _ => None,
        })
        .collect();
    let slots = match ProcedureList::read(pe, object).procs {
        ProcNames::Slots(slots) => slots,
        ProcNames::NoNameArray { .. } => Vec::new(),
    };
    let first_slot = slots.len().saturating_sub(targets.len());
    let procedures = slots.get(first_slot..).unwrap_or_default();
    let (public, private): (Vec<_>, Vec<_>) = (0_u16..)
        .zip(procedures)
        .partition(|(_, slot)| matches!(slot, ProcedureName::Public(_)));
    let mut methods = BTreeMap::new();
    for (target, (index, slot)) in targets.iter().zip(public.into_iter().chain(private)) {
        let name = match slot {
            ProcedureName::Public(name) => Some(name.clone()),
            ProcedureName::Private => None,
        };
        methods.insert(
            *target,
            ObjectMethod {
                object: object.name.clone(),
                index,
                slot: first_slot.saturating_add(usize::from(index)),
                name,
            },
        );
    }
    methods
}

/// Gives each procedure that a link table of `pe` names, by the address of
/// its code.
#[must_use]
pub fn object_methods(pe: &PeImage<'_>) -> BTreeMap<u32, ObjectMethod> {
    objects_of(pe)
        .iter()
        .flat_map(|object| methods_of(pe, object))
        .collect()
}

/// Gives the stack bytes of an argument.
const fn argument_size(vb_type: &VbType, by_reference: bool) -> u32 {
    if by_reference {
        return 4;
    }
    match vb_type {
        VbType::Double | VbType::Date | VbType::Currency => 8,
        VbType::Variant => 16,
        _ => 4,
    }
}

/// Gives the name of each argument of `entry`, by its frame offset.
fn arguments_of(entry: &ProcedureEntry) -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();
    let mut at = FIRST_ARGUMENT_AT;
    match entry {
        ProcedureEntry::Public {
            prototype: Some(prototype),
            ..
        } => {
            for argument in &prototype.arguments {
                out.insert(at, argument.name.clone());
                let size = argument_size(
                    &argument.entry.vb_type,
                    argument.entry.by_ref || argument.entry.array,
                );
                at = at.saturating_add(size);
            }
        }
        ProcedureEntry::Handler { parameters, .. } => {
            for parameter in parameters {
                let (by_value, text) = match parameter.strip_prefix("ByVal ") {
                    Some(rest) => (true, rest),
                    None => (false, parameter.strip_prefix("ByRef ").unwrap_or(parameter)),
                };
                let name = text.split(' ').next().unwrap_or(text);
                out.insert(at, name.to_owned());
                let size = match (by_value, text.rsplit(' ').next()) {
                    (true, Some("Double" | "Date" | "Currency")) => 8,
                    (true, Some("Variant")) => 16,
                    _ => 4,
                };
                at = at.saturating_add(size);
            }
        }
        _ => {}
    }
    out
}

/// Gives the entries of `inside` that each module of `run` holds, in the
/// order of `run`.
///
/// A count of a module holds its procedures and its `Declare` statements.
/// So a run of one module holds each entry. In a longer run, the counts
/// divide the entries when their sum is the number of entries. When one
/// count alone is the number of entries, that module holds them. Otherwise
/// the file does not show the division, and each module gets no entry.
fn split_among_modules<'e>(pe: &PeImage<'_>, run: &[Object], inside: &'e [u32]) -> Vec<&'e [u32]> {
    if let [_] = run {
        return vec![inside];
    }
    let counts: Vec<usize> = run
        .iter()
        .map(|object| match ProcedureList::read(pe, object).procs {
            ProcNames::NoNameArray { proc_count } => {
                usize::try_from(proc_count).unwrap_or(usize::MAX)
            }
            ProcNames::Slots(slots) => slots.len(),
        })
        .collect();
    if counts
        .iter()
        .try_fold(0_usize, |sum, count| sum.checked_add(*count))
        == Some(inside.len())
    {
        let mut rest = inside;
        return counts
            .iter()
            .map(|count| {
                let (part, tail) = rest.split_at(*count);
                rest = tail;
                part
            })
            .collect();
    }
    let mut alone = counts.iter().filter(|count| **count == inside.len());
    if alone.next().is_some() && alone.next().is_none() {
        return counts
            .iter()
            .map(|count| if *count == inside.len() { inside } else { &[] })
            .collect();
    }
    vec![&[]; run.len()]
}

/// Gives each procedure of a standard module that `listing` holds, by the
/// address of its code.
///
/// The file holds no name for such a procedure. The compiler lays out the
/// objects in the order of the object table, and the procedures of an
/// object in the order of the source. So the procedures that no link table
/// names, between the code of the object before a module and the code of
/// the object after it, are the procedures of that module. Modules beside
/// each other divide the procedures as [`split_among_modules`] tells.
fn module_methods(pe: &PeImage<'_>, listing: &Listing) -> BTreeMap<u32, ObjectMethod> {
    let objects = objects_of(pe);
    let ranges: Vec<Option<(u32, u32)>> = objects
        .iter()
        .map(|object| {
            let methods = methods_of(pe, object);
            Some((*methods.keys().next()?, *methods.keys().next_back()?))
        })
        .collect();
    let owned = object_methods(pe);
    let is_module = |index: usize| {
        objects
            .get(index)
            .is_some_and(|object| classify(object.f_object_type) == ObjectKind::Module)
    };
    let lasts: Vec<u32> = ranges
        .iter()
        .scan(0_u32, |highest, range| {
            let before = *highest;
            if let Some((_, last)) = range {
                *highest = before.max(*last);
            }
            Some(before)
        })
        .collect();
    let mut firsts: Vec<u32> = ranges
        .iter()
        .rev()
        .scan(u32::MAX, |lowest, range| {
            let after = *lowest;
            if let Some((first, _)) = range {
                *lowest = after.min(*first);
            }
            Some(after)
        })
        .collect();
    firsts.reverse();
    let entries: BTreeSet<u32> = listing
        .procedures
        .iter()
        .map(|procedure| procedure.entry.get())
        .filter(|entry| !owned.contains_key(entry))
        .collect();
    let mut modules = BTreeMap::new();
    let mut index = 0;
    while index < objects.len() {
        if !is_module(index) {
            index = index.saturating_add(1);
            continue;
        }
        let start = index;
        while is_module(index) {
            index = index.saturating_add(1);
        }
        let (Some(low), Some(high)) = (lasts.get(start), firsts.get(index.saturating_sub(1)))
        else {
            continue;
        };
        if low >= high {
            continue;
        }
        let inside: Vec<u32> = entries
            .range((Bound::Excluded(*low), Bound::Excluded(*high)))
            .copied()
            .collect();
        let run = objects.get(start..index).unwrap_or_default();
        for (object, part) in run.iter().zip(split_among_modules(pe, run, &inside)) {
            for (place, entry) in (0_u16..).zip(part) {
                modules.entry(*entry).or_insert_with(|| ObjectMethod {
                    object: object.name.clone(),
                    index: place,
                    slot: usize::from(place),
                    name: None,
                });
            }
        }
        if modules.len() >= entries.len() {
            break;
        }
    }
    modules
}

/// What the file gives for one object of the program.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ObjectFacts {
    /// The name of the object.
    pub name: String,
    /// The address of the `ObjectInfo` of the object.
    pub object_info: u32,
    /// The GUID of the default interface of the object.
    pub default_iid: Option<[u8; 16]>,
    /// The GUID of the interface of the object that a form holds before
    /// the GUID of its base interface.
    pub own_iid: Option<[u8; 16]>,
    /// The members of the object that the file names, by vtable offset.
    pub callees: Callees,
    /// The code of each procedure of the object, by vtable offset.
    pub slots: BTreeMap<u16, u32>,
}

/// Gives the text of `function`: its name, and how a property goes.
fn function_text(function: &TypeFunction) -> String {
    let name = member_name(function);
    match function.kinds.first().map(String::as_str) {
        Some(kind @ ("get" | "let" | "set")) => format!("{name} ({kind})"),
        _ => name,
    }
}

/// Gives the GUID of the interface that the file holds right before the
/// GUID of the base interface of `object`.
fn own_iid(pe: &PeImage<'_>, object: &Object) -> Option<[u8; 16]> {
    let optional = object
        .lp_object_info
        .get()
        .checked_add(OPTIONAL_OBJECT_INFO_AT)?;
    let base = pe
        .region_at_va(Va::new(optional))?
        .u32_le(Off::new(BASE_IID_AT))?;
    let own = base.checked_sub(GUID_LEN)?;
    pe.region_at_va(Va::new(own))?
        .take(Off::new(0), GUID_LEN)?
        .try_into()
        .ok()
}

/// Gives what the file tells about `object`.
fn facts_of(pe: &PeImage<'_>, object: &Object, types: Option<&VbTypes>) -> ObjectFacts {
    let mut slots = BTreeMap::new();
    let links = read_method_links(pe, object).unwrap_or_default();
    for (slot, link) in (links.first_slot..).zip(&links.slots) {
        if let LinkSlot::Native(target) = link
            && let Some(offset) = slot
                .checked_mul(4)
                .and_then(|offset| u16::try_from(offset).ok())
        {
            slots.insert(offset, target.get());
        }
    }
    ObjectFacts {
        name: object.name.clone(),
        object_info: object.lp_object_info.get(),
        default_iid: default_iid(pe, object),
        own_iid: own_iid(pe, object),
        callees: profile(pe, object, types),
        slots,
    }
}

/// The names that the text of a listing needs.
pub struct Names<'t> {
    /// The name of each handler, by its address.
    pub handlers: BTreeMap<u32, String>,
    /// Each procedure that a link table names, by its address.
    pub methods: BTreeMap<u32, ObjectMethod>,
    /// What the file gives for each object.
    pub objects: Vec<ObjectFacts>,
    /// The name of each argument, by the address of the procedure and the
    /// frame offset.
    arguments: BTreeMap<u32, BTreeMap<u32, String>>,
    /// The index in [`Names::objects`] of the object of each interface GUID.
    object_of_iid: BTreeMap<[u8; 16], usize>,
    /// The index in [`Names::objects`] of the object of each `ObjectInfo`.
    object_of_info: BTreeMap<u32, usize>,
    /// The index in [`Names::objects`] of the object of each name.
    object_of_name: BTreeMap<String, usize>,
    /// The types file, which names an import by its ordinal and a member
    /// of an interface by its vtable offset.
    pub types: Option<&'t VbTypes>,
    /// The `Declare` table of the program.
    pub declares: Option<DeclareTable>,
}

impl<'t> Names<'t> {
    /// Gives the names of `pe`. A caller with no report gives no handler
    /// name.
    #[must_use]
    pub fn read(
        pe: &PeImage<'_>,
        listing: &Listing,
        report: Option<&Report>,
        types: Option<&'t VbTypes>,
    ) -> Self {
        let objects = objects_of(pe);
        let declares = header_region(pe)
            .ok()
            .and_then(|region| VbHeader::read(&region).ok())
            .and_then(|header| ProjectInfo::read(pe, header.lp_project_data).ok())
            .map(|info| DeclareTable::read(pe, &info));
        let objects: Vec<ObjectFacts> = objects
            .iter()
            .map(|object| facts_of(pe, object, types))
            .collect();
        let mut object_of_iid = BTreeMap::new();
        let mut object_of_info = BTreeMap::new();
        let mut object_of_name = BTreeMap::new();
        for (index, object) in objects.iter().enumerate() {
            for iid in [object.default_iid, object.own_iid].into_iter().flatten() {
                object_of_iid.entry(iid).or_insert(index);
            }
            object_of_info.entry(object.object_info).or_insert(index);
            object_of_name.entry(object.name.clone()).or_insert(index);
        }
        let methods: BTreeMap<u32, ObjectMethod> = object_methods(pe)
            .into_iter()
            .chain(module_methods(pe, listing))
            .collect();
        let entries: BTreeMap<&str, &Vec<ProcedureEntry>> = report
            .map(|report| report.objects.as_slice())
            .unwrap_or_default()
            .iter()
            .filter_map(|object| match &object.procedures {
                ObjectProcedures::Slots(slots) => Some((object.name.as_str(), slots)),
                ObjectProcedures::NoNameArray { .. } => None,
            })
            .collect();
        let arguments = methods
            .iter()
            .filter_map(|(address, method)| {
                let entry = entries.get(method.object.as_str())?.get(method.slot)?;
                Some((*address, arguments_of(entry)))
            })
            .collect();
        Names {
            handlers: report.map(handler_names).unwrap_or_default(),
            methods,
            arguments,
            objects,
            object_of_iid,
            object_of_info,
            object_of_name,
            types,
            declares,
        }
    }

    /// Gives the text of the function at `offset` of the interface `name`.
    fn function_text(&self, name: &str, offset: u16) -> Option<String> {
        Some(function_text(
            self.types?.interface(name)?.function(offset)?,
        ))
    }

    /// Gives the text of the member at `offset` of `object`, after the name
    /// `prefix`.
    fn member_text(&self, object: &ObjectFacts, prefix: &str, offset: u16) -> Option<String> {
        if let Some((control, _)) = object.callees.control(offset) {
            return Some(format!("{prefix}.{control}"));
        }
        if let Some((field, get)) = object.callees.variable(offset) {
            let way = if get { "get" } else { "let" };
            return Some(match object.callees.variable_name(field) {
                Some(name) => format!("{prefix}.{name} ({way})"),
                None => format!("{prefix}, public variable at {field:#X} ({way})"),
            });
        }
        if let Some(method) = object
            .slots
            .get(&offset)
            .and_then(|target| self.methods.get(target))
        {
            return Some(match &method.name {
                Some(name) => format!("{prefix}.{name}"),
                None => format!("{prefix}.proc{}", method.index),
            });
        }
        let base = object.callees.base()?;
        Some(format!("{prefix}.{}", self.function_text(base, offset)?))
    }

    /// Gives the text of the member at `offset` of the interface `iid`.
    fn interface_text(&self, iid: &[u8; 16], offset: u16) -> Option<String> {
        if *iid == [0; 16] {
            let function = control_array_function("", offset)?;
            return Some(format!("control array.{}", function_text(&function)));
        }
        if let Some(name) = self.types.and_then(|types| types.interface_of_iid(iid)) {
            return Some(format!("{name}.{}", self.function_text(name, offset)?));
        }
        let object = self.objects.get(*self.object_of_iid.get(iid)?)?;
        self.member_text(object, &object.name, offset)
    }

    /// Gives the text of the member at `offset` of the object of the
    /// procedure at `entry`.
    fn me_text(&self, entry: Va, offset: u16) -> String {
        match offset {
            0 => return "Me.QueryInterface".to_owned(),
            4 => return "Me.AddRef".to_owned(),
            8 => return "Me.Release".to_owned(),
            _ => {}
        }
        self.methods
            .get(&entry.get())
            .and_then(|method| {
                let index = self.object_of_name.get(&method.object)?;
                let object = self.objects.get(*index)?;
                self.member_text(object, "Me", offset)
            })
            .unwrap_or_else(|| format!("Me, vtable offset {offset:#X}"))
    }

    /// Gives the name of the procedure at `va`.
    fn procedure(&self, va: Va) -> String {
        self.handlers
            .get(&va.get())
            .cloned()
            .or_else(|| self.methods.get(&va.get()).map(ObjectMethod::text))
            .unwrap_or_else(|| format!("sub_{:08X}", va.get()))
    }

    /// Gives the text of `note`, in the procedure at `entry`.
    #[must_use]
    pub fn note(&self, pe: &PeImage<'_>, note: &Note, entry: Va) -> String {
        match note {
            Note::Import(function) => match &function.name {
                ImportName::Name(name) => match helpers::idiom(name) {
                    Some(idiom) => format!("{name}: {idiom}"),
                    None => name.clone(),
                },
                ImportName::Ordinal(ordinal) => {
                    match self.types.and_then(|types| types.import(*ordinal)) {
                        Some(import) => match helpers::basic_function(&import.name) {
                            Some(basic) => format!("{basic} ({}, ordinal {ordinal})", import.name),
                            None => format!("{} (ordinal {ordinal})", import.name),
                        },
                        None => format!("ordinal {ordinal} of {}", function.dll),
                    }
                }
            },
            Note::Procedure(va) => self.procedure(*va),
            Note::Declare(stub) => pe
                .region_at_va(*stub)
                .and_then(|region| region.va_le(Off::new(DECLARE_PUSH_AT)))
                .and_then(|descriptor| self.declares.as_ref()?.export_at(pe, descriptor))
                .map_or_else(
                    || format!("Declare stub {:08X}", stub.get()),
                    |export| format!("Declare {export}"),
                ),
            Note::Text(text) => format!("{text:?}"),
            Note::Number(text) => text.clone(),
            Note::LateMember(name) => format!("late-bound member {name}"),
            Note::NewObject(object) => self
                .object_of_info
                .get(&object.get())
                .and_then(|index| self.objects.get(*index))
                .map_or_else(
                    || format!("New, class at {:08X}", object.get()),
                    |facts| format!("New {}", facts.name),
                ),
            Note::Argument(offset) => self
                .arguments
                .get(&entry.get())
                .and_then(|names| names.get(offset))
                .cloned()
                .unwrap_or_default(),
            Note::Instance { object, offset } => self
                .object_of_info
                .get(&object.get())
                .and_then(|index| self.objects.get(*index))
                .and_then(|facts| self.member_text(facts, &facts.name, *offset))
                .unwrap_or_else(|| format!("default instance, vtable offset {offset:#X}")),
            Note::Me(offset) => self.me_text(entry, *offset),
            Note::Interface { iid, offset } => {
                self.interface_text(iid, *offset).unwrap_or_else(|| {
                    format!("interface {}, vtable offset {offset:#X}", guid_text(iid))
                })
            }
        }
    }
}

/// Gives the text of `listing`.
#[must_use]
pub fn listing_text(pe: &PeImage<'_>, listing: &Listing, names: &Names<'_>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for procedure in &listing.procedures {
        let _ = writeln!(
            out,
            "\n{} at {:08X}",
            names.procedure(procedure.entry),
            procedure.entry.get()
        );
        for line in &procedure.lines {
            let notes: Vec<String> = line
                .notes
                .iter()
                .map(|note| names.note(pe, note, procedure.entry))
                .filter(|text| !text.is_empty())
                .collect();
            if notes.is_empty() {
                let _ = writeln!(out, "  {:08X}  {}", line.va.get(), line.text);
            } else {
                let _ = writeln!(
                    out,
                    "  {:08X}  {:<36} ; {}",
                    line.va.get(),
                    line.text,
                    notes.join("; ")
                );
            }
        }
        for jump in &procedure.open_jumps {
            let _ = writeln!(
                out,
                "  {:08X}  this jump goes to an unknown place",
                jump.get()
            );
        }
        for place in &procedure.undecoded {
            let _ = writeln!(
                out,
                "  {:08X}  these bytes are not an instruction",
                place.get()
            );
        }
    }
    if listing.truncated {
        let _ = writeln!(
            out,
            "\nThe listing stops at {MAX_INSTRUCTIONS} instructions."
        );
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{ProcedureEntry, arguments_of, prologue_handler, stub_target};

    /// `sub dword ptr [esp+4], 0x3F`, then `jmp +0x423`.
    const STUB: [u8; 13] = [
        0x81, 0x6C, 0x24, 0x04, 0x3F, 0, 0, 0, 0xE9, 0x23, 0x04, 0, 0,
    ];

    #[test]
    fn a_handler_gives_each_argument_its_frame_offset() {
        let entry = ProcedureEntry::Handler {
            name: "a_Click".to_owned(),
            parameters: vec![
                "ByVal X As Double".to_owned(),
                "Y As Double".to_owned(),
                "ByVal Z As Variant".to_owned(),
                "Last As Long".to_owned(),
            ],
        };
        let names = arguments_of(&entry);
        assert_eq!(names[&0x0C], "X");
        assert_eq!(names[&0x14], "Y");
        assert_eq!(names[&0x18], "Z");
        assert_eq!(names[&0x28], "Last");
    }

    #[test]
    fn a_stub_gives_the_address_after_it_plus_the_displacement() {
        assert_eq!(stub_target(&STUB, 0x0040_1000), Some(0x0040_1430));
    }

    #[test]
    fn a_stub_at_the_top_of_the_address_space_gives_no_target() {
        assert_eq!(stub_target(&STUB, u32::MAX - 4), None);
    }

    #[test]
    fn bytes_of_another_shape_are_not_a_stub() {
        let mut other = STUB;
        other[8] = 0xE8;
        assert_eq!(stub_target(&other, 0x0040_1000), None);
    }

    #[test]
    fn a_prologue_gives_the_address_that_it_pushes() {
        let bytes = [
            0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x0C, 0x68, 0xA6, 0x10, 0x40, 0,
        ];
        assert_eq!(prologue_handler(&bytes), Some(0x0040_10A6));
    }

    #[test]
    fn a_prologue_with_another_frame_size_is_refused() {
        let bytes = [
            0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x10, 0x68, 0xA6, 0x10, 0x40, 0,
        ];
        assert_eq!(prologue_handler(&bytes), None);
    }
}
