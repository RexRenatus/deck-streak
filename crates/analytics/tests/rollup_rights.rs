//! Analytics' data-rights port exports and erases both of its tables (SPEC-071 A26; R23;
//! SPEC-021; CHARTER 13): the declaration names `daily_rollup` and `daily_lang_stats` as exported
//! and erased, an export carries every row, and an erase leaves both empty. Every row is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;

use deck_streak_analytics::data_rights::AnalyticsDataRights;
use deck_streak_analytics::metrics::{DailyMetrics, LanguageDay};
use deck_streak_analytics::rollup::{RolledDay, record_card_state, roll_up};
use deck_streak_analytics::score::{Baseline, compute_score};
use deck_streak_analytics::snapshot::CardState;
use deck_streak_kernel::{
    CourseCode, DataRights, Db, Disposition, StudyDay, TableRights, UtcMillis,
};
use tempfile::TempDir;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

async fn count(db: &Db, table: &str) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
        .fetch_one(db.reader())
        .await
        .expect("the count reads")
}

#[allow(
    clippy::too_many_lines,
    reason = "A26 is one export and one erase, judged table by table in one test"
)]
#[tokio::test]
async fn the_rollup_tables_are_exported_and_erased() {
    let port = AnalyticsDataRights;
    let declaration = port.declaration().expect("the declaration holds");
    assert_eq!(declaration.context(), "analytics");
    assert_eq!(
        declaration.tables(),
        [
            TableRights {
                table: "daily_rollup",
                disposition: Disposition::ExportAndErase,
            },
            TableRights {
                table: "daily_lang_stats",
                disposition: Disposition::ExportAndErase,
            },
        ],
        "the owner's rollups are the owner's data"
    );

    let scratch = TempDir::new().expect("a scratch directory");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    let courses = [
        CourseCode::new("qaa").expect("a code"),
        CourseCode::new("qab").expect("a code"),
    ];
    let mut write = db.write().await.expect("a write");
    for offset in 0..3 {
        let day = StudyDay::from_epoch_day(20_000 + offset);
        let metrics = DailyMetrics {
            reviews: 10 + offset,
            seconds: 300.0,
            ..DailyMetrics::empty(day)
        };
        let languages: Vec<LanguageDay> = courses
            .iter()
            .map(|course| LanguageDay {
                day,
                course: *course,
                reviews: 5,
                seconds: 150.0,
                answered: 0,
                passed: 0,
            })
            .collect();
        let score = compute_score(
            &metrics,
            None,
            2,
            Baseline {
                reviews: 10.0,
                minutes: 5.0,
            },
        );
        roll_up(
            &mut write,
            &RolledDay {
                metrics: &metrics,
                languages: &languages,
                fingerprint: "synthetic",
                score: &score,
            },
            UtcMillis::from_epoch_millis(1_000 + offset),
        )
        .await
        .expect("the day rolls up");
    }
    record_card_state(
        &mut write,
        StudyDay::from_epoch_day(20_002),
        &CardState::default(),
        UtcMillis::from_epoch_millis(5_000),
    )
    .await
    .expect("a card state records");
    write.commit().await.expect("the seeds commit");

    let mut connection = db.reader().acquire().await.expect("a connection");
    let exported = port
        .export(&mut connection)
        .await
        .expect("the export reads");
    drop(connection);
    let tables: Vec<&str> = exported.iter().map(|table| table.table).collect();
    assert_eq!(tables, ["daily_rollup", "daily_lang_stats"]);
    let rollups = examined("exported rollups", exported[0].rows.clone());
    assert_eq!(rollups.len(), 3, "every rollup is exported");
    let days: BTreeSet<i64> = rollups
        .iter()
        .map(|row| row["study_day"].as_i64().expect("a day"))
        .collect();
    assert_eq!(days, BTreeSet::from([20_000, 20_001, 20_002]));
    for column in [
        "reviews",
        "seconds",
        "score",
        "card_state_src",
        "settled_at",
        "fingerprint",
    ] {
        assert!(
            rollups[0].get(column).is_some(),
            "the export carries {column}"
        );
    }
    assert_eq!(rollups[2]["card_state_src"], "live:5000");
    assert_eq!(
        examined("exported per-course rows", exported[1].rows.clone()).len(),
        6,
        "every per-course row is exported"
    );

    let mut write = db.write().await.expect("a write");
    port.erase(&mut write).await.expect("the erase runs");
    write.commit().await.expect("the erase commits");
    assert_eq!(
        count(&db, "daily_rollup").await,
        0,
        "an erase empties the rollups"
    );
    assert_eq!(
        count(&db, "daily_lang_stats").await,
        0,
        "and the per-course rows"
    );
}
