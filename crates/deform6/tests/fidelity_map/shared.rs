#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "the test harness, embedded in two binaries: it reads a vendored corpus this \
              repository controls, and it reports a fault in this repository loudly"
)]

//! The shared half of the committed fidelity map, `tests/fidelity.toml`.
//!
//! Two binaries compile this file. `crates/deform6/tests/fidelity_map.rs` is
//! the gate, and `crates/xtask` is the writer (`cargo run -p xtask --
//! update-fidelity`). Both take the measurement, the text and the parser from
//! here, so the gate and the writer cannot disagree about the shape of the
//! file.
//!
//! This file sits in a directory that Cargo does not build as a test target.
//! It holds no test of its own, so nothing here runs twice.
//!
//! # The text is written by hand, and read by the `toml` crate
//!
//! [`render`] builds the text with `format!`. [`parse`] reads it with
//! `toml::Table`. The two are independent, so a render fault that makes text
//! the parser reads in a different way shows as a difference.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use deform6::fidelity::Verdict;
use deform6::fidelity::census::{Array, Outcome};
use deform6::fidelity::walk::{Reason, walk};

/// The number of executables the corpus vendors.
pub(crate) const EXPECTED_PROGRAM_COUNT: usize = 44;

/// The structures the walk grades, in the order the walk reaches them. The
/// names are the ones each emitter gives as `Emit::STRUCTURE`.
pub(crate) const STRUCTURES: &[&str] = &[
    "VBHeader",
    "ProjectInfo",
    "GuiTableEntry",
    "GuiObjectInfo",
    "Object",
    "ObjectInfo",
    "PrivateObj",
    "OptionalObjectInfo",
    "ControlInfo",
    "EventStub",
];

/// The arrays the census counts, in the order the walk reaches them.
pub(crate) const ARRAYS: &[Array] = &[
    Array::DeclareEntries,
    Array::GuiTable,
    Array::Objects,
    Array::Controls,
    Array::EventSlots,
];

/// The name an array has in the map.
///
/// An exhaustive `match`, with no wildcard arm. A new array fails to compile
/// here until somebody gives it a name, and the map never holds `Debug` text.
pub(crate) const fn array_name(array: Array) -> &'static str {
    match array {
        Array::DeclareEntries => "DeclareEntries",
        Array::GuiTable => "GuiTable",
        Array::Objects => "Objects",
        Array::Controls => "Controls",
        Array::EventSlots => "EventSlots",
    }
}

/// The comment block above the first table. A constant, so a rewrite never
/// copies it from the old file.
pub(crate) const HEADER: &str = r#"# The committed fidelity map: what the fidelity walk measures in each corpus
# program.
#
# Rewrite this file with `cargo run -p xtask -- update-fidelity`. Do not edit
# it by hand. A rewrite on a clean tree gives no diff, and
# `cargo test -p deform6 --test fidelity_map` holds the tree to this file.
#
# A `layout` table describes one structure. `length` is its size in bytes.
# `modelled` is the number of bytes that the emitter writes back.
# `unmodelled` lists the ranges that it does not write, as [offset, length]
# pairs from the start of the structure. Every record of a structure has the
# same layout.
#
# A `program` table describes one corpus program. Its key is the path of the
# executable, relative to `corpus/`.
#
# For each structure, `records` is the number of records that the walk
# graded. `same` and `differs` count the modelled bytes that equal the file
# and the modelled bytes that do not. `absent` counts the records that the
# program does not have, such as the `PrivateObj` of a standard module.
# `refused` counts the records that the reader could not read.
#
# For each array, `rows` is the number of arrays that the walk counted.
# `declared` is the sum of the counts that the file declares, and `returned`
# is the sum of the entries that the reader returned. `not_whole` counts the
# arrays for which the two are not equal.
#
# A byte that is the same is coverage, not proof. A field that is read at one
# offset and written back at the same offset cannot differ from itself.
#
# An `EventStub` record is 13 bytes, and the emitter writes all 13. The reader
# keeps what 8 of them hold: `imm32`, and the handler address that it works
# out from the jump. The emitter works the jump back out of that address. The
# other 5 bytes are the opcodes of the native stub. The reader checks them and
# keeps no field for them. It decodes no stub of another shape, so such a
# stub shows as a refused record.
"#;

