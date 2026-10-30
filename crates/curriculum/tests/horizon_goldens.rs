//! The horizon, its readout and its constants equal the predecessor's (SPEC-091 A8, A9 and A19; R6
//! and R12): every case of `goldens/horizon.json` (`horizon.py:compute_horizon`),
//! `goldens/horizon_readout.json` (`horizon.py:build_readout`) and `goldens/horizon.constants.json`.
//! Every card is synthetic, and a row's copy count stands for that many identical cards.

// An integration test is test code: its helpers panic on a malformed golden, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;

use deck_streak_curriculum::horizon::{
    FLAT_PEAK_REVIEWS, HORIZON_DAYS, NEW_CARD_LIFETIME_REVIEWS, WINDOW_DAYS, build_readout,
    compute_horizon,
};
use deck_streak_ingest::reader::Card;
use deck_streak_kernel::Track;
use serde_json::Value;

fn integer(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

fn count(value: &Value) -> u64 {
    value.as_u64().unwrap_or_else(|| panic!("a count: {value}"))
}

/// The cards of a golden's rows `[queue, ctype, due, odid, copies]`: each row as many identical
/// cards as its copy count says. The only `Card` literal in this file.
fn cards(rows: &Value) -> Vec<Card> {
    let mut built = Vec::new();
    for (index, row) in rows
        .as_array()
        .expect("the case's cards")
        .iter()
        .enumerate()
    {
        let id = i64::try_from(index).expect("few rows") + 1;
        let card = Card {
            id,
            note_id: id,
            deck_id: 1,
            original_deck_id: integer(&row[3]),
            queue: integer(&row[0]),
            kind: integer(&row[1]),
            due: integer(&row[2]),
            interval: 0,
            factor: 2500,
            reps: 0,
            lapses: 0,
            track: Track::Language,
            course: None,
            tier: None,
            memory: None,
        };
        let copies = usize::try_from(integer(&row[4])).expect("a copy count");
        built.extend(std::iter::repeat_n(card, copies));
    }
    built
}

/// The classes a golden's cases name.
fn classes_of(name: &str) -> BTreeSet<String> {
    let golden = golden::read(&golden::committed(name)).expect("the golden");
    golden
        .cases
        .iter()
        .filter_map(|case| case.class.clone())
        .collect()
}

#[test]
fn the_horizon_matches_the_predecessors_golden() {
    let examined = golden::each_case("horizon", |case| {
        let input = &case.input;
        let scan = compute_horizon(&cards(&input["cards"]), integer(&input["today"]));
        let out = &case.output;
        let curve: Vec<u64> = out["curve"]
            .as_array()
            .expect("the curve")
            .iter()
            .map(count)
            .collect();
        assert_eq!(scan.curve, curve, "the curve of {input}");
        assert_eq!(
            scan.beyond_horizon,
            count(&out["beyond_horizon"]),
            "beyond the horizon, {input}"
        );
        assert_eq!(scan.new_count, count(&out["new_count"]), "new, {input}");
        assert_eq!(
            scan.excluded_count,
            count(&out["excluded_count"]),
            "excluded, {input}"
        );
        assert_eq!(
            scan.total_cards,
            count(&out["total_cards"]),
            "the total, {input}"
        );
    });
    let classes = classes_of("horizon");
    for class in [
        "empty",
        "excluded",
        "type-new",
        "queue-new",
        "intraday-1",
        "intraday-4",
        "day-relearn",
        "legacy-borrowed",
        "modern-borrowed",
        "unknown-queue",
        "negative-today",
        "overdue",
        "edge-364",
        "edge-365",
        "window-29",
        "window-30",
        "peak-25",
        "peak-26",
        "peak-tie",
        "arrears-only",
    ] {
        assert!(classes.contains(class), "the golden has a {class} case");
    }
    assert!(examined.count >= 20, "{examined}: every class is present");
}

#[test]
fn the_horizon_readout_matches_the_predecessors_golden() {
    let examined = golden::each_case("horizon_readout", |case| {
        let input = &case.input;
        let dial = input["desired_retention_pct"]
            .as_f64()
            .expect("the desired retention");
        let readout = build_readout(&cards(&input["cards"]), integer(&input["today"]), dial);
        let out = &case.output;
        assert_eq!(
            readout.window_days,
            usize::try_from(integer(&out["window_days"])).expect("a window"),
            "the window, {input}"
        );
        assert_eq!(
            readout.window_reviews,
            count(&out["window_reviews"]),
            "the window's reviews, {input}"
        );
        assert_eq!(
            readout.total_cards,
            count(&out["total_cards"]),
            "the total, {input}"
        );
        assert_eq!(
            readout.other_cards,
            count(&out["other_cards"]),
            "the other cards, {input}"
        );
        assert_eq!(readout.new_count, count(&out["new_count"]), "new, {input}");
        assert_eq!(
            readout.excluded_count,
            count(&out["excluded_count"]),
            "excluded, {input}"
        );
        assert_eq!(
            readout.due_later,
            count(&out["due_later"]),
            "due later, {input}"
        );
        assert_eq!(
            readout.beyond_horizon,
            count(&out["beyond_horizon"]),
            "beyond the horizon, {input}"
        );
        assert_eq!(
            readout.is_flat,
            out["is_flat"].as_bool().expect("a flag"),
            "flat, {input}"
        );
        assert_eq!(
            readout.peak_day,
            usize::try_from(integer(&out["peak_day"])).expect("a day"),
            "the peak day, {input}"
        );
        assert_eq!(
            readout.peak_reviews,
            count(&out["peak_reviews"]),
            "the peak's reviews, {input}"
        );
        assert_eq!(
            readout.new_card_lifetime_reviews,
            count(&out["new_card_lifetime_reviews"]),
            "the price of a new card, {input}"
        );
        assert_eq!(
            readout.desired_retention_pct.to_bits(),
            out["desired_retention_pct"]
                .as_f64()
                .expect("the dial")
                .to_bits(),
            "the dial, by its bits, {input}"
        );
        assert_eq!(
            readout.relief_text,
            out["relief_text"].as_str().expect("the relief text"),
            "the relief text, {input}"
        );
        assert_eq!(
            readout.honest_text,
            out["honest_text"].as_str().expect("the honest text"),
            "the honest text, {input}"
        );
    });
    let classes = classes_of("horizon_readout");
    for class in [
        "dial",
        "peak-25",
        "peak-26",
        "grouped-1000",
        "grouped-12345",
        "grouped-million",
    ] {
        assert!(classes.contains(class), "the golden has a {class} case");
    }
    assert!(
        examined.count >= 40,
        "{examined}: every class and dial is present"
    );
}

#[test]
fn the_horizon_constants_equal_the_predecessors() {
    let mut named = BTreeSet::new();
    let examined = golden::each_case("horizon.constants", |case| {
        let name = case.input["name"].as_str().expect("a constant's name");
        let ours = match name {
            "horizon.HORIZON_DAYS" => u64::try_from(HORIZON_DAYS).expect("a length"),
            "horizon.WINDOW_DAYS" => u64::try_from(WINDOW_DAYS).expect("a window"),
            "horizon.FLAT_PEAK_REVIEWS" => FLAT_PEAK_REVIEWS,
            "divest.NEW_CARD_LIFETIME_REVIEWS" => NEW_CARD_LIFETIME_REVIEWS,
            other => panic!("a constant the port does not hold: {other}"),
        };
        assert_eq!(
            ours,
            count(&case.output),
            "{name}: ours {ours}, the predecessor's {}",
            case.output
        );
        named.insert(name.to_owned());
    });
    assert_eq!(named.len(), 4, "{examined}: all four constants are named");
}
