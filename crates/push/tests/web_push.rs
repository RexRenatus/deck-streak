//! SPEC-343 A11 to A17: the web push sender against the recording push service fake. Each body is
//! decrypted by the fake's own RFC 8291 code and each VAPID token verified with the `k` it carried;
//! every key is generated in memory, every value is synthetic, and the clock is the kernel's manual
//! one.

#![allow(
    clippy::expect_used,
    reason = "a helper builds what every test needs and panics like a test, but clippy's \
              allow-expect-in-tests reaches only #[test] functions"
)]

mod support;

use std::time::Duration;

use base64ct::{Base64UrlUnpadded, Encoding};
use deck_streak_push::{
    BuildError, CollapseKey, Notification, Origin, PushServices, Refusal, Sent, Unreached,
    WebPushSender, WebPushSettings,
};
use serde_json::json;

use support::fake_push_service::FakePushService;
use support::keys::{CONTACT, ENDPOINT_PATH, TestKey, TestSubscriber};
use support::rfc8291::Undecrypted;
use support::{Answer, DEADLINE, Recorded, START, clock};

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
    Notification::new("Synthetic title", "Synthetic body", Duration::from_mins(1))
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

#[tokio::test]
async fn a12_the_vapid_header_carries_a_jwt_the_fake_verifies_with_k() {
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
    let vapid = FakePushService::vapid(&received[0]).expect("vapid t=..., k=...");
    let public = key.verifying().to_encoded_point(false);
    assert_eq!(
        vapid.key,
        Base64UrlUnpadded::encode_string(public.as_bytes())
    );
    let verified =
        FakePushService::token(&received[0]).expect("the fake verifies the token with k");
    assert_eq!(verified.header, json!({"typ": "JWT", "alg": "ES256"}));
    assert_eq!(
        verified.claims,
        json!({
            "aud": service.fake.origin(),
            "exp": START / 1000 + 12 * 60 * 60,
            "sub": CONTACT,
        })
    );
}

#[tokio::test]
async fn a13_ttl_urgency_and_topic_carry_the_expiry_and_the_collapse_key() {
    let service = FakePushService::start().await;
    let key = TestKey::generate();
    let browser = TestSubscriber::generate();
    let sender = build_sender(&key, listing(&[&service]));
    let subscription = sender
        .subscription(&service.endpoint(), &browser.p256dh(), &browser.auth())
        .expect("a listed endpoint is admitted");
    let keyed = notification().with_collapse_key(CollapseKey::new("streak-day").expect("a key"));
    let at_once = Notification::new("Synthetic title", "Synthetic body", Duration::ZERO);

    assert_eq!(sender.deliver(&subscription, &keyed).await, Sent::Delivered);
    assert_eq!(
        sender.deliver(&subscription, &at_once).await,
        Sent::Delivered
    );

    let received = service.fake.received();
    assert_eq!(received.len(), 2);
    let headers = |request: &Recorded| {
        (
            request.header("ttl").map(str::to_owned),
            request.header("urgency").map(str::to_owned),
            request.header("topic").map(str::to_owned),
        )
    };
    assert_eq!(
        headers(&received[0]),
        (
            Some("60".to_owned()),
            Some("normal".to_owned()),
            Some("streak-day".to_owned())
        )
    );
    assert_eq!(
        headers(&received[1]),
        (Some("0".to_owned()), Some("normal".to_owned()), None)
    );
}

#[tokio::test]
async fn a14_a_404_or_410_reports_the_subscription_gone() {
    let service = FakePushService::start().await;
    let key = TestKey::generate();
    let browser = TestSubscriber::generate();
    let sender = build_sender(&key, listing(&[&service]));
    let subscription = sender
        .subscription(&service.endpoint(), &browser.p256dh(), &browser.auth())
        .expect("a listed endpoint is admitted");
    service
        .fake
        .script([Answer::status(404), Answer::status(410)]);

    let first = sender.deliver(&subscription, &notification()).await;
    let second = sender.deliver(&subscription, &notification()).await;

    assert_eq!(
        (first, second),
        (Sent::Gone { since: None }, Sent::Gone { since: None })
    );
    assert_eq!(service.fake.received().len(), 2, "one request each");
}

#[tokio::test]
async fn a15_try_later_answers_carry_retry_after_and_are_not_retried() {
    let service = FakePushService::start().await;
    let key = TestKey::generate();
    let browser = TestSubscriber::generate();
    let sender = build_sender(&key, listing(&[&service]));
    let subscription = sender
        .subscription(&service.endpoint(), &browser.p256dh(), &browser.auth())
        .expect("a listed endpoint is admitted");
    service.fake.script([
        Answer::status(429).header("retry-after", "120"),
        Answer::status(503),
    ]);

    let limited = sender.deliver(&subscription, &notification()).await;
    let unavailable = sender.deliver(&subscription, &notification()).await;

    assert_eq!(
        limited,
        Sent::RetryLater {
            after: Some(Duration::from_mins(2))
        }
    );
    assert_eq!(unavailable, Sent::RetryLater { after: None });
    assert_eq!(service.fake.received().len(), 2, "one request each");
}

