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
//! SPEC-041 builds the router's core: the typed policy compiled from `notifications-policy.json`
//! ([`policy`]), the occasion and its vocabulary ([`occasion`]), the quiet window ([`quiet`]), the
//! router that decides, delivers and flushes ([`router`]), its ledger of decisions, deliveries,
//! held celebrations, the in-app feed and the owner's settings ([`ledger`]), the bot transport port
//! whose calls only the router can make ([`transport`]), and the data-rights port over its tables
//! ([`data_rights`]).
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod data_rights;
pub mod ledger;
pub mod occasion;
pub mod policy;
pub mod quiet;
pub mod router;
pub mod transport;

pub use occasion::{
    Class, DedupeKey, DedupeScope, Kind, LapseContext, Occasion, OccasionError, Surface, Tier,
};
pub use policy::{Policy, PolicyError};
pub use router::{Decision, Flushed, Hold, Pass, Reason, Router};
pub use transport::{BotTransport, PushFuture, Pushed};
