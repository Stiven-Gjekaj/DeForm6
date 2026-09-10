//! What went wrong, where it went wrong, and how bad it is.
//!
//! The model has two levels.
//!
//! A [`Defect`] is what the parser found inside the file. It carries a
//! [`Site`], which names the byte offset and the field, and a [`DefectKind`],
//! which names what the code expected to find there. `AGENTS.md` requires
//! both, so the message lives next to the variant and there is no second
//! `match` that can drift away from it.
//!
//! A [`Refusal`] is what the caller of the library gets back. It names one of
//! the outcomes in the locked exit code table and nothing more. A refusal
//! sentence holds no byte offset and no path. The offset belongs to a
//! `Defect`, which is the evidence field of the report. The path belongs to
//! the command line, which is the only part of the system that has one.

/// Where a problem is, and what the code wanted to find there.
///
/// `PartialEq` and `Eq` are derived so a [`Defect`] can sit inside [`Report`],
/// which itself derives `PartialEq` for the `Result<Report, Refusal>`
/// comparison the phase 1 test suite needs. `WINDOWS.md` finding 3 records
/// that this derive was the reason `inspect` used to drop the defects it
/// collected; every field here is plain data (a number or static text), so
/// the derive costs nothing.
///
/// [`Report`]: crate::vb::Report
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Site {
    /// The absolute file offset. `Region::file_offset` gives this value.
    pub offset: u32,
    /// The address the offset came from, when it came from one.
    pub rva: Option<u32>,
    /// The structure that is being read, such as `"VbHeader"`.
    pub structure: &'static str,
    /// The field that is being read, such as `"lpProjectData"`.
    pub field: &'static str,
}

/// What the parser found, and what it expected instead.
///
/// Every message names the byte offset in hexadecimal. A person who reads a
/// defect opens the file at that offset and sees the same bytes.
///
/// `PartialEq` and `Eq` are derived for the same reason [`Site`] derives
/// them: every variant holds plain data, and the derive is what lets
/// [`Defect`], and in turn [`Report`], compare with `assert_eq!`.
///
/// [`Report`]: crate::vb::Report
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, thiserror::Error)]
pub enum DefectKind {
    /// A signature does not hold the bytes the format requires.
    #[error("expected {expected} at offset {offset:#x}, found {found:#x}")]
    BadMagic {
        /// The absolute file offset of the signature.
        offset: u32,
        /// The signature the format requires, such as `"VB5!"`.
        expected: &'static str,
        /// The value that is there instead.
        found: u32,
    },

    /// An offset plus a length leaves the range a `u32` holds.
    #[error("offset {offset:#x} plus length {len:#x} overflows a u32")]
    OffsetOverflow {
        /// The absolute file offset the sum started from.
        offset: u32,
        /// The length that was added to it.
        len: u32,
    },

    /// An offset points outside the bytes the caller gave the library.
    #[error("offset {offset:#x} is past the end of the {file_len} byte file")]
    PastEndOfFile {
        /// The absolute file offset that is out of range.
        offset: u32,
        /// The real length of the byte slice.
        file_len: u64,
    },

    /// A count field asks for more items than the file holds.
    #[error("count {count} at offset {offset:#x} exceeds the {max} that the file can hold")]
    ImplausibleCount {
        /// The absolute file offset of the count field.
        offset: u32,
        /// The count the file asks for.
        count: u32,
        /// The largest count the real length of the file allows.
        max: u32,
    },

    /// An address falls in no section, so it maps to no file offset.
    #[error("address {va:#x} at offset {offset:#x} is in no section")]
    UnmappedAddress {
        /// The absolute file offset of the pointer that held the address.
        offset: u32,
        /// The address that maps nowhere.
        va: u32,
    },

