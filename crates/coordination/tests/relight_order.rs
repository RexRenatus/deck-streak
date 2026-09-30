//! The relight's order (SPEC-076 A44, A45, A46; R27, R28): a day is due for its celebration exactly
//! when its relight grant commits, and it stays due across a failed fold and a crash until the
//! router decides it. The properties, over every case below: at most one celebration per relight
//! day (S1); no celebration without a committed grant (S2); and every committed grant is celebrated
//! once the recomputes run (L1), also when a crash falls between the fold's commit and the route.
//! A route that fails for a committed grant leaves its day due, so a later cycle celebrates it once
//! (A51, A52). Every review, card and instant is synthetic and set by hand.

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
/// The second return day of [`two_returns`]: five silent days after the first reopen the lapse.
const SECOND: i64 = D0 + 6;
/// The day of the sync that first reads both returns and commits both grants.
const SYNC: i64 = D0 + 7;

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

/// [`history`] with a return of three reviews on `D0`, five silent days that reopen the lapse, and
/// a second return of three reviews on [`SECOND`]: one fold at [`SYNC`] grants both relights, so
/// two days are due in one route.
fn two_returns() -> CollectionData {
    let mut reviews: Vec<Review> = (D0 - 20..=D0 - 16).map(|d| review(at(d, 9))).collect();
    for day in [D0, SECOND] {
        reviews.extend((0..3).map(|n| review(at(day, 8 + n))));
    }
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
    let today = match fold(world, process, data, now, crash).await {
        Ok(today) => today,
        Err(ended) => return ended,
    };
    route_due_relights(&world.router, &process.due, &world.db, today)
        .await
        .expect("the due relights route");
    Ended::Routed
}

/// [`cycle`] as the daemon's cycle runs it: a route that answers an error is logged and the cycle
/// goes on, as `sync_cycle` does, so a failed route is judged by what later cycles send.
async fn logged_cycle(
    world: &World,
    process: &mut Process,
    data: &CollectionData,
    now: i64,
    crash: bool,
) -> Ended {
    let today = match fold(world, process, data, now, crash).await {
        Ok(today) => today,
        Err(ended) => return ended,
    };
    if let Err(error) = route_due_relights(&world.router, &process.due, &world.db, today).await {
        println!("the due relights could not be read: {error}");
    }
    Ended::Routed
}

/// The fold of one cycle at `now` over `data`, then the crash when `crash`: the study day to route
/// on when the fold committed and the process lives on, else how the cycle ended.
async fn fold(
    world: &World,
    process: &mut Process,
    data: &CollectionData,
    now: i64,
    crash: bool,
) -> Result<StudyDay, Ended> {
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
        return Err(Ended::Failed);
    }
    if crash {
        *process = start();
        return Err(Ended::Crashed);
    }
    Ok(today)
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

/// The route-failure seam (A51, A52), created by the test in its own database: a table of the keys
/// whose route fails, and a trigger on each write of the router's ledger. A key listed under
/// `claim` fails the router's claim of it, so nothing is claimed or sent; a key listed under
/// `record` fails the decision record written after its claim committed and its line was pushed.
/// Either failure answers the route of that day with an error, as any failed write of the router's
/// ledger does, and the production code carries no hook for it.
const ROUTE_FAILURE_SEAM: &str = "
    CREATE TABLE route_failures (
        dedupe_key TEXT NOT NULL,
        ledger TEXT NOT NULL CHECK (ledger IN ('claim', 'record')),
        created_at INTEGER NOT NULL
    ) STRICT;
    CREATE TRIGGER the_claim_fails BEFORE INSERT ON notification_deliveries
    WHEN EXISTS (SELECT 1 FROM route_failures
                 WHERE ledger = 'claim' AND dedupe_key = NEW.dedupe_key)
    BEGIN SELECT RAISE(ABORT, 'the claim of this key fails'); END;
    CREATE TRIGGER the_record_fails BEFORE INSERT ON notification_decisions
    WHEN EXISTS (SELECT 1 FROM route_failures
                 WHERE ledger = 'record' AND dedupe_key = NEW.dedupe_key)
    BEGIN SELECT RAISE(ABORT, 'the decision record of this key fails'); END;
";

/// Which write of the router's ledger fails for a failing day's key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LedgerWrite {
    /// The claim: nothing is claimed and nothing is sent.
    Claim,
    /// The decision record, after the claim committed and the line was pushed.
    Record,
}

impl LedgerWrite {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Record => "record",
        }
    }
}

/// Creates the seam in `db`, with no key failing yet.
async fn seam(db: &Db) {
    let mut write = db.write().await.expect("a write");
    sqlx::raw_sql(ROUTE_FAILURE_SEAM)
        .execute(&mut *write)
        .await
        .expect("the seam is created");
    write.commit().await.expect("the commit");
}

