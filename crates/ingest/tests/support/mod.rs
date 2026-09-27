//! Support for ingest's integration tests (SPEC-022): ADR-022's synthetic collection, the engine's
//! own sync server on a loopback port, and a fresh process for every measured operation.
//!
//! A test that needs another process re-executes its own test binary for itself alone
//! (`<binary> <test> --exact`), with [`ROLE`] naming what the child does. So a budget test's
//! measured child holds nothing but the operation it measures, and its `VmHWM` is that
//! operation's peak (ADR-022): the fixture's build and the server stay in other processes.
//!
//! The engine's sync server reads its users from `SYNC_USER1` in its process environment.
//! Setting an environment variable is `unsafe` in edition 2024 and this workspace forbids unsafe
//! code, so the server runs in a child whose environment `Command::env` sets. The server exits
//! when its parent closes the child's standard input, so a parent that dies leaves no server.

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

pub mod synthetic;

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;

use anki::collection::CollectionBuilder;
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::sync::http_server::{SimpleServer, SyncServerConfig, default_ip_header};
use anki::sync::login::sync_login;
use anki::timestamp::TimestampMillis;
use tokio::runtime::Runtime;

/// The environment variable that names a re-executed test binary's role.
pub const ROLE: &str = "DECKSTREAK_TEST_ROLE";
/// The role that serves the engine's sync server.
pub const SERVER: &str = "sync-server";
/// The role that runs one measured operation and reports it.
pub const MEASURE: &str = "measure";
/// The server's base folder, handed to a server child.
pub const SERVER_BASE: &str = "DECKSTREAK_TEST_SERVER_BASE";
/// A collection's path, handed to a measured child.
pub const COLLECTION: &str = "DECKSTREAK_TEST_COLLECTION";
/// The sync server's endpoint, handed to a measured child.
pub const ENDPOINT: &str = "DECKSTREAK_TEST_ENDPOINT";

/// The synthetic account the local sync server knows. Neither value is a secret: the server
/// listens on a loopback port for the life of one test.
pub const USERNAME: &str = "synthetic-owner";
/// The synthetic account's password.
pub const PASSWORD: &str = "synthetic-password";

const REPORTED: &str = "deckstreak-measured ";
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

/// A value the parent handed this child in its environment.
///
/// # Panics
///
/// When the parent handed no such value.
#[must_use]
pub fn handed(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("the parent hands its child {name}"))
}

/// This test binary, re-executed for `test` alone as `role`.
fn child(test: &str, role: &str) -> Command {
    assert!(
        env::var_os(ROLE).is_none(),
        "a child process never starts another: {test} was re-executed as {role} without its role"
    );
    let binary = env::current_exe().unwrap_or_else(|error| panic!("the test binary: {error}"));
    let mut command = Command::new(binary);
    command
        .args([test, "--exact", "--nocapture", "--test-threads", "1"])
        .env(ROLE, role);
    command
}

/// A single-threaded runtime for the engine's asynchronous sync, driven by `block_on`.
///
/// # Panics
///
/// When the runtime cannot be built.
#[must_use]
pub fn runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| panic!("a tokio runtime: {error}"))
}

/// What a measured child reported, by name.
#[derive(Debug)]
pub struct Report(BTreeMap<String, String>);

impl Report {
    /// The reported value `name`, parsed.
    ///
    /// # Panics
    ///
    /// When the child reported no `name`, or a value that does not parse.
    #[must_use]
    pub fn value<T: std::str::FromStr>(&self, name: &str) -> T {
        let text = self
            .0
            .get(name)
            .unwrap_or_else(|| panic!("the measured child reported no {name}: {:?}", self.0));
        text.parse()
            .unwrap_or_else(|_| panic!("the measured child reported {name}={text}"))
    }
}

/// Runs `test` alone in a fresh process as [`MEASURE`], with `handed` in its environment, and
/// reads what it reported.
///
/// # Panics
///
/// When the child cannot run or fails, naming its output.
#[must_use]
pub fn measure(test: &str, handed: &[(&str, &OsStr)]) -> Report {
    let mut command = child(test, MEASURE);
    for (name, value) in handed {
        command.env(name, value);
    }
    let output = command
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|error| panic!("the measured child of {test}: {error}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the measured child of {test} failed ({}):\n{stdout}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    Report(
        stdout
            .lines()
            .filter_map(|line| marked(line, REPORTED)?.split_once('='))
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect(),
    )
}

/// In a measured child: reports `name` as `value` to the parent.
#[allow(
    clippy::print_stdout,
    reason = "the child reports to its parent on stdout"
)]
pub fn report(name: &str, value: impl std::fmt::Display) {
    println!("{REPORTED}{name}={value}");
}

