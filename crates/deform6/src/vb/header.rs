//! The `VBHeader`, and the entry point stub that reaches it.
//!
//! The entry point of a Visual Basic 5 or 6 standard executable is a two
//! instruction stub. The first instruction pushes the virtual address of the
//! `VBHeader`. The second calls the runtime. This module matches the first
//! byte and validates what the pushed address holds. It does not disassemble.
//!
//! `STRUCTURES.md` section 1.2 recommends this shape: match, then validate.
//! The strongest check it names, that the byte at `entry + 5` is `0xE8` and
//! that the call target is an import thunk, needs a disassembler. That check
//! is a confidence upgrade for the Phase 4 report, not a gate here.

use crate::error::Refusal;
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region};

/// The opcode of `push imm32`.
///
/// All 44 corpus executables begin their entry point with this byte.
/// `[VERIFIED: local, 44 of 44]`
const PUSH_IMM32: u8 = 0x68;

/// The four bytes that begin a `VBHeader`.
///
/// This magic is present in Visual Basic 5 and in Visual Basic 6 alike, so it
/// proves that the file is Visual Basic and it does not prove the version.
/// The version comes from the name of the imported runtime, which is what
/// DET-03 requires.
const VB_MAGIC: &[u8; 4] = b"VB5!";

/// The size of the `VBHeader` structure.
///
/// `STRUCTURES.md` section 2 gives `0x68` = 104 bytes, and all five sources
/// agree.
const HEADER_SIZE: u32 = 0x68;

/// The bound on one header string.
///
/// `Region::cstr` needs a mandatory maximum, so a header with no NUL byte
/// cannot make the scan run to the end of the section.
const STRING_MAX: u32 = 0x104;

/// The width of the window that [`header_region`] returns.
///
/// The structure is `HEADER_SIZE` bytes and the four strings it names live
/// **after** it, at offsets measured from the same base. The window must
/// therefore cover the structure and the string pool that follows it.
///
/// The largest header relative offset in the corpus is 171, and the furthest
/// byte any corpus string reaches is 196. `[VERIFIED: local, 44 of 44]` This
/// window is 364 bytes, which is the structure plus one `STRING_MAX`, so an
/// offset that leaves it is a refusal rather than a read of the rest of the
/// section.
const HEADER_WINDOW: u32 = HEADER_SIZE + STRING_MAX;

