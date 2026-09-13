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

use crate::error::{Defect, Severity};
use crate::journal::Mode;

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

// --- Plan 04-06, Task 2: the three words, the basis, the evidence and the
// defect array ---------------------------------------------------------

use crate::vb::Report;
use crate::vb::propstream::PropertyValue;
use crate::write::model::ProjectModel;

/// Gives the path segment one property's own item takes: its own
/// recovered name, when it has one, or `opcode<N>` for a
/// [`PropertyValue::Undecoded`] value, which carries no name at all.
///
/// No wildcard arm: a variant this crate adds later fails to compile here
/// until somebody decides its own segment word.
fn property_word(property: &PropertyValue) -> String {
    match property {
        PropertyValue::Byte { name, .. }
        | PropertyValue::Boolean { name, .. }
        | PropertyValue::Integer { name, .. }
        | PropertyValue::Long { name, .. }
        | PropertyValue::Single { name, .. }
        | PropertyValue::Text { name, .. }
        | PropertyValue::Position { name, .. }
        | PropertyValue::Font { name, .. }
        | PropertyValue::Blob { name, .. }
        | PropertyValue::BlobUnreadable { name, .. } => name.clone(),
        PropertyValue::Undecoded { opcode, .. } => format!("opcode{opcode}"),
    }
}

/// Builds the report item one property earns, when it earns one at all.
///
/// A resource blob this repository read earns [`Confidence::Proven`]: its
/// own bytes were read at its own byte offset, by a named rule
/// (`crate::vb::frx::extract_blob`). A resource blob this repository
/// could not read, and an opcode this repository names no decoder for,
/// both earn [`Confidence::Unrecoverable`], reusing the item
/// [`crate::write::values::format_value`] already builds for them, with
/// its own real offset unchanged: this is a field assignment, not a
/// second decision about the same fact. Every other property value is
/// written directly, in full, into the project this run writes; it earns
/// no item of its own.
///
/// No wildcard arm: a variant this crate adds later fails to compile here
/// until somebody decides whether it earns an item.
///
/// The item's own `path` is always empty; the caller fills in the real
/// path, the same convention [`crate::write::values::format_value`]
/// already uses.
fn item_for_property(property: &PropertyValue) -> Option<ReportItem> {
    match property {
        PropertyValue::Blob { name, offset, .. } => Some(ReportItem {
            path: String::new(),
            confidence: Confidence::Proven,
            basis: format!(
                "the {name} property's own resource blob was read at its own byte \
                 offset in the executable"
            ),
            evidence: vec![Evidence {
                offset: *offset,
                structure: "PropertyValue",
                field: "Blob",
                note: None,
            }],
        }),
        PropertyValue::BlobUnreadable { .. } | PropertyValue::Undecoded { .. } => {
            crate::write::values::format_value(property, false).1
        }
        PropertyValue::Byte { .. }
        | PropertyValue::Boolean { .. }
        | PropertyValue::Integer { .. }
        | PropertyValue::Long { .. }
        | PropertyValue::Single { .. }
        | PropertyValue::Text { .. }
        | PropertyValue::Position { .. }
        | PropertyValue::Font { .. } => None,
    }
}

/// Backfills one [`Evidence`] record, anchored at the executable's own
/// project header, for an item that arrived with none.
///
/// A run level or a whole object choice this crate makes has no byte of
/// its own to point at, and [`Report::header_offset`] is a real,
/// already-read offset every [`Report`] carries: phase 1 measured it
/// directly out of the file, so this function never computes a new
/// offset, it only anchors the item to one the reading side already
/// recorded.
///
/// `pub(crate)`, not private: plan 04-08's `write::project` calls this
/// directly on every item the individual writers (`write_vbp`,
/// `write_form`, `write_cls`, `write_bas`) collect on their own, not only
/// on the `model_items` this file's own [`build`] walks, so that every
/// item in the shipped report carries evidence, per RPT-04.
pub(crate) fn with_header_evidence(mut item: ReportItem, report: &Report) -> ReportItem {
    if item.evidence.is_empty() {
        item.evidence.push(Evidence {
            offset: report.header_offset.get(),
            structure: "VbHeader",
            field: "header_offset",
            note: Some(
                "this choice has no byte of its own; the offset anchors to the \
                 executable's own project header"
                    .to_owned(),
            ),
        });
    }
    item
}

