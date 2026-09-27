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

use std::future::Future;
use std::hash::{BuildHasher, RandomState};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use deck_streak_kernel::{Clock, CredentialLoader, KernelError, StudyDayRule, UtcMillis};

use crate::engine::{AnkiEngine, EngineError, SyncLogin, SyncOutcome};
use crate::lock::CollectionLock;
use crate::settings::{SYNC_PASSWORD, SYNC_USERNAME, SyncSettings};
use crate::sync_runs::{ReasonCode, SyncRun, SyncRunStore, Trigger};

/// The most attempts one run makes (`constants.py:SYNC_RETRY_ATTEMPTS`).
pub const SYNC_RETRY_ATTEMPTS: u32 = 3;
/// The first wait between attempts, in seconds, doubled after each (`SYNC_RETRY_BASE_SECS`).
pub const SYNC_RETRY_BASE_SECS: f64 = 2.0;
/// The largest jitter, as a fraction of the wait it is added to (`SYNC_RETRY_JITTER_FRAC`).
pub const SYNC_RETRY_JITTER_FRAC: f64 = 0.25;
/// The longest one attempt may run, in seconds (`SYNC_TIMEOUT_SECS`).
pub const SYNC_TIMEOUT_SECS: f64 = 300.0;
/// How often one attempt reopens a locked collection (`COLLECTION_OPEN_RETRIES`).
pub const COLLECTION_OPEN_RETRIES: u32 = 3;
/// The first wait before a reopen, in seconds, doubled after each
/// (`COLLECTION_OPEN_RETRY_BASE_SECS`).
pub const COLLECTION_OPEN_RETRY_BASE_SECS: f64 = 0.25;
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
    pub fn wait_after(&self, attempt: u32, draw: f64) -> f64 {
        let wait = self.base_secs * doubled(attempt.saturating_sub(1));
        wait + wait * self.jitter_frac * draw
    }

    /// The wait before reopen `reopen` (from 0) of a locked collection, in seconds.
    #[must_use]
    pub fn reopen_wait(&self, reopen: u32) -> f64 {
        self.open_retry_base_secs * doubled(reopen)
    }
}

