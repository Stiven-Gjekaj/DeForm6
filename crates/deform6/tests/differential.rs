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

// `pub(crate)`, not private: `tests/ratios.rs` (plan 02-09) reaches this
// module, and every item it needs from it, through the same `#[path]`
// embedding that brings in `program_counts` below -- a private module here
// would block that access outright, since a private item is invisible to
// the parent module an embedding `#[path]` creates.
#[path = "support/mod.rs"]
#[allow(
    dead_code,
    reason = "cargo compiles this shared module into every test binary and each binary uses a \
              different subset"
)]
pub(crate) mod support;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use deform6::Report;
use deform6::read::pe::PeImage;
use deform6::read::region::Va;
use deform6::vb::classify::{self, ObjectKind as RecoveredKind};
use deform6::vb::controltree::ControlKind;
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::object::{Object, ObjectTable};
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::privateobj::{ProcNames, Procedure, ProcedureCounts, ProcedureList};
use deform6::vb::project::{ObjectTableHead, ProjectInfo};
use deform6::vb::propstream::PropertyValue;
use deform6::vb::{ControlReport, FormReport};

use support::{frm, source, vbp};

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
///
/// `pub(crate)`: `tests/ratios.rs` (plan 02-09) reaches this through the
/// same `#[path]` embedding as [`program_counts`], so it never opens a
/// second, independent path to the same object array.
pub(crate) fn recovered_objects(image: &PeImage<'_>) -> Vec<Object> {
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
///
/// `pub(crate)`, not private: plan 02-09's `tests/ratios.rs` and
/// `crates/xtask` both reach this exact function through that `#[path]`
/// mechanism, per T-02-45 ("both take their counts from the one function
/// plan 02-08 wrote"), and a private item is invisible to the parent module
/// an embedding `#[path]` creates. No behaviour changes; only the
/// visibility widens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProgramCounts {
    /// Procedures the library recovered as [`Procedure::Public`], over the
    /// objects the standard-module and absent-source rules keep.
    pub(crate) recovered: u32,
    /// Procedure slots the compiled binary's object table declares
    /// (`Object.proc_count`), over the same kept objects. This is the
    /// binary's own count, not the source's: the two agree in total across
    /// the corpus only because every slot the binary declares that the
    /// source keeps private is still counted here, and only the *recovered*
    /// side is asserted equal to the source's own public-procedure count.
    pub(crate) declared_by_binary: u32,
    /// Procedure slots the standard-module rule caps: the `proc_count` of
    /// every object this program declares as a `.bas` module, which has no
    /// procedure name array to recover a name through at all.
    pub(crate) capped_by_standard_module: u32,
    /// Objects excluded from this program's comparison: every standard
    /// module, plus one if this program carries the corpus's one
    /// absent-source object.
    pub(crate) objects_excluded: u32,
}

