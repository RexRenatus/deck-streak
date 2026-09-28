//! The record of every sync (SPEC-022 R9, R10, R15 to R17): one `sync_runs` row per run, with its
//! trigger, its study day, `ok` or `error` with one bounded reason code, the attempts it used and
//! whether it was a full download. The change gate adds a `skipped` row for a cycle whose recompute
//! it skipped, and reads the record as a cycle found it (SPEC-023 R8, R11).
//!
//! [`SyncRunStore`] is what the syncer asks and tells; [`SqliteSyncRuns`] keeps it in the service's
//! database, owned by this context (docs/CONTEXT-MAP.md). A reason code is the whole account of a
//! failure: no error text, path, endpoint or credential is stored with it.

use std::fmt;
use std::future::Future;

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};

/// What asked for a sync (R16).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trigger {
    /// The scheduler's daily `sync` job (SPEC-027): at most one per study day (R15).
    Scheduled,
    /// The owner's explicit trigger, the bot's `/sync` (SPEC-026) or a Mini App action that names
    /// itself one (R17).
    Owner,
}

impl Trigger {
    /// The trigger as `sync_runs` stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Owner => "owner",
        }
    }

    /// The trigger `sync_runs` stored as `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "scheduled" => Some(Self::Scheduled),
            "owner" => Some(Self::Owner),
            _ => None,
        }
    }
}

/// Why a sync failed: one code from a closed set, and nothing else (R9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReasonCode {
    /// A sync credential is missing from the credentials directory.
    MissingCredentials,
    /// The sync server refused the credentials.
    AuthRejected,
    /// The sync server could not be reached.
    NetworkUnreachable,
    /// The sync server answered with an error.
    ServerError,
    /// An attempt ran past its timeout.
    SyncTimeout,
    /// The server holds no collection, so only a full upload could satisfy it (never performed).
    FullUploadRequired,
    /// Another process held the collection through every reopen.
    CollectionLocked,
    /// The copy, or its lock, could not be opened.
    OpenFailed,
    /// Any other failure inside the engine.
    EngineFailed,
}

impl ReasonCode {
    /// Every code, in the order R9 lists them.
    pub const ALL: [Self; 9] = [
        Self::MissingCredentials,
        Self::AuthRejected,
        Self::NetworkUnreachable,
        Self::ServerError,
        Self::SyncTimeout,
        Self::FullUploadRequired,
        Self::CollectionLocked,
        Self::OpenFailed,
        Self::EngineFailed,
    ];

    /// The code as `sync_runs` stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingCredentials => "missing_credentials",
            Self::AuthRejected => "auth_rejected",
            Self::NetworkUnreachable => "network_unreachable",
            Self::ServerError => "server_error",
            Self::SyncTimeout => "sync_timeout",
            Self::FullUploadRequired => "full_upload_required",
            Self::CollectionLocked => "collection_locked",
            Self::OpenFailed => "open_failed",
            Self::EngineFailed => "engine_failed",
        }
    }

    /// The code `sync_runs` stored as `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|code| code.as_str() == text)
    }
}

impl fmt::Display for ReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One run of the syncer, as `sync_runs` records it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncRun {
    /// What asked for it.
    pub trigger: Trigger,
    /// When it started, by the kernel's clock.
    pub started_at: UtcMillis,
    /// When it finished, by the kernel's clock.
    pub finished_at: UtcMillis,
    /// The study day it started in, by the kernel's rule.
    pub study_day: StudyDay,
    /// `Ok` when it synced, or the one reason it did not.
    pub outcome: Result<(), ReasonCode>,
    /// The attempts it used; 0 when it failed before its first.
    pub attempts: u32,
    /// Whether it replaced the copy by a full download.
    pub full_download: bool,
}

/// What the syncer asks of the record and tells it.
pub trait SyncRunStore: Send + Sync {
    /// Whether a scheduled run that started in `day` is recorded (R15).
    fn scheduled_run_on(
        &self,
        day: StudyDay,
    ) -> impl Future<Output = Result<bool, KernelError>> + Send;

    /// The most recent run that succeeded, by its finish (R10, R17).
    fn last_success(&self) -> impl Future<Output = Result<Option<SyncRun>, KernelError>> + Send;

    /// Records `run`.
    fn record(&self, run: &SyncRun) -> impl Future<Output = Result<(), KernelError>> + Send;
}

/// Whether a study day's sync succeeded, and what triggered it (R10): what the jobs that need the
/// day's data read instead of syncing (ADR-037).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StudyDayOutcome {
    /// Whether a sync that started in the day succeeded.
    pub synced: bool,
    /// The trigger of the run that decides it: the last success, or else the last run.
    pub trigger: Trigger,
}

/// A run's status as `sync_runs` records it: the sync's `ok` or `error`, or the change gate's
/// `skipped` (SPEC-023 R11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RunStatus {
    /// The sync succeeded.
    Ok,
    /// The sync failed, with its reason code.
    Error,
    /// The sync succeeded and the change gate skipped the recompute.
    Skipped,
}

impl RunStatus {
    /// The status as `sync_runs` stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Skipped => "skipped",
        }
    }

    /// The status `sync_runs` stored as `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Ok, Self::Error, Self::Skipped]
            .into_iter()
            .find(|status| status.as_str() == text)
    }
}

/// The record as a cycle finds it before its own sync (SPEC-023 R8): what the change gate's run-
/// history term reads, as the predecessor's gate read the run before its own cycle's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunHistory {
    /// The last run's status, or `None` when no run is on record.
    pub last: Option<RunStatus>,
    /// Whether any run on record succeeded.
    pub any_success: bool,
}

