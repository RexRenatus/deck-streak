//! The recompute's fold (SPEC-071 R15 to R19; ADR-071): each closed study day is settled once, in
//! order, with the state it had at its close, and then the current study day is evaluated.
//!
//! After every successful sync the recompute runs the fold over the window it read (SPEC-023):
//!
//! 1. the first recompute, which finds no settled day, rolls up every past study day of the window
//!    in the predecessor's historical form, with no today-only rule ([`Evaluation::Backfill`]);
//! 2. every closed day after the settle cursor is settled, oldest first, once a successful sync has
//!    started after its close ([`Evaluation::Settle`]); the most recently closed day is settled with
//!    its end-of-day state; the cursor is analytics' record, the rollup's `settled_at`;
//! 3. the current study day is evaluated as far as it has gone ([`Evaluation::Current`]);
//! 4. every past study day of the window is revisited: rolled up again only when its reviews changed,
//!    and re-scored in the historical form ([`Evaluation::Revisit`]).
//!
//! Every context registers one [`DayStep`] in one [`Phase`], and a day's steps run in the phases'
//! fixed order, [`PHASES`], declared here once (R19). The fold holds no rule of any context: which
//! day is evaluated how, and in which order, is all it decides. How a later SPEC adds its step,
//! without editing this module: it implements [`DayStep`] for its context in its own file under
//! `recompute/`, and the composition root registers it with [`Fold::register`] in its phase.
//!
//! The awards' celebrations run between the fold's writes, never inside one (SPEC-073 R4, R11;
//! ADR-303): the router opens its own write, so the fold hands every badge and record whose mark is
//! unset to the [`Offers`] it was given before each settled day's write and the current day's
//! write, and once more after its last write. Each offer is its own transaction, so a crash between
//! any two of them leaves every award either marked or still owed.

pub mod analytics_step;
pub mod badges;
pub mod band_badges;
pub mod day_bonuses;
pub mod habits;
pub mod mint;
pub mod progress;
pub mod records;
pub mod streaks;
pub mod xp;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use deck_streak_analytics::rollup::{self, fingerprint};
use deck_streak_ingest::reader::{CollectionData, Review, is_study_event};
use deck_streak_kernel::{
    CourseCode, Db, KernelError, PortFuture, StudyDay, StudyDayRule, UtcMillis,
};
use deck_streak_notifications::{DedupeKey, LapseContext, Occasion, Policy, Router, Surface, Tier};
use sqlx::SqliteConnection;

/// The phases a day's steps run in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// 1. The rollup and the score (analytics, SPEC-071).
    RollupAndScore,
    /// 2. Base XP (progression, SPEC-072).
    BaseXp,
    /// 3. The streaks and the governor (SPEC-076).
    StreaksAndGovernor,
    /// 4. The other contexts' day steps: habits, focus, quests and chests.
    DaySteps,
    /// 5. The derived bonuses: consistency and Ascendant (SPEC-072).
    DerivedBonuses,
    /// 6. The coin mint (economy, SPEC-082).
    CoinMint,
    /// 7. Awards: badges, records and season nodes.
    Awards,
}

/// The phases, in the one order a day's steps run in (R19).
pub const PHASES: [Phase; 7] = [
    Phase::RollupAndScore,
    Phase::BaseXp,
    Phase::StreaksAndGovernor,
    Phase::DaySteps,
    Phase::DerivedBonuses,
    Phase::CoinMint,
    Phase::Awards,
];

impl Phase {
    /// The phase's place in [`PHASES`], from 1.
    #[must_use]
    pub fn number(self) -> usize {
        PHASES
            .iter()
            .position(|phase| *phase == self)
            .map_or(0, |index| index + 1)
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "phase {} ({self:?})", self.number())
    }
}

/// How the fold evaluates a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evaluation {
    /// A past day of the first recompute, in the historical form: no today-only rule runs (R17).
    Backfill {
        /// Whether the day's reviews differ from what its rollup was made of (R18).
        reviews_changed: bool,
    },
    /// The day's one settle (R16, R17).
    Settle {
        /// Whether it is the most recently closed day, which is settled with its end-of-day state.
        end_of_day: bool,
    },
    /// The current study day, as far as it has gone.
    Current,
    /// A past day after its settle or backfill: rolled up again only when its reviews changed (R18),
    /// re-scored in the historical form (R14).
    Revisit {
        /// Whether the day's reviews differ from what its rollup was made of.
        reviews_changed: bool,
    },
}

