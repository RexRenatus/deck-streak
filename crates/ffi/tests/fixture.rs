//! The review fixture (SPEC-348 R8, A11).
//!
//! The `review-fixture` example is a thin caller of the builder in `support/review.rs`, and no
//! test of this crate runs an example, so this test includes that builder alone and runs it into
//! a directory under the target's scratch space; the Apple job runs the example itself. The test
//! opens what was written with the engine and reads each file's own header, so a builder that
//! wrote nothing, or wrote something else, fails here.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

#[path = "support/review.rs"]
mod review;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;

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

#[test]
fn the_review_fixture_holds_four_cards_and_two_files() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-review-fixture")
        .join(format!("{}-{stamp}", std::process::id()));
    let fixture = review::build(&dir).expect("the builder writes the review fixture");
    assert!(
        fixture.collection.is_file(),
        "the review fixture wrote its collection at {}",
        fixture.collection.display()
    );

    let media = fixture.dir.join("collection.media");
    let mut names: Vec<String> = examined(
        "media file(s)",
        std::fs::read_dir(&media)
            .expect("the media folder is read")
            .map(|entry| {
                entry
                    .expect("a media entry is read")
                    .file_name()
                    .into_string()
                    .expect("a media name is UTF-8")
            })
            .collect(),
    );
    names.sort();
    assert_eq!(
        names,
        ["dot.png", "tone.wav"],
        "the media folder holds the PNG and the WAV"
    );
    let png = std::fs::read(media.join("dot.png")).expect("the PNG is read");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "the PNG's signature");
    assert_eq!(&png[12..16], b"IHDR", "the PNG opens with its header chunk");
    assert_eq!(
        &png[16..24],
        &[0, 0, 0, 64, 0, 0, 0, 64],
        "the PNG is 64 by 64 pixels"
    );
    let wav = std::fs::read(media.join("tone.wav")).expect("the WAV is read");
    assert_eq!(&wav[..4], b"RIFF", "the WAV is a RIFF file");
    assert_eq!(&wav[8..12], b"WAVE", "the RIFF file holds a WAVE");

    let col = CollectionBuilder::new(&fixture.collection)
        .build()
        .expect("the engine opens the fixture's collection");
    let deck = col
        .get_deck_id("Review")
        .expect("the decks are read")
        .expect("the fixture holds the deck Review");
    let new_cards: Vec<i64> = col
        .storage
        .db()
        .prepare("select id from cards where did = ? and type = 0 order by id")
        .expect("the card read is prepared")
        .query_map([deck.0], |row| row.get(0))
        .expect("the cards are read")
        .collect::<Result<_, _>>()
        .expect("each card's id is read");
    let mut built = vec![
        fixture.cards.text,
        fixture.cards.image,
        fixture.cards.sound,
        fixture.cards.speech,
    ];
    built.sort_unstable();
    assert_eq!(
        new_cards, built,
        "the deck Review holds the builder's four cards, all new"
    );
}
