//! The quests context's fold steps (SPEC-411 A2 to A7; R2 to R6): the chest step rolls a day's
//! sessions once, at the day's settle or while it is current and only after a session's gap; the
//! sweep pays a stale chest once on its own day; the token-bonus step settles a window's bonus and
//! never lowers a closed day's; and a refused draw keeps the day. Every review, card, chest, token,
//! draw and instant is synthetic and set by hand.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::chests::{ChestStep, ChestSweepStep, TokenBonusStep};
use deck_streak_coordination::recompute::xp::XpStep;
use deck_streak_coordination::recompute::{
    DayEvaluation, DayStep, Evaluation, Fold, FoldInput, Phase, RecomputeFacts,
};
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_ingest::tier::Tier;
use deck_streak_kernel::{Db, StudyDay, StudyDayRule, Track, UtcMillis};
use deck_streak_progression::settle::settled_of_day;
use deck_streak_quests::draw::{Draw, DrawError};
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
const MINUTE_MS: i64 = 60_000;
/// SPEC-081's session gap, written out: a session ended this long before now has closed.
const GAP_MS: i64 = 600_000;
/// A study day near the present.
const D0: i64 = 20_000;
/// The collection was created a thousand study days before [`D0`].
const CREATED: i64 = D0 - 1_000;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

/// `hour` o'clock UTC of study day `day` under the default rule.
const fn at(day: i64, hour: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS
}

/// A good answer of `card` at `instant`, on a mature card: 24 XP at the base rate (10, times 1.2
/// for good, 2.0 for mature, 1.0 for a review and 1.0 untagged).
const fn review(instant: i64, card: i64) -> Review {
    Review {
        id: instant,
        card_id: card,
        ease: 3,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind: 1,
    }
}

/// A session from `start`: fifteen reviews of fifteen cards from `first_card`, one a minute, so it
/// ends fourteen minutes after it starts, meets the chest floor and earns 360 XP at the base rate.
fn session(start: i64, first_card: i64) -> Vec<Review> {
    (0..15)
        .map(|n| review(start + n * MINUTE_MS, first_card + n))
        .collect()
}

/// A language card `id` in the review queue.
const fn card(id: i64) -> Card {
    Card {
        id,
        note_id: id,
        deck_id: 1,
        original_deck_id: 0,
        queue: 2,
        kind: 2,
        due: 1_000,
        interval: 30,
        factor: 2500,
        reps: 3,
        lapses: 0,
        track: Track::Language,
        course: None,
        tier: None,
        memory: None,
    }
}

/// The collection of `reviews`, with a card for each card they answer.
fn collection(reviews: Vec<Review>) -> CollectionData {
    let mut ids: Vec<i64> = reviews.iter().map(|review| review.card_id).collect();
    ids.sort_unstable();
    ids.dedup();
    CollectionData {
        reviews,
        cards: ids.into_iter().map(card).collect(),
        created_at: UtcMillis::from_epoch_millis(at(CREATED, 12)),
        deck_names: BTreeMap::from([(1, "Synthetic".to_owned())]),
    }
}

/// A scripted draw: it answers its script in order, a `None` as the generator's refusal, and
/// refuses every draw past the script; it counts every call.
#[derive(Clone)]
struct Scripted {
    script: Arc<Mutex<Vec<Option<f64>>>>,
    calls: Arc<AtomicUsize>,
}

