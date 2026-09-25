//! `cargo run -p xtask -- export-builds <dir>` writes each corpus program in
//! a form that the Visual Basic 6 IDE builds on a Windows host.
//!
//! DeForm6 does not run on the host that the author has: Windows XP. The
//! Rust standard library needs Windows 10, or Windows 7 through tier-3
//! targets. So this machine writes the projects, the host builds them with
//! `build.bat`, and this machine reads the logs back into `tests/builds.toml`.
//!
//! # The export directory
//!
//! - `extracted/pNN/` holds the files that DeForm6 writes for program `NN`:
//!   the files of [`build_record::project_files`].
//! - `original/pNN/` holds a copy of the directory of the project file that
//!   the corpus holds for the same program, with no executable in it.
//! - `manifest.txt` names each program: its short name, its key, the project
//!   file of each side, and the hash of the extracted files.
//! - `build.bat` builds each side of each program with `VB6.EXE /make`, and
//!   writes `logs\`. VB6 writes each executable into `C:\deform6-out` on the
//!   host, so no executable comes back into this directory.
//!
//! The short names, `p01` to `p44`, keep the paths short on the host. They
//! follow the order of the keys.
//!
//! # The probe
//!
//! `export-builds --probe <dir>` writes four small projects in place of the
//! corpus, in the same layout. Each one shows how VB6 reports one kind of
//! result: a project that builds, a syntax error, a component that is not
//! on the host, and a form property that VB6 does not know. The importer
//! reads the logs by the rules that a run of the probe shows. This code
//! writes each project, so no third party byte is in it. Each project is on
//! both sides, so the two builds of one project also show whether VB6 gives
//! the same result twice.

use std::fmt::Write as _;
use std::path::Path;

use crate::build_record::{self, corpus_root, executables, program_key};
use crate::ratios::differential::support::vbp;

/// The default place of `VB6.EXE` on the host. The first argument of
/// `build.bat` replaces it.
const DEFAULT_VB6: &str = r"C:\Program Files\Microsoft Visual Studio\VB98\VB6.EXE";

/// The directory on the host that receives the executables that VB6 builds.
/// It is on the host's own disk, so no executable reaches the shared
/// directory.
const HOST_OUT_DIR: &str = r"C:\deform6-out";

/// The characters that `cmd` gives a meaning to inside a batch file, even
/// between quotes, and the characters that end a path or a line. A name
/// that holds one of them is refused.
const REFUSED_IN_A_NAME: &[char] = &['%', '^', '&', '!', '"', '\\', '/', '\r', '\n'];

/// The files of one project: each name, with its bytes.
type ProjectFiles = Vec<(String, Vec<u8>)>;

/// One program as the export wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Exported {
    /// `p01` to `p99`.
    pub(crate) short: String,
    /// The path of the executable, relative to `corpus/`.
    pub(crate) key: String,
    /// The file name of the project file in `original/<short>/`.
    pub(crate) original_vbp: String,
    /// The file name of the project file in `extracted/<short>/`.
    pub(crate) extracted_vbp: String,
    /// The hash of the extracted files, from [`build_record::files_hash`].
    pub(crate) files: String,
}

/// Runs `export-builds`.
pub(crate) fn run_export(args: &[String]) -> i32 {
    let (probe, dir) = match export_args(args) {
        Ok(read) => read,
        Err(message) => {
            eprintln!("xtask: {message}");
            return 1;
        }
    };
    let exported = if probe {
        export_probe(dir)
    } else {
        export(dir)
    };
    match exported {
        Ok(count) => {
            println!(
                "xtask: exported {count} programs to {}. Run build.bat there on the Windows \
                 host, then run `cargo run -p xtask -- import-builds {}`.",
                dir.display(),
                dir.display()
            );
            0
        }
        Err(message) => {
            eprintln!("xtask: {message}");
            1
        }
    }
}

/// Reads the arguments of `export-builds`: `[--probe] <dir>`. Gives whether
/// the probe is asked for, and the directory.
///
/// A directory that starts with `-` is refused, so that a flag with no
/// directory after it is never read as the name of a directory.
pub(crate) fn export_args(args: &[String]) -> Result<(bool, &Path), String> {
    match args {
        [dir] if !dir.starts_with('-') => Ok((false, Path::new(dir))),
        [flag, dir] if flag == "--probe" && !dir.starts_with('-') => Ok((true, Path::new(dir))),
        _ => Err("usage: cargo run -p xtask -- export-builds [--probe] <dir>".to_owned()),
    }
}