/// The ranges of one structure that the emitter does not write, and the
/// counts that follow from them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The size of the structure in bytes.
    pub(crate) length: u32,
    /// The number of bytes that the emitter writes.
    pub(crate) modelled: u32,
    /// The ranges that the emitter does not write, as `(offset, length)`
    /// from the start of the structure.
    pub(crate) unmodelled: Vec<(u32, u32)>,
}

/// One structure's totals in one program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Graded {
    pub(crate) records: u32,
    pub(crate) same: u32,
    pub(crate) differs: u32,
    pub(crate) absent: u32,
    pub(crate) refused: u32,
}

/// One array's totals in one program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counted {
    pub(crate) rows: u32,
    pub(crate) declared: u32,
    pub(crate) returned: u32,
    pub(crate) not_whole: u32,
}

/// Everything the map holds about one program.
///
/// Every structure in [`STRUCTURES`] and every array in [`ARRAYS`] has an
/// entry, including the ones that hold only zeros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProgramMap {
    pub(crate) structures: BTreeMap<&'static str, Graded>,
    pub(crate) arrays: BTreeMap<&'static str, Counted>,
}

impl ProgramMap {
    /// A program with a zero entry for every structure and every array.
    pub(crate) fn empty() -> Self {
        Self {
            structures: STRUCTURES
                .iter()
                .map(|name| (*name, Graded::default()))
                .collect(),
            arrays: ARRAYS
                .iter()
                .map(|array| (array_name(*array), Counted::default()))
                .collect(),
        }
    }
}

/// The whole map: one layout for each structure, and one entry for each
/// program, sorted by key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FidelityMap {
    pub(crate) layouts: BTreeMap<&'static str, Layout>,
    pub(crate) programs: BTreeMap<String, ProgramMap>,
}

/// Gives the path of the committed map: the workspace root's `tests/`
/// directory, beside `tests/ratios.toml`. `CARGO_MANIFEST_DIR` is a crate
/// directory in both binaries that compile this file, so the path is the same
/// in both.
pub(crate) fn fidelity_toml_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fidelity.toml")
}

/// Gives the directory that holds the `corpus/` this workspace vendors.
pub(crate) fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Gives every `.exe` under `root`, compared case-insensitively.
///
/// Skips each directory named `fetched`. The fetched programs have no
/// licence that permits a committed file derived from them, so the map must
/// never measure one.
pub(crate) fn executables(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    collect(root, &mut out)?;
    Ok(out)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|err| format!("reading {}: {err}", dir.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|err| format!("reading an entry of {}: {err}", dir.display()))?
            .path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "fetched") {
                continue;
            }
            collect(&path, out)?;
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push(path);
        }
    }
    Ok(())
}

/// Gives a program's key: its path relative to `root`, joined with `/` on
/// every platform.
pub(crate) fn program_key(exe: &Path, root: &Path) -> Result<String, String> {
    let relative = exe
        .strip_prefix(root)
        .map_err(|err| format!("{} is not under {}: {err}", exe.display(), root.display()))?;
    Ok(relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
}

fn structure_name(name: &str) -> Option<&'static str> {
    STRUCTURES.iter().copied().find(|known| *known == name)
}

fn array_by_name(name: &str) -> Option<&'static str> {
    ARRAYS
        .iter()
        .map(|array| array_name(*array))
        .find(|known| *known == name)
}

fn add(total: &mut u32, amount: u32, what: &str) -> Result<(), String> {
    *total = total
        .checked_add(amount)
        .ok_or_else(|| format!("the {what} total leaves a u32"))?;
    Ok(())
}

/// Walks every corpus program under [`corpus_root`] and measures the map.
///
/// # Errors
///
/// Gives an error when a program does not walk, when the walk grades a
/// structure the map does not name, when two records of one structure have
/// different layouts, when a structure has no record in the whole corpus,
/// and when a total leaves a `u32`.
pub(crate) fn measure() -> Result<FidelityMap, String> {
    let root = corpus_root();
    let mut map = FidelityMap::default();
    for exe in executables(&root)? {
        let key = program_key(&exe, &root)?;
        let data =
            std::fs::read(&exe).map_err(|err| format!("reading {}: {err}", exe.display()))?;
        let program = measure_program(&key, &data, &mut map.layouts)?;
        map.programs.insert(key, program);
    }
    for name in STRUCTURES {
        if !map.layouts.contains_key(name) {
            return Err(format!(
                "no corpus program has a {name} record, so its layout cannot be measured"
            ));
        }
    }
    Ok(map)
}

