//! Readings' data-rights port (SPEC-045 A13; SPEC-021): `reading_topic_days` and `reading_runs` are
//! exported whole and erased, inside the caller's transaction.
//!
//! The test writes synthetic rows through the readings' store into a migrated temporary database.

use deck_streak_kernel::{DataRights, Db, Disposition, StudyDay, UtcMillis};
use deck_streak_readings::data_rights::{
    READING_RUNS_TABLE, READING_TOPIC_DAYS_TABLE, READINGS_CONTEXT, ReadingsDataRights,
};
use deck_streak_readings::day_set::digest;
use deck_streak_readings::state::{CouldNotTell, RunOutcome, TopicState};
use deck_streak_readings::store::{DaySetRecord, ReadingRun, RunTrigger, SqliteReadings, TopicDay};
use deck_streak_readings::topic::TopicKey;
use serde_json::json;

/// How many rows `table` holds.
async fn count(db: &Db, table: &str) -> i64 {
    let sql = match table {
        READING_TOPIC_DAYS_TABLE => "SELECT count(*) FROM reading_topic_days",
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
    assert_eq!(exported.len(), 2);
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
