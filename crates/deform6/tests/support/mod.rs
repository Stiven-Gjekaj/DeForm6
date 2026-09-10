//! The independent harness the differential comparison (plan 02-08) and
//! the recovery ratio pin (plan 02-09) both build on.
//!
//! `vbp` is the second, independent `.vbp` reader. `frm` is the second,
//! independent `.frm` reader. `rules` is the written exclusion rules
//! VER-04 requires. `source` is the second, independent reader for the
//! source an executable was built from. None of the four calls anything
//! in `src/`: a harness that shares a reader with the library it tests
//! would agree with a bug in that reader, and none of these four shares
//! anything.

pub mod frm;
pub mod rules;
pub mod source;
pub mod vbp;
