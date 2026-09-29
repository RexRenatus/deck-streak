//! The one router's photo and prepared share (SPEC-132 A1 to A11): a photo is routed in the
//! router's order at T2, refuses by name what it cannot do, and a share is prepared without a
//! delivery. Each case runs over a scripted transport on a manual clock.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use deck_streak_kernel::StudyDayRule;
use deck_streak_notifications::{
    BotTransport, FileId, NotNow, Pass, Photo, PhotoDecision, PhotoError, PhotoFuture, PhotoPushed,
    Prepared, PushFuture, Pushed, Reason, Router, ShareFuture, Surface, Tier,
};
use support::{DAY, Harness, at};

/// The file id the scripted transport answers a delivered photo with.
const FILE: &str = "synthetic-file-id";

/// A transport with every call of the port, answering what it was told and recording what it saw.
struct Photographer {
    photo: Mutex<PhotoPushed>,
    share: Mutex<Prepared>,
    photos: Mutex<Vec<(usize, String)>>,
    shares: Mutex<Vec<(String, String)>>,
    lines: AtomicUsize,
}

impl Photographer {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            photo: Mutex::new(PhotoPushed::Delivered {
                file_id: FileId::new(FILE).expect("a file id"),
            }),
            share: Mutex::new(Prepared::Ready {
                id: "synthetic-prepared".to_owned(),
            }),
            photos: Mutex::default(),
            shares: Mutex::default(),
            lines: AtomicUsize::new(0),
        })
    }

    fn photos(&self) -> Vec<(usize, String)> {
        self.photos
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn shares(&self) -> Vec<(String, String)> {
        self.shares
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn answer_photo(&self, answer: PhotoPushed) {
        *self.photo.lock().unwrap_or_else(PoisonError::into_inner) = answer;
    }

    fn answer_share(&self, answer: Prepared) {
        *self.share.lock().unwrap_or_else(PoisonError::into_inner) = answer;
    }
}

impl BotTransport for Photographer {
    fn push_message<'a>(&'a self, _pass: &'a Pass, _text: &'a str) -> PushFuture<'a> {
        self.lines.fetch_add(1, Ordering::SeqCst);
        Box::pin(std::future::ready(Pushed::Delivered))
    }

    fn push_pin<'a>(&'a self, _pass: &'a Pass, _text: &'a str) -> PushFuture<'a> {
        self.lines.fetch_add(1, Ordering::SeqCst);
        Box::pin(std::future::ready(Pushed::Delivered))
    }

    fn push_dice<'a>(&'a self, _pass: &'a Pass, _emoji: &'a str) -> PushFuture<'a> {
        self.lines.fetch_add(1, Ordering::SeqCst);
        Box::pin(std::future::ready(Pushed::Delivered))
    }

    fn push_photo<'a>(
        &'a self,
        _pass: &'a Pass,
        photo: &'a Photo,
        caption: &'a str,
    ) -> PhotoFuture<'a> {
        self.photos
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((photo.bytes().len(), caption.to_owned()));
        let answer = self
            .photo
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        Box::pin(std::future::ready(answer))
    }

    fn prepare_share<'a>(
        &'a self,
        _pass: &'a Pass,
        file: &'a FileId,
        caption: &'a str,
    ) -> ShareFuture<'a> {
        self.shares
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((file.as_str().to_owned(), caption.to_owned()));
        let answer = self
            .share
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        Box::pin(std::future::ready(answer))
    }
}

/// A synthetic PNG of `width` by `height` pixels, padded with zeros to `length` bytes: its
/// signature and header are real, and its pixels are not.
fn png(width: u32, height: u32, length: usize) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    bytes.extend_from_slice(&13_u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
    bytes.resize(length.max(bytes.len()), 0);
    bytes
}

/// A small valid photo captioned `caption`.
fn photo(caption: &str) -> Photo {
    Photo::new(png(64, 48, 128), caption).expect("a photo within every bound")
}

/// The harness of the bot `bot`, joined to a router at noon of `DAY`.
async fn harness_with(bot: &Arc<Photographer>) -> Harness {
    let mut harness = Harness::without_bot(at(DAY, 12, 0)).await;
    harness.router = Router::new(
        Arc::clone(&harness.policy),
        harness.db.clone(),
        harness.clock.clone(),
        StudyDayRule::default(),
    )
    .with_bot(Arc::clone(bot) as Arc<dyn BotTransport>);
    harness
}

