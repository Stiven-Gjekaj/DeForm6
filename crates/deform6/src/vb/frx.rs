//! `.frx` resource blob extraction: the inline blob reader, the running
//! offset cursor and the image-signature sniff.
//!
//! Plan 03-07 fills this module. It serves FRM-05. Plan 03-15 corrects
//! [`BlobCursor::take`]'s own advance against a real committed `.frx`,
//! wires [`extract_blob`] into `propstream.rs`'s resource blob arm, and
//! proves both end to end.
//!
//! # The `.frx` offset is not in the executable
//!
//! `STRUCTURES.md` section 8.8 states the single most important fact this
//! module is built around: a bulk property's `.frx` offset is never stored
//! in the compiled executable. It is a running cursor a decompiler
//! synthesises: it starts at 0 for each form and advances by the inline
//! blob's own declared length plus [`FRX_ITEM_HEADER_LEN`] after every
//! blob. [`BlobCursor`] is that cursor, and it is the only place this
//! module computes an offset. `AGENTS.md`'s measurement rule says the same
//! thing in general terms: give the number that can be proved, not one
//! calculated from a part.
//!
//! # Plan 03-15's correction, measured against real `.frx` files
//!
//! `STRUCTURES.md` section 8.8 states the advance is `blobLen + 12`,
//! citing SVBD's own `modFrx`. Plan 03-15 measured eleven real gaps
//! across two committed `.frx` files this repository vendors
//! (`corpus/vb6-code/Game-physics-basic/FormPhysics.frx`, nine gaps, and
//! `corpus/vb6-code/Transparency-2D/frmTransparency.frx`, two gaps) and
//! found the real advance is `blobLen + 4` in all eleven, with the file
//! ending exactly at the last item's own end. `Fast_Flames.exe`'s own
//! inline blob at file offset `0x13d5` is byte for byte identical to the
//! whole of the committed `frmFire.frx`: the length field, the eight byte
//! header and the image bytes, with nothing else in between. A `.frx`
//! item on disk is therefore the four byte length field itself (the same
//! field the executable already carries inline) followed directly by
//! `declared_len` bytes (the eight byte header plus the image): `4 +
//! declared_len` bytes total, never `12 + declared_len`. [`FRX_ITEM_HEADER_LEN`]
//! keeps its name; its value and its doc comment now state what those four
//! bytes are.
//!
//! # This module recovers bytes into memory. It writes no file.
//!
//! `03-RESEARCH.md`'s "Phase boundary note" and this plan's own frontmatter
//! record the reasoning: `inspect` writes nothing to disk (DET-06, phase 1
//! and phase 2), and phase 4's plan 04-04 owns the `.frx` writer, because
//! the `.frm` writer and the `.frx` writer are one component that must
//! share this same cursor.
//!
//! # Never guess a width, never size an allocation from an unchecked length
//!
//! `AGENTS.md`, "The file is hostile": a length field in the file never
//! decides an allocation on its own. [`extract_blob`] checks the declared
//! length against the remaining bytes of the block before it takes any
//! subregion, and it checks the declared length is at least 8 (a checked
//! subtraction, never a plain one) before it computes the image byte count.

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::region::{Off, Region};

/// The four byte little endian length value that means "the property is
/// absent". `STRUCTURES.md` section 8.8.
const ABSENT_LEN: u32 = 0xFFFF_FFFF;

/// The width of the inline picture header every non-absent blob carries,
/// counted by the declared length: `blobLen = imageLen + 8`.
const PICTURE_HEADER_LEN: u32 = 8;

/// One resource blob recovered from the inline property stream.
///
/// The eight header bytes and the image bytes are kept as separate fields,
/// per this plan's own action text: the twelve byte `.frx` item header a
/// writer adds counts the declared length and the declared length minus 8
/// differently, so a caller with only one of the two numbers cannot write
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blob {
    /// The eight inline picture header bytes, exactly as the file holds
    /// them.
    pub header: [u8; 8],
    /// The image bytes: `declared_len - 8` bytes, the checked subtraction
    /// [`extract_blob`] already performed.
    pub image: Vec<u8>,
    /// The blob's own declared length (`blobLen`), read from the file:
    /// `image.len()` plus [`PICTURE_HEADER_LEN`]. Carried on the blob so
    /// [`BlobCursor::take`] never has to recompute it from `image.len()`.
    pub declared_len: u32,
    /// The absolute file offset the blob's own length field was read from.
    pub offset: u32,
}

