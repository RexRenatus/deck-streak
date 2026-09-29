//! The bot's reveal (SPEC-084 A14; R8): a T3 celebration sends a placeholder, waits the reveal's
//! pause and edits the placeholder into the message; when the edit gives up, the message is sent
//! anew, and when the placeholder gives up, the message is sent in its place.
//!
//! The router renders through the bot's own transport to a fake Bot API, so each tier's calls are
//! read as Telegram would receive them; the pause is a noted wait that returns at once.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use std::sync::Arc;

use deck_streak_bot::OwnerChat;
use deck_streak_bot::transport::SEND_ATTEMPTS;
use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_notifications::ladder::{REVEAL_PAUSE, REVEAL_PLACEHOLDER};
use deck_streak_notifications::{
    Decision, DedupeKey, LapseContext, Occasion, Policy, Router, Surface, Tier,
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
