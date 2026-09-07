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
use crate::read::region::{Off, Region, Va};

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

/// The `VBHeader`, which `STRUCTURES.md` section 2 also calls
/// `EXEPROJECTINFO`.
///
/// # One structure, two kinds of pointer
///
/// This is the detail that makes a field look right when it is read the wrong
/// way, so it is enforced by the types and not by this comment.
///
/// `lp_sub_main`, `lp_project_data`, `lp_gui_table` and `lp_external_table`
/// are **virtual addresses**. They reach bytes only through
/// [`PeImage::region_at_va`], which subtracts the image base with a checked
/// subtraction and then resolves through the section table.
///
/// `o_project_exe_name`, `o_project_title`, `o_help_file` and
/// `o_project_name` are **byte offsets from the start of this structure**.
/// They reach bytes only through `Region::cstr` on the header window itself.
/// They never touch the section table and they never touch the image base.
///
/// `STRUCTURES.md` section 13 records what a parser that carries only `u32`
/// does with this. A first attempt at the corpus check read `0x58` as a
/// virtual address, resolved it, and reported the string `MZ` out of the DOS
/// stub with no error. The value in that file is `0x78` and the header sits
/// at file offset `0x1760`.
///
/// [`Va`] has no conversion into [`Off`], no `From<u32>` and no `Add`, so
/// there is no expression that crosses the two spaces without going through
/// `Va::to_rva`, which takes an image base and would be visibly wrong here.
///
/// [`Va`]: crate::read::region::Va
/// [`PeImage::region_at_va`]: crate::read::pe::PeImage::region_at_va
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VbHeader {
    /// The four magic bytes at offset 0, as the file holds them.
    ///
    /// [`header_region`] has already refused the file when these are not
    /// `VB5!`, so carrying them is not a second check. It exists so the
    /// report prints four characters that were read out of the file rather
    /// than four characters that were written into this source. Those are
    /// different properties.
    pub signature: [u8; 4],
    /// The build number of the runtime the file was linked against.
    pub runtime_build: u16,
    /// The address of `Sub Main`.
    ///
    /// Null means the startup object is a form and not a module.
    pub lp_sub_main: Va,
    /// The address of `ProjectInfo`, which is the spine of the whole file.
    pub lp_project_data: Va,
    /// The bitmask of the intrinsic control classes with identifier 0 to 31.
    pub f_mdl_int_ctls: u32,
    /// The same bitmask for the identifiers 32 to 63.
    pub f_mdl_int_ctls2: u32,
    /// The number of entries in the GUI table, which is a loop bound.
    pub w_form_count: u16,
    /// The number of entries in the external component table.
    pub w_external_count: u16,
    /// The address of the GUI table.
    pub lp_gui_table: Va,
    /// The address of the external component table.
    pub lp_external_table: Va,
    /// The header relative offset of the executable name.
    pub o_project_exe_name: Off,
    /// The header relative offset of the project title.
    pub o_project_title: Off,
    /// The header relative offset of the help file name.
    pub o_help_file: Off,
    /// The header relative offset of the project name.
    pub o_project_name: Off,
    /// The executable name with the extension removed.
    ///
    /// This is the `.vbp` `ExeName32` value minus `.exe`. The extension is
    /// not in the file. `[VERIFIED: local, 44 of 44]`
    pub exe_name: String,
    /// The project title, which is the `.vbp` `Title` value.
    ///
    /// VB6 omits the `Title` key when the title equals the project name.
    /// `[VERIFIED: local, 44 of 44]`
    pub title: String,
    /// The help file name, which is the `.vbp` `HelpFile` value.
    pub help_file: String,
    /// The project name, which is the `.vbp` `Name` value.
    pub project_name: String,
}

