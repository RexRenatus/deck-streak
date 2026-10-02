//! The law track's mastery pillar (SPEC-077 R9).

/// Points one active law leech takes off the pillar.
pub const MASTERY_LEECH_PENALTY: f64 = 3.0;
/// The most the leeches take off the pillar.
pub const MASTERY_LEECH_PENALTY_CAP: f64 = 30.0;

/// The law mastery pillar for `law_leech_active` active leeches.
#[must_use]
pub fn mastery_pillar(_law_leech_active: i64) -> f64 {
    -1.0
}
