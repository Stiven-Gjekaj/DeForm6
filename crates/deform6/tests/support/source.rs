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
/// procedure declaration, giving whether it is public and its name.
///
/// Gives `None` for a declaration whose name is empty (which does not
/// occur in real VB6 source, but `Region` has no infallible accessor and
/// neither does a token stream a hostile or merely unusual file could
/// shape however it likes), and for any line that opens no declaration at
/// all: a comment, a blank line, a statement inside a procedure body, or a
/// continuation line the caller has already filtered out before this runs.
fn declaration(line: &str) -> Option<(bool, String)> {
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

    Some((is_public, name.to_owned()))
}

/// Gives the name of every procedure that `path` declares and that `keep`
/// accepts, in the order of the file. `keep` is given whether the
/// declaration is public.
fn procedures(path: &Path, keep: fn(bool) -> bool) -> Vec<String> {
    scoped_procedures(path)
        .into_iter()
        .filter(|(is_public, _)| keep(*is_public))
        .map(|(_, name)| name)
        .collect()
}

/// Gives each procedure that `path` declares, with whether it is public, in
/// the order of the file.
fn scoped_procedures(path: &Path) -> Vec<(bool, String)> {
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
        if let Some(found) = declaration(line.trim()) {
            out.push(found);
        }
    }
    out
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
    procedures(path, |is_public| is_public)
}

/// Gives the ordered list of every procedure `path` declares: public,
/// private or friend, by the same rules as [`declared_public_procedures`].
///
/// Gives an empty list when the file cannot be read.
#[must_use]
pub fn declared_procedures(path: &Path) -> Vec<String> {
    procedures(path, |_is_public| true)
}

/// Gives the bytes that a caller pushes for one argument, from its text in
/// a declaration, such as `ByVal X As Double` or `Optional Name As String`:
/// 4 for an argument by reference or an array, and the size of the value
/// for an argument by value. A `Double`, a `Currency` and a `Date` are 8
/// bytes, a `Variant` is 16, and each other type is 4. An argument with no
/// `As` is a `Variant`.
fn argument_size(argument: &str) -> u32 {
    let text = argument.trim().to_ascii_lowercase();
    let words: Vec<&str> = text.split_whitespace().collect();
    let by_value = words.contains(&"byval");
    let array = text.contains("()");
    let kind = words
        .iter()
        .position(|word| *word == "as")
        .and_then(|at| words.get(at + 1))
        .copied()
        .unwrap_or("variant");
    if !by_value || array {
        return 4;
    }
    match kind {
        "double" | "currency" | "date" => 8,
        "variant" => 16,
        _ => 4,
    }
}

/// Splits an argument list at each comma that no parenthesis holds.
fn arguments(list: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (at, byte) in list.bytes().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => {
                out.push(&list[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    out.push(&list[start..]);
    out.into_iter()
        .filter(|argument| !argument.trim().is_empty())
        .collect()
}

/// Gives the argument list of a declaration: the text between the first
/// `(` and the `)` that closes it.
fn argument_list(declaration: &str) -> &str {
    let Some(open) = declaration.find('(') else {
        return "";
    };
    let mut depth = 0_i32;
    for (at, byte) in declaration.bytes().enumerate().skip(open) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return &declaration[open + 1..at];
                }
            }
            _ => {}
        }
    }
    &declaration[open + 1..]
}

/// Gives each procedure that `path` declares, by the rules of
/// [`declared_procedures`], with the bytes that a caller pushes for it: 4
/// for `Me`, the size of each argument, and 4 for the address of the result
/// of a `Function` or a `Property Get`.
///
/// A declaration that continues onto the next lines is read with those
/// lines. Gives an empty list when the file cannot be read.
#[must_use]
pub fn declared_argument_sizes(path: &Path) -> Vec<(String, u32)> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let text: String = bytes.iter().copied().map(char::from).collect();

    let mut logical = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if continues_to_next_line(line) {
            current.push_str(line.trim_end().trim_end_matches('_'));
            current.push(' ');
        } else {
            current.push_str(line);
            logical.push(std::mem::take(&mut current));
        }
    }

    let mut out = Vec::new();
    for line in logical {
        let line = line.trim();
        let Some((_, name)) = declaration(line) else {
            continue;
        };
        let words: Vec<&str> = line.split_whitespace().collect();
        let returns =
            words.contains(&"Function") || words.windows(2).any(|pair| pair == ["Property", "Get"]);
        let size = 4
            + arguments(argument_list(line))
                .into_iter()
                .map(argument_size)
                .sum::<u32>()
            + if returns { 4 } else { 0 };
        out.push((name, size));
    }
    out
}

/// Gives the text of `line` before its comment: before the first `'` that
/// is outside a string, or empty for a line that opens with `Rem`.
fn without_comment(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed == "Rem" || trimmed.starts_with("Rem ") {
        return "";
    }
    let mut in_string = false;
    for (at, character) in line.char_indices() {
        match character {
            '"' => in_string = !in_string,
            '\'' if !in_string => return &line[..at],
            _ => {}
        }
    }
    line
}

/// Gives the name and the body of each procedure that `path` declares, in
/// the order of [`declared_argument_sizes`]: the logical lines between the
/// declaration and its `End Sub`, `End Function` or `End Property`, trimmed,
/// with no comment and no empty line.
///
/// Gives an empty list when the file cannot be read.
#[must_use]
pub fn procedure_bodies(path: &Path) -> Vec<(String, Vec<String>)> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let text: String = bytes.iter().copied().map(char::from).collect();
    let mut logical = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if continues_to_next_line(line) {
            current.push_str(line.trim_end().trim_end_matches('_'));
            current.push(' ');
        } else {
            current.push_str(line);
            logical.push(std::mem::take(&mut current));
        }
    }
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut inside = false;
    for line in logical {
        let line = without_comment(&line).trim().to_owned();
        if let Some((_, name)) = declaration(&line) {
            out.push((name, Vec::new()));
            inside = true;
        } else if inside {
            if ["End Sub", "End Function", "End Property"]
                .iter()
                .any(|end| line.starts_with(end))
            {
                inside = false;
            } else if !line.is_empty()
                && let Some((_, body)) = out.last_mut()
            {
                body.push(line);
            }
        }
    }
    out
}

/// Gives each procedure that `path` declares, with whether it is public, by
/// the rules of [`declared_procedures`].
#[must_use]
pub fn declared_procedure_scopes(path: &Path) -> Vec<(bool, String)> {
    scoped_procedures(path)
}

/// Gives the number of public variables that `path` declares: each name of
/// a `Public` line before the first procedure, such as the two of
/// `Public X As Long, Y As Long`. A `Public` constant, declaration, type,
/// enumeration or event is not a variable.
///
/// Gives 0 when the file cannot be read.
#[must_use]
pub fn declared_public_variables(path: &Path) -> usize {
    let Ok(bytes) = std::fs::read(path) else {
        return 0;
    };
    let text: String = bytes.iter().copied().map(char::from).collect();
    let mut count = 0;
    for line in text.lines() {
        let line = line.trim();
        if declaration(line).is_some() {
            break;
        }
        let Some(rest) = line.strip_prefix("Public ") else {
            continue;
        };
        let first = rest.split_whitespace().next().unwrap_or_default();
        if matches!(first, "Const" | "Declare" | "Type" | "Enum" | "Event") {
            continue;
        }
        count += arguments(rest).len();
    }
    count
}
