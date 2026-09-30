//! A stored reading reads back as it was written, and the closed names parse only as themselves
//! (SPEC-046 R9, R10).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_readings::day_set::digest;
use deck_streak_readings::reading::ReadingId;
use deck_streak_readings::state::{AgentCause, ReadingGate};
use deck_streak_readings::store::{NewReading, SqliteReadings, VaultStatus};
use deck_streak_readings::topic::TopicKey;

fn reading(topic: &str, generated_at: i64, vault: VaultStatus) -> NewReading {
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
        vault,
    }
}

#[tokio::test]
async fn a_stored_reading_reads_back_whole_with_either_vault_status() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let store = SqliteReadings::new(db);
    let written = reading(
        "law/evidence",
        1_000,
        VaultStatus::Written("notes/reading.md".to_owned()),
    );
    let failed = reading("law/torts", 2_000, VaultStatus::Failed);
    store.store_reading(&written).await.expect("a reading");
    store.store_reading(&failed).await.expect("a reading");

    let all = store.readings().await.expect("the readings");
    let read: Vec<(&NewReading, u32)> = all
        .iter()
        .map(|stored| (&stored.reading, stored.carried_nights))
        .collect();
    assert_eq!(read, vec![(&written, 0), (&failed, 0)], "oldest first");

    let latest = store
        .latest_reading(&written.topic)
        .await
        .expect("a read")
        .expect("a reading of the topic");
    assert_eq!(latest.reading, written);

    store.carry_reading(&written.id).await.expect("a carry");
    store.carry_reading(&written.id).await.expect("a carry");
    let carried = store
        .latest_reading(&written.topic)
        .await
        .expect("a read")
        .expect("a reading of the topic");
    assert_eq!(carried.carried_nights, 2, "two nights carried");
    let other = store
        .latest_reading(&failed.topic)
        .await
        .expect("a read")
        .expect("a reading of the topic");
    assert_eq!(other.carried_nights, 0, "only the named reading carried");

    let topics = store.known_topics().await.expect("the topics");
    assert_eq!(topics, vec![written.topic, failed.topic]);
}

#[test]
fn a_reading_id_parses_only_as_thirty_two_lowercase_hex_digits() {
    let id = ReadingId::of("law/evidence", 20_000, &digest(&[1]));
    assert_eq!(
        ReadingId::parse(id.as_str()).map(|parsed| parsed.as_str().to_owned()),
        Some(id.as_str().to_owned())
    );
    assert!(ReadingId::parse(&"0123456789abcdef".repeat(2)).is_some());
    assert!(ReadingId::parse(&"0123456789ABCDEF".repeat(2)).is_none());
    assert!(ReadingId::parse(&"0123456789abcdeg".repeat(2)).is_none());
    assert!(ReadingId::parse(&"0123456789abcde".repeat(2)).is_none());
    assert!(ReadingId::parse(&"0123456789abcdef".repeat(3)).is_none());
    assert!(ReadingId::parse("").is_none());
}

#[test]
fn the_closed_names_parse_as_themselves_and_nothing_else() {
    for gate in ReadingGate::ALL {
        assert_eq!(ReadingGate::parse(gate.as_str()), Some(gate));
    }
    for cause in AgentCause::ALL {
        assert_eq!(AgentCause::parse(cause.as_str()), Some(cause));
    }
    assert_eq!(ReadingGate::parse("nonesuch"), None);
    assert_eq!(AgentCause::parse("nonesuch"), None);
    assert_eq!(ReadingGate::parse(""), None);
}
