//! The scheduling port and its FSRS-7 implementation (SPEC-494 A1 to A4 and A7 to A9).
//!
//! A1 and A2 compare the port with today's direct path, which each builds itself from the review
//! rows with the FSRS-7 crate's own functions (its history selection, its batch replay and its
//! stock projection), the rounding and the clamp, and a copy of the engine-day rule; never with
//! the core. The replay fixtures of A1 are built as the replay suite builds them, with the
//! engine's own API on a test-only engine handle and the clock's now (`tests/replay.rs:96-166`,
//! `:400-482`). Every other expected value is typed as a literal.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anki::card::CardId;
use anki::collection::{Collection, CollectionBuilder};
use anki::decks::DeckId;
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::timestamp::TimestampMillis;
use anki_proto::backend::BackendError;
use anki_proto::collection::{CloseCollectionRequest, OpenCollectionRequest};
use deck_streak_engine_core::dispatch::{Dispatcher, Refusal};
use deck_streak_engine_core::late::EngineDay;
use deck_streak_engine_core::replay::{CardReplay, Fsrs7, Schedule};
use deck_streak_engine_core::scheduler::{
    DayQueue, EventKey, FoldError, LoadError, QueueCard, Recall, ReviewEvent, SchedulerPort,
    day_queue,
};
use deck_streak_engine_core::table::Transport;
use deck_streak_fsrs7::convert::{self, RevlogRow};
use deck_streak_fsrs7::replay::{self as batch, Method};
use deck_streak_fsrs7::stock::{self, FSRS};
use prost::Message;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `BackendCollectionService.CloseCollection`.
const CLOSE_COLLECTION: (u32, u32) = (3, 1);

/// The default deck, where every built card starts.
const HOME: i64 = 1;
/// Milliseconds in a day.
const DAY_MS: i64 = 86_400_000;
/// Milliseconds in an hour.
const HOUR_MS: i64 = 3_600_000;
/// How far each built collection's creation is moved back, so its day count is not 0.
const DAYS_BACK: i64 = 10;
/// A review card, as the engine's `CardType` numbers it.
const REVIEW_TYPE: i64 = 2;
/// The desired retention every replay here is asked for.
const RETENTION: f32 = 0.9;
/// A maximum interval no replay here reaches.
const MAX_IVL: u32 = 36_500;

/// A frozen copy of the replay's one statement (`src/dispatch.rs:65-68` at the cut), run through
/// the test's own engine handle.
const STATEMENT: &str = "SELECT r.cid, r.id, r.ease, r.type, r.factor, c.type \
    FROM revlog AS r JOIN cards AS c ON c.id = r.cid \
    WHERE (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) \
    IN (SELECT value FROM json_each(?1)) ORDER BY r.cid, r.id";

/// The replay's refusal of a retention that is no recall.
const RETENTION_REFUSAL: &str = "the replay's retention is not a recall in its range";

/// The FSRS-7 implementation's fixed identifier, typed: the crate's name and the pin's 8-hex
/// prefix, then two zero bytes.
const IDENTIFIER: [u8; 32] = *b"deck-streak-fsrs7 rev 4bc0a097\0\0";

/// The pinned revision's FSRS-7 default parameters, typed by hand as the replay suite types them
/// (`tests/replay.rs:1012-1016`): a second parameter vector, of the one length a fitted vector
/// has.
const DEFAULT_PARAMETERS: [f32; 34] = [
    0.1104, 2.2395, 3.9221, 11.7841, 6.1686, 0.6457, 3.6807, 1.9795, 0.0, 1.3826, 0.7024, 0.5999,
    0.8146, 0.6398, 1.0, 1.3207, 0.6707, 3.8668, 0.4416, 0.0934, 1.8631, 0.6162, 1.0869, 0.1567,
    0.0801, 0.2421, 0.9464, 0.1433, 0.7145, 0.0, 0.5667, 0.3734, 0.5333, 0.3048,
];

/// The first key of the hand-built events of A7 to A9.
const KEY: i64 = 1_760_000_000_000;

/// A collection built for one test, closed.
struct Built {
    /// The test's own directory.
    dir: PathBuf,
    /// The collection file.
    collection: PathBuf,
    /// The cards, one per Basic note, in the order they were added.
    cards: Vec<i64>,
}

/// The test's own engine handle on a closed collection file.
fn engine(collection: &Path) -> Collection {
    CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the test's collection")
}