#[tokio::test]
async fn a_transport_without_push_photo_answers_unsupported() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let occasion = harness.celebration("card-1", Surface::Bot);

    let decision = harness
        .router
        .route_photo(&occasion, &photo("synthetic caption"))
        .await
        .expect("the photo is routed");

    assert_eq!(
        decision,
        PhotoDecision::Withheld {
            reason: Reason::PhotoUnsupported
        }
    );
    assert!(
        harness.bot.pushes().is_empty(),
        "an unsupported photo makes no call"
    );
}

#[tokio::test]
async fn a_transport_without_prepare_share_answers_unsupported() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let file = FileId::new(FILE).expect("a file id");

    let prepared = harness
        .router
        .prepare_share(&file, "synthetic caption")
        .await;

    assert_eq!(prepared, Prepared::Unsupported);
    assert!(
        harness.bot.pushes().is_empty(),
        "an unsupported share makes no call"
    );

    let bare = Harness::without_bot(at(DAY, 12, 0)).await;
    assert_eq!(
        bare.router.prepare_share(&file, "synthetic caption").await,
        Prepared::Unsupported,
        "no transport joined is unsupported too"
    );
}

#[tokio::test]
async fn an_unsupported_photo_is_withheld_by_name_and_sends_nothing() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let occasion = harness.celebration("card-3", Surface::Bot);

    let decision = harness
        .router
        .route_photo(&occasion, &photo("synthetic caption"))
        .await
        .expect("the photo is routed");

    assert_eq!(
        decision,
        PhotoDecision::Withheld {
            reason: Reason::PhotoUnsupported
        }
    );
    let recorded = harness.last_decision().await;
    assert_eq!(recorded.arm, "withhold");
    assert_eq!(recorded.reason.as_deref(), Some("photo_unsupported"));
    assert_eq!(recorded.kind, "celebration:withheld");
    assert_eq!(harness.decisions().await.len(), 1);
    assert!(harness.bot.pushes().is_empty(), "nothing else is sent");
    assert_eq!(harness.deliveries().await, 0);
    assert!(harness.feed().await.is_empty());
    assert!(harness.queue().await.is_empty());
}

#[tokio::test]
async fn a_photo_in_quiet_hours_is_not_held() {
    let bot = Photographer::new();
    let harness = harness_with(&bot).await;
    harness.clock.set(at(DAY, 23, 30));
    let occasion = harness.celebration("card-4", Surface::Bot);

    let decision = harness
        .router
        .route_photo(&occasion, &photo("synthetic caption"))
        .await
        .expect("the photo is routed");

    assert_eq!(
        decision,
        PhotoDecision::NotNow {
            reason: NotNow::QuietHours
        }
    );
    assert!(bot.photos().is_empty(), "nothing is sent in quiet hours");
    assert!(harness.queue().await.is_empty(), "nothing is held");
    assert!(harness.decisions().await.is_empty(), "nothing is recorded");
    assert_eq!(harness.deliveries().await, 0);

    harness.clock.set(at(DAY + 1, 9, 0));
    let again = harness
        .router
        .route_photo(&occasion, &photo("synthetic caption"))
        .await
        .expect("the photo is routed again");
    assert!(
        matches!(again, PhotoDecision::Sent { .. }),
        "the key was never claimed, so the caller's retry after the window sends"
    );
}

#[tokio::test]
async fn a_failed_photo_records_nothing() {
    let bot = Photographer::new();
    bot.answer_photo(PhotoPushed::Failed);
    let harness = harness_with(&bot).await;
    let occasion = harness.celebration("card-5", Surface::Bot);

    let decision = harness
        .router
        .route_photo(&occasion, &photo("synthetic caption"))
        .await
        .expect("the photo is routed");

    assert_eq!(
        decision,
        PhotoDecision::NotNow {
            reason: NotNow::SendFailed
        }
    );
    assert_eq!(bot.photos().len(), 1, "the call was made once");
    assert!(harness.decisions().await.is_empty(), "nothing is recorded");
    assert!(harness.queue().await.is_empty(), "nothing is held");
    assert_eq!(harness.deliveries().await, 0);

    bot.answer_photo(PhotoPushed::Delivered {
        file_id: FileId::new(FILE).expect("a file id"),
    });
    // The policy's outage cooldown (`send_failure.outage_cooldown_ms`).
    let cooldown = 60_000_u64;
    harness.clock.advance(Duration::from_millis(cooldown + 1));
    let again = harness
        .router
        .route_photo(&occasion, &photo("synthetic caption"))
        .await
        .expect("the photo is routed again");
    assert!(
        matches!(again, PhotoDecision::Sent { .. }),
        "the failed photo released its claim, so a later raise is not already_recorded"
    );
}

