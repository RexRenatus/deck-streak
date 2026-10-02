//! The owner gate: an update from anyone but the owner is dropped with no reply, every callback from
//! the owner is answered, and oversized inbound input is not dispatched (SPEC-026 A1, A4, A10; R4,
//! R5, R9; CHARTER 14).
//!
//! Each update goes through the bot's own handlers to a fake Bot API, so "no reply" is read as no
//! call at all, and every absence is paired with the owner's own update, which is answered.

// An integration test is test code: its helpers panic on a failed check, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;
#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_bot::commands::CONFIRM_ERASE;
use deck_streak_bot::gate::{
    Admission, Dropped, MAX_CALLBACK_DATA_BYTES, MAX_INBOUND_TEXT, OwnerMessage, admit,
};
use fake_bot_api::{
    Bench, GROUP, OWNER, STRANGER, ScriptedSync, incoming, message, owner, owner_says, owner_taps,
    tap,
};
use frankenstein::updates::Update;
use serde_json::{Value, json};

/// The gate's decision for the update `value`.
fn decide(value: Value) -> Admission {
    let update: Update = serde_json::from_value(value).expect("an update frankenstein reads");
    admit(&update.content, owner())
}

#[tokio::test]
async fn an_update_from_anyone_but_the_owner_is_dropped_without_a_reply() {
    let bench = Bench::start().await;
    let mut commands = bench.commands(ScriptedSync::default());
    // Every way an update is not the owner's, in the owner's private chat.
    let strangers = [
        (
            message(1, STRANGER, STRANGER, "private", "/start"),
            "not_owner",
        ),
        (
            message(2, STRANGER, OWNER, "private", "/privacy"),
            "not_owner",
        ),
        (
            message(3, OWNER, GROUP, "group", "/start"),
            "not_owners_private_chat",
        ),
        (
            message(4, OWNER, OWNER, "group", "/start"),
            "not_owners_private_chat",
        ),
        (
            message(5, OWNER, STRANGER, "private", "/start"),
            "not_owners_private_chat",
        ),
        (tap(6, STRANGER, CONFIRM_ERASE, 1), "not_owner"),
        (tap(7, STRANGER, "noise", 1), "not_owner"),
    ];
    println!("examined {} update(s) not the owner's", strangers.len());
    for (update, reason) in &strangers {
        let kind = if update.get("callback_query").is_some() {
            "callback_query"
        } else {
            "message"
        };
        assert_eq!(
            decide(update.clone()),
            Admission::Dropped(Dropped { kind, reason }),
            "{update}"
        );
        commands.handle(incoming(update.clone())).await;
    }
    // A kind the bot does not ask for is dropped too.
    let edited = json!({"update_id": 8, "edited_message": message(8, OWNER, OWNER, "private", "/start")["message"]});
    assert_eq!(
        decide(edited.clone()),
        Admission::Dropped(Dropped {
            kind: "update",
            reason: "kind_not_handled"
        })
    );
    commands.handle(incoming(edited)).await;
    // An update frankenstein cannot read is dropped by its id alone.
    commands
        .handle(incoming(
            json!({"update_id": 9, "message": {"message_id": "not a number"}}),
        ))
        .await;
    assert_eq!(bench.fake.calls().len(), 0, "{:?}", bench.fake.calls());

    // The owner's own message in the owner's private chat is answered: the absence above is
    // the gate's, not a harness that never replies.
    commands.handle(incoming(owner_says(10, "/privacy"))).await;
    assert_eq!(bench.fake.calls_of("sendMessage").len(), 1);
    assert_eq!(
        decide(owner_says(11, "/privacy")),
        Admission::Message(OwnerMessage {
            text: "/privacy".to_owned(),
            message_id: 11,
        })
    );
}

