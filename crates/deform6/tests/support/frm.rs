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

    /// Parses this form's text into its root `Begin`/`End` blocks. See
    /// [`parse_blocks`].
    #[must_use]
    pub fn blocks(&self) -> Vec<Block> {
        parse_blocks(&self.text)
    }
}

/// One `Name = Value` line inside a control block or a property block.
/// The name is trimmed and the value is trimmed, with a leading and
/// trailing double quote removed when both are present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub value: String,
}

/// A `BeginProperty ... EndProperty` block. Its only fixed field is the
/// block name, for example `Font`; it also carries the properties and
/// any further-nested property blocks declared directly inside it. A
/// property block never appears in a [`Block`]'s `children` list: the
/// grammar keeps property blocks and control blocks in two separate
/// child lists on purpose, so a caller can never mistake a `Font` block
/// for a control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyBlock {
    pub name: String,
    pub properties: Vec<Property>,
    pub property_blocks: Vec<PropertyBlock>,
}

/// A `Begin ... End` control block: the class and the name as written on
/// the `Begin` line, the properties declared directly inside it, the
/// property blocks declared inside it, and the child control blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub class: String,
    pub name: String,
    pub properties: Vec<Property>,
    pub property_blocks: Vec<PropertyBlock>,
    pub children: Vec<Block>,
}

/// Splits a `Name = Value` line at its first `=` sign, trims both sides,
/// and drops a leading and trailing double quote when both are present.
/// Gives `None` for a line with no `=`.
///
/// Splits on the first `=` only. `Caption = "A = B"` carries an `=`
/// inside its own value; splitting on every `=` sign truncates it. This
/// is the RED-phase failure this task's own acceptance criteria names,
/// captured in the SUMMARY for this plan.
fn split_property_line(line: &str) -> Option<(String, String)> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let (name, value) = line.split_once('=')?;
    let name = name.trim().to_owned();
    let value = value.trim();
    let value = if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].to_owned()
    } else {
        value.to_owned()
    };
    Some((name, value))
}

/// One entry on the parse stack: either an open control block or an
/// open property block, so `End` and `EndProperty` each close only their
/// own kind.
enum Frame {
    Control(Block),
    Property(PropertyBlock),
}

