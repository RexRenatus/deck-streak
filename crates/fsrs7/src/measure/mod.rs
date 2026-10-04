//! The replay-time harness (SPEC-342 R5, R7 and R8; ADR-353 D6): the synthetic history, the timed
//! grid and its fixed lines, and the report over both targets. The crate's examples only call it,
//! so every rule here is tested and mutated with the crate.

pub mod history;
pub mod report;

use std::io::{self, Write};

use fsrs7::{FSRS, FSRSError, FSRSItem};

use crate::convert;
use crate::replay::{self, Method};
use report::Summary;

/// The grid's review-row counts (SPEC-342 R7).
pub const REVIEWS: [usize; 3] = [10_000, 100_000, 1_000_000];
/// The grid's mean reviews per card (SPEC-342 R7).
pub const MEANS: [usize; 2] = [8, 32];
/// Timed runs per cell; a cell's line carries their median, minimum and maximum.
pub const RUNS: usize = 5;
/// Untimed runs per cell before the timed ones, so the first timed run pays no first-touch cost.
pub const WARMUPS: usize = 1;

/// Why a grid stopped.
#[derive(Debug)]
pub enum MeasureError {
    /// The scheduler refused the model or an item.
    Replay(FSRSError),
    /// A line could not be written.
    Write(io::Error),
}

/// Runs every cell of `reviews` by `means` by both methods, from the pinned revision's FSRS-7
/// defaults, and writes one fixed line per cell to `out`. `clock` reads milliseconds; it is read
/// before and after each timed run, and never around a warm-up.
///
/// # Errors
///
/// [`MeasureError::Replay`] when the scheduler refuses the model or an item, and
/// [`MeasureError::Write`] when `out` refuses a line.
pub fn run_grid(
    target: &str,
    reviews: &[usize],
    means: &[usize],
    clock: &mut dyn FnMut() -> f64,
    out: &mut dyn Write,
) -> Result<(), MeasureError> {
    let model = FSRS::new(&[]).map_err(MeasureError::Replay)?;
    for &count in reviews {
        for &mean in means {
            let items: Vec<FSRSItem> = convert::histories(&history::rows(count, mean))
                .into_iter()
                .map(|card| card.item)
                .collect();
            for method in Method::ALL {
                for _ in 0..WARMUPS {
                    replay::replay(method, &model, items.clone()).map_err(MeasureError::Replay)?;
                }
                let mut runs_ms = Vec::with_capacity(RUNS);
                let mut checksum = 0.0;
                for _ in 0..RUNS {
                    let input = items.clone();
                    let started = clock();
                    let states =
                        replay::replay(method, &model, input).map_err(MeasureError::Replay)?;
                    runs_ms.push(clock() - started);
                    checksum = replay::checksum(&states);
                }
                let summary = Summary {
                    target: target.to_owned(),
                    method,
                    reviews: count,
                    cards: items.len(),
                    mean,
                    runs_ms,
                    checksum,
                };
                writeln!(out, "{}", summary.line()).map_err(MeasureError::Write)?;
            }
        }
    }
    Ok(())
}
