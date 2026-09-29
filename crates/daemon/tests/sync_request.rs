//! SPEC-059: the owner's `/sync` is a request for the sync job, never a cycle in the bot.

#![allow(clippy::expect_used)]

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use deck_streak_bot::{OwnerSync, Scores, SyncAnswer, SyncOutcome, SyncRefusal};
use deck_streak_daemon::sync_request::{
    ANSWER_BOUND_SECS, Doorbell, Flush, POLL_SECS, Pause, Progress, RING_GAP_SECS, RequestLedger,
    SyncRequester,
};
use deck_streak_daemon::sync_request::{
    DEFAULT_REQUEST_PATH, FileDoorbell, REQUEST_PATH_ENV, SqliteRequestLedger, request_path,
};
use deck_streak_ingest::gate::{Anchor, Probe};
use deck_streak_ingest::state::SqliteIngestState;
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger};
use deck_streak_kernel::{
    Clock, Db, Environment, KernelError, ManualClock, StudyDay, StudyDayRule, UtcMillis,
};
use deck_streak_notifications::{
    BotTransport, Decision, DedupeKey, Hold, LapseContext, Occasion, Pass, Policy, PushFuture,
    Pushed, Router, Surface, Tier,
};

const START: i64 = 1_800_000_000_000;

#[derive(Clone)]
struct Shared(Arc<ManualClock>);

impl Clock for Shared {
    fn now(&self) -> UtcMillis {
        self.0.now()
    }
}

#[derive(Clone)]
struct Wait(Arc<ManualClock>);

impl Pause for Wait {
    fn pause(&self, by: Duration) -> impl Future<Output = ()> + Send {
        self.0.advance(by);
        async {}
    }
}

#[derive(Clone, Default)]
struct Script {
    steps: Arc<Mutex<VecDeque<Progress>>>,
    polls: Arc<Mutex<usize>>,
}

impl Script {
    fn of(steps: impl IntoIterator<Item = Progress>) -> Self {
        Self {
            steps: Arc::new(Mutex::new(steps.into_iter().collect())),
            polls: Arc::default(),
        }
    }
    fn polls(&self) -> usize {
        *self.polls.lock().expect("the lock is not poisoned")
    }
}

impl RequestLedger for Script {
    async fn request(&self, _at: UtcMillis) -> Result<(), KernelError> {
        Ok(())
    }
    async fn progress(&self, _since: UtcMillis) -> Result<Progress, KernelError> {
        let mut polls = self.polls.lock().expect("the poll count locks");
        *polls += 1;
        assert!(*polls <= 200, "the wait never ends");
        drop(polls);
        let next = self
            .steps
            .lock()
            .expect("the script locks")
            .pop_front()
            .unwrap_or(Progress::Waiting);
        Ok(next)
    }
}

#[derive(Clone)]
struct Bell {
    clock: Arc<ManualClock>,
    rings: Arc<Mutex<Vec<i64>>>,
    broken: bool,
}

impl Bell {
    fn new(clock: &Arc<ManualClock>, broken: bool) -> Self {
        Self {
            clock: clock.clone(),
            rings: Arc::default(),
            broken,
        }
    }
    fn rings(&self) -> Vec<i64> {
        self.rings.lock().expect("the lock is not poisoned").clone()
    }
}

impl Doorbell for Bell {
    fn ring(&self) -> io::Result<()> {
        if self.broken {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        self.rings
            .lock()
            .expect("the ring is recorded")
            .push(self.clock.now().epoch_millis());
        Ok(())
    }
}

type Requester = SyncRequester<Shared, Script, Bell, Wait>;

fn requester(script: &Script, broken: bool) -> (Requester, Bell, Arc<ManualClock>) {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START)));
    let bell = Bell::new(&clock, broken);
    let port = SyncRequester::new(
        Shared(clock.clone()),
        script.clone(),
        bell.clone(),
        Wait(clock.clone()),
    );
    (port, bell, clock)
}

