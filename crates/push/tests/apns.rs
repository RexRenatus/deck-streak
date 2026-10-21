//! SPEC-343 A1 to A9: the APNs sender against two recording APNs fakes, one per environment. Each
//! provider token is verified by the fake with the test's public key; every key is generated in
//! memory, every value is synthetic, and time moves on the kernel's manual clock, never by sleeping.

#![allow(
    clippy::expect_used,
    reason = "a helper builds what every test needs and panics like a test, but clippy's \
              allow-expect-in-tests reaches only #[test] functions"
)]

mod support;

use std::sync::Arc;
use std::time::Duration;

use axum::http::Version;
use deck_streak_kernel::{ManualClock, UtcMillis};
use deck_streak_push::{
    ApnsSender, ApnsSettings, BuildError, CollapseKey, Device, Environment, Notification, Origin,
    Refusal, Sent, Unreached,
};
use serde_json::json;
use tokio::sync::Notify;

use support::fake_apns::{FakeApns, gone, refusal};
use support::keys::{KEY_ID, TEAM_ID, TOPIC, TestKey, device_token};
use support::{Answer, DEADLINE, Recorded, START, clock, verify_jwt};

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
    Notification::new("Synthetic title", "Synthetic body", Duration::from_hours(1))
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
    assert_eq!(
        verified.claims,
        json!({"iss": TEAM_ID, "iat": START_SECOND})
    );

    // The fake's check is the signature's: another key does not verify the same token.
    let stranger = TestKey::generate().verifying();
    assert_eq!(
        verify_jwt(&FakeApns::raw_token(&received[0]), &stranger),
        None
    );
}

#[tokio::test]
async fn a3_the_provider_token_is_reused_inside_its_window_and_reminted_after() {
    let rig = Rig::start().await;

    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);
    rig.clock.advance(Duration::from_mins(19));
    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);
    rig.clock.advance(Duration::from_mins(26));
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
fn expired() -> Answer {
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
    assert_eq!(
        received.len(),
        3,
        "the send and its one resend, after the prime"
    );
    assert_eq!(sent, Sent::Rejected(Refusal::ProviderToken));
    let tokens: Vec<String> = received.iter().map(FakeApns::raw_token).collect();
    assert_eq!(
        tokens[1], tokens[0],
        "the refused send carried the held token"
    );
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

#[tokio::test]
async fn a7_try_later_answers_are_reported_and_not_retried() {
    let rig = Rig::start().await;
    rig.development.fake.script([
        refusal(429, "TooManyRequests"),
        refusal(500, "InternalServerError"),
        refusal(503, "ServiceUnavailable"),
    ]);

    let mut outcomes = Vec::new();
    for sent in 1..=3 {
        outcomes.push(rig.deliver(&alert()).await);
        assert_eq!(rig.received().len(), sent, "one request each");
    }

    assert_eq!(outcomes, vec![Sent::RetryLater { after: None }; 3]);
}

#[tokio::test]
async fn a8_an_oversize_payload_is_refused_before_any_request() {
    let rig = Rig::start().await;
    // The payload is `{"aps":{"alert":{"title":T,"body":B}}}`; the test measures its own frame.
    let frame = json!({"aps": {"alert": {"title": "Synthetic title", "body": ""}}})
        .to_string()
        .len();
    let sized = |payload: usize| {
        Notification::new(
            "Synthetic title",
            &"x".repeat(payload - frame),
            Duration::from_hours(1),
        )
    };

    assert_eq!(
        rig.deliver(&sized(4097)).await,
        Sent::Rejected(Refusal::TooLarge)
    );
    assert_eq!(
        rig.received().len(),
        0,
        "the oversize payload made no request"
    );
    assert_eq!(rig.deliver(&sized(4096)).await, Sent::Delivered);

    let received = rig.received();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].body.len(), 4096);
}

#[tokio::test]
async fn a9_each_device_reaches_its_own_environment_only() {
    let rig = Rig::start().await;
    let production = Device::new(&device_token(), Environment::Production).expect("a device");

    assert_eq!(
        rig.sender.deliver(&production, &alert()).await,
        Sent::Delivered
    );
    assert_eq!(
        (
            rig.production.fake.received().len(),
            rig.development.fake.received().len()
        ),
        (1, 0),
        "a production device reaches the production service alone"
    );

    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);
    assert_eq!(
        (
            rig.production.fake.received().len(),
            rig.development.fake.received().len()
        ),
        (1, 1),
        "a development device reaches the development service alone"
    );
}

