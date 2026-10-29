//! The card health panel on iPhone and iPad through the native adapter (SPEC-507 B1 to B4;
//! ADR-525).
//!
//! B1 builds the card health collection the app's tests open (`support/health.rs`), opens it
//! through the adapter, chooses its deck as the app does and reads its card's record through
//! `Engine::card_health`, the export the app calls. B2 holds the native record equal to the core's,
//! field for field, over generated rows, so the core's differential against the analytics crate
//! (SPEC-507 A2) holds for the record the app shows. B3 judges the recall part and its lines in
//! every state, beside the card's stored FSRS memory state. B4 words the panel's sections and
//! censuses every line of its own words for a verdict, after a planted verdict is caught.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

#[expect(
    dead_code,
    reason = "this test reads the wire helpers alone, never the synthetic collection"
)]
mod support;

#[path = "support/health.rs"]
mod health_fixture;

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_engine_core::health::{
    self as core, Provenance, Readout, ReviewRow, SchedulerState,
};
use deck_streak_ffi::engine::Engine;
use deck_streak_ffi::health::{
    CardHealth, HealthLogged, HealthProvenance, HealthReadout, HealthReview, HealthSection,
    HealthState, card_health_lines,
};
use health_fixture::{BASE_MS, CARD, OLDER_ROWS};
use support::wire;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `DecksService.DeckTree`.
const DECK_TREE: (u32, u32) = (7, 4);
/// `DecksService.SetCurrentDeck`.
const SET_CURRENT_DECK: (u32, u32) = (7, 22);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// The readout this build's native adapter gives: none, as no port carries one yet.
const ABSENT: HealthReadout = HealthReadout::NotAvailable {
    state: HealthState::Absent,
};
/// A Ready readout: 91.2345% now, 12 days to 90%.
const READY: Readout = Readout::Ready {
    recall_ppm: 912_345,
    days_to_90: 12,
};
/// Each state but Ready, the core's and the native one, with the line its recall section reads,
/// written out here rather than read back from the adapter, so a wrong pairing or word fails.
const STATES: [(SchedulerState, HealthState, &str); 4] = [
    (
        SchedulerState::Absent,
        HealthState::Absent,
        "Not available in this build.",
    ),
    (
        SchedulerState::Fixture,
        HealthState::Fixture,
        "Not available: the scheduler is a test fixture.",
    ),
    (
        SchedulerState::Unready,
        HealthState::Unready,
        "Not available: the scheduler has not read this card yet.",
    ),
    (
        SchedulerState::Stale,
        HealthState::Stale,
        "Not available: the scheduler's state is older than this card's last review.",
    ),
];
/// The words that would judge a card rather than state a fact about it (SPEC-507 R8), and the
/// word stems that would.
const VERDICT_WORDS: [&str; 9] = [
    "hard",
    "difficult",
    "ambiguous",
    "leech",
    "risk",
    "risky",
    "problem",
    "problems",
    "weak",
];
const VERDICT_STEMS: [&str; 2] = ["struggl", "trouble"];
/// The answer names a review row may show: the review's own, which record what was pressed.
const ANSWER_NAMES: [&str; 5] = ["Set by hand", "Again", "Hard", "Good", "Easy"];

/// Prints how many `what` the test examined and refuses none: a population that came back empty
/// judged nothing, and every assertion over it would pass.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_secs()
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("the engine's text is UTF-8")
}

