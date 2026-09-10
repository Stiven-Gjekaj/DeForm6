//! The `deform6` command line: one subcommand, six exit codes.
//!
//! The exit code names the reason, per `CONTEXT.md`. A loop over a directory
//! of files nobody remembers the origin of can sort them by number without
//! reading English, so the table is a published contract and the numbering
//! must never move.
//!
//! | Code | Meaning |
//! |---|---|
//! | 0 | The file was read |
//! | 1 | Not a PE file |
//! | 2 | A PE file, but it holds no Visual Basic runtime |
//! | 3 | Visual Basic, but not version 6 |
//! | 4 | Visual Basic 6, but damaged |
//! | 5 | An internal error, including a usage error |
//!
//! The infallible clap entry point calls the process exit function itself,
//! with status 2 for a usage error. Code 2 is locked to "a PE file, but it
//! holds no Visual Basic runtime", so a user who mistypes a flag must not
//! collide with that meaning. This crate parses through the fallible entry
//! point and maps every error itself. Neither the infallible parse function
//! nor the process exit function is called anywhere in this crate.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser as _;

use deform6::Report;
use deform6::vb::classify::ObjectKind;
use deform6::vb::functyp::{Argument, DefaultValue, Prototype, TypeEntry, VbType};
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::privateobj::Gap;
use deform6::vb::project::{Declaration, ExportName};
use deform6::vb::{ObjectProcedures, ProcedureEntry};

/// `deform6`: reads a compiled Visual Basic 6 executable and reports what it
/// holds.
#[derive(clap::Parser)]
#[command(name = "deform6", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Reads one executable and prints what DeForm6 found in it.
    Inspect {
        /// The executable to read.
        ///
        /// Taken as a path, not as a string, so a path that is not valid
        /// UTF-8 is not mangled and not rejected.
        input: PathBuf,

        /// A property opcode table a user built with
        /// `xtask derive-opcode-table`, on their own machine, from their
        /// own lawful Visual Basic 6 install.
        ///
        /// Absent this flag, DeForm6 uses the small safe-provenance subset
        /// built into the binary. Taken as a path, not a string, for the
        /// same reason `input` is.
        #[arg(long)]
        opcode_table: Option<PathBuf>,
    },
}

/// The exit code a run of this program gives back to its caller.
///
/// Per `CONTEXT.md`, all six are defined now, even though `Damaged` has no
/// `--salvage` route in this phase. `Damaged` is in fact already reachable: a
/// truncated Visual Basic 6 file produces it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Exit {
    /// The file was read.
    Ok = 0,
    /// The bytes are not a portable executable, or not one this tool reads.
    NotPe = 1,
    /// A portable executable that holds no Visual Basic runtime.
    NoVbRuntime = 2,
    /// A Visual Basic runtime that is not version 6.
    NotVb6 = 3,
    /// Visual Basic 6, but a structure inside it did not resolve.
    Damaged = 4,
    /// An internal error: the file could not be read, or the arguments were
    /// not usable.
    Internal = 5,
}

impl From<Exit> for ExitCode {
    /// The cast passes the lint wall. `clippy::cast_possible_truncation` does
    /// not fire on an enum-to-integer cast on a `#[repr(u8)]` fieldless enum.
    fn from(exit: Exit) -> Self {
        Self::from(exit as u8)
    }
}

fn main() -> ExitCode {
    match Cli::try_parse() {
        Ok(cli) => run(&cli).into(),
        Err(err) => {
            // `print` sends help and version to stdout and a usage error to
            // stderr, exactly as the infallible entry point would have.
            let _ = err.print();
            match err.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    Exit::Ok
                }
                _ => Exit::Internal,
            }
            .into()
        }
    }
}

fn run(cli: &Cli) -> Exit {
    match &cli.command {
        Command::Inspect {
            input,
            opcode_table,
        } => run_inspect(input, opcode_table.as_deref()),
    }
}

