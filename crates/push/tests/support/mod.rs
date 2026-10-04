//! The push tests' support (SPEC-343 R10): keys made in memory, recording fakes on loopback ports,
//! the fakes' own token verification and RFC 8291 decryption, and the clock every sender reads.

#![allow(
    dead_code,
    reason = "each test target includes this module and calls the part it needs"
)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test support panics like a test (clippy.toml), but clippy's allow-*-in-tests reaches \
              only #[test] functions, not a support module's helpers"
)]

pub mod fake_apns;
pub mod fake_push_service;
pub mod keys;
pub mod rfc8291;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::{StatusCode, Version};
use axum::response::{IntoResponse, Response};
use base64ct::{Base64UrlUnpadded, Encoding};
use deck_streak_kernel::{ManualClock, UtcMillis};
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::Notify;

/// The instant every test's clock starts at, in epoch milliseconds: a synthetic, round instant.
pub const START: i64 = 1_800_000_000_000;
/// The deadline of every sender a test builds that does not test the deadline.
pub const DEADLINE: Duration = Duration::from_secs(10);
/// How long a test waits for a request it expects before it fails, rather than hang.
const ARRIVAL_BOUND: Duration = Duration::from_secs(10);

/// A manual clock at [`START`].
pub fn clock() -> Arc<ManualClock> {
    Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START)))
}

/// One request a fake received.
#[derive(Clone, Debug)]
pub struct Recorded {
    /// The method.
    pub method: String,
    /// The path.
    pub path: String,
    /// The HTTP version it arrived over.
    pub version: Version,
    /// Every header, names in lower case, in arrival order.
    pub headers: Vec<(String, String)>,
    /// The raw body.
    pub body: Vec<u8>,
}

impl Recorded {
    /// The value of the header `name`, when it was sent once.
    pub fn header(&self, name: &str) -> Option<&str> {
        let mut found = self
            .headers
            .iter()
            .filter(|(header, _)| header == name)
            .map(|(_, value)| value.as_str());
        let first = found.next();
        assert!(found.next().is_none(), "the header {name} was sent twice");
        first
    }
}

/// One scripted answer.
#[derive(Clone, Debug)]
pub struct Answer {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
    gate: Option<Arc<Notify>>,
    silent: bool,
}

impl Answer {
    /// An answer with `status` and no body.
    pub fn status(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: String::new(),
            gate: None,
            silent: false,
        }
    }

    /// The same answer with `body`.
    pub fn body(self, body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            ..self
        }
    }

    /// The same answer with a header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    /// The same answer, held until the test opens `gate`.
    pub fn held(self, gate: &Arc<Notify>) -> Self {
        Self {
            gate: Some(Arc::clone(gate)),
            ..self
        }
    }

    /// No answer at all: the request is held for the rest of the test.
    pub fn silence() -> Self {
        Self {
            silent: true,
            ..Self::status(200)
        }
    }
}

#[derive(Default)]
struct Script {
    received: Vec<Recorded>,
    answers: VecDeque<Answer>,
}

#[derive(Clone)]
struct Shared {
    script: Arc<Mutex<Script>>,
    arrived: Arc<Notify>,
    unscripted: u16,
}

/// A recording fake on a loopback port: it records every request and answers as the test scripts
/// it, and with `unscripted` once the script runs out.
pub struct Fake {
    origin: String,
    shared: Shared,
}

impl Fake {
    /// A fake on a free loopback port, answering `unscripted` when nothing is scripted. It speaks
    /// HTTP/1.1, and HTTP/2 by prior knowledge, on the test's own runtime.
    pub async fn start(unscripted: u16) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a loopback port");
        let origin = format!("http://{}", listener.local_addr().expect("its address"));
        let shared = Shared {
            script: Arc::default(),
            arrived: Arc::new(Notify::new()),
            unscripted,
        };
        let app = Router::new()
            .fallback(answer)
            .with_state(shared.clone());
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("the fake serves");
        });
        Self { origin, shared }
    }

    /// The fake's origin, `http://127.0.0.1:<port>`.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// Queues `answers`, in order.
    pub fn script(&self, answers: impl IntoIterator<Item = Answer>) {
        self.shared
            .script
            .lock()
            .unwrap()
            .answers
            .extend(answers);
    }

    /// Every request so far.
    pub fn received(&self) -> Vec<Recorded> {
        self.shared.script.lock().unwrap().received.clone()
    }

    /// Waits until `count` requests have arrived, and fails the test if they do not within a bound.
    pub async fn arrived(&self, count: usize) {
        let wait = async {
            loop {
                let notified = self.shared.arrived.notified();
                if self.shared.script.lock().unwrap().received.len() >= count {
                    return;
                }
                notified.await;
            }
        };
        tokio::time::timeout(ARRIVAL_BOUND, wait)
            .await
            .unwrap_or_else(|_| panic!("{count} request(s) did not arrive"));
    }
}

async fn answer(State(shared): State<Shared>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let body: Bytes = axum::body::to_bytes(body, 1 << 20)
        .await
        .expect("a body under a mebibyte");
    let recorded = Recorded {
        method: parts.method.to_string(),
        path: parts.uri.path().to_owned(),
        version: parts.version,
        headers: parts
            .headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    value.to_str().expect("a text header").to_owned(),
                )
            })
            .collect(),
        body: body.to_vec(),
    };
    let scripted = {
        let mut script = shared.script.lock().unwrap();
        script.received.push(recorded);
        script.answers.pop_front()
    };
    shared.arrived.notify_waiters();
    let answer = scripted.unwrap_or_else(|| Answer::status(shared.unscripted));
    if answer.silent {
        std::future::pending::<()>().await;
    }
    if let Some(gate) = answer.gate {
        gate.notified().await;
    }
    let mut response = (
        StatusCode::from_u16(answer.status).expect("a status"),
        answer.body,
    )
        .into_response();
    for (name, value) in answer.headers {
        response.headers_mut().insert(
            axum::http::HeaderName::from_bytes(name.as_bytes()).expect("a header name"),
            value.parse().expect("a header value"),
        );
    }
    response
}

/// A JWT a fake verified: its header and its claims.
#[derive(Clone, Debug, PartialEq)]
pub struct Verified {
    /// The JOSE header.
    pub header: Value,
    /// The claims.
    pub claims: Value,
}

/// `token`'s header and claims, when it is a compact JWS whose ES256 signature verifies with `key`.
/// The fake parses it itself: three base64url parts, JSON in the first two, and the fixed-size
/// `r||s` signature over the first two in the third.
pub fn verify_jwt(token: &str, key: &VerifyingKey) -> Option<Verified> {
    let mut parts = token.split('.');
    let (header, claims, signature) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let signature = Signature::from_slice(&Base64UrlUnpadded::decode_vec(signature).ok()?).ok()?;
    key.verify(format!("{header}.{claims}").as_bytes(), &signature)
        .ok()?;
    let json = |part: &str| -> Option<Value> {
        serde_json::from_slice(&Base64UrlUnpadded::decode_vec(part).ok()?).ok()
    };
    Some(Verified {
        header: json(header)?,
        claims: json(claims)?,
    })
}
