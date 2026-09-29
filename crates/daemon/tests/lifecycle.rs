//! The binary tells systemd it is ready, keeps the watchdog fed and says when it stops, then exits
//! 0; the watchdog's interval is the predecessor's (SPEC-025 A11, A12, R8); and the `api` role
//! refuses to start without identity's credentials, by the missing one's id (SPEC-024 R3).
//!
//! A11 runs the built binary with a temporary state directory, a temporary credentials directory
//! holding synthetic credentials, and a temporary datagram socket as its `NOTIFY_SOCKET`, sends
//! SIGTERM with the system's `kill`, and reads the socket. It waits on the socket's messages, never
//! on the passing of time, under one generous bound that fails with every message seen. The child's
//! environment is set on its `Command` alone, and cleared first, so nothing of this process's
//! environment (a `NOTIFY_SOCKET` of its own) reaches it.

// An integration test is test code: its helpers panic on a failed child, and the golden reader
// prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::ErrorKind;
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::SocketAddr;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use deck_streak_daemon::lifecycle::{
    HEARTBEAT_DIVISOR, MIN_WATCHDOG, NOTIFY_SOCKET, Notifier, NotifyState, WATCHDOG_PID,
    WATCHDOG_USEC, spawn_heartbeat, watchdog_interval, watchdog_timeout,
};
use deck_streak_kernel::Environment;
use serde_json::{Value, json};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

/// The bound on the whole run: generous, because a loaded machine is slow.
const DEADLINE: Duration = Duration::from_mins(2);
/// How long one read of the socket waits before the test looks at the child again.
const POLL: Duration = Duration::from_millis(250);
/// Identity's two credentials, each with a synthetic value: an owner id of fewer than seven digits
/// and a token that never has the Bot API token's shape (SPEC-024 R11).
const CREDENTIALS: [(&str, &str); 2] = [
    ("owner-user-id", "4242"),
    ("telegram-bot-token", "synthetic-webapp-signing-token"),
];

/// A credentials directory under `parent` holding `credentials`, each as systemd writes one: a
/// file named by its id.
fn credentials_directory(parent: &Path, credentials: &[(&str, &str)]) -> PathBuf {
    let directory = parent.join("credentials");
    std::fs::create_dir(&directory).expect("the credentials directory");
    for (id, value) in credentials {
        std::fs::write(directory.join(id), format!("{value}\n")).expect("a credential file");
    }
    directory
}

/// Reads the socket until it receives `wanted`, recording every message in `seen`.
///
/// # Errors
///
/// What happened instead: the child exited first, or the deadline passed.
fn receive_until(
    socket: &UnixDatagram,
    wanted: &str,
    seen: &mut Vec<String>,
    exited: &mpsc::Receiver<Output>,
    started: Instant,
) -> Result<(), String> {
    let mut buffer = [0_u8; 256];
    loop {
        match socket.recv(&mut buffer) {
            Ok(length) => {
                let message = String::from_utf8_lossy(&buffer[..length]).into_owned();
                let done = message == wanted;
                seen.push(message);
                if done {
                    return Ok(());
                }
            }
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                if let Ok(output) = exited.try_recv() {
                    return Err(format!(
                        "the binary exited ({}) before {wanted}; messages seen: {seen:?}\n{}",
                        output.status,
                        String::from_utf8_lossy(&output.stdout)
                    ));
                }
                if started.elapsed() > DEADLINE {
                    return Err(format!(
                        "no {wanted} within {DEADLINE:?}; messages seen: {seen:?}"
                    ));
                }
            }
            Err(error) => return Err(format!("the socket failed: {error}; seen: {seen:?}")),
        }
    }
}

