//! The `.bas`/`.cls` writer: the two fixed preambles, the empty procedure
//! signature every recovered procedure becomes, and the code region
//! emitter `write::frm` (plan 04-04) shares.
//!
//! `.planning/research/FILE-FORMATS.md` section 5: the `.bas` header is
//! one line, and the `.cls` preamble is thirteen lines, byte identical
//! across the whole corpus apart from the name. This file writes exactly
//! those fixed shapes. The class grammar and the form grammar look alike
//! and are not alike: the class block pads a name to twenty and writes one
//! space after the equals sign; the form block (`write::frm`, plan 04-04)
//! pads to sixteen and writes three. No line template helper is shared
//! between them.
//!
//! No statement of Visual Basic is ever written into a procedure body:
//! this milestone recovers metadata only. Plan 04-05 completes this file
//! across three tasks: the two preambles (this task), the empty procedure
//! signature, and the shared code region.

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

// --- Plan 04-05, Task 1: the two preambles, measured apart -----------------

/// Which of the two file kinds share the five line `Attribute` block: a
/// `.cls` class or a `.frm` form. A `.bas` module writes only `Attribute
/// VB_Name` and never reaches this type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributeFileKind {
    /// A `.cls` class: `VB_Creatable = True`, `VB_PredeclaredId = False`.
    Class,
    /// A `.frm` form: `VB_Creatable = False`, `VB_PredeclaredId = True`.
    Form,
}

/// Builds the five `Attribute` lines every `.cls` and `.frm` file writes,
/// always all five, in this fixed order, every time.
/// `.planning/research/FILE-FORMATS.md` section 5.2 and section 9 gap 3: no
/// corpus file omits one, and whether the loader tolerates a missing
/// attribute is unverified, so the safe default is to write all five
/// always.
///
/// Two of the five values differ between a class and a form
/// (`VB_Creatable`, `VB_PredeclaredId`); the other three
/// (`VB_GlobalNameSpace`, `VB_Exposed`, and `VB_Name` itself carrying
/// `name`) do not. One function knows all five keys, so the class writer
/// in this file and the form writer in `write::frm` (plan 04-04) call the
/// same one and cannot drift apart on which three agree.
#[must_use]
pub fn form_attribute_block(name: &SafeName, kind: AttributeFileKind) -> Vec<String> {
    let (creatable, predeclared) = match kind {
        AttributeFileKind::Class => ("True", "False"),
        AttributeFileKind::Form => ("False", "True"),
    };
    vec![
        format!("Attribute VB_Name = \"{}\"", name.as_str()),
        "Attribute VB_GlobalNameSpace = False".to_owned(),
        format!("Attribute VB_Creatable = {creatable}"),
        format!("Attribute VB_PredeclaredId = {predeclared}"),
        "Attribute VB_Exposed = False".to_owned(),
    ]
}

/// The field width the three longest names in [`CLS_PREAMBLE_PROPERTIES`]
/// pad exactly to. Measured by hand this session, byte for byte, from
/// `corpus/vb6-code/Fire-effect/FastDrawing.cls`: `DataBindingBehavior` (19
/// characters) gets 1 space (19 + 1 = 20); `DataSourceBehavior` and
/// `MTSTransactionMode` (18 characters each) get 2 spaces (18 + 2 = 20).
/// All three land on this width. `MultiUse` (8 characters) and
/// `Persistable` (11 characters) do not: both get exactly 1 space
/// regardless, far short of this width. This is not a general "pad to 20"
/// formula. A formula that computed `20 - name.len()` for every row would
/// give `MultiUse` 12 spaces, not the 1 the corpus proves. So
/// [`CLS_PREAMBLE_PROPERTIES`] stores each row's own measured space count
/// directly, and this constant documents the width the three longest rows
/// happen to share rather than driving the calculation.
pub const CLS_NAME_PAD: usize = 20;

