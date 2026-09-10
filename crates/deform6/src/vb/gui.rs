//! The GUI table, `GUIObjectInfo`, and the property stream each form's own
//! control block opens.
//!
//! `STRUCTURES.md` sections 8.1 and 8.2 give the offsets. `03-RESEARCH.md`
//! Pattern 1 gives the measured evidence, byte for byte, against
//! `corpus/public-domain/LockWorkStation/LockWorkStation.exe`.
//!
//! This file locates the stream and bounds it. It reads only the form's own
//! outermost control block: the name, the type, and the declared length.
//! Walking the rest of the tree (the child controls, the scope-byte runs
//! between them) is plan 03-04's `controltree.rs`. This file's own tracer
//! test, `tests/form_tracer.rs`, says so in its own doc comment, so nobody
//! mistakes the tracer for the differential gate plan 03-10 builds.
//!
//! # The window-before-fields discipline
//!
//! Every structure this file reads is narrowed to its own bounded window
//! before any field inside it is read, the same discipline `vb/object.rs`
//! documents: a fresh `subregion` is taken per structure, never one region
//! indexed by hand.
//!
//! # `GUIObjectInfo` is not 4-byte aligned internally
//!
//! A single byte at offset `0x04` throws every field after it onto an odd
//! offset. `Region`'s field readers assume no alignment, so this needs no
//! special handling; it is called out here because it is easy to suspect a
//! missing byte if the first read from a new sample produces garbage.

use crate::error::Refusal;
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Va};
use crate::vb::header::VbHeader;

/// The size of one `tGuiTable` entry. `STRUCTURES.md` section 8.1: `0x50`
/// bytes, and it is the format's own validation gate: `lStructSize` at the
/// entry's own offset `0x00` must equal this value.
pub const GUI_ENTRY_SIZE: u32 = 0x50;

/// The size of the fixed `GUIObjectInfo` header, before the property stream
/// that follows it. `STRUCTURES.md` section 8.2: `0x5D` bytes.
pub const GUI_OBJECT_INFO_SIZE: u32 = 0x5D;

/// One entry of the GUI table: the address of one form's `GUIObjectInfo`.
///
/// `STRUCTURES.md` section 8.1 names several other fields in this entry as
/// unresolved (`[G]`). Only `aFormPointer`, the one field this phase needs,
/// is carried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuiTableEntry {
    /// The virtual address of this form's [`GuiObjectInfo`] block.
    pub a_form_pointer: Va,
}

/// The GUI table: `VBHeader.wFormCount` entries at `VBHeader.lpGuiTable`,
/// one per form. It does not include modules or classes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuiTable {
    /// Every entry the walk recovered, in array order.
    pub entries: Vec<GuiTableEntry>,
}

impl GuiTable {
    /// Walks the GUI table.
    ///
    /// Each element is narrowed to its own [`GUI_ENTRY_SIZE`]-byte window
    /// before any field inside it is read, the loop bound is
    /// `header.w_form_count`, and the element offset is computed with
    /// `checked_mul` on a bare `u32`, then `subregion`: the same shape
    /// `vb::object::ObjectTable::walk` uses for its own array.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when the GUI table pointer is in no
    /// section, when the file ends inside an entry, or when an entry's
    /// `lStructSize` is not [`GUI_ENTRY_SIZE`]. `STRUCTURES.md` section 8.1
    /// names this the one cheap validation gate the format gives; a bad
    /// value there refuses the whole walk, because the array's stride is not
    /// proven for any entry after one that fails it.
    pub fn walk(pe: &PeImage<'_>, header: &VbHeader) -> Result<Self, Refusal> {
        let array = pe
            .region_at_va(header.lp_gui_table)
            .ok_or(Refusal::Damaged("the GUI table pointer is in no section"))?;

        let mut entries = Vec::new();
        for i in 0_u32..u32::from(header.w_form_count) {
            let at = i
                .checked_mul(GUI_ENTRY_SIZE)
                .ok_or(Refusal::Damaged("the GUI table index overflows a u32"))?;
            let entry = array
                .subregion(Off::new(at), GUI_ENTRY_SIZE)
                .ok_or(Refusal::Damaged("the file ends inside a GUI table entry"))?;

            let l_struct_size = entry
                .u32_le(Off::new(0x00))
                .ok_or(Refusal::Damaged("a GUI table entry holds no lStructSize"))?;
            if l_struct_size != GUI_ENTRY_SIZE {
                let offset = entry.file_offset(Off::new(0x00)).map_or(0, Off::get);
                return Err(damaged(format!(
                    "the GUI table entry at file offset {offset:#x} holds lStructSize \
                     {l_struct_size:#x}, not {GUI_ENTRY_SIZE:#x}"
                )));
            }

            let a_form_pointer = entry.va_le(Off::new(0x48)).ok_or(Refusal::Damaged(
                "a GUI table entry holds no address for its form",
            ))?;
            entries.push(GuiTableEntry { a_form_pointer });
        }

        Ok(Self { entries })
    }
}

