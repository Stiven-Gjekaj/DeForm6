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
//! this milestone recovers metadata only, and every procedure this file
//! writes is a signature line, an empty body, and a closing line, nothing
//! more.

use super::model::{LineWriter, SafeName};
use crate::report::{Confidence, ReportItem};
use crate::vb::functyp::{Argument, DefaultValue, PropertyKind, Prototype, TypeEntry, VbType};
use crate::vb::{ObjectProcedures, ProcedureEntry};
use crate::write::values::escape_inline_string;

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

// --- Plan 04-05, Task 2: the empty procedure with the right signature ------

/// One written procedure's own signature line and its matching closing
/// line. The body between them holds nothing: not a comment, not a
/// placeholder statement, nothing. Plan 04-07 owns the uncertainty
/// comment, and this plan writes no body line of any kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    /// The whole signature line: the scope word, the procedure, function
    /// or property word, the name, the argument list, and the return type
    /// when the procedure is a function.
    pub declaration: String,
    /// The matching closing line: `End Sub`, `End Function` or
    /// `End Property`.
    pub closing: &'static str,
}

/// Builds one procedure's [`Signature`]: the scope word (`"Public"` or
/// `"Private"`), the procedure, function or property word, `name`, the
/// argument list in declaration order, and the return type when the
/// prototype names a function.
///
/// `prototype` is `None` for two different facts this crate never
/// collapses into one: a public procedure whose type descriptor did not
/// resolve (its argument list is not in the file), and a private slot,
/// which this crate never invents a name for (`OBJ-06`). Both give the
/// same shape here, a no argument procedure, because neither carries an
/// argument list to write; the caller decides `name` and the scope word,
/// and records the difference as a distinct report item.
///
/// Reuses the exact join order `crates/deform6-cli/src/main.rs`'s own
/// `format_prototype`/`format_argument`/`format_type_entry`/
/// `format_vb_type`/`format_default` already established and measured
/// against the corpus in Phase 2: `Optional`/`ByRef` prefix, the name,
/// `()` for an array, `As <type>`, `= <default>`.
#[must_use]
pub fn format_signature(scope: &str, name: &str, prototype: Option<&Prototype>) -> Signature {
    let Some(prototype) = prototype else {
        return Signature {
            declaration: format!("{scope} Sub {name}()"),
            closing: "End Sub",
        };
    };

    let (head, closing) = signature_head(prototype);
    let args: Vec<String> = prototype
        .arguments
        .iter()
        .enumerate()
        .map(|(index, arg)| format_argument(arg, index))
        .collect();
    let mut declaration = format!("{scope} {head} {name}({})", args.join(", "));
    if let Some(return_type) = &prototype.return_type {
        declaration.push_str(" As ");
        declaration.push_str(&format_type_entry(return_type));
    }
    Signature {
        declaration,
        closing,
    }
}

/// Gives the head word (`"Sub"`, `"Function"`, `"Property Get"`,
/// `"Property Let"` or `"Property Set"`) and the matching closing line for
/// `prototype`. No wildcard arm: a sixth [`PropertyKind`] variant is a
/// compile error until this table names its own word.
fn signature_head(prototype: &Prototype) -> (&'static str, &'static str) {
    match prototype.property_kind {
        PropertyKind::None => {
            if prototype.return_type.is_some() {
                ("Function", "End Function")
            } else {
                ("Sub", "End Sub")
            }
        }
        PropertyKind::Get => ("Property Get", "End Property"),
        PropertyKind::Let => ("Property Let", "End Property"),
        PropertyKind::Set => ("Property Set", "End Property"),
    }
}

/// Renders one argument: the `Optional`/`ByRef` prefix, the name, `()` for
/// an array, ` As <type>`, and ` = <default>` when the reader recovered
/// one. `index` names this argument's own position in the declaration,
/// used only to build a placeholder when the reader's own name did not
/// resolve (an empty `Argument::name`, a real and documented gap on
/// `Argument::name`): a blank identifier is not legal Visual Basic, and
/// this crate's whole purpose is a project that still builds.
fn format_argument(arg: &Argument, index: usize) -> String {
    let name = if arg.name.is_empty() {
        format!("Arg{}", index.saturating_add(1))
    } else {
        arg.name.clone()
    };

    let mut prefix = String::new();
    if arg.entry.optional {
        prefix.push_str("Optional ");
    }
    if arg.entry.by_ref {
        prefix.push_str("ByRef ");
    }

    let mut piece = format!("{prefix}{name}");
    if arg.entry.array {
        piece.push_str("()");
    }
    piece.push_str(" As ");
    piece.push_str(&format_type_entry(&arg.entry));
    if let Some(default) = &arg.default {
        piece.push_str(" = ");
        piece.push_str(&format_default(default));
    }
    piece
}

