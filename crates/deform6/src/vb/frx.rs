//! `.frx` resource blob extraction: the inline blob reader, the running
//! offset cursor and the image-signature sniff.
//!
//! Plan 03-07 fills this module. It serves FRM-05.
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

use crate::error::{Defect, DefectKind, Site};
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
    use super::{Blob, extract_blob};
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
}
