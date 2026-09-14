//! Uncertainty markers written into the code region of a `.bas`, `.cls` or
//! `.frm` file, after the last `Attribute` line, where an apostrophe
//! comment is legal.
//!
//! `docs/FILE-FORMATS.md` section 2.6 proves, over 7379
//! measured header lines, that a `Begin` block holds no comment and no
//! blank line. Section 2.7 gives the one place a comment is legal: the
//! code region, starting on the line after the last `Attribute` line. This
//! module is the one place doubt about a recovered fact can become a line
//! in a written file. Everywhere else, doubt stays in the JSON report.
//!
//! [`uncertainty_comments`] is the one function this module exposes, and it
//! takes report items, never free text: a call site that could write a
//! comment with no matching report item would break the promise that every
//! comment in the written output traces back to a line in the report.

use crate::report::{Confidence, ReportItem};

/// The one line that opens a block of comments this module writes, naming
/// the tool so a developer who opens the recovered file six months from
/// now can tell these lines from their own comments, and can find every
/// one with a plain text search. A fixed string, never built from a
/// recovered fact, and never written when the block below it is empty.
pub const MARKER: &str = "'DeForm6 could not fully recover the facts named below. \
The JSON report next to this file names every one, with its own evidence.";

/// Turns every item in `items` whose path sits at or under `prefix` into
/// one comment line, opened by one [`MARKER`] line when at least one item
/// matches. Gives an empty list, with no marker line of its own, when no
/// item matches.
///
/// An item whose confidence is [`Confidence::Proven`] produces no line:
/// this module marks only a fact this repository doubts, never one it read
/// directly.
///
/// Every produced line begins with an apostrophe at column zero: there is
/// no indented comment, and no comment written after code on the same
/// line. Every line ending character (`\r`, `\n`) is removed from a
/// recovered basis before it reaches a comment line, because a comment
/// runs to the end of the line the caller's own line writer gives it, and
/// a line ending buried inside recovered text would end the comment early
/// and let the rest of that text become code.
#[must_use]
pub fn uncertainty_comments(items: &[ReportItem], prefix: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for item in items {
        if item.confidence == Confidence::Proven {
            continue;
        }
        if !path_is_under(&item.path, prefix) {
            continue;
        }
        lines.push(comment_line(item));
    }

    if lines.is_empty() {
        return lines;
    }

    let mut block = Vec::with_capacity(lines.len().saturating_add(1));
    block.push(MARKER.to_owned());
    block.append(&mut lines);
    block
}

