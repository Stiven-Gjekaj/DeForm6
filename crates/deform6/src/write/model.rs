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

/// Which of the five kinds of Visual Basic component a [`SafeName`] names.
///
/// Threaded through [`SafeName::new`] because the generated name for an
/// empty recovery differs by kind, and a reader of the report must be able
/// to tell which kind of thing a generated or faulted name belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameKind {
    /// The whole project: the `.vbp` file's own name.
    Project,
    /// A `.frm` form.
    Form,
    /// A `.bas` standard module.
    Module,
    /// A `.cls` class.
    Class,
    /// A control on a form.
    Control,
}

/// Gives the generated name [`SafeName::new`] uses for an empty recovered
/// name of `kind`. No wildcard arm: a sixth kind is a compile error until
/// this table names its own generated word.
fn generated_name(kind: NameKind) -> &'static str {
    match kind {
        NameKind::Project => "UnnamedProject",
        NameKind::Form => "UnnamedForm",
        NameKind::Module => "UnnamedModule",
        NameKind::Class => "UnnamedClass",
        NameKind::Control => "UnnamedControl",
    }
}

/// One change [`SafeName::new`] or [`SafeNameIssuer::issue`] had to make to
/// land a raw recovered string on a legal, safe name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameFault {
    /// A character at `position` (a character index into the raw string,
    /// never a byte offset) was not a letter, a digit or an underscore, and
    /// was replaced with an underscore.
    IllegalCharacter {
        /// The character index the illegal character was found at.
        position: usize,
        /// The character that was replaced.
        character: char,
    },
    /// The name did not start with a letter, so a leading underscore (when
    /// there was one) was removed and a leading `A` was added.
    LeadingNonLetter,
    /// The raw recovered name was empty, so a generated name for its own
    /// [`NameKind`] was used instead.
    EmptyName,
    /// The name was over [`MAX_NAME_LEN`] encoded bytes and was clamped to
    /// it.
    Clamped {
        /// The encoded byte count before clamping.
        original_len: usize,
    },
    /// The name collided with a name already issued, and a numeric suffix
    /// was appended to make it distinct.
    Collided {
        /// The name this one collided with, before the suffix was added.
        with: String,
    },
}

/// A recovered name, made safe to become a file path, a `.vbp` component
/// line, an `Attribute VB_Name` value, and a path key in the JSON report.
///
/// `ProjectModel`, `FormModel`, `ControlModel` and `CodeModel` (plan 04-03)
/// carry no bare `String` name field beside their own `SafeName`: a writer
/// that wants a path has [`SafeName::file_name`] and nothing else.
/// [`SafeName::raw`] carries the original string forward for the report's
/// own evidence only; it is never used to build a path or a `.frm` line by
/// any writer in this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafeName {
    value: String,
    raw: String,
}

