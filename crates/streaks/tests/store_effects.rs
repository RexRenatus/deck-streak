//! The streak store's effects (SPEC-076 A33): each writer's stored row is read back with a raw
//! query the store does not own, and each reader answers the rows written by raw SQL, so a
//! function that does nothing, or answers a constant, fails here. Every row is synthetic.

#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_streaks::freeze::{FreezeEvent, FreezeReason};
use deck_streak_streaks::store;
use deck_streak_streaks::streak::StreakState;
use sqlx::{Row, SqliteConnection};

const AT: i64 = 1_700_000_000_000;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn at() -> UtcMillis {
    UtcMillis::from_epoch_millis(AT)
}

async fn open(directory: &tempfile::TempDir) -> Db {
    Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens")
}

async fn count(connection: &mut SqliteConnection, table: &str) -> i64 {
    let query = match table {
        "streak_state" => "SELECT COUNT(*) FROM streak_state",
        "habit_strength" => "SELECT COUNT(*) FROM habit_strength",
        "freeze_events" => "SELECT COUNT(*) FROM freeze_events",
        other => panic!("no count for {other}"),
    };
    sqlx::query(query)
        .fetch_one(connection)
        .await
        .expect("a count")
        .get(0)
}

/// A rule (A33): a state written for a track is the row read back, for both values of every flag
/// and a second write replacing the first.
#[tokio::test]
async fn a_state_written_is_the_state_read_back_for_every_flag() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    assert_eq!(
        store::state(&mut write, "language").await.expect("read"),
        None
    );
    let mut cases = 0;
    for track in ["language", "law"] {
        for armed in [false, true] {
            for last in [None, Some(day(19_990))] {
                let state = StreakState {
                    current: 9,
                    longest: 12,
                    freezes: 2,
                    last_study_day: last,
                    comeback_armed: armed,
                };
                store::upsert_state(&mut write, track, &state, at())
                    .await
                    .expect("written");
                let row = sqlx::query(
                    "SELECT current_days, longest_days, freezes, last_study_day, comeback_armed,
                            created_at FROM streak_state WHERE track = ?1",
                )
                .bind(track)
                .fetch_one(&mut *write)
                .await
                .expect("the row");
                assert_eq!(row.get::<i64, _>(0), 9);
                assert_eq!(row.get::<i64, _>(1), 12);
                assert_eq!(row.get::<i64, _>(2), 2);
                assert_eq!(row.get::<Option<i64>, _>(3), last.map(StudyDay::epoch_day));
                assert_eq!(row.get::<i64, _>(4), i64::from(armed));
                assert_eq!(row.get::<i64, _>(5), AT);
                assert_eq!(
                    store::state(&mut write, track).await.expect("read"),
                    Some(state)
                );
                cases += 1;
            }
        }
    }
    assert_eq!(count(&mut write, "streak_state").await, 2);
    // A raw row with a negative count reads as zero, and another track stays unread.
    sqlx::query("UPDATE streak_state SET current_days = 0, longest_days = 0 WHERE track = 'law'")
        .execute(&mut *write)
        .await
        .expect("an update");
    let law = store::state(&mut write, "law")
        .await
        .expect("read")
        .expect("a row");
    assert_eq!((law.current, law.longest), (0, 0));
    println!("state population: {cases} writes");
    assert_eq!(cases, 8);
}

/// A rule (A33): the strength of each day is stored, the latest day's is the one read, a rewrite
/// replaces the day's value, and no rows read as none.
#[tokio::test]
async fn the_latest_strength_is_the_latest_day_stored() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    assert_eq!(
        store::latest_strength(&mut write).await.expect("read"),
        None
    );
    for (number, value) in [(19_998, 0.25), (20_000, 0.75), (19_999, 0.5)] {
        store::put_strength(&mut write, day(number), value, at())
            .await
            .expect("written");
    }
    assert_eq!(count(&mut write, "habit_strength").await, 3);
    assert_eq!(
        store::latest_strength(&mut write).await.expect("read"),
        Some(0.75)
    );
    store::put_strength(&mut write, day(20_000), 0.125, at())
        .await
        .expect("rewritten");
    assert_eq!(count(&mut write, "habit_strength").await, 3);
    let stored: f64 = sqlx::query("SELECT strength FROM habit_strength WHERE study_day = 20000")
        .fetch_one(&mut *write)
        .await
        .expect("the row")
        .get(0);
    assert!((stored - 0.125).abs() < f64::EPSILON);
    assert_eq!(
        store::latest_strength(&mut write).await.expect("read"),
        Some(0.125)
    );
}

