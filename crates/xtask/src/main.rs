//! The developer tools this workspace runs by hand, never shipped.
//!
//! `cargo run -p xtask -- update-ratios` rewrites `tests/ratios.toml`.
//! `cargo run -p xtask -- derive-opcode-table` writes the opcode table
//! `deform6::vb::opcodes::OpcodeTable::parse` reads, from a type library on
//! a Windows host; see `opcode_table.rs`'s own doc comment for why this
//! command reads no file anywhere else. Any other argument, or none, prints
//! the usage line and exits non-zero: a mistyped subcommand is a loud
//! failure, not a silent success.
//!
//! This crate carries the full workspace lint wall at its own top level
//! (see `Cargo.toml`'s `[lints] workspace = true`): a broken developer
//! tool should fail the gate like anything else here. The file this crate
//! embeds below, `ratios.rs`, carries its own narrower, documented allow
//! list -- it is the test harness, read here rather than reimplemented,
//! per T-02-45 ("both take their counts from the one function plan 02-08
//! wrote").

// `crates/deform6/tests/ratios.rs` gives the shared formatting (`HEADER`,
// `format_entry`, `format_ratio`, `parse_ratios_toml`, `PinnedEntry`,
// `declared_total`, `ratios_toml_path`) and, through its own `differential`
// submodule, `program_counts`: the one function that computes the two
// pinned counts, and the `support` module, the second, independent
// `.vbp`/source reader. `#[path]` recompiles `ratios.rs`'s source directly
// into this crate as a submodule, and `ratios.rs`'s own `#[path =
// "differential.rs"] mod differential;` then resolves relative to
// `ratios.rs`'s own directory regardless of who embedded it, giving
// `ratios::differential` here.
//
// This crate deliberately does NOT also declare its own, separate `#[path
// = "../../deform6/tests/differential.rs"] mod differential;`: two
// independent embeddings of the same file would compile as two distinct,
// incompatible `ProgramCounts` types with the same name (confirmed by
// E0308 when this was tried), so there is exactly one embedding of
// `differential.rs` in this binary's build graph, reached the same way
// `crates/deform6/tests/ratios.rs` itself reaches it.
#[path = "../../deform6/tests/ratios.rs"]
#[allow(
    dead_code,
    reason = "this module gets embedded into two different binaries, this file's own --test \
              target and this one, and each uses a different subset -- the same shape \
              support/mod.rs documents on its own module doc comment. This binary calls only \
              the shared writer/reader surface (format_entry, format_ratio, parse_ratios_toml, \
              declared_total, ratios_toml_path, HEADER, PinnedEntry) plus program_counts and \
              recovered_objects through its own differential submodule; the gate-only machinery \
              (check_program, gate_failures, the message builders) belongs to the --test target \
              alone"
)]
mod ratios;

mod opcode_table;

use std::path::Path;

use deform6::read::pe::PeImage;
use ratios::differential::program_counts;
use ratios::differential::support::vbp;

fn main() {
    std::process::exit(run(std::env::args().skip(1).collect()));
}

/// Runs one subcommand and gives the process exit code.
///
/// Takes the argument vector directly (not `std::env::args()`) so a test
/// can drive this without spawning a process.
fn run(args: Vec<String>) -> i32 {
    match args.first().map(String::as_str) {
        Some("update-ratios") => update_ratios(),
        Some("derive-opcode-table") => {
            opcode_table::derive_opcode_table(args.get(1..).unwrap_or(&[]))
        }
        Some("--help" | "-h") => {
            println!("{USAGE}");
            0
        }
        Some(other) => {
            eprintln!("xtask: unknown subcommand {other:?}\n{USAGE}");
            1
        }
        None => {
            eprintln!("xtask: missing subcommand\n{USAGE}");
            1
        }
    }
}

const USAGE: &str = "usage: cargo run -p xtask -- update-ratios | derive-opcode-table";

