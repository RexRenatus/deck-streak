//! Road to C2's progress view (SPEC-077 R15, T5): the stored progress of each configured course,
//! ordered by name, read by `GET /api/progress` and the bot's progress command alike.
//!
//! The api and the bot hold no edge to the curriculum context, so the view re-exports the stored
//! rows' types they render.

use deck_streak_curriculum::store::progress;
pub use deck_streak_curriculum::store::{StoredBand, StoredProgress};
use deck_streak_kernel::{Courses, Db, KernelError};

/// The stored progress of each of the configured `courses`, ordered by name (R4), filtered as
/// `stored_mature` filters it: a stored row of a course no longer configured is left out, and a
/// configured course with no stored row yet is absent, so the view is empty before the first
/// recompute.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn progress_view(db: &Db, courses: &Courses) -> Result<Vec<StoredProgress>, KernelError> {
    let mut connection = db.reader().acquire().await?;
    let mut configured: Vec<StoredProgress> = progress(&mut connection)
        .await?
        .into_iter()
        .filter(|stored| {
            courses
                .courses()
                .iter()
                .any(|course| course.code.as_str() == stored.course)
        })
        .collect();
    configured.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(configured)
}
