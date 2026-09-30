//! Readings' data-rights port (SPEC-045 A13; SPEC-021): `reading_topic_days` and `reading_runs` are
//! exported whole and erased, inside the caller's transaction.
//!
//! The test writes synthetic rows through the readings' store into a migrated temporary database.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_kernel::{DataRights, Db, Disposition, StudyDay, UtcMillis};
use deck_streak_readings::attempts::{AttemptOutcome, AttemptRecord, AttemptTelemetry};
use deck_streak_readings::data_rights::{
    READING_ATTEMPTS_TABLE, READING_RUNS_TABLE, READING_TOPIC_DAYS_TABLE, READINGS_CONTEXT,
    READINGS_TABLE, ReadingsDataRights,
};
use deck_streak_readings::day_set::digest;
use deck_streak_readings::reading::ReadingId;
use deck_streak_readings::state::ReadingGate;
use deck_streak_readings::state::{CouldNotTell, RunOutcome, TopicState};
use deck_streak_readings::store::{DaySetRecord, ReadingRun, RunTrigger, SqliteReadings, TopicDay};
use deck_streak_readings::store::{NewReading, VaultStatus};
use deck_streak_readings::topic::TopicKey;
use serde_json::json;

/// How many rows `table` holds.
async fn count(db: &Db, table: &str) -> i64 {
    let sql = match table {
        READING_TOPIC_DAYS_TABLE => "SELECT count(*) FROM reading_topic_days",
        READINGS_TABLE => "SELECT count(*) FROM readings",
        READING_ATTEMPTS_TABLE => "SELECT count(*) FROM reading_attempts",
        _ => "SELECT count(*) FROM reading_runs",
    };
    sqlx::query_scalar(sql)
        .fetch_one(db.reader())
        .await
        .expect("a count")
}

#[tokio::test]
async fn the_topic_days_and_runs_are_exported_and_erased() {
    let port = ReadingsDataRights;
    let declaration = port.declaration().expect("the port's declaration");
    assert_eq!(declaration.context(), READINGS_CONTEXT);
    let tables: Vec<(&str, &Disposition)> = declaration
        .tables()
        .iter()
        .map(|rights| (rights.table, &rights.disposition))
        .collect();
    assert_eq!(
        tables,
        [
            (READING_TOPIC_DAYS_TABLE, &Disposition::ExportAndErase),
            (READING_RUNS_TABLE, &Disposition::ExportAndErase),
            (READINGS_TABLE, &Disposition::ExportAndErase),
            (READING_ATTEMPTS_TABLE, &Disposition::ExportAndErase),
        ],
        "the readings' record is the owner's data: exported and erased (CHARTER 13)"
    );

    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let store = SqliteReadings::new(db.clone());
    let day = StudyDay::from_epoch_day(20_000);
    let run = ReadingRun {
        trigger: RunTrigger::Owner,
        study_day: day,
        started_at: UtcMillis::from_epoch_millis(1_000),
        finished_at: UtcMillis::from_epoch_millis(1_500),
        outcome: RunOutcome::CouldNotTell(CouldNotTell::DaySetResolveTimeout),
        unmapped_decks: 3,
    };
    let id = store.record_run(&run).await.expect("a run");
    let ready = TopicDay {
        study_day: day,
        topic: TopicKey::parse("law/evidence").expect("a topic key"),
        state: TopicState::Ready,
        day_set: Some(DaySetRecord {
            digest: digest(&[11, 12]),
            card_ids: vec![11, 12],
            note_ids: vec![110],
        }),
    };
    store
        .record_topic_day(id, &ready, UtcMillis::from_epoch_millis(1_500))
        .await
        .expect("a topic day");

    let mut connection = db.reader().acquire().await.expect("a connection");
    let exported = port.export(&mut connection).await.expect("the export");
    drop(connection);
    assert_eq!(exported.len(), 4);
    assert_eq!(exported[0].table, READING_TOPIC_DAYS_TABLE);
    assert_eq!(
        exported[0].rows,
        [json!({
            "id": 1,
            "run_id": id.get(),
            "study_day": 20_000,
            "topic": "law/evidence",
            "state": "ready",
            "class": null,
            "reason": null,
            "digest": digest(&[11, 12]),
            "card_ids": "[11,12]",
            "note_ids": "[110]",
            "new_cards": 2,
            "created_at": 1_500,
        })]
    );
    assert_eq!(exported[1].table, READING_RUNS_TABLE);
    assert_eq!(
        exported[1].rows,
        [json!({
            "id": id.get(),
            "trigger": "owner",
            "study_day": 20_000,
            "started_at": 1_000,
            "finished_at": 1_500,
            "outcome": "could_not_tell",
            "class": "rail_broken",
            "reason": "day_set_resolve_timeout",
            "unmapped_decks": 3,
            "created_at": 1_500,
        })]
    );

    let mut write = db.write().await.expect("a write");
    port.erase(&mut write).await.expect("the erase");
    write.commit().await.expect("the erase commits");
    assert_eq!(
        (
            count(&db, READING_TOPIC_DAYS_TABLE).await,
            count(&db, READING_RUNS_TABLE).await
        ),
        (0, 0),
        "the erase emptied both tables"
    );
    // The tables are still there to take the next run.
    store.record_run(&run).await.expect("a run after the erase");
    assert_eq!(count(&db, READING_RUNS_TABLE).await, 1);
    db.close().await;
}

