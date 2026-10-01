//! The study conditions and the hour windows are the predecessor's (SPEC-073 A6 to A8; R6, R7):
//! every condition over the golden's contexts, each threshold on both sides; every threshold equal
//! to the predecessor's constant; and the hour windows over reviews around their edges.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeMap;

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Hour, StudyDay, StudyDayRule, UtcOffset};
use deck_streak_progression::badges::conditions::{
    self, BadgeContext, STUDY_KEYS, Snapshot, conditions,
};
use deck_streak_progression::badges::hours::count_in_local_hours;
use serde_json::Value;

/// A golden field as an unsigned count.
fn count(value: &Value) -> u64 {
    value
        .as_u64()
        .unwrap_or_else(|| panic!("a count, not {value}"))
}

/// A golden field as a signed number.
fn number(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("a number, not {value}"))
}

/// A golden field as a float.
fn float(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("a float, not {value}"))
}

/// The badge context a golden case holds.
fn context_of(input: &Value) -> BadgeContext {
    BadgeContext {
        lifetime: count(&input["lifetime"]),
        streak_current: count(&input["streak_current"]),
        comeback_armed: input["comeback_armed"].as_bool().expect("a flag"),
        day_reviews: count(&input["day_reviews"]),
        day_decks: count(&input["day_decks"]),
        day_avg_seconds: float(&input["day_avg_seconds"]),
        snapshot: Snapshot {
            mature_count: number(&input["mature_count"]),
            leech_active: number(&input["leech_active"]),
            backlog: number(&input["backlog"]),
            due_today: number(&input["due_today"]),
        },
        score_total: number(&input["score_total"]),
        week_scores: input["week_scores"]
            .as_array()
            .expect("the week's scores")
            .iter()
            .map(number)
            .collect(),
        week_reviews: count(&input["week_reviews"]),
        week_decks: count(&input["week_decks"]),
        week_retention: float(&input["week_retention"]),
        mature30_answered: count(&input["mature30_answered"]),
        mature30_retention: float(&input["mature30_retention"]),
        night_owl: count(&input["night_owl"]),
        early_bird: count(&input["early_bird"]),
        cleared_backlog: count(&input["cleared_backlog"]),
        iron_will_ok: input["iron_will_ok"].as_bool().expect("a flag"),
    }
}

#[test]
fn the_study_badge_conditions_match_the_parity_golden() {
    let mut judged = 0_usize;
    let mut met = 0_usize;
    golden::each_case("badge_conditions", |case| {
        let ours: BTreeMap<&str, bool> = conditions(&context_of(&case.input)).into_iter().collect();
        let expected = case.output.as_object().expect("each condition by key");
        assert_eq!(ours.len(), expected.len(), "every study badge is judged");
        for (key, flag) in expected {
            let flag = flag.as_bool().expect("a flag");
            assert_eq!(
                ours.get(key.as_str()).copied(),
                Some(flag),
                "{key} over {}",
                case.input
            );
            judged += 1;
            met += usize::from(flag);
        }
    });
    println!("examined {judged} condition(s), {met} met");
    assert!(
        met > 0 && met < judged,
        "both sides of the thresholds are judged"
    );
}

