//! SPEC-365 A3 to A6 (R1 to R3; ADR-376 D2, D3): an owner's answer records its one card with its
//! one grade, and a request that names another card, another rating or no `CardAnswer` at all is
//! refused before the engine sees it.
//!
//! Each test opens a synthetic collection the engine builds, on a native dispatcher, takes the
//! states the queue gives its new cards, and records an answer through `run_answer` as an adapter
//! does. The expected next state is the engine's own state for the grade, named here grade by
//! grade, never the grade's own pick, and what the engine wrote is read from the collection's rows.
//!
//! SPEC-378 A1 to A3 (#716; ADR-389): a grade recorded through the answer is an ordinary answer,
//! and no exemption holds it. These three open no collection. They name the call that records a
//! grade by its literal pair and name, never read from the core, and each holds one side of it:
//! the table answers it for a press on both transports, no exempt row names it, and `run` holds it
//! for a press and never for a gesture.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;
use anki_proto::scheduler::card_answer::Rating;
use anki_proto::scheduler::scheduling_state::{Kind, normal};
use anki_proto::scheduler::{
    CardAnswer, GetQueuedCardsRequest, QueuedCards, SchedulingState, SchedulingStates,
};
use deck_streak_engine_core::answer::{
    AnswerRefusal, Grade, OwnerAnswer, answer_request, shown_states,
};
use deck_streak_engine_core::dispatch::{Dispatcher, Read, Refusal};
use deck_streak_engine_core::table::{
    ANSWERED, Decision, EXEMPT, Exempt, ExemptWrite, TargetKind, Transport, decide,
};
use prost::Message;
use serde_json::Value;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// The snapshot's column for the card's repetitions: `[id, queue, type, due, ivl, reps, lapses]`.
const REPS: usize = 5;
/// The engine's card type for a card in learning (`cards.type`).
const LEARNING: i64 = 1;
/// Bytes that are no `CardAnswer` and no `SchedulingStates`: a length-delimited field 1 whose
/// length runs past the end.
const NO_MESSAGE: [u8; 3] = [0x0a, 0x05, 0x01];
/// Each press against each rating it does not name: Again against Hard, Good and Easy, and Good
/// against Again, Hard and Easy, as the engine numbers them (1, 2, 3 and 0, 1, 3).
const OTHER_RATINGS: [(Grade, Rating); 6] = [
    (Grade::Again, Rating::Hard),
    (Grade::Again, Rating::Good),
    (Grade::Again, Rating::Easy),
    (Grade::Good, Rating::Again),
    (Grade::Good, Rating::Hard),
    (Grade::Good, Rating::Easy),
];

/// A native dispatcher with `synthetic` open.
fn opened(synthetic: &support::Synthetic) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &support::open_request(synthetic))
        .expect("the native dispatcher opens the collection");
    dispatcher
}

/// The synthetic collection's two new cards, each with the states the queue showed it with, in
/// the queue's order.
fn shown(dispatcher: &Dispatcher) -> Vec<(i64, SchedulingStates)> {
    let request = GetQueuedCardsRequest {
        fetch_limit: 2,
        intraday_learning_only: false,
    };
    let (service, method) = GET_QUEUED_CARDS;
    let queued = dispatcher
        .run(service, method, &request.encode_to_vec())
        .expect("the queue is admitted");
    QueuedCards::decode(queued.as_slice())
        .expect("the queue decodes")
        .cards
        .into_iter()
        .map(|queued| {
            (
                queued.card.expect("a queued card carries its card").id,
                queued.states.expect("a queued card carries its states"),
            )
        })
        .collect()
}

/// One card's repetitions, by the core's fixed snapshot read.
fn reps(dispatcher: &Dispatcher, card: i64) -> Option<i64> {
    let reply = dispatcher
        .read(Read::CardSnapshot(card))
        .expect("the snapshot reads");
    serde_json::from_slice::<Value>(&reply)
        .expect("the engine replies with JSON rows")
        .pointer(&format!("/0/{REPS}"))
        .and_then(Value::as_i64)
}

fn now_millis() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    i64::try_from(millis).expect("the clock's milliseconds fit an i64")
}

/// An owner's press of `grade` on `card`, recorded with `request` through the one door.
fn press(
    dispatcher: &Dispatcher,
    card: i64,
    grade: Grade,
    request: &[u8],
) -> Result<(), AnswerRefusal> {
    dispatcher
        .run_answer(OwnerAnswer::from_press(card, grade), request)
        .map(|_| ())
}

