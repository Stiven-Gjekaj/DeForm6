#![no_main]

//! The one fuzz target this crate holds.
//!
//! This target calls `deform6::inspect` twice over the same fuzzer bytes,
//! once in `Mode::Strict` and once in `Mode::Salvage`, calls
//! `deform6::write::project` a third time when the salvage read succeeds,
//! and calls `deform6::fidelity::walk::walk` over the same bytes.
//!
//! The fidelity walk is a fourth call and not a fourth target. It resolves
//! the same pointer chain `inspect` does, through its own code, and it is
//! public, so the crate's no-panic claim covers it. One target keeps the
//! corpus shared: an input that reaches deep into the object array is as
//! valuable to the walk as it is to `inspect`, and two targets would each
//! have to find it.
//! It must call both modes. The salvage path reaches code the strict path
//! refuses before it gets there, and that code is the least exercised in
//! the crate. A target that called one mode would leave it unfuzzed. This
//! is the whole reason the roadmap names this risk.
//!
//! This target asserts nothing about either call's result. It drops both.
//! A refusal is the correct answer for almost every input a fuzzer
//! generates, and an assertion on the result variant would report a
//! correct refusal as a crash. The only outcome this target treats as a
//! fault is a panic, and the release profile's `panic = "abort"` ends the
//! process on one, which is what lets libFuzzer write the input down.

use deform6::journal::Mode;
use deform6::vb::opcodes::OpcodeTable;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let table = OpcodeTable::builtin();

    // Strict: proves the default path never panics on hostile bytes.
    let _ = deform6::inspect(data, &table, Mode::Strict);

    // Salvage: proves the path strict refuses before it reaches never
    // panics either. When the salvage read succeeds, the writer runs over
    // the same report and the same bytes, in the same mode.
    if let Ok(report) = deform6::inspect(data, &table, Mode::Salvage) {
        let _ = deform6::write::project(&report, data, Mode::Salvage);
    }

    // Fidelity: the second walk takes the same untrusted bytes through its
    // own resolution of the chain, and it is public.
    let _ = deform6::fidelity::walk::walk(data);
});
