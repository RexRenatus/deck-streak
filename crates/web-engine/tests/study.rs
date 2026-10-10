//! SPEC-338 A1, A2 and A17, SPEC-371 A16 and SPEC-383 A17 to A19: the study rule the web
//! engine's `wasm32` module runs, judged natively, with the undo's mirrors of the core's verdicts
//! (SPEC-371 R6, R7; SPEC-383 R9).

// The examined helper prints its count on purpose; clippy.toml's in-test allowances cover only
// `#[test]` bodies.
#![allow(clippy::print_stdout)]

use deck_streak_web_engine::study::{
    Change, Files, Grade, LastAnswer, LastMark, Recorded, Returns, STUDY_CALLS, Shown, StudyError,
    UndoRefusal, Wanted, admit, engine_languages, flag_change, grade, last_answer_for,
    last_mark_for, mark_view, media_type, service, shown_for, undo_view,
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
    // SPEC-365 A9: the wire names two grades. Hard (2) and Easy (4) are refused by name, before
    // anything reaches the engine, and so is a number outside Anki's buttons.
    let refused = [
        (2, StudyError::NotAGrade(2)),
        (4, StudyError::NotAGrade(4)),
        (0, StudyError::RatingOutOfRange(0)),
        (5, StudyError::RatingOutOfRange(5)),
    ];
    for (wire, error) in examined("refused wire rating(s)", refused.to_vec()) {
        assert_eq!(grade(wire), Err(error), "wire rating {wire}");
    }
    assert_eq!(
        StudyError::NotAGrade(4).to_string(),
        "rating 4 is not a grade: a press records 1 or 3"
    );
    // Anki's buttons number Again 1 and Good 3; its Rating enum numbers them 0 and 2, and each
    // picks its own next state of the two the card was shown with.
    let graded = [(1, Grade::Again, 0, "again"), (3, Grade::Good, 2, "good")];
    for (wire, expected, rating, state) in examined("graded wire rating(s)", graded.to_vec()) {
        assert_eq!(grade(wire), Ok(expected), "wire rating {wire}");
        assert_eq!(expected.rating(), rating, "{expected:?}'s Rating");
        assert_eq!(
            expected.pick("again", "good"),
            state,
            "{expected:?}'s next state"
        );
    }
}

