//! The collection lock serialises syncs (SPEC-022 A12, R7).
//!
//! SPEC-470 A4 to A6: not red first; they call the added `Held::hold_to_end`; rows S47001 to
//! S47004 are their reds.

mod support;

use std::fs::{OpenOptions, TryLockError};
use std::sync::Arc;
use std::time::Duration;

use deck_streak_ingest::engine::SyncOutcome;
use deck_streak_ingest::lock::CollectionLock;
use deck_streak_ingest::sync::SyncReport;
use deck_streak_ingest::sync_runs::Trigger;
use support::{Fixture, MemoryRuns, ScriptedEngine, Step};
use tokio::sync::{Notify, mpsc};

/// Polls `ready` until it holds, and says whether it did; the polling bounds a wait for another
/// thread, and no assertion reads the time it took.
async fn until(mut ready: impl FnMut() -> bool) -> bool {
    for _ in 0..10_000 {
        if ready() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    false
}

#[test]
fn a_second_sync_waits_for_the_collection_lock_and_never_overlaps() {
    // Two syncers over one state directory: two processes contending for one copy.
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let release = Arc::new(Notify::new());
    let engine = ScriptedEngine::new([
        Step::Hold(Arc::clone(&release), SyncOutcome::Synced),
        Step::Answer(SyncOutcome::NoChanges),
    ]);
    let first =
        Arc::new(fixture.syncer(engine.clone(), MemoryRuns::default(), support::clock_at(0)));
    let second =
        Arc::new(fixture.syncer(engine.clone(), MemoryRuns::default(), support::clock_at(0)));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime");
    let (earlier, later) = runtime.block_on(async {
        let earlier = tokio::spawn({
            let first = Arc::clone(&first);
            async move { first.sync(Trigger::Owner).await }
        });
        // The first holds the lock, and is inside the engine until it is let go.
        let holding = until(|| engine.normal_syncs() == 1).await;
        assert!(
            holding,
            "the first sync never held the collection lock inside the engine"
        );
        let later = tokio::spawn({
            let second = Arc::clone(&second);
            async move { second.sync(Trigger::Owner).await }
        });
        // The second gets every chance to overlap before the first is let go.
        for _ in 0..50 {
            if engine.normal_syncs() > 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        let entered = engine.normal_syncs();
        release.notify_one();
        let earlier = earlier.await.expect("the first sync ends");
        let later = later.await.expect("the second sync ends");
        assert_eq!(
            entered, 1,
            "the second sync entered the engine while the first held the lock"
        );
        (earlier, later)
    });
    for report in [earlier, later] {
        assert!(
            matches!(report, Ok(SyncReport::Ran { ref run, .. }) if run.outcome.is_ok()),
            "{report:?}"
        );
    }
    assert_eq!(
        engine.normal_syncs(),
        2,
        "the second sync ran once the first let go"
    );
    assert_eq!(
        engine.most_active(),
        1,
        "no two syncs were ever inside the engine at once"
    );
}

#[tokio::test]
async fn a_try_take_answers_the_lock_when_free_and_nothing_while_held() {
    // SPEC-094 R8: the non-waiting take is `Some` on a free lock, `None` while another holder has
    // it, and `Some` again once that holder releases.
    let directory = tempfile::tempdir().expect("a directory");
    let lock = CollectionLock::new(directory.path().join("try.lock"));
    let first = lock
        .try_exclusive()
        .await
        .expect("the take")
        .expect("a free lock is taken");
    let second = lock.try_exclusive().await.expect("the take");
    assert!(second.is_none(), "a held lock is not taken: {second:?}");
    first.release().expect("release");
    let third = lock.try_exclusive().await.expect("the take");
    assert!(third.is_some(), "a released lock is taken again");
}

#[tokio::test]
async fn a_held_lock_handed_to_a_task_runs_to_its_end_when_its_caller_stops_waiting() {
    let dir = tempfile::tempdir().expect("a directory");
    let lock = CollectionLock::new(dir.path().join("instruments.lock"));
    let held = lock
        .try_exclusive()
        .await
        .expect("the take")
        .expect("a free lock is taken");
    let go = Arc::new(Notify::new());
    let (sent, mut ends) = mpsc::channel(1);
    let work = {
        let go = Arc::clone(&go);
        async move {
            go.notified().await;
            sent.send(7_u32).await.expect("the test reads the end");
            7_u32
        }
    };
    let mut caller = Box::pin(held.hold_to_end(work));
    let waited = tokio::time::timeout(Duration::from_millis(50), &mut caller).await;
    // The caller stops waiting while the work runs.
    drop(caller);
    let kept = lock.try_exclusive().await.expect("lock").is_none();
    go.notify_one();
    let ended = tokio::time::timeout(Duration::from_secs(10), ends.recv())
        .await
        .ok()
        .flatten();
    let mut free = false;
    for _ in 0..10_000 {
        if let Some(again) = lock.try_exclusive().await.expect("lock") {
            again.release().expect("release");
            free = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert_eq!(
        (waited.is_err(), kept, ended, free),
        (true, true, Some(7), true),
        "(the caller stopped waiting, the lock kept while the work ran, the work's end, the lock \
         free after it)"
    );
}

#[tokio::test]
async fn a_task_that_panics_releases_the_lock_and_its_caller_sees_the_panic() {
    let dir = tempfile::tempdir().expect("a directory");
    let lock = CollectionLock::new(dir.path().join("instruments.lock"));
    let held = lock
        .try_exclusive()
        .await
        .expect("the take")
        .expect("a free lock is taken");
    let work = async {
        assert!(!std::hint::black_box(true), "the work panicked");
    };
    let answer = tokio::spawn(held.hold_to_end(work)).await;
    let free = lock.try_exclusive().await.expect("lock").is_some();
    assert_eq!(
        (answer.is_err_and(|error| error.is_panic()), free),
        (true, true),
        "(the caller saw the work's panic, the lock free after it)"
    );
}

#[test]
fn a_task_its_runtime_drops_keeps_the_lock_until_the_process_ends() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = dir.path().join("instruments.lock");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let lock = CollectionLock::new(path.clone());
        let held = lock
            .try_exclusive()
            .await
            .expect("the take")
            .expect("a free lock is taken");
        let (sent, mut started) = mpsc::channel(1);
        let work = async move {
            sent.send(()).await.expect("the test reads the start");
            std::future::pending::<()>().await;
        };
        drop(tokio::spawn(held.hold_to_end(work)));
        tokio::time::timeout(Duration::from_secs(10), started.recv())
            .await
            .expect("the work started")
            .expect("the work started");
    });
    // The runtime stops while the work runs, as a stopping process's does.
    runtime.shutdown_timeout(Duration::from_millis(100));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("the lock file");
    let answer = file.try_lock();
    assert!(
        matches!(answer, Err(TryLockError::WouldBlock)),
        "the lock read free after the runtime dropped the task: {answer:?}"
    );
}
