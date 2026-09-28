//! The gates (SPEC-045 A5, A6, A7): a healthy last sync resolves however old the collection file
//! is; a failed last sync is could-not-tell before any collection work; and two study days without
//! study pause every topic, until one review on either day resumes them.
//!
//! A5 builds its collection with ingest's own synthetic builder and resolves it through the
//! readings' production queue port over Anki's engine, on a clock injected 40 and 400 days past the
//! file's last write. Every deck, card and review is synthetic.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use deck_streak_ingest::engine::RslibEngine;
use deck_streak_ingest::sync_runs::RunStatus;
use deck_streak_kernel::{ManualClock, Offload, OffloadWorkers, UtcMillis};
use deck_streak_readings::day_set::{self, EngineQueue, ResolveInputs, StudyDayResolution};
use deck_streak_readings::gates::{LastSync, pause_window, review_floor, studied_before};
use deck_streak_readings::state::{Class, CouldNotTell, RunOutcome, TopicState};
use support::synthetic::{self, PlannedCard, PlannedReview};
use support::{
    DAY_MS, RecordingQueue, TODAY, card, collection, deck_names, during, id_of, review, root,
    studied,
};

/// The card ids of `resolution`'s day set for `topic`, or none.
fn cards_of(resolution: &StudyDayResolution, topic: &str) -> BTreeSet<i64> {
    resolution
        .day_set_of(topic)
        .map(|day_set| day_set.card_ids.iter().copied().collect())
        .unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_healthy_sync_resolves_whatever_the_collection_files_age() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let state = dir.path().join("state");
    let scratch = dir.path().join("scratch");
    for folder in [&state, &scratch] {
        fs::create_dir_all(folder).expect("a folder");
    }
    let settings = support::settings(&state);
    let cards = [
        PlannedCard {
            id: 11,
            deck: "Casebook::Evidence",
            filtered: false,
        },
        PlannedCard {
            id: 12,
            deck: "Casebook::Evidence",
            filtered: false,
        },
        PlannedCard {
            id: 21,
            deck: "Tongue Alpha::Unit 01",
            filtered: false,
        },
    ];
    synthetic::build_planned(&settings.copy_path(), &[], &cards, &[]);
    let written = UtcMillis::from_system_time(
        fs::metadata(settings.copy_path())
            .and_then(|metadata| metadata.modified())
            .expect("the copy's last write"),
    );
    let taxonomy = support::example_taxonomy();
    for days_old in [40, 400] {
        // The service's clock stands `days_old` days after the file was last written, and the owner
        // studied the study day before.
        let now = UtcMillis::from_epoch_millis(written.epoch_millis() + days_old * DAY_MS);
        let today = support::rule().study_day(now);
        synthetic::add_reviews(
            &settings.copy_path(),
            &[PlannedReview {
                id: now.epoch_millis() - DAY_MS,
                card: 21,
                kind: 1,
                ease: 3,
            }],
        );
        let clock = Arc::new(ManualClock::new(now));
        let reader = synthetic::reader(&settings, "", None, clock.clone());
        let data = reader.read(review_floor(now)).await.expect("ingest reads");
        let offload = Offload::new(OffloadWorkers::new(1).expect("one worker"), clock);
        let queue = EngineQueue::new(RslibEngine, &settings, scratch.clone(), offload);
        let resolution = day_set::resolve(
            ResolveInputs {
                today,
                rule: support::rule(),
                last_sync: LastSync::from_last_run(Some(RunStatus::Ok)),
                taxonomy: Some(&taxonomy),
                read: Ok(&data),
            },
            &queue,
        )
        .await;
        assert_eq!(resolution.outcome, RunOutcome::Resolved, "{days_old} days");
        assert_eq!(
            cards_of(&resolution, "law/evidence"),
            BTreeSet::from([11, 12])
        );
        assert_eq!(cards_of(&resolution, "language/qaa"), BTreeSet::from([21]));
    }
}

/// Decks of two topics, with one new card each.
fn two_topics() -> deck_streak_ingest::reader::CollectionData {
    let names = deck_names(&[&["Casebook", "Evidence"], &["Tongue Alpha", "Unit 01"]]);
    let evidence = id_of(&names, &["Casebook", "Evidence"]);
    let unit = id_of(&names, &["Tongue Alpha", "Unit 01"]);
    collection(
        names,
        vec![card(1, evidence, 0), card(2, unit, 0)],
        vec![studied(TODAY - 1, 3)],
    )
}

