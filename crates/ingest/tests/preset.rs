//! SPEC-387 A1 to A6 and A8 to A10: the preset read, the proposal, its settle, its text and the
//! listing, over a preset collection the engine builds from fixed inputs (section 3), and the
//! census that no preset path names an engine write method.
//!
//! The expected presets are the fixture's own definition, and the defaults a proposal must carry
//! are the engine's own, read by the fixture's builder through the engine's deck-options read and
//! never through the preset module.

// An integration test is test code: its helpers panic on a fixture that cannot be built, and it
// prints the examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use deck_streak_ingest::engine::RslibEngine;
use deck_streak_ingest::preset::{
    ParameterField, Preset, PresetCommand, PresetError, PresetSnapshot, PresetStore, Proposal,
    ProposalState, ProposeOutcome, VerifyOutcome, answer, listing, proposal_text, propose_text,
    read_presets, verify_text,
};
use deck_streak_kernel::{Db, UtcMillis};
use support::Fixture;
use support::synthetic::{
    self, PRESET_DEFAULT, PRESET_FIVE, PRESET_FIVE_FSRS5, PRESET_FOUR, PRESET_FOUR_FSRS4,
    PRESET_MAIN, PRESET_MAIN_FSRS6, PRESET_MAIN_RETENTION, PRESET_ON_DEFAULTS, PresetPlan,
    PresetSetup,
};

/// The files whose code may name no engine write method (R8, A8).
const CENSUS: [&str; 2] = [
    "crates/ingest/src/preset.rs",
    "crates/daemon/src/role_preset.rs",
];
/// The engine's write methods, and the write port, as whole identifiers: a call that changes a
/// card, a deck, a preset, a setting or the server's collection, or that pushes a local change.
const WRITE_METHODS: [&str; 22] = [
    "CollectionWrite",
    "set_due_date",
    "update_card",
    "update_cards",
    "answer_card",
    "full_upload",
    "full_download",
    "normal_sync",
    "write_sync",
    "update_deck_configs",
    "add_or_update_deck_config_legacy",
    "add_deck_config",
    "remove_deck_config",
    "update_deck",
    "add_deck",
    "set_current_deck",
    "set_config",
    "set_config_json",
    "set_config_bool",
    "remove_config",
    "transact",
    "transact_no_undo",
];

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn now() -> UtcMillis {
    UtcMillis::from_system_time(SystemTime::now())
}

/// A deployment whose private copy is a preset collection in the process's own zone.
fn copy() -> (Fixture, PresetPlan) {
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let plan = synthetic::build_presets(&fixture.copy(), PresetSetup::process_zone());
    (fixture, plan)
}

/// Every preset the fixture defines, ascending by id, as the read must return it.
fn expected(plan: &PresetPlan) -> Vec<Preset> {
    let deck = |name: &str| plan.decks[name];
    let mut main_decks = vec![deck("Main"), deck("Main::Sub")];
    main_decks.sort_unstable();
    vec![
        Preset {
            id: PRESET_DEFAULT,
            name: "Default".to_owned(),
            vector: Vec::new(),
            field: ParameterField::Empty,
            desired_retention: 0.9,
            deck_ids: vec![deck("Default")],
            non_new_cards: 0,
        },
        Preset {
            id: PRESET_MAIN,
            name: "Main".to_owned(),
            vector: PRESET_MAIN_FSRS6.to_vec(),
            field: ParameterField::Fsrs6,
            desired_retention: PRESET_MAIN_RETENTION,
            deck_ids: main_decks,
            non_new_cards: 5,
        },
        Preset {
            id: PRESET_FIVE,
            name: "Five".to_owned(),
            vector: PRESET_FIVE_FSRS5.to_vec(),
            field: ParameterField::Fsrs5,
            desired_retention: 0.9,
            deck_ids: vec![deck("Five")],
            non_new_cards: 1,
        },
        Preset {
            id: PRESET_FOUR,
            name: "Four".to_owned(),
            vector: PRESET_FOUR_FSRS4.to_vec(),
            field: ParameterField::Fsrs4,
            desired_retention: 0.9,
            deck_ids: Vec::new(),
            non_new_cards: 0,
        },
        Preset {
            id: PRESET_ON_DEFAULTS,
            name: "On defaults".to_owned(),
            vector: plan.defaults.clone(),
            field: ParameterField::Fsrs6,
            desired_retention: 0.9,
            deck_ids: vec![deck("Fresh")],
            non_new_cards: 0,
        },
    ]
}

