//! The one way a test captures log lines (SPEC-024, the 2026-09-30 amendment).
//!
//! A test that captures lines receives every line its code emits, whichever thread first reached
//! that line's callsite and whatever the other tests in its binary did first. This file is
//! compiled into a test binary, and never into production code:
//!
//! ```text
//! #[path = "../../../tools/log-capture/capture.rs"]
//! mod log_capture;
//! ```
//!
//! It is one file included by path, so every capture in the workspace is made one way and the
//! killer in `crates/kernel/tests/log_capture_class.rs` proves that way for all of them.

// Each including test binary calls only the entry it needs.
#![allow(dead_code)]

use std::sync::OnceLock;

use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Subscriber};

/// A dispatcher registered for the life of the test binary and never dropped.
///
/// `tracing-core` computes a new callsite's interest from every registered dispatcher only while
/// two or more are registered (`Dispatchers::rebuilder`); with exactly one it asks the reaching
/// thread's default alone (`dispatcher::get_default`). A thread with no subscriber that first
/// reaches a line while a capture is the only registered dispatcher therefore caches the line as
/// never enabled, and the capture on another thread never sees it. This dispatcher is registered
/// before any capture, so a capture is never the only one, and a callsite's interest always
/// includes the capture's.
static FLOOR: OnceLock<Dispatch> = OnceLock::new();

/// Registers the floor dispatcher before a capture registers its own.
fn register_floor() {
    FLOOR.get_or_init(|| Dispatch::new(NoSubscriber::default()));
}

/// Runs `body` with `subscriber` as this thread's default, and returns what it returns.
pub fn with_capture<S, T>(subscriber: S, body: impl FnOnce() -> T) -> T
where
    S: Subscriber + Send + Sync + 'static,
{
    register_floor();
    tracing::subscriber::with_default(subscriber, body)
}

/// Makes `subscriber` this thread's default until the guard drops.
pub fn hold_capture<S>(subscriber: S) -> DefaultGuard
where
    S: Subscriber + Send + Sync + 'static,
{
    register_floor();
    tracing::subscriber::set_default(subscriber)
}
