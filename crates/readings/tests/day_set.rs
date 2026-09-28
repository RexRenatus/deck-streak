//! The day set (SPEC-045 A1, A2, A4, A9, A10): the scheduler's own queue per root, attributed by
//! each card's original deck; the resolver and the digest equal the predecessor's goldens; a
//! saturated root is a configuration fault; and a resolution past its budget is a broken rail.
//!
//! A1 builds its collection with ingest's own synthetic builder, which drives Anki's engine, and
//! resolves it through the readings' production queue port over the engine. A10 runs on tokio's
//! paused clock. Every deck, card and review is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use deck_streak_ingest::engine::RslibEngine;
use deck_streak_kernel::{ManualClock, Offload, OffloadWorkers};
use deck_streak_readings::day_set::{
    self, DaySetQuery, EngineQueue, QueuedCard, RESOLVE_BUDGET, ResolveInputs, StudyDayResolution,
    TopicEnd, UndeterminedRoot, digest, resolve_day_sets, saturated,
};
use deck_streak_readings::gates::{LastSync, review_floor};
use deck_streak_readings::state::{Class, CouldNotTell, RunOutcome, TopicState};
use serde_json::{Value, json};
use support::synthetic::{self, PlannedCard, PlannedReview};
use support::{RecordingQueue, TODAY, card, collection, deck_names, id_of, root, studied};

/// A1's decks, as ingest's builder names them: two subjects under one law root, a language deck,
/// and a deck of no topic.
const EVIDENCE: &str = "Casebook::Evidence";
const TORTS: &str = "Casebook::Torts";
const TONGUE: &str = "Tongue Alpha::Unit 01";
const MISC: &str = "Misc";

/// A1's collection: fifteen new cards in each law subject, three of Evidence's borrowed by the
/// engine's filtered deck, four language cards and two cards of no topic.
fn a1_cards() -> Vec<PlannedCard> {
    let mut cards = Vec::new();
    for id in 101..=115 {
        cards.push(PlannedCard {
            id,
            deck: EVIDENCE,
            filtered: id <= 103,
        });
    }
    for id in 201..=215 {
        cards.push(PlannedCard {
            id,
            deck: TORTS,
            filtered: false,
        });
    }
    for id in 301..=304 {
        cards.push(PlannedCard {
            id,
            deck: TONGUE,
            filtered: false,
        });
    }
    for id in 401..=402 {
        cards.push(PlannedCard {
            id,
            deck: MISC,
            filtered: false,
        });
    }
    cards
}

/// The card ids of `resolution`'s day set for `topic`, or none.
fn cards_of(resolution: &StudyDayResolution, topic: &str) -> BTreeSet<i64> {
    resolution
        .day_set_of(topic)
        .map(|day_set| day_set.card_ids.iter().copied().collect())
        .unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_day_set_is_the_schedulers_queue_per_root_attributed_by_original_deck() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let state = dir.path().join("state");
    let scratch = dir.path().join("scratch");
    for folder in [&state, &scratch] {
        fs::create_dir_all(folder).expect("a folder");
    }
    let settings = support::settings(&state);
    let cards = a1_cards();
    // One qualifying review the day before, so the pause gate lets the resolution run.
    let reviews = [PlannedReview {
        id: support::during(TODAY - 1, 2),
        card: 401,
        kind: 1,
        ease: 3,
    }];
    let planned = synthetic::build_planned(&settings.copy_path(), &[MISC], &cards, &reviews);
    assert!(
        planned.filtered.is_some(),
        "the engine built the filtered deck"
    );

    let clock = Arc::new(ManualClock::new(support::now()));
    let reader = synthetic::reader(&settings, "", None, clock.clone());
    let data = reader
        .read(review_floor(support::now()))
        .await
        .expect("ingest reads the copy");
    let offload = Offload::new(OffloadWorkers::new(1).expect("one worker"), clock);
    let queue = EngineQueue::new(RslibEngine, &settings, scratch.clone(), offload);
    let taxonomy = support::example_taxonomy();
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

    assert_eq!(resolution.outcome, RunOutcome::Resolved);
    let topics: Vec<&str> = resolution.topics.iter().map(|t| t.topic.as_str()).collect();
    assert_eq!(topics, ["language/qaa", "law/evidence", "law/torts"]);
    let evidence = cards_of(&resolution, "law/evidence");
    let torts = cards_of(&resolution, "law/torts");
    let borrowed: BTreeSet<i64> = (101..=103).collect();
    // The filtered deck's cards are attributed by their original deck, law/evidence.
    assert!(
        borrowed.is_subset(&evidence),
        "every borrowed card is in law/evidence: {evidence:?}"
    );
    // The law root's own cards are one query under the root's budget: 20 new cards, not the 27 its
    // two subjects' own budgets would give.
    let own: BTreeSet<i64> = evidence.union(&torts).copied().collect();
    let own: BTreeSet<i64> = own.difference(&borrowed).copied().collect();
    let budget = usize::try_from(synthetic::NEW_PER_DAY).expect("a count");
    assert_eq!(own.len(), budget, "{own:?}");
    assert!(
        own.iter()
            .all(|id| (104..=115).contains(id) || (201..=215).contains(id))
    );
    assert!(evidence.iter().all(|id| (101..=115).contains(id)));
    assert!(torts.iter().all(|id| (201..=215).contains(id)));
    assert_eq!(
        cards_of(&resolution, "language/qaa"),
        (301..=304).collect::<BTreeSet<i64>>()
    );
    // Each card's note came from ingest's read: the builder gives a card its own id's note.
    for resolved in &resolution.topics {
        if let TopicEnd::DaySet(day_set) = &resolved.end {
            assert_eq!(day_set.note_ids, day_set.card_ids, "{}", resolved.topic);
        }
    }
    // The deck of no topic is counted, not attributed, and the filtered deck is never a topic's.
    assert_eq!(resolution.unmapped.len(), 1);
    assert_eq!(
        (
            resolution.unmapped[0].deck_name.as_str(),
            resolution.unmapped[0].card_count
        ),
        ("Misc", 2)
    );
    // The throwaway copy the engine read is gone: its directory is empty, so it can be removed.
    fs::remove_dir(&scratch).expect("the throwaway copy was removed, leaving its directory empty");
}