#[tokio::test]
async fn a_failed_photo_opens_the_breaker_for_the_next_photo() {
    let bot = Photographer::new();
    bot.answer_photo(PhotoPushed::Failed);
    let harness = harness_with(&bot).await;
    let failed = harness.celebration("card-5b", Surface::Bot);
    let next = harness.celebration("card-5c", Surface::Bot);

    let first = harness
        .router
        .route_photo(&failed, &photo("synthetic caption"))
        .await
        .expect("the photo is routed");
    assert_eq!(
        first,
        PhotoDecision::NotNow {
            reason: NotNow::SendFailed
        }
    );
    assert_eq!(bot.photos().len(), 1, "the failed call was made once");

    // The bot would now deliver, and the clock is inside the outage cooldown
    // (`send_failure.outage_cooldown_ms`, 60,000 ms): the open breaker refuses before any call.
    bot.answer_photo(PhotoPushed::Delivered {
        file_id: FileId::new(FILE).expect("a file id"),
    });
    harness.clock.advance(Duration::from_millis(1_000));
    let second = harness
        .router
        .route_photo(&next, &photo("synthetic caption"))
        .await
        .expect("the photo is routed again");

    assert_eq!(
        second,
        PhotoDecision::NotNow {
            reason: NotNow::SendFailed
        },
        "an open breaker answers send_failed, not quiet_hours and not a send"
    );
    assert_eq!(
        bot.photos().len(),
        1,
        "no second push_photo inside the cooldown"
    );
    assert!(harness.decisions().await.is_empty(), "nothing is recorded");
    assert!(harness.queue().await.is_empty(), "nothing is held");
}

#[tokio::test]
async fn a_delivered_photo_is_recorded_once() {
    let bot = Photographer::new();
    let harness = harness_with(&bot).await;
    let occasion = harness.celebration("card-6", Surface::Bot);

    let first = harness
        .router
        .route_photo(&occasion, &photo("synthetic caption"))
        .await
        .expect("the photo is routed");
    assert_eq!(
        first,
        PhotoDecision::Sent {
            file_id: FileId::new(FILE).expect("a file id")
        }
    );
    assert_eq!(bot.photos(), vec![(128, "synthetic caption".to_owned())]);
    let recorded = harness.last_decision().await;
    assert_eq!(
        (recorded.arm.as_str(), recorded.key.as_str()),
        ("send", "card-6")
    );
    assert_eq!(recorded.surface, "bot");
    assert_eq!(harness.deliveries().await, 1);

    let second = harness
        .router
        .route_photo(&occasion, &photo("synthetic caption"))
        .await
        .expect("the photo is routed again");
    assert_eq!(
        second,
        PhotoDecision::Withheld {
            reason: Reason::AlreadyRecorded
        }
    );
    assert_eq!(bot.photos().len(), 1, "the second raise sends nothing");
    assert_eq!(harness.deliveries().await, 1);
    assert_eq!(harness.decisions().await.len(), 2);
}

#[tokio::test]
async fn a_photo_spends_no_celebration_budget() {
    let bot = Photographer::new();
    let harness = harness_with(&bot).await;
    let art = harness
        .celebration("card-7", Surface::Bot)
        .with_event("ceremony", None);

    let decision = harness
        .router
        .route_photo(&art, &photo("synthetic caption"))
        .await
        .expect("the photo is routed");
    assert!(matches!(decision, PhotoDecision::Sent { .. }));
    let recorded = harness.last_decision().await;
    assert_eq!(recorded.requested, Tier::T2.as_str(), "it asks T2");
    assert_eq!(recorded.rendered, Tier::T2.as_str(), "it renders T2");

    let ceremony = harness
        .celebration("ceremony-7", Surface::Bot)
        .with_event("ceremony", None);
    let next = harness
        .router
        .route(&ceremony)
        .await
        .expect("the ceremony is routed");
    assert_eq!(
        next,
        deck_streak_notifications::Decision::Sent {
            surface: Surface::Bot,
            tier: Tier::T5
        },
        "the week's one T5 is still unspent"
    );
}

