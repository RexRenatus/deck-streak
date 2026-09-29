//! The transport: a 429 is waited out and the same request repeated, as the predecessor did; every
//! dynamic value is escaped where it enters the markup; and the counts tell a failed send from a
//! delivered one (SPEC-026 A2, A11, A12; R6, R8, R10).
//!
//! Every test sends through a fake Bot API on a loopback port. The transport's waits go to the
//! fake's recorder, which notes each and returns at once, so the golden's waits are read from what
//! it noted rather than slept (ADR-026); a network error is an answer withheld past the client's
//! timeout.

// An integration test is test code: its helpers panic on a failed check, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;
#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::time::Duration;

use deck_streak_bot::commands::{privacy_reply, sync_reply};
use deck_streak_bot::gate::{MAX_CALLBACK_DATA_BYTES, MAX_INBOUND_TEXT};
use deck_streak_bot::poll::LONG_POLL_SECONDS;
use deck_streak_bot::transport::{
    API_URL, DEFAULT_RETRY_AFTER, HTTP_TIMEOUT, MAX_TEXT_UTF16, SEND_ATTEMPTS, TokioTimer, Waits,
    escape_attribute, escape_html,
};
use deck_streak_bot::{ApiUrl, Scores, SendCounts, Sent, SyncAnswer, SyncOutcome, TransportError};
use deck_streak_kernel::Environment;
use fake_bot_api::{Answer, FakeBotApi, OWNER, TOKEN, payload};
use serde_json::{Value, json};

/// The fake's answer for one response of `goldens/send_retry.json`.
fn answer_of(response: &Value) -> Answer {
    if response["network_error"] == true {
        return Answer::Silence;
    }
    let status = u16::try_from(response["status"].as_u64().expect("a status")).expect("a u16");
    match (status, response["retry_after"].as_u64()) {
        (200, _) => Answer::Result(json!({
            "message_id": 7,
            "date": 0,
            "chat": {"id": OWNER, "type": "private"},
        })),
        (429, Some(seconds)) => Answer::too_many(seconds),
        (429, None) => Answer::too_many_bare(),
        (other, _) => Answer::status(other),
    }
}

#[tokio::test]
async fn a_429_waits_retry_after_seconds_and_repeats_the_same_request() {
    let golden = golden::read(&golden::committed("send_retry")).expect("the golden reads");
    println!(
        "examined {} case(s) of {}",
        golden.cases.len(),
        golden.function
    );
    assert!(!golden.cases.is_empty(), "examined 0 cases");
    // Each case's attempts and waits first, then what the sends delivered and counted: a transport
    // that never retries fails on the first 429, not on its counts.
    let mut outcomes = Vec::new();
    for case in &golden.cases {
        let responses = case.input["responses"].as_array().expect("responses");
        let fake = FakeBotApi::start().await;
        let directory = tempfile::tempdir().expect("a temporary directory");
        let transport = fake.transport(directory.path());
        fake.script("sendMessage", responses.iter().map(answer_of));

        let sent = transport
            .send_html(OWNER, "a synthetic message", None)
            .await;

        let calls = fake.calls_of("sendMessage");
        let expected = &case.output;
        let label = format!("{:?} {}", case.class, case.input);
        assert_eq!(
            Some(calls.len() as u64),
            expected["requests"].as_u64(),
            "{label}: the requests"
        );
        assert!(
            calls.windows(2).all(|pair| pair[0].body == pair[1].body),
            "{label}: every attempt repeats the same request"
        );
        // The golden lists the waits the predecessor slept: a 429's before the next attempt, and
        // none after the last attempt or after any other failure.
        let waits: Vec<Duration> = expected["waits_seconds"]
            .as_array()
            .expect("waits")
            .iter()
            .map(|wait| Duration::from_secs_f64(wait.as_f64().expect("seconds")))
            .collect();
        assert_eq!(fake.waits(), waits, "{label}: the waits");
        outcomes.push((label, sent, transport.counts(), expected));
    }
    for (label, sent, counts, expected) in outcomes {
        let delivered = expected["delivered"] == true;
        assert_eq!(
            matches!(sent, Sent::Delivered { .. }),
            delivered,
            "{label}: delivered"
        );
        let marker = expected["send_marker"].as_array().expect("a marker");
        assert_eq!(
            counts,
            SendCounts {
                attempted: marker[0].as_u64().expect("attempted"),
                delivered: marker[1].as_u64().expect("delivered"),
            },
            "{label}: the counts"
        );
    }
}

