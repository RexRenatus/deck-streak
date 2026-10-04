//! The writing log's use cases and step (SPEC-078 A10, A10c, A10d; R6 to R8, R18; ADR-078): a
//! chip's toggle settles `write:<code>` and `write:all` in its own write and a second toggle
//! restores the day, a chip drawn for a closed day toggles nothing, and the fold's writing step
//! settles each evaluated day's writing from its log. Every course, confirmation and instant is
//! synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use deck_streak_coordination::habits::writing::toggle;
use deck_streak_coordination::habits::{Checklist, ChecklistLine, HabitWriter, Written};
use deck_streak_coordination::recompute::habits::HabitsStep;
use deck_streak_coordination::recompute::writing::WritingStep;
use deck_streak_coordination::recompute::{Fold, FoldInput, Phase};
use deck_streak_ingest::reader::CollectionData;
use deck_streak_kernel::{CourseCode, Courses, Db, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_progression::settle::settled_of_day;
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// A Monday.
const MONDAY: i64 = 20_101;
/// The Tuesday after it.
const TUESDAY: i64 = 20_102;

/// Two synthetic courses: `qaa` reads only, `qab` (alias `b`) is the one writing course.
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

fn code(text: &str) -> CourseCode {
    CourseCode::new(text).expect("a synthetic code")
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

fn writer(db: &Db, now: i64) -> HabitWriter<'_> {
    HabitWriter {
        db,
        router: None,
        rule: StudyDayRule::default(),
        now: UtcMillis::from_epoch_millis(now),
    }
}

/// Every `write:` row settled on `day`, as source and amount, by source.
async fn write_rows(db: &Db, day: i64) -> Vec<(String, u32)> {
    let mut write = db.write().await.expect("a write");
    let mut rows: Vec<(String, u32)> = settled_of_day(&mut write, StudyDay::from_epoch_day(day))
        .await
        .expect("the settled rows read")
        .into_iter()
        .filter(|row| row.source.starts_with("write:"))
        .map(|row| {
            assert_eq!(row.track, "language", "writing XP is on the language track");
            (row.source, row.amount)
        })
        .collect();
    rows.sort();
    rows
}

/// The rows of [`write_rows`] that hold XP: a zero row and no row both settle nothing.
async fn amounts(db: &Db, day: i64) -> BTreeMap<String, u32> {
    write_rows(db, day)
        .await
        .into_iter()
        .filter(|(_, amount)| *amount > 0)
        .collect()
}

/// The writing log's rows, as code and day, in order.
async fn confirmations(db: &Db) -> Vec<(String, i64)> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT code, study_day FROM writing_log ORDER BY study_day, code",
    )
    .fetch_all(db.reader())
    .await
    .expect("the log reads")
}

/// Writes a confirmation into the log alone, with no settle, as a write that failed after it would.
async fn plant_confirmation(db: &Db, code: &str, day: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query("INSERT INTO writing_log (code, study_day, created_at) VALUES (?1, ?2, 1000)")
        .bind(code)
        .bind(day)
        .execute(&mut *write)
        .await
        .expect("a planted confirmation");
    write.commit().await.expect("commit");
}

/// Writes an open settled row of `source` on `day`, as an earlier settle left it.
async fn plant_settled(db: &Db, day: i64, source: &str, amount: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
         VALUES (?1, ?2, 'language', ?3, 0, 1000)",
    )
    .bind(day)
    .bind(source)
    .bind(amount)
    .execute(&mut *write)
    .await
    .expect("a planted settled row");
    write.commit().await.expect("commit");
}

/// The day's writing XP when `qab`, the only writing course, is confirmed.
fn confirmed_day() -> BTreeMap<String, u32> {
    BTreeMap::from([("write:all".to_owned(), 100), ("write:qab".to_owned(), 75)])
}

/// The checklist of `day` with `qab` confirmed or not, and the streak it makes.
fn qab_checklist(day: i64, confirmed: bool) -> Checklist {
    let streak = u32::from(confirmed);
    Checklist {
        day: StudyDay::from_epoch_day(day),
        lines: vec![ChecklistLine {
            code: code("qab"),
            confirmed,
            streak,
        }],
        streak,
    }
}

