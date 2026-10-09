//! The static library reads the sync service's statement of its minimum client level before every
//! sync login (SPEC-374 A4, A5; ADR-385 D6).
//!
//! Each test logs in through [`Engine::run`] as the app does, at the endpoint of a recording
//! loopback server: a listener that records each request line it is sent and answers the
//! statement's path as the case says, and every other path with a 503. The text a person reads is
//! the message the app decodes from the refusal's `BackendError`, field 1, and every sentence
//! expected is written here from SPEC-374 R6, never read from the core.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

#[expect(
    dead_code,
    reason = "the handshake tests encode and decode with the wire helpers alone, so they build no \
              synthetic collection"
)]
mod support;

use std::io::{Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use deck_streak_ffi::engine::{Engine, EngineRefusal};
use support::wire;

/// The engine's login call, `BackendSyncService.SyncLogin`.
const SYNC_LOGIN: (u32, u32) = (1, 3);
/// `BackendError.Kind.INVALID_INPUT`, the core's refusal shape: absent on the wire, so 0.
const INVALID_INPUT: u64 = 0;

/// The request line of the statement's read.
const STATEMENT_READ: &str = "GET /api/sync/minimum-client HTTP/1.1";
/// The path every sync request of the engine's sits under.
const SYNC_POST: &str = "POST /sync/";

/// A statement whose minimum is above the level this tree builds.
const BELOW_ANSWER: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                            Content-Length: 26\r\nConnection: close\r\n\r\n\
                            {\"minimum_client_level\":2}";
/// A statement whose minimum equals the level this tree builds: the edge that still admits.
const ADMITTING_ANSWER: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                                Content-Length: 26\r\nConnection: close\r\n\r\n\
                                {\"minimum_client_level\":1}";
/// A redirect from the statement's path to a path that answers an admitting statement.
const REDIRECT_ANSWER: &str = "HTTP/1.1 302 Found\r\nLocation: /admitting\r\n\
                               Content-Length: 0\r\nConnection: close\r\n\r\n";
/// What the recording server answers every path but the statement's: a refusal of its own.
const UNAVAILABLE_ANSWER: &str =
    "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// R6's sentence for a client below the minimum.
const BELOW: &str = "This version of DeckStreak is older than the oldest the sync service accepts. Update DeckStreak to sync.";
/// R6's sentence when a statement was read and does not decode.
const UNDECODABLE: &str = "The sync service's statement of the oldest version it accepts could not be read, so nothing was synced.";
/// The beginning of R6's sentence when no statement was read, which the app's UI test reads.
const NETWORK: &str = "A network error occurred.";
/// The login guard's rule for plain http to anything but a loopback address (SPEC-347 R2).
const SCHEME: &str =
    "the sync login's endpoint is neither https nor plain http to a loopback address";

/// A synthetic account: no server here knows it.
const USER: &str = "learner-one";
/// The synthetic account's password.
const SECRET: &str = "hunter-two";

/// A loopback listener that records the request line of every connection it is sent, answers the
/// statement's path with `statement` and every other path with a 503, and closes each connection.
struct Recorder {
    address: SocketAddr,
    lines: Arc<Mutex<Vec<String>>>,
    stopped: Arc<AtomicBool>,
    serving: Option<JoinHandle<()>>,
}

impl Recorder {
    fn start(statement: &'static str) -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .expect("the recorder binds a loopback port");
        let address = listener.local_addr().expect("the recorder's address reads");
        let lines = Arc::new(Mutex::new(Vec::new()));
        let stopped = Arc::new(AtomicBool::new(false));
        let (record, stop) = (Arc::clone(&lines), Arc::clone(&stopped));
        let serving = thread::spawn(move || {
            for connection in listener.incoming() {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(connection) = connection {
                    serve(connection, statement, &record);
                }
            }
        });
        Self {
            address,
            lines,
            stopped,
            serving: Some(serving),
        }
    }

    /// The endpoint a login names to reach this listener by its loopback address.
    fn endpoint(&self) -> String {
        format!("http://{}/", self.address)
    }

    /// The request lines recorded so far, in the order the connections arrived.
    fn lines(&self) -> Vec<String> {
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        if let Some(serving) = self.serving.take() {
            let _ = serving.join();
        }
    }
}

/// Records one connection's request line and answers it.
fn serve(mut connection: TcpStream, statement: &str, record: &Mutex<Vec<String>>) {
    let _ = connection.set_read_timeout(Some(Duration::from_secs(5)));
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16_384 {
        match connection.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => break,
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let line = head.lines().next().unwrap_or_default().to_owned();
    if line.is_empty() {
        return;
    }
    let answer = match line.split(' ').nth(1) {
        Some("/api/sync/minimum-client") => statement,
        Some("/admitting") => ADMITTING_ANSWER,
        _ => UNAVAILABLE_ANSWER,
    };
    record
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(line);
    let _ = connection.write_all(answer.as_bytes());
    let _ = connection.shutdown(Shutdown::Write);
    // The rest of the request is read and dropped, so closing sends no reset ahead of the answer.
    let _ = std::io::copy(&mut connection, &mut std::io::sink());
}

/// A loopback endpoint on a port nothing listens on.
fn closed_endpoint() -> String {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port to close again");
    let port = listener.local_addr().expect("the bound port reads").port();
    drop(listener);
    format!("http://{}:{port}/", Ipv4Addr::LOCALHOST)
}

fn engine() -> Arc<Engine> {
    Engine::new(Vec::new()).expect("the engine starts from the default init message")
}

/// `SyncLoginRequest`: the user (field 1), the password (2) and the endpoint (3).
fn login_request(endpoint: &str) -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_bytes(&mut out, 1, USER.as_bytes());
    wire::put_bytes(&mut out, 2, SECRET.as_bytes());
    wire::put_bytes(&mut out, 3, endpoint.as_bytes());
    out
}

/// A login through the adapter at `endpoint`: the refusal's kind and the message the app shows,
/// or what else it answered.
fn logged_in(endpoint: &str) -> Result<(u64, String), String> {
    match engine().run(SYNC_LOGIN.0, SYNC_LOGIN.1, login_request(endpoint)) {
        Err(EngineRefusal::Engine { error }) => Ok((
            wire::varint(&error, 2),
            String::from_utf8(wire::bytes(&error, 1)).expect("a string field is UTF-8"),
        )),
        other => Err(format!("not the engine's refusal: {other:?}")),
    }
}

#[test]
fn a_login_below_the_minimum_stops_before_any_sync_request_and_says_why() {
    let recorder = Recorder::start(BELOW_ANSWER);

    let refused = logged_in(&recorder.endpoint());

    assert_eq!(
        recorder.lines(),
        vec![STATEMENT_READ.to_owned()],
        "below the minimum, the statement's GET is the only request: nothing under the sync path"
    );
    assert_eq!(
        refused,
        Ok((INVALID_INPUT, BELOW.to_owned())),
        "the app shows the below sentence"
    );
}

#[test]
fn the_statement_is_read_first_and_only_an_admitting_one_reaches_the_login() {
    let admitting = Recorder::start(ADMITTING_ANSWER);
    let reached = logged_in(&admitting.endpoint());
    let lines = admitting.lines();
    assert_eq!(
        lines.first().map(String::as_str),
        Some(STATEMENT_READ),
        "the statement's GET comes first: {lines:?}"
    );
    assert!(
        lines.get(1).is_some_and(|line| line.starts_with(SYNC_POST)),
        "an admitting statement is followed by the login's request under the sync path: {lines:?}"
    );
    assert!(
        reached
            .as_ref()
            .is_ok_and(|(kind, _)| *kind != INVALID_INPUT),
        "the login reached the recorder, whose 503 the engine refuses in its own kind: {reached:?}"
    );

    let unanswered = logged_in(&closed_endpoint());
    assert!(
        unanswered
            .as_ref()
            .is_ok_and(|(kind, message)| *kind == INVALID_INPUT && message.starts_with(NETWORK)),
        "a statement nobody answered refuses with the network sentence: {unanswered:?}"
    );

    let redirecting = Recorder::start(REDIRECT_ANSWER);
    let redirected = logged_in(&redirecting.endpoint());
    assert_eq!(
        (redirecting.lines(), redirected),
        (
            vec![STATEMENT_READ.to_owned()],
            Ok((INVALID_INPUT, UNDECODABLE.to_owned()))
        ),
        "a redirect is not followed: its status is not 200, so the statement is undecodable"
    );

    let guarded = Recorder::start(ADMITTING_ANSWER);
    let named = format!("http://localhost:{}/", guarded.address.port());
    let refused = logged_in(&named);
    assert_eq!(
        (guarded.lines(), refused),
        (Vec::<String>::new(), Ok((INVALID_INPUT, SCHEME.to_owned()))),
        "an endpoint the guard refuses is never read, and the login keeps the guard's sentence"
    );
}

/// An admitting statement, `{"minimum_client_level":1}`, padded with JSON whitespace to `length`
/// bytes of body, answered whole with its length stated.
fn padded_answer(length: usize) -> &'static str {
    let statement = r#"{"minimum_client_level":1}"#;
    let body = format!("{statement}{}", " ".repeat(length - statement.len()));
    let answer = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {length}\r\n\
         Connection: close\r\n\r\n{body}"
    );
    Box::leak(answer.into_boxed_str())
}

/// MUTATION COVERAGE (SPEC-374 R8, ADR-385 D6), written after the implementation and green at it:
/// the read keeps a body of up to 1024 bytes, so an admitting statement of exactly 1024 reaches
/// the login, and a body one byte over is dropped whole, so the same statement decides undecodable
/// and nothing under the sync path is sent.
#[test]
fn the_statement_read_keeps_a_body_at_its_cap_and_drops_one_over_it() {
    let at_cap = Recorder::start(padded_answer(1024));
    let reached = logged_in(&at_cap.endpoint());
    let lines = at_cap.lines();
    assert!(
        lines.first().map(String::as_str) == Some(STATEMENT_READ)
            && lines.get(1).is_some_and(|line| line.starts_with(SYNC_POST)),
        "an admitting statement of exactly 1024 bytes is kept and the login follows it: {lines:?}"
    );
    assert!(
        reached
            .as_ref()
            .is_ok_and(|(kind, _)| *kind != INVALID_INPUT),
        "the login reached the recorder, whose 503 the engine refuses in its own kind: {reached:?}"
    );

    let over_cap = Recorder::start(padded_answer(1025));
    let refused = logged_in(&over_cap.endpoint());
    assert_eq!(
        (over_cap.lines(), refused),
        (
            vec![STATEMENT_READ.to_owned()],
            Ok((INVALID_INPUT, UNDECODABLE.to_owned()))
        ),
        "a body one byte over the cap is dropped whole: the statement is undecodable"
    );
}
