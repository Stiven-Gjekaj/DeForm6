//! The complete `.vbp` writer.
//!
//! `docs/FILE-FORMATS.md` section 1: a `.vbp` is a flat list
//! of `Key=Value` lines, with no space around the `=` and no line indented.
//! This grammar shares no line-template helper with `write::frm` or
//! `write::code`, whose own grammars differ in indent, name pad and space
//! count.

use crate::report::{Confidence, Evidence, ReportItem};
use crate::vb::Report;
use crate::vb::classify::ObjectKind;
use crate::vb::project::Component;

use super::model::{CodeKind, CodeModel, FormModel, LineWriter, ProjectModel, Startup};
use super::values::escape_inline_string;

// ---------------------------------------------------------------------
// Plan 04-03, Task 1: the component lines, in recovered order.
// ---------------------------------------------------------------------

/// The 34 setting keys `docs/FILE-FORMATS.md` section 1.4
/// gives, in the corpus order. Held as an ordered constant, not as a
/// sequence of statements, so a reader can compare this list against the
/// document line by line, and so a test can assert the whole order in one
/// line.
///
/// This list never names `ResFile32`: the resource script key names a
/// `.res` file this phase never writes, and a build task is documented to
/// fail when that key is present and the file is not.
pub const SETTING_ORDER: [&str; 34] = [
    "IconForm",
    "Startup",
    "HelpFile",
    "Title",
    "ExeName32",
    "Command32",
    "Name",
    "HelpContextID",
    "CompatibleMode",
    "MajorVer",
    "MinorVer",
    "RevisionVer",
    "AutoIncrementVer",
    "ServerSupportFiles",
    "VersionComments",
    "VersionCompanyName",
    "VersionProductName",
    "VersionLegalCopyright",
    "VersionFileDescription",
    "CompilationType",
    "OptimizationType",
    "FavorPentiumPro(tm)",
    "CodeViewDebugInfo",
    "NoAliasing",
    "BoundsCheck",
    "OverflowCheck",
    "FlPointCheck",
    "FDIVCheck",
    "UnroundedFP",
    "StartMode",
    "Unattended",
    "Retained",
    "ThreadPerObject",
    "MaxNumberOfThreads",
];

/// The ten compiler flags this phase can never recover from a native
/// executable, and the IDE default this writer gives each one.
///
/// `FavorPentiumPro(tm)` is the one flag whose IDE default is not zero:
/// Visual Basic 6 favors the Pentium Pro instruction path in a new,
/// unmodified project. Every other flag defaults to the safe, checks-on
/// state `corpus/public-domain/LockWorkStation/LockWorkStation.vbp` itself
/// carries, an unmodified project this repository can read directly rather
/// than a guess.
const DEFAULT_COMPILER_FLAGS: [(&str, i32); 10] = [
    ("CompilationType", 0),
    ("OptimizationType", 0),
    ("FavorPentiumPro(tm)", -1),
    ("CodeViewDebugInfo", 0),
    ("NoAliasing", 0),
    ("BoundsCheck", 0),
    ("OverflowCheck", 0),
    ("FlPointCheck", 0),
    ("FDIVCheck", 0),
    ("UnroundedFP", 0),
];

/// Writes the complete `.vbp`: the component lines in recovered order, the
/// 34 key setting block, and the transaction server section, plus every
/// report item a default value this function chose produced.
///
/// `report` supplies the facts no [`ProjectModel`] carries: the recovered
/// object table order (interleaving forms, modules and classes exactly as
/// the executable declared them, per [`Report::objects`]), the title, the
/// executable name and the help file `01-05-SUMMARY.md` settled, and the
/// external component table. `model` supplies every [`crate::write::model::SafeName`] this
/// function writes: no line here builds a file name itself, and every one
/// comes from [`crate::write::model::SafeName::as_str`] or [`crate::write::model::SafeName::file_name`].
///
/// # Determinism
///
/// Two calls with the same `report` and the same `model` give the same
/// bytes and the same report items: nothing here reads a hash keyed map,
/// and this function reaches no static or thread-local state.
#[must_use]
pub fn write_vbp(report: &Report, model: &ProjectModel) -> (Vec<u8>, Vec<ReportItem>) {
    let mut writer = LineWriter::new();
    let mut items = Vec::new();

    writer.push_line("Type=Exe");

    let component_lines = write_components(&mut writer, report, model, &mut items);
    if component_lines == 0 {
        items.push(ReportItem {
            path: crate::report::META_PATH.to_owned(),
            confidence: Confidence::Unrecoverable,
            basis: "the project holds no component line: no form, no module, no class and no \
                    declared external component"
                .to_owned(),
            evidence: Vec::new(),
        });
    }

    write_settings(&mut writer, report, model, &mut items);

    // A blank line always separates the setting block from the section
    // header: measured on every corpus `.vbp` that carries the section
    // (`Fast_Flames.exe`'s own `FlameTest.vbp`, `Map Editor.vbp`,
    // `LockWorkStation.vbp`), never omitted.
    writer.push_line("");
    writer.push_line("[MS Transaction Server]");
    writer.push_line("AutoRefresh=1");

    (writer.finish().0, items)
}