/// Builds the limits list every run states in plain words.
///
/// `opcode_table_summary` is the one line naming which opcode table this
/// run used and how many entries it holds, the same summary
/// `deform6-cli`'s own `load_opcode_table` already builds. This function
/// does not choose that wording a second time; it takes the caller's own
/// summary of the table this run actually used, so the report never
/// disagrees with the line the command line already printed.
///
/// `mode` names the run's own policy first, before the four lines every
/// run already stated: a strict run says it assumed nothing, because
/// [`crate::journal::Journal::record`] already refused before this
/// function could ever see a `Recoverable` defect in `defects`; a salvage
/// run says it continued past one, and every assumption line
/// [`assumption_lines`] builds follows immediately after. Neither run's
/// line claims more than [`Mode`] itself decided.
///
/// The second line states, in plain words, that full recompilation did not
/// run: it needs the Visual Basic 6 IDE on Windows, this run had neither,
/// and a structural check ran in its place. It never claims the IDE
/// opened this project, the one claim the roadmap's own named risk
/// forbids.
fn build_limits(mode: Mode, defects: &[Defect], opcode_table_summary: &str) -> Vec<String> {
    let mode_line = match mode {
        Mode::Strict => "This is a strict run. It refused any file whose read had to \
             assume a value, so this run assumed none."
            .to_owned(),
        Mode::Salvage => "This is a salvage run. It continued past a defect the read had \
             to assume a value for, and every assumption it made is listed \
             below."
            .to_owned(),
    };

    let mut limits = vec![
        mode_line,
        "Full recompilation did not run. It needs the Visual Basic 6 IDE on \
         Windows, and this run had neither. A structural check ran in its \
         place, and it never opened this project in the IDE."
            .to_owned(),
        opcode_table_summary.to_owned(),
        "Every inline string this run wrote inline holds 97 encoded bytes \
         or fewer, the proven floor of an open threshold. A longer string \
         became a resource reference instead of a guessed cutoff."
            .to_owned(),
        "This run assumes a Western code page. Every character above \
         U+00FF was replaced with a question mark and reported, never \
         guessed at a different code page."
            .to_owned(),
    ];
    limits.extend(assumption_lines(defects));
    limits
}

/// One line per `Recoverable` defect, naming the assumption a salvage run
/// made in its place.
///
/// Walks `defects` in the order they were recorded (the same order
/// [`crate::journal::Journal::record`] pushed them in, before it ever
/// applied a policy, per that function's own doc comment), and keeps only
/// the ones whose severity is [`Severity::Recoverable`]: a `Tolerated`
/// defect costs one item and the run assumed nothing in its place, so it
/// earns no line here. A `Fatal` defect never reaches this function at
/// all, because [`crate::vb::inspect`] already refused before `defects`
/// was ever assembled.
///
/// This never filters [`ProjectReport::defects`] itself: RPT-05 requires
/// every defect the run met, `Tolerated` included, and this function only
/// adds lines to [`ProjectReport::limits`].
fn assumption_lines(defects: &[Defect]) -> Vec<String> {
    defects
        .iter()
        .filter(|defect| defect.kind.severity() == Severity::Recoverable)
        .map(|defect| {
            format!(
                "Assumed at offset {:#x} ({}.{}): {}",
                defect.site.offset, defect.site.structure, defect.site.field, defect.kind
            )
        })
        .collect()
}