#[tokio::test]
async fn every_dynamic_value_is_html_escaped() {
    // The escape: `&` first, then `<` and `>`, each written once.
    assert_eq!(
        escape_html("<b>Tom & Jerry</b>"),
        "&lt;b&gt;Tom &amp; Jerry&lt;/b&gt;"
    );
    assert_eq!(escape_html("&lt;"), "&amp;lt;");
    assert_eq!(escape_html("plain"), "plain");
    assert_eq!(
        escape_attribute("https://x.example/?a=\"1\"&b=<2>"),
        "https://x.example/?a=&quot;1&quot;&amp;b=&lt;2&gt;"
    );

    // Where a value enters the markup: a reason code with markup in it reaches Telegram escaped.
    let hostile = "<i>&injected</i>";
    let answer = Ok(SyncAnswer {
        sync: SyncOutcome::Failed {
            reason: hostile.to_owned(),
        },
        scores: Scores::Recomputed,
    });
    let reply = sync_reply(&answer);
    assert!(
        reply.text.contains("&lt;i&gt;&amp;injected&lt;/i&gt;"),
        "the reason is escaped: {}",
        reply.text
    );
    assert!(!reply.text.contains(hostile), "{}", reply.text);

    let fake = FakeBotApi::start().await;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = fake.transport(directory.path());
    let sent = transport.send_html(OWNER, &reply.text, None).await;
    assert!(matches!(sent, Sent::Delivered { .. }), "{sent:?}");
    let call = fake.calls_of("sendMessage").pop().expect("one send");
    assert_eq!(
        call.body["text"], reply.text,
        "the text is sent as rendered"
    );
    assert_eq!(call.body["parse_mode"], "HTML", "every text is HTML");
    assert_eq!(
        call.body["link_preview_options"],
        json!({"is_disabled": true}),
        "link previews are off"
    );
    assert!(
        call.body.get("disable_web_page_preview").is_none(),
        "no replaced field"
    );

    // An attribute's value is escaped as an attribute.
    assert!(
        privacy_reply().text.contains("<a href=\"https://"),
        "{}",
        privacy_reply().text
    );
}

#[tokio::test]
async fn the_delivery_marker_tells_a_failed_send_from_a_delivered_one() {
    let fake = FakeBotApi::start().await;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = fake.transport(directory.path());
    assert_eq!(
        transport.counts(),
        SendCounts::default(),
        "nothing sent yet"
    );

    let delivered = transport.send_html(OWNER, "delivered", None).await;
    assert!(matches!(delivered, Sent::Delivered { .. }), "{delivered:?}");
    assert_eq!(
        transport.counts(),
        SendCounts {
            attempted: 1,
            delivered: 1
        }
    );

    // Every attempt refused: attempted moves, delivered does not.
    fake.script(
        "sendMessage",
        (0..SEND_ATTEMPTS).map(|_| Answer::status(502)),
    );
    let failed = transport.send_html(OWNER, "refused", None).await;
    assert_eq!(failed, Sent::Failed);
    assert_eq!(
        transport.counts(),
        SendCounts {
            attempted: 2,
            delivered: 1
        }
    );

    // A message is counted once, whatever its chunks: two chunks, both delivered.
    let long = format!("{}\n\n{}", "a".repeat(MAX_TEXT_UTF16), "b".repeat(10));
    let two = transport.send_html(OWNER, &long, None).await;
    assert!(matches!(two, Sent::Delivered { .. }), "{two:?}");
    assert_eq!(
        transport.counts(),
        SendCounts {
            attempted: 3,
            delivered: 2
        }
    );

    // Its second chunk refused on every attempt: not delivered.
    let before = fake.calls_of("sendMessage").len();
    fake.script(
        "sendMessage",
        std::iter::once(Answer::Result(json!({
            "message_id": 9,
            "date": 0,
            "chat": {"id": OWNER, "type": "private"},
        })))
        .chain((0..SEND_ATTEMPTS).map(|_| Answer::status(500))),
    );
    let half = transport.send_html(OWNER, &long, None).await;
    assert_eq!(half, Sent::Failed);
    assert_eq!(
        fake.calls_of("sendMessage").len() - before,
        1 + SEND_ATTEMPTS as usize,
        "the first chunk once, the second in every attempt"
    );
    assert_eq!(
        transport.counts(),
        SendCounts {
            attempted: 4,
            delivered: 2
        }
    );

    // A request the server never answers is a failed attempt too.
    fake.script("sendMessage", (0..SEND_ATTEMPTS).map(|_| Answer::Silence));
    assert_eq!(
        transport.send_html(OWNER, "silence", None).await,
        Sent::Failed
    );
    assert_eq!(
        transport.counts(),
        SendCounts {
            attempted: 5,
            delivered: 2
        }
    );
}