/// The fixture's preset with the id `id`.
fn fixture_preset(plan: &PresetPlan, id: i64) -> Preset {
    expected(plan)
        .into_iter()
        .find(|preset| preset.id == id)
        .expect("the fixture defines the preset")
}

/// The values of a line `<label><v1>, <v2>, ...`, each parsed as the owner's app parses a value.
fn values_after(text: &str, label: &str) -> Vec<f32> {
    let line = text
        .lines()
        .find_map(|line| line.strip_prefix(label))
        .unwrap_or_else(|| panic!("no line starts {label:?}: {text:?}"));
    line.split(", ")
        .map(|value| value.parse().expect("each value parses"))
        .collect()
}

/// How many proposals the record holds, and how many of them are open, for `preset`.
async fn rows_of(db: &Db, preset: i64) -> (i64, i64) {
    let mut write = db.write().await.expect("a read in a write");
    let counted: (i64, i64) = sqlx::query_as(
        "SELECT count(*), coalesce(sum(state = 'open'), 0) FROM preset_proposals \
         WHERE preset_id = ?1",
    )
    .bind(preset)
    .fetch_one(&mut *write)
    .await
    .expect("the record is counted");
    write.commit().await.expect("the read ends");
    counted
}

/// Every row of the record, each as `(id, state, settled_at, retention_kept)`.
async fn states(db: &Db) -> Vec<(i64, String, Option<i64>, Option<i64>)> {
    let mut write = db.write().await.expect("a read in a write");
    let rows = sqlx::query_as(
        "SELECT id, state, settled_at, retention_kept FROM preset_proposals ORDER BY id",
    )
    .fetch_all(&mut *write)
    .await
    .expect("the record is read");
    write.commit().await.expect("the read ends");
    rows
}

/// The proposal `propose` recorded, or a panic naming what it answered instead.
async fn recorded(store: &PresetStore, preset: &Preset, defaults: &[f32], at: i64) -> Proposal {
    match store
        .propose(preset, defaults, UtcMillis::from_epoch_millis(at))
        .await
        .expect("the proposal runs")
    {
        ProposeOutcome::Recorded(proposal) => proposal,
        other => panic!("nothing was recorded: {other:?}"),
    }
}

#[tokio::test]
async fn every_preset_is_read_with_its_vector_retention_decks_and_non_new_cards() {
    let (fixture, plan) = copy();
    let snapshot = read_presets(&RslibEngine, &fixture.settings(), now())
        .await
        .expect("the copy is read");
    assert_eq!(snapshot.presets, expected(&plan));
    assert_eq!(snapshot.defaults, plan.defaults);
    assert_eq!(
        plan.defaults.len(),
        21,
        "the engine's defaults are FSRS-6's"
    );
}

#[tokio::test]
async fn the_stored_vector_comes_from_the_newest_field_that_holds_one() {
    let (fixture, plan) = copy();
    let snapshot = read_presets(&RslibEngine, &fixture.settings(), now())
        .await
        .expect("the copy is read");
    let fields: Vec<(i64, ParameterField)> = snapshot
        .presets
        .iter()
        .map(|preset| (preset.id, preset.field))
        .collect();
    assert_eq!(
        fields,
        vec![
            (PRESET_DEFAULT, ParameterField::Empty),
            (PRESET_MAIN, ParameterField::Fsrs6),
            (PRESET_FIVE, ParameterField::Fsrs5),
            (PRESET_FOUR, ParameterField::Fsrs4),
            (PRESET_ON_DEFAULTS, ParameterField::Fsrs6),
        ]
    );
    let vectors: Vec<(i64, Vec<f32>)> = snapshot
        .presets
        .iter()
        .map(|preset| (preset.id, preset.vector.clone()))
        .collect();
    assert_eq!(
        vectors,
        vec![
            (PRESET_DEFAULT, Vec::new()),
            (PRESET_MAIN, PRESET_MAIN_FSRS6.to_vec()),
            (PRESET_FIVE, PRESET_FIVE_FSRS5.to_vec()),
            (PRESET_FOUR, PRESET_FOUR_FSRS4.to_vec()),
            (PRESET_ON_DEFAULTS, plan.defaults.clone()),
        ]
    );
}

