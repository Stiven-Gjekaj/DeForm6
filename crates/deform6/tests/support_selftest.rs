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

/// Reads a file as Latin-1 bytes, the same rule every reader in this
/// harness uses. `std::fs::read_to_string` would refuse a source file
/// that carries a byte above 127 in a comment, which this corpus does.
fn read_latin1(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(bytes.iter().copied().map(char::from).collect())
}

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

/// The declared object totals over the 44 selected project files: 105
/// objects in total, 53 forms, 8 standard modules and 44 classes, and
/// no other kind occurs anywhere in this corpus.
const EXPECTED_OBJECT_TOTAL: usize = 105;
const EXPECTED_FORM_COUNT: usize = 53;
const EXPECTED_MODULE_COUNT: usize = 8;
const EXPECTED_CLASS_COUNT: usize = 44;

/// Selects the one project file for every corpus executable and gives
/// every object each one declares, alongside the project file that
/// declared it. Every test in this file that needs the whole corpus's
/// declared objects goes through this one function, so a change to the
/// selection rule cannot quietly diverge between tests.
fn all_declared_objects() -> Vec<(PathBuf, support::vbp::DeclaredObject)> {
    let executables = vbp::executables();
    let projects = vbp::project_files();
    let mut out = Vec::new();
    for exe in &executables {
        let project_path = vbp::select_project_file(exe, &projects)
            .unwrap_or_else(|err| panic!("{}: {err}", exe.display()));
        let project = support::vbp::Project::read(&project_path);
        for object in project.declared_objects() {
            out.push((project_path.clone(), object));
        }
    }
    out
}

#[test]
fn the_declared_object_totals_are_one_hundred_and_five_split_fifty_three_eight_forty_four() {
    let declared = all_declared_objects();
    assert_eq!(
        declared.len(),
        EXPECTED_OBJECT_TOTAL,
        "found {} declared objects across the 44 selected project files, wanted \
         {EXPECTED_OBJECT_TOTAL}",
        declared.len()
    );

    let mut by_kind: HashMap<&str, usize> = HashMap::new();
    for (_, object) in &declared {
        let key = match object.kind {
            support::vbp::ObjectKind::Form => "Form",
            support::vbp::ObjectKind::Module => "Module",
            support::vbp::ObjectKind::Class => "Class",
            support::vbp::ObjectKind::UserControl => "UserControl",
            support::vbp::ObjectKind::PropertyPage => "PropertyPage",
            support::vbp::ObjectKind::UserDocument => "UserDocument",
            support::vbp::ObjectKind::Designer => "Designer",
            support::vbp::ObjectKind::RelatedDoc => "RelatedDoc",
        };
        *by_kind.entry(key).or_insert(0) += 1;
    }

    assert_eq!(
        by_kind.get("Form").copied().unwrap_or(0),
        EXPECTED_FORM_COUNT
    );
    assert_eq!(
        by_kind.get("Module").copied().unwrap_or(0),
        EXPECTED_MODULE_COUNT
    );
    assert_eq!(
        by_kind.get("Class").copied().unwrap_or(0),
        EXPECTED_CLASS_COUNT
    );
    for other in [
        "UserControl",
        "PropertyPage",
        "UserDocument",
        "Designer",
        "RelatedDoc",
    ] {
        assert_eq!(
            by_kind.get(other).copied().unwrap_or(0),
            0,
            "this corpus declares no {other} objects; a non-zero count here means the corpus \
             changed and the totals above need re-measuring"
        );
    }
}

#[test]
fn a_forms_object_name_is_the_attribute_not_the_file_name() {
    // `Mandelbrot.frm`'s `Attribute VB_Name` is `frmFractal`, which
    // shares no substring with the file's own name. Taking the object
    // name from the file name (with its extension removed) instead of
    // scanning for the attribute would report `Mandelbrot`, not
    // `frmFractal`, and this is the case that catches that.
    let declared = all_declared_objects();
    let mandelbrot_form = declared
        .iter()
        .find(|(_, o)| {
            o.kind == support::vbp::ObjectKind::Form
                && o.source_file.file_name().and_then(|n| n.to_str()) == Some("Mandelbrot.frm")
        })
        .expect("the corpus vendors Mandelbrot/Mandelbrot.frm as a declared Form");
    assert_eq!(mandelbrot_form.1.name.as_deref(), Some("frmFractal"));
}

