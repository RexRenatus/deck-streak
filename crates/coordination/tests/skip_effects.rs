//! What a recorded skip day does to the game at the next recompute (SPEC-083 A15 to A18; R11 to
//! R13): it bridges both streaks without a freeze, leaves the consistency run unchanged and arms no
//! Ascendant, neither counts toward nor ends the governor's silent run, and after an undo the day is
//! a missed day. Each scenario runs beside a control database with no skip. Every review, card,
//! skip and instant is synthetic and set by hand.

// An integration test is test code: its helpers panic on a failed read, and it prints what it
// examined on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::progression::level_view::level_view;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::day_bonuses::DayBonusesStep;
use deck_streak_coordination::recompute::streaks::StreaksStep;
use deck_streak_coordination::recompute::xp::XpStep;
use deck_streak_coordination::recompute::{Fold, FoldInput, Phase};
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_ingest::skip::{SkipId, SkipStore};
use deck_streak_kernel::{Db, StudyDay, StudyDayRule, Track, UtcMillis};
use deck_streak_progression::buffs::is_ascendant_day;
use deck_streak_progression::settle::settled_of_day;
use sqlx::Row;
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
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

/// A review of `card` at `instant`, answered good.
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

/// A review-queue card `id` on `track`, due on collection day number `due`.
const fn card(id: i64, due: i64, track: Track) -> Card {
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
        tier: None,
        memory: None,
    }
}

/// A collection of `reviews` over a language card (1) and a law card (2).
fn collection(reviews: Vec<Review>) -> CollectionData {
    CollectionData {
        reviews,
        cards: vec![
            card(1, D0 + 100 - CREATED, Track::Language),
            card(2, D0 + 100 - CREATED, Track::Law),
        ],
        created_at: UtcMillis::from_epoch_millis(at(CREATED, 12)),
        deck_names: BTreeMap::from([(1, "Synthetic".to_owned())]),
    }
}

/// One review of `card` on each study day of `days`.
fn reviews_on(days: impl IntoIterator<Item = i64>, card: i64) -> Vec<Review> {
    days.into_iter().map(|d| review(at(d, 9), card)).collect()
}

/// The streaks' fold: analytics' rollup, then the streaks and the governor.
fn streak_fold() -> Fold {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    let (step, _due) = StreaksStep::new();
    fold.register(Phase::StreaksAndGovernor, Box::new(step))
        .expect("the streaks step is phase 3's");
    fold
}

/// Progression's fold: analytics' rollup, the base XP and the derived bonuses.
fn xp_fold() -> Fold {
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

/// One recompute at noon of each study day of `days`, each after that day's sync.
async fn daily(fold: &Fold, db: &Db, data: &CollectionData, days: impl IntoIterator<Item = i64>) {
    for d in days {
        recompute(fold, db, data, at(d, 12), d).await;
    }
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

/// Records an applied skip of study day `number`.
async fn skip(db: &Db, number: i64) -> SkipId {
    let skips = SkipStore::new(db.clone());
    let id = skips
        .begin(
            day(number),
            None,
            UtcMillis::from_epoch_millis(at(number, 5)),
        )
        .await
        .expect("the take is recorded");
    skips
        .settle_applied(id, 4, false)
        .await
        .expect("the take settles");
    id
}

/// `(current, longest, freezes)` of `track`'s streak row, when it has one.
async fn streak(db: &Db, track: &str) -> Option<(i64, i64, i64)> {
    let mut write = db.write().await.expect("a write");
    sqlx::query("SELECT current_days, longest_days, freezes FROM streak_state WHERE track = ?")
        .bind(track)
        .fetch_optional(&mut *write)
        .await
        .expect("the streak row reads")
        .map(|row| (row.get(0), row.get(1), row.get(2)))
}

/// Every freeze event, as (study day, delta, reason).
async fn events(db: &Db) -> Vec<(i64, i64, String)> {
    let mut write = db.write().await.expect("a write");
    sqlx::query("SELECT study_day, delta, reason FROM freeze_events ORDER BY study_day, reason")
        .fetch_all(&mut *write)
        .await
        .expect("the freeze events read")
        .into_iter()
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .collect()
}

/// The events of study day `number`.
fn of_day(events: &[(i64, i64, String)], number: i64) -> Vec<(i64, i64, String)> {
    events
        .iter()
        .filter(|(event_day, _, _)| *event_day == number)
        .cloned()
        .collect()
}

/// The governor's stored lapse anchor.
async fn anchor(db: &Db) -> Option<i64> {
    let mut write = db.write().await.expect("a write");
    sqlx::query("SELECT lapse_since FROM governor_state WHERE id = 1")
        .fetch_optional(&mut *write)
        .await
        .expect("the governor row reads")
        .and_then(|row| row.get::<Option<i64>, _>(0))
}

/// Plants the rollup of study day `number`, on pace (a score of 77).
async fn rollup(db: &Db, number: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
         mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
         consistency, retention, workload, volume, mastery, score_at_close, settled_at, \
         fingerprint, created_at, updated_at) \
         VALUES (?1, 42, 0, 42, 0, 0, 600.0, 40, 36, 90.0, 1, 2, 14.0, 0, 0, 40, 36, 64, 60, 2, \
         5, 30, 'live:1736911800000', 77, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, \
         'synthetic', 1000, 1000)",
    )
    .bind(number)
    .execute(&mut *write)
    .await
    .expect("the synthetic rollup is written");
    write.commit().await.expect("the commit");
}