/// A rule (A33): the governor row is written with the anchor and the standby flag and leaves the
/// notice day alone, and it is read back for both flags; a missing row reads as the start row.
#[tokio::test]
async fn the_governor_row_is_written_and_read_for_both_flags() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    sqlx::query("UPDATE governor_state SET notified_day = 19995")
        .execute(&mut *write)
        .await
        .expect("a notice day");
    let mut cases = 0;
    for standby in [false, true] {
        for anchor in [None, Some(day(19_990))] {
            store::put_governor(&mut write, anchor, standby, at())
                .await
                .expect("written");
            let row = sqlx::query("SELECT lapse_since, standby, notified_day FROM governor_state")
                .fetch_one(&mut *write)
                .await
                .expect("the row");
            assert_eq!(
                row.get::<Option<i64>, _>(0),
                anchor.map(StudyDay::epoch_day)
            );
            assert_eq!(row.get::<i64, _>(1), i64::from(standby));
            assert_eq!(row.get::<Option<i64>, _>(2), Some(19_995));
            let read = store::governor(&mut write).await.expect("read");
            assert_eq!(read.lapse_since, anchor);
            assert_eq!(read.standby, standby);
            assert_eq!(read.notified_day, Some(day(19_995)));
            cases += 1;
        }
    }
    sqlx::query("DELETE FROM governor_state")
        .execute(&mut *write)
        .await
        .expect("a delete");
    let none = store::governor(&mut write).await.expect("read");
    assert_eq!(
        (none.lapse_since, none.standby, none.notified_day),
        (None, false, None)
    );
    println!("governor population: {cases} writes");
    assert_eq!(cases, 4);
}

/// A rule (A33): the freeze events are inserted once each and read in the order written; a row
/// whose reason the code does not know is left out of the list.
#[tokio::test]
async fn the_freeze_events_are_stored_once_and_read_in_order() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    assert!(store::events(&mut write).await.expect("read").is_empty());
    let events = [
        FreezeEvent {
            day: day(19_999),
            delta: -1,
            reason: FreezeReason::Consumed,
        },
        FreezeEvent {
            day: day(19_999),
            delta: 0,
            reason: FreezeReason::StreakBreak,
        },
        FreezeEvent {
            day: day(20_000),
            delta: 2,
            reason: FreezeReason::StreakEarn,
        },
        FreezeEvent {
            day: day(20_000),
            delta: 1,
            reason: FreezeReason::Chest,
        },
    ];
    store::insert_events(&mut write, &events, at())
        .await
        .expect("written");
    store::insert_events(&mut write, &events[..3], at())
        .await
        .expect("again");
    assert_eq!(count(&mut write, "freeze_events").await, 4);
    let created: i64 = sqlx::query("SELECT MIN(created_at) FROM freeze_events")
        .fetch_one(&mut *write)
        .await
        .expect("a row")
        .get(0);
    assert_eq!(created, AT);
    assert_eq!(store::events(&mut write).await.expect("read"), events);
}

/// A rule (A33): the freezes the outside paid are the net of the chest, weekly quest, season and
/// shop rows alone; the fold's own rows do not count. The nets tried are -2, 0, 1 and 3.
#[tokio::test]
async fn the_outside_freezes_are_the_net_of_the_paid_reasons_only() {
    let mut nets = Vec::new();
    for rows in [
        vec![("shop", -1), ("chest", -1)],
        vec![],
        vec![("season", 1)],
        vec![("chest", 1), ("weekly_quest", 1), ("shop", 1)],
    ] {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let db = open(&directory).await;
        let mut write = db.write().await.expect("a write");
        let mut expected = 0;
        for (index, (reason, delta)) in rows.iter().enumerate() {
            sqlx::query("INSERT INTO freeze_events (study_day, delta, reason, created_at) VALUES (?1, ?2, ?3, 1)")
                .bind(19_000 + i64::try_from(index).expect("small"))
                .bind(delta)
                .bind(reason)
                .execute(&mut *write)
                .await
                .expect("a row");
            expected += delta;
        }
        for (reason, delta) in [("consumed", -1), ("streak_earn", 5), ("streak_break", 0)] {
            sqlx::query("INSERT INTO freeze_events (study_day, delta, reason, created_at) VALUES (18000, ?1, ?2, 1)")
                .bind(delta)
                .bind(reason)
                .execute(&mut *write)
                .await
                .expect("a fold row");
        }
        let net = store::external_freezes(&mut write).await.expect("read");
        assert_eq!(net, expected);
        nets.push(net);
    }
    println!("external-freeze population: nets {nets:?}");
    assert_eq!(nets, [-2, 0, 1, 3]);
}