/// Writes every component line: `Form=`, `Module=` and `Class=` in
/// [`Report::objects`]'s own recovered order (never grouped by kind), then
/// every `Object=` line [`Report::components`] declares. Gives the number
/// of lines written.
fn write_components(
    writer: &mut LineWriter,
    report: &Report,
    model: &ProjectModel,
    items: &mut Vec<ReportItem>,
) -> u32 {
    let mut written = 0_u32;

    for object in &report.objects {
        match object.kind {
            ObjectKind::Form => {
                if let Some(form) = find_form(model, &object.name) {
                    writer.push_line(&format!("Form={}", form.name.file_name("frm")));
                    written = written.saturating_add(1);
                }
            }
            ObjectKind::Module | ObjectKind::Unknown(_) => {
                if let Some(code) = find_code(model, CodeKind::Module, &object.name) {
                    writer.push_line(&format!(
                        "Module={}; {}",
                        code.name.as_str(),
                        code.name.file_name("bas")
                    ));
                    written = written.saturating_add(1);
                }
            }
            ObjectKind::Class => {
                if let Some(code) = find_code(model, CodeKind::Class, &object.name) {
                    writer.push_line(&format!(
                        "Class={}; {}",
                        code.name.as_str(),
                        code.name.file_name("cls")
                    ));
                    written = written.saturating_add(1);
                }
            }
        }
    }

    for component in &report.components {
        match object_line(component) {
            Ok((line, item)) => {
                writer.push_line(&line);
                written = written.saturating_add(1);
                items.push(item);
            }
            Err(item) => items.push(item),
        }
    }

    written
}

/// Finds the [`FormModel`] whose own raw recovered name matches `raw_name`.
///
/// Matches on [`SafeName::raw`], never on [`SafeName::as_str`]: the object
/// table and the GUI table are two different structures, joined here by the
/// name the file itself gave both, before either one was sanitized.
fn find_form<'m>(model: &'m ProjectModel, raw_name: &str) -> Option<&'m FormModel> {
    model.forms.iter().find(|form| form.name.raw() == raw_name)
}

/// Finds the [`CodeModel`] of kind `kind` whose own raw recovered name
/// matches `raw_name`. See [`find_form`] for why the match is on the raw
/// name.
fn find_code<'m>(model: &'m ProjectModel, kind: CodeKind, raw_name: &str) -> Option<&'m CodeModel> {
    model
        .code
        .iter()
        .find(|code| code.kind == kind && code.name.raw() == raw_name)
}

/// A control library that a corpus project file declares, found by the
/// class identifier that the executable holds for the control.
struct CorpusLibrary {
    /// The class identifier at `oUuid`, in the text that
    /// [`Component::ouuid_text`] gives.
    class_id: &'static str,
    /// The type library identifier, the version and the locale, as the
    /// `Object=` line of the corpus project file gives them.
    declaration: &'static str,
}

