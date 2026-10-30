//! The `bot` role runs the bot's loop under the shared lifecycle: it drains what was queued before
//! it started, tells systemd it is ready once the first long poll is issued, answers the owner,
//! requests the owner's `/sync` through the sync job's request file, and on SIGTERM confirms its
//! offset, says it is stopping and exits 0 (SPEC-026 R1, R2, R11, R13; ADR-025, ADR-026). With a
//! courses file named in its settings, it answers the owner's progress command from those courses
//! (SPEC-077 R16).
//!
//! It runs the built binary against the bot's own fake Bot API on a loopback port, with a temporary
//! state directory, a temporary credentials directory of synthetic credentials, and a temporary
//! datagram socket as its `NOTIFY_SOCKET`. It waits on the socket's messages and the fake's calls,
//! never on the passing of time, under one generous bound that fails with what was seen.

// An integration test is test code: its helpers panic on a failed child.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../bot/tests/support/fake_bot_api.rs"]
mod fake_bot_api;

use std::io::ErrorKind;
use std::os::unix::net::UnixDatagram;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use deck_streak_curriculum::progress::{BandProgress, CourseProgress};
use deck_streak_curriculum::store::put_progress;
use deck_streak_daemon::lifecycle::WATCHDOG_USEC;
use deck_streak_ingest::state::SqliteIngestState;
use deck_streak_kernel::courses::COURSES_FILE;
use deck_streak_kernel::{CourseCode, Db, UtcMillis};
use fake_bot_api::{APP_URL, Answer, Call, FakeBotApi, OWNER, TOKEN, owner_says, payload};
use serde_json::json;

/// The bound on the whole run: generous, because a build may still be warming the binary's pages.
const DEADLINE: Duration = Duration::from_mins(2);

/// The role's process, stopped with SIGKILL if the test fails before it stops it: a failed
/// assertion never leaves a bot polling behind it.
struct Role(Child);

impl Drop for Role {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            drop(self.0.kill());
            drop(self.0.wait());
        }
    }
}

/// Reads `socket` until it receives `wanted` or the deadline passes, recording every message.
fn receive_until(socket: &UnixDatagram, wanted: &str, seen: &mut Vec<String>, started: Instant) {
    let mut buffer = [0_u8; 256];
    while !seen.iter().any(|message| message == wanted) {
        assert!(started.elapsed() < DEADLINE, "no {wanted}; seen: {seen:?}");
        match socket.recv(&mut buffer) {
            Ok(length) => seen.push(String::from_utf8_lossy(&buffer[..length]).into_owned()),
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(error) => panic!("the socket failed: {error}; seen: {seen:?}"),
        }
    }
}

/// A credentials directory under `parent`, holding identity's two credentials, synthetic.
fn credentials(parent: &Path) -> std::path::PathBuf {
    let directory = parent.join("credentials");
    std::fs::create_dir(&directory).expect("the credentials directory");
    std::fs::write(directory.join("owner-user-id"), format!("{OWNER}\n")).expect("the owner");
    std::fs::write(directory.join("telegram-bot-token"), format!("{TOKEN}\n")).expect("the token");
    directory
}

/// Starts `deckstreakd bot` against `fake`, in `directory`: its state, its credentials and its
/// notify socket, whose other end is returned, reading with a short timeout.
fn start_role(fake: &FakeBotApi, directory: &Path) -> (Role, UnixDatagram) {
    start_role_with(fake, directory, &[])
}