/// Builds the card health collection in a scratch directory of the test's own, opens it through
/// the adapter and chooses its deck, as the app does.
fn opened(test: &str) -> (Arc<Engine>, health_fixture::Health) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-card-health")
        .join(format!("{test}-{}-{stamp}", std::process::id()));
    let fixture =
        health_fixture::build(&dir).expect("the builder writes the card health collection");
    let engine = Engine::new(Vec::new()).expect("the engine starts from the default init message");
    let mut open = Vec::new();
    for (field, path) in [
        (1, fixture.collection.clone()),
        (2, fixture.dir.join("collection.media")),
        (3, fixture.dir.join("collection.media.db")),
    ] {
        let path = path.to_str().expect("a scratch path is UTF-8");
        wire::put_bytes(&mut open, field, path.as_bytes());
    }
    let (service, method) = OPEN_COLLECTION;
    engine
        .run(service, method, open)
        .expect("the adapter opens the card health collection");
    let mut tree_request = Vec::new();
    wire::put_varint_field(&mut tree_request, 1, now_secs());
    let (service, method) = DECK_TREE;
    let tree = engine
        .run(service, method, tree_request)
        .expect("the adapter answers the deck tree");
    let deck = wire::repeated(&tree, 3)
        .into_iter()
        .find(|child| text(wire::bytes(child, 2)) == health_fixture::DECK)
        .map(|child| wire::signed(&child, 1))
        .expect("the card health collection's deck is a child of the root");
    let mut chosen = Vec::new();
    wire::put_varint_field(&mut chosen, 1, u64::from_ne_bytes(deck.to_ne_bytes()));
    let (service, method) = SET_CURRENT_DECK;
    engine
        .run(service, method, chosen)
        .expect("the adapter chooses the card health collection's deck");
    (engine, fixture)
}

/// The queue's first card's id, as the engine answers it through the adapter, with the states it
/// is shown with: the app's tests show this card first.
fn head(engine: &Engine) -> i64 {
    let mut request = Vec::new();
    wire::put_varint_field(&mut request, 1, 1);
    let (service, method) = GET_QUEUED_CARDS;
    let reply = engine
        .run(service, method, request)
        .expect("the queue is on the allow-list");
    let queued = wire::repeated(&reply, 1);
    let first = queued
        .first()
        .expect("the card health collection's deck queues a card");
    wire::signed(&wire::bytes(first, 1), 1)
}

/// The record of the health card, read through the adapter's export.
fn record_of(engine: &Engine) -> CardHealth {
    engine
        .card_health(CARD)
        .expect("the two fixed reads run")
        .expect("the card exists")
}

/// The newest 200 of the health card's rows as `(instant less BASE_MS, button, type, capped
/// time)`: its seven rows newest first, then the entries set by hand down to the eighth oldest.
fn newest_rows() -> Vec<(i64, u8, u8, u32)> {
    [
        (7_000, 3, 1, 29_998),
        (6_000, 3, 2, 20_001),
        (5_000, 0, 4, 0),
        (4_000, 1, 1, 12_000),
        (3_000, 3, 0, 60_000),
        (2_000, 1, 0, 0),
        (1_000, 3, 0, 8_000),
    ]
    .into_iter()
    .chain((1..=OLDER_ROWS - 7).map(|age| (-age * 1_000, 0, 4, 0)))
    .collect()
}

/// The lines of the section titled `title`, if the panel has one.
fn section(sections: &[HealthSection], title: &str) -> Option<Vec<String>> {
    sections
        .iter()
        .find(|section| section.title == title)
        .map(|section| section.lines.clone())
}

/// Each section as `(title, lines)`, for an assertion that quotes them.
fn titled(sections: &[HealthSection]) -> Vec<(&str, Vec<&str>)> {
    sections
        .iter()
        .map(|section| {
            (
                section.title.as_str(),
                section.lines.iter().map(String::as_str).collect(),
            )
        })
        .collect()
}

