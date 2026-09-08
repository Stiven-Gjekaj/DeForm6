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

//! A second, independent reader for the source an executable was built
//! from: what the author declared public.
//!
//! **This file names nothing from `deform6`.** No `use deform6::...`, no
//! shared type, no shared constant. `AGENTS.md`'s Measurement section
//! requires the recovery ratio to be measured against the original source
//! the executable was built from, never against what the tool itself
//! produced; this file is the reader that gives that original-source side.
//! Per D-06, a grep over this file's non-comment lines proves the point
//! better than a sentence can.
//!
//! # What one line must look like to count
//!
//! A declaration is matched on its own physical line: optional leading
//! whitespace (trimmed away before matching), an optional scope keyword
//! (`Public`, `Private` or `Friend`), an optional `Static`, then one of
//! `Sub`, `Function`, `Property Get`, `Property Let` or `Property Set`,
//! then the procedure's name. A missing scope keyword is treated as
//! `Public`, because that is VB6's own default for a procedure in a form
//! or a class. A getter, a setter and a letter with the same name count as
//! three separate entries, because each is its own declaration line.
//!
//! # A continued argument list is not three declarations
//!
//! A VB6 argument list may wrap onto the next physical line with a
//! trailing line continuation (a space, then `_`, at the very end of the
//! line). `Grayscale-effect/pdOpenSaveDialog.cls` declares
//! `GetOpenFileName` this way, with nine continuation lines carrying its
//! ten optional arguments. Only the first physical line of a declaration
//! is matched; every line that continues the previous one is skipped
//! outright, never handed to the declaration matcher at all. Skipping the
//! continuation lines by construction, rather than trusting that their
//! text happens not to look like a declaration, is what keeps a
//! ten-argument continued declaration from being counted more than once.

use std::path::Path;

/// Says whether a physical line continues onto the next one: VB6 requires
/// a space immediately before the trailing `_`, so this checks the exact
/// two-character suffix rather than a bare `_`, which would also match an
/// identifier that legitimately ends in an underscore with no space
/// before it.
fn continues_to_next_line(line: &str) -> bool {
    line.trim_end().ends_with(" _")
}

/// Matches one already-trimmed, non-continuation physical line as a
/// public procedure declaration, giving its name.
///
/// Gives `None` for a private or friend declaration, for a declaration
/// whose name is empty (which does not occur in real VB6 source, but
/// `Region` has no infallible accessor and neither does a token stream a
/// hostile or merely unusual file could shape however it likes), and for
/// any line that opens no declaration at all: a comment, a blank line, a
/// statement inside a procedure body, or a continuation line the caller
/// has already filtered out before this runs.
fn public_declaration_name(line: &str) -> Option<String> {
    let tokens: Vec<&str> = line.split_whitespace().collect();

    let mut idx = 0;
    let is_public = match tokens.first().copied() {
        Some("Public") => {
            idx += 1;
            true
        }
        Some("Private" | "Friend") => {
            idx += 1;
            false
        }
        // No scope keyword at all: VB6 treats this as Public, per the
        // module doc comment.
        _ => true,
    };

    if tokens.get(idx).copied() == Some("Static") {
        idx += 1;
    }

    let keyword = tokens.get(idx).copied()?;
    let consumed = match keyword {
        "Sub" | "Function" => 1,
        "Property" if matches!(tokens.get(idx + 1).copied(), Some("Get" | "Let" | "Set")) => 2,
        _ => return None,
    };
    idx += consumed;

    // The name sits immediately after the keyword, glued to its opening
    // parenthesis with no space in every corpus source file this reader
    // has seen (`GetImageWidth(ByRef ...`), so the name is the text
    // before the first `(` on that token.
    let raw_name = tokens.get(idx).copied()?;
    let name = raw_name.split('(').next().unwrap_or_default().trim();
    if name.is_empty() {
        return None;
    }

    is_public.then(|| name.to_owned())
}

/// Gives the ordered list of procedures `path` declares public: a
/// subroutine, a function, or a property getter, setter or letter, with
/// no scope keyword or with the `Public` keyword.
///
/// The file is read as Latin-1 bytes and every byte becomes its own code
/// point, the same rule `support/vbp.rs`'s `Project::read` uses for the
/// text it reads: a corpus source file can carry a byte above 127 in a
/// comment or a string literal, and a UTF-8-aware decode would either
/// lose it or refuse the file.
///
/// Gives an empty list when the file cannot be read. This is not the same
/// claim as "the object declares no public procedures": an object whose
/// source the repository does not hold (`Edge-detection`'s orphan
/// `cCommonDialog.cls`, per `support::rules`'s absent-source rule) is
/// dropped from both sides of a comparison by that rule, never counted as
/// zero declared here and then treated as a shortfall.
#[must_use]
pub fn declared_public_procedures(path: &Path) -> Vec<String> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let text: String = bytes.iter().copied().map(char::from).collect();

    let mut out = Vec::new();
    let mut in_continuation = false;
    for line in text.lines() {
        let is_continuation = in_continuation;
        // Whether *this* line itself trails off onto the next one is
        // decided before the `continue` below, so a declaration whose
        // argument list runs to several continuation lines in a row (as
        // `GetOpenFileName` does) stays skipped for every one of them,
        // not just the first.
        in_continuation = continues_to_next_line(line);
        if is_continuation {
            continue;
        }
        if let Some(name) = public_declaration_name(line.trim()) {
            out.push(name);
        }
    }
    out
}
