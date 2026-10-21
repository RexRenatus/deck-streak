//! The law-tiers source the API serves (SPEC-072 R24): the in-scope law-track cards counted by
//! Bloom tier, and the day's law review XP split by each review's card tier.
//!
//! It reads the collection through ingest's reader, the one door to the private copy, and prices
//! each review with progression's `review_xp`, the function the fold's XP step prices with, so the
//! split cannot drift from what `reviews_law` settled. Coordination owns it because the
//! [`LawTierSource`] trait is coordination's and this crate already sees ingest and progression.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use deck_streak_ingest::reader::{CollectionReader, ReadError};
use deck_streak_ingest::tier::Tier;
use deck_streak_kernel::{KernelError, StudyDay, StudyDayRule, Track, UtcMillis};
use deck_streak_progression::review_xp::review_xp;

use super::level_view::{LawTierSource, LawTiers};

/// One day, in milliseconds.
const DAY_MS: i64 = 86_400_000;
/// How many days before the study day the read reaches back, so that a rollover hour and a UTC
/// offset, whichever way they move the day's edge, leave none of its reviews behind.
const READ_MARGIN_DAYS: i64 = 2;

/// The slot of `tier` in [`LawTiers`]: `T1` to `T4`, then the cards with none.
const fn slot(tier: Option<Tier>) -> usize {
    match tier {
        Some(tier) => tier as usize,
        None => 4,
    }
}

/// The law tiers, read from the collection copy `reader` names, with study days decided by `rule`.
#[derive(Clone, Debug)]
pub struct CollectionLawTiers {
    reader: CollectionReader,
    rule: StudyDayRule,
}

impl CollectionLawTiers {
    /// A source that reads through `reader` and places each review on a day by `rule`.
    #[must_use]
    pub const fn new(reader: CollectionReader, rule: StudyDayRule) -> Self {
        Self { reader, rule }
    }
}

/// A read's refusal, as the kernel's error the route answers 500 for.
fn refused(error: ReadError) -> KernelError {
    match error {
        ReadError::Copy(error) => error,
        ReadError::Lock(source) => KernelError::Database(sqlx::Error::Io(source)),
        ReadError::WriteRefused => KernelError::Database(sqlx::Error::Protocol(
            "the collection copy refused a write".to_owned(),
        )),
    }
}

impl LawTierSource for CollectionLawTiers {
    fn law_tiers<'a>(
        &'a self,
        today: StudyDay,
    ) -> Pin<Box<dyn Future<Output = Result<LawTiers, KernelError>> + Send + 'a>> {
        Box::pin(async move {
            let floor = today
                .epoch_day()
                .saturating_sub(READ_MARGIN_DAYS)
                .saturating_mul(DAY_MS);
            let data = self.reader.read(floor).await.map_err(refused)?;
            let mut tiers = LawTiers::default();
            let mut law_cards: HashMap<i64, Option<Tier>> = HashMap::new();
            for card in data.cards.iter().filter(|card| card.track == Track::Law) {
                tiers.cards[slot(card.tier)] += 1;
                law_cards.insert(card.id, card.tier);
            }
            for review in &data.reviews {
                let day = self.rule.study_day(UtcMillis::from_epoch_millis(review.id));
                if day != today {
                    continue;
                }
                if let Some(tier) = law_cards.get(&review.card_id) {
                    tiers.xp[slot(*tier)] += u64::from(review_xp(review, *tier));
                }
            }
            Ok(tiers)
        })
    }
}
