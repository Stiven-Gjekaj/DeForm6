#![forbid(unsafe_code)]

//! DeForm6 reads a compiled Visual Basic 6 executable.
//!
//! This crate holds the reading side of the tool. It recovers metadata only.
//! It does not recover statements.
//!
//! Every input is untrusted. No function in this crate panics on any input.
//!
//! The modules below are declared here as a set, so that two plans in one
//! wave never edit this file.

pub mod error;
pub mod journal;
pub mod read;
pub mod vb;

pub use error::Refusal;
pub use vb::{Report, inspect};