fn elapsed_secs(clock: &ManualClock) -> i64 {
    (clock.now().epoch_millis() - START) / 1000
}

fn crate_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Whether `source` names the cycle or its engine, which the bot's wiring must not.
fn names_the_cycle(source: &str) -> bool {
    ["OwnerSyncCycle", "sync_cycle", "RslibEngine"]
        .iter()
        .any(|name| source.contains(name))
}

#[test]
fn the_bot_port_requests_the_job_and_never_runs_the_cycle() {
    assert!(
        names_the_cycle("let sync = OwnerSyncCycle::new(env);"),
        "a planted call to the cycle is seen"
    );
    assert!(!names_the_cycle("let sync = SyncRequester::new(clock);"));
    let bot = std::fs::read_to_string(crate_file("src/role_bot.rs")).expect("the bot role reads");
    assert!(
        !names_the_cycle(&bot),
        "the bot role runs no cycle in its own process"
    );
    assert!(
        bot.contains("SyncRequester"),
        "the bot's port requests the job"
    );
    let wiring = std::fs::read_to_string(crate_file("src/wiring.rs")).expect("wiring reads");
    assert!(
        !wiring.contains("impl OwnerSync for"),
        "the cycle is not a port the bot can be given"
    );
}

#[tokio::test]
async fn a_second_request_waits_out_the_ring_gap() {
    let script = Script::of([
        Progress::Ran { failure: None },
        Progress::Ran { failure: None },
    ]);
    let (port, bell, _clock) = requester(&script, false);
    let first = port.sync_now().await;
    let second = port.sync_now().await;
    assert!(first.is_ok() && second.is_ok(), "{first:?} {second:?}");
    let rings = bell.rings();
    assert_eq!(rings.len(), 2, "each request rings once");
    assert!(
        rings[1] - rings[0] == RING_GAP_SECS * 1000,
        "two rings are exactly {RING_GAP_SECS} s apart when the first answer was immediate: {rings:?}"
    );
}

#[tokio::test]
async fn a_request_inside_the_reuse_window_is_answered_reused() {
    let script = Script::of([Progress::Waiting, Progress::Reused]);
    let (port, bell, _clock) = requester(&script, false);
    let answer = port.sync_now().await;
    assert_eq!(
        answer,
        Ok(SyncAnswer {
            sync: SyncOutcome::Reused,
            scores: Scores::Recomputed,
        })
    );
    assert_eq!(bell.rings().len(), 1);
}