/// The synthetic deck names of a golden case, by id.
fn golden_deck_names(input: &Value) -> BTreeMap<i64, String> {
    input["deck_names"]
        .as_object()
        .expect("deck names")
        .iter()
        .map(|(id, name)| {
            (
                id.parse().expect("a deck id"),
                name.as_str().expect("a name").to_owned(),
            )
        })
        .collect()
}

/// The queries of a golden case.
fn golden_queries(input: &Value) -> Vec<DaySetQuery> {
    input["queries"]
        .as_array()
        .expect("queries")
        .iter()
        .map(|query| DaySetQuery {
            root: query["root_label"].as_str().expect("a label").to_owned(),
            cards: query["cards"]
                .as_array()
                .expect("cards")
                .iter()
                .map(|card| QueuedCard {
                    id: card["id"].as_i64().expect("an id"),
                    note_id: None,
                    deck_id: card["did"].as_i64().expect("a deck"),
                    original_deck_id: card["odid"].as_i64().expect("an original deck"),
                })
                .collect(),
        })
        .collect()
}

/// The undetermined roots of a golden case.
fn golden_undetermined(input: &Value) -> Vec<UndeterminedRoot> {
    input["undetermined"]
        .as_array()
        .expect("undetermined roots")
        .iter()
        .map(|root| UndeterminedRoot {
            root: root["root_label"].as_str().expect("a label").to_owned(),
            reason: CouldNotTell::parse(root["reason"].as_str().expect("a reason"))
                .expect("a closed reason"),
        })
        .collect()
}

#[test]
fn the_day_set_resolution_matches_the_parity_golden() {
    let taxonomy = support::example_taxonomy();
    let examined = golden::each_case("resolve_day_sets", |case| {
        let resolution = resolve_day_sets(
            &golden_queries(&case.input),
            &golden_deck_names(&case.input),
            &taxonomy,
            &golden_undetermined(&case.input),
        );
        let ported = json!({
            "active_topics": resolution.active.iter().map(|topic| json!({
                "topic_key": topic.topic.as_str(),
                "deck_ids": topic.deck_ids,
                "new_cards": topic.card_ids,
                "digest": topic.digest,
            })).collect::<Vec<_>>(),
            "unmapped_decks": resolution.unmapped.iter().map(|deck| json!({
                "deck_name": deck.deck_name,
                "card_count": deck.card_count,
            })).collect::<Vec<_>>(),
            "no_new_today": resolution.no_new_today,
            "undetermined": resolution.undetermined.iter().map(|root| json!({
                "root_label": root.root,
                "reason": root.reason.as_str(),
            })).collect::<Vec<_>>(),
        });
        assert_eq!(ported, case.output, "the resolution of {}", case.input);
    });
    assert_eq!(examined.function, "prereading.resolve_day_sets");
}

