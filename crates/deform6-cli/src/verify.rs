//! `extract --verify-build`: builds the written project with the Visual
//! Basic 6 IDE that the user owns, and records what it gave.
//!
//! DeForm6 does not ship the IDE. It finds `VB6.EXE` on the user's machine
//! and runs `VB6.EXE /make`. The result goes into the JSON report as the
//! key `build`, in the shape that `crates/deform6/schema/report.schema.json`
//! gives.
//!
//! The rule that reads the result comes from runs on the author's Windows
//! XP host with VB6 SP6, which `crates/xtask/src/builds.rs` records:
//!
//! - `/make` gives exit code 0 and writes `Build of '<name>' succeeded.`
//!   when it builds the project.
//! - It gives exit code 1 when it does not, and writes no such line.
//! - A control class that it cannot load does not stop it. It puts a
//!   picture box in place of the control, builds the project, and writes a
//!   `.log` file beside the form. This is `built with load errors`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// The longest time that one build may take. A build of a corpus program
/// takes less than two seconds on the author's host. A dialog that stops
/// the IDE takes forever, so the run ends it.
pub(crate) const BUILD_TIMEOUT: Duration = Duration::from_secs(600);

/// The directory, inside the output directory, that receives the
/// executable and the text that `/out` writes.
pub(crate) const BUILD_DIR: &str = "build";

/// The name of the text file that `/out` writes into [`BUILD_DIR`].
const OUT_FILE: &str = "vb6-out.txt";

/// The environment variable that names `VB6.EXE` when `--compiler` is
/// absent.
pub(crate) const COMPILER_VARIABLE: &str = "DEFORM6_VB6";

/// What one build gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Exit code 0, the success line, and no load log.
    Built,
    /// Exit code 0 and the success line, with a load log beside a form.
    BuiltWithLoadErrors,
    /// Any other shape.
    Failed,
    /// The build ran longer than [`BUILD_TIMEOUT`], and the run ended it.
    TimedOut,
    /// No compiler was found, or it did not start.
    NotRun,
}

impl Outcome {
    /// The word that the report gives for this outcome. The schema names
    /// the same five words.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Built => "built",
            Self::BuiltWithLoadErrors => "built with load errors",
            Self::Failed => "failed",
            Self::TimedOut => "timed out",
            Self::NotRun => "not run",
        }
    }
}

/// The result of one build, as the report records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildResult {
    /// The `VB6.EXE` that ran, or `None` when none was found.
    pub(crate) compiler: Option<PathBuf>,
    /// What the build gave.
    pub(crate) outcome: Outcome,
    /// The exit code of `VB6.EXE`, or `None` when it did not end by itself.
    pub(crate) exit_code: Option<i32>,
    /// The lines of the `/out` text and of each load log, in that order.
    /// A line of a load log starts with the name of its file.
    pub(crate) messages: Vec<String>,
}

impl BuildResult {
    /// A result for a build that did not run, with the reason as its one
    /// message.
    pub(crate) fn not_run(reason: String) -> Self {
        Self {
            compiler: None,
            outcome: Outcome::NotRun,
            exit_code: None,
            messages: vec![reason],
        }
    }

    /// The value of the report key `build`.
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "compiler": self.compiler.as_ref().map(|path| path.display().to_string()),
            "outcome": self.outcome.as_str(),
            "exit_code": self.exit_code,
            "messages": self.messages,
        })
    }
}

/// Reads the shape of one finished build. `out_text` is what `/out` wrote,
/// and `load_logs` holds the name and the text of each `.log` file beside
/// the sources.
pub(crate) fn classify(exit_code: i32, out_text: &str, load_logs: &[(String, String)]) -> Outcome {
    let succeeded = out_text
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with("Build of '") && line.ends_with("' succeeded."));
    match (exit_code == 0, succeeded, load_logs.is_empty()) {
        (true, true, true) => Outcome::Built,
        (true, true, false) => Outcome::BuiltWithLoadErrors,
        _ => Outcome::Failed,
    }
}

/// Gives the lines of `out_text` and of each load log that are not empty.
pub(crate) fn messages(out_text: &str, load_logs: &[(String, String)]) -> Vec<String> {
    let mut out: Vec<String> = out_text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect();
    for (name, text) in load_logs {
        out.extend(
            text.lines()
                .map(str::trim_end)
                .filter(|line| !line.trim().is_empty())
                .map(|line| format!("{name}: {line}")),
        );
    }
    out
}