/// Builds the [`Site`] every [`extract_blob`] defect uses.
const fn site(offset: u32) -> Site {
    Site {
        offset,
        rva: None,
        structure: "Blob",
        field: "blobLen",
    }
}

/// Gives `end`, refusing when `start + width` would run past `block_end` or
/// past the end of a `u32`.
fn ends_within(start: u32, width: u32, block_end: u32) -> Option<u32> {
    let end = start.checked_add(width)?;
    if end > block_end { None } else { Some(end) }
}

/// Reads one inline resource blob at `at`, bounded by `block`.
///
/// `block` is a [`Region`] already bounded to the control block the blob's
/// length field sits in; `at` is the window-relative offset of that four
/// byte length field. Gives `(blob, consumed, defect)`: the blob when there
/// is one, the byte count [`walk_properties`] should advance its own cursor
/// by, and a [`Defect`] naming the byte offset when the read could not
/// complete.
///
/// Absent (`0xFFFFFFFF`) is a normal state, not a fault: it consumes 4
/// bytes, gives no blob, and gives no defect.
///
/// [`walk_properties`]: crate::vb::propstream::walk_properties
#[must_use]
pub fn extract_blob(block: &Region<'_>, at: Off) -> (Option<Blob>, u32, Option<Defect>) {
    let offset = block.file_offset(at).map_or(0, Off::get);
    let block_end = block.len();

    // Bound the length field's own four bytes before reading them. A
    // crafted `at` near the block's own end must not read past it.
    let Some(after_len) = ends_within(at.get(), 4, block_end) else {
        let kind = DefectKind::ImplausibleCount {
            offset,
            count: 4,
            max: block_end.saturating_sub(at.get()),
        };
        return (
            None,
            0,
            Some(Defect {
                site: site(offset),
                kind,
            }),
        );
    };

    // `ends_within` above already proved these four bytes sit inside
    // `block`, so this read cannot fail against a well-formed `Region`.
    let Some(blob_len) = block.u32_le(at) else {
        let kind = DefectKind::ImplausibleCount {
            offset,
            count: 4,
            max: 0,
        };
        return (
            None,
            0,
            Some(Defect {
                site: site(offset),
                kind,
            }),
        );
    };

    if blob_len == ABSENT_LEN {
        return (None, 4, None);
    }

    // Check the declared length against the remaining bytes of the block
    // before any subregion is taken and before any allocation is sized
    // from it. A crafted value of 0xFFFFFFFE must never reach a take() at
    // all, so it can never ask for four gigabytes.
    let Some(payload_end) = ends_within(after_len, blob_len, block_end) else {
        let overflowed = after_len.checked_add(blob_len).is_none();
        let kind = if overflowed {
            DefectKind::OffsetOverflow {
                offset,
                len: blob_len,
            }
        } else {
            DefectKind::ImplausibleCount {
                offset,
                count: blob_len,
                max: block_end.saturating_sub(after_len),
            }
        };
        return (
            None,
            4,
            Some(Defect {
                site: site(offset),
                kind,
            }),
        );
    };

    // Only now, with the declared length bound-checked against the real
    // file, decide whether it can hold its own eight byte header.
    // `checked_sub` is the check itself: a declared length below 8 gives
    // `None` here rather than wrapping to a very large number that a later
    // bound check would then run against the wrong value.
    let Some(image_len) = blob_len.checked_sub(PICTURE_HEADER_LEN) else {
        let kind = DefectKind::BlobLenTooSmall { offset, blob_len };
        return (
            None,
            4,
            Some(Defect {
                site: site(offset),
                kind,
            }),
        );
    };

    let header: [u8; 8] = block
        .take(Off::new(after_len), PICTURE_HEADER_LEN)
        .and_then(|bytes| bytes.try_into().ok())
        .unwrap_or([0u8; 8]);

    let image = after_len
        .checked_add(PICTURE_HEADER_LEN)
        .and_then(|image_start| block.take(Off::new(image_start), image_len))
        .map(<[u8]>::to_vec)
        .unwrap_or_default();

    let consumed = payload_end.saturating_sub(at.get());

    let blob = Blob {
        header,
        image,
        declared_len: blob_len,
        offset,
    };
    (Some(blob), consumed, None)
}

