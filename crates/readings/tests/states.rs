//! The closed states (SPEC-045 A8, A14): every state, class and reason is stored and read back
//! distinctly, and `ai_route_absent` is a state of its own, never a failure or a refusal.
//!
//! Each test writes to a migrated temporary database through the readings' store. Every topic is
//! synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;

use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_readings::day_set::digest;
use deck_streak_readings::state::{Class, CouldNotTell, FailedReason, RunOutcome, TopicState};
use deck_streak_readings::store::{DaySetRecord, ReadingRun, RunTrigger, SqliteReadings, TopicDay};
use deck_streak_readings::topic::TopicKey;
use tempfile::TempDir;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// A migrated temporary database and the readings' store over it.
async fn store() -> (TempDir, Db, SqliteReadings) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let store = SqliteReadings::new(db.clone());
    (directory, db, store)
}

/// A run of `outcome` on study day 20000.
fn run(outcome: RunOutcome) -> ReadingRun {
    ReadingRun {
        trigger: RunTrigger::Scheduled,
        study_day: StudyDay::from_epoch_day(20_000),
        started_at: UtcMillis::from_epoch_millis(1_000),
        finished_at: UtcMillis::from_epoch_millis(2_000),
        outcome,
        unmapped_decks: 2,
    }
}

/// The day set a state carries when it is stored: a generated topic's own, else none.
fn day_set_for(state: TopicState) -> Option<DaySetRecord> {
    matches!(state, TopicState::Ready | TopicState::Failed(_)).then(|| DaySetRecord {
        digest: digest(&[5, 3]),
        card_ids: vec![3, 5],
        note_ids: vec![30],
    })
}

