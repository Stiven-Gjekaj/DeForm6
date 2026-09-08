#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The gate the whole phase exists to build: what `deform6` recovers,
//! compared against the source the executable was built from, over all 44
//! corpus programs, in both directions.
//!
//! This file proves the object graph against the source the 44 corpus
//! executables were built from, in both directions: every declared object
//! is recovered and every recovered object is declared (VER-01, VER-02,
//! VER-03), and the same both-directions comparison holds for the public
//! procedure names an object's source declares (VER-04). It does not prove
//! any prototype detail beyond the name: `tests/type_descriptors.rs` covers
//! `FuncTypDesc`, argument lists and default values. It measures nothing
//! through a round trip of this project's own output: the declared side of
//! every comparison here comes from `support::vbp` (the project file) and
//! `support::source` (the original source text), never from a second call
//! into `deform6` itself.
//!
//! # Both directions, always
//!
//! Per D-02, every comparison in this file computes and asserts **both**
//! set differences: what the source declares that nothing recovered, and
//! what something recovered that the source never declared. The first
//! corpus walk in this repository looped on the object array's rounded-up
//! capacity, asserted only that the declared names were a subset of the
//! recovered names, and reported "44 of 44" while the extra capacity slots
//! silently added unexpected names. That over-count was invisible for
//! hours behind a green check, because a subset check cannot see an
//! over-count. [`fabricated_object_passes_a_subset_check_and_fails_the_two_directional_check`]
//! is the test that would have caught it.

#[path = "support/mod.rs"]
#[allow(
    dead_code,
    reason = "cargo compiles this shared module into every test binary and each binary uses a \
              different subset"
)]
mod support;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use deform6::read::pe::PeImage;
use deform6::read::region::Va;
use deform6::vb::classify::{self, ObjectKind as RecoveredKind};
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::object::{Object, ObjectTable};
use deform6::vb::privateobj::{ProcNames, Procedure, ProcedureCounts, ProcedureList};
use deform6::vb::project::{ObjectTableHead, ProjectInfo};

use support::{source, vbp};

/// The count this whole harness is built to protect: 44 vendored
/// executables. Asserted before the sweep starts, so a corpus file added
/// or removed is a loud failure rather than a silently shorter run.
const EXPECTED_EXECUTABLE_COUNT: usize = 44;

/// The declared object total, summed over all 44 programs: 53 forms, 8
/// standard modules, 44 classes.
const EXPECTED_OBJECT_TOTAL: usize = 105;
const EXPECTED_FORM_COUNT: usize = 53;
const EXPECTED_MODULE_COUNT: usize = 8;
const EXPECTED_CLASS_COUNT: usize = 44;

/// The public procedure total, summed over the objects the standard-module
/// and absent-source rules keep: 185 declared by source, 185 recovered by
/// the library, matching one for one across the corpus.
const EXPECTED_PROCEDURE_TOTAL: u32 = 185;

/// The standard-module rule excludes every one of the 8 module objects in
/// the corpus, and the 23 procedure slots they declare, from both sides.
const EXPECTED_STANDARD_MODULE_OBJECTS_EXCLUDED: usize = 8;
const EXPECTED_STANDARD_MODULE_SLOTS_EXCLUDED: u32 = 23;

/// The absent-source rule excludes exactly one object, `Edge-detection`'s
/// `cCommonDialog.cls`, from both sides.
const EXPECTED_ABSENT_SOURCE_OBJECTS_EXCLUDED: usize = 1;

/// The number of corpus programs that recover no public procedure at all:
/// every object in the program is a form or a class whose only procedures
/// are private event handlers, which is correct behaviour, not a shortfall.
const EXPECTED_ZERO_RECOVERY_PROGRAM_COUNT: usize = 13;

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks `corpus/` recursively and gives every file whose extension is
/// `.exe`, compared case-insensitively.
///
/// This is copied from `tests/corpus_sweep.rs`, not shared with it or with
/// `support::vbp::executables`. Plan 01-08 chose duplication on purpose so
/// two tests can fail independently, and this test must be able to fail
/// while the phase 1 sweep passes.
fn executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(&corpus_root(), &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("reading an entry of {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push(path);
        }
    }
}