#[test]
fn every_object_name_resolves_via_the_attribute_scan_or_the_one_recorded_fallback() {
    let declared = all_declared_objects();
    let resolved = declared.iter().filter(|(_, o)| o.name.is_some()).count();
    // 104 resolve; the one that does not is `Edge-detection`'s
    // `cCommonDialog.cls`, and it does not resolve because the fallback
    // (below) covers it. This assertion is really "every object has a
    // name", proved via the two other tests that constrain how it got one.
    assert_eq!(resolved, EXPECTED_OBJECT_TOTAL);
}

/// A 40-line prefix scan for `Attribute VB_Name`, reproduced here on
/// purpose so this test can prove the whole-file scan in
/// `support::vbp::Project::declared_objects` is necessary. This is a
/// deliberately wrong reader, kept only to demonstrate the trap; the
/// real reader (`support::vbp::find_vb_name`, private to that module)
/// never bounds the scan this way.
fn find_vb_name_forty_line_prefix(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let text: String = bytes.iter().copied().map(char::from).collect();
    for line in text.lines().take(40) {
        let s = line.trim();
        let Some(rest) = s.strip_prefix("Attribute VB_Name") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let quote_first = rest.find('"')?;
        let quote_last = rest.rfind('"')?;
        if quote_last <= quote_first {
            continue;
        }
        return Some(rest[quote_first + 1..quote_last].to_owned());
    }
    None
}

#[test]
fn a_forty_line_prefix_scan_fails_on_most_forms_because_the_attribute_sits_after_the_form_block() {
    let executables = vbp::executables();
    let projects = vbp::project_files();

    let mut form_sources = Vec::new();
    for exe in &executables {
        let project_path = vbp::select_project_file(exe, &projects)
            .unwrap_or_else(|err| panic!("{}: {err}", exe.display()));
        let project = support::vbp::Project::read(&project_path);
        for object in project.declared_objects() {
            if object.kind == support::vbp::ObjectKind::Form {
                form_sources.push(object.source_file);
            }
        }
    }
    assert_eq!(form_sources.len(), EXPECTED_FORM_COUNT);

    let matched_by_prefix_scan = form_sources
        .iter()
        .filter(|src| find_vb_name_forty_line_prefix(src).is_some())
        .count();

    // The plan this harness was built from states the 40-line scan
    // matches none of the 53 forms. Measured against the real corpus,
    // two forms are short enough that `Attribute VB_Name` sits inside
    // the first 40 lines by accident: `LockWorkStation`'s
    // `FrmLockWorkStation.frm` (line 16) and
    // `SK-Gradient-Sample__VB6`'s `Form1.frm` (line 38). The claim of
    // zero is corrected here to the measured number; the trap the test
    // exists to prove -- that a bounded scan cannot be trusted for a
    // form -- holds regardless, since the scan still fails on 51 of 53.
    assert_eq!(
        matched_by_prefix_scan,
        2,
        "a 40-line prefix scan matched {matched_by_prefix_scan} of {} forms; expected exactly \
         2, both short enough for the attribute to land inside the bound by chance",
        form_sources.len()
    );
}

#[test]
fn exactly_one_object_in_the_corpus_needs_the_fallback_name() {
    let declared = all_declared_objects();
    let fallbacks: Vec<_> = declared
        .iter()
        .filter(|(_, o)| o.name_source == Some(support::vbp::NameSource::Fallback))
        .collect();
    assert_eq!(
        fallbacks.len(),
        1,
        "expected exactly one object in the corpus to need the project-line fallback name, \
         found {}: {:?}",
        fallbacks.len(),
        fallbacks
            .iter()
            .map(|(p, o)| format!("{} ({:?})", p.display(), o.source_file))
            .collect::<Vec<_>>()
    );
    let (_, object) = fallbacks[0];
    assert_eq!(object.kind, support::vbp::ObjectKind::Class);
    assert!(
        !object.source_file.exists(),
        "the fallback must only be used when the referenced source file is absent"
    );
}

