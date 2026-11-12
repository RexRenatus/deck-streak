//! Phase 4's writing step (SPEC-078 R6 to R8; ADR-078): the fold settles each evaluated day's
//! writing XP from the writing log, every writing course's `write:<code>` and the day's
//! `write:all`, as the recompute. It heals a writing write whose settle was left undone, and a
//! source the log or the courses no longer hold settles to nothing where its row exists.

use std::collections::BTreeMap;

use deck_streak_habits::store;
use deck_streak_habits::writing::{
    WRITE_ALL_SOURCE, write_source, writing_courses, writing_day_xp,
};
use deck_streak_kernel::{CourseCode, Courses, PortFuture};
use deck_streak_progression::settle::settled_of_day;
use sqlx::SqliteConnection;

use super::habits::settle_habit;
use super::{DayEvaluation, DayStep, Evaluation, Phase};

/// The name the fold's report gives this step.
pub const WRITING_STEP: &str = "habits.writing_xp";

/// The prefix of every writing source.
const WRITE_PREFIX: &str = "write:";

/// The writing step, over the owner's courses.
#[derive(Debug)]
pub struct WritingStep {
    courses: Courses,
}

impl WritingStep {
    /// The step, settling the writing courses of `courses`.
    #[must_use]
    pub const fn new(courses: Courses) -> Self {
        Self { courses }
    }
}

impl DayStep for WritingStep {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        WRITING_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let closed = !matches!(day.evaluation, Evaluation::Current);
            let at = day.facts.now;
            let rows: Vec<_> = store::confirmations_between(write, day.day, day.day)
                .await?
                .into_iter()
                .filter_map(|(code, logged)| CourseCode::new(&code).map(|code| (code, logged)))
                .collect();
            let xp = writing_day_xp(&writing_courses(&self.courses), day.day, &rows);
            let mut sources: BTreeMap<String, u32> = settled_of_day(write, day.day)
                .await?
                .into_iter()
                .filter(|row| row.source.starts_with(WRITE_PREFIX))
                .map(|row| (row.source, 0))
                .collect();
            sources.extend(
                xp.courses
                    .iter()
                    .map(|(code, amount)| (write_source(*code), *amount)),
            );
            sources.insert(WRITE_ALL_SOURCE.to_owned(), xp.all);
            for (source, amount) in sources {
                settle_habit(write, day.day, &source, amount, closed, at).await?;
            }
            Ok(())
        })
    }
}
