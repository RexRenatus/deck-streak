//! The agent's data-rights port (SPEC-043 A13): `agent_runs` is exported whole and erased.
#![allow(clippy::expect_used)]

use deck_streak_agent::data_rights::{AGENT_CONTEXT, AgentDataRights};
use deck_streak_agent::runs::{AGENT_RUNS_TABLE, AgentRuns, RunRecord};
use deck_streak_agent::verdict::{Cause, Delivered, Telemetry, Verdict};
use deck_streak_kernel::{DataRights, Db, Disposition, UtcMillis};
use serde_json::json;

#[tokio::test]
async fn the_agent_runs_are_exported_and_erased() {
    let dir = tempfile::tempdir().expect("a directory");
    let db = Db::open(&dir.path().join("t.db"))
        .await
        .expect("a database");
    let port = AgentDataRights;
    let declaration = port.declaration().expect("a declaration");
    assert_eq!(declaration.context(), AGENT_CONTEXT);
    assert_eq!(declaration.tables().len(), 1);
    assert_eq!(declaration.tables()[0].table, AGENT_RUNS_TABLE);
    assert_eq!(
        declaration.tables()[0].disposition,
        Disposition::ExportAndErase
    );

    let runs = AgentRuns::new(db.clone());
    let verdict = Verdict::Unavailable(Cause::TurnCap);
    let telemetry = Telemetry {
        turns: 30,
        input_tokens: 5,
        output_tokens: 6,
        cost_micro_usd: 7,
        duration_ms: 8,
    };
    let id = runs
        .record(&RunRecord {
            duty: "daily-reading",
            template: "t",
            subject: "s",
            verdict: &verdict,
            telemetry: Some(telemetry),
            at: UtcMillis::from_epoch_millis(42),
        })
        .await
        .expect("a run");
    let mut connection = db.reader().acquire().await.expect("a connection");
    let exported = port.export(&mut connection).await.expect("the export");
    drop(connection);
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].table, AGENT_RUNS_TABLE);
    assert_eq!(
        exported[0].rows,
        [
            json!({"id": id, "duty": "daily-reading", "template": "t", "subject": "s", "verdict": "unavailable", "cause": "turn_cap", "class": null, "turns": 30, "input_tokens": 5, "output_tokens": 6, "cost_micro_usd": 7, "duration_ms": 8, "created_at": 42})
        ]
    );
    let mut write = db.write().await.expect("a write");
    port.erase(&mut write).await.expect("the erase");
    write.commit().await.expect("commits");
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM agent_runs")
        .fetch_one(db.reader())
        .await
        .expect("a count");
    assert_eq!(left, 0, "the erase emptied the table");
    db.close().await;
}

#[tokio::test]
async fn a_run_older_than_the_retention_is_pruned_and_a_newer_one_kept() {
    use deck_streak_agent::runs::RETENTION_DAYS;
    assert_eq!(RETENTION_DAYS, 90, "privacy.json declares 90 days");
    let dir = tempfile::tempdir().expect("a directory");
    let db = Db::open(&dir.path().join("t.db"))
        .await
        .expect("a database");
    let runs = AgentRuns::new(db.clone());
    let verdict = Verdict::AiRouteAbsent;
    for at in [10, 20, 30] {
        runs.record(&RunRecord {
            duty: "d",
            template: "t",
            subject: "s",
            verdict: &verdict,
            telemetry: None,
            at: UtcMillis::from_epoch_millis(at),
        })
        .await
        .expect("a run");
    }
    assert_eq!(
        runs.prune_before(UtcMillis::from_epoch_millis(20))
            .await
            .expect("a prune"),
        1
    );
    let left: Vec<i64> = sqlx::query_scalar("SELECT created_at FROM agent_runs ORDER BY id")
        .fetch_all(db.reader())
        .await
        .expect("rows");
    assert_eq!(left, [20, 30]);
    db.close().await;
}

#[tokio::test]
async fn a_delivered_run_records_the_telemetry_its_verdict_carries() {
    let dir = tempfile::tempdir().expect("a directory");
    let db = Db::open(&dir.path().join("t.db"))
        .await
        .expect("a database");
    let runs = AgentRuns::new(db.clone());
    let verdict = Verdict::Delivered(Delivered {
        output: "text".into(),
        telemetry: Telemetry {
            turns: 3,
            input_tokens: 11,
            output_tokens: 12,
            cost_micro_usd: 13,
            duration_ms: 14,
        },
    });
    let id = runs
        .record(&RunRecord {
            duty: "d",
            template: "t",
            subject: "s",
            verdict: &verdict,
            telemetry: None,
            at: UtcMillis::from_epoch_millis(1),
        })
        .await
        .expect("a run");
    let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT turns, input_tokens, output_tokens, cost_micro_usd, duration_ms \
         FROM agent_runs WHERE id = ?1",
    )
    .bind(id)
    .fetch_one(db.reader())
    .await
    .expect("a row");
    assert_eq!(row, (3, 11, 12, 13, 14));
    db.close().await;
}
