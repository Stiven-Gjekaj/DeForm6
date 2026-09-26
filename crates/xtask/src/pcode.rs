//! `cargo run -p xtask -- export-pcode [--probe] <dir>` writes each corpus
//! project in a form that the Visual Basic 6 IDE builds as P-code on a
//! Windows host.
//!
//! Each corpus program is native code: each corpus project file holds the
//! line `CompilationType=0`. Phase 9 builds the same projects as P-code, with
//! only that line changed: its value becomes [`PCODE_VALUE`].
//!
//! # The export directory
//!
//! - `pcode/pNN/` holds a copy of the directory of the corpus project file
//!   for program `NN`, with no executable, and with the one line changed in
//!   the project file. VB6 writes its executable into the same directory, so
//!   that the executable comes back with the logs.
//! - `manifest.txt` names each project: its short name, its key, its project
//!   file, and the hash of its files.
//! - `build.bat` builds each project with `VB6.EXE /make`, and writes
//!   `logs\`.
//! - `sendpcode.bat` sends each log, each `.log` file, each executable and
//!   each `.bin` file out through the serial port `COM1`, in the frame of
//!   [`builds::render_sender`].
//!
//! The short names, `p01` to `p44`, are the names of `export-builds`, in the
//! order of the keys, so that `p06` names the same program in both.
//!
//! # The probe
//!
//! `export-pcode --probe <dir>` writes the project `probe/ok` of the build
//! probe three times, with `CompilationType` 0, -1 and 1.
//! `pcode/p01/bytes.bin` holds each byte value from 0 to 255 four times. The
//! serial line must bring it back unchanged. This code writes each file, so
//! no third party byte is in it.

use std::fmt::Write as _;
use std::path::Path;

use crate::build_record;
use crate::builds;
use crate::ratios::differential::support::vbp;

/// The name of the side in the export, in the logs and in the capture.
const SIDE: &str = "pcode";

/// The line that each corpus project file holds, which makes VB6 build
/// native code.
const NATIVE_LINE: &str = "CompilationType=0";

/// The value of `CompilationType` that the export writes, which makes VB6
/// build P-code.
///
/// Measured on the author's XP host with VB6 SP6, by the probe: the values
/// `-1` and `1` both give an executable whose `lpNativeCode` is 0, and `0`
/// gives native code. The IDE writes `-1`: of the 19 project files in the
/// VB6 install of that host that hold the key, 18 hold `0`, 1 holds `-1`,
/// and none holds `1`.
const PCODE_VALUE: &str = "-1";

/// The values of `CompilationType` that the probe builds, each with the key
/// of its project and the name of its executable.
const PROBE_VALUES: [(&str, &str, &str); 3] = [
    ("0", "probe/compilation-type-0", "PcodeProbe0"),
    ("-1", "probe/compilation-type-minus-1", "PcodeProbeMinus1"),
    ("1", "probe/compilation-type-1", "PcodeProbe1"),
];

/// The number of bytes of `bytes.bin`: each byte value four times.
const PROBE_BYTES_LEN: usize = 1024;

/// One project as the export wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PcodeProgram {
    /// `p01` to `p99`.
    pub(crate) short: String,
    /// The key of the program.
    pub(crate) key: String,
    /// The file name of the project file in `pcode/<short>/`.
    pub(crate) vbp: String,
    /// The hash of the files of the project, from
    /// [`build_record::files_hash`].
    pub(crate) source: String,
}

/// Runs `export-pcode`.
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
                "xtask: exported {count} projects to {}. Run build.bat there on the Windows \
                 host, then send the results with sendpcode.bat.",
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

/// Reads the arguments of `export-pcode`: `[--probe] <dir>`. Gives whether
/// the probe is asked for, and the directory. A directory that starts with
/// `-` is refused, so that a flag is never read as the name of a directory.
pub(crate) fn export_args(args: &[String]) -> Result<(bool, &Path), String> {
    match args {
        [dir] if !dir.starts_with('-') => Ok((false, Path::new(dir))),
        [flag, dir] if flag == "--probe" && !dir.starts_with('-') => Ok((true, Path::new(dir))),
        _ => Err("usage: cargo run -p xtask -- export-pcode [--probe] <dir>".to_owned()),
    }
}

