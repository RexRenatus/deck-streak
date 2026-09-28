//! Wiring the contexts' adapters for the roles: the one database every role opens (SPEC-025 R11;
//! ADR-008, ADR-025), and the bot's two ports that another context answers (SPEC-026 R10, R11).
//!
//! The database is `deck_streak.db` in the directory systemd gives the units (`StateDirectory=`,
//! passed as `$STATE_DIRECTORY`). Every role opens it through [`open_database`], because sqlx's
//! `SQLite` migrator takes no lock: two roles starting at once could both find a migration
//! unapplied and both apply it, or collide switching a fresh file to WAL, and one of them would
//! fail to start. [`open_database`] holds an exclusive lock on a file beside the database while
//! the kernel's `Db::open` sets the pragmas and applies the migrations, so a second role waits and
//! then finds every migration applied. The race is closed, not made rarer
//! (`docs/schematics/service-lifecycle.md`).
//!
//! The bot's transport counts what it sent; [`TransportMarker`] hands those counts to
//! coordination's `DeliveryMarker` (SPEC-027 R6). The bot's `/sync` asks an [`OwnerSync`];
//! [`OwnerSyncCycle`] answers it with ingest's sync and coordination's cycle, which the bot cannot
//! name (docs/CONTEXT-MAP.md).

use std::fs::{File, OpenOptions};
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use deck_streak_bot::{OwnerSync, Scores, SyncAnswer, SyncOutcome, SyncRefusal, Transport};
use deck_streak_coordination::delivery::{DeliveryCounts, DeliveryMarker};
use deck_streak_coordination::obligations::Obligations;
use deck_streak_coordination::sync_cycle::{
    CycleError, CycleParts, CycleReport, Recompute, sync_cycle,
};
use deck_streak_ingest::engine::RslibEngine;
use deck_streak_ingest::gate::ChangeGate;
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::settings::{ScopeSettings, SyncSettings};
use deck_streak_ingest::sync::{SyncReport, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    Clock, CredentialLoader, CredentialsDirectory, Db, Environment, KernelError, Offload, Redactor,
    Setting, SettingsError, StudyDayRule, SystemClock,
};

/// The directory systemd gives a unit for its state (`StateDirectory=`), where the database lives.
pub const STATE_DIRECTORY: &str = "STATE_DIRECTORY";
/// The database's file name in the state directory (ADR-008; the deployment schematic).
pub const DATABASE_FILE: &str = "deck_streak.db";
/// The lock file a role holds while it opens and migrates the database: a file of its own beside
/// the database, never the database itself, because closing any descriptor of the database file
/// drops every POSIX lock the process holds on it, `SQLite`'s included.
pub const OPEN_LOCK_FILE: &str = "deck_streak.db-open.lock";

/// The directory the database lives in: an absolute path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDirectory(PathBuf);

impl StateDirectory {
    /// The directory at `path`, or `None` when the path is not absolute.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Option<Self> {
        let path = path.into();
        path.is_absolute().then_some(Self(path))
    }

    /// The directory [`STATE_DIRECTORY`] names.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Missing`] when it is unset or blank, and [`SettingsError::Malformed`] when
    /// it is not an absolute path.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        env.required(STATE_DIRECTORY)
    }

    /// The directory's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// The database's path.
    #[must_use]
    pub fn database(&self) -> PathBuf {
        self.0.join(DATABASE_FILE)
    }
}

impl Setting for StateDirectory {
    const SHAPE: &'static str = "an absolute directory path";

    fn parse(text: &str) -> Option<Self> {
        Self::new(text)
    }
}

