//! SPEC-338 A1, A2 and A17: the study rule the web engine's `wasm32` module runs, judged natively.

// The examined helper prints its count on purpose; clippy.toml's in-test allowances cover only
// `#[test]` bodies.
#![allow(clippy::print_stdout)]

use deck_streak_web_engine::study::{
    Answer, BuryOf, Files, STUDY_CALLS, Shown, StudyError, Wanted, admit, bury_of,
    engine_languages, service, shown_for, toggled_red,
};

fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[test]
fn a_rating_on_the_wire_picks_its_answer_and_its_next_state() {
    // Anki's buttons number the answers 1 to 4; its Rating enum numbers them 0 to 3, and its
    // scheduling states hold one next state per answer, in the same order.
    let expected = [
        (1, Answer::Again, 0, "again"),
        (2, Answer::Hard, 1, "hard"),
        (3, Answer::Good, 2, "good"),
        (4, Answer::Easy, 3, "easy"),
    ];
    for (wire, answer, rating, state) in examined("wire rating(s)", expected.to_vec()) {
        let got = Answer::from_wire(wire);
        assert_eq!(got, Ok(answer), "wire rating {wire}");
        assert_eq!(answer.rating(), rating, "{answer:?}'s Rating");
        assert_eq!(
            answer.pick("again", "hard", "good", "easy"),
            state,
            "{answer:?}'s next state"
        );
    }
}

#[test]
fn a_rating_outside_one_to_four_is_refused() {
    for wire in examined("out-of-range rating(s)", vec![0, 5, 6, 255, u32::MAX]) {
        let refused = Answer::from_wire(wire);
        assert_eq!(
            refused,
            Err(StudyError::RatingOutOfRange(wire)),
            "wire rating {wire}"
        );
        assert_eq!(
            refused.map_err(|error| error.to_string()),
            Err(format!("rating {wire} is outside 1 to 4"))
        );
    }
}

#[test]
fn run_method_admits_only_the_study_calls() {
    // The oracle, written apart from the table: the eight calls open, close, undo, the queue,
    // the answer, the notetype names and the two calls that seed a synthetic collection.
    let study = [
        (3, 0, "open_collection"),
        (3, 1, "close_collection"),
        (3, 8, "undo"),
        (13, 3, "get_queued_cards"),
        (13, 4, "answer_card"),
        (23, 8, "get_notetype_names"),
        (25, 0, "new_note"),
        (25, 2, "add_notes"),
        // The review's eight: the deck list, the card view, its labels and undo label, bury and
        // flag (SPEC-350 R1, M10).
        (7, 4, "deck_tree"),
        (7, 22, "set_current_deck"),
        (27, 6, "render_existing_card"),
        (27, 9, "strip_av_tags"),
        (13, 24, "describe_next_states"),
        (3, 7, "get_undo_status"),
        (13, 14, "bury_or_suspend_cards"),
        (5, 4, "set_flag"),
    ];
    for (svc, method, name) in examined("study call(s)", study.to_vec()) {
        assert_eq!(
            admit(svc, method),
            Ok(name),
            "service {svc} method {method}"
        );
    }
    // ADR-337's exempt writes, by index at the pinned commit: Forget, set due date, upgrading the
    // scheduler, deleting a preset, updating presets, deleting cards, changing a note's type and
    // deleting notes. Each is refused by name.
    let exempt = [
        (13, 17, "schedule_cards_as_new"),
        (13, 19, "set_due_date"),
        (13, 26, "upgrade_scheduler"),
        (11, 5, "remove_deck_config"),
        (11, 7, "update_deck_configs"),
        (5, 2, "remove_cards"),
        (23, 15, "change_notetype"),
        (25, 7, "remove_notes"),
    ];
    for (svc, method, name) in examined("exempt write(s)", exempt.to_vec()) {
        assert_eq!(
            admit(svc, method).map_err(|error| error.to_string()),
            Err(format!(
                "run_method refuses service {svc} method {method}: not a study call"
            )),
            "{name}"
        );
    }
    // Every pair the dispatcher can number is refused unless it is a study call.
    let mut admitted = Vec::new();
    let pairs: Vec<(u32, u32)> = (0..=64)
        .flat_map(|svc| (0..=64).map(move |method| (svc, method)))
        .collect();
    for (svc, method) in examined("service and method pair(s)", pairs) {
        match admit(svc, method) {
            Ok(_) => admitted.push((svc, method)),
            Err(error) => {
                assert_eq!(
                    error,
                    StudyError::CallRefused {
                        service: svc,
                        method
                    }
                );
            }
        }
    }
    let mut expected: Vec<(u32, u32)> = study.iter().map(|&(s, m, _)| (s, m)).collect();
    expected.sort_unstable();
    assert_eq!(admitted, expected);
    // The table the module exports names the same calls under the same services.
    assert_eq!(STUDY_CALLS.len(), study.len());
    assert_eq!(
        [
            service::COLLECTION,
            service::SCHEDULER,
            service::NOTETYPES,
            service::NOTES
        ],
        [3, 13, 23, 25]
    );
}