/// The fixed header at a form's `aFormPointer`, and the gateway to the
/// property stream that follows it.
///
/// Only `lPropertiesLength` is a public field. `STRUCTURES.md` section 8.2
/// names several other fields (`guidObjectGUI`, `uuidUnknown1`,
/// `guidCOMEventsIID`, nine unknown dwords) that no plan in this phase
/// reads; carrying them would be dead weight with no test to hold it to.
#[derive(Debug)]
pub struct GuiObjectInfo<'a> {
    /// The total byte length of the property stream that follows this
    /// header: the form's own control block, every child, and every
    /// scope-byte run between them.
    pub l_properties_length: u32,
    /// The window from `aFormPointer` to the end of its section. Kept so
    /// [`GuiObjectInfo::form_stream`] can carve the property stream's own
    /// window out of it without re-resolving the address.
    window: Region<'a>,
}

impl<'a> GuiObjectInfo<'a> {
    /// Reads the fixed `GUIObjectInfo` header at `a_form_pointer`.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when `a_form_pointer` is in no section,
    /// or when the file ends inside the [`GUI_OBJECT_INFO_SIZE`]-byte
    /// header.
    pub fn read(pe: &PeImage<'a>, a_form_pointer: Va) -> Result<Self, Refusal> {
        let window = pe.region_at_va(a_form_pointer).ok_or(Refusal::Damaged(
            "a GUI table entry's aFormPointer is in no section",
        ))?;
        let header =
            window
                .subregion(Off::new(0), GUI_OBJECT_INFO_SIZE)
                .ok_or(Refusal::Damaged(
                    "the file ends inside a GUIObjectInfo block",
                ))?;
        let l_properties_length = header.u32_le(Off::new(0x59)).ok_or(Refusal::Damaged(
            "a GUIObjectInfo block holds no lPropertiesLength",
        ))?;
        Ok(Self {
            l_properties_length,
            window,
        })
    }

    /// Gives the property stream this header introduces.
    ///
    /// The stream starts at `aFormPointer + GUI_OBJECT_INFO_SIZE` and runs
    /// for `l_properties_length` bytes, built with `subregion`, which uses
    /// `checked_add` internally: a length field larger than the real file
    /// gives `None` here, never a buffer sized from the file.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] naming the file offset and the declared
    /// length when the stream runs past the end of the file.
    pub fn form_stream(&self) -> Result<FormStream<'a>, Refusal> {
        let offset = self
            .window
            .file_offset(Off::new(GUI_OBJECT_INFO_SIZE))
            .map_or(0, Off::get);
        let region = self
            .window
            .subregion(Off::new(GUI_OBJECT_INFO_SIZE), self.l_properties_length)
            .ok_or_else(|| {
                damaged(format!(
                    "the property stream at file offset {offset:#x} declares \
                     {} bytes, which runs past the end of the file",
                    self.l_properties_length
                ))
            })?;
        Ok(FormStream { region })
    }
}

/// The property stream a `GuiObjectInfo` introduces: a depth-first
/// serialisation of the control tree, starting with the form's own block.
///
/// This type reads only the form's own outermost block (`STRUCTURES.md`
/// section 8.4's non-array layout). Reading child controls and the
/// scope-byte runs between them is plan 03-04's `controltree.rs`.
#[derive(Debug)]
pub struct FormStream<'a> {
    region: Region<'a>,
}