/// Renders a [`TypeEntry`]'s own base type. Modifiers are the caller's own
/// job: this function only ever names the type after `As`.
fn format_type_entry(entry: &TypeEntry) -> String {
    format_vb_type(&entry.vb_type)
}

/// Renders a [`VbType`] as the legal Visual Basic keyword this crate
/// writes into a signature line. No wildcard arm: a type code this table
/// does not hold is a compile error until somebody decides its word.
///
/// Three variants this crate cannot resolve to a real class or interface
/// name (`Internal`, `ComIFace`, `ComObj`, each carrying only a raw,
/// unresolved address) write the same word a plain `VbType::Object` does:
/// `Object` is the one legal Visual Basic keyword that covers all of
/// them, and this crate never writes the raw address into the signature
/// line itself, since that address is not a type name. `Unknown` (a type
/// code this table has never measured) writes `Variant`, the one type
/// every value can hold. `HResult` writes `Long`, its own underlying COM
/// representation, since Visual Basic has no `HResult` keyword. All four
/// substitutions exist for one reason: `WRT-07` asks for a project that
/// still builds, and a signature line naming an invented keyword, or no
/// keyword at all, would not build.
fn format_vb_type(vb_type: &VbType) -> String {
    match vb_type {
        VbType::Boolean => "Boolean".to_owned(),
        VbType::Byte => "Byte".to_owned(),
        VbType::Integer => "Integer".to_owned(),
        VbType::Long => "Long".to_owned(),
        VbType::Single => "Single".to_owned(),
        VbType::Double => "Double".to_owned(),
        VbType::Date => "Date".to_owned(),
        VbType::Currency => "Currency".to_owned(),
        VbType::Variant => "Variant".to_owned(),
        VbType::Str => "String".to_owned(),
        VbType::Object | VbType::Internal(_) | VbType::ComIFace(_) | VbType::ComObj(_) => {
            "Object".to_owned()
        }
        VbType::HResult => "Long".to_owned(),
        VbType::Unknown(_) => "Variant".to_owned(),
    }
}

/// Renders an `Optional` argument's own recovered default as a legal
/// Visual Basic literal. No wildcard arm.
///
/// `Boolean` writes the capitalised keywords `True`/`False`: Rust's own
/// `bool` display gives the lower case `true`/`false`, which is not a
/// legal Visual Basic literal and would not compile. `Text` is escaped
/// through [`escape_inline_string`] (plan 04-02's own inline string rule):
/// wrapped in double quotes with every inner double quote doubled, never
/// through Rust's own debug formatting, which escapes with a backslash
/// Visual Basic does not accept.
fn format_default(default: &DefaultValue) -> String {
    match default {
        DefaultValue::Empty => "Empty".to_owned(),
        DefaultValue::Integer(value) => value.to_string(),
        DefaultValue::Single(value) => value.to_string(),
        DefaultValue::Boolean(value) => (if *value { "True" } else { "False" }).to_owned(),
        DefaultValue::Byte(value) => value.to_string(),
        DefaultValue::Text(value) => escape_inline_string(value),
    }
}

/// The generated name a private procedure's own signature uses. `OBJ-06`
/// forbids inventing a recovered name; this is not one. The file holds a
/// null where the real name would be, and this crate must still write a
/// legal Visual Basic identifier for the `Sub` statement to exist at all.
/// `index` is this procedure's own position in the object's procedure
/// array, so two private slots in the same object never collide.
fn generated_procedure_name(index: usize) -> String {
    format!("UnnamedProcedure{index}")
}