impl SafeName {
    /// Builds a [`SafeName`] from a raw recovered string of kind `kind`,
    /// applying every rule in this fixed order, and recording a
    /// [`NameFault`] for each rule that fired:
    ///
    /// 1. An empty raw name becomes [`generated_name`] for `kind`, and
    ///    records [`NameFault::EmptyName`]. Nothing else in this list fires
    ///    for an empty name, since [`generated_name`] is already legal.
    /// 2. Every character that is not a letter, a digit or an underscore
    ///    becomes an underscore, and each one records a
    ///    [`NameFault::IllegalCharacter`] naming its own character index
    ///    and the character found there.
    /// 3. A name that does not start with a letter has its own leading
    ///    underscore (when it has one) removed, then takes a leading `A`,
    ///    and records [`NameFault::LeadingNonLetter`].
    /// 4. The result is clamped to [`MAX_NAME_LEN`] encoded bytes — a byte
    ///    count under this crate's own Latin-1-as-code-point convention,
    ///    never a count of Unicode scalar values and never a count of
    ///    grapheme clusters — and records [`NameFault::Clamped`] with the
    ///    byte count before clamping.
    ///
    /// This order matters: clamping before the leading-letter fix would
    /// give a different 40 byte result than clamping after it, since the
    /// fix can both remove a byte (the leading underscore) and add one
    /// (the leading `A`).
    ///
    /// [`SafeNameIssuer::issue`] applies the fifth rule, the collision
    /// suffix, which needs to see every name already issued and so cannot
    /// live on this pure, single-name constructor.
    #[must_use]
    pub fn new(raw: &str, kind: NameKind) -> (Self, Vec<NameFault>) {
        let mut faults = Vec::new();

        let mut value = if raw.is_empty() {
            faults.push(NameFault::EmptyName);
            generated_name(kind).to_owned()
        } else {
            let mut sanitized = String::with_capacity(raw.len());
            for (position, ch) in raw.chars().enumerate() {
                if is_legal_identifier_char(ch) {
                    sanitized.push(ch);
                } else {
                    sanitized.push('_');
                    faults.push(NameFault::IllegalCharacter {
                        position,
                        character: ch,
                    });
                }
            }
            sanitized
        };

        if !value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
        {
            if value.starts_with('_') {
                value.remove(0);
            }
            value = format!("A{value}");
            faults.push(NameFault::LeadingNonLetter);
        }

        let (mut encoded, _substituted) = encode_windows_1252(&value);
        if encoded.len() > MAX_NAME_LEN {
            faults.push(NameFault::Clamped {
                original_len: encoded.len(),
            });
            encoded.truncate(MAX_NAME_LEN);
        }
        let value: String = encoded.iter().copied().map(char::from).collect();

        (
            Self {
                value,
                raw: raw.to_owned(),
            },
            faults,
        )
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

/// Issues [`SafeName`] values one at a time, making a colliding name
/// distinct from every name already issued.
///
/// Holds every name issued so far in an ordered `Vec`, never a hash keyed
/// map: the collision suffix depends on the order names are issued in, and
/// an iteration order that varies per process would make the written
/// report vary per process too (RPT-01).
#[derive(Default)]
pub struct SafeNameIssuer {
    issued: Vec<String>,
}

impl SafeNameIssuer {
    /// Starts an issuer with no names issued yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a [`SafeName`] from `raw`, then, only if it collides with a
    /// name this issuer already gave out, appends a numeric suffix inside
    /// [`MAX_NAME_LEN`] and records [`NameFault::Collided`] naming the
    /// name it collided with.
    pub fn issue(&mut self, raw: &str, kind: NameKind) -> (SafeName, Vec<NameFault>) {
        let (mut safe, mut faults) = SafeName::new(raw, kind);

        if self.issued.iter().any(|name| name == safe.as_str()) {
            let original = safe.as_str().to_owned();
            let mut suffix: u32 = 1;
            loop {
                let candidate = suffixed(&original, suffix);
                if !self.issued.contains(&candidate) {
                    safe.value = candidate;
                    faults.push(NameFault::Collided { with: original });
                    break;
                }
                suffix = suffix.saturating_add(1);
            }
        }

        self.issued.push(safe.as_str().to_owned());
        (safe, faults)
    }
}

/// Appends `suffix` to `base`, truncating `base` first so the result never
/// exceeds [`MAX_NAME_LEN`] encoded bytes.
fn suffixed(base: &str, suffix: u32) -> String {
    let suffix_text = suffix.to_string();
    let budget = MAX_NAME_LEN.saturating_sub(suffix_text.len());
    let (mut encoded, _substituted) = encode_windows_1252(base);
    encoded.truncate(budget);
    let truncated: String = encoded.iter().copied().map(char::from).collect();
    format!("{truncated}{suffix_text}")
}

/// A letter, a digit, or an underscore: the set [`SafeName::new`] never
/// replaces.
///
/// `char::is_alphanumeric` is Unicode aware, not ASCII only: a Latin-1
/// letter such as `é` counts as legal here, the same way this crate's own
/// `char::from(byte)` read convention treats it as an ordinary letter, not
/// as punctuation to strip. This crate's own byte-counting convention still
/// applies once the name reaches [`encode_windows_1252`]: `é` (`U+00E9`)
/// encodes to the single byte `0xE9`, never to the two UTF-8 bytes Rust's
/// own `str::len` would count.
fn is_legal_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{LineWriter, NameFault, NameKind, SafeName, SafeNameIssuer, encode_windows_1252};

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
    fn safe_name_passes_through_a_legal_identifier_unchanged_and_records_no_fault() {
        let (name, faults) = SafeName::new("frmFire", NameKind::Form);
        assert_eq!(name.as_str(), "frmFire");
        assert_eq!(name.raw(), "frmFire");
        assert!(faults.is_empty(), "{faults:?}");
    }

    #[test]
    fn safe_name_replaces_a_path_separator_a_dot_dot_and_a_nul_byte_and_names_them_as_faults() {
        let (name, faults) = SafeName::new("../../etc/passwd\0", NameKind::Control);
        assert!(!name.as_str().contains('/'));
        assert!(!name.as_str().contains('\\'));
        assert!(!name.as_str().contains(".."));
        assert!(!name.as_str().contains('\0'));
        let illegal: Vec<char> = faults
            .iter()
            .filter_map(|fault| match fault {
                NameFault::IllegalCharacter { character, .. } => Some(*character),
                NameFault::LeadingNonLetter
                | NameFault::EmptyName
                | NameFault::Clamped { .. }
                | NameFault::Collided { .. } => None,
            })
            .collect();
        assert!(illegal.contains(&'/'), "{faults:?}");
        assert!(illegal.contains(&'.'), "{faults:?}");
        assert!(illegal.contains(&'\0'), "{faults:?}");
    }

    #[test]
    fn safe_name_gives_a_leading_letter_to_a_name_that_starts_with_a_digit_and_records_a_fault() {
        let (name, faults) = SafeName::new("1Bad", NameKind::Control);
        assert!(
            name.as_str()
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic())
        );
        assert!(faults.contains(&NameFault::LeadingNonLetter), "{faults:?}");
    }

