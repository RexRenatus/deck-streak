//! The holds cannot be escaped (SPEC-514 A4): over seeded random configured predicates, every card
//! the wrapped query selects meets every hold, and the text the engine runs parses back to the
//! wrapped query; a planted text that closes the wrap's group is refused before the wrap, while
//! the same text wrapped as text would select a card past the holds.
//!
//! The predicates come from a seeded generator written here: the lock admits no property-testing
//! crate yet, so the property runs over a fixed seed's draws and prints how many it examined.

#![allow(clippy::expect_used)]

use deck_streak_search::{
    Card, Collection, Deck, EngineDay, HOLDS, Limits, Note, Notetype, Profile, Query, REVIEW_ONLY,
    Review, SkipQuery, eval, holds, selects,
};

/// The draws the property examines.
const DRAWS: usize = 3_000;
/// The seed of every draw.
const SEED: u64 = 0x5EA2_C514;

/// A small xorshift generator: the same draws for the same seed.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).expect("a bound")).expect("an index")
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

/// Terms of the skip profile, among them every hold's own term, negated or not.
const TERMS: [&str; 26] = [
    "deck:A",
    "deck:\"Ds Home\"",
    "deck:Ds*",
    "deck:filtered",
    "deck:*",
    "tag:ds",
    "tag:x*",
    "is:new",
    "is:learn",
    "is:review",
    "is:due",
    "is:buried",
    "is:suspended",
    "prop:due=0",
    "prop:due>-2",
    "prop:due<=1",
    "prop:ivl>=3",
    "prop:reps=0",
    "card:1",
    "card:2",
    "note:ds*",
    "amber",
    "\"ember fjord\"",
    "b_sil",
    "c*t",
    "\"deck:A\"",
];
const JOINS: [&str; 4] = [" ", " or ", " OR ", " and "];

fn expression(draw: &mut Draw, depth: usize) -> String {
    let count = 1 + draw.below(3);
    let mut parts = Vec::with_capacity(count);
    for _ in 0..count {
        let part = match draw.below(if depth == 0 { 2 } else { 4 }) {
            0 => draw.pick(&TERMS).to_owned(),
            1 => format!("-{}", draw.pick(&TERMS)),
            2 => format!("({})", expression(draw, depth - 1)),
            _ => format!("-({})", expression(draw, depth - 1)),
        };
        parts.push(part);
    }
    let mut out = parts[0].clone();
    for part in &parts[1..] {
        out.push_str(draw.pick(&JOINS));
        out.push_str(part);
    }
    out
}

/// A collection whose cards take every queue, type and deck the terms name, with due dates on
/// either side of today.
fn collection(draw: &mut Draw) -> Collection {
    let decks = vec![
        Deck {
            id: 2,
            name: "Ds Home".to_owned(),
        },
        Deck {
            id: 3,
            name: "Ds Home::A".to_owned(),
        },
        Deck {
            id: 4,
            name: "A".to_owned(),
        },
        Deck {
            id: 5,
            name: "Ds Filtered".to_owned(),
        },
    ];
    let words = ["amber", "ember fjord", "basil", "cobalt", "dune"];
    let notes = (0..4)
        .map(|i| Note {
            id: i + 1,
            notetype: 0,
            fields: vec![
                words[draw.below(words.len())].to_owned(),
                words[draw.below(words.len())].to_owned(),
            ],
            tags: if draw.below(2) == 0 {
                vec!["ds".to_owned()]
            } else {
                vec!["xy::z".to_owned()]
            },
        })
        .collect();
    let cards = (0..12)
        .map(|i| {
            let queue = [-3, -2, -1, 0, 1, 2, 3, 4][draw.below(8)];
            let kind = [0, 1, 2, 3][draw.below(4)];
            let deck_id = [2, 3, 4, 5][draw.below(4)];
            let borrowed = deck_id == 5;
            Card {
                id: 1_000_000 + i,
                note: draw.below(4),
                deck_id,
                original_deck_id: if borrowed { 2 } else { 0 },
                ord: u16::try_from(draw.below(2)).expect("an ordinal"),
                kind,
                queue,
                due: if matches!(queue, 1 | 4) {
                    10_000 + i64::try_from(draw.below(200_000)).expect("seconds") - 100_000
                } else {
                    i64::try_from(draw.below(5)).expect("days") + 98
                },
                original_due: if borrowed { 100 } else { 0 },
                interval: i64::try_from(draw.below(6)).expect("days"),
                reps: i64::try_from(draw.below(3)).expect("reps"),
                lapses: 0,
            }
        })
        .collect();
    Collection {
        decks,
        notetypes: vec![Notetype {
            id: 1,
            name: "ds.prompt-answer".to_owned(),
            fields: vec!["Prompt".to_owned(), "Answer".to_owned()],
            templates: vec!["Forward".to_owned(), "Reverse".to_owned()],
            sort_field: 0,
        }],
        notes,
        cards,
        revlog: vec![Review {
            id: 1,
            card: 0,
            ease: 3,
        }],
    }
}

