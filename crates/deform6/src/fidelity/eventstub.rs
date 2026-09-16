//! Writing a bound event stub back over its own bytes.
//!
//! # Every value here comes from `docs/STRUCTURES.md` section 8.6
//!
//! A native stub is 13 bytes. `81 6C 24 04 <imm32>` is
//! `sub dword ptr [esp+4], imm32`, and `E9 <rel32>` is `jmp rel32`. The
//! handler address is `stub + 0x0D + rel32`. Not one of these values is copied
//! from `vb::controlinfo`, which is the reader this grades.
//!
//! # Five of the thirteen bytes are asserted, not read
//!
//! The reader reads `imm32` at `0x04` and `rel32` at `0x09`. It never reads
//! the two opcodes, so it decodes every stub as if the stub had the native
//! shape. This emitter writes that shape: the opcode bytes at `0x00` and at
//! `0x08`. A stub of another shape therefore shows as bytes that differ,
//! which is the fault that the assumption would hide.
//!
//! That is true only when the reader can work out a handler address from the
//! stub. A P-code stub at an address in a corpus program ends with `0xC3`,
//! which is the high byte of what the reader takes as `rel32`. The handler
//! address then goes below 0, and the reader keeps no handler.
//! [`EventStubRecord::of`] gives `None`, and the walk records the stub as
//! refused.
//!
//! # `rel32` is not verbatim
//!
//! The reader keeps the handler address, not `rel32`. This emitter works
//! `rel32` back out as `handler_address - (stub + 0x0D)`, with the length
//! restated from section 8.6. If the reader adds a wrong length, the four
//! bytes at `0x09` differ. After `ProcCount`, `rel32` is the second field that
//! this module writes back from a value the reader worked out.
//!
//! The arithmetic wraps. When the reader is right, the result is the file's
//! own four bytes. When the reader is wrong, the result is different, because
//! two different handler addresses cannot give the same four bytes.
//!
//! # Why `Emit` is on a record type
//!
//! A stub is reached through one slot of an event table, and only a bound
//! slot with a decoded handler has one. [`EventStubRecord::of`] gives `None`
//! for every other slot, so such a slot can produce neither a ledger nor a
//! fault. `of` names every field of the slot and of the handler, and uses no
//! `..`, so a field added to the reader stops this file compiling until
//! someone decides whether this emitter models it.

use crate::fidelity::{Emit, Slate};
use crate::read::region::{Off, Region, Va};
use crate::vb::controlinfo::{EventSlot, StubHandler};

/// `STRUCTURES.md` section 8.6: the opcode of `sub dword ptr [esp+4], imm32`.
const SUB_ESP4: [u8; 4] = [0x81, 0x6C, 0x24, 0x04];

/// `STRUCTURES.md` section 8.6: the opcode of `jmp rel32`.
const JMP_REL32: u8 = 0xE9;

/// `STRUCTURES.md` section 8.6: the jump counts from `stub + 0x0D`, the end of
/// the stub.
const JUMP_END: u32 = 0x0D;

/// The stub that one bound event slot names, as one record that can be
/// written back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventStubRecord {
    stub: Va,
    imm32: u32,
    handler_address: u32,
}

impl EventStubRecord {
    /// Gives the record of the stub that `slot` names, or `None` for an
    /// unbound slot and for a bound slot whose stub did not decode.
    #[must_use]
    pub const fn of(slot: &EventSlot) -> Option<Self> {
        match *slot {
            EventSlot::Bound {
                index: _,
                stub,
                handler:
                    Some(StubHandler {
                        is_method: _,
                        imm32,
                        handler_address,
                    }),
            } => Some(Self {
                stub,
                imm32,
                handler_address,
            }),
            EventSlot::Bound {
                index: _,
                stub: _,
                handler: None,
            }
            | EventSlot::Unbound { index: _ } => None,
        }
    }
}

impl Emit for EventStubRecord {
    /// `STRUCTURES.md` section 8.6: 13 bytes, `[C]`.
    const LEN: u32 = 0x0D;
    const STRUCTURE: &'static str = "EventStub";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        let rel32 = self
            .handler_address
            .wrapping_sub(self.stub.get().wrapping_add(JUMP_END));
        slate.put_bytes(Off::new(0x00), &SUB_ESP4)?; // sub dword ptr [esp+4]
        slate.put_u32_le(Off::new(0x04), self.imm32)?; // imm32
        slate.put_u8(Off::new(0x08), JMP_REL32)?; // jmp
        slate.put_u32_le(Off::new(0x09), rel32)?; // rel32
        Some(())
    }
}

/// Tells whether `window`, the 13 bytes of one stub, holds the two opcodes of
/// the native stub.
///
/// The fidelity walk asks this only to say why a stub that a slot names has
/// no record.
#[must_use]
pub(crate) fn has_native_shape(window: &Region<'_>) -> bool {
    window.take(Off::new(0x00), 4) == Some(SUB_ESP4.as_slice())
        && window.u8(Off::new(0x08)) == Some(JMP_REL32)
}

/// The bytes of an event stub that no field of [`EventStubRecord`]
/// reproduces, as `(structure relative offset, length)` pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[];

