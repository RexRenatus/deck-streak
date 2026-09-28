//! A manual clock carries the study day across the rollover, and never across midnight, without
//! sleeping (SPEC-020 A9).

use std::time::{Duration, UNIX_EPOCH};

use deck_streak_kernel::{Clock, Hour, ManualClock, StudyDayRule, UtcMillis, UtcOffset};

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
/// Five hours east of UTC, so the local day is not the UTC day.
const OFFSET_MINUTES: i16 = 300;

#[test]
fn a_manual_clock_carries_a_study_day_across_the_rollover_without_sleeping() {
    let rule = StudyDayRule::new(
        Hour::new(4).expect("an hour"),
        UtcOffset::from_minutes(OFFSET_MINUTES).expect("an offset"),
    );
    // 22:00 local, on a synthetic local day.
    let evening = UtcMillis::from_epoch_millis(
        20_000 * DAY_MS + 22 * HOUR_MS - i64::from(OFFSET_MINUTES) * MINUTE_MS,
    );
    let clock = ManualClock::new(evening);
    let studied = rule.study_day(clock.now());

    clock.advance(Duration::from_hours(3));
    assert_eq!(
        rule.study_day(clock.now()),
        studied,
        "01:00 local is past midnight and still the same study day"
    );

    clock.advance(Duration::from_millis(3 * 3_600_000 - 1));
    assert_eq!(
        rule.study_day(clock.now()),
        studied,
        "the last millisecond before the rollover is still the same study day"
    );

    clock.advance(Duration::from_millis(1));
    assert_eq!(
        rule.study_day(clock.now()).epoch_day(),
        studied.epoch_day() + 1,
        "04:00 local turns the study day"
    );

    // A replay moves the clock back.
    clock.set(evening);
    assert_eq!(clock.now(), evening);
    assert_eq!(rule.study_day(clock.now()), studied);
}

#[test]
fn a_system_time_after_the_epoch_reads_as_its_milliseconds() {
    let time = UNIX_EPOCH + Duration::from_millis(1_234);
    assert_eq!(UtcMillis::from_system_time(time).epoch_millis(), 1_234);
}

#[test]
fn a_time_before_the_epoch_reads_as_negative_milliseconds() {
    let time = UNIX_EPOCH - Duration::from_millis(1_234);
    assert_eq!(UtcMillis::from_system_time(time).epoch_millis(), -1_234);
}
