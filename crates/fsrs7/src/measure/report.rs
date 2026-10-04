//! A cell's fixed line (SPEC-342 R7) and the report over both targets' lines (R8).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::{MEANS, REVIEWS, RUNS};
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
        let mut runs = self.runs_ms.clone();
        runs.sort_by(f64::total_cmp);
        let at = |index: usize| runs.get(index).copied().unwrap_or(f64::NAN);
        let (median, min, max) = (at(runs.len() / 2), at(0), at(runs.len() - 1));
        #[allow(
            clippy::cast_precision_loss,
            reason = "a review-row count of the grid is exact in an f64"
        )]
        let per_review_us = median * 1000.0 / self.reviews as f64;
        format!(
            "{PREFIX} target={} method={} reviews={} cards={} mean={} runs={} median_ms={median:.3} \
             min_ms={min:.3} max_ms={max:.3} per_review_us={per_review_us:.4} checksum={:.3}",
            self.target,
            self.method.name(),
            self.reviews,
            self.cards,
            self.mean,
            self.runs_ms.len(),
            self.checksum,
        )
    }
}

/// A cell of the grid: its method, review rows and mean.
type Key = (Method, usize, usize);

/// One target's parsed line.
#[derive(Clone, Debug)]
struct Cell {
    target: String,
    key: Key,
    cards: usize,
    runs: usize,
    median_ms: f64,
    min_ms: f64,
    max_ms: f64,
    per_review_us: f64,
    checksum: f64,
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
    let mut refusals = Vec::new();
    let native = cells(TARGETS[0], native, &mut refusals);
    let wasm = cells(TARGETS[1], wasm, &mut refusals);
    let mut rows = String::new();
    for key in grid() {
        let (Some(here), Some(there)) = (native.get(&key), wasm.get(&key)) else {
            continue;
        };
        if (here.checksum - there.checksum).abs() > TOLERANCE * here.checksum.abs() {
            refusals.push(format!(
                "cell {}: checksums disagree: {} {:.3} and {} {:.3}",
                named(key),
                TARGETS[0],
                here.checksum,
                TARGETS[1],
                there.checksum
            ));
        }
        let _ = writeln!(
            rows,
            "| {} | {} | {} | {} | {} | {} | {:.3} |",
            key.0.name(),
            key.1,
            key.2,
            here.cards,
            timing(here),
            timing(there),
            here.checksum
        );
    }
    let web = web_target(web_check).unwrap_or_else(|refusal| {
        refusals.push(refusal);
        String::new()
    });
    if !refusals.is_empty() {
        return Err(refusals);
    }
    Ok(format!(
        "## FSRS-7 replay time (SPEC-342)\n\n\
         Median of {RUNS} runs in ms (minimum to maximum), and the median's microseconds per \
         review row.\n\n\
         | method | reviews | mean | cards | {} | {} | checksum |\n\
         |---|---|---|---|---|---|---|\n\
         {rows}\n\
         web target (`wasm32-unknown-unknown`) check: {web}\n",
        TARGETS[0], TARGETS[1]
    ))
}

/// Every cell of the grid, in order.
fn grid() -> Vec<Key> {
    let mut keys = Vec::new();
    for method in Method::ALL {
        for reviews in REVIEWS {
            for mean in MEANS {
                keys.push((method, reviews, mean));
            }
        }
    }
    keys
}

/// A cell as a refusal names it.
fn named((method, reviews, mean): Key) -> String {
    format!("{} reviews={reviews} mean={mean}", method.name())
}

/// A cell's timing, as the report's table prints it.
fn timing(cell: &Cell) -> String {
    format!(
        "{:.3} ({:.3} to {:.3}), {:.4} µs",
        cell.median_ms, cell.min_ms, cell.max_ms, cell.per_review_us
    )
}

/// `target`'s cells by key, with a refusal for each line that is not a cell of the grid in the
/// fixed form, each cell reported twice, and each cell missing.
fn cells(target: &str, text: &str, refusals: &mut Vec<String>) -> BTreeMap<Key, Cell> {
    let mut held = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let Some(cell) = parse(line) else {
            refusals.push(format!(
                "{target} line {}: not in the fixed form: {line}",
                index + 1
            ));
            continue;
        };
        if cell.target != target {
            refusals.push(format!(
                "{target} line {}: names target {}",
                index + 1,
                cell.target
            ));
        } else if !grid().contains(&cell.key) {
            refusals.push(format!(
                "{target}: cell {} is outside the grid",
                named(cell.key)
            ));
        } else if cell.runs != RUNS {
            refusals.push(format!(
                "{target}: cell {} ran {} times, not {RUNS}",
                named(cell.key),
                cell.runs
            ));
        } else if held.insert(cell.key, cell.clone()).is_some() {
            refusals.push(format!(
                "{target}: cell {} is reported twice",
                named(cell.key)
            ));
        }
    }
    for key in grid() {
        if !held.contains_key(&key) {
            refusals.push(format!("{target}: cell {} is missing", named(key)));
        }
    }
    held
}

/// A line in the fixed form, or None.
fn parse(line: &str) -> Option<Cell> {
    let mut words = line.split_whitespace();
    if words.next()? != PREFIX {
        return None;
    }
    let mut values = Vec::with_capacity(KEYS.len());
    for key in KEYS {
        let (name, value) = words.next()?.split_once('=')?;
        if name != key {
            return None;
        }
        values.push(value);
    }
    if words.next().is_some() {
        return None;
    }
    let number = |at: usize| values[at].parse::<f64>().ok();
    let count = |at: usize| values[at].parse::<usize>().ok();
    Some(Cell {
        target: values[0].to_owned(),
        key: (Method::named(values[1])?, count(2)?, count(4)?),
        cards: count(3)?,
        runs: count(5)?,
        median_ms: number(6)?,
        min_ms: number(7)?,
        max_ms: number(8)?,
        per_review_us: number(9)?,
        checksum: number(10)?,
    })
}

/// The web target's check as the report prints it, or why it is refused.
fn web_target(check: &str) -> Result<String, String> {
    let check = check.trim();
    if check.is_empty() {
        return Err("no web-target check line".to_owned());
    }
    if check == "pass" {
        return Ok("pass".to_owned());
    }
    match check.strip_prefix("fail crates=") {
        Some(crates) if !crates.is_empty() => Ok(format!("fail ({crates})")),
        _ => Err(format!(
            "the web-target check reads {check:?}, not pass or fail crates=<names>"
        )),
    }
}
