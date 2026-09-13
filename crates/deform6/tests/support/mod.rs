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
//!
//! `hostile` is a fifth module, added by phase 5, and it is not part of
//! the differential harness the paragraph above describes. It builds a
//! hostile Visual Basic 6 shaped image from literals this file owns
//! outright, so the image may be committed as a regression fixture
//! without breaching the rule `AGENTS.md` states on a fixture calculated
//! from a third party file. Every byte `hostile` produces is written in
//! that module; none is read out of a file on disk.

pub mod frm;
pub mod hostile;
pub mod rules;
pub mod source;
pub mod vbp;
