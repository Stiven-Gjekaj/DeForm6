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

fn grayscale_path() -> PathBuf {
    corpus_root().join("vb6-code/Grayscale-effect/Grayscale.exe")
}

fn map_editor_path() -> PathBuf {
    corpus_root().join("vb6-code/Map-editor-2D/Map Editor.exe")
}

/// The corpus's smallest program: one form, one procedure, and no external
/// `Declare` table at all.
fn lock_work_station_path() -> PathBuf {
    corpus_root().join("public-domain/LockWorkStation/LockWorkStation.exe")
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

/// The eight-line head phase 1 locked prints unchanged, compared line for
/// line so a change to it is a failure and not a surprise. Phase 2 appends
/// sections below it; it does not reshape it.
#[test]
fn inspecting_the_corpus_file_prints_the_locked_eight_line_head_unchanged() {
    let path = mandelbrot_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let lines: Vec<&str> = stdout.lines().collect();
    let expected_head = [
        "File      Mandelbrot.exe  (28672 bytes)",
        "Format    PE32, 3 sections",
        "Runtime   MSVBVM60.DLL  (Visual Basic 6)",
        "Header    VB5! at 0x00001760  build 0x2636",
        "Project   Mandelbrot_Fractal_Demo",
        "Title     Mandelbrot Fractal Demo",
        "Mode      native",
        "Objects   1",
    ];
    assert!(
        lines.len() > expected_head.len(),
        "stdout must hold more than the locked head once phase 2's sections \
         are appended: {stdout:?}"
    );
    assert_eq!(&lines[..expected_head.len()], expected_head.as_slice());
}

/// Below the locked head, one line per object gives the object name and its
/// kind.
#[test]
fn grayscale_prints_one_line_per_object_with_its_name_and_its_kind() {
    let path = grayscale_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    for expected in [
        "frmGrayscale  (form)",
        "pdOpenSaveDialog  (class)",
        "FastDrawing  (class)",
    ] {
        assert!(
            stdout.contains(expected),
            "stdout did not hold {expected:?}: {stdout:?}"
        );
    }
}

/// Under each object, one line per public procedure gives a prototype with
/// the argument names, the argument types and the `ByRef`, `Array` and
/// `Optional` modifiers, and an optional argument with a default prints the
/// default. A private procedure prints as `private`, with no name and no
/// invented identifier.
#[test]
fn grayscale_prints_prototypes_with_modifiers_and_defaults_and_marks_private_procedures() {
    let path = grayscale_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    // GetImageWidth: one ByRef argument, a return type, no default.
    assert!(
        stdout.contains("GetImageWidth(ByRef srcPictureBox As Object"),
        "stdout was: {stdout:?}"
    );
    assert!(stdout.contains(") As Long"), "stdout was: {stdout:?}");

    // GetImageData2D: an Array argument (dstPixelData) and an Optional
    // argument with a recovered Boolean default.
    assert!(
        stdout.contains("dstPixelData() As Byte"),
        "the Array modifier must print as (): {stdout:?}"
    );
    assert!(
        stdout.contains("Optional fixOrientation As Boolean = false"),
        "stdout was: {stdout:?}"
    );

    // Every private slot prints the bare word, with no index and no name.
    let private_lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| *line == "private")
        .collect();
    assert!(
        !private_lines.is_empty(),
        "at least one private procedure must print: {stdout:?}"
    );
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("private") {
            assert_eq!(
                trimmed, "private",
                "a private line must be the bare word, with no index or name: {line:?}"
            );
        }
    }
}

