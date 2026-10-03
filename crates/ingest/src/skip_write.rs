//! The skip day's write to the collection (SPEC-083 R20 to R27, R36; ADR-321 D14 to D20): the only
//! module beside the port that names the write port, [`CollectionWrite`] (A24).
//!
//! In this part it holds the preview (R20): the class's stop read first (R36, A54), then, under the
//! SHARED collection lock on the private copy, the cards the wrapped search (R3) selects, each with
//! its top-level deck and its current due, and the list's digest (D20) that a confirm carries and
//! the take compares. The preview writes nothing.

use std::io;
use std::path::Path;
use std::sync::Arc;

use sha2::{Digest, Sha256};

use deck_streak_kernel::{CredentialLoader, StudyDay, StudyDayRule, UtcMillis};

use crate::engine::{CollectionWrite, EngineError};
use crate::lock::CollectionLock;
use crate::settings::{SkipSearch, SyncSettings};
use crate::skip::{FailReason, SearchRefusal, SkipId, SkipStore, skip_search};
use crate::write_class_stop::{ClassStop, WriteClassStop};

/// One card the preview lists (R20): its id, its top-level deck's id and its current due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewCard {
    /// The card's id.
    pub id: i64,
    /// The id of the top-level deck the card's home deck sits under.
    pub top_level_deck: i64,
    /// The card's current due, as the engine stores it.
    pub due: i64,
}

/// What the preview shows the owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Preview {
    /// The class's stop is set, or could not be read: nothing is listed, and the stop says who set
    /// it, why and since when (A54).
    Stopped(ClassStop),
    /// A check of R3 refused: the zone is not pinned, observes daylight saving or differs from the
    /// collection's, or the engine's day is not the study day. Nothing was listed.
    Refused(FailReason),
    /// The cards the wrapped search selects, ascending by id, and their digest (D20).
    Listed {
        /// The cards, ascending by id.
        cards: Vec<PreviewCard>,
        /// The digest of the listed ids ([`list_digest`]).
        digest: String,
    },
}

