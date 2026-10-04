//! SPEC-343 A11 to A17: the web push sender against the recording push service fake. Each body is
//! decrypted by the fake's own RFC 8291 code and each VAPID token verified with the `k` it carried;
//! every key is generated in memory, every value is synthetic, and the clock is the kernel's manual
//! one.

mod support;

use std::time::Duration;

use deck_streak_push::{
    BuildError, Notification, Origin, PushServices, Sent, WebPushSender, WebPushSettings,
};
use serde_json::json;

use support::fake_push_service::FakePushService;
use support::keys::{CONTACT, ENDPOINT_PATH, TestKey, TestSubscriber};
use support::rfc8291::Undecrypted;
use support::{DEADLINE, clock};

/// A sender whose list holds `services`, signing with `key`.
fn build_sender(key: &TestKey, services: PushServices) -> WebPushSender {
    WebPushSender::new(
        WebPushSettings {
            key_pem: &key.pem,
            contact: CONTACT,
            services,
            deadline: DEADLINE,
        },
        clock(),
    )
    .expect("the sender builds")
}

/// The list holding exactly `fakes`' origins.
fn listing(fakes: &[&FakePushService]) -> PushServices {
    PushServices::new(
        fakes
            .iter()
            .map(|fake| Origin::new(fake.fake.origin()).expect("a loopback origin")),
    )
}

fn notification() -> Notification {
    Notification::new("Synthetic title", "Synthetic body", Duration::from_secs(60))
}

#[tokio::test]
async fn a16_an_endpoint_off_the_list_is_refused_before_any_request() {
    let listed = FakePushService::start().await;
    let unlisted = FakePushService::start().await;
    let key = TestKey::generate();
    let browser = TestSubscriber::generate();
    let sender = build_sender(&key, listing(&[&listed]));

    // The behaviour first: an endpoint whose origin the list does not hold is refused, by name.
    let refused = sender.subscription(&unlisted.endpoint(), &browser.p256dh(), &browser.auth());
    assert_eq!(refused.err(), Some(BuildError::OffTheList));

    // A sender built with no list refuses every endpoint: the check fails closed (ADR-354 D7).
    let unlisting = build_sender(&key, PushServices::default());
    let closed = unlisting.subscription(&listed.endpoint(), &browser.p256dh(), &browser.auth());
    assert_eq!(closed.err(), Some(BuildError::OffTheList));

    // The listed endpoint is admitted and its message delivered, in one request to its own fake.
    let admitted = sender
        .subscription(&listed.endpoint(), &browser.p256dh(), &browser.auth())
        .expect("a listed endpoint is admitted");
    let sent = sender.deliver(&admitted, &notification()).await;
    assert_eq!(sent, Sent::Delivered);
    assert_eq!(listed.fake.received().len(), 1);
    assert_eq!(unlisted.fake.received().len(), 0);
}

#[tokio::test]
async fn a11_a_message_reaches_the_fake_encrypted_and_decrypts_to_its_json() {
    let service = FakePushService::start().await;
    let key = TestKey::generate();
    let browser = TestSubscriber::generate();
    let sender = build_sender(&key, listing(&[&service]));
    let subscription = sender
        .subscription(&service.endpoint(), &browser.p256dh(), &browser.auth())
        .expect("a listed endpoint is admitted");

    assert_eq!(
        sender.deliver(&subscription, &notification()).await,
        Sent::Delivered
    );

    let received = service.fake.received();
    assert_eq!(received.len(), 1);
    let request = &received[0];
    assert_eq!(
        FakePushService::plaintext(request, &browser),
        Ok(json!({"title": "Synthetic title", "body": "Synthetic body"}))
    );
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, ENDPOINT_PATH);
    assert_eq!(request.header("content-encoding"), Some("aes128gcm"));
    assert_eq!(
        request.header("content-type"),
        Some("application/octet-stream")
    );

    // The fake's decryption is its own: another browser's key does not open the body.
    let stranger = TestSubscriber::generate();
    assert_eq!(
        FakePushService::plaintext(request, &stranger),
        Err(Undecrypted::Tag)
    );
}
