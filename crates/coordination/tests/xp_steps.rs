//! The recompute's XP steps (SPEC-072 A5, A6, A15, A17, A18; R4, R5, R11, R12, R14, R19): review XP
//! and the daily bonuses settled into `xp_settlement` in phase 2, the derived bonuses in phase 5,
//! and the level-up line raised through the router once. Every review, card and instant is
//! synthetic and set by hand.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::level_up::announce_level_up;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::day_bonuses::DayBonusesStep;
use deck_streak_coordination::recompute::xp::XpStep;
use deck_streak_coordination::recompute::{Fold, FoldInput, Phase};
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_ingest::tier::Tier;
use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, Track, UtcMillis};
use deck_streak_notifications::{BotTransport, Pass, Policy, PushFuture, Pushed, Router};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::review_xp::review_xp;
use deck_streak_progression::settle::{SettledRow, settled_of_day};
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// A study day near the present.
const D0: i64 = 20_000;
/// The collection was created a thousand study days before [`D0`].
const CREATED: i64 = D0 - 1_000;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

/// `hour` o'clock UTC of study day `day` under the default rule.
const fn at(day: i64, hour: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS
}

/// A review of `card` at `instant`, of a card whose interval is 25 days.
const fn review(instant: i64, card: i64, ease: i64) -> Review {
    Review {
        id: instant,
        card_id: card,
        ease,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind: 1,
    }
}

/// A review-queue card `id` due on collection day number `due`, on `track`, with its `tier`.
const fn card(id: i64, due: i64, track: Track, tier: Option<Tier>) -> Card {
    Card {
        id,
        note_id: id,
        deck_id: 1,
        original_deck_id: 0,
        queue: 2,
        kind: 2,
        due,
        interval: 30,
        factor: 2500,
        reps: 3,
        lapses: 0,
        track,
        course: None,
        tier,
    }
}

/// The collection day number of study day `day`.
const fn number(day: i64) -> i64 {
    day - CREATED
}

fn collection(reviews: Vec<Review>, cards: Vec<Card>) -> CollectionData {
    CollectionData {
        reviews,
        cards,
        created_at: UtcMillis::from_epoch_millis(at(CREATED, 12)),
        deck_names: BTreeMap::from([(1, "Synthetic".to_owned())]),
    }
}

/// The fold of analytics' step and the XP steps.
fn fold() -> Fold {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    fold.register(Phase::BaseXp, Box::new(XpStep))
        .expect("the XP step is phase 2's");
    fold.register(Phase::DerivedBonuses, Box::new(DayBonusesStep))
        .expect("the bonus step is phase 5's");
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
        },
    )
    .await
    .expect("the fold runs");
}

async fn settled(db: &Db, number: i64) -> Vec<SettledRow> {
    let mut write = db.write().await.expect("a write");
    settled_of_day(&mut write, day(number))
        .await
        .expect("the settled rows read")
}

fn amount_of(rows: &[SettledRow], source: &str, track: &str) -> Option<u32> {
    rows.iter()
        .find(|row| row.source == source && row.track == track)
        .map(|row| row.amount)
}

#[tokio::test]
async fn a_law_review_never_adds_to_the_language_reviews_source() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    // Card 1 is a language card, card 2 a law card: two reviews each, of different eases.
    let reviews = vec![
        review(at(D0, 9), 1, 3),
        review(at(D0, 10), 1, 1),
        review(at(D0, 11), 2, 3),
        review(at(D0, 12), 2, 4),
    ];
    let cards = vec![
        card(1, number(D0) + 90, Track::Language, None),
        card(2, number(D0) + 90, Track::Law, Some(Tier::T3)),
    ];
    let data = collection(reviews.clone(), cards);
    recompute(&fold(), &db, &data, at(D0, 14), D0 - 1).await;

    let language: u32 = examined(
        "language reviews",
        reviews.iter().filter(|r| r.card_id == 1).collect(),
    )
    .into_iter()
    .map(|r| review_xp(r, None))
    .sum();
    let law: u32 = examined(
        "law reviews",
        reviews.iter().filter(|r| r.card_id == 2).collect(),
    )
    .into_iter()
    .map(|r| review_xp(r, Some(Tier::T3)))
    .sum();
    let rows = settled(&db, D0).await;
    assert_eq!(
        amount_of(&rows, "reviews", "language"),
        Some(language),
        "the language source holds the language card's reviews and no law review"
    );
    assert_eq!(
        amount_of(&rows, "reviews_law", "law"),
        Some(law),
        "the law source holds the law card's reviews, with its tier"
    );
    assert_ne!(language, law, "the two sources differ, so a mix-up shows");
}

#[tokio::test]
async fn an_untagged_law_card_and_a_tagged_language_card_earn_the_base_rate() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let reviews = vec![review(at(D0, 9), 1, 3), review(at(D0, 10), 2, 3)];
    // The language card carries a tier and the law card none: the tier reaches neither's rate.
    let cards = vec![
        card(1, number(D0) + 90, Track::Language, Some(Tier::T4)),
        card(2, number(D0) + 90, Track::Law, None),
    ];
    let data = collection(reviews.clone(), cards);
    recompute(&fold(), &db, &data, at(D0, 14), D0 - 1).await;

    let base = review_xp(&reviews[0], None);
    let boosted = review_xp(&reviews[0], Some(Tier::T4));
    assert_ne!(
        base, boosted,
        "the tier changes the rate, so the test can tell"
    );
    let rows = settled(&db, D0).await;
    assert_eq!(amount_of(&rows, "reviews", "language"), Some(base));
    assert_eq!(amount_of(&rows, "reviews_law", "law"), Some(base));
}