/// `count` Basic notes in the default deck, so `count` cards, with the engine's own API, and the
/// collection's creation moved back `DAYS_BACK` days, as `tests/replay.rs:96-132` builds them.
fn built(test: &str, count: usize) -> Built {
    let dir = support::scratch("engine-core-scheduler-seam", test);
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    let mut col = engine(&collection);
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let cards = (0..count)
        .map(|index| {
            let mut note = basic.new_note();
            note.set_field(0, format!("seam front {index}"))
                .expect("the front is set");
            note.set_field(1, format!("seam back {index}"))
                .expect("the back is set");
            col.add_note(&mut note, DeckId(HOME))
                .expect("the engine adds the note to the default deck");
            col.storage
                .db()
                .query_row("select id from cards where nid = ?", [note.id.0], |row| {
                    row.get(0)
                })
                .expect("the note has one card")
        })
        .collect();
    col.storage
        .db()
        .execute("update col set crt = crt - ?", [DAYS_BACK * 86_400])
        .expect("the collection's creation moves back");
    col.close(None).expect("the engine closes the collection");
    Built {
        dir,
        collection,
        cards,
    }
}

fn now_millis() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    i64::try_from(millis).expect("the clock's milliseconds fit an i64")
}

/// Answers `card` with `rating` at `at`, as the engine's own reviewer does.
fn answer(col: &mut Collection, card: i64, rating: Rating, at: i64) {
    let states = col
        .get_scheduling_states(CardId(card))
        .expect("the engine reads the card's next states");
    let new_state = match rating {
        Rating::Again => states.again,
        Rating::Hard => states.hard,
        Rating::Good => states.good,
        Rating::Easy => states.easy,
    };
    col.answer_card(&mut CardAnswer {
        card_id: CardId(card),
        current_state: states.current,
        new_state,
        rating,
        answered_at: TimestampMillis(at),
        milliseconds_taken: 1000,
        custom_data: None,
        from_queue: false,
    })
    .expect("the engine answers the card");
}

/// The id of `card`'s newest review-log row.
fn newest_row(col: &Collection, card: i64) -> i64 {
    col.storage
        .db()
        .query_row("select max(id) from revlog where cid = ?", [card], |row| {
            row.get(0)
        })
        .expect("the card has a review-log row")
}

/// `card`'s type, due and interval, read by a fresh engine on the file.
fn engine_schedule(collection: &Path, card: i64) -> (i64, i32, i32) {
    let col = engine(collection);
    let read = col
        .storage
        .db()
        .query_row(
            "select type, due, ivl from cards where id = ?",
            [card],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("the card reads");
    col.close(None).expect("the engine closes the collection");
    read
}

/// The rows of the frozen statement over the default deck, read by a fresh engine on the file.
fn statement_rows(collection: &Path) -> Vec<[i64; 6]> {
    let col = engine(collection);
    let rows = {
        let db = col.storage.db();
        let mut statement = db
            .prepare(STATEMENT)
            .expect("the frozen statement prepares");
        statement
            .query_map([format!("[{HOME}]")], |row| {
                Ok([
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ])
            })
            .expect("the frozen statement runs")
            .map(|row| row.expect("a row reads"))
            .collect()
    };
    col.close(None).expect("the engine closes the collection");
    rows
}

fn text(path: &Path) -> String {
    path.to_str().expect("a scratch path is UTF-8").to_owned()
}

/// A web dispatcher with `built`'s collection open.
fn opened(built: &Built) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Web, &[]).expect("the engine starts from the default init");
    let request = OpenCollectionRequest {
        collection_path: text(&built.collection),
        media_folder_path: text(&built.dir.join("collection.media")),
        media_db_path: text(&built.dir.join("collection.media.db")),
    }
    .encode_to_vec();
    dispatcher
        .run(OPEN_COLLECTION.0, OPEN_COLLECTION.1, &request)
        .expect("the web dispatcher opens the collection");
    dispatcher
}

/// Closes the dispatcher's collection.
fn close(dispatcher: &Dispatcher) {
    dispatcher
        .run(
            CLOSE_COLLECTION.0,
            CLOSE_COLLECTION.1,
            &CloseCollectionRequest::default().encode_to_vec(),
        )
        .expect("the web dispatcher closes the collection");
}

/// A copy of the engine-day rule (`src/replay.rs:186-189` at the cut): today's day count, less
/// one for each whole day the instant `at`, in milliseconds, lies before today began.
fn engine_day_of(day: EngineDay, at: i64) -> i64 {
    let began = day.next_day_at - 86_400;
    i64::from(day.days_elapsed) + (at.div_euclid(1_000) - began).div_euclid(86_400)
}

/// An interval rounded to the nearest whole day, as today's direct path rounds it.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "today's direct path casts the same way: a float-to-integer cast saturates"
)]
fn rounded(interval: f32) -> u32 {
    interval.round() as u32
}

