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
//! coordination's `DeliveryMarker` (SPEC-027 R6). The owner's cycle, [`OwnerSyncCycle`], is run by
//! the sync job when a request is stored (SPEC-059); the bot's `/sync` port only requests it
//! (`sync_request.rs`).
//!
//! Every cycle recomputes through the fold (SPEC-071 R15, R19). A role that runs cycles loads a
//! [`RecomputeSetup`] once, at its start: the owner's courses, refused when they disagree with the
//! readings taxonomy, their digest recorded with the settings generation, and the fold with every
//! step registered in its phase by [`recompute_fold`]. The setup hands the reader its courses and
//! each cycle its fold.
//! The notification router's bot transport is the bot's `OwnerChat`, joined to the router here by
//! [`router`] (SPEC-041 R13), and the owner's `/sync` flushes that router after a sync that
//! succeeds (R7).

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_bot::{OwnerChat, Scores, SyncAnswer, SyncOutcome, SyncRefusal, Transport};
use deck_streak_coordination::courses::{CoursesDisagree, agree};
use deck_streak_coordination::delivery::{DeliveryCounts, DeliveryMarker};
use deck_streak_coordination::obligations::Obligations;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::day_bonuses::DayBonusesStep;
use deck_streak_coordination::recompute::xp::XpStep;
use deck_streak_coordination::recompute::{Fold, FoldError, Phase};
use deck_streak_coordination::sync_cycle::{
    CycleError, CycleParts, CycleReport, Recompute, sync_cycle,
};
use deck_streak_identity::Owner;
use deck_streak_ingest::engine::{AnkiEngine, RslibEngine};
use deck_streak_ingest::gate::ChangeGate;
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::settings::{ScopeSettings, SyncSettings};
use deck_streak_ingest::sync::{SyncReport, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    Clock, Courses, CoursesError, CredentialLoader, CredentialsDirectory, Db, Environment,
    KernelError, Offload, Redactor, Setting, SettingsError, StudyDayRule, SystemClock,
};
use deck_streak_notifications::{Policy, Router};
use deck_streak_readings::taxonomy::{Taxonomy, TaxonomyError, TaxonomyPath};

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

/// The recompute's fold, with every step registered in its phase (SPEC-071 R19): phase 1's
/// analytics step, counting leeches by `analytics`. A later SPEC registers its step here, in its
/// own phase, without touching the fold.
///
/// # Errors
///
/// [`FoldError::OutsidePhase`] when a step is registered outside its phase.
pub fn recompute_fold(analytics: AnalyticsSettings) -> Result<Fold, FoldError> {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(analytics)),
    )?;
    fold.register(Phase::BaseXp, Box::new(XpStep))?;
    fold.register(Phase::DerivedBonuses, Box::new(DayBonusesStep))?;
    Ok(fold)
}