/// True when `path` is `prefix` itself, or `prefix` followed by a `/`
/// separator: a boundary aware match, so a prefix naming `frmMain` never
/// matches a sibling form's own path, `frmMain2`.
fn path_is_under(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

/// Builds one item's own comment line: an apostrophe at column zero, the
/// item's own path, the basis in plain words with every line ending
/// character stripped out, and the byte offset from the item's own first
/// evidence record.
fn comment_line(item: &ReportItem) -> String {
    let basis = strip_line_endings(&item.basis);
    let offset = item.evidence.first().map_or_else(
        || "no recorded byte offset".to_owned(),
        |evidence| format!("byte offset {:#x}", evidence.offset),
    );
    format!("'{}: {basis} ({offset})", item.path)
}

/// Removes every carriage return and line feed from `text`. A comment runs
/// to the end of the line the caller's own line writer gives it; a line
/// ending buried inside recovered text would end the comment there and let
/// the rest of the text reach the file as code, rather than as the comment
/// this function is building.
fn strip_line_endings(text: &str) -> String {
    text.chars()
        .filter(|ch| *ch != '\r' && *ch != '\n')
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{MARKER, uncertainty_comments};
    use crate::report::{Confidence, Evidence, ReportItem};

    fn item(path: &str, confidence: Confidence, basis: &str, offset: u32) -> ReportItem {
        ReportItem {
            path: path.to_owned(),
            confidence,
            basis: basis.to_owned(),
            evidence: vec![Evidence {
                offset,
                structure: "Test",
                field: "fixture",
                note: None,
            }],
        }
    }

    #[test]
    fn every_produced_line_begins_with_an_apostrophe_at_column_zero() {
        let items = vec![item(
            "/forms/frmMain",
            Confidence::Unrecoverable,
            "the file holds a null where the name would be",
            0x10,
        )];
        let lines = uncertainty_comments(&items, "/forms/frmMain");
        assert!(!lines.is_empty());
        for line in &lines {
            assert!(line.starts_with('\''), "{line:?}");
            assert!(!line.starts_with(" '"), "{line:?}");
        }
    }

    #[test]
    fn an_item_of_the_first_confidence_word_produces_no_line() {
        let items = vec![item(
            "/forms/frmMain",
            Confidence::Proven,
            "the resource blob was read at its own byte offset",
            0x20,
        )];
        let lines = uncertainty_comments(&items, "/forms/frmMain");
        assert!(lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn an_item_of_the_second_word_produces_a_line_carrying_the_path_the_basis_and_an_offset() {
        let items = vec![item(
            "/forms/frmMain",
            Confidence::Inferred,
            "the file declares no startup form",
            0x30,
        )];
        let lines = uncertainty_comments(&items, "/forms/frmMain");
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0], MARKER);
        assert!(lines[1].contains("/forms/frmMain"), "{lines:?}");
        assert!(
            lines[1].contains("the file declares no startup form"),
            "{lines:?}"
        );
        assert!(lines[1].contains("0x30"), "{lines:?}");
    }

    #[test]
    fn an_item_of_the_third_word_produces_a_line_carrying_the_path_the_basis_and_an_offset() {
        let items = vec![item(
            "/forms/frmMain/controls/cmdOk",
            Confidence::Unrecoverable,
            "opcode 9 names no decoder",
            0x40,
        )];
        let lines = uncertainty_comments(&items, "/forms/frmMain");
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines[1].contains("/forms/frmMain/controls/cmdOk"),
            "{lines:?}"
        );
        assert!(lines[1].contains("opcode 9 names no decoder"), "{lines:?}");
        assert!(lines[1].contains("0x40"), "{lines:?}");
    }

    #[test]
    fn a_basis_holding_a_carriage_return_and_a_line_feed_produces_exactly_one_comment_line() {
        let items = vec![item(
            "/forms/frmMain",
            Confidence::Unrecoverable,
            "a recovered string\r\nholding both line ending bytes",
            0x50,
        )];
        let lines = uncertainty_comments(&items, "/forms/frmMain");
        // One marker line, one comment line: never more, since the
        // embedded line ending bytes must never split the comment.
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(!lines[1].contains('\r'), "{lines:?}");
        assert!(!lines[1].contains('\n'), "{lines:?}");
        assert!(
            lines[1].contains("a recovered stringholding both line ending bytes"),
            "{lines:?}"
        );
    }

    #[test]
    fn an_empty_item_list_gives_zero_lines_including_no_marker_line() {
        let lines = uncertainty_comments(&[], "/forms/frmMain");
        assert!(lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn a_prefix_that_matches_nothing_gives_zero_lines() {
        let items = vec![item(
            "/forms/frmOther",
            Confidence::Unrecoverable,
            "a fact about a different form",
            0x60,
        )];
        let lines = uncertainty_comments(&items, "/forms/frmMain");
        assert!(lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn a_prefix_that_matches_some_items_and_not_others_gives_only_the_matching_ones() {
        let items = vec![
            item(
                "/forms/frmMain",
                Confidence::Unrecoverable,
                "a fact about frmMain itself",
                0x70,
            ),
            item(
                "/forms/frmOther",
                Confidence::Unrecoverable,
                "a fact about a different form",
                0x80,
            ),
            item(
                "/forms/frmMain/controls/cmdOk",
                Confidence::Inferred,
                "a fact about a control inside frmMain",
                0x90,
            ),
        ];
        let lines = uncertainty_comments(&items, "/forms/frmMain");
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(lines.iter().any(|line| line.contains("frmMain itself")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("a control inside frmMain"))
        );
        assert!(!lines.iter().any(|line| line.contains("different form")));
    }

    #[test]
    fn a_prefix_never_matches_a_sibling_form_whose_name_it_is_a_string_prefix_of() {
        let items = vec![item(
            "/forms/frmMain2",
            Confidence::Unrecoverable,
            "a fact about a sibling form with a similar name",
            0xA0,
        )];
        let lines = uncertainty_comments(&items, "/forms/frmMain");
        assert!(lines.is_empty(), "{lines:?}");
    }
}