/// The statement's rows as review-log rows.
fn revlog(rows: &[[i64; 6]]) -> Vec<RevlogRow> {
    rows.iter()
        .map(|&[cid, id, ease, kind, factor, _]| RevlogRow {
            cid,
            id,
            ease: u8::try_from(ease).expect("an ease fits a u8"),
            kind: u8::try_from(kind).expect("a kind fits a u8"),
            factor: u32::try_from(factor).expect("a factor fits a u32"),
        })
        .collect()
}

/// One card's values on today's direct path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Expected {
    /// The interval at the retention, rounded.
    whole: u32,
    /// The stock stability and difficulty, as bits.
    stock: (u32, u32),
    /// A review card's schedule: the rounded interval held to the maximum and to 1, due the last
    /// review's engine day plus it.
    schedule: Option<Schedule>,
}

/// Today's direct path over the statement's `rows`, built from the FSRS-7 crate's own functions:
/// its history selection, its batch replay and its stock projection at 0.9, then the round, the
/// clamp and the engine-day rule's copy.
fn control(rows: &[[i64; 6]], day: EngineDay, max_ivl: u32) -> BTreeMap<i64, Expected> {
    let types: BTreeMap<i64, i64> = rows.iter().map(|row| (row[0], row[5])).collect();
    let revlog = revlog(rows);
    let model = FSRS::new(&[]).expect("no parameters select the pinned defaults");
    let histories = convert::histories(&revlog);
    let items = histories
        .iter()
        .map(|history| history.item.clone())
        .collect();
    let states = batch::replay(Method::Batch, &model, items).expect("the batch replays");
    histories
        .iter()
        .zip(states)
        .map(|(history, state)| {
            let projected = stock::project(&model, state, RETENTION);
            let whole = rounded(projected.interval);
            let schedule = (types.get(&history.cid) == Some(&REVIEW_TYPE)).then(|| {
                let ivl = whole.min(max_ivl).max(1);
                let due = engine_day_of(day, history.last_id) + i64::from(ivl);
                Schedule {
                    ivl,
                    due: i32::try_from(due).expect("a due fits an i32"),
                }
            });
            let expected = Expected {
                whole,
                stock: (
                    projected.stability.to_bits(),
                    projected.difficulty.to_bits(),
                ),
                schedule,
            };
            (history.cid, expected)
        })
        .collect()
}

/// The port's events for the statement's `rows`, built here: per card, its rated rows of a kind
/// the crate keeps, by id, the last as many as its history holds, each keyed by its id and dated
/// by its engine day, in key order.
fn port_events(rows: &[[i64; 6]], day: EngineDay) -> Vec<ReviewEvent> {
    let revlog = revlog(rows);
    let mut events: Vec<ReviewEvent> = convert::histories(&revlog)
        .iter()
        .flat_map(|history| {
            let mut kept: Vec<&RevlogRow> = revlog
                .iter()
                .filter(|row| {
                    row.cid == history.cid
                        && row.ease != 0
                        && !convert::DROPPED_KINDS.contains(&row.kind)
                })
                .collect();
            kept.sort_by_key(|row| row.id);
            let start = kept.len().saturating_sub(history.item.reviews.len());
            kept[start..]
                .iter()
                .map(|row| ReviewEvent {
                    card: row.cid,
                    note: 0,
                    deck: 0,
                    preset: 0,
                    grade: row.ease,
                    answer_ms: 0,
                    key: EventKey(row.id),
                    study_day: i32::try_from(engine_day_of(day, row.id))
                        .expect("an engine day fits an i32"),
                })
                .collect::<Vec<_>>()
        })
        .collect();
    events.sort_by_key(|event| event.key);
    events
}

/// A port of the pinned defaults with every event of `events` folded, in order.
fn folded(events: &[ReviewEvent]) -> (Fsrs7, <Fsrs7 as SchedulerPort>::State) {
    let port = Fsrs7::new(&[]).expect("the pinned defaults make a port");
    let mut state = port.new_state();
    for event in events {
        port.fold(&mut state, event)
            .expect("the port folds each kept review");
    }
    (port, state)
}

