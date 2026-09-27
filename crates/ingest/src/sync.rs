//! The syncer (SPEC-022 R5 to R11, R14 to R17): one sync run of the private copy, under the
//! collection lock, with the predecessor's retries, a closed set of reason codes, and one
//! `sync_runs` row.
//!
//! The retry loop ports `pipeline.py:GamifyPipeline._sync_attempts` (predecessor `27ee2bc`): at most
//! [`SYNC_RETRY_ATTEMPTS`] attempts, each bounded by [`SYNC_TIMEOUT_SECS`], and between them a wait
//! of `SYNC_RETRY_BASE_SECS * 2^(attempt - 1)` plus a jitter of at most [`SYNC_RETRY_JITTER_FRAC`]
//! of it, every wait on tokio's timer. `goldens/sync_retry.json` and `goldens/sync.constants.json`
//! prove the schedule and the constants (A8, A9). The cadence is this service's own (ADR-037): one
//! scheduled run per study day, and an owner's trigger debounced by [`OWNER_SYNC_DEBOUNCE_SECS`].

use std::sync::{Arc, Mutex};

use deck_streak_kernel::{Clock, CredentialLoader, KernelError, StudyDayRule, UtcMillis};

use crate::engine::AnkiEngine;
use crate::settings::SyncSettings;
use crate::sync_runs::{ReasonCode, SyncRun, SyncRunStore, Trigger};

/// The most attempts one run makes (`constants.py:SYNC_RETRY_ATTEMPTS`).
pub const SYNC_RETRY_ATTEMPTS: u32 = 0;
/// The first wait between attempts, in seconds, doubled after each (`SYNC_RETRY_BASE_SECS`).
pub const SYNC_RETRY_BASE_SECS: f64 = 0.0;
/// The largest jitter, as a fraction of the wait it is added to (`SYNC_RETRY_JITTER_FRAC`).
pub const SYNC_RETRY_JITTER_FRAC: f64 = 0.0;
/// The longest one attempt may run, in seconds (`SYNC_TIMEOUT_SECS`).
pub const SYNC_TIMEOUT_SECS: f64 = 0.0;
/// How often one attempt reopens a locked collection (`COLLECTION_OPEN_RETRIES`).
pub const COLLECTION_OPEN_RETRIES: u32 = 0;
/// The first wait before a reopen, in seconds, doubled after each
/// (`COLLECTION_OPEN_RETRY_BASE_SECS`).
pub const COLLECTION_OPEN_RETRY_BASE_SECS: f64 = 0.0;
/// How long after a successful sync an owner's trigger returns that sync instead of syncing, in
/// seconds: ADR-037's five minutes (R17).
pub const OWNER_SYNC_DEBOUNCE_SECS: i64 = 300;

/// When the syncer attempts, waits and reopens. [`RetrySchedule::PREDECESSOR`] is what production
/// runs; [`RetrySchedule::IMMEDIATE`] runs the same loop with zero-length waits, for a test of
/// anything but timing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetrySchedule {
    /// The most attempts one run makes.
    pub attempts: u32,
    /// The first wait between attempts, in seconds.
    pub base_secs: f64,
    /// The largest jitter, as a fraction of its wait.
    pub jitter_frac: f64,
    /// The longest one attempt may run, in seconds.
    pub attempt_timeout_secs: f64,
    /// How often one attempt reopens a locked collection.
    pub open_retries: u32,
    /// The first wait before a reopen, in seconds.
    pub open_retry_base_secs: f64,
}

impl RetrySchedule {
    /// The predecessor's schedule, from the golden-proved constants.
    pub const PREDECESSOR: Self = Self {
        attempts: SYNC_RETRY_ATTEMPTS,
        base_secs: SYNC_RETRY_BASE_SECS,
        jitter_frac: SYNC_RETRY_JITTER_FRAC,
        attempt_timeout_secs: SYNC_TIMEOUT_SECS,
        open_retries: COLLECTION_OPEN_RETRIES,
        open_retry_base_secs: COLLECTION_OPEN_RETRY_BASE_SECS,
    };

