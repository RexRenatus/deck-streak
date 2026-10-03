//! The XP exchange readout's fold and reads (SPEC-075 R4, R5; A3, A4, A5, A12; #80): each source
//! bucket's XP over the graduations of the distinct days it paid on, equal to the predecessor's
//! goldens; no rate where nothing graduated; and the window's rows read from both XP tables.

// An integration test is test code: its helpers panic on a malformed case or a failed fixture,
// and the examined counts are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeMap;

use deck_streak_kernel::{Db, StudyDay};
use deck_streak_progression::exchange::{SourceRate, XpRow, bucket, exchange_rates, xp_rows};
use serde_json::{Value, json};

/// The study day with epoch day number `day`.
const fn day(day: i64) -> StudyDay {
    StudyDay::from_epoch_day(day)
}

/// An XP row.
fn row(on: i64, source: &str, amount: i64) -> XpRow {
    XpRow {
        study_day: day(on),
        source: source.to_owned(),
        amount,
    }
}

/// A golden integer field.
fn int(value: &Value, name: &str) -> i64 {
    value[name]
        .as_i64()
        .unwrap_or_else(|| panic!("an integer {name} in {value}"))
}

/// A rate as JSON, its float compared bit for bit through its exact text.
fn rate_json(rate: &SourceRate) -> Value {
    json!({
        "source": rate.source,
        "total_xp": rate.total_xp,
        "graduated_cards": rate.graduated_cards,
        "rate": rate.rate,
        "rate_defined": rate.rate_defined,
    })
}

#[test]
fn the_exchange_readout_matches_the_parity_golden() {
    let examined = golden::each_case("exchange_rates", |case| {
        let rows: Vec<XpRow> = case.input["rows"]
            .as_array()
            .expect("the case's rows")
            .iter()
            .map(|r| {
                row(
                    int(r, "day"),
                    r["source"].as_str().expect("a source"),
                    int(r, "amount"),
                )
            })
            .collect();
        let graduations: BTreeMap<StudyDay, i64> = case.input["rollups"]
            .as_array()
            .expect("the case's rollups")
            .iter()
            .map(|r| (day(int(r, "day")), int(r, "graduations")))
            .collect();
        let ours: Vec<Value> = exchange_rates(&rows, &graduations)
            .iter()
            .map(rate_json)
            .collect();
        let theirs = case.output.as_array().expect("the case's rates");
        assert_eq!(ours.len(), theirs.len(), "{}", case.input);
        for (mine, golden) in ours.iter().zip(theirs) {
            assert_eq!(mine["source"], golden["source"], "{}", case.input);
            assert_eq!(mine["total_xp"], golden["total_xp"], "{}", case.input);
            assert_eq!(
                mine["graduated_cards"], golden["graduated_cards"],
                "{}",
                case.input
            );
            assert_eq!(
                mine["rate_defined"], golden["rate_defined"],
                "{}",
                case.input
            );
            assert_eq!(
                mine["rate"].as_f64().map(f64::to_bits),
                golden["rate"].as_f64().map(f64::to_bits),
                "{}",
                case.input
            );
            assert_eq!(
                mine["rate"].is_null(),
                golden["rate"].is_null(),
                "{}",
                case.input
            );
        }
    });
    assert_eq!(examined.function, "exchange.exchange_rates");
}

#[test]
fn a_bucket_with_no_graduation_has_an_undefined_rate() {
    // focus pays on 19_998 (a rollup with no graduation) and 19_997 (no rollup at all); reviews
    // pays on 20_000, when two cards graduated.
    let rows = [
        row(19_998, "focus", 50),
        row(19_997, "focus", 60),
        row(20_000, "reviews", 7),
    ];
    let graduations = BTreeMap::from([(day(19_998), 0), (day(20_000), 2)]);
    assert_eq!(
        exchange_rates(&rows, &graduations),
        vec![
            SourceRate {
                source: "focus".to_owned(),
                total_xp: 110,
                graduated_cards: 0,
                rate: None,
                rate_defined: false,
            },
            SourceRate {
                source: "reviews".to_owned(),
                total_xp: 7,
                graduated_cards: 2,
                rate: Some(3.5),
                rate_defined: true,
            },
        ]
    );
}

#[test]
fn the_bucket_of_a_source_matches_the_parity_golden() {
    let examined = golden::each_case("exchange_normalize_source", |case| {
        let source = case.input["source"].as_str().expect("a source");
        assert_eq!(
            bucket(source),
            case.output.as_str().expect("a bucket"),
            "{source:?}"
        );
    });
    assert_eq!(examined.function, "exchange.normalize_source");
}

/// A migrated database in `scratch` holding one grant and two settled rows, each table with a row
/// inside the window 19_998 to 20_000 and one outside it.
async fn seeded(scratch: &tempfile::TempDir) -> Db {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let mut write = db.write().await.expect("a write");
    for (on, source, amount) in [(20_000_i64, "quest:1", 30_i64), (19_990, "quest:2", 50)] {
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
    for (on, amount) in [(19_999_i64, 12_i64), (19_995, 99)] {
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

#[tokio::test]
async fn the_window_rows_read_both_xp_tables() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = seeded(&scratch).await;
    let mut read = db.reader().acquire().await.expect("a reader");

    // The window holds the grant of 20_000 and the settled row of 19_999, and nothing else.
    let mut windowed = xp_rows(&mut read, Some((day(19_998), day(20_000))))
        .await
        .expect("the window's rows");
    windowed.sort();
    assert_eq!(
        windowed,
        vec![row(19_999, "reviews", 12), row(20_000, "quest:1", 30)]
    );

    // No window reads every row of both tables.
    let mut every = xp_rows(&mut read, None).await.expect("every row");
    every.sort();
    println!("examined {} XP row(s)", every.len());
    assert_eq!(
        every,
        vec![
            row(19_990, "quest:2", 50),
            row(19_995, "reviews", 99),
            row(19_999, "reviews", 12),
            row(20_000, "quest:1", 30),
        ]
    );
    drop(read);
    db.close().await;
}
