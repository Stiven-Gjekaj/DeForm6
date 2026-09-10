//! The encoding-validating string reader: `VbStr`, `StrEncoding`.
//!
//! The cursor always advances by the declared length, never by however far
//! the string decode happened to read. `03-RESEARCH.md` Pattern 3 and
//! `STRUCTURES.md` section 9.3 both state this rule. It is the one rule in
//! this module that must never be relaxed: a wrong encoding then costs one
//! property, and every property after it in the same block is still read at
//! the right offset. A read that set its own advance would shift every
//! later property in the same block, and nothing would report it.
//!
//! `VbStr` exposes the decoded text and the declared end, and nothing else.
//! There is no method that gives a length or an advance derived from the
//! decoded text: a caller has exactly one cursor to take.
//!
//! Plan 03-05 fills this module. It serves FRM-03.

use crate::error::{Defect, DefectKind, Site};
use crate::read::region::{Off, Region};

/// The encoding a [`VbStr`] read decodes text with.
///
/// `STRUCTURES.md` section 9.3: no public tool reads an encoding flag from
/// the form property stream. `VbStr::read` defaults to
/// [`StrEncoding::Ascii`] and retries once as the other variant when the
/// first attempt does not land.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StrEncoding {
    /// Single-byte. Each byte becomes its own Latin-1 code point.
    Ascii,
    /// Two-byte little-endian code units.
    Utf16,
}

/// A `String`-typed property, read with the declared-length cursor
/// discipline `03-RESEARCH.md` Pattern 3 states.
///
/// This type gives no method that computes a cursor from the decoded text.
/// [`VbStr::declared_end`] is the only advance a caller may take, in every
/// successful case and in every refused case alike.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VbStr {
    text: String,
    declared_end: Off,
}

impl VbStr {
    /// Gives the decoded text. Empty when the read did not land under
    /// either encoding.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Gives the declared end: the start, plus 2 for the length field, plus
    /// the declared length, plus 1 for the trailing null byte.
    ///
    /// This is the only cursor a caller may take. It holds the same value
    /// in every successful case and in every refused case, because the
    /// cursor comes from the file's own declared length, never from how far
    /// the decoder read.
    #[must_use]
    pub const fn declared_end(&self) -> Off {
        self.declared_end
    }

    /// Reads a `String`-typed property at `at`, in `region`, with
    /// `encoding`.
    ///
    /// `at` is relative to `region`'s own base, matching every other
    /// [`Region`] accessor. The declared end is computed first, entirely
    /// from the length field at `at`, before any text byte is read: 2 bytes
    /// for the length field, then the declared length, then 1 byte for the
    /// trailing null. Every step uses [`Off::checked_add`] on a bare `u32`.
    /// `STRUCTURES.md` section 8.5 gives this `2 + n + 1` width for a
    /// `String` payload.
    ///
    /// A declared end that runs past `region`'s own bound, or whose
    /// arithmetic overflows a `u32`, gives a [`Defect`] naming the byte
    /// offset. No allocation is sized from the declared length before that
    /// bound check.
    #[must_use]
    pub fn read(region: &Region<'_>, at: Off, encoding: StrEncoding) -> (Self, Option<Defect>) {
        let offset = region.file_offset(at).map_or(0, Off::get);

        // A length field that does not fit at `at` is read as 0, the same
        // defensive default `vb/controltree.rs::read_control_header` uses
        // for a count field it cannot read. The bound check below still
        // catches the case where `at` itself leaves no room for anything.
        let declared_len = region.u16_le(at).unwrap_or(0);

        let Some(text_start) = at.checked_add(2) else {
            return Self::overflow(offset, declared_len);
        };
        let Some(after_text) = text_start.checked_add(u32::from(declared_len)) else {
            return Self::overflow(offset, declared_len);
        };
        let Some(declared_end) = after_text.checked_add(1) else {
            return Self::overflow(offset, declared_len);
        };

        if declared_end.get() > region.len() {
            let max = region.len().saturating_sub(at.get());
            return Self::region_overflow(offset, declared_len, declared_end, max);
        }

        let text = decode(region, text_start, declared_len, encoding).unwrap_or_default();
        (Self { text, declared_end }, None)
    }