#[tokio::test]
async fn backlog_zero_is_settled_only_for_a_day_with_its_snapshot() {
    let reviews = vec![
        review(at(D0, 9), 1, 3),
        review(at(D0 + 1, 9), 1, 3),
        review(at(D0 + 2, 9), 1, 3),
    ];
    let now = at(D0 + 2, 12);
    // Nothing overdue and nothing due: the closing day and the current day hold their snapshots.
    let clear = vec![card(1, number(D0) + 100, Track::Language, None)];
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    recompute(
        &fold(),
        &db,
        &collection(reviews.clone(), clear),
        now,
        D0 + 2,
    )
    .await;
    let mut with_zero = Vec::new();
    for number in [D0, D0 + 1, D0 + 2] {
        let rows = settled(&db, number).await;
        with_zero.push((number, amount_of(&rows, "backlog_zero", "language")));
    }
    assert_eq!(
        examined("days", with_zero),
        vec![(D0, None), (D0 + 1, Some(100)), (D0 + 2, Some(100))],
        "the backfilled day has no snapshot, the closing day and the current day have"
    );

    // A card overdue: the snapshots have a backlog, so no day settles the bonus.
    let overdue = vec![
        card(1, number(D0) + 100, Track::Language, None),
        card(2, number(D0 + 2) - 5, Track::Language, None),
    ];
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    recompute(&fold(), &db, &collection(reviews, overdue), now, D0 + 2).await;
    for number in [D0, D0 + 1, D0 + 2] {
        let rows = settled(&db, number).await;
        assert_eq!(amount_of(&rows, "backlog_zero", "language"), None);
    }
}

#[tokio::test]
async fn two_recomputes_of_one_day_leave_identical_xp_totals() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let reviews = vec![
        review(at(D0, 9), 1, 3),
        review(at(D0 + 1, 9), 1, 3),
        review(at(D0 + 1, 10), 2, 4),
        review(at(D0 + 2, 9), 1, 2),
    ];
    let cards = vec![
        card(1, number(D0) + 100, Track::Language, None),
        card(2, number(D0) + 100, Track::Law, Some(Tier::T2)),
    ];
    let data = collection(reviews, cards);
    let fold = fold();
    let ledger = SqliteXpLedger::new(db.clone());

    recompute(&fold, &db, &data, at(D0 + 2, 12), D0 + 2).await;
    let first_total = ledger.total().await.expect("the total reads");
    let mut first = Vec::new();
    for number in [D0, D0 + 1, D0 + 2] {
        first.push(settled(&db, number).await);
    }
    recompute(&fold, &db, &data, at(D0 + 2, 12), D0 + 2).await;
    recompute(&fold, &db, &data, at(D0 + 2, 13), D0 + 2).await;
    let mut second = Vec::new();
    for number in [D0, D0 + 1, D0 + 2] {
        second.push(settled(&db, number).await);
    }
    assert_eq!(
        ledger.total().await.expect("the total reads"),
        first_total,
        "a replay changes no total"
    );
    assert!(first_total.get() > 0, "the days earned XP");
    assert_eq!(
        examined("days", first),
        second,
        "a replay changes no settled row"
    );
}

/// A bot transport that records every push and delivers it.
#[derive(Default)]
struct Recording(Mutex<Vec<String>>);

impl BotTransport for Recording {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(text.to_owned());
            Pushed::Delivered
        })
    }
}

#[tokio::test]
async fn a_level_up_is_raised_once_when_a_recompute_crosses_a_threshold() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let bot = Arc::new(Recording::default());
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let noon = UtcMillis::from_epoch_millis(at(D0 + 2, 12));
    let router = Router::new(
        policy,
        db.clone(),
        Arc::new(ManualClock::new(noon)),
        StudyDayRule::default(),
    )
    .with_bot(bot.clone());
    let reviews = vec![
        review(at(D0, 9), 1, 3),
        review(at(D0 + 1, 9), 1, 3),
        review(at(D0 + 2, 9), 1, 3),
    ];
    let data = collection(
        reviews,
        vec![card(1, number(D0) + 100, Track::Language, None)],
    );
    let fold = fold();
    let ledger = SqliteXpLedger::new(db.clone());

    let before = ledger.level().await.expect("the level reads");
    recompute(&fold, &db, &data, at(D0 + 2, 12), D0 + 2).await;
    let after = ledger.level().await.expect("the level reads");
    assert!(after > before, "the recompute's XP crosses a level");
    let raised = announce_level_up(&router, before, after, day(D0 + 2))
        .await
        .expect("the line routes");
    assert!(raised.is_some(), "a crossing raises the line");
    assert_eq!(
        bot.0.lock().unwrap_or_else(PoisonError::into_inner).len(),
        1
    );

    // A recompute over the same data crosses nothing, and the level already reached is not raised
    // again even when asked for.
    recompute(&fold, &db, &data, at(D0 + 2, 13), D0 + 2).await;
    let again = ledger.level().await.expect("the level reads");
    assert_eq!(again, after, "a replay stays on its level");
    assert!(
        announce_level_up(&router, after, again, day(D0 + 2))
            .await
            .expect("the line routes")
            .is_none(),
        "no crossing, no line"
    );
    let _ = announce_level_up(&router, before, after, day(D0 + 2)).await;
    assert_eq!(
        bot.0.lock().unwrap_or_else(PoisonError::into_inner).len(),
        1,
        "the level's key is recorded once"
    );
}
