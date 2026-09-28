//! `cargo run -p xtask -- export-lift-builds <dir>` writes each P-code
//! program of `corpus-pcode/` as `extract --lift` writes it, for a build with
//! the Visual Basic 6 IDE on a Windows host.
//!
//! The layout is the layout of `export-builds` (see `builds.rs`):
//! `extracted/pNN/` holds the project that DeForm6 writes with the lifted
//! bodies, `original/pNN/` holds the project of the source, and `build.bat`
//! builds each side. The logs tell which lifted project compiles, and the
//! first error of each one that does not.
//!
//! The P-code table and the types file are not in the repository. The
//! command reads them from `derived/`, or from `--pcode-table` and
//! `--vb-types`.

use std::path::Path;

use deform6::journal::Mode;
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::pcode::PcodeTable;
use deform6::vb::types::VbTypes;

use crate::build_record::{self, corpus_root, executables, program_key};
use crate::builds::{
    Exported, HOST_OUT_DIR, check_batch_name, copy_without_executables, crlf_text, prepare,
    short_name, write_files, write_lists,
};
use crate::pcode_record::pcode_root;
use crate::ratios::differential::support::vbp;

/// The options of `export-lift-builds`.
struct Options {
    table: String,
    types: String,
    dir: String,
}

/// Reads `[--pcode-table <file>] [--vb-types <file>] <dir>`.
fn options(args: &[String]) -> Option<Options> {
    let mut table = crate::pcode_table::DEFAULT_OUTPUT_PATH.to_owned();
    let mut types = crate::vb_types::DEFAULT_OUTPUT_PATH.to_owned();
    let mut dir = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--pcode-table" => table.clone_from(rest.next()?),
            "--vb-types" => types.clone_from(rest.next()?),
            flag if flag.starts_with('-') => return None,
            path if dir.is_none() => dir = Some(path.to_owned()),
            _ => return None,
        }
    }
    Some(Options {
        table,
        types,
        dir: dir?,
    })
}

/// Gives the files that `extract --lift` writes for the program `bytes`,
/// through [`build_record::build_files`].
fn lifted_files(
    bytes: &[u8],
    table: &PcodeTable,
    types: &VbTypes,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut report =
        deform6::inspect_with_types(bytes, &OpcodeTable::builtin(), Some(types), Mode::Strict)
            .map_err(|err| format!("inspect: {err}"))?;
    let lifted = deform6::vb::bodies::lift_objects(bytes, &report, table, Some(types))
        .map_err(|err| format!("lift: {err}"))?;
    for (object, lifted) in report.objects.iter_mut().zip(lifted) {
        object.lifted = lifted;
    }
    let written = deform6::write::project(&report, bytes, Mode::Strict)
        .map_err(|err| format!("write::project: {err}"))?;
    Ok(build_record::build_files(
        written
            .files
            .into_iter()
            .map(|file| (file.name, file.bytes))
            .collect(),
    ))
}

/// Writes the whole export into `dir`, and gives the number of programs.
fn export(options: &Options) -> Result<usize, String> {
    let read = |path: &str| std::fs::read(path).map_err(|err| format!("reading {path}: {err}"));
    let table = PcodeTable::parse(&read(&options.table)?)
        .map_err(|err| format!("{}: {err}", options.table))?;
    let types = VbTypes::parse(&read(&options.types)?)
        .map_err(|err| format!("{}: {err}", options.types))?;
    let dir = Path::new(&options.dir);
    prepare(dir)?;
    let root = pcode_root();
    let corpus = corpus_root();
    let projects = vbp::project_files();
    let mut exported = Vec::new();
    for (index, exe) in executables(&root)?.iter().enumerate() {
        let short = short_name(index.checked_add(1).ok_or("too many programs")?)?;
        let key = program_key(exe, &root)?;
        let bytes =
            std::fs::read(exe).map_err(|err| format!("reading {}: {err}", exe.display()))?;
        let files = lifted_files(&bytes, &table, &types).map_err(|err| format!("{key}: {err}"))?;
        write_files(&dir.join("extracted").join(&short), &files)
            .map_err(|err| format!("{key}: {err}"))?;
        let extracted_vbp = files
            .iter()
            .map(|(name, _)| name)
            .find(|name| {
                Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("vbp"))
            })
            .cloned()
            .ok_or_else(|| format!("{key}: DeForm6 wrote no project file"))?;
        let original = vbp::select_project_file(&corpus.join(&key), &projects)
            .map_err(|err| format!("{key}: no project file: {err:?}"))?;
        let original_dir = original
            .parent()
            .ok_or_else(|| format!("{key}: the project file has no directory"))?;
        copy_without_executables(original_dir, &dir.join("original").join(&short))
            .map_err(|err| format!("{key}: {err}"))?;
        let original_vbp = original
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| format!("{key}: the project file has no name"))?;
        check_batch_name(&original_vbp).map_err(|err| format!("{key}: {err}"))?;
        exported.push(Exported {
            short,
            key,
            original_vbp,
            extracted_vbp,
            files: build_record::files_hash(&files),
        });
    }
    write_lists(dir, &exported)?;
    let runs = dir.join("runs.bat");
    std::fs::write(&runs, render_runs(&exported))
        .map_err(|err| format!("writing {}: {err}", runs.display()))?;
    Ok(exported.len())
}

