//! Byte fidelity: what this reader understands of a structure, measured
//! against the bytes the compiler that made the file wrote.
//!
//! Every structure this crate parses can be written back over the bytes it
//! was read from and compared against them. A byte that comes back different
//! is a field this reader gets wrong. A byte that is never written is a field
//! this reader does not reproduce. Neither is a matter of opinion once it is
//! measured, and the gap register states both in prose today.
//!
//! # The emitters do not live beside the readers, on purpose
//!
//! Every `impl Emit` in this module restates its field offsets from
//! `docs/STRUCTURES.md`. None of them copies an offset from the reader it is
//! graded against.
//!
//! `tests/support/mod.rs` already states the rule this follows: a harness
//! that shares a reader with the library it tests agrees with a bug in that
//! reader, and the failure is invisible. An emitter written next to its
//! reader, from the same constants, would report a clean diff for a field
//! both sides place at the wrong offset. Two independent statements of the
//! layout are what make a clean diff mean anything at all.
//!
//! This is also why a commit that adds an emitter changes nothing under `vb/`
//! or `read/`. When an emitter needs a field that a reader drops, a separate
//! commit first makes the reader keep that field.
//!
//! # A verbatim field cannot differ
//!
//! A field read at offset X and written back at offset X, with no arithmetic
//! between, can never disagree with itself. Most fields are verbatim, so most
//! of this measurement reports coverage rather than correctness, and a clean
//! map must never be read as "this structure is proven correct".
//!
//! Two things it does prove. The coverage figure is real: the bytes no field
//! claims are counted and pinned. And the two statements of the layout agree,
//! which is a genuine check on both.
//!
//! The fields worth watching are the ones that are **not** verbatim.
//! `crate::vb::object::Object::proc_count` is the first: the reader clamps it
//! to what the file can hold, so it differs in exactly the programs where the
//! clamp fires. The jump of an event stub is the second: the reader keeps the
//! handler address that it works out, and [`eventstub`] works the jump back
//! out of that address with its own arithmetic.
//!
//! An emitter can also write constant bytes that the reader checks and does
//! not keep. The event stub emitter writes the opcode bytes of the native
//! stub. The reader decodes no stub of another shape, so such a stub shows as
//! a refused record, and never as bytes that differ.
//!
//! The clamps on `wFormCount`, `dwControlCount` and `wEventCount` do not work
//! that way. They only bound a loop: the reader returns fewer entries, and no
//! byte it keeps changes. The byte diff cannot see such a clamp. The
//! [`census`] can: it counts what the file declares against what the reader
//! returns.
//!
//! # `Fault` is this repository's mistake, never the file's
//!
//! A [`crate::error::Defect`] is what the parser found in the user's file. A
//! [`Fault`] is what this module's own emitter did wrong. They must not share
//! a type: [`crate::error::Refusal::Damaged`] renders as "this Visual Basic 6
//! executable is damaged", and printing that sentence because a `put` in this
//! crate wrote the same byte twice would be a lie about someone's file.

pub mod census;
pub mod controlinfo;
pub mod eventstub;
pub mod gui;
pub mod guiobjectinfo;
pub mod header;
pub mod ledger;
pub mod object;
pub mod objectinfo;
pub mod optionalobjectinfo;
pub mod privateobj;
pub mod project;
pub mod slate;
pub mod walk;

pub use ledger::{Ledger, Run, Span, Verdict};
pub use slate::Slate;

use crate::read::region::{Off, Region};

/// What this module's own emitter did wrong.
///
/// Never a statement about the file under test. See the module doc comment.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum Fault {
    /// The structure declares more bytes than a slate holds.
    #[error("{structure} declares {len} bytes, which is more than a slate holds")]
    Oversize {
        /// The structure that declared the length.
        structure: &'static str,
        /// The length it declared.
        len: u32,
    },
    /// The window given for the structure is shorter than the structure.
    #[error("the window for {structure} holds {have} bytes and the structure is {want}")]
    Short {
        /// The structure being graded.
        structure: &'static str,
        /// The length the structure declares.
        want: u32,
        /// The length the window holds.
        have: u32,
    },
    /// The window has no absolute file offset, so nothing can be placed.
    #[error("the window for {structure} has no absolute file offset")]
    Unplaced {
        /// The structure being graded.
        structure: &'static str,
    },
    /// A write inside `emit` was refused. The emitter is wrong, not the file.
    #[error("the emitter for {structure} refused a write at structure offset {at:#x}")]
    Refused {
        /// The structure whose emitter refused.
        structure: &'static str,
        /// The structure relative offset of the first refused write.
        at: u32,
    },
    /// The slate was laid at one place and graded against another.
    #[error("{structure} was laid at a different place from the window it was graded against")]
    Ungraded {
        /// The structure being graded.
        structure: &'static str,
    },
}