/// `2^power` as the predecessor's `2 ** power` makes it: exact for every power a schedule uses.
fn doubled(power: u32) -> f64 {
    2_f64.powi(i32::try_from(power).unwrap_or(i32::MAX))
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
            jitter: Mutex::new(random_jitter()),
        }
        .warned()
    }

    /// Logs R13's WARN once, when the syncer is built at the service's start.
    fn warned(self) -> Self {
        self.settings.warn_if_cleartext();
        self
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
        *self.jitter.lock().unwrap_or_else(PoisonError::into_inner) = Box::new(jitter);
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
        // Every guard reads the record under the lock, so a run that waited for another reads
        // what that one recorded.
        let Ok(held) = CollectionLock::new(self.settings.lock_path())
            .exclusive()
            .await
        else {
            let started = self.clock.now();
            return self
                .recorded(
                    trigger,
                    started,
                    Err(ReasonCode::OpenFailed),
                    0,
                    false,
                    Vec::new(),
                )
                .await;
        };
        let started = self.clock.now();
        let report = match trigger {
            Trigger::Scheduled
                if self
                    .store
                    .scheduled_run_on(self.rule.study_day(started))
                    .await? =>
            {
                SyncReport::RefusedToday
            }
            Trigger::Owner => match self.recent_success(started).await? {
                Some(last) => SyncReport::Debounced { last },
                None => self.run(trigger, started).await?,
            },
            Trigger::Scheduled => self.run(trigger, started).await?,
        };
        // R7: an explicit unlock before the lock file closes. A failed unlock is released by the
        // close that follows it.
        let _ = held.release();
        Ok(report)
    }

    /// The last success, when it finished less than [`OWNER_SYNC_DEBOUNCE_SECS`] before `now`.
    async fn recent_success(&self, now: UtcMillis) -> Result<Option<SyncRun>, SyncError> {
        let debounce = OWNER_SYNC_DEBOUNCE_SECS * 1000;
        Ok(self.store.last_success().await?.filter(|last| {
            now.epoch_millis()
                .saturating_sub(last.finished_at.epoch_millis())
                < debounce
        }))
    }

    /// One run: the login, the attempts, and the record.
    async fn run(&self, trigger: Trigger, started: UtcMillis) -> Result<SyncReport, SyncError> {
        match self.login() {
            Ok(login) => {
                let (outcome, attempts, full_download, waits) = self.attempts(&login).await;
                self.recorded(trigger, started, outcome, attempts, full_download, waits)
                    .await
            }
            Err(reason) => {
                self.recorded(trigger, started, Err(reason), 0, false, Vec::new())
                    .await
            }
        }
    }

    /// Records the run and reports it. A failure is logged by its reason code alone (R9).
    async fn recorded(
        &self,
        trigger: Trigger,
        started: UtcMillis,
        outcome: Result<(), ReasonCode>,
        attempts: u32,
        full_download: bool,
        waits_seconds: Vec<f64>,
    ) -> Result<SyncReport, SyncError> {
        if let Err(reason) = outcome {
            tracing::warn!(reason = reason.as_str(), attempts, "the sync failed");
        }
        let run = SyncRun {
            trigger,
            started_at: started,
            finished_at: self.clock.now(),
            study_day: self.rule.study_day(started),
            outcome,
            attempts,
            full_download,
        };
        self.store.record(&run).await?;
        Ok(SyncReport::Ran { run, waits_seconds })
    }

    /// The login, from the two credentials the kernel's loader reads (R5).
    fn login(&self) -> Result<SyncLogin, ReasonCode> {
        let missing = |_| ReasonCode::MissingCredentials;
        let username = self.credentials.load(SYNC_USERNAME).map_err(missing)?;
        let password = self.credentials.load(SYNC_PASSWORD).map_err(missing)?;
        Ok(SyncLogin::new(
            self.settings.endpoint().as_str(),
            username.expose(),
            password.expose(),
        ))
    }

    /// The predecessor's retry loop (R8): the outcome, the attempts made, whether the copy was
    /// replaced by a full download, and the waits between attempts.
    async fn attempts(&self, login: &SyncLogin) -> (Result<(), ReasonCode>, u32, bool, Vec<f64>) {
        let limit = Duration::from_secs_f64(self.schedule.attempt_timeout_secs);
        let mut waits = Vec::new();
        let mut attempt = 0;
        loop {
            attempt += 1;
            let result = tokio::time::timeout(limit, self.attempt(login))
                .await
                .unwrap_or(Err(ReasonCode::SyncTimeout));
            match result {
                Ok(full_download) => return (Ok(()), attempt, full_download, waits),
                Err(reason) if attempt >= self.schedule.attempts => {
                    return (Err(reason), attempt, false, waits);
                }
                Err(_) => {
                    let wait = self.schedule.wait_after(attempt, self.draw());
                    waits.push(wait);
                    tokio::time::sleep(Duration::from_secs_f64(wait)).await;
                }
            }
        }
    }

    /// One attempt (R6): a normal sync; a full download when the server demands a full sync and
    /// holds a collection; `full_upload_required` when it holds none. Nothing is ever uploaded.
    async fn attempt(&self, login: &SyncLogin) -> Result<bool, ReasonCode> {
        let copy = self.settings.copy_path();
        match self
            .reopening(|| self.engine.normal_sync(&copy, login))
            .await?
        {
            SyncOutcome::NoChanges | SyncOutcome::Synced => Ok(false),
            SyncOutcome::FullSyncRequired { download_ok: true } => {
                self.reopening(|| self.engine.full_download(&copy, login))
                    .await?;
                Ok(true)
            }
            SyncOutcome::FullSyncRequired { download_ok: false } => {
                Err(ReasonCode::FullUploadRequired)
            }
        }
    }

    /// Runs `call`, reopening a locked collection up to the schedule's reopens (R8); every other
    /// failure is its reason code at once.
    async fn reopening<T, F, Call>(&self, mut call: Call) -> Result<T, ReasonCode>
    where
        Call: FnMut() -> F,
        F: Future<Output = Result<T, EngineError>>,
    {
        let mut reopen = 0;
        loop {
            match call().await {
                Ok(value) => return Ok(value),
                Err(EngineError::CollectionLocked) if reopen < self.schedule.open_retries => {
                    let wait = self.schedule.reopen_wait(reopen);
                    tokio::time::sleep(Duration::from_secs_f64(wait)).await;
                    reopen += 1;
                }
                Err(error) => return Err(reason_for(error)),
            }
        }
    }

    /// The next jitter draw.
    fn draw(&self) -> f64 {
        (self.jitter.lock().unwrap_or_else(PoisonError::into_inner))()
    }
}

/// The reason code an engine failure is recorded as (R9).
const fn reason_for(error: EngineError) -> ReasonCode {
    match error {
        EngineError::CollectionLocked => ReasonCode::CollectionLocked,
        EngineError::OpenFailed => ReasonCode::OpenFailed,
        EngineError::AuthRejected => ReasonCode::AuthRejected,
        EngineError::NetworkUnreachable => ReasonCode::NetworkUnreachable,
        EngineError::ServerError => ReasonCode::ServerError,
        EngineError::Timeout => ReasonCode::SyncTimeout,
        EngineError::EngineFailed => ReasonCode::EngineFailed,
    }
}

/// Draws in `[0, 1)` from the standard library's randomly keyed hasher: jitter needs spread, not
/// secrecy, and the predecessor drew it the same way (`random.random`).
fn random_jitter() -> Jitter {
    let keys = RandomState::new();
    let mut counter = 0_u64;
    Box::new(move || {
        counter = counter.wrapping_add(1);
        let bits = keys.hash_one(counter) >> 11;
        #[allow(
            clippy::cast_precision_loss,
            reason = "a 53-bit integer converts to f64 exactly"
        )]
        let fraction = bits as f64 / (1_u64 << 53) as f64;
        fraction
    })
}
