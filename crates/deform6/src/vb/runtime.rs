//! Which Visual Basic runtime the executable imports.
//!
//! The version comes from the name of the imported runtime DLL, and never
//! from the four byte signature at the head of the Visual Basic header.
//! Visual Basic 5 and Visual Basic 6 both write those four bytes, so the
//! signature cannot tell the two apart. The runtime DLL name is the
//! discriminator the format itself uses, four independent implementations
//! use it, and all 44 corpus executables agree.
//!
//! This module holds a pure classifier over two name lists.

use crate::error::Refusal;

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
/// runs. And `object` resolves a delay load name with a wrapping subtraction
/// and never reads the `attributes` field of the descriptor, while the old
/// Visual C++ 6 delay load format stores virtual addresses in that table
/// rather than relative virtual addresses. The bit that tells the two formats
/// apart is not documented in the Microsoft PE specification.
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{Runtime, VB4_32_DLL, VB5_DLL, VB6_DLL, classify};
    use crate::error::Refusal;

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
