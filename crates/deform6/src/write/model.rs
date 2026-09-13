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
///
/// The `<= 0xFF` bound makes this legality guarantee hold on its own,
/// unconditionally: without it, a character above `U+00FF` that
/// `is_alphanumeric` still accepts survives this check unchanged, and only
/// [`encode_windows_1252`], called afterward inside [`SafeName::new`],
/// replaces it with `?`. That `?` is neither a legal VB6 identifier
/// character nor a legal Windows file-name character, so the guarantee this
/// function exists to give would depend on an invariant enforced several
/// modules away (every string `crate::vb::Report` carries is itself decoded
/// with `char::from(byte)`, so it can never hold a character above `U+00FF`
/// in practice) rather than on this function itself. [`SafeName::new`] is a
/// `pub fn` any future caller, including a fuzz harness, can call with an
/// arbitrary `&str`.
fn is_legal_identifier_char(ch: char) -> bool {
    (ch.is_alphanumeric() && (ch as u32) <= 0xFF) || ch == '_'
}

// --- Plan 04-01, Task 3: the complete `ProjectModel` -----------------------

use crate::report::{Confidence, ReportItem};
use crate::vb::classify::ObjectKind;
use crate::vb::controltree::ControlKind;
use crate::vb::functyp::Prototype;
use crate::vb::propstream::PropertyValue;
use crate::vb::{ControlReport, ObjectProcedures, ProcedureEntry, Report};

/// Every fact the five writers (`vbp`, `frm`, `code`, `values`, `comment`)
/// and the report builder (`report.rs`, plan 04-06) need from one recovered
/// [`Report`]. Built once by [`from_report`]. No plan after this task adds
/// a field here: a writer that finds it needs one more fact must find it
/// already present, or the model was not complete.
///
/// | Field | Consumer |
/// |---|---|
/// | `name` | `write::vbp` (the `.vbp` file's own name), `report.rs` (the JSON report's own file name) |
/// | `forms` | `write::frm` (one `.frm`/`.frx` pair per entry), `write::vbp` (`Form=` lines) |
/// | `code` | `write::code` (one `.bas`/`.cls` per entry), `write::vbp` (`Module=`/`Class=` lines) |
/// | `startup` | `write::vbp` (the `Startup=` line) |
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectModel {
    /// The project's own safe name: the `.vbp` file's own stem and the
    /// JSON report's own file name.
    pub name: SafeName,
    /// Every form the project declares, in [`Report::forms`]'s own order.
    pub forms: Vec<FormModel>,
    /// Every standard module and class the project declares, in
    /// [`Report::objects`]'s own order (skipping every `Form`-kind entry,
    /// which is joined into `forms` instead).
    pub code: Vec<CodeModel>,
    /// Which form starts the project, or [`Startup::SubMain`] when it has
    /// none.
    pub startup: Startup,
}

/// Which form starts the project.
///
/// | Field | Consumer |
/// |---|---|
/// | `Form`/`SubMain` | `write::vbp` (the `Startup=` line's own value) |
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Startup {
    /// The named form starts the project: the first form in object table
    /// order, when the project declares at least one.
    Form(SafeName),
    /// The project declares no form at all; `"Sub Main"` starts it.
    SubMain,
}

/// One form: its own control tree, its own procedures (its code region's
/// `Sub`/`Function`/`Property` signatures), and every resource blob its
/// controls carry, in the order the `.frx` writer must pack them.
///
/// | Field | Consumer |
/// |---|---|
/// | `name` | `write::frm` (`Attribute VB_Name`, the `.frm`/`.frx` file names), `write::vbp` (`Form=`, `Startup=`) |
/// | `tree_refused` | `write::frm` (an honestly empty `Begin...End` vs a genuinely empty one), `report.rs` |
/// | `controls` | `write::frm` (the whole `Begin...End` tree) |
/// | `procedures` | `write::code` (the form's own code region signatures) |
/// | `blobs` | `write::frm` (the `.frx` file's own byte layout) |
#[derive(Clone, Debug, PartialEq)]
pub struct FormModel {
    /// The form's own safe name.
    pub name: SafeName,
    /// `true` when this form's own control tree walk refused (a
    /// `StructureUnreadable` defect naming `"ControlTree"`), as distinct
    /// from a form that is genuinely empty. `04-RESEARCH.md` Pitfall 2:
    /// both arrive with an empty `controls` list, and only this flag tells
    /// them apart.
    pub tree_refused: bool,
    /// Every control this form's own tree holds, in the same depth-first
    /// order [`FormReport::controls`] gives them: index 0 is always the
    /// form's own outermost block.
    pub controls: Vec<ControlModel>,
    /// The form's own procedures, joined from the `Form`-kind object table
    /// entry that shares this form's own name.
    pub procedures: Vec<ProcedureModel>,
    /// Every resource blob this form's own controls carry, in control tree
    /// order: the order the `.frx` writer must pack them in.
    pub blobs: Vec<BlobRef>,
}

