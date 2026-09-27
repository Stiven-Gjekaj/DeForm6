//! `derive-pcode-table`: writes the P-code dispatch table of the Visual
//! Basic 6 runtime, with the name of each handler, from a copy of
//! `MSVBVM60.DLL` that the user owns and the symbol file that Microsoft
//! serves for it.
//!
//! # What the runtime holds
//!
//! The runtime runs P-code with a dispatch table: one entry for each opcode
//! byte, each the address of a handler. The public symbol `_tblByteDisp`
//! names the primary table of 256 entries. Five of its entries, for the
//! bytes `0xFB` to `0xFF`, go to the handlers `_lblBEX_Lead0` to
//! `_lblBEX_Lead4`. Each of these reads the next byte and jumps through a
//! table of its own: `xor eax,eax / mov al,[esi] / inc esi`, then
//! `jmp [eax*4+table]`. A lead handler can check the byte first, with
//! `cmp eax,imm8 / ja`, and its table then holds `imm8 + 1` entries.
//!
//! The name of a handler is a public symbol `_lblEX_<name>`. One address can
//! have many names, because many opcodes that do nothing share one handler.
//!
//! # The two layouts
//!
//! Microsoft reordered the runtime after the link. The symbol file gives
//! each public symbol at a segment and an offset of the layout before the
//! reorder, and its OMAP maps that layout to the layout of the file. The base
//! of each segment in the source layout is not in the symbol file. This tool
//! finds it from the exports of the DLL: an export and the public symbol of
//! the same name give the base, and all of them must give the same base.
//!
//! # What may enter the repository
//!
//! This tool is committed. The DLL, the symbol file and the table that this
//! tool writes are not: `.gitignore` excludes [`DEFAULT_OUTPUT_PATH`].

use std::collections::{BTreeMap, BTreeSet};

use object::{Object, ObjectSection};

use crate::pdb2::{Dbi, Omap, Pdb2, Public, publics, u32_at};

/// The default output path, relative to the workspace root.
pub(crate) const DEFAULT_OUTPUT_PATH: &str = "derived/pcode-table.toml";

/// The public symbol of the primary dispatch table.
const PRIMARY_TABLE: &str = "_tblByteDisp";

/// The prefix of the public symbol of a handler.
const HANDLER_PREFIX: &str = "_lblEX_";

/// The prefix of the public symbol of a lead handler, before its number.
const LEAD_PREFIX: &str = "_lblBEX_Lead";

/// The first opcode byte that is a lead byte.
const FIRST_LEAD: u8 = 0xFB;

/// The bytes that open a lead handler: `xor eax,eax / mov al,[esi] / inc
/// esi`.
const LEAD_OPEN: [u8; 5] = [0x33, 0xC0, 0x8A, 0x06, 0x46];

/// The bytes of `jmp [eax*4+imm32]`, before the address of the table.
const JUMP_THROUGH_TABLE: [u8; 3] = [0xFF, 0x24, 0x85];

/// The length of a lead handler with no check: [`LEAD_OPEN`], then the jump
/// through the table with its four-byte address.
const LEAD_SHORT: usize = 12;

/// The length of a lead handler with a check: [`LEAD_OPEN`], `cmp eax,imm8`
/// and `ja rel8` in five bytes, then the jump through the table.
const LEAD_LONG: usize = 17;

/// The image of the DLL: its base, its sections and its exports.
#[derive(Debug, Default)]
pub(crate) struct Image {
    /// The image base.
    pub(crate) base: u32,
    /// Each section: its address relative to the base, and its bytes.
    pub(crate) sections: Vec<(u32, Vec<u8>)>,
    /// Each export by name, with its address relative to the base.
    pub(crate) exports: BTreeMap<String, u32>,
}

