//! The owner's `/sync` as a request for the sync job (SPEC-059; ADR-066).
//!
//! The bot's memory limit is below what one sync needs, so the bot runs no cycle: [`SyncRequester`]
//! stores the owner's request (the rescore flag), rings a doorbell (a file a path unit watches),
//! and waits a bounded time for the job's outcome. The doorbell carries nothing the job reads.

use std::fs::OpenOptions;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use deck_streak_bot::{OwnerSync, Scores, SyncAnswer, SyncOutcome, SyncRefusal};
use deck_streak_ingest::state::SqliteIngestState;
use deck_streak_ingest::sync_runs::{RunStatus, SqliteSyncRuns};
use deck_streak_kernel::{Clock, Db, Environment, KernelError, Setting, SettingsError, UtcMillis};
use deck_streak_notifications::Router;

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
    /// The job refused the request (SPEC-128); `reason` is the refusal's code.
    Refused {
        /// The refusal's code, one of the closed set.
        reason: String,
    },
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

/// The flush the bot makes when the owner's sync has succeeded: the notification router's, in
/// production (SPEC-041 R7; SPEC-059 R8). A port so the bot's answer can be tested without one.
pub trait Flush: Send + Sync {
    /// Delivers what quiet hours and failed sends held.
    ///
    /// # Errors
    ///
    /// The store's error, when the queue cannot be read or written.
    fn flush(&self) -> impl Future<Output = Result<(), KernelError>> + Send;
}

/// No flush: a requester built without a router.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFlush;

impl Flush for NoFlush {
    async fn flush(&self) -> Result<(), KernelError> {
        Ok(())
    }
}

impl Flush for Arc<Router> {
    async fn flush(&self) -> Result<(), KernelError> {
        let flushed = Router::flush(self).await?;
        tracing::info!(?flushed, "the notification router flushed");
        Ok(())
    }
}

/// The bot's `/sync` port: a request for the job, never a cycle.
#[derive(Debug)]
pub struct SyncRequester<C, L, D, P, F = NoFlush> {
    clock: C,
    ledger: L,
    doorbell: D,
    pause: P,
    flush: F,
    last_ring: Mutex<Option<UtcMillis>>,
}

impl<C: Clock, L: RequestLedger, D: Doorbell, P: Pause> SyncRequester<C, L, D, P> {
    /// A requester over its four ports, with no flush.
    #[must_use]
    pub const fn new(clock: C, ledger: L, doorbell: D, pause: P) -> Self {
        Self {
            clock,
            ledger,
            doorbell,
            pause,
            flush: NoFlush,
            last_ring: Mutex::new(None),
        }
    }

    /// The same requester, flushing through `flush` once it observes the owner's request answered
    /// by a sync that ran and succeeded (SPEC-059 R8).
    #[must_use]
    pub fn with_flush<F: Flush>(self, flush: F) -> SyncRequester<C, L, D, P, F> {
        SyncRequester {
            clock: self.clock,
            ledger: self.ledger,
            doorbell: self.doorbell,
            pause: self.pause,
            flush,
            last_ring: self.last_ring,
        }
    }
}

impl<C: Clock, L: RequestLedger, D: Doorbell, P: Pause, F: Flush> SyncRequester<C, L, D, P, F> {
    /// Rings the doorbell no sooner than [`RING_GAP_SECS`] after the last ring. The slot is
    /// reserved before the wait, so concurrent requests take successive slots.
    async fn ring(&self) -> Result<(), SyncRefusal> {
        let now = self.clock.now().epoch_millis();
        let slot = {
            let mut last = self
                .last_ring
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let slot = last.map_or(now, |last: UtcMillis| {
                now.max(last.epoch_millis() + RING_GAP_SECS * 1000)
            });
            *last = Some(UtcMillis::from_epoch_millis(slot));
            slot
        };
        let wait = u64::try_from(slot - now).unwrap_or_default();
        self.pause.pause(Duration::from_millis(wait)).await;
        self.doorbell.ring().map_err(|error| {
            tracing::error!(%error, "the sync request could not be written");
            SyncRefusal {
                reason: "sync_request_unwritten",
            }
        })
    }