/// Computes [`ProgramCounts`] for one program, from its already-read
/// recovered objects and already-read declared objects.
///
/// Joining `declared` to `recovered` by name is the same join
/// [`every_declared_object_is_recovered_and_every_recovered_object_is_declared_across_the_corpus`]
/// performs; per D-02's own already-proven claim (105 of 105, both
/// directions), every declared object with a name finds exactly one
/// recovered match here, on the real corpus.
pub(crate) fn program_counts(
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

// The forms and controls comparison: both directions, against `support::frm`,
// the second, independent `.frm` reader, never against `deform6`'s own
// output.

/// One declared form: its own compiled name (the same name
/// [`program_counts`] already joins recovered objects to) and its root
/// `Begin VB.Form`/`VB.MDIForm` block, read through `support::frm`.
struct DeclaredForm {
    name: String,
    root: frm::Block,
}

/// Reads every form `declared` names, through `support::frm`. A `Form=`
/// line whose own compiled name did not resolve (no attribute, no fallback
/// prefix) is skipped: there is nothing to join it to on the recovered
/// side. `support/frm.rs`'s own whole-corpus test already proves one root
/// form block per `.frm` file (54 across 54 files), so the first root
/// block is the form.
fn declared_forms(declared: &[vbp::DeclaredObject]) -> Vec<DeclaredForm> {
    declared
        .iter()
        .filter(|d| d.kind == vbp::ObjectKind::Form && d.source_file.exists())
        .filter_map(|d| {
            let name = d.name.clone()?;
            let form = frm::Form::read(&d.source_file);
            let root = form.blocks().into_iter().next()?;
            Some(DeclaredForm { name, root })
        })
        .collect()
}

/// One declared control, flattened from a declared form's own block tree in
/// depth-first order. Index `0` of the flattened list (the form's own root
/// block) is the form itself, matching `Report::forms`'s own
/// `FormReport::controls[0]`.
struct DeclaredControl {
    name: String,
    class: String,
    array_index: Option<u16>,
    properties: Vec<frm::Property>,
}

/// Flattens `root` (and every descendant) into `out`, in the same
/// depth-first order `deform6::vb::controltree::walk` gives the recovered
/// side.
fn flatten_declared(root: &frm::Block, out: &mut Vec<DeclaredControl>) {
    let array_index = root
        .properties
        .iter()
        .find(|p| p.name == "Index")
        .and_then(|p| p.value.parse::<u16>().ok());
    out.push(DeclaredControl {
        name: root.name.clone(),
        class: root.class.clone(),
        array_index,
        properties: root.properties.clone(),
    });
    for child in &root.children {
        flatten_declared(child, out);
    }
}

/// The comparison key for one control: its own name, plus its array index
/// when it has one, because a control array's elements share one name.
fn control_key(name: &str, array_index: Option<u16>) -> String {
    match array_index {
        Some(index) => format!("{name}[{index}]"),
        None => name.to_owned(),
    }
}

/// Tells whether a recovered control's own type agrees with the class the
/// `.frm` source's own `Begin` line names.
///
/// The recovered `cType` name maps to the class the `Begin` line names,
/// with the `VB.` prefix removed: a `cType` of `4` gives `CommandButton`
/// and the source line reads `Begin VB.CommandButton`. An external
/// control's class name is compared whole, because its own `Begin` line
/// reads the full programmatic class name and carries no `VB.` prefix.
fn declared_class_name_matches(
    declared_class: &str,
    kind: &ControlKind,
    external_class_name: Option<&str>,
) -> bool {
    if matches!(kind, ControlKind::External) {
        return external_class_name.is_some_and(|class| class == declared_class);
    }
    let stripped = declared_class.strip_prefix("VB.").unwrap_or(declared_class);
    stripped == format!("{kind:?}")
}

/// Gives the property name a recovered [`PropertyValue`] carries, or `None`
/// for [`PropertyValue::Undecoded`], which names no property at all.
fn recovered_property_name(value: &PropertyValue) -> Option<&str> {
    match value {
        PropertyValue::Byte { name, .. }
        | PropertyValue::Boolean { name, .. }
        | PropertyValue::Integer { name, .. }
        | PropertyValue::Long { name, .. }
        | PropertyValue::Single { name, .. }
        | PropertyValue::Text { name, .. }
        | PropertyValue::Position { name, .. }
        | PropertyValue::Font { name, .. } => Some(name.as_str()),
        PropertyValue::Undecoded { .. } => None,
    }
}

/// Gives the text a recovered [`PropertyValue`] would carry in a `.frm`
/// file, for the payload shapes a `.frm` writes as one plain `Name = Value`
/// line: `Byte`, `Boolean`, `Integer`, `Long`, `Single` and `Text`.
///
/// `Position` and `Font` are excluded on purpose: a `.frm` file writes a
/// position as four separate `Left`/`Top`/`Width`/`Height` lines and a font
/// as a nested `BeginProperty Font` block, never as one line named
/// `Position` or `Font`, so no declared property of either name ever exists
/// to compare against; this function's own `None` is what lets the caller
/// skip them by construction rather than by a special case. `Undecoded`
/// gives `None` for the same reason `PropertyValue::undecoded_message`
/// documents: this repository does not decode it, so there is nothing to
/// compare, and it is never asked to guess.
///
/// `Long` renders as a `.frm` colour literal, `&H{bits:08X}&`, never as a
/// plain signed decimal. `BackColor` (opcode 3, plan 03-13) is the only
/// `Long`-typed row this table carries: a `.frm` writes `BackColor =
/// &H80000005&`, and the recovered value is the same 32 bits read as a
/// signed `i32` (`-2147483643`). The two texts would differ while the value
/// is identical, so this puts both sides in the one form a `.frm` actually
/// writes, per this plan's own hazard note. The bits themselves are never
/// changed: `value as u32` is a bit-for-bit reinterpretation, not an
/// arithmetic conversion, so a value that genuinely differs still renders a
/// different text and the comparison can still fail.
fn recovered_property_text(value: &PropertyValue) -> Option<String> {
    match value {
        PropertyValue::Byte { value, .. } => Some(value.to_string()),
        PropertyValue::Boolean { value, .. } => Some(value.to_string()),
        PropertyValue::Integer { value, .. } => Some(value.to_string()),
        PropertyValue::Long { value, .. } => Some(format!("&H{:08X}&", (*value).cast_unsigned())),
        PropertyValue::Single { value, .. } => Some(value.to_string()),
        PropertyValue::Text { value, .. } => Some(value.clone()),
        PropertyValue::Position { .. }
        | PropertyValue::Font { .. }
        | PropertyValue::Undecoded { .. } => None,
    }
}

/// Strips a trailing `'...` VB6 comment from an unquoted `.frm` property
/// value, the same way `WindowState = 1  'Minimized` reads as `1` once its
/// own inline comment is dropped.
///
/// Never applied to a `Text` property's own declared value: that value has
/// already been quote-stripped by `support::frm::split_property_line`, and
/// a genuine apostrophe inside a caption is real text, not a comment
/// marker. The caller only reaches this function for a numeric-payload
/// property, where a trailing `'comment` can never be part of the value
/// itself.
fn strip_frm_comment(value: &str) -> &str {
    value.split('\'').next().unwrap_or(value).trim()
}

/// Tells whether a declared `.frm` value names a resource file rather than
/// carrying a literal: `"name.frx":OFFSET`, or `$"name.frx":OFFSET` for a
/// Unicode-flagged resource reference. `support::frm::split_property_line`
/// only strips a leading and trailing double quote when both are present;
/// this shape's own trailing digits mean the leading quote survives into
/// the value this function reads, which is what the check below looks for.
///
/// The corpus proves this shape is real, not a hypothetical: `Gradient.frm`
/// declares `lblExplanation.Caption = $"Gradient.frx":0000`. No corpus
/// Form's own `Caption` (this plan's new opcode 1 row) happens to use it,
/// but a `Text`-typed row on another control type could reach this same
/// shape in a later plan, and this check is written to hold for that case
/// too, not only for the one this corpus happens to exercise today.
fn declared_value_names_a_resource_file(value: &str) -> bool {
    let value = value.strip_prefix('$').unwrap_or(value);
    value.starts_with('"') && value.contains(".frx\":")
}

/// `declared_value_names_a_resource_file` on the two real shapes the corpus
/// carries (`Gradient.frm`'s own `$"Gradient.frx":0000`, and every plain
/// `"name.frx":OFFSET` line the earlier grep found), and on a plain literal
/// caption that must never be mistaken for one.
#[test]
fn declared_value_names_a_resource_file_recognises_both_frx_shapes_and_not_a_plain_literal() {
    assert!(declared_value_names_a_resource_file(
        "$\"Gradient.frx\":0000"
    ));
    assert!(declared_value_names_a_resource_file("\"frmFire.frx\":0000"));
    assert!(!declared_value_names_a_resource_file(
        "Even Faster Real-Time Fire Effect - www.tannerhelland.com"
    ));
    assert!(!declared_value_names_a_resource_file("&H80000005&"));
}

/// Both directions of one form's own control diff: every declared control
/// name (plus array index) with no matching recovered control, and every
/// recovered one with no matching declared control.
fn diff_controls(declared: &[DeclaredControl], recovered: &[ControlReport]) -> DirectionalDiff {
    let declared_keys: BTreeSet<String> = declared
        .iter()
        .map(|c| control_key(&c.name, c.array_index))
        .collect();
    let recovered_keys: BTreeSet<String> = recovered
        .iter()
        .map(|c| control_key(&c.name, c.array_index))
        .collect();
    two_directional_diff(&declared_keys, &recovered_keys)
}

/// The four counts `tests/ratios.toml` pins for one program: how many forms
/// the source declares and how many the tool recovered a real tree for, and
/// the same pair for controls, summed over every form whose own tree the
/// tool built.
///
/// A form whose own tree the tool could not build (`FormReport::controls`
/// is empty; see `vb/mod.rs::compose_form`) still counts toward
/// `form_declared`, because the source does declare it, and it is excluded
/// from `control_declared`/`control_recovered` both: its own controls are
/// already the reason `form_recovered` is short one, and counting them a
/// second time as missing controls would double the same shortfall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FormsControlsCounts {
    pub(crate) form_declared: u32,
    pub(crate) form_recovered: u32,
    pub(crate) control_declared: u32,
    pub(crate) control_recovered: u32,
}

