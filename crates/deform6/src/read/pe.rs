//! The portable executable envelope, and the owned section table.
//!
//! This is the only file in the crate that names the `object` crate. Every
//! type that leaves this file is ours: [`SectionInfo`] is an owned copy of a
//! section header, and [`PeReject`] folds the nine English strings that
//! `object` produces into the three outcomes a person needs. The seam is
//! narrow so that it stays cheap to replace.
//!
//! `object` uses `unsafe` for performance and its README says that malformed
//! input yields an error rather than a panic. The library forbids `unsafe`,
//! so every unsafe line in the process belongs to `object` or to the standard
//! library. The Phase 5 fuzz target enters through this public API, so a
//! crash inside `object` is found here and reported upstream.

use object::LittleEndian as LE;
use object::read::pe::{ImageNtHeaders as _, ImageOptionalHeader as _, PeFile32};

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::region::{Off, Region, Rva, Va};

/// The number of bytes from the PE signature to the optional header.
///
/// Four bytes of signature and twenty bytes of COFF file header.
const SIGNATURE_AND_FILE_HEADER: u32 = 24;

/// The number of bytes one section header takes in the section table.
const SECTION_HEADER_LEN: u32 = 40;

/// Why DeForm6 does not read this portable executable.
///
/// `NotPe` carries the `object::Error` so that the Phase 4 report can print
/// it as an evidence field. `object::Error` is `Copy` and `Eq`, so this type
/// is too. Its text is reachable through `Display` only, because the field
/// inside it is private, so the report converts with `to_string` at that
/// boundary.
///
/// The other two are ours, not `object`'s. `object` reads a 64 bit image and
/// an ARM image without complaint, and DeForm6 does not.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum PeReject {
    /// The bytes are not a portable executable.
    #[error("this is not a portable executable: {0}")]
    NotPe(object::Error),
    /// The image is for another processor.
    #[error("this portable executable is not an i386 image")]
    NotI386,
    /// The image is not a 32 bit image.
    #[error("this portable executable is not a 32 bit image")]
    NotPe32,
}

impl From<PeReject> for Refusal {
    /// Keeps `object::Error` out of the public error type.
    ///
    /// All three rejections fold into the three refusals that map to exit
    /// code 1. There is no value in showing nine different English strings
    /// from a third party crate to a person who typed a filename.
    fn from(reject: PeReject) -> Self {
        match reject {
            PeReject::NotPe(_) => Self::NotPe,
            PeReject::NotI386 => Self::NotI386,
            PeReject::NotPe32 => Self::NotPe32,
        }
    }
}

/// One section of the image, as an owned value.
///
/// Nothing outside this file holds an `object` type, so this struct copies
/// the five fields the address map needs. It carries no lifetime, so a
/// section list outlives both the parse and the bytes it came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SectionInfo {
    /// The eight raw bytes of the name, padded with NUL.
    pub name: [u8; 8],
    /// Where the section starts in memory, as a relative virtual address.
    pub virtual_address: Rva,
    /// How many bytes the section takes in memory.
    pub virtual_size: u32,
    /// Where the bytes of the section start in the file.
    pub pointer_to_raw_data: Off,
    /// How many bytes of the section the file holds.
    pub size_of_raw_data: u32,
}

impl SectionInfo {
    /// Gives the number of bytes of this section that the file holds.
    ///
    /// This is the smaller of the virtual size and the raw size, always. It
    /// handles both directions the format allows. When the virtual size is
    /// larger, the difference is the zero filled tail, and the file holds no
    /// byte for it. When the raw size is larger, the difference is file
    /// alignment padding, which the loader does not map, so reading it would
    /// return bytes at an address the program never sees.
    ///
    /// On the corpus, 7 of 132 sections have a virtual size larger than the
    /// raw size, all of them `.data`. The other 125 have the padding.
    ///
    /// There is no fallback that swaps in the raw size when the virtual size
    /// is 0. No corpus section declares a zero virtual size, but an old
    /// linker writes 0 there for an object file, and the fallback would let a
    /// hand made file turn any padding into readable data. A section with a
    /// zero mapped length maps nothing, so every address in it resolves to
    /// nothing and the section is named in the refusal.
    ///
    /// `u32::min` is not a `const fn` on 1.97.1, so this is an `if`.
    #[must_use]
    pub const fn mapped_len(&self) -> u32 {
        if self.virtual_size < self.size_of_raw_data {
            self.virtual_size
        } else {
            self.size_of_raw_data
        }
    }
}