/// The fold of habits' two day steps, the reading step then the writing step.
fn fold() -> Fold {
    let mut fold = Fold::default();
    fold.register(Phase::DaySteps, Box::new(HabitsStep))
        .expect("the habit step is phase 4's");
    fold.register(Phase::DaySteps, Box::new(WritingStep::new(courses())))
        .expect("the writing step is phase 4's");
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

async fn recompute(db: &Db, now: i64) {
    fold()
        .run(
            db,
            &FoldInput {
                data: &no_reviews(),
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

#[tokio::test]
async fn toggling_twice_restores_the_days_writing_xp() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let monday = StudyDay::from_epoch_day(MONDAY);
    let before = amounts(&db, MONDAY).await;
    assert!(before.is_empty(), "no writing XP yet");

    let first = toggle(&writer(&db, noon(MONDAY)), &courses(), code("qab"), monday).await;
    assert_eq!(
        amounts(&db, MONDAY).await,
        confirmed_day(),
        "one toggle settles the course's 75 and the day's bonus of 100: {first:?}"
    );
    assert_eq!(confirmations(&db).await, [("qab".to_owned(), MONDAY)]);
    assert!(
        matches!(&first, Ok(Written::Done(checklist)) if *checklist == qab_checklist(MONDAY, true)),
        "the toggle answers with the day's checklist: {first:?}"
    );

    let second = toggle(&writer(&db, noon(MONDAY)), &courses(), code("qab"), monday).await;
    assert_eq!(
        amounts(&db, MONDAY).await,
        before,
        "a second toggle restores the day's writing XP: {second:?}"
    );
    assert!(
        confirmations(&db).await.is_empty(),
        "the confirmation is cleared"
    );
    assert!(
        matches!(&second, Ok(Written::Done(checklist)) if *checklist == qab_checklist(MONDAY, false)),
        "{second:?}"
    );
}

#[tokio::test]
async fn a_chip_for_a_closed_day_toggles_nothing() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    // The chip was drawn on Monday, and the owner taps it on Tuesday.
    let tapped = toggle(
        &writer(&db, noon(TUESDAY)),
        &courses(),
        code("qab"),
        StudyDay::from_epoch_day(MONDAY),
    )
    .await;
    assert!(
        matches!(&tapped, Ok(Written::DayClosed(checklist)) if *checklist == qab_checklist(TUESDAY, false)),
        "a Monday chip tapped on Tuesday answers with Tuesday's checklist and toggles nothing: \
         {tapped:?}"
    );
    assert!(confirmations(&db).await.is_empty(), "nothing was confirmed");
    assert!(amounts(&db, MONDAY).await.is_empty(), "Monday is untouched");
    assert!(
        amounts(&db, TUESDAY).await.is_empty(),
        "Tuesday is untouched"
    );

    // A chip drawn on Tuesday toggles Tuesday.
    let today = toggle(
        &writer(&db, noon(TUESDAY)),
        &courses(),
        code("qab"),
        StudyDay::from_epoch_day(TUESDAY),
    )
    .await;
    assert!(
        matches!(&today, Ok(Written::Done(checklist)) if *checklist == qab_checklist(TUESDAY, true)),
        "{today:?}"
    );
    assert_eq!(amounts(&db, TUESDAY).await, confirmed_day());
}

#[tokio::test]
async fn the_writing_step_settles_each_days_writing_from_its_log() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    // A confirmation whose settle was left undone, and a row of a course that no longer writes.
    plant_confirmation(&db, "qab", MONDAY).await;
    plant_settled(&db, MONDAY, "write:qaa", 75).await;
    recompute(&db, noon(MONDAY)).await;
    assert_eq!(
        write_rows(&db, MONDAY).await,
        [
            ("write:all".to_owned(), 100),
            ("write:qaa".to_owned(), 0),
            ("write:qab".to_owned(), 75),
        ],
        "the day's writing is settled from its log, and a held row the log does not pay is zeroed"
    );

    // The confirmation cleared from the log alone: the next recompute settles the day to nothing.
    let mut write = db.write().await.expect("a write");
    sqlx::query("DELETE FROM writing_log")
        .execute(&mut *write)
        .await
        .expect("the log clears");
    write.commit().await.expect("commit");
    recompute(&db, noon(MONDAY)).await;
    assert!(
        amounts(&db, MONDAY).await.is_empty(),
        "a day with no confirmation holds no writing XP"
    );
}