    async fn answer(&self) -> Result<SyncAnswer, SyncRefusal> {
        let since = self.clock.now();
        self.ledger.request(since).await.map_err(|error| {
            tracing::error!(%error, "the owner's request could not be stored");
            SyncRefusal {
                reason: "rescore_unrecorded",
            }
        })?;
        self.ring().await?;
        let deadline = since.epoch_millis() + ANSWER_BOUND_SECS * 1000;
        loop {
            let progress = self.ledger.progress(since).await.map_err(|error| {
                tracing::error!(%error, "the owner's request could not be followed");
                SyncRefusal {
                    reason: "sync_progress_unread",
                }
            })?;
            let sync = match progress {
                Progress::Ran { failure: None } => SyncOutcome::Synced,
                Progress::Ran {
                    failure: Some(reason),
                } => SyncOutcome::Failed { reason },
                Progress::Reused | Progress::Refused { .. } => SyncOutcome::Reused,
                Progress::Waiting if self.clock.now().epoch_millis() >= deadline => {
                    return Ok(SyncAnswer {
                        sync: SyncOutcome::StillRunning,
                        scores: Scores::Unchanged,
                    });
                }
                Progress::Waiting => {
                    self.pause
                        .pause(Duration::from_secs(POLL_SECS.unsigned_abs()))
                        .await;
                    continue;
                }
            };
            if sync == SyncOutcome::Synced
                && let Err(error) = self.flush.flush().await
            {
                tracing::error!(%error, "the notification router could not flush");
            }
            return Ok(SyncAnswer {
                sync,
                scores: Scores::Recomputed,
            });
        }
    }
}

impl<C: Clock, L: RequestLedger, D: Doorbell, P: Pause, F: Flush> OwnerSync
    for SyncRequester<C, L, D, P, F>
{
    fn sync_now(&self) -> impl Future<Output = Result<SyncAnswer, SyncRefusal>> + Send {
        self.answer()
    }
}

/// The store's side of a request in production: the rescore flag and the owner's run rows.
#[derive(Clone, Debug)]
pub struct SqliteRequestLedger {
    db: Db,
}

impl SqliteRequestLedger {
    /// The ledger over `db`.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }
}

impl RequestLedger for SqliteRequestLedger {
    async fn request(&self, at: UtcMillis) -> Result<(), KernelError> {
        SqliteIngestState::new(self.db.clone())
            .request_rescore(at)
            .await
    }

    async fn progress(&self, since: UtcMillis) -> Result<Progress, KernelError> {
        if owner_request_pending(&self.db).await? {
            return Ok(Progress::Waiting);
        }
        let run = SqliteSyncRuns::new(self.db.clone())
            .owner_run_since(since)
            .await?;
        Ok(run.map_or(Progress::Reused, |run| Progress::Ran {
            failure: (run.status == RunStatus::Error)
                .then(|| run.reason.unwrap_or_else(|| "unknown".to_owned())),
        }))
    }
}

/// The doorbell in production: the request file the path unit watches. It is created or emptied
/// and closed, and its content is never read by anyone.
#[derive(Clone, Debug)]
pub struct FileDoorbell {
    path: PathBuf,
}

impl FileDoorbell {
    /// The doorbell at `path`.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Doorbell for FileDoorbell {
    fn ring(&self) -> io::Result<()> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .map(drop)
    }
}

/// The wait in production: the runtime's timer.
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioPause;

impl Pause for TokioPause {
    fn pause(&self, by: Duration) -> impl Future<Output = ()> + Send {
        tokio::time::sleep(by)
    }
}

/// The environment variable that moves the request file; the default is [`DEFAULT_REQUEST_PATH`].
pub const REQUEST_PATH_ENV: &str = "DECKSTREAK_SYNC_REQUEST_PATH";
/// Where the bot rings the doorbell and the path unit listens (`deploy/systemd`).
pub const DEFAULT_REQUEST_PATH: &str = "/run/deck-streak-sync/request";

/// An absolute path setting: where the request file is.
struct RequestFile(PathBuf);

impl Setting for RequestFile {
    const SHAPE: &'static str = "an absolute file path";

    fn parse(text: &str) -> Option<Self> {
        let path = PathBuf::from(text);
        path.is_absolute().then_some(Self(path))
    }
}

/// The request file: [`REQUEST_PATH_ENV`] when set, else the default.
///
/// # Errors
///
/// [`SettingsError::Malformed`] when the variable is set and is not an absolute path.
pub fn request_path(env: &Environment) -> Result<PathBuf, SettingsError> {
    Ok(env
        .optional::<RequestFile>(REQUEST_PATH_ENV)?
        .map_or_else(|| PathBuf::from(DEFAULT_REQUEST_PATH), |file| file.0))
}

/// Whether the store holds an owner's request the job has not served.
///
/// # Errors
///
/// [`KernelError::Database`] when the state cannot be read.
pub async fn owner_request_pending(db: &Db) -> Result<bool, KernelError> {
    Ok(SqliteIngestState::new(db.clone())
        .load()
        .await?
        .rescore_pending)
}