#[test]
fn the_native_record_reads_the_health_cards_figures() {
    let (engine, fixture) = opened("figures");
    assert_eq!(fixture.card, CARD, "the builder fixes the card's id");
    assert_eq!(
        head(&engine),
        CARD,
        "the deck's queue shows the health card, with its states"
    );
    let record = record_of(&engine);
    examined("review row(s)", record.reviews.clone());
    // six answers capped: 0, 8,000, 12,000, 20,001, 29,998 and 60,000. Their sum, 129,999, is odd,
    // and its mean, 21,666.5, rounds half up; the two middle ones' mean, 16,000.5, is floored.
    assert_eq!(
        (
            record.card,
            record.reps,
            record.lapses,
            record.logged,
            record.rows,
            record.answer_ms_total,
            record.answer_ms_mean,
            record.answer_ms_median,
        ),
        (
            CARD,
            8,
            1,
            HealthLogged {
                answers: 6,
                lapses: 1
            },
            207,
            129_999,
            Some(21_667),
            Some(16_000),
        ),
        "the card's counters, the rows' counts and the capped figures"
    );
    assert_eq!(
        record
            .reviews
            .iter()
            .map(|review| (
                review.at_ms - BASE_MS,
                review.grade,
                review.kind,
                review.answer_ms_capped
            ))
            .collect::<Vec<_>>(),
        newest_rows(),
        "the newest 200 of the card's 207 rows by time, newest first, each with its capped time"
    );
    assert_eq!(
        (record.scheduler, record.provenance),
        (ABSENT, HealthProvenance::Collection { created_ms: CARD }),
        "the absent state, and the card came through the collection at the instant its id records"
    );
    assert_eq!(
        engine.card_health(CARD + 1),
        Ok(None),
        "an id no card has reads as no record"
    );
}

/// A seeded generator of the differential's rows (`SplitMix64`), so a failure repeats.
struct Rows(u64);

impl Rows {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> i64 {
        i64::try_from(self.next() % bound).expect("a bound below i64::MAX")
    }

    /// A time near the cap, far past it, negative, or anywhere below the cap.
    fn time(&mut self) -> i64 {
        match self.below(6) {
            0 => i64::from(core::ANSWER_CAP_MS) + self.below(3) - 1,
            1 => self.below(4_000_000),
            2 => -self.below(9_000),
            _ => self.below(70_000),
        }
    }
}

/// The native record the core's `record` must read as: each field as it is, each enum by its own
/// variant, written out here rather than through the adapter's conversion.
fn expected(record: &core::CardHealth) -> CardHealth {
    let Provenance::Collection { created_ms } = record.provenance;
    let scheduler = match record.scheduler {
        Readout::Ready {
            recall_ppm,
            days_to_90,
        } => HealthReadout::Ready {
            recall_ppm,
            days_to_90,
        },
        Readout::NotAvailable(state) => HealthReadout::NotAvailable {
            state: STATES
                .iter()
                .find(|(core, _, _)| *core == state)
                .map(|(_, native, _)| *native)
                .expect("each state but Ready is listed"),
        },
    };
    CardHealth {
        card: record.card,
        reps: record.reps,
        lapses: record.lapses,
        logged: HealthLogged {
            answers: record.logged.answers,
            lapses: record.logged.lapses,
        },
        rows: record.rows,
        reviews: record
            .reviews
            .iter()
            .map(|review| HealthReview {
                at_ms: review.at_ms,
                grade: review.grade,
                kind: review.kind,
                interval: review.interval,
                answer_ms_capped: review.answer_ms_capped,
            })
            .collect(),
        answer_ms_total: record.answer_ms_total,
        answer_ms_mean: record.answer_ms_mean,
        answer_ms_median: record.answer_ms_median,
        scheduler,
        provenance: HealthProvenance::Collection { created_ms },
    }
}

