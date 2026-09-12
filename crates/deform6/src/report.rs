//! The confidence report: [`ProjectReport`], [`ReportItem`], [`Confidence`]
//! and [`Evidence`], locked in this plan's first commit so that plans
//! 04-02 through 04-05 can build report items in wave 2 while plan 04-06
//! completes the builder in the same wave, without either widening the
//! other's file.
//!
//! [`ProjectReport::to_json`] is the one place this crate builds JSON
//! text, and it is always `serde_json::to_string_pretty` over a value that
//! derives `serde::Serialize`, never hand assembled: `04-RESEARCH.md`'s own
//! "Don't Hand-Roll" table names manual JSON string building as exactly
//! the deceptively complex text problem a real library exists to own.

use crate::error::Defect;

/// The reserved path a run level report item takes: a fact that belongs to
/// no object, no form and no control, such as which opcode table subset
/// this run used. RPT-02's own flat array stays the one place a reader
/// looks for every fact this run produced, so a run level fact is an item
/// with this path, not a separate top level field.
pub const META_PATH: &str = "/meta";

/// The whole confidence report one run of [`crate::write::project`]
/// produces.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ProjectReport {
    /// Every report item this run produced, in the order the writers
    /// built them. Never a hash keyed map: iteration order decides the
    /// bytes of the written JSON, and this crate's own map type
    /// randomises it per process.
    pub items: Vec<ReportItem>,
    /// Every defect `crate::vb::inspect` collected, carried over from
    /// [`crate::vb::Report::defects`] unchanged.
    pub defects: Vec<Defect>,
    /// Every limit this run reached or came close to, such as a name
    /// clamped to [`crate::write::model::MAX_NAME_LEN`] or a control tree
    /// nested to [`crate::write::model::MAX_NESTING_DEPTH`]. Plan 04-06
    /// owns populating this list; it is empty until then.
    pub limits: Vec<String>,
}

impl ProjectReport {
    /// Serialises this report as pretty printed JSON.
    ///
    /// Always `serde_json::to_string_pretty` over `self`, a value whose
    /// own `Serialize` derive walks its fields in source order, never a
    /// hand-built string. Gives an empty string on the failure this call
    /// cannot reach in practice (every field here is plain data with a
    /// working `Serialize` impl), rather than panicking on a value this
    /// crate itself built.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// One fact this run recovered, graded by how sure it is, and where in
/// the project it belongs.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ReportItem {
    /// The path this item belongs to, such as
    /// `/forms/frmMain/controls/cmdOk`, or [`META_PATH`] for a run level
    /// fact.
    pub path: String,
    /// How sure this repository is of the fact this item names.
    pub confidence: Confidence,
    /// The one sentence stating why this item carries the confidence it
    /// does.
    pub basis: String,
    /// Every piece of evidence this item rests on, in the order it was
    /// found.
    pub evidence: Vec<Evidence>,
}

/// How sure this repository is of one recovered fact.
///
/// Exactly three variants, serialising to the three lower case words a
/// reader of the JSON report sees. A `match` a caller writes over this
/// enum should carry no wildcard arm: a fourth variant would then be a
/// compile error until every caller decides what it means, not a silent
/// default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// The exact byte this repository read names the fact directly.
    Proven,
    /// This repository chose the fact because the file gives no other
    /// answer, and the basis says so.
    Inferred,
    /// This repository could not recover the fact at all.
    Unrecoverable,
}

/// One piece of evidence a [`ReportItem`] rests on: a byte offset, the
/// structure and field it came from, and an optional note.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Evidence {
    /// The absolute file offset the evidence was read from.
    pub offset: u32,
    /// The structure the evidence came from, such as `"GuiObjectInfo"`.
    pub structure: &'static str,
    /// The field the evidence came from, such as `"lPropertiesLength"`.
    pub field: &'static str,
    /// An optional note giving context a byte offset alone does not
    /// carry.
    pub note: Option<String>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{Confidence, Evidence, META_PATH, ProjectReport, ReportItem};

    #[test]
    fn confidence_serialises_to_the_three_lower_case_words() {
        assert_eq!(
            serde_json::to_string(&Confidence::Proven).unwrap(),
            "\"proven\""
        );
        assert_eq!(
            serde_json::to_string(&Confidence::Inferred).unwrap(),
            "\"inferred\""
        );
        assert_eq!(
            serde_json::to_string(&Confidence::Unrecoverable).unwrap(),
            "\"unrecoverable\""
        );
    }

    #[test]
    fn to_json_round_trips_a_report_with_one_item_and_no_defects() {
        let report = ProjectReport {
            items: vec![ReportItem {
                path: META_PATH.to_owned(),
                confidence: Confidence::Inferred,
                basis: "the file does not declare a startup form".to_owned(),
                evidence: vec![Evidence {
                    offset: 0x10,
                    structure: "GuiTable",
                    field: "entries",
                    note: None,
                }],
            }],
            defects: Vec::new(),
            limits: Vec::new(),
        };
        let json = report.to_json();
        assert!(json.contains("\"inferred\""));
        assert!(json.contains(META_PATH));
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("to_json must produce valid JSON");
        assert!(parsed.is_object());
    }
}
