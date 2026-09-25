#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]

//! The structural recompilation check.
//!
//! Full recompilation cannot run here. It needs the Visual Basic 6 IDE on
//! Windows, and this sandbox has neither. This file is the check that runs
//! in its place: it drives the real write path over all 44 corpus programs,
//! then reads every file it wrote back through the second, independent
//! reader in `tests/support/`, the same reader `differential.rs` already
//! uses to read the original corpus. Nothing here claims that a run of this
//! file starts the VB6 IDE, opens a project, or compiles one; it proves only
//! that the files this phase writes are shaped the way the roadmap says
//! they must be.
//!
//! **This file names nothing from `deform6::write::frm`,
//! `deform6::write::vbp`, `deform6::write::code`, `deform6::write::values`
//! or `deform6::write::comment`.** Its one reach into the writing side of
//! the library is [`deform6::write::project`], the entry point that
//! produces the files. Every fact this file checks about the files that
//! entry point returns comes from `tests/support/frm.rs` and
//! `tests/support/vbp.rs`: a checker that shared a parser with the writer it
//! tests would agree with a bug in that writer, and the failure would be
//! invisible.

#[path = "support/mod.rs"]
#[allow(
    dead_code,
    reason = "cargo compiles this shared module into every test binary and this one uses only \
              the frm and vbp readers"
)]
mod support;

use std::path::{Path, PathBuf};

use deform6::vb::classify::ObjectKind;
use deform6::vb::opcodes::OpcodeTable;
use support::frm::{self, Block};
use support::vbp;

/// The longest control or class name the roadmap allows: 40 characters.
/// Stated here as a plain number, not imported from
/// `deform6::write::model::MAX_NAME_LEN`: this check proves the roadmap's
/// own number, not the writer's own constant, so a change to one without
/// the other is a loud failure rather than an agreement between two copies
/// of the same value.
const MAX_LEGAL_NAME_LEN: usize = 40;

/// The deepest a control tree may nest, per the roadmap: 7.
const MAX_LEGAL_NESTING_DEPTH: usize = 7;

/// The exact sentence `crate::report::build_limits` states as this run's
/// first limit line. Held here as this check's own independent copy, so an
/// assertion that the two agree can never silently become an assertion that
/// this file agrees with itself: the two literals are written in two
/// different sessions of reading the same roadmap requirement, and a future
/// edit to either one that drifts from the other fails this test, naming
/// both.
const RECOMPILATION_LIMIT_STATEMENT: &str = "Full recompilation did not run. It needs the \
                                              Visual Basic 6 IDE on Windows, and this run had \
                                              neither. A structural check ran in its place, and \
                                              it never opened this project in the IDE.";

// --- Task 1: the corpus walk and the write path, driven once per program --

/// Gives the directory that holds the `corpus/` this workspace vendors.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Walks `corpus/` recursively and gives every file whose extension is
/// `.exe`, compared case-insensitively, sorted.
fn executables() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(&corpus_root(), &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push(path);
        }
    }
}

/// A fresh, empty temporary directory this run owns, named after `label` so
/// two programs in the same run never collide.
fn fresh_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "deform6-extract-structural-{}-{label}",
        std::process::id()
    ));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("creating a fresh temp directory must succeed");
    dir
}

/// One corpus program, read, inspected, written into its own temporary
/// directory, and read back through nothing but the independent readers.
struct ExtractedProgram {
    /// The executable's own path, relative to `corpus/`, for every failure
    /// message this file prints.
    key: String,
    /// The directory this program's own files were written into.
    dir: PathBuf,
    /// Every file [`deform6::write::project`] returned for this program.
    files: Vec<deform6::write::WrittenFile>,
    /// The read side's own recovered counts, kept only for the independent
    /// text file count this file cross-checks the write side against: a
    /// fact the write side never computed, from a call that ran before the
    /// write side ever saw the data.
    forms_declared: usize,
    code_objects_declared: usize,
    /// The JSON confidence report the same write path run produced.
    limits: Vec<String>,
}

/// Runs the whole write path once over `exe`, then writes every file it
/// returned to a fresh temporary directory. Panics loudly on any refusal:
/// the corpus is vendored and fixed, and roadmap success criterion 1 states
/// plainly that every one of the 44 programs writes and exits 0.
fn extract_one(exe: &Path, root: &Path, label: &str) -> ExtractedProgram {
    let key = exe.strip_prefix(root).unwrap_or(exe).display().to_string();
    let data =
        std::fs::read(exe).unwrap_or_else(|err| panic!("{key}: reading the executable: {err}"));
    let table = OpcodeTable::builtin();
    let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict)
        .unwrap_or_else(|err| panic!("{key}: inspect refused this program: {err}"));

    let forms_declared = report.forms.len();
    let code_objects_declared = report
        .objects
        .iter()
        .filter(|object| object.kind != ObjectKind::Form)
        .count();

    let written = deform6::write::project(&report, &data, deform6::journal::Mode::Strict)
        .unwrap_or_else(|err| panic!("{key}: write::project refused this program: {err}"));

    let dir = fresh_dir(label);
    for file in &written.files {
        std::fs::write(dir.join(&file.name), &file.bytes)
            .unwrap_or_else(|err| panic!("{key}: writing {}: {err}", file.name));
    }

    ExtractedProgram {
        key,
        dir,
        files: written.files,
        forms_declared,
        code_objects_declared,
        limits: written.report.limits,
    }
}

