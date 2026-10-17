//! SPEC-470 A1 to A3 (R2, R3; ADR-487 D3 and D5): an instrument run holds the instrument lock
//! until its read ends. A1 drops the run's caller while its read runs, A2 drops the caller and then
//! lets the read end, and A3 drops the run's runtime while its read runs.
//!
//! RED-FIRST: each compiles at the base and fails there by its assertion. Every row is synthetic.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs::{OpenOptions, TryLockError};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use deck_streak_coordination::instruments::{
    BoxFuture, InstrumentRunner, Instruments, LOCK_FILE, OnDemandRefusal,
};
use deck_streak_ingest::lock::CollectionLock;
use deck_streak_insights::instrument::{Cadence, ReportEnvelope};
use deck_streak_insights::registry::{Row, State};
use deck_streak_kernel::{Clock, Db, ManualClock, StudyDayRule, UtcMillis};
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::mpsc;

/// The one live weekly instrument.
static ROWS: [Row; 1] = [Row {
    id: "alpha",
    cadence: Cadence::Weekly,
    state: State::Live,
}];

const DAY_MS: i64 = 86_400_000;
/// The longest a gated read waits for its gate.
const GATE_WAIT: Duration = Duration::from_mins(1);
/// The longest a test waits for a gated read to start.
const STARTED_WAIT: Duration = Duration::from_secs(10);
/// The most polls, a millisecond apart, a test spends waiting for another thread.
const POLLS: u32 = 10_000;

/// A gate the test opens, which a gated read waits on.
struct Gate {
    open: Mutex<bool>,
    opened: Condvar,
}

impl Gate {
    fn new() -> Self {
        Self {
            open: Mutex::new(false),
            opened: Condvar::new(),
        }
    }

    fn open(&self) {
        *self.open.lock().unwrap() = true;
        self.opened.notify_all();
    }

    /// Waits up to `GATE_WAIT` for the gate; true when it opened.
    fn wait(&self) -> bool {
        let open = self.open.lock().unwrap();
        let (open, _) = self
            .opened
            .wait_timeout_while(open, GATE_WAIT, |open| !*open)
            .unwrap();
        *open
    }
}

/// Opens the gate when dropped, so a failed assertion still ends the read and no red hangs.
struct OpenOnDrop(Arc<Gate>);

impl Drop for OpenOnDrop {
    fn drop(&mut self) {
        self.0.open();
    }
}

/// An instrument whose read runs on a blocking thread until the test opens its gate.
struct Gated {
    gate: Arc<Gate>,
    started: mpsc::Sender<()>,
    reading: Arc<AtomicUsize>,
    most: Arc<AtomicUsize>,
}

impl InstrumentRunner for Gated {
    fn id(&self) -> &'static str {
        "alpha"
    }

    fn schema_version(&self) -> u32 {
        1
    }

    fn run(&self, study_day: i64) -> BoxFuture<'_, Result<ReportEnvelope, String>> {
        let gate = Arc::clone(&self.gate);
        let started = self.started.clone();
        let reading = Arc::clone(&self.reading);
        let most = Arc::clone(&self.most);
        Box::pin(async move {
            let opened = tokio::task::spawn_blocking(move || {
                let now = reading.fetch_add(1, Ordering::SeqCst) + 1;
                most.fetch_max(now, Ordering::SeqCst);
                started
                    .blocking_send(())
                    .expect("the test waits for the start");
                let opened = gate.wait();
                reading.fetch_sub(1, Ordering::SeqCst);
                opened
            })
            .await
            .expect("the read ends");
            if opened {
                Ok(ReportEnvelope {
                    instrument: "alpha".to_owned(),
                    study_day,
                    schema_version: 1,
                    failed_reads: Vec::new(),
                    report: json!({ "ok": true }),
                })
            } else {
                Err("the gate never opened".to_owned())
            }
        })
    }
}

struct Rig {
    dir: TempDir,
    instruments: Arc<Instruments>,
    lock: CollectionLock,
    gate: Arc<Gate>,
    started: mpsc::Receiver<()>,
    most: Arc<AtomicUsize>,
}