/// Why the database could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum WiringError {
    /// The open lock could not be taken or released; the source says why.
    #[error("the database's open lock could not be taken or released")]
    OpenLock(#[source] io::Error),
    /// The kernel refused to open or migrate the database; the source says why.
    #[error(transparent)]
    Kernel(#[from] KernelError),
}

/// Opens the database in `state` and applies every migration, holding the open lock throughout,
/// so no two roles ever migrate the same file at once. The wait for the lock runs on `offload`, off
/// the async runtime, and a wait of a second or more is logged as a slow operation.
///
/// # Errors
///
/// [`WiringError::OpenLock`] when the lock file cannot be opened, locked or unlocked, and
/// [`WiringError::Kernel`] when the database cannot be opened or migrated.
pub async fn open_database(offload: &Offload, state: &StateDirectory) -> Result<Db, WiringError> {
    let lock_path = state.path().join(OPEN_LOCK_FILE);
    let lock = offload
        .run("take the database's open lock", move || {
            take_open_lock(&lock_path)
        })
        .await?
        .map_err(WiringError::OpenLock)?;
    let opened = Db::open(&state.database()).await;
    // Released explicitly, never only by closing: a descriptor a child process inherited would
    // keep a lock that only a close releases held.
    let released = lock.unlock();
    drop(lock);
    let database = opened?;
    released.map_err(WiringError::OpenLock)?;
    Ok(database)
}

/// Opens (creating it if missing) the lock file at `path` and waits for its exclusive lock.
fn take_open_lock(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.lock()?;
    Ok(file)
}

/// The bot transport's counts, as coordination's `DeliveryMarker` (SPEC-026 R10, SPEC-027 R6): a
/// message attempted and never delivered is a failed send, and one never attempted is a job that
/// did not engage the notifier.
#[derive(Clone, Debug)]
pub struct TransportMarker(Arc<Transport>);

impl TransportMarker {
    /// The marker over `transport`'s counts.
    #[must_use]
    pub const fn new(transport: Arc<Transport>) -> Self {
        Self(transport)
    }
}

impl DeliveryMarker for TransportMarker {
    fn counts(&self) -> DeliveryCounts {
        let _ = &self.0;
        DeliveryCounts::default()
    }
}

/// The owner's `/sync` (SPEC-026 R11; ADR-037; SPEC-023 R8): the owner's rescore is marked
/// pending first, so the next cycle recomputes whatever it finds even when this one cannot run;
/// then one sync cycle runs with the trigger `owner`, over the sync's settings and credentials read
/// when it runs, as the `sync` job reads them. The debounce of an owner's trigger is the syncer's
/// (SPEC-022 R17).
#[derive(Debug)]
pub struct OwnerSyncCycle {
    env: Environment,
    redactor: Redactor,
    db: Db,
    offload: Offload,
    rule: StudyDayRule,
}

impl OwnerSyncCycle {
    /// The owner's sync over `db`, reading its settings from `env` and its credentials through a
    /// loader that registers them with `redactor`, blocking work on `offload`, and study days by
    /// `rule`.
    #[must_use]
    pub const fn new(
        env: Environment,
        redactor: Redactor,
        db: Db,
        offload: Offload,
        rule: StudyDayRule,
    ) -> Self {
        Self {
            env,
            redactor,
            db,
            offload,
            rule,
        }
    }

    async fn run(&self) -> Result<SyncAnswer, SyncRefusal> {
        let _ = (
            &self.env,
            &self.redactor,
            &self.db,
            &self.offload,
            self.rule,
        );
        let _ = (
            ChangeGate::new,
            SyncSettings::from_env,
            CredentialsDirectory::from_env,
            ScopeSettings::from_env,
            CollectionReader::new,
            Syncer::<RslibEngine, SqliteSyncRuns>::new,
            CycleParts::<RslibEngine>::new,
            Obligations::new,
            Trigger::Owner,
        );
        let _ = (
            SystemClock,
            CredentialLoader::new,
            refused,
            cycle_reason,
            answer_of,
        );
        let _ = sync_cycle::<RslibEngine>;
        let _: Option<Arc<dyn Clock>> = None;
        Err(SyncRefusal {
            reason: "sync_settings_refused",
        })
    }
}

impl OwnerSync for OwnerSyncCycle {
    fn sync_now(&self) -> impl Future<Output = Result<SyncAnswer, SyncRefusal>> + Send {
        self.run()
    }
}

/// The refusal `reason`, logged with its cause: the cause names a setting or a step, never a
/// value.
fn refused(reason: &'static str, error: &dyn std::fmt::Display) -> SyncRefusal {
    tracing::error!(reason, %error, "the owner's sync could not run");
    SyncRefusal { reason }
}

/// The reason code of a cycle that could not run to its end: the `sync` job's own
/// (`role_job.rs`). A failed sync is not one: it is a recorded run, answered as such.
const fn cycle_reason(error: &CycleError) -> &'static str {
    let _ = error;
    "sync_record_failed"
}

