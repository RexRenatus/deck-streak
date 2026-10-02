//! The badges view's pure rules (SPEC-073 R16, R18): every catalog badge either reads a stored
//! input or is listed without progress, each streak badge's threshold is progression's own
//! boundary, a locked badge carries its input against its threshold, and the earned badges are
//! ordered most recently awarded first. The two reads the surfaces share, the badges view and the
//! records view for today, are run over a migrated database holding synthetic rows.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_coordination::progression::badges_view::{
    EarnedLine, Input, PROGRESS_INPUTS, Progress, ProgressInputs, WITHOUT_PROGRESS, badges_view,
    locked, most_recent_first, progress,
};
use deck_streak_coordination::progression::records_view::{RecordLine, records_now};
use deck_streak_kernel::{Courses, Db, KernelError, StudyDay, UtcMillis};
use deck_streak_progression::badges::catalog::catalog;
use deck_streak_progression::badges::conditions::{
    BadgeContext, CENTURION_DAY_REVIEWS, FOREST_GUARDIAN_COUNT, LEGENDARY_DAY_SCORE,
    MATURITY_MILESTONE_COUNT, POLYGLOT_DECKS_DAY, conditions,
};
use deck_streak_progression::records::RecordKind;

/// The four streak badges, whose thresholds the view states as numbers.
const STREAK_KEYS: [&str; 4] = [
    "week_warrior",
    "monthly_monk",
    "century_flame",
    "year_of_iron",
];

/// `items`, after printing how many there are; refuses an empty population.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Whether progression's own condition holds `key` at a current streak of `days`.
fn holds(key: &str, days: i64) -> bool {
    let context = BadgeContext {
        streak_current: u64::try_from(days).unwrap_or(0),
        ..BadgeContext::default()
    };
    conditions(&context)
        .iter()
        .any(|(condition, met)| *condition == key && *met)
}

/// A threshold as the view's `i64`.
fn wide(threshold: u64) -> i64 {
    i64::try_from(threshold).expect("a threshold fits")
}

/// Synthetic stored inputs: a three-day streak, 42 reviews over two decks, a score of 77, and 64
/// mature cards recorded today.
const INPUTS: ProgressInputs = ProgressInputs {
    streak: 3,
    day_reviews: 42,
    day_decks: 2,
    day_score: 77,
    mature_cards: Some(64),
};

/// An earned badge awarded at `at`.
fn earned(key: &str, tier: u32, at: i64) -> EarnedLine {
    EarnedLine {
        key: key.to_owned(),
        tier,
        name: format!("Name of {key}"),
        emoji: "🏅".to_owned(),
        study_day: StudyDay::from_epoch_day(20_102),
        awarded_at: UtcMillis::from_epoch_millis(at),
    }
}

#[test]
fn every_catalog_badge_carries_progress_or_is_listed_without_it() {
    let keys = examined(
        "catalog badge(s)",
        catalog(&Courses::default())
            .into_iter()
            .map(|badge| badge.key)
            .collect(),
    );
    for key in &keys {
        let read = PROGRESS_INPUTS
            .iter()
            .filter(|(input, _)| input == key)
            .count();
        let listed = WITHOUT_PROGRESS
            .iter()
            .filter(|listed| *listed == key)
            .count();
        assert_eq!(read + listed, 1, "{key} is in exactly one of the two lists");
    }
    assert_eq!(PROGRESS_INPUTS.len() + WITHOUT_PROGRESS.len(), keys.len());
    let read: Vec<&str> = PROGRESS_INPUTS.iter().map(|(key, _)| *key).collect();
    assert_eq!(
        read,
        [
            "week_warrior",
            "monthly_monk",
            "century_flame",
            "year_of_iron",
            "centurion_day",
            "maturity_milestone",
            "forest_guardian",
            "polyglot",
            "legendary_day"
        ]
    );
}

#[test]
fn each_streak_threshold_is_progressions_own_boundary() {
    for key in examined("streak badge(s)", STREAK_KEYS.to_vec()) {
        let found = progress(key, &ProgressInputs::default());
        assert!(found.is_some(), "{key} carries progress");
        let threshold = found.map_or(0, |found| found.threshold);
        assert!(holds(key, threshold), "{key} holds at {threshold}");
        assert!(
            !holds(key, threshold - 1),
            "{key} does not hold at {threshold} - 1"
        );
    }
    let streaks: Vec<&str> = PROGRESS_INPUTS
        .iter()
        .filter(|(_, input)| *input == Input::Streak)
        .map(|(key, _)| *key)
        .collect();
    assert_eq!(streaks, STREAK_KEYS);
}

