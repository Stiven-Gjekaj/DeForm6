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

// --- Plan 04-06, Task 1: the path key --------------------------------------
//
// Research open question 1 asked what a run level path looks like, next to
// a path for a real recovered thing. The answer: a run level fact stays an
// item in the same flat array RPT-02 asks for, and takes the reserved
// [`META_PATH`] constant above. No separate top level field is added, so
// the flat array stays the one place a reader looks for every fact this
// run produced.
//
// Every path below is built by one function per shape, never by string
// concatenation at a call site, and every name segment comes from a
// `SafeName`, never from a raw recovered string: `SafeName::new` already
// proved a raw name holding a separator cannot survive sanitization, so a
// path built from one can never split into a different shape than the one
// its caller asked for.

use crate::write::model::{CodeKind, SafeName};

/// Builds the path for a form: `/forms/<name>`.
///
/// Every other path a form or a control inside one takes extends this one,
/// so a reader who greps for `/forms/` finds every fact this run holds
/// about every form, at the same root.
#[must_use]
pub fn path_for_form(form: &SafeName) -> String {
    format!("/forms/{}", form.as_str())
}

/// Extends [`path_for_form`] with the `controls` segment and `control`'s
/// own name: the exact shape the roadmap's own example gives for a
/// control inside a form, `/forms/frmMain/controls/cmdOk`.
#[must_use]
pub fn path_for_control(form: &SafeName, control: &SafeName) -> String {
    format!("{}/controls/{}", path_for_form(form), control.as_str())
}

/// Extends [`path_for_control`] with the `properties` segment and
/// `property`'s own name.
///
/// `property` is a plain string, not a [`SafeName`]: a property name
/// comes from the opcode table this run used, the crate's own built in
/// subset or one a user supplies, never from a raw string this crate
/// reads out of the hostile executable, so it has no recovered form that
/// needs sanitizing.
#[must_use]
pub fn path_for_property(form: &SafeName, control: &SafeName, property: &str) -> String {
    format!("{}/properties/{property}", path_for_control(form, control))
}

/// Gives the path segment naming a [`CodeKind`]'s own kind of object: the
/// segment [`path_for_code`] places before the object's own name.
///
/// No wildcard arm: a third [`CodeKind`] variant fails to compile here
/// until somebody decides its own segment word.
fn code_path_segment(kind: CodeKind) -> &'static str {
    match kind {
        CodeKind::Module => "modules",
        CodeKind::Class => "classes",
    }
}

/// Builds the path for a standard module or a class: a segment naming its
/// own kind, then its own name, per this task's own rule for an object
/// that is not a form.
#[must_use]
pub fn path_for_code(kind: CodeKind, name: &SafeName) -> String {
    format!("/{}/{}", code_path_segment(kind), name.as_str())
}

/// Issues a report item's own path, extending, never overwriting, a path
/// that collides with one already issued.
///
/// Holds every path issued so far in an ordered `Vec`, never a hash keyed
/// map, for the reason [`crate::write::model::SafeNameIssuer`] already
/// states for names: the suffix a collision gets depends on issue order,
/// and a process dependent iteration order would make the written report
/// vary run to run, which is exactly what RPT-05's determinism
/// requirement forbids.
#[derive(Default)]
pub struct PathIssuer {
    issued: Vec<String>,
}

impl PathIssuer {
    /// Starts an issuer with no path issued yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gives `path` unchanged the first time it is issued. A second call
    /// with the same `path` gets a `#` and a number appended, distinct
    /// from every path already issued, so the item that issued it first
    /// keeps its own path and the one that collided is never silently
    /// lost.
    #[must_use]
    pub fn issue(&mut self, path: String) -> String {
        if !self.issued.contains(&path) {
            self.issued.push(path.clone());
            return path;
        }
        let mut suffix: u32 = 2;
        loop {
            let candidate = format!("{path}#{suffix}");
            if !self.issued.contains(&candidate) {
                self.issued.push(candidate.clone());
                return candidate;
            }
            suffix = suffix.saturating_add(1);
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{
        Confidence, Evidence, META_PATH, PathIssuer, ProjectReport, ReportItem, path_for_code,
        path_for_control, path_for_form, path_for_property,
    };
    use crate::write::model::{CodeKind, NameKind, SafeName};

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

    // --- Plan 04-06, Task 1: the path key -----------------------------------

    #[test]
    fn a_control_inside_a_form_matches_the_roadmaps_own_example_path() {
        let (form, _faults) = SafeName::new("frmMain", NameKind::Form);
        let (control, _faults) = SafeName::new("cmdOk", NameKind::Control);
        assert_eq!(
            path_for_control(&form, &control),
            "/forms/frmMain/controls/cmdOk"
        );
    }

    #[test]
    fn a_property_on_a_control_extends_the_controls_path() {
        let (form, _faults) = SafeName::new("frmMain", NameKind::Form);
        let (control, _faults) = SafeName::new("cmdOk", NameKind::Control);
        let control_path = path_for_control(&form, &control);
        let property_path = path_for_property(&form, &control, "Caption");
        assert_eq!(property_path, format!("{control_path}/properties/Caption"));
    }

    #[test]
    fn every_path_segment_comes_from_a_sanitized_name_not_a_raw_one() {
        let (form, _faults) = SafeName::new("frmMain", NameKind::Form);
        let (control, faults) = SafeName::new("evil/name", NameKind::Control);
        assert!(!faults.is_empty(), "the separator must be sanitized away");
        assert!(!control.as_str().contains('/'));

        let path = path_for_control(&form, &control);
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        assert_eq!(
            segments.len(),
            4,
            "a sanitized control name must never split the path into more \
             segments than forms, frmMain, controls, and the one control \
             name: {path}"
        );
    }

    #[test]
    fn a_run_level_item_carries_the_reserved_path() {
        let item = ReportItem {
            path: META_PATH.to_owned(),
            confidence: Confidence::Inferred,
            basis: "a run level choice, not a fact about one object".to_owned(),
            evidence: Vec::new(),
        };
        assert_eq!(item.path, META_PATH);
    }

    #[test]
    fn a_class_path_and_a_module_path_each_name_their_own_kind() {
        let (class_name, _faults) = SafeName::new("CFire", NameKind::Class);
        let (module_name, _faults) = SafeName::new("Utility", NameKind::Module);
        assert_eq!(
            path_for_code(CodeKind::Class, &class_name),
            "/classes/CFire"
        );
        assert_eq!(
            path_for_code(CodeKind::Module, &module_name),
            "/modules/Utility"
        );
    }

    #[test]
    fn a_form_itself_gets_the_bare_forms_path() {
        let (form, _faults) = SafeName::new("frmMain", NameKind::Form);
        assert_eq!(path_for_form(&form), "/forms/frmMain");
    }

    #[test]
    fn two_items_that_would_collide_get_two_different_paths_and_both_are_kept() {
        let mut issuer = PathIssuer::new();
        let first = issuer.issue("/forms/frmMain".to_owned());
        let second = issuer.issue("/forms/frmMain".to_owned());
        assert_eq!(first, "/forms/frmMain");
        assert_ne!(first, second);
        assert!(second.starts_with("/forms/frmMain#"), "{second}");
    }
}
