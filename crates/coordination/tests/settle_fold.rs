//! The recompute settles each study day once, in order, with the state it had at its close
//! (SPEC-071 A16 to A21; R15 to R19; ADR-071). Every review, card and deck is synthetic, and every
//! instant is set by hand: no test waits for time to pass.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_analytics::rollup::{RollupStore, StoredDay, recent_volumes};
use deck_streak_analytics::score::{ScoreState, baseline_window, compute_score, raw_streak};
use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_analytics::snapshot::{CardState, card_snapshot};
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::{
    DayEvaluation, DayStep, Evaluation, Fold, FoldError, FoldInput, FoldReport, PHASES, Phase,
};
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_kernel::{Db, PortFuture, StudyDay, StudyDayRule, Track, UtcMillis};
use sqlx::SqliteConnection;
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

/// `hour` o'clock UTC of study day `day` under the default rule (rollover at 04:00 UTC): hours 4
/// to 27 fall inside the day.
const fn at(day: i64, hour: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS
}

/// A review of `card` at `instant`.
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

/// Two reviews on each of `days`, of cards 1 and 2.
fn reviews_on(days: &[i64]) -> Vec<Review> {
    days.iter()
        .flat_map(|&number| [review(at(number, 9), 1, 3), review(at(number, 10), 2, 1)])
        .collect()
}

/// A review-queue card `id` due on collection day number `due`, with `interval` and `lapses`.
const fn card(id: i64, due: i64, interval: i64, lapses: i64) -> Card {
    Card {
        id,
        note_id: id,
        deck_id: 1,
        original_deck_id: 0,
        queue: 2,
        kind: 2,
        due,
        interval,
        factor: 2500,
        reps: 3,
        lapses,
        track: Track::Language,
        course: None,
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

/// What a probe step saw: its phase and name, the day, and how the day was evaluated.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Seen {
    phase: Phase,
    name: &'static str,
    day: StudyDay,
    evaluation: Evaluation,
}

type Log = Arc<Mutex<Vec<Seen>>>;

/// A step that only records what it was handed.
struct Probe {
    phase: Phase,
    name: &'static str,
    log: Log,
}

impl DayStep for Probe {
    fn phase(&self) -> Phase {
        self.phase
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        _write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            self.log
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(Seen {
                    phase: self.phase,
                    name: self.name,
                    day: day.day,
                    evaluation: day.evaluation,
                });
            Ok(())
        })
    }
}

fn probe(phase: Phase, name: &'static str, log: &Log) -> Box<dyn DayStep> {
    Box::new(Probe {
        phase,
        name,
        log: Arc::clone(log),
    })
}

