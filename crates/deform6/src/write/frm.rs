//! The thin `.frm`/`.frx` writer this plan's tracer needs; plan 04-04
//! completes the full grammar (`write_form`, `FormFiles`, the menus-last
//! ordering rule, the `Object.` prefix, the OCX colour branch).
//!
//! This file owns the one call this whole phase must never duplicate:
//! [`crate::vb::frx::BlobCursor::take`]. One cursor per form, called once
//! per resource blob, in the exact order this file emits the property
//! that carries it. No offset is computed any other way; see
//! [`BlobCursor`]'s own doc comment for why an independent computation
//! drifts.

use crate::error::Refusal;
use crate::report::{Confidence, Evidence, ReportItem};
use crate::vb::controltree::ControlKind;
use crate::vb::frx::{self, BlobCursor};
use crate::vb::propstream::{FontBlock, PositionBlock, PropertyValue};
use crate::vb::{ControlReport, FormReport, ObjectProcedures, ProcedureEntry};
use crate::write::values;

use super::model::{ControlModel, FormModel, LineWriter, NameKind, ProcedureModel, SafeName};

/// The bytes of one form's `.frm` file and its own `.frx` resource file.
pub(crate) struct ThinFormOutput {
    /// The `.frm` file's own bytes.
    pub frm: Vec<u8>,
    /// The `.frx` file's own bytes, packed end to end in the order this
    /// file's own [`BlobCursor`] assigned them.
    pub frx: Vec<u8>,
}

/// One property line this task's thin writer already knows how to render,
/// keyed by the name it sorts and prints under.
enum RenderedProperty {
    /// A pre-rendered value, printed after the padded name and the `=`.
    Plain(String),
    /// A `Font` property, printed as its own `BeginProperty`/`EndProperty`
    /// block with its own fixed key order.
    Font(FontBlock),
}

/// Writes one form's own `.frm` and `.frx` bytes.
///
/// `data` is the executable's own bytes: a resource blob's own bytes are
/// re-read from `data` at the exact range
/// [`crate::vb::propstream::PropertyValue::Blob`] names, because that
/// variant deliberately does not carry them forward. `frx_file_name` is
/// the `.frx` file name this form's own `Icon`/`Picture`/`Text` lines
/// reference.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when this form's own blobs would overflow
/// a `u32` `.frx` offset; see [`BlobCursor::take`].
pub(crate) fn write_form_thin(
    form: &FormReport,
    data: &[u8],
    frx_file_name: &str,
) -> Result<ThinFormOutput, Refusal> {
    let mut writer = LineWriter::new();
    let mut frx = Vec::new();
    let mut blob_cursor = BlobCursor::new();

    writer.push_line("VERSION 5.00");

    let children = children_of(&form.controls);

    if let Some(root) = form.controls.first() {
        let (name, _faults) = SafeName::new(&root.name, NameKind::Form);
        write_control_block(
            &mut writer,
            &form.controls,
            &children,
            0,
            0,
            data,
            &mut blob_cursor,
            &mut frx,
            frx_file_name,
        )?;

        writer.push_line(&format!("Attribute VB_Name = \"{}\"", name.as_str()));
        writer.push_line("Attribute VB_GlobalNameSpace = False");
        writer.push_line("Attribute VB_Creatable = False");
        writer.push_line("Attribute VB_PredeclaredId = True");
        writer.push_line("Attribute VB_Exposed = False");
    }

    let (frm, _substituted) = writer.finish();
    Ok(ThinFormOutput { frm, frx })
}

/// Builds, for every control index, the list of its own children's
/// indexes, in the order [`FormReport::controls`] already gives them. A
/// plain `Vec` indexed by position, never a hash keyed map: this list
/// decides the order controls are written in, and that order must never
/// vary between two runs over the same report.
fn children_of(controls: &[ControlReport]) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); controls.len()];
    for (index, control) in controls.iter().enumerate() {
        if let Some(parent) = control.parent
            && let Some(list) = children.get_mut(parent)
        {
            list.push(index);
        }
    }
    children
}

/// Writes one control's own `Begin ... End` block, its own properties, and
/// every one of its own children, depth first.
#[allow(clippy::too_many_arguments)]
fn write_control_block(
    writer: &mut LineWriter,
    controls: &[ControlReport],
    children: &[Vec<usize>],
    index: usize,
    depth: usize,
    data: &[u8],
    blob_cursor: &mut BlobCursor,
    frx: &mut Vec<u8>,
    frx_file_name: &str,
) -> Result<(), Refusal> {
    let Some(control) = controls.get(index) else {
        return Ok(());
    };
    let own_indent = "   ".repeat(depth);
    let inner_depth = depth.saturating_add(1);
    let class = vb_class_name(&control.kind);
    let (name, _faults) = SafeName::new(&control.name, name_kind_for(&control.kind));
    writer.push_line(&format!("{own_indent}Begin {class} {} ", name.as_str()));

    let rendered = collect_properties(control, data, blob_cursor, frx, frx_file_name)?;
    write_rendered_properties(writer, &rendered, inner_depth);

    if let Some(kids) = children.get(index) {
        for &child in kids {
            write_control_block(
                writer,
                controls,
                children,
                child,
                inner_depth,
                data,
                blob_cursor,
                frx,
                frx_file_name,
            )?;
        }
    }

    writer.push_line(&format!("{own_indent}End"));
    Ok(())
}

