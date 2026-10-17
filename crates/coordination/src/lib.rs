//! # deck-streak-coordination
//!
//! What this context owns: The use cases and scheduled jobs that cross contexts: the per-sync
//! recompute in order, the nightly and hourly jobs with the cron-fire ledger, the registry of every
//! context's data-rights port with the owner's export and erase over it, the resolution of a study
//! day's reading topics, and each owner action and read the bot and the Mini App share, so both
//! surfaces run one code path (the score reads, SPEC-071).
//!
//! What it does not own: Any domain rule: it calls the contexts in order and holds none of
//! their logic.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod courses;
pub mod data_rights;
pub mod data_rights_registry;
pub mod delivery;
pub mod drills;
pub mod freeze;
pub mod habits;
pub mod held_flush;
pub mod inbox_capture;
pub mod instruments;
pub mod jobs;
pub mod ladder_facts;
pub mod lapse;
pub mod law;
pub mod ledger;
pub mod level_up;
pub mod liveness;
pub mod maintenance;
pub mod obligations;
pub mod progress_view;
pub mod progression;
pub mod readings;
pub mod recompute;
pub mod relight;
pub mod runner;
pub mod score;
pub mod sensitive_decks;
pub mod skip;
pub mod streak_views;
pub mod sync_cycle;
pub mod wallet_view;