/// The width of the `.frx` item's own four byte length field: the one part
/// of an item on disk that `declared_len` does not itself count.
///
/// `STRUCTURES.md` section 8.8 names this quantity `12`, the three
/// `FRXITEMHDR` fields (`dwSizeImageEx`, `dwKey`, `dwSizeImage`), crediting
/// Brad Martinez through SVBD's own `modFrx`. Plan 03-15 measured eleven
/// real gaps across two committed `.frx` files this repository vendors and
/// found the real value is `4`, not `12`: a `.frx` item on disk is the same
/// four byte length field the executable already carries inline, followed
/// directly by `declared_len` bytes (the eight byte picture header plus the
/// image), with no separate twelve byte header in between. See the module
/// doc comment for the full measurement and the corpus files it names.
pub const FRX_ITEM_HEADER_LEN: u32 = 4;

/// The running `.frx` offset cursor.
///
/// `STRUCTURES.md` section 8.8: the `.frx` offset is a cursor a decompiler
/// synthesises, never a value the file stores. There is exactly one cursor
/// and exactly one place the offset is computed, in [`BlobCursor::take`]:
/// the `.frm` writer and the `.frx` writer are one component (phase 4's
/// plan 04-04), and any independent computation of an offset drifts.
/// `AGENTS.md`'s measurement rule states the same thing in general terms:
/// give the number that can be proved, not one calculated from a part.
///
/// A form's own cursor starts at 0 and never carries over from a form read
/// before it: two forms in one program each get their own `.frx` file in
/// phase 4, so a shared cursor would put the second form's first blob at a
/// non-zero offset in a file that holds nothing before it. Build a fresh
/// [`BlobCursor::new`] per form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlobCursor {
    offset: u32,
}

impl BlobCursor {
    /// Starts a fresh cursor at offset 0.
    #[must_use]
    pub const fn new() -> Self {
        Self { offset: 0 }
    }

    /// Gives the offset this blob's own `.frx` item would start at, then
    /// advances the cursor by `blob`'s own declared length plus
    /// [`FRX_ITEM_HEADER_LEN`], the one place this module computes an
    /// offset.
    ///
    /// # Errors
    ///
    /// Refuses, naming the running offset, when the advance would overflow
    /// a `u32`: a form whose blobs sum past a `u32` cannot produce a valid
    /// `.frx` offset at all.
    pub fn take(&mut self, blob: &Blob) -> Result<u32, Refusal> {
        let current = self.offset;
        let Some(next) = current
            .checked_add(blob.declared_len)
            .and_then(|v| v.checked_add(FRX_ITEM_HEADER_LEN))
        else {
            return Err(damaged(format!(
                "the .frx offset cursor at {current} overflows a u32 advancing past this blob"
            )));
        };
        self.offset = next;
        Ok(current)
    }
}

/// Builds a [`Refusal::Damaged`] whose message is computed at runtime.
///
/// The same escape hatch `vb/gui.rs::damaged` and `vb/controltree.rs::damaged`
/// document: `Refusal::Damaged` takes `&'static str`, and this module's own
/// required refusal (a running offset that overflows a `u32`) must name a
/// value computed at run time. Every path that reaches this function is
/// already fatal to the whole form's own blob recovery.
fn damaged(message: String) -> Refusal {
    let leaked: &'static str = Box::leak(message.into_boxed_str());
    Refusal::Damaged(leaked)
}

/// A resource blob's container format, detected from its own first bytes.
///
/// `STRUCTURES.md` section 8.8: the executable records no format tag for a
/// blob, so the format is detected the same way any other tool detects an
/// unlabelled image: from a short signature at the start of the bytes, or
/// at a fixed offset for EMF. DeForm6 never decodes the image; it
/// classifies the container and copies the blob verbatim, so an unknown
/// format costs nothing and a wrong guess would cost the blob. This follows
/// `vb/classify.rs`'s own discipline: a name for a signature this module
/// recognises, the raw bytes for one it does not, and no refusal either
/// way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    /// `42 4D`.
    Bmp,
    /// `47 49 46`.
    Gif,
    /// `FF D8`.
    Jpeg,
    /// `D7 CD`, the Aldus placeable WMF key.
    Wmf,
    /// `20 45 4D 46` at byte offset 40.
    Emf,
    /// `00 00 01 00`.
    Ico,
    /// `00 00 02 00`.
    Cur,
    /// No known signature matched. Carries the first bytes the blob held,
    /// up to 4, so the report can show what was actually there without
    /// naming a format nobody proved.
    Unknown(Vec<u8>),
}