/// The number of corpus programs `update-ratios` refuses to write fewer
/// than. Matches `EXPECTED_PROGRAM_COUNT` in `crates/deform6/tests/ratios.rs`
/// and `differential.rs`'s own `EXPECTED_EXECUTABLE_COUNT`: this is the
/// same 44 every gate in this workspace already pins.
const MINIMUM_PROGRAM_COUNT: usize = 44;

/// One corpus program's key and its measured counts, ready to format.
struct Measured {
    key: String,
    recovered: u32,
    declared: u32,
}

/// Gives one program's key: the executable's path relative to `corpus/`,
/// with the platform's own separator normalised to `/` so the key never
/// depends on which operating system built it.
fn program_key(exe: &Path, root: &Path) -> Result<String, String> {
    let rel = exe
        .strip_prefix(root)
        .map_err(|err| format!("{}: not under {}: {err}", exe.display(), root.display()))?;
    Ok(rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
}

/// Walks every corpus program and computes its measured counts, through
/// [`program_counts`] alone -- the same function `crates/deform6/tests/ratios.rs`
/// calls, per T-02-45.
fn measure_all() -> Result<Vec<Measured>, String> {
    let root = vbp::corpus_root();
    let projects = vbp::project_files();
    let mut out = Vec::new();

    for exe in vbp::executables() {
        let key = program_key(&exe, &root)?;
        let bytes =
            std::fs::read(&exe).map_err(|err| format!("reading {}: {err}", exe.display()))?;
        let image = PeImage::parse(&bytes)
            .map_err(|err| format!("{}: PeImage::parse: {err:?}", exe.display()))?;
        let recovered_objects = ratios::differential::recovered_objects(&image);

        let project_path = vbp::select_project_file(&exe, &projects)
            .map_err(|err| format!("{}: {err}", exe.display()))?;
        let project = vbp::Project::read(&project_path);
        let declared = project.declared_objects();

        let counts = program_counts(&image, &declared, &recovered_objects);
        out.push(Measured {
            key,
            recovered: counts.recovered,
            declared: ratios::declared_total(&counts),
        });
    }

    out.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(out)
}

/// Builds the whole file's bytes: [`ratios::HEADER`], then every measured
/// program's block from [`ratios::format_entry`], joined so exactly one
/// blank line separates two entries and none trails the last one -- the
/// same shape the committed file already carries.
fn render(measured: &[Measured]) -> String {
    let mut body = String::new();
    for (i, m) in measured.iter().enumerate() {
        if i > 0 {
            body.push('\n');
        }
        body.push_str(&ratios::format_entry(&m.key, m.recovered, m.declared));
    }
    format!("{}{body}", ratios::HEADER)
}

/// Gives, for each key both the old file and the fresh measurement carry,
/// one line describing what changed: unchanged keys give nothing. A pure
/// function (it returns lines rather than printing them) so a test can
/// check its output directly instead of capturing stdout.
fn describe_changes(
    old: &std::collections::BTreeMap<String, ratios::PinnedEntry>,
    measured: &[Measured],
) -> Vec<String> {
    let mut lines = Vec::new();
    for m in measured {
        let Some(old_entry) = old.get(&m.key) else {
            lines.push(format!(
                "{}: new entry: {} of {}",
                m.key, m.recovered, m.declared
            ));
            continue;
        };
        if old_entry.recovered != m.recovered || old_entry.declared != m.declared {
            lines.push(format!(
                "{}: {} of {} -> {} of {}",
                m.key, old_entry.recovered, old_entry.declared, m.recovered, m.declared
            ));
        }
    }
    let measured_keys: std::collections::BTreeSet<&str> =
        measured.iter().map(|m| m.key.as_str()).collect();
    for stale_key in old.keys().filter(|k| !measured_keys.contains(k.as_str())) {
        lines.push(format!(
            "{stale_key}: removed, no corpus program names it any more"
        ));
    }
    lines
}

/// Refuses a walk that found fewer than [`MINIMUM_PROGRAM_COUNT`] programs.
/// A pure, one-argument function so a test can drive both branches without
/// touching the real corpus.
fn check_minimum_program_count(found: usize) -> Result<(), String> {
    if found < MINIMUM_PROGRAM_COUNT {
        return Err(format!(
            "found {found} corpus programs, refusing to write fewer than {MINIMUM_PROGRAM_COUNT}"
        ));
    }
    Ok(())
}

/// `ratios::format_entry` is the one function this writer and the `MOVED
/// UP` message both call, per T-02-45; the two never risk disagreeing over
/// the TOML grammar, because there is only one formatter. This parse is a
/// second, independent check that what that formatter produced is
/// genuinely valid TOML before it is written -- the `toml` crate this
/// phase adds for exactly this reads rendered text back, never writes it,
/// so the shared formatter stays the single source of the file's shape.
fn validate_toml(rendered: &str) -> Result<(), String> {
    rendered
        .parse::<toml::Table>()
        .map(|_| ())
        .map_err(|err| format!("the file this command rendered is not valid TOML: {err}"))
}

/// Rewrites `tests/ratios.toml`.
fn update_ratios() -> i32 {
    match update_ratios_inner() {
        Ok((count, changes)) => {
            for line in changes {
                println!("{line}");
            }
            println!(
                "xtask: wrote {count} entries to {}",
                ratios::ratios_toml_path().display()
            );
            0
        }
        Err(message) => {
            eprintln!("xtask: {message}");
            1
        }
    }
}

fn update_ratios_inner() -> Result<(usize, Vec<String>), String> {
    let measured = measure_all()?;
    check_minimum_program_count(measured.len())?;

    let path = ratios::ratios_toml_path();
    let changes = match std::fs::read_to_string(&path) {
        Ok(old_text) => describe_changes(&ratios::parse_ratios_toml(&old_text), &measured),
        Err(_) => Vec::new(),
    };

    let rendered = render(&measured);
    validate_toml(&rendered)?;
    std::fs::write(&path, &rendered).map_err(|err| format!("writing {}: {err}", path.display()))?;

    Ok((measured.len(), changes))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::integer_division,
        reason = "a test builds the state it needs and must fail loudly when that state is wrong"
    )]

    use super::{
        Measured, check_minimum_program_count, describe_changes, program_key, render, run,
        validate_toml,
    };
    use std::path::Path;

    #[test]
    fn program_key_normalises_to_forward_slashes() {
        let root = Path::new("/corpus");
        let key = program_key(Path::new("/corpus/vb6-code/Sepia-effect/Sepia.exe"), root)
            .expect("the path is under root");
        assert_eq!(key, "vb6-code/Sepia-effect/Sepia.exe");
    }

    #[test]
    fn program_key_refuses_a_path_outside_root() {
        let root = Path::new("/corpus");
        assert!(program_key(Path::new("/elsewhere/A.exe"), root).is_err());
    }

    #[test]
    fn render_preserves_the_order_it_is_given_and_produces_valid_toml() {
        let measured = vec![
            Measured {
                key: "b/B.exe".to_owned(),
                recovered: 1,
                declared: 2,
            },
            Measured {
                key: "a/A.exe".to_owned(),
                recovered: 3,
                declared: 4,
            },
        ];
        let rendered = render(&measured);
        assert!(
            validate_toml(&rendered).is_ok(),
            "render must produce valid TOML: {rendered}"
        );
        let b_pos = rendered.find("b/B.exe").expect("b/B.exe is rendered");
        let a_pos = rendered.find("a/A.exe").expect("a/A.exe is rendered");
        assert!(
            b_pos < a_pos,
            "render must not reorder its input; sorting is measure_all's job, not render's"
        );
    }

    /// Behaviour four ("the block the `MOVED UP` message prints is byte for
    /// byte the block the command writes for that entry"): the writer's
    /// per-entry block, sliced out of a full render, must equal
    /// `ratios::format_entry` called directly with the same three values --
    /// the same call `crates/deform6/tests/ratios.rs`'s own `MOVED UP`
    /// message builder makes. Both this test and that one exercise the one
    /// function `format_entry`, so a change to its shape cannot pass one
    /// and silently break the other.
    #[test]
    fn the_writers_per_entry_block_is_byte_for_byte_format_entrys_output() {
        let measured = vec![Measured {
            key: "vb6-code/Grayscale-effect/Grayscale.exe".to_owned(),
            recovered: 12,
            declared: 34,
        }];
        let rendered = render(&measured);
        let expected_block =
            super::ratios::format_entry("vb6-code/Grayscale-effect/Grayscale.exe", 12, 34);
        assert!(
            rendered.ends_with(&expected_block),
            "the rendered file's one entry must be exactly format_entry's output:\nrendered:\n{rendered}\nexpected block:\n{expected_block}"
        );
    }

    #[test]
    fn check_minimum_program_count_refuses_a_short_walk_and_says_how_many_it_found() {
        let err = check_minimum_program_count(43).expect_err("43 is below the minimum");
        assert!(err.contains("43"));
        assert!(check_minimum_program_count(44).is_ok());
    }

    #[test]
    fn describe_changes_reports_a_change_a_new_entry_and_a_removal_and_says_nothing_for_unchanged()
    {
        let mut old = std::collections::BTreeMap::new();
        old.insert(
            "changed/A.exe".to_owned(),
            super::ratios::PinnedEntry {
                recovered: 1,
                declared: 10,
                ratio_text: "0.10".to_owned(),
                form_declared: 0,
                form_recovered: 0,
                control_declared: 0,
                control_recovered: 0,
            },
        );
        old.insert(
            "unchanged/B.exe".to_owned(),
            super::ratios::PinnedEntry {
                recovered: 2,
                declared: 10,
                ratio_text: "0.20".to_owned(),
                form_declared: 0,
                form_recovered: 0,
                control_declared: 0,
                control_recovered: 0,
            },
        );
        old.insert(
            "removed/C.exe".to_owned(),
            super::ratios::PinnedEntry {
                recovered: 3,
                declared: 10,
                ratio_text: "0.30".to_owned(),
                form_declared: 0,
                form_recovered: 0,
                control_declared: 0,
                control_recovered: 0,
            },
        );
        let measured = vec![
            Measured {
                key: "changed/A.exe".to_owned(),
                recovered: 2,
                declared: 10,
            },
            Measured {
                key: "unchanged/B.exe".to_owned(),
                recovered: 2,
                declared: 10,
            },
            Measured {
                key: "new/D.exe".to_owned(),
                recovered: 5,
                declared: 10,
            },
        ];
        let lines = describe_changes(&old, &measured);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("changed/A.exe") && l.contains("1 of 10 -> 2 of 10"))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("new/D.exe") && l.contains("new entry"))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("removed/C.exe") && l.contains("removed"))
        );
        assert!(
            !lines.iter().any(|l| l.contains("unchanged/B.exe")),
            "an unchanged key must produce no line: {lines:?}"
        );
    }

    #[test]
    fn help_exits_zero_and_an_unknown_or_missing_argument_exits_non_zero() {
        assert_eq!(run(vec!["--help".to_owned()]), 0);
        assert_eq!(run(vec!["-h".to_owned()]), 0);
        assert_ne!(run(vec!["bogus".to_owned()]), 0);
        assert_ne!(run(Vec::new()), 0);
    }

    // Behaviours one through three ("rewrites the file and reports the
    // count", "a clean-tree rerun produces no diff", "an edited entry is
    // restored and the change is reported") are deliberately NOT a #[test]
    // here: `run(["update-ratios"])` writes the real, committed
    // `tests/ratios.toml` in place, and `cargo test --workspace` runs
    // separate test binaries concurrently by default, so a #[test] here
    // that writes that file races `crates/deform6/tests/ratios.rs`'s own
    // tests reading it in a sibling process -- a real, not hypothetical,
    // source of flaky failures. The plan's own acceptance command,
    // `cargo run -p xtask -- update-ratios && git diff --exit-code --
    // tests/ratios.toml`, proves the same three behaviours the safe way:
    // sequentially, outside the test harness's own parallelism, exactly as
    // this plan's SUMMARY records having been run for real.
}