/// Gives the project file `vbp` with its line `CompilationType=0` changed to
/// `CompilationType=<value>`.
///
/// The text must hold that line exactly once, with CRLF after it. Any other
/// text is refused, because the build would then not be the corpus project
/// with one line changed.
pub(crate) fn with_compilation_type(vbp: &[u8], value: &str) -> Result<Vec<u8>, String> {
    let text: String = vbp.iter().copied().map(char::from).collect();
    let found = text
        .split("\r\n")
        .filter(|line| line.starts_with("CompilationType="))
        .count();
    let native = text
        .split("\r\n")
        .filter(|line| *line == NATIVE_LINE)
        .count();
    if found != 1 || native != 1 {
        return Err(format!(
            "the project file holds {found} CompilationType lines, and {native} of them is \
             {NATIVE_LINE}; exactly one line {NATIVE_LINE} is needed"
        ));
    }
    let changed: Vec<String> = text
        .split("\r\n")
        .map(|line| {
            if line == NATIVE_LINE {
                format!("CompilationType={value}")
            } else {
                line.to_owned()
            }
        })
        .collect();
    // Each character came from one byte, so each maps back to one byte.
    changed
        .join("\r\n")
        .chars()
        .map(|c| {
            u8::try_from(u32::from(c)).map_err(|_ignored| "a character left a byte".to_owned())
        })
        .collect()
}

/// Gives the bytes of `bytes.bin`: each byte value from 0 to 255, four
/// times, in order.
pub(crate) fn probe_bytes() -> Vec<u8> {
    (0..=u8::MAX).cycle().take(PROBE_BYTES_LEN).collect()
}

/// Gives the three probe projects: each key, with the files of the project.
///
/// Each one is the project `probe/ok` of the build probe, with its own name,
/// and with the line `CompilationType=0` changed to the value of the probe.
pub(crate) fn probe_projects() -> Result<Vec<(&'static str, builds::ProjectFiles)>, String> {
    let (_key, ok) = builds::probe_projects()
        .into_iter()
        .find(|(key, _files)| *key == "probe/ok")
        .ok_or("the build probe holds no project probe/ok")?;
    let module = ok
        .iter()
        .find(|(name, _bytes)| name == "Module1.bas")
        .map(|(_name, bytes)| bytes.clone())
        .ok_or("the project probe/ok holds no Module1.bas")?;

    let mut out = Vec::new();
    for (value, key, exe) in PROBE_VALUES {
        let native = builds::crlf_text(&[
            "Type=Exe",
            "Module=Module1; Module1.bas",
            "Startup=\"Sub Main\"",
            &format!("ExeName32=\"{exe}.exe\""),
            &format!("Name=\"{exe}\""),
            NATIVE_LINE,
        ]);
        let vbp = with_compilation_type(native.as_bytes(), value)?;
        out.push((
            key,
            vec![
                ("Probe.vbp".to_owned(), vbp),
                ("Module1.bas".to_owned(), module.clone()),
            ],
        ));
    }
    Ok(out)
}

/// Writes the whole export into `dir`, and gives the number of programs.
///
/// The `source` of each program is the hash of the files of the corpus
/// project, from [`build_record::source_files`], before the one line
/// changes.
fn export(dir: &Path) -> Result<usize, String> {
    builds::prepare(dir)?;
    let root = build_record::corpus_root();
    let projects = vbp::project_files();
    let exes = build_record::executables(&root)?;
    if exes.len() != build_record::EXPECTED_PROGRAM_COUNT {
        return Err(format!(
            "found {} corpus programs, and the record holds {}",
            exes.len(),
            build_record::EXPECTED_PROGRAM_COUNT
        ));
    }

    let mut exported = Vec::new();
    for (index, exe) in exes.iter().enumerate() {
        let short = builds::short_name(index.checked_add(1).ok_or("too many programs")?)?;
        let key = build_record::program_key(exe, &root)?;
        let original = vbp::select_project_file(exe, &projects)
            .map_err(|err| format!("{key}: no project file: {err:?}"))?;
        let original_dir = original
            .parent()
            .ok_or_else(|| format!("{key}: the project file has no directory"))?;
        let vbp_name = original
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| format!("{key}: the project file has no name"))?;
        builds::check_batch_name(&vbp_name).map_err(|err| format!("{key}: {err}"))?;
        let source = build_record::files_hash(
            &build_record::source_files(original_dir).map_err(|err| format!("{key}: {err}"))?,
        );

        let project = dir.join(SIDE).join(&short);
        builds::copy_without_executables(original_dir, &project)
            .map_err(|err| format!("{key}: {err}"))?;
        let copied = project.join(&vbp_name);
        let bytes =
            std::fs::read(&copied).map_err(|err| format!("reading {}: {err}", copied.display()))?;
        let changed =
            with_compilation_type(&bytes, PCODE_VALUE).map_err(|err| format!("{key}: {err}"))?;
        std::fs::write(&copied, changed)
            .map_err(|err| format!("writing {}: {err}", copied.display()))?;

        exported.push(PcodeProgram {
            short,
            key,
            vbp: vbp_name,
            source,
        });
    }

    write_lists(dir, &exported)?;
    Ok(exported.len())
}