/// The control libraries that this repository measured in its own corpus.
///
/// The executable does not hold the type library identifier that an
/// `Object=` line declares. `docs/STRUCTURES.md` section 7.3.1 records the
/// search. The executable holds the class identifier of the control, at the
/// bytes that `oUuid` names. Each row joins that class identifier to the
/// `Object=` line of the project file that the executable was built from.
/// This is the source of the corpus rows of the opcode table: a fact that
/// this repository measured in its own corpus, not a fact from another
/// tool.
///
/// Measured against:
/// - `MSWinsockLib.Winsock` in `MSWINSCK.OCX`: the three executables and
///   the three project files in `corpus/public-domain/SK-TFTP-Sample__VB6/Client`,
///   `corpus/public-domain/SK-TFTP-Sample__VB6/Server` and
///   `corpus/public-domain/SK-Winsock-Sample__VB6`. Each executable holds
///   the class identifier of this row, and each project file declares the
///   line of this row.
const CORPUS_LIBRARIES: [CorpusLibrary; 1] = [CorpusLibrary {
    class_id: "248DD896-BB45-11CF-9ABC-0080C7E7B78D",
    declaration: "{248DD890-BB45-11CF-9ABC-0080C7E7B78D}#1.0#0",
}];

/// Builds the `Object=` line of one declared external component, and the
/// report item that grades it. `Err` (naming the reason) when the
/// component's own sixteen byte identifier did not resolve at all, per
/// [`Component::ouuid_text`]'s own doc comment, or when the component's own
/// recovered file name holds a line break.
///
/// A class identifier that [`CORPUS_LIBRARIES`] holds gives the line that
/// the corpus project files declare, graded `inferred`. Any other class
/// identifier is written where the type library identifier goes, graded
/// `unrecoverable`: Visual Basic 6 does not load that line. The first build
/// run, which `docs/ROADMAP.md` records under phase 8, gave
/// `'MSWINSCK.OCX' could not be loaded` for each of the three corpus
/// programs while this function wrote the class identifier for all of
/// them. The version and the locale of that line are
/// IDE defaults; `Component` carries no field for either, because neither
/// survives compilation into this table.
///
/// The file name is written verbatim, never through a [`SafeName`]: it
/// names a file that already exists on the machine the project rebuilds
/// on, not a file this phase writes, so [`SafeName`]'s own rules (built for
/// a name this phase turns into a path it creates) do not apply to it. It
/// still gets the same T-4-05 line-break guard [`write_quoted_setting`]
/// gives every other raw field this file writes: a raw carriage return or
/// line feed in the file name would end the `Object=` line early and inject
/// an attacker-chosen extra line into the `.vbp`.
fn object_line(component: &Component) -> Result<(String, ReportItem), ReportItem> {
    let Some(identifier) = component.ouuid_text.as_ref() else {
        return Err(ReportItem {
            path: format!("/components/{}", component.name),
            confidence: Confidence::Unrecoverable,
            basis: "this component's own sixteen byte identifier did not resolve; no \
                    Object= line was written for it"
                .to_owned(),
            evidence: Vec::new(),
        });
    };
    if component.file_name.contains('\r') || component.file_name.contains('\n') {
        return Err(ReportItem {
            path: format!("/components/{}", component.name),
            confidence: Confidence::Unrecoverable,
            basis: "this component's own recovered file name holds a line break; writing it \
                    inline would end the Object= line early, so no Object= line was written \
                    for it"
                .to_owned(),
            evidence: Vec::new(),
        });
    }
    // The offset is where the sixteen bytes of the identifier start, which
    // the `oUuid` field names. A reader checks the identifier there.
    let evidence = vec![Evidence {
        offset: component.ouuid_field_offset,
        structure: "ExternalComponentEntry",
        field: "oUuid",
        note: Some("the sixteen bytes that the oUuid field names".to_owned()),
    }];
    let library = CORPUS_LIBRARIES
        .iter()
        .find(|library| library.class_id == identifier.as_str());
    let (line, confidence, basis) = match library {
        Some(library) => (
            format!("Object={}; {}", library.declaration, component.file_name),
            Confidence::Inferred,
            "the executable does not hold the type library identifier, the version or the \
             locale of this Object= line; the line is the one that the corpus project files \
             declare for the class identifier that the oUuid field names",
        ),
        None => (
            format!("Object={{{identifier}}}#1.0#0; {}", component.file_name),
            Confidence::Unrecoverable,
            "the executable does not hold the type library identifier of this Object= line; \
             the class identifier that the oUuid field names is written in its place, and \
             Visual Basic 6 does not load the line until the type library identifier of the \
             control replaces it; the version and the locale are IDE defaults, never a \
             recovery",
        ),
    };
    let item = ReportItem {
        path: format!("/components/{}", component.name),
        confidence,
        basis: basis.to_owned(),
        evidence,
    };
    Ok((line, item))
}

