//! The `api` role: the HTTP service the Mini App calls through the reverse proxy (SPEC-025,
//! SPEC-024; ADR-007, ADR-010, ADR-025, ADR-038).
//!
//! It starts in the order `docs/schematics/service-lifecycle.md` draws. The settings are read, and
//! a missing or malformed one refuses start by name. Identity's two credentials, the owner's user id
//! and the bot token, are read through the kernel's loader with the redactor the log writer reads,
//! so neither value can reach a later line; a missing or malformed one refuses start by its id
//! (`docs/schematics/startup-settings-and-secrets.md`). The shutdown signal's handlers are
//! installed. The loopback listener is bound; systemd hears `READY=1`, and the heartbeat starts. The
//! database then opens under the open lock while the API already answers readiness with 503, and
//! 200 once it is open. On SIGTERM the role says `STOPPING=1`, drains, closes the database and
//! returns. A database that fails to open stops the role the same way, and the role returns the
//! failure.

use std::sync::Arc;

use deck_streak_api::settings::LISTEN;
use deck_streak_api::{ApiError, ApiState, ListenAddress, OwnerAccess, Readiness};
use deck_streak_coordination::progression::law_tiers::CollectionLawTiers;
use deck_streak_coordination::progression::level_view::LawTierSource;
use deck_streak_identity::sync_seal::SealSecret;
use deck_streak_identity::{Freshness, IdentityError, LinkingConfig, OwnerGate};
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::settings::{ScopeSettings, SyncSettings};
use deck_streak_kernel::{
    Clock, Conventions, ConventionsError, Courses, CredentialLoader, CredentialsDirectory,
    Environment, KernelSettings, Offload, Redactor, SettingsError, SystemClock,
};
use tokio::sync::oneshot;

use crate::lifecycle::{self, Notifier, NotifyState, ShutdownSignal};
use crate::snapshot_lister::CommandLister;
use crate::wiring::{self, StateDirectory, WiringError};