/// Starts the role as [`start_role`] does, with the further `settings` set, each to a path. A
/// state directory a test has already seeded is kept.
fn start_role_with(
    fake: &FakeBotApi,
    directory: &Path,
    settings: &[(&str, &Path)],
) -> (Role, UnixDatagram) {
    let socket_path = directory.join("notify.socket");
    let socket = UnixDatagram::bind(&socket_path).expect("the notify socket binds");
    socket
        .set_read_timeout(Some(Duration::from_millis(250)))
        .expect("a read timeout");
    let state = directory.join("state");
    std::fs::create_dir_all(&state).expect("the state directory");
    let child = Command::new(env!("CARGO_BIN_EXE_deckstreakd"))
        .arg("bot")
        .env_clear()
        .env("STATE_DIRECTORY", &state)
        .env("CREDENTIALS_DIRECTORY", credentials(directory))
        .env("DECKSTREAK_MINI_APP_URL", APP_URL)
        .env("DECKSTREAK_BOT_API_URL", fake.base_url())
        // A request file whose directory does not exist: the request cannot be rung, so the answer
        // comes at once instead of after the wait for a job no unit starts here.
        .env(
            "DECKSTREAK_SYNC_REQUEST_PATH",
            directory.join("absent").join("request"),
        )
        .env("NOTIFY_SOCKET", &socket_path)
        .env(WATCHDOG_USEC, "5000000")
        .envs(settings.iter().copied())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the binary starts");
    (Role(child), socket)
}

