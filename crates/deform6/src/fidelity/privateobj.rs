//! Writing a present `PrivateObj` back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 6.1
//!
//! Not one of them is copied from `vb::privateobj`, which is the reader this
//! grades. Section 6.1 marks the whole layout `[L]`, and section 5.4 gives a
//! rival reading of the same 64 bytes that it marks `[D]` and says not to use.
//!
//! # Why `Emit` is on a record type and not on `PrivateObj`
//!
//! [`PrivateObj`] is an enum. `Absent` means the object has no private object
//! at all, so there is no record on disk to write back. If `Emit` were
//! implemented on the enum, `Absent` would have to refuse its first write, and
//! `lay` would then report `Fault::Refused { at: 0 }`, which says this
//! repository refused a write at offset zero. That would be false.
//!
//! [`PrivateObjRecord::of`] gives `None` for `Absent`, so an absent record can
//! produce neither a ledger nor a fault: the type rules it out. The walk
//! records the object as ungraded instead, with the reason `Absent`.
//!
//! `of` names every field of `Present` and uses no `..`. A field added to the
//! reader therefore stops this file compiling until someone decides whether
//! this emitter models it.
//!
//! # What this reader models, and what it leaves
//!
//! Five fields. The bytes no field claims are named by section 6.1:
//!
//! | Bytes | Fields section 6.1 names there |
//! |---|---|
//! | `0x00` | `nul1`, `lpParentLink`, `unk1`, `nul2` |
//! | `0x14` | `nul3` |
//! | `0x1C` | `nul4` |
//! | `0x28` | `lpNull`, `nul5`, `nul6`, `nul7`, `unk3`, `unk4` |

use crate::fidelity::{Emit, Slate};
use crate::read::region::{Off, Va};
use crate::vb::privateobj::PrivateObj;

/// The fields of a present `PrivateObj`, as one record that can be written
/// back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivateObjRecord {
    cnt_public_vars: u16,
    cnt_events: u16,
    lp_func_type_info: Va,
    lp_public_vars: Va,
    lp_events_type_info: Va,
}

impl PrivateObjRecord {
    /// Gives the record a present `PrivateObj` was read from, or `None` for
    /// an absent one, which has no record on disk.
    #[must_use]
    pub const fn of(value: &PrivateObj) -> Option<Self> {
        match *value {
            PrivateObj::Present {
                cnt_public_vars,
                cnt_events,
                lp_func_type_info,
                lp_events_type_info,
                lp_public_vars,
            } => Some(Self {
                cnt_public_vars,
                cnt_events,
                lp_func_type_info,
                lp_public_vars,
                lp_events_type_info,
            }),
            PrivateObj::Absent => None,
        }
    }
}

impl Emit for PrivateObjRecord {
    /// `STRUCTURES.md` section 6.1: 64 bytes.
    const LEN: u32 = 0x40;
    const STRUCTURE: &'static str = "PrivateObj";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_u16_le(Off::new(0x10), self.cnt_public_vars)?; // cntPublicVars
        slate.put_u16_le(Off::new(0x12), self.cnt_events)?; // cntEvents
        slate.put_va_le(Off::new(0x18), self.lp_func_type_info)?; // lpFuncTypeInfo
        slate.put_va_le(Off::new(0x20), self.lp_public_vars)?; // lpPublicVars
        slate.put_va_le(Off::new(0x24), self.lp_events_type_info)?; // lpEventsTypeInfo
        Some(())
    }
}

/// The bytes of a `PrivateObj` no field of [`PrivateObjRecord`] reproduces,
/// as `(structure relative offset, length)` pairs.
///
/// Computed from `emit` by laying a value and reading back which bytes were
/// written, and pasted here. Never worked out by hand.
pub const UNMODELLED: &[(u32, u32)] = &[(0x00, 16), (0x14, 4), (0x1C, 4), (0x28, 24)];

/// The number of `PrivateObj` bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 16;

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

    use super::{MODELLED_BYTES, PrivateObjRecord, UNMODELLED};
    use crate::fidelity::ledger::{Span, Verdict};
    use crate::fidelity::{Emit, compare};
    use crate::read::region::{Off, Region, Va};
    use crate::vb::privateobj::PrivateObj;

    fn present() -> PrivateObj {
        PrivateObj::Present {
            cnt_public_vars: 0x0011,
            cnt_events: 0x0022,
            lp_func_type_info: Va::new(0x0040_9018),
            lp_events_type_info: Va::new(0x0040_9024),
            lp_public_vars: Va::new(0x0040_9020),
        }
    }

    fn record() -> PrivateObjRecord {
        PrivateObjRecord::of(&present()).unwrap()
    }

    /// The 64 bytes that record would have been read out of, with every byte
    /// no field claims set to `0xA5`.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x40];
        let mut put = |at: usize, src: &[u8]| raw[at..at + src.len()].copy_from_slice(src);
        put(0x10, &0x0011_u16.to_le_bytes());
        put(0x12, &0x0022_u16.to_le_bytes());
        put(0x18, &0x0040_9018_u32.to_le_bytes());
        put(0x20, &0x0040_9020_u32.to_le_bytes());
        put(0x24, &0x0040_9024_u32.to_le_bytes());
        raw
    }

    #[test]
    fn an_absent_private_object_has_no_record_to_write_back() {
        assert_eq!(PrivateObjRecord::of(&PrivateObj::Absent), None);
    }

    #[test]
    fn a_present_private_object_carries_all_five_fields_into_its_record() {
        let record = record();
        assert_eq!(record.cnt_public_vars, 0x0011);
        assert_eq!(record.cnt_events, 0x0022);
        assert_eq!(record.lp_func_type_info, Va::new(0x0040_9018));
        assert_eq!(record.lp_public_vars, Va::new(0x0040_9020));
        assert_eq!(record.lp_events_type_info, Va::new(0x0040_9024));
    }

    #[test]
    fn the_private_obj_record_is_sixty_four_bytes() {
        assert_eq!(PrivateObjRecord::LEN, 0x40);
        assert_eq!(PrivateObjRecord::LEN, 64);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        // The two addresses at 0x20 and 0x24 are declared in the enum in the
        // other order. Distinct values catch an emitter that followed the
        // declaration order instead of the format.
        let raw = bytes();
        let ledger = compare(&record(), &Region::new(&raw, Off::new(0x4000))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_sixteen_of_the_sixty_four_bytes() {
        let raw = bytes();
        let ledger = compare(&record(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 16);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 48);
        assert_eq!(MODELLED_BYTES + 48, PrivateObjRecord::LEN);
    }

    #[test]
    fn the_unmodelled_ranges_are_the_ones_the_constant_names() {
        let raw = bytes();
        let ledger = compare(&record(), &Region::new(&raw, Off::new(0))).unwrap();
        let measured: Vec<Span> = ledger
            .runs_with(Verdict::Unmodelled)
            .map(|run| run.span)
            .collect();
        let declared: Vec<Span> = UNMODELLED
            .iter()
            .map(|(at, len)| Span::new(Off::new(*at), *len))
            .collect();
        assert_eq!(measured, declared);
    }

    #[test]
    fn an_event_count_wrong_by_one_shows_up_at_its_own_offset_and_nowhere_else() {
        let raw = bytes();
        let mut wrong = record();
        wrong.cnt_events ^= 1;
        let ledger = compare(&wrong, &Region::new(&raw, Off::new(0x4000))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x4012), 1)]);
        assert!(ledger.tiles());
    }
}