/// Gives the `VB.<Name>` class this control's own kind writes on its
/// `Begin` line. An external (OCX) control's real class name is plan
/// 04-04's own job; this task names it generically, since no corpus
/// program this task's tracer reads carries one.
fn vb_class_name(kind: &ControlKind) -> String {
    let name = match kind {
        ControlKind::PictureBox => "PictureBox",
        ControlKind::Label => "Label",
        ControlKind::TextBox => "TextBox",
        ControlKind::Frame => "Frame",
        ControlKind::CommandButton => "CommandButton",
        ControlKind::CheckBox => "CheckBox",
        ControlKind::OptionButton => "OptionButton",
        ControlKind::ComboBox => "ComboBox",
        ControlKind::ListBox => "ListBox",
        ControlKind::HScrollBar => "HScrollBar",
        ControlKind::VScrollBar => "VScrollBar",
        ControlKind::Timer => "Timer",
        ControlKind::Form => "Form",
        ControlKind::DriveListBox => "DriveListBox",
        ControlKind::DirListBox => "DirListBox",
        ControlKind::FileListBox => "FileListBox",
        ControlKind::Menu => "Menu",
        ControlKind::MdiForm => "MDIForm",
        ControlKind::Shape => "Shape",
        ControlKind::Line => "Line",
        ControlKind::Image => "Image",
        ControlKind::Data => "Data",
        ControlKind::Ole => "OLE",
        ControlKind::UserControl => "UserControl",
        ControlKind::PropertyPage => "PropertyPage",
        ControlKind::UserDocument => "UserDocument",
        ControlKind::External | ControlKind::Unknown(_) => "Control",
    };
    format!("VB.{name}")
}

/// Gives the [`NameKind`] a control's own name is issued under: `Form` for
/// the form or MDIForm root block, `Control` for everything else.
fn name_kind_for(kind: &ControlKind) -> NameKind {
    if matches!(kind, ControlKind::Form | ControlKind::MdiForm) {
        NameKind::Form
    } else {
        NameKind::Control
    }
}

/// Collects every property this control's own stream decoded into a
/// renderable line, expanding a [`PropertyValue::Position`] into its own
/// four named lines (`Left`, `Top`, `Width`, `Height`), the shape every
/// corpus `.frm` writes. A property this repository could not decode
/// ([`PropertyValue::Undecoded`] or [`PropertyValue::BlobUnreadable`]) is
/// omitted, per `04-RESEARCH.md` Pitfall 1: plan 04-06 turns the omission
/// into a report item.
fn collect_properties(
    control: &ControlReport,
    data: &[u8],
    blob_cursor: &mut BlobCursor,
    frx: &mut Vec<u8>,
    frx_file_name: &str,
) -> Result<Vec<(String, RenderedProperty)>, Refusal> {
    let mut out = Vec::new();
    for property in &control.properties {
        match property {
            PropertyValue::Byte { name, value } => {
                out.push((name.clone(), RenderedProperty::Plain(value.to_string())));
            }
            PropertyValue::Boolean { name, value } => {
                out.push((
                    name.clone(),
                    RenderedProperty::Plain(format_bool(*value != 0)),
                ));
            }
            PropertyValue::Integer { name, value } => {
                out.push((name.clone(), RenderedProperty::Plain(value.to_string())));
            }
            PropertyValue::Long { name, value } => {
                out.push((name.clone(), RenderedProperty::Plain(value.to_string())));
            }
            PropertyValue::Single { name, value } => {
                out.push((name.clone(), RenderedProperty::Plain(value.to_string())));
            }
            PropertyValue::Text { name, value } => {
                let escaped = value.replace('"', "\"\"");
                out.push((
                    name.clone(),
                    RenderedProperty::Plain(format!("\"{escaped}\"")),
                ));
            }
            PropertyValue::Position { value, .. } => {
                let (left, top, width, height) = position_fields(value);
                out.push(("Left".to_owned(), RenderedProperty::Plain(left)));
                out.push(("Top".to_owned(), RenderedProperty::Plain(top)));
                out.push(("Width".to_owned(), RenderedProperty::Plain(width)));
                out.push(("Height".to_owned(), RenderedProperty::Plain(height)));
            }
            PropertyValue::Font { name, value } => {
                out.push((name.clone(), RenderedProperty::Font(value.clone())));
            }
            PropertyValue::Blob {
                name,
                offset,
                declared_len,
                ..
            } => {
                if let Some(frx_offset) =
                    append_blob(data, *offset, *declared_len, blob_cursor, frx)?
                {
                    out.push((
                        name.clone(),
                        RenderedProperty::Plain(format!("\"{frx_file_name}\":{frx_offset:04X}")),
                    ));
                }
            }
            PropertyValue::BlobUnreadable { .. } | PropertyValue::Undecoded { .. } => {}
        }
    }
    Ok(out)
}

/// Renders the four coordinates a [`PositionBlock`] carries as plain
/// decimal strings, in `(left, top, width, height)` order.
fn position_fields(value: &PositionBlock) -> (String, String, String, String) {
    match value {
        PositionBlock::Short {
            left,
            top,
            width,
            height,
        } => (
            left.to_string(),
            top.to_string(),
            width.to_string(),
            height.to_string(),
        ),
        PositionBlock::Long {
            left,
            top,
            width,
            height,
        } => (
            left.to_string(),
            top.to_string(),
            width.to_string(),
            height.to_string(),
        ),
    }
}