/// A structure that writes itself back over the bytes it was read from.
///
/// # The contract
///
/// 1. [`Emit::emit`] writes every byte this reader models, and no other byte.
///    A byte no field of the model reproduces is a byte `emit` does not
///    write. Never pad and never zero fill a hole: the unwritten byte is the
///    answer, and [`Slate`] turns it into [`Verdict::Unmodelled`].
/// 2. `emit` writes each byte exactly once. A slate refuses the second write.
/// 3. [`Emit::LEN`] is the fixed length of the record on disk, taken from
///    `docs/STRUCTURES.md`.
/// 4. Every offset inside `emit` is read from `docs/STRUCTURES.md`, and never
///    copied from the reader this grades. See the module doc comment for why
///    the whole measurement depends on that.
/// 5. `emit` may write a constant byte that the reader checks and does not
///    keep, such as an opcode. The module doc comment of that emitter says
///    which bytes these are.
///
/// A structure whose length is not fixed cannot implement this trait, because
/// an associated constant cannot depend on `self`. That is deliberate. Those
/// structures need a length that comes from their own bytes, which is a
/// different contract and a later segment's work.
pub trait Emit {
    /// The number of bytes this structure takes on disk.
    const LEN: u32;
    /// The name this structure carries in a ledger.
    const STRUCTURE: &'static str;
    /// Writes every byte this reader models onto `slate`.
    fn emit(&self, slate: &mut Slate) -> Option<()>;
}

/// Lays `value` down on a fresh slate whose first byte is at `base`.
///
/// # Errors
///
/// Returns [`Fault::Oversize`] when no slate holds the structure, and
/// [`Fault::Refused`] when a write inside `emit` was refused, naming the first
/// offset it refused.
pub fn lay<T: Emit>(value: &T, base: Off) -> Result<Slate, Fault> {
    let mut slate = Slate::new(T::LEN, base).ok_or(Fault::Oversize {
        structure: T::STRUCTURE,
        len: T::LEN,
    })?;
    if value.emit(&mut slate).is_none() {
        return Err(Fault::Refused {
            structure: T::STRUCTURE,
            at: slate.refused().map_or(0, Off::get),
        });
    }
    Ok(slate)
}