/// One control: its own name, its own type, its own place in the tree, and
/// the properties its own stream decoded.
///
/// | Field | Consumer |
/// |---|---|
/// | `name` | `write::frm` (the `Begin` line's own control name, `Index=`) |
/// | `kind` | `write::frm` (the `Begin` line's own `VB.<Name>` class) |
/// | `array_index` | `write::frm` (the `Index=` property) |
/// | `parent` | `write::frm` (the tree's own nesting) |
/// | `depth` | `write::frm` (indent, and the [`MAX_NESTING_DEPTH`] check) |
/// | `is_menu` | `write::frm` (the menus-last ordering rule, plan 04-04) |
/// | `is_external` | `write::frm` (the `Object.` prefix and the OCX colour branch, plan 04-04) |
/// | `properties` | `write::values` (plan 04-02, every property line) |
#[derive(Clone, Debug, PartialEq)]
pub struct ControlModel {
    /// The control's own safe name.
    pub name: SafeName,
    /// The control's own type.
    pub kind: ControlKind,
    /// The control array `Index`, when this control is one element of an
    /// array: the reader's own value, or one this model generated in issue
    /// order when the reader gave none and the name still repeats.
    pub array_index: Option<u16>,
    /// The index of this control's own parent in the same
    /// [`FormModel::controls`] list. `None` for the form itself.
    pub parent: Option<usize>,
    /// This control's own nesting depth, computed once from the parent
    /// chain: `0` for the form itself. Never recomputed by a writer.
    pub depth: usize,
    /// `true` for the menu control kind.
    pub is_menu: bool,
    /// `true` for the external (OCX) control kind.
    pub is_external: bool,
    /// Every property this control's own stream decoded, in stream order.
    pub properties: Vec<PropertyValue>,
}

/// One standard module or one class: its own name and its own procedures.
///
/// | Field | Consumer |
/// |---|---|
/// | `name` | `write::code` (`Attribute VB_Name`, the file name), `write::vbp` (`Module=`/`Class=`) |
/// | `kind` | `write::code` (`.bas` vs `.cls`, the preamble's own `VB_Creatable`/`VB_PredeclaredId`), `write::vbp` (`Module=` vs `Class=`) |
/// | `procedures` | `write::code` (the code region's own signatures) |
#[derive(Clone, Debug, PartialEq)]
pub struct CodeModel {
    /// The module's or the class's own safe name.
    pub name: SafeName,
    /// Whether this is a standard module or a class.
    pub kind: CodeKind,
    /// Every procedure this object's own arrays gave, joined by index.
    pub procedures: Vec<ProcedureModel>,
}

/// Whether a [`CodeModel`] is a standard module or a class: the `.bas`
/// versus `.cls` split, and the two attribute values that differ between
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeKind {
    /// A `.bas` standard module.
    Module,
    /// A `.cls` class.
    Class,
}

/// One procedure slot: its own recovered name, when it is public, and its
/// own recovered prototype, when the type descriptor array resolved one.
///
/// | Field | Consumer |
/// |---|---|
/// | `name`/`prototype` | `write::code` (plan 04-05: an empty-body `Sub`/`Function`/`Property` with this signature, or nothing at all for a private slot) |
#[derive(Clone, Debug, PartialEq)]
pub struct ProcedureModel {
    /// The procedure's own recovered name. `None` for a private slot: per
    /// OBJ-06, nothing is invented, no name and no placeholder.
    pub name: Option<String>,
    /// The procedure's own recovered prototype, when the type descriptor
    /// array resolved one at the same index. `None` when it did not, or
    /// when `name` is already `None`.
    pub prototype: Option<Prototype>,
}