impl Image {
    /// Reads a 32-bit PE image with the `object` crate.
    ///
    /// # Errors
    ///
    /// Gives an error when the file is not a 32-bit PE image, or when a
    /// section or an export cannot be read.
    pub(crate) fn from_pe(dll: &[u8]) -> Result<Self, String> {
        let file = object::read::pe::PeFile32::parse(dll)
            .map_err(|err| format!("the DLL is not a 32-bit PE image: {err}"))?;
        let base = u32::try_from(file.relative_address_base())
            .map_err(|_| "the image base does not fit a u32".to_owned())?;
        let rva = |address: u64| {
            u32::try_from(address)
                .ok()
                .and_then(|address| address.checked_sub(base))
                .ok_or_else(|| format!("the address {address:#x} is not in the image"))
        };
        let mut sections = Vec::new();
        for section in file.sections() {
            let data = section
                .data()
                .map_err(|err| format!("a section cannot be read: {err}"))?;
            sections.push((rva(section.address())?, data.to_vec()));
        }
        let mut exports = BTreeMap::new();
        for export in file
            .exports()
            .map_err(|err| format!("the exports cannot be read: {err}"))?
        {
            let export = export.map_err(|err| format!("an export cannot be read: {err}"))?;
            let (object::NameOrOrdinal::Name(name), object::ExportTarget::Address { address }) =
                (export.name(), export.target())
            else {
                continue;
            };
            exports.insert(String::from_utf8_lossy(name).into_owned(), rva(address)?);
        }
        Ok(Self {
            base,
            sections,
            exports,
        })
    }

    /// Gives `len` bytes at `rva`, when one section holds all of them.
    fn bytes(&self, rva: u32, len: usize) -> Option<&[u8]> {
        self.sections.iter().find_map(|(start, data)| {
            let at = usize::try_from(rva.checked_sub(*start)?).ok()?;
            data.get(at..at.checked_add(len)?)
        })
    }

    /// Reads a table entry, an absolute address, at `rva`, and gives the
    /// address that it holds relative to the base.
    fn entry(&self, rva: u32) -> Option<u32> {
        u32_at(self.bytes(rva, 4)?, 0)?.checked_sub(self.base)
    }
}

/// What the symbol file gives: the public symbols and the two OMAPs.
#[derive(Debug, Default)]
pub(crate) struct Symbols {
    /// Each public symbol.
    pub(crate) publics: Vec<Public>,
    /// The OMAP from the layout of the file to the source layout.
    pub(crate) to_src: Omap,
    /// The OMAP from the source layout to the layout of the file.
    pub(crate) from_src: Omap,
}

impl Symbols {
    /// Reads the public symbols and the two OMAPs of a program database 2.00.
    ///
    /// # Errors
    ///
    /// Gives an error when the file is not a program database 2.00, or when
    /// it has no OMAP.
    pub(crate) fn from_pdb(file: &[u8]) -> Result<Self, String> {
        let pdb = Pdb2::parse(file)?;
        let dbi = Dbi::parse(pdb.stream(3).ok_or("the file has no DBI stream")?)?;
        let stream = |index: Option<u16>, what: &str| {
            index
                .and_then(|index| pdb.stream(index))
                .ok_or_else(|| format!("the file has no {what}"))
        };
        Ok(Self {
            publics: publics(stream(Some(dbi.sym_records), "symbol record stream")?)?,
            to_src: Omap::parse(stream(dbi.omap_to_src, "OMAP to the source")?)?,
            from_src: Omap::parse(stream(dbi.omap_from_src, "OMAP from the source")?)?,
        })
    }
}

/// The table that holds a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Table {
    /// The primary table, `_tblByteDisp`.
    Primary,
    /// The table of a lead byte: 0 for `0xFB`, up to 4 for `0xFF`.
    Lead(u8),
}

impl Table {
    /// The name of the table in the file that this tool writes.
    fn key(self) -> String {
        match self {
            Self::Primary => "primary".to_owned(),
            Self::Lead(lead) => format!("lead{lead}"),
        }
    }
}

/// One slot of a dispatch table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Slot {
    /// The table that holds the slot.
    pub(crate) table: Table,
    /// The opcode byte, which is the index of the slot.
    pub(crate) opcode: u8,
    /// The address of the handler, relative to the image base.
    pub(crate) handler: u32,
    /// Each name of the handler, without `_lblEX_`, sorted. Empty when no
    /// public symbol names the address.
    pub(crate) names: Vec<String>,
}

