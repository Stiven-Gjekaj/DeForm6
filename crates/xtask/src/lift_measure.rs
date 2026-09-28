//! The measure of the lift against the source that the program was built
//! from.
//!
//! The lift names each variable by its offset, so a statement cannot be
//! compared with its source as text. What both sides can give is a set of
//! tokens: each string literal, and each name after a `.`, such as
//! `.Caption` or `.SendData`. For each procedure, the tokens of its source
//! body are held against the tokens of its lifted statements. A token that
//! both sides give counts as matched, one time for each time that both
//! give it. A procedure that does not lift matches nothing.
//!
//! `Me.` and `VBA.` are removed from both sides first: the lift writes
//! `Me.lblX.Caption` and `VBA.Err().Clear()` where the source can write
//! `lblX.Caption` and `Err.Clear`.
//!
//! The source side comes from `crates/deform6/tests/support/`, the reader
//! that names nothing from `deform6`, so the measure is not the lift held
//! against itself.

use std::collections::BTreeMap;

/// The counts of one program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Measure {
    /// The tokens that both the source and the lift give.
    pub matched: usize,
    /// The tokens of the source.
    pub source: usize,
    /// The tokens of the lift.
    pub lifted: usize,
}

impl Measure {
    /// Adds the counts of one procedure: its source lines and its lifted
    /// lines, which are empty when it does not lift.
    pub(crate) fn add(&mut self, source: &[String], lifted: &[String]) {
        let mut want: BTreeMap<String, usize> = BTreeMap::new();
        for token in source.iter().flat_map(|line| tokens(line)) {
            self.source = self.source.saturating_add(1);
            let count = want.entry(token).or_default();
            *count = count.saturating_add(1);
        }
        for token in lifted.iter().flat_map(|line| tokens(line)) {
            self.lifted = self.lifted.saturating_add(1);
            if let Some(count) = want.get_mut(&token).filter(|count| **count > 0) {
                *count = count.saturating_sub(1);
                self.matched = self.matched.saturating_add(1);
            }
        }
    }

    /// Adds the counts of `other`.
    pub(crate) fn merge(&mut self, other: Self) {
        self.matched = self.matched.saturating_add(other.matched);
        self.source = self.source.saturating_add(other.source);
        self.lifted = self.lifted.saturating_add(other.lifted);
    }
}

