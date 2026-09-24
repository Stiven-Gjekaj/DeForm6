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
    /// The address of the byte at `offset`, as a relative virtual address.
    ///
    /// It is `None` when the byte is in no section, such as a byte of a
    /// section header, or when the reader does not know the address of the
    /// byte. A site whose `offset` is 0 because the reader does not know
    /// where the byte is also gives `None`. This field never holds an address
    /// that the byte points at: a defect about a pointer carries the address
    /// that the pointer holds in its [`DefectKind`].
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

    /// Two sections claim the same addresses.
    #[error(
        "the section whose address is at offset {offset:#x} overlaps the section whose bytes start at offset {other:#x}"
    )]
    SectionOverlap {
        /// The absolute file offset of the `VirtualAddress` field of the
        /// second section header.
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

    /// Two fields that each tell whether an object is a standard module do
    /// not agree.
    ///
    /// Bit `0x2` of `Object.fObjectType` tells whether the object has an
    /// `OptionalObjectInfo` block. `ObjectInfo.lpPrivateObject` tells whether
    /// it has a `PrivateObj`. A standard module has neither, and in the
    /// corpus every other object has both. `STRUCTURES.md` section 5.5 asks
    /// for a disagreement to be reported, and not resolved.
    #[error(
        "the private object address {pointer:#x} at offset {offset:#x} disagrees with fObjectType {object_type:#x} about whether the object is a standard module"
    )]
    ModuleMarkerMismatch {
        /// The absolute file offset of `ObjectInfo.lpPrivateObject`.
        offset: u32,
        /// The value `ObjectInfo.lpPrivateObject` holds.
        pointer: u32,
        /// The value `Object.fObjectType` holds.
        object_type: u32,
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

    /// An event slot names a stub that does not have the native shape.
    ///
    /// `STRUCTURES.md` section 8.6 gives the native stub as `81 6C 24 04
    /// <imm32>`, then `E9 <rel32>`. A stub that holds other bytes at one of
    /// these five places, such as a P-code stub, is not decoded. The slot
    /// stays bound and keeps its stub address, and it has no handler.
    #[error(
        "the event stub at offset {offset:#x} holds {found:02x?}, and a native stub holds 81 6c 24 04 at +0x00 and e9 at +0x08"
    )]
    UnknownStubShape {
        /// The absolute file offset of the stub.
        offset: u32,
        /// The 13 bytes of the stub, in file order.
        found: [u8; 13],
    },

    /// A length-prefixed name field declares a length of zero.
    ///
    /// Plan 03-04: a control's declared name length can legitimately be
    /// zero. The control keeps every other field; only the name is empty.
    #[error("the name at offset {offset:#x} has a declared length of zero")]
    EmptyName {
        /// The absolute file offset of the two-byte length of the name.
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
        /// The absolute file offset of the two-byte index.
        offset: u32,
        /// The high byte of the two-byte index value.
        high: u8,
    },

    /// A `String`-typed property did not land on its declared end under
    /// either encoding tried.
    ///
    /// Plan 03-05: `VbStr::read` tries the caller's own encoding, retries
    /// once as the other, and refuses when neither decode consumes exactly
    /// the declared length with a null byte at the declared end. The
    /// property is unrecoverable; the block around it is not, because the
    /// cursor still advances to the declared end.
    #[error(
        "the string at offset {offset:#x}, declared length {declared_len}, did not land under either encoding tried ({first_encoding} then {second_encoding})"
    )]
    UnrecoverableString {
        /// The absolute file offset of the string's own length field.
        offset: u32,
        /// The declared length, in bytes, of the string's own text.
        declared_len: u16,
        /// The encoding tried first.
        first_encoding: &'static str,
        /// The encoding retried second.
        second_encoding: &'static str,
    },

    /// An external control's class name holds no dot separating the library
    /// part from the component part.
    ///
    /// Plan 03-08: `STRUCTURES.md` section 8.7's own examples always hold
    /// one, so a name with none cannot be joined against the external
    /// component table by library. The whole string is still kept as the
    /// library part.
    #[error(
        "the class name at offset {offset:#x} holds no dot separating the library from the component"
    )]
    ClassNameNoDot {
        /// The absolute file offset of the class name's own length field.
        offset: u32,
    },

    /// A component entry's `GUIDlength` field holds a value that is neither
    /// `-1` (no binary GUID) nor `72` (a 36 character UTF-16 GUID).
    ///
    /// Plan 03-08: `STRUCTURES.md` section 7.3 names only these two values.
    /// Any other value is not decoded, and the component keeps its other
    /// fields.
    #[error(
        "the value {value} at offset {offset:#x} is neither -1 nor 72, so the textual GUID is not decoded"
    )]
    GuidLengthUnexpected {
        /// The absolute file offset of the `GUIDlength` field.
        offset: u32,
        /// The value found.
        value: i32,
    },

    /// The fixed OCX header's own reserved field, at offset `0x04` from its
    /// signature, does not hold the value every known sample gives.
    ///
    /// Plan 03-08: `STRUCTURES.md` section 8.7 names the field reserved,
    /// always `8`. A different value does not lose the three properties
    /// after it: `_ExtentX`, `_ExtentY` and `_Version` still read.
    #[error("the reserved field at offset {offset:#x} holds {value:#x}, not the expected value 8")]
    OcxReservedFieldUnexpected {
        /// The absolute file offset of the reserved field.
        offset: u32,
        /// The value found.
        value: u32,
    },

    /// A resource blob's declared length is not the absent sentinel
    /// (`0xFFFFFFFF`) yet is too small to hold its own eight byte inline
    /// picture header.
    ///
    /// Plan 03-07: `blobLen` counts the eight byte inline picture header
    /// plus the image bytes, so a real blob's declared length is never
    /// below 8. A smaller value cannot be split into a header and an image
    /// byte count with a checked subtraction; this is the refusal that
    /// subtraction gives instead of wrapping to a very large number.
    #[error(
        "the blob length {blob_len} at offset {offset:#x} is too small to hold its own 8 byte picture header"
    )]
    BlobLenTooSmall {
        /// The absolute file offset of the blob's own length field.
        offset: u32,
        /// The declared length that was too small.
        blob_len: u32,
    },

    /// One structure could not be read, and the walk that reached it
    /// continues over the rest of the report. The reason says why.
    ///
    /// Plan 03-10: the composed `inspect` walk converts a [`Refusal`] into
    /// this defect for a form's own `GuiObjectInfo`, its property stream, its
    /// control tree, its `ControlInfoTable`, or one control's own event
    /// table. Any of these refusing costs the one form or the one control,
    /// never the whole file. `vb/functyp.rs` gives it for an `optionalVals`
    /// block that holds more value records than the reader reads, and the
    /// prototype then keeps no default.
    #[error("the structure at offset {offset:#x} could not be read: {reason}")]
    StructureUnreadable {
        /// The absolute file offset the structure was read from, when one
        /// was known; `0` when it was not.
        offset: u32,
        /// The refusal's own message, carried verbatim.
        reason: String,
    },

    /// One item's own address resolves to nothing, and the item is skipped.
    ///
    /// Phase 5: [`DefectKind::UnmappedAddress`] is `Fatal`, because it was
    /// written for the one spine pointer whose loss means nothing
    /// downstream resolves. This variant names the same fact, an address
    /// that maps into no section, for a call site where the address
    /// belongs to one item only: `vb/project.rs::DeclareTable::read` loses
    /// one `Declare` entry and keeps walking the rest of the table. Nothing
    /// downstream of this item rests on the value this pointer would have
    /// named. The same reader loses the whole `Declare` table when the
    /// table's own address maps nowhere, and the rest of the program still
    /// reads.
    #[error("address {va:#x} at offset {offset:#x} is in no section, and the item is skipped")]
    ItemAddressUnmapped {
        /// The absolute file offset of the pointer that held the address.
        offset: u32,
        /// The address that maps nowhere.
        va: u32,
    },

    /// One item's own offset plus length arithmetic overflows a `u32`, and
    /// the item is skipped.
    ///
    /// Phase 5: [`DefectKind::OffsetOverflow`] is `Fatal`, because it was
    /// written for a range every later bound check rests on. This variant
    /// names the same overflow for a call site where the range belongs to
    /// one item only: a resource blob's declared length
    /// (`vb/frx.rs::extract_blob`), a string's declared end
    /// (`vb/vbstr.rs::VbStr::overflow`), or one prototype's optional value
    /// records (`vb/functyp.rs`). The item that overflowed is lost; the
    /// rest of the read continues.
    #[error("offset {offset:#x} plus length {len:#x} overflows a u32, and the item is skipped")]
    ItemOffsetOverflow {
        /// The absolute file offset the sum started from.
        offset: u32,
        /// The length that was added to it.
        len: u32,
    },

    /// One item's own address maps into a section, and the file holds fewer
    /// bytes of that section than the item needs. The item is skipped.
    ///
    /// The address is in a section, so this is not
    /// [`DefectKind::ItemAddressUnmapped`]. `vb/project.rs::DeclareTable::read`
    /// gives it for a `Declare` descriptor that starts too near the end of
    /// its section.
    #[error(
        "the {len} bytes at address {va:#x}, which the pointer at offset {offset:#x} names, run past the bytes that the file holds for their section, and the item is skipped"
    )]
    ItemCutShort {
        /// The absolute file offset of the pointer that held the address.
        offset: u32,
        /// The address of the item.
        va: u32,
        /// The number of bytes that the reader must read at that address.
        len: u32,
    },

    /// Bytes that the reader needs run past the end of the bytes that can
    /// hold them: the end of their section, or the end that the declared
    /// length of their block gives.
    ///
    /// The site names the pointer or the field that led the reader to these
    /// bytes. Nothing is read from them, and the item keeps every other
    /// field. A `Declare` descriptor that its section cuts short gives
    /// [`DefectKind::ItemCutShort`] instead, because that reader skips the
    /// whole item.
    #[error(
        "the {len} bytes at offset {offset:#x} run past offset {end:#x}, where their section or their block ends, and the item keeps its other fields"
    )]
    RunsPastEnd {
        /// The absolute file offset of the first of the bytes.
        offset: u32,
        /// The number of bytes that the reader needs there. It is
        /// `u32::MAX` when that number leaves a `u32`.
        len: u32,
        /// The absolute file offset where their section or their block ends.
        end: u32,
    },

    /// A field that the format fixes at one value holds another value.
    ///
    /// The reader does not read the item that holds the field, and nothing is
    /// invented in its place. `vb/functyp.rs` gives it for a `FuncTypDesc`
    /// whose `constFFFF` is not `0xFFFF`.
    #[error(
        "the field at offset {offset:#x} holds {found:#x}, where the format gives {expected:#x}, and the item is not read"
    )]
    UnexpectedConstant {
        /// The absolute file offset of the field.
        offset: u32,
        /// The value that the format gives.
        expected: u32,
        /// The value that the field holds.
        found: u32,
    },

    /// The text that a name pointer names is not a Visual Basic identifier.
    ///
    /// An identifier starts with an ASCII letter or an underscore, and each of
    /// its bytes is an ASCII letter, a digit or an underscore. The name is not
    /// kept, and the item keeps its other fields. The site names the pointer.
    #[error(
        "the {len} bytes of text at offset {offset:#x} are not an identifier, and the item keeps its other fields"
    )]
    NotAnIdentifier {
        /// The absolute file offset where the text starts.
        offset: u32,
        /// The number of bytes of the text, before its NUL.
        len: u32,
    },

    /// The relative jump of a native event stub gives an address outside the
    /// 32-bit address space.
    ///
    /// The jump counts from the end of the 13-byte stub. The slot stays bound
    /// and keeps its stub address, and it has no handler. The site names the
    /// `rel32` field of the stub, because the fault is in the stub and not in
    /// the slot.
    #[error(
        "the relative jump {rel} at offset {offset:#x}, which counts from the end of the stub at address {va:#x}, leaves the address space, and the slot keeps no handler"
    )]
    JumpOutOfRange {
        /// The absolute file offset of the 4-byte relative jump, at `+0x09`
        /// of the stub.
        offset: u32,
        /// The address of the stub.
        va: u32,
        /// The relative jump, as the file holds it.
        rel: i32,
    },

    /// A field holds a value that is not one of the values that the reader
    /// knows for it.
    ///
    /// The reader does not guess what the value means. It does not read what
    /// the field describes, and nothing is invented in its place.
    /// `vb/functyp.rs` gives it for a value tag in `optionalVals` that is not
    /// one of the six tags that the corpus holds, and for a leading byte of a
    /// type buffer that is neither `0x1E` nor `0x00`.
    #[error(
        "the field at offset {offset:#x} holds {value:#x}, which is not a value that the reader knows, and the reader does not read what the field describes"
    )]
    UnknownValue {
        /// The absolute file offset of the field.
        offset: u32,
        /// The value that the field holds.
        value: u32,
    },

    /// One item's own type field holds a value that no source describes, and
    /// the item is skipped.
    ///
    /// `vb/project.rs::DeclareTable::read` gives it for a `Declare` entry
    /// whose `dwEntryType` is neither 6 (internal) nor 7 (external). The
    /// reader does not guess what the entry names, and it keeps walking the
    /// rest of the table.
    #[error(
        "the type {value} at offset {offset:#x} is not a type that the reader knows, and the item is skipped"
    )]
    ItemTypeUnknown {
        /// The absolute file offset of the type field.
        offset: u32,
        /// The value that the type field holds.
        value: u32,
    },
}

