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
//!   writes `logs\`. VB6 writes each executable into `deform6-out` on the
//!   system drive of the host, so no executable comes back into this
//!   directory.
//! - `sendlogs.bat` sends each log of the build out through the serial port
//!   `COM1`, for a host that cannot write into this directory. The author's
//!   XP host is such a host: the export goes in on a CD image, and the logs
//!   come out through the serial port, which UTM joins to a terminal device
//!   on this machine. See [`render_sendlogs`] for the format.
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

/// The default place of `VB6.EXE` on the host, from the host's own
/// `%ProgramFiles%`. The first argument of `build.bat` replaces it.
///
/// The XP host of the author has Windows on `E:`, and no `C:\Program Files`
/// at all. A fixed drive letter is therefore wrong on at least one host.
const DEFAULT_VB6: &str = r"%ProgramFiles%\Microsoft Visual Studio\VB98\VB6.EXE";

/// The directory on the host that receives the executables that VB6 builds.
/// It is on the host's system drive, `%SystemDrive%`, so no executable
/// reaches the shared directory.
const HOST_OUT_DIR: &str = r"%SystemDrive%\deform6-out";

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
        .map_err(|err| format!("writing {}: {err}", batch.display()))?;
    let sender = dir.join("sendlogs.bat");
    std::fs::write(&sender, render_sendlogs())
        .map_err(|err| format!("writing {}: {err}", sender.display()))
}

/// Renders `sendlogs.bat`, with CRLF at the end of each line.
///
/// The file sends each file in `logs\`, then each `.log` file under
/// `original\` and `extracted\`, out through `COM1`. Each file goes out as
/// a line `===FILE <size> <path>===`, the bytes of the file as `copy /b`
/// sends them, and a line `===EOF===` on a line of its own. The size lets
/// the reader find a byte that the serial line lost or added. The file then sends
/// `===OUT===`, the list of the executables that VB6 built, and `===END===`.
/// `mode` turns off each handshake, because a terminal device on this
/// machine gives no handshake signal.
pub(crate) fn render_sendlogs() -> String {
    let lines = [
        "@echo off",
        "rem Sends each log of the build out through COM1, for the other machine",
        "rem to read.",
        "mode COM1: baud=115200 parity=n data=8 stop=1 to=off xon=off odsr=off \
         octs=off dtr=on rts=on idsr=off >nul",
        r#"pushd "%~dp0""#,
        r#"for %%f in (logs\*.*) do call :send "%%f""#,
        r#"for /r original %%f in (*.log) do call :send "%%f""#,
        r#"for /r extracted %%f in (*.log) do call :send "%%f""#,
        ">COM1 echo ===OUT===",
        ">COM1 dir /s /b %SystemDrive%\\deform6-out\\*.exe",
        ">COM1 echo ===END===",
        "popd",
        "exit /b 0",
        "",
        ":send",
        ">COM1 echo ===FILE %~z1 %~1===",
        r#"copy /b "%~1" COM1 >nul"#,
        ">COM1 echo.",
        ">COM1 echo ===EOF===",
        "goto :eof",
    ];
    let mut out = String::new();
    for line in lines {
        out.push_str(line);
        out.push_str("\r\n");
    }
    out
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

/// Runs `import-builds`.
pub(crate) fn run_import(args: &[String]) -> i32 {
    let (capture, dir) = match import_args(args) {
        Ok(read) => read,
        Err(message) => {
            eprintln!("xtask: {message}");
            return 1;
        }
    };
    if let Some(capture) = capture {
        let unpacked = std::fs::read(capture)
            .map_err(|err| format!("reading {}: {err}", capture.display()))
            .and_then(|bytes| unpack_capture(&bytes, dir));
        match unpacked {
            Ok(count) => println!("xtask: unpacked {count} files into {}", dir.display()),
            Err(message) => {
                eprintln!("xtask: {message}");
                return 1;
            }
        }
    }
    match import_and_write(dir) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            println!(
                "xtask: wrote {}",
                build_record::builds_toml_path().display()
            );
            0
        }
        Err(message) => {
            eprintln!("xtask: {message}");
            1
        }
    }
}

/// Reads the arguments of `import-builds`: `[--capture <file>] <dir>`. Gives
/// the capture of `sendlogs.bat`, when there is one, and the directory. A
/// word that starts with `-` in the place of a path is refused.
pub(crate) fn import_args(args: &[String]) -> Result<(Option<&Path>, &Path), String> {
    match args {
        [dir] if !dir.starts_with('-') => Ok((None, Path::new(dir))),
        [flag, file, dir]
            if flag == "--capture" && !file.starts_with('-') && !dir.starts_with('-') =>
        {
            Ok((Some(Path::new(file)), Path::new(dir)))
        }
        _ => Err("usage: cargo run -p xtask -- import-builds [--capture <file>] <dir>".to_owned()),
    }
}