/// Computes [`FormsControlsCounts`] for one program, from its already-read
/// declared objects and its already-built [`Report`].
pub(crate) fn forms_controls_counts(
    declared: &[vbp::DeclaredObject],
    report: &Report,
) -> FormsControlsCounts {
    let mut counts = FormsControlsCounts::default();
    for declared_form in declared_forms(declared) {
        counts.form_declared = counts.form_declared.saturating_add(1);
        let Some(recovered_form) = report.forms.iter().find(|f| f.name == declared_form.name)
        else {
            continue;
        };
        if recovered_form.controls.is_empty() {
            continue;
        }
        counts.form_recovered = counts.form_recovered.saturating_add(1);

        let mut declared_controls = Vec::new();
        flatten_declared(&declared_form.root, &mut declared_controls);
        counts.control_declared = counts
            .control_declared
            .saturating_add(u32::try_from(declared_controls.len()).unwrap_or(u32::MAX));
        counts.control_recovered = counts
            .control_recovered
            .saturating_add(u32::try_from(recovered_form.controls.len()).unwrap_or(u32::MAX));
    }
    counts
}

/// Gives every form-level and control-level disagreement between `declared`
/// and `report.forms`, in both directions, for one program: a declared form
/// or control the tool did not recover, and a recovered one the source does
/// not declare. Also checks, for every matched control pair, that the
/// recovered type agrees with the declared class and that every recovered,
/// named, plain-line property this repository decoded agrees with the
/// declared text.
pub(crate) fn form_control_mismatches(
    declared: &[vbp::DeclaredObject],
    report: &Report,
) -> Vec<String> {
    let mut failures = Vec::new();

    let forms = declared_forms(declared);
    let declared_names: BTreeSet<String> = forms.iter().map(|f| f.name.clone()).collect();
    let recovered_names: BTreeSet<String> = report.forms.iter().map(|f| f.name.clone()).collect();
    let form_diff = two_directional_diff(&declared_names, &recovered_names);
    if !form_diff.is_empty() {
        failures.push(format!(
            "forms declared-but-not-recovered: {:?}; recovered-but-not-declared: {:?}",
            form_diff.declared_not_recovered, form_diff.recovered_not_declared
        ));
    }

    for declared_form in &forms {
        let Some(recovered_form) = report.forms.iter().find(|f| f.name == declared_form.name)
        else {
            continue;
        };
        if recovered_form.controls.is_empty() {
            continue;
        }

        let mut declared_controls = Vec::new();
        flatten_declared(&declared_form.root, &mut declared_controls);

        let control_diff = diff_controls(&declared_controls, &recovered_form.controls);
        if !control_diff.is_empty() {
            failures.push(format!(
                "{}: controls declared-but-not-recovered: {:?}; recovered-but-not-declared: {:?}",
                declared_form.name,
                control_diff.declared_not_recovered,
                control_diff.recovered_not_declared
            ));
        }

        for declared_control in &declared_controls {
            let Some(recovered_control) = recovered_form.controls.iter().find(|c| {
                c.name == declared_control.name && c.array_index == declared_control.array_index
            }) else {
                continue;
            };

            let external_class_name = recovered_control
                .external
                .as_ref()
                .map(|external| external.class_name.as_str());
            if !declared_class_name_matches(
                &declared_control.class,
                &recovered_control.kind,
                external_class_name,
            ) {
                failures.push(format!(
                    "{}.{}: declared class {:?}, recovered kind {:?}",
                    declared_form.name,
                    declared_control.name,
                    declared_control.class,
                    recovered_control.kind
                ));
            }

            for recovered_property in &recovered_control.properties {
                let Some(name) = recovered_property_name(recovered_property) else {
                    continue;
                };
                let Some(recovered_text) = recovered_property_text(recovered_property) else {
                    continue;
                };
                let Some(declared_property) =
                    declared_control.properties.iter().find(|p| p.name == name)
                else {
                    continue;
                };

                if matches!(recovered_property, PropertyValue::Text { .. })
                    && declared_value_names_a_resource_file(&declared_property.value)
                {
                    // A declared value of the form `"name.frx":OFFSET` or
                    // `$"name.frx":OFFSET` names a resource file, not a
                    // literal (the corpus proves this shape reaches a
                    // `Text`-typed row: `Gradient.frm`'s own `lblExplanation.
                    // Caption` reads `$"Gradient.frx":0000`). There is no
                    // literal on the declared side to compare against, so
                    // this property's text comparison is skipped here, with
                    // the reason recorded beside the skip. The property
                    // itself is still counted as recovered: the match above
                    // already found it by name, and only this one text
                    // check is what does not run.
                    continue;
                }

                let declared_text = if matches!(recovered_property, PropertyValue::Text { .. }) {
                    declared_property.value.as_str()
                } else {
                    strip_frm_comment(&declared_property.value)
                };
                if declared_text != recovered_text {
                    failures.push(format!(
                        "{}.{}.{name}: declared {:?}, recovered {:?}",
                        declared_form.name,
                        declared_control.name,
                        declared_property.value,
                        recovered_text
                    ));
                }
            }
        }
    }

    failures
}