/// Formats a VB boolean: `-1` then two spaces then `'True`, or `0` then
/// three spaces then `'False`. `.planning/research/FILE-FORMATS.md`
/// section 3.4.
fn format_bool(value: bool) -> String {
    if value {
        "-1  'True".to_owned()
    } else {
        "0   'False".to_owned()
    }
}

/// Re-reads one resource blob's own bytes out of `data`, at the range
/// `offset..offset + 4 + declared_len` that
/// [`crate::vb::propstream::PropertyValue::Blob`] names, appends them to
/// `frx`, and gives the `.frx` offset [`BlobCursor::take`] assigned this
/// blob. Gives `None`, writing nothing, when the range does not fit inside
/// `data`: a hostile or truncated file must not stop the rest of this
/// form's own properties from writing.
fn append_blob(
    data: &[u8],
    offset: u32,
    declared_len: u32,
    blob_cursor: &mut BlobCursor,
    frx: &mut Vec<u8>,
) -> Result<Option<u32>, Refusal> {
    let blob = frx::Blob {
        header: [0u8; 8],
        image: Vec::new(),
        declared_len,
        offset,
    };
    let frx_offset = blob_cursor.take(&blob)?;

    let Ok(start) = usize::try_from(offset) else {
        return Ok(None);
    };
    let Ok(declared) = usize::try_from(declared_len) else {
        return Ok(None);
    };
    let Some(total) = 4_usize.checked_add(declared) else {
        return Ok(None);
    };
    let Some(end) = start.checked_add(total) else {
        return Ok(None);
    };
    let Some(bytes) = data.get(start..end) else {
        return Ok(None);
    };

    frx.extend_from_slice(bytes);
    Ok(Some(frx_offset))
}

/// Writes every rendered property in case insensitive ascending order by
/// name, at `depth`. Plan 04-04 owns the full ordering rule (menus last,
/// `BeginProperty` blocks sorted in by their own name); this task's thin
/// writer sorts every property together, which is already correct for a
/// form that declares no menu, per this task's own tracer.
fn write_rendered_properties(
    writer: &mut LineWriter,
    rendered: &[(String, RenderedProperty)],
    depth: usize,
) {
    let mut order: Vec<usize> = (0..rendered.len()).collect();
    order.sort_by(|&a, &b| {
        let (Some((name_a, _)), Some((name_b, _))) = (rendered.get(a), rendered.get(b)) else {
            return std::cmp::Ordering::Equal;
        };
        name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });

    let indent = "   ".repeat(depth);
    for index in order {
        let Some((name, property)) = rendered.get(index) else {
            continue;
        };
        match property {
            RenderedProperty::Plain(value) => {
                writer.push_line(&format!("{indent}{name:<16}=   {value}"));
            }
            RenderedProperty::Font(font) => write_font_block(writer, &indent, font),
        }
    }
}

/// Writes a `Font` property as its own `BeginProperty`/`EndProperty`
/// block, with the seven keys in the fixed order
/// `.planning/research/FILE-FORMATS.md` section 3.9 gives.
fn write_font_block(writer: &mut LineWriter, indent: &str, font: &FontBlock) {
    let inner = format!("{indent}   ");
    writer.push_line(&format!("{indent}BeginProperty Font "));

    let escaped_name = font.name.replace('"', "\"\"");
    writer.push_line(&format!("{inner}{:<16}=   \"{escaped_name}\"", "Name"));

    let size = f64::from(font.size_raw) / 10_000.0;
    writer.push_line(&format!("{inner}{:<16}=   {size}", "Size"));

    writer.push_line(&format!("{inner}{:<16}=   {}", "Charset", font.charset));
    writer.push_line(&format!("{inner}{:<16}=   {}", "Weight", font.weight));
    writer.push_line(&format!(
        "{inner}{:<16}=   {}",
        "Underline",
        format_bool(font.underline)
    ));
    writer.push_line(&format!(
        "{inner}{:<16}=   {}",
        "Italic",
        format_bool(font.italic)
    ));
    writer.push_line(&format!(
        "{inner}{:<16}=   {}",
        "Strikethrough",
        format_bool(font.strikethrough)
    ));
    writer.push_line(&format!("{indent}EndProperty"));
}

// --- Plan 04-04, Task 1: one cursor per form, and the resource file the
// form file agrees with -----------------------------------------------------

/// The width every property name is padded to before the `=` on a `.frm`
/// line, before the mandatory three spaces. `.planning/research/
/// FILE-FORMATS.md` section 3.1: measured at 5899 of 5899 property lines.
/// A name at or over this width still gets exactly one space, per section
/// 3.1's own `Object.Width` proof that the field is a minimum, never a
/// truncation.
pub const FRM_NAME_PAD: usize = 16;

/// The number of spaces one nesting depth level indents by.
/// `.planning/research/FILE-FORMATS.md` section 2.4: measured at 0, 3, 6, 9
/// and 12 spaces across the whole corpus.
pub const FRM_INDENT: usize = 3;