/// A parsed 32 bit i386 portable executable.
#[derive(Debug)]
pub struct PeImage<'a> {
    data: &'a [u8],
    file: PeFile32<'a>,
    image_base: u32,
    entry_rva: Rva,
    sections: Vec<SectionInfo>,
    defects: Vec<Defect>,
}

impl<'a> PeImage<'a> {
    /// Reads the headers of a portable executable.
    ///
    /// # A successful parse is not evidence of an intact file
    ///
    /// This validates headers only. It does not check that the bytes a
    /// section declares are present. The Mandelbrot corpus file truncated to
    /// one eighth of its 28,672 bytes still parses, and so does the same file
    /// with one byte in the middle flipped. Any code that treats this method
    /// as a validity gate is a bug.
    ///
    /// Every read after this parse goes through a `Region`, whose bounds come
    /// from the real length of the byte slice, never from a field in the
    /// file.
    ///
    /// # Errors
    ///
    /// Returns [`PeReject::NotPe`] when `object` refuses the headers,
    /// [`PeReject::NotI386`] when the machine is not i386, and
    /// [`PeReject::NotPe32`] when the optional header is not a PE32 header.
    pub fn parse(data: &'a [u8]) -> Result<Self, PeReject> {
        let file = PeFile32::parse(data).map_err(PeReject::NotPe)?;
        let nt = file.nt_headers();

        // `machine` is a field wrapped in an endian type, so it needs `get`.
        if nt.file_header().machine.get(LE) != object::pe::IMAGE_FILE_MACHINE_I386 {
            return Err(PeReject::NotI386);
        }
        // `magic` is a trait method and it returns a plain `u16`.
        if nt.optional_header().magic() != object::pe::IMAGE_NT_OPTIONAL_HDR32_MAGIC {
            return Err(PeReject::NotPe32);
        }

        // `image_base` returns `u64` even here, because the trait is shared
        // with the PE32+ optional header. The saturating narrowing is never
        // reached on a PE32 image, whose field is a `u32`.
        let image_base = u32::try_from(nt.optional_header().image_base()).unwrap_or(u32::MAX);
        // This is a relative virtual address. It is not a virtual address and
        // it is not a file offset. Wrap it here so nothing downstream can
        // confuse the three.
        let entry_rva = Rva::new(nt.optional_header().address_of_entry_point());

        let table = file.section_table();
        let mut sections = Vec::with_capacity(table.len());
        for section in table.iter() {
            sections.push(SectionInfo {
                name: section.name,
                virtual_address: Rva::new(section.virtual_address.get(LE)),
                virtual_size: section.virtual_size.get(LE),
                pointer_to_raw_data: Off::new(section.pointer_to_raw_data.get(LE)),
                size_of_raw_data: section.size_of_raw_data.get(LE),
            });
        }

        let table_at = section_table_offset(
            file.dos_header().nt_headers_offset(),
            nt.file_header().size_of_optional_header.get(LE),
        );
        let defects = overlap_defects(&sections, table_at);

        Ok(Self {
            data,
            file,
            image_base,
            entry_rva,
            sections,
            defects,
        })
    }

    /// Gives the address the loader maps the first byte of the image to.
    #[must_use]
    pub const fn image_base(&self) -> u32 {
        self.image_base
    }

    /// Gives the entry point as a relative virtual address.
    #[must_use]
    pub const fn entry_rva(&self) -> Rva {
        self.entry_rva
    }

    /// Gives the owned section table, in section table order.
    #[must_use]
    pub fn sections(&self) -> &[SectionInfo] {
        &self.sections
    }

    /// Gives the defects the parse found in the section table.
    ///
    /// The only defect this file records is an overlap between two sections,
    /// and it is recorded once, at parse time. No corpus file overlaps, so
    /// this list is empty for all 44 of them.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }

