//! # deck-streak-readings
//!
//! What this context owns: The flagship daily pre-study readings: the day set from the
//! scheduler's new cards, one reading per topic, generation through the agent, the coverage and
//! safety gates, the read tap, the studied measure, rollover and archive, on-demand
//! regeneration, the one comeback reading per lapse, reading XP, and lane health.
//!
//! What it does not own: The minutes-of-reading habit (`/read`), which is the habits context's.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
