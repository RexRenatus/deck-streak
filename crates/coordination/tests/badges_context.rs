//! The badge context of an evaluated day is the predecessor's (SPEC-073 A9; R5): built from
//! synthetic reviews, rollups, a card snapshot and a streak, it equals `goldens/badge_context.json`
//! (`pipeline.py:GamifyPipeline._evaluate_and_award`) field by field, and the badges it earns are
//! the ones the predecessor awarded.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeMap;

use deck_streak_coordination::progression::badge_context::{DayState, badge_context};
use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Hour, StudyDay, StudyDayRule, UtcOffset};
use deck_streak_progression::badges::conditions::{Snapshot, conditions};
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

/// The case's study-day rule: its rollover hour at its offset.
fn rule_of(input: &Value) -> StudyDayRule {
    let hour = u8::try_from(count(&input["rollover_hour"])).expect("an hour");
    let minutes = i16::try_from(number(&input["tz_offset_minutes"])).expect("an offset");
    StudyDayRule::new(
        Hour::new(hour).expect("a rollover hour"),
        UtcOffset::from_minutes(minutes).expect("an offset"),
    )
}

/// The case's reviews: `[id_ms, cid, ease, ivl, last_ivl, time_ms, rtype]`, as the adapter builds
/// them (factor 0).
fn reviews_of(input: &Value) -> Vec<Review> {
    input["reviews"]
        .as_array()
        .expect("the reviews")
        .iter()
        .map(|row| Review {
            id: number(&row[0]),
            card_id: number(&row[1]),
            ease: number(&row[2]),
            interval: number(&row[3]),
            last_interval: number(&row[4]),
            factor: 0,
            taken_ms: number(&row[5]),
            kind: number(&row[6]),
        })
        .collect()
}

#[test]
#[allow(clippy::too_many_lines)]
fn the_badge_context_matches_the_parity_golden() {
    let examined = golden::each_case("badge_context", |case| {
        let input = &case.input;
        let output = &case.output;
        let label = case.class.as_deref().unwrap_or("a case");
        let reviews = reviews_of(input);
        // The adapter's card-to-deck map: each card's deck is (cid - 1000) / 100.
        let card_decks: BTreeMap<i64, i64> = reviews
            .iter()
            .map(|review| (review.card_id, (review.card_id - 1_000).div_euclid(100)))
            .collect();
        let today = number(&input["today"]);
        // The stub store's rollups, most recent first: the i-th is the day `today - i`.
        let rollups: Vec<(StudyDay, i64)> = input["rollups"]
            .as_array()
            .expect("the rollups")
            .iter()
            .zip(0_i64..)
            .map(|(rollup, back)| {
                (
                    StudyDay::from_epoch_day(today - back),
                    number(&rollup["score"]),
                )
            })
            .collect();
        let snapshot = &input["snapshot"];
        let state = DayState {
            reviews: &reviews,
            rule: rule_of(input),
            day: StudyDay::from_epoch_day(today),
            card_decks: &card_decks,
            snapshot: Snapshot {
                mature_count: number(&snapshot["mature_count"]),
                leech_active: number(&snapshot["leech_active"]),
                backlog: number(&snapshot["backlog"]),
                due_today: number(&snapshot["due_today"]),
            },
            streak_current: count(&input["streak_current"]),
            comeback_armed: input["comeback_armed"].as_bool().expect("a flag"),
            lifetime: count(&input["lifetime"]),
            score_total: number(&input["today_score"]),
            rollups: &rollups,
        };
        let context = badge_context(&state);
        assert_eq!(
            context.lifetime,
            count(&output["lifetime_reviews"]),
            "{label}: lifetime"
        );
        assert_eq!(
            context.day_reviews,
            count(&output["day_reviews"]),
            "{label}: day reviews"
        );
        assert_eq!(
            context.day_decks,
            count(&output["day_decks"]),
            "{label}: day decks"
        );
        assert_eq!(
            context.score_total,
            number(&output["score_total"]),
            "{label}: score"
        );
        let week_scores: Vec<i64> = output["week_scores"]
            .as_array()
            .expect("the week's scores")
            .iter()
            .map(number)
            .collect();
        assert_eq!(context.week_scores, week_scores, "{label}: week scores");
        assert_eq!(
            context.week_reviews,
            count(&output["week_reviews"]),
            "{label}: week reviews"
        );
        assert_eq!(
            context.week_decks,
            count(&output["week_decks"]),
            "{label}: week decks"
        );
        assert_eq!(
            context.week_retention.to_bits(),
            float(&output["week_retention"]).to_bits(),
            "{label}: week retention {} against {}",
            context.week_retention,
            output["week_retention"]
        );
        assert_eq!(
            context.mature30_retention.to_bits(),
            float(&output["mature30_retention"]).to_bits(),
            "{label}: mature retention {} against {}",
            context.mature30_retention,
            output["mature30_retention"]
        );
        assert_eq!(
            context.mature30_answered,
            count(&output["mature30_answered"]),
            "{label}: mature answers"
        );
        assert_eq!(
            context.night_owl,
            count(&output["night_owl"]),
            "{label}: night owl"
        );
        assert_eq!(
            context.early_bird,
            count(&output["early_bird"]),
            "{label}: early bird"
        );
        assert_eq!(
            context.cleared_backlog,
            count(&output["cleared_backlog"]),
            "{label}: cleared backlog"
        );
        assert_eq!(
            context.iron_will_ok,
            output["iron_will_ok"].as_bool().expect("a flag"),
            "{label}: iron will"
        );
        let awarded: Vec<&str> = conditions(&context)
            .into_iter()
            .filter_map(|(key, met)| met.then_some(key))
            .collect();
        let expected: Vec<&str> = output["awarded"]
            .as_array()
            .expect("the awarded keys")
            .iter()
            .map(|key| key.as_str().expect("a key"))
            .collect();
        assert_eq!(awarded, expected, "{label}: the badges the context earns");
    });
    assert_eq!(examined.count, 9, "the golden's nine cases");
}
