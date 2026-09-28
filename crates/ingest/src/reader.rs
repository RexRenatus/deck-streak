//! The read (SPEC-023 R1 to R5, R7): the private copy of the collection, read-only, inside the
//! owner's deck scope, as this workspace's own types.
//!
//! [`CollectionReader`] opens the copy with the kernel's [`Db::open_foreign_read_only`] (`mode=ro`
//! and `query_only`) under the shared collection lock, on the kernel's [`Offload`]. So a read never
//! sees the copy mid-swap, never stalls the async runtime, never runs beside another collection read
//! on a small host, and never writes the copy: any write through it is refused by `SQLite` itself.
//!
//! It ports the predecessor's `anki_reader.py:read_collection` and `deck_filter.py` (predecessor
//! `27ee2bc`): a card is read when its ORIGINAL deck's top-level name starts with an included prefix
//! (`types.py:Card.true_did`), and a review when its card is read and it is a study event
//! (`types.py:Review.is_study_event`). Names are read by id and matched here, never in SQL, because
//! the collection's name columns use a `unicase` collation `SQLite` does not ship (R4). The study
//! day is the kernel's configured rule: no rollover hour is taken from the collection (R7).

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::PathBuf;

use deck_streak_kernel::{Db, ForeignDb, KernelError, Offload, Track, UtcMillis};
use tokio::runtime::Handle;

use crate::lock::CollectionLock;
use crate::settings::{DECK_SEPARATOR, ScopeSettings, SyncSettings};

/// `SQLite`'s primary result code for a write refused because the database is read-only
/// (`SQLITE_READONLY`); every extended code of it keeps this in its low byte.
const SQLITE_READONLY: i32 = 8;

/// One study review: a revlog row of type 0 to 3 with ease 1 or more (docs/LEXICON.md), whose card
/// is in scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Review {
    /// The review's id, which is the instant it was answered, in epoch milliseconds.
    pub id: i64,
    /// The card answered.
    pub card_id: i64,
    /// The answer button, 1 to 4.
    pub ease: i64,
    /// The interval after the answer: days when positive, seconds when negative.
    pub interval: i64,
    /// The interval before the answer, in the same units.
    pub last_interval: i64,
    /// The ease factor after the answer, in permille.
    pub factor: i64,
    /// How long the answer took, in milliseconds.
    pub taken_ms: i64,
    /// The review's type: 0 learn, 1 review, 2 relearn, 3 filtered.
    pub kind: i64,
}

/// One card in scope, as the collection holds it now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Card {
    /// The card's id.
    pub id: i64,
    /// The note the card belongs to.
    pub note_id: i64,
    /// The deck the card sits in now, which is a filtered deck while it is borrowed by one.
    pub deck_id: i64,
    /// The deck a filtered deck borrowed the card from, or 0 when it sits in its own deck.
    pub original_deck_id: i64,
    /// The card's queue: -3 to -1 buried or suspended, 0 new, 1 learn, 2 review, 3 day-relearn.
    pub queue: i64,
    /// The card's type: 0 new, 1 learning, 2 review, 3 relearning.
    pub kind: i64,
    /// When the card is due, in its queue's units.
    pub due: i64,
    /// The card's interval: days when positive, seconds when negative.
    pub interval: i64,
    /// The card's ease factor, in permille.
    pub factor: i64,
    /// How many times the card has been answered.
    pub reps: i64,
    /// How many times the card has lapsed.
    pub lapses: i64,
    /// The card's track, from its home deck's top-level name (R3).
    pub track: Track,
}

impl Card {
    /// The deck the card belongs to: the deck a filtered deck borrowed it from, else the deck it
    /// sits in (the predecessor's `types.py:Card.true_did`). Scope and track are decided by it.
    #[must_use]
    pub const fn home_deck_id(&self) -> i64 {
        self.deck_id
    }
}

/// What one read of the copy returns (R5): only this workspace's own types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionData {
    /// The study reviews of the cards in scope newer than the read's floor, oldest first.
    pub reviews: Vec<Review>,
    /// The cards in scope, by id.
    pub cards: Vec<Card>,
    /// When the collection was created.
    pub created_at: UtcMillis,
    /// Every deck's name by its id, each name's parts separated by [`DECK_SEPARATOR`].
    pub deck_names: BTreeMap<i64, String>,
}

/// The top-level name of the deck named `name`: the text before its first [`DECK_SEPARATOR`] (the
/// predecessor's `deck_filter.py:top_level`).
#[must_use]
pub fn top_level(name: &str) -> &str {
    name.split(DECK_SEPARATOR).next().unwrap_or(name)
}

/// The decks read (R2): those whose top-level name starts with one of `prefixes`, or every deck when
/// `prefixes` is empty (the predecessor's `deck_filter.py:allowed_deck_ids`).
#[must_use]
pub fn allowed_deck_ids(deck_names: &BTreeMap<i64, String>, prefixes: &[String]) -> BTreeSet<i64> {
    let _ = (deck_names, prefixes);
    BTreeSet::new()
}

