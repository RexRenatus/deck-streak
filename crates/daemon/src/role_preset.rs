//! The `preset` role: the owner's read of every preset, and the proposal that moves one preset to
//! the scheduler's defaults, run by hand on the host (SPEC-387 R4, R5, R7; ADR-401 D10).
//!
//! `deckstreakd preset list` prints every preset, the one with the most non-new cards first.
//! `deckstreakd preset propose <preset id>` records a proposal and prints it, with the steps the
//! owner takes in their own Anki app and the values that undo it. `deckstreakd preset verify
//! <proposal id>` settles an open proposal by what the private copy holds as of the last sync.
//! Every path reads the copy under the collection's shared lock through ingest's preset module and
//! writes nothing to the collection (R8). Like every role it reads the kernel's settings and the
//! state directory, and opens the database under the open lock.

use std::ffi::OsString;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::SystemTime;

use deck_streak_daemon::wiring::{self, StateDirectory, WiringError};
use deck_streak_ingest::engine::RslibEngine;
use deck_streak_ingest::preset::{self, PresetCommand, PresetError};
use deck_streak_ingest::settings::SyncSettings;
use deck_streak_kernel::{
    Environment, KernelError, KernelSettings, Offload, SettingsError, SystemClock, UtcMillis,
};

/// The command the arguments after `preset` name: `list` alone, or `propose` or `verify` and one
/// id. Any other shape names none, so the binary refuses it with the usage line and code 2 before
/// it opens the database.
pub fn command(arguments: &[OsString]) -> Option<PresetCommand> {
    let _ = arguments;
    None
}

/// Why the `preset` role stopped before it finished.
#[derive(Debug, thiserror::Error)]
pub enum PresetRoleError {
    /// A setting refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The database could not be opened.
    #[error("the database could not be opened")]
    Database(#[source] WiringError),
    /// The preset path refused or failed; nothing was written to the collection.
    #[error("the preset path stopped")]
    Preset(#[source] PresetError),
    /// The write to standard output could not run.
    #[error("the answer's write to standard output did not complete")]
    Offload(#[source] KernelError),
    /// Standard output refused the answer.
    #[error("standard output refused the answer")]
    Output(#[source] io::Error),
}

/// Runs `command`, prints its answer, and returns the process's exit code.
///
/// # Errors
///
/// Every refusal of [`PresetRoleError`]; each fails the run with code 1.
pub async fn run(env: &Environment, command: PresetCommand) -> Result<u8, PresetRoleError> {
    let kernel = KernelSettings::from_env(env)?;
    let state = StateDirectory::from_env(env)?;
    let sync = SyncSettings::from_env(env)?;
    let offload = Offload::new(kernel.offload_workers, Arc::new(SystemClock));
    let db = wiring::open_database(&offload, &state)
        .await
        .map_err(PresetRoleError::Database)?;
    let now = UtcMillis::from_system_time(SystemTime::now());
    let answered = preset::answer(&RslibEngine, &db, &sync, command, now).await;
    db.close().await;
    let mut text = answered.map_err(PresetRoleError::Preset)?;
    text.push('\n');
    offload
        .run("write the preset answer to standard output", move || {
            let mut out = io::stdout().lock();
            out.write_all(text.as_bytes())?;
            out.flush()
        })
        .await
        .map_err(PresetRoleError::Offload)?
        .map_err(PresetRoleError::Output)?;
    Ok(0)
}
