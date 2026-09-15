//! Writing an [`Object`] record back over its own bytes.
//!
//! # Every offset here comes from `docs/STRUCTURES.md` section 5.1
//!
//! Not one of them is copied from `vb::object`, which is the reader this
//! grades.
//!
//! # This is the first structure whose grading can fail
//!
//! Thirteen of the fourteen header fields are verbatim, so the header can
//! only ever report coverage. `ProcCount` is not verbatim.
//! `vb::object::bound_proc_count` compares the raw count against the number
//! of entries the file can actually hold behind `lpProcNamesArray`, and when
//! the raw value is larger it returns the bound instead and raises an
//! `ImplausibleCount` defect. The value this emitter writes back at `0x1C` is
//! therefore the cooked one, and it differs from the file in exactly the
//! programs where the clamp fired, and in no others.
//!
//! That is the "differs in some programs and not others" class the whole
//! measurement exists to find, and it comes out of the mechanism with no
//! special handling.
//!
//! # What this reader models, and what it leaves
//!
//! Five fields, 20 of the 48 bytes.
//!
//! | Bytes | Length | Fields section 5.1 names there |
//! |---|---|---|
//! | `0x04` | 20 | `dwReserved`, `lpPublicBytes`, `lpStaticBytes`, `lpModulePublic`, `lpModuleStatic` |
//! | `0x24` | 4 | `oStaticVars` |
//! | `0x2C` | 4 | `dwNull` |
//!
//! # `lpszObjectName` at `0x18` was the reason this measurement exists
//!
//! The reader first kept only the name it resolved through that address, and
//! threw the address away. Those four bytes therefore graded
//! [`Unmodelled`](crate::fidelity::Verdict::Unmodelled) although the reader
//! plainly read them, which is what showed that an `Unmodelled` run means
//! "this model cannot reproduce these bytes" and not "the reader never
//! looked".
//!
//! `Object` now carries `lpsz_object_name` and the four bytes are modelled.
//! The distinction the case exposed is still real, and it still caps what a
//! coverage figure can claim: a reader that reads a field, uses it, and does
//! not keep it understands more than its coverage figure reports.

use crate::fidelity::{Emit, Slate};
use crate::read::region::Off;
use crate::vb::object::Object;

impl Emit for Object {
    /// `STRUCTURES.md` section 5.1: `0x30` = 48 bytes, and all five sources
    /// agree field for field.
    const LEN: u32 = 0x30;
    const STRUCTURE: &'static str = "Object";

    fn emit(&self, slate: &mut Slate) -> Option<()> {
        slate.put_va_le(Off::new(0x00), self.lp_object_info)?; // lpObjectInfo
        slate.put_va_le(Off::new(0x18), self.lpsz_object_name)?; // lpszObjectName
        slate.put_u32_le(Off::new(0x1C), self.proc_count)?; // ProcCount
        slate.put_va_le(Off::new(0x20), self.lp_proc_names_array)?; // lpProcNamesArray
        slate.put_u32_le(Off::new(0x28), self.f_object_type)?; // fObjectType
        Some(())
    }
}

/// The bytes of an `Object` no field of [`Object`] reproduces, as
/// `(structure relative offset, length)` pairs.
pub const UNMODELLED: &[(u32, u32)] = &[(0x04, 20), (0x24, 4), (0x2C, 4)];

