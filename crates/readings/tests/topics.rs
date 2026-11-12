//! Topics (SPEC-045 A3, A11): a deck's topic follows the predecessor's law-subject parse, an unsafe
//! slug is reported unmapped, and the topics come only from the configured taxonomy.
//!
//! Every deck name and both taxonomies are synthetic: the example file, and a second one written
//! here that reads the same decks another way.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;

use deck_streak_kernel::{Environment, SettingsError};
use deck_streak_readings::day_set::{
    self, DaySetQuery, QueuedCard, ResolveInputs, resolve_day_sets, universe,
};
use deck_streak_readings::gates::LastSync;
use deck_streak_readings::state::{Class, CouldNotTell, RunOutcome, TopicState};
use deck_streak_readings::taxonomy::{READINGS_TAXONOMY, Taxonomy, TaxonomyError, TaxonomyPath};
use deck_streak_readings::topic::{TopicKey, law_subject, slug, topic_of};
use serde_json::Value;
use support::{RecordingQueue, TODAY, collection, deck, deck_names, studied};

/// A second synthetic taxonomy over the same decks: another law root, the example's law root read
/// as a language, and the same writing root.
const OTHER: &str = r#"{
  "schema": "deckstreak.readings.taxonomy.v1",
  "law": {"roots": ["Ledger"], "bands": []},
  "languages": [
    {"deck": "Casebook", "code": "qac", "display": "Alpha", "term_field": "Headword"}
  ],
  "writing_roots": ["Composition"]
}"#;

/// Decks and the topic each takes under the example taxonomy.
const KEYS: [(&[&str], Option<&str>); 14] = [
    (&["Casebook"], None),
    (&["Casebook", "Method"], Some("law/method")),
    (&["Casebook", "Evidence", "Unit 01"], Some("law/evidence")),
    (
        &[
            "Casebook",
            "Year One",
            "Public Law",
            "Constitutional Law",
            "Unit 02",
        ],
        Some("law/constitutional-law"),
    ),
    (
        &["Casebook", "Year Two", "Private Law"],
        Some("law/private-law"),
    ),
    (
        &[
            "Casebook",
            "Year Three",
            "Skills",
            "Civil_Procedure -- Practice",
        ],
        Some("law/civil-procedure-practice"),
    ),
    (&["Casebook", "  Contracts  "], Some("law/contracts")),
    (
        &["Casebook", "Year Two", "Private Law", "Procedure & Proof"],
        None,
    ),
    (&["Casebook", "契約", "Unit 01"], None),
    (&["Tongue Beta", "Unit 03"], Some("language/qab")),
    (&["Composition", "Alpha"], Some("language/qaa")),
    (&["Composition", "Gamma"], None),
    (&["Composition"], None),
    (&["Misc"], None),
];

#[test]
fn topic_keys_match_the_parity_golden() {
    let taxonomy = support::example_taxonomy();
    let examined = golden::each_case("law_subject", |case| {
        let name = case.input["deck_name"].as_str().expect("a deck name");
        let subject = law_subject(name, &taxonomy).map(Value::from);
        assert_eq!(
            subject.unwrap_or(Value::Null),
            case.output,
            "the subject of {name:?}"
        );
    });
    assert_eq!(examined.function, "leeches._law_subject");

    for (parts, key) in KEYS {
        let topic = topic_of(&deck(parts), &taxonomy);
        assert_eq!(topic.as_ref().map(TopicKey::as_str), key, "{parts:?}");
    }
    assert_eq!(
        slug("  Civil__Procedure - -Practice "),
        "civil-procedure-practice"
    );
    assert_eq!(slug("-\u{1f}Torts\u{1c}"), "torts");

    // A subject whose slug is refused is reported unmapped, never forced into a key.
    let names = deck_names(&[&["Casebook", "Year Two", "Private Law", "Procedure & Proof"]]);
    let (id, unsafe_name) = names.iter().next().expect("a deck");
    let query = DaySetQuery {
        root: "Casebook".to_owned(),
        cards: vec![QueuedCard {
            id: 7,
            note_id: Some(70),
            deck_id: *id,
            original_deck_id: 0,
        }],
    };
    let resolution = resolve_day_sets(&[query], &names, &taxonomy, &[]);
    assert_eq!(resolution.active.len(), 0, "{:?}", resolution.active);
    assert_eq!(resolution.unmapped.len(), 1);
    assert_eq!(resolution.unmapped[0].deck_name, *unsafe_name);
    assert_eq!(resolution.unmapped[0].card_count, 1);
    assert_eq!(resolution.no_new_today, ["Casebook"]);
}