/// The request an adapter writes for a press of `grade` on `card`, shown with `states`: the
/// grade's pick of the next state and the grade's own rating.
fn request(card: i64, states: &SchedulingStates, grade: Grade) -> Vec<u8> {
    answer_request(
        card,
        states.current.clone(),
        grade.pick(states.again.clone(), states.good.clone()),
        grade,
        now_millis(),
        1000,
    )
}

/// A `CardAnswer` for `card`, shown with `states`, naming `rating` and the engine's own next state
/// for that rating, so the engine itself would record it.
fn rated(card: i64, states: &SchedulingStates, rating: Rating) -> Vec<u8> {
    let next = match rating {
        Rating::Again => states.again.clone(),
        Rating::Hard => states.hard.clone(),
        Rating::Good => states.good.clone(),
        Rating::Easy => states.easy.clone(),
    };
    CardAnswer {
        card_id: card,
        current_state: states.current.clone(),
        new_state: next,
        rating: rating as i32,
        answered_at_millis: now_millis(),
        milliseconds_taken: 1000,
    }
    .encode_to_vec()
}

/// The steps a learning state leaves, as the engine gave it; `None` for any other state.
fn remaining_steps(state: Option<&SchedulingState>) -> Option<i64> {
    let Some(Kind::Normal(normal)) = state.and_then(|state| state.kind.as_ref()) else {
        return None;
    };
    match normal.kind.as_ref() {
        Some(normal::Kind::Learning(learning)) => Some(i64::from(learning.remaining_steps)),
        _ => None,
    }
}

/// Each card's type, steps left and repetitions, read with the engine's own API from the
/// collection file once the dispatcher has let it go.
fn rows(synthetic: &support::Synthetic, dispatcher: Dispatcher, cards: &[i64]) -> Vec<[i64; 3]> {
    drop(dispatcher);
    let col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine reopens the collection");
    let rows = cards
        .iter()
        .map(|card| {
            col.storage
                .db()
                .query_row(
                    "select type, left, reps from cards where id = ?",
                    [card],
                    |row| Ok([row.get(0)?, row.get(1)?, row.get(2)?]),
                )
                .expect("each card has its row")
        })
        .collect();
    col.close(None).expect("the engine closes the collection");
    rows
}

#[test]
fn a_press_answers_its_one_card_with_its_grade() {
    let synthetic = support::synthetic("answer-its-grade");
    let dispatcher = opened(&synthetic);
    let [(first, first_states), (second, second_states)]: [(i64, SchedulingStates); 2] =
        shown(&dispatcher)
            .try_into()
            .expect("the queue shows the two new cards");
    let before = [reps(&dispatcher, first), reps(&dispatcher, second)];
    let pressed = [
        press(
            &dispatcher,
            first,
            Grade::Again,
            &request(first, &first_states, Grade::Again),
        ),
        press(
            &dispatcher,
            second,
            Grade::Good,
            &request(second, &second_states, Grade::Good),
        ),
    ];
    let after = rows(&synthetic, dispatcher, &[first, second]);
    // The engine's own next state for each grade, read from the states it showed each card with.
    let again = remaining_steps(first_states.again.as_ref())
        .expect("the engine's Again state for a new card is a learning state");
    let good = remaining_steps(second_states.good.as_ref())
        .expect("the engine's Good state for a new card is a learning state");
    assert_eq!(
        (before, pressed, after),
        (
            [Some(0), Some(0)],
            [Ok(()), Ok(())],
            vec![[LEARNING, again, 1], [LEARNING, good, 1]]
        ),
        "Again and Good each answer their own card once, into the engine's own next state for the grade"
    );
    assert_ne!(
        again, good,
        "the engine's Again and Good states leave different steps, so the rows tell them apart"
    );
}

#[test]
fn a_press_for_one_card_cannot_answer_another() {
    let synthetic = support::synthetic("answer-another-card");
    let dispatcher = opened(&synthetic);
    let [(first, first_states), (second, _)]: [(i64, SchedulingStates); 2] = shown(&dispatcher)
        .try_into()
        .expect("the queue shows the two new cards");
    // The press names the second card and the request the first, the head of the queue, which the
    // engine itself would answer: only the press's own check stands between them.
    let pressed = press(
        &dispatcher,
        second,
        Grade::Good,
        &request(first, &first_states, Grade::Good),
    );
    assert_eq!(
        (pressed, reps(&dispatcher, first), reps(&dispatcher, second)),
        (
            Err(AnswerRefusal::NotTheCard {
                pressed: second,
                named: first
            }),
            Some(0),
            Some(0)
        ),
        "a press on one card refuses a request that names the other, and neither card is answered"
    );
}