/// The number of `Object` bytes this reader reproduces.
pub const MODELLED_BYTES: u32 = 20;

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

    use super::{MODELLED_BYTES, UNMODELLED};
    use crate::fidelity::ledger::{Span, Verdict};
    use crate::fidelity::{Emit, compare};
    use crate::read::region::{Off, Region, Va};
    use crate::vb::object::Object;

    fn object() -> Object {
        Object {
            lp_object_info: Va::new(0x0040_5000),
            lpsz_object_name: Va::new(0x0040_7000),
            name: "Form1".to_owned(),
            proc_count: 7,
            lp_proc_names_array: Va::new(0x0040_6000),
            f_object_type: 0x0000_0021,
        }
    }

    /// The 48 bytes the object would have been read out of. The unmodelled
    /// bytes are `0xA5`, which the emitter cannot produce, so a grading that
    /// compared bytes rather than coverage would call them differences.
    fn bytes() -> Vec<u8> {
        let mut raw = vec![0xA5_u8; 0x30];
        let mut put = |at: usize, src: &[u8]| raw[at..at + src.len()].copy_from_slice(src);
        put(0x00, &0x0040_5000_u32.to_le_bytes());
        put(0x18, &0x0040_7000_u32.to_le_bytes());
        put(0x1C, &7_u32.to_le_bytes());
        put(0x20, &0x0040_6000_u32.to_le_bytes());
        put(0x28, &0x21_u32.to_le_bytes());
        raw
    }

    #[test]
    fn the_object_record_is_forty_eight_bytes() {
        assert_eq!(Object::LEN, 0x30);
        assert_eq!(Object::LEN, 48);
    }

    #[test]
    fn every_modelled_field_lands_on_the_offset_the_format_document_gives() {
        let raw = bytes();
        let ledger = compare(&object(), &Region::new(&raw, Off::new(0x800))).unwrap();
        assert!(ledger.tiles());
        assert!(
            ledger.is_clean(),
            "a modelled field landed somewhere else: {:?}",
            ledger.runs_with(Verdict::Differs).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_reader_models_twenty_of_the_forty_eight_object_bytes() {
        let raw = bytes();
        let ledger = compare(&object(), &Region::new(&raw, Off::new(0))).unwrap();
        assert_eq!(ledger.bytes_with(Verdict::Same), MODELLED_BYTES);
        assert_eq!(ledger.bytes_with(Verdict::Same), 20);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 28);
        assert_eq!(MODELLED_BYTES + 28, Object::LEN);
    }

    #[test]
    fn the_three_unmodelled_ranges_are_the_ones_the_constant_names() {
        let raw = bytes();
        let ledger = compare(&object(), &Region::new(&raw, Off::new(0))).unwrap();
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
    fn the_object_name_address_at_eighteen_hex_is_modelled_now_that_the_reader_keeps_it() {
        // This test was written the other way round first: the reader
        // resolved lpszObjectName and threw the address away, so these four
        // bytes graded Unmodelled although the reader read them. That is what
        // the measurement found, and the reader now keeps the address.
        let raw = bytes();
        let ledger = compare(&object(), &Region::new(&raw, Off::new(0))).unwrap();

        assert!(
            ledger
                .runs_with(Verdict::Unmodelled)
                .all(|run| !(run.span.at <= Off::new(0x18)
                    && Off::new(0x18) < run.span.end().unwrap())),
            "0x18 must no longer sit in an unmodelled run"
        );
        let covering = ledger
            .runs_with(Verdict::Same)
            .find(|run| run.span.at <= Off::new(0x18) && Off::new(0x18) < run.span.end().unwrap())
            .expect("some Same run must cover 0x18");
        // 0x18, 0x1C and 0x20 are all modelled and adjacent, so they
        // coalesce into one run of twelve. A ledger never leaves two
        // neighbouring runs carrying the same verdict.
        assert_eq!(covering.span, Span::new(Off::new(0x18), 12));
    }

    #[test]
    fn a_clamped_procedure_count_differs_only_in_the_bytes_that_actually_differ() {
        // What the map reports when bound_proc_count clamps.
        //
        // The grading is by byte and not by field, so a count of 7 in the
        // file against a bound of 3 in the model reports **one** differing
        // byte, not four: the three high bytes of the little endian u32 are
        // zero on both sides and grade `Same`. Widening a difference to the
        // field that holds it would need the slate to remember field extents,
        // which would be more machinery reporting no more truth. A person
        // reading "0x1C differs" finds the field by reading one short `emit`.
        let raw = bytes();
        let mut clamped = object();
        clamped.proc_count = 3;
        let ledger = compare(&clamped, &Region::new(&raw, Off::new(0x800))).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x81C), 1)]);
        assert!(ledger.tiles());
    }

    #[test]
    fn a_count_that_differs_in_every_byte_reports_the_whole_field() {
        // The companion to the test above: when all four bytes really do
        // differ, all four are reported, so the one byte result there is a
        // property of those two values and not a cap in the grading.
        let raw = bytes();
        let mut clamped = object();
        clamped.proc_count = 0x0101_0101;
        let ledger = compare(&clamped, &Region::new(&raw, Off::new(0))).unwrap();
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x1C), 4)]);
    }
}
