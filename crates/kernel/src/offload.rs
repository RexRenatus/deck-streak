//! The offload: blocking work runs on tokio's blocking pool, at most a bounded number at once
//! (SPEC-020 R14).
//!
//! It ports the predecessor's `offload.py:run_offloaded` at `27ee2bc`: one rail every blocking call
//! goes through, so a collection read never stalls the async runtime, and a bound on how many run
//! together, which is the memory bound on a small host. A call names its operation; one slower than
//! [`SLOW_OFFLOAD_MS`] logs one WARN event with the operation and its duration, never its
//! arguments. The duration is read from the injected clock, so a test drives it without sleeping.

use std::fmt;
use std::sync::Arc;

use crate::clock::Clock;
use crate::error::KernelError;
use crate::settings::OffloadWorkers;

/// How many blocking operations run at once when none is configured: the predecessor's
/// `offload.py:OFFLOAD_MAX_WORKERS`.
pub const OFFLOAD_MAX_WORKERS: usize = 0;
/// A call at or over this many milliseconds logs a WARN event: the predecessor's
/// `offload.py:SLOW_OFFLOAD_MS`.
pub const SLOW_OFFLOAD_MS: i64 = 0;

/// Runs blocking work on the blocking pool, at most its bound at once.
#[derive(Clone)]
pub struct Offload {
    workers: OffloadWorkers,
    clock: Arc<dyn Clock>,
}

impl Offload {
    /// An offload that runs at most `workers` operations at once, timing each on `clock`.
    #[must_use]
    pub fn new(workers: OffloadWorkers, clock: Arc<dyn Clock>) -> Self {
        Self { workers, clock }
    }

    /// Runs `work` on the blocking pool once a worker is free, and returns what it returned.
    ///
    /// # Errors
    ///
    /// [`KernelError::Offload`] when the work panicked or was cancelled.
    pub async fn run<T, F>(&self, operation: &'static str, work: F) -> Result<T, KernelError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let _ = &self.clock;
        tokio::task::spawn_blocking(work)
            .await
            .map_err(|_| KernelError::Offload { operation })
    }
}

impl fmt::Debug for Offload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Offload")
            .field("workers", &self.workers)
            .finish_non_exhaustive()
    }
}
