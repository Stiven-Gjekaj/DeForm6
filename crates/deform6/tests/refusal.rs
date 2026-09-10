#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! Every refusal, from fixtures patched in memory.
//!
//! Each fixture starts from the corpus file `Mandelbrot.exe` and patches one
//! imported DLL name, at the offset the file's own import descriptor names.
//! `PeImage::dll_name_sites` gives that offset. It is never a byte search: a
//! search for the runtime name could hit the same bytes in `.rsrc` or in a
//! string table, which is the class of mistake `AGENTS.md` names directly.
//!
//! Nothing is written to disk. The patched bytes live in a `Vec<u8>` for the
//! duration of one test function, and the source they come from is a
//! `corpus/` program committed under a redistributable licence, recorded in
//! `corpus/NOTICES`.
//!
//! **What this proves, and what it does not.** A patched import name proves
//! the discrimination logic in `deform6::vb::runtime::classify`. It does not
//! prove that DeForm6 handles a genuine Visual Basic 5 binary, whose header
//! layout differs after header-relative offset `0x30`, nor a genuine .NET
//! assembly, which carries a common language runtime header that the patched
//! file does not. DeForm6 refuses both before it reads any of that, so these
//! fixtures cover the behaviour this phase promises. When the corpus manifest
//! gains a real Visual Basic 5 sample in a later phase, add a second test
//! beside this one and keep this one: it runs offline and it runs on a clean
//! clone.

use deform6::Refusal;
use deform6::read::pe::PeImage;
use deform6::vb::opcodes::OpcodeTable;

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// The opcode table every `inspect` call in this file reads properties
/// through. None of these tests reach the property stream (every fixture
/// here is refused before that point); the builtin subset is passed only
/// because `inspect` takes one.
fn builtin_table() -> OpcodeTable {
    OpcodeTable::builtin()
}

/// Reads the general purpose corpus file this whole module patches.
fn mandelbrot() -> Vec<u8> {
    std::fs::read(corpus_root().join("vb6-code/Mandelbrot/Mandelbrot.exe")).unwrap()
}

/// Gives a copy of `data` whose first imported DLL name is `replacement`.
///
/// The offset comes from `PeImage::dll_name_sites`, which resolves the
/// address the import descriptor itself names. `replacement` must be exactly
/// as long as the name it covers, so no offset moves and a wrong-length
/// replacement is a loud failure rather than a corrupted image.
fn with_the_runtime_named(data: &[u8], replacement: &[u8]) -> Vec<u8> {
    let image = PeImage::parse(data).unwrap();
    let sites = image.dll_name_sites().unwrap();
    let (offset, len) = sites[0];
    let at = usize::try_from(offset.get()).unwrap();
    let len = usize::try_from(len).unwrap();
    assert_eq!(
        replacement.len(),
        len,
        "the replacement must be as long as the name it covers"
    );
    let mut out = data.to_vec();
    out[at..at + len].copy_from_slice(replacement);
    out
}

/// Gives a copy of `data` whose fifteenth data directory (index 14) names a
/// real section, which is what `PeImage::has_clr_header` reads.
///
/// Data directory 14 is zero in all 44 corpus executables, so the corpus
/// gives `has_clr_header` no positive case. This fixture is the only place in
/// the phase that sets it, which is what makes
/// `Refusal::NoVbRuntime { dot_net: true }` observable at all.
///
/// The helper asserts that both words were zero before the write, so a
/// future corpus file that already carries the header would make this
/// fixture a no-op rather than a silent pass.
fn with_a_clr_directory(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    let lfanew = u32::from_le_bytes(out[0x3c..0x40].try_into().unwrap());
    // 24 reaches the optional header from the PE signature. 96 reaches the
    // data directories from the start of a PE32 optional header. The
    // fifteenth directory is 14 entries of 8 bytes further on.
    let at = usize::try_from(lfanew).unwrap() + 24 + 96 + 14 * 8;
    let address = u32::from_le_bytes(out[at..at + 4].try_into().unwrap());
    let size = u32::from_le_bytes(out[at + 4..at + 8].try_into().unwrap());
    assert_eq!(address, 0, "data directory 14 already names an address");
    assert_eq!(size, 0, "data directory 14 already declares a size");

    let first = PeImage::parse(data).unwrap().sections()[0]
        .virtual_address
        .get();
    out[at..at + 4].copy_from_slice(&first.to_le_bytes());
    // 0x48 is the size of the common language runtime header structure.
    out[at + 4..at + 8].copy_from_slice(&0x48_u32.to_le_bytes());
    out
}

