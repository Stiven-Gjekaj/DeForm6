//! `derive-opcode-table`: writes the table [`deform6::vb::opcodes::OpcodeTable::parse`]
//! reads, from a type library a user owns a lawful copy of.
//!
//! Split into two halves on purpose, because only one half can ever run on
//! the host that builds this repository.
//!
//! [`serialize_table`] is plain Rust. It takes `(control_type, opcode,
//! payload_type, property_name)` tuples and writes the TOML format
//! `03-CONTEXT.md` D-01's checkpoint chose. It runs on every host, and it
//! is fully unit tested here, against tables this file's own tests build in
//! memory, per `AGENTS.md`'s "build the state a test needs inside the
//! test."
//!
//! The COM walk that produces those tuples from a type library is behind
//! `#[cfg(windows)]`. On every other host `derive_opcode_table` refuses with
//! one sentence naming the requirement and reads no file, matching the
//! usage-line refusal `xtask`'s own `run` already gives an unknown
//! subcommand.
//!
//! Per `AGENTS.md`'s "What may enter this repository": this tool is
//! DeForm6's own original code, and it is committed. The table it writes is
//! calculated from a third party file the author does not own, so it is
//! never committed; `.gitignore` excludes its default output path.

use deform6::vb::opcodes::PayloadType;
use std::collections::BTreeMap;

/// The tool's default output path, relative to the workspace root.
///
/// `.gitignore` excludes exactly this path. Nested one directory down so
/// the pattern's own extraction (a plan verification step greps for a line
/// starting with a non-`/` character, since `git check-ignore` treats a
/// leading `/` argument as an OS-absolute path rather than a repo-relative
/// one) has a directory segment to anchor on instead.
pub const DEFAULT_OUTPUT_PATH: &str = "derived/opcode-table.toml";

/// One row of a serialized table, matching the shape
/// [`toml::to_string`] writes it in.
#[derive(serde::Serialize)]
#[allow(
    dead_code,
    reason = "constructed by serialize_table, whose only real caller is the #[cfg(windows)] \
              COM walk this host never compiles; this file's own tests are the other caller, \
              and both leave the plain bin target's own dead-code analysis with nothing live \
              to point at on a non-Windows host"
)]
struct SerRow {
    name: String,
    payload: &'static str,
}

/// Writes the TOML table format `OpcodeTable::parse` reads: one table per
/// control type, each key an opcode, each value a `{ name, payload }` row.
///
/// Takes the tuples directly, in the order
/// `(control_type, opcode, payload_type, property_name)`, so a test can
/// pass rows built in memory and assert on the result, rather than reading
/// them from a file.
///
/// The `toml` crate owns every quoting and escaping decision a property
/// name needs; this function builds a plain, serializable document and
/// hands it to [`toml::to_string`] rather than formatting text by hand,
/// so a name holding a quote or a backslash is never this function's
/// problem to get right.
///
/// `#[allow(dead_code)]`: the same reason [`SerRow`] carries one. This
/// function's real caller is the `#[cfg(windows)]` COM walk, which this
/// host never compiles; this file's own tests call it directly, and cargo
/// clippy's own `--all-targets` bin-target pass sees neither.
#[must_use]
#[allow(
    dead_code,
    reason = "the real caller is the #[cfg(windows)] COM walk, which this host never compiles; \
              this file's own tests call it directly"
)]
pub fn serialize_table(rows: &[(u8, u8, PayloadType, &str)]) -> String {
    let mut doc: BTreeMap<String, BTreeMap<String, SerRow>> = BTreeMap::new();
    for &(control_type, opcode, payload, name) in rows {
        doc.entry(control_type.to_string()).or_default().insert(
            opcode.to_string(),
            SerRow {
                name: name.to_owned(),
                payload: payload_word(payload),
            },
        );
    }
    // This document is built entirely from Rust `String`s and a fixed set
    // of static payload words this function itself chose; nothing in its
    // shape can trigger `toml::to_string`'s only failure modes (a NaN or
    // infinite float, neither of which this document ever contains). The
    // fallback below is never exercised by a real call; it exists so this
    // function stays infallible rather than reaching for `unwrap`, which
    // this workspace's lint wall denies.
    toml::to_string(&doc).unwrap_or_default()
}

/// Gives the TOML string form of a payload type, matching
/// `deform6::vb::opcodes::PayloadType`'s own `serde::Deserialize` derive,
/// which reads a unit variant back from its bare Rust identifier.
const fn payload_word(payload: PayloadType) -> &'static str {
    match payload {
        PayloadType::Byte => "Byte",
        PayloadType::Boolean => "Boolean",
        PayloadType::Integer => "Integer",
        PayloadType::Long => "Long",
        PayloadType::Single => "Single",
        PayloadType::Text => "Text",
        PayloadType::Picture => "Picture",
        PayloadType::Font => "Font",
        PayloadType::Position => "Position",
    }
}

