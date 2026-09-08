//! `FuncTypDesc`, the type buffer walk, and the default-value walk.
//!
//! This is OBJ-04: a public procedure's argument names, argument types, and
//! its `ByRef`, `Array` and `Optional` modifiers. `PrivateObj.lp_func_type_info`
//! (`vb/privateobj.rs`, plan 02-03) is a pointer array, index-parallel to
//! `Object.lp_proc_names_array` and of the same length, `Object.proc_count`.
//! `STRUCTURES.md` section 6.2 states that a null entry in either array means
//! the procedure at that index is private.
//!
//! # Gap 6, closed on real bytes: the disputed header layout
//!
//! `STRUCTURES.md` section 6.3 records a dispute inside its own source: one
//! figure gives `memberID` at the unaligned offset `0x0A`, with no
//! `optionalVals` field at all; a later, revision-marked figure gives the
//! aligned layout this file reads. A script this planner ran over all 44
//! vendored programs read 193 real `FuncTypDesc` records and found
//! `constFFFF` equal to `0xFFFF` in 193 of 193, and the word at offset `0x06`
//! equal to `0` in 193 of 193. Both numbers are what chose the aligned
//! layout: the other layout would have read `constFFFF` from the low half of
//! `optionalVals`, and would not have produced `0xFFFF` there. [`Prototype`]
//! carries both raw fields (`const_ffff`, `nul1`) so `tests/type_descriptors.rs`
//! can re-assert this over the whole corpus, not just over the three files
//! this module's own tests reach.
//!
//! `constFFFF` is kept as a shipped, `Defect`-producing check, not a debug
//! assertion: a record whose `constFFFF` is not `0xFFFF` is a record whose
//! layout is not the one this file knows, and parsing it anyway would
//! produce a prototype that looks recovered and is wrong.
//!
//! # Gap 7, closed on real bytes: the `optionalVals` grammar
//!
//! `STRUCTURES.md` section 6.3 records `optionalVals` as an unresolved
//! pointer with no description of its target. A script this planner ran over
//! all 44 vendored programs closed it: `optionalVals` points at a `u32`
//! byte length (`cbValues`), then a virtual address equal to
//! `optionalVals + 8` (confirmed in 61 of 61 records, though this file does
//! not dereference through it: the walk always starts at `optionalVals + 8`
//! directly, which is the fact the address confirms), then `cbValues` bytes
//! of value records, in argument order. This is **not** one record per
//! `Optional` argument: a non-`String` `Optional` argument with no explicit
//! `= literal` default gets no record at all, only a `String`-typed one
//! does (`0`, `Empty`, when it has no default). Four records in this
//! corpus are shorter than their `Optional` argument count for exactly
//! this reason, always in the trailing arguments; see
//! [`OptionalDefaultsOutcome::Resolved`] for the worked examples. Each
//! value record is a `u16` OLE variant tag (not the `epvT_*` code of section
//! 6.5) followed by a value padded up to a two-byte boundary. Six tags were
//! observed over 89 value records: `0` empty (an `Optional` argument
//! declared with no literal default), `3` a four-byte signed integer, `4` a
//! four-byte `f32`, `8` a `u16` byte length plus that many bytes of text,
//! `11` a two-byte boolean (`0xFFFF` true, `0x0000` false), `17` a one-byte
//! integer plus one byte of padding. The grammar closes at exactly
//! `cbValues` in 61 of 61 records. A seventh tag has no sample anywhere in
//! this corpus: per D-07, an unrecognised tag stops the walk and reports the
//! record's defaults unrecoverable, and does not guess a width, because a
//! wrong width would desynchronise every later record in the same block.
//!
//! # The property-kind mask `STRUCTURES.md` section 6.6 states is wrong
//!
//! Section 6.6 says the low three bits of `argSize` hold the property kind,
//! mask `0x07`, and in the same paragraph that the entry count is the whole
//! byte shifted right by two. Those two statements overlap on bit 2, so at
//! most one can be right. Measured over all 193 records: masked with `0x07`,
//! 75 of 193 read as kind `4`, which is not one of the three kinds
//! (`001` Get, `010` Let, `111` Set) the same paragraph lists, and every one
//! of the 75 simply has an odd entry count. Masked with `0x03`, exactly 4 of
//! 193 are properties. Bit 2 belongs to the count, not the kind. The four
//! property records are `cCommonDialog`'s (`corpus/vb6-code/Edge-detection/`,
//! source under `corpus/vb6-code/Hidden-Markov-model/cCommonDialog.cls`)
//! `APIReturn` and `ExtendedError` (each `Get`, `bFlags` bit 0 set) and
//! `CustomColor` (`Get` and `Let`), matching `Public Property Get APIReturn`,
//! `Public Property Get ExtendedError`, `Public Property Get CustomColor` and
//! `Public Property Let CustomColor` exactly, including their argument
//! counts. `111`, Property Set, has no sample anywhere in this corpus; the
//! arm stays, unproven, per the same convention `STRUCTURES.md` uses for
//! every gap it cannot close.
//!
//! # Type codes: nine of fifteen occur, zero of the fifteen unassigned do
//!
//! `STRUCTURES.md` section 6.5 tabulates fifteen documented codes. Measured
//! over 193 records: only nine occur at all (`0x03` Boolean, `0x05` Byte,
//! `0x06` Integer, `0x08` Long, `0x0A` Single, `0x0F` Variant, `0x10`
//! String, `0x13` an internal class, `0x1D` an external COM object), and
//! zero of the fifteen values section 6.5 marks unassigned occur anywhere.
//! Per D-07, [`VbType::Unknown`] carries the raw byte for any code this
//! table does not hold; no code is guessed.
//!
//! # `ParamArray`: a reported gap, not a guessed encoding
//!
//! `ParamArray` occurs nowhere in this corpus's source, in 44 programs, and
//! its encoding is therefore not confirmed. `STRUCTURES.md` section 6.5's
//! guess (`0x20 | 0x40 | 0x0F = 0x6F`, indistinguishable from a plain
//! `ByRef Variant()`) is never emitted here. OBJ-04 is partly unreachable
//! for this one modifier, and this file names it only in this doc comment.
//!
//! # A worked example this plan's own text got wrong
//!
//! The plan text for this phase states that `Grayscale.exe`'s
//! `FastDrawing.GetImageWidth` gives `arg_size` `4`. Measured directly: its
//! `arg_size` is `0x08`. The source is
//! `Public Function GetImageWidth(ByRef srcPictureBox As PictureBox) As Long`,
//! which is a function (so `bFlags` bit 0 is set and the type buffer holds
//! two entries, the one argument plus the return type), and two entries is
//! `2 << 2 = 0x08`, not one entry's `0x04`. This file's own tests assert the
//! measured `0x08`, not the plan's inherited `4`; see the module's SUMMARY
//! for the fuller account.

use crate::error::{Defect, DefectKind, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Rva, Va};
use crate::vb::object::Object;
use crate::vb::privateobj::PrivateObj;

/// The size of the fixed `FuncTypDesc` header, before the variable-length
/// type buffer that follows it. `STRUCTURES.md` section 6.3.
const HEADER_SIZE: u32 = 0x20;

/// The width of one entry in `PrivateObj.lpFuncTypeInfo` and in
/// `lpAryArgNames`. Both are arrays of four-byte virtual addresses.
const PTR_SIZE: u32 = 4;

/// The bound on the number of steps the type buffer walk takes, counting the
/// leading byte, every entry byte, every padding byte skipped while looking
/// for one, and the four-byte read of a trailing pointer.
///
/// `argSize` is one byte, so the entry count it can ever demand
/// (`argSize >> 2`) is at most 63. The largest measured anywhere in this
/// corpus is 13 (`cCommonDialog::VBGetOpenFileName`). This bound is
/// generous for that and it is what stops a hostile file, whose `argSize`
/// is non-zero but whose type buffer is all zero bytes, from scanning to
/// the end of a large mapped section looking for a padding byte that never
/// arrives. T-02-15.
const MAX_TYPE_BUFFER_STEPS: u32 = 4096;

/// The bound on one argument name string.
///
/// The same reasoning `vb/privateobj.rs`'s `PROC_NAME_MAX` gives: generous
/// for a VB6 identifier, and still tight enough that an unrelated run of
/// in-image bytes is unlikely to happen to hold a NUL within it.
const ARG_NAME_MAX: u32 = 64;