/// Checks the port and the replay against the control over `rows` on `day`, and returns the
/// replay's schedules by card.
fn checked(
    dispatcher: &Dispatcher,
    rows: &[[i64; 6]],
    day: EngineDay,
    max_ivl: u32,
) -> BTreeMap<i64, Option<Schedule>> {
    let expected = control(rows, day, max_ivl);
    let (port, state) = folded(&port_events(rows, day));
    let target = Recall::new(RETENTION).expect("0.9 is a recall");
    let through_port: BTreeMap<i64, (u32, (u32, u32))> = expected
        .keys()
        .map(|&card| {
            let whole = port
                .interval_days(&state, card, target)
                .expect("the port gives a folded card's whole days");
            let (stability, difficulty) = port
                .stock_values(&state, card, target)
                .expect("the port gives a folded card's stock values");
            (card, (whole, (stability.to_bits(), difficulty.to_bits())))
        })
        .collect();
    assert_eq!(
        through_port,
        expected
            .iter()
            .map(|(&card, value)| (card, (value.whole, value.stock)))
            .collect::<BTreeMap<_, _>>(),
        "the port's whole days and stock bits equal today's direct path for every card"
    );
    let replayed: BTreeMap<i64, CardReplay> = dispatcher
        .replay(&[HOME], &[], RETENTION, max_ivl, day)
        .expect("the replay runs")
        .into_iter()
        .map(|entry| (entry.card, entry))
        .collect();
    assert_eq!(
        replayed
            .values()
            .map(|entry| (
                entry.card,
                (
                    (entry.stability.to_bits(), entry.difficulty.to_bits()),
                    entry.schedule
                )
            ))
            .collect::<BTreeMap<_, _>>(),
        expected
            .iter()
            .map(|(&card, value)| (card, (value.stock, value.schedule)))
            .collect::<BTreeMap<_, _>>(),
        "the replay's stock bits and schedules equal today's direct path for every card"
    );
    replayed
        .into_iter()
        .map(|(card, entry)| (card, entry.schedule))
        .collect()
}

/// The message of a refused replay, or `None` for a replay that ran.
fn refusal_text(reply: &Result<Vec<CardReplay>, Refusal>) -> Option<String> {
    match reply {
        Err(Refusal::Engine { error }) => BackendError::decode(error.as_slice())
            .ok()
            .map(|error| error.message),
        _ => None,
    }
}

#[test]
fn fsrs7_behind_the_port_gives_todays_due_days_on_the_replay_fixtures() {
    // The replay suite's schedule fixture (tests/replay.rs:400-457): a first Easy and a first
    // Again whose card the engine makes a review card due today, each at the clock's now.
    let review = built("seam-review-schedule", 2);
    let (easy, again) = (review.cards[0], review.cards[1]);
    let mut col = engine(&review.collection);
    answer(&mut col, easy, Rating::Easy, now_millis());
    answer(&mut col, again, Rating::Again, now_millis());
    col.set_due_date(&[CardId(again)], "0", None)
        .expect("the engine makes the card a review card due today");
    col.close(None).expect("the engine closes the collection");
    let (_, due, ivl) = engine_schedule(&review.collection, easy);
    let day = due - ivl;
    let rows = statement_rows(&review.collection);
    let dispatcher = opened(&review);
    let today = dispatcher
        .engine_day()
        .expect("the engine reads its day before the replay");
    let full = checked(&dispatcher, &rows, today, MAX_IVL);
    let held = checked(&dispatcher, &rows, today, 3);
    let refused = [1.0, f32::NAN]
        .map(|retention| refusal_text(&dispatcher.replay(&[HOME], &[], retention, MAX_IVL, today)));
    close(&dispatcher);
    assert_eq!(
        (full[&easy], full[&again], held[&easy]),
        (
            Some(Schedule {
                ivl: 54,
                due: day + 54
            }),
            Some(Schedule {
                ivl: 1,
                due: day + 1
            }),
            Some(Schedule {
                ivl: 3,
                due: day + 3
            })
        ),
        "the pins hold through the port: 54 days, 1 held, and the maximum of 3"
    );
    assert_eq!(
        refused,
        [
            Some(RETENTION_REFUSAL.to_owned()),
            Some(RETENTION_REFUSAL.to_owned())
        ],
        "a retention of 1.0 and of NaN is refused with the replay's refusal text"
    );

    // The replay suite's fixture of a review before today began (tests/replay.rs:459-482).
    let before_today = built("seam-review-before-today", 1);
    let card = before_today.cards[0];
    let mut col = engine(&before_today.collection);
    answer(&mut col, card, Rating::Easy, now_millis());
    let reviewed_at = newest_row(&col, card) / 1000;
    col.close(None).expect("the engine closes the collection");
    let day = EngineDay {
        days_elapsed: 1000,
        next_day_at: reviewed_at + 3 * 86_400 + 1234,
    };
    let rows = statement_rows(&before_today.collection);
    let dispatcher = opened(&before_today);
    let before = checked(&dispatcher, &rows, day, MAX_IVL);
    close(&dispatcher);
    assert_eq!(
        before[&card],
        Some(Schedule { ivl: 54, due: 1051 }),
        "the pin of day 1051 holds through the port"
    );
}