    /// Gives the section that holds `rva`, or nothing.
    ///
    /// # This is the one address to file offset predicate
    ///
    /// `section_for`, [`PeImage::rva_to_off`], [`PeImage::va_to_off`],
    /// [`PeImage::region_at`] and [`PeImage::region_at_va`] all route through
    /// this method, so they cannot disagree with one another.
    ///
    /// The `object` crate offers two helpers that answer this same question
    /// and give opposite answers for one address. One compares the distance
    /// against the virtual size of the section and says yes for an address in
    /// the zero filled tail. The other compares the distance against the
    /// mapped length and says no for the same address. A third helper is
    /// built on the first one, so two calls in the same program can resolve
    /// one address two ways. 7 of the 132 corpus sections have such a tail,
    /// so the disagreement is reachable, not theoretical. DeForm6 owns one
    /// predicate and calls neither helper.
    ///
    /// The distance is a `checked_sub`, so an address below the section
    /// returns nothing rather than a wrapped distance. The comparison is
    /// strictly less than the mapped length, so the first address above the
    /// mapped bytes is in no section.
    ///
    /// The first section in section table order wins. First match is
    /// deterministic. When two sections overlap, the Windows loader gives the
    /// later one, so the two disagree, and [`PeImage::defects`] reports the
    /// overlap that the parse found.
    #[must_use]
    pub fn section_for(&self, rva: Rva) -> Option<&SectionInfo> {
        self.sections
            .iter()
            .find(|s| match rva.get().checked_sub(s.virtual_address.get()) {
                Some(distance) => distance < s.mapped_len(),
                None => false,
            })
    }

    /// Converts a relative virtual address into a file offset.
    ///
    /// An address in no section resolves to nothing, and the caller turns
    /// that into an `UnmappedAddress` defect. There is no fallback that
    /// treats the address as a file offset. On an image whose file alignment
    /// equals its section alignment the two often agree, so that fallback
    /// works until the one file where they differ, and it then reports real
    /// bytes from the wrong place with no error. There is no fallback to the
    /// header region either: nothing Visual Basic writes points there.
    #[must_use]
    pub fn rva_to_off(&self, rva: Rva) -> Option<Off> {
        let section = self.section_for(rva)?;
        let distance = rva.get().checked_sub(section.virtual_address.get())?;
        section.pointer_to_raw_data.checked_add(distance)
    }

    /// Converts a virtual address into a file offset.
    ///
    /// An address below the image base resolves to nothing, because
    /// `Va::to_rva` is a `checked_sub`.
    #[must_use]
    pub fn va_to_off(&self, va: Va) -> Option<Off> {
        self.rva_to_off(va.to_rva(self.image_base)?)
    }

    /// Gives a bounded window that starts at `rva` and runs to the end of the
    /// mapped bytes of its section.
    ///
    /// The base of the window is the file offset, which is what lets a defect
    /// three levels down name an absolute byte offset. `object` cannot give
    /// that: its own helpers return a bare slice or a pair of integers with
    /// no offset attached.
    ///
    /// A section that declares more raw data than the file holds gives a
    /// short window rather than nothing. The bytes that do exist are real,
    /// and every read inside the window is still bounded by the real length
    /// of the byte slice.
    #[must_use]
    pub fn region_at(&self, rva: Rva) -> Option<Region<'a>> {
        let section = self.section_for(rva)?;
        let distance = rva.get().checked_sub(section.virtual_address.get())?;
        let offset = section.pointer_to_raw_data.checked_add(distance)?;
        let mapped = section.mapped_len().checked_sub(distance)?;
        let start = usize::try_from(offset.get()).unwrap_or(usize::MAX);
        let want = usize::try_from(mapped).unwrap_or(usize::MAX);
        let rest = self.data.get(start..)?;
        // `get` gives nothing when the section declares more bytes than the
        // file holds. The window is then everything that is left.
        let bytes = rest.get(..want).unwrap_or(rest);
        Some(Region::new(bytes, offset))
    }

    /// Gives a bounded window that starts at a virtual address.
    ///
    /// This is `Va::to_rva` followed by [`PeImage::region_at`]. It is the
    /// only route from a virtual address to bytes, because `Va` has no
    /// conversion into `Off` and no `Add`.
    #[must_use]
    pub fn region_at_va(&self, va: Va) -> Option<Region<'a>> {
        self.region_at(va.to_rva(self.image_base)?)
    }

    /// Tells whether the image declares a common language runtime header.
    ///
    /// This reads data directory 14 and nothing else. Plan 01-06 uses it to
    /// say "this is a .NET assembly" instead of "this holds no Visual Basic
    /// runtime". Data directory 14 is zero in all 44 corpus executables, so
    /// the positive case comes from a fixture that a test builds in memory.
    #[must_use]
    pub fn has_clr_header(&self) -> bool {
        self.file
            .data_directory(object::pe::IMAGE_DIRECTORY_ENTRY_COM_DESCRIPTOR)
            .is_some()
    }
}

