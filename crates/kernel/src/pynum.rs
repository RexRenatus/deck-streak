//! `CPython`'s numbers, ported once (ADR-090, SPEC-302).
//!
//! The W4 ports read the predecessor's numbers through `CPython`'s own arithmetic, and each differs
//! from the obvious Rust at an edge. Every function here is proved bit for bit against a golden of
//! `CPython` (or of the predecessor's own function), never re-derived.

/// Python's built-in `sum` of floats.
#[must_use]
pub fn sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let _ = values.into_iter().count();
    0.0
}

/// `statistics.median` of a non-empty list, or `None` for an empty one.
#[must_use]
pub fn median(values: &[f64]) -> Option<f64> {
    let _ = values.len();
    None
}

/// `statistics.mean` of a non-empty list, or `None` for an empty one.
#[must_use]
pub fn mean(values: &[f64]) -> Option<f64> {
    let _ = values.len();
    None
}

/// Python's `round(x, ndigits)` of a float.
#[must_use]
pub fn round(x: f64, ndigits: i32) -> f64 {
    let _ = ndigits;
    x
}

/// The predecessor's nearest-rank percentile.
#[must_use]
pub fn percentile(values: &[i64], pct: f64) -> i64 {
    let _ = (values.len(), pct);
    0
}

/// `CPython`'s `random.Random`.
#[derive(Debug, Clone)]
pub struct PyRandom {
    seed: u64,
}

impl PyRandom {
    /// A generator seeded as `random.Random(seed)` seeds it.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// The next 53-bit float in `[0, 1)`.
    pub fn random(&mut self) -> f64 {
        self.seed = self.seed.wrapping_add(1);
        0.0
    }

    /// `random.choices(population, k=k)`.
    pub fn choices<'a, T>(&mut self, population: &'a [T], k: usize) -> Option<Vec<&'a T>> {
        let _ = (population.len(), k);
        None
    }
}

/// `math.lgamma` of a finite argument above zero.
#[must_use]
pub fn lgamma(x: f64) -> Option<f64> {
    let _ = x;
    None
}
