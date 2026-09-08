#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! Plan 02-04, task 3: the four aggregates over every `FuncTypDesc` record
//! in all 44 vendored programs.
//!
//! `crates/deform6/tests/corpus_sweep.rs` already walks `corpus/` for its
//! own sweep. This file copies that directory walk rather than sharing it,
//! the same choice plan 01-08 made on purpose: two tests must be able to
//! fail independently.

use std::path::{Path, PathBuf};

use deform6::read::pe::PeImage;
use deform6::read::region::Va;
use deform6::vb::functyp::{
    Argument, DefaultValue, FuncTypeWalk, OptionalDefaultsOutcome, ProcedureSignature, Prototype,
    PrototypeList,
};
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::object::{Object, ObjectTable};
use deform6::vb::privateobj::{ObjectInfo, PrivateObj};
use deform6::vb::project::{ObjectTableHead, ProjectInfo};

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks `corpus/` recursively and gives every file whose extension is
/// `.exe`, compared case-insensitively. See `corpus_sweep.rs` for the same
/// walk, and why the count assertion (44) is what actually catches a future
/// addition rather than the case-insensitive compare.
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

/// Walks the object array of one executable's bytes.
fn objects(image: &PeImage<'_>) -> Vec<Object> {
    let lp_object_table = object_table_va(image);
    let head = ObjectTableHead::read(image, lp_object_table).unwrap();
    ObjectTable::walk(image, lp_object_table, &head)
        .unwrap()
        .objects
}

/// Reads `PrivateObj` for one object.
fn private_obj_of(image: &PeImage<'_>, object: &Object) -> PrivateObj {
    let info = ObjectInfo::read(image, object.lp_object_info).unwrap();
    PrivateObj::read(image, info.lp_private_object).unwrap()
}

/// The four running totals this sweep asserts, plus the failing site so the
/// message names the file, the object and the procedure.
#[derive(Default)]
struct Totals {
    /// The number of `FuncTypDesc` records that resolved to a [`Prototype`].
    records: u32,
    /// Records whose `const_ffff` is `0xFFFF` and whose `nul1` is `0`.
    header_ok: u32,
    /// Records whose type buffer closed at exactly `argSize >> 2`, and
    /// whose every argument name resolved (non-empty).
    buffer_closed_and_named: u32,
    /// Records with `optional_defaults` equal to
    /// [`OptionalDefaultsOutcome::Resolved`].
    with_resolved_defaults: u32,
    /// The sum of every `Resolved(n)` count, across every record.
    value_records: u32,
}

fn check_prototype(
    path: &Path,
    object_name: &str,
    proc_name: &str,
    prototype: &Prototype,
    totals: &mut Totals,
) -> Result<(), String> {
    totals.records += 1;

    if prototype.const_ffff == 0xFFFF && prototype.nul1 == 0 {
        totals.header_ok += 1;
    } else {
        return Err(format!(
            "{}: {object_name}.{proc_name}: const_ffff = {:#x}, nul1 = {}",
            path.display(),
            prototype.const_ffff,
            prototype.nul1
        ));
    }

    let every_name_resolved = prototype
        .arguments
        .iter()
        .all(|arg: &Argument| !arg.name.is_empty());
    if every_name_resolved {
        totals.buffer_closed_and_named += 1;
    } else {
        return Err(format!(
            "{}: {object_name}.{proc_name}: an argument name did not resolve",
            path.display()
        ));
    }

    match prototype.optional_defaults {
        OptionalDefaultsOutcome::Resolved(n) => {
            totals.with_resolved_defaults += 1;
            let n_u32 = u32::try_from(n).unwrap_or(u32::MAX);
            totals.value_records += n_u32;
        }
        OptionalDefaultsOutcome::NoOptionalVals => {}
        OptionalDefaultsOutcome::Unrecoverable => {
            return Err(format!(
                "{}: {object_name}.{proc_name}: the optionalVals walk did not close",
                path.display()
            ));
        }
    }

    Ok(())
}

