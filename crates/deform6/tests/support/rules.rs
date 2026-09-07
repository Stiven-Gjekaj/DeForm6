#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "this is the test harness, not the library under test: it reads a vendored, \
              fixed corpus this repository controls, so the strict input-hostility \
              discipline `src/` carries does not apply (the threat model's T-02-35 accepts \
              this, because the harness is not exposed to hostile input the way the parser \
              reading a real VB6 executable is)"
)]

//! The written exclusion rules VER-04 requires: what the compiler does
//! not keep, expressed as data, never as a branch that names a program.
//!
//! Every rule here is an identifier, a one-sentence reason, and a
//! predicate over a structural fact ([`Candidate`]). No predicate ever
//! compares against a source tree path, a program name, or anything
//! else that would let one rule quietly become a per-program allowance.
//! A test in `support_selftest.rs` proves each rule against a real,
//! named case; this file stays generic on purpose so that a change
//! here cannot smuggle a program-specific exception past review.

use std::collections::HashMap;

/// The visibility a Visual Basic procedure declaration carries. VB6
/// treats an absent modifier as `Public`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Friend,
}

/// One structural fact a rule decides whether to exclude from a
/// differential comparison. The shape stays generic: nothing here
/// carries a source tree path or a program name, so `Rule::excludes`
/// cannot become a branch that names one.
#[derive(Debug, Clone, Copy)]
pub enum Candidate {
    /// A procedure the compiled object table could in principle name,
    /// tagged with whether its declaring object is a standard module
    /// and the visibility its own source line carries.
    Procedure {
        in_standard_module: bool,
        visibility: Visibility,
    },
    /// A source file found under a project's own directory, tagged
    /// with whether the project file's own key list references it and
    /// whether the repository holds it on disk.
    SourceListing {
        declared_in_project: bool,
        exists_on_disk: bool,
    },
    /// Something no recoverable structure carries under any
    /// circumstance: a local variable name, a comment, source
    /// formatting, or a private name. `PROJECT.md`'s "Out of Scope"
    /// list names these directly.
    NeverKept,
}

/// One written exclusion rule: what it drops from a comparison, and
/// why.
pub struct Rule {
    pub id: &'static str,
    pub reason: &'static str,
    predicate: fn(Candidate) -> bool,
}

impl Rule {
    /// Says whether this rule drops the given candidate.
    #[must_use]
    pub fn excludes(&self, candidate: Candidate) -> bool {
        (self.predicate)(candidate)
    }
}

/// Applies `rule` to both the declared side and the recovered side of a
/// comparison, from one evaluation.
///
/// Per VER-04, the absent-source rule must exclude its match from both
/// sides, never from one side only: dropping a candidate from the
/// expectation while still counting it in what was recovered would make
/// the recovered count exceed the declared count, which looks like
/// over-recovery rather than the exclusion bug it actually is.
/// Returning one evaluation twice, rather than two separate calls a
/// future caller could let drift apart, is what keeps this symmetric by
/// construction.
#[must_use]
pub fn apply_symmetrically(rule: &Rule, candidate: Candidate) -> (bool, bool) {
    let excluded = rule.excludes(candidate);
    (excluded, excluded)
}

fn standard_module_predicate(candidate: Candidate) -> bool {
    matches!(
        candidate,
        Candidate::Procedure {
            in_standard_module: true,
            ..
        }
    )
}

fn scope_predicate(candidate: Candidate) -> bool {
    match candidate {
        Candidate::Procedure {
            in_standard_module: false,
            visibility,
        } => visibility != Visibility::Public,
        _ => false,
    }
}

fn unlisted_source_predicate(candidate: Candidate) -> bool {
    matches!(
        candidate,
        Candidate::SourceListing {
            declared_in_project: false,
            exists_on_disk: true,
        }
    )
}

fn absent_source_predicate(candidate: Candidate) -> bool {
    matches!(
        candidate,
        Candidate::SourceListing {
            declared_in_project: true,
            exists_on_disk: false,
        }
    )
}

fn never_kept_predicate(candidate: Candidate) -> bool {
    matches!(candidate, Candidate::NeverKept)
}