#[test]
fn the_study_calls_are_the_reviews_pairs() {
    // The oracle, written apart from the table (SPEC-350 R1, M10): DEV's eight calls, then the
    // review's eight, each named as the engine names its method.
    let review = vec![
        (3, 0, "open_collection"),
        (3, 1, "close_collection"),
        (3, 8, "undo"),
        (13, 3, "get_queued_cards"),
        (13, 4, "answer_card"),
        (23, 8, "get_notetype_names"),
        (25, 0, "new_note"),
        (25, 2, "add_notes"),
        (7, 4, "deck_tree"),
        (7, 22, "set_current_deck"),
        (27, 6, "render_existing_card"),
        (27, 9, "strip_av_tags"),
        (13, 24, "describe_next_states"),
        (3, 7, "get_undo_status"),
        (13, 14, "bury_or_suspend_cards"),
        (5, 4, "set_flag"),
    ];
    let review = examined("review study call(s)", review);
    assert_eq!(STUDY_CALLS.to_vec(), review);
    for &(svc, method, name) in &review {
        assert_eq!(
            admit(svc, method),
            Ok(name),
            "service {svc} method {method}"
        );
    }
}

#[test]
fn only_the_shown_card_is_rated_buried_or_flagged() {
    // SPEC-350 R2: the kept card is card 42; a gesture for card 41 reaches nothing.
    let kept = Shown {
        card: 42,
        states: "the states shown with card 42",
        flag: 0,
    };
    assert_eq!(shown_for(Some(&kept), 41), Err(StudyError::NotShown));
    assert_eq!(shown_for::<&str>(None, 42), Err(StudyError::NotShown));
    assert_eq!(
        shown_for(Some(&kept), 42),
        Ok(&Shown {
            card: 42,
            states: "the states shown with card 42",
            flag: 0,
        })
    );
    assert_eq!(
        StudyError::NotShown.to_string(),
        "not-shown: the card is not the one on screen"
    );
}

#[test]
fn the_flag_toggles_red() {
    // SPEC-350 R7: no flag turns red, red turns to none, and any other flag turns red.
    assert_eq!(toggled_red(0), 1);
    assert_eq!(toggled_red(1), 0);
    assert_eq!(toggled_red(2), 1);
    assert_eq!(toggled_red(7), 1);
}

#[test]
fn bury_is_the_users_bury_of_the_shown_card() {
    // SPEC-350 R2: one card, no note, and the user's bury, the engine's mode 2.
    assert_eq!(
        bury_of(42),
        BuryOf {
            card_ids: vec![42],
            note_ids: vec![],
            mode: 2,
        }
    );
}

#[test]
fn the_engine_speaks_english_when_no_language_is_given() {
    // SPEC-350 R4: the page's list reaches the engine as sent, and an empty one is English.
    assert_eq!(engine_languages(Vec::new()), vec!["en".to_owned()]);
    assert_eq!(
        engine_languages(vec!["ja".to_owned(), "en".to_owned()]),
        vec!["ja".to_owned(), "en".to_owned()]
    );
}

#[test]
fn each_name_the_core_asks_for_is_wanted_once() {
    // SPEC-350 R14: a name the files lack is wanted once, with the limit the core asked for; a
    // name the files hold is answered and not wanted.
    let files = Files::new([("kept.png".to_owned(), vec![1, 2, 3])]);
    let wanted = Wanted::new(&files);
    assert_eq!(wanted.ask("cat.mp3", 9), None);
    assert_eq!(wanted.ask("cat.mp3", 9), None);
    assert_eq!(wanted.ask("kept.png", 9), Some(vec![1, 2, 3]));
    assert_eq!(wanted.ask("dog.ogg", 7), None);
    assert_eq!(
        wanted.into_names(),
        vec![("cat.mp3".to_owned(), 9), ("dog.ogg".to_owned(), 7)]
    );
}

#[test]
fn a_file_is_read_no_further_than_its_limit() {
    // SPEC-350 R14: the core asks for one byte past its cap and decides on the length it reads.
    let files = Files::new([
        ("long.wav".to_owned(), vec![1, 2, 3, 4, 5]),
        ("short.wav".to_owned(), vec![6]),
    ]);
    assert_eq!(files.read("long.wav", 3), Some(vec![1, 2, 3]));
    assert_eq!(files.read("long.wav", 0), Some(vec![]));
    assert_eq!(files.read("short.wav", 3), Some(vec![6]));
    assert_eq!(files.read("long.wav", u64::MAX), Some(vec![1, 2, 3, 4, 5]));
    assert_eq!(files.read("absent.wav", 3), None);
}
