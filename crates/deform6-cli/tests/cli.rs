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

/// The corpus's one flat, three-control form: `Command1`, `Picture1` and
/// `Label1`, all direct siblings of the form with no intervening container.
fn gradient_sample_path() -> PathBuf {
    corpus_root().join("public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe")
}

/// A form with a real third party control, `wsPop`, an `MSWinsockLib.Winsock`
/// instance nested inside `Frame1`.
fn winsock_sample_path() -> PathBuf {
    corpus_root().join("public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe")
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

/// `inspect` with no `--opcode-table` flag exits 0 and its report names the
/// builtin subset.
#[test]
fn inspect_with_no_opcode_table_flag_exits_zero_and_prints_the_builtin_line() {
    let path = lock_work_station_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert!(
        stdout.to_ascii_lowercase().contains("builtin"),
        "stdout did not name the builtin opcode table: {stdout:?}"
    );
}

/// `--opcode-table` pointing at a file that does not exist is a usage
/// error, exit 5, not a defect in the executable under inspection.
#[test]
fn inspect_with_an_opcode_table_flag_pointing_at_a_missing_file_exits_five() {
    let path = lock_work_station_path();
    let missing = std::env::temp_dir()
        .join("deform6-cli-test-a-missing-opcode-table-that-does-not-exist.toml");
    let (code, stdout, stderr) = run(&[
        OsStr::new("inspect"),
        OsStr::new("--opcode-table"),
        missing.as_os_str(),
        path.as_os_str(),
    ]);
    assert_eq!(code, 5, "stderr was: {stderr}");
    assert_one_line_refusal(code, &stdout, &stderr);
}

/// `--opcode-table` pointing at a malformed table exits 5 and the refusal
/// names the line number, matching `deform6::vb::opcodes::TableError`'s own
/// `Display`. The malformed table is a temporary file this test writes and
/// removes; `AGENTS.md` bars committing a table file, malformed or not.
#[test]
fn inspect_with_a_malformed_opcode_table_exits_five_and_prints_a_line_number() {
    let path = lock_work_station_path();
    let table_path = std::env::temp_dir().join(format!(
        "deform6-cli-test-a-malformed-opcode-table-{}.toml",
        std::process::id()
    ));
    // Missing the required "payload" field: a genuinely malformed row, not
    // a hand-simulated one.
    fs::write(&table_path, "[13]\n31 = { name = \"DrawMode\" }\n").unwrap();

    let (code, stdout, stderr) = run(&[
        OsStr::new("inspect"),
        OsStr::new("--opcode-table"),
        table_path.as_os_str(),
        path.as_os_str(),
    ]);
    fs::remove_file(&table_path).ok();

    assert_eq!(code, 5, "stderr was: {stderr}");
    assert_one_line_refusal(code, &stdout, &stderr);
    assert!(
        stderr.contains("line 2"),
        "stderr did not name the malformed row's line: {stderr:?}"
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

// --- Plan 03-10, Task 2: inspect prints the form tree ---------------------

/// Names the form (`Form1`, four control(s): the form's own outermost
/// block plus its three flat children) and each of its three named
/// controls.
#[test]
fn gradient_sample_names_the_form_and_its_three_controls() {
    let path = gradient_sample_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    assert!(
        stdout.contains("Form1  (4 control(s))"),
        "stdout did not name the form with its own control count: {stdout:?}"
    );
    for expected in ["Command1", "Picture1", "Label1"] {
        assert!(
            stdout.contains(expected),
            "stdout did not name control {expected:?}: {stdout:?}"
        );
    }
}

/// A control inside a container prints with a deeper indent than its own
/// parent: `Grayscale.exe`'s `frameShades` holds `hscrShades` and
/// `lblShades` as direct children. The executor changed the indentation to
/// a fixed zero, ran this test once, and saw it fail because every line's
/// own leading space count came back equal; the two counts that failure
/// printed are recorded in `03-10-SUMMARY.md`. Reverted before committing.
#[test]
fn a_control_inside_a_container_indents_deeper_than_its_parent() {
    let path = grayscale_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let leading_spaces = |line: &str| line.len() - line.trim_start_matches(' ').len();

    let parent_line = stdout
        .lines()
        .find(|line| line.trim_start().starts_with("frameShades"))
        .expect("frameShades must appear in the tree");
    let child_line = stdout
        .lines()
        .find(|line| line.trim_start().starts_with("hscrShades"))
        .expect("hscrShades must appear in the tree, nested inside frameShades");

    let parent_indent = leading_spaces(parent_line);
    let child_indent = leading_spaces(child_line);
    assert!(
        child_indent > parent_indent,
        "hscrShades (indent {child_indent}) must indent deeper than its own parent frameShades \
         (indent {parent_indent}): parent line {parent_line:?}, child line {child_line:?}"
    );
}

/// The report of a program with a real third party control names the type
/// library it does not hold, and the CLSID plan 03-16's own measurement
/// selects: the entry's own `oUuid` field, at its own measured byte offset
/// `0x1f18`.
#[test]
fn winsock_sample_prints_the_clsid_and_the_opaque_blob_statement() {
    let path = winsock_sample_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    assert!(
        stdout.contains("248DD896-BB45-11CF-9ABC-0080C7E7B78D"),
        "stdout did not print the joined CLSID: {stdout:?}"
    );
    assert!(
        stdout.contains("0x1f18"),
        "stdout did not print the byte offset the CLSID was read from: {stdout:?}"
    );
    assert!(
        stdout.to_lowercase().contains("type library"),
        "stdout did not print the opaque blob statement naming the type library: {stdout:?}"
    );
}

/// Plan 03-18: a bound event slot's own native handler address now reaches
/// a live run. `gradient_sample_path`'s own `Command1` slot 0 is the stub
/// `03-09-SUMMARY.md` proved byte for byte, and this test reads the address
/// out of the live run's own output, never a corpus address written into
/// the test as a literal.
#[test]
fn gradient_sample_prints_the_bound_handler_address_and_no_unbound_slot_carries_one() {
    let path = gradient_sample_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let bound_line = stdout
        .lines()
        .find(|line| line.contains("event slot") && line.contains(": bound,"))
        .expect("stdout must hold at least one bound event slot line");
    assert!(
        bound_line.contains("handler at 0x"),
        "the bound slot line did not print a handler address: {bound_line:?}"
    );

    let address = bound_line
        .split("handler at ")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .expect("the bound slot line must carry a parsable address")
        .trim();
    assert!(
        address.starts_with("0x"),
        "the extracted address did not carry the 0x prefix: {address:?}"
    );
    let hex_digits = &address[2..];
    assert_eq!(
        hex_digits.len(),
        8,
        "the address must print as eight hexadecimal digits: {address:?}"
    );
    u32::from_str_radix(hex_digits, 16)
        .expect("the extracted text after the 0x prefix must parse as hexadecimal");

    for line in stdout.lines() {
        if line.contains("event slot") && line.contains(": unbound,") {
            assert!(
                !line.contains("0x"),
                "an unbound slot line must carry no address: {line:?}"
            );
        }
    }
}

/// Plan 03-16: a joined CLSID always carries a caveat naming what the value
/// is not, since this repository's own research never confirmed either
/// candidate field against a project file's own declared identifier. The
/// caveat and the CLSID's own printed value are both present, so a reader
/// can compare the two without opening the project file.
#[test]
fn winsock_sample_prints_the_caveat_beside_the_joined_clsid() {
    let path = winsock_sample_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let clsid_line = stdout
        .lines()
        .find(|line| line.trim_start().starts_with("CLSID ="))
        .expect("the joined CLSID line must be present");
    let clsid_index = stdout
        .find(clsid_line)
        .expect("the found line must be in stdout");
    let caveat_line = stdout[clsid_index..]
        .lines()
        .nth(1)
        .expect("a line must follow the CLSID line");

    assert!(
        caveat_line.contains("248DD896-BB45-11CF-9ABC-0080C7E7B78D"),
        "the caveat line did not repeat the reported CLSID: {caveat_line:?}"
    );
    assert!(
        caveat_line.to_lowercase().contains("project file"),
        "the caveat line did not name the project file's own declared identifier: \
         {caveat_line:?}"
    );
    assert!(
        caveat_line.contains("oUuid"),
        "the caveat line did not name the field the value was read from: {caveat_line:?}"
    );
}

/// A control whose class name joins no component still prints the stated
/// reason plan 03-08 already gives, unchanged by plan 03-16: `print_clsid`'s
/// own `None` branch (`"CLSID: {reason}"`) is untouched by this plan's own
/// diff, only its `Some` branch gained the caveat print.
///
/// No corpus program in this repository has an external control whose class
/// name joins no component: the whole corpus's one third party control,
/// `wsPop`, always matches. `vb/ocx.rs`'s own unit tests
/// (`a_class_name_differing_by_one_character_gives_no_clsid_and_a_reason`,
/// `a_program_with_zero_components_gives_no_clsid_with_the_same_reason`)
/// prove the unjoined reason text itself is unchanged, against a synthetic
/// fixture, since no real corpus file can exercise it. This test proves the
/// complementary half at the CLI layer: the real, joined winsock sample
/// output never carries that unjoined wording, so the two paths remain
/// distinct.
#[test]
fn the_real_joined_winsock_sample_never_prints_the_unjoined_reason_wording() {
    let path = winsock_sample_path();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    assert!(
        !stdout.contains("not recoverable from this file"),
        "the real, joined winsock sample must not print the no-match reason: {stdout:?}"
    );
}

// --- Plan 05-01, Task 2: `--salvage` on `inspect` -------------------------

/// Copies `data` and writes `value` into the two bytes at
/// `VBHeader.wExternalCount`, learning the header's own file offset from
/// `deform6::inspect` over the unpatched bytes. Mirrors
/// `crates/deform6/tests/salvage.rs`'s own helper of the same purpose; that
/// file proves the library behaviour, this one proves the command line
/// surface, and the two never diverge because both learn the offset from
/// the same call rather than a literal each keeps separately.
fn patched_fast_flames_missing_a_nul_terminator() -> Vec<u8> {
    let data = fs::read(fast_flames_path()).unwrap();
    let table = deform6::vb::opcodes::OpcodeTable::builtin();
    let unpatched = deform6::inspect(&data, &table, deform6::journal::Mode::Salvage)
        .expect("the shipped file must inspect cleanly in salvage mode");
    let at = unpatched.header_offset.get().checked_add(0x46).unwrap();
    let at = usize::try_from(at).unwrap();
    let mut patched = data;
    patched[at..at + 2].copy_from_slice(&1_u16.to_le_bytes());
    patched
}

/// Absent `--salvage`, a file whose read had to assume a value refuses,
/// exit code 4, naming the byte offset. The patched bytes are written to a
/// process-unique path under the OS temp directory and removed again
/// before this test returns; they are never committed, so this is not the
/// third party fixture `AGENTS.md` bars from the repository.
#[test]
fn inspect_on_a_patched_file_exits_four_and_names_the_offset() {
    let patched = patched_fast_flames_missing_a_nul_terminator();
    let path = std::env::temp_dir().join(format!(
        "deform6-cli-test-patched-fast-flames-{}.exe",
        std::process::id()
    ));
    fs::write(&path, &patched).unwrap();
    let (code, stdout, stderr) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    fs::remove_file(&path).ok();

    assert_eq!(code, 4, "stdout was: {stdout:?}, stderr was: {stderr:?}");
    assert!(stdout.is_empty(), "stdout was: {stdout:?}");
    assert!(
        stderr.contains("0x1da4"),
        "stderr must name the byte offset 0x1da4: {stderr:?}"
    );
}

/// With `--salvage`, the same patched file produces its report and exits 0.
#[test]
fn inspect_with_salvage_on_the_same_patched_file_exits_zero_and_prints_the_report() {
    let patched = patched_fast_flames_missing_a_nul_terminator();
    let path = std::env::temp_dir().join(format!(
        "deform6-cli-test-patched-fast-flames-salvage-{}.exe",
        std::process::id()
    ));
    fs::write(&path, &patched).unwrap();
    let (code, stdout, stderr) = run(&[
        OsStr::new("inspect"),
        OsStr::new("--salvage"),
        path.as_os_str(),
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(code, 0, "stderr was: {stderr}");
    assert!(
        stdout.contains("File"),
        "a successful run must print the report: {stdout:?}"
    );
}

/// Absent `--salvage`, `extract` on a patched file exits 4 and writes
/// nothing: the output directory this test names is never created, since
/// `run_extract` refuses before `resolve_output_dir` ever runs.
#[test]
fn extract_on_a_patched_file_without_salvage_exits_four_and_writes_nothing() {
    let patched = patched_fast_flames_missing_a_nul_terminator();
    let path = std::env::temp_dir().join(format!(
        "deform6-cli-test-extract-patched-fast-flames-{}.exe",
        std::process::id()
    ));
    fs::write(&path, &patched).unwrap();
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deform6-cli-test-extract-salvage-refused-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&out_dir).ok();

    let (code, stdout, stderr) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        out_dir.as_os_str(),
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(code, 4, "stdout was: {stdout:?}, stderr was: {stderr:?}");
    assert!(
        !out_dir.exists(),
        "a refused run must not create the output directory"
    );
}

/// With `--salvage`, the same patched file writes the project and a report
/// whose limits array names the one assumption the run made.
#[test]
fn extract_with_salvage_on_a_patched_file_writes_the_project_and_the_assumption() {
    let patched = patched_fast_flames_missing_a_nul_terminator();
    let path = std::env::temp_dir().join(format!(
        "deform6-cli-test-extract-patched-fast-flames-salvage-{}.exe",
        std::process::id()
    ));
    fs::write(&path, &patched).unwrap();
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deform6-cli-test-extract-salvage-ok-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&out_dir).ok();

    let (code, _stdout, stderr) = run(&[
        OsStr::new("extract"),
        OsStr::new("--salvage"),
        path.as_os_str(),
        OsStr::new("-o"),
        out_dir.as_os_str(),
    ]);
    fs::remove_file(&path).ok();
    assert_eq!(code, 0, "stderr was: {stderr}");

    let report_path = out_dir.join("VBFire2.report.json");
    let report_bytes = fs::read_to_string(&report_path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", report_path.display()));
    assert!(
        report_bytes.contains("0x1da4"),
        "the written report must name the assumed byte offset 0x1da4: {report_bytes}"
    );
}

// --- Plan 04-01, Task 1: `extract` writes a project directory -------------

/// `corpus/vb6-code/Fire-effect/Fast_Flames.exe`, this task's one tracer
/// program.
fn fast_flames_path() -> PathBuf {
    corpus_root().join("vb6-code/Fire-effect/Fast_Flames.exe")
}

/// `extract` creates the output directory, writes the five expected files
/// into it, exits 0, and leaves the corpus directory it read from
/// untouched. The output directory lives under `CARGO_TARGET_TMPDIR`, the
/// path cargo gives an integration test binary for exactly this purpose.
#[test]
fn extract_creates_the_directory_and_writes_the_expected_files_leaving_the_corpus_untouched() {
    let path = fast_flames_path();
    let corpus_dir = path
        .parent()
        .expect("the corpus file has a parent directory");
    let before = dir_snapshot(corpus_dir);

    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("deform6-cli-test-extract-{}", std::process::id()));
    fs::remove_dir_all(&out_dir).ok();

    let (code, _stdout, stderr) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let entries: Vec<String> = fs::read_dir(&out_dir)
        .expect("the output directory must exist")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    for expected in [
        "VBFire2.vbp",
        "frmFire.frm",
        "frmFire.frx",
        "FastDrawing.cls",
        "VBFire2.report.json",
    ] {
        assert!(
            entries.iter().any(|entry| entry == expected),
            "entries: {entries:?}"
        );
    }
    assert_eq!(
        entries.len(),
        5,
        "extract must write nothing else into the output directory: {entries:?}"
    );

    let after = dir_snapshot(corpus_dir);
    assert_eq!(
        before, after,
        "extract must not change the corpus directory it read from"
    );

    fs::remove_dir_all(&out_dir).ok();
}

// --- Plan 04-08, Task 1: the subcommand, its three flags, and the exit
// codes it reuses ------------------------------------------------------

/// A file that is not a portable executable gives the same exit code from
/// `extract` as it gives from `inspect`: the reading step this subcommand
/// shares reuses `exit_for` unchanged.
#[test]
fn extract_on_a_file_that_is_not_a_portable_executable_gives_the_same_exit_code_as_inspect() {
    let path = std::env::temp_dir().join(format!(
        "deform6-cli-test-extract-not-a-program-{}.txt",
        std::process::id()
    ));
    fs::write(&path, b"this is not a program\n").unwrap();

    let (inspect_code, _out, _err) = run(&[OsStr::new("inspect"), path.as_os_str()]);
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deform6-cli-test-extract-not-a-program-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&out_dir).ok();
    let (extract_code, extract_stdout, extract_stderr) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        out_dir.as_os_str(),
    ]);
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&out_dir).ok();

    assert_eq!(
        inspect_code, 1,
        "inspect's own exit code changed underneath this test"
    );
    assert_eq!(
        extract_code, inspect_code,
        "extract stderr was: {extract_stderr}"
    );
    assert_one_line_refusal(extract_code, &extract_stdout, &extract_stderr);
}

