//! The flush of held celebrations (SPEC-084 A10): it re-caps each for the flush's study day, renders
//! in full the two of highest held tier in the order they were held, and rolls the rest up at most
//! at T2, as the golden of the predecessor's `flush_deferred_celebrations` does.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use std::collections::BTreeMap;

use deck_streak_notifications::Flushed;
use deck_streak_notifications::occasion::StreakFacts;
use serde_json::Value;
use sqlx::Row;
use support::ladder::{
    HeldSeed, assert_calls, clear_owner_message, port_calls, queue_rows, script, scripted,
    seed_held, seed_owner_message, tier,
};
use support::{DAY, Harness, MINUTE_MS, at, day};

/// What became of one held celebration.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Fate {
    /// Still held, at its tier, with its hold and its failed sends.
    Held(String, String, i64),
    /// Delivered at a tier: in full, as a reaction, or named by the rollup.
    Settled(String),
    /// Given up: abandoned, or named as abandoned by the rollup.
    Dropped,
}

/// Each key's fate as the golden's output says.
fn wanted(input: &Value, output: &Value) -> BTreeMap<String, Fate> {
    let mut fates = BTreeMap::new();
    for row in input["rows"].as_array().expect("rows") {
        let key = row["key"].as_str().expect("a key").to_owned();
        let rank = tier(row["tier"].as_u64().expect("a tier"))
            .as_str()
            .to_owned();
        let hold = row["hold"].as_str().expect("a hold").to_owned();
        let tries = row["tries"].as_i64().expect("a count");
        fates.insert(key, Fate::Held(rank, hold, tries));
    }
    for settled in output["settled"].as_array().expect("settled") {
        let key = settled[0].as_str().expect("a key").to_owned();
        let rank = settled[1].as_u64().expect("a tier");
        let fate = if rank == 0 {
            Fate::Dropped
        } else {
            Fate::Settled(tier(rank).as_str().to_owned())
        };
        fates.insert(key, fate);
    }
    for latched in output["latched"].as_array().expect("latched") {
        let key = latched[0].as_str().expect("a key").to_owned();
        let rank = latched[1].as_u64().expect("a tier");
        let fate = if rank == 0 {
            Fate::Dropped
        } else {
            Fate::Held(
                tier(rank).as_str().to_owned(),
                latched[2].as_str().unwrap_or("quiet").to_owned(),
                latched[3].as_i64().unwrap_or(0),
            )
        };
        fates.insert(key, fate);
    }
    fates
}

/// Each key's fate as the router left it.
async fn fates(harness: &Harness, keys: impl Iterator<Item = String>) -> BTreeMap<String, Fate> {
    let queue = queue_rows(harness).await;
    let mut fates = BTreeMap::new();
    for key in keys {
        let fate = if let Some(row) = queue.iter().find(|row| row.0 == key) {
            if row.1 == "abandoned" {
                Fate::Dropped
            } else {
                Fate::Held(row.2.clone(), row.3.clone(), row.4)
            }
        } else {
            let last = sqlx::query(
                "SELECT arm, tier_rendered FROM notification_decisions WHERE dedupe_key = ? \
                 ORDER BY id DESC LIMIT 1",
            )
            .bind(&key)
            .fetch_one(harness.db.reader())
            .await
            .expect("a settled celebration has a decision");
            let arm: String = last.get(0);
            if arm == "send" {
                Fate::Settled(last.get(1))
            } else {
                Fate::Dropped
            }
        };
        fates.insert(key, fate);
    }
    fates
}

#[tokio::test]
async fn a_flush_ranks_re_caps_and_rolls_up_as_the_parity_golden_does() {
    let mut cases = Vec::new();
    let examined = golden::each_case("celebration_flush", |case| {
        cases.push((case.input.clone(), case.output.clone(), case.class.clone()));
    });
    assert!(examined.count > 0);
    for (input, output, class) in cases {
        let now = at(DAY, 12, 0);
        let (harness, bot) = scripted(now).await;
        let broke = input["broke"].as_bool() == Some(true);
        let cap = if broke { 1 } else { 5 };
        let mut rows: Vec<Value> = input["rows"].as_array().expect("rows").clone();
        rows.sort_by_key(|row| row["order"].as_i64());
        let mut tiers = BTreeMap::new();
        for row in &rows {
            let key = row["key"].as_str().expect("a key");
            let rank = row["tier"].as_u64().expect("a tier");
            tiers.insert(key.to_owned(), rank.min(cap));
            let seed = HeldSeed {
                key,
                tier: tier(rank),
                hold: row["hold"].as_str().expect("a hold"),
                tries: row["tries"].as_i64().expect("a count"),
                state: "held",
                deferred_at: now.epoch_millis()
                    - row["age_minutes"].as_i64().expect("an age") * MINUTE_MS,
                study_day: DAY,
            };
            seed_held(&harness, &seed).await;
        }
        match input["owner"]["age_minutes"].as_i64() {
            Some(age) => {
                seed_owner_message(&harness, 5, now.epoch_millis() - age * MINUTE_MS).await;
            }
            None => clear_owner_message(&harness).await,
        }
        let fail: Vec<String> = serde_json::from_value(input["fail"].clone()).expect("a list");
        let calls = output["calls"].as_array().expect("calls");
        let expected = port_calls(calls, &fail, |group| {
            group[0][0]
                .as_str()
                .and_then(|key| tiers.get(key).copied())
                .unwrap_or(2)
        });
        script(&bot, &expected);
        let facts = broke.then_some(StreakFacts {
            last_study_day: Some(day(DAY)),
            current: 1,
            longest: 2,
        });
        let flushed = harness.router.flush_with(facts).await.expect("a flush");
        let context = format!("{class:?} {input}");
        assert_calls(&bot, &expected, &context);
        let engaged = u32::try_from(output["engaged"].as_u64().expect("a count")).expect("a count");
        assert_eq!(
            flushed,
            Flushed::Ran { sends: engaged },
            "{context}: the messages the flush delivered"
        );
        let keys = rows
            .iter()
            .map(|row| row["key"].as_str().expect("a key").to_owned());
        assert_eq!(
            fates(&harness, keys).await,
            wanted(&input, &output),
            "{context}: what became of each held celebration"
        );
    }
}