impl FormStream<'_> {
    /// Reads the form's own block `Length` field, at offset `0x00`.
    #[must_use]
    pub fn length(&self) -> Option<u16> {
        self.region.u16_le(Off::new(0x00))
    }

    /// Reads the form's own name, at offset `0x07`, whose length is the
    /// `u16` at offset `0x05`.
    ///
    /// Each byte becomes its own Latin-1 code point, the rule `object.rs`'s
    /// own `read_name` uses. `String::from_utf8_lossy` is never used here: a
    /// byte in `0x80` to `0xFF` would become the replacement character and
    /// the name would be lost. Gives `None` when the name length or the name
    /// bytes cannot be read within the stream's own window; that never
    /// happens on the corpus this plan's tracer reads, and a hostile file
    /// that shrinks its own declared length loses the name rather than
    /// causing a read past the window, because `Region::take` is bounded.
    #[must_use]
    pub fn name(&self) -> Option<String> {
        let name_len = self.region.u16_le(Off::new(0x05))?;
        let bytes = self.region.take(Off::new(0x07), u32::from(name_len))?;
        Some(bytes.iter().copied().map(char::from).collect())
    }

    /// Reads the form's own `cType` byte, at offset `0x08` plus the name
    /// length. `STRUCTURES.md` section 8.4.1 gives `13` for `Form`.
    #[must_use]
    pub fn control_type(&self) -> Option<u8> {
        let name_len = self.region.u16_le(Off::new(0x05))?;
        let at = 0x08_u32.checked_add(u32::from(name_len))?;
        self.region.u8(Off::new(at))
    }
}