/// A missing required flag (`-o`) is a usage error: the internal error
/// code, never the code reserved for a file that holds no Visual Basic
/// runtime.
#[test]
fn extract_with_no_output_flag_exits_five_and_not_two() {
    let path = fast_flames_path();
    let (code, stdout, stderr) = run(&[OsStr::new("extract"), path.as_os_str()]);
    assert_eq!(code, 5, "stderr was: {stderr}");
    assert_ne!(
        code, 2,
        "a usage error must not be told apart as \"no Visual Basic runtime\""
    );
    assert!(stdout.is_empty(), "stdout was: {stdout:?}");
}

/// An output path that cannot be created (here, a path that already names
/// a regular file, not a directory) gives the internal error code and a
/// message naming the path.
#[test]
fn extract_with_an_unwritable_output_path_exits_five_and_names_the_path() {
    let path = fast_flames_path();
    let blocking_file = std::env::temp_dir().join(format!(
        "deform6-cli-test-extract-blocking-file-{}",
        std::process::id()
    ));
    fs::write(&blocking_file, b"not a directory").unwrap();

    let (code, stdout, stderr) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        blocking_file.as_os_str(),
    ]);
    fs::remove_file(&blocking_file).ok();

    assert_eq!(code, 5, "stderr was: {stderr}");
    assert!(stdout.is_empty(), "stdout was: {stdout:?}");
    assert!(
        stderr.contains(&blocking_file.display().to_string()),
        "stderr did not name the unwritable path: {stderr:?}"
    );
}