fn is_resource_file(name: &str) -> bool {
    name.ends_with(".frx")
}

fn is_report_file(name: &str) -> bool {
    name.ends_with(".report.json")
}

fn is_text_file(name: &str) -> bool {
    !is_resource_file(name) && !is_report_file(name)
}

// --- Named assertion 1: every component line names a file that exists -----

/// Checks every `Form=`, `Module=` and `Class=` line one written `.vbp`
/// declares against the files that actually exist beside it, through
/// `support::vbp::Project::declared_objects`, the independent reader
/// `differential.rs` already trusts for the same fact over the original
/// corpus.
fn check_components_name_existing_files(
    program_key: &str,
    declared: &[vbp::DeclaredObject],
) -> Vec<String> {
    let mut failures = Vec::new();
    for object in declared {
        if !object.source_file.exists() {
            failures.push(format!(
                "{program_key}: a {:?} component line names {}, which does not exist",
                object.kind,
                object.source_file.display()
            ));
        }
    }
    failures
}

// --- Named assertion 2: Startup= names a form a Form= line brings in ------

/// Checks the written `.vbp`'s own `Startup=` value against the forms its
/// own `Form=` lines declare. `"Sub Main"` is the one value this project's
/// own writer gives when it declares no form at all, per `write::vbp`'s own
/// rule; every other value must equal one declared form's own recovered
/// name.
fn check_startup_names_a_declared_form(
    program_key: &str,
    startup: Option<&str>,
    declared: &[vbp::DeclaredObject],
) -> Vec<String> {
    let Some(startup) = startup else {
        return vec![format!(
            "{program_key}: the written .vbp names no Startup= value at all"
        )];
    };
    if startup == "Sub Main" {
        return Vec::new();
    }
    let names_a_form = declared.iter().any(|object| {
        object.kind == vbp::ObjectKind::Form && object.name.as_deref() == Some(startup)
    });
    if names_a_form {
        Vec::new()
    } else {
        vec![format!(
            "{program_key}: Startup=\"{startup}\" names no form a Form= line brings in"
        )]
    }
}

// --- Named assertion 3: every control and class name is a legal identifier

/// Checks one recovered name: a legal VB6 identifier starts with an ASCII
/// letter, holds only letters, digits and underscores after that, and is
/// [`MAX_LEGAL_NAME_LEN`] characters or fewer. Gives the failure message
/// naming `context` and `name` when the check fails, or `None` when it
/// passes.
fn check_one_identifier(context: &str, name: &str) -> Option<String> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Some(format!("{context}: the name is empty"));
    };
    if !first.is_ascii_alphabetic() {
        return Some(format!(
            "{context}: {name:?} does not start with an ASCII letter"
        ));
    }
    if !chars.all(|ch| ch.is_alphanumeric() || ch == '_') {
        return Some(format!(
            "{context}: {name:?} holds a character that is not a letter, a digit or an underscore"
        ));
    }
    let len = name.chars().count();
    if len > MAX_LEGAL_NAME_LEN {
        return Some(format!(
            "{context}: {name:?} is {len} characters long, over the {MAX_LEGAL_NAME_LEN} \
             character limit"
        ));
    }
    None
}

/// Walks a written form's own parsed control tree (root and every
/// descendant) and checks every control's own name as a legal identifier.
fn check_control_identifiers(file_label: &str, block: &Block, failures: &mut Vec<String>) {
    let context = format!("{file_label}: control {:?}", block.name);
    if let Some(message) = check_one_identifier(&context, &block.name) {
        failures.push(message);
    }
    for child in &block.children {
        check_control_identifiers(file_label, child, failures);
    }
}

// --- Named assertion 4: nesting depth is 7 or less -------------------------

/// Walks a written form's own parsed control tree and checks that no
/// control sits deeper than [`MAX_LEGAL_NESTING_DEPTH`].
fn check_nesting_depth(file_label: &str, block: &Block, depth: usize, failures: &mut Vec<String>) {
    if depth > MAX_LEGAL_NESTING_DEPTH {
        failures.push(format!(
            "{file_label}: control {:?} sits at nesting depth {depth}, over the \
             {MAX_LEGAL_NESTING_DEPTH} level limit",
            block.name
        ));
    }
    for child in &block.children {
        check_nesting_depth(file_label, child, depth.saturating_add(1), failures);
    }
}

