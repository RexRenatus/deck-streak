//! The skip tables are the owner's data (SPEC-083 R17, A23): both are declared, exported and
//! erased.

// An integration test is test code.
#![allow(clippy::expect_used)]

mod support;

use deck_streak_ingest::data_rights::{
    IngestDataRights, SKIP_CARD_SNAPSHOT_TABLE, SKIP_DAYS_TABLE,
};
use deck_streak_ingest::skip::SkipStore;
use deck_streak_kernel::{DataRights, Disposition, StudyDay, UtcMillis};

#[tokio::test]
async fn the_skip_tables_are_exported_and_erased() {
    let declaration = IngestDataRights.declaration().expect("well formed");
    for table in [SKIP_DAYS_TABLE, SKIP_CARD_SNAPSHOT_TABLE] {
        assert_eq!(
            declaration.disposition(table),
            Some(&Disposition::ExportAndErase),
            "{table} is the owner's data: exported and erased (CHARTER 13)"
        );
    }
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    let skips = SkipStore::new(db.clone());
    let id = skips
        .begin(
            StudyDay::from_epoch_day(20_000),
            Some(9),
            UtcMillis::from_epoch_millis(5),
        )
        .await
        .expect("a take");
    skips.settle_applied(id, 2, true).await.expect("settle");
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO skip_card_snapshot (skip_id, card_id, prior_due, prior_queue, prior_type, \
         prior_interval, prior_ease_factor, prior_original_deck_id, prior_original_due, created_at) \
         VALUES (?1, 42, 3, 2, 2, 10, 2500, 0, 0, 5)",
    )
    .bind(id.get())
    .execute(&mut *write)
    .await
    .expect("a snapshot row");
    let exported = IngestDataRights.export(&mut write).await.expect("exports");
    let days = exported
        .iter()
        .find(|t| t.table == SKIP_DAYS_TABLE)
        .expect("skip_days exported");
    assert_eq!(days.rows.len(), 1, "{:?}", days.rows);
    assert_eq!(days.rows[0]["study_day"], 20_000);
    assert_eq!(days.rows[0]["state"], "applied");
    let cards = exported
        .iter()
        .find(|t| t.table == SKIP_CARD_SNAPSHOT_TABLE)
        .expect("the snapshot is exported");
    assert_eq!(cards.rows.len(), 1);
    assert_eq!(cards.rows[0]["card_id"], 42);
    IngestDataRights.erase(&mut write).await.expect("erases");
    write.commit().await.expect("commits");
    let days_left: i64 = sqlx::query_scalar("SELECT count(*) FROM skip_days")
        .fetch_one(db.reader())
        .await
        .expect("a count");
    assert_eq!(days_left, 0, "skip_days is erased");
    let cards_left: i64 = sqlx::query_scalar("SELECT count(*) FROM skip_card_snapshot")
        .fetch_one(db.reader())
        .await
        .expect("a count");
    assert_eq!(cards_left, 0, "skip_card_snapshot is erased");
}
