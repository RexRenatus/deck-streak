//! SPEC-343 A1 to A9: the APNs sender against two recording APNs fakes, one per environment. Each
//! provider token is verified by the fake with the test's public key; every key is generated in
//! memory, every value is synthetic, and time moves on the kernel's manual clock, never by sleeping.

mod support;

use std::sync::Arc;
use std::time::Duration;

use axum::http::Version;
use deck_streak_kernel::{ManualClock, UtcMillis};
use deck_streak_push::{
    ApnsSender, ApnsSettings, CollapseKey, Device, Environment, Notification, Origin, Refusal, Sent,
};
use serde_json::json;
use tokio::sync::Notify;

use support::fake_apns::{FakeApns, gone, refusal};
use support::keys::{KEY_ID, TEAM_ID, TOPIC, TestKey, device_token};
use support::{DEADLINE, Recorded, START, clock, verify_jwt};

/// A sender wired to a development fake and a production fake, with its key and its clock.
struct Rig {
    development: FakeApns,
    production: FakeApns,
    clock: Arc<ManualClock>,
    sender: ApnsSender,
}

impl Rig {
    async fn start() -> Self {
        Self::with_deadline(DEADLINE).await
    }

    async fn with_deadline(deadline: Duration) -> Self {
        let key = TestKey::generate();
        let development = FakeApns::start(key.verifying()).await;
        let production = FakeApns::start(key.verifying()).await;
        let clock = clock();
        let sender = ApnsSender::new(
            ApnsSettings {
                key_pem: &key.pem,
                key_id: KEY_ID,
                team_id: TEAM_ID,
                topic: TOPIC,
                development: Origin::new(development.fake.origin()).expect("a loopback origin"),
                production: Origin::new(production.fake.origin()).expect("a loopback origin"),
                deadline,
            },
            clock.clone(),
        )
        .expect("the sender builds");
        Self {
            development,
            production,
            clock,
            sender,
        }
    }

    /// Sends `notification` to the synthetic development device.
    async fn deliver(&self, notification: &Notification) -> Sent {
        self.sender.deliver(&device(), notification).await
    }

    /// Every request the development fake received.
    fn received(&self) -> Vec<Recorded> {
        self.development.fake.received()
    }
}

/// The synthetic device, registered with the development service.
fn device() -> Device {
    Device::new(&device_token(), Environment::Development).expect("a synthetic device token")
}

/// An alert that lives an hour.
fn alert() -> Notification {
    Notification::new("Synthetic title", "Synthetic body", Duration::from_secs(3600))
}

/// The clock's epoch second at the start of every test.
const START_SECOND: i64 = START / 1000;

#[tokio::test]
async fn a1_an_alert_reaches_the_fake_with_its_path_headers_and_body() {
    let rig = Rig::start().await;
    let notification =
        alert().with_collapse_key(CollapseKey::new("streak-day").expect("a collapse key"));

    assert_eq!(rig.deliver(&notification).await, Sent::Delivered);

    let received = rig.received();
    assert_eq!(received.len(), 1);
    let request = &received[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("/3/device/{}", device_token()));
    assert_eq!(request.version, Version::HTTP_2);
    assert!(
        request
            .header("authorization")
            .is_some_and(|value| value.starts_with("bearer ")),
        "a bearer provider token"
    );
    assert_eq!(request.header("apns-push-type"), Some("alert"));
    assert_eq!(request.header("apns-priority"), Some("10"));
    assert_eq!(request.header("apns-topic"), Some(TOPIC));
    let expiration = (START_SECOND + 3600).to_string();
    assert_eq!(request.header("apns-expiration"), Some(expiration.as_str()));
    assert_eq!(request.header("apns-collapse-id"), Some("streak-day"));
    assert_eq!(
        FakeApns::json(request),
        json!({"aps": {"alert": {"title": "Synthetic title", "body": "Synthetic body"}}})
    );

    // No time to live expires at once, and no collapse key sends no collapse id.
    rig.clock.advance(Duration::from_secs(7));
    let at_once = Notification::new("Synthetic title", "Synthetic body", Duration::ZERO);
    assert_eq!(rig.deliver(&at_once).await, Sent::Delivered);
    let received = rig.received();
    assert_eq!(received.len(), 2);
    assert_eq!(received[1].header("apns-expiration"), Some("0"));
    assert_eq!(received[1].header("apns-collapse-id"), None);
}

