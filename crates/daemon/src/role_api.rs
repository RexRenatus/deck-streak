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
use deck_streak_identity::{Freshness, IdentityError, OwnerGate};
use deck_streak_kernel::{
    Clock, CredentialLoader, CredentialsDirectory, Environment, KernelSettings, Offload, Redactor,
    SettingsError, SystemClock,
};
use tokio::sync::oneshot;

use crate::lifecycle::{self, Notifier, NotifyState, ShutdownSignal};
use crate::wiring::{self, StateDirectory, WiringError};

/// Why the `api` role stopped with an error.
#[derive(Debug, thiserror::Error)]
pub enum ApiRoleError {
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
}

/// Why the role stopped serving.
enum Stop {
    /// SIGTERM or SIGINT.
    Signal,
    /// The database failed to open.
    Database(WiringError),
}

/// The API's state as the role composes it: the readiness the database opens into, the owner's
/// access, and (SPEC-072 R24) the law tiers' source when the settings name a collection to read.
///
/// The composition lives here so the daemon's own test can drive the router the role serves.
#[must_use]
pub fn api_state(
    _env: &Environment,
    _offload: &Offload,
    readiness: Readiness,
    access: OwnerAccess,
) -> ApiState {
    ApiState::new(readiness).with_owner(access)
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
    let gate = OwnerGate::load(
        &CredentialLoader::new(credentials, redactor.clone()),
        freshness,
    )?;
    let notifier = Notifier::from_env(env);
    let shutdown = ShutdownSignal::install().map_err(ApiRoleError::Signals)?;

    let listener = deck_streak_api::bind(listen).await?;
    let bound = listener.local_addr().map_err(|source| ApiError::Bind {
        setting: LISTEN,
        source,
    })?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let readiness = Readiness::new();
    let access = OwnerAccess::new(gate, Arc::clone(&clock), kernel.study_day_rule);
    let offload = Offload::new(kernel.offload_workers, clock);
    let router = deck_streak_api::router(api_state(env, &offload, readiness.clone(), access));
    tracing::info!(listen = %bound, "the api role serves");
    notifier.notify(NotifyState::Ready);
    let heartbeat = lifecycle::spawn_heartbeat(notifier.clone(), env);

    let (failed, failure) = oneshot::channel();
    let opener = {
        let readiness = readiness.clone();
        tokio::spawn(async move {
            match wiring::open_database(&offload, &state).await {
                Ok(database) => {
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
