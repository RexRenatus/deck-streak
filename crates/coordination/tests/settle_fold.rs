//! The recompute settles each study day once, in order, with the state it had at its close
//! (SPEC-071 A16 to A21; R15 to R19; ADR-071). Every review, card and deck is synthetic, and every
//! instant is set by hand: no test waits for time to pass.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_analytics::rollup::{RollupStore, StoredDay, fingerprint, recent_volumes};
use deck_streak_analytics::score::{ScoreState, baseline_window, compute_score, raw_streak};
use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_analytics::snapshot::{CardState, card_snapshot};
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::{
    DayEvaluation, DayStep, Evaluation, Fold, FoldError, FoldInput, FoldReport, PHASES, Phase,
};
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_kernel::{Db, KernelError, PortFuture, StudyDay, StudyDayRule, Track, UtcMillis};
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

/// R15's wiring, recorded under A18 (`docs/red-first/SPEC-071.md`): the sync cycle runs the fold after
/// its recompute's read, with the study day of the latest successful sync, so a cycle whose sync
/// failed evaluates the current day and settles nothing, and the next cycle whose sync succeeds
/// settles the day that closed before it. The deployment is a scratch one: an engine the test
/// switches from failing to succeeding, an empty copy the engine itself created, and the service's
/// database.
/// SPEC-071 R16, R17: which evaluations run the today-only rules, and which roll a day up again.
#[test]
fn each_evaluation_says_whether_it_runs_todays_rules_and_rolls_the_day_up_again() {
    let table = examined(
        "evaluations",
        vec![
            (
                Evaluation::Backfill {
                    reviews_changed: false,
                },
                false,
                false,
            ),
            (
                Evaluation::Backfill {
                    reviews_changed: true,
                },
                false,
                true,
            ),
            (Evaluation::Settle { end_of_day: false }, true, true),
            (Evaluation::Settle { end_of_day: true }, true, true),
            (Evaluation::Current, true, true),
            (
                Evaluation::Revisit {
                    reviews_changed: false,
                },
                false,
                false,
            ),
            (
                Evaluation::Revisit {
                    reviews_changed: true,
                },
                false,
                true,
            ),
        ],
    );
    for (evaluation, today_only, rerolls) in table {
        assert_eq!(
            (evaluation.runs_today_only_rules(), evaluation.rerolls()),
            (today_only, rerolls),
            "{evaluation:?}"
        );
    }
}

/// SPEC-071 R19: a fold lists its steps in the phases' order and, within a phase, in the order they
/// were registered; a phase reads as its number and its name.
#[test]
fn a_fold_lists_its_steps_by_phase_then_by_registration() {
    let log = Log::default();
    let mut fold = Fold::default();
    fold.register(Phase::BaseXp, probe(Phase::BaseXp, "probe.first", &log))
        .expect("the probe is phase 2's");
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    fold.register(Phase::BaseXp, probe(Phase::BaseXp, "probe.second", &log))
        .expect("the probe is phase 2's");

    let steps = vec![
        (Phase::RollupAndScore, "analytics.rollup_and_score"),
        (Phase::BaseXp, "probe.first"),
        (Phase::BaseXp, "probe.second"),
    ];
    assert_eq!(fold.steps(), steps);
    assert_eq!(format!("{fold:?}"), format!("{steps:?}"));
    assert_eq!(
        Phase::RollupAndScore.to_string(),
        "phase 1 (RollupAndScore)"
    );
    assert_eq!(Phase::BaseXp.to_string(), "phase 2 (BaseXp)");
}