#[tokio::test]
async fn a_proposal_carries_the_released_defaults_and_records_the_undo_values() {
    let (fixture, plan) = copy();
    let db = fixture.db().await;
    let store = PresetStore::new(db.clone());
    let main = fixture_preset(&plan, PRESET_MAIN);
    assert_eq!(
        plan.defaults.len(),
        21,
        "the engine's defaults are FSRS-6's"
    );
    assert_ne!(
        plan.defaults, main.vector,
        "the fixture's fit is not the defaults"
    );
    let proposal = recorded(&store, &main, &plan.defaults, 7_000).await;
    assert_eq!(
        proposal.proposed_vector, plan.defaults,
        "the proposed vector is the engine's defaults"
    );
    assert_eq!(
        proposal,
        Proposal {
            id: proposal.id,
            preset_id: PRESET_MAIN,
            preset_name: "Main".to_owned(),
            prior_vector: PRESET_MAIN_FSRS6.to_vec(),
            prior_field: ParameterField::Fsrs6,
            proposed_vector: plan.defaults.clone(),
            desired_retention: PRESET_MAIN_RETENTION,
            non_new_cards: 5,
            state: ProposalState::Open,
            settled_at: None,
            retention_kept: None,
            created_at: 7_000,
        }
    );
    // The record holds the same values, each vector as a JSON array that parses back exactly.
    let mut write = db.write().await.expect("a read in a write");
    let row: (i64, String, String, String, f64, i64, String, i64) = sqlx::query_as(
        "SELECT preset_id, preset_name, prior_vector, prior_field, desired_retention, \
         non_new_cards, proposed_vector, created_at FROM preset_proposals",
    )
    .fetch_one(&mut *write)
    .await
    .expect("one row is recorded");
    write.commit().await.expect("the read ends");
    let parsed = |text: &str| -> Vec<f32> {
        let inner = text
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
            .expect("a JSON array");
        inner
            .split(',')
            .map(|value| value.parse().expect("a number"))
            .collect()
    };
    assert_eq!(
        (row.0, row.1.as_str(), parsed(&row.2), row.3.as_str()),
        (PRESET_MAIN, "Main", PRESET_MAIN_FSRS6.to_vec(), "fsrs6")
    );
    assert_eq!(row.4.to_bits(), f64::from(PRESET_MAIN_RETENTION).to_bits());
    assert_eq!((row.5, parsed(&row.6), row.7), (5, plan.defaults, 7_000));
}

#[tokio::test]
async fn a_preset_already_on_the_defaults_records_no_proposal() {
    let (fixture, plan) = copy();
    let db = fixture.db().await;
    let store = PresetStore::new(db.clone());
    let fresh = fixture_preset(&plan, PRESET_ON_DEFAULTS);
    assert_eq!(
        fresh.vector, plan.defaults,
        "the fixture's preset is on the defaults"
    );
    let outcome = store
        .propose(&fresh, &plan.defaults, now())
        .await
        .expect("the proposal runs");
    assert_eq!(
        rows_of(&db, PRESET_ON_DEFAULTS).await,
        (0, 0),
        "nothing is recorded"
    );
    assert_eq!(outcome, ProposeOutcome::OnDefaults);
    let text = propose_text(&fresh, &outcome);
    assert!(
        text.contains("On defaults is already on the scheduler's defaults"),
        "{text:?}"
    );
}

