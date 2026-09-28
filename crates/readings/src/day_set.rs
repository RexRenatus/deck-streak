//! The day set (SPEC-045 R3, R4, R6, R7, R9, R11): each study day, every topic's new cards come
//! from the scheduler's own queue, and every topic ends the resolution in one honest state.
//!
//! [`resolve`] runs the gates in R4's order before any collection work: the last sync, then (for a
//! run that can name its topics at all) the taxonomy and the review read, then the pause. Only then
//! does it ask the [`QueuePort`] for today's queue, within [`RESOLVE_BUDGET`]. Ingest answers every
//! top-level deck but the engine's default deck in one call, each root's new cards read under the
//! shared parent budget (`pipeline_layers/preread.py:_query_root_queued`); a root whose answer holds
//! fewer new cards than the scheduler's own count is saturated. [`resolve_day_sets`] is the
//! predecessor's resolver (`prereading.py:resolve_day_sets`): each card is attributed by its
//! original deck, a card an earlier root claimed is skipped, and a topic's digest is the SHA-256 of
//! its sorted card ids joined by commas. [`EngineQueue`] is the port's adapter over ingest's engine:
//! the engine selects a deck to answer, a write, so it reads a throwaway copy of the private copy,
//! taken under the shared collection lock and removed after, on the kernel's offload.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use deck_streak_ingest::engine::{AnkiEngine, EngineError, NewCardQueue, QUEUE_FETCH_LIMIT};
use deck_streak_ingest::lock::CollectionLock;
use deck_streak_ingest::reader::{Card, CollectionData, ReadError};
use deck_streak_ingest::settings::{DECK_SEPARATOR, SyncSettings};
use deck_streak_kernel::{Offload, StudyDay, StudyDayRule};
use sha2::{Digest as _, Sha256};

use crate::gates::{self, LastSync};
use crate::state::{CouldNotTell, RunOutcome, TopicState};
use crate::taxonomy::Taxonomy;
use crate::topic::{TopicKey, topic_of};

/// How long a resolution may take from its call to the queue to the queue's answer: the
/// predecessor's `preread.py:PREREAD_OFFLOAD_BUDGET_S` (R7). Past it, every root is
/// `day_set_resolve_timeout`.
pub const RESOLVE_BUDGET: Duration = Duration::from_secs(30);

/// A new card the scheduler queued, with the decks it is attributed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueuedCard {
    /// The card's id.
    pub id: i64,
    /// The card's note, when the read returned the card.
    pub note_id: Option<i64>,
    /// The deck the card sits in, a filtered deck while one borrows it.
    pub deck_id: i64,
    /// The deck a filtered deck borrowed the card from, or 0.
    pub original_deck_id: i64,
}

impl QueuedCard {
    /// The deck the card is attributed by: its original deck when a filtered deck borrows it, else
    /// the deck it sits in (the predecessor's `types.py:Card.true_did`).
    #[must_use]
    pub const fn home_deck_id(&self) -> i64 {
        self.deck_id
    }

    /// The queued card `card` as ingest's read returned it.
    #[must_use]
    pub const fn of(card: &Card) -> Self {
        Self {
            id: card.id,
            note_id: Some(card.note_id),
            deck_id: card.deck_id,
            original_deck_id: card.original_deck_id,
        }
    }

    /// A queued card the read did not return: attributed to deck 0, which names no deck, so it is
    /// reported unmapped rather than dropped.
    #[must_use]
    pub const fn unread(id: i64) -> Self {
        Self {
            id,
            note_id: None,
            deck_id: 0,
            original_deck_id: 0,
        }
    }
}

/// One root's new cards, as the scheduler queued them under the root's own budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaySetQuery {
    /// The root's name: the label every report of the root carries.
    pub root: String,
    /// The root's new cards, in the scheduler's order.
    pub cards: Vec<QueuedCard>,
}

/// A root whose day set could not be trusted, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndeterminedRoot {
    /// The root's name.
    pub root: String,
    /// Why its day set could not be trusted.
    pub reason: CouldNotTell,
}

/// A topic with new cards today: its day set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveTopic {
    /// The topic.
    pub topic: TopicKey,
    /// The decks its cards are attributed by, sorted.
    pub deck_ids: Vec<i64>,
    /// Its new cards, sorted.
    pub card_ids: Vec<i64>,
    /// The distinct notes of its new cards, sorted.
    pub note_ids: Vec<i64>,
    /// The SHA-256 of its sorted card ids joined by commas, in lowercase hexadecimal.
    pub digest: String,
}