/// The grid of A2: each card two reviews, rated 1 to 4 then 1 to 4, the second 1 to 30 days and
/// 0 or 6 hours after the first; the first ids distinct from a fixed base.
fn grid() -> Vec<[RevlogRow; 2]> {
    let base = 1_700_000_000_000_i64;
    let mut cells = Vec::new();
    for first in 1..=4_u8 {
        for second in 1..=4_u8 {
            for gap in 1..=30_i64 {
                for offset in [0, 6] {
                    let card = i64::try_from(cells.len()).expect("a small count") + 1;
                    let first_id = base + card;
                    let second_id = first_id + gap * DAY_MS + offset * HOUR_MS;
                    cells.push([
                        RevlogRow {
                            cid: card,
                            id: first_id,
                            ease: first,
                            kind: 0,
                            factor: 0,
                        },
                        RevlogRow {
                            cid: card,
                            id: second_id,
                            ease: second,
                            kind: 1,
                            factor: 2500,
                        },
                    ]);
                }
            }
        }
    }
    cells
}

#[test]
fn fsrs7_whole_days_equal_todays_rounding_over_a_fixed_grid() {
    let cells = support::examined("grid cell(s)", grid());
    let rows: Vec<RevlogRow> = cells.iter().flatten().copied().collect();
    let model = FSRS::new(&[]).expect("no parameters select the pinned defaults");
    let histories = convert::histories(&rows);
    let items = histories
        .iter()
        .map(|history| history.item.clone())
        .collect();
    let states = batch::replay(Method::Batch, &model, items).expect("the grid replays");
    let intervals: BTreeMap<i64, f32> = histories
        .iter()
        .zip(states)
        .map(|(history, state)| {
            (
                history.cid,
                stock::project(&model, state, RETENTION).interval,
            )
        })
        .collect();
    let mut events: Vec<ReviewEvent> = rows
        .iter()
        .map(|row| ReviewEvent {
            card: row.cid,
            note: 0,
            deck: 0,
            preset: 0,
            grade: row.ease,
            answer_ms: 0,
            key: EventKey(row.id),
            study_day: i32::try_from(row.id.div_euclid(DAY_MS)).expect("a day fits an i32"),
        })
        .collect();
    events.sort_by_key(|event| event.key);
    let (port, state) = folded(&events);
    let target = Recall::new(RETENTION).expect("0.9 is a recall");
    let through_port: BTreeMap<i64, u32> = intervals
        .keys()
        .map(|&card| {
            let whole = port
                .interval_days(&state, card, target)
                .expect("the port gives a folded card's whole days");
            (card, whole)
        })
        .collect();
    assert_eq!(
        through_port,
        intervals
            .iter()
            .map(|(&card, &interval)| (card, rounded(interval)))
            .collect::<BTreeMap<_, _>>(),
        "the port's whole days equal today's rounding at every cell"
    );
    let apart = intervals
        .values()
        .filter(|interval| **interval > 1.0 && interval.fract() > 0.0 && interval.fract() < 0.5)
        .count();
    println!(
        "{apart} of {} grid cell(s) round and round up apart",
        intervals.len()
    );
    assert_eq!(intervals.len(), 960, "every cell of the grid is replayed");
    assert!(
        apart > 0,
        "the grid holds a cell whose interval rounds and rounds up apart"
    );
}

/// A card for the day queue.
fn queued(card: i64, note: i64, due: i32) -> QueueCard {
    QueueCard { card, note, due }
}

#[test]
fn two_cards_of_one_note_never_share_a_study_day() {
    let cards = [
        queued(1, 7, 10),
        queued(2, 7, 10),
        queued(3, 8, 10),
        queued(4, 7, 9),
        queued(5, 9, 11),
    ];
    let queue = day_queue(&cards, 10);
    assert_eq!(
        queue,
        DayQueue {
            day: 10,
            cards: vec![4, 3],
            moved: vec![1, 2],
            unlinked: 0
        },
        "the overdue sibling stays, the later two move, another note's card stays and a card due \
         tomorrow is not queued"
    );
    assert_eq!(queue.moved_to(), 11, "a moved card moves to the next day");
}