/// Measures one program, and adds each structure's layout to `layouts` the
/// first time the walk grades one.
///
/// # Errors
///
/// See [`measure`].
pub(crate) fn measure_program(
    key: &str,
    data: &[u8],
    layouts: &mut BTreeMap<&'static str, Layout>,
) -> Result<ProgramMap, String> {
    let found = walk(data).map_err(|err| format!("{key}: the fidelity walk stopped: {err}"))?;
    let mut program = ProgramMap::empty();

    for ledger in &found.ledgers {
        let name = structure_name(ledger.structure).ok_or_else(|| {
            format!(
                "{key}: the walk graded a {} record, and the map names no such structure",
                ledger.structure
            )
        })?;
        let graded = program
            .structures
            .get_mut(name)
            .expect("every structure has an entry");
        add(&mut graded.records, 1, "record")?;
        add(
            &mut graded.same,
            ledger.bytes_with(Verdict::Same),
            "same byte",
        )?;
        add(
            &mut graded.differs,
            ledger.bytes_with(Verdict::Differs),
            "differing byte",
        )?;

        let unmodelled: Vec<(u32, u32)> = ledger
            .runs_with(Verdict::Unmodelled)
            .map(|run| (run.span.at.get() - ledger.base.get(), run.span.len))
            .collect();
        let unwritten = unmodelled.iter().map(|(_, len)| *len).sum::<u32>();
        let layout = Layout {
            length: ledger.len,
            modelled: ledger.len - unwritten,
            unmodelled,
        };
        match layouts.get(name) {
            Some(known) if *known != layout => {
                return Err(format!(
                    "{key}: a {name} record at file offset {:#x} has the layout {layout:?}, \
                     and an earlier record has {known:?}",
                    ledger.base.get()
                ));
            }
            Some(_) => {}
            None => {
                layouts.insert(name, layout);
            }
        }
    }

    for row in &found.ungraded {
        let name = structure_name(row.structure).ok_or_else(|| {
            format!(
                "{key}: the walk could not grade a {} record, and the map names no such structure",
                row.structure
            )
        })?;
        let graded = program
            .structures
            .get_mut(name)
            .expect("every structure has an entry");
        match row.reason {
            Reason::Absent => add(&mut graded.absent, 1, "absent record")?,
            Reason::Refused(_) => add(&mut graded.refused, 1, "refused record")?,
        }
    }

    for count in &found.counts {
        let counted = program
            .arrays
            .get_mut(array_name(count.array))
            .expect("every array has an entry");
        add(&mut counted.rows, 1, "row")?;
        add(&mut counted.declared, count.declared, "declared")?;
        add(&mut counted.returned, count.recovered, "returned")?;
        if count.outcome != Outcome::Whole {
            add(&mut counted.not_whole, 1, "not whole")?;
        }
    }

    Ok(program)
}

/// Renders a TOML key or string, quoted and escaped by the `toml` crate.
fn quoted(text: &str) -> String {
    toml::Value::String(text.to_owned()).to_string()
}

/// Renders one layout table.
pub(crate) fn render_layout(name: &str, layout: &Layout) -> String {
    let ranges = layout
        .unmodelled
        .iter()
        .map(|(at, len)| format!("[0x{at:02X}, {len}]"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "[layout.{name}]\nlength = {}\nmodelled = {}\nunmodelled = [{ranges}]\n",
        layout.length, layout.modelled
    )
}

/// Renders one program table. The gate prints the same text as the block to
/// paste, so the two cannot differ.
pub(crate) fn render_program(key: &str, program: &ProgramMap) -> String {
    let mut out = format!("[program.{}]\n", quoted(key));
    for name in STRUCTURES {
        let graded = program.structures.get(name).copied().unwrap_or_default();
        out.push_str(&format!(
            "structure.{name} = {{ records = {}, same = {}, differs = {}, absent = {}, refused = {} }}\n",
            graded.records, graded.same, graded.differs, graded.absent, graded.refused
        ));
    }
    for array in ARRAYS {
        let name = array_name(*array);
        let counted = program.arrays.get(name).copied().unwrap_or_default();
        out.push_str(&format!(
            "array.{name} = {{ rows = {}, declared = {}, returned = {}, not_whole = {} }}\n",
            counted.rows, counted.declared, counted.returned, counted.not_whole
        ));
    }
    out
}

/// Renders the whole file: [`HEADER`], then every layout in [`STRUCTURES`]
/// order, then every program in key order. One blank line comes before each
/// table, and the file ends with one newline.
pub(crate) fn render(map: &FidelityMap) -> String {
    let mut out = String::from(HEADER);
    for name in STRUCTURES {
        if let Some(layout) = map.layouts.get(name) {
            out.push('\n');
            out.push_str(&render_layout(name, layout));
        }
    }
    for (key, program) in &map.programs {
        out.push('\n');
        out.push_str(&render_program(key, program));
    }
    out
}

fn table<'a>(value: &'a toml::Value, what: &str) -> Result<&'a toml::Table, String> {
    value
        .as_table()
        .ok_or_else(|| format!("{what} is not a table"))
}