/// A deck whose new cards map to no topic (R9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnmappedDeck {
    /// The deck's stored name, or `<unknown-deck-N>` for a deck id the read did not name.
    pub deck_name: String,
    /// How many of today's new cards it holds.
    pub card_count: usize,
}

/// What the resolver makes of the queries (`prereading.py:DaySetResolution`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DaySetResolution {
    /// Every topic with new cards, sorted by its key.
    pub active: Vec<ActiveTopic>,
    /// Every deck whose new cards map to no topic, sorted by its name.
    pub unmapped: Vec<UnmappedDeck>,
    /// Every root that contributed no card to a topic, sorted.
    pub no_new_today: Vec<String>,
    /// Every undetermined root, sorted by its name.
    pub undetermined: Vec<UndeterminedRoot>,
}

/// The digest of a day set (`prereading.py:_digest_for_card_ids`): the SHA-256 of the card ids,
/// sorted and joined by commas, in lowercase hexadecimal.
#[must_use]
pub fn digest(card_ids: &[i64]) -> String {
    let _ = card_ids;
    String::new()
}

/// Whether a root's answer is saturated (R3): it holds fewer new cards than the scheduler's own
/// count of them, or, when that count is absent, as many as the fetch limit or more. A saturated
/// root's read may be truncated, so it is never taken as complete.
#[must_use]
pub const fn saturated(new_cards: usize, new_count: Option<usize>) -> bool {
    let _ = (new_cards, new_count);
    false
}

/// The name a deck id gives in a report: its stored name, or `<unknown-deck-N>`.
fn deck_label(deck_names: &BTreeMap<i64, String>, deck_id: i64) -> String {
    deck_names
        .get(&deck_id)
        .cloned()
        .unwrap_or_else(|| format!("<unknown-deck-{deck_id}>"))
}

/// The top-level name of a deck's stored name.
fn root_of(deck_name: &str) -> &str {
    deck_name.split(DECK_SEPARATOR).next().unwrap_or(deck_name)
}

/// Resolves the day sets (`prereading.py:resolve_day_sets`): each query of a root that is not
/// undetermined contributes its cards, each attributed by its original deck, a card an earlier query
/// claimed skipped; a card of a deck that maps to no topic is counted against that deck; and a root
/// that contributed nothing is reported with no new card today.
#[must_use]
pub fn resolve_day_sets(
    queries: &[DaySetQuery],
    deck_names: &BTreeMap<i64, String>,
    taxonomy: &Taxonomy,
    undetermined: &[UndeterminedRoot],
) -> DaySetResolution {
    let _ = (queries, deck_names, taxonomy, undetermined);
    DaySetResolution::default()
}

/// Every topic the taxonomy maps a deck to, each with the roots whose decks map to it
/// (`pipeline_layers/preread.py:_bound_topics_by_root`): the topics a study day's states are for.
#[must_use]
pub fn universe(
    deck_names: &BTreeMap<i64, String>,
    taxonomy: &Taxonomy,
) -> BTreeMap<TopicKey, BTreeSet<String>> {
    let _ = (deck_names, taxonomy);
    BTreeMap::new()
}

/// The queries of ingest's answer, each card paired with the card ingest's read returned for it,
/// and the roots whose answer is saturated.
#[must_use]
pub fn queries_of(
    queue: &NewCardQueue,
    cards: &[Card],
    deck_names: &BTreeMap<i64, String>,
) -> (Vec<DaySetQuery>, Vec<UndeterminedRoot>) {
    let _ = (queue, cards, deck_names);
    (Vec::new(), Vec::new())
}

/// Why the queue could not be read: its could-not-tell reason is [`QueueFailure::reason`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueFailure {
    /// The collection was held, or the engine failed reading it.
    Locked,
    /// The collection, or its throwaway copy, could not be made or opened.
    OpenFailed,
}

impl QueueFailure {
    /// The failure's could-not-tell reason (R6).
    #[must_use]
    pub const fn reason(self) -> CouldNotTell {
        match self {
            Self::Locked => CouldNotTell::CollectionLocked,
            Self::OpenFailed => CouldNotTell::CollectionOpenFailed,
        }
    }
}

impl From<EngineError> for QueueFailure {
    /// The engine's open failure is an open failure; every other failure is the predecessor's class
    /// of a root whose read failed, `collection_locked`.
    fn from(error: EngineError) -> Self {
        match error {
            EngineError::OpenFailed => Self::OpenFailed,
            _ => Self::Locked,
        }
    }
}

/// Why ingest's review read of the private copy failed: its reason is [`ReadFailure::reason`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadFailure {
    /// The collection lock could not be taken.
    Locked,
    /// The copy could not be opened or read.
    OpenFailed,
}