/// Which kind of `.frx` record a [`BlobRef`] names.
///
/// One variant today: the picture record, the only kind
/// [`crate::vb::propstream::PropertyValue::Blob`] ever names, since
/// `crate::vb::propstream` has no reader yet for the `$` long string record
/// or the list record (`04-RESEARCH.md`'s own resolved open question 3).
/// Plan 04-04 widens this the day a second kind's own reader exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlobKind {
    /// The inline picture record: `crate::vb::frx::extract_blob`'s own
    /// eight byte header plus the image bytes.
    Picture,
}

/// One resource blob one form's own controls carry, in control tree order:
/// the order the `.frx` writer must pack the matching bytes in.
///
/// | Field | Consumer |
/// |---|---|
/// | every field | `write::frm` (plan 04-04: the `.frx` writer re-reads `source_offset..source_offset + 4 + declared_len` out of the executable's own bytes, the same range `crate::vb::propstream::PropertyValue::Blob` names, and writes the property line naming `frx_offset`) |
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobRef {
    /// The index, in [`FormModel::controls`], of the control this blob
    /// belongs to.
    pub control_index: usize,
    /// The property this blob is the value of, such as `"Icon"`.
    pub property_name: String,
    /// The absolute file offset of the blob's own four byte length field
    /// in the executable.
    pub source_offset: u32,
    /// The blob's own declared length (`blobLen`).
    pub declared_len: u32,
    /// The `.frx` offset [`crate::vb::frx::BlobCursor::take`] already gave
    /// this blob, during the same read pass that recovered
    /// [`crate::vb::propstream::PropertyValue::Blob`]. This model never
    /// computes an offset a second way; see that type's own doc comment
    /// for why the writer's own future pass (plan 04-04) still calls
    /// `BlobCursor::take` again, over its own output order, rather than
    /// reusing this field directly.
    pub frx_offset: u32,
    /// Which kind of `.frx` record this blob is.
    pub kind: BlobKind,
}

/// Builds the complete [`ProjectModel`] a recovered [`Report`] describes,
/// plus every [`ReportItem`] a choice this function made produced.
///
/// `data` is the executable's own bytes. This function does not read them
/// itself (every [`BlobRef`] carries only the range a future writer must
/// re-read); it takes `data` so its own signature already matches the one
/// plan 04-04's `.frx` writer will need, and so a caller never has to
/// thread the bytes through two different model-building calls.
///
/// # Determinism
///
/// Every name this function issues comes from one [`SafeNameIssuer`],
/// walked in [`Report::forms`] then [`Report::objects`] order: two calls
/// over the same `report` give the same [`ProjectModel`], field for field,
/// because nothing here reads a hash keyed map to decide an order.
#[must_use]
pub fn from_report(report: &Report, _data: &[u8]) -> (ProjectModel, Vec<ReportItem>) {
    let mut items = Vec::new();
    let mut names = SafeNameIssuer::new();

    let project_name = names.issue(&report.project_name, NameKind::Project).0;

    let mut forms = Vec::new();
    for form_report in &report.forms {
        let (controls, mut control_items) = build_controls(&mut names, &form_report.controls);
        items.append(&mut control_items);

        let form_name = match controls.first() {
            Some(root) => root.name.clone(),
            None => names.issue(&form_report.name, NameKind::Form).0,
        };

        let tree_refused = form_report
            .defects
            .iter()
            .any(|defect| defect.site.structure == "ControlTree");

        let matching_object = report
            .objects
            .iter()
            .find(|object| object.kind == ObjectKind::Form && object.name == form_report.name);
        let procedures = match matching_object {
            Some(object) => procedures_from(&object.procedures),
            None => {
                items.push(ReportItem {
                    path: format!("/forms/{}", form_name.as_str()),
                    confidence: Confidence::Unrecoverable,
                    basis: "no object table entry of kind Form shares this form's own name; \
                            its procedures cannot be recovered"
                        .to_owned(),
                    evidence: Vec::new(),
                });
                Vec::new()
            }
        };

        let blobs = blob_refs(&form_report.controls);

        forms.push(FormModel {
            name: form_name,
            tree_refused,
            controls,
            procedures,
            blobs,
        });
    }

    for object in &report.objects {
        if object.kind == ObjectKind::Form {
            let has_form_report = report.forms.iter().any(|form| form.name == object.name);
            if !has_form_report {
                let (name, _faults) = names.issue(&object.name, NameKind::Form);
                items.push(ReportItem {
                    path: format!("/forms/{}", name.as_str()),
                    confidence: Confidence::Unrecoverable,
                    basis: "an object table entry of kind Form names no matching GUI table \
                            entry; it has no control tree"
                        .to_owned(),
                    evidence: Vec::new(),
                });
            }
        }
    }

    let mut code = Vec::new();
    for object in &report.objects {
        match object.kind {
            ObjectKind::Module => {
                let (name, _faults) = names.issue(&object.name, NameKind::Module);
                code.push(CodeModel {
                    name,
                    kind: CodeKind::Module,
                    procedures: procedures_from(&object.procedures),
                });
            }
            ObjectKind::Class => {
                let (name, _faults) = names.issue(&object.name, NameKind::Class);
                code.push(CodeModel {
                    name,
                    kind: CodeKind::Class,
                    procedures: procedures_from(&object.procedures),
                });
            }
            ObjectKind::Unknown(raw) => {
                let (name, _faults) = names.issue(&object.name, NameKind::Module);
                items.push(ReportItem {
                    path: format!("/objects/{}", name.as_str()),
                    confidence: Confidence::Inferred,
                    basis: format!(
                        "the object's own type value {raw:#010x} is not one this repository \
                         classifies; it is treated as a module"
                    ),
                    evidence: Vec::new(),
                });
                code.push(CodeModel {
                    name,
                    kind: CodeKind::Module,
                    procedures: procedures_from(&object.procedures),
                });
            }
            ObjectKind::Form => {}
        }
    }

    let startup = match forms.first() {
        Some(form) => {
            items.push(ReportItem {
                path: crate::report::META_PATH.to_owned(),
                confidence: Confidence::Inferred,
                basis: "the executable does not declare a startup form; the first form in \
                        object table order was chosen"
                    .to_owned(),
                evidence: Vec::new(),
            });
            Startup::Form(form.name.clone())
        }
        None => Startup::SubMain,
    };

    (
        ProjectModel {
            name: project_name,
            forms,
            code,
            startup,
        },
        items,
    )
}

