//! The grant port: every XP grant is requested through it, and a replay writes nothing and says so
//! (SPEC-040 R2 to R6, R10; ADR-040).
//!
//! A request names its study day, its source, its track, its amount and its scope. The source is an
//! opaque token, refused when it is built if it is not one, so a malformed source never reaches a
//! write. [`GrantPort`] is what a use case asks; [`crate::ledger::SqliteXpLedger`] answers it from
//! the service's database. The port pays and never takes back: it has no operation that debits,
//! updates or deletes a grant (CHARTER 5).

use std::future::Future;

use deck_streak_kernel::{KernelError, StudyDay, Track, UtcMillis};

use crate::xp::XpAmount;

/// The grammar every grant source matches, as its refusal names it.
pub const SOURCE_GRAMMAR: &str = "^[a-z0-9][a-z0-9:._-]{0,127}$";

/// The longest source the grammar admits, in bytes.
pub const SOURCE_MAX_LEN: usize = 128;

/// What a grant was for: an opaque token such as `reading:read:<id>`, matching [`SOURCE_GRAMMAR`].
///
/// A token names a kind of award and its subject, and nothing of the learner's own text: the
/// grammar leaves no room for a sentence, a space or a capital.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GrantSource(String);

impl GrantSource {
    /// The source `token`.
    ///
    /// # Errors
    ///
    /// [`SourceRefused`] when `token` does not match [`SOURCE_GRAMMAR`]. The refusal names the rule
    /// and never the token.
    pub fn new(token: &str) -> Result<Self, SourceRefused> {
        if is_source_token(token) {
            Ok(Self(token.to_owned()))
        } else {
            Err(SourceRefused)
        }
    }

    /// The token, as the ledger stores it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether `token` matches [`SOURCE_GRAMMAR`]: a lowercase ASCII letter or a digit, then at most
/// 127 more of those or of `:`, `.`, `_` and `-`.
fn is_source_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() <= SOURCE_MAX_LEN
        && bytes
            .first()
            .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":._-".contains(byte)
        })
}

/// A text that is not a grant source. It names the rule and never the value, because a text that
/// fails the grammar may carry what no log line should.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a grant source is an opaque token matching {grammar}", grammar = SOURCE_GRAMMAR)]
pub struct SourceRefused;

/// How often a source pays on one track (ADR-040).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GrantScope {
    /// At most once per (study day, source, track).
    PerDay,
    /// At most once per (source, track), across every study day.
    Once,
}

impl GrantScope {
    /// The scope as `xp_ledger` stores it: `per-day` or `once`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PerDay => "per-day",
            Self::Once => "once",
        }
    }
}

/// One grant, as a use case requests it (R2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantRequest {
    /// The study day the grant is for, by the kernel's rule.
    pub study_day: StudyDay,
    /// What the grant is for.
    pub source: GrantSource,
    /// The track it pays on.
    pub track: Track,
    /// The XP it pays.
    pub amount: XpAmount,
    /// Whether its key is per study day or once ever.
    pub scope: GrantScope,
}

/// What the port answers a request (R3, R4). A caller matches on it: a replay is not an error, and
/// it is not a second payment either.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantAnswer {
    /// The grant was written, paying this amount.
    Granted(XpAmount),
    /// The request's key was already granted: nothing was written, and this is the amount the
    /// ledger already holds for it.
    AlreadyGranted(XpAmount),
}

/// The one way XP is granted (R10): a coordination use case asks it, and no other code writes the
/// ledger.
pub trait GrantPort: Send + Sync {
    /// Grants `request` at the instant `at`, or answers that its key was already granted. The check
    /// and the write are one `BEGIN IMMEDIATE` transaction, so two concurrent requests for one key
    /// write one row (R5).
    fn grant(
        &self,
        request: &GrantRequest,
        at: UtcMillis,
    ) -> impl Future<Output = Result<GrantAnswer, KernelError>> + Send;
}
