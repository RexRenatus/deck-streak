//! The law track's mastery pillar (SPEC-077 R9).

/// Points one active law leech takes off the pillar.
pub const MASTERY_LEECH_PENALTY: f64 = 3.0;
/// The most the leeches take off the pillar.
pub const MASTERY_LEECH_PENALTY_CAP: f64 = 30.0;

/// The law mastery pillar for `law_leech_active` active leeches.
#[must_use]
pub fn mastery_pillar(law_leech_active: i64) -> f64 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a leech count is far below 2^52"
    )]
    let penalty = (MASTERY_LEECH_PENALTY * law_leech_active as f64).min(MASTERY_LEECH_PENALTY_CAP);
    (100.0 - penalty).clamp(0.0, 100.0)
}
