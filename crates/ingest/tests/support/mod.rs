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

pub mod logs;
pub mod recording;
pub mod synthetic;

use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use anki::collection::CollectionBuilder;
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::sync::http_server::{SimpleServer, SyncServerConfig, default_ip_header};
use anki::sync::login::sync_login;
use anki::timestamp::TimestampMillis;
use deck_streak_ingest::engine::{AnkiEngine, EngineError, NewCardQueue, SyncLogin, SyncOutcome};
use deck_streak_ingest::settings::{
    STATE_DIRECTORY, SYNC_ENDPOINT, SYNC_PASSWORD, SYNC_USERNAME, SyncSettings,
};
use deck_streak_ingest::sync::{RetrySchedule, Syncer};
use deck_streak_ingest::sync_runs::{SyncRun, SyncRunStore, Trigger};
use deck_streak_kernel::{
    Clock, CredentialLoader, CredentialsDirectory, Db, Environment, KernelError, ManualClock,
    Redactor, StudyDay, StudyDayRule, UtcMillis,
};
use tokio::runtime::Runtime;
use tokio::sync::Notify;
use tokio::time::Instant;

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

/// One step of a scripted engine's normal sync.
#[derive(Clone)]
pub enum Step {
    /// Answer with this outcome at once.
    Answer(SyncOutcome),
    /// Fail with this kind at once.
    Fail(EngineError),
    /// Never answer: only the attempt's timeout ends it.
    Hang,
    /// Wait until the notify is released, then answer with the outcome.
    Hold(Arc<Notify>, SyncOutcome),
}

#[derive(Default)]
struct Script {
    steps: Mutex<VecDeque<Step>>,
    normal_syncs: AtomicUsize,
    full_downloads: AtomicUsize,
    active: AtomicUsize,
    most_active: AtomicUsize,
    started: Mutex<Vec<Instant>>,
}

/// An engine whose normal syncs follow a script, and that counts what it is asked: a request the
/// syncer never made is a call this engine never saw. The last step repeats when the script runs
/// out.
#[derive(Clone, Default)]
pub struct ScriptedEngine(Arc<Script>);

impl ScriptedEngine {
    /// An engine that plays `steps` in order.
    #[must_use]
    pub fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        let engine = Self::default();
        engine.0.steps.lock().unwrap().extend(steps);
        engine
    }

    /// The normal syncs it was asked for.
    #[must_use]
    pub fn normal_syncs(&self) -> usize {
        self.0.normal_syncs.load(Ordering::SeqCst)
    }

    /// The full downloads it was asked for.
    #[must_use]
    pub fn full_downloads(&self) -> usize {
        self.0.full_downloads.load(Ordering::SeqCst)
    }

    /// The most normal syncs that were running at one time.
    #[must_use]
    pub fn most_active(&self) -> usize {
        self.0.most_active.load(Ordering::SeqCst)
    }

    /// When each normal sync started, on tokio's clock.
    #[must_use]
    pub fn started(&self) -> Vec<Instant> {
        self.0.started.lock().unwrap().clone()
    }

    fn next(&self) -> Step {
        let mut steps = self.0.steps.lock().unwrap();
        if steps.len() > 1 {
            steps.pop_front().unwrap()
        } else {
            steps
                .front()
                .cloned()
                .unwrap_or(Step::Answer(SyncOutcome::NoChanges))
        }
    }
}

impl AnkiEngine for ScriptedEngine {
    fn new_card_queue(&self, _collection: &Path) -> Result<NewCardQueue, EngineError> {
        Ok(NewCardQueue::default())
    }

    async fn normal_sync(
        &self,
        _collection: &Path,
        _login: &SyncLogin,
    ) -> Result<SyncOutcome, EngineError> {
        let step = self.next();
        self.0.normal_syncs.fetch_add(1, Ordering::SeqCst);
        self.0.started.lock().unwrap().push(Instant::now());
        let active = self.0.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.0.most_active.fetch_max(active, Ordering::SeqCst);
        let answer = match step {
            Step::Answer(outcome) => Ok(outcome),
            Step::Fail(kind) => Err(kind),
            Step::Hang => std::future::pending().await,
            Step::Hold(release, outcome) => {
                release.notified().await;
                Ok(outcome)
            }
        };
        self.0.active.fetch_sub(1, Ordering::SeqCst);
        answer
    }