fn number(table: &toml::Table, field: &str, what: &str) -> Result<u32, String> {
    let value = table
        .get(field)
        .ok_or_else(|| format!("{what} has no {field}"))?;
    let integer = value
        .as_integer()
        .ok_or_else(|| format!("{what}.{field} is not an integer"))?;
    u32::try_from(integer)
        .map_err(|_ignored| format!("{what}.{field} = {integer} is not a count that fits a u32"))
}

fn only_fields(table: &toml::Table, fields: &[&str], what: &str) -> Result<(), String> {
    for key in table.keys() {
        if !fields.contains(&key.as_str()) {
            return Err(format!("{what} has an unknown entry {key:?}"));
        }
    }
    for field in fields {
        if !table.contains_key(*field) {
            return Err(format!("{what} has no {field}"));
        }
    }
    Ok(())
}

fn parse_layout(value: &toml::Value, what: &str) -> Result<Layout, String> {
    let fields = table(value, what)?;
    only_fields(fields, &["length", "modelled", "unmodelled"], what)?;
    let ranges = fields
        .get("unmodelled")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("{what}.unmodelled is not an array"))?;
    let mut unmodelled = Vec::new();
    for range in ranges {
        let pair = range
            .as_array()
            .filter(|pair| pair.len() == 2)
            .ok_or_else(|| format!("{what}.unmodelled holds an entry that is not a pair"))?;
        let mut numbers = pair.iter().map(|part| {
            part.as_integer()
                .and_then(|integer| u32::try_from(integer).ok())
                .ok_or_else(|| format!("{what}.unmodelled holds a value that is not a u32"))
        });
        let at = numbers
            .next()
            .ok_or_else(|| format!("{what}.unmodelled is short"))??;
        let len = numbers
            .next()
            .ok_or_else(|| format!("{what}.unmodelled is short"))??;
        unmodelled.push((at, len));
    }
    Ok(Layout {
        length: number(fields, "length", what)?,
        modelled: number(fields, "modelled", what)?,
        unmodelled,
    })
}

fn parse_program(key: &str, value: &toml::Value) -> Result<ProgramMap, String> {
    let what = format!("program.{}", quoted(key));
    let fields = table(value, &what)?;
    only_fields(fields, &["structure", "array"], &what)?;
    let mut program = ProgramMap::empty();

    let structures = table(&fields["structure"], &format!("{what}.structure"))?;
    only_fields(structures, STRUCTURES, &format!("{what}.structure"))?;
    for (name, entry) in structures {
        let entry_what = format!("{what}.structure.{name}");
        let entry = table(entry, &entry_what)?;
        only_fields(
            entry,
            &["records", "same", "differs", "absent", "refused"],
            &entry_what,
        )?;
        let known = structure_name(name).expect("only_fields admits only known names");
        program.structures.insert(
            known,
            Graded {
                records: number(entry, "records", &entry_what)?,
                same: number(entry, "same", &entry_what)?,
                differs: number(entry, "differs", &entry_what)?,
                absent: number(entry, "absent", &entry_what)?,
                refused: number(entry, "refused", &entry_what)?,
            },
        );
    }

    let array_names: Vec<&str> = ARRAYS.iter().map(|array| array_name(*array)).collect();
    let arrays = table(&fields["array"], &format!("{what}.array"))?;
    only_fields(arrays, &array_names, &format!("{what}.array"))?;
    for (name, entry) in arrays {
        let entry_what = format!("{what}.array.{name}");
        let entry = table(entry, &entry_what)?;
        only_fields(
            entry,
            &["rows", "declared", "returned", "not_whole"],
            &entry_what,
        )?;
        let known = array_by_name(name).expect("only_fields admits only known names");
        program.arrays.insert(
            known,
            Counted {
                rows: number(entry, "rows", &entry_what)?,
                declared: number(entry, "declared", &entry_what)?,
                returned: number(entry, "returned", &entry_what)?,
                not_whole: number(entry, "not_whole", &entry_what)?,
            },
        );
    }
    Ok(program)
}