#[tokio::test]
async fn a17_an_oversize_plaintext_is_refused_before_any_request() {
    let service = FakePushService::start().await;
    let key = TestKey::generate();
    let browser = TestSubscriber::generate();
    let sender = build_sender(&key, listing(&[&service]));
    let subscription = sender
        .subscription(&service.endpoint(), &browser.p256dh(), &browser.auth())
        .expect("a listed endpoint is admitted");
    // The plaintext is `{"title":T,"body":B}`; the test measures its own frame.
    let frame = json!({"title": "Synthetic title", "body": ""})
        .to_string()
        .len();
    let sized = |plaintext: usize| {
        Notification::new(
            "Synthetic title",
            &"x".repeat(plaintext - frame),
            Duration::from_mins(1),
        )
    };

    assert_eq!(
        sender.deliver(&subscription, &sized(3993)).await,
        Sent::Delivered
    );
    assert_eq!(
        sender.deliver(&subscription, &sized(3994)).await,
        Sent::Rejected(Refusal::TooLarge)
    );

    let received = service.fake.received();
    assert_eq!(received.len(), 1, "the oversize message made no request");
    let delivered = FakePushService::plaintext(&received[0], &browser).expect("it decrypts");
    assert_eq!(delivered.to_string().len(), 3993);
}

// MUTATION COVERAGE: green at the base where the behaviour already stood, each holds a branch the
// criteria above leave unexamined.

#[tokio::test]
async fn every_other_answer_is_read_by_its_status_after_one_request() {
    let service = FakePushService::start().await;
    let key = TestKey::generate();
    let browser = TestSubscriber::generate();
    let sender = build_sender(&key, listing(&[&service]));
    let subscription = sender
        .subscription(&service.endpoint(), &browser.p256dh(), &browser.auth())
        .expect("a listed endpoint is admitted");
    let table = [
        (413, Sent::Rejected(Refusal::TooLarge)),
        (401, Sent::Rejected(Refusal::ProviderToken)),
        (403, Sent::Rejected(Refusal::ProviderToken)),
        (400, Sent::Rejected(Refusal::Request)),
        (405, Sent::Rejected(Refusal::Request)),
        (301, Sent::Failed(Unreached::Redirect)),
        (307, Sent::Failed(Unreached::Redirect)),
        (202, Sent::Delivered),
    ];
    service
        .fake
        .script(table.iter().map(|(status, _)| Answer::status(*status)));

    let mut read = Vec::new();
    for _ in &table {
        read.push(sender.deliver(&subscription, &notification()).await);
    }

    let expected: Vec<Sent> = table.iter().map(|(_, sent)| *sent).collect();
    assert_eq!(read, expected);
    assert_eq!(
        service.fake.received().len(),
        table.len(),
        "one request each"
    );
}

#[test]
fn a_collapse_key_past_32_characters_is_refused() {
    let longest = "a".repeat(32);
    assert_eq!(
        CollapseKey::new(&longest).map(|key| key.as_str().to_owned()),
        Ok(longest)
    );
    assert_eq!(
        CollapseKey::new("Synthetic-key_09").map(|key| key.as_str().to_owned()),
        Ok("Synthetic-key_09".to_owned())
    );
    for refused in [
        "a".repeat(33),
        String::new(),
        "a.b".to_owned(),
        "a b".to_owned(),
    ] {
        assert_eq!(
            CollapseKey::new(&refused),
            Err(BuildError::CollapseKey),
            "{refused:?} is refused"
        );
    }
}

#[tokio::test]
async fn a_subscription_that_does_not_parse_is_refused() {
    let service = FakePushService::start().await;
    let key = TestKey::generate();
    let browser = TestSubscriber::generate();
    let sender = build_sender(&key, listing(&[&service]));
    let endpoint = service.endpoint();
    let (p256dh, auth) = (browser.p256dh(), browser.auth());
    let not_a_point = Base64UrlUnpadded::encode_string(&[0_u8; 65]);
    let short_auth = Base64UrlUnpadded::encode_string(&[7_u8; 15]);
    let long_auth = Base64UrlUnpadded::encode_string(&[7_u8; 17]);

    let refused = [
        ("not a uri", p256dh.as_str(), auth.as_str()),
        (ENDPOINT_PATH, p256dh.as_str(), auth.as_str()),
        (endpoint.as_str(), "!!!", auth.as_str()),
        (endpoint.as_str(), not_a_point.as_str(), auth.as_str()),
        (endpoint.as_str(), p256dh.as_str(), "!!!"),
        (endpoint.as_str(), p256dh.as_str(), short_auth.as_str()),
        (endpoint.as_str(), p256dh.as_str(), long_auth.as_str()),
    ];
    for (endpoint, p256dh, auth) in refused {
        assert_eq!(
            sender.subscription(endpoint, p256dh, auth).err(),
            Some(BuildError::Subscription),
            "{endpoint:?} {p256dh:?} {auth:?} is refused"
        );
    }
    assert_eq!(
        sender
            .subscription("http://push.synthetic.invalid/endpoint", &p256dh, &auth)
            .err(),
        Some(BuildError::Origin)
    );
    assert!(
        sender.subscription(&endpoint, &p256dh, &auth).is_ok(),
        "the well-formed control is admitted"
    );
    assert!(service.fake.received().is_empty(), "no parse sends");
}
