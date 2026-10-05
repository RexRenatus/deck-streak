//! The endpoint guard on the engine's sync login (SPEC-347 R2; A3, A4).
//!
//! A3 judges the guard alone over a table of endpoints, each admitted or refused by the rule it
//! breaks; A4 sends a refused login through a native dispatcher, where the guard must answer before
//! the engine sees the call. Every expected refusal is a literal written here from the SPEC's rule,
//! never read from the guard.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture fails its test, and an enumerating test prints what it examined"
)]

mod support;

use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::sync::SyncLoginRequest;
use deck_streak_engine_core::dispatch::{Dispatcher, Refusal};
use deck_streak_engine_core::login_guard;
use deck_streak_engine_core::table::Transport;
use prost::Message;

/// `BackendSyncService.SyncLogin`.
const SYNC_LOGIN: (u32, u32) = (1, 3);

/// A synthetic account: short values no refusal may repeat.
const USER: &str = "learner-one";
/// The synthetic account's password.
const PASSWORD: &str = "hunter-two";

/// The rule a login that names no endpoint breaks.
const ABSENT: &str = "the sync login names no endpoint";
/// The rule an endpoint that does not parse as a URL breaks.
const UNPARSEABLE: &str = "the sync login's endpoint is not a URL";
/// The rule an endpoint carrying a username or a password breaks.
const CREDENTIALS: &str = "the sync login's endpoint carries a username or a password";
/// The rule every scheme but `https`, and plain `http` to anything but a loopback address, breaks.
const SCHEME: &str =
    "the sync login's endpoint is neither https nor plain http to a loopback address";
/// The rule a request that does not decode as a login breaks.
const UNDECODABLE: &str = "the sync login request does not decode";

/// The encoded `SyncLoginRequest` for the synthetic account and `endpoint`.
fn login(endpoint: Option<&str>) -> Vec<u8> {
    SyncLoginRequest {
        username: USER.to_owned(),
        password: PASSWORD.to_owned(),
        endpoint: endpoint.map(str::to_owned),
    }
    .encode_to_vec()
}

/// A refusal's kind and message, decoded with the engine's own schema.
fn decoded(error: &[u8]) -> (Kind, String) {
    let refusal = BackendError::decode(error).expect("a refusal decodes as the engine's error");
    (refusal.kind(), refusal.message)
}

/// The requests A3 judges: what each is, its bytes, and the rule that refuses it, or `None` when
/// it is admitted. Every `https` endpoint and every plain `http` one to a loopback IP literal is
/// admitted; a URL carrying a user or a password is refused whatever its scheme.
fn endpoints() -> Vec<(&'static str, Vec<u8>, Option<&'static str>)> {
    vec![
        ("an absent endpoint", login(None), Some(ABSENT)),
        ("an empty endpoint", login(Some("")), Some(ABSENT)),
        (
            "https://example.invalid/",
            login(Some("https://example.invalid/")),
            None,
        ),
        (
            "https://sync.example.invalid:8443/base/",
            login(Some("https://sync.example.invalid:8443/base/")),
            None,
        ),
        (
            "http://127.0.0.1:27701/",
            login(Some("http://127.0.0.1:27701/")),
            None,
        ),
        (
            "http://127.8.9.10/",
            login(Some("http://127.8.9.10/")),
            None,
        ),
        (
            "http://[::1]:27701/",
            login(Some("http://[::1]:27701/")),
            None,
        ),
        ("not a url", login(Some("not a url")), Some(UNPARSEABLE)),
        ("https://", login(Some("https://")), Some(UNPARSEABLE)),
        (
            "http://example.invalid/",
            login(Some("http://example.invalid/")),
            Some(SCHEME),
        ),
        (
            "http://localhost:27701/",
            login(Some("http://localhost:27701/")),
            Some(SCHEME),
        ),
        (
            "http://192.0.2.1/",
            login(Some("http://192.0.2.1/")),
            Some(SCHEME),
        ),
        (
            "http://[2001:db8::1]/",
            login(Some("http://[2001:db8::1]/")),
            Some(SCHEME),
        ),
        (
            "http://127.0.0.1@example.invalid/",
            login(Some("http://127.0.0.1@example.invalid/")),
            Some(CREDENTIALS),
        ),
        (
            "https://user:pass@example.invalid/",
            login(Some("https://user:pass@example.invalid/")),
            Some(CREDENTIALS),
        ),
        (
            "https://user@example.invalid/",
            login(Some("https://user@example.invalid/")),
            Some(CREDENTIALS),
        ),
        (
            "http://:pass@127.0.0.1/",
            login(Some("http://:pass@127.0.0.1/")),
            Some(CREDENTIALS),
        ),
        (
            "ftp://example.invalid/",
            login(Some("ftp://example.invalid/")),
            Some(SCHEME),
        ),
        (
            "file:///collection.anki2",
            login(Some("file:///collection.anki2")),
            Some(SCHEME),
        ),
        (
            "ws://127.0.0.1:27701/",
            login(Some("ws://127.0.0.1:27701/")),
            Some(SCHEME),
        ),
        (
            "a request that does not decode",
            vec![0xff],
            Some(UNDECODABLE),
        ),
    ]
}

#[test]
fn a3_the_guard_admits_https_and_loopback_http_alone() {
    let cases = support::examined("endpoint(s)", endpoints());
    for (what, _, rule) in &cases {
        println!("{what}: {}", rule.unwrap_or("admitted"));
    }
    let judged: Vec<(&str, Option<(Kind, String)>)> = cases
        .iter()
        .map(|(what, request, _)| {
            (
                *what,
                login_guard::check(request)
                    .err()
                    .map(|error| decoded(&error)),
            )
        })
        .collect();
    let expected: Vec<(&str, Option<(Kind, String)>)> = cases
        .iter()
        .map(|(what, _, rule)| {
            (
                *what,
                rule.map(|rule| (Kind::InvalidInput, rule.to_owned())),
            )
        })
        .collect();
    assert_eq!(
        judged, expected,
        "each endpoint is admitted or refused by its rule"
    );
}

#[test]
fn a4_a_refused_login_never_reaches_the_engine_and_names_no_secret() {
    let endpoint = "ftp://127.0.0.1:1/";
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let reply = dispatcher.run(SYNC_LOGIN.0, SYNC_LOGIN.1, &login(Some(endpoint)));
    let Err(Refusal::Engine { error }) = &reply else {
        panic!(
            "a login to a non-HTTP scheme is refused in the engine's error shape, not {reply:?}"
        );
    };
    let (kind, message) = decoded(error);
    assert_eq!(
        (kind, message.as_str()),
        (Kind::InvalidInput, SCHEME),
        "the guard refuses the login before the engine sees it, by its rule"
    );
    let named: Vec<&str> = [endpoint, "127.0.0.1", USER, PASSWORD]
        .into_iter()
        .filter(|secret| message.contains(secret))
        .collect();
    assert_eq!(
        named,
        Vec::<&str>::new(),
        "the guard's sentence names neither the endpoint nor the account"
    );
}
