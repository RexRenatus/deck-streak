//! Times the FSRS-7 replay over the grid and prints one fixed line per cell (SPEC-342 R7). CI runs
//! it natively on one thread and as `wasm32-wasip1` under Node's V8 (ADR-353 D2); the harness is
//! the crate's `measure` module, which this only calls (ADR-353 D6).

use std::io::{self, Write as _};
use std::time::Instant;

use deck_streak_fsrs7::measure::{self, MEANS, MeasureError, REVIEWS};

/// The target this build runs on, as the line names it.
#[cfg(target_os = "wasi")]
const TARGET: &str = "wasm32-wasip1";
/// The target this build runs on, as the line names it.
#[cfg(not(target_os = "wasi"))]
const TARGET: &str = "native";

fn main() -> Result<(), MeasureError> {
    let started = Instant::now();
    let mut clock = || started.elapsed().as_secs_f64() * 1000.0;
    let mut out = io::stdout().lock();
    measure::run_grid(TARGET, &REVIEWS, &MEANS, &mut clock, &mut out)?;
    out.flush().map_err(MeasureError::Write)
}
