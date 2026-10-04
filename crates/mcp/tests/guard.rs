//! SPEC-119 A9 to A12, A15 and A40: the guard's layer answers every request that presents no
//! granted bearer before its inner service is called, with one response for every cause, and lets
//! a granted bearer through carrying its grant's scopes (R9 to R12; T13). A13 and A14 (section
//! 14): through the served stack, each grant reaches the tools of its scopes and no other, and a
//! tool outside the scope answers the tool error `unauthorized` and no data (R12, R17).
//!
//! Every token is built from parts at run time.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

mod support;

use std::convert::Infallible;
use std::fmt::Write as _;
use std::fs;
use std::sync::{Arc, Mutex, PoisonError};

use axum::body::{Body, to_bytes};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{HeaderMap, HeaderValue, Request, Response, StatusCode};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, ManualClock, Redactor, UtcMillis,
};
use deck_streak_mcp::{DENIED, Granted, Grants, Guard, GuardLayer, Outcome, Scope};
use sha2::{Digest, Sha256};
use tower::{Layer, ServiceExt, service_fn};

/// The most bytes a test reads of a response body.
const BODY_READ_LIMIT: usize = 4096;

/// The core credential's token, built from parts.
fn core_token() -> String {
    format!("guard-core-{}", "c".repeat(30))
}

/// The law-track credential's token, built from parts.
fn law_token() -> String {
    format!("guard-law-{}", "l".repeat(30))
}

/// A guard over the core and law-track credentials, and the clock its limiter reads.
fn guard() -> (Arc<Guard>, Arc<ManualClock>) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for (id, value) in [
        ("mcp-core-token", core_token()),
        ("mcp-law-track-token", law_token()),
    ] {
        fs::write(directory.path().join(id), format!("{value}\n")).expect("a credential");
    }
    let path = CredentialsDirectory::new(directory.path()).expect("an absolute path");
    let grants =
        Grants::load(&CredentialLoader::new(path, Redactor::new())).expect("the grants load");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
        1_760_000_000_000,
    )));
    let guard = Arc::new(Guard::new(grants, Arc::<ManualClock>::clone(&clock)));
    (guard, clock)
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

/// What the layer answered, and what its inner service saw: one entry for each call, the scope
/// names of the grant the request carried (`None` when it carried none).
#[derive(Debug)]
struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
    seen: Vec<Option<Vec<&'static str>>>,
}

/// Sends `request` through the guard's layer around a service that records what it is given.
async fn send(guard: &Arc<Guard>, request: Request<Body>) -> Answer {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    let inner = service_fn(move |request: Request<Body>| {
        let record = Arc::clone(&record);
        async move {
            let scopes = request
                .extensions()
                .get::<Granted>()
                .map(|granted| granted.scopes().names());
            record
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(scopes);
            Ok::<_, Infallible>(Response::new(Body::from("served")))
        }
    });
    let response = GuardLayer::new(Arc::clone(guard))
        .layer(inner)
        .oneshot(request)
        .await
        .unwrap_or_else(|never| match never {});
    let (parts, body) = response.into_parts();
    let body = to_bytes(body, BODY_READ_LIMIT).await.expect("a body");
    let seen = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    Answer {
        status: parts.status,
        headers: parts.headers,
        body: body.to_vec(),
        seen,
    }
}

/// A JSON-RPC request to the MCP path, with `authorization` as its headers' values, in order.
fn request(method: &str, authorization: &[HeaderValue]) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .body(Body::from(format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":{{}}}}"#
        )))
        .expect("a request");
    for value in authorization {
        request.headers_mut().append(AUTHORIZATION, value.clone());
    }
    request
}

/// A header value of these bytes.
fn value(bytes: &[u8]) -> HeaderValue {
    HeaderValue::from_bytes(bytes).expect("a header value")
}

