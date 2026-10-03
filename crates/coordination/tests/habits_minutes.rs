//! The minutes log's use cases (SPEC-078 A3 to A6b, A30; R2 to R5, R18; ADR-078): an entry and its
//! undo each settle `read:` and `readgoal:` in their own write, the fold's habit step heals a settle
//! left undone, and a habit write that crosses a level is announced once. Every course, entry and
//! instant is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::habits::{
    Course, HabitError, HabitWriter, Logged, Undone, log_minutes, undo_entry, undo_newest,
};
use deck_streak_coordination::recompute::habits::HabitsStep;
use deck_streak_coordination::recompute::{Fold, FoldInput, Phase};
use deck_streak_ingest::reader::CollectionData;
use deck_streak_kernel::{
    Courses, Db, Hour, ManualClock, StudyDay, StudyDayRule, UtcMillis, UtcOffset,
};
use deck_streak_notifications::{BotTransport, Pass, Policy, PushFuture, Pushed, Router};
use deck_streak_progression::settle::settled_of_day;
use serde_json::Value;
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
const MINUTE_MS: i64 = 60_000;
/// A Monday: its study week runs to [`SUNDAY`].
const MONDAY: i64 = 20_101;
/// The Sunday that ends [`MONDAY`]'s week.
const SUNDAY: i64 = 20_107;

/// Two synthetic courses: `qaa` (alias `a`) and `qab` (alias `b`).
const COURSES: &str = r#"{
  "schema": "deckstreak.courses.v1",
  "courses": [
    {"code": "qaa", "name": "Course Qaa", "flag": "F", "deck_root": "Qaa", "alias": "a",
     "writing": false, "unit_bands": {}},
    {"code": "qab", "name": "Course Qab", "flag": "F", "deck_root": "Qab", "alias": "b",
     "writing": true, "unit_bands": {}}
  ],
  "focus_subjects": []
}"#;

fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

/// Noon UTC of study day `day` under the default rule.
const fn noon(day: i64) -> i64 {
    day * DAY_MS + 12 * HOUR_MS
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens")
}

fn writer<'a>(db: &'a Db, router: Option<&'a Router>, now: i64) -> HabitWriter<'a> {
    HabitWriter {
        db,
        router,
        rule: StudyDayRule::default(),
        now: UtcMillis::from_epoch_millis(now),
    }
}

/// Logs `minutes` for `course` at noon of `day`, and asserts it was logged.
async fn log(db: &Db, day: i64, course: Course<'_>, minutes: i64) -> Logged {
    let logged = log_minutes(
        &writer(db, None, noon(day)),
        &courses(),
        course,
        minutes,
        "",
    )
    .await;
    assert!(logged.is_ok(), "the entry is logged: {logged:?}");
    logged.expect("checked above")
}

/// Writes an entry into the log alone, with no settle, as a write that failed after it would.
async fn plant(db: &Db, code: &str, day: i64, minutes: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO minutes_log (code, study_day, minutes, note, created_at) \
         VALUES (?1, ?2, ?3, '', 1000)",
    )
    .bind(code)
    .bind(day)
    .bind(minutes)
    .execute(&mut *write)
    .await
    .expect("a planted entry");
    write.commit().await.expect("commit");
}

/// Every `read:` and `readgoal:` row settled from `first` to `last`, as day, source and amount.
async fn habit_rows(db: &Db, first: i64, last: i64) -> Vec<(i64, String, u32)> {
    let mut write = db.write().await.expect("a write");
    let mut rows = Vec::new();
    for day in first..=last {
        for row in settled_of_day(&mut write, StudyDay::from_epoch_day(day))
            .await
            .expect("the settled rows read")
        {
            if row.source.starts_with("read:") || row.source.starts_with("readgoal:") {
                assert_eq!(row.track, "language", "habit XP is on the language track");
                rows.push((day, row.source, row.amount));
            }
        }
    }
    rows.sort();
    rows
}