/// Builds one procedure slot's own [`Signature`] and, when the slot is not
/// a full recovery, the one [`ReportItem`] naming why.
///
/// The three procedure shapes this crate ever meets are three different
/// facts about the file, and each gets its own report item rather than
/// being collapsed into "no signature": a public procedure with a
/// prototype is a full recovery and gets none; a public procedure with no
/// prototype is a name without a shape (every procedure in a standard
/// module meets this, since a standard module carries no type
/// descriptors at all); a private slot is a name the file marks null.
/// `path` is left empty: this function knows only the procedure, never the
/// object or the form it belongs to, so the caller fills in the real path
/// before the item enters the project report, the same convention
/// `write::values::format_value` already established.
#[must_use]
pub fn format_procedure_entry(
    entry: &ProcedureEntry,
    index: usize,
) -> (Signature, Option<ReportItem>) {
    match entry {
        ProcedureEntry::Public { name, prototype } => match prototype {
            Some(prototype) => (format_signature("Public", name, Some(prototype)), None),
            None => {
                let signature = format_signature("Public", name, None);
                let item = ReportItem {
                    path: String::new(),
                    confidence: Confidence::Inferred,
                    basis: format!(
                        "{name} has no recovered prototype; this object's own type \
                         descriptor array did not resolve one, which happens for every \
                         procedure in a standard module by construction, so the argument \
                         list is not in the file"
                    ),
                    evidence: Vec::new(),
                };
                (signature, Some(item))
            }
        },
        ProcedureEntry::Private => {
            let name = generated_procedure_name(index);
            let signature = format_signature("Private", &name, None);
            let item = ReportItem {
                path: String::new(),
                confidence: Confidence::Unrecoverable,
                basis: format!(
                    "the file holds a null where this procedure's own name would be; \
                     {name} is generated for this signature, never recovered"
                ),
                evidence: Vec::new(),
            };
            (signature, Some(item))
        }
    }
}

/// Builds every procedure line `procedures` gives, in the reader's own
/// array order, plus every [`ReportItem`] the three procedure shapes and
/// the no-name-array shape produced.
///
/// An object that carries no procedure name array at all
/// ([`ObjectProcedures::NoNameArray`]) writes no signature at all: the
/// count is real (`proc_count`, read from the file), and the names are
/// genuinely not there to write. A report item carrying the count is the
/// honest answer; writing zero signatures with no item would read as an
/// object with no procedures, which is a different, false claim.
#[must_use]
pub fn format_procedures(procedures: &ObjectProcedures) -> (Vec<String>, Vec<ReportItem>) {
    let mut lines = Vec::new();
    let mut items = Vec::new();

    match procedures {
        ObjectProcedures::NoNameArray { proc_count } => {
            items.push(ReportItem {
                path: String::new(),
                confidence: Confidence::Unrecoverable,
                basis: format!(
                    "this object declares {proc_count} procedure slot(s), but it carries \
                     no procedure name array at all; no signature can be written for any \
                     of them"
                ),
                evidence: Vec::new(),
            });
        }
        ObjectProcedures::Slots(entries) => {
            for (index, entry) in entries.iter().enumerate() {
                let (signature, item) = format_procedure_entry(entry, index);
                lines.push(signature.declaration);
                lines.push(signature.closing.to_owned());
                if let Some(item) = item {
                    items.push(item);
                }
            }
        }
    }

    (lines, items)
}

// --- Plan 04-05, Task 3: the shared code region, and determinism ----------

/// Appends one [`ReportItem`] naming every character this call's own
/// writer could not represent in Windows-1252, when there is at least one.
/// `.planning/research/FILE-FORMATS.md` section 6.1: the emitter never
/// falls back to UTF-8; it substitutes `?` and records the substitution.
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

/// Emits the code region every `.bas`, `.cls` and `.frm` file shares:
/// [`crate::write::comment::uncertainty_comments`]'s own lines for every
/// `items` entry at or under `path_prefix`, then one empty procedure per
/// procedure slot `procedures` recovered, in the reader's own array order.
///
/// This is the one call site [`crate::write::comment::uncertainty_comments`]
/// has in the whole phase. The three file writers in this phase (this
/// file's own [`write_bas`] and [`write_cls`], and `write::frm`'s form
/// writer) all call this one function, and none of them calls the comment
/// emitter itself: a comment can enter the written output only through
/// this one place. None of the three writers holds a code region of its
/// own: the code region is genuinely the same in all three file kinds,
/// unlike the line templates above it, which differ in all three.
///
/// Reaches no process global mutable state: everything this function needs
/// comes in as a parameter, and everything it produces comes back as a
/// return value. Two calls with the same input give byte identical output.
#[must_use]
pub fn write_code_region(
    items: &[ReportItem],
    path_prefix: &str,
    procedures: &ObjectProcedures,
) -> (Vec<String>, Vec<ReportItem>) {
    let mut lines: Vec<String> = crate::write::comment::uncertainty_comments(items, path_prefix);
    let (procedure_lines, procedure_items) = format_procedures(procedures);
    lines.extend(procedure_lines);
    (lines, procedure_items)
}

