//! # deck-streak-identity
//!
//! What this context owns: Who is asking: Telegram Mini App `initData` validated on the server
//! (HMAC-SHA256 with the `WebAppData` key, a bounded `auth_date`, a constant-time compare), the
//! owner allow-list, sessions, and the sign-in methods linked to the Telegram account.
//!
//! What it does not own: No product data. It answers who the caller is and whether they are the
//! owner.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
