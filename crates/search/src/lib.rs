//! # deck-streak-search
//!
//! What this crate owns: the product's own search (SPEC-514, ADR-532). A search is read once into
//! a typed [`Query`] under a [`Profile`] of the closed grammar, within [`Limits`], and every
//! refusal is named by a [`Refusal`] that never carries the search's text. The evaluator [`eval`]
//! answers the cards a query selects over a [`Collection`]'s rows on an [`EngineDay`], held to the
//! engine's goldens, and the skip day's wrap ([`SkipQuery`]) joins its holds to the configured
//! search by structure, never by text.
//!
//! What it does not own: reading a collection, running the engine, or the study day's rule. No
//! source here reads a clock, a file, the environment, the network, a process or a thread, and the
//! crate depends only on `deck-streak-study-day`, so it has no edge to the kernel.

pub mod collection;
pub mod day;
pub mod eval;
pub mod parse;
pub mod pattern;
pub mod profile;
pub mod query;
pub mod skip;

pub use collection::{Card, Collection, Deck, Note, Notetype, Review};
pub use day::EngineDay;
pub use eval::{LEARN_AHEAD_SECS, eval, selects};
pub use parse::{Limits, Refusal};
pub use profile::Profile;
pub use query::{Compare, Operator, Prop, Query, State, Template, Term};
pub use skip::{HOLDS, REVIEW_ONLY, SkipQuery, holds};
