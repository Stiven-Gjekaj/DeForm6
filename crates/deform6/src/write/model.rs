//! The write-side primitives every emitter in this phase shares: the
//! Windows-1252 encoder, the CRLF line writer, and [`SafeName`], the one
//! way a recovered name becomes a file path.
//!
//! Plan 04-01 fills the primitives this task needs. Plan 04-02 completes
//! the legality rule [`SafeName::new`] applies (`NameKind`, `NameFault`,
//! the collision and clamp rules, recorded as faults). Plan 04-03 completes
//! `ProjectModel` and the rest of the model tree. This file exists now so
//! that every writer in this phase names a name through one type, from its
//! very first commit.
//!
//! # The encoder is the exact inverse of the reader's own convention
//!
//! `crate::vb::vbstr` decodes every byte as its own Latin-1 code point:
//! `bytes.iter().copied().map(char::from).collect()`. [`encode_windows_1252`]
//! is the exact functional inverse: a character at or below `\u{FF}` writes
//! back the byte it came from, and anything above is replaced with a
//! question mark and reported. A standards correct Windows-1252 codec
//! disagrees with this reader in the byte range `0x80` to `0x9F`, so this
//! crate never takes a dependency on one; see `04-RESEARCH.md`'s
//! "Alternatives Considered" for the full argument.

/// The longest name any writer in this phase emits, counted in encoded
/// bytes, never in Unicode scalar values and never in grapheme clusters.
/// `.planning/research/FILE-FORMATS.md` section 7.3: the IDE truncates a
/// control or a class name over this length silently, and the failure
/// shows up later, in a log file the user may never open.
pub const MAX_NAME_LEN: usize = 40;

/// The deepest a control tree may nest before the IDE refuses to load it.
/// `.planning/research/FILE-FORMATS.md` section 7.3.
pub const MAX_NESTING_DEPTH: usize = 7;

/// The longest inline string this phase writes without falling back to the
/// `.frx`. `.planning/research/FILE-FORMATS.md` section 3.3: the corpus's
/// longest inline string is 97 characters, and the threshold above it is
/// not resolved, so this constant states the proven floor, not a guessed
/// ceiling.
pub const MAX_INLINE_STRING_LEN: usize = 97;

/// Encodes `text` as Windows-1252 bytes, the exact inverse of this crate's
/// own `char::from(byte)` read convention.
///
/// Gives the encoded bytes and the list of characters that could not be
/// represented: each substituted character becomes a literal `?` byte in
/// the output, and is also returned so a caller can report the
/// substitution. `.planning/research/FILE-FORMATS.md` section 6.1 states
/// the rule for the emitter: never fall back to UTF-8, replace the
/// character, and record the substitution.
#[must_use]
pub fn encode_windows_1252(text: &str) -> (Vec<u8>, Vec<char>) {
    let mut bytes = Vec::with_capacity(text.len());
    let mut substituted = Vec::new();
    for ch in text.chars() {
        let code = ch as u32;
        if code <= 0xFF {
            #[allow(clippy::cast_possible_truncation)] // code <= 0xFF checked above
            bytes.push(code as u8);
        } else {
            bytes.push(b'?');
            substituted.push(ch);
        }
    }
    (bytes, substituted)
}

/// Accumulates lines and terminates every one with the two bytes `0x0D
/// 0x0A`, including the last, and writes no byte order mark.
///
/// `.planning/research/FILE-FORMATS.md` section 6.2: every line of every
/// text file this phase writes ends with CRLF, the file itself ends with
/// CRLF, and no file carries a bare `LF`. Section 6.3: no file carries a
/// byte order mark. This is the one place this phase writes either fact,
/// so every emitter shares it rather than re-deriving it.
#[derive(Default)]
pub struct LineWriter {
    bytes: Vec<u8>,
    substituted: Vec<char>,
}

impl LineWriter {
    /// Starts an empty writer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Encodes `line` as Windows-1252 and appends it, followed by `0x0D
    /// 0x0A`. Every substituted character is recorded.
    pub fn push_line(&mut self, line: &str) {
        let (encoded, substituted) = encode_windows_1252(line);
        self.bytes.extend_from_slice(&encoded);
        self.bytes.extend_from_slice(b"\r\n");
        self.substituted.extend(substituted);
    }

    /// Gives the accumulated bytes and every substituted character, in the
    /// order the substitutions occurred.
    #[must_use]
    pub fn finish(self) -> (Vec<u8>, Vec<char>) {
        (self.bytes, self.substituted)
    }
}

/// A recovered name, made safe to become a file path, a `.vbp` component
/// line, an `Attribute VB_Name` value, and a path key in the JSON report.
///
/// Plan 04-02 completes the full legality rule (`NameKind`, the fault
/// list, the collision suffix). This task's own version applies the
/// minimum rule the threat model needs from the first commit: no path
/// separator, no `..` sequence, no NUL byte, and a leading letter, so that
/// no writer this task adds can reach a raw [`crate::vb::Report`] string to
/// build a path. [`SafeName::raw`] carries the original string forward for
/// the report's own evidence; it is not used to build a path or a `.frm`
/// line by any writer in this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafeName {
    value: String,
    raw: String,
}