// --- Plan 04-08, Task 2: resolve the directory, check every path against
// it, and build before you write ----------------------------------------

/// A second run into a directory that already holds files refuses without
/// `--force`, naming the directory, and the same run with `--force`
/// succeeds and overwrites.
#[test]
fn a_second_run_into_a_populated_directory_refuses_without_force_and_succeeds_with_it() {
    let path = fast_flames_path();
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deform6-cli-test-extract-force-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&out_dir).ok();

    let (first_code, _out, first_err) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(first_code, 0, "stderr was: {first_err}");

    let (second_code, second_stdout, second_stderr) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(second_code, 5, "stderr was: {second_stderr}");
    assert!(second_stdout.is_empty(), "stdout was: {second_stdout:?}");
    assert!(
        second_stderr.contains(&out_dir.canonicalize().unwrap().display().to_string()),
        "the refusal must name the directory: {second_stderr:?}"
    );

    let (force_code, _out, force_err) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        out_dir.as_os_str(),
        OsStr::new("--force"),
    ]);
    assert_eq!(force_code, 0, "stderr was: {force_err}");
    let entries: Vec<String> = fs::read_dir(&out_dir)
        .expect("the output directory must still exist")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries.len(), 5, "entries: {entries:?}");

    fs::remove_dir_all(&out_dir).ok();
}

