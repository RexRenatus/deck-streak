//! The badges view's pure rules (SPEC-073 R16, R18): every catalog badge either reads a stored
//! input or is listed without progress, each streak badge's threshold is progression's own
//! boundary, a locked badge carries its input against its threshold, and the earned badges are
//! ordered most recently awarded first.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_coordination::progression::badges_view::{
    EarnedLine, Input, PROGRESS_INPUTS, Progress, ProgressInputs, WITHOUT_PROGRESS, locked,
    most_recent_first, progress,
};
use deck_streak_kernel::{Courses, StudyDay, UtcMillis};
use deck_streak_progression::badges::catalog::catalog;
use deck_streak_progression::badges::conditions::{
    BadgeContext, CENTURION_DAY_REVIEWS, FOREST_GUARDIAN_COUNT, LEGENDARY_DAY_SCORE,
    MATURITY_MILESTONE_COUNT, POLYGLOT_DECKS_DAY, conditions,
};

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
