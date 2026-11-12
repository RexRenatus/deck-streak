//! Notifications' data-rights port (SPEC-041 A14; R11; SPEC-021): the port lists the router's five
//! tables, each exported and erased, then the owner's latest message, reset in place (SPEC-084
//! R14); the export carries every row the router wrote; and an erase, inside the caller's
//! transaction, empties every one of the five.

// An integration test is test code: its fixtures panic on a failed setup, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use deck_streak_kernel::{DataRights, Disposition, TableRights};
use deck_streak_notifications::data_rights::{NOTIFICATIONS_CONTEXT, NotificationsDataRights};
use deck_streak_notifications::{LapseContext, Surface, Tier};
use serde_json::{Map, Value};
use sqlx::Row;
use support::{DAY, Harness, at};

/// The router's five tables.
const TABLES: [&str; 5] = [
    "notification_decisions",
    "notification_deliveries",
    "notification_queue",
    "in_app_feed",
    "notification_settings",
];

/// The rows each of the router's tables holds.
async fn counts(harness: &Harness) -> Vec<(&'static str, i64)> {
    let mut counts = Vec::new();
    for table in TABLES {
        let count: i64 = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
            .fetch_one(harness.db.reader())
            .await
            .expect("a table is counted")
            .get(0);
        counts.push((table, count));
    }
    counts
}

#[tokio::test]
async fn the_notification_tables_are_exported_and_erased() {
    let port = NotificationsDataRights;
    let declaration = port.declaration().expect("the port declares its tables");
    assert_eq!(declaration.context(), NOTIFICATIONS_CONTEXT);
    let mut no_message = Map::new();
    no_message.insert("message_id".to_owned(), Value::Null);
    no_message.insert("arrived_at".to_owned(), Value::Null);
    let mut tables: Vec<TableRights> = TABLES
        .map(|table| TableRights {
            table,
            disposition: Disposition::ExportAndErase,
        })
        .into();
    tables.push(TableRights {
        table: "owner_last_message",
        disposition: Disposition::ResetInPlace { row: no_message },
    });
    assert_eq!(
        declaration.tables(),
        tables,
        "the router's tables are the owner's data: exported and erased (CHARTER 13), and the \
         owner's latest message is reset in place"
    );

    // A row in every table: a send to the bot, a Mini App send, a deferral, and a setting.
    let harness = Harness::new(at(DAY, 12, 0)).await;
    for origin in [Surface::Bot, Surface::MiniApp] {
        let _sent = harness
            .router
            .route(&harness.celebration(&format!("export:{}", origin.as_str()), origin))
            .await
            .expect("a decision");
    }
    harness.clock.set(at(DAY, 23, 30));
    let _held = harness
        .router
        .route(&harness.occasion(
            "celebration",
            "export:held",
            Surface::Bot,
            Tier::T2,
            DAY,
            LapseContext::NoLapse,
        ))
        .await
        .expect("a decision");
    harness.set("habit_enabled", "1").await;
    let before = counts(&harness).await;
    println!("examined {} table(s)", before.len());
    assert!(
        before.iter().all(|(_, count)| *count > 0),
        "every table holds a row: {before:?}"
    );

    let mut write = harness.db.write().await.expect("a write");
    let export = port.export(&mut write).await.expect("the export");
    let exported: Vec<(&str, i64)> = export
        .iter()
        .map(|table| {
            (
                table.table,
                i64::try_from(table.rows.len()).expect("a count"),
            )
        })
        .collect();
    port.erase(&mut write).await.expect("the erase");
    write.commit().await.expect("the erase commits");

    let mut exported_before = before.clone();
    exported_before.push(("owner_last_message", 1));
    assert_eq!(
        exported, exported_before,
        "the export carries every row of every table"
    );
    assert_eq!(
        counts(&harness).await,
        TABLES.map(|table| (table, 0)),
        "the erase empties every table"
    );
}
