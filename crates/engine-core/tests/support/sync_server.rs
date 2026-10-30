//! The engine's own sync server on a loopback port, for the core's one-way tests (SPEC-364
//! section 3).
//!
//! Ported from the native adapter's test support (`crates/ffi/tests/support/sync_server.rs`), not
//! shared with it: the server's user comes from `SYNC_USER1` in its process environment, and
//! setting an environment variable is `unsafe` in edition 2024, which this workspace forbids. So a
//! test that needs the server re-executes its own test binary for itself alone
//! (`<binary> <test> --exact`), with [`ROLE`] naming the server, and `Command::env` sets the child's
//! environment. The server exits when its parent closes the child's standard input, so a parent that
//! dies leaves no server.

#![allow(
    clippy::expect_used,
    reason = "test support panics like a test (clippy.toml), but clippy's allow-*-in-tests reaches \
              only #[test] functions, not a support module's helpers"
)]

use std::env;
use std::io::{BufRead, BufReader, Read};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
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
        .join("engine-core-sync")
        .join(format!("{test}-{}-{stamp}", std::process::id()))
}

/// The engine's own sync server, serving the synthetic account on a loopback port from a child
/// process.
#[derive(Debug)]
pub struct SyncServer {
    child: Child,
    stdin: Option<ChildStdin>,
    _stdout: BufReader<ChildStdout>,
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
        let endpoint = loop {
            line.clear();
            let read = stdout
                .read_line(&mut line)
                .unwrap_or_else(|error| panic!("the sync server child of {test}: {error}"));
            assert!(
                read > 0,
                "the sync server child of {test} exited before it listened"
            );
            if let Some(address) = marked(&line, LISTENING) {
                break format!("http://{address}/");
            }
        };
        Self {
            child,
            stdin,
            _stdout: stdout,
            endpoint,
        }
    }

    /// The server's endpoint, `http://127.0.0.1:<port>/`.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl Drop for SyncServer {
    fn drop(&mut self) {
        // Closing its standard input stops the server; the kill covers a server that hangs.
        drop(self.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
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
