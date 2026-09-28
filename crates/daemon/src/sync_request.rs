//! The owner's `/sync` as a request for the sync job (SPEC-059; ADR-066).
//!
//! The bot's memory limit is below what one sync needs, so the bot runs no cycle: [`SyncRequester`]
//! stores the owner's request (the rescore flag), rings a doorbell (a file a path unit watches),
//! and waits a bounded time for the job's outcome. The doorbell carries nothing the job reads.

use std::future::Future;
use std::io;
use std::sync::Mutex;
use std::time::Duration;

use deck_streak_bot::{OwnerSync, SyncAnswer, SyncRefusal};
use deck_streak_kernel::{Clock, Db, KernelError, UtcMillis};

/// The least gap between two rings of the doorbell, in seconds: under the job unit's start limit,
/// so a burst of requests never blocks the timer's own start.
pub const RING_GAP_SECS: i64 = 15;
/// How often the requester looks for the outcome, in seconds.
pub const POLL_SECS: i64 = 2;
/// The longest the owner waits for an outcome before being told the sync is still running.
pub const ANSWER_BOUND_SECS: i64 = 120;

/// How far the job has got with one request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// The request is still pending.
    Waiting,
    /// The job ran a sync for it; `failure` is the reason code when it failed.
    Ran {
        /// The sync's reason code, when it failed.
        failure: Option<String>,
    },
    /// The request was served without a sync: the reuse window answered it.
    Reused,
}

/// The store's side of a request.
pub trait RequestLedger: Send + Sync {
    /// Records the owner's request at `at`.
    fn request(&self, at: UtcMillis) -> impl Future<Output = Result<(), KernelError>> + Send;
    /// How far the job has got with the request made at `since`.
    fn progress(
        &self,
        since: UtcMillis,
    ) -> impl Future<Output = Result<Progress, KernelError>> + Send;
}

/// The doorbell: one write the path unit sees.
pub trait Doorbell: Send + Sync {
    /// Touches the request file.
    ///
    /// # Errors
    ///
    /// The write's error, when the file cannot be written.
    fn ring(&self) -> io::Result<()>;
}

/// A wait, injected so a test moves no real time.
pub trait Pause: Send + Sync {
    /// Waits `by`.
    fn pause(&self, by: Duration) -> impl Future<Output = ()> + Send;
}

/// The bot's `/sync` port: a request for the job, never a cycle.
#[derive(Debug)]
pub struct SyncRequester<C, L, D, P> {
    clock: C,
    ledger: L,
    doorbell: D,
    pause: P,
    last_ring: Mutex<Option<UtcMillis>>,
}

impl<C: Clock, L: RequestLedger, D: Doorbell, P: Pause> SyncRequester<C, L, D, P> {
    /// A requester over its four ports.
    #[must_use]
    pub const fn new(clock: C, ledger: L, doorbell: D, pause: P) -> Self {
        Self {
            clock,
            ledger,
            doorbell,
            pause,
            last_ring: Mutex::new(None),
        }
    }
}

impl<C: Clock, L: RequestLedger, D: Doorbell, P: Pause> OwnerSync for SyncRequester<C, L, D, P> {
    fn sync_now(&self) -> impl Future<Output = Result<SyncAnswer, SyncRefusal>> + Send {
        let _ = (
            &self.clock,
            &self.ledger,
            &self.doorbell,
            &self.pause,
            &self.last_ring,
        );
        async { Err(SyncRefusal { reason: "unbuilt" }) }
    }
}

/// Whether the store holds an owner's request the job has not served.
///
/// # Errors
///
/// [`KernelError::Database`] when the state cannot be read.
pub async fn owner_request_pending(db: &Db) -> Result<bool, KernelError> {
    let _ = db;
    Ok(false)
}
