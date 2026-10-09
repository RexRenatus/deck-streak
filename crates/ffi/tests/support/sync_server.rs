//! The engine's own sync server on a loopback port, for the adapter's login tests (SPEC-347 R3).
//!
//! Ported from ingest's test support (`crates/ingest/tests/support/mod.rs`), not shared with it:
//! the server's user comes from `SYNC_USER1` in its process environment, and setting an
//! environment variable is `unsafe` in edition 2024, which this workspace forbids. So a test that
//! needs the server re-executes its own test binary for itself alone (`<binary> <test> --exact`),
//! with [`ROLE`] naming the server, and `Command::env` sets the child's environment. The server
//! exits when its parent closes the child's standard input, so a parent that dies leaves no server.
//!
//! The server sits behind a statement front (SPEC-374 R8): a loopback listener in the parent that
//! answers the sync service's statement of its minimum client level with an admitting one, as the
//! service does, and splices every other connection to the engine's server. The endpoint a test
//! logs in at is the front's, so the adapter's read of the statement before a login is answered.

#![allow(
    clippy::expect_used,
    reason = "test support panics like a test (clippy.toml), but clippy's allow-*-in-tests reaches \
              only #[test] functions, not a support module's helpers"
)]

use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use anki::sync::http_server::{SimpleServer, SyncServerConfig, default_ip_header};
use tokio::runtime::Runtime;

/// The environment variable that names a re-executed test binary's role.
const ROLE: &str = "DECKSTREAK_TEST_ROLE";
/// The role that serves the engine's sync server.
pub const SERVER: &str = "sync-server";
/// The server's base folder, handed to a server child.
const SERVER_BASE: &str = "DECKSTREAK_TEST_SERVER_BASE";

/// The synthetic account the local sync server knows. Neither value is a secret: the server
/// listens on a loopback port for the life of one test.
pub const USERNAME: &str = "synthetic-owner";
/// The synthetic account's password.
pub const PASSWORD: &str = "synthetic-password";

const LISTENING: &str = "deckstreak-listening ";

/// The request line of the adapter's read of the statement, up to its version.
const STATEMENT_REQUEST: &[u8] = b"GET /api/sync/minimum-client ";
/// The front's answer to that read: an admitting statement, its minimum the level this tree builds.
const ADMITTING_ANSWER: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                                  Content-Length: 26\r\nConnection: close\r\n\r\n\
                                  {\"minimum_client_level\":1}";

/// The text after `marker` on a child's output line. libtest prints `test <name> ... ` with no
/// newline before it runs the test, so a child's first line arrives glued to that prefix.
fn marked<'line>(line: &'line str, marker: &str) -> Option<&'line str> {
    line.find(marker)
        .map(|at| line[at + marker.len()..].trim_end())
}

/// The role this process was started for, when it is a re-executed child.
#[must_use]
pub fn role() -> Option<String> {
    env::var(ROLE).ok()
}

/// This test binary, re-executed for `test` alone as `role`.
fn child(test: &str, role: &str) -> Command {
    assert!(
        env::var_os(ROLE).is_none(),
        "a child process never starts another: {test} was re-executed as {role} without its role"
    );
    let binary = env::current_exe().expect("the test binary's path");
    let mut command = Command::new(binary);
    command
        .args([test, "--exact", "--nocapture", "--test-threads", "1"])
        .env(ROLE, role);
    command
}

/// A single-threaded runtime for the engine's asynchronous server and login, driven by
/// `block_on`.
///
/// # Panics
///
/// When the runtime cannot be built.
#[must_use]
pub fn runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a tokio runtime")
}

/// A folder of `test`'s own under the target's scratch space, for the server's accounts.
fn base(test: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-login")
        .join(format!("{test}-{}-{stamp}", std::process::id()))
}

/// The engine's own sync server, serving the synthetic account on a loopback port from a child
/// process.
#[derive(Debug)]
pub struct SyncServer {
    child: Child,
    stdin: Option<ChildStdin>,
    _stdout: BufReader<ChildStdout>,
    front: Front,
    endpoint: String,
}

