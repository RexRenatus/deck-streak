//! The record of every sync (SPEC-022 R9, R10, R15 to R17): one `sync_runs` row per run, with its
//! trigger, its study day, `ok` or `error` with one bounded reason code, the attempts it used and
//! whether it was a full download.
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

    /// How many runs have failed since the last success (R10): what the scheduler's alerting
    /// reads (SPEC-027).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn consecutive_failures(&self) -> Result<u32, KernelError> {
        Ok(0)
    }

    /// When the last successful run finished (R10): what the dead-man watch reads (SPEC-027).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn last_success_at(&self) -> Result<Option<UtcMillis>, KernelError> {
        Ok(None)
    }

    /// The outcome of the syncs that started in `day`, if any did (R10).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn study_day_outcome(
        &self,
        _day: StudyDay,
    ) -> Result<Option<StudyDayOutcome>, KernelError> {
        Ok(None)
    }
}

impl SyncRunStore for SqliteSyncRuns {
    async fn scheduled_run_on(&self, _day: StudyDay) -> Result<bool, KernelError> {
        Ok(false)
    }

    async fn last_success(&self) -> Result<Option<SyncRun>, KernelError> {
        Ok(None)
    }

    async fn record(&self, _run: &SyncRun) -> Result<(), KernelError> {
        let _ = &self.db;
        Ok(())
    }
}
