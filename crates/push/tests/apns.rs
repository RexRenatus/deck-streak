//! SPEC-343 A1 to A9: the APNs sender against two recording APNs fakes, one per environment. Each
//! provider token is verified by the fake with the test's public key; every key is generated in
//! memory, every value is synthetic, and time moves on the kernel's manual clock, never by sleeping.

mod support;

use std::sync::Arc;
use std::time::Duration;

use axum::http::Version;
use deck_streak_kernel::ManualClock;
use deck_streak_push::{
    ApnsSender, ApnsSettings, CollapseKey, Device, Environment, Notification, Origin, Sent,
};
use serde_json::json;

use support::fake_apns::FakeApns;
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