/// The five written exclusion rules VER-04 requires.
///
/// Every rule is proved, in `support_selftest.rs`, against at least one
/// real, named case; [`tally`] is the generic instrument that test
/// leans on to prove no rule matches zero cases.
pub const RULES: &[Rule] = &[
    Rule {
        id: "standard-module",
        reason: "A standard module carries no procedure names. Type data is part of the \
                  dispatch plumbing every form, class and user control carries and the \
                  compiler cannot strip, and a standard module is not such an object; its \
                  procedure name array pointer is null outright, so its procedures are \
                  name-less through the object table rather than merely prototype-less.",
        predicate: standard_module_predicate,
    },
    Rule {
        id: "scope",
        reason: "Only a public procedure survives by name. A procedure the source declares \
                  private or friend produces a null entry in the name array, which is \
                  correct behaviour and not a shortfall.",
        predicate: scope_predicate,
    },
    Rule {
        id: "unlisted-source",
        reason: "An object the project file does not list was never compiled in, even when \
                  its source file sits on disk.",
        predicate: unlisted_source_predicate,
    },
    Rule {
        id: "absent-source",
        reason: "An object the project file lists whose source the repository does not hold \
                  has no source-side expectation, so it is dropped from both sides of the \
                  comparison rather than counted as a shortfall on one side only.",
        predicate: absent_source_predicate,
    },
    Rule {
        id: "never-kept",
        reason: "A local variable name, a private procedure's name, a comment and source \
                  formatting are never kept by the compiled binary under any circumstance, so \
                  no rule needs to recover them.",
        predicate: never_kept_predicate,
    },
];

/// Tallies, for each rule in [`RULES`], how many of `candidates` it
/// excludes.
///
/// This is the instrument that keeps the rule set honest: a rule whose
/// tally stays zero is either wrong or no longer needed, and either way
/// somebody has to look at it before it ships. The candidates
/// themselves come from the caller; this function knows nothing about
/// where they came from, which is what keeps it free of any program
/// name.
#[must_use]
pub fn tally(candidates: impl IntoIterator<Item = Candidate>) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = RULES.iter().map(|r| (r.id, 0)).collect();
    for candidate in candidates {
        for rule in RULES {
            if rule.excludes(candidate) {
                *counts.entry(rule.id).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// The keywords that open a Visual Basic procedure declaration line,
/// after any visibility modifier and any `Static` modifier are removed.
const PROCEDURE_KEYWORDS: &[&str] = &[
    "Sub",
    "Function",
    "Property Get",
    "Property Let",
    "Property Set",
];

/// Scans a source file's whole text for every procedure declaration it
/// opens, giving each one's visibility.
///
/// A `Sub`, a `Function`, a `Property Get`/`Let`/`Set`, and a `Declare`
/// each consume one slot in the compiled object's procedure array: a
/// `Private Declare Function` in a corpus form still counts toward that
/// form's `ProcCount`, exactly as a `Private Sub` does. VB6 treats an
/// absent visibility modifier as `Public`.
#[must_use]
pub fn scan_procedure_visibilities(text: &str) -> Vec<Visibility> {
    let mut out = Vec::new();
    for line in text.lines() {
        let s = line.trim();
        let mut tokens = s.split_whitespace();
        let Some(first) = tokens.clone().next() else {
            continue;
        };
        let visibility = match first {
            "Public" => {
                tokens.next();
                Visibility::Public
            }
            "Private" => {
                tokens.next();
                Visibility::Private
            }
            "Friend" => {
                tokens.next();
                Visibility::Friend
            }
            _ => Visibility::Public,
        };
        let rest: Vec<&str> = tokens.collect();
        let Some(&next) = rest.first() else {
            continue;
        };
        let rest = if next == "Static" {
            &rest[1..]
        } else {
            &rest[..]
        };
        let joined = rest.join(" ");

        if joined.starts_with("Declare ") {
            out.push(visibility);
            continue;
        }
        for keyword in PROCEDURE_KEYWORDS {
            if joined == *keyword || joined.starts_with(&format!("{keyword} ")) {
                out.push(visibility);
                break;
            }
        }
    }
    out
}