    /// Two sections claim the same bytes of the file.
    #[error(
        "the section header at offset {offset:#x} overlaps the section that starts at {other:#x}"
    )]
    SectionOverlap {
        /// The absolute file offset of the second section header.
        offset: u32,
        /// The file offset the first section starts at.
        other: u32,
    },

    /// A string runs to the end of its bounded window with no terminator.
    #[error(
        "the text at offset {offset:#x} has no nul terminator in the {limit} bytes that follow"
    )]
    NoNulTerminator {
        /// The absolute file offset the text starts at.
        offset: u32,
        /// The number of bytes that were searched.
        limit: u32,
    },

    /// Two count fields that must agree do not agree.
    #[error(
        "count {count} at offset {offset:#x} does not match the {expected} that {other_field} gives"
    )]
    CountMismatch {
        /// The absolute file offset of the count that was read.
        offset: u32,
        /// The count that was read.
        count: u32,
        /// The count the other field gives.
        expected: u32,
        /// The name of the other field, such as `"wTotalObjects"`.
        other_field: &'static str,
    },

    /// A pointer inside one item resolves to nothing.
    ///
    /// This is not the one spine pointer that reaches the item, which
    /// [`DefectKind::UnmappedAddress`] already covers as fatal. This is a
    /// leaf pointer inside an item that has already been reached: the item
    /// keeps every other field, and the value this pointer would have named
    /// is empty rather than invented.
    #[error(
        "address {va:#x} at offset {offset:#x} resolves to nothing, and the item keeps its other fields"
    )]
    UnreadablePointer {
        /// The absolute file offset of the pointer that held the address.
        offset: u32,
        /// The address that maps nowhere.
        va: u32,
    },

    /// A length-prefixed name field declares a length of zero.
    ///
    /// Plan 03-04: a control's declared name length can legitimately be
    /// zero. The control keeps every other field; only the name is empty.
    #[error("the name at offset {offset:#x} has a declared length of zero")]
    EmptyName {
        /// The absolute file offset of the control block that holds the
        /// name.
        offset: u32,
    },

    /// A two-byte field's high byte carries a value the corpus has never
    /// proven meaningful.
    ///
    /// Plan 03-04: the control array `Index` field is read as two bytes,
    /// defensively, per `03-RESEARCH.md` assumption A4. No corpus index
    /// exceeds 24, so the high byte has never been observed non-zero. This
    /// surfaces the case rather than deciding it; the low-byte-derived value
    /// is still used.
    #[error(
        "the array index at offset {offset:#x} carries a non-zero high byte {high:#x}, which no corpus sample proves meaningful"
    )]
    IndexHighByteSet {
        /// The absolute file offset of the control block that holds the
        /// index.
        offset: u32,
        /// The high byte of the two-byte index value.
        high: u8,
    },
}

/// How bad a defect is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
pub enum Severity {
    /// The file is not what it claims. Nothing downstream is meaningful.
    Fatal,
    /// One item is unreadable. The rest of the graph still stands.
    Recoverable,
}

impl DefectKind {
    /// Severity is a property of the defect, and it is decided in one place.
    ///
    /// The `match` has one arm for each variant and no wildcard arm. A new
    /// variant therefore fails to compile until somebody decides its
    /// severity. A wildcard arm lets a new variant take a default in silence.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        match *self {
            // The file says it is one thing and it is another thing. The
            // spine of the structure graph starts here, so nothing after it
            // means anything.
            Self::BadMagic { .. } => Severity::Fatal,
            // The file describes a range that cannot exist. Every later
            // bound check rests on that range.
            Self::OffsetOverflow { .. } => Severity::Fatal,
            // The file points outside itself. There are no bytes to read.
            Self::PastEndOfFile { .. } => Severity::Fatal,
            // A spine pointer that maps nowhere stops the walk.
            Self::UnmappedAddress { .. } => Severity::Fatal,
            // A count is a leaf. The parser reads the items it reaches and
            // the rest of the graph still stands.
            Self::ImplausibleCount { .. } => Severity::Recoverable,
            // An overlap makes one address ambiguous. DeForm6 owns one
            // predicate and picks one section, so the walk continues.
            Self::SectionOverlap { .. } => Severity::Recoverable,
            // One string is unreadable. The item that holds it keeps its
            // other fields.
            Self::NoNulTerminator { .. } => Severity::Recoverable,
            // Two counts disagree. The parser takes the smaller one and
            // reports the disagreement.
            Self::CountMismatch { .. } => Severity::Recoverable,
            // A leaf pointer inside an item, not the one spine pointer that
            // reaches the item. The item that holds it keeps its other
            // fields, so the walk that found it continues.
            Self::UnreadablePointer { .. } => Severity::Recoverable,
            // A control with no name still carries its type. The tree keeps
            // the control.
            Self::EmptyName { .. } => Severity::Recoverable,
            // The corpus has never proven a non-zero high byte meaningful,
            // but the value is still used. The control keeps every field.
            Self::IndexHighByteSet { .. } => Severity::Recoverable,
        }
    }
}