/// Fails `write` for the key of each of `days` until [`disarm`].
async fn arm(db: &Db, days: &[i64], write: LedgerWrite) {
    let mut connection = db.write().await.expect("a write");
    for day in days {
        sqlx::query(
            "INSERT INTO route_failures (dedupe_key, ledger, created_at) VALUES (?1, ?2, ?3)",
        )
        .bind(format!("relight:{day}"))
        .bind(write.as_str())
        .bind(at(D0, 0))
        .execute(&mut *connection)
        .await
        .expect("the failure is armed");
    }
    connection.commit().await.expect("the commit");
}

/// Lets every write of the router's ledger succeed again.
async fn disarm(db: &Db) {
    let mut write = db.write().await.expect("a write");
    sqlx::query("DELETE FROM route_failures")
        .execute(&mut *write)
        .await
        .expect("the failures are disarmed");
    write.commit().await.expect("the commit");
}

/// For the relight of `day`: how many claims its key holds, and how many sends were recorded.
async fn decided(db: &Db, day: i64) -> (i64, i64) {
    let mut write = db.write().await.expect("a write");
    let row = sqlx::query(
        "SELECT (SELECT COUNT(*) FROM notification_deliveries WHERE dedupe_key = ?1), \
                (SELECT COUNT(*) FROM notification_decisions \
                 WHERE dedupe_key = ?1 AND arm = 'send')",
    )
    .bind(format!("relight:{day}"))
    .fetch_one(&mut *write)
    .await
    .expect("the router's ledger reads");
    (row.get(0), row.get(1))
}

/// The keys of every relight the router claimed.
async fn claimed(db: &Db) -> Vec<String> {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "SELECT dedupe_key FROM notification_deliveries WHERE dedupe_key LIKE 'relight:%' \
         ORDER BY id",
    )
    .fetch_all(&mut *write)
    .await
    .expect("the claims read")
    .into_iter()
    .map(|row| row.get(0))
    .collect()
}

/// Which of the two due days fails its route.
#[derive(Clone, Copy, Debug)]
enum FailingDay {
    First,
    Second,
    Both,
}

/// Which cycle's route fails first: the one whose fold commits both grants, or the next one, after
/// a restart between that fold's commit and its route.
#[derive(Clone, Copy, Debug)]
enum FailingCycle {
    Grant,
    AfterRestart,
}

#[derive(Clone, Copy, Debug)]
struct RouteCase {
    cycle: FailingCycle,
    day: FailingDay,
    write: LedgerWrite,
    consecutive: u32,
    crash_before_retry: bool,
}

/// Every failing cycle × failing day × failing ledger write × one or two consecutive failed routes
/// × a restart, or none, between the last failed route and the retry.
fn route_population() -> Vec<RouteCase> {
    let mut cases = Vec::new();
    for cycle in [FailingCycle::Grant, FailingCycle::AfterRestart] {
        for day in [FailingDay::First, FailingDay::Second, FailingDay::Both] {
            for write in [LedgerWrite::Claim, LedgerWrite::Record] {
                for consecutive in [1, 2] {
                    for crash_before_retry in [false, true] {
                        cases.push(RouteCase {
                            cycle,
                            day,
                            write,
                            consecutive,
                            crash_before_retry,
                        });
                    }
                }
            }
        }
    }
    cases
}

/// Runs `case` and answers the properties it breaks: the eve opens the lapse; the sync at [`SYNC`]
/// commits both grants; the case's routes fail; then, with nothing failing, a restart when the case
/// says and two fair cycles route what is still due.
async fn run_route(case: RouteCase) -> Vec<String> {
    let world = world().await;
    let mut process = start();
    let ended = cycle(&world, &mut process, &history(0), at(D0 - 1, 12), false).await;
    assert_eq!(ended, Ended::Routed, "the eve's cycle routes");
    seam(&world.db).await;
    let failing = match case.day {
        FailingDay::First => vec![D0],
        FailingDay::Second => vec![SECOND],
        FailingDay::Both => vec![D0, SECOND],
    };
    arm(&world.db, &failing, case.write).await;
    let data = two_returns();
    let mut hour = 12;
    if matches!(case.cycle, FailingCycle::AfterRestart) {
        let ended = logged_cycle(&world, &mut process, &data, at(SYNC, hour), true).await;
        assert_eq!(
            ended,
            Ended::Crashed,
            "{case:?}: the grants commit, then the restart"
        );
        hour += 1;
    }
    for _ in 0..case.consecutive {
        let ended = logged_cycle(&world, &mut process, &data, at(SYNC, hour), false).await;
        assert_eq!(ended, Ended::Routed, "{case:?}: the failing cycle routes");
        hour += 1;
    }
    assert_eq!(
        grants(&world.db).await,
        vec![D0, SECOND],
        "{case:?}: both grants committed"
    );
    // The route of each failing day failed where the case says: its claim holds nothing, or its
    // claim holds and no send was recorded.
    for &day in &failing {
        let (claims, sends) = decided(&world.db, day).await;
        match case.write {
            LedgerWrite::Claim => assert_eq!(claims, 0, "{case:?}: relight:{day} unclaimed"),
            LedgerWrite::Record => assert_eq!(sends, 0, "{case:?}: relight:{day} unrecorded"),
        }
    }
    disarm(&world.db).await;
    if case.crash_before_retry {
        let ended = logged_cycle(&world, &mut process, &data, at(SYNC, hour), true).await;
        assert_eq!(
            ended,
            Ended::Crashed,
            "{case:?}: the restart before the retry"
        );
        hour += 1;
    }
    for _ in 0..2 {
        logged_cycle(&world, &mut process, &data, at(SYNC, hour), false).await;
        hour += 1;
    }
    judge(&world, &process, case).await
}

