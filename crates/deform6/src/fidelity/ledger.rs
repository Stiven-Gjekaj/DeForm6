//! The result of grading one structure: every one of its bytes, in runs, each
//! run carrying what it says about this reader.
//!
//! # A partition, not a boolean and not a list of bad bytes
//!
//! A [`Ledger`] tiles the structure exactly: ascending, no gap, no overlap,
//! every byte in exactly one run, and no two neighbouring runs carrying the
//! same verdict. That invariant is what makes the result self checking, and it
//! is the same property `Tiling` in `vb::gui` already holds the property
//! stream to.
//!
//! A boolean would answer "did anything differ", which is the least useful
//! question. The useful questions are which bytes differed and how many bytes
//! nothing claims, and both need ranges.
//!
//! # These three words describe one file, never the corpus
//!
//! [`Verdict::Same`] means this structure, in this one file, came back
//! unchanged. It does not mean the field is correct, and the word "proven" is
//! deliberately absent from this enum: no single file can prove a field.
//!
//! The corpus fold over 44 ledgers is what produces the useful classification,
//! and it is a later segment's work:
//!
//! | Across 44 files | What it means |
//! |---|---|
//! | Differs in every one | A field this reader does not reproduce |
//! | Differs in some only | A field this reader models and gets wrong |
//! | Differs in none | Two statements of the layout agree |

use crate::read::region::Off;

/// A run of bytes at an absolute file offset.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Span {
    /// The absolute file offset of the first byte.
    pub at: Off,
    /// The number of bytes.
    pub len: u32,
}

impl Span {
    /// Builds a span.
    #[must_use]
    pub const fn new(at: Off, len: u32) -> Self {
        Self { at, len }
    }

    /// Gives the offset one byte past the run.
    ///
    /// Returns `None` when the sum leaves a `u32`.
    #[must_use]
    pub const fn end(self) -> Option<Off> {
        self.at.checked_add(self.len)
    }
}

/// What one run of bytes says about this reader, in one file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// The emitter wrote these bytes and they equal the file's own.
    Same,
    /// The emitter wrote these bytes and they do not equal the file's own.
    ///
    /// This is a fault in this reader. The file is not at fault.
    Differs,
    /// The emitter wrote nothing here.
    ///
    /// **This is wider than "the reader never read these bytes".** It means
    /// the bytes cannot be reproduced from what the model keeps, which also
    /// covers a field the reader reads, uses, and then discards.
    ///
    /// The first case measured was `lpszObjectName` at `Object + 0x18`:
    /// `crate::vb::object::ObjectTable::walk` resolved the name through that
    /// address and kept only the string, so four bytes the reader plainly
    /// read graded `Unmodelled`. `Object` now carries the address as well, so
    /// no structure graded today holds such a field.
    ///
    /// The distinction stays real for every structure not yet graded, and it
    /// caps what a coverage figure can claim: a reader that reads a field and
    /// does not keep it understands more than its coverage reports.
    Unmodelled,
}

/// One run of bytes and its verdict.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Run {
    /// Where the run is and how long it is.
    pub span: Span,
    /// What the run says about this reader.
    pub verdict: Verdict,
}

/// Every byte of one structure, at one place, in one file, graded.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ledger {
    /// The name of the structure, from `Emit::STRUCTURE`.
    pub structure: &'static str,
    /// The absolute file offset of the structure's first byte.
    pub base: Off,
    /// The length of the structure in bytes.
    pub len: u32,
    /// The runs, tiling `[base, base + len)`. See the module doc comment.
    pub runs: Vec<Run>,
}

impl Ledger {
    /// Gives every run carrying `verdict`.
    pub fn runs_with(&self, verdict: Verdict) -> impl Iterator<Item = &Run> {
        self.runs.iter().filter(move |run| run.verdict == verdict)
    }

    /// Gives the number of bytes carrying `verdict`.
    ///
    /// The fold saturates rather than wrapping. A wrapped total is a small
    /// number that reads as a good result.
    #[must_use]
    pub fn bytes_with(&self, verdict: Verdict) -> u32 {
        self.runs_with(verdict)
            .fold(0_u32, |total, run| total.saturating_add(run.span.len))
    }