/// The exact twelve bytes the corpus proves the IDE leaves behind when a
/// picture property exists but its own blob was removed:
/// `.planning/research/FILE-FORMATS.md` section 4.3. The first four bytes
/// are a declared length of 8, the eight byte inline picture header alone
/// with zero image bytes; the next four are the `6C 74 00 00` marker; the
/// last four are the header's own zeroed length and format fields.
pub const EMPTY_PICTURE_RECORD: [u8; 12] = [
    0x08, 0x00, 0x00, 0x00, 0x6C, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// The bytes of one form's `.frm` file, and its own `.frx` resource file
/// when this form names at least one blob.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormFiles {
    /// The `.frm` file's own bytes.
    pub frm: Vec<u8>,
    /// The `.frx` file's own bytes, packed end to end in the order this
    /// file's own [`BlobCursor`] assigned them, or `None` when this form
    /// names no blob at all: a form that names no blob must not get an
    /// empty `.frx` file beside it.
    pub frx: Option<Vec<u8>>,
}

/// One property line, before this control's own ordering rules place it: a
/// plain value, a `BeginProperty`/`EndProperty` block (the `Font` shape),
/// or a resource blob whose `.frx` offset is not yet assigned, because
/// assigning one is a side effect that must happen in the property's own
/// final emission order, never in raw stream order.
#[derive(Clone, Debug, PartialEq)]
enum PendingLine {
    /// A pre-rendered value, printed after the padded name and the `=`.
    Plain(String),
    /// A `Font` property's own seven keys, printed as their own
    /// `BeginProperty`/`EndProperty` block.
    PropertyBlock(Vec<(String, String)>),
    /// A resource blob: the byte range still needs a `.frx` offset, given
    /// only once this control's own final property order is known.
    PendingBlob {
        /// The blob's own absolute file offset.
        offset: u32,
        /// The blob's own declared length.
        declared_len: u32,
    },
}

/// One property line, ready to write: every [`PendingLine::PendingBlob`]
/// has already been resolved against the one [`BlobCursor`] this form
/// owns, in this control's own final emission order.
#[derive(Clone, Debug, PartialEq)]
enum ResolvedLine {
    /// A pre-rendered value, printed after the padded name and the `=`.
    Plain(String),
    /// A `Font` property's own seven keys.
    PropertyBlock(Vec<(String, String)>),
}

/// Pads `name` to `width`, or, when `name` is already `width` characters or
/// more, appends exactly one space: `.planning/research/FILE-FORMATS.md`
/// section 3.1's own `Object.Width` proof that the field is a minimum, not
/// a truncation.
fn pad_name(name: &str, width: usize) -> String {
    if name.chars().count() < width {
        format!("{name:<width$}")
    } else {
        format!("{name} ")
    }
}

/// Gives the path segment [`ReportItem`]s about this form's own controls
/// and properties are keyed under: the form's own path for the form's own
/// outermost control (`index == 0`), and the control's own path for every
/// other control.
fn control_report_path(form: &FormModel, control: &ControlModel, index: usize) -> String {
    if index == 0 {
        crate::report::path_for_form(&form.name)
    } else {
        crate::report::path_for_control(&form.name, &control.name)
    }
}

/// Gives the path segment one property's own item takes: its own recovered
/// name, or `opcode<N>` for a [`PropertyValue::Undecoded`] value, which
/// carries no name at all. Mirrors `crate::report::property_word` exactly;
/// duplicated rather than shared, because that function is private to its
/// own file, and section 2.5's own "no shared line template" rule applies
/// to path building here for the same reason it applies to grammar.
fn property_name(property: &PropertyValue) -> String {
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

/// Turns one control's own decoded properties into [`PendingLine`]s, plus
/// every [`ReportItem`] an omission earned. Calls
/// [`crate::write::values::format_value`] for every property, which is
/// side effect free: no `.frx` offset is assigned here, only which
/// properties need one later is decided.
fn collect_pending_lines(
    form: &FormModel,
    control: &ControlModel,
    index: usize,
) -> (Vec<(String, PendingLine)>, Vec<ReportItem>) {
    let mut out = Vec::new();
    let mut items = Vec::new();

    for property in &control.properties {
        let (formatted, item) = values::format_value(property, control.is_external);
        if let Some(mut item) = item {
            item.path = control_report_path(form, control, index);
            items.push(item);
        }

        match formatted {
            values::FormattedValue::Line(text) => {
                out.push((property_name(property), PendingLine::Plain(text)));
            }
            values::FormattedValue::Multi(lines) => {
                if let PropertyValue::Font { name, .. } = property {
                    out.push((
                        name.clone(),
                        PendingLine::PropertyBlock(
                            lines.into_iter().map(|(n, v)| (n.to_owned(), v)).collect(),
                        ),
                    ));
                } else {
                    for (name, value) in lines {
                        out.push((name.to_owned(), PendingLine::Plain(value)));
                    }
                }
            }
            values::FormattedValue::Resource => match property {
                PropertyValue::Blob {
                    name,
                    offset,
                    declared_len,
                    ..
                } => {
                    out.push((
                        name.clone(),
                        PendingLine::PendingBlob {
                            offset: *offset,
                            declared_len: *declared_len,
                        },
                    ));
                }
                PropertyValue::Text { name, .. } => {
                    items.push(ReportItem {
                        path: control_report_path(form, control, index),
                        confidence: Confidence::Unrecoverable,
                        basis: format!(
                            "{name} is too long, or holds a line break, to write inline, \
                             and this repository carries no byte range to move it into the \
                             .frx; the property is omitted"
                        ),
                        evidence: Vec::new(),
                    });
                }
                _ => {}
            },
            values::FormattedValue::Omit => {}
        }
    }

    (out, items)
}

/// Advances `blob_cursor` and appends [`EMPTY_PICTURE_RECORD`] to `frx`,
/// for a picture property whose file marks it present but whose blob is
/// absent.
///
/// `crate::vb::propstream::walk_properties` does not carry this case
/// forward as a [`PropertyValue`] today: an absent blob (`0xFFFFFFFF`)
/// produces no property at all, the same as a property never touched, per
/// plan 03-15's own design. This function exists and is tested directly
/// against the twelve corpus-measured bytes so the writer already knows
/// the shape the day the read side is widened to carry the fact forward;
/// it has no production call site yet.
#[allow(
    dead_code,
    reason = "no PropertyValue this repository builds carries a present-but-absent blob yet; \
              this function is tested directly against the corpus-measured bytes so the \
              writer already knows the shape the day propstream.rs is widened to carry it"
)]
fn write_empty_picture_record(
    blob_cursor: &mut BlobCursor,
    frx: &mut Vec<u8>,
) -> Result<u32, Refusal> {
    let blob = frx::Blob {
        header: [0u8; 8],
        image: Vec::new(),
        declared_len: 8,
        offset: 0,
    };
    let frx_offset = blob_cursor.take(&blob)?;
    frx.extend_from_slice(&EMPTY_PICTURE_RECORD);
    Ok(frx_offset)
}

