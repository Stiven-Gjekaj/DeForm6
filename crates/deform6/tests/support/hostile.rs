#![allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]

//! Hostile images this repository builds and owns outright.
//!
//! `AGENTS.md`, the section named "What may enter this repository", bars a
//! binary, a source file, or a derived fixture from a third party system
//! the author does not own, and it says in the same sentence that this
//! includes a fixture calculated from such a file. A truncated corpus
//! program is such a fixture. So is a corpus program with one field
//! patched. Neither may be committed. An image this module builds may be,
//! because every byte in it is a literal written in this file: it is
//! copied from no file on disk and calculated from no third party file.
//! An input a fuzzer generated may also be committed, once a human has
//! read it, which the written procedure in `regressions.rs` records.
//!
//! This module calls nothing under `crates/deform6/src/`, matching the
//! rule the doc comment of [`super`] states for the other three modules
//! in this directory.

/// The size of one GUI table entry: `lStructSize` at offset `0x00`, a form
/// pointer at offset `0x48`, and `0x50` bytes total.
///
/// `STRUCTURES.md` section 8.1 gives this value. It is written here as a
/// literal, not imported from `deform6::vb::gui`, because this module
/// calls nothing under `src/`.
const GUI_ENTRY_SIZE: u32 = 0x50;

/// Builds a 4096 byte, 32 bit i386 portable executable holding one mapped
/// section and one valid GUI table entry in that section.
///
/// The layout, in the order the bytes are written:
///
/// 1. `MZ`, the two letter signature, at file offset `0x00`.
/// 2. The offset of the PE header, `0x40`, at file offset `0x3c`.
/// 3. `PE\0\0`, the header signature, at file offset `0x40`.
/// 4. `0x014c`, the machine value for i386, in the COFF file header.
/// 5. `1`, the section count, in the COFF file header.
/// 6. `224`, the optional header size, in the COFF file header.
/// 7. `0x0102`, the characteristics, in the COFF file header.
/// 8. `0x010b`, the optional header magic for a 32 bit image.
/// 9. `0x0040_0000`, the image base.
/// 10. One section header, named `.text`, giving a mapped length, a
///     virtual address of `0x1000`, a raw size equal to the mapped
///     length, and a raw pointer of `0x400`.
/// 11. One GUI table entry at the start of that section: `lStructSize` of
///     [`GUI_ENTRY_SIZE`] at the entry's own offset `0x00`, and a form
///     pointer of `0x0040_1010`, which resolves inside the section, at
///     the entry's own offset `0x48`.
///
/// The mapped section is sized to hold that one entry and nothing more.
/// The file is then padded with zero bytes to 4096 total, so the file is
/// large and the mapped region is small. That gap is the whole fixture: a
/// `VBHeader` declaring a form count of `0xFFFF` against this one-entry
/// region is impossible for the region and not obviously impossible for
/// the file, which is the shape `GuiTable::walk`'s own
/// `ImplausibleCount` defect bounds. This builder stops at the image
/// itself; no `VBHeader` is written into it, because the plan 05-02
/// fixture this layout is copied from did not write one into its own
/// image either. It built a `VbHeader` value in memory and passed it to
/// `GuiTable::walk` directly. A caller that wants the same defect out of
/// this image can do the same.
///
/// The function is deterministic. Two calls give two equal vectors.
#[must_use]
pub fn gui_table_overcount_4k() -> Vec<u8> {
    const LFANEW: usize = 0x40;
    const OPTIONAL: usize = LFANEW + 24;
    const SECTION: usize = OPTIONAL + 224;
    const SECTION_START: usize = 0x400;
    const TOTAL_LEN: usize = 4096;

    let mut entry = vec![0_u8; GUI_ENTRY_SIZE as usize];
    entry[0x00..0x04].copy_from_slice(&GUI_ENTRY_SIZE.to_le_bytes());
    entry[0x48..0x4c].copy_from_slice(&0x0040_1010_u32.to_le_bytes());

    let mapped_len = u32::try_from(entry.len()).unwrap_or(0);
    let minimum_len = SECTION_START + entry.len();
    assert!(
        TOTAL_LEN >= minimum_len,
        "the fixed total length {TOTAL_LEN} is shorter than the {minimum_len} bytes the \
         mapped section alone already needs"
    );

    let mut out = vec![0_u8; minimum_len];
    out[0] = b'M';
    out[1] = b'Z';
    out[0x3c..0x40].copy_from_slice(&u32::try_from(LFANEW).unwrap_or(0).to_le_bytes());
    out[LFANEW..LFANEW + 4].copy_from_slice(b"PE\0\0");

    // The COFF file header.
    out[LFANEW + 4..LFANEW + 6].copy_from_slice(&0x014c_u16.to_le_bytes());
    out[LFANEW + 6..LFANEW + 8].copy_from_slice(&1_u16.to_le_bytes());
    out[LFANEW + 20..LFANEW + 22].copy_from_slice(&224_u16.to_le_bytes());
    out[LFANEW + 22..LFANEW + 24].copy_from_slice(&0x0102_u16.to_le_bytes());

    // The PE32 optional header.
    out[OPTIONAL..OPTIONAL + 2].copy_from_slice(&0x010b_u16.to_le_bytes());
    out[OPTIONAL + 0x1c..OPTIONAL + 0x20].copy_from_slice(&0x0040_0000_u32.to_le_bytes());

    // One section: RVA 0x1000, file offset 0x400, mapped length sized to
    // hold the one GUI table entry and nothing more.
    out[SECTION..SECTION + 8].copy_from_slice(b".text\0\0\0");
    out[SECTION + 8..SECTION + 12].copy_from_slice(&mapped_len.to_le_bytes());
    out[SECTION + 12..SECTION + 16].copy_from_slice(&0x1000_u32.to_le_bytes());
    out[SECTION + 16..SECTION + 20].copy_from_slice(&mapped_len.to_le_bytes());
    out[SECTION + 20..SECTION + 24]
        .copy_from_slice(&u32::try_from(SECTION_START).unwrap_or(0).to_le_bytes());
    out[SECTION + 36..SECTION + 40].copy_from_slice(&0x6000_0020_u32.to_le_bytes());

    out[SECTION_START..SECTION_START + entry.len()].copy_from_slice(&entry);

    out.resize(TOTAL_LEN, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::gui_table_overcount_4k;

    #[test]
    fn the_image_is_exactly_4096_bytes() {
        let bytes = gui_table_overcount_4k();
        assert_eq!(bytes.len(), 4096);
    }

    #[test]
    fn two_calls_give_equal_vectors() {
        assert_eq!(gui_table_overcount_4k(), gui_table_overcount_4k());
    }

    #[test]
    fn the_image_opens_with_the_portable_executable_signature() {
        let bytes = gui_table_overcount_4k();
        assert_eq!(&bytes[0..2], b"MZ");
    }
}
