//! The curriculum store reads back what it writes, and every read and write does its work
//! (SPEC-077 R6, R7, R10, R18).
//!
//! Each test names the mutants it kills; they reach the crate only through `store`'s public items,
//! over a file-backed database per test, as `progress_store.rs` does. Every row is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use deck_streak_curriculum::law::LawDues;
use deck_streak_curriculum::progress::{BandProgress, CourseProgress};
use deck_streak_curriculum::store::{
    self, BandMilestone, NewMilestone, StoredBand, StoredProgress,
};
use deck_streak_kernel::{CourseCode, Db, StudyDay, UtcMillis};

/// A database in `directory`, opened and migrated.
async fn open(directory: &tempfile::TempDir) -> Db {
    Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens")
}

/// A course's progress with two bands.
fn alpha(mature: u32, unit: Option<u32>) -> CourseProgress {
    CourseProgress {
        code: CourseCode::new("al").expect("a course code"),
        name: "Alpha".to_owned(),
        flag: "a".to_owned(),
        total_cards: 20,
        mature_cards: mature,
        mastery_pct: 61.5,
        current_band: "A2",
        bands: vec![
            BandProgress {
                band: "A1",
                total: 8,
                mature: 7,
                pct: 90.0,
                achieved: true,
            },
            BandProgress {
                band: "A2",
                total: 12,
                mature: 5,
                pct: 43.25,
                achieved: false,
            },
        ],
        current_unit: unit,
    }
}

/// A second course's progress.
fn beta() -> CourseProgress {
    CourseProgress {
        code: CourseCode::new("be").expect("a course code"),
        name: "Beta".to_owned(),
        flag: "b".to_owned(),
        total_cards: 9,
        mature_cards: 3,
        mastery_pct: 12.25,
        current_band: "A1",
        bands: Vec::new(),
        current_unit: None,
    }
}

#[tokio::test]
async fn stored_progress_reads_back_what_was_written() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    store::put_progress(
        &mut write,
        &alpha(12, Some(7)),
        UtcMillis::from_epoch_millis(2_000),
    )
    .await
    .expect("alpha writes");
    store::put_progress(&mut write, &beta(), UtcMillis::from_epoch_millis(4_000))
        .await
        .expect("beta writes");
    write.commit().await.expect("the writes commit");

    let mut read = db.reader().acquire().await.expect("a read");
    let stored = store::progress(&mut read).await.expect("the progress");
    assert_eq!(
        stored,
        [
            StoredProgress {
                course: "al".to_owned(),
                name: "Alpha".to_owned(),
                flag: "a".to_owned(),
                mastery_pct: 61.5,
                current_band: "A2".to_owned(),
                mature_cards: 12,
                total_cards: 20,
                current_unit: Some(7),
                bands: vec![
                    StoredBand {
                        band: "A1".to_owned(),
                        total: 8,
                        mature: 7,
                        pct: 90.0,
                        achieved: true,
                    },
                    StoredBand {
                        band: "A2".to_owned(),
                        total: 12,
                        mature: 5,
                        pct: 43.25,
                        achieved: false,
                    },
                ],
                updated_at: UtcMillis::from_epoch_millis(2_000),
            },
            StoredProgress {
                course: "be".to_owned(),
                name: "Beta".to_owned(),
                flag: "b".to_owned(),
                mastery_pct: 12.25,
                current_band: "A1".to_owned(),
                mature_cards: 3,
                total_cards: 9,
                current_unit: None,
                bands: Vec::new(),
                updated_at: UtcMillis::from_epoch_millis(4_000),
            },
        ],
        "both courses read back whole, in course order"
    );
}

#[tokio::test]
async fn a_second_write_replaces_the_course_row() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    store::put_progress(
        &mut write,
        &alpha(12, Some(7)),
        UtcMillis::from_epoch_millis(2_000),
    )
    .await
    .expect("the first write");
    store::put_progress(
        &mut write,
        &alpha(15, Some(9)),
        UtcMillis::from_epoch_millis(6_000),
    )
    .await
    .expect("the second write");
    write.commit().await.expect("the writes commit");

    let mut read = db.reader().acquire().await.expect("a read");
    let stored = store::progress(&mut read).await.expect("the progress");
    assert_eq!(stored.len(), 1, "one row per course");
    assert_eq!(stored[0].mature_cards, 15);
    assert_eq!(stored[0].current_unit, Some(9));
    assert_eq!(stored[0].updated_at, UtcMillis::from_epoch_millis(6_000));
}

#[tokio::test]
async fn the_stored_bands_are_each_courses_current_band() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    let empty = store::stored_bands(&mut write).await.expect("the bands");
    assert!(
        empty.is_empty(),
        "no course is stored before the first write"
    );
    store::put_progress(
        &mut write,
        &alpha(12, Some(7)),
        UtcMillis::from_epoch_millis(2_000),
    )
    .await
    .expect("alpha writes");
    store::put_progress(&mut write, &beta(), UtcMillis::from_epoch_millis(4_000))
        .await
        .expect("beta writes");
    write.commit().await.expect("the writes commit");

    let mut read = db.reader().acquire().await.expect("a read");
    let bands = store::stored_bands(&mut read).await.expect("the bands");
    assert_eq!(
        bands,
        BTreeMap::from([
            ("al".to_owned(), "A2".to_owned()),
            ("be".to_owned(), "A1".to_owned())
        ])
    );
}

