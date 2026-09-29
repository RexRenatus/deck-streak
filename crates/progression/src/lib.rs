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
//!
//! The XP ledger (SPEC-040, ADR-040): [`xp`] holds the unsigned amount, the total and the level
//! curve; [`grant`] the grant port every use case grants through; [`ledger`] the repository over
//! `xp_ledger` that answers it; and [`data_rights`] the ledger's export and erase.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod bonus;
pub mod consistency;
pub mod data_rights;
pub mod economy_config;
pub mod grant;
pub mod ledger;
pub mod level;
pub mod review_xp;
pub mod settle;
pub mod xp;
