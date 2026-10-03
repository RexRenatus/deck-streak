//! SPEC-119 A17 to A21: the limiter equals the predecessor's (`mcp_auth.py:DrillAuth.require`,
//! `_bucket_id`, `_is_rate_limited`, `_record_failure` at `27ee2bc`), a granted token never limits
//! itself, and a refusal's event names its outcome, bucket and scope and never the token (R13, R14;
//! T3, T4, T7, T8).
//!
//! Every token is built from parts at run time.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fmt::{self, Write as _};
use std::fs;
use std::sync::{Arc, Mutex, PoisonError};

use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, HeaderValue};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, ManualClock, Redactor, UtcMillis,
};
use deck_streak_mcp::guard::DENIED;
use deck_streak_mcp::limiter::{HISTORY_CAP, MAX_BUCKETS, MAX_FAILURES, WINDOW_MILLIS};
use deck_streak_mcp::{Bucket, Grants, Guard, Outcome, Scope};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

/// The instant every case starts at, in epoch milliseconds.
const START: i64 = 1_760_000_000_000;

/// A guard over a core and a law-track credential holding `core` and `law`, and its clock.
fn guard_over(core: &str, law: &str) -> (Guard, Arc<ManualClock>) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for (id, value) in [("mcp-core-token", core), ("mcp-law-track-token", law)] {
        fs::write(directory.path().join(id), format!("{value}\n")).expect("a credential");
    }
    let path = CredentialsDirectory::new(directory.path()).expect("an absolute path");
    let grants =
        Grants::load(&CredentialLoader::new(path, Redactor::new())).expect("the grants load");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START)));
    (Guard::new(grants, Arc::<ManualClock>::clone(&clock)), clock)
}

/// The core credential's token, built from parts.
fn core_token() -> String {
    format!("limiter-core-{}", "c".repeat(30))
}

/// The law-track credential's token, built from parts.
fn law_token() -> String {
    format!("limiter-law-{}", "l".repeat(30))
}