impl Scripted {
    fn new(script: &[Option<f64>]) -> Self {
        let mut reversed = script.to_vec();
        reversed.reverse();
        Self {
            script: Arc::new(Mutex::new(reversed)),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Draw for Scripted {
    fn draw(&mut self) -> Result<f64, DrawError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let next = self
            .script
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop()
            .flatten();
        next.ok_or(DrawError::Generator(getrandom::Error::UNSUPPORTED))
    }
}

/// The draws of one Rare chest paying 60 XP: 0.2 rolls a Rare (20 points, past Legendary's 1 and
/// Epic's 7, under Rare's 30), and 0.99 pays 30 + 30 of its band, under the cap of 30% of 360.
const RARE_60: [Option<f64>; 2] = [Some(0.2), Some(0.99)];

/// The fold of analytics' step, the XP step and the quests context's three steps.
fn fold(draw: &Scripted) -> Fold {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    fold.register(Phase::BaseXp, Box::new(XpStep))
        .expect("the XP step is phase 2's");
    fold.register(
        Phase::DaySteps,
        Box::new(ChestStep::with_draw(draw.clone())),
    )
    .expect("the chest step is phase 4's");
    fold.register(Phase::DaySteps, Box::new(ChestSweepStep))
        .expect("the sweep step is phase 4's");
    fold.register(Phase::DerivedBonuses, Box::new(TokenBonusStep))
        .expect("the token-bonus step is phase 5's");
    fold
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

async fn recompute(fold: &Fold, db: &Db, data: &CollectionData, now: i64, synced_in: i64) {
    fold.run(
        db,
        &FoldInput {
            data,
            rule: StudyDayRule::default(),
            now: UtcMillis::from_epoch_millis(now),
            synced_in: Some(day(synced_in)),
            courses_digest: Some("0123456789abcdef"),
            base_reviews: 0,
            offers: None,
        },
    )
    .await
    .expect("the fold runs");
}

/// A stored chest: (study day, session start, rarity, payout, state).
type ChestRow = (i64, i64, String, i64, String);
/// A ledger grant: (study day, source, track, amount, scope).
type GrantRow = (i64, String, String, i64, String);

async fn chests(db: &Db) -> Vec<ChestRow> {
    let mut write = db.write().await.expect("a write");
    sqlx::query_as(
        "SELECT study_day, session_start, rarity, payout_xp, state FROM chests ORDER BY id",
    )
    .fetch_all(&mut *write)
    .await
    .expect("the chests read")
}

async fn grants(db: &Db) -> Vec<GrantRow> {
    let mut write = db.write().await.expect("a write");
    sqlx::query_as("SELECT study_day, source, track, amount, scope FROM xp_ledger ORDER BY id")
        .fetch_all(&mut *write)
        .await
        .expect("the ledger reads")
}

/// The amount settled for `source` on `track` of study day `number`, none when no row holds it.
async fn settled(db: &Db, number: i64, source: &str, track: &str) -> Option<u32> {
    let mut write = db.write().await.expect("a write");
    settled_of_day(&mut write, day(number))
        .await
        .expect("the settled rows read")
        .into_iter()
        .find(|row| row.source == source && row.track == track)
        .map(|row| row.amount)
}

/// Stores a session chest of study day `study_day`, its payout and state as given; its id.
async fn seed_chest(
    db: &Db,
    study_day: i64,
    start: i64,
    rarity: &str,
    xp: i64,
    state: &str,
) -> i64 {
    let mut write = db.write().await.expect("a write");
    let id = sqlx::query(
        "INSERT INTO chests (study_day, origin, session_start, rarity, payout_xp, state, \
         created_at) VALUES (?1, 'session', ?2, ?3, ?4, ?5, 0)",
    )
    .bind(study_day)
    .bind(start)
    .bind(rarity)
    .bind(xp)
    .bind(state)
    .execute(&mut *write)
    .await
    .expect("a chest stored")
    .last_insert_rowid();
    write.commit().await.expect("the chest committed");
    id
}

fn row(day: i64, start: i64, rarity: &str, xp: i64, state: &str) -> ChestRow {
    (day, start, rarity.to_owned(), xp, state.to_owned())
}

fn grant(day: i64, source: &str, xp: i64) -> GrantRow {
    (
        day,
        source.to_owned(),
        "language".to_owned(),
        xp,
        "once".to_owned(),
    )
}

#[tokio::test]
async fn the_chest_step_settles_a_closed_day_once() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let draw = Scripted::new(&RARE_60);
    let fold = fold(&draw);
    // One session on the day before D0, settled by the first fold and swept by its current day.
    let start = at(D0 - 1, 12);
    let data = collection(session(start, 1));
    recompute(&fold, &db, &data, at(D0, 12), D0).await;
    recompute(&fold, &db, &data, at(D0, 13), D0).await;
    assert_eq!(
        (chests(&db).await, grants(&db).await, draw.calls()),
        (
            vec![row(D0 - 1, start, "rare", 60, "resolved")],
            vec![grant(D0 - 1, "chest:1", 60)],
            2
        ),
        "the closed day's session is rolled once, by two draws, and its XP is granted once"
    );
}

#[tokio::test]
async fn the_chest_step_rolls_nothing_on_a_backfill_or_a_revisit() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let draw = Scripted::new(&RARE_60);
    let fold = fold(&draw);
    // A session five days back, which the first fold backfills and both folds revisit, and one
    // this morning, which the current day rolls.
    let morning = at(D0, 9);
    let mut reviews = session(at(D0 - 5, 12), 1);
    reviews.extend(session(morning, 101));
    let data = collection(reviews);
    recompute(&fold, &db, &data, at(D0, 15), D0).await;
    recompute(&fold, &db, &data, at(D0, 16), D0).await;
    assert_eq!(
        (chests(&db).await, grants(&db).await, draw.calls()),
        (vec![row(D0, morning, "rare", 60, "sealed")], vec![], 2),
        "only today's session is rolled; the backfilled and revisited day rolls and grants nothing"
    );
}

#[tokio::test]
async fn a_session_still_open_earns_no_chest_until_its_gap_has_passed() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let draw = Scripted::new(&RARE_60);
    let fold = fold(&draw);
    let start = at(D0, 12);
    let end = start + 14 * MINUTE_MS;
    let data = collection(session(start, 1));
    recompute(&fold, &db, &data, end + GAP_MS - 1, D0).await;
    let before = (chests(&db).await, draw.calls());
    recompute(&fold, &db, &data, end + GAP_MS, D0).await;
    assert_eq!(
        (before, (chests(&db).await, draw.calls())),
        ((vec![], 0), (vec![row(D0, start, "rare", 60, "sealed")], 2)),
        "a gap less one millisecond after its end the session earns nothing; at the gap it earns its chest"
    );
}

