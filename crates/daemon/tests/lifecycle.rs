//! The binary tells systemd it is ready, keeps the watchdog fed and says when it stops, then exits
//! 0; and the watchdog's interval is the predecessor's (SPEC-025 A11, A12, R8).
//!
//! A11 runs the built binary with a temporary state directory and a temporary datagram socket as
//! its `NOTIFY_SOCKET`, sends SIGTERM with the system's `kill`, and reads the socket. It waits on
//! the socket's messages, never on the passing of time, under one generous bound that fails with
//! every message seen. The child's environment is set on its `Command` alone, and cleared first,
//! so nothing of this process's environment (a `NOTIFY_SOCKET` of its own) reaches it.

// An integration test is test code: its helpers panic on a failed child, and the golden reader
// prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::os::unix::net::UnixDatagram;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use deck_streak_daemon::lifecycle::{
    HEARTBEAT_DIVISOR, MIN_WATCHDOG, WATCHDOG_PID, WATCHDOG_USEC, watchdog_interval,
    watchdog_timeout,
};
use deck_streak_kernel::Environment;
use serde_json::{Value, json};

/// The bound on the whole run: generous, because a loaded machine is slow.
const DEADLINE: Duration = Duration::from_secs(120);
/// How long one read of the socket waits before the test looks at the child again.
const POLL: Duration = Duration::from_millis(250);

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

    let started = Instant::now();
    let child = Command::new(env!("CARGO_BIN_EXE_deckstreakd"))
        .arg("api")
        .env_clear()
        .env("DECKSTREAK_API_LISTEN", "127.0.0.1:0")
        .env("STATE_DIRECTORY", &state)
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
