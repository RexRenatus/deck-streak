//! Ingest's settings: where the sync server is, and where the copy and its lock live (SPEC-022 R5,
//! R7, R13).
//!
//! The endpoint is a setting and never a secret: the account that logs in to it arrives as two
//! credentials through the kernel's loader. The state directory is the one systemd gives the unit
//! (`StateDirectory=`), so the copy survives a restart and nothing else writes there.

use std::fmt;
use std::path::PathBuf;

use deck_streak_kernel::{Environment, Setting, SettingsError};

/// The sync server's URL (required).
pub const SYNC_ENDPOINT: &str = "DECKSTREAK_SYNC_ENDPOINT";
/// The directory systemd passes a unit with `StateDirectory=`; the copy and its lock live there.
pub const STATE_DIRECTORY: &str = "STATE_DIRECTORY";
/// The credential that holds the sync account's username.
pub const SYNC_USERNAME: &str = "anki-sync-username";
/// The credential that holds the sync account's password.
pub const SYNC_PASSWORD: &str = "anki-sync-password";
/// The copy's file name in the state directory.
pub const COPY_FILE: &str = "collection.anki2";
/// The collection lock's file name in the state directory (R7).
pub const LOCK_FILE: &str = "collection.lock";

/// The sync server's URL. Its `Debug` prints the scheme alone: the address is private
/// configuration, and a log line or a panic message never carries it.
#[derive(Clone, PartialEq, Eq)]
pub struct SyncEndpoint(String);

impl SyncEndpoint {
    /// The URL, for the engine's login and nothing else.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the endpoint is plain `http:`, which sends the credential in the clear (R13).
    #[must_use]
    pub fn is_cleartext(&self) -> bool {
        self.0.starts_with("http://")
    }
}

impl fmt::Debug for SyncEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SyncEndpoint(..)")
    }
}

impl Setting for SyncEndpoint {
    const SHAPE: &'static str = "an http: or https: URL that names a host";

    fn parse(text: &str) -> Option<Self> {
        let (scheme, rest) = text.split_once("://")?;
        let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let named =
            !host.is_empty() && !host.starts_with(':') && !host.contains(char::is_whitespace);
        (matches!(scheme, "http" | "https") && named).then(|| Self(text.to_owned()))
    }
}

/// The state directory: the first of the absolute paths systemd lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDirectory(PathBuf);

impl Setting for StateDirectory {
    const SHAPE: &'static str = "an absolute path";

    fn parse(text: &str) -> Option<Self> {
        let first = PathBuf::from(text.split(':').next().unwrap_or_default());
        first.is_absolute().then_some(Self(first))
    }
}

/// Where the sync server is and where the copy lives.
#[derive(Clone, Debug)]
pub struct SyncSettings {
    endpoint: SyncEndpoint,
    state: StateDirectory,
}

impl SyncSettings {
    /// Reads the settings from `env`, refusing a missing or malformed one by name.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Missing`] naming [`SYNC_ENDPOINT`] or [`STATE_DIRECTORY`] when one is not
    /// set, and [`SettingsError::Malformed`] naming the setting when it has another shape.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        Ok(Self {
            endpoint: env.required(SYNC_ENDPOINT)?,
            state: env.required(STATE_DIRECTORY)?,
        })
    }

    /// The sync server's URL.
    #[must_use]
    pub const fn endpoint(&self) -> &SyncEndpoint {
        &self.endpoint
    }

    /// The copy of the collection.
    #[must_use]
    pub fn copy_path(&self) -> PathBuf {
        self.state.0.join(COPY_FILE)
    }

    /// The collection lock (R7).
    #[must_use]
    pub fn lock_path(&self) -> PathBuf {
        self.state.0.join(LOCK_FILE)
    }

    /// Logs one WARN naming [`SYNC_ENDPOINT`], never its value, when the endpoint is plain
    /// `http:` (R13): the transport is the owner's decision, and the log says what it costs.
    pub fn warn_if_cleartext(&self) {
        if self.endpoint.is_cleartext() {
            tracing::warn!(
                setting = SYNC_ENDPOINT,
                "the sync endpoint is plain http: the sync credential crosses the network in the \
                 clear"
            );
        }
    }
}