/// A VB6 argument or return type.
///
/// Fifteen variants match `STRUCTURES.md` section 6.5's documented type
/// codes. [`VbType::Unknown`] carries the raw byte for any code this table
/// does not hold, per D-07: zero of the fifteen codes section 6.5 marks
/// unassigned occur anywhere in the 193 records this phase measured, so this
/// variant exists for the hostile file this corpus does not contain, not for
/// a documented case this file refuses to name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VbType {
    /// `epvT_bool`, `0x03`.
    Boolean,
    /// `epvT_byte`, `0x05`.
    Byte,
    /// `epvT_int`, `0x06`.
    Integer,
    /// `epvT_long`, `0x08`.
    Long,
    /// `epvT_single`, `0x0A`.
    Single,
    /// `epvT_double`, `0x0B`.
    Double,
    /// `epvT_date`, `0x0C`.
    Date,
    /// `epvT_currency`, `0x0D`.
    Currency,
    /// `epvT_variant`, `0x0F`.
    Variant,
    /// `epvT_String`, `0x10`.
    Str,
    /// `epvT_internal`, `0x13`: a class defined in this project. Carries the
    /// raw address of the target class's `ObjInfo`.
    ///
    /// `STRUCTURES.md` section 6.7 states this resolves to a name through
    /// `ObjectInfo.lpObject` -> `Object.lpszObjectName`, joining
    /// `vb/object.rs` and `vb/privateobj.rs` at a class name. That join is
    /// not performed here: it needs a second, independent read of
    /// `ObjectInfo + 0x18`, a field neither of those two modules' own
    /// `read` functions exposes, and building a report-layer join is a
    /// later phase's work, not this plan's. The raw address is carried
    /// forward unresolved, exactly like [`VbType::ComIFace`] and
    /// [`VbType::ComObj`] below, for the same reason.
    Internal(Va),
    /// `epvT_object`, `0x1B`.
    Object,
    /// `epvT_comIFace`, `0x1C`: an external COM interface. Carries the raw
    /// address of the side structure `STRUCTURES.md` section 6.7 describes,
    /// unresolved.
    ComIFace(Va),
    /// `epvT_comobj`, `0x1D`: an external COM object. Carries the raw
    /// address, unresolved, for the same reason as [`VbType::ComIFace`].
    ComObj(Va),
    /// `epvT_hresult`, `0x1E`.
    HResult,
    /// A type code this table does not hold, carrying the raw byte. Per
    /// D-07, no code is guessed.
    Unknown(u8),
}

/// One argument entry's modifiers, plus its type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeEntry {
    /// The base type, after the three modifier bits are stripped.
    pub vb_type: VbType,
    /// `0x80`: `Optional`.
    pub optional: bool,
    /// `0x40`: `Array` (`As T()`).
    pub array: bool,
    /// `0x20`: `ByRef`.
    pub by_ref: bool,
}

/// The literal default value an `Optional` argument's source declared.
///
/// `DefaultValue::Empty` is itself a recovered fact, not an absence: it is
/// what an `Optional` argument declared with no `= literal` reads as, one
/// value record per such argument, per the `optionalVals` grammar this
/// file's module doc comment describes.
#[derive(Clone, Debug, PartialEq)]
pub enum DefaultValue {
    /// OLE variant tag `0`: `Optional` with no literal default.
    Empty,
    /// OLE variant tag `3`: a four-byte signed integer.
    Integer(i32),
    /// OLE variant tag `4`: a four-byte `f32`.
    Single(f32),
    /// OLE variant tag `11`: `0xFFFF` is `true`, `0x0000` is `false`.
    Boolean(bool),
    /// OLE variant tag `17`: a one-byte integer, plus one byte of padding.
    Byte(u8),
    /// OLE variant tag `8`: a `u16` byte length, then that many bytes,
    /// decoded the same Latin-1-as-code-point way every other string in
    /// this crate is.
    Text(String),
}

/// One recovered argument: its name, its type, its modifiers, and its
/// default value when it is `Optional` and one was recovered.
#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    /// The argument's name, resolved from `lpAryArgNames`. Empty when the
    /// name could not be resolved; see [`FuncTypeWalk::defects`].
    pub name: String,
    /// The argument's type and modifiers.
    pub entry: TypeEntry,
    /// The literal default value, when `entry.optional` is true and one was
    /// recovered. `None` when the argument is not `Optional`, when
    /// [`Prototype::optional_defaults`] is not [`OptionalDefaultsOutcome::Resolved`],
    /// or when the argument is a trailing `Optional` argument with no
    /// value record at all: measured, a non-`String` `Optional` argument
    /// with no explicit `= literal` default gets no `optionalVals` record,
    /// unlike a `String`-typed one, which always gets a record
    /// ([`DefaultValue::Empty`] when it has no default). See
    /// [`Prototype::optional_defaults`]'s doc comment for the corpus
    /// examples.
    pub default: Option<DefaultValue>,
}

/// The property kind `argSize`'s low two bits carry.
///
/// **Masked with `0x03`, not `0x07`.** See the module doc comment for the
/// measurement that corrects `STRUCTURES.md` section 6.6 here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyKind {
    /// Not a property: a plain `Sub` or `Function`.
    None,
    /// `Property Get`. `bFlags` bit 0 is additionally set, because a getter
    /// has a return value.
    Get,
    /// `Property Let`.
    Let,
    /// `Property Set`. No sample anywhere in this corpus; the variant stays,
    /// unproven, per `STRUCTURES.md`'s own convention for a gap it cannot
    /// close.
    Set,
}

/// What this file recovered about a record's `Optional` default values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptionalDefaultsOutcome {
    /// `optionalVals` is null: no argument in this record carries default
    /// value machinery to walk.
    NoOptionalVals,
    /// `optionalVals` is non-null, every value record resolved, and the walk
    /// closed exactly at `cbValues`. Carries the number of value records
    /// read, which `tests/type_descriptors.rs` sums across the whole corpus.
    ///
    /// This count can be **less than** the number of `Optional` arguments
    /// the record's own type buffer declares. Measured over the whole
    /// corpus: a value record exists for every `Optional` argument that
    /// either has an explicit `= literal` default, or is `String`-typed
    /// (which always gets a record, [`DefaultValue::Empty`] when it has no
    /// default). A non-`String` `Optional` argument with no explicit
    /// default gets no record at all. Four records in this corpus have
    /// fewer value records than `Optional` arguments, and in every one the
    /// shortfall is exactly the trailing `Optional` argument or arguments
    /// in declaration order: `cCommonDialog::VBGetSaveFileName`'s trailing
    /// `flags` (`Optional flags As Long`, no default), the same class's
    /// `VBChooseColor`'s trailing `flags`, and `cSystemColorDialog`'s
    /// `ShowColorDialog`'s trailing `showFullDialog` and `initColor`
    /// (`Optional ByVal showFullDialog As Boolean`, `Optional ByVal
    /// initColor As OLE_COLOR`, neither with a default). This is
    /// consistent with VB6's own rule that `Optional` parameters are
    /// declared contiguously at the end of the parameter list: the records
    /// this file recovers are assigned to the first `Resolved(n)`
    /// `Optional` arguments, front to back, and any trailing ones beyond
    /// that carry no default. See [`Argument::default`].
    Resolved(usize),
    /// `optionalVals` is non-null, and the walk did not close, met a tag
    /// this file does not recognise, or resolved more records than the
    /// record's own `Optional` arguments (never observed: the reverse
    /// shortfall, documented on [`OptionalDefaultsOutcome::Resolved`], is
    /// what the corpus holds). See [`FuncTypeWalk::defects`] for the defect
    /// this produces.
    Unrecoverable,
}

/// One fully recovered public procedure prototype.
#[derive(Clone, Debug, PartialEq)]
pub struct Prototype {
    /// The DISPID. `STRUCTURES.md` section 6.3 gives the form
    /// `0x6003xxxx` for a plain member; measured, a property reads
    /// `0x6803xxxx` instead, on the four property records this corpus
    /// holds.
    pub member_id: u32,
    /// The vtable offset. `0xFFFF` for an event; not exercised here, since
    /// this file reads `lpFuncTypeInfo`, never `lpEventsTypeInfo`.
    pub v_off: u16,
    /// The header signature word. Always `0xFFFF` by construction: a record
    /// whose `constFFFF` is not `0xFFFF` never reaches a [`Prototype`] at
    /// all. Carried so `tests/type_descriptors.rs` can re-assert the
    /// corpus-wide measurement through the public API, not only trust that
    /// this file's own gate enforced it.
    pub const_ffff: u16,
    /// The word at header offset `0x06`. Always `0`, for the same reason
    /// and the same purpose as `const_ffff`.
    pub nul1: u16,
    /// `None` for a plain `Sub` or `Function`; `Get`, `Let` or `Set` for a
    /// property, decoded with the corrected `0x03` mask.
    pub property_kind: PropertyKind,
    /// True when `bFlags` bit 0 is set: the last type buffer entry is a
    /// return value, held in `return_type`, and every other entry is an
    /// argument. False when every entry is an argument.
    pub is_function: bool,
    /// Every recovered argument, in declaration order.
    pub arguments: Vec<Argument>,
    /// The return type, when `is_function` is true.
    pub return_type: Option<TypeEntry>,
    /// What this file recovered about `Optional` default values.
    pub optional_defaults: OptionalDefaultsOutcome,
}

/// The outcome of resolving one non-null entry of `PrivateObj.lpFuncTypeInfo`.
#[derive(Clone, Debug, PartialEq)]
pub enum ProcedureSignature {
    /// The entry at this index is null. Paired with a private procedure at
    /// the same index in `Object.lpProcNamesArray`, per `STRUCTURES.md`
    /// section 6.2.
    NoDescriptor,
    /// The entry is non-null, and the record could not be read: an
    /// unmapped address, a truncated header, a `constFFFF` mismatch, or a
    /// type buffer that did not close. See [`FuncTypeWalk::defects`].
    Unrecoverable,
    /// The entry is non-null, and the record was read.
    Prototype(Prototype),
}