#[tokio::test]
async fn a_preset_holds_at_most_one_open_proposal() {
    let (fixture, plan) = copy();
    let db = fixture.db().await;
    let store = PresetStore::new(db.clone());
    let main = fixture_preset(&plan, PRESET_MAIN);
    let first = recorded(&store, &main, &plan.defaults, 7_000).await;
    let again = store
        .propose(&main, &plan.defaults, UtcMillis::from_epoch_millis(8_000))
        .await
        .expect("the second proposal runs");
    assert_eq!(again, ProposeOutcome::AlreadyOpen(first.clone()));
    assert_eq!(rows_of(&db, PRESET_MAIN).await, (1, 1));
    let text = propose_text(&main, &again);
    assert!(text.contains("already open"), "{text:?}");
    assert!(text.contains(&proposal_text(&first)), "{text:?}");

    // Two connections racing on one preset leave exactly one open row.
    let five = fixture_preset(&plan, PRESET_FIVE);
    let other = PresetStore::new(db.clone());
    let (left, right) = tokio::join!(
        store.propose(&five, &plan.defaults, now()),
        other.propose(&five, &plan.defaults, now())
    );
    let outcomes = [
        left.expect("one racer runs"),
        right.expect("the other racer runs"),
    ];
    let recorded_count = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ProposeOutcome::Recorded(_)))
        .count();
    let reused = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ProposeOutcome::AlreadyOpen(_)))
        .count();
    assert_eq!((recorded_count, reused), (1, 1), "{outcomes:?}");
    assert_eq!(rows_of(&db, PRESET_FIVE).await, (1, 1));

    // A direct second open row for one preset is refused by the partial unique index.
    let mut write = db.write().await.expect("a write");
    let direct = sqlx::query(
        "INSERT INTO preset_proposals (preset_id, preset_name, prior_vector, prior_field, \
         proposed_vector, desired_retention, non_new_cards, state, created_at) \
         VALUES (?1, 'Main', '[]', 'empty', '[]', 0.9, 0, 'open', 9000)",
    )
    .bind(PRESET_MAIN)
    .execute(&mut *write)
    .await;
    assert!(direct.is_err(), "a second open row was admitted");
    drop(write);

    // A settled proposal leaves room for a new open one beside it.
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO preset_proposals (preset_id, preset_name, prior_vector, prior_field, \
         proposed_vector, desired_retention, non_new_cards, state, settled_at, retention_kept, \
         created_at) VALUES (?1, 'Four', '[]', 'empty', '[]', 0.9, 0, 'moved', 9500, 1, 9000)",
    )
    .bind(PRESET_FOUR)
    .execute(&mut *write)
    .await
    .expect("a settled row is recorded");
    write.commit().await.expect("the settled row commits");
    let four = fixture_preset(&plan, PRESET_FOUR);
    recorded(&store, &four, &plan.defaults, 10_000).await;
    assert_eq!(rows_of(&db, PRESET_FOUR).await, (2, 1));
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn verify_settles_an_open_proposal_once_by_what_the_copy_holds() {
    let (fixture, plan) = copy();
    let db = fixture.db().await;
    let store = PresetStore::new(db.clone());
    let main = fixture_preset(&plan, PRESET_MAIN);
    let proposal = recorded(&store, &main, &plan.defaults, 7_000).await;
    let holding = |vector: &[f32], retention: f32| PresetSnapshot {
        presets: vec![Preset {
            vector: vector.to_vec(),
            desired_retention: retention,
            ..main.clone()
        }],
        defaults: plan.defaults.clone(),
    };

    // The copy still holds the prior vector: the proposal stays open and nothing is recorded.
    let open = store
        .verify(
            &holding(&PRESET_MAIN_FSRS6, PRESET_MAIN_RETENTION),
            proposal.id,
            UtcMillis::from_epoch_millis(8_000),
        )
        .await
        .expect("the verify runs");
    assert_eq!(open, VerifyOutcome::Unchanged(proposal.clone()));

    // The copy holds the proposed vector: settled moved, with the time and the retention kept.
    let moved = store
        .verify(
            &holding(&plan.defaults, PRESET_MAIN_RETENTION),
            proposal.id,
            UtcMillis::from_epoch_millis(9_000),
        )
        .await
        .expect("the verify runs");
    let settled = Proposal {
        state: ProposalState::Moved,
        settled_at: Some(9_000),
        retention_kept: Some(true),
        ..proposal.clone()
    };
    assert_eq!(moved, VerifyOutcome::Settled(settled.clone()));
    assert!(verify_text(&open).contains("still open"), "{open:?}");
    assert!(verify_text(&moved).contains("moved at 9000"), "{moved:?}");
    assert!(
        verify_text(&moved).contains("desired retention unchanged"),
        "{moved:?}"
    );

    // A settled proposal is never settled again, whatever the copy holds next.
    let diverging: Vec<f32> = PRESET_FIVE_FSRS5.to_vec();
    let again = store
        .verify(
            &holding(&diverging, PRESET_MAIN_RETENTION),
            proposal.id,
            UtcMillis::from_epoch_millis(10_000),
        )
        .await
        .expect("the verify runs");
    assert_eq!(again, VerifyOutcome::Unchanged(settled.clone()));
    assert!(
        verify_text(&again).contains("already settled moved"),
        "{again:?}"
    );
    assert_eq!(
        states(&db).await,
        vec![(proposal.id, "moved".to_owned(), Some(9_000), Some(1))]
    );

    // Neither the prior vector nor the proposed one: settled diverged.
    let five = fixture_preset(&plan, PRESET_FIVE);
    let second = recorded(&store, &five, &plan.defaults, 11_000).await;
    let elsewhere = PresetSnapshot {
        presets: vec![Preset {
            vector: PRESET_FOUR_FSRS4.to_vec(),
            ..five.clone()
        }],
        defaults: plan.defaults.clone(),
    };
    let diverged = store
        .verify(&elsewhere, second.id, UtcMillis::from_epoch_millis(12_000))
        .await
        .expect("the verify runs");
    assert_eq!(
        diverged,
        VerifyOutcome::Settled(Proposal {
            state: ProposalState::Diverged,
            settled_at: Some(12_000),
            retention_kept: None,
            ..second
        })
    );
    assert!(verify_text(&diverged).contains("diverged"), "{diverged:?}");

    // Moved with the desired retention changed in the same save.
    let four = fixture_preset(&plan, PRESET_FOUR);
    let third = recorded(&store, &four, &plan.defaults, 13_000).await;
    let changed = PresetSnapshot {
        presets: vec![Preset {
            vector: plan.defaults.clone(),
            desired_retention: 0.8,
            ..four.clone()
        }],
        defaults: plan.defaults.clone(),
    };
    let kept = store
        .verify(&changed, third.id, UtcMillis::from_epoch_millis(14_000))
        .await
        .expect("the verify runs");
    assert_eq!(
        kept,
        VerifyOutcome::Settled(Proposal {
            state: ProposalState::Moved,
            settled_at: Some(14_000),
            retention_kept: Some(false),
            ..third
        })
    );
    assert!(
        verify_text(&kept).contains("desired retention changed"),
        "{kept:?}"
    );
    assert!(matches!(
        store.verify(&changed, 999, now()).await,
        Err(PresetError::UnknownProposal)
    ));
}

