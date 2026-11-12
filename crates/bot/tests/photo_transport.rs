//! The bot's photo and prepared share (SPEC-132 A12 to A14): each is one call of the Bot API,
//! made through the one router, and neither is retried. The router renders through the bot's own
//! transport to a fake Bot API, so each call is read as Telegram would receive it.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use std::sync::Arc;

use deck_streak_bot::OwnerChat;
use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_notifications::{
    DedupeKey, FileId, LapseContext, NotNow, Occasion, Photo, PhotoDecision, Policy, Prepared,
    Router, Surface, Tier,
};
use fake_bot_api::{Answer, FakeBotApi, OWNER};
use serde_json::Value;

/// A synthetic PNG header of 64 by 48 pixels, padded to 128 bytes.
fn png() -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    bytes.extend_from_slice(&13_u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&64_u32.to_be_bytes());
    bytes.extend_from_slice(&48_u32.to_be_bytes());
    bytes.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
    bytes.resize(128, 0);
    bytes
}

/// The router over the fake Bot API, and the occasion the photos are raised for.
async fn bench(fake: &FakeBotApi) -> (Router, Occasion) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = Arc::new(fake.transport(directory.path()));
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let noon = UtcMillis::from_epoch_millis(20_000 * 86_400_000 + 12 * 3_600_000);
    let occasion = Occasion::new(
        policy.kind("celebration").expect("the celebration kind"),
        DedupeKey::new("card-1").expect("a key"),
        Surface::Bot,
        Tier::T2,
        "synthetic card",
        StudyDay::from_epoch_day(20_000),
        LapseContext::NoLapse,
    )
    .expect("an occasion");
    let router = Router::new(
        policy,
        db,
        Arc::new(ManualClock::new(noon)),
        StudyDayRule::default(),
    )
    .with_bot(Arc::new(OwnerChat::new(transport, fake_bot_api::owner())));
    // The directory lives as long as the test: leak it to the process, a temp dir of one run.
    std::mem::forget(directory);
    (router, occasion)
}

#[tokio::test]
async fn push_photo_is_one_send_photo_to_the_owner() {
    let fake = FakeBotApi::start().await;
    let (router, occasion) = bench(&fake).await;
    let photo = Photo::new(png(), "<b>Card</b> synthetic").expect("a photo");

    let decision = router
        .route_photo(&occasion, &photo)
        .await
        .expect("the photo is routed");

    assert_eq!(
        decision,
        PhotoDecision::Sent {
            file_id: FileId::new("size-large").expect("a file id")
        },
        "the file id is the largest size's, not the first or the last"
    );
    let calls = fake.calls_of("sendPhoto");
    assert_eq!(calls.len(), 1, "one sendPhoto");
    assert_eq!(fake.calls().len(), 1, "and no other call");
    let body = &calls[0].body;
    assert_eq!(body["chat_id"], Value::String(OWNER.to_string()));
    assert_eq!(body["caption"], "<b>Card</b> synthetic");
    assert_eq!(body["parse_mode"], "HTML");
    assert_eq!(body["photo"]["content_type"], "image/png");
    assert!(calls[0].token_ok);
}

#[tokio::test]
async fn prepare_share_is_one_prepared_inline_message() {
    let fake = FakeBotApi::start().await;
    let (router, _occasion) = bench(&fake).await;
    let file = FileId::new("size-large").expect("a file id");

    let prepared = router.prepare_share(&file, "<b>Card</b> synthetic").await;

    assert_eq!(
        prepared,
        Prepared::Ready {
            id: "prepared-1".to_owned()
        }
    );
    let calls = fake.calls_of("savePreparedInlineMessage");
    assert_eq!(calls.len(), 1, "one savePreparedInlineMessage");
    assert_eq!(fake.calls().len(), 1, "and no other call");
    let body = &calls[0].body;
    assert_eq!(body["user_id"], OWNER);
    assert_eq!(body["result"]["type"], "photo");
    assert_eq!(body["result"]["photo_file_id"], "size-large");
    assert_eq!(body["result"]["caption"], "<b>Card</b> synthetic");
    assert_eq!(body["result"]["parse_mode"], "HTML");
    assert_eq!(body["allow_user_chats"], true);
    assert_eq!(body["allow_bot_chats"], true);
    assert_eq!(body["allow_group_chats"], true);
    assert_eq!(body["allow_channel_chats"], true);
}

#[tokio::test]
async fn a_refused_photo_or_share_answers_failed() {
    for scripted in [
        Answer::Refused {
            status: 400,
            description: "Bad Request: wrong file",
            retry_after: None,
        },
        Answer::too_many(1),
        Answer::Garbage(502),
        Answer::Silence,
    ] {
        let fake = FakeBotApi::start().await;
        let (router, occasion) = bench(&fake).await;
        fake.script("sendPhoto", [scripted]);
        let photo = Photo::new(png(), "synthetic").expect("a photo");
        assert_eq!(
            router
                .route_photo(&occasion, &photo)
                .await
                .expect("the photo is routed"),
            PhotoDecision::NotNow {
                reason: NotNow::SendFailed
            }
        );
        assert_eq!(
            fake.calls_of("sendPhoto").len(),
            1,
            "a photo is never retried"
        );
    }
    for scripted in [
        Answer::Refused {
            status: 400,
            description: "Bad Request: wrong file",
            retry_after: None,
        },
        Answer::too_many(1),
        Answer::Garbage(502),
        Answer::Silence,
    ] {
        let fake = FakeBotApi::start().await;
        let (router, _occasion) = bench(&fake).await;
        fake.script("savePreparedInlineMessage", [scripted]);
        let file = FileId::new("size-large").expect("a file id");
        assert_eq!(
            router.prepare_share(&file, "synthetic").await,
            Prepared::Failed
        );
        assert_eq!(
            fake.calls_of("savePreparedInlineMessage").len(),
            1,
            "a share is never retried"
        );
    }
}
