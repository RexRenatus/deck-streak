//! The engine port: the one door through which this workspace reaches Anki's own Rust engine.
//!
//! ADR-009 chose Anki's engine for ingest and ADR-022 measured it before the choice was final.
//! Every engine type stays behind [`AnkiEngine`], so the rest of the workspace learns this crate's
//! own types and never Anki's (the anti-corruption layer of docs/CONTEXT-MAP.md). [`RslibEngine`]
//! is the adapter over the engine at upstream tag `26.09.3`, which the root manifest's `[patch]`
//! entry replaces with the maintainer's fork at revision
//! `57382da085e6752738dc4bb617789be836a23300`: that tag plus one fix, so the engine's protobuf
//! build script stops rerunning on every cargo command (ADR-058, SPEC-055).
//!
//! Nothing here carries the engine's error text, a path, the endpoint or a credential out of the
//! port: a failure is one [`EngineError`] kind (SPEC-022 R9).

use std::fmt;
use std::future::Future;
use std::path::Path;

use anki::card::CardQueueNumber;
use anki::collection::{Collection, CollectionBuilder};
use anki::error::{AnkiError, DbErrorKind, NetworkErrorKind, SyncErrorKind};
use anki::sync::collection::normal::SyncActionRequired;
use anki::sync::login::{SyncAuth, sync_login};

/// How many queued cards the scheduler is asked for per top-level deck: the fetch limit the
/// predecessor's day-set query passes (`preread.py:_query_root_queued`, predecessor `27ee2bc`).
/// The day set's own delivery (#31) proves it against a golden.
pub const QUEUE_FETCH_LIMIT: usize = 1000;

/// Today's queued new cards of one top-level deck, as the engine's scheduler resolves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootQueue {
    /// The top-level deck's id in the collection.
    pub deck_id: i64,
    /// The new cards the scheduler returned, as card ids, in the scheduler's order.
    pub new_cards: Vec<i64>,
    /// The scheduler's own count of today's new cards for the deck. It exceeds the length of
    /// `new_cards` only when [`QUEUE_FETCH_LIMIT`] truncated the read, which a caller must treat
    /// as an incomplete queue rather than a complete one.
    pub new_count: usize,
}

/// Today's new-card queue of every top-level deck except the engine's built-in default deck.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewCardQueue {
    /// One entry per top-level deck, in the order the engine lists deck names.
    pub roots: Vec<RootQueue>,
}

impl NewCardQueue {
    /// Every new card queued today, across every top-level deck.
    #[must_use]
    pub fn new_card_total(&self) -> usize {
        self.roots.iter().map(|root| root.new_cards.len()).sum()
    }
}

/// Where, and as whom, the engine syncs. Its `Debug` prints none of the three values, so a login
/// that reaches a log line or a panic message leaks nothing (SPEC-022 R9).
#[derive(Clone)]
pub struct SyncLogin {
    endpoint: String,
    username: String,
    password: String,
}

impl SyncLogin {
    /// A login for the sync server at `endpoint` (an `http:` or `https:` URL).
    #[must_use]
    pub fn new(
        endpoint: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            username: username.into(),
            password: password.into(),
        }
    }
}

impl fmt::Debug for SyncLogin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SyncLogin { .. }")
    }
}

/// What one normal sync found, and so did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    /// Neither the copy nor the server had a change.
    NoChanges,
    /// The copy and the server exchanged their changes.
    Synced,
    /// The server demands a full sync. `download_ok` is false when the server holds no
    /// collection, so only a full upload could satisfy it, which ingest never performs.
    FullSyncRequired {
        /// Whether a full download can satisfy the demand.
        download_ok: bool,
    },
}

