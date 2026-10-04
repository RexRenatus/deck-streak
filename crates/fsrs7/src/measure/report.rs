//! A cell's fixed line (SPEC-342 R7) and the report over both targets' lines (R8).

use crate::replay::Method;

/// The word each fixed line starts with.
pub const PREFIX: &str = "fsrs7-replay";
/// The fixed line's keys, in order, after [`PREFIX`].
pub const KEYS: [&str; 11] = [
    "target",
    "method",
    "reviews",
    "cards",
    "mean",
    "runs",
    "median_ms",
    "min_ms",
    "max_ms",
    "per_review_us",
    "checksum",
];
/// The two targets a report reads: native, and the WASI module under Node's V8 (ADR-353 D2).
pub const TARGETS: [&str; 2] = ["native", "wasm32-wasip1"];
/// The most two targets' checksums of one cell may differ by, relative to the native one: one part
/// in 10,000 (SPEC-342 R8).
pub const TOLERANCE: f64 = 1e-4;

/// One cell's timed runs, as the grid measured them.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    /// The target the runs were taken on.
    pub target: String,
    /// The replay's method.
    pub method: Method,
    /// The cell's review rows.
    pub reviews: usize,
    /// The cards those rows make.
    pub cards: usize,
    /// The cell's mean reviews per card.
    pub mean: usize,
    /// Each timed run, in milliseconds, in the order taken.
    pub runs_ms: Vec<f64>,
    /// The replay's checksum.
    pub checksum: f64,
}

impl Summary {
    /// The cell's line in the fixed form: the median, minimum and maximum of its runs in
    /// milliseconds, the median's cost per review row in microseconds, and the checksum.
    #[must_use]
    pub fn line(&self) -> String {
        String::new()
    }
}

/// The report over the native and wasm targets' lines and the web target's check, as markdown; or
/// every reason it is refused (SPEC-342 R8): a line not in the fixed form, a cell missing, doubled
/// or outside the grid, two checksums apart by more than [`TOLERANCE`], and a web-target check
/// that is absent or reads neither `pass` nor `fail crates=<names>`.
///
/// # Errors
///
/// Every refusal, each naming its target, line or cell.
pub fn render(native: &str, wasm: &str, web_check: &str) -> Result<String, Vec<String>> {
    let _ = (native, wasm, web_check);
    Ok(String::new())
}