#[test]
fn a_locked_badge_carries_its_input_against_its_threshold() {
    let cases = [
        ("week_warrior", 3, 7),
        ("monthly_monk", 3, 30),
        ("century_flame", 3, 100),
        ("year_of_iron", 3, 365),
        ("centurion_day", 42, wide(CENTURION_DAY_REVIEWS)),
        ("polyglot", 2, wide(POLYGLOT_DECKS_DAY)),
        ("legendary_day", 77, LEGENDARY_DAY_SCORE),
        ("maturity_milestone", 64, MATURITY_MILESTONE_COUNT),
        ("forest_guardian", 64, FOREST_GUARDIAN_COUNT),
    ];
    for (key, value, threshold) in examined("progress case(s)", cases.to_vec()) {
        assert_eq!(
            progress(key, &INPUTS),
            Some(Progress { value, threshold }),
            "{key}"
        );
    }
    let unrecorded = ProgressInputs {
        mature_cards: None,
        ..INPUTS
    };
    assert_eq!(progress("maturity_milestone", &unrecorded), None);
    assert_eq!(progress("forest_guardian", &unrecorded), None);
    for key in [
        "grinder",
        "night_owl",
        "ink_week",
        "focus_week",
        "no_such_badge",
    ] {
        assert_eq!(progress(key, &INPUTS), None, "{key} has no stored input");
    }
}

#[test]
fn locked_lists_the_unearned_catalog_with_its_criteria_and_progress() {
    let earned = [earned("week_warrior", 0, 1_000)];
    let courses = Courses::default();
    let lines = locked(&courses, &earned, &INPUTS);

    assert_eq!(lines.len(), catalog(&courses).len() - 1);
    let lines = examined("locked badge(s)", lines);
    let first = lines.first().expect("a locked badge");
    assert_eq!(
        (
            first.key.as_str(),
            first.name.as_str(),
            first.emoji.as_str()
        ),
        ("first_steps", "First Steps", "👟")
    );
    assert_eq!(first.criteria, "Your first ever review");
    assert_eq!((first.family, first.progress), ("study", None));
    assert!(lines.iter().all(|line| line.key != "week_warrior"));
    let monk = lines
        .iter()
        .find(|line| line.key == "monthly_monk")
        .expect("monthly_monk is locked");
    assert_eq!(
        monk.progress,
        Some(Progress {
            value: 3,
            threshold: 30
        })
    );
    let families: Vec<(&str, &str)> = lines
        .iter()
        .filter(|line| ["first_page", "focus_initiate"].contains(&line.key.as_str()))
        .map(|line| (line.key.as_str(), line.family))
        .collect();
    assert_eq!(
        families,
        [("first_page", "habit"), ("focus_initiate", "focus")]
    );
    for line in lines.iter().filter(|line| line.family != "study") {
        assert_eq!(
            line.progress, None,
            "{} shows no fabricated progress",
            line.key
        );
    }
}

#[test]
fn the_earned_badges_are_ordered_most_recent_first() {
    let ordered = most_recent_first(vec![
        earned("grinder", 0, 1_000),
        earned("polyglot", 0, 3_000),
        earned("centurion_day", 0, 2_000),
        earned("legendary_day", 0, 3_000),
        earned("legendary_day", 1, 3_000),
    ]);
    let keys: Vec<(&str, u32)> = ordered
        .iter()
        .map(|line| (line.key.as_str(), line.tier))
        .collect();
    assert_eq!(
        keys,
        [
            ("legendary_day", 0),
            ("legendary_day", 1),
            ("polyglot", 0),
            ("centurion_day", 0),
            ("grinder", 0)
        ]
    );
}

/// The synthetic study day the database tests read: 2025-01-14, as an epoch day.
const TODAY: i64 = 20_102;