/// Every way the independent property check finds `queue` wrong for `cards` on `day`: the
/// candidates are the cards due on or before the day, a note of 0 or less is no note, and the
/// order is by due day then card.
fn violations(cards: &[QueueCard], day: i32, queue: &DayQueue) -> Vec<&'static str> {
    let mut found = Vec::new();
    let mut candidates: Vec<QueueCard> = cards
        .iter()
        .copied()
        .filter(|card| card.due <= day)
        .collect();
    candidates.sort_by_key(|card| (card.due, card.card));
    let of = |id: i64| -> QueueCard {
        *candidates
            .iter()
            .find(|card| card.card == id)
            .expect("a queued or moved card is a candidate")
    };
    let mut all: Vec<i64> = queue.cards.iter().chain(&queue.moved).copied().collect();
    all.sort_unstable();
    let mut wanted: Vec<i64> = candidates.iter().map(|card| card.card).collect();
    wanted.sort_unstable();
    if all != wanted {
        return vec!["the queued and moved cards are not a partition of the candidates"];
    }
    let linked: Vec<i64> = queue
        .cards
        .iter()
        .map(|&id| of(id).note)
        .filter(|&note| note > 0)
        .collect();
    if linked.iter().collect::<BTreeSet<_>>().len() != linked.len() {
        found.push("two queued cards share a note");
    }
    let position = |id: i64| candidates.iter().position(|card| card.card == id);
    if !queue.moved.iter().all(|&moved| {
        of(moved).note > 0
            && queue
                .cards
                .iter()
                .any(|&kept| of(kept).note == of(moved).note && position(kept) < position(moved))
    }) {
        found.push("a moved card's note is held by no earlier queued card");
    }
    let unlinked = candidates.iter().filter(|card| card.note <= 0).count();
    if usize::try_from(queue.unlinked).ok() != Some(unlinked) {
        found.push("the unlinked count is not the candidates with no note");
    }
    let ordered = |ids: &[i64]| {
        ids.windows(2)
            .all(|pair| position(pair[0]) < position(pair[1]))
    };
    if !ordered(&queue.cards) || !ordered(&queue.moved) {
        found.push("a list is not in the order of due day then card");
    }
    found
}

#[test]
fn the_sibling_rule_holds_over_every_small_queue() {
    let values = [0_i64, 1, 2];
    let mut queues = Vec::new();
    for notes in 0..81_usize {
        for dues in 0..81_usize {
            let cards: Vec<QueueCard> = (0..4_u32)
                .map(|place| {
                    let digit = |code: usize| (code / 3_usize.pow(place)) % 3;
                    queued(
                        i64::from(place) + 1,
                        values[digit(notes)],
                        i32::try_from(values[digit(dues)]).expect("a small due"),
                    )
                })
                .collect();
            queues.push(cards);
        }
    }
    let queues = support::examined("queue(s) of four cards", queues);
    let wrong: Vec<(Vec<QueueCard>, Vec<&str>)> = queues
        .iter()
        .map(|cards| (cards.clone(), violations(cards, 1, &day_queue(cards, 1))))
        .filter(|(_, found)| !found.is_empty())
        .collect();
    assert_eq!(queues.len(), 6561, "every queue of four cards is judged");
    assert_eq!(
        wrong.len(),
        0,
        "the property check finds no queue wrong; the first: {:?}",
        wrong.first()
    );
}

#[test]
fn a_card_with_no_note_id_turns_the_rule_off_and_is_counted() {
    let cards = [
        queued(1, 0, 5),
        queued(2, 0, 4),
        queued(3, -3, 5),
        queued(4, 6, 5),
    ];
    assert_eq!(
        day_queue(&cards, 5),
        DayQueue {
            day: 5,
            cards: vec![2, 1, 3, 4],
            moved: Vec::new(),
            unlinked: 3
        },
        "every card with no note id is queued and counted, and the linked card stays"
    );
}

/// A hand-built event of card `card` at `key` with `grade` and `answer_ms`.
fn event(card: i64, key: i64, grade: u8, answer_ms: u32) -> ReviewEvent {
    ReviewEvent {
        card,
        note: card * 10,
        deck: 1,
        preset: 1,
        grade,
        answer_ms,
        key: EventKey(key),
        study_day: i32::try_from(key.div_euclid(DAY_MS)).expect("a day fits an i32"),
    }
}

#[test]
fn a_fold_refuses_a_key_not_after_the_last_and_leaves_the_state_unchanged() {
    let port = Fsrs7::new(&[]).expect("the pinned defaults make a port");
    let mut state = port.new_state();
    let first = event(1, KEY, 3, 1_000);
    assert_eq!(port.fold(&mut state, &first), Ok(()), "a first event folds");
    let before = port.checkpoint(&state);
    let last = EventKey(KEY);
    let refused = [
        (
            event(1, KEY, 3, 1_000),
            FoldError::KeyNotAfter { last, key: last },
        ),
        (
            event(1, KEY - 1, 3, 1_000),
            FoldError::KeyNotAfter {
                last,
                key: EventKey(KEY - 1),
            },
        ),
        (
            event(1, KEY + 1, 0, 1_000),
            FoldError::GradeOutOfRange { grade: 0 },
        ),
        (
            event(1, KEY + 1, 5, 1_000),
            FoldError::GradeOutOfRange { grade: 5 },
        ),
        (
            event(1, KEY + 1, 3, 3_600_001),
            FoldError::AnswerTooLong {
                answer_ms: 3_600_001,
            },
        ),
    ];
    for (refused, error) in support::examined("refused event(s)", refused.to_vec()) {
        assert_eq!(
            port.fold(&mut state, &refused),
            Err(error),
            "{refused:?} is refused"
        );
        assert_eq!(
            port.checkpoint(&state),
            before,
            "the state is unchanged after the refusal of {refused:?}"
        );
    }
    let accepted = [
        event(1, KEY + DAY_MS, 3, 1_000),
        event(2, KEY + 2 * DAY_MS, 4, 1_000),
        event(1, KEY + 3 * DAY_MS, 3, 3_600_000),
    ];
    let mut length = before.len();
    for accepted in support::examined("accepted event(s)", accepted.to_vec()) {
        assert_eq!(
            port.fold(&mut state, &accepted),
            Ok(()),
            "{accepted:?} folds"
        );
        length += 49;
        assert_eq!(
            port.checkpoint(&state).len(),
            length,
            "an accepted event adds its 49 bytes"
        );
    }
}