/// Loads the opcode table `inspect` uses for this run, and the one line the
/// report names it by.
///
/// Absent `opcode_table_path`, this is [`OpcodeTable::builtin`] and the
/// builtin line. Present, this reads the file (the byte-returning read, not
/// the string-returning one, because a table file may hold a byte that is
/// not valid UTF-8) and calls [`OpcodeTable::parse`] on the bytes: the
/// library opens no file, this crate does, matching the split `lib.rs`
/// documents for `inspect` itself.
///
/// A failure to read the file, or a [`deform6::vb::opcodes::TableError`]
/// from `parse`, is [`Exit::Internal`], code 5: per plan 01-08, a usage
/// error the tool cannot act on is 5, and this is an argument the tool
/// cannot use, not a defect in the executable under test, so it is never
/// [`Exit::Damaged`].
fn load_opcode_table(opcode_table_path: Option<&Path>) -> Result<(OpcodeTable, String), Exit> {
    let Some(path) = opcode_table_path else {
        let table = OpcodeTable::builtin();
        let count = table.len();
        return Ok((
            table,
            format!("Opcode table  builtin subset, {count} entries"),
        ));
    };

    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("could not read {}: {err}", path.display());
            return Err(Exit::Internal);
        }
    };

    match OpcodeTable::parse(&bytes) {
        Ok(table) => {
            let count = table.len();
            Ok((
                table,
                format!("Opcode table  {}, {count} entries", path.display()),
            ))
        }
        Err(err) => {
            eprintln!("{err}");
            Err(Exit::Internal)
        }
    }
}

/// Reads `path`, runs [`deform6::inspect`] over the bytes, and prints one
/// outcome.
///
/// An input-output error is [`Exit::Internal`], code 5, and not
/// [`Exit::NotPe`]: the file was never read, so nothing about its format is
/// known. The opcode table is loaded before the executable is read, so a
/// bad `--opcode-table` argument is reported as the usage error it is,
/// before this run does any work over the file under inspection.
fn run_inspect(path: &Path, opcode_table_path: Option<&Path>) -> Exit {
    let (_table, table_summary) = match load_opcode_table(opcode_table_path) {
        Ok(loaded) => loaded,
        Err(exit) => return exit,
    };

    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("could not read {}: {err}", path.display());
            return Exit::Internal;
        }
    };

    match deform6::inspect(&data) {
        Ok(report) => {
            print_report(path, &report, &table_summary);
            Exit::Ok
        }
        Err(refusal) => {
            eprintln!("{refusal}");
            exit_for(refusal)
        }
    }
}

/// Maps a refusal to the exit code the locked table gives it.
///
/// The match has no wildcard arm, so a variant added later is a compile
/// error until its code is decided.
fn exit_for(refusal: deform6::Refusal) -> Exit {
    match refusal {
        deform6::Refusal::NotPe | deform6::Refusal::NotI386 | deform6::Refusal::NotPe32 => {
            Exit::NotPe
        }
        deform6::Refusal::NoVbRuntime { .. } => Exit::NoVbRuntime,
        deform6::Refusal::IsVb5 | deform6::Refusal::IsVb4 => Exit::NotVb6,
        deform6::Refusal::Damaged(_) => Exit::Damaged,
    }
}

/// Prints the locked eight-line shape, then the gaps, the object graph and
/// the external declarations below it, in that order.
///
/// The Runtime and Header lines print `report.runtime_dll` and
/// `report.signature`, which are values [`deform6::inspect`] read out of the
/// file. Nothing here writes what a Visual Basic 6 file is supposed to hold;
/// it prints what this one does.
///
/// The eight-line head phase 1 locked stays exactly as it was: this phase
/// appends sections below it rather than reshaping it. `crates/deform6-cli/
/// tests/cli.rs` compares the head line for line.
///
/// # Gaps come first, per D-10
///
/// The gaps section, including the standard-module cap, prints immediately
/// after the head and before the object graph. A reader meets the cap
/// before they meet the per-object procedure counts the cap explains,
/// rather than discovering the cap only after wondering why a module's
/// procedures carry no names.
///
/// # The opcode table line
///
/// `table_summary` names which opcode table produced this run's property
/// names, and how many entries it holds. It prints last, so a reader who
/// wonders where a property name came from finds the answer at the end of
/// the report, without reading the command line back.
fn print_report(path: &Path, report: &Report, table_summary: &str) {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let signature = String::from_utf8_lossy(&report.signature).into_owned();
    let mode = if report.native { "native" } else { "p-code" };

    print_line("File", &format!("{name}  ({} bytes)", report.file_len));
    print_line(
        "Format",
        &format!("PE32, {} sections", report.section_count),
    );
    print_line(
        "Runtime",
        &format!("{}  (Visual Basic 6)", report.runtime_dll),
    );
    print_line(
        "Header",
        &format!(
            "{signature} at {:#010x}  build {:#x}",
            report.header_offset.get(),
            report.runtime_build
        ),
    );
    print_line("Project", &report.project_name);
    print_line("Title", &report.title);
    print_line("Mode", mode);
    print_line("Objects", &report.object_count.to_string());

    println!();
    print_gaps(report);
    println!();
    print_objects(report);
    println!();
    print_declarations(report);
    println!();
    println!("{table_summary}");
}