/// Gives the short name of the program at `index`, counted from 1.
pub(crate) fn short_name(index: usize) -> Result<String, String> {
    if index == 0 || index > 99 {
        return Err(format!(
            "a short name holds two digits, so it cannot name program {index}"
        ));
    }
    Ok(format!("p{index:02}"))
}

/// Refuses a file name that `cmd` would change, or that is not one plain
/// file name.
pub(crate) fn check_batch_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("a file name is empty".to_owned());
    }
    if let Some(found) = name.chars().find(|c| REFUSED_IN_A_NAME.contains(c)) {
        return Err(format!(
            "the file name {name:?} holds {found:?}, which build.bat cannot hold safely"
        ));
    }
    Ok(())
}

/// Makes `dir`, which must not exist or must be empty. An export into a
/// directory that holds an old export would mix old logs with new files.
fn prepare(dir: &Path) -> Result<(), String> {
    if dir.exists() {
        let mut entries =
            std::fs::read_dir(dir).map_err(|err| format!("reading {}: {err}", dir.display()))?;
        if entries.next().is_some() {
            return Err(format!(
                "{} is not empty. Give an empty or a new directory, so that no old log \
                 mixes with the new files",
                dir.display()
            ));
        }
    }
    std::fs::create_dir_all(dir).map_err(|err| format!("making {}: {err}", dir.display()))
}

/// Writes `files` into `dir`, which this makes.
fn write_files(dir: &Path, files: &[(String, Vec<u8>)]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|err| format!("making {}: {err}", dir.display()))?;
    for (name, bytes) in files {
        check_batch_name(name)?;
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|err| format!("writing {}: {err}", path.display()))?;
    }
    Ok(())
}

/// Copies the directory `from` into `to`, with each subdirectory, and with
/// no file whose extension is `exe`.
fn copy_without_executables(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|err| format!("making {}: {err}", to.display()))?;
    let entries =
        std::fs::read_dir(from).map_err(|err| format!("reading {}: {err}", from.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|err| format!("reading an entry of {}: {err}", from.display()))?
            .path();
        let Some(name) = path.file_name() else {
            continue;
        };
        let target = to.join(name);
        if path.is_dir() {
            copy_without_executables(&path, &target)?;
        } else if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            std::fs::copy(&path, &target).map_err(|err| {
                format!("copying {} to {}: {err}", path.display(), target.display())
            })?;
        }
    }
    Ok(())
}

/// Gives the name of the one project file in `files`.
fn project_file_name(files: &[(String, Vec<u8>)]) -> Result<String, String> {
    let mut names = files.iter().map(|(name, _bytes)| name).filter(|name| {
        Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("vbp"))
    });
    let first = names
        .next()
        .ok_or("DeForm6 wrote no project file".to_owned())?;
    if names.next().is_some() {
        return Err("DeForm6 wrote more than one project file".to_owned());
    }
    Ok(first.clone())
}