// --- Named assertion 5: properties inside a block are alphabetical --------

/// `true` for a block this repository writes with its own stream order
/// preserved rather than sorted: an external (OCX) control, whose class is
/// outside the `VB.` library of the intrinsic controls, such as
/// `MSWinsockLib.Winsock`. This mirrors `write::frm::write_model_control_block`'s
/// own `is_external` skip, read here as a written fact rather than shared
/// as code: the independent reader has no `is_external` field of its own,
/// only the class name the file itself carries.
fn skips_alphabetical_order(block: &Block) -> bool {
    !block.class.starts_with("VB.")
}

/// Walks a written form's own parsed control tree and checks that every
/// block's own direct properties (never a nested `BeginProperty` block,
/// whose own key order is fixed by the format, not by this rule) are in
/// case-insensitive ascending order by name.
fn check_property_order(file_label: &str, block: &Block, failures: &mut Vec<String>) {
    if !skips_alphabetical_order(block) {
        for pair in block.properties.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            if a.name.to_lowercase() > b.name.to_lowercase() {
                failures.push(format!(
                    "{file_label}: block {:?} writes {:?} before {:?}, out of case-insensitive \
                     alphabetical order",
                    block.name, a.name, b.name
                ));
            }
        }
    }
    for child in &block.children {
        check_property_order(file_label, child, failures);
    }
}

// --- Named assertion 6: every menu comes after every other control --------

/// Walks a written form's own parsed control tree and checks that, inside
/// every block's own child list, once a `VB.Menu` child appears, every
/// child after it is also a `VB.Menu`.
fn check_menus_last(file_label: &str, block: &Block, failures: &mut Vec<String>) {
    let mut seen_menu = false;
    for child in &block.children {
        let is_menu = child.class == "VB.Menu";
        if is_menu {
            seen_menu = true;
        } else if seen_menu {
            failures.push(format!(
                "{file_label}: block {:?} writes non-menu control {:?} after a menu control",
                block.name, child.name
            ));
        }
    }
    for child in &block.children {
        check_menus_last(file_label, child, failures);
    }
}

// --- Resource offset resolution: every .frx offset a .frm names lands on --
// --- a valid record header inside the .frx that was actually written ------

/// Parses every `"name.frx":OFFSET` (or `$"name.frx":OFFSET`) hex offset a
/// written `.frm`'s own text declares for `frx_file_name`, in the order the
/// lines give them.
fn frx_offsets_named_in(frm_text: &str, frx_file_name: &str) -> Vec<u32> {
    let marker = format!("\"{frx_file_name}\":");
    let mut offsets = Vec::new();
    for line in frm_text.lines() {
        let Some(position) = line.find(marker.as_str()) else {
            continue;
        };
        let after = &line[position.saturating_add(marker.len())..];
        let hex: String = after.chars().take_while(char::is_ascii_hexdigit).collect();
        if let Ok(offset) = u32::from_str_radix(&hex, 16) {
            offsets.push(offset);
        }
    }
    offsets
}

/// Seeks every offset [`frx_offsets_named_in`] found in `frx_bytes`, reads
/// the four byte length there, and checks the record it describes ends
/// inside the file. Separately checks that the last (highest-offset)
/// record ends exactly at the file's own end, the property the corpus
/// proves for every committed resource file.
fn check_resource_offsets(
    file_label: &str,
    frx_file_name: &str,
    frm_text: &str,
    frx_bytes: &[u8],
) -> Vec<String> {
    let mut failures = Vec::new();
    let mut last_end: Option<usize> = None;

    for offset in frx_offsets_named_in(frm_text, frx_file_name) {
        let at = usize::try_from(offset).unwrap_or(usize::MAX);
        let Some(length_bytes) = frx_bytes.get(at..at.saturating_add(4)) else {
            failures.push(format!(
                "{file_label}: offset {offset:#06X} into {frx_file_name} has no four byte \
                 length header inside the file"
            ));
            continue;
        };
        let declared_len = u32::from_le_bytes(
            length_bytes
                .try_into()
                .unwrap_or_else(|_| panic!("exactly four bytes were sliced")),
        );
        let declared_len_usize = usize::try_from(declared_len).unwrap_or(usize::MAX);
        let Some(end) = at
            .checked_add(4)
            .and_then(|v| v.checked_add(declared_len_usize))
        else {
            failures.push(format!(
                "{file_label}: the record at offset {offset:#06X} in {frx_file_name} overflows \
                 while computing its own end"
            ));
            continue;
        };
        if end > frx_bytes.len() {
            failures.push(format!(
                "{file_label}: the record at offset {offset:#06X} in {frx_file_name} declares \
                 length {declared_len}, ending at byte {end}, past the file's own {} bytes",
                frx_bytes.len()
            ));
            continue;
        }
        last_end = Some(last_end.map_or(end, |current| current.max(end)));
    }

    if let Some(end) = last_end
        && end != frx_bytes.len()
    {
        failures.push(format!(
            "{file_label}: the last record in {frx_file_name} ends at byte {end}, not at the \
             file's own end, byte {}",
            frx_bytes.len()
        ));
    }

    failures
}

