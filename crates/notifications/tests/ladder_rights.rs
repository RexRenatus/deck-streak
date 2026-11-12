//! The owner's latest message under the data-rights port (SPEC-084 A17, R14): notifications' port
//! declares `owner_last_message` reset in place, exports its row, and an erase resets the row to no
//! message, inside the caller's transaction.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

mod support;

use deck_streak_kernel::{DataRights, Disposition};
use deck_streak_notifications::data_rights::NotificationsDataRights;
use serde_json::{Value, json};
use sqlx::Row;
use support::ladder::seed_owner_message;
use support::{DAY, Harness, at};

/// The table this delivery adds.
const TABLE: &str = "owner_last_message";

#[tokio::test]
async fn the_owners_latest_message_is_exported_and_reset_by_an_erase() {
    let port = NotificationsDataRights;
    let declaration = port.declaration().expect("the port declares its tables");
    let declared = declaration
        .tables()
        .iter()
        .find(|rights| rights.table == TABLE)
        .map(|rights| rights.disposition.clone());
    let mut reset = serde_json::Map::new();
    reset.insert("message_id".to_owned(), Value::Null);
    reset.insert("arrived_at".to_owned(), Value::Null);
    assert_eq!(
        declared,
        Some(Disposition::ResetInPlace { row: reset }),
        "the owner's latest message is reset in place: no message"
    );

    let harness = Harness::new(at(DAY, 12, 0)).await;
    seed_owner_message(&harness, 4_242, 1_700_000_000_789).await;
    let mut write = harness.db.write().await.expect("a write");
    let export = port.export(&mut write).await.expect("the export");
    let rows: Vec<Value> = export
        .iter()
        .filter(|table| table.table == TABLE)
        .flat_map(|table| table.rows.clone())
        .map(|mut row| {
            row.as_object_mut()
                .expect("a row is an object")
                .remove("created_at");
            row
        })
        .collect();
    assert_eq!(
        rows,
        [json!({"id": 1, "message_id": 4_242, "arrived_at": 1_700_000_000_789_i64})],
        "the export carries the row"
    );
    port.erase(&mut write).await.expect("the erase");
    write.commit().await.expect("the erase commits");

    let after = sqlx::query("SELECT id, message_id, arrived_at FROM owner_last_message")
        .fetch_all(harness.db.reader())
        .await
        .expect("the table is read");
    let after: Vec<(i64, Option<i64>, Option<i64>)> = after
        .iter()
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .collect();
    assert_eq!(
        after,
        [(1, None, None)],
        "the erase resets the row in place"
    );
}
