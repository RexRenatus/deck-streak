//! The badge catalog (SPEC-073 R2): the predecessor's 40 badges, each with its key, name, emoji and
//! description at tier 0, equal to `goldens/badge_catalog.json`.
//!
//! Five descriptions name courses of the predecessor's own owner; here they are rendered from the
//! configured courses (ADR-087), so a catalog never carries a course the owner does not study.

use deck_streak_kernel::Courses;

/// The family a badge belongs to: which context decides it is earned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// The 24 study badges, decided by the recompute's badge step (R5 to R8).
    Study,
    /// The eight habit badges (reading and writing), decided by the habit context.
    Habit,
    /// The eight focus badges, decided by the focus context.
    Focus,
}

/// One catalog badge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Badge {
    /// The key, unique across the catalog.
    pub key: String,
    /// The name a screen shows.
    pub name: String,
    /// The emoji a screen shows.
    pub emoji: String,
    /// The criterion in words.
    pub description: String,
    /// The tier, 0 for every catalog badge.
    pub tier: u32,
    /// Which context decides the badge.
    pub family: Family,
}

/// The catalog under `courses`: 40 badges, each at tier 0, in the predecessor's order.
#[must_use]
pub fn catalog(courses: &Courses) -> Vec<Badge> {
    let _ = courses;
    Vec::new()
}

/// Whether `key` names a catalog badge.
#[must_use]
pub fn is_catalog_key(key: &str) -> bool {
    let _ = key;
    false
}
