//! Phase 4's progress step (SPEC-077 R6, R7, R10; ADR-077): Road to C2 for the current study day.
//!
//! The step runs for the current study day only, because it reads the current card state and the
//! recompute's clock, never a closed day's. Inside the day's write it:
//!
//! - computes each course's progress with curriculum's rules and stores it in `language_progress`;
//! - counts the law dues, the backlog plus the cards due today over the law track's cards, with
//!   analytics' card snapshot at the day's collection day number, and stores them in `law_dues`;
//! - compares each course's current band with the band stored before it: a first sighting records
//!   a silent baseline, and a band-up records its milestone once and grants its XP through
//!   progression's `grant_on` on the same connection, so the grant is in the day's base before the
//!   derived bonuses, as the predecessor's cycle places it.
//!
//! The band badge is phase 7's (`band_badges`), and the band-up's celebration is offered between the
//! fold's writes from the milestone's unset mark (ADR-303). Every rule is curriculum's, analytics' or
//! progression's; the step only chooses which, for the day the fold evaluates.

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_analytics::snapshot::card_snapshot;
use deck_streak_curriculum::law::LawDues;
use deck_streak_curriculum::progress::{BandStep, XP_BONUS_BAND_UP, band_step, course_progress};
use deck_streak_curriculum::store::{self, NewMilestone, Recorded};
use deck_streak_ingest::calendar::collection_day_number;
use deck_streak_ingest::reader::Card;
use deck_streak_kernel::{Courses, Db, KernelError, PortFuture, StudyDay, Track, UtcMillis};
use deck_streak_progression::grant::{GrantRequest, GrantScope, GrantSource};
use deck_streak_progression::ledger::grant_on;
use deck_streak_progression::xp::XpAmount;
use sqlx::SqliteConnection;

use super::{Celebrate, Celebration, DayEvaluation, DayStep, Evaluation, Phase};

/// The name the fold's report gives this step.
pub const PROGRESS_STEP: &str = "curriculum.progress";

/// Phase 4's progress step, over the owner's courses.
#[derive(Debug)]
pub struct ProgressStep {
    courses: Courses,
    settings: AnalyticsSettings,
}

impl ProgressStep {
    /// The step, computing progress against `courses` and taking the card snapshot with analytics'
    /// `settings`.
    #[must_use]
    pub const fn new(courses: Courses, settings: AnalyticsSettings) -> Self {
        Self { courses, settings }
    }

    /// Stores each course's progress and the law dues for `day`, and records each course's first
    /// sighting or band-up, granting a band-up's XP, all inside `write`.
    async fn record_progress(
        &self,
        day: &DayEvaluation<'_>,
        write: &mut SqliteConnection,
    ) -> Result<(), KernelError> {
        if day.evaluation != Evaluation::Current {
            return Ok(());
        }
        let facts = day.facts;
        let data = facts.data;
        // The bands stored before this recompute are what each course's current band is compared
        // with, so they are read before any course's row is rewritten.
        let stored = store::stored_bands(write).await?;
        let now_sec = facts.now.epoch_millis().div_euclid(1_000);
        let courses = course_progress(&data.cards, &data.deck_names, &self.courses, now_sec);
        for course in &courses {
            let held = stored.get(course.code.as_str()).map(String::as_str);
            match band_step(held, course.current_band) {
                BandStep::FirstSighting(band) => {
                    let baseline = NewMilestone {
                        course: &course.code,
                        band,
                        study_day: day.day,
                        baseline: true,
                        at: facts.now,
                    };
                    // A course seen again after an erase finds its baseline written or not; either
                    // way it owes nothing.
                    let _ = store::record_milestone(write, &baseline).await?;
                }
                BandStep::BandUp(band) => {
                    let reached = NewMilestone {
                        course: &course.code,
                        band,
                        study_day: day.day,
                        baseline: false,
                        at: facts.now,
                    };
                    // Only a band recorded for the first time is paid: a band reached again,
                    // after a drop, pays nothing.
                    if store::record_milestone(write, &reached).await? == Recorded::New {
                        grant_band_up(write, course.code.as_str(), band, day.day, facts.now)
                            .await?;
                    }
                }
                BandStep::Unchanged => {}
            }
            store::put_progress(write, course, facts.now).await?;
        }
        let law: Vec<Card> = data
            .cards
            .iter()
            .filter(|card| card.track == Track::Law)
            .copied()
            .collect();
        let number = collection_day_number(facts.rule, data.created_at, day.day);
        let snapshot = card_snapshot(&law, self.settings.leech_threshold.get(), number);
        let dues = LawDues {
            study_day: day.day,
            backlog: count(snapshot.backlog),
            due_today: count(snapshot.due_today),
        };
        store::put_law_dues(write, &dues, facts.now).await
    }
}