/// Writes the probe into `dir`, and gives the number of projects.
fn export_probe(dir: &Path) -> Result<usize, String> {
    builds::prepare(dir)?;
    let mut exported = Vec::new();
    for (index, (key, files)) in probe_projects()?.into_iter().enumerate() {
        let short = builds::short_name(index.checked_add(1).ok_or("too many probes")?)?;
        let project = dir.join(SIDE).join(&short);
        builds::write_files(&project, &files)?;
        if index == 0 {
            let path = project.join("bytes.bin");
            std::fs::write(&path, probe_bytes())
                .map_err(|err| format!("writing {}: {err}", path.display()))?;
        }
        exported.push(PcodeProgram {
            short,
            key: key.to_owned(),
            vbp: "Probe.vbp".to_owned(),
            source: build_record::files_hash(&files),
        });
    }
    write_lists(dir, &exported)?;
    Ok(exported.len())
}

/// Writes `manifest.txt`, `build.bat` and `sendpcode.bat` into `dir`.
fn write_lists(dir: &Path, exported: &[PcodeProgram]) -> Result<(), String> {
    for (name, text) in [
        ("manifest.txt", render_manifest(exported)),
        ("build.bat", render_batch(exported)),
        ("sendpcode.bat", render_sender()),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, text).map_err(|err| format!("writing {}: {err}", path.display()))?;
    }
    Ok(())
}

/// Renders `manifest.txt`: a comment line, then one line for each project,
/// with the four fields apart by tabs.
pub(crate) fn render_manifest(programs: &[PcodeProgram]) -> String {
    let mut out = String::from("# short\tkey\tproject\tsource\n");
    for program in programs {
        let _ignored = writeln!(
            out,
            "{}\t{}\t{}\t{}",
            program.short, program.key, program.vbp, program.source
        );
    }
    out
}

