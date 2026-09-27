//! The argument width of a P-code handler, read from its machine code.
//!
//! On entry to a handler, `esi` points at the first byte after the opcode.
//! A handler reads its arguments at `[esi + k]`, and it goes to the next
//! opcode through a dispatch jump:
//!
//! ```text
//! xor eax, eax
//! mov al, byte ptr [esi + k]
//! add esi, k + 1
//! jmp dword ptr [4*eax + table]
//! ```
//!
//! The width is the offset of the next opcode from the entry value of `esi`.
//! [`fall_through`] follows the code from the entry, and at each conditional
//! jump it takes the fall-through path, because the jumps that the corpus
//! showed lead to the error paths. It adds each constant change to `esi`,
//! and it stops at the first of these:
//!
//! - the dispatch jump: the width is the offset of the byte that `al`
//!   reads;
//! - another write to `esi`, such as the reload of a branch: the width is
//!   the end of the last argument read;
//! - a return, or a jump through a register: the same.
//!
//! At a jump through another table, such as the table of the `Variant`
//! types of `ForVar` and `NextVar`, [`fall_through`] follows each entry of
//! the table from the same state. A path that reaches the dispatch jump
//! gives the width; when two do, the larger one. When no path reaches it,
//! the path that reloads `esi` with the largest end of an argument gives
//! the width, as the loop of `NextVar` does with its target. A return is an
//! error path and gives nothing. The table has no end mark, so the entries
//! can run on into the next table: the table of `ForVar` runs on into the
//! table of `NextVar`, whose paths reload `esi`. The dispatch paths come
//! first for this reason. It follows one table on a path, and at a table
//! inside a table it stops as at a jump through a register.
//!
//! A `call` keeps `esi`, because `esi` is a callee-saved register.
//!
//! # A counted argument
//!
//! `FFreeStr`, `FFreeVar` and `FFreeAd` read a 16-bit byte count, and then
//! they free that many bytes of entries in a loop. [`next_offsets`] follows
//! each path, and for these handlers it gives the offsets 4, 6, 8 and more:
//! one for each number of passes through the loop. Such a handler has a
//! [`Width::Counted`] argument. `FFree1Str` and the other handlers that
//! free one entry use the same loop from a count of 1, and their offsets
//! start at 2: their width is fixed.
//!
//! # What the corpus showed
//!
//! With these widths, each of the 680 P-code bodies of `corpus-pcode/`
//! decodes to its exact end, or to an exit and fewer than four bytes of
//! padding. `check-pcode-table` repeats that check.

use std::collections::BTreeSet;

use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, Mnemonic, OpKind, Register};

use crate::pcode_table::Image;

/// The largest number of instructions that one path follows.
const STEP_LIMIT: usize = 400;

/// The largest number of states that [`next_offsets`] visits.
const STATE_LIMIT: usize = 4000;

/// The largest number of entries of a table that [`fall_through`] follows.
const TABLE_LIMIT: u32 = 64;

/// The length of the longest x86 instruction.
const MAX_INSTRUCTION: usize = 15;

/// The argument width of a handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Width {
    /// The handler reads this many bytes of arguments.
    Fixed(u32),
    /// The first two bytes are a byte count, and that many bytes follow.
    Counted,
}

/// The end of the fall-through path of a handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Trace {
    /// The dispatch jump, with the offset of the next opcode.
    Next(u32),
    /// Another write to `esi`, with the end of the last argument read.
    Reset(u32),
    /// A return, with the end of the last argument read.
    Ret(u32),
    /// A jump through a register or another table, with the end of the last
    /// argument read.
    Indirect(u32),
    /// The code could not be followed.
    Lost,
}

impl Trace {
    /// The width that this end gives, or `None` for [`Trace::Lost`].
    const fn width(self) -> Option<u32> {
        match self {
            Self::Next(width) | Self::Reset(width) | Self::Ret(width) | Self::Indirect(width) => {
                Some(width)
            }
            Self::Lost => None,
        }
    }
}

