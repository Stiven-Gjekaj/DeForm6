//! Which Visual Basic runtime the executable imports.
//!
//! The version comes from the name of the imported runtime DLL, and never
//! from the four byte signature at the head of the Visual Basic header.
//! Visual Basic 5 and Visual Basic 6 both write those four bytes, so the
//! signature cannot tell the two apart. The runtime DLL name is the
//! discriminator the format itself uses, four independent implementations
//! use it, and all 44 corpus executables agree.
//!
//! This module holds a pure classifier over two name lists, and one wrapper
//! that reads those lists out of a parsed image.

use crate::error::Refusal;
use crate::read::pe::PeImage;

/// The runtime that Visual Basic 6 links against.
pub const VB6_DLL: &str = "MSVBVM60.DLL";

/// The runtime that Visual Basic 5 links against.
pub const VB5_DLL: &str = "MSVBVM50.DLL";

/// The runtime that 32 bit Visual Basic 4 links against.
pub const VB4_32_DLL: &str = "VB40032.DLL";

/// The runtime that 16 bit Visual Basic 4 links against.
///
/// **Nothing compares against this name, and nothing can.** A 16 bit Visual
/// Basic 4 program is an NE executable, not a PE one, so
/// `PeImage::parse` refuses it with [`Refusal::NotPe`] and exit code 1 before
/// any import name is read. The constant records the intent and costs
/// nothing. Do not write a test that asserts a 16 bit file reaches a Visual
/// Basic refusal, because it does not.
pub const VB4_16_DLL: &str = "VB40016.DLL";

/// The runtime that DeForm6 reads.
///
/// There is one variant, and there is no `Vb5` or `Vb4` value, because those
/// are refusals rather than outcomes. A file that DeForm6 reads is a Visual
/// Basic 6 file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Runtime {
    /// The image imports the Visual Basic 6 runtime.
    Vb6,
}

/// Decides the Visual Basic version from a list of imported DLL names.
///
/// This function takes data and never takes a file, so every case it answers
/// is reachable from a literal list in a test.
///
/// The returned `String` is the entry that matched, cloned verbatim out of
/// `imports`. It is not [`VB6_DLL`] and it is not rebuilt from anything.
/// [`Runtime`] has one variant, so a caller that compared the result against
/// `Runtime::Vb6` would learn only that the call returned `Ok`. The name is
/// the value that carries evidence: the sweep over the corpus reads it to
/// show that each file names its runtime, and the command line prints it, so
/// that line reports a value out of the file rather than a literal out of
/// this source.
///
/// # The comparison folds ASCII only
///
/// The import directory stores a name as raw bytes with no case rule, and the
/// Windows loader resolves a module name without regard to case, so a linker
/// writes whatever case it likes. `PeImage::imported_dlls` already folds with
/// `to_ascii_uppercase`, and this function folds again with
/// `eq_ignore_ascii_case`, so it is correct on any caller's list.
///
/// The fold is ASCII and never Unicode. The Unicode uppercase mapping of a
/// stray high byte can produce a longer string and a wrong comparison. All 44
/// corpus executables hold the upper case ASCII name already, so the corpus
/// never exercises the fold. It costs one method call and the counter example
/// is a file nobody has yet.
///
/// # The delay load list never decides
///
/// `delay_loaded` is taken and is deliberately read by no decision branch.
/// Three reasons, and all three hold together.
///
/// No corpus file has a delay load directory at all, in 44 of 44. The Visual
/// Basic 6 linker does not emit one for the runtime, because the runtime
/// entry point is called from the entry stub itself, before any user code
/// runs. And the parser behind `PeImage::delay_loaded_dlls` resolves a delay
/// load name with a wrapping subtraction and never reads the `attributes`
/// field of the descriptor, while the old Visual C++ 6 delay load format
/// stores virtual addresses in that table rather than relative virtual
/// addresses. The bit that tells the two formats apart is not documented in
/// the Microsoft PE specification. The doc comment on
/// `PeImage::delay_loaded_dlls` records the measurement.
///
/// So the list is read and reported as information, and it is given no vote.
/// A file whose only Visual Basic runtime reference is delay loaded is
/// refused, and the command line says the delay load table was seen.
///
/// The argument is taken rather than dropped because that is what makes the
/// property testable. A caller hands this function a delay load list holding
/// the Visual Basic 6 runtime and watches it still refuse. A function that
/// did not take the list could not be asked the question.
///
/// # There is no scan for the signature bytes
///
/// A packed file whose import directory the packer rewrote is refused, and
/// the refusal names the check that failed. There is no fallback that scans
/// the executable sections for the four byte signature. Four bytes occur by
/// chance in the resource section of an unrelated program, and a default that
/// scanned for them would report a Visual Basic 6 project that does not
/// exist. That is a wrong answer stated confidently. Phase 5 adds the scan
/// behind `--salvage`, with every result marked inferred.
///
/// # Errors
///
/// Returns [`Refusal::IsVb5`] or [`Refusal::IsVb4`] when the list names a
/// Visual Basic runtime that DeForm6 does not read, and
/// [`Refusal::NoVbRuntime`] when it names none.
pub fn classify(
    imports: &[String],
    delay_loaded: &[String],
    dot_net: bool,
) -> Result<(Runtime, String), Refusal> {
    // Read once, so the reason above is visible at the one place a future
    // editor would reach for the list.
    let _ = delay_loaded;

    if let Some(name) = imports
        .iter()
        .find(|name| name.eq_ignore_ascii_case(VB6_DLL))
    {
        return Ok((Runtime::Vb6, name.clone()));
    }
    if imports
        .iter()
        .any(|name| name.eq_ignore_ascii_case(VB5_DLL))
    {
        return Err(Refusal::IsVb5);
    }
    if imports
        .iter()
        .any(|name| name.eq_ignore_ascii_case(VB4_32_DLL))
    {
        return Err(Refusal::IsVb4);
    }
    Err(Refusal::NoVbRuntime { dot_net })
}

