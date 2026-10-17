//! The law-tiers source over a seeded collection copy (SPEC-072 R24, A31): the law-track cards
//! counted by tier and the study day's law review XP split by each review's card tier, priced by
//! progression's `review_xp`.
//!
//! The collection is the engine's own empty copy with synthetic notes, cards and reviews inserted
//! by hand into its default deck, which the law root setting names.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use deck_streak_coordination::progression::law_tiers::CollectionLawTiers;
use deck_streak_coordination::progression::level_view::{LawTierSource, LawTiers};
use deck_streak_ingest::engine::{AnkiEngine, RslibEngine};
use deck_streak_ingest::reader::{CollectionReader, Review};
use deck_streak_ingest::settings::{
    LAW_DECK_ROOT, STATE_DIRECTORY, SYNC_ENDPOINT, ScopeSettings, SyncSettings,
};
use deck_streak_ingest::tier::Tier;
use deck_streak_kernel::{
    Clock, Environment, ManualClock, Offload, OffloadWorkers, StudyDay, StudyDayRule, UtcMillis,
};
use deck_streak_progression::review_xp::review_xp;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, Connection};

/// 2025-01-14T12:00:00Z, noon of the study day under test.
const NOON_TODAY: i64 = 1_736_856_000_000;
/// The study day of [`NOON_TODAY`], as an epoch day.
const TODAY: i64 = 20_102;
/// One day, in milliseconds.
const DAY_MS: i64 = 86_400_000;
/// The engine's default deck, which the law root names here.
const LAW_ROOT: &str = "Default";

/// The cards of the seeded collection: (id, the note's tags, the tier the reader must find).
const CARDS: [(i64, &str, Option<Tier>); 7] = [
    (1001, "T1", Some(Tier::T1)),
    (1002, "t1 vocab", Some(Tier::T1)),
    (1003, "T2", Some(Tier::T2)),
    (1004, "T3", Some(Tier::T3)),
    (1005, "T4", Some(Tier::T4)),
    (1006, "vocab", None),
    (1007, "", None),
];

/// The reviews of the seeded collection: (offset from noon today in days, the card).
const REVIEWS: [(i64, i64); 9] = [
    (0, 1001),
    (0, 1002),
    (0, 1003),
    (0, 1003),
    (0, 1004),
    (0, 1005),
    (0, 1006),
    // Two reviews of the day before, which today's XP leaves out.
    (-1, 1001),
    (-1, 1005),
];

/// The review a seeded row stands for, as the reader hands it back.
fn review_of(place: usize, days: i64, card: i64) -> Review {
    Review {
        id: NOON_TODAY + days * DAY_MS + i64::try_from(place).expect("a small index"),
        card_id: card,
        ease: 3,
        interval: 10,
        last_interval: 5,
        factor: 2500,
        taken_ms: 4000,
        kind: 1,
    }
}

/// Inserts the synthetic notes, cards and reviews into the copy at `path`.
async fn seed(path: &Path) {
    let mut connection = SqliteConnectOptions::from_str("sqlite://")
        .expect("options")
        .filename(path)
        .connect()
        .await
        .expect("the copy opens for writing");
    for (id, tags, _) in CARDS {
        sqlx::query(
            "INSERT INTO notes (id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data) \
             VALUES (?1, ?2, 1, 0, 0, ?3, 'front\u{1f}back', 'front', 0, 0, '')",
        )
        .bind(id)
        .bind(format!("guid{id}"))
        .bind(format!(" {tags} "))
        .execute(&mut connection)
        .await
        .expect("a note");
        sqlx::query(
            "INSERT INTO cards (id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, \
             reps, lapses, left, odue, odid, flags, data) \
             VALUES (?1, ?1, 1, 0, 0, 0, 2, 2, 100, 10, 2500, 1, 0, 0, 0, 0, 0, '{}')",
        )
        .bind(id)
        .execute(&mut connection)
        .await
        .expect("a card");
    }
    for (place, (days, card)) in REVIEWS.into_iter().enumerate() {
        let review = review_of(place, days, card);
        sqlx::query(
            "INSERT INTO revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
             VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(review.id)
        .bind(review.card_id)
        .bind(review.ease)
        .bind(review.interval)
        .bind(review.last_interval)
        .bind(review.factor)
        .bind(review.taken_ms)
        .bind(review.kind)
        .execute(&mut connection)
        .await
        .expect("a review");
    }
    connection.close().await.expect("the copy closes");
}

/// The tier of the card `card` of the seeded collection.
fn tier_of(card: i64) -> Option<Tier> {
    CARDS
        .iter()
        .find(|(id, _, _)| *id == card)
        .and_then(|(_, _, tier)| *tier)
}

/// The place of `tier` in the view: `T1` to `T4`, then none.
const fn slot(tier: Option<Tier>) -> usize {
    match tier {
        Some(tier) => tier as usize,
        None => 4,
    }
}

/// The source over a seeded copy, its law root `law_root`.
async fn source(scratch: &tempfile::TempDir, law_root: Option<&str>) -> CollectionLawTiers {
    let state = scratch.path().join("state");
    std::fs::create_dir_all(&state).expect("a state directory");
    let mut variables = vec![
        (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
        (STATE_DIRECTORY, state.as_os_str()),
    ];
    if let Some(root) = &law_root {
        variables.push((LAW_DECK_ROOT, OsStr::new(root)));
    }
    let env = Environment::from_vars(variables);
    let settings = SyncSettings::from_env(&env).expect("the settings");
    RslibEngine
        .new_card_queue(&settings.copy_path())
        .expect("the engine creates the copy");
    seed(&settings.copy_path()).await;
    let clock: Arc<dyn Clock> =
        Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(NOON_TODAY)));
    let offload = Offload::new(OffloadWorkers::new(1).expect("one worker"), clock);
    let scope = ScopeSettings::from_env(&env).expect("the scope");
    CollectionLawTiers::new(
        CollectionReader::new(&settings, scope, offload),
        StudyDayRule::default(),
    )
}

#[tokio::test]
async fn the_law_cards_are_counted_by_tier_and_today_s_reviews_are_priced_by_their_card() {
    let scratch = tempfile::tempdir().expect("a scratch");
    let source = source(&scratch, Some(LAW_ROOT)).await;
    let mut cards = [0_u64; 5];
    for (_, _, tier) in CARDS {
        cards[slot(tier)] += 1;
    }
    let mut xp = [0_u64; 5];
    for (place, (days, card)) in REVIEWS.into_iter().enumerate() {
        if days == 0 {
            let tier = tier_of(card);
            xp[slot(tier)] += u64::from(review_xp(&review_of(place, days, card), tier));
        }
    }
    assert!(
        xp.iter().all(|amount| *amount > 0),
        "every tier earned: {xp:?}"
    );
    let tiers = source
        .law_tiers(StudyDay::from_epoch_day(TODAY))
        .await
        .expect("the tiers are read");
    assert_eq!(tiers, LawTiers { cards, xp });
}

#[tokio::test]
async fn a_collection_with_no_law_root_has_no_law_cards_and_no_law_xp() {
    let scratch = tempfile::tempdir().expect("a scratch");
    let source = source(&scratch, None).await;
    let tiers = source
        .law_tiers(StudyDay::from_epoch_day(TODAY))
        .await
        .expect("the tiers are read");
    assert_eq!(
        tiers,
        LawTiers::default(),
        "every seeded card is a language card"
    );
}
