#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The exit codes, the locked output shape, and the read-only promise.
//!
//! `CARGO_BIN_EXE_deform6` is set by cargo for an integration test in this
//! package, because `crates/deform6-cli/Cargo.toml` names its bin target
//! `deform6`. It is the bin target name and not the package name.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

fn mandelbrot_path() -> PathBuf {
    corpus_root().join("vb6-code/Mandelbrot/Mandelbrot.exe")
}

/// Runs the built binary and gives its exit code, stdout and stderr.
fn run(args: &[&OsStr]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_deform6"))
        .args(args)
        .output()
        .unwrap();
    let code = out.status.code().expect("the process was not signalled");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (code, stdout, stderr)
}

/// Asserts DET-04's "one clear sentence", made mechanical: stdout is empty
/// and stderr holds exactly one non-empty line.
fn assert_one_line_refusal(code: i32, stdout: &str, stderr: &str) {
    assert!(
        stdout.is_empty(),
        "code {code} wrote to stdout as well as refusing: {stdout:?}"
    );
    let trimmed = stderr.trim_end_matches('\n');
    assert!(
        !trimmed.is_empty(),
        "code {code} left stderr empty, so the caller was told nothing"
    );
    assert!(
        !trimmed.contains('\n'),
        "code {code} wrote more than one line to stderr: {trimmed:?}"
    );
}

/// Snapshots every entry of `dir` as its path, its length and its
/// modification time.
///
/// This measures the thing `inspect` promises: that a run changes no file on
/// disk. A size alone is not a state, so the length and the modification
/// time are both taken, and the path makes a renamed or deleted entry visible
/// too.
fn dir_snapshot(dir: &Path) -> BTreeSet<(PathBuf, u64, SystemTime)> {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    let mut out = BTreeSet::new();
    for entry in entries {
        let entry = entry.unwrap();
        let meta = entry.metadata().unwrap();
        out.insert((entry.path(), meta.len(), meta.modified().unwrap()));
    }
    out
}

#[test]
fn inspecting_the_corpus_file_prints_the_eight_line_shape_and_exits_zero() {
    let path = mandelbrot_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let lines: Vec<&str> = stdout.lines().collect();
    let labels = [
        "File", "Format", "Runtime", "Header", "Project", "Title", "Mode", "Objects",
    ];
    assert_eq!(
        lines.len(),
        labels.len(),
        "stdout did not hold eight lines: {stdout:?}"
    );
    for (line, label) in lines.iter().zip(labels) {
        assert!(
            line.starts_with(label),
            "line {line:?} does not open with the label {label:?}: stdout was {stdout:?}"
        );
    }

    // The `.vbp` beside the executable declares
    // `Name="Mandelbrot_Fractal_Demo"` and `CompilationType=0`, which is
    // native.
    assert!(
        stdout.contains("Mandelbrot_Fractal_Demo"),
        "stdout was: {stdout:?}"
    );
    assert!(stdout.contains("native"), "stdout was: {stdout:?}");
}

#[test]
fn inspecting_the_corpus_file_changes_no_file_on_disk() {
    let path = mandelbrot_path();
    let dir = path
        .parent()
        .expect("the corpus file has a parent directory");
    let before = dir_snapshot(dir);

    let (code, _stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let after = dir_snapshot(dir);
    assert_eq!(
        before, after,
        "the directory changed after a read-only inspect"
    );
}

#[test]
fn a_short_text_file_exits_one_writes_nothing_to_stdout_and_one_line_to_stderr() {
    let path = std::env::temp_dir().join(format!(
        "deform6-cli-test-not-a-program-{}.txt",
        std::process::id()
    ));
    fs::write(&path, b"this is not a program\n").unwrap();

    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    fs::remove_file(&path).ok();

    assert_eq!(code, 1, "stderr was: {stderr}");
    assert_one_line_refusal(code, &stdout, &stderr);
}

#[test]
fn a_truncated_visual_basic_6_executable_exits_four() {
    let data = fs::read(mandelbrot_path()).unwrap();
    let half = &data[..data.len() / 2];
    let path = std::env::temp_dir().join(format!(
        "deform6-cli-test-truncated-{}.exe",
        std::process::id()
    ));
    fs::write(&path, half).unwrap();

    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    fs::remove_file(&path).ok();

    assert_eq!(code, 4, "stderr was: {stderr}");
    assert_one_line_refusal(code, &stdout, &stderr);
}

#[test]
fn a_path_that_does_not_exist_exits_five() {
    let path = std::env::temp_dir().join("deform6-cli-test-a-file-that-does-not-exist.exe");
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 5, "stderr was: {stderr}");
    assert_one_line_refusal(code, &stdout, &stderr);
}

/// A usage error must not collide with exit code 2, which is locked to "a PE
/// file, but it holds no Visual Basic runtime". `clap`'s own usage message is
/// several lines, which is its shape and not this project's, so this test
/// checks the exit code only.
#[test]
fn an_unknown_subcommand_exits_five_and_not_two() {
    let (code, stdout, stderr) = run(&[OsStr::new("frobnicate")]);
    assert_eq!(code, 5, "stderr was: {stderr}");
    assert_ne!(
        code, 2,
        "a usage error must not be told apart as \"no Visual Basic runtime\""
    );
    assert!(stdout.is_empty(), "stdout was: {stdout:?}");
}

#[test]
fn no_arguments_exits_five_and_not_two() {
    let (code, stdout, stderr) = run(&[]);
    assert_eq!(code, 5, "stderr was: {stderr}");
    assert_ne!(
        code, 2,
        "a usage error must not be told apart as \"no Visual Basic runtime\""
    );
    assert!(stdout.is_empty(), "stdout was: {stdout:?}");
}

#[test]
fn help_exits_zero() {
    let (code, _stdout, stderr) = run(&[OsStr::new("--help")]);
    assert_eq!(code, 0, "stderr was: {stderr}");
}

#[test]
fn version_exits_zero() {
    let (code, _stdout, stderr) = run(&[OsStr::new("--version")]);
    assert_eq!(code, 0, "stderr was: {stderr}");
}