#[tokio::test]
async fn the_sweep_pays_a_stale_chest_once_on_its_own_day() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let stale_start = at(D0 - 3, 12);
    let morning_start = at(D0 - 1, 22);
    let stale = seed_chest(&db, D0 - 3, stale_start, "rare", 40, "sealed").await;
    seed_chest(&db, D0 - 1, morning_start, "legendary", 150, "vaulted").await;
    let data = collection(Vec::new());
    let facts = RecomputeFacts::new(
        &data,
        StudyDayRule::default(),
        UtcMillis::from_epoch_millis(at(D0, 12)),
        None,
    );
    let evaluations = [
        Evaluation::Backfill {
            reviews_changed: true,
        },
        Evaluation::Settle { end_of_day: true },
        Evaluation::Revisit {
            reviews_changed: true,
        },
        Evaluation::Current,
        Evaluation::Current,
    ];
    let mut paid = Vec::new();
    for evaluation in evaluations {
        let today = DayEvaluation {
            day: day(D0),
            evaluation,
            facts: &facts,
        };
        let mut write = db.write().await.expect("a write");
        ChestSweepStep
            .evaluate(&today, &mut write)
            .await
            .expect("the sweep runs");
        write.commit().await.expect("the day committed");
        paid.push(grants(&db).await.len());
    }
    assert_eq!(
        (paid, grants(&db).await, chests(&db).await),
        (
            vec![0, 0, 0, 1, 1],
            vec![grant(D0 - 3, &format!("chest:{stale}"), 40)],
            vec![
                row(D0 - 3, stale_start, "rare", 40, "resolved"),
                row(D0 - 1, morning_start, "legendary", 150, "vaulted"),
            ]
        ),
        "only the current day sweeps, once, paying the stale chest on its own day and track"
    );
}

#[tokio::test]
async fn the_token_bonus_step_settles_the_window_and_a_revisit_never_lowers_it() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let draw = Scripted::new(&[]);
    let fold = fold(&draw);
    // A token whose window opened at noon of D0 and ends two hours later.
    let opened = at(D0, 12);
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO xp_tokens (chest_id, granted_at, activated_at, window_ends_at, consumed, \
         created_at) VALUES (0, ?1, ?1, ?2, 0, ?1)",
    )
    .bind(opened)
    .bind(opened + 2 * HOUR_MS)
    .execute(&mut *write)
    .await
    .expect("a token stored");
    write.commit().await.expect("the token committed");
    let first = review(opened + 10 * MINUTE_MS, 1);
    let second = review(opened + 20 * MINUTE_MS, 2);
    let both = collection(vec![first, second]);
    let one = collection(vec![first]);
    let mut amounts = Vec::new();
    for (data, now, synced_in) in [
        (&both, at(D0, 13), D0),
        (&one, at(D0, 13) + MINUTE_MS, D0),
        (&both, at(D0, 13) + 2 * MINUTE_MS, D0),
        (&one, at(D0 + 1, 12), D0 + 1),
    ] {
        recompute(&fold, &db, data, now, synced_in).await;
        amounts.push(settled(&db, D0, "2x:1", "language").await);
    }
    assert_eq!(
        amounts,
        [Some(48), Some(24), Some(48), Some(48)],
        "the open day's bonus is replaced as it moves, and its settle and revisit never lower it"
    );
}

