//! # deck-streak-publishing
//!
//! What this context owns: The opt-in public achievement export: achieved aggregates only,
//! scrubbed of secrets, addresses, private words and every forward-looking date, with a
//! first-parent-root history check.
//!
//! What it does not own: Anything planned: the public page shows only what already happened.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
