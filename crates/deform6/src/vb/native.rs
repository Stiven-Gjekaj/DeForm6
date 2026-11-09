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
//! # The bounds
//!
//! The walk follows the control flow from each entry. It never decodes
//! bytes in a straight line past a return, because the compiler puts data
//! between procedures. Each instruction count is limited by
//! [`MAX_INSTRUCTIONS`], so a crafted file cannot make the walk allocate
//! without end.

use std::collections::{BTreeMap, BTreeSet};

use iced_x86::{
    Code, Decoder, DecoderOptions, FlowControl, Formatter, Instruction, InstructionInfoFactory,
    IntelFormatter, Mnemonic, OpAccess, OpKind, Register,
};

use crate::read::pe::{ImportName, ImportedFunction, PeImage};
use crate::read::region::{Off, Region, Va};
use crate::vb::Report;
use crate::vb::controlinfo::EventReport;
use crate::vb::header::{VbHeader, header_region};
use crate::vb::helpers;
use crate::vb::links::{LinkSlot, read_method_links};
use crate::vb::object::{Object, ObjectTable};
use crate::vb::privateobj::{ProcNames, Procedure as ProcedureName, ProcedureList};
use crate::vb::project::{DeclareTable, ObjectTableHead, ProjectInfo};
use crate::vb::types::{VbTypes, guid_text};

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

/// How many instructions before a check block the search goes.
const MAX_CHECK_REACH: usize = 64;

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
                    }
                }
                _ => {}
            }
        }
        notes
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
        let mut me = MeTracker::default();
        let in_method = self.methods.contains(&entry);
        for (at, instruction) in seen {
            let mut text = String::new();
            formatter.format(instruction, &mut text);
            let mut notes = self.notes(instruction);
            if instruction.flow_control() == FlowControl::IndirectCall
                && instruction.op0_kind() == OpKind::Register
                && let Some(function) = loaded.get(&instruction.op0_register())
            {
                notes.push(Note::Import(function.clone()));
            }
            if in_method && let Some(offset) = me.slot_of(instruction) {
                notes.push(Note::Me(offset));
            }
            let copied_object = me.copy_target(instruction);
            for used in info.info(instruction).used_registers() {
                if matches!(
                    used.access(),
                    OpAccess::Write
                        | OpAccess::CondWrite
                        | OpAccess::ReadWrite
                        | OpAccess::ReadCondWrite
                ) {
                    loaded.remove(&used.register().full_register32());
                    if !clears_the_low_bit(instruction) {
                        me.forget(used.register().full_register32());
                    }
                }
            }
            me.learn(instruction);
            if let Some(register) = copied_object {
                me.objects.insert(register);
            }
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
        lines
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
                line.notes.retain(|old| !matches!(old, Note::Me(_)));
                line.notes.push(note);
            }
        }
    }

    /// Gives the call that the check at index `check` belongs to, and its
    /// note.
    ///
    /// The compiler gives the helper four values: the offset, the address
    /// of the GUID, the object and the result. The call is the last one
    /// before them that goes through that offset of a vtable.
    fn checked_call(&self, instructions: &[Instruction], check: usize) -> Option<(usize, Note)> {
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
        let (at, _) = before
            .get(..first)?
            .iter()
            .enumerate()
            .rev()
            .take(MAX_CHECK_REACH)
            .find(|(_, instruction)| calls_vtable_slot(instruction, offset))?;
        let offset = u16::try_from(offset).ok()?;
        Some((at, Note::Interface { iid, offset }))
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

/// The registers that hold the object of a procedure, or its vtable.
#[derive(Default)]
struct MeTracker {
    /// Each register that holds the object.
    objects: BTreeSet<Register>,
    /// Each register that holds the vtable of the object.
    vtables: BTreeSet<Register>,
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
            for register in [Register::EAX, Register::ECX, Register::EDX] {
                self.forget(register);
            }
            return;
        }
        if instruction.code() != Code::Mov_r32_rm32 || instruction.op1_kind() != OpKind::Memory {
            return;
        }
        let base = instruction.memory_base();
        let offset = instruction.memory_displacement32();
        if instruction.memory_index() != Register::None {
            return;
        }
        if base == Register::EBP && offset == ME_ARGUMENT_AT {
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
    let procedures = slots
        .get(slots.len().saturating_sub(targets.len())..)
        .unwrap_or_default();
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

/// The names that the text of a listing needs.
pub struct Names<'t> {
    /// The name of each handler, by its address.
    pub handlers: BTreeMap<u32, String>,
    /// Each procedure that a link table names, by its address.
    pub methods: BTreeMap<u32, ObjectMethod>,
    /// The types file, which names an import by its ordinal.
    pub types: Option<&'t VbTypes>,
    /// The `Declare` table of the program.
    pub declares: Option<DeclareTable>,
}

impl Names<'_> {
    /// Gives the name of the procedure at `va`.
    fn procedure(&self, va: Va) -> String {
        self.handlers
            .get(&va.get())
            .cloned()
            .or_else(|| self.methods.get(&va.get()).map(ObjectMethod::text))
            .unwrap_or_else(|| format!("sub_{:08X}", va.get()))
    }

    /// Gives the text of `note`.
    fn note(&self, pe: &PeImage<'_>, note: &Note) -> String {
        match note {
            Note::Import(function) => match &function.name {
                ImportName::Name(name) => match helpers::idiom(name) {
                    Some(idiom) => format!("{name}: {idiom}"),
                    None => name.clone(),
                },
                ImportName::Ordinal(ordinal) => {
                    match self.types.and_then(|types| types.import(*ordinal)) {
                        Some(import) => format!("{} (ordinal {ordinal})", import.name),
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
            Note::Me(offset) => format!("Me, vtable offset {offset:#X}"),
            Note::Interface { iid, offset } => {
                format!("interface {}, vtable offset {offset:#X}", guid_text(iid))
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
            let notes: Vec<String> = line.notes.iter().map(|note| names.note(pe, note)).collect();
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
    use super::{prologue_handler, stub_target};

    /// `sub dword ptr [esp+4], 0x3F`, then `jmp +0x423`.
    const STUB: [u8; 13] = [
        0x81, 0x6C, 0x24, 0x04, 0x3F, 0, 0, 0, 0xE9, 0x23, 0x04, 0, 0,
    ];

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