// --- Plan 03-13, Task 2: the Form colour row and an honest colour ---------
// --- comparison ------------------------------------------------------------

/// `recovered_property_text` renders a `Long` value the way a `.frm` writes
/// a colour: `&H80000005&`, matching `frmFire.frm` line 4 bit for bit, not
/// the plain signed decimal `-2147483643` the raw `i32` would otherwise
/// print as.
#[test]
fn recovered_property_text_renders_a_long_value_as_the_frm_colour_literal_shape() {
    let value = PropertyValue::Long {
        name: "BackColor".to_owned(),
        value: 0x8000_0005_u32.cast_signed(),
    };
    assert_eq!(
        recovered_property_text(&value).as_deref(),
        Some("&H80000005&")
    );
}

/// The rendering never collapses two distinct values to the same text: a
/// value one greater than `frmFire.frm`'s own declared colour still renders
/// a different literal, so the comparison this rendering feeds can still
/// fail. `AGENTS.md`: "when you add a test, break the thing it covers on
/// purpose and watch it fail" -- this is the instrument that would catch a
/// rendering bug collapsing every colour to one canonical text.
#[test]
fn a_recovered_colour_one_greater_than_the_declared_value_still_renders_a_different_text() {
    let declared = "&H80000005&";
    let matching = PropertyValue::Long {
        name: "BackColor".to_owned(),
        value: 0x8000_0005_u32.cast_signed(),
    };
    assert_eq!(
        recovered_property_text(&matching).as_deref(),
        Some(declared)
    );

    let differing = PropertyValue::Long {
        name: "BackColor".to_owned(),
        value: 0x8000_0006_u32.cast_signed(),
    };
    assert_ne!(
        recovered_property_text(&differing).as_deref(),
        Some(declared),
        "a colour that differs by one must still render a different text"
    );
}

