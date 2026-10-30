//! # deck-streak-study-day
//!
//! What this crate owns: the study day the kernel counts in (SPEC-497, ADR-515): the rollover rule
//! in a fixed UTC offset and its ISO calendar, the instant it is computed from ([`UtcMillis`]) and
//! the error a malformed date gives ([`IsoDateError`]).
//!
//! What it does not own: the clock. No source here reads the system time, a file, the
//! environment, the network, a process or a thread, and the crate depends on nothing, so the
//! server and both clients link one rule. The kernel depends on it and re-exports each item at the
//! path it always had; `SystemClock` and `ManualClock` stay in the kernel.

pub mod error;
pub mod instant;
pub mod study_day;

pub use error::IsoDateError;
pub use instant::UtcMillis;
pub use study_day::{Hour, StudyDay, StudyDayRule, UtcOffset};