#[test]
fn a_press_records_only_its_own_grade() {
    let mut judged = Vec::new();
    for (index, (grade, rating)) in OTHER_RATINGS.into_iter().enumerate() {
        let synthetic = support::synthetic(&format!("answer-other-rating-{index}"));
        let dispatcher = opened(&synthetic);
        let (card, states) = shown(&dispatcher)
            .into_iter()
            .next()
            .expect("the queue shows a new card");
        let pressed = press(&dispatcher, card, grade, &rated(card, &states, rating));
        judged.push((grade, rating as i32, pressed, reps(&dispatcher, card)));
    }
    let refused: Vec<_> = OTHER_RATINGS
        .into_iter()
        .map(|(grade, rating)| {
            (
                grade,
                rating as i32,
                Err(AnswerRefusal::NotTheGrade {
                    pressed: grade,
                    named: rating as i32,
                }),
                Some(0),
            )
        })
        .collect();
    assert_eq!(
        judged, refused,
        "each press refuses every rating but its own grade's, and its card is not answered"
    );
    println!(
        "examined {} of {} press and rating pair(s)",
        judged.len(),
        OTHER_RATINGS.len()
    );
    assert_eq!(
        judged.len(),
        OTHER_RATINGS.len(),
        "every press and rating pair reached its assertion"
    );
}

#[test]
fn a_request_that_is_not_a_card_answer_is_refused() {
    let synthetic = support::synthetic("answer-undecodable");
    let dispatcher = opened(&synthetic);
    let (card, _) = shown(&dispatcher)
        .into_iter()
        .next()
        .expect("the queue shows a new card");
    let pressed = press(&dispatcher, card, Grade::Good, &NO_MESSAGE);
    assert_eq!(
        (pressed, reps(&dispatcher, card)),
        (Err(AnswerRefusal::Undecodable), Some(0)),
        "bytes that are no CardAnswer are refused before the engine sees them, and the card is not answered"
    );
    assert_eq!(
        shown_states(&NO_MESSAGE).map(|_| ()),
        Err(AnswerRefusal::Undecodable),
        "the same bytes are no SchedulingStates either, and a native press's states refuse them"
    );
}

/// MUTATION COVERAGE, not red first (SPEC-365 R3): added after the code to kill the mutant that
/// answers every refusal with empty text. The web's `rate` hands this text to the page as its
/// refusal, so each variant's sentence is held whole, with the values spelt here.
#[test]
fn each_answer_refusal_reads_as_its_own_sentence() {
    let refusals = [
        AnswerRefusal::Undecodable,
        AnswerRefusal::NotTheCard {
            pressed: 7,
            named: 8,
        },
        AnswerRefusal::NotTheGrade {
            pressed: Grade::Good,
            named: 1,
        },
        AnswerRefusal::Engine {
            error: vec![1, 2, 3],
        },
    ];
    let sentences: Vec<String> = refusals.iter().map(ToString::to_string).collect();
    assert_eq!(
        sentences,
        [
            "the answer is not the engine's card answer",
            "the press named card 7; the answer names card 8",
            "the press named Good; the answer names rating 1",
            "the engine refused the answer (3 bytes)",
        ],
        "each refusal reads as its own sentence, with the values it carries"
    );
    println!("examined {} of 4 refusal(s)", sentences.len());
}

/// The call that records a grade, as the engine numbers it (SPEC-365 R4): written here, never read
/// from the core, so a table that moves the call is judged against this literal (SPEC-378 R1).
const ANSWER_CARD: (u32, u32) = (13, 4);
/// The engine's name for [`ANSWER_CARD`].
const ANSWER_CARD_NAME: &str = "SchedulerService.AnswerCard";
/// Both transports: the native adapter's and the web engine's.
const TRANSPORTS: [Transport; 2] = [Transport::Native, Transport::Web];

/// The names of the rows of `exempt` that hold the call recording a grade, by its pair or by its
/// name (SPEC-378 R2).
fn answers_among(exempt: &[Exempt]) -> Vec<&'static str> {
    let (service, method) = ANSWER_CARD;
    exempt
        .iter()
        .filter(|row| row.is(service, method) || row.name == ANSWER_CARD_NAME)
        .map(|row| row.name)
        .collect()
}