/// What the owner is told of `report`: the sync, then the recompute.
fn answer_of(report: &CycleReport) -> SyncAnswer {
    let _ = (report, Recompute::Skipped, Scores::Unchanged);
    let _: Option<&SyncReport> = None;
    SyncAnswer {
        sync: SyncOutcome::Synced,
        scores: Scores::Recomputed,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use deck_streak_bot::{ApiUrl, OwnerSync, Scores, Sent, SyncOutcome, SyncRefusal, Transport};
    use deck_streak_coordination::delivery::{DeliveryCounts, DeliveryMarker};
    use deck_streak_coordination::sync_cycle::{CycleError, CycleReport, Recompute};
    use deck_streak_ingest::gate::RunReason;
    use deck_streak_ingest::state::SqliteIngestState;
    use deck_streak_ingest::sync::SyncReport;
    use deck_streak_ingest::sync_runs::{ReasonCode, SyncRun, Trigger};
    use deck_streak_kernel::{
        CredentialLoader, CredentialsDirectory, Db, Environment, KernelError, Offload,
        OffloadWorkers, Redactor, StudyDay, StudyDayRule, SystemClock, UtcMillis,
    };

    use super::{OwnerSyncCycle, TransportMarker, answer_of, cycle_reason};

    /// A run of the given outcome, with synthetic instants.
    fn run(outcome: Result<(), ReasonCode>) -> SyncRun {
        SyncRun {
            trigger: Trigger::Owner,
            started_at: UtcMillis::from_epoch_millis(1_000),
            finished_at: UtcMillis::from_epoch_millis(2_000),
            study_day: StudyDay::from_epoch_day(20_000),
            outcome,
            attempts: 1,
            full_download: false,
        }
    }

    #[test]
    fn the_owner_is_told_what_the_sync_and_the_recompute_did() {
        let ran = Recompute::Ran {
            reason: RunReason::RescorePending,
            reviews: 3,
            cards: 5,
        };
        let cases = [
            (
                SyncReport::Ran {
                    run: run(Ok(())),
                    waits_seconds: Vec::new(),
                },
                ran.clone(),
                SyncOutcome::Synced,
                Scores::Recomputed,
            ),
            (
                SyncReport::Ran {
                    run: run(Err(ReasonCode::ServerError)),
                    waits_seconds: vec![30.0],
                },
                ran.clone(),
                SyncOutcome::Failed {
                    reason: "server_error".to_owned(),
                },
                Scores::Recomputed,
            ),
            (
                SyncReport::Debounced { last: run(Ok(())) },
                Recompute::Skipped,
                SyncOutcome::Reused,
                Scores::Unchanged,
            ),
            (
                SyncReport::RefusedToday,
                ran,
                SyncOutcome::NotRun {
                    reason: "refused_today".to_owned(),
                },
                Scores::Recomputed,
            ),
        ];
        for (sync, recompute, told, scores) in cases {
            let answer = answer_of(&CycleReport { sync, recompute });
            assert_eq!(answer.sync, told);
            assert_eq!(answer.scores, scores);
        }
    }

    #[test]
    fn a_cycle_that_cannot_finish_is_refused_with_its_steps_reason_code() {
        let cause = || KernelError::LoggingInstalled;
        assert_eq!(
            cycle_reason(&CycleError::History(cause())),
            "sync_record_failed"
        );
        assert_eq!(
            cycle_reason(&CycleError::Obligations(cause())),
            "obligations_unreadable"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_owners_sync_marks_the_rescore_before_it_reads_its_settings() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let db = Db::open(&directory.path().join("deck_streak.db"))
            .await
            .expect("the database opens");
        let workers = OffloadWorkers::new(1).expect("one worker is in range");
        let cycle = OwnerSyncCycle::new(
            Environment::from_vars(Vec::<(String, String)>::new()),
            Redactor::new(),
            db.clone(),
            Offload::new(workers, Arc::new(SystemClock)),
            StudyDayRule::default(),
        );
        let answer = cycle.sync_now().await;
        assert_eq!(
            answer,
            Err(SyncRefusal {
                reason: "sync_settings_refused"
            }),
            "no sync endpoint is set"
        );
        let state = SqliteIngestState::new(db.clone())
            .load()
            .await
            .expect("the state reads");
        assert!(
            state.rescore_pending,
            "the owner's rescore waits for the next cycle"
        );
        db.close().await;
    }

    #[tokio::test]
    async fn the_marker_reads_the_transports_counts() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            directory.path().join("telegram-bot-token"),
            "synthetic-token\n",
        )
        .expect("a credential");
        let token = CredentialLoader::new(
            CredentialsDirectory::new(directory.path()).expect("an absolute path"),
            Redactor::new(),
        )
        .load("telegram-bot-token")
        .expect("the token loads");
        // A closed loopback port: every attempt fails at once.
        let api = ApiUrl::new("http://127.0.0.1:9").expect("a loopback URL");
        let transport = Arc::new(Transport::new(&api, &token).expect("the transport builds"));
        let marker = TransportMarker::new(Arc::clone(&transport));
        assert_eq!(marker.counts(), DeliveryCounts::default());
        assert_eq!(
            transport.send_html(4242, "refused", None).await,
            Sent::Failed
        );
        assert_eq!(
            marker.counts(),
            DeliveryCounts {
                attempted: 1,
                delivered: 0
            }
        );
    }
}
