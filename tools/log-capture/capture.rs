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

use std::cell::Cell;
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
/// A `Dispatch` made outside this helper registers one, and so does an interest rebuild with no
/// dispatcher registered, which raises the level filter above `OFF`. The census refuses the
/// names that do either, from a fixed list of the pinned crates' public items.
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

thread_local! {
    /// How many captures this helper holds on this thread. It decides whether a capture is
    /// nested: the default alone cannot, since it is not the floor where the floor is missing too.
    static HELD: Cell<usize> = const { Cell::new(0) };
}

/// Counts one more capture held on this thread.
fn raise(held: &Cell<usize>) {
    held.set(held.get() + 1);
}

/// Counts one capture fewer held on this thread, when a capture ends.
fn lower(held: &Cell<usize>) {
    held.set(held.get() - 1);
}

/// Refuses a capture made while this thread already holds one, and a capture made where the floor
/// is not this thread's default, each by a message naming its own cause.
///
/// A nested capture would take every line from the outer one, so a test asserting on the outer
/// capture could pass while its lines went elsewhere. A thread holding no capture has the floor as
/// its default, so outside a dispatcher's own call any other default means a capture is held.
/// Inside a dispatcher's own call the default reads as none while any thread holds a scoped
/// default, and as the global default while none does. So the count of captures this helper holds
/// on this thread decides nesting, and a default that is not the floor while that count is 0 is
/// refused as a missing floor. A capture held on another thread is no obstacle: this thread's count
/// is 0 and its default is still the floor.
fn refuse_nested_capture() {
    let held = HELD.with(Cell::get);
    assert!(
        held == 0,
        "a capture nested inside another capture on one thread is refused"
    );
    let on_floor = tracing::dispatcher::get_default(tracing::Dispatch::is::<Floor>);
    assert!(
        on_floor,
        "a capture is refused: the floor is not this thread's default"
    );
}

/// The count `with_capture` raised for its body, lowered when the body returns or unwinds.
struct Scoped;

impl Drop for Scoped {
    fn drop(&mut self) {
        HELD.with(lower);
    }
}

/// A capture `hold_capture` made: `subscriber` stays this thread's default, and the capture stays
/// counted, until this guard drops.
#[must_use = "dropping the guard ends the capture"]
pub struct CaptureGuard {
    _default: DefaultGuard,
}

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        HELD.with(lower);
    }
}

/// Runs `body` with `subscriber` as this thread's default, and returns what it returns.
pub fn with_capture<S, T>(subscriber: S, body: impl FnOnce() -> T) -> T
where
    S: Subscriber + Send + Sync + 'static,
{
    register_floor();
    refuse_nested_capture();
    HELD.with(raise);
    let _scoped = Scoped;
    tracing::subscriber::with_default(subscriber, body)
}

/// Makes `subscriber` this thread's default until the guard drops.
pub fn hold_capture<S>(subscriber: S) -> CaptureGuard
where
    S: Subscriber + Send + Sync + 'static,
{
    register_floor();
    refuse_nested_capture();
    let default = tracing::subscriber::set_default(subscriber);
    HELD.with(raise);
    CaptureGuard { _default: default }
}
