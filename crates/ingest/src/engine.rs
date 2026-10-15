//! The engine port: the one door through which this workspace reaches Anki's own Rust engine.
//!
//! ADR-009 chose Anki's engine for ingest and ADR-022 measured it before the choice was final.
//! Every engine type stays behind [`AnkiEngine`], so the rest of the workspace learns this crate's
//! own types and never Anki's (the anti-corruption layer of docs/CONTEXT-MAP.md). [`RslibEngine`]
//! is the adapter over the engine at upstream tag `26.09.3`, which the root manifest's `[patch]`
//! entry replaces with the maintainer's fork at revision
//! `2cfa70478a1174f49cf98fc71a8b8c47fc54b3fb`, the fork's tag `deckstreak-pin-26.09.3-wasm32-sync`:
//! that tag plus the fix that stops the engine's protobuf build script rerunning on every cargo
//! command (ADR-058, SPEC-055) and the web engine's `wasm32` patches, each gated off the native
//! build (ADR-348, SPEC-338), the browser's sync transport among them (SPEC-364).
//!
//! The skip day's write to the collection has a second port, [`CollectionWrite`], which only
//! [`RslibEngine`] implements and only the skip's write module names (SPEC-083 A24, ADR-321 D14):
//! the read port's test fakes never reach a card write or a push.
//!
//! Nothing here carries the engine's error text, a path, the endpoint or a credential out of the
//! port: a failure is one [`EngineError`] kind (SPEC-022 R9).

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::path::Path;

use anki::card::{CardId, CardQueueNumber};
use anki::collection::{Collection, CollectionBuilder};
use anki::decks::DeckId;
use anki::error::{AnkiError, DbErrorKind, NetworkErrorKind, SyncErrorKind};
use anki::search::SortMode;
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

/// What the engine's day is computed from, read without computing it (SPEC-083 R3). A day
/// computation in client mode first rewrites a configured UTC offset that differs from the
/// process's zone (the pinned engine's `rslib/src/scheduler/mod.rs:90-108`), so a skip reads these
/// before it lets the engine compute a day on any copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionFacts {
    /// The configured UTC offset in minutes WEST of UTC, as the engine stores it (UTC+05:30 is
    /// -330); `None` when the collection holds none, which a skip counts as an offset that
    /// differs, since the engine writes one at its first day computation.
    pub utc_offset_west: Option<i32>,
    /// The hour the engine's day rolls over at, local to the process's zone.
    pub rollover_hour: u8,
}

/// The engine's own day (SPEC-083 R3), as its scheduler computes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineDay {
    /// Whole days since the collection's creation: the frame the engine's due dates count in.
    pub days_elapsed: i64,
    /// When the engine's day ends, in epoch seconds: its next rollover.
    pub next_day_at: i64,
}

/// A card a search selected, with the scheduling state a skip lists, records and compares (SPEC-083
/// R20, R22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DueCard {
    /// The card's id.
    pub id: i64,
    /// The top-level deck of the card's home deck: its original deck while a filtered deck borrows
    /// it, else its deck.
    pub top_level_deck: i64,
    /// The card's due: for a review card, a day number in the engine's frame.
    pub due: i64,
    /// The card's queue.
    pub queue: i64,
    /// The card's type.
    pub kind: i64,
    /// The interval, in days.
    pub interval: i64,
    /// The ease factor, in permille.
    pub factor: i64,
    /// The original deck: zero unless a filtered deck borrows the card.
    pub original_deck: i64,
    /// The original due: zero unless a filtered deck borrows the card.
    pub original_due: i64,
    /// The card's modification time, in epoch seconds.
    pub mtime: i64,
}