/// Gives the address of `ProjectInfo` that the file itself holds.
fn project_data_va(image: &PeImage<'_>) -> Va {
    let hdr = header_region(image).unwrap();
    VbHeader::read(&hdr).unwrap().lp_project_data
}

/// Gives the address of the object table that the file itself holds.
fn object_table_va(image: &PeImage<'_>) -> Va {
    ProjectInfo::read(image, project_data_va(image))
        .unwrap()
        .lp_object_table
}

/// Walks the object array of one executable's bytes. This is the recovered
/// side of every comparison in this file, read through the library and
/// nothing else.
fn recovered_objects(image: &PeImage<'_>) -> Vec<Object> {
    let lp_object_table = object_table_va(image);
    let head = ObjectTableHead::read(image, lp_object_table).unwrap();
    ObjectTable::walk(image, lp_object_table, &head)
        .unwrap()
        .objects
}

/// Labels a recovered `fObjectType`'s classification the same way
/// [`declared_kind_label`] labels a `.vbp` object key, so the two can be
/// compared as plain strings.
fn recovered_kind_label(kind: RecoveredKind) -> &'static str {
    match kind {
        RecoveredKind::Form => "Form",
        RecoveredKind::Module => "Module",
        RecoveredKind::Class => "Class",
        RecoveredKind::Unknown(_) => "Unknown",
    }
}

/// Labels a `.vbp` object key the same way [`recovered_kind_label`] labels
/// a recovered `fObjectType`.
fn declared_kind_label(kind: vbp::ObjectKind) -> &'static str {
    match kind {
        vbp::ObjectKind::Form => "Form",
        vbp::ObjectKind::Module => "Module",
        vbp::ObjectKind::Class => "Class",
        vbp::ObjectKind::UserControl => "UserControl",
        vbp::ObjectKind::PropertyPage => "PropertyPage",
        vbp::ObjectKind::UserDocument => "UserDocument",
        vbp::ObjectKind::Designer => "Designer",
        vbp::ObjectKind::RelatedDoc => "RelatedDoc",
    }
}

/// Both set differences between what the source declares and what the
/// library recovered, computed together so a caller can never assert only
/// one of them by accident. Per D-02, that is the exact shape of mistake
/// this file exists to refuse.
#[derive(Debug, Clone)]
struct DirectionalDiff {
    /// Declared but never recovered: a real shortfall.
    declared_not_recovered: Vec<String>,
    /// Recovered but never declared: an over-count, the shape of mistake a
    /// subset-only check cannot see.
    recovered_not_declared: Vec<String>,
}

impl DirectionalDiff {
    fn is_empty(&self) -> bool {
        self.declared_not_recovered.is_empty() && self.recovered_not_declared.is_empty()
    }
}

/// Compares `declared` against `recovered` in both directions.
///
/// This is the one function every whole-corpus comparison in this file
/// calls, and the one function the doctored-input tests below call too.
/// Keeping the two-directional logic in one place, rather than writing the
/// comparison inline at each call site, is what makes deliberate breakage
/// 2 (below) able to cripple both call sites identically.
fn two_directional_diff(
    declared: &BTreeSet<String>,
    recovered: &BTreeSet<String>,
) -> DirectionalDiff {
    DirectionalDiff {
        declared_not_recovered: declared.difference(recovered).cloned().collect(),
        recovered_not_declared: recovered.difference(declared).cloned().collect(),
    }
}

/// The one-directional check whose blind spot this whole file exists to
/// refuse: true whenever every declared name also appears recovered, with
/// no regard for whatever else the recovered side might additionally
/// contain.
fn declared_is_a_subset_of_recovered(
    declared: &BTreeSet<String>,
    recovered: &BTreeSet<String>,
) -> bool {
    declared.is_subset(recovered)
}