/// S1, S2 and L1 over the world's two grants, and the due list emptied by the fair cycles.
async fn judge(world: &World, process: &Process, case: RouteCase) -> Vec<String> {
    let granted: Vec<String> = grants(&world.db)
        .await
        .into_iter()
        .map(|day| format!("relight:{day}"))
        .collect();
    let claimed = claimed(&world.db).await;
    let mut broken = Vec::new();
    for key in &granted {
        match claimed.iter().filter(|claim| *claim == key).count() {
            0 => broken.push(format!("L1 {case:?}: {key} was never celebrated")),
            1 => {}
            n => broken.push(format!("S1 {case:?}: {key} was claimed {n} times")),
        }
    }
    for key in claimed.iter().filter(|claim| !granted.contains(claim)) {
        broken.push(format!(
            "S2 {case:?}: {key} was celebrated with no committed grant"
        ));
    }
    let pushed = world.bot.sends();
    if pushed != granted.len() {
        broken.push(format!(
            "S1/L1 {case:?}: {pushed} lines pushed for {} grants",
            granted.len()
        ));
    }
    let due = process.due.pending(&world.db).await.expect("the due read");
    if !due.is_empty() {
        broken.push(format!(
            "R27 {case:?}: {due:?} still due after two fair cycles"
        ));
    }
    broken
}

/// A51: the router's claim of a committed grant's key fails, so its route answers an error: the day
/// stays due, and once the claim can be written again the next cycle celebrates it, once.
#[tokio::test]
async fn a_route_that_fails_leaves_the_day_due_and_a_later_cycle_celebrates_it_once() {
    let world = world().await;
    let mut process = start();
    cycle(&world, &mut process, &history(0), at(D0 - 1, 12), false).await;
    seam(&world.db).await;
    arm(&world.db, &[D0], LedgerWrite::Claim).await;
    let ended = logged_cycle(&world, &mut process, &history(3), at(D0 + 1, 12), false).await;
    assert_eq!(
        ended,
        Ended::Routed,
        "the settle's fold commits and the route runs"
    );
    assert_eq!(grants(&world.db).await, vec![D0], "the grant committed");
    assert_eq!(
        decided(&world.db, D0).await,
        (0, 0),
        "the route failed at the claim"
    );
    assert_eq!(world.bot.sends(), 0, "nothing was pushed");
    assert_eq!(
        process.due.pending(&world.db).await.expect("the due read"),
        vec![StudyDay::from_epoch_day(D0)],
        "a day whose route fails stays due"
    );
    disarm(&world.db).await;
    let ended = logged_cycle(&world, &mut process, &history(3), at(D0 + 1, 18), false).await;
    assert_eq!(ended, Ended::Routed, "the later cycle routes");
    assert_eq!(
        decided(&world.db, D0).await,
        (1, 1),
        "the later cycle sent it"
    );
    assert_eq!(
        world.bot.sends(),
        1,
        "the committed grant is celebrated once"
    );
    assert_eq!(
        process.due.pending(&world.db).await.expect("the due read"),
        Vec::<StudyDay>::new(),
        "the decided day leaves the list"
    );
}

/// A52: over every failing cycle, failing day, failing ledger write, run of consecutive failed
/// routes and restart before the retry, each committed grant is celebrated once, no other day is,
/// and the due list empties.
#[tokio::test]
async fn every_failed_route_leaves_its_day_due_until_one_celebration() {
    let cases = route_population();
    let claims = cases
        .iter()
        .filter(|case| case.write == LedgerWrite::Claim)
        .count();
    println!(
        "examined {} failed-route case(s), {claims} failing the claim and {} the record",
        cases.len(),
        cases.len() - claims
    );
    assert_eq!(
        cases.len(),
        48,
        "2 cycles x 3 days x 2 writes x 2 runs x 2 restarts"
    );
    let mut broken = Vec::new();
    for case in cases {
        broken.extend(run_route(case).await);
    }
    assert_eq!(
        broken,
        Vec::<String>::new(),
        "S1, S2 and L1 hold, and the due list empties, in every case"
    );
}