impl SyncServer {
    /// Starts the server as a child of `test`, and waits until it listens.
    ///
    /// # Panics
    ///
    /// When the child cannot start or exits before it listens.
    #[must_use]
    pub fn start(test: &str) -> Self {
        let mut child = child(test, SERVER)
            .env(SERVER_BASE, base(test))
            .env("SYNC_USER1", format!("{USERNAME}:{PASSWORD}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("the sync server child of {test}: {error}"));
        let stdin = child.stdin.take();
        let mut stdout = BufReader::new(
            child
                .stdout
                .take()
                .unwrap_or_else(|| panic!("the sync server child of {test} has no stdout")),
        );
        let mut line = String::new();
        let server: SocketAddr = loop {
            line.clear();
            let read = stdout
                .read_line(&mut line)
                .unwrap_or_else(|error| panic!("the sync server child of {test}: {error}"));
            assert!(
                read > 0,
                "the sync server child of {test} exited before it listened"
            );
            if let Some(address) = marked(&line, LISTENING) {
                break address.parse().unwrap_or_else(|error| {
                    panic!("the sync server's address {address}: {error}")
                });
            }
        };
        let front = Front::start(server);
        let endpoint = format!("http://{}/", front.address);
        Self {
            child,
            stdin,
            _stdout: stdout,
            front,
            endpoint,
        }
    }

    /// The endpoint a client logs in at: the statement front's, `http://127.0.0.1:<port>/`.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl Drop for SyncServer {
    fn drop(&mut self) {
        self.front.stop();
        // Closing its standard input stops the server; the kill covers a server that hangs.
        drop(self.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The statement front: a loopback listener that answers the statement's read and splices every
/// other connection to the engine's server.
#[derive(Debug)]
struct Front {
    address: SocketAddr,
    stopped: Arc<AtomicBool>,
}

impl Front {
    /// Listens on a loopback port and serves each connection on a thread of its own.
    fn start(server: SocketAddr) -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .expect("the statement front binds a loopback port");
        let address = listener
            .local_addr()
            .expect("the statement front's address reads");
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopped);
        thread::spawn(move || {
            for client in listener.incoming() {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(client) = client {
                    thread::spawn(move || answer(client, server));
                }
            }
        });
        Self { address, stopped }
    }

    /// Stops accepting: the flag is set, then one connection wakes the listener to read it.
    fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
    }
}

/// Answers one connection: the statement's read with an admitting statement, and anything else by
/// splicing it, from its first byte, to the engine's server at `server`.
fn answer(mut client: TcpStream, server: SocketAddr) {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\n") && head.len() < 4096 {
        match client.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
    }
    if head.starts_with(STATEMENT_REQUEST) {
        let _ = client.write_all(ADMITTING_ANSWER);
        let _ = client.shutdown(Shutdown::Write);
        let _ = std::io::copy(&mut client, &mut std::io::sink());
        return;
    }
    let Ok(mut upstream) = TcpStream::connect(server) else {
        return;
    };
    if upstream.write_all(&head).is_err() {
        return;
    }
    let (Ok(mut client_reader), Ok(mut upstream_writer)) =
        (client.try_clone(), upstream.try_clone())
    else {
        return;
    };
    let sending = thread::spawn(move || {
        let _ = std::io::copy(&mut client_reader, &mut upstream_writer);
        let _ = upstream_writer.shutdown(Shutdown::Write);
    });
    let _ = std::io::copy(&mut upstream, &mut client);
    let _ = client.shutdown(Shutdown::Write);
    let _ = sending.join();
}

/// In a [`SERVER`] child: serves the engine's sync server over its base folder on a loopback
/// port, reports the address, and exits when the parent closes this process's standard input.
///
/// # Panics
///
/// When the parent handed no base folder, or the server cannot bind a loopback port or fails
/// while serving.
#[allow(
    clippy::print_stdout,
    reason = "the child reports its address to its parent on stdout"
)]
pub fn serve() {
    let base = PathBuf::from(
        env::var(SERVER_BASE)
            .unwrap_or_else(|_| panic!("the parent hands its child {SERVER_BASE}")),
    );
    thread::spawn(|| {
        let _ = std::io::stdin().read_to_end(&mut Vec::new());
        std::process::exit(0);
    });
    runtime().block_on(async {
        let (address, server) = SimpleServer::make_server(SyncServerConfig {
            host: Ipv4Addr::LOCALHOST.into(),
            port: 0,
            base_folder: base,
            ip_header: default_ip_header(),
        })
        .await
        .unwrap_or_else(|error| panic!("the sync server binds a loopback port: {error}"));
        println!("{LISTENING}{address}");
        server
            .await
            .unwrap_or_else(|error| panic!("the sync server serves: {error}"));
    });
}