    /// Tells whether no run differs.
    ///
    /// Never call this "proven". See the module doc comment.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.runs_with(Verdict::Differs).next().is_none()
    }

    /// Tells whether the runs tile the structure exactly.
    ///
    /// Ascending from `base`, no gap, no overlap, ending exactly at
    /// `base + len`, and no two neighbours sharing a verdict. A ledger that
    /// fails this has a fault in the grading, not in the file, and the corpus
    /// test asserts it on every structure of every program.
    #[must_use]
    pub fn tiles(&self) -> bool {
        let mut want = self.base;
        let mut previous: Option<Verdict> = None;
        for run in &self.runs {
            if run.span.at != want || run.span.len == 0 {
                return false;
            }
            if previous == Some(run.verdict) {
                return false;
            }
            let Some(end) = run.span.end() else {
                return false;
            };
            want = end;
            previous = Some(run.verdict);
        }
        self.base.checked_add(self.len) == Some(want)
    }
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

    use super::{Ledger, Run, Span, Verdict};
    use crate::read::region::Off;

    fn run(at: u32, len: u32, verdict: Verdict) -> Run {
        Run {
            span: Span::new(Off::new(at), len),
            verdict,
        }
    }

    fn ledger(base: u32, len: u32, runs: Vec<Run>) -> Ledger {
        Ledger {
            structure: "Test",
            base: Off::new(base),
            len,
            runs,
        }
    }

    #[test]
    fn a_span_gives_the_offset_one_byte_past_its_run() {
        assert_eq!(Span::new(Off::new(4), 6).end(), Some(Off::new(10)));
        assert_eq!(Span::new(Off::new(u32::MAX), 1).end(), None);
    }

    #[test]
    fn the_runs_tile_the_structure_with_no_gap_and_no_overlap_and_no_two_neighbours_agree() {
        let good = ledger(
            0x100,
            12,
            vec![
                run(0x100, 4, Verdict::Same),
                run(0x104, 4, Verdict::Unmodelled),
                run(0x108, 4, Verdict::Same),
            ],
        );
        assert!(good.tiles());
    }

    #[test]
    fn a_gap_between_two_runs_does_not_tile() {
        let gapped = ledger(
            0,
            12,
            vec![run(0, 4, Verdict::Same), run(8, 4, Verdict::Same)],
        );
        assert!(!gapped.tiles());
    }

    #[test]
    fn an_overlap_between_two_runs_does_not_tile() {
        let overlapping = ledger(
            0,
            8,
            vec![run(0, 4, Verdict::Same), run(2, 6, Verdict::Differs)],
        );
        assert!(!overlapping.tiles());
    }

    #[test]
    fn two_neighbours_carrying_the_same_verdict_do_not_tile() {
        // Uncoalesced runs would double count in the corpus fold, so this is
        // a fault in the grading and not a presentation detail.
        let uncoalesced = ledger(
            0,
            8,
            vec![run(0, 4, Verdict::Same), run(4, 4, Verdict::Same)],
        );
        assert!(!uncoalesced.tiles());
    }

    #[test]
    fn runs_that_stop_short_of_the_declared_length_do_not_tile() {
        let short = ledger(0, 12, vec![run(0, 8, Verdict::Same)]);
        assert!(!short.tiles());
    }

    #[test]
    fn a_zero_length_run_does_not_tile() {
        let empty = ledger(
            0,
            4,
            vec![run(0, 0, Verdict::Differs), run(0, 4, Verdict::Same)],
        );
        assert!(!empty.tiles());
    }

    #[test]
    fn bytes_with_totals_only_the_runs_that_carry_the_verdict() {
        let mixed = ledger(
            0,
            16,
            vec![
                run(0, 4, Verdict::Same),
                run(4, 6, Verdict::Unmodelled),
                run(10, 2, Verdict::Differs),
                run(12, 4, Verdict::Same),
            ],
        );
        assert_eq!(mixed.bytes_with(Verdict::Same), 8);
        assert_eq!(mixed.bytes_with(Verdict::Unmodelled), 6);
        assert_eq!(mixed.bytes_with(Verdict::Differs), 2);
        assert_eq!(mixed.runs_with(Verdict::Same).count(), 2);
    }

    #[test]
    fn a_ledger_with_no_differing_run_is_clean_and_one_with_a_differing_run_is_not() {
        let clean = ledger(
            0,
            8,
            vec![run(0, 4, Verdict::Same), run(4, 4, Verdict::Unmodelled)],
        );
        assert!(clean.is_clean());

        let dirty = ledger(
            0,
            8,
            vec![run(0, 4, Verdict::Same), run(4, 4, Verdict::Differs)],
        );
        assert!(!dirty.is_clean());
    }

    #[test]
    fn a_wholly_unmodelled_structure_is_clean_because_nothing_was_claimed() {
        // Clean is not a good result on its own. A structure this reader
        // models nothing of also reports clean, which is why the coverage
        // count is asserted beside it everywhere.
        let nothing = ledger(0, 8, vec![run(0, 8, Verdict::Unmodelled)]);
        assert!(nothing.is_clean());
        assert_eq!(nothing.bytes_with(Verdict::Same), 0);
    }
}
