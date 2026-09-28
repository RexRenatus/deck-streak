//! The `data` role: the owner's export and erase, run by hand on the host (SPEC-021 R8; CHARTER 13,
//! 14; `docs/schematics/data-rights-export-and-erase.md`).
//!
//! `deckstreakd data export` writes the owner's export, one JSON object, as one line of standard
//! output: the one line there that does not open with a journal priority, and on success the only
//! line the role writes, so a copy redirected to a file is the document alone.
//! `deckstreakd data erase --confirm ERASE` erases every context's tables through coordination's
//! registry and logs what it did; any other form of the erase is refused by the binary with the
//! usage line and code 2, before the database is opened. Like every role it reads the kernel's
//! settings and the state directory, and opens the database under the open lock.
//!
//! The role is the owner's own act on the host: only a shell on the host, as the service's user,
//! reaches the database it opens. The bot's `/export` and `/delete` reach the same use cases behind
//! the owner gate (SPEC-024, SPEC-026).

use std::ffi::OsString;
use std::io::{self, Write};
use std::sync::Arc;

use deck_streak_coordination::data_rights_registry::{erase_all, export_all};
use deck_streak_daemon::wiring::{self, StateDirectory, WiringError};
use deck_streak_kernel::{
    Db, Environment, KernelError, KernelSettings, Offload, SettingsError, SystemClock,
};
use deck_streak_privacy::PrivacyError;

/// The word an erase runs with: the predecessor's (`server.py:erase_all_data` at `27ee2bc`).
pub const CONFIRMATION: &str = "ERASE";

/// What the role was asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataCommand {
    /// Write the owner's export to standard output.
    Export,
    /// Erase the owner's data.
    Erase,
}

impl DataCommand {
    /// The command the arguments after `data` name: `export` alone, or `erase --confirm ERASE`
    /// exactly. Any other erase names none, so the binary refuses it with the usage line and code 2
    /// before it opens the database.
    pub fn from_arguments(arguments: &[OsString]) -> Option<Self> {
        match arguments {
            [verb] if verb == "export" => Some(Self::Export),
            [verb, flag, word]
                if verb == "erase" && flag == "--confirm" && word == CONFIRMATION =>
            {
                Some(Self::Erase)
            }
            _ => None,
        }
    }
}

/// Why the `data` role stopped before it finished.
#[derive(Debug, thiserror::Error)]
pub enum DataRoleError {
    /// A setting refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The database could not be opened.
    #[error("the database could not be opened")]
    Database(#[source] WiringError),
    /// The export was refused or failed; nothing was written.
    #[error("the owner's data could not be exported")]
    Export(#[source] PrivacyError),
    /// The erase was refused or failed; its refusal says whether anything was erased.
    #[error("the owner's data could not be erased")]
    Erase(#[source] PrivacyError),
    /// The write to standard output could not run.
    #[error("the export's write to standard output did not complete")]
    Offload(#[source] KernelError),
    /// Standard output refused the export.
    #[error("standard output refused the export")]
    Output(#[source] io::Error),
}

/// Runs `command`, and returns the process's exit code.
///
/// # Errors
///
/// Every refusal of [`DataRoleError`]; each fails the run with code 1.
pub async fn run(env: &Environment, command: DataCommand) -> Result<u8, DataRoleError> {
    let kernel = KernelSettings::from_env(env)?;
    let state = StateDirectory::from_env(env)?;
    let offload = Offload::new(kernel.offload_workers, Arc::new(SystemClock));
    let db = wiring::open_database(&offload, &state)
        .await
        .map_err(DataRoleError::Database)?;
    let outcome = match command {
        DataCommand::Export => export(&db, &offload).await,
        DataCommand::Erase => erase(&db).await,
    };
    db.close().await;
    outcome.map(|()| 0)
}

/// Writes the export as one line of standard output, off the async runtime.
async fn export(db: &Db, offload: &Offload) -> Result<(), DataRoleError> {
    let exported = export_all(db).await.map_err(DataRoleError::Export)?;
    let mut line = exported.to_line();
    line.push('\n');
    offload
        .run("write the export to standard output", move || {
            let mut out = io::stdout().lock();
            out.write_all(line.as_bytes())?;
            out.flush()
        })
        .await
        .map_err(DataRoleError::Offload)?
        .map_err(DataRoleError::Output)
}

/// Erases the owner's data, and logs what the erase did.
async fn erase(db: &Db) -> Result<(), DataRoleError> {
    let erasure = erase_all(db).await.map_err(DataRoleError::Erase)?;
    tracing::info!(
        emptied = erasure.emptied.len(),
        reset = erasure.reset.len(),
        kept = erasure.kept.len(),
        checkpoint_busy = erasure.checkpoint_busy,
        "the owner's data was erased"
    );
    if erasure.checkpoint_busy {
        tracing::warn!(
            "a reader held the write-ahead log, so older frames of it remain until the next \
             maintenance checkpoint truncates it"
        );
    }
    Ok(())
}
