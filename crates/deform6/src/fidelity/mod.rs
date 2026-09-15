//! Byte fidelity: what this reader understands of a structure, measured
//! against the bytes the compiler that made the file wrote.
//!
//! Every structure this crate parses can be written back over the bytes it
//! was read from and compared against them. A byte that comes back different
//! is a field this reader gets wrong. A byte that is never written is a field
//! this reader does not reproduce. Neither is a matter of opinion once it is
//! measured, and the gap register states both in prose today.
//!
//! # The emitters do not live beside the readers, on purpose
//!
//! Every `impl Emit` in this module restates its field offsets from
//! `docs/STRUCTURES.md`. None of them copies an offset from the reader it is
//! graded against.
//!
//! `tests/support/mod.rs` already states the rule this follows: a harness
//! that shares a reader with the library it tests agrees with a bug in that
//! reader, and the failure is invisible. An emitter written next to its
//! reader, from the same constants, would report a clean diff for a field
//! both sides place at the wrong offset. Two independent statements of the
//! layout are what make a clean diff mean anything at all.
//!
//! This is also why nothing under `vb/` or `read/` changes when a structure
//! joins this module.
//!
//! # A verbatim field cannot differ
//!
//! A field read at offset X and written back at offset X, with no arithmetic
//! between, can never disagree with itself. Most fields are verbatim, so most
//! of this measurement reports coverage rather than correctness, and a clean
//! map must never be read as "this structure is proven correct".
//!
//! Two things it does prove. The coverage figure is real: the bytes no field
//! claims are counted and pinned. And the two statements of the layout agree,
//! which is a genuine check on both.
//!
//! The fields worth watching are the ones that are **not** verbatim.
//! `crate::vb::object::Object::proc_count` is the first: the reader clamps it
//! to what the file can hold, so it differs in exactly the programs where the
//! clamp fires.
//!
//! # `Fault` is this repository's mistake, never the file's
//!
//! A [`crate::error::Defect`] is what the parser found in the user's file. A
//! [`Fault`] is what this module's own emitter did wrong. They must not share
//! a type: [`crate::error::Refusal::Damaged`] renders as "this Visual Basic 6
//! executable is damaged", and printing that sentence because a `put` in this
//! crate wrote the same byte twice would be a lie about someone's file.

pub mod slate;

pub use slate::Slate;

/// What this module's own emitter did wrong.
///
/// Never a statement about the file under test. See the module doc comment.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum Fault {
    /// The structure declares more bytes than a slate holds.
    #[error("{structure} declares {len} bytes, which is more than a slate holds")]
    Oversize {
        /// The structure that declared the length.
        structure: &'static str,
        /// The length it declared.
        len: u32,
    },
    /// The window given for the structure is shorter than the structure.
    #[error("the window for {structure} holds {have} bytes and the structure is {want}")]
    Short {
        /// The structure being graded.
        structure: &'static str,
        /// The length the structure declares.
        want: u32,
        /// The length the window holds.
        have: u32,
    },
    /// The window has no absolute file offset, so nothing can be placed.
    #[error("the window for {structure} has no absolute file offset")]
    Unplaced {
        /// The structure being graded.
        structure: &'static str,
    },
    /// A write inside `emit` was refused. The emitter is wrong, not the file.
    #[error("the emitter for {structure} refused a write at structure offset {at:#x}")]
    Refused {
        /// The structure whose emitter refused.
        structure: &'static str,
        /// The structure relative offset of the first refused write.
        at: u32,
    },
}