#[test]
fn the_native_record_carries_the_cores_figures_over_generated_rows() {
    let readouts = [
        READY,
        Readout::NotAvailable(SchedulerState::Absent),
        Readout::NotAvailable(SchedulerState::Fixture),
        Readout::NotAvailable(SchedulerState::Unready),
        Readout::NotAvailable(SchedulerState::Stale),
    ];
    let cases: Vec<(i64, Readout)> = (0..300).zip(readouts.into_iter().cycle()).collect();
    let mut generator = Rows(5_070);
    let mut mismatches = Vec::new();
    let (mut over_cap, mut past_bound, mut negative) = (0, 0, 0);
    for (case, readout) in examined("generated card(s)", cases) {
        let count = generator.below(241);
        let mut rows: Vec<ReviewRow> = Vec::new();
        for index in 0..count {
            let (ease, time, kind) = (generator.below(5), generator.time(), generator.below(6));
            if core::is_answer(kind, ease) && time > i64::from(core::ANSWER_CAP_MS) {
                over_cap += 1;
            }
            if core::is_answer(kind, ease) && time < 0 {
                negative += 1;
            }
            rows.push([
                BASE_MS + index * 1_000,
                ease,
                time,
                kind,
                generator.below(40) - 10,
            ]);
        }
        if count > 200 {
            past_bound += 1;
        }
        let card = BASE_MS - 1 - case;
        let snapshot = [card, 2, 2, 0, 12, generator.below(60), generator.below(12)];
        let record = core::card_health(snapshot, &rows, readout);
        let native = CardHealth::from(record.clone());
        if (native.answer_ms_total, native.logged.answers)
            != (record.answer_ms_total, record.logged.answers)
        {
            mismatches.push(format!(
                "case {case}: native {} answers {} ms, core {} answers {} ms",
                native.logged.answers,
                native.answer_ms_total,
                record.logged.answers,
                record.answer_ms_total
            ));
        }
        if native != expected(&record) {
            mismatches.push(format!("case {case}: the native record is not the core's"));
        }
    }
    assert!(
        over_cap > 0 && past_bound > 0 && negative > 0,
        "the generator drew {over_cap} answer(s) past the cap, {past_bound} card(s) past the \
         200-row bound and {negative} negative time(s); each must be judged"
    );
    assert_eq!(
        mismatches,
        Vec::<String>::new(),
        "the native record and the core's"
    );
}

#[test]
fn no_scheduler_figure_reaches_the_native_recall_lines_outside_ready() {
    let (engine, _fixture) = opened("states");
    let record = record_of(&engine);
    // the card stores stability 12.34 days and difficulty 5.6, as a card the previous scheduler
    // kept does; no part of the native record carries them
    let text = format!("{record:?}");
    assert!(
        !text.contains('.') && !text.contains("12.34") && !text.contains("5.6"),
        "no stored stability or difficulty, and no fraction, in {text}"
    );
    assert_eq!(
        record.scheduler, ABSENT,
        "this build's adapter answers the absent state"
    );
    let states = examined("state(s) but Ready", STATES.to_vec());
    for (_, state, line) in states {
        let sections = card_health_lines(Some(CardHealth {
            scheduler: HealthReadout::NotAvailable { state },
            ..record.clone()
        }));
        assert_eq!(
            section(&sections, "Recall"),
            Some(vec![line.to_owned()]),
            "the recall rows read the state's line alone"
        );
        let stored: Vec<&String> = sections
            .iter()
            .flat_map(|section| section.lines.iter())
            .filter(|line| line.contains("12.3") || line.contains("5.6"))
            .collect();
        assert_eq!(
            stored,
            Vec::<&String>::new(),
            "no line carries the stored stability or difficulty"
        );
    }
    let ready = card_health_lines(Some(CardHealth {
        scheduler: HealthReadout::Ready {
            recall_ppm: 912_345,
            days_to_90: 12,
        },
        ..record
    }));
    assert_eq!(
        section(&ready, "Recall"),
        Some(
            [
                "The scheduler estimates",
                "Chance of recall now: 91.2%",
                "Days until recall falls to 90%: 12"
            ]
            .map(String::from)
            .to_vec()
        ),
        "a Ready readout reads the scheduler's label and its two figures"
    );
}

/// The verdict words and stems `line` holds, and whether it says "at risk".
fn verdicts(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    let mut found: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| {
            VERDICT_WORDS.contains(word) || VERDICT_STEMS.iter().any(|stem| word.starts_with(stem))
        })
        .map(str::to_owned)
        .collect();
    if lower.contains("at risk") {
        found.push("at risk".to_owned());
    }
    found
}