/// Asserts `answer` is the one refusal, R11's: 401, `WWW-Authenticate: Bearer`, the body
/// `unauthorized` and nothing else, and that the inner service never saw the request.
fn assert_refused(answer: &Answer, what: &str) {
    assert_eq!(
        answer.status,
        StatusCode::UNAUTHORIZED,
        "{what}: {answer:?}"
    );
    assert_eq!(
        answer.headers.get(WWW_AUTHENTICATE),
        Some(&HeaderValue::from_static("Bearer")),
        "{what}: {answer:?}"
    );
    assert_eq!(answer.headers.len(), 1, "{what}: {answer:?}");
    assert_eq!(answer.body, b"unauthorized", "{what}: {answer:?}");
    assert_eq!(answer.seen, Vec::new(), "{what}: the inner service saw it");
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_with_no_bearer_is_refused() {
    let (guard, _clock) = guard();
    for method in ["initialize", "tools/list", "tools/call"] {
        let answer = send(&guard, request(method, &[])).await;
        assert_refused(&answer, &format!("{method} with no Authorization header"));
    }

    // And the refusal went to the absent bucket.
    let refusal = guard
        .admit(&HeaderMap::new())
        .expect_err("no header is refused");
    assert_eq!(refusal.bucket().as_str(), bucket_of(b""));
}

#[tokio::test(flavor = "current_thread")]
async fn a_malformed_bearer_is_refused() {
    let (guard, _clock) = guard();
    let core = core_token();
    let (head, tail) = core.split_at(10);
    let cases: Vec<(&str, Vec<HeaderValue>)> = vec![
        ("an empty header", vec![value(b"")]),
        ("Bearer alone", vec![value(b"Bearer")]),
        ("an empty token", vec![value(b"Bearer ")]),
        (
            "another scheme of six letters",
            vec![value(format!("Digest {core}").as_bytes())],
        ),
        (
            "another scheme",
            vec![value(format!("Basic {core}").as_bytes())],
        ),
        (
            "two headers",
            vec![
                value(format!("Bearer {core}").as_bytes()),
                value(format!("Bearer {core}").as_bytes()),
            ],
        ),
        (
            "a token holding a space",
            vec![value(format!("Bearer {head} {tail}").as_bytes())],
        ),
        (
            "two spaces before the token",
            vec![value(format!("Bearer  {core}").as_bytes())],
        ),
        (
            "a trailing space",
            vec![value(format!("Bearer {core} ").as_bytes())],
        ),
        (
            "a token holding a non-ASCII byte",
            vec![value(format!("Bearer {core}\u{e9}").as_bytes())],
        ),
    ];
    for (what, authorization) in &cases {
        let answer = send(&guard, request("initialize", authorization)).await;
        assert_refused(&answer, what);

        // No token parsed, so the refusal's bucket is the first header's whole value's (T3).
        let mut headers = HeaderMap::new();
        for header in authorization {
            headers.append(AUTHORIZATION, header.clone());
        }
        let refusal = guard
            .admit(&headers)
            .expect_err("a malformed bearer is refused");
        assert_eq!(
            refusal.bucket().as_str(),
            bucket_of(authorization[0].as_bytes()),
            "{what}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_wrong_bearer_is_refused() {
    let (guard, _clock) = guard();
    let core = core_token();
    let mut last = core.clone();
    last.pop();
    last.push('x');
    let prefix = core[..core.len() - 1].to_owned();
    let longer = format!("{core}c");
    for (what, token) in [
        ("a token differing in its last character", &last),
        ("a prefix of a granted token", &prefix),
        ("a granted token with one character added", &longer),
    ] {
        let header = value(format!("Bearer {token}").as_bytes());
        let answer = send(&guard, request("initialize", &[header])).await;
        assert_refused(&answer, what);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn every_request_refusal_is_the_same_response() {
    let (guard, _clock) = guard();
    let core = core_token();
    let mut wrong = core.clone();
    wrong.pop();
    wrong.push('w');
    let wrong_header = value(format!("Bearer {wrong}").as_bytes());

    let absent = send(&guard, request("initialize", &[])).await;
    assert_refused(&absent, "absent");
    let mut answers = vec![
        (
            "empty",
            send(&guard, request("initialize", &[value(b"")])).await,
        ),
        (
            "malformed",
            send(
                &guard,
                request("initialize", &[value(format!("Digest {core}").as_bytes())]),
            )
            .await,
        ),
    ];
    // Five wrong attempts fill the wrong token's bucket, so the sixth is limited.
    for _ in 0..5 {
        answers.push((
            "wrong",
            send(
                &guard,
                request("initialize", std::slice::from_ref(&wrong_header)),
            )
            .await,
        ));
    }
    let mut headers = HeaderMap::new();
    headers.append(AUTHORIZATION, wrong_header.clone());
    let limited = guard
        .admit(&headers)
        .expect_err("the wrong token is refused");
    assert_eq!(limited.outcome(), Outcome::RateLimited);
    answers.push((
        "rate-limited",
        send(&guard, request("initialize", &[wrong_header])).await,
    ));

    for (what, answer) in &answers {
        assert_eq!(answer.status, absent.status, "{what}");
        assert_eq!(answer.headers, absent.headers, "{what}");
        assert_eq!(answer.body, absent.body, "{what}");
        assert_eq!(answer.seen, Vec::new(), "{what}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_token_in_the_query_string_is_ignored() {
    let (guard, _clock) = guard();
    let core = core_token();
    for uri in [
        format!("/mcp?token={core}"),
        format!("/mcp?access_token={core}&authorization=Bearer%20{core}"),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri(uri.as_str())
            .body(Body::empty())
            .expect("a request");
        let answer = send(&guard, request).await;
        assert_refused(&answer, &uri);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn each_grant_holds_its_scopes_and_no_other() {
    let (guard, _clock) = guard();
    let core = core_token();
    let law = law_token();

    // Through the layer, each granted bearer reaches the inner service once, carrying its grant.
    for (what, header, scopes) in [
        ("the core token", format!("Bearer {core}"), vec!["core"]),
        (
            "the core token, its scheme in capitals",
            format!("BEARER {core}"),
            vec!["core"],
        ),
        (
            "the law-track token, its scheme in lower case",
            format!("bearer {law}"),
            vec!["core", "law_track"],
        ),
    ] {
        let answer = send(&guard, request("tools/list", &[value(header.as_bytes())])).await;
        assert_eq!(answer.seen, vec![Some(scopes)], "{what}: {answer:?}");
        assert_eq!(answer.status, StatusCode::OK, "{what}");
        assert_eq!(answer.body, b"served", "{what}");
    }

    // The guard's scope check allows each grant exactly its own scopes.
    for (token, law_track_allowed) in [(&core, false), (&law, true)] {
        let mut headers = HeaderMap::new();
        headers.append(AUTHORIZATION, value(format!("Bearer {token}").as_bytes()));
        let granted = guard.admit(&headers).expect("a granted token is admitted");
        assert!(guard.authorize(&granted, Scope::Core).is_ok());
        let law_track = guard.authorize(&granted, Scope::LawTrack);
        assert_eq!(law_track.is_ok(), law_track_allowed, "{law_track:?}");
        if let Err(refusal) = law_track {
            assert_eq!(refusal.outcome(), Outcome::Denied);
            assert_eq!(refusal.bucket().as_str(), bucket_of(token.as_bytes()));
        }
    }
}

/// The answer of a `get_law_track` call over the served stack with `token`, and the reads the law
/// track saw.
async fn law_track_call(token: &str) -> (serde_json::Value, usize, support::Served) {
    let law = support::ScriptedLaw::answering(support::full_block());
    let served = support::Served::start(Arc::clone(&law)).await;
    let reply = served
        .post("/mcp", Some(token), &support::call_law_track())
        .await;
    assert_eq!(
        reply.status,
        200,
        "a granted call is answered: {}",
        reply.text()
    );
    (reply.result(), law.reads(), served)
}

#[tokio::test(flavor = "current_thread")]
async fn each_grant_reaches_its_scopes_and_no_other() {
    // The law-track token reaches `get_law_track`, which reads the law track once.
    let (allowed, reads, _) = law_track_call(&support::law_token()).await;
    assert_ne!(
        allowed["isError"],
        serde_json::Value::Bool(true),
        "the law-track token: {allowed}"
    );
    assert!(
        allowed["structuredContent"].is_object(),
        "the law-track token gets the law track: {allowed}"
    );
    assert_eq!(reads, 1, "the law-track token's call reads once");

    // The core token reaches no `law_track` tool: the call is answered, and reads nothing.
    let (refused, reads, _) = law_track_call(&support::core_token()).await;
    assert_eq!(
        refused["isError"],
        serde_json::Value::Bool(true),
        "the core token: {refused}"
    );
    assert_eq!(reads, 0, "the core token's call read the law track");
}

#[tokio::test(flavor = "current_thread")]
async fn a_tool_outside_the_scope_answers_unauthorized() {
    let core = support::core_token();
    let (result, reads, served) = law_track_call(&core).await;

    // The tool error whose whole text is the one refusal word, and no data.
    assert_eq!(
        result["isError"],
        serde_json::Value::Bool(true),
        "a core-token call: {result}"
    );
    let content = result["content"].as_array().expect("the error's content");
    assert_eq!(content.len(), 1, "one content block: {result}");
    assert_eq!(content[0]["type"], "text", "{result}");
    assert_eq!(content[0]["text"], DENIED, "{result}");
    assert!(
        result.get("structuredContent").is_none(),
        "a refused call carries data: {result}"
    );
    assert_eq!(reads, 0, "a refused call read the law track");

    // The refusal went through the limiter, in the core token's bucket (R13).
    let buckets: Vec<(String, usize)> = served
        .guard
        .limiter()
        .snapshot()
        .into_iter()
        .map(|(bucket, failures)| (bucket.as_str().to_owned(), failures.len()))
        .collect();
    assert_eq!(buckets, vec![(bucket_of(core.as_bytes()), 1)]);
}