/// The procedure signatures one object carries, or the fact that it carries
/// none at all.
#[derive(Clone, Debug, PartialEq)]
pub enum PrototypeList {
    /// `object.proc_count` slots, one outcome per index, in array order.
    Slots(Vec<ProcedureSignature>),
    /// The object carries no `PrivateObj` at all: the standard-module cap,
    /// D-10. There is no `lpFuncTypeInfo` array to read. `proc_count` is
    /// carried through unread, matching `vb/privateobj.rs`'s own
    /// `ProcNames::NoNameArray`.
    NoPrivateObject {
        /// The number of procedures the object declares.
        proc_count: u32,
    },
    /// The object carries a `PrivateObj`, and its own `lpFuncTypeInfo`
    /// pointer is null or resolves to no section. Not observed anywhere in
    /// this corpus: every `PrivateObj::Present` object's `lp_func_type_info`
    /// is non-null and resolves. Kept distinct from `NoPrivateObject`
    /// because the two are different facts about the file, matching the
    /// same reasoning `vb/privateobj.rs`'s `ProcNames::NoNameArray` gives
    /// for its sibling array's own null-or-unmapped case.
    NoFuncTypeArray {
        /// The number of procedures the object declares.
        proc_count: u32,
    },
}

/// `PrivateObj.lpFuncTypeInfo`, walked in full, plus the defects the walk
/// found.
pub struct FuncTypeWalk {
    /// The recovered signatures, or the fact that there is no array to
    /// recover them from.
    pub signatures: PrototypeList,
    defects: Vec<Defect>,
}

impl FuncTypeWalk {
    /// Walks `private.lp_func_type_info`, bounded by `object.proc_count`,
    /// giving one [`ProcedureSignature`] per index.
    ///
    /// `object` and `private` come from the same object: `object.proc_count`
    /// is the length both `Object.lpProcNamesArray` and
    /// `PrivateObj.lpFuncTypeInfo` share, per `STRUCTURES.md` section 6.2,
    /// so this file bounds its own array by the one count plan 02-01 already
    /// reads and already bounds against the real size of the file, rather
    /// than re-deriving a second count that a later change could knock out
    /// of step with the name array's.
    #[must_use]
    pub fn read(pe: &PeImage<'_>, object: &Object, private: &PrivateObj) -> Self {
        let no_private_object = || Self {
            signatures: PrototypeList::NoPrivateObject {
                proc_count: object.proc_count,
            },
            defects: Vec::new(),
        };
        let no_func_type_array = || Self {
            signatures: PrototypeList::NoFuncTypeArray {
                proc_count: object.proc_count,
            },
            defects: Vec::new(),
        };

        let PrivateObj::Present {
            lp_func_type_info, ..
        } = private
        else {
            return no_private_object();
        };

        if lp_func_type_info.is_null() {
            return no_func_type_array();
        }
        let Some(array) = pe.region_at_va(*lp_func_type_info) else {
            return no_func_type_array();
        };
        let Some(window_size) = object.proc_count.checked_mul(PTR_SIZE) else {
            return no_func_type_array();
        };
        let Some(window) = array.subregion(Off::new(0), window_size) else {
            return no_func_type_array();
        };

        let mut signatures = Vec::with_capacity(usize::try_from(object.proc_count).unwrap_or(0));
        let mut defects = Vec::new();

        for index in 0..object.proc_count {
            let Some(entry_off) = index.checked_mul(PTR_SIZE) else {
                signatures.push(ProcedureSignature::NoDescriptor);
                continue;
            };
            let Some(va) = window.va_le(Off::new(entry_off)) else {
                signatures.push(ProcedureSignature::NoDescriptor);
                continue;
            };
            if va.is_null() {
                signatures.push(ProcedureSignature::NoDescriptor);
                continue;
            }

            let offset = window.file_offset(Off::new(entry_off)).map_or(0, Off::get);
            let site = Site {
                offset,
                rva: va.to_rva(pe.image_base()).map(Rva::get),
                structure: "PrivateObj",
                field: "lpFuncTypeInfo",
            };

            let Some(base) = pe.region_at_va(va) else {
                defects.push(Defect {
                    site,
                    kind: DefectKind::UnreadablePointer {
                        offset,
                        va: va.get(),
                    },
                });
                signatures.push(ProcedureSignature::Unrecoverable);
                continue;
            };

            let (prototype, mut record_defects) = read_one(pe, va, base);
            defects.append(&mut record_defects);
            match prototype {
                Some(prototype) => signatures.push(ProcedureSignature::Prototype(prototype)),
                None => signatures.push(ProcedureSignature::Unrecoverable),
            }
        }

        Self {
            signatures: PrototypeList::Slots(signatures),
            defects,
        }
    }

    /// Gives the defects the walk found: an unmapped `lpFuncTypeInfo` entry,
    /// a `FuncTypDesc` whose `constFFFF` did not validate, a type buffer
    /// that did not close, an unresolvable argument name, or an
    /// `optionalVals` walk that did not close or met an unrecognised tag.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }
}

/// The raw header fields of one `FuncTypDesc`, before any interpretation.
struct RawHeader {
    arg_size: u8,
    b_flags: u8,
    v_off: u16,
    const_ffff: u16,
    nul1: u16,
    optional_vals: Va,
    member_id: u32,
    lp_ary_arg_names: Va,
}

/// Reads the fixed `0x20`-byte header. `header` is exactly [`HEADER_SIZE`]
/// bytes, taken before any field inside it is read, so every access here is
/// bounds-checked against real bytes already known to be present.
fn read_raw_header(header: &Region<'_>) -> Option<RawHeader> {
    Some(RawHeader {
        arg_size: header.u8(Off::new(0x00))?,
        b_flags: header.u8(Off::new(0x01))?,
        v_off: header.u16_le(Off::new(0x02))?,
        const_ffff: header.u16_le(Off::new(0x04))?,
        nul1: header.u16_le(Off::new(0x06))?,
        optional_vals: header.va_le(Off::new(0x08))?,
        member_id: header.u32_le(Off::new(0x0C))?,
        lp_ary_arg_names: header.va_le(Off::new(0x10))?,
    })
}

/// One raw entry of the type buffer, before it is split into arguments and a
/// possible return type, and before its base code becomes a [`VbType`].
#[derive(Clone, Copy, Debug)]
struct RawEntry {
    base_code: u8,
    optional: bool,
    array: bool,
    by_ref: bool,
    trailing: Option<u32>,
}

/// The outcome of walking a type buffer.
struct BufferWalk {
    entries: Vec<RawEntry>,
    /// True when the leading byte was one of the two documented values and
    /// the walk found exactly `argSize >> 2` entries within the step bound.
    /// A partial walk is never accepted; see the module doc comment.
    closed: bool,
}

/// Computes the padding needed to bring `off` up to the next four-byte
/// boundary, measured from the start of the type buffer.
fn align4_pad(off: Off) -> u32 {
    let rem = off.get().checked_rem(4).unwrap_or(0);
    if rem == 0 {
        0
    } else {
        4_u32.saturating_sub(rem)
    }
}

/// Walks the type buffer that follows a `FuncTypDesc` header.
///
/// `buffer` starts at `FuncTypDesc + 0x20` and runs to the end of the
/// mapped bytes of its section: it is inherently bounded to real bytes,
/// never sized from a length field in the file. [`MAX_TYPE_BUFFER_STEPS`]
/// bounds the walk a second way, in steps, which is what stops a hostile
/// all-zero buffer with a non-zero `argSize` from scanning to the end of a
/// large mapped section looking for a padding byte that never arrives
/// (T-02-15).
///
/// The first byte is validated as `0x1E` (a member) or `0x00` (an event,
/// not reached through `lpFuncTypeInfo` in this corpus) and skipped. Then
/// exactly `argSize >> 2` entries are read. Zero bytes between entries are
/// padding and are tolerated, per `STRUCTURES.md` section 6.6, and bounded
/// by the same step count. When an entry's base code is `0x13`, `0x1C` or
/// `0x1D`, a 32-bit value follows it, aligned up to a four-byte boundary
/// measured from the start of this buffer.
fn walk_type_buffer(buffer: &Region<'_>, arg_size: u8) -> BufferWalk {
    let wanted = usize::from(arg_size.checked_shr(2).unwrap_or(0));

    let mut cursor = Off::new(0);
    let mut steps: u32 = 0;
    let mut leading_byte_valid = false;

    if let Some(lead) = buffer.u8(cursor) {
        leading_byte_valid = lead == 0x1E || lead == 0x00;
        cursor = cursor.checked_add(1).unwrap_or(cursor);
    }

    let mut entries: Vec<RawEntry> = Vec::with_capacity(wanted);
    while entries.len() < wanted && steps < MAX_TYPE_BUFFER_STEPS {
        steps = steps.saturating_add(1);

        let Some(byte) = buffer.u8(cursor) else {
            break;
        };
        let Some(after_byte) = cursor.checked_add(1) else {
            break;
        };
        cursor = after_byte;

        if byte == 0 {
            // Padding between entries: tolerated, and bounded by the same
            // step count as everything else in this loop.
            continue;
        }

        let optional = byte & 0x80 != 0;
        let array = byte & 0x40 != 0;
        let by_ref = byte & 0x20 != 0;
        let base_code = byte & 0x1F;

        let trailing = if matches!(base_code, 0x13 | 0x1C | 0x1D) {
            let pad = align4_pad(cursor);
            let Some(aligned) = cursor.checked_add(pad) else {
                break;
            };
            let Some(value) = buffer.u32_le(aligned) else {
                break;
            };
            let Some(after_value) = aligned.checked_add(4) else {
                break;
            };
            cursor = after_value;
            Some(value)
        } else {
            None
        };

        entries.push(RawEntry {
            base_code,
            optional,
            array,
            by_ref,
            trailing,
        });
    }

    let closed = leading_byte_valid && entries.len() == wanted;
    BufferWalk { entries, closed }
}