/// A fold of analytics' step and a probe in phase 2.
fn fold(log: &Log) -> Fold {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    fold.register(Phase::BaseXp, probe(Phase::BaseXp, "probe.base_xp", log))
        .expect("the probe is phase 2's");
    fold
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

async fn recompute(
    fold: &Fold,
    db: &Db,
    data: &CollectionData,
    now: i64,
    synced_in: i64,
) -> FoldReport {
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
    .expect("the fold runs")
}

async fn rollup(db: &Db, number: i64) -> StoredDay {
    RollupStore::new(db.clone())
        .days(day(number), day(number))
        .await
        .expect("the rollup reads")
        .pop()
        .unwrap_or_else(|| panic!("study day {number} has a rollup"))
}

fn seen(log: &Log) -> Vec<Seen> {
    log.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

fn days(numbers: &[i64]) -> Vec<StudyDay> {
    numbers.iter().copied().map(day).collect()
}

#[tokio::test]
async fn the_fold_settles_each_closed_day_once_oldest_first() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let log = Log::default();
    let fold = fold(&log);
    let data = collection(
        reviews_on(&[D0, D0 + 1, D0 + 2, D0 + 3, D0 + 4]),
        Vec::new(),
    );

    let first = recompute(&fold, &db, &data, at(D0 + 2, 10), D0 + 2).await;
    assert_eq!(
        first.settled,
        days(&[D0 + 1]),
        "the first recompute settles the closing day"
    );
    let second = recompute(&fold, &db, &data, at(D0 + 5, 6), D0 + 5).await;
    assert_eq!(
        second.settled,
        days(&[D0 + 2, D0 + 3, D0 + 4]),
        "every closed day after the cursor, oldest first"
    );
    for number in [D0 + 2, D0 + 3, D0 + 4] {
        assert_eq!(
            rollup(&db, number).await.settled_at,
            Some(UtcMillis::from_epoch_millis(at(D0 + 5, 6))),
            "study day {number} carries the instant it was settled"
        );
    }
    let third = recompute(&fold, &db, &data, at(D0 + 5, 8), D0 + 5).await;
    assert_eq!(
        third.settled,
        Vec::<StudyDay>::new(),
        "a second recompute settles no day"
    );

    // Across all three, each day was settled exactly once, and in order.
    let settles: Vec<StudyDay> = seen(&log)
        .into_iter()
        .filter(|seen| matches!(seen.evaluation, Evaluation::Settle { .. }))
        .map(|seen| seen.day)
        .collect();
    assert_eq!(
        examined("settles", settles),
        days(&[D0 + 1, D0 + 2, D0 + 3, D0 + 4])
    );
}

#[tokio::test]
async fn the_closing_day_is_settled_with_its_end_of_day_card_state() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let log = Log::default();
    let fold = fold(&log);
    // Card 11 is due on D0+1 and overdue on D0+2; card 12 is overdue on both; card 13 is a leech.
    let cards = vec![
        card(11, number(D0 + 1), 30, 0),
        card(12, number(D0 + 1) - 3, 5, 0),
        card(13, number(D0 + 1) + 40, 30, 9),
    ];
    let data = collection(reviews_on(&[D0, D0 + 1]), cards.clone());
    let now = at(D0 + 2, 5);
    let report = recompute(&fold, &db, &data, now, D0 + 2).await;
    assert_eq!(report.settled, days(&[D0 + 1]));

    let closing = rollup(&db, D0 + 1).await;
    let end_of_day = card_snapshot(&cards, 8, number(D0 + 1));
    let live = card_snapshot(&cards, 8, number(D0 + 2));
    assert_ne!(end_of_day, live, "the two day numbers see different states");
    assert_eq!(
        closing.card_state,
        Some(CardState::from(&end_of_day)),
        "the closing day's card state is taken at its own day number"
    );
    assert_eq!(closing.card_state_src, Some(format!("live:{now}")));
    let expected = closing_total(&db, &data, &closing, &end_of_day).await;
    let with_live = closing_total(&db, &data, &closing, &live).await;
    assert_ne!(
        expected, with_live,
        "the fixture tells the two states apart"
    );
    assert_eq!(
        closing.score_at_close,
        Some(expected),
        "the score it closed with is kept"
    );

    // The current day carries the live state.
    let current = rollup(&db, D0 + 2).await;
    assert_eq!(current.card_state, Some(CardState::from(&live)));

    // A later recompute leaves what the day closed with.
    let later = recompute(&fold, &db, &data, at(D0 + 3, 7), D0 + 3).await;
    assert_eq!(later.settled, days(&[D0 + 2]));
    let again = rollup(&db, D0 + 1).await;
    assert_eq!(again.card_state, closing.card_state);
    assert_eq!(again.card_state_src, closing.card_state_src);
    assert_eq!(again.score_at_close, closing.score_at_close);
    assert_eq!(again.settled_at, closing.settled_at);
}

