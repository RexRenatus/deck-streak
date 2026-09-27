//! # deck-streak-notifications
//!
//! What this context owns: The one router for the bot and the Mini App: the celebration ladder
//! T0-T5, weekly budgets, the streak-break cap, dedupe, quiet hours and deferral, the
//! failed-send hold, nudge budgets, the comeback cap, the holdout and the withhold ledger; and
//! the digests, briefs and nudges it sends.
//!
//! What it does not own: The words of a message, which the duty that writes it owns, and any
//! transport, which the bot and API adapters implement.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
