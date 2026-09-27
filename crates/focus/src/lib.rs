//! # deck-streak-focus
//!
//! What this context owns: The focus timer (server-authoritative end time, cycles) and
//! deep-work XP accounting and statistics.
//!
//! What it does not own: No notification policy.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