impl Evaluation {
    /// Whether the predecessor's today-only rules run for the day: at its settle, and while it is
    /// the current day (ADR-071).
    #[must_use]
    pub const fn runs_today_only_rules(self) -> bool {
        matches!(self, Self::Settle { .. } | Self::Current)
    }

    /// Whether the day's reviews are rolled up again (R18): at its settle, while it is current, and
    /// when its reviews changed.
    #[must_use]
    pub const fn rerolls(self) -> bool {
        match self {
            Self::Settle { .. } | Self::Current => true,
            Self::Backfill { reviews_changed } | Self::Revisit { reviews_changed } => {
                reviews_changed
            }
        }
    }
}

/// What every evaluation of one recompute shares: the window read, and its shape by day and card.
#[derive(Debug)]
pub struct RecomputeFacts<'a> {
    /// The window the recompute read.
    pub data: &'a CollectionData,
    /// The study-day rule.
    pub rule: StudyDayRule,
    /// When the recompute runs.
    pub now: UtcMillis,
    /// The current study day.
    pub today: StudyDay,
    /// The digest of the owner's courses, which every day's fingerprint carries.
    pub courses_digest: Option<&'a str>,
    reviews_by_day: BTreeMap<StudyDay, Vec<Review>>,
    card_decks: BTreeMap<i64, i64>,
    card_courses: BTreeMap<i64, CourseCode>,
    base_reviews: u64,
}

impl<'a> RecomputeFacts<'a> {
    /// The facts of a recompute over `data` at `now`.
    #[must_use]
    pub fn new(
        data: &'a CollectionData,
        rule: StudyDayRule,
        now: UtcMillis,
        courses_digest: Option<&'a str>,
    ) -> Self {
        let mut reviews_by_day: BTreeMap<StudyDay, Vec<Review>> = BTreeMap::new();
        for review in &data.reviews {
            let day = rule.study_day(UtcMillis::from_epoch_millis(review.id));
            reviews_by_day.entry(day).or_default().push(*review);
        }
        Self {
            data,
            rule,
            now,
            today: rule.study_day(now),
            courses_digest,
            reviews_by_day,
            card_decks: data
                .cards
                .iter()
                .map(|card| (card.id, card.home_deck_id()))
                .collect(),
            card_courses: data
                .cards
                .iter()
                .filter_map(|card| card.course.map(|course| (card.id, course)))
                .collect(),
            base_reviews: 0,
        }
    }

    /// These facts, with the study reviews at or before the window's floor that ingest counted
    /// (SPEC-023 R6): the lifetime the badges and the milestone read starts from them.
    #[must_use]
    pub const fn with_base_reviews(mut self, base_reviews: u64) -> Self {
        self.base_reviews = base_reviews;
        self
    }

    /// The lifetime study reviews through `day` (SPEC-073 R5, R15): the window's base, and every
    /// study review of the window on or before `day`.
    #[must_use]
    pub fn lifetime_through(&self, day: StudyDay) -> u64 {
        let window = self
            .reviews_by_day
            .range(..=day)
            .flat_map(|(_, reviews)| reviews)
            .filter(|review| is_study_event(review.kind, review.ease))
            .count();
        self.base_reviews
            .saturating_add(u64::try_from(window).unwrap_or(u64::MAX))
    }

    /// The study reviews answered on `day`, in the window's order.
    #[must_use]
    pub fn reviews_of(&self, day: StudyDay) -> &[Review] {
        self.reviews_by_day.get(&day).map_or(&[], Vec::as_slice)
    }

    /// The study days the window holds a review on.
    #[must_use]
    pub fn study_days(&self) -> BTreeSet<StudyDay> {
        self.reviews_by_day.keys().copied().collect()
    }

    /// Each card's home deck.
    #[must_use]
    pub const fn card_decks(&self) -> &BTreeMap<i64, i64> {
        &self.card_decks
    }

    /// Each card's course, for the cards that have one.
    #[must_use]
    pub const fn card_courses(&self) -> &BTreeMap<i64, CourseCode> {
        &self.card_courses
    }
}

/// One day as the fold hands it to a step.
#[derive(Debug)]
pub struct DayEvaluation<'a> {
    /// The day evaluated.
    pub day: StudyDay,
    /// How it is evaluated.
    pub evaluation: Evaluation,
    /// What the recompute shares.
    pub facts: &'a RecomputeFacts<'a>,
}