/// This process's peak resident set so far (`VmHWM` in `/proc/self/status`), in KiB.
///
/// # Panics
///
/// When `/proc/self/status` holds no `VmHWM` line: the measurement is Linux's, and a platform
/// without it measures nothing.
#[must_use]
pub fn peak_resident_kib() -> u64 {
    let status = fs::read_to_string("/proc/self/status")
        .unwrap_or_else(|error| panic!("/proc/self/status: {error}"));
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|value| value.trim().strip_suffix("kB"))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or_else(|| panic!("/proc/self/status holds no VmHWM line"))
}

/// Where a sync server over `base` keeps the synthetic account's collection. The folder exists
/// on return, so a fixture can be built there before the server starts.
///
/// # Panics
///
/// When the folder cannot be created.
#[must_use]
pub fn server_collection(base: &Path) -> PathBuf {
    let folder = base.join(USERNAME);
    fs::create_dir_all(&folder).unwrap_or_else(|error| panic!("{}: {error}", folder.display()));
    folder.join("collection.anki2")
}

/// The engine's own sync server, serving `base` on a loopback port from a child process.
#[derive(Debug)]
pub struct SyncServer {
    child: Child,
    stdin: Option<ChildStdin>,
    _stdout: BufReader<ChildStdout>,
    endpoint: String,
}

impl SyncServer {
    /// Starts the server over `base` as a child of `test`, and waits until it listens.
    ///
    /// # Panics
    ///
    /// When the child cannot start or exits before it listens.
    #[must_use]
    pub fn start(test: &str, base: &Path) -> Self {
        let mut child = child(test, SERVER)
            .env(SERVER_BASE, base)
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

/// In a [`SERVER`] child: serves the engine's sync server over [`SERVER_BASE`] on a loopback
/// port, reports the address, and exits when the parent closes this process's standard input.
///
/// # Panics
///
/// When the server cannot bind a loopback port or fails while serving.
#[allow(
    clippy::print_stdout,
    reason = "the child reports its address to its parent on stdout"
)]
pub fn serve() {
    let base = PathBuf::from(handed(SERVER_BASE));
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

/// Plays the owner's other Anki client: answers `reviews` cards of the first top-level deck's
/// queue as Good in the collection at `collection`, then sends them to the server at `endpoint`
/// with the engine's own normal sync. No code of this workspace runs here; this is the review
/// "made on another client" that the port's sync then pulls.
///
/// # Panics
///
/// When the collection has fewer cards to answer, or the engine fails.
pub fn review_on_another_client(
    runtime: &Runtime,
    collection: &Path,
    endpoint: &str,
    reviews: usize,
) {
    let mut col = CollectionBuilder::new(collection)
        .build()
        .unwrap_or_else(|error| panic!("the other client opens its collection: {error}"));
    let (root, _) = col
        .get_all_deck_names(true)
        .unwrap_or_else(|error| panic!("the other client lists its decks: {error}"))
        .into_iter()
        .find(|(_, name)| !name.contains("::"))
        .unwrap_or_else(|| panic!("the other client has a top-level deck"));
    col.set_current_deck(root)
        .unwrap_or_else(|error| panic!("the other client selects a deck: {error}"));
    for answered in 0..reviews {
        let next = col
            .get_queued_cards(1, false)
            .unwrap_or_else(|error| panic!("the other client's queue: {error}"))
            .cards
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("the other client ran out of cards after {answered}"));
        col.answer_card(&mut CardAnswer {
            card_id: next.card.id(),
            current_state: next.states.current,
            new_state: next.states.good,
            rating: Rating::Good,
            answered_at: TimestampMillis::now(),
            milliseconds_taken: 3000,
            custom_data: None,
            from_queue: true,
        })
        .unwrap_or_else(|error| panic!("the other client answers a card: {error}"));
    }
    let mut auth = runtime
        .block_on(sync_login(
            USERNAME,
            PASSWORD,
            Some(endpoint.to_owned()),
            engine_client(),
        ))
        .unwrap_or_else(|error| panic!("the other client logs in: {error}"));
    auth.endpoint = endpoint.parse().ok();
    let answered = col
        .sync_meta()
        .expect("the other client's sync stamps")
        .modified;
    runtime
        .block_on(col.normal_sync(auth, engine_client()))
        .unwrap_or_else(|error| panic!("the other client syncs its reviews: {error}"));
    // A completed exchange moves the modified stamp to the server's new one (the engine reports
    // it as `NoChanges`, like a sync with nothing to do).
    let synced = col
        .sync_meta()
        .expect("the other client's sync stamps")
        .modified;
    assert!(
        synced.0 > answered.0,
        "the other client's sync sends its {reviews} review(s) to the server"
    );
    col.close(None)
        .unwrap_or_else(|error| panic!("the other client closes its collection: {error}"));
}

/// A fresh HTTP client of the engine's own type, built by its `Default`, as the port builds one.
fn engine_client<Client: Default>() -> Client {
    Client::default()
}