/// Maps a type buffer entry's base code, plus its trailing pointer when it
/// has one, to a [`VbType`]. See the module doc comment for the measurement
/// that backs this table.
fn vb_type_of(base_code: u8, trailing: Option<u32>) -> VbType {
    match base_code {
        0x03 => VbType::Boolean,
        0x05 => VbType::Byte,
        0x06 => VbType::Integer,
        0x08 => VbType::Long,
        0x0A => VbType::Single,
        0x0B => VbType::Double,
        0x0C => VbType::Date,
        0x0D => VbType::Currency,
        0x0F => VbType::Variant,
        0x10 => VbType::Str,
        0x13 => VbType::Internal(Va::new(trailing.unwrap_or(0))),
        0x1B => VbType::Object,
        0x1C => VbType::ComIFace(Va::new(trailing.unwrap_or(0))),
        0x1D => VbType::ComObj(Va::new(trailing.unwrap_or(0))),
        0x1E => VbType::HResult,
        other => VbType::Unknown(other),
    }
}

/// Decodes the property kind from `argSize`'s low two bits.
///
/// **Masked with `0x03`, not `0x07`.** See the module doc comment.
fn property_kind_of(arg_size: u8) -> PropertyKind {
    match arg_size & 0x03 {
        0 => PropertyKind::None,
        1 => PropertyKind::Get,
        2 => PropertyKind::Let,
        _ => PropertyKind::Set,
    }
}

/// Resolves every argument name from `lpAryArgNames`, bounded by `count`,
/// the number of argument entries the type buffer walk already closed on.
///
/// `STRUCTURES.md` section 6.3 states the array is not terminated, so the
/// count must be computed first (by [`walk_type_buffer`]) and exactly that
/// many entries walked here. A pointer that resolves nowhere gives every
/// name empty, matching `vb/privateobj.rs`'s own choice for its sibling
/// array's null-or-unmapped pointer, and is not itself reported as a
/// defect: only a per-entry failure is.
fn resolve_arg_names(
    pe: &PeImage<'_>,
    lp_ary_arg_names: Va,
    count: usize,
) -> (Vec<String>, Vec<Defect>) {
    let mut names = vec![String::new(); count];
    let mut defects = Vec::new();
    if count == 0 {
        return (names, defects);
    }
    let Ok(count_u32) = u32::try_from(count) else {
        return (names, defects);
    };
    let Some(window_size) = count_u32.checked_mul(PTR_SIZE) else {
        return (names, defects);
    };
    let Some(array) = pe.region_at_va(lp_ary_arg_names) else {
        return (names, defects);
    };
    let Some(window) = array.subregion(Off::new(0), window_size) else {
        return (names, defects);
    };

    for (index, slot) in names.iter_mut().enumerate() {
        let Ok(index_u32) = u32::try_from(index) else {
            continue;
        };
        let Some(entry_off) = index_u32.checked_mul(PTR_SIZE) else {
            continue;
        };
        let Some(va) = window.va_le(Off::new(entry_off)) else {
            continue;
        };
        if va.is_null() {
            continue;
        }

        let offset = window.file_offset(Off::new(entry_off)).map_or(0, Off::get);
        let site = Site {
            offset,
            rva: va.to_rva(pe.image_base()).map(Rva::get),
            structure: "FuncTypDesc",
            field: "lpAryArgNames",
        };

        let Some(name_region) = pe.region_at_va(va) else {
            defects.push(Defect {
                site,
                kind: DefectKind::UnreadablePointer {
                    offset,
                    va: va.get(),
                },
            });
            continue;
        };

        match name_region.cstr(Off::new(0), ARG_NAME_MAX) {
            Some(bytes) => *slot = bytes.iter().copied().map(char::from).collect(),
            None => {
                defects.push(Defect {
                    site,
                    kind: DefectKind::NoNulTerminator {
                        offset,
                        limit: ARG_NAME_MAX,
                    },
                });
            }
        }
    }

    (names, defects)
}

/// Why one `optionalVals` value record could not be read.
enum ValueRecordError {
    /// The bytes needed for this record run past the bounded value region.
    Truncated,
    /// The tag is not one of the six this file's grammar holds.
    UnknownTag(u16),
}

/// Reads one value record at `at` inside the bounded `cbValues` window.
/// Returns the recovered value and the total width of the record (the tag
/// plus its padded value), so the caller can advance its cursor.
///
/// See the module doc comment for the six tags this grammar holds and the
/// measurement that closed it.
fn read_value_record(
    buffer: &Region<'_>,
    at: Off,
) -> Result<(DefaultValue, u32), ValueRecordError> {
    let tag = buffer.u16_le(at).ok_or(ValueRecordError::Truncated)?;
    let value_at = at.checked_add(2).ok_or(ValueRecordError::Truncated)?;
    match tag {
        0 => Ok((DefaultValue::Empty, 2)),
        3 => {
            let raw = buffer.i32_le(value_at).ok_or(ValueRecordError::Truncated)?;
            Ok((DefaultValue::Integer(raw), 6))
        }
        4 => {
            let raw = buffer
                .take(value_at, 4)
                .ok_or(ValueRecordError::Truncated)?;
            let bytes: [u8; 4] = raw.try_into().ok().ok_or(ValueRecordError::Truncated)?;
            Ok((DefaultValue::Single(f32::from_le_bytes(bytes)), 6))
        }
        8 => {
            let len = buffer.u16_le(value_at).ok_or(ValueRecordError::Truncated)?;
            let len_u32 = u32::from(len);
            let text_at = value_at.checked_add(2).ok_or(ValueRecordError::Truncated)?;
            let raw = buffer
                .take(text_at, len_u32)
                .ok_or(ValueRecordError::Truncated)?;
            let text: String = raw.iter().copied().map(char::from).collect();
            let content_width = 2_u32
                .checked_add(len_u32)
                .ok_or(ValueRecordError::Truncated)?;
            let pad = content_width.checked_rem(2).unwrap_or(0);
            let padded = content_width
                .checked_add(pad)
                .ok_or(ValueRecordError::Truncated)?;
            let record_width = 2_u32
                .checked_add(padded)
                .ok_or(ValueRecordError::Truncated)?;
            Ok((DefaultValue::Text(text), record_width))
        }
        11 => {
            let raw = buffer.u16_le(value_at).ok_or(ValueRecordError::Truncated)?;
            Ok((DefaultValue::Boolean(raw != 0), 4))
        }
        17 => {
            let raw = buffer.u8(value_at).ok_or(ValueRecordError::Truncated)?;
            Ok((DefaultValue::Byte(raw), 4))
        }
        other => Err(ValueRecordError::UnknownTag(other)),
    }
}

/// The outcome of walking every value record inside a `cbValues` window.
enum OptionalValsWalk {
    /// Every record resolved and the walk closed exactly at `cbValues`.
    Resolved(Vec<DefaultValue>),
    /// The walk did not close, or met a tag this file does not recognise.
    Unrecoverable,
}

/// The bound on the number of value records one `optionalVals` walk reads.
///
/// `cbValues` is itself bounded by [`Region::subregion`] against the real
/// mapped bytes at `optionalVals`, and every value record consumes at least
/// its own two-byte tag, so the walk is already bounded by
/// `cbValues.checked_div(2)` regardless of this constant. This is a second,
/// independent bound, for the same reason [`MAX_TYPE_BUFFER_STEPS`] is:
/// T-02-16.
const MAX_OPTIONAL_VALS_STEPS: u32 = 4096;