impl VbHeader {
    /// Reads the header out of the window that [`header_region`] gives.
    ///
    /// The four strings are resolved inside that same window. Nothing here
    /// reads the file and nothing here reads the image base.
    ///
    /// The sixteen bytes at header relative `0x68` to `0x77` are not read.
    /// They are zero in every corpus file and `STRUCTURES.md` does not name
    /// them. The string pool starts at `0x78` in every corpus file as well,
    /// and this code does not assume that either: it follows the offset in
    /// the field, as the format requires. A length is never derived from the
    /// next offset.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] naming the field when the window is too
    /// short to hold it, and naming the string when the offset leaves the
    /// window or no NUL byte appears within [`STRING_MAX`] bytes of it.
    pub fn read(hdr: &Region<'_>) -> Result<Self, Refusal> {
        let signature: [u8; 4] = hdr
            .take(Off::new(0x00), 4)
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .ok_or(Refusal::Damaged("the VB header holds no signature"))?;
        let o_project_exe_name =
            off_at(hdr, 0x58, "the VB header holds no executable name offset")?;
        let o_project_title = off_at(hdr, 0x5C, "the VB header holds no project title offset")?;
        let o_help_file = off_at(hdr, 0x60, "the VB header holds no help file offset")?;
        let o_project_name = off_at(hdr, 0x64, "the VB header holds no project name offset")?;
        Ok(Self {
            signature,
            runtime_build: hdr
                .u16_le(Off::new(0x04))
                .ok_or(Refusal::Damaged("the VB header holds no runtime build"))?,
            lp_sub_main: va_at(hdr, 0x2C, "the VB header holds no address for Sub Main")?,
            lp_project_data: va_at(hdr, 0x30, "the VB header holds no address for ProjectInfo")?,
            f_mdl_int_ctls: hdr
                .u32_le(Off::new(0x34))
                .ok_or(Refusal::Damaged("the VB header holds no control bitmask"))?,
            f_mdl_int_ctls2: hdr.u32_le(Off::new(0x38)).ok_or(Refusal::Damaged(
                "the VB header holds no second control bitmask",
            ))?,
            w_form_count: hdr
                .u16_le(Off::new(0x44))
                .ok_or(Refusal::Damaged("the VB header holds no form count"))?,
            w_external_count: hdr.u16_le(Off::new(0x46)).ok_or(Refusal::Damaged(
                "the VB header holds no external component count",
            ))?,
            lp_gui_table: va_at(
                hdr,
                0x4C,
                "the VB header holds no address for the GUI table",
            )?,
            lp_external_table: va_at(
                hdr,
                0x50,
                "the VB header holds no address for the external component table",
            )?,
            o_project_exe_name,
            o_project_title,
            o_help_file,
            o_project_name,
            exe_name: header_string(
                hdr,
                o_project_exe_name,
                "the VB header executable name is not a bounded string",
            )?,
            title: header_string(
                hdr,
                o_project_title,
                "the VB header project title is not a bounded string",
            )?,
            help_file: header_string(
                hdr,
                o_help_file,
                "the VB header help file name is not a bounded string",
            )?,
            project_name: header_string(
                hdr,
                o_project_name,
                "the VB header project name is not a bounded string",
            )?,
        })
    }
}

/// Reads a virtual address out of the header window.
fn va_at(hdr: &Region<'_>, at: u32, what: &'static str) -> Result<Va, Refusal> {
    hdr.va_le(Off::new(at)).ok_or(Refusal::Damaged(what))
}

/// Reads a header relative offset out of the header window.
fn off_at(hdr: &Region<'_>, at: u32, what: &'static str) -> Result<Off, Refusal> {
    hdr.off_le(Off::new(at)).ok_or(Refusal::Damaged(what))
}

