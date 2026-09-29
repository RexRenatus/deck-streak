//! The attempts' retention (SPEC-046 R10): ninety days, counted in whole days.

use deck_streak_kernel::UtcMillis;
use deck_streak_readings::attempts::{RETAIN_DAYS, retention_cutoff};

#[test]
fn an_attempt_is_kept_for_exactly_ninety_whole_days() {
    assert_eq!(RETAIN_DAYS, 90, "ninety days");
    let now = UtcMillis::from_epoch_millis(1_000_000_000_000);
    assert_eq!(
        retention_cutoff(now).epoch_millis(),
        1_000_000_000_000 - 90 * 86_400_000,
        "the cutoff is ninety days of 86 400 000 milliseconds back"
    );
}