#[tokio::test]
async fn the_owner_is_answered_within_the_bound() {
    // The outcome, when the job finishes.
    let script = Script::of([
        Progress::Waiting,
        Progress::Waiting,
        Progress::Ran { failure: None },
    ]);
    let (port, _bell, clock) = requester(&script, false);
    assert_eq!(
        port.sync_now().await,
        Ok(SyncAnswer {
            sync: SyncOutcome::Synced,
            scores: Scores::Recomputed,
        })
    );
    assert_eq!(elapsed_secs(&clock), 2 * POLL_SECS);

    // A failed sync is told as such.
    let script = Script::of([Progress::Ran {
        failure: Some("server_error".to_owned()),
    }]);
    let (port, _bell, _clock) = requester(&script, false);
    assert_eq!(
        port.sync_now().await,
        Ok(SyncAnswer {
            sync: SyncOutcome::Failed {
                reason: "server_error".to_owned()
            },
            scores: Scores::Recomputed,
        })
    );

    // A sync that never finishes is answered at the bound, not left silent.
    let script = Script::of([]);
    let (port, _bell, clock) = requester(&script, false);
    assert_eq!(
        port.sync_now().await,
        Ok(SyncAnswer {
            sync: SyncOutcome::StillRunning,
            scores: Scores::Unchanged,
        })
    );
    let waited = elapsed_secs(&clock);
    assert!(
        (ANSWER_BOUND_SECS..ANSWER_BOUND_SECS + POLL_SECS).contains(&waited),
        "waited {waited} s"
    );
    assert!(script.polls() <= usize::try_from(ANSWER_BOUND_SECS / POLL_SECS + 1).unwrap());

    // A request that cannot be rung is refused, with a reason.
    let (port, _bell, _clock) = requester(&Script::of([]), true);
    assert_eq!(
        port.sync_now().await,
        Err(SyncRefusal {
            reason: "sync_request_unwritten"
        })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_store_tells_waiting_reused_synced_and_failed_apart() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let ledger = SqliteRequestLedger::new(db.clone());
    let at = UtcMillis::from_epoch_millis(START);
    assert_eq!(ledger.progress(at).await.expect("reads"), Progress::Reused);
    ledger.request(at).await.expect("the request is stored");
    assert_eq!(ledger.progress(at).await.expect("reads"), Progress::Waiting);
    let anchor = Anchor {
        probe: Probe {
            newest_review_id: 1,
            card_count: 1,
            card_fingerprint: 1,
        },
        study_day: StudyDay::from_epoch_day(20_000),
        recomputed_at: at,
        settings_generation: 1,
    };
    SqliteIngestState::new(db.clone())
        .write_anchor(&anchor, at)
        .await
        .expect("the recompute clears the flag");
    let runs = SqliteSyncRuns::new(db.clone());
    let record = |outcome| SyncRun {
        trigger: Trigger::Owner,
        started_at: UtcMillis::from_epoch_millis(START + 1_000),
        finished_at: UtcMillis::from_epoch_millis(START + 2_000),
        study_day: StudyDay::from_epoch_day(20_000),
        outcome,
        attempts: 1,
        full_download: false,
    };
    runs.record(&record(Ok(()))).await.expect("recorded");
    assert_eq!(
        ledger.progress(at).await.expect("reads"),
        Progress::Ran { failure: None }
    );
    runs.record(&record(Err(ReasonCode::ServerError)))
        .await
        .expect("recorded");
    let Progress::Ran { failure } = ledger.progress(at).await.expect("reads") else {
        panic!("the owner's run is read");
    };
    assert!(failure.is_some(), "a failed run carries its reason");
    db.close().await;
}

#[test]
fn the_request_file_is_the_setting_or_the_default() {
    let unset = Environment::from_vars(Vec::<(String, String)>::new());
    assert_eq!(
        request_path(&unset).expect("the default"),
        PathBuf::from(DEFAULT_REQUEST_PATH)
    );
    let set = Environment::from_vars([(REQUEST_PATH_ENV, "/run/elsewhere/request")]);
    assert_eq!(
        request_path(&set).expect("the setting"),
        PathBuf::from("/run/elsewhere/request")
    );
    let relative = Environment::from_vars([(REQUEST_PATH_ENV, "request")]);
    assert!(
        request_path(&relative).is_err(),
        "a relative path is refused"
    );
}

#[test]
fn the_doorbell_writes_its_file_and_refuses_an_absent_directory() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let file = directory.path().join("request");
    FileDoorbell::new(file.clone())
        .ring()
        .expect("the ring is written");
    assert!(file.is_file(), "the ring leaves the request file");
    let absent = directory.path().join("absent").join("request");
    assert!(
        FileDoorbell::new(absent).ring().is_err(),
        "an absent directory is refused"
    );
}

/// A flush that counts its calls, and fails when told to.
#[derive(Clone, Default)]
struct Flushes {
    calls: Arc<Mutex<usize>>,
    broken: bool,
}

impl Flushes {
    fn calls(&self) -> usize {
        *self.calls.lock().expect("the count locks")
    }
}

impl Flush for Flushes {
    async fn flush(&self) -> Result<(), KernelError> {
        *self.calls.lock().expect("the count locks") += 1;
        if self.broken {
            return Err(KernelError::Offload { operation: "flush" });
        }
        Ok(())
    }
}