/// Grants a band-up's XP inside the day's write, through progression's `grant_on` and never the
/// grant port, whose `grant` opens a write of its own (ADR-077): once ever, on the language track,
/// with the band lowercased in its source, which the source grammar requires.
async fn grant_band_up(
    write: &mut SqliteConnection,
    course: &str,
    band: &str,
    day: StudyDay,
    now: UtcMillis,
) -> Result<(), KernelError> {
    let token = format!("bandup:{course}:{}", band.to_ascii_lowercase());
    let source = GrantSource::new(&token)
        .map_err(|refused| KernelError::Database(sqlx::Error::Protocol(refused.to_string())))?;
    let amount = u32::try_from(XP_BONUS_BAND_UP).map_err(|_| {
        KernelError::Database(sqlx::Error::Protocol(
            "the band-up's XP is not a grant amount".to_owned(),
        ))
    })?;
    let request = GrantRequest {
        study_day: day,
        source,
        track: Track::Language,
        amount: XpAmount::new(amount),
        scope: GrantScope::Once,
    };
    let _ = grant_on(write, &request, now).await?;
    Ok(())
}

/// A count of cards from the card snapshot, which counts from zero up.
fn count(cards: i64) -> u32 {
    u32::try_from(cards.max(0)).unwrap_or(u32::MAX)
}

impl DayStep for ProgressStep {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        PROGRESS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(self.record_progress(day, write))
    }
}

/// The ladder's event a band-up's celebration is raised as: the notifications policy renders it at
/// T5 and exempts it from the weekly budget (SPEC-084).
pub const BAND_UP_EVENT: &str = "band_up";

/// The router's once-ever dedupe key of the band-up of `course` to `band`: the band lowercased, as
/// the key's grammar requires and as the band-up's grant source spells it (ADR-077).
#[must_use]
pub fn band_up_key(course: &str, band: &str) -> String {
    format!("bandup:{course}:{}", band.to_ascii_lowercase())
}

/// The line a band-up's celebration carries: the course's flag and name, and the band reached.
#[must_use]
pub fn band_up_line(flag: &str, name: &str, band: &str) -> String {
    format!("{flag} {name} reached {band}")
}

/// Offers every band-up whose celebration is still owed to `celebrate`, between the fold's writes
/// (ADR-303, ADR-077). Each is handed to the router, and only once the router has answered is its
/// mark set, in a write of its own that marks it only while it is still unset; a band-up the router
/// did not answer stays owed, and the next offers hand it over again.
///
/// # Errors
///
/// [`KernelError::Database`] when the owed band-ups cannot be read or a mark cannot be written.
pub async fn offer_band_ups(
    celebrate: &dyn Celebrate,
    db: &Db,
    now: UtcMillis,
    today: StudyDay,
) -> Result<(), KernelError> {
    let (owed, courses) = {
        let mut read = db.reader().acquire().await?;
        let owed = store::owed_band_ups(&mut read).await?;
        let courses = store::progress(&mut read).await?;
        (owed, courses)
    };
    for band_up in owed {
        // A course's stored progress names it; one erased since its band-up is named by its code.
        let text = courses
            .iter()
            .find(|course| course.course == band_up.course)
            .map_or_else(
                || format!("{} reached {}", band_up.course, band_up.band),
                |course| band_up_line(&course.flag, &course.name, &band_up.band),
            );
        let celebration = Celebration {
            event: BAND_UP_EVENT,
            key: band_up_key(&band_up.course, &band_up.band),
            text,
            study_day: today,
        };
        // The mark is set only once the router has answered (ADR-303).
        if let Err(error) = celebrate.celebrate(&celebration).await {
            tracing::warn!(key = %celebration.key, %error, "the router did not answer; the band-up stays owed");
            continue;
        }
        let mut write = db.write().await?;
        let _ = store::mark_band_up(&mut write, &band_up.course, &band_up.band, now).await?;
        write.commit().await?;
    }
    Ok(())
}
