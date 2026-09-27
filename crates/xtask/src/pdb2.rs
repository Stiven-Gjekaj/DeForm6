//! A reader of the program database format 2.00, the format of the symbol
//! file that Microsoft serves for the Visual Basic 6 runtime.
//!
//! The file starts with the text `Microsoft C/C++ program database 2.00`.
//! It holds a number of streams, which are lists of pages. This reader reads
//! only what `derive-pcode-table` needs:
//!
//! - the stream directory;
//! - the header of the DBI stream (stream 3), for the number of the symbol
//!   record stream and for the two OMAP streams;
//! - the public symbols in the symbol record stream, each with its segment
//!   and its offset;
//! - an OMAP stream, which maps an address of one layout to the address of
//!   the other.
//!
//! The runtime was reordered after the link. So the address of a public
//! symbol is an address of the layout before the reorder, and the OMAP from
//! the source layout gives its address in the file.
//!
//! The file comes from outside the repository. Each read checks its bounds,
//! and a fault gives an error that names the place.

/// The first 44 bytes of the file.
const SIGNATURE: &[u8] = b"Microsoft C/C++ program database 2.00\r\n\x1aJG\0\0";

/// The largest page size that this reader accepts.
const MAX_PAGE: usize = 0x1_0000;

/// The stream size that marks a stream that is not present.
const NO_STREAM: u32 = u32::MAX;

/// The record type of a public symbol with a length-prefixed name,
/// `S_PUB32_ST`.
const S_PUB32_ST: u16 = 0x1009;

/// The stream number that marks a debug stream that is not present.
const NO_DEBUG_STREAM: u16 = 0xFFFF;

/// The length of the DBI header of the 1997 format.
const DBI_HEADER_LEN: usize = 64;

/// Reads a little-endian `u16` at `at`.
pub(crate) fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let end = at.checked_add(2)?;
    Some(u16::from_le_bytes(bytes.get(at..end)?.try_into().ok()?))
}

/// Reads a little-endian `u32` at `at`.
pub(crate) fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(at..end)?.try_into().ok()?))
}

/// Converts a `u32` from the file to a `usize`.
fn to_usize(value: u32) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| format!("the value {value:#x} does not fit a usize"))
}

/// A program database 2.00, with each stream read into memory.
#[derive(Debug)]
pub(crate) struct Pdb2 {
    streams: Vec<Vec<u8>>,
}

impl Pdb2 {
    /// Reads the stream directory and each stream.
    ///
    /// # Errors
    ///
    /// Gives an error when the file does not start with [`SIGNATURE`], when
    /// the page size is 0 or larger than [`MAX_PAGE`], and when a size or a
    /// page number points past the end of the file.
    pub(crate) fn parse(file: &[u8]) -> Result<Self, String> {
        if !file.starts_with(SIGNATURE) {
            return Err("the file is not a program database 2.00".to_owned());
        }
        let page = to_usize(u32_at(file, 44).ok_or("the file ends in its header")?)?;
        if page == 0 || page > MAX_PAGE {
            return Err(format!(
                "the page size {page:#x} is not one this reader accepts"
            ));
        }
        let dir_size = u32_at(file, 52).ok_or("the file ends in its header")?;
        let dir_pages = page_numbers(file, 60, dir_size, page)?;
        let dir = gather(file, &dir_pages, dir_size, page)?;

        let count = usize::from(u16_at(&dir, 0).ok_or("the stream directory is empty")?);
        let mut sizes = Vec::new();
        for index in 0..count {
            let at = index
                .checked_mul(8)
                .and_then(|at| at.checked_add(4))
                .ok_or("a stream directory offset overflows")?;
            sizes.push(u32_at(&dir, at).ok_or_else(|| {
                format!("the stream directory ends before the size of stream {index}")
            })?);
        }
        let mut at = count
            .checked_mul(8)
            .and_then(|at| at.checked_add(4))
            .ok_or("a stream directory offset overflows")?;
        let mut streams = Vec::new();
        for (index, size) in sizes.into_iter().enumerate() {
            if size == NO_STREAM {
                streams.push(Vec::new());
                continue;
            }
            let pages = page_numbers(&dir, at, size, page)
                .map_err(|err| format!("stream {index}: {err}"))?;
            at = pages
                .len()
                .checked_mul(2)
                .and_then(|len| at.checked_add(len))
                .ok_or("a stream directory offset overflows")?;
            streams.push(
                gather(file, &pages, size, page).map_err(|err| format!("stream {index}: {err}"))?,
            );
        }
        Ok(Self { streams })
    }

