//! Ingest's data-rights port (SPEC-022 A13, R12): `sync_runs` is exported and erased.

mod support;

use deck_streak_ingest::data_rights::{IngestDataRights, SYNC_RUNS_TABLE};
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger};
use deck_streak_kernel::{DataRights, Disposition, StudyDay, UtcMillis};

#[tokio::test]
async fn the_ingest_port_declares_sync_runs_exported_and_erased() {
    let declaration = IngestDataRights
        .declaration()
        .expect("ingest's declaration is well formed");
    assert_eq!(declaration.context(), "ingest");
    assert_eq!(
        declaration.disposition(SYNC_RUNS_TABLE),
        Some(&Disposition::ExportAndErase),
        "sync_runs is the owner's data: exported and erased (CHARTER 13)"
    );

    // What the declaration says, the port does: a recorded run is exported, then erased.
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    let runs = SqliteSyncRuns::new(db.clone());
    runs.record(&SyncRun {
        trigger: Trigger::Scheduled,
        started_at: UtcMillis::from_epoch_millis(1_000),
        finished_at: UtcMillis::from_epoch_millis(2_000),
        study_day: StudyDay::from_epoch_day(0),
        outcome: Err(ReasonCode::ServerError),
        attempts: 3,
        full_download: false,
    })
    .await
    .expect("a run is recorded");
    let mut write = db.write().await.expect("a write");
    let exported = IngestDataRights
        .export(&mut write)
        .await
        .expect("the port exports");
    let table = exported
        .iter()
        .find(|table| table.table == SYNC_RUNS_TABLE)
        .expect("sync_runs is exported");
    assert_eq!(table.rows.len(), 1, "{:?}", table.rows);
    assert_eq!(table.rows[0]["reason"], "server_error");
    IngestDataRights
        .erase(&mut write)
        .await
        .expect("the port erases");
    write.commit().await.expect("the erase commits");
    assert_eq!(
        runs.consecutive_failures().await.expect("a read"),
        0,
        "the erased record holds no run"
    );
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM sync_runs")
        .fetch_one(db.reader())
        .await
        .expect("a count");
    assert_eq!(remaining, 0);
}