impl SafeName {
    /// Builds a [`SafeName`] from a raw recovered string.
    ///
    /// Every character that is not a letter, a digit or an underscore
    /// becomes an underscore. A name that does not start with a letter
    /// takes a leading `A`. An empty name becomes `Unnamed`. The result is
    /// clamped to [`MAX_NAME_LEN`] encoded bytes, a byte count under this
    /// crate's own Latin-1-as-code-point convention, never a count of
    /// Unicode scalar values and never a count of grapheme clusters.
    #[must_use]
    pub fn new(raw: &str) -> Self {
        let mut value: String = raw
            .chars()
            .map(|ch| {
                if is_legal_identifier_char(ch) {
                    ch
                } else {
                    '_'
                }
            })
            .collect();

        if value.is_empty() {
            value = "Unnamed".to_owned();
        }
        if !value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
        {
            value = format!("A{value}");
        }

        let (mut encoded, _substituted) = encode_windows_1252(&value);
        if encoded.len() > MAX_NAME_LEN {
            encoded.truncate(MAX_NAME_LEN);
        }
        let value = encoded.iter().copied().map(char::from).collect();

        Self {
            value,
            raw: raw.to_owned(),
        }
    }

    /// Gives the safe name, the only form any writer in this crate uses to
    /// build a `.vbp` component line, an `Attribute VB_Name` value, or a
    /// `Begin` block's own control name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Gives the original recovered string. Its one consumer is a report
    /// item's own evidence; no writer in this crate uses it to build a
    /// path or a line.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Builds the file name this safe name owns, with `extension` appended
    /// after a literal dot. This is the only way this phase builds a file
    /// name.
    #[must_use]
    pub fn file_name(&self, extension: &str) -> String {
        format!("{}.{extension}", self.value)
    }
}

/// A letter, a digit, or an underscore: the set [`SafeName::new`] never
/// replaces.
fn is_legal_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{LineWriter, SafeName, encode_windows_1252};

    #[test]
    fn encode_windows_1252_inverts_char_from_byte_for_every_byte_value() {
        for byte in 0u32..=0xFF {
            let ch = char::from_u32(byte).expect("every byte value is a valid char");
            let (bytes, substituted) = encode_windows_1252(&ch.to_string());
            assert!(substituted.is_empty());
            assert_eq!(bytes, vec![u8::try_from(byte).unwrap()]);
        }
    }

    #[test]
    fn encode_windows_1252_substitutes_a_character_above_0xff_and_reports_it() {
        let (bytes, substituted) = encode_windows_1252("caf\u{e9}\u{20ac}");
        assert_eq!(bytes, b"caf\xe9?");
        assert_eq!(substituted, vec!['\u{20ac}']);
    }

    #[test]
    fn line_writer_terminates_every_line_including_the_last_with_crlf() {
        let mut writer = LineWriter::new();
        writer.push_line("VERSION 5.00");
        writer.push_line("Begin VB.Form frmFire ");
        let (bytes, _substituted) = writer.finish();
        assert!(bytes.ends_with(b"\r\n"));
        assert_eq!(bytes.windows(2).filter(|w| *w == b"\r\n").count(), 2);
        assert!(!bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    }

    #[test]
    fn safe_name_passes_through_a_legal_identifier_unchanged() {
        let name = SafeName::new("frmFire");
        assert_eq!(name.as_str(), "frmFire");
        assert_eq!(name.raw(), "frmFire");
    }

    #[test]
    fn safe_name_replaces_a_path_separator_a_dot_dot_and_a_nul_byte() {
        let name = SafeName::new("../../etc/passwd\0");
        assert!(!name.as_str().contains('/'));
        assert!(!name.as_str().contains('\\'));
        assert!(!name.as_str().contains(".."));
        assert!(!name.as_str().contains('\0'));
    }

    #[test]
    fn safe_name_gives_a_leading_letter_to_a_name_that_starts_with_a_digit() {
        let name = SafeName::new("1Bad");
        assert!(
            name.as_str()
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic())
        );
    }

    #[test]
    fn safe_name_clamps_to_forty_encoded_bytes() {
        let raw = "A".repeat(60);
        let name = SafeName::new(&raw);
        assert_eq!(name.as_str().len(), super::MAX_NAME_LEN);
    }

    #[test]
    fn safe_name_file_name_appends_the_extension_after_a_dot() {
        let name = SafeName::new("frmFire");
        assert_eq!(name.file_name("frm"), "frmFire.frm");
        assert_eq!(name.file_name("frx"), "frmFire.frx");
    }
}
