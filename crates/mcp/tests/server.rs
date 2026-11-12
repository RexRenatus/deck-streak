//! SPEC-119 A22 to A25 and A43 to A45 (section 14): the served stack answers only at `/mcp`, caps a
//! request body at 64 KiB, refuses a foreign `Host`, sheds a ninth request in flight at once,
//! refuses a request without a granted bearer before it takes a slot, and answers `initialize` as
//! JSON with no session id (R2 to R4, R11, R12; ADR-329 D4, D5).
//!
//! Every test serves the production router on a loopback port and bounds every wait, so a request
//! that queues fails its test rather than holding the suite. Every token is built from parts.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

mod support;

use std::sync::Arc;

use serde_json::Value;
use support::{
    BOUND, Reply, ScriptedLaw, Served, call_law_track, core_token, full_block, initialize,
    law_token, list_tools,
};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

/// The requests the stack serves at once (R4), stated here as the number the SPEC states.
const IN_FLIGHT: usize = 8;

/// The largest request body the stack reads (R2): 64 KiB.
const BODY_CAP: usize = 64 * 1024;

/// A `tools/list` request padded to exactly `size` bytes with the white space JSON allows after a
/// value, so the bytes the cap counts are the only thing that differs.
fn padded_list(size: usize) -> Vec<u8> {
    let mut body = list_tools();
    let fill = size
        .checked_sub(body.len())
        .expect("the size holds the request");
    body.resize(body.len() + fill, b' ');
    assert_eq!(body.len(), size, "the padded request's length");
    body
}

/// `count` law-track calls sent with the law-track token, each held in the law track until the
/// gate opens.
fn held(served: &Arc<Served>, count: usize) -> Vec<JoinHandle<Reply>> {
    (0..count)
        .map(|_| {
            let served = Arc::clone(served);
            tokio::spawn(async move {
                served
                    .post("/mcp", Some(&law_token()), &call_law_track())
                    .await
            })
        })
        .collect()
}

