//! SPEC-343 A18: every send writes one line that names its sender and its outcome, and no line the
//! test's thread logs, from any crate, holds a device token, an endpoint, a request path or a
//! token. The lines are captured the one way the workspace captures them, the log-capture helper.

mod support;

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use deck_streak_push::{
    ApnsSender, ApnsSettings, Device, Environment, Notification, Origin, PushServices, Sent,
    WebPushSender, WebPushSettings,
};

use support::fake_apns::{FakeApns, gone, refusal};
use support::fake_push_service::FakePushService;
use support::keys::{
    CONTACT, ENDPOINT_PATH, KEY_ID, TEAM_ID, TOPIC, TestKey, TestSubscriber, device_token,
};
use support::{Answer, DEADLINE, START, clock};

/// The outcome names one sender's five scripted answers stand for, in order.
const OUTCOMES: [&str; 5] = ["delivered", "gone", "rejected", "retry-later", "failed"];

#[tokio::test]
async fn a18_no_log_line_carries_a_token_an_endpoint_or_a_path() {
    let lines = Lines::default();
    let _logging = log_capture::hold_capture(lines.clone());
    let key = TestKey::generate();
    let apns = FakeApns::start(key.verifying()).await;
    let service = FakePushService::start().await;
    let origin = |fake: &support::Fake| Origin::new(fake.origin()).expect("a loopback origin");
    let apns_sender = ApnsSender::new(
        ApnsSettings {
            key_pem: &key.pem,
            key_id: KEY_ID,
            team_id: TEAM_ID,
            topic: TOPIC,
            development: origin(&apns.fake),
            production: origin(&apns.fake),
            deadline: DEADLINE,
        },
        clock(),
    )
    .expect("the APNs sender builds");
    let web_sender = WebPushSender::new(
        WebPushSettings {
            key_pem: &key.pem,
            contact: CONTACT,
            services: PushServices::new([origin(&service.fake)]),
            deadline: DEADLINE,
        },
        clock(),
    )
    .expect("the web push sender builds");
    let browser = TestSubscriber::generate();
    let subscription = web_sender
        .subscription(&service.endpoint(), &browser.p256dh(), &browser.auth())
        .expect("a listed endpoint is admitted");
    let device = Device::new(&device_token(), Environment::Development).expect("a device");
    let notification =
        Notification::new("Synthetic title", "Synthetic body", Duration::from_secs(60));
    apns.fake.script([
        Answer::status(200),
        gone("Unregistered", START),
        refusal(400, "BadDeviceToken"),
        Answer::status(429),
        Answer::status(301),
    ]);
    service.fake.script([
        Answer::status(201),
        Answer::status(410),
        Answer::status(413),
        Answer::status(503),
        Answer::status(301),
    ]);

    let mut answered: Vec<Sent> = Vec::new();
    for _ in OUTCOMES {
        answered.push(apns_sender.deliver(&device, &notification).await);
    }
    for _ in OUTCOMES {
        answered.push(web_sender.deliver(&subscription, &notification).await);
    }

    // The present case first: the sends reached every outcome, one line per call names its sender
    // and its outcome, in order, and the crate logged nothing else.
    let names: Vec<&str> = answered.iter().map(|sent| sent.name()).collect();
    assert_eq!(names, [OUTCOMES, OUTCOMES].concat(), "{answered:?}");
    let logged = lines.logged();
    let ours: Vec<Vec<&str>> = logged
        .iter()
        .map(|line| line.split_whitespace().collect::<Vec<&str>>())
        .filter(|words| {
            words
                .get(1)
                .is_some_and(|target| target.starts_with("deck_streak_push"))
        })
        .collect();
    let expected: Vec<(String, String)> = OUTCOMES
        .iter()
        .map(|outcome| ("sender=apns".to_owned(), format!("outcome={outcome}")))
        .chain(
            OUTCOMES
                .iter()
                .map(|outcome| ("sender=web-push".to_owned(), format!("outcome={outcome}"))),
        )
        .collect();
    assert_eq!(ours.len(), expected.len(), "a line per call: {logged:#?}");
    for (words, (sender, outcome)) in ours.iter().zip(&expected) {
        assert!(
            words.contains(&sender.as_str()) && words.contains(&outcome.as_str()),
            "{words:?} names {sender} and {outcome}"
        );
    }

    // No line holds a device token, an endpoint, a request path or a token sent.
    let mut held = vec![
        device_token(),
        service.endpoint(),
        ENDPOINT_PATH.to_owned(),
        "/3/device/".to_owned(),
    ];
    let sent_tokens: Vec<String> = apns
        .fake
        .received()
        .iter()
        .map(FakeApns::raw_token)
        .chain(service.fake.received().iter().map(|request| {
            FakePushService::vapid(request)
                .expect("a VAPID header")
                .token
        }))
        .collect();
    assert_eq!(sent_tokens.len(), 10, "a token per request");
    assert!(
        sent_tokens.iter().all(|token| token.len() > 64),
        "{sent_tokens:?}"
    );
    held.extend(sent_tokens);
    println!("examined {} logged line(s)", logged.len());
    let leaking: Vec<&String> = logged
        .iter()
        .filter(|line| held.iter().any(|value| line.contains(value.as_str())))
        .collect();
    assert!(leaking.is_empty(), "a line holds a value: {leaking:#?}");
}

/// Every line the test's thread logs, at every level and from every target: each event, and each
/// span's fields, as `name=value` pairs.
#[derive(Clone, Default)]
struct Lines(Arc<Mutex<Vec<String>>>);

impl Lines {
    fn logged(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn push(&self, line: String) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
}

/// One event's or span's fields, as `name=value` pairs.
#[derive(Default)]
struct Fields(Vec<String>);

impl tracing::field::Visit for Fields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.push(format!("{}={value}", field.name()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.0.push(format!("{}={value:?}", field.name()));
    }
}

impl tracing::Subscriber for Lines {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        let mut fields = Fields::default();
        span.record(&mut fields);
        self.push(format!(
            "span {} {}",
            span.metadata().name(),
            fields.0.join(" ")
        ));
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        let mut fields = Fields::default();
        values.record(&mut fields);
        self.push(format!("record {}", fields.0.join(" ")));
    }

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let metadata = event.metadata();
        self.push(format!(
            "{} {} {}",
            metadata.level(),
            metadata.target(),
            fields.0.join(" ")
        ));
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}
