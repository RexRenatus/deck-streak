//! The progress and band badge steps hold at their boundaries (SPEC-077 R6, R7, R10).
//!
//! Each test names the mutants it kills; they reach the crate only through the recompute steps'
//! public items, over a temporary database. Every card, deck and course here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

use std::collections::BTreeMap;

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::recompute::band_badges::BandBadgesStep;
use deck_streak_coordination::recompute::progress::{
    ProgressStep, band_up_key, band_up_line, offer_band_ups,
};
use deck_streak_coordination::recompute::{DayStep, Evaluation};
use deck_streak_curriculum::law::LawDues;
use deck_streak_curriculum::progress::CourseProgress;
use deck_streak_curriculum::store::{self, NewMilestone};
use deck_streak_ingest::calendar::collection_day_number;
use deck_streak_ingest::reader::{Card, CollectionData};
use deck_streak_kernel::{CourseCode, Courses, Db, StudyDayRule, Track, UtcMillis};
use support::{D0, Recorder, at, card, day, run_step, scratch};

/// One synthetic course.
const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[{"code":"be","name":"Beta",
"flag":"b","deck_root":"Beta Course","alias":"b","writing":false,
"unit_bands":{"A1":[1,4],"A2":[5,9],"B1":[10,14]}}]}"#;

fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

#[test]
fn the_steps_register_under_their_exact_names() {
    let progress = ProgressStep::new(courses(), AnalyticsSettings::default());
    assert_eq!(progress.name(), "curriculum.progress");
    let badges = BandBadgesStep::new(courses());
    assert_eq!(badges.name(), "curriculum.band_badges");
}

#[test]
fn a_band_ups_line_and_key_are_spelled_exactly() {
    assert_eq!(band_up_line("b", "Beta", "A2"), "b Beta reached A2");
    assert_eq!(band_up_key("be", "A2"), "bandup:be:a2");
}

/// A window of law and language cards, none of a course, over the collection created a day before
/// the study day.
fn window(cards: Vec<Card>) -> (CollectionData, i64) {
    let created_at = UtcMillis::from_epoch_millis(at(D0 - 1_000, 12));
    let number = collection_day_number(StudyDayRule::default(), created_at, day(D0));
    let data = CollectionData {
        reviews: Vec::new(),
        cards,
        created_at,
        deck_names: BTreeMap::new(),
    };
    (data, number)
}

#[tokio::test]
async fn the_law_dues_count_only_the_law_tracks_cards() {
    let scratch = scratch().await;
    let db = &scratch.db;
    let (_, number) = window(Vec::new());
    let mut cards = Vec::new();
    // Three law cards overdue, two due today and one due tomorrow; two language cards overdue and
    // one due today, which are never the law's.
    for (id, due, track) in [
        (1, number - 2, Track::Law),
        (2, number - 1, Track::Law),
        (3, number - 5, Track::Law),
        (4, number, Track::Law),
        (5, number, Track::Law),
        (6, number + 1, Track::Law),
        (7, number - 1, Track::Language),
        (8, number - 3, Track::Language),
        (9, number, Track::Language),
    ] {
        cards.push(Card {
            due,
            track,
            ..card(id, id)
        });
    }
    let (data, _) = window(cards);
    let progress = ProgressStep::new(courses(), AnalyticsSettings::default());
    run_step(
        db,
        &progress,
        &data,
        0,
        (D0, Evaluation::Current),
        at(D0, 12),
    )
    .await;

    let mut read = db.reader().acquire().await.expect("a read");
    assert_eq!(
        store::law_dues(&mut read).await.expect("the dues"),
        Some(LawDues {
            study_day: day(D0),
            backlog: 3,
            due_today: 2,
        }),
        "the dues count the law cards overdue and due today, and no language card"
    );
}

/// Seeds `code`'s progress as a course named `name` flagged `flag`.
async fn seed_course(db: &Db, code: &str, name: &str, flag: &str) {
    let progress = CourseProgress {
        code: CourseCode::new(code).expect("a course code"),
        name: name.to_owned(),
        flag: flag.to_owned(),
        total_cards: 4,
        mature_cards: 2,
        mastery_pct: 50.0,
        current_band: "A2",
        bands: Vec::new(),
        current_unit: None,
    };
    let mut write = db.write().await.expect("a write");
    store::put_progress(&mut write, &progress, UtcMillis::from_epoch_millis(1_000))
        .await
        .expect("the progress writes");
    write.commit().await.expect("the write commits");
}

/// Records the owed band-up of `code` to `band`.
async fn owe(db: &Db, code: &str, band: &str, at: i64) {
    let course = CourseCode::new(code).expect("a course code");
    let milestone = NewMilestone {
        course: &course,
        band,
        study_day: day(D0),
        baseline: false,
        at: UtcMillis::from_epoch_millis(at),
    };
    let mut write = db.write().await.expect("a write");
    let _ = store::record_milestone(&mut write, &milestone)
        .await
        .expect("the milestone records");
    write.commit().await.expect("the write commits");
}

#[tokio::test]
async fn an_owed_band_up_is_offered_under_its_own_courses_name() {
    let scratch = scratch().await;
    let db = &scratch.db;
    seed_course(db, "al", "Alpha", "a").await;
    seed_course(db, "be", "Beta", "b").await;
    owe(db, "be", "A2", 5_000).await;
    // A course erased since its band-up is named by its code.
    owe(db, "zz", "B1", 6_000).await;

    let router = Recorder::default();
    offer_band_ups(&router, db, UtcMillis::from_epoch_millis(9_000), day(D0))
        .await
        .expect("the offers");
    let mut handed: Vec<(String, String, String)> = router
        .handed()
        .into_iter()
        .map(|celebration| {
            (
                celebration.event.to_owned(),
                celebration.key,
                celebration.text,
            )
        })
        .collect();
    handed.sort();
    assert_eq!(
        handed,
        [
            (
                "band_up".to_owned(),
                "bandup:be:a2".to_owned(),
                "b Beta reached A2".to_owned()
            ),
            (
                "band_up".to_owned(),
                "bandup:zz:b1".to_owned(),
                "zz reached B1".to_owned()
            ),
        ],
        "each band-up carries its own course's flag and name, or its code when erased"
    );
}
