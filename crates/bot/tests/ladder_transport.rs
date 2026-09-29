//! The bot's reveal (SPEC-084 A14; R8): a T3 celebration sends a placeholder, waits the reveal's
//! pause and edits the placeholder into the message; when the edit gives up, the message is sent
//! anew, and when the placeholder gives up, the message is sent in its place. And the ladder's
//! other renders on the bot's transport (R8, R9): the reaction to the owner's latest message, the
//! dice and the pinned message, each as Telegram receives it.
//!
//! The router renders through the bot's own transport to a fake Bot API, so each tier's calls are
//! read as Telegram would receive them; the pause is a noted wait that returns at once.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use std::sync::Arc;

use deck_streak_bot::transport::SEND_ATTEMPTS;
use deck_streak_bot::{OwnerChat, SendCounts};
use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_notifications::ladder::{
    DICE_EMOJI, REACTION_EMOJI, REVEAL_PAUSE, REVEAL_PLACEHOLDER,
};
use deck_streak_notifications::{
    Decision, DedupeKey, Hold, LapseContext, Occasion, Policy, Router, Surface, Tier, owner_message,
};
use fake_bot_api::{Answer, FakeBotApi, OWNER, payload};
use serde_json::json;

/// The celebration's message.
const TEXT: &str = "<b>A</b> synthetic record";

