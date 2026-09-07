#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! Exercises `support::vbp` and `support::rules` against the real
//! corpus, independent of `cargo test -p deform6 --test differential`
//! (plan 02-08), so a bug in the harness itself is caught here first.

#[path = "support/mod.rs"]
#[allow(
    dead_code,
    reason = "cargo compiles this shared module into every test binary and each binary uses a \
              different subset"
)]
mod support;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use support::vbp;

/// The count this whole harness is built to protect: 44 vendored
/// executables.
const EXPECTED_EXECUTABLE_COUNT: usize = 44;

/// `Brightness-effect/Part 3 - DIBs/Brightness3.vbp` declares
/// `dibBrightness.exe`, which the repository does not vendor. 44
/// executables, 45 project files.
const EXPECTED_PROJECT_FILE_COUNT: usize = 45;

/// The number of executables whose entry directory holds more than one
/// project file, so the `ExeName32` key is doing real work choosing
/// among them: the two under `SK-TFTP-Sample__VB6`, the three under
/// `machineLanguageConversion`, and the three under `Brightness-effect`.
const EXPECTED_MULTI_CANDIDATE_COUNT: usize = 8;

#[test]
fn every_executable_resolves_to_exactly_one_project_file() {
    let executables = vbp::executables();
    assert_eq!(
        executables.len(),
        EXPECTED_EXECUTABLE_COUNT,
        "found {} corpus executables, wanted {EXPECTED_EXECUTABLE_COUNT}",
        executables.len()
    );

    let projects = vbp::project_files();
    let mut failed = Vec::new();
    for exe in &executables {
        if let Err(err) = vbp::select_project_file(exe, &projects) {
            failed.push(format!("{}: {err}", exe.display()));
        }
    }
    assert!(
        failed.is_empty(),
        "{} of {} executables did not resolve to exactly one project file:\n{}",
        failed.len(),
        executables.len(),
        failed.join("\n")
    );
}

#[test]
fn eight_of_the_forty_four_executables_needed_the_key_to_choose_among_siblings() {
    let executables = vbp::executables();
    let projects = vbp::project_files();

    let mut needed_the_key = 0usize;
    for exe in &executables {
        let dir = vbp::entry_dir(exe).expect("every corpus executable has an entry directory");
        let candidates = projects.iter().filter(|p| p.starts_with(&dir)).count();
        if candidates > 1 {
            needed_the_key += 1;
        }
    }

    assert_eq!(
        needed_the_key,
        EXPECTED_MULTI_CANDIDATE_COUNT,
        "{needed_the_key} of {} executables had more than one project file candidate in their \
         entry directory, wanted {EXPECTED_MULTI_CANDIDATE_COUNT}; a selection that never has \
         to choose is a selection that cannot be proved right",
        executables.len()
    );
}

#[test]
fn a_corpus_wide_index_keyed_on_exe_name_32_alone_collides() {
    let projects = vbp::project_files();
    let index = vbp::corpus_wide_exe_name_index(&projects);

    let collisions: HashMap<&String, &Vec<PathBuf>> =
        index.iter().filter(|(_, paths)| paths.len() > 1).collect();

    assert!(
        !collisions.is_empty(),
        "expected a corpus-wide ExeName32 index to collide on at least one executable name; \
         found none, which would mean the entry-directory scope in \
         `support::vbp::select_project_file` is decoration rather than a fix for a real \
         collision"
    );
    // The collision is real, not a fixture: it names two vendored project
    // files that both compile to a file named `Project1.exe`.
    let (_, paths) = collisions
        .iter()
        .next()
        .expect("just proved collisions is non-empty");
    assert_eq!(
        paths.len(),
        2,
        "the known collision in this corpus is between exactly two project files"
    );
}

#[test]
fn the_corpus_holds_forty_five_project_files_and_the_one_without_an_executable_is_never_selected() {
    let projects = vbp::project_files();
    assert_eq!(
        projects.len(),
        EXPECTED_PROJECT_FILE_COUNT,
        "found {} corpus project files, wanted {EXPECTED_PROJECT_FILE_COUNT}",
        projects.len()
    );

    let executables = vbp::executables();
    let mut selected = Vec::new();
    for exe in &executables {
        let chosen = vbp::select_project_file(exe, &projects)
            .unwrap_or_else(|err| panic!("{}: {err}", exe.display()));
        selected.push(chosen);
    }
    assert_eq!(selected.len(), EXPECTED_EXECUTABLE_COUNT);

    let unvendored = projects
        .iter()
        .find(|p| project_names_an_executable_the_corpus_does_not_hold(p, &executables))
        .expect("the corpus holds exactly one project file whose executable is not vendored");
    assert!(
        !selected.contains(unvendored),
        "{} declares an executable this repository does not hold, and must never be selected \
         for any executable under test",
        unvendored.display()
    );
}

/// Says whether a project file's `ExeName32` names an executable that no
/// file in `executables` carries. Asks the corpus itself, rather than
/// hardcoding the one case this session found, so a future corpus change
/// cannot silently invalidate the assertion this backs.
fn project_names_an_executable_the_corpus_does_not_hold(
    path: &Path,
    executables: &[PathBuf],
) -> bool {
    let Some(exe_name) = support::vbp::Project::read(path).get("ExeName32") else {
        return false;
    };
    !executables.iter().any(|exe| {
        exe.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(&exe_name))
    })
}

#[test]
fn the_sepia_title_carries_its_inner_quotes() {
    let projects = vbp::project_files();
    let sepia = projects
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("Sepia.vbp"))
        .expect("the corpus vendors Sepia.vbp");
    let project = support::vbp::Project::read(sepia);
    assert_eq!(
        project.get("Title").as_deref(),
        Some("Sepia / \"Antique\" Image Filter")
    );
}

#[test]
fn an_absent_title_key_is_reported_as_absent_not_as_an_empty_string() {
    let projects = vbp::project_files();
    let gradient = projects
        .iter()
        .find(|p| {
            p.to_str()
                .is_some_and(|s| s.contains("SK-Gradient-Sample__VB6"))
        })
        .expect("the corpus vendors SK-Gradient-Sample__VB6/Project1.vbp");
    let project = support::vbp::Project::read(gradient);
    assert_eq!(
        project.get("Title"),
        None,
        "the title key is absent from this file; an absent key must not read as an empty \
         string, because the compiler omits the key entirely when the title equals the \
         project name"
    );
    assert_eq!(project.get("Name").as_deref(), Some("Project1"));
}

#[test]
fn a_byte_above_127_round_trips_through_the_project_reader() {
    let projects = vbp::project_files();
    let brightness4 = projects
        .iter()
        .find(|p| {
            p.to_str()
                .is_some_and(|s| s.contains("Brightness-effect") && s.contains("Even faster DIBs"))
                && p.file_name().and_then(|n| n.to_str()) == Some("Brightness.vbp")
        })
        .expect("the corpus vendors Brightness-effect/Part 4 - Even faster DIBs/Brightness.vbp");
    let project = support::vbp::Project::read(brightness4);
    let copyright = project
        .get("VersionLegalCopyright")
        .expect("this project's VersionLegalCopyright key is present");
    assert!(
        copyright.contains('\u{A9}'),
        "expected the copyright sign (byte 0xA9, Latin-1) to round-trip as U+00A9 in {copyright:?}"
    );
}