/// Writes the whole export into `dir`, and gives the number of programs.
fn export(dir: &Path) -> Result<usize, String> {
    prepare(dir)?;
    let root = corpus_root();
    let projects = vbp::project_files();
    let exes = executables(&root)?;
    if exes.len() != build_record::EXPECTED_PROGRAM_COUNT {
        return Err(format!(
            "found {} corpus programs, and the record holds {}",
            exes.len(),
            build_record::EXPECTED_PROGRAM_COUNT
        ));
    }

    let mut exported = Vec::new();
    for (index, exe) in exes.iter().enumerate() {
        let short = short_name(index.checked_add(1).ok_or("too many programs")?)?;
        let key = program_key(exe, &root)?;
        let bytes =
            std::fs::read(exe).map_err(|err| format!("reading {}: {err}", exe.display()))?;

        let files = build_record::project_files(&bytes).map_err(|err| format!("{key}: {err}"))?;
        write_files(&dir.join("extracted").join(&short), &files)
            .map_err(|err| format!("{key}: {err}"))?;
        let extracted_vbp = project_file_name(&files).map_err(|err| format!("{key}: {err}"))?;

        let original = vbp::select_project_file(exe, &projects)
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
    Ok(exported.len())
}

/// The four probe projects: a key, and the files of the project. The key
/// names what the probe tests.
pub(crate) fn probe_projects() -> Vec<(&'static str, ProjectFiles)> {
    let crlf = |lines: &[&str]| -> Vec<u8> {
        let mut out = String::new();
        for line in lines {
            out.push_str(line);
            out.push_str("\r\n");
        }
        out.into_bytes()
    };
    let module = |body: &[&str]| -> Vec<u8> {
        let mut lines = vec!["Attribute VB_Name = \"Module1\"", "Option Explicit", ""];
        lines.extend_from_slice(body);
        crlf(&lines)
    };
    let module_project = |name: &str, extra: &[&str]| -> Vec<u8> {
        let exe = format!("ExeName32=\"{name}.exe\"");
        let title = format!("Name=\"{name}\"");
        let mut lines = vec!["Type=Exe"];
        lines.extend_from_slice(extra);
        lines.extend_from_slice(&["Module=Module1; Module1.bas", "Startup=\"Sub Main\""]);
        lines.push(&exe);
        lines.push(&title);
        crlf(&lines)
    };
    let file = |name: &str, bytes: Vec<u8>| (name.to_owned(), bytes);

    vec![
        (
            "probe/ok",
            vec![
                file("Probe.vbp", module_project("ProbeOk", &[])),
                file("Module1.bas", module(&["Sub Main()", "End Sub"])),
            ],
        ),
        (
            "probe/syntax",
            vec![
                file("Probe.vbp", module_project("ProbeSyntax", &[])),
                file(
                    "Module1.bas",
                    module(&["Sub Main()", "    Dim x As Long", "    x =", "End Sub"]),
                ),
            ],
        ),
        (
            "probe/missing-ocx",
            vec![
                file(
                    "Probe.vbp",
                    module_project(
                        "ProbeMissingOcx",
                        &["Object={0D5F4C9A-6E5B-4F32-9A11-3C8F1D2B7E10}#1.0#0; NOPE.OCX"],
                    ),
                ),
                file("Module1.bas", module(&["Sub Main()", "End Sub"])),
            ],
        ),
        (
            "probe/bad-property",
            vec![
                file(
                    "Probe.vbp",
                    crlf(&[
                        "Type=Exe",
                        "Form=Form1.frm",
                        "Startup=\"Form1\"",
                        "ExeName32=\"ProbeBadProperty.exe\"",
                        "Name=\"ProbeBadProperty\"",
                    ]),
                ),
                file(
                    "Form1.frm",
                    crlf(&[
                        "VERSION 5.00",
                        "Begin VB.Form Form1",
                        "   Caption         =   \"Probe\"",
                        "   ClientHeight    =   3000",
                        "   ClientLeft      =   60",
                        "   ClientTop       =   345",
                        "   ClientWidth     =   4000",
                        "   BogusProperty   =   1",
                        "   ScaleHeight     =   3000",
                        "   ScaleWidth      =   4000",
                        "End",
                        "Attribute VB_Name = \"Form1\"",
                        "Attribute VB_GlobalNameSpace = False",
                        "Attribute VB_Creatable = False",
                        "Attribute VB_PredeclaredId = True",
                        "Attribute VB_Exposed = False",
                        "Option Explicit",
                    ]),
                ),
            ],
        ),
    ]
}

/// Writes the probe into `dir`, and gives the number of projects. Each
/// project is on both sides.
fn export_probe(dir: &Path) -> Result<usize, String> {
    prepare(dir)?;
    let mut exported = Vec::new();
    for (index, (key, files)) in probe_projects().into_iter().enumerate() {
        let short = short_name(index.checked_add(1).ok_or("too many probes")?)?;
        write_files(&dir.join("extracted").join(&short), &files)?;
        write_files(&dir.join("original").join(&short), &files)?;
        let vbp = project_file_name(&files)?;
        exported.push(Exported {
            short,
            key: key.to_owned(),
            original_vbp: vbp.clone(),
            extracted_vbp: vbp,
            files: build_record::files_hash(&files),
        });
    }
    write_lists(dir, &exported)?;
    Ok(exported.len())
}

/// Writes `manifest.txt` and `build.bat` into `dir`.
fn write_lists(dir: &Path, exported: &[Exported]) -> Result<(), String> {
    let manifest = dir.join("manifest.txt");
    std::fs::write(&manifest, render_manifest(exported))
        .map_err(|err| format!("writing {}: {err}", manifest.display()))?;
    let batch = dir.join("build.bat");
    std::fs::write(&batch, render_batch(exported))
        .map_err(|err| format!("writing {}: {err}", batch.display()))
}

/// Renders `manifest.txt`: a comment line, then one line for each program,
/// with the five fields apart by tabs.
pub(crate) fn render_manifest(programs: &[Exported]) -> String {
    let mut out =
        String::from("# short\tkey\toriginal project\textracted project\textracted files\n");
    for program in programs {
        let _ignored = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}",
            program.short, program.key, program.original_vbp, program.extracted_vbp, program.files
        );
    }
    out
}