/// Why the engine failed, as one bounded kind. No variant carries text from the engine, so none
/// can carry a path, the endpoint or a credential (SPEC-022 R9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    /// Another process holds the collection.
    #[error("the collection is locked by another process")]
    CollectionLocked,
    /// The collection could not be opened or created.
    #[error("the collection could not be opened")]
    OpenFailed,
    /// The sync server refused the credentials.
    #[error("the sync server refused the credentials")]
    AuthRejected,
    /// The sync server could not be reached.
    #[error("the sync server could not be reached")]
    NetworkUnreachable,
    /// The sync server answered with an error.
    #[error("the sync server answered with an error")]
    ServerError,
    /// The sync server stopped answering within the engine's own timeout.
    #[error("the sync server stopped answering in time")]
    Timeout,
    /// Any other failure inside the engine.
    #[error("the engine failed")]
    EngineFailed,
}

/// The engine port. The sync methods return futures because the engine's sync is asynchronous;
/// the caller brings the runtime, and bounds each attempt with its own timer (SPEC-022 R8).
pub trait AnkiEngine {
    /// Opens the collection at `collection` and resolves today's new-card queue of every
    /// top-level deck, the way the predecessor's day-set query does.
    ///
    /// The scheduler builds a deck's queue for the current deck, so this sets the current deck
    /// of the collection it is given: call it on a copy that is never synced back.
    ///
    /// # Errors
    ///
    /// [`EngineError::CollectionLocked`] or [`EngineError::OpenFailed`] when the collection
    /// cannot be opened, and [`EngineError::EngineFailed`] when the scheduler fails.
    fn new_card_queue(&self, collection: &Path) -> Result<NewCardQueue, EngineError>;

    /// Logs in and runs one normal (incremental) sync of the copy at `collection`, media never
    /// synced.
    ///
    /// # Errors
    ///
    /// Every [`EngineError`] kind, by the failure's cause.
    fn normal_sync(
        &self,
        collection: &Path,
        login: &SyncLogin,
    ) -> impl Future<Output = Result<SyncOutcome, EngineError>> + Send;

    /// Logs in and replaces the copy at `collection` with the server's collection. The engine
    /// writes the download beside the copy and swaps it in only when it is complete and passes
    /// an integrity check.
    ///
    /// # Errors
    ///
    /// Every [`EngineError`] kind, by the failure's cause; the old copy is left as it was.
    fn full_download(
        &self,
        collection: &Path,
        login: &SyncLogin,
    ) -> impl Future<Output = Result<(), EngineError>> + Send;
}

/// The adapter over Anki's engine (`rslib`), at the upstream tag and the fork's revision the
/// workspace manifest pins (ADR-058).
#[derive(Debug, Clone, Copy, Default)]
pub struct RslibEngine;

impl AnkiEngine for RslibEngine {
    fn new_card_queue(&self, collection: &Path) -> Result<NewCardQueue, EngineError> {
        let mut col = open(collection)?;
        let queue = queue_of_every_root(&mut col);
        let closed = col.close(None).map_err(bounded);
        let queue = queue?;
        closed?;
        Ok(queue)
    }

    async fn normal_sync(
        &self,
        collection: &Path,
        login: &SyncLogin,
    ) -> Result<SyncOutcome, EngineError> {
        // The copy opens before the login, so a locked copy fails before any request (R8).
        let mut col = open(collection)?;
        let auth = log_in(login).await?;
        let before = col.sync_meta().map(|meta| meta.modified);
        let synced = col.normal_sync(auth, engine_client()).await;
        let after = col.sync_meta().map(|meta| meta.modified);
        let closed = col.close(None).map_err(bounded);
        let output = synced.map_err(bounded)?;
        let moved = before.map_err(bounded)? != after.map_err(bounded)?;
        closed?;
        Ok(match output.required {
            SyncActionRequired::FullSyncRequired { download_ok, .. } => {
                SyncOutcome::FullSyncRequired { download_ok }
            }
            // The engine reports a completed exchange as `NoChanges` too (`normal_sync_inner`
            // ends by setting it), so the collection's modified stamp tells the two apart: a
            // completed exchange always moves it to the server's new stamp (`finalize_sync`).
            _ if moved => SyncOutcome::Synced,
            _ => SyncOutcome::NoChanges,
        })
    }

