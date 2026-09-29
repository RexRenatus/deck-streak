//! A stored reading's read line and studied measure round-trip through the store, and the closed
//! names of the verdict and the vault tick parse only as themselves (SPEC-047 R8, R9).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_readings::day_set::digest;
use deck_streak_readings::reading::ReadingId;
use deck_streak_readings::store::{NewReading, SqliteReadings, VaultStatus, VaultTick};
use deck_streak_readings::studied::Verdict;
use deck_streak_readings::topic::TopicKey;

fn reading(topic: &str, generated_at: i64) -> NewReading {
    let stored = digest(&[11, 12]);
    NewReading {
        id: ReadingId::of(topic, 20_000, &stored),
        topic: TopicKey::parse(topic).expect("a topic key"),
        study_day: StudyDay::from_epoch_day(20_000),
        digest: stored,
        persona: "a synthetic persona".to_owned(),
        text: format!("a synthetic reading of {topic}"),
        word_count: 5,
        minutes: 2,
        card_ids: vec![11, 12],
        note_count: 2,
        generated_at: UtcMillis::from_epoch_millis(generated_at),
        vault: VaultStatus::Failed,
    }
}

async fn store() -> (tempfile::TempDir, SqliteReadings) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (directory, SqliteReadings::new(db))
}

#[test]
fn the_closed_names_parse_only_as_themselves() {
    for verdict in [Verdict::Open, Verdict::Studied, Verdict::Retired] {
        assert_eq!(Verdict::parse(verdict.as_str()), Some(verdict));
    }
    assert_eq!(Verdict::Open.as_str(), "open");
    assert_eq!(Verdict::Studied.as_str(), "studied");
    assert_eq!(Verdict::Retired.as_str(), "retired");
    assert_eq!(Verdict::parse("done"), None);
    for tick in [VaultTick::None, VaultTick::Written, VaultTick::Pending] {
        assert_eq!(VaultTick::parse(tick.as_str()), Some(tick));
    }
    assert_eq!(VaultTick::None.as_str(), "none");
    assert_eq!(VaultTick::Written.as_str(), "written");
    assert_eq!(VaultTick::Pending.as_str(), "pending");
    assert_eq!(VaultTick::parse("sent"), None);
}

#[tokio::test]
async fn the_read_line_is_set_once_and_the_first_instant_stays() {
    let (_directory, store) = store().await;
    let stored = reading("law/evidence", 1_000);
    store.store_reading(&stored).await.expect("a reading");

    let fresh = store.progress(&stored.id).await.expect("a read");
    let fresh = fresh.expect("the reading has a row");
    assert_eq!(fresh.read_at, None);
    assert_eq!(fresh.verdict, Verdict::Open);
    assert_eq!(fresh.vault_tick, VaultTick::None);
    assert_eq!(fresh.card_ids, vec![11, 12]);

    let first = UtcMillis::from_epoch_millis(5_000);
    assert!(store.mark_read(&stored.id, first).await.expect("a tap"));
    let later = UtcMillis::from_epoch_millis(9_000);
    assert!(!store.mark_read(&stored.id, later).await.expect("a tap"));
    let read = store.progress(&stored.id).await.expect("a read");
    assert_eq!(read.expect("a row").read_at, Some(first));

    let missing = ReadingId::of("law/torts", 20_000, &digest(&[1]));
    assert!(!store.mark_read(&missing, first).await.expect("a tap"));
    assert_eq!(store.progress(&missing).await.expect("a read"), None);
}

#[tokio::test]
async fn the_vault_tick_and_the_measure_read_back() {
    let (_directory, store) = store().await;
    let stored = reading("law/evidence", 1_000);
    store.store_reading(&stored).await.expect("a reading");

    for tick in [VaultTick::Pending, VaultTick::Written] {
        store
            .set_vault_tick(&stored.id, tick)
            .await
            .expect("a tick");
        let now = store.progress(&stored.id).await.expect("a read");
        assert_eq!(now.expect("a row").vault_tick, tick);
    }

    store
        .record_measure(&stored.id, 1, Verdict::Retired, None)
        .await
        .expect("a measure");
    let retired = store.progress(&stored.id).await.expect("a read");
    let retired = retired.expect("a row");
    assert_eq!(retired.studied_count, 1);
    assert_eq!(retired.verdict, Verdict::Retired);
    assert_eq!(retired.studied_at, None);

    let at = UtcMillis::from_epoch_millis(7_000);
    store
        .record_measure(&stored.id, 2, Verdict::Studied, Some(at))
        .await
        .expect("a measure");
    store
        .record_measure(&stored.id, 3, Verdict::Studied, None)
        .await
        .expect("a measure");
    let studied = store.progress(&stored.id).await.expect("a read");
    let studied = studied.expect("a row");
    assert_eq!(studied.studied_count, 3);
    assert_eq!(studied.verdict, Verdict::Studied);
    assert_eq!(studied.studied_at, Some(at), "the first instant stays");
}

#[tokio::test]
async fn the_unsettled_readings_are_the_ones_not_yet_studied_oldest_first() {
    let (_directory, store) = store().await;
    let newer = reading("law/torts", 3_000);
    let older = reading("law/evidence", 1_000);
    let done = reading("law/contracts", 2_000);
    for each in [&newer, &older, &done] {
        store.store_reading(each).await.expect("a reading");
    }
    store
        .record_measure(
            &done.id,
            2,
            Verdict::Studied,
            Some(UtcMillis::from_epoch_millis(9)),
        )
        .await
        .expect("a measure");
    let unsettled = store.unsettled().await.expect("the unsettled");
    let ids: Vec<&ReadingId> = unsettled.iter().map(|each| &each.id).collect();
    assert_eq!(ids, vec![&older.id, &newer.id]);
}