#[test]
fn the_binary_notifies_ready_watchdog_and_stopping_then_exits_zero() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let socket_path = directory.path().join("notify.socket");
    let socket = UnixDatagram::bind(&socket_path).expect("the notify socket binds");
    socket
        .set_read_timeout(Some(POLL))
        .expect("a read timeout on the socket");
    let state = directory.path().join("state");
    std::fs::create_dir(&state).expect("the state directory");
    let credentials = credentials_directory(directory.path(), &CREDENTIALS);

    let started = Instant::now();
    let child = Command::new(env!("CARGO_BIN_EXE_deckstreakd"))
        .arg("api")
        .env_clear()
        .env("DECKSTREAK_API_LISTEN", "127.0.0.1:0")
        .env("STATE_DIRECTORY", &state)
        .env("CREDENTIALS_DIRECTORY", &credentials)
        .env("NOTIFY_SOCKET", &socket_path)
        // The predecessor's floor, five seconds: the heartbeat is armed, and its first ping
        // follows READY=1 at once.
        .env(WATCHDOG_USEC, "5000000")
        .env("RUST_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    let pid = child.id().to_string();
    let (exit, exited) = mpsc::sync_channel(1);
    let waiter = std::thread::spawn(move || {
        let output = child.wait_with_output().expect("the binary is waited for");
        exit.send(output).expect("the test is listening");
    });

    let mut seen = Vec::new();
    let ready = receive_until(&socket, "READY=1", &mut seen, &exited, started);
    assert_eq!(ready, Ok(()));
    let fed = receive_until(&socket, "WATCHDOG=1", &mut seen, &exited, started);
    assert_eq!(fed, Ok(()));
    assert_eq!(
        seen.first().map(String::as_str),
        Some("READY=1"),
        "{seen:?}"
    );

    let killed = Command::new("kill")
        .args(["-TERM", &pid])
        .status()
        .expect("the system's kill runs");
    assert!(killed.success(), "kill -TERM {pid} failed: {killed}");
    let stopping = receive_until(&socket, "STOPPING=1", &mut seen, &exited, started);
    assert_eq!(stopping, Ok(()));

    let remaining = DEADLINE.saturating_sub(started.elapsed());
    let output = exited
        .recv_timeout(remaining)
        .unwrap_or_else(|_| panic!("the binary did not exit within {DEADLINE:?}; seen: {seen:?}"));
    waiter.join().expect("the waiter thread");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Every message the binary sent is one of the three, in the order systemd reads them.
    let kinds: BTreeSet<&str> = seen.iter().map(String::as_str).collect();
    assert_eq!(
        kinds,
        BTreeSet::from(["READY=1", "STOPPING=1", "WATCHDOG=1"]),
        "{seen:?}"
    );
    assert_eq!(
        seen.last().map(String::as_str),
        Some("STOPPING=1"),
        "{seen:?}"
    );
}

#[test]
fn the_api_role_refuses_start_without_an_identity_credential() {
    // Each of identity's credentials missing in turn: the role exits 1 before it binds, and its
    // first line is an ERROR event naming the missing credential's id and no credential's value.
    for (missing, _) in CREDENTIALS {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let state = directory.path().join("state");
        std::fs::create_dir(&state).expect("the state directory");
        let present: Vec<(&str, &str)> = CREDENTIALS
            .into_iter()
            .filter(|(id, _)| *id != missing)
            .collect();
        let credentials = credentials_directory(directory.path(), &present);
        let child = Command::new(env!("CARGO_BIN_EXE_deckstreakd"))
            .arg("api")
            .env_clear()
            .env("DECKSTREAK_API_LISTEN", "127.0.0.1:0")
            .env("STATE_DIRECTORY", &state)
            .env("CREDENTIALS_DIRECTORY", &credentials)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary starts");
        let pid = child.id().to_string();
        let (exit, exited) = mpsc::sync_channel(1);
        let waiter = std::thread::spawn(move || {
            let output = child.wait_with_output().expect("the binary is waited for");
            exit.send(output).expect("the test is listening");
        });
        let Ok(output) = exited.recv_timeout(DEADLINE) else {
            // It started instead of refusing: stop it, and fail naming the credential.
            let _ = Command::new("kill").args(["-TERM", &pid]).status();
            panic!("the api role started without the credential {missing}");
        };
        waiter.join().expect("the waiter thread");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(1), "{missing}: {stdout}");
        let first = stdout.lines().next().unwrap_or_default();
        assert!(first.starts_with("<3>"), "{missing}: {stdout}");
        assert!(
            first.contains(missing),
            "the refusal does not name {missing}: {first}"
        );
        for (_, value) in CREDENTIALS {
            assert!(
                !stdout.contains(value),
                "a credential's value was logged: {stdout}"
            );
        }
    }
}

