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
use crate::report::ProjectReport;
use crate::vb::Report;
use crate::vb::classify::ObjectKind;
use model::{NameKind, SafeNameIssuer};

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
/// # Errors
///
/// Returns [`Refusal::Damaged`] when a form's own `.frx` offset cursor
/// would overflow a `u32`; see [`crate::vb::frx::BlobCursor::take`].
pub fn project(report: &Report, data: &[u8]) -> Result<WrittenProject, Refusal> {
    let mut files: Vec<WrittenFile> = Vec::new();
    // One issuer for the whole run, so two objects that sanitize or clamp
    // to the same name are still told apart, in the order this function
    // issues names in: the project name first, then every form, then every
    // module and class, matching RPT-01's determinism requirement.
    let mut names = SafeNameIssuer::new();

    let (project_name, _project_faults) = names.issue(&report.project_name, NameKind::Project);

    let vbp_bytes = vbp::write_vbp_thin(report);
    files.push(WrittenFile {
        name: project_name.file_name("vbp"),
        bytes: vbp_bytes,
    });

    for form in &report.forms {
        let (form_name, _faults) = names.issue(&form.name, NameKind::Form);
        let frx_name = form_name.file_name("frx");
        let output = frm::write_form_thin(form, data, &frx_name)?;
        files.push(WrittenFile {
            name: form_name.file_name("frm"),
            bytes: output.frm,
        });
        files.push(WrittenFile {
            name: frx_name,
            bytes: output.frx,
        });
    }

    for object in &report.objects {
        match object.kind {
            ObjectKind::Class => {
                let (name, _faults) = names.issue(&object.name, NameKind::Class);
                files.push(WrittenFile {
                    name: name.file_name("cls"),
                    bytes: code::write_cls_thin(&name),
                });
            }
            ObjectKind::Module => {
                let (name, _faults) = names.issue(&object.name, NameKind::Module);
                files.push(WrittenFile {
                    name: name.file_name("bas"),
                    bytes: code::write_bas_thin(&name),
                });
            }
            ObjectKind::Form | ObjectKind::Unknown(_) => {}
        }
    }

    let project_report = ProjectReport {
        items: Vec::new(),
        defects: report.defects.clone(),
        limits: Vec::new(),
    };
    let report_bytes = project_report.to_json().into_bytes();
    files.push(WrittenFile {
        name: format!("{}.report.json", project_name.as_str()),
        bytes: report_bytes,
    });

    Ok(WrittenProject {
        files,
        report: project_report,
    })
}
