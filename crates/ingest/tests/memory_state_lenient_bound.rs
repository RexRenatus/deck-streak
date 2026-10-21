//! SPEC-480 A7 (R5; ADR-497 D4): the pass bound is the identity on every admitted input.
//!
//! MUTATION COVERAGE, not red first: the bound changes no output; row S48007 is its red.
//!
//! The test holds the base's reader, `leniently`, `number` and `parse` as they stood before the
//! bound, verbatim but for their names, and reads 4096 texts drawn from a fixed seed through both
//! readers: every text the bounded reader parses, to a state or to none, the base reader parses
//! the same.

#![allow(clippy::expect_used, reason = "a failed fixture should fail its test")]

use deck_streak_ingest::memory_state::{DEFAULT_DECAY, MemoryState, parse};

/// The base's reader before the pass bound, verbatim but for the `base_` names.
///
/// Reads `text` the way the predecessor's JSON reader does: the bare words `NaN`, `Infinity` and
/// `-Infinity` and a number too large for a float read as values that no field accepts, so the
/// field is dropped and the rest of the object survives, rather than the whole text failing.
fn base_leniently(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    let mut in_string = false;
    while at < bytes.len() {
        let byte = bytes[at];
        if in_string {
            let first = text[at..].chars().next().map_or(1, char::len_utf8);
            let escaped = if byte == b'\\' {
                text[at + first..].chars().next().map_or(0, char::len_utf8)
            } else {
                0
            };
            let next = at + first + escaped;
            out.push_str(&text[at..next]);
            in_string = byte != b'"';
            at = next;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            out.push('"');
            at += 1;
        } else if let Some(word) = ["-Infinity", "Infinity", "NaN"]
            .iter()
            .find(|word| text[at..].starts_with(**word))
        {
            out.push_str("null");
            at += word.len();
        } else if byte == b'-' || byte.is_ascii_digit() {
            let run = text[at..]
                .find(|c: char| !matches!(c, '0'..='9' | '-' | '+' | '.' | 'e' | 'E'))
                .map_or(text.len(), |n| at + n);
            let token = &text[at..run];
            if token.parse::<f64>().is_ok_and(f64::is_infinite) {
                out.push_str("null");
            } else {
                out.push_str(token);
            }
            at = run;
        } else {
            let width = text[at..].chars().next().map_or(1, char::len_utf8);
            out.push_str(&text[at..at + width]);
            at += width;
        }
    }
    out
}

/// A finite number under `key`; a JSON boolean counts as 1 or 0, as it does in the predecessor.
fn base_number(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<f64> {
    match object.get(key)? {
        serde_json::Value::Number(n) => n.as_f64().filter(|v| v.is_finite()),
        serde_json::Value::Bool(flag) => Some(f64::from(u8::from(*flag))),
        _ => None,
    }
}

/// The memory state in a card's `data` text, or none.
///
/// None for missing, empty, unparsable or non-object text, and for a stability that is missing,
/// not finite or not positive. A decay that is missing or not positive is [`DEFAULT_DECAY`].
#[must_use]
fn base_parse(data: Option<&str>) -> Option<MemoryState> {
    let text = data.filter(|text| !text.is_empty())?;
    let value: serde_json::Value = serde_json::from_str(&base_leniently(text)).ok()?;
    let object = value.as_object()?;
    let stability = base_number(object, "s").filter(|s| *s > 0.0)?;
    let decay = base_number(object, "decay")
        .filter(|d| *d > 0.0)
        .unwrap_or(DEFAULT_DECAY);
    Some(MemoryState {
        stability,
        difficulty: base_number(object, "d").unwrap_or(0.0),
        decay,
        desired_retention: base_number(object, "dr"),
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the predecessor's int() of a float"
        )]
        last_review_sec: base_number(object, "lrt").map(|seconds| seconds.trunc() as i64),
    })
}

/// The seed every run draws its texts from, so each run reads the same population.
const SEED: u64 = 4_801_822;

/// How many texts the population holds.
const TEXTS: usize = 4096;

/// The pieces a text is drawn from: JSON's punctuation and a space, the state's keys and one
/// stranger, finite, overflowing and negative numbers, the predecessor's bare words, JSON's
/// literals, strings holding an escape, a multibyte character, bare words and an escaped quote,
/// and a bare multibyte character and a bare letter.
const PIECES: &[&str] = &[
    "{",
    "}",
    "[",
    "]",
    ",",
    ":",
    " ",
    r#""s""#,
    r#""d""#,
    r#""decay""#,
    r#""dr""#,
    r#""lrt""#,
    r#""k""#,
    "2",
    "12.5",
    "-3",
    "0",
    "4.25e1",
    "1e999",
    "-1e999",
    "NaN",
    "Infinity",
    "-Infinity",
    "true",
    "false",
    "null",
    r#""\n""#,
    "\"\u{e9}\"",
    r#""NaN 1e999""#,
    r#""\"""#,
    "\u{e9}",
    "x",
];

/// The keys an object's pairs are drawn from.
const KEYS: &[&str] = &[
    r#""s""#,
    r#""d""#,
    r#""decay""#,
    r#""dr""#,
    r#""lrt""#,
    r#""k""#,
];

/// The positive numbers an object's stability is drawn from.
const POSITIVE: &[&str] = &["2", "12.5", "4.25e1"];

/// A xorshift64 stream over [`SEED`], so every run draws the same texts with no crate added.
struct Draw(u64);

impl Draw {
    /// The stream's next value.
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    /// A drawn index below `count`.
    fn below(&mut self, count: usize) -> usize {
        let count = u64::try_from(count).expect("a count fits in u64");
        usize::try_from(self.next() % count).expect("an index below a usize count fits in usize")
    }

    /// A drawn element of `items`.
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

/// The seeded population: the even texts an object that opens `{"s":` with a positive number,
/// then zero to six `,key:value` pairs and `}`, with one piece inserted at a drawn position half
/// the time; the odd ones one to twenty-four drawn pieces.
fn seeded_texts() -> Vec<String> {
    let mut draw = Draw(SEED);
    (0..TEXTS)
        .map(|index| {
            let mut pieces: Vec<&str> = Vec::new();
            if index % 2 == 0 {
                pieces.push(r#"{"s":"#);
                pieces.push(draw.pick(POSITIVE));
                for _ in 0..draw.below(7) {
                    pieces.push(",");
                    pieces.push(draw.pick(KEYS));
                    pieces.push(":");
                    pieces.push(draw.pick(PIECES));
                }
                pieces.push("}");
                if draw.below(2) == 0 {
                    let at = draw.below(pieces.len() + 1);
                    pieces.insert(at, draw.pick(PIECES));
                }
            } else {
                for _ in 0..=draw.below(24) {
                    pieces.push(draw.pick(PIECES));
                }
            }
            pieces.concat()
        })
        .collect()
}

#[test]
fn the_bounded_reader_parses_every_seeded_text_as_the_base_reader_does() {
    let texts = seeded_texts();
    let mut differ = Vec::new();
    let mut parsed = 0_usize;
    for (index, text) in texts.iter().enumerate() {
        let bounded = parse(Some(text));
        if bounded != base_parse(Some(text)) {
            differ.push(index);
        }
        if bounded.is_some() {
            parsed += 1;
        }
    }
    println!("examined {} texts, {parsed} parsed to a state", texts.len());
    assert_eq!(
        (differ, texts.len(), parsed > 0),
        (Vec::<usize>::new(), TEXTS, true)
    );
}