/// An unknown object kind prints the word `unknown` and the raw value in
/// hexadecimal, per D-08, and the run still exits 0. No corpus program
/// carries one (only `Form`, `Module` and `Class` occur, per plan 02-02), so
/// this test patches `frmFractal`'s `fObjectType` to a value none of the
/// three match.
#[test]
fn a_patched_unknown_object_kind_prints_its_raw_value_and_exits_zero() {
    let data = fs::read(mandelbrot_path()).unwrap();
    // `fObjectType` for `frmFractal` is `0x0001_8083` at file offset
    // `0x1c08`, resolved the same way `vb::object`'s own tests resolve it:
    // through the pointer chain, not a byte search. The offset is recorded
    // here because this test file cannot import the library's own private
    // test helpers; a corpus-wide grep for this exact byte pattern
    // (`\x83\x80\x01\x00` at this file's own object element) is how it was
    // found, and `deform6::inspect` on the unpatched file confirms the value
    // it replaces really is `0x0001_8083`.
    let mut bytes = data.clone();
    let pos = bytes
        .windows(4)
        .position(|w| w == [0x83, 0x80, 0x01, 0x00])
        .expect("frmFractal's fObjectType must be found in the unpatched file");
    bytes[pos..pos + 4].copy_from_slice(&0xFEED_1234_u32.to_le_bytes());

    let path = std::env::temp_dir().join(format!(
        "deform6-cli-test-unknown-kind-{}.exe",
        std::process::id()
    ));
    fs::write(&path, &bytes).unwrap();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    fs::remove_file(&path).ok();

    assert_eq!(code, 0, "stderr was: {stderr}");
    assert!(
        stdout.contains("unknown, raw value 0xfeed1234"),
        "stdout was: {stdout:?}"
    );
}

/// A standard module prints its procedure count and a sentence saying its
/// names are not reachable through this structure, rather than an empty
/// list.
#[test]
fn map_editor_prints_its_standard_modules_procedure_count_and_not_an_empty_list() {
    let path = map_editor_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    assert!(
        stdout.contains(
            "1 procedure(s) declared; their names are not reachable through this structure"
        ),
        "stdout was: {stdout:?}"
    );
    assert!(
        stdout.contains(
            "7 procedure(s) declared; their names are not reachable through this structure"
        ),
        "stdout was: {stdout:?}"
    );
}

/// Running the command on `Grayscale.exe` prints three objects and twelve
/// prototypes, and exits 0. The plan's own text additionally claims "eight
/// private markers for the class whose procedures are declared friend"; see
/// the SUMMARY for the correction (the measured number is six, on
/// `pdOpenSaveDialog`, not eight).
#[test]
fn grayscale_prints_three_objects_and_twelve_prototypes_and_exits_zero() {
    let path = grayscale_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    // Scoped to the Object graph section alone: the Declarations section
    // below it prints its own indented marker lines, which must not be
    // counted as procedure prototypes.
    let graph_start = stdout
        .find("Object graph\n")
        .expect("stdout must hold an Object graph section");
    let declarations_start = stdout
        .find("\nDeclarations\n")
        .expect("stdout must hold a Declarations section");
    let graph = &stdout[graph_start..declarations_start];

    let object_lines = graph
        .lines()
        .filter(|line| line.contains("(form)") || line.contains("(class)"))
        .count();
    assert_eq!(object_lines, 3, "graph was: {graph:?}");

    // A prototype line is a non-private, non-blank, indented line that is
    // not an object header line (no "(form)"/"(class)"/"(module)" suffix).
    let prototype_lines = graph
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && trimmed != "private"
                && !trimmed.ends_with("(form)")
                && !trimmed.ends_with("(class)")
                && !trimmed.ends_with("(module)")
                && line.starts_with("    ")
        })
        .count();
    assert_eq!(prototype_lines, 12, "graph was: {graph:?}");

    // pdOpenSaveDialog declares six procedure slots (two Friend Function
    // members plus four Private Declare Function lines, per plan 02-07's
    // own correction), and every one of the six is private, since Friend is
    // not Public.
    let private_lines = graph
        .lines()
        .filter(|line| line.trim() == "private")
        .count();
    assert_eq!(
        private_lines, 22,
        "34 total slots minus 12 public equals 22 private, across all three objects: {graph:?}"
    );
}

