//! Road to C2's progress view (SPEC-077 R15, T5, T28): the stored progress of the configured
//! courses alone, ordered by name, and nothing before the first recompute stores one. The api's
//! route and the bot's command render this view, so its filter and its order are held here, in its
//! own crate. Every course here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

use deck_streak_coordination::progress_view::{StoredBand, StoredProgress, progress_view};
use deck_streak_curriculum::progress::{BandProgress, CourseProgress};
use deck_streak_curriculum::store::put_progress;
use deck_streak_kernel::{CourseCode, Courses, Db, UtcMillis};
use support::{D0, at, scratch};

/// Two synthetic courses whose names sort the other way round from their codes: `be` is Zeta and
/// `ga` is Alpha. A third code, `zz`, is configured nowhere.
const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[
{"code":"be","name":"Zeta","flag":"z","deck_root":"Zeta Course","alias":"z","writing":false,
"unit_bands":{"A1":[1,4]}},
{"code":"ga","name":"Alpha","flag":"a","deck_root":"Alpha Course","alias":"a","writing":false,
"unit_bands":{"A1":[1,4]}}]}"#;

/// The configured courses.
fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

/// The instant every row here is stored at.
fn stored_at() -> UtcMillis {
    UtcMillis::from_epoch_millis(at(D0, 12))
}

/// Stores course `code`'s progress under `name`, with `mature` mature cards, as a recompute would.
async fn store_course(db: &Db, code: &str, name: &str, mature: u32) {
    let progress = CourseProgress {
        code: CourseCode::new(code).expect("a course code"),
        name: name.to_owned(),
        flag: format!("{code}-flag"),
        total_cards: mature + 3,
        mature_cards: mature,
        mastery_pct: 62.5,
        current_band: "A1",
        bands: vec![BandProgress {
            band: "A1",
            total: mature + 3,
            mature,
            pct: 62.5,
            achieved: false,
        }],
        current_unit: Some(2),
    };
    let mut write = db.write().await.expect("a write");
    put_progress(&mut write, &progress, stored_at())
        .await
        .expect("the progress writes");
    write.commit().await.expect("the progress commits");
}

/// Course `code`'s row as the view answers it, stored by [`store_course`].
fn expected(code: &str, name: &str, mature: u32) -> StoredProgress {
    StoredProgress {
        course: code.to_owned(),
        name: name.to_owned(),
        flag: format!("{code}-flag"),
        mastery_pct: 62.5,
        current_band: "A1".to_owned(),
        mature_cards: mature,
        total_cards: mature + 3,
        current_unit: Some(2),
        bands: vec![StoredBand {
            band: "A1".to_owned(),
            total: mature + 3,
            mature,
            pct: 62.5,
            achieved: false,
        }],
        updated_at: stored_at(),
    }
}

#[tokio::test]
async fn the_view_is_empty_before_the_first_recompute() {
    let fresh = scratch().await;
    let view = progress_view(&fresh.db, &courses())
        .await
        .expect("the empty store reads");
    assert_eq!(view, Vec::new(), "nothing is stored, so nothing is shown");
}

#[tokio::test]
async fn the_view_holds_the_configured_courses_by_name_and_no_stale_row() {
    let stored = scratch().await;
    store_course(&stored.db, "be", "Zeta", 70).await;
    store_course(&stored.db, "ga", "Alpha", 45).await;
    store_course(&stored.db, "zz", "Stale", 900).await;
    let view = progress_view(&stored.db, &courses())
        .await
        .expect("the stored progress reads");
    assert_eq!(
        view,
        vec![expected("ga", "Alpha", 45), expected("be", "Zeta", 70)],
        "the configured courses' stored rows, whole, Alpha before Zeta although be sorts before ga; \
         the stale row of zz is absent"
    );

    let unconfigured = scratch().await;
    store_course(&unconfigured.db, "zz", "Stale", 900).await;
    assert_eq!(
        progress_view(&unconfigured.db, &courses())
            .await
            .expect("the stored progress reads"),
        Vec::new(),
        "a stored course that is configured nowhere is never shown"
    );
}