/// Reads a map from its text.
///
/// Strict: an unknown entry, a missing entry, a value that is not a count, and
/// a structure or array the map does not name are all errors.
///
/// # Errors
///
/// Gives an error that names the entry at fault.
pub(crate) fn parse(text: &str) -> Result<FidelityMap, String> {
    let root: toml::Table = text
        .parse()
        .map_err(|err| format!("tests/fidelity.toml is not valid TOML: {err}"))?;
    only_fields(&root, &["layout", "program"], "tests/fidelity.toml")?;

    let mut map = FidelityMap::default();
    let layouts = table(&root["layout"], "layout")?;
    only_fields(layouts, STRUCTURES, "layout")?;
    for (name, value) in layouts {
        let known = structure_name(name).expect("only_fields admits only known names");
        map.layouts
            .insert(known, parse_layout(value, &format!("layout.{name}"))?);
    }

    for (key, value) in table(&root["program"], "program")? {
        map.programs.insert(key.clone(), parse_program(key, value)?);
    }
    Ok(map)
}

/// Which way a number moved, as the walk measures it against the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    /// The walk measures a worse number than the map holds.
    Regression,
    /// The walk measures a better number than the map holds.
    MovedUp,
    /// The number moved, and neither way is better.
    Changed,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Regression => "REGRESSION",
            Self::MovedUp => "MOVED UP",
            Self::Changed => "CHANGED",
        })
    }
}

/// Which way is better for one field.
#[derive(Clone, Copy)]
enum Better {
    Higher,
    Lower,
    Neither,
}

fn direction(better: Better, held: u32, measured: u32) -> Direction {
    match (better, measured > held) {
        (Better::Neither, _) => Direction::Changed,
        (Better::Higher, true) | (Better::Lower, false) => Direction::MovedUp,
        (Better::Higher, false) | (Better::Lower, true) => Direction::Regression,
    }
}

/// One difference between two maps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Change {
    /// One number of one program moved.
    Value {
        key: String,
        field: String,
        held: u32,
        measured: u32,
        direction: Direction,
    },
    /// One number of one layout moved.
    LayoutValue {
        structure: &'static str,
        field: &'static str,
        held: u32,
        measured: u32,
        direction: Direction,
    },
    /// The unmodelled ranges of one layout moved.
    LayoutRanges {
        structure: &'static str,
        held: Vec<(u32, u32)>,
        measured: Vec<(u32, u32)>,
    },
    /// One layout is in only one of the two maps.
    LayoutPresence {
        structure: &'static str,
        in_map: bool,
    },
    /// The walk measures a program that the map does not hold.
    Missing { key: String },
    /// The map holds a program that the walk does not measure.
    Stale { key: String },
}

impl Change {
    /// The program this change belongs to, if any.
    pub(crate) fn key(&self) -> Option<&str> {
        match self {
            Self::Value { key, .. } | Self::Missing { key } | Self::Stale { key } => Some(key),
            Self::LayoutValue { .. } | Self::LayoutRanges { .. } | Self::LayoutPresence { .. } => {
                None
            }
        }
    }
}

fn ranges_text(ranges: &[(u32, u32)]) -> String {
    let parts: Vec<String> = ranges
        .iter()
        .map(|(at, len)| format!("[0x{at:02X}, {len}]"))
        .collect();
    format!("[{}]", parts.join(", "))
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value {
                key,
                field,
                held,
                measured,
                direction,
            } => write!(
                f,
                "{key}: {direction} {field}: the map holds {held}, the walk measures {measured}"
            ),
            Self::LayoutValue {
                structure,
                field,
                held,
                measured,
                direction,
            } => write!(
                f,
                "layout.{structure}: {direction} {field}: the map holds {held}, the walk measures {measured}"
            ),
            Self::LayoutRanges {
                structure,
                held,
                measured,
            } => write!(
                f,
                "layout.{structure}: {} unmodelled: the map holds {}, the walk measures {}",
                Direction::Changed,
                ranges_text(held),
                ranges_text(measured)
            ),
            Self::LayoutPresence { structure, in_map } => {
                if *in_map {
                    write!(
                        f,
                        "layout.{structure}: the map holds this layout, and the walk grades no such record"
                    )
                } else {
                    write!(
                        f,
                        "layout.{structure}: the walk grades this structure, and the map holds no layout for it"
                    )
                }
            }
            Self::Missing { key } => write!(
                f,
                "{key}: this corpus program has no table in tests/fidelity.toml"
            ),
            Self::Stale { key } => write!(
                f,
                "{key}: tests/fidelity.toml has a table for this path, and no corpus program has it"
            ),
        }
    }
}