/// The five `.cls` property lines a Standard EXE project writes, byte
/// identical across the whole 46 file corpus:
/// `(name, spaces before "=", value, comment word)`. Held as ordered data,
/// not as five separate `push_line` calls, so a reader can compare this
/// table against `.planning/research/FILE-FORMATS.md` section 5.2 without
/// reading control flow, and so a test can assert the whole block in one
/// place. Values proven **(a)** 46 of 46: for a Standard EXE project all
/// five properties are inert, all take `0` except `MultiUse`, which the
/// IDE writes as `-1`.
pub(crate) const CLS_PREAMBLE_PROPERTIES: [(&str, usize, &str, &str); 5] = [
    ("MultiUse", 1, "-1", "True"),
    ("Persistable", 1, "0", "NotPersistable"),
    ("DataBindingBehavior", 1, "0", "vbNone"),
    ("DataSourceBehavior", 2, "0", "vbNone"),
    ("MTSTransactionMode", 2, "0", "NotAnMTSObject"),
];

/// Renders one `.cls` property line: a two space indent, the name, its own
/// measured space count, `=`, one space, the value, two spaces, then the
/// comment word behind an apostrophe.
#[must_use]
fn cls_property_line(name: &str, spaces_before_eq: usize, value: &str, comment: &str) -> String {
    let padding = " ".repeat(spaces_before_eq);
    format!("  {name}{padding}= {value}  '{comment}")
}

/// Builds the thirteen line `.cls` preamble: the version line, `BEGIN`,
/// the five property lines [`CLS_PREAMBLE_PROPERTIES`] gives, `END`, then
/// the five `Attribute` lines [`form_attribute_block`] gives for
/// [`AttributeFileKind::Class`]. The version line's minor part is a single
/// digit: `VERSION 1.0 CLASS`, never `VERSION 5.00`, per
/// `.planning/research/FILE-FORMATS.md` section 5.2's own layout note.
#[must_use]
pub fn cls_preamble_lines(name: &SafeName) -> Vec<String> {
    let mut lines = vec!["VERSION 1.0 CLASS".to_owned(), "BEGIN".to_owned()];
    for (prop_name, spaces, value, comment) in CLS_PREAMBLE_PROPERTIES {
        lines.push(cls_property_line(prop_name, spaces, value, comment));
    }
    lines.push("END".to_owned());
    lines.extend(form_attribute_block(name, AttributeFileKind::Class));
    lines
}

