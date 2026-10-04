//! The `mcp` role: the MCP adapter's server, which the owner's agent calls on a loopback address
//! (SPEC-119; ADR-119, ADR-329).
//!
//! It starts in the `api` role's order. The settings are read, and a missing or malformed one
//! refuses start by name: the kernel's, the listen address, which must be a loopback address (R3),
//! and the state and credentials directories. The guard's tokens are read through the kernel's
//! loader with the redactor the log writer reads, once, and nowhere else (R6; A39): a missing or
//! malformed core token refuses start by its id. The shutdown signal's handlers are installed, the
//! loopback listener is bound, systemd hears `READY=1`, and the heartbeat starts. The database then
//! opens, raced with the shutdown signal; the server serves `get_law_track` over it behind the
//! guard. On SIGTERM the role says `STOPPING=1`, drains, closes the database and returns. A
//! database that fails to open stops the role, and the role returns the failure.

use std::io;
use std::sync::Arc;

use deck_streak_kernel::{
    Clock, CredentialLoader, CredentialsDirectory, Environment, KernelSettings, Offload, Redactor,
    SettingsError, SystemClock,
};
use deck_streak_mcp::settings::LISTEN;
use deck_streak_mcp::{Grants, Guard, LedgerLawTrack, ListenAddress, McpError, server};

use crate::lifecycle::{self, Notifier, NotifyState, ShutdownSignal};
use crate::wiring::{self, StateDirectory, WiringError};

/// Why the `mcp` role stopped with an error.
#[derive(Debug, thiserror::Error)]
pub enum McpRoleError {
    /// A setting refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The adapter refused start: the listen address, or a credential by its id.
    #[error(transparent)]
    Mcp(#[from] McpError),
    /// The shutdown signal's handlers could not be installed.
    #[error("the role could not install its shutdown signal handlers")]
    Signals(#[source] io::Error),
    /// The listen address could not be bound. It names the setting, never its value.
    #[error("the address the setting {setting} names could not be bound")]
    Bind {
        /// The listen address's setting.
        setting: &'static str,
        /// The socket's error.
        #[source]
        source: io::Error,
    },
    /// The server stopped serving with an error.
    #[error("the server stopped serving")]
    Serve(#[source] io::Error),
    /// The database could not be opened; the role stopped.
    #[error("the database could not be opened")]
    Database(#[source] WiringError),
}

/// Says `STOPPING=1` and stops the heartbeat, as the role leaves before it serves.
fn stop_before_serving(notifier: &Notifier, heartbeat: Option<tokio::task::JoinHandle<()>>) {
    notifier.notify(NotifyState::Stopping);
    if let Some(heartbeat) = heartbeat {
        heartbeat.abort();
    }
}

/// Runs the `mcp` role until SIGTERM (or SIGINT), and returns once every request in flight has
/// finished. `redactor` is the one the process's log writer reads: every credential the role
/// loads is registered with it.
///
/// # Errors
///
/// Every refusal of [`McpRoleError`]: a setting or a credential refuses start before anything is
/// bound, and a database that fails to open stops the role.
pub async fn run(env: &Environment, redactor: &Redactor) -> Result<(), McpRoleError> {
    // Every role reads the kernel's settings, so a malformed one refuses start in each of them.
    let kernel = KernelSettings::from_env(env)?;
    let listen = ListenAddress::from_env(env)?;
    let state = StateDirectory::from_env(env)?;
    let credentials = CredentialsDirectory::from_env(env)?;
    let grants = Grants::load(&CredentialLoader::new(credentials, redactor.clone()))?;
    let notifier = Notifier::from_env(env);
    let shutdown = ShutdownSignal::install().map_err(McpRoleError::Signals)?;

    let bind_refused = |source| McpRoleError::Bind {
        setting: LISTEN,
        source,
    };
    let listener = server::bind(listen).await.map_err(bind_refused)?;
    let bound = listener.local_addr().map_err(bind_refused)?;
    tracing::info!(listen = %bound, "the mcp role is bound");
    notifier.notify(NotifyState::Ready);
    let heartbeat = lifecycle::spawn_heartbeat(notifier.clone(), env);

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let offload = Offload::new(kernel.offload_workers, Arc::clone(&clock));
    let mut shutdown = Box::pin(shutdown.received());
    let opened = tokio::select! {
        opened = wiring::open_database(&offload, &state) => Some(opened),
        () = &mut shutdown => None,
    };
    let database = match opened {
        Some(Ok(database)) => database,
        Some(Err(error)) => {
            stop_before_serving(&notifier, heartbeat);
            return Err(McpRoleError::Database(error));
        }
        None => {
            stop_before_serving(&notifier, heartbeat);
            tracing::info!("the mcp role stopped before the database opened");
            return Ok(());
        }
    };
    tracing::info!("the database is open and migrated");

    let guard = Arc::new(Guard::new(grants, Arc::clone(&clock)));
    let law = Arc::new(LedgerLawTrack::new(
        database.clone(),
        clock,
        kernel.study_day_rule,
    ));
    let router = server::router(law, guard);
    tracing::info!("the mcp role serves");
    let stopping = notifier.clone();
    let served = server::serve(listener, router, async move {
        shutdown.await;
        stopping.notify(NotifyState::Stopping);
    })
    .await;

    if let Some(heartbeat) = heartbeat {
        heartbeat.abort();
    }
    database.close().await;
    served.map_err(McpRoleError::Serve)?;
    tracing::info!("the mcp role stopped");
    Ok(())
}