/// The language amount `source` holds on study day `number`, or none.
async fn language(db: &Db, number: i64, source: &str) -> Option<u32> {
    let mut write = db.write().await.expect("a write");
    settled_of_day(&mut write, day(number))
        .await
        .expect("the settled rows read")
        .into_iter()
        .find(|row| row.source == source && row.track == "language")
        .map(|row| row.amount)
}

/// The language and law reviews of a run with a gap at `D0 - 2`, through [`D0`].
fn a_run_with_a_gap() -> CollectionData {
    let studied = (D0 - 6..=D0 - 3).chain(D0 - 1..=D0);
    let mut reviews = reviews_on(studied.clone(), 1);
    reviews.extend(reviews_on(studied, 2));
    collection(reviews)
}

#[tokio::test]
async fn a_skip_day_bridges_both_streaks_without_a_freeze() {
    let data = a_run_with_a_gap();
    let fold = streak_fold();
    let control_dir = TempDir::new().expect("a scratch directory");
    let control = database(&control_dir).await;
    daily(&fold, &control, &data, D0 - 3..=D0).await;
    let skipped_dir = TempDir::new().expect("a scratch directory");
    let skipped = database(&skipped_dir).await;
    skip(&skipped, D0 - 2).await;
    daily(&fold, &skipped, &data, D0 - 3..=D0).await;

    let (control_events, skipped_events) = (events(&control).await, events(&skipped).await);
    println!(
        "A15: examined {} control and {} skipped freeze events",
        control_events.len(),
        skipped_events.len()
    );
    assert_eq!(
        of_day(&skipped_events, D0 - 2),
        Vec::new(),
        "the skip day spends no freeze and breaks nothing"
    );
    assert!(
        of_day(&control_events, D0 - 2)
            .iter()
            .any(|(_, _, reason)| reason == "consumed"),
        "without the skip the gap spends a freeze: {control_events:?}"
    );
    let (control_language, skipped_language) = (
        streak(&control, "language").await.expect("a language row"),
        streak(&skipped, "language").await.expect("a language row"),
    );
    assert_eq!(
        skipped_language.2,
        control_language.2 + 1,
        "the freeze the gap spent is still held: {control_language:?} {skipped_language:?}"
    );
    let (control_law, skipped_law) = (
        streak(&control, "law").await.expect("a law row"),
        streak(&skipped, "law").await.expect("a law row"),
    );
    assert!(
        skipped_law.0 > control_law.0,
        "the law run is bridged: {control_law:?} {skipped_law:?}"
    );
}