    async fn full_download(
        &self,
        _collection: &Path,
        _login: &SyncLogin,
    ) -> Result<(), EngineError> {
        self.0.full_downloads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// A run record kept in memory: for the paused-time tests, which must not wait on the database
/// while a timer is pending (SPEC-022 section 7).
#[derive(Clone, Default)]
pub struct MemoryRuns(Arc<Mutex<Vec<SyncRun>>>);

impl MemoryRuns {
    /// Every run recorded, in order.
    #[must_use]
    pub fn runs(&self) -> Vec<SyncRun> {
        self.0.lock().unwrap().clone()
    }
}

impl SyncRunStore for MemoryRuns {
    async fn scheduled_run_on(&self, day: StudyDay) -> Result<bool, KernelError> {
        Ok(self
            .runs()
            .iter()
            .any(|run| run.trigger == Trigger::Scheduled && run.study_day == day))
    }

    async fn last_success(&self) -> Result<Option<SyncRun>, KernelError> {
        Ok(self
            .runs()
            .into_iter()
            .filter(|run| run.outcome.is_ok())
            .max_by_key(|run| run.finished_at))
    }

    async fn record(&self, run: &SyncRun) -> Result<(), KernelError> {
        self.0.lock().unwrap().push(run.clone());
        Ok(())
    }
}

/// A scratch deployment: a state directory, a credentials directory holding the synthetic
/// account, and the service's database, all removed when the fixture drops.
pub struct Fixture {
    scratch: tempfile::TempDir,
    endpoint: String,
}

impl Fixture {
    /// A deployment that syncs from `endpoint`.
    #[must_use]
    pub fn new(endpoint: &str) -> Self {
        let scratch = tempfile::tempdir().unwrap();
        for folder in ["state", "credentials"] {
            fs::create_dir_all(scratch.path().join(folder)).unwrap();
        }
        let credentials = scratch.path().join("credentials");
        fs::write(credentials.join(SYNC_USERNAME), format!("{USERNAME}\n")).unwrap();
        fs::write(credentials.join(SYNC_PASSWORD), format!("{PASSWORD}\n")).unwrap();
        Self {
            scratch,
            endpoint: endpoint.to_owned(),
        }
    }

    /// The scratch directory everything lives in.
    #[must_use]
    pub fn scratch(&self) -> &Path {
        self.scratch.path()
    }

    /// The settings the service would read, for this deployment.
    #[must_use]
    pub fn settings(&self) -> SyncSettings {
        let state = self.scratch.path().join("state");
        SyncSettings::from_env(&Environment::from_vars([
            (SYNC_ENDPOINT, OsStr::new(&self.endpoint)),
            (STATE_DIRECTORY, state.as_os_str()),
        ]))
        .unwrap()
    }

    /// The kernel's loader over this deployment's credentials directory.
    #[must_use]
    pub fn credentials(&self) -> CredentialLoader {
        let directory = CredentialsDirectory::new(self.scratch.path().join("credentials")).unwrap();
        CredentialLoader::new(directory, Redactor::new())
    }

    /// The copy of the collection the syncer keeps.
    #[must_use]
    pub fn copy(&self) -> PathBuf {
        self.settings().copy_path()
    }

    /// The service's database for this deployment, migrated.
    pub async fn db(&self) -> Db {
        Db::open(&self.scratch.path().join("deckstreak.db"))
            .await
            .unwrap()
    }

    /// A syncer for this deployment over `engine`, recording in `store`, on `clock`, with every
    /// wait zero-length.
    #[must_use]
    pub fn syncer<E: AnkiEngine + Sync, S: SyncRunStore>(
        &self,
        engine: E,
        store: S,
        clock: Arc<dyn Clock>,
    ) -> Syncer<E, S> {
        Syncer::new(
            engine,
            store,
            self.settings(),
            self.credentials(),
            clock,
            StudyDayRule::default(),
        )
        .with_schedule(RetrySchedule::IMMEDIATE)
    }
}

/// A manual clock at `millis` past the epoch.
#[must_use]
pub fn clock_at(millis: i64) -> Arc<ManualClock> {
    Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(millis)))
}

/// Plays the owner's other Anki client forcing a full sync, as a desktop does after a change of
/// schema: marks its collection at `collection` changed in schema and uploads it whole to the
/// server at `endpoint`. No code of this workspace runs here.
///
/// # Panics
///
/// When the engine fails.
pub fn upload_from_another_client(runtime: &Runtime, collection: &Path, endpoint: &str) {
    let mut col = CollectionBuilder::new(collection)
        .build()
        .unwrap_or_else(|error| panic!("the other client opens its collection: {error}"));
    col.set_schema_modified()
        .unwrap_or_else(|error| panic!("the other client changes its schema: {error}"));
    let mut auth = runtime
        .block_on(sync_login(
            USERNAME,
            PASSWORD,
            Some(endpoint.to_owned()),
            engine_client(),
        ))
        .unwrap_or_else(|error| panic!("the other client logs in: {error}"));
    auth.endpoint = endpoint.parse().ok();
    runtime
        .block_on(col.full_upload(auth, engine_client()))
        .unwrap_or_else(|error| panic!("the other client uploads its collection: {error}"));
}

/// Plays the owner's other Anki client changing one setting: sets `key` to `value` in the
/// collection at `collection`, then sends the change to the server at `endpoint` with the engine's
/// own normal sync. No code of this workspace runs here.
///
/// # Panics
///
/// When the engine fails.
pub fn change_setting_on_another_client(
    runtime: &Runtime,
    collection: &Path,
    endpoint: &str,
    key: &str,
    value: i64,
) {
    let mut col = CollectionBuilder::new(collection)
        .build()
        .unwrap_or_else(|error| panic!("the other client opens its collection: {error}"));
    col.set_config_json(key, &value, false)
        .unwrap_or_else(|error| panic!("the other client changes a setting: {error}"));
    let mut auth = runtime
        .block_on(sync_login(
            USERNAME,
            PASSWORD,
            Some(endpoint.to_owned()),
            engine_client(),
        ))
        .unwrap_or_else(|error| panic!("the other client logs in: {error}"));
    auth.endpoint = endpoint.parse().ok();
    runtime
        .block_on(col.normal_sync(auth, engine_client()))
        .unwrap_or_else(|error| panic!("the other client syncs its setting: {error}"));
    col.close(None)
        .unwrap_or_else(|error| panic!("the other client closes its collection: {error}"));
}
