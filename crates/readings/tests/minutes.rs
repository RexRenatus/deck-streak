//! A reading's word count and minutes (SPEC-046 A16, R9).

#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_readings::reading::{ReadingId, minutes, word_count};

fn words(n: usize) -> String {
    vec!["word"; n].join(" ")
}

#[test]
fn reading_minutes_count_words_at_200_per_minute() {
    assert_eq!(
        minutes(&words(200), None),
        1,
        "two hundred words is one minute"
    );
    assert_eq!(minutes(&words(201), None), 2, "one word more rounds up");
    assert_eq!(minutes(&words(199), None), 1);
    assert_eq!(minutes(&words(1), None), 1, "a single word is one minute");
    assert_eq!(minutes(&words(800), None), 4);
    assert_eq!(
        minutes(&words(1500), None),
        8,
        "1500 words is seven and a half minutes, rounded up"
    );
    assert_eq!(
        minutes(&words(1400), Some("es")),
        7,
        "a language other than Chinese or Japanese"
    );
}

#[test]
fn chinese_and_japanese_count_characters_at_a_word_rate() {
    let zh = "字".repeat(300); // 1.5 characters to a word: 200 words
    assert_eq!(minutes(&zh, Some("zh")), 1);
    assert_eq!(minutes(&"字".repeat(301), Some("zh")), 2);
    let ja = "字".repeat(400); // 2.0 characters to a word: 200 words
    assert_eq!(minutes(&ja, Some("ja")), 1);
    assert_eq!(minutes(&"字".repeat(401), Some("ja")), 2);
}

#[test]
fn the_word_count_counts_alphanumeric_words() {
    assert_eq!(word_count("one two  three\nfour"), 4);
    assert_eq!(
        word_count("it's a well-known rule."),
        5,
        "an apostrophe joins, a hyphen splits"
    );
    assert_eq!(word_count(""), 0);
}

#[test]
fn the_reading_id_is_32_hex_digits_of_the_topic_the_first_day_and_the_digest() {
    let digest = "a".repeat(64);
    let id = ReadingId::of("law/evidence", 20_000, &digest);
    assert_eq!(id.as_str().len(), 32);
    assert!(
        id.as_str()
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    assert_eq!(
        id,
        ReadingId::of("law/evidence", 20_000, &digest),
        "it is stable"
    );
    assert_ne!(
        id,
        ReadingId::of("law/torts", 20_000, &digest),
        "the topic moves it"
    );
    assert_ne!(
        id,
        ReadingId::of("law/evidence", 20_001, &digest),
        "the first day moves it"
    );
    assert_ne!(
        id,
        ReadingId::of("law/evidence", 20_000, &"b".repeat(64)),
        "the digest moves it"
    );
    assert_eq!(
        id.as_str(),
        "af26362a828a521e787ded0de814f604",
        "the first 32 hex digits of SHA-256 over the topic, the day and the digest, newline-joined"
    );
    println!("examined 1 reading id");
}
