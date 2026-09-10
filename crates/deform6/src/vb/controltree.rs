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
//! format." [`walk`] implements the recommendation literally — read `0xFF`,
//! then scope bytes, counting `0x02` as a pop, stopping on `0x01` (open a
//! child), `0x03` (sibling), `0x04` (end the form), `0x05` (menu), or
//! anything else (unrecognised) — and gates the result on
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

use crate::error::{Defect, DefectKind, Refusal, Site};
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

/// Builds a [`Refusal::Damaged`] whose message is computed at runtime.
///
/// The same escape hatch `vb/gui.rs::damaged` documents: `Refusal::Damaged`
/// takes `&'static str`, and this module's own required refusals (an
/// unreadable control block, a scope run with no terminator) must name a
/// byte offset a hostile file put the bad value at. Every path that reaches
/// this function is already fatal to the whole tree.
fn damaged(message: String) -> Refusal {
    let leaked: &'static str = Box::leak(message.into_boxed_str());
    Refusal::Damaged(leaked)
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
/// `current_is_menu` changes how `0x02` reads. This session measured, over
/// `corpus/vb6-code/Grayscale-effect/Grayscale.exe`'s own `mnuFile` and
/// `mnuOpenImage` menu entries, that a run of exactly `0xFF 0x02` — with no
/// further byte — makes the menu control just read the parent of the next
/// one, the same role `0x01` plays for every other control. Every other
/// control in this session's corpus uses `0x01` for that role and never
/// shows a bare `0x02`; only a menu control shows it, and `STRUCTURES.md`
/// section 8.9 already names SVBD's own separate, heuristic handling for
/// menus. Outside a menu control, `0x02` still adds a pop and continues,
/// confirmed against `corpus/vb6-code/Grayscale-effect/Grayscale.exe`'s own
/// `lblShades` to `frameDecompose` transition (`0xFF 0x02 0x03`, one pop
/// then a sibling).
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when the file ends inside the run, or when
/// [`MAX_SCOPE_RUN`] bytes pass with no terminating byte.
fn read_scope_run(
    region: &Region<'_>,
    at: Off,
    current_is_menu: bool,
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
            0x02 if current_is_menu => return Ok((ScopeRun::OpenChild { pops }, run_len)),
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

/// One control in the tree: its header, its parent, and its children.
///
/// Holds indices into [`ControlTree::nodes`] rather than references, so the
/// tree needs no lifetime and no allocation per edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlNode {
    /// The control's own header: its type, name, and array index.
    pub header: ControlHeader,
    /// The index of this control's parent in [`ControlTree::nodes`]. `None`
    /// for the root, which is the form itself.
    pub parent: Option<usize>,
    /// The indices of this control's children, in stream order.
    pub children: Vec<usize>,
}

/// The control tree: every node the walk recovered, in depth-first stream
/// order, plus the root index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlTree {
    /// Every node, in depth-first stream order. Index `0` is always the
    /// root.
    pub nodes: Vec<ControlNode>,
    /// The index of the root node (the form itself) in [`Self::nodes`].
    pub root: usize,
}

/// Reads one control block at `at` in `region`: its `Length`, its header,
/// and its own byte span.
///
/// Gives `(ControlHeader, Vec<Defect>, length)`, where `length` is the raw
/// declared `Length` value (not `Length + 2`).
///
/// # Errors
///
/// Returns [`Refusal::Damaged`] when the file ends before the `Length`
/// field, when `Length` is `0` (which would not advance the cursor — the
/// research measured this exact value appearing when a flat jump ignores
/// the scope run), or when the block's own declared span runs past the end
/// of the file.
fn read_block(region: &Region<'_>, at: Off) -> Result<(ControlHeader, Vec<Defect>, u32), Refusal> {
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
    Ok((header, defects, u32::from(length)))
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
/// A `Length` of `0` is never treated as "not enough room left" — it is
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
fn close_walk(
    region: &Region<'_>,
    stack: &mut Vec<usize>,
    pops: u8,
    end_at: Off,
    tiling: &mut Tiling,
) -> Result<(), Refusal> {
    let bounded_pops = u8::try_from(stack.len().saturating_sub(1))
        .unwrap_or(pops)
        .min(pops);
    for _ in 0..bounded_pops {
        stack.pop();
    }
    let tail = region.len().saturating_sub(end_at.get());
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
/// [`block_fits`] finds implausible — see [`close_walk`]). Every block's own
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
pub fn walk(
    stream: &FormStream<'_>,
    tiling: &mut Tiling,
) -> Result<(ControlTree, Vec<Defect>), Refusal> {
    let region = stream.region();
    let mut defects = Vec::new();

    let root_at = Off::new(0);
    let (root_header, root_defects, root_length) = read_block(region, root_at)?;
    defects.extend(root_defects);
    let root_content = root_length.checked_sub(1).ok_or(Refusal::Damaged(
        "a control block's Length is too small to hold its own header",
    ))?;
    tiling.account(root_content)?;

    let mut nodes = vec![ControlNode {
        header: root_header,
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
        let (run, run_len) = read_scope_run(region, sep_at, current_is_menu)?;
        tiling.account(run_len)?;
        let run_offset = region.file_offset(sep_at).map_or(0, Off::get);

        match run {
            ScopeRun::Unrecognised { byte, .. } => {
                return Err(damaged(format!(
                    "the scope run at file offset {run_offset:#x} holds an unrecognised byte {byte:#x}"
                )));
            }
            ScopeRun::EndForm { pops } => {
                let end_at = sep_at.checked_add(run_len).ok_or(Refusal::Damaged(
                    "the control tree walk's own cursor overflows a u32",
                ))?;
                close_walk(region, &mut stack, pops, end_at, tiling)?;
                break;
            }
            ScopeRun::OpenChild { pops } | ScopeRun::Sibling { pops } | ScopeRun::Menu { pops } => {
                let next_at = sep_at.checked_add(run_len).ok_or(Refusal::Damaged(
                    "the control tree walk's own cursor overflows a u32",
                ))?;

                if !block_fits(region, next_at) {
                    close_walk(region, &mut stack, pops, next_at, tiling)?;
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

                let (header, node_defects, length) = read_block(region, next_at)?;
                defects.extend(node_defects);
                let content = length.checked_sub(1).ok_or(Refusal::Damaged(
                    "a control block's Length is too small to hold its own header",
                ))?;
                tiling.account(content)?;

                let idx = nodes.len();
                nodes.push(ControlNode {
                    header,
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
        ARRAY_FLAG, ControlKind, classify_control_type, read_array_index, read_control_header, walk,
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
    /// `CustomFilters.exe`, which this corpus does not hold — the `.vbp`
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

    /// Walks the whole control tree of the first form in `data`.
    fn walk_first_form(
        data: &[u8],
    ) -> Result<(super::ControlTree, Vec<crate::error::Defect>), Refusal> {
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
    /// assertion — the form's own direct child list, by name, in stream
    /// order — is built to catch.
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
        let err = super::read_scope_run(&region, Off::new(0), false).unwrap_err();
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
}