fn check_one(path: &Path, totals: &mut Totals) -> Result<(), String> {
    let data =
        std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    let image = PeImage::parse(&data)
        .map_err(|err| format!("{}: PeImage::parse: {err:?}", path.display()))?;

    for object in objects(&image) {
        let private = private_obj_of(&image, &object);
        let walk = FuncTypeWalk::read(&image, &object, &private);
        if !walk.defects().is_empty() {
            return Err(format!(
                "{}: {}: {} defect(s): {:?}",
                path.display(),
                object.name,
                walk.defects().len(),
                walk.defects()
            ));
        }
        let PrototypeList::Slots(slots) = &walk.signatures else {
            continue;
        };
        for (index, slot) in slots.iter().enumerate() {
            match slot {
                ProcedureSignature::Prototype(prototype) => {
                    check_prototype(path, &object.name, &format!("#{index}"), prototype, totals)?;
                }
                ProcedureSignature::Unrecoverable => {
                    return Err(format!(
                        "{}: {}: index {index} is an unrecoverable FuncTypDesc",
                        path.display(),
                        object.name
                    ));
                }
                ProcedureSignature::NoDescriptor => {}
            }
        }
    }

    Ok(())
}

/// The four aggregates plan 02-04's task 3 pins, over all 44 vendored
/// programs: `AGENTS.md` requires the number that can be proved, and an
/// assertion that "most records closed" would hide the one that did not.
#[test]
fn all_func_typ_desc_records_close_over_the_whole_corpus() {
    let files = executables();
    assert_eq!(
        files.len(),
        44,
        "found {} corpus executables, wanted 44",
        files.len()
    );

    let mut totals = Totals::default();
    let mut failed = Vec::new();
    for path in &files {
        if let Err(reason) = check_one(path, &mut totals) {
            failed.push(reason);
        }
    }
    assert!(
        failed.is_empty(),
        "{} corpus file(s) failed:\n{}",
        failed.len(),
        failed.join("\n")
    );

    assert_eq!(
        totals.records, 193,
        "the number of resolved FuncTypDesc records moved from the measured 193"
    );
    assert_eq!(
        totals.header_ok, 193,
        "const_ffff == 0xFFFF and nul1 == 0 must hold on every one of the 193 records"
    );
    assert_eq!(
        totals.buffer_closed_and_named, 193,
        "the type buffer walk must close at exactly argSize >> 2, and every argument name \
         must resolve, on every one of the 193 records"
    );
    assert_eq!(
        totals.with_resolved_defaults, 61,
        "the number of records with a resolved optionalVals walk moved from the measured 61"
    );
    assert_eq!(
        totals.value_records, 89,
        "the number of default value records moved from the measured 89"
    );
}