/// One context's work for one day, run in its phase, inside the day's write.
pub trait DayStep: Send + Sync {
    /// The phase the step belongs to.
    fn phase(&self) -> Phase;

    /// The step's name, as the fold's report and log name it.
    fn name(&self) -> &'static str;

    /// Evaluates `day` inside `write`, the transaction the fold holds for the day.
    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()>;
}

/// One celebration an award owes (SPEC-073 R4, R11): the ladder's event, the once-ever dedupe key,
/// the line, and the study day it is raised on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Celebration {
    /// The ladder's event: `badge`, `record` or `band_up`.
    pub event: &'static str,
    /// The dedupe key: `badge:<key>:<tier>`, `pr:<kind>:<epoch day>` or `bandup:<code>:<band>`.
    pub key: String,
    /// The line.
    pub text: String,
    /// The study day the celebration is raised on.
    pub study_day: StudyDay,
}

/// The port a celebration is handed to (SPEC-073 R4, R11): the router, in production.
///
/// `Ok` means the router answered, whatever it decided (sent, deferred or withheld), so the award's
/// mark may be set; an error means no answer, and the award stays owed. The router's once-ever
/// dedupe key keeps a key offered twice to one send.
pub trait Celebrate: Send + Sync {
    /// Hands `celebration` to the router and waits for its answer.
    fn celebrate<'a>(&'a self, celebration: &'a Celebration) -> PortFuture<'a, ()>;
}

/// The offers the fold runs between its writes (ADR-303): every badge, record and band-up whose mark
/// is unset is handed to the router, and each one it answered is marked in a write of its own.
pub trait Offers: fmt::Debug + Send + Sync {
    /// Offers every owed celebration through `db` at `now`, raised on the study day `today`.
    fn offer<'a>(&'a self, db: &'a Db, now: UtcMillis, today: StudyDay) -> PortFuture<'a, ()>;
}

/// The awards' offers: the owed badges, then the owed records, then the owed band-ups (ADR-077),
/// each through `celebrate`.
pub struct AwardOffers {
    celebrate: std::sync::Arc<dyn Celebrate>,
}

impl AwardOffers {
    /// The offers that hand each owed celebration to `celebrate`.
    #[must_use]
    pub fn new(celebrate: std::sync::Arc<dyn Celebrate>) -> Self {
        Self { celebrate }
    }
}

impl fmt::Debug for AwardOffers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AwardOffers").finish_non_exhaustive()
    }
}

impl Offers for AwardOffers {
    fn offer<'a>(&'a self, db: &'a Db, now: UtcMillis, today: StudyDay) -> PortFuture<'a, ()> {
        Box::pin(async move {
            // An offer that fails is retried by the next offers; it never stops the fold.
            if let Err(error) = badges::offer_badges(&*self.celebrate, db, now, today).await {
                tracing::error!(%error, "the owed badges could not be offered");
            }
            if let Err(error) = records::offer_records(&*self.celebrate, db, now, today).await {
                tracing::error!(%error, "the owed records could not be offered");
            }
            if let Err(error) = progress::offer_band_ups(&*self.celebrate, db, now, today).await {
                tracing::error!(%error, "the owed band-ups could not be offered");
            }
            Ok(())
        })
    }
}

/// The router as the awards' [`Celebrate`] port: a `celebration` occasion with the award's event,
/// on the bot (SPEC-041, SPEC-084).
impl Celebrate for Router {
    fn celebrate<'a>(&'a self, celebration: &'a Celebration) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let policy = Policy::compiled().map_err(refused)?;
            let kind = policy
                .kind(CELEBRATION_KIND)
                .ok_or_else(|| refused("the policy has no celebration kind"))?;
            let key = DedupeKey::new(&celebration.key).map_err(refused)?;
            let occasion = Occasion::new(
                kind,
                key,
                Surface::Bot,
                Tier::T2,
                celebration.text.clone(),
                celebration.study_day,
                LapseContext::NoLapse,
            )
            .map_err(refused)?
            .with_event(celebration.event, None);
            self.route(&occasion).await.map(|_| ())
        })
    }
}

/// The policy's kind an award's celebration is raised as.
const CELEBRATION_KIND: &str = "celebration";

/// A refusal the offer cannot recover from, as the database error kind the cycle already reports.
fn refused(reason: impl fmt::Display) -> KernelError {
    KernelError::Database(sqlx::Error::Protocol(format!("{reason}")))
}

