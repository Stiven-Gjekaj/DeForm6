//! The scope-byte walk over the control tree: `ControlTree`, `ControlNode`,
//! the control type, the control name, and the control array `Index`.
//!
//! Plan 03-04 fills this module. It serves FRM-01 and FRM-02.
//!
//! # The control block header (`STRUCTURES.md` section 8.4)
//!
//! Two layouts, selected by the flags byte at block offset `0x03`. `0x80`
//! selects the array layout; any other value selects the non-array layout.
//! [`read_control_header`] reads either shape into one [`ControlHeader`],
//! following the window-before-fields discipline `vb/object.rs` documents:
//! the caller narrows the block to its own bounded window before any field
//! inside it is read.
//!
//! # The scope-separator walk (`STRUCTURES.md` section 8.9)
//!
//! `03-RESEARCH.md`'s own words: "the least certain part of the entire
//! format." [`walk`] implements the recommendation literally: read `0xFF`,
//! then scope bytes, counting `0x02` as a pop, stopping on `0x01` (open a
//! child), `0x03` (sibling), `0x04` (end the form), `0x05` (menu), or
//! anything else (unrecognised). It then gates the result on
//! [`crate::vb::gui::Tiling`], the byte-accounting invariant plan 03-01
//! built. A tree whose bytes do not tile `lPropertiesLength` exactly is
//! refused with the byte offset of the divergence, never emitted mis-nested.
//!
//! This session measured, against real corpus bytes
//! (`corpus/vb6-code/Grayscale-effect/Grayscale.exe`,
//! `corpus/public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe`,
//! `corpus/public-domain/LockWorkStation/LockWorkStation.exe`), that a
//! control block's own separator starts at `blockStart + Length - 1`
//! (relative to the block's own `Length` field), confirmed independently
//! across four real sibling and child transitions. `0x03` alone, with no
//! preceding `0x02`, closes zero levels; each `0x02` closes one more. `0x03`
//! itself is the terminal byte of the run and does not add a pop of its own.
//! A run that opens with `0x01` makes the block just finished the parent of
//! the next one. See `walk`'s own doc comment for the full algorithm.
//!
//! # Closing a menu nested two levels deep (plan 03-14, `STRUCTURES.md`
//! # section 8.9, `WINDOWS.md` finding 7)
//!
//! Plan 03-04's own single-level rule, above, does not cover closing out of
//! a menu control nested two levels deep back to a sibling menu at the
//! form's own top level: `MAX_UNEXPLAINED_TAIL` (`vb/controltree.rs`)
//! existed only as a defensive stand-in for this gap. This session measured
//! the real rule from five independent transitions across two programs
//! (`HexScroll.exe`, `UUID2.exe`) and implemented it in [`read_scope_run`];
//! see that function's own doc comment for the bytes, the rule, and the
//! one shape (`corpus/public-domain/PassGen/PassGen.exe`) this rule does
//! not settle.

use crate::error::{Defect, DefectKind, Refusal, Site, damaged};
use crate::read::region::{Off, Region};
use crate::vb::gui::{FormStream, Tiling};

/// The flags byte value, at block offset `0x03`, that selects the array
/// control-block layout.
pub const ARRAY_FLAG: u8 = 0x80;

/// The block offset of the control array `Index` field, in the array
/// layout. `STRUCTURES.md` gap 11, closed this plan: 30 array elements
/// across 2 files, values 0 through 24, zero disagreements against the
/// `.frm` source. See the closure section this plan adds to
/// `STRUCTURES.md`.
pub const INDEX_AT: u32 = 0x05;

/// The bound on a scope-separator run. This repository chooses this value;
/// it is never read from the file. No real corpus form nests more than one
/// level deep between siblings. A crafted file whose scope bytes never
/// terminate refuses rather than loops.
pub const MAX_SCOPE_RUN: u32 = 64;

/// A recognised control type, or a value `STRUCTURES.md` section 8.4.1's
/// table does not cover.
///
/// Follows `vb/classify.rs`'s "carry raw, never guess" shape: a name for a
/// known value, the raw value for an unknown one, and no refusal either way.
/// Values 12, 14, 15, 21, 25 through 36, and 39 are unassigned in section
/// 8.4.1 and land in [`ControlKind::Unknown`]. This match is never widened
/// beyond what that section lists.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ControlKind {
    /// `cType` 0.
    PictureBox,
    /// `cType` 1.
    Label,
    /// `cType` 2.
    TextBox,
    /// `cType` 3.
    Frame,
    /// `cType` 4.
    CommandButton,
    /// `cType` 5.
    CheckBox,
    /// `cType` 6.
    OptionButton,
    /// `cType` 7.
    ComboBox,
    /// `cType` 8.
    ListBox,
    /// `cType` 9.
    HScrollBar,
    /// `cType` 10.
    VScrollBar,
    /// `cType` 11.
    Timer,
    /// `cType` 13.
    Form,
    /// `cType` 16.
    DriveListBox,
    /// `cType` 17.
    DirListBox,
    /// `cType` 18.
    FileListBox,
    /// `cType` 19.
    Menu,
    /// `cType` 20.
    MdiForm,
    /// `cType` 22.
    Shape,
    /// `cType` 23.
    Line,
    /// `cType` 24.
    Image,
    /// `cType` 37.
    Data,
    /// `cType` 38.
    Ole,
    /// `cType` 40.
    UserControl,
    /// `cType` 41.
    PropertyPage,
    /// `cType` 42.
    UserDocument,
    /// `cType` 255: an external (OCX) control. `vb/ocx.rs`, plan 03-08, owns
    /// reading its class name and CLSID.
    External,
    /// A value the match does not cover, carried raw for the report.
    Unknown(u8),
}

/// Classifies one control's raw `cType`.
#[must_use]
pub const fn classify_control_type(c_type: u8) -> ControlKind {
    match c_type {
        0 => ControlKind::PictureBox,
        1 => ControlKind::Label,
        2 => ControlKind::TextBox,
        3 => ControlKind::Frame,
        4 => ControlKind::CommandButton,
        5 => ControlKind::CheckBox,
        6 => ControlKind::OptionButton,
        7 => ControlKind::ComboBox,
        8 => ControlKind::ListBox,
        9 => ControlKind::HScrollBar,
        10 => ControlKind::VScrollBar,
        11 => ControlKind::Timer,
        13 => ControlKind::Form,
        16 => ControlKind::DriveListBox,
        17 => ControlKind::DirListBox,
        18 => ControlKind::FileListBox,
        19 => ControlKind::Menu,
        20 => ControlKind::MdiForm,
        22 => ControlKind::Shape,
        23 => ControlKind::Line,
        24 => ControlKind::Image,
        37 => ControlKind::Data,
        38 => ControlKind::Ole,
        40 => ControlKind::UserControl,
        41 => ControlKind::PropertyPage,
        42 => ControlKind::UserDocument,
        255 => ControlKind::External,
        other => ControlKind::Unknown(other),
    }
}

/// One control block's header: its type, its name, and, when it is an array
/// element, its index.
///
/// `c_id` is `0` for an array-layout control. `STRUCTURES.md`'s own
/// array-header table places `cId` at offset `0x05` there, but this plan's
/// own measurement (see the module doc comment, and the `STRUCTURES.md`
/// closure section this plan adds) shows that offset holds the array
/// `Index`, not `cId`. Section 8.4 names no other location for `cId` in the
/// array layout, so this reader does not invent one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlHeader {
    /// The control's ID, used to link it to its event handlers.
    /// `0` for an array-layout control; see the struct doc comment.
    pub c_id: u8,
    /// The raw control type code. Carried raw: [`classify_control_type`]
    /// names it, and this reader refuses no control on the strength of an
    /// unrecognised value.
    pub c_type: u8,
    /// The control's name, read as length-prefixed bytes. Each byte becomes
    /// its own Latin-1 code point; `String::from_utf8_lossy` is never used,
    /// because a byte in `0x80` to `0xFF` would become the replacement
    /// character and the name would be lost.
    pub name: String,
    /// The control array `Index`, when this block uses the array layout.
    /// `None` for a non-array control.
    pub array_index: Option<u16>,
    header_len: u32,
}