#[tokio::test]
async fn a_skip_day_leaves_the_consistency_run_unchanged_and_arms_no_ascendant() {
    // The level view's run: on-pace rollups on D0 and D0 + 2, and no rollup on D0 + 1.
    let mut runs = Vec::new();
    for skipped in [false, true] {
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        rollup(&db, D0).await;
        rollup(&db, D0 + 2).await;
        if skipped {
            skip(&db, D0 + 1).await;
        }
        runs.push(
            level_view(&db, day(D0 + 3))
                .await
                .expect("the view reads")
                .run,
        );
    }
    assert_eq!(
        runs,
        [1, 2],
        "a missed day tiers the run down; a skip day leaves it as it was"
    );

    // The fold's consistency bonus on D0 + 3 reads the same run.
    let data = collection(reviews_on([D0, D0 + 2, D0 + 3], 1));
    let mut bonuses = Vec::new();
    for skipped in [false, true] {
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        if skipped {
            skip(&db, D0 + 1).await;
        }
        let now = at(D0 + 3, 12);
        recompute(&xp_fold(), &db, &data, now, D0 + 3).await;
        // The studied days before D0 + 3 were on pace; D0 + 1 keeps the score of a day unstudied.
        let mut write = db.write().await.expect("a write");
        sqlx::query("UPDATE daily_rollup SET score = 80 WHERE study_day < ?1 AND reviews > 0")
            .bind(D0 + 3)
            .execute(&mut *write)
            .await
            .expect("the synthetic scores are written");
        write.commit().await.expect("commit");
        recompute(&xp_fold(), &db, &data, now, D0 + 3).await;
        bonuses.push(language(&db, D0 + 3, "consistency").await);
    }
    let (control, skipped) = (bonuses[0], bonuses[1]);
    assert!(
        skipped > control,
        "the skip day keeps the longer run's bonus: {bonuses:?}"
    );

    // The Ascendant: a day after an on-pace day with backlog zero arms it, unless it is a skip day.
    let data = collection(reviews_on([D0, D0 + 1, D0 + 2], 1));
    let mut armed = Vec::new();
    for skipped in [false, true] {
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        if skipped {
            skip(&db, D0 + 2).await;
        }
        let now = at(D0 + 2, 12);
        recompute(&xp_fold(), &db, &data, now, D0 + 2).await;
        let mut write = db.write().await.expect("a write");
        sqlx::query("UPDATE daily_rollup SET score = 80 WHERE study_day < ?1")
            .bind(D0 + 2)
            .execute(&mut *write)
            .await
            .expect("the synthetic scores are written");
        write.commit().await.expect("commit");
        recompute(&xp_fold(), &db, &data, now, D0 + 2).await;
        let mut read = db.reader().acquire().await.expect("a reader");
        armed.push(
            is_ascendant_day(&mut read, day(D0 + 2))
                .await
                .expect("the buff reads"),
        );
    }
    println!("A16: examined runs {runs:?}, bonuses {bonuses:?}, armed {armed:?}");
    assert_eq!(armed, [true, false], "a skip day arms no Ascendant");
}

#[tokio::test]
async fn a_skip_day_neither_counts_nor_ends_the_governors_silent_run() {
    let fold = streak_fold();
    // Three silent days, D0 - 3 to D0 - 1, settled by D0's recompute: a lapse opens on the first.
    let data = collection(reviews_on(D0 - 10..=D0 - 4, 1));
    let mut anchors = Vec::new();
    for skipped in [false, true] {
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        if skipped {
            skip(&db, D0 - 2).await;
        }
        daily(&fold, &db, &data, D0 - 4..=D0).await;
        anchors.push(anchor(&db).await);
    }
    assert_eq!(
        anchors,
        [Some(D0 - 3), None],
        "a skip inside three silent days leaves two, and no lapse"
    );

    // Four silent days, D0 - 4 to D0 - 1, with a skip between them: three still count, and the
    // walk goes on past the skip to the run's first silent day.
    let data = collection(reviews_on(D0 - 10..=D0 - 5, 1));
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    skip(&db, D0 - 2).await;
    daily(&fold, &db, &data, D0 - 5..=D0).await;
    let walked = anchor(&db).await;
    println!("A17: examined anchors {anchors:?} and {walked:?}");
    assert_eq!(walked, Some(D0 - 4), "the walk does not end at the skip");
}

#[tokio::test]
async fn after_an_undo_the_day_is_a_missed_day_at_the_next_recompute() {
    let data = a_run_with_a_gap();
    let fold = streak_fold();
    let control_dir = TempDir::new().expect("a scratch directory");
    let control = database(&control_dir).await;
    daily(&fold, &control, &data, D0 - 3..=D0).await;

    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let id = skip(&db, D0 - 2).await;
    daily(&fold, &db, &data, D0 - 3..=D0 - 1).await;
    let settled = events(&db).await;
    assert_eq!(
        of_day(&settled, D0 - 2),
        Vec::new(),
        "while the skip stands its day is bridged"
    );

    SkipStore::new(db.clone())
        .mark_undone(id, UtcMillis::from_epoch_millis(at(D0 - 1, 18)))
        .await
        .expect("the undo is recorded");
    recompute(&fold, &db, &data, at(D0, 12), D0).await;
    let after = events(&db).await;
    println!(
        "A18: examined {} events before and {} after the undo",
        settled.len(),
        after.len()
    );
    assert_eq!(
        of_day(&after, D0 - 2),
        Vec::new(),
        "the transitions already settled stay as they were"
    );
    assert_eq!(
        streak(&db, "language").await,
        streak(&control, "language").await,
        "the next recompute counts the day as missed, as if it had never been skipped"
    );
    assert_eq!(
        streak(&db, "law").await,
        streak(&control, "law").await,
        "the law run reads the day as missed too"
    );
}