/// Why a step cannot be registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FoldError {
    /// The step belongs to another phase than the one it was registered in.
    #[error("the step {step} belongs to {declared} and was registered in {registered}")]
    OutsidePhase {
        /// The step's name.
        step: &'static str,
        /// The phase it belongs to.
        declared: Phase,
        /// The phase it was registered in.
        registered: Phase,
    },
}

/// What one recompute's fold is given.
#[derive(Clone, Copy, Debug)]
pub struct FoldInput<'a> {
    /// The window the recompute read.
    pub data: &'a CollectionData,
    /// The study-day rule.
    pub rule: StudyDayRule,
    /// When the recompute runs.
    pub now: UtcMillis,
    /// The study day in which the latest successful sync started, if any: a day closed before it
    /// may be settled (R15).
    pub synced_in: Option<StudyDay>,
    /// The digest of the owner's courses.
    pub courses_digest: Option<&'a str>,
    /// The study reviews at or before the window's floor (SPEC-023 R6), the lifetime's start.
    pub base_reviews: u64,
    /// The offers run between the fold's writes (ADR-303); `None` runs none.
    pub offers: Option<&'a dyn Offers>,
}

/// What one recompute's fold did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoldReport {
    /// The past days rolled up in the historical form by the first recompute, oldest first.
    pub backfilled: Vec<StudyDay>,
    /// The days settled, oldest first.
    pub settled: Vec<StudyDay>,
    /// The current study day, evaluated.
    pub current: Option<StudyDay>,
    /// Every day whose reviews were rolled up again (R18), oldest first.
    pub rerolled: Vec<StudyDay>,
    /// How many past days were revisited.
    pub revisited: usize,
}

/// The fold: every registered step, run in the phases' order for each day evaluated.
#[derive(Default)]
pub struct Fold {
    steps: Vec<Box<dyn DayStep>>,
}

impl fmt::Debug for Fold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.steps.iter().map(|step| (step.phase(), step.name())))
            .finish()
    }
}

impl Fold {
    /// Registers `step` in `phase`.
    ///
    /// # Errors
    ///
    /// [`FoldError::OutsidePhase`] when the step belongs to another phase.
    pub fn register(&mut self, phase: Phase, step: Box<dyn DayStep>) -> Result<(), FoldError> {
        if step.phase() != phase {
            return Err(FoldError::OutsidePhase {
                step: step.name(),
                declared: step.phase(),
                registered: phase,
            });
        }
        // Kept in the phases' order; steps of one phase keep their registration order.
        let at = self
            .steps
            .iter()
            .position(|registered| registered.phase() > phase)
            .unwrap_or(self.steps.len());
        self.steps.insert(at, step);
        Ok(())
    }

    /// Every registered step, as `(phase, name)`, in the order a day runs them.
    #[must_use]
    pub fn steps(&self) -> Vec<(Phase, &'static str)> {
        self.steps
            .iter()
            .map(|step| (step.phase(), step.name()))
            .collect()
    }

