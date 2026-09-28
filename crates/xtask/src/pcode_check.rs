//! `check-pcode-table`: decodes each P-code body of `corpus-pcode/` with a
//! table that `derive-pcode-table` wrote, and counts the bodies that decode
//! to their end.
//!
//! The table is not in the repository, so no test can hold it. This command
//! is the check that the user runs on the table that they derive. It decodes
//! with [`deform6::vb::pcode::disassemble`], whose doc comment gives the
//! rules, and it names each body that does not decode to its end. It exits
//! with 1 when one body does not. It also counts the bodies that
//! [`deform6::vb::lift::lift`] lifts to statements; that count only reports,
//! and it does not fail the command. With `--vb-types`, the lift also knows
//! the interfaces of the controls that `derive-vb-types` wrote.
//!
//! It also measures the lift against the source of each program, as
//! [`crate::lift_measure`] gives it. With `--lift-pins <file>` it compares
//! that measure with the file, and exits with 1 and the new numbers when
//! one program moves. `--write-lift-pins <file>` writes the file.
//!
//! With `--vb-types`, it also writes the project of each program as
//! `extract --vb-types` does, and holds each event handler that it writes
//! against the source: the declaration line of the handler must be a line
//! of the source. It exits with 1 when one is not.

use deform6::read::pe::PeImage;
use deform6::vb::context::callees_of_project;
use deform6::vb::header::{VbHeader, header_region};
use std::collections::BTreeMap;
use std::path::PathBuf;

use deform6::vb::lift::{lift_method, render};
use deform6::vb::object::ObjectTable;
use deform6::vb::pcode::{PcodeTable, disassemble};
use deform6::vb::procdesc::{MethodEntry, read_method_table};
use deform6::vb::project::{ObjectTableHead, ProjectInfo};
use deform6::vb::types::VbTypes;

use crate::build_record;
use crate::lift_measure::{Measure, parse_pins, render_pins};
use crate::pcode_record;
use crate::pcode_table::DEFAULT_OUTPUT_PATH;
use crate::ratios::differential::support::source::{procedure_bodies, without_comment};
use crate::ratios::differential::support::vbp::{Project, project_files, select_project_file};

/// The options of `check-pcode-table`.
#[derive(Default)]
struct Options {
    table: Option<String>,
    types: Option<String>,
    pins: Option<String>,
    write_pins: Option<String>,
}

/// Reads the arguments of `check-pcode-table`.
fn options(args: &[String]) -> Option<Options> {
    let mut out = Options::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--vb-types" => out.types = Some(rest.next()?.clone()),
            "--lift-pins" => out.pins = Some(rest.next()?.clone()),
            "--write-lift-pins" => out.write_pins = Some(rest.next()?.clone()),
            flag if flag.starts_with("--") => return None,
            path if out.table.is_none() => out.table = Some(path.to_owned()),
            _ => return None,
        }
    }
    Some(out)
}