/// The total the closing day scores with `snapshot`, over the rollups before it.
async fn closing_total(
    db: &Db,
    data: &CollectionData,
    closing: &StoredDay,
    snapshot: &deck_streak_analytics::snapshot::CardSnapshot,
) -> i64 {
    let study_days: BTreeSet<StudyDay> = data
        .reviews
        .iter()
        .map(|review| StudyDayRule::default().study_day(UtcMillis::from_epoch_millis(review.id)))
        .collect();
    let mut connection = db.reader().acquire().await.expect("a connection");
    let recent = recent_volumes(&mut connection, closing.metrics.day)
        .await
        .expect("the volumes read");
    compute_score(
        &closing.metrics,
        Some(ScoreState::from(snapshot)),
        raw_streak(&study_days, closing.metrics.day),
        baseline_window(&recent, closing.metrics.day),
    )
    .total
}

#[tokio::test]
async fn a_closed_day_waits_for_a_successful_sync_after_its_close() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let log = Log::default();
    let fold = fold(&log);
    let data = collection(reviews_on(&[D0, D0 + 1, D0 + 2]), Vec::new());

    let first = recompute(&fold, &db, &data, at(D0 + 1, 10), D0 + 1).await;
    assert_eq!(first.settled, days(&[D0]));
    // D0+1 has closed, and the last successful sync started before it closed: it stays owed, and
    // only the current day is evaluated.
    let waiting = recompute(&fold, &db, &data, at(D0 + 2, 5), D0 + 1).await;
    assert_eq!(
        waiting.settled,
        Vec::<StudyDay>::new(),
        "no settle before the sync"
    );
    assert_eq!(waiting.current, Some(day(D0 + 2)));
    assert_eq!(rollup(&db, D0 + 1).await.settled_at, None, "still owed");
    // The sync that started after the close lets it settle.
    let synced = recompute(&fold, &db, &data, at(D0 + 2, 7), D0 + 2).await;
    assert_eq!(synced.settled, days(&[D0 + 1]));
    assert!(rollup(&db, D0 + 1).await.settled_at.is_some());
}

#[tokio::test]
async fn the_first_recompute_backfills_the_window_without_today_only_rules() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let log = Log::default();
    let fold = fold(&log);
    let studied = [D0 - 10, D0 - 5, D0 - 3, D0 - 1, D0];
    let data = collection(reviews_on(&studied), vec![card(11, number(D0), 30, 0)]);
    let report = recompute(&fold, &db, &data, at(D0 + 1, 9), D0 + 1).await;

    let past = days(&[D0 - 10, D0 - 5, D0 - 3, D0 - 1]);
    assert_eq!(
        report.backfilled, past,
        "every past study day of the window"
    );
    assert_eq!(
        report.settled,
        days(&[D0]),
        "then the most recently closed day is settled"
    );
    assert_eq!(report.current, Some(day(D0 + 1)));
    for backfilled in examined("backfilled days", past.clone()) {
        let row = rollup(&db, backfilled.epoch_day()).await;
        assert_eq!(row.metrics.reviews, 2, "{backfilled} is rolled up");
        assert_eq!(
            row.card_state, None,
            "{backfilled} is in the historical form"
        );
        assert_eq!(
            row.settled_at, None,
            "{backfilled} is backfilled, never settled"
        );
        let evaluations: Vec<Evaluation> = seen(&log)
            .into_iter()
            .filter(|seen| seen.day == backfilled)
            .map(|seen| seen.evaluation)
            .collect();
        assert!(!evaluations.is_empty(), "{backfilled} was evaluated");
        assert!(
            evaluations
                .iter()
                .all(|evaluation| !evaluation.runs_today_only_rules()),
            "{backfilled} ran a today-only rule: {evaluations:?}"
        );
    }
    let settled = rollup(&db, D0).await;
    assert!(settled.card_state.is_some() && settled.settled_at.is_some());
}

