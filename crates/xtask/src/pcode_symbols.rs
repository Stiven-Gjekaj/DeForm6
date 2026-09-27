//! `fetch-pcode-symbols`: fetches the symbol file that matches a copy of
//! `MSVBVM60.DLL`, from the symbol server of Microsoft.
//!
//! The DLL names its symbol file in a CodeView record of its debug
//! directory. The runtime of the XP host holds an `NB10` record: the name
//! `MSVBVM60.pdb`, a four-byte signature and an age. The server keeps the
//! file at `<name>/<SIGNATURE><AGE>/<name>`, with both values in upper case
//! hexadecimal.
//!
//! The file that comes back is checked before it is written: it must be a
//! program database 2.00 whose info stream (stream 1) holds the same
//! signature and the same age. The file is written below `derived/`, which
//! `.gitignore` excludes. It never enters the repository.

use object::pe::IMAGE_DIRECTORY_ENTRY_DEBUG;

use crate::pdb2::{Pdb2, u32_at};

/// The symbol server.
const SERVER: &str = "https://msdl.microsoft.com/download/symbols";

/// The directory that the fetched file goes into, relative to the workspace
/// root.
const DEFAULT_DIR: &str = "derived/symbols";

/// The length of one entry of the debug directory.
const DEBUG_ENTRY_LEN: usize = 28;

/// The type of a debug entry that holds a CodeView record.
const CODEVIEW: u32 = 2;

/// The symbol file that a DLL names: its name, and the signature and the age
/// that an `NB10` record gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SymbolId {
    /// The name of the symbol file.
    pub(crate) name: String,
    /// The signature, a time stamp.
    pub(crate) signature: u32,
    /// The age.
    pub(crate) age: u32,
}

impl SymbolId {
    /// The address of the file on the symbol server.
    pub(crate) fn url(&self) -> String {
        format!(
            "{SERVER}/{name}/{signature:08X}{age:X}/{name}",
            name = self.name,
            signature = self.signature,
            age = self.age
        )
    }
}

/// Reads a CodeView record.
///
/// # Errors
///
/// Gives an error for a record that is not `NB10`. An `RSDS` record names a
/// program database 7.00, which `xtask` does not read.
pub(crate) fn codeview_id(record: &[u8]) -> Result<SymbolId, String> {
    match record.get(..4) {
        Some(b"NB10") => {
            let signature = u32_at(record, 8).ok_or("the NB10 record is cut")?;
            let age = u32_at(record, 12).ok_or("the NB10 record is cut")?;
            let name = record.get(16..).ok_or("the NB10 record is cut")?;
            let name = name.split(|byte| *byte == 0).next().unwrap_or_default();
            if name.is_empty() || name.iter().any(|byte| !byte.is_ascii_graphic()) {
                return Err("the NB10 record names no symbol file".to_owned());
            }
            Ok(SymbolId {
                name: String::from_utf8_lossy(name).into_owned(),
                signature,
                age,
            })
        }
        Some(b"RSDS") => Err(
            "the DLL names a program database 7.00, and xtask reads only the format 2.00"
                .to_owned(),
        ),
        _ => Err("the CodeView record has no known signature".to_owned()),
    }
}

/// Finds the CodeView record in the debug directory of a 32-bit PE image.
///
/// # Errors
///
/// Gives an error when the file is not a 32-bit PE image, has no debug
/// directory, or has no CodeView record.
pub(crate) fn symbol_id(dll: &[u8]) -> Result<SymbolId, String> {
    let file = object::read::pe::PeFile32::parse(dll)
        .map_err(|err| format!("the DLL is not a 32-bit PE image: {err}"))?;
    let directory = file
        .data_directory(IMAGE_DIRECTORY_ENTRY_DEBUG)
        .ok_or("the DLL has no debug directory")?
        .data(dll, &file.section_table())
        .map_err(|err| format!("the debug directory cannot be read: {err}"))?;
    for entry in directory.chunks_exact(DEBUG_ENTRY_LEN) {
        if u32_at(entry, 12) != Some(CODEVIEW) {
            continue;
        }
        let size = u32_at(entry, 16).ok_or("a debug entry is cut")?;
        let at = u32_at(entry, 24).ok_or("a debug entry is cut")?;
        let start = usize::try_from(at).map_err(|_| "a debug offset does not fit")?;
        let end = usize::try_from(size)
            .ok()
            .and_then(|size| start.checked_add(size))
            .ok_or("a debug size overflows")?;
        let record = dll
            .get(start..end)
            .ok_or("the CodeView record runs past the end of the DLL")?;
        return codeview_id(record);
    }
    Err("the DLL has no CodeView record".to_owned())
}