    /// The declared end's own arithmetic overflows a `u32`. There is no
    /// declared end to trust, so this saturates to `Off`'s own maximum: a
    /// caller's own bound check then refuses cleanly rather than reading
    /// past the file.
    fn overflow(offset: u32, declared_len: u16) -> (Self, Option<Defect>) {
        let len = u32::from(declared_len).saturating_add(3);
        let defect = Defect {
            site: Site {
                offset,
                rva: None,
                structure: "VbStr",
                field: "declared_end",
            },
            kind: DefectKind::OffsetOverflow { offset, len },
        };
        (
            Self {
                text: String::new(),
                declared_end: Off::new(u32::MAX),
            },
            Some(defect),
        )
    }

    /// The declared end runs past the end of the enclosing region. No
    /// allocation is sized from `declared_len` before this check runs.
    fn region_overflow(
        offset: u32,
        declared_len: u16,
        declared_end: Off,
        max: u32,
    ) -> (Self, Option<Defect>) {
        let defect = Defect {
            site: Site {
                offset,
                rva: None,
                structure: "VbStr",
                field: "declared_end",
            },
            kind: DefectKind::ImplausibleCount {
                offset,
                count: u32::from(declared_len),
                max,
            },
        };
        (
            Self {
                text: String::new(),
                declared_end,
            },
            Some(defect),
        )
    }
}