/// Runs `check-pcode-table [<table>] [--vb-types <types>] [--lift-pins
/// <file>] [--write-lift-pins <file>]`.
pub(crate) fn run(args: &[String]) -> i32 {
    let Some(options) = options(args) else {
        eprintln!(
            "usage: cargo run -p xtask -- check-pcode-table [<table>] [--vb-types <types>] \
             [--lift-pins <file>] [--write-lift-pins <file>]"
        );
        return 1;
    };
    let path = options.table.as_deref().unwrap_or(DEFAULT_OUTPUT_PATH);
    match check(path, options.types.as_deref()) {
        Ok((bodies, failures, lifted, measures)) => {
            for failure in &failures {
                println!("{failure}");
            }
            let decoded = bodies.saturating_sub(failures.len());
            println!("{decoded} of {bodies} P-code bodies decode to their end");
            println!("{lifted} of {bodies} P-code bodies lift to statements");
            let mut total = Measure::default();
            for measure in measures.values() {
                total.merge(*measure);
            }
            println!(
                "{} of {} tokens of the source are in the lift, which gives {} tokens",
                total.matched, total.source, total.lifted
            );
            let mut status = i32::from(!failures.is_empty());
            if let Some(types) = &options.types {
                match check_handlers(types) {
                    Ok((written, strays)) => {
                        for stray in &strays {
                            println!("{stray}");
                        }
                        println!(
                            "{} of {written} event handlers that extract writes are lines of \
                             their source",
                            written.saturating_sub(strays.len())
                        );
                        if !strays.is_empty() {
                            status = 1;
                        }
                    }
                    Err(err) => {
                        eprintln!("check-pcode-table: {err}");
                        status = 1;
                    }
                }
            }
            if let Some(out) = &options.write_pins
                && let Err(err) = std::fs::write(out, render_pins(&measures))
            {
                eprintln!("check-pcode-table: writing {out}: {err}");
                status = 1;
            }
            if let Some(pins) = &options.pins {
                match std::fs::read_to_string(pins)
                    .map_err(|err| format!("reading {pins}: {err}"))
                    .and_then(|text| parse_pins(&text))
                {
                    Ok(pinned) => {
                        for (key, measure) in &measures {
                            if pinned.get(key) != Some(measure) {
                                println!(
                                    "{key}: the lift measure moved to matched = {}, source = {}, \
                                     lifted = {}; {pins} holds {:?}",
                                    measure.matched,
                                    measure.source,
                                    measure.lifted,
                                    pinned.get(key)
                                );
                                status = 1;
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("check-pcode-table: {err}");
                        status = 1;
                    }
                }
            }
            status
        }
        Err(err) => {
            eprintln!("check-pcode-table: {err}");
            1
        }
    }
}

/// Gives `line` as Basic compares a declaration: in lower case, with no
/// comment and no space at its ends. Basic does not tell the case of a
/// name apart.
fn declaration_key(line: &str) -> String {
    without_comment(line).trim().to_lowercase()
}

/// Writes the project of each program in `corpus-pcode/` with the types
/// file at `types_path`, and gives the number of event handlers that it
/// writes and a line for each one whose declaration is not a line of the
/// source of its program, as [`declaration_key`] compares them.
fn check_handlers(types_path: &str) -> Result<(usize, Vec<String>), String> {
    let bytes = std::fs::read(types_path).map_err(|err| format!("reading {types_path}: {err}"))?;
    let types = VbTypes::parse(&bytes).map_err(|err| format!("{types_path}: {err}"))?;
    let root = pcode_record::pcode_root();
    let corpus = build_record::corpus_root();
    let projects = project_files();
    let mut written = 0_usize;
    let mut strays = Vec::new();
    for exe in build_record::executables(&root)? {
        let key = build_record::program_key(&exe, &root)?;
        let project = select_project_file(&corpus.join(&key), &projects)
            .map_err(|err| format!("{key}: {err:?}"))?;
        let mut source = std::collections::BTreeSet::new();
        for object in Project::read(&project).declared_objects() {
            let text = std::fs::read(&object.source_file).unwrap_or_default();
            for line in String::from_utf8_lossy(&text).lines() {
                source.insert(declaration_key(line));
            }
        }
        let bytes = std::fs::read(&exe).map_err(|err| format!("reading {key}: {err}"))?;
        let report = deform6::inspect_with_types(
            &bytes,
            &deform6::vb::opcodes::OpcodeTable::builtin(),
            Some(&types),
            deform6::journal::Mode::Salvage,
        )
        .map_err(|err| format!("{key}: {err}"))?;
        let handlers: Vec<String> = report
            .objects
            .iter()
            .filter_map(|object| match &object.procedures {
                deform6::vb::ObjectProcedures::Slots(slots) => Some(slots),
                deform6::vb::ObjectProcedures::NoNameArray { .. } => None,
            })
            .flatten()
            .filter_map(|slot| match slot {
                deform6::vb::ProcedureEntry::Handler { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect();
        let project = deform6::write::project(&report, &bytes, deform6::journal::Mode::Salvage)
            .map_err(|err| format!("{key}: {err}"))?;
        for file in &project.files {
            for line in String::from_utf8_lossy(&file.bytes).lines() {
                let is_handler = handlers
                    .iter()
                    .any(|name| line.starts_with(&format!("Private Sub {name}(")));
                if !is_handler {
                    continue;
                }
                written = written.saturating_add(1);
                if !source.contains(&declaration_key(line)) {
                    strays.push(format!("{key}: {}: {line}", file.name));
                }
            }
        }
    }
    Ok((written, strays))
}

/// Decodes each body of each program in `corpus-pcode/`, and gives the
/// number of bodies and a line for each body that does not decode.
#[allow(
    clippy::type_complexity,
    reason = "the counts, the failures and the measure of each program"
)]
fn check(
    path: &str,
    types_path: Option<&str>,
) -> Result<(usize, Vec<String>, usize, BTreeMap<String, Measure>), String> {
    let bytes = std::fs::read(path).map_err(|err| format!("reading {path}: {err}"))?;
    let table = PcodeTable::parse(&bytes).map_err(|err| format!("{path}: {err}"))?;
    let types = match types_path {
        None => None,
        Some(types_path) => {
            let bytes =
                std::fs::read(types_path).map_err(|err| format!("reading {types_path}: {err}"))?;
            Some(VbTypes::parse(&bytes).map_err(|err| format!("{types_path}: {err}"))?)
        }
    };
    let root = pcode_record::pcode_root();
    let mut bodies = 0_usize;
    let mut lifted = 0_usize;
    let mut failures = Vec::new();
    let mut measures = BTreeMap::new();
    let corpus = build_record::corpus_root();
    let projects = project_files();
    for exe in build_record::executables(&root)? {
        let key = build_record::program_key(&exe, &root)?;
        let project = select_project_file(&corpus.join(&key), &projects)
            .map_err(|err| format!("{key}: {err:?}"))?;
        let sources: BTreeMap<String, PathBuf> = Project::read(&project)
            .declared_objects()
            .into_iter()
            .filter_map(|object| Some((object.name?, object.source_file)))
            .collect();
        let mut measure = Measure::default();
        let bytes = std::fs::read(&exe).map_err(|err| format!("reading {key}: {err}"))?;
        let pe = PeImage::parse(&bytes).map_err(|err| format!("{key}: {err}"))?;
        let header = VbHeader::read(&header_region(&pe).map_err(|err| format!("{key}: {err}"))?)
            .map_err(|err| format!("{key}: {err}"))?;
        let info = ProjectInfo::read(&pe, header.lp_project_data)
            .map_err(|err| format!("{key}: {err}"))?;
        let head = ObjectTableHead::read(&pe, info.lp_object_table)
            .map_err(|err| format!("{key}: {err}"))?;
        let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
            .map_err(|err| format!("{key}: {err}"))?;
        let all_callees = callees_of_project(&pe, &objects.objects, &table, types.as_ref());
        for (object, callees) in objects.objects.iter().zip(&all_callees) {
            let methods = read_method_table(&pe, object.lp_object_info)
                .map_err(|err| format!("{key}: {}: {err}", object.name))?;
            let source_bodies = sources
                .get(&object.name)
                .map(|source| procedure_bodies(source))
                .unwrap_or_default();
            let mut position = 0_usize;
            for entry in &methods.entries {
                let MethodEntry::Descriptor { index, descriptor } = entry else {
                    continue;
                };
                bodies = bodies.saturating_add(1);
                let body = descriptor
                    .body(&pe)
                    .ok_or_else(|| format!("{key}: a body cannot be read"))?;
                let listing = disassemble(&body, &table);
                let lines = match lift_method(&listing, &table, callees, types.as_ref(), *index) {
                    Ok(stmts) => {
                        lifted = lifted.saturating_add(1);
                        render(&stmts)
                    }
                    Err(_) => Vec::new(),
                };
                if let Some((_, source)) = source_bodies.get(position) {
                    measure.add(source, &lines);
                }
                position = position.saturating_add(1);
                if !listing.end.is_complete() {
                    failures.push(format!(
                        "{key}: {} descriptor {:#x}: {:?}",
                        object.name,
                        descriptor.va.get(),
                        listing.end
                    ));
                }
            }
        }
        measures.insert(key, measure);
    }
    Ok((bodies, failures, lifted, measures))
}

#[cfg(test)]
mod tests {
    use super::declaration_key;

    #[test]
    fn a_declaration_compares_with_no_case_and_no_comment() {
        assert_eq!(
            declaration_key("Private Sub Command16_Click()      ' Prev CD Track"),
            declaration_key("Private Sub command16_click()")
        );
        assert_eq!(
            declaration_key("Private Sub P_MouseUp(x As Single)"),
            "private sub p_mouseup(x as single)"
        );
        assert_ne!(
            declaration_key("Private Sub A_Click(Index As Integer)"),
            declaration_key("Private Sub A_Click()")
        );
    }
}
