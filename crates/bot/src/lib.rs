//! # deck-streak-bot
//!
//! What this context owns: The Telegram bot adapter: the Bot API transport (flood control,
//! escaping, chunking), commands and callbacks over the coordination use cases, the owner gate,
//! and the bot transport of the notification router.
//!
//! What it does not own: Any use case of its own, and any second decision on whether a message
//! may be sent.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