/// A run writes nothing outside the resolved output directory: a listing
/// of the whole repository working tree, taken before and after the run,
/// is identical. The output directory lives under `CARGO_TARGET_TMPDIR`,
/// outside the repository, so this is a strict "zero new entries" check,
/// which is the strongest form of "the only new entries are inside the
/// resolved output directory."
#[test]
fn extract_changes_no_file_anywhere_in_the_repository_working_tree() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root must resolve");
    let before = dir_snapshot(&repo_root);

    let path = fast_flames_path();
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deform6-cli-test-extract-repo-untouched-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&out_dir).ok();
    let (code, _stdout, stderr) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    fs::remove_dir_all(&out_dir).ok();

    let after = dir_snapshot(&repo_root);
    assert_eq!(
        before, after,
        "extract must change no file anywhere in the repository working tree"
    );
}

/// The files land in the directory a symbolic link really points at, and
/// the run's own printed `--report` path names the real directory, never
/// the link. Unix-only: `std::os::unix::fs::symlink`.
#[test]
fn extract_into_a_symlinked_directory_writes_at_the_real_directory() {
    let path = fast_flames_path();
    let real_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deform6-cli-test-extract-symlink-real-{}",
        std::process::id()
    ));
    let link_path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deform6-cli-test-extract-symlink-link-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&real_dir).ok();
    fs::remove_file(&link_path).ok();
    fs::create_dir_all(&real_dir).unwrap();
    std::os::unix::fs::symlink(&real_dir, &link_path).expect("creating the symlink must succeed");

    let report_path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deform6-cli-test-extract-symlink-report-{}.json",
        std::process::id()
    ));
    fs::remove_file(&report_path).ok();

    let (code, stdout, stderr) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        link_path.as_os_str(),
        OsStr::new("--report"),
        report_path.as_os_str(),
    ]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let entries: Vec<String> = fs::read_dir(&real_dir)
        .expect("the real directory must hold the written files")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        entries.iter().any(|entry| entry == "frmFire.frm"),
        "entries in the real directory: {entries:?}"
    );
    assert!(
        !entries.iter().any(|entry| entry.ends_with(".report.json")),
        "the report must not also land in the output directory once --report is given: \
         {entries:?}"
    );

    assert!(
        report_path.exists(),
        "the report must exist at the user-named path"
    );
    let printed = stdout.trim();
    let resolved_report = report_path
        .canonicalize()
        .expect("the report now exists and must canonicalize");
    assert_eq!(
        Path::new(printed),
        resolved_report,
        "the run must print the resolved report path: {stdout:?}"
    );

    fs::remove_dir_all(&real_dir).ok();
    fs::remove_file(&link_path).ok();
    fs::remove_file(&report_path).ok();
}