    async fn full_download(&self, collection: &Path, login: &SyncLogin) -> Result<(), EngineError> {
        // The engine closes this collection, downloads beside it, checks the download's integrity
        // and renames it over the copy; a copy that does not exist yet starts as an empty one. It
        // opens before the login, so a locked copy fails before any request (R8).
        let col = open(collection)?;
        let auth = log_in(login).await?;
        col.full_download(auth, engine_client())
            .await
            .map_err(bounded)
    }
}

/// A fresh HTTP client of the engine's own type, built by its `Default`: the HTTP/1 client of the
/// engine's feature set, as the engine's backend builds one for the predecessor. The type is
/// inferred from the engine's signature, so this crate names no HTTP library.
fn engine_client<Client: Default>() -> Client {
    Client::default()
}

/// Opens (or, when it does not exist yet, creates) the collection at `collection`.
fn open(collection: &Path) -> Result<Collection, EngineError> {
    CollectionBuilder::new(collection)
        .build()
        .map_err(|error| match bounded(error) {
            EngineError::EngineFailed => EngineError::OpenFailed,
            kind => kind,
        })
}

/// Today's new-card queue of every top-level deck, as the predecessor's day-set query reads it:
/// select the deck, ask the scheduler for its queue, keep the new cards and the scheduler's own
/// new count (`preread.py:_query_root_queued`, predecessor `27ee2bc`).
fn queue_of_every_root(col: &mut Collection) -> Result<NewCardQueue, EngineError> {
    // `true` skips the engine's built-in default deck, which the predecessor skips by name.
    let decks = col.get_all_deck_names(true).map_err(bounded)?;
    let mut roots = Vec::new();
    for (deck_id, name) in decks {
        if name.contains("::") {
            continue;
        }
        col.set_current_deck(deck_id).map_err(bounded)?;
        let queued = col
            .get_queued_cards(QUEUE_FETCH_LIMIT, false)
            .map_err(bounded)?;
        let new_cards = queued
            .cards
            .iter()
            .filter(|entry| matches!(entry.card.queue_number(), CardQueueNumber::New))
            .map(|entry| entry.card.id().0)
            .collect();
        roots.push(RootQueue {
            deck_id: deck_id.0,
            new_cards,
            new_count: queued.new_count,
        });
    }
    Ok(NewCardQueue { roots })
}

/// Logs in for a host key. The engine's login answers the key alone, so the endpoint is set
/// here, normalised the way the engine's own settings conversion does (`join("./")`).
async fn log_in(login: &SyncLogin) -> Result<SyncAuth, EngineError> {
    let mut auth = sync_login(
        login.username.as_str(),
        login.password.as_str(),
        Some(login.endpoint.clone()),
        engine_client(),
    )
    .await
    .map_err(bounded)?;
    auth.endpoint = login.endpoint.parse().ok();
    auth.endpoint = auth.endpoint.and_then(|endpoint| endpoint.join("./").ok());
    if auth.endpoint.is_none() {
        return Err(EngineError::EngineFailed);
    }
    Ok(auth)
}

/// The engine's error as one bounded kind; its text, which can hold a path, is dropped here.
fn bounded(error: AnkiError) -> EngineError {
    match error {
        AnkiError::DbError { source } if source.kind == DbErrorKind::Locked => {
            EngineError::CollectionLocked
        }
        AnkiError::SyncError { source } if source.kind == SyncErrorKind::AuthFailed => {
            EngineError::AuthRejected
        }
        AnkiError::SyncError { .. } => EngineError::ServerError,
        AnkiError::NetworkError { source } if source.kind == NetworkErrorKind::Timeout => {
            EngineError::Timeout
        }
        AnkiError::NetworkError { .. } => EngineError::NetworkUnreachable,
        _ => EngineError::EngineFailed,
    }
}
