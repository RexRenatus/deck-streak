//! # deck-streak-daemon
//!
//! What this context owns: The composition root: it reads the configuration, builds every
//! context's adapters, joins their ports, and runs the API, the bot and the scheduled jobs
//! under systemd. Nothing depends on it.
//!
//! What it does not own: Any logic a context could hold.
//!
//! The `deckstreakd` binary (`src/main.rs`) dispatches on its first argument to a role. What the
//! roles share lives here, where the tests reach it: the process lifecycle under systemd
//! ([`lifecycle`]), the adapters the roles build and the ports they join ([`wiring`]), the `api`
//! role ([`role_api`], SPEC-025), the `bot` role ([`role_bot`], SPEC-026) and the `mcp` role
//! ([`role_mcp`], SPEC-119). The `api` role's listing of the archive for the snapshot answer runs
//! the list command, bounded ([`snapshot_lister`], SPEC-377).
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod drill_vault;
pub mod lifecycle;
pub mod role_api;
pub mod role_bot;
pub mod role_mcp;
pub mod snapshot_lister;
pub mod sync_request;
pub mod wiring;