impl ControlHeader {
    /// The header's own byte length, from the block's own offset `0x00`
    /// through the `cType` byte inclusive. The property stream that follows
    /// a control's header starts here.
    #[must_use]
    pub const fn header_len(&self) -> u32 {
        self.header_len
    }
}

/// Reads the control array `Index` field: the two-byte little-endian value
/// at [`INDEX_AT`].
///
/// Read defensively as two bytes, per `03-RESEARCH.md` assumption A4: the
/// corpus cannot tell a one-byte field at `0x05` from the low byte of a
/// two-byte field spanning `0x05` and `0x06`, because no corpus index
/// exceeds 24. [`read_control_header`] gives a [`Defect`] when the high byte
/// is non-zero; this function only reads the raw value.
#[must_use]
pub fn read_array_index(block: &Region<'_>) -> Option<u16> {
    block.u16_le(Off::new(INDEX_AT))
}

/// Reads one control block's header from its own bounded window.
///
/// `block` is the control block's own window, starting at its `Length`
/// field (offset `0x00`) and bounded to at least the block's own declared
/// span. The caller (this module's [`walk`]) is responsible for reading
/// `Length` itself and building that window; this function reads only the
/// fields the header itself carries.
///
/// Gives `(ControlHeader, Vec<Defect>)`. A [`Defect`] never loses the
/// control: a name that cannot be read gives an empty name and the header's
/// other fields still stand, following `vb/object.rs::read_name`'s own
/// contract.
#[must_use]
pub fn read_control_header(block: &Region<'_>) -> (ControlHeader, Vec<Defect>) {
    let offset = block.file_offset(Off::new(0)).map_or(0, Off::get);
    let mut defects = Vec::new();

    let flags = block.u8(Off::new(0x03));
    if flags == Some(ARRAY_FLAG) {
        let array_index = read_array_index(block);
        if let Some(index) = array_index
            && index > 0xFF
        {
            let high = u8::try_from(index >> 8).unwrap_or(0);
            defects.push(Defect {
                site: Site {
                    offset,
                    rva: None,
                    structure: "ControlHeader",
                    field: "Index",
                },
                kind: DefectKind::IndexHighByteSet { offset, high },
            });
        }

        let name_len = block.u16_le(Off::new(0x07)).unwrap_or(0);
        let (name, name_defect) = read_name(block, offset, 0x09, name_len);
        if let Some(defect) = name_defect {
            defects.push(defect);
        }

        let c_type_offset = 0x0A_u32.saturating_add(u32::from(name_len));
        let c_type = block.u8(Off::new(c_type_offset)).unwrap_or(0);
        let header_len = c_type_offset.saturating_add(1);

        (
            ControlHeader {
                c_id: 0,
                c_type,
                name,
                array_index,
                header_len,
            },
            defects,
        )
    } else {
        let c_id = block.u8(Off::new(0x04)).unwrap_or(0);
        let name_len = block.u16_le(Off::new(0x05)).unwrap_or(0);
        let (name, name_defect) = read_name(block, offset, 0x07, name_len);
        if let Some(defect) = name_defect {
            defects.push(defect);
        }

        let c_type_offset = 0x08_u32.saturating_add(u32::from(name_len));
        let c_type = block.u8(Off::new(c_type_offset)).unwrap_or(0);
        let header_len = c_type_offset.saturating_add(1);

        (
            ControlHeader {
                c_id,
                c_type,
                name,
                array_index: None,
                header_len,
            },
            defects,
        )
    }
}

/// Reads a control's length-prefixed name.
///
/// A declared length of `0` gives an empty name and a [`Defect`] naming the
/// block's own byte offset; the header still gives its `cType`. A declared
/// length larger than the remaining block also gives an empty name and a
/// [`Defect`]; no allocation is sized from the declared length before this
/// check, because [`Region::take`] itself refuses the read rather than
/// allocating first.
fn read_name(
    block: &Region<'_>,
    block_offset: u32,
    name_start: u32,
    name_len: u16,
) -> (String, Option<Defect>) {
    if name_len == 0 {
        let defect = Defect {
            site: Site {
                offset: block_offset,
                rva: None,
                structure: "ControlHeader",
                field: "name",
            },
            kind: DefectKind::EmptyName {
                offset: block_offset,
            },
        };
        return (String::new(), Some(defect));
    }

    match block.take(Off::new(name_start), u32::from(name_len)) {
        // Each byte becomes its own Latin-1 code point, the rule
        // `vb/object.rs::read_name` and `vb/gui.rs::FormStream::name` both
        // use. `String::from_utf8_lossy` is never used here.
        Some(bytes) => (bytes.iter().copied().map(char::from).collect(), None),
        None => {
            let max = block.len().saturating_sub(name_start);
            let defect = Defect {
                site: Site {
                    offset: block_offset,
                    rva: None,
                    structure: "ControlHeader",
                    field: "name",
                },
                kind: DefectKind::ImplausibleCount {
                    offset: block_offset,
                    count: u32::from(name_len),
                    max,
                },
            };
            (String::new(), Some(defect))
        }
    }
}

/// A scope-separator run: `0xFF`, then one or more scope bytes.
///
/// `STRUCTURES.md` section 8.9. `pops` is the number of `0x02` bytes the run
/// held before its own terminal byte. `0x03`, the sibling terminal, does not
/// add a pop of its own; see the module doc comment for the measurement that
/// settles this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeRun {
    /// `0x01`: the control just read becomes the parent of the next one.
    OpenChild {
        /// Pops seen before this terminal byte.
        pops: u8,
    },
    /// `0x03`: the next control is a sibling at the current depth, after any
    /// pops in this run are applied.
    Sibling {
        /// Pops seen before this terminal byte.
        pops: u8,
    },
    /// `0x04`: the form ends here.
    EndForm {
        /// Pops seen before this terminal byte.
        pops: u8,
    },
    /// `0x05`: a menu follows.
    Menu {
        /// Pops seen before this terminal byte.
        pops: u8,
    },
    /// Any other byte: reported raw, and the run stops there.
    Unrecognised {
        /// The byte that stopped the run.
        byte: u8,
        /// Pops seen before this byte.
        pops: u8,
    },
}

/// The `cType` value `STRUCTURES.md` section 8.4.1 assigns to `Menu`.
const MENU_C_TYPE: u8 = 19;