#[test]
fn an_empty_file_is_not_a_portable_executable() {
    assert_eq!(deform6::inspect(&[], &builtin_table()), Err(Refusal::NotPe));
}

#[test]
fn a_short_text_file_is_not_a_portable_executable() {
    let data = b"this is not a program\n";
    assert_eq!(
        deform6::inspect(data, &builtin_table()),
        Err(Refusal::NotPe)
    );
}

#[test]
fn a_pe_file_importing_no_recognised_visual_basic_runtime_is_refused() {
    let bytes = with_the_runtime_named(&mandelbrot(), b"KERNEL32.DLL");
    assert_eq!(
        deform6::inspect(&bytes, &builtin_table()),
        Err(Refusal::NoVbRuntime { dot_net: false })
    );
}

/// Naming an assembly's loader shim in the import table does not make a file
/// a .NET assembly. This fixture proves only that a second unrecognised name
/// is still refused, with `dot_net` false, because data directory 14 is
/// untouched.
#[test]
fn an_unrecognised_import_name_is_refused_as_holding_no_visual_basic_runtime() {
    let bytes = with_the_runtime_named(&mandelbrot(), b"mscoree.dll\0");
    assert_eq!(
        deform6::inspect(&bytes, &builtin_table()),
        Err(Refusal::NoVbRuntime { dot_net: false })
    );
}

/// This is the only fixture in the phase that sets data directory 14, so it
/// is the only place `Refusal::NoVbRuntime { dot_net: true }` is observed
/// end to end.
#[test]
fn a_common_language_runtime_header_makes_the_refusal_name_a_dot_net_assembly() {
    let named = with_the_runtime_named(&mandelbrot(), b"mscoree.dll\0");
    let bytes = with_a_clr_directory(&named);
    assert_eq!(
        deform6::inspect(&bytes, &builtin_table()),
        Err(Refusal::NoVbRuntime { dot_net: true })
    );
}

#[test]
fn a_visual_basic_5_runtime_name_is_refused_by_name() {
    let bytes = with_the_runtime_named(&mandelbrot(), b"MSVBVM50.DLL");
    assert_eq!(
        deform6::inspect(&bytes, &builtin_table()),
        Err(Refusal::IsVb5)
    );
}

#[test]
fn a_visual_basic_4_runtime_name_is_refused_by_name() {
    let bytes = with_the_runtime_named(&mandelbrot(), b"VB40032.DLL\0");
    assert_eq!(
        deform6::inspect(&bytes, &builtin_table()),
        Err(Refusal::IsVb4)
    );
}

#[test]
fn a_truncated_visual_basic_6_executable_is_damaged_and_not_refused_as_a_non_pe() {
    let data = mandelbrot();
    let half = &data[..data.len() / 2];
    let refusal = deform6::inspect(half, &builtin_table()).unwrap_err();
    assert!(
        matches!(refusal, Refusal::Damaged(_)),
        "a truncated Visual Basic 6 file must be damaged, exit code 4, and not \
         Refusal::NotPe, exit code 1: {refusal:?}"
    );
}

/// The control. Without this test, a bug that made `inspect` refuse every
/// file would leave the seven refusal tests above green, and `AGENTS.md`
/// warns about exactly that failure shape.
#[test]
fn the_unmodified_corpus_file_is_accepted() {
    let data = mandelbrot();
    let report = deform6::inspect(&data, &builtin_table());
    assert!(
        report.is_ok(),
        "the unpatched corpus file was refused: {report:?}"
    );
}