#[test]
fn the_watchdog_interval_follows_the_predecessors_divisor() {
    // The port holds the predecessor's two constants, read by the parity oracle.
    let port: BTreeMap<&str, Value> = BTreeMap::from([
        (
            "watchdog._MIN_WATCHDOG_SEC",
            json!(MIN_WATCHDOG.as_secs_f64()),
        ),
        (
            "watchdog._HEARTBEAT_DIVISOR",
            json!(f64::from(HEARTBEAT_DIVISOR)),
        ),
    ]);
    let mut golden = BTreeMap::new();
    golden::each_case("watchdog.constants", |case| {
        let name = case.input["name"].as_str().unwrap_or_default().to_owned();
        let value = port.get(name.as_str()).unwrap_or_else(|| {
            panic!("the golden names {name}, which the lifecycle does not hold")
        });
        assert_eq!(value, &case.output, "{name}");
        golden.insert(name, case.output.as_f64().expect("a number"));
    });
    assert_eq!(golden.len(), port.len(), "every constant was proved");
    let floor = golden["watchdog._MIN_WATCHDOG_SEC"];
    let divisor = golden["watchdog._HEARTBEAT_DIVISOR"];

    // The interval applies them, as the predecessor's `create_heartbeat` does: no heartbeat below
    // the floor; at and above it, the timeout over the divisor.
    for millis in [1_000, 4_999, 5_000, 6_000, 30_000, 90_000] {
        let timeout = Duration::from_millis(millis);
        let expected = (timeout.as_secs_f64() >= floor).then(|| timeout.div_f64(divisor));
        let interval = watchdog_interval(timeout);
        assert_eq!(interval.is_some(), expected.is_some(), "{timeout:?}");
        if let (Some(interval), Some(expected)) = (interval, expected) {
            assert!(
                interval.abs_diff(expected) <= Duration::from_nanos(1),
                "{timeout:?}: {interval:?}, not {expected:?}"
            );
        }
    }

    // The timeout is the one systemd armed for this process: WATCHDOG_USEC, unless WATCHDOG_PID
    // names another process (sd_watchdog_enabled(3), the predecessor's `watchdog_interval_s`).
    let pid = 4242;
    let ninety = Duration::from_secs(90);
    let armed = Environment::from_vars([(WATCHDOG_USEC, "90000000")]);
    assert_eq!(watchdog_timeout(&armed, pid), Some(ninety));
    let addressed = Environment::from_vars([(WATCHDOG_USEC, "90000000"), (WATCHDOG_PID, "4242")]);
    assert_eq!(watchdog_timeout(&addressed, pid), Some(ninety));
    let another = Environment::from_vars([(WATCHDOG_USEC, "90000000"), (WATCHDOG_PID, "4343")]);
    assert_eq!(watchdog_timeout(&another, pid), None);
    for unarmed in ["", "0", "ninety", "-5"] {
        let env = Environment::from_vars([(WATCHDOG_USEC, unarmed)]);
        assert_eq!(watchdog_timeout(&env, pid), None, "{unarmed:?}");
    }
    assert_eq!(watchdog_timeout(&Environment::default(), pid), None);
}

/// One event a [`Recorder`] saw: its level and each field's text.
struct Seen {
    level: Level,
    fields: BTreeMap<String, String>,
}

/// A subscriber that keeps every event it is given, so a test reads what the lifecycle logged.
#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<Seen>>>);

struct Fields(BTreeMap<String, String>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_owned());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().to_owned(), value.to_string());
    }
}

impl Recorder {
    fn seen(&self) -> Vec<(Level, BTreeMap<String, String>)> {
        let events = self.0.lock().expect("the recorder's lock");
        events
            .iter()
            .map(|seen| (seen.level, seen.fields.clone()))
            .collect()
    }
}

