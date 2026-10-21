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
//! succeeds (R7). Every recompute cycle [`RecomputeSetup::cycle`] hands out holds a router with no
//! transport, which holds the celebrations it routes for those senders (SPEC-319).

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use deck_streak_agent::{
    CefrBand, DeckFuture, DeckGate, DeckScope, DeckVerdict, LiveBand, Subject, SubjectKind,
};
use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_bot::{OwnerChat, Scores, SyncAnswer, SyncOutcome, Transport};
use deck_streak_coordination::courses::{CoursesDisagree, agree};
use deck_streak_coordination::delivery::{DeliveryCounts, DeliveryMarker};
use deck_streak_coordination::inbox_capture::{InboxCaptures, LayoutInForce, RealFs};
use deck_streak_coordination::instruments::{
    BoxFuture, Frame, InstrumentListing, InstrumentRunner, InstrumentService, Instruments,
    OnDemandRefusal, ReadSource, StoredReport,
};
use deck_streak_coordination::obligations::Obligations;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::badges::BadgesStep;
use deck_streak_coordination::recompute::band_badges::BandBadgesStep;
use deck_streak_coordination::recompute::day_bonuses::DayBonusesStep;
use deck_streak_coordination::recompute::habit_badges::HabitBadgesStep;
use deck_streak_coordination::recompute::habits::HabitsStep;
use deck_streak_coordination::recompute::mint::MintStep;
use deck_streak_coordination::recompute::progress::ProgressStep;
use deck_streak_coordination::recompute::records::RecordsStep;
use deck_streak_coordination::recompute::streaks::{RelightDue, StreaksStep};
use deck_streak_coordination::recompute::writing::WritingStep;
use deck_streak_coordination::recompute::xp::XpStep;
use deck_streak_coordination::recompute::{Fold, FoldError, Phase};
use deck_streak_coordination::sync_cycle::{
    CycleError, CycleParts, CycleReport, Recompute, sync_cycle,
};
use deck_streak_curriculum::store::stored_bands;
use deck_streak_identity::Owner;
use deck_streak_identity::sync_seal::{SealError, SealSecret};
use deck_streak_ingest::engine::{AnkiEngine, RslibEngine};
use deck_streak_ingest::gate::{ChangeGate, GateError};
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::sensitive::{Admission, SqliteSensitiveDecks, admits};
use deck_streak_ingest::settings::{ScopeSettings, SyncSettings};
use deck_streak_ingest::state::RefusalReason;
use deck_streak_ingest::structure::StructureReads;
use deck_streak_ingest::sync::{SyncError, SyncReport, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_ingest::window::WindowError;
use deck_streak_insights::dark_fields::DarkFields;
use deck_streak_kernel::CredentialError;
use deck_streak_kernel::{
    Clock, Conventions, ConventionsError, Courses, CoursesError, CredentialLoader,
    CredentialsDirectory, Db, Environment, KernelError, Offload, PortFuture, Redactor, Setting,
    SettingsError, StudyDayRule, SystemClock,
};
use deck_streak_notifications::{Policy, Router};
use deck_streak_readings::taxonomy::{Taxonomy, TaxonomyError, TaxonomyPath};
use deck_streak_vault::config::{VAULT_ROOT, VaultRoot};

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

/// The vault inbox's captures for the `api` role's quick capture and the `bot` role's media
/// (SPEC-118 R4, R6, R10): the configured vault root and the layout in force, or `None` when the
/// role saves no capture. The vault is an owner choice (ADR-011), so an unset root is not a start
/// refusal: the api role's route then answers 503 `vault_not_open`, and the bot role answers the
/// owner's media with its failed-save line. A root of the wrong shape, or a layout file that cannot
/// be read or is not a layout, is logged by its rule, never by its value, and saves no capture
/// either. Nothing is created here: each capture locates the inbox anew (R4).
#[must_use]
pub fn inbox_captures(env: &Environment) -> Option<Arc<InboxCaptures<RealFs>>> {
    let root = match env.optional::<VaultRoot>(VAULT_ROOT) {
        Ok(Some(root)) => root,
        Ok(None) => {
            tracing::info!("no vault is configured, so no capture is saved");
            return None;
        }
        Err(error) => {
            tracing::warn!(%error, "the vault root is refused, so no capture is saved");
            return None;
        }
    };
    let layout = match LayoutInForce::from_env(env) {
        Ok(layout) => layout,
        Err(error) => {
            tracing::warn!(%error, "the vault layout is refused, so no capture is saved");
            return None;
        }
    };
    Some(Arc::new(InboxCaptures::new(
        RealFs,
        root.path().to_path_buf(),
        layout,
    )))
}

/// The unit credential role the API reads its seal secret under (SPEC-363 R5). The unit's line
/// that loads it ships with the owner's host step, not with this code: a unit that names a missing
/// credential fails to start.
pub const SEAL_SECRET_ROLE: &str = "sync-seal-secret";

/// Why the seal secret refuses the API's start. It names the credential's role, never a byte of it.
#[derive(Debug, thiserror::Error)]
pub enum SealSecretError {
    /// The credential holds fewer bytes than a seal secret needs.
    #[error("the credential {SEAL_SECRET_ROLE} is too short to be a seal secret")]
    TooShort(#[source] SealError),
    /// The credential could not be read; the error names its role.
    #[error(transparent)]
    Credential(#[from] CredentialError),
}

/// The seal secret the API's credentials hold, read once at start through `loader` (SPEC-363 R5,
/// A17). An absent credential turns the release off rather than refusing start: `None`, with one
/// info line naming the role and never a value. One shorter than 32 bytes refuses start by the
/// role, and so does every other error, as the API's other credentials do. The API's role composes
/// this reader at start, beside its other credentials, and an absent secret leaves the release off.
///
/// # Errors
///
/// [`SealSecretError::TooShort`] for a secret under 32 bytes, and
/// [`SealSecretError::Credential`] for a credential that is empty, unreadable or not text.
pub fn seal_secret(loader: &CredentialLoader) -> Result<Option<SealSecret>, SealSecretError> {
    let secret = match loader.load(SEAL_SECRET_ROLE) {
        Ok(secret) => secret,
        Err(CredentialError::Missing { id }) => {
            tracing::info!(
                role = id,
                "no seal secret is configured: the sync seal release is off"
            );
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    SealSecret::new(secret.expose().as_bytes())
        .map(Some)
        .map_err(SealSecretError::TooShort)
}

/// The recompute's fold, with every step registered in its phase (SPEC-071 R19): phase 1's
/// analytics step, counting leeches by `analytics`. A later SPEC registers its step here, in its
/// own phase, without touching the fold. The badge step awards against no configured course, and
/// Road to C2's steps read none.
///
/// # Errors
///
/// [`FoldError::OutsidePhase`] when a step is registered outside its phase.
pub fn recompute_fold(analytics: AnalyticsSettings) -> Result<Fold, FoldError> {
    recompute_fold_with_relights(analytics, Courses::default()).map(|(fold, _due)| fold)
}

/// [`recompute_fold`] over the owner's `courses`, which the badge step awards against (SPEC-073
/// R4) and Road to C2's progress and band badge steps read (SPEC-077 R6), and the handle the
/// streaks step answers its due relights on, for the cycle that routes them after the fold's
/// commit (SPEC-076 R27).
///
/// # Errors
///
/// [`FoldError::OutsidePhase`] when a step is registered outside its phase.
pub fn recompute_fold_with_relights(
    analytics: AnalyticsSettings,
    courses: Courses,
) -> Result<(Fold, RelightDue), FoldError> {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(analytics)),
    )?;
    fold.register(Phase::BaseXp, Box::new(XpStep))?;
    let (streaks, due) = StreaksStep::new();
    fold.register(Phase::StreaksAndGovernor, Box::new(streaks))?;
    fold.register(
        Phase::DaySteps,
        Box::new(ProgressStep::new(courses.clone(), analytics)),
    )?;
    fold.register(Phase::DaySteps, Box::new(HabitsStep))?;
    let writing = WritingStep::new(courses.clone());
    fold.register(Phase::DaySteps, Box::new(writing))?;
    fold.register(Phase::DerivedBonuses, Box::new(DayBonusesStep))?;
    fold.register(Phase::CoinMint, Box::new(MintStep))?;
    let habit_badges = HabitBadgesStep::new(courses.clone());
    fold.register(Phase::Awards, Box::new(habit_badges))?;
    fold.register(Phase::Awards, Box::new(BadgesStep::new(courses.clone())))?;
    fold.register(Phase::Awards, Box::new(RecordsStep))?;
    fold.register(Phase::Awards, Box::new(BandBadgesStep::new(courses)))?;
    Ok((fold, due))
}

/// Road to C2's live band for the persona engine (SPEC-077 R8, T26): the stored current band of
/// the configured course a language subject names, so a mentor writes at the band the owner has
/// reached rather than the roster's. Its production caller is the persona engine's output path
/// (#566); until that runs, A9 is its only caller.
pub struct CurriculumLiveBand {
    db: Db,
    courses: Courses,
}

impl CurriculumLiveBand {
    /// The live band over `db`'s stored course progress, for the configured `courses`.
    #[must_use]
    pub const fn new(db: Db, courses: Courses) -> Self {
        Self { db, courses }
    }
}

impl LiveBand for CurriculumLiveBand {
    fn band<'a>(&'a self, subject: &'a Subject) -> PortFuture<'a, Option<CefrBand>> {
        Box::pin(async move {
            if subject.kind() != SubjectKind::Language {
                return Ok(None);
            }
            let Some((_kind, area)) = subject.as_str().split_once('/') else {
                return Ok(None);
            };
            let configured = self
                .courses
                .courses()
                .iter()
                .find(|course| course.code.as_str() == area);
            let Some(course) = configured else {
                return Ok(None);
            };
            let mut connection = self.db.reader().acquire().await?;
            let bands = stored_bands(&mut connection).await?;
            Ok(bands
                .get(course.code.as_str())
                .and_then(|band| CefrBand::parse(band)))
        })
    }
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
    /// The notification policy the cycles' router reads does not parse (SPEC-319 R4).
    #[error(transparent)]
    Policy(#[from] deck_streak_notifications::PolicyError),
    /// The celebrations' switch could not be seeded (SPEC-319 R4).
    #[error("the celebrations' switch could not be seeded")]
    Switch(#[source] KernelError),
}

/// What every cycle of a role recomputes with, loaded once at the role's start (SPEC-071 R1, R3,
/// R4, R15): the owner's courses, the fold, and the notification policy of the router every cycle
/// holds (SPEC-319 R1).
#[derive(Clone, Debug)]
pub struct RecomputeSetup {
    courses: Courses,
    fold: Arc<Fold>,
    policy: Arc<Policy>,
    relights: RelightDue,
    instruments: Option<Arc<Instruments>>,
}

impl RecomputeSetup {
    /// Loads the owner's courses from `env` (R1), refuses start when they disagree with the
    /// readings taxonomy (R3), records their digest with the settings generation in `db`, which
    /// bumps the generation when they changed (R4), and builds the fold (R19). It compiles the
    /// notification policy and seeds the celebrations' switch off where none is stored (SPEC-319
    /// R4).
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
        let (fold, relights) =
            recompute_fold_with_relights(AnalyticsSettings::from_env(env)?, courses.clone())?;
        let policy = Arc::new(Policy::compiled()?);
        deck_streak_notifications::router::seed_celebrations_off(&policy, db, SystemClock.now())
            .await
            .map_err(RecomputeError::Switch)?;
        Ok(Self {
            courses,
            fold: Arc::new(fold),
            policy,
            relights,
            instruments: None,
        })
    }

    /// This setup, running `instruments` after the recompute of every cycle it hands out
    /// (SPEC-094 R7).
    #[must_use]
    pub fn with_instruments(mut self, instruments: Option<Arc<Instruments>>) -> Self {
        self.instruments = instruments;
        self
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

    /// `parts`, recomputing through the fold over `db`, with study days decided by `rule` (R15),
    /// and holding the router of [`holding_router`] (SPEC-319 R1).
    #[must_use]
    pub fn cycle<E: AnkiEngine + Sync>(
        &self,
        parts: CycleParts<E>,
        db: Db,
        rule: StudyDayRule,
    ) -> CycleParts<E> {
        let digest = self.courses.digest().map(str::to_owned);
        let parts = parts
            .with_fold(Arc::clone(&self.fold), db.clone(), rule, digest)
            .with_relights(self.relights.clone())
            .with_flush(Arc::new(holding_router(Arc::clone(&self.policy), db, rule)));
        match &self.instruments {
            Some(instruments) => parts.with_instruments(Arc::clone(instruments)),
            None => parts,
        }
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
    /// The refusal's [`RefusalReason`], when the cycle could not run to its end: a closed set, so a
    /// code outside it does not compile (SPEC-128 amendment, ADR-193).
    pub async fn run(&self) -> Result<SyncAnswer, RefusalReason> {
        let clock = Arc::new(SystemClock);
        let gate = ChangeGate::new(self.db.clone(), self.rule, clock.clone());
        gate.state()
            .request_rescore(clock.now())
            .await
            .map_err(|error| refused(Step::Rescore, &error))?;
        let settings =
            SyncSettings::from_env(&self.env).map_err(|error| refused(Step::Settings, &error))?;
        let directory = CredentialsDirectory::from_env(&self.env)
            .map_err(|error| refused(Step::Credentials, &error))?;
        let scope =
            ScopeSettings::from_env(&self.env).map_err(|error| refused(Step::Scope, &error))?;
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
            .map_err(|error| refused(Step::of(&error), &error))?;
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

/// The router every recompute cycle holds (SPEC-319 R1, R2, R3): `policy` over `db`, reading study
/// days by `rule` on the system's clock, with no bot transport. It holds each bot celebration it
/// routes for the processes that hold the bot's credentials, so the job loads none (ADR-066).
#[must_use]
pub fn holding_router(policy: Arc<Policy>, db: Db, rule: StudyDayRule) -> Router {
    Router::new(policy, db, Arc::new(SystemClock), rule).holding()
}

/// A step of the owner's sync whose failure refuses it (SPEC-128 A16; ADR-193): the four reads
/// `run` makes before the cycle, then one step for each kind of the cycle's own step errors. The
/// refusal is logged under the step's name, so two steps that give one code are told apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// Marking the owner's rescore pending.
    Rescore,
    /// Reading the sync's settings.
    Settings,
    /// Reading the credentials directory.
    Credentials,
    /// Reading the scope's settings.
    Scope,
    /// Reading the run record before the sync.
    History,
    /// Running the sync and recording its run.
    Sync,
    /// Reading the registered obligations' deadlines.
    Obligations,
    /// The change gate's probe of the copy.
    GateProbe,
    /// The change gate's read of its state and record of its decision or anchor.
    GateRecord,
    /// Reading the window from the copy.
    WindowRead,
    /// Reading or writing the window's base.
    WindowBase,
    /// The recompute's fold.
    Recompute,
}

impl Step {
    /// The cycle's step whose failure `error` is: each kind of each step's error is named, so a
    /// kind added to one does not compile until it is given a step.
    const fn of(error: &CycleError) -> Self {
        match error {
            CycleError::History(_) => Self::History,
            CycleError::Sync(SyncError::Store(_)) => Self::Sync,
            CycleError::Obligations(_) => Self::Obligations,
            CycleError::Gate(GateError::Read(_)) => Self::GateProbe,
            CycleError::Gate(GateError::Record(_)) => Self::GateRecord,
            CycleError::Window(WindowError::Read(_)) => Self::WindowRead,
            CycleError::Window(WindowError::State(_)) => Self::WindowBase,
            CycleError::Recompute(_) => Self::Recompute,
        }
    }

    /// The step's name in the refusal's log.
    const fn name(self) -> &'static str {
        match self {
            Self::Rescore => "rescore",
            Self::Settings => "settings",
            Self::Credentials => "credentials",
            Self::Scope => "scope",
            Self::History => "history",
            Self::Sync => "sync",
            Self::Obligations => "obligations",
            Self::GateProbe => "gate_probe",
            Self::GateRecord => "gate_record",
            Self::WindowRead => "window_read",
            Self::WindowBase => "window_base",
            Self::Recompute => "recompute",
        }
    }

    /// The code the step's refusal records. A cycle's step gives the `sync` job's own
    /// (`role_job.rs`); a failed sync is not a step's failure: it is a recorded run, answered as
    /// such.
    const fn reason(self) -> RefusalReason {
        match self {
            Self::Rescore => RefusalReason::RescoreUnrecorded,
            Self::Settings => RefusalReason::SyncSettingsRefused,
            Self::Credentials => RefusalReason::CredentialsDirectoryRefused,
            Self::Scope => RefusalReason::ScopeSettingsRefused,
            Self::History | Self::Sync => RefusalReason::SyncRecordFailed,
            Self::Obligations => RefusalReason::ObligationsUnreadable,
            Self::GateProbe
            | Self::GateRecord
            | Self::WindowRead
            | Self::WindowBase
            | Self::Recompute => RefusalReason::RecomputeFailed,
        }
    }
}

/// The refusal of `step`, logged under the step's name with its code and its cause: the cause
/// names a setting or a step, never a value. The code is the step's own, from the closed enum, so
/// no other code can be recorded through it.
fn refused(step: Step, error: &dyn std::fmt::Display) -> RefusalReason {
    let reason = step.reason();
    tracing::error!(
        step = step.name(),
        reason = reason.as_str(),
        %error,
        "the owner's sync could not run"
    );
    reason
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

/// How many drill grades the memory port gives a persona at most (SPEC-110 R12).
pub const DRILL_GRADES_LIMIT: i64 = 10;

/// The drill-grades memory port (SPEC-044 R7; SPEC-110 R12): a subject's newest grades, each a
/// drill's type and its accepted XP, never the drill's text, its id or its answer.
pub struct DrillGradesMemory {
    db: Db,
}

impl DrillGradesMemory {
    /// The port over `db`.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }
}

impl deck_streak_agent::MemoryPort for DrillGradesMemory {
    fn read<'a>(
        &'a self,
        subject: &'a deck_streak_agent::Subject,
    ) -> deck_streak_kernel::PortFuture<'a, Vec<String>> {
        Box::pin(async move {
            let mut connection = self.db.reader().acquire().await?;
            let grades = deck_streak_vault::drill_store::recent_grades(
                &mut connection,
                subject.as_str(),
                DRILL_GRADES_LIMIT,
            )
            .await?;
            Ok(grades
                .into_iter()
                .map(|grade| format!("{} xp {}", grade.drill_type, grade.xp))
                .collect())
        })
    }
}

/// The deck gate over the learner's marks (SPEC-381 R5; ADR-392 D2): it reads the marks at every
/// judgement, so a mark made after a day's selection still holds for the run, and it decides with
/// ingest's one rule, [`admits`], and nothing else. A failed read is judged as an unreadable set,
/// which `admits` refuses, so the gate fails closed.
pub struct MarksDeckGate {
    marks: SqliteSensitiveDecks,
    tree: BTreeMap<i64, String>,
}

impl MarksDeckGate {
    /// The gate over `marks`, judging against `tree`, the collection's deck names by id.
    #[must_use]
    pub const fn new(marks: SqliteSensitiveDecks, tree: BTreeMap<i64, String>) -> Self {
        Self { marks, tree }
    }
}

impl DeckGate for MarksDeckGate {
    fn judge<'a>(&'a self, scope: DeckScope<'a>) -> DeckFuture<'a> {
        Box::pin(async move {
            let marked = self.marks.read_marked().await.ok();
            judge_deck_scope(marked.as_ref(), &self.tree, scope)
        })
    }
}

/// Judges every card of `scope` with [`admits`] over `marked` (`None` when the marks could not be
/// read) and `tree`, and counts: any card that cannot be judged makes the scope unreadable, else
/// any card kept away makes it kept away, else it is admitted. The verdict carries counts only.
pub fn judge_deck_scope(
    marked: Option<&BTreeSet<i64>>,
    tree: &BTreeMap<i64, String>,
    scope: DeckScope<'_>,
) -> DeckVerdict {
    let mut kept_away = 0;
    let mut not_judged = 0;
    for card in scope.cards {
        match admits(marked, tree, card.home, card.current) {
            Admission::Admitted => {}
            Admission::KeptAway => kept_away += 1,
            Admission::Unresolved | Admission::Unreadable => not_judged += 1,
        }
    }
    if not_judged > 0 {
        DeckVerdict::Unreadable { cards: not_judged }
    } else if kept_away > 0 {
        DeckVerdict::KeptAway { cards: kept_away }
    } else {
        DeckVerdict::Admitted
    }
}

/// The one way a test captures log lines (SPEC-024, the 2026-09-30 amendment).
#[cfg(test)]
#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::sync::{Arc, Mutex, PoisonError};

    use deck_streak_bot::{ApiUrl, Scores, Sent, SyncAnswer, SyncOutcome, Transport};
    use deck_streak_coordination::delivery::{DeliveryCounts, DeliveryMarker};
    use deck_streak_coordination::sync_cycle::{CycleReport, Recompute};
    use deck_streak_ingest::engine::{AnkiEngine, RslibEngine};
    use deck_streak_ingest::gate::RunReason;
    use deck_streak_ingest::settings::SyncSettings;
    use deck_streak_ingest::state::{RefusalReason, SqliteIngestState};
    use deck_streak_ingest::sync::SyncReport;
    use deck_streak_ingest::sync_runs::{ReasonCode, SyncRun, Trigger};
    use deck_streak_kernel::{
        CredentialLoader, CredentialsDirectory, Db, Environment, Offload, OffloadWorkers, Redactor,
        StudyDay, StudyDayRule, SystemClock, UtcMillis,
    };

    use deck_streak_analytics::settings::AnalyticsSettings;
    use deck_streak_coordination::recompute::Phase;
    use deck_streak_coordination::recompute::analytics_step::ANALYTICS_STEP;
    use deck_streak_coordination::recompute::badges::BADGES_STEP;
    use deck_streak_coordination::recompute::band_badges::BAND_BADGES_STEP;
    use deck_streak_coordination::recompute::day_bonuses::DAY_BONUSES_STEP;
    use deck_streak_coordination::recompute::habit_badges::HABIT_BADGES_STEP;
    use deck_streak_coordination::recompute::habits::HABITS_STEP;
    use deck_streak_coordination::recompute::mint::MINT_STEP;
    use deck_streak_coordination::recompute::progress::PROGRESS_STEP;
    use deck_streak_coordination::recompute::records::RECORDS_STEP;
    use deck_streak_coordination::recompute::streaks::STREAKS_STEP;
    use deck_streak_coordination::recompute::writing::WRITING_STEP;
    use deck_streak_coordination::recompute::xp::XP_STEP;

    use super::{MarksDeckGate, judge_deck_scope};
    use super::{OwnerSyncCycle, RecomputeSetup, TransportMarker, answer_of, recompute_fold};

    use super::log_capture;

    use std::collections::{BTreeMap, BTreeSet};

    use deck_streak_agent::{CardDecks, DeckGate, DeckScope, DeckVerdict};
    use deck_streak_ingest::sensitive::SqliteSensitiveDecks;

    /// A deck tree: `Law`, `Law::Evidence` under it, `Spanish`, and a filtered deck.
    fn tree() -> BTreeMap<i64, String> {
        [
            (1, "Law"),
            (2, "Law\u{1f}Evidence"),
            (3, "Spanish"),
            (40, "Filtered"),
        ]
        .into_iter()
        .map(|(id, name)| (id, name.to_owned()))
        .collect()
    }

    /// One card in `home`, sitting in `current`.
    const fn card(home: i64, current: i64) -> CardDecks {
        CardDecks { home, current }
    }

    /// SPEC-381 R5: the scope's verdict counts `admits` per card, an unjudgeable card first.
    #[test]
    fn the_deck_gate_counts_each_card_by_the_one_rule_and_an_unjudged_card_comes_first() {
        let tree = tree();
        let law = BTreeSet::from([1]);
        let none = BTreeSet::new();
        let judge = |marked: Option<&BTreeSet<i64>>, cards: &[CardDecks]| {
            judge_deck_scope(marked, &tree, DeckScope { cards })
        };
        assert_eq!(
            judge(Some(&law), &[card(2, 2), card(3, 40), card(3, 3)]),
            DeckVerdict::KeptAway { cards: 1 },
            "a card under a marked deck is kept away; the rest are counted out"
        );
        assert_eq!(
            judge(Some(&law), &[card(3, 3), card(2, 40), card(1, 1)]),
            DeckVerdict::KeptAway { cards: 2 },
            "a card borrowed from a marked deck's child is kept away too"
        );
        assert_eq!(
            judge(Some(&law), &[card(2, 2), card(99, 3)]),
            DeckVerdict::Unreadable { cards: 1 },
            "a deck the tree does not hold makes the scope unreadable, before any kept-away card"
        );
        assert_eq!(
            judge(None, &[card(3, 3), card(3, 40)]),
            DeckVerdict::Unreadable { cards: 2 },
            "unreadable marks judge no card"
        );
        assert_eq!(
            judge(Some(&none), &[card(2, 2), card(3, 40)]),
            DeckVerdict::Admitted,
            "with no mark every resolved card is admitted"
        );
        assert_eq!(
            judge(Some(&law), &[card(3, 3), card(3, 40)]),
            DeckVerdict::Admitted,
            "an unmarked deck's cards are admitted"
        );
    }

    /// SPEC-381 R5: the gate reads the marks at every judgement, and a set it cannot read is
    /// judged unreadable, never admitted.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_marks_deck_gate_reads_the_marks_at_each_judgement_and_fails_closed() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let db = Db::open(&directory.path().join("deck_streak.db"))
            .await
            .expect("the database opens");
        let marks = SqliteSensitiveDecks::new(db.clone());
        let gate = MarksDeckGate::new(marks.clone(), tree());
        let cards = [card(2, 2), card(3, 3)];
        assert_eq!(
            gate.judge(DeckScope { cards: &cards }).await,
            DeckVerdict::Admitted,
            "no deck is marked yet"
        );
        marks
            .mark(1, UtcMillis::from_epoch_millis(1_000))
            .await
            .expect("the mark is stored");
        assert_eq!(
            gate.judge(DeckScope { cards: &cards }).await,
            DeckVerdict::KeptAway { cards: 1 },
            "a mark made after the gate was built holds"
        );
        db.close().await;
        assert_eq!(
            gate.judge(DeckScope { cards: &cards }).await,
            DeckVerdict::Unreadable { cards: 2 },
            "a closed database reads as unreadable marks"
        );
    }

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
    fn the_recompute_fold_registers_the_analytics_xp_and_streak_steps_in_their_phases() {
        let fold = recompute_fold(AnalyticsSettings::default()).expect("every step in its phase");
        assert_eq!(
            fold.steps(),
            [
                (Phase::RollupAndScore, ANALYTICS_STEP),
                (Phase::BaseXp, XP_STEP),
                (Phase::StreaksAndGovernor, STREAKS_STEP),
                (Phase::DaySteps, PROGRESS_STEP),
                (Phase::DaySteps, HABITS_STEP),
                (Phase::DaySteps, WRITING_STEP),
                (Phase::DerivedBonuses, DAY_BONUSES_STEP),
                (Phase::CoinMint, MINT_STEP),
                (Phase::Awards, HABIT_BADGES_STEP),
                (Phase::Awards, BADGES_STEP),
                (Phase::Awards, RECORDS_STEP),
                (Phase::Awards, BAND_BADGES_STEP),
            ]
        );
    }

    /// A22: Road to C2's two steps run in production, the progress step in phase 4 right after the
    /// streaks step and the band badge step in phase 7 right after the records step (SPEC-077 R6).
    #[test]
    fn the_recompute_fold_registers_road_to_c2s_steps() {
        let fold = recompute_fold(AnalyticsSettings::default()).expect("every step in its phase");
        let steps = fold.steps();
        let at = |step: (Phase, &str)| steps.iter().position(|registered| *registered == step);
        let streaks = at((Phase::StreaksAndGovernor, STREAKS_STEP)).expect("the streaks step");
        assert_eq!(
            at((Phase::DaySteps, PROGRESS_STEP)),
            Some(streaks + 1),
            "the progress step is registered in phase 4, right after the streaks step"
        );
        let records = at((Phase::Awards, RECORDS_STEP)).expect("the records step");
        assert_eq!(
            at((Phase::Awards, BAND_BADGES_STEP)),
            Some(records + 1),
            "the band badge step is registered in phase 7, right after the records step"
        );
    }

    #[test]
    fn the_recompute_fold_registers_the_habit_step_in_the_day_steps_phase() {
        let fold = recompute_fold(AnalyticsSettings::default()).expect("every step in its phase");
        let steps = fold.steps();
        assert_eq!(
            steps.get(2..7),
            Some(
                &[
                    (Phase::StreaksAndGovernor, STREAKS_STEP),
                    (Phase::DaySteps, PROGRESS_STEP),
                    (Phase::DaySteps, HABITS_STEP),
                    (Phase::DaySteps, WRITING_STEP),
                    (Phase::DerivedBonuses, DAY_BONUSES_STEP),
                ][..]
            ),
            "the habit step is phase 4's, after the streaks and before the derived bonuses (SPEC-078 R5)"
        );
    }

    /// A35: the habit badges step runs in production, in phase 7 right before the badge step
    /// (SPEC-078 R17 as amended).
    #[test]
    fn the_recompute_fold_registers_the_habit_badges_step_in_the_awards_phase() {
        let fold = recompute_fold(AnalyticsSettings::default()).expect("every step in its phase");
        let steps = fold.steps();
        let at = |step: (Phase, &str)| steps.iter().position(|registered| *registered == step);
        let badges = at((Phase::Awards, BADGES_STEP)).expect("the badge step");
        assert_eq!(
            at((Phase::Awards, HABIT_BADGES_STEP)),
            badges.checked_sub(1),
            "the habit badges step is registered in phase 7, right before the badge step"
        );
    }

    /// A35b: the writing step runs in production, in phase 4 right after the habit step
    /// (SPEC-078 R6).
    #[test]
    fn the_recompute_fold_registers_the_writing_step_after_the_habit_step() {
        let fold = recompute_fold(AnalyticsSettings::default()).expect("every step in its phase");
        let steps = fold.steps();
        let at = |step: (Phase, &str)| steps.iter().position(|registered| *registered == step);
        let habits = at((Phase::DaySteps, HABITS_STEP)).expect("the habit step");
        assert_eq!(
            at((Phase::DaySteps, WRITING_STEP)),
            Some(habits + 1),
            "the writing step is registered in phase 4, right after the habit step"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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
            Err(RefusalReason::SyncSettingsRefused),
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_owners_sync_without_a_credentials_directory_is_refused_by_its_own_code() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let db = Db::open(&directory.path().join("deck_streak.db"))
            .await
            .expect("the database opens");
        let workers = OffloadWorkers::new(1).expect("one worker is in range");
        let env = Environment::from_vars([
            (
                "DECKSTREAK_SYNC_ENDPOINT",
                std::ffi::OsString::from("http://127.0.0.1:9/"),
            ),
            ("STATE_DIRECTORY", directory.path().as_os_str().to_owned()),
        ]);
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
        assert_eq!(
            cycle.run().await,
            Err(RefusalReason::CredentialsDirectoryRefused),
            "no credentials directory is set"
        );
        db.close().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_owners_sync_that_cannot_mark_the_rescore_is_refused_by_its_own_code() {
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
        db.close().await;
        assert_eq!(
            cycle.run().await,
            Err(RefusalReason::RescoreUnrecorded),
            "the ledger is closed, so the request cannot be marked"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_owners_cycle_that_cannot_read_its_run_record_is_refused_by_the_sync_code() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let credentials = directory.path().join("credentials");
        std::fs::create_dir(&credentials).expect("the credentials directory is made");
        let db = Db::open(&directory.path().join("deck_streak.db"))
            .await
            .expect("the database opens");
        let mut write = db.write().await.expect("a write");
        sqlx::query("ALTER TABLE sync_runs RENAME TO sync_runs_unread")
            .execute(&mut *write)
            .await
            .expect("the run record is moved out of the cycle's reach");
        write.commit().await.expect("the rename commits");
        let workers = OffloadWorkers::new(1).expect("one worker is in range");
        let env = Environment::from_vars([
            (
                "DECKSTREAK_SYNC_ENDPOINT",
                std::ffi::OsString::from("http://127.0.0.1:9/"),
            ),
            ("STATE_DIRECTORY", directory.path().as_os_str().to_owned()),
            ("CREDENTIALS_DIRECTORY", credentials.as_os_str().to_owned()),
        ]);
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
        for call in ["first", "second"] {
            assert_eq!(
                cycle.run().await,
                Err(RefusalReason::SyncRecordFailed),
                "the {call} run: the cycle's first step cannot read the run record"
            );
        }
        db.close().await;
    }

    /// The steps `run` refuses from before the cycle, in the order it reaches them, each with the
    /// code SPEC-128 gives its refusal (A16). The code is the specification's, never read from the
    /// code under test, so a remap in the code is a failure here and not a new expectation.
    const RUN_STEPS: &[(&str, RefusalReason)] = &[
        ("rescore", RefusalReason::RescoreUnrecorded),
        ("settings", RefusalReason::SyncSettingsRefused),
        ("credentials", RefusalReason::CredentialsDirectoryRefused),
        ("scope", RefusalReason::ScopeSettingsRefused),
    ];

    /// The cycle's steps whose failure refuses the owner's sync at the cycle's own site, one for
    /// each kind of the cycle's own step errors, each with the code SPEC-128 gives its refusal
    /// (A16).
    const CYCLE_STEPS: &[(&str, RefusalReason)] = &[
        ("history", RefusalReason::SyncRecordFailed),
        ("sync", RefusalReason::SyncRecordFailed),
        ("obligations", RefusalReason::ObligationsUnreadable),
        ("gate_probe", RefusalReason::RecomputeFailed),
        ("gate_record", RefusalReason::RecomputeFailed),
        ("window_read", RefusalReason::RecomputeFailed),
        ("window_base", RefusalReason::RecomputeFailed),
        ("recompute", RefusalReason::RecomputeFailed),
    ];

    /// The run record moved out of the cycle's reach: the cycle's first read of it fails.
    const UNREAD_RUNS: &str = "ALTER TABLE sync_runs RENAME TO sync_runs_unread";

    /// Every run the sync writes is refused, after the history has read the record.
    const REFUSED_RUN: &str = "CREATE TRIGGER refused_run BEFORE INSERT ON sync_runs \
         BEGIN SELECT RAISE(ABORT, 'refused'); END";

    /// Each run the sync writes brings a second, `ok` run whose study day cannot be evaluated. The
    /// history reads only ids and statuses and the sync only writes, so the first read of a run's
    /// study day, the obligations', is the one that fails.
    const UNREADABLE_DAY: [&str; 3] = [
        "ALTER TABLE sync_runs RENAME TO sync_runs_base",
        "CREATE VIEW sync_runs AS SELECT id, trigger, \
         CASE WHEN attempts = 424242 THEN abs(id * 0 + (-9223372036854775807 - 1)) \
         ELSE study_day END AS study_day, \
         started_at, finished_at, status, reason, attempts, full_download, created_at \
         FROM sync_runs_base",
        "CREATE TRIGGER sync_runs_write INSTEAD OF INSERT ON sync_runs BEGIN \
         INSERT INTO sync_runs_base (trigger, study_day, started_at, finished_at, status, reason, \
         attempts, full_download, created_at) VALUES (NEW.trigger, NEW.study_day, \
         NEW.started_at, NEW.finished_at, NEW.status, NEW.reason, NEW.attempts, \
         NEW.full_download, NEW.created_at); \
         INSERT INTO sync_runs_base (trigger, study_day, started_at, finished_at, status, reason, \
         attempts, full_download, created_at) VALUES (NEW.trigger, NEW.study_day, \
         NEW.started_at, NEW.finished_at, 'ok', NULL, 424242, 0, NEW.created_at); END",
    ];

    /// The gate's anchor cannot be written: the probe, the window and the recompute have run.
    const REFUSED_ANCHOR: &str = "CREATE TRIGGER refused_anchor \
         BEFORE UPDATE OF anchor_recomputed_at ON ingest_state \
         BEGIN SELECT RAISE(ABORT, 'refused'); END";

    /// The copy's decks moved out of the window's reach: the gate probes only reviews and cards.
    const UNREAD_DECKS: &str = "ALTER TABLE decks RENAME TO decks_unread";

    /// The window's base cannot be written: the gate has read the state and decided to run.
    const REFUSED_WINDOW: &str = "CREATE TRIGGER refused_window \
         BEFORE UPDATE OF window_floor ON ingest_state BEGIN SELECT RAISE(ABORT, 'refused'); END";

    /// The analytics rollup moved out of the recompute's reach, after the window was read: it is
    /// the first table the fold reads. The fault names no table a census reserves to its owner, so
    /// it needs no exemption.
    const UNREAD_ROLLUP: &str = "ALTER TABLE daily_rollup RENAME TO daily_rollup_unread";

    /// An owner's sync on a fresh ledger and state directory, with one step's fault installed.
    struct Refusing {
        _directory: tempfile::TempDir,
        db: Db,
        cycle: OwnerSyncCycle,
    }

    /// The fault table, keyed by the step: an owner's sync in which `step`, and no step before it,
    /// fails. `None` for a step the table has no fault for. Every sync runs with an empty
    /// credentials directory, so it fails at once and records a failed run.
    async fn failing_at(step: &str) -> Option<Refusing> {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let credentials = directory.path().join("credentials");
        std::fs::create_dir(&credentials).expect("the credentials directory is made");
        let db = Db::open(&directory.path().join("deck_streak.db"))
            .await
            .expect("the database opens");
        let mut variables = vec![
            (
                "DECKSTREAK_SYNC_ENDPOINT",
                OsString::from("http://127.0.0.1:9/"),
            ),
            ("STATE_DIRECTORY", directory.path().as_os_str().to_owned()),
            ("CREDENTIALS_DIRECTORY", credentials.as_os_str().to_owned()),
        ];
        let mut ledger: &[&'static str] = &[];
        // The statements run in a copy of the collection the engine creates, when the step's
        // fault needs one; without a copy the gate's probe is the first step to fail.
        let mut copy: Option<&[&'static str]> = None;
        match step {
            // The ledger is closed once the cycle is built; no copy of the collection exists.
            "rescore" | "gate_probe" => {}
            "settings" => variables.clear(),
            "credentials" => variables.truncate(2),
            "scope" => variables.push((
                "DECKSTREAK_LAW_DECK_ROOT",
                OsString::from("Law\u{1f}Evidence"),
            )),
            "history" => ledger = &[UNREAD_RUNS],
            "sync" => ledger = &[REFUSED_RUN],
            "obligations" => ledger = &UNREADABLE_DAY,
            "gate_record" => {
                copy = Some(&[]);
                ledger = &[REFUSED_ANCHOR];
            }
            "window_read" => copy = Some(&[UNREAD_DECKS]),
            "window_base" => {
                copy = Some(&[]);
                ledger = &[REFUSED_WINDOW];
            }
            "recompute" => {
                copy = Some(&[]);
                ledger = &[UNREAD_ROLLUP];
            }
            _ => return None,
        }
        let env = Environment::from_vars(variables);
        if let Some(copy) = copy {
            let settings = SyncSettings::from_env(&env).expect("the sync's settings");
            RslibEngine
                .new_card_queue(&settings.copy_path())
                .expect("the engine creates the copy");
            // A rename re-reads the copy's schema, whose indexes name the engine's collation.
            let options = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(settings.copy_path())
                .collation("unicase", str::cmp);
            let mut connection =
                <sqlx::SqliteConnection as sqlx::Connection>::connect_with(&options)
                    .await
                    .expect("the copy opens");
            for statement in copy {
                sqlx::query(*statement)
                    .execute(&mut connection)
                    .await
                    .expect("the step's fault is installed in the copy");
            }
            sqlx::Connection::close(connection)
                .await
                .expect("the copy closes");
        }
        let mut write = db.write().await.expect("a write");
        for statement in ledger {
            sqlx::query(*statement)
                .execute(&mut *write)
                .await
                .expect("the step's fault is installed");
        }
        write.commit().await.expect("the fault commits");
        let recompute = RecomputeSetup::load(&env, &db)
            .await
            .expect("no courses and no taxonomy are configured");
        let cycle = OwnerSyncCycle::new(
            env,
            Redactor::new(),
            db.clone(),
            Offload::new(
                OffloadWorkers::new(1).expect("one worker is in range"),
                Arc::new(SystemClock),
            ),
            StudyDayRule::default(),
            recompute,
        );
        if step == "rescore" {
            db.close().await;
        }
        Some(Refusing {
            _directory: directory,
            db,
            cycle,
        })
    }

    /// One refusal as `refused` logs it: the step it names and the code it gives.
    type Logged = (Option<String>, Option<String>);

    /// The refusals logged on the test's thread.
    #[derive(Clone, Default)]
    struct Refusals(Arc<Mutex<Vec<Logged>>>);

    impl Refusals {
        /// Captures the refusals logged on the test's thread while the guard lives. The capture
        /// goes through `log_capture`, so a refusal another test's thread reached first is not
        /// cached as never enabled.
        fn capture() -> (Self, log_capture::CaptureGuard) {
            let refusals = Self::default();
            let guard = log_capture::hold_capture(refusals.clone());
            (refusals, guard)
        }

        /// The refusals logged since the last call.
        fn take(&self) -> Vec<Logged> {
            std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
        }
    }

    /// One event's step, code and message.
    #[derive(Default)]
    struct Fields {
        step: Option<String>,
        reason: Option<String>,
        message: String,
    }

    impl tracing::field::Visit for Fields {
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            match field.name() {
                "step" => self.step = Some(value.to_owned()),
                "reason" => self.reason = Some(value.to_owned()),
                "message" => value.clone_into(&mut self.message),
                _ => {}
            }
        }

        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = format!("{value:?}");
            }
        }
    }

    impl tracing::Subscriber for Refusals {
        // Error events only: the service's alert quotes the journal's error lines, so a refusal
        // logged below error is not one the table may count as named.
        fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
            *metadata.level() == tracing::Level::ERROR
        }

        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            let mut fields = Fields::default();
            event.record(&mut fields);
            if fields.message == "the owner's sync could not run" {
                self.0
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push((fields.step, fields.reason));
            }
        }

        fn enter(&self, _span: &tracing::span::Id) {}

        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// Asserts that `answer` refused the owner's sync by `code`, and that the one refusal `logged`
    /// names `step` and gives `code`. An answer is a failure swallowed into an `Ok`.
    fn refuses(
        answer: &Result<SyncAnswer, RefusalReason>,
        logged: &[Logged],
        step: &str,
        code: RefusalReason,
    ) {
        match answer {
            Ok(answer) => panic!("the {step} step's failure was answered: {answer:?}"),
            Err(reason) => assert_eq!(*reason, code, "the {step} step refuses by its own code"),
        }
        assert_eq!(
            logged,
            [(Some(step.to_owned()), Some(code.as_str().to_owned()))],
            "the {step} step's refusal is logged once, under its own name"
        );
    }

    /// Asserts that `step` was driven twice: a refusal right on the first run and wrong on a
    /// retry is caught only by the second drive.
    fn driven_twice(step: &str, drives: usize) {
        assert_eq!(
            drives, 2,
            "the {step} step is driven twice, each time on a fresh ledger"
        );
    }

    /// A16: every step whose failure refuses the owner's sync is driven by its own fault, twice,
    /// each time on a fresh ledger, and refuses by its step's code, logged under the step's name.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn every_failing_step_refuses_the_owners_sync_by_its_own_code_and_name() {
        let (refusals, _logging) = Refusals::capture();
        for (step, code) in RUN_STEPS.iter().chain(CYCLE_STEPS) {
            let mut drives = 0;
            for _ in 0..2 {
                let refusing = failing_at(step)
                    .await
                    .expect("the fault table has a fault for every step");
                let answer = refusing.cycle.run().await;
                refuses(&answer, &refusals.take(), step, *code);
                refusing.db.close().await;
                drives += 1;
            }
            driven_twice(step, drives);
        }
    }

    /// A17: the table has a row for every step the source gives a refusal: each `?` in `run`
    /// before the cycle's own site, and each kind of cycle error `Step::of` names. A new step with
    /// no row fails here, and so does a row with no step.
    #[test]
    fn the_table_has_a_row_for_every_step_that_refuses_the_owners_sync() {
        let source = include_str!("wiring.rs");
        let (code, _) = source
            .split_once("#[cfg(test)]\nmod tests")
            .expect("the tests follow the code");
        let (_, run) = code
            .split_once("    pub async fn run(&self)")
            .expect("run is declared");
        let (run, _) = run.split_once("\n    }\n").expect("run ends");
        let sites: usize = run
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .map(|line| line.matches('?').count())
            .sum();
        let arms: usize = code
            .lines()
            .filter(|line| line.contains("=>"))
            .map(|line| line.matches("CycleError::").count())
            .sum();
        assert_eq!(
            sites,
            RUN_STEPS.len() + 1,
            "each `?` in run before the cycle's own is a step in the table"
        );
        assert_eq!(
            arms,
            CYCLE_STEPS.len(),
            "each kind of cycle error `Step::of` names is a step in the table"
        );
    }

    /// The table's check refuses an answer: a step's failure swallowed into an `Ok` fails it.
    #[test]
    #[should_panic(expected = "was answered")]
    fn the_table_fails_a_step_whose_failure_is_answered() {
        let answer = Ok(SyncAnswer {
            sync: SyncOutcome::Reused,
            scores: Scores::Unchanged,
        });
        let logged = [(
            Some("gate_probe".to_owned()),
            Some(RefusalReason::RecomputeFailed.as_str().to_owned()),
        )];
        refuses(
            &answer,
            &logged,
            "gate_probe",
            RefusalReason::RecomputeFailed,
        );
    }

    /// The table's check refuses a refusal logged under another step's name, though its code is the
    /// step's own: two steps that give one code are told apart by the name.
    #[test]
    #[should_panic(expected = "under its own name")]
    fn the_table_fails_a_refusal_logged_under_another_steps_name() {
        let logged = [(
            Some("gate_probe".to_owned()),
            Some(RefusalReason::RecomputeFailed.as_str().to_owned()),
        )];
        refuses(
            &Err(RefusalReason::RecomputeFailed),
            &logged,
            "window_base",
            RefusalReason::RecomputeFailed,
        );
    }

    /// The table's check refuses a step driven once: a retry that answers by another code is
    /// caught only by the second drive.
    #[test]
    #[should_panic(expected = "driven twice")]
    fn the_table_fails_a_step_driven_once() {
        driven_twice("window_base", 1);
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

    /// The one spelling a multi-thread test of this module wears (#577, SPEC-325 R1): two
    /// workers whatever the host's core count, so the tests libtest runs at once draw a bounded
    /// number of threads, and the driver's thread for each connection is never refused for them.
    const BOUNDED_MULTI_THREAD: &str =
        "#[tokio::test(flavor = \"multi_thread\", worker_threads = 2)]";

    /// The multi-thread `tokio::test` attributes in `source`, and those among them that are not
    /// [`BOUNDED_MULTI_THREAD`], each as its trimmed line.
    fn multi_thread_attributes(source: &str) -> (Vec<&str>, Vec<&str>) {
        let attributes: Vec<&str> = source
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("#[tokio::test(") && line.contains("multi_thread"))
            .collect();
        let unbounded = attributes
            .iter()
            .copied()
            .filter(|line| *line != BOUNDED_MULTI_THREAD)
            .collect();
        (attributes, unbounded)
    }

    /// A1 (#577, SPEC-325 R2): a runtime built from the module's multi-thread attribute runs two
    /// workers, read from the runtime itself, not one worker per core of the host.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_multi_thread_wiring_test_runs_on_two_workers_whatever_the_host() {
        assert_eq!(
            tokio::runtime::Handle::current().metrics().num_workers(),
            2,
            "the runtime runs the attribute's two workers, not one per core of the host"
        );
    }

    /// A2 (#577, SPEC-325 R3): every multi-thread `tokio::test` in this file wears the bounded
    /// attribute, so the module's peak thread count does not grow with the host; a planted
    /// unbounded attribute is refused by its line.
    #[test]
    fn every_multi_thread_wiring_test_bounds_its_runtime_to_two_workers() {
        let (attributes, unbounded) = multi_thread_attributes(include_str!("wiring.rs"));
        assert_eq!(
            unbounded,
            Vec::<&str>::new(),
            "each multi-thread test names worker_threads = 2"
        );
        println!(
            "examined {} multi-thread test attribute(s)",
            attributes.len()
        );
        assert!(
            !attributes.is_empty(),
            "examined 0 multi-thread test attributes: the population is empty, so nothing was judged"
        );
        let planted = "    #[tokio::test(flavor = \"multi_thread\")]\n    async fn planted() {}\n";
        assert_eq!(
            multi_thread_attributes(planted).1,
            vec!["#[tokio::test(flavor = \"multi_thread\")]"],
            "a planted unbounded attribute is refused by its line"
        );
    }

    /// A17: the API starts with the release off when its credentials hold no seal secret, and
    /// refuses start by the credential's role when they hold one under 32 bytes (SPEC-363 R5).
    #[test]
    fn a_missing_seal_secret_turns_the_release_off() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let loader = || {
            CredentialLoader::new(
                CredentialsDirectory::new(directory.path()).expect("an absolute path"),
                Redactor::new(),
            )
        };
        let absent = super::seal_secret(&loader());
        assert!(
            matches!(absent, Ok(None)),
            "no seal secret turns the release off: {absent:?}"
        );

        let path = directory.path().join(super::SEAL_SECRET_ROLE);
        let refused = vec![
            ("an empty seal secret", String::new()),
            ("a seal secret of 16 bytes", "q".repeat(16)),
            ("a seal secret of 31 bytes", "q".repeat(31)),
        ];
        println!("examined {} seal secrets that refuse start", refused.len());
        for (what, value) in refused {
            std::fs::write(&path, format!("{value}\n")).expect("a credential");
            let error = super::seal_secret(&loader()).expect_err("the secret refuses start");
            let text = error.to_string();
            assert!(
                text.contains(super::SEAL_SECRET_ROLE),
                "{what} refuses start by the credential's role: {text}"
            );
            assert!(
                value.is_empty() || !text.contains(value.as_str()),
                "{what} is never named by its value"
            );
        }

        let accepted = vec![
            ("a seal secret of 32 bytes", "q".repeat(32)),
            ("a seal secret of 100 bytes", "q".repeat(100)),
        ];
        println!(
            "examined {} seal secrets that turn the release on",
            accepted.len()
        );
        for (what, value) in accepted {
            std::fs::write(&path, format!("{value}\n")).expect("a credential");
            let read = super::seal_secret(&loader());
            assert!(
                matches!(read, Ok(Some(_))),
                "{what} turns the release on: {read:?}"
            );
        }
    }
}

/// Why the instruments cannot be built. Each names a setting, never a value.
#[derive(Debug, thiserror::Error)]
pub enum InstrumentsError {
    /// The owner's note conventions refused start (SPEC-094 R1, R2; ADR-096).
    #[error(transparent)]
    Conventions(#[from] ConventionsError),
    /// The private copy's settings or its scope refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
}

/// Dark Fields' reads: the structure of the private copy, within scope, read through the ingest
/// context's reader so the instruments' host never opens the copy itself (ADR-094).
struct StructureSource {
    reader: CollectionReader,
}

impl ReadSource<StructureReads> for StructureSource {
    fn read(&self) -> BoxFuture<'_, Result<StructureReads, String>> {
        Box::pin(async move {
            self.reader.read_structure().await.map_err(|error| {
                tracing::error!(%error, "the copy's structure could not be read");
                "the copy's structure could not be read".to_owned()
            })
        })
    }
}

/// The instruments a role runs: each live instrument's frame over its reads, one at a time behind
/// the lock in `state`, storing into `db` (SPEC-094 R7, R8).
///
/// The owner's note conventions are loaded here once, so a malformed file or a forbidden label
/// refuses the role's start before any instrument can read them (R1, R2).
///
/// # Errors
///
/// Every refusal of [`InstrumentsError`].
pub fn build_instruments(
    env: &Environment,
    db: Db,
    state: &StateDirectory,
    offload: Offload,
    rule: StudyDayRule,
) -> Result<Arc<Instruments>, InstrumentsError> {
    let _conventions = Conventions::load(env)?;
    let settings = SyncSettings::from_env(env)?;
    let scope = ScopeSettings::from_env(env)?;
    let reader = CollectionReader::new(&settings, scope, offload.clone());
    let dark_fields: Arc<dyn InstrumentRunner> =
        Arc::new(Frame::new(DarkFields, StructureSource { reader }, offload));
    Ok(Arc::new(Instruments::new(
        db,
        state.path(),
        Arc::new(SystemClock),
        rule,
        vec![dark_fields],
    )))
}

/// [`build_instruments`] for a role: a private copy's settings that refuse leave the role without
/// instruments (logged; the sync's own refusal is recorded by its cycle), while a conventions file
/// that refuses stops the role's start.
///
/// # Errors
///
/// [`ConventionsError`] when the owner's conventions refuse start.
pub fn instruments_for_role(
    env: &Environment,
    db: Db,
    state: &StateDirectory,
    offload: Offload,
    rule: StudyDayRule,
) -> Result<Option<Arc<Instruments>>, ConventionsError> {
    match build_instruments(env, db, state, offload, rule) {
        Ok(instruments) => Ok(Some(instruments)),
        Err(InstrumentsError::Conventions(error)) => Err(error),
        Err(InstrumentsError::Settings(error)) => {
            tracing::warn!(%error, "the instruments are not available: the copy's settings refuse");
            Ok(None)
        }
    }
}

/// The instruments as the api role holds them: the api binds before the database opens, so the
/// service is filled in when the opener finishes, and answers "not ready" until then.
#[derive(Clone, Debug, Default)]
pub struct LateInstruments(Arc<OnceLock<Arc<Instruments>>>);

impl LateInstruments {
    /// An empty holder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fills the holder once the instruments are built.
    pub fn fill(&self, instruments: Arc<Instruments>) {
        drop(self.0.set(instruments));
    }

    fn not_ready() -> KernelError {
        KernelError::Offload {
            operation: "instruments_not_ready",
        }
    }
}

impl InstrumentService for LateInstruments {
    fn list(&self) -> BoxFuture<'_, Result<Vec<InstrumentListing>, KernelError>> {
        Box::pin(async move {
            match self.0.get() {
                Some(instruments) => instruments.list().await,
                None => Err(Self::not_ready()),
            }
        })
    }

    fn report<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<StoredReport>, KernelError>> {
        Box::pin(async move {
            match self.0.get() {
                Some(instruments) => instruments.report(id).await,
                None => Err(Self::not_ready()),
            }
        })
    }

    fn run<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<StoredReport, OnDemandRefusal>> {
        Box::pin(async move {
            match self.0.get() {
                Some(instruments) => instruments.run(id).await,
                None => Err(OnDemandRefusal::Store(Self::not_ready())),
            }
        })
    }
}
