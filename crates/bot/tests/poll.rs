//! The long poll: it confirms each update by its offset, drains what was queued before the start
//! without replaying it, and backs off as the predecessor did (SPEC-026 A7, A8, A9; R2, R3, R13).
//!
//! The bot's own loop runs against a fake Bot API until the test's shutdown future resolves; every
//! request's offset, timeout and kinds are read from what the fake recorded, and every wait from its
//! recorder (ADR-026).

// An integration test is test code: its helpers panic on a failed check, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;
#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::cell::Cell;
use std::time::Duration;

use deck_streak_bot::poll::{DRAIN_OFFSET, LONG_POLL_SECONDS, backoff};
use deck_streak_bot::run;
use fake_bot_api::{Answer, Bench, Call, STRANGER, ScriptedSync, message, owner_says};
use serde_json::{Value, json};

/// The `getUpdates` calls among `calls`.
fn polls(calls: &[Call]) -> Vec<&Call> {
    calls
        .iter()
        .filter(|call| call.method == "getUpdates")
        .collect()
}

#[tokio::test]
async fn the_poll_confirms_each_update_by_its_offset() {
    let bench = Bench::start().await;
    bench.fake.script(
        "getUpdates",
        [
            // The drain: nothing queued.
            Answer::updates(Vec::new()),
            Answer::updates(vec![
                owner_says(10, "/privacy"),
                message(11, STRANGER, STRANGER, "private", "/privacy"),
            ]),
            Answer::updates(vec![owner_says(12, "hello")]),
        ],
    );
    let mut commands = bench.commands(ScriptedSync::default());
    let fake = bench.fake.clone();
    run(
        &bench.transport,
        &mut commands,
        async move { fake.until(|calls| polls(calls).len() >= 4).await },
        || {},
    )
    .await;

    let calls = bench.fake.calls();
    let polls = polls(&calls);
    let offsets: Vec<&Value> = polls.iter().map(|call| &call.body["offset"]).collect();
    assert_eq!(
        offsets[..4],
        [&json!(DRAIN_OFFSET), &json!(0), &json!(12), &json!(13)],
        "each poll confirms the updates before its offset"
    );
    for poll in &polls[1..4] {
        assert_eq!(
            poll.body["timeout"], LONG_POLL_SECONDS,
            "a long poll: {}",
            poll.body
        );
        assert_eq!(
            poll.body["allowed_updates"],
            json!(["message", "callback_query"]),
            "only the kinds the bot handles"
        );
    }
    assert_eq!(
        bench.fake.calls_of("sendMessage").len(),
        2,
        "the owner's two messages answered once each, the stranger's not at all"
    );
    // The loop was stopped while waiting on its fourth poll: every handled update was already
    // confirmed by it, so no confirmation followed.
    assert_eq!(
        polls.len(),
        4,
        "{:?}",
        polls.iter().map(|call| &call.body).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn updates_queued_before_start_are_drained_and_not_replayed() {
    let bench = Bench::start().await;
    bench.fake.script(
        "getUpdates",
        [Answer::updates(vec![owner_says(20, "/privacy")])],
    );
    let mut commands = bench.commands(ScriptedSync::default());
    let fake = bench.fake.clone();
    run(
        &bench.transport,
        &mut commands,
        async move { fake.until(|calls| polls(calls).len() >= 2).await },
        || {},
    )
    .await;

    let calls = bench.fake.calls();
    let methods: Vec<&str> = calls.iter().map(|call| call.method.as_str()).collect();
    assert_eq!(
        methods,
        vec![
            "deleteWebhook",
            "deleteMyCommands",
            "setMyCommands",
            "getUpdates",
            "getUpdates"
        ],
        "no webhook, the menu, the drain, then the first poll"
    );
    let drain = &calls[3].body;
    assert_eq!(
        drain["offset"], DRAIN_OFFSET,
        "the drain reads the queue's newest update"
    );
    assert_ne!(drain["timeout"], LONG_POLL_SECONDS, "and waits for none");
    assert_eq!(
        calls[4].body["offset"], 21,
        "the first poll confirms everything queued"
    );
    assert!(
        bench.fake.calls_of("sendMessage").is_empty(),
        "a queued update is never replayed"
    );
}

#[tokio::test]
async fn the_poll_backoff_matches_the_predecessors_golden() {
    let golden = golden::read(&golden::committed("poll_backoff")).expect("the golden reads");
    println!(
        "examined {} case(s) of {}",
        golden.cases.len(),
        golden.function
    );
    assert!(!golden.cases.is_empty(), "examined 0 cases");
    for case in &golden.cases {
        let failures =
            u32::try_from(case.input["failures"].as_u64().expect("failures")).expect("a count");
        let delay = case.output["delay_seconds"].as_f64().expect("seconds");
        assert_eq!(
            backoff(failures),
            Duration::from_secs_f64(delay),
            "{failures} consecutive failure(s)"
        );
    }

    // The loop waits it: three failed polls wait the golden's first three delays, and a success
    // starts the count again, so the next failure waits the first delay once more.
    let delay = |failures: u64| {
        golden
            .cases
            .iter()
            .find(|case| case.input["failures"] == failures)
            .and_then(|case| case.output["delay_seconds"].as_f64())
            .map(Duration::from_secs_f64)
            .expect("the golden holds the count")
    };
    let bench = Bench::start().await;
    bench.fake.script(
        "getUpdates",
        [
            Answer::updates(Vec::new()),
            Answer::status(502),
            Answer::status(502),
            Answer::status(409),
            Answer::updates(Vec::new()),
            Answer::Garbage(502),
        ],
    );
    let mut commands = bench.commands(ScriptedSync::default());
    let fake = bench.fake.clone();
    run(
        &bench.transport,
        &mut commands,
        async move { fake.until(|calls| polls(calls).len() >= 7).await },
        || {},
    )
    .await;
    assert_eq!(
        bench.fake.waits(),
        vec![delay(1), delay(2), delay(3), delay(1)],
        "the waits after each failed poll; none after the success"
    );
    assert_eq!(polls(&bench.fake.calls()).len(), 7);
}

#[tokio::test]
async fn the_role_is_ready_once_the_first_long_poll_is_issued() {
    let bench = Bench::start().await;
    // The start's first request fails once: it is retried after the first backoff.
    bench.fake.script("deleteWebhook", [Answer::status(500)]);
    let mut commands = bench.commands(ScriptedSync::default());
    let fake = bench.fake.clone();
    let calls_at_ready = Cell::new(None);
    run(
        &bench.transport,
        &mut commands,
        async move { fake.until(|calls| polls(calls).len() >= 2).await },
        || calls_at_ready.set(Some(bench.fake.calls().len())),
    )
    .await;
    let calls = bench.fake.calls();
    let methods: Vec<&str> = calls.iter().map(|call| call.method.as_str()).collect();
    assert_eq!(
        methods[..5],
        [
            "deleteWebhook",
            "deleteWebhook",
            "deleteMyCommands",
            "setMyCommands",
            "getUpdates"
        ]
    );
    assert_eq!(
        bench.fake.waits(),
        vec![backoff(1)],
        "the failed start waited its backoff"
    );
    assert_eq!(
        calls_at_ready.get(),
        Some(5),
        "ready after the drain, as the first long poll is issued"
    );
}

#[tokio::test]
async fn a_shutdown_finishes_the_batch_in_hand_and_confirms_its_offset() {
    let bench = Bench::start().await;
    bench.fake.script(
        "getUpdates",
        [
            Answer::updates(Vec::new()),
            Answer::updates(vec![owner_says(30, "/privacy"), owner_says(31, "/start")]),
        ],
    );
    let mut commands = bench.commands(ScriptedSync::default());
    let fake = bench.fake.clone();
    // The shutdown arrives while the batch is handled: as the first reply is sent.
    run(
        &bench.transport,
        &mut commands,
        async move {
            fake.until(|calls| calls.iter().any(|call| call.method == "sendMessage"))
                .await;
        },
        || {},
    )
    .await;
    assert_eq!(
        bench.fake.calls_of("sendMessage").len(),
        2,
        "both updates of the batch are handled"
    );
    let calls = bench.fake.calls();
    let polls = polls(&calls);
    let last = polls.last().expect("a confirmation");
    assert_eq!(
        last.body["offset"], 32,
        "the offset past the batch is confirmed"
    );
    assert_ne!(last.body["timeout"], LONG_POLL_SECONDS, "without waiting");
    assert_eq!(polls.len(), 3, "the drain, the poll, the confirmation");
}

#[tokio::test]
async fn an_update_that_cannot_be_read_is_confirmed_and_not_dispatched() {
    let bench = Bench::start().await;
    bench.fake.script(
        "getUpdates",
        [
            Answer::updates(Vec::new()),
            Answer::updates(vec![
                json!({"update_id": 40, "message": {"message_id": "not a number"}}),
                owner_says(41, "/privacy"),
            ]),
        ],
    );
    let mut commands = bench.commands(ScriptedSync::default());
    let fake = bench.fake.clone();
    run(
        &bench.transport,
        &mut commands,
        async move { fake.until(|calls| polls(calls).len() >= 3).await },
        || {},
    )
    .await;
    let calls = bench.fake.calls();
    assert_eq!(polls(&calls)[2].body["offset"], 42, "past both updates");
    assert_eq!(
        bench.fake.calls_of("sendMessage").len(),
        1,
        "only the readable one answered"
    );
}
