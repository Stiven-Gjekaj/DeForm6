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
use crate::vb::controltree::ControlKind;
use crate::vb::frx::{self, BlobCursor};
use crate::vb::propstream::{FontBlock, PositionBlock, PropertyValue};
use crate::vb::{ControlReport, FormReport};

use super::model::{LineWriter, SafeName};

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
        let name = SafeName::new(&root.name);
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
    let name = SafeName::new(&control.name);
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
