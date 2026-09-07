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

//! A second, independent `.vbp` reader.
//!
//! **This file names nothing from `deform6`.** No `use deform6::...`, no
//! shared type, no shared constant. It reads bytes and text with plain
//! `std` and answers questions about a project file on its own terms. A
//! harness that shared a reader with the library it tests would agree
//! with a bug in that reader, so plan 02-08's differential is only
//! evidence because this file owes it nothing. Per D-06, a grep over this
//! file's non-comment lines proves the point better than a sentence can.
//!
//! # The selection rule, in three lines, so plan 02-08 does not reimplement it
//!
//! 1. Scope the search to the executable's own corpus entry directory
//!    (the immediate child of `corpus/vb6-code/` or `corpus/public-domain/`
//!    that holds it) before looking at any project file's `ExeName32`.
//! 2. Inside that scope, select the one project file whose `ExeName32`
//!    equals the executable's file name, compared case-insensitively.
//! 3. Zero matches and two matches are both loud failures. A corpus-wide
//!    index built without the entry-directory scope collides:
//!    `SK-MCI-Sample__VB6/MCI.VBP` and `SK-Gradient-Sample__VB6/Project1.vbp`
//!    both declare `ExeName32="Project1.exe"`, and both projects ship a
//!    `demo/Project1.exe`. Scoping first is not decoration; it is the
//!    difference between comparing a program against its own source and
//!    comparing it against a stranger's.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Gives the directory that holds the `corpus/` this workspace vendors.
///
/// This is copied from `tests/corpus_sweep.rs`, not shared with it. Two
/// files under `tests/` are two separate crates to `cargo`; there is no
/// `use` path between them, and even if there were, D-06 would forbid
/// this file from depending on anything it did not read itself.
#[must_use]
pub fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks a directory recursively and gives every file whose extension
/// matches `ext`, compared case-insensitively.
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

/// Gives every executable under `corpus/`, sorted.
///
/// **Iterate this, never [`project_files`].** The repository holds 45
/// project files and 44 executables:
/// `Brightness-effect/Part 3 - DIBs/Brightness3.vbp` declares
/// `dibBrightness.exe`, which the repository does not vendor. A harness
/// that walked project files would test one project with no binary to
/// check it against.
#[must_use]
pub fn executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_by_extension(&corpus_root(), "exe", &mut out);
    out.sort();
    out
}

/// Gives every project file under `corpus/`, sorted.
#[must_use]
pub fn project_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_by_extension(&corpus_root(), "vbp", &mut out);
    out.sort();
    out
}

/// Gives the corpus entry directory an executable sits under: the
/// immediate child of `corpus/vb6-code/` or `corpus/public-domain/` that
/// contains it.
///
/// Gives `None` when the path is not under either bucket, which never
/// happens for a path [`executables`] returns.
#[must_use]
pub fn entry_dir(exe: &Path) -> Option<PathBuf> {
    let root = corpus_root();
    for bucket in ["vb6-code", "public-domain"] {
        let base = root.join(bucket);
        if let Ok(rel) = exe.strip_prefix(&base) {
            let first = rel.components().next()?;
            return Some(base.join(first));
        }
    }
    None
}

/// Takes the text between the first quote and the last quote on a
/// value, per D-05.
///
/// `Sepia-effect/Sepia.vbp` carries
/// `Title="Sepia / "Antique" Image Filter"`. A reader written as
/// `"([^"]*)"` stops at the first inner quote and returns `Sepia / `
/// without failing. This takes everything between the first quote and
/// the last quote on the line instead, inner quotes included, and
/// returns `None` when there are fewer than two quotes: an absent key
/// is not an empty value.
fn quoted_value(rest: &str) -> Option<String> {
    let first = rest.find('"')?;
    let last = rest.rfind('"')?;
    if last <= first {
        return None;
    }
    Some(rest[first + 1..last].to_owned())
}

/// Splits a `.vbp` line at its first `=` sign, after trimming a
/// trailing `\r`. Gives `None` for a line with no `=`, such as a
/// section header or a blank line.
fn key_value(line: &str) -> Option<(&str, &str)> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    line.split_once('=')
}

/// A `.vbp` file, read as Latin-1 bytes and held as the exact text the
/// compiler wrote.
///
/// Every byte becomes its own Latin-1 code point, the same rule
/// `crates/deform6/src/vb/project.rs` uses for the strings it reads.
/// `Brightness-effect/Part 4 - Even faster DIBs/Brightness.vbp` carries a
/// `©2020 Tanner Helland` copyright line; a UTF-8-aware decode would
/// either lose that line or refuse the file, and neither is correct for
/// a file this corpus vendors on purpose.
pub struct Project {
    pub path: PathBuf,
    text: String,
}

