//! # deck-streak-progression
//!
//! What this context owns: The XP ledger and its idempotent grant port, per-review XP, daily
//! bonuses, the level curve, the Bloom-tier multiplier, badges, personal records, the milestone
//! ladder, seasons and season nodes, the personal leaderboard and XP exchange rates.
//!
//! What it does not own: No coins and no streak state. A penalty never touches XP, levels or
//! badges.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
