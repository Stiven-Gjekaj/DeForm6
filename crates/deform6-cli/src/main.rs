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

/// Prints the locked eight-line shape.
///
/// The Runtime and Header lines print `report.runtime_dll` and
/// `report.signature`, which are values [`deform6::inspect`] read out of the
/// file. Nothing here writes what a Visual Basic 6 file is supposed to hold;
/// it prints what this one does.
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
}

/// Prints one line of the report, with the label padded to ten columns.
fn print_line(label: &str, value: &str) {
    println!("{label:<10}{value}");
}