/// One collection's decks: under the example taxonomy three topics, under the other two.
fn one_collection() -> Vec<&'static [&'static str]> {
    vec![
        &["Casebook"],
        &["Casebook", "Evidence"],
        &["Casebook", "Year One", "Public Law", "Torts"],
        &["Tongue Alpha"],
        &["Tongue Alpha", "Unit 01"],
        &["Composition", "Alpha"],
        &["Ledger", "Assets"],
        &["Misc"],
    ]
}

#[tokio::test]
async fn the_topics_come_only_from_the_configured_taxonomy() {
    let example = support::example_taxonomy();
    let other = Taxonomy::parse(OTHER).expect("the other taxonomy");
    let names = deck_names(&one_collection());
    let keys = |taxonomy: &Taxonomy| -> BTreeSet<String> {
        universe(&names, taxonomy)
            .into_keys()
            .map(|topic| topic.as_str().to_owned())
            .collect()
    };
    assert_eq!(
        keys(&example),
        BTreeSet::from([
            "language/qaa".to_owned(),
            "law/evidence".to_owned(),
            "law/torts".to_owned()
        ])
    );
    assert_eq!(
        keys(&other),
        BTreeSet::from(["language/qac".to_owned(), "law/assets".to_owned()])
    );

    // The same collection, resolved on a paused night under each taxonomy: each names its own.
    let data = collection(names.clone(), Vec::new(), Vec::new());
    for (taxonomy, expected) in [(&example, keys(&example)), (&other, keys(&other))] {
        let queue = RecordingQueue::answering(Vec::new());
        let resolution = day_set::resolve(
            ResolveInputs {
                today: support::today(),
                rule: support::rule(),
                last_sync: LastSync::Succeeded,
                taxonomy: Some(taxonomy),
                read: Ok(&data),
            },
            &queue,
        )
        .await;
        let resolved_topics: BTreeSet<String> = resolution
            .topics
            .iter()
            .map(|topic| topic.topic.as_str().to_owned())
            .collect();
        assert_eq!(resolved_topics, expected);
        assert_eq!(resolution.outcome, RunOutcome::Paused);
    }

    // With no taxonomy, the run is a configuration fault with no topic, and the queue is not asked.
    let studied_data = collection(names, Vec::new(), vec![studied(TODAY - 1, 1)]);
    let queue = RecordingQueue::answering(Vec::new());
    let resolution = day_set::resolve(
        ResolveInputs {
            today: support::today(),
            rule: support::rule(),
            last_sync: LastSync::Succeeded,
            taxonomy: None,
            read: Ok(&studied_data),
        },
        &queue,
    )
    .await;
    assert_eq!(
        resolution.outcome,
        RunOutcome::CouldNotTell(CouldNotTell::TaxonomyMissing)
    );
    assert_eq!(
        TopicState::CouldNotTell(CouldNotTell::TaxonomyMissing).class(),
        Some(Class::ConfigFault)
    );
    assert_eq!(resolution.topics.len(), 0);
    assert_eq!(queue.calls(), 0);

    // A file that is not a taxonomy is refused, without quoting it.
    let refused = Taxonomy::parse(&OTHER.replace("qac", "Not A Slug")).expect_err("a bad code");
    assert!(matches!(
        refused,
        TaxonomyError::Refused {
            field: "languages.code",
            ..
        }
    ));
    assert!(!refused.to_string().contains("Not A Slug"));
    let refused = Taxonomy::parse(&OTHER.replace("writing_roots", "writing")).expect_err("a field");
    assert!(matches!(refused, TaxonomyError::Malformed { .. }));
    let refused = Taxonomy::parse(&OTHER.replace(".v1", ".v2")).expect_err("a schema");
    assert!(matches!(refused, TaxonomyError::OtherSchema));
}

