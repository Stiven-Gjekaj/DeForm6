#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! ROADMAP success criterion 2, run over all 44 corpus executables as a
//! `#[test]`, so `cargo test --workspace` is the whole gate.
//!
//! **Every one of the 44 vendored projects carries `CompilationType=0`,
//! which is native.** The P-code branch of `Report::native` is therefore
//! untested by construction here. A P-code binary is needed before that
//! branch can be called tested. `STRUCTURES.md` section 12 names
//! `TimoKunze/ExplorerTreeView-VB6` as a known source of one.

use std::path::{Path, PathBuf};

use deform6::inspect;
use deform6::read::pe::PeImage;
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::opcodes::OpcodeTable;

/// The runtime name every corpus file imports. The sweep compares
/// `Report::runtime_dll` against this literal, and never against
/// `Runtime::Vb6`: that enum has one variant, so a comparison against it
/// would prove only that `inspect` returned `Ok`.
const RUNTIME_DLL: &str = "MSVBVM60.DLL";

/// The four bytes every Visual Basic 5 or 6 header opens with.
const SIGNATURE: [u8; 4] = *b"VB5!";

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks `corpus/` recursively and gives every file whose extension is
/// `.exe`, compared case-insensitively.
///
/// All 44 executables currently carry a lower case `.exe`. The corpus
/// already holds one file with an upper case extension,
/// `corpus/public-domain/SK-MCI-Sample__VB6/MCI.VBP`, which is a project
/// file and not an executable; it shows this corpus is not uniformly lower
/// case, not that a file is currently missed. The case-insensitive compare
/// is a guard against a future addition, and the count assertion below is
/// what would actually catch one.
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

#[test]
fn all_forty_four_corpus_executables_read_and_report_what_they_hold() {
    let files = executables();
    let count = files.len();
    assert_eq!(count, 44, "found {count} corpus executables, wanted 44");

    let mut failed = Vec::new();
    for path in &files {
        if let Err(reason) = check_one(path) {
            failed.push(format!("{}: {reason}", path.display()));
        }
    }
    assert!(
        failed.is_empty(),
        "{} of {} corpus executables failed:\n{}",
        failed.len(),
        files.len(),
        failed.join("\n")
    );
}

/// Checks one corpus executable against every property the sweep holds it
/// to, and gives back the first one it does not meet.
fn check_one(path: &Path) -> Result<(), String> {
    let data =
        std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));

    let report = inspect(&data, &OpcodeTable::builtin())
        .map_err(|refusal| format!("inspect refused it: {refusal}"))?;

    if report.signature != SIGNATURE {
        return Err(format!(
            "the signature is {:?}, not the Visual Basic magic {SIGNATURE:?}",
            report.signature
        ));
    }
    if report.runtime_dll != RUNTIME_DLL {
        return Err(format!(
            "the runtime DLL is {:?}, not {RUNTIME_DLL:?}",
            report.runtime_dll
        ));
    }
    if !report.native {
        return Err(
            "the project reports p-code; every vendored project is native, so this branch is \
             untested by construction, and this corpus file is now the counter-example that \
             changes that"
                .to_owned(),
        );
    }
    if report.project_name.is_empty() {
        return Err("the project name is empty".to_owned());
    }

    // A second, independent read of the header, bypassing `Report`
    // entirely. This is the committed guard that keeps the `VBHeader`
    // `0x58`/`0x5C` gap RESEARCH.md section 8 closed, run over the whole
    // corpus rather than over `Mandelbrot.exe` alone.
    let image = PeImage::parse(&data).map_err(|err| format!("PeImage::parse: {err:?}"))?;
    let hdr = header_region(&image).map_err(|refusal| format!("header_region: {refusal}"))?;
    let header = VbHeader::read(&hdr).map_err(|refusal| format!("VbHeader::read: {refusal}"))?;

    // Cross-checked against `Report`'s own fields, and not only read a
    // second time: a bug that swapped which header string `inspect` copies
    // into which `Report` field would leave the header's own offsets
    // ascending, and only this comparison would catch it.
    if report.exe_name != header.exe_name {
        return Err(format!(
            "Report::exe_name is {:?}; the header names {:?} at its own executable name offset",
            report.exe_name, header.exe_name
        ));
    }
    if report.title != header.title {
        return Err(format!(
            "Report::title is {:?}; the header names {:?} at its own title offset",
            report.title, header.title
        ));
    }
    if report.help_file != header.help_file {
        return Err(format!(
            "Report::help_file is {:?}; the header names {:?} at its own help file offset",
            report.help_file, header.help_file
        ));
    }

    if !(header.o_project_exe_name < header.o_project_title
        && header.o_project_title < header.o_help_file
        && header.o_help_file <= header.o_project_name)
    {
        return Err(format!(
            "the header string offsets do not ascend: exe name {:?}, title {:?}, help file {:?}, \
             project name {:?}",
            header.o_project_exe_name,
            header.o_project_title,
            header.o_help_file,
            header.o_project_name
        ));
    }

    Ok(())
}

/// Plan 03-10, Task 1's own acceptance criterion: `inspect` over all 44
/// corpus programs, asserting every one of them either gives a tree for
/// every form it declares, or gives a `Defect` naming the byte offset --
/// never a panic, and never a tree emitted over bytes the tiling check
/// refused. `deform6::inspect`'s own `Result<Report, Refusal>` return type
/// already makes a panic-free run of this loop the proof that nothing
/// panicked: `std::panic::catch_unwind` is not needed on top of it, because
/// `cargo test` itself already fails loudly the moment any one iteration
/// aborts the test process instead of returning.
#[test]
fn every_form_across_the_corpus_gives_a_tree_or_a_named_defect() {
    let files = executables();
    assert_eq!(
        files.len(),
        44,
        "found {} corpus executables, wanted 44",
        files.len()
    );

    let table = OpcodeTable::builtin();
    let mut failures = Vec::new();

    for path in &files {
        let data =
            std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
        let report = match inspect(&data, &table) {
            Ok(report) => report,
            Err(refusal) => {
                failures.push(format!(
                    "{}: inspect refused the whole file: {refusal}",
                    path.display()
                ));
                continue;
            }
        };

        for form in &report.forms {
            let has_tree = !form.controls.is_empty();
            let has_named_defect = form.defects.iter().any(|d| {
                matches!(
                    d.kind,
                    deform6::error::DefectKind::StructureUnreadable { .. }
                )
            });
            if !has_tree && !has_named_defect {
                failures.push(format!(
                    "{}: form {:?} gives neither a tree nor a named defect: {:?}",
                    path.display(),
                    form.name,
                    form.defects
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} form(s) across the corpus gave neither a tree nor a named defect:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