#[test]
fn where_both_the_attribute_and_the_prefix_are_available_they_agree_fifty_one_of_fifty_one() {
    let declared = all_declared_objects();

    let mut both_available = 0usize;
    let mut agree = 0usize;
    for (_, object) in &declared {
        // Only an object resolved via the attribute has an attribute
        // value to compare; the fallback case has no attribute at all
        // (that is why it needed the fallback), and a form has no
        // prefix (the next test covers that).
        if object.name_source != Some(support::vbp::NameSource::Attribute) {
            continue;
        }
        let Some(prefix) = &object.prefix else {
            continue;
        };
        both_available += 1;
        if Some(prefix.as_str()) == object.name.as_deref() {
            agree += 1;
        }
    }
    assert_eq!(both_available, 51);
    assert_eq!(agree, 51);
}

#[test]
fn a_form_line_carries_no_prefix_so_the_fallback_cannot_serve_a_form() {
    // A synthetic project line, built inside this test per AGENTS.md's
    // "build the state that a test needs inside the test": a `Form=`
    // line whose referenced source file does not exist anywhere.
    let dir = std::env::temp_dir().join("deform6-vbp-selftest-form-fallback");
    std::fs::create_dir_all(&dir).expect("creating a scratch directory for this test");
    let project_path = dir.join("Synthetic.vbp");
    std::fs::write(&project_path, "Type=Exe\r\nForm=MissingForm.frm\r\n")
        .expect("writing the synthetic project file");

    let project = support::vbp::Project::read(&project_path);
    let objects = project.declared_objects();
    assert_eq!(objects.len(), 1);
    let form = &objects[0];
    assert_eq!(form.kind, support::vbp::ObjectKind::Form);
    assert!(
        form.name.is_none(),
        "a form whose source file is missing has no prefix to fall back to and must report a \
         gap, not a guess: got {:?}",
        form.name
    );
    assert_eq!(form.name_source, None);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn neither_orphan_common_dialog_class_appears_in_its_projects_declared_list() {
    let projects = vbp::project_files();
    let orphan_projects: Vec<PathBuf> = projects
        .iter()
        .filter(|p| {
            p.to_str().is_some_and(|s| {
                s.contains("Hidden-Markov-model") || s.contains("Randomize-effects")
            })
        })
        .cloned()
        .collect();
    assert_eq!(
        orphan_projects.len(),
        2,
        "expected exactly one project file each for Hidden-Markov-model and Randomize-effects"
    );

    for project_path in &orphan_projects {
        let project = support::vbp::Project::read(project_path);
        let names_a_common_dialog_class = project.declared_objects().iter().any(|o| {
            o.source_file
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("cCommonDialog.cls"))
        });
        assert!(
            !names_a_common_dialog_class,
            "{} must not declare cCommonDialog.cls: the project file never lists it, so the \
             compiler never built it in, even though the source sits on disk",
            project_path.display()
        );
    }
}

#[test]
fn five_rules_exist_each_with_an_identifier_and_a_reason() {
    assert_eq!(support::rules::RULES.len(), 5);
    for rule in support::rules::RULES {
        assert!(!rule.id.is_empty());
        assert!(
            rule.reason.len() > 20,
            "rule {:?} carries a reason too short to be a real explanation: {:?}",
            rule.id,
            rule.reason
        );
    }
}

