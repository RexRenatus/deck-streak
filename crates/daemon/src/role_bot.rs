//! The `bot` role: the Telegram bot, long-polling for the owner's updates (SPEC-026 R1, R13;
//! ADR-010, ADR-025, ADR-026, ADR-038).
//!
//! It starts in the order `docs/schematics/bot-update-loop.md` draws. The settings are read, and a
//! missing or malformed one refuses start by name: the kernel's, the state directory, the Mini
//! App's URL and the Bot API's base URL. Identity's two credentials, the owner's user id and the bot
//! token, are read through the kernel's loader with the redactor the log writer reads, so neither
//! value can reach a later line. The shutdown signal's handlers are installed, and the database
//! opens under the open lock. Then the loop starts: no webhook, the owner's menu, the drain, and
//! systemd hears `READY=1` as the first long poll is issued, the heartbeat following at once. On
//! SIGTERM the loop finishes the batch in hand and confirms its offset; the role says `STOPPING=1`,
//! closes the database and returns.
//!
//! The owner's `/sync` runs a sync cycle in this role (R11), through wiring's [`OwnerSyncCycle`],
//! which flushes the notification router joined to this role's transport (SPEC-041 R7, R13). The
//! compiled notification policy is read at start, and a policy it refuses refuses start by its key.

use std::cell::Cell;
use std::sync::Arc;

use deck_streak_bot::{ApiUrl, Commands, MiniAppUrl, Transport, TransportError};
use deck_streak_identity::owner::TELEGRAM_BOT_TOKEN;
use deck_streak_identity::{IdentityError, Owner};
use deck_streak_kernel::{
    Clock, CredentialLoader, CredentialsDirectory, Environment, KernelSettings, Offload, Redactor,
    SettingsError, SystemClock,
};
use deck_streak_notifications::{Policy, PolicyError};

use crate::lifecycle::{self, Notifier, NotifyState, ShutdownSignal};
use crate::wiring::{self, OwnerSyncCycle, StateDirectory, WiringError};

/// Why the `bot` role stopped with an error.
#[derive(Debug, thiserror::Error)]
pub enum BotRoleError {
    /// A setting refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// One of identity's credentials refused start, by its id.
    #[error(transparent)]
    Identity(#[from] IdentityError),
    /// The Bot API's client could not be built.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The shutdown signal's handlers could not be installed.
    #[error("the role could not install its shutdown signal handlers")]
    Signals(#[source] std::io::Error),
    /// The database could not be opened.
    #[error("the database could not be opened")]
    Database(#[source] WiringError),
    /// The compiled notification policy refused start, by its key.
    #[error(transparent)]
    Policy(#[from] PolicyError),
}

/// Runs the `bot` role until SIGTERM (or SIGINT), and returns once the batch in hand is handled
/// and its offset confirmed. `redactor` is the one the process's log writer reads: every
/// credential the role loads is registered with it.
///
/// # Errors
///
/// Every refusal of [`BotRoleError`], each before the first request to the Bot API.
pub async fn run(env: &Environment, redactor: &Redactor) -> Result<(), BotRoleError> {
    // Every role reads the kernel's settings, so a malformed one refuses start in each of them.
    let kernel = KernelSettings::from_env(env)?;
    let state = StateDirectory::from_env(env)?;
    let app = MiniAppUrl::from_env(env)?;
    let api = ApiUrl::from_env(env)?;
    let credentials = CredentialsDirectory::from_env(env)?;
    let loader = CredentialLoader::new(credentials, redactor.clone());
    let owner = Owner::load(&loader)?;
    let token = loader
        .load(TELEGRAM_BOT_TOKEN)
        .map_err(IdentityError::from)?;
    if token.expose().trim().is_empty() {
        return Err(IdentityError::Malformed {
            id: TELEGRAM_BOT_TOKEN,
            expected: "the bot's token, not blank",
        }
        .into());
    }
    let policy = Policy::compiled()?;
    let transport = Arc::new(Transport::new(&api, &token)?);
    let notifier = Notifier::from_env(env);
    let shutdown = ShutdownSignal::install().map_err(BotRoleError::Signals)?;

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let offload = Offload::new(kernel.offload_workers, clock);
    let db = wiring::open_database(&offload, &state)
        .await
        .map_err(BotRoleError::Database)?;
    let router = wiring::router(
        policy,
        db.clone(),
        kernel.study_day_rule,
        Arc::clone(&transport),
        owner,
    );
    let sync = OwnerSyncCycle::new(
        env.clone(),
        redactor.clone(),
        db.clone(),
        offload,
        kernel.study_day_rule,
    )
    .with_router(Arc::new(router));
    let mut commands = Commands::new(Arc::clone(&transport), owner, app, db.clone(), sync);

    let heartbeat = Cell::new(None);
    deck_streak_bot::run(&transport, &mut commands, shutdown.received(), || {
        tracing::info!("the bot role serves");
        notifier.notify(NotifyState::Ready);
        heartbeat.set(lifecycle::spawn_heartbeat(notifier.clone(), env));
    })
    .await;

    notifier.notify(NotifyState::Stopping);
    if let Some(heartbeat) = heartbeat.take() {
        heartbeat.abort();
    }
    db.close().await;
    tracing::info!("the bot role stopped");
    Ok(())
}
