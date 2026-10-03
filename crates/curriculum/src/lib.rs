//! # deck-streak-curriculum
//!
//! What this context owns: What is being learned: Road to C2 and CEFR bands per language, the
//! forecast, the adaptive daily goal, skill strands and weak spots, cross-language balance,
//! memory health, the obligation horizon, the Can-Do ladder, the law track summary and LSAT
//! board, and leech remediation.
//!
//! What it does not own: No XP amounts and no notification decisions, except the one amount the
//! band-up pays, which the predecessor kept outside its economy file (`progress::XP_BONUS_BAND_UP`).
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod data_rights;
pub mod horizon;
pub mod law;
pub mod progress;
pub mod store;
pub mod unit_bands;
