//! `ObjectInfo`, `PrivateObj`, and the procedure name walk.
//!
//! This is OBJ-03 and OBJ-06: recover every public procedure name an object
//! carries, and report a private procedure as private rather than inventing a
//! name for it. It also carries [`PrivateObj`]'s two open counts,
//! `cnt_public_vars` and `cnt_events`, and the one array plan 02-05 adds a
//! walk for: [`event_descriptor_addresses`].
//!
//! # Two structures this file is named for and does not implement
//!
//! Plan 02-05 owns the public variable and event half of this file, and it
//! ships neither a `PubVarDesc` walk nor an event decoder. Both are measured
//! absences in this corpus, not unfinished work:
//!
//! - `cnt_public_vars` does not count source-level `Public` declarations
//!   (D-14). [`PrivateObj::gaps`] carries a non-zero value to the report as
//!   an open question, and no function anywhere in this crate walks
//!   `lpPublicVars`.
//! - No corpus program carries a single `EventDesc`: `cnt_events` is `0` in
//!   97 of 97 objects that hold a `PrivateObj`. [`event_descriptor_addresses`]
//!   walks the pointer array anyway, proven only by synthetic fixtures, and
//!   a named test in this file's own test module fires the day a real
//!   sample arrives. No event name is ever produced (D-09).
//!
//! # Two null cases, not one
//!
//! `STRUCTURES.md` section 5.1 says a null entry in `Object.lpProcNamesArray`
//! means the procedure at that index is private. That is a fact about one
//! entry. A whole `lpProcNamesArray` **pointer** of `0` is a different fact
//! about the object: measured across the 44 corpus programs, this is exactly
//! the 8 of 105 objects that are standard modules (`.bas`), and their real
//! `ProcCount` (1 to 7) is never reachable through this structure at all. The
//! two cases are modelled as two different values of two different shapes so
//! that a caller cannot mistake "every procedure is private" for "this object
//! carries no procedure names whatsoever". See [`ProcNames`].
//!
//! # A non-null entry is not necessarily a name either
//!
//! `CONTEXT.md`, "The procedure name array does not behave as documented",
//! corrects the worked example this plan inherited. `Mandelbrot.exe`'s
//! `frmFractal` was asserted to hold nine **null** entries. Measured, its
//! nine entries are **not** null: the array was never written by the
//! compiler and still holds a fragment of a build-machine path,
//! `mData\Oracle\Java\`, read back as nine little-endian dwords. Every one of
//! those nine procedures genuinely is private (the source declares all nine
//! `Private`), but the reason is not "the entry is null". Every non-null
//! entry is therefore validated before it is trusted, never merely resolved
//! and printed. See [`ProcedureList::read`] for the exact test this file
//! applies, and why it applies it.
//!
//! # The standard-module cap, stated here before any ratio is pinned
//!
//! GD's own research, which `RESEARCH.md` section 6 carries forward, states
//! the reason plainly: this type data is emitted into `.text` as part of the
//! standard `IDispatch` plumbing every user form, class and user control
//! needs at run time, and the compiler cannot strip it. A standard module is
//! not such an object. Its procedures are declared, `Object.ProcCount`
//! reports them correctly, and yet none of them has a name or a prototype
//! reachable through `Object`, `ObjectInfo` or `PrivateObj`. Measured: 8 of 8
//! corpus module objects carry a null `lpProcNamesArray` pointer, with
//! `ProcCount` from 1 to 7. This is a cap on what OBJ-03 and OBJ-04 can ever
//! promise for a `.bas`, not a per-file anomaly, and it is named here, before
//! plan 02-09 pins the first ratio, per decision D-10.

use crate::error::{Defect, DefectKind, Refusal, Site};
use crate::read::pe::PeImage;
use crate::read::region::{Off, Region, Rva, Va};
use crate::vb::object::Object;

/// The size of the `ObjectInfo` structure.
///
/// `STRUCTURES.md` section 5.2 gives `0x38` = 56 bytes.
const OBJECT_INFO_SIZE: u32 = 0x38;

/// The size of the `PrivateObj` structure.
///
/// `STRUCTURES.md` section 6.1 gives `0x40` = 64 bytes.
const PRIVATE_OBJ_SIZE: u32 = 0x40;

/// The sentinel `ObjectInfo.lpPrivateObject` carries for a standard module.
///
/// `STRUCTURES.md` section 5.2 cites Semi VB Decompiler's note that this
/// field is `-1` for a `.bas`. Measured: it is also plainly `0` in a
/// synthetic value nothing in the corpus needs, and `PrivateObj::read` treats
/// both as the same fact, per the plan's own instruction.
const NO_PRIVATE_OBJECT: u32 = 0xFFFF_FFFF;

/// The width of one entry in `Object.lpProcNamesArray`.
const PROC_NAME_PTR_SIZE: u32 = 4;

/// The width of one entry in `PrivateObj.lpEventsTypeInfo`.
///
/// Both arrays hold a 32-bit virtual address per entry, so this equals
/// [`PROC_NAME_PTR_SIZE`]; it is a separate constant because the two arrays
/// are unrelated facts about the file that happen to share a width, not the
/// same fact under two names.
const EVENT_DESC_PTR_SIZE: u32 = 4;

/// The bound on a procedure name string.
///
/// This is deliberately tighter than the `0x104` bound every other string in
/// this crate uses for an object or a project name. `CONTEXT.md`'s own
/// measurement script used exactly this bound as part of the test that
/// separates a real procedure name from an uninitialised array entry read
/// back as a virtual address: 64 bytes is generous for a VB6 identifier
/// (whose language limit is far shorter) and still tight enough that an
/// unrelated run of in-image bytes is unlikely to happen to hold a NUL within
/// it.
const PROC_NAME_MAX: u32 = 64;

/// The head of `ObjectInfo`, reached from `Object.lpObjectInfo`.
///
/// Two fields are read, per `STRUCTURES.md` section 5.2. `wMethodCount` and
/// `lpMethods` point into code in a native build and are Phase 3 and later
/// work; `lpProjectData` is in-memory scratch. None of the three is read
/// here, and this doc comment is the reason a later reader should not extend
/// [`ObjectInfo::read`] to reach for them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectInfo {
    /// This object's own index in the object array.
    pub w_object_index: u16,
    /// The address of this object's `PrivateObj`, or the sentinel that means
    /// there is none.
    ///
    /// This is a raw `u32` and not a [`Va`]. `-1` is a sentinel, not an
    /// address: a `Va` that carried `0xFFFF_FFFF` would invite a caller to
    /// resolve it, and there is nothing at that address to resolve.
    /// [`PrivateObj::read`] takes this raw value and decides the sentinel
    /// question before it builds a [`Va`] from anything.
    pub lp_private_object: u32,
}

impl ObjectInfo {
    /// Reads `ObjectInfo` at the address `Object.lpObjectInfo` holds.
    ///
    /// The window is exactly [`OBJECT_INFO_SIZE`] bytes, taken before any
    /// field is read, matching the window-before-fields discipline
    /// `vb/project.rs` documents.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when the address is in no section, and
    /// when the file holds fewer than 56 bytes there.
    pub fn read(pe: &PeImage<'_>, lp_object_info: Va) -> Result<Self, Refusal> {
        let at = pe
            .region_at_va(lp_object_info)
            .ok_or(Refusal::Damaged("the ObjectInfo pointer is in no section"))?;
        let window = at
            .subregion(Off::new(0), OBJECT_INFO_SIZE)
            .ok_or(Refusal::Damaged(
                "the file ends inside the ObjectInfo structure",
            ))?;
        Ok(Self {
            w_object_index: u16_at(&window, 0x02, "ObjectInfo holds no object index")?,
            lp_private_object: u32_at(
                &window,
                0x0C,
                "ObjectInfo holds no address for its PrivateObj",
            )?,
        })
    }
}