#[tokio::test]
async fn the_readings_and_attempts_are_exported_and_erased() {
    let port = ReadingsDataRights;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let store = SqliteReadings::new(db.clone());
    let day = StudyDay::from_epoch_day(20_000);
    let run = store
        .record_run(&ReadingRun {
            trigger: RunTrigger::Owner,
            study_day: day,
            started_at: UtcMillis::from_epoch_millis(1_000),
            finished_at: UtcMillis::from_epoch_millis(1_500),
            outcome: RunOutcome::CouldNotTell(CouldNotTell::DaySetResolveTimeout),
            unmapped_decks: 0,
        })
        .await
        .expect("a run");
    let topic = TopicKey::parse("law/evidence").expect("a topic key");
    let stored = digest(&[11, 12]);
    store
        .store_reading(&NewReading {
            id: ReadingId::of("law/evidence", 20_000, &stored),
            topic: topic.clone(),
            study_day: day,
            digest: stored.clone(),
            persona: "a synthetic persona".to_owned(),
            text: "a synthetic reading".to_owned(),
            word_count: 3,
            minutes: 1,
            card_ids: vec![11, 12],
            note_count: 2,
            generated_at: UtcMillis::from_epoch_millis(1_400),
            vault: VaultStatus::Failed,
        })
        .await
        .expect("a reading");
    store
        .record_attempt(&AttemptRecord {
            run,
            topic,
            study_day: day,
            attempt: 2,
            repair_gate: Some(ReadingGate::Band),
            outcome: AttemptOutcome::Passed,
            telemetry: AttemptTelemetry {
                turns: 3,
                input_tokens: 100,
                output_tokens: 50,
                cost_micro_usd: 12_000,
                duration_ms: 900,
            },
            at: UtcMillis::from_epoch_millis(1_450),
        })
        .await
        .expect("an attempt");

    let mut connection = db.reader().acquire().await.expect("a connection");
    let exported = port.export(&mut connection).await.expect("the export");
    drop(connection);
    let tables: Vec<&str> = exported.iter().map(|table| table.table).collect();
    assert_eq!(
        tables,
        [
            READING_TOPIC_DAYS_TABLE,
            READING_RUNS_TABLE,
            READINGS_TABLE,
            READING_ATTEMPTS_TABLE
        ],
        "all four tables are exported, in the declaration's order"
    );
    let readings = &exported[2].rows;
    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0]["topic"], json!("law/evidence"));
    assert_eq!(readings[0]["text"], json!("a synthetic reading"));
    assert_eq!(readings[0]["vault_status"], json!("vault_write_failed"));
    let attempts = &exported[3].rows;
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0]["attempt"], json!(2));
    assert_eq!(attempts[0]["repair_gate"], json!("band"));
    assert_eq!(attempts[0]["verdict"], json!("passed"));
    assert_eq!(attempts[0]["input_tokens"], json!(100));

    let mut write = db.write().await.expect("a write");
    port.erase(&mut write).await.expect("the erase");
    write.commit().await.expect("the erase commits");
    for table in [READINGS_TABLE, READING_ATTEMPTS_TABLE, READING_RUNS_TABLE] {
        assert_eq!(count(&db, table).await, 0, "the erase emptied {table}");
    }
    db.close().await;
}