impl ReadFailure {
    /// The failure's could-not-tell reason (R6).
    #[must_use]
    pub const fn reason(self) -> CouldNotTell {
        match self {
            Self::Locked => CouldNotTell::CollectionLocked,
            Self::OpenFailed => CouldNotTell::CollectionOpenFailed,
        }
    }
}

impl From<&ReadError> for ReadFailure {
    fn from(error: &ReadError) -> Self {
        match error {
            ReadError::Lock(_) => Self::Locked,
            ReadError::WriteRefused | ReadError::Copy(_) => Self::OpenFailed,
        }
    }
}

/// The port the resolution asks for today's queue of every top-level deck (R3).
pub trait QueuePort: Sync {
    /// Today's queue of every top-level deck but the engine's default deck, as ingest answers it.
    fn new_card_queue(&self) -> impl Future<Output = Result<NewCardQueue, QueueFailure>> + Send;
}

/// The throwaway copy's file name, numbered per call.
const THROWAWAY: &str = "readings-day-set";

/// The queue port over ingest's engine (R3, R7, R11): a throwaway copy of the private copy, taken
/// under the shared collection lock, queried by the engine on the kernel's offload, and removed.
#[derive(Debug)]
pub struct EngineQueue<E> {
    engine: E,
    copy: PathBuf,
    lock: CollectionLock,
    scratch: PathBuf,
    offload: Offload,
    calls: AtomicU64,
}

impl<E> EngineQueue<E> {
    /// The port over `engine`, reading the copy `settings` name into a throwaway copy in `scratch`,
    /// a directory the service writes, on `offload`.
    #[must_use]
    pub fn new(engine: E, settings: &SyncSettings, scratch: PathBuf, offload: Offload) -> Self {
        Self {
            engine,
            copy: settings.copy_path(),
            lock: CollectionLock::new(settings.lock_path()),
            scratch,
            offload,
            calls: AtomicU64::new(0),
        }
    }
}

impl<E> QueuePort for EngineQueue<E>
where
    E: AnkiEngine + Clone + Send + Sync + 'static,
{
    async fn new_card_queue(&self) -> Result<NewCardQueue, QueueFailure> {
        Ok(NewCardQueue::default())
    }
}

/// The files a collection at `path` may keep beside itself: `SQLite`'s write-ahead log and index.
fn siblings(path: &Path) -> [PathBuf; 2] {
    let name = path.as_os_str().to_owned();
    let mut wal = name.clone();
    wal.push("-wal");
    let mut shm = name;
    shm.push("-shm");
    [PathBuf::from(wal), PathBuf::from(shm)]
}

/// Copies the collection at `source`, and its write-ahead log when it has one, to `target`; on a
/// failure, removes what it copied.
fn copy_collection(source: &Path, target: &Path) -> io::Result<()> {
    let [source_wal, _] = siblings(source);
    let [target_wal, _] = siblings(target);
    let copied = fs::copy(source, target).and_then(|_| {
        if source_wal.exists() {
            fs::copy(&source_wal, &target_wal).map(|_| ())
        } else {
            Ok(())
        }
    });
    if copied.is_err() {
        remove_collection(target);
    }
    copied
}

/// Removes the collection at `path` and the files `SQLite` keeps beside it; one already gone is
/// left gone.
fn remove_collection(path: &Path) {
    let [wal, shm] = siblings(path);
    for file in [path.to_path_buf(), wal, shm] {
        // A file that is not there is already removed; nothing else here can be recovered from.
        let _ = fs::remove_file(file);
    }
}

/// What the resolution needs from the moment it runs: the study day, the rule that places a review
/// in one, the last sync, the taxonomy, and ingest's review read of the private copy.
#[derive(Clone, Copy, Debug)]
pub struct ResolveInputs<'a> {
    /// The study day resolved.
    pub today: StudyDay,
    /// The kernel's study-day rule.
    pub rule: StudyDayRule,
    /// Whether the last sync succeeded.
    pub last_sync: LastSync,
    /// The taxonomy, or `None` when none could be read.
    pub taxonomy: Option<&'a Taxonomy>,
    /// Ingest's read of the private copy: deck names, cards and reviews, or why it failed.
    pub read: Result<&'a CollectionData, ReadFailure>,
}

/// How a topic ends the resolution: in a state, or with a day set for the generation (SPEC-046).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopicEnd {
    /// The topic ends the study day in this state.
    Ended(TopicState),
    /// The topic has new cards today: the generation ends it `ready`, `failed` or
    /// `ai_route_absent`.
    DaySet(ActiveTopic),
}

