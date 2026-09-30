//! What a reading earns (SPEC-047 R6): the two amounts, the two once-scoped sources and the track.

#![allow(clippy::expect_used)]

use deck_streak_kernel::Track;
use deck_streak_readings::reading::ReadingId;
use deck_streak_readings::topic::TopicKey;
use deck_streak_readings::xp::{
    MAX_READING_XP, READ_XP, STUDIED_XP, read_source, studied_source, track_of,
};

fn id() -> ReadingId {
    ReadingId::parse(&"ab".repeat(16)).expect("32 lowercase hex digits")
}

#[test]
fn the_read_tap_earns_forty() {
    assert_eq!(READ_XP, 40);
}

#[test]
fn a_studied_reading_earns_sixty() {
    assert_eq!(STUDIED_XP, 60);
    assert_eq!(MAX_READING_XP, 100);
}

#[test]
fn the_read_and_studied_sources_are_distinct_per_reading() {
    let reading = id();
    assert_eq!(
        read_source(&reading),
        format!("reading:{}:read", reading.as_str())
    );
    assert_eq!(
        studied_source(&reading),
        format!("reading:{}:studied", reading.as_str())
    );
}

#[test]
fn a_topic_earns_on_the_track_of_its_prefix() {
    let law = TopicKey::parse("law/contracts").expect("a law topic");
    let language = TopicKey::parse("language/es").expect("a language topic");
    assert_eq!(track_of(&law), Track::Law);
    assert_eq!(track_of(&language), Track::Language);
}