#[test]
fn every_declared_object_is_recovered_and_every_recovered_object_is_declared_across_the_corpus() {
    let exes = executables();
    assert_eq!(
        exes.len(),
        EXPECTED_EXECUTABLE_COUNT,
        "found {} corpus executables, wanted {EXPECTED_EXECUTABLE_COUNT}; a file was added or \
         removed",
        exes.len()
    );

    let projects = vbp::project_files();

    let mut total_declared = 0usize;
    let mut total_recovered = 0usize;
    let mut kind_totals: HashMap<&'static str, usize> = HashMap::new();
    let mut failures = Vec::new();

    for exe in &exes {
        let data =
            std::fs::read(exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
        let image = PeImage::parse(&data)
            .unwrap_or_else(|err| panic!("{}: PeImage::parse: {err:?}", exe.display()));
        let recovered = recovered_objects(&image);

        let project_path = vbp::select_project_file(exe, &projects)
            .unwrap_or_else(|err| panic!("{}: {err}", exe.display()));
        let project = vbp::Project::read(&project_path);
        let declared = project.declared_objects();

        let declared_names: BTreeSet<String> =
            declared.iter().filter_map(|o| o.name.clone()).collect();
        let recovered_names: BTreeSet<String> = recovered.iter().map(|o| o.name.clone()).collect();

        let diff = two_directional_diff(&declared_names, &recovered_names);
        if !diff.is_empty() {
            failures.push(format!(
                "{}: declared-but-not-recovered: {:?}; recovered-but-not-declared: {:?}",
                exe.display(),
                diff.declared_not_recovered,
                diff.recovered_not_declared
            ));
        }

        total_declared += declared_names.len();
        total_recovered += recovered_names.len();

        for d in &declared {
            let Some(name) = &d.name else { continue };
            let Some(r) = recovered.iter().find(|o| &o.name == name) else {
                continue;
            };
            let recovered_kind = recovered_kind_label(classify::classify(r.f_object_type));
            let declared_kind = declared_kind_label(d.kind);
            if recovered_kind != declared_kind {
                failures.push(format!(
                    "{}: {name}: declared kind {declared_kind}, recovered kind {recovered_kind}",
                    exe.display()
                ));
            }
            *kind_totals.entry(recovered_kind).or_insert(0) += 1;
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} corpus executables disagree:\n{}",
        failures.len(),
        exes.len(),
        failures.join("\n")
    );

    assert_eq!(
        total_declared, EXPECTED_OBJECT_TOTAL,
        "the .vbp files declare {total_declared} objects across the corpus, wanted \
         {EXPECTED_OBJECT_TOTAL}"
    );
    assert_eq!(
        total_recovered, EXPECTED_OBJECT_TOTAL,
        "the library recovered {total_recovered} objects across the corpus, wanted \
         {EXPECTED_OBJECT_TOTAL}"
    );

    // Object recovery is asserted as an equality, in both directions, and
    // not pinned as a ratio anywhere in this file. It is 44 of 44 with no
    // variance across this corpus (D-11): a pinned number here could never
    // move, and AGENTS.md says a test that cannot fail is worse than none.
    // The pinned ratio plan 02-09 owns is over procedures, where the
    // variance is real.
    assert_eq!(
        kind_totals.get("Form").copied().unwrap_or(0),
        EXPECTED_FORM_COUNT
    );
    assert_eq!(
        kind_totals.get("Module").copied().unwrap_or(0),
        EXPECTED_MODULE_COUNT
    );
    assert_eq!(
        kind_totals.get("Class").copied().unwrap_or(0),
        EXPECTED_CLASS_COUNT
    );
    assert_eq!(
        kind_totals.get("Unknown").copied().unwrap_or(0),
        0,
        "an Unknown recovered kind means this corpus now carries an fObjectType this phase has \
         not measured; classify() must learn it before this assertion can move"
    );
}

/// Gives the declared and the recovered object name sets for
/// `Mandelbrot.exe`, the corpus's smallest program (one object). This is
/// real corpus data, not a synthetic literal: the doctored-input tests
/// below start from a real, already-agreeing pair and doctor one side of
/// it, which is what proves the two-directional check catches a
/// disagreement a subset check would miss on real data, not only on data
/// built to order.
fn mandelbrot_object_names() -> (BTreeSet<String>, BTreeSet<String>) {
    let exe = corpus_root().join("vb6-code/Mandelbrot/Mandelbrot.exe");
    let data = std::fs::read(&exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
    let image = PeImage::parse(&data).unwrap();
    let recovered: BTreeSet<String> = recovered_objects(&image)
        .into_iter()
        .map(|o| o.name)
        .collect();

    let projects = vbp::project_files();
    let project_path = vbp::select_project_file(&exe, &projects).unwrap();
    let project = vbp::Project::read(&project_path);
    let declared: BTreeSet<String> = project
        .declared_objects()
        .into_iter()
        .filter_map(|o| o.name)
        .collect();

    (declared, recovered)
}

#[test]
fn fabricated_object_passes_a_subset_check_and_fails_the_two_directional_check() {
    let (declared, recovered) = mandelbrot_object_names();
    // The real corpus data agrees before anything is doctored: this is
    // exactly the shape of "44 of 44" the original episode reported, on
    // real, unmodified data.
    assert_eq!(
        declared, recovered,
        "Mandelbrot.exe's real object lists must agree before either \
                                       side is doctored"
    );

    let mut doctored_recovered = recovered.clone();
    doctored_recovered.insert("fabricatedObjectNothingDeclares".to_owned());

    assert!(
        declared_is_a_subset_of_recovered(&declared, &doctored_recovered),
        "the doctored recovered set still contains every declared name, so a subset check \
         passes on it -- exactly the blind spot the original episode fell into"
    );

    let diff = two_directional_diff(&declared, &doctored_recovered);
    assert!(
        !diff.is_empty(),
        "the two-directional check must catch the fabricated name a subset check cannot see"
    );
    assert!(diff.declared_not_recovered.is_empty());
    assert_eq!(
        diff.recovered_not_declared,
        vec!["fabricatedObjectNothingDeclares".to_owned()],
        "the failure must name the one direction that actually disagrees"
    );
}

#[test]
fn removing_a_recovered_object_fails_the_two_directional_check_in_the_other_direction() {
    let (declared, recovered) = mandelbrot_object_names();
    assert_eq!(declared, recovered);

    let mut doctored_recovered = recovered.clone();
    let removed = doctored_recovered
        .iter()
        .next()
        .cloned()
        .expect("Mandelbrot.exe declares at least one object");
    doctored_recovered.remove(&removed);

    let diff = two_directional_diff(&declared, &doctored_recovered);
    assert!(!diff.is_empty());
    assert_eq!(
        diff.declared_not_recovered,
        vec![removed],
        "removing a recovered object must be caught in the declared-not-recovered direction, \
         the opposite direction from the fabricated-object test above"
    );
    assert!(diff.recovered_not_declared.is_empty());
}

// The procedure comparison: both directions, with the standard-module and
// absent-source rules applied to both sides.

/// The four numbers plan 02-09 pins and plan 02-10 prints, for one program.
///
/// This function lives here, in `differential.rs`, rather than in
/// `support/`: it must call `deform6` for the recovered side, which
/// `support::vbp` and `support::source` are forbidden from doing (D-06). A
/// future plan that needs this from a second test binary can reach it with
/// `#[path = "../differential.rs"] mod differential;`, the same mechanism
/// `support/mod.rs` already uses to share a file across binaries, pointed
/// at a file that is not literally named `mod.rs`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ProgramCounts {
    /// Procedures the library recovered as [`Procedure::Public`], over the
    /// objects the standard-module and absent-source rules keep.
    recovered: u32,
    /// Procedure slots the compiled binary's object table declares
    /// (`Object.proc_count`), over the same kept objects. This is the
    /// binary's own count, not the source's: the two agree in total across
    /// the corpus only because every slot the binary declares that the
    /// source keeps private is still counted here, and only the *recovered*
    /// side is asserted equal to the source's own public-procedure count.
    declared_by_binary: u32,
    /// Procedure slots the standard-module rule caps: the `proc_count` of
    /// every object this program declares as a `.bas` module, which has no
    /// procedure name array to recover a name through at all.
    capped_by_standard_module: u32,
    /// Objects excluded from this program's comparison: every standard
    /// module, plus one if this program carries the corpus's one
    /// absent-source object.
    objects_excluded: u32,
}

/// Computes [`ProgramCounts`] for one program, from its already-read
/// recovered objects and already-read declared objects.
///
/// Joining `declared` to `recovered` by name is the same join
/// [`every_declared_object_is_recovered_and_every_recovered_object_is_declared_across_the_corpus`]
/// performs; per D-02's own already-proven claim (105 of 105, both
/// directions), every declared object with a name finds exactly one
/// recovered match here, on the real corpus.
fn program_counts(
    image: &PeImage<'_>,
    declared: &[vbp::DeclaredObject],
    recovered: &[Object],
) -> ProgramCounts {
    let mut counts = ProgramCounts::default();
    for d in declared {
        let Some(name) = &d.name else { continue };
        let Some(r) = recovered.iter().find(|o| &o.name == name) else {
            continue;
        };
        let list = ProcedureList::read(image, r);
        let c = ProcedureCounts::of(&list.procs);

        if d.kind == vbp::ObjectKind::Module {
            counts.capped_by_standard_module += c.declared;
            counts.objects_excluded += 1;
            continue;
        }
        if !d.source_file.exists() {
            counts.objects_excluded += 1;
            continue;
        }

        counts.recovered += c.recovered;
        counts.declared_by_binary += c.declared;
    }
    counts
}

/// Gives the recovered object's public procedure names, from the library
/// alone: [`ProcedureList::read`] takes only the [`Object`] this file
/// already recovered, never `support::source`.
fn recovered_public_names(image: &PeImage<'_>, object: &Object) -> BTreeSet<String> {
    match ProcedureList::read(image, object).procs {
        ProcNames::Slots(slots) => slots
            .into_iter()
            .filter_map(|p| match p {
                Procedure::Public(name) => Some(name),
                Procedure::Private => None,
            })
            .collect(),
        ProcNames::NoNameArray { .. } => BTreeSet::new(),
    }
}

#[test]
fn every_declared_public_procedure_is_recovered_and_every_recovered_one_is_declared_across_the_corpus()
 {
    let exes = executables();
    let projects = vbp::project_files();

    let mut total_recovered = 0u32;
    let mut total_declared_by_source = 0u32;
    let mut total_capped = 0u32;
    let mut standard_module_objects = 0usize;
    let mut absent_source_objects = 0usize;
    let mut zero_recovery_programs = Vec::new();
    let mut failures = Vec::new();

    for exe in &exes {
        let data =
            std::fs::read(exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
        let image = PeImage::parse(&data)
            .unwrap_or_else(|err| panic!("{}: PeImage::parse: {err:?}", exe.display()));
        let recovered = recovered_objects(&image);

        let project_path = vbp::select_project_file(exe, &projects)
            .unwrap_or_else(|err| panic!("{}: {err}", exe.display()));
        let project = vbp::Project::read(&project_path);
        let declared = project.declared_objects();

        let counts = program_counts(&image, &declared, &recovered);
        total_recovered += counts.recovered;
        total_capped += counts.capped_by_standard_module;
        if counts.recovered == 0 {
            zero_recovery_programs.push(exe.display().to_string());
        }

        for d in &declared {
            let Some(name) = &d.name else { continue };
            if d.kind == vbp::ObjectKind::Module {
                standard_module_objects += 1;
                continue;
            }
            if !d.source_file.exists() {
                absent_source_objects += 1;
                continue;
            }
            let Some(r) = recovered.iter().find(|o| &o.name == name) else {
                failures.push(format!(
                    "{}: {name}: declared but not recovered at all (the object comparison \
                     above should already have caught this)",
                    exe.display()
                ));
                continue;
            };

            let recovered_names = recovered_public_names(&image, r);
            let declared_names: BTreeSet<String> =
                source::declared_public_procedures(&d.source_file)
                    .into_iter()
                    .collect();

            let diff = two_directional_diff(&declared_names, &recovered_names);
            if !diff.is_empty() {
                failures.push(format!(
                    "{}: {name}: declared-not-recovered: {:?}; recovered-not-declared: {:?}",
                    exe.display(),
                    diff.declared_not_recovered,
                    diff.recovered_not_declared
                ));
            }
            total_declared_by_source += u32::try_from(declared_names.len()).unwrap_or(u32::MAX);
        }
    }

    assert!(
        failures.is_empty(),
        "{} object/program disagreements over their procedure names:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        total_recovered, EXPECTED_PROCEDURE_TOTAL,
        "the library recovered {total_recovered} public procedure names over the objects the \
         rules keep, wanted {EXPECTED_PROCEDURE_TOTAL}"
    );
    assert_eq!(
        total_declared_by_source, EXPECTED_PROCEDURE_TOTAL,
        "the source declares {total_declared_by_source} public procedures over the objects the \
         rules keep, wanted {EXPECTED_PROCEDURE_TOTAL}"
    );
    assert_eq!(
        standard_module_objects, EXPECTED_STANDARD_MODULE_OBJECTS_EXCLUDED,
        "the standard-module rule must exclude exactly {EXPECTED_STANDARD_MODULE_OBJECTS_EXCLUDED} \
         objects"
    );
    assert_eq!(
        total_capped, EXPECTED_STANDARD_MODULE_SLOTS_EXCLUDED,
        "the standard-module rule must cap exactly {EXPECTED_STANDARD_MODULE_SLOTS_EXCLUDED} \
         procedure slots"
    );
    assert_eq!(
        absent_source_objects, EXPECTED_ABSENT_SOURCE_OBJECTS_EXCLUDED,
        "the absent-source rule must exclude exactly {EXPECTED_ABSENT_SOURCE_OBJECTS_EXCLUDED} \
         object"
    );

    zero_recovery_programs.sort();
    println!(
        "{} of {} programs recovered no public procedure at all:",
        zero_recovery_programs.len(),
        exes.len()
    );
    for program in &zero_recovery_programs {
        println!("  {program}");
    }
    assert_eq!(
        zero_recovery_programs.len(),
        EXPECTED_ZERO_RECOVERY_PROGRAM_COUNT,
        "expected exactly {EXPECTED_ZERO_RECOVERY_PROGRAM_COUNT} programs to recover no public \
         procedure at all; found {}: {zero_recovery_programs:?}",
        zero_recovery_programs.len()
    );
}

#[test]
fn program_counts_gives_the_four_pinned_numbers_for_grayscale() {
    // Grayscale-effect: 12 of 34 recovered, per CONTEXT.md's own worked
    // example. Three objects, no standard module, no absent source: the
    // four numbers reduce to the two everyone already knows.
    let exe = corpus_root().join("vb6-code/Grayscale-effect/Grayscale.exe");
    let data = std::fs::read(&exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
    let image = PeImage::parse(&data).unwrap();
    let recovered = recovered_objects(&image);

    let projects = vbp::project_files();
    let project_path = vbp::select_project_file(&exe, &projects).unwrap();
    let project = vbp::Project::read(&project_path);
    let declared = project.declared_objects();

    let counts = program_counts(&image, &declared, &recovered);
    assert_eq!(
        counts,
        ProgramCounts {
            recovered: 12,
            declared_by_binary: 34,
            capped_by_standard_module: 0,
            objects_excluded: 0,
        },
        "Grayscale-effect declares no standard module and no absent-source object; its four \
         numbers should be exactly the 12 of 34 CONTEXT.md's worked example gives, with the \
         other two at zero"
    );
}

#[test]
fn removing_the_absent_source_object_from_only_the_declared_side_makes_recovered_exceed_declared() {
    let exe = corpus_root().join("vb6-code/Edge-detection/Edge_Detection.exe");
    let data = std::fs::read(&exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
    let image = PeImage::parse(&data).unwrap();
    let recovered = recovered_objects(&image);

    let projects = vbp::project_files();
    let project_path = vbp::select_project_file(&exe, &projects).unwrap();
    let project = vbp::Project::read(&project_path);
    let declared = project.declared_objects();

    let missing_source: Vec<&vbp::DeclaredObject> = declared
        .iter()
        .filter(|o| o.kind != vbp::ObjectKind::Module && !o.source_file.exists())
        .collect();
    assert_eq!(
        missing_source.len(),
        1,
        "Edge-detection must carry exactly one non-module object whose source this repository \
         does not hold"
    );
    let absent_name = missing_source[0].name.clone();
    assert!(
        recovered
            .iter()
            .any(|r| Some(&r.name) == absent_name.as_ref()),
        "the compiler still built the absent-source object into the binary; only its source is \
         missing, so the recovered side must still carry it"
    );

    let recovered_total = recovered.len();
    let declared_total = declared.len();
    assert_eq!(
        recovered_total, declared_total,
        "Edge-detection carries no standard module, so \
                                                   before either rule is applied the two totals \
                                                   already agree (4 objects)"
    );

    // Correctly excluded from both sides, per the absent-source rule.
    let declared_both_sides = declared_total - 1;
    let recovered_both_sides = recovered_total - 1;
    assert_eq!(declared_both_sides, recovered_both_sides);

    // The shape of wrong answer this harness exists to refuse: excluding
    // the absent-source object from the declared side only. The recovered
    // side (the real compiled binary) still carries it, because the
    // compiler built it in; only the source text is missing.
    let declared_one_sided = declared_total - 1;
    let recovered_one_sided = recovered_total;
    assert!(
        recovered_one_sided > declared_one_sided,
        "excluding an object from the declared side only must make the recovered total exceed \
         the declared total: {recovered_one_sided} > {declared_one_sided}"
    );
}

#[test]
fn a_fabricated_procedure_name_passes_a_subset_check_and_fails_the_two_directional_check() {
    // FastDrawing.cls: four real public procedures, GetImageWidth,
    // GetImageHeight, GetImageData2D and SetImageData2D. Real data, agreeing
    // before anything is doctored, for the same reason
    // `mandelbrot_object_names` gives real data at the object layer: the
    // proof matters on data that actually agrees first.
    let exe = corpus_root().join("vb6-code/Grayscale-effect/Grayscale.exe");
    let data = std::fs::read(&exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
    let image = PeImage::parse(&data).unwrap();
    let recovered = recovered_objects(&image);
    let fast_drawing = recovered
        .iter()
        .find(|o| o.name == "FastDrawing")
        .expect("Grayscale.exe recovers an object named FastDrawing");
    let recovered_names = recovered_public_names(&image, fast_drawing);

    let source_path = corpus_root().join("vb6-code/Grayscale-effect/FastDrawing.cls");
    let declared_names: BTreeSet<String> = source::declared_public_procedures(&source_path)
        .into_iter()
        .collect();

    assert_eq!(
        declared_names, recovered_names,
        "FastDrawing.cls's real declared and recovered procedure names must agree before \
         either side is doctored"
    );

    let mut doctored_recovered = recovered_names.clone();
    doctored_recovered.insert("fabricatedProcedureNothingDeclares".to_owned());

    assert!(
        declared_is_a_subset_of_recovered(&declared_names, &doctored_recovered),
        "the doctored recovered set still contains every declared name, so a subset check \
         passes on it"
    );

    let diff = two_directional_diff(&declared_names, &doctored_recovered);
    assert!(
        !diff.is_empty(),
        "the two-directional check must catch the fabricated procedure name a subset check \
         cannot see"
    );
    assert!(diff.declared_not_recovered.is_empty());
    assert_eq!(
        diff.recovered_not_declared,
        vec!["fabricatedProcedureNothingDeclares".to_owned()]
    );
}