// --- Plan 04-08, Task 3: all 44 corpus programs, exit 0, nothing outside
// the directory ----------------------------------------------------------

/// The count `04-08-PLAN.md`'s own verification pins: the number of `.exe`
/// files the corpus vendors today. A sweep that reads fewer than this
/// silently passes every other assertion it makes, which is the failure
/// shape `AGENTS.md` itself names: "a search for some names is not a
/// search for all of them."
const CORPUS_EXECUTABLE_COUNT: usize = 44;

/// Walks `corpus/` recursively and gives every file whose extension is
/// `.exe`, compared case-insensitively, sorted so the sweep below runs in
/// a stable order.
fn all_corpus_executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_for_executables(&corpus_root(), &mut out);
    out.sort();
    out
}

fn walk_for_executables(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            walk_for_executables(&path, out);
        } else if path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push(path);
        }
    }
}

/// The number of forms, among `report`'s own, whose own control tree holds
/// at least one control carrying a readable resource blob: the same fact
/// [`deform6::write::frm::FormFiles::frx`] being `Some` or `None` turns on,
/// since a form that names no blob gets no `.frx` file beside it.
fn forms_holding_a_blob(report: &deform6::Report) -> usize {
    report
        .forms
        .iter()
        .filter(|form| {
            form.controls.iter().any(|control| {
                control.properties.iter().any(|property| {
                    matches!(
                        property,
                        deform6::vb::propstream::PropertyValue::Blob { .. }
                    )
                })
            })
        })
        .count()
}