/// One declaration line prints per external import entry, holding the
/// library name and the export name. `Grayscale.exe` declares nine
/// `Declare` entries, of which one is internal (resolved inside the
/// runtime), so eight lines print and none for the internal entry.
#[test]
fn grayscale_prints_one_declaration_line_per_external_import_and_none_for_the_internal_one() {
    let path = grayscale_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let declaration_lines = stdout
        .lines()
        .filter(|line| {
            line.trim_start().contains('!') && line.starts_with("  ") && !line.starts_with("    ")
        })
        .count();
    assert_eq!(declaration_lines, 8, "stdout was: {stdout:?}");

    for expected in [
        "gdi32!StretchDIBits",
        "gdi32!GetDIBits",
        "gdi32!SetStretchBltMode",
        "gdi32!GetObjectA",
        "kernel32!lstrlenW",
        "comdlg32!CommDlgExtendedError",
        "comdlg32!GetSaveFileNameW",
        "comdlg32!GetOpenFileNameW",
    ] {
        assert!(
            stdout.contains(expected),
            "stdout did not hold {expected:?}: {stdout:?}"
        );
    }
}

/// Each declaration carries a marker saying the Visual Basic level procedure
/// name and the alias are not in the file, that the argument list is not in
/// the file, and that the owning module and the scope are not in the file.
#[test]
fn mandelbrot_prints_one_declaration_with_its_three_markers() {
    let path = mandelbrot_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    assert!(stdout.contains("gdi32!SetPixelV"), "stdout was: {stdout:?}");
    assert!(
        stdout.contains("the Visual Basic procedure name and its Alias are not in this file"),
        "stdout was: {stdout:?}"
    );
    assert!(
        stdout.contains("the argument names and types of this Declare are not in this file"),
        "stdout was: {stdout:?}"
    );
    assert!(
        stdout.contains(
            "the owning module and the Public or Private marker of this Declare are not in \
             this file"
        ),
        "stdout was: {stdout:?}"
    );
}

/// A gaps section prints below the graph, listing every open gap the run
/// found: `Map Editor.exe` carries a non-zero `cntPublicVars` on
/// `pdOpenSaveDialog` and a standard-module cap over its two modules.
#[test]
fn map_editor_gaps_section_lists_the_cap_and_the_unexplained_public_var_count() {
    let path = map_editor_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    assert!(
        stdout.contains("the standard-module cap applies to 2 object(s) and 8 procedure slot(s)"),
        "stdout was: {stdout:?}"
    );
    assert!(
        stdout.contains("cntPublicVars is 4, and its meaning is unresolved"),
        "stdout was: {stdout:?}"
    );
}

/// The gaps section prints before any recovery number, so a reader meets
/// the cap before they meet the ratio: it comes before the object graph,
/// which is where a standard module's procedure count (the number the cap
/// explains) is printed.
#[test]
fn the_gaps_section_prints_before_the_object_graph() {
    let path = map_editor_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let gaps_at = stdout
        .find("Gaps")
        .expect("stdout must hold a Gaps section");
    let graph_at = stdout
        .find("Object graph")
        .expect("stdout must hold an Object graph section");
    assert!(
        gaps_at < graph_at,
        "the Gaps section must print before the Object graph section: {stdout:?}"
    );
}

/// Every inferred item prints with its marker: per D-09, the ordinal alias
/// path is the only inferred item this phase produces, and no corpus
/// program has ever produced one (a corpus-wide script found zero ordinal
/// exports across 220 external entries). A real run's declarations
/// therefore never carry the inferred marker, which this test checks over
/// `Grayscale.exe`'s eight declarations directly, rather than only
/// asserting the claim in a comment.
#[test]
fn no_declaration_in_a_real_run_carries_the_inferred_marker() {
    let path = grayscale_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert!(
        !stdout.contains("inferred"),
        "no real export in this corpus is an ordinal alias, so nothing should print the \
         inferred marker: {stdout:?}"
    );
}

/// `A program with no external import prints the section with a line saying
/// there are none, rather than printing nothing.
#[test]
fn lock_work_station_prints_an_empty_declarations_section_and_not_nothing() {
    let path = lock_work_station_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    assert!(stdout.contains("Declarations"), "stdout was: {stdout:?}");
    assert!(
        stdout.contains("there are no external declarations in this file"),
        "stdout was: {stdout:?}"
    );
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