/// Builds, for every control index, the list of its own children's
/// indexes, in the order [`FormModel::controls`] already gives them.
fn children_of_model(controls: &[ControlModel]) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); controls.len()];
    for (index, control) in controls.iter().enumerate() {
        if let Some(parent) = control.parent
            && let Some(list) = children.get_mut(parent)
        {
            list.push(index);
        }
    }
    children
}

/// Writes one control's own `Begin ... End` block, its own properties in
/// their own resolved emission order, and every one of its own children,
/// depth first.
#[allow(clippy::too_many_arguments)]
fn write_model_control_block(
    writer: &mut LineWriter,
    form: &FormModel,
    children: &[Vec<usize>],
    index: usize,
    data: &[u8],
    blob_cursor: &mut BlobCursor,
    frx: &mut Vec<u8>,
    frx_file_name: &str,
    items: &mut Vec<ReportItem>,
) -> Result<(), Refusal> {
    let Some(control) = form.controls.get(index) else {
        return Ok(());
    };
    let depth = control.depth;
    let own_indent = " ".repeat(FRM_INDENT.saturating_mul(depth));
    let class = vb_class_name(&control.kind);

    writer.push_line(&format!(
        "{own_indent}Begin {class} {} ",
        control.name.as_str()
    ));

    let (pending, mut render_items) = collect_pending_lines(form, control, index);
    items.append(&mut render_items);

    let mut order: Vec<usize> = (0..pending.len()).collect();
    if !control.is_external {
        order.sort_by(|&a, &b| {
            let name_a = pending
                .get(a)
                .map_or_else(String::new, |(n, _)| n.to_lowercase());
            let name_b = pending
                .get(b)
                .map_or_else(String::new, |(n, _)| n.to_lowercase());
            name_a.cmp(&name_b)
        });
    }

    let mut resolved: Vec<(String, ResolvedLine)> = Vec::with_capacity(order.len());
    for position in order {
        let Some((name, line)) = pending.get(position).cloned() else {
            continue;
        };
        let resolved_line = match line {
            PendingLine::Plain(value) => ResolvedLine::Plain(value),
            PendingLine::PropertyBlock(lines) => ResolvedLine::PropertyBlock(lines),
            PendingLine::PendingBlob {
                offset,
                declared_len,
            } => match append_blob(data, offset, declared_len, blob_cursor, frx)? {
                Some(frx_offset) => ResolvedLine::Plain(values::format_resource_reference(
                    frx_file_name,
                    frx_offset,
                    false,
                )),
                None => {
                    items.push(ReportItem {
                        path: control_report_path(form, control, index),
                        confidence: Confidence::Unrecoverable,
                        basis: format!(
                            "{name}'s own declared byte range does not fit inside the \
                             executable this run read; the property is omitted"
                        ),
                        evidence: vec![Evidence {
                            offset,
                            structure: "PropertyValue",
                            field: "Blob",
                            note: None,
                        }],
                    });
                    continue;
                }
            },
        };
        resolved.push((name, resolved_line));
    }

    write_resolved_lines(writer, &resolved, depth.saturating_add(1));

    if let Some(kids) = children.get(index) {
        for &child in kids {
            write_model_control_block(
                writer,
                form,
                children,
                child,
                data,
                blob_cursor,
                frx,
                frx_file_name,
                items,
            )?;
        }
    }

    writer.push_line(&format!("{own_indent}End"));
    Ok(())
}

/// Writes every resolved property line, already in final order, at
/// `depth`.
fn write_resolved_lines(
    writer: &mut LineWriter,
    resolved: &[(String, ResolvedLine)],
    depth: usize,
) {
    let indent = " ".repeat(FRM_INDENT.saturating_mul(depth));
    for (name, line) in resolved {
        match line {
            ResolvedLine::Plain(value) => {
                writer.push_line(&format!(
                    "{indent}{}=   {value}",
                    pad_name(name, FRM_NAME_PAD)
                ));
            }
            ResolvedLine::PropertyBlock(lines) => {
                writer.push_line(&format!("{indent}BeginProperty {name} "));
                let inner_indent = " ".repeat(FRM_INDENT.saturating_mul(depth.saturating_add(1)));
                for (inner_name, inner_value) in lines {
                    writer.push_line(&format!(
                        "{inner_indent}{}=   {inner_value}",
                        pad_name(inner_name, FRM_NAME_PAD)
                    ));
                }
                writer.push_line(&format!("{indent}EndProperty"));
            }
        }
    }
}

