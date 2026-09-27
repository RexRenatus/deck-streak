//! # deck-streak-daemon
//!
//! What this context owns: The composition root: it reads the configuration, builds every
//! context's adapters, joins their ports, and runs the API, the bot and the scheduled jobs
//! under systemd. Nothing depends on it.
//!
//! What it does not own: Any logic a context could hold.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
