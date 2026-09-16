#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The two fields that each tell whether an object is a standard module.
//!
//! Bit `0x2` of `Object.fObjectType` tells whether the object has an
//! `OptionalObjectInfo` block. `ObjectInfo.lpPrivateObject` tells whether it
//! has a `PrivateObj`. `inspect` compares the two for every object, and
//! reports a disagreement as `DefectKind::ModuleMarkerMismatch`. The defect
//! is `Tolerated`, because each reader still follows its own field, so a
//! strict run reports it and continues.
//!
//! No corpus program has a disagreement, so the tests below patch
//! SK-Gradient in memory. It holds one form, object 0, and one standard
//! module, object 1. `AGENTS.md` bars a committed patched program.
//!
//! # The offsets come from the walk and from this file
//!
//! The fidelity walk names where each `Object` starts. This file reads
//! `lpObjectInfo` and `fObjectType` out of that record by hand, resolves the
//! address by hand, and requires the walk to have graded an `ObjectInfo` at
//! the same place. The reader under test gives none of these numbers.
//!
//! # This file keeps its own corpus walk
//!
//! Copied rather than shared, for the reason `tests/corpus_sweep.rs` gives:
//! two corpus tests must be able to fail independently.

use std::path::{Path, PathBuf};

use deform6::error::{Defect, DefectKind};
use deform6::fidelity::walk::walk;
use deform6::journal::Mode;
use deform6::read::pe::PeImage;
use deform6::read::region::Rva;
use deform6::vb::opcodes::OpcodeTable;

/// The number of executables the corpus vendors.
const EXPECTED_EXECUTABLE_COUNT: usize = 44;

/// The number of objects the 44 corpus programs declare between them.
const EXPECTED_OBJECT_COUNT: usize = 105;

/// `STRUCTURES.md` section 5.1: `lpObjectInfo` sits at `Object + 0x00`.
const LP_OBJECT_INFO: usize = 0x00;

/// `STRUCTURES.md` section 5.1: `fObjectType` sits at `Object + 0x28`.
const F_OBJECT_TYPE: usize = 0x28;

/// `STRUCTURES.md` section 5.2: `lpPrivateObject` sits at `ObjectInfo + 0x0C`.
const LP_PRIVATE_OBJECT: usize = 0x0C;

/// The sentinel `lpPrivateObject` holds for a standard module.
const MODULE_SENTINEL: u32 = 0xFFFF_FFFF;

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks `corpus/` recursively and gives every file whose extension is
/// `.exe`, compared case-insensitively.
fn executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_dir(&corpus_root(), &mut out);
    out.sort();
    out
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("reading an entry of {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push(path);
        }
    }
}