/// Gives the place under the export of a file that `sendlogs.bat` sent.
///
/// A path `logs\<name>` goes to `logs/<name>`. Any other path must run
/// through `original\pNN\` or `extracted\pNN\`, and goes to the same place
/// under the export. A component `..` or `.`, or an empty one, is refused.
pub(crate) fn capture_place(path: &str) -> Result<std::path::PathBuf, String> {
    let parts: Vec<&str> = path.split('\\').collect();
    let start = if parts
        .first()
        .is_some_and(|first| first.eq_ignore_ascii_case("logs"))
    {
        0
    } else {
        parts
            .windows(2)
            .position(|pair| match pair {
                [side, short] => {
                    (side.eq_ignore_ascii_case("original")
                        || side.eq_ignore_ascii_case("extracted"))
                        && short.len() == 3
                        && short.starts_with('p')
                        && short.bytes().skip(1).all(|byte| byte.is_ascii_digit())
                }
                _ => false,
            })
            .ok_or_else(|| format!("the path {path:?} runs through no project directory"))?
    };
    let mut place = std::path::PathBuf::new();
    for (index, part) in parts.iter().enumerate().skip(start) {
        if part.is_empty() || *part == "." || *part == ".." {
            return Err(format!("the path {path:?} holds the component {part:?}"));
        }
        // The side is written in lower case, as the export writes it.
        if index == start {
            place.push(part.to_ascii_lowercase());
        } else {
            place.push(part);
        }
    }
    if place.components().count() < 2 {
        return Err(format!("the path {path:?} names no file"));
    }
    Ok(place)
}

/// Unpacks a capture of `sendlogs.bat` into `dir`, and gives the number of
/// files.
///
/// Each file is a line `===FILE <size> <path>===`, exactly `<size>` bytes,
/// and a line `===EOF===`. The capture must hold `===OUT===` and end with
/// `===END===`, so that an import never reads half of a transfer. A body
/// that is not `<size>` bytes long is refused, because the serial line lost
/// or added a byte. The export must hold no `logs` directory yet, so that no
/// old log mixes with the new ones.
pub(crate) fn unpack_capture(capture: &[u8], dir: &Path) -> Result<usize, String> {
    let find = |needle: &[u8], from: usize| -> Option<usize> {
        capture
            .get(from..)?
            .windows(needle.len())
            .position(|window| window == needle)
            .and_then(|at| at.checked_add(from))
    };
    if dir.join("logs").exists() {
        return Err(format!(
            "{} already holds a logs directory, so the capture is not unpacked",
            dir.display()
        ));
    }
    let out = find(b"===OUT===", 0).ok_or("the capture holds no ===OUT=== line")?;
    if find(b"===END===", out).is_none() {
        return Err("the capture holds no ===END=== line, so the transfer is not whole".to_owned());
    }

    let mut files: Vec<(std::path::PathBuf, &[u8])> = Vec::new();
    let mut at = 0_usize;
    while let Some(start) = find(b"===FILE ", at).filter(|start| *start < out) {
        let head_from = start.checked_add(8).ok_or("the capture is too long")?;
        let head_end = find(b"===\r\n", head_from).ok_or("a ===FILE line has no end")?;
        let head = capture
            .get(head_from..head_end)
            .ok_or("a ===FILE line has no text")?;
        let head = String::from_utf8_lossy(head);
        let (size, path) = head
            .split_once(' ')
            .ok_or_else(|| format!("the line ===FILE {head}=== holds no size"))?;
        let size: usize = size
            .parse()
            .map_err(|_ignored| format!("the line ===FILE {head}=== holds no size"))?;
        let body_from = head_end.checked_add(5).ok_or("the capture is too long")?;
        let body_end = body_from
            .checked_add(size)
            .ok_or("the capture is too long")?;
        let tail_end = body_end.checked_add(13).ok_or("the capture is too long")?;
        if capture.get(body_end..tail_end) != Some(&b"\r\n===EOF===\r\n"[..]) {
            return Err(format!(
                "the body of {path} is not {size} bytes long: the serial line lost or added a byte"
            ));
        }
        let body = capture
            .get(body_from..body_end)
            .ok_or("a body runs past the capture")?;
        files.push((capture_place(path)?, body));
        at = tail_end;
    }

    for (place, body) in &files {
        let target = dir.join(place);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("making {}: {err}", parent.display()))?;
        }
        std::fs::write(&target, body)
            .map_err(|err| format!("writing {}: {err}", target.display()))?;
    }
    Ok(files.len())
}

