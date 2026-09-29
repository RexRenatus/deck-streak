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
    ANSWER_BOUND_SECS, Doorbell, POLL_SECS, Pause, Progress, RING_GAP_SECS, RequestLedger,
    SyncRequester, owner_request_pending,
};
use deck_streak_daemon::sync_request::{
    DEFAULT_REQUEST_PATH, REQUEST_PATH_ENV, SqliteRequestLedger, request_path,
};
use deck_streak_ingest::gate::{Anchor, Probe};
use deck_streak_ingest::state::SqliteIngestState;
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger};
use deck_streak_kernel::{Clock, Db, Environment, KernelError, ManualClock, StudyDay, UtcMillis};

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

#[tokio::test(flavor = "multi_thread")]
async fn a_planted_request_payload_changes_nothing() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let planted = directory.path().join("request");
    std::fs::write(&planted, br#"{"trigger":"owner","force":true}"#).expect("a planted file");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    assert!(
        !owner_request_pending(&db).await.expect("the state reads"),
        "a file and its payload start no owner cycle"
    );
    SqliteIngestState::new(db.clone())
        .request_rescore(UtcMillis::from_epoch_millis(START))
        .await
        .expect("the owner's request is stored");
    assert!(
        owner_request_pending(&db).await.expect("the state reads"),
        "the stored request does"
    );
    db.close().await;
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
