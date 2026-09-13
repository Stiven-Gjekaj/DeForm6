//! The thin `.vbp` writer plan 04-01's tracer needs, and the complete
//! `.vbp` writer this plan adds beside it.
//!
//! `.planning/research/FILE-FORMATS.md` section 1: a `.vbp` is a flat list
//! of `Key=Value` lines, with no space around the `=` and no line indented.
//! This grammar shares no line-template helper with `write::frm` or
//! `write::code`, whose own grammars differ in indent, name pad and space
//! count.

use crate::report::{Confidence, Evidence, ReportItem};
use crate::vb::Report;
use crate::vb::classify::ObjectKind;
use crate::vb::project::Component;

use super::model::{CodeKind, CodeModel, FormModel, LineWriter, NameKind, ProjectModel, SafeName};

/// Writes the thin `.vbp` plan 04-01's tracer needs.
///
/// Line 1 is always `Type=Exe`. Every form becomes a `Form=` line, in
/// [`Report::forms`]'s own order; every module and every class becomes a
/// `Module=` or a `Class=` line, in [`Report::objects`]'s own order, each
/// carrying its own VB name and its own file name separated by `; `, per
/// section 1.3. `Startup=` names the first form in object table order, or
/// `"Sub Main"` when the project holds no form at all.
///
/// This function names each component through its own, independent
/// [`SafeName::new`] call, not through the one [`super::model::SafeNameIssuer`]
/// `write::project` threads across the whole run: a name that collides with
/// another component's name is [`write_vbp`]'s own job, once `write::project`
/// switches over to building a [`ProjectModel`] and calling it instead. No
/// corpus form in this task's own tracer collides, so this thin path names
/// each component correctly today.
#[must_use]
pub(crate) fn write_vbp_thin(report: &Report) -> Vec<u8> {
    let mut writer = LineWriter::new();
    writer.push_line("Type=Exe");

    for form in &report.forms {
        let (name, _faults) = SafeName::new(&form.name, NameKind::Form);
        writer.push_line(&format!("Form={}", name.file_name("frm")));
    }

    for object in &report.objects {
        match object.kind {
            ObjectKind::Module => {
                let (name, _faults) = SafeName::new(&object.name, NameKind::Module);
                writer.push_line(&format!(
                    "Module={}; {}",
                    name.as_str(),
                    name.file_name("bas")
                ));
            }
            ObjectKind::Class => {
                let (name, _faults) = SafeName::new(&object.name, NameKind::Class);
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
            let (name, _faults) = SafeName::new(&form.name, NameKind::Form);
            writer.push_line(&format!("Startup=\"{}\"", name.as_str()));
        }
        None => writer.push_line("Startup=\"Sub Main\""),
    }

    writer.finish().0
}

// ---------------------------------------------------------------------
// Plan 04-03, Task 1: the component lines, in recovered order.
// ---------------------------------------------------------------------

/// Writes the complete `.vbp`. Task 1 gives the project type key and the
/// component lines; a later task in this same plan adds the 34 key setting
/// block between them and the transaction server section, per section 1.7's
/// own rule for the file's overall shape.
///
/// `report` supplies the facts no [`ProjectModel`] carries: the recovered
/// object table order (interleaving forms, modules and classes exactly as
/// the executable declared them, per [`Report::objects`]) and the external
/// component table. `model` supplies every [`SafeName`] this function
/// writes: no line here builds a file name itself, and every one comes
/// from [`SafeName::as_str`] or [`SafeName::file_name`].
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
            Some((line, item)) => {
                writer.push_line(&line);
                written = written.saturating_add(1);
                items.push(item);
            }
            None => items.push(ReportItem {
                path: format!("/components/{}", component.name),
                confidence: Confidence::Unrecoverable,
                basis: "this component's own sixteen byte identifier did not resolve; no \
                        Object= line was written for it"
                    .to_owned(),
                evidence: Vec::new(),
            }),
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

/// Builds the `Object=` line one declared external component gives, and the
/// report item stating the identifier is not confirmed. `None` when the
/// component's own sixteen byte identifier did not resolve at all, per
/// [`Component::ouuid_text`]'s own doc comment.
///
/// The written identifier is the closest field this repository ever
/// resolves to a declared `Object=` identifier, never the confirmed one:
/// plan 03-16's own measurement found it differs from every corpus `.vbp`'s
/// own declared value by exactly one byte, on every sample it could check.
/// The version and the locale are IDE defaults; `Component` carries no
/// field for either, because neither survives compilation into this table.
///
/// The file name is written verbatim, never through a [`SafeName`]: it
/// names a file that already exists on the machine the project rebuilds
/// on, not a file this phase writes, so [`SafeName`]'s own rules (built for
/// a name this phase turns into a path it creates) do not apply to it.
fn object_line(component: &Component) -> Option<(String, ReportItem)> {
    let identifier = component.ouuid_text.as_ref()?;
    let line = format!("Object={{{identifier}}}#1.0#0; {}", component.file_name);
    let item = ReportItem {
        path: format!("/components/{}", component.name),
        confidence: Confidence::Unrecoverable,
        basis: "this component's own declared identifier is not confirmed against the \
                project file; the closest recovered field is written instead, and its own \
                version and locale are IDE defaults, never a recovery"
            .to_owned(),
        evidence: vec![Evidence {
            offset: component.ouuid_field_offset,
            structure: "ExternalComponentEntry",
            field: "oUuid",
            note: None,
        }],
    };
    Some((line, item))
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
    use super::write_vbp;
    use crate::read::region::Off;
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
}
