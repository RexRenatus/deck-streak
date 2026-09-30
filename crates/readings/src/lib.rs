//! # deck-streak-readings
//!
//! What this context owns: The flagship daily pre-study readings: the day set from the
//! scheduler's new cards, one reading per topic, generation through the agent, the coverage and
//! safety gates, the read tap, the studied measure, rollover and archive, on-demand
//! regeneration, the one comeback reading per lapse, reading XP, and the readings' health.
//!
//! What it does not own: The minutes-of-reading habit (`/read`), which is the habits context's.
//!
//! SPEC-045 builds the first of it: the private taxonomy ([`taxonomy`]), deck to topic
//! ([`topic`]), the last-sync and pause gates ([`gates`]), the day set from the scheduler's own
//! queue ([`day_set`]), the closed states a topic ends a study day in ([`state`]), their record
//! ([`store`]) and its data-rights port ([`data_rights`]).
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod attempts;
pub mod coverage;
pub mod data_rights;
pub mod day_set;
pub mod form;
pub mod gates;
pub mod reading;
pub mod repair;
pub mod seed;
pub mod state;
pub mod store;
pub mod taxonomy;
pub mod topic;