    #[test]
    fn safe_name_clamps_a_forty_one_byte_name_to_forty_and_records_the_original_byte_count() {
        let raw = "A".repeat(41);
        let (name, faults) = SafeName::new(&raw, NameKind::Control);
        assert_eq!(name.as_str().len(), super::MAX_NAME_LEN);
        assert!(
            faults.contains(&NameFault::Clamped { original_len: 41 }),
            "{faults:?}"
        );
    }

    #[test]
    fn safe_name_counts_a_character_above_0x7f_as_one_encoded_byte_not_as_two_utf8_bytes() {
        // 'e' with an acute accent, U+00E9: two bytes in UTF-8, one byte
        // under this crate's own Latin-1-as-code-point encoding. A clamp
        // decision that measured Rust's own `str::len` (UTF-8 bytes)
        // instead of the encoded byte count would wrongly clamp this 40
        // character name, whose own `str::len` is 41.
        let raw = format!("caf\u{e9}{}", "x".repeat(36));
        assert_eq!(
            raw.chars().count(),
            40,
            "the fixture must hold 40 characters"
        );
        assert_eq!(
            raw.len(),
            41,
            "the fixture's own UTF-8 byte length must differ from its character count"
        );

        let (name, faults) = SafeName::new(&raw, NameKind::Control);

        assert!(
            faults.is_empty(),
            "a name whose encoded byte count is exactly 40 must not clamp: {faults:?}"
        );
        let (encoded, _substituted) = encode_windows_1252(name.as_str());
        assert_eq!(
            encoded.len(),
            40,
            "the encoded byte count must be 40, matching the character count, never the 41 byte \
             UTF-8 length Rust's own str::len would give"
        );
    }

    #[test]
    fn safe_name_gives_a_generated_name_for_its_own_kind_on_an_empty_raw_name() {
        let (name, faults) = SafeName::new("", NameKind::Control);
        assert_eq!(name.as_str(), "UnnamedControl");
        assert!(faults.contains(&NameFault::EmptyName), "{faults:?}");

        let (form_name, _faults) = SafeName::new("", NameKind::Form);
        assert_eq!(form_name.as_str(), "UnnamedForm");
    }

    #[test]
    fn safe_name_file_name_appends_the_extension_after_a_dot() {
        let (name, _faults) = SafeName::new("frmFire", NameKind::Form);
        assert_eq!(name.file_name("frm"), "frmFire.frm");
        assert_eq!(name.file_name("frx"), "frmFire.frx");
    }

    #[test]
    fn two_names_that_clamp_to_the_same_forty_bytes_are_made_distinct_by_the_issuer() {
        let mut issuer = SafeNameIssuer::new();
        // Exactly 40 bytes: issued unchanged, with no fault of its own.
        let short = "A".repeat(40);
        // 45 bytes: clamps to the same 40 `A`s the first name already took.
        let long = "A".repeat(45);

        let (first, first_faults) = issuer.issue(&short, NameKind::Control);
        let (second, second_faults) = issuer.issue(&long, NameKind::Control);

        assert_eq!(first.as_str(), short);
        assert!(first_faults.is_empty(), "{first_faults:?}");

        assert_ne!(first.as_str(), second.as_str());
        assert!(
            second_faults
                .iter()
                .any(|fault| matches!(fault, NameFault::Collided { .. })),
            "{second_faults:?}"
        );
        assert!(second.as_str().len() <= super::MAX_NAME_LEN);
    }

    #[test]
    fn no_model_type_in_this_file_holds_a_bare_string_name_field() {
        // A compile-time invariant, not a runtime assertion: SafeName's own
        // `value` field is private to this module, and the only way any
        // other module reaches a name is `as_str`, `raw` or `file_name`.
        // This test exists so a future reader who adds a public `String`
        // name field beside a `SafeName` sees a place that says why not to.
        let (name, _faults) = SafeName::new("frmFire", NameKind::Form);
        assert_eq!(
            name.as_str(),
            name.file_name("frm").trim_end_matches(".frm")
        );
    }
}