/// Six events over three cards, in key order.
fn six_events() -> Vec<ReviewEvent> {
    vec![
        event(1, KEY, 3, 2_000),
        event(2, KEY + 1, 1, 9_000),
        event(3, KEY + 2, 4, 1_500),
        event(1, KEY + 3 * DAY_MS, 2, 4_000),
        event(2, KEY + 5 * DAY_MS, 3, 3_000),
        event(1, KEY + 9 * DAY_MS, 3, 2_500),
    ]
}

/// Each card's whole days at 0.9 in `state`.
fn whole_days(
    port: &Fsrs7,
    state: &<Fsrs7 as SchedulerPort>::State,
) -> Vec<Result<u32, FoldError>> {
    let target = Recall::new(RETENTION).expect("0.9 is a recall");
    (1..=3)
        .map(|card| port.interval_days(state, card, target))
        .collect()
}

#[test]
fn a_checkpoint_restores_to_equal_bytes_and_equal_intervals() {
    let events = support::examined("folded event(s)", six_events());
    let (port, state) = folded(&events);
    let bytes = port.checkpoint(&state);
    let restored = port
        .restore(&bytes)
        .expect("the port restores its own checkpoint");
    assert_eq!(
        port.checkpoint(&restored),
        bytes,
        "a round trip gives equal bytes"
    );
    assert_eq!(
        whole_days(&port, &restored),
        whole_days(&port, &state),
        "a round trip gives equal whole days per card"
    );
    assert_eq!(
        (bytes.get(..4), bytes.get(4), bytes.get(5..37), bytes.len()),
        (
            Some(&b"DSSP"[..]),
            Some(&1),
            Some(&IDENTIFIER[..]),
            45 + 49 * 6
        ),
        "the magic, the format, the identifier and 45 bytes plus 49 per event"
    );
    let mut changed = events.clone();
    changed[3].grade = 4;
    let (_, other) = folded(&changed);
    assert_ne!(
        port.checkpoint(&other),
        bytes,
        "one grade changed gives a different checkpoint"
    );
    let (_, again) = folded(&events);
    assert_eq!(
        port.checkpoint(&again),
        bytes,
        "two fresh folds of one set give equal checkpoints"
    );
    let mut long = bytes.clone();
    long.push(0);
    let mut foreign_model = bytes.clone();
    foreign_model[30] ^= 1;
    let refused: Vec<Result<(), LoadError>> = [
        &bytes[..bytes.len() - 1],
        &long[..],
        &b"these bytes are not a checkpoint of anything at all"[..],
        &foreign_model[..],
    ]
    .iter()
    .map(|candidate| port.restore(candidate).map(|_| ()))
    .collect();
    assert_eq!(
        refused,
        vec![
            Err(LoadError::Truncated),
            Err(LoadError::NotACheckpoint),
            Err(LoadError::NotACheckpoint),
            Err(LoadError::OtherModel)
        ],
        "one byte short, one byte long, foreign bytes and another identifier are refused"
    );
    let second = Fsrs7::new(&DEFAULT_PARAMETERS).expect("the typed defaults make a port");
    let mut theirs = second.new_state();
    for event in &events {
        second
            .fold(&mut theirs, event)
            .expect("the second port folds the events");
    }
    assert_eq!(
        whole_days(&port, &theirs),
        vec![Err(FoldError::OtherModel); 3],
        "a state made by a port of another parameter vector is refused"
    );
}

