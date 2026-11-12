//! The minutes log's store reads and writes exactly what the log holds (SPEC-078 R2, R4, R21): an
//! entry is written whole and answers its own id, the newest is read back whole, a removal takes
//! only the entry it names, and each sum covers its span's both bounds, in its stated order. Every
//! answer is checked against the table as plain SQL reads it, never against another store call.
//! Every course, day and instant is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_habits::minutes::Entry;
use deck_streak_habits::store::{
    LoggedEntry, all_time_by_code, insert, minutes_between, minutes_by_code, newest, remove,
};
use deck_streak_kernel::{CourseCode, Db, StudyDay, UtcMillis};
use sqlx::{Sqlite, Transaction};
use tempfile::TempDir;

/// A Monday's epoch day: its study week runs to [`SUNDAY`].
const MONDAY: i64 = 20_101;
/// The Sunday that ends [`MONDAY`]'s week.
const SUNDAY: i64 = 20_107;

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens")
}

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn code(text: &str) -> CourseCode {
    CourseCode::new(text).expect("a synthetic course code")
}

/// Writes one entry with plain SQL, as the oracle's own fixture.
async fn plant(write: &mut Transaction<'static, Sqlite>, code: &str, day: i64, minutes: i64) {
    sqlx::query(
        "INSERT INTO minutes_log (code, study_day, minutes, note, created_at) \
         VALUES (?1, ?2, ?3, '', 1000)",
    )
    .bind(code)
    .bind(day)
    .bind(minutes)
    .execute(&mut **write)
    .await
    .expect("a planted entry");
}

/// One row of the log: id, code, study day, minutes, note and when it was written.
type Row = (i64, String, i64, i64, String, i64);

/// Every row of the log, as plain SQL reads it, in id order.
async fn rows(write: &mut Transaction<'static, Sqlite>) -> Vec<Row> {
    sqlx::query_as(
        "SELECT id, code, study_day, minutes, note, created_at FROM minutes_log ORDER BY id",
    )
    .fetch_all(&mut **write)
    .await
    .expect("the log reads")
}

#[tokio::test]
async fn the_store_writes_an_entry_whole_and_answers_its_id() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let mut write = db.write().await.expect("a write");
    let first = Entry {
        code: code("qaa"),
        minutes: 30,
        note: "synthetic note".to_owned(),
    };
    let second = Entry {
        code: code("qab"),
        minutes: 45,
        note: String::new(),
    };
    let first_id = insert(
        &mut write,
        &first,
        day(MONDAY),
        UtcMillis::from_epoch_millis(1_000),
    )
    .await
    .expect("the first entry is written");
    let second_id = insert(
        &mut write,
        &second,
        day(MONDAY + 1),
        UtcMillis::from_epoch_millis(2_000),
    )
    .await
    .expect("the second entry is written");
    assert_eq!(
        rows(&mut write).await,
        [
            (
                1,
                "qaa".to_owned(),
                MONDAY,
                30,
                "synthetic note".to_owned(),
                1_000
            ),
            (2, "qab".to_owned(), MONDAY + 1, 45, String::new(), 2_000),
        ],
        "each entry is written whole, in the order logged"
    );
    assert_eq!(
        (first_id, second_id),
        (1, 2),
        "each write answers the id the log gave its entry"
    );
    write.commit().await.expect("commit");
}

#[tokio::test]
async fn the_newest_entry_is_read_back_whole() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let mut write = db.write().await.expect("a write");
    assert_eq!(
        newest(&mut write).await.expect("the newest reads"),
        None,
        "an empty log has no newest entry"
    );
    plant(&mut write, "qaa", MONDAY, 30).await;
    plant(&mut write, "qab", MONDAY + 2, 45).await;
    assert_eq!(
        newest(&mut write).await.expect("the newest reads"),
        Some(LoggedEntry {
            id: 2,
            code: "qab".to_owned(),
            day: day(MONDAY + 2),
            minutes: 45,
        }),
        "the entry of the highest id, with its course, day and minutes"
    );
    write.commit().await.expect("commit");
}

#[tokio::test]
async fn a_removal_takes_only_the_entry_it_names() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let mut write = db.write().await.expect("a write");
    plant(&mut write, "qaa", MONDAY, 30).await;
    plant(&mut write, "qab", MONDAY, 45).await;
    remove(&mut write, 1).await.expect("the removal");
    assert_eq!(
        rows(&mut write).await,
        [(2, "qab".to_owned(), MONDAY, 45, String::new(), 1_000)],
        "entry 1 is gone and entry 2 stays"
    );
    write.commit().await.expect("commit");
}

#[tokio::test]
async fn minutes_between_sums_one_course_over_both_bounds() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let mut write = db.write().await.expect("a write");
    plant(&mut write, "qaa", MONDAY, 30).await;
    plant(&mut write, "qaa", MONDAY + 2, 15).await;
    plant(&mut write, "qaa", MONDAY + 3, 7).await;
    plant(&mut write, "qab", MONDAY + 1, 20).await;
    let mut sums = Vec::new();
    for (course, first, last) in [
        ("qaa", MONDAY, MONDAY + 2),
        ("qaa", MONDAY + 2, MONDAY + 2),
        ("qab", MONDAY, SUNDAY),
        ("qab", SUNDAY + 1, SUNDAY + 7),
    ] {
        let sum = minutes_between(&mut write, course, day(first), day(last))
            .await
            .expect("the sum reads");
        sums.push(sum);
    }
    assert_eq!(
        sums,
        [45, 15, 20, 0],
        "both bounds count, other days and other courses do not"
    );
    write.commit().await.expect("commit");
}

#[tokio::test]
async fn minutes_by_code_sums_each_course_of_a_span_in_code_order() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let mut write = db.write().await.expect("a write");
    plant(&mut write, "qab", MONDAY, 20).await;
    plant(&mut write, "qaa", MONDAY + 1, 30).await;
    plant(&mut write, "qaa", SUNDAY, 15).await;
    plant(&mut write, "qaa", SUNDAY + 1, 60).await;
    assert_eq!(
        minutes_by_code(&mut write, day(MONDAY), day(SUNDAY))
            .await
            .expect("the sums read"),
        [("qaa".to_owned(), 45), ("qab".to_owned(), 20)],
        "each course of the week once, with its minutes there, in code order"
    );
    write.commit().await.expect("commit");
}

#[tokio::test]
async fn all_time_by_code_sums_each_course_in_the_order_first_logged() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let mut write = db.write().await.expect("a write");
    plant(&mut write, "qab", MONDAY, 20).await;
    plant(&mut write, "qaa", MONDAY + 1, 30).await;
    plant(&mut write, "qab", SUNDAY + 9, 10).await;
    assert_eq!(
        all_time_by_code(&mut write).await.expect("the sums read"),
        [("qab".to_owned(), 30), ("qaa".to_owned(), 30)],
        "every course once, with all its minutes, the first logged first"
    );
    write.commit().await.expect("commit");
}
