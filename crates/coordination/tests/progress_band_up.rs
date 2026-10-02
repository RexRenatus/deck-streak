//! The band-up's record and grant (SPEC-077 R6, R7; A8): the progress step, run in phase 4 for the
//! current study day, records a course seen for the first time as a silent baseline, and pays no XP,
//! awards no badge and owes no celebration for it, through phase 7's band badge step as well. Every
//! card, deck and course here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

use std::collections::BTreeMap;

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::recompute::Evaluation;
use deck_streak_coordination::recompute::band_badges::BandBadgesStep;
use deck_streak_coordination::recompute::progress::ProgressStep;
use deck_streak_ingest::reader::{Card, CollectionData};
use deck_streak_kernel::{CourseCode, Courses, Db, UtcMillis};
use support::{D0, at, badges, card, run_step, scratch};

/// One synthetic course whose first three bands each hold one unit's deck.
const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[{"code":"be","name":"Beta",
"flag":"b","deck_root":"Beta Course","alias":"b","writing":false,
"unit_bands":{"A1":[1,4],"A2":[5,9],"B1":[10,14]}}]}"#;

/// The owner's courses.
fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

/// A window with one mature review card of course `be` in each of units 3, 7 and 12: every card's
/// mastery is 1, so A1, A2 and B1 are achieved and the course's current band is B1.
fn window() -> CollectionData {
    let decks = [(1, "Unit 03"), (2, "Unit 07"), (3, "Unit 12")];
    let cards: Vec<Card> = decks
        .iter()
        .map(|(deck, _)| Card {
            course: CourseCode::new("be"),
            ..card(*deck, *deck)
        })
        .collect();
    let deck_names: BTreeMap<i64, String> = decks
        .iter()
        .map(|(deck, unit)| (*deck, format!("Beta Course\u{1f}{unit}")))
        .collect();
    CollectionData {
        reviews: Vec::new(),
        cards,
        created_at: UtcMillis::from_epoch_millis(at(D0 - 1_000, 12)),
        deck_names,
    }
}

/// Every milestone stored, as `(course, band, study day, baseline, marked)`, by course and band.
async fn milestones(db: &Db) -> Vec<(String, String, i64, i64, bool)> {
    sqlx::query_as(
        "SELECT course, band, study_day, baseline, celebrated_at IS NOT NULL \
         FROM band_milestones ORDER BY course, band",
    )
    .fetch_all(db.reader())
    .await
    .expect("the milestones")
}

/// Every course's stored current band, as `(course, band)`, by course.
async fn stored_bands(db: &Db) -> Vec<(String, String)> {
    sqlx::query_as("SELECT course, current_band FROM language_progress ORDER BY course")
        .fetch_all(db.reader())
        .await
        .expect("the stored progress")
}

/// How many XP rows the ledger holds.
async fn xp_rows(db: &Db) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM xp_ledger")
        .fetch_one(db.reader())
        .await
        .expect("the XP ledger's count")
}

/// One recompute of the current study day at `now`: phase 4's progress step, then phase 7's band
/// badge step, each in its own write as the fold runs them.
async fn recompute(db: &Db, data: &CollectionData, now: i64) {
    let courses = courses();
    let progress = ProgressStep::new(courses.clone(), AnalyticsSettings::default());
    run_step(db, &progress, data, 0, (D0, Evaluation::Current), now).await;
    let band_badges = BandBadgesStep::new(courses);
    run_step(db, &band_badges, data, 0, (D0, Evaluation::Current), now).await;
}

#[tokio::test]
async fn the_first_sighting_of_a_course_is_a_silent_baseline() {
    let scratch = scratch().await;
    let db = &scratch.db;
    let data = window();

    recompute(db, &data, at(D0, 12)).await;

    assert_eq!(
        milestones(db).await,
        [("be".to_owned(), "B1".to_owned(), D0, 1, true)],
        "the first sighting records the current band as a silent baseline, owing nothing"
    );
    assert_eq!(
        stored_bands(db).await,
        [("be".to_owned(), "B1".to_owned())],
        "the course's progress is stored with its current band"
    );
    assert_eq!(xp_rows(db).await, 0, "a first sighting pays no XP");
    assert_eq!(badges(db).await, [], "a first sighting awards no badge");

    // A second recompute of the same day sees the stored band: nothing is recorded or paid.
    recompute(db, &data, at(D0, 18)).await;
    assert_eq!(
        milestones(db).await,
        [("be".to_owned(), "B1".to_owned(), D0, 1, true)],
        "the baseline is recorded once"
    );
    assert_eq!(xp_rows(db).await, 0, "the unchanged band pays no XP");
    assert_eq!(badges(db).await, [], "the unchanged band awards no badge");
}