#[tokio::test]
async fn an_edit_answered_not_modified_counts_as_delivered() {
    let fake = FakeBotApi::start().await;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = fake.transport(directory.path());
    fake.script(
        "editMessageText",
        [Answer::Refused {
            status: 400,
            description: "Bad Request: message is not modified: specified new message content \
                          and reply markup are exactly the same",
            retry_after: None,
        }],
    );
    let edited = transport.edit_html(OWNER, 41, "the same text").await;
    assert_eq!(edited, Sent::Delivered { message_id: 41 });
    let calls = fake.calls_of("editMessageText");
    assert_eq!(calls.len(), 1, "a no-op edit is not repeated");
    assert_eq!(calls[0].body["message_id"], 41);
    assert_eq!(calls[0].body["parse_mode"], "HTML");

    // Any other 400 is a failure, repeated as a send's.
    fake.script(
        "editMessageText",
        (0..SEND_ATTEMPTS).map(|_| Answer::Refused {
            status: 400,
            description: "Bad Request: can't parse entities",
            retry_after: None,
        }),
    );
    assert_eq!(transport.edit_html(OWNER, 42, "text").await, Sent::Failed);
    assert_eq!(
        transport.counts(),
        SendCounts {
            attempted: 2,
            delivered: 1
        }
    );

    // An edit carries one message: a text over the bound is refused before any request.
    let before = fake.calls_of("editMessageText").len();
    let long = "a".repeat(MAX_TEXT_UTF16 + 1);
    assert_eq!(transport.edit_html(OWNER, 43, &long).await, Sent::Failed);
    assert_eq!(fake.calls_of("editMessageText").len(), before);
}