/// Builds the complete [`ProjectReport`] one run of
/// [`crate::write::project`] produces.
///
/// Walks `model_items` (from [`crate::write::model::from_report`]) first,
/// backfilling evidence for any item that arrived with none, then walks
/// every form's own controls in [`ProjectModel::forms`] order and every
/// control's own properties in stream order, adding one item for every
/// property that earns one. The defect array is attached whole, from
/// [`Report::defects`], never filtered: a run that continued past a
/// defect still reports it, per RPT-05. `opcode_table_summary` and `mode`
/// flow straight into [`build_limits`], which is also where `mode` earns
/// its one assumption line per `Recoverable` defect.
///
/// # Determinism
///
/// Every collection this function walks is already an ordered `Vec`, in
/// [`Report`]'s or [`ProjectModel`]'s own recovered order, and every path
/// this function issues comes from one [`PathIssuer`], so two calls over
/// the same `report`, `model`, `model_items`, `opcode_table_summary` and
/// `mode` give the same [`ProjectReport`], field for field.
#[must_use]
pub fn build(
    report: &Report,
    model: &ProjectModel,
    model_items: Vec<ReportItem>,
    opcode_table_summary: &str,
    mode: Mode,
) -> ProjectReport {
    let mut paths = PathIssuer::new();
    let mut items = Vec::with_capacity(model_items.len());

    for item in model_items {
        let item = with_header_evidence(item, report);
        let path = paths.issue(item.path.clone());
        items.push(ReportItem { path, ..item });
    }

    for form in &model.forms {
        for control in &form.controls {
            for property in &control.properties {
                if let Some(item) = item_for_property(property) {
                    let path = paths.issue(path_for_property(
                        &form.name,
                        &control.name,
                        &property_word(property),
                    ));
                    items.push(ReportItem { path, ..item });
                }
            }
        }
    }

    ProjectReport {
        items,
        defects: report.defects.clone(),
        limits: build_limits(mode, &report.defects, opcode_table_summary),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{
        Confidence, Evidence, META_PATH, Mode, PathIssuer, ProjectReport, ReportItem, build,
        build_limits, item_for_property, path_for_code, path_for_control, path_for_form,
        path_for_property, property_word, with_header_evidence,
    };
    use crate::error::{Defect, DefectKind, Site};
    use crate::read::region::Off;
    use crate::vb::controltree::ControlKind;
    use crate::vb::opcodes::OpcodeTable;
    use crate::vb::propstream::PropertyValue;
    use crate::vb::runtime::Runtime;
    use crate::vb::{ControlReport, FormReport, ObjectReport, Report};
    use crate::write::model::{CodeKind, NameKind, SafeName};

    /// `corpus/vb6-code/Fire-effect/Fast_Flames.exe`, this task's own
    /// query and determinism program: the same corpus program
    /// `extract_tracer.rs` already exercises end to end.
    const FAST_FLAMES: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Fire-effect/Fast_Flames.exe"
    ));

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

    // --- Plan 04-06, Task 2: the three words, the basis, the evidence and
    // the defect array -------------------------------------------------

    /// Builds a minimal, valid [`Report`], the way
    /// `write::model::tests::minimal_report` does: every field this
    /// module's own building never reads carries a literal, per
    /// `AGENTS.md`'s "build the state a test needs inside the test".
    fn minimal_report(
        objects: Vec<ObjectReport>,
        forms: Vec<FormReport>,
        defects: Vec<Defect>,
    ) -> Report {
        Report {
            file_len: 0,
            section_count: 0,
            runtime: Runtime::Vb6,
            runtime_dll: "MSVBVM60.DLL".to_owned(),
            signature: *b"VB5!",
            header_offset: Off::new(0x40),
            runtime_build: 0,
            project_name: "TestProject".to_owned(),
            title: String::new(),
            exe_name: String::new(),
            help_file: String::new(),
            native: true,
            object_count: u16::try_from(objects.len()).unwrap_or(0),
            objects,
            declarations: Vec::new(),
            components: Vec::new(),
            forms,
            defects,
        }
    }

    /// Builds a minimal [`ControlReport`] naming `name` and `kind`, with
    /// `properties` and no other optional field.
    fn control_with_properties(
        name: &str,
        kind: ControlKind,
        parent: Option<usize>,
        properties: Vec<PropertyValue>,
    ) -> ControlReport {
        ControlReport {
            name: name.to_owned(),
            kind,
            array_index: None,
            parent,
            properties,
            external: None,
            external_reason: None,
            ocx_header: None,
            opaque_message: None,
            events: Vec::new(),
        }
    }

    fn a_blob(name: &str, offset: u32) -> PropertyValue {
        PropertyValue::Blob {
            name: name.to_owned(),
            offset,
            declared_len: 20,
            image_len: 12,
            format: crate::vb::frx::ImageFormat::Unknown(Vec::new()),
            frx_offset: 0,
        }
    }

    fn an_undecoded(opcode: u8, offset: u32, control_type: &str) -> PropertyValue {
        PropertyValue::Undecoded {
            opcode,
            offset,
            control_type: control_type.to_owned(),
            bytes_not_read: 4,
        }
    }

    #[test]
    fn a_blob_property_earns_confidence_proven_with_its_own_offset() {
        let property = a_blob("Icon", 0x300);
        let item = item_for_property(&property).expect("a blob must earn an item");
        assert_eq!(item.confidence, Confidence::Proven);
        assert_eq!(item.evidence.len(), 1);
        assert_eq!(item.evidence[0].offset, 0x300);
        assert!(!item.basis.is_empty());
    }

    #[test]
    fn an_undecoded_property_earns_confidence_unrecoverable_with_its_own_offset() {
        let property = an_undecoded(9, 0x310, "Form");
        let item = item_for_property(&property).expect("an undecoded property must earn an item");
        assert_eq!(item.confidence, Confidence::Unrecoverable);
        assert_eq!(item.evidence.len(), 1);
        assert_eq!(item.evidence[0].offset, 0x310);
    }

    #[test]
    fn a_decoded_scalar_property_earns_no_item_of_its_own() {
        let property = PropertyValue::Boolean {
            name: "Visible".to_owned(),
            value: -1,
        };
        assert!(item_for_property(&property).is_none());
    }

    #[test]
    fn property_word_gives_the_recovered_name_for_a_named_property() {
        let property = a_blob("Icon", 0x10);
        assert_eq!(property_word(&property), "Icon");
    }

    #[test]
    fn property_word_uses_the_opcode_number_for_an_undecoded_property() {
        let property = an_undecoded(7, 0x10, "Form");
        assert_eq!(property_word(&property), "opcode7");
    }

    #[test]
    fn with_header_evidence_backfills_only_when_the_item_arrives_with_none() {
        let report = minimal_report(Vec::new(), Vec::new(), Vec::new());
        let empty = ReportItem {
            path: META_PATH.to_owned(),
            confidence: Confidence::Inferred,
            basis: "a run level choice with no byte of its own".to_owned(),
            evidence: Vec::new(),
        };
        let filled = with_header_evidence(empty, &report);
        assert_eq!(filled.evidence.len(), 1);
        assert_eq!(filled.evidence[0].offset, report.header_offset.get());

        let already = ReportItem {
            evidence: vec![Evidence {
                offset: 0x99,
                structure: "X",
                field: "Y",
                note: None,
            }],
            ..filled.clone()
        };
        let unchanged = with_header_evidence(already.clone(), &report);
        assert_eq!(unchanged.evidence, already.evidence);
    }

    #[test]
    fn build_attaches_the_defect_array_whole_and_never_filters_it() {
        let defect = Defect {
            site: Site {
                offset: 0x10,
                rva: None,
                structure: "GuiObjectInfo",
                field: "lPropertiesLength",
            },
            kind: DefectKind::StructureUnreadable {
                offset: 0x10,
                reason: "synthetic, built inside the test".to_owned(),
            },
        };
        let report = minimal_report(Vec::new(), Vec::new(), vec![defect.clone(), defect]);
        let (model, items) = crate::write::model::from_report(&report, &[]);
        let built = build(
            &report,
            &model,
            items,
            "Opcode table: test fixture, 0 entries",
            Mode::Strict,
        );
        assert_eq!(built.defects.len(), report.defects.len());
        assert_eq!(built.defects, report.defects);
    }

    #[test]
    fn every_item_in_a_built_report_has_a_non_empty_basis_and_at_least_one_evidence_record() {
        let root = control_with_properties("frmPics", ControlKind::Form, None, Vec::new());
        let child = control_with_properties(
            "Picture1",
            ControlKind::PictureBox,
            Some(0),
            vec![a_blob("Icon", 0x200), an_undecoded(7, 0x220, "PictureBox")],
        );
        let form = FormReport {
            name: "frmPics".to_owned(),
            controls: vec![root, child],
            defects: Vec::new(),
        };
        let report = minimal_report(Vec::new(), vec![form], Vec::new());
        let (model, items) = crate::write::model::from_report(&report, &[]);
        let built = build(
            &report,
            &model,
            items,
            "Opcode table: test fixture, 0 entries",
            Mode::Strict,
        );

        assert!(!built.items.is_empty());
        for item in &built.items {
            assert!(!item.basis.is_empty(), "{item:?}");
            assert!(!item.evidence.is_empty(), "{item:?}");
        }

        assert!(
            built
                .items
                .iter()
                .any(|item| item.confidence == Confidence::Proven),
            "{:?}",
            built.items
        );
        assert!(
            built
                .items
                .iter()
                .any(|item| item.confidence == Confidence::Unrecoverable
                    && item.basis.contains("opcode 7")),
            "{:?}",
            built.items
        );
    }

    // --- Plan 04-06, Task 3: byte identical across two runs, and the
    // limits stated in the file ------------------------------------------

    /// Runs `inspect`, builds the model and the report over `Fast_Flames.exe`
    /// with the builtin opcode table, the same table `deform6-cli` uses
    /// when the user supplies no `--opcode-table` of its own.
    fn built_report_for_fast_flames() -> ProjectReport {
        let table = OpcodeTable::builtin();
        let report = crate::vb::inspect(FAST_FLAMES, &table, crate::journal::Mode::Strict)
            .expect("Fast_Flames.exe must inspect cleanly");
        let (model, items) = crate::write::model::from_report(&report, FAST_FLAMES);
        let summary = format!("Opcode table: builtin subset, {} entries", table.len());
        build(&report, &model, items, &summary, Mode::Strict)
    }

    #[test]
    fn serialising_a_built_report_twice_gives_two_byte_identical_strings() {
        let built = built_report_for_fast_flames();
        let first = built.to_json();
        let second = built.to_json();
        assert_eq!(first, second);
    }

    #[test]
    fn the_whole_write_path_run_twice_over_fast_flames_gives_byte_identical_report_files() {
        let first_bytes = built_report_for_fast_flames().to_json().into_bytes();
        let second_bytes = built_report_for_fast_flames().to_json().into_bytes();
        assert_eq!(first_bytes, second_bytes);
    }

    #[test]
    fn two_runs_over_fast_flames_give_an_equal_project_report() {
        let first = built_report_for_fast_flames();
        let second = built_report_for_fast_flames();
        assert_eq!(first, second);
    }

    #[test]
    fn the_limits_list_states_that_full_recompilation_did_not_run() {
        let limits = build_limits(Mode::Strict, &[], "Opcode table: test fixture, 0 entries");
        assert!(
            limits.iter().any(|line| line.contains("did not run")),
            "{limits:?}"
        );
    }

    #[test]
    fn the_limits_list_names_the_opcode_table_the_run_used() {
        let summary = "Opcode table: builtin subset, 74 entries";
        let limits = build_limits(Mode::Strict, &[], summary);
        assert!(limits.iter().any(|line| line == summary), "{limits:?}");
    }

    #[test]
    fn the_limits_list_never_implies_the_ide_opened_the_project() {
        let limits = build_limits(Mode::Strict, &[], "Opcode table: test fixture, 0 entries");
        for line in &limits {
            assert!(
                !line.to_lowercase().contains("the ide opened"),
                "a limit line must never claim the IDE opened this project: {line}"
            );
        }
    }

    #[test]
    fn build_limits_is_never_empty() {
        assert!(
            !build_limits(Mode::Strict, &[], "Opcode table: test fixture, 0 entries").is_empty()
        );
    }

    #[test]
    fn a_query_for_inferred_items_returns_paths_for_fast_flames_exe() {
        let built = built_report_for_fast_flames();
        let inferred_paths: Vec<&str> = built
            .items
            .iter()
            .filter(|item| item.confidence == Confidence::Inferred)
            .map(|item| item.path.as_str())
            .collect();
        assert!(
            !inferred_paths.is_empty(),
            "Fast_Flames.exe must produce at least one inferred item: {:?}",
            built.items
        );
    }
}