    /// Gives stream `index`, or `None` when the file has no such stream.
    pub(crate) fn stream(&self, index: u16) -> Option<&[u8]> {
        self.streams.get(usize::from(index)).map(Vec::as_slice)
    }
}

/// Reads the page numbers of a stream of `size` bytes from `list` at `at`.
fn page_numbers(list: &[u8], at: usize, size: u32, page: usize) -> Result<Vec<u16>, String> {
    let count = to_usize(size)?.div_ceil(page);
    let mut out = Vec::new();
    for index in 0..count {
        let place = index
            .checked_mul(2)
            .and_then(|off| at.checked_add(off))
            .ok_or("a page list offset overflows")?;
        out.push(u16_at(list, place).ok_or_else(|| {
            format!("the page list ends before page {index} of a stream of {size} bytes")
        })?);
    }
    Ok(out)
}

/// Joins the pages of a stream and cuts the result to `size` bytes.
fn gather(file: &[u8], pages: &[u16], size: u32, page: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for number in pages {
        let start = usize::from(*number)
            .checked_mul(page)
            .ok_or("a page offset overflows")?;
        let end = start.checked_add(page).ok_or("a page offset overflows")?;
        // The last page of the file can be short.
        let bytes = file
            .get(start..end)
            .or_else(|| file.get(start..))
            .filter(|bytes| !bytes.is_empty())
            .ok_or_else(|| format!("page {number} is past the end of the file"))?;
        out.extend_from_slice(bytes);
    }
    let size = to_usize(size)?;
    if out.len() < size {
        return Err(format!(
            "the pages hold {} bytes of a stream of {size}",
            out.len()
        ));
    }
    out.truncate(size);
    Ok(out)
}

/// What the DBI stream gives: the stream of the symbol records and the two
/// OMAP streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dbi {
    /// The stream that holds the symbol records.
    pub(crate) sym_records: u16,
    /// The OMAP from the layout of the file to the source layout.
    pub(crate) omap_to_src: Option<u16>,
    /// The OMAP from the source layout to the layout of the file.
    pub(crate) omap_from_src: Option<u16>,
}

impl Dbi {
    /// Reads the header of the DBI stream and its list of debug streams.
    ///
    /// # Errors
    ///
    /// Gives an error when the header does not start with the signature -1,
    /// and when a length points past the end of the stream.
    pub(crate) fn parse(dbi: &[u8]) -> Result<Self, String> {
        if u32_at(dbi, 0) != Some(u32::MAX) {
            return Err("the DBI stream does not start with the signature -1".to_owned());
        }
        let field = |at: usize| u32_at(dbi, at).ok_or("the DBI header is cut");
        let sym_records = u16_at(dbi, 20).ok_or("the DBI header is cut")?;
        let mut at = DBI_HEADER_LEN;
        // cbGpModi, cbSC, cbSecMap, cbFileInfo, cbTSMap, then cbECInfo.
        for place in [24, 28, 32, 36, 40, 52] {
            at = at
                .checked_add(to_usize(field(place)?)?)
                .ok_or("a DBI offset overflows")?;
        }
        let debug_len = to_usize(field(48)?)?;
        let debug = |index: usize| -> Result<Option<u16>, String> {
            if index.checked_mul(2).is_none_or(|len| len >= debug_len) {
                return Ok(None);
            }
            let place = index
                .checked_mul(2)
                .and_then(|off| at.checked_add(off))
                .ok_or("a DBI offset overflows")?;
            let stream = u16_at(dbi, place).ok_or("the DBI debug header is cut")?;
            Ok((stream != NO_DEBUG_STREAM).then_some(stream))
        };
        Ok(Self {
            sym_records,
            omap_to_src: debug(3)?,
            omap_from_src: debug(4)?,
        })
    }
}

/// A public symbol: a name at an offset in a segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Public {
    /// The name, as the record holds it.
    pub(crate) name: String,
    /// The segment, counted from 1.
    pub(crate) segment: u16,
    /// The offset in the segment.
    pub(crate) offset: u32,
}

