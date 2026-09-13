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
//!
//! `extract`'s own new failure modes — an input this run could not read, an
//! output directory that already holds files without `--force`, a report
//! path this run could not write, and a recovered file name that would
//! escape the resolved output directory — all map to code 5, the same
//! usage-error bucket a missing or malformed `--opcode-table` argument
//! already uses. The table's own numbering does not move.

use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use clap::Parser as _;

use deform6::Report;
use deform6::vb::classify::ObjectKind;
use deform6::vb::controlinfo::EventReport;
use deform6::vb::controltree::ControlKind;
use deform6::vb::functyp::{Argument, DefaultValue, Prototype, TypeEntry, VbType};
use deform6::vb::ocx::{Clsid, ExternalControl, OcxHeader};
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::privateobj::Gap;
use deform6::vb::project::{Declaration, ExportName};
use deform6::vb::propstream::PropertyValue;
use deform6::vb::{ControlReport, FormReport, ObjectProcedures, ProcedureEntry};

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

    /// Reads one executable and writes a Visual Basic 6 project directory
    /// that VB6 can open.
    Extract {
        /// The executable to read.
        ///
        /// Taken as a path, not as a string, for the same reason
        /// `Inspect::input` already is.
        input: PathBuf,

        /// The directory `extract` writes the project into.
        ///
        /// Every file this run writes is built in memory first; the
        /// directory is resolved to an absolute, canonicalized path, and
        /// every file this run is about to write is checked to be a
        /// direct child of it, before the directory is created and files
        /// land in it. A short and a long form: `-o`/`--output`, locked by
        /// plan 04-01's own research, since no earlier `deform6-cli`
        /// convention exists for a directory-taking flag.
        #[arg(short, long)]
        output: PathBuf,

        /// Moves the JSON report to this path instead of
        /// `<output>/<name>.report.json`.
        ///
        /// Resolved the same way `output` is, but never checked to be a
        /// child of it: the user named this path directly, so it is not an
        /// escape. The resolved path is printed once the report is
        /// written.
        #[arg(long)]
        report: Option<PathBuf>,

        /// Overwrites a non-empty output directory.
        ///
        /// Absent this flag, a run into a directory that already holds at
        /// least one entry refuses, naming the directory. Every file is
        /// still checked to be a direct child of the resolved directory
        /// first, whether or not this flag is given.
        #[arg(long)]
        force: bool,
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
        Command::Extract {
            input,
            output,
            report,
            force,
        } => run_extract(input, output, report.as_deref(), *force),
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
    let (table, table_summary) = match load_opcode_table(opcode_table_path) {
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

    match deform6::inspect(&data, &table) {
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

/// Reads `input`, runs [`deform6::inspect`] and then
/// [`deform6::write::project`] over the bytes, and writes every returned
/// file into `output`.
///
/// Every file this run produces is built in memory before any byte
/// reaches the disk: `output` is created, and files are written into it,
/// only after both calls succeed. A refusal from either call therefore
/// leaves `output` exactly as it was before this run. Every new failure
/// mode this subcommand introduces (an unreadable input file, a refusal
/// from [`deform6::write::project`], or a directory or a file this run
/// could not create) maps to [`Exit::Internal`], code 5, the same usage
/// error bucket [`load_opcode_table`]'s own failures already use: the
/// numbering must never move.
fn run_extract(input: &Path, output: &Path, report_path: Option<&Path>, force: bool) -> Exit {
    // Kept alive for the whole call: `deform6::write::project` re-reads a
    // resource blob's own bytes out of this same slice, so it must outlive
    // the write step. A later refactor that drops this early would compile
    // only if a copy of the whole executable were introduced instead.
    let data = match std::fs::read(input) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("could not read {}: {err}", input.display());
            return Exit::Internal;
        }
    };

    let table = OpcodeTable::builtin();
    let inspected = match deform6::inspect(&data, &table) {
        Ok(inspected) => inspected,
        Err(refusal) => {
            eprintln!("{refusal}");
            return exit_for(refusal);
        }
    };

    // The whole project is built in memory here, before `output` is
    // touched at all. A refusal from this call leaves the file system
    // exactly as it was before this run started.
    let written = match deform6::write::project(&inspected, &data) {
        Ok(written) => written,
        Err(refusal) => {
            eprintln!("{refusal}");
            return Exit::Internal;
        }
    };

    let resolved_output = match resolve_output_dir(output) {
        Ok(resolved) => resolved,
        Err(exit) => return exit,
    };

    write_project(&written, &resolved_output, report_path, force)
}

/// Resolves `output` to an absolute, canonicalized path, following
/// symbolic links, creating the directory first when it does not exist
/// yet: `std::fs::canonicalize` refuses a path that is not there.
///
/// Creating an empty directory here is not "a byte written": the whole
/// project already sits in memory by the time this function runs (see
/// [`run_extract`]), so a refusal that follows (an existing directory with
/// files and no `--force`, or a file name that would escape) still leaves
/// the directory with zero entries, never a partial project.
fn resolve_output_dir(output: &Path) -> Result<PathBuf, Exit> {
    if let Err(err) = std::fs::create_dir_all(output) {
        eprintln!("could not create {}: {err}", output.display());
        return Err(Exit::Internal);
    }
    match std::fs::canonicalize(output) {
        Ok(resolved) => Ok(resolved),
        Err(err) => {
            eprintln!("could not resolve {}: {err}", output.display());
            Err(Exit::Internal)
        }
    }
}

/// Refuses a `resolved_dir` that already holds at least one entry, unless
/// `force` is given. An empty existing directory, or one this run just
/// created, is fine either way.
fn ensure_directory_is_writable(resolved_dir: &Path, force: bool) -> Result<(), Exit> {
    if force {
        return Ok(());
    }
    match std::fs::read_dir(resolved_dir) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                eprintln!(
                    "{} already holds files; pass --force to overwrite",
                    resolved_dir.display()
                );
                Err(Exit::Internal)
            } else {
                Ok(())
            }
        }
        Err(err) => {
            eprintln!("could not read {}: {err}", resolved_dir.display());
            Err(Exit::Internal)
        }
    }
}