impl Project {
    /// Reads a `.vbp` file from disk.
    ///
    /// # Panics
    ///
    /// Panics when the file cannot be read. The corpus is vendored and
    /// fixed; a missing project file the caller already resolved is a
    /// harness bug, not a hostile-input case this file must survive (see
    /// the module doc comment's note on the accepted threat, T-02-35).
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

    /// Gives the quoted value of a string-valued key, such as `Title`
    /// or `ExeName32`, per D-05's quoting rule. Gives `None` when the
    /// key is absent from the file, which is a different thing from an
    /// empty value: `SK-Gradient-Sample__VB6/Project1.vbp` has no
    /// `Title=` key at all, because the compiler omits it when the
    /// title equals the project name.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<String> {
        for line in self.text.lines() {
            let s = line.trim();
            let Some((k, rest)) = key_value(s) else {
                continue;
            };
            if k == key {
                return quoted_value(rest);
            }
        }
        None
    }
}

/// Why [`select_project_file`] could not name exactly one project file
/// for an executable.
#[derive(Debug)]
pub enum SelectionError {
    /// No project file inside the executable's own entry directory
    /// declares an `ExeName32` matching the executable's file name.
    NoCandidate { exe: PathBuf, entry_dir: PathBuf },
    /// More than one project file inside the entry directory declares a
    /// matching `ExeName32`.
    MultipleCandidates {
        exe: PathBuf,
        candidates: Vec<PathBuf>,
    },
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCandidate { exe, entry_dir } => write!(
                f,
                "no project file under {} declares the executable name {}",
                entry_dir.display(),
                exe.display()
            ),
            Self::MultipleCandidates { exe, candidates } => {
                write!(
                    f,
                    "more than one project file declares the executable name {}: ",
                    exe.display()
                )?;
                for (i, candidate) in candidates.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", candidate.display())?;
                }
                Ok(())
            }
        }
    }
}

/// Selects the one project file, among `projects`, that declares `exe`
/// as its `ExeName32`.
///
/// The search is scoped to `exe`'s own corpus entry directory before
/// `ExeName32` is applied. See the module doc comment for why: a
/// corpus-wide index keyed on `ExeName32` alone collides on this corpus.
///
/// # Errors
///
/// Returns [`SelectionError::NoCandidate`] when no project file in the
/// entry directory matches, and [`SelectionError::MultipleCandidates`]
/// when more than one does. Both name the executable; the second also
/// names every candidate.
pub fn select_project_file(exe: &Path, projects: &[PathBuf]) -> Result<PathBuf, SelectionError> {
    let dir = entry_dir(exe).unwrap_or_else(|| exe.to_owned());
    let exe_name = exe.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let matches: Vec<PathBuf> = projects
        .iter()
        .filter(|p| p.starts_with(&dir))
        .filter(|p| {
            Project::read(p)
                .get("ExeName32")
                .is_some_and(|v| v.eq_ignore_ascii_case(exe_name))
        })
        .cloned()
        .collect();

    match matches.len() {
        1 => Ok(matches[0].clone()),
        0 => Err(SelectionError::NoCandidate {
            exe: exe.to_owned(),
            entry_dir: dir,
        }),
        _ => Err(SelectionError::MultipleCandidates {
            exe: exe.to_owned(),
            candidates: matches,
        }),
    }
}

/// Builds an index of every project file's `ExeName32` value, across the
/// whole corpus, with no entry-directory scope.
///
/// This exists only so a test can prove why [`select_project_file`] must
/// not build one: on this corpus, it collides.
/// `SK-MCI-Sample__VB6/MCI.VBP` and `SK-Gradient-Sample__VB6/Project1.vbp`
/// both declare `ExeName32="Project1.exe"`.
#[must_use]
pub fn corpus_wide_exe_name_index(projects: &[PathBuf]) -> HashMap<String, Vec<PathBuf>> {
    let mut index: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for path in projects {
        if let Some(exe_name) = Project::read(path).get("ExeName32") {
            index
                .entry(exe_name.to_ascii_lowercase())
                .or_default()
                .push(path.clone());
        }
    }
    index
}

/// The eight keys a `.vbp` file uses to declare an object, matched
/// exactly against the text before the first `=` sign. Per D-03 the
/// declared object list comes from these keys and from nothing else:
/// never a directory glob, which would count an orphan source file
/// (one the project file never lists) as a recovery failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Form,
    Module,
    Class,
    UserControl,
    PropertyPage,
    UserDocument,
    Designer,
    RelatedDoc,
}