/// Reads each `S_PUB32_ST` record of a symbol record stream. A record of
/// another type is skipped.
///
/// # Errors
///
/// Gives an error when a record runs past the end of the stream.
pub(crate) fn publics(records: &[u8]) -> Result<Vec<Public>, String> {
    let mut out = Vec::new();
    let mut at = 0_usize;
    while at < records.len() {
        let len = usize::from(u16_at(records, at).ok_or("a symbol record is cut")?);
        let next = at
            .checked_add(2)
            .and_then(|at| at.checked_add(len))
            .ok_or("a symbol record offset overflows")?;
        let record = records
            .get(at..next)
            .ok_or_else(|| format!("the symbol record at {at:#x} runs past the stream"))?;
        if u16_at(record, 2) == Some(S_PUB32_ST) {
            let offset = u32_at(record, 8).ok_or("a public symbol is cut")?;
            let segment = u16_at(record, 12).ok_or("a public symbol is cut")?;
            let name_len = usize::from(*record.get(14).ok_or("a public symbol is cut")?);
            let name_end = name_len.checked_add(15).ok_or("a name length overflows")?;
            let name = record
                .get(15..name_end)
                .ok_or_else(|| format!("the name of the public symbol at {at:#x} is cut"))?;
            out.push(Public {
                name: String::from_utf8_lossy(name).into_owned(),
                segment,
                offset,
            });
        }
        at = next;
    }
    Ok(out)
}

/// An OMAP: pairs of (source address, target address), sorted by the source
/// address.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Omap {
    pairs: Vec<(u32, u32)>,
}

impl Omap {
    /// Reads an OMAP stream: eight bytes for each pair.
    ///
    /// # Errors
    ///
    /// Gives an error when the pairs are not sorted by their source address.
    pub(crate) fn parse(stream: &[u8]) -> Result<Self, String> {
        let mut pairs = Vec::new();
        for pair in stream.chunks_exact(8) {
            let from = u32_at(pair, 0).ok_or("an OMAP pair is cut")?;
            let to = u32_at(pair, 4).ok_or("an OMAP pair is cut")?;
            if pairs.last().is_some_and(|&(last, _)| last > from) {
                return Err(format!("the OMAP is not sorted at {from:#x}"));
            }
            pairs.push((from, to));
        }
        Ok(Self { pairs })
    }

