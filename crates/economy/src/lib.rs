//! # deck-streak-economy
//!
//! What this context owns: The coin wallet, the only confiscable stake: mints, fines under the
//! daily loss cap and the zero floor, and the coin shop.
//!
//! What it does not own: No XP, streak, level, badge or CEFR state, which are never
//! confiscable.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