/// Tells whether `character` can be part of a Basic name.
const fn is_name(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// Gives `line` with each `Me.` and `VBA.` that starts a name removed: a
/// name starts where no name character and no `.` comes before it.
fn without_owner(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while !rest.is_empty() {
        let starts_a_name = !out
            .chars()
            .last()
            .is_some_and(|before| is_name(before) || before == '.');
        let owner = ["Me.", "VBA."]
            .iter()
            .find(|owner| starts_a_name && rest.starts_with(**owner));
        if let Some(owner) = owner {
            rest = rest.get(owner.len()..).unwrap_or_default();
        } else if let Some(character) = rest.chars().next() {
            out.push(character);
            rest = rest.get(character.len_utf8()..).unwrap_or_default();
        }
    }
    out
}

/// Gives the tokens of one line: each string literal with its quotes, and
/// each name after a `.` in lower case with its `.`.
pub(crate) fn tokens(line: &str) -> Vec<String> {
    let line = without_owner(line);
    let characters: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut at = 0_usize;
    while let Some(&character) = characters.get(at) {
        if character == '"' {
            let mut text = String::from('"');
            at = at.saturating_add(1);
            while let Some(&next) = characters.get(at) {
                at = at.saturating_add(1);
                if next == '"' {
                    if characters.get(at) == Some(&'"') {
                        text.push_str("\"\"");
                        at = at.saturating_add(1);
                        continue;
                    }
                    break;
                }
                text.push(next);
            }
            text.push('"');
            out.push(text);
            continue;
        }
        let after_value = at
            .checked_sub(1)
            .and_then(|before| characters.get(before))
            .is_some_and(|before| is_name(*before) || *before == ')');
        let starts_name = characters
            .get(at.saturating_add(1))
            .is_some_and(char::is_ascii_alphabetic);
        if character == '.' && after_value && starts_name {
            let mut name = String::from('.');
            at = at.saturating_add(1);
            while let Some(&next) = characters.get(at).filter(|next| is_name(**next)) {
                name.push(next.to_ascii_lowercase());
                at = at.saturating_add(1);
            }
            out.push(name);
            continue;
        }
        at = at.saturating_add(1);
    }
    out
}

/// Renders the pins of each program as TOML.
pub(crate) fn render_pins(measures: &BTreeMap<String, Measure>) -> String {
    let mut out = String::from(
        "# The measure of the lift against the source, one entry per P-code program.\n\
         # `check-pcode-table --lift-pins` compares its measure with this file.\n\
         # It holds for a P-code table and a types file derived from `MSVBVM60.DLL`,\n\
         # `VB6.OLB` and `MSWINSCK.OCX`. `crates/xtask/src/lift_measure.rs` gives\n\
         # the tokens.\n",
    );
    for (key, measure) in measures {
        out.push_str(&format!(
            "\n[{key:?}]\nmatched = {}\nsource = {}\nlifted = {}\n",
            measure.matched, measure.source, measure.lifted
        ));
    }
    out
}

/// Reads the pins of a file that [`render_pins`] wrote.
///
/// # Errors
///
/// Gives an error when the text is not TOML, or an entry lacks a count.
pub(crate) fn parse_pins(text: &str) -> Result<BTreeMap<String, Measure>, String> {
    let table: toml::Table = text.parse().map_err(|err| format!("{err}"))?;
    let mut out = BTreeMap::new();
    for (key, value) in table {
        let count = |field: &str| {
            value
                .get(field)
                .and_then(toml::Value::as_integer)
                .and_then(|count| usize::try_from(count).ok())
                .ok_or_else(|| format!("{key} has no {field}"))
        };
        out.insert(
            key.clone(),
            Measure {
                matched: count("matched")?,
                source: count("source")?,
                lifted: count("lifted")?,
            },
        );
    }
    Ok(out)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Measure, parse_pins, render_pins, tokens};

    #[test]
    fn a_line_gives_its_strings_and_the_names_after_a_dot() {
        assert_eq!(
            tokens(r#"lblX.Caption = hScrollX.Value & " px""#),
            [".caption", ".value", "\" px\""]
        );
        assert_eq!(
            tokens(r#"Call Me.WskClient.SendData("Msg ""A""")"#),
            [".senddata", "\"Msg \"\"A\"\"\""]
        );
        assert_eq!(tokens("Call VBA.Err().Clear()"), [".clear"]);
        assert_eq!(tokens("x = 1.5 + Me.field_54"), Vec::<String>::new());
        assert_eq!(tokens("Name.Me.Text"), [".me", ".text"]);
    }

    #[test]
    fn a_token_matches_once_for_each_time_that_both_sides_give_it() {
        let mut measure = Measure::default();
        measure.add(
            &[r#"a.Text = "x""#.to_owned(), "b.Text = c.Text".to_owned()],
            &[r#"Me.a.Text = "x""#.to_owned(), "Me.b.Tag = 1".to_owned()],
        );
        assert_eq!(
            measure,
            Measure {
                matched: 2,
                source: 4,
                lifted: 3
            }
        );
        let mut twice = Measure::default();
        twice.add(&["a.Text = 1".to_owned()], &["a.Text = b.Text".to_owned()]);
        assert_eq!(twice.matched, 1);
        let mut none = Measure::default();
        none.add(&["a.Text = 1".to_owned()], &[]);
        assert_eq!(none.matched, 0);
        assert_eq!(none.source, 1);
    }

    #[test]
    fn the_pins_read_back_as_they_were_written() {
        let mut measures = BTreeMap::new();
        measures.insert(
            "vb6-code/A b/A.exe".to_owned(),
            Measure {
                matched: 1,
                source: 2,
                lifted: 3,
            },
        );
        assert_eq!(parse_pins(&render_pins(&measures)).unwrap(), measures);
        assert!(parse_pins("[\"x\"]\nmatched = 1\n").is_err());
    }
}