impl Subscriber for Recorder {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = Fields(BTreeMap::new());
        event.record(&mut fields);
        self.0.lock().expect("the recorder's lock").push(Seen {
            level: *event.metadata().level(),
            fields: fields.0,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

fn notifier_for(socket: Option<&str>) -> Notifier {
    let env = match socket {
        Some(socket) => Environment::from_vars([(NOTIFY_SOCKET, socket)]),
        None => Environment::default(),
    };
    Notifier::from_env(&env)
}

fn read_datagram(socket: &UnixDatagram) -> String {
    socket
        .set_read_timeout(Some(DEADLINE))
        .expect("a read timeout");
    let mut buffer = [0_u8; 64];
    let length = socket.recv(&mut buffer).expect("a datagram");
    String::from_utf8_lossy(&buffer[..length]).into_owned()
}

#[test]
fn a_notifier_is_enabled_only_for_a_socket_of_a_form_it_speaks() {
    let recorder = Recorder::default();
    let disabled = tracing::subscriber::with_default(recorder.clone(), || {
        [
            notifier_for(None),
            notifier_for(Some("relative/notify.sock")),
            notifier_for(Some("notify.sock")),
        ]
    });
    for notifier in &disabled {
        assert!(!notifier.is_enabled());
        assert!(!notifier.notify(NotifyState::Ready));
    }
    let warnings: Vec<_> = recorder
        .seen()
        .into_iter()
        .filter(|(level, _)| *level == Level::WARN)
        .collect();
    assert_eq!(
        warnings.len(),
        2,
        "one WARN per socket of a form it does not speak"
    );
    assert!(
        warnings[0].1["message"].contains("a form this service does not speak"),
        "{:?}",
        warnings[0].1
    );

    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("notify.sock");
    let path_text = path.to_str().expect("a UTF-8 path");
    assert!(notifier_for(Some(path_text)).is_enabled());
    assert!(notifier_for(Some("@deck-streak-enabled")).is_enabled());
}

#[test]
fn a_notifier_delivers_each_state_to_a_path_or_an_abstract_socket() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("notify.sock");
    let receiver = UnixDatagram::bind(&path).expect("a datagram socket");
    let notifier = notifier_for(path.to_str());
    assert!(notifier.notify(NotifyState::Ready));
    assert_eq!(read_datagram(&receiver), "READY=1");
    assert!(notifier.notify(NotifyState::Stopping));
    assert_eq!(read_datagram(&receiver), "STOPPING=1");

    let name = format!("deck-streak-lifecycle-{}", std::process::id());
    let address = SocketAddr::from_abstract_name(name.as_bytes()).expect("an abstract name");
    let abstract_receiver = UnixDatagram::bind_addr(&address).expect("an abstract socket");
    let notifier = notifier_for(Some(&format!("@{name}")));
    assert!(notifier.notify(NotifyState::Watchdog));
    assert_eq!(read_datagram(&abstract_receiver), "WATCHDOG=1");
}

#[test]
fn a_failed_send_is_logged_once_per_episode_of_failures() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("notify.sock");
    let notifier = notifier_for(path.to_str());
    let recorder = Recorder::default();
    tracing::subscriber::with_default(recorder.clone(), || {
        let warned = || {
            recorder
                .seen()
                .into_iter()
                .filter(|(level, fields)| {
                    *level == Level::WARN && fields["message"].contains("was not sent")
                })
                .count()
        };
        // Nothing listens: the send fails, and the first failure of the episode is logged.
        assert!(!notifier.notify(NotifyState::Ready));
        assert_eq!(warned(), 1);
        // The episode goes on: the same failure is not logged again.
        assert!(!notifier.notify(NotifyState::Watchdog));
        assert_eq!(warned(), 1);
        // A send that gets through ends the episode, so the next failure is logged afresh.
        let receiver = UnixDatagram::bind(&path).expect("a datagram socket");
        assert!(notifier.notify(NotifyState::Watchdog));
        assert_eq!(read_datagram(&receiver), "WATCHDOG=1");
        assert_eq!(warned(), 1);
        drop(receiver);
        std::fs::remove_file(&path).expect("the socket's file");
        assert!(!notifier.notify(NotifyState::Stopping));
        assert_eq!(warned(), 2);
        let last = recorder.seen().pop().expect("an event");
        assert_eq!(last.1["state"], "STOPPING=1");
    });
}

#[test]
fn a_watchdog_below_the_minimum_arms_no_heartbeat_and_says_both_times_in_milliseconds() {
    let recorder = Recorder::default();
    let env = Environment::from_vars([(WATCHDOG_USEC, "1234000")]);
    let heartbeat = tracing::subscriber::with_default(recorder.clone(), || {
        spawn_heartbeat(notifier_for(None), &env)
    });
    assert!(heartbeat.is_none());
    let seen = recorder.seen();
    let warnings: Vec<_> = seen
        .iter()
        .filter(|(level, _)| *level == Level::WARN)
        .collect();
    assert_eq!(warnings.len(), 1, "{seen:?}");
    assert_eq!(warnings[0].1["watchdog_ms"], "1234");
    assert_eq!(
        warnings[0].1["minimum_ms"],
        MIN_WATCHDOG.as_millis().to_string()
    );
}