async fn rig() -> Rig {
    let dir = TempDir::new().expect("dir");
    let db = Db::open(&dir.path().join("ds.db")).await.expect("open");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
        20_000 * DAY_MS + 3_600_000,
    )));
    let gate = Arc::new(Gate::new());
    let (sender, started) = mpsc::channel(8);
    let most = Arc::new(AtomicUsize::new(0));
    let runner: Arc<dyn InstrumentRunner> = Arc::new(Gated {
        gate: Arc::clone(&gate),
        started: sender,
        reading: Arc::new(AtomicUsize::new(0)),
        most: Arc::clone(&most),
    });
    let instruments = Arc::new(
        Instruments::new(db, dir.path(), clock, StudyDayRule::default(), vec![runner])
            .with_rows(&ROWS),
    );
    let lock = CollectionLock::new(dir.path().join(LOCK_FILE));
    Rig {
        dir,
        instruments,
        lock,
        gate,
        started,
        most,
    }
}

#[tokio::test]
async fn a_run_whose_caller_went_away_keeps_the_lock_until_its_read_ends() {
    let mut rig = rig().await;
    let _open = OpenOnDrop(Arc::clone(&rig.gate));
    let mut run = Box::pin(rig.instruments.run_on_demand("alpha"));
    tokio::select! {
        ended = &mut run => panic!("the run ended before its read started: {ended:?}"),
        started = tokio::time::timeout(STARTED_WAIT, rig.started.recv()) => {
            started.expect("the read started").expect("the read started");
        }
    }
    // The caller goes away while the read runs.
    drop(run);
    let kept = rig.lock.try_exclusive().await.expect("lock");
    let second = rig.instruments.run_on_demand("alpha").await;
    let step = rig.instruments.step().await.expect("step");
    assert_eq!(
        (
            kept.is_none(),
            matches!(second, Err(OnDemandRefusal::InProgress)),
            step.deferred,
            rig.most.load(Ordering::SeqCst),
        ),
        (true, true, vec!["alpha".to_owned()], 1),
        "(the lock reads held, a second request is answered in progress, the weekly step defers \
         the instrument, the most reads at once)"
    );
}

#[tokio::test]
async fn a_run_whose_caller_went_away_still_stores_its_report_and_frees_the_lock() {
    let mut rig = rig().await;
    let open = OpenOnDrop(Arc::clone(&rig.gate));
    let mut run = Box::pin(rig.instruments.run_on_demand("alpha"));
    tokio::select! {
        ended = &mut run => panic!("the run ended before its read started: {ended:?}"),
        started = tokio::time::timeout(STARTED_WAIT, rig.started.recv()) => {
            started.expect("the read started").expect("the read started");
        }
    }
    // The caller goes away while the read runs, then the read ends.
    drop(run);
    drop(open);
    let mut stored = None;
    for _ in 0..POLLS {
        stored = rig.instruments.store().get("alpha").await.expect("get");
        if stored.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let mut free = false;
    for _ in 0..POLLS {
        if let Some(held) = rig.lock.try_exclusive().await.expect("lock") {
            held.release().expect("release");
            free = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert_eq!(
        (stored.map(|report| report.instrument), free),
        (Some("alpha".to_owned()), true),
        "(the report the run stored after its caller went, the lock free after it)"
    );
}

#[test]
fn a_run_its_runtime_dropped_keeps_the_lock_until_the_process_ends() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    let (dir, gate) = runtime.block_on(async {
        let mut rig = rig().await;
        let instruments = Arc::clone(&rig.instruments);
        drop(tokio::spawn(async move {
            instruments.run_on_demand("alpha").await
        }));
        tokio::time::timeout(STARTED_WAIT, rig.started.recv())
            .await
            .expect("the read started")
            .expect("the read started");
        (rig.dir, rig.gate)
    });
    let _open = OpenOnDrop(gate);
    // The runtime stops while the read runs, as a stopping process's does.
    runtime.shutdown_timeout(Duration::from_millis(100));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir.path().join(LOCK_FILE))
        .expect("the lock file");
    let answer = file.try_lock();
    assert!(
        matches!(answer, Err(TryLockError::WouldBlock)),
        "the lock read free after the runtime stopped while the read ran: {answer:?}"
    );
}
