//! The draw a chest is rolled from (SPEC-081 R5, ADR-081).
//!
//! The draw is a port this context declares: the grant step asks it for each draw and never
//! reads a generator itself, so a test hands it listed draws, or a failing one, and the one
//! implementation reads the operating system's generator. A draw is a 53-bit fraction in
//! [0, 1), the resolution of the predecessor's `random.SystemRandom().random()`.

/// 2^-53: the step between two consecutive 53-bit fractions.
const STEP: f64 = 1.0 / 9_007_199_254_740_992.0;

/// Why a draw could not be taken. The grant step writes nothing for a chest whose draw failed.
#[derive(Debug, thiserror::Error)]
pub enum DrawError {
    /// The operating system's generator refused the draw.
    #[error("the operating system's generator refused a draw")]
    Generator(#[source] getrandom::Error),
}

/// A source of draws: each one a 53-bit fraction in [0, 1), or the reason none was taken.
pub trait Draw {
    /// The next draw.
    ///
    /// # Errors
    ///
    /// [`DrawError`] when no draw could be taken.
    fn draw(&mut self) -> Result<f64, DrawError>;
}

/// The operating system's generator, the one source a chest is rolled from in production.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsDraw;

impl Draw for OsDraw {
    fn draw(&mut self) -> Result<f64, DrawError> {
        getrandom::u64().map(fraction).map_err(DrawError::Generator)
    }
}

/// The top 53 bits of `bits` as a fraction in [0, 1): every value a multiple of 2^-53, and every
/// multiple below 1 reachable.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "a value below 2^53 converts to f64 exactly"
)]
pub fn fraction(bits: u64) -> f64 {
    (bits >> 11) as f64 * STEP
}
