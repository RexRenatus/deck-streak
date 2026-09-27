//! # deck-streak-kernel
//!
//! What this context owns: The shared kernel: identifiers, the study day and its 04:00 rollover
//! calendar, the track (language or law), the verdict type, the error type, typed configuration
//! and credentials, the clock, and the `SQLite` repository base (WAL, foreign keys, busy timeout,
//! `BEGIN IMMEDIATE`) with the data-rights port every context implements.
//!
//! What it does not own: No domain rule of any context. A type enters here only when two
//! contexts that may not depend on each other both need it.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