/// Builds every [`support::rules::Candidate`] the whole corpus supports,
/// covering every rule at least once. This is the real-corpus input the
/// "every rule matches something" instrument tallies; see
/// `support::rules::tally` for the generic counting logic itself.
fn all_rule_candidates() -> Vec<support::rules::Candidate> {
    use support::rules::Candidate;

    let mut out = Vec::new();

    // Procedure candidates: every procedure in every declared object,
    // tagged with whether its object is a standard module.
    for (_, object) in all_declared_objects() {
        let Ok(bytes) = std::fs::read(&object.source_file) else {
            continue;
        };
        let text: String = bytes.iter().copied().map(char::from).collect();
        let in_standard_module = object.kind == support::vbp::ObjectKind::Module;
        for visibility in support::rules::scan_procedure_visibilities(&text) {
            out.push(Candidate::Procedure {
                in_standard_module,
                visibility,
            });
        }
    }

    // Source-listing candidates: every declared object's source-file
    // presence (covers the absent-source rule), plus the two orphan
    // common dialog class sources (covers the unlisted-source rule).
    for (_, object) in all_declared_objects() {
        out.push(Candidate::SourceListing {
            declared_in_project: true,
            exists_on_disk: object.source_file.exists(),
        });
    }
    for project_path in vbp::project_files().iter().filter(|p| {
        p.to_str()
            .is_some_and(|s| s.contains("Hidden-Markov-model") || s.contains("Randomize-effects"))
    }) {
        let dir = project_path
            .parent()
            .expect("a project file always has a parent directory");
        let orphan = dir.join("cCommonDialog.cls");
        if orphan.exists() {
            out.push(Candidate::SourceListing {
                declared_in_project: false,
                exists_on_disk: true,
            });
        }
    }

    // A never-kept candidate: a real local variable declaration exists
    // in the corpus (`Mandelbrot.frm`'s `Form_Load` declares one with
    // `Dim`), which is the kind of fact this rule covers.
    let mandelbrot_form = all_declared_objects()
        .into_iter()
        .find(|(_, o)| {
            o.kind == support::vbp::ObjectKind::Form
                && o.source_file.file_name().and_then(|n| n.to_str()) == Some("Mandelbrot.frm")
        })
        .map(|(_, o)| o.source_file);
    if let Some(path) = mandelbrot_form {
        let text = read_latin1(&path).unwrap_or_default();
        if text.lines().any(|l| l.trim_start().starts_with("Dim ")) {
            out.push(Candidate::NeverKept);
        }
    }

    out
}

#[test]
fn every_rule_matches_at_least_one_real_corpus_case() {
    let candidates = all_rule_candidates();
    let tally = support::rules::tally(candidates);
    let zero: Vec<&str> = tally
        .iter()
        .filter(|&(_, &count)| count == 0)
        .map(|(&id, _)| id)
        .collect();
    assert!(
        zero.is_empty(),
        "the following rules matched zero real corpus cases: {zero:?}; a rule that matches \
         nothing is either wrong or no longer needed"
    );
}

#[test]
fn the_standard_module_rule_excludes_exactly_eight_objects_and_twenty_three_procedure_slots() {
    let declared = all_declared_objects();
    let module_objects: Vec<_> = declared
        .iter()
        .filter(|(_, o)| o.kind == support::vbp::ObjectKind::Module)
        .collect();
    assert_eq!(module_objects.len(), 8);

    let mut slots = 0usize;
    for (_, object) in &module_objects {
        let Some(text) = read_latin1(&object.source_file) else {
            continue;
        };
        slots += support::rules::scan_procedure_visibilities(&text).len();
    }
    assert_eq!(
        slots, 23,
        "the 8 standard module objects in the corpus declare 23 procedure slots (Sub, \
         Function, Property and Declare lines combined); found {slots}"
    );
}

#[test]
fn the_scope_rule_excludes_every_non_public_procedure_grayscale_effects_dialog_class_declares() {
    let declared = all_declared_objects();
    let dialog_class = declared
        .iter()
        .find(|(p, o)| {
            p.to_str().is_some_and(|s| s.contains("Grayscale-effect"))
                && o.source_file.file_name().and_then(|n| n.to_str())
                    == Some("pdOpenSaveDialog.cls")
        })
        .map(|(_, o)| o.source_file.clone())
        .expect("the corpus vendors Grayscale-effect/pdOpenSaveDialog.cls");

    let text = read_latin1(&dialog_class).expect("reading pdOpenSaveDialog.cls");
    let visibilities = support::rules::scan_procedure_visibilities(&text);
    // The plan this harness was built from states this class declares
    // "its two procedures" friend. Measured against the real file: it
    // declares six, two `Friend Function` members and four `Private
    // Declare Function` lines (the four API imports near the top of the
    // file). A `Private Declare` consumes a procedure slot exactly as a
    // `Private Sub` does (per `CONTEXT.md`'s `frmFractal` measurement),
    // so all six, not two, are non-public. The plan's "two" undercounts
    // the real case; the correction is recorded in the SUMMARY. Note
    // also: plain `grep` (no `-a`) silently treats this Latin-1 file as
    // binary and reports zero matches for "Friend" -- a trap for anyone
    // re-verifying this by hand.
    assert_eq!(
        visibilities.len(),
        6,
        "expected six procedure declarations (four Private Declare, two Friend Function), \
         found {visibilities:?}"
    );
    let friend_count = visibilities
        .iter()
        .filter(|v| **v == support::rules::Visibility::Friend)
        .count();
    let private_count = visibilities
        .iter()
        .filter(|v| **v == support::rules::Visibility::Private)
        .count();
    assert_eq!(friend_count, 2);
    assert_eq!(private_count, 4);

    let scope_rule = support::rules::RULES
        .iter()
        .find(|r| r.id == "scope")
        .expect("the scope rule exists");
    for visibility in visibilities {
        let candidate = support::rules::Candidate::Procedure {
            in_standard_module: false,
            visibility,
        };
        assert!(
            scope_rule.excludes(candidate),
            "the scope rule must exclude every procedure this class declares, since none are \
             Public: got {visibility:?}"
        );
    }
}

