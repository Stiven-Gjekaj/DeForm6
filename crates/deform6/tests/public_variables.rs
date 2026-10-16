#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "this is the test harness, not the library under test: it reads the vendored \
              corpus that this repository controls"
)]

//! The public variables of each object of the corpus against its source.
//!
//! For each executable of `corpus/` and `corpus-pcode/`, each object that
//! has a source file in the folder of the executable under `corpus/`, found
//! by its `Attribute VB_Name`, gives its public variables through
//! [`deform6::vb::privateobj::public_variables`]. The names must be the
//! names of the `Public` variables of the source, in the order of the
//! source. The test counts the objects and the variables, and pins the
//! totals.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use deform6::read::pe::PeImage;
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::object::ObjectTable;
use deform6::vb::privateobj::{ObjectInfo, PrivateObj, public_variables};
use deform6::vb::project::{ObjectTableHead, ProjectInfo};

/// Gives each `.exe` under `root`, sorted.
fn executables(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
            {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Gives the text of each source file in `dir`, by its `Attribute VB_Name`.
fn sources(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let source = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                ["frm", "bas", "cls", "ctl"].contains(&ext.to_ascii_lowercase().as_str())
            });
        if !source {
            continue;
        }
        let text: String = std::fs::read(&path)
            .unwrap()
            .into_iter()
            .map(char::from)
            .collect();
        let name = text.lines().find_map(|line| {
            line.strip_prefix("Attribute VB_Name = \"")
                .and_then(|rest| rest.split('"').next())
                .map(str::to_owned)
        });
        if let Some(name) = name {
            out.insert(name, text);
        }
    }
    out
}

/// Gives the names of the `Public` variables of `text`, in the order of the
/// source: each name of a line that starts with `Public` and declares no
/// procedure, type, enumeration, constant, event or `Declare`.
fn declared(text: &str) -> Vec<String> {
    const NOT_A_VARIABLE: [&str; 8] = [
        "Sub", "Function", "Property", "Enum", "Type", "Const", "Event", "Declare",
    ];
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("Public ") else {
            continue;
        };
        let rest = rest.split('\'').next().unwrap_or("");
        let rest = rest
            .trim_start()
            .strip_prefix("WithEvents ")
            .unwrap_or(rest);
        if NOT_A_VARIABLE.contains(&rest.split_whitespace().next().unwrap_or("")) {
            continue;
        }
        for part in rest.split(',') {
            let part = part
                .trim_start()
                .strip_prefix("WithEvents ")
                .unwrap_or(part);
            let name: String = part
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            out.push(name);
        }
    }
    out
}

/// The counts of one corpus.
#[derive(Debug, Default, PartialEq, Eq)]
struct Census {
    /// The objects with a `PrivateObj` and a source file.
    objects: usize,
    /// The public variables that the source declares in those objects.
    variables: usize,
    /// The objects whose variables differ from the source, by name.
    differ: Vec<String>,
}

/// Compares each object of each executable under `root` with its source,
/// which is in the same folder under `corpus/`.
fn census(root: &Path) -> Census {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let mut out = Census::default();
    for exe in executables(root) {
        let data = std::fs::read(&exe).unwrap();
        let image = PeImage::parse(&data).unwrap();
        let header = VbHeader::read(&header_region(&image).unwrap()).unwrap();
        let info = ProjectInfo::read(&image, header.lp_project_data).unwrap();
        let head = ObjectTableHead::read(&image, info.lp_object_table).unwrap();
        let table = ObjectTable::walk(&image, info.lp_object_table, &head).unwrap();
        let relative = exe.parent().unwrap().strip_prefix(root).unwrap();
        let texts = sources(&corpus.join(relative));
        for object in &table.objects {
            let Some(text) = texts.get(&object.name) else {
                continue;
            };
            let Ok(object_info) = ObjectInfo::read(&image, object.lp_object_info) else {
                continue;
            };
            let private = PrivateObj::read(&image, object_info.lp_private_object).unwrap();
            if private == PrivateObj::Absent {
                continue;
            }
            let found: Vec<String> = public_variables(&image, &private)
                .into_iter()
                .map(|variable| variable.name)
                .collect();
            let expected = declared(text);
            out.objects += 1;
            out.variables += expected.len();
            if found != expected {
                out.differ.push(format!(
                    "{} {}: {found:?} against {expected:?}",
                    exe.display(),
                    object.name
                ));
            }
        }
    }
    out
}

#[test]
fn each_public_variable_of_the_native_corpus_has_the_name_of_its_source() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let found = census(&root);
    assert!(found.differ.is_empty(), "{:#?}", found.differ);
    assert_eq!(
        (found.objects, found.variables),
        (91, 62),
        "the native corpus now gives {} objects and {} public variables",
        found.objects,
        found.variables
    );
}

#[test]
fn each_public_variable_of_the_pcode_corpus_has_the_name_of_its_source() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus-pcode");
    let found = census(&root);
    assert!(found.differ.is_empty(), "{:#?}", found.differ);
    assert_eq!(
        (found.objects, found.variables),
        (86, 62),
        "the P-code corpus now gives {} objects and {} public variables",
        found.objects,
        found.variables
    );
}
