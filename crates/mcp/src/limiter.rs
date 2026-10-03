//! The limiter of failed attempts (SPEC-119 R13; `mcp_auth.py:DrillAuth._is_rate_limited` and
//! `_record_failure` at `27ee2bc`; ADR-320 D2, D4).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deck_streak_kernel::{Clock, UtcMillis};

/// How long a failure stays fresh, in milliseconds (`_RATE_LIMIT_WINDOW_SECS`).
pub const WINDOW_MILLIS: i64 = 0;

/// The fresh failures at which a bucket is limited (`_RATE_LIMIT_MAX_FAILURES`).
pub const MAX_FAILURES: usize = 0;

/// The most buckets the limiter keeps (`_RATE_LIMIT_MAX_BUCKETS`).
pub const MAX_BUCKETS: usize = 0;

/// The most failures one bucket keeps (`_RATE_LIMIT_BUCKET_HISTORY_CAP`).
pub const HISTORY_CAP: usize = 0;

/// What the guard decided for one request or one scope check (R13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The presented token's grant holds the scope.
    Allowed,
    /// Refused, and the failure recorded in its bucket.
    Denied,
    /// Refused while its bucket holds [`MAX_FAILURES`] fresh failures; nothing recorded.
    RateLimited,
}

impl Outcome {
    /// The outcome's name, as the predecessor's goldens and the log spell it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
            Self::RateLimited => "rate_limited",
        }
    }
}

/// A bucket of failed attempts: what was presented, as a short hash (`_bucket_id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bucket(String);

impl Bucket {
    /// The bucket of the presented bytes.
    #[must_use]
    pub fn of(presented: &[u8]) -> Self {
        let _ = presented;
        Self(String::new())
    }

    /// The bucket as its hexadecimal text, the form the log writes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One bucket's failures, oldest first.
struct Failures {
    bucket: Bucket,
    at: Vec<i64>,
}

/// The buckets of failed attempts, shared by every request and every scope check.
pub struct Limiter {
    clock: Arc<dyn Clock>,
    buckets: Mutex<Vec<Failures>>,
}

impl Limiter {
    /// An empty limiter reading `clock`.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            buckets: Mutex::new(Vec::new()),
        }
    }

    /// A refusal in `bucket`, decided at one reading of the clock.
    pub fn refuse(&self, bucket: &Bucket) -> Outcome {
        let now = self.clock.now();
        self.decide(bucket, now)
    }

    /// Records one failure in `bucket` directly, as `_record_failure` does (SPEC-119 T7).
    pub fn record(&self, bucket: &Bucket) {
        let _ = (bucket, self.clock.now());
    }

    /// Each bucket in order, oldest first, with its failures' instants in epoch milliseconds.
    #[must_use]
    pub fn snapshot(&self) -> Vec<(Bucket, Vec<i64>)> {
        self.lock()
            .iter()
            .map(|failures| (failures.bucket.clone(), failures.at.clone()))
            .collect()
    }

    /// The decision for one failure in `bucket` at `now`.
    fn decide(&self, bucket: &Bucket, now: UtcMillis) -> Outcome {
        let _ = (bucket, now, self.lock());
        Outcome::Denied
    }

    /// The buckets, recovered when a holder panicked: every statement leaves them consistent.
    fn lock(&self) -> MutexGuard<'_, Vec<Failures>> {
        self.buckets.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