/// Prints one line of the report, with the label padded to ten columns.
fn print_line(label: &str, value: &str) {
    println!("{label:<10}{value}");
}

/// Prints one line per object, giving its name and its kind, then one line
/// per procedure slot beneath it.
///
/// Per D-08 an unknown kind prints the word `unknown` and the raw value in
/// hexadecimal, and the run still exits 0: refusing a file over a type value
/// nobody has documented is the failure this phase exists to avoid.
fn print_objects(report: &Report) {
    println!("Object graph");
    for object in &report.objects {
        println!("  {}  ({})", object.name, format_kind(&object.kind));
        print_procedures(&object.procedures);
    }
}

/// Prints the word a reader sees for one object's kind.
fn format_kind(kind: &ObjectKind) -> String {
    match kind {
        ObjectKind::Form => "form".to_owned(),
        ObjectKind::Module => "module".to_owned(),
        ObjectKind::Class => "class".to_owned(),
        ObjectKind::Unknown(value) => format!("unknown, raw value {value:#010x}"),
    }
}

/// Prints one line per procedure slot an object declares, or, for a
/// standard module, the sentence D-13 asks for instead of an empty list.
fn print_procedures(procedures: &ObjectProcedures) {
    match procedures {
        ObjectProcedures::NoNameArray { proc_count } => {
            println!(
                "    {proc_count} procedure(s) declared; their names are not reachable through this structure"
            );
        }
        ObjectProcedures::Slots(entries) => {
            for entry in entries {
                match entry {
                    ProcedureEntry::Private => println!("    private"),
                    ProcedureEntry::Public { name, prototype } => {
                        println!("    {}", format_prototype(name, prototype.as_ref()));
                    }
                }
            }
        }
    }
}

/// Prints a public procedure as a prototype: its name, its argument list and
/// its return type when the descriptor says the member returns a value.
///
/// `prototype` is `None` when the name resolved but the type descriptor at
/// the same slot did not; the name still prints, with an empty argument
/// list, rather than being withheld.
fn format_prototype(name: &str, prototype: Option<&Prototype>) -> String {
    let Some(prototype) = prototype else {
        return format!("{name}()");
    };
    let args: Vec<String> = prototype.arguments.iter().map(format_argument).collect();
    let mut line = format!("{name}({})", args.join(", "));
    if let Some(return_type) = &prototype.return_type {
        line.push_str(" As ");
        line.push_str(&format_type_entry(return_type));
    }
    line
}

/// Prints one argument: its `Optional` and `ByRef` modifiers, its name, its
/// `Array` modifier, its type, and its recovered default when it has one.
fn format_argument(arg: &Argument) -> String {
    let mut prefix = String::new();
    if arg.entry.optional {
        prefix.push_str("Optional ");
    }
    if arg.entry.by_ref {
        prefix.push_str("ByRef ");
    }

    let mut piece = format!("{prefix}{}", arg.name);
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

/// Prints a type entry's base type. Modifiers are printed by the caller,
/// which knows whether it is printing an argument or a return type.
fn format_type_entry(entry: &TypeEntry) -> String {
    format_vb_type(&entry.vb_type)
}

/// Prints a `VbType`. Per D-07, an unrecognised type code prints the raw
/// byte and a marker, never a guessed name.
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
        VbType::Object => "Object".to_owned(),
        VbType::HResult => "HResult".to_owned(),
        VbType::Internal(va) => format!(
            "Object (an internal class; its name is not resolved through this path, raw address {:#010x})",
            va.get()
        ),
        VbType::ComIFace(va) => format!(
            "Object (an external COM interface, unresolved, raw address {:#010x})",
            va.get()
        ),
        VbType::ComObj(va) => format!(
            "Object (an external COM object, unresolved, raw address {:#010x})",
            va.get()
        ),
        VbType::Unknown(code) => format!("unrecognised type, raw value {code:#04x}"),
    }
}

/// Prints an `Optional` argument's recovered default value.
fn format_default(default: &DefaultValue) -> String {
    match default {
        DefaultValue::Empty => "Empty".to_owned(),
        DefaultValue::Integer(value) => value.to_string(),
        DefaultValue::Single(value) => value.to_string(),
        DefaultValue::Boolean(value) => value.to_string(),
        DefaultValue::Byte(value) => value.to_string(),
        DefaultValue::Text(value) => format!("{value:?}"),
    }
}

