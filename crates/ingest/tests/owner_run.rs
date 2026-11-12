//! SPEC-059: the owner's latest sync run since an instant, which the bot's wait reads.

use deck_streak_ingest::sync_runs::{
    OwnerRun, ReasonCode, RunStatus, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger,
};
use deck_streak_kernel::{Db, StudyDay, UtcMillis};

fn run(trigger: Trigger, at: i64, outcome: Result<(), ReasonCode>) -> SyncRun {
    SyncRun {
        trigger,
        started_at: UtcMillis::from_epoch_millis(at),
        finished_at: UtcMillis::from_epoch_millis(at + 10),
        study_day: StudyDay::from_epoch_day(20_000),
        outcome,
        attempts: 1,
        full_download: false,
    }
}

fn since(at: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(at)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_owners_latest_run_since_an_instant_is_read() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let runs = SqliteSyncRuns::new(db.clone());
    assert_eq!(runs.owner_run_since(since(0)).await.unwrap(), None);

    runs.record(&run(Trigger::Owner, 1_000, Ok(())))
        .await
        .unwrap();
    runs.record(&run(
        Trigger::Scheduled,
        2_000,
        Err(ReasonCode::ServerError),
    ))
    .await
    .unwrap();
    assert_eq!(
        runs.owner_run_since(since(500)).await.unwrap(),
        Some(OwnerRun {
            status: RunStatus::Ok,
            reason: None
        }),
        "a scheduled run is not the owner's"
    );
    assert_eq!(
        runs.owner_run_since(since(1_001)).await.unwrap(),
        None,
        "a run that started before the request is not its answer"
    );

    runs.record(&run(Trigger::Owner, 3_000, Err(ReasonCode::ServerError)))
        .await
        .unwrap();
    assert_eq!(
        runs.owner_run_since(since(500)).await.unwrap(),
        Some(OwnerRun {
            status: RunStatus::Error,
            reason: Some("server_error".to_owned())
        }),
        "the latest owner run answers"
    );
    db.close().await;
}