/// A migrated database in `scratch` holding today's rollup (42 study reviews over two decks in
/// 600 seconds, 64 mature cards, a score of 77) and a language streak of 3 days.
async fn seeded(scratch: &tempfile::TempDir) -> Db {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
         mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
         consistency, retention, workload, volume, mastery, score_at_close, settled_at, \
         fingerprint, created_at, updated_at) \
         VALUES (?1, 42, 0, 42, 0, 0, 600.0, 40, 36, 90.0, 1, 2, 14.0, 0, 0, 40, 36, 64, 60, 2, \
         5, 30, 'live:1736911800000', 77, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, \
         'synthetic', 1000, 1000)",
    )
    .bind(TODAY)
    .execute(&mut *write)
    .await
    .expect("the synthetic rollup is written");
    sqlx::query(
        "INSERT INTO streak_state (track, current_days, longest_days, freezes, last_study_day, \
         comeback_armed, created_at) VALUES ('language', 3, 9, 1, ?1, 0, 1000)",
    )
    .bind(TODAY)
    .execute(&mut *write)
    .await
    .expect("the synthetic streak is written");
    for (key, name, emoji, day, at) in [
        ("grinder", "Grinder", "\u{2699}", TODAY - 3, 2_000_i64),
        ("polyglot", "Polyglot", "\u{1f310}", TODAY - 1, 5_000),
    ] {
        sqlx::query(
            "INSERT INTO badges_earned (badge_key, tier, name, emoji, study_day, celebrated_at, \
             created_at) VALUES (?1, 0, ?2, ?3, ?4, ?5, ?5)",
        )
        .bind(key)
        .bind(name)
        .bind(emoji)
        .bind(day)
        .bind(at)
        .execute(&mut *write)
        .await
        .expect("the synthetic badge is written");
    }
    for (kind, value, day, previous) in [
        ("best_score", 120_i64, TODAY - 5, 100_i64),
        ("most_minutes", 8, TODAY - 3, 5),
    ] {
        sqlx::query(
            "INSERT INTO records (kind, value, study_day, previous, celebrated_at, created_at) \
             VALUES (?1, ?2, ?3, ?4, 1000, 1000)",
        )
        .bind(kind)
        .bind(value)
        .bind(day)
        .bind(previous)
        .execute(&mut *write)
        .await
        .expect("the synthetic record is written");
    }
    write.commit().await.expect("the commit");
    db
}

#[tokio::test]
async fn the_badges_view_reads_the_stored_badges_and_their_inputs() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = seeded(&scratch).await;
    let view = badges_view(&db, StudyDay::from_epoch_day(TODAY), &Courses::default())
        .await
        .expect("the badges view reads");

    // The earned badges are the stored rows, the later award first.
    assert_eq!(
        view.earned,
        [
            EarnedLine {
                key: "polyglot".to_owned(),
                tier: 0,
                name: "Polyglot".to_owned(),
                emoji: "\u{1f310}".to_owned(),
                study_day: StudyDay::from_epoch_day(TODAY - 1),
                awarded_at: UtcMillis::from_epoch_millis(5_000),
            },
            EarnedLine {
                key: "grinder".to_owned(),
                tier: 0,
                name: "Grinder".to_owned(),
                emoji: "\u{2699}".to_owned(),
                study_day: StudyDay::from_epoch_day(TODAY - 3),
                awarded_at: UtcMillis::from_epoch_millis(2_000),
            },
        ]
    );

    // A locked badge reads its stored input: the streak's 3 days, today's 42 reviews, 64 mature
    // cards. An earned badge is not locked.
    let locked_progress: Vec<(&str, Option<Progress>)> = view
        .locked
        .iter()
        .map(|line| (line.key.as_str(), line.progress))
        .collect();
    let seen = examined("locked badge(s) read from the database", locked_progress);
    let of = |key: &str| {
        seen.iter()
            .find(|(seen_key, _)| *seen_key == key)
            .map(|(_, progress)| *progress)
    };
    assert_eq!(
        of("monthly_monk"),
        Some(Some(Progress {
            value: 3,
            threshold: 30
        }))
    );
    assert_eq!(
        of("centurion_day"),
        Some(Some(Progress {
            value: 42,
            threshold: wide(CENTURION_DAY_REVIEWS)
        }))
    );
    assert_eq!(
        of("maturity_milestone"),
        Some(Some(Progress {
            value: 64,
            threshold: MATURITY_MILESTONE_COUNT
        }))
    );
    assert_eq!(of("polyglot"), None);
    assert_eq!(of("grinder"), None);
    db.close().await;
}

#[tokio::test]
async fn the_records_view_reads_each_stored_record_against_todays_rollup() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = seeded(&scratch).await;
    let view = records_now(&db, StudyDay::from_epoch_day(TODAY))
        .await
        .expect("the records view reads");

    // Today holds 77 points and ten whole minutes; the minutes record is reached, so the record to
    // chase is the best score, 43 points away.
    assert_eq!(
        view.lines,
        [
            RecordLine {
                kind: RecordKind::BestScore,
                label: RecordKind::BestScore.label(),
                value: 120,
                study_day: StudyDay::from_epoch_day(TODAY - 5),
                previous: 100,
                today: 77,
            },
            RecordLine {
                kind: RecordKind::MostMinutes,
                label: RecordKind::MostMinutes.label(),
                value: 8,
                study_day: StudyDay::from_epoch_day(TODAY - 3),
                previous: 5,
                today: 10,
            },
        ]
    );
    assert_eq!(view.chase, Some((RecordKind::BestScore, 43)));
    db.close().await;
}