/// Turns [`ProcedureModel`]s into the [`ObjectProcedures`] shape
/// [`crate::write::code::write_code_region`] already consumes: a public
/// slot with a recovered name becomes [`ProcedureEntry::Public`], and a
/// slot with none becomes [`ProcedureEntry::Private`].
fn model_procedures_to_object_procedures(procedures: &[ProcedureModel]) -> ObjectProcedures {
    let slots = procedures
        .iter()
        .map(|procedure| match &procedure.name {
            Some(name) => ProcedureEntry::Public {
                name: name.clone(),
                prototype: procedure.prototype.clone(),
            },
            None => ProcedureEntry::Private,
        })
        .collect();
    ObjectProcedures::Slots(slots)
}

/// Appends one [`ReportItem`] naming every character this call's own
/// writer could not represent in Windows-1252, when there is at least one.
/// Mirrors `crate::write::code::push_substitution_item`; duplicated
/// rather than shared, since that function is private to its own file.
fn push_substitution_item(items: &mut Vec<ReportItem>, substituted: &[char]) {
    if substituted.is_empty() {
        return;
    }
    items.push(ReportItem {
        path: String::new(),
        confidence: Confidence::Unrecoverable,
        basis: format!(
            "{} character(s) in this file could not be represented in Windows-1252 and \
             were replaced with '?': {substituted:?}",
            substituted.len()
        ),
        evidence: Vec::new(),
    });
}

