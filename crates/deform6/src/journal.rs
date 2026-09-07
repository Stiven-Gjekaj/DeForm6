//! The one place that applies the strict policy or the salvage policy.
//!
//! A parse site does not know which mode the run is in. It builds a
//! [`Defect`], hands it to [`Journal::record`] with a fallback value, and uses
//! the `?` operator. The policy lives in `record` and nowhere else. Nothing
//! else in this crate reads a [`Mode`].
//!
//! Two properties follow from that.
//!
//! The journal pushes the defect before it applies the policy, so a run that
//! refuses still holds the evidence of what it saw. The failure message and
//! the Phase 4 report read the same value.
//!
//! A hostile file cannot reach a salvage path in a strict run, because no
//! parse site holds a branch on the mode to reach it with.
//!
//! [`Mode::Salvage`] has no command line route in Phase 1. The `--salvage`
//! flag arrives in Phase 5. The arm is written and tested now, because
//! threading a flag through every parse site later costs more than writing the
//! choke point once.

use crate::error::{Defect, Error, Severity};

/// How much damage a run accepts before it refuses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Any defect refuses the file. This is the default in Phase 1.
    Strict,
    /// A recoverable defect gives a fallback and the run continues.
    Salvage,
}

/// What the run saw, and the policy it applies to what it saw.
#[derive(Clone, Debug)]
pub struct Journal {
    mode: Mode,
    defects: Vec<Defect>,
}

impl Journal {
    /// Start a journal that applies `mode`.
    #[must_use]
    pub const fn new(mode: Mode) -> Self {
        Self {
            mode,
            defects: Vec::new(),
        }
    }

    /// Every defect the run recorded, in the order it recorded them.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }

    /// Record a defect. Return the fallback, or refuse.
    ///
    /// `Fatal` refuses in both modes. `Recoverable` refuses in
    /// [`Mode::Strict`] and gives back `fallback` in [`Mode::Salvage`].
    ///
    /// The push happens before the match. That is what makes the defect
    /// present on the path that refuses as well as on the path that
    /// continues.
    pub fn record<T>(&mut self, defect: Defect, fallback: T) -> Result<T, Error> {
        self.defects.push(defect.clone());
        match (defect.kind.severity(), self.mode) {
            (Severity::Fatal, _) => Err(Error::Refused(defect)),
            (Severity::Recoverable, Mode::Strict) => Err(Error::Refused(defect)),
            (Severity::Recoverable, Mode::Salvage) => Ok(fallback),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{Journal, Mode};
    use crate::error::{Defect, DefectKind, Site};

    /// The value a salvage run gives back in place of the item it lost.
    const FALLBACK: u32 = 4242;

    fn a_site() -> Site {
        Site {
            offset: 0x1000,
            rva: None,
            structure: "VbHeader",
            field: "lpProjectData",
        }
    }

    /// A defect whose kind is fatal.
    fn a_fatal_defect() -> Defect {
        Defect {
            site: a_site(),
            kind: DefectKind::BadMagic {
                offset: 0x1760,
                expected: "VB5!",
                found: 0,
            },
        }
    }

    /// A defect whose kind is recoverable, at the offset the caller names.
    fn a_recoverable_defect_at(offset: u32) -> Defect {
        Defect {
            site: a_site(),
            kind: DefectKind::SectionOverlap {
                offset,
                other: 0x0400,
            },
        }
    }

    fn a_recoverable_defect() -> Defect {
        a_recoverable_defect_at(0x0600)
    }

    #[test]
    fn a_fatal_defect_refuses_in_strict_mode() {
        let mut journal = Journal::new(Mode::Strict);
        let outcome = journal.record(a_fatal_defect(), FALLBACK);
        assert!(
            outcome.is_err(),
            "a fatal defect must refuse in strict mode: {outcome:?}"
        );
    }

    #[test]
    fn a_fatal_defect_refuses_in_salvage_mode_as_well() {
        let mut journal = Journal::new(Mode::Salvage);
        let outcome = journal.record(a_fatal_defect(), FALLBACK);
        assert!(
            outcome.is_err(),
            "fatal always refuses, in both modes, and salvage does not soften it: {outcome:?}"
        );
    }

    #[test]
    fn a_recoverable_defect_refuses_in_strict_mode() {
        let mut journal = Journal::new(Mode::Strict);
        let outcome = journal.record(a_recoverable_defect(), FALLBACK);
        assert!(
            outcome.is_err(),
            "strict mode refuses every defect: {outcome:?}"
        );
    }

    #[test]
    fn a_recoverable_defect_gives_back_the_fallback_in_salvage_mode() {
        let mut journal = Journal::new(Mode::Salvage);
        let outcome = journal.record(a_recoverable_defect(), FALLBACK);
        assert!(
            outcome.is_ok(),
            "salvage mode continues past a recoverable defect: {outcome:?}"
        );
        assert_eq!(
            outcome.ok(),
            Some(FALLBACK),
            "the value that comes back must be the fallback that went in"
        );
    }

    #[test]
    fn a_defect_is_recorded_in_all_four_cases() {
        for mode in [Mode::Strict, Mode::Salvage] {
            for defect in [a_fatal_defect(), a_recoverable_defect()] {
                let wanted = format!("{}", defect.kind);
                let mut journal = Journal::new(mode);
                let _ = journal.record(defect, FALLBACK);
                assert_eq!(
                    journal.defects().len(),
                    1,
                    "a run in {mode:?} must hold the defect it saw, \
                     including the run that refuses"
                );
                assert_eq!(
                    format!("{}", journal.defects()[0].kind),
                    wanted,
                    "the journal must hold the defect that was recorded"
                );
            }
        }
    }

    #[test]
    fn two_defects_appear_in_the_order_they_were_recorded() {
        let mut journal = Journal::new(Mode::Salvage);
        let _ = journal.record(a_recoverable_defect_at(0x11), FALLBACK);
        let _ = journal.record(a_recoverable_defect_at(0x22), FALLBACK);
        assert_eq!(journal.defects().len(), 2, "both defects must be present");
        let first = format!("{}", journal.defects()[0].kind);
        let second = format!("{}", journal.defects()[1].kind);
        assert!(
            first.contains("0x11"),
            "the first defect recorded must be the first in the journal: {first}"
        );
        assert!(
            second.contains("0x22"),
            "the second defect recorded must be the second in the journal: {second}"
        );
    }
}