fn push_value(
    out: &mut Vec<Change>,
    key: &str,
    field: String,
    better: Better,
    held: u32,
    measured: u32,
) {
    if held != measured {
        out.push(Change::Value {
            key: key.to_owned(),
            field,
            held,
            measured,
            direction: direction(better, held, measured),
        });
    }
}

fn program_changes(out: &mut Vec<Change>, key: &str, held: &ProgramMap, measured: &ProgramMap) {
    for name in STRUCTURES {
        let old = held.structures.get(name).copied().unwrap_or_default();
        let new = measured.structures.get(name).copied().unwrap_or_default();
        let field = |column: &str| format!("structure.{name}.{column}");
        push_value(
            out,
            key,
            field("records"),
            Better::Higher,
            old.records,
            new.records,
        );
        push_value(out, key, field("same"), Better::Higher, old.same, new.same);
        push_value(
            out,
            key,
            field("differs"),
            Better::Lower,
            old.differs,
            new.differs,
        );
        push_value(
            out,
            key,
            field("absent"),
            Better::Neither,
            old.absent,
            new.absent,
        );
        push_value(
            out,
            key,
            field("refused"),
            Better::Lower,
            old.refused,
            new.refused,
        );
    }
    for array in ARRAYS {
        let name = array_name(*array);
        let old = held.arrays.get(name).copied().unwrap_or_default();
        let new = measured.arrays.get(name).copied().unwrap_or_default();
        let field = |column: &str| format!("array.{name}.{column}");
        push_value(out, key, field("rows"), Better::Higher, old.rows, new.rows);
        push_value(
            out,
            key,
            field("declared"),
            Better::Neither,
            old.declared,
            new.declared,
        );
        push_value(
            out,
            key,
            field("returned"),
            Better::Higher,
            old.returned,
            new.returned,
        );
        push_value(
            out,
            key,
            field("not_whole"),
            Better::Lower,
            old.not_whole,
            new.not_whole,
        );
    }
}

fn layout_changes(out: &mut Vec<Change>, held: &FidelityMap, measured: &FidelityMap) {
    for structure in STRUCTURES.iter().copied() {
        match (held.layouts.get(structure), measured.layouts.get(structure)) {
            (Some(old), Some(new)) => {
                if old.length != new.length {
                    out.push(Change::LayoutValue {
                        structure,
                        field: "length",
                        held: old.length,
                        measured: new.length,
                        direction: Direction::Changed,
                    });
                }
                if old.modelled != new.modelled {
                    out.push(Change::LayoutValue {
                        structure,
                        field: "modelled",
                        held: old.modelled,
                        measured: new.modelled,
                        direction: direction(Better::Higher, old.modelled, new.modelled),
                    });
                }
                if old.unmodelled != new.unmodelled {
                    out.push(Change::LayoutRanges {
                        structure,
                        held: old.unmodelled.clone(),
                        measured: new.unmodelled.clone(),
                    });
                }
            }
            (Some(_), None) => out.push(Change::LayoutPresence {
                structure,
                in_map: true,
            }),
            (None, Some(_)) => out.push(Change::LayoutPresence {
                structure,
                in_map: false,
            }),
            (None, None) => {}
        }
    }
}

/// Lists every value in which `measured` differs from `held`: the layouts
/// first, in [`STRUCTURES`] order, then the programs in key order. An equal
/// value gives no entry.
pub(crate) fn changes(held: &FidelityMap, measured: &FidelityMap) -> Vec<Change> {
    let mut out = Vec::new();
    layout_changes(&mut out, held, measured);
    let mut keys: Vec<&String> = held
        .programs
        .keys()
        .chain(measured.programs.keys())
        .collect();
    keys.sort();
    keys.dedup();
    for key in keys {
        match (held.programs.get(key), measured.programs.get(key)) {
            (Some(old), Some(new)) => program_changes(&mut out, key, old, new),
            (Some(_), None) => out.push(Change::Stale { key: key.clone() }),
            (None, Some(_)) => out.push(Change::Missing { key: key.clone() }),
            (None, None) => {}
        }
    }
    out
}