/// Writes one form's own `.frm` and `.frx` bytes, from its complete
/// [`FormModel`].
///
/// Builds one [`BlobCursor`] here, before the control loop, and never
/// shares it with another form: a second call to this function always
/// starts its own cursor at zero. `data` is the executable's own bytes:
/// every resource blob's own bytes are re-read from `data`, because
/// [`PropertyValue::Blob`] deliberately does not carry them forward.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when this form's own blobs would overflow
/// a `u32` `.frx` offset; see [`BlobCursor::take`].
pub fn write_form(form: &FormModel, data: &[u8]) -> Result<(FormFiles, Vec<ReportItem>), Refusal> {
    let mut writer = LineWriter::new();
    let mut items = Vec::new();
    let mut blob_cursor = BlobCursor::new();
    let mut frx_bytes = Vec::new();
    let frx_file_name = form.name.file_name("frx");

    writer.push_line("VERSION 5.00");

    if form.controls.is_empty() {
        writer.push_line(&format!("Begin VB.Form {} ", form.name.as_str()));
        writer.push_line("End");
    } else {
        let children = children_of_model(&form.controls);
        write_model_control_block(
            &mut writer,
            form,
            &children,
            0,
            data,
            &mut blob_cursor,
            &mut frx_bytes,
            &frx_file_name,
            &mut items,
        )?;
    }

    let attribute_lines = crate::write::code::form_attribute_block(
        &form.name,
        crate::write::code::AttributeFileKind::Form,
    );
    for line in &attribute_lines {
        writer.push_line(line);
    }

    let procedures = model_procedures_to_object_procedures(&form.procedures);
    let (region_lines, mut code_items) = crate::write::code::write_code_region(&[], &procedures);
    for line in &region_lines {
        writer.push_line(line);
    }
    items.append(&mut code_items);

    let (frm_bytes, substituted) = writer.finish();
    push_substitution_item(&mut items, &substituted);

    let frx = if frx_bytes.is_empty() {
        None
    } else {
        Some(frx_bytes)
    };

    Ok((
        FormFiles {
            frm: frm_bytes,
            frx,
        },
        items,
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{
        BlobCursor, ControlKind, EMPTY_PICTURE_RECORD, PropertyValue, write_empty_picture_record,
        write_form,
    };
    use crate::vb::frx;
    use crate::vb::opcodes::OpcodeTable;
    use crate::write::model::{ControlModel, FormModel, NameKind, SafeName};

    /// Gives the absolute path to a file under this repository's own
    /// vendored `corpus/`.
    fn corpus_path(relative: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus")
            .join(relative)
    }

    /// Runs `inspect` and `from_report` over a corpus executable, at run
    /// time, and gives its first form's own [`FormModel`] plus the raw
    /// executable bytes.
    fn first_form(exe_relative: &str) -> (FormModel, Vec<u8>) {
        let data = std::fs::read(corpus_path(exe_relative)).expect("reading the corpus exe");
        let table = OpcodeTable::builtin();
        let report = crate::vb::inspect(&data, &table).expect("inspect must succeed");
        let (model, _items) = crate::write::model::from_report(&report, &data);
        let form = model
            .forms
            .into_iter()
            .next()
            .expect("at least one form must be recovered");
        (form, data)
    }

    /// Parses every `"name.frx":OFFSET` (or `$"name.frx":OFFSET`) hex
    /// offset a committed `.frm` declares, in the order the file's own
    /// lines give them. Copied from `crate::vb::frx`'s own test module,
    /// which this task's own action text asks to repeat from the write
    /// side, not share: two independent readings of the same fact are the
    /// point.
    fn declared_frx_offsets(frm_relative: &str) -> Vec<u32> {
        let bytes = std::fs::read(corpus_path(frm_relative)).expect("reading the committed .frm");
        let text: String = bytes.iter().copied().map(char::from).collect();
        let mut offsets = Vec::new();
        for line in text.lines() {
            let Some(colon) = line.find(".frx\":") else {
                continue;
            };
            let hex = &line[colon + 6..];
            let hex: String = hex.chars().take_while(char::is_ascii_hexdigit).collect();
            if let Ok(offset) = u32::from_str_radix(&hex, 16) {
                offsets.push(offset);
            }
        }
        offsets
    }

    /// Drives every declared offset of one committed `.frx` file through a
    /// fresh [`BlobCursor`], reading the declared length the real file
    /// holds at each declared offset, and asserts the cursor reproduces
    /// the whole sequence exactly.
    ///
    /// This is the same measurement `crate::vb::frx`'s own test module
    /// makes, repeated here so this file's own reliance on the shipped
    /// cursor is proved directly, not assumed from a source grep alone.
    /// Both files are read at run time, per `AGENTS.md`'s rule that a
    /// derived fixture from a corpus binary never enters the repository.
    fn assert_cursor_reproduces_declared_offsets(frm_relative: &str, frx_relative: &str) {
        let declared_offsets = declared_frx_offsets(frm_relative);
        assert!(
            declared_offsets.len() >= 2,
            "{frm_relative} must declare at least two resource offsets for this test to prove \
             anything about the gaps between them"
        );
        let frx_bytes =
            std::fs::read(corpus_path(frx_relative)).expect("reading the committed .frx");

        let mut cursor = BlobCursor::new();
        let mut got = Vec::new();
        for &offset in &declared_offsets {
            let at = usize::try_from(offset).expect("a real .frx offset fits in usize");
            let declared_len = u32::from_le_bytes(
                frx_bytes
                    .get(at..at.saturating_add(4))
                    .expect("the committed .frx holds 4 bytes at every declared offset")
                    .try_into()
                    .expect("exactly 4 bytes were sliced"),
            );
            let blob = frx::Blob {
                header: [0u8; 8],
                image: Vec::new(),
                declared_len,
                offset: 0,
            };
            got.push(
                cursor
                    .take(&blob)
                    .expect("no corpus form overflows a u32 offset"),
            );
        }
        assert_eq!(
            got, declared_offsets,
            "{frm_relative}'s own declared offset sequence must round-trip through the cursor \
             exactly, driven from this file's own test module"
        );
    }

    #[test]
    fn the_cursor_reproduces_form_physics_frxs_own_ten_declared_offsets() {
        assert_cursor_reproduces_declared_offsets(
            "vb6-code/Game-physics-basic/FormPhysics.frm",
            "vb6-code/Game-physics-basic/FormPhysics.frx",
        );
    }

    #[test]
    fn the_cursor_reproduces_frm_transparencys_own_three_declared_offsets() {
        assert_cursor_reproduces_declared_offsets(
            "vb6-code/Transparency-2D/frmTransparency.frm",
            "vb6-code/Transparency-2D/frmTransparency.frx",
        );
    }

    /// `Fast_Flames.exe`'s form recovers exactly one resource blob (its
    /// own `Icon`) with the builtin opcode table: every `PictureBox` in
    /// this corpus carries its own picture under an opcode the builtin
    /// table does not name, so this is the one reliable single-blob
    /// fixture the full write path can prove itself against end to end.
    #[test]
    fn the_written_frx_matches_fast_flames_byte_for_byte_through_the_full_write_path() {
        let (form, data) = first_form("vb6-code/Fire-effect/Fast_Flames.exe");
        let (files, _items) = write_form(&form, &data).expect("write_form must succeed");
        let committed = std::fs::read(corpus_path("vb6-code/Fire-effect/frmFire.frx"))
            .expect("reading the committed .frx");
        assert_eq!(
            files.frx.as_deref(),
            Some(committed.as_slice()),
            "the written .frx must equal the committed one byte for byte"
        );
    }

    #[test]
    fn the_last_record_of_a_written_frx_ends_exactly_at_the_end_of_the_file() {
        let (form, data) = first_form("vb6-code/Fire-effect/Fast_Flames.exe");
        let (files, _items) = write_form(&form, &data).expect("write_form must succeed");
        let frx = files.frx.expect("this form names at least one blob");
        let committed = std::fs::read(corpus_path("vb6-code/Fire-effect/frmFire.frx"))
            .expect("reading the committed .frx");
        assert_eq!(frx.len(), committed.len());
    }

    #[test]
    fn a_second_forms_own_first_blob_still_starts_at_offset_zero() {
        let (form_a, data_a) = first_form("vb6-code/Fire-effect/Fast_Flames.exe");
        write_form(&form_a, &data_a).expect("write_form must succeed");

        let (form_b, data_b) =
            first_form("public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe");
        let (files_b, _items_b) = write_form(&form_b, &data_b).expect("write_form must succeed");
        let frm_text = String::from_utf8_lossy(&files_b.frm);
        assert!(
            frm_text.contains(":0000"),
            "the second form's own first blob must start at offset zero, unaffected by the \
             first call's own cursor: {frm_text}"
        );
    }

    #[test]
    fn empty_picture_record_is_exactly_twelve_bytes_and_the_marker_sits_at_byte_four() {
        assert_eq!(EMPTY_PICTURE_RECORD.len(), 12);
        assert_eq!(&EMPTY_PICTURE_RECORD[4..8], &[0x6C, 0x74, 0x00, 0x00]);
        let mut cursor = BlobCursor::new();
        let mut frx = Vec::new();
        let offset =
            write_empty_picture_record(&mut cursor, &mut frx).expect("must not overflow a u32");
        assert_eq!(offset, 0);
        assert_eq!(frx, EMPTY_PICTURE_RECORD.to_vec());
    }

    #[test]
    fn a_form_with_no_blob_produces_no_resource_file() {
        let (name, _faults) = SafeName::new("frmNoBlob", NameKind::Form);
        let control = ControlModel {
            name: name.clone(),
            kind: ControlKind::Form,
            array_index: None,
            parent: None,
            depth: 0,
            is_menu: false,
            is_external: false,
            properties: vec![PropertyValue::Boolean {
                name: "Visible".to_owned(),
                value: -1,
            }],
        };
        let form = FormModel {
            name,
            tree_refused: false,
            controls: vec![control],
            procedures: Vec::new(),
            blobs: Vec::new(),
        };
        let (files, _items) = write_form(&form, &[]).expect("write_form must succeed");
        assert!(files.frx.is_none());
    }

    #[test]
    fn an_unreadable_blob_gives_a_report_item_and_does_not_advance_the_cursor() {
        let (name, _faults) = SafeName::new("frmBad", NameKind::Form);
        let control = ControlModel {
            name: name.clone(),
            kind: ControlKind::Form,
            array_index: None,
            parent: None,
            depth: 0,
            is_menu: false,
            is_external: false,
            properties: vec![
                PropertyValue::BlobUnreadable {
                    name: "Icon".to_owned(),
                    offset: 0x10,
                },
                PropertyValue::Blob {
                    name: "Picture".to_owned(),
                    offset: 0,
                    declared_len: 8,
                    image_len: 0,
                    format: crate::vb::frx::ImageFormat::Unknown(Vec::new()),
                    frx_offset: 0,
                },
            ],
        };
        let form = FormModel {
            name,
            tree_refused: false,
            controls: vec![control],
            procedures: Vec::new(),
            blobs: Vec::new(),
        };
        let mut data = 8_u32.to_le_bytes().to_vec();
        data.extend_from_slice(&[0u8; 8]);

        let (files, items) = write_form(&form, &data).expect("write_form must succeed");
        assert!(
            items
                .iter()
                .any(|item| item.confidence == crate::report::Confidence::Unrecoverable),
            "{items:?}"
        );
        let frm_text = String::from_utf8_lossy(&files.frm);
        assert!(
            frm_text.contains(":0000"),
            "the real blob after the unreadable one must still start at offset zero: {frm_text}"
        );
    }

    #[test]
    fn a_blobs_own_frx_offset_is_assigned_in_sorted_emission_order_not_raw_stream_order() {
        let (name, _faults) = SafeName::new("frmOrder", NameKind::Form);
        let mut data = vec![0u8; 200];
        // "Apple" lives at file offset 100: declared_len 8, header all 0xAA.
        data[100..104].copy_from_slice(&8_u32.to_le_bytes());
        data[104..112].copy_from_slice(&[0xAA; 8]);
        // "Zebra" lives at file offset 0: declared_len 8, header all 0xBB.
        data[0..4].copy_from_slice(&8_u32.to_le_bytes());
        data[4..12].copy_from_slice(&[0xBB; 8]);

        let control = ControlModel {
            name: name.clone(),
            kind: ControlKind::Form,
            array_index: None,
            parent: None,
            depth: 0,
            is_menu: false,
            is_external: false,
            properties: vec![
                // Raw stream order: Zebra, then Apple.
                PropertyValue::Blob {
                    name: "Zebra".to_owned(),
                    offset: 0,
                    declared_len: 8,
                    image_len: 0,
                    format: crate::vb::frx::ImageFormat::Unknown(Vec::new()),
                    frx_offset: 0,
                },
                PropertyValue::Blob {
                    name: "Apple".to_owned(),
                    offset: 100,
                    declared_len: 8,
                    image_len: 0,
                    format: crate::vb::frx::ImageFormat::Unknown(Vec::new()),
                    frx_offset: 0,
                },
            ],
        };
        let form = FormModel {
            name,
            tree_refused: false,
            controls: vec![control],
            procedures: Vec::new(),
            blobs: Vec::new(),
        };

        let (files, _items) = write_form(&form, &data).expect("write_form must succeed");
        let frx = files.frx.expect("this form names at least one blob");

        // Alphabetically, "Apple" precedes "Zebra", so Apple's own header
        // bytes (0xAA) must be packed first in the .frx, at offset 0, even
        // though the raw property stream held Zebra first.
        assert_eq!(
            frx.get(4..12),
            Some([0xAAu8; 8].as_slice()),
            "Apple's own bytes must be packed first: {frx:?}"
        );
        assert_eq!(
            frx.get(16..24),
            Some([0xBBu8; 8].as_slice()),
            "Zebra's own bytes must follow, at .frx offset 12: {frx:?}"
        );

        let frm_text = String::from_utf8_lossy(&files.frm);
        let apple_pos = frm_text.find("Apple").expect("an Apple line must exist");
        let zebra_pos = frm_text.find("Zebra").expect("a Zebra line must exist");
        assert!(
            apple_pos < zebra_pos,
            "Apple must be written before Zebra: {frm_text}"
        );
        assert!(frm_text.contains("\"frmOrder.frx\":0000"), "{frm_text}");
        assert!(frm_text.contains("\"frmOrder.frx\":000C"), "{frm_text}");
    }
}