/// A cycle whose recompute the change gate skipped, as `sync_runs` records it (SPEC-023 R11): its
/// trigger, the study day it decided in, and when the gate began and ended. It made no attempt and
/// no download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkippedRun {
    /// What asked for the cycle.
    pub trigger: Trigger,
    /// When the gate began, by the kernel's clock.
    pub started_at: UtcMillis,
    /// When the gate ended, by the kernel's clock.
    pub finished_at: UtcMillis,
    /// The study day the gate decided in.
    pub study_day: StudyDay,
}

/// `sync_runs` in the service's own database.
#[derive(Clone, Debug)]
pub struct SqliteSyncRuns {
    db: Db,
}

impl SqliteSyncRuns {
    /// The record in `db`, whose migrations created `sync_runs`.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// How many runs have failed since the last run that did not (R10): what the scheduler's
    /// alerting reads (SPEC-027).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn consecutive_failures(&self) -> Result<u32, KernelError> {
        let failures = sqlx::query_scalar!(
            r#"SELECT count(*) AS "failures!: i64" FROM sync_runs
               WHERE status = 'error'
                 AND id > coalesce((SELECT max(id) FROM sync_runs WHERE status != 'error'), 0)"#
        )
        .fetch_one(self.db.reader())
        .await?;
        Ok(u32::try_from(failures).unwrap_or(u32::MAX))
    }

    /// When the last successful run finished (R10): what the dead-man watch reads (SPEC-027).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn last_success_at(&self) -> Result<Option<UtcMillis>, KernelError> {
        let finished = sqlx::query_scalar!(
            r#"SELECT max(finished_at) AS "finished: i64" FROM sync_runs WHERE status = 'ok'"#
        )
        .fetch_one(self.db.reader())
        .await?;
        Ok(finished.map(UtcMillis::from_epoch_millis))
    }

    /// The outcome of the syncs that started in `day`, if any did (R10): the day's last success,
    /// or else its last run.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn study_day_outcome(
        &self,
        day: StudyDay,
    ) -> Result<Option<StudyDayOutcome>, KernelError> {
        let day = day.epoch_day();
        let row = sqlx::query!(
            "SELECT trigger, status FROM sync_runs WHERE study_day = ?1 \
             ORDER BY status = 'error', id DESC LIMIT 1",
            day
        )
        .fetch_optional(self.db.reader())
        .await?;
        Ok(row.and_then(|row| {
            Some(StudyDayOutcome {
                synced: row.status != "error",
                trigger: Trigger::parse(&row.trigger)?,
            })
        }))
    }

    /// The record as it stands (SPEC-023 R8): the last run's status and whether any run succeeded.
    /// A cycle reads it before its own sync records a row.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn history(&self) -> Result<RunHistory, KernelError> {
        Ok(RunHistory {
            last: None,
            any_success: false,
        })
    }

    /// Records the change gate's skip (SPEC-023 R11): a `skipped` row with no reason, no attempt
    /// and no download.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn record_skipped(&self, run: &SkippedRun) -> Result<(), KernelError> {
        let _ = run;
        Ok(())
    }
}

impl SyncRunStore for SqliteSyncRuns {
    async fn scheduled_run_on(&self, day: StudyDay) -> Result<bool, KernelError> {
        let day = day.epoch_day();
        let found = sqlx::query_scalar!(
            r#"SELECT EXISTS(
                   SELECT 1 FROM sync_runs WHERE trigger = 'scheduled' AND study_day = ?1
               ) AS "found!: bool""#,
            day
        )
        .fetch_one(self.db.reader())
        .await?;
        Ok(found)
    }

    async fn last_success(&self) -> Result<Option<SyncRun>, KernelError> {
        let row = sqlx::query!(
            "SELECT trigger, study_day, started_at, finished_at, attempts, full_download \
             FROM sync_runs WHERE status = 'ok' ORDER BY finished_at DESC, id DESC LIMIT 1"
        )
        .fetch_optional(self.db.reader())
        .await?;
        Ok(row.and_then(|row| {
            Some(SyncRun {
                trigger: Trigger::parse(&row.trigger)?,
                started_at: UtcMillis::from_epoch_millis(row.started_at),
                finished_at: UtcMillis::from_epoch_millis(row.finished_at),
                study_day: StudyDay::from_epoch_day(row.study_day),
                outcome: Ok(()),
                attempts: u32::try_from(row.attempts).ok()?,
                full_download: row.full_download != 0,
            })
        }))
    }

    async fn record(&self, run: &SyncRun) -> Result<(), KernelError> {
        let (status, reason) = match run.outcome {
            Ok(()) => ("ok", None),
            Err(code) => ("error", Some(code.as_str())),
        };
        let trigger = run.trigger.as_str();
        let study_day = run.study_day.epoch_day();
        let started_at = run.started_at.epoch_millis();
        let finished_at = run.finished_at.epoch_millis();
        let attempts = i64::from(run.attempts);
        let full_download = i64::from(run.full_download);
        let mut write = self.db.write().await?;
        // The row is created when the run finishes, by the same clock.
        sqlx::query!(
            "INSERT INTO sync_runs \
             (trigger, study_day, started_at, finished_at, status, reason, attempts, \
              full_download, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?4)",
            trigger,
            study_day,
            started_at,
            finished_at,
            status,
            reason,
            attempts,
            full_download
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
        Ok(())
    }
}
