//! # deck-streak-ingest
//!
//! What this context owns: The anti-corruption layer for Anki: syncing a private copy of the
//! collection from the owner's Anki sync server, reading it read-only inside the bounded
//! window, the change gate that skips a recompute when nothing changed, the scheduler's queue
//! of today's new cards, and the one write back to Anki, the skip day.
//!
//! What it does not own: No game rule. It publishes reviews, cards, notes, decks and queues as
//! this workspace's own types, so no other context learns Anki's schema.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod calendar;
pub mod data_rights;
pub mod engine;
pub mod gate;
pub mod lock;
pub mod memory_state;
pub mod reader;
pub mod settings;
pub mod skip;
pub mod skip_write;
pub mod state;
pub mod structure;
pub mod study_days;
pub mod sync;
pub mod sync_runs;
pub mod tier;
pub mod window;
pub mod wire;
pub mod write_class_stop;

// SPEC-387's preset read and proposal, in a group of its own so that the declarations the crate's
// source-text guards pin keep their neighbours.
pub mod preset;