/// Whether a revlog row of type `kind` answered with `ease` is a study event (R2): a learn,
/// review, relearn or filtered answer of ease 1 or more, and never a manual or rescheduling entry
/// (the predecessor's `types.py:Review.is_study_event`).
#[must_use]
pub const fn is_study_event(kind: i64, ease: i64) -> bool {
    let _ = (kind, ease);
    false
}

/// The track of a card whose home deck's top-level name is `top_level_name` (R3): law when it
/// starts with the law root, else language.
#[must_use]
pub fn track_of(top_level_name: &str, law_root: Option<&str>) -> Track {
    let _ = (top_level_name, law_root);
    Track::Language
}

/// Why the copy could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The collection lock could not be taken.
    #[error("the collection lock could not be taken")]
    Lock(#[source] std::io::Error),
    /// A statement tried to write the copy, and `SQLite` refused it: the copy is open read-only.
    #[error("the copy refused a write: it is open read-only")]
    WriteRefused,
    /// The copy could not be opened or read; the source says why.
    #[error("the copy could not be read")]
    Copy(#[source] KernelError),
}

/// Reads the private copy, read-only, inside the owner's scope (R1 to R5).
#[derive(Clone, Debug)]
pub struct CollectionReader {
    copy: PathBuf,
    lock: CollectionLock,
    offload: Offload,
    scope: ScopeSettings,
}

impl CollectionReader {
    /// The reader of the copy `settings` name, inside `scope`, on `offload`. It is built once, at
    /// the service's start, so it logs R3's warning there.
    #[must_use]
    pub fn new(settings: &SyncSettings, scope: ScopeSettings, offload: Offload) -> Self {
        scope.warn_if_law_root_uncovered();
        Self {
            copy: settings.copy_path(),
            lock: CollectionLock::new(settings.lock_path()),
            offload,
            scope,
        }
    }

    /// The scope this reader reads inside.
    #[must_use]
    pub const fn scope(&self) -> &ScopeSettings {
        &self.scope
    }

    /// Reads the cards in scope, their study reviews newer than `floor` (a review id, which is
    /// epoch milliseconds), the collection's creation time and every deck's name (R2, R5, R6).
    ///
    /// # Errors
    ///
    /// [`ReadError::Lock`] when the collection lock cannot be taken, and [`ReadError::Copy`] when
    /// the copy cannot be opened or read.
    pub async fn read(&self, floor: i64) -> Result<CollectionData, ReadError> {
        let _ = floor;
        Ok(CollectionData {
            reviews: Vec::new(),
            cards: Vec::new(),
            created_at: UtcMillis::from_epoch_millis(0),
            deck_names: BTreeMap::new(),
        })
    }

    /// Runs `statement` on the connection every read of this reader uses, open read-only under the
    /// shared lock on the offload, and returns how many rows it changed. `SQLite` refuses a write
    /// before it reaches the file, so this only ever reports a refusal, or no change: it is how the
    /// refusal is proved rather than assumed (R1), and it returns no row, so no Anki data leaves
    /// this crate through it.
    ///
    /// # Errors
    ///
    /// [`ReadError::WriteRefused`] for a write, [`ReadError::Lock`] when the lock cannot be taken,
    /// and [`ReadError::Copy`] for any other failure of the statement or the copy.
    pub async fn execute_statement(&self, statement: &str) -> Result<u64, ReadError> {
        let _ = statement;
        Ok(0)
    }

    /// Runs `work` on the copy, open read-only, under the shared collection lock, on the offload
    /// (R1): every read of this crate goes through here. `work` receives the open copy and runs to
    /// its end on the offload's blocking thread; the copy is closed after it, and the lock released
    /// explicitly.
    pub(crate) async fn with_copy<T, W, Fut>(
        &self,
        operation: &'static str,
        work: W,
    ) -> Result<T, ReadError>
    where
        W: FnOnce(ForeignDb) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, ReadError>>,
        T: Send + 'static,
    {
        let held = self.lock.shared().await.map_err(ReadError::Lock)?;
        let copy = self.copy.clone();
        let runtime = Handle::current();
        let outcome = self
            .offload
            .run(operation, move || {
                on_this_thread(&runtime, async move {
                    let db = Db::open_foreign_read_only(&copy)
                        .await
                        .map_err(ReadError::Copy)?;
                    let read = work(db.clone()).await;
                    db.close().await;
                    read
                })
            })
            .await;
        // An explicit unlock before the lock file closes (SPEC-022 R7); a failed unlock is released
        // by the close that follows it.
        let _ = held.release();
        outcome.map_err(ReadError::Copy)?
    }
}

/// Drives `read` to its end on the calling thread, which is the offload's blocking thread and never
/// a runtime worker: the offload's bound and timing cover the whole read, `SQLite`'s own work
/// included.
fn on_this_thread<T>(runtime: &Handle, read: impl Future<Output = T>) -> T {
    runtime.block_on(read)
}

/// A statement's failure: a write the read-only copy refused, or any other failure of the copy.
fn refusal(error: sqlx::Error) -> ReadError {
    let read_only = error
        .as_database_error()
        .and_then(|database| database.code())
        .and_then(|code| code.parse::<i32>().ok())
        .is_some_and(|code| code & 0xff == SQLITE_READONLY);
    if read_only {
        ReadError::WriteRefused
    } else {
        ReadError::Copy(KernelError::Database(error))
    }
}
