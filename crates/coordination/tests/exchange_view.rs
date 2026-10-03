//! The XP exchange readout's view (SPEC-075 R4 to R6, R8; A6, A7, A8; #80): the window equals the
//! predecessor's golden; the readout reads both XP tables and loses no row in the window, each
//! bucket's graduations those of the days it paid on; and it writes nothing.

// An integration test is test code: its helpers panic on a malformed case or a failed fixture,
// and the examined counts are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_coordination::progression::exchange_view::{exchange_view, exchange_window};
use deck_streak_kernel::{Db, StudyDay};
use deck_streak_progression::exchange::SourceRate;
use serde_json::{Value, json};

/// Today, as an epoch day.
const TODAY: i64 = 20_000;

/// The study day with epoch day number `day`.
const fn day(day: i64) -> StudyDay {
    StudyDay::from_epoch_day(day)
}

/// A golden integer field.
fn int(value: &Value, name: &str) -> i64 {
    value[name]
        .as_i64()
        .unwrap_or_else(|| panic!("an integer {name} in {value}"))
}

/// A rate.
fn rate(source: &str, total_xp: i64, graduated_cards: i64) -> SourceRate {
    #[allow(
        clippy::cast_precision_loss,
        reason = "the test's counts are far below 2^53"
    )]
    let rate = (graduated_cards > 0).then(|| total_xp as f64 / graduated_cards as f64);
    SourceRate {
        source: source.to_owned(),
        total_xp,
        graduated_cards,
        rate,
        rate_defined: graduated_cards > 0,
    }
}

#[test]
fn the_exchange_window_matches_the_parity_golden() {
    let examined = golden::each_case("exchange_window", |case| {
        let window = exchange_window(int(&case.input, "days"), day(int(&case.input, "today")));
        let ours = json!({
            "start": window.map(|(first, _)| first.epoch_day()),
            "end": window.map(|(_, last)| last.epoch_day()),
        });
        assert_eq!(ours, case.output, "{}", case.input);
    });
    assert_eq!(examined.function, "server.create_server");
}

/// A migrated database in `scratch` holding grants and settled rows inside and outside the last
/// three days, and the rollups their days' graduations come from.
///
/// Inside the window (19_998 to 20_000): `quest:1` 30 and `quest:2` 45 granted on 20_000, which
/// graduated 4 cards; `reviews` settled 12 on 19_999 (2 graduated) and 20 on 20_000; `focus`
/// granted 9 on 19_998, which has no rollup. Outside it: `quest:3` 500 granted and `reviews` 99
/// settled on 19_990, which graduated 50.
async fn seeded(scratch: &tempfile::TempDir) -> Db {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let mut write = db.write().await.expect("a write");
    for (on, graduated) in [(19_990_i64, 50_i64), (19_999, 2), (20_000, 4)] {
        sqlx::query(
            "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, \
             relearn_count, filtered_count, seconds, answered, passed, true_retention, \
             graduations, decks_studied, avg_answer_seconds, young_answered, young_passed, \
             mature_answered, mature_passed, mature_count, young_count, leech_active, backlog, \
             due_today, card_state_src, score, consistency, retention, workload, volume, mastery, \
             score_at_close, settled_at, fingerprint, created_at, updated_at) \
             VALUES (?1, 42, 0, 42, 0, 0, 600.0, 40, 36, 90.0, ?2, 2, 14.0, 0, 0, 40, 36, 64, 60, \
             2, 5, 30, 'live:1736911800000', 77, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, \
             'synthetic', 1000, 1000)",
        )
        .bind(on)
        .bind(graduated)
        .execute(&mut *write)
        .await
        .expect("the synthetic rollup is written");
    }
    for (on, source, amount) in [
        (20_000_i64, "quest:1", 30_i64),
        (20_000, "quest:2", 45),
        (19_998, "focus", 9),
        (19_990, "quest:3", 500),
    ] {
        sqlx::query(
            "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
             VALUES (?1, ?2, 'language', ?3, 'per-day', 1000)",
        )
        .bind(on)
        .bind(source)
        .bind(amount)
        .execute(&mut *write)
        .await
        .expect("the synthetic grant is written");
    }
    for (on, amount) in [(19_999_i64, 12_i64), (20_000, 20), (19_990, 99)] {
        sqlx::query(
            "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
             VALUES (?1, 'reviews', 'language', ?2, 1, 1000)",
        )
        .bind(on)
        .bind(amount)
        .execute(&mut *write)
        .await
        .expect("the synthetic settlement is written");
    }
    write.commit().await.expect("the commit");
    db
}

/// The seeded readout of the last three days.
fn the_seeded_window() -> Vec<SourceRate> {
    vec![
        rate("focus", 9, 0),
        rate("quest:", 75, 4),
        rate("reviews", 32, 6),
    ]
}

/// Every row of both XP tables, in table and id order, as text.
async fn every_xp_row(db: &Db) -> Vec<String> {
    let mut read = db.reader().acquire().await.expect("a reader");
    let mut rows = Vec::new();
    for statement in [
        "SELECT 'ledger', id, study_day, source, track, amount, scope, created_at \
         FROM xp_ledger ORDER BY id",
        "SELECT 'settlement', id, study_day, source, track, amount, CAST(closed AS TEXT), \
         created_at FROM xp_settlement ORDER BY id",
    ] {
        let found: Vec<(String, i64, i64, String, String, i64, String, i64)> =
            sqlx::query_as(statement)
                .fetch_all(&mut *read)
                .await
                .expect("the XP rows are read");
        rows.extend(found.into_iter().map(|row| format!("{row:?}")));
    }
    rows
}

/// The sum of both XP tables' amounts from `first` to `last`, read apart from the readout.
async fn window_xp(db: &Db, first: i64, last: i64) -> i64 {
    let mut read = db.reader().acquire().await.expect("a reader");
    sqlx::query_scalar(
        "SELECT (SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE study_day BETWEEN ?1 AND ?2) \
         + (SELECT COALESCE(SUM(amount), 0) FROM xp_settlement WHERE study_day BETWEEN ?1 AND ?2)",
    )
    .bind(first)
    .bind(last)
    .fetch_one(&mut *read)
    .await
    .expect("the window's XP")
}

#[tokio::test]
async fn the_readout_reads_both_xp_tables_and_loses_no_row() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = seeded(&scratch).await;
    let view = exchange_view(&db, day(TODAY), 3)
        .await
        .expect("the readout is read");
    assert_eq!(view.window, Some((day(19_998), day(TODAY))));
    println!("examined {} bucket(s)", view.rates.len());
    assert_eq!(view.rates, the_seeded_window());
    let answered: i64 = view.rates.iter().map(|rate| rate.total_xp).sum();
    assert_eq!(answered, window_xp(&db, 19_998, TODAY).await);
    db.close().await;
}

#[tokio::test]
async fn the_readout_writes_nothing() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = seeded(&scratch).await;
    let before = every_xp_row(&db).await;
    println!("examined {} XP row(s)", before.len());
    assert_eq!(before.len(), 7, "every seeded row is read back");
    let view = exchange_view(&db, day(TODAY), 3)
        .await
        .expect("the readout is read");
    assert_eq!(view.rates, the_seeded_window());
    assert_eq!(every_xp_row(&db).await, before);
    db.close().await;
}
