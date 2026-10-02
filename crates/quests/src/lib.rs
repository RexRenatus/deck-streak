//! # deck-streak-quests
//!
//! What this context owns: Variable rewards: the daily quest arc, the weekly meta-quest,
//! session chests with their pity counters, double-XP tokens, Perfect Week smoke bombs and the
//! ghost race.
//!
//! What it does not own: No purchase of anything random: chests and coins are never sold.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod chest_store;
pub mod chests;
pub mod draw;
pub mod pity;
pub mod sessions;
pub mod tokens;