/// `rows`, with its first row replaced by `with`.
fn first_replaced(rows: &[Exempt], with: Exempt) -> Vec<Exempt> {
    let mut planted = rows.to_vec();
    if let Some(first) = planted.first_mut() {
        *first = with;
    }
    planted
}

/// MUTATION COVERAGE, not red first (SPEC-378 A1, R1): it pins the classing the base already has.
/// The call that records a grade is the one answered row, and the table holds it for an owner's
/// press on both transports, where an exempt write would read `NeedsGesture`.
#[test]
fn the_answer_is_decided_for_a_press_on_both_transports_never_for_a_gesture() {
    let (service, method) = ANSWER_CARD;
    let decided: Vec<(Transport, Decision)> = TRANSPORTS
        .into_iter()
        .map(|transport| (transport, decide(transport, service, method)))
        .collect();
    let answered: Vec<(u32, u32, &str)> = ANSWERED
        .iter()
        .map(|row| (row.service, row.method, row.name))
        .collect();
    assert_eq!(
        (decided, answered),
        (
            vec![
                (Transport::Native, Decision::NeedsAnswer),
                (Transport::Web, Decision::NeedsAnswer),
            ],
            vec![(13, 4, ANSWER_CARD_NAME)],
        ),
        "the call that records a grade is the one answered row, held for an owner's press on both transports"
    );
    println!("examined {} of 2 transport(s)", TRANSPORTS.len());
}

/// MUTATION COVERAGE, not red first (SPEC-378 A2, R2): it pins the table the base already has. No
/// exempt row names the call that records a grade, whatever else the exempt table holds, and a row
/// planted to name it is refused by the row's own name, whether it joins the table, takes the
/// answer's pair or takes the answer's name.
#[test]
fn no_exempt_row_holds_the_answer_and_a_planted_one_is_refused_by_name() {
    let rows = EXEMPT.to_vec();
    let first = *rows
        .first()
        .expect("the exempt table holds a row to plant over");
    let (service, method) = ANSWER_CARD;
    let joined: Vec<Exempt> = rows
        .iter()
        .copied()
        .chain([Exempt {
            write: ExemptWrite::Undo,
            service,
            method,
            name: ANSWER_CARD_NAME,
            kind: TargetKind::Card,
        }])
        .collect();
    let renumbered = first_replaced(
        &rows,
        Exempt {
            service,
            method,
            ..first
        },
    );
    let renamed = first_replaced(
        &rows,
        Exempt {
            name: ANSWER_CARD_NAME,
            ..first
        },
    );
    assert_eq!(
        [
            rows.as_slice(),
            joined.as_slice(),
            renumbered.as_slice(),
            renamed.as_slice(),
        ]
        .map(answers_among),
        [
            vec![],
            vec![ANSWER_CARD_NAME],
            vec![first.name],
            vec![ANSWER_CARD_NAME],
        ],
        "no exempt row names the call that records a grade; a planted row that joins, takes its pair or takes its name is refused by name"
    );
    support::examined("exempt row(s)", rows);
}

/// MUTATION COVERAGE, not red first (SPEC-378 A3, R3): it pins the refusal the base already has.
/// `run` refuses the call that records a grade as held for an owner's press on both transports,
/// before the engine sees it, and never as held for a gesture.
#[test]
fn run_holds_an_answer_for_a_press_on_both_transports_never_for_a_gesture() {
    let (service, method) = ANSWER_CARD;
    let refused: Vec<(Transport, Result<Vec<u8>, Refusal>)> = TRANSPORTS
        .into_iter()
        .map(|transport| {
            let dispatcher =
                Dispatcher::start(transport, &[]).expect("the engine starts from the default init");
            (transport, dispatcher.run(service, method, &NO_MESSAGE))
        })
        .collect();
    assert_eq!(
        refused,
        vec![
            (
                Transport::Native,
                Err(Refusal::NeedsAnswer {
                    service: 13,
                    method: 4
                })
            ),
            (
                Transport::Web,
                Err(Refusal::NeedsAnswer {
                    service: 13,
                    method: 4
                })
            ),
        ],
        "run holds the call that records a grade for an owner's press on both transports, never for a gesture"
    );
    println!("examined {} of 2 transport(s)", TRANSPORTS.len());
}