// --- Task 2: the whole tree encoding sweep ---------------------------------

/// Checks one written text file: no byte order mark, every line feed byte
/// paired into a CRLF (so no bare line feed exists anywhere), the file's
/// own last two bytes are that pair, and no byte in the file, if it happens
/// to validate as UTF-8 at all, decodes to anything above plain ASCII. A
/// Windows-1252 encoder writes one byte per character; a genuine non-ASCII
/// character it wrote would almost never also validate as UTF-8 (a lone
/// byte such as `0xA9` is not a legal UTF-8 lead byte on its own), so a file
/// that does validate as UTF-8 and holds a non-ASCII character is the loud
/// sign that the encoder was bypassed somewhere and raw UTF-8 bytes leaked
/// into a Windows-1252 file.
fn check_text_file_encoding(name: &str, bytes: &[u8]) -> Vec<String> {
    let mut failures = Vec::new();

    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        failures.push(format!("{name}: begins with a byte order mark"));
    }

    let line_feed_count = bytes.iter().filter(|&&byte| byte == b'\n').count();
    let crlf_pair_count = bytes.windows(2).filter(|pair| *pair == b"\r\n").count();
    if line_feed_count != crlf_pair_count {
        failures.push(format!(
            "{name}: holds {line_feed_count} line feed byte(s) but only {crlf_pair_count} CRLF \
             pair(s); a bare line feed exists"
        ));
    }

    if !bytes.is_empty() && !bytes.ends_with(b"\r\n") {
        failures.push(format!(
            "{name}: the file's own last two bytes are not the CRLF pair"
        ));
    }

    if let Ok(text) = std::str::from_utf8(bytes)
        && !text.is_ascii()
    {
        failures.push(format!(
            "{name}: the bytes validate as UTF-8 and hold a character above plain ASCII; a \
             Windows-1252 encoder writes one byte per character and never a UTF-8 multi-byte \
             sequence"
        ));
    }

    failures
}

// --- The main corpus-wide run ----------------------------------------------