#[test]
fn the_badge_constants_equal_the_predecessors() {
    let ours: BTreeMap<&str, f64> = [
        (
            "constants.LIFETIME_REVIEWS_GRINDER",
            conditions::LIFETIME_REVIEWS_GRINDER as f64,
        ),
        (
            "constants.LIFETIME_REVIEWS_MARATHONER",
            conditions::LIFETIME_REVIEWS_MARATHONER as f64,
        ),
        (
            "constants.CENTURION_DAY_REVIEWS",
            conditions::CENTURION_DAY_REVIEWS as f64,
        ),
        (
            "constants.SHARPSHOOTER_RETENTION",
            conditions::SHARPSHOOTER_RETENTION,
        ),
        (
            "constants.SHARPSHOOTER_MIN_REVIEWS",
            conditions::SHARPSHOOTER_MIN_REVIEWS as f64,
        ),
        (
            "constants.SNIPER_ELITE_RETENTION",
            conditions::SNIPER_ELITE_RETENTION,
        ),
        (
            "constants.SNIPER_ELITE_MIN_MATURE",
            conditions::SNIPER_ELITE_MIN_MATURE as f64,
        ),
        (
            "constants.BACKLOG_SLAYER_CLEARED",
            conditions::BACKLOG_SLAYER_CLEARED as f64,
        ),
        (
            "constants.NIGHT_OWL_REVIEWS",
            conditions::NIGHT_OWL_REVIEWS as f64,
        ),
        (
            "constants.EARLY_BIRD_REVIEWS",
            conditions::EARLY_BIRD_REVIEWS as f64,
        ),
        (
            "constants.EARLY_BIRD_END_HOUR",
            f64::from(conditions::EARLY_BIRD_END_HOUR),
        ),
        (
            "constants.MATURITY_MILESTONE_COUNT",
            conditions::MATURITY_MILESTONE_COUNT as f64,
        ),
        (
            "constants.FOREST_GUARDIAN_COUNT",
            conditions::FOREST_GUARDIAN_COUNT as f64,
        ),
        (
            "constants.POLYGLOT_DECKS_DAY",
            conditions::POLYGLOT_DECKS_DAY as f64,
        ),
        (
            "constants.GLOBETROTTER_DECKS_WEEK",
            conditions::GLOBETROTTER_DECKS_WEEK as f64,
        ),
        (
            "constants.PERFECT_WEEK_SCORE",
            conditions::PERFECT_WEEK_SCORE as f64,
        ),
        (
            "constants.SPEED_DEMON_REVIEWS",
            conditions::SPEED_DEMON_REVIEWS as f64,
        ),
        (
            "constants.SPEED_DEMON_AVG_SECONDS",
            conditions::SPEED_DEMON_AVG_SECONDS,
        ),
        (
            "constants.IRON_WILL_DAYS",
            conditions::IRON_WILL_DAYS as f64,
        ),
        (
            "gamification.badges.LEECH_TAMER_MIN_LIFETIME",
            conditions::LEECH_TAMER_MIN_LIFETIME as f64,
        ),
    ]
    .into_iter()
    .collect();
    let mut compared = 0_usize;
    golden::each_case("badges.constants", |case| {
        let name = case.input["name"].as_str().expect("the constant's name");
        let theirs = float(&case.output);
        assert_eq!(ours.get(name).copied(), Some(theirs), "the constant {name}");
        compared += 1;
    });
    println!("examined {compared} constant(s)");
    assert_eq!(compared, ours.len(), "every threshold R6 names is compared");
    assert_eq!(STUDY_KEYS.len(), 24, "the 24 study badges");
}

#[test]
fn the_hour_counts_match_the_parity_golden() {
    let mut counted = 0_u64;
    golden::each_case("count_reviews_in_local_hours", |case| {
        let input = &case.input;
        let rule = StudyDayRule::new(
            Hour::new(u8::try_from(count(&input["rollover_hour"])).expect("an hour"))
                .expect("a rollover hour"),
            UtcOffset::from_minutes(
                i16::try_from(number(&input["tz_offset_minutes"])).expect("minutes"),
            )
            .expect("an offset"),
        );
        let reviews: Vec<Review> = input["reviews"]
            .as_array()
            .expect("the reviews")
            .iter()
            .map(|row| Review {
                id: number(&row[0]),
                card_id: number(&row[1]),
                ease: number(&row[2]),
                interval: number(&row[3]),
                last_interval: number(&row[4]),
                factor: 2500,
                taken_ms: number(&row[5]),
                kind: number(&row[6]),
            })
            .collect();
        let ours = count_in_local_hours(
            &reviews,
            rule,
            StudyDay::from_epoch_day(number(&input["day"])),
            u8::try_from(count(&input["start_hour"])).expect("an hour"),
            u8::try_from(count(&input["end_hour"])).expect("an hour"),
        );
        let theirs = count(&case.output);
        assert_eq!(ours, theirs, "the count of {}", input);
        counted += theirs;
    });
    println!("examined reviews counted in windows: {counted}");
    assert!(counted > 0, "some window counts a review");
}
