//! A11 (SPEC-602 R11): the field codec, `join(split(s)) == s`, `split(join(fs)) == fs` when no
//! field holds U+001F, and the separator U+001F, over fixed vectors and a fixed-seed generator.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use deck_streak_anki_format::fields::{FIELD_SEPARATOR, join, split};
use support::examined;

/// A fixed-seed generator (xorshift64), so every run draws the same texts.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn pick(&mut self, alphabet: &[char], most: u64) -> String {
        (0..self.next() % (most + 1))
            .map(|_| alphabet[usize::try_from(self.next()).expect("an index") % alphabet.len()])
            .collect()
    }
}

const WITH_SEPARATOR: [char; 6] = ['a', 'B', ' ', '\u{e9}', '\u{1e}', '\u{1f}'];
const WITHOUT_SEPARATOR: [char; 5] = ['a', 'B', ' ', '\u{e9}', '\u{1e}'];

#[test]
fn join_of_split_is_the_text() {
    let mut texts: Vec<String> = ["", "a", "\u{1f}", "a\u{1f}b", "\u{1f}\u{1f}x\u{1f}"]
        .iter()
        .map(|text| (*text).to_owned())
        .collect();
    let mut rng = Seeded(0x0006_02f1_e1d5);
    texts.extend((0..300).map(|_| rng.pick(&WITH_SEPARATOR, 12)));
    for text in examined("text(s)", texts) {
        assert_eq!(join(&split(&text)), text, "{text:?}");
    }
}

#[test]
fn split_of_join_is_the_fields_without_a_separator() {
    let mut cases: Vec<Vec<String>> = vec![
        vec![String::new()],
        vec!["front".to_owned(), "back".to_owned()],
        vec![String::new(), String::new(), "x".to_owned()],
    ];
    let mut rng = Seeded(0x0602_c0de);
    for _ in 0..300 {
        let count = 1 + rng.next() % 5;
        cases.push(
            (0..count)
                .map(|_| rng.pick(&WITHOUT_SEPARATOR, 6))
                .collect(),
        );
    }
    for fields in examined("field list(s)", cases) {
        assert_eq!(split(&join(&fields)), fields, "{fields:?}");
    }
}

#[test]
fn the_separator_is_the_unit_separator() {
    assert_eq!(FIELD_SEPARATOR, '\u{1f}', "the separator");
    assert_eq!(join(&["a", "b"]), "a\u{1f}b", "the joined text");
    assert_eq!(
        split("a\u{1f}b\u{1e}c"),
        ["a", "b\u{1e}c"],
        "the split fields"
    );
    examined("separator check(s)", vec![3]);
}