/// The number of modules and classes `report`'s own object table declares:
/// one `.bas` or `.cls` file each, per roadmap success criterion 1.
fn module_and_class_count(report: &deform6::Report) -> usize {
    report
        .objects
        .iter()
        .filter(|object| {
            matches!(
                object.kind,
                deform6::vb::classify::ObjectKind::Module
                    | deform6::vb::classify::ObjectKind::Class
            )
        })
        .count()
}

/// Roadmap success criterion 1, run over the whole corpus: every one of
/// the 44 programs extracts, exits 0, and writes exactly the file count
/// per kind the recovered report proves -- one project file, one form
/// file per form, one resource file per form that holds a blob, one
/// module or class file per module and class, and one report -- with
/// nothing written outside its own output directory. A repository-wide
/// listing, taken once before the sweep and once after, proves the whole
/// sweep together changed nothing outside the resolved output
/// directories it was given.
#[test]
fn all_corpus_programs_extract_with_exit_zero_and_the_written_file_count_matches_the_report() {
    let executables = all_corpus_executables();
    assert_eq!(
        executables.len(),
        CORPUS_EXECUTABLE_COUNT,
        "the sweep read {} executable(s) but the corpus is pinned to hold {}; the corpus \
         changed size and this count needs re-measuring",
        executables.len(),
        CORPUS_EXECUTABLE_COUNT
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root must resolve");
    let before = dir_snapshot(&repo_root);

    let table = deform6::vb::opcodes::OpcodeTable::builtin();
    let mut total_by_kind: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();

    for (index, exe_path) in executables.iter().enumerate() {
        let data = fs::read(exe_path)
            .unwrap_or_else(|err| panic!("reading {}: {err}", exe_path.display()));
        let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict)
            .unwrap_or_else(|refusal| {
                panic!("{} did not inspect cleanly: {refusal}", exe_path.display())
            });

        let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("deform6-cli-sweep-{}-{index}", std::process::id()));
        fs::remove_dir_all(&out_dir).ok();

        let (code, stdout, stderr) = run(&[
            OsStr::new("extract"),
            exe_path.as_os_str(),
            OsStr::new("-o"),
            out_dir.as_os_str(),
        ]);
        assert_eq!(
            code,
            0,
            "{}: extract must exit 0; stderr was: {stderr}",
            exe_path.display()
        );
        assert!(
            stdout.is_empty(),
            "{}: extract prints nothing to stdout by default: {stdout:?}",
            exe_path.display()
        );

        let entries: Vec<String> = fs::read_dir(&out_dir)
            .unwrap_or_else(|err| {
                panic!(
                    "{}: reading {}: {err}",
                    exe_path.display(),
                    out_dir.display()
                )
            })
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        let vbp_count = entries.iter().filter(|e| e.ends_with(".vbp")).count();
        assert_eq!(
            vbp_count,
            1,
            "{}: expected exactly one .vbp, got: {entries:?}",
            exe_path.display()
        );

        let frm_count = entries.iter().filter(|e| e.ends_with(".frm")).count();
        assert_eq!(
            frm_count,
            report.forms.len(),
            "{}: .frm count {frm_count} did not match the recovered form count {}: {entries:?}",
            exe_path.display(),
            report.forms.len()
        );

        let expected_frx = forms_holding_a_blob(&report);
        let frx_count = entries.iter().filter(|e| e.ends_with(".frx")).count();
        assert_eq!(
            frx_count,
            expected_frx,
            "{}: .frx count {frx_count} did not match the number of forms holding a blob \
             {expected_frx}: {entries:?}",
            exe_path.display()
        );

        let expected_code = module_and_class_count(&report);
        let code_count = entries
            .iter()
            .filter(|e| e.ends_with(".bas") || e.ends_with(".cls"))
            .count();
        assert_eq!(
            code_count,
            expected_code,
            "{}: module/class file count {code_count} did not match the recovered count \
             {expected_code}: {entries:?}",
            exe_path.display()
        );

        let report_count = entries
            .iter()
            .filter(|e| e.ends_with(".report.json"))
            .count();
        assert_eq!(
            report_count,
            1,
            "{}: expected exactly one report file, got: {entries:?}",
            exe_path.display()
        );

        let expected_total = 1 + report.forms.len() + expected_frx + expected_code + 1;
        assert_eq!(
            entries.len(),
            expected_total,
            "{}: wrote {} file(s), expected {expected_total}: {entries:?}",
            exe_path.display(),
            entries.len()
        );

        *total_by_kind.entry("vbp").or_insert(0) += vbp_count;
        *total_by_kind.entry("frm").or_insert(0) += frm_count;
        *total_by_kind.entry("frx").or_insert(0) += frx_count;
        *total_by_kind.entry("bas_or_cls").or_insert(0) += code_count;
        *total_by_kind.entry("report_json").or_insert(0) += report_count;

        fs::remove_dir_all(&out_dir).ok();
    }

    // Printed once, for the SUMMARY to quote verbatim: the total number of
    // files this sweep wrote across all 44 programs, by kind.
    println!("plan 04-08 corpus sweep totals: {total_by_kind:?}");

    let after = dir_snapshot(&repo_root);
    assert_eq!(
        before, after,
        "the whole sweep changed a file somewhere in the repository working tree"
    );
}

