//! The archive's listing for the snapshot answer (SPEC-377 R14; ADR-388 D16): the list command the
//! deployment names, run with its arguments and no shell, bounded in time and in output.
//!
//! This is the red stub: the lister is wired to nothing, and answers nothing.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use deck_streak_api::snapshot_routes::SnapshotLister;
use deck_streak_kernel::{CredentialsDirectory, Environment, Setting, SettingsError};

/// The setting naming the list command: an absolute program and its arguments, split on
/// whitespace. It carries no credential.
pub const ARCHIVE_LIST_COMMAND: &str = "DECKSTREAK_ARCHIVE_LIST_COMMAND";
/// The role name of the archive's list-only credential in the service's credential directory.
pub const ARCHIVE_LIST_CREDENTIAL: &str = "archive-list";
/// The longest the command runs before it is killed: under the stack's request timeout.
pub const TIME_BOUND: Duration = Duration::from_secs(8);
/// The most bytes of the command's output read.
pub const OUTPUT_BOUND: usize = 65_536;

/// The list command's words: an absolute program and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListCommand {
    program: PathBuf,
    arguments: Vec<String>,
}

impl Setting for ListCommand {
    const SHAPE: &'static str = "an absolute program and its arguments";

    fn parse(text: &str) -> Option<Self> {
        let mut words = text.split_whitespace();
        let program = PathBuf::from(words.next()?);
        program.is_absolute().then(|| Self {
            program,
            arguments: words.map(str::to_owned).collect(),
        })
    }
}

/// The list command as the snapshot route's lister.
#[derive(Clone, Debug)]
pub struct CommandLister {
    command: ListCommand,
    credential: PathBuf,
    time: Duration,
    output: usize,
}

impl CommandLister {
    /// The lister running `command` with the credential file `credential` as its last argument,
    /// killed at `time` and read to `output` bytes.
    #[must_use]
    pub const fn bounded(
        command: ListCommand,
        credential: PathBuf,
        time: Duration,
        output: usize,
    ) -> Self {
        Self {
            command,
            credential,
            time,
            output,
        }
    }

    /// The lister the role's settings and credentials configure: none while the list command's
    /// setting or the archive's list-only credential is absent.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] when the setting is set and its program is not absolute.
    pub fn configured(
        env: &Environment,
        credentials: &CredentialsDirectory,
    ) -> Result<Option<Self>, SettingsError> {
        let Some(command) = env.optional::<ListCommand>(ARCHIVE_LIST_COMMAND)? else {
            return Ok(None);
        };
        let credential = credentials.path().join(ARCHIVE_LIST_CREDENTIAL);
        Ok(Some(Self::bounded(
            command,
            credential,
            TIME_BOUND,
            OUTPUT_BOUND,
        )))
    }
}

impl SnapshotLister for CommandLister {
    fn list(&self) -> Pin<Box<dyn Future<Output = Option<Vec<String>>> + Send + '_>> {
        // The red stub: the lister is wired to nothing.
        let _ = (&self.command, &self.credential, self.time, self.output);
        Box::pin(async { None })
    }
}