#[test]
fn a_rating_outside_one_to_four_is_refused() {
    for wire in examined("out-of-range rating(s)", vec![0, 5, 6, 255, u32::MAX]) {
        let refused = grade(wire);
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
    // SPEC-365 A10: AnswerCard is no study call. Only an owner's press records a grade, through
    // the core's answer door, so `run_method` refuses it as it refuses any other pair.
    assert_eq!(
        admit(13, 4),
        Err(StudyError::CallRefused {
            service: 13,
            method: 4
        }),
        "AnswerCard"
    );
    // SPEC-371 A16 (R6): Undo is an exempt write, held for the owner's gesture, so `run_method`
    // refuses it as it refuses Forget; the card's one line of text is a study call.
    assert_eq!(
        (admit(3, 8), admit(27, 14)),
        (
            Err(StudyError::CallRefused {
                service: 3,
                method: 8
            }),
            Ok("html_to_text_line")
        ),
        "Undo is refused and HtmlToTextLine admitted"
    );
    // The oracle, written apart from the table: the six calls open, close, the queue, the
    // notetype names and the two calls that seed a synthetic collection.
    let study = [
        (3, 0, "open_collection"),
        (3, 1, "close_collection"),
        (13, 3, "get_queued_cards"),
        (23, 8, "get_notetype_names"),
        (25, 0, "new_note"),
        (25, 2, "add_notes"),
        // The review's nine: the deck list, the card view, its labels and undo label, bury and
        // flag (SPEC-350 R1, M10), and the card's one line of text (SPEC-371 R6).
        (7, 4, "deck_tree"),
        (7, 22, "set_current_deck"),
        (27, 6, "render_existing_card"),
        (27, 9, "strip_av_tags"),
        (27, 14, "html_to_text_line"),
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
    // scheduler, deleting a preset, updating presets, deleting cards, changing a note's type,
    // deleting notes and, since SPEC-371, undo. Each is refused by name.
    let exempt = [
        (3, 8, "undo"),
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
    // The oracle, written apart from the table (SPEC-350 R1, M10): DEV's seven calls, less the
    // answer only an owner's press records (SPEC-365 R7) and the undo only the owner's gesture
    // runs (SPEC-371 R6), then the review's nine, each named as the engine names its method.
    let review = vec![
        (3, 0, "open_collection"),
        (3, 1, "close_collection"),
        (13, 3, "get_queued_cards"),
        (23, 8, "get_notetype_names"),
        (25, 0, "new_note"),
        (25, 2, "add_notes"),
        (7, 4, "deck_tree"),
        (7, 22, "set_current_deck"),
        (27, 6, "render_existing_card"),
        (27, 9, "strip_av_tags"),
        (27, 14, "html_to_text_line"),
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

#[test]
fn a_sound_takes_the_type_the_table_gives_its_name() {
    // SPEC-350 R15: the table is the caller's, its extension read without case; a name with no
    // extension, or one the table lacks, has no type.
    let types = [("mp3", "audio/mpeg"), ("ogg", "audio/ogg")];
    assert_eq!(media_type("cat.MP3", &types), Some("audio/mpeg"));
    assert_eq!(media_type("dog.wav.ogg", &types), Some("audio/ogg"));
    assert_eq!(media_type("bird.wav", &types), None);
    assert_eq!(media_type("ogg", &types), None);
}

#[test]
fn the_sync_calls_are_the_login_and_the_normal_sync() {
    // The oracle, written apart from the list: the sync service is the backend's first, its login
    // is its method 3 and its normal sync its method 5, each named as the engine names its method
    // (SPEC-364 R1). The one-way sync (1,6) is the core's alone and never the web engine's.
    let calls = deck_streak_web_engine::study::SYNC_CALLS.to_vec();
    assert_eq!(
        calls,
        vec![(1, 3, "sync_login"), (1, 5, "sync_collection")],
        "the web side names the sync login and the normal sync, and no other sync pair"
    );
    examined("sync call(s)", calls);
}

#[test]
fn a_confirmation_reaches_only_the_kept_answer_at_its_step() {
    // SPEC-371 R7: the page confirms only what it was offered, so a confirmation that names another
    // card, another step, or an answer when none is kept is refused as changed.
    let kept = LastAnswer {
        card: 42,
        grade: Grade::Good,
        returns: Returns::Learning,
        recorded: Recorded {
            step: 7,
            label: "Answer Card".to_owned(),
            review: 1001,
        },
    };
    assert_eq!(last_answer_for(Some(&kept), 42, 7), Ok(&kept));
    let stale = vec![(41, 7), (42, 8), (41, 8), (42, 6)];
    for (card, step) in examined("stale confirmation(s)", stale) {
        assert_eq!(
            last_answer_for(Some(&kept), card, step),
            Err(StudyError::NotUndoable(UndoRefusal::Changed)),
            "card {card} at step {step}"
        );
    }
    assert_eq!(
        last_answer_for(None, 42, 7),
        Err(StudyError::NotUndoable(UndoRefusal::Changed))
    );
}

#[test]
fn a_refused_undo_reads_as_its_own_sentence() {
    // SPEC-371 R12: the session reads the prefix, `undo-synced` for a synced answer and
    // `not-undoable` for every other refusal, so each sentence is held whole.
    let sentences = vec![
        (UndoRefusal::Synced, "undo-synced: the answer has synced"),
        (UndoRefusal::Gone, "not-undoable: the answer is gone"),
        (
            UndoRefusal::NotTheCard,
            "not-undoable: the answer is of another card",
        ),
        (
            UndoRefusal::Changed,
            "not-undoable: something changed after the answer",
        ),
    ];
    for (refusal, sentence) in examined("undo refusal(s)", sentences) {
        assert_eq!(
            StudyError::NotUndoable(refusal).to_string(),
            sentence,
            "{refusal:?}"
        );
    }
}

#[test]
fn the_offer_names_its_grade_and_state_and_why_none_is_made() {
    // SPEC-371 R7: the offer's words, each written here apart from the rule.
    assert_eq!([Grade::Again.word(), Grade::Good.word()], ["again", "good"]);
    let kinds = vec![
        (Returns::New, "new"),
        (Returns::Learning, "learning"),
        (Returns::Review, "review"),
        (Returns::Relearning, "relearning"),
        (Returns::Preview, "preview"),
    ];
    for (kind, word) in examined("kind(s) of state", kinds) {
        assert_eq!(kind.word(), word, "{kind:?}");
    }
    // a synced answer is not offered and the page says so; any other refusal offers nothing
    let reasons = vec![
        (UndoRefusal::Synced, "synced"),
        (UndoRefusal::Gone, "none"),
        (UndoRefusal::NotTheCard, "none"),
        (UndoRefusal::Changed, "none"),
    ];
    for (refusal, why) in examined("reason(s) for no offer", reasons) {
        assert_eq!(refusal.why(), why, "{refusal:?}");
    }
}

#[test]
fn the_card_view_offers_an_undo_only_for_the_reviews_own_unsynced_answer() {
    // SPEC-371 R7: `answer` when the core admits the kept answer, `synced` when it has synced, and
    // nothing when no answer is kept or any other refusal holds.
    let views = vec![
        (None, None),
        (Some(Ok(())), Some("answer")),
        (Some(Err(UndoRefusal::Synced)), Some("synced")),
        (Some(Err(UndoRefusal::Gone)), None),
        (Some(Err(UndoRefusal::NotTheCard)), None),
        (Some(Err(UndoRefusal::Changed)), None),
    ];
    for (judged, view) in examined("judged record(s)", views) {
        assert_eq!(undo_view(judged), view, "{judged:?}");
    }
}

#[test]
fn a_mark_view_names_its_kind() {
    // SPEC-383 R9: the card view names the kept change's kind when the core admits its undo,
    // `change-synced` when it has synced, and nothing for every other refusal or no change kept.
    let bury = Change::Bury(Returns::Review);
    let flag = Change::Flag {
        before: 0,
        after: 1,
    };
    let named = vec![
        (Some((bury, Ok(()))), Some("bury")),
        (Some((flag, Ok(()))), Some("flag")),
        (
            Some((bury, Err(UndoRefusal::Synced))),
            Some("change-synced"),
        ),
        (
            Some((flag, Err(UndoRefusal::Synced))),
            Some("change-synced"),
        ),
        (Some((bury, Err(UndoRefusal::Gone))), None),
        (Some((flag, Err(UndoRefusal::NotTheCard))), None),
        (Some((bury, Err(UndoRefusal::Changed))), None),
        (None, None),
    ];
    for (judged, view) in examined("judged change(s)", named) {
        assert_eq!(mark_view(judged), view, "{judged:?}");
    }
}

#[test]
fn a_mark_record_matches_only_its_card_and_step() {
    // SPEC-383 R9: the page confirms only the change it was offered, so a confirmation that names
    // another card, another step, or a change when none is kept is refused as changed.
    let kept = LastMark {
        card: 42,
        step: 7,
        change: Change::Bury(Returns::New),
        record: "the core's record",
    };
    assert_eq!(last_mark_for(Some(&kept), 42, 7), Ok(&kept));
    let stale = vec![(41, 7), (42, 8), (41, 8), (42, 6)];
    for (card, step) in examined("stale confirmation(s)", stale) {
        assert_eq!(
            last_mark_for(Some(&kept), card, step),
            Err(StudyError::NotUndoable(UndoRefusal::Changed)),
            "card {card} at step {step}"
        );
    }
    assert_eq!(
        last_mark_for::<&str>(None, 42, 7),
        Err(StudyError::NotUndoable(UndoRefusal::Changed))
    );
}

#[test]
fn a_flag_offer_says_what_the_undo_puts_back() {
    // SPEC-383 R9, R11: red on a card with no flag was added, red taken off was removed, and red
    // over any other flag replaced it. The engine numbers red 1 and no flag 0.
    let changes = vec![
        ((0, 1), "added"),
        ((1, 0), "removed"),
        ((2, 1), "replaced"),
        ((4, 1), "replaced"),
        ((7, 1), "replaced"),
    ];
    for ((before, after), said) in examined("flag change(s)", changes) {
        assert_eq!(
            flag_change(before, after),
            said,
            "from flag {before} to flag {after}"
        );
    }
}