/// Decodes `declared_len` bytes at `text_start`, under `encoding`.
///
/// ASCII maps each byte to its own Latin-1 code point, the rule
/// `vb/object.rs::read_name` and `vb/controltree.rs::read_name` both use.
/// `String::from_utf8_lossy` is never used here: a byte in 0x80 to 0xFF
/// would become the replacement character and the text would be lost.
///
/// UTF-16 pairs the bytes little endian. [`Region::take`] is the only route
/// to the bytes, so a `declared_len` that does not fit is refused by
/// `Region` itself, never read from padding.
fn decode(
    region: &Region<'_>,
    text_start: Off,
    declared_len: u16,
    encoding: StrEncoding,
) -> Option<String> {
    match encoding {
        StrEncoding::Ascii => {
            let bytes = region.take(text_start, u32::from(declared_len))?;
            Some(bytes.iter().copied().map(char::from).collect())
        }
        StrEncoding::Utf16 => {
            // The largest even number of bytes at or below `declared_len`,
            // found by clearing the low bit. An odd `declared_len` leaves
            // one byte unpaired; that byte is not consumed here.
            let consumed = u32::from(declared_len) & !1_u32;
            let bytes = region.take(text_start, consumed)?;
            let mut units = Vec::new();
            for pair in bytes.chunks_exact(2) {
                let raw: [u8; 2] = pair.try_into().ok()?;
                units.push(u16::from_le_bytes(raw));
            }
            Some(String::from_utf16_lossy(&units))
        }
    }
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
    use super::{StrEncoding, VbStr};
    use crate::error::DefectKind;
    use crate::read::region::{Off, Region};

    #[test]
    fn a_declared_length_of_zero_gives_an_empty_string_and_a_declared_end_three_bytes_past_the_start()
     {
        let buf = [0x00_u8, 0x00, 0x00];
        let region = Region::new(&buf, Off::new(0));
        let (s, defect) = VbStr::read(&region, Off::new(0), StrEncoding::Ascii);
        assert_eq!(s.text(), "");
        assert_eq!(s.declared_end(), Off::new(3));
        assert!(defect.is_none());
    }

    #[test]
    fn a_declared_length_of_five_gives_a_declared_end_eight_bytes_past_the_start() {
        let mut buf = vec![0x05_u8, 0x00];
        buf.extend_from_slice(b"Hello");
        buf.push(0x00);
        let region = Region::new(&buf, Off::new(0));
        let (s, defect) = VbStr::read(&region, Off::new(0), StrEncoding::Ascii);
        assert_eq!(s.declared_end(), Off::new(8));
        assert!(defect.is_none());
    }

    #[test]
    fn the_same_declared_end_holds_when_the_five_bytes_are_all_0xff() {
        let mut buf = vec![0x05_u8, 0x00];
        buf.extend_from_slice(&[0xFF; 5]);
        buf.push(0x00);
        let region = Region::new(&buf, Off::new(0));
        let (s, _defect) = VbStr::read(&region, Off::new(0), StrEncoding::Ascii);
        assert_eq!(s.declared_end(), Off::new(8));
    }

    #[test]
    fn an_ascii_read_of_the_byte_0xa9_gives_its_own_latin1_code_point() {
        let buf = [0x01_u8, 0x00, 0xA9, 0x00];
        let region = Region::new(&buf, Off::new(0));
        let (s, _defect) = VbStr::read(&region, Off::new(0), StrEncoding::Ascii);
        assert_eq!(s.text(), "\u{A9}");
    }

    #[test]
    fn a_declared_length_larger_than_the_region_gives_a_defect_naming_both_numbers_and_still_gives_the_declared_end()
     {
        // length = 100 (0x64), but only 2 more bytes follow.
        let buf = [0x64_u8, 0x00, 0xAA, 0xAA];
        let region = Region::new(&buf, Off::new(0));
        let (s, defect) = VbStr::read(&region, Off::new(0), StrEncoding::Ascii);
        let defect = defect.expect("a declared length past the region must refuse");
        let message = format!("{}", defect.kind);
        assert!(message.contains("100"), "{message}");
        assert!(message.contains("0x0"), "{message}");
        assert!(matches!(defect.kind, DefectKind::ImplausibleCount { .. }));
        // 0 + 2 (length field) + 100 (declared) + 1 (trailing null) = 103.
        assert_eq!(s.declared_end(), Off::new(103));
        assert_eq!(s.text(), "");
    }

    #[test]
    fn a_region_overflow_defect_names_the_absolute_file_offset_when_the_region_has_a_nonzero_base()
    {
        let buf = [0x64_u8, 0x00, 0xAA, 0xAA];
        let region = Region::new(&buf, Off::new(0x1000));
        let (_s, defect) = VbStr::read(&region, Off::new(0), StrEncoding::Ascii);
        let defect = defect.expect("must refuse");
        let message = format!("{}", defect.kind);
        assert!(message.contains("0x1000"), "{message}");
    }

    #[test]
    fn a_declared_length_whose_end_overflows_a_u32_gives_a_defect_naming_the_offset() {
        let buf = [0_u8; 4];
        let region = Region::new(&buf, Off::new(0));
        let (s, defect) = VbStr::read(&region, Off::new(u32::MAX - 1), StrEncoding::Ascii);
        let defect = defect.expect("an offset this close to u32::MAX must overflow, not wrap");
        assert!(matches!(defect.kind, DefectKind::OffsetOverflow { .. }));
        let message = format!("{}", defect.kind);
        assert!(
            message.contains(&format!("{:#x}", u32::MAX - 1)),
            "{message}"
        );
        assert_eq!(s.declared_end(), Off::new(u32::MAX));
    }

    #[test]
    fn a_utf16_read_pairs_bytes_little_endian() {
        // "AB" as UTF-16LE: 0x41 0x00 0x42 0x00.
        let mut buf = vec![0x04_u8, 0x00];
        buf.extend_from_slice(&[0x41, 0x00, 0x42, 0x00]);
        buf.push(0x00);
        let region = Region::new(&buf, Off::new(0));
        let (s, _defect) = VbStr::read(&region, Off::new(0), StrEncoding::Utf16);
        assert_eq!(s.text(), "AB");
        assert_eq!(s.declared_end(), Off::new(7));
    }

    #[test]
    fn an_unreadable_length_field_defaults_to_a_declared_length_of_zero_and_does_not_panic() {
        // Only one byte in the whole region: the u16 length field itself
        // does not fit at offset 0.
        let buf = [0x00_u8];
        let region = Region::new(&buf, Off::new(0));
        let (s, defect) = VbStr::read(&region, Off::new(0), StrEncoding::Ascii);
        assert!(defect.is_some());
        assert_eq!(s.text(), "");
    }
}