// ---------------------------------------------------------------------
// Plan 04-03, Task 2: the setting block, the startup key, and every
// default named as a default.
// ---------------------------------------------------------------------

/// Writes a quoted setting line, refusing a value that holds a line break
/// rather than writing one.
///
/// T-4-05: a raw carriage return or line feed inside a quoted value would
/// end the line early, and the bytes after it would parse as the start of
/// a new key. A value that holds one is refused: an empty value is written
/// instead, and the refusal is recorded as a report item, never silently
/// dropped.
fn write_quoted_setting(
    writer: &mut LineWriter,
    items: &mut Vec<ReportItem>,
    key: &str,
    value: &str,
) {
    if value.contains('\r') || value.contains('\n') {
        writer.push_line(&format!("{key}=\"\""));
        items.push(ReportItem {
            path: crate::report::META_PATH.to_owned(),
            confidence: Confidence::Unrecoverable,
            basis: format!(
                "the recovered {key} value holds a line break; writing it inline would end \
                 the line early, so an empty value was written instead"
            ),
            evidence: Vec::new(),
        });
    } else {
        writer.push_line(&format!("{key}={}", escape_inline_string(value)));
    }
}

/// Writes the 34 key setting block, in [`SETTING_ORDER`]'s own order.
///
/// `IconForm` and `Startup` both take the model's own [`Startup`] decision:
/// the executable does not declare which form's icon becomes the EXE icon
/// any more than it declares a startup form, and [`super::model::from_report`]
/// already recorded the one inferred choice this repository makes for
/// both. `IconForm` is omitted, along with `Startup`'s own form name, when
/// the model holds no form at all: there is no form left to name.
///
/// `Title`, `ExeName32` and `HelpFile` take the values Phase 1 resolved at
/// the disputed header offsets `0x58` (`oProjectExeName`) and `0x5C`
/// (`oProjectTitle`), settled in `01-05-SUMMARY.md`, and the help file
/// field beside them: these are proven, not chosen, and this function
/// writes them verbatim rather than a default.
///
/// The five version string keys (`VersionComments` and the four optional
/// ones) are never written: this repository recovers none of the five from
/// a native executable, and the corpus itself omits an empty version
/// string rather than writing an empty pair of quotes.
fn write_settings(
    writer: &mut LineWriter,
    report: &Report,
    model: &ProjectModel,
    items: &mut Vec<ReportItem>,
) {
    match &model.startup {
        Startup::Form(name) => {
            writer.push_line(&format!("IconForm=\"{}\"", name.as_str()));
            writer.push_line(&format!("Startup=\"{}\"", name.as_str()));
        }
        Startup::SubMain => {
            writer.push_line("Startup=\"Sub Main\"");
        }
    }

    write_quoted_setting(writer, items, "HelpFile", &report.help_file);
    write_quoted_setting(writer, items, "Title", &report.title);
    write_quoted_setting(
        writer,
        items,
        "ExeName32",
        &format!("{}.exe", report.exe_name),
    );
    writer.push_line("Command32=\"\"");
    write_quoted_setting(writer, items, "Name", model.name.as_str());
    writer.push_line("HelpContextID=\"0\"");
    writer.push_line("CompatibleMode=\"0\"");
    writer.push_line("MajorVer=1");
    writer.push_line("MinorVer=0");
    writer.push_line("RevisionVer=0");
    writer.push_line("AutoIncrementVer=0");
    writer.push_line("ServerSupportFiles=0");

    for (key, value) in DEFAULT_COMPILER_FLAGS {
        writer.push_line(&format!("{key}={value}"));
        items.push(ReportItem {
            path: crate::report::META_PATH.to_owned(),
            confidence: Confidence::Inferred,
            basis: format!(
                "a native executable does not carry the {key} compiler flag; the IDE default \
                 is written"
            ),
            evidence: Vec::new(),
        });
    }

    writer.push_line("StartMode=0");
    writer.push_line("Unattended=0");
    writer.push_line("Retained=0");
    writer.push_line("ThreadPerObject=0");
    writer.push_line("MaxNumberOfThreads=1");
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
    use super::{SETTING_ORDER, object_line, write_vbp};
    use crate::read::region::Off;
    use crate::report::{Confidence, Evidence};
    use crate::vb::classify::ObjectKind;
    use crate::vb::controltree::ControlKind;
    use crate::vb::project::Component;
    use crate::vb::runtime::Runtime;
    use crate::vb::{ControlReport, FormReport, ObjectProcedures, ObjectReport, Report};
    use crate::write::model::from_report;

    /// Builds a minimal, valid [`Report`] over the given objects, forms and
    /// components. Every other field carries a literal this module's own
    /// tests never read, per `AGENTS.md`'s "build the state a test needs
    /// inside the test".
    fn minimal_report(
        objects: Vec<ObjectReport>,
        forms: Vec<FormReport>,
        components: Vec<Component>,
    ) -> Report {
        Report {
            file_len: 0,
            section_count: 0,
            runtime: Runtime::Vb6,
            runtime_dll: "MSVBVM60.DLL".to_owned(),
            signature: *b"VB5!",
            header_offset: Off::new(0),
            runtime_build: 0,
            project_name: "TestProject".to_owned(),
            title: "Test Title".to_owned(),
            exe_name: "TestExe".to_owned(),
            help_file: String::new(),
            native: true,
            object_count: u16::try_from(objects.len()).unwrap_or(0),
            objects,
            declarations: Vec::new(),
            components,
            forms,
            defects: Vec::new(),
        }
    }

    /// Builds a minimal [`ControlReport`] naming `name` and `kind`, with
    /// every optional field empty and no properties.
    fn minimal_control(name: &str, kind: ControlKind, parent: Option<usize>) -> ControlReport {
        ControlReport {
            name: name.to_owned(),
            kind,
            array_index: None,
            parent,
            properties: Vec::new(),
            external: None,
            external_reason: None,
            ocx_header: None,
            opaque_message: None,
            events: Vec::new(),
        }
    }

    /// Builds a minimal [`ObjectReport`] naming `name` and `kind`, with no
    /// procedures and no gaps.
    fn minimal_object(name: &str, kind: ObjectKind) -> ObjectReport {
        ObjectReport {
            name: name.to_owned(),
            kind,
            procedures: ObjectProcedures::Slots(Vec::new()),
            gaps: Vec::new(),
        }
    }

    /// Builds a minimal [`FormReport`] naming `name`, with one root control
    /// of kind [`ControlKind::Form`] carrying the same name, and no defect.
    fn minimal_form(name: &str) -> FormReport {
        FormReport {
            name: name.to_owned(),
            controls: vec![minimal_control(name, ControlKind::Form, None)],
            defects: Vec::new(),
        }
    }

    /// Builds a [`Component`] whose own `ouuid_text` resolved, so
    /// [`super::object_line`] writes an `Object=` line for it.
    fn resolved_component(name: &str, file_name: &str) -> Component {
        Component {
            file_name: file_name.to_owned(),
            library: format!("{name}Lib.{name}"),
            name: name.to_owned(),
            guid_offset: Off::new(0),
            guid_length: -1,
            guid_text: None,
            o_uuid: Off::new(0x04),
            ouuid_field_offset: 0x100,
            ouuid_text: Some("248DD896-BB45-11CF-9ABC-0080C7E7B78D".to_owned()),
        }
    }

    fn lines(bytes: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn the_first_line_is_the_project_type_key_with_no_space_around_the_equals() {
        let report = minimal_report(Vec::new(), Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        assert_eq!(lines(&bytes).first(), Some(&"Type=Exe".to_owned()));
    }

    #[test]
    fn form_class_module_form_interleave_in_recovered_object_table_order() {
        let objects = vec![
            minimal_object("frmA", ObjectKind::Form),
            minimal_object("ClsB", ObjectKind::Class),
            minimal_object("ModC", ObjectKind::Module),
            minimal_object("frmD", ObjectKind::Form),
        ];
        let forms = vec![minimal_form("frmA"), minimal_form("frmD")];
        let report = minimal_report(objects, forms, Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);

        let component_lines: Vec<String> = lines(&bytes)
            .into_iter()
            .filter(|line| {
                line.starts_with("Form=")
                    || line.starts_with("Class=")
                    || line.starts_with("Module=")
            })
            .collect();
        assert_eq!(
            component_lines,
            vec![
                "Form=frmA.frm".to_owned(),
                "Class=ClsB; ClsB.cls".to_owned(),
                "Module=ModC; ModC.bas".to_owned(),
                "Form=frmD.frm".to_owned(),
            ]
        );
    }

    #[test]
    fn a_module_line_reads_vb_name_semicolon_space_file_name_even_when_they_differ() {
        let objects = vec![minimal_object("Declaration_Module", ObjectKind::Module)];
        let report = minimal_report(objects, Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        assert!(
            lines(&bytes).contains(&"Module=Declaration_Module; Declaration_Module.bas".to_owned())
        );
    }

    #[test]
    fn a_class_line_reads_vb_name_semicolon_space_file_name() {
        let objects = vec![minimal_object("MyClass", ObjectKind::Class)];
        let report = minimal_report(objects, Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        assert!(lines(&bytes).contains(&"Class=MyClass; MyClass.cls".to_owned()));
    }

    #[test]
    fn a_form_line_carries_only_the_file_name() {
        let objects = vec![minimal_object("frmOnly", ObjectKind::Form)];
        let forms = vec![minimal_form("frmOnly")];
        let report = minimal_report(objects, forms, Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        let form_line = lines(&bytes)
            .into_iter()
            .find(|line| line.starts_with("Form="))
            .expect("a Form= line must be written");
        assert_eq!(form_line, "Form=frmOnly.frm");
    }

    #[test]
    fn a_declared_component_line_holds_no_double_quote_character() {
        let components = vec![resolved_component("Winsock", "MSWINSCK.OCX")];
        let report = minimal_report(Vec::new(), Vec::new(), components);
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        let object_line = lines(&bytes)
            .into_iter()
            .find(|line| line.starts_with("Object="))
            .expect("an Object= line must be written");
        assert!(!object_line.contains('"'), "{object_line}");
        assert!(object_line.contains("MSWINSCK.OCX"), "{object_line}");
    }

    /// The evidence of an `Object=` line gives the offset of the sixteen
    /// bytes of the identifier. Its note says that these are the bytes that
    /// `oUuid` names, and not the field itself.
    #[test]
    fn the_evidence_of_a_declared_component_gives_the_bytes_that_the_o_uuid_field_names() {
        let (_line, item) = object_line(&resolved_component("Winsock", "MSWINSCK.OCX")).unwrap();
        assert_eq!(
            item.evidence,
            [Evidence {
                offset: 0x100,
                structure: "ExternalComponentEntry",
                field: "oUuid",
                note: Some("the sixteen bytes that the oUuid field names".to_owned()),
            }]
        );
    }

    /// The class identifier of the Winsock control gives the line that the
    /// corpus project files declare, and the item says that the line is
    /// inferred.
    #[test]
    fn a_class_identifier_in_the_corpus_table_gives_the_declared_type_library() {
        let (line, item) = object_line(&resolved_component("Winsock", "MSWINSCK.OCX")).unwrap();
        assert_eq!(
            line,
            "Object={248DD890-BB45-11CF-9ABC-0080C7E7B78D}#1.0#0; MSWINSCK.OCX"
        );
        assert_eq!(item.confidence, Confidence::Inferred);
    }

    /// A class identifier that the table does not hold is written where the
    /// type library identifier goes, and the item says that the line is
    /// unrecoverable.
    #[test]
    fn a_class_identifier_not_in_the_corpus_table_is_written_and_graded_unrecoverable() {
        let mut component = resolved_component("Grid", "GRID32.OCX");
        component.ouuid_text = Some("00000000-0000-0000-0000-0000000000AB".to_owned());
        let (line, item) = object_line(&component).unwrap();
        assert_eq!(
            line,
            "Object={00000000-0000-0000-0000-0000000000AB}#1.0#0; GRID32.OCX"
        );
        assert_eq!(item.confidence, Confidence::Unrecoverable);
    }

    #[test]
    fn a_component_with_no_resolved_identifier_writes_no_object_line_and_an_item() {
        let mut component = resolved_component("Winsock", "MSWINSCK.OCX");
        component.ouuid_text = None;
        let report = minimal_report(Vec::new(), Vec::new(), vec![component]);
        let (model, _items) = from_report(&report, &[]);
        let (bytes, items) = write_vbp(&report, &model);
        assert!(!lines(&bytes).iter().any(|line| line.starts_with("Object=")));
        assert!(items.iter().any(|item| item.path.contains("Winsock")));
    }

    #[test]
    fn a_component_whose_file_name_holds_a_line_break_writes_no_object_line_and_an_item() {
        // CR-03: a raw line break in a component's own recovered file name
        // must not reach the .vbp: it would end the Object= line early and
        // inject an attacker-chosen extra line, the same T-4-05 threat
        // write_quoted_setting already guards every other raw field
        // against.
        let component = resolved_component("Winsock", "MSWINSCK.OCX\r\nEvilKey=Injected");
        let report = minimal_report(Vec::new(), Vec::new(), vec![component]);
        let (model, _items) = from_report(&report, &[]);
        let (bytes, items) = write_vbp(&report, &model);
        assert!(!lines(&bytes).iter().any(|line| line.starts_with("Object=")));
        assert!(
            !lines(&bytes).iter().any(|line| line.contains("EvilKey")),
            "the crafted file name must not inject its own extra line: {:?}",
            lines(&bytes)
        );
        assert!(items.iter().any(|item| item.path.contains("Winsock")));
    }

    #[test]
    fn a_project_with_zero_objects_still_writes_the_type_key_and_a_report_item() {
        let report = minimal_report(Vec::new(), Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, items) = write_vbp(&report, &model);
        assert_eq!(lines(&bytes).first(), Some(&"Type=Exe".to_owned()));
        assert!(
            items
                .iter()
                .any(|item| item.basis.contains("no component line")),
            "{items:?}"
        );
    }

    // --- Task 2: the setting block, the startup key, and every default -----

    #[test]
    fn setting_order_names_the_thirty_four_keys_in_section_1_4s_own_order() {
        assert_eq!(SETTING_ORDER.len(), 34);
        assert_eq!(SETTING_ORDER[0], "IconForm");
        assert_eq!(SETTING_ORDER[33], "MaxNumberOfThreads");
        assert_eq!(SETTING_ORDER[19], "CompilationType");
    }

    #[test]
    fn a_quoted_keys_value_is_wrapped_in_double_quotes_and_a_bare_keys_value_is_not() {
        let objects = vec![minimal_object("frmMain", ObjectKind::Form)];
        let forms = vec![minimal_form("frmMain")];
        let report = minimal_report(objects, forms, Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("HelpFile=\"\""));
        assert!(text.contains("CompilationType=0"));
        assert!(!text.contains("CompilationType=\"0\""));
    }

    #[test]
    fn the_startup_keys_value_is_a_name_one_of_the_written_form_lines_brings_in() {
        let objects = vec![minimal_object("frmMain", ObjectKind::Form)];
        let forms = vec![minimal_form("frmMain")];
        let report = minimal_report(objects, forms, Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        assert!(lines(&bytes).contains(&"Startup=\"frmMain\"".to_owned()));
        assert!(lines(&bytes).contains(&"Form=frmMain.frm".to_owned()));
    }

    #[test]
    fn a_model_with_zero_forms_writes_the_main_procedure_literal_for_startup() {
        let report = minimal_report(Vec::new(), Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        assert!(lines(&bytes).contains(&"Startup=\"Sub Main\"".to_owned()));
        assert!(
            !lines(&bytes)
                .iter()
                .any(|line| line.starts_with("IconForm="))
        );
    }

    #[test]
    fn every_compiler_flag_produces_one_report_item_of_confidence_inferred() {
        let report = minimal_report(Vec::new(), Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (_bytes, items) = write_vbp(&report, &model);
        let inferred_flag_items = items
            .iter()
            .filter(|item| item.basis.contains("compiler flag"))
            .count();
        assert_eq!(inferred_flag_items, 10, "{items:?}");
    }

    #[test]
    fn the_last_two_lines_are_the_section_header_and_its_one_key() {
        let report = minimal_report(Vec::new(), Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        let all = lines(&bytes);
        let len = all.len();
        assert_eq!(
            all.get(len.saturating_sub(2)),
            Some(&"[MS Transaction Server]".to_owned())
        );
        assert_eq!(
            all.get(len.saturating_sub(1)),
            Some(&"AutoRefresh=1".to_owned())
        );
    }

    #[test]
    fn a_blank_line_separates_the_setting_block_from_the_section_header() {
        let report = minimal_report(Vec::new(), Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        let all = lines(&bytes);
        let len = all.len();
        assert_eq!(all.get(len.saturating_sub(3)), Some(&String::new()));
    }

    #[test]
    fn title_exe_name_and_help_file_come_from_the_report_verbatim() {
        let mut report = minimal_report(Vec::new(), Vec::new(), Vec::new());
        report.title = "FlameTest".to_owned();
        report.exe_name = "Fast_Flames".to_owned();
        report.help_file = "help.hlp".to_owned();
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        assert!(lines(&bytes).contains(&"Title=\"FlameTest\"".to_owned()));
        assert!(lines(&bytes).contains(&"ExeName32=\"Fast_Flames.exe\"".to_owned()));
        assert!(lines(&bytes).contains(&"HelpFile=\"help.hlp\"".to_owned()));
    }

    // --- Task 3: the three edge shapes a project file has to survive -------

    #[test]
    fn a_model_with_zero_forms_gives_the_whole_loadable_shape() {
        let objects = vec![minimal_object("Mod1", ObjectKind::Module)];
        let report = minimal_report(objects, Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        let all = lines(&bytes);
        assert_eq!(all.first(), Some(&"Type=Exe".to_owned()));
        assert!(all.iter().any(|line| line.starts_with("Module=")));
        assert!(!all.iter().any(|line| line.starts_with("Form=")));
        assert!(all.contains(&"Startup=\"Sub Main\"".to_owned()));
    }

    #[test]
    fn a_model_with_exactly_one_form_and_nothing_else_gives_startup_naming_it() {
        let objects = vec![minimal_object("frmSolo", ObjectKind::Form)];
        let forms = vec![minimal_form("frmSolo")];
        let report = minimal_report(objects, forms, Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        assert!(lines(&bytes).contains(&"Startup=\"frmSolo\"".to_owned()));
        let form_lines: Vec<String> = lines(&bytes)
            .into_iter()
            .filter(|line| {
                line.starts_with("Form=")
                    || line.starts_with("Module=")
                    || line.starts_with("Class=")
            })
            .collect();
        assert_eq!(form_lines, vec!["Form=frmSolo.frm".to_owned()]);
    }

    #[test]
    fn two_objects_whose_raw_names_collide_name_two_different_files() {
        let objects = vec![
            minimal_object(&"A".repeat(40), ObjectKind::Module),
            minimal_object(&"A".repeat(45), ObjectKind::Module),
        ];
        let report = minimal_report(objects, Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        let module_lines: Vec<String> = lines(&bytes)
            .into_iter()
            .filter(|line| line.starts_with("Module="))
            .collect();
        assert_eq!(module_lines.len(), 2, "{module_lines:?}");
        assert_ne!(module_lines[0], module_lines[1]);
    }

    #[test]
    fn a_model_whose_only_object_is_of_unknown_kind_still_gives_one_component_line() {
        let objects = vec![minimal_object("Weird1", ObjectKind::Unknown(0xDEAD_BEEF))];
        let report = minimal_report(objects, Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        let (bytes, _items) = write_vbp(&report, &model);
        assert!(lines(&bytes).contains(&"Module=Weird1; Weird1.bas".to_owned()));
    }

    #[test]
    fn two_calls_to_the_writer_on_one_model_give_byte_identical_output() {
        let objects = vec![
            minimal_object("frmMain", ObjectKind::Form),
            minimal_object("ClsA", ObjectKind::Class),
        ];
        let forms = vec![minimal_form("frmMain")];
        let components = vec![resolved_component("Winsock", "MSWINSCK.OCX")];
        let report = minimal_report(objects, forms, components);
        let (model, _items) = from_report(&report, &[]);
        let (first, first_items) = write_vbp(&report, &model);
        let (second, second_items) = write_vbp(&report, &model);
        assert_eq!(first, second);
        assert_eq!(first_items, second_items);
    }
}
