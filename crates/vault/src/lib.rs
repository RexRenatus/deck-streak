//! # deck-streak-vault
//!
//! What this context owns: The anti-corruption layer for the owner's second-brain vault: the
//! versioned file contract, configured paths, atomic writes under ignored temp names, the
//! content rails that block executable Obsidian content, staged duty runs, the stats bridge,
//! drills and inbox capture.
//!
//! What it does not own: The vault's sync infrastructure, guards and backups, which stay in
//! private operations.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
