//! The limiter of failed attempts (SPEC-119 R13; `mcp_auth.py:DrillAuth._is_rate_limited` and
//! `_record_failure` at `27ee2bc`; ADR-320 D2, D4).

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deck_streak_kernel::{Clock, UtcMillis};
use sha2::{Digest, Sha256};

/// How long a failure stays fresh, in milliseconds (`_RATE_LIMIT_WINDOW_SECS`).
pub const WINDOW_MILLIS: i64 = 60_000;

/// The fresh failures at which a bucket is limited (`_RATE_LIMIT_MAX_FAILURES`).
pub const MAX_FAILURES: usize = 5;

/// The most buckets the limiter keeps (`_RATE_LIMIT_MAX_BUCKETS`).
pub const MAX_BUCKETS: usize = 512;

/// The most failures one bucket keeps (`_RATE_LIMIT_BUCKET_HISTORY_CAP`).
pub const HISTORY_CAP: usize = 10;

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
    /// The bucket of the presented bytes: the first 16 hexadecimal characters of their SHA-256
    /// digest, and nothing presented hashes as `\x00absent` (`_bucket_id`; R13 as amended by T3).
    #[must_use]
    pub fn of(presented: &[u8]) -> Self {
        let hashed: &[u8] = if presented.is_empty() {
            b"\x00absent"
        } else {
            presented
        };
        let mut hex = String::with_capacity(16);
        for byte in Sha256::digest(hashed).iter().take(8) {
            let _ = write!(hex, "{byte:02x}");
        }
        Self(hex)
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

    /// Records one failure in `bucket` directly, as `_record_failure` does (SPEC-119 T7), at one
    /// reading of the clock.
    pub fn record(&self, bucket: &Bucket) {
        let now = self.clock.now();
        record_failure(&mut self.lock(), bucket, now);
    }

    /// Each bucket in order, oldest first, with its failures' instants in epoch milliseconds.
    #[must_use]
    pub fn snapshot(&self) -> Vec<(Bucket, Vec<i64>)> {
        self.lock()
            .iter()
            .map(|failures| (failures.bucket.clone(), failures.at.clone()))
            .collect()
    }

    /// The decision for one failure in `bucket` at `now`, in one critical section
    /// (`_is_rate_limited`, then `_record_failure`). A bucket holding [`MAX_FAILURES`] fresh
    /// failures is limited and records nothing; its stale failures are dropped and it keeps its
    /// place (T4). Any other failure is recorded and denied.
    fn decide(&self, bucket: &Bucket, now: UtcMillis) -> Outcome {
        let mut buckets = self.lock();
        if let Some(failures) = buckets
            .iter_mut()
            .find(|failures| failures.bucket == *bucket)
        {
            failures.at.retain(|at| fresh(*at, now));
            if failures.at.len() >= MAX_FAILURES {
                return Outcome::RateLimited;
            }
        }
        record_failure(&mut buckets, bucket, now);
        Outcome::Denied
    }

    /// The buckets, recovered when a holder panicked: every statement leaves them consistent.
    fn lock(&self) -> MutexGuard<'_, Vec<Failures>> {
        self.buckets.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Whether a failure at `at` is still fresh at `now`: younger than [`WINDOW_MILLIS`]. A failure
/// stamped after `now` (the wall clock stepped back) counts as fresh.
fn fresh(at: i64, now: UtcMillis) -> bool {
    now.epoch_millis().saturating_sub(at) < WINDOW_MILLIS
}

/// Records one failure in `bucket` at `now` (`_record_failure`): its stale failures dropped, the
/// newest [`HISTORY_CAP`] kept, the bucket moved last, and the oldest buckets evicted past
/// [`MAX_BUCKETS`].
fn record_failure(buckets: &mut Vec<Failures>, bucket: &Bucket, now: UtcMillis) {
    let mut at = match buckets
        .iter()
        .position(|failures| failures.bucket == *bucket)
    {
        Some(index) => buckets.remove(index).at,
        None => Vec::new(),
    };
    at.retain(|instant| fresh(*instant, now));
    at.push(now.epoch_millis());
    let excess = at.len().saturating_sub(HISTORY_CAP);
    at.drain(..excess);
    buckets.push(Failures {
        bucket: bucket.clone(),
        at,
    });
    while buckets.len() > MAX_BUCKETS {
        buckets.remove(0);
    }
}