/// Collapses `.` and `..` components lexically, without touching the file
/// system: the one way to check a path that does not exist yet for
/// containment, since [`std::fs::canonicalize`] refuses a path that has
/// not been written.
///
/// A leading `..` past the root, or past whatever this function has
/// already pushed, is dropped rather than made to underflow: [`PathBuf`]
/// itself refuses to pop past its own start, so this is a no-op in that
/// case, never a panic.
fn lexically_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Builds the on-disk destination for every file `files` gives, verified
/// to be a direct child of `resolved_dir`: [`lexically_normalize`] the
/// joined path, then compare its own parent against `resolved_dir` by
/// value, never by a string prefix. Touches no file system: this is the
/// one place a recovered file name becomes a path, and it is checked here,
/// before any byte reaches disk, so a test can drive it with names built
/// inside the test (T-4-02).
///
/// # Errors
///
/// Gives one message naming the file and the directory it would have
/// escaped, the first time any file's own path is not a direct child of
/// `resolved_dir`. The whole run refuses; nothing already checked is kept.
fn plan_writes<'a>(
    files: impl Iterator<Item = &'a deform6::write::WrittenFile>,
    resolved_dir: &Path,
) -> Result<Vec<(PathBuf, &'a [u8])>, String> {
    let mut plan = Vec::new();
    for file in files {
        let candidate = lexically_normalize(&resolved_dir.join(&file.name));
        let Some(parent) = candidate.parent() else {
            return Err(format!(
                "{:?} names a path with no parent directory; refusing the whole run",
                file.name
            ));
        };
        if parent != resolved_dir {
            return Err(format!(
                "{:?} would write outside {}, at {}; refusing the whole run",
                file.name,
                resolved_dir.display(),
                candidate.display()
            ));
        }
        plan.push((candidate, file.bytes.as_slice()));
    }
    Ok(plan)
}