/// Why a role's recompute cannot start. Each names a setting or a step, never a value.
#[derive(Debug, thiserror::Error)]
pub enum RecomputeError {
    /// The courses file refused start (SPEC-071 R1).
    #[error(transparent)]
    Courses(#[from] CoursesError),
    /// A setting refused start: the readings taxonomy's path or the leech threshold.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The readings taxonomy could not be loaded.
    #[error(transparent)]
    Taxonomy(#[from] TaxonomyError),
    /// The courses file and the readings taxonomy map a language deck to two codes (R3).
    #[error(transparent)]
    Disagree(#[from] CoursesDisagree),
    /// The courses' digest could not be recorded with the settings generation (R4).
    #[error("the courses' digest could not be recorded")]
    Digest(#[source] KernelError),
    /// A step was registered outside its phase (R19).
    #[error(transparent)]
    Fold(#[from] FoldError),
}

/// What every cycle of a role recomputes with, loaded once at the role's start (SPEC-071 R1, R3,
/// R4, R15): the owner's courses, and the fold.
#[derive(Clone, Debug)]
pub struct RecomputeSetup {
    courses: Courses,
    fold: Arc<Fold>,
}

impl RecomputeSetup {
    /// Loads the owner's courses from `env` (R1), refuses start when they disagree with the
    /// readings taxonomy (R3), records their digest with the settings generation in `db`, which
    /// bumps the generation when they changed (R4), and builds the fold (R19).
    ///
    /// # Errors
    ///
    /// Every refusal of [`RecomputeError`].
    pub async fn load(env: &Environment, db: &Db) -> Result<Self, RecomputeError> {
        let courses = Courses::load(env)?;
        let taxonomy = match TaxonomyPath::from_env(env)? {
            Some(path) => Some(Taxonomy::load(path.as_path())?),
            None => None,
        };
        agree(&courses, taxonomy.as_ref())?;
        db.record_courses_digest(courses.digest())
            .await
            .map_err(RecomputeError::Digest)?;
        let fold = recompute_fold(AnalyticsSettings::from_env(env)?)?;
        Ok(Self {
            courses,
            fold: Arc::new(fold),
        })
    }

    /// The reader of the copy `settings` names, within `scope`, giving each card its course (R2).
    #[must_use]
    pub fn reader(
        &self,
        settings: &SyncSettings,
        scope: ScopeSettings,
        offload: Offload,
    ) -> CollectionReader {
        CollectionReader::new(settings, scope, offload).with_courses(self.courses.clone())
    }

    /// `parts`, recomputing through the fold over `db`, with study days decided by `rule` (R15).
    #[must_use]
    pub fn cycle<E: AnkiEngine + Sync>(
        &self,
        parts: CycleParts<E>,
        db: Db,
        rule: StudyDayRule,
    ) -> CycleParts<E> {
        let digest = self.courses.digest().map(str::to_owned);
        parts.with_fold(Arc::clone(&self.fold), db, rule, digest)
    }
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
        let counts = self.0.counts();
        DeliveryCounts {
            attempted: counts.attempted,
            delivered: counts.delivered,
        }
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
    recompute: RecomputeSetup,
}

impl OwnerSyncCycle {
    /// The owner's sync over `db`, reading its settings from `env` and its credentials through a
    /// loader that registers them with `redactor`, blocking work on `offload`, study days by
    /// `rule`, and recomputing through `recompute`, which the role loaded at its start.
    #[must_use]
    pub const fn new(
        env: Environment,
        redactor: Redactor,
        db: Db,
        offload: Offload,
        rule: StudyDayRule,
        recompute: RecomputeSetup,
    ) -> Self {
        Self {
            env,
            redactor,
            db,
            offload,
            rule,
            recompute,
        }
    }

    /// Runs the owner's cycle once, in the process that calls it: the sync job's, never the
    /// bot's (SPEC-059).
    ///
    /// # Errors
    ///
    /// The refusal, with its reason code, when the cycle could not run to its end.
    pub async fn run(&self) -> Result<SyncAnswer, SyncRefusal> {
        let clock = Arc::new(SystemClock);
        let gate = ChangeGate::new(self.db.clone(), self.rule, clock.clone());
        gate.state()
            .request_rescore(clock.now())
            .await
            .map_err(|error| refused("rescore_unrecorded", &error))?;
        let settings = SyncSettings::from_env(&self.env)
            .map_err(|error| refused("sync_settings_refused", &error))?;
        let directory = CredentialsDirectory::from_env(&self.env)
            .map_err(|error| refused("credentials_directory_refused", &error))?;
        let scope = ScopeSettings::from_env(&self.env)
            .map_err(|error| refused("scope_settings_refused", &error))?;
        let reader = self
            .recompute
            .reader(&settings, scope, self.offload.clone());
        let syncer = Syncer::new(
            RslibEngine,
            SqliteSyncRuns::new(self.db.clone()),
            settings,
            CredentialLoader::new(directory, self.redactor.clone()),
            clock.clone(),
            self.rule,
        );
        let parts = self.recompute.cycle(
            CycleParts::new(syncer, reader, gate, Obligations::new(), clock),
            self.db.clone(),
            self.rule,
        );
        let report = sync_cycle(&parts, Trigger::Owner)
            .await
            .map_err(|error| refused(cycle_reason(&error), &error))?;
        Ok(answer_of(&report))
    }
}

/// The notification router of `policy` over `db`, reading study days by `rule` on the system's
/// clock, that delivers the bot's occasions to the `owner`'s chat through the bot's `transport`
/// (SPEC-041 R13).
#[must_use]
pub fn router(
    policy: Policy,
    db: Db,
    rule: StudyDayRule,
    transport: Arc<Transport>,
    owner: Owner,
) -> Router {
    Router::new(Arc::new(policy), db, Arc::new(SystemClock), rule)
        .with_bot(Arc::new(OwnerChat::new(transport, owner)))
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
    match error {
        CycleError::History(_) | CycleError::Sync(_) => "sync_record_failed",
        CycleError::Obligations(_) => "obligations_unreadable",
        CycleError::Gate(_) | CycleError::Window(_) | CycleError::Recompute(_) => {
            "recompute_failed"
        }
    }
}

/// What the owner is told of `report`: the sync, then the recompute.
fn answer_of(report: &CycleReport) -> SyncAnswer {
    let sync = match &report.sync {
        SyncReport::Ran { run, .. } => match run.outcome {
            Ok(()) => SyncOutcome::Synced,
            Err(reason) => SyncOutcome::Failed {
                reason: reason.as_str().to_owned(),
            },
        },
        SyncReport::Debounced { .. } => SyncOutcome::Reused,
        SyncReport::RefusedToday => SyncOutcome::NotRun {
            reason: "refused_today".to_owned(),
        },
    };
    let scores = match report.recompute {
        Recompute::Ran { .. } => Scores::Recomputed,
        Recompute::Skipped => Scores::Unchanged,
    };
    SyncAnswer { sync, scores }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use deck_streak_bot::{ApiUrl, Scores, Sent, SyncOutcome, SyncRefusal, Transport};
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

    use deck_streak_analytics::settings::AnalyticsSettings;
    use deck_streak_coordination::recompute::Phase;
    use deck_streak_coordination::recompute::analytics_step::ANALYTICS_STEP;
    use deck_streak_coordination::recompute::day_bonuses::DAY_BONUSES_STEP;
    use deck_streak_coordination::recompute::xp::XP_STEP;

    use super::{
        OwnerSyncCycle, RecomputeSetup, TransportMarker, answer_of, cycle_reason, recompute_fold,
    };

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
        assert_eq!(
            cycle_reason(&CycleError::Recompute(cause())),
            "recompute_failed"
        );
    }

    #[test]
    fn the_recompute_fold_registers_the_analytics_and_xp_steps_in_their_phases() {
        let fold = recompute_fold(AnalyticsSettings::default()).expect("every step in its phase");
        assert_eq!(
            fold.steps(),
            [
                (Phase::RollupAndScore, ANALYTICS_STEP),
                (Phase::BaseXp, XP_STEP),
                (Phase::DerivedBonuses, DAY_BONUSES_STEP),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_owners_sync_marks_the_rescore_before_it_reads_its_settings() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let db = Db::open(&directory.path().join("deck_streak.db"))
            .await
            .expect("the database opens");
        let workers = OffloadWorkers::new(1).expect("one worker is in range");
        let env = Environment::from_vars(Vec::<(String, String)>::new());
        let recompute = RecomputeSetup::load(&env, &db)
            .await
            .expect("no courses and no taxonomy are configured");
        let cycle = OwnerSyncCycle::new(
            env,
            Redactor::new(),
            db.clone(),
            Offload::new(workers, Arc::new(SystemClock)),
            StudyDayRule::default(),
            recompute,
        );
        let answer = cycle.run().await;
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