/// How bad a defect is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
pub enum Severity {
    /// The file is not what it claims. Nothing downstream is meaningful.
    Fatal,
    /// The reader continued the read by using a value the file does not
    /// state.
    ///
    /// A fact in the report rests on an assumption this run made. Strict
    /// mode refuses a defect at this severity, because the run cannot name
    /// the value without inventing it.
    Recoverable,
    /// A defect that costs one item and puts nothing in its place.
    ///
    /// The report holds one fewer fact, and every fact it does hold came
    /// from the file. `Tolerated` is what lets strict mode refuse a
    /// damaged file without refusing an undamaged one: every defect the 44
    /// vendored corpus programs raise carries this severity, never
    /// `Recoverable`, so a strict run that refuses on `Recoverable` and
    /// continues past `Tolerated` refuses exactly the files that assumed a
    /// value, and none that only lost one.
    Tolerated,
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
            // The reader clamps the count to a value the file does not
            // state, so a fact downstream rests on that assumption.
            Self::ImplausibleCount { .. } => Severity::Recoverable,
            // The reader picks one of two sections that claim the same
            // bytes, a choice the file does not state.
            Self::SectionOverlap { .. } => Severity::Recoverable,
            // The reader ends the string at a bound the file does not
            // state, so the text is not what the file says.
            Self::NoNulTerminator { .. } => Severity::Recoverable,
            // The reader takes the smaller of two disagreeing counts, a
            // choice the file does not state.
            Self::CountMismatch { .. } => Severity::Recoverable,
            // Each reader still follows its own field, and the check chooses
            // nothing: the object loses what one of the two fields withholds,
            // an optional block or a private object, and nothing is invented
            // in its place. An fObjectType value nobody has measured is never
            // a refusal (D-08), so its bit is not one either.
            Self::ModuleMarkerMismatch { .. } => Severity::Tolerated,
            // The item keeps its other fields, and the pointer's target is
            // simply absent: nothing is invented in its place.
            Self::UnreadablePointer { .. } => Severity::Tolerated,
            // The slot keeps its index and its stub address, and only the
            // handler is absent: no handler is invented from bytes of a shape
            // the reader does not decode.
            Self::UnknownStubShape { .. } => Severity::Tolerated,
            // The control keeps its type, and no name is invented in its
            // place.
            Self::EmptyName { .. } => Severity::Tolerated,
            // The control keeps every field; the high byte costs nothing
            // and invents nothing.
            Self::IndexHighByteSet { .. } => Severity::Tolerated,
            // The cursor still advances to the declared end, and no text
            // is guessed in its place.
            Self::UnrecoverableString { .. } => Severity::Tolerated,
            // The reader treats the whole text as the library part, a
            // split the file does not state.
            Self::ClassNameNoDot { .. } => Severity::Recoverable,
            // The component loses only its textual identifier; every other
            // field still reads.
            Self::GuidLengthUnexpected { .. } => Severity::Tolerated,
            // The other three header fields still read; nothing replaces
            // the reserved value.
            Self::OcxReservedFieldUnexpected { .. } => Severity::Tolerated,
            // The one property is reported unreadable, and nothing is
            // written in its place.
            Self::BlobLenTooSmall { .. } => Severity::Tolerated,
            // The one form, control or table is reported refused, and
            // nothing is written in its place.
            Self::StructureUnreadable { .. } => Severity::Tolerated,
            // The item this pointer would have named is absent, a fact
            // downstream rests on the run continuing without it, and strict
            // mode refuses rather than assume the item away.
            Self::ItemAddressUnmapped { .. } => Severity::Recoverable,
            // The item this range would have named is absent, a fact
            // downstream rests on the run continuing without it, and strict
            // mode refuses rather than assume the item away.
            Self::ItemOffsetOverflow { .. } => Severity::Recoverable,
            // The item is absent, as it is for an address in no section, and
            // strict mode refuses rather than assume the item away.
            Self::ItemCutShort { .. } => Severity::Recoverable,
            // Nothing is read from the bytes, and nothing is invented in
            // their place. The item keeps every other field.
            Self::RunsPastEnd { .. } => Severity::Tolerated,
            // The item is not read, and nothing is invented in its place.
            Self::UnexpectedConstant { .. } => Severity::Tolerated,
            // The name is absent, and no name is invented in its place.
            Self::NotAnIdentifier { .. } => Severity::Tolerated,
            // The slot keeps its stub address, and no handler is invented in
            // its place.
            Self::JumpOutOfRange { .. } => Severity::Tolerated,
            // The reader does not read what the field describes, and nothing
            // is invented in its place.
            Self::UnknownValue { .. } => Severity::Tolerated,
            // The item is absent, as it is for an address in no section, and
            // strict mode refuses rather than assume the item away.
            Self::ItemTypeUnknown { .. } => Severity::Recoverable,
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

/// Builds a [`Refusal::Damaged`] whose message is computed at run time.
///
/// `Refusal::Damaged` takes `&'static str` crate-wide; the exit code table
/// in this enum's own doc comment is locked, so this helper does not widen
/// the variant to carry an owned `String`. `Box::leak` is the narrow,
/// deliberate escape hatch a caller reaches only when it must name a value
/// known only at run time (a byte offset, a count) inside an already-fatal
/// refusal.
///
/// This was three byte for byte identical private copies (`vb/gui.rs`,
/// `vb/controltree.rs`, `vb/frx.rs`) before phase 3 code review finding
/// WR-01; this is the one shared copy that replaces them.
///
/// # Memory cost
///
/// The leak is real and it does not reclaim. A single command line run
/// leaks one short string once, and the process exits soon after, so the
/// cost is harmless there. A long running host that calls this library in a
/// loop over many hostile files (a fuzz target, or a batch scanner) grows
/// this leak without bound, one string per refusal that reaches this
/// function. This repository has no such host yet: closing the leak for
/// good needs `Refusal::Damaged` to carry an owned `String` instead, which
/// touches every one of its construction sites crate-wide, not only this
/// function's three former callers. Phase 5 owns the fuzz host that would
/// first turn this cost into a real problem.
pub(crate) fn damaged(message: String) -> Refusal {
    let leaked: &'static str = Box::leak(message.into_boxed_str());
    Refusal::Damaged(leaked)
}

/// Builds the [`Refusal`] a strict run gives back for `defect`.
///
/// This is the one place a [`Defect`] becomes a [`Refusal`]. The message
/// names three things: the defect kind's own sentence, which already
/// interpolates the byte offset in hexadecimal, the structure name from
/// [`Site::structure`], and the field name from [`Site::field`].
///
/// This reaches `damaged`, and therefore leaks one short string per
/// refusal, the same cost `damaged`'s own doc comment already names. The
/// fuzz job in plan 05-03 turns leak detection off for exactly this reason:
/// a strict run over a hostile file now reaches this function far more
/// often than the three call sites `damaged` was written for.
#[must_use]
pub fn refusal_for_defect(defect: &Defect) -> Refusal {
    damaged(format!(
        "{} ({}.{})",
        defect.kind, defect.site.structure, defect.site.field
    ))
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
                DefectKind::ModuleMarkerMismatch {
                    offset: 0x8c,
                    pointer: 0xffff_ffff,
                    object_type: 0x0001_8083,
                },
                0x8c,
            ),
            (
                DefectKind::UnreadablePointer {
                    offset: 0x99,
                    va: 0x0040_2000,
                },
                0x99,
            ),
            (
                DefectKind::UnknownStubShape {
                    offset: 0x9e,
                    found: [
                        0x33, 0xc0, 0xba, 0x34, 0x12, 0x40, 0x00, 0x68, 0x34, 0x12, 0x40, 0x00,
                        0xc3,
                    ],
                },
                0x9e,
            ),
            (
                DefectKind::ItemAddressUnmapped {
                    offset: 0xaa,
                    va: 0x0040_3000,
                },
                0xaa,
            ),
            (
                DefectKind::ItemOffsetOverflow {
                    offset: 0xbb,
                    len: 0xffff_ffff,
                },
                0xbb,
            ),
            (
                DefectKind::ItemCutShort {
                    offset: 0xcc,
                    va: 0x0040_4000,
                    len: 8,
                },
                0xcc,
            ),
            (
                DefectKind::RunsPastEnd {
                    offset: 0xdd,
                    len: 13,
                    end: 0xe0,
                },
                0xdd,
            ),
            (
                DefectKind::UnexpectedConstant {
                    offset: 0xee,
                    expected: 0xffff,
                    found: 0x1234,
                },
                0xee,
            ),
            (
                DefectKind::NotAnIdentifier {
                    offset: 0xef,
                    len: 13,
                },
                0xef,
            ),
            (
                DefectKind::JumpOutOfRange {
                    offset: 0xf1,
                    va: 0x0040_1100,
                    rel: i32::MIN,
                },
                0xf1,
            ),
            (
                DefectKind::UnknownValue {
                    offset: 0xf2,
                    value: 99,
                },
                0xf2,
            ),
            (
                DefectKind::ItemTypeUnknown {
                    offset: 0xf3,
                    value: 99,
                },
                0xf3,
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
    fn a_module_marker_mismatch_message_names_both_values_and_both_fields() {
        let kind = DefectKind::ModuleMarkerMismatch {
            offset: 0x1a3c,
            pointer: 0xffff_ffff,
            object_type: 0x0001_8083,
        };
        let message = format!("{kind}");
        assert!(message.contains("0x1a3c"), "{message}");
        assert!(message.contains("0xffffffff"), "{message}");
        assert!(message.contains("0x18083"), "{message}");
        assert!(message.contains("fObjectType"), "{message}");
        assert!(message.contains("private object address"), "{message}");
    }

    #[test]
    fn an_unknown_stub_shape_message_names_the_bytes_found_and_the_bytes_expected() {
        let kind = DefectKind::UnknownStubShape {
            offset: 0x19d0,
            found: [
                0x33, 0xc0, 0xba, 0xd0, 0x19, 0x40, 0x00, 0x68, 0xd0, 0x19, 0x40, 0x00, 0xc3,
            ],
        };
        let message = format!("{kind}");
        assert!(message.contains("0x19d0"), "{message}");
        assert!(
            message.contains("[33, c0, ba, d0, 19, 40, 00, 68, d0, 19, 40, 00, c3]"),
            "{message}"
        );
        assert!(message.contains("81 6c 24 04"), "{message}");
        assert!(message.contains("e9 at +0x08"), "{message}");
    }

    #[test]
    fn an_item_cut_short_message_names_the_address_and_the_length() {
        let kind = DefectKind::ItemCutShort {
            offset: 0x1ae0,
            va: 0x0040_3ffc,
            len: 8,
        };
        let message = format!("{kind}");
        assert!(message.contains("0x1ae0"), "{message}");
        assert!(message.contains("0x403ffc"), "{message}");
        assert!(message.contains("the 8 bytes"), "{message}");
        assert!(message.contains("run past"), "{message}");
    }

    #[test]
    fn a_runs_past_end_message_names_the_bytes_and_where_they_end() {
        let kind = DefectKind::RunsPastEnd {
            offset: 0x42b,
            len: 13,
            end: 0x430,
        };
        let message = format!("{kind}");
        assert!(
            message.contains("the 13 bytes at offset 0x42b"),
            "{message}"
        );
        assert!(message.contains("run past offset 0x430"), "{message}");
    }

    #[test]
    fn an_unexpected_constant_message_names_both_values() {
        let kind = DefectKind::UnexpectedConstant {
            offset: 0x1d24,
            expected: 0xffff,
            found: 0x1234,
        };
        let message = format!("{kind}");
        assert!(message.contains("offset 0x1d24 holds 0x1234"), "{message}");
        assert!(message.contains("the format gives 0xffff"), "{message}");
    }

    #[test]
    fn a_not_an_identifier_message_names_the_text_and_its_length() {
        let kind = DefectKind::NotAnIdentifier {
            offset: 0x2f10,
            len: 13,
        };
        let message = format!("{kind}");
        assert!(
            message.contains("the 13 bytes of text at offset 0x2f10"),
            "{message}"
        );
        assert!(message.contains("not an identifier"), "{message}");
    }

    #[test]
    fn a_jump_out_of_range_message_names_the_jump_and_the_stub() {
        let kind = DefectKind::JumpOutOfRange {
            offset: 0x509,
            va: 0x0040_1100,
            rel: -2_147_483_648,
        };
        let message = format!("{kind}");
        assert!(
            message.contains("the relative jump -2147483648 at offset 0x509"),
            "{message}"
        );
        assert!(
            message.contains("the stub at address 0x401100"),
            "{message}"
        );
    }

    #[test]
    fn an_unknown_value_message_names_the_field_and_its_value() {
        let kind = DefectKind::UnknownValue {
            offset: 0x408,
            value: 99,
        };
        let message = format!("{kind}");
        assert!(
            message.contains("the field at offset 0x408 holds 0x63"),
            "{message}"
        );
        assert!(
            message.contains("not a value that the reader knows"),
            "{message}"
        );
    }

    #[test]
    fn an_item_type_unknown_message_names_the_type_and_its_offset() {
        let kind = DefectKind::ItemTypeUnknown {
            offset: 0x400,
            value: 99,
        };
        let message = format!("{kind}");
        assert!(message.contains("the type 99 at offset 0x400"), "{message}");
        assert!(message.contains("the item is skipped"), "{message}");
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
    fn a_fatal_kind_is_fatal_a_recoverable_kind_is_recoverable_and_a_tolerated_kind_is_tolerated() {
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
            DefectKind::ClassNameNoDot { offset: 0 },
            DefectKind::ItemAddressUnmapped { offset: 0, va: 0 },
            DefectKind::ItemOffsetOverflow { offset: 0, len: 0 },
            DefectKind::ItemCutShort {
                offset: 0,
                va: 0,
                len: 0,
            },
            DefectKind::ItemTypeUnknown {
                offset: 0,
                value: 0,
            },
        ];
        for kind in recoverable {
            assert_eq!(
                kind.severity(),
                Severity::Recoverable,
                "{kind:?} must be recoverable, because a fact downstream rests on a value \
                 the reader assumed"
            );
        }

        let tolerated = [
            DefectKind::UnreadablePointer { offset: 0, va: 0 },
            DefectKind::UnknownStubShape {
                offset: 0,
                found: [0; 13],
            },
            DefectKind::EmptyName { offset: 0 },
            DefectKind::IndexHighByteSet { offset: 0, high: 0 },
            DefectKind::UnrecoverableString {
                offset: 0,
                declared_len: 0,
                first_encoding: "utf-16",
                second_encoding: "ansi",
            },
            DefectKind::GuidLengthUnexpected {
                offset: 0,
                value: 0,
            },
            DefectKind::OcxReservedFieldUnexpected {
                offset: 0,
                value: 0,
            },
            DefectKind::BlobLenTooSmall {
                offset: 0,
                blob_len: 0,
            },
            DefectKind::StructureUnreadable {
                offset: 0,
                reason: String::new(),
            },
            DefectKind::ModuleMarkerMismatch {
                offset: 0,
                pointer: 0,
                object_type: 0,
            },
            DefectKind::RunsPastEnd {
                offset: 0,
                len: 0,
                end: 0,
            },
            DefectKind::UnexpectedConstant {
                offset: 0,
                expected: 0,
                found: 0,
            },
            DefectKind::NotAnIdentifier { offset: 0, len: 0 },
            DefectKind::JumpOutOfRange {
                offset: 0,
                va: 0,
                rel: 0,
            },
            DefectKind::UnknownValue {
                offset: 0,
                value: 0,
            },
        ];
        for kind in tolerated {
            assert_eq!(
                kind.severity(),
                Severity::Tolerated,
                "{kind:?} must be tolerated, because it costs one item and invents nothing \
                 in its place"
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
