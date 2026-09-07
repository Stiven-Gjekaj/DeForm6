//! The independent harness the differential comparison (plan 02-08) and
//! the recovery ratio pin (plan 02-09) both build on.
//!
//! `vbp` is the second, independent `.vbp` reader. `rules` is the written
//! exclusion rules VER-04 requires. Neither calls anything in `src/`: a
//! harness that shares a reader with the library it tests would agree with
//! a bug in that reader, and this one shares nothing.

pub mod rules;
pub mod vbp;