/// The state of one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct State {
    /// The address of the next instruction, relative to the image base.
    rva: u32,
    /// The change to `esi` since the entry.
    off: i64,
    /// The end of the last argument read, relative to the entry value.
    ext: i64,
    /// The offset of the byte that `al` read last, if one was read.
    al: Option<i64>,
}

/// What one instruction does to a path.
enum Step {
    /// The path goes on at these states: one, or two at a conditional jump,
    /// with the fall-through state first.
    Go(Vec<State>),
    /// The path ends.
    End(Trace),
    /// A jump through the table at this address, relative to the image base,
    /// with the state before the jump.
    Table(u32, State),
}

/// Decodes the instruction at `rva`.
fn decode(image: &Image, rva: u32) -> Option<Instruction> {
    let bytes = (1..=MAX_INSTRUCTION)
        .rev()
        .find_map(|len| image.bytes(rva, len))?;
    let ip = u64::from(image.base.checked_add(rva)?);
    let instruction = Decoder::with_ip(32, bytes, ip, DecoderOptions::NONE).decode();
    (!instruction.is_invalid()).then_some(instruction)
}

/// Gives the displacement of a memory operand as a signed value.
fn displacement(instruction: &Instruction) -> i64 {
    i64::from(i32::from_ne_bytes(
        instruction.memory_displacement32().to_ne_bytes(),
    ))
}

/// Gives the immediate of operand 1 as a signed value, when it has one.
fn immediate(instruction: &Instruction) -> Option<i64> {
    match instruction.op_kind(1) {
        OpKind::Immediate8 | OpKind::Immediate8to32 | OpKind::Immediate32 => {
            Some(i64::from_ne_bytes(instruction.immediate(1).to_ne_bytes()))
        }
        _ => None,
    }
}

/// Tells whether a memory operand is `[esi + disp]`.
fn on_esi(instruction: &Instruction) -> bool {
    instruction.memory_base() == Register::ESI && instruction.memory_index() == Register::None
}

/// Converts an end offset to a width. A negative end gives 0.
fn width_of(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}

