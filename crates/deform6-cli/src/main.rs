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
        Command::Inspect { input } => run_inspect(input),
    }
}

/// Reads `path`, runs [`deform6::inspect`] over the bytes, and prints one
/// outcome.
///
/// An input-output error is [`Exit::Internal`], code 5, and not
/// [`Exit::NotPe`]: the file was never read, so nothing about its format is
/// known.
fn run_inspect(path: &Path) -> Exit {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("could not read {}: {err}", path.display());
            return Exit::Internal;
        }
    };

    match deform6::inspect(&data) {
        Ok(report) => {
            print_report(path, &report);
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

/// Prints the locked eight-line shape, then the object graph below it.
///
/// The Runtime and Header lines print `report.runtime_dll` and
/// `report.signature`, which are values [`deform6::inspect`] read out of the
/// file. Nothing here writes what a Visual Basic 6 file is supposed to hold;
/// it prints what this one does.
///
/// The eight-line head phase 1 locked stays exactly as it was: this task
/// appends a section below it rather than reshaping it. `crates/deform6-cli/
/// tests/cli.rs` compares the head line for line.
fn print_report(path: &Path, report: &Report) {
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
    print_objects(report);
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