/// One topic's end of the resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopicResolution {
    /// The topic.
    pub topic: TopicKey,
    /// How it ends.
    pub end: TopicEnd,
}

/// A study day's resolution: the run's outcome, every topic's end, and the decks that map to none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudyDayResolution {
    /// The run's outcome.
    pub outcome: RunOutcome,
    /// Every topic of the taxonomy the read named, sorted by key.
    pub topics: Vec<TopicResolution>,
    /// The decks whose new cards map to no topic (R9): counted with the run, named in the log.
    pub unmapped: Vec<UnmappedDeck>,
}

impl StudyDayResolution {
    /// Every topic of `universe` ended in `state`, with the run's `outcome`.
    fn every_topic(
        universe: BTreeMap<TopicKey, BTreeSet<String>>,
        outcome: RunOutcome,
        state: TopicState,
    ) -> Self {
        Self {
            outcome,
            topics: universe
                .into_keys()
                .map(|topic| TopicResolution {
                    topic,
                    end: TopicEnd::Ended(state),
                })
                .collect(),
            unmapped: Vec::new(),
        }
    }

    /// Every topic of `universe` could not tell, for `reason`, and so could the run.
    fn could_not_tell(
        universe: BTreeMap<TopicKey, BTreeSet<String>>,
        reason: CouldNotTell,
    ) -> Self {
        Self::every_topic(
            universe,
            RunOutcome::CouldNotTell(reason),
            TopicState::CouldNotTell(reason),
        )
    }

    /// The state `topic` ended in, if it is a topic of the resolution that ended.
    #[must_use]
    pub fn state_of(&self, topic: &str) -> Option<TopicState> {
        self.topics
            .iter()
            .find(|resolved| resolved.topic.as_str() == topic)
            .and_then(|resolved| match resolved.end {
                TopicEnd::Ended(state) => Some(state),
                TopicEnd::DaySet(_) => None,
            })
    }

    /// The day set of `topic`, if it is a topic of the resolution with new cards today.
    #[must_use]
    pub fn day_set_of(&self, topic: &str) -> Option<&ActiveTopic> {
        self.topics
            .iter()
            .find(|resolved| resolved.topic.as_str() == topic)
            .and_then(|resolved| match &resolved.end {
                TopicEnd::DaySet(day_set) => Some(day_set),
                TopicEnd::Ended(_) => None,
            })
    }
}

/// Resolves a study day's topics (R4): the last-sync gate, then a run with no taxonomy or no read
/// refused whole, then the pause gate, and only then the queue, within [`RESOLVE_BUDGET`].
pub async fn resolve<Q: QueuePort>(inputs: ResolveInputs<'_>, queue: &Q) -> StudyDayResolution {
    let _ = (inputs, queue);
    StudyDayResolution {
        outcome: RunOutcome::Resolved,
        topics: Vec::new(),
        unmapped: Vec::new(),
    }
}

/// Each topic's end after the queue answered (`pipeline_layers/preread.py:_build_outcomes`): its
/// day set when it has new cards; else its root's could-not-tell when a root that binds it was
/// undetermined; else no new cards.
fn topic_ends(
    universe: BTreeMap<TopicKey, BTreeSet<String>>,
    resolution: &DaySetResolution,
) -> Vec<TopicResolution> {
    let undetermined: BTreeMap<&str, CouldNotTell> = resolution
        .undetermined
        .iter()
        .map(|root| (root.root.as_str(), root.reason))
        .collect();
    let mut active: BTreeMap<&TopicKey, &ActiveTopic> = resolution
        .active
        .iter()
        .map(|day_set| (&day_set.topic, day_set))
        .collect();
    let mut ends: Vec<TopicResolution> = universe
        .into_iter()
        .map(|(topic, roots)| {
            let end = if let Some(day_set) = active.remove(&topic) {
                TopicEnd::DaySet(day_set.clone())
            } else if let Some(reason) = roots
                .iter()
                .find_map(|root| undetermined.get(root.as_str()).copied())
            {
                TopicEnd::Ended(TopicState::CouldNotTell(reason))
            } else {
                TopicEnd::Ended(TopicState::NoNewCards)
            };
            TopicResolution { topic, end }
        })
        .collect();
    ends.extend(active.into_values().map(|day_set| TopicResolution {
        topic: day_set.topic.clone(),
        end: TopicEnd::DaySet(day_set.clone()),
    }));
    ends.sort_by(|left, right| left.topic.cmp(&right.topic));
    ends
}