/// Gives the name of the export that a public symbol names: the name
/// without one leading `_` and without a trailing `@<digits>`.
fn export_name(public: &str) -> &str {
    let name = public.strip_prefix('_').unwrap_or(public);
    match name.rsplit_once('@') {
        Some((head, tail)) if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) => head,
        _ => name,
    }
}

/// Finds the base of each segment in the source layout, from the exports.
///
/// # Errors
///
/// Gives an error when two exports give two bases for one segment.
fn segment_bases(image: &Image, symbols: &Symbols) -> Result<BTreeMap<u16, u32>, String> {
    let mut found: BTreeMap<u16, BTreeSet<u32>> = BTreeMap::new();
    for public in &symbols.publics {
        let Some(&rva) = image.exports.get(export_name(&public.name)) else {
            continue;
        };
        let Some(base) = symbols
            .to_src
            .translate(rva)
            .and_then(|source| source.checked_sub(public.offset))
        else {
            continue;
        };
        found.entry(public.segment).or_default().insert(base);
    }
    let mut out = BTreeMap::new();
    for (segment, bases) in found {
        let mut bases = bases.into_iter();
        let (Some(base), None) = (bases.next(), bases.next()) else {
            return Err(format!(
                "the exports give more than one base for segment {segment}"
            ));
        };
        out.insert(segment, base);
    }
    Ok(out)
}

/// Gives the address of a public symbol in the layout of the file.
fn address(symbols: &Symbols, bases: &BTreeMap<u16, u32>, public: &Public) -> Option<u32> {
    symbols
        .from_src
        .translate(bases.get(&public.segment)?.checked_add(public.offset)?)
}

/// Reads the code of a lead handler at `rva`, and gives the address of its
/// table relative to the base and the number of entries of the table.
///
/// # Errors
///
/// Gives an error when the code is not one of the two measured shapes.
fn lead_table(image: &Image, rva: u32) -> Result<(u32, u16), String> {
    let code = image
        .bytes(rva, LEAD_LONG)
        .or_else(|| image.bytes(rva, LEAD_SHORT))
        .ok_or_else(|| format!("the lead handler at {rva:#x} is not in a section"))?;
    let refuse =
        || format!("the lead handler at {rva:#x} holds {code:02x?}, which is no known shape");
    let rest = code.strip_prefix(&LEAD_OPEN[..]).ok_or_else(refuse)?;
    let (count, jump) = match rest {
        [0x83, 0xF8, last, 0x77, _, jump @ ..] => {
            (u16::from(*last).checked_add(1).ok_or_else(refuse)?, jump)
        }
        jump => (256, jump),
    };
    let table = jump
        .strip_prefix(&JUMP_THROUGH_TABLE[..])
        .and_then(|address| u32_at(address, 0))
        .and_then(|address| address.checked_sub(image.base))
        .ok_or_else(refuse)?;
    Ok((table, count))
}