/// Research open question 2, settled: `Map Editor.exe`'s own `Main` form,
/// whose control tree walk refuses (`docs/WINDOWS.md` finding 8),
/// still exits 0, still gets a `.frm` file, and the report marks it
/// `unrecoverable` rather than silently omitting it.
#[test]
fn map_editor_writes_a_form_file_for_its_refused_form_and_the_report_marks_it_unrecoverable() {
    let path = map_editor_path();
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deform6-cli-test-map-editor-refused-form-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&out_dir).ok();

    let (code, _stdout, stderr) = run(&[
        OsStr::new("extract"),
        path.as_os_str(),
        OsStr::new("-o"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let entries: Vec<String> = fs::read_dir(&out_dir)
        .expect("the output directory must exist")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        entries.iter().any(|entry| entry == "Main.frm"),
        "the refused form must still get a .frm file: {entries:?}"
    );

    let data = fs::read(&path).unwrap();
    let table = deform6::vb::opcodes::OpcodeTable::builtin();
    let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict)
        .expect("Map Editor.exe must inspect cleanly");
    let written = deform6::write::project(&report, &data, deform6::journal::Mode::Strict)
        .expect("write::project must not refuse");
    let main_form_path = deform6::report::path_for_form(
        &deform6::write::model::SafeName::new("Main", deform6::write::model::NameKind::Form).0,
    );
    assert!(
        written.report.items.iter().any(|item| {
            item.path == main_form_path
                && item.confidence == deform6::report::Confidence::Unrecoverable
        }),
        "no item at {main_form_path:?} is graded unrecoverable: {:?}",
        written.report.items
    );

    fs::remove_dir_all(&out_dir).ok();
}
