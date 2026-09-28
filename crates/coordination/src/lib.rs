//! # deck-streak-coordination
//!
//! What this context owns: The use cases and scheduled jobs that cross contexts: the per-sync
//! recompute in order, the nightly and hourly jobs with the cron-fire ledger, and each owner
//! action the bot and the Mini App share, so both surfaces run one code path.
//!
//! What it does not own: Any domain rule: it calls the contexts in order and holds none of
//! their logic.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod data_rights;
pub mod delivery;
pub mod jobs;
pub mod ledger;
pub mod liveness;
pub mod maintenance;
pub mod obligations;
pub mod runner;
pub mod sync_cycle;