/// Records one milestone on `day`.
async fn record(
    write: &mut sqlx::SqliteConnection,
    course: &str,
    band: &str,
    day: i64,
    baseline: bool,
    at: i64,
) {
    let course = CourseCode::new(course).expect("a course code");
    let milestone = NewMilestone {
        course: &course,
        band,
        study_day: StudyDay::from_epoch_day(day),
        baseline,
        at: UtcMillis::from_epoch_millis(at),
    };
    let _ = store::record_milestone(write, &milestone)
        .await
        .expect("the milestone records");
}

fn milestone(
    course: &str,
    band: &str,
    day: i64,
    baseline: bool,
    celebrated: Option<i64>,
) -> BandMilestone {
    BandMilestone {
        course: course.to_owned(),
        band: band.to_owned(),
        study_day: StudyDay::from_epoch_day(day),
        baseline,
        celebrated_at: celebrated.map(UtcMillis::from_epoch_millis),
    }
}

#[tokio::test]
async fn milestones_read_back_with_their_baseline_flag_and_mark() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    record(&mut write, "al", "A1", 20_000, true, 1_000).await;
    record(&mut write, "al", "A2", 20_003, false, 5_000).await;
    write.commit().await.expect("the writes commit");

    let mut read = db.reader().acquire().await.expect("a read");
    let all = store::milestones(&mut read).await.expect("the milestones");
    assert_eq!(
        all,
        [
            milestone("al", "A1", 20_000, true, Some(1_000)),
            milestone("al", "A2", 20_003, false, None),
        ],
        "a baseline reads as a baseline and a band-up as one"
    );
}

#[tokio::test]
async fn band_ups_on_a_day_are_that_days_non_baseline_milestones() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    record(&mut write, "al", "A1", 20_003, true, 1_000).await;
    record(&mut write, "al", "A2", 20_003, false, 5_000).await;
    record(&mut write, "be", "A2", 20_003, false, 5_100).await;
    record(&mut write, "be", "B1", 20_004, false, 6_000).await;
    write.commit().await.expect("the writes commit");

    let mut read = db.reader().acquire().await.expect("a read");
    let day = store::band_ups_on(&mut read, StudyDay::from_epoch_day(20_003))
        .await
        .expect("the day's band-ups");
    assert_eq!(
        day,
        [
            milestone("al", "A2", 20_003, false, None),
            milestone("be", "A2", 20_003, false, None),
        ],
        "the baseline and the other day's band-up are not the day's"
    );
}

#[tokio::test]
async fn owed_band_ups_are_the_unmarked_non_baselines_oldest_first() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    record(&mut write, "al", "A1", 20_000, true, 1_000).await;
    record(&mut write, "be", "B1", 20_005, false, 9_000).await;
    record(&mut write, "al", "A2", 20_003, false, 5_000).await;
    record(&mut write, "al", "B1", 20_004, false, 7_000).await;
    let marked = store::mark_band_up(&mut write, "al", "B1", UtcMillis::from_epoch_millis(8_000))
        .await
        .expect("the mark writes");
    assert!(marked);
    write.commit().await.expect("the writes commit");

    let mut read = db.reader().acquire().await.expect("a read");
    let owed = store::owed_band_ups(&mut read).await.expect("the owed");
    assert_eq!(
        owed,
        [
            milestone("al", "A2", 20_003, false, None),
            milestone("be", "B1", 20_005, false, None),
        ],
        "the baseline and the marked band-up are not owed; the older is first"
    );
}

#[tokio::test]
async fn law_dues_are_none_before_the_first_write_and_read_back_after() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = open(&directory).await;
    let mut write = db.write().await.expect("a write");
    assert_eq!(store::law_dues(&mut write).await.expect("the dues"), None);
    let dues = LawDues {
        study_day: StudyDay::from_epoch_day(20_003),
        backlog: 4,
        due_today: 6,
    };
    store::put_law_dues(&mut write, &dues, UtcMillis::from_epoch_millis(5_000))
        .await
        .expect("the dues write");
    write.commit().await.expect("the write commits");

    let mut read = db.reader().acquire().await.expect("a read");
    assert_eq!(
        store::law_dues(&mut read).await.expect("the dues"),
        Some(dues)
    );

    let mut write = db.write().await.expect("a write");
    let later = LawDues {
        study_day: StudyDay::from_epoch_day(20_004),
        backlog: 9,
        due_today: 2,
    };
    store::put_law_dues(&mut write, &later, UtcMillis::from_epoch_millis(6_000))
        .await
        .expect("the dues replace");
    write.commit().await.expect("the write commits");
    let mut read = db.reader().acquire().await.expect("a read");
    assert_eq!(
        store::law_dues(&mut read).await.expect("the dues"),
        Some(later),
        "a second write replaces the dues"
    );
}

/// A source-text guard: the crate root declares its six modules, public and bare, as one block after the crate's lint attributes.
/// Row S07724 mutates this block, a change no behaviour test can see.
#[test]
fn the_crate_root_declares_the_curriculum_modules_publicly() {
    let root = include_str!("../src/lib.rs");
    assert!(
        root.contains(
            "clippy::all)]\n\npub mod data_rights;\npub mod horizon;\npub mod law;\npub mod progress;\n\
             pub mod store;\npub mod unit_bands;\n"
        ),
        "the crate root declares its six modules with no attribute among them"
    );
}