/// Builds the one line `.bas` header: `.planning/research/FILE-FORMATS.md`
/// section 5.1. There is no `VERSION` line and no `BEGIN` block for a
/// module: the whole header is this one line, and the code region starts
/// on the next line.
#[must_use]
pub fn bas_header_line(name: &SafeName) -> String {
    format!("Attribute VB_Name = \"{}\"", name.as_str())
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
        AttributeFileKind, CLS_NAME_PAD, CLS_PREAMBLE_PROPERTIES, bas_header_line,
        cls_preamble_lines, form_attribute_block,
    };
    use crate::write::model::{NameKind, SafeName};

    fn name(raw: &str) -> SafeName {
        SafeName::new(raw, NameKind::Module).0
    }

    // --- Task 1: the two preambles -----------------------------------

    #[test]
    fn a_written_module_files_first_line_is_the_one_line_header() {
        assert_eq!(
            bas_header_line(&name("Logic_Module")),
            "Attribute VB_Name = \"Logic_Module\""
        );
    }

    #[test]
    fn a_module_file_has_no_version_line_and_no_begin_block() {
        let header = bas_header_line(&name("Logic_Module"));
        assert!(!header.contains("VERSION"), "{header:?}");
        assert!(!header.contains("BEGIN"), "{header:?}");
    }

    #[test]
    fn the_thirteen_class_preamble_lines_match_the_corpus_byte_for_byte() {
        let lines = cls_preamble_lines(&name("FastDrawing"));
        assert_eq!(
            lines,
            vec![
                "VERSION 1.0 CLASS".to_owned(),
                "BEGIN".to_owned(),
                "  MultiUse = -1  'True".to_owned(),
                "  Persistable = 0  'NotPersistable".to_owned(),
                "  DataBindingBehavior = 0  'vbNone".to_owned(),
                "  DataSourceBehavior  = 0  'vbNone".to_owned(),
                "  MTSTransactionMode  = 0  'NotAnMTSObject".to_owned(),
                "END".to_owned(),
                "Attribute VB_Name = \"FastDrawing\"".to_owned(),
                "Attribute VB_GlobalNameSpace = False".to_owned(),
                "Attribute VB_Creatable = True".to_owned(),
                "Attribute VB_PredeclaredId = False".to_owned(),
                "Attribute VB_Exposed = False".to_owned(),
            ]
        );
        assert_eq!(lines.len(), 13);
    }

    #[test]
    fn the_class_property_line_for_the_longest_name_has_one_space_after_the_equals_sign() {
        let (property_name, _spaces, value, _comment) = CLS_PREAMBLE_PROPERTIES[2];
        assert_eq!(property_name, "DataBindingBehavior");
        let lines = cls_preamble_lines(&name("X"));
        let line = lines
            .iter()
            .find(|line| line.starts_with("  DataBindingBehavior"))
            .expect("the DataBindingBehavior line must exist");
        assert_eq!(line, &format!("  DataBindingBehavior = {value}  'vbNone"));
        // Exactly one space between "=" and the value: this is the space
        // count this session measured by hand and records in the SUMMARY.
        assert!(line.contains("= 0"), "{line:?}");
        assert!(!line.contains("=  0"), "{line:?}");
    }

    #[test]
    fn cls_name_pad_is_twenty() {
        assert_eq!(CLS_NAME_PAD, 20);
    }

    #[test]
    fn the_attribute_block_is_always_five_lines_for_a_class() {
        let lines = form_attribute_block(&name("FastDrawing"), AttributeFileKind::Class);
        assert_eq!(lines.len(), 5);
        assert!(lines.contains(&"Attribute VB_Creatable = True".to_owned()));
        assert!(lines.contains(&"Attribute VB_PredeclaredId = False".to_owned()));
    }

    #[test]
    fn the_attribute_block_is_always_five_lines_for_a_form() {
        let lines = form_attribute_block(&name("frmFire"), AttributeFileKind::Form);
        assert_eq!(lines.len(), 5);
        assert!(lines.contains(&"Attribute VB_Creatable = False".to_owned()));
        assert!(lines.contains(&"Attribute VB_PredeclaredId = True".to_owned()));
    }

    #[test]
    fn the_class_and_form_attribute_blocks_differ_in_exactly_two_values_and_agree_in_three() {
        let class = form_attribute_block(&name("Same"), AttributeFileKind::Class);
        let form = form_attribute_block(&name("Same"), AttributeFileKind::Form);
        assert_eq!(class.len(), 5);
        assert_eq!(form.len(), 5);
        let differing = class
            .iter()
            .zip(form.iter())
            .filter(|(class_line, form_line)| class_line != form_line)
            .count();
        assert_eq!(differing, 2, "class: {class:?}, form: {form:?}");
        // Names all five keys, in order: VB_Name, VB_GlobalNameSpace,
        // VB_Creatable, VB_PredeclaredId, VB_Exposed.
        for line in &class {
            assert!(
                line.starts_with("Attribute VB_Name")
                    || line.starts_with("Attribute VB_GlobalNameSpace")
                    || line.starts_with("Attribute VB_Creatable")
                    || line.starts_with("Attribute VB_PredeclaredId")
                    || line.starts_with("Attribute VB_Exposed"),
                "{line:?}"
            );
        }
    }

    #[test]
    fn cls_preamble_properties_holds_exactly_five_entries() {
        assert_eq!(CLS_PREAMBLE_PROPERTIES.len(), 5);
    }
}