/// Gives what one instruction does to a path. `tables` holds the addresses
/// of the dispatch tables.
fn step(image: &Image, tables: &BTreeSet<u32>, state: State) -> Step {
    let Some(instruction) = decode(image, state.rva) else {
        return Step::End(Trace::Lost);
    };
    let mut next = state;
    let Some(after) = u32::try_from(instruction.next_ip())
        .ok()
        .and_then(|ip| ip.checked_sub(image.base))
    else {
        return Step::End(Trace::Lost);
    };
    next.rva = after;

    let reads_al = instruction.mnemonic() == Mnemonic::Mov
        && instruction.op0_register() == Register::AL
        && instruction.op_kind(1) == OpKind::Memory
        && on_esi(&instruction);
    if reads_al {
        next.al = state.off.checked_add(displacement(&instruction));
    } else if (0..instruction.op_count()).any(|op| instruction.op_kind(op) == OpKind::Memory)
        && on_esi(&instruction)
    {
        let size = i64::try_from(instruction.memory_size().size()).unwrap_or(0);
        if let Some(end) = state
            .off
            .checked_add(displacement(&instruction))
            .and_then(|at| at.checked_add(size))
        {
            next.ext = next.ext.max(end);
        }
    }

    let is_dispatch = instruction.mnemonic() == Mnemonic::Jmp
        && instruction.op_kind(0) == OpKind::Memory
        && instruction.memory_base() == Register::None
        && instruction.memory_index() == Register::EAX
        && instruction.memory_index_scale() == 4
        && instruction
            .memory_displacement32()
            .checked_sub(image.base)
            .is_some_and(|table| tables.contains(&table));
    if is_dispatch {
        return Step::End(next.al.map_or(Trace::Lost, |al| Trace::Next(width_of(al))));
    }

    let writes_esi = instruction.op_count() > 0
        && instruction.op_kind(0) == OpKind::Register
        && instruction.op0_register() == Register::ESI;
    let exchanges_esi = instruction.mnemonic() == Mnemonic::Xchg
        && instruction.op_kind(1) == OpKind::Register
        && instruction.op1_register() == Register::ESI;
    let esi_change = match instruction.mnemonic() {
        Mnemonic::Add if writes_esi => immediate(&instruction),
        Mnemonic::Sub if writes_esi => immediate(&instruction).and_then(i64::checked_neg),
        Mnemonic::Inc if writes_esi => Some(1),
        Mnemonic::Dec if writes_esi => Some(-1),
        Mnemonic::Lea if writes_esi && on_esi(&instruction) => Some(displacement(&instruction)),
        _ => None,
    };
    if let Some(change) = esi_change {
        let Some(off) = next.off.checked_add(change) else {
            return Step::End(Trace::Lost);
        };
        next.off = off;
    } else if (writes_esi
        && !matches!(
            instruction.mnemonic(),
            Mnemonic::Cmp | Mnemonic::Test | Mnemonic::Push
        ))
        || exchanges_esi
        || matches!(
            instruction.mnemonic(),
            Mnemonic::Lodsb
                | Mnemonic::Lodsw
                | Mnemonic::Lodsd
                | Mnemonic::Movsb
                | Mnemonic::Movsw
                | Mnemonic::Movsd
                | Mnemonic::Cmpsb
        )
    {
        return Step::End(Trace::Reset(width_of(next.ext)));
    }

    let target = || {
        u32::try_from(instruction.near_branch_target())
            .ok()
            .and_then(|ip| ip.checked_sub(image.base))
    };
    match instruction.flow_control() {
        FlowControl::Return => Step::End(Trace::Ret(width_of(next.ext))),
        FlowControl::UnconditionalBranch => match target() {
            Some(rva) => Step::Go(vec![State { rva, ..next }]),
            None => Step::End(Trace::Lost),
        },
        FlowControl::IndirectBranch => {
            let table = (instruction.op_kind(0) == OpKind::Memory
                && instruction.memory_base() == Register::None
                && instruction.memory_index() != Register::None
                && instruction.memory_index_scale() == 4)
                .then(|| instruction.memory_displacement32().checked_sub(image.base))
                .flatten();
            match table {
                Some(table) => Step::Table(table, next),
                None => Step::End(Trace::Indirect(width_of(next.ext))),
            }
        }
        FlowControl::ConditionalBranch => match target() {
            Some(rva) => Step::Go(vec![next, State { rva, ..next }]),
            None => Step::End(Trace::Lost),
        },
        FlowControl::Interrupt | FlowControl::Exception | FlowControl::XbeginXabortXend => {
            Step::End(Trace::Lost)
        }
        FlowControl::Next | FlowControl::Call | FlowControl::IndirectCall => Step::Go(vec![next]),
    }
}

/// Follows the fall-through path from `entry`.
pub(crate) fn fall_through(image: &Image, tables: &BTreeSet<u32>, entry: u32) -> Trace {
    let state = State {
        rva: entry,
        off: 0,
        ext: 0,
        al: None,
    };
    follow(image, tables, state, false)
}

/// Follows the fall-through path from `state`. A jump through a table is
/// followed through each entry, unless the path is already inside a table.
fn follow(image: &Image, tables: &BTreeSet<u32>, mut state: State, in_table: bool) -> Trace {
    let mut seen = BTreeSet::new();
    for _ in 0..STEP_LIMIT {
        if !seen.insert((state.rva, state.off)) {
            return Trace::Lost;
        }
        match step(image, tables, state) {
            Step::End(trace) => return trace,
            Step::Table(_, at) if in_table => return Trace::Indirect(width_of(at.ext)),
            Step::Table(table, at) => return through_table(image, tables, table, at),
            Step::Go(states) => match states.first() {
                Some(first) => state = *first,
                None => return Trace::Lost,
            },
        }
    }
    Trace::Lost
}

