//! # deck-streak-discipline
//!
//! What this context owns: Opt-in commitment devices: committed windows, the doomscroll
//! tripwire rail, confession, streak wagers, commitment contracts, panic, pardon and re-arm,
//! hard mode, the Beeminder rung and the instant-loop free spin.
//!
//! What it does not own: Any imposed penalty: every device is opt-in, bounded by the economy's
//! loss cap and silenced in a lapse.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
