//! The settlement's and the buffs' reads answer what the tables hold, whole (SPEC-072 R6, R17,
//! R20 to R22), and a recompute over a day that is or was closed keeps the higher amount and stays
//! closed (R8).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{Db, StudyDay, Track, UtcMillis};
use deck_streak_progression::buffs::{arm_ascendant, is_ascendant_day};
use deck_streak_progression::settle::{
    SettleCause, SettleRequest, SettledRow, day_rows, settle, settled_amount, settled_of_day,
};

const AT: i64 = 1_700_000_000_000;
const DAY: i64 = 20_000;

async fn database() -> (tempfile::TempDir, Db) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (directory, db)
}

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

async fn settled(db: &Db, source: &str, track: Track, amount: u32, closed: bool, day_number: i64) {
    let request = SettleRequest {
        study_day: day(day_number),
        source,
        track,
        amount,
        closed,
    };
    let mut write = db.write().await.expect("a write");
    settle(
        &mut write,
        &request,
        SettleCause::Recompute,
        UtcMillis::from_epoch_millis(AT),
    )
    .await
    .expect("the settlement runs");
    write.commit().await.expect("commit");
}

#[tokio::test]
async fn an_open_row_recomputed_closed_keeps_the_higher_amount_and_closes() {
    let (_directory, db) = database().await;
    settled(&db, "reviews", Track::Language, 100, false, DAY).await;
    settled(&db, "reviews", Track::Language, 60, true, DAY).await;
    let mut read = db.reader().acquire().await.expect("a reader");
    let rows = settled_of_day(&mut read, day(DAY)).await.expect("read");
    assert_eq!(
        rows,
        vec![SettledRow {
            source: "reviews".to_owned(),
            track: "language".to_owned(),
            amount: 100,
            closed: true,
        }]
    );
}

#[tokio::test]
async fn a_closed_row_recomputed_open_keeps_the_higher_amount_and_stays_closed() {
    let (_directory, db) = database().await;
    settled(&db, "reviews", Track::Language, 100, true, DAY).await;
    settled(&db, "reviews", Track::Language, 60, false, DAY).await;
    let mut read = db.reader().acquire().await.expect("a reader");
    let rows = settled_of_day(&mut read, day(DAY)).await.expect("read");
    assert_eq!(
        rows,
        vec![SettledRow {
            source: "reviews".to_owned(),
            track: "language".to_owned(),
            amount: 100,
            closed: true,
        }]
    );
}

#[tokio::test]
async fn the_reads_answer_the_rows_of_the_day_and_no_other() {
    let (_directory, db) = database().await;
    settled(&db, "reviews", Track::Language, 30, false, DAY).await;
    settled(&db, "reviews_law", Track::Law, 12, true, DAY).await;
    settled(&db, "studied", Track::Language, 5, true, DAY + 1).await;
    {
        let mut write = db.write().await.expect("a write");
        for (day_number, source, amount) in [(DAY, "reading:read:a", 7), (DAY + 1, "other", 9)] {
            sqlx::query(
                "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
                 VALUES (?1, ?2, 'language', ?3, 'per-day', 1000)",
            )
            .bind(day_number)
            .bind(source)
            .bind(amount)
            .execute(&mut *write)
            .await
            .expect("the grant is written");
        }
        write.commit().await.expect("commit");
    }
    let mut read = db.reader().acquire().await.expect("a reader");

    let rows = settled_of_day(&mut read, day(DAY)).await.expect("read");
    assert_eq!(
        rows,
        vec![
            SettledRow {
                source: "reviews".to_owned(),
                track: "language".to_owned(),
                amount: 30,
                closed: false,
            },
            SettledRow {
                source: "reviews_law".to_owned(),
                track: "law".to_owned(),
                amount: 12,
                closed: true,
            },
        ]
    );

    assert_eq!(
        settled_amount(&mut read, day(DAY), "reviews", Track::Language)
            .await
            .expect("read"),
        Some(30)
    );
    assert_eq!(
        settled_amount(&mut read, day(DAY), "reviews_law", Track::Law)
            .await
            .expect("read"),
        Some(12)
    );
    assert_eq!(
        settled_amount(&mut read, day(DAY), "reviews_law", Track::Language)
            .await
            .expect("read"),
        None
    );
    assert_eq!(
        settled_amount(&mut read, day(DAY + 2), "reviews", Track::Language)
            .await
            .expect("read"),
        None
    );

    let mut held = day_rows(&mut read, day(DAY)).await.expect("read");
    held.sort();
    assert_eq!(
        held,
        vec![
            ("reading:read:a".to_owned(), 7),
            ("reviews".to_owned(), 30),
            ("reviews_law".to_owned(), 12),
        ]
    );
    assert!(
        day_rows(&mut read, day(DAY + 2))
            .await
            .expect("read")
            .is_empty()
    );
}

#[tokio::test]
async fn the_ascendant_buff_is_armed_once_and_read_by_its_day() {
    let (_directory, db) = database().await;
    let at = UtcMillis::from_epoch_millis(AT);
    {
        let mut write = db.write().await.expect("a write");
        assert!(!is_ascendant_day(&mut write, day(DAY)).await.expect("read"));
        assert!(
            arm_ascendant(&mut write, day(DAY), at)
                .await
                .expect("armed")
        );
        assert!(
            !arm_ascendant(&mut write, day(DAY), at)
                .await
                .expect("a second arm"),
            "a day that holds the buff is left as it is"
        );
        write.commit().await.expect("commit");
    }
    let mut read = db.reader().acquire().await.expect("a reader");
    assert!(is_ascendant_day(&mut read, day(DAY)).await.expect("read"));
    assert!(
        !is_ascendant_day(&mut read, day(DAY + 1))
            .await
            .expect("read")
    );
    assert!(
        !is_ascendant_day(&mut read, day(DAY - 1))
            .await
            .expect("read")
    );
    let held: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM buffs")
        .fetch_one(&mut *read)
        .await
        .expect("count");
    assert_eq!(held, 1);
}
