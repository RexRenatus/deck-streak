//! Ingest's settings: where the sync server is, and where the copy and its lock live (SPEC-022 R5,
//! R7, R13); which decks are read, and which of them are the law track (SPEC-023 R2, R3).
//!
//! The endpoint is a setting and never a secret: the account that logs in to it arrives as two
//! credentials through the kernel's loader. The state directory is the one systemd gives the unit
//! (`StateDirectory=`), so the copy survives a restart and nothing else writes there. Deck names are
//! the owner's private configuration, so the scope's `Debug` counts its prefixes and never prints
//! one.

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
/// The top-level deck-name prefixes whose decks are read, comma-separated; empty or unset reads every
/// deck (SPEC-023 R2).
pub const INCLUDE_DECKS: &str = "DECKSTREAK_INCLUDE_DECKS";
/// The top-level deck name the law track's decks start with, optional (SPEC-023 R3).
pub const LAW_DECK_ROOT: &str = "DECKSTREAK_LAW_DECK_ROOT";
/// The separator of a deck's name as the collection stores it: the text before the first one is the
/// deck's top-level name.
pub const DECK_SEPARATOR: char = '\x1f';

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

/// The include list (SPEC-023 R2): the top-level deck-name prefixes whose decks are read. Empty
/// reads every deck, as the predecessor's empty list did (`deck_filter.py:allowed_deck_ids`).
///
/// Its `Debug` counts the prefixes and never prints one: a deck name is the owner's private
/// configuration, and a log line or a panic message never carries it.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct IncludeDecks(Vec<String>);

impl IncludeDecks {
    /// The list of `prefixes`, each kept as given.
    #[must_use]
    pub fn new<P: Into<String>>(prefixes: impl IntoIterator<Item = P>) -> Self {
        Self(prefixes.into_iter().map(Into::into).collect())
    }

    /// The prefixes, in the order they were set.
    #[must_use]
    pub fn prefixes(&self) -> &[String] {
        &self.0
    }

    /// Whether the list is empty, and so reads every deck.
    #[must_use]
    pub fn reads_every_deck(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for IncludeDecks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IncludeDecks({} prefix(es))", self.0.len())
    }
}

impl Setting for IncludeDecks {
    const SHAPE: &'static str = "top-level deck-name prefixes, separated by commas";

    /// Every comma-separated part, trimmed, and none that is blank: the predecessor's reading of
    /// its include list (`config.py:_env_include_decks`).
    fn parse(text: &str) -> Option<Self> {
        Some(Self(
            text.split(',')
                .map(str::trim)
                .filter(|prefix| !prefix.is_empty())
                .map(str::to_owned)
                .collect(),
        ))
    }
}

/// The law root (SPEC-023 R3): a card whose top-level deck name starts with it is on the law track.
/// Its `Debug` never prints it.
#[derive(Clone, PartialEq, Eq)]
pub struct LawDeckRoot(String);

impl LawDeckRoot {
    /// The root, as set.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for LawDeckRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LawDeckRoot(..)")
    }
}

impl Setting for LawDeckRoot {
    const SHAPE: &'static str = "a top-level deck name";

    /// A top-level name: one with the deck separator in it names a subdeck, which no top-level
    /// name can start with.
    fn parse(text: &str) -> Option<Self> {
        (!text.contains(DECK_SEPARATOR)).then(|| Self(text.to_owned()))
    }
}

/// Which decks are read, and which of them are the law track (SPEC-023 R2, R3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScopeSettings {
    include: IncludeDecks,
    law_root: Option<LawDeckRoot>,
}

impl ScopeSettings {
    /// The scope of `include`, with `law_root` as the law track's root when one is set.
    #[must_use]
    pub const fn new(include: IncludeDecks, law_root: Option<LawDeckRoot>) -> Self {
        Self { include, law_root }
    }

    /// Reads the scope from `env`: both settings are optional.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] naming [`LAW_DECK_ROOT`] when the root holds the deck separator.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        Ok(Self {
            include: env.optional(INCLUDE_DECKS)?.unwrap_or_default(),
            law_root: env.optional(LAW_DECK_ROOT)?,
        })
    }

    /// The include list.
    #[must_use]
    pub const fn include(&self) -> &IncludeDecks {
        &self.include
    }

    /// The law root, when one is set.
    #[must_use]
    pub fn law_root(&self) -> Option<&str> {
        self.law_root.as_ref().map(LawDeckRoot::as_str)
    }

    /// Whether a non-empty include list has no prefix the law root starts with, so no deck of the
    /// law track is read.
    #[must_use]
    pub fn law_root_uncovered(&self) -> bool {
        match &self.law_root {
            Some(root) if !self.include.reads_every_deck() => !self
                .include
                .prefixes()
                .iter()
                .any(|prefix| root.as_str().starts_with(prefix.as_str())),
            _ => false,
        }
    }

    /// Logs one WARN naming [`INCLUDE_DECKS`] and [`LAW_DECK_ROOT`], never their values, when the
    /// include list covers no deck of the law root (R3, the predecessor's warning at
    /// `config.py:Settings.validate`): a language-only scope stays legal, and the log says what it
    /// costs.
    pub fn warn_if_law_root_uncovered(&self) {
        if self.law_root_uncovered() {
            tracing::warn!(
                include = INCLUDE_DECKS,
                law_root = LAW_DECK_ROOT,
                "no prefix of the include list covers the law root: no card of the law track is \
                 read until one does"
            );
        }
    }
}