/// Derives each slot of the primary table and of the five lead tables.
///
/// # Errors
///
/// Gives an error when the symbol file does not name the primary table or
/// a lead handler, when a lead byte does not go to its lead handler, and
/// when a table runs past its section.
pub(crate) fn derive(image: &Image, symbols: &Symbols) -> Result<Vec<Slot>, String> {
    let bases = segment_bases(image, symbols)?;
    let mut names: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut named = BTreeMap::new();
    for public in &symbols.publics {
        let Some(at) = address(symbols, &bases, public) else {
            continue;
        };
        if let Some(name) = public.name.strip_prefix(HANDLER_PREFIX) {
            names.entry(at).or_default().insert(name.to_owned());
        }
        named.insert(public.name.as_str(), at);
    }
    let symbol = |name: &str| {
        named
            .get(name)
            .copied()
            .ok_or_else(|| format!("the symbol file does not place {name}"))
    };
    let slot = |table: Table, opcode: u8, handler: u32| Slot {
        table,
        opcode,
        handler,
        names: names
            .get(&handler)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default(),
    };
    let read_table = |start: u32, count: u16, table: Table| -> Result<Vec<Slot>, String> {
        let mut out = Vec::new();
        for opcode in 0..count {
            let opcode = u8::try_from(opcode).map_err(|_| "a table holds more than 256 entries")?;
            let at = u32::from(opcode)
                .checked_mul(4)
                .and_then(|off| start.checked_add(off))
                .ok_or("a table offset overflows")?;
            let handler = image
                .entry(at)
                .ok_or_else(|| format!("the {} table ends at entry {opcode:#04x}", table.key()))?;
            out.push(slot(table, opcode, handler));
        }
        Ok(out)
    };

    let mut slots = read_table(symbol(PRIMARY_TABLE)?, 256, Table::Primary)?;
    let leads: Vec<(u8, u32)> = slots
        .iter()
        .filter(|slot| slot.opcode >= FIRST_LEAD)
        .map(|slot| (slot.opcode, slot.handler))
        .collect();
    for (opcode, handler) in leads {
        let lead = opcode
            .checked_sub(FIRST_LEAD)
            .ok_or("a lead byte is below 0xFB")?;
        let expected = symbol(&format!("{LEAD_PREFIX}{lead}"))?;
        if handler != expected {
            return Err(format!(
                "the lead byte {opcode:#04x} goes to {handler:#x}, and {LEAD_PREFIX}{lead} is at \
                 {expected:#x}"
            ));
        }
        let (start, count) = lead_table(image, handler)?;
        slots.extend(read_table(start, count, Table::Lead(lead))?);
    }
    Ok(slots)
}

/// One row of the written file.
#[derive(serde::Serialize)]
struct Row<'a> {
    handler: String,
    names: &'a [String],
}

/// Renders the slots as TOML: one table for each dispatch table, keyed by
/// the opcode as two hexadecimal digits.
pub(crate) fn render(slots: &[Slot]) -> Result<String, String> {
    let mut tables: BTreeMap<String, BTreeMap<String, Row<'_>>> = BTreeMap::new();
    for slot in slots {
        tables.entry(slot.table.key()).or_default().insert(
            format!("{:02X}", slot.opcode),
            Row {
                handler: format!("{:#010x}", slot.handler),
                names: &slot.names,
            },
        );
    }
    let body =
        toml::to_string(&tables).map_err(|err| format!("the table cannot be written: {err}"))?;
    Ok(format!(
        "# The P-code dispatch table of MSVBVM60.DLL, written by\n# `cargo run -p xtask -- \
         derive-pcode-table`. Do not commit this file.\n#\n# `handler` is the address of the \
         handler relative to the image base.\n# `names` are the public symbols of the handler \
         without `_lblEX_`.\n\n{body}"
    ))
}

/// Runs `derive-pcode-table <dll> <pdb> [--out <path>]`.
pub(crate) fn run(args: &[String]) -> i32 {
    let (dll, pdb, out) = match args {
        [dll, pdb] => (dll, pdb, DEFAULT_OUTPUT_PATH),
        [dll, pdb, flag, out] if flag == "--out" => (dll, pdb, out.as_str()),
        _ => {
            eprintln!("usage: cargo run -p xtask -- derive-pcode-table <dll> <pdb> [--out <path>]");
            return 1;
        }
    };
    match derive_files(dll, pdb, out) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(err) => {
            eprintln!("derive-pcode-table: {err}");
            1
        }
    }
}