/// A stored form of each state: its name, class and reason.
type Stored = (&'static str, Option<Class>, Option<String>);

/// The mapping the store writes a state by.
fn stored(state: TopicState) -> Stored {
    (state.name(), state.class(), state.reason())
}

/// Every pair of states a mapping gives one stored form: none, for a mapping that keeps every state
/// apart.
fn collapsed(states: &[TopicState], mapping: impl Fn(TopicState) -> Stored) -> Vec<String> {
    let mut seen: BTreeMap<Stored, TopicState> = BTreeMap::new();
    let mut found = Vec::new();
    for state in states {
        if let Some(first) = seen.insert(mapping(*state), *state) {
            found.push(format!("{first} and {state}"));
        }
    }
    found
}

#[tokio::test]
async fn every_state_and_class_is_stored_and_read_back_distinctly() {
    let states = examined("topic state(s)", TopicState::every());
    // Six could-not-tell reasons, eighteen failed reasons and the four states that carry none.
    assert_eq!(states.len(), 28);
    let (_directory, db, store) = store().await;
    let first = store
        .record_run(&run(RunOutcome::Resolved))
        .await
        .expect("a run");
    let day = StudyDay::from_epoch_day(20_000);
    let mut written = Vec::new();
    for (index, state) in states.iter().enumerate() {
        let topic = TopicKey::parse(&format!("law/synthetic-{index}")).expect("a topic key");
        let topic_day = TopicDay {
            study_day: day,
            topic,
            state: *state,
            day_set: day_set_for(*state),
        };
        store
            .record_topic_day(first, &topic_day, UtcMillis::from_epoch_millis(2_000))
            .await
            .expect("every state is written");
        written.push(topic_day);
    }
    let read = store
        .topic_days(day)
        .await
        .expect("the topic days read back");
    let mut read: Vec<TopicDay> = read.into_iter().map(|stored| stored.day).collect();
    read.sort_by_key(|topic_day| {
        topic_day.topic.as_str()["law/synthetic-".len()..]
            .parse::<usize>()
            .expect("an index")
    });
    assert_eq!(read, written, "every state reads back as itself");

    // Each class goes with its reason alone, and the stored forms are all distinct.
    for state in &states {
        if let TopicState::CouldNotTell(reason) = state {
            assert_eq!(state.class(), Some(reason.class()));
            let expected = if matches!(
                reason,
                CouldNotTell::TaxonomyMissing | CouldNotTell::DaySetFetchSaturated
            ) {
                Class::ConfigFault
            } else {
                Class::RailBroken
            };
            assert_eq!(reason.class(), expected, "{reason:?}");
        }
    }
    assert_eq!(collapsed(&states, stored), Vec::<String>::new());
    // A planted mapping that writes a timeout as a locked collection collapses the two.
    let planted = |state: TopicState| match state {
        TopicState::CouldNotTell(CouldNotTell::DaySetResolveTimeout) => {
            stored(TopicState::CouldNotTell(CouldNotTell::CollectionLocked))
        }
        other => stored(other),
    };
    assert_eq!(
        collapsed(&states, planted),
        ["could_not_tell (collection_locked) and could_not_tell (day_set_resolve_timeout)"]
    );

    // Every run outcome reads back as itself, and a later run the same day replaces a state.
    for outcome in examined("run outcome(s)", RunOutcome::every()) {
        store.record_run(&run(outcome)).await.expect("a run");
    }
    let runs = store.runs().await.expect("the runs read back");
    let outcomes: Vec<RunOutcome> = runs.iter().skip(1).map(|(_, run)| run.outcome).collect();
    assert_eq!(outcomes, RunOutcome::every());
    assert_eq!(runs[0].1, run(RunOutcome::Resolved));
    let (later, _) = runs.last().copied().expect("a run");
    let replaced = TopicDay {
        study_day: day,
        topic: TopicKey::parse("law/synthetic-0").expect("a topic key"),
        state: TopicState::Paused,
        day_set: None,
    };
    store
        .record_topic_day(later, &replaced, UtcMillis::from_epoch_millis(3_000))
        .await
        .expect("a later state");
    let again = store.topic_days(day).await.expect("the topic days");
    assert_eq!(again.len(), states.len(), "one row per study day and topic");
    let row = again
        .iter()
        .find(|stored| stored.day.topic.as_str() == "law/synthetic-0")
        .expect("the topic's row");
    assert_eq!((row.run, &row.day), (later, &replaced));
    assert!(later > first, "the later run's id follows the first's");
    db.close().await;
}

#[tokio::test]
async fn ai_route_absent_is_a_state_of_its_own_and_never_a_failure() {
    let absent = TopicState::AiRouteAbsent;
    assert_eq!(stored(absent), ("ai_route_absent", None, None));
    assert!(
        !absent.is_failure(),
        "an absent route is a setting, not a failure"
    );
    assert!(
        !absent.refuses(),
        "an absent route is never counted as a refusal"
    );
    // It is neither `failed` nor `could_not_tell`, which are failures or refusals.
    let failed = TopicState::Failed(FailedReason::AgentUnavailable(
        deck_streak_readings::state::AgentCause::ProxyUnreachable,
    ));
    assert!(failed.is_failure());
    assert!(failed.refuses());
    assert!(TopicState::CouldNotTell(CouldNotTell::SyncFailed).refuses());
    assert_ne!(stored(absent), stored(failed));

    let (_directory, db, store) = store().await;
    let first = store
        .record_run(&run(RunOutcome::AiRouteAbsent))
        .await
        .expect("a run");
    let day = StudyDay::from_epoch_day(20_000);
    let topic_day = TopicDay {
        study_day: day,
        topic: TopicKey::parse("language/qaa").expect("a topic key"),
        state: absent,
        day_set: None,
    };
    store
        .record_topic_day(first, &topic_day, UtcMillis::from_epoch_millis(2_000))
        .await
        .expect("the state is written");
    let read = store
        .topic_days(day)
        .await
        .expect("the topic day reads back");
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].day, topic_day);
    assert_eq!(read[0].day.state.class(), None);
    assert_eq!(read[0].day.state.reason(), None);
    let runs = store.runs().await.expect("the run reads back");
    assert_eq!(runs[0].1.outcome, RunOutcome::AiRouteAbsent);
    // Stored with a reason or a class, it is refused: the table holds it as itself alone.
    assert_eq!(
        TopicState::from_stored(
            "ai_route_absent",
            None,
            Some("agent_unavailable:run_failed")
        ),
        None
    );
    assert_eq!(
        TopicState::from_stored("ai_route_absent", Some("rail_broken"), None),
        None
    );
    let mut write = db.write().await.expect("a write");
    let refused = sqlx::query(
        "INSERT INTO reading_topic_days (run_id, study_day, topic, state, class, reason, digest, \
         card_ids, note_ids, new_cards, created_at) \
         VALUES (?1, 20001, 'language/qab', 'ai_route_absent', NULL, \
         'agent_unavailable:run_failed', NULL, '[]', '[]', 0, 1)",
    )
    .bind(first.get())
    .execute(&mut *write)
    .await;
    assert!(
        refused.is_err(),
        "the table refuses a reason on ai_route_absent"
    );
    drop(write);
    db.close().await;
}
