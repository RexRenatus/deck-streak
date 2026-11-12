//! The scope (SPEC-023 A3, A4, A5, A7): which decks and reviews are read equals the predecessor's,
//! a borrowed card is scoped by its home deck, and a law root the include list cannot reach is
//! warned by name at start.

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use std::collections::BTreeMap;

use deck_streak_ingest::reader::{allowed_deck_ids, is_study_event, track_of};
use deck_streak_ingest::settings::{INCLUDE_DECKS, LAW_DECK_ROOT, ScopeSettings};
use deck_streak_kernel::{Environment, SettingsError, Track};
use support::Fixture;
use support::logs::Logs;
use support::synthetic::{self, FILTERED_DECK};

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

/// An endpoint no test contacts: the reader never syncs.
const ENDPOINT: &str = "http://127.0.0.1:9/";

#[test]
fn the_deck_scope_matches_the_predecessors_golden() {
    golden::each_case("allowed_deck_ids", |case| {
        let names: BTreeMap<i64, String> = case.input["deck_names"]
            .as_object()
            .unwrap_or_else(|| panic!("deck names by id: {}", case.input))
            .iter()
            .map(|(id, name)| {
                let id = id.parse().unwrap_or_else(|_| panic!("a deck id: {id}"));
                let name = name
                    .as_str()
                    .unwrap_or_else(|| panic!("a deck name: {name}"));
                (id, name.to_owned())
            })
            .collect();
        let prefixes: Vec<String> = case.input["include_prefixes"]
            .as_array()
            .unwrap_or_else(|| panic!("the prefixes: {}", case.input))
            .iter()
            .map(|prefix| prefix.as_str().unwrap_or_default().to_owned())
            .collect();
        let expected: Vec<i64> = case
            .output
            .as_array()
            .unwrap_or_else(|| panic!("the allowed ids: {}", case.output))
            .iter()
            .filter_map(serde_json::Value::as_i64)
            .collect();
        let allowed: Vec<i64> = allowed_deck_ids(&names, &prefixes).into_iter().collect();
        assert_eq!(allowed, expected, "{}", case.input);
    });
}

#[test]
fn the_study_event_rule_matches_the_predecessors_golden() {
    golden::each_case("study_event", |case| {
        let kind = case.input["rtype"].as_i64().unwrap_or_default();
        let ease = case.input["ease"].as_i64().unwrap_or_default();
        let expected = case
            .output
            .as_bool()
            .unwrap_or_else(|| panic!("a verdict: {}", case.output));
        assert_eq!(is_study_event(kind, ease), expected, "{}", case.input);
    });
}

#[tokio::test]
async fn a_card_in_a_filtered_deck_is_scoped_by_its_original_deck() {
    let fixture = Fixture::new(ENDPOINT);
    let planned = synthetic::build_scoped(&fixture.copy());
    let cram = planned.deck(FILTERED_DECK);

    // The engine's filtered deck borrowed a Law card and a Maths card. Scoped to Law, the borrowed
    // Law card is read by its home deck, and the borrowed Maths card is not.
    let law = synthetic::reader(&fixture.settings(), "Law", None, support::clock_at(0))
        .read(0)
        .await
        .expect("the copy reads");
    let cards: Vec<(i64, i64, i64, i64)> = law
        .cards
        .iter()
        .map(|card| {
            (
                card.id,
                card.deck_id,
                card.original_deck_id,
                card.home_deck_id(),
            )
        })
        .collect();
    let torts = planned.deck("Law::Torts");
    let evidence = planned.deck("Law::Evidence");
    assert_eq!(
        cards,
        [(1001, evidence, 0, evidence), (1002, cram, torts, torts)],
        "a card is scoped by the deck it belongs to, not the filtered deck it sits in"
    );
    // Scoped to the filtered deck's own name, neither borrowed card is read: their home decks are
    // outside the scope.
    let borrowed = synthetic::reader(
        &fixture.settings(),
        FILTERED_DECK,
        None,
        support::clock_at(0),
    )
    .read(0)
    .await
    .expect("the copy reads");
    assert_eq!(
        borrowed
            .cards
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        Vec::<i64>::new()
    );
    assert!(
        borrowed.deck_names.contains_key(&cram),
        "the filtered deck exists: {:?}",
        borrowed.deck_names
    );
}

#[test]
fn a_law_root_outside_the_include_list_is_warned_by_name() {
    let fixture = Fixture::new(ENDPOINT);
    let warned = |include: &str, law_root: Option<&str>| {
        let logs = Logs::default();
        log_capture::with_capture(logs.recorder(), || {
            let _reader =
                synthetic::reader(&fixture.settings(), include, law_root, support::clock_at(0));
        });
        logs.warnings()
    };

    let warnings = warned("Zeta, Kappa", Some("Omicron"));
    assert_eq!(warnings.len(), 1, "one WARN at start: {warnings:?}");
    assert_eq!(warnings[0].field("include"), Some(INCLUDE_DECKS));
    assert_eq!(warnings[0].field("law_root"), Some(LAW_DECK_ROOT));
    for value in ["Zeta", "Kappa", "Omicron"] {
        assert!(
            warnings[0]
                .fields
                .values()
                .all(|text| !text.contains(value)),
            "the WARN names the settings and never their values: {warnings:?}"
        );
    }
    // A prefix the root starts with covers it; so does an empty list, which reads every deck; and
    // with no root there is nothing to cover.
    for (include, law_root) in [
        ("Zeta, Omi", Some("Omicron")),
        ("", Some("Omicron")),
        ("Zeta", None),
    ] {
        assert_eq!(
            warned(include, law_root).len(),
            0,
            "{include:?} with {law_root:?}"
        );
    }
}

#[test]
fn the_track_is_law_when_the_home_decks_top_level_name_starts_with_the_law_root() {
    assert_eq!(track_of("Law", Some("Law")), Track::Law);
    // R3 reads a prefix, as the include list does.
    assert_eq!(track_of("Law Review", Some("Law")), Track::Law);
    assert_eq!(track_of("Language", Some("Law")), Track::Language);
    assert_eq!(track_of("law", Some("Law")), Track::Language);
    assert_eq!(track_of("La", Some("Law")), Track::Language);
    // With no root, every card is on the language track.
    assert_eq!(track_of("Law", None), Track::Language);
}

#[test]
fn the_include_list_is_trimmed_and_the_law_root_is_a_top_level_name() {
    let scope = ScopeSettings::from_env(&Environment::from_vars([
        (INCLUDE_DECKS, " Law ,, Language , "),
        (LAW_DECK_ROOT, " Law "),
    ]))
    .expect("the scope parses");
    assert_eq!(scope.include().prefixes(), ["Law", "Language"]);
    assert_eq!(scope.law_root(), Some("Law"));
    assert_eq!(
        format!("{scope:?}"),
        "ScopeSettings { include: IncludeDecks(2 prefix(es)), law_root: Some(LawDeckRoot(..)) }"
    );
    let every = ScopeSettings::from_env(&Environment::from_vars([(INCLUDE_DECKS, " , ")]))
        .expect("the scope parses");
    assert!(every.include().reads_every_deck());
    assert_eq!(every.law_root(), None);
    let refused = ScopeSettings::from_env(&Environment::from_vars([(
        LAW_DECK_ROOT,
        "Law\u{1f}Evidence",
    )]));
    assert!(
        matches!(refused, Err(SettingsError::Malformed { setting, .. }) if setting == LAW_DECK_ROOT),
        "{refused:?}"
    );
}