    /// Runs the fold over `input` against `db` (R15 to R18).
    ///
    /// # Errors
    ///
    /// [`KernelError`] when a read or a write of a step or of the cursor fails; the days settled
    /// before it stay settled.
    pub async fn run(&self, db: &Db, input: &FoldInput<'_>) -> Result<FoldReport, KernelError> {
        let facts = RecomputeFacts::new(input.data, input.rule, input.now, input.courses_digest)
            .with_base_reviews(input.base_reviews);
        let today = facts.today;
        let closed = previous(today);
        let study_days: Vec<StudyDay> = facts
            .study_days()
            .into_iter()
            .filter(|day| *day < today)
            .collect();
        let mut report = FoldReport::default();

        let mut write = db.write().await?;
        let cursor = rollup::settle_cursor(&mut write).await?;
        let stored = rollup::fingerprints(&mut write).await?;
        // (1) The first recompute: every past study day before the closing one, historically.
        let first_owed = if let Some(cursor) = cursor {
            next(cursor)
        } else {
            for day in study_days.iter().copied().filter(|day| *day < closed) {
                let reviews_changed = changed(&facts, &stored, day);
                let evaluation = Evaluation::Backfill { reviews_changed };
                self.evaluate(&facts, day, evaluation, &mut write).await?;
                report.backfilled.push(day);
                if reviews_changed {
                    report.rerolled.push(day);
                }
            }
            closed
        };
        write.commit().await?;

        // (2) Each owed day, oldest first, once a successful sync started after its close (R15).
        let mut day = first_owed;
        while day <= closed && input.synced_in.is_some_and(|synced| day < synced) {
            // Every owed celebration is offered before the day's write can replace it (ADR-303).
            offer_owed(input, db, today).await;
            let mut write = db.write().await?;
            // Another fold may have settled since this one read the cursor: the cursor read again
            // inside this write decides the day owed, the day after it or, with none, this day
            // (R16, ADR-313). A write for any other day commits nothing.
            let owed = rollup::settle_cursor(&mut write).await?.map_or(day, next);
            if owed != day {
                write.rollback().await?;
                day = owed;
                continue;
            }
            let end_of_day = day == closed;
            self.evaluate(&facts, day, Evaluation::Settle { end_of_day }, &mut write)
                .await?;
            // The cursor moves in the same write as the day's steps (R16).
            if !rollup::record_settled(&mut write, day, input.now).await? {
                tracing::error!(%day, "a settled day has no rollup: phase 1's step is missing");
            }
            write.commit().await?;
            report.settled.push(day);
            report.rerolled.push(day);
            day = next(day);
        }

        // (3) The current study day, as far as it has gone.
        offer_owed(input, db, today).await;
        let mut write = db.write().await?;
        self.evaluate(&facts, today, Evaluation::Current, &mut write)
            .await?;
        write.commit().await?;
        report.current = Some(today);

        // (4) Every past study day of the window that is not owed, and the days just settled:
        // rolled up again only when their reviews changed, and re-scored (R14, R18).
        let mut write = db.write().await?;
        let settled_through = report.settled.last().copied().or(cursor);
        let stored = rollup::fingerprints(&mut write).await?;
        let revisit: BTreeSet<StudyDay> = study_days
            .iter()
            .copied()
            .filter(|day| {
                report.backfilled.contains(day) || settled_through.is_some_and(|last| *day <= last)
            })
            .chain(report.settled.iter().copied())
            .collect();
        for day in revisit {
            let reviews_changed = changed(&facts, &stored, day);
            let evaluation = Evaluation::Revisit { reviews_changed };
            self.evaluate(&facts, day, evaluation, &mut write).await?;
            report.revisited += 1;
            if reviews_changed {
                report.rerolled.push(day);
            }
        }
        write.commit().await?;
        // The awards the last writes made are offered once they are committed.
        offer_owed(input, db, today).await;
        report.rerolled.push(today);
        report.rerolled.sort_unstable();
        report.rerolled.dedup();
        tracing::info!(
            backfilled = report.backfilled.len(),
            settled = report.settled.len(),
            revisited = report.revisited,
            rerolled = report.rerolled.len(),
            "the recompute's fold ran"
        );
        Ok(report)
    }

    /// Runs every step for `day`, in the phases' order, inside `write`.
    async fn evaluate(
        &self,
        facts: &RecomputeFacts<'_>,
        day: StudyDay,
        evaluation: Evaluation,
        write: &mut SqliteConnection,
    ) -> Result<(), KernelError> {
        let evaluated = DayEvaluation {
            day,
            evaluation,
            facts,
        };
        for step in &self.steps {
            step.evaluate(&evaluated, write).await?;
        }
        Ok(())
    }
}

/// Runs the fold's offers, if it was given any, between two of its writes: a failed offer is logged
/// and leaves its award owed, and never fails the fold (ADR-303).
async fn offer_owed(input: &FoldInput<'_>, db: &Db, today: StudyDay) {
    if let Some(offers) = input.offers
        && let Err(error) = offers.offer(db, input.now, today).await
    {
        tracing::error!(%error, "the owed celebrations could not be offered");
    }
}

/// Whether `day`'s reviews differ from what its stored rollup was made of (R18).
fn changed(facts: &RecomputeFacts<'_>, stored: &BTreeMap<StudyDay, String>, day: StudyDay) -> bool {
    let print = fingerprint(facts.reviews_of(day), facts.courses_digest);
    stored.get(&day) != Some(&print)
}

/// The study day before `day`.
const fn previous(day: StudyDay) -> StudyDay {
    StudyDay::from_epoch_day(day.epoch_day() - 1)
}

/// The study day after `day`.
const fn next(day: StudyDay) -> StudyDay {
    StudyDay::from_epoch_day(day.epoch_day() + 1)
}