#[tokio::test]
async fn a_photo_with_its_kind_off_is_never_pushed() {
    let bot = Photographer::new();
    let harness = harness_with(&bot).await;
    harness.set("celebrations_enabled", "0").await;
    let occasion = harness.celebration("card-8", Surface::Bot);

    let decision = harness
        .router
        .route_photo(&occasion, &photo("synthetic caption"))
        .await
        .expect("the photo is routed");

    assert_eq!(
        decision,
        PhotoDecision::Withheld {
            reason: Reason::NudgesDisabled
        }
    );
    assert!(
        bot.photos().is_empty(),
        "a switched-off kind is never pushed"
    );
    let recorded = harness.last_decision().await;
    assert_eq!(recorded.reason.as_deref(), Some("nudges_disabled"));
}

#[test]
fn a_caption_is_counted_in_utf16_units() {
    // Telegram counts a caption in UTF-16 units after entity parsing: a character outside the
    // Basic Multilingual Plane counts two, so 513 of them are 1,026 units.
    let emoji = |count: usize| "\u{1F600}".repeat(count);
    assert!(
        Photo::new(png(64, 48, 64), emoji(512)).is_ok(),
        "512 emoji are 1,024 units"
    );
    assert_eq!(
        Photo::new(png(64, 48, 64), emoji(513)).err(),
        Some(PhotoError::Caption),
        "513 emoji are 1,026 units"
    );
    let combining = "e\u{301}".repeat(512);
    assert!(
        Photo::new(png(64, 48, 64), combining.clone()).is_ok(),
        "a combining sequence counts each of its units"
    );
    assert_eq!(
        Photo::new(png(64, 48, 64), format!("{combining}e")).err(),
        Some(PhotoError::Caption)
    );
}

#[test]
fn an_oversized_photo_is_refused_before_any_call() {
    let refused = |bytes: Vec<u8>, caption: &str| Photo::new(bytes, caption).err();

    assert_eq!(
        refused(png(64, 48, 10_000_001), ""),
        Some(PhotoError::TooLarge)
    );
    assert!(Photo::new(png(64, 48, 10_000_000), "").is_ok());
    assert_eq!(
        refused(png(5_000, 5_001, 64), ""),
        Some(PhotoError::Dimensions)
    );
    assert!(Photo::new(png(5_000, 5_000, 64), "").is_ok());
    assert_eq!(refused(png(2_100, 100, 64), ""), Some(PhotoError::Ratio));
    assert!(Photo::new(png(2_000, 100, 64), "").is_ok());
    assert_eq!(refused(png(100, 2_100, 64), ""), Some(PhotoError::Ratio));
    assert_eq!(
        refused(png(64, 48, 64), &"x".repeat(1_025)),
        Some(PhotoError::Caption)
    );
    assert!(Photo::new(png(64, 48, 64), "x".repeat(1_024)).is_ok());
    assert_eq!(refused(png(0, 48, 64), ""), Some(PhotoError::Dimensions));
    assert_eq!(
        refused(b"not an image".to_vec(), ""),
        Some(PhotoError::Format)
    );
    assert_eq!(PhotoError::TooLarge.code(), "photo_invalid");
    assert_eq!(FileId::new("").err(), Some(PhotoError::FileId));
}

#[tokio::test]
async fn a_prepared_share_records_no_delivery() {
    let bot = Photographer::new();
    let harness = harness_with(&bot).await;
    let file = FileId::new(FILE).expect("a file id");

    let prepared = harness
        .router
        .prepare_share(&file, "synthetic caption")
        .await;

    assert_eq!(
        prepared,
        Prepared::Ready {
            id: "synthetic-prepared".to_owned()
        }
    );
    assert_eq!(
        bot.shares(),
        vec![(FILE.to_owned(), "synthetic caption".to_owned())]
    );
    assert!(harness.decisions().await.is_empty());
    assert_eq!(harness.deliveries().await, 0);
    assert!(harness.feed().await.is_empty());
    assert!(harness.queue().await.is_empty());

    bot.answer_share(Prepared::Failed);
    assert_eq!(
        harness
            .router
            .prepare_share(&file, "synthetic caption")
            .await,
        Prepared::Failed,
        "the transport's outcome is the answer"
    );
    assert!(harness.decisions().await.is_empty());
}

#[test]
fn every_reason_is_named() {
    assert!(Reason::ALL.contains(&Reason::PhotoUnsupported));
    assert_eq!(Reason::PhotoUnsupported.as_str(), "photo_unsupported");
    assert_eq!(Reason::ALL.len(), 7);
}