/// Resolves a user-named `--report` path the same way `output` is
/// resolved: follows symbolic links on the directory that will hold it.
/// Never checked against the output directory: the user named this path
/// directly, so it is not an escape (T-4-23).
fn resolve_report_path(path: &Path) -> Result<PathBuf, Exit> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let Some(file_name) = path.file_name() else {
        eprintln!("{} names no file", path.display());
        return Err(Exit::Internal);
    };
    if let Err(err) = std::fs::create_dir_all(parent) {
        eprintln!("could not create {}: {err}", parent.display());
        return Err(Exit::Internal);
    }
    match std::fs::canonicalize(parent) {
        Ok(resolved_parent) => Ok(resolved_parent.join(file_name)),
        Err(err) => {
            eprintln!(
                "could not resolve the directory holding {}: {err}",
                path.display()
            );
            Err(Exit::Internal)
        }
    }
}

/// Writes every file `written` holds into `resolved_dir`, having already
/// resolved it (see [`resolve_output_dir`]): refuses a non-empty directory
/// without `--force`, plans and checks every path with [`plan_writes`]
/// before writing a single byte, then writes. When `report_path` is
/// given, the JSON report goes there instead of into `resolved_dir`, is
/// never checked for containment, and its own resolved path is printed.
fn write_project(
    written: &deform6::write::WrittenProject,
    resolved_dir: &Path,
    report_path: Option<&Path>,
    force: bool,
) -> Exit {
    if let Err(exit) = ensure_directory_is_writable(resolved_dir, force) {
        return exit;
    }

    let files = written
        .files
        .iter()
        .filter(|file| report_path.is_none() || !file.name.ends_with(".report.json"));
    let plan = match plan_writes(files, resolved_dir) {
        Ok(plan) => plan,
        Err(message) => {
            eprintln!("{message}");
            return Exit::Internal;
        }
    };

    for (path, bytes) in &plan {
        if let Err(err) = std::fs::write(path, bytes) {
            eprintln!("could not write {}: {err}", path.display());
            return Exit::Internal;
        }
    }

    if let Some(report_path) = report_path {
        let resolved_report = match resolve_report_path(report_path) {
            Ok(resolved) => resolved,
            Err(exit) => return exit,
        };
        let report_bytes = written.report.to_json().into_bytes();
        if let Err(err) = std::fs::write(&resolved_report, report_bytes) {
            eprintln!("could not write {}: {err}", resolved_report.display());
            return Exit::Internal;
        }
        println!("{}", resolved_report.display());
    }

    Exit::Ok
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
    print_forms(report);
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

/// Prints one section per form: its own name, its control count, and every
/// control's own tree beneath it. Per FRM-01, a control's indentation shows
/// its parent, so the tree is visible without a second pass.
///
/// A form whose own control tree could not be built prints the refusal each
/// of its own defects names, and no tree: `inspect` never builds a tree it
/// cannot prove.
fn print_forms(report: &Report) {
    println!("Forms");
    if report.forms.is_empty() {
        println!("  this file declares no forms");
        return;
    }
    for form in &report.forms {
        print_form(form);
    }
}

/// Prints one form's own section.
fn print_form(form: &FormReport) {
    println!("  {}  ({} control(s))", form.name, form.controls.len());
    if form.controls.is_empty() {
        for defect in &form.defects {
            println!("    refused: {defect}");
        }
        return;
    }
    for (index, control) in form.controls.iter().enumerate() {
        print_control(control, control_depth(&form.controls, index));
    }
}

/// Gives the depth of `controls[index]`: the number of `parent` hops back to
/// the root. The root itself (the form's own outermost block, `parent:
/// None`) is depth `0`.
///
/// Bounded by `controls.len()`: a cycle in `parent` links would otherwise
/// loop the printer forever, and `AGENTS.md` bars trusting the file that
/// far. No corpus program produces one; the bound is defensive, not a value
/// read from the file.
fn control_depth(controls: &[ControlReport], index: usize) -> usize {
    let mut depth = 0_usize;
    let mut current = index;
    for _ in 0..=controls.len() {
        let Some(parent) = controls.get(current).and_then(|c| c.parent) else {
            return depth;
        };
        depth = depth.saturating_add(1);
        current = parent;
    }
    depth
}

/// Prints one control: its name, its type, its array index when it has one,
/// indented to show its own parent, then its properties, its external
/// control facts when its type is 255, and its event slots.
fn print_control(control: &ControlReport, depth: usize) {
    let indent = "  ".repeat(depth.saturating_add(2));
    let kind = format_control_kind(&control.kind);
    match control.array_index {
        Some(array_index) => println!("{indent}{}  ({kind}, Index={array_index})", control.name),
        None => println!("{indent}{}  ({kind})", control.name),
    }

    for property in &control.properties {
        print_property(&indent, property);
    }

    if let Some(external) = &control.external {
        print_external(
            &indent,
            external,
            control.external_reason.as_deref(),
            control.ocx_header.as_ref(),
            control.opaque_message.as_deref(),
        );
    }

    for event in &control.events {
        print_event(&indent, event);
    }
}

/// Prints the word a reader sees for one control's type: a name, or, for a
/// value `03-RESEARCH.md` section 8.4.1's table does not cover, the raw
/// value carried inside `ControlKind::Unknown`'s own `Debug` rendering.
fn format_control_kind(kind: &ControlKind) -> String {
    format!("{kind:?}")
}

/// Prints one property. A property this repository can name prints its
/// name and its value. A property this repository cannot name prints
/// present and undecoded, with its byte offset, its opcode number and the
/// control type, and names the command line flag that would supply a
/// table. A resource blob prints its own recovered facts, or, when it
/// could not be read, that it is present and unreadable.
fn print_property(indent: &str, property: &PropertyValue) {
    match property {
        PropertyValue::Byte { name, value } => println!("{indent}  {name} = {value}"),
        PropertyValue::Boolean { name, value } => println!("{indent}  {name} = {value}"),
        PropertyValue::Integer { name, value } => println!("{indent}  {name} = {value}"),
        PropertyValue::Long { name, value } => println!("{indent}  {name} = {value}"),
        PropertyValue::Single { name, value } => println!("{indent}  {name} = {value}"),
        PropertyValue::Text { name, value } => println!("{indent}  {name} = {value:?}"),
        PropertyValue::Position { name, value } => println!("{indent}  {name} = {value:?}"),
        PropertyValue::Font { name, value } => println!("{indent}  {name} = {value:?}"),
        PropertyValue::Blob {
            name,
            offset,
            declared_len,
            image_len,
            format,
            frx_offset,
        } => {
            println!(
                "{indent}  {name}: resource blob at offset {offset:#x}, declared length \
                 {declared_len}, {image_len} image byte(s), format {}, .frx offset \
                 {frx_offset:#x} (an offset into a file this run did not write)",
                format_image_format(format)
            );
        }
        PropertyValue::BlobUnreadable { name, offset } => {
            println!(
                "{indent}  {name}: resource blob at offset {offset:#x}: present and unreadable."
            );
        }
        PropertyValue::Undecoded {
            opcode,
            offset,
            control_type,
            ..
        } => {
            println!(
                "{indent}  property opcode {opcode} at offset {offset:#x} on {control_type}: \
                 not decoded. Run with --opcode-table to supply one."
            );
        }
    }
}

/// Prints the word a reader sees for one resource blob's own detected
/// container format. `Unknown` names no format that was never proved and
/// prints only how many prefix bytes were checked, never the raw bytes: per
/// this plan's own threat model, the print arm names counts and offsets
/// only.
fn format_image_format(format: &deform6::vb::frx::ImageFormat) -> String {
    match format {
        deform6::vb::frx::ImageFormat::Bmp => "BMP".to_owned(),
        deform6::vb::frx::ImageFormat::Gif => "GIF".to_owned(),
        deform6::vb::frx::ImageFormat::Jpeg => "JPEG".to_owned(),
        deform6::vb::frx::ImageFormat::Wmf => "WMF".to_owned(),
        deform6::vb::frx::ImageFormat::Emf => "EMF".to_owned(),
        deform6::vb::frx::ImageFormat::Ico => "ICO".to_owned(),
        deform6::vb::frx::ImageFormat::Cur => "CUR".to_owned(),
        deform6::vb::frx::ImageFormat::Unknown(bytes) => {
            format!("unrecognised ({} prefix byte(s) checked)", bytes.len())
        }
    }
}

/// Prints an external (`cType` 255) control's own facts: its class name,
/// its CLSID (with the caveat naming what that value is not, per plan
/// 03-16) or the stated reason for having none, its extents, and the
/// opaque blob statement.
fn print_external(
    indent: &str,
    external: &ExternalControl,
    reason: Option<&str>,
    header: Option<&OcxHeader>,
    opaque_message: Option<&str>,
) {
    println!("{indent}  class name = {}", external.class_name);
    print_clsid(indent, external.clsid.as_ref(), reason);
    if let Some(header) = header {
        println!(
            "{indent}  extents = {} x {} (HiMetric), version {}",
            header.extent_x, header.extent_y, header.version
        );
    }
    if let Some(message) = opaque_message {
        println!("{indent}  property blob: not decoded. {message}");
    }
}

/// Prints a joined CLSID, or the stated reason none was joined.
///
/// Plan 03-16: `reason` also carries a caveat when `clsid` is `Some`, since
/// this repository's own research never confirmed the field a reported
/// CLSID comes from against a project file's own declared identifier. The
/// caveat is printed on its own line beneath the value, so a reader sees
/// what the value is and what it is not without opening the project file.
fn print_clsid(indent: &str, clsid: Option<&Clsid>, reason: Option<&str>) {
    match clsid {
        Some(clsid) => {
            println!("{indent}  CLSID = {clsid}");
            if let Some(caveat) = reason {
                println!("{indent}  {caveat}");
            }
        }
        None => println!(
            "{indent}  CLSID: {}",
            reason.unwrap_or("not recoverable from this file")
        ),
    }
}

/// Prints one event slot: its index, its bound state, its name or the
/// stated reason for having none, and, for a bound slot, its handler's own
/// native address or the stated reason it is not decoded. An unbound slot
/// prints exactly what it printed before this plan: no address, no
/// placeholder, no zero.
///
/// The address prints as eight hexadecimal digits with a `0x` prefix
/// (`{:#010x}`), the same width `print_report`'s own `Header` line already
/// uses for a full virtual address read from the file. `print_property`
/// and the CLSID caveat both print a byte offset unpadded (`{:#x}`), but a
/// byte offset is a small position inside one file and a handler address
/// is a full address the same shape the Header line already prints, so
/// this line follows the Header line, not the offset lines.
fn print_event(indent: &str, event: &EventReport) {
    match event {
        EventReport::Named {
            index,
            event_name,
            handler_address,
            ..
        } => match handler_address {
            Some(address) => println!(
                "{indent}  event slot {index}: bound, {event_name}, handler at {address:#010x}"
            ),
            None => println!(
                "{indent}  event slot {index}: bound, {event_name}, handler address not decoded"
            ),
        },
        EventReport::BoundUnnamed {
            index,
            handler_address,
            ..
        } => match handler_address {
            Some(address) => println!(
                "{indent}  event slot {index}: bound, handler at {address:#010x}, name not \
                 decoded. Run with --event-name-table to supply one."
            ),
            None => println!(
                "{indent}  event slot {index}: bound, handler address not decoded, name not \
                 decoded. Run with --event-name-table to supply one."
            ),
        },
        EventReport::Unbound { index, .. } => println!(
            "{indent}  event slot {index}: unbound, not decoded. Run with --event-name-table to \
             supply one."
        ),
    }
}

// --- Plan 04-08, Task 2: the containment check, driven with names built
// inside the test, because the corpus holds no hostile name -------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::*;
    use deform6::report::ProjectReport;
    use deform6::write::model::{NameKind, SafeName};
    use deform6::write::{WrittenFile, WrittenProject};

    /// A [`WrittenProject`] whose one file carries `name` verbatim: the
    /// hostile name a real [`SafeName`] can never produce, built directly
    /// here because the corpus holds none.
    fn project_named(name: &str) -> WrittenProject {
        WrittenProject {
            files: vec![WrittenFile {
                name: name.to_owned(),
                bytes: b"hostile".to_vec(),
            }],
            report: ProjectReport {
                items: Vec::new(),
                defects: Vec::new(),
                limits: Vec::new(),
            },
        }
    }

    /// A fresh, empty, resolved (canonicalized) temporary directory this
    /// test owns; the caller removes it.
    fn fresh_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "deform6-cli-test-containment-{label}-{}-{}",
            std::process::id(),
            label.len()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("creating a fresh temp directory must succeed");
        std::fs::canonicalize(&dir).expect("canonicalizing a directory that exists must succeed")
    }

    /// Asserts `dir` holds no entry at all.
    fn assert_empty(dir: &Path) {
        let entries: Vec<_> = std::fs::read_dir(dir)
            .expect("reading the directory must succeed")
            .collect();
        assert!(
            entries.is_empty(),
            "the run must write nothing when it refuses: {entries:?}"
        );
    }

    #[test]
    fn a_file_name_holding_a_path_separator_refuses_the_whole_run_and_writes_nothing() {
        let resolved = fresh_dir("separator");
        let project = project_named("sub/escape.frm");

        let exit = write_project(&project, &resolved, None, false);

        assert_eq!(exit, Exit::Internal);
        assert_empty(&resolved);
        std::fs::remove_dir_all(&resolved).ok();
    }

    #[test]
    fn a_file_name_holding_a_parent_directory_sequence_refuses_the_whole_run_and_writes_nothing() {
        let resolved = fresh_dir("parent-dir");
        let project = project_named("../escape.frm");

        let exit = write_project(&project, &resolved, None, false);

        assert_eq!(exit, Exit::Internal);
        assert_empty(&resolved);
        std::fs::remove_dir_all(&resolved).ok();
    }

    #[test]
    fn an_absolute_file_name_refuses_the_whole_run_and_writes_nothing() {
        let resolved = fresh_dir("absolute");
        let project = project_named("/etc/escape.frm");

        let exit = write_project(&project, &resolved, None, false);

        assert_eq!(exit, Exit::Internal);
        assert_empty(&resolved);
        std::fs::remove_dir_all(&resolved).ok();
    }

    /// A run that refuses one file refuses the whole run: a project whose
    /// first file is legitimate and second file is hostile still writes
    /// zero files, not the one legitimate file.
    #[test]
    fn a_hostile_name_after_a_legitimate_one_still_writes_nothing_at_all() {
        let resolved = fresh_dir("partial");
        let project = WrittenProject {
            files: vec![
                WrittenFile {
                    name: "Project1.vbp".to_owned(),
                    bytes: b"Type=Exe\r\n".to_vec(),
                },
                WrittenFile {
                    name: "../escape.frm".to_owned(),
                    bytes: b"hostile".to_vec(),
                },
            ],
            report: ProjectReport {
                items: Vec::new(),
                defects: Vec::new(),
                limits: Vec::new(),
            },
        };

        let exit = write_project(&project, &resolved, None, false);

        assert_eq!(exit, Exit::Internal);
        assert_empty(&resolved);
        std::fs::remove_dir_all(&resolved).ok();
    }

    /// The containment check is a second line of defence, not the only
    /// one: [`SafeName`] itself already refuses to produce any of the
    /// three hostile shapes above, for any raw name at all.
    #[test]
    fn the_sanitized_name_type_cannot_produce_a_separator_a_parent_sequence_or_an_absolute_path() {
        for raw in [
            "a/b",
            "a\\b",
            "..",
            "../c",
            "/etc/passwd",
            "C:\\Windows\\System32",
        ] {
            let (name, _faults) = SafeName::new(raw, NameKind::Form);
            let file_name = name.file_name("frm");
            assert!(!file_name.contains('/'), "{file_name:?} from {raw:?}");
            assert!(!file_name.contains('\\'), "{file_name:?} from {raw:?}");
            assert!(!file_name.contains(".."), "{file_name:?} from {raw:?}");
            assert!(
                !Path::new(&file_name).is_absolute(),
                "{file_name:?} from {raw:?}"
            );
        }
    }
}