/// Runs `derive-opcode-table`. `args` is everything after the subcommand
/// name; on every host but Windows it is read no further, because the walk
/// this subcommand performs cannot run here at all.
///
/// Gives the process exit code, matching `xtask::run`'s own convention.
pub fn derive_opcode_table(args: &[String]) -> i32 {
    #[cfg(not(windows))]
    {
        let _ = args;
        eprintln!(
            "xtask: derive-opcode-table needs a Windows host with a lawful, locally owned \
             Visual Basic 6 install, and it read no file. Its default output path is \
             {DEFAULT_OUTPUT_PATH}, which .gitignore excludes."
        );
        1
    }
    #[cfg(windows)]
    {
        windows_walk::run(args, DEFAULT_OUTPUT_PATH)
    }
}

/// The COM walk over a type library. `#[cfg(windows)]`: this module never
/// compiles on any other host, and `derive_opcode_table` never calls into
/// it on any other host either.
///
/// # Untested by construction on this repository's own gate
///
/// Every clean-clone `cargo test --workspace` this repository has ever run,
/// including this plan's own, runs on a host `cfg(windows)` excludes this
/// module from. A Windows host with a lawful, locally owned Visual Basic 6
/// install is needed before this walk can be called tested, the same
/// standing `crates/deform6/src/vb/project.rs`'s `CompileMode::PCode`
/// branch is in for a P-code binary this corpus does not hold. Do not claim
/// this walk works; it has not been run.
#[cfg(windows)]
mod windows_walk {
    /// Walks a type library over COM and writes the derived table.
    ///
    /// Gives the process exit code. This is a stub: the real COM walk needs
    /// a Windows-only dependency this workspace does not carry today, and
    /// adding one is follow-up work for a human at a Windows host, per
    /// `03-CONTEXT.md` D-01's own scoping. It fails loudly rather than
    /// silently, which is `AGENTS.md`'s bar for a branch that has not been
    /// proven, not a claim that the walk is finished.
    pub(crate) fn run(_args: &[String], _default_output_path: &str) -> i32 {
        eprintln!("xtask: the COM walk over a type library is not yet implemented");
        1
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{payload_word, serialize_table};
    use deform6::vb::opcodes::{OpcodeTable, PayloadType};

    #[test]
    fn an_empty_list_serializes_to_a_table_that_parses_to_zero_rows() {
        let rendered = serialize_table(&[]);
        let table = OpcodeTable::parse(rendered.as_bytes()).unwrap();
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn one_row_serializes_and_parses_back_to_the_same_entry() {
        let rows = [(13_u8, 31_u8, PayloadType::Byte, "DrawMode")];
        let rendered = serialize_table(&rows);
        let table = OpcodeTable::parse(rendered.as_bytes()).unwrap();
        let entry = table.lookup(13, 31).unwrap();
        assert_eq!(entry.name, "DrawMode");
        assert_eq!(entry.payload, PayloadType::Byte);
    }

    /// A property name that holds a character the format must quote or
    /// escape: a double quote and a backslash, either of which breaks a
    /// hand-written `"{name}"` format string that does not escape them.
    #[test]
    fn a_name_holding_a_quote_and_a_backslash_round_trips_intact() {
        let name = "Weird\"Name\\Here";
        let rows = [(1_u8, 5_u8, PayloadType::Text, name)];
        let rendered = serialize_table(&rows);
        let table = OpcodeTable::parse(rendered.as_bytes()).unwrap();
        assert_eq!(table.lookup(1, 5).unwrap().name, name);
    }

    /// A round trip through this crate's own `serialize_table` and
    /// `deform6`'s own `OpcodeTable::parse` is self agreement, not
    /// verification: it proves only that the two agree with each other.
    /// The real check on the safe-provenance subset is plan 03-10's
    /// differential gate, against the committed `.frm` source.
    #[test]
    fn a_multi_row_multi_control_type_table_round_trips_through_both_functions() {
        let rows = [
            (13_u8, 31_u8, PayloadType::Byte, "DrawMode"),
            (4_u8, 31_u8, PayloadType::Byte, "Appearance"),
            (1_u8, 31_u8, PayloadType::Byte, "BackStyle"),
            (4_u8, 4_u8, PayloadType::Position, "Position"),
        ];
        let rendered = serialize_table(&rows);
        let table = OpcodeTable::parse(rendered.as_bytes()).unwrap();
        assert_eq!(table.len(), rows.len());
        for &(control_type, opcode, payload, name) in &rows {
            let entry = table.lookup(control_type, opcode).unwrap();
            assert_eq!(entry.name, name);
            assert_eq!(entry.payload, payload);
        }
    }

    #[test]
    fn payload_word_gives_the_exact_rust_identifier_for_every_variant() {
        assert_eq!(payload_word(PayloadType::Byte), "Byte");
        assert_eq!(payload_word(PayloadType::Boolean), "Boolean");
        assert_eq!(payload_word(PayloadType::Integer), "Integer");
        assert_eq!(payload_word(PayloadType::Long), "Long");
        assert_eq!(payload_word(PayloadType::Single), "Single");
        assert_eq!(payload_word(PayloadType::Text), "Text");
        assert_eq!(payload_word(PayloadType::Picture), "Picture");
        assert_eq!(payload_word(PayloadType::Font), "Font");
        assert_eq!(payload_word(PayloadType::Position), "Position");
    }
}