    /// Maps `address` through the last pair whose source address is not
    /// above it. Gives `None` when no pair is there, when that pair maps to
    /// 0 (the code was removed), and when the sum overflows.
    pub(crate) fn translate(&self, address: u32) -> Option<u32> {
        let index = self.pairs.partition_point(|&(from, _)| from <= address);
        let &(from, to) = self.pairs.get(index.checked_sub(1)?)?;
        if to == 0 {
            return None;
        }
        to.checked_add(address.checked_sub(from)?)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
pub(crate) mod tests {
    use super::{Dbi, Omap, Pdb2, Public, SIGNATURE, publics};

    /// Builds a program database 2.00 with a page size of 0x40, holding
    /// `streams`. The directory is on page 1, and each stream follows it.
    pub(crate) fn pdb(streams: &[&[u8]]) -> Vec<u8> {
        const PAGE: usize = 0x40;
        let pages_of = |len: usize| len.div_ceil(PAGE);
        let mut next = 2_u16;
        let mut dir = Vec::new();
        dir.extend_from_slice(&u16::try_from(streams.len()).unwrap().to_le_bytes());
        dir.extend_from_slice(&[0, 0]);
        for stream in streams {
            dir.extend_from_slice(&u32::try_from(stream.len()).unwrap().to_le_bytes());
            dir.extend_from_slice(&[0; 4]);
        }
        let mut body = Vec::new();
        for stream in streams {
            for chunk in stream.chunks(PAGE) {
                dir.extend_from_slice(&next.to_le_bytes());
                next += 1;
                let mut page = chunk.to_vec();
                page.resize(PAGE, 0);
                body.extend_from_slice(&page);
            }
        }
        assert!(
            pages_of(dir.len()) == 1,
            "the test directory must fit one page"
        );
        let mut file = SIGNATURE.to_vec();
        file.extend_from_slice(&u32::try_from(PAGE).unwrap().to_le_bytes());
        file.extend_from_slice(&[0; 4]);
        file.extend_from_slice(&u32::try_from(dir.len()).unwrap().to_le_bytes());
        file.extend_from_slice(&[0; 4]);
        file.extend_from_slice(&1_u16.to_le_bytes());
        file.resize(PAGE, 0);
        let mut page = dir;
        page.resize(PAGE, 0);
        file.extend_from_slice(&page);
        file.extend_from_slice(&body);
        file
    }

    #[test]
    fn each_stream_reads_back_across_its_pages() {
        let long: Vec<u8> = (0..150_u8).collect();
        let file = pdb(&[b"abc", &long, b""]);
        let parsed = Pdb2::parse(&file).unwrap();
        assert_eq!(parsed.stream(0), Some(&b"abc"[..]));
        assert_eq!(parsed.stream(1), Some(&long[..]));
        assert_eq!(parsed.stream(2), Some(&b""[..]));
        assert_eq!(parsed.stream(3), None);
    }

    #[test]
    fn a_file_of_another_format_and_a_page_past_the_end_are_refused() {
        assert!(Pdb2::parse(b"Microsoft C/C++ MSF 7.00\r\n").is_err());
        let mut file = pdb(&[b"abc"]);
        let cut = file.len() - 0x40;
        file.truncate(cut);
        let err = Pdb2::parse(&file).unwrap_err();
        assert!(err.contains("past the end"), "{err}");
    }

    /// A DBI header of the 1997 format with the symbol records in stream 11,
    /// no module, and the debug streams 5, none, none, 6 and 7.
    fn dbi() -> Vec<u8> {
        let mut out = vec![0_u8; 64];
        out[0..4].copy_from_slice(&u32::MAX.to_le_bytes());
        out[20..22].copy_from_slice(&11_u16.to_le_bytes());
        out[48..52].copy_from_slice(&10_u32.to_le_bytes());
        for stream in [5_u16, 0xFFFF, 0xFFFF, 6, 7] {
            out.extend_from_slice(&stream.to_le_bytes());
        }
        out
    }

    #[test]
    fn the_dbi_header_gives_the_symbol_stream_and_the_two_omaps() {
        assert_eq!(
            Dbi::parse(&dbi()).unwrap(),
            Dbi {
                sym_records: 11,
                omap_to_src: Some(6),
                omap_from_src: Some(7),
            }
        );
        let mut other = dbi();
        other[0] = 0;
        assert!(Dbi::parse(&other).is_err());
    }

    /// One `S_PUB32_ST` record for `name` at `segment:offset`.
    fn public(name: &str, segment: u16, offset: u32) -> Vec<u8> {
        let mut body = 0x1009_u16.to_le_bytes().to_vec();
        body.extend_from_slice(&0_u32.to_le_bytes());
        body.extend_from_slice(&offset.to_le_bytes());
        body.extend_from_slice(&segment.to_le_bytes());
        body.push(u8::try_from(name.len()).unwrap());
        body.extend_from_slice(name.as_bytes());
        let mut out = u16::try_from(body.len()).unwrap().to_le_bytes().to_vec();
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn each_public_symbol_is_read_and_a_record_of_another_type_is_skipped() {
        let mut records = public("_tblByteDisp", 2, 0x5980);
        records.extend_from_slice(&[4, 0, 0x0C, 0x11, 0, 0]);
        records.extend_from_slice(&public("_lblEX_Branch", 2, 0x10));
        assert_eq!(
            publics(&records).unwrap(),
            [
                Public {
                    name: "_tblByteDisp".to_owned(),
                    segment: 2,
                    offset: 0x5980,
                },
                Public {
                    name: "_lblEX_Branch".to_owned(),
                    segment: 2,
                    offset: 0x10,
                },
            ]
        );
        let cut = records.len() - 1;
        assert!(publics(&records[..cut]).is_err());
    }

    #[test]
    fn the_omap_maps_through_the_last_pair_below_and_gives_none_for_removed_code() {
        let mut stream = Vec::new();
        for (from, to) in [(0x1000_u32, 0x5000_u32), (0x1100, 0), (0x1200, 0x2000)] {
            stream.extend_from_slice(&from.to_le_bytes());
            stream.extend_from_slice(&to.to_le_bytes());
        }
        let omap = Omap::parse(&stream).unwrap();
        assert_eq!(omap.translate(0x0FFF), None);
        assert_eq!(omap.translate(0x1000), Some(0x5000));
        assert_eq!(omap.translate(0x10FF), Some(0x50FF));
        assert_eq!(omap.translate(0x1150), None);
        assert_eq!(omap.translate(0x1234), Some(0x2034));
        let mut unsorted = stream[8..].to_vec();
        unsorted.extend_from_slice(&stream[..8]);
        assert!(Omap::parse(&unsorted).is_err());
    }
}