/// The bucket of the presented bytes, computed here: the first 16 lower-case hexadecimal
/// characters of their SHA-256, the empty value's being that of `\x00absent` (R13, T3).
fn bucket_of(presented: &[u8]) -> String {
    let raw: &[u8] = if presented.is_empty() {
        b"\x00absent"
    } else {
        presented
    };
    let mut hex = String::new();
    for byte in Sha256::digest(raw).iter().take(8) {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Headers presenting `token` as a bearer, or no header when the token is empty (the predecessor
/// was handed an empty token when none was presented).
fn bearer(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if !token.is_empty() {
        headers.append(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).expect("a header value"),
        );
    }
    headers
}

/// One request for `scope`: the layer's admission, then the scope check, as a tool would ask it.
fn require(guard: &Guard, token: &str, scope: Scope) -> Outcome {
    match guard.admit(&bearer(token)) {
        Ok(granted) => match guard.authorize(&granted, scope) {
            Ok(()) => Outcome::Allowed,
            Err(refusal) => refusal.outcome(),
        },
        Err(refusal) => refusal.outcome(),
    }
}

/// A golden token: its parts joined, each a string or a `[character, count]` run.
fn joined(parts: &Value) -> String {
    let mut token = String::new();
    for part in parts.as_array().expect("a token's parts") {
        match part {
            Value::String(text) => token.push_str(text),
            Value::Array(run) => {
                let character = run[0].as_str().expect("a run's character");
                let count = run[1].as_u64().expect("a run's count");
                token.push_str(&character.repeat(usize::try_from(count).expect("a count")));
            }
            other => panic!("a token part {other}"),
        }
    }
    token
}

/// The scope a golden names.
fn scope(name: &str) -> Scope {
    Scope::ALL
        .into_iter()
        .find(|scope| scope.name() == name)
        .unwrap_or_else(|| panic!("the scope {name}"))
}

#[test]
fn the_limiter_matches_its_golden() {
    golden::each_case("mcp_auth_limiter", |case| {
        let grants = case.input["grants"].as_array().expect("the grants");
        let granted: Vec<(String, Value)> = grants
            .iter()
            .map(|grant| (joined(&grant["token"]), grant["scopes"].clone()))
            .collect();
        // The golden's grants are the core credential's and the law-track credential's.
        assert_eq!(
            granted
                .iter()
                .map(|(_, scopes)| scopes.clone())
                .collect::<Vec<_>>(),
            vec![
                serde_json::json!(["core"]),
                serde_json::json!(["core", "law_track"])
            ],
            "{:?}",
            case.class
        );
        let (guard, clock) = guard_over(&granted[0].0, &granted[1].0);

        let mut outcomes = Vec::new();
        for call in case.input["calls"].as_array().expect("the calls") {
            clock.set(UtcMillis::from_epoch_millis(
                call["at"].as_i64().expect("an instant"),
            ));
            let token = joined(&call["token"]);
            let outcome = match call["op"].as_str().expect("an op") {
                "require" => {
                    let name = call["scope"].as_str().expect("a scope");
                    require(&guard, &token, scope(name)).name()
                }
                "record" => {
                    guard.limiter().record(&Bucket::of(token.as_bytes()));
                    "recorded"
                }
                other => panic!("the op {other}"),
            };
            outcomes.push(Value::from(outcome));
        }
        let buckets: Vec<Value> = guard
            .limiter()
            .snapshot()
            .into_iter()
            .map(|(bucket, at)| serde_json::json!([bucket.as_str(), at]))
            .collect();

        assert_eq!(
            Value::from(outcomes),
            case.output["outcomes"],
            "the outcomes of {:?}",
            case.class
        );
        assert_eq!(
            Value::from(buckets),
            case.output["buckets"],
            "the buckets of {:?}",
            case.class
        );
    });
}

#[test]
fn the_bucket_matches_its_golden() {
    let (guard, _clock) = guard_over(&core_token(), &law_token());
    golden::each_case("mcp_auth_bucket", |case| {
        let value = case.input["value"].as_str().expect("a value");
        let expected = case.output.as_str().expect("a bucket");
        let class = case.class.as_deref().expect("a class");

        // A token is presented as a bearer; any other value is a header's whole value.
        let mut headers = HeaderMap::new();
        let header = if class == "token" {
            format!("Bearer {value}")
        } else {
            value.to_owned()
        };
        headers.append(
            AUTHORIZATION,
            HeaderValue::from_bytes(header.as_bytes()).expect("a header value"),
        );
        let refusal = guard.admit(&headers).expect_err("the value is refused");
        assert_eq!(refusal.bucket().as_str(), expected, "{class} {value:?}");
        assert_eq!(
            Bucket::of(value.as_bytes()).as_str(),
            expected,
            "{class} {value:?}"
        );

        // No header at all is the empty value's bucket.
        if class == "empty" {
            let absent = guard
                .admit(&HeaderMap::new())
                .expect_err("no header is refused");
            assert_eq!(absent.bucket().as_str(), expected);
        }
    });
}

#[test]
fn the_guard_constants_match_the_golden() {
    let golden = golden::read(&golden::committed("mcp_auth.constants")).expect("the golden");
    let constant = |name: &str| -> Value {
        golden
            .cases
            .iter()
            .find(|case| case.input["name"] == name)
            .map_or_else(|| panic!("the constant {name}"), |case| case.output.clone())
    };
    let seconds = constant("mcp_auth._RATE_LIMIT_WINDOW_SECS")
        .as_f64()
        .expect("seconds");
    // The window in whole milliseconds, compared as text: the golden's is a float of seconds.
    assert_eq!(
        WINDOW_MILLIS.to_string(),
        (seconds * 1000.0).round().to_string()
    );
    assert_eq!(
        Value::from(MAX_FAILURES),
        constant("mcp_auth._RATE_LIMIT_MAX_FAILURES")
    );
    assert_eq!(
        Value::from(MAX_BUCKETS),
        constant("mcp_auth._RATE_LIMIT_MAX_BUCKETS")
    );
    assert_eq!(
        Value::from(HISTORY_CAP),
        constant("mcp_auth._RATE_LIMIT_BUCKET_HISTORY_CAP")
    );
    assert_eq!(Value::from(DENIED), constant("mcp_auth.DENIED_MESSAGE"));
    assert_eq!(
        Value::from(Scope::LawTrack.name()),
        constant("mcp_auth.SCOPE_LAW_TRACK")
    );
    println!("examined {} constant(s)", golden.cases.len());
    assert_eq!(golden.cases.len(), 6);
}

#[test]
fn a_granted_token_never_limits_itself() {
    let core = core_token();
    let (guard, _clock) = guard_over(&core, &law_token());
    let admitted = guard.admit(&bearer(&core));
    assert!(admitted.is_ok(), "the core token is admitted: {admitted:?}");
    let granted = admitted.expect("admitted");

    // Ten out-of-scope failures through the guard's scope check fill the core token's bucket.
    let failures: Vec<Outcome> = (0..10)
        .map(|_| match guard.authorize(&granted, Scope::LawTrack) {
            Ok(()) => Outcome::Allowed,
            Err(refusal) => refusal.outcome(),
        })
        .collect();
    let mut expected = vec![Outcome::Denied; 5];
    expected.extend([Outcome::RateLimited; 5]);
    assert_eq!(failures, expected);

    // The core token is still admitted by the request layer, and allowed core.
    let again = guard.admit(&bearer(&core));
    assert!(again.is_ok(), "the core token is still admitted: {again:?}");
    let again = again.expect("admitted");
    let core_scope = guard.authorize(&again, Scope::Core);
    assert!(core_scope.is_ok(), "core is still allowed: {core_scope:?}");

    // Its bucket holds the five failures it recorded, and nothing more.
    let buckets: Vec<(String, usize)> = guard
        .limiter()
        .snapshot()
        .into_iter()
        .map(|(bucket, at)| (bucket.as_str().to_owned(), at.len()))
        .collect();
    assert_eq!(buckets, vec![(bucket_of(core.as_bytes()), 5)]);
}

/// A subscriber that keeps every event and span it is given, each as one line of its level,
/// target, name and fields, each field's text as recorded.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<String>>>);

impl Captured {
    fn lines(&self) -> Vec<String> {
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

/// A line of recorded fields.
struct Fields(String);

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        let _ = write!(self.0, " {}={value}", field.name());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let _ = write!(self.0, " {}={value:?}", field.name());
    }
}

impl Subscriber for Captured {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut fields = Fields(format!("span {}", span.metadata().name()));
        span.record(&mut fields);
        self.push(fields.0);
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, values: &Record<'_>) {
        let mut fields = Fields("span record".to_owned());
        values.record(&mut fields);
        self.push(fields.0);
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        let mut fields = Fields(format!("{} {}", metadata.level(), metadata.target()));
        event.record(&mut fields);
        self.push(fields.0);
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

#[test]
fn a_refusal_logs_the_bucket_and_never_the_token() {
    let core = core_token();
    let law = law_token();
    let (guard, _clock) = guard_over(&core, &law);
    let mut wrong = core.clone();
    wrong.pop();
    wrong.push('w');
    let malformed = format!("Digest {core}");

    let captured = Captured::default();
    // Each call's answer is kept, and judged after the events, so the events are judged first.
    let answers = log_capture::with_capture(captured.clone(), || {
        let mut headers = HeaderMap::new();
        headers.append(
            AUTHORIZATION,
            HeaderValue::from_str(&malformed).expect("a header value"),
        );
        let mut answers = vec![
            guard.admit(&headers).is_ok(),
            guard.admit(&HeaderMap::new()).is_ok(),
        ];
        for _ in 0..6 {
            answers.push(guard.admit(&bearer(&wrong)).is_ok());
        }
        match guard.admit(&bearer(&core)) {
            Ok(granted) => {
                answers.push(true);
                answers.push(guard.authorize(&granted, Scope::Core).is_ok());
                answers.push(guard.authorize(&granted, Scope::LawTrack).is_ok());
            }
            Err(_) => answers.push(false),
        }
        answers
    });
    let lines = captured.lines();

    let event_line = |outcome: &str, bucket: &str, scope: &str| {
        format!("WARN deck_streak_mcp::guard outcome={outcome} bucket={bucket} scope={scope}")
    };
    let wrong_bucket = bucket_of(wrong.as_bytes());
    let mut expected = vec![
        event_line("denied", &bucket_of(malformed.as_bytes()), "core"),
        event_line("denied", &bucket_of(b""), "core"),
    ];
    for _ in 0..5 {
        expected.push(event_line("denied", &wrong_bucket, "core"));
    }
    expected.push(event_line("rate_limited", &wrong_bucket, "core"));
    expected.push(event_line(
        "denied",
        &bucket_of(core.as_bytes()),
        "law_track",
    ));
    assert_eq!(lines, expected);
    let mut admitted = vec![false; 8];
    admitted.extend([true, true, false]);
    assert_eq!(
        answers, admitted,
        "eight refusals, then core admitted and allowed core only"
    );

    // No line holds a token, a header's value or a token's full digest.
    let full_digest = |token: &str| {
        let mut hex = String::new();
        for byte in Sha256::digest(token.as_bytes()) {
            let _ = write!(hex, "{byte:02x}");
        }
        hex
    };
    let secrets = [
        core.clone(),
        law.clone(),
        wrong.clone(),
        malformed.clone(),
        full_digest(&core),
        full_digest(&law),
        full_digest(&wrong),
    ];
    println!("examined {} line(s)", lines.len());
    for line in &lines {
        for secret in &secrets {
            assert!(!line.contains(secret.as_str()), "{line}");
        }
    }
}