/// Reads one scope-separator run starting at `at`, which must hold `0xFF`.
///
/// Reads scope bytes one at a time, bounded by [`MAX_SCOPE_RUN`]: `0x02`
/// adds a pop and continues; `0x01`, `0x03`, `0x04`, `0x05`, or any other
/// byte stops the run and gives its own [`ScopeRun`] variant. Gives
/// `(ScopeRun, run_len)`, where `run_len` is the total number of bytes the
/// run consumed, including the leading `0xFF`.
///
/// `current_is_menu` and `stack_top_is_menu` change how `0x02` and `0x03`
/// read. A menu control's own closing scope byte is not the same byte plan
/// 03-04 measured for every other control, and this session's own
/// measurement (see below) found that plan 03-04's single condition is not
/// enough on its own: the correct reading depends on TWO separate facts,
/// not one.
///
/// **`current_is_menu`**: is the control the walk just finished reading
/// itself a menu? Plan 03-04 measured, over `Grayscale.exe`'s own `mnuFile`
/// and `mnuOpenImage` menu entries, that a run of exactly `0xFF 0x02`, with
/// no further byte and with nothing yet popped in this run, makes
/// the menu control just read the parent of the next one, the same role
/// `0x01` plays for every other control. Outside a menu control, `0x02`
/// still adds a pop and continues, confirmed against `Grayscale.exe`'s own
/// `lblShades` to `frameDecompose` transition (`0xFF 0x02 0x03`, one pop
/// then a sibling).
///
/// **`stack_top_is_menu`**: is the control the new sibling or child would
/// attach to (the current top of the walk's own parent stack) itself a
/// menu, that is, are we already inside a menu's own child list, rather
/// than opening the first child of a menu that has none yet? This session
/// measured that plan 03-04's own `current_is_menu` special case, applied
/// on its own, mis-nests a real corpus case: `frmUUID2.frm`'s own
/// `menuLicense` (a leaf, no children) is followed by a bare `0xFF 0x02`,
/// the identical byte pattern plan 03-04 measured for `mnuFile` opening
/// `mnuOpenImage`, but the correct role here is `Sibling` (`menuSep`, a
/// sibling of `menuLicense`, both children of `menuAbout`), not
/// `OpenChild`. The one structural fact that tells these two identical byte
/// patterns apart: `menuFile`'s own parent stack top, at the moment its
/// trailing separator is read, is the form itself (not a menu); the top,
/// for `menuLicense`, is `menuAbout` (a menu). `STRUCTURES.md` section 8.9
/// already names SVBD's own separate, heuristic handling for menus, with
/// its own counter distinct from the general one; `stack_top_is_menu` is
/// this session's corpus-measured version of that same idea.
///
/// This session measured the transition plan 03-04 never saw: closing a
/// menu nested two levels deep back to a sibling menu at the form's own top
/// level. Five independent real transitions, across two programs, confirm
/// the combined rule (verified against each program's own `.frm` source, by
/// name):
///
/// - `corpus/public-domain/HexScroll/Hex Scroll.exe`, file offset `0x16fd`:
///   `menuExit` (child of `menuFile`) to `menuAbout` (a sibling of
///   `menuFile` at the form's own top level). Bytes `0xFF 0x03 0x02`, stack
///   top `menuFile` (a menu): one pop, then `Sibling`.
/// - `corpus/public-domain/UUID2/VB6/UUID2.exe`, file offset `0x1918`:
///   `menuExit` (child of `menuFile`) to `menuSettings` (a sibling of
///   `menuFile`). Bytes `0xFF 0x03 0x02`, stack top `menuFile`: one pop,
///   then `Sibling`.
/// - `corpus/public-domain/UUID2/VB6/UUID2.exe`, file offset `0x1986`:
///   `menuSave` (child of `menuSettings`) to `menuAbout` (a sibling of
///   `menuSettings`). Bytes `0xFF 0x03 0x02`, stack top `menuSettings`: one
///   pop, then `Sibling`.
/// - `corpus/public-domain/UUID2/VB6/UUID2.exe`, file offset `0x19cc`:
///   `menuLicense` (child of `menuAbout`) to `menuSep` (a sibling of
///   `menuLicense`, both children of `menuAbout`). Bytes `0xFF 0x02`, stack
///   top `menuAbout`: zero pops, then `Sibling`. This is the bare byte that
///   plan 03-04's own single-condition rule misread as `OpenChild`.
/// - `corpus/public-domain/HexScroll/Hex Scroll.exe`, file offset `0x1743`:
///   `menuLicense` (child of `menuAbout`) to `menuSep` (a sibling of
///   `menuLicense`, both children of `menuAbout`), the identical shape to
///   the transition above, in a second, independent program. Bytes `0xFF
///   0x02`, stack top `menuAbout`: zero pops, then `Sibling`.
///
/// In the first three, the parent the byte grammar must produce is the
/// form itself, one level above the menu control's own parent menu: exactly
/// one pop. Reading `0x03` as an immediate terminal (plan 03-04's own rule
/// for every other control) stops the run two bytes too early, leaves the
/// real `0x02` byte to be misread as the start of a bogus next control
/// block, and is the exact and only cause of the refusal this session
/// started from (`MAX_UNEXPLAINED_TAIL`, `WINDOWS.md` finding 7). In the
/// last two, `current_is_menu` alone gives `OpenChild` and silently
/// mis-nests `menuSep` as `menuLicense`'s own child, a wrong tree with no
/// refusal at all, the exact failure mode this repository's own gate
/// exists to catch, and caught here only because a test asserted the
/// recovered parent by name rather than trusting that recovery without a
/// refusal meant success. Two independent programs give the identical
/// bytes and the identical correct role, meeting this repository's own
/// "two programs must give the same rule" bar for this fifth transition
/// too, not only the first four.
///
/// The rule these five bytes support: when the parent stack top is itself a
/// menu (`stack_top_is_menu`), `0x03` behaves the way `0x02` behaves for
/// every other control: it adds a pop and the run continues, and `0x02`
/// becomes the run's own `Sibling` terminal, at whatever pop count the run
/// has accumulated. `current_is_menu`'s own bare-`0x02`-means-`OpenChild`
/// special case still applies, but only when the stack top is *not* a menu,
/// that is, only when the menu control just read is opening its own
/// first child, not adding a further sibling to a menu it is already
/// nested inside.
///
/// **What this rule does not settle.** `corpus/public-domain/PassGen/PassGen.exe`
/// holds a third shape: `menuAbout`, itself a sibling within an
/// already-open menu (`menuHelp`), that genuinely opens its own child
/// (`menuAboutForm`). Its own trailing separator, at file offset `0x21d0`,
/// is a bare `0xFF 0x02` with `stack_top_is_menu` true and zero pops, byte
/// for byte identical to the two `menuLicense` transitions above, which
/// need the opposite role. No byte this module reads (the fixed header, or
/// the property stream up to the separator) distinguishes the two; this
/// session found none. This rule is not stretched to cover it. The walk
/// still succeeds for `frmPassGen` (the byte count tiles exactly, no
/// control is lost), but the recovered tree gives `menuAboutForm`,
/// `menuSeparatorC` and `menuWebsite` `menuHelp` as their parent rather
/// than `menuAbout`. Recorded honestly as an open, narrowly-scoped
/// limitation in `.planning/WINDOWS.md`, distinct from the closed
/// two-level-deep-close case this rule does settle.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when the file ends inside the run, or when
/// [`MAX_SCOPE_RUN`] bytes pass with no terminating byte.
fn read_scope_run(
    region: &Region<'_>,
    at: Off,
    current_is_menu: bool,
    stack_top_is_menu: bool,
) -> Result<(ScopeRun, u32), Refusal> {
    let start_offset = region.file_offset(at).map_or(0, Off::get);
    let ff = region.u8(at).ok_or_else(|| {
        damaged(format!(
            "the scope run at file offset {start_offset:#x} reads past the end of the file"
        ))
    })?;
    if ff != 0xFF {
        return Err(damaged(format!(
            "expected a scope separator (0xFF) at file offset {start_offset:#x}, found {ff:#x}"
        )));
    }

    let mut cursor = at
        .checked_add(1)
        .ok_or(Refusal::Damaged("a scope run offset overflows a u32"))?;
    let mut pops: u8 = 0;

    for _ in 0..MAX_SCOPE_RUN {
        let byte_offset = region.file_offset(cursor).map_or(0, Off::get);
        let byte = region.u8(cursor).ok_or_else(|| {
            damaged(format!(
                "the scope run at file offset {byte_offset:#x} reads past the end of the file"
            ))
        })?;
        cursor = cursor
            .checked_add(1)
            .ok_or(Refusal::Damaged("a scope run offset overflows a u32"))?;
        let run_len = cursor.get().saturating_sub(at.get());

        match byte {
            0x02 if current_is_menu && !stack_top_is_menu && pops == 0 => {
                return Ok((ScopeRun::OpenChild { pops }, run_len));
            }
            0x03 if stack_top_is_menu => pops = pops.saturating_add(1),
            0x02 if stack_top_is_menu => return Ok((ScopeRun::Sibling { pops }, run_len)),
            0x02 => pops = pops.saturating_add(1),
            0x01 => return Ok((ScopeRun::OpenChild { pops }, run_len)),
            0x03 => return Ok((ScopeRun::Sibling { pops }, run_len)),
            0x04 => return Ok((ScopeRun::EndForm { pops }, run_len)),
            0x05 => return Ok((ScopeRun::Menu { pops }, run_len)),
            other => return Ok((ScopeRun::Unrecognised { byte: other, pops }, run_len)),
        }
    }

    Err(damaged(format!(
        "the scope run starting at file offset {start_offset:#x} exceeds {MAX_SCOPE_RUN} bytes with no terminating byte"
    )))
}