/// The rows of [`habit_rows`] that hold XP: a zero row and no row both settle nothing.
async fn amounts(db: &Db, first: i64, last: i64) -> BTreeMap<(i64, String), u32> {
    habit_rows(db, first, last)
        .await
        .into_iter()
        .filter(|(_, _, amount)| *amount > 0)
        .map(|(day, source, amount)| ((day, source), amount))
        .collect()
}

async fn entries(db: &Db) -> i64 {
    let mut write = db.write().await.expect("a write");
    sqlx::query_scalar("SELECT COUNT(*) FROM minutes_log")
        .fetch_one(&mut *write)
        .await
        .expect("the count")
}

/// The fold of habits' step alone.
fn fold() -> Fold {
    let mut fold = Fold::default();
    fold.register(Phase::DaySteps, Box::new(HabitsStep))
        .expect("the habit step is phase 4's");
    fold
}

fn no_reviews() -> CollectionData {
    CollectionData {
        reviews: Vec::new(),
        cards: Vec::new(),
        created_at: UtcMillis::from_epoch_millis(noon(MONDAY - 1_000)),
        deck_names: BTreeMap::new(),
    }
}

async fn recompute(fold: &Fold, db: &Db, data: &CollectionData, now: i64) {
    fold.run(
        db,
        &FoldInput {
            data,
            rule: StudyDayRule::default(),
            now: UtcMillis::from_epoch_millis(now),
            synced_in: Some(StudyDay::from_epoch_day(MONDAY)),
            courses_digest: Some("0123456789abcdef"),
            base_reviews: 0,
            offers: None,
        },
    )
    .await
    .expect("the fold runs");
}

/// A golden's cases, read whole, as input, output and class.
fn cases(name: &str) -> Vec<(Value, Value, Option<String>)> {
    let mut cases = Vec::new();
    let examined = golden::each_case(name, |case| {
        cases.push((case.input.clone(), case.output.clone(), case.class.clone()));
    });
    println!("examined {} case(s) of {name}", examined.count);
    assert!(examined.count > 0, "the golden holds cases");
    cases
}

/// A case's entries: course, study day and minutes.
fn case_entries(input: &Value) -> Vec<(String, i64, i64)> {
    input["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|entry| {
            (
                entry[0].as_str().expect("a code").to_owned(),
                entry[1].as_i64().expect("a day"),
                entry[2].as_i64().expect("minutes"),
            )
        })
        .collect()
}

#[tokio::test]
async fn minutes_only_logs_the_most_used_course() {
    for (input, output, _) in cases("habit_most_used") {
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        for (code, day, minutes) in case_entries(&input) {
            plant(&db, &code, day, minutes).await;
        }
        let today = input["today"].as_i64().expect("today");
        let logged = log_minutes(
            &writer(&db, None, noon(today)),
            &courses(),
            Course::MostUsed,
            5,
            "",
        )
        .await;
        match output.as_str() {
            Some(code) => assert!(
                matches!(&logged, Ok(logged) if logged.code.as_str() == code && logged.minutes == 5),
                "{input}: logged for {code}: {logged:?}"
            ),
            None => assert!(
                matches!(logged, Err(HabitError::NoCourse)),
                "{input}: refused for want of a course: {logged:?}"
            ),
        }
    }
}