/// What one push of the skip's write found (SPEC-083 R24, R25). Whether the engine's sync began
/// tells a failure that left the server's collection as it was from one whose outcome is not known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteSync {
    /// The server answered the exchange: it holds what the copy sent.
    Accepted,
    /// The server demands a full sync, which a skip never performs (R24). The engine answers the
    /// demand before it sends any change.
    FullSyncRequired {
        /// Whether a full download could satisfy the demand.
        download_ok: bool,
    },
    /// The copy could not be opened, or the login failed: the engine's sync never began, so nothing
    /// reached the server's collection.
    NotStarted(EngineError),
    /// The engine's sync began and failed: the server may hold the change (R25).
    Unknown(EngineError),
}

/// The skip's write port (SPEC-083 R21 to R27; ADR-321 D14): beside [`AnkiEngine`], which reads
/// and pulls, the one door to the engine's card writes and to a push of the copy's own changes.
/// Only [`RslibEngine`] implements it, so no test fake of the read port reaches it, and only the
/// skip's write module names it (A24). Each method opens and closes the collection itself, so a
/// test's hook between two steps can open the same file.
pub trait CollectionWrite {
    /// The configured UTC offset and the rollover hour of the collection at `collection`, read
    /// without a day computation.
    ///
    /// # Errors
    ///
    /// [`EngineError::CollectionLocked`] or [`EngineError::OpenFailed`] when the collection cannot
    /// be opened, and [`EngineError::EngineFailed`] when its scheduler has no rollover hour.
    fn facts(&self, collection: &Path) -> Result<CollectionFacts, EngineError>;

    /// The engine's day for the collection at `collection`. In client mode the engine first rewrites
    /// a configured UTC offset that differs from the process's zone, so a skip calls this only after
    /// [`CollectionWrite::facts`] matched.
    ///
    /// # Errors
    ///
    /// [`EngineError::CollectionLocked`] or [`EngineError::OpenFailed`] when the collection cannot
    /// be opened, and [`EngineError::EngineFailed`] when the scheduler fails.
    fn engine_day(&self, collection: &Path) -> Result<EngineDay, EngineError>;

    /// The cards `search` selects in the collection at `collection`, ascending by id, each with its
    /// scheduling state. A search on the due date computes the engine's day, so a skip calls this
    /// only after [`CollectionWrite::facts`] matched.
    ///
    /// # Errors
    ///
    /// [`EngineError::CollectionLocked`] or [`EngineError::OpenFailed`] when the collection cannot
    /// be opened, and [`EngineError::EngineFailed`] when the search or the read fails.
    fn due_cards(&self, collection: &Path, search: &str) -> Result<Vec<DueCard>, EngineError>;

    /// The engine's own Set Due Date over the cards `cards` with the day spec `spec`, which writes
    /// one review-log row of type 4 with ease 0 for each card it moves (R18, R23). It names no
    /// config key, so it records no setting.
    ///
    /// # Errors
    ///
    /// [`EngineError::CollectionLocked`] or [`EngineError::OpenFailed`] when the collection cannot
    /// be opened, and [`EngineError::EngineFailed`] when the reschedule fails; the engine's
    /// transaction then leaves every card as it was.
    fn set_due_date(&self, collection: &Path, cards: &[i64], spec: &str)
    -> Result<(), EngineError>;

    /// Logs in and runs one normal (incremental) sync of the copy at `collection`, media never
    /// synced: no retry, no full sync and no timer of its own (R24, R26).
    fn write_sync(
        &self,
        collection: &Path,
        login: &SyncLogin,
    ) -> impl Future<Output = WriteSync> + Send;
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

impl CollectionWrite for RslibEngine {
    fn facts(&self, collection: &Path) -> Result<CollectionFacts, EngineError> {
        let col = open(collection)?;
        let utc_offset_west = col.get_configured_utc_offset();
        let rollover_hour = col.rollover_for_current_scheduler().map_err(bounded);
        let closed = col.close(None).map_err(bounded);
        let rollover_hour = rollover_hour?;
        closed?;
        Ok(CollectionFacts {
            utc_offset_west,
            rollover_hour,
        })
    }

