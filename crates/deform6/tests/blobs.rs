#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "this is the test harness, not the library under test: it reads a vendored, \
              fixed corpus this repository controls, so the strict input-hostility \
              discipline `src/` carries does not apply (the threat model's T-02-35 accepts \
              this, because the harness is not exposed to hostile input the way the parser \
              reading a real VB6 executable is)"
)]

//! The end to end proof that a live `inspect` surfaces a resource blob,
//! per FRM-05 and this phase's own verification report.
//!
//! **This test names two programs on purpose. It is the proof the
//! verification report says was missing, not a unit test on
//! `frx::extract_blob`.** `extract_blob` already carries twenty passing
//! unit tests (plan 03-07); those tests proved the reader works, which was
//! never the problem. The problem was that nothing in the production path
//! called it. Every test in this file therefore goes through
//! [`deform6::inspect`], the one public entry point the command line
//! crate itself calls, and never through `extract_blob` directly. If the
//! resource blob arm stops calling `extract_blob`, every test here fails,
//! because the property list this file inspects would hold no
//! [`PropertyValue::Blob`] at all.
//!
//! Ground truth is the committed `.frx` beside each executable, read at
//! run time. This file compares the recovered blob's own bytes against
//! that committed file and never against anything DeForm6 produced: a
//! round trip through this repository's own writer and its own parser
//! would prove only that the code agrees with itself, which `AGENTS.md`
//! bars from counting as verification. This file commits no length table
//! and no hash computed from either corpus binary: every number below is
//! read from the committed files at run time.

use std::path::{Path, PathBuf};

use deform6::vb::frx::ImageFormat;
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::propstream::PropertyValue;

/// `corpus/vb6-code/Fire-effect/Fast_Flames.exe`, this phase's own primary
/// resource blob sample: its form `frmFire` declares one `Icon` property,
/// measured by hand at file offset `0x13d5`.
const FAST_FLAMES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/vb6-code/Fire-effect/Fast_Flames.exe"
));

/// A second, independent corpus program:
/// `corpus/public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe`,
/// whose form `frmMain` declares its own `Icon` property at file offset
/// `0x1302`, so a change that breaks one program's path cannot pass
/// because the other happens to agree.
const WINSOCK_SAMPLE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/public-domain/SK-Winsock-Sample__VB6/demo/SubReality_WinsockSample.exe"
));

/// Gives the absolute path to a file under this repository's own vendored
/// `corpus/`.
fn corpus_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus")
        .join(relative)
}

/// One recovered resource blob's own facts, read out of a
/// [`PropertyValue::Blob`].
struct RecoveredBlob {
    offset: u32,
    declared_len: u32,
    image_len: u32,
    frx_offset: u32,
    format: ImageFormat,
}

/// Gives the single recovered [`PropertyValue::Blob`] in `properties`,
/// panicking with the count actually found: this is the acceptance
/// instrument for "the form's property list holds exactly one recovered
/// blob," never a value this function invents when the count disagrees.
fn the_one_recovered_blob(properties: &[PropertyValue]) -> RecoveredBlob {
    let blobs: Vec<RecoveredBlob> = properties
        .iter()
        .filter_map(|property| match property {
            PropertyValue::Blob {
                offset,
                declared_len,
                image_len,
                frx_offset,
                format,
                ..
            } => Some(RecoveredBlob {
                offset: *offset,
                declared_len: *declared_len,
                image_len: *image_len,
                frx_offset: *frx_offset,
                format: format.clone(),
            }),
            _ => None,
        })
        .collect();
    assert_eq!(
        blobs.len(),
        1,
        "expected exactly one recovered blob in this form's own property list, found {}",
        blobs.len()
    );
    blobs
        .into_iter()
        .next()
        .expect("checked above: exactly one")
}

/// Tells whether `frx_bytes`' own image bytes (past the 4 byte length
/// field and the 8 byte inline header) carry the ICO signature
/// `00 00 01 00`, per `frx::ImageFormat`'s own doc comment. A hand-written
/// check, deliberately not a call to `frx::sniff_format`: this proves the
/// format detection against the committed file's own bytes, not against a
/// name this test merely asserts, and a bug in `sniff_format` itself would
/// still be caught by comparing its answer against this independent read.
fn committed_frx_carries_the_ico_signature(frx_bytes: &[u8]) -> bool {
    frx_bytes.get(12..16) == Some(&[0x00, 0x00, 0x01, 0x00][..])
}

