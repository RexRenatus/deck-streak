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

use deck_streak_kernel::{
    CourseCode, Courses, Db, ForeignDb, KernelError, Offload, Track, UtcMillis,
};
use sqlx::AssertSqlSafe;
use tokio::runtime::Handle;

use crate::lock::CollectionLock;
use crate::settings::{DECK_SEPARATOR, ScopeSettings, SyncSettings};

/// `SQLite`'s primary result code for a write refused because the database is read-only
/// (`SQLITE_READONLY`); every extended code of it keeps this in its low byte.
const SQLITE_READONLY: i32 = 8;

/// Every deck's id and stored name. The name is only ever read, by id: it is never compared,
/// ordered or aggregated in SQL (R4).
const DECK_NAMES: &str = "SELECT id, name FROM decks ORDER BY id";
/// The cards whose home deck (the original deck while a filtered deck borrows the card) is one of
/// the ids in `?1`, a JSON array: [`Card::home_deck_id`] in SQL, as the predecessor's recount
/// wrote it.
const CARDS: &str = "SELECT id, nid, did, odid, queue, type, due, ivl, factor, reps, lapses \
     FROM cards WHERE (CASE WHEN odid != 0 THEN odid ELSE did END) IN (SELECT value FROM json_each(?1)) \
     ORDER BY id";
/// The revlog rows newer than the floor `?1` of the cards whose home deck is one of the ids in
/// `?2`, oldest first. The study-event rule is applied to them here, not in SQL.
const REVIEWS: &str = "SELECT r.id, r.cid, r.ease, r.ivl, r.lastIvl, r.factor, r.time, r.type \
     FROM revlog r JOIN cards c ON c.id = r.cid \
     WHERE r.id > ?1 AND (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) \
     IN (SELECT value FROM json_each(?2)) ORDER BY r.id";
/// The collection's creation stamp, in epoch seconds (the predecessor's `read_config` reads
/// `col.crt`); no rollover hour is read with it (R7).
const CREATED: &str = "SELECT crt FROM col ORDER BY id LIMIT 1";

/// A card row as [`CARDS`] selects it.
type CardRow = (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64);
/// A review row as [`REVIEWS`] selects it.
type ReviewRow = (i64, i64, i64, i64, i64, i64, i64, i64);

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
    /// The card's course: the course whose deck root is its home deck's top-level name, or none
    /// (SPEC-071 R2). Only the code travels with the card.
    pub course: Option<CourseCode>,
    /// The Bloom tier of the card's note (SPEC-072 R3), reduced from its tags inside the read; the
    /// tags themselves never leave ingest.
    pub tier: Option<crate::tier::Tier>,
}