/// Follows each entry of the table at `table` from `state`, up to the first
/// entry that is not an address in the image, and gives the largest width
/// of a dispatch path, or else of a path that reloads `esi`.
fn through_table(image: &Image, tables: &BTreeSet<u32>, table: u32, state: State) -> Trace {
    let mut next: Option<u32> = None;
    let mut reset: Option<u32> = None;
    let wider = |old: Option<u32>, new: u32| Some(old.map_or(new, |old| old.max(new)));
    for index in 0..TABLE_LIMIT {
        let Some(entry) = index
            .checked_mul(4)
            .and_then(|offset| table.checked_add(offset))
            .and_then(|at| image.bytes(at, 4))
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
            .and_then(|address| address.checked_sub(image.base))
            .filter(|entry| image.bytes(*entry, 1).is_some())
        else {
            break;
        };
        let path = follow(
            image,
            tables,
            State {
                rva: entry,
                ..state
            },
            true,
        );
        match path {
            Trace::Next(width) => next = wider(next, width),
            Trace::Reset(width) => reset = wider(reset, width),
            Trace::Ret(_) | Trace::Indirect(_) | Trace::Lost => {}
        }
    }
    Trace::Indirect(next.or(reset).unwrap_or_else(|| width_of(state.ext)))
}

/// Follows each path from `entry`, and gives each offset of the next opcode
/// that a dispatch jump reaches.
pub(crate) fn next_offsets(image: &Image, tables: &BTreeSet<u32>, entry: u32) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    let mut stack = vec![(
        State {
            rva: entry,
            off: 0,
            ext: 0,
            al: None,
        },
        0_usize,
    )];
    let mut seen = BTreeSet::new();
    while let Some((state, steps)) = stack.pop() {
        if steps > STEP_LIMIT || seen.len() > STATE_LIMIT || !seen.insert(state) {
            continue;
        }
        match step(image, tables, state) {
            Step::End(Trace::Next(offset)) => {
                out.insert(offset);
            }
            Step::End(_) | Step::Table(..) => {}
            Step::Go(states) => {
                let steps = steps.saturating_add(1);
                stack.extend(states.into_iter().rev().map(|state| (state, steps)));
            }
        }
    }
    out
}

