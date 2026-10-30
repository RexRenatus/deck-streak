//! The offload runs at most its bound of blocking tasks at once, and a slow call logs one warning
//! with its operation and duration and nothing of its arguments (SPEC-020 A16, R14).
//!
//! No test sleeps: the blocking tasks wait on channels the test releases one at a time, and the
//! slow call advances a manual clock from inside its own work.

// An integration test is test code: its helpers panic on a poisoned lock.
#![allow(clippy::expect_used)]

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use deck_streak_kernel::offload::SLOW_OFFLOAD_MS;
use deck_streak_kernel::{
    Clock, ManualClock, Offload, OffloadWorkers, Redactor, UtcMillis, logging,
};
use tracing_subscriber::EnvFilter;

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

/// How many blocking tasks may run at once in the bound test, and how many are offered.
const BOUND: usize = 2;
const TASKS: usize = 5;

/// A writer the log lines of one test collect in.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl io::Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("the capture lock")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Captured {
    fn lines(&self) -> Vec<String> {
        let bytes = self.0.lock().expect("the capture lock").clone();
        String::from_utf8(bytes)
            .expect("UTF-8 lines")
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

#[tokio::test]
async fn the_offload_runs_at_most_its_bound_of_blocking_tasks_at_once() {
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(0)));
    let offload = Offload::new(OffloadWorkers::new(BOUND).expect("a bound"), clock);
    let running = Arc::new(AtomicUsize::new(0));
    let most = Arc::new(AtomicUsize::new(0));
    let (started, mut starts) = tokio::sync::mpsc::channel::<usize>(TASKS);
    let (release, released) = mpsc::sync_channel::<()>(TASKS);
    let released = Arc::new(Mutex::new(released));

    let mut calls = Vec::new();
    for task in 0..TASKS {
        let (offload, running, most) = (offload.clone(), running.clone(), most.clone());
        let (started, released) = (started.clone(), released.clone());
        calls.push(tokio::spawn(async move {
            offload
                .run("bound-probe", move || {
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    most.fetch_max(now, Ordering::SeqCst);
                    started.blocking_send(task).expect("the test listens");
                    released
                        .lock()
                        .expect("the release lock")
                        .recv()
                        .expect("the test releases every task");
                    running.fetch_sub(1, Ordering::SeqCst);
                    task
                })
                .await
        }));
    }

    // The first tasks to start hold every worker, and none is released yet.
    for _ in 0..BOUND {
        starts.recv().await.expect("a task starts");
    }
    assert_eq!(running.load(Ordering::SeqCst), BOUND);
    // Each release lets exactly one waiting task start in its place.
    for _ in BOUND..TASKS {
        release.send(()).expect("a task waits");
        starts.recv().await.expect("a waiting task starts");
        assert!(running.load(Ordering::SeqCst) <= BOUND);
    }
    for _ in 0..BOUND {
        release.send(()).expect("a task waits");
    }
    let mut finished = Vec::new();
    for call in calls {
        finished.push(
            call.await
                .expect("the call's task")
                .expect("the offloaded work"),
        );
    }
    finished.sort_unstable();
    assert_eq!(finished, (0..TASKS).collect::<Vec<_>>());
    assert_eq!(most.load(Ordering::SeqCst), BOUND);
}

#[tokio::test]
async fn a_slow_offload_logs_one_warning_with_its_operation_and_duration() {
    let captured = Captured::default();
    let writer = captured.clone();
    let _logs = log_capture::hold_capture(logging::subscriber(
        Redactor::new(),
        move || writer.clone(),
        EnvFilter::new("warn"),
    ));
    let manual = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(0)));
    let clock: Arc<dyn Clock> = manual.clone();
    let offload = Offload::new(OffloadWorkers::new(1).expect("a bound"), clock);
    let slow_by = Duration::from_millis(u64::try_from(SLOW_OFFLOAD_MS).expect("a positive bound"));

    let fast = offload
        .run("fast-read", || "an argument never logged")
        .await;
    assert_eq!(fast.expect("the fast work"), "an argument never logged");
    let slow = {
        let manual = manual.clone();
        offload.run("slow-read", move || {
            manual.advance(slow_by);
            "an argument never logged"
        })
    }
    .await;
    assert_eq!(slow.expect("the slow work"), "an argument never logged");

    let lines = captured.lines();
    assert_eq!(
        lines.len(),
        1,
        "one warning, for the slow call alone: {lines:#?}"
    );
    assert!(lines[0].starts_with("<4>"), "{}", lines[0]);
    let event: serde_json::Value = serde_json::from_str(&lines[0][3..]).expect("a JSON event");
    assert_eq!(event["level"], "WARN");
    assert_eq!(event["operation"], "slow-read");
    assert_eq!(event["duration_ms"], SLOW_OFFLOAD_MS);
    assert!(
        !lines[0].contains("an argument never logged"),
        "{}",
        lines[0]
    );
}

#[test]
fn the_offload_shows_its_worker_bound_in_debug() {
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(0)));
    let offload = Offload::new(OffloadWorkers::new(3).expect("a bound"), clock);
    let shown = format!("{offload:?}");
    assert!(shown.starts_with("Offload { workers: "), "{shown}");
    assert!(shown.contains('3'), "{shown}");
}