impl Card {
    /// The deck the card belongs to: the deck a filtered deck borrowed it from, else the deck it
    /// sits in (the predecessor's `types.py:Card.true_did`). Scope and track are decided by it.
    #[must_use]
    pub const fn home_deck_id(&self) -> i64 {
        if self.original_deck_id != 0 {
            self.original_deck_id
        } else {
            self.deck_id
        }
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
    deck_names
        .iter()
        .filter(|(_, name)| {
            prefixes.is_empty()
                || prefixes
                    .iter()
                    .any(|prefix| top_level(name).starts_with(prefix.as_str()))
        })
        .map(|(id, _)| *id)
        .collect()
}

/// The course of a card whose home deck is named `name` (SPEC-071 R2): the course whose deck root
/// EQUALS the name's top-level name, never one it only starts with (the predecessor's
/// `progress.py:language_of`), or none. The read's scope matches by prefix (R2 above); a course never
/// does, so a deck in scope can belong to no course.
#[must_use]
pub fn course_of(courses: &Courses, name: &str) -> Option<CourseCode> {
    let root = top_level(name);
    courses
        .courses()
        .iter()
        .find(|course| course.deck_root == root)
        .map(|course| course.code)
}

/// Whether a revlog row of type `kind` answered with `ease` is a study event (R2): a learn,
/// review, relearn or filtered answer of ease 1 or more, and never a manual or rescheduling entry
/// (the predecessor's `types.py:Review.is_study_event`).
#[must_use]
pub const fn is_study_event(kind: i64, ease: i64) -> bool {
    matches!(kind, 0..=3) && ease >= 1
}

/// The track of a card whose home deck's top-level name is `top_level_name` (R3): law when it
/// starts with the law root, else language.
#[must_use]
pub fn track_of(top_level_name: &str, law_root: Option<&str>) -> Track {
    match law_root {
        Some(root) if top_level_name.starts_with(root) => Track::Law,
        _ => Track::Language,
    }
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
    courses: Courses,
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
            courses: Courses::default(),
        }
    }

    /// This reader, giving each card it reads its course from `courses` (SPEC-071 R2), which the
    /// kernel loaded once at start.
    #[must_use]
    pub fn with_courses(mut self, courses: Courses) -> Self {
        self.courses = courses;
        self
    }

    /// The courses this reader gives each card its course from.
    #[must_use]
    pub const fn courses(&self) -> &Courses {
        &self.courses
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
        let prefixes = self.scope.include().prefixes().to_vec();
        let law_root = self.scope.law_root().map(str::to_owned);
        let courses = self.courses.clone();
        self.with_copy("read_collection", move |copy| async move {
            let deck_names = deck_names(&copy).await?;
            let scope = scope_ids(&allowed_deck_ids(&deck_names, &prefixes));
            let cards: Vec<CardRow> = sqlx::query_as(CARDS)
                .bind(&scope)
                .fetch_all(copy.reader())
                .await
                .map_err(read_failed)?;
            let reviews: Vec<ReviewRow> = sqlx::query_as(REVIEWS)
                .bind(floor)
                .bind(&scope)
                .fetch_all(copy.reader())
                .await
                .map_err(read_failed)?;
            let created: Option<i64> = sqlx::query_scalar(CREATED)
                .fetch_optional(copy.reader())
                .await
                .map_err(read_failed)?;
            let name_of = |home: i64| deck_names.get(&home).map_or("", String::as_str);
            let track = |home: i64| track_of(top_level(name_of(home)), law_root.as_deref());
            let cards = cards
                .into_iter()
                .map(|row| {
                    let (id, note_id, deck_id, original_deck_id, queue, kind) =
                        (row.0, row.1, row.2, row.3, row.4, row.5);
                    let mut card = Card {
                        id,
                        note_id,
                        deck_id,
                        original_deck_id,
                        queue,
                        kind,
                        due: row.6,
                        interval: row.7,
                        factor: row.8,
                        reps: row.9,
                        lapses: row.10,
                        track: Track::Language,
                        course: None,
                        tier: None,
                    };
                    card.track = track(card.home_deck_id());
                    card.course = course_of(&courses, name_of(card.home_deck_id()));
                    card
                })
                .collect();
            let reviews = reviews
                .into_iter()
                .filter(|row| is_study_event(row.7, row.2))
                .map(|row| Review {
                    id: row.0,
                    card_id: row.1,
                    ease: row.2,
                    interval: row.3,
                    last_interval: row.4,
                    factor: row.5,
                    taken_ms: row.6,
                    kind: row.7,
                })
                .collect();
            Ok(CollectionData {
                reviews,
                cards,
                created_at: UtcMillis::from_epoch_millis(created.unwrap_or(0).saturating_mul(1000)),
                deck_names,
            })
        })
        .await
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
        let statement = statement.to_owned();
        self.with_copy("execute_statement", move |copy| async move {
            // Audited: this door exists to run the caller's statement on the read-only connection,
            // which refuses any write, and it returns no row.
            sqlx::raw_sql(AssertSqlSafe(statement))
                .execute(copy.reader())
                .await
                .map(|done| done.rows_affected())
                .map_err(refusal)
        })
        .await
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

/// Every deck's id and stored name, from the open copy.
pub(crate) async fn deck_names(copy: &ForeignDb) -> Result<BTreeMap<i64, String>, ReadError> {
    let decks: Vec<(i64, String)> = sqlx::query_as(DECK_NAMES)
        .fetch_all(copy.reader())
        .await
        .map_err(read_failed)?;
    Ok(decks.into_iter().collect())
}

/// The ids of the decks read, as the JSON array a query's `json_each` expands: the ids are bound,
/// never written into the SQL.
pub(crate) fn scope_ids(allowed: &BTreeSet<i64>) -> String {
    let ids: Vec<String> = allowed.iter().map(i64::to_string).collect();
    format!("[{}]", ids.join(","))
}

/// A read's failure: the copy could not be read.
pub(crate) fn read_failed(error: sqlx::Error) -> ReadError {
    ReadError::Copy(KernelError::Database(error))
}

/// A statement's failure: a write the read-only copy refused, or any other failure of the copy.
fn refusal(error: sqlx::Error) -> ReadError {
    let read_only = error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .and_then(|code| code.parse::<i32>().ok())
        .is_some_and(|code| code & 0xff == SQLITE_READONLY);
    if read_only {
        ReadError::WriteRefused
    } else {
        read_failed(error)
    }
}