/// Gives the width of the handler at `entry`, or `None` when its code cannot
/// be followed.
pub(crate) fn width(image: &Image, tables: &BTreeSet<u32>, entry: u32) -> Option<Width> {
    let offsets: Vec<u32> = next_offsets(image, tables, entry).into_iter().collect();
    let counted = offsets.len() >= 4
        && offsets.first() == Some(&4)
        && offsets
            .windows(2)
            .all(|pair| matches!(pair, [a, b] if b.checked_sub(*a) == Some(2)));
    if counted {
        return Some(Width::Counted);
    }
    fall_through(image, tables, entry).width().map(Width::Fixed)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{Trace, Width, fall_through, width};
    use crate::pcode_table::Image;

    const BASE: u32 = 0x7342_0000;
    const TABLE: u32 = 0x1000;
    const CODE: u32 = 0x2000;

    /// An image with a dispatch table at [`TABLE`] and `code` at [`CODE`].
    fn image(code: &[u8]) -> (Image, BTreeSet<u32>) {
        let mut section = vec![0xCC_u8; 0x2000];
        section[0x1000..0x1000 + code.len()].copy_from_slice(code);
        let image = Image {
            base: BASE,
            sections: vec![(0x1000, section)],
            exports: BTreeMap::new(),
        };
        (image, [TABLE].into_iter().collect())
    }

    /// `jmp dword ptr [4*eax + TABLE]`.
    fn dispatch() -> Vec<u8> {
        let mut code = vec![0xFF, 0x24, 0x85];
        code.extend_from_slice(&(BASE + TABLE).to_le_bytes());
        code
    }

    fn width_of(code: &[u8]) -> Option<Width> {
        let (image, tables) = image(code);
        width(&image, &tables, CODE)
    }

    #[test]
    fn a_handler_that_steps_esi_gives_the_offset_of_the_next_opcode() {
        // movsx eax, byte [esi] / push eax / xor eax,eax / mov al,[esi+1] /
        // add esi,2 / dispatch: LitI2_Byte.
        let mut code = vec![
            0x0F, 0xBE, 0x06, 0x50, 0x33, 0xC0, 0x8A, 0x46, 0x01, 0x83, 0xC6, 0x02,
        ];
        code.extend(dispatch());
        assert_eq!(width_of(&code), Some(Width::Fixed(1)));
    }

    #[test]
    fn a_step_of_esi_before_the_tail_is_counted() {
        // add esi,4 / xor eax,eax / mov al,[esi+2] / add esi,3 / dispatch.
        let mut code = vec![
            0x83, 0xC6, 0x04, 0x33, 0xC0, 0x8A, 0x46, 0x02, 0x83, 0xC6, 0x03,
        ];
        code.extend(dispatch());
        assert_eq!(width_of(&code), Some(Width::Fixed(6)));
    }

    #[test]
    fn a_branch_that_reloads_esi_gives_the_end_of_its_argument() {
        // movzx esi, word [esi] / add esi,[ebp-0x58] / xor eax,eax /
        // mov al,[esi] / inc esi / dispatch: Branch.
        let mut code = vec![
            0x0F, 0xB7, 0x36, 0x03, 0x75, 0xA8, 0x33, 0xC0, 0x8A, 0x06, 0x46,
        ];
        code.extend(dispatch());
        let (image, tables) = image(&code);
        assert_eq!(fall_through(&image, &tables, CODE), Trace::Reset(2));
    }

    #[test]
    fn a_jump_through_another_table_follows_its_entries_to_a_dispatch() {
        // add esi,2 / jmp [4*eax + 0x1800]; the table holds a path that
        // reads a 16-bit target and dispatches after it, a path that returns,
        // and then a word that is not an address.
        let mut section = vec![0xCC_u8; 0x2000];
        let mut code = vec![0x83, 0xC6, 0x02, 0xFF, 0x24, 0x85];
        code.extend_from_slice(&(BASE + 0x1800).to_le_bytes());
        section[0x1000..0x1000 + code.len()].copy_from_slice(&code);
        for (at, entry) in [(0x800, 0x2100_u32), (0x804, 0x2200), (0x808, 0)] {
            let value = if entry == 0 { 0 } else { BASE + entry };
            section[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        let mut read = vec![0x8A, 0x46, 0x02, 0x83, 0xC6, 0x03];
        read.extend(dispatch());
        section[0x1100..0x1100 + read.len()].copy_from_slice(&read);
        section[0x1200] = 0xC3;
        let image = Image {
            base: BASE,
            sections: vec![(0x1000, section)],
            exports: BTreeMap::new(),
        };
        let tables: BTreeSet<u32> = [TABLE].into_iter().collect();
        assert_eq!(width(&image, &tables, CODE), Some(Width::Fixed(4)));
    }

    #[test]
    fn a_dispatch_path_of_a_table_comes_before_a_path_that_reloads_esi() {
        // add esi,2 / jmp [4*eax + 0x1800]; the table holds a dispatch path
        // after a 16-bit argument, and a path that reads a wider argument
        // and reloads esi, as the next table does.
        let mut section = vec![0xCC_u8; 0x2000];
        let mut code = vec![0x83, 0xC6, 0x02, 0xFF, 0x24, 0x85];
        code.extend_from_slice(&(BASE + 0x1800).to_le_bytes());
        section[0x1000..0x1000 + code.len()].copy_from_slice(&code);
        for (at, entry) in [(0x800, 0x2100_u32), (0x804, 0x2200)] {
            section[at..at + 4].copy_from_slice(&(BASE + entry).to_le_bytes());
        }
        let mut read = vec![0x8A, 0x46, 0x02, 0x83, 0xC6, 0x03];
        read.extend(dispatch());
        section[0x1100..0x1100 + read.len()].copy_from_slice(&read);
        // mov esi, dword [esi+2]: an argument of 4 bytes that reloads esi.
        section[0x1200..0x1203].copy_from_slice(&[0x8B, 0x76, 0x02]);
        let image = Image {
            base: BASE,
            sections: vec![(0x1000, section)],
            exports: BTreeMap::new(),
        };
        let tables: BTreeSet<u32> = [TABLE].into_iter().collect();
        assert_eq!(width(&image, &tables, CODE), Some(Width::Fixed(4)));
    }

    #[test]
    fn the_fall_through_path_is_taken_at_a_conditional_jump() {
        // test eax,eax / jz +5 / mov al,[esi+2] / add esi,3 / dispatch; the
        // jump goes to a path with another width.
        let mut code = vec![0x85, 0xC0, 0x74, 0x0D, 0x8A, 0x46, 0x02, 0x83, 0xC6, 0x03];
        code.extend(dispatch());
        code.extend([0x8A, 0x46, 0x06]);
        code.extend(dispatch());
        assert_eq!(width_of(&code), Some(Width::Fixed(2)));
    }

    #[test]
    fn a_call_keeps_esi_and_the_path_goes_on_after_it() {
        // call +0 / mov al,[esi+2] / add esi,3 / dispatch.
        let mut code = vec![
            0xE8, 0x00, 0x00, 0x00, 0x00, 0x8A, 0x46, 0x02, 0x83, 0xC6, 0x03,
        ];
        code.extend(dispatch());
        assert_eq!(width_of(&code), Some(Width::Fixed(2)));
    }

    #[test]
    fn a_change_of_esi_by_a_register_ends_the_path() {
        // movsx eax, word [esi+2] / add esi,4 / sub esi,ecx / mov eax,[esi+8]
        // / ret: ExitProcCbHresult.
        let code = [
            0x0F, 0xBF, 0x46, 0x02, 0x83, 0xC6, 0x04, 0x2B, 0xF1, 0x8B, 0x46, 0x08, 0xC3,
        ];
        assert_eq!(width_of(&code), Some(Width::Fixed(4)));
    }

    #[test]
    fn a_return_and_a_jump_through_a_register_end_the_path() {
        assert_eq!(width_of(&[0x0F, 0xB7, 0x06, 0xC3]), Some(Width::Fixed(2)));
        assert_eq!(
            width_of(&[0x8B, 0x46, 0x02, 0xFF, 0xE0]),
            Some(Width::Fixed(6))
        );
    }

    /// movsx edi, word [esi] / add esi,2 / shr edi,1, then a loop that reads
    /// a word, adds 2 to `esi` and counts `edi` down, then the tail.
    fn counted_loop(open: &[u8]) -> Vec<u8> {
        let mut code = open.to_vec();
        let start = i8::try_from(code.len()).unwrap();
        code.extend([0x0F, 0xBF, 0x06, 0x83, 0xC6, 0x02, 0x4F]);
        let back = start - i8::try_from(code.len() + 2).unwrap();
        code.extend([0x75, back.to_le_bytes()[0]]);
        code.extend([0x33, 0xC0, 0x8A, 0x06, 0x46]);
        code.extend(dispatch());
        code
    }

    #[test]
    fn a_loop_over_a_counted_list_gives_a_counted_width() {
        let code = counted_loop(&[0x0F, 0xBF, 0x3E, 0x83, 0xC6, 0x02, 0xD1, 0xEF]);
        assert_eq!(width_of(&code), Some(Width::Counted));
    }

    #[test]
    fn the_same_loop_from_a_count_of_one_gives_a_fixed_width() {
        let code = counted_loop(&[0xBF, 0x01, 0x00, 0x00, 0x00]);
        assert_eq!(width_of(&code), Some(Width::Fixed(2)));
    }

    #[test]
    fn code_that_cannot_be_decoded_gives_no_width() {
        assert_eq!(width_of(&[0x8A, 0x46]), None);
        assert_eq!(width_of(&[0x0F, 0x0B]), None);
    }
}