/// Writes the complete `.cls` file: the thirteen line preamble
/// ([`cls_preamble_lines`]), then the shared code region
/// ([`write_code_region`]) built from `items`, `path_prefix` and
/// `procedures`. Every line, including the last, ends with the two line
/// ending bytes; the file carries no byte order mark. Gives the file's own
/// bytes and every [`ReportItem`] this call produced.
#[must_use]
pub fn write_cls(
    name: &SafeName,
    procedures: &ObjectProcedures,
    items: &[ReportItem],
    path_prefix: &str,
) -> (Vec<u8>, Vec<ReportItem>) {
    let mut writer = LineWriter::new();
    for line in cls_preamble_lines(name) {
        writer.push_line(&line);
    }
    let (region_lines, mut result_items) = write_code_region(items, path_prefix, procedures);
    for line in &region_lines {
        writer.push_line(line);
    }
    let (bytes, substituted) = writer.finish();
    push_substitution_item(&mut result_items, &substituted);
    (bytes, result_items)
}

/// Writes the complete `.bas` file: the one line header
/// ([`bas_header_line`]), then the shared code region
/// ([`write_code_region`]). Gives the file's own bytes and every
/// [`ReportItem`] this call produced.
#[must_use]
pub fn write_bas(
    name: &SafeName,
    procedures: &ObjectProcedures,
    items: &[ReportItem],
    path_prefix: &str,
) -> (Vec<u8>, Vec<ReportItem>) {
    let mut writer = LineWriter::new();
    writer.push_line(&bas_header_line(name));
    let (region_lines, mut result_items) = write_code_region(items, path_prefix, procedures);
    for line in &region_lines {
        writer.push_line(line);
    }
    let (bytes, substituted) = writer.finish();
    push_substitution_item(&mut result_items, &substituted);
    (bytes, result_items)
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
        cls_preamble_lines, form_attribute_block, write_bas, write_cls, write_code_region,
    };
    use crate::report::{Confidence, Evidence, ReportItem};
    use crate::vb::{ObjectProcedures, ProcedureEntry};
    use crate::write::model::{NameKind, SafeName};

    fn item(path: &str, confidence: Confidence, basis: &str) -> ReportItem {
        ReportItem {
            path: path.to_owned(),
            confidence,
            basis: basis.to_owned(),
            evidence: vec![Evidence {
                offset: 0x10,
                structure: "Test",
                field: "fixture",
                note: None,
            }],
        }
    }

    fn name(raw: &str) -> SafeName {
        SafeName::new(raw, NameKind::Module).0
    }

    fn text(bytes: &[u8]) -> String {
        bytes.iter().copied().map(char::from).collect()
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
        let (bytes, _items) = write_bas(
            &name("Logic_Module"),
            &ObjectProcedures::Slots(vec![]),
            &[],
            "/modules/Logic_Module",
        );
        let content = text(&bytes);
        assert!(!content.contains("VERSION"), "{content:?}");
        assert!(!content.contains("BEGIN"), "{content:?}");
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

    // --- Task 3: the shared code region and determinism ----------------

    #[test]
    fn write_bas_called_twice_on_one_input_gives_byte_identical_output() {
        let procedures = ObjectProcedures::Slots(vec![ProcedureEntry::Private]);
        let (first, first_items) = write_bas(&name("Mod1"), &procedures, &[], "/modules/Mod1");
        let (second, second_items) = write_bas(&name("Mod1"), &procedures, &[], "/modules/Mod1");
        assert_eq!(first, second);
        assert_eq!(first_items, second_items);
    }

    #[test]
    fn write_cls_called_twice_on_one_input_gives_byte_identical_output() {
        let procedures = ObjectProcedures::Slots(vec![]);
        let (first, first_items) = write_cls(&name("Cls1"), &procedures, &[], "/classes/Cls1");
        let (second, second_items) = write_cls(&name("Cls1"), &procedures, &[], "/classes/Cls1");
        assert_eq!(first, second);
        assert_eq!(first_items, second_items);
    }

    #[test]
    fn a_written_cls_files_last_two_bytes_are_the_line_ending_pair_and_lf_count_equals_pair_count()
    {
        let (bytes, _items) = write_cls(
            &name("Cls1"),
            &ObjectProcedures::Slots(vec![]),
            &[],
            "/classes/Cls1",
        );
        assert!(bytes.ends_with(b"\r\n"));
        let crlf_pairs = bytes.windows(2).filter(|window| *window == b"\r\n").count();
        let lf_count = bytes.iter().filter(|byte| **byte == b'\n').count();
        assert_eq!(crlf_pairs, lf_count);
    }

    #[test]
    fn no_written_file_begins_with_a_byte_order_mark() {
        let (cls_bytes, _items) = write_cls(
            &name("Cls1"),
            &ObjectProcedures::Slots(vec![]),
            &[],
            "/classes/Cls1",
        );
        let (bas_bytes, _items) = write_bas(
            &name("Mod1"),
            &ObjectProcedures::Slots(vec![]),
            &[],
            "/modules/Mod1",
        );
        assert!(!cls_bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
        assert!(!bas_bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    }

    #[test]
    fn a_character_above_0xff_in_a_comments_own_basis_produces_a_recorded_substitution() {
        let items = vec![item(
            "/classes/Cls1",
            Confidence::Unrecoverable,
            "caf\u{e9}\u{20ac}",
        )];
        let (_bytes, result_items) = write_cls(
            &name("Cls1"),
            &ObjectProcedures::Slots(vec![]),
            &items,
            "/classes/Cls1",
        );
        let substitution = result_items
            .iter()
            .find(|result_item| result_item.basis.contains('\u{20ac}'))
            .expect("a substitution item must be recorded");
        assert_eq!(substitution.confidence, Confidence::Unrecoverable);
    }

    #[test]
    fn write_code_region_reaches_the_comment_emitter_and_supplies_no_comment_of_its_own() {
        let items = vec![item(
            "/modules/Mod1",
            Confidence::Unrecoverable,
            "a fact plan 04-07's own emitter names",
        )];
        let expected = crate::write::comment::uncertainty_comments(&items, "/modules/Mod1");
        assert!(!expected.is_empty(), "the fixture must produce a comment");

        let (lines, _items) =
            write_code_region(&items, "/modules/Mod1", &ObjectProcedures::Slots(vec![]));
        assert_eq!(lines, expected);
    }

    #[test]
    fn write_code_region_supplies_no_line_of_its_own_when_no_item_matches_the_prefix() {
        let (lines, _items) =
            write_code_region(&[], "/modules/Mod1", &ObjectProcedures::Slots(vec![]));
        assert!(lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn an_object_with_no_procedure_name_array_writes_no_lines_and_one_item_via_write_code_region() {
        let (lines, items) = write_code_region(
            &[],
            "/modules/Mod1",
            &ObjectProcedures::NoNameArray { proc_count: 7 },
        );
        assert!(lines.is_empty());
        assert_eq!(items.len(), 1);
        assert!(items[0].basis.contains('7'), "{items:?}");
    }

    // --- Task 2: the one call site, and the comment's own position -----

    #[test]
    fn a_written_module_files_first_comment_line_comes_after_its_one_header_line() {
        let items = vec![item(
            "/modules/Mod1",
            Confidence::Unrecoverable,
            "a fact about this module the reading side could not resolve",
        )];
        let (bytes, _items) = write_bas(
            &name("Mod1"),
            &ObjectProcedures::Slots(vec![]),
            &items,
            "/modules/Mod1",
        );
        let content = text(&bytes);
        let lines: Vec<&str> = content.split("\r\n").collect();
        let header_index = lines
            .iter()
            .position(|line| line.starts_with("Attribute VB_Name"))
            .expect("the one header line must exist");
        let comment_index = lines
            .iter()
            .position(|line| line.starts_with('\''))
            .expect("a comment line must exist");
        assert!(
            comment_index > header_index,
            "header at {header_index}, comment at {comment_index}: {lines:?}"
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod signatures {
    use super::{format_procedure_entry, format_procedures, format_signature};
    use crate::read::region::Va;
    use crate::vb::functyp::{
        Argument, DefaultValue, OptionalDefaultsOutcome, PropertyKind, Prototype, TypeEntry, VbType,
    };
    use crate::vb::{ObjectProcedures, ProcedureEntry};

    fn entry(vb_type: VbType) -> TypeEntry {
        TypeEntry {
            vb_type,
            optional: false,
            array: false,
            by_ref: false,
        }
    }

    fn prototype(
        arguments: Vec<Argument>,
        is_function: bool,
        return_type: Option<TypeEntry>,
    ) -> Prototype {
        Prototype {
            member_id: 0x6003_0001,
            v_off: 0,
            const_ffff: 0xFFFF,
            nul1: 0,
            property_kind: PropertyKind::None,
            is_function,
            arguments,
            return_type,
            optional_defaults: OptionalDefaultsOutcome::NoOptionalVals,
        }
    }

    #[test]
    fn a_function_with_two_arguments_one_optional_and_one_an_array_gives_an_exact_signature() {
        let proto = prototype(
            vec![
                Argument {
                    name: "Bar".to_owned(),
                    entry: TypeEntry {
                        optional: true,
                        ..entry(VbType::Long)
                    },
                    default: None,
                },
                Argument {
                    name: "Baz".to_owned(),
                    entry: TypeEntry {
                        array: true,
                        ..entry(VbType::Variant)
                    },
                    default: None,
                },
            ],
            true,
            Some(entry(VbType::Long)),
        );
        let signature = format_signature("Public", "Foo", Some(&proto));
        assert_eq!(
            signature.declaration,
            "Public Function Foo(Optional Bar As Long, Baz() As Variant) As Long"
        );
        assert_eq!(signature.closing, "End Function");
    }

    #[test]
    fn a_by_reference_argument_carries_its_modifier_before_the_name() {
        let proto = prototype(
            vec![Argument {
                name: "Value".to_owned(),
                entry: TypeEntry {
                    by_ref: true,
                    ..entry(VbType::Integer)
                },
                default: None,
            }],
            false,
            None,
        );
        let signature = format_signature("Public", "Grow", Some(&proto));
        assert_eq!(
            signature.declaration,
            "Public Sub Grow(ByRef Value As Integer)"
        );
        assert_eq!(signature.closing, "End Sub");
    }

    #[test]
    fn an_argument_with_a_recovered_default_carries_it_after_the_type() {
        let proto = prototype(
            vec![Argument {
                name: "Flags".to_owned(),
                entry: TypeEntry {
                    optional: true,
                    ..entry(VbType::Long)
                },
                default: Some(DefaultValue::Integer(3)),
            }],
            false,
            None,
        );
        let signature = format_signature("Public", "Configure", Some(&proto));
        assert_eq!(
            signature.declaration,
            "Public Sub Configure(Optional Flags As Long = 3)"
        );
    }

    #[test]
    fn a_sub_with_no_arguments_and_no_return_type_gives_a_plain_signature() {
        let proto = prototype(vec![], false, None);
        let signature = format_signature("Public", "Tick", Some(&proto));
        assert_eq!(signature.declaration, "Public Sub Tick()");
        assert_eq!(signature.closing, "End Sub");
    }

    #[test]
    fn a_property_get_gives_property_get_head_and_end_property_closing() {
        let mut proto = prototype(vec![], true, Some(entry(VbType::Long)));
        proto.property_kind = PropertyKind::Get;
        let signature = format_signature("Public", "Count", Some(&proto));
        assert_eq!(signature.declaration, "Public Property Get Count() As Long");
        assert_eq!(signature.closing, "End Property");
    }

    #[test]
    fn a_property_let_gives_property_let_head_and_end_property_closing() {
        let mut proto = prototype(
            vec![Argument {
                name: "Value".to_owned(),
                entry: entry(VbType::Long),
                default: None,
            }],
            false,
            None,
        );
        proto.property_kind = PropertyKind::Let;
        let signature = format_signature("Public", "Count", Some(&proto));
        assert_eq!(
            signature.declaration,
            "Public Property Let Count(Value As Long)"
        );
        assert_eq!(signature.closing, "End Property");
    }

    #[test]
    fn format_signature_never_produces_a_body_line() {
        let proto = prototype(vec![], false, None);
        let signature = format_signature("Public", "Tick", Some(&proto));
        let lines = vec![signature.declaration, signature.closing.to_owned()];
        assert_eq!(lines.len(), 2, "{lines:?}");
    }

    #[test]
    fn a_public_procedure_with_no_prototype_gives_a_no_argument_signature_and_one_inferred_item() {
        let entry_value = ProcedureEntry::Public {
            name: "Fire".to_owned(),
            prototype: None,
        };
        let (signature, item) = format_procedure_entry(&entry_value, 0);
        assert_eq!(signature.declaration, "Public Sub Fire()");
        let item = item.expect("a report item must be produced");
        assert_eq!(item.confidence, crate::report::Confidence::Inferred);
    }

    #[test]
    fn a_private_procedure_gives_a_private_no_argument_signature_under_a_generated_name() {
        let (signature, item) = format_procedure_entry(&ProcedureEntry::Private, 2);
        assert_eq!(signature.declaration, "Private Sub UnnamedProcedure2()");
        let item = item.expect("a report item must be produced");
        assert_eq!(item.confidence, crate::report::Confidence::Unrecoverable);
    }

    #[test]
    fn an_unknown_vb_type_writes_variant_and_a_com_type_writes_object() {
        let unknown_proto = prototype(
            vec![Argument {
                name: "Raw".to_owned(),
                entry: entry(VbType::Unknown(0x7F)),
                default: None,
            }],
            false,
            None,
        );
        let signature = format_signature("Public", "Weird", Some(&unknown_proto));
        assert_eq!(signature.declaration, "Public Sub Weird(Raw As Variant)");

        let com_proto = prototype(
            vec![Argument {
                name: "Obj".to_owned(),
                entry: entry(VbType::ComObj(Va::new(0x1000))),
                default: None,
            }],
            false,
            None,
        );
        let signature = format_signature("Public", "Take", Some(&com_proto));
        assert_eq!(signature.declaration, "Public Sub Take(Obj As Object)");
    }

    #[test]
    fn a_default_boolean_value_writes_the_capitalised_vb6_keyword_not_rusts_lower_case() {
        let proto = prototype(
            vec![Argument {
                name: "Flag".to_owned(),
                entry: TypeEntry {
                    optional: true,
                    ..entry(VbType::Boolean)
                },
                default: Some(DefaultValue::Boolean(true)),
            }],
            false,
            None,
        );
        let signature = format_signature("Public", "Toggle", Some(&proto));
        assert!(signature.declaration.contains("= True"), "{signature:?}");
        assert!(!signature.declaration.contains("true"), "{signature:?}");
    }

    #[test]
    fn an_empty_argument_name_gets_a_generated_placeholder_not_a_blank_identifier() {
        let proto = prototype(
            vec![Argument {
                name: String::new(),
                entry: entry(VbType::Long),
                default: None,
            }],
            false,
            None,
        );
        let signature = format_signature("Public", "Broken", Some(&proto));
        assert_eq!(signature.declaration, "Public Sub Broken(Arg1 As Long)");
    }

    #[test]
    fn an_object_with_no_procedure_name_array_gives_no_lines_and_one_item_naming_the_count() {
        let (lines, items) = format_procedures(&ObjectProcedures::NoNameArray { proc_count: 7 });
        assert!(lines.is_empty());
        assert_eq!(items.len(), 1);
        let item = items.first().expect("one report item must exist");
        assert!(item.basis.contains('7'), "{item:?}");
    }
}

// --- Plan 04-07, Task 2: the corpus wide sweep, proving the one call site
// reaches no other file kind and reached at least one real one ------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod sweep {
    use crate::vb::opcodes::OpcodeTable;
    use crate::vb::{ObjectProcedures, ProcedureEntry};
    use crate::write::model::{CodeKind, ProcedureModel, from_report};
    use std::path::{Path, PathBuf};

    fn corpus_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
    }

    fn executables() -> Vec<PathBuf> {
        let mut out = Vec::new();
        walk(&corpus_root(), &mut out);
        out.sort();
        out
    }

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let entry = entry.unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
            {
                out.push(path);
            }
        }
    }

    /// Turns this crate's own Windows-1252/Latin-1-as-codepoint bytes back
    /// into a `String`, the same convention every reader and writer in
    /// this crate already uses.
    fn text(bytes: &[u8]) -> String {
        bytes.iter().copied().map(char::from).collect()
    }

    /// Splits `text` on the CRLF pair every line writer in this crate
    /// terminates a line with.
    fn lines(text: &str) -> Vec<&str> {
        text.split("\r\n").collect()
    }

    /// True when `line`'s first non space character is an apostrophe: a
    /// real comment. A boolean or an enumeration property line also
    /// carries a trailing apostrophe partway through itself (the value
    /// decoration the IDE itself writes), which this check does not match,
    /// because it looks only at the first non space character, never at
    /// whether the line holds an apostrophe anywhere.
    fn is_a_comment_line(line: &str) -> bool {
        line.trim_start().starts_with('\'')
    }

    /// Mirrors `write::frm`'s own private `model_procedures_to_object_procedures`:
    /// a named procedure becomes a public slot, an unnamed one a private
    /// slot. Kept local to this test rather than shared, since that
    /// function is private to its own file, and this test needs it only to
    /// drive [`crate::write::code::write_cls`]/[`crate::write::code::write_bas`]
    /// with a real corpus object's own procedure count.
    fn object_procedures(procedures: &[ProcedureModel]) -> ObjectProcedures {
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

    #[test]
    fn no_written_text_file_carries_a_comment_above_its_own_boundary_and_at_least_one_program_carries_one_below_it()
     {
        let table = OpcodeTable::builtin();
        let mut comment_lines_below_boundary = 0_usize;
        let mut program_naming_one: Option<String> = None;

        for exe_path in executables() {
            let data = std::fs::read(&exe_path)
                .unwrap_or_else(|err| panic!("reading {}: {err}", exe_path.display()));
            let Ok(report) = crate::vb::inspect(&data, &table) else {
                continue;
            };
            let (model, _model_items) = from_report(&report, &data);

            // The `.vbp` has no code region and no comment syntax at all:
            // every line must fail the comment check, with no boundary to
            // split above from below.
            let (vbp_bytes, _vbp_items) = crate::write::vbp::write_vbp(&report, &model);
            for line in lines(&text(&vbp_bytes)) {
                assert!(
                    !is_a_comment_line(line),
                    "a comment reached the .vbp file of {}: {line:?}",
                    exe_path.display()
                );
            }

            for form in &model.forms {
                let (files, _items) = crate::write::frm::write_form(form, &report.defects, &data)
                    .unwrap_or_else(|err| {
                        panic!(
                            "writing form {} of {} must succeed: {err}",
                            form.name.as_str(),
                            exe_path.display()
                        )
                    });
                let frm_text = text(&files.frm);
                let frm_lines = lines(&frm_text);
                let first_attribute = frm_lines.iter().position(|line| {
                    line.starts_with("Attribute VB_Name")
                        || line.starts_with("Attribute VB_GlobalNameSpace")
                        || line.starts_with("Attribute VB_Creatable")
                        || line.starts_with("Attribute VB_PredeclaredId")
                        || line.starts_with("Attribute VB_Exposed")
                });
                let Some(first_attribute) = first_attribute else {
                    panic!(
                        "form {} of {} must carry its five Attribute lines",
                        form.name.as_str(),
                        exe_path.display()
                    );
                };
                for line in &frm_lines[..first_attribute] {
                    assert!(
                        !is_a_comment_line(line),
                        "a comment reached above the first Attribute line in form {} of {}: {line:?}",
                        form.name.as_str(),
                        exe_path.display()
                    );
                }
                let last_attribute = frm_lines
                    .iter()
                    .rposition(|line| line.starts_with("Attribute VB_Exposed"))
                    .unwrap_or(first_attribute);
                let below = frm_lines
                    .get(last_attribute.saturating_add(1)..)
                    .unwrap_or(&[]);
                let below_comments = below.iter().filter(|line| is_a_comment_line(line)).count();
                if below_comments > 0 {
                    comment_lines_below_boundary =
                        comment_lines_below_boundary.saturating_add(below_comments);
                    program_naming_one.get_or_insert_with(|| {
                        format!("{} (form {})", exe_path.display(), form.name.as_str())
                    });
                }
            }

            for code in &model.code {
                let prefix = crate::report::path_for_code(code.kind, &code.name);
                let procedures = object_procedures(&code.procedures);
                let bytes = match code.kind {
                    CodeKind::Class => {
                        crate::write::code::write_cls(&code.name, &procedures, &[], &prefix).0
                    }
                    CodeKind::Module => {
                        crate::write::code::write_bas(&code.name, &procedures, &[], &prefix).0
                    }
                };
                let code_text = text(&bytes);
                let code_lines = lines(&code_text);
                let header = code_lines
                    .iter()
                    .position(|line| line.starts_with("Attribute VB_Name"))
                    .unwrap_or(0);
                for line in &code_lines[..=header] {
                    assert!(
                        !is_a_comment_line(line),
                        "a comment reached at or above the header line of a code object in {}: {line:?}",
                        exe_path.display()
                    );
                }
            }
        }

        assert!(
            comment_lines_below_boundary > 0,
            "no corpus program produced even one comment line below the boundary of any \
             written form; the emitter may not be wired at all"
        );
        assert!(
            program_naming_one.is_some(),
            "a positive count with no named program is a contradiction in this test's own logic"
        );
    }
}