/// Reads the export in `dir`, checks that it holds the 44 corpus programs,
/// and writes `tests/builds.toml`. Gives one line for each program whose
/// result moved since the old file.
fn import_and_write(dir: &Path) -> Result<Vec<String>, String> {
    let record = import_record(dir)?;
    let root = corpus_root();
    let mut keys = Vec::new();
    for exe in executables(&root)? {
        keys.push(program_key(&exe, &root)?);
    }
    check_corpus_keys(&record, &keys)
        .map_err(|err| format!("the export in {}: {err}", dir.display()))?;

    let path = build_record::builds_toml_path();
    let changes = match std::fs::read_to_string(&path) {
        Ok(old_text) => match build_record::parse(&old_text) {
            Ok(old) => changes(&old, &record),
            Err(err) => vec![format!(
                "the old file does not parse, so no change is listed: {err}"
            )],
        },
        Err(_) => vec!["there is no old file, so no change is listed".to_owned()],
    };
    let rendered = build_record::render(&record);
    if build_record::parse(&rendered)? != record {
        return Err("the rendered record does not parse back to the same record".to_owned());
    }
    std::fs::write(&path, &rendered).map_err(|err| format!("writing {}: {err}", path.display()))?;
    Ok(changes)
}

/// Refuses a record that does not name exactly the corpus programs `keys`.
/// A probe is never written into `tests/builds.toml`.
pub(crate) fn check_corpus_keys(
    record: &build_record::BuildRecord,
    keys: &[String],
) -> Result<(), String> {
    let imported: Vec<&String> = record.programs.keys().collect();
    if imported != keys.iter().collect::<Vec<_>>() {
        return Err(format!(
            "it does not hold the {} corpus programs, so it is not imported. A probe is never \
             imported",
            keys.len()
        ));
    }
    Ok(())
}

/// Gives one line for each program whose `files` or whose results differ
/// between `old` and `new`.
fn changes(old: &build_record::BuildRecord, new: &build_record::BuildRecord) -> Vec<String> {
    let mut lines = Vec::new();
    for (key, program) in &new.programs {
        let describe = |p: &build_record::ProgramBuild| {
            format!(
                "original {}, extracted {}",
                p.original.outcome.word(),
                p.extracted.outcome.word()
            )
        };
        match old.programs.get(key) {
            None => lines.push(format!("{key}: new: {}", describe(program))),
            Some(held) if held != program => {
                lines.push(format!(
                    "{key}: {} -> {}",
                    describe(held),
                    describe(program)
                ));
            }
            Some(_) => {}
        }
    }
    for key in old.programs.keys() {
        if !new.programs.contains_key(key) {
            lines.push(format!("{key}: removed"));
        }
    }
    lines
}

/// Reads `manifest.txt`, which [`render_manifest`] writes.
pub(crate) fn parse_manifest(text: &str) -> Result<Vec<Exported>, String> {
    let mut out = Vec::new();
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        let [short, key, original_vbp, extracted_vbp, files] = fields.as_slice() else {
            return Err(format!(
                "the manifest line {line:?} does not hold five fields"
            ));
        };
        out.push(Exported {
            short: (*short).to_owned(),
            key: (*key).to_owned(),
            original_vbp: (*original_vbp).to_owned(),
            extracted_vbp: (*extracted_vbp).to_owned(),
            files: (*files).to_owned(),
        });
    }
    Ok(out)
}

/// Gives the Windows line and the `VB6.EXE` line of `logs\environment.txt`.
/// The first is the line that `ver` writes. The second is the line that
/// `dir` writes for `VB6.EXE`, with each run of spaces made one space.
pub(crate) fn parse_environment(text: &str) -> Result<(String, String), String> {
    let windows = text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("Microsoft Windows"))
        .ok_or("logs/environment.txt holds no Windows version line")?;
    let vb6 = text
        .lines()
        .map(str::trim)
        .find(|line| line.to_ascii_uppercase().ends_with(" VB6.EXE"))
        .ok_or("logs/environment.txt holds no line for VB6.EXE")?;
    Ok((
        windows.to_owned(),
        vb6.split_whitespace().collect::<Vec<_>>().join(" "),
    ))
}

/// Cuts each path in `line` that runs through the project directory
/// `\<side>\<short>\` to start after that directory. A path starts after the
/// last quote before it, because VB6 puts each path between single quotes.
pub(crate) fn cut_paths(line: &str, side: &str, short: &str) -> String {
    let marker = format!("\\{side}\\{short}\\").to_ascii_lowercase();
    let mut out = line.to_owned();
    while let Some(at) = out.to_ascii_lowercase().find(&marker) {
        let start = out
            .get(..at)
            .and_then(|before| before.rfind('\''))
            .and_then(|quote| quote.checked_add(1))
            .unwrap_or(0);
        let Some(end) = at.checked_add(marker.len()) else {
            break;
        };
        let (Some(head), Some(tail)) = (out.get(..start), out.get(end..)) else {
            break;
        };
        out = format!("{head}{tail}");
    }
    out
}

