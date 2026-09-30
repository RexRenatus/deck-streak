//! When a reading is studied (SPEC-047 A4, A5; R3): the rule equals the predecessor's golden, and
//! only reviews inside the two study day window count.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Clock, Hour, ManualClock, StudyDayRule, UtcMillis, UtcOffset};
use deck_streak_readings::studied::{
    STUDIED_MAJORITY_PCT, Window, is_studied, qualifies, studied_count,
};

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
/// Study day 20 000 starts at 04:00 UTC of that epoch day (the default rule).
const DAY_START: i64 = 20_000 * DAY_MS + 4 * HOUR_MS;

fn at(millis: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(millis)
}

fn review(card_id: i64, id: i64, kind: i64, ease: i64) -> Review {
    Review {
        id,
        card_id,
        ease,
        interval: 1,
        last_interval: 0,
        factor: 2500,
        taken_ms: 1000,
        kind,
    }
}

#[test]
fn the_studied_rule_matches_the_parity_golden() {
    let examined = golden::each_case("is_studied", |case| {
        let studied = u32::try_from(case.input["studied"].as_u64().expect("a count"))
            .expect("a count of a card set");
        let covered = u32::try_from(case.input["covered"].as_u64().expect("a count"))
            .expect("a count of a card set");
        assert_eq!(
            Some(is_studied(studied, covered)),
            case.output.as_bool(),
            "{studied} of {covered}"
        );
    });
    assert_eq!(examined.function, "preread_tracking.is_studied");
    assert_eq!(STUDIED_MAJORITY_PCT, 80);
    assert!(!is_studied(0, 0), "an empty covered set is never studied");
    assert!(is_studied(4, 5) && !is_studied(3, 5), "the 80 percent line");
}

#[test]
fn only_reviews_inside_the_two_study_day_window_count() {
    let rule = StudyDayRule::default();
    let generated = at(DAY_START + 6 * HOUR_MS);
    let window = Window {
        generated_at: generated,
        study_day: rule.study_day(generated),
    };
    assert_eq!(window.study_day.epoch_day(), 20_000);

    // Before the generation instant: never counts, even on the same study day.
    assert!(!window.counts(rule, at(DAY_START + 6 * HOUR_MS - 1)));
    // From the generation instant through the last millisecond of study day d + 1.
    assert!(window.counts(rule, generated));
    assert!(window.counts(rule, at(DAY_START + DAY_MS)));
    assert!(window.counts(rule, at(DAY_START + 2 * DAY_MS - 1)));
    // The rollover that starts d + 2 closes the window.
    assert!(!window.counts(rule, at(DAY_START + 2 * DAY_MS)));

    // The window is over at the rollover of d + 2, by the injected clock.
    let clock = ManualClock::new(at(DAY_START + 2 * DAY_MS - 1));
    assert!(!window.is_over(rule, clock.now()));
    clock.set(at(DAY_START + 2 * DAY_MS));
    assert!(window.is_over(rule, clock.now()));

    // The count is of covered cards with a qualifying review inside the window, once each.
    let covered = [11, 12, 13, 14];
    let reviews = [
        review(11, DAY_START + 7 * HOUR_MS, 0, 3),
        review(11, DAY_START + 8 * HOUR_MS, 1, 3),
        review(12, DAY_START + DAY_MS + HOUR_MS, 2, 1),
        review(13, DAY_START + 2 * DAY_MS, 1, 3),
        review(14, DAY_START + 5 * HOUR_MS, 1, 3),
        review(99, DAY_START + 7 * HOUR_MS, 1, 3),
        review(12, DAY_START + 7 * HOUR_MS, 4, 3),
        review(13, DAY_START + 7 * HOUR_MS, 1, 0),
    ];
    assert_eq!(studied_count(&window, rule, &covered, &reviews), 2);

    assert!(qualifies(&review(1, 1, 0, 1)) && qualifies(&review(1, 1, 3, 4)));
    assert!(!qualifies(&review(1, 1, 4, 3)) && !qualifies(&review(1, 1, 1, 0)));
    assert!(!qualifies(&review(1, 1, -1, 3)));
}

#[test]
fn the_window_follows_the_configured_offset() {
    // At UTC+9 with the 04:00 rollover, study day 20 000 starts at 19:00 UTC of epoch day 19 999,
    // so the window's edges sit nine hours from where a UTC reading would put them.
    let rule = StudyDayRule::new(
        Hour::new(4).expect("an hour"),
        UtcOffset::from_minutes(9 * 60).expect("an offset"),
    );
    let day_start = 20_000 * DAY_MS - 5 * HOUR_MS;
    let generated = at(day_start + HOUR_MS);
    let window = Window {
        generated_at: generated,
        study_day: rule.study_day(generated),
    };
    assert_eq!(window.study_day.epoch_day(), 20_000);
    assert!(
        window.counts(rule, at(day_start + 2 * DAY_MS - 1)),
        "the last instant of d + 1"
    );
    assert!(
        !window.counts(rule, at(day_start + 2 * DAY_MS)),
        "the rollover that starts d + 2, in the configured offset"
    );
    assert!(!window.is_over(rule, at(day_start + 2 * DAY_MS - 1)));
    assert!(window.is_over(rule, at(day_start + 2 * DAY_MS)));
}