/// Parses a `.frm` file's text into its root `Begin ... End` blocks.
///
/// Reads the file line by line. Trims each line: this parser does not
/// depend on the exact indentation and does not depend on exactly one
/// space before an equals sign, matching the shape of the file rather
/// than one form of it. A line that starts with `Begin ` opens a
/// control block; `BeginProperty ` opens a property block; `End` closes
/// the innermost control block; `EndProperty` closes the innermost
/// property block; any other line holding an `=` becomes a property on
/// whichever block is innermost. The file reads a fixed, vendored
/// corpus, so a malformed file is a repository fault and a panic-free
/// silent skip is the wrong response for one; this parser instead drops
/// a close with no matching open, which a whole-corpus count test would
/// catch as a shortfall.
#[must_use]
pub fn parse_blocks(text: &str) -> Vec<Block> {
    let mut roots: Vec<Block> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();

        if let Some(rest) = line.strip_prefix("BeginProperty ") {
            let name = rest.trim().to_owned();
            stack.push(Frame::Property(PropertyBlock {
                name,
                properties: Vec::new(),
                property_blocks: Vec::new(),
            }));
            continue;
        }

        if let Some(rest) = line.strip_prefix("Begin ") {
            let mut words = rest.split_whitespace();
            let class = words.next().unwrap_or("").to_owned();
            let name = words.next().unwrap_or("").to_owned();
            stack.push(Frame::Control(Block {
                class,
                name,
                properties: Vec::new(),
                property_blocks: Vec::new(),
                children: Vec::new(),
            }));
            continue;
        }

        if line == "EndProperty" {
            let Some(Frame::Property(closed)) = stack.pop() else {
                continue;
            };
            match stack.last_mut() {
                Some(Frame::Control(parent)) => parent.property_blocks.push(closed),
                Some(Frame::Property(parent)) => parent.property_blocks.push(closed),
                None => {}
            }
            continue;
        }

        if line == "End" {
            let Some(Frame::Control(closed)) = stack.pop() else {
                continue;
            };
            match stack.last_mut() {
                Some(Frame::Control(parent)) => parent.children.push(closed),
                _ => roots.push(closed),
            }
            continue;
        }

        if let Some((name, value)) = split_property_line(line) {
            match stack.last_mut() {
                Some(Frame::Control(block)) => block.properties.push(Property { name, value }),
                Some(Frame::Property(block)) => block.properties.push(Property { name, value }),
                None => {}
            }
        }
    }

    roots
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

    #[test]
    fn a_form_with_no_lines_gives_an_empty_text_and_an_empty_block_list() {
        let roots = parse_blocks("");
        assert!(
            roots.is_empty(),
            "an empty form text must give an empty block list, found {roots:?}"
        );
    }

    #[test]
    fn a_begin_inside_a_begin_gives_one_child_on_the_outer_block() {
        let text = "Begin VB.Form Form1\n   Begin VB.CommandButton Command1\n      Caption = \
                     \"OK\"\n   End\nEnd\n";
        let roots = parse_blocks(text);
        assert_eq!(roots.len(), 1, "expected exactly one root block");
        let form = &roots[0];
        assert_eq!(form.class, "VB.Form");
        assert_eq!(form.name, "Form1");
        assert_eq!(
            form.children.len(),
            1,
            "expected the outer block to have exactly one child, found {:?}",
            form.children
        );
        let button = &form.children[0];
        assert_eq!(button.class, "VB.CommandButton");
        assert_eq!(button.name, "Command1");
        assert_eq!(
            button.properties,
            vec![Property {
                name: "Caption".to_owned(),
                value: "OK".to_owned(),
            }]
        );
    }

    #[test]
    fn a_begin_property_font_block_does_not_appear_in_the_child_control_list() {
        let text = "Begin VB.Form Form1\n   BeginProperty Font\n      Name = \"Arial\"\n   \
                     EndProperty\nEnd\n";
        let roots = parse_blocks(text);
        assert_eq!(roots.len(), 1);
        let form = &roots[0];
        assert!(
            form.children.is_empty(),
            "a BeginProperty block must never appear in the control child list, found {:?}",
            form.children
        );
        assert_eq!(form.property_blocks.len(), 1);
        assert_eq!(form.property_blocks[0].name, "Font");
        assert_eq!(
            form.property_blocks[0].properties,
            vec![Property {
                name: "Name".to_owned(),
                value: "Arial".to_owned(),
            }]
        );
    }

    #[test]
    fn a_line_with_four_spaces_before_the_equals_sign_parses_the_same_as_one_space() {
        let one_space = "Begin VB.Form Form1\n   Caption = \"Hi\"\nEnd\n";
        let four_spaces = "Begin VB.Form Form1\n   Caption    =    \"Hi\"\nEnd\n";
        let one = parse_blocks(one_space);
        let four = parse_blocks(four_spaces);
        assert_eq!(
            one[0].properties, four[0].properties,
            "a line with more than one space before the equals sign must parse the same as a \
             line with one space"
        );
    }

    #[test]
    fn a_value_holding_an_equals_sign_is_not_truncated() {
        let text = "Begin VB.Form Form1\n   Caption = \"A = B\"\nEnd\n";
        let roots = parse_blocks(text);
        assert_eq!(
            roots[0].properties,
            vec![Property {
                name: "Caption".to_owned(),
                value: "A = B".to_owned(),
            }],
            "a value holding an equals sign must not be truncated at the first inner ="
        );
    }

    /// Recursively counts every property named `name` under `block`,
    /// including every descendant control block. Property-block
    /// properties (inside a `BeginProperty` block, such as `Font`'s own
    /// `Name`) are not counted: `Index` never appears inside a property
    /// block in this corpus.
    fn count_property(block: &Block, name: &str) -> usize {
        let mut count = block.properties.iter().filter(|p| p.name == name).count();
        for child in &block.children {
            count += count_property(child, name);
        }
        count
    }

    #[test]
    fn the_whole_corpus_holds_forty_eight_index_properties() {
        let mut total = 0usize;
        for path in forms() {
            let form = Form::read(&path);
            for root in form.blocks() {
                total += count_property(&root, "Index");
            }
        }
        assert_eq!(
            total, 48,
            "found {total} Index properties across the whole corpus, wanted 48"
        );
    }

    #[test]
    fn the_whole_corpus_holds_fifty_four_root_form_blocks() {
        let mut total = 0usize;
        for path in forms() {
            let form = Form::read(&path);
            for root in form.blocks() {
                if root.class.starts_with("VB.Form") || root.class.starts_with("VB.MDIForm") {
                    total += 1;
                }
            }
        }
        assert_eq!(
            total, 54,
            "found {total} root form blocks across the whole corpus, wanted 54"
        );
    }
}