#[tokio::test]
async fn the_minutes_xp_settles_as_the_predecessors_golden() {
    let mut exactly_210 = 0;
    for (input, output, class) in cases("habit_reading_settled") {
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        let entries = case_entries(&input);
        for (code, day, minutes) in &entries {
            log(&db, *day, Course::Token(code), *minutes).await;
        }
        let first = entries.iter().map(|entry| entry.1).min().expect("a day") - 7;
        let last = entries.iter().map(|entry| entry.1).max().expect("a day") + 7;
        let expected: Vec<(i64, String, u32)> = output
            .as_array()
            .expect("rows")
            .iter()
            .map(|row| {
                (
                    row[0].as_i64().expect("a day"),
                    row[1].as_str().expect("a source").to_owned(),
                    u32::try_from(row[2].as_u64().expect("an amount")).expect("fits"),
                )
            })
            .collect();
        assert_eq!(habit_rows(&db, first, last).await, expected, "{input}");
        if class.as_deref() == Some("a week of exactly 210") {
            exactly_210 += 1;
        }
    }
    assert_eq!(
        exactly_210, 1,
        "a week of exactly 210 minutes is in the golden"
    );
}

#[tokio::test]
async fn the_weekly_bonus_is_keyed_on_the_study_weeks_first_day() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    // Two hours east of UTC: local Monday 03:59 is still Sunday's study day, 04:00 is Monday's.
    let rule = StudyDayRule::new(
        Hour::new(4).expect("an hour"),
        UtcOffset::from_minutes(120).expect("an offset"),
    );
    let local_monday = MONDAY * DAY_MS - 120 * MINUTE_MS;
    for (instant, minutes) in [
        (local_monday + 4 * HOUR_MS - MINUTE_MS, 210),
        (local_monday + 4 * HOUR_MS, 211),
    ] {
        let writer = HabitWriter {
            db: &db,
            router: None,
            rule,
            now: UtcMillis::from_epoch_millis(instant),
        };
        let logged = log_minutes(&writer, &courses(), Course::Token("qaa"), minutes, "").await;
        assert!(logged.is_ok(), "{logged:?}");
    }
    assert_eq!(
        habit_rows(&db, MONDAY - 14, MONDAY + 14).await,
        [
            (MONDAY - 7, "readgoal:qaa".to_owned(), 150),
            (MONDAY - 1, "read:qaa".to_owned(), 240),
            (MONDAY, "read:qaa".to_owned(), 240),
            (MONDAY, "readgoal:qaa".to_owned(), 150),
        ],
        "03:59 counts in the week before, 04:00 in the new one"
    );
}

#[tokio::test]
async fn undo_restores_the_prior_settled_xp() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    log(&db, MONDAY, Course::Token("qaa"), 100).await;
    log(&db, MONDAY + 1, Course::Token("qaa"), 100).await;
    let prior = amounts(&db, MONDAY - 7, SUNDAY + 7).await;
    assert_eq!(
        prior.get(&(MONDAY, "readgoal:qaa".to_owned())),
        None,
        "200 minutes earn no bonus"
    );

    // The entry crosses the week's goal on a day that is closed when it is undone, a week later.
    let crossing = log(&db, MONDAY + 2, Course::Token("a"), 30).await;
    assert_eq!((crossing.week_minutes, crossing.goal_bonus), (230, 150));
    assert_eq!(
        amounts(&db, MONDAY, MONDAY)
            .await
            .get(&(MONDAY, "readgoal:qaa".to_owned())),
        Some(&150),
        "the entry's week earned its bonus"
    );
    let undone = undo_newest(&writer(&db, None, noon(SUNDAY + 2))).await;
    assert!(
        matches!(
            &undone,
            Ok(Undone::Removed { code, minutes: 30, day }) if code == "qaa"
                && *day == StudyDay::from_epoch_day(MONDAY + 2)
        ),
        "the newest entry is removed: {undone:?}"
    );
    assert_eq!(entries(&db).await, 2, "one entry was removed");
    assert_eq!(
        amounts(&db, MONDAY - 7, SUNDAY + 7).await,
        prior,
        "the closed day's XP and the earlier week's bonus are as they were"
    );
}