#[test]
fn the_structural_check_passes_for_all_forty_four_corpus_programs() {
    let programs = executables();
    assert_eq!(
        programs.len(),
        44,
        "found {} corpus executables, wanted 44",
        programs.len()
    );

    let root = corpus_root();
    let mut failures: Vec<String> = Vec::new();
    let mut programs_checked = 0_usize;
    let mut text_files_checked = 0_usize;
    let mut extracted_dirs: Vec<PathBuf> = Vec::new();

    for (index, exe) in programs.iter().enumerate() {
        let extracted = extract_one(exe, &root, &index.to_string());
        extracted_dirs.push(extracted.dir.clone());
        programs_checked = programs_checked.saturating_add(1);
        let key = extracted.key.as_str();

        // Every written file, classified: vbp, frm/frx pairs, bas/cls.
        let vbp_file = extracted
            .files
            .iter()
            .find(|file| file.name.ends_with(".vbp"));
        let Some(vbp_file) = vbp_file else {
            failures.push(format!("{key}: no .vbp file was written at all"));
            continue;
        };

        let vbp_path = extracted.dir.join(&vbp_file.name);
        let project = vbp::Project::read(&vbp_path);
        let declared = project.declared_objects();

        failures.extend(check_components_name_existing_files(key, &declared));
        failures.extend(check_startup_names_a_declared_form(
            key,
            project.get("Startup").as_deref(),
            &declared,
        ));

        // Cross-check the text file count against the read side's own
        // independent counts: the vbp plus one file per declared form plus
        // one file per non-form object, computed from `Report` before the
        // write side ever ran.
        let expected_text_files = 1_usize
            .saturating_add(extracted.forms_declared)
            .saturating_add(extracted.code_objects_declared);
        let actual_text_files = extracted
            .files
            .iter()
            .filter(|file| is_text_file(&file.name))
            .count();
        if actual_text_files != expected_text_files {
            failures.push(format!(
                "{key}: wrote {actual_text_files} text file(s), the read side's own object \
                 graph names {expected_text_files}"
            ));
        }

        // Every recovered class or module name: a legal identifier.
        for file in extracted
            .files
            .iter()
            .filter(|file| file.name.ends_with(".bas") || file.name.ends_with(".cls"))
        {
            let path = extracted.dir.join(&file.name);
            let text = read_latin1(&path);
            match attribute_vb_name(&text) {
                Some(name) => {
                    let context = format!("{key}: {}: object", file.name);
                    if let Some(message) = check_one_identifier(&context, &name) {
                        failures.push(message);
                    }
                }
                None => failures.push(format!(
                    "{key}: {} carries no Attribute VB_Name line",
                    file.name
                )),
            }
        }

        // Every written form: its own control tree and, when it names one,
        // its own resource file.
        for file in extracted
            .files
            .iter()
            .filter(|file| file.name.ends_with(".frm"))
        {
            let frm_path = extracted.dir.join(&file.name);
            let form = frm::Form::read(&frm_path);
            let file_label = format!("{key}: {}", file.name);

            let roots = form.blocks();
            for block in &roots {
                check_control_identifiers(&file_label, block, &mut failures);
                check_nesting_depth(&file_label, block, 0, &mut failures);
                check_property_order(&file_label, block, &mut failures);
                check_menus_last(&file_label, block, &mut failures);
            }

            let frx_file_name = file.name.replace(".frm", ".frx");
            if let Some(frx_file) = extracted
                .files
                .iter()
                .find(|candidate| candidate.name == frx_file_name)
            {
                let frx_path = extracted.dir.join(&frx_file.name);
                let frx_bytes = std::fs::read(&frx_path)
                    .unwrap_or_else(|err| panic!("{key}: reading {frx_file_name}: {err}"));
                failures.extend(check_resource_offsets(
                    &file_label,
                    &frx_file_name,
                    &form.text,
                    &frx_bytes,
                ));
            }
        }

        // Every written text file: the encoding sweep. Excludes .frx (a
        // resource file, binary by kind, never by a guess from content) and
        // .report.json (this phase's own JSON output, UTF-8 by design, not
        // a Visual Basic project file).
        for file in extracted
            .files
            .iter()
            .filter(|file| is_text_file(&file.name))
        {
            failures.extend(check_text_file_encoding(&file.name, &file.bytes));
            text_files_checked = text_files_checked.saturating_add(1);
        }

        if !extracted
            .limits
            .iter()
            .any(|line| line.contains("did not run"))
        {
            failures.push(format!(
                "{key}: the report's own limits list states nowhere that full recompilation did \
                 not run"
            ));
        }
    }

    assert_eq!(
        programs_checked,
        programs.len(),
        "this run checked {programs_checked} program(s), but {} executables exist under the \
         corpus root",
        programs.len()
    );
    assert!(
        text_files_checked > 0,
        "the sweep read zero text files across every program; it passed nothing"
    );
    assert!(
        failures.is_empty(),
        "{} structural failure(s) across {} program(s):\n{}",
        failures.len(),
        programs_checked,
        failures.join("\n")
    );

    for dir in &extracted_dirs {
        std::fs::remove_dir_all(dir).ok();
    }
}

// --- Each Object= line is one that the original project declares --------

/// Each `Object=` line that the write path gives for a corpus program is a
/// line that the project file of that program declares. The corpus gives
/// three such lines, one for each program that holds a Winsock control.
///
/// The count holds the check to the corpus. Without it, a writer that gave
/// no `Object=` line at all would pass.
#[test]
fn each_object_line_is_one_that_the_original_project_declares() {
    let root = corpus_root();
    let projects = vbp::project_files();
    let mut failures: Vec<String> = Vec::new();
    let mut written_lines = 0_usize;

    for (index, exe) in executables().iter().enumerate() {
        let extracted = extract_one(exe, &root, &format!("object-{index}"));
        let key = extracted.key.as_str();
        let written_vbp = extracted
            .files
            .iter()
            .find(|file| file.name.ends_with(".vbp"))
            .unwrap_or_else(|| panic!("{key}: no .vbp file was written"));
        let written = vbp::Project::read(&extracted.dir.join(&written_vbp.name)).values("Object");
        let original =
            vbp::select_project_file(exe, &projects).unwrap_or_else(|err| panic!("{key}: {err}"));
        let declared = vbp::Project::read(&original).values("Object");

        for line in &written {
            written_lines = written_lines.saturating_add(1);
            if !declared.contains(line) {
                failures.push(format!(
                    "{key}: writes Object={line}, and {} declares {declared:?}",
                    original.display()
                ));
            }
        }
        std::fs::remove_dir_all(&extracted.dir).ok();
    }

    assert!(
        failures.is_empty(),
        "{} Object= line(s) that the original project does not declare:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        written_lines, 3,
        "the corpus programs gave {written_lines} Object= line(s), and three programs hold a \
         Winsock control"
    );
}

