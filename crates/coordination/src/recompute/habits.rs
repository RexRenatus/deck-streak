//! Phase 4's habit step (SPEC-078 R5; ADR-078): the fold settles every course's reading XP of the
//! day from the minutes log, and on a study week's first day that course's weekly bonus from the
//! week's minutes, as the recompute. It heals a habit write whose settle was left undone, and a
//! source the log no longer holds settles to nothing where its row exists.

use std::collections::BTreeSet;

use deck_streak_habits::minutes::{goal_bonus, is_week_start, reading_xp, week_end};
use deck_streak_habits::store;
use deck_streak_kernel::{KernelError, PortFuture, StudyDay, Track, UtcMillis};
use deck_streak_progression::settle::{
    SettleCause, SettleError, SettleRequest, settle, settled_amount, settled_of_day,
};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Evaluation, Phase};

/// The name the fold's report gives this step.
pub const HABITS_STEP: &str = "habits.reading_xp";

/// The prefix of a course's reading XP source.
const READ_PREFIX: &str = "read:";

/// The prefix of a course's weekly bonus source.
const GOAL_PREFIX: &str = "readgoal:";

/// Habits' step.
#[derive(Clone, Copy, Debug, Default)]
pub struct HabitsStep;

impl DayStep for HabitsStep {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        HABITS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let closed = !matches!(day.evaluation, Evaluation::Current);
            let at = day.facts.now;
            let read = minutes_with_held(write, day.day, day.day, READ_PREFIX).await?;
            for (code, minutes) in read {
                let source = format!("{READ_PREFIX}{code}");
                settle_habit(write, day.day, &source, reading_xp(minutes), closed, at).await?;
            }
            if is_week_start(day.day) {
                let week =
                    minutes_with_held(write, day.day, week_end(day.day), GOAL_PREFIX).await?;
                for (code, minutes) in week {
                    let source = format!("{GOAL_PREFIX}{code}");
                    settle_habit(write, day.day, &source, goal_bonus(minutes), closed, at).await?;
                }
            }
            Ok(())
        })
    }
}

/// Every code with minutes from `first` to `last`, with its minutes, and every code `first` holds
/// a `prefix` row for, at nothing when the log no longer has it, in code order.
async fn minutes_with_held(
    write: &mut SqliteConnection,
    first: StudyDay,
    last: StudyDay,
    prefix: &str,
) -> Result<Vec<(String, u32)>, KernelError> {
    let logged = store::minutes_by_code(write, first, last).await?;
    let held: BTreeSet<String> = settled_of_day(write, first)
        .await?
        .into_iter()
        .filter_map(|row| row.source.strip_prefix(prefix).map(str::to_owned))
        .collect();
    let mut codes: BTreeSet<String> = held;
    codes.extend(logged.iter().map(|(code, _)| code.clone()));
    Ok(codes
        .into_iter()
        .map(|code| {
            let minutes = logged
                .iter()
                .find(|(logged_code, _)| *logged_code == code)
                .map_or(0, |(_, minutes)| *minutes);
            (code, minutes)
        })
        .collect())
}

/// Settles `amount` of `source` on `day`, on the language track, as the recompute. A row that
/// does not exist is not written for an amount of nothing.
async fn settle_habit(
    write: &mut SqliteConnection,
    day: StudyDay,
    source: &str,
    amount: u32,
    closed: bool,
    at: UtcMillis,
) -> Result<(), KernelError> {
    if amount == 0
        && settled_amount(write, day, source, Track::Language)
            .await?
            .is_none()
    {
        return Ok(());
    }
    let request = SettleRequest {
        study_day: day,
        source,
        track: Track::Language,
        amount,
        closed,
    };
    match settle(write, &request, SettleCause::Recompute, at).await {
        Ok(_) => Ok(()),
        Err(SettleError::Database(error)) => Err(KernelError::Database(error)),
        Err(SettleError::NotDerived) => Err(KernelError::Database(sqlx::Error::Protocol(
            "the habit step settled a source outside the derived registry".to_owned(),
        ))),
    }
}