/// One problem in the file: where it is, and what it is.
///
/// This value is the error and the evidence at the same time. The Phase 4
/// report serialises the value that the failure message prints, so the two do
/// not drift apart.
///
/// `PartialEq` and `Eq` are derived so [`Report`] can carry a `Vec<Defect>`
/// and still derive `PartialEq` itself. `WINDOWS.md` finding 3: this is the
/// fix. Before this derive existed, `inspect` collected defects and had
/// nowhere to put them, because `Report` needed to compare and `Defect` could
/// not.
///
/// [`Report`]: crate::vb::Report
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, thiserror::Error)]
#[error("{site:?}: {kind}")]
pub struct Defect {
    /// Where the problem is.
    pub site: Site,
    /// What the problem is.
    pub kind: DefectKind,
}

/// The error that the parsing layer of the library returns.
///
/// The library never opens a file, so there is no input or output variant
/// here. The library takes a byte slice. The command line crate owns the file
/// system and maps its own errors to exit code 5.
#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    /// A defect stopped the read. The defect says where and why.
    #[error("refused: {0}")]
    Refused(Defect),
    /// The file is a portable executable and it is not Visual Basic 6.
    #[error("not a VB6 executable: {0}")]
    NotVb6(&'static str),
    /// The file uses the Visual Basic 5 runtime.
    #[error("this is a VB5 executable, which DeForm6 does not read")]
    IsVb5,
}