/// Every error variant's text, typed.
fn displays() -> Vec<(String, &'static str)> {
    let key = EventKey(KEY);
    vec![
        (
            FoldError::KeyNotAfter { last: key, key }.to_string(),
            "the review event's key is not after the last folded key",
        ),
        (
            FoldError::GradeOutOfRange { grade: 0 }.to_string(),
            "the review event's grade is outside 1 to 4",
        ),
        (
            FoldError::AnswerTooLong { answer_ms: 1 }.to_string(),
            "the review event's answer time is above the cap of 3,600,000 milliseconds",
        ),
        (
            FoldError::UnknownCard { card: 1 }.to_string(),
            "the scheduling state holds no review of the card",
        ),
        (
            FoldError::BeforeLastReview { card: 1, at_day: 1 }.to_string(),
            "the day asked for is before the card's last review",
        ),
        (
            FoldError::NotFinite { card: 1 }.to_string(),
            "the scheduler's value for the card is not a finite number",
        ),
        (
            FoldError::OtherModel.to_string(),
            "the scheduling state was made under another model's parameters",
        ),
        (
            FoldError::ModelRefused.to_string(),
            "the scheduler refuses the model's parameters or a card's history",
        ),
        (
            LoadError::Truncated.to_string(),
            "the checkpoint's bytes end before its events do",
        ),
        (
            LoadError::NotACheckpoint.to_string(),
            "the bytes are not a checkpoint of the scheduling port",
        ),
        (
            LoadError::OtherModel.to_string(),
            "the checkpoint is another implementation's",
        ),
        (
            LoadError::Refused(FoldError::OtherModel).to_string(),
            "the checkpoint holds an event the fold refuses",
        ),
    ]
}

/// The pin's revision in the root manifest's `fsrs7` line.
fn pinned_revision() -> String {
    let manifest = std::fs::read_to_string(support::workspace().join("Cargo.toml"))
        .expect("the root manifest reads");
    let line = manifest
        .lines()
        .find(|line| line.starts_with("fsrs7 = { package = \"fsrs\", git ="))
        .expect("the root manifest names the FSRS-7 pin");
    let (_, after) = line
        .split_once("rev = \"")
        .expect("the pin names a revision");
    after
        .split('"')
        .next()
        .expect("the revision is quoted")
        .to_owned()
}

#[test]
fn predict_is_a_pure_probe_and_the_version_names_the_model() {
    assert_eq!(
        (Recall::MIN.to_bits(), Recall::MAX.to_bits()),
        (0x3900_0000, 0x3F7F_F800),
        "the recall bounds are 2^-13 and 1 - 2^-13"
    );
    assert_eq!(
        [0.0, 1.0, f32::NAN, 0.9]
            .map(|value| Recall::new(value).map(|recall| recall.get().to_bits())),
        [None, None, None, Some(0.9_f32.to_bits())],
        "a recall outside the bounds, or NaN, is refused"
    );
    assert_eq!(
        [2.0, f32::NAN].map(|value| Recall::clamped(value).map(|recall| recall.get().to_bits())),
        [Some(0x3F7F_F800), None],
        "a clamped recall is held to the bounds and NaN is refused"
    );
    let events = support::examined("folded event(s)", six_events());
    let (port, state) = folded(&events);
    let before = port.checkpoint(&state);
    let last = events[5].study_day;
    assert_eq!(
        port.predict(&state, 1, last)
            .map(|recall| recall.get().to_bits()),
        Ok(0x3F7F_F800),
        "on the card's last review day the curve reads its top, held to the bound"
    );
    assert_eq!(
        (
            port.predict(&state, 1, last - 1),
            port.predict(&state, 9, last)
        ),
        (
            Err(FoldError::BeforeLastReview {
                card: 1,
                at_day: last - 1
            }),
            Err(FoldError::UnknownCard { card: 9 })
        ),
        "a day before the last review and an unknown card are refused"
    );
    let curve: Vec<u32> = (0..=100)
        .map(|days| {
            port.predict(&state, 1, last + days)
                .expect("a day after the last review is predicted")
                .get()
                .to_bits()
        })
        .collect();
    let curve = support::examined("predicted day(s)", curve);
    assert!(
        curve
            .windows(2)
            .all(|pair| f32::from_bits(pair[1]) <= f32::from_bits(pair[0]))
            && curve
                .iter()
                .all(|&bits| (0x3900_0000..=0x3F7F_F800).contains(&bits)),
        "the recall never rises over 100 days and stays in its bounds: {curve:?}"
    );
    assert_eq!(port.checkpoint(&state), before, "a probe changes nothing");
    let (texts, typed): (Vec<String>, Vec<&str>) =
        support::examined("error variant(s)", displays())
            .into_iter()
            .unzip();
    assert_eq!(texts, typed, "every error variant reads as its text");
    let second = Fsrs7::new(&DEFAULT_PARAMETERS).expect("the typed defaults make a port");
    assert_eq!(
        (port.version_id(), second.version_id()),
        (IDENTIFIER, IDENTIFIER),
        "the identifier is fixed whatever the parameters"
    );
    assert_eq!(
        pinned_revision().get(..8).map(str::as_bytes),
        IDENTIFIER.get(22..30),
        "the identifier's 8-hex prefix begins the root manifest's FSRS-7 revision"
    );
}