/// Checks that `pdb` is a program database 2.00 whose info stream holds the
/// signature and the age of `id`.
///
/// # Errors
///
/// Gives an error that names the two values when they differ.
pub(crate) fn check(pdb: &[u8], id: &SymbolId) -> Result<(), String> {
    let parsed = Pdb2::parse(pdb)?;
    let info = parsed
        .stream(1)
        .ok_or("the symbol file has no info stream")?;
    let found = (
        u32_at(info, 4).ok_or("the info stream is cut")?,
        u32_at(info, 8).ok_or("the info stream is cut")?,
    );
    if found != (id.signature, id.age) {
        return Err(format!(
            "the symbol file has signature {:08X} and age {}, and the DLL names {:08X} and {}",
            found.0, found.1, id.signature, id.age
        ));
    }
    Ok(())
}

/// Runs `fetch-pcode-symbols <dll> [--out <dir>]`.
pub(crate) fn run(args: &[String]) -> i32 {
    let (dll, dir) = match args {
        [dll] => (dll, DEFAULT_DIR),
        [dll, flag, dir] if flag == "--out" => (dll, dir.as_str()),
        _ => {
            eprintln!("usage: cargo run -p xtask -- fetch-pcode-symbols <dll> [--out <dir>]");
            return 1;
        }
    };
    match fetch(dll, dir) {
        Ok(path) => {
            println!("wrote {path}");
            0
        }
        Err(err) => {
            eprintln!("fetch-pcode-symbols: {err}");
            1
        }
    }
}

/// Reads the DLL, fetches its symbol file, checks it and writes it into
/// `dir`.
fn fetch(dll: &str, dir: &str) -> Result<String, String> {
    let bytes = std::fs::read(dll).map_err(|err| format!("reading {dll}: {err}"))?;
    let id = symbol_id(&bytes)?;
    let url = id.url();
    let mut response = ureq::get(&url)
        .header("User-Agent", "Microsoft-Symbol-Server/10.0.0.0")
        .call()
        .map_err(|err| format!("{url}: {err}"))?;
    let pdb = response
        .body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_vec()
        .map_err(|err| format!("{url}: {err}"))?;
    check(&pdb, &id)?;
    std::fs::create_dir_all(dir).map_err(|err| format!("creating {dir}: {err}"))?;
    let path = std::path::Path::new(dir).join(&id.name);
    std::fs::write(&path, &pdb).map_err(|err| format!("writing {}: {err}", path.display()))?;
    Ok(path.display().to_string())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{SymbolId, check, codeview_id};
    use crate::pdb2::tests::pdb;

    /// The NB10 record of the runtime of the XP host, with its values built
    /// here.
    fn nb10(signature: u32, age: u32, name: &[u8]) -> Vec<u8> {
        let mut record = b"NB10".to_vec();
        record.extend_from_slice(&0_u32.to_le_bytes());
        record.extend_from_slice(&signature.to_le_bytes());
        record.extend_from_slice(&age.to_le_bytes());
        record.extend_from_slice(name);
        record.push(0);
        record
    }

    #[test]
    fn an_nb10_record_gives_the_address_on_the_symbol_server() {
        let id = codeview_id(&nb10(0x4719_3E36, 1, b"MSVBVM60.pdb")).unwrap();
        assert_eq!(
            id,
            SymbolId {
                name: "MSVBVM60.pdb".to_owned(),
                signature: 0x4719_3E36,
                age: 1,
            }
        );
        assert_eq!(
            id.url(),
            "https://msdl.microsoft.com/download/symbols/MSVBVM60.pdb/47193E361/MSVBVM60.pdb"
        );
    }

    #[test]
    fn a_record_of_another_format_or_with_no_name_is_refused() {
        assert!(
            codeview_id(b"RSDS0123456789abcdef")
                .unwrap_err()
                .contains("7.00")
        );
        assert!(codeview_id(b"XXXX").is_err());
        assert!(codeview_id(&nb10(1, 1, b"")).is_err());
        assert!(codeview_id(&nb10(1, 1, b"a/b c")).is_err());
    }

    #[test]
    fn a_symbol_file_is_accepted_only_with_the_signature_and_the_age_of_the_dll() {
        let mut info = 0x0130_BA2C_u32.to_le_bytes().to_vec();
        info.extend_from_slice(&0x4719_3E36_u32.to_le_bytes());
        info.extend_from_slice(&1_u32.to_le_bytes());
        let file = pdb(&[b"", &info]);
        let id = |signature, age| SymbolId {
            name: "MSVBVM60.pdb".to_owned(),
            signature,
            age,
        };
        assert!(check(&file, &id(0x4719_3E36, 1)).is_ok());
        let err = check(&file, &id(0x4719_3E37, 1)).unwrap_err();
        assert!(err.contains("47193E36"), "{err}");
        assert!(check(&file, &id(0x4719_3E36, 2)).is_err());
    }
}