/// Reads the runtime out of a parsed image.
///
/// This is [`classify`] with the three lists taken from the image. It gives
/// back the pair `classify` gives back, so the matched runtime name reaches
/// the caller.
///
/// # A missing import directory is not damage
///
/// The import walk produces two outcomes and they mean different things to a
/// person sorting a directory of files.
///
/// An error means the data directory names an address that maps nowhere, or
/// the descriptors run past the end of the file. That is [`Refusal::Damaged`]
/// and exit code 4. A truncated Visual Basic 6 file must not be reported as
/// "this holds no Visual Basic runtime".
///
/// A list, empty or not, goes to `classify`. An empty list is what a missing
/// import data directory produces, and `classify` answers it with
/// [`Refusal::NoVbRuntime`] and exit code 2. That is neither an error nor
/// damage. The empty case is not a branch here, because `classify` already
/// holds the rule and a second copy of it would be a second place for it to
/// drift.
///
/// # A packed image
///
/// If a packer rewrote the entry point and the import directory, none of this
/// holds. Three sub-cases, and the answer to each:
///
/// | What the file looks like | What DeForm6 says |
/// |---|---|
/// | No import data directory | [`Refusal::NoVbRuntime`], exit 2 |
/// | The directory address maps nowhere, or the descriptors are truncated | [`Refusal::Damaged`], exit 4 |
/// | The directory reads and names only the stub imports of the packer | [`Refusal::NoVbRuntime`], exit 2 |
///
/// Confidence in the discrimination itself is high. Confidence that the
/// import table is present is a different question, and for a file with no
/// readable import directory DeForm6 has no second opinion in this phase and
/// does not pretend to. **Do not add a fallback here.** Phase 5 adds the
/// salvage path behind a flag, and marks every result it produces inferred.
///
/// # The delay load walk cannot change the answer
///
/// A failure of the delay load walk degrades to an empty list rather than to
/// a refusal. The list decides nothing, so a delay load directory that cannot
/// be read must not be able to move the outcome.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when the import directory cannot be read, and
/// the refusal that [`classify`] gives for the names it finds.
pub fn runtime_of(pe: &PeImage<'_>) -> Result<(Runtime, String), Refusal> {
    let imports = pe
        .imported_dlls()
        .map_err(|_| Refusal::Damaged("the import directory is unreadable"))?;
    let delay_loaded = pe.delay_loaded_dlls().unwrap_or_default();
    classify(&imports, &delay_loaded, pe.has_clr_header())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    clippy::panic,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{Runtime, VB4_32_DLL, VB5_DLL, VB6_DLL, classify, runtime_of};
    use crate::error::Refusal;
    use crate::read::pe::PeImage;

    /// The general purpose corpus file.
    ///
    /// A test inside `src/` reaches a corpus file this way and never through
    /// a path. The library takes a byte slice and names no file system type,
    /// and the grep that proves it does not know a test module from library
    /// code. A file under `tests/` is a separate crate root and may keep a
    /// path helper.
    const MANDELBROT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
    ));

    /// Gives a copy of `data` whose first imported DLL name is `replacement`.
    ///
    /// The offset comes from `PeImage::dll_name_sites`, which resolves the
    /// address in the import descriptor. **It is never a byte search.** A
    /// search for the runtime name could hit the same bytes in the resource
    /// section or in a string table, and the patch would then land on a byte
    /// that no pointer in the file names.
    ///
    /// The replacement must be as long as the name it covers, so a wrong
    /// length is a loud failure rather than a corrupted image. `MSVBVM60.DLL`
    /// is 12 bytes. `VB40032.DLL` is 11, so its replacement carries its own
    /// trailing NUL and the original NUL becomes a second, harmless one.
    ///
    /// Nothing is written to disk. The bytes live for one test function.
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
        // The site must hold the runtime name now, or the patch proves
        // nothing about the runtime.
        assert_eq!(
            String::from_utf8_lossy(&data[at..at + len]).into_owned(),
            VB6_DLL
        );

        let mut out = data.to_vec();
        out[at..at + len].copy_from_slice(replacement);
        out
    }

    /// Gives a copy of `data` whose data directory `index` holds `address`
    /// and `size`.
    ///
    /// Data directory 14 is zero in all 44 corpus executables, so the corpus
    /// gives `PeImage::has_clr_header` no positive case. This builds one in
    /// memory. The same helper, with index 1 and two zeroes, takes the import
    /// data directory away and builds the "no import directory" case.
    ///
    /// The helper asserts that the write changes the entry, so a fixture that
    /// has become a no-op fails loudly rather than passing for the wrong
    /// reason.
    fn with_data_directory(data: &[u8], index: usize, address: u32, size: u32) -> Vec<u8> {
        let mut out = data.to_vec();
        let lfanew = u32::from_le_bytes(out[0x3c..0x40].try_into().unwrap());
        // 24 reaches the optional header from the PE signature. 96 reaches
        // the data directories from the start of a PE32 optional header.
        // Each entry is eight bytes.
        let at = usize::try_from(lfanew).unwrap() + 24 + 96 + index * 8;
        let was_address = u32::from_le_bytes(out[at..at + 4].try_into().unwrap());
        let was_size = u32::from_le_bytes(out[at + 4..at + 8].try_into().unwrap());
        assert_ne!(
            (was_address, was_size),
            (address, size),
            "data directory {index} already holds this value, so the patch proves nothing"
        );

        out[at..at + 4].copy_from_slice(&address.to_le_bytes());
        out[at + 4..at + 8].copy_from_slice(&size.to_le_bytes());
        out
    }

    /// The address of the first section, which is a mapped address in every
    /// corpus file.
    fn the_first_section_address(data: &[u8]) -> u32 {
        PeImage::parse(data)
            .unwrap()
            .sections()
            .first()
            .expect("the corpus file must hold at least one section")
            .virtual_address
            .get()
    }

    #[test]
    fn the_corpus_file_uses_the_visual_basic_6_runtime() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let (runtime, _) = runtime_of(&image).unwrap();
        assert_eq!(runtime, Runtime::Vb6);
    }

    #[test]
    fn the_corpus_file_names_its_runtime_and_the_name_reaches_the_caller() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let (_, matched) = runtime_of(&image).unwrap();
        // The sweep in plan 01-08 reads this value for each of the 44 files,
        // and the command line prints it. It comes out of the file.
        assert_eq!(matched, VB6_DLL);
    }

    #[test]
    fn a_patched_visual_basic_5_runtime_name_is_refused() {
        let bytes = with_the_runtime_named(MANDELBROT, b"MSVBVM50.DLL");
        let image = PeImage::parse(&bytes).unwrap();
        assert_eq!(runtime_of(&image).unwrap_err(), Refusal::IsVb5);
    }

    #[test]
    fn a_patched_32_bit_visual_basic_4_runtime_name_is_refused() {
        // 11 bytes of name and the NUL that ends it, which is 12 in all.
        let bytes = with_the_runtime_named(MANDELBROT, b"VB40032.DLL\0");
        let image = PeImage::parse(&bytes).unwrap();
        assert_eq!(runtime_of(&image).unwrap_err(), Refusal::IsVb4);
    }

    #[test]
    fn a_patched_name_that_is_no_visual_basic_runtime_is_refused() {
        let bytes = with_the_runtime_named(MANDELBROT, b"KERNEL32.DLL");
        let image = PeImage::parse(&bytes).unwrap();
        assert_eq!(
            runtime_of(&image).unwrap_err(),
            Refusal::NoVbRuntime { dot_net: false }
        );
    }

    #[test]
    fn a_truncated_file_is_damaged_and_not_a_file_without_a_runtime() {
        // The parse still succeeds. A successful parse is not evidence of an
        // intact file.
        let half = &MANDELBROT[..MANDELBROT.len() / 2];
        let image = PeImage::parse(half).unwrap();
        let refusal = runtime_of(&image).unwrap_err();
        assert!(
            matches!(refusal, Refusal::Damaged(_)),
            "an unreadable import directory is damage and exit code 4, \
             and it is not the exit code 2 that means no Visual Basic runtime: {refusal:?}"
        );
    }

    #[test]
    fn an_image_with_no_import_data_directory_is_not_damaged() {
        let bytes = with_data_directory(MANDELBROT, 1, 0, 0);
        let image = PeImage::parse(&bytes).unwrap();
        // The list is empty rather than an error, so this is exit code 2.
        assert!(image.imported_dlls().unwrap().is_empty());
        assert_eq!(
            runtime_of(&image).unwrap_err(),
            Refusal::NoVbRuntime { dot_net: false }
        );
    }

    #[test]
    fn a_common_language_runtime_directory_makes_the_refusal_name_a_dot_net_assembly() {
        // Data directory 14 is zero in all 44 corpus files, so a `runtime_of`
        // that passed a hard coded `false` would satisfy every other test in
        // this module. This is the one input that carries the directory.
        let plain = with_the_runtime_named(MANDELBROT, b"KERNEL32.DLL");
        let address = the_first_section_address(&plain);
        // 0x48 is the size of the common language runtime header structure.
        let bytes = with_data_directory(&plain, 14, address, 0x48);

        let image = PeImage::parse(&bytes).unwrap();
        assert!(image.has_clr_header());
        assert_eq!(
            runtime_of(&image).unwrap_err(),
            Refusal::NoVbRuntime { dot_net: true }
        );
    }

    /// Turns a literal list into the owned list the classifier takes.
    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    /// The empty list, for the argument a test does not exercise.
    fn none() -> Vec<String> {
        Vec::new()
    }

    #[test]
    fn a_list_holding_the_visual_basic_6_runtime_classifies_as_version_6() {
        let imports = names(&[VB6_DLL]);
        let (runtime, matched) = classify(&imports, &none(), false).unwrap();
        assert_eq!(runtime, Runtime::Vb6);
        assert_eq!(matched, VB6_DLL);
    }

    #[test]
    fn a_lower_case_runtime_name_matches_and_comes_back_in_the_case_the_file_holds() {
        let imports = names(&["msvbvm60.dll"]);
        let (runtime, matched) = classify(&imports, &none(), false).unwrap();
        assert_eq!(runtime, Runtime::Vb6);
        // The name is the entry out of the list, not the constant. A
        // classifier that returned `VB6_DLL` would pass the test above and
        // fail this one.
        assert_eq!(matched, "msvbvm60.dll");
        assert_ne!(matched, VB6_DLL);
    }

    #[test]
    fn the_visual_basic_5_runtime_is_refused_by_name() {
        let imports = names(&[VB5_DLL]);
        assert_eq!(
            classify(&imports, &none(), false).unwrap_err(),
            Refusal::IsVb5
        );
    }

    #[test]
    fn the_32_bit_visual_basic_4_runtime_is_refused_by_name() {
        let imports = names(&[VB4_32_DLL]);
        assert_eq!(
            classify(&imports, &none(), false).unwrap_err(),
            Refusal::IsVb4
        );
    }

    #[test]
    fn a_list_with_no_visual_basic_runtime_is_refused() {
        let imports = names(&["KERNEL32.DLL"]);
        assert_eq!(
            classify(&imports, &none(), false).unwrap_err(),
            Refusal::NoVbRuntime { dot_net: false }
        );
    }

    #[test]
    fn the_dot_net_flag_travels_into_the_refusal() {
        let imports = names(&["KERNEL32.DLL"]);
        assert_eq!(
            classify(&imports, &none(), true).unwrap_err(),
            Refusal::NoVbRuntime { dot_net: true }
        );
    }

    #[test]
    fn an_empty_import_list_is_refused() {
        assert_eq!(
            classify(&none(), &none(), false).unwrap_err(),
            Refusal::NoVbRuntime { dot_net: false }
        );
    }

    #[test]
    fn a_delay_loaded_visual_basic_6_runtime_never_decides_the_runtime() {
        let imports = names(&["KERNEL32.DLL"]);
        let delay_loaded = names(&[VB6_DLL]);
        assert_eq!(
            classify(&imports, &delay_loaded, false).unwrap_err(),
            Refusal::NoVbRuntime { dot_net: false }
        );
    }

    #[test]
    fn the_matched_name_is_the_runtime_entry_and_not_the_first_entry() {
        let imports = names(&["KERNEL32.DLL", VB6_DLL]);
        let (runtime, matched) = classify(&imports, &none(), false).unwrap();
        assert_eq!(runtime, Runtime::Vb6);
        assert_eq!(matched, VB6_DLL);
        assert_ne!(matched, imports[0]);
    }
}