/// Lays `value` back over the bytes it was read from and grades each one.
///
/// `original` is a window whose byte 0 is the structure's first byte. A
/// window longer than the structure is narrowed to [`Emit::LEN`], so the
/// caller may pass the region it already holds.
///
/// # Errors
///
/// Returns [`Fault::Unplaced`] when the window has no absolute file offset,
/// [`Fault::Short`] when it is shorter than the structure, whatever [`lay`]
/// returns, and [`Fault::Ungraded`] when the slate and the window disagree
/// about where they are.
pub fn compare<T: Emit>(value: &T, original: &Region<'_>) -> Result<Ledger, Fault> {
    let base = original.file_offset(Off::new(0)).ok_or(Fault::Unplaced {
        structure: T::STRUCTURE,
    })?;
    let window = original
        .subregion(Off::new(0), T::LEN)
        .ok_or(Fault::Short {
            structure: T::STRUCTURE,
            want: T::LEN,
            have: original.len(),
        })?;
    let slate = lay(value, base)?;
    let runs = slate.against(&window).ok_or(Fault::Ungraded {
        structure: T::STRUCTURE,
    })?;
    Ok(Ledger {
        structure: T::STRUCTURE,
        base,
        len: T::LEN,
        runs,
    })
}

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

    use super::{Emit, Fault, Slate, compare, lay};
    use crate::fidelity::ledger::{Span, Verdict};
    use crate::read::region::{Off, Region};

    /// An eight byte record whose reader models two fields and skips the four
    /// bytes between them, built here rather than read from a file, per the
    /// rule that a test builds the state it needs.
    struct Pair {
        first: u16,
        last: u16,
    }

    impl Emit for Pair {
        const LEN: u32 = 8;
        const STRUCTURE: &'static str = "Pair";

        fn emit(&self, slate: &mut Slate) -> Option<()> {
            slate.put_u16_le(Off::new(0x00), self.first)?;
            slate.put_u16_le(Off::new(0x06), self.last)?;
            Some(())
        }
    }

    /// A record whose emitter writes the same field twice, which is the
    /// copy-paste fault a slate exists to refuse.
    struct Doubled;

    impl Emit for Doubled {
        const LEN: u32 = 8;
        const STRUCTURE: &'static str = "Doubled";

        fn emit(&self, slate: &mut Slate) -> Option<()> {
            slate.put_u32_le(Off::new(0x00), 1)?;
            slate.put_u32_le(Off::new(0x00), 2)?;
            Some(())
        }
    }

    fn window(bytes: &[u8], base: u32) -> Region<'_> {
        Region::new(bytes, Off::new(base))
    }

    #[test]
    fn a_record_that_matches_the_file_grades_same_where_it_wrote_and_unmodelled_between() {
        let file = [0x11, 0x22, 0x00, 0x00, 0x00, 0x00, 0x33, 0x44];
        let pair = Pair {
            first: 0x2211,
            last: 0x4433,
        };
        let ledger = compare(&pair, &window(&file, 0x100)).unwrap();

        assert_eq!(ledger.structure, "Pair");
        assert_eq!(ledger.base, Off::new(0x100));
        assert_eq!(ledger.len, 8);
        assert!(ledger.tiles());
        assert!(ledger.is_clean());
        assert_eq!(ledger.bytes_with(Verdict::Same), 4);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 4);
        assert_eq!(ledger.bytes_with(Verdict::Differs), 0);

        let unmodelled: Vec<Span> = ledger
            .runs_with(Verdict::Unmodelled)
            .map(|run| run.span)
            .collect();
        assert_eq!(unmodelled, vec![Span::new(Off::new(0x102), 4)]);
    }

    #[test]
    fn an_unwritten_byte_that_happens_to_equal_the_original_is_unmodelled_and_never_same() {
        // Every byte of the file is zero, and a slate starts zero filled, so
        // a grading that compared bytes alone would call all eight `Same` and
        // report full coverage of a reader that models two fields. The four
        // bytes between the fields must stay `Unmodelled`.
        let file = [0_u8; 8];
        let pair = Pair { first: 0, last: 0 };
        let ledger = compare(&pair, &window(&file, 0)).unwrap();

        assert_eq!(ledger.bytes_with(Verdict::Same), 4);
        assert_eq!(ledger.bytes_with(Verdict::Unmodelled), 4);
        assert!(ledger.tiles());
    }

    #[test]
    fn one_wrong_field_reports_a_difference_at_that_field_and_nowhere_else() {
        let file = [0x11, 0x22, 0x00, 0x00, 0x00, 0x00, 0x33, 0x44];
        let wrong = Pair {
            first: 0x2211,
            last: 0x4432,
        };
        let ledger = compare(&wrong, &window(&file, 0x40)).unwrap();

        assert!(!ledger.is_clean());
        let differs: Vec<Span> = ledger
            .runs_with(Verdict::Differs)
            .map(|run| run.span)
            .collect();
        assert_eq!(differs, vec![Span::new(Off::new(0x46), 1)]);
        assert!(ledger.tiles());
    }

    #[test]
    fn a_window_longer_than_the_record_is_narrowed_to_the_records_own_length() {
        let file = [0x11, 0x22, 0, 0, 0, 0, 0x33, 0x44, 0xFF, 0xFF, 0xFF];
        let pair = Pair {
            first: 0x2211,
            last: 0x4433,
        };
        let ledger = compare(&pair, &window(&file, 0)).unwrap();
        assert_eq!(ledger.len, 8);
        assert!(ledger.is_clean());
    }

    #[test]
    fn a_window_shorter_than_the_record_is_a_fault_that_names_both_lengths() {
        let file = [0x11, 0x22, 0x00];
        let pair = Pair {
            first: 0x2211,
            last: 0,
        };
        assert_eq!(
            compare(&pair, &window(&file, 0)).unwrap_err(),
            Fault::Short {
                structure: "Pair",
                want: 8,
                have: 3,
            }
        );
    }

    #[test]
    fn an_emitter_that_writes_a_byte_twice_is_a_fault_naming_this_repository_and_the_offset() {
        // The message must never say the file is damaged. The file is fine.
        let err = lay(&Doubled, Off::new(0)).unwrap_err();
        assert_eq!(
            err,
            Fault::Refused {
                structure: "Doubled",
                at: 0,
            }
        );
        assert!(err.to_string().contains("the emitter for Doubled"));
        assert!(!err.to_string().contains("damaged"));
    }

    #[test]
    fn lay_places_the_slate_at_the_base_it_was_given() {
        let pair = Pair {
            first: 0xBEEF,
            last: 0xF00D,
        };
        let slate = lay(&pair, Off::new(0x2000)).unwrap();
        assert_eq!(slate.file_offset(Off::new(0)), Some(Off::new(0x2000)));
        assert_eq!(slate.covered_len(), 4);
        assert_eq!(
            slate.region().take(Off::new(0), 8).unwrap(),
            &[0xEF, 0xBE, 0x00, 0x00, 0x00, 0x00, 0x0D, 0xF0]
        );
    }

    #[test]
    fn a_slate_graded_against_a_window_based_somewhere_else_refuses_to_answer() {
        let file = [0_u8; 8];
        let pair = Pair { first: 0, last: 0 };
        let slate = lay(&pair, Off::new(0x100)).unwrap();
        // The window says it starts at 0x200. The slate was laid at 0x100.
        assert!(slate.against(&window(&file, 0x200)).is_none());
        assert!(slate.against(&window(&file, 0x100)).is_some());
    }
}
