//! The study day's one error (SPEC-497 R1). It names the shape a date must have, and never the
//! text it was given.

use std::fmt;

/// A text that is not an ISO calendar date of the proleptic Gregorian calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsoDateError;

impl fmt::Display for IsoDateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not an ISO calendar date (YYYY-MM-DD)")
    }
}

impl std::error::Error for IsoDateError {}
