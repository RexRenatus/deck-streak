//! What the badge and records steps' tests share (SPEC-073 A10, A11, A14 to A16): a database in a
//! temporary directory, days and instants under the default rule, seeded rollups, card states and
//! streaks, synthetic reviews and cards, a recording [`Celebrate`] port, a recording bot behind a
//! real router, and one step run in one write. Every number is synthetic and set by hand.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use deck_streak_analytics::metrics::daily_metrics;
use deck_streak_analytics::rollup::{self, RolledDay};
use deck_streak_analytics::score::Score;
use deck_streak_analytics::snapshot::CardState;
use deck_streak_coordination::recompute::{
    Celebrate, Celebration, DayEvaluation, DayStep, Evaluation, RecomputeFacts,
};
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_kernel::{
    Db, KernelError, ManualClock, PortFuture, StudyDay, StudyDayRule, Track, UtcMillis,
};
use deck_streak_notifications::{BotTransport, Pass, Policy, PushFuture, Pushed, Router};
use deck_streak_streaks::store::upsert_state;
use deck_streak_streaks::streak::StreakState;
use tempfile::TempDir;

/// Milliseconds in a day.
pub const DAY_MS: i64 = 86_400_000;
/// Milliseconds in an hour.
pub const HOUR_MS: i64 = 3_600_000;
/// The default rule's rollover hour, UTC.
pub const ROLLOVER: i64 = 4;
/// A study day near the present.
pub const D0: i64 = 20_000;
/// The courses' digest every fixture's facts carry.
pub const DIGEST: &str = "0123456789abcdef";

/// Study day number `number`.
pub const fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

/// `hour` o'clock UTC within study day `day` under the default rule: an hour before the rollover
/// falls on the calendar day after.
pub const fn at(day: i64, hour: i64) -> i64 {
    if hour < ROLLOVER {
        (day + 1) * DAY_MS + hour * HOUR_MS
    } else {
        day * DAY_MS + hour * HOUR_MS
    }
}

/// A database in a temporary directory, kept alive with it.
pub struct Scratch {
    _dir: TempDir,
    /// The database.
    pub db: Db,
}

