//! The landmarks' offers (SPEC-102 R5, R6; ADR-322): the historical landmarks are offered between
//! the fold's writes, as the owed awards are (ADR-303), from a cursor only the router's answers
//! move.
//!
//! Each offer call of one recompute owes, oldest first, every landmark dated after the cursor
//! `landmarks_offered_through` (X) and at or before the settle cursor the call reads (S), and every
//! landmark due on the day evaluated, less the keys the router already answered in this recompute.
//! It hands each to the router, then moves X to S, or to the day before the oldest owed landmark
//! the router did not answer, in a write of its own and never backwards. The first offer call seeds
//! the predecessor's mark and X := yesterday in one write; the recompute whose seed read no mark is
//! the first run, which owes only the first landmark due today on every call and never moves X.
//! `formal/tla/LandmarkOnce` models these steps.

use std::collections::{BTreeSet, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deck_streak_analytics::rollup;
use deck_streak_ingest::reader::CollectionData;
use deck_streak_kernel::{Db, KernelError, PortFuture, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_notifications::landmark_settings;
use deck_streak_notifications::landmarks::{
    ANNIVERSARY_EVENT_TYPE, Landmark, compute_landmarks, due_today, high_water_mark,
    render_landmark,
};

use super::streaks::StudyDays;
use super::{Celebrate, Celebration, Offers};

/// The landmarks' offers of one recompute, over the study days of the whole scoped log.
pub struct LandmarkOffers {
    celebrate: Arc<dyn Celebrate>,
    study_days: Vec<StudyDay>,
    language: BTreeSet<StudyDay>,
    run: Mutex<Run>,
}

/// What one recompute's offer calls share: whether it is the first run, once its seed has read
/// the mark, and the keys the router has answered.
#[derive(Default)]
struct Run {
    first: Option<bool>,
    answered: HashSet<String>,
}

impl LandmarkOffers {
    /// The offers that hand each owed landmark of `study_days`, the whole log's, to `celebrate`.
    /// An anniversary is honest when the window `data` holds no language study day on its day.
    #[must_use]
    pub fn new(
        celebrate: Arc<dyn Celebrate>,
        study_days: Vec<StudyDay>,
        data: &CollectionData,
        rule: StudyDayRule,
    ) -> Self {
        Self {
            celebrate,
            study_days,
            language: StudyDays::from_data(data, rule).language,
            run: Mutex::new(Run::default()),
        }
    }

    fn run(&self) -> MutexGuard<'_, Run> {
        self.run.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether this recompute is the first run: its first call seeds the mark and the cursor in
    /// one write, and every later call keeps that write's answer.
    async fn first(
        &self,
        db: &Db,
        landmarks: &[Landmark],
        now: UtcMillis,
        today: StudyDay,
    ) -> Result<bool, KernelError> {
        if let Some(first) = self.run().first {
            return Ok(first);
        }
        let seed_through = previous(today);
        let first =
            landmark_settings::seed(db, &high_water_mark(landmarks), seed_through, now).await?;
        self.run().first = Some(first);
        Ok(first)
    }

    /// The celebration of `landmark`, raised on `today`.
    fn celebration(&self, landmark: &Landmark, today: StudyDay) -> Celebration {
        let gap_honest =
            landmark.event == ANNIVERSARY_EVENT_TYPE && !self.language.contains(&landmark.day);
        Celebration {
            event: landmark.event,
            key: landmark.key.clone(),
            text: render_landmark(landmark, gap_honest),
            study_day: today,
        }
    }
}

impl fmt::Debug for LandmarkOffers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LandmarkOffers")
            .field("study_days", &self.study_days.len())
            .finish_non_exhaustive()
    }
}

impl Offers for LandmarkOffers {
    fn offer<'a>(&'a self, db: &'a Db, now: UtcMillis, today: StudyDay) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let landmarks = compute_landmarks(&self.study_days, today);
            let first = self.first(db, &landmarks, now, today).await?;
            // X and S on one reader; the advance below uses this S and never reads it again.
            let (through, settled) = {
                let mut read = db.reader().acquire().await?;
                let through = landmark_settings::offered_through(&mut read).await?;
                (through, rollup::settle_cursor(&mut read).await?)
            };
            let due = due_today(&landmarks, today);
            let owed: Vec<&Landmark> = if first {
                due.into_iter().take(1).collect()
            } else if let Some(through) = through {
                landmarks
                    .iter()
                    .filter(|landmark| {
                        let settled_since = landmark.day > through
                            && settled.is_some_and(|settled| landmark.day <= settled);
                        settled_since || landmark.day == today
                    })
                    .collect()
            } else {
                due
            };
            // The owed landmarks are oldest first, so the first one unanswered is the oldest.
            let mut unanswered: Option<StudyDay> = None;
            for landmark in owed {
                if self.run().answered.contains(&landmark.key) {
                    continue;
                }
                let celebration = self.celebration(landmark, today);
                match self.celebrate.celebrate(&celebration).await {
                    Ok(()) => {
                        self.run().answered.insert(landmark.key.clone());
                    }
                    Err(error) => {
                        tracing::warn!(
                            key = %landmark.key,
                            %error,
                            "the router did not answer; the landmark stays owed"
                        );
                        unanswered = unanswered.or(Some(landmark.day));
                    }
                }
            }
            // The first run never moves the cursor, and nothing settled moves it nowhere.
            let (false, Some(settled)) = (first, settled) else {
                return Ok(());
            };
            let target = match unanswered {
                Some(day) => previous(day).min(settled),
                None => settled,
            };
            if through.is_none_or(|through| target > through) {
                landmark_settings::advance(db, target, now).await?;
            }
            Ok(())
        })
    }
}

/// The study day before `day`.
const fn previous(day: StudyDay) -> StudyDay {
    StudyDay::from_epoch_day(day.epoch_day() - 1)
}