/// `PrivateObj`, reached from `ObjectInfo.lpPrivateObject`.
///
/// # The module sentinel is a case, not an error
///
/// A standard module (`.bas`) is not a COM object, so it carries no
/// `PrivateObj` at all: `STRUCTURES.md` section 5.2 records that
/// `lpPrivateObject` reads `-1` for one, and this file also treats a plain
/// `0` the same way. Refusing here would lose every module in the corpus, so
/// the absence is a state this type carries rather than an error this
/// function returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrivateObj {
    /// The object carries a real `PrivateObj`.
    Present {
        /// The number of entries in the `PubVarDesc` array, as
        /// `STRUCTURES.md` section 6.1 names it.
        ///
        /// **Distrust this field.** Per decision D-14, it does not count
        /// source-level `Public variable As Type` declarations, and no
        /// stride hypothesis over `lpPublicVars` ever converges against it
        /// (`STRUCTURES.md` section 11, gap 8, stays open). Three measured
        /// counter-examples, each against a source file this repository
        /// vendors beside the binary that reports it:
        ///
        /// - `pdOpenSaveDialog.cls`, present in 15 of the 44 corpus
        ///   programs, declares zero `Public variable As Type` lines (only
        ///   a `Public Enum`), and every corpus binary reports `4`.
        /// - `frmMain.frm` in `Artificial-life` declares zero `Public`
        ///   anything and reports `5`, while carrying 10 top-level named
        ///   controls.
        /// - `Organism.cls` in `Artificial-life` declares 17 real public
        ///   variables, some on comma-joined lines, and reports `23`.
        ///
        /// This is the same shape `wCompiledObjects` turned out to be
        /// (`CONTEXT.md`, "One field to distrust by default"): a rounded
        /// capacity, not a count, found only because a project declaring
        /// one or two objects reported four. The resolution path is a
        /// controlled compile-and-diff experiment against a class with a
        /// known, isolated set of `Public` declarations, never more reading
        /// of this document. This field is carried, unexplained;
        /// [`PrivateObj::gaps`] surfaces a non-zero value to the report as
        /// an open question rather than an answer. Nothing downstream
        /// should be built on the assumption that it means what its name
        /// says.
        cnt_public_vars: u16,
        /// The number of entries in the `EventDesc` array.
        ///
        /// Measured `0` in 97 of 97 corpus objects that carry a
        /// `PrivateObj` at all (the other 8 of the 105 corpus objects are
        /// standard modules and have no `PrivateObj` to carry this field).
        /// `[VERIFIED: local]` [`event_descriptor_addresses`] walks the
        /// pointer array this count sizes, against synthetic fixtures only,
        /// for exactly this reason: the corpus carries no sample of a
        /// non-zero value, and this file's own test module carries a named
        /// test that fires the day one does.
        cnt_events: u16,
        /// The address of the `FuncTypDesc` pointer array.
        ///
        /// Index-parallel to `Object.lpProcNamesArray`, and of the same
        /// length, `Object.proc_count`. `STRUCTURES.md` section 6.1 states
        /// plainly that the count for this array is not carried in
        /// `PrivateObj` at all; it comes from `Object.ProcCount`, which plan
        /// 02-01 already reads and already bounds against the real size of
        /// the file. Plan 02-04 walks this array. Nothing here re-derives a
        /// count for it.
        lp_func_type_info: Va,
        /// The address of the `EventDesc` pointer array.
        ///
        /// An array of `cnt_events` pointers, per `STRUCTURES.md` section
        /// 6.2's asymmetry note, never inline records. Walked by
        /// [`event_descriptor_addresses`], which gives the address of each
        /// entry and decodes none of them.
        lp_events_type_info: Va,
        /// The address of the `PubVarDesc` array.
        ///
        /// `STRUCTURES.md` gap 8 records that no record stride hypothesis
        /// over this array converges against the corpus. No function in
        /// this crate walks it; [`PrivateObj::gaps`] is the only thing this
        /// file does with it, and that is to say a non-zero
        /// `cnt_public_vars` is a fact nobody has explained yet.
        lp_public_vars: Va,
    },
    /// The object carries no `PrivateObj`. This is the standard-module case.
    Absent,
}

impl PrivateObj {
    /// Reads `PrivateObj` at the address `ObjectInfo.lpPrivateObject` holds,
    /// or reports its absence.
    ///
    /// `lp_private_object` is the raw value [`ObjectInfo::lp_private_object`]
    /// carries. A plain `0` and the sentinel `0xFFFF_FFFF` both mean the
    /// object has no private object, per the doc comment on [`PrivateObj`],
    /// and neither is dereferenced: the sentinel check happens before this
    /// function builds a [`Va`] from anything.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Damaged`] when the address is a real address but it
    /// is in no section, and when the file holds fewer than 64 bytes there.
    pub fn read(pe: &PeImage<'_>, lp_private_object: u32) -> Result<Self, Refusal> {
        if lp_private_object == 0 || lp_private_object == NO_PRIVATE_OBJECT {
            return Ok(Self::Absent);
        }

        let va = Va::new(lp_private_object);
        let at = pe
            .region_at_va(va)
            .ok_or(Refusal::Damaged("the PrivateObj pointer is in no section"))?;
        let window = at
            .subregion(Off::new(0), PRIVATE_OBJ_SIZE)
            .ok_or(Refusal::Damaged(
                "the file ends inside the PrivateObj structure",
            ))?;

        Ok(Self::Present {
            cnt_public_vars: u16_at(&window, 0x10, "PrivateObj holds no public variable count")?,
            cnt_events: u16_at(&window, 0x12, "PrivateObj holds no event count")?,
            lp_func_type_info: va_at(
                &window,
                0x18,
                "PrivateObj holds no address for its FuncTypDesc array",
            )?,
            lp_public_vars: va_at(
                &window,
                0x20,
                "PrivateObj holds no address for its public variables",
            )?,
            lp_events_type_info: va_at(
                &window,
                0x24,
                "PrivateObj holds no address for its EventDesc array",
            )?,
        })
    }

    /// Gives the raw `cntPublicVars` field this object's `PrivateObj`
    /// carries, or `None` for a standard module, which has no `PrivateObj`
    /// to carry it.
    ///
    /// Named after the field, not after what the field is supposed to
    /// mean: this gives the count the structure holds, and per D-14 that
    /// count is not a number of recovered public variables. See the doc
    /// comment on `cnt_public_vars` inside [`PrivateObj::Present`] for the
    /// three measured counter-examples that make the distrust necessary.
    #[must_use]
    pub fn public_var_field(&self) -> Option<u16> {
        match self {
            Self::Present {
                cnt_public_vars, ..
            } => Some(*cnt_public_vars),
            Self::Absent => None,
        }
    }

