//! The maintenance job checkpoints the write-ahead log, optimises the planner's statistics, and
//! prunes ledger rows past the retention (SPEC-027 A13; R9).

// An integration test is test code: its helpers panic on a failed database, and the examined
// counts are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;

use deck_streak_coordination::jobs::FireDate;
use deck_streak_coordination::maintenance::{CRON_FIRES_RETENTION_DAYS, upkeep};
use deck_streak_kernel::Db;

/// Today, as the maintenance fire's date: a synthetic epoch day.
const TODAY: i64 = 20_000;

#[tokio::test]
async fn maintenance_checkpoints_optimises_and_prunes_the_ledger() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("deck_streak.db");
    let db = Db::open(&path).await.expect("the database opens");

    // Ledger rows at ages 0, 89, 90, 91 and 200 days, for two jobs, plus a year of rows of a third
    // job so the planner's statistics have a table worth describing.
    let ages = [0, 89, 90, 91, 200];
    let mut write = db.write().await.expect("a write");
    for job in ["synthetic_daily", "synthetic_other"] {
        for age in ages {
            sqlx::query(
                "INSERT INTO cron_fires (job_id, fire_date, first_seen_at, updated_at, \
                 last_fire_at, ok_count, last_outcome, created_at) \
                 VALUES (?1, ?2, 0, 0, 0, 1, 'ok', 0)",
            )
            .bind(job)
            .bind(TODAY - age)
            .execute(&mut *write)
            .await
            .expect("a planted row");
        }
    }
    for age in 0..365 {
        sqlx::query(
            "INSERT INTO cron_fires (job_id, fire_date, first_seen_at, updated_at, \
             last_fire_at, ok_count, last_outcome, created_at) \
             VALUES ('synthetic_hourly', ?1, 0, 0, 0, 24, 'ok', 0)",
        )
        .bind(TODAY - 1000 - age)
        .execute(&mut *write)
        .await
        .expect("a planted row");
    }
    write.commit().await.expect("the rows commit");
    let wal = directory.path().join("deck_streak.db-wal");
    let before = fs::metadata(&wal).expect("the write-ahead log").len();
    assert!(before > 0, "the planted rows sit in the write-ahead log");

    let done = upkeep(&db, FireDate::from_epoch_day(TODAY))
        .await
        .expect("the upkeep runs");

    // Pruned: every row more than 90 days old; kept: the rows 90 days old and younger.
    assert_eq!(CRON_FIRES_RETENTION_DAYS, 90);
    assert_eq!(done.pruned, 2 * 2 + 365, "{done:?}");
    let kept: Vec<(String, i64)> =
        sqlx::query_as("SELECT job_id, fire_date FROM cron_fires ORDER BY job_id, fire_date")
            .fetch_all(db.reader())
            .await
            .expect("the ledger reads");
    let mut expected = Vec::new();
    for job in ["synthetic_daily", "synthetic_other"] {
        for age in [90, 89, 0] {
            expected.push((job.to_owned(), TODAY - age));
        }
    }
    assert_eq!(kept, expected, "the rows within the retention survive");
    println!("examined {} ledger row(s)", ages.len() * 2 + 365);

    // Optimised: the planner holds statistics for the ledger.
    let described: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sqlite_stat1 WHERE tbl = 'cron_fires'")
            .fetch_one(db.reader())
            .await
            .expect("the planner's statistics read");
    assert!(described > 0, "PRAGMA optimize analysed the ledger");

    // Checkpointed: TRUNCATE moved every frame into the database and emptied the log.
    assert!(!done.checkpoint_busy, "{done:?}");
    let after = fs::metadata(&wal).expect("the write-ahead log").len();
    assert_eq!(after, 0, "TRUNCATE leaves an empty write-ahead log");
    db.close().await;
}
