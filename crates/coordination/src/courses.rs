//! The owner's courses agree with the readings taxonomy, or start is refused (SPEC-071 R3;
//! ADR-087).
//!
//! The courses file (the kernel's) and the readings taxonomy (readings', ADR-045) are two private
//! files that can both name a language deck with its code. Neither context may read the other's, so
//! the agreement is a use case here, checked once at start: a language deck the two files map to
//! different codes refuses start, naming both settings and neither value. A deck only one file names
//! is not detected; ADR-087 accepts that.

use deck_streak_kernel::Courses;
use deck_streak_kernel::courses::COURSES_FILE;
use deck_streak_readings::taxonomy::{READINGS_TAXONOMY, Taxonomy};

/// The two files map one language deck to two different codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the settings {courses} and {taxonomy} map a language deck to two different codes")]
pub struct CoursesDisagree {
    /// The courses file's setting.
    pub courses: &'static str,
    /// The readings taxonomy's setting.
    pub taxonomy: &'static str,
}

/// Whether `courses` agree with `taxonomy`, when the taxonomy is configured.
///
/// # Errors
///
/// [`CoursesDisagree`] when a language deck of the taxonomy is a course's deck root under another
/// code.
pub fn agree(courses: &Courses, taxonomy: Option<&Taxonomy>) -> Result<(), CoursesDisagree> {
    let Some(taxonomy) = taxonomy else {
        return Ok(());
    };
    let disagrees = taxonomy.languages().iter().any(|language| {
        courses.courses().iter().any(|course| {
            course.deck_root == language.deck() && course.code.as_str() != language.code()
        })
    });
    if disagrees {
        Err(CoursesDisagree {
            courses: COURSES_FILE,
            taxonomy: READINGS_TAXONOMY,
        })
    } else {
        Ok(())
    }
}