/// Reads a file of VB6 as text. VB6 writes ANSI text. A byte that is not
/// UTF-8 becomes U+FFFD, and no line is dropped for it.
fn read_text(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("reading {}: {err}", path.display())),
    }
}

/// Gives each `.log` file under `dir`, as a path relative to `dir` with `/`,
/// with its bytes as text, in the order of the paths.
fn load_logs(dir: &Path) -> Result<Vec<(String, String)>, String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, String)>) -> Result<(), String> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Ok(());
        };
        for entry in entries {
            let path = entry
                .map_err(|err| format!("reading an entry of {}: {err}", dir.display()))?
                .path();
            if path.is_dir() {
                walk(base, &path, out)?;
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
            {
                let relative = path
                    .strip_prefix(base)
                    .map_err(|err| format!("{}: {err}", path.display()))?
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                let text = read_text(&path)?.unwrap_or_default();
                out.push((relative, text));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out)?;
    out.sort();
    Ok(out)
}

/// Reads the result of one side of one program.
///
/// The rule comes from runs on the XP host with VB6 SP6. `VB6.EXE /make`
/// gives exit code 0 and writes the line `Build of '<name>' succeeded.` when
/// it builds the project. It gives exit code 1 when it does not, and writes
/// no such line. A load error also writes a `.log` file beside the source
/// file. The probe showed these shapes.
///
/// VB6 can go past some load errors. For a control whose class it cannot
/// load, it puts a picture box in place of the control, and it builds the
/// project. It then gives exit code 0, the success line and a `.log` file.
/// The second build run of the corpus showed this shape, and the side is
/// `built with load errors`.
///
/// A side with no `.exit` file did not run. Any other shape is refused,
/// because no run showed it.
pub(crate) fn read_side(
    dir: &Path,
    program: &Exported,
    side: &str,
) -> Result<build_record::Side, String> {
    let name = format!("{}-{side}", program.short);
    let logs = dir.join("logs");
    let Some(exit_text) = read_text(&logs.join(format!("{name}.exit")))? else {
        return Ok(build_record::Side {
            outcome: build_record::Outcome::NotRun,
            messages: Vec::new(),
        });
    };
    let exit: i64 = exit_text.trim().parse().map_err(|_ignored| {
        format!("logs/{name}.exit holds {exit_text:?}, which is not a number")
    })?;
    let out = read_text(&logs.join(format!("{name}.txt")))?.unwrap_or_default();
    let lines: Vec<String> = out
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(|line| cut_paths(line, side, &program.short))
        .collect();
    let succeeded = lines
        .iter()
        .any(|line| line.starts_with("Build of '") && line.ends_with("' succeeded."));
    let load_logs = load_logs(&dir.join(side).join(&program.short))?;

    let outcome = match (exit == 0, succeeded, load_logs.is_empty()) {
        (true, true, true) => {
            return Ok(build_record::Side {
                outcome: build_record::Outcome::Built,
                messages: Vec::new(),
            });
        }
        (true, true, false) => build_record::Outcome::BuiltWithLoadErrors,
        (false, false, _) => build_record::Outcome::Failed,
        _ => {
            return Err(format!(
                "{} {side}: exit code {exit}, a success line {}, and {} load log files do not \
                 agree with the rule that the runs on the host measured",
                program.key,
                if succeeded { "present" } else { "absent" },
                load_logs.len()
            ));
        }
    };
    let mut messages = lines;
    for (path, text) in load_logs {
        for line in text
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.is_empty())
        {
            messages.push(format!("{path}: {}", cut_paths(line, side, &program.short)));
        }
    }
    Ok(build_record::Side { outcome, messages })
}

