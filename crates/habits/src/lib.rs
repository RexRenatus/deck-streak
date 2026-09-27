//! # deck-streak-habits
//!
//! What this context owns: Manual habits: the minutes-of-reading log, the daily writing
//! confirmation, the habit board and reading analytics.
//!
//! What it does not own: The daily pre-study readings, which are the readings context's.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
