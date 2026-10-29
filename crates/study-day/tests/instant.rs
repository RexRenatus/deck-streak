//! The instant converts a system time to whole milliseconds on either side of the epoch, and the date
//! error reads as it did in the kernel, computed by this crate (SPEC-497 A3; SPEC-039 R17).

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use deck_streak_study_day::{IsoDateError, UtcMillis};

#[test]
fn a_system_time_after_the_epoch_reads_as_its_milliseconds() {
    let time = UNIX_EPOCH + Duration::from_millis(1_234);
    assert_eq!(UtcMillis::from_system_time(time).epoch_millis(), 1_234);
    assert_eq!(UtcMillis::from_system_time(UNIX_EPOCH).epoch_millis(), 0);
    assert_eq!(UtcMillis::from_epoch_millis(-7).epoch_millis(), -7);
}

#[test]
fn a_time_before_the_epoch_reads_as_negative_milliseconds() {
    let time = UNIX_EPOCH - Duration::from_millis(1_234);
    assert_eq!(UtcMillis::from_system_time(time).epoch_millis(), -1_234);
}

#[test]
fn the_date_error_reads_as_it_did_in_the_kernel() {
    // The message `thiserror` derived in the kernel, written by hand here, so no log line changes.
    assert_eq!(
        IsoDateError.to_string(),
        "not an ISO calendar date (YYYY-MM-DD)"
    );
    assert_eq!(format!("{IsoDateError:?}"), "IsoDateError");
    // It is an error of its own, with no cause behind it.
    let error: &dyn Error = &IsoDateError;
    assert!(error.source().is_none());
    assert_eq!(
        "1970-02-30".parse::<deck_streak_study_day::StudyDay>(),
        Err(IsoDateError)
    );
}
