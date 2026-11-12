//! A card's Bloom tier is read from its note's tags and kept as a tier, never as text (SPEC-072
//! A3, A4; R3; ADR-072). Every tag here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use deck_streak_ingest::tier::{Tier, parse_tier};
use support::Fixture;
use support::synthetic::{self, PlannedCard};

const ENDPOINT: &str = "http://127.0.0.1:9/";

#[test]
fn the_bloom_tier_parse_matches_the_parity_golden() {
    let mut tiers = std::collections::BTreeSet::new();
    golden::each_case("parse_tier", |case| {
        // The golden's tags that are not text (a null, an integer, a list) are the predecessor's
        // own type check; a note's tags here are always text.
        let Some(tags) = case.input["tags"].as_str() else {
            return;
        };
        let expected = case.output.as_str();
        let ours = parse_tier(tags).map(Tier::as_str);
        assert_eq!(ours, expected, "the tier of {tags:?}");
        tiers.extend(ours);
    });
    assert_eq!(
        tiers.len(),
        4,
        "every tier was parsed at least once: {tiers:?}"
    );
}

#[tokio::test]
async fn the_reader_keeps_each_cards_tier_and_never_its_tags() {
    let fixture = Fixture::new(ENDPOINT);
    let planned = [
        PlannedCard {
            id: 1,
            deck: "Qaa::Unit 01",
            filtered: false,
        },
        PlannedCard {
            id: 2,
            deck: "Qaa::Unit 01",
            filtered: false,
        },
        PlannedCard {
            id: 3,
            deck: "Qaa::Unit 02",
            filtered: false,
        },
        PlannedCard {
            id: 4,
            deck: "Qaa::Unit 02",
            filtered: true,
        },
    ];
    synthetic::build_planned(&fixture.copy(), &[], &planned, &[]);
    synthetic::set_tags(&fixture.copy(), 1, " Zzsecret::Topic T3 ");
    synthetic::set_tags(&fixture.copy(), 2, "t1x Zzsecret::Other");
    synthetic::set_tags(&fixture.copy(), 4, "T4");
    let reader = synthetic::reader(&fixture.settings(), "", None, support::clock_at(0));
    let data = reader.read(0).await.expect("the copy reads");
    let read: Vec<(i64, Option<&str>)> = data
        .cards
        .iter()
        .map(|card| (card.id, card.tier.map(Tier::as_str)))
        .collect();
    println!("examined {} cards read", read.len());
    assert_eq!(
        read,
        vec![(1, Some("T3")), (2, None), (3, None), (4, Some("T4"))]
    );
    let output = format!("{data:?}");
    assert!(
        !output.contains("Zzsecret") && !output.contains("Topic"),
        "no tags text reached the read's output"
    );
}
