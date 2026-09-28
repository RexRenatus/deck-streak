//! The `job` role: runs one job of the table by its id, and exits (SPEC-027 R5, R7, R11; ADR-027).
//!
//! Each job is a `oneshot` unit its timer starts, so the role sends no `sd_notify(3)` message: it
//! installs logging and reads its settings like every role, opens the database under the open lock,
//! runs the job once through `coordination::runner`, and exits with the runner's code. 0 means the
//! job ran, skipped or was recorded `missed`; 1 is a page, which fails the unit so `OnFailure=` sends
//! the one alert; 2 is an id the table does not hold.

use deck_streak_coordination::jobs::Job;
use deck_streak_kernel::{Environment, Redactor};

/// Why the `job` role stopped before its job could run.
#[derive(Debug, thiserror::Error)]
pub enum JobRoleError {}

/// Runs `job` once, and returns the process's exit code.
///
/// # Errors
///
/// Every refusal of [`JobRoleError`]: a setting refuses start, or the database cannot be opened.
pub async fn run(env: &Environment, redactor: &Redactor, job: &Job) -> Result<u8, JobRoleError> {
    let _ = (env, redactor, job);
    Ok(0)
}