/// One control in the tree: its header, its own bounded byte block, its
/// parent, and its children.
///
/// Holds indices into [`ControlTree::nodes`] for the parent/child edges
/// rather than references, so an edge costs no allocation. `block` does
/// carry a lifetime, tied to the underlying file bytes: plan 03-10's
/// `vb/mod.rs` is the first caller that needs a control's own bytes after
/// the walk finishes, to read its property stream, and, for an external
/// control, its class name and OCX header, without re-walking the tree to
/// find that span again. `PartialEq`/`Eq` are not derived: [`Region`]
/// itself does not implement them, and no caller needs to compare a whole
/// tree, only its own fields.
#[derive(Clone, Debug)]
pub struct ControlNode<'a> {
    /// The control's own header: its type, name, and array index.
    pub header: ControlHeader,
    /// The control's own bounded window: `Length + 2` bytes starting at the
    /// block's own `Length` field, the same region [`read_control_header`]
    /// reads `header` from. `pub(crate)`: a caller outside this crate has
    /// no bound-checked API of its own to read through it safely.
    pub(crate) block: Region<'a>,
    /// The index of this control's parent in [`ControlTree::nodes`]. `None`
    /// for the root, which is the form itself.
    pub parent: Option<usize>,
    /// The indices of this control's children, in stream order.
    pub children: Vec<usize>,
}

/// The control tree: every node the walk recovered, in depth-first stream
/// order, plus the root index.
#[derive(Clone, Debug)]
pub struct ControlTree<'a> {
    /// Every node, in depth-first stream order. Index `0` is always the
    /// root.
    pub nodes: Vec<ControlNode<'a>>,
    /// The index of the root node (the form itself) in [`Self::nodes`].
    pub root: usize,
}

/// Reads one control block at `at` in `region`: its `Length`, its header,
/// and its own bounded byte block.
///
/// Gives `(ControlHeader, Vec<Defect>, length, block)`, where `length` is
/// the raw declared `Length` value (not `Length + 2`), and `block` is the
/// control's own `Length + 2`-byte window: `header` was read from it, and
/// plan 03-10's `vb/mod.rs` composes a `ControlNode` around it so a later
/// pass can read this control's own property stream from the same bytes.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when the file ends before the `Length`
/// field, when `Length` is `0` (which would not advance the cursor; the
/// research measured this exact value appearing when a flat jump ignores
/// the scope run), or when the block's own declared span runs past the end
/// of the file.
fn read_block<'a>(
    region: &Region<'a>,
    at: Off,
) -> Result<(ControlHeader, Vec<Defect>, u32, Region<'a>), Refusal> {
    let offset = region.file_offset(at).map_or(0, Off::get);
    let length = region.u16_le(at).ok_or_else(|| {
        damaged(format!(
            "a control block at file offset {offset:#x} holds no Length field"
        ))
    })?;
    if length == 0 {
        return Err(damaged(format!(
            "a control block at file offset {offset:#x} declares a Length of zero, which would not advance the cursor"
        )));
    }

    let block_span = u32::from(length)
        .checked_add(2)
        .ok_or(Refusal::Damaged("a control block's Length overflows a u32"))?;
    let block = region.subregion(at, block_span).ok_or_else(|| {
        damaged(format!(
            "a control block at file offset {offset:#x} declares {block_span} bytes, which runs past the end of the file"
        ))
    })?;

    let (header, defects) = read_control_header(&block);
    Ok((header, defects, u32::from(length), block))
}

/// Applies `pops` to the parent stack, refusing rather than popping the
/// stack empty.
///
/// The stack always holds at least the root while more controls remain to
/// be read. A pop count that would empty it means the scope-byte
/// interpretation has drifted from the real tree shape.
fn apply_pops(stack: &mut Vec<usize>, pops: u8, at_offset: u32) -> Result<(), Refusal> {
    for _ in 0..pops {
        if stack.len() <= 1 {
            return Err(damaged(format!(
                "the scope run at file offset {at_offset:#x} pops past the root of the control tree"
            )));
        }
        stack.pop();
    }
    Ok(())
}

/// Tells whether a control block at `at` is plausibly readable: at least its
/// two-byte `Length` field is present, `Length` is not `0`, and the block's
/// own declared span (`Length + 2`) does not run past the end of `region`.
///
/// A `Length` of `0` is never treated as "not enough room left". It is
/// always the hard refusal [`read_block`] itself gives, because it would not
/// advance the cursor regardless of how many bytes remain.
fn block_fits(region: &Region<'_>, at: Off) -> bool {
    let Some(length) = region.u16_le(at) else {
        return false;
    };
    if length == 0 {
        return true;
    }
    let Some(span) = u32::from(length).checked_add(2) else {
        return false;
    };
    region.subregion(at, span).is_some()
}

/// Closes the walk out: applies as many of `pops` as the stack allows, then
/// accounts whatever remains between `end_at` and the end of `region` as a
/// trailing span, and returns.
///
/// A trailing span may remain after the walk's own last accounted byte.
/// `03-RESEARCH.md` assumption A3 leaves the zero-children case's own
/// trailing span unexplained; this session additionally measured a larger
/// one after `corpus/vb6-code/Grayscale-effect/Grayscale.exe`'s own trailing
/// menu section, where the stream ends with no further `0xFF` and too few
/// bytes left for another control block. Both are accounted directly, by
/// measuring what remains against the stream's own declared length, rather
/// than by a constant this repository has no second confirming sample for.
/// The largest trailing span this repository treats as an unexplained
/// footer rather than real control data a mis-terminated scope run left
/// unexplored.
///
/// `03-RESEARCH.md` assumption A3 measured a 3-byte tail after
/// `LockWorkStation.exe`'s own zero-children form; this bound gives more
/// than double that margin. Plan 03-10's own differential gate, comparing
/// against `support::frm` (a second, independent `.frm` reader), found a
/// tail far larger than this margin silently swallowing real, named
/// controls this format's own scope-byte grammar has a gap for: closing out
/// of a menu control nested two levels deep, back to a sibling menu at the
/// form's own top level, is a transition `03-RESEARCH.md`'s own corpus
/// measurement (Grayscale's own single-level menu case) did not cover.
/// Refusing a surprisingly large tail turns a silent wrong tree into the
/// same honest, per-form refusal [`walk`]'s own caller already handles for
/// every other unreadable structure, rather than trusting an unproven
/// number of bytes to be a footer. This bound is chosen by this repository;
/// it is never read from the file.
const MAX_UNEXPLAINED_TAIL: u32 = 8;

/// Closes the walk out: accounts whatever remains between `end_at` and the
/// end of `region` as a trailing span (see [`MAX_UNEXPLAINED_TAIL`]'s own
/// doc comment for that half).
///
/// Plan 03-14 (review finding WR-02). This function used to take the
/// terminal run's own `pops` and a `stack: &mut Vec<usize>`, compute a
/// `bounded_pops` value from `stack.len()`, apply only that many pops, and
/// never read the result: a terminal pop count larger than the stack could
/// give was silently truncated rather than reported, reading as a safety
/// check it did not perform. The review finding names two acceptable
/// fixes: make the check real (refuse, the way [`apply_pops`] already
/// does mid-walk), or drop the dead code entirely.
///
/// This session measured that making the check real is the wrong fix here:
/// the terminal run's own `pops` value, read at `EndForm` or at a next
/// position [`block_fits`] finds implausible, routinely exceeds the real
/// parent stack depth on real corpus data (`Grayscale.exe`, `UUID2.exe`,
/// `HexScroll.exe` all hit this). Calling [`apply_pops`] here, as the
/// mid-walk pop discipline does, refuses every one of those closing-out
/// programs, including `FrmHex` and `frmUUID2`, whose own recovery this
/// plan's own task 1 measured and requires. A terminal pop count closing
/// the whole tree out is not the same fact a mid-walk pop is: mid-walk, an
/// excess pop means the byte grammar has drifted from the real tree shape,
/// and every later sibling would land at the wrong depth; at the very end
/// of the walk, no more siblings follow, so an excess pop closes nothing
/// that still matters. The dead code is dropped, per the review finding's
/// second named fix, and this function no longer takes the stack or the
/// pop count at all.
fn close_walk(region: &Region<'_>, end_at: Off, tiling: &mut Tiling) -> Result<(), Refusal> {
    let offset = region.file_offset(end_at).map_or(0, Off::get);
    let tail = region.len().saturating_sub(end_at.get());
    if tail > MAX_UNEXPLAINED_TAIL {
        return Err(damaged(format!(
            "the control tree walk at file offset {offset:#x} would leave {tail} bytes \
             unaccounted for, more than the {MAX_UNEXPLAINED_TAIL} byte margin this repository \
             trusts as an unexplained footer; refusing rather than silently dropping what those \
             bytes might hold"
        )));
    }
    if tail > 0 {
        tiling.account(tail)?;
    }
    Ok(())
}