/// Tells whether `image` holds `signature` at byte offset `at`.
///
/// Reads through [`Region::take`], so a blob shorter than `at + signature.len()`
/// gives `false` rather than a panic: this is "reads no byte past the end"
/// for every signature check below.
fn signature_at(image: &[u8], at: u32, signature: &[u8]) -> bool {
    let region = Region::new(image, Off::new(0));
    let len = u32::try_from(signature.len()).unwrap_or(0);
    region.take(Off::new(at), len) == Some(signature)
}

/// Detects `image`'s own container format from its first bytes, per
/// [`ImageFormat`]'s own doc comment. Never reads past the end of `image`.
#[must_use]
pub fn sniff_format(image: &[u8]) -> ImageFormat {
    if signature_at(image, 0, &[0x42, 0x4D]) {
        return ImageFormat::Bmp;
    }
    if signature_at(image, 0, &[0x47, 0x49, 0x46]) {
        return ImageFormat::Gif;
    }
    if signature_at(image, 0, &[0xFF, 0xD8]) {
        return ImageFormat::Jpeg;
    }
    if signature_at(image, 0, &[0xD7, 0xCD]) {
        return ImageFormat::Wmf;
    }
    if signature_at(image, 0, &[0x00, 0x00, 0x01, 0x00]) {
        return ImageFormat::Ico;
    }
    if signature_at(image, 0, &[0x00, 0x00, 0x02, 0x00]) {
        return ImageFormat::Cur;
    }
    if signature_at(image, 40, &[0x20, 0x45, 0x4D, 0x46]) {
        return ImageFormat::Emf;
    }
    let region = Region::new(image, Off::new(0));
    let prefix_len = region.len().min(4);
    let prefix = region
        .take(Off::new(0), prefix_len)
        .map(<[u8]>::to_vec)
        .unwrap_or_default();
    ImageFormat::Unknown(prefix)
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
    use super::{Blob, BlobCursor, FRX_ITEM_HEADER_LEN, ImageFormat, extract_blob, sniff_format};
    use crate::read::region::{Off, Region};

    // --- Task 1: the inline blob and its bounds --------------------------

    #[test]
    fn a_length_of_0xffffffff_consumes_four_bytes_gives_no_blob_and_no_defect() {
        let bytes = ABSENT_LEN_BYTES.to_vec();
        let region = Region::new(&bytes, Off::new(0));
        let (blob, consumed, defect) = extract_blob(&region, Off::new(0));
        assert!(blob.is_none());
        assert_eq!(consumed, 4);
        assert!(defect.is_none(), "{defect:?}");
    }

    const ABSENT_LEN_BYTES: [u8; 4] = 0xFFFF_FFFF_u32.to_le_bytes();

    #[test]
    fn a_length_of_eight_gives_a_blob_with_zero_image_bytes_and_consumes_twelve_bytes() {
        // The deleted form icon shape STRUCTURES.md section 8.8 gives as a
        // concrete example: `08 00 00 00 6C 74 00 00 00 00 00 00`.
        let mut bytes = 8_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x6C, 0x74, 0x00, 0x00]);
        let region = Region::new(&bytes, Off::new(0));
        let (blob, consumed, defect) = extract_blob(&region, Off::new(0));
        assert!(defect.is_none(), "{defect:?}");
        let blob: Blob = blob.expect("a length of 8 must give a blob");
        assert_eq!(blob.image.len(), 0);
        assert_eq!(consumed, 12);
        assert_eq!(
            blob.header,
            [0x08, 0x00, 0x00, 0x00, 0x6C, 0x74, 0x00, 0x00]
        );
        assert_eq!(blob.declared_len, 8);
    }

    #[test]
    fn a_length_of_108_gives_a_blob_with_100_image_bytes_and_consumes_112_bytes() {
        let mut bytes = 108_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0xAA; 8]);
        bytes.extend(std::iter::repeat_n(0xBB_u8, 100));
        let region = Region::new(&bytes, Off::new(0));
        let (blob, consumed, defect) = extract_blob(&region, Off::new(0));
        assert!(defect.is_none(), "{defect:?}");
        let blob = blob.unwrap();
        assert_eq!(blob.image.len(), 100);
        assert_eq!(consumed, 112);
        assert!(blob.image.iter().all(|&b| b == 0xBB));
        assert_eq!(blob.header, [0xAA; 8]);
    }

    #[test]
    fn a_length_of_three_gives_a_defect_naming_the_value_and_the_offset() {
        // Three padding bytes follow the length field, so the block holds
        // enough room for a declared length of 3: this test exercises the
        // "too small to hold its own header" refusal on its own, distinct
        // from "the declared length exceeds the remaining bytes" refusal
        // the next test below covers.
        let mut bytes = 3_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0, 0, 0]);
        let region = Region::new(&bytes, Off::new(0x2000));
        let (blob, consumed, defect) = extract_blob(&region, Off::new(0));
        assert!(blob.is_none());
        assert_eq!(consumed, 4);
        let defect = defect.expect("a length of 3 cannot hold its own 8 byte header");
        let message = format!("{}", defect.kind);
        assert!(message.contains('3'), "{message}");
        assert!(message.contains("0x2000"), "{message}");
    }

    #[test]
    fn a_length_of_0xfffffffe_gives_a_defect_and_attempts_no_four_gigabyte_allocation() {
        let bytes = 0xFFFF_FFFE_u32.to_le_bytes().to_vec();
        let region = Region::new(&bytes, Off::new(0));
        let (blob, _consumed, defect) = extract_blob(&region, Off::new(0));
        assert!(blob.is_none());
        assert!(defect.is_some());
        // Reaching this assertion at all, without the process hanging or
        // aborting on an allocation, is the proof: `extract_blob` returned
        // rather than sizing a `Vec` from 0xFFFFFFFE.
    }

    #[test]
    fn a_declared_length_larger_than_the_remaining_bytes_of_the_block_gives_a_defect() {
        let mut bytes = 1000_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0; 8]); // only 8 bytes actually remain
        let region = Region::new(&bytes, Off::new(0x1000));
        let (blob, consumed, defect) = extract_blob(&region, Off::new(0));
        assert!(blob.is_none());
        assert_eq!(consumed, 4);
        let defect = defect.expect("1000 exceeds the 8 bytes that remain");
        let message = format!("{}", defect.kind);
        assert!(message.contains("1000"), "{message}");
        assert!(message.contains("0x1000"), "{message}");
    }

    #[test]
    fn a_declared_length_whose_end_overflows_a_u32_gives_a_defect_naming_the_offset() {
        let mut bytes = (u32::MAX - 2).to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0u8; 8]);
        let region = Region::new(&bytes, Off::new(0x3000));
        let (blob, _consumed, defect) = extract_blob(&region, Off::new(0));
        assert!(blob.is_none());
        let defect = defect.expect("an overflowing end must refuse");
        let message = format!("{}", defect.kind);
        assert!(message.contains("0x3000"), "{message}");
    }

    #[test]
    fn extract_blob_at_a_nonzero_offset_names_the_absolute_file_offset_in_a_defect() {
        let mut bytes = vec![0xEE; 5];
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        let region = Region::new(&bytes, Off::new(0x9000));
        let (blob, _consumed, defect) = extract_blob(&region, Off::new(5));
        assert!(blob.is_none());
        let defect = defect.expect("length 3 refuses");
        let message = format!("{}", defect.kind);
        // 0x9000 + 5 = 0x9005.
        assert!(message.contains("0x9005"), "{message}");
    }

    #[test]
    fn a_declared_length_that_lands_exactly_on_the_block_end_is_accepted() {
        let mut bytes = 20_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0xCC; 8]);
        bytes.extend_from_slice(&[0xDD; 12]);
        let region = Region::new(&bytes, Off::new(0));
        let (blob, consumed, defect) = extract_blob(&region, Off::new(0));
        assert!(defect.is_none(), "{defect:?}");
        let blob = blob.unwrap();
        assert_eq!(blob.image.len(), 12);
        assert_eq!(consumed, 24);
    }

    // --- Task 2: the running offset cursor and the image format sniff ----

    /// Builds a [`Blob`] with the given declared length; every other field
    /// is a synthetic placeholder, since `BlobCursor::take` reads only
    /// `declared_len`.
    fn a_blob(declared_len: u32) -> Blob {
        Blob {
            header: [0u8; 8],
            image: Vec::new(),
            declared_len,
            offset: 0,
        }
    }

    #[test]
    fn frx_item_header_len_is_four() {
        assert_eq!(FRX_ITEM_HEADER_LEN, 4);
    }

    #[test]
    fn a_fresh_cursor_starts_at_zero() {
        let mut cursor = BlobCursor::new();
        let offset = cursor.take(&a_blob(8)).unwrap();
        assert_eq!(offset, 0);
    }

    #[test]
    fn three_blobs_of_length_8_108_and_8_give_offsets_0_12_and_124() {
        let mut cursor = BlobCursor::new();
        let first = cursor.take(&a_blob(8)).unwrap();
        let second = cursor.take(&a_blob(108)).unwrap();
        let third = cursor.take(&a_blob(8)).unwrap();
        assert_eq!(
            [first, second, third],
            [0, 12, 124],
            "the plus 4 (the length field's own width) must apply once per blob, not once per \
             form; see the module doc comment's eleven-gap measurement"
        );
    }

    #[test]
    fn a_new_form_starts_its_cursor_at_zero_after_a_previous_form_advanced_it() {
        let mut first_form = BlobCursor::new();
        first_form.take(&a_blob(108)).unwrap();
        assert_ne!(first_form.take(&a_blob(8)).unwrap(), 0);

        let mut second_form = BlobCursor::new();
        let offset = second_form.take(&a_blob(8)).unwrap();
        assert_eq!(
            offset, 0,
            "a second form's own cursor must not carry over the first form's"
        );
    }

    #[test]
    fn take_gives_a_refusal_naming_the_offset_when_the_advance_overflows_a_u32() {
        let mut cursor = BlobCursor::new();
        // The first take leaves the cursor at exactly u32::MAX: a valid
        // offset, right at the edge. The second take's own advance (0 plus
        // the length field's own 4 bytes) then overflows.
        cursor
            .take(&a_blob(u32::MAX - FRX_ITEM_HEADER_LEN))
            .unwrap();
        let err = cursor.take(&a_blob(0)).unwrap_err();
        let message = format!("{err}");
        assert!(!message.is_empty());
    }

    #[test]
    fn sniff_format_recognises_each_of_the_six_first_byte_signatures() {
        assert_eq!(sniff_format(&[0x42, 0x4D, 0, 0]), ImageFormat::Bmp);
        assert_eq!(sniff_format(&[0x47, 0x49, 0x46, 0]), ImageFormat::Gif);
        assert_eq!(sniff_format(&[0xFF, 0xD8, 0, 0]), ImageFormat::Jpeg);
        assert_eq!(sniff_format(&[0xD7, 0xCD, 0, 0]), ImageFormat::Wmf);
        assert_eq!(sniff_format(&[0x00, 0x00, 0x01, 0x00]), ImageFormat::Ico);
        assert_eq!(sniff_format(&[0x00, 0x00, 0x02, 0x00]), ImageFormat::Cur);
    }

    #[test]
    fn sniff_format_finds_the_emf_signature_at_offset_forty() {
        let mut image = vec![0u8; 40];
        image.extend_from_slice(&[0x20, 0x45, 0x4D, 0x46]);
        assert_eq!(sniff_format(&image), ImageFormat::Emf);
    }

    #[test]
    fn sniff_format_on_an_unrecognised_prefix_gives_unknown_and_carries_the_bytes() {
        let image = [0x99, 0x88, 0x77, 0x66];
        assert_eq!(
            sniff_format(&image),
            ImageFormat::Unknown(vec![0x99, 0x88, 0x77, 0x66])
        );
    }

    #[test]
    fn sniff_format_on_a_blob_of_zero_image_bytes_gives_unknown_with_no_panic() {
        assert_eq!(sniff_format(&[]), ImageFormat::Unknown(Vec::new()));
    }

    #[test]
    fn sniff_format_on_a_blob_shorter_than_the_signature_it_needs_reads_no_byte_past_the_end() {
        // 20 bytes: far short of the 44 the EMF signature needs (offset 40
        // plus its own 4 bytes). Must not panic and must not falsely match.
        let image = vec![0u8; 20];
        assert_ne!(sniff_format(&image), ImageFormat::Emf);
    }

    #[test]
    fn a_zero_length_blob_still_advances_the_cursor_by_the_item_header_alone() {
        // A short blob still round-trips through the cursor: this proves
        // BlobCursor::take reads declared_len off the Blob it is given,
        // not a constant.
        let mut cursor = BlobCursor::new();
        let offset = cursor.take(&a_blob(0)).unwrap();
        assert_eq!(offset, 0);
        let expected_second_offset = FRX_ITEM_HEADER_LEN;
        assert_eq!(cursor.take(&a_blob(0)).unwrap(), expected_second_offset);
    }

    // --- Plan 03-15, Task 1: the cursor against two committed .frx files --

    /// Gives the absolute path to a file under this repository's own
    /// vendored `corpus/`.
    fn corpus_path(relative: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus")
            .join(relative)
    }

    /// Parses every `"name.frx":OFFSET` (or `$"name.frx":OFFSET`) hex
    /// offset a committed `.frm` declares, in the order the file's own
    /// lines give them.
    ///
    /// Reads the file at run time. This function commits no table of
    /// offsets: it reads the same committed `.frm` bytes every run, and a
    /// caller that changed the corpus would see this parse differently
    /// rather than see a stale literal disagree with it.
    fn declared_frx_offsets(frm_relative: &str) -> Vec<u32> {
        let bytes = std::fs::read(corpus_path(frm_relative)).expect("reading the committed .frm");
        let text: String = bytes.iter().copied().map(char::from).collect();
        let mut offsets = Vec::new();
        for line in text.lines() {
            let Some(colon) = line.find(".frx\":") else {
                continue;
            };
            let hex = &line[colon + 6..];
            let hex: String = hex.chars().take_while(char::is_ascii_hexdigit).collect();
            if let Ok(offset) = u32::from_str_radix(&hex, 16) {
                offsets.push(offset);
            }
        }
        offsets
    }

    /// Drives every declared offset of one committed `.frx` file through a
    /// fresh [`BlobCursor`], reading the declared length the real file
    /// holds at each declared offset, and asserts the cursor reproduces the
    /// whole sequence exactly.
    ///
    /// Both files are read at run time, per `AGENTS.md`'s rule that a
    /// derived fixture from a corpus binary never enters the repository:
    /// this function commits no length table and no hash computed from
    /// either file, only the two relative paths that name them.
    fn assert_cursor_reproduces_declared_offsets(frm_relative: &str, frx_relative: &str) {
        let declared_offsets = declared_frx_offsets(frm_relative);
        assert!(
            declared_offsets.len() >= 2,
            "{frm_relative} must declare at least two resource offsets for this test to prove \
             anything about the gaps between them"
        );
        let frx_bytes =
            std::fs::read(corpus_path(frx_relative)).expect("reading the committed .frx");

        let mut cursor = BlobCursor::new();
        let mut got = Vec::new();
        for &offset in &declared_offsets {
            let at = usize::try_from(offset).expect("a real .frx offset fits in usize");
            let declared_len = u32::from_le_bytes(
                frx_bytes[at..at + 4]
                    .try_into()
                    .expect("the committed .frx holds 4 bytes at every declared offset"),
            );
            got.push(cursor.take(&a_blob(declared_len)).unwrap());
        }
        assert_eq!(
            got, declared_offsets,
            "{frm_relative}'s own declared offset sequence must round-trip through the cursor \
             exactly; see the module doc comment for how this was measured"
        );
    }

    /// `corpus/vb6-code/Game-physics-basic/FormPhysics.frm` declares ten
    /// resource offsets into `FormPhysics.frx`. Nine real gaps between them,
    /// each measured by hand while this plan was written, are `declared
    /// length + 4`.
    #[test]
    fn the_cursor_reproduces_form_physics_frxs_own_ten_declared_offsets() {
        assert_cursor_reproduces_declared_offsets(
            "vb6-code/Game-physics-basic/FormPhysics.frm",
            "vb6-code/Game-physics-basic/FormPhysics.frx",
        );
    }

    /// `corpus/vb6-code/Transparency-2D/frmTransparency.frm` declares three
    /// resource offsets into `frmTransparency.frx`, an independent second
    /// sample from a different corpus program.
    #[test]
    fn the_cursor_reproduces_frm_transparencys_own_three_declared_offsets() {
        assert_cursor_reproduces_declared_offsets(
            "vb6-code/Transparency-2D/frmTransparency.frm",
            "vb6-code/Transparency-2D/frmTransparency.frx",
        );
    }
}
