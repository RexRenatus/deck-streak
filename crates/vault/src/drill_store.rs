//! The vault's two drill tables (SPEC-110 R6, R11; `migrations/011001_vault_drills.sql`): the
//! answer row that makes an answer once-only, and the grade row the post-back records before it
//! asks the grant port to pay.

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

/// Records the answer of `drill_id`, and says whether this call made the row: `false` means the
/// drill has an answer already.
///
/// # Errors
///
/// [`KernelError::Database`] when the insert fails.
pub async fn insert_answer(
    connection: &mut SqliteConnection,
    drill_id: &str,
    day: StudyDay,
    surface: Surface,
    at: UtcMillis,
) -> Result<bool, KernelError> {
    let day = day.epoch_day();
    let surface = surface.as_str();
    let at = at.epoch_millis();
    let done = sqlx::query!(
        "INSERT INTO drill_answers (drill_id, study_day, surface, created_at)
         VALUES (?1, ?2, ?3, ?4) ON CONFLICT (drill_id) DO NOTHING",
        drill_id,
        day,
        surface,
        at
    )
    .execute(connection)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Whether `drill_id` has an answer row.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn has_answer(
    connection: &mut SqliteConnection,
    drill_id: &str,
) -> Result<bool, KernelError> {
    let row = sqlx::query!(
        r#"SELECT count(*) AS "n!: i64" FROM drill_answers WHERE drill_id = ?1"#,
        drill_id
    )
    .fetch_one(connection)
    .await?;
    Ok(row.n > 0)
}

/// Records a grade, once per drill, and says whether this call made the row. A second call for the
/// same drill changes nothing, so a poll may be repeated.
///
/// # Errors
///
/// [`KernelError::Database`] when the insert fails, including an XP outside 10 to 25.
pub async fn record_grade(
    connection: &mut SqliteConnection,
    grade: &GradeRow,
    at: UtcMillis,
) -> Result<bool, KernelError> {
    let day = grade.study_day.epoch_day();
    let at = at.epoch_millis();
    let done = sqlx::query!(
        "INSERT INTO drill_grades (drill_id, drill_type, subject, xp, study_day, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT (drill_id) DO NOTHING",
        grade.drill_id,
        grade.drill_type,
        grade.subject,
        grade.xp,
        day,
        at
    )
    .execute(connection)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// The newest `limit` grades of `subject`, newest first (the memory port's read, SPEC-044 R7).
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn recent_grades(
    connection: &mut SqliteConnection,
    subject: &str,
    limit: i64,
) -> Result<Vec<GradeRow>, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT drill_id AS "drill_id!", drill_type, subject, xp, study_day
           FROM drill_grades WHERE subject = ?1
           ORDER BY created_at DESC, rowid DESC LIMIT ?2"#,
        subject,
        limit
    )
    .fetch_all(connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| GradeRow {
            drill_id: row.drill_id,
            drill_type: row.drill_type,
            subject: row.subject,
            xp: row.xp,
            study_day: StudyDay::from_epoch_day(row.study_day),
        })
        .collect())
}