/// Builds a [`Refusal::Damaged`] whose message is computed at runtime.
///
/// `Refusal::Damaged` takes `&'static str`. Every other call site in this
/// crate passes a literal, because `error.rs`'s own module doc states the
/// rule for the type: "A refusal sentence holds no byte offset and no
/// path." This module's own required refusals are the first exception: an
/// out-of-range `lStructSize` and a property stream that tiles the wrong
/// number of bytes must each name the byte offset a hostile file put the
/// bad value at, so a person can open the file there. `Box::leak` is the
/// narrow, deliberate escape hatch: every path that reaches this function
/// is already fatal to the whole file, `inspect` returns immediately after,
/// and the small, bounded string this leaks is reclaimed when the process
/// exits. This is a smaller change than widening `Refusal::Damaged` itself
/// to `String`, which would touch the eleven existing call sites across two
/// earlier phases that do not need one.
fn damaged(message: String) -> Refusal {
    let leaked: &'static str = Box::leak(message.into_boxed_str());
    Refusal::Damaged(leaked)
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
    use super::{GUI_ENTRY_SIZE, GuiObjectInfo, GuiTable};
    use crate::error::Refusal;
    use crate::read::pe::PeImage;
    use crate::read::region::Va;
    use crate::vb::header::{VbHeader, header_region};

    /// The corpus program this task's tracer is worked against.
    const LOCK_WORK_STATION: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/LockWorkStation/LockWorkStation.exe"
    ));

    /// Builds a `VbHeader` literal with only the two fields `GuiTable::walk`
    /// reads set to a caller-chosen value. Every other field is zero or
    /// empty: this file's tests do not read them.
    fn header_with_gui_table(lp_gui_table: Va, w_form_count: u16) -> VbHeader {
        VbHeader {
            signature: *b"VB5!",
            runtime_build: 0,
            lp_sub_main: Va::new(0),
            lp_project_data: Va::new(0),
            f_mdl_int_ctls: 0,
            f_mdl_int_ctls2: 0,
            w_form_count,
            w_external_count: 0,
            lp_gui_table,
            lp_external_table: Va::new(0),
            o_project_exe_name: crate::read::region::Off::new(0),
            o_project_title: crate::read::region::Off::new(0),
            o_help_file: crate::read::region::Off::new(0),
            o_project_name: crate::read::region::Off::new(0),
            exe_name: String::new(),
            title: String::new(),
            help_file: String::new(),
            project_name: String::new(),
        }
    }

    /// Builds a minimal 32 bit i386 portable executable with one section at
    /// RVA `0x1000` / file offset `0x400`, and writes `extra` at the start
    /// of that section.
    ///
    /// Copied from `vb::object::tests::synthetic_image_with_a_short_mapped_section`,
    /// with the mapped length sized to `extra` instead of fixed, so this one
    /// helper serves every test in this module that needs a GUI table entry
    /// at a known, resolvable address.
    fn synthetic_image(extra: &[u8]) -> Vec<u8> {
        const LFANEW: usize = 0x40;
        const OPTIONAL: usize = LFANEW + 24;
        const SECTION: usize = OPTIONAL + 224;
        const SECTION_START: usize = 0x400;

        let mapped_len = u32::try_from(extra.len().max(0x10)).unwrap();
        let file_len = SECTION_START + usize::try_from(mapped_len).unwrap() + 0x10;

        let mut out = vec![0_u8; file_len];
        out[0] = b'M';
        out[1] = b'Z';
        out[0x3c..0x40].copy_from_slice(&u32::try_from(LFANEW).unwrap().to_le_bytes());
        out[LFANEW..LFANEW + 4].copy_from_slice(b"PE\0\0");

        // The COFF file header.
        out[LFANEW + 4..LFANEW + 6].copy_from_slice(&0x014c_u16.to_le_bytes());
        out[LFANEW + 6..LFANEW + 8].copy_from_slice(&1_u16.to_le_bytes());
        out[LFANEW + 20..LFANEW + 22].copy_from_slice(&224_u16.to_le_bytes());
        out[LFANEW + 22..LFANEW + 24].copy_from_slice(&0x0102_u16.to_le_bytes());

        // The PE32 optional header.
        out[OPTIONAL..OPTIONAL + 2].copy_from_slice(&0x010b_u16.to_le_bytes());
        out[OPTIONAL + 0x1c..OPTIONAL + 0x20].copy_from_slice(&0x0040_0000_u32.to_le_bytes());

        // One section: RVA 0x1000, file offset 0x400, mapped length sized
        // to hold `extra`.
        out[SECTION..SECTION + 8].copy_from_slice(b".text\0\0\0");
        out[SECTION + 8..SECTION + 12].copy_from_slice(&mapped_len.to_le_bytes());
        out[SECTION + 12..SECTION + 16].copy_from_slice(&0x1000_u32.to_le_bytes());
        out[SECTION + 16..SECTION + 20].copy_from_slice(&mapped_len.to_le_bytes());
        out[SECTION + 20..SECTION + 24]
            .copy_from_slice(&u32::try_from(SECTION_START).unwrap().to_le_bytes());
        out[SECTION + 36..SECTION + 40].copy_from_slice(&0x6000_0020_u32.to_le_bytes());

        out[SECTION_START..SECTION_START + extra.len()].copy_from_slice(extra);
        out
    }

    /// Gives the GUI table walked out of the real corpus file.
    fn lock_work_station_gui_table() -> GuiTable {
        let image = PeImage::parse(LOCK_WORK_STATION).unwrap();
        let hdr = header_region(&image).unwrap();
        let header = VbHeader::read(&hdr).unwrap();
        GuiTable::walk(&image, &header).unwrap()
    }

    #[test]
    fn the_walk_gives_one_entry_for_the_lock_work_station_form_count() {
        let table = lock_work_station_gui_table();
        assert_eq!(table.entries.len(), 1);
    }

    #[test]
    fn the_walk_reads_the_a_form_pointer_of_a_synthetic_entry() {
        let mut entry = vec![0_u8; GUI_ENTRY_SIZE.try_into().unwrap()];
        entry[0x00..0x04].copy_from_slice(&GUI_ENTRY_SIZE.to_le_bytes());
        entry[0x48..0x4c].copy_from_slice(&0x0040_2000_u32.to_le_bytes());
        let bytes = synthetic_image(&entry);
        let image = PeImage::parse(&bytes).unwrap();
        let header = header_with_gui_table(Va::new(0x0040_1000), 1);

        let table = GuiTable::walk(&image, &header).unwrap();
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].a_form_pointer, Va::new(0x0040_2000));
    }

    #[test]
    fn a_gui_table_entry_with_the_wrong_lstructsize_is_refused_and_names_its_offset() {
        let mut entry = vec![0_u8; GUI_ENTRY_SIZE.try_into().unwrap()];
        // 0x51, not 0x50: STRUCTURES.md section 8.1's one validation gate.
        entry[0x00..0x04].copy_from_slice(&0x51_u32.to_le_bytes());
        let bytes = synthetic_image(&entry);
        let image = PeImage::parse(&bytes).unwrap();
        let header = header_with_gui_table(Va::new(0x0040_1000), 1);

        let err = GuiTable::walk(&image, &header).unwrap_err();
        let Refusal::Damaged(message) = err else {
            panic!("expected Refusal::Damaged, got {err:?}");
        };
        // The entry sits at file offset 0x400, the synthetic section start.
        assert!(
            message.contains("0x400"),
            "the message does not name the entry's byte offset: {message}"
        );
        assert!(message.contains("0x51"), "{message}");
    }

    #[test]
    fn a_gui_table_pointer_in_no_section_is_refused() {
        let bytes = synthetic_image(&[]);
        let image = PeImage::parse(&bytes).unwrap();
        let header = header_with_gui_table(Va::new(0x00F0_0000), 1);

        assert_eq!(
            GuiTable::walk(&image, &header),
            Err(Refusal::Damaged("the GUI table pointer is in no section"))
        );
    }

    #[test]
    fn the_lock_work_station_form_object_info_gives_the_measured_properties_length() {
        let image = PeImage::parse(LOCK_WORK_STATION).unwrap();
        let table = lock_work_station_gui_table();
        let info = GuiObjectInfo::read(&image, table.entries[0].a_form_pointer).unwrap();
        // 03-RESEARCH.md Pattern 1: lPropertiesLength is 0x4f (79).
        assert_eq!(info.l_properties_length, 0x4f);
    }

    #[test]
    fn the_lock_work_station_form_block_gives_its_name_length_and_type() {
        let image = PeImage::parse(LOCK_WORK_STATION).unwrap();
        let table = lock_work_station_gui_table();
        let info = GuiObjectInfo::read(&image, table.entries[0].a_form_pointer).unwrap();
        let stream = info.form_stream().unwrap();

        // 03-RESEARCH.md Pattern 1: Length is 0x4a (74).
        assert_eq!(stream.length(), Some(0x4a));
        assert_eq!(stream.name().as_deref(), Some("FrmLockWorkStation"));
        // STRUCTURES.md section 8.4.1: cType 13 is Form.
        assert_eq!(stream.control_type(), Some(13));
    }

    #[test]
    fn a_properties_length_larger_than_the_file_is_refused() {
        // One combined window: the GUI table entry at offset 0, and the
        // GUIObjectInfo header at offset 0x100, so both sit inside the
        // same mapped section and `mapped_len` covers both.
        let mut extra = vec![0_u8; 0x200];
        extra[0x00..0x04].copy_from_slice(&GUI_ENTRY_SIZE.to_le_bytes());
        // VA 0x00401100: RVA 0x1100 = section RVA 0x1000 + offset 0x100.
        extra[0x48..0x4c].copy_from_slice(&0x0040_1100_u32.to_le_bytes());
        extra[0x100 + 0x59..0x100 + 0x5D].copy_from_slice(&0xFFFF_FF00_u32.to_le_bytes());

        let bytes = synthetic_image(&extra);
        let image = PeImage::parse(&bytes).unwrap();
        let header = header_with_gui_table(Va::new(0x0040_1000), 1);

        let table = GuiTable::walk(&image, &header).unwrap();
        let info = GuiObjectInfo::read(&image, table.entries[0].a_form_pointer).unwrap();
        assert_eq!(info.l_properties_length, 0xFFFF_FF00);

        let err = info.form_stream().unwrap_err();
        let Refusal::Damaged(message) = err else {
            panic!("expected Refusal::Damaged, got {err:?}");
        };
        assert!(message.contains("4294967040"), "{message}");
    }
}