#[tokio::test]
async fn every_callback_from_the_owner_is_answered() {
    let bench = Bench::start().await;
    let mut commands = bench.commands(ScriptedSync::default());
    let oversized = "x".repeat(MAX_CALLBACK_DATA_BYTES + 1);
    let taps = [
        owner_taps(1, CONFIRM_ERASE, 50),
        owner_taps(2, "noise", 51),
        owner_taps(3, &oversized, 52),
        json!({
            "update_id": 4,
            "callback_query": {
                "id": "tap-4",
                "from": {"id": OWNER, "is_bot": false, "first_name": "Synthetic"},
                "chat_instance": "synthetic-instance",
            },
        }),
    ];
    println!("examined {} tap(s) of the owner's", taps.len());
    for tap in &taps {
        commands.handle(incoming(tap.clone())).await;
    }
    let answered: Vec<Value> = bench
        .fake
        .calls_of("answerCallbackQuery")
        .into_iter()
        .map(|call| call.body["callback_query_id"].clone())
        .collect();
    assert_eq!(
        answered,
        vec![
            json!("tap-1"),
            json!("tap-2"),
            json!("tap-3"),
            json!("tap-4")
        ],
        "each tap answered once, by its id"
    );
    // A stranger's tap is never answered.
    commands
        .handle(incoming(tap(5, STRANGER, "noise", 53)))
        .await;
    assert_eq!(bench.fake.calls_of("answerCallbackQuery").len(), 4);
}

#[tokio::test]
async fn oversized_inbound_input_is_not_dispatched() {
    // The caps are the predecessor's.
    let constants = golden::read(&golden::committed("bot.constants")).expect("the golden reads");
    let cap = |name: &str| {
        constants
            .cases
            .iter()
            .find(|case| case.input["name"] == name)
            .and_then(|case| case.output.as_u64())
            .and_then(|cap| usize::try_from(cap).ok())
            .unwrap_or_else(|| panic!("{name}"))
    };
    assert_eq!(MAX_INBOUND_TEXT, cap("bot._MAX_INBOUND_TEXT"));
    assert_eq!(MAX_CALLBACK_DATA_BYTES, cap("bot._MAX_CALLBACK_DATA"));

    let bench = Bench::start().await;
    let mut commands = bench.commands(ScriptedSync::default());

    // A text one character over the cap is dropped; one at the cap is dispatched. Each is a
    // command followed by padding, counted in characters as the predecessor counted.
    let at_cap = format!(
        "/privacy {}",
        "é".repeat(MAX_INBOUND_TEXT - "/privacy ".len())
    );
    let over_cap = format!("{at_cap}é");
    assert_eq!(at_cap.chars().count(), MAX_INBOUND_TEXT);
    assert_eq!(
        decide(owner_says(1, &over_cap)),
        Admission::Dropped(Dropped {
            kind: "message",
            reason: "text_over_cap"
        })
    );
    commands.handle(incoming(owner_says(1, &over_cap))).await;
    assert_eq!(bench.fake.calls().len(), 0, "{:?}", bench.fake.calls());
    commands.handle(incoming(owner_says(2, &at_cap))).await;
    assert_eq!(
        bench.fake.calls_of("sendMessage").len(),
        1,
        "at the cap it is answered"
    );

    // Callback data over its bytes is answered and not dispatched; at the cap it is dispatched.
    let over = "é".repeat(MAX_CALLBACK_DATA_BYTES / 2) + "x";
    assert_eq!(over.len(), MAX_CALLBACK_DATA_BYTES + 1);
    assert_eq!(
        decide(owner_taps(3, &over, 60)),
        Admission::AnswerOnly {
            callback_id: "tap-3".to_owned(),
            dropped: Dropped {
                kind: "callback_query",
                reason: "data_over_cap"
            }
        }
    );
    let at = "é".repeat(MAX_CALLBACK_DATA_BYTES / 2);
    assert!(
        matches!(decide(owner_taps(4, &at, 61)), Admission::Callback(callback) if callback.data.as_deref() == Some(at.as_str())),
        "at the cap the tap is dispatched"
    );
    let sends = bench.fake.calls_of("sendMessage").len();
    commands.handle(incoming(owner_taps(3, &over, 60))).await;
    assert_eq!(
        bench.fake.calls_of("answerCallbackQuery").len(),
        1,
        "answered"
    );
    assert_eq!(
        bench.fake.calls_of("sendMessage").len(),
        sends,
        "and nothing dispatched"
    );
}
