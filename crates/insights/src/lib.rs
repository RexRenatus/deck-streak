//! # deck-streak-insights
//!
//! What this context owns: Read-only research instruments behind one instrument port, and the
//! chart series the Mini App renders on the client.
//!
//! What it does not own: Any state: an instrument refuses rather than invents, and writes
//! nothing.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod dark_fields;
pub mod instrument;
pub mod registry;
