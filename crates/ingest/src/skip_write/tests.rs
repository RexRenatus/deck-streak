use super::{now_millis, observes_daylight_saving};
use chrono::{NaiveDate, TimeZone, Utc};
use deck_streak_kernel::UtcMillis;
use std::time::{SystemTime, UNIX_EPOCH};

fn system_millis() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the clock is after the epoch")
            .as_millis(),
    )
    .expect("the millisecond count fits")
}

#[test]
fn now_millis_reads_the_clock_between_two_readings() {
    let before = system_millis();
    let read = now_millis();
    let after = system_millis();
    assert!(before > 1_000_000_000_000, "the clock reads a real instant");
    assert!(
        (before..=after).contains(&read),
        "{read} is not within {before}..={after}"
    );
}

/// The marker the child process reads: it runs the check below in a process whose zone is set
/// from the start, since this crate's library cannot set `TZ` in its own process.
const CHILD: &str = "SKIP_WRITE_DAYLIGHT_CHILD";

#[test]
fn daylight_saving_looks_only_two_calendar_years_ahead() {
    if std::env::var_os(CHILD).is_some() {
        // Tokyo observed daylight saving in 1948 to 1951 and not in 1946 or 1947, so the
        // window of the instant's year and the next sees none, and a window that reaches
        // farther would.
        let day = NaiveDate::from_ymd_opt(1946, 6, 1)
            .and_then(|day| day.and_hms_opt(0, 0, 0))
            .expect("a valid date");
        let now = UtcMillis::from_epoch_millis(Utc.from_utc_datetime(&day).timestamp_millis());
        assert!(!observes_daylight_saving(now));
        return;
    }
    let exe = std::env::current_exe().expect("the test binary");
    let status = std::process::Command::new(exe)
        .args([
            "--exact",
            "skip_write::tests::daylight_saving_looks_only_two_calendar_years_ahead",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("TZ", "Asia/Tokyo")
        .status()
        .expect("the child runs");
    assert!(status.success(), "the child's check failed");
}
