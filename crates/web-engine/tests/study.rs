//! SPEC-338 A1, A2 and A17: the study rule the web engine's `wasm32` module runs, judged natively.

use deck_streak_web_engine::study::{Answer, STUDY_CALLS, StudyError, admit, service};

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