/// Gives the file offset of the first section header.
///
/// The saturating fallback is safe: an offset of `u32::MAX` names no section
/// header that any later read reaches, and every defect built from it is
/// still a defect.
fn section_table_offset(nt_headers_offset: u32, size_of_optional_header: u16) -> u32 {
    nt_headers_offset
        .checked_add(SIGNATURE_AND_FILE_HEADER)
        .and_then(|at| at.checked_add(u32::from(size_of_optional_header)))
        .unwrap_or(u32::MAX)
}

/// Gives the file offset of the section header at `index`.
fn section_header_offset(table_at: u32, index: usize) -> u32 {
    u32::try_from(index)
        .ok()
        .and_then(|i| i.checked_mul(SECTION_HEADER_LEN))
        .and_then(|step| table_at.checked_add(step))
        .unwrap_or(u32::MAX)
}

/// Reports every pair of sections that claims the same address range.
///
/// This runs once, after the section list is built, and never per lookup. It
/// is O(n squared) over at most 96 sections, which is cheap and obvious.
///
/// The format requires a linker to write sections that ascend and do not
/// overlap, and a hostile file is not the output of a linker. The Windows
/// loader maps sections in table order, so a later section that overlaps an
/// earlier one wins in memory, while `section_for` takes the first match. The two disagree, and that disagreement is why the overlap is
/// reported rather than resolved in silence: a file with overlapping sections
/// is either damaged or built to make two parsers see different bytes.
///
/// **This rule is untested by construction.** All 44 corpus executables have
/// exactly 3 sections, `.text`, `.data` and `.rsrc`, ascending, so no corpus
/// file reaches this branch. A fuzz input or a hand made regression file has
/// to exercise it, in the same way the P-code branch is called out.
///
/// A section whose mapped length is zero claims no byte, so it overlaps
/// nothing. An end that leaves a `u32` is reported as an overlap rather than
/// clamped to `u32::MAX`, because a section that runs past the end of the
/// address space is not a section whose end can be trusted.
fn overlap_defects(sections: &[SectionInfo], table_at: u32) -> Vec<Defect> {
    let mut out = Vec::new();
    for (i, a) in sections.iter().enumerate() {
        for (j, b) in sections.iter().enumerate().skip(i.saturating_add(1)) {
            if a.mapped_len() == 0 || b.mapped_len() == 0 {
                continue;
            }
            let a_start = a.virtual_address.get();
            let b_start = b.virtual_address.get();
            let ends = a
                .virtual_address
                .checked_add(a.mapped_len())
                .zip(b.virtual_address.checked_add(b.mapped_len()));
            let overlaps = match ends {
                Some((a_end, b_end)) => a_start < b_end.get() && b_start < a_end.get(),
                None => true,
            };
            if overlaps {
                let offset = section_header_offset(table_at, j);
                out.push(Defect {
                    site: Site {
                        offset,
                        rva: Some(b_start),
                        structure: "ImageSectionHeader",
                        field: "VirtualAddress",
                    },
                    kind: DefectKind::SectionOverlap {
                        offset,
                        other: a.pointer_to_raw_data.get(),
                    },
                });
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{PeImage, PeReject, SectionInfo};
    use crate::error::Refusal;
    use crate::read::region::{Off, Rva, Va};

    /// The general purpose corpus file.
    const MANDELBROT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
    ));

    /// The corpus file that holds a section with a zero filled tail.
    ///
    /// `Mandelbrot.exe` cannot serve here. All three of its sections declare
    /// a raw size larger than their virtual size, so it has no tail and a
    /// tail test against it would pass while proving nothing. 7 of the 132
    /// corpus sections have a tail and all 7 are `.data`.
    const PASSGEN: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/PassGen/PassGen.exe"
    ));

    /// Gives the first section whose virtual size exceeds its raw size.
    ///
    /// The index is not hard coded. A test that named section 1 would keep
    /// passing against a section that no longer has a tail.
    fn the_section_with_a_zero_filled_tail(image: &PeImage) -> SectionInfo {
        *image
            .sections()
            .iter()
            .find(|s| s.virtual_size > s.size_of_raw_data)
            .expect(
                "this corpus file must hold a section whose virtual size exceeds its raw size, \
                 or the zero filled tail is not exercised at all",
            )
    }

    #[test]
    fn the_corpus_file_parses_and_names_its_three_sections() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        assert_eq!(image.image_base(), 0x0040_0000);
        let names: Vec<String> = image
            .sections()
            .iter()
            .map(|s| {
                String::from_utf8_lossy(&s.name)
                    .trim_end_matches('\0')
                    .to_string()
            })
            .collect();
        assert_eq!(names, vec![".text", ".data", ".rsrc"]);
    }

    #[test]
    fn an_empty_slice_is_not_a_portable_executable() {
        assert!(matches!(PeImage::parse(&[]), Err(PeReject::NotPe(_))));
    }

    #[test]
    fn a_short_text_file_is_not_a_portable_executable() {
        let text = b"this is not a program\n";
        assert!(matches!(PeImage::parse(text), Err(PeReject::NotPe(_))));
    }

    #[test]
    fn an_ne_image_is_not_a_portable_executable() {
        let mut bytes = vec![0u8; 0x80];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        bytes[0x3c..0x40].copy_from_slice(&0x40_u32.to_le_bytes());
        bytes[0x40] = b'N';
        bytes[0x41] = b'E';
        assert!(matches!(PeImage::parse(&bytes), Err(PeReject::NotPe(_))));
    }

    #[test]
    fn the_entry_point_lies_inside_the_text_section() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let entry = image.entry_rva().get();
        assert_ne!(entry, 0);
        let text = &image.sections()[0];
        assert_eq!(&text.name[..5], b".text");
        assert!(entry >= text.virtual_address.get());
        assert!(entry < text.virtual_address.get() + text.mapped_len());
    }

    #[test]
    fn the_corpus_file_holds_no_clr_header() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        assert!(!image.has_clr_header());
    }

    /// Gives a copy of `data` whose fifteenth data directory names a real
    /// section.
    ///
    /// Data directory 14 is zero in all 44 corpus executables, so the corpus
    /// gives `has_clr_header` no positive case. This builds one in memory.
    /// Nothing is written to disk.
    ///
    /// The helper asserts that both words were zero before the write. A
    /// future corpus file that already carries a common language runtime
    /// header would make this fixture a no-op, and the test would then pass
    /// for the wrong reason.
    fn with_a_clr_directory(data: &[u8]) -> Vec<u8> {
        let mut out = data.to_vec();
        let lfanew = u32::from_le_bytes(out[0x3c..0x40].try_into().unwrap());
        // 24 reaches the optional header from the PE signature. 96 reaches
        // the data directories from the start of a PE32 optional header. The
        // fifteenth directory is 14 entries of 8 bytes further on.
        let at = usize::try_from(lfanew).unwrap() + 24 + 96 + 14 * 8;
        let address = u32::from_le_bytes(out[at..at + 4].try_into().unwrap());
        let size = u32::from_le_bytes(out[at + 4..at + 8].try_into().unwrap());
        assert_eq!(address, 0, "data directory 14 already names an address");
        assert_eq!(size, 0, "data directory 14 already declares a size");

        let first = PeImage::parse(data).unwrap().sections()[0]
            .virtual_address
            .get();
        out[at..at + 4].copy_from_slice(&first.to_le_bytes());
        // 0x48 is the size of the common language runtime header structure.
        out[at + 4..at + 8].copy_from_slice(&0x48_u32.to_le_bytes());
        out
    }

    #[test]
    fn a_patched_data_directory_fourteen_makes_the_image_a_dot_net_assembly() {
        let patched = with_a_clr_directory(MANDELBROT);
        let image = PeImage::parse(&patched).unwrap();
        assert!(image.has_clr_header());
    }

    #[test]
    fn every_reject_folds_into_the_matching_refusal() {
        let not_pe = PeImage::parse(&[]).unwrap_err();
        assert_eq!(Refusal::from(not_pe), Refusal::NotPe);
        assert_eq!(Refusal::from(PeReject::NotI386), Refusal::NotI386);
        assert_eq!(Refusal::from(PeReject::NotPe32), Refusal::NotPe32);
    }

    #[test]
    fn the_entry_point_resolves_to_the_file_offset_of_its_first_stub_byte() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let offset = image.rva_to_off(image.entry_rva()).unwrap();
        let at = usize::try_from(offset.get()).unwrap();
        assert!(at < MANDELBROT.len());
        assert_eq!(MANDELBROT[at], 0x68);
    }

    #[test]
    fn an_address_in_a_zero_filled_tail_resolves_to_nothing() {
        let image = PeImage::parse(PASSGEN).unwrap();
        let section = the_section_with_a_zero_filled_tail(&image);
        // The boundary comes from the raw field, never from `mapped_len`.
        // A test that asked the method it covers where the boundary is would
        // move its own goal posts when that method changed.
        let first_unmapped = section.virtual_address.get() + section.size_of_raw_data;
        assert!(first_unmapped < section.virtual_address.get() + section.virtual_size);
        assert_eq!(image.rva_to_off(Rva::new(first_unmapped)), None);
        assert!(image.section_for(Rva::new(first_unmapped)).is_none());
        assert!(image.region_at(Rva::new(first_unmapped)).is_none());
    }

    #[test]
    fn the_last_mapped_address_of_that_section_still_resolves() {
        let image = PeImage::parse(PASSGEN).unwrap();
        let section = the_section_with_a_zero_filled_tail(&image);
        let last_mapped = section.virtual_address.get() + section.size_of_raw_data - 1;
        let offset = image.rva_to_off(Rva::new(last_mapped)).unwrap();
        let at = usize::try_from(offset.get()).unwrap();
        assert!(at < PASSGEN.len());
    }

    #[test]
    fn an_address_in_the_file_alignment_padding_resolves_to_nothing() {
        // The other direction of the same rule. The loader maps the virtual
        // size, so the bytes above it in the file are padding at an address
        // the program never sees. 125 of the 132 corpus sections have it.
        //
        // This case is what a `mapped_len` of `size_of_raw_data` alone would
        // let through, and the zero filled tail cannot catch that, because
        // for a section with a tail the raw size **is** the smaller of the
        // two.
        let image = PeImage::parse(MANDELBROT).unwrap();
        let section = *image
            .sections()
            .iter()
            .find(|s| s.size_of_raw_data > s.virtual_size)
            .expect(
                "this corpus file must hold a section whose raw size exceeds its virtual size, \
                 or the file alignment padding is not exercised at all",
            );
        let first_padding = section.virtual_address.get() + section.virtual_size;
        assert!(first_padding < section.virtual_address.get() + section.size_of_raw_data);
        assert_eq!(image.rva_to_off(Rva::new(first_padding)), None);
        assert!(image.section_for(Rva::new(first_padding)).is_none());

        let last_mapped = first_padding - 1;
        assert!(image.rva_to_off(Rva::new(last_mapped)).is_some());
    }

    #[test]
    fn an_address_above_every_section_resolves_to_nothing() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let above = image
            .sections()
            .iter()
            .map(|s| s.virtual_address.get() + s.mapped_len())
            .max()
            .unwrap();
        // The value is a plausible file offset as well, so a fallback that
        // treated the address as an offset would answer here.
        assert!(usize::try_from(above).unwrap() <= MANDELBROT.len());
        assert_eq!(image.rva_to_off(Rva::new(above)), None);
        assert_eq!(image.rva_to_off(Rva::new(0xffff_ffff)), None);
    }

    #[test]
    fn a_virtual_address_below_the_image_base_resolves_to_nothing() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let below = Va::new(image.image_base() - 1);
        assert_eq!(image.va_to_off(below), None);
        assert_eq!(image.va_to_off(Va::new(0x1000)), None);
    }

    #[test]
    fn a_region_starts_at_the_offset_that_rva_to_off_gives() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let rva = image.entry_rva();
        let region = image.region_at(rva).unwrap();
        assert_eq!(region.file_offset(Off::new(0)), image.rva_to_off(rva));
        assert_eq!(region.u8(Off::new(0)), Some(0x68));
    }

    #[test]
    fn region_at_va_and_region_at_agree_on_every_address() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let base = image.image_base();
        let mut resolved = 0_u32;
        for step in 0..0x400_u32 {
            let rva = Rva::new(step * 0x40);
            let va = Va::new(base + rva.get());
            match (image.region_at(rva), image.region_at_va(va)) {
                (Some(by_rva), Some(by_va)) => {
                    assert_eq!(
                        by_rva.file_offset(Off::new(0)),
                        by_va.file_offset(Off::new(0))
                    );
                    assert_eq!(by_rva.len(), by_va.len());
                    resolved += 1;
                }
                (None, None) => {}
                _ => panic!("the two routes disagree at rva {:#x}", rva.get()),
            }
        }
        assert!(resolved > 0, "no address resolved, so nothing was compared");
    }

    #[test]
    fn the_section_list_outlives_the_bytes_it_was_built_from() {
        // The bytes live in this block and the section list leaves it. A
        // `SectionInfo` that borrowed an `object` type would not compile
        // here, because `object` borrows the byte slice.
        let sections: Vec<SectionInfo> = {
            let bytes: Vec<u8> = MANDELBROT.to_vec();
            let image = PeImage::parse(&bytes).unwrap();
            image.sections().to_vec()
        };
        assert_eq!(sections.len(), 3);
        assert_eq!(&sections[0].name[..5], b".text");
    }
}