/// The start's calls, up to the first long poll, which `READY=1` follows.
fn assert_started(calls: &[Call]) {
    let methods: Vec<&str> = calls.iter().map(|call| call.method.as_str()).collect();
    assert_eq!(
        methods[..5],
        [
            "deleteWebhook",
            "deleteMyCommands",
            "setMyCommands",
            "getUpdates",
            "getUpdates"
        ],
        "ready as the first long poll is issued"
    );
    assert!(
        calls.iter().all(|call| call.token_ok),
        "every call names the token"
    );
    assert_eq!(
        calls[2].body["scope"],
        json!({"type": "chat", "chat_id": OWNER})
    );
    assert_eq!(
        calls[4].body["offset"], 2,
        "the queued update is confirmed, not replayed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_bot_role_drains_answers_the_owner_and_stops_on_sigterm() {
    let fake = FakeBotApi::start().await;
    fake.script(
        "getUpdates",
        [
            // Queued before the start: drained, never answered.
            Answer::updates(vec![owner_says(1, "/start")]),
            Answer::updates(vec![owner_says(5, "/privacy"), owner_says(6, "/sync")]),
        ],
    );
    let directory = tempfile::tempdir().expect("a temporary directory");
    let (mut role, socket) = start_role(&fake, directory.path());
    let started = Instant::now();

    let mut seen = Vec::new();
    let (socket, mut seen) = tokio::task::spawn_blocking(move || {
        receive_until(&socket, "READY=1", &mut seen, started);
        (socket, seen)
    })
    .await
    .expect("the reader completes");
    // READY=1 went out as the first long poll was issued: the drain is done by then, and the poll
    // itself follows.
    let at_ready = fake.calls();
    assert!(
        at_ready
            .iter()
            .filter(|call| call.method == "getUpdates")
            .count()
            >= 1,
        "the drain comes before READY=1: {:?}",
        at_ready.iter().map(|call| &call.method).collect::<Vec<_>>()
    );
    tokio::time::timeout(DEADLINE, fake.until(|calls| calls.len() >= 5))
        .await
        .expect("the first long poll");
    assert_started(&fake.calls());

    // The owner's two commands are answered, the sync as a request for the job.
    tokio::time::timeout(
        DEADLINE,
        fake.until(|calls| {
            calls
                .iter()
                .filter(|call| call.method == "sendMessage")
                .count()
                >= 2
        }),
    )
    .await
    .expect("both commands answered");
    let sends = fake.calls_of("sendMessage");
    assert_eq!(
        payload(&sends[0])["text"]
            .as_str()
            .map(|text| text.starts_with("<b>Your data</b>")),
        Some(true)
    );
    let sync = sends[1].body["text"].as_str().expect("a text").to_owned();
    assert!(
        sync.contains("sync_request_unwritten"),
        "the request could not be written, and the answer says so: {sync}"
    );
    assert!(
        fake.calls_of("sendChatAction").len() == 1,
        "typing before the sync"
    );

    let killed = Command::new("kill")
        .args(["-TERM", &role.0.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(killed.success());
    let (status, seen) = tokio::task::spawn_blocking(move || {
        receive_until(&socket, "STOPPING=1", &mut seen, started);
        let status = role.0.wait().expect("the child exits");
        (status, seen)
    })
    .await
    .expect("the reader completes");
    assert_eq!(status.code(), Some(0), "a clean stop; seen: {seen:?}");
    assert!(
        seen.iter().any(|message| message == "WATCHDOG=1"),
        "{seen:?}"
    );

    // The offset past the owner's commands was confirmed before the stop.
    let offsets: Vec<i64> = fake
        .calls_of("getUpdates")
        .iter()
        .filter_map(|call| call.body["offset"].as_i64())
        .collect();
    assert!(
        offsets.contains(&7),
        "the offset past update 6 was sent: {offsets:?}"
    );

    // The owner's rescore waits for the next cycle, as the request marked it.
    let db = Db::open(&directory.path().join("state").join("deck_streak.db"))
        .await
        .expect("the database opens");
    let ingest = SqliteIngestState::new(db.clone())
        .load()
        .await
        .expect("the state reads");
    assert!(
        ingest.rescore_pending,
        "the /sync marked the owner's rescore"
    );
    db.close().await;
}

/// One synthetic course, which the courses file the role's settings name configures.
const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[
{"code":"qaa","name":"Course Qaa","flag":"F","deck_root":"Qaa","alias":"a","writing":false,
"unit_bands":{"A1":[1,4]}}]}"#;

/// Stores course `qaa`'s progress in the database the role opens under `state`, as a recompute
/// would.
async fn store_progress(state: &Path) {
    std::fs::create_dir(state).expect("the state directory");
    let db = Db::open(&state.join("deck_streak.db"))
        .await
        .expect("the database opens");
    let progress = CourseProgress {
        code: CourseCode::new("qaa").expect("a course code"),
        name: "Course Qaa".to_owned(),
        flag: "F".to_owned(),
        total_cards: 8,
        mature_cards: 5,
        mastery_pct: 62.5,
        current_band: "A1",
        bands: vec![BandProgress {
            band: "A1",
            total: 8,
            mature: 5,
            pct: 62.5,
            achieved: false,
        }],
        current_unit: Some(2),
    };
    let mut write = db.write().await.expect("a write");
    put_progress(&mut write, &progress, UtcMillis::from_epoch_millis(1_000))
        .await
        .expect("the progress writes");
    write.commit().await.expect("the progress commits");
    db.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_bot_role_answers_progress_from_the_courses_its_settings_name() {
    let fake = FakeBotApi::start().await;
    fake.script(
        "getUpdates",
        [
            // Queued before the start: drained, never answered.
            Answer::updates(vec![owner_says(1, "/start")]),
            Answer::updates(vec![owner_says(5, "/progress")]),
        ],
    );
    let directory = tempfile::tempdir().expect("a temporary directory");
    store_progress(&directory.path().join("state")).await;
    let courses = directory.path().join("courses.json");
    std::fs::write(&courses, COURSES).expect("the courses file");
    let (_role, _socket) = start_role_with(&fake, directory.path(), &[(COURSES_FILE, &courses)]);

    tokio::time::timeout(
        DEADLINE,
        fake.until(|calls| calls.iter().any(|call| call.method == "sendMessage")),
    )
    .await
    .expect("the progress command answered");
    let sends = fake.calls_of("sendMessage");
    assert_eq!(
        payload(&sends[0])["text"],
        json!(
            "<b>Road to C2</b>\nF <b>Course Qaa</b>: A1, 63% mastery, unit 2\n<i>Mastery is an estimate from your reviews: the average, over the cards it counts, of how likely you are to recall each card now, with cards not yet firmly learned counted for less and new or suspended cards counted as 0.</i>"
        ),
        "the stored progress of the course the settings name; a role that read no course would say \
         none is stored yet"
    );
}
