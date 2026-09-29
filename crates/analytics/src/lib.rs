//! # deck-streak-analytics
//!
//! What this context owns: Per-study-day facts recomputed from reviews: the daily rollup,
//! per-language statistics, the five-pillar daily score and its grade band, and the today
//! snapshot.
//!
//! What it does not own: No reward. XP, streaks and coins read these facts and own their own
//! rules.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod constants;
pub mod data_rights;
pub mod metrics;
pub mod rollup;
pub mod score;
pub mod settings;
pub mod snapshot;