#[tokio::test]
async fn a_stale_undo_removes_nothing() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let older = log(&db, MONDAY, Course::Token("qab"), 40).await;
    let newer = log(&db, MONDAY, Course::Token("qab"), 20).await;
    let before = amounts(&db, MONDAY, MONDAY).await;
    let stale = undo_entry(&writer(&db, None, noon(MONDAY)), older.entry_id).await;
    assert!(matches!(stale, Ok(Undone::Stale)), "{stale:?}");
    assert_eq!(entries(&db).await, 2, "a stale button removes nothing");
    assert_eq!(
        amounts(&db, MONDAY, MONDAY).await,
        before,
        "and settles nothing"
    );
    let removed = undo_entry(&writer(&db, None, noon(MONDAY)), newer.entry_id).await;
    assert!(
        matches!(removed, Ok(Undone::Removed { minutes: 20, .. })),
        "the newest entry's button removes it: {removed:?}"
    );
    assert_eq!(entries(&db).await, 1);
    assert_eq!(
        amounts(&db, MONDAY, MONDAY)
            .await
            .get(&(MONDAY, "read:qab".to_owned())),
        Some(&80),
        "the day's XP is the remaining entry's"
    );
}

#[tokio::test]
async fn the_recompute_heals_a_habit_settle_left_undone() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    plant(&db, "qaa", MONDAY, 150).await;
    plant(&db, "qaa", MONDAY, 60).await;
    plant(&db, "qab", MONDAY, 30).await;
    assert!(
        habit_rows(&db, MONDAY, SUNDAY).await.is_empty(),
        "nothing settled yet"
    );
    recompute(&fold(), &db, &no_reviews(), noon(MONDAY)).await;
    assert_eq!(
        habit_rows(&db, MONDAY - 7, SUNDAY + 7).await,
        [
            (MONDAY, "read:qaa".to_owned(), 240),
            (MONDAY, "read:qab".to_owned(), 60),
            (MONDAY, "readgoal:qaa".to_owned(), 150),
        ],
        "the day's XP and its week's bonus are settled from the log"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn entries_and_a_recompute_serialise() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let fold = Arc::new(fold());
    let logging = {
        let db = db.clone();
        tokio::spawn(async move {
            let mut logged = 0;
            for _ in 0..20 {
                let result = log_minutes(
                    &writer(&db, None, noon(MONDAY)),
                    &courses(),
                    Course::Token("qaa"),
                    15,
                    "",
                )
                .await;
                if result.is_ok() {
                    logged += 1;
                }
            }
            logged
        })
    };
    let folding = {
        let db = db.clone();
        let fold = Arc::clone(&fold);
        tokio::spawn(async move {
            let data = no_reviews();
            for _ in 0..10 {
                recompute(&fold, &db, &data, noon(MONDAY)).await;
            }
        })
    };
    let logged = logging.await.expect("the entries ran");
    folding.await.expect("the recompute ran");
    assert_eq!(logged, 20, "every entry was logged");
    assert_eq!(entries(&db).await, 20);
    assert_eq!(
        habit_rows(&db, MONDAY, SUNDAY).await,
        [
            (MONDAY, "read:qaa".to_owned(), 240),
            (MONDAY, "readgoal:qaa".to_owned(), 150),
        ],
        "the settled XP is the rule over the final log: 300 minutes"
    );
}

/// A bot transport that records every push and delivers it.
#[derive(Default)]
struct Recording(Mutex<Vec<String>>);

impl BotTransport for Recording {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(text.to_owned());
            Pushed::Delivered
        })
    }
}