// --- Each control class is the class that the original form declares ----

/// Adds the class of `block`, and of each block inside it, to `classes`,
/// by the control name in lower case. The elements of a control array share
/// one name and one class.
fn collect_classes(block: &Block, classes: &mut std::collections::BTreeMap<String, String>) {
    classes.insert(block.name.to_lowercase(), block.class.clone());
    for child in &block.children {
        collect_classes(child, classes);
    }
}

/// Each `Begin` line that the write path gives for a corpus form names the
/// class that the original form declares for the control of the same name.
/// The corpus holds three external controls, one in each program that holds
/// a Winsock control.
///
/// The count holds the check to the corpus. Without it, a writer that gave
/// no block for an external control would pass.
#[test]
fn each_control_class_is_the_class_that_the_original_form_declares() {
    let root = corpus_root();
    let projects = vbp::project_files();
    let mut failures: Vec<String> = Vec::new();
    let mut names_compared = 0_usize;
    let mut external_names = 0_usize;

    for (index, exe) in executables().iter().enumerate() {
        let extracted = extract_one(exe, &root, &format!("class-{index}"));
        let key = extracted.key.as_str();
        let original =
            vbp::select_project_file(exe, &projects).unwrap_or_else(|err| panic!("{key}: {err}"));
        let declared = vbp::Project::read(&original).declared_objects();

        for file in extracted
            .files
            .iter()
            .filter(|file| file.name.ends_with(".frm"))
        {
            let form = frm::Form::read(&extracted.dir.join(&file.name));
            let Some(name) = attribute_vb_name(&form.text) else {
                failures.push(format!(
                    "{key}: {} holds no Attribute VB_Name line",
                    file.name
                ));
                continue;
            };
            let Some(source) = declared.iter().find(|object| {
                object.kind == vbp::ObjectKind::Form
                    && object
                        .name
                        .as_deref()
                        .is_some_and(|declared_name| declared_name.eq_ignore_ascii_case(&name))
            }) else {
                failures.push(format!(
                    "{key}: {} declares no form named {name}",
                    original.display()
                ));
                continue;
            };

            let mut original_classes = std::collections::BTreeMap::new();
            for block in &frm::Form::read(&source.source_file).blocks() {
                collect_classes(block, &mut original_classes);
            }
            let mut written_classes = std::collections::BTreeMap::new();
            for block in &form.blocks() {
                collect_classes(block, &mut written_classes);
            }
            for (control, class) in &written_classes {
                names_compared = names_compared.saturating_add(1);
                if !class.starts_with("VB.") {
                    external_names = external_names.saturating_add(1);
                }
                if original_classes.get(control) != Some(class) {
                    failures.push(format!(
                        "{key}: {}: control {control} is written as {class}, and the original \
                         form declares {:?}",
                        file.name,
                        original_classes.get(control)
                    ));
                }
            }
        }
        std::fs::remove_dir_all(&extracted.dir).ok();
    }

    assert!(
        failures.is_empty(),
        "{} of {names_compared} control class(es) that the original form does not declare:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        external_names, 3,
        "the corpus forms gave {external_names} external control(s), and three programs hold a \
         Winsock control"
    );
}

/// Reads `path` as Latin-1 bytes, this crate's own read and write
/// convention: each byte maps to its own code point, never
/// `String::from_utf8_lossy`. Copied here, not shared with any writing
/// module: this is a plain text read, the same one `tests/support/frm.rs`
/// already performs on its own.
fn read_latin1(path: &Path) -> String {
    let bytes =
        std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    bytes.iter().copied().map(char::from).collect()
}

/// Scans `text` line by line for `Attribute VB_Name = "..."` and gives the
/// quoted name. Mirrors `tests/support/vbp.rs`'s own `find_vb_name`,
/// deliberately duplicated rather than imported: `find_vb_name` is private
/// to its own file, and a second, independent scan of the same line shape
/// is exactly the discipline this whole file exists to apply.
fn attribute_vb_name(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("Attribute VB_Name") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim();
        let first = rest.find('"')?;
        let last = rest.rfind('"')?;
        if last > first {
            return Some(rest[first.saturating_add(1)..last].to_owned());
        }
    }
    None
}

// --- A dedicated test for the exact recompilation statement, run over one -
// --- program rather than all 44: the sentence itself is not per-program ---