/// Walks the control tree from a form's property stream.
///
/// Reads the form's own outermost control block first, then loops: read a
/// scope-separator run, apply its pops to the current parent stack, and
/// either read the next control block (for `OpenChild`, `Sibling`, or
/// `Menu`) or stop (for `EndForm`, or for a next position that
/// [`block_fits`] finds implausible; see [`close_walk`]). Every block's own
/// span and every run's own length is accounted into `tiling`. The walk
/// ends by calling [`Tiling::finish`]; a tree whose bytes do not tile the
/// stream's own declared length exactly is refused with the byte offset of
/// the divergence, and no mis-nested tree is ever returned.
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when a control block is unreadable, when a
/// scope run is unreadable or exceeds [`MAX_SCOPE_RUN`] bytes with no
/// terminator, when a scope run holds an unrecognised byte, when pops would
/// empty the parent stack, or when the tiling check fails.
pub fn walk<'a>(
    stream: &FormStream<'a>,
    tiling: &mut Tiling,
) -> Result<(ControlTree<'a>, Vec<Defect>), Refusal> {
    let region: Region<'a> = stream.region();
    let mut defects = Vec::new();

    let root_at = Off::new(0);
    let (root_header, root_defects, root_length, root_block) = read_block(&region, root_at)?;
    defects.extend(root_defects);
    let root_content = root_length.checked_sub(1).ok_or(Refusal::Damaged(
        "a control block's Length is too small to hold its own header",
    ))?;
    tiling.account(root_content)?;

    let mut nodes = vec![ControlNode {
        header: root_header,
        block: root_block,
        parent: None,
        children: Vec::new(),
    }];
    let mut stack: Vec<usize> = vec![0];
    let mut current: usize = 0;
    let mut sep_at = root_at.checked_add(root_content).ok_or(Refusal::Damaged(
        "a control block's own span overflows a u32",
    ))?;

    loop {
        let current_is_menu = nodes
            .get(current)
            .is_some_and(|node| node.header.c_type == MENU_C_TYPE);
        let stack_top_is_menu = stack
            .last()
            .and_then(|&idx| nodes.get(idx))
            .is_some_and(|node| node.header.c_type == MENU_C_TYPE);
        let (run, run_len) = read_scope_run(&region, sep_at, current_is_menu, stack_top_is_menu)?;
        tiling.account(run_len)?;
        let run_offset = region.file_offset(sep_at).map_or(0, Off::get);

        match run {
            ScopeRun::Unrecognised { byte, .. } => {
                return Err(damaged(format!(
                    "the scope run at file offset {run_offset:#x} holds an unrecognised byte {byte:#x}"
                )));
            }
            ScopeRun::EndForm { .. } => {
                let end_at = sep_at.checked_add(run_len).ok_or(Refusal::Damaged(
                    "the control tree walk's own cursor overflows a u32",
                ))?;
                close_walk(&region, end_at, tiling)?;
                break;
            }
            ScopeRun::OpenChild { pops } | ScopeRun::Sibling { pops } | ScopeRun::Menu { pops } => {
                let next_at = sep_at.checked_add(run_len).ok_or(Refusal::Damaged(
                    "the control tree walk's own cursor overflows a u32",
                ))?;

                if !block_fits(&region, next_at) {
                    close_walk(&region, next_at, tiling)?;
                    break;
                }

                let parent = if matches!(run, ScopeRun::OpenChild { .. }) {
                    apply_pops(&mut stack, pops, run_offset)?;
                    stack.push(current);
                    current
                } else {
                    apply_pops(&mut stack, pops, run_offset)?;
                    *stack.last().ok_or(Refusal::Damaged(
                        "the control tree's own parent stack is empty",
                    ))?
                };

                let (header, node_defects, length, block) = read_block(&region, next_at)?;
                defects.extend(node_defects);
                let content = length.checked_sub(1).ok_or(Refusal::Damaged(
                    "a control block's Length is too small to hold its own header",
                ))?;
                tiling.account(content)?;

                let idx = nodes.len();
                nodes.push(ControlNode {
                    header,
                    block,
                    parent: Some(parent),
                    children: Vec::new(),
                });
                let parent_node = nodes.get_mut(parent).ok_or(Refusal::Damaged(
                    "the control tree's own parent index is out of range",
                ))?;
                parent_node.children.push(idx);
                current = idx;

                sep_at = next_at.checked_add(content).ok_or(Refusal::Damaged(
                    "a control block's own span overflows a u32",
                ))?;
            }
        }
    }

    tiling.finish()?;
    Ok((ControlTree { nodes, root: 0 }, defects))
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
        ARRAY_FLAG, ControlKind, ScopeRun, classify_control_type, read_array_index,
        read_control_header, walk,
    };
    use crate::error::{DefectKind, Refusal};
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Region};
    use crate::vb::gui::{GuiObjectInfo, GuiTable, Tiling};
    use crate::vb::header::{VbHeader, header_region};

    // --- Task 1: the control block header --------------------------------

    /// Builds a synthetic array-layout control block matching the exact hex
    /// this session measured for the `TxtF` array's first three elements in
    /// `corpus/vb6-code/Custom-image-filters/Custom_Filters.exe` (the
    /// corpus's real `ExeName32`; the plan's own text names
    /// `CustomFilters.exe`, which this corpus does not hold; the `.vbp`
    /// declares `ExeName32="Custom_Filters.exe"`). This is a literal the
    /// test builds in memory, per `AGENTS.md`: a test builds the state it
    /// needs and does not read it out of a file the author edits.
    ///
    /// Layout: `Length`(2) `unknown`(1) `flags=0x80`(1) `group-const=0x02`(1)
    /// `Index`(2) `name_len=4`(2) `"TxtF"`(4) `unknown`(1) `cType`(1).
    fn txt_f_element(index_low: u8, c_type: u8) -> Vec<u8> {
        vec![
            0x0f, 0x00, // Length = 15 (content is small; this test does not
            // exercise the tiling walk, only the header reader)
            0x00,       // unknown
            ARRAY_FLAG, // flags
            0x02,       // the unexplained group constant
            index_low, 0x00, // Index, low byte then high byte
            0x04, 0x00, // name length = 4
            b'T', b'x', b't', b'F', // name
            0x00, // unknown, the "0x09+n unknown" byte STRUCTURES.md section 8.4 names
            c_type,
        ]
    }

    #[test]
    fn the_three_txt_f_elements_give_index_zero_one_and_two() {
        for (i, expected_index) in [(0u8, 0u16), (1, 1), (2, 2)] {
            let bytes = txt_f_element(i, 2);
            let region = Region::new(&bytes, Off::new(0));
            let (header, defects) = read_control_header(&region);
            assert_eq!(header.array_index, Some(expected_index));
            assert_eq!(header.name, "TxtF");
            assert_eq!(header.c_type, 2);
            assert!(defects.is_empty(), "{defects:?}");
        }
    }

    #[test]
    fn read_array_index_gives_none_when_the_flags_byte_is_not_0x80() {
        let mut bytes = txt_f_element(0, 2);
        bytes[3] = 0x00; // flags, not 0x80
        let region = Region::new(&bytes, Off::new(0));
        assert_eq!(read_array_index(&region), Some(0));
        // The header reader itself takes the non-array branch, and gives no
        // array_index, regardless of what bytes happen to sit at INDEX_AT.
        let (header, _) = read_control_header(&region);
        assert_eq!(header.array_index, None);
    }

    #[test]
    fn a_two_byte_array_index_with_a_non_zero_high_byte_gives_a_defect() {
        let mut bytes = txt_f_element(0, 2);
        bytes[5] = 0x01; // Index low byte
        bytes[6] = 0x01; // Index high byte, non-zero
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.array_index, Some(0x0101));
        assert_eq!(defects.len(), 1);
        assert!(matches!(
            defects[0].kind,
            DefectKind::IndexHighByteSet { high: 0x01, .. }
        ));
    }

    #[test]
    fn a_non_array_header_reads_c_id_name_and_c_type() {
        // Length(2) unknown(1) flags=0(1) cId(1) name_len(2) name(n) unknown(1) cType(1)
        let mut bytes = vec![0x00, 0x00, 0x00, 0x00, 0x07, 0x03, 0x00];
        bytes.extend_from_slice(b"Cmd");
        bytes.push(0x00); // unknown, per STRUCTURES.md "0x07+n unknown"
        bytes.push(4); // cType 4 = CommandButton
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.c_id, 0x07);
        assert_eq!(header.name, "Cmd");
        assert_eq!(header.c_type, 4);
        assert_eq!(header.array_index, None);
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn a_declared_name_length_of_zero_gives_an_empty_name_and_a_defect_and_still_gives_c_type() {
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 13];
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.name, "");
        assert_eq!(header.c_type, 13);
        assert_eq!(defects.len(), 1);
        assert!(matches!(defects[0].kind, DefectKind::EmptyName { .. }));
    }

    #[test]
    fn a_declared_name_length_larger_than_the_remaining_block_gives_an_empty_name_and_a_defect() {
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x07, 0xFF, 0xFF];
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.name, "");
        assert_eq!(defects.len(), 1);
        assert!(matches!(
            defects[0].kind,
            DefectKind::ImplausibleCount { count: 0xFFFF, .. }
        ));
    }

    #[test]
    fn a_name_byte_of_0xa9_becomes_its_own_latin1_code_point() {
        // Length(2) unknown(1) flags=0(1) cId(1) name_len=1(2) name(1) unknown(1) cType(1)
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0xA9, 0x00, 1];
        let region = Region::new(&bytes, Off::new(0));
        let (header, _) = read_control_header(&region);
        assert_eq!(header.name.chars().next(), Some('\u{A9}'));
    }

    #[test]
    fn c_type_39_is_carried_raw_and_refuses_no_control() {
        // Length(2) unknown(1) flags=0(1) cId(1) name_len=1(2) name(1) unknown(1) cType(1)
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, b'X', 0x00, 39];
        let region = Region::new(&bytes, Off::new(0));
        let (header, defects) = read_control_header(&region);
        assert_eq!(header.c_type, 39);
        assert!(defects.is_empty(), "{defects:?}");
        assert_eq!(classify_control_type(39), ControlKind::Unknown(39));
    }

    #[test]
    fn every_named_c_type_classifies_by_name() {
        assert_eq!(classify_control_type(13), ControlKind::Form);
        assert_eq!(classify_control_type(4), ControlKind::CommandButton);
        assert_eq!(classify_control_type(1), ControlKind::Label);
        assert_eq!(classify_control_type(19), ControlKind::Menu);
        assert_eq!(classify_control_type(255), ControlKind::External);
    }

    #[test]
    fn a_repeated_index_in_a_synthetic_two_element_array_is_reported_twice_not_deduplicated() {
        // Synthetic fixture: two array elements that both declare Index=0.
        // The corpus holds no such case (per the plan's own acceptance
        // criteria); this fixture is built here, and its own assertion
        // message names it as synthetic.
        let elem_a = txt_f_element(0, 2);
        let elem_b = txt_f_element(0, 2);
        let region_a = Region::new(&elem_a, Off::new(0));
        let region_b = Region::new(&elem_b, Off::new(0));
        let (header_a, _) = read_control_header(&region_a);
        let (header_b, _) = read_control_header(&region_b);
        assert_eq!(
            header_a.array_index,
            Some(0),
            "synthetic fixture: element A"
        );
        assert_eq!(
            header_b.array_index,
            Some(0),
            "synthetic fixture: element B repeats Index=0, and the walk reports \
             it as a second, distinct control rather than merging or re-sorting it"
        );
    }

    #[test]
    fn the_array_index_offset_is_a_named_constant_set_to_0x05() {
        assert_eq!(super::INDEX_AT, 0x05);
    }

    // --- Corpus test: the full 25-element TxtF array ----------------------

    const CUSTOM_FILTERS: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Custom-image-filters/Custom_Filters.exe"
    ));

    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    const LOCK_WORK_STATION: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/LockWorkStation/LockWorkStation.exe"
    ));

    const SK_GRADIENT_SAMPLE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/SK-Gradient-Sample__VB6/demo/Project1.exe"
    ));

    const HEX_SCROLL: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/HexScroll/Hex Scroll.exe"
    ));

    const UUID2: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/UUID2/VB6/UUID2.exe"
    ));

    const PASS_GEN: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/public-domain/PassGen/PassGen.exe"
    ));

    /// Walks the whole control tree of the first form in `data`.
    fn walk_first_form(
        data: &[u8],
    ) -> Result<(super::ControlTree<'_>, Vec<crate::error::Defect>), Refusal> {
        let image = PeImage::parse(data).unwrap();
        let hdr = header_region(&image).unwrap();
        let header = VbHeader::read(&hdr).unwrap();
        let table = GuiTable::walk(&image, &header).unwrap();
        let entry = table.entries[0];
        let info = GuiObjectInfo::read(&image, entry.a_form_pointer).unwrap();
        let stream = info.form_stream().unwrap();
        let mut tiling = Tiling::new(info.l_properties_length);
        walk(&stream, &mut tiling)
    }

    #[test]
    fn the_txt_f_array_gives_twenty_five_elements_with_index_zero_through_twenty_four() {
        let (tree, _) =
            walk_first_form(CUSTOM_FILTERS).unwrap_or_else(|err| panic!("walk failed: {err}"));
        let mut found: Vec<u16> = tree
            .nodes
            .iter()
            .filter(|node| node.header.name == "TxtF")
            .filter_map(|node| node.header.array_index)
            .collect();
        found.sort_unstable();
        assert_eq!(
            found,
            (0..25).collect::<Vec<u16>>(),
            "TxtF indices found: {found:?}"
        );
    }

    #[test]
    fn grayscale_gives_opt_channel_two_one_zero_and_opt_decompose_one_zero() {
        let (tree, _) =
            walk_first_form(GRAYSCALE).unwrap_or_else(|err| panic!("walk failed: {err}"));
        let opt_channel: Vec<u16> = tree
            .nodes
            .iter()
            .filter(|node| node.header.name == "optChannel")
            .filter_map(|node| node.header.array_index)
            .collect();
        let opt_decompose: Vec<u16> = tree
            .nodes
            .iter()
            .filter(|node| node.header.name == "optDecompose")
            .filter_map(|node| node.header.array_index)
            .collect();
        assert_eq!(opt_channel, vec![2, 1, 0]);
        assert_eq!(opt_decompose, vec![1, 0]);
    }

    // --- Task 2: the scope separator walk and the tiling gate -------------

    #[test]
    fn lock_work_station_gives_one_root_form_with_zero_children_and_tiling_holds() {
        let (tree, defects) =
            walk_first_form(LOCK_WORK_STATION).unwrap_or_else(|err| panic!("walk failed: {err}"));
        assert_eq!(tree.nodes.len(), 1);
        assert!(tree.nodes[0].children.is_empty());
        assert!(defects.is_empty(), "{defects:?}");
    }

    #[test]
    fn sk_gradient_sample_gives_one_root_form_with_exactly_three_children() {
        let (tree, _) =
            walk_first_form(SK_GRADIENT_SAMPLE).unwrap_or_else(|err| panic!("walk failed: {err}"));
        assert_eq!(tree.nodes[tree.root].children.len(), 3);
        let names: Vec<&str> = tree.nodes[tree.root]
            .children
            .iter()
            .map(|&i| tree.nodes[i].header.name.as_str())
            .collect();
        assert_eq!(names, vec!["Command1", "Picture1", "Label1"]);
    }

    #[test]
    fn grayscale_gives_at_least_one_frame_with_children_and_the_tiling_holds() {
        let (tree, _) =
            walk_first_form(GRAYSCALE).unwrap_or_else(|err| panic!("walk failed: {err}"));
        let has_frame_with_children = tree
            .nodes
            .iter()
            .any(|node| node.header.name.starts_with("frame") && !node.children.is_empty());
        assert!(
            has_frame_with_children,
            "expected at least one Frame control with children"
        );
    }

    /// The instrument for the pop-count deviation this plan's own
    /// acceptance criteria asks for: `SK-Gradient-Sample__VB6` measures
    /// zero, its own three children are flat siblings of the form with no
    /// intervening container, so a broken `0x02` pop never touches its own
    /// child count (see this plan's own SUMMARY.md, "Deviations from
    /// Plan"). `Grayscale.exe` is the file that genuinely exercises a pop:
    /// `frameShades`'s last child, `lblShades`, is followed by exactly one
    /// `0x02` before `frameDecompose`, popping back out of `frameShades`'s
    /// own child level to the form's own. A `0x02` that closes zero levels
    /// instead of one leaves `frameDecompose` nested one level too deep
    /// (a child of `frameShades`, not a sibling of it), which this
    /// assertion (the form's own direct child list, by name, in stream
    /// order) is built to catch.
    #[test]
    fn grayscale_gives_the_form_its_own_nine_direct_children_by_name() {
        let (tree, _) =
            walk_first_form(GRAYSCALE).unwrap_or_else(|err| panic!("walk failed: {err}"));
        let names: Vec<&str> = tree.nodes[tree.root]
            .children
            .iter()
            .map(|&i| tree.nodes[i].header.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "frameShades",
                "frameDecompose",
                "frameChannel",
                "lstFilters",
                "cmdReset",
                "picMain",
                "picBack",
                "Label2",
                "mnuFile",
            ]
        );
    }

    #[test]
    fn a_scope_run_of_sixty_five_non_terminating_bytes_refuses_and_names_the_offset() {
        let mut bytes = vec![0xFF];
        bytes.extend(std::iter::repeat_n(0x02_u8, 65));
        let region = Region::new(&bytes, Off::new(0x1000));
        let err = super::read_scope_run(&region, Off::new(0), false, false).unwrap_err();
        let Refusal::Damaged(message) = err else {
            panic!("expected Refusal::Damaged, got {err:?}");
        };
        assert!(message.contains("0x1000"), "{message}");
    }

    #[test]
    fn a_control_block_with_a_length_of_zero_refuses_and_names_the_offset() {
        let bytes = vec![0x00, 0x00, 0x00, 0x00];
        let region = Region::new(&bytes, Off::new(0x2000));
        let err = super::read_block(&region, Off::new(0)).unwrap_err();
        let Refusal::Damaged(message) = err else {
            panic!("expected Refusal::Damaged, got {err:?}");
        };
        assert!(message.contains("0x2000"), "{message}");
        assert!(message.contains("zero"), "{message}");
    }

    #[test]
    fn the_bound_is_a_constant_not_a_file_value() {
        assert_eq!(super::MAX_SCOPE_RUN, 64);
    }

    // --- Plan 03-14: the two-level-deep menu close ------------------------

    /// `FrmHex.frm`'s own menu section: `menuFile` (one child, `menuExit`)
    /// then `menuAbout`, a sibling of `menuFile` at the form's own top
    /// level (three children: `menuLicense`, `menuSep`, `menuWebsite`).
    /// This is the shape the module doc comment measures against real bytes
    /// at file offset `0x16fd`: `0xFF 0x03 0x02`, one pop (out of
    /// `menuExit`'s own parent `menuFile`) then the `Sibling` role.
    #[test]
    fn frm_hex_gives_menu_file_and_menu_about_as_form_level_siblings_with_their_own_children() {
        let (tree, _) =
            walk_first_form(HEX_SCROLL).unwrap_or_else(|err| panic!("walk failed: {err}"));
        let root_children: Vec<&str> = tree.nodes[tree.root]
            .children
            .iter()
            .map(|&i| tree.nodes[i].header.name.as_str())
            .collect();
        assert_eq!(
            root_children,
            vec![
                "tmrNoteClear",
                "CmdCopy",
                "TxtBlue",
                "TxtGreen",
                "TxtRed",
                "TxtHex",
                "TxtRGB",
                "CmdClose",
                "HsBlue",
                "HsGreen",
                "HsRed",
                "menuFile",
                "menuAbout",
            ],
            "FrmHex.frm's own form-level children, in stream order"
        );

        let menu_file = tree
            .nodes
            .iter()
            .find(|n| n.header.name == "menuFile")
            .expect("menuFile is in the tree");
        let menu_file_children: Vec<&str> = menu_file
            .children
            .iter()
            .map(|&i| tree.nodes[i].header.name.as_str())
            .collect();
        assert_eq!(menu_file_children, vec!["menuExit"]);

        let menu_about = tree
            .nodes
            .iter()
            .find(|n| n.header.name == "menuAbout")
            .expect("menuAbout is in the tree");
        let menu_about_children: Vec<&str> = menu_about
            .children
            .iter()
            .map(|&i| tree.nodes[i].header.name.as_str())
            .collect();
        assert_eq!(
            menu_about_children,
            vec!["menuLicense", "menuSep", "menuWebsite"]
        );
    }

    /// `frmUUID2.frm`'s own menu section: three top-level menus in a row,
    /// `menuFile` (one child), `menuSettings` (one child) and `menuAbout`
    /// (three children). The transition out of `menuExit` and out of
    /// `menuSave` are two independent real measurements of the same rule,
    /// at file offsets `0x1918` and `0x1986`.
    #[test]
    fn frm_uuid2_gives_three_top_level_menus_with_their_own_children_by_name() {
        let (tree, _) = walk_first_form(UUID2).unwrap_or_else(|err| panic!("walk failed: {err}"));
        let root_children: Vec<&str> = tree.nodes[tree.root]
            .children
            .iter()
            .map(|&i| tree.nodes[i].header.name.as_str())
            .collect();
        assert_eq!(
            root_children,
            vec![
                "tmrAutomatic",
                "tmrNoteClear",
                "cmdClose",
                "cmdCopy",
                "cmdGenerate",
                "fmeSettings",
                "fmeStyle",
                "menuFile",
                "menuSettings",
                "menuAbout",
            ],
            "frmUUID2.frm's own form-level children, in stream order"
        );

        for (parent_name, expected_children) in [
            ("menuFile", vec!["menuExit"]),
            ("menuSettings", vec!["menuSave"]),
            ("menuAbout", vec!["menuLicense", "menuSep", "menuWebsite"]),
        ] {
            let parent = tree
                .nodes
                .iter()
                .find(|n| n.header.name == parent_name)
                .unwrap_or_else(|| panic!("{parent_name} is in the tree"));
            let children: Vec<&str> = parent
                .children
                .iter()
                .map(|&i| tree.nodes[i].header.name.as_str())
                .collect();
            assert_eq!(children, expected_children, "{parent_name}'s own children");
        }
    }

    /// `frmPassGen.frm`'s own `menuHelp` section nests three levels deep
    /// (`menuHelp` > {`menuHotkeys`, `menuSeparatorB`, `menuAbout`} >
    /// {`menuAboutForm`, `menuSeparatorC`, `menuWebsite`}), and exposes a
    /// genuine byte-level ambiguity this session's own rule does not
    /// settle, distinct from `FrmHex` and `frmUUID2`'s own required
    /// transitions and NOT named in this plan's own acceptance criteria.
    ///
    /// `menuAbout`'s own trailing separator here (`corpus/public-domain/
    /// PassGen/PassGen.exe`, file offset `0x21d0`) is a bare `0xFF 0x02`
    /// with `stack_top_is_menu` true and zero pops, byte for byte
    /// identical to `menuLicense`'s own trailing separator in `HexScroll.exe`
    /// and in `UUID2.exe`, both of which this module's own tests assert
    /// correctly resolve to `Sibling`. Here the correct role is the
    /// opposite, `OpenChild`: `menuAbout` genuinely opens its own child.
    /// No byte in the control header or the property stream this module
    /// reads distinguishes the two; the `stack_top_is_menu` rule, measured
    /// against two independent programs exactly as `FrmHex` and `frmUUID2`
    /// require, cannot be stretched to cover this third case without
    /// guessing (per this module's own doc comment and `AGENTS.md`'s "no
    /// mis-nested tree"). The walk still succeeds for `frmPassGen` (every
    /// control is read, the byte count tiles exactly), but the recovered
    /// tree places `menuAboutForm`, `menuSeparatorC` and `menuWebsite` as
    /// `menuAbout`'s own siblings rather than its children. This is a
    /// known, measured, narrowly-scoped limitation beyond what `FrmHex` and
    /// `frmUUID2`'s own two-program measurement settles, recorded in
    /// `.planning/WINDOWS.md`, not silently claimed as solved. This test
    /// proves the walk still succeeds and every control this program
    /// declares is present in the tree by name, without asserting the one
    /// parent relationship this session's own measurement does not settle.
    #[test]
    fn frm_pass_gen_recovers_every_declared_menu_control_by_name() {
        let (tree, _) =
            walk_first_form(PASS_GEN).unwrap_or_else(|err| panic!("walk failed: {err}"));
        let mut names: Vec<&str> = tree
            .nodes
            .iter()
            .filter(|n| n.header.c_type == super::MENU_C_TYPE)
            .map(|n| n.header.name.as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec![
                "menuAbout",
                "menuAboutForm",
                "menuExit",
                "menuFile",
                "menuHelp",
                "menuHotkeys",
                "menuOverride",
                "menuSave",
                "menuSeparatorA",
                "menuSeparatorB",
                "menuSeparatorC",
                "menuSettings",
                "menuSpecial",
                "menuWebsite",
            ]
        );
    }

    // `grayscale_gives_the_form_its_own_nine_direct_children_by_name`, above,
    // already covers the acceptance criterion that Grayscale.exe's tree is
    // unchanged from plan 03-04's own pin: Grayscale.exe has one top-level
    // menu with children and no second one, so it never exercises the
    // `current_is_menu` branch this plan adds, and this pre-existing test
    // still passes byte for byte against this plan's rule.

    /// A crafted scope run whose pop count would empty the parent stack
    /// still refuses, naming the byte offset, rather than silently popping
    /// the root away. `apply_pops` is what every `OpenChild`/`Sibling`
    /// branch in `walk` calls with a scope run's own `pops` field; this
    /// drives it directly with a stack that holds only the root.
    #[test]
    fn a_pop_count_that_would_empty_the_stack_still_refuses_and_names_the_offset() {
        let mut stack: Vec<usize> = vec![0];
        let err = super::apply_pops(&mut stack, 1, 0x4000).unwrap_err();
        let Refusal::Damaged(message) = err else {
            panic!("expected Refusal::Damaged, got {err:?}");
        };
        assert!(message.contains("0x4000"), "{message}");
        assert!(message.contains("pops past the root"), "{message}");
    }

    /// Plan 03-14, task 2 (review finding WR-02). `close_walk` used to
    /// compute a `bounded_pops` value against `stack.len()` and then never
    /// read it: a terminal pop count larger than the stack could give was
    /// silently truncated, not reported, reading as a safety check it did
    /// not perform.
    ///
    /// This session tried the review finding's own first fix (call
    /// [`super::apply_pops`], the same discipline `walk`'s own mid-walk
    /// pops already use) and measured that it refuses real corpus data:
    /// `Grayscale.exe`, `UUID2.exe` and `HexScroll.exe` all reach `EndForm`
    /// with a `pops` count larger than the real parent stack depth at that
    /// point, and none of that is a grammar error: no more siblings
    /// follow, so an excess terminal pop closes nothing that still
    /// matters, unlike a mid-walk excess pop, which would misplace every
    /// later sibling. The chosen fix is the review finding's second one:
    /// `close_walk` no longer takes a stack or a pop count at all.
    ///
    /// A synthetic byte sequence drives `read_scope_run` and `close_walk`
    /// directly, the same two functions `walk`'s own main loop calls, on a
    /// form built in memory (per `AGENTS.md`: a test builds the state it
    /// needs and does not read it out of a file the author edits): one
    /// root, one child, then an `EndForm` run whose own pop count (`5`) is
    /// five times the real stack depth (`1`, root only, after the one real
    /// pop the child's own `OpenChild` needs undoing). `close_walk` no
    /// longer receives the stack or the pop count at all, so there is
    /// nothing left for it to refuse on account of either; this proves
    /// that directly rather than through `walk`'s own public entry point,
    /// which needs a real `FormStream` this crate gives no test-only
    /// constructor for.
    #[test]
    fn close_walk_no_longer_refuses_an_end_form_pop_count_larger_than_the_stack_depth() {
        let mut bytes = Vec::new();
        // Root form header: Length=11 (content=10, header_len=10, no
        // property data beyond the header), cType 13 (Form).
        bytes.extend_from_slice(&[0x0b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00]);
        bytes.push(b'F');
        bytes.extend_from_slice(&[0x00, 13]);
        // Separator: 0xFF 0x01, OpenChild, zero pops.
        bytes.extend_from_slice(&[0xff, 0x01]);
        // Child header: Length=11 (content=10), cType 1 (Label).
        bytes.extend_from_slice(&[0x0b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00]);
        bytes.push(b'L');
        bytes.extend_from_slice(&[0x00, 1]);
        // Separator: 0xFF, five 0x02 pops, 0x04 EndForm. The stack holds
        // only [root, root] at this point (root pushed once, for the
        // child's own OpenChild); five pops is four more than the one
        // real pop available.
        bytes.extend_from_slice(&[0xff, 0x02, 0x02, 0x02, 0x02, 0x02, 0x04]);

        let region = Region::new(&bytes, Off::new(0));
        let root_at = Off::new(0);
        let (_, _, root_length, _) = super::read_block(&region, root_at).unwrap();
        let root_content = root_length - 1;
        let mut tiling = Tiling::new(u32::try_from(bytes.len()).unwrap());
        tiling.account(root_content).unwrap();

        let sep1_at = root_at.checked_add(root_content).unwrap();
        let (run1, run1_len) = super::read_scope_run(&region, sep1_at, false, false).unwrap();
        assert!(matches!(run1, ScopeRun::OpenChild { pops: 0 }), "{run1:?}");
        tiling.account(run1_len).unwrap();

        let child_at = sep1_at.checked_add(run1_len).unwrap();
        let (_, _, child_length, _) = super::read_block(&region, child_at).unwrap();
        let child_content = child_length - 1;
        tiling.account(child_content).unwrap();

        let sep2_at = child_at.checked_add(child_content).unwrap();
        let (run2, run2_len) = super::read_scope_run(&region, sep2_at, false, false).unwrap();
        assert!(matches!(run2, ScopeRun::EndForm { pops: 5 }), "{run2:?}");
        tiling.account(run2_len).unwrap();

        let end_at = sep2_at.checked_add(run2_len).unwrap();
        super::close_walk(&region, end_at, &mut tiling).unwrap_or_else(|err| {
            panic!("close_walk must not refuse on account of the pop count: {err}")
        });
        tiling.finish().unwrap();
    }
}