/// SPEC-071 R14, R16: a clock stepped back to the last settled day evaluates it as the current
/// day, and never again as a past day to revisit.
#[tokio::test]
async fn a_clock_stepped_back_to_a_settled_day_evaluates_it_only_as_the_current_day() {
    let scratch = tempfile::tempdir().expect("a scratch");
    let db = database(&scratch).await;
    let log = Log::default();
    let fold = fold(&log);
    let data = collection(reviews_on(&[D0 - 3, D0 - 2, D0 - 1]), Vec::new());

    let first = recompute(&fold, &db, &data, at(D0, 12), D0).await;
    assert_eq!(first.settled, days(&[D0 - 1]), "{first:?}");
    log.lock().unwrap_or_else(PoisonError::into_inner).clear();

    let again = recompute(&fold, &db, &data, at(D0 - 1, 12), D0).await;
    let last: Vec<Evaluation> = seen(&log)
        .iter()
        .filter(|seen| seen.day == day(D0 - 1))
        .map(|seen| seen.evaluation)
        .collect();
    assert_eq!(last, [Evaluation::Current], "{again:?}");
}

/// The ERROR events this crate logs on the test's thread.
#[derive(Clone, Default)]
struct Errors(Arc<Mutex<Vec<String>>>);

impl Errors {
    fn logged(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl tracing::Subscriber for Errors {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let metadata = event.metadata();
        if *metadata.level() == tracing::Level::ERROR
            && metadata.target().starts_with("deck_streak_coordination")
        {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(metadata.target().to_owned());
        }
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

/// SPEC-071 R16: settling a day that no step of phase 1 rolled up is logged as an error, once, and
/// a fold whose phase 1 rolls the day up settles it without one.
#[tokio::test]
async fn settling_a_day_no_step_rolled_up_logs_one_error() {
    let errors = Errors::default();
    let _logging = tracing::subscriber::set_default(errors.clone());
    let data = collection(reviews_on(&[D0 - 1]), Vec::new());
    let log = Log::default();

    let scratch = tempfile::tempdir().expect("a scratch");
    let mut bare = Fold::default();
    bare.register(Phase::BaseXp, probe(Phase::BaseXp, "probe.base_xp", &log))
        .expect("the probe is phase 2's");
    let report = recompute(&bare, &database(&scratch).await, &data, at(D0, 12), D0).await;
    assert_eq!(report.settled, days(&[D0 - 1]));
    assert_eq!(errors.logged().len(), 1, "{:?}", errors.logged());

    let rolled = tempfile::tempdir().expect("a scratch");
    let report = recompute(&fold(&log), &database(&rolled).await, &data, at(D0, 12), D0).await;
    assert_eq!(report.settled, days(&[D0 - 1]));
    assert_eq!(
        errors.logged().len(),
        1,
        "no further error: {:?}",
        errors.logged()
    );
}

/// The digest of the owner's courses that [`recompute`] hands the fold.
const COURSES_DIGEST: &str = "0123456789abcdef";

/// Runs the fold over `data` at `now`, after a successful sync that started on study day
/// `synced_in`, with `courses_digest`, and hands back what it answered, a refusal included.
async fn run_fold(
    fold: &Fold,
    db: &Db,
    data: &CollectionData,
    now: i64,
    synced_in: i64,
    courses_digest: &str,
) -> Result<FoldReport, KernelError> {
    fold.run(
        db,
        &FoldInput {
            data,
            rule: StudyDayRule::default(),
            now: UtcMillis::from_epoch_millis(now),
            synced_in: Some(day(synced_in)),
            courses_digest: Some(courses_digest),
        },
    )
    .await
}

/// SPEC-071 R9, recorded under A6: a settled day that a late review re-rolls, once its cards have
/// moved, keeps the card state, the provenance and the score it closed with, and a backfilled day
/// that a late review re-rolls keeps the card state it never had. A6's own test holds analytics'
/// store to this; this one holds the fold's steps to it, whose revisit is the evaluation that
/// re-rolls a past day.
#[tokio::test]
async fn a_late_review_rerolls_a_settled_day_and_keeps_what_it_closed_with() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let log = Log::default();
    let fold = fold(&log);
    // At D0+1's close card 11 is due, card 12 is overdue and card 13 is a leech.
    let cards = vec![
        card(11, number(D0 + 1), 30, 0),
        card(12, number(D0 + 1) - 3, 5, 0),
        card(13, number(D0 + 1) + 40, 30, 9),
    ];
    let before = collection(reviews_on(&[D0, D0 + 1]), cards.clone());
    let first = recompute(&fold, &db, &before, at(D0 + 2, 5), D0 + 2).await;
    assert_eq!(first.backfilled, days(&[D0]));
    assert_eq!(first.settled, days(&[D0 + 1]));
    let closed = rollup(&db, D0 + 1).await;
    assert!(
        closed.card_state.is_some() && closed.score_at_close.is_some(),
        "the day closed with a card state and a score: {closed:?}"
    );

    // By the next recompute cards 11 and 12 have been answered again, and a review of each past
    // day reached the copy late.
    let moved = vec![
        card(11, number(D0 + 1) + 30, 60, 0),
        card(12, number(D0 + 1) + 1, 1, 1),
        card(13, number(D0 + 1) + 40, 30, 9),
    ];
    assert_ne!(
        card_snapshot(&moved, 8, number(D0 + 1)),
        card_snapshot(&cards, 8, number(D0 + 1)),
        "the cards' state at the closed day's own day number has moved"
    );
    let mut reviews = reviews_on(&[D0, D0 + 1]);
    reviews.extend([review(at(D0, 20), 3, 4), review(at(D0 + 1, 20), 3, 4)]);
    reviews.sort_by_key(|review| review.id);
    let after = collection(reviews, moved);
    let second = recompute(&fold, &db, &after, at(D0 + 2, 9), D0 + 2).await;
    assert_eq!(
        examined("re-rolled days", second.rerolled.clone()),
        days(&[D0, D0 + 1, D0 + 2]),
        "both past days are rolled up again, and the current day"
    );

    let again = rollup(&db, D0 + 1).await;
    assert_eq!(again.metrics.reviews, 3, "the late review counts");
    assert_eq!(
        again.card_state, closed.card_state,
        "the card state it closed with"
    );
    assert_eq!(
        again.card_state_src, closed.card_state_src,
        "its provenance"
    );
    assert_eq!(
        again.score_at_close, closed.score_at_close,
        "the score it closed with"
    );
    assert_eq!(again.settled_at, closed.settled_at, "its one settle");
    let backfilled = rollup(&db, D0).await;
    assert_eq!(backfilled.metrics.reviews, 3, "the late review counts");
    assert_eq!(
        (backfilled.card_state, backfilled.card_state_src),
        (None, None),
        "a day never recorded keeps no card state"
    );
}

/// A phase-7 step that counts each settle it is handed, in the day's own write: a count stays only
/// when the settle's write commits.
struct SettleCounter;

impl DayStep for SettleCounter {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        "probe.settle_counter"
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            if matches!(day.evaluation, Evaluation::Settle { .. }) {
                sqlx::query("INSERT INTO probe_settles (study_day) VALUES (?1)")
                    .bind(day.day.epoch_day())
                    .execute(&mut *write)
                    .await?;
            }
            Ok(())
        })
    }
}