// MUTATION COVERAGE: each test below holds a behaviour no criterion names alone, beside its positive
// control, so that a mutant removing it fails a test.

#[tokio::test]
async fn a_young_token_is_not_reminted_on_an_expired_answer() {
    let rig = Rig::start().await;
    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);

    // 19 minutes on, APNs's floor forbids a new token: the refusal is the answer, with no resend.
    rig.clock.advance(minutes(19));
    rig.development.fake.script([expired()]);
    assert_eq!(
        rig.deliver(&alert()).await,
        Sent::Rejected(Refusal::ProviderToken)
    );
    assert_eq!(rig.received().len(), 2, "no resend inside the floor");

    // The positive control: the same refusal at 20 minutes earns the resend.
    rig.clock.advance(minutes(1));
    rig.development.fake.script([expired()]);
    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);
    let tokens: Vec<String> = rig.received().iter().map(FakeApns::raw_token).collect();
    assert_eq!(tokens.len(), 4);
    assert_eq!(
        (
            tokens[1] == tokens[0],
            tokens[2] == tokens[0],
            tokens[3] == tokens[0]
        ),
        (true, true, false),
        "no token was minted inside the floor, and one was at it"
    );
}

#[tokio::test]
async fn an_oversize_error_body_is_not_read_past_its_bound() {
    let rig = Rig::start().await;
    // An error body of `size` bytes whose reason refuses the device token.
    let frame = json!({"reason": "BadDeviceToken", "padding": ""})
        .to_string()
        .len();
    let refusing = |size: usize| {
        Answer::status(400).body(
            json!({"reason": "BadDeviceToken", "padding": "x".repeat(size - frame)}).to_string(),
        )
    };
    rig.development
        .fake
        .script([refusing(4096), refusing(4097)]);

    // The positive control: a body at the bound is read, and its reason decides the outcome.
    assert_eq!(rig.deliver(&alert()).await, Sent::Rejected(Refusal::Token));
    // Past the bound it is not read, so the reason is unknown and the status alone decides.
    assert_eq!(
        rig.deliver(&alert()).await,
        Sent::Rejected(Refusal::Request)
    );
    assert_eq!(rig.received().len(), 2, "one request each");
}

#[tokio::test]
async fn two_calls_refused_together_mint_one_token() {
    let rig = Rig::start().await;
    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);
    rig.clock.advance(minutes(30));

    // Both calls send the held token and both are refused; the second refusal is held until the
    // first call has reminted and resent, and the clock has moved a second, so a second mint would
    // carry a later `iat` and show.
    let gate = Arc::new(Notify::new());
    rig.development
        .fake
        .script([expired(), expired().held(&gate)]);
    let controller = async {
        rig.development.fake.arrived(4).await;
        rig.clock.advance(Duration::from_secs(1));
        gate.notify_one();
    };
    let notification = alert();
    let (first, second, ()) = tokio::join!(
        rig.deliver(&notification),
        rig.deliver(&notification),
        controller
    );

    assert_eq!((first, second), (Sent::Delivered, Sent::Delivered));
    let received = rig.received();
    let tokens: Vec<String> = received.iter().map(FakeApns::raw_token).collect();
    assert_eq!(
        tokens.len(),
        5,
        "the prime, two refused sends and two resends"
    );
    let held = tokens[0].clone();
    let reminted = tokens[3].clone();
    assert_ne!(reminted, held);
    assert_eq!(
        tokens,
        vec![held.clone(), held.clone(), held, reminted.clone(), reminted]
    );
    let minted = rig
        .development
        .token(&received[3])
        .expect("the fake verifies the new token");
    assert_eq!(minted.claims["iat"], json!(START_SECOND + 30 * 60));
}

