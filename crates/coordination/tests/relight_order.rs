//! The relight's order (SPEC-076 A44, A45, A46; R27, R28): a day is due for its celebration exactly
//! when its relight grant commits, and it stays due across a failed fold and a crash until the
//! router decides it. The properties, over every case below: at most one celebration per relight
//! day (S1); no celebration without a committed grant (S2); and every committed grant is celebrated
//! once the recomputes run (L1), also when a crash falls between the fold's commit and the route.
//! Every review, card and instant is synthetic and set by hand.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::streaks::{RelightDue, StreaksStep};
use deck_streak_coordination::recompute::{DayEvaluation, DayStep, Fold, FoldInput, Phase};
use deck_streak_coordination::relight::route_due_relights;
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_kernel::{
    Db, KernelError, ManualClock, PortFuture, StudyDay, StudyDayRule, Track, UtcMillis,
};
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
        }],
        created_at: UtcMillis::from_epoch_millis(at(CREATED, 12)),
        deck_names: BTreeMap::from([(1, "Synthetic".to_owned())]),
    }
}

/// Five old study days, a silence of nine days, and a return day with `returned` reviews: three
/// qualify for the relight, two do not.
fn history(returned: i64) -> CollectionData {
    let mut reviews: Vec<Review> = (D0 - 20..=D0 - 16).map(|d| review(at(d, 9))).collect();
    reviews.extend((0..returned).map(|n| review(at(D0, 8 + n))));
    data(reviews)
}

/// A phase 4 step that fails the write of its armed day once: the write rolls back after the
/// streak step has run on it, and the fold returns an error.
struct FailOnce(Arc<Mutex<Option<i64>>>);

impl DayStep for FailOnce {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        "relight_order.fail_once"
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        _write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let mut armed = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if *armed == Some(day.day.epoch_day()) {
                *armed = None;
                return Err(KernelError::Database(sqlx::Error::Protocol(
                    "a later step of the day's write fails".to_owned(),
                )));
            }
            Ok(())
        })
    }
}

#[derive(Default)]
struct Recording(Mutex<Vec<String>>);

impl Recording {
    fn sends(&self) -> usize {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).len()
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

/// One run of the daemon: its fold, the handle its cycle routes the relights from, and the failure
/// armed in it. A crash drops it, and a restart starts a new one over the same database.
struct Process {
    fold: Fold,
    due: RelightDue,
    armed: Arc<Mutex<Option<i64>>>,
}

fn start() -> Process {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    let (step, due) = StreaksStep::new();
    fold.register(Phase::StreaksAndGovernor, Box::new(step))
        .expect("the streaks step is phase 3's");
    let armed = Arc::new(Mutex::new(None));
    fold.register(Phase::DaySteps, Box::new(FailOnce(armed.clone())))
        .expect("the failing step is phase 4's");
    Process { fold, due, armed }
}

/// What outlives a process: the database, the router over it, and the bot's record of sends.
struct World {
    _scratch: TempDir,
    db: Db,
    clock: Arc<ManualClock>,
    router: Router,
    bot: Arc<Recording>,
}

async fn world() -> World {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(at(
        D0 - 1,
        12,
    ))));
    let bot = Arc::new(Recording::default());
    let router = Router::new(
        Arc::new(Policy::compiled().expect("the compiled policy parses")),
        db.clone(),
        clock.clone(),
        StudyDayRule::default(),
    )
    .with_bot(bot.clone());
    World {
        _scratch: scratch,
        db,
        clock,
        router,
        bot,
    }
}

/// How a cycle ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ended {
    /// The fold returned an error, so the cycle routed nothing.
    Failed,
    /// The fold committed, and the process crashed before the route.
    Crashed,
    /// The fold committed, and the due relights were routed.
    Routed,
}

/// One sync cycle at `now` over `data`: the fold, then, as the cycle does after a fold that
/// commits, the route of every due relight. With `crash`, the process dies between the fold's
/// commit and the route, and `process` is replaced by a restarted one.
async fn cycle(
    world: &World,
    process: &mut Process,
    data: &CollectionData,
    now: i64,
    crash: bool,
) -> Ended {
    let rule = StudyDayRule::default();
    let today = rule.study_day(UtcMillis::from_epoch_millis(now));
    world.clock.set(UtcMillis::from_epoch_millis(now));
    let ran = process
        .fold
        .run(
            &world.db,
            &FoldInput {
                data,
                rule,
                now: UtcMillis::from_epoch_millis(now),
                synced_in: Some(today),
                courses_digest: Some("0123456789abcdef"),
            },
        )
        .await;
    if ran.is_err() {
        return Ended::Failed;
    }
    if crash {
        *process = start();
        return Ended::Crashed;
    }
    route_due_relights(&world.router, &process.due, &world.db, today)
        .await
        .expect("the due relights route");
    Ended::Routed
}

/// The study days of every relight grant the ledger holds.
async fn grants(db: &Db) -> Vec<i64> {
    let mut write = db.write().await.expect("a write");
    sqlx::query("SELECT study_day FROM xp_ledger WHERE source LIKE 'relight:%' ORDER BY id")
        .fetch_all(&mut *write)
        .await
        .expect("the ledger reads")
        .into_iter()
        .map(|row| row.get(0))
        .collect()
}

/// Where the return day's grant is first written: at a recompute while the day is current, or at
/// its settle, when its reviews first reach the sync after the day has closed.
#[derive(Clone, Copy, Debug)]
enum GrantAt {
    Current,
    Settle,
}