/// Reads the whole export in `dir` into a record.
pub(crate) fn import_record(dir: &Path) -> Result<build_record::BuildRecord, String> {
    let manifest = read_text(&dir.join("manifest.txt"))?
        .ok_or_else(|| format!("{} holds no manifest.txt", dir.display()))?;
    let environment = read_text(&dir.join("logs").join("environment.txt"))?
        .ok_or_else(|| format!("{} holds no logs/environment.txt", dir.display()))?;
    let (windows, vb6) = parse_environment(&environment)?;
    let mut record = build_record::BuildRecord {
        windows,
        vb6,
        programs: std::collections::BTreeMap::new(),
    };
    for program in parse_manifest(&manifest)? {
        let built = build_record::ProgramBuild {
            files: program.files.clone(),
            original: read_side(dir, &program, "original")?,
            extracted: read_side(dir, &program, "extracted")?,
        };
        record.programs.insert(program.key.clone(), built);
    }
    Ok(record)
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
        Exported, capture_place, check_batch_name, check_corpus_keys, cut_paths, export,
        export_args, export_probe, import_args, import_record, parse_environment, parse_manifest,
        probe_projects, read_side, render_batch, render_manifest, render_sendlogs, run_export,
        short_name, unpack_capture,
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
        assert!(
            text.contains(r#"/outdir "%SystemDrive%\deform6-out\%1\%2""#),
            "{text}"
        );
        assert!(
            text.contains(r"set VB6=%ProgramFiles%\Microsoft Visual Studio\VB98\VB6.EXE"),
            "{text}"
        );
        // No drive letter is fixed.
        assert!(!text.contains(r"C:\"), "{text}");
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
        let sender = std::fs::read_to_string(dir.join("sendlogs.bat")).unwrap();
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
        assert_eq!(sender, super::render_sendlogs());
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

    // --- The importer. The texts below have the shapes that the runs on the
    // XP host with VB6 SP6 gave: the probe, and the second build run of the
    // corpus. Each test builds its own export directory in the temporary
    // directory.

    /// A new, empty directory in the temporary directory.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("deform6-import-{name}-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("logs")).unwrap();
        dir
    }

    /// Writes the exit code, the `/out` text and the load logs of one side.
    fn write_side(
        dir: &Path,
        short: &str,
        side: &str,
        exit: Option<&str>,
        out: &str,
        logs: &[(&str, &str)],
    ) {
        if let Some(exit) = exit {
            std::fs::write(dir.join("logs").join(format!("{short}-{side}.exit")), exit).unwrap();
        }
        std::fs::write(dir.join("logs").join(format!("{short}-{side}.txt")), out).unwrap();
        let project = dir.join(side).join(short);
        std::fs::create_dir_all(&project).unwrap();
        for (name, text) in logs {
            std::fs::write(project.join(name), text).unwrap();
        }
    }

    const BLANKS: &str = "\r\n\r\n\r\n";

    /// Reads one side of the program `short`, in a directory that holds only
    /// that side.
    fn side_of(
        name: &str,
        short: &str,
        exit: Option<&str>,
        out: &str,
        logs: &[(&str, &str)],
    ) -> Result<build_record::Side, String> {
        let dir = scratch(name);
        write_side(&dir, short, "extracted", exit, out, logs);
        let side = read_side(&dir, &program(short, "k/K.exe", "K.vbp"), "extracted");
        std::fs::remove_dir_all(&dir).unwrap();
        side
    }

    /// A build that VB6 finished gives exit code 0 and a success line, and
    /// is `built` with no message.
    #[test]
    fn a_build_that_vb6_finished_is_built() {
        let side = side_of(
            "built",
            "p01",
            Some("0\r\n"),
            &format!("{BLANKS}Build of 'ProbeOk.exe' succeeded.\r\n"),
            &[],
        )
        .unwrap();
        assert_eq!(side.outcome, build_record::Outcome::Built);
        assert!(side.messages.is_empty());
    }

    /// A syntax error gives exit code 1 and two lines. Each path starts at
    /// the project directory.
    #[test]
    fn a_syntax_error_is_failed_with_its_lines() {
        let out = format!(
            "{BLANKS}Compile Error in File 'E:\\deform6\\probe\\extracted\\p02\\Module1.bas', \
             Line 4 : Syntax error\r\nBuild of 'ProbeSyntax.exe' failed.\r\n"
        );
        let side = side_of("syntax", "p02", Some("1\r\n"), &out, &[]).unwrap();
        assert_eq!(side.outcome, build_record::Outcome::Failed);
        assert_eq!(
            side.messages,
            [
                "Compile Error in File 'Module1.bas', Line 4 : Syntax error",
                "Build of 'ProbeSyntax.exe' failed.",
            ]
        );
    }

    /// A component that is not on the host gives exit code 1 and one line,
    /// with no line about the build.
    #[test]
    fn a_missing_component_is_failed_with_its_line() {
        let out = format!(
            "{BLANKS}'E:\\deform6\\probe\\extracted\\p03\\NOPE.OCX' could not be loaded\r\n"
        );
        let side = side_of("ocx", "p03", Some("1\r\n"), &out, &[]).unwrap();
        assert_eq!(side.outcome, build_record::Outcome::Failed);
        assert_eq!(side.messages, ["'NOPE.OCX' could not be loaded"]);
    }

    /// A load error gives exit code 1, three lines, and a `.log` file beside
    /// the form. The lines of the `.log` file come after the three lines,
    /// each after the name of its file.
    #[test]
    fn a_load_error_is_failed_with_the_lines_of_its_log_file() {
        let out = format!(
            "{BLANKS}Errors during load. Refer to \
             'E:\\deform6\\probe\\extracted\\p04\\Form1.log' for details\r\n\
             'E:\\deform6\\probe\\extracted\\p04\\Form1.frm' could not be loaded.\r\n\
             Build of 'ProbeBadProperty.exe' failed.\r\n"
        );
        let log = (
            "Form1.log",
            "Line 8: The property name BogusProperty in Form1 is invalid.\r\n",
        );
        let side = side_of("load", "p04", Some("1\r\n"), &out, &[log]).unwrap();
        assert_eq!(side.outcome, build_record::Outcome::Failed);
        assert_eq!(
            side.messages,
            [
                "Errors during load. Refer to 'Form1.log' for details",
                "'Form1.frm' could not be loaded.",
                "Build of 'ProbeBadProperty.exe' failed.",
                "Form1.log: Line 8: The property name BogusProperty in Form1 is invalid.",
            ]
        );
    }

    /// A control class that VB6 cannot load gives exit code 0, two lines and
    /// a `.log` file beside the form. VB6 puts a picture box in place of the
    /// control and builds the project. The lines of the `.log` file come
    /// after the two lines, each after the name of its file.
    #[test]
    fn a_build_after_errors_during_load_is_built_with_load_errors() {
        let out = format!(
            "{BLANKS}Errors during load. Refer to \
             'E:\\deform6\\c2a\\extracted\\p06\\Form1.log' for details\r\n\
             Build of 'TFTPClient.exe' succeeded.\r\n"
        );
        let log = (
            "Form1.log",
            "Line 17: Class VB.Control of control WskClient was not a loaded control class.\r\n",
        );
        let side = side_of("loadbuilt", "p06", Some("0\r\n"), &out, &[log]).unwrap();
        assert_eq!(side.outcome, build_record::Outcome::BuiltWithLoadErrors);
        assert_eq!(
            side.messages,
            [
                "Errors during load. Refer to 'Form1.log' for details",
                "Build of 'TFTPClient.exe' succeeded.",
                "Form1.log: Line 17: Class VB.Control of control WskClient was not a loaded \
                 control class.",
            ]
        );
    }

    /// A side with no exit code did not run.
    #[test]
    fn a_side_with_no_exit_code_did_not_run() {
        let side = side_of("notrun", "p05", None, "", &[]).unwrap();
        assert_eq!(side.outcome, build_record::Outcome::NotRun);
        assert!(side.messages.is_empty());
    }

    /// A shape that no run showed is refused: exit code 0 with no success
    /// line, exit code 1 with one, with a load log or without one, and an
    /// exit code that is not a number.
    #[test]
    fn a_shape_that_no_run_showed_is_refused() {
        let success = format!("{BLANKS}Build of 'A.exe' succeeded.\r\n");
        let failed = "Build of 'A.exe' failed.\r\n";
        let log = [("Form1.log", "Line 1: x\r\n")];
        assert!(side_of("r1", "p01", Some("0"), failed, &[]).is_err());
        assert!(side_of("r2", "p01", Some("1"), &success, &[]).is_err());
        assert!(side_of("r3", "p01", Some("1"), &success, &log).is_err());
        assert!(side_of("r4", "p01", Some("zero"), &success, &[]).is_err());
    }

    /// Each path through the project directory starts after that directory,
    /// in any case. A path through another program's directory stays.
    #[test]
    fn a_path_is_cut_at_its_own_project_directory() {
        assert_eq!(
            cut_paths(
                r"'E:\x\EXTRACTED\P02\A.frm' and 'e:\y\extracted\p02\B.frx'",
                "extracted",
                "p02"
            ),
            "'A.frm' and 'B.frx'"
        );
        assert_eq!(
            cut_paths(r"'E:\x\extracted\p03\A.frm'", "extracted", "p02"),
            r"'E:\x\extracted\p03\A.frm'"
        );
        assert_eq!(cut_paths("no path here", "original", "p01"), "no path here");
    }

    /// The environment gives the line of `ver`, and the line of `dir` for
    /// `VB6.EXE` with each run of spaces made one.
    #[test]
    fn the_environment_gives_the_windows_line_and_the_vb6_line() {
        let text = "\r\nMicrosoft Windows XP [Version 5.1.2600]\r\n Volume in drive E has no \
                    label.\r\n\r\n Directory of E:\\Program Files\\Microsoft Visual \
                    Studio\\VB98\r\n\r\n02/23/2004  12:00 AM         1,895,424 VB6.EXE\r\n      \
                    1 File(s)      1,895,424 bytes\r\n";
        assert_eq!(
            parse_environment(text).unwrap(),
            (
                "Microsoft Windows XP [Version 5.1.2600]".to_owned(),
                "02/23/2004 12:00 AM 1,895,424 VB6.EXE".to_owned()
            )
        );
        assert!(parse_environment("no version\r\n").is_err());
    }

    /// The manifest reads back what `render_manifest` writes, and a line with
    /// four fields is refused.
    #[test]
    fn the_manifest_reads_back_what_the_export_writes() {
        let programs = [
            program("p01", "a/A.exe", "A.vbp"),
            program("p02", "b/Part 3 - B/B.exe", "B B.vbp"),
        ];
        assert_eq!(
            parse_manifest(&render_manifest(&programs)).unwrap(),
            programs
        );
        assert!(parse_manifest("p01\ta/A.exe\tA.vbp\tA.vbp\n").is_err());
    }

    /// An import reads each side of each program in the manifest, and takes
    /// the hash of the files from the manifest.
    #[test]
    fn an_import_reads_each_side_of_each_program_in_the_manifest() {
        let dir = scratch("record");
        let programs = [
            program("p01", "a/A.exe", "A.vbp"),
            program("p02", "b/B.exe", "B.vbp"),
        ];
        std::fs::write(dir.join("manifest.txt"), render_manifest(&programs)).unwrap();
        std::fs::write(
            dir.join("logs/environment.txt"),
            "Microsoft Windows XP [Version 5.1.2600]\r\n02/23/2004  12:00 AM  1,895,424 \
             VB6.EXE\r\n",
        )
        .unwrap();
        let success = format!("{BLANKS}Build of 'A.exe' succeeded.\r\n");
        let missing = "'x\\extracted\\p02\\N.OCX' could not be loaded\r\n";
        write_side(&dir, "p01", "original", Some("0"), &success, &[]);
        write_side(&dir, "p01", "extracted", Some("0"), &success, &[]);
        write_side(&dir, "p02", "original", Some("0"), &success, &[]);
        write_side(&dir, "p02", "extracted", Some("1"), missing, &[]);

        let record = import_record(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        let record = record.unwrap();

        assert_eq!(record.windows, "Microsoft Windows XP [Version 5.1.2600]");
        assert_eq!(record.programs.len(), 2);
        let b = &record.programs["b/B.exe"];
        assert_eq!(b.files, programs[1].files);
        assert_eq!(b.original.outcome, build_record::Outcome::Built);
        assert_eq!(b.extracted.outcome, build_record::Outcome::Failed);
        assert_eq!(b.extracted.messages, ["'N.OCX' could not be loaded"]);
    }

    /// Only a record of exactly the corpus programs is imported.
    #[test]
    fn only_a_record_of_the_corpus_programs_is_imported() {
        let side = build_record::Side {
            outcome: build_record::Outcome::Built,
            messages: Vec::new(),
        };
        let mut record = build_record::BuildRecord::default();
        for key in ["a/A.exe", "b/B.exe"] {
            record.programs.insert(
                key.to_owned(),
                build_record::ProgramBuild {
                    files: format!("sha256:{}", "0".repeat(64)),
                    original: side.clone(),
                    extracted: side.clone(),
                },
            );
        }
        let keys = ["a/A.exe".to_owned(), "b/B.exe".to_owned()];
        assert!(check_corpus_keys(&record, &keys).is_ok());
        assert!(check_corpus_keys(&record, &keys[..1]).is_err());
        let probe = ["probe/ok".to_owned(), "probe/syntax".to_owned()];
        assert!(check_corpus_keys(&record, &probe).is_err());
    }

    /// The log sender has CRLF at the end of each line. It sends each log,
    /// and each `.log` file of both sides, between the markers that the
    /// importer reads, with `copy /b`, so no byte changes on the way.
    #[test]
    fn the_log_sender_sends_each_log_between_the_markers() {
        let text = render_sendlogs();
        assert!(text.ends_with("\r\n"));
        assert_eq!(text.matches('\n').count(), text.matches("\r\n").count());
        for line in [
            r#"for %%f in (logs\*.*) do call :send "%%f""#,
            r#"for /r original %%f in (*.log) do call :send "%%f""#,
            r#"for /r extracted %%f in (*.log) do call :send "%%f""#,
            ">COM1 echo ===FILE %~z1 %~1===",
            r#"copy /b "%~1" COM1 >nul"#,
            ">COM1 echo ===EOF===",
            ">COM1 echo ===OUT===",
            ">COM1 echo ===END===",
        ] {
            assert!(
                text.split("\r\n").any(|held| held == line),
                "{line}\n{text}"
            );
        }
        assert!(text.contains("octs=off"), "{text}");
    }

    /// Builds a capture as `sendlogs.bat` sends it: each file with its size
    /// and its path, then the list of executables, then the end line.
    fn capture_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (path, body) in files {
            out.extend_from_slice(format!("===FILE {} {path}===\r\n", body.len()).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\r\n===EOF===\r\n");
        }
        out.extend_from_slice(
            b"===OUT===\r\nE:\\deform6-out\\p01\\original\\A.exe\r\n===END===\r\n",
        );
        out
    }

    /// A capture unpacks each file to its place in the export, with the same
    /// bytes, and an absolute path keeps only its part from the side on.
    #[test]
    fn a_capture_unpacks_each_file_to_its_place_with_the_same_bytes() {
        let dir = scratch("unpack");
        std::fs::remove_dir_all(dir.join("logs")).unwrap();
        let capture = capture_of(&[
            ("logs\\p01-original.exit", b"0\r\n"),
            (
                "logs\\p01-original.txt",
                b"\r\n\r\nBuild of 'A.exe' succeeded.\r\n",
            ),
            (
                "E:\\deform6\\c1\\Extracted\\p01\\Forms\\Form1.log",
                b"Line 8: x\r\n\xff",
            ),
        ]);
        let count = unpack_capture(&capture, &dir);
        let exit = std::fs::read(dir.join("logs/p01-original.exit"));
        let log = std::fs::read(dir.join("extracted/p01/Forms/Form1.log"));
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(count.unwrap(), 3);
        assert_eq!(exit.unwrap(), b"0\r\n");
        assert_eq!(log.unwrap(), b"Line 8: x\r\n\xff");
    }

    /// A body that is one byte short or one byte long is refused, and so is
    /// a capture with no end line.
    #[test]
    fn a_capture_with_a_wrong_size_or_no_end_is_refused() {
        let dir = scratch("sizes");
        std::fs::remove_dir_all(dir.join("logs")).unwrap();
        let good = capture_of(&[("logs\\p01-original.exit", b"0\r\n")]);
        let short =
            String::from_utf8(good.clone())
                .unwrap()
                .replacen("===FILE 3 ", "===FILE 2 ", 1);
        let long = String::from_utf8(good.clone())
            .unwrap()
            .replacen("===FILE 3 ", "===FILE 4 ", 1);
        let cut = &good[..good.len() - 12];
        let results = [
            unpack_capture(short.as_bytes(), &dir),
            unpack_capture(long.as_bytes(), &dir),
            unpack_capture(cut, &dir),
        ];
        let wrote = dir.join("logs").exists();
        std::fs::remove_dir_all(&dir).unwrap();

        for result in &results {
            assert!(result.is_err(), "{result:?}");
        }
        assert!(
            results[0]
                .as_ref()
                .unwrap_err()
                .contains("lost or added a byte")
        );
        assert!(results[2].as_ref().unwrap_err().contains("===END==="));
        assert!(!wrote, "a refused capture writes no file");
    }

    /// A capture is refused when the export already holds logs, so that no
    /// old log mixes with the new ones.
    #[test]
    fn a_capture_into_an_export_with_logs_is_refused() {
        let dir = scratch("again");
        let result = unpack_capture(&capture_of(&[]), &dir);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(
            result
                .unwrap_err()
                .contains("already holds a logs directory")
        );
    }

    /// A path goes to `logs/` or to its place under a side. A path through
    /// no project directory, or with `..`, is refused.
    #[test]
    fn a_path_of_the_capture_goes_to_its_place_under_the_export() {
        assert_eq!(
            capture_place(r"logs\p02-extracted.txt").unwrap(),
            Path::new("logs/p02-extracted.txt")
        );
        assert_eq!(
            capture_place(r"E:\deform6\c1\ORIGINAL\p44\Form1.log").unwrap(),
            Path::new("original/p44/Form1.log")
        );
        for refused in [
            r"E:\deform6\c1\Form1.log",
            r"E:\x\original\p01\..\..\escape.log",
            r"E:\x\original\p01\",
            r"E:\x\original\pAB\Form1.log",
            r"logs\",
        ] {
            assert!(capture_place(refused).is_err(), "{refused}");
        }
    }

    /// The command takes a directory, with `--capture <file>` before it or
    /// not, and refuses any other shape.
    #[test]
    fn the_import_takes_a_directory_and_an_optional_capture() {
        let owned = |words: &[&str]| -> Vec<String> {
            words.iter().map(|word| (*word).to_owned()).collect()
        };
        assert_eq!(import_args(&owned(&["d"])).unwrap(), (None, Path::new("d")));
        assert_eq!(
            import_args(&owned(&["--capture", "c.bin", "d"])).unwrap(),
            (Some(Path::new("c.bin")), Path::new("d"))
        );
        for refused in [
            owned(&[]),
            owned(&["--capture"]),
            owned(&["--capture", "c.bin"]),
            owned(&["--other", "c.bin", "d"]),
            owned(&["--capture", "--x", "d"]),
        ] {
            assert!(import_args(&refused).is_err(), "{refused:?}");
        }
    }
}