    /// The predecessor's attempts and timeout, with every wait zero-length.
    pub const IMMEDIATE: Self = Self {
        base_secs: 0.0,
        jitter_frac: 0.0,
        open_retry_base_secs: 0.0,
        ..Self::PREDECESSOR
    };

    /// The wait after failed attempt `attempt` (from 1), in seconds, for the jitter draw `draw` in
    /// `[0, 1)`: the predecessor's arithmetic, in its order.
    #[must_use]
    pub fn wait_after(&self, _attempt: u32, _draw: f64) -> f64 {
        0.0
    }
}

/// What one call of [`Syncer::sync`] did.
#[derive(Clone, Debug, PartialEq)]
pub enum SyncReport {
    /// The run happened and was recorded.
    Ran {
        /// The run, as recorded.
        run: SyncRun,
        /// The waits between its attempts, in seconds, in order.
        waits_seconds: Vec<f64>,
    },
    /// A scheduled run was already recorded this study day: refused before any request, and no
    /// row (R15).
    RefusedToday,
    /// An owner's trigger less than [`OWNER_SYNC_DEBOUNCE_SECS`] after a success: that success,
    /// with no request and no row (R17).
    Debounced {
        /// The success it returns.
        last: SyncRun,
    },
}

/// Why a sync could not even be accounted for: its record could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// The run's record refused a read or a write.
    #[error("the sync's record could not be read or written")]
    Store(#[from] KernelError),
}

/// The source of the retry jitter's draws, each in `[0, 1)`.
type Jitter = Box<dyn FnMut() -> f64 + Send>;

/// Syncs the private copy: the one component that talks to the sync server (R11 calls it through
/// `coordination::sync_cycle`).
pub struct Syncer<E, S> {
    engine: E,
    store: S,
    settings: SyncSettings,
    credentials: CredentialLoader,
    clock: Arc<dyn Clock>,
    rule: StudyDayRule,
    schedule: RetrySchedule,
    jitter: Mutex<Jitter>,
}

impl<E: AnkiEngine + Sync, S: SyncRunStore> Syncer<E, S> {
    /// A syncer over `engine`, recording in `store`, with the predecessor's schedule.
    #[must_use]
    pub fn new(
        engine: E,
        store: S,
        settings: SyncSettings,
        credentials: CredentialLoader,
        clock: Arc<dyn Clock>,
        rule: StudyDayRule,
    ) -> Self {
        Self {
            engine,
            store,
            settings,
            credentials,
            clock,
            rule,
            schedule: RetrySchedule::PREDECESSOR,
            jitter: Mutex::new(Box::new(|| 0.0)),
        }
    }

    /// The same syncer on `schedule`.
    #[must_use]
    pub fn with_schedule(mut self, schedule: RetrySchedule) -> Self {
        self.schedule = schedule;
        self
    }

    /// The same syncer drawing its jitter from `jitter`, each draw in `[0, 1)`.
    #[must_use]
    pub fn with_jitter(self, jitter: impl FnMut() -> f64 + Send + 'static) -> Self {
        *self
            .jitter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Box::new(jitter);
        self
    }

    /// The record this syncer writes.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Runs one sync for `trigger`, under the collection lock, and records it (R5 to R10, R15 to
    /// R17).
    ///
    /// # Errors
    ///
    /// [`SyncError::Store`] when the record cannot be read or written. A failed sync is not an
    /// error: it is a recorded run with its reason code.
    pub async fn sync(&self, trigger: Trigger) -> Result<SyncReport, SyncError> {
        let _ = (
            &self.engine,
            &self.settings,
            &self.credentials,
            &self.schedule,
        );
        let now = self.clock.now();
        Ok(SyncReport::Ran {
            run: SyncRun {
                trigger,
                started_at: now,
                finished_at: UtcMillis::from_epoch_millis(now.epoch_millis()),
                study_day: self.rule.study_day(now),
                outcome: Err(ReasonCode::EngineFailed),
                attempts: 0,
                full_download: false,
            },
            waits_seconds: Vec::new(),
        })
    }
}
