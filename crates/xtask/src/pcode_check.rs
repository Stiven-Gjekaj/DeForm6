//! `check-pcode-table`: decodes each P-code body of `corpus-pcode/` with a
//! table that `derive-pcode-table` wrote, and counts the bodies that decode
//! to their end.
//!
//! The table is not in the repository, so no test can hold it. This command
//! is the check that the user runs on the table that they derive. It decodes
//! with [`deform6::vb::pcode::disassemble`], whose doc comment gives the
//! rules, and it names each body that does not decode to its end. It exits
//! with 1 when one body does not. It also counts the bodies that
//! [`deform6::vb::lift::lift`] lifts to statements; that count only reports,
//! and it does not fail the command.

use deform6::read::pe::PeImage;
use deform6::vb::header::{VbHeader, header_region};
use deform6::vb::lift::lift;
use deform6::vb::object::ObjectTable;
use deform6::vb::pcode::{PcodeTable, disassemble};
use deform6::vb::procdesc::read_method_table;
use deform6::vb::project::{ObjectTableHead, ProjectInfo};

use crate::build_record;
use crate::pcode_record;
use crate::pcode_table::DEFAULT_OUTPUT_PATH;

/// Runs `check-pcode-table [<table>]`.
pub(crate) fn run(args: &[String]) -> i32 {
    let path = match args {
        [] => DEFAULT_OUTPUT_PATH,
        [path] => path.as_str(),
        _ => {
            eprintln!("usage: cargo run -p xtask -- check-pcode-table [<table>]");
            return 1;
        }
    };
    match check(path) {
        Ok((bodies, failures, lifted)) => {
            for failure in &failures {
                println!("{failure}");
            }
            let decoded = bodies.saturating_sub(failures.len());
            println!("{decoded} of {bodies} P-code bodies decode to their end");
            println!("{lifted} of {bodies} P-code bodies lift to statements");
            i32::from(!failures.is_empty())
        }
        Err(err) => {
            eprintln!("check-pcode-table: {err}");
            1
        }
    }
}

/// Decodes each body of each program in `corpus-pcode/`, and gives the
/// number of bodies and a line for each body that does not decode.
fn check(path: &str) -> Result<(usize, Vec<String>, usize), String> {
    let bytes = std::fs::read(path).map_err(|err| format!("reading {path}: {err}"))?;
    let table = PcodeTable::parse(&bytes).map_err(|err| format!("{path}: {err}"))?;
    let root = pcode_record::pcode_root();
    let mut bodies = 0_usize;
    let mut lifted = 0_usize;
    let mut failures = Vec::new();
    for exe in build_record::executables(&root)? {
        let key = build_record::program_key(&exe, &root)?;
        let bytes = std::fs::read(&exe).map_err(|err| format!("reading {key}: {err}"))?;
        let pe = PeImage::parse(&bytes).map_err(|err| format!("{key}: {err}"))?;
        let header = VbHeader::read(&header_region(&pe).map_err(|err| format!("{key}: {err}"))?)
            .map_err(|err| format!("{key}: {err}"))?;
        let info = ProjectInfo::read(&pe, header.lp_project_data)
            .map_err(|err| format!("{key}: {err}"))?;
        let head = ObjectTableHead::read(&pe, info.lp_object_table)
            .map_err(|err| format!("{key}: {err}"))?;
        let objects = ObjectTable::walk(&pe, info.lp_object_table, &head)
            .map_err(|err| format!("{key}: {err}"))?;
        for object in &objects.objects {
            let methods = read_method_table(&pe, object.lp_object_info)
                .map_err(|err| format!("{key}: {}: {err}", object.name))?;
            for descriptor in methods.descriptors() {
                bodies = bodies.saturating_add(1);
                let body = descriptor
                    .body(&pe)
                    .ok_or_else(|| format!("{key}: a body cannot be read"))?;
                let listing = disassemble(&body, &table);
                if lift(&listing, &table).is_ok() {
                    lifted = lifted.saturating_add(1);
                }
                if !listing.end.is_complete() {
                    failures.push(format!(
                        "{key}: {} descriptor {:#x}: {:?}",
                        object.name,
                        descriptor.va.get(),
                        listing.end
                    ));
                }
            }
        }
    }
    Ok((bodies, failures, lifted))
}