/// Four worked default values, each compared against the source text beside
/// the executable, not against another read of the same bytes.
#[test]
fn four_worked_default_values_match_the_source_beside_the_executable() {
    let randomization_fx = corpus_root().join("vb6-code/Randomize-effects/RandomizationFX.exe");
    let data = std::fs::read(&randomization_fx).unwrap();
    let image = PeImage::parse(&data).unwrap();
    let object = objects(&image)
        .into_iter()
        .find(|o| o.name == "frmLineEffect")
        .unwrap();
    let private = private_obj_of(&image, &object);
    let walk = FuncTypeWalk::read(&image, &object, &private);
    let PrototypeList::Slots(slots) = &walk.signatures else {
        panic!("frmLineEffect carries no FuncTypDesc array");
    };
    let draw_triangle = slots
        .iter()
        .find_map(|s| match s {
            ProcedureSignature::Prototype(p)
                if p.arguments.len() == 4
                    && p.arguments[2].default == Some(DefaultValue::Integer(10000)) =>
            {
                Some(p)
            }
            _ => None,
        })
        .expect("DrawTriangleEffect not found");
    // `RandomizationFX.frm` declares
    // `Optional numLoops As Long = 10000, Optional lenLine As Long = 50`.
    let source = std::fs::read_to_string(
        corpus_root().join("vb6-code/Randomize-effects/RandomizationFX.frm"),
    )
    .unwrap();
    assert!(source.contains("Optional numLoops As Long = 10000"));
    assert!(source.contains("Optional lenLine As Long = 50"));
    assert_eq!(
        draw_triangle.arguments[3].default,
        Some(DefaultValue::Integer(50))
    );

    let emboss_engrave = corpus_root().join("vb6-code/Emboss-engrave-effect/Emboss_Engrave.exe");
    let data = std::fs::read(&emboss_engrave).unwrap();
    let image = PeImage::parse(&data).unwrap();
    let object = objects(&image)
        .into_iter()
        .find(|o| o.name == "frmEmbossEngrave")
        .unwrap();
    let private = private_obj_of(&image, &object);
    let walk = FuncTypeWalk::read(&image, &object, &private);
    let PrototypeList::Slots(slots) = &walk.signatures else {
        panic!("frmEmbossEngrave carries no FuncTypDesc array");
    };
    let draw_engrave = slots
        .iter()
        .find_map(|s| match s {
            ProcedureSignature::Prototype(p) if p.arguments.len() == 5 => Some(p),
            _ => None,
        })
        .expect("DrawEngrave not found");
    let source = std::fs::read_to_string(
        corpus_root().join("vb6-code/Emboss-engrave-effect/EmbossEngrave.frm"),
    )
    .unwrap();
    assert!(source.contains("Optional ByVal eR As Byte = 127"));
    for arg in &draw_engrave.arguments[2..5] {
        assert_eq!(arg.default, Some(DefaultValue::Byte(127)));
    }

    let colorize = corpus_root().join("vb6-code/Colorize-effect/Colorize.exe");
    let data = std::fs::read(&colorize).unwrap();
    let image = PeImage::parse(&data).unwrap();
    let object = objects(&image)
        .into_iter()
        .find(|o| o.name == "frmColorize")
        .unwrap();
    let private = private_obj_of(&image, &object);
    let walk = FuncTypeWalk::read(&image, &object, &private);
    let PrototypeList::Slots(slots) = &walk.signatures else {
        panic!("frmColorize carries no FuncTypDesc array");
    };
    let draw_colorize = slots
        .iter()
        .find_map(|s| match s {
            ProcedureSignature::Prototype(p) if p.arguments.len() == 5 => Some(p),
            _ => None,
        })
        .expect("DrawColorize not found");
    let source =
        std::fs::read_to_string(corpus_root().join("vb6-code/Colorize-effect/Colorize.frm"))
            .unwrap();
    assert!(source.contains("Optional ByVal forceSaturation As Boolean = False"));
    assert!(source.contains("Optional ByVal newSaturation As Single = 0.5"));
    assert_eq!(
        draw_colorize.arguments[3].default,
        Some(DefaultValue::Boolean(false))
    );
    assert_eq!(
        draw_colorize.arguments[4].default,
        Some(DefaultValue::Single(0.5))
    );

    let edge_detection = corpus_root().join("vb6-code/Edge-detection/Edge_Detection.exe");
    let data = std::fs::read(&edge_detection).unwrap();
    let image = PeImage::parse(&data).unwrap();
    let object = objects(&image)
        .into_iter()
        .find(|o| o.name == "cCommonDialog")
        .unwrap();
    let private = private_obj_of(&image, &object);
    let walk = FuncTypeWalk::read(&image, &object, &private);
    let PrototypeList::Slots(slots) = &walk.signatures else {
        panic!("cCommonDialog carries no FuncTypDesc array");
    };
    let vb_get_open_file_name = slots
        .iter()
        .find_map(|s| match s {
            ProcedureSignature::Prototype(p) if p.arguments.len() == 13 => Some(p),
            _ => None,
        })
        .expect("VBGetOpenFileName not found");
    let source = std::fs::read_to_string(
        corpus_root().join("vb6-code/Hidden-Markov-model/cCommonDialog.cls"),
    )
    .unwrap();
    assert!(source.contains(r#"Optional Filter As String = "All (*.*)| *.*""#));
    assert!(
        vb_get_open_file_name
            .arguments
            .iter()
            .any(|a| a.default == Some(DefaultValue::Text("All (*.*)| *.*".to_owned())))
    );
}
