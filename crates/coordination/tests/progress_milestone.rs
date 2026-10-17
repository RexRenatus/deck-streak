//! The next milestone reads Road to C2's mature cards (SPEC-077 R17a; A21): the sum of `mature`
//! over the stored course progress of the configured courses, and `pending` before the first
//! recompute stores one of them. Every course here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

use deck_streak_coordination::progression::milestone_view::{
    MilestoneView, milestone_view, stored_mature,
};
use deck_streak_curriculum::progress::CourseProgress;
use deck_streak_curriculum::store::put_progress;
use deck_streak_kernel::{CourseCode, Courses, Db, UtcMillis};
use deck_streak_progression::milestone::next_milestone;
use support::{D0, at, scratch};

/// Two synthetic courses; a third code, `zz`, is configured nowhere.
const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[
{"code":"be","name":"Beta","flag":"b","deck_root":"Beta Course","alias":"b","writing":false,
"unit_bands":{"A1":[1,4]}},
{"code":"ga","name":"Gamma","flag":"g","deck_root":"Gamma Course","alias":"g","writing":false,
"unit_bands":{"A1":[1,4]}}]}"#;

/// The configured courses.
fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

/// Stores course `code`'s progress with `mature` mature cards, as a recompute would.
async fn store_course(db: &Db, code: &str, mature: u32) {
    let progress = CourseProgress {
        code: CourseCode::new(code).expect("a course code"),
        name: code.to_owned(),
        flag: code.to_owned(),
        total_cards: mature + 3,
        mature_cards: mature,
        mastery_pct: 50.0,
        current_band: "A1",
        bands: Vec::new(),
        current_unit: Some(1),
    };
    let mut write = db.write().await.expect("a write");
    put_progress(
        &mut write,
        &progress,
        UtcMillis::from_epoch_millis(at(D0, 12)),
    )
    .await
    .expect("the progress writes");
    write.commit().await.expect("the progress commits");
}

#[tokio::test]
async fn the_milestone_reads_the_courses_mature_cards() {
    let (reviews, streak) = (1_000, 30);
    let stored = scratch().await;
    store_course(&stored.db, "be", 70).await;
    store_course(&stored.db, "ga", 45).await;
    store_course(&stored.db, "zz", 900).await;
    let mature = stored_mature(&stored.db, &courses())
        .await
        .expect("the stored progress reads");
    assert_eq!(
        mature,
        Some(115),
        "the configured courses' stored mature cards, summed; an unconfigured course is not"
    );
    assert_eq!(
        milestone_view(reviews, streak, mature),
        MilestoneView::Next(next_milestone(reviews, streak, 115)),
        "the milestone reads that sum"
    );

    let fresh = scratch().await;
    let before = stored_mature(&fresh.db, &courses())
        .await
        .expect("the empty store reads");
    assert_eq!(before, None, "before the first recompute nothing is stored");
    assert_eq!(
        milestone_view(reviews, streak, before),
        MilestoneView::Pending,
        "the milestone stays pending, never computed from 0"
    );

    let unconfigured = scratch().await;
    store_course(&unconfigured.db, "zz", 900).await;
    assert_eq!(
        stored_mature(&unconfigured.db, &courses())
            .await
            .expect("the stored progress reads"),
        None,
        "a stored course that is configured nowhere stores no configured course"
    );

    let zero = scratch().await;
    store_course(&zero.db, "be", 0).await;
    assert_eq!(
        stored_mature(&zero.db, &courses())
            .await
            .expect("the stored progress reads"),
        Some(0),
        "a stored course with no mature card is 0, not pending"
    );
}