impl ObjectKind {
    /// Matches a `.vbp` key exactly against the eight object kinds.
    /// Gives `None` for every other key, including `ExeName32`,
    /// `Title` and `Reference`.
    fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "Form" => Self::Form,
            "Module" => Self::Module,
            "Class" => Self::Class,
            "UserControl" => Self::UserControl,
            "PropertyPage" => Self::PropertyPage,
            "UserDocument" => Self::UserDocument,
            "Designer" => Self::Designer,
            "RelatedDoc" => Self::RelatedDoc,
            _ => return None,
        })
    }
}

/// Where a declared object's name came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSource {
    /// `Attribute VB_Name` inside the referenced source file, found by
    /// scanning the whole file. The attribute sits after the whole
    /// `Begin ... End` block for a form, which can run to thousands of
    /// lines when the form embeds picture data, so a bounded scan
    /// misses it.
    Attribute,
    /// The prefix before the semicolon on the project line. Used only
    /// when the repository does not hold the referenced source file, so
    /// the attribute is unreachable. A form line carries no prefix, so
    /// this can never name a form.
    Fallback,
}

/// One object a project file declares.
#[derive(Debug, Clone)]
pub struct DeclaredObject {
    pub kind: ObjectKind,
    /// The compiled object name. `None` when neither the attribute nor
    /// the fallback prefix could name it: a form whose source file is
    /// missing has no recoverable name and must be reported as a gap,
    /// never guessed.
    pub name: Option<String>,
    pub name_source: Option<NameSource>,
    /// The raw prefix before the semicolon on the project line, kept
    /// separately from `name` so a test can cross-check the two. A
    /// `Form=` line has none: only `Module=`, `Class=`, `UserControl=`,
    /// `PropertyPage=` and `UserDocument=` lines carry a prefix.
    pub prefix: Option<String>,
    /// The source file the project line references, resolved relative
    /// to the project file's own directory. May not exist on disk.
    pub source_file: PathBuf,
}

/// Scans a source file's whole text for `Attribute VB_Name = "..."` and
/// gives the quoted name.
///
/// The whole file is scanned, not a bounded prefix. A 40-line prefix
/// scan matches none of the forms in this corpus whose form definition
/// block runs long, because the attribute is written after the whole
/// `Begin VB.Form ... End` block. Gives `None` when the file cannot be
/// read (missing from the repository) or carries no such line.
fn find_vb_name(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let text: String = bytes.iter().copied().map(char::from).collect();
    for line in text.lines() {
        let s = line.trim();
        let Some(rest) = s.strip_prefix("Attribute VB_Name") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        if let Some(name) = quoted_value(rest) {
            return Some(name);
        }
    }
    None
}

impl Project {
    /// Gives every object this project file declares, per D-03: read
    /// from the eight object keys and from nothing else, never from a
    /// directory listing.
    ///
    /// `Hidden-Markov-model` and `Randomize-effects` each carry a
    /// `cCommonDialog.cls` source file on disk that neither project
    /// file lists; walking this list rather than the directory is what
    /// keeps both out, because the compiler never built either one in.
    #[must_use]
    pub fn declared_objects(&self) -> Vec<DeclaredObject> {
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        let mut out = Vec::new();
        for line in self.text.lines() {
            let s = line.trim();
            let Some((k, rest)) = key_value(s) else {
                continue;
            };
            let Some(kind) = ObjectKind::from_key(k) else {
                continue;
            };
            // Unlike a string property, an object line's value is never
            // quoted. `Form=File.frm` carries only a file name;
            // `Class=Name; File.cls` carries a prefix, a semicolon, and
            // a file name. A key with no semicolon has no prefix.
            let (prefix, file) = match rest.split_once(';') {
                Some((p, f)) => (Some(p.trim().to_owned()), f.trim().to_owned()),
                None => (None, rest.trim().to_owned()),
            };
            let source_file = dir.join(&file);

            let (name, name_source) = match find_vb_name(&source_file) {
                Some(attribute_name) => (Some(attribute_name), Some(NameSource::Attribute)),
                None => match prefix.clone() {
                    Some(p) if !source_file.exists() => (Some(p), Some(NameSource::Fallback)),
                    _ => (None, None),
                },
            };

            out.push(DeclaredObject {
                kind,
                name,
                name_source,
                prefix,
                source_file,
            });
        }
        out
    }
}
