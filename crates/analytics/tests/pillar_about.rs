//! The English score pillar sentences state the pillars' own figures (SPEC-606 R4, A4): the digit
//! runs of the five sentences, in order, are the figures the shipped pillar functions compute from
//! the fixed inputs below, never typed.

// An integration test is test code: it panics on an unreadable message file.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_analytics::constants::MATURE_IVL_DAYS;
use deck_streak_analytics::score::{consistency, mastery, retention, volume, workload};

/// The five sentences' keys, in the order the breakdown shows the pillars.
const KEYS: [&str; 5] = [
    "pillar_consistency_about",
    "pillar_retention_about",
    "pillar_workload_about",
    "pillar_volume_about",
    "pillar_mastery_about",
];

/// Whether a pillar's value is `figure`, far inside the rounding a sentence shows.
fn is(value: f64, figure: f64) -> bool {
    (value - figure).abs() < 1e-9
}

/// The first of `candidates` for which `holds` is true.
fn first<T: Copy>(mut candidates: impl Iterator<Item = T>, holds: impl Fn(T) -> bool) -> T {
    candidates
        .find(|&candidate| holds(candidate))
        .expect("a candidate in the searched range")
}

/// Each pillar's figures, in the order its sentence states them.
fn computed() -> Vec<String> {
    // consistency: studying today, the streak's most, and the streak that reaches it
    let streak = first(0..=1000_i64, |days| is(consistency(1, days), 100.0));
    let studied = consistency(1, 0);
    let consistency_figures = [studied, consistency(1, streak) - studied];

    // retention: the floor and its score, the ceiling and its score, the small sample, its target
    let floor = first((0..=100_i32).rev(), |pct| {
        is(retention(f64::from(pct), 1000), 0.0)
    });
    let ceiling = first(0..=100_i32, |pct| {
        is(retention(f64::from(pct), 1000), 100.0)
    });
    let sample = first(1..=1000_i64, |answered| {
        is(retention(f64::from(floor), answered), 0.0)
    });
    let target = 2.0 * retention(f64::from(floor), 1);

    // workload: one point off, the overdue cards that take it, and the most taken off
    let full = workload(1, 0, 0, true);
    let backlog = first(0..=1_000_000_i64, |overdue| {
        full - workload(1, 0, overdue, true) >= 1.0 - 1e-9
    });
    let workload_figures = [
        full - workload(1, 0, backlog, true),
        full - workload(1, 0, 1_000_000, true),
    ];

    // mastery: the graduations that fill it, the mature interval, the full score, a leech's step,
    // and the most the leeches take off
    let graduations = first(0..=1000_i64, |count| is(mastery(count, 0), 100.0));
    let filled = mastery(graduations, 0);

    vec![
        format!("{:.0}", consistency_figures[0]),
        format!("{:.0}", consistency_figures[1]),
        format!("{streak:.0}"),
        format!("{floor:.0}"),
        format!("{:.0}", retention(f64::from(floor), 1000)),
        format!("{ceiling:.0}"),
        format!("{:.0}", retention(f64::from(ceiling), 1000)),
        format!("{sample:.0}"),
        format!("{target:.0}"),
        format!("{:.0}", workload_figures[0]),
        format!("{backlog:.0}"),
        format!("{:.0}", workload_figures[1]),
        format!("{:.0}", volume(20, 10.0, 20.0, 10.0)),
        format!("{:.0}", volume(25, 12.5, 20.0, 10.0)),
        format!("{graduations:.0}"),
        format!("{MATURE_IVL_DAYS:.0}"),
        format!("{filled:.0}"),
        format!("{:.0}", filled - mastery(graduations, 1)),
        format!("{:.0}", filled - mastery(graduations, 1_000_000)),
    ]
}

#[test]
fn each_pillar_sentence_states_its_pillars_own_figures() {
    let messages: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../web/app/messages/en.json"
        ))
        .expect("the English message file"),
    )
    .expect("the English message file parses");
    let found: Vec<String> = KEYS
        .iter()
        .flat_map(|key| {
            messages[*key]
                .as_str()
                .unwrap_or("")
                .split(|c: char| !c.is_ascii_digit())
                .filter(|run| !run.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(found, computed());
    println!(
        "examined {} figures of the five pillar sentences",
        found.len()
    );
    assert!(
        found.len() >= 19,
        "examined {} figures, fewer than the 19 the five sentences state",
        found.len()
    );
}