    /// Gives this object's open questions about its `PrivateObj` fields.
    ///
    /// Today this is exactly one question, when it applies at all: per
    /// D-14, [`Self::public_var_field`] does not count source-level
    /// `Public` declarations, and this file does not know what it counts.
    /// A standard module and an object whose count is `0` both give an
    /// empty list. A `0` agrees, trivially, with "zero declared public
    /// variables", so it raises no question worth reporting; a non-zero
    /// count is the fact this plan cannot explain, and it reaches the
    /// report as a [`Gap`] rather than silently.
    ///
    /// Plan 02-10 prints this list next to the object it belongs to. This
    /// function resolves nothing about the gap; it only says that one
    /// exists.
    #[must_use]
    pub fn gaps(&self) -> Vec<Gap> {
        match self.public_var_field() {
            Some(count) if count != 0 => vec![Gap::UnexplainedPublicVarCount(count)],
            Some(_) | None => Vec::new(),
        }
    }
}

/// An open question one object's `PrivateObj` fields raise, carried to the
/// report as a fact to investigate rather than answered by a guess.
///
/// Nothing in this crate resolves a `Gap`. It exists so the report can say
/// plainly what DeForm6 does not know, per D-07: an unresolved structure is
/// a reported gap, not an implementation written from a figure and never
/// exercised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    /// `cnt_public_vars` is non-zero, and per D-14 that count does not name
    /// a number of source-level `Public` declarations. `STRUCTURES.md`
    /// gap 8 records the record stride behind `lpPublicVars` as unresolved:
    /// no fixed or type-conditional stride closes cleanly over the corpus.
    /// This carries the raw count and nothing else; no variable name is
    /// claimed from it.
    UnexplainedPublicVarCount(u16),
}

/// One procedure slot: a recovered public name, or a private procedure.
///
/// Per OBJ-06, [`Procedure::Private`] carries nothing. There is no index
/// number, no placeholder and no name derived from a vtable offset: a
/// private procedure has no name in this file, and none is invented for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Procedure {
    /// A recovered public procedure name.
    Public(String),
    /// A private procedure, or an entry this file could not validate as a
    /// name. See [`ProcedureList::read`] for what "could not validate"
    /// means, and why an unresolvable entry ends up here rather than being
    /// printed as a recovered name.
    Private,
}

/// The procedure slots one object carries, or the fact that it carries none.
///
/// These two states are kept apart on purpose, per D-13: an empty list and
/// "this object has no name array at all" are different facts about the
/// file, and collapsing them would erase the difference the standard-module
/// cap depends on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcNames {
    /// `Object.proc_count` slots, each resolved to [`Procedure::Public`] or
    /// [`Procedure::Private`].
    Slots(Vec<Procedure>),
    /// The object carries no `lpProcNamesArray` at all: the pointer itself is
    /// null (or, defensively, resolves to no section). `proc_count` is
    /// still the number of procedures the object declares; there is simply
    /// no array to read their names through. Measured: this is exactly the
    /// 8 of 105 corpus objects that are standard modules, with `proc_count`
    /// from 1 to 7. See the module doc comment.
    NoNameArray {
        /// The number of procedures the object declares, carried through
        /// unread: there is nothing here to bound it against.
        proc_count: u32,
    },
}

/// The three numbers a report needs for one object's procedures: how many
/// slots it declares, how many of those this file recovered a public name
/// for, and whether it carries a name array at all.
///
/// Plan 02-09 pins this and plan 02-10 prints it. Putting the three numbers
/// together here stops each of them from recomputing the arithmetic
/// differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcedureCounts {
    /// The number of procedure slots the object declares.
    pub declared: u32,
    /// The number of those slots recovered as [`Procedure::Public`].
    pub recovered: u32,
    /// True when the object carries no procedure name array at all (the
    /// `.bas` cap, D-13). When true, `recovered` is always `0`, and OBJ-03
    /// cannot be attempted for this object through this structure at all.
    pub no_name_array: bool,
}

impl ProcedureCounts {
    /// Computes the three numbers from a [`ProcNames`] value.
    ///
    /// This file never averages, rounds or computes a ratio: that
    /// arithmetic is plan 02-09's, pinned in one place. This gives the two
    /// counts a ratio is built from, and nothing else.
    #[must_use]
    pub fn of(procs: &ProcNames) -> Self {
        match procs {
            ProcNames::Slots(slots) => {
                let recovered = slots
                    .iter()
                    .filter(|proc| matches!(proc, Procedure::Public(_)))
                    .count();
                Self {
                    declared: u32::try_from(slots.len()).unwrap_or(u32::MAX),
                    recovered: u32::try_from(recovered).unwrap_or(u32::MAX),
                    no_name_array: false,
                }
            }
            ProcNames::NoNameArray { proc_count } => Self {
                declared: *proc_count,
                recovered: 0,
                no_name_array: true,
            },
        }
    }
}

/// The procedure slots one object carries, resolved from
/// `Object.lpProcNamesArray`, plus the defects the walk found.
pub struct ProcedureList {
    /// The recovered slots, or the fact that there is no array to recover
    /// them from.
    pub procs: ProcNames,
    defects: Vec<Defect>,
}

impl ProcedureList {
    /// Walks `object.lp_proc_names_array`, giving one [`Procedure`] per
    /// entry.
    ///
    /// # The pointer is checked before the loop
    ///
    /// `RESEARCH.md`'s "Anti-Patterns to Avoid" names this as the first trap
    /// of the phase: code that only tests `entry == 0` inside a loop over
    /// `proc_count` either skips the loop by accident, or, if it checks the
    /// pointer first and then unconditionally trusts `proc_count`, ends up
    /// dereferencing address zero. This function checks
    /// `object.lp_proc_names_array.is_null()` and returns
    /// [`ProcNames::NoNameArray`] before anything calls
    /// [`PeImage::region_at_va`], so a null pointer is never resolved and no
    /// byte is ever read at address zero. A non-null pointer that
    /// nonetheless resolves to no section (not observed anywhere in the
    /// corpus) is treated the same way, for the same reason, and is not
    /// reported as a defect: [`crate::vb::project::DeclareTable::read`]
    /// treats an unmapped table pointer identically, and this file follows
    /// that precedent.
    ///
    /// # Every non-null entry is validated before it is trusted
    ///
    /// This is the correction the module doc comment describes.
    /// `STRUCTURES.md` section 5.1 states that a null entry means the
    /// procedure at that index is private, and that rule is safe in the
    /// direction it is used: a null entry never names a procedure. The
    /// converse does not hold. A non-null entry earns
    /// [`Procedure::Public`] only when every one of these holds:
    ///
    /// 1. It resolves inside a mapped section
    ///    ([`PeImage::region_at_va`]).
    /// 2. The bytes there are NUL terminated within [`PROC_NAME_MAX`] bytes.
    /// 3. The first byte is an ASCII letter or an underscore.
    /// 4. Every byte is an ASCII alphanumeric character or an underscore.
    ///
    /// This is the exact test `CONTEXT.md`'s own measurement script used:
    /// 193 of the corpus's entries passed it, agreeing exactly with the 193
    /// `FuncTypDesc` records plan 02-04 counts by a wholly independent
    /// route. An entry that fails any part of this is [`Procedure::Private`],
    /// plus a defect naming the offset and the raw address, so an
    /// uninitialised array entry is reported as a gap and never presented as
    /// a recovered name.
    ///
    /// # `Mandelbrot.exe`'s `frmFractal`
    ///
    /// This is the corpus case that exercises the correction directly.
    /// Every one of its nine entries is non-null, and every one fails step 1
    /// above: read as a virtual address, each resolves to no section, because
    /// the array was never written by the compiler and holds a fragment of a
    /// build-machine path instead. All nine therefore give
    /// [`Procedure::Private`], each with its own defect, which is the
    /// corrected fact: not nine null entries, but nine uninitialised ones.
    #[must_use]
    pub fn read(pe: &PeImage<'_>, object: &Object) -> Self {
        if object.lp_proc_names_array.is_null() {
            return Self {
                procs: ProcNames::NoNameArray {
                    proc_count: object.proc_count,
                },
                defects: Vec::new(),
            };
        }

        let no_name_array = || Self {
            procs: ProcNames::NoNameArray {
                proc_count: object.proc_count,
            },
            defects: Vec::new(),
        };

        let Some(array) = pe.region_at_va(object.lp_proc_names_array) else {
            return no_name_array();
        };
        let Some(window_size) = object.proc_count.checked_mul(PROC_NAME_PTR_SIZE) else {
            return no_name_array();
        };
        let Some(window) = array.subregion(Off::new(0), window_size) else {
            return no_name_array();
        };

        let mut procs = Vec::with_capacity(usize::try_from(object.proc_count).unwrap_or(0));
        let mut defects = Vec::new();
        for index in 0..object.proc_count {
            let (proc, defect) = resolve_entry(pe, &window, index);
            procs.push(proc);
            if let Some(defect) = defect {
                defects.push(defect);
            }
        }

        Self {
            procs: ProcNames::Slots(procs),
            defects,
        }
    }