#[test]
fn the_digest_matches_the_parity_golden() {
    let examined = golden::each_case("digest_for_card_ids", |case| {
        let ids: Vec<i64> = case.input["card_ids"]
            .as_array()
            .expect("card ids")
            .iter()
            .map(|id| id.as_i64().expect("an id"))
            .collect();
        assert_eq!(
            Value::from(digest(&ids)),
            case.output,
            "the digest of {ids:?}"
        );
    });
    assert_eq!(examined.function, "prereading._digest_for_card_ids");
}

#[tokio::test]
async fn a_saturated_root_is_a_config_fault() {
    // Without the scheduler's own count, 1000 cards (the fetch limit) or more is saturated.
    assert!(saturated(1000, None));
    assert!(saturated(1001, None));
    assert!(!saturated(999, None));
    // With it, fewer cards than the count is saturated, and as many is complete.
    assert!(saturated(4, Some(5)));
    assert!(saturated(0, Some(1)));
    assert!(!saturated(5, Some(5)));
    assert!(!saturated(0, Some(0)));

    let names = deck_names(&[
        &["Casebook"],
        &["Casebook", "Evidence"],
        &["Tongue Alpha"],
        &["Tongue Alpha", "Unit 01"],
    ]);
    let law = id_of(&names, &["Casebook"]);
    let evidence = id_of(&names, &["Casebook", "Evidence"]);
    let tongue = id_of(&names, &["Tongue Alpha"]);
    let unit = id_of(&names, &["Tongue Alpha", "Unit 01"]);
    let data = collection(
        names,
        vec![
            card(1, evidence, 0),
            card(2, evidence, 0),
            card(3, evidence, 0),
            card(10, unit, 0),
            card(11, unit, 0),
        ],
        vec![studied(TODAY - 1, 2)],
    );
    // The law root answered three of the five new cards it counts; the language root all of its.
    let queue =
        RecordingQueue::answering(vec![root(law, &[1, 2, 3], 5), root(tongue, &[10, 11], 2)]);
    let taxonomy = support::example_taxonomy();
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
    assert_eq!(resolution.outcome, RunOutcome::Resolved);
    let saturated_state = resolution
        .state_of("law/evidence")
        .expect("law/evidence ended");
    assert_eq!(
        saturated_state,
        TopicState::CouldNotTell(CouldNotTell::DaySetFetchSaturated)
    );
    assert_eq!(saturated_state.class(), Some(Class::ConfigFault));
    assert_eq!(
        cards_of(&resolution, "language/qaa"),
        BTreeSet::from([10, 11])
    );
}

#[tokio::test(start_paused = true)]
async fn a_resolution_past_its_budget_is_rail_broken() {
    assert_eq!(RESOLVE_BUDGET, Duration::from_secs(30));
    let names = deck_names(&[&["Casebook", "Evidence"], &["Tongue Alpha"]]);
    let evidence = id_of(&names, &["Casebook", "Evidence"]);
    let data = collection(
        names,
        vec![card(1, evidence, 0)],
        vec![studied(TODAY - 2, 3)],
    );
    let taxonomy = support::example_taxonomy();
    let inputs = ResolveInputs {
        today: support::today(),
        rule: support::rule(),
        last_sync: LastSync::Succeeded,
        taxonomy: Some(&taxonomy),
        read: Ok(&data),
    };
    let answer = vec![root(evidence, &[1], 1)];

    // A queue that answers a millisecond past the budget, on tokio's paused clock.
    let late =
        RecordingQueue::answering(answer.clone()).after(RESOLVE_BUDGET + Duration::from_millis(1));
    let started = tokio::time::Instant::now();
    let resolution = day_set::resolve(inputs, &late).await;
    assert_eq!(
        started.elapsed(),
        RESOLVE_BUDGET,
        "the budget is waited out, no longer"
    );
    assert_eq!(
        resolution.outcome,
        RunOutcome::CouldNotTell(CouldNotTell::DaySetResolveTimeout)
    );
    for topic in ["language/qaa", "law/evidence"] {
        let state = resolution.state_of(topic).expect("the topic ended");
        assert_eq!(
            state,
            TopicState::CouldNotTell(CouldNotTell::DaySetResolveTimeout),
            "{topic}"
        );
        assert_eq!(state.class(), Some(Class::RailBroken));
    }
    assert_eq!(late.calls(), 1);

    // A queue that answers a millisecond inside the budget resolves.
    let early = RecordingQueue::answering(answer)
        .after(RESOLVE_BUDGET.saturating_sub(Duration::from_millis(1)));
    let resolution = day_set::resolve(inputs, &early).await;
    assert_eq!(resolution.outcome, RunOutcome::Resolved);
    assert_eq!(cards_of(&resolution, "law/evidence"), BTreeSet::from([1]));
    assert_eq!(
        resolution.state_of("language/qaa"),
        Some(TopicState::NoNewCards)
    );
}
