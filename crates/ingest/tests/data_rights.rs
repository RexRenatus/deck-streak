//! Ingest's data-rights port (SPEC-022 A13, R12): `sync_runs` is exported and erased; and
//! (SPEC-023 A20, R13) `ingest_state` is exported and reset in place; and (SPEC-387 A12, R9)
//! `preset_proposals` is exported and erased.

mod support;

use deck_streak_ingest::data_rights::{INGEST_STATE_TABLE, IngestDataRights, SYNC_RUNS_TABLE};
use deck_streak_ingest::gate::{Anchor, AnchorState, Probe};
use deck_streak_ingest::state::{IngestState, RefusalReason, SqliteIngestState};
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger};
use deck_streak_ingest::window::WindowBase;
use deck_streak_kernel::{DataRights, Disposition, StudyDay, UtcMillis};
use serde_json::{Value, json};

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

#[tokio::test]
async fn the_ingest_port_resets_its_state_in_place() {
    let declaration = IngestDataRights
        .declaration()
        .expect("ingest's declaration is well formed");
    let disposition = declaration.disposition(INGEST_STATE_TABLE);
    assert!(
        matches!(disposition, Some(Disposition::ResetInPlace { .. })),
        "ingest_state is a singleton: reset in place (CHARTER 13), not {disposition:?}"
    );
    let Some(Disposition::ResetInPlace { row: reset }) = disposition.cloned() else {
        return;
    };

    // An anchor, a pending rescore and a window base are written...
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    let state = SqliteIngestState::new(db.clone());
    let now = UtcMillis::from_epoch_millis(1_000_000);
    let anchor = Anchor {
        probe: Probe {
            newest_review_id: 42,
            card_count: 7,
            card_fingerprint: 1027,
        },
        study_day: StudyDay::from_epoch_day(20_000),
        recomputed_at: now,
        settings_generation: 2,
    };
    state
        .write_anchor(&anchor, now)
        .await
        .expect("the anchor is written");
    state
        .request_rescore(now)
        .await
        .expect("a rescore is requested");
    let base = WindowBase {
        floor: 500,
        count: 9,
    };
    state
        .write_window_base(base, now)
        .await
        .expect("the base is written");

    // ...exported as the one row...
    let mut write = db.write().await.expect("a write");
    let exported = IngestDataRights
        .export(&mut write)
        .await
        .expect("the port exports");
    let table = exported
        .iter()
        .find(|table| table.table == INGEST_STATE_TABLE)
        .expect("ingest_state is exported");
    assert_eq!(table.rows.len(), 1, "{:?}", table.rows);
    let exported_row = &table.rows[0];
    assert_eq!(
        (
            &exported_row["anchor_newest_review_id"],
            &exported_row["anchor_card_count"],
            &exported_row["rescore_pending"],
            &exported_row["window_count"],
        ),
        (&json!(42), &json!(7), &json!(1), &json!(9))
    );

    // ...and an erase keeps the row, holding exactly the declared reset values.
    IngestDataRights
        .erase(&mut write)
        .await
        .expect("the port erases");
    write.commit().await.expect("the erase commits");
    let rows: Vec<(i64, i64)> = sqlx::query_as("SELECT id, created_at FROM ingest_state")
        .fetch_all(db.reader())
        .await
        .expect("the row is read");
    assert_eq!(rows.len(), 1, "reset in place, never deleted: {rows:?}");
    let mut after = db.write().await.expect("a write");
    let erased = IngestDataRights
        .export(&mut after)
        .await
        .expect("the port exports");
    drop(after);
    let erased_row = &erased
        .iter()
        .find(|table| table.table == INGEST_STATE_TABLE)
        .expect("ingest_state is exported")
        .rows[0];
    for (column, value) in &reset {
        assert_eq!(&erased_row[column.as_str()], value, "{column}");
    }
    assert_eq!(erased_row["created_at"], exported_row["created_at"]);
    assert_eq!(
        erased_row["anchor_newest_review_id"],
        Value::Null,
        "the anchor is among the declared reset: {reset:?}"
    );
    assert_eq!(
        state.load().await.expect("the state reads"),
        IngestState {
            anchor: AnchorState::Missing,
            rescore_pending: false,
            refusal: None,
            window_base: None,
        }
    );
}