/// Runs `statement` in a write of its own.
async fn execute(db: &Db, statement: &'static str) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(statement)
        .execute(&mut *write)
        .await
        .unwrap_or_else(|error| panic!("{statement}: {error}"));
    write.commit().await.expect("the write commits");
}

/// How many settles of study day `number` committed their steps' work.
async fn committed_settles(db: &Db, number: i64) -> i64 {
    let mut connection = db.reader().acquire().await.expect("a connection");
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM probe_settles WHERE study_day = ?1")
        .bind(number)
        .fetch_one(&mut *connection)
        .await
        .expect("the count reads")
}

/// SPEC-071 R16, recorded under A16: the cursor moves in the same write as the day's steps, so a
/// settle whose cursor cannot be recorded commits none of its steps' work, and the recompute that
/// settles the day at last commits it once. A trigger the test installs refuses the cursor's
/// write, the one statement that names `settled_at`: a failure inside a step could not tell one
/// write from a settle split in two, whose steps would commit before its cursor.
#[tokio::test]
async fn a_settle_whose_cursor_is_refused_commits_none_of_its_steps_work() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let log = Log::default();
    let mut fold = fold(&log);
    fold.register(Phase::Awards, Box::new(SettleCounter))
        .expect("the counter is phase 7's");
    execute(
        &db,
        "CREATE TABLE probe_settles (study_day INTEGER NOT NULL) STRICT",
    )
    .await;
    execute(
        &db,
        "CREATE TRIGGER refuse_the_cursor BEFORE UPDATE OF settled_at ON daily_rollup \
         BEGIN SELECT RAISE(ABORT, 'the cursor is refused'); END",
    )
    .await;
    let data = collection(reviews_on(&[D0, D0 + 1]), Vec::new());

    let refused = run_fold(&fold, &db, &data, at(D0 + 1, 10), D0 + 1, COURSES_DIGEST).await;
    assert!(
        refused.is_err(),
        "the refused cursor fails the recompute: {refused:?}"
    );
    let evaluated: Vec<StudyDay> = seen(&log)
        .into_iter()
        .filter(|seen| matches!(seen.evaluation, Evaluation::Settle { .. }))
        .map(|seen| seen.day)
        .collect();
    assert_eq!(
        evaluated,
        days(&[D0]),
        "the settle's steps ran before its cursor was refused"
    );
    assert_eq!(
        committed_settles(&db, D0).await,
        0,
        "the refused settle committed none of its steps' work"
    );

    execute(&db, "DROP TRIGGER refuse_the_cursor").await;
    let settled = recompute(&fold, &db, &data, at(D0 + 1, 11), D0 + 1).await;
    assert_eq!(
        settled.settled,
        days(&[D0]),
        "the day stayed owed, and is settled now"
    );
    assert_eq!(
        committed_settles(&db, D0).await,
        1,
        "its steps' work is committed once, with its one settle"
    );
    let again = recompute(&fold, &db, &data, at(D0 + 1, 12), D0 + 1).await;
    assert_eq!(
        again.settled,
        Vec::<StudyDay>::new(),
        "a second recompute settles no day"
    );
    assert_eq!(
        committed_settles(&db, D0).await,
        1,
        "and commits no settle's work"
    );
}