/// Why DeForm6 does not read this file.
///
/// This is the public outcome type. Plan 01-08 maps each variant to one of
/// the locked exit codes, and prints the one sentence below it.
///
/// The full set is defined now. `Damaged` is not reachable from the command
/// line until Phase 5 adds `--salvage`. The set is complete so that the
/// numbering never moves.
///
/// | Variant | Exit code |
/// |---|---|
/// | `NotPe` | 1 |
/// | `NotI386` | 1 |
/// | `NotPe32` | 1 |
/// | `NoVbRuntime` | 2 |
/// | `IsVb5` | 3 |
/// | `IsVb4` | 3 |
/// | `Damaged` | 4 |
///
/// Each sentence is one line. It holds no byte offset and no path.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum Refusal {
    /// The bytes are not a portable executable at all.
    #[error("this file is not a portable executable")]
    NotPe,
    /// The image is for another processor.
    #[error(
        "this portable executable is for another processor, and DeForm6 reads i386 images only"
    )]
    NotI386,
    /// The image is not a 32 bit image.
    #[error("this portable executable is not a 32 bit image, and DeForm6 reads 32 bit images only")]
    NotPe32,
    /// The image imports no Visual Basic runtime.
    ///
    /// The flag gives the reader a better sentence when the image is a .NET
    /// assembly. It stays one variant and one exit code, because the exit
    /// code table is locked.
    #[error("{}", if *dot_net {
        "this portable executable is a .NET assembly, and it holds no Visual Basic runtime"
    } else {
        "this portable executable holds no Visual Basic runtime"
    })]
    NoVbRuntime {
        /// True when the image holds a common language runtime header.
        dot_net: bool,
    },
    /// The image uses the Visual Basic 5 runtime.
    #[error("this file uses the Visual Basic 5 runtime, and DeForm6 reads Visual Basic 6 only")]
    IsVb5,
    /// The image uses the 32 bit Visual Basic 4 runtime.
    #[error("this file uses the Visual Basic 4 runtime, and DeForm6 reads Visual Basic 6 only")]
    IsVb4,
    /// The image is Visual Basic 6 and the parser does not walk it.
    ///
    /// The payload names what the parser expected to find.
    #[error("this Visual Basic 6 executable is damaged: {0}")]
    Damaged(&'static str),
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{Defect, DefectKind, Refusal, Severity, Site};

    /// A site with values that no `DefectKind` message reads.
    fn a_site() -> Site {
        Site {
            offset: 0x1000,
            rva: Some(0x2000),
            structure: "VbHeader",
            field: "lpProjectData",
        }
    }

    /// Every kind, each with the offset that its message must name.
    ///
    /// The list is a literal. A new variant that is absent from it is visible
    /// in review. A list derived from the enum grows by itself and proves
    /// nothing new.
    fn every_kind_with_its_offset() -> Vec<(DefectKind, u32)> {
        vec![
            (
                DefectKind::BadMagic {
                    offset: 0x11,
                    expected: "VB5!",
                    found: 0x4142_4344,
                },
                0x11,
            ),
            (
                DefectKind::OffsetOverflow {
                    offset: 0x22,
                    len: 0xffff_ffff,
                },
                0x22,
            ),
            (
                DefectKind::PastEndOfFile {
                    offset: 0x33,
                    file_len: 28672,
                },
                0x33,
            ),
            (
                DefectKind::ImplausibleCount {
                    offset: 0x44,
                    count: 70000,
                    max: 512,
                },
                0x44,
            ),
            (
                DefectKind::UnmappedAddress {
                    offset: 0x55,
                    va: 0x0040_1000,
                },
                0x55,
            ),
            (
                DefectKind::SectionOverlap {
                    offset: 0x66,
                    other: 0x0999,
                },
                0x66,
            ),
            (
                DefectKind::NoNulTerminator {
                    offset: 0x77,
                    limit: 260,
                },
                0x77,
            ),
            (
                DefectKind::CountMismatch {
                    offset: 0x88,
                    count: 3,
                    expected: 5,
                    other_field: "wTotalObjects",
                },
                0x88,
            ),
            (
                DefectKind::UnreadablePointer {
                    offset: 0x99,
                    va: 0x0040_2000,
                },
                0x99,
            ),
        ]
    }

    #[test]
    fn every_defect_message_names_its_byte_offset_in_hexadecimal() {
        for (kind, offset) in every_kind_with_its_offset() {
            let message = format!("{kind}");
            let wanted = format!("{offset:#x}");
            assert!(
                message.contains(&wanted),
                "the message of {kind:?} does not name its offset {wanted}: {message}"
            );
        }
    }

    #[test]
    fn a_bad_magic_message_names_what_it_expected_there() {
        let kind = DefectKind::BadMagic {
            offset: 0x1760,
            expected: "VB5!",
            found: 0,
        };
        let message = format!("{kind}");
        assert!(message.contains("VB5!"), "{message}");
        assert!(message.contains("0x1760"), "{message}");
    }

    #[test]
    fn a_fatal_kind_is_fatal_and_a_recoverable_kind_is_recoverable() {
        let fatal = [
            DefectKind::BadMagic {
                offset: 0,
                expected: "VB5!",
                found: 0,
            },
            DefectKind::OffsetOverflow { offset: 0, len: 0 },
            DefectKind::PastEndOfFile {
                offset: 0,
                file_len: 0,
            },
            DefectKind::UnmappedAddress { offset: 0, va: 0 },
        ];
        for kind in fatal {
            assert_eq!(
                kind.severity(),
                Severity::Fatal,
                "{kind:?} must be fatal, because nothing downstream of it means anything"
            );
        }

        let recoverable = [
            DefectKind::ImplausibleCount {
                offset: 0,
                count: 0,
                max: 0,
            },
            DefectKind::SectionOverlap {
                offset: 0,
                other: 0,
            },
            DefectKind::NoNulTerminator {
                offset: 0,
                limit: 0,
            },
            DefectKind::CountMismatch {
                offset: 0,
                count: 0,
                expected: 0,
                other_field: "wTotalObjects",
            },
            DefectKind::UnreadablePointer { offset: 0, va: 0 },
        ];
        for kind in recoverable {
            assert_eq!(
                kind.severity(),
                Severity::Recoverable,
                "{kind:?} must be recoverable, because the rest of the graph still stands"
            );
        }
    }

    /// A value of a type that is not `Serialize` does not compile here.
    fn accepts_only_serialize<T: serde::Serialize>(_value: &T) {}

    #[test]
    fn a_site_a_kind_and_a_defect_all_serialise() {
        let defect = Defect {
            site: a_site(),
            kind: DefectKind::UnmappedAddress {
                offset: 0x30,
                va: 0x0040_1000,
            },
        };
        accepts_only_serialize(&defect.site);
        accepts_only_serialize(&defect.kind);
        accepts_only_serialize(&defect);
    }

    #[test]
    fn a_defect_message_holds_both_the_site_and_the_kind() {
        let defect = Defect {
            site: a_site(),
            kind: DefectKind::UnmappedAddress {
                offset: 0x30,
                va: 0x0040_1000,
            },
        };
        let message = format!("{defect}");
        assert!(message.contains("lpProjectData"), "{message}");
        assert!(message.contains("0x30"), "{message}");
    }

    /// Every refusal, with `NoVbRuntime` present once for each flag value.
    ///
    /// The list is a literal for the reason the kind list is a literal.
    const EVERY_REFUSAL: [Refusal; 8] = [
        Refusal::NotPe,
        Refusal::NotI386,
        Refusal::NotPe32,
        Refusal::NoVbRuntime { dot_net: false },
        Refusal::NoVbRuntime { dot_net: true },
        Refusal::IsVb5,
        Refusal::IsVb4,
        Refusal::Damaged("the entry point is in no section"),
    ];

    #[test]
    fn every_refusal_sentence_is_one_non_empty_line() {
        for refusal in EVERY_REFUSAL {
            let sentence = format!("{refusal}");
            assert!(!sentence.is_empty(), "{refusal:?} renders nothing");
            assert!(
                !sentence.contains('\n'),
                "{refusal:?} renders more than one line: {sentence}"
            );
        }
    }

    #[test]
    fn no_refusal_sentence_dumps_a_byte_or_an_offset() {
        for refusal in EVERY_REFUSAL {
            let sentence = format!("{refusal}");
            assert!(
                !sentence.contains("0x"),
                "{refusal:?} puts a raw value in the sentence a person reads: {sentence}"
            );
        }
    }

    #[test]
    fn no_refusal_sentence_carries_a_path() {
        for refusal in EVERY_REFUSAL {
            let sentence = format!("{refusal}");
            assert!(
                !sentence.contains('/'),
                "{refusal:?} carries a path, and the command line owns the path: {sentence}"
            );
            assert!(
                !sentence.contains('\\'),
                "{refusal:?} carries a path, and the command line owns the path: {sentence}"
            );
        }
    }

    #[test]
    fn the_dot_net_flag_gives_a_second_sentence_and_not_a_second_variant() {
        let plain = format!("{}", Refusal::NoVbRuntime { dot_net: false });
        let dot_net = format!("{}", Refusal::NoVbRuntime { dot_net: true });
        assert_ne!(
            plain, dot_net,
            "the .NET case must read differently, or the flag buys nothing"
        );
        assert!(dot_net.contains(".NET"), "{dot_net}");
    }

    #[test]
    fn a_refusal_compares_by_variant_and_not_by_wording() {
        assert_ne!(Refusal::IsVb5, Refusal::NoVbRuntime { dot_net: false });
        assert_ne!(
            Refusal::NoVbRuntime { dot_net: false },
            Refusal::NoVbRuntime { dot_net: true }
        );
        assert_eq!(Refusal::IsVb5, Refusal::IsVb5);
        assert_eq!(
            Refusal::NoVbRuntime { dot_net: true },
            Refusal::NoVbRuntime { dot_net: true }
        );
    }

    /// `WINDOWS.md` finding 3: `Report` derives `PartialEq`, and `Defect`
    /// used to be unable to. Two equal defects must compare equal, and two
    /// defects that differ in either their site or their kind must not, or a
    /// `Vec<Defect>` inside `Report` would compare as equal when it should
    /// not.
    #[test]
    fn a_defect_compares_by_its_site_and_its_kind() {
        let one = Defect {
            site: a_site(),
            kind: DefectKind::UnreadablePointer {
                offset: 0x99,
                va: 0x0040_2000,
            },
        };
        let same = Defect {
            site: a_site(),
            kind: DefectKind::UnreadablePointer {
                offset: 0x99,
                va: 0x0040_2000,
            },
        };
        let different_site = Defect {
            site: Site {
                offset: 0x2000,
                ..a_site()
            },
            kind: one.kind.clone(),
        };
        let different_kind = Defect {
            site: a_site(),
            kind: DefectKind::UnreadablePointer {
                offset: 0x99,
                va: 0x0040_3000,
            },
        };
        assert_eq!(one, same);
        assert_ne!(one, different_site);
        assert_ne!(one, different_kind);
    }
}
