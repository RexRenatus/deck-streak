//! Every response carries a request id, an INFO event names each request's matched route and its
//! status, and a header marked sensitive never reaches the log (SPEC-025 A8, A9, R4).
//!
//! What leaves the process is measured on a child process: this test binary run again, running only
//! an ignored test that installs the kernel's logging (the one every role of the daemon installs)
//! and serves requests through `deck_streak_api::layered`, so the subscriber and stdout are a
//! role's own.

// An integration test is test code: its helpers panic on a failed child, and a child reports its
// responses on stdout on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::process::Command;

use axum::Router;
use axum::body::Body;
use axum::http::header::{AUTHORIZATION, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request};
use axum::routing::get;
use deck_streak_api::layered;
use deck_streak_kernel::{Redactor, logging};
use tower::ServiceExt;

/// A session cookie's synthetic value.
const COOKIE_SECRET: &str = "saffron-lantern-quiver";
/// A synthetic value standing for the Mini App's `initData`, which travels as `Authorization: tma`.
const AUTHORIZATION_SECRET: &str = "cobalt-thistle-meadow";
/// A synthetic session cookie a response sets.
const SET_COOKIE_SECRET: &str = "juniper-anvil-harbor";
/// A header value nobody marks sensitive, which the log must show.
const VISIBLE: &str = "plain-probe-value";
/// A query string standing for data a URL may carry, which the log must never show.
const QUERY_SECRET: &str = "tgWebAppData-standin";

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Runs this test binary again, running only the ignored test `child` with `RUST_LOG` at `level`,
/// and returns every line it wrote to stdout.
fn child_stdout(child: &str, level: &str) -> Vec<String> {
    let output = Command::new(std::env::current_exe().expect("this test binary's path"))
        .args([
            "--exact",
            child,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("RUST_LOG", level)
        .output()
        .expect("the child process runs");
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(
        output.status.success(),
        "the child {child} failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    stdout.lines().map(str::to_owned).collect()
}

/// The response report a child printed for `probe`: its status and its request id.
fn reported<'a>(lines: &'a [String], probe: &str) -> Option<(&'a str, &'a str)> {
    let prefix = format!("response probe={probe} ");
    lines.iter().find_map(|line| {
        let rest = line.strip_prefix(&prefix)?;
        let (status, id) = rest.split_once(' ')?;
        Some((
            status.strip_prefix("status=")?,
            id.strip_prefix("request-id=")?,
        ))
    })
}

/// The event lines: each a JSON object after its journal priority.
fn events(lines: &[String]) -> Vec<&str> {
    lines
        .iter()
        .filter(|line| line.len() > 3 && line.starts_with('<') && line[3..].starts_with('{'))
        .map(String::as_str)
        .collect()
}

/// Whether `id` is a request id as `MakeRequestUuid` writes one: a hyphenated UUID.
fn is_uuid(id: &str) -> bool {
    id.len() == 36
        && id.char_indices().all(|(at, c)| match at {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// A handler that panics, as a defect in any route would.
async fn panics() -> &'static str {
    panic!("a synthetic handler panic");
}

#[tokio::test]
#[ignore = "a child process: every_response_carries_a_request_id_and_an_info_event_with_the_matched_route runs it"]
async fn child_serves_requests_under_the_layers() {
    logging::install(&Redactor::new()).expect("the first install in this process");
    let app = layered(
        Router::new()
            .route("/api/test/items/{item}", get(|| async { "item" }))
            .route("/api/test/panic", get(panics)),
    );
    let probes = [
        ("routed", format!("/api/test/items/42?{QUERY_SECRET}=1")),
        ("unrouted", "/api/test/nowhere".to_owned()),
        ("panicked", "/api/test/panic".to_owned()),
    ];
    for (probe, path) in probes {
        let request = Request::get(path)
            .body(Body::empty())
            .expect("a well-formed request");
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("the router is infallible");
        let id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("none");
        println!(
            "response probe={probe} status={} request-id={id}",
            response.status().as_u16()
        );
    }
}

#[test]
fn every_response_carries_a_request_id_and_an_info_event_with_the_matched_route() {
    let lines = child_stdout("child_serves_requests_under_the_layers", "info");
    let responses = examined(
        "response(s)",
        ["routed", "unrouted", "panicked"]
            .into_iter()
            .map(|probe| (probe, reported(&lines, probe)))
            .collect(),
    );
    let events = events(&lines);
    for (probe, report) in responses {
        let (status, id) =
            report.unwrap_or_else(|| panic!("the child reported no {probe} response"));
        // Every response carries a request id, a 404 and a caught panic's 500 included.
        assert!(
            is_uuid(id),
            "the {probe} response carries no request id ({id}):\n{lines:#?}"
        );
        // An INFO event, visible at the default level, closes each request, naming its status and
        // carrying the same request id in its span.
        let finished = events
            .iter()
            .find(|line| line.contains("finished processing request") && line.contains(id));
        let Some(finished) = finished else {
            panic!("no event closes the {probe} request {id}:\n{lines:#?}");
        };
        assert!(finished.starts_with("<6>"), "{finished}");
        assert!(
            finished.contains(&format!("\"status\":{status}")),
            "{finished}"
        );
        assert!(finished.contains("\"method\":\"GET\""), "{finished}");
    }
    // The routed request's span names its matched route, never the path it was asked by.
    let (_, routed) = reported(&lines, "routed").expect("the routed response");
    let routed_event = events
        .iter()
        .find(|line| line.contains("finished processing request") && line.contains(routed))
        .expect("the routed request's event");
    assert!(
        routed_event.contains("\"route\":\"/api/test/items/{item}\""),
        "{routed_event}"
    );
    // No line carries the raw path or its query string: a URL may carry initData.
    for line in &lines {
        assert!(
            !line.contains("/items/42"),
            "a raw path reached the log: {line}"
        );
        assert!(
            !line.contains(QUERY_SECRET),
            "a query string reached the log: {line}"
        );
    }
}

#[tokio::test]
#[ignore = "a child process: sensitive_header_values_never_reach_the_log runs it"]
async fn child_logs_a_request_carrying_sensitive_headers() {
    logging::install(&Redactor::new()).expect("the first install in this process");
    // A route that logs every header it was given, as any handler, or a debug line, could.
    let app = layered(Router::new().route(
        "/api/test/headers",
        get(|headers: HeaderMap| async move {
            tracing::info!(probe = "a9", headers = ?headers, "the route logged its request headers");
            let session = format!("deckstreak_session={SET_COOKIE_SECRET}; Path=/; Secure; HttpOnly");
            ([(SET_COOKIE, session)], "logged")
        }),
    ));
    let request = Request::get("/api/test/headers")
        .header(COOKIE, format!("deckstreak_session={COOKIE_SECRET}"))
        .header(
            AUTHORIZATION,
            format!("tma auth_date=1&hash={AUTHORIZATION_SECRET}"),
        )
        .header("x-probe", VISIBLE)
        .body(Body::empty())
        .expect("a well-formed request");
    let response = app
        .oneshot(request)
        .await
        .expect("the router is infallible");
    let marked = response
        .headers()
        .get(SET_COOKIE)
        .is_some_and(HeaderValue::is_sensitive);
    println!(
        "response probe=a9 status={} set-cookie-sensitive={marked}",
        response.status().as_u16()
    );
}

#[test]
fn sensitive_header_values_never_reach_the_log() {
    // Every level, so no event of any layer or of the route can hide a leak.
    let lines = examined(
        "line(s)",
        child_stdout("child_logs_a_request_carrying_sensitive_headers", "trace"),
    );
    // The route's event was logged, so the check below reads a log that holds the headers.
    let events = events(&lines);
    let logged = events
        .iter()
        .find(|line| line.contains("\"probe\":\"a9\""))
        .unwrap_or_else(|| panic!("the route's event was not logged:\n{lines:#?}"));
    for line in &lines {
        for secret in [COOKIE_SECRET, AUTHORIZATION_SECRET, SET_COOKIE_SECRET] {
            assert!(
                !line.contains(secret),
                "a sensitive header's value reached the log: {line}"
            );
        }
    }
    // The headers were logged, each sensitive one as the mark, the others as written.
    assert!(logged.contains(VISIBLE), "{logged}");
    assert_eq!(logged.matches("Sensitive").count(), 2, "{logged}");
    // The response's cookie is marked before any layer that logs could read it.
    assert!(
        lines
            .iter()
            .any(|line| line == "response probe=a9 status=200 set-cookie-sensitive=true"),
        "{lines:#?}"
    );
}