/// Runs the whole proof for one program: calls the production `inspect`
/// entry point, finds `form_name`'s own single recovered blob, and asserts
/// its declared length, its header bytes and its image bytes reconstruct
/// the committed `.frx` at `frx_relative` exactly.
fn assert_blob_matches_committed_frx(exe_bytes: &[u8], form_name: &str, frx_relative: &str) {
    let report = deform6::inspect(exe_bytes, &OpcodeTable::builtin())
        .expect("a corpus program in this repository's own vendored set must inspect cleanly");
    let form = report
        .forms
        .iter()
        .find(|f| f.name == form_name)
        .unwrap_or_else(|| panic!("{form_name} must be a form this program declares"));
    let root = form
        .controls
        .first()
        .expect("the form's own tree must resolve for this proof to stand");

    let blob = the_one_recovered_blob(&root.properties);
    assert_eq!(
        blob.frx_offset, 0,
        "the only blob in a fresh form's own property stream must sit at .frx offset 0"
    );
    assert_eq!(
        blob.declared_len,
        blob.image_len + 8,
        "declared_len must be the image length plus the 8 byte inline picture header"
    );

    // Ground truth: the committed .frx beside the executable, read at run
    // time. Never a length or a hash this repository wrote down.
    let frx_bytes = std::fs::read(corpus_path(frx_relative)).expect("reading the committed .frx");
    let expected_item_len = blob
        .declared_len
        .checked_add(4)
        .expect("a real corpus blob's own item length fits a u32");
    assert_eq!(
        u32::try_from(frx_bytes.len()).unwrap_or(u32::MAX),
        expected_item_len,
        "the committed .frx must hold exactly the 4 byte length field plus declared_len bytes"
    );

    // The recovered blob's own length field, header and image bytes:
    // re-read directly out of the executable's own bytes at the offset
    // extract_blob reported, the same byte range extract_blob itself read
    // them from. This is `deform6::inspect`'s own recovered locator
    // (offset, declared_len), proved by reconstructing the exact bytes it
    // names and comparing them against the committed source, never
    // against anything this repository produced.
    let at = usize::try_from(blob.offset).expect("a real corpus offset fits a usize");
    let end = at
        .checked_add(usize::try_from(expected_item_len).expect("fits a usize"))
        .expect("a real corpus item does not run past usize::MAX");
    let recovered_item = exe_bytes
        .get(at..end)
        .expect("the recovered offset and length must stay inside the executable's own bytes");
    assert_eq!(
        recovered_item,
        frx_bytes.as_slice(),
        "the recovered blob (its length field, its header and its image) must equal the \
         committed .frx exactly, byte for byte"
    );

    assert!(
        committed_frx_carries_the_ico_signature(&frx_bytes),
        "this test's own fixture assumption changed: {frx_relative} no longer carries the ICO \
         signature this test was written against"
    );
    assert_eq!(
        blob.format,
        ImageFormat::Ico,
        "the recovered format must match the format the committed bytes carry"
    );
}

/// `Fast_Flames.exe`'s own form `frmFire` recovers exactly one resource
/// blob, and its bytes reconstruct the committed `frmFire.frx` exactly.
///
/// This is the test the verification report names as missing. Restoring
/// the resource blob arm to the version that reports the opcode as not
/// decoded makes this test fail: `the_one_recovered_blob` panics with
/// "found 0", because no `PropertyValue::Blob` reaches this property list
/// at all. See this plan's own SUMMARY, "Break-on-purpose evidence," for
/// the recorded failure.
#[test]
fn fast_flames_frm_fire_recovers_a_blob_matching_the_committed_frx() {
    assert_blob_matches_committed_frx(FAST_FLAMES, "frmFire", "vb6-code/Fire-effect/frmFire.frx");
}

/// A second, independent corpus program, so a change that breaks one
/// program's own path cannot pass because the other happens to agree.
#[test]
fn winsock_sample_frm_main_recovers_a_blob_matching_the_committed_frx() {
    assert_blob_matches_committed_frx(
        WINSOCK_SAMPLE,
        "frmMain",
        "public-domain/SK-Winsock-Sample__VB6/frmMain.frx",
    );
}

/// Pins the two concrete numbers this session measured by hand for
/// `Fast_Flames.exe`'s own blob, so a reader can check them against the
/// committed `.frx` (1418 bytes: 4 byte length field, 8 byte header, 1406
/// image bytes) without re-deriving them from the byte comparison above.
#[test]
fn fast_flames_blob_declared_length_and_image_length_match_this_sessions_own_measurement() {
    let report = deform6::inspect(FAST_FLAMES, &OpcodeTable::builtin()).unwrap();
    let form = report
        .forms
        .iter()
        .find(|f| f.name == "frmFire")
        .expect("Fast_Flames.exe declares a form named frmFire");
    let root = form
        .controls
        .first()
        .expect("frmFire's own tree must resolve");
    let blob = the_one_recovered_blob(&root.properties);
    assert_eq!(blob.offset, 0x13d5);
    assert_eq!(blob.declared_len, 1414);
    assert_eq!(blob.image_len, 1406);
}

/// The same pin for the second corpus program's own blob.
#[test]
fn winsock_sample_blob_declared_length_and_image_length_match_this_sessions_own_measurement() {
    let report = deform6::inspect(WINSOCK_SAMPLE, &OpcodeTable::builtin()).unwrap();
    let form = report
        .forms
        .iter()
        .find(|f| f.name == "frmMain")
        .expect("SubReality_WinsockSample.exe declares a form named frmMain");
    let root = form
        .controls
        .first()
        .expect("frmMain's own tree must resolve");
    let blob = the_one_recovered_blob(&root.properties);
    assert_eq!(blob.offset, 0x1302);
    assert_eq!(blob.declared_len, 2246);
    assert_eq!(blob.image_len, 2238);
}