/// Each line of `text` whose code (outside a whole-line comment or a trailing ` //` one) names a
/// write method as a whole identifier, as `<path>:<line>: <name>`.
fn findings(path: &str, text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        let code = trimmed.split(" //").next().unwrap_or_default();
        for word in code.split(|character: char| !(character.is_alphanumeric() || character == '_'))
        {
            if WRITE_METHODS.contains(&word) {
                found.push(format!("{path}:{}: {word}", number + 1));
            }
        }
    }
    found
}

#[test]
fn no_preset_path_names_an_engine_write_method() {
    // The planted line is refused by the name it holds, first, so a census gone blind fails.
    assert_eq!(
        findings("planted.rs", "use crate::engine::CollectionWrite;\n"),
        vec!["planted.rs:1: CollectionWrite".to_owned()]
    );
    assert_eq!(
        findings(
            "planted.rs",
            "let _ = col.get_deck_configs_for_update(deck);\n"
        ),
        Vec::<String>::new(),
        "the engine's deck-options read is not a write method"
    );
    let files = examined(
        "files",
        CENSUS
            .iter()
            .map(|path| {
                let text = fs::read_to_string(root().join(path)).expect("the file is read");
                (*path, text)
            })
            .collect(),
    );
    let found: Vec<String> = files
        .iter()
        .flat_map(|(path, text)| findings(path, text))
        .collect();
    assert_eq!(found, Vec::<String>::new());
}

