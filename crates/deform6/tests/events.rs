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

//! The end to end proof that a live `inspect` surfaces a bound event
//! slot's own native handler address, per the narrowed FRM-06 (plan 03-11,
//! citing D-02) and this phase's own re-verification.
//!
//! **This file names two programs on purpose. It is the proof the
//! re-verification report says was missing, not a unit test on
//! `decode_stub` or `StubHandler`.** `StubHandler::handler_address` already
//! carries unit tests proven byte for byte against a real corpus stub
//! (`03-09-SUMMARY.md`); those tests proved the decoder works, which was
//! never the problem. The problem was that nothing in the production path
//! carried the value past `controlinfo.rs`'s own module: `EventReport` had
//! no field for it, so `report_events` threw it away and `print_event` had
//! nothing to print. This is the second time in this phase a value was
//! computed, unit-tested and never wired to anything that reads it:
//! `frx::extract_blob` (FRM-05, closed by `crates/deform6/tests/blobs.rs`,
//! plan 03-15) was the first. `StubHandler::handler_address` (FRM-06) is
//! the second, found by the phase's own re-verification and closed here.
//! Every test in this file therefore reaches the address only through
//! [`deform6::inspect`], the one public entry point the command line crate
//! itself calls, and never through `decode_stub` or `StubHandler` directly.
//! If the wiring `report_events` carries stops filling `handler_address`,
//! every test here fails, because no bound slot's own report would carry
//! an address to compare.
//!
//! Ground truth is computed by hand, at run time, from the same two
//! corpus executables `deform6::inspect` itself reads. This file walks the
//! public reading API to the raw event slot's own `stub` address
//! (`PeImage::parse`, `header_region`, `VbHeader::read`,
//! `ProjectInfo::read`, `ObjectTableHead::read`, `ObjectTable::walk`,
//! `ControlInfoTable::read`, `read_event_table`) and stops there: from each
//! bound slot it takes the `stub` address and nothing else. It then reads
//! the four signed bytes at `stub` plus `0x09` and computes `stub` plus
//! `13` plus that signed value **in this file's own code**, never by
//! calling `decode_stub` and never by reading a `StubHandler`. This is the
//! same choice `blobs.rs` makes when it checks the image signature by hand
//! instead of calling `frx::sniff_format`: a second, independent read is
//! what makes the comparison mean something. This file commits no address
//! literal taken from a corpus binary: every expected value is computed
//! from the committed corpus bytes at run time.

use std::collections::BTreeSet;

use deform6::read::pe::PeImage;
use deform6::read::region::{Off, Va};
use deform6::vb::classify::{self, ObjectKind};
use deform6::vb::controlinfo::{ControlInfoTable, EventReport, EventSlot, read_event_table};
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::object::ObjectTable;
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::project::{ObjectTableHead, ProjectInfo};

/// `corpus/vb6-code/Fire-effect/Fast_Flames.exe`: ROADMAP success
/// criterion 6's own program, whose form `frmFire` carries a bound event
/// slot 8 on the form itself, plus two more bound slots this session's own
/// measurement found on `cmdStop` and `cmdStart` (slot 0 each). See
/// `03-18-SUMMARY.md` for the null delimited sweep this count came from.
const FAST_FLAMES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/vb6-code/Fire-effect/Fast_Flames.exe"
));

/// A second, independent corpus program:
/// `corpus/vb6-code/Mandelbrot/Mandelbrot.exe`, whose form `frmFractal`
/// carries seven bound event slots across four controls, measured this
/// session (`frmFractal` itself, `CmdReset`, `scrAccuracy`, `CmdRedraw`
/// slot 0 each, and `PicDraw` slots 13, 14 and 15). A change that breaks
/// one program's own path cannot pass because the other happens to agree.
const MANDELBROT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
));

/// The literal name `ControlInfoTable::read`'s own module doc comment
/// gives the form's own entry, never the form's own declared name. Named
/// here, and not imported: `controlinfo::FORM_SELF_ENTRY_NAME` is
/// `pub(crate)`, unreachable from this file, and this file's own
/// independence from that module's internals is the point.
const FORM_SELF_ENTRY_NAME: &str = "Form";

/// Decodes one native event stub by hand: reads the four signed bytes at
/// `stub_va` plus `0x09` and computes the handler address as `stub_va`
/// plus `13` plus that signed value, with checked arithmetic over the
/// whole signed range. `None` when `stub_va` resolves to no section or the
/// stub's own 13 bytes cannot be read in full.
///
/// This is a second, independent decoder, deliberately not a call to
/// `controlinfo::decode_stub`: see this file's own module doc comment for
/// why.
fn compute_handler_address_by_hand(pe: &PeImage<'_>, stub_va: Va) -> Option<u32> {
    let stub = pe.region_at_va(stub_va)?;
    let rel32 = stub.i32_le(Off::new(0x09))?;
    let handler_start = stub_va.get().checked_add(13)?;
    handler_start.checked_add_signed(rel32)
}