/// A fresh, migrated database.
pub async fn scratch() -> Scratch {
    let dir = TempDir::new().expect("a scratch directory");
    let db = Db::open(&dir.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    Scratch { _dir: dir, db }
}

/// A study answer of review type at `instant` on `card`, with `ease`, the interval it had going in
/// (`last_interval`, 21 days or more is mature) and the milliseconds it took.
pub const fn answer(
    instant: i64,
    card: i64,
    ease: i64,
    last_interval: i64,
    taken_ms: i64,
) -> Review {
    Review {
        id: instant,
        card_id: card,
        ease,
        interval: last_interval + 5,
        last_interval,
        factor: 2500,
        taken_ms,
        kind: 1,
    }
}

/// A review card `id` in deck `deck`.
pub const fn card(id: i64, deck: i64) -> Card {
    Card {
        id,
        note_id: id,
        deck_id: deck,
        original_deck_id: 0,
        queue: 2,
        kind: 2,
        due: 0,
        interval: 30,
        factor: 2500,
        reps: 3,
        lapses: 0,
        track: Track::Language,
        course: None,
        tier: None,
    }
}

/// A window of `reviews` over `cards`.
pub fn collection(reviews: Vec<Review>, cards: Vec<Card>) -> CollectionData {
    let decks: BTreeMap<i64, String> = cards
        .iter()
        .map(|card| (card.deck_id, format!("Synthetic {}", card.deck_id)))
        .collect();
    CollectionData {
        reviews,
        cards,
        created_at: UtcMillis::from_epoch_millis(at(D0 - 1_000, 12)),
        deck_names: decks,
    }
}

/// One stored rollup as a fixture writes it.
#[derive(Clone, Copy, Debug)]
pub struct RollupSeed {
    /// Its study day number.
    pub day: i64,
    /// Its study reviews.
    pub reviews: i64,
    /// Their seconds.
    pub seconds: f64,
    /// Its stored score.
    pub score: i64,
    /// The score it closed with, if recorded.
    pub score_at_close: Option<i64>,
    /// Its recorded card state, if any.
    pub card_state: Option<CardState>,
}

impl RollupSeed {
    /// A rollup of `day` holding `score`, with no reviews, no close and no card state.
    pub const fn scored(day: i64, score: i64) -> Self {
        Self {
            day,
            reviews: 0,
            seconds: 0.0,
            score,
            score_at_close: None,
            card_state: None,
        }
    }
}

/// Writes every rollup of `seeds`, in one write.
pub async fn seed_rollups(db: &Db, seeds: &[RollupSeed]) {
    let mut write = db.write().await.expect("a write");
    for seed in seeds {
        let mut metrics = daily_metrics(
            &[],
            StudyDayRule::default(),
            day(seed.day),
            &BTreeMap::new(),
        );
        metrics.reviews = seed.reviews;
        metrics.seconds = seed.seconds;
        let score = Score {
            consistency: 0.0,
            retention: 0.0,
            workload: 0.0,
            volume: 0.0,
            mastery: 0.0,
            total: seed.score,
            grade_label: "",
            grade_emoji: "",
        };
        let rolled = RolledDay {
            metrics: &metrics,
            languages: &[],
            fingerprint: "seeded",
            score: &score,
        };
        let now = UtcMillis::from_epoch_millis(at(seed.day, 12));
        rollup::roll_up(&mut write, &rolled, now)
            .await
            .expect("the rollup writes");
        if let Some(close) = seed.score_at_close {
            rollup::record_close(&mut write, day(seed.day), close)
                .await
                .expect("the close writes");
        }
        if let Some(state) = seed.card_state {
            rollup::record_card_state(&mut write, day(seed.day), &state, now)
                .await
                .expect("the card state writes");
        }
    }
    write.commit().await.expect("the rollups commit");
}

/// A card state with `mature` mature cards, `leeches` active leeches, `backlog` overdue cards and
/// `due` cards due today.
pub const fn card_state(mature: i64, leeches: i64, backlog: i64, due: i64) -> CardState {
    CardState {
        mature_count: mature,
        young_count: 0,
        leech_active: leeches,
        backlog,
        due_today: due,
    }
}

/// Stores the language streak as `current` days long, its comeback `armed` or not.
pub async fn seed_streak(db: &Db, current: u32, armed: bool, last: i64) {
    let mut write = db.write().await.expect("a write");
    let state = StreakState {
        current,
        longest: current,
        freezes: 0,
        last_study_day: Some(day(last)),
        comeback_armed: armed,
    };
    upsert_state(
        &mut write,
        "language",
        &state,
        UtcMillis::from_epoch_millis(at(last, 12)),
    )
    .await
    .expect("the streak writes");
    write.commit().await.expect("the streak commits");
}

/// Runs `step` for `day` as `evaluation` over `data`, at `now`, with `base` reviews before the
/// window, in one write, then commits it.
pub async fn run_step(
    db: &Db,
    step: &dyn DayStep,
    data: &CollectionData,
    base: u64,
    (day_number, evaluation): (i64, Evaluation),
    now: i64,
) {
    let facts = RecomputeFacts::new(
        data,
        StudyDayRule::default(),
        UtcMillis::from_epoch_millis(now),
        Some(DIGEST),
    )
    .with_base_reviews(base);
    let evaluated = DayEvaluation {
        day: day(day_number),
        evaluation,
        facts: &facts,
    };
    let mut write = db.write().await.expect("a write");
    step.evaluate(&evaluated, &mut write)
        .await
        .expect("the step evaluates");
    write.commit().await.expect("the day commits");
}

/// Every badge stored, as `(key, tier, study day, marked)`, by key.
pub async fn badges(db: &Db) -> Vec<(String, i64, i64, bool)> {
    sqlx::query_as::<_, (String, i64, i64, bool)>(
        "SELECT badge_key, tier, study_day, celebrated_at IS NOT NULL FROM badges_earned \
         ORDER BY badge_key, tier",
    )
    .fetch_all(db.reader())
    .await
    .expect("the badges read")
}

/// Every record stored, as `(kind, value, study day, previous, marked)`, by kind.
pub async fn records(db: &Db) -> Vec<(String, i64, i64, i64, bool)> {
    sqlx::query_as::<_, (String, i64, i64, i64, bool)>(
        "SELECT kind, value, study_day, previous, celebrated_at IS NOT NULL FROM records \
         ORDER BY kind",
    )
    .fetch_all(db.reader())
    .await
    .expect("the records read")
}

/// Stores `(kind, value, day, previous)` rows as records already celebrated.
pub async fn seed_records(db: &Db, rows: &[(&str, i64, i64, i64)]) {
    let mut write = db.write().await.expect("a write");
    for (kind, value, study_day, previous) in rows {
        sqlx::query(
            "INSERT INTO records (kind, value, study_day, previous, celebrated_at, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        )
        .bind(*kind)
        .bind(*value)
        .bind(*study_day)
        .bind(*previous)
        .bind(at(*study_day, 12))
        .execute(&mut *write)
        .await
        .expect("the record writes");
    }
    write.commit().await.expect("the records commit");
}

/// A [`Celebrate`] port that records each celebration it is handed and answers `Ok`, or no answer.
#[derive(Default)]
pub struct Recorder {
    handed: Mutex<Vec<Celebration>>,
    silent: bool,
}

impl Recorder {
    /// A recorder whose router never answers.
    pub fn silent() -> Self {
        Self {
            handed: Mutex::new(Vec::new()),
            silent: true,
        }
    }

    /// Every celebration handed to it, in order.
    pub fn handed(&self) -> Vec<Celebration> {
        self.handed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Every key handed to it, in order.
    pub fn keys(&self) -> Vec<String> {
        self.handed()
            .into_iter()
            .map(|celebration| celebration.key)
            .collect()
    }
}

impl Celebrate for Recorder {
    fn celebrate<'a>(&'a self, celebration: &'a Celebration) -> PortFuture<'a, ()> {
        Box::pin(async move {
            self.handed
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(celebration.clone());
            if self.silent {
                Err(no_answer())
            } else {
                Ok(())
            }
        })
    }
}

/// A [`Celebrate`] port that, before it answers, marks the offered row at `at` with `mark` (an
/// `UPDATE` binding `?1` to `at`), as a second offer interleaved between this offer's read of the
/// owed rows and its own mark would (ADR-303: a mark is set only while the row is still unset).
pub struct MarksFirst {
    db: Db,
    mark: &'static str,
    at: i64,
}

impl MarksFirst {
    /// The port that marks through `mark`, at the instant `at`, in a write of its own on `db`.
    pub fn new(db: &Db, mark: &'static str, at: i64) -> Self {
        Self {
            db: db.clone(),
            mark,
            at,
        }
    }
}

impl Celebrate for MarksFirst {
    fn celebrate<'a>(&'a self, _celebration: &'a Celebration) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let mut write = self.db.write().await?;
            sqlx::query(self.mark)
                .bind(self.at)
                .execute(&mut *write)
                .await?;
            write.commit().await?;
            Ok(())
        })
    }
}

