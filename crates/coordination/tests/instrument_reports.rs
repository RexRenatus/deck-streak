//! SPEC-094 A16 and A17: one latest report per instrument, replaced in one write, and exported and
//! erased by coordination's port. Every row is synthetic.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use deck_streak_coordination::data_rights::CoordinationDataRights;
use deck_streak_coordination::instruments::InstrumentStore;
use deck_streak_insights::instrument::ReportEnvelope;
use deck_streak_kernel::{DataRights, Db, UtcMillis};
use serde_json::json;
use tempfile::TempDir;

fn envelope(instrument: &str, study_day: i64, marker: i64) -> ReportEnvelope {
    ReportEnvelope {
        instrument: instrument.to_owned(),
        study_day,
        schema_version: 1,
        failed_reads: Vec::new(),
        report: json!({ "marker": marker }),
    }
}

async fn database(dir: &TempDir) -> Db {
    Db::open(&dir.path().join("ds.db")).await.expect("open")
}

async fn row_count(db: &Db) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM instrument_reports")
        .fetch_one(db.reader())
        .await
        .expect("count")
}

#[tokio::test]
async fn a_report_replaces_its_instruments_previous_one() {
    let dir = TempDir::new().expect("dir");
    let db = database(&dir).await;
    let store = InstrumentStore::new(db.clone());
    store
        .replace(
            &envelope("alpha", 20_000, 1),
            UtcMillis::from_epoch_millis(1_000),
        )
        .await
        .expect("first");
    store
        .replace(
            &envelope("beta", 20_001, 7),
            UtcMillis::from_epoch_millis(1_500),
        )
        .await
        .expect("other instrument");
    store
        .replace(
            &envelope("alpha", 20_008, 2),
            UtcMillis::from_epoch_millis(2_000),
        )
        .await
        .expect("second");

    let latest = store.get("alpha").await.expect("get").expect("a report");
    assert_eq!(latest.study_day, 20_008);
    assert_eq!(latest.created_at, 2_000);
    assert_eq!(latest.report["report"]["marker"], 2);
    assert_eq!(row_count(&db).await, 2, "one row per instrument");
    let other = store.get("beta").await.expect("get").expect("beta kept");
    assert_eq!(other.report["report"]["marker"], 7);
    assert_eq!(store.list().await.expect("list").len(), 2);
}

#[tokio::test]
async fn the_instrument_reports_are_exported_and_erased() {
    let dir = TempDir::new().expect("dir");
    let db = database(&dir).await;
    let store = InstrumentStore::new(db.clone());
    store
        .replace(
            &envelope("alpha", 20_000, 1),
            UtcMillis::from_epoch_millis(1_000),
        )
        .await
        .expect("seed");
    let port = CoordinationDataRights;
    let declaration = port.declaration().expect("declaration");
    assert!(
        declaration
            .tables()
            .iter()
            .any(|t| t.table == "instrument_reports"),
        "the table is declared"
    );

    let mut connection = db.reader().acquire().await.expect("connection");
    let exported = port.export(&mut connection).await.expect("export");
    let table = exported
        .iter()
        .find(|t| t.table == "instrument_reports")
        .expect("the export carries the table");
    assert_eq!(table.rows.len(), 1);
    drop(connection);

    let mut transaction = db.write().await.expect("write");
    port.erase(&mut transaction).await.expect("erase");
    transaction.commit().await.expect("commit");
    assert_eq!(row_count(&db).await, 0);
}