#[test]
fn the_reports_limits_state_the_exact_recompilation_sentence() {
    let exe = corpus_root().join("vb6-code/Fire-effect/Fast_Flames.exe");
    let data = std::fs::read(&exe).expect("reading Fast_Flames.exe");
    let table = OpcodeTable::builtin();
    let report = deform6::inspect(&data, &table, deform6::journal::Mode::Strict)
        .expect("inspect must succeed");
    let written = deform6::write::project(&report, &data, deform6::journal::Mode::Strict)
        .expect("write::project must succeed");

    // Plan 05-01 task 3 adds a mode line before the four lines every run
    // already stated, so the recompilation sentence is no longer the first
    // entry; it must still appear, verbatim, somewhere in the list.
    assert!(
        written
            .report
            .limits
            .iter()
            .any(|line| line == RECOMPILATION_LIMIT_STATEMENT),
        "the report's own limits must hold this check's own copy of the recompilation \
         sentence, verbatim: {RECOMPILATION_LIMIT_STATEMENT:?}, limits were: \
         {:?}",
        written.report.limits
    );
}

// --- Deliberate breakages, one per named assertion, built by hand rather --
// --- than by mutating a real corpus program, per this task's own rule ----
// --- that a test that cannot fail is worse than no test at all -----------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod deliberate_breakages {
    // `super::support`, never a second `#[path]` declaration here: a
    // `#[path]` attribute on a module nested inside this inline `mod`
    // would resolve against this module's own implied, nonexistent
    // directory, never this file's own real directory (the exact pitfall
    // plan 04-04's own `write::frm` doc comment names). `support` is
    // declared once, at this file's own top level, and reached from here
    // through `super`.
    use super::support::frm::{Block, Property};
    use super::{
        check_control_identifiers, check_menus_last, check_nesting_depth, check_property_order,
        check_resource_offsets,
    };

    fn leaf(class: &str, name: &str) -> Block {
        Block {
            class: class.to_owned(),
            name: name.to_owned(),
            properties: Vec::new(),
            property_blocks: Vec::new(),
            children: Vec::new(),
        }
    }

    /// One of this task's own four named deliberate breakages: a `.vbp`
    /// that declares a component line naming a file this repository never
    /// wrote. `super::vbp::DeclaredObject` is built by hand, per
    /// `AGENTS.md`'s "build the state a test needs inside the test": no
    /// corpus program leaves a component line dangling, so this shape is
    /// only ever exercised here.
    #[test]
    fn a_component_line_naming_a_missing_file_fails_the_component_check() {
        let missing = super::vbp::DeclaredObject {
            kind: super::vbp::ObjectKind::Form,
            name: Some("frmGhost".to_owned()),
            name_source: None,
            prefix: None,
            source_file: std::path::PathBuf::from(
                "/deform6-fixture-that-never-exists/frmGhost.frm",
            ),
        };
        let failures = super::check_components_name_existing_files("fixture.exe", &[missing]);
        assert!(
            !failures.is_empty(),
            "a component line naming a file that does not exist must fail the check"
        );
        assert!(failures[0].contains("frmGhost"), "{failures:?}");
    }

    #[test]
    fn a_control_name_holding_an_illegal_character_fails_the_identifier_check() {
        let mut root = leaf("VB.Form", "frmOk");
        root.children.push(leaf("VB.CommandButton", "cmd/Bad"));
        let mut failures = Vec::new();
        check_control_identifiers("fixture.frm", &root, &mut failures);
        assert!(
            !failures.is_empty(),
            "a control name holding an illegal character must fail the check"
        );
        assert!(failures[0].contains("cmd/Bad"), "{failures:?}");
    }

    #[test]
    fn a_control_nested_past_the_depth_limit_fails_the_nesting_check() {
        let mut block = leaf("VB.Form", "frmDeep");
        {
            let mut current = &mut block;
            for level in 1..=8 {
                current
                    .children
                    .push(leaf("VB.Frame", &format!("Frame{level}")));
                current = current.children.last_mut().expect("just pushed");
            }
        }
        let mut failures = Vec::new();
        check_nesting_depth("fixture.frm", &block, 0, &mut failures);
        assert!(
            !failures.is_empty(),
            "a control past the depth limit must fail the check"
        );
        assert!(failures[0].contains("Frame8"), "{failures:?}");
    }

    #[test]
    fn a_reversed_property_order_fails_the_alphabetical_check() {
        let mut block = leaf("VB.Form", "frmOrder");
        block.properties = vec![
            Property {
                name: "Visible".to_owned(),
                value: "0".to_owned(),
            },
            Property {
                name: "Caption".to_owned(),
                value: "\"Hi\"".to_owned(),
            },
        ];
        let mut failures = Vec::new();
        check_property_order("fixture.frm", &block, &mut failures);
        assert!(
            !failures.is_empty(),
            "reversed property order must fail the check"
        );
        assert!(
            failures[0].contains("Visible") && failures[0].contains("Caption"),
            "{failures:?}"
        );
    }

    /// An external control keeps the order of its own stream, so the same
    /// reversed order passes the check for a class outside `VB.`.
    #[test]
    fn a_reversed_property_order_passes_for_an_external_control() {
        let mut block = leaf("MSWinsockLib.Winsock", "wsPop");
        block.properties = vec![
            Property {
                name: "Visible".to_owned(),
                value: "0".to_owned(),
            },
            Property {
                name: "Caption".to_owned(),
                value: "\"Hi\"".to_owned(),
            },
        ];
        let mut failures = Vec::new();
        check_property_order("fixture.frm", &block, &mut failures);
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn a_menu_before_a_non_menu_sibling_fails_the_menus_last_check() {
        let mut block = leaf("VB.Form", "frmMenu");
        block.children = vec![leaf("VB.Menu", "mnuFile"), leaf("VB.CommandButton", "cmd1")];
        let mut failures = Vec::new();
        check_menus_last("fixture.frm", &block, &mut failures);
        assert!(
            !failures.is_empty(),
            "a non-menu control after a menu control must fail the check"
        );
        assert!(failures[0].contains("cmd1"), "{failures:?}");
    }

    #[test]
    fn a_resource_offset_shifted_by_one_byte_fails_the_offset_resolution_check() {
        // A real, minimal .frx: two records, each declared length 4 with
        // four zero payload bytes, sixteen bytes total. The second record
        // starts at offset 8, right after the first record's own four byte
        // header and four byte payload.
        let frx_bytes: Vec<u8> = vec![
            4, 0, 0, 0, 0, 0, 0, 0, // record 1 at offset 0
            4, 0, 0, 0, 0, 0, 0, 0, // record 2 at offset 8
        ];
        let good_text = "   Icon            =   \"frmX.frx\":0000\r\n   Picture         =   \
                          \"frmX.frx\":0008\r\n";
        assert!(
            check_resource_offsets("fixture.frm", "frmX.frx", good_text, &frx_bytes).is_empty(),
            "the two real, unshifted offsets must resolve cleanly"
        );

        // Shifted by one byte: the second offset now reads four bytes
        // starting one byte into the first record's own payload, landing
        // on a length field this fixture never wrote.
        let shifted_text = "   Icon            =   \"frmX.frx\":0000\r\n   Picture         =   \
                             \"frmX.frx\":0009\r\n";
        let failures = check_resource_offsets("fixture.frm", "frmX.frx", shifted_text, &frx_bytes);
        assert!(
            !failures.is_empty(),
            "an offset shifted by one byte must fail the resolution check: {failures:?}"
        );
        assert!(
            failures.iter().any(|f| f.contains("frmX.frx")),
            "{failures:?}"
        );
    }

    // --- Task 2: the three encoding sweep breakages -----------------------

    #[test]
    fn a_file_beginning_with_a_byte_order_mark_fails_the_encoding_sweep() {
        let clean = b"VERSION 5.00\r\n".to_vec();
        assert!(
            super::check_text_file_encoding("clean.frm", &clean).is_empty(),
            "a clean file must pass the sweep"
        );

        let mut with_bom = vec![0xEF, 0xBB, 0xBF];
        with_bom.extend_from_slice(b"VERSION 5.00\r\n");
        let failures = super::check_text_file_encoding("bom.frm", &with_bom);
        assert!(
            !failures.is_empty(),
            "a file beginning with a byte order mark must fail the sweep"
        );
        assert!(failures[0].contains("byte order mark"), "{failures:?}");
    }

    #[test]
    fn a_bare_line_feed_fails_the_encoding_sweep() {
        let bare_lf = b"VERSION 5.00\r\nBegin VB.Form\nEnd\r\n".to_vec();
        let failures = super::check_text_file_encoding("bare_lf.frm", &bare_lf);
        assert!(
            !failures.is_empty(),
            "a bare line feed with no matching carriage return must fail the sweep"
        );
        assert!(failures[0].contains("bare line feed"), "{failures:?}");
    }

    #[test]
    fn a_byte_above_the_windows_1252_range_fails_the_encoding_sweep() {
        // The Windows-1252 encoder writes one byte per character; a raw
        // UTF-8 encoding of a character above U+00FF (here, U+20AC, the
        // Euro sign) leaking into the file is the shape this check exists
        // to catch: `€` encodes to the three bytes 0xE2 0x82 0xAC, which
        // together validate as UTF-8 and decode to a character above plain
        // ASCII, the one thing a genuine Windows-1252 byte almost never
        // does on its own.
        let mut leaked_utf8 = b"Caption =   \"".to_vec();
        leaked_utf8.extend_from_slice("€".as_bytes());
        leaked_utf8.extend_from_slice(b"\"\r\n");
        let failures = super::check_text_file_encoding("leaked_utf8.frm", &leaked_utf8);
        assert!(
            !failures.is_empty(),
            "a leaked UTF-8 encoded character must fail the sweep"
        );
        assert!(failures[0].contains("UTF-8"), "{failures:?}");
    }
}
