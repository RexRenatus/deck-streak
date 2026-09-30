//! The one way a test captures log lines (SPEC-024, the 2026-09-30 amendment).
//!
//! A test that captures lines receives every line its thread emits, provided the floor is
//! installed first and nothing but this helper registers a dispatcher or a callsite. The killer's
//! census holds every test file to that. A line emitted inside a dispatcher's own call, such as the
//! closure `tracing::dispatcher::get_default` runs, reaches no subscriber: `tracing` drops it by
//! design, with or without this helper. This file is compiled into a test binary, and never into
//! production code:
//!
//! ```text
//! #[path = "../../../tools/log-capture/capture.rs"]
//! mod log_capture;
//! ```
//!
//! It is one file included by path, so every capture in the workspace is made one way. The killer
//! in `crates/kernel/tests/log_capture_class.rs` checks that no test captures another way.

// Each including test binary calls only the entry it needs.
#![allow(dead_code)]

use std::sync::OnceLock;

use tracing::level_filters::LevelFilter;
use tracing::span::{Attributes, Record};
use tracing::subscriber::{DefaultGuard, Interest};
use tracing::{Dispatch, Event, Id, Metadata, Subscriber};

/// The default of every thread that holds no capture: installed once, as the global default,
/// before any capture, and never dropped.
///
/// `tracing-core` caches a callsite's interest when a thread first reaches it.
/// While at most one dispatcher was registered at the last registration (`Dispatchers::rebuilder`),
/// it asks only the reaching thread's default (`dispatcher::get_default`) and takes no lock.
/// That answer can be stored after a capture registered on another thread, overwriting the
/// capture's.
///
/// A thread holding no capture has this floor as its default. The floor answers every callsite
/// `sometimes` and enables nothing, so an answer computed once it is installed is never `never`.
///
/// An answer computed before the floor is installed can be `never`, and can still be stored after
/// a capture registers. So nothing may register a dispatcher or a callsite before the floor.
/// No `tracing` macro does: the level filter starts `OFF` and rises only when a dispatcher
/// registers. A `Dispatch` made outside this helper would raise it, and the census refuses one.
struct Floor;

impl Subscriber for Floor {
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::sometimes()
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        false
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::OFF)
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(0xDEAD)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, _: &Event<'_>) {}

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

/// Set once the floor is the global default.
static FLOOR: OnceLock<()> = OnceLock::new();

/// Installs the floor as the global default before a capture registers its own dispatcher.
fn register_floor() {
    FLOOR.get_or_init(|| {
        let installed = tracing::dispatcher::set_global_default(Dispatch::new(Floor));
        assert!(
            installed.is_ok(),
            "a test binary that captures lines installs no other global default"
        );
    });
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