/// The seconds that `runs.bat` gives a program to show its first window
/// before it sends the marker, and after it.
const RUN_WAIT_SECONDS: u8 = 6;

/// Renders `runs.bat`, with CRLF at the end of each line.
///
/// After `build.bat`, the file starts each executable that VB6 built, the
/// original side and then the extracted side of each program. It waits
/// [`RUN_WAIT_SECONDS`] seconds, sends `===SHOT <short> <side>===` out
/// through `COM1`, waits again, and ends the program. The other machine
/// takes a picture of the screen at each marker, so the first window of each
/// rebuilt program can be compared with the first window of its original.
/// A side with no executable sends `===NONE <short> <side>===`. `ping`
/// waits, because Windows XP has no `timeout`.
fn render_runs(programs: &[Exported]) -> String {
    let wait = format!(
        "ping -n {} 127.0.0.1 >nul",
        RUN_WAIT_SECONDS.saturating_add(1)
    );
    let mut lines: Vec<String> = [
        "@echo off",
        "rem Starts each program that build.bat built, and sends a marker through",
        "rem COM1 while its first window shows, for the other machine to take a",
        "rem picture of the screen.",
        "mode COM1: baud=115200 parity=n data=8 stop=1 to=off xon=off odsr=off octs=off dtr=on rts=on idsr=off >nul",
    ]
    .iter()
    .map(|line| (*line).to_owned())
    .collect();
    for program in programs {
        for side in ["original", "extracted"] {
            lines.push(format!("call :run {} {side}", program.short));
        }
    }
    lines.extend(
        [
            ">COM1 echo ===END===",
            "exit /b 0",
            "",
            ":run",
            "set FOUND=",
            &format!(
                r#"for %%e in ("{HOST_OUT_DIR}\%1\%2\*.exe") do call :one %1 %2 "%%~e" "%%~nxe""#
            ),
            r#"if "%FOUND%"=="" >COM1 echo ===NONE %1 %2==="#,
            "goto :eof",
            "",
            ":one",
            "set FOUND=1",
            r#"start "" %3"#,
            &wait,
            ">COM1 echo ===SHOT %1 %2===",
            &wait,
            r#"taskkill /f /im %4 >nul 2>&1"#,
            "goto :eof",
        ]
        .iter()
        .map(|line| (*line).to_owned()),
    );
    crlf_text(&lines)
}

/// Runs `export-lift-builds`.
pub(crate) fn run(args: &[String]) -> i32 {
    let Some(options) = options(args) else {
        eprintln!(
            "usage: cargo run -p xtask -- export-lift-builds [--pcode-table <file>] \
             [--vb-types <file>] <dir>"
        );
        return 1;
    };
    match export(&options) {
        Ok(count) => {
            println!(
                "xtask: exported {count} lifted programs to {}. Run build.bat there on the \
                 Windows host.",
                options.dir
            );
            0
        }
        Err(err) => {
            eprintln!("xtask: {err}");
            1
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{options, render_runs};
    use crate::builds::Exported;

    #[test]
    fn the_run_file_starts_each_side_of_each_program_and_sends_a_marker() {
        let program = |short: &str| Exported {
            short: short.to_owned(),
            key: "k".to_owned(),
            original_vbp: "a.vbp".to_owned(),
            extracted_vbp: "b.vbp".to_owned(),
            files: "h".to_owned(),
        };
        let text = render_runs(&[program("p01"), program("p02")]);
        assert!(!text.replace("\r\n", "").contains('\n'));
        assert!(text.ends_with("\r\n"));
        let calls: Vec<&str> = text
            .split("\r\n")
            .filter(|line| line.starts_with("call :run"))
            .collect();
        assert_eq!(
            calls,
            [
                "call :run p01 original",
                "call :run p01 extracted",
                "call :run p02 original",
                "call :run p02 extracted"
            ]
        );
        assert!(text.contains(r#"for %%e in ("%SystemDrive%\deform6-out\%1\%2\*.exe")"#));
        assert!(text.contains(">COM1 echo ===SHOT %1 %2==="));
        assert!(text.contains("ping -n 7 127.0.0.1 >nul"));
        assert!(text.contains("taskkill /f /im %4"));
    }

    #[test]
    fn the_options_take_two_files_and_one_directory() {
        let args = |text: &str| -> Vec<String> { text.split(' ').map(str::to_owned).collect() };
        let read = options(&args("--pcode-table t.toml --vb-types v.toml out")).unwrap();
        assert_eq!(
            (read.table.as_str(), read.types.as_str(), read.dir.as_str()),
            ("t.toml", "v.toml", "out")
        );
        let default = options(&args("out")).unwrap();
        assert_eq!(default.table, "derived/pcode-table.toml");
        assert!(options(&args("out other")).is_none());
        assert!(options(&args("--lift")).is_none());
        assert!(options(&args("--vb-types")).is_none());
    }
}