/// Renders `build.bat`, with CRLF at the end of each line.
///
/// The file builds each project in `pcode\pNN\`, with the head of
/// [`builds::batch_head`] and the routine of [`builds::batch_tail`]. VB6
/// writes the executable into the directory of the project. Before each
/// build, the routine deletes each old executable there, so that an
/// executable in the directory is always the result of the last build.
pub(crate) fn render_batch(programs: &[PcodeProgram]) -> String {
    let mut lines = builds::batch_head(&[
        "rem Builds each project that DeForm6 exported as P-code, with the Visual",
        "rem Basic 6 IDE. Run this file on the Windows host. The first argument, when",
        "rem you give one, is the path of VB6.EXE.",
    ]);
    for program in programs {
        lines.push(format!(
            r#"call :build {} {SIDE} "{SIDE}\{}\{}""#,
            program.short, program.short, program.vbp
        ));
    }
    lines.extend(builds::batch_tail(
        r"%BASE%\%2\%1",
        &[r#"if exist "%BASE%\%2\%1\*.exe" del /q "%BASE%\%2\%1\*.exe""#.to_owned()],
    ));
    builds::crlf_text(&lines)
}

/// Renders `sendpcode.bat`, with CRLF at the end of each line.
///
/// The file sends each file in `logs\`, then each `.log` file, each
/// executable and each `.bin` file under `pcode\`, in the frame of
/// [`builds::render_sender`]. After `===OUT===` it lists the executables
/// under `pcode\`.
pub(crate) fn render_sender() -> String {
    builds::render_sender(
        &[
            "rem Sends each log and each executable of the build out through COM1,",
            "rem for the other machine to read.",
        ],
        &[
            r#"for %%f in (logs\*.*) do call :send "%%f""#,
            r#"for /r pcode %%f in (*.log) do call :send "%%f""#,
            r#"for /r pcode %%f in (*.exe) do call :send "%%f""#,
            r#"for /r pcode %%f in (*.bin) do call :send "%%f""#,
        ],
        r"pcode\*.exe",
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{
        PcodeProgram, export, export_args, export_probe, probe_bytes, probe_projects, render_batch,
        render_manifest, render_sender, with_compilation_type,
    };
    use crate::build_record;
    use crate::ratios::differential::support::vbp;

    fn program(short: &str, vbp: &str) -> PcodeProgram {
        PcodeProgram {
            short: short.to_owned(),
            key: format!("k/{short}.exe"),
            vbp: vbp.to_owned(),
            source: format!("sha256:{}", "0".repeat(64)),
        }
    }

    /// The one line `CompilationType=0` changes, and every other byte stays.
    #[test]
    fn the_native_line_changes_and_nothing_else() {
        let vbp = b"Type=Exe\r\nCompilationType=0\r\nOptimizationType=0\r\n";
        assert_eq!(
            with_compilation_type(vbp, "-1").unwrap(),
            b"Type=Exe\r\nCompilationType=-1\r\nOptimizationType=0\r\n"
        );
    }

    /// A project file with no line, with two lines, or with a line of
    /// another value, is refused.
    #[test]
    fn a_project_file_without_exactly_one_native_line_is_refused() {
        for vbp in [
            &b"Type=Exe\r\n"[..],
            b"CompilationType=0\r\nCompilationType=0\r\n",
            b"CompilationType=-1\r\n",
            b"CompilationType=0\r\nCompilationType=-1\r\n",
            b"CompilationType=0\n",
        ] {
            assert!(
                with_compilation_type(vbp, "-1").is_err(),
                "{}",
                String::from_utf8_lossy(vbp)
            );
        }
    }

    /// `bytes.bin` holds each byte value four times, in order.
    #[test]
    fn the_probe_bytes_hold_each_value_four_times() {
        let bytes = probe_bytes();
        assert_eq!(bytes.len(), 1024);
        for value in 0..=u8::MAX {
            assert_eq!(bytes.iter().filter(|b| **b == value).count(), 4, "{value}");
        }
        assert_eq!(&bytes[254..258], &[254, 255, 0, 1]);
    }

    /// The three probe projects hold the values 0, -1 and 1, each in its own
    /// project file, with its own name.
    #[test]
    fn the_probe_builds_the_three_values() {
        let projects = probe_projects().unwrap();
        let lines: Vec<String> = projects
            .iter()
            .map(|(_key, files)| {
                let (_name, vbp) = files.iter().find(|(name, _)| name == "Probe.vbp").unwrap();
                String::from_utf8_lossy(vbp)
                    .split("\r\n")
                    .filter(|line| {
                        line.starts_with("CompilationType=") || line.starts_with("Name=")
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        assert_eq!(
            lines,
            [
                "Name=\"PcodeProbe0\" CompilationType=0",
                "Name=\"PcodeProbeMinus1\" CompilationType=-1",
                "Name=\"PcodeProbe1\" CompilationType=1",
            ]
        );
    }

    /// Each line of `build.bat` ends with CRLF. The file builds each project
    /// in order, into the directory of the project, and deletes an old
    /// executable there first.
    #[test]
    fn the_batch_file_builds_each_project_into_its_own_directory() {
        let text = render_batch(&[program("p01", "A.vbp"), program("p02", "B B.vbp")]);
        assert!(text.ends_with("\r\n"));
        assert_eq!(text.matches('\n').count(), text.matches("\r\n").count());
        let calls: Vec<&str> = text
            .split("\r\n")
            .filter(|line| line.starts_with("call :build"))
            .collect();
        assert_eq!(
            calls,
            [
                r#"call :build p01 pcode "pcode\p01\A.vbp""#,
                r#"call :build p02 pcode "pcode\p02\B B.vbp""#,
            ]
        );
        let lines: Vec<&str> = text.split("\r\n").collect();
        let delete = lines
            .iter()
            .position(|line| {
                *line == r#"if exist "%BASE%\%2\%1\*.exe" del /q "%BASE%\%2\%1\*.exe""#
            })
            .expect("the old executable is deleted");
        let start = lines
            .iter()
            .position(|line| line.starts_with(r#"start "" /wait"#))
            .expect("the build starts");
        assert!(delete < start, "{text}");
        assert!(text.contains(r#"/outdir "%BASE%\%2\%1""#), "{text}");
    }

    /// The sender sends each log, each `.log` file, each executable and each
    /// `.bin` file under `pcode\`, in the frame that the importer reads.
    #[test]
    fn the_sender_sends_each_executable_between_the_markers() {
        let text = render_sender();
        assert!(text.ends_with("\r\n"));
        for line in [
            r#"for %%f in (logs\*.*) do call :send "%%f""#,
            r#"for /r pcode %%f in (*.log) do call :send "%%f""#,
            r#"for /r pcode %%f in (*.exe) do call :send "%%f""#,
            r#"for /r pcode %%f in (*.bin) do call :send "%%f""#,
            ">COM1 echo ===FILE %~z1 %~1===",
            r#"copy /b "%~1" COM1 >nul"#,
            ">COM1 echo ===OUT===",
            r">COM1 dir /s /b pcode\*.exe",
            ">COM1 echo ===END===",
        ] {
            assert!(
                text.split("\r\n").any(|held| held == line),
                "{line}\n{text}"
            );
        }
    }

    /// The manifest holds a comment line, then one line with four fields for
    /// each project.
    #[test]
    fn the_manifest_holds_one_line_for_each_project() {
        let text = render_manifest(&[program("p01", "A.vbp")]);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with('#'));
        assert_eq!(lines[1].split('\t').count(), 4);
    }

    /// The probe writes three projects, the file `bytes.bin` beside the
    /// first, and the three lists.
    #[test]
    fn the_probe_export_writes_three_projects_and_the_bytes() {
        let dir = std::env::temp_dir().join(format!("deform6-pcode-probe-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        assert_eq!(export_probe(&dir).unwrap(), 3);
        for short in ["p01", "p02", "p03"] {
            assert!(
                dir.join("pcode").join(short).join("Probe.vbp").is_file(),
                "{short}"
            );
        }
        assert_eq!(
            std::fs::read(dir.join("pcode/p01/bytes.bin")).unwrap(),
            probe_bytes()
        );
        for name in ["manifest.txt", "build.bat", "sendpcode.bat"] {
            assert!(dir.join(name).is_file(), "{name}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The arguments are `[--probe] <dir>`. A flag with no directory is
    /// refused.
    #[test]
    fn the_arguments_are_a_directory_and_the_probe_flag() {
        let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            export_args(&args(&["out"])).unwrap(),
            (false, std::path::Path::new("out"))
        );
        assert_eq!(
            export_args(&args(&["--probe", "out"])).unwrap(),
            (true, std::path::Path::new("out"))
        );
        assert!(export_args(&args(&["--probe"])).is_err());
        assert!(export_args(&args(&["--probe", "--x"])).is_err());
        assert!(export_args(&args(&[])).is_err());
    }

    /// An export of the whole corpus writes 44 projects. Each one holds the
    /// P-code line once and the native line not at all, and no executable.
    /// The source hash of each one is the hash of the corpus files, before
    /// the change. An export into a directory that is not empty is refused.
    #[test]
    fn an_export_writes_each_corpus_project_as_p_code() {
        let dir = std::env::temp_dir().join(format!("deform6-export-pcode-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);

        let count = export(&dir).unwrap();
        let manifest = std::fs::read_to_string(dir.join("manifest.txt")).unwrap();
        let batch = std::fs::read_to_string(dir.join("build.bat")).unwrap();
        let sender = std::fs::read_to_string(dir.join("sendpcode.bat")).unwrap();
        let again = export(&dir);

        let root = build_record::corpus_root();
        let projects = vbp::project_files();
        let exes = build_record::executables(&root).unwrap();
        let mut checked = 0;
        for (line, exe) in manifest.lines().skip(1).zip(&exes) {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields[1], build_record::program_key(exe, &root).unwrap());
            let original = vbp::select_project_file(exe, &projects).unwrap();
            let corpus_files = build_record::source_files(original.parent().unwrap()).unwrap();
            assert_eq!(fields[3], build_record::files_hash(&corpus_files), "{line}");

            let copied = std::fs::read(dir.join("pcode").join(fields[0]).join(fields[2])).unwrap();
            let text = String::from_utf8_lossy(&copied);
            let lines: Vec<&str> = text.split("\r\n").collect();
            // The line that the probe measured, as a literal, so that a
            // change of the value in the code fails here.
            let pcode = lines.iter().filter(|l| **l == "CompilationType=-1").count();
            let native = lines.iter().filter(|l| **l == "CompilationType=0").count();
            assert_eq!((pcode, native), (1, 0), "{line}");
            checked += 1;
        }
        let executables = build_record::executables(&dir.join("pcode")).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(count, 44);
        assert_eq!(checked, 44);
        assert_eq!(batch.matches("call :build").count(), 44);
        assert_eq!(sender, render_sender());
        assert!(executables.is_empty(), "{executables:?}");
        assert!(again.unwrap_err().contains("is not empty"));
    }
}