/// A migrated database in `scratch` with nothing studied: no rollup, no streak, no award, no record.
async fn unstudied(scratch: &tempfile::TempDir) -> Db {
    Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens")
}

/// Renames `table` away, so a read of it alone refuses.
async fn rename_away(db: &Db, table: &str) {
    let rename = match table {
        "daily_rollup" => "ALTER TABLE daily_rollup RENAME TO gone_daily_rollup",
        "streak_state" => "ALTER TABLE streak_state RENAME TO gone_streak_state",
        other => panic!("no rename is written for {other}"),
    };
    let mut write = db.write().await.expect("a write");
    sqlx::query(rename)
        .execute(&mut *write)
        .await
        .expect("the table is renamed away");
    write.commit().await.expect("the commit");
}

#[tokio::test]
async fn an_unstudied_database_shows_every_locked_badge_at_zero_progress() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = unstudied(&scratch).await;
    let view = badges_view(&db, StudyDay::from_epoch_day(TODAY), &Courses::default())
        .await
        .expect("the badges view reads");
    let seen = examined(
        "locked badge(s) of an unstudied database",
        view.locked
            .iter()
            .map(|line| (line.key.as_str(), line.progress))
            .collect::<Vec<_>>(),
    );
    let of = |key: &str| {
        seen.iter()
            .find(|(seen_key, _)| *seen_key == key)
            .map(|(_, progress)| *progress)
    };
    for (key, threshold) in [
        ("week_warrior", 7),
        ("centurion_day", wide(CENTURION_DAY_REVIEWS)),
        ("polyglot", wide(POLYGLOT_DECKS_DAY)),
        ("legendary_day", LEGENDARY_DAY_SCORE),
    ] {
        assert_eq!(
            of(key),
            Some(Some(Progress {
                value: 0,
                threshold
            })),
            "{key}"
        );
    }
    db.close().await;
}

#[tokio::test]
async fn a_badges_view_whose_rollup_read_alone_refuses_is_an_error() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = unstudied(&scratch).await;
    rename_away(&db, "daily_rollup").await;
    let view = badges_view(&db, StudyDay::from_epoch_day(TODAY), &Courses::default()).await;
    assert!(matches!(view, Err(KernelError::Database(_))), "{view:?}");
    db.close().await;
}

#[tokio::test]
async fn a_badges_view_whose_streak_read_alone_refuses_is_an_error() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = unstudied(&scratch).await;
    rename_away(&db, "streak_state").await;
    let view = badges_view(&db, StudyDay::from_epoch_day(TODAY), &Courses::default()).await;
    assert!(matches!(view, Err(KernelError::Database(_))), "{view:?}");
    db.close().await;
}

#[tokio::test]
async fn records_with_no_rollup_for_today_read_today_as_zero() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = unstudied(&scratch).await;
    let mut write = db.write().await.expect("a write");
    for (kind, value, day, previous) in [
        ("best_score", 120_i64, TODAY - 5, 100_i64),
        ("most_reviews", 300, TODAY - 4, 250),
        ("most_minutes", 8, TODAY - 3, 5),
    ] {
        sqlx::query(
            "INSERT INTO records (kind, value, study_day, previous, celebrated_at, created_at) \
             VALUES (?1, ?2, ?3, ?4, 1000, 1000)",
        )
        .bind(kind)
        .bind(value)
        .bind(day)
        .bind(previous)
        .execute(&mut *write)
        .await
        .expect("the synthetic record is written");
    }
    write.commit().await.expect("the commit");
    let view = records_now(&db, StudyDay::from_epoch_day(TODAY))
        .await
        .expect("the records view reads");
    let today_and_value: Vec<(RecordKind, i64, i64)> = view
        .lines
        .iter()
        .map(|line| (line.kind, line.today, line.value))
        .collect();
    assert_eq!(
        examined("record line(s) with no rollup", today_and_value),
        [
            (RecordKind::BestScore, 0, 120),
            (RecordKind::MostReviews, 0, 300),
            (RecordKind::MostMinutes, 0, 8),
        ]
    );
    // Nothing studied: the record to chase is the nearest, and its distance is its whole value.
    assert_eq!(view.chase, Some((RecordKind::MostMinutes, 8)));
    db.close().await;
}

#[tokio::test]
async fn a_records_view_whose_rollup_read_alone_refuses_is_an_error() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = unstudied(&scratch).await;
    rename_away(&db, "daily_rollup").await;
    let view = records_now(&db, StudyDay::from_epoch_day(TODAY)).await;
    assert!(matches!(view, Err(KernelError::Database(_))), "{view:?}");
    db.close().await;
}
