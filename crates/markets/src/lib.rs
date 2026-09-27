//! # deck-streak-markets
//!
//! What this context owns: Self-prediction markets over the owner's own next day: pricing,
//! escrow, settlement and the Oracle ladder.
//!
//! What it does not own: No stake other than coins.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