/// Finds `VB6.EXE`.
///
/// The order: `explicit` (the `--compiler` argument), then the variable
/// [`COMPILER_VARIABLE`], then `Microsoft Visual Studio\VB98\VB6.EXE` under
/// `ProgramFiles(x86)` and under `ProgramFiles`. `var` reads one
/// environment variable, so that a test can give its own environment.
///
/// # Errors
///
/// Gives a message that names the next action, when no candidate is a
/// file.
pub(crate) fn find_compiler(
    explicit: Option<&Path>,
    var: impl Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return if path.is_file() {
            Ok(path.to_path_buf())
        } else {
            Err(format!(
                "--compiler names {}, which is not a file; give the path of VB6.EXE",
                path.display()
            ))
        };
    }
    if let Some(value) = var(COMPILER_VARIABLE) {
        let path = PathBuf::from(value);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(format!(
                "{COMPILER_VARIABLE} names {}, which is not a file; give the path of VB6.EXE",
                path.display()
            ))
        };
    }
    for root in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(value) = var(root) {
            let path = PathBuf::from(value)
                .join("Microsoft Visual Studio")
                .join("VB98")
                .join("VB6.EXE");
            if path.is_file() {
                return Ok(path);
            }
        }
    }
    Err(format!(
        "found no VB6.EXE; install the Visual Basic 6 IDE, or give its path with \
         --compiler or {COMPILER_VARIABLE}"
    ))
}

/// Builds `vbp` with `compiler`, and reads the result.
///
/// The executable and the `/out` text go into `<project_dir>/build`. The
/// load logs are the `.log` files that the build leaves in `project_dir`.
/// A build that runs longer than `timeout` is ended.
pub(crate) fn run(
    compiler: &Path,
    vbp: &Path,
    project_dir: &Path,
    timeout: Duration,
) -> BuildResult {
    let build_dir = project_dir.join(BUILD_DIR);
    let out_path = build_dir.join(OUT_FILE);
    let failed_to_start = |reason: String| BuildResult {
        compiler: Some(compiler.to_path_buf()),
        ..BuildResult::not_run(reason)
    };
    if let Err(err) = std::fs::create_dir_all(&build_dir) {
        return failed_to_start(format!("could not create {}: {err}", build_dir.display()));
    }
    // A load log or an `/out` text of an earlier run would change the result
    // of this one.
    std::fs::remove_file(&out_path).ok();
    for (name, _) in load_logs(project_dir) {
        std::fs::remove_file(project_dir.join(name)).ok();
    }

    let mut child = match Command::new(compiler)
        .arg("/make")
        .arg(vbp)
        .arg("/outdir")
        .arg(&build_dir)
        .arg("/out")
        .arg(&out_path)
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return failed_to_start(format!("could not start {}: {err}", compiler.display()));
        }
    };

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(None) | Err(_) => {
                child.kill().ok();
                child.wait().ok();
                break None;
            }
        }
    };

    let out_text = std::fs::read(&out_path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    let logs = load_logs(project_dir);
    let messages = messages(&out_text, &logs);
    match status.and_then(|status| status.code()) {
        Some(code) => BuildResult {
            compiler: Some(compiler.to_path_buf()),
            outcome: classify(code, &out_text, &logs),
            exit_code: Some(code),
            messages,
        },
        None => BuildResult {
            compiler: Some(compiler.to_path_buf()),
            outcome: Outcome::TimedOut,
            exit_code: None,
            messages,
        },
    }
}

/// Gives the name and the text of each `.log` file in `dir`, sorted by
/// name. A file that cannot be read gives no entry.
fn load_logs(dir: &Path) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
        })
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            let bytes = std::fs::read(&path).ok()?;
            Some((name, String::from_utf8_lossy(&bytes).into_owned()))
        })
        .collect();
    out.sort();
    out
}

