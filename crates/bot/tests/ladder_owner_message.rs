//! The owner's latest message (SPEC-084 A13; R13): every message the owner gate admits records its
//! id and the instant it arrived, which a T1 celebration reacts to, and a message from another chat
//! records nothing.
//!
//! Each update goes through the bot's own handlers, so the record is read as the gate and the
//! handlers leave it, over a fake Bot API.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use std::time::Duration;

use deck_streak_kernel::UtcMillis;
use deck_streak_notifications::owner_message::{self, LatestMessage};
use fake_bot_api::{
    BENCH_STARTED_AT, Bench, GROUP, OWNER, STRANGER, ScriptedSync, incoming, message,
};
use serde_json::{Value, json};

/// The update `update_id` carrying the message `message_id` from `from` in `chat`: the two ids
/// differ, so the record is read as the message's id and never the update's.
fn said(update_id: i64, message_id: i64, from: i64, chat: i64, chat_type: &str) -> Value {
    let mut update = message(update_id, from, chat, chat_type, "a synthetic line");
    update["message"]["message_id"] = json!(message_id);
    update
}

#[tokio::test]
async fn the_owners_latest_message_is_recorded_and_no_other_chats() {
    let bench = Bench::start().await;
    let mut commands = bench.commands(ScriptedSync::default());
    let latest = || async {
        owner_message::latest(&bench.db)
            .await
            .expect("the owner's latest message is read")
    };
    assert_eq!(
        latest().await,
        None,
        "no message before the owner writes one"
    );

    commands
        .handle(incoming(said(11, 901, OWNER, OWNER, "private")))
        .await;
    let first = LatestMessage {
        message_id: 901,
        arrived_at: UtcMillis::from_epoch_millis(BENCH_STARTED_AT),
    };
    assert_eq!(
        latest().await,
        Some(first),
        "the owner's message: its id and the instant it arrived"
    );

    // Another chat's message, a minute on: a stranger in their own chat, a stranger in the owner's
    // chat, and the owner in a group. The gate admits none of them, so none is recorded.
    bench.clock.advance(Duration::from_mins(1));
    for (update, from, chat, chat_type) in [
        (12, STRANGER, STRANGER, "private"),
        (13, STRANGER, OWNER, "private"),
        (14, OWNER, GROUP, "group"),
    ] {
        commands
            .handle(incoming(said(update, 950 + update, from, chat, chat_type)))
            .await;
        assert_eq!(
            latest().await,
            Some(first),
            "update {update}, from {from} in {chat}: another chat's message is never recorded"
        );
    }

    // The owner's next message replaces the first: the latest one, at its own instant.
    bench.clock.advance(Duration::from_mins(1));
    commands
        .handle(incoming(said(15, 902, OWNER, OWNER, "private")))
        .await;
    assert_eq!(
        latest().await,
        Some(LatestMessage {
            message_id: 902,
            arrived_at: UtcMillis::from_epoch_millis(BENCH_STARTED_AT + 120_000),
        }),
        "the latest message replaces the one before it"
    );
}