#[tokio::test]
async fn every_other_answer_is_read_by_its_status_after_one_request() {
    let rig = Rig::start().await;
    assert_eq!(rig.deliver(&alert()).await, Sent::Delivered);
    // The held token is 20 minutes old, so only the reason keeps a 403 from earning a resend.
    rig.clock.advance(minutes(20));
    let table = [
        (
            refusal(403, "InvalidProviderToken"),
            Sent::Rejected(Refusal::ProviderToken),
        ),
        (
            refusal(413, "PayloadTooLarge"),
            Sent::Rejected(Refusal::TooLarge),
        ),
        (
            refusal(400, "BadPriority"),
            Sent::Rejected(Refusal::Request),
        ),
        (refusal(404, "BadPath"), Sent::Rejected(Refusal::Request)),
        (
            refusal(405, "MethodNotAllowed"),
            Sent::Rejected(Refusal::Request),
        ),
        (
            Answer::status(301).header("location", "https://moved.synthetic.invalid"),
            Sent::Failed(Unreached::Redirect),
        ),
        (
            Answer::status(307).header("location", "https://moved.synthetic.invalid"),
            Sent::Failed(Unreached::Redirect),
        ),
        (Answer::status(202), Sent::Delivered),
    ];

    let mut read = Vec::new();
    for (count, (answer, _)) in table.iter().enumerate() {
        rig.development.fake.script([answer.clone()]);
        read.push(rig.deliver(&alert()).await);
        assert_eq!(rig.received().len(), count + 2, "one request each");
    }
    println!("examined {} answer(s)", table.len());

    let expected: Vec<Sent> = table.iter().map(|(_, sent)| *sent).collect();
    assert_eq!(read, expected);
}

#[tokio::test]
async fn an_answer_past_the_deadline_fails_it() {
    // The one real wait: a short deadline against a fake that never answers.
    let rig = Rig::with_deadline(Duration::from_millis(200)).await;
    rig.development.fake.script([Answer::silence()]);

    assert_eq!(
        rig.deliver(&alert()).await,
        Sent::Failed(Unreached::Deadline)
    );
    assert_eq!(rig.received().len(), 1);
}

#[tokio::test]
async fn a_closed_port_fails_the_connection() {
    let key = TestKey::generate();
    let closed = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a loopback port");
        format!("http://{}", listener.local_addr().expect("its address"))
    };
    let origin = Origin::new(&closed).expect("a loopback origin");
    let sender = ApnsSender::new(
        ApnsSettings {
            key_pem: &key.pem,
            key_id: KEY_ID,
            team_id: TEAM_ID,
            topic: TOPIC,
            development: origin.clone(),
            production: origin,
            deadline: DEADLINE,
        },
        clock(),
    )
    .expect("the sender builds");

    assert_eq!(
        sender.deliver(&device(), &alert()).await,
        Sent::Failed(Unreached::Connection)
    );
}

#[test]
fn a_device_token_is_hexadecimal_text_of_at_most_200_characters() {
    let refused = [
        String::new(),
        "a".repeat(201),
        "zz".repeat(32),
        "ab/../".repeat(8),
        format!("{} ", device_token()),
    ];
    let wrongly_admitted: Vec<&String> = refused
        .iter()
        .filter(|token| {
            Device::new(token, Environment::Development) != Err(BuildError::DeviceToken)
        })
        .collect();
    println!("examined {} refused token(s)", refused.len());
    assert_eq!(wrongly_admitted, Vec::<&String>::new());

    // The positive control: 200 hexadecimal characters, in either case, are a device.
    for token in ["ab".repeat(100), "AB".repeat(100), "0".to_owned()] {
        let device = Device::new(&token, Environment::Production).expect("a device token");
        assert_eq!(device.environment(), Environment::Production);
    }
}

#[test]
fn a_key_that_does_not_parse_refuses_the_build() {
    fn settings(key_pem: &str) -> ApnsSettings<'_> {
        ApnsSettings {
            key_pem,
            key_id: KEY_ID,
            team_id: TEAM_ID,
            topic: TOPIC,
            development: Origin::new("https://development.synthetic.invalid").expect("an origin"),
            production: Origin::new("https://production.synthetic.invalid").expect("an origin"),
            deadline: DEADLINE,
        }
    }

    let refused = ApnsSender::new(settings("not a key"), clock());
    assert_eq!(refused.err(), Some(BuildError::Key));

    // The positive control: the test's own key builds a sender.
    let key = TestKey::generate();
    assert!(ApnsSender::new(settings(&key.pem), clock()).is_ok());
}
