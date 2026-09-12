//! The thin `.vbp` writer this plan's tracer needs; plan 04-03 completes
//! the full grammar (the setting block, references, `[MS Transaction
//! Server]`).
//!
//! `.planning/research/FILE-FORMATS.md` section 1: a `.vbp` is a flat list
//! of `Key=Value` lines, with no space around the `=`. This file writes the
//! three facts the tracer test can check today: `Type=Exe` first, the
//! component lines the recovered forms and objects give, and a `Startup=`
//! line naming a form a `Form=` line brings in.

use crate::vb::Report;
use crate::vb::classify::ObjectKind;

use super::model::{LineWriter, SafeName};

/// Writes the thin `.vbp` this task's tracer needs.
///
/// Line 1 is always `Type=Exe`. Every form becomes a `Form=` line, in
/// [`Report::forms`]'s own order; every module and every class becomes a
/// `Module=` or a `Class=` line, in [`Report::objects`]'s own order, each
/// carrying its own VB name and its own file name separated by `; `, per
/// section 1.3. `Startup=` names the first form in object table order, or
/// `"Sub Main"` when the project holds no form at all.
#[must_use]
pub(crate) fn write_vbp_thin(report: &Report) -> Vec<u8> {
    let mut writer = LineWriter::new();
    writer.push_line("Type=Exe");

    for form in &report.forms {
        let name = SafeName::new(&form.name);
        writer.push_line(&format!("Form={}", name.file_name("frm")));
    }

    for object in &report.objects {
        let name = SafeName::new(&object.name);
        match object.kind {
            ObjectKind::Module => {
                writer.push_line(&format!(
                    "Module={}; {}",
                    name.as_str(),
                    name.file_name("bas")
                ));
            }
            ObjectKind::Class => {
                writer.push_line(&format!(
                    "Class={}; {}",
                    name.as_str(),
                    name.file_name("cls")
                ));
            }
            ObjectKind::Form | ObjectKind::Unknown(_) => {}
        }
    }

    match report.forms.first() {
        Some(form) => {
            let name = SafeName::new(&form.name);
            writer.push_line(&format!("Startup=\"{}\"", name.as_str()));
        }
        None => writer.push_line("Startup=\"Sub Main\""),
    }

    writer.finish().0
}