/// Opens the gate and requires every held call to answer 200 within the bound.
async fn release(gate: &Semaphore, calls: Vec<JoinHandle<Reply>>) {
    gate.add_permits(calls.len());
    for call in calls {
        let reply = tokio::time::timeout(BOUND, call)
            .await
            .expect("a held call answers once released")
            .expect("the held call's task");
        assert_eq!(reply.status, 200, "a released call: {}", reply.text());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn only_the_mcp_path_is_served() {
    let served = Served::start(ScriptedLaw::answering(full_block())).await;

    // The one path answers.
    let mcp = served
        .post("/mcp", Some(&core_token()), &list_tools())
        .await;
    assert_eq!(mcp.status, 200, "/mcp: {}", mcp.text());

    // Every other path answers 404, the predecessor's other transports' paths among them.
    for path in ["/", "/mcp/", "/mcp/tools", "/sse", "/messages", "/api/law"] {
        let other = served.post(path, Some(&core_token()), &list_tools()).await;
        assert_eq!(other.status, 404, "{path}: {}", other.text());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_body_over_64_kib_is_refused() {
    let served = Served::start(ScriptedLaw::answering(full_block())).await;

    // One byte over the cap is refused, 413, whatever the request holds.
    let over = served
        .post("/mcp", Some(&core_token()), &padded_list(BODY_CAP + 1))
        .await;
    assert_eq!(
        over.status,
        413,
        "a body of 64 KiB and one byte: {}",
        over.text()
    );

    // A body of exactly the cap is read and answered.
    let at = served
        .post("/mcp", Some(&core_token()), &padded_list(BODY_CAP))
        .await;
    assert_eq!(at.status, 200, "a body of 64 KiB: {}", at.text());
}

#[tokio::test(flavor = "current_thread")]
async fn a_foreign_host_is_refused() {
    let served = Served::start(ScriptedLaw::answering(full_block())).await;
    let port = served.address.port();

    // A name that is not a loopback name is refused, with or without a port: a page that rebinds a
    // name to the loopback address reaches nothing.
    for host in [
        format!("example.invalid:{port}"),
        "example.invalid".to_owned(),
        format!("localhost.example.invalid:{port}"),
    ] {
        let foreign = served
            .post_as(&host, "/mcp", Some(&core_token()), &initialize())
            .await;
        assert_eq!(foreign.status, 403, "Host {host}: {}", foreign.text());
    }

    // Each loopback name is admitted.
    for host in [
        format!("localhost:{port}"),
        served.address.to_string(),
        format!("[::1]:{port}"),
    ] {
        let local = served
            .post_as(&host, "/mcp", Some(&core_token()), &initialize())
            .await;
        assert_eq!(local.status, 200, "Host {host}: {}", local.text());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn the_ninth_request_in_flight_is_shed() {
    let gate = Arc::new(Semaphore::new(0));
    let law = ScriptedLaw::holding(full_block(), Arc::clone(&gate));
    let served = Arc::new(Served::start(Arc::clone(&law)).await);

    let calls = held(&served, IN_FLIGHT);
    assert!(
        law.entered(IN_FLIGHT).await,
        "only {} of {IN_FLIGHT} calls reached the law track within {BOUND:?}",
        law.reads()
    );

    // A ninth granted request answers 503 at once, inside the bound, and never reaches a tool.
    let ninth = served
        .post("/mcp", Some(&core_token()), &list_tools())
        .await;
    assert_eq!(ninth.status, 503, "the ninth request: {}", ninth.text());

    release(&gate, calls).await;

    // With the eight answered, the bound is free again.
    let after = served
        .post("/mcp", Some(&core_token()), &list_tools())
        .await;
    assert_eq!(after.status, 200, "after the release: {}", after.text());
}

#[tokio::test(flavor = "current_thread")]
async fn the_served_stack_refuses_initialize_without_a_bearer() {
    let law = ScriptedLaw::answering(full_block());
    let served = Served::start(Arc::clone(&law)).await;

    let refused = served.post("/mcp", None, &initialize()).await;
    assert_eq!(
        refused.status,
        401,
        "initialize without a bearer: {}",
        refused.text()
    );
    assert_eq!(refused.text(), "unauthorized");
    assert_eq!(refused.header("www-authenticate"), Some("Bearer"));

    // A wrong bearer is refused the same way, and a tool call reaches no tool.
    let wrong = format!("served-wrong-{}", "w".repeat(30));
    let call = served.post("/mcp", Some(&wrong), &call_law_track()).await;
    assert_eq!(
        call.status,
        401,
        "a call with a wrong bearer: {}",
        call.text()
    );
    assert_eq!(call.text(), "unauthorized");
    assert_eq!(law.reads(), 0, "a refused call read the law track");
}

#[tokio::test(flavor = "current_thread")]
async fn a_refused_request_holds_no_slot() {
    let gate = Arc::new(Semaphore::new(0));
    let law = ScriptedLaw::holding(full_block(), Arc::clone(&gate));
    let served = Arc::new(Served::start(Arc::clone(&law)).await);

    let calls = held(&served, IN_FLIGHT);
    assert!(
        law.entered(IN_FLIGHT).await,
        "only {} of {IN_FLIGHT} calls reached the law track within {BOUND:?}",
        law.reads()
    );

    // With every slot held, a request with no bearer is refused by the guard, 401, not shed.
    for attempt in 0..3 {
        let refused = served.post("/mcp", None, &initialize()).await;
        assert_eq!(
            refused.status,
            401,
            "refusal {attempt} with every slot held: {}",
            refused.text()
        );
    }

    release(&gate, calls).await;
}

#[tokio::test(flavor = "current_thread")]
async fn initialize_answers_json_and_no_session_id() {
    let served = Served::start(ScriptedLaw::answering(full_block())).await;

    let answer = served
        .post("/mcp", Some(&core_token()), &initialize())
        .await;
    assert_eq!(answer.status, 200, "initialize: {}", answer.text());
    let content_type = answer.header("content-type").unwrap_or_default();
    assert!(
        content_type.starts_with("application/json"),
        "initialize answered {content_type:?}"
    );
    assert_eq!(
        answer.header("mcp-session-id"),
        None,
        "initialize issued a session id"
    );
    let result = answer.result();
    assert!(
        result["capabilities"]["tools"].is_object(),
        "the server declares its tools: {result}"
    );
    assert_eq!(
        result["protocolVersion"],
        Value::from("2025-06-18"),
        "the negotiated version: {result}"
    );

    // A second request carries no session and is served on its own.
    let list = served
        .post("/mcp", Some(&core_token()), &list_tools())
        .await;
    assert_eq!(
        list.status,
        200,
        "a request with no session: {}",
        list.text()
    );
    assert_eq!(list.header("mcp-session-id"), None);
}

#[tokio::test(flavor = "current_thread")]
async fn serve_answers_a_request_before_its_shutdown_resolves_then_returns() {
    let law = ScriptedLaw::answering(full_block());
    let guard = support::guard();
    let router = deck_streak_mcp::server::router(law, Arc::clone(&guard));
    let listener = deck_streak_mcp::server::bind(
        deck_streak_mcp::ListenAddress::loopback(std::net::SocketAddr::from((
            std::net::Ipv4Addr::LOCALHOST,
            0,
        )))
        .expect("a loopback address"),
    )
    .await
    .expect("a loopback port");
    let address = listener.local_addr().expect("the bound address");
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let serving = tokio::spawn(deck_streak_mcp::server::serve(
        listener,
        router,
        async move {
            drop(stopped.await);
        },
    ));
    let served = Served { address, guard };

    let answer = served
        .post("/mcp", Some(&core_token()), &initialize())
        .await;
    assert_eq!(answer.status, 200, "initialize: {}", answer.text());
    assert!(
        !serving.is_finished(),
        "serve returned before its shutdown resolved"
    );

    stop.send(()).expect("serve still waits on its shutdown");
    let finished = tokio::time::timeout(BOUND, serving)
        .await
        .expect("serve returns within the bound once its shutdown resolves")
        .expect("the serving task does not panic");
    finished.expect("serve ends without an error");
}