    /// Gives the defects the walk found: an entry that resolved to no
    /// section, one with no NUL terminator within [`PROC_NAME_MAX`] bytes, or
    /// one whose bytes are not a plausible identifier.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }
}

/// Resolves one entry of `lpProcNamesArray` to a [`Procedure`], plus the
/// defect this file records when the entry is non-null but does not survive
/// validation.
///
/// `window` is the `proc_count * 4` byte subregion [`ProcedureList::read`]
/// already took, so every offset here is `index * 4`. The fallbacks below
/// `window.va_le` cannot be reached, because `index < proc_count` and the
/// window is exactly `proc_count * 4` bytes: they stay because `Region` has
/// no infallible accessor, and no test covers them for that reason.
fn resolve_entry(pe: &PeImage<'_>, window: &Region<'_>, index: u32) -> (Procedure, Option<Defect>) {
    let Some(entry_off) = index.checked_mul(PROC_NAME_PTR_SIZE) else {
        return (Procedure::Private, None);
    };
    let Some(va) = window.va_le(Off::new(entry_off)) else {
        return (Procedure::Private, None);
    };
    if va.is_null() {
        return (Procedure::Private, None);
    }

    let offset = window.file_offset(Off::new(entry_off)).map_or(0, Off::get);
    let site = Site {
        offset,
        rva: va.to_rva(pe.image_base()).map(Rva::get),
        structure: "Object",
        field: "lpProcNamesArray",
    };

    let Some(name_region) = pe.region_at_va(va) else {
        let defect = Defect {
            site,
            kind: DefectKind::UnreadablePointer {
                offset,
                va: va.get(),
            },
        };
        return (Procedure::Private, Some(defect));
    };

    let Some(bytes) = name_region.cstr(Off::new(0), PROC_NAME_MAX) else {
        let defect = Defect {
            site,
            kind: DefectKind::NoNulTerminator {
                offset,
                limit: PROC_NAME_MAX,
            },
        };
        return (Procedure::Private, Some(defect));
    };

    if is_plausible_identifier(bytes) {
        let name = bytes.iter().copied().map(char::from).collect();
        (Procedure::Public(name), None)
    } else {
        // The address resolved, and a NUL terminator was found, but the
        // bytes do not read as an identifier: the same shape of evidence a
        // build-machine path fragment would leave if a garbage dword ever
        // happened to land inside a mapped section. Not observed anywhere
        // in the corpus, this file does not use `error.rs`'s
        // `UnmappedAddress`, because the address did resolve; it reuses
        // `UnreadablePointer`, which already carries exactly the raw value
        // CONTEXT.md asks a gap to carry, and is the nearest fit among the
        // kinds this plan's file boundary leaves reachable.
        let defect = Defect {
            site,
            kind: DefectKind::UnreadablePointer {
                offset,
                va: va.get(),
            },
        };
        (Procedure::Private, Some(defect))
    }
}