#[tokio::test]
async fn a_document_is_uploaded_from_memory_with_its_caption() {
    let fake = FakeBotApi::start().await;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = fake.transport(directory.path());
    fake.script("sendDocument", [Answer::too_many(4)]);
    let bytes = br#"{"schema":"synthetic"}"#;

    let sent = transport
        .send_document(OWNER, "synthetic.json", bytes, "a <b>caption</b>")
        .await;

    assert!(matches!(sent, Sent::Delivered { .. }), "{sent:?}");
    let calls = fake.calls_of("sendDocument");
    assert_eq!(calls.len(), 2, "a 429, then the same upload again");
    assert_eq!(
        fake.waits(),
        vec![Duration::from_secs(4)],
        "its retry_after waited out"
    );
    let upload = &calls[1].body;
    assert_eq!(upload["chat_id"], OWNER.to_string());
    assert_eq!(upload["caption"], "a <b>caption</b>");
    assert_eq!(upload["parse_mode"], "HTML");
    assert_eq!(upload["document"]["filename"], "synthetic.json");
    assert_eq!(upload["document"]["content_type"], "application/json");
    assert_eq!(upload["document"]["text"], r#"{"schema":"synthetic"}"#);
    assert_eq!(calls[0].body, calls[1].body, "the same request each time");

    // A caption at its bound goes as it is.
    let bound = "c".repeat(1024);
    let sent = transport
        .send_document(OWNER, "synthetic.json", bytes, &bound)
        .await;
    assert!(matches!(sent, Sent::Delivered { .. }), "{sent:?}");
    let last = fake.calls_of("sendDocument").pop().expect("the upload");
    assert_eq!(
        last.body["caption"], bound,
        "a caption of 1024 units is kept"
    );

    // A caption over its bound is left off; the document still goes.
    let long = "c".repeat(1025);
    let sent = transport
        .send_document(OWNER, "synthetic.json", bytes, &long)
        .await;
    assert!(matches!(sent, Sent::Delivered { .. }), "{sent:?}");
    let last = fake.calls_of("sendDocument").pop().expect("the upload");
    assert!(last.body.get("caption").is_none(), "{}", last.body);
    assert_eq!(
        transport.counts(),
        SendCounts {
            attempted: 3,
            delivered: 3
        }
    );
}

#[test]
fn the_api_url_is_https_or_loopback_http() {
    let accepted = [
        ("https://api.telegram.org", "https://api.telegram.org"),
        ("https://api.telegram.org/", "https://api.telegram.org"),
        ("http://127.0.0.1:8081", "http://127.0.0.1:8081"),
        ("http://[::1]:8081", "http://[::1]:8081"),
        ("http://localhost:8081", "http://localhost:8081"),
    ];
    for (text, url) in accepted {
        assert_eq!(
            ApiUrl::new(text).map(|api| api.as_str().to_owned()),
            Some(url.to_owned()),
            "{text}"
        );
    }
    let refused = [
        "http://api.telegram.org",
        "http://192.0.2.10:8081",
        "ftp://127.0.0.1",
        "https://",
        "https://@api.telegram.org",
        "https://api.telegram.org?x=1",
        "https://api.telegram.org#x",
        "https://api .telegram.org",
        "api.telegram.org",
    ];
    for text in refused {
        assert_eq!(ApiUrl::new(text), None, "{text}");
    }
    let unset = Environment::from_vars(Vec::<(String, String)>::new());
    assert_eq!(
        ApiUrl::from_env(&unset).map(|api| api.as_str().to_owned()),
        Ok("https://api.telegram.org".to_owned()),
        "unset, it is Telegram's own"
    );
    let set = Environment::from_vars([(API_URL, "http://127.0.0.1:9")]);
    assert_eq!(
        ApiUrl::from_env(&set).map(|api| api.as_str().to_owned()),
        Ok("http://127.0.0.1:9".to_owned())
    );
    let plain = Environment::from_vars([(API_URL, "http://api.telegram.org")]);
    assert!(
        ApiUrl::from_env(&plain).is_err(),
        "cleartext to a remote host is refused"
    );
}

#[test]
fn the_api_url_names_no_path() {
    // The root is the base: its `/` is dropped, and the token is appended to the host alone.
    assert_eq!(
        ApiUrl::new("https://api.telegram.org/").map(|api| api.as_str().to_owned()),
        Some("https://api.telegram.org".to_owned())
    );
    let refused = [
        "https://api.telegram.org/x",
        "https://api.telegram.org/x/y",
        "https://api.telegram.org/x/",
        "https://api.telegram.org//",
        "http://127.0.0.1:8081/x",
    ];
    for text in refused {
        assert_eq!(ApiUrl::new(text), None, "{text}");
    }
}

#[test]
fn the_constants_equal_the_predecessors() {
    let mut constants = golden::read(&golden::committed("bot.constants"))
        .expect("the golden reads")
        .cases
        .into_iter()
        .map(|case| {
            (
                case.input["name"].as_str().expect("a name").to_owned(),
                case.output,
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    println!("examined {} constant(s) of bot.constants", constants.len());
    let mut take = |name: &str| constants.remove(name).unwrap_or_else(|| panic!("{name}"));
    assert_eq!(take("constants.TELEGRAM_MAX_LEN"), json!(MAX_TEXT_UTF16));
    assert_eq!(take("bot._MAX_INBOUND_TEXT"), json!(MAX_INBOUND_TEXT));
    assert_eq!(
        take("bot._MAX_CALLBACK_DATA"),
        json!(MAX_CALLBACK_DATA_BYTES)
    );
    assert_eq!(
        take("telegram._DEFAULT_RETRY_AFTER"),
        json!(DEFAULT_RETRY_AFTER.as_secs_f64())
    );
    assert_eq!(
        take("telegram.TelegramNotifier.send_html.__kwdefaults__"),
        json!({"max_retries": SEND_ATTEMPTS})
    );
    assert!(
        constants.is_empty(),
        "every constant is checked: {constants:?}"
    );

    let timeouts = golden::read(&golden::committed("bot.timeouts")).expect("the golden reads");
    println!(
        "examined {} case(s) of {}",
        timeouts.cases.len(),
        timeouts.function
    );
    let bot = &timeouts.cases[0].output;
    assert_eq!(bot["long_poll_seconds"], json!(LONG_POLL_SECONDS));
    let http = HTTP_TIMEOUT.as_secs_f64();
    for timeout in ["connect", "read", "write", "pool"] {
        assert_eq!(
            bot["http_timeout_seconds"][timeout],
            json!(http),
            "{timeout}"
        );
    }
    assert!(
        HTTP_TIMEOUT > Duration::from_secs(u64::from(LONG_POLL_SECONDS)),
        "a long poll that returns empty is never cut off"
    );
}

#[tokio::test]
async fn the_transport_never_shows_its_token() {
    let fake = FakeBotApi::start().await;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = fake.transport(directory.path());
    let shown = format!("{transport:?}");
    assert!(!shown.contains(TOKEN), "{shown}");
    assert!(shown.contains("Transport"), "{shown}");
    // Every request names the token, in its URL alone.
    assert!(
        transport.answer_callback("tap-1").await,
        "the answer is taken"
    );
    let calls = fake.calls();
    assert!(
        !calls.is_empty() && calls.iter().all(|call| call.token_ok),
        "{calls:?}"
    );
    assert!(
        !payload(&calls[0]).to_string().contains(TOKEN),
        "no body carries the token"
    );
}

#[tokio::test(start_paused = true)]
async fn the_services_waits_run_on_tokios_timer() {
    // No socket here, so paused time holds: the wait takes its duration on tokio's clock.
    let started = tokio::time::Instant::now();
    TokioTimer.wait(Duration::from_secs(29)).await;
    assert_eq!(started.elapsed(), Duration::from_secs(29));
}

#[tokio::test]
async fn a_failed_request_says_how_it_failed_and_never_what_it_carried() {
    let fake = FakeBotApi::start().await;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = fake.transport(directory.path());
    fake.script(
        "deleteWebhook",
        [Answer::status(409), Answer::Garbage(502), Answer::Silence],
    );
    let refused = transport.delete_webhook().await;
    assert!(
        matches!(refused, Err(TransportError::Api { code: 409 })),
        "a refusal names its error code: {refused:?}"
    );
    let unreadable = transport.delete_webhook().await;
    assert!(
        matches!(unreadable, Err(TransportError::Unreadable)),
        "an answer that is not the Bot API's: {unreadable:?}"
    );
    let silent = transport.delete_webhook().await;
    assert!(
        matches!(silent, Err(TransportError::Http(_))),
        "no answer before the timeout: {silent:?}"
    );
    for error in [refused, unreadable, silent] {
        let shown = format!(
            "{:?} {}",
            error,
            error
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_default()
        );
        assert!(!shown.contains(TOKEN), "{shown}");
    }
    assert!(
        transport.delete_webhook().await.is_ok(),
        "then Telegram's own answer"
    );
}

#[tokio::test]
async fn the_routers_pushes_go_to_the_owners_chat_as_html() {
    use std::sync::Arc;

    use deck_streak_bot::OwnerChat;
    use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, UtcMillis};
    use deck_streak_notifications::{
        Decision, DedupeKey, LapseContext, Occasion, Policy, Reason, Router, Surface, Tier,
    };

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
    let nudge = |key: &str| {
        Occasion::new(
            policy.kind("habit").expect("the habit kind"),
            DedupeKey::new(key).expect("a key"),
            Surface::Bot,
            Tier::T2,
            "<b>A</b> synthetic line",
            StudyDay::from_epoch_day(20_000),
            LapseContext::NoLapse,
        )
        .expect("an occasion")
    };

    let sent = router.route(&nudge("habit:one")).await.expect("a decision");
    fake.script(
        "sendMessage",
        (0..SEND_ATTEMPTS).map(|_| Answer::status(502)),
    );
    let failed = router.route(&nudge("habit:two")).await.expect("a decision");

    assert_eq!(
        (sent, failed),
        (
            Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T2
            },
            Decision::Withheld {
                surface: Surface::Bot,
                reason: Reason::NoNotifier
            }
        ),
        "a refused push is a push that did not answer"
    );
    let calls = fake.calls_of("sendMessage");
    assert_eq!(calls.len(), 1 + SEND_ATTEMPTS as usize);
    assert_eq!(calls[0].body["chat_id"], json!(OWNER), "the owner's chat");
    assert_eq!(
        (
            payload(&calls[0])["text"].clone(),
            payload(&calls[0])["parse_mode"].clone()
        ),
        (json!("<b>A</b> synthetic line"), json!("HTML"))
    );
}
