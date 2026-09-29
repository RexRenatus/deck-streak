//! # deck-streak-bot
//!
//! What this context owns: The Telegram bot adapter: the Bot API transport (flood control,
//! escaping, chunking), commands and callbacks over the coordination use cases, the owner gate,
//! and the bot transport of the notification router.
//!
//! What it does not own: Any use case of its own, and any second decision on whether a message
//! may be sent.
//!
//! SPEC-026 builds the transport and the owner gate: the one client to the Bot API
//! ([`transport`]), the split of a long text into messages that each parse ([`chunk`]), the gate
//! every update passes ([`gate`]), the long poll with its drain, offset and backoff ([`poll`]), and
//! the owner's commands ([`commands`]). SPEC-071 adds the owner's `/score` ([`score_commands`]). The
//! daemon's `bot` role builds them and joins the transport's
//! counts to coordination's delivery marker (ADR-026).
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod chunk;
pub mod commands;
pub mod gate;
pub mod poll;
pub mod score_commands;
pub mod transport;

pub use commands::{Commands, MiniAppUrl, OwnerSync, Scores, SyncAnswer, SyncOutcome, SyncRefusal};
pub use poll::run;
pub use transport::{ApiUrl, SendCounts, Sent, Transport, TransportError};
