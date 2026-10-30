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
//!
//! Badges, records and the next milestone (SPEC-073, ADR-303): [`badges`] holds the catalog, the
//! award port over `badges_earned`, the study conditions and the hour windows; [`records`] the
//! record detection and the record to chase; and [`milestone`] the three ladders and the nearest
//! rung.
//!
//! The personal board and the XP exchange readout (SPEC-075, ADR-075): [`board`] ranks the
//! learner's own stored days and labels the board's rows; [`exchange`] reads both XP tables and
//! folds them into each source bucket's XP per graduated card.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod badges;
pub mod board;
pub mod bonus;
pub mod buffs;
pub mod consistency;
pub mod data_rights;
pub mod economy_config;
pub mod exchange;
pub mod grant;
pub mod ledger;
pub mod level;
pub mod milestone;
pub mod records;
pub mod review_xp;
pub mod settle;
pub mod xp;

/// The read side of the settlement, at the crate root: a day's settled rows are what the level view
/// shows, and reading them is not the write that only the recompute and the owner's correction make
/// through [`settle::settle`].
pub use settle::{SettledRow, settled_days, settled_of_day};