/// The error of a router that did not answer.
pub fn no_answer() -> KernelError {
    KernelError::Database(sqlx::Error::Protocol(
        "the router did not answer".to_owned(),
    ))
}

/// A bot transport that records every message it delivers.
#[derive(Default)]
pub struct RecordingBot(Mutex<Vec<String>>);

impl RecordingBot {
    /// Every text delivered, in order.
    pub fn sent(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn push(&self, text: &str) -> Pushed {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(text.to_owned());
        Pushed::Delivered
    }
}

impl BotTransport for RecordingBot {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move { self.push(text) })
    }

    fn push_reveal<'a>(
        &'a self,
        _pass: &'a Pass,
        _placeholder: &'a str,
        text: &'a str,
        _pause: Duration,
    ) -> PushFuture<'a> {
        Box::pin(async move { self.push(text) })
    }
}

/// The real router over `db`, its clock at noon of `day_number` (outside the quiet hours), sending
/// through `bot`.
pub fn router(db: &Db, day_number: i64, bot: &Arc<RecordingBot>) -> Arc<Router> {
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let noon = UtcMillis::from_epoch_millis(at(day_number, 12));
    Arc::new(
        Router::new(
            policy,
            db.clone(),
            Arc::new(ManualClock::new(noon)),
            StudyDayRule::default(),
        )
        .with_bot(bot.clone()),
    )
}

/// A router reached, after which the evaluation stops before it can mark what it offered: it hands
/// the celebration to the router, then answers as if nothing came back.
pub struct RouteThenFail(pub Arc<Router>);

impl Celebrate for RouteThenFail {
    fn celebrate<'a>(&'a self, celebration: &'a Celebration) -> PortFuture<'a, ()> {
        Box::pin(async move {
            self.0.celebrate(celebration).await?;
            Err(no_answer())
        })
    }
}