/// Reads one NUL terminated string at a header relative offset.
///
/// The scan runs inside the header window and it is bounded by
/// [`STRING_MAX`]. A string with no NUL inside that bound is a refusal and
/// not a truncation.
///
/// Each byte is mapped with `char::from`, which gives the Latin-1 code point.
/// `String::from_utf8_lossy` is wrong here: a byte in `0x80` to `0xFF` would
/// become the replacement character and the name would be lost. The full
/// Windows-1252 table arrives with `VbStr` in Phase 3.
fn header_string(hdr: &Region<'_>, at: Off, what: &'static str) -> Result<String, Refusal> {
    let bytes = hdr.cstr(at, STRING_MAX).ok_or(Refusal::Damaged(what))?;
    Ok(bytes.iter().copied().map(char::from).collect())
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
    use super::{HEADER_SIZE, PUSH_IMM32, VbHeader, header_region};
    use crate::error::Refusal;
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Rva, Va};

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
        let at = usize::try_from(header_offset_the_hard_way().get()).unwrap();
        let mut out = MANDELBROT.to_vec();
        out[at..at + 4].copy_from_slice(b"XB5!");
        out
    }

    /// Gives the file offset of the header without calling
    /// [`header_region`].
    ///
    /// A test must not ask the function it covers where its boundary is.
    /// This walks the stub with the plan 01-04 primitives only, so a change
    /// inside `header_region` moves the answer of that function and not the
    /// answer of this one.
    fn header_offset_the_hard_way() -> Off {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let entry = image.region_at(image.entry_rva()).unwrap();
        let va = entry.va_le(Off::new(1)).unwrap();
        image.va_to_off(va).unwrap()
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
        // The expected offset is walked again with the plan 01-04 primitives
        // only, so the test does not ask `header_region` where its own
        // window starts.
        let at = header_offset_the_hard_way();
        assert_eq!(hdr.file_offset(Off::new(0)), Some(at));
        // The bytes are taken from the raw file, not from the window, so the
        // base is checked against something the window did not produce.
        let raw = usize::try_from(at.get()).unwrap();
        assert_eq!(&MANDELBROT[raw..raw + 4], b"VB5!");
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

    /// Reads the corpus header.
    fn mandelbrot_header() -> VbHeader {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let hdr = header_region(&image).unwrap();
        VbHeader::read(&hdr).unwrap()
    }

    /// The `.vbp` beside `Mandelbrot.exe` declares these four values.
    ///
    /// ```text
    /// HelpFile=""
    /// Title="Mandelbrot Fractal Demo"
    /// ExeName32="Mandelbrot.exe"
    /// Name="Mandelbrot_Fractal_Demo"
    /// ```
    ///
    /// These four assertions are also the proof that the two kinds of
    /// pointer are not confused. The field at `0x58` holds `0x78`. Resolved
    /// as a header relative offset it reaches `Mandelbrot`. Resolved as a
    /// virtual address it reaches nothing in this crate, because `0x78` is
    /// below the image base, and it reached the string `MZ` in the survey
    /// that made the mistake with bare `u32` arithmetic.
    #[test]
    fn the_executable_name_is_the_vbp_exe_name_without_its_extension() {
        assert_eq!(mandelbrot_header().exe_name, "Mandelbrot");
    }

    #[test]
    fn the_title_is_the_vbp_title() {
        assert_eq!(mandelbrot_header().title, "Mandelbrot Fractal Demo");
    }

    #[test]
    fn the_help_file_is_the_vbp_help_file_and_it_is_empty() {
        assert_eq!(mandelbrot_header().help_file, "");
    }

    #[test]
    fn the_project_name_is_the_vbp_name() {
        assert_eq!(mandelbrot_header().project_name, "Mandelbrot_Fractal_Demo");
    }

    #[test]
    fn the_project_data_address_resolves_to_a_readable_region() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let hdr = header_region(&image).unwrap();
        let header = VbHeader::read(&hdr).unwrap();
        assert!(!header.lp_project_data.is_null());
        let project = image.region_at_va(header.lp_project_data).unwrap();
        assert!(!project.is_empty());
    }

    #[test]
    fn a_null_sub_main_means_the_startup_object_is_a_form() {
        assert!(mandelbrot_header().lp_sub_main.is_null());
    }

    #[test]
    fn the_signature_is_carried_as_the_four_bytes_the_file_holds() {
        // The offset is walked again with the plan 01-04 primitives, so this
        // does not ask the window under test where it starts.
        let at = usize::try_from(header_offset_the_hard_way().get()).unwrap();
        let header = mandelbrot_header();
        // Compared against the raw file, not against a literal written here,
        // so the field is proved to carry what the file holds.
        assert_eq!(header.signature, MANDELBROT[at..at + 4]);
    }

    /// `STRUCTURES.md` section 13 says a survey read `0x58` as a virtual
    /// address and reported the string `MZ` out of the DOS stub. This crate
    /// cannot reach that string, and this test says why.
    ///
    /// The value is `0x78`. It is below the image base, so `Va::to_rva` gives
    /// nothing, and it is below the address of the first section, so a route
    /// that forgot the image base gives nothing either. The survey needed a
    /// third fault as well: a fallback that reads an unresolved address as a
    /// file offset. `PeImage` has no such fallback, which is threat T-01-12.
    #[test]
    fn the_executable_name_offset_reaches_nothing_when_it_is_read_as_an_address() {
        let image = PeImage::parse(MANDELBROT).unwrap();
        let raw = mandelbrot_header().o_project_exe_name.get();
        assert!(raw < image.image_base());
        assert!(Va::new(raw).to_rva(image.image_base()).is_none());
        assert!(image.region_at_va(Va::new(raw)).is_none());
        assert!(image.region_at(Rva::new(raw)).is_none());
    }

    #[test]
    fn the_four_string_offsets_ascend() {
        let header = mandelbrot_header();
        assert!(header.o_project_exe_name < header.o_project_title);
        assert!(header.o_project_title < header.o_help_file);
        assert!(header.o_help_file <= header.o_project_name);
    }

    /// The offsets and values that [`a_synthetic_vb_image`] writes.
    ///
    /// Every scalar is a different number. Two fields that held the same
    /// value could be swapped with no test noticing.
    mod synth {
        pub const ENTRY_RVA: u32 = 0x1000;
        pub const ENTRY_OFF: usize = 0x400;
        pub const HEADER_RVA: u32 = 0x1100;
        pub const HEADER_OFF: usize = 0x500;
        pub const IMAGE_BASE: u32 = 0x0040_0000;
        pub const RUNTIME_BUILD: u16 = 0x1234;
        pub const SUB_MAIN: u32 = 0x0040_1200;
        pub const PROJECT_DATA: u32 = 0x0040_1300;
        pub const CTLS: u32 = 0xAABB_CCDD;
        pub const CTLS2: u32 = 0x1122_3344;
        pub const FORM_COUNT: u16 = 5;
        pub const EXTERNAL_COUNT: u16 = 7;
        pub const GUI_TABLE: u32 = 0x0040_1400;
        pub const EXTERNAL_TABLE: u32 = 0x0040_1500;
        pub const EXE_NAME: &str = "Synth";
        pub const TITLE: &str = "Synthetic Title";
        pub const HELP_FILE: &str = "Synth.hlp";
        pub const PROJECT_NAME: &str = "Synth_Project";
    }

    /// Builds an image whose one section is at address 0x1000 and at file
    /// offset 0x400.
    ///
    /// Both corpus files put their `.text` section at
    /// `virtual_address == pointer_to_raw_data`, so on either of them an
    /// address and a file offset are the same number and no assertion can
    /// tell them apart. Here they differ by 0xC00.
    ///
    /// Every scalar field holds a different value, so a read at the wrong
    /// offset gives the wrong number rather than the right one by accident.
    /// The corpus cannot do this either: its `lpSubMain` is zero, its
    /// `wExternalCount` is zero, and nothing pins the rest.
    fn a_synthetic_vb_image() -> Vec<u8> {
        const LFANEW: usize = 0x40;
        const OPTIONAL: usize = LFANEW + 24;
        const SECTION: usize = OPTIONAL + 224;

        let mut out = vec![0_u8; 0x800];
        out[0] = b'M';
        out[1] = b'Z';
        out[0x3c..0x40].copy_from_slice(&u32::try_from(LFANEW).unwrap().to_le_bytes());
        out[LFANEW..LFANEW + 4].copy_from_slice(b"PE\0\0");

        // The COFF file header: i386, one section, a 224 byte optional
        // header, and the executable flag.
        out[LFANEW + 4..LFANEW + 6].copy_from_slice(&0x014c_u16.to_le_bytes());
        out[LFANEW + 6..LFANEW + 8].copy_from_slice(&1_u16.to_le_bytes());
        out[LFANEW + 20..LFANEW + 22].copy_from_slice(&224_u16.to_le_bytes());
        out[LFANEW + 22..LFANEW + 24].copy_from_slice(&0x0102_u16.to_le_bytes());

        // The PE32 optional header.
        out[OPTIONAL..OPTIONAL + 2].copy_from_slice(&0x010b_u16.to_le_bytes());
        out[OPTIONAL + 0x10..OPTIONAL + 0x14].copy_from_slice(&synth::ENTRY_RVA.to_le_bytes());
        out[OPTIONAL + 0x1c..OPTIONAL + 0x20].copy_from_slice(&synth::IMAGE_BASE.to_le_bytes());
        out[OPTIONAL + 0x20..OPTIONAL + 0x24].copy_from_slice(&0x1000_u32.to_le_bytes());
        out[OPTIONAL + 0x24..OPTIONAL + 0x28].copy_from_slice(&0x200_u32.to_le_bytes());
        out[OPTIONAL + 0x38..OPTIONAL + 0x3c].copy_from_slice(&0x2000_u32.to_le_bytes());
        out[OPTIONAL + 0x3c..OPTIONAL + 0x40].copy_from_slice(&0x800_u32.to_le_bytes());
        out[OPTIONAL + 0x5c..OPTIONAL + 0x60].copy_from_slice(&16_u32.to_le_bytes());

        // One section, at address 0x1000 and at file offset 0x400.
        out[SECTION..SECTION + 8].copy_from_slice(b".text\0\0\0");
        out[SECTION + 8..SECTION + 12].copy_from_slice(&0x400_u32.to_le_bytes());
        out[SECTION + 12..SECTION + 16].copy_from_slice(&0x1000_u32.to_le_bytes());
        out[SECTION + 16..SECTION + 20].copy_from_slice(&0x400_u32.to_le_bytes());
        out[SECTION + 20..SECTION + 24].copy_from_slice(&0x400_u32.to_le_bytes());
        out[SECTION + 36..SECTION + 40].copy_from_slice(&0x6000_0020_u32.to_le_bytes());

        // The entry stub: push the address of the header, then call.
        let head_va = synth::IMAGE_BASE + synth::HEADER_RVA;
        out[synth::ENTRY_OFF] = PUSH_IMM32;
        out[synth::ENTRY_OFF + 1..synth::ENTRY_OFF + 5].copy_from_slice(&head_va.to_le_bytes());
        out[synth::ENTRY_OFF + 5] = 0xE8;

        // The header itself.
        let h = synth::HEADER_OFF;
        out[h..h + 4].copy_from_slice(b"VB5!");
        out[h + 0x04..h + 0x06].copy_from_slice(&synth::RUNTIME_BUILD.to_le_bytes());
        out[h + 0x2C..h + 0x30].copy_from_slice(&synth::SUB_MAIN.to_le_bytes());
        out[h + 0x30..h + 0x34].copy_from_slice(&synth::PROJECT_DATA.to_le_bytes());
        out[h + 0x34..h + 0x38].copy_from_slice(&synth::CTLS.to_le_bytes());
        out[h + 0x38..h + 0x3C].copy_from_slice(&synth::CTLS2.to_le_bytes());
        out[h + 0x44..h + 0x46].copy_from_slice(&synth::FORM_COUNT.to_le_bytes());
        out[h + 0x46..h + 0x48].copy_from_slice(&synth::EXTERNAL_COUNT.to_le_bytes());
        out[h + 0x4C..h + 0x50].copy_from_slice(&synth::GUI_TABLE.to_le_bytes());
        out[h + 0x50..h + 0x54].copy_from_slice(&synth::EXTERNAL_TABLE.to_le_bytes());

        // The string pool, written end to end with one NUL between the
        // strings, and each offset taken from where the string actually
        // landed rather than from a table written by hand.
        let mut at = 0x78_usize;
        for (slot, text) in [
            (0x58_usize, synth::EXE_NAME),
            (0x5C, synth::TITLE),
            (0x60, synth::HELP_FILE),
            (0x64, synth::PROJECT_NAME),
        ] {
            out[h + slot..h + slot + 4].copy_from_slice(&u32::try_from(at).unwrap().to_le_bytes());
            out[h + at..h + at + text.len()].copy_from_slice(text.as_bytes());
            at += text.len() + 1;
        }
        out
    }

    #[test]
    fn the_header_window_is_based_on_a_file_offset_and_not_on_an_address() {
        let bytes = a_synthetic_vb_image();
        let image = PeImage::parse(&bytes).unwrap();
        let hdr = header_region(&image).unwrap();
        let at = hdr.file_offset(Off::new(0)).unwrap();
        assert_eq!(at, Off::new(u32::try_from(synth::HEADER_OFF).unwrap()));
        // On either corpus file these two numbers are equal, because `.text`
        // has `virtual_address == pointer_to_raw_data`, so the assertion
        // above has no content there. Here they differ by 0xC00.
        assert_ne!(at.get(), synth::HEADER_RVA);
        assert_eq!(image.rva_to_off(Rva::new(synth::HEADER_RVA)), Some(at));
    }

    #[test]
    fn every_scalar_field_is_read_from_the_offset_the_layout_gives() {
        let bytes = a_synthetic_vb_image();
        let image = PeImage::parse(&bytes).unwrap();
        let header = VbHeader::read(&header_region(&image).unwrap()).unwrap();
        assert_eq!(header.signature, *b"VB5!");
        assert_eq!(header.runtime_build, synth::RUNTIME_BUILD);
        assert_eq!(header.lp_sub_main, Va::new(synth::SUB_MAIN));
        assert_eq!(header.lp_project_data, Va::new(synth::PROJECT_DATA));
        assert_eq!(header.f_mdl_int_ctls, synth::CTLS);
        assert_eq!(header.f_mdl_int_ctls2, synth::CTLS2);
        assert_eq!(header.w_form_count, synth::FORM_COUNT);
        assert_eq!(header.w_external_count, synth::EXTERNAL_COUNT);
        assert_eq!(header.lp_gui_table, Va::new(synth::GUI_TABLE));
        assert_eq!(header.lp_external_table, Va::new(synth::EXTERNAL_TABLE));
    }

    #[test]
    fn the_four_strings_resolve_when_the_address_and_the_file_offset_differ() {
        let bytes = a_synthetic_vb_image();
        let image = PeImage::parse(&bytes).unwrap();
        let header = VbHeader::read(&header_region(&image).unwrap()).unwrap();
        assert_eq!(header.exe_name, synth::EXE_NAME);
        assert_eq!(header.title, synth::TITLE);
        assert_eq!(header.help_file, synth::HELP_FILE);
        assert_eq!(header.project_name, synth::PROJECT_NAME);
    }

    #[test]
    fn a_string_offset_that_leaves_the_header_window_is_refused() {
        let mut bytes = a_synthetic_vb_image();
        // A value that is inside the section but outside the window.
        bytes[synth::HEADER_OFF + 0x58..synth::HEADER_OFF + 0x5C]
            .copy_from_slice(&0x0200_u32.to_le_bytes());
        let image = PeImage::parse(&bytes).unwrap();
        let hdr = header_region(&image).unwrap();
        assert_eq!(
            VbHeader::read(&hdr).unwrap_err(),
            Refusal::Damaged("the VB header executable name is not a bounded string")
        );
    }

    #[test]
    fn a_string_with_no_terminator_inside_the_bound_is_refused() {
        let mut bytes = a_synthetic_vb_image();
        // Fill the whole pool with non zero bytes, so no NUL follows the
        // executable name offset inside the window.
        let from = synth::HEADER_OFF + 0x78;
        bytes[from..0x800].fill(b'A');
        let image = PeImage::parse(&bytes).unwrap();
        let hdr = header_region(&image).unwrap();
        assert_eq!(
            VbHeader::read(&hdr).unwrap_err(),
            Refusal::Damaged("the VB header executable name is not a bounded string")
        );
    }
}
