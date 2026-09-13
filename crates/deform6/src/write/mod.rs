//! The write side of DeForm6: turns a recovered [`crate::vb::Report`] into
//! the bytes of a Visual Basic 6 project, and the confidence report beside
//! it.
//!
//! [`project`] is the whole public entry point. It opens no file and writes
//! no file, the same discipline [`crate::vb::inspect`] already states in
//! its own doc comment, so that a future fuzz target can drive the writer
//! in memory, and so that `deform6-cli` stays the only part of this
//! workspace that touches the file system.
//!
//! The six modules below are declared here as a set, in one commit, for the
//! same reason `crate::vb`'s own module list states: each later plan in
//! this phase then edits only the one file it owns, and this file never
//! becomes a merge point for two plans in one wave.

pub mod code;
pub mod comment;
pub mod frm;
pub mod model;
pub mod values;
pub mod vbp;

use crate::error::Refusal;
use crate::journal::Mode;
use crate::report::{PathIssuer, ProjectReport, ReportItem};
use crate::vb::Report;
use crate::vb::classify::ObjectKind;
use crate::vb::opcodes::OpcodeTable;
use model::CodeKind;

/// One file this phase writes: its own file name, and its own bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrittenFile {
    /// The file name, built through [`SafeName::file_name`] wherever the
    /// name came from a recovered string.
    pub name: String,
    /// The file's own bytes, exactly as they would land on disk.
    pub bytes: Vec<u8>,
}

/// Every file [`project`] wrote, in the order a caller should write them,
/// plus the confidence report the same run built.
#[derive(Clone, Debug, PartialEq)]
pub struct WrittenProject {
    /// One entry per file this phase writes: the `.vbp`, one `.frm` and one
    /// `.frx` per form, one `.bas` per module, one `.cls` per class, and
    /// the JSON report last.
    pub files: Vec<WrittenFile>,
    /// The same confidence report [`ProjectReport::to_json`] serialises
    /// into the last entry of [`Self::files`].
    pub report: ProjectReport,
}

