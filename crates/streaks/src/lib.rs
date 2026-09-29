//! # deck-streak-streaks
//!
//! What this context owns: The language and law streaks with freezes, heat tiers and lapse
//! decay, habit strength and the anti-abandonment governor (armed, standby, lapse), and the
//! relight ritual.
//!
//! What it does not own: Nothing confiscable: a streak is never staked or fined.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod constants;
pub mod data_rights;
pub mod freeze;
pub mod governor;
pub mod lapse;
pub mod law;
pub mod relight;
pub mod streak;
pub mod strength;
