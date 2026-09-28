//! `cargo run -p xtask -- export-lift-builds <dir>` writes each P-code
//! program of `corpus-pcode/` as `extract --lift` writes it, for a build with
//! the Visual Basic 6 IDE on a Windows host.
//!
//! The layout is the layout of `export-builds` (see `builds.rs`):
//! `extracted/pNN/` holds the project that DeForm6 writes with the lifted
//! bodies, `original/pNN/` holds the project of the source, and `build.bat`
//! builds each side. The logs tell which lifted project compiles, and the
//! first error of each one that does not.
//!
//! The P-code table and the types file are not in the repository. The
//! command reads them from `derived/`, or from `--pcode-table` and
//! `--vb-types`.

use std::path::Path;

use deform6::journal::Mode;
use deform6::vb::opcodes::OpcodeTable;
use deform6::vb::pcode::PcodeTable;
use deform6::vb::types::VbTypes;

use crate::build_record::{self, corpus_root, executables, program_key};
use crate::builds::{
    Exported, check_batch_name, copy_without_executables, prepare, short_name, write_files,
    write_lists,
};
use crate::pcode_record::pcode_root;
use crate::ratios::differential::support::vbp;

/// The options of `export-lift-builds`.
struct Options {
    table: String,
    types: String,
    dir: String,
}

/// Reads `[--pcode-table <file>] [--vb-types <file>] <dir>`.
fn options(args: &[String]) -> Option<Options> {
    let mut table = crate::pcode_table::DEFAULT_OUTPUT_PATH.to_owned();
    let mut types = crate::vb_types::DEFAULT_OUTPUT_PATH.to_owned();
    let mut dir = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--pcode-table" => table.clone_from(rest.next()?),
            "--vb-types" => types.clone_from(rest.next()?),
            flag if flag.starts_with('-') => return None,
            path if dir.is_none() => dir = Some(path.to_owned()),
            _ => return None,
        }
    }
    Some(Options {
        table,
        types,
        dir: dir?,
    })
}

/// Gives the files that `extract --lift` writes for the program `bytes`,
/// through [`build_record::build_files`].
fn lifted_files(
    bytes: &[u8],
    table: &PcodeTable,
    types: &VbTypes,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut report =
        deform6::inspect_with_types(bytes, &OpcodeTable::builtin(), Some(types), Mode::Strict)
            .map_err(|err| format!("inspect: {err}"))?;
    let lifted = deform6::vb::bodies::lift_objects(bytes, &report, table, Some(types))
        .map_err(|err| format!("lift: {err}"))?;
    for (object, lifted) in report.objects.iter_mut().zip(lifted) {
        object.lifted = lifted;
    }
    let written = deform6::write::project(&report, bytes, Mode::Strict)
        .map_err(|err| format!("write::project: {err}"))?;
    Ok(build_record::build_files(
        written
            .files
            .into_iter()
            .map(|file| (file.name, file.bytes))
            .collect(),
    ))
}

/// Writes the whole export into `dir`, and gives the number of programs.
fn export(options: &Options) -> Result<usize, String> {
    let read = |path: &str| std::fs::read(path).map_err(|err| format!("reading {path}: {err}"));
    let table = PcodeTable::parse(&read(&options.table)?)
        .map_err(|err| format!("{}: {err}", options.table))?;
    let types = VbTypes::parse(&read(&options.types)?)
        .map_err(|err| format!("{}: {err}", options.types))?;
    let dir = Path::new(&options.dir);
    prepare(dir)?;
    let root = pcode_root();
    let corpus = corpus_root();
    let projects = vbp::project_files();
    let mut exported = Vec::new();
    for (index, exe) in executables(&root)?.iter().enumerate() {
        let short = short_name(index.checked_add(1).ok_or("too many programs")?)?;
        let key = program_key(exe, &root)?;
        let bytes =
            std::fs::read(exe).map_err(|err| format!("reading {}: {err}", exe.display()))?;
        let files = lifted_files(&bytes, &table, &types).map_err(|err| format!("{key}: {err}"))?;
        write_files(&dir.join("extracted").join(&short), &files)
            .map_err(|err| format!("{key}: {err}"))?;
        let extracted_vbp = files
            .iter()
            .map(|(name, _)| name)
            .find(|name| {
                Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("vbp"))
            })
            .cloned()
            .ok_or_else(|| format!("{key}: DeForm6 wrote no project file"))?;
        let original = vbp::select_project_file(&corpus.join(&key), &projects)
            .map_err(|err| format!("{key}: no project file: {err:?}"))?;
        let original_dir = original
            .parent()
            .ok_or_else(|| format!("{key}: the project file has no directory"))?;
        copy_without_executables(original_dir, &dir.join("original").join(&short))
            .map_err(|err| format!("{key}: {err}"))?;
        let original_vbp = original
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| format!("{key}: the project file has no name"))?;
        check_batch_name(&original_vbp).map_err(|err| format!("{key}: {err}"))?;
        exported.push(Exported {
            short,
            key,
            original_vbp,
            extracted_vbp,
            files: build_record::files_hash(&files),
        });
    }
    write_lists(dir, &exported)?;
    Ok(exported.len())
}

/// Runs `export-lift-builds`.
pub(crate) fn run(args: &[String]) -> i32 {
    let Some(options) = options(args) else {
        eprintln!(
            "usage: cargo run -p xtask -- export-lift-builds [--pcode-table <file>] \
             [--vb-types <file>] <dir>"
        );
        return 1;
    };
    match export(&options) {
        Ok(count) => {
            println!(
                "xtask: exported {count} lifted programs to {}. Run build.bat there on the \
                 Windows host.",
                options.dir
            );
            0
        }
        Err(err) => {
            eprintln!("xtask: {err}");
            1
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::options;

    #[test]
    fn the_options_take_two_files_and_one_directory() {
        let args = |text: &str| -> Vec<String> { text.split(' ').map(str::to_owned).collect() };
        let read = options(&args("--pcode-table t.toml --vb-types v.toml out")).unwrap();
        assert_eq!(
            (read.table.as_str(), read.types.as_str(), read.dir.as_str()),
            ("t.toml", "v.toml", "out")
        );
        let default = options(&args("out")).unwrap();
        assert_eq!(default.table, "derived/pcode-table.toml");
        assert!(options(&args("out other")).is_none());
        assert!(options(&args("--lift")).is_none());
        assert!(options(&args("--vb-types")).is_none());
    }
}