/// Gives `report_json` with the key `build` set to `result`.
///
/// # Errors
///
/// Gives a message when `report_json` is not a JSON object.
pub(crate) fn with_build(report_json: &str, result: &BuildResult) -> Result<String, String> {
    let mut value: serde_json::Value = serde_json::from_str(report_json)
        .map_err(|err| format!("the report is not JSON: {err}"))?;
    let Some(object) = value.as_object_mut() else {
        return Err("the report is not a JSON object".to_owned());
    };
    object.insert("build".to_owned(), result.to_json());
    serde_json::to_string_pretty(&value).map_err(|err| format!("could not write the report: {err}"))
}

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

    const SUCCESS: &str = "Build of 'Project1' succeeded.\r\n";

    fn log(name: &str, text: &str) -> (String, String) {
        (name.to_owned(), text.to_owned())
    }

    #[test]
    fn exit_zero_and_the_success_line_with_no_log_is_built() {
        assert_eq!(classify(0, SUCCESS, &[]), Outcome::Built);
    }

    #[test]
    fn a_load_log_beside_a_form_makes_a_build_with_load_errors() {
        let logs = [log(
            "Form1.log",
            "Line 12: Class X.Y of control z was not a loaded control class.",
        )];
        assert_eq!(classify(0, SUCCESS, &logs), Outcome::BuiltWithLoadErrors);
    }

    #[test]
    fn exit_one_or_no_success_line_is_failed() {
        assert_eq!(classify(1, SUCCESS, &[]), Outcome::Failed);
        assert_eq!(
            classify(0, "Compile error in File 'a.bas'\r\n", &[]),
            Outcome::Failed
        );
        assert_eq!(classify(0, "", &[]), Outcome::Failed);
        let logs = [log("Form1.log", "x")];
        assert_eq!(classify(1, "", &logs), Outcome::Failed);
    }

    #[test]
    fn the_messages_hold_the_out_lines_then_each_log_line_with_its_file() {
        let logs = [log("Form1.log", "Line 3: bad\r\n\r\nLine 4: worse\r\n")];
        assert_eq!(
            messages("first\r\n\r\nBuild of 'P' failed.\r\n", &logs),
            vec![
                "first".to_owned(),
                "Build of 'P' failed.".to_owned(),
                "Form1.log: Line 3: bad".to_owned(),
                "Form1.log: Line 4: worse".to_owned(),
            ]
        );
    }

    #[test]
    fn the_five_outcomes_give_the_five_words_of_the_schema() {
        let words: Vec<&str> = [
            Outcome::Built,
            Outcome::BuiltWithLoadErrors,
            Outcome::Failed,
            Outcome::TimedOut,
            Outcome::NotRun,
        ]
        .iter()
        .map(|outcome| outcome.as_str())
        .collect();
        assert_eq!(
            words,
            [
                "built",
                "built with load errors",
                "failed",
                "timed out",
                "not run"
            ]
        );
    }

    /// A fresh directory for one test; the caller removes it.
    fn fresh_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "deform6-cli-test-verify-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Puts an empty `VB6.EXE` at `<root>/Microsoft Visual Studio/VB98`.
    fn plant_vb6(root: &Path) -> PathBuf {
        let dir = root.join("Microsoft Visual Studio").join("VB98");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("VB6.EXE");
        std::fs::write(&path, b"").unwrap();
        path
    }

    #[test]
    fn the_search_takes_the_explicit_path_first_then_the_variable_then_program_files() {
        let dir = fresh_dir("search");
        let explicit = dir.join("mine.exe");
        std::fs::write(&explicit, b"").unwrap();
        let named = dir.join("named.exe");
        std::fs::write(&named, b"").unwrap();
        let x86 = dir.join("x86");
        let planted_x86 = plant_vb6(&x86);
        let plain = dir.join("plain");
        let planted_plain = plant_vb6(&plain);

        let all = |name: &str| match name {
            COMPILER_VARIABLE => Some(named.clone().into_os_string()),
            "ProgramFiles(x86)" => Some(x86.clone().into_os_string()),
            "ProgramFiles" => Some(plain.clone().into_os_string()),
            _ => None,
        };
        assert_eq!(find_compiler(Some(&explicit), all), Ok(explicit.clone()));
        assert_eq!(find_compiler(None, all), Ok(named.clone()));

        let no_variable = |name: &str| {
            if name == COMPILER_VARIABLE {
                None
            } else {
                all(name)
            }
        };
        assert_eq!(find_compiler(None, no_variable), Ok(planted_x86));

        let plain_only = |name: &str| {
            if name == "ProgramFiles" {
                all(name)
            } else {
                None
            }
        };
        assert_eq!(find_compiler(None, plain_only), Ok(planted_plain));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_compiler_gives_a_message_that_names_the_next_action() {
        let err = find_compiler(None, |_| None).unwrap_err();
        assert!(err.contains("--compiler"), "{err}");
        assert!(err.contains(COMPILER_VARIABLE), "{err}");

        let missing = std::env::temp_dir().join("deform6-cli-test-verify-no-such-vb6.exe");
        let err = find_compiler(Some(&missing), |_| None).unwrap_err();
        assert!(err.contains("not a file"), "{err}");
    }

    #[test]
    fn the_build_key_goes_into_the_report_and_a_non_object_is_refused() {
        let result = BuildResult {
            compiler: Some(PathBuf::from("VB6.EXE")),
            outcome: Outcome::BuiltWithLoadErrors,
            exit_code: Some(0),
            messages: vec!["m".to_owned()],
        };
        let text = with_build("{\"items\": [], \"defects\": [], \"limits\": []}", &result).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["build"]["outcome"], "built with load errors");
        assert_eq!(value["build"]["exit_code"], 0);
        assert_eq!(value["build"]["compiler"], "VB6.EXE");
        assert_eq!(value["items"], serde_json::json!([]));

        assert!(with_build("[1]", &result).is_err());
        assert!(with_build("not json", &result).is_err());
    }
}
