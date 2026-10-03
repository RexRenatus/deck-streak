//! The study days of the whole scoped log (SPEC-102 R1, ADR-318): every day on which a review of
//! a card in scope is a study event, read once each and oldest first, with no window.

use std::collections::BTreeSet;

use deck_streak_kernel::{StudyDay, StudyDayRule, UtcMillis};

use crate::reader::{
    CollectionReader, ReadError, allowed_deck_ids, deck_names, read_failed, scope_ids,
};

/// The review ids above the floor `?1` that are study events of the cards whose home deck is one
/// of the ids in `?2`, oldest first, `?3` at a time: the SQL mirror of `reader::is_study_event`
/// and `reader::Card::home_deck_id`, as `window.rs` writes its own count.
const STUDY_EVENT_IDS: &str = "SELECT r.id FROM revlog r JOIN cards c ON c.id = r.cid \
     WHERE r.id > ?1 AND r.type IN (0, 1, 2, 3) AND r.ease >= 1 \
     AND (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) IN (SELECT value FROM json_each(?2)) \
     ORDER BY r.id LIMIT ?3";

/// How many review ids one page of the read holds.
const PAGE_SIZE: i64 = 50_000;

impl CollectionReader {
    /// Every study day of a scoped study event in the whole log, once each, oldest first.
    ///
    /// # Errors
    /// [`ReadError`] when the private copy cannot be read.
    pub async fn study_days(&self, rule: StudyDayRule) -> Result<Vec<StudyDay>, ReadError> {
        let prefixes = self.scope().include().prefixes().to_vec();
        self.with_copy("read_study_days", move |copy| async move {
            let decks = deck_names(&copy).await?;
            let scope = scope_ids(&allowed_deck_ids(&decks, &prefixes));
            let mut days = BTreeSet::new();
            let mut floor = 0_i64;
            loop {
                let page: Vec<i64> = sqlx::query_scalar(STUDY_EVENT_IDS)
                    .bind(floor)
                    .bind(&scope)
                    .bind(PAGE_SIZE)
                    .fetch_all(copy.reader())
                    .await
                    .map_err(read_failed)?;
                let Some(&last) = page.last() else {
                    break;
                };
                for id in &page {
                    days.insert(rule.study_day(UtcMillis::from_epoch_millis(*id)));
                }
                floor = last;
            }
            Ok(days.into_iter().collect())
        })
        .await
    }
}
