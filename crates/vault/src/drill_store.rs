//! The vault's two drill tables (SPEC-110 R6, R11). This is the inert shape the tests are written
//! against: every call answers the empty case, and the queries follow in their own commit.

use deck_streak_kernel::{KernelError, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

/// The table of answers.
pub const DRILL_ANSWERS_TABLE: &str = "drill_answers";
/// The table of grades.
pub const DRILL_GRADES_TABLE: &str = "drill_grades";

/// The surface an answer came through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// The Telegram bot.
    Bot,
    /// The mini app and the API behind it.
    MiniApp,
}

impl Surface {
    /// The value the table's `CHECK` names.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bot => "bot",
            Self::MiniApp => "mini_app",
        }
    }
}

/// One graded drill as the table holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GradeRow {
    /// The drill's id.
    pub drill_id: String,
    /// The drill's type.
    pub drill_type: String,
    /// The drill's subject.
    pub subject: String,
    /// The accepted XP.
    pub xp: i64,
    /// The study day of the poll that recorded it.
    pub study_day: StudyDay,
}

/// Records the answer of `drill_id`; `false` means the drill has an answer already.
///
/// # Errors
///
/// [`KernelError::Database`] when the insert fails.
pub async fn insert_answer(
    _connection: &mut SqliteConnection,
    _drill_id: &str,
    _day: StudyDay,
    _surface: Surface,
    _at: UtcMillis,
) -> Result<bool, KernelError> {
    Ok(true)
}

/// Whether `drill_id` has an answer row.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn has_answer(
    _connection: &mut SqliteConnection,
    _drill_id: &str,
) -> Result<bool, KernelError> {
    Ok(false)
}

/// Records a grade, once per drill; `false` means the drill has a grade already.
///
/// # Errors
///
/// [`KernelError::Database`] when the insert fails.
pub async fn record_grade(
    _connection: &mut SqliteConnection,
    _grade: &GradeRow,
    _at: UtcMillis,
) -> Result<bool, KernelError> {
    Ok(true)
}

/// The newest `limit` grades of `subject`, newest first.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn recent_grades(
    _connection: &mut SqliteConnection,
    _subject: &str,
    _limit: i64,
) -> Result<Vec<GradeRow>, KernelError> {
    Ok(Vec::new())
}