/// Walks the public reading API to `data`'s own single form's
/// `ControlInfoTable`, and hand-decodes every bound slot's own handler
/// address, giving `(control name, slot index, handler address)` for every
/// slot whose stub resolved and could be decoded.
///
/// The form's own event slot carries the literal control-info name
/// [`FORM_SELF_ENTRY_NAME`], never the form's own declared name; this
/// matches `vb/mod.rs`'s own `compose_form`, which reports the root
/// control's own events under `header.name` (the form's declared name),
/// not under the literal `ControlInfo` entry name.
fn hand_decoded_bound_events(data: &[u8]) -> Vec<(String, u16, u32)> {
    let image = PeImage::parse(data).expect("a corpus program in this test's own set parses");
    let hdr = header_region(&image).expect("a corpus program carries a VB header");
    let header = VbHeader::read(&hdr).expect("a corpus program's VB header reads");
    let info = ProjectInfo::read(&image, header.lp_project_data)
        .expect("a corpus program's ProjectInfo reads");
    let head = ObjectTableHead::read(&image, info.lp_object_table)
        .expect("a corpus program's object table head reads");
    let table = ObjectTable::walk(&image, info.lp_object_table, &head)
        .expect("a corpus program's object table walks");
    let form_object = table
        .objects
        .iter()
        .find(|object| classify::classify(object.f_object_type) == ObjectKind::Form)
        .expect("a corpus program used by this test declares at least one form");

    let control_info_table = ControlInfoTable::read(&image, form_object)
        .expect("a corpus program's ControlInfoTable reads");

    let mut found = Vec::new();
    for entry in &control_info_table.entries {
        let control_name = if entry.name == FORM_SELF_ENTRY_NAME {
            form_object.name.clone()
        } else {
            entry.name.clone()
        };

        let event_table =
            read_event_table(&image, entry).expect("a corpus ControlInfo's own event table reads");
        for slot in &event_table.slots {
            let EventSlot::Bound { index, stub, .. } = slot else {
                continue;
            };
            if let Some(address) = compute_handler_address_by_hand(&image, *stub) {
                found.push((control_name.clone(), *index, address));
            }
        }
    }
    found
}

/// Runs the whole proof for one program: calls the production
/// `deform6::inspect` entry point, collects every bound slot's own
/// `(control name, index, handler address)` triple `form_name`'s own
/// controls report, and asserts that multiset equals this file's own hand
/// decode exactly, with the exact bound count this session measured.
///
/// Also asserts the negative case in the same run: at least one unbound
/// slot exists, and the bound and unbound `(control name, index)` pairs
/// share no member. `EventReport::Unbound` carries no `handler_address`
/// field at all, so the type itself already guarantees an unbound slot
/// carries no address; this assertion is the structural half, that the two
/// index sets stay disjoint.
fn assert_handler_addresses_match(data: &[u8], form_name: &str, exact_bound_count: usize) {
    let report = deform6::inspect(data, &OpcodeTable::builtin())
        .expect("a corpus program in this repository's own vendored set must inspect cleanly");
    let form = report
        .forms
        .iter()
        .find(|f| f.name == form_name)
        .unwrap_or_else(|| panic!("{form_name} must be a form this program declares"));

    let mut recovered: Vec<(String, u16, u32)> = Vec::new();
    let mut recovered_unbound: Vec<(String, u16)> = Vec::new();
    for control in &form.controls {
        for event in &control.events {
            match event {
                EventReport::Named {
                    control_name,
                    index,
                    handler_address: Some(address),
                    ..
                }
                | EventReport::BoundUnnamed {
                    control_name,
                    index,
                    handler_address: Some(address),
                } => recovered.push((control_name.clone(), *index, *address)),
                EventReport::Unbound {
                    control_name,
                    index,
                } => recovered_unbound.push((control_name.clone(), *index)),
                // A bound slot whose stub did not decode: excluded from the
                // address comparison on purpose. Neither program this file
                // reads measures one (see the module doc comment); the
                // exact-count assertion below would catch a change that
                // silently started producing one.
                EventReport::Named { .. } | EventReport::BoundUnnamed { .. } => {}
            }
        }
    }

    let mut expected = hand_decoded_bound_events(data);
    recovered.sort();
    expected.sort();

    assert_eq!(
        recovered, expected,
        "{form_name}: the handler addresses deform6::inspect reports must equal this file's own \
         independent hand decode"
    );
    assert_eq!(
        recovered.len(),
        exact_bound_count,
        "{form_name}: expected exactly {exact_bound_count} bound slot(s) with a decoded \
         address, found {}: a multiset comparison of two empty sets would pass and prove \
         nothing, so the exact count is asserted here",
        recovered.len()
    );
    assert!(
        !recovered.is_empty(),
        "{form_name}: at least one bound slot with a decoded address is required for this \
         comparison to mean anything"
    );

    assert!(
        !recovered_unbound.is_empty(),
        "{form_name}: this form must carry at least one unbound slot for the negative case to \
         mean anything"
    );
    let bound_pairs: BTreeSet<(String, u16)> = recovered
        .iter()
        .map(|(name, index, _)| (name.clone(), *index))
        .collect();
    let unbound_pairs: BTreeSet<(String, u16)> = recovered_unbound.into_iter().collect();
    assert!(
        bound_pairs.is_disjoint(&unbound_pairs),
        "{form_name}: a slot must not appear in both the bound and the unbound sets"
    );
}

/// `Fast_Flames.exe`'s own form `frmFire` reports the native address of
/// every one of its own three bound event slots, matching this file's own
/// independent hand decode exactly.
///
/// Restoring the bound arm of `report_events` to give `handler_address:
/// None` unconditionally makes this test fail: `recovered` would then
/// carry zero triples, not the three the hand decode found. See this
/// plan's own SUMMARY, "Break-on-purpose evidence," for the recorded
/// failure.
#[test]
fn fast_flames_frm_fire_bound_handler_addresses_match_an_independent_hand_decode() {
    assert_handler_addresses_match(FAST_FLAMES, "frmFire", 3);
}

/// A second, independent corpus program, so a change that breaks one
/// program's own path cannot pass because the other happens to agree.
#[test]
fn mandelbrot_frm_fractal_bound_handler_addresses_match_an_independent_hand_decode() {
    assert_handler_addresses_match(MANDELBROT, "frmFractal", 7);
}