/// Reads the two files, derives the table and writes it to `out`.
fn derive_files(dll: &str, pdb: &str, out: &str) -> Result<String, String> {
    let read = |path: &str| std::fs::read(path).map_err(|err| format!("reading {path}: {err}"));
    let image = Image::from_pe(&read(dll)?)?;
    let symbols = Symbols::from_pdb(&read(pdb)?)?;
    let slots = derive(&image, &symbols)?;
    if let Some(parent) = std::path::Path::new(out).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("creating {}: {err}", parent.display()))?;
    }
    std::fs::write(out, render(&slots)?).map_err(|err| format!("writing {out}: {err}"))?;
    let named = slots.iter().filter(|slot| !slot.names.is_empty()).count();
    let handlers: BTreeSet<u32> = slots.iter().map(|slot| slot.handler).collect();
    Ok(format!(
        "wrote {out}: {} slots, {named} with a name, {} different handlers",
        slots.len(),
        handlers.len()
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Image, Slot, Symbols, Table, derive, export_name, lead_table, render};
    use crate::pdb2::{Omap, Public};

    const BASE: u32 = 0x7342_0000;

    /// An OMAP of one pair: `from` maps to `to`.
    fn omap(from: u32, to: u32) -> Omap {
        let mut stream = from.to_le_bytes().to_vec();
        stream.extend_from_slice(&to.to_le_bytes());
        Omap::parse(&stream).unwrap()
    }

    fn public(name: &str, offset: u32) -> Public {
        Public {
            name: name.to_owned(),
            segment: 2,
            offset,
        }
    }

    /// A lead handler that reads a table at `table` of `last + 1` entries,
    /// or of 256 when `last` is `None`.
    fn lead_code(table: u32, last: Option<u8>) -> Vec<u8> {
        let mut code = vec![0x33, 0xC0, 0x8A, 0x06, 0x46];
        if let Some(last) = last {
            code.extend_from_slice(&[0x83, 0xF8, last, 0x77, 0x10]);
        }
        code.extend_from_slice(&[0xFF, 0x24, 0x85]);
        code.extend_from_slice(&(BASE + table).to_le_bytes());
        code
    }

    /// An image with one section at `0x1000`, and symbols whose source
    /// layout is the same layout moved up by `0x100`. Segment 2 starts at
    /// `0x1100` in the source layout, and the export `Engine` gives it.
    ///
    /// The primary table is at `0x1000`. Entry 0 goes to `0x1800`, which is
    /// named `Bos` and `LargeBos`. Each other entry below `0xFB` goes to
    /// `0x1804`, which has no name. The five lead bytes go to lead handlers
    /// at `0x1900` + 0x20 * n. Lead 0 has a table of 256 entries at
    /// `0x1400`, and leads 1 to 4 have tables of 2 entries at `0x1800`.
    fn fixture() -> (Image, Symbols) {
        let mut section = vec![0_u8; 0x1000];
        let put = |section: &mut Vec<u8>, rva: u32, bytes: &[u8]| {
            let at = usize::try_from(rva - 0x1000).unwrap();
            section[at..at + bytes.len()].copy_from_slice(bytes);
        };
        for opcode in 0..256_u32 {
            let target = match opcode {
                0 => 0x1800,
                0xFB..=0xFF => 0x1900 + 0x20 * (opcode - 0xFB),
                _ => 0x1804,
            };
            put(
                &mut section,
                0x1000 + 4 * opcode,
                &(BASE + target).to_le_bytes(),
            );
            put(
                &mut section,
                0x1400 + 4 * opcode,
                &(BASE + 0x1800).to_le_bytes(),
            );
        }
        put(&mut section, 0x1800, &(BASE + 0x1800).to_le_bytes());
        put(&mut section, 0x1804, &(BASE + 0x1804).to_le_bytes());
        for lead in 0..5_u32 {
            let code = if lead == 0 {
                lead_code(0x1400, None)
            } else {
                lead_code(0x1800, Some(1))
            };
            put(&mut section, 0x1900 + 0x20 * lead, &code);
        }
        let mut exports = BTreeMap::new();
        exports.insert("Engine".to_owned(), 0x1000);
        let image = Image {
            base: BASE,
            sections: vec![(0x1000, section)],
            exports,
        };
        let mut publics = vec![
            public("_Engine@0", 0),
            public("_tblByteDisp", 0),
            public("_lblEX_Bos", 0x800),
            public("_lblEX_LargeBos", 0x800),
        ];
        for lead in 0..5_u32 {
            publics.push(public(&format!("_lblBEX_Lead{lead}"), 0x900 + 0x20 * lead));
        }
        let symbols = Symbols {
            publics,
            to_src: omap(0x1000, 0x1100),
            from_src: omap(0x1100, 0x1000),
        };
        (image, symbols)
    }

    #[test]
    fn the_export_name_drops_one_underscore_and_the_argument_size() {
        assert_eq!(export_name("_MethCallEngine@0"), "MethCallEngine");
        assert_eq!(export_name("___vbaUdtVar@8"), "__vbaUdtVar");
        assert_eq!(export_name("_rtcArray"), "rtcArray");
        assert_eq!(export_name("?Fn@@YGXZ"), "?Fn@@YGXZ");
    }

    #[test]
    fn the_primary_table_and_each_lead_table_are_read_with_their_names() {
        let (image, symbols) = fixture();
        let slots = derive(&image, &symbols).unwrap();
        assert_eq!(slots.len(), 256 + 256 + 4 * 2);
        assert_eq!(
            slots[0],
            Slot {
                table: Table::Primary,
                opcode: 0,
                handler: 0x1800,
                names: vec!["Bos".to_owned(), "LargeBos".to_owned()],
            }
        );
        assert!(slots[1].names.is_empty());
        assert_eq!(slots[0xFB].handler, 0x1900);
        let lead0: Vec<&Slot> = slots.iter().filter(|s| s.table == Table::Lead(0)).collect();
        assert_eq!(lead0.len(), 256);
        assert_eq!(lead0[0xFF].names, ["Bos", "LargeBos"]);
        let lead4: Vec<&Slot> = slots.iter().filter(|s| s.table == Table::Lead(4)).collect();
        assert_eq!(lead4.len(), 2);
    }

    #[test]
    fn two_exports_that_give_two_bases_for_one_segment_are_refused() {
        let (mut image, mut symbols) = fixture();
        image.exports.insert("Other".to_owned(), 0x1010);
        symbols.publics.push(public("_Other", 0x20));
        let err = derive(&image, &symbols).unwrap_err();
        assert!(err.contains("more than one base"), "{err}");
    }

    #[test]
    fn a_lead_byte_that_does_not_go_to_its_lead_handler_is_refused() {
        let (image, mut symbols) = fixture();
        symbols.publics.retain(|p| p.name != "_lblBEX_Lead2");
        symbols.publics.push(public("_lblBEX_Lead2", 0x980));
        let err = derive(&image, &symbols).unwrap_err();
        assert!(err.contains("0xfd goes to"), "{err}");
    }

    #[test]
    fn a_lead_handler_of_another_shape_is_refused() {
        let (mut image, _) = fixture();
        image.sections[0].1[0x900] = 0x90;
        let err = lead_table(&image, 0x1900).unwrap_err();
        assert!(err.contains("no known shape"), "{err}");
        let (image, _) = fixture();
        assert_eq!(lead_table(&image, 0x1900).unwrap(), (0x1400, 256));
        assert_eq!(lead_table(&image, 0x1920).unwrap(), (0x1800, 2));
    }

    #[test]
    fn the_rendered_table_keys_each_slot_by_its_table_and_its_opcode() {
        let slots = [
            Slot {
                table: Table::Primary,
                opcode: 0x13,
                handler: 0x1038F5,
                names: vec!["ExitProcHresult".to_owned()],
            },
            Slot {
                table: Table::Lead(4),
                opcode: 0,
                handler: 0x1804,
                names: Vec::new(),
            },
        ];
        let text = render(&slots).unwrap();
        let parsed: toml::Table = text.parse().unwrap();
        assert_eq!(
            parsed["primary"]["13"]["names"][0].as_str(),
            Some("ExitProcHresult")
        );
        assert_eq!(
            parsed["primary"]["13"]["handler"].as_str(),
            Some("0x001038f5")
        );
        assert_eq!(
            parsed["lead4"]["00"]["names"].as_array().map(Vec::len),
            Some(0)
        );
    }
}
