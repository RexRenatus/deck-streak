//! # deck-streak-agent
//!
//! What this context owns: The AI agent: duty runs through the headless Claude Code runner
//! behind the subscription proxy, persona instantiation from public templates and the private
//! roster, prompt composition with untrusted text fenced as data, the output gate that runs the
//! packs' blocking checks, per-run turn and time caps, and fail-closed degradation.
//!
//! What it does not own: Any credential: the device key reaches the runner's environment at
//! launch and never this code's.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