/// Where the return cycle's fold fails, after its streak step has run: nowhere, on the return
/// day's own write, or on the write after it (the settle's grant is then committed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Failure {
    None,
    GrantWrite,
    LaterWrite,
}

#[derive(Clone, Copy, Debug)]
struct Case {
    grant_at: GrantAt,
    failure: Failure,
    later_qualifies: bool,
    crash: bool,
}

/// Every fold-failure point, crossed with the later sync's outcome (the return day still qualifies,
/// or an undo leaves it two reviews) and with a crash between the first committed fold and its
/// route. The current day's write is the fold's last, so no later write follows it.
fn population() -> Vec<Case> {
    let mut cases = Vec::new();
    for grant_at in [GrantAt::Current, GrantAt::Settle] {
        for failure in [Failure::None, Failure::GrantWrite, Failure::LaterWrite] {
            if matches!(grant_at, GrantAt::Current) && failure == Failure::LaterWrite {
                continue;
            }
            for later_qualifies in [true, false] {
                for crash in [false, true] {
                    cases.push(Case {
                        grant_at,
                        failure,
                        later_qualifies,
                        crash,
                    });
                }
            }
        }
    }
    cases
}

/// Runs `case` and answers the properties it breaks: the eve of the return day opens the lapse;
/// the return cycle sees three reviews and fails where the case says; the later sync the same day
/// sees three or two; a crash falls after the first fold that commits; then two fair days of
/// cycles recompute with nothing failing.
async fn run(case: Case) -> Vec<String> {
    let world = world().await;
    let mut process = start();
    let ended = cycle(&world, &mut process, &history(0), at(D0 - 1, 12), false).await;
    assert_eq!(ended, Ended::Routed, "the eve's cycle routes");
    let first = match case.grant_at {
        GrantAt::Current => D0,
        GrantAt::Settle => D0 + 1,
    };
    let failing = match case.failure {
        Failure::None => None,
        Failure::GrantWrite => Some(D0),
        Failure::LaterWrite => Some(D0 + 1),
    };
    *process.armed.lock().unwrap_or_else(PoisonError::into_inner) = failing;
    let later = history(if case.later_qualifies { 3 } else { 2 });
    let mut crash = case.crash;
    for (data, now) in [
        (&history(3), at(first, 12)),
        (&later, at(first, 18)),
        (&later, at(first + 1, 12)),
        (&later, at(first + 1, 18)),
        (&later, at(first + 2, 12)),
    ] {
        if cycle(&world, &mut process, data, now, crash).await == Ended::Crashed {
            crash = false;
        }
    }
    let sends = world.bot.sends();
    let granted = grants(&world.db).await.contains(&D0);
    let mut broken = Vec::new();
    if sends > 1 {
        broken.push(format!("S1 {case:?}: {sends} celebrations"));
    }
    if sends >= 1 && !granted {
        broken.push(format!(
            "S2 {case:?}: a celebration with no committed grant"
        ));
    }
    if granted && sends == 0 {
        broken.push(format!("L1 {case:?}: a committed grant never celebrated"));
    }
    broken
}

#[tokio::test]
async fn a_celebration_is_sent_only_for_a_grant_that_committed() {
    let world = world().await;
    let mut process = start();
    cycle(&world, &mut process, &history(0), at(D0 - 1, 12), false).await;
    *process.armed.lock().unwrap_or_else(PoisonError::into_inner) = Some(D0);
    let ended = cycle(&world, &mut process, &history(3), at(D0, 12), false).await;
    assert_eq!(ended, Ended::Failed, "the return day's write rolls back");
    assert_eq!(
        process.due.pending(&world.db).await.expect("the due read"),
        Vec::<StudyDay>::new(),
        "a write that rolled back leaves no day due"
    );
    // The later sync reads two reviews for the return day (an undo): it no longer qualifies.
    let ended = cycle(&world, &mut process, &history(2), at(D0, 18), false).await;
    assert_eq!(ended, Ended::Routed, "the later cycle's fold commits");
    assert_eq!(
        grants(&world.db).await,
        Vec::<i64>::new(),
        "no grant committed"
    );
    assert_eq!(
        world.bot.sends(),
        0,
        "no celebration is sent for a grant that never committed"
    );
}

#[tokio::test]
async fn a_crash_between_the_commit_and_the_route_is_recovered_at_the_next_cycle() {
    let world = world().await;
    let mut process = start();
    cycle(&world, &mut process, &history(0), at(D0 - 1, 12), false).await;
    // The return day's third review reaches the sync after the day has closed: its settle grants
    // the relight, closes the lapse and moves past the day, and the process dies before the route.
    let ended = cycle(&world, &mut process, &history(3), at(D0 + 1, 12), true).await;
    assert_eq!(
        ended,
        Ended::Crashed,
        "the settle's fold commits, then the crash"
    );
    assert_eq!(grants(&world.db).await, vec![D0], "the grant committed");
    let ended = cycle(&world, &mut process, &history(3), at(D0 + 1, 18), false).await;
    assert_eq!(ended, Ended::Routed, "the restarted process's cycle routes");
    assert_eq!(
        world.bot.sends(),
        1,
        "the committed grant is celebrated once after the restart"
    );
}

#[tokio::test]
async fn every_failure_point_later_sync_and_crash_keeps_one_celebration_per_committed_grant() {
    let cases = population();
    println!("examined {} relight order case(s)", cases.len());
    assert!(!cases.is_empty(), "examined 0 cases: nothing was judged");
    let mut broken = Vec::new();
    for case in cases {
        broken.extend(run(case).await);
    }
    assert_eq!(
        broken,
        Vec::<String>::new(),
        "S1, S2 and L1 hold in every case"
    );
}