    fn engine_day(&self, collection: &Path) -> Result<EngineDay, EngineError> {
        let mut col = open(collection)?;
        let timing = col.timing_today().map_err(bounded);
        let closed = col.close(None).map_err(bounded);
        let timing = timing?;
        closed?;
        Ok(EngineDay {
            days_elapsed: i64::from(timing.days_elapsed),
            next_day_at: timing.next_day_at.0,
        })
    }

    fn due_cards(&self, collection: &Path, search: &str) -> Result<Vec<DueCard>, EngineError> {
        let mut col = open(collection)?;
        let cards = cards_of(&mut col, search);
        let closed = col.close(None).map_err(bounded);
        let cards = cards?;
        closed?;
        Ok(cards)
    }

    fn set_due_date(
        &self,
        collection: &Path,
        cards: &[i64],
        spec: &str,
    ) -> Result<(), EngineError> {
        let mut col = open(collection)?;
        let ids: Vec<CardId> = cards.iter().copied().map(CardId).collect();
        // No context key: with one the engine records the spec as a setting, and the reschedule
        // changes no setting (R23).
        let moved = col
            .set_due_date(&ids, spec, None)
            .map(|_| ())
            .map_err(bounded);
        let closed = col.close(None).map_err(bounded);
        moved?;
        closed
    }