/// A rule (A33): buying a freeze raises the language track's freezes by exactly one from every
/// count held (start state included) and writes one event of delta 1 with the reason given.
#[tokio::test]
async fn a_freeze_added_raises_the_held_count_by_one_and_is_recorded() {
    let mut cases = 0;
    for held in [None, Some(0_u32), Some(1), Some(2)] {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let db = open(&directory).await;
        let mut write = db.write().await.expect("a write");
        let before = StreakState {
            current: 5,
            longest: 8,
            freezes: held.unwrap_or(StreakState::start().freezes),
            last_study_day: Some(day(19_999)),
            comeback_armed: true,
        };
        if held.is_some() {
            store::upsert_state(&mut write, "language", &before, at())
                .await
                .expect("seeded");
        }
        store::add_freeze(&mut write, day(20_000), FreezeReason::Shop, at())
            .await
            .expect("added");
        let after = store::state(&mut write, "language")
            .await
            .expect("read")
            .expect("a row");
        assert_eq!(after.freezes, before.freezes + 1, "held {held:?}");
        if held.is_some() {
            assert_eq!(
                StreakState {
                    freezes: before.freezes,
                    ..after
                },
                before,
                "only the freezes change"
            );
        }
        assert_eq!(
            store::events(&mut write).await.expect("read"),
            [FreezeEvent {
                day: day(20_000),
                delta: 1,
                reason: FreezeReason::Shop
            }]
        );
        cases += 1;
    }
    println!("add-freeze population: {cases} starting counts");
    assert_eq!(cases, 4);
}

/// Every day in the relight's due list, read with a raw query the store does not own.
async fn due_days(connection: &mut SqliteConnection) -> Vec<(i64, i64)> {
    sqlx::query("SELECT study_day, created_at FROM relight_due ORDER BY study_day")
        .fetch_all(connection)
        .await
        .expect("the due rows")
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect()
}

/// A rule (A33, R27): the relight's due list keeps a day at its first write, is read oldest first,
/// loses a day once it is cleared, and holds no day a rolled-back write put in it. The three
/// functions are called only from coordination, so this crate's own tests read each one back.
#[tokio::test]
async fn the_relight_due_list_holds_each_day_once_until_it_is_cleared() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    let later = UtcMillis::from_epoch_millis(AT + 5);
    store::put_relight_due(&mut write, day(19_995), at())
        .await
        .expect("written");
    store::put_relight_due(&mut write, day(19_990), at())
        .await
        .expect("written");
    store::put_relight_due(&mut write, day(19_995), later)
        .await
        .expect("a day already due is left as it is");
    assert_eq!(due_days(&mut write).await, [(19_990, AT), (19_995, AT)]);
    sqlx::query("INSERT INTO relight_due (study_day, created_at) VALUES (19985, 1)")
        .execute(&mut *write)
        .await
        .expect("a raw row");
    assert_eq!(
        store::relight_due(&mut write).await.expect("read"),
        [day(19_985), day(19_990), day(19_995)]
    );
    store::clear_relight_due(&mut write, day(19_990))
        .await
        .expect("cleared");
    store::clear_relight_due(&mut write, day(20_000))
        .await
        .expect("a day never due clears nothing");
    assert_eq!(due_days(&mut write).await, [(19_985, 1), (19_995, AT)]);
    write.commit().await.expect("committed");

    let mut rolled = db.write().await.expect("a write");
    store::put_relight_due(&mut rolled, day(20_001), at())
        .await
        .expect("written");
    rolled.rollback().await.expect("rolled back");
    let mut read = db.write().await.expect("a write");
    assert_eq!(
        store::relight_due(&mut read).await.expect("read"),
        [day(19_985), day(19_995)]
    );
}