/// A review field as the fingerprint's table names it, and a change to it.
type FieldChange = (&'static str, fn(&mut Review));

/// SPEC-071 R18, recorded under A21: a day's fingerprint digests every field of each of its study
/// reviews and the courses file's digest, so a recompute rolls a past day up again when any one of
/// them changed, and no other past day. cargo-mutants never removes one element of the list the
/// fingerprint digests, so each field has its own row here.
#[tokio::test]
async fn a_change_to_any_review_field_or_to_the_courses_rerolls_the_day() {
    let reviews = reviews_on(&[D0, D0 + 1]);
    let fields: Vec<FieldChange> = vec![
        ("id", |review| review.id += 1_000),
        ("card_id", |review| review.card_id += 10),
        ("ease", |review| review.ease += 1),
        ("interval", |review| review.interval += 1),
        ("last_interval", |review| review.last_interval += 1),
        ("factor", |review| review.factor += 100),
        ("taken_ms", |review| review.taken_ms += 1),
        ("kind", |review| review.kind += 1),
    ];
    assert_eq!(
        rerolled_after(&reviews, &reviews, COURSES_DIGEST).await,
        days(&[D0 + 2]),
        "with nothing changed, only the current day is rolled up again"
    );
    for (field, change) in examined("review fields", fields) {
        let mut changed = reviews.clone();
        change(&mut changed[0]);
        assert_ne!(
            fingerprint(&changed[..2], Some(COURSES_DIGEST)),
            fingerprint(&reviews[..2], Some(COURSES_DIGEST)),
            "the fingerprint digests a review's {field}"
        );
        assert_eq!(
            rerolled_after(&reviews, &changed, COURSES_DIGEST).await,
            days(&[D0, D0 + 2]),
            "a changed {field} rolls its day up again, beside the current day"
        );
    }

    // Another courses file: every past day's fingerprint changes with it.
    let other = "fedcba9876543210";
    assert_ne!(
        fingerprint(&reviews[..2], Some(other)),
        fingerprint(&reviews[..2], Some(COURSES_DIGEST)),
        "the fingerprint digests the courses file's digest"
    );
    assert_eq!(
        rerolled_after(&reviews, &reviews, other).await,
        days(&[D0, D0 + 1, D0 + 2]),
        "a changed courses file rolls every past day up again"
    );
}

/// The days a recompute over `after`, with `courses_digest`, rolls up again, following a first
/// recompute over `before` that backfilled D0 and settled D0+1. A past day's rollup carries the
/// later recompute's instant exactly when that recompute says it rolled the day up again.
async fn rerolled_after(
    before: &[Review],
    after: &[Review],
    courses_digest: &str,
) -> Vec<StudyDay> {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let fold = fold(&Log::default());
    let first = recompute(
        &fold,
        &db,
        &collection(before.to_vec(), Vec::new()),
        at(D0 + 2, 10),
        D0 + 2,
    )
    .await;
    assert_eq!(
        (first.backfilled, first.settled),
        (days(&[D0]), days(&[D0 + 1]))
    );
    let now = at(D0 + 2, 12);
    let report = run_fold(
        &fold,
        &db,
        &collection(after.to_vec(), Vec::new()),
        now,
        D0 + 2,
        courses_digest,
    )
    .await
    .expect("the fold runs");
    for past in [D0, D0 + 1] {
        let moved = rollup(&db, past).await.updated_at == UtcMillis::from_epoch_millis(now);
        assert_eq!(
            moved,
            report.rerolled.contains(&day(past)),
            "study day {past}'s rollup says whether it was rolled up again"
        );
    }
    db.close().await;
    report.rerolled
}

mod cycle {
    use std::ffi::OsStr;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, PoisonError};

    use deck_streak_analytics::rollup::RollupStore;
    use deck_streak_coordination::obligations::Obligations;
    use deck_streak_coordination::recompute::analytics_step::{ANALYTICS_STEP, AnalyticsStep};
    use deck_streak_coordination::recompute::{Fold, Phase};
    use deck_streak_coordination::sync_cycle::{CycleParts, Recompute, sync_cycle};
    use deck_streak_ingest::engine::{
        AnkiEngine, EngineError, NewCardQueue, RslibEngine, SyncLogin, SyncOutcome,
    };
    use deck_streak_ingest::gate::ChangeGate;
    use deck_streak_ingest::reader::CollectionReader;
    use deck_streak_ingest::settings::{
        STATE_DIRECTORY, SYNC_ENDPOINT, SYNC_PASSWORD, SYNC_USERNAME, ScopeSettings, SyncSettings,
    };
    use deck_streak_ingest::state::SqliteIngestState;
    use deck_streak_ingest::sync::{SyncReport, Syncer};
    use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
    use deck_streak_kernel::{
        CredentialLoader, CredentialsDirectory, Db, Environment, ManualClock, Offload,
        OffloadWorkers, Redactor, StudyDay, StudyDayRule, UtcMillis,
    };

    use super::{D0, DAY_MS, HOUR_MS, day};

    /// An engine whose syncs fail until the test lets them succeed; a sync that succeeds finds no
    /// change, so the copy stays the empty collection the engine created.
    #[derive(Clone, Default)]
    struct Switched(Arc<AtomicBool>);

    impl AnkiEngine for Switched {
        fn new_card_queue(&self, _collection: &Path) -> Result<NewCardQueue, EngineError> {
            Ok(NewCardQueue::default())
        }

        async fn normal_sync(
            &self,
            _collection: &Path,
            _login: &SyncLogin,
        ) -> Result<SyncOutcome, EngineError> {
            if self.0.load(Ordering::SeqCst) {
                Ok(SyncOutcome::NoChanges)
            } else {
                Err(EngineError::AuthRejected)
            }
        }

        async fn full_download(
            &self,
            _collection: &Path,
            _login: &SyncLogin,
        ) -> Result<(), EngineError> {
            Ok(())
        }
    }

    /// The days of the window's last two study days that have a rollup, and which are settled.
    async fn rolled(db: &Db) -> Vec<(StudyDay, bool)> {
        RollupStore::new(db.clone())
            .days(day(D0 - 1), day(D0))
            .await
            .expect("the rollups read")
            .into_iter()
            .map(|stored| (stored.metrics.day, stored.settled_at.is_some()))
            .collect()
    }

    #[tokio::test]
    async fn a_sync_cycle_runs_the_fold_and_settles_only_after_a_successful_sync() {
        let scratch = tempfile::tempdir().expect("a scratch");
        let state = scratch.path().join("state");
        let credentials = scratch.path().join("credentials");
        for folder in [&state, &credentials] {
            fs::create_dir_all(folder).expect("a folder");
        }
        for (id, value) in [
            (SYNC_USERNAME, "synthetic-owner\n"),
            (SYNC_PASSWORD, "synthetic-password\n"),
        ] {
            fs::write(credentials.join(id), value).expect("a credential");
        }
        let settings = SyncSettings::from_env(&Environment::from_vars([
            (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
            (STATE_DIRECTORY, state.as_os_str()),
        ]))
        .expect("the settings");
        RslibEngine
            .new_card_queue(&settings.copy_path())
            .expect("the engine creates the copy");
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .expect("the database");
        // The study day D0, an hour past its rollover: D0 - 1 has just closed.
        let first = D0 * DAY_MS + 5 * HOUR_MS;
        let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(first)));
        let engine = Switched::default();
        let syncer = Syncer::new(
            engine.clone(),
            SqliteSyncRuns::new(db.clone()),
            settings.clone(),
            CredentialLoader::new(
                CredentialsDirectory::new(credentials).expect("an absolute directory"),
                Redactor::new(),
            ),
            clock.clone(),
            StudyDayRule::default(),
        );
        let offload = Offload::new(OffloadWorkers::new(1).expect("one worker"), clock.clone());
        let reader = CollectionReader::new(&settings, ScopeSettings::default(), offload);
        let gate = ChangeGate::new(db.clone(), StudyDayRule::default(), clock.clone());
        let mut fold = Fold::default();
        fold.register(Phase::RollupAndScore, Box::new(AnalyticsStep::default()))
            .expect("phase 1's step");
        let cycle = CycleParts::new(syncer, reader, gate, Obligations::new(), clock.clone())
            .with_fold(Arc::new(fold), db.clone(), StudyDayRule::default(), None);
        assert_eq!(
            cycle.fold().map(Fold::steps),
            Some(vec![(Phase::RollupAndScore, ANALYTICS_STEP)]),
            "the cycle holds the fold it was handed"
        );

        // The sync fails, and no sync ever succeeded: the recompute still runs, and the fold rolls
        // the current day up, but the day that closed stays owed.
        let failed = sync_cycle(&cycle, Trigger::Owner)
            .await
            .expect("the cycle runs");
        assert!(
            matches!(failed.recompute, Recompute::Ran { .. }),
            "the gate never skips after a failed sync: {:?}",
            failed.recompute
        );
        assert_eq!(
            rolled(&db).await,
            [(day(D0), false)],
            "the fold ran for the current day, and settled nothing"
        );

        // The owner's next cycle, ten minutes on, syncs: the closed day is settled by it.
        engine.0.store(true, Ordering::SeqCst);
        let second = first + 10 * 60_000;
        clock.set(UtcMillis::from_epoch_millis(second));
        let succeeded = sync_cycle(&cycle, Trigger::Owner)
            .await
            .expect("the cycle runs");
        assert!(matches!(succeeded.recompute, Recompute::Ran { .. }));
        assert_eq!(
            rolled(&db).await,
            [(day(D0 - 1), true), (day(D0), false)],
            "the successful sync's recompute settled the closed day"
        );
        let settled = RollupStore::new(db.clone())
            .days(day(D0 - 1), day(D0 - 1))
            .await
            .expect("the rollup reads");
        assert_eq!(
            settled[0].settled_at,
            Some(UtcMillis::from_epoch_millis(second)),
            "settled by the cycle that followed the successful sync"
        );
        db.close().await;
    }

    /// A scratch deployment's cycle over `engine`, reading `clock`: the owner's two credentials,
    /// the empty copy the engine itself creates, the service's database, and a fold of analytics'
    /// step whose days carry `courses_digest`.
    async fn deployment<E: AnkiEngine + Sync>(
        scratch: &Path,
        engine: E,
        clock: &Arc<ManualClock>,
        courses_digest: Option<&str>,
    ) -> (Db, CycleParts<E>) {
        let state = scratch.join("state");
        let credentials = scratch.join("credentials");
        for folder in [&state, &credentials] {
            fs::create_dir_all(folder).expect("a folder");
        }
        for (id, value) in [
            (SYNC_USERNAME, "synthetic-owner\n"),
            (SYNC_PASSWORD, "synthetic-password\n"),
        ] {
            fs::write(credentials.join(id), value).expect("a credential");
        }
        let settings = SyncSettings::from_env(&Environment::from_vars([
            (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
            (STATE_DIRECTORY, state.as_os_str()),
        ]))
        .expect("the settings");
        RslibEngine
            .new_card_queue(&settings.copy_path())
            .expect("the engine creates the copy");
        let db = Db::open(&scratch.join("deckstreak.db"))
            .await
            .expect("the database");
        let syncer = Syncer::new(
            engine,
            SqliteSyncRuns::new(db.clone()),
            settings.clone(),
            CredentialLoader::new(
                CredentialsDirectory::new(credentials).expect("an absolute directory"),
                Redactor::new(),
            ),
            clock.clone(),
            StudyDayRule::default(),
        );
        let offload = Offload::new(OffloadWorkers::new(1).expect("one worker"), clock.clone());
        let reader = CollectionReader::new(&settings, ScopeSettings::default(), offload);
        let gate = ChangeGate::new(db.clone(), StudyDayRule::default(), clock.clone());
        let mut fold = Fold::default();
        fold.register(Phase::RollupAndScore, Box::new(AnalyticsStep::default()))
            .expect("phase 1's step");
        let cycle = CycleParts::new(syncer, reader, gate, Obligations::new(), clock.clone())
            .with_fold(
                Arc::new(fold),
                db.clone(),
                StudyDayRule::default(),
                courses_digest.map(str::to_owned),
            );
        (db, cycle)
    }

    /// An engine whose syncs succeed and find no change; the first one moves the clock to `finish`
    /// while it runs, as a sync that started before a rollover and finished after it.
    #[derive(Clone)]
    struct AcrossTheRollover {
        clock: Arc<ManualClock>,
        finish: Arc<Mutex<Option<UtcMillis>>>,
    }

    impl AnkiEngine for AcrossTheRollover {
        fn new_card_queue(&self, _collection: &Path) -> Result<NewCardQueue, EngineError> {
            Ok(NewCardQueue::default())
        }

        async fn normal_sync(
            &self,
            _collection: &Path,
            _login: &SyncLogin,
        ) -> Result<SyncOutcome, EngineError> {
            let finish = self
                .finish
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take();
            if let Some(finish) = finish {
                self.clock.set(finish);
            }
            Ok(SyncOutcome::NoChanges)
        }

        async fn full_download(
            &self,
            _collection: &Path,
            _login: &SyncLogin,
        ) -> Result<(), EngineError> {
            Ok(())
        }
    }

    /// SPEC-071 R15, recorded under A18: a successful sync that started before the rollover and
    /// finished after it is the sync of the day that closed, so that day stays owed; the owner's
    /// next sync, which starts after the close, settles it.
    #[tokio::test]
    async fn a_sync_across_the_rollover_leaves_the_day_it_started_in_owed() {
        let scratch = tempfile::tempdir().expect("a scratch");
        // D0 - 1 closes at D0's rollover, 04:00 UTC: the sync starts ten minutes before it and
        // finishes five minutes after it.
        let rollover = D0 * DAY_MS + 4 * HOUR_MS;
        let started = UtcMillis::from_epoch_millis(rollover - 10 * 60_000);
        let finished = UtcMillis::from_epoch_millis(rollover + 5 * 60_000);
        let rule = StudyDayRule::default();
        assert_eq!(
            (rule.study_day(started), rule.study_day(finished)),
            (day(D0 - 1), day(D0)),
            "the sync starts in D0 - 1 and finishes in D0"
        );
        let clock = Arc::new(ManualClock::new(started));
        let engine = AcrossTheRollover {
            clock: Arc::clone(&clock),
            finish: Arc::new(Mutex::new(Some(finished))),
        };
        let (db, cycle) = deployment(scratch.path(), engine, &clock, None).await;

        let across = sync_cycle(&cycle, Trigger::Owner)
            .await
            .expect("the cycle runs");
        let SyncReport::Ran { run, .. } = &across.sync else {
            panic!("the sync ran: {:?}", across.sync);
        };
        assert!(run.outcome.is_ok(), "the sync succeeded: {run:?}");
        assert_eq!(
            (run.started_at, run.finished_at, run.study_day),
            (started, finished, day(D0 - 1)),
            "it ran across the rollover, and is recorded in the day it started in"
        );
        assert!(
            matches!(across.recompute, Recompute::Ran { .. }),
            "{:?}",
            across.recompute
        );
        assert_eq!(
            rolled(&db).await,
            [(day(D0), false)],
            "the fold rolled the current day up, and the day that closed stays owed"
        );

        // The owner's next /sync, ten minutes on, marks a rescore as every owner's sync does; its
        // sync starts after the close, and its recompute settles the day.
        let next = UtcMillis::from_epoch_millis(finished.epoch_millis() + 10 * 60_000);
        clock.set(next);
        SqliteIngestState::new(db.clone())
            .request_rescore(next)
            .await
            .expect("the rescore is marked");
        let after = sync_cycle(&cycle, Trigger::Owner)
            .await
            .expect("the cycle runs");
        assert!(
            matches!(after.recompute, Recompute::Ran { .. }),
            "{:?}",
            after.recompute
        );
        assert_eq!(
            rolled(&db).await,
            [(day(D0 - 1), true), (day(D0), false)],
            "the sync that started after the close settled the day"
        );
        db.close().await;
    }

    /// SPEC-071 R4, recorded under A14: a run of the sync job records the courses' digest at its
    /// start, then runs its cycle. With the courses file unchanged, neither a later start nor any
    /// cycle's recompute moves the settings generation.
    #[tokio::test]
    async fn cycles_with_an_unchanged_courses_file_leave_the_settings_generation_alone() {
        let scratch = tempfile::tempdir().expect("a scratch");
        let first = D0 * DAY_MS + 5 * HOUR_MS;
        let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(first)));
        let courses = "0123456789abcdef";
        let engine = Switched::default();
        engine.0.store(true, Ordering::SeqCst);
        let (db, cycle) = deployment(scratch.path(), engine, &clock, Some(courses)).await;
        assert!(
            db.record_courses_digest(Some(courses))
                .await
                .expect("recorded"),
            "the first start records the digest"
        );
        assert_eq!(db.settings_generation().await.expect("readable"), 1);

        // The scheduled job, on two study days in turn.
        for run in 0..2 {
            clock.set(UtcMillis::from_epoch_millis(first + run * DAY_MS));
            assert!(
                !db.record_courses_digest(Some(courses))
                    .await
                    .expect("recorded"),
                "run {run}: the job's start finds the courses unchanged"
            );
            let report = sync_cycle(&cycle, Trigger::Scheduled)
                .await
                .expect("the cycle runs");
            assert!(
                matches!(report.recompute, Recompute::Ran { .. }),
                "run {run}: the recompute ran: {:?}",
                report.recompute
            );
            assert_eq!(
                db.settings_generation().await.expect("readable"),
                1,
                "run {run}: the generation stays where the first start put it"
            );
        }
        assert_eq!(
            rolled(&db).await,
            [(day(D0 - 1), true), (day(D0), true)],
            "each run's fold settled the day that closed before it"
        );
        db.close().await;
    }
}
