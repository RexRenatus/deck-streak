//! The minutes log is the owner's data (SPEC-078 A22; R21): habits' port declares `minutes_log`
//! exported and erased, the export holds every entry, and an erase leaves the table empty. The
//! writing log is too (SPEC-078 A22b), and it holds a course at most once a study day (A10b).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_habits::data_rights::{HABITS_CONTEXT, HabitsDataRights, MINUTES_LOG_TABLE};
use deck_streak_habits::store::{
    clear_writing, confirm_writing, confirmations_between, confirmations_through,
};
use deck_streak_kernel::{DataRights, Db, Disposition, StudyDay, UtcMillis};

async fn database(directory: &tempfile::TempDir) -> Db {
    Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens")
}

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
        .map(|rights| {
            (
                rights.table,
                matches!(rights.disposition, Disposition::ExportAndErase),
            )
        })
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

#[tokio::test]
async fn a_course_is_confirmed_once_a_day() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = database(&directory).await;
    let (day, next) = (
        StudyDay::from_epoch_day(20_104),
        StudyDay::from_epoch_day(20_105),
    );
    let at = UtcMillis::from_epoch_millis(1_000);
    let mut write = db.write().await.expect("a write");
    let first = confirm_writing(&mut write, "qab", day, at)
        .await
        .expect("a confirmation");
    let again = confirm_writing(&mut write, "qab", day, at)
        .await
        .expect("a confirmation");
    let other = confirm_writing(&mut write, "qac", day, at)
        .await
        .expect("a confirmation");
    let later = confirm_writing(&mut write, "qab", next, at)
        .await
        .expect("a confirmation");
    assert_eq!(
        (first, again, other, later),
        (true, false, true, true),
        "written once a day"
    );
    let rows = confirmations_between(&mut write, day, next)
        .await
        .expect("the rows");
    assert_eq!(
        rows,
        [
            ("qab".to_owned(), day),
            ("qac".to_owned(), day),
            ("qab".to_owned(), next)
        ],
        "one row per course and day, by day then code"
    );
    let duplicate = sqlx::query(
        "INSERT INTO writing_log (code, study_day, created_at) VALUES ('qab', 20104, 2000)",
    )
    .execute(&mut *write)
    .await;
    assert!(
        duplicate.is_err(),
        "the table refuses a second row for a course's day"
    );
    assert_eq!(
        confirmations_through(&mut write, day)
            .await
            .expect("a count"),
        2
    );
    assert!(
        clear_writing(&mut write, "qab", day)
            .await
            .expect("a clear")
    );
    assert!(
        !clear_writing(&mut write, "qab", day)
            .await
            .expect("a clear"),
        "nothing left"
    );
    assert_eq!(
        confirmations_through(&mut write, next)
            .await
            .expect("a count"),
        2
    );
    write.commit().await.expect("commit");
}

#[tokio::test]
async fn the_writing_log_is_exported_and_erased() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = database(&directory).await;
    let port = HabitsDataRights;
    let declaration = port.declaration().expect("the declaration");
    assert!(
        declaration
            .tables()
            .iter()
            .any(|rights| rights.table == "writing_log"
                && matches!(rights.disposition, Disposition::ExportAndErase)),
        "writing_log is declared exported and erased"
    );
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO writing_log (code, study_day, created_at) \
         VALUES ('qab', 20104, 1000), ('qac', 20104, 1500), ('qab', 20105, 2000)",
    )
    .execute(&mut *write)
    .await
    .expect("three synthetic confirmations");
    let exported = port.export(&mut write).await.expect("the export");
    let writing = exported
        .iter()
        .find(|table| table.table == "writing_log")
        .expect("the writing log is exported");
    assert_eq!(writing.rows.len(), 3, "every confirmation is exported");
    assert_eq!(writing.rows[0]["code"], "qab");
    assert_eq!(writing.rows[2]["study_day"], 20_105);
    port.erase(&mut write).await.expect("the erase");
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM writing_log")
        .fetch_one(&mut *write)
        .await
        .expect("the count");
    assert_eq!(left, 0, "an erase leaves the writing log empty");
    write.commit().await.expect("commit");
}
