//! # deck-streak-privacy
//!
//! What this context owns: Data rights: export and erase over every context's data-rights port,
//! proved symmetric, and the retention purge.
//!
//! What it does not own: Any table: each context owns its tables and answers for them through
//! the port.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