/// Prints the gaps section: every open question this run found, per D-10.
///
/// This prints before [`print_objects`], so a reader meets the
/// standard-module cap before they meet the per-object procedure counts it
/// explains. The standard-module cap always prints, even at zero, because
/// D-10 asks for it to be stated, not only reached for when a count looks
/// low.
fn print_gaps(report: &Report) {
    println!("Gaps");

    let (cap_objects, cap_slots) = standard_module_cap(report);
    println!(
        "  the standard-module cap applies to {cap_objects} object(s) and {cap_slots} \
         procedure slot(s) in this file; a standard module's procedure names are not \
         reachable through this structure at all"
    );

    for object in &report.objects {
        if let ObjectKind::Unknown(value) = object.kind {
            println!(
                "  {:?} has an unrecognised type value {value:#010x}",
                object.name
            );
        }
    }

    for object in &report.objects {
        for prototype in prototype_entries(&object.procedures) {
            for arg in &prototype.arguments {
                if let VbType::Unknown(code) = arg.entry.vb_type {
                    println!(
                        "  {:?}'s argument {:?} has an unrecognised type code {code:#04x}",
                        object.name, arg.name
                    );
                }
            }
            if let Some(TypeEntry {
                vb_type: VbType::Unknown(code),
                ..
            }) = prototype.return_type
            {
                println!(
                    "  {:?}'s return type has an unrecognised type code {code:#04x}",
                    object.name
                );
            }
        }
    }

    for object in &report.objects {
        for gap in &object.gaps {
            let Gap::UnexplainedPublicVarCount(count) = gap;
            println!(
                "  {:?}: cntPublicVars is {count}, and its meaning is unresolved",
                object.name
            );
        }
    }

    for defect in &report.defects {
        if defect.site.structure == "FuncTypDesc"
            || (defect.site.structure == "PrivateObj" && defect.site.field == "lpFuncTypeInfo")
        {
            println!("  a procedure's type descriptor was reported unrecoverable: {defect}");
        }
    }
}

/// Sums the standard-module cap over every object in the report: how many
/// objects carry no procedure name array at all, and how many procedure
/// slots that applies to.
fn standard_module_cap(report: &Report) -> (usize, u32) {
    let mut objects = 0_usize;
    let mut slots = 0_u32;
    for object in &report.objects {
        if let ObjectProcedures::NoNameArray { proc_count } = object.procedures {
            objects = objects.saturating_add(1);
            slots = slots.saturating_add(proc_count);
        }
    }
    (objects, slots)
}

/// Gives every resolved [`Prototype`] an object's procedures carry, skipping
/// a private slot and a slot whose type descriptor did not resolve.
fn prototype_entries(procedures: &ObjectProcedures) -> Vec<&Prototype> {
    match procedures {
        ObjectProcedures::Slots(entries) => entries
            .iter()
            .filter_map(|entry| match entry {
                ProcedureEntry::Public {
                    prototype: Some(prototype),
                    ..
                } => Some(prototype),
                ProcedureEntry::Public {
                    prototype: None, ..
                }
                | ProcedureEntry::Private => None,
            })
            .collect(),
        ObjectProcedures::NoNameArray { .. } => Vec::new(),
    }
}

/// Prints the declarations section: one line per external `Declare`
/// statement, per OBJ-05.
///
/// Only an external (`dwEntryType == 7`) entry ever reaches
/// `Report.declarations`; an internal entry is resolved inside the runtime
/// and is never in this list, per `vb/project.rs`'s own `DeclareTable::read`.
/// A program with no external import prints the heading and a line saying
/// there are none, rather than printing nothing: silence reads as "the tool
/// did not look."
fn print_declarations(report: &Report) {
    println!("Declarations");
    if report.declarations.is_empty() {
        println!("  there are no external declarations in this file");
        return;
    }
    for declaration in &report.declarations {
        let export = match &declaration.export {
            ExportName::Name(name) => name.clone(),
            // Per D-09, an inferred item prints with its marker: no corpus
            // program has ever produced this path (a corpus-wide script
            // found zero ordinal exports across 220 external entries), so
            // it is exercised only by this branch's own logic, never by a
            // real run.
            ExportName::OrdinalInferred(ordinal) => format!("#{ordinal} (inferred alias)"),
        };
        println!("  {}!{export}", declaration.library);
        println!("    {}", Declaration::NAME_MARKER);
        println!("    {}", Declaration::ARGUMENTS_MARKER);
        println!("    {}", Declaration::SCOPE_MARKER);
    }
}
