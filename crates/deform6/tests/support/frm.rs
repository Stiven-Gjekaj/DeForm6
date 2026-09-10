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

//! A second, independent `.frm` reader.
//!
//! **This file names nothing from `deform6`.** No `use deform6::...`, no
//! shared type, no shared constant. No import of the library, no shared
//! type, no shared constant. A harness that shared a reader with the
//! library it tests would agree with a bug in that reader, so plan
//! 03-03's differential is only evidence because this file owes it
//! nothing.

use std::path::{Path, PathBuf};

/// Gives the directory that holds the `corpus/` this workspace vendors.
///
/// This is copied from `tests/support/vbp.rs`, not shared with it. Two
/// files under `tests/` are two separate crates to `cargo`; there is no
/// `use` path between them.
#[must_use]
pub fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks a directory recursively and gives every file whose extension
/// matches `ext`, compared without regard to case.
///
/// `corpus/public-domain/SK-MCI-Sample__VB6/MCI.FRM` carries an upper
/// case extension. A reader that compares the extension case-sensitively
/// finds 53 of 54 forms and reports a full pass over a set that is
/// missing one; this is the RED-phase failure this task's own acceptance
/// criteria names, captured in the SUMMARY for this plan.
fn walk_by_extension(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_dir() {
            walk_by_extension(&path, ext, out);
        } else if path
            .extension()
            .is_some_and(|found| found.eq_ignore_ascii_case(ext))
        {
            out.push(path);
        }
    }
}

/// Gives every form under `corpus/`, sorted.
///
/// 54 paths. One of them,
/// `corpus/public-domain/SK-MCI-Sample__VB6/MCI.FRM`, carries an upper
/// case extension. A reader that compares the extension case-sensitively
/// finds 53 of 54 forms and reports a full pass over a set that is
/// missing one.
#[must_use]
pub fn forms() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_by_extension(&corpus_root(), "frm", &mut out);
    out.sort();
    out
}

/// A `.frm` file, read as Latin-1 bytes and held as the exact text the
/// compiler wrote.
pub struct Form {
    pub path: PathBuf,
    pub text: String,
}

impl Form {
    /// Reads a `.frm` file from disk.
    ///
    /// Reads bytes, never a string, then maps each byte to its own
    /// Latin-1 code point. `corpus/vb6-code/Threshold-effect/
    /// Threshold.frm` holds byte 0xA9 at offset 5669; the
    /// string-returning read refuses this file. A reader that skips a
    /// file it cannot decode as UTF-8 scores 53 of 53 and hides the one
    /// it dropped. This is the RED-phase failure this task's own
    /// acceptance criteria names, captured in the SUMMARY for this plan.
    ///
    /// # Panics
    ///
    /// Panics when the file cannot be read. The corpus is vendored and
    /// fixed; a missing form file the caller already resolved is a
    /// harness bug, not a hostile-input case this file must survive.
    #[must_use]
    pub fn read(path: &Path) -> Self {
        let bytes =
            std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
        let text: String = bytes.iter().copied().map(char::from).collect();
        Self {
            path: path.to_owned(),
            text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forms_gives_fifty_four_paths() {
        let found = forms();
        assert_eq!(
            found.len(),
            54,
            "found {} corpus forms, wanted 54",
            found.len()
        );
    }

    #[test]
    fn one_of_the_returned_paths_ends_with_the_upper_case_mci_frm() {
        let found = forms();
        assert!(
            found
                .iter()
                .any(|p| p.to_str().is_some_and(|s| s.ends_with("MCI.FRM"))),
            "expected one corpus form to end with the upper case extension MCI.FRM, found: \
             {found:?}"
        );
    }

    #[test]
    fn form_read_succeeds_on_the_non_utf8_corpus_file_and_keeps_byte_0xa9() {
        let path = corpus_root().join("vb6-code/Threshold-effect/Threshold.frm");
        assert!(
            path.exists(),
            "the corpus vendors vb6-code/Threshold-effect/Threshold.frm"
        );
        let form = Form::read(&path);
        assert!(!form.text.is_empty(), "expected non-empty text");
        let byte_at_5669 = form.text.chars().nth(5669);
        assert_eq!(
            byte_at_5669,
            Some('\u{A9}'),
            "expected byte 0xA9 at offset 5669 to round-trip as U+00A9, found {byte_at_5669:?}"
        );
    }
}