#[tokio::test]
async fn a_habit_write_that_raises_the_level_is_announced_once() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let bot = Arc::new(Recording::default());
    let router = Router::new(
        Arc::new(Policy::compiled().expect("the compiled policy parses")),
        db.clone(),
        Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(noon(MONDAY)))),
        StudyDayRule::default(),
    )
    .with_bot(bot.clone());
    let pushes = || bot.0.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let write = writer(&db, Some(&router), noon(MONDAY));

    // 50 minutes earn 100 XP: level 1 to level 2.
    let logged = log_minutes(&write, &courses(), Course::Token("qaa"), 50, "").await;
    assert!(logged.is_ok(), "{logged:?}");
    let first = pushes();
    assert_eq!(first.len(), 1, "the crossing is announced: {first:?}");
    assert!(first[0].contains("Level 2"), "{first:?}");
    let logged = log_minutes(&write, &courses(), Course::Token("qaa"), 10, "").await;
    assert!(logged.is_ok(), "{logged:?}");
    assert_eq!(pushes().len(), 1, "no crossing, no announcement");

    // Undone below the level and crossed again: the level's key is already spent.
    for _ in 0..2 {
        let undone = undo_newest(&write).await;
        assert!(matches!(undone, Ok(Undone::Removed { .. })), "{undone:?}");
    }
    let logged = log_minutes(&write, &courses(), Course::Token("qaa"), 50, "").await;
    assert!(logged.is_ok(), "{logged:?}");
    assert_eq!(pushes().len(), 1, "a level is announced once ever");
}

/// Every `read:` and `readgoal:` row settled from `first` to `last`, as day, source, amount and
/// whether its day was closed when it was settled.
async fn closed_rows(db: &Db, first: i64, last: i64) -> Vec<(i64, String, u32, bool)> {
    let mut write = db.write().await.expect("a write");
    let mut rows = Vec::new();
    for day in first..=last {
        for row in settled_of_day(&mut write, StudyDay::from_epoch_day(day))
            .await
            .expect("the settled rows read")
        {
            if row.source.starts_with("read:") || row.source.starts_with("readgoal:") {
                rows.push((day, row.source, row.amount, row.closed));
            }
        }
    }
    rows.sort();
    rows
}

#[tokio::test]
async fn an_entry_settles_its_own_day_open_and_an_earlier_week_start_closed() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    log(&db, MONDAY + 1, Course::Token("qaa"), 30).await;
    assert_eq!(
        closed_rows(&db, MONDAY - 7, SUNDAY + 7).await,
        [
            (MONDAY, "readgoal:qaa".to_owned(), 0, true),
            (MONDAY + 1, "read:qaa".to_owned(), 60, false),
        ],
        "Tuesday's entry: today's read: is open, Monday's readgoal: is closed"
    );
}

#[tokio::test]
async fn an_entry_on_its_weeks_first_day_settles_that_day_open() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    log(&db, MONDAY, Course::Token("qab"), 30).await;
    assert_eq!(
        closed_rows(&db, MONDAY - 7, SUNDAY + 7).await,
        [
            (MONDAY, "read:qab".to_owned(), 60, false),
            (MONDAY, "readgoal:qab".to_owned(), 0, false),
        ],
        "Monday's entry: the week's first day is today, so both rows are open"
    );
}

#[tokio::test]
async fn an_undo_on_a_later_day_settles_the_entrys_day_closed() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    log(&db, MONDAY + 1, Course::Token("qaa"), 30).await;
    let undone = undo_newest(&writer(&db, None, noon(MONDAY + 3))).await;
    assert!(
        matches!(undone, Ok(Undone::Removed { minutes: 30, .. })),
        "{undone:?}"
    );
    assert_eq!(
        closed_rows(&db, MONDAY - 7, SUNDAY + 7).await,
        [
            (MONDAY, "readgoal:qaa".to_owned(), 0, true),
            (MONDAY + 1, "read:qaa".to_owned(), 0, true),
        ],
        "undone on Thursday: Tuesday's read: and Monday's readgoal: are both closed"
    );
}

#[tokio::test]
async fn a_habit_writers_debug_shows_its_rule_and_instant_only() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    assert_eq!(
        format!("{:?}", writer(&db, None, noon(MONDAY))),
        "HabitWriter { router: false, rule: StudyDayRule { rollover_hour: Hour(4), \
         utc_offset: UtcOffset(0) }, now: UtcMillis(1736769600000), .. }",
        "the router's presence, the rule and the instant; the database is left out"
    );
}