/// Follows the entry point stub and gives a window on the `VBHeader`.
///
/// The window starts at the `VB5!` magic and its base is the file offset of
/// that magic, so a defect inside the header names an absolute byte offset
/// with no caller threading one through.
///
/// Only the opcode `0x68` is accepted. `STRUCTURES.md` section 1.3 records
/// that one prior tool also accepts `0x5A` at the entry point and `0x11` at
/// `entry + 5`, with no sample and no explanation, and it notes that `0x5A`
/// is `pop edx`, which does not fit the five byte layout. A variant with no
/// sample is a guess with a code path, and a code path that nobody can build
/// a file for is a place a hostile file can go where a test cannot follow.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when the entry point is in no section, when
/// the first byte is not a push of an immediate, when the pushed address is
/// in no section, or when the four bytes there are not the magic.
pub fn header_region<'a>(pe: &PeImage<'a>) -> Result<Region<'a>, Refusal> {
    let entry = pe
        .region_at(pe.entry_rva())
        .ok_or(Refusal::Damaged("the entry point is in no section"))?;
    if entry.u8(Off::new(0)) != Some(PUSH_IMM32) {
        return Err(Refusal::Damaged(
            "the entry point is not a push of an immediate",
        ));
    }
    // The operand is typed as a virtual address at the read. Nothing
    // downstream may treat it as anything else.
    let va = entry.va_le(Off::new(1)).ok_or(Refusal::Damaged(
        "the push operand runs past the end of the section",
    ))?;
    // `region_at_va` is the only route from a virtual address to bytes. It is
    // `Va::to_rva`, a checked subtraction of the image base, and then the one
    // section predicate. An address below the image base gives nothing, so it
    // becomes a refusal rather than a read of the DOS stub.
    let hdr = pe
        .region_at_va(va)
        .ok_or(Refusal::Damaged("the pushed address is in no section"))?;
    if hdr.take(Off::new(0), 4) != Some(VB_MAGIC.as_slice()) {
        return Err(Refusal::Damaged("the header does not begin with VB5!"));
    }
    // The window is clamped to the bytes that the section holds rather than
    // refused when it is short. The bytes that do exist are real, and every
    // read inside the window is still bounded. This is the same choice
    // `Region::cstr` makes, and for the same reason.
    let width = HEADER_WINDOW.min(hdr.len());
    hdr.subregion(Off::new(0), width)
        .ok_or(Refusal::Damaged("the VB header window is not readable"))
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
    use super::{HEADER_SIZE, PUSH_IMM32, header_region};
    use crate::error::Refusal;
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Va};

    /// The worked example of `RESEARCH.md` section 8.4.
    ///
    /// The test reads every value it needs out of these bytes. It pins no
    /// size, no section count and no build number, because those are
    /// properties of one build and not of the format.
    const MANDELBROT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
    ));

    /// Copies the corpus bytes and writes one byte at the entry point.
    ///
    /// The offset comes from the file's own header, through
    /// `PeImage::rva_to_off`. Nothing searches for a byte pattern, and
    /// nothing is written to disk.
    fn with_entry_opcode(byte: u8) -> Vec<u8> {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let at = image.rva_to_off(image.entry_rva()).unwrap();
        let mut out = MANDELBROT.to_vec();
        out[usize::try_from(at.get()).unwrap()] = byte;
        out
    }

    /// Copies the corpus bytes and writes the `u32` at `entry + 1`.
    fn with_pushed_operand(value: u32) -> Vec<u8> {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let at = usize::try_from(image.rva_to_off(image.entry_rva()).unwrap().get()).unwrap();
        let mut out = MANDELBROT.to_vec();
        out[at + 1..at + 5].copy_from_slice(&value.to_le_bytes());
        out
    }

    /// Copies the corpus bytes and writes over the four magic bytes.
    fn with_broken_magic() -> Vec<u8> {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let hdr = header_region(&image).unwrap();
        let at = usize::try_from(hdr.file_offset(Off::new(0)).unwrap().get()).unwrap();
        let mut out = MANDELBROT.to_vec();
        out[at..at + 4].copy_from_slice(b"XB5!");
        out
    }

    #[test]
    fn the_entry_point_pushes_an_immediate_that_lands_on_the_visual_basic_magic() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let entry = image.region_at(image.entry_rva()).unwrap();
        assert_eq!(entry.u8(Off::new(0)), Some(PUSH_IMM32));
        let hdr = header_region(&image).unwrap();
        assert_eq!(hdr.take(Off::new(0), 4), Some(b"VB5!".as_slice()));
        assert!(hdr.len() >= HEADER_SIZE);
    }

    #[test]
    fn the_header_window_is_based_on_the_file_offset_of_the_magic() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let hdr = header_region(&image).unwrap();
        let at = usize::try_from(hdr.file_offset(Off::new(0)).unwrap().get()).unwrap();
        // The bytes are taken from the raw file, not from the window, so the
        // base is checked against something the window did not produce.
        assert_eq!(&MANDELBROT[at..at + 4], b"VB5!");
    }

    #[test]
    fn an_entry_opcode_of_0x5a_is_refused_because_no_corpus_file_shows_it() {
        let bytes = with_entry_opcode(0x5A);
        let image = PeImage::parse(&bytes).unwrap();
        assert_eq!(
            header_region(&image).unwrap_err(),
            Refusal::Damaged("the entry point is not a push of an immediate")
        );
    }

    #[test]
    fn an_entry_opcode_of_0x11_is_refused_because_no_corpus_file_shows_it() {
        let bytes = with_entry_opcode(0x11);
        let image = PeImage::parse(&bytes).unwrap();
        assert_eq!(
            header_region(&image).unwrap_err(),
            Refusal::Damaged("the entry point is not a push of an immediate")
        );
    }

    #[test]
    fn a_header_that_does_not_begin_with_the_magic_is_refused() {
        let bytes = with_broken_magic();
        let image = PeImage::parse(&bytes).unwrap();
        assert_eq!(
            header_region(&image).unwrap_err(),
            Refusal::Damaged("the header does not begin with VB5!")
        );
    }

    #[test]
    fn a_pushed_address_below_the_image_base_is_refused_and_the_dos_stub_is_not_read() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        // A value below the image base. `Va::to_rva` is a checked
        // subtraction, so this gives nothing rather than a wrapped address.
        let low = image.image_base() - 1;
        let bytes = with_pushed_operand(low);
        let broken = PeImage::parse(&bytes).unwrap();
        assert_eq!(
            header_region(&broken).unwrap_err(),
            Refusal::Damaged("the pushed address is in no section")
        );
        // The file does begin with the DOS magic, so a route that resolved
        // the low address to file offset 0 would have read "MZ" and reported
        // it with no error.
        assert_eq!(&bytes[0..2], b"MZ");
        assert!(Va::new(low).to_rva(image.image_base()).is_none());
    }
}