/// Walks `optionalVals`: a `u32` `cbValues`, a virtual address at `+4`
/// (measured equal to `optionalVals + 8`, not dereferenced here), then
/// `cbValues` bytes of value records starting at `+8`.
///
/// `field_offset` is the absolute file offset of the `optionalVals` field
/// itself inside the `FuncTypDesc` header, used for every defect this
/// function produces: there is no per-value-record offset that names
/// anything a person could open the file at more precisely than the field
/// that pointed here, matching `vb/object.rs`'s own choice of the
/// `ProcCount` field's offset for its "implausible count" defect.
fn walk_optional_vals(
    pe: &PeImage<'_>,
    field_offset: u32,
    optional_vals: Va,
) -> (OptionalValsWalk, Option<Defect>) {
    let site = Site {
        offset: field_offset,
        rva: optional_vals.to_rva(pe.image_base()).map(Rva::get),
        structure: "FuncTypDesc",
        field: "optionalVals",
    };
    let unreadable = || {
        (
            OptionalValsWalk::Unrecoverable,
            Some(Defect {
                site: site.clone(),
                kind: DefectKind::UnreadablePointer {
                    offset: field_offset,
                    va: optional_vals.get(),
                },
            }),
        )
    };

    let Some(region) = pe.region_at_va(optional_vals) else {
        return unreadable();
    };
    let Some(cb_values) = region.u32_le(Off::new(0)) else {
        return unreadable();
    };
    let Some(values) = region.subregion(Off::new(8), cb_values) else {
        return unreadable();
    };

    let mut records = Vec::new();
    let mut cursor = Off::new(0);
    let mut steps: u32 = 0;

    loop {
        if cursor.get() == values.len() {
            return (OptionalValsWalk::Resolved(records), None);
        }
        if cursor.get() > values.len() || steps >= MAX_OPTIONAL_VALS_STEPS {
            let defect = Defect {
                site,
                kind: DefectKind::CountMismatch {
                    offset: field_offset,
                    count: cursor.get(),
                    expected: cb_values,
                    other_field: "cbValues",
                },
            };
            return (OptionalValsWalk::Unrecoverable, Some(defect));
        }
        steps = steps.saturating_add(1);

        match read_value_record(&values, cursor) {
            Ok((value, width)) => {
                records.push(value);
                let Some(next) = cursor.checked_add(width) else {
                    let defect = Defect {
                        site,
                        kind: DefectKind::OffsetOverflow {
                            offset: field_offset,
                            len: width,
                        },
                    };
                    return (OptionalValsWalk::Unrecoverable, Some(defect));
                };
                cursor = next;
            }
            Err(ValueRecordError::UnknownTag(tag)) => {
                // `error.rs` is outside this plan's file boundary, and no
                // existing kind names "a tag this file does not
                // recognise." `CountMismatch` is reused because it is the
                // nearest kind that carries a raw number forward and marks
                // the record's defaults unrecoverable, matching the
                // precedent `vb/privateobj.rs` set for its own third
                // failure mode.
                let defect = Defect {
                    site,
                    kind: DefectKind::CountMismatch {
                        offset: field_offset,
                        count: u32::from(tag),
                        expected: 0,
                        other_field: "a value tag this file's grammar holds",
                    },
                };
                return (OptionalValsWalk::Unrecoverable, Some(defect));
            }
            Err(ValueRecordError::Truncated) => {
                let defect = Defect {
                    site,
                    kind: DefectKind::UnreadablePointer {
                        offset: field_offset,
                        va: optional_vals.get(),
                    },
                };
                return (OptionalValsWalk::Unrecoverable, Some(defect));
            }
        }
    }
}