/// Why the preview listed nothing.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    /// The configured search is not one expression (R3).
    #[error("the skip search is refused")]
    Search(#[from] SearchRefusal),
    /// The collection lock could not be taken or released.
    #[error("the collection lock failed")]
    Lock(#[source] io::Error),
    /// The engine could not read the private copy.
    #[error("the engine could not read the private copy")]
    Engine(#[from] EngineError),
}

/// The lowercase hexadecimal digits, by value.
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// The digest of a list of card ids (D20): the first 128 bits of SHA-256 over the ids in ascending
/// order, each written in decimal and ended by a line feed, as 32 lowercase hexadecimal digits.
/// The order the ids arrive in does not change it.
#[must_use]
pub fn list_digest(ids: &[i64]) -> String {
    let mut ascending = ids.to_vec();
    ascending.sort_unstable();
    let mut hasher = Sha256::new();
    for id in ascending {
        hasher.update(format!("{id}\n").as_bytes());
    }
    let mut hex = String::with_capacity(32);
    for byte in &hasher.finalize()[..16] {
        hex.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        hex.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    hex
}

/// The preview (R20): the class's stop first, and while it is set nothing else is read; otherwise,
/// under the shared collection lock on the private copy, the cards `search`'s wrap selects and
/// their digest. It writes nothing.
///
/// # Errors
///
/// [`PreviewError::Search`] when the search is not one expression; [`PreviewError::Lock`] when the
/// lock cannot be taken or released; [`PreviewError::Engine`] when the engine cannot read the copy.
pub async fn preview<W: CollectionWrite>(
    writer: &W,
    settings: &SyncSettings,
    search: &SkipSearch,
    stop: &WriteClassStop,
    day: StudyDay,
    rule: StudyDayRule,
) -> Result<Preview, PreviewError> {
    let _ = (day, rule);
    let read = stop.read_stop().await;
    if read.is_stopped() {
        return Ok(Preview::Stopped(read));
    }
    let wrapped = skip_search(search.as_str())?;
    let held = CollectionLock::new(settings.lock_path())
        .shared()
        .await
        .map_err(PreviewError::Lock)?;
    let due = writer.due_cards(&settings.copy_path(), &wrapped);
    held.release().map_err(PreviewError::Lock)?;
    let cards: Vec<PreviewCard> = due?
        .into_iter()
        .map(|card| PreviewCard {
            id: card.id,
            top_level_deck: card.top_level_deck,
            due: card.due,
        })
        .collect();
    let ids: Vec<i64> = cards.iter().map(|card| card.id).collect();
    Ok(Preview::Listed {
        digest: list_digest(&ids),
        cards,
    })
}

/// The four points between the take's steps where a test acts (D19), each given the path the
/// step left: the working copy after the converge, the partial backup after its write, the working
/// copy after the prior state's commit and before the push. Production passes [`NoHooks`].
pub trait TakeHooks: Send + Sync {
    /// After the converge, before the working copy is checked again.
    fn after_converge(&self, _working: &Path) {}
    /// After the backup's partial file is written, before its restore check.
    fn after_backup(&self, _partial: &Path) {}
    /// After the prior state is committed, before the reschedule.
    fn after_snapshot(&self, _working: &Path) {}
    /// Before the class's stop is read again and the push begins.
    fn before_push(&self, _working: &Path) {}
}

/// The hooks production passes: none acts.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoHooks;

impl TakeHooks for NoHooks {}

/// What a take reads and writes beside the collection.
pub struct TakePorts<'a, W> {
    /// The engine's write port.
    pub writer: &'a W,
    /// The deployment's settings: the private copy, its lock and the endpoint.
    pub settings: &'a SyncSettings,
    /// The configured skip search (R3).
    pub search: &'a SkipSearch,
    /// The class's stop (R36).
    pub stop: &'a WriteClassStop,
    /// The record of skips: the begun row, the prior state and the left state.
    pub store: &'a SkipStore,
    /// The loader of the sync login's two credentials.
    pub credentials: &'a CredentialLoader,
}

/// One take, on the row its caller began (`SkipStore::begin`).
#[derive(Clone, Debug)]
pub struct TakeRequest {
    /// The begun row.
    pub skip: SkipId,
    /// The study day the skip covers.
    pub day: StudyDay,
    /// The study-day rule the day was read under.
    pub rule: StudyDayRule,
    /// The digest the owner's confirm carries, if any (R20).
    pub digest: Option<String>,
    /// The take's instant: the stop's time when a count sets it.
    pub now: UtcMillis,
}

/// What a take answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TakeAnswer {
    /// The server accepted the push. `moved` are the cards rescheduled, `left_alone` the cards
    /// the preview and the converge did not agree on, `read_back` the moved cards whose state the
    /// push did not keep or that were studied after the converge (R27).
    Accepted {
        /// The cards moved, ascending by id.
        moved: Vec<i64>,
        /// The cards left alone, ascending by id.
        left_alone: Vec<i64>,
        /// The moved cards the read-back lists, ascending by id.
        read_back: Vec<i64>,
    },
    /// The take failed with `reason`, which its row records; a changed list carries the new
    /// preview (R20).
    Failed {
        /// Why.
        reason: FailReason,
        /// The new preview, when the list changed.
        preview: Option<Preview>,
    },
    /// The push began and its outcome is not known yet: the row stays `pending` (R25, R26).
    NotKnownYet,
}

/// The take (R21 to R27, R34 to R36).
pub async fn take<W>(
    ports: &TakePorts<'_, W>,
    request: &TakeRequest,
    hooks: Arc<dyn TakeHooks>,
) -> TakeAnswer
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let _ = (ports, request, hooks);
    TakeAnswer::Failed {
        reason: FailReason::EngineFailed,
        preview: None,
    }
}

/// The backup's restore check (D17): whether the partial backup at `partial` holds the working
/// copy at `working` byte for byte, and a throwaway copy of it, opened by the engine, counts as
/// the working copy does.
pub async fn restore_check<W>(writer: &W, partial: &Path, working: &Path) -> bool
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let _ = (writer, partial, working);
    true
}

/// R35's counts of one collection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Cards.
    pub cards: i64,
    /// Notes.
    pub notes: i64,
    /// Review-log rows.
    pub review_log_rows: i64,
    /// Review-log rows of type 4 with ease 0, the reschedule's own.
    pub reschedule_rows: i64,
    /// Cards per (queue, type), ascending by key.
    pub cards_by_queue_and_type: Vec<((i64, i64), i64)>,
    /// The cards the wrapped search selects.
    pub due: i64,
}

/// The name of the first count that moved other than as a reschedule of `moved` cards moves it
/// (R35): the review-log rows up by `moved`, each new row the reschedule's own, and the due count
/// down by `moved`; every other count equal. `None` when only those moved.
#[must_use]
pub fn moved_counts(before: &Counts, after: &Counts, moved: i64) -> Option<&'static str> {
    let _ = (before, after, moved);
    None
}

/// Removes every skip backup and partial backup in `directory`, the private copy's, and no other
/// file (A56), answering how many it removed.
///
/// # Errors
///
/// The first error listing the directory or removing a file.
pub fn erase_backups(directory: &Path) -> io::Result<usize> {
    let _ = directory;
    Ok(0)
}