/// Reads SK-Gradient, the program every patch in this file starts from.
fn sk_gradient() -> Vec<u8> {
    let path = corpus_root().join("public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe");
    std::fs::read(&path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

/// Where one object's two markers are, found as the module doc comment says.
struct Markers {
    /// The file offset of `Object.fObjectType`.
    object_type_at: usize,
    /// The file offset of `ObjectInfo.lpPrivateObject`.
    pointer_at: usize,
    /// The RVA of `ObjectInfo.lpPrivateObject`.
    pointer_rva: u32,
}

fn markers(data: &[u8], object: usize) -> Markers {
    let found = walk(data).unwrap();
    let objects: Vec<_> = found
        .ledgers
        .iter()
        .filter(|ledger| ledger.structure == "Object")
        .collect();
    let base = usize::try_from(objects[object].base.get()).unwrap();

    let pe = PeImage::parse(data).unwrap();
    let info_rva = u32_at(data, base + LP_OBJECT_INFO) - pe.image_base();
    let info_at = pe.rva_to_off(Rva::new(info_rva)).unwrap().get();
    assert!(
        found
            .ledgers
            .iter()
            .any(|ledger| ledger.structure == "ObjectInfo" && ledger.base.get() == info_at),
        "the walk graded no ObjectInfo at {info_at:#x}, where object {object} points"
    );

    Markers {
        object_type_at: base + F_OBJECT_TYPE,
        pointer_at: usize::try_from(info_at).unwrap() + LP_PRIVATE_OBJECT,
        pointer_rva: info_rva + u32::try_from(LP_PRIVATE_OBJECT).unwrap(),
    }
}

/// Writes `value` at `at` and requires the write to change the file.
fn patch(data: &mut [u8], at: usize, value: u32) {
    let bytes = value.to_le_bytes();
    assert_ne!(
        data[at..at + 4],
        bytes,
        "the patch at {at:#x} must change the file, or this test proves nothing"
    );
    data[at..at + 4].copy_from_slice(&bytes);
}

/// Every `ModuleMarkerMismatch` a run of `data` in `mode` raises.
fn mismatches(data: &[u8], mode: Mode) -> Vec<Defect> {
    let report = deform6::inspect(data, &OpcodeTable::builtin(), mode)
        .unwrap_or_else(|err| panic!("a {mode:?} run refused the patched file: {err}"));
    report
        .defects
        .into_iter()
        .filter(|defect| matches!(defect.kind, DefectKind::ModuleMarkerMismatch { .. }))
        .collect()
}

/// Requires `found` to be exactly one mismatch, at the pointer `markers`
/// names, holding the two values given.
fn assert_one_mismatch(found: &[Defect], markers: &Markers, pointer: u32, object_type: u32) {
    assert_eq!(found.len(), 1, "wanted one mismatch, found {found:?}");
    let defect = &found[0];
    let at = u32::try_from(markers.pointer_at).unwrap();
    assert_eq!(defect.site.offset, at, "{defect}");
    assert_eq!(defect.site.rva, Some(markers.pointer_rva), "{defect}");
    assert_eq!(defect.site.structure, "ObjectInfo");
    assert_eq!(defect.site.field, "lpPrivateObject");
    assert_eq!(
        defect.kind,
        DefectKind::ModuleMarkerMismatch {
            offset: at,
            pointer,
            object_type,
        }
    );
}

#[test]
fn no_corpus_program_raises_a_module_marker_mismatch() {
    let files = executables();
    assert_eq!(files.len(), EXPECTED_EXECUTABLE_COUNT);
    let mut objects = 0_usize;
    let mut found = Vec::new();
    for path in &files {
        let data = std::fs::read(path).unwrap();
        let report = deform6::inspect(&data, &OpcodeTable::builtin(), Mode::Strict)
            .unwrap_or_else(|err| panic!("{}: inspect refused it: {err}", path.display()));
        objects += report.objects.len();
        for defect in &report.defects {
            if matches!(defect.kind, DefectKind::ModuleMarkerMismatch { .. }) {
                found.push(format!("{}: {defect}", path.display()));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    assert_eq!(objects, EXPECTED_OBJECT_COUNT);
}

#[test]
fn a_form_whose_private_object_address_is_the_module_sentinel_raises_one_mismatch_at_that_byte() {
    let mut data = sk_gradient();
    let form = markers(&data, 0);
    let object_type = u32_at(&data, form.object_type_at);
    assert_eq!(
        object_type & 0x2,
        0x2,
        "object 0 must claim an optional block"
    );
    patch(&mut data, form.pointer_at, MODULE_SENTINEL);

    assert_one_mismatch(
        &mismatches(&data, Mode::Salvage),
        &form,
        MODULE_SENTINEL,
        object_type,
    );

    // The file offset and the RVA the defect gives are the same byte.
    let pe = PeImage::parse(&data).unwrap();
    assert_eq!(
        pe.rva_to_off(Rva::new(form.pointer_rva))
            .map(|off| off.get()),
        Some(u32::try_from(form.pointer_at).unwrap())
    );
}

#[test]
fn a_strict_run_reports_a_module_marker_mismatch_and_continues() {
    // Each reader still follows its own field, so the run has assumed
    // nothing, and a strict run does not refuse.
    let mut data = sk_gradient();
    let form = markers(&data, 0);
    let object_type = u32_at(&data, form.object_type_at);
    patch(&mut data, form.pointer_at, MODULE_SENTINEL);

    assert_one_mismatch(
        &mismatches(&data, Mode::Strict),
        &form,
        MODULE_SENTINEL,
        object_type,
    );
}

#[test]
fn a_module_whose_object_type_claims_an_optional_block_raises_one_mismatch_at_its_private_object_address()
 {
    let mut data = sk_gradient();
    let module = markers(&data, 1);
    assert_eq!(u32_at(&data, module.pointer_at), MODULE_SENTINEL);
    let object_type = u32_at(&data, module.object_type_at);
    assert_eq!(object_type & 0x2, 0, "object 1 must be a standard module");
    patch(&mut data, module.object_type_at, object_type | 0x2);

    assert_one_mismatch(
        &mismatches(&data, Mode::Salvage),
        &module,
        MODULE_SENTINEL,
        object_type | 0x2,
    );
}

#[test]
fn a_zero_private_object_address_agrees_with_a_module_and_disagrees_with_a_form() {
    // A zero names no PrivateObj, as the sentinel does.
    let mut module_data = sk_gradient();
    let module = markers(&module_data, 1);
    patch(&mut module_data, module.pointer_at, 0);
    assert!(mismatches(&module_data, Mode::Salvage).is_empty());

    let mut form_data = sk_gradient();
    let form = markers(&form_data, 0);
    let object_type = u32_at(&form_data, form.object_type_at);
    patch(&mut form_data, form.pointer_at, 0);
    assert_one_mismatch(
        &mismatches(&form_data, Mode::Salvage),
        &form,
        0,
        object_type,
    );
}