#[tokio::test]
async fn a_failed_draw_keeps_the_day_and_the_next_fold_grants_the_chest() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let draw = Scripted::new(&[None, Some(0.2), Some(0.99)]);
    let fold = fold(&draw);
    let start = at(D0, 12);
    let data = collection(session(start, 1));
    recompute(&fold, &db, &data, at(D0, 13), D0).await;
    let first = (
        chests(&db).await,
        settled(&db, D0, "reviews", "language").await,
    );
    recompute(&fold, &db, &data, at(D0, 13) + MINUTE_MS, D0).await;
    assert_eq!(
        (first, chests(&db).await, draw.calls()),
        (
            (vec![], Some(360)),
            vec![row(D0, start, "rare", 60, "sealed")],
            3
        ),
        "the refused draw writes no chest and keeps the day's XP; the next fold grants the chest"
    );
}

/// A card `id` on `track` whose note carries `tier`.
const fn tagged(id: i64, track: Track, tier: Tier) -> Card {
    Card {
        track,
        tier: Some(tier),
        ..card(id)
    }
}

/// A hard answer of `card` at `instant` on a young card: 13 XP at the base rate (10, times 1.0 for
/// hard, 1.3 for young, 1.0 for a review and 1.0 untagged).
const fn hard_young(instant: i64, card: i64) -> Review {
    Review {
        ease: 2,
        interval: 5,
        last_interval: 2,
        ..review(instant, card)
    }
}

/// Stores a token activated at `opened` whose window ends at `ends`, not yet consumed.
async fn seed_open_token(db: &Db, opened: i64, ends: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO xp_tokens (chest_id, granted_at, activated_at, window_ends_at, consumed, \
         created_at) VALUES (0, ?1, ?1, ?2, 0, ?1)",
    )
    .bind(opened)
    .bind(ends)
    .execute(&mut *write)
    .await
    .expect("a token stored");
    write.commit().await.expect("the token committed");
}

#[test]
fn the_fold_names_each_quests_step_in_its_phase() {
    let draw = Scripted::new(&[]);
    assert_eq!(
        fold(&draw).steps(),
        vec![
            (Phase::RollupAndScore, "analytics.rollup_and_score"),
            (Phase::BaseXp, "progression.base_xp"),
            (Phase::DaySteps, "quests.chests"),
            (Phase::DaySteps, "quests.chest_sweep"),
            (Phase::DerivedBonuses, "quests.token_bonuses"),
        ],
        "the fold's report and log name each quests step by its own name, in its phase"
    );
}

#[tokio::test]
async fn a_token_bonus_counts_a_law_cards_tier_and_never_a_language_cards() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    // A token whose window opened at noon of D0 and ends two hours later.
    let opened = at(D0, 12);
    seed_open_token(&db, opened, opened + 2 * HOUR_MS).await;
    // A law card tagged T4 earns 24 times 5, 120 XP; a language card tagged T2 earns 24, untagged.
    let data = CollectionData {
        reviews: vec![
            review(opened + 10 * MINUTE_MS, 1),
            review(opened + 20 * MINUTE_MS, 2),
        ],
        cards: vec![
            tagged(1, Track::Law, Tier::T4),
            tagged(2, Track::Language, Tier::T2),
        ],
        ..collection(Vec::new())
    };
    let facts = RecomputeFacts::new(
        &data,
        StudyDayRule::default(),
        UtcMillis::from_epoch_millis(at(D0, 13)),
        None,
    );
    let today = DayEvaluation {
        day: day(D0),
        evaluation: Evaluation::Current,
        facts: &facts,
    };
    let mut write = db.write().await.expect("a write");
    TokenBonusStep
        .evaluate(&today, &mut write)
        .await
        .expect("the token-bonus step runs");
    write.commit().await.expect("the day committed");
    assert_eq!(
        settled(&db, D0, "2x:1", "language").await,
        Some(144),
        "the window's bonus is the law card's T4 120 and the language card's untagged 24"
    );
}

#[tokio::test]
async fn a_sessions_payout_cap_counts_its_own_reviews_and_no_other_of_its_day() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let draw = Scripted::new(&RARE_60);
    let fold = fold(&draw);
    // Fifteen hard answers on young cards from 09:00 earn 195 XP, so the session's Rare is capped at
    // 58, 30% of 195, under the 60 its draws pay. One good answer at 11:00 of the same day, outside
    // the session, earns 24 XP and no chest; counted in the session, it would lift the cap to 65.
    let start = at(D0, 9);
    let mut reviews: Vec<Review> = (0..15)
        .map(|n| hard_young(start + n * MINUTE_MS, 1 + n))
        .collect();
    reviews.push(review(at(D0, 11), 101));
    let data = collection(reviews);
    recompute(&fold, &db, &data, at(D0, 13), D0).await;
    assert_eq!(
        (chests(&db).await, draw.calls()),
        (vec![row(D0, start, "rare", 58, "sealed")], 2),
        "the session's Rare is capped by its own reviews' XP, never by its day's other reviews"
    );
}
