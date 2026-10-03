//! The relight (SPEC-076 A17, A18, A24, A25; R16, R18, R26, R27): a return day with three reviews
//! earns its XP once per episode, on the fold's own connection, and its celebration is routed after
//! the fold's commit. Every review, card and instant is synthetic and set by hand.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::streaks::{RelightDue, StreaksStep};
use deck_streak_coordination::recompute::{DayEvaluation, DayStep, Fold, FoldInput, Phase};
use deck_streak_coordination::relight::route_due_relights;
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_kernel::{Db, ManualClock, PortFuture, StudyDay, StudyDayRule, Track, UtcMillis};
use deck_streak_notifications::{BotTransport, Pass, Policy, PushFuture, Pushed, Router};
use sqlx::{Row, SqliteConnection};
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
const D0: i64 = 20_000;
const CREATED: i64 = D0 - 1_000;

const fn at(day: i64, hour: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS
}

const fn review(instant: i64) -> Review {
    Review {
        id: instant,
        card_id: 1,
        ease: 3,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind: 1,
    }
}

fn data(reviews: Vec<Review>) -> CollectionData {
    CollectionData {
        reviews,
        cards: vec![Card {
            id: 1,
            note_id: 1,
            deck_id: 1,
            original_deck_id: 0,
            queue: 2,
            kind: 2,
            due: 1_090,
            interval: 30,
            factor: 2500,
            reps: 3,
            lapses: 0,
            track: Track::Language,
            course: None,
            tier: None,
            memory: None,
        }],
        created_at: UtcMillis::from_epoch_millis(at(CREATED, 12)),
        deck_names: BTreeMap::from([(1, "Synthetic".to_owned())]),
    }
}

/// Five old study days, a silence of nine days, and a return day with `returned` reviews.
fn history(returned: i64) -> CollectionData {
    let mut reviews: Vec<Review> = (D0 - 20..=D0 - 16).map(|d| review(at(d, 9))).collect();
    reviews.extend((0..returned).map(|n| review(at(D0, 8 + n))));
    data(reviews)
}

/// A phase 4 step that reads, on the fold's own connection, how many relight grants it can see.
struct Probe(Arc<Mutex<Vec<i64>>>);

impl DayStep for Probe {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        "relight_settle.probe"
    }

    fn evaluate<'a>(
        &'a self,
        _day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let seen: i64 =
                sqlx::query("SELECT COUNT(*) FROM xp_ledger WHERE source LIKE 'relight:%'")
                    .fetch_one(&mut *write)
                    .await
                    .map_err(deck_streak_kernel::KernelError::Database)?
                    .get(0);
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(seen);
            Ok(())
        })
    }
}

fn fold(probe: Option<Arc<Mutex<Vec<i64>>>>) -> (Fold, RelightDue) {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    let (step, due) = StreaksStep::new();
    fold.register(Phase::StreaksAndGovernor, Box::new(step))
        .expect("the streaks step is phase 3's");
    if let Some(seen) = probe {
        fold.register(Phase::DaySteps, Box::new(Probe(seen)))
            .expect("the probe is phase 4's");
    }
    (fold, due)
}

async fn recompute(fold: &Fold, db: &Db, data: &CollectionData, now: i64, synced_in: i64) {
    fold.run(
        db,
        &FoldInput {
            data,
            rule: StudyDayRule::default(),
            now: UtcMillis::from_epoch_millis(now),
            synced_in: Some(StudyDay::from_epoch_day(synced_in)),
            courses_digest: Some("0123456789abcdef"),
            base_reviews: 0,
            offers: None,
        },
    )
    .await
    .expect("the fold runs");
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

/// `(study day, amount)` of every relight grant the ledger holds.
async fn grants(db: &Db) -> Vec<(i64, i64)> {
    let mut write = db.write().await.expect("a write");
    sqlx::query("SELECT study_day, amount FROM xp_ledger WHERE source LIKE 'relight:%' ORDER BY id")
        .fetch_all(&mut *write)
        .await
        .expect("the ledger reads")
        .into_iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect()
}

#[tokio::test]
async fn a_lapse_relights_once_per_episode() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let (fold, _due) = fold(None);
    let data = history(3);
    // The lapse is open on the eve of the return day.
    recompute(&fold, &db, &history(0), at(D0 - 1, 12), D0 - 1).await;
    assert_eq!(
        grants(&db).await,
        Vec::<(i64, i64)>::new(),
        "a silence earns nothing"
    );
    recompute(&fold, &db, &data, at(D0, 12), D0).await;
    recompute(&fold, &db, &data, at(D0, 18), D0).await;
    recompute(&fold, &db, &data, at(D0 + 1, 12), D0 + 1).await;
    assert_eq!(
        grants(&db).await,
        vec![(D0, 100)],
        "three recomputes of the return day grant its relight once"
    );
}

#[tokio::test]
async fn a_return_day_relights_at_its_settle_with_its_whole_count() {
    for (returned, want) in [(2_i64, Vec::<(i64, i64)>::new()), (3, vec![(D0, 100)])] {
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        let (fold, _due) = fold(None);
        recompute(&fold, &db, &history(0), at(D0 - 1, 12), D0 - 1).await;
        // No recompute runs while the return day is current: its settle is the first to see it.
        recompute(&fold, &db, &history(returned), at(D0 + 1, 12), D0 + 1).await;
        assert_eq!(
            grants(&db).await,
            want,
            "{returned} reviews on the return day"
        );
    }
}

#[tokio::test]
async fn a_relight_is_granted_on_the_folds_connection() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (fold, _due) = fold(Some(seen.clone()));
    recompute(&fold, &db, &history(0), at(D0 - 1, 12), D0 - 1).await;
    seen.lock().unwrap_or_else(PoisonError::into_inner).clear();
    // A second writer would wait out the busy timeout against the fold's held transaction.
    tokio::time::timeout(
        Duration::from_secs(10),
        recompute(&fold, &db, &history(3), at(D0, 12), D0),
    )
    .await
    .expect("the recompute completes without a second writer");
    let visible = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert!(
        visible.last().is_some_and(|count| *count == 1),
        "the grant is on the fold's connection before phase 4 reads it: {visible:?}"
    );
}

#[derive(Default)]
struct Recording(Mutex<Vec<String>>);

impl Recording {
    fn pushes(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

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
async fn a_second_recompute_routes_the_relight_and_one_send_is_recorded() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(at(D0, 12))));
    let bot = Arc::new(Recording::default());
    let router = Router::new(
        Arc::new(Policy::compiled().expect("the compiled policy parses")),
        db.clone(),
        clock,
        StudyDayRule::default(),
    )
    .with_bot(bot.clone());
    let (fold, due) = fold(None);
    recompute(&fold, &db, &history(0), at(D0 - 1, 12), D0 - 1).await;
    let today = StudyDay::from_epoch_day(D0);
    for now in [at(D0, 12), at(D0, 18)] {
        recompute(&fold, &db, &history(3), now, D0).await;
        assert_eq!(
            due.pending(&db).await.expect("the due relights read"),
            vec![today],
            "every qualifying recompute answers the relight as due"
        );
        route_due_relights(&router, &due, &db, today)
            .await
            .expect("the relight routes");
        assert_eq!(
            due.pending(&db).await.expect("the due relights read"),
            Vec::<StudyDay>::new(),
            "a relight the router decided is no longer due"
        );
    }
    assert_eq!(
        bot.pushes().len(),
        1,
        "the router's once-ever dedupe sends one line"
    );
    assert_eq!(grants(&db).await, vec![(D0, 100)]);
}