/// Tells whether a run of bytes reads as a plausible VB6 identifier.
///
/// The rule is `CONTEXT.md`'s own measurement rule: the first byte is an
/// ASCII letter or an underscore, and every byte is an ASCII alphanumeric
/// character or an underscore. An empty slice is not plausible: a NUL as the
/// very first byte terminates `cstr` immediately and gives no name at all.
fn is_plausible_identifier(bytes: &[u8]) -> bool {
    let Some(&first) = bytes.first() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    bytes
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Gives the address of each `EventDesc` one object's `PrivateObj` carries,
/// read from the pointer array at `lp_events_type_info`.
///
/// A module (`PrivateObj::Absent`) gives an empty list; there is no
/// `PrivateObj` to hold the array in the first place.
///
/// # Addresses only, never a decoder
///
/// `STRUCTURES.md` section 6.4 records that an `EventDesc` has the same
/// layout as a `FuncTypDesc`. Plan 02-04's `functyp.rs` writes that decoder,
/// so Phase 3 can call it on the addresses this function gives; carrying a
/// second copy of that decoder in this file would let the two drift apart,
/// and the two plans of this wave must not depend on each other. Neither
/// does this function recover an event's *name*: `STRUCTURES.md` records
/// that the name strings are present with no pointer to them, and
/// recovering one means scanning the run of strings after the last
/// procedure name and matching by position, a heuristic that belongs to
/// Phase 3. Gap 9 stays open and unreached here, per D-09.
///
/// # This corpus carries no event descriptor at all
///
/// A script run over all 44 vendored programs read `cnt_events` from every
/// one of the 97 objects that carry a `PrivateObj`, and found `0` in 97 of
/// 97. The other 8 objects are standard modules and have no `PrivateObj` at
/// all. So this function's loop is exercised only by the two synthetic
/// fixtures in this file's own test module; no corpus file reaches it.
///
/// # The pointer is checked before the loop
///
/// The same shape [`ProcedureList::read`] found real and common for
/// `Object.lpProcNamesArray`: a null array pointer with a non-zero count
/// dereferences address zero unless the pointer is resolved, and rejected,
/// before anything trusts the count as a loop bound. `cnt_events * 4` is
/// bounded with a checked multiply and the window is taken with
/// `Region::subregion`, so it can never exceed the real mapped length at
/// `lp_events_type_info`.
#[must_use]
pub fn event_descriptor_addresses(pe: &PeImage<'_>, private: &PrivateObj) -> Vec<Va> {
    let PrivateObj::Present {
        cnt_events,
        lp_events_type_info,
        ..
    } = private
    else {
        return Vec::new();
    };
    if lp_events_type_info.is_null() {
        return Vec::new();
    }
    let Some(array) = pe.region_at_va(*lp_events_type_info) else {
        return Vec::new();
    };
    let Some(window_size) = u32::from(*cnt_events).checked_mul(EVENT_DESC_PTR_SIZE) else {
        return Vec::new();
    };
    let Some(window) = array.subregion(Off::new(0), window_size) else {
        return Vec::new();
    };

    let mut addresses = Vec::with_capacity(usize::from(*cnt_events));
    for index in 0..u32::from(*cnt_events) {
        let Some(entry_off) = index.checked_mul(EVENT_DESC_PTR_SIZE) else {
            break;
        };
        let Some(va) = window.va_le(Off::new(entry_off)) else {
            break;
        };
        addresses.push(va);
    }
    addresses
}

/// Reads an unsigned 16-bit value out of a structure window.
fn u16_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<u16, Refusal> {
    window.u16_le(Off::new(at)).ok_or(Refusal::Damaged(what))
}

/// Reads an unsigned 32-bit value out of a structure window.
fn u32_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<u32, Refusal> {
    window.u32_le(Off::new(at)).ok_or(Refusal::Damaged(what))
}

/// Reads a virtual address out of a structure window.
fn va_at(window: &Region<'_>, at: u32, what: &'static str) -> Result<Va, Refusal> {
    window.va_le(Off::new(at)).ok_or(Refusal::Damaged(what))
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
        Gap, OBJECT_INFO_SIZE, ObjectInfo, PrivateObj, ProcNames, Procedure, ProcedureCounts,
        ProcedureList, event_descriptor_addresses,
    };
    use crate::error::Refusal;
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Va};
    use crate::vb::header::{VbHeader, header_region};
    use crate::vb::object::{Object, ObjectTable};
    use crate::vb::project::{ObjectTableHead, ProjectInfo};

    /// The program this task's tracer is worked against: three objects, none
    /// a module, none sharing the same `PrivateObj` shape as another.
    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    /// The program whose object count and object capacity differ, and whose
    /// two standard modules give the module sentinel. `[VERIFIED: local]`
    /// `Attribute VB_Name` inside each `.bas` names them the opposite of
    /// their file names: `Subs.bas` is `Attribute VB_Name = "Declaration_Module"`
    /// and `Declarations.bas` is `Attribute VB_Name = "Sub_Module"`.
    const MAP_EDITOR: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Map-editor-2D/Map Editor.exe"
    ));

    /// The program whose worked example this plan corrects: `frmFractal`'s
    /// nine `lpProcNamesArray` entries are not null. See the module doc
    /// comment.
    const MANDELBROT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Mandelbrot/Mandelbrot.exe"
    ));

    /// The program this plan's `cnt_public_vars` counter-examples are
    /// measured against, alongside `Grayscale.exe` above. `frmMain` and
    /// `Organism` are two of its objects.
    const ARTIFICIAL_LIFE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Artificial-life/Artificial Life.exe"
    ));

    /// `Artificial-life`'s `frmMain.frm`, read directly per `AGENTS.md`'s
    /// "build the state that a test needs inside the test": the declared
    /// count below is counted from these bytes, not copied from a document.
    const ARTIFICIAL_LIFE_FRM_MAIN_SRC: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Artificial-life/frmMain.frm"
    ));

    /// `Artificial-life`'s `Organism.cls`, read for the same reason.
    const ARTIFICIAL_LIFE_ORGANISM_SRC: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Artificial-life/Organism.cls"
    ));

    /// `Grayscale-effect`'s `pdOpenSaveDialog.cls`, read for the same
    /// reason.
    const GRAYSCALE_PD_OPEN_SAVE_DIALOG_SRC: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/pdOpenSaveDialog.cls"
    ));

    /// Counts source-level `Public variable As Type` declarations in a
    /// `.frm`/`.cls` source file, the way `CONTEXT.md`'s own measurement
    /// script counted them: a line beginning `Public `, whose next word is
    /// not a declaration keyword that is not a variable (`Sub`, `Function`,
    /// `Property`, `Enum`, `Type`, `Const`, `Event`, `Declare`), counts one
    /// variable per comma-separated name on that line, matching lines such
    /// as `Public oX As Long, oY As Long`.
    ///
    /// Read with `char::from(byte)`, per phase 1's Latin-1 rule, matching
    /// every other string read in this crate.
    fn count_declared_public_variables(source: &[u8]) -> usize {
        const NOT_A_VARIABLE: [&str; 8] = [
            "Sub", "Function", "Property", "Enum", "Type", "Const", "Event", "Declare",
        ];
        let text: String = source.iter().copied().map(char::from).collect();
        let mut count = 0_usize;
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("Public ") else {
                continue;
            };
            let keyword = rest.split_whitespace().next().unwrap_or("");
            if NOT_A_VARIABLE.contains(&keyword) {
                continue;
            }
            count += 1 + rest.matches(',').count();
        }
        count
    }

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
    /// chain `inspect` uses, and gives every recovered `Object`.
    ///
    /// This duplicates the walk `vb/object.rs`'s own tests carry, on
    /// purpose: the two modules must be able to fail independently, and a
    /// shared test helper would make one change break both.
    fn objects(data: &[u8]) -> Vec<Object> {
        let image = PeImage::parse(data).unwrap();
        let lp_object_table = object_table_va(data);
        let head = ObjectTableHead::read(&image, lp_object_table).unwrap();
        ObjectTable::walk(&image, lp_object_table, &head)
            .unwrap()
            .objects
    }

    /// `Grayscale.exe` declares one form and two classes. None is a module,
    /// so `ObjectInfo::read` resolves for all three, and each one's
    /// `lp_private_object` is a real address: neither `0` nor the module
    /// sentinel `0xFFFF_FFFF`.
    #[test]
    fn grayscale_object_info_resolves_for_all_three_objects_with_a_real_private_object() {
        let objs = objects(GRAYSCALE);
        assert_eq!(objs.len(), 3);
        let image = PeImage::parse(GRAYSCALE).unwrap();
        for (index, object) in objs.iter().enumerate() {
            let info = ObjectInfo::read(&image, object.lp_object_info).unwrap();
            assert_eq!(usize::from(info.w_object_index), index);
            assert_ne!(info.lp_private_object, 0);
            assert_ne!(info.lp_private_object, 0xFFFF_FFFF);
        }
    }

    /// `Map Editor.exe` declares two standard modules among its five objects.
    /// Each one's `ObjectInfo.lpPrivateObject` is the module sentinel, and
    /// reading it gives [`PrivateObj::Absent`] rather than a refusal: a
    /// refusal here would lose every module in the corpus.
    #[test]
    fn map_editor_module_objects_give_private_obj_absent_and_object_info_still_reads() {
        let objs = objects(MAP_EDITOR);
        assert_eq!(objs.len(), 5);
        let image = PeImage::parse(MAP_EDITOR).unwrap();

        let declaration_module = &objs[1];
        let sub_module = &objs[2];
        assert_eq!(declaration_module.name, "Declaration_Module");
        assert_eq!(declaration_module.proc_count, 1);
        assert_eq!(sub_module.name, "Sub_Module");
        assert_eq!(sub_module.proc_count, 7);

        for module in [declaration_module, sub_module] {
            let info = ObjectInfo::read(&image, module.lp_object_info).unwrap();
            assert_eq!(info.lp_private_object, 0xFFFF_FFFF);
            assert_eq!(
                PrivateObj::read(&image, info.lp_private_object).unwrap(),
                PrivateObj::Absent
            );
        }
    }

    /// A plain `0`, not only the sentinel `0xFFFF_FFFF`, also means the
    /// object carries no `PrivateObj`. No corpus program in this module
    /// exercises the plain-zero case, so this is a direct, synthetic check
    /// on the raw value alone: it needs no `PeImage` at all, because both
    /// sentinel checks happen before this function resolves an address.
    #[test]
    fn a_plain_zero_private_object_address_is_also_absent() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        assert_eq!(PrivateObj::read(&image, 0).unwrap(), PrivateObj::Absent);
    }

    /// `FastDrawing`, `Grayscale.exe`'s third object, gives a non-null
    /// `lp_func_type_info` and carries `cnt_public_vars` and `cnt_events`.
    #[test]
    fn grayscale_fast_drawing_private_obj_gives_non_null_func_type_info_and_carries_counts() {
        let objs = objects(GRAYSCALE);
        let fast_drawing = &objs[2];
        assert_eq!(fast_drawing.name, "FastDrawing");

        let image = PeImage::parse(GRAYSCALE).unwrap();
        let info = ObjectInfo::read(&image, fast_drawing.lp_object_info).unwrap();
        let private = PrivateObj::read(&image, info.lp_private_object).unwrap();
        match private {
            PrivateObj::Present {
                lp_func_type_info,
                cnt_public_vars,
                cnt_events,
                ..
            } => {
                assert!(!lp_func_type_info.is_null());
                // `cnt_public_vars` is carried and is not asserted against a
                // meaning: see the doc comment on `PrivateObj::Present`.
                let _ = cnt_public_vars;
                assert_eq!(cnt_events, 0);
            }
            PrivateObj::Absent => panic!("FastDrawing is a class, not a module"),
        }
    }

    /// `cnt_events` is `0` for every object in all three vendored programs
    /// this module reads. `RESEARCH.md` records the same finding across the
    /// whole 44-file corpus: no sample of a non-zero value exists anywhere.
    #[test]
    fn cnt_events_is_zero_for_every_object_in_the_three_vendored_programs() {
        for data in [GRAYSCALE, MAP_EDITOR, MANDELBROT] {
            let image = PeImage::parse(data).unwrap();
            for object in objects(data) {
                let info = ObjectInfo::read(&image, object.lp_object_info).unwrap();
                match PrivateObj::read(&image, info.lp_private_object).unwrap() {
                    PrivateObj::Present { cnt_events, .. } => assert_eq!(cnt_events, 0),
                    PrivateObj::Absent => {}
                }
            }
        }
    }

    /// A patched `lpObjectInfo` that resolves nowhere refuses the read.
    #[test]
    fn an_object_info_pointer_in_no_section_is_damaged() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let nowhere = Va::new(image.image_base() + 0x00F0_0000);
        assert!(image.region_at_va(nowhere).is_none());
        assert_eq!(
            ObjectInfo::read(&image, nowhere).unwrap_err(),
            Refusal::Damaged("the ObjectInfo pointer is in no section")
        );
    }

    /// A file cut off inside `ObjectInfo` is refused at the window, and is
    /// not read in part.
    #[test]
    fn a_file_truncated_inside_object_info_is_damaged_and_is_not_read_in_part() {
        let objs = objects(GRAYSCALE);
        let object = &objs[0];

        let image = PeImage::parse(GRAYSCALE).unwrap();
        let region = image.region_at_va(object.lp_object_info).unwrap();
        let start = usize::try_from(region.file_offset(Off::new(0)).unwrap().get()).unwrap();
        let cut = start + 0x10;
        assert!(cut < start + usize::try_from(OBJECT_INFO_SIZE).unwrap());
        assert!(cut < GRAYSCALE.len());

        let bytes = GRAYSCALE[..cut].to_vec();
        let truncated = PeImage::parse(&bytes).unwrap();
        assert_eq!(
            ObjectInfo::read(&truncated, object.lp_object_info).unwrap_err(),
            Refusal::Damaged("the file ends inside the ObjectInfo structure")
        );
    }

    /// `FastDrawing` gives eight procedure slots: four `Public`, at indices 4
    /// to 7, named `GetImageWidth`, `GetImageHeight`, `GetImageData2D` and
    /// `SetImageData2D`; the first four are `Private`, the four `Private
    /// Declare Function` lines its source carries.
    #[test]
    fn grayscale_fast_drawing_gives_eight_slots_four_public_four_private() {
        let objs = objects(GRAYSCALE);
        let fast_drawing = &objs[2];
        assert_eq!(fast_drawing.name, "FastDrawing");
        assert_eq!(fast_drawing.proc_count, 8);

        let image = PeImage::parse(GRAYSCALE).unwrap();
        let list = ProcedureList::read(&image, fast_drawing);
        assert!(list.defects().is_empty());
        assert_eq!(
            list.procs,
            ProcNames::Slots(vec![
                Procedure::Private,
                Procedure::Private,
                Procedure::Private,
                Procedure::Private,
                Procedure::Public("GetImageWidth".to_owned()),
                Procedure::Public("GetImageHeight".to_owned()),
                Procedure::Public("GetImageData2D".to_owned()),
                Procedure::Public("SetImageData2D".to_owned()),
            ])
        );
    }

    /// `pdOpenSaveDialog` gives six slots and every one is `Private`: its
    /// source declares four `Private Declare Function` lines and two
    /// `Friend Function` members, and only `Public` survives here.
    #[test]
    fn grayscale_pd_open_save_dialog_gives_six_slots_all_private() {
        let objs = objects(GRAYSCALE);
        let pd_open_save_dialog = &objs[1];
        assert_eq!(pd_open_save_dialog.name, "pdOpenSaveDialog");
        assert_eq!(pd_open_save_dialog.proc_count, 6);

        let image = PeImage::parse(GRAYSCALE).unwrap();
        let list = ProcedureList::read(&image, pd_open_save_dialog);
        assert!(list.defects().is_empty());
        assert_eq!(list.procs, ProcNames::Slots(vec![Procedure::Private; 6]));
    }

    /// The correction this plan carries. `CONTEXT.md` measured that
    /// `frmFractal`'s nine `lpProcNamesArray` entries are not null: every one
    /// is a non-null address that resolves to no section, a fragment of a
    /// build-machine path the compiler never overwrote. Every one of the
    /// nine gives `Procedure::Private`, each with its own defect, which is
    /// what makes these nine uninitialised and not simply null.
    #[test]
    fn mandelbrot_frm_fractal_gives_nine_slots_all_private_and_uninitialised() {
        let objs = objects(MANDELBROT);
        let frm_fractal = &objs[0];
        assert_eq!(frm_fractal.name, "frmFractal");
        assert_eq!(frm_fractal.proc_count, 9);
        assert!(!frm_fractal.lp_proc_names_array.is_null());

        let image = PeImage::parse(MANDELBROT).unwrap();
        let list = ProcedureList::read(&image, frm_fractal);
        assert_eq!(list.procs, ProcNames::Slots(vec![Procedure::Private; 9]));
        assert_eq!(
            list.defects().len(),
            9,
            "every one of the nine entries is non-null and resolves to no section, so every \
             one must leave a defect behind it: {:?}",
            list.defects()
        );
    }

    /// `Map Editor.exe`'s two standard modules give the absent-array state,
    /// carrying their own `proc_count`, and no slot list at all.
    #[test]
    fn map_editor_both_standard_modules_give_the_absent_array_state() {
        let objs = objects(MAP_EDITOR);
        let declaration_module = &objs[1];
        let sub_module = &objs[2];
        assert!(declaration_module.lp_proc_names_array.is_null());
        assert!(sub_module.lp_proc_names_array.is_null());

        let image = PeImage::parse(MAP_EDITOR).unwrap();
        assert_eq!(
            ProcedureList::read(&image, declaration_module).procs,
            ProcNames::NoNameArray { proc_count: 1 }
        );
        assert_eq!(
            ProcedureList::read(&image, sub_module).procs,
            ProcNames::NoNameArray { proc_count: 7 }
        );
    }

    /// The absent-array state and a list of nine `Private` slots are
    /// different values, and this compares them and finds them different:
    /// an empty-looking count is not the same fact as no array at all.
    #[test]
    fn the_absent_array_state_and_a_list_of_nine_private_slots_are_different_values() {
        let absent = ProcNames::NoNameArray { proc_count: 9 };
        let nine_private = ProcNames::Slots(vec![Procedure::Private; 9]);
        assert_ne!(absent, nine_private);
    }

    /// A synthetic object whose `lp_proc_names_array` is null and whose
    /// `proc_count` is 7 gives the absent-array state, not a list of seven
    /// private slots: the pointer is checked before the loop, so nothing
    /// ever calls `region_at_va` on the null address, let alone reads
    /// through it.
    #[test]
    fn a_synthetic_object_with_a_null_array_pointer_and_a_nonzero_count_gives_the_absent_state() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let object = Object {
            lp_object_info: Va::new(0x0040_1000),
            name: "SyntheticModule".to_owned(),
            proc_count: 7,
            lp_proc_names_array: Va::new(0),
            f_object_type: 0x0001_8001,
        };

        let list = ProcedureList::read(&image, &object);
        assert_eq!(list.procs, ProcNames::NoNameArray { proc_count: 7 });
        assert!(list.defects().is_empty());
    }

    /// Sums `ProcedureCounts` over every object of one program.
    fn program_totals(data: &[u8]) -> (u32, u32, u32) {
        let image = PeImage::parse(data).unwrap();
        let mut declared = 0_u32;
        let mut recovered = 0_u32;
        let mut capped = 0_u32;
        for object in objects(data) {
            let list = ProcedureList::read(&image, &object);
            let counts = ProcedureCounts::of(&list.procs);
            declared += counts.declared;
            recovered += counts.recovered;
            if counts.no_name_array {
                capped += counts.declared;
            }
        }
        (declared, recovered, capped)
    }

    /// The measured recovery, summed over every object of each program: this
    /// planner's own script over the vendored corpus, not a document.
    /// `Grayscale.exe` recovers 12 of 34 declared slots. `Mandelbrot.exe`
    /// recovers 0 of 9: every one of its nine slots is private, not merely
    /// null (see the correction above). `Map Editor.exe` recovers 0 of 36,
    /// and 8 of those 36 sit in the two standard modules that carry no name
    /// array at all: the cap is a number this test states, not a shortfall
    /// it silently absorbs.
    #[test]
    fn the_three_vendored_programs_recover_the_measured_number_of_names() {
        assert_eq!(program_totals(GRAYSCALE), (34, 12, 0));
        assert_eq!(program_totals(MANDELBROT), (9, 0, 0));
        assert_eq!(program_totals(MAP_EDITOR), (36, 0, 8));
    }

    /// `ProcedureCounts::of` gives the three numbers a report needs for one
    /// object: slots declared, names recovered, and whether the object
    /// carries a name array at all.
    #[test]
    fn procedure_counts_of_gives_the_three_numbers_for_one_object() {
        let slots = ProcedureCounts::of(&ProcNames::Slots(vec![
            Procedure::Private,
            Procedure::Public("GetImageWidth".to_owned()),
            Procedure::Public("GetImageHeight".to_owned()),
        ]));
        assert_eq!(
            slots,
            ProcedureCounts {
                declared: 3,
                recovered: 2,
                no_name_array: false,
            }
        );

        let absent = ProcedureCounts::of(&ProcNames::NoNameArray { proc_count: 7 });
        assert_eq!(
            absent,
            ProcedureCounts {
                declared: 7,
                recovered: 0,
                no_name_array: true,
            }
        );
    }

    /// Gives the `PrivateObj` one named object of one program carries.
    /// Panics if the object is a module (no `PrivateObj` to give) or if the
    /// name does not appear in the program at all, both of which would be a
    /// broken fixture rather than a measured result.
    fn private_obj_of(data: &[u8], name: &str) -> PrivateObj {
        let objs = objects(data);
        let object = objs
            .iter()
            .find(|object| object.name == name)
            .unwrap_or_else(|| panic!("no object named {name} in this fixture"));
        let image = PeImage::parse(data).unwrap();
        let info = ObjectInfo::read(&image, object.lp_object_info).unwrap();
        PrivateObj::read(&image, info.lp_private_object).unwrap()
    }

    /// `pdOpenSaveDialog.cls`, present in 15 of the 44 corpus programs,
    /// declares zero source-level `Public variable As Type` lines, only a
    /// `Public Enum`. Every corpus binary, `Grayscale.exe` included, reports
    /// `cnt_public_vars` of `4` for it regardless. Per D-14, this is the
    /// first of the three measured counter-examples that prove the field
    /// does not count what its name says.
    #[test]
    fn grayscale_pd_open_save_dialog_public_var_count_disagrees_with_its_source() {
        let declared = count_declared_public_variables(GRAYSCALE_PD_OPEN_SAVE_DIALOG_SRC);
        assert_eq!(
            declared, 0,
            "pdOpenSaveDialog.cls declares no source-level Public variable, only a Public Enum"
        );

        match private_obj_of(GRAYSCALE, "pdOpenSaveDialog") {
            PrivateObj::Present {
                cnt_public_vars, ..
            } => assert_eq!(cnt_public_vars, 4),
            PrivateObj::Absent => panic!("pdOpenSaveDialog is a class, not a module"),
        }
    }

    /// `frmMain.frm` in `Artificial-life` declares zero `Public` anything,
    /// while carrying 10 top-level named controls, and reports
    /// `cnt_public_vars` of `5`. The second measured counter-example.
    #[test]
    fn artificial_life_frm_main_public_var_count_disagrees_with_its_source() {
        let declared = count_declared_public_variables(ARTIFICIAL_LIFE_FRM_MAIN_SRC);
        assert_eq!(
            declared, 0,
            "frmMain.frm declares no source-level Public variable at all"
        );

        match private_obj_of(ARTIFICIAL_LIFE, "frmMain") {
            PrivateObj::Present {
                cnt_public_vars, ..
            } => assert_eq!(cnt_public_vars, 5),
            PrivateObj::Absent => panic!("frmMain is a form, not a module"),
        }
    }

    /// `Organism.cls` in `Artificial-life` declares 17 real public
    /// variables, some on comma-joined lines, and reports `cnt_public_vars`
    /// of `23`. The third measured counter-example: not merely absent from
    /// an over-count, but a genuine, non-zero declared count that still
    /// disagrees with the binary's number.
    #[test]
    fn artificial_life_organism_public_var_count_disagrees_with_its_source() {
        let declared = count_declared_public_variables(ARTIFICIAL_LIFE_ORGANISM_SRC);
        assert_eq!(
            declared, 17,
            "Organism.cls declares 17 real Public variables, some on comma-joined lines"
        );

        match private_obj_of(ARTIFICIAL_LIFE, "Organism") {
            PrivateObj::Present {
                cnt_public_vars, ..
            } => assert_eq!(cnt_public_vars, 23),
            PrivateObj::Absent => panic!("Organism is a class, not a module"),
        }
    }

    /// The covering test this plan's own deliberate breakage found missing:
    /// see this plan's SUMMARY for the breakage that produced no failure
    /// without this test. A non-zero `cnt_public_vars` produces exactly one
    /// [`Gap`], naming that count; a zero count, and the absent state, both
    /// produce no gap at all.
    #[test]
    fn the_gap_list_is_non_empty_exactly_when_the_public_var_count_is_non_zero() {
        let non_zero = PrivateObj::Present {
            cnt_public_vars: 4,
            cnt_events: 0,
            lp_func_type_info: Va::new(0),
            lp_events_type_info: Va::new(0),
            lp_public_vars: Va::new(0),
        };
        assert_eq!(non_zero.gaps(), vec![Gap::UnexplainedPublicVarCount(4)]);

        let zero = PrivateObj::Present {
            cnt_public_vars: 0,
            cnt_events: 0,
            lp_func_type_info: Va::new(0),
            lp_events_type_info: Va::new(0),
            lp_public_vars: Va::new(0),
        };
        assert!(zero.gaps().is_empty());

        assert!(PrivateObj::Absent.gaps().is_empty());
    }

    /// `event_descriptor_addresses` gives the empty list for every object of
    /// the three vendored programs this module reads, because every one of
    /// them reports an event count of `0`.
    #[test]
    fn event_descriptor_addresses_is_empty_for_every_object_of_the_three_vendored_programs() {
        for data in [GRAYSCALE, MAP_EDITOR, MANDELBROT] {
            let image = PeImage::parse(data).unwrap();
            for object in objects(data) {
                let info = ObjectInfo::read(&image, object.lp_object_info).unwrap();
                let private = PrivateObj::read(&image, info.lp_private_object).unwrap();
                assert_eq!(event_descriptor_addresses(&image, &private), Vec::new());
            }
        }
    }

    /// The instrument. `RESEARCH.md` and `CONTEXT.md` both record it: no
    /// corpus program carries a single event descriptor, 97 of 97 objects
    /// that hold a `PrivateObj`. This test is what watches for the day that
    /// changes: its failure message names the file and the object, because
    /// a program carrying an event descriptor has entered the corpus and
    /// the decode path this phase leaves unbuilt now needs a real sample.
    #[test]
    fn no_corpus_program_in_this_module_carries_an_event_descriptor() {
        for (label, data) in [
            ("Grayscale.exe", GRAYSCALE),
            ("Map Editor.exe", MAP_EDITOR),
            ("Mandelbrot.exe", MANDELBROT),
        ] {
            let image = PeImage::parse(data).unwrap();
            for object in objects(data) {
                let info = ObjectInfo::read(&image, object.lp_object_info).unwrap();
                let private = PrivateObj::read(&image, info.lp_private_object).unwrap();
                if let PrivateObj::Present { cnt_events, .. } = private {
                    assert_eq!(
                        cnt_events, 0,
                        "{label}'s {} reports a non-zero event count ({cnt_events}): a real \
                         event descriptor has entered the corpus, and the decode path plan \
                         02-05 left unbuilt now needs a test against a real sample",
                        object.name
                    );
                }
            }
        }
    }

    /// Builds a minimal 32 bit i386 portable executable with one section at
    /// RVA `0x1000` / file offset `0x400`, so [`event_descriptor_addresses`]
    /// has real mapped bytes to walk. This fixture tests the pointer walk
    /// itself, not the RVA-to-file-offset map: `vb/object.rs`'s own
    /// synthetic fixture already exercises the case where the two disagree,
    /// and duplicating that concern here would test the same fact twice
    /// under a different name.
    fn synthetic_image_with_event_pointers(pointers: &[u32]) -> Vec<u8> {
        const LFANEW: usize = 0x40;
        const OPTIONAL: usize = LFANEW + 24;
        const SECTION: usize = OPTIONAL + 224;

        let mut out = vec![0_u8; 0x600];
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

        // One section: RVA 0x1000, file offset 0x400, mapped length 0x100.
        out[SECTION..SECTION + 8].copy_from_slice(b".text\0\0\0");
        out[SECTION + 8..SECTION + 12].copy_from_slice(&0x100_u32.to_le_bytes());
        out[SECTION + 12..SECTION + 16].copy_from_slice(&0x1000_u32.to_le_bytes());
        out[SECTION + 16..SECTION + 20].copy_from_slice(&0x100_u32.to_le_bytes());
        out[SECTION + 20..SECTION + 24].copy_from_slice(&0x400_u32.to_le_bytes());
        out[SECTION + 36..SECTION + 40].copy_from_slice(&0x6000_0020_u32.to_le_bytes());

        for (index, pointer) in pointers.iter().enumerate() {
            let at = 0x400 + index * 4;
            out[at..at + 4].copy_from_slice(&pointer.to_le_bytes());
        }

        out
    }

    /// A synthetic object with an event count of 3 and a pointer array of
    /// three real addresses gives back exactly those three addresses: the
    /// walk is exercised by something, even though no corpus file reaches
    /// it.
    #[test]
    fn a_synthetic_object_with_three_event_pointers_gives_those_three_addresses() {
        let pointers = [0x0040_1000_u32, 0x0040_1004_u32, 0x0040_1008_u32];
        let bytes = synthetic_image_with_event_pointers(&pointers);
        let image = PeImage::parse(&bytes).unwrap();
        let private = PrivateObj::Present {
            cnt_public_vars: 0,
            cnt_events: 3,
            lp_func_type_info: Va::new(0),
            lp_events_type_info: Va::new(0x0040_1000),
            lp_public_vars: Va::new(0),
        };

        assert_eq!(
            event_descriptor_addresses(&image, &private),
            vec![
                Va::new(0x0040_1000),
                Va::new(0x0040_1004),
                Va::new(0x0040_1008)
            ]
        );
    }

    /// A synthetic object with an event count of 3 and a null pointer array
    /// gives an empty list. No `PeImage` fixture is built for this one: the
    /// null check happens before [`PeImage::region_at_va`] is ever called,
    /// so the `Grayscale.exe` image already parsed above is enough to prove
    /// no byte is read at address zero through any image at all.
    #[test]
    fn a_synthetic_object_with_a_null_event_pointer_array_gives_an_empty_list() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let private = PrivateObj::Present {
            cnt_public_vars: 0,
            cnt_events: 3,
            lp_func_type_info: Va::new(0),
            lp_events_type_info: Va::new(0),
            lp_public_vars: Va::new(0),
        };

        assert_eq!(event_descriptor_addresses(&image, &private), Vec::new());
    }
}
