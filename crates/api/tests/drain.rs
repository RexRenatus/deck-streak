//! The shutdown signal lets an in-flight request finish before `serve` returns (SPEC-025 A10, R7).
//!
//! The request travels over a real loopback socket, and the test waits on events (the handler
//! running, the connection closing), each under a generous bound, never on the passing of time.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use deck_streak_api::{ListenAddress, bind, layered, serve};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch};

/// How long any one step may take on a loaded machine before the test fails naming it.
const BOUND: Duration = Duration::from_mins(1);

async fn bounded<T>(step: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(BOUND, future)
        .await
        .unwrap_or_else(|_| panic!("{step} did not happen within {BOUND:?}"))
}

#[tokio::test]
async fn the_shutdown_signal_lets_an_in_flight_request_finish() {
    // The handler reports that it runs, then holds its request until the test releases it.
    let (entered, mut entries) = mpsc::channel::<()>(1);
    let (release, released) = watch::channel(false);
    let app = layered(Router::new().route(
        "/api/test/held",
        get(move || {
            let entered = entered.clone();
            let mut released = released.clone();
            async move {
                entered.send(()).await.expect("the test is listening");
                released
                    .wait_for(|released| *released)
                    .await
                    .expect("the test holds the sender");
                "finished"
            }
        }),
    ));

    let address = ListenAddress::loopback(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .expect("the IPv4 loopback address");
    let listener = bind(address).await.expect("the loopback listener binds");
    let local = listener.local_addr().expect("a bound address");
    let (signal, signalled) = oneshot::channel::<()>();
    let server = tokio::spawn(serve(listener, app, async move {
        // A dropped sender is a signal too: the test is over.
        let _ = signalled.await;
    }));

    let mut stream = TcpStream::connect(local)
        .await
        .expect("the listener accepts");
    stream
        .write_all(b"GET /api/test/held HTTP/1.1\r\nhost: localhost\r\nconnection: close\r\n\r\n")
        .await
        .expect("the request is written");
    bounded("the handler running", entries.recv())
        .await
        .expect("the handler reports");

    // The signal arrives while the request is in flight: serve keeps running until it finishes.
    signal.send(()).expect("serve is listening for the signal");
    for _ in 0..32 {
        tokio::task::yield_now().await;
    }
    assert!(
        !server.is_finished(),
        "serve returned while a request was still in flight"
    );

    release.send(true).expect("the handler is listening");
    let mut response = Vec::new();
    bounded("the response", stream.read_to_end(&mut response))
        .await
        .expect("the connection closes after the response");
    let response = String::from_utf8(response).expect("a UTF-8 response");
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response:?}");
    assert!(response.ends_with("\r\n\r\nfinished"), "{response:?}");

    // With the request finished, serve returns, and returns well.
    let outcome = bounded("serve returning", server)
        .await
        .expect("the serve task completes");
    assert!(outcome.is_ok(), "{outcome:?}");
}