/// Each call's method, and the text it carried.
fn calls(fake: &FakeBotApi) -> Vec<(String, String)> {
    fake.calls()
        .iter()
        .map(|call| {
            (
                call.method.clone(),
                payload(call)["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect()
}

#[tokio::test]
async fn a_reveal_edits_its_placeholder_and_falls_back_to_a_new_message() {
    let fake = FakeBotApi::start().await;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = Arc::new(fake.transport(directory.path()));
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let noon = UtcMillis::from_epoch_millis(20_000 * 86_400_000 + 12 * 3_600_000);
    let router = Router::new(
        Arc::clone(&policy),
        db,
        Arc::new(ManualClock::new(noon)),
        StudyDayRule::default(),
    )
    .with_bot(Arc::new(OwnerChat::new(
        Arc::clone(&transport),
        fake_bot_api::owner(),
    )));
    // A record is a T3 event: its celebration renders as a reveal.
    let record = |key: &str| {
        Occasion::new(
            policy.kind("celebration").expect("the celebration kind"),
            DedupeKey::new(key).expect("a key"),
            Surface::Bot,
            Tier::T2,
            TEXT,
            StudyDay::from_epoch_day(20_000),
            LapseContext::NoLapse,
        )
        .expect("an occasion")
        .with_event("record", None)
    };
    let revealed = Decision::Sent {
        surface: Surface::Bot,
        tier: Tier::T3,
    };
    let placeholder = (String::from("sendMessage"), REVEAL_PLACEHOLDER.to_owned());
    let edit = (String::from("editMessageText"), TEXT.to_owned());
    let anew = (String::from("sendMessage"), TEXT.to_owned());

    // The reveal: the placeholder, the pause, then the placeholder edited into the message.
    let decision = router
        .route(&record("record:one"))
        .await
        .expect("a decision");
    assert_eq!(decision, revealed, "a record renders as a reveal");
    assert_eq!(calls(&fake), vec![placeholder.clone(), edit.clone()]);
    let sent = fake.calls();
    assert_eq!(sent[0].body["chat_id"], json!(OWNER), "the owner's chat");
    assert_eq!(
        payload(&sent[1])["message_id"],
        json!(sent[0].message_id.expect("the placeholder's id")),
        "the edit names the placeholder"
    );
    assert_eq!(
        fake.waits(),
        vec![REVEAL_PAUSE],
        "one wait between the two: the reveal's pause"
    );

    // The edit gives up after its attempts: the message is sent anew.
    fake.script(
        "editMessageText",
        (0..SEND_ATTEMPTS).map(|_| Answer::status(502)),
    );
    let before = fake.calls().len();
    let decision = router
        .route(&record("record:two"))
        .await
        .expect("a decision");
    assert_eq!(decision, revealed, "the message still arrived");
    let mut expected = vec![placeholder.clone()];
    expected.extend((0..SEND_ATTEMPTS).map(|_| edit.clone()));
    expected.push(anew.clone());
    assert_eq!(calls(&fake)[before..], expected[..], "a failed edit");

    // The placeholder gives up after its attempts: the message is sent in its place, unedited.
    fake.script(
        "sendMessage",
        (0..SEND_ATTEMPTS).map(|_| Answer::status(502)),
    );
    let before = fake.calls().len();
    let decision = router
        .route(&record("record:three"))
        .await
        .expect("a decision");
    assert_eq!(decision, revealed, "the message still arrived");
    let mut expected: Vec<_> = (0..SEND_ATTEMPTS).map(|_| placeholder.clone()).collect();
    expected.push(anew);
    assert_eq!(calls(&fake)[before..], expected[..], "a failed placeholder");
    println!("examined: 3 reveals");
}

#[tokio::test]
async fn the_reaction_the_dice_and_the_pin_reach_the_owners_chat() {
    let fake = FakeBotApi::start().await;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = Arc::new(fake.transport(directory.path()));
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let noon = UtcMillis::from_epoch_millis(20_000 * 86_400_000 + 12 * 3_600_000);
    let router = Router::new(
        Arc::clone(&policy),
        db.clone(),
        Arc::new(ManualClock::new(noon)),
        StudyDayRule::default(),
    )
    .with_bot(Arc::new(OwnerChat::new(
        Arc::clone(&transport),
        fake_bot_api::owner(),
    )));
    let celebration = |key: &str, tier: Tier, event: Option<&str>| {
        let occasion = Occasion::new(
            policy.kind("celebration").expect("the celebration kind"),
            DedupeKey::new(key).expect("a key"),
            Surface::Bot,
            tier,
            TEXT,
            StudyDay::from_epoch_day(20_000),
            LapseContext::NoLapse,
        )
        .expect("an occasion");
        match event {
            Some(event) => occasion.with_event(event, None),
            None => occasion,
        }
    };
    let sent = |tier| Decision::Sent {
        surface: Surface::Bot,
        tier,
    };
    let since = |before: usize| -> Vec<(String, serde_json::Value)> {
        fake.calls()[before..]
            .iter()
            .map(|call| (call.method.clone(), call.body.clone()))
            .collect()
    };

    // T1 reacts to the owner's latest message, an hour old, with the ladder's emoji.
    owner_message::record(
        &db,
        901,
        UtcMillis::from_epoch_millis(noon.epoch_millis() - 3_600_000),
    )
    .await
    .expect("the owner's message is recorded");
    let decision = router
        .route(&celebration("small:one", Tier::T1, None))
        .await
        .expect("a decision");
    assert_eq!(decision, sent(Tier::T1), "a T1 is a reaction");
    let calls = since(0);
    assert_eq!(calls.len(), 1, "one reaction: {calls:?}");
    assert_eq!(calls[0].0, "setMessageReaction");
    assert_eq!(calls[0].1["chat_id"], json!(OWNER), "in the owner's chat");
    assert_eq!(
        calls[0].1["message_id"],
        json!(901),
        "to the owner's message"
    );
    assert_eq!(
        calls[0].1["reaction"],
        json!([{"type": "emoji", "emoji": REACTION_EMOJI}]),
        "with the ladder's emoji"
    );
    assert_eq!(
        transport.counts(),
        SendCounts::default(),
        "a reaction is no message"
    );

    // T4: the dice, then the message; both are messages.
    let before = fake.calls().len();
    let decision = router
        .route(&celebration("dice:one", Tier::T2, Some("queue_zero")))
        .await
        .expect("a decision");
    assert_eq!(decision, sent(Tier::T4), "a queue at zero is a T4");
    let calls = since(before);
    let methods: Vec<&str> = calls.iter().map(|(method, _)| method.as_str()).collect();
    assert_eq!(
        methods,
        ["sendDice", "sendMessage"],
        "the dice, then the message"
    );
    assert_eq!(calls[0].1["chat_id"], json!(OWNER), "in the owner's chat");
    assert_eq!(calls[0].1["emoji"], json!(DICE_EMOJI), "the ladder's dice");
    assert_eq!(calls[1].1["text"], json!(TEXT));
    assert_eq!(
        transport.counts(),
        SendCounts {
            attempted: 2,
            delivered: 2
        },
        "a dice is a message"
    );

    // T5: the dice, the message, then the message pinned without a notification. A pin is no
    // message.
    let before = fake.calls().len();
    let decision = router
        .route(&celebration("trophy:one", Tier::T2, Some("ceremony")))
        .await
        .expect("a decision");
    assert_eq!(decision, sent(Tier::T5), "a ceremony is a T5");
    let calls = since(before);
    let methods: Vec<&str> = calls.iter().map(|(method, _)| method.as_str()).collect();
    assert_eq!(methods, ["sendDice", "sendMessage", "pinChatMessage"]);
    let message = fake.calls()[before + 1]
        .message_id
        .expect("the message's id");
    assert_eq!(calls[2].1["chat_id"], json!(OWNER), "in the owner's chat");
    assert_eq!(
        calls[2].1["message_id"],
        json!(message),
        "the message is pinned"
    );
    assert_eq!(
        calls[2].1["disable_notification"],
        json!(true),
        "without a notification"
    );
    assert_eq!(
        transport.counts(),
        SendCounts {
            attempted: 4,
            delivered: 4
        }
    );

    // A pin that gives up leaves the message delivered; a band-up keeps its T5 over the budget.
    fake.script(
        "pinChatMessage",
        (0..SEND_ATTEMPTS).map(|_| Answer::status(502)),
    );
    let before = fake.calls().len();
    let decision = router
        .route(&celebration("trophy:two", Tier::T2, Some("band_up")))
        .await
        .expect("a decision");
    assert_eq!(decision, sent(Tier::T5), "the message arrived, unpinned");
    let methods: Vec<String> = since(before)
        .into_iter()
        .map(|(method, _)| method)
        .collect();
    let mut expected = vec!["sendDice".to_owned(), "sendMessage".to_owned()];
    expected.extend((0..SEND_ATTEMPTS).map(|_| "pinChatMessage".to_owned()));
    assert_eq!(methods, expected, "a failed pin");

    // The pinned message gives up: the message is sent anew as a line, and nothing is pinned.
    fake.script(
        "sendMessage",
        (0..SEND_ATTEMPTS).map(|_| Answer::status(502)),
    );
    let before = fake.calls().len();
    let decision = router
        .route(&celebration("trophy:three", Tier::T2, Some("band_up")))
        .await
        .expect("a decision");
    assert_eq!(decision, sent(Tier::T5), "the message still arrived");
    let methods: Vec<String> = since(before)
        .into_iter()
        .map(|(method, _)| method)
        .collect();
    let mut expected = vec!["sendDice".to_owned()];
    expected.extend((0..=SEND_ATTEMPTS).map(|_| "sendMessage".to_owned()));
    assert_eq!(methods, expected, "a failed pinned message");

    // A message id the Bot API's type cannot hold is never sent: the reaction is refused, and the
    // celebration is held for the send.
    owner_message::record(
        &db,
        i64::from(i32::MAX) + 1,
        UtcMillis::from_epoch_millis(noon.epoch_millis() - 3_600_000),
    )
    .await
    .expect("the owner's message is recorded");
    let before = fake.calls().len();
    let decision = router
        .route(&celebration("small:two", Tier::T1, None))
        .await
        .expect("a decision");
    assert_eq!(
        decision,
        Decision::Deferred {
            surface: Surface::Bot,
            hold: Hold::Send
        },
        "a reaction that cannot be made is held"
    );
    assert_eq!(
        since(before),
        Vec::new(),
        "no request names another message"
    );
    println!("examined: 6 renders");
}
