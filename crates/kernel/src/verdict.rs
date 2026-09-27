//! The verdict: the outcome of a check or a transition, which a caller must match on (SPEC-020 R9,
//! docs/LEXICON.md).
//!
//! It is `#[must_use]`, and the workspace denies `unused_must_use`, so a refusal can never be
//! dropped on the floor. Dropping one does not compile:
//!
//! ```compile_fail
//! #![deny(unused_must_use)]
//! use deck_streak_kernel::Verdict;
//!
//! fn check() -> Verdict<&'static str> {
//!     Verdict::Refuse("stale")
//! }
//!
//! check();
//! ```
//!
//! and the same program that matches on it does:
//!
//! ```
//! #![deny(unused_must_use)]
//! use deck_streak_kernel::Verdict;
//!
//! fn check() -> Verdict<&'static str> {
//!     Verdict::Refuse("stale")
//! }
//!
//! match check() {
//!     Verdict::Pass => unreachable!("the check refuses"),
//!     Verdict::Refuse(reason) => assert_eq!(reason, "stale"),
//! }
//! ```

/// A check's outcome: it passed, or it refused for a reason the caller must handle.
#[must_use = "a verdict is matched on: dropping one ignores a refusal"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict<R> {
    /// The check passed.
    Pass,
    /// The check refused, for this reason.
    Refuse(R),
}

impl<R> Verdict<R> {
    /// Whether the check passed.
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }

    /// The refusal's reason, or `None` when the check passed.
    #[must_use]
    pub fn refusal(self) -> Option<R> {
        match self {
            Self::Pass => None,
            Self::Refuse(reason) => Some(reason),
        }
    }

    /// The verdict as a `Result`, so `?` can carry a refusal up.
    ///
    /// # Errors
    ///
    /// The refusal's reason, when the check refused.
    pub fn into_result(self) -> Result<(), R> {
        match self {
            Self::Pass => Ok(()),
            Self::Refuse(reason) => Err(reason),
        }
    }
}
