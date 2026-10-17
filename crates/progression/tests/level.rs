//! The level of a total is the predecessor's, and it is read from the ledger's total (SPEC-040 A4,
//! A5; R7, R8; CHARTER 8).
//!
//! The golden `level_for_xp.json` holds what `gamification/xp.py:level_for_xp` returned at
//! `27ee2bc`, over the curve's thresholds for levels 1 to 100 and one XP below each, over seeded
//! thresholds where a floating-point root misplaces the level, and over the widest total. Nothing
//! here derives an expected level from the port.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::{Db, StudyDay, Track, UtcMillis};
use deck_streak_progression::grant::{
    GrantAnswer, GrantPort, GrantRequest, GrantScope, GrantSource,
};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::xp::{XpAmount, XpTotal, level_for, xp_to_reach};

/// The total a golden case asks about.
fn total_of(case: &golden::Case) -> u64 {
    case.input["total_xp"]
        .as_u64()
        .unwrap_or_else(|| panic!("total_xp is an unsigned integer in {}", case.input))
}

#[test]
fn the_level_of_a_total_matches_the_parity_golden() {
    let mut levels = BTreeMap::new();
    let mut classes = BTreeSet::new();
    golden::each_case("level_for_xp", |case| {
        let total = total_of(case);
        let expected = case
            .output
            .as_u64()
            .expect("a level is an unsigned integer");
        let level = level_for(XpTotal::new(total));
        assert_eq!(u64::from(level.get()), expected, "the level of {total} XP");
        levels.insert(total, expected);
        classes.extend(case.class.clone());
    });
    // The totals a port gets wrong were examined, not only small ones.
    for class in [
        "threshold",
        "below",
        "large-threshold",
        "large-below",
        "widest",
    ] {
        assert!(classes.contains(class), "no {class} case was examined");
    }

    // Where the predecessor's level rises, from one XP below a total to the total itself, that
    // total is the level's threshold, and the XP to reach the level is exactly it; level 1 begins
    // at no XP.
    let mut thresholds = BTreeMap::new();
    for (&total, &level) in &levels {
        let rises = total == 0
            || total
                .checked_sub(1)
                .and_then(|below| levels.get(&below))
                .is_some_and(|&below| below + 1 == level);
        if rises {
            thresholds.insert(level, total);
        }
    }
    println!(
        "examined {} threshold(s) where the level rises",
        thresholds.len()
    );
    for (&level, &total) in &thresholds {
        let reached = level_for(XpTotal::new(total));
        assert_eq!(u64::from(reached.get()), level);
        assert_eq!(
            xp_to_reach(reached),
            XpTotal::new(total),
            "the XP to reach level {level}"
        );
    }
    // Every level from 1 to 100 was examined at its threshold.
    let small: Vec<u64> = thresholds
        .keys()
        .copied()
        .filter(|level| *level <= 100)
        .collect();
    assert_eq!(small, (1..=100).collect::<Vec<u64>>());
}

/// A migrated temporary database, and the ledger over it.
async fn ledger() -> (tempfile::TempDir, Db, SqliteXpLedger) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let ledger = SqliteXpLedger::new(db.clone());
    (directory, db, ledger)
}

/// Grants `amount` for `source` on `track`, for study day `day`, once.
async fn grant(ledger: &SqliteXpLedger, day: i64, source: &str, track: Track, amount: XpAmount) {
    let request = GrantRequest {
        study_day: StudyDay::from_epoch_day(day),
        source: GrantSource::new(source).expect("a source of the grammar"),
        track,
        amount,
        scope: GrantScope::Once,
    };
    let answer = ledger
        .grant(&request, UtcMillis::from_epoch_millis(1_700_000_000_000))
        .await
        .expect("the grant runs");
    assert_eq!(answer, GrantAnswer::Granted(amount));
}

/// The ledger's level, total and per-track totals, as numbers.
async fn reading(ledger: &SqliteXpLedger) -> (u32, u64, u64, u64) {
    let level = ledger.level().await.expect("the level is read");
    let total = ledger.total().await.expect("the total is read");
    let language = ledger
        .track_total(Track::Language)
        .await
        .expect("the language total is read");
    let law = ledger
        .track_total(Track::Law)
        .await
        .expect("the law total is read");
    (level.get(), total.get(), language.get(), law.get())
}

#[tokio::test]
async fn the_level_is_derived_from_the_ledger_total() {
    let (_directory, db, ledger) = ledger().await;
    assert_eq!(reading(&ledger).await, (1, 0, 0, 0), "no grant is level 1");
    grant(
        &ledger,
        20_000,
        "reading:read:r1",
        Track::Language,
        XpAmount::new(60),
    )
    .await;
    grant(
        &ledger,
        20_000,
        "reading:studied:r1",
        Track::Law,
        XpAmount::new(39),
    )
    .await;
    assert_eq!(
        reading(&ledger).await,
        (1, 99, 60, 39),
        "one XP short of level 2"
    );
    // The grant that crosses the threshold raises the level, read from the total.
    grant(
        &ledger,
        20_001,
        "reading:read:r2",
        Track::Law,
        XpAmount::new(1),
    )
    .await;
    assert_eq!(
        reading(&ledger).await,
        (2, 100, 60, 40),
        "level 2 begins at 100 XP"
    );

    // No level is stored: a row that reaches the ledger by any path moves the level on the next
    // read, because the level is derived from the rows' sum each time.
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
         VALUES (20001, 'synthetic:raw', 'language', 200, 'once', 1)",
    )
    .execute(&mut *write)
    .await
    .expect("a raw row");
    write.commit().await.expect("committed");
    assert_eq!(
        reading(&ledger).await,
        (3, 300, 260, 40),
        "level 3 begins at 300 XP"
    );
}