#[tokio::test]
async fn a2_the_provider_token_is_an_es256_jwt_the_fake_verifies() {
    let rig = Rig::start().await;

    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);

    let received = rig.received();
    assert_eq!(received.len(), 1);
    let verified = rig
        .development
        .token(&received[0])
        .expect("the fake verifies the token with the test's public key");
    assert_eq!(verified.header, json!({"alg": "ES256", "kid": KEY_ID}));
    assert_eq!(verified.claims, json!({"iss": TEAM_ID, "iat": START_SECOND}));

    // The fake's check is the signature's: another key does not verify the same token.
    let stranger = TestKey::generate().verifying();
    assert_eq!(verify_jwt(&FakeApns::raw_token(&received[0]), &stranger), None);
}

#[tokio::test]
async fn a3_the_provider_token_is_reused_inside_its_window_and_reminted_after() {
    let rig = Rig::start().await;

    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);
    rig.clock.advance(Duration::from_secs(19 * 60));
    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);
    rig.clock.advance(Duration::from_secs(26 * 60));
    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);

    let received = rig.received();
    assert_eq!(received.len(), 3);
    let tokens: Vec<String> = received.iter().map(FakeApns::raw_token).collect();
    assert_eq!(tokens[1], tokens[0], "19 minutes on, the token is reused");
    assert_ne!(tokens[2], tokens[0], "45 minutes on, a new token is minted");
    let minted = rig
        .development
        .token(&received[2])
        .expect("the fake verifies the new token");
    assert_eq!(minted.claims["iat"], json!(START_SECOND + 45 * 60));
}

/// The minutes a test moves its clock by, as a duration.
fn minutes(count: u64) -> Duration {
    Duration::from_secs(count * 60)
}

/// APNs's answer to a provider token it no longer accepts.
fn expired() -> support::Answer {
    refusal(403, "ExpiredProviderToken")
}

#[tokio::test]
async fn a4_an_expired_provider_token_is_reminted_once_and_resent() {
    let rig = Rig::start().await;
    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);
    rig.clock.advance(minutes(20));

    // The token minted at the start is 20 minutes old and refused: one resend with a new token.
    // That resend is refused too, and held while its token grows 20 minutes old as well, so a
    // second resend would be admitted by age alone: only the bound of one resend stops it.
    let gate = Arc::new(Notify::new());
    rig.development
        .fake
        .script([expired(), expired().held(&gate)]);
    let controller = async {
        rig.development.fake.arrived(3).await;
        rig.clock.advance(minutes(20));
        gate.notify_one();
    };
    let notification = alert();
    let (sent, ()) = tokio::join!(rig.deliver(&notification), controller);

    let received = rig.received();
    assert_eq!(received.len(), 3, "the send and its one resend, after the prime");
    assert_eq!(sent, Sent::Rejected(Refusal::ProviderToken));
    let tokens: Vec<String> = received.iter().map(FakeApns::raw_token).collect();
    assert_eq!(tokens[1], tokens[0], "the refused send carried the held token");
    assert_ne!(tokens[2], tokens[1], "the resend carried a new token");
    let reminted = rig
        .development
        .token(&received[2])
        .expect("the fake verifies the new token");
    assert_eq!(reminted.claims["iat"], json!(START_SECOND + 20 * 60));
}

#[tokio::test]
async fn a5_a_410_reports_the_device_gone_with_its_timestamp() {
    let rig = Rig::start().await;
    // Apple's timestamps: when it last knew each device's token as valid, in epoch milliseconds.
    let unregistered = START - 86_400_000;
    let expired_token = START - 3_600_000;
    rig.development.fake.script([
        gone("Unregistered", unregistered),
        gone("ExpiredToken", expired_token),
    ]);

    let first = rig.deliver(&alert()).await;
    assert_eq!(rig.received().len(), 1, "one request");
    let second = rig.deliver(&alert()).await;
    assert_eq!(rig.received().len(), 2, "one request");

    assert_eq!(
        (first, second),
        (
            Sent::Gone {
                since: Some(UtcMillis::from_epoch_millis(unregistered))
            },
            Sent::Gone {
                since: Some(UtcMillis::from_epoch_millis(expired_token))
            },
        )
    );
}

#[tokio::test]
async fn a6_a_refused_device_token_is_never_retried() {
    let rig = Rig::start().await;
    rig.development.fake.script([
        refusal(400, "BadDeviceToken"),
        refusal(400, "DeviceTokenNotForTopic"),
    ]);

    let bad = rig.deliver(&alert()).await;
    assert_eq!(rig.received().len(), 1, "one request");
    let not_for_topic = rig.deliver(&alert()).await;
    assert_eq!(rig.received().len(), 2, "one request");

    assert_eq!(
        (bad, not_for_topic),
        (
            Sent::Rejected(Refusal::Token),
            Sent::Rejected(Refusal::Token)
        )
    );
}