/// The number of event stub bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 13;

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::integer_division,
        reason = "a test builds the state it needs and must fail loudly when that state is wrong"
    )]

    use super::{EventStubRecord, MODELLED_BYTES, UNMODELLED, has_native_shape};
    use crate::fidelity::ledger::{Span, Verdict};
    use crate::fidelity::{Emit, compare, lay};
    use crate::read::region::{Off, Region, Va};
    use crate::vb::controlinfo::{EventSlot, StubHandler};

    /// The address the stub below sits at.
    const STUB: u32 = 0x0040_1ED0;

    /// Gives the record that a bound slot naming a stub at [`STUB`] with
    /// `imm32` and `handler_address` would give.
    fn record(imm32: u32, handler_address: u32) -> EventStubRecord {
        EventStubRecord::of(&EventSlot::Bound {
            index: 0,
            stub: Va::new(STUB),
            handler: Some(StubHandler {
                is_method: false,
                imm32,
                handler_address,
            }),
        })
        .unwrap()
    }

    /// The 13 bytes of a native stub, built from section 8.6.
    fn native(imm32: u32, rel32: i32) -> Vec<u8> {
        let mut raw = vec![0x81, 0x6C, 0x24, 0x04];
        raw.extend_from_slice(&imm32.to_le_bytes());
        raw.push(0xE9);
        raw.extend_from_slice(&rel32.to_le_bytes());
        raw
    }

    fn handler(rel32: i32) -> u32 {
        STUB.checked_add(13)
            .unwrap()
            .checked_add_signed(rel32)
            .unwrap()
    }

    #[test]
    fn an_unbound_slot_and_a_bound_slot_with_no_handler_have_no_stub_record() {
        assert_eq!(EventStubRecord::of(&EventSlot::Unbound { index: 3 }), None);
        assert_eq!(
            EventStubRecord::of(&EventSlot::Bound {
                index: 3,
                stub: Va::new(STUB),
                handler: None,
            }),
            None
        );
    }

    #[test]
    fn the_event_stub_is_thirteen_bytes() {
        assert_eq!(EventStubRecord::LEN, 0x0D);
        assert_eq!(EventStubRecord::LEN, 13);
    }

    #[test]
    fn every_byte_lands_where_the_format_document_puts_it() {
        let raw = native(0x3F, 0x0430);
        let ledger = compare(
            &record(0x3F, handler(0x0430)),
            &Region::new(&raw, Off::new(0x1ED0)),
        )
        .unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a byte landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_emitter_writes_all_thirteen_bytes() {
        let slate = lay(&record(0x3F, handler(0x0430)), Off::new(0)).unwrap();
        assert_eq!(slate.covered_len(), MODELLED_BYTES);
        assert_eq!(MODELLED_BYTES, 13);
        assert!(UNMODELLED.is_empty());
        let raw = native(0x3F, 0x0430);
        let ledger = compare(
            &record(0x3F, handler(0x0430)),
            &Region::new(&raw, Off::new(0)),
        )
        .unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), 13);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 0);
    }

    #[test]
    fn a_handler_wrong_by_one_differs_at_the_jump_and_nowhere_else() {
        let raw = native(0x3F, 0x0430);
        let ledger = compare(
            &record(0x3F, handler(0x0430) + 1),
            &Region::new(&raw, Off::new(0x1ED0)),
        )
        .unwrap();
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x1ED9), 1)]);
        assert!(ledger.tiles());
    }

    #[test]
    fn a_stub_with_other_opcode_bytes_differs_at_the_two_opcodes_and_nowhere_else() {
        // The reader reads imm32 at 0x04 and rel32 at 0x09 whatever the
        // opcodes are, so the record holds the same values as for a native
        // stub. Only the bytes the emitter asserts can show the difference.
        let mut raw = native(0x3F, 0x0430);
        raw[0x00..0x04].copy_from_slice(&[0x90, 0x90, 0x90, 0x90]);
        raw[0x08] = 0x90;
        let ledger = compare(
            &record(0x3F, handler(0x0430)),
            &Region::new(&raw, Off::new(0)),
        )
        .unwrap();
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(
            differs,
            vec![Span::new(Off::new(0x00), 4), Span::new(Off::new(0x08), 1)]
        );
        assert_eq!(ledger.bytes_with(Verdict::Same), 8);
    }

    #[test]
    fn a_window_has_the_native_shape_only_when_all_five_opcode_bytes_are_there() {
        let raw = native(0x3F, 0x0430);
        assert!(has_native_shape(&Region::new(&raw, Off::new(0))));
        for at in 0..13 {
            let mut changed = raw.clone();
            changed[at] ^= 0x01;
            let opcode = [0x00, 0x01, 0x02, 0x03, 0x08].contains(&at);
            assert_eq!(
                has_native_shape(&Region::new(&changed, Off::new(0))),
                !opcode,
                "byte {at:#x}"
            );
        }
        assert!(
            !has_native_shape(&Region::new(&raw[..0x08], Off::new(0))),
            "a window that ends before the jmp opcode"
        );
    }

    #[test]
    fn a_negative_jump_comes_back_as_the_file_holds_it() {
        let raw = native(0x3F, -0x0200);
        let ledger = compare(
            &record(0x3F, handler(-0x0200)),
            &Region::new(&raw, Off::new(0x1ED0)),
        )
        .unwrap();
        assert!(ledger.is_clean());
        assert_eq!(ledger.bytes_with(Verdict::Same), 13);
    }
}