#[tokio::test]
async fn the_proposal_text_names_the_values_the_steps_the_reach_and_the_undo() {
    let (fixture, plan) = copy();
    let store = PresetStore::new(fixture.db().await);
    let main = fixture_preset(&plan, PRESET_MAIN);
    let proposal = recorded(&store, &main, &plan.defaults, 7_000).await;
    let text = proposal_text(&proposal);
    assert!(text.contains("Main"), "the text names the preset: {text:?}");
    let values = values_after(&text, "Values: ");
    assert_eq!(values.len(), 21);
    assert_eq!(
        values, plan.defaults,
        "the values parse back to the defaults"
    );
    for step in [
        "open this preset's options",
        "paste the values above into its FSRS parameters",
        "Leave \"Reschedule cards on change\" off",
        "leave desired retention as it is",
        "Save, then sync.",
        "the memory state of 5 non-new card(s) is recomputed from their own reviews",
        "No review, due date or interval changes.",
        "These defaults replace a fit of the current scheduler generation.",
    ] {
        assert!(text.contains(step), "the text lacks {step:?}: {text:?}");
    }
    assert_eq!(
        values_after(&text, "Undo: paste these values back: "),
        PRESET_MAIN_FSRS6.to_vec(),
        "the undo values parse back to the prior vector"
    );
    let shown = propose_text(&main, &ProposeOutcome::Recorded(proposal.clone()));
    assert!(
        shown.contains(&format!("Proposal {} recorded", proposal.id)),
        "{shown:?}"
    );
    assert!(shown.contains(&text), "{shown:?}");

    // An FSRS-5 vector is no fit of the current generation, and its undo pastes it back.
    let five = fixture_preset(&plan, PRESET_FIVE);
    let older = proposal_text(&recorded(&store, &five, &plan.defaults, 8_000).await);
    assert!(!older.contains("replace a fit"), "{older:?}");
    assert_eq!(
        values_after(&older, "Undo: paste these values back: "),
        PRESET_FIVE_FSRS5.to_vec()
    );

    // An empty field's undo clears the box.
    let default = fixture_preset(&plan, PRESET_DEFAULT);
    let empty = proposal_text(&recorded(&store, &default, &plan.defaults, 9_000).await);
    assert!(
        empty.contains("Undo: clear the parameters box."),
        "{empty:?}"
    );
    assert!(!empty.contains("replace a fit"), "{empty:?}");
}

#[tokio::test]
async fn the_listing_puts_the_main_preset_first() {
    let (fixture, _plan) = copy();
    let settings = fixture.settings();
    let snapshot = read_presets(&RslibEngine, &settings, now())
        .await
        .expect("the copy is read");
    let expected = [
        "1001 Main: field fsrs6, not on the defaults, desired retention 0.85, 2 deck(s), \
         5 non-new card(s)",
        "1002 Five: field fsrs5, not on the defaults, desired retention 0.9, 1 deck(s), \
         1 non-new card(s)",
        "1 Default: field empty, not on the defaults, desired retention 0.9, 1 deck(s), \
         0 non-new card(s)",
        "1003 Four: field fsrs4, not on the defaults, desired retention 0.9, 0 deck(s), \
         0 non-new card(s)",
        "1004 On defaults: field fsrs6, on the defaults, desired retention 0.9, 1 deck(s), \
         0 non-new card(s)",
    ];
    assert_eq!(listing(&snapshot), expected);
    let db = fixture.db().await;
    let answered = answer(&RslibEngine, &db, &settings, PresetCommand::List, now())
        .await
        .expect("the role's answer");
    assert_eq!(answered, expected.join("\n"));
}

#[tokio::test]
async fn a_missing_copy_is_refused_and_never_created() {
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let read = read_presets(&RslibEngine, &fixture.settings(), now()).await;
    assert!(matches!(read, Err(PresetError::NoCopy)), "{read:?}");
    assert!(!fixture.copy().exists(), "the read created a copy");
}

#[tokio::test]
async fn a_copy_whose_offset_differs_from_the_process_zone_is_refused() {
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let setup = PresetSetup {
        utc_offset_west: Some(synthetic::process_offset_west() + 60),
        rollover: Some(4),
    };
    synthetic::build_presets(&fixture.copy(), setup);
    let read = read_presets(&RslibEngine, &fixture.settings(), now()).await;
    assert!(matches!(read, Err(PresetError::ZoneDiffers)), "{read:?}");
}