/// Why the `api` role stopped with an error.
#[derive(Debug, thiserror::Error)]
pub enum ApiRoleError {
    /// The owner's note conventions refused start (SPEC-094 R2; ADR-096).
    #[error(transparent)]
    Conventions(#[from] ConventionsError),
    /// A setting refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// One of identity's credentials refused start, by its id.
    #[error(transparent)]
    Identity(#[from] IdentityError),
    /// The API refused start or stopped serving.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// The shutdown signal's handlers could not be installed.
    #[error("the role could not install its shutdown signal handlers")]
    Signals(#[source] std::io::Error),
    /// The database could not be opened; the role drained and stopped.
    #[error("the database could not be opened")]
    Database(#[source] WiringError),
    /// The seal secret is held and refuses start, by its role (SPEC-363 R5).
    #[error(transparent)]
    SealSecret(#[from] wiring::SealSecretError),
}

/// Why the role stopped serving.
enum Stop {
    /// SIGTERM or SIGINT.
    Signal,
    /// The database failed to open.
    Database(WiringError),
}

/// The API's state as the role composes it: the readiness the database opens into, the owner's
/// access, (SPEC-072 R24) the law tiers' source when the settings name a collection to read, and
/// (SPEC-118 R10) the vault inbox's quick captures when the settings name a vault root.
///
/// The composition lives here so the daemon's own test can drive the router the role serves.
#[must_use]
pub fn api_state(
    env: &Environment,
    offload: &Offload,
    readiness: Readiness,
    access: OwnerAccess,
) -> ApiState {
    let state = ApiState::new(readiness)
        .with_owner(access)
        .with_courses(courses(env));
    let state = match law_tier_source(env, offload) {
        Some(source) => state.with_law_tiers(source),
        None => state,
    };
    let state = match crate::drill_vault::open(env) {
        Some(notes) => state.with_drills(notes),
        None => state,
    };
    match wiring::inbox_captures(env) {
        Some(captures) => state.with_inbox(captures),
        None => state,
    }
}

/// The courses the badge catalog's descriptions are rendered from (SPEC-073 R16): the configured
/// ones, or the defaults when the setting refuses, which the log says.
fn courses(env: &Environment) -> Courses {
    match Courses::load(env) {
        Ok(courses) => courses,
        Err(error) => {
            tracing::warn!(%error, "the badge catalog uses the default courses: the setting refused");
            Courses::default()
        }
    }
}

/// The law tiers' source, when the settings name a collection copy to read and a scope to read it
/// in. A role whose settings do not (the API can run apart from the sync) serves the view as
/// unavailable, and says why in its log.
fn law_tier_source(env: &Environment, offload: &Offload) -> Option<Arc<dyn LawTierSource>> {
    let settings = match SyncSettings::from_env(env) {
        Ok(settings) => settings,
        Err(refusal) => {
            tracing::warn!(%refusal, "the law tiers are unavailable: the collection is not named");
            return None;
        }
    };
    let scope = match ScopeSettings::from_env(env) {
        Ok(scope) => scope,
        Err(refusal) => {
            tracing::warn!(%refusal, "the law tiers are unavailable: the read's scope refuses");
            return None;
        }
    };
    let reader = CollectionReader::new(&settings, scope, offload.clone());
    let rule = match KernelSettings::from_env(env) {
        Ok(kernel) => kernel.study_day_rule,
        Err(refusal) => {
            tracing::warn!(%refusal, "the law tiers are unavailable: the study day is unknown");
            return None;
        }
    };
    Some(Arc::new(CollectionLawTiers::new(reader, rule)))
}

/// `state`, releasing the web client's sealing key under `secret` when the API's credentials hold
/// one (SPEC-363 R5, B14). With none the release route stays off and the role still starts.
///
/// The composition lives here so the daemon's own test can drive the router the role serves.
#[must_use]
pub fn with_seal_secret(state: ApiState, secret: Option<SealSecret>) -> ApiState {
    match secret {
        Some(secret) => state.with_seal(secret),
        None => state,
    }
}

/// `state`, answering the snapshot route from `lister` when the role's settings and credentials
/// configure one (SPEC-377 R14). With none the route answers unknown and the role still starts.
#[must_use]
pub fn with_snapshot_lister(state: ApiState, lister: Option<CommandLister>) -> ApiState {
    match lister {
        Some(lister) => state.with_snapshot(Arc::new(lister)),
        None => state,
    }
}

/// Runs the `api` role until SIGTERM (or SIGINT), and returns once every request in flight has
/// finished. `redactor` is the one the process's log writer reads: every credential the role
/// loads is registered with it.
///
/// # Errors
///
/// Every refusal of [`ApiRoleError`]: a setting or a credential refuses start before anything is
/// bound, and a database that fails to open stops the role after a drain.
pub async fn run(env: &Environment, redactor: &Redactor) -> Result<(), ApiRoleError> {
    // Every role reads the kernel's settings, so a malformed one refuses start in each of them.
    let kernel = KernelSettings::from_env(env)?;
    let listen = ListenAddress::from_env(env)?;
    let state = StateDirectory::from_env(env)?;
    let freshness = Freshness::from_env(env)?;
    let credentials = CredentialsDirectory::from_env(env)?;
    // The archive's lister is read before the loader takes the directory: a list command that is
    // set and malformed refuses start, and an absent command or credential turns it off (SPEC-377
    // R14).
    let archive_lister = CommandLister::configured(env, &credentials)?;
    let loader = CredentialLoader::new(credentials, redactor.clone());
    let gate = OwnerGate::load(&loader, freshness)?;
    // The seal secret is read beside the other credentials, before anything is bound: an absent one
    // turns the release off, and one that is held and malformed refuses start (SPEC-363 R5).
    let seal = wiring::seal_secret(&loader)?;
    // The conventions refuse start here, before anything is bound (SPEC-094 R2).
    let _conventions = Conventions::load(env)?;
    // So does a public origin that is set and is not an https origin (SPEC-359 R1).
    let linking = LinkingConfig::from_env(env)?;
    let notifier = Notifier::from_env(env);
    let shutdown = ShutdownSignal::install().map_err(ApiRoleError::Signals)?;

    let listener = deck_streak_api::bind(listen).await?;
    let bound = listener.local_addr().map_err(|source| ApiError::Bind {
        setting: LISTEN,
        source,
    })?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let readiness = Readiness::new();
    let owner = gate.owner();
    let access = OwnerAccess::new(gate, Arc::clone(&clock), kernel.study_day_rule);
    let late = wiring::LateInstruments::new();
    let offload = Offload::new(kernel.offload_workers, clock);
    let router = deck_streak_api::router(with_snapshot_lister(
        with_seal_secret(
            api_state(env, &offload, readiness.clone(), access)
                .with_linking(linking, owner)
                .with_instruments(Arc::new(late.clone())),
            seal,
        ),
        archive_lister,
    ));
    tracing::info!(listen = %bound, "the api role serves");
    notifier.notify(NotifyState::Ready);
    let heartbeat = lifecycle::spawn_heartbeat(notifier.clone(), env);

    let (failed, failure) = oneshot::channel();
    let rule = kernel.study_day_rule;
    let opener = {
        let readiness = readiness.clone();
        let env = env.clone();
        tokio::spawn(async move {
            match wiring::open_database(&offload, &state).await {
                Ok(database) => {
                    match wiring::instruments_for_role(
                        &env,
                        database.clone(),
                        &state,
                        offload.clone(),
                        rule,
                    ) {
                        Ok(Some(instruments)) => late.fill(instruments),
                        Ok(None) => {}
                        Err(error) => {
                            tracing::error!(%error, "the owner's conventions refuse the instruments");
                        }
                    }
                    readiness.database_opened(database);
                    tracing::info!("the database is open and migrated");
                }
                Err(error) => drop(failed.send(error)),
            }
        })
    };

    let (stopped, reason) = oneshot::channel();
    let stopping = notifier.clone();
    deck_streak_api::serve(listener, router, async move {
        let why = tokio::select! {
            () = shutdown.received() => Stop::Signal,
            Ok(error) = failure => Stop::Database(error),
        };
        stopping.notify(NotifyState::Stopping);
        drop(stopped.send(why));
    })
    .await?;

    opener.abort();
    if let Some(heartbeat) = heartbeat {
        heartbeat.abort();
    }
    if let Some(database) = readiness.database() {
        database.close().await;
    }
    match reason.await {
        Ok(Stop::Database(error)) => Err(ApiRoleError::Database(error)),
        Ok(Stop::Signal) | Err(_) => {
            tracing::info!("the api role stopped");
            Ok(())
        }
    }
}