fn flushing(script: &Script, flushes: &Flushes) -> impl OwnerSync {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START)));
    SyncRequester::new(
        Shared(clock.clone()),
        script.clone(),
        Bell::new(&clock, false),
        Wait(clock),
    )
    .with_flush(flushes.clone())
}

#[tokio::test]
async fn the_bot_flushes_its_router_after_the_owners_sync_succeeds() {
    let flushes = Flushes::default();
    let script = Script::of([Progress::Waiting, Progress::Ran { failure: None }]);
    let answer = flushing(&script, &flushes).sync_now().await;
    assert_eq!(
        answer,
        Ok(SyncAnswer {
            sync: SyncOutcome::Synced,
            scores: Scores::Recomputed,
        })
    );
    assert_eq!(
        flushes.calls(),
        1,
        "one flush, after the sync that ran and succeeded"
    );
}

#[tokio::test]
async fn the_bot_never_flushes_after_a_failed_reused_or_unanswered_sync() {
    let flushes = Flushes::default();
    let failed = Script::of([Progress::Ran {
        failure: Some("server_error".to_owned()),
    }]);
    assert!(flushing(&failed, &flushes).sync_now().await.is_ok());
    let reused = Script::of([Progress::Reused]);
    assert!(flushing(&reused, &flushes).sync_now().await.is_ok());
    let unanswered = Script::of([]);
    let answer = flushing(&unanswered, &flushes).sync_now().await;
    assert_eq!(
        answer.map(|answer| answer.sync),
        Ok(SyncOutcome::StillRunning)
    );
    assert_eq!(flushes.calls(), 0, "no flush without a sync that succeeded");
}

#[tokio::test]
async fn a_flush_that_fails_never_changes_the_owners_answer() {
    let flushes = Flushes {
        broken: true,
        ..Flushes::default()
    };
    let script = Script::of([Progress::Ran { failure: None }]);
    let answer = flushing(&script, &flushes).sync_now().await;
    assert_eq!(
        answer,
        Ok(SyncAnswer {
            sync: SyncOutcome::Synced,
            scores: Scores::Recomputed,
        })
    );
    assert_eq!(flushes.calls(), 1, "the flush was tried");
}

/// A bot transport that records every push and delivers it.
#[derive(Default)]
struct Recording(Mutex<Vec<String>>);

impl BotTransport for Recording {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            self.0
                .lock()
                .expect("the lock is not poisoned")
                .push(text.to_owned());
            Pushed::Delivered
        })
    }
}

#[tokio::test]
async fn the_real_router_flush_delivers_a_celebration_quiet_hours_held() {
    const DAY: i64 = 20_000;
    const DAY_MS: i64 = 86_400_000;
    const MINUTE_MS: i64 = 60_000;
    let scratch = tempfile::tempdir().expect("a scratch");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
        DAY * DAY_MS + (23 * 60 + 30) * MINUTE_MS,
    )));
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let bot = Arc::new(Recording::default());
    let router = Router::new(
        Arc::clone(&policy),
        db,
        clock.clone(),
        StudyDayRule::default(),
    )
    .with_bot(bot.clone());
    let celebration = Occasion::new(
        policy.kind("celebration").expect("the celebration kind"),
        DedupeKey::new("level-up:9").expect("a key"),
        Surface::Bot,
        Tier::T2,
        "synthetic level-up",
        StudyDay::from_epoch_day(DAY),
        LapseContext::NoLapse,
    )
    .expect("an occasion");
    assert_eq!(
        router.route(&celebration).await.expect("a decision"),
        Decision::Deferred {
            surface: Surface::Bot,
            hold: Hold::Quiet
        },
        "23:30 is inside the quiet window"
    );
    clock.set(UtcMillis::from_epoch_millis(
        (DAY + 1) * DAY_MS + 8 * 60 * MINUTE_MS,
    ));

    Flush::flush(&Arc::new(router))
        .await
        .expect("the flush runs");

    assert_eq!(
        bot.0.lock().expect("the lock is not poisoned").clone(),
        ["synthetic level-up"]
    );
}