#[test]
fn the_unlisted_source_rule_excludes_the_two_orphan_common_dialog_classes() {
    let unlisted_source_rule = support::rules::RULES
        .iter()
        .find(|r| r.id == "unlisted-source")
        .expect("the unlisted-source rule exists");

    let orphan_projects: Vec<PathBuf> = vbp::project_files()
        .into_iter()
        .filter(|p| {
            p.to_str().is_some_and(|s| {
                s.contains("Hidden-Markov-model") || s.contains("Randomize-effects")
            })
        })
        .collect();
    assert_eq!(orphan_projects.len(), 2);

    for project_path in &orphan_projects {
        let dir = project_path
            .parent()
            .expect("a project file always has a parent directory");
        let orphan = dir.join("cCommonDialog.cls");
        assert!(
            orphan.exists(),
            "{} expects an orphan cCommonDialog.cls on disk",
            project_path.display()
        );
        let candidate = support::rules::Candidate::SourceListing {
            declared_in_project: false,
            exists_on_disk: true,
        };
        assert!(unlisted_source_rule.excludes(candidate));
    }
}

#[test]
fn the_absent_source_rule_excludes_from_both_sides_and_never_lets_recovered_exceed_declared() {
    let edge_detection = vbp::project_files()
        .into_iter()
        .find(|p| {
            p.to_str().is_some_and(|s| s.contains("Edge-detection"))
                && p.file_name().and_then(|n| n.to_str()) == Some("EdgeDetection.vbp")
        })
        .expect("the corpus vendors Edge-detection/EdgeDetection.vbp");
    let project = support::vbp::Project::read(&edge_detection);
    let declared = project.declared_objects();
    assert_eq!(declared.len(), 4, "Edge-detection declares four objects");

    let missing_source_count = declared.iter().filter(|o| !o.source_file.exists()).count();
    assert_eq!(
        missing_source_count, 1,
        "expected exactly one of Edge-detection's declared objects to have no source file on \
         disk"
    );

    let absent_source_rule = support::rules::RULES
        .iter()
        .find(|r| r.id == "absent-source")
        .expect("the absent-source rule exists");

    // "recovered" is simulated as the compiled binary's own object
    // count: every object the .vbp declares, since the compiler built
    // all four in, including the one whose source this repository no
    // longer holds.
    let recovered_total = declared.len();
    let mut declared_after = 0usize;
    let mut recovered_excluded = 0usize;
    for object in &declared {
        let candidate = support::rules::Candidate::SourceListing {
            declared_in_project: true,
            exists_on_disk: object.source_file.exists(),
        };
        let (excluded_declared, excluded_recovered) =
            support::rules::apply_symmetrically(absent_source_rule, candidate);
        if !excluded_declared {
            declared_after += 1;
        }
        if excluded_recovered {
            recovered_excluded += 1;
        }
    }
    let recovered_after = recovered_total - recovered_excluded;

    assert_eq!(declared_after, 3);
    assert_eq!(
        recovered_after, declared_after,
        "the absent-source rule must exclude its match from both sides; a one-sided exclusion \
         would let the recovered count ({recovered_after}) exceed the declared count \
         ({declared_after})"
    );
}