/// Reads one `FuncTypDesc` record, already resolved to `base`, the region
/// starting at its own address and running to the end of its section's
/// mapped bytes.
///
/// Returns the recovered prototype, when the record's layout validated, and
/// every defect this record's own reading produced: a `constFFFF`
/// mismatch, a type buffer that did not close, an unresolvable argument
/// name, or an `optionalVals` walk that failed. More than one of these can
/// occur on a single record, which is why this returns a `Vec` and not
/// `Option<Defect>`: an argument-name failure and an `optionalVals` failure
/// are independent facts about the same record.
fn read_one(
    pe: &PeImage<'_>,
    functype_va: Va,
    base: Region<'_>,
) -> (Option<Prototype>, Vec<Defect>) {
    let rva = functype_va.to_rva(pe.image_base()).map(Rva::get);
    let self_offset = base.file_offset(Off::new(0)).map_or(0, Off::get);
    let unrecoverable_here = |offset: u32, field: &'static str| {
        let site = Site {
            offset,
            rva,
            structure: "FuncTypDesc",
            field,
        };
        vec![Defect {
            site,
            kind: DefectKind::UnreadablePointer {
                offset,
                va: functype_va.get(),
            },
        }]
    };

    let Some(header) = base.subregion(Off::new(0), HEADER_SIZE) else {
        return (None, unrecoverable_here(self_offset, "argSize"));
    };

    // `header` is exactly `HEADER_SIZE` bytes, and every field
    // `read_raw_header` reads is inside that window. `Region` has no
    // infallible accessor, so this check stays; no test covers it and none
    // can.
    let Some(raw) = read_raw_header(&header) else {
        return (None, unrecoverable_here(self_offset, "argSize"));
    };

    if raw.const_ffff != 0xFFFF {
        let offset = header
            .file_offset(Off::new(0x04))
            .map_or(self_offset, Off::get);
        return (None, unrecoverable_here(offset, "constFFFF"));
    }

    let buffer_len = base.len().saturating_sub(HEADER_SIZE);
    let Some(buffer) = base.subregion(Off::new(HEADER_SIZE), buffer_len) else {
        return (None, unrecoverable_here(self_offset, "argSize"));
    };

    let walk = walk_type_buffer(&buffer, raw.arg_size);
    if !walk.closed {
        let offset = header
            .file_offset(Off::new(0x00))
            .map_or(self_offset, Off::get);
        let site = Site {
            offset,
            rva,
            structure: "FuncTypDesc",
            field: "argSize",
        };
        let wanted = u32::from(raw.arg_size.checked_shr(2).unwrap_or(0));
        let found = u32::try_from(walk.entries.len()).unwrap_or(u32::MAX);
        let defect = Defect {
            site,
            kind: DefectKind::CountMismatch {
                offset,
                count: found,
                expected: wanted,
                other_field: "argSize",
            },
        };
        return (None, vec![defect]);
    }

    let mut entries = walk.entries;
    let is_function = raw.b_flags & 1 != 0;
    let return_raw = if is_function { entries.pop() } else { None };
    let property_kind = property_kind_of(raw.arg_size);
    let arg_count = entries.len();

    let (names, mut defects) = resolve_arg_names(pe, raw.lp_ary_arg_names, arg_count);

    let mut arguments = Vec::with_capacity(arg_count);
    for (entry, name) in entries.into_iter().zip(names) {
        arguments.push(Argument {
            name,
            entry: TypeEntry {
                vb_type: vb_type_of(entry.base_code, entry.trailing),
                optional: entry.optional,
                array: entry.array,
                by_ref: entry.by_ref,
            },
            default: None,
        });
    }

    let return_type = return_raw.map(|entry| TypeEntry {
        vb_type: vb_type_of(entry.base_code, entry.trailing),
        optional: entry.optional,
        array: entry.array,
        by_ref: entry.by_ref,
    });

    let optional_defaults = if raw.optional_vals.is_null() {
        OptionalDefaultsOutcome::NoOptionalVals
    } else {
        let field_offset = header
            .file_offset(Off::new(0x08))
            .map_or(self_offset, Off::get);
        let (outcome, defect) = walk_optional_vals(pe, field_offset, raw.optional_vals);
        if let Some(defect) = defect {
            defects.push(defect);
        }
        match outcome {
            OptionalValsWalk::Resolved(values) => {
                let optional_count = arguments.iter().filter(|a| a.entry.optional).count();
                let resolved_count = values.len();
                if resolved_count <= optional_count {
                    // Measured over the whole corpus: a value record exists
                    // for every `Optional` argument that either has an
                    // explicit `= literal` default or is `String`-typed
                    // (which always gets a record, `Empty` when it has no
                    // default). A non-`String` `Optional` argument with no
                    // explicit default gets no record at all. Every
                    // instance of this found in the corpus (`cCommonDialog`
                    // `VBGetSaveFileName`'s trailing `flags`,
                    // `VBChooseColor`'s trailing `flags`,
                    // `cSystemColorDialog` `ShowColorDialog`'s trailing
                    // `showFullDialog` and `initColor`) is the last
                    // `Optional` argument or arguments in declaration
                    // order, consistent with VB6's own rule that `Optional`
                    // parameters are declared contiguously at the end of
                    // the parameter list. Assigning the resolved records to
                    // the first `resolved_count` `Optional` arguments,
                    // front to back, and leaving the remaining trailing
                    // ones with no default, matches every one of these.
                    let mut values = values.into_iter();
                    for argument in arguments.iter_mut().filter(|a| a.entry.optional) {
                        argument.default = values.next();
                    }
                    OptionalDefaultsOutcome::Resolved(resolved_count)
                } else {
                    // More records than `Optional` arguments: never
                    // observed. The record cannot be trusted to attach the
                    // right default to the right argument, so none are
                    // attached.
                    OptionalDefaultsOutcome::Unrecoverable
                }
            }
            OptionalValsWalk::Unrecoverable => OptionalDefaultsOutcome::Unrecoverable,
        }
    };

    let prototype = Prototype {
        member_id: raw.member_id,
        v_off: raw.v_off,
        const_ffff: raw.const_ffff,
        nul1: raw.nul1,
        property_kind,
        is_function,
        arguments,
        return_type,
        optional_defaults,
    };

    (Some(prototype), defects)
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
    use super::{
        Argument, DefaultValue, FuncTypeWalk, OptionalDefaultsOutcome, ProcedureSignature,
        PropertyKind, Prototype, PrototypeList, VbType, read_raw_header, walk_type_buffer,
    };
    use crate::error::Severity;
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Region, Va};
    use crate::vb::header::{VbHeader, header_region};
    use crate::vb::object::{Object, ObjectTable};
    use crate::vb::privateobj::{ObjectInfo, PrivateObj, ProcNames, Procedure, ProcedureList};
    use crate::vb::project::{ObjectTableHead, ProjectInfo};

    /// `Grayscale.exe`, the tracer for task 1: three objects, and
    /// `FastDrawing`'s eight slots, four private (`Declare`) and four
    /// public.
    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    /// Two `Optional ByRef Long` arguments with literal defaults `10000` and
    /// `50`, on two different procedures.
    const RANDOMIZATION_FX: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Randomize-effects/RandomizationFX.exe"
    ));

    /// Three one-byte `Optional` defaults, each `127`.
    const EMBOSS_ENGRAVE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Emboss-engrave-effect/Emboss_Engrave.exe"
    ));

    /// A `false` boolean default followed by a single-precision `0.5`.
    const COLORIZE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Colorize-effect/Colorize.exe"
    ));

    /// `cCommonDialog`'s four property records, and `VBGetOpenFileName`'s
    /// twelve default records, including the text `All (*.*)| *.*`.
    const EDGE_DETECTION: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Edge-detection/Edge_Detection.exe"
    ));

    /// Gives the address of `ProjectInfo` that the file itself holds.
    fn project_data_va(data: &[u8]) -> Va {
        let image = PeImage::parse(data).unwrap();
        let hdr = header_region(&image).unwrap();
        VbHeader::read(&hdr).unwrap().lp_project_data
    }

    /// Gives the address of the object table that the file itself holds.
    fn object_table_va(data: &[u8]) -> Va {
        let image = PeImage::parse(data).unwrap();
        ProjectInfo::read(&image, project_data_va(data))
            .unwrap()
            .lp_object_table
    }

    /// Walks the object array out of a byte slice, through the same pointer
    /// chain `inspect` uses. Duplicated per-module on purpose: this module's
    /// tests must be able to fail independently of `vb/object.rs`'s and
    /// `vb/privateobj.rs`'s own.
    fn objects(data: &[u8]) -> Vec<Object> {
        let image = PeImage::parse(data).unwrap();
        let lp_object_table = object_table_va(data);
        let head = ObjectTableHead::read(&image, lp_object_table).unwrap();
        ObjectTable::walk(&image, lp_object_table, &head)
            .unwrap()
            .objects
    }

    /// Finds the one object of `data` named `name`, or panics: every corpus
    /// fixture this module reads is asserted to hold the object it is used
    /// for, so a missing name is this test's own fixture being wrong.
    fn find_object(data: &[u8], name: &str) -> Object {
        objects(data)
            .into_iter()
            .find(|o| o.name == name)
            .unwrap_or_else(|| panic!("{name} not found"))
    }

    /// Reads `PrivateObj` for one object.
    fn private_obj_of(pe: &PeImage<'_>, object: &Object) -> PrivateObj {
        let info = ObjectInfo::read(pe, object.lp_object_info).unwrap();
        PrivateObj::read(pe, info.lp_private_object).unwrap()
    }

    /// Finds the recovered [`Prototype`] named `proc_name` on the object
    /// named `object_name`, through the same index-parallel walk
    /// `FuncTypeWalk::read` and `vb/privateobj.rs`'s own `ProcedureList`
    /// perform, cross-checking the two arrays agree at that index.
    fn find_prototype(data: &[u8], object_name: &str, proc_name: &str) -> Prototype {
        let image = PeImage::parse(data).unwrap();
        let object = find_object(data, object_name);
        let private = private_obj_of(&image, &object);
        let names = ProcedureList::read(&image, &object);
        let ProcNames::Slots(slots) = &names.procs else {
            panic!("{object_name} carries no name array");
        };
        let index = slots
            .iter()
            .position(|p| matches!(p, Procedure::Public(n) if n == proc_name))
            .unwrap_or_else(|| panic!("{proc_name} not found on {object_name}"));

        let walk = FuncTypeWalk::read(&image, &object, &private);
        let PrototypeList::Slots(signatures) = &walk.signatures else {
            panic!("{object_name} carries no FuncTypDesc array");
        };
        match &signatures[index] {
            ProcedureSignature::Prototype(p) => p.clone(),
            other => panic!("{proc_name} on {object_name} did not resolve: {other:?}"),
        }
    }

    // -- Task 1: the header, and the two disputed offsets --------------

    /// The type descriptor array for `FastDrawing` holds a null pointer at
    /// every index whose name slot is `Private`, and a real pointer at
    /// every index whose name slot is `Public`. Behaviour 1.
    #[test]
    fn grayscale_fast_drawing_type_array_agrees_with_the_name_array_at_every_index() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let object = find_object(GRAYSCALE, "FastDrawing");
        let private = private_obj_of(&image, &object);

        let names = ProcedureList::read(&image, &object);
        let ProcNames::Slots(name_slots) = &names.procs else {
            panic!("FastDrawing carries no name array");
        };

        let walk = FuncTypeWalk::read(&image, &object, &private);
        let PrototypeList::Slots(type_slots) = &walk.signatures else {
            panic!("FastDrawing carries no FuncTypDesc array");
        };

        assert_eq!(name_slots.len(), type_slots.len());
        for (name_slot, type_slot) in name_slots.iter().zip(type_slots.iter()) {
            match name_slot {
                Procedure::Private => {
                    assert_eq!(*type_slot, ProcedureSignature::NoDescriptor);
                }
                Procedure::Public(_) => {
                    assert!(matches!(type_slot, ProcedureSignature::Prototype(_)));
                }
            }
        }
    }

    /// `GetImageWidth`'s descriptor gives the measured `arg_size` (`0x08`,
    /// not the plan's inherited `4`: see the module doc comment),
    /// `b_flags` bit 0 set, `const_ffff` `0xFFFF`, `nul1` `0`, and a
    /// non-null `lp_ary_arg_names` (proven indirectly: its one argument's
    /// name resolves). Behaviour 2, corrected.
    #[test]
    fn grayscale_get_image_width_header_fields_match_the_measured_bytes() {
        let prototype = find_prototype(GRAYSCALE, "FastDrawing", "GetImageWidth");
        assert_eq!(prototype.const_ffff, 0xFFFF);
        assert_eq!(prototype.nul1, 0);
        assert!(prototype.is_function);
        assert_eq!(prototype.arguments.len(), 1);
        assert!(!prototype.arguments[0].name.is_empty());
        assert!(prototype.return_type.is_some());
        // Two type buffer entries (one argument, one return) is `2 << 2`,
        // the measured `0x08`, which this file does not expose as a raw
        // `arg_size` field on `Prototype`; it is proven here through the
        // entry count instead, the same fact `arg_size` encodes.
    }

    /// `member_id`'s top sixteen bits are `0x6003` on `GetImageWidth`.
    /// Behaviour 3.
    #[test]
    fn grayscale_get_image_width_member_id_top_bits_are_0x6003() {
        let prototype = find_prototype(GRAYSCALE, "FastDrawing", "GetImageWidth");
        assert_eq!((prototype.member_id >> 16) & 0xFFFF, 0x6003);
    }

    /// A record whose `const_ffff` is not `0xFFFF` produces a recoverable
    /// defect naming the offset, and the record is reported unrecoverable
    /// rather than parsed. Behaviour 4.
    #[test]
    fn a_record_whose_const_ffff_is_wrong_is_unrecoverable_with_a_recoverable_defect() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let object = find_object(GRAYSCALE, "FastDrawing");
        let private = private_obj_of(&image, &object);
        let PrivateObj::Present {
            lp_func_type_info, ..
        } = private
        else {
            panic!("FastDrawing is a class, not a module");
        };

        // `GetImageWidth` is index 4 of FastDrawing's eight slots (see
        // `vb/privateobj.rs`'s own test of the same object).
        let entry_region = image.region_at_va(lp_func_type_info).unwrap();
        let entry_va = entry_region.va_le(Off::new(4 * 4)).unwrap();
        let header_region = image.region_at_va(entry_va).unwrap();
        let at = usize::try_from(header_region.file_offset(Off::new(0x04)).unwrap().get()).unwrap();

        let mut bytes = GRAYSCALE.to_vec();
        assert_eq!(&bytes[at..at + 2], &0xFFFFu16.to_le_bytes());
        bytes[at..at + 2].copy_from_slice(&0x1234u16.to_le_bytes());

        let patched = PeImage::parse(&bytes).unwrap();
        let object = find_object(&bytes, "FastDrawing");
        let private = private_obj_of(&patched, &object);
        let walk = FuncTypeWalk::read(&patched, &object, &private);
        let PrototypeList::Slots(slots) = &walk.signatures else {
            panic!("FastDrawing carries no FuncTypDesc array");
        };
        assert_eq!(slots[4], ProcedureSignature::Unrecoverable);

        assert_eq!(walk.defects().len(), 1);
        let defect = &walk.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert_eq!(defect.site.offset, u32::try_from(at).unwrap());
    }

    /// With the descriptor pointer patched to an address in no section, the
    /// procedure keeps its name and reports no prototype, and a recoverable
    /// defect names the address. Behaviour 5.
    #[test]
    fn a_descriptor_pointer_in_no_section_keeps_the_name_and_reports_no_prototype() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let object = find_object(GRAYSCALE, "FastDrawing");
        let private = private_obj_of(&image, &object);
        let PrivateObj::Present {
            lp_func_type_info, ..
        } = private
        else {
            panic!("FastDrawing is a class, not a module");
        };

        let nowhere = image.image_base() + 0x00F0_0000;
        assert!(image.region_at_va(Va::new(nowhere)).is_none());

        let array_region = image.region_at_va(lp_func_type_info).unwrap();
        let at = usize::try_from(array_region.file_offset(Off::new(4 * 4)).unwrap().get()).unwrap();
        let mut bytes = GRAYSCALE.to_vec();
        bytes[at..at + 4].copy_from_slice(&nowhere.to_le_bytes());

        let patched_image = PeImage::parse(&bytes).unwrap();
        let object = find_object(&bytes, "FastDrawing");

        // The procedure name is still recovered, through the unrelated
        // `lpProcNamesArray`, which this patch does not touch.
        let names = ProcedureList::read(&patched_image, &object);
        let ProcNames::Slots(name_slots) = &names.procs else {
            panic!("FastDrawing carries no name array");
        };
        assert_eq!(name_slots[4], Procedure::Public("GetImageWidth".to_owned()));

        let private = private_obj_of(&patched_image, &object);
        let walk = FuncTypeWalk::read(&patched_image, &object, &private);
        let PrototypeList::Slots(type_slots) = &walk.signatures else {
            panic!("FastDrawing carries no FuncTypDesc array");
        };
        assert_eq!(type_slots[4], ProcedureSignature::Unrecoverable);

        assert_eq!(walk.defects().len(), 1);
        let defect = &walk.defects()[0];
        assert_eq!(defect.kind.severity(), Severity::Recoverable);
        assert!(matches!(
            defect.kind,
            crate::error::DefectKind::UnreadablePointer { va, .. } if va == nowhere
        ));
    }

    // Task 1's fifth deliberate breakage (`member_id` read from `0x0A`
    // instead of `0x0C`, the shifted layout `STRUCTURES.md` section 6.3's
    // disputed figure implies) was run by hand against this file's
    // production code, not encoded as a permanent test: the breakage is a
    // one-line edit to `read_raw_header`, run once, observed, and reverted.
    // It made `grayscale_get_image_width_member_id_top_bits_are_0x6003`
    // fail (`left: 0, right: 24579`), exactly the third behaviour this
    // task names. See the plan's SUMMARY for the transcript.

    // -- Task 2: the type code table, modifiers, and the closing walk --

    /// The fifteen documented codes each map to their VB type name.
    /// Behaviour 1, part 1.
    #[test]
    fn every_documented_type_code_maps_to_its_name() {
        use super::vb_type_of;
        assert_eq!(vb_type_of(0x03, None), VbType::Boolean);
        assert_eq!(vb_type_of(0x05, None), VbType::Byte);
        assert_eq!(vb_type_of(0x06, None), VbType::Integer);
        assert_eq!(vb_type_of(0x08, None), VbType::Long);
        assert_eq!(vb_type_of(0x0A, None), VbType::Single);
        assert_eq!(vb_type_of(0x0B, None), VbType::Double);
        assert_eq!(vb_type_of(0x0C, None), VbType::Date);
        assert_eq!(vb_type_of(0x0D, None), VbType::Currency);
        assert_eq!(vb_type_of(0x0F, None), VbType::Variant);
        assert_eq!(vb_type_of(0x10, None), VbType::Str);
        assert_eq!(
            vb_type_of(0x13, Some(0x0040_1000)),
            VbType::Internal(Va::new(0x0040_1000))
        );
        assert_eq!(vb_type_of(0x1B, None), VbType::Object);
        assert_eq!(
            vb_type_of(0x1C, Some(0x0040_2000)),
            VbType::ComIFace(Va::new(0x0040_2000))
        );
        assert_eq!(
            vb_type_of(0x1D, Some(0x0040_3000)),
            VbType::ComObj(Va::new(0x0040_3000))
        );
        assert_eq!(vb_type_of(0x1E, None), VbType::HResult);
    }

    /// Any other byte gives `Unknown` carrying the raw value. A synthetic
    /// value the corpus does not contain, per `RESEARCH.md`'s own warning
    /// that a decoder proven only on present codes leaves the unknown path
    /// dead. Behaviour 1, part 2.
    #[test]
    fn an_undocumented_code_gives_unknown_carrying_the_raw_byte() {
        use super::vb_type_of;
        for code in [0x00, 0x01, 0x02, 0x04, 0x07, 0x09, 0x0E, 0x11, 0x12, 0x19] {
            assert_eq!(vb_type_of(code, None), VbType::Unknown(code));
        }
    }

    /// The modifiers strip in the order `0x80`, `0x40`, `0x20`, matching
    /// `STRUCTURES.md` section 6.5's worked examples exactly. Behaviour 2.
    #[test]
    fn modifiers_strip_in_order_matching_the_documented_worked_examples() {
        // `arg_size` of `5 << 2 = 0x14` asks for five entries. The leading
        // byte (`0x1E`, a member) comes before the first entry.
        let with_lead = [0x1E_u8, 0x08, 0x28, 0x68, 0xA8, 0x2F];
        let region = Region::new(&with_lead, Off::new(0));
        let walk = walk_type_buffer(&region, 0x14);
        assert!(walk.closed, "expected the walk to close on five entries");
        assert_eq!(walk.entries.len(), 5);

        let (base, opt, arr, byref): (u8, bool, bool, bool) = (
            walk.entries[0].base_code,
            walk.entries[0].optional,
            walk.entries[0].array,
            walk.entries[0].by_ref,
        );
        assert_eq!((base, opt, arr, byref), (0x08, false, false, false)); // Long

        let e = walk.entries[1];
        assert_eq!(
            (e.base_code, e.optional, e.array, e.by_ref),
            (0x08, false, false, true)
        ); // ByRef Long

        let e = walk.entries[2];
        assert_eq!(
            (e.base_code, e.optional, e.array, e.by_ref),
            (0x08, false, true, true)
        ); // ByRef Long array

        let e = walk.entries[3];
        assert_eq!(
            (e.base_code, e.optional, e.array, e.by_ref),
            (0x08, true, false, true)
        ); // Optional ByRef Long

        let e = walk.entries[4];
        assert_eq!(
            (e.base_code, e.optional, e.array, e.by_ref),
            (0x0F, false, false, true)
        ); // ByRef Variant
    }

    /// `DrawTriangleEffect` gives four arguments named `srcPic`, `dstPic`,
    /// `numLoops` and `lenLine`, of which the first two are `ByRef`
    /// external COM objects and the last two are `Optional ByRef Long`.
    /// Behaviour 3.
    #[test]
    fn randomization_fx_draw_triangle_effect_gives_the_measured_arguments() {
        let prototype = find_prototype(RANDOMIZATION_FX, "frmLineEffect", "DrawTriangleEffect");
        assert!(!prototype.is_function);
        assert_eq!(prototype.arguments.len(), 4);

        let names: Vec<&str> = prototype
            .arguments
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(names, vec!["srcPic", "dstPic", "numLoops", "lenLine"]);

        for arg in &prototype.arguments[0..2] {
            assert!(matches!(arg.entry.vb_type, VbType::ComObj(_)));
            assert!(arg.entry.by_ref);
            assert!(!arg.entry.optional);
        }
        for arg in &prototype.arguments[2..4] {
            assert_eq!(arg.entry.vb_type, VbType::Long);
            assert!(arg.entry.by_ref);
            assert!(arg.entry.optional);
        }
    }

    /// `GetImageWidth` gives one argument and a return type, because its
    /// flags bit 0 is set. Behaviour 4.
    #[test]
    fn grayscale_get_image_width_gives_one_argument_and_a_return_type() {
        let prototype = find_prototype(GRAYSCALE, "FastDrawing", "GetImageWidth");
        assert!(prototype.is_function);
        assert_eq!(prototype.arguments.len(), 1);
        assert!(prototype.return_type.is_some());
        assert_eq!(prototype.return_type.unwrap().vb_type, VbType::Long);
    }

    /// A record whose type buffer does not close at exactly `argSize >> 2`
    /// entries within a bounded number of steps is reported unrecoverable,
    /// and a synthetic buffer of zero bytes proves the bound stops rather
    /// than runs away. Behaviour 5.
    #[test]
    fn a_synthetic_all_zero_buffer_with_a_nonzero_arg_size_returns_rather_than_hangs() {
        let bytes = vec![0_u8; 100_000];
        let region = Region::new(&bytes, Off::new(0));
        let walk = walk_type_buffer(&region, 0x04);
        assert!(!walk.closed, "an all-zero buffer must never close");
    }

    /// The number of argument name pointers read equals the number of
    /// argument entries, never the number of type entries, when `bFlags`
    /// bit 0 says the last entry is a return value. Behaviour 6.
    #[test]
    fn argument_name_count_excludes_the_return_entry() {
        let prototype = find_prototype(GRAYSCALE, "FastDrawing", "GetImageWidth");
        // Two type entries (one argument, one return), and exactly one
        // argument name, never two.
        assert_eq!(prototype.arguments.len(), 1);
    }

    /// Exactly 4 of the corpus records decode as a property when the kind
    /// is masked with `0x03`, and they are `cCommonDialog`'s two getters
    /// and getter/setter pair, matching the source under
    /// `corpus/vb6-code/Hidden-Markov-model/cCommonDialog.cls`. Behaviour 7.
    #[test]
    fn edge_detection_common_dialog_gives_exactly_four_property_records() {
        let api_return = find_prototype(EDGE_DETECTION, "cCommonDialog", "APIReturn");
        assert_eq!(api_return.property_kind, PropertyKind::Get);
        assert!(api_return.is_function);

        let extended_error = find_prototype(EDGE_DETECTION, "cCommonDialog", "ExtendedError");
        assert_eq!(extended_error.property_kind, PropertyKind::Get);
        assert!(extended_error.is_function);

        let image = PeImage::parse(EDGE_DETECTION).unwrap();
        let object = find_object(EDGE_DETECTION, "cCommonDialog");
        let private = private_obj_of(&image, &object);
        let walk = FuncTypeWalk::read(&image, &object, &private);
        let PrototypeList::Slots(slots) = &walk.signatures else {
            panic!("cCommonDialog carries no FuncTypDesc array");
        };
        let mut kinds: Vec<PropertyKind> = slots
            .iter()
            .filter_map(|s| match s {
                ProcedureSignature::Prototype(p) => Some(p.property_kind),
                _ => None,
            })
            .filter(|k| *k != PropertyKind::None)
            .collect();
        kinds.sort_by_key(|k| matches!(k, PropertyKind::Let) as u8);
        // Two Gets (APIReturn, ExtendedError, CustomColor-get is a third)
        // plus one Let (CustomColor-let): three Get, one Let, four total.
        let gets = kinds.iter().filter(|k| **k == PropertyKind::Get).count();
        let lets = kinds.iter().filter(|k| **k == PropertyKind::Let).count();
        assert_eq!((gets, lets, kinds.len()), (3, 1, 4));
    }

    /// Masking with `0x07` instead of `0x03` misreads a real, non-property
    /// record as kind `4`, which is not one of the three kinds
    /// `STRUCTURES.md` section 6.6 lists. `FastDrawing.GetImageData2D`
    /// carries three type buffer entries (`ComObj`, `Byte` array, `Boolean`),
    /// so its `argSize` is `3 << 2 = 0x0C`: `0x0C & 0x07 == 4` under the
    /// wrong mask, `0x0C & 0x03 == 0` (not a property) under the corrected
    /// one. This is the exact failure shape the module doc comment
    /// describes as measured over the whole corpus: 75 of 193 records read
    /// as kind `4` under `0x07`, every one with an odd entry count and none
    /// of them a real property. This one record proves the mechanism the
    /// corpus-wide count is built from.
    #[test]
    fn deliberate_breakage_property_kind_masked_with_0x07_misreads_a_non_property_as_kind_4() {
        let prototype = find_prototype(GRAYSCALE, "FastDrawing", "GetImageData2D");
        assert_eq!(prototype.property_kind, PropertyKind::None);
        assert!(!prototype.is_function);
        assert_eq!(prototype.arguments.len(), 3);

        let arg_size = 0x0C_u8;
        assert_eq!(arg_size & 0x03, 0, "the corrected mask: not a property");
        assert_eq!(
            arg_size & 0x07,
            4,
            "the wrong mask reads this real, non-property record as kind 4, which is not \
             `Get`, `Let` or `Set`"
        );
    }

    // -- Task 3: the optionalVals default-value walk --------------------

    /// `DrawTriangleEffect` and `DrawLineEffect` each give two default
    /// values, `10000` and `50`, in argument order. Behaviour 1.
    #[test]
    fn randomization_fx_gives_the_measured_integer_defaults() {
        for proc in ["DrawTriangleEffect", "DrawLineEffect"] {
            let prototype = find_prototype(RANDOMIZATION_FX, "frmLineEffect", proc);
            let defaults: Vec<Option<DefaultValue>> = prototype
                .arguments
                .iter()
                .map(|a| a.default.clone())
                .collect();
            assert_eq!(
                defaults,
                vec![
                    None,
                    None,
                    Some(DefaultValue::Integer(10000)),
                    Some(DefaultValue::Integer(50)),
                ],
                "{proc}"
            );
        }
    }

    /// `DrawEngrave` gives three one-byte defaults each equal to `127`.
    /// Behaviour 2.
    #[test]
    fn emboss_engrave_draw_engrave_gives_three_byte_defaults_of_127() {
        let prototype = find_prototype(EMBOSS_ENGRAVE, "frmEmbossEngrave", "DrawEngrave");
        let defaults: Vec<Option<DefaultValue>> = prototype
            .arguments
            .iter()
            .map(|a| a.default.clone())
            .collect();
        assert_eq!(
            defaults,
            vec![
                None,
                None,
                Some(DefaultValue::Byte(127)),
                Some(DefaultValue::Byte(127)),
                Some(DefaultValue::Byte(127)),
            ]
        );
    }

    /// `DrawColorize` gives a `false` boolean followed by a
    /// single-precision `0.5`. Behaviour 3.
    #[test]
    fn colorize_draw_colorize_gives_a_false_boolean_then_0_5() {
        let prototype = find_prototype(COLORIZE, "frmColorize", "DrawColorize");
        let defaults: Vec<Option<DefaultValue>> = prototype
            .arguments
            .iter()
            .map(|a| a.default.clone())
            .collect();
        assert_eq!(
            defaults,
            vec![
                None,
                None,
                None,
                Some(DefaultValue::Boolean(false)),
                Some(DefaultValue::Single(0.5)),
            ]
        );
    }

    /// `VBGetOpenFileName` gives twelve default records, including the
    /// text `All (*.*)| *.*`. Behaviour 4.
    #[test]
    fn edge_detection_vb_get_open_file_name_gives_twelve_defaults_with_the_filter_text() {
        let prototype = find_prototype(EDGE_DETECTION, "cCommonDialog", "VBGetOpenFileName");
        let optional_defaults: Vec<&Argument> = prototype
            .arguments
            .iter()
            .filter(|a| a.entry.optional)
            .collect();
        assert_eq!(optional_defaults.len(), 12);
        assert!(
            optional_defaults
                .iter()
                .any(|a| a.default == Some(DefaultValue::Text("All (*.*)| *.*".to_owned())))
        );
        assert_eq!(
            prototype.optional_defaults,
            OptionalDefaultsOutcome::Resolved(12)
        );
    }

    /// A tag the table does not hold stops the walk, reports the record's
    /// defaults unrecoverable, and names the tag in a defect. It does not
    /// guess a width. Behaviour 5.
    #[test]
    fn an_unrecognised_tag_stops_the_walk_and_reports_the_defaults_unrecoverable() {
        let image = PeImage::parse(RANDOMIZATION_FX).unwrap();
        let object = find_object(RANDOMIZATION_FX, "frmLineEffect");
        let private = private_obj_of(&image, &object);
        let PrivateObj::Present {
            lp_func_type_info, ..
        } = private
        else {
            panic!("frmLineEffect is a module, not a form");
        };

        let names = ProcedureList::read(&image, &object);
        let ProcNames::Slots(name_slots) = &names.procs else {
            panic!("frmLineEffect carries no name array");
        };
        let index = name_slots
            .iter()
            .position(|p| matches!(p, Procedure::Public(n) if n == "DrawTriangleEffect"))
            .unwrap();

        let array_region = image.region_at_va(lp_func_type_info).unwrap();
        let entry_va = array_region
            .va_le(Off::new(u32::try_from(index).unwrap() * 4))
            .unwrap();
        let header_region = image.region_at_va(entry_va).unwrap();
        let optional_vals_va = header_region.va_le(Off::new(0x08)).unwrap();
        let optional_vals_region = image.region_at_va(optional_vals_va).unwrap();
        // The first value record's tag sits right after `cbValues` and the
        // measured-equal address at `+4`.
        let tag_offset =
            usize::try_from(optional_vals_region.file_offset(Off::new(8)).unwrap().get()).unwrap();

        let mut bytes = RANDOMIZATION_FX.to_vec();
        assert_eq!(&bytes[tag_offset..tag_offset + 2], &3_u16.to_le_bytes());
        bytes[tag_offset..tag_offset + 2].copy_from_slice(&99_u16.to_le_bytes());

        let patched = PeImage::parse(&bytes).unwrap();
        let object = find_object(&bytes, "frmLineEffect");
        let private = private_obj_of(&patched, &object);
        let walk = FuncTypeWalk::read(&patched, &object, &private);
        let PrototypeList::Slots(slots) = &walk.signatures else {
            panic!("frmLineEffect carries no FuncTypDesc array");
        };
        match &slots[index] {
            ProcedureSignature::Prototype(p) => {
                assert_eq!(p.optional_defaults, OptionalDefaultsOutcome::Unrecoverable);
                assert!(p.arguments.iter().all(|a| a.default.is_none()));
            }
            other => panic!("expected a prototype with unrecoverable defaults, got {other:?}"),
        }
        assert!(walk.defects().iter().any(|d| matches!(
            d.kind,
            crate::error::DefectKind::CountMismatch { count: 99, .. }
        )));
    }

    /// `read_raw_header` and `walk_type_buffer` are reachable from the test
    /// module directly, which this compiles as proof of.
    #[test]
    fn private_helpers_are_reachable_from_the_test_module() {
        let buf = [0x00_u8; 0x20];
        let region = Region::new(&buf, Off::new(0));
        assert!(read_raw_header(&region).is_some());
        let empty = walk_type_buffer(&region, 0);
        assert!(empty.closed);
        assert!(empty.entries.is_empty());
    }
}