/// Whether `line` is a review row's line: its UTC date, an answer name or a button's number, and
/// its time in seconds to one decimal.
fn is_review_line(line: &str) -> bool {
    let parts: Vec<&str> = line.split(", ").collect();
    let [date, name, time] = parts.as_slice() else {
        return false;
    };
    let date_shape = date.len() == 10
        && date.char_indices().all(|(at, c)| {
            if at == 4 || at == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        });
    let named = ANSWER_NAMES.contains(name) || name.chars().all(|c| c.is_ascii_digit());
    let timed = time
        .strip_suffix(" s")
        .and_then(|seconds| seconds.split_once('.'))
        .is_some_and(|(whole, tenth)| {
            !whole.is_empty()
                && whole.chars().all(|c| c.is_ascii_digit())
                && tenth.len() == 1
                && tenth.chars().all(|c| c.is_ascii_digit())
        });
    date_shape && named && timed
}

/// The panel's own words in `sections`, each as `title: line`: every title and every line, but a
/// review row's line, whose answer name is the review's own (SPEC-507 R8). A review row's line
/// that is not of a row's shape is kept, so it is judged too.
fn own_words(sections: &[HealthSection]) -> Vec<String> {
    let mut words = Vec::new();
    for section in sections {
        words.push(section.title.clone());
        for line in &section.lines {
            if section.title != "Newest reviews" || !is_review_line(line) {
                words.push(format!("{}: {line}", section.title));
            }
        }
    }
    words
}

#[test]
fn the_panels_lines_word_the_record_and_name_no_verdict() {
    let (engine, _fixture) = opened("words");
    let sections = card_health_lines(Some(record_of(&engine)));
    let history: Vec<String> = [
        "The newest 200 of 207 reviews.",
        "2023-11-14, Good, 30.0 s",
        "2023-11-14, Good, 20.0 s",
        "2023-11-14, Set by hand, 0.0 s",
        "2023-11-14, Again, 12.0 s",
        "2023-11-14, Good, 60.0 s",
        "2023-11-14, Again, 0.0 s",
        "2023-11-14, Good, 8.0 s",
    ]
    .into_iter()
    .map(String::from)
    .chain((0..193).map(|_| "2023-11-14, Set by hand, 0.0 s".to_owned()))
    .collect();
    assert_eq!(
        titled(&sections),
        vec![
            (
                "Counts and times",
                vec![
                    "Reviews: 8",
                    "Lapses: 1",
                    "Time answering, each answer at most 60 s: 130.0 s",
                    "Mean answer time: 21.7 s",
                    "Median answer time: 16.0 s",
                    "The review log holds 6 answers and 1 lapses.",
                ]
            ),
            ("Recall", vec!["Not available in this build."]),
            (
                "Where it came from",
                vec!["Came through your collection, created 2023-11-14."]
            ),
            (
                "Newest reviews",
                history.iter().map(String::as_str).collect()
            ),
        ],
        "the health card's sections, word by word"
    );
    a_quiet_card_a_named_card_and_no_record_read_their_words();
    the_census_catches_a_planted_verdict_and_finds_none_in_the_panel();
}