/// Builds every [`ControlModel`] one form's own tree holds, resolving a
/// control array `Index` the reader gave none for, per this task's own
/// rule: use the reader's own index where it has one, generate one in
/// issue order where it has none and the name still repeats.
fn build_controls(
    names: &mut SafeNameIssuer,
    controls: &[ControlReport],
) -> (Vec<ControlModel>, Vec<ReportItem>) {
    let mut items = Vec::new();

    // How many controls share each raw name, counted with an ordered
    // `Vec`, never a hash keyed map: this list only ever answers "does
    // this name repeat", not "in what order", so a `Vec` scanned linearly
    // is both correct and simple for the corpus's own small control counts.
    let mut name_counts: Vec<(String, usize)> = Vec::new();
    for control in controls {
        match name_counts
            .iter_mut()
            .find(|(name, _)| *name == control.name)
        {
            Some(entry) => entry.1 = entry.1.saturating_add(1),
            None => name_counts.push((control.name.clone(), 1)),
        }
    }

    let mut generated_counters: Vec<(String, u16)> = Vec::new();
    let mut models = Vec::with_capacity(controls.len());

    for (index, control) in controls.iter().enumerate() {
        // Only the root control (the form itself, index 0) maps to a file
        // name, so only it goes through the shared, collision-resolving
        // issuer. A repeated name among the rest is a legitimate VB6
        // control array, told apart by `Index`, never by a file-uniqueness
        // suffix: two array elements are meant to share one name.
        let (name, _faults) = if index == 0 {
            names.issue(&control.name, name_kind_for_control(&control.kind))
        } else {
            SafeName::new(&control.name, name_kind_for_control(&control.kind))
        };

        let repeats = name_counts
            .iter()
            .find(|(existing, _)| *existing == control.name)
            .is_some_and(|(_, count)| *count > 1);

        let array_index = match control.array_index {
            Some(value) => Some(value),
            None if repeats => {
                let value = match generated_counters
                    .iter_mut()
                    .find(|(existing, _)| *existing == control.name)
                {
                    Some(entry) => {
                        let issued = entry.1;
                        entry.1 = entry.1.saturating_add(1);
                        issued
                    }
                    None => {
                        generated_counters.push((control.name.clone(), 1));
                        0
                    }
                };
                items.push(ReportItem {
                    path: format!("/forms/*/controls/{}", name.as_str()),
                    confidence: Confidence::Inferred,
                    basis: "the file gives no Index for this repeated control name; one was \
                            generated in issue order"
                        .to_owned(),
                    evidence: Vec::new(),
                });
                Some(value)
            }
            None => None,
        };

        let depth = control_depth(controls, index);

        models.push(ControlModel {
            name,
            kind: control.kind,
            array_index,
            parent: control.parent,
            depth,
            is_menu: matches!(control.kind, ControlKind::Menu),
            is_external: matches!(control.kind, ControlKind::External),
            properties: control.properties.clone(),
        });
    }

    (models, items)
}

