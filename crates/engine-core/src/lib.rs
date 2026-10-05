//! The engine core: the one client-side holder of Anki's engine (SPEC-345, ADR-356).
//!
//! Both clients reach the engine through a [`dispatch::Dispatcher`] started on their own
//! [`table::Transport`]: the native adapter (`deck-streak-ffi`) on `Native`, the web engine
//! (`deck-streak-web-engine`) on `Web`. A call reaches the engine only when the core's table admits
//! its pair on that transport; an exempt write is held for an owner's gesture, and every other pair
//! is refused before the engine sees it. The engine's database door takes no SQL from an adapter:
//! a read is one of a closed set whose statements the core holds.
//!
//! - [`table`]: the transports, the ordinary table with its transport columns, the exempt table
//!   and the decision for a pair (R2, R3).
//! - [`dispatch`]: the dispatcher, its refusal and its fixed reads (R1, R4).
//! - [`login_guard`]: the endpoint guard on the engine's sync login (SPEC-347 R2).
//!
//! The core depends on the engine and on no crate of this workspace; only the two client
//! adapters depend on it (ADR-356 D4, held by the graph census).

#![forbid(unsafe_code)]

pub mod dispatch;
pub mod login_guard;
pub mod table;