/// The full pipeline, not only the rendering function in isolation:
/// doctoring `frmFire`'s own recovered `BackColor` one greater than
/// `frmFire.frm`'s own declared `&H80000005&` still fails
/// `form_control_mismatches`, naming `BackColor` in the failure. Proves the
/// corpus-wide gate this plan wires the rendering into can still fail, not
/// only the rendering helper it calls.
#[test]
fn a_doctored_back_color_one_greater_than_the_declared_value_fails_the_full_comparison() {
    let exe = corpus_root().join("vb6-code/Fire-effect/Fast_Flames.exe");
    let data = std::fs::read(&exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
    let table = OpcodeTable::builtin();
    let report = deform6::inspect(&data, &table).unwrap();

    let mut doctored_report = report;
    let form = doctored_report
        .forms
        .iter_mut()
        .find(|f| f.name == "frmFire")
        .expect("Fast_Flames.exe declares a form named frmFire");
    let root = form
        .controls
        .first_mut()
        .expect("frmFire's own tree must resolve for this test to doctor it");
    let back_color = root
        .properties
        .iter_mut()
        .find_map(|p| match p {
            PropertyValue::Long { name, value } if name == "BackColor" => Some(value),
            _ => None,
        })
        .expect("frmFire's own BackColor must resolve for this test to doctor it");
    *back_color = back_color
        .checked_add(1)
        .expect("frmFire's own BackColor is nowhere near i32::MAX");

    let projects = vbp::project_files();
    let project_path = vbp::select_project_file(&exe, &projects).unwrap();
    let project = vbp::Project::read(&project_path);
    let declared = project.declared_objects();

    let failures = form_control_mismatches(&declared, &doctored_report);
    assert!(
        failures.iter().any(|f| f.contains("BackColor")),
        "a BackColor doctored one greater than the declared value must still fail the \
         comparison: {failures:?}"
    );
}

#[test]
fn every_declared_form_and_control_is_recovered_and_every_recovered_one_is_declared_across_the_corpus()
 {
    let exes = executables();
    let projects = vbp::project_files();
    let table = OpcodeTable::builtin();

    let mut totals = FormsControlsCounts::default();
    let mut failures = Vec::new();

    for exe in &exes {
        let data =
            std::fs::read(exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
        let report = deform6::inspect(&data, &table)
            .unwrap_or_else(|err| panic!("{}: inspect refused the file: {err}", exe.display()));

        let project_path = vbp::select_project_file(exe, &projects)
            .unwrap_or_else(|err| panic!("{}: {err}", exe.display()));
        let project = vbp::Project::read(&project_path);
        let declared = project.declared_objects();

        let program_failures = form_control_mismatches(&declared, &report);
        for failure in program_failures {
            failures.push(format!("{}: {failure}", exe.display()));
        }

        let counts = forms_controls_counts(&declared, &report);
        totals.form_declared = totals.form_declared.saturating_add(counts.form_declared);
        totals.form_recovered = totals.form_recovered.saturating_add(counts.form_recovered);
        totals.control_declared = totals
            .control_declared
            .saturating_add(counts.control_declared);
        totals.control_recovered = totals
            .control_recovered
            .saturating_add(counts.control_recovered);
    }

    assert!(
        failures.is_empty(),
        "{} form/control disagreement(s) across the corpus:\n{}",
        failures.len(),
        failures.join("\n")
    );

    assert_eq!(
        totals.form_declared, EXPECTED_FORM_RECOVERY_TOTAL,
        "the source declares {} forms across the corpus, wanted {EXPECTED_FORM_RECOVERY_TOTAL}",
        totals.form_declared
    );
    println!(
        "forms: {} of {} recovered; controls: {} of {} recovered",
        totals.form_recovered,
        totals.form_declared,
        totals.control_recovered,
        totals.control_declared
    );
}

/// The total corpus form count this differential relies on: 53, one short
/// of `support/frm.rs`'s own whole-corpus, filesystem-walk total of 54
/// (`the_whole_corpus_holds_fifty_four_root_form_blocks`).
///
/// The gap is the same orphaned project `support/vbp.rs`'s own doc comment
/// already names: `Brightness-effect/Part 3 - DIBs/Brightness3.vbp`
/// declares `dibBrightness.exe`, which this repository does not vendor.
/// `[VERIFIED: local]` this session: the corpus's 44 `.vbp` files that do
/// have a matching executable declare 54 `Form=` lines in total (matching
/// the 54 files on disk), but this comparison walks executables, not raw
/// `.vbp` files, per executable via `vbp::select_project_file`, so the one
/// form the orphaned project alone declares is never reached: there is no
/// compiled binary for `deform6::inspect` to recover it from.
const EXPECTED_FORM_RECOVERY_TOTAL: u32 = 53;

/// Fabricating a recovered form nothing declares passes a subset check and
/// fails the two-directional check, the same shape
/// [`fabricated_object_passes_a_subset_check_and_fails_the_two_directional_check`]
/// already proves at the object layer.
#[test]
fn a_fabricated_form_fails_the_two_directional_check_and_names_it() {
    let exe = corpus_root().join("vb6-code/Mandelbrot/Mandelbrot.exe");
    let data = std::fs::read(&exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
    let table = OpcodeTable::builtin();
    let report = deform6::inspect(&data, &table).unwrap();

    let mut doctored = report.forms.clone();
    doctored.push(FormReport {
        name: "fabricatedFormNothingDeclares".to_owned(),
        controls: Vec::new(),
        defects: Vec::new(),
    });
    let mut doctored_report = report;
    doctored_report.forms = doctored;

    let projects = vbp::project_files();
    let project_path = vbp::select_project_file(&exe, &projects).unwrap();
    let project = vbp::Project::read(&project_path);
    let declared = project.declared_objects();

    let failures = form_control_mismatches(&declared, &doctored_report);
    assert!(
        failures
            .iter()
            .any(|f| f.contains("fabricatedFormNothingDeclares")),
        "a form nothing declares must be named in the failure list: {failures:?}"
    );
}

/// Removing a recovered form the source does declare fails the two
/// directional check in the other direction, and names it.
#[test]
fn removing_a_recovered_form_fails_the_two_directional_check_in_the_other_direction() {
    let exe = corpus_root().join("vb6-code/Mandelbrot/Mandelbrot.exe");
    let data = std::fs::read(&exe).unwrap_or_else(|err| panic!("reading {}: {err}", exe.display()));
    let table = OpcodeTable::builtin();
    let report = deform6::inspect(&data, &table).unwrap();
    assert_eq!(
        report.forms.len(),
        1,
        "Mandelbrot.exe must declare exactly one form for this test to remove it"
    );
    let removed_name = report.forms[0].name.clone();

    let mut doctored_report = report;
    doctored_report.forms.clear();

    let projects = vbp::project_files();
    let project_path = vbp::select_project_file(&exe, &projects).unwrap();
    let project = vbp::Project::read(&project_path);
    let declared = project.declared_objects();

    let failures = form_control_mismatches(&declared, &doctored_report);
    assert!(
        failures.iter().any(|f| f.contains(&removed_name)),
        "the missing form {removed_name:?} must be named in the failure list: {failures:?}"
    );
}

/// VER-06 / D-04, proven again at this gate's own level, not only inside
/// `support::frm`'s own self-tests (`support_selftest.rs`): `EXCLUSIONS`
/// must still name exactly the one known upstream defect, `frmHMM.frx`,
/// and `frmHMM.frm` beside it must never be excluded. If a future edit
/// emptied `EXCLUSIONS`, `frmHMM.frx`'s own corrupted bytes would need to
/// be treated as real evidence somewhere this repository reads a `.frx`
/// file, which is the fault this exclusion exists to prevent.
#[test]
fn the_frm_hmm_frx_exclusion_still_names_the_one_upstream_defect() {
    assert_eq!(
        frm::EXCLUSIONS.len(),
        1,
        "support::frm::EXCLUSIONS holds {} entries; this differential gate expects exactly the \
         one named exclusion, corpus/vb6-code/Hidden-Markov-model/frmHMM.frx, or a future .frx \
         comparison would need to treat that file's own corrupted bytes as real evidence",
        frm::EXCLUSIONS.len()
    );
    assert!(
        frm::EXCLUSIONS[0].path.ends_with("frmHMM.frx"),
        "the one exclusion this gate relies on must name frmHMM.frx, found {:?}",
        frm::EXCLUSIONS[0].path
    );

    let frm_path = corpus_root().join("vb6-code/Hidden-Markov-model/frmHMM.frm");
    assert!(
        frm::excluded_reason(&frm_path).is_none(),
        "frmHMM.frm itself must never be excluded, only frmHMM.frx beside it"
    );
}