#[tokio::test]
async fn a_failed_last_sync_is_could_not_tell_before_any_collection_work() {
    let taxonomy = support::example_taxonomy();
    let studied_night = two_topics();
    let silent_night = collection(studied_night.deck_names.clone(), Vec::new(), Vec::new());
    let roots = vec![root(1, &[1, 2], 2)];
    for last in [Some(RunStatus::Error), None] {
        let last_sync = LastSync::from_last_run(last);
        assert!(!last_sync.succeeded(), "{last:?}");
        // With or without study the two days before: the sync gate comes first.
        for data in [&studied_night, &silent_night] {
            let queue = RecordingQueue::answering(roots.clone());
            let resolution = day_set::resolve(
                ResolveInputs {
                    today: support::today(),
                    rule: support::rule(),
                    last_sync,
                    taxonomy: Some(&taxonomy),
                    read: Ok(data),
                },
                &queue,
            )
            .await;
            assert_eq!(queue.calls(), 0, "the queue port is never called");
            assert_eq!(
                resolution.outcome,
                RunOutcome::CouldNotTell(CouldNotTell::SyncFailed)
            );
            let topics: Vec<&str> = resolution.topics.iter().map(|t| t.topic.as_str()).collect();
            assert_eq!(topics, ["language/qaa", "law/evidence"]);
            for topic in topics {
                let state = resolution.state_of(topic).expect("the topic ended");
                assert_eq!(state, TopicState::CouldNotTell(CouldNotTell::SyncFailed));
                assert_eq!(state.class(), Some(Class::RailBroken));
            }
        }
    }
    // The control: a successful sync on a studied night asks the queue once.
    for last in [Some(RunStatus::Ok), Some(RunStatus::Skipped)] {
        let queue = RecordingQueue::answering(roots.clone());
        let resolution = day_set::resolve(
            ResolveInputs {
                today: support::today(),
                rule: support::rule(),
                last_sync: LastSync::from_last_run(last),
                taxonomy: Some(&taxonomy),
                read: Ok(&studied_night),
            },
            &queue,
        )
        .await;
        assert_eq!(queue.calls(), 1, "{last:?}");
        assert_eq!(resolution.outcome, RunOutcome::Resolved);
    }
}

#[tokio::test]
async fn two_days_without_study_pause_every_topic() {
    let taxonomy = support::example_taxonomy();
    let base = two_topics();
    let roots = vec![root(1, &[1, 2], 2)];
    let yesterday = TODAY - 1;
    let before_yesterday = TODAY - 2;
    assert_eq!(
        pause_window(support::today()).map(|day| day.epoch_day()),
        [yesterday, before_yesterday]
    );
    // Reviews that are no study of the two days before: none at all; one three days before; one
    // today; a manual entry and a rescheduling on the day before; an unanswered review; and one a
    // millisecond before the day before yesterday's rollover.
    let paused = [
        Vec::new(),
        vec![studied(TODAY - 3, 5)],
        vec![studied(TODAY, 0)],
        vec![
            review(during(yesterday, 2), 4, 0),
            review(during(yesterday, 3), 5, 0),
        ],
        vec![review(during(yesterday, 2), 1, 0)],
        vec![review(during(before_yesterday, 0) - 1, 1, 3)],
    ];
    for reviews in paused {
        let data = collection(base.deck_names.clone(), base.cards.clone(), reviews.clone());
        let queue = RecordingQueue::answering(roots.clone());
        let resolution = day_set::resolve(
            ResolveInputs {
                today: support::today(),
                rule: support::rule(),
                last_sync: LastSync::Succeeded,
                taxonomy: Some(&taxonomy),
                read: Ok(&data),
            },
            &queue,
        )
        .await;
        assert_eq!(resolution.outcome, RunOutcome::Paused, "{reviews:?}");
        assert_eq!(queue.calls(), 0, "a paused night never asks the queue");
        for topic in ["language/qaa", "law/evidence"] {
            assert_eq!(
                resolution.state_of(topic),
                Some(TopicState::Paused),
                "{topic}"
            );
        }
        assert!(!studied_before(
            &data.reviews,
            support::today(),
            support::rule()
        ));
    }
    // One qualifying review on either day resumes, each study type counted, from the rollover on.
    let resumed = [
        vec![studied(yesterday, 20)],
        vec![review(during(before_yesterday, 0), 1, 3)],
        vec![review(during(before_yesterday, 6), 0, 1)],
        vec![review(during(yesterday, 1), 2, 4)],
        vec![review(during(yesterday, 23), 3, 2)],
    ];
    for reviews in resumed {
        let data = collection(base.deck_names.clone(), base.cards.clone(), reviews.clone());
        let queue = RecordingQueue::answering(roots.clone());
        let resolution = day_set::resolve(
            ResolveInputs {
                today: support::today(),
                rule: support::rule(),
                last_sync: LastSync::Succeeded,
                taxonomy: Some(&taxonomy),
                read: Ok(&data),
            },
            &queue,
        )
        .await;
        assert_eq!(resolution.outcome, RunOutcome::Resolved, "{reviews:?}");
        assert_eq!(queue.calls(), 1);
        assert_eq!(cards_of(&resolution, "law/evidence"), BTreeSet::from([1]));
        assert!(studied_before(
            &data.reviews,
            support::today(),
            support::rule()
        ));
    }
    // The floor the review read takes keeps every review of the two days before.
    assert!(review_floor(support::now()) < during(before_yesterday, 0));
}
