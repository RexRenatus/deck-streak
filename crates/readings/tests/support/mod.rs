//! Support for readings' integration tests (SPEC-045): the example taxonomy, synthetic collection
//! data, a queue port that counts its calls, and ingest's own synthetic collection builder, which
//! drives Anki's engine (ADR-045, at acceptance), included by path.
//!
//! Every deck name, card and review here is synthetic. The study day is the kernel's default rule:
//! it turns over at 04:00 UTC.

#![allow(
    dead_code,
    reason = "each test target includes this module and calls the part it needs"
)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test support panics like a test (clippy.toml), but clippy's allow-*-in-tests reaches \
              only #[test] functions, not a support module's helpers"
)]

#[path = "../../../ingest/tests/support/synthetic.rs"]
pub mod synthetic;

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use deck_streak_ingest::engine::{NewCardQueue, RootQueue};
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_ingest::settings::{DECK_SEPARATOR, STATE_DIRECTORY, SYNC_ENDPOINT, SyncSettings};
use deck_streak_kernel::{Environment, StudyDay, StudyDayRule, Track, UtcMillis};
use deck_streak_readings::day_set::{QueueFailure, QueuePort};
use deck_streak_readings::taxonomy::Taxonomy;

/// A day, in milliseconds.
pub const DAY_MS: i64 = 86_400_000;
/// An hour, in milliseconds.
pub const HOUR_MS: i64 = 3_600_000;
/// The default rule's rollover hour.
pub const ROLLOVER_HOUR: i64 = 4;
/// A study day near the present, as its epoch day number.
pub const TODAY: i64 = 20_000;
/// An endpoint no test contacts: nothing here syncs.
pub const ENDPOINT: &str = "http://127.0.0.1:9/";

/// The kernel's default study-day rule: 04:00 UTC.
#[must_use]
pub fn rule() -> StudyDayRule {
    StudyDayRule::default()
}

/// Today, as a study day.
#[must_use]
pub const fn today() -> StudyDay {
    StudyDay::from_epoch_day(TODAY)
}

/// The instant `hours` past the rollover that starts study day `day`, by the default rule.
#[must_use]
pub const fn during(day: i64, hours: i64) -> i64 {
    day * DAY_MS + ROLLOVER_HOUR * HOUR_MS + hours * HOUR_MS
}

/// An hour into today.
#[must_use]
pub const fn now() -> UtcMillis {
    UtcMillis::from_epoch_millis(during(TODAY, 1))
}

/// The committed example taxonomy's path.
#[must_use]
pub fn example_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/config/readings-taxonomy.example.json")
}

/// The committed example taxonomy: the synthetic taxonomy the goldens were drawn under.
#[must_use]
pub fn example_taxonomy() -> Taxonomy {
    Taxonomy::load(&example_path()).expect("the example taxonomy is a taxonomy")
}

/// A deck's stored name, from its parts.
#[must_use]
pub fn deck(parts: &[&str]) -> String {
    parts.join(&DECK_SEPARATOR.to_string())
}

/// Deck ids for `names`, from 1001 in order, each name given by its parts.
#[must_use]
pub fn deck_names(names: &[&[&str]]) -> BTreeMap<i64, String> {
    names
        .iter()
        .zip(1001..)
        .map(|(parts, id)| (id, deck(parts)))
        .collect()
}

/// The id `deck_names` gave the deck of `parts`.
#[must_use]
pub fn id_of(deck_names: &BTreeMap<i64, String>, parts: &[&str]) -> i64 {
    let name = deck(parts);
    deck_names
        .iter()
        .find(|(_, stored)| **stored == name)
        .map(|(id, _)| *id)
        .expect("the deck is named")
}

/// A new card `id` of note `id + 10_000` in the deck `deck_id`, borrowed from `original_deck_id`
/// when that is not 0.
#[must_use]
pub const fn card(id: i64, deck_id: i64, original_deck_id: i64) -> Card {
    Card {
        id,
        note_id: id + 10_000,
        deck_id,
        original_deck_id,
        queue: 0,
        kind: 0,
        due: 0,
        interval: 0,
        factor: 0,
        reps: 0,
        lapses: 0,
        track: Track::Language,
        course: None,
        tier: None,
        memory: None,
    }
}

/// A review answered at `at` (epoch milliseconds), of revlog type `kind` with answer `ease`.
#[must_use]
pub const fn review(at: i64, kind: i64, ease: i64) -> Review {
    Review {
        id: at,
        card_id: 1,
        ease,
        interval: 1,
        last_interval: 0,
        factor: 2500,
        taken_ms: 4000,
        kind,
    }
}

/// A qualifying review `hours` into study day `day`.
#[must_use]
pub const fn studied(day: i64, hours: i64) -> Review {
    review(during(day, hours), 1, 3)
}

/// What ingest's read returns: `deck_names`, `cards` and `reviews`.
#[must_use]
pub fn collection(
    deck_names: BTreeMap<i64, String>,
    cards: Vec<Card>,
    reviews: Vec<Review>,
) -> CollectionData {
    CollectionData {
        reviews,
        cards,
        created_at: UtcMillis::from_epoch_millis(0),
        deck_names,
    }
}

/// One root of a queue: the deck `deck_id`, its new cards, and the scheduler's own count of them.
#[must_use]
pub fn root(deck_id: i64, new_cards: &[i64], new_count: usize) -> RootQueue {
    RootQueue {
        deck_id,
        new_cards: new_cards.to_vec(),
        new_count,
    }
}

/// A queue port that counts its calls, and answers after `delay` of tokio's clock.
pub struct RecordingQueue {
    answer: Result<NewCardQueue, QueueFailure>,
    delay: Duration,
    calls: AtomicUsize,
}

impl RecordingQueue {
    /// A port that answers `roots` at once.
    #[must_use]
    pub fn answering(roots: Vec<RootQueue>) -> Self {
        Self {
            answer: Ok(NewCardQueue { roots }),
            delay: Duration::ZERO,
            calls: AtomicUsize::new(0),
        }
    }

    /// A port that fails with `failure` at once.
    #[must_use]
    pub fn failing(failure: QueueFailure) -> Self {
        Self {
            answer: Err(failure),
            delay: Duration::ZERO,
            calls: AtomicUsize::new(0),
        }
    }

    /// The same port, answering only after `delay` of tokio's clock.
    #[must_use]
    pub fn after(self, delay: Duration) -> Self {
        Self { delay, ..self }
    }

    /// How many times the port was asked.
    #[must_use]
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl QueuePort for RecordingQueue {
    async fn new_card_queue(&self) -> Result<NewCardQueue, QueueFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        self.answer.clone()
    }
}

/// Ingest's settings for a copy kept in `state`.
#[must_use]
pub fn settings(state: &Path) -> SyncSettings {
    SyncSettings::from_env(&Environment::from_vars([
        (SYNC_ENDPOINT, OsStr::new(ENDPOINT)),
        (STATE_DIRECTORY, state.as_os_str()),
    ]))
    .expect("the settings parse")
}