/// Renders `build.bat`, with CRLF at the end of each line.
///
/// `cmd` on Windows XP finds a label only in a file with CRLF line ends. The
/// file does not use a block in parentheses, because a path can hold a
/// parenthesis. `start "" /wait` makes the file wait for `VB6.EXE`, which is
/// a windowed program, and it gives its exit code in `%ERRORLEVEL%`. Each
/// redirection comes first on its line, so that a digit before `>` is not
/// read as a handle number.
///
/// `BASE` is the directory of the file with no `\` at its end. `pushd`
/// gives a share a drive letter, and `%CD%` is then the root of that drive,
/// which ends with `\`. A path made from it would hold `\\`.
///
/// Before each build, the file deletes the log, the exit code and each
/// `.log` file beside the project, so that a second run of the file leaves
/// no old result. The corpus holds no `.log` file.
pub(crate) fn render_batch(programs: &[Exported]) -> String {
    let mut lines: Vec<String> = [
        "@echo off",
        "rem Builds each project that DeForm6 exported, with the Visual Basic 6 IDE.",
        "rem Run this file on the Windows host. The first argument, when you give",
        "rem one, is the path of VB6.EXE.",
        "setlocal",
    ]
    .iter()
    .map(|line| (*line).to_owned())
    .collect();
    lines.push(format!("set VB6={DEFAULT_VB6}"));
    lines.extend(
        [
            r#"if not "%~1"=="" set VB6=%~1"#,
            r#"if not exist "%VB6%" goto novb6"#,
            r#"pushd "%~dp0""#,
            "set BASE=%CD%",
            r#"if "%BASE:~-1%"=="\" set BASE=%BASE:~0,-1%"#,
            "if not exist logs mkdir logs",
            r">logs\environment.txt ver",
            r#">>logs\environment.txt dir "%VB6%""#,
        ]
        .iter()
        .map(|line| (*line).to_owned()),
    );
    for program in programs {
        lines.push(format!(
            r#"call :build {} original "original\{}\{}""#,
            program.short, program.short, program.original_vbp
        ));
        lines.push(format!(
            r#"call :build {} extracted "extracted\{}\{}""#,
            program.short, program.short, program.extracted_vbp
        ));
    }
    lines.extend(
        [
            "popd",
            "echo The builds are done. The logs are in the logs directory.",
            "exit /b 0",
            "",
            ":novb6",
            "echo VB6.EXE is not at %VB6%. Give its path as the first argument.",
            "exit /b 1",
            "",
            ":build",
            "echo Building %1 %2",
            r#"if exist "logs\%1-%2.txt" del "logs\%1-%2.txt""#,
            r#"if exist "logs\%1-%2.exit" del "logs\%1-%2.exit""#,
            r#"if exist "%~dp3*.log" del /q "%~dp3*.log""#,
        ]
        .iter()
        .map(|line| (*line).to_owned()),
    );
    lines.push(format!(
        r#"if not exist "{HOST_OUT_DIR}\%1\%2" mkdir "{HOST_OUT_DIR}\%1\%2""#
    ));
    lines.push(format!(
        r#"start "" /wait "%VB6%" /make "%BASE%\%~3" /outdir "{HOST_OUT_DIR}\%1\%2" /out "%BASE%\logs\%1-%2.txt""#
    ));
    lines.push(r#">"logs\%1-%2.exit" echo %ERRORLEVEL%"#.to_owned());
    lines.push("goto :eof".to_owned());

    let mut out = String::new();
    for line in lines {
        out.push_str(&line);
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test builds its own values; a wrong value must fail loudly"
)]
mod tests {
    use super::{
        Exported, check_batch_name, export, export_args, export_probe, probe_projects,
        render_batch, render_manifest, run_export, short_name,
    };
    use crate::build_record;
    use std::path::Path;

    fn program(short: &str, key: &str, vbp: &str) -> Exported {
        Exported {
            short: short.to_owned(),
            key: key.to_owned(),
            original_vbp: vbp.to_owned(),
            extracted_vbp: vbp.to_owned(),
            files: format!("sha256:{}", "0".repeat(64)),
        }
    }

    /// Each line of `build.bat` ends with CRLF, and no line ends with a bare
    /// LF, because `cmd` on Windows XP finds a label only then.
    #[test]
    fn each_line_of_the_batch_file_ends_with_crlf() {
        let text = render_batch(&[program("p01", "a/A.exe", "A.vbp")]);
        assert!(text.ends_with("\r\n"));
        let lf = text.matches('\n').count();
        let crlf = text.matches("\r\n").count();
        assert_eq!(lf, crlf);
        assert!(lf > 10);
    }

    /// The batch file builds the original side, then the extracted side, of
    /// each program, in the order of the programs, and each path is in
    /// quotes.
    #[test]
    fn the_batch_file_builds_each_side_of_each_program_in_order() {
        let text = render_batch(&[
            program("p01", "a/A.exe", "A.vbp"),
            program("p02", "b/B B.exe", "B B.vbp"),
        ]);
        let calls: Vec<&str> = text
            .split("\r\n")
            .filter(|line| line.starts_with("call :build"))
            .collect();
        assert_eq!(
            calls,
            [
                r#"call :build p01 original "original\p01\A.vbp""#,
                r#"call :build p01 extracted "extracted\p01\A.vbp""#,
                r#"call :build p02 original "original\p02\B B.vbp""#,
                r#"call :build p02 extracted "extracted\p02\B B.vbp""#,
            ]
        );
        assert!(text.contains(r#"/outdir "C:\deform6-out\%1\%2""#), "{text}");
        // No path is made from %CD%, which ends with a backslash at the root
        // of a drive.
        assert!(!text.contains(r"%CD%\"), "{text}");
        assert!(text.contains(r#"/make "%BASE%\%~3""#), "{text}");
        assert!(
            text.contains(r#">"logs\%1-%2.exit" echo %ERRORLEVEL%"#),
            "{text}"
        );
    }

    /// A name that `cmd` would expand, or that is not one plain file name,
    /// is refused. A name with a space and a dot is kept.
    #[test]
    fn a_name_that_cmd_would_change_is_refused() {
        for name in [
            "A%B.vbp", "A^B.vbp", "A&B.vbp", "A!B.vbp", "A\"B.vbp", r"A\B.vbp", "A/B.vbp", "",
        ] {
            assert!(check_batch_name(name).is_err(), "{name:?}");
        }
        assert!(check_batch_name("Part 3 - DIBs.vbp").is_ok());
    }

    /// A short name has two digits, and there is none for 0 or for more
    /// than 99 programs.
    #[test]
    fn a_short_name_has_two_digits() {
        assert_eq!(short_name(1).unwrap(), "p01");
        assert_eq!(short_name(44).unwrap(), "p44");
        assert!(short_name(0).is_err());
        assert!(short_name(100).is_err());
    }

    /// The manifest holds a comment line, then one line with five fields
    /// for each program.
    #[test]
    fn the_manifest_holds_one_line_for_each_program() {
        let text = render_manifest(&[program("p01", "a/A.exe", "A.vbp")]);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with('#'));
        assert_eq!(
            lines[1].split('\t').collect::<Vec<_>>(),
            [
                "p01",
                "a/A.exe",
                "A.vbp",
                "A.vbp",
                &format!("sha256:{}", "0".repeat(64))
            ]
        );
    }

    /// An export of the whole corpus into a new directory writes 44
    /// programs. Each extracted directory holds the files that DeForm6
    /// writes, byte for byte, and one project file. No original directory
    /// holds an executable. An export into a directory that is not empty is
    /// refused.
    #[test]
    fn an_export_writes_each_program_and_refuses_a_directory_that_is_not_empty() {
        let dir = std::env::temp_dir().join(format!("deform6-export-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);

        let count = export(&dir).unwrap();
        let manifest = std::fs::read_to_string(dir.join("manifest.txt")).unwrap();
        let batch = std::fs::read_to_string(dir.join("build.bat")).unwrap();
        let again = export(&dir);

        let root = build_record::corpus_root();
        let exes = build_record::executables(&root).unwrap();
        let mut checked = 0;
        for (line, exe) in manifest.lines().skip(1).zip(&exes) {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields[1], build_record::program_key(exe, &root).unwrap());
            let files = build_record::project_files(&std::fs::read(exe).unwrap()).unwrap();
            assert_eq!(fields[4], build_record::files_hash(&files));
            for (name, bytes) in &files {
                let on_disk = std::fs::read(dir.join("extracted").join(fields[0]).join(name));
                assert_eq!(on_disk.unwrap(), *bytes, "{} {name}", fields[0]);
            }
            assert!(
                dir.join("original")
                    .join(fields[0])
                    .join(fields[2])
                    .is_file()
            );
            checked += 1;
        }
        let executables_in_original = build_record::executables(&dir.join("original")).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(count, 44);
        assert_eq!(checked, 44);
        assert_eq!(batch.matches("call :build").count(), 88);
        assert!(
            executables_in_original.is_empty(),
            "{executables_in_original:?}"
        );
        assert!(again.unwrap_err().contains("is not empty"));
    }

    /// Each probe project is written with CRLF, and holds the one thing that
    /// it tests. Only the syntax probe holds a line with no expression after
    /// `=`, and only the component probe names a component.
    #[test]
    fn each_probe_project_holds_the_one_thing_that_it_tests() {
        let probes = probe_projects();
        let keys: Vec<&str> = probes.iter().map(|(key, _files)| *key).collect();
        assert_eq!(
            keys,
            [
                "probe/ok",
                "probe/syntax",
                "probe/missing-ocx",
                "probe/bad-property"
            ]
        );
        for (key, files) in &probes {
            for (name, bytes) in files {
                let text = String::from_utf8(bytes.clone()).unwrap();
                assert!(text.ends_with("\r\n"), "{key} {name}");
                assert_eq!(text.matches('\n').count(), text.matches("\r\n").count());
            }
            let all: String = files
                .iter()
                .map(|(_name, bytes)| String::from_utf8(bytes.clone()).unwrap())
                .collect();
            assert_eq!(all.contains("    x =\r\n"), *key == "probe/syntax", "{key}");
            assert_eq!(
                all.contains("Object="),
                *key == "probe/missing-ocx",
                "{key}"
            );
            assert_eq!(
                all.contains("BogusProperty"),
                *key == "probe/bad-property",
                "{key}"
            );
        }
    }

    /// A probe export writes four projects, each on both sides with the same
    /// bytes, and a batch file with eight builds.
    #[test]
    fn a_probe_export_writes_four_projects_on_both_sides() {
        let dir = std::env::temp_dir().join(format!("deform6-probe-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        let count = export_probe(&dir).unwrap();
        let manifest = std::fs::read_to_string(dir.join("manifest.txt")).unwrap();
        let batch = std::fs::read_to_string(dir.join("build.bat")).unwrap();
        let same = ["Probe.vbp", "Module1.bas"].iter().all(|name| {
            std::fs::read(dir.join("original/p01").join(name)).unwrap()
                == std::fs::read(dir.join("extracted/p01").join(name)).unwrap()
        });
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(count, 4);
        assert_eq!(manifest.lines().count(), 5);
        assert!(manifest.contains("\tprobe/bad-property\t"), "{manifest}");
        assert_eq!(batch.matches("call :build").count(), 8);
        assert!(same);
    }

    /// The command takes a directory, with `--probe` before it or not. A
    /// flag with no directory after it, a word that is not `--probe`, and
    /// no argument are refused. The refused calls go only to the parser, so
    /// that a fault here cannot write a directory.
    #[test]
    fn the_command_takes_a_directory_and_an_optional_probe_flag() {
        let owned = |words: &[&str]| -> Vec<String> {
            words.iter().map(|word| (*word).to_owned()).collect()
        };
        let plain = owned(&["out"]);
        assert_eq!(export_args(&plain).unwrap(), (false, Path::new("out")));
        let probe = owned(&["--probe", "out"]);
        assert_eq!(export_args(&probe).unwrap(), (true, Path::new("out")));
        for refused in [
            owned(&[]),
            owned(&["--probe"]),
            owned(&["--other", "out"]),
            owned(&["--probe", "--probe"]),
            owned(&["out", "more"]),
        ] {
            assert!(export_args(&refused).is_err(), "{refused:?}");
        }

        let dir = std::env::temp_dir().join(format!("deform6-probe-args-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        let status = run_export(&["--probe".to_owned(), dir.to_string_lossy().into_owned()]);
        let manifest = std::fs::read_to_string(dir.join("manifest.txt")).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(status, 0);
        assert_eq!(manifest.lines().count(), 5);
    }
}
