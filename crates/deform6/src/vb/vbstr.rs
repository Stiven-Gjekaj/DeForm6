//! The encoding-validating string reader: `VbStr`, `StrEncoding`.
//!
//! The cursor always advances by the declared length, never by however far
//! the string decode happened to read.
//!
//! Plan 03-05 fills this module. It serves FRM-03.