/// Gives the [`NameKind`] a control's own name is issued under: `Form` for
/// the form or MDIForm root block, `Control` for everything else.
fn name_kind_for_control(kind: &ControlKind) -> NameKind {
    if matches!(kind, ControlKind::Form | ControlKind::MdiForm) {
        NameKind::Form
    } else {
        NameKind::Control
    }
}

/// Gives the depth of `controls[index]`: the number of `parent` hops back
/// to the root, which is depth `0`. Bounded by `controls.len()`, the same
/// defensive bound `deform6-cli`'s own `control_depth` uses: a cycle in
/// `parent` links must not loop this walk forever, though no corpus
/// program produces one.
fn control_depth(controls: &[ControlReport], index: usize) -> usize {
    let mut depth = 0_usize;
    let mut current = index;
    for _ in 0..=controls.len() {
        let Some(parent) = controls.get(current).and_then(|control| control.parent) else {
            return depth;
        };
        depth = depth.saturating_add(1);
        current = parent;
    }
    depth
}

/// Reshapes an already-joined [`ObjectProcedures`] (the read side already
/// joined the recovered names with the recovered prototypes, by index)
/// into the [`ProcedureModel`] list the writers in this module need.
fn procedures_from(procedures: &ObjectProcedures) -> Vec<ProcedureModel> {
    match procedures {
        ObjectProcedures::Slots(slots) => slots
            .iter()
            .map(|slot| match slot {
                ProcedureEntry::Public { name, prototype } => ProcedureModel {
                    name: Some(name.clone()),
                    prototype: prototype.clone(),
                },
                ProcedureEntry::Private => ProcedureModel {
                    name: None,
                    prototype: None,
                },
            })
            .collect(),
        ObjectProcedures::NoNameArray { .. } => Vec::new(),
    }
}

