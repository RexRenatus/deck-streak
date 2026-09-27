//! # deck-streak-api
//!
//! What this context owns: The HTTPS API the Mini App calls: axum routes over the coordination
//! use cases, the `initData` extractor, and the in-app transport of the notification router.
//!
//! What it does not own: Any use case of its own: a route maps a request to a coordination use
//! case.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]
