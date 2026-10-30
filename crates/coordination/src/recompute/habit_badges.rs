//! Phase 7's habit badges step (SPEC-078 R9, R17 as amended; ADR-078): the fold awards each habit
//! badge the evaluated day earns, judged on a context built from the logs through that day, never
//! the wall clock's. Each award goes through SPEC-073's award port with its mark unset, and the
//! offers that drain every unmarked badge celebrate it through the router, once.

use deck_streak_habits::badges::{HabitBadgeContext, earned};
use deck_streak_habits::minutes::{READING_WEEKLY_GOAL_MIN, week_start};
use deck_streak_habits::store;
use deck_streak_habits::writing::{writing_courses, writing_streak};
use deck_streak_kernel::{CourseCode, Courses, KernelError, PortFuture, StudyDay};
use deck_streak_progression::badges::award::{Award, AwardError, NewBadge, award};
use deck_streak_progression::badges::catalog::catalog;
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The habit badges step's name, as the fold's report and log name it.
pub const HABIT_BADGES_STEP: &str = "habits.badges";

/// The first study day the writing log is read from: every confirmation through the evaluated day
/// counts toward its streak.
const FIRST_DAY: StudyDay = StudyDay::from_epoch_day(i64::MIN);

/// Phase 7's habit badges step, over the owner's courses.
#[derive(Debug)]
pub struct HabitBadgesStep {
    courses: Courses,
}

impl HabitBadgesStep {
    /// The step, awarding against `courses`.
    #[must_use]
    pub const fn new(courses: Courses) -> Self {
        Self { courses }
    }
}

impl DayStep for HabitBadgesStep {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        HABIT_BADGES_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            if !day.evaluation.runs_today_only_rules() {
                return Ok(());
            }
            let context = context(&self.courses, day.day, write).await?;
            let catalog = catalog(&self.courses);
            for key in earned(&context) {
                let Some(badge) = catalog.iter().find(|badge| badge.key == key) else {
                    return Err(refused(format!("the catalog has no badge {key}")));
                };
                let new = NewBadge {
                    key,
                    tier: badge.tier,
                    name: &badge.name,
                    emoji: &badge.emoji,
                    study_day: day.day,
                    at: day.facts.now,
                };
                match award(write, &self.courses, &new).await {
                    Ok(Award::Awarded | Award::AlreadyAwarded) => {}
                    Err(AwardError::Database(error)) => return Err(error),
                    Err(AwardError::UnknownKey) => {
                        return Err(refused(format!("the award port refused the key {key}")));
                    }
                }
            }
            Ok(())
        })
    }
}

/// The habit badge context of `day`, from the logs through it: each log's entries, the writing
/// streak, the configured courses read in the day's study week, every course's minutes there, and
/// whether each configured course met the weekly goal.
async fn context(
    courses: &Courses,
    day: StudyDay,
    write: &mut SqliteConnection,
) -> Result<HabitBadgeContext, KernelError> {
    let rows: Vec<_> = store::confirmations_between(write, FIRST_DAY, day)
        .await?
        .into_iter()
        .filter_map(|(code, logged)| CourseCode::new(&code).map(|code| (code, logged)))
        .collect();
    let week = store::minutes_by_code(write, week_start(day), day).await?;
    let configured: Vec<u32> = courses
        .courses()
        .iter()
        .map(|course| {
            week.iter()
                .find(|(code, _)| code == course.code.as_str())
                .map_or(0, |(_, minutes)| *minutes)
        })
        .collect();
    Ok(HabitBadgeContext {
        reading_entries: store::entries_through(write, day).await?,
        writing_entries: store::confirmations_through(write, day).await?,
        writing_all_streak: writing_streak(&writing_courses(courses), &rows, day),
        langs_read_this_week: count(configured.iter().filter(|minutes| **minutes > 0).count()),
        week_total_min: week
            .iter()
            .fold(0, |total: u32, (_, minutes)| total.saturating_add(*minutes)),
        all_langs_goal_met: configured
            .iter()
            .all(|minutes| *minutes >= READING_WEEKLY_GOAL_MIN),
        courses: count(configured.len()),
    })
}

/// A count as the context takes it; one past a `u32` saturates.
fn count(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// A refusal the fold reports as the day's failure.
fn refused(reason: impl std::fmt::Display) -> KernelError {
    KernelError::Database(sqlx::Error::Protocol(format!("{reason}")))
}
