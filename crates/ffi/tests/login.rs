//! The engine's sync login through the native adapter, against the engine's own sync server on a
//! loopback port (SPEC-347 R1, R3; A1, A2).
//!
//! Each test that needs the server re-executes its own binary as the server's child
//! ([`sync_server::SERVER`]); the parent logs in through [`Engine::run`] as a native client does.
//! The host key the adapter returns is compared with the one the engine's own login, called
//! directly, receives from the same server, so the expected value is never the adapter's own.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

#[expect(
    dead_code,
    reason = "the login tests encode and decode with the wire helpers alone, so they build no \
              synthetic collection"
)]
mod support;

#[path = "support/sync_server.rs"]
mod sync_server;

use std::sync::Arc;

use anki::sync::login::sync_login;
use deck_streak_ffi::engine::{Engine, EngineRefusal};
use support::wire;
use sync_server::{PASSWORD, SERVER, SyncServer, USERNAME};

/// The engine's login call, `BackendSyncService.SyncLogin`.
const SYNC_LOGIN: (u32, u32) = (1, 3);
/// `BackendError.Kind.SYNC_AUTH_ERROR`.
const SYNC_AUTH_ERROR: u64 = 7;
/// A password the server's synthetic account does not have.
const WRONG_PASSWORD: &str = "not-the-password";

fn engine() -> Arc<Engine> {
    Engine::new(Vec::new()).expect("the engine starts from the default init message")
}

/// `SyncLoginRequest`: the user (field 1), the password (2) and the endpoint (3).
fn login_request(username: &str, password: &str, endpoint: &str) -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_bytes(&mut out, 1, username.as_bytes());
    wire::put_bytes(&mut out, 2, password.as_bytes());
    wire::put_bytes(&mut out, 3, endpoint.as_bytes());
    out
}

/// A string field's text.
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("a string field is UTF-8")
}

/// A fresh HTTP client of the engine's own type, built by its `Default`, as ingest's tests build
/// one.
fn engine_client<Client: Default>() -> Client {
    Client::default()
}

/// The host key the engine's own login, called directly and not through the adapter, receives
/// from the server at `endpoint` for the synthetic account.
fn engines_own_host_key(endpoint: &str) -> String {
    sync_server::runtime()
        .block_on(sync_login(
            USERNAME,
            PASSWORD,
            Some(endpoint.to_owned()),
            engine_client(),
        ))
        .unwrap_or_else(|error| panic!("the engine's own login: {error}"))
        .hkey
}

#[test]
fn a1_a_login_through_the_adapter_returns_the_servers_host_key() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("a1_a_login_through_the_adapter_returns_the_servers_host_key");
    let reply = engine()
        .run(
            SYNC_LOGIN.0,
            SYNC_LOGIN.1,
            login_request(USERNAME, PASSWORD, server.endpoint()),
        )
        .map(|auth| (text(&wire::bytes(&auth, 1)), text(&wire::bytes(&auth, 2))));
    let host_key = engines_own_host_key(server.endpoint());
    assert_eq!(
        reply,
        Ok((host_key.clone(), server.endpoint().to_owned())),
        "a login through the adapter answers with the server's host key and the endpoint it was sent"
    );
    assert!(!host_key.is_empty(), "the server mints a host key");
}

#[test]
fn a2_a_wrong_password_is_the_engines_auth_refusal_and_names_no_secret() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server =
        SyncServer::start("a2_a_wrong_password_is_the_engines_auth_refusal_and_names_no_secret");
    let refusal = engine()
        .run(
            SYNC_LOGIN.0,
            SYNC_LOGIN.1,
            login_request(USERNAME, WRONG_PASSWORD, server.endpoint()),
        )
        .expect_err("a wrong password is answered with no host key");
    let EngineRefusal::Engine { error } = &refusal else {
        panic!("a wrong password reaches the engine and the engine refuses it, not {refusal:?}");
    };
    let message = text(&wire::bytes(error, 1));
    assert_eq!(
        wire::varint(error, 2),
        SYNC_AUTH_ERROR,
        "a wrong password is the engine's auth refusal, not: {message}"
    );
    assert!(
        !message.is_empty(),
        "the refusal carries the engine's message"
    );

    let host_key = engines_own_host_key(server.endpoint());
    let texts = [
        ("the refusal's sentence", refusal.to_string()),
        ("the refusal's debug form", format!("{refusal:?}")),
        ("the engine's message", message),
        (
            "the refusal's bytes",
            String::from_utf8_lossy(error).into_owned(),
        ),
    ];
    let secrets = [
        ("the password sent", WRONG_PASSWORD),
        ("the account's password", PASSWORD),
        ("the account's host key", host_key.as_str()),
    ];
    let named: Vec<(&str, &str)> = texts
        .iter()
        .flat_map(|(text, value)| {
            secrets
                .iter()
                .filter(|(_, secret)| value.contains(secret))
                .map(move |(secret, _)| (*text, *secret))
        })
        .collect();
    assert_eq!(
        (texts.len(), secrets.len(), named),
        (4, 3, Vec::new()),
        "no refusal text names a password or the host key"
    );
}
