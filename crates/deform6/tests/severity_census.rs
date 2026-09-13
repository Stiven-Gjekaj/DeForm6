#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The corpus wide proof that no vendored program raises anything above
//! `Severity::Tolerated`.
//!
//! `Journal::record` refuses a `Recoverable` defect in strict mode and
//! continues past a `Tolerated` one in both modes. This session measured
//! that the 44 vendored corpus programs raise 429 defects between them
//! (36 of the 44 programs raise at least one) and every one of those
//! defects is `Tolerated`. If a future plan reclassifies a `DefectKind` arm
//! back to `Recoverable` (or adds a new arm that is `Recoverable` by
//! mistake) and a vendored, undamaged program raises it, this test names
//! the file, the byte offset, the structure, the field and the severity
//! that was found, so the regression is visible immediately rather than
//! surfacing later as a strict refusal of a program nobody damaged.
//!
//! # Where the whole-file defect list comes from
//!
//! `Report::defects` is the complete list: `vb/mod.rs`'s own `compose_form`
//! extends it with every one of a form's own defects before that form's
//! `FormReport` is returned, so a form-scoped defect lives in both
//! `Report::defects` and its own `FormReport::defects`, not one or the
//! other. This test walks `Report::defects` alone and asserts, rather than
//! assumes, that every form-level defect is already present in it; adding
//! `FormReport::defects` a second time would double count exactly the
//! form-scoped defects, which is what an earlier draft of this test did
//! (it reported 430, one more than the 429 `Report::defects` alone holds).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use deform6::error::{DefectKind, Severity};
use deform6::inspect;
use deform6::vb::opcodes::OpcodeTable;

/// Gives the directory that holds the `corpus/` this workspace vendors.
///
/// Copied from `tests/corpus_sweep.rs`, which owns the same helper for the
/// same reason: both walk the same vendored directory.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks `corpus/` recursively and gives every file whose extension is
/// `.exe`, compared case-insensitively.
///
/// Copied from `tests/corpus_sweep.rs`; see that file's own doc comment on
/// `executables` for why the compare is case-insensitive and why the count
/// assertion, not the compare, is what actually guards against a miss.
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

/// The `DefectKind` variant's own name, with no wildcard arm: a new variant
/// fails to compile here until this function names it, the same discipline
/// `DefectKind::severity` itself already holds to.
const fn kind_name(kind: &DefectKind) -> &'static str {
    match kind {
        DefectKind::BadMagic { .. } => "BadMagic",
        DefectKind::OffsetOverflow { .. } => "OffsetOverflow",
        DefectKind::PastEndOfFile { .. } => "PastEndOfFile",
        DefectKind::ImplausibleCount { .. } => "ImplausibleCount",
        DefectKind::UnmappedAddress { .. } => "UnmappedAddress",
        DefectKind::SectionOverlap { .. } => "SectionOverlap",
        DefectKind::NoNulTerminator { .. } => "NoNulTerminator",
        DefectKind::CountMismatch { .. } => "CountMismatch",
        DefectKind::UnreadablePointer { .. } => "UnreadablePointer",
        DefectKind::EmptyName { .. } => "EmptyName",
        DefectKind::IndexHighByteSet { .. } => "IndexHighByteSet",
        DefectKind::UnrecoverableString { .. } => "UnrecoverableString",
        DefectKind::ClassNameNoDot { .. } => "ClassNameNoDot",
        DefectKind::GuidLengthUnexpected { .. } => "GuidLengthUnexpected",
        DefectKind::OcxReservedFieldUnexpected { .. } => "OcxReservedFieldUnexpected",
        DefectKind::BlobLenTooSmall { .. } => "BlobLenTooSmall",
        DefectKind::StructureUnreadable { .. } => "StructureUnreadable",
    }
}

#[test]
fn every_defect_the_corpus_raises_is_tolerated() {
    let files = executables();
    let count = files.len();
    assert_eq!(count, 44, "found {count} corpus executables, wanted 44");

    let mut total_defects = 0_usize;
    let mut programs_with_a_defect = 0_usize;
    let mut by_kind: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut above_tolerated = Vec::new();

    for path in &files {
        let data =
            std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
        let report = inspect(&data, &OpcodeTable::builtin())
            .unwrap_or_else(|refusal| panic!("{}: inspect refused it: {refusal}", path.display()));

        // Every form-level defect must already be one of the report-level
        // defects, proven here rather than assumed, per `compose_form`'s own
        // `defects.extend(form_defects.iter().cloned())` before it returns.
        // `Report::defects` is therefore already the whole-file list, and
        // walking `FormReport::defects` as well would double count.
        for form in &report.forms {
            for form_defect in &form.defects {
                assert!(
                    report.defects.contains(form_defect),
                    "{}: a form-level defect is missing from Report::defects: {form_defect}",
                    path.display()
                );
            }
        }

        if !report.defects.is_empty() {
            programs_with_a_defect += 1;
        }
        total_defects += report.defects.len();

        for defect in &report.defects {
            *by_kind.entry(kind_name(&defect.kind)).or_insert(0) += 1;
            if defect.kind.severity() != Severity::Tolerated {
                above_tolerated.push(format!(
                    "{}: offset {:#x}, structure {}, field {}, severity {:?}: {}",
                    path.display(),
                    defect.site.offset,
                    defect.site.structure,
                    defect.site.field,
                    defect.kind.severity(),
                    defect.kind
                ));
            }
        }
    }

    assert!(
        above_tolerated.is_empty(),
        "{} defect(s) above Severity::Tolerated in the vendored corpus, which a strict run \
         would now refuse:\n{}",
        above_tolerated.len(),
        above_tolerated.join("\n")
    );

    assert!(
        total_defects > 0,
        "the corpus wide walk found zero defects; an empty walk proves nothing about the \
         severity table, so this assertion refuses to let the census pass on an empty result"
    );

    eprintln!(
        "severity_census: {programs_with_a_defect} of {count} corpus programs raised at least \
         one defect, {total_defects} defects in total, across {} distinct defect kinds: \
         {by_kind:?}",
        by_kind.len()
    );
}