/// B4's records beside the health card's: a card with no row, a card whose rows all fit, and no
/// record.
fn a_quiet_card_a_named_card_and_no_record_read_their_words() {
    // A card with no row: no answer time, no log line and no history.
    let quiet = card_health_lines(Some(CardHealth::from(core::card_health(
        [CARD, 0, 0, 0, 0, 0, 0],
        &[],
        Readout::NotAvailable(SchedulerState::Absent),
    ))));
    assert_eq!(
        titled(&quiet),
        vec![
            (
                "Counts and times",
                vec![
                    "Reviews: 0",
                    "Lapses: 0",
                    "Time answering, each answer at most 60 s: 0.0 s",
                    "Mean answer time: No answers yet",
                    "Median answer time: No answers yet",
                ]
            ),
            ("Recall", vec!["Not available in this build."]),
            (
                "Where it came from",
                vec!["Came through your collection, created 2023-11-14."]
            ),
        ],
        "a card with no row"
    );

    // A card whose rows all fit: a Hard, an Easy past the cap and a row whose button has no name;
    // only its lapses differ from the log's, so the log line shows.
    let rows: [ReviewRow; 3] = [
        [BASE_MS + 1_000, 2, 5_000, 1, 3],
        [BASE_MS + 2_000, 4, 61_000, 1, 9],
        [BASE_MS + 3_000, 7, 1_000, 5, 0],
    ];
    let named = card_health_lines(Some(CardHealth::from(core::card_health(
        [CARD, 2, 2, 0, 9, 2, 1],
        &rows,
        READY,
    ))));
    assert_eq!(
        titled(&named),
        vec![
            (
                "Counts and times",
                vec![
                    "Reviews: 2",
                    "Lapses: 1",
                    "Time answering, each answer at most 60 s: 65.0 s",
                    "Mean answer time: 32.5 s",
                    "Median answer time: 32.5 s",
                    "The review log holds 2 answers and 0 lapses.",
                ]
            ),
            (
                "Recall",
                vec![
                    "The scheduler estimates",
                    "Chance of recall now: 91.2%",
                    "Days until recall falls to 90%: 12",
                ]
            ),
            (
                "Where it came from",
                vec!["Came through your collection, created 2023-11-14."]
            ),
            (
                "Newest reviews",
                vec![
                    "2023-11-14, 7, 1.0 s",
                    "2023-11-14, Easy, 60.0 s",
                    "2023-11-14, Hard, 5.0 s",
                ]
            ),
        ],
        "a card whose rows all fit, each named by its button"
    );

    // No record: one line that says so.
    assert_eq!(
        titled(&card_health_lines(None)),
        vec![("Card info", vec!["This card's record could not be read."])],
        "a record that could not be read"
    );
}

/// B4's census: it catches a planted verdict and passes a review row's answer name, then reads
/// every line of the panel's own words over generated records in every state.
fn the_census_catches_a_planted_verdict_and_finds_none_in_the_panel() {
    // The census bites: a planted verdict is caught, by its word and by its phrase, and a review
    // row's answer name is not the panel's own word.
    let planted = [
        HealthSection {
            title: "Recall".to_owned(),
            lines: vec!["This card is a leech at risk.".to_owned()],
        },
        HealthSection {
            title: "Newest reviews".to_owned(),
            lines: vec![
                "2023-11-14, Hard, 5.0 s".to_owned(),
                "A hard card".to_owned(),
            ],
        },
    ];
    assert_eq!(
        own_words(&planted)
            .iter()
            .flat_map(|line| verdicts(line))
            .collect::<Vec<_>>(),
        ["leech", "risk", "at risk", "hard"]
            .map(String::from)
            .to_vec(),
        "the census catches each planted verdict and passes the answer name"
    );

    // Every line of the panel's own words over generated records in every state: none is empty,
    // and none judges the card.
    let mut generator = Rows(50_700);
    let mut lines = Vec::new();
    for case in 0..100 {
        let count = generator.below(230);
        let rows: Vec<ReviewRow> = (0..count)
            .map(|index| {
                [
                    BASE_MS + index * 1_000,
                    generator.below(5),
                    generator.time(),
                    generator.below(6),
                    1,
                ]
            })
            .collect();
        let readout = match case % 5 {
            0 => READY,
            other => Readout::NotAvailable(STATES[usize::try_from(other - 1).expect("an index")].0),
        };
        let snapshot = [CARD, 2, 2, 0, 1, generator.below(60), generator.below(12)];
        let sections = card_health_lines(Some(CardHealth::from(core::card_health(
            snapshot, &rows, readout,
        ))));
        lines.extend(own_words(&sections));
    }
    lines.extend(own_words(&card_health_lines(None)));
    let lines = examined("line(s) of the panel's own words", lines);
    let empty: Vec<&String> = lines
        .iter()
        .filter(|line| line.trim().is_empty() || line.ends_with(": "))
        .collect();
    assert_eq!(
        empty,
        Vec::<&String>::new(),
        "a line VoiceOver would read as nothing"
    );
    let judged: Vec<(&String, Vec<String>)> = lines
        .iter()
        .map(|line| (line, verdicts(line)))
        .filter(|(_, found)| !found.is_empty())
        .collect();
    assert_eq!(
        judged,
        Vec::<(&String, Vec<String>)>::new(),
        "no line of the panel's own words judges the card"
    );
}