#[test]
fn a_taxonomy_file_that_names_a_deck_badly_is_refused_whole_and_never_quoted() {
    let example = support::example_taxonomy();
    assert_eq!(example.law_roots(), ["Casebook"]);
    assert_eq!(example.bands(), ["Year One", "Year Two", "Year Three"]);
    assert_eq!(example.writing_roots(), ["Composition"]);
    let languages: Vec<(&str, &str, &str, &str)> = example
        .languages()
        .iter()
        .map(|language| {
            (
                language.deck(),
                language.code(),
                language.display(),
                language.term_field(),
            )
        })
        .collect();
    assert_eq!(
        languages,
        [
            ("Tongue Alpha", "qaa", "Alpha", "Headword"),
            ("Tongue Beta", "qab", "Beta", "Headword")
        ]
    );
    // Its debug form counts the names and prints none.
    assert_eq!(
        format!("{example:?}"),
        "Taxonomy(1 law root(s), 3 band(s), 2 language(s), 1 writing root(s))"
    );
    assert_eq!(format!("{:?}", example.languages()[0]), "Language { .. }");

    // A blank name, a name holding the deck separator, a name given twice, and a blank term field
    // are each refused by the field, and the refusal quotes nothing of the file.
    let planted = [
        (OTHER.replace("\"Ledger\"", "\"   \""), "law.roots"),
        (
            OTHER.replace("\"Ledger\"", "\"Led\\u001fger\""),
            "law.roots",
        ),
        (
            OTHER.replace("\"bands\": []", "\"bands\": [\"Tier\", \"Tier\"]"),
            "law.bands",
        ),
        (
            OTHER.replace("\"Composition\"]", "\"Composition\", \"Composition\"]"),
            "writing_roots",
        ),
        (
            OTHER.replace("\"display\": \"Alpha\"", "\"display\": \"\""),
            "languages.display",
        ),
        (
            OTHER.replace("\"deck\": \"Casebook\"", "\"deck\": \"\""),
            "languages.deck",
        ),
        (
            OTHER.replace("\"term_field\": \"Headword\"", "\"term_field\": \" \""),
            "languages.term_field",
        ),
    ];
    for (text, field) in planted {
        let refused = Taxonomy::parse(&text).expect_err(field);
        assert!(
            matches!(refused, TaxonomyError::Refused { field: named, .. } if named == field),
            "{field}: {refused:?}"
        );
        assert!(!refused.to_string().contains("Ledger"), "{refused}");
    }
    let doubled = OTHER.replace(
        "\"term_field\": \"Headword\"}",
        "\"term_field\": \"Headword\"}, {\"deck\": \"Casebook\", \"code\": \"qad\", \
         \"display\": \"Delta\", \"term_field\": \"Headword\"}",
    );
    assert!(matches!(
        Taxonomy::parse(&doubled).expect_err("a deck given twice"),
        TaxonomyError::Refused {
            field: "languages.deck",
            ..
        }
    ));

    // The file's path is a setting: absolute when set, and unset means no taxonomy.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let file = directory.path().join("taxonomy.json");
    let env = Environment::from_vars([(READINGS_TAXONOMY, file.as_os_str())]);
    let path = TaxonomyPath::from_env(&env)
        .expect("an absolute path")
        .expect("the setting is set");
    assert_eq!(path.as_path(), file.as_path());
    let relative = Environment::from_vars([(READINGS_TAXONOMY, "taxonomy.json")]);
    assert_eq!(
        TaxonomyPath::from_env(&relative),
        Err(SettingsError::Malformed {
            setting: READINGS_TAXONOMY,
            expected: "an absolute path to the taxonomy file",
        })
    );
    assert_eq!(TaxonomyPath::from_env(&Environment::default()), Ok(None));
    // A file that is not there cannot be read; one that is reads as the taxonomy it holds.
    assert!(matches!(
        Taxonomy::load(&file),
        Err(TaxonomyError::Unreadable(_))
    ));
    std::fs::write(&file, OTHER).expect("the file is written");
    assert_eq!(
        Taxonomy::load(path.as_path()).expect("the taxonomy"),
        Taxonomy::parse(OTHER).expect("the taxonomy")
    );
}