/// A day on which day 100 is today.
const TODAY: EngineDay = EngineDay {
    days_elapsed: 100,
    next_day_at: 10_000,
    now: 5_000,
};

/// Room enough for the wrap: its suffix, one group and the holds' six terms.
const ROOMY: Limits = Limits {
    length: 4096,
    depth: 16,
    arity: 64,
};

#[test]
fn the_holds_cannot_be_escaped_by_any_configured_predicate() {
    let mut draw = Draw(SEED);
    let held = holds();
    let (mut accepted, mut selected) = (0, 0);
    for _ in 0..DRAWS {
        let configured = expression(&mut draw, 2);
        let rows = collection(&mut draw);
        let Ok(skip) = SkipQuery::new(&configured) else {
            continue;
        };
        accepted += 1;
        assert_eq!(
            Profile::OPERATOR.parse_within(skip.text(), ROOMY),
            Ok(skip.query()),
            "the text the engine runs reads back as the wrapped query: {configured:?}"
        );
        for index in eval(&skip.query(), &rows, TODAY) {
            selected += 1;
            for hold in &held {
                assert!(
                    selects(hold, &rows, TODAY, index),
                    "{configured:?} selected card {index} past the hold {hold:?}"
                );
            }
        }
    }
    println!(
        "examined {DRAWS} drawn predicate(s): {accepted} wrapped, {selected} selected card(s)"
    );
    assert!(
        accepted * 2 > DRAWS,
        "most drawn predicates are the skip profile's"
    );
    assert!(
        selected > 0,
        "no wrapped predicate selected a card, so no hold was examined"
    );
}

#[test]
fn a_planted_closing_parenthesis_is_refused_and_would_escape_a_text_wrap() {
    let planted = [
        "deck:A) or (deck:A",
        "is:suspended) OR (is:suspended",
        "amber) or (amber",
        "tag:ds) or -(is:new",
        "deck:A) or deck:A (deck:A",
    ];
    let rows = Collection {
        decks: vec![Deck {
            id: 4,
            name: "A".to_owned(),
        }],
        notetypes: vec![Notetype {
            id: 1,
            name: "ds.prompt-answer".to_owned(),
            fields: vec!["Prompt".to_owned()],
            templates: vec!["Forward".to_owned()],
            sort_field: 0,
        }],
        notes: vec![Note {
            id: 1,
            notetype: 0,
            fields: vec!["amber".to_owned()],
            tags: vec!["ds".to_owned()],
        }],
        cards: vec![Card {
            id: 1,
            note: 0,
            deck_id: 4,
            original_deck_id: 0,
            ord: 0,
            kind: 2,
            queue: -1,
            due: 100,
            original_due: 0,
            interval: 3,
            reps: 1,
            lapses: 0,
        }],
        revlog: Vec::new(),
    };
    for text in planted {
        let refusal = SkipQuery::new(text).expect_err("a planted escape is refused");
        assert!(refusal.is_structural(), "{text:?}: {refusal:?}");
        let wrapped_as_text = format!("({text}) {REVIEW_ONLY} {HOLDS}");
        let escaped: Query = Profile::OPERATOR
            .parse(&wrapped_as_text)
            .unwrap_or_else(|refusal| panic!("{wrapped_as_text:?} parses: {refusal}"));
        assert_eq!(
            eval(&escaped, &rows, TODAY),
            vec![0],
            "{text:?} wrapped as text selects the suspended card"
        );
    }
    println!("examined {} planted escape(s)", planted.len());
}