#[tokio::test]
async fn the_steps_run_in_their_phase_order() {
    let log = Log::default();
    let mut fold = Fold::default();
    // Registered out of order: the phases, not the registration, decide the order.
    for phase in [
        Phase::Awards,
        Phase::BaseXp,
        Phase::CoinMint,
        Phase::StreaksAndGovernor,
        Phase::DerivedBonuses,
        Phase::DaySteps,
    ] {
        fold.register(phase, probe(phase, "probe", &log))
            .expect("a step in its own phase");
    }
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    let order: Vec<Phase> = fold.steps().into_iter().map(|(phase, _)| phase).collect();
    assert_eq!(order, PHASES, "the steps run in the declared phase order");
    let numbers: Vec<usize> = PHASES.iter().map(|phase| phase.number()).collect();
    assert_eq!(numbers, [1, 2, 3, 4, 5, 6, 7]);

    // A derived bonus registered as base XP would pay before its base: refused.
    let refused = fold
        .register(
            Phase::BaseXp,
            probe(Phase::DerivedBonuses, "misplaced", &log),
        )
        .expect_err("a step outside its phase");
    assert_eq!(
        refused,
        FoldError::OutsidePhase {
            step: "misplaced",
            declared: Phase::DerivedBonuses,
            registered: Phase::BaseXp,
        }
    );
    assert!(fold.steps().iter().all(|(_, name)| *name != "misplaced"));

    // Run over a window: each evaluated day sees the phases 2 to 7 in order.
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let data = collection(reviews_on(&[D0, D0 + 1]), Vec::new());
    recompute(&fold, &db, &data, at(D0 + 2, 9), D0 + 2).await;
    let mut by_day: BTreeMap<(StudyDay, String), Vec<Phase>> = BTreeMap::new();
    for seen in seen(&log) {
        by_day
            .entry((seen.day, format!("{:?}", seen.evaluation)))
            .or_default()
            .push(seen.phase);
    }
    for (evaluated, phases) in examined("evaluated days", by_day.into_iter().collect()) {
        assert_eq!(phases, PHASES[1..], "the phases of {evaluated:?}");
    }
}

#[tokio::test]
async fn a_recompute_rerolls_only_changed_settling_and_current_days() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let log = Log::default();
    let fold = fold(&log);
    let before = collection(reviews_on(&[D0, D0 + 1, D0 + 2, D0 + 3]), Vec::new());
    let first = recompute(&fold, &db, &before, at(D0 + 3, 10), D0 + 3).await;
    assert_eq!(first.backfilled, days(&[D0, D0 + 1]));
    assert_eq!(first.settled, days(&[D0 + 2]));

    // Two days later: a review of D0+1 reached the copy late, and D0+3 and D0+4 were studied.
    let mut reviews = reviews_on(&[D0, D0 + 1, D0 + 2, D0 + 3, D0 + 4]);
    reviews.push(review(at(D0 + 1, 20), 3, 4));
    reviews.sort_by_key(|review| review.id);
    let after = collection(reviews, Vec::new());
    let second = recompute(&fold, &db, &after, at(D0 + 5, 6), D0 + 5).await;
    assert_eq!(second.settled, days(&[D0 + 3, D0 + 4]));
    assert_eq!(
        examined("re-rolled days", second.rerolled.clone()),
        days(&[D0 + 1, D0 + 3, D0 + 4, D0 + 5]),
        "the changed day, the settling days and the current day, and no other"
    );
    let unchanged = UtcMillis::from_epoch_millis(at(D0 + 3, 10));
    let moved = UtcMillis::from_epoch_millis(at(D0 + 5, 6));
    assert_eq!(
        rollup(&db, D0).await.updated_at,
        unchanged,
        "D0 was not re-rolled"
    );
    assert_eq!(
        rollup(&db, D0 + 2).await.updated_at,
        unchanged,
        "D0+2 was not re-rolled"
    );
    let late = rollup(&db, D0 + 1).await;
    assert_eq!(late.metrics.reviews, 3, "the late review counts");
    assert_eq!(late.updated_at, moved);
    assert!(second.revisited > 0, "the past days were revisited");
}