/// Turns `report` plus the executable bytes it was built from into the
/// files a Visual Basic 6 project directory holds.
///
/// This function opens no file and writes no file: every byte it produces
/// lives in the returned [`WrittenProject`] until a caller (`deform6-cli`'s
/// own `run_extract`) decides where they land. `data` is the executable's
/// own bytes, kept alive for this call because a resource blob's bytes are
/// re-read from it: [`crate::vb::propstream::PropertyValue::Blob`]
/// deliberately does not carry them forward.
///
/// Plan 04-08 switches this function over from the thin writers plans
/// 04-01 through 04-05 staged (`write_vbp_thin`, `write_form_thin`,
/// `write_cls_thin`, `write_bas_thin`) to the complete writers those same
/// plans built beside them, and wires [`crate::report::build`] so the
/// shipped report carries real items and real limits instead of the empty
/// arrays every prior plan in this phase left in place. See each writer's
/// own module for its own grammar; this function only decides the order
/// they run in and how their own report items are merged into one array.
///
/// # Merging report items from more than one writer
///
/// `write_vbp`, `write_form`, `write_cls` and `write_bas` each collect
/// their own [`crate::report::ReportItem`]s as they write, some with a
/// real path already (a property item, a tree-refused item), some with an
/// empty path a caller must fill in (a procedure signature item, a
/// Windows-1252 substitution item). [`crate::report::build`] separately
/// derives its own items from `model_items` (the choices
/// [`model::from_report`] made while building the model) and from every
/// control's own properties, graded a second time by confidence. This
/// function therefore issues every item's path through one shared
/// [`PathIssuer`], after backfilling evidence with
/// [`crate::report::with_header_evidence`] for any item that still carries
/// none, so a path a writer chose cannot collide, unnoticed, with a path
/// [`crate::report::build`] chose independently. A control's property may
/// therefore earn two items, one from the writer that omitted its line,
/// one from `build`'s own confidence grading of the same property, at two
/// different paths (a control path and a more specific property path).
/// This is redundant, not wrong: RPT-02 asks for a flat array keyed by
/// path, not for exactly one item per fact, and every item this run
/// produces still carries a real path, a basis and at least one evidence
/// record. Unifying the two derivations into one pass is future work.
///
/// `mode` names the run's own policy, and this function does one thing
/// with it: hands it to [`crate::report::build`], which states it in the
/// report's own limits list and lists an assumption line per `Recoverable`
/// defect when it is [`Mode::Salvage`]. `mode` must be the
/// same value the caller's own [`crate::vb::inspect`] call used to produce
/// `report`: a report that graded its items from a salvage read while
/// naming a strict run would misstate its own provenance.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when a form's own `.frx` offset cursor
/// would overflow a `u32`; see [`crate::vb::frx::BlobCursor::take`].
pub fn project(report: &Report, data: &[u8], mode: Mode) -> Result<WrittenProject, Refusal> {
    let (model, model_items) = model::from_report(report, data);

    let mut files: Vec<WrittenFile> = Vec::new();
    let mut paths = PathIssuer::new();
    let mut items: Vec<ReportItem> = Vec::new();

    let (vbp_bytes, vbp_items) = vbp::write_vbp(report, &model);
    files.push(WrittenFile {
        name: model.name.file_name("vbp"),
        bytes: vbp_bytes,
    });
    merge_items(&mut items, &mut paths, report, vbp_items, || {
        crate::report::META_PATH.to_owned()
    });

    for (form_model, form_report) in model.forms.iter().zip(report.forms.iter()) {
        let (form_files, form_items) = frm::write_form(form_model, &form_report.defects, data)?;
        files.push(WrittenFile {
            name: form_model.name.file_name("frm"),
            bytes: form_files.frm,
        });
        if let Some(frx_bytes) = form_files.frx {
            files.push(WrittenFile {
                name: form_model.name.file_name("frx"),
                bytes: frx_bytes,
            });
        }
        merge_items(&mut items, &mut paths, report, form_items, || {
            crate::report::path_for_form(&form_model.name)
        });
    }

    let code_reports = report
        .objects
        .iter()
        .filter(|object| matches!(object.kind, ObjectKind::Class | ObjectKind::Module));
    for (code_model, object_report) in model.code.iter().zip(code_reports) {
        let path_prefix = crate::report::path_for_code(code_model.kind, &code_model.name);
        let (bytes, code_items) = match code_model.kind {
            CodeKind::Class => code::write_cls(
                &code_model.name,
                &object_report.procedures,
                &[],
                &path_prefix,
            ),
            CodeKind::Module => code::write_bas(
                &code_model.name,
                &object_report.procedures,
                &[],
                &path_prefix,
            ),
        };
        let extension = match code_model.kind {
            CodeKind::Class => "cls",
            CodeKind::Module => "bas",
        };
        files.push(WrittenFile {
            name: code_model.name.file_name(extension),
            bytes,
        });
        merge_items(&mut items, &mut paths, report, code_items, || {
            path_prefix.clone()
        });
    }

    // `Command::Extract` carries no `--opcode-table` flag: every extract
    // run uses the builtin subset, matching `deform6-cli`'s own
    // `load_opcode_table` default. If a future plan adds that flag here,
    // the table (and this summary) must be threaded through as a
    // parameter instead of assumed.
    let opcode_table_summary = format!(
        "Opcode table  builtin subset, {} entries",
        OpcodeTable::builtin().len()
    );
    let built = crate::report::build(report, &model, model_items, &opcode_table_summary, mode);
    for item in built.items {
        let path = paths.issue(item.path);
        items.push(ReportItem { path, ..item });
    }

    let project_report = ProjectReport {
        items,
        defects: built.defects,
        limits: built.limits,
    };
    let report_bytes = project_report.to_json().into_bytes();
    files.push(WrittenFile {
        name: format!("{}.report.json", model.name.as_str()),
        bytes: report_bytes,
    });

    Ok(WrittenProject {
        files,
        report: project_report,
    })
}

/// Finalises every item one writer returned: fills an empty path with
/// `default_path`, backfills evidence when the writer left none, issues
/// the final path through `paths` (extending, never overwriting, a
/// collision), and appends the result to `items`.
fn merge_items(
    items: &mut Vec<ReportItem>,
    paths: &mut PathIssuer,
    report: &Report,
    writer_items: Vec<ReportItem>,
    default_path: impl Fn() -> String,
) {
    for mut item in writer_items {
        if item.path.is_empty() {
            item.path = default_path();
        }
        let item = crate::report::with_header_evidence(item, report);
        let path = paths.issue(item.path.clone());
        items.push(ReportItem { path, ..item });
    }
}
