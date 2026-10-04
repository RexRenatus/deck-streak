//! SPEC-343 A19: each sender's, the subscription's, the device's and both settings' `Debug` names
//! its type and what it holds that is safe to show, and holds no key material, no token sent, no
//! device token and no endpoint. Each sender has sent once first, so it holds a token to leak.

mod support;

use std::time::Duration;

use deck_streak_push::{
    ApnsSender, ApnsSettings, Device, Environment, Notification, Origin, PushServices, Sent,
    WebPushSender, WebPushSettings,
};

use support::fake_apns::FakeApns;
use support::fake_push_service::FakePushService;
use support::keys::{
    CONTACT, ENDPOINT_PATH, KEY_ID, TEAM_ID, TOPIC, TestKey, TestSubscriber, device_token,
};
use support::{DEADLINE, Fake, clock};

#[tokio::test]
async fn a19_debug_holds_no_key_token_or_endpoint() {
    let key = TestKey::generate();
    let apns = FakeApns::start(key.verifying()).await;
    let service = FakePushService::start().await;
    let origin = |fake: &Fake| Origin::new(fake.origin()).expect("a loopback origin");
    let apns_settings = ApnsSettings {
        key_pem: &key.pem,
        key_id: KEY_ID,
        team_id: TEAM_ID,
        topic: TOPIC,
        development: origin(&apns.fake),
        production: origin(&apns.fake),
        deadline: DEADLINE,
    };
    let apns_settings_debug = format!("{apns_settings:?}");
    let apns_sender = ApnsSender::new(apns_settings, clock()).expect("the APNs sender builds");
    let web_settings = WebPushSettings {
        key_pem: &key.pem,
        contact: CONTACT,
        services: PushServices::new([origin(&service.fake)]),
        deadline: DEADLINE,
    };
    let web_settings_debug = format!("{web_settings:?}");
    let web_sender = WebPushSender::new(web_settings, clock()).expect("the web push sender builds");
    let browser = TestSubscriber::generate();
    let subscription = web_sender
        .subscription(&service.endpoint(), &browser.p256dh(), &browser.auth())
        .expect("a listed endpoint is admitted");
    let device = Device::new(&device_token(), Environment::Development).expect("a device");
    let notification =
        Notification::new("Synthetic title", "Synthetic body", Duration::from_secs(60));
    assert_eq!(
        apns_sender.deliver(&device, &notification).await,
        Sent::Delivered
    );
    assert_eq!(
        web_sender.deliver(&subscription, &notification).await,
        Sent::Delivered
    );
    let provider_token = FakeApns::raw_token(&apns.fake.received()[0]);
    let vapid_token = FakePushService::vapid(&service.fake.received()[0])
        .expect("a VAPID header")
        .token;

    // The present case first: each output names its type and what it holds that is safe to show.
    let outputs = [
        (
            "ApnsSender",
            format!("{apns_sender:?}"),
            &["Signer", "Client", "Origin("][..],
        ),
        (
            "WebPushSender",
            format!("{web_sender:?}"),
            &["Signer", "Client", "Origin("][..],
        ),
        ("Subscription", format!("{subscription:?}"), &[][..]),
        ("Device", format!("{device:?}"), &["Development"][..]),
        ("ApnsSettings", apns_settings_debug, &[][..]),
        ("WebPushSettings", web_settings_debug, &[][..]),
    ];
    for (name, output, shown) in &outputs {
        assert!(
            output.starts_with(name),
            "{name}'s Debug names its type: {output}"
        );
        for part in *shown {
            assert!(
                output.contains(part),
                "{name}'s Debug shows {part}: {output}"
            );
        }
    }

    // No output holds key material, a token sent, the device token or the endpoint.
    let mut held = key.pem_body();
    held.extend([
        key.scalar_debug(),
        provider_token,
        vapid_token,
        device_token(),
        service.endpoint(),
        ENDPOINT_PATH.to_owned(),
        browser.p256dh(),
        browser.auth(),
    ]);
    assert!(
        held.iter().all(|value| value.len() >= 16),
        "every value is long enough to find: {held:?}"
    );
    println!("examined {} Debug output(s)", outputs.len());
    for (name, output, _) in &outputs {
        let leaked: Vec<&String> = held
            .iter()
            .filter(|value| output.contains(value.as_str()))
            .collect();
        assert!(leaked.is_empty(), "{name}'s Debug holds {leaked:?}");
    }
}