/// Collects every resource blob `controls` holds, in control tree order:
/// the order [`FormModel::blobs`] must pack the matching `.frx` bytes in.
fn blob_refs(controls: &[ControlReport]) -> Vec<BlobRef> {
    let mut blobs = Vec::new();
    for (index, control) in controls.iter().enumerate() {
        for property in &control.properties {
            if let PropertyValue::Blob {
                name,
                offset,
                declared_len,
                frx_offset,
                ..
            } = property
            {
                blobs.push(BlobRef {
                    control_index: index,
                    property_name: name.clone(),
                    source_offset: *offset,
                    declared_len: *declared_len,
                    frx_offset: *frx_offset,
                    kind: BlobKind::Picture,
                });
            }
        }
    }
    blobs
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{
        CodeKind, Confidence, LineWriter, NameFault, NameKind, SafeName, SafeNameIssuer, Startup,
        encode_windows_1252, from_report,
    };
    use crate::error::{Defect, DefectKind, Site};
    use crate::read::region::Off;
    use crate::vb::classify::ObjectKind;
    use crate::vb::controltree::ControlKind;
    use crate::vb::propstream::PropertyValue;
    use crate::vb::runtime::Runtime;
    use crate::vb::{ControlReport, FormReport, ObjectProcedures, ObjectReport, Report};

    /// Builds a minimal, valid [`Report`] over the given objects and
    /// forms. Every other field carries a literal this module's own model
    /// building never reads, so a test can hold on to exactly the two
    /// lists it cares about, per `AGENTS.md`'s "build the state a test
    /// needs inside the test".
    fn minimal_report(objects: Vec<ObjectReport>, forms: Vec<FormReport>) -> Report {
        Report {
            file_len: 0,
            section_count: 0,
            runtime: Runtime::Vb6,
            runtime_dll: "MSVBVM60.DLL".to_owned(),
            signature: *b"VB5!",
            header_offset: Off::new(0),
            runtime_build: 0,
            project_name: "TestProject".to_owned(),
            title: String::new(),
            exe_name: String::new(),
            help_file: String::new(),
            native: true,
            object_count: u16::try_from(objects.len()).unwrap_or(0),
            objects,
            declarations: Vec::new(),
            components: Vec::new(),
            forms,
            defects: Vec::new(),
        }
    }

    /// Builds a minimal [`ControlReport`] naming `name` and `kind`, with
    /// every optional field empty and no properties.
    fn minimal_control(name: &str, kind: ControlKind, parent: Option<usize>) -> ControlReport {
        ControlReport {
            name: name.to_owned(),
            kind,
            array_index: None,
            parent,
            properties: Vec::new(),
            external: None,
            external_reason: None,
            ocx_header: None,
            opaque_message: None,
            events: Vec::new(),
        }
    }

    /// Builds a minimal [`ObjectReport`] naming `name` and `kind`, with no
    /// procedures and no gaps.
    fn minimal_object(name: &str, kind: ObjectKind) -> ObjectReport {
        ObjectReport {
            name: name.to_owned(),
            kind,
            procedures: ObjectProcedures::Slots(Vec::new()),
            gaps: Vec::new(),
        }
    }

    /// Builds the [`Defect`] `compose_form` records when a form's own
    /// control tree walk refuses.
    fn control_tree_defect() -> Defect {
        Defect {
            site: Site {
                offset: 0,
                rva: None,
                structure: "ControlTree",
                field: "read",
            },
            kind: DefectKind::StructureUnreadable {
                offset: 0,
                reason: "synthetic, built inside the test".to_owned(),
            },
        }
    }

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
    fn safe_name_rejects_a_character_above_u_00ff_and_never_produces_a_question_mark() {
        // WR-02: `char::is_alphanumeric` alone accepts any Unicode letter
        // or digit, not only this crate's own Latin-1-as-code-point range
        // (0x00 to 0xFF). Before this fix, such a character survived the
        // legality pass unchanged, and only `encode_windows_1252`, called
        // afterward inside `SafeName::new`, replaced it with '?' -- a
        // character that is neither a legal VB6 identifier character nor a
        // legal Windows file-name character.
        let (name, faults) = SafeName::new("\u{100}bc", NameKind::Control);
        assert!(
            !name.as_str().contains('?'),
            "a SafeName must never contain '?': {:?}",
            name.as_str()
        );
        assert!(
            faults.iter().any(|fault| matches!(
                fault,
                NameFault::IllegalCharacter {
                    position: 0,
                    character: '\u{100}'
                }
            )),
            "{faults:?}"
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

    // --- Plan 04-01, Task 3: `ProjectModel` ---------------------------------

    #[test]
    fn a_project_with_zero_forms_gives_sub_main_and_no_form_bearing_entry() {
        let report = minimal_report(Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        assert!(model.forms.is_empty());
        assert_eq!(model.startup, Startup::SubMain);
    }

    #[test]
    fn a_project_with_at_least_one_form_gives_startup_form_and_an_inferred_item() {
        let form = FormReport {
            name: "frmMain".to_owned(),
            controls: vec![minimal_control("frmMain", ControlKind::Form, None)],
            defects: Vec::new(),
        };
        let object = minimal_object("frmMain", ObjectKind::Form);
        let report = minimal_report(vec![object], vec![form]);

        let (model, items) = from_report(&report, &[]);

        assert_eq!(model.forms.len(), 1);
        let Startup::Form(name) = &model.startup else {
            panic!("expected Startup::Form, got {:?}", model.startup);
        };
        assert_eq!(name.as_str(), "frmMain");
        assert!(
            items
                .iter()
                .any(|item| item.confidence == Confidence::Inferred
                    && item.basis.contains("does not declare a startup form")),
            "{items:?}"
        );
    }

    #[test]
    fn a_form_with_an_empty_control_list_and_a_control_tree_defect_gives_tree_refused_true() {
        let form = FormReport {
            name: "frmBroken".to_owned(),
            controls: Vec::new(),
            defects: vec![control_tree_defect()],
        };
        let report = minimal_report(Vec::new(), vec![form]);
        let (model, _items) = from_report(&report, &[]);
        assert_eq!(model.forms.len(), 1);
        assert!(model.forms[0].tree_refused);
    }

    #[test]
    fn a_form_with_an_empty_control_list_and_no_defect_gives_tree_refused_false() {
        let form = FormReport {
            name: "frmEmpty".to_owned(),
            controls: Vec::new(),
            defects: Vec::new(),
        };
        let report = minimal_report(Vec::new(), vec![form]);
        let (model, _items) = from_report(&report, &[]);
        assert_eq!(model.forms.len(), 1);
        assert!(!model.forms[0].tree_refused);
    }

    #[test]
    fn a_repeated_control_name_with_no_reader_supplied_index_gets_generated_indexes_in_issue_order()
    {
        let form = FormReport {
            name: "frmArray".to_owned(),
            controls: vec![
                minimal_control("frmArray", ControlKind::Form, None),
                minimal_control("Text1", ControlKind::TextBox, Some(0)),
                minimal_control("Text1", ControlKind::TextBox, Some(0)),
                minimal_control("Text1", ControlKind::TextBox, Some(0)),
            ],
            defects: Vec::new(),
        };
        let report = minimal_report(Vec::new(), vec![form]);
        let (model, items) = from_report(&report, &[]);

        let text_controls: Vec<&super::ControlModel> = model.forms[0]
            .controls
            .iter()
            .filter(|control| control.name.as_str() == "Text1")
            .collect();
        assert_eq!(text_controls.len(), 3);
        let indexes: Vec<Option<u16>> = text_controls.iter().map(|c| c.array_index).collect();
        assert_eq!(indexes, vec![Some(0), Some(1), Some(2)]);

        let generated_items = items
            .iter()
            .filter(|item| item.basis.contains("generated in issue order"))
            .count();
        assert_eq!(generated_items, 3, "{items:?}");
    }

    #[test]
    fn a_control_with_its_own_reader_given_index_is_never_overwritten() {
        let form = FormReport {
            name: "frmArray".to_owned(),
            controls: vec![
                minimal_control("frmArray", ControlKind::Form, None),
                minimal_control("Text1", ControlKind::TextBox, Some(0)),
            ],
            defects: Vec::new(),
        };
        let mut controls = form.controls.clone();
        if let Some(text) = controls.get_mut(1) {
            text.array_index = Some(7);
        }
        let form = FormReport { controls, ..form };
        let report = minimal_report(Vec::new(), vec![form]);
        let (model, _items) = from_report(&report, &[]);
        assert_eq!(
            model.forms[0].controls.get(1).and_then(|c| c.array_index),
            Some(7)
        );
    }

    #[test]
    fn an_object_of_unknown_kind_becomes_a_module_and_names_the_raw_type_value() {
        let object = minimal_object("Weird1", ObjectKind::Unknown(0xDEAD_BEEF));
        let report = minimal_report(vec![object], Vec::new());
        let (model, items) = from_report(&report, &[]);

        assert_eq!(model.code.len(), 1);
        assert_eq!(model.code[0].kind, CodeKind::Module);
        assert!(
            items.iter().any(|item| item.basis.contains("0xdeadbeef")),
            "{items:?}"
        );
    }

    #[test]
    fn a_form_object_with_no_matching_form_report_gives_an_unrecoverable_item() {
        let object = minimal_object("frmOrphan", ObjectKind::Form);
        let report = minimal_report(vec![object], Vec::new());
        let (_model, items) = from_report(&report, &[]);
        assert!(
            items
                .iter()
                .any(|item| item.confidence == Confidence::Unrecoverable
                    && item.path.contains("frmOrphan")),
            "{items:?}"
        );
    }

    #[test]
    fn a_form_report_with_no_matching_object_gives_an_unrecoverable_item_and_still_builds_a_form() {
        let form = FormReport {
            name: "frmNoObject".to_owned(),
            controls: vec![minimal_control("frmNoObject", ControlKind::Form, None)],
            defects: Vec::new(),
        };
        let report = minimal_report(Vec::new(), vec![form]);
        let (model, items) = from_report(&report, &[]);

        assert_eq!(model.forms.len(), 1, "the form must still be built");
        assert!(
            items
                .iter()
                .any(|item| item.confidence == Confidence::Unrecoverable
                    && item.path.contains("frmNoObject")),
            "{items:?}"
        );
    }

    #[test]
    fn a_project_with_an_empty_object_list_gives_no_code_and_no_panic() {
        let report = minimal_report(Vec::new(), Vec::new());
        let (model, _items) = from_report(&report, &[]);
        assert!(model.code.is_empty());
    }

    #[test]
    fn control_depth_is_computed_once_from_the_parent_chain() {
        let form = FormReport {
            name: "frmNested".to_owned(),
            controls: vec![
                minimal_control("frmNested", ControlKind::Form, None),
                minimal_control("Frame1", ControlKind::Frame, Some(0)),
                minimal_control("Command1", ControlKind::CommandButton, Some(1)),
            ],
            defects: Vec::new(),
        };
        let report = minimal_report(Vec::new(), vec![form]);
        let (model, _items) = from_report(&report, &[]);
        let depths: Vec<usize> = model.forms[0].controls.iter().map(|c| c.depth).collect();
        assert_eq!(depths, vec![0, 1, 2]);
    }

    #[test]
    fn is_menu_and_is_external_are_true_only_for_their_own_control_kind() {
        let form = FormReport {
            name: "frmMenu".to_owned(),
            controls: vec![
                minimal_control("frmMenu", ControlKind::Form, None),
                minimal_control("mnuFile", ControlKind::Menu, Some(0)),
                minimal_control("wsPop", ControlKind::External, Some(0)),
                minimal_control("Command1", ControlKind::CommandButton, Some(0)),
            ],
            defects: Vec::new(),
        };
        let report = minimal_report(Vec::new(), vec![form]);
        let (model, _items) = from_report(&report, &[]);
        let controls = &model.forms[0].controls;
        assert!(!controls[0].is_menu && !controls[0].is_external);
        assert!(controls[1].is_menu && !controls[1].is_external);
        assert!(!controls[2].is_menu && controls[2].is_external);
        assert!(!controls[3].is_menu && !controls[3].is_external);
    }

    #[test]
    fn blob_refs_are_collected_in_control_tree_order() {
        let mut root = minimal_control("frmPics", ControlKind::Form, None);
        root.properties = vec![PropertyValue::Blob {
            name: "Icon".to_owned(),
            offset: 0x100,
            declared_len: 20,
            image_len: 12,
            format: crate::vb::frx::ImageFormat::Unknown(Vec::new()),
            frx_offset: 0,
        }];
        let mut child = minimal_control("Picture1", ControlKind::PictureBox, Some(0));
        child.properties = vec![PropertyValue::Blob {
            name: "Picture".to_owned(),
            offset: 0x200,
            declared_len: 30,
            image_len: 22,
            format: crate::vb::frx::ImageFormat::Unknown(Vec::new()),
            frx_offset: 24,
        }];
        let form = FormReport {
            name: "frmPics".to_owned(),
            controls: vec![root, child],
            defects: Vec::new(),
        };
        let report = minimal_report(Vec::new(), vec![form]);
        let (model, _items) = from_report(&report, &[]);

        let blobs = &model.forms[0].blobs;
        assert_eq!(blobs.len(), 2);
        assert_eq!(blobs[0].control_index, 0);
        assert_eq!(blobs[0].property_name, "Icon");
        assert_eq!(blobs[1].control_index, 1);
        assert_eq!(blobs[1].property_name, "Picture");
    }

    #[test]
    fn two_runs_over_the_same_report_give_an_equal_model_and_equal_items() {
        let form = FormReport {
            name: "frmMain".to_owned(),
            controls: vec![minimal_control("frmMain", ControlKind::Form, None)],
            defects: Vec::new(),
        };
        let object = minimal_object("frmMain", ObjectKind::Form);
        let report = minimal_report(vec![object], vec![form]);

        let (first_model, first_items) = from_report(&report, &[]);
        let (second_model, second_items) = from_report(&report, &[]);
        assert_eq!(first_model, second_model);
        assert_eq!(first_items, second_items);
    }
}
