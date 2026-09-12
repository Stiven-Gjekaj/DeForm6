//! The thin `.bas`/`.cls` writer this plan's tracer needs; plan 04-05
//! completes the full grammar (the code region, empty-body procedure
//! signatures).
//!
//! `.planning/research/FILE-FORMATS.md` section 5: the `.bas` header is
//! one line, and the `.cls` preamble is thirteen lines, byte identical
//! across the whole corpus apart from the name. This file writes exactly
//! those fixed shapes; the code region itself is plan 04-05's job.

use super::model::{LineWriter, SafeName};

/// Writes the thin `.cls` file this task's tracer needs: the fixed
/// thirteen line preamble `.planning/research/FILE-FORMATS.md` section 5.2
/// gives, with `name` as the only variable.
#[must_use]
pub(crate) fn write_cls_thin(name: &SafeName) -> Vec<u8> {
    let mut writer = LineWriter::new();
    writer.push_line("VERSION 1.0 CLASS");
    writer.push_line("BEGIN");
    writer.push_line("  MultiUse = -1  'True");
    writer.push_line("  Persistable = 0  'NotPersistable");
    writer.push_line("  DataBindingBehavior = 0  'vbNone");
    writer.push_line("  DataSourceBehavior  = 0  'vbNone");
    writer.push_line("  MTSTransactionMode  = 0  'NotAnMTSObject");
    writer.push_line("END");
    writer.push_line(&format!("Attribute VB_Name = \"{}\"", name.as_str()));
    writer.push_line("Attribute VB_GlobalNameSpace = False");
    writer.push_line("Attribute VB_Creatable = True");
    writer.push_line("Attribute VB_PredeclaredId = False");
    writer.push_line("Attribute VB_Exposed = False");
    writer.finish().0
}

/// Writes the thin `.bas` file this task's tracer needs: the one line
/// header `.planning/research/FILE-FORMATS.md` section 5.1 gives.
#[must_use]
pub(crate) fn write_bas_thin(name: &SafeName) -> Vec<u8> {
    let mut writer = LineWriter::new();
    writer.push_line(&format!("Attribute VB_Name = \"{}\"", name.as_str()));
    writer.finish().0
}