    async fn write_sync(&self, collection: &Path, login: &SyncLogin) -> WriteSync {
        // The copy opens before the login, so a locked copy fails before any request.
        let mut col = match open(collection) {
            Ok(col) => col,
            Err(error) => return WriteSync::NotStarted(error),
        };
        let auth = match log_in(login).await {
            Ok(auth) => auth,
            Err(error) => return WriteSync::NotStarted(error),
        };
        let synced = col.normal_sync(auth, engine_client()).await;
        // The server's answer is the push's outcome: a close that fails after it cannot take back
        // what the server holds, and the read-back reopens the copy and reports what it finds.
        let _closed = col.close(None);
        match synced {
            Ok(output) => match output.required {
                SyncActionRequired::FullSyncRequired { download_ok, .. } => {
                    WriteSync::FullSyncRequired { download_ok }
                }
                _ => WriteSync::Accepted,
            },
            Err(error) => WriteSync::Unknown(bounded(error)),
        }
    }
}

/// The columns a skip reads of each card a search selected, by the ids in `?1`, a JSON array,
/// ascending by id. The engine's card type keeps these fields private, so they are read through the
/// engine's own connection, which its storage hands out for exactly this.
const CARD_STATE: &str = "SELECT id, did, odid, due, queue, type, ivl, factor, odue, mod \
     FROM cards WHERE id IN (SELECT value FROM json_each(?1)) ORDER BY id";

/// The cards `search` selects in `col`, ascending by id, each with its scheduling state and the
/// top-level deck of its home deck.
fn cards_of(col: &mut Collection, search: &str) -> Result<Vec<DueCard>, EngineError> {
    let selected = col
        .search_cards(search, SortMode::NoOrder)
        .map_err(bounded)?;
    let listed = selected
        .iter()
        .map(|CardId(id)| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let top_levels = top_level_decks(&col.get_all_deck_names(false).map_err(bounded)?);
    let mut statement = col
        .storage
        .db()
        .prepare(CARD_STATE)
        .map_err(|_| EngineError::EngineFailed)?;
    let rows = statement
        .query_map((format!("[{listed}]"),), |row| {
            let mut columns = [0_i64; 10];
            for (index, column) in columns.iter_mut().enumerate() {
                *column = row.get(index)?;
            }
            Ok(columns)
        })
        .map_err(|_| EngineError::EngineFailed)?;
    rows.map(|row| {
        let [
            id,
            deck,
            original_deck,
            due,
            queue,
            kind,
            interval,
            factor,
            original_due,
            mtime,
        ] = row.map_err(|_| EngineError::EngineFailed)?;
        let home = if original_deck == 0 {
            deck
        } else {
            original_deck
        };
        Ok(DueCard {
            id,
            top_level_deck: top_levels
                .get(&home)
                .copied()
                .ok_or(EngineError::EngineFailed)?,
            due,
            queue,
            kind,
            interval,
            factor,
            original_deck,
            original_due,
            mtime,
        })
    })
    .collect()
}

/// Every deck's id mapped to its top-level deck's id, from the engine's deck names, whose levels
/// are joined by `::`.
fn top_level_decks(names: &[(DeckId, String)]) -> HashMap<i64, i64> {
    let by_name: HashMap<&str, i64> = names
        .iter()
        .map(|(DeckId(id), name)| (name.as_str(), *id))
        .collect();
    names
        .iter()
        .filter_map(|(DeckId(id), name)| {
            let top = name.split("::").next()?;
            Some((*id, *by_name.get(top)?))
        })
        .collect()
}

/// A fresh HTTP client of the engine's own type, built by its `Default`: the HTTP/1 client of the
/// engine's feature set, as the engine's backend builds one for the predecessor. The type is
/// inferred from the engine's signature, so this crate names no HTTP library.
fn engine_client<Client: Default>() -> Client {
    Client::default()
}

/// Opens (or, when it does not exist yet, creates) the collection at `collection`.
pub(crate) fn open(collection: &Path) -> Result<Collection, EngineError> {
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
pub(crate) fn bounded(error: AnkiError) -> EngineError {
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

#[cfg(test)]
mod tests {
    use anki::error::{DbError, NetworkError, SyncError};

    use super::*;

    fn db(kind: DbErrorKind) -> AnkiError {
        AnkiError::DbError {
            source: DbError {
                info: String::from("a path the port must drop"),
                kind,
            },
        }
    }

    fn sync(kind: SyncErrorKind) -> AnkiError {
        AnkiError::SyncError {
            source: SyncError {
                info: String::from("an endpoint the port must drop"),
                kind,
            },
        }
    }

    fn network(kind: NetworkErrorKind) -> AnkiError {
        AnkiError::NetworkError {
            source: NetworkError {
                info: String::from("a host the port must drop"),
                kind,
            },
        }
    }

    #[test]
    fn a_locked_database_is_the_locked_collection_and_no_other_database_error_is() {
        assert_eq!(
            bounded(db(DbErrorKind::Locked)),
            EngineError::CollectionLocked
        );
        assert_eq!(bounded(db(DbErrorKind::Corrupt)), EngineError::EngineFailed);
        assert_eq!(bounded(db(DbErrorKind::Other)), EngineError::EngineFailed);
    }

    #[test]
    fn a_refused_login_is_auth_rejected_and_every_other_sync_error_is_a_server_error() {
        assert_eq!(
            bounded(sync(SyncErrorKind::AuthFailed)),
            EngineError::AuthRejected
        );
        assert_eq!(
            bounded(sync(SyncErrorKind::ServerError)),
            EngineError::ServerError
        );
        assert_eq!(
            bounded(sync(SyncErrorKind::Conflict)),
            EngineError::ServerError
        );
        assert_eq!(
            bounded(sync(SyncErrorKind::Other)),
            EngineError::ServerError
        );
    }

    #[test]
    fn a_network_timeout_is_a_timeout_and_every_other_network_error_is_unreachable() {
        assert_eq!(
            bounded(network(NetworkErrorKind::Timeout)),
            EngineError::Timeout
        );
        assert_eq!(
            bounded(network(NetworkErrorKind::Offline)),
            EngineError::NetworkUnreachable
        );
        assert_eq!(
            bounded(network(NetworkErrorKind::Other)),
            EngineError::NetworkUnreachable
        );
    }

    #[test]
    fn an_error_of_no_named_family_is_an_engine_failure() {
        assert_eq!(bounded(AnkiError::Interrupted), EngineError::EngineFailed);
    }

    #[test]
    fn a_login_prints_none_of_its_parts() {
        let login = SyncLogin::new("https://host.invalid/", "account", "secret");
        assert_eq!(format!("{login:?}"), "SyncLogin { .. }");
    }
}
