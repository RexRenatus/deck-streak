//! The minutes log is the owner's data (SPEC-078 A22; R21): habits' port declares `minutes_log`
//! exported and erased, the export holds every entry, and an erase leaves the table empty.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_habits::data_rights::{HABITS_CONTEXT, HabitsDataRights, MINUTES_LOG_TABLE};
use deck_streak_kernel::{DataRights, Db, Disposition};

#[tokio::test]
async fn the_habit_tables_are_exported_and_erased() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let port = HabitsDataRights;
    let declaration = port.declaration().expect("the declaration");
    assert_eq!(declaration.context(), HABITS_CONTEXT);
    let tables: Vec<_> = declaration
        .tables()
        .iter()
        .map(|rights| (rights.table, matches!(rights.disposition, Disposition::ExportAndErase)))
        .collect();
    assert_eq!(tables, [(MINUTES_LOG_TABLE, true)]);

    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO minutes_log (code, study_day, minutes, note, created_at) \
         VALUES ('qaa', 20101, 30, 'synthetic note', 1000), ('qab', 20102, 600, '', 2000)",
    )
    .execute(&mut *write)
    .await
    .expect("two synthetic entries");
    let exported = port.export(&mut write).await.expect("the export");
    assert_eq!(exported.len(), 1, "one table");
    assert_eq!(exported[0].table, MINUTES_LOG_TABLE);
    assert_eq!(exported[0].rows.len(), 2, "every entry is exported");
    assert_eq!(exported[0].rows[0]["note"], "synthetic note");
    assert_eq!(exported[0].rows[1]["minutes"], 600);
    port.erase(&mut write).await.expect("the erase");
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM minutes_log")
        .fetch_one(&mut *write)
        .await
        .expect("the count");
    assert_eq!(left, 0, "an erase leaves the log empty");
    write.commit().await.expect("commit");
}