#[tokio::test]
async fn the_refused_record_is_exported_and_an_erase_clears_it() {
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    let state = SqliteIngestState::new(db.clone());
    state
        .record_refusal(
            RefusalReason::SyncSettingsRefused,
            UtcMillis::from_epoch_millis(7_000),
        )
        .await
        .expect("a refusal is recorded");
    let mut write = db.write().await.expect("a write");
    let exported = IngestDataRights
        .export(&mut write)
        .await
        .expect("the port exports");
    let row = &exported
        .iter()
        .find(|table| table.table == INGEST_STATE_TABLE)
        .expect("ingest_state is exported")
        .rows[0];
    assert_eq!(row["refused_reason"], "sync_settings_refused");
    assert_eq!(row["refused_at"], 7_000);
    IngestDataRights
        .erase(&mut write)
        .await
        .expect("the port erases");
    write.commit().await.expect("the erase commits");
    assert_eq!(state.load().await.expect("read").refusal, None);
}

#[test]
fn the_declared_reset_row_clears_the_anchor_and_the_base_and_no_more() {
    let declaration = IngestDataRights
        .declaration()
        .expect("ingest's declaration is well formed");
    let Some(Disposition::ResetInPlace { row }) = declaration.disposition(INGEST_STATE_TABLE)
    else {
        panic!("ingest_state is reset in place");
    };
    let cleared = [
        "anchor_newest_review_id",
        "anchor_card_count",
        "anchor_card_fingerprint",
        "anchor_study_day",
        "anchor_recomputed_at",
        "anchor_settings_generation",
        "window_floor",
        "window_count",
        "refused_at",
        "refused_reason",
    ];
    assert_eq!(row.len(), cleared.len() + 1, "{row:?}");
    for column in cleared {
        assert_eq!(row.get(column), Some(&Value::Null), "{column}");
    }
    assert_eq!(row.get("rescore_pending"), Some(&json!(0)));
}

#[tokio::test]
async fn preset_proposals_are_exported_and_erased() {
    let declaration = IngestDataRights
        .declaration()
        .expect("ingest's declaration is well formed");
    assert_eq!(
        declaration.disposition("preset_proposals"),
        Some(&Disposition::ExportAndErase),
        "preset_proposals is the owner's data: exported and erased (SPEC-387 R9)"
    );

    // What the declaration says, the port does: an open and a settled proposal are exported with
    // their values, then erased.
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO preset_proposals (preset_id, preset_name, prior_vector, prior_field, \
         proposed_vector, desired_retention, non_new_cards, state, settled_at, retention_kept, \
         created_at) VALUES \
         (1001, 'Main', '[0.5,1.25]', 'fsrs6', '[0.25,2.5]', 0.85, 5, 'open', NULL, NULL, 7000), \
         (1002, 'Five', '[]', 'empty', '[0.25,2.5]', 0.9, 1, 'moved', 9000, 1, 8000)",
    )
    .execute(&mut *write)
    .await
    .expect("two proposals are recorded");
    let exported = IngestDataRights
        .export(&mut write)
        .await
        .expect("the port exports");
    let table = exported
        .iter()
        .find(|table| table.table == "preset_proposals")
        .expect("preset_proposals is exported");
    let names: Vec<(&Value, &Value, &Value)> = table
        .rows
        .iter()
        .map(|row| (&row["preset_name"], &row["state"], &row["prior_vector"]))
        .collect();
    assert_eq!(
        names,
        vec![
            (&json!("Main"), &json!("open"), &json!("[0.5,1.25]")),
            (&json!("Five"), &json!("moved"), &json!("[]")),
        ]
    );
    IngestDataRights
        .erase(&mut write)
        .await
        .expect("the port erases");
    write.commit().await.expect("the erase commits");
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM preset_proposals")
        .fetch_one(db.reader())
        .await
        .expect("a count");
    assert_eq!(remaining, 0, "the erased record holds no proposal");
}
