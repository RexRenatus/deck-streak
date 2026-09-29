//! A fake Bot API for the bot's tests (SPEC-026 section 3): an axum server on a loopback port that
//! records every call and answers as each test scripts it, on the test's own tokio runtime.
//!
//! Unscripted, it answers as Telegram would: a message for a send or an edit, `true` for the rest,
//! and a long poll with nothing queued waits (a second at most, not its `timeout`) and returns no
//! update. A test queues answers per method: a result, a refusal with its status and `retry_after`,
//! an answer that is not the Bot API's, or silence past the client's timeout, which the client
//! meets as a network error.
//!
//! The transport it builds waits through a recorder, which notes each wait and returns at once: the
//! waits are read from what it noted, in real time. tokio's paused time cannot hold them, because
//! it moves its clock to the next timer whenever the runtime parks, a loopback request in flight
//! included, so the client's own timeout would fire on every request (ADR-026).
//!
//! Every id is synthetic: the owner and the stranger have fewer than seven digits, and the token
//! never has the Bot API token's shape (SPEC-024 R11).

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

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use deck_streak_bot::transport::{Incoming, Waits};
use deck_streak_bot::{
    ApiUrl, Commands, MiniAppUrl, OwnerSync, SyncAnswer, SyncRefusal, Transport,
};
use deck_streak_identity::Owner;
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, Db, ManualClock, Redactor, Secret, StudyDayRule,
    TelegramUserId, UtcMillis,
};
use serde_json::{Map, Value, json};
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::time::Instant;

/// The bot's synthetic token: no Bot API token's shape.
pub const TOKEN: &str = "synthetic-bot-token";
/// The owner's synthetic Telegram user id, which is also the owner's private chat's id.
pub const OWNER: i64 = 4242;
/// Another synthetic user.
pub const STRANGER: i64 = 5151;
/// A synthetic group's chat id.
pub const GROUP: i64 = -7171;
/// The timeout of the transports the tests build: short, so a silent answer is a network error in
/// a quarter of a second.
pub const CLIENT_TIMEOUT: Duration = Duration::from_millis(250);
/// How long a silent answer holds its request: past the client's timeout.
const SILENCE: Duration = Duration::from_secs(2);
/// The longest an unscripted long poll holds its request.
const LONG_POLL_HOLD: Duration = Duration::from_secs(1);

/// Every wait a transport's waits were asked for, in order. Each returns at once.
#[derive(Clone, Default)]
pub struct RecordedWaits(Arc<Mutex<Vec<Duration>>>);

impl RecordedWaits {
    /// The waits so far.
    #[must_use]
    pub fn noted(&self) -> Vec<Duration> {
        self.0.lock().unwrap().clone()
    }
}

impl Waits for RecordedWaits {
    fn wait(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        self.0.lock().unwrap().push(duration);
        Box::pin(tokio::task::yield_now())
    }
}

/// One call the bot made.
#[derive(Clone, Debug)]
pub struct Call {
    /// The Bot API method.
    pub method: String,
    /// The request's parameters: its JSON body, or a `multipart/form-data` body's fields, with a
    /// file's field as an object of its `filename`, `content_type` and `text`.
    pub body: Value,
    /// When it arrived.
    pub at: Instant,
    /// Whether it named the synthetic token.
    pub token_ok: bool,
    /// The message id the fake answered a send or an edit with, when it answered one.
    pub message_id: Option<i64>,
}

/// One scripted answer.
#[derive(Clone, Debug)]
pub enum Answer {
    /// `{"ok": true, "result": ...}`.
    Result(Value),
    /// A refusal with its HTTP status, as the Bot API writes one; a 429 may carry `retry_after`.
    Refused {
        /// The HTTP status, which is also the `error_code`.
        status: u16,
        /// The `description`.
        description: &'static str,
        /// The `parameters.retry_after`, when the refusal carries one.
        retry_after: Option<u64>,
    },
    /// An answer that is not the Bot API's, with this status.
    Garbage(u16),
    /// No answer before the client's timeout: the client meets a network error.
    Silence,
}

impl Answer {
    /// A 429 with `retry_after` seconds.
    #[must_use]
    pub const fn too_many(retry_after: u64) -> Self {
        Self::Refused {
            status: 429,
            description: "Too Many Requests: retry later",
            retry_after: Some(retry_after),
        }
    }

    /// A 429 with no `retry_after`.
    #[must_use]
    pub const fn too_many_bare() -> Self {
        Self::Refused {
            status: 429,
            description: "Too Many Requests",
            retry_after: None,
        }
    }

    /// A refusal with `status` and no parameters.
    #[must_use]
    pub const fn status(status: u16) -> Self {
        Self::Refused {
            status,
            description: "Synthetic refusal",
            retry_after: None,
        }
    }

    /// A batch of updates for `getUpdates`.
    #[must_use]
    pub fn updates(updates: Vec<Value>) -> Self {
        Self::Result(Value::Array(updates))
    }
}

#[derive(Default)]
struct Inner {
    calls: Vec<Call>,
    scripts: HashMap<String, VecDeque<Answer>>,
    next_message_id: i64,
}

/// The fake: its address, what it recorded, and the waits of the transports it built.
#[derive(Clone)]
pub struct FakeBotApi {
    base: String,
    inner: Arc<Mutex<Inner>>,
    changed: Arc<Notify>,
    waits: RecordedWaits,
}

impl FakeBotApi {
    /// Starts the fake on a loopback port, on the current runtime.
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a loopback port");
        let base = format!("http://{}", listener.local_addr().expect("its address"));
        let fake = Self {
            base,
            inner: Arc::new(Mutex::new(Inner {
                next_message_id: 100,
                ..Inner::default()
            })),
            changed: Arc::new(Notify::new()),
            waits: RecordedWaits::default(),
        };
        let app = Router::new()
            .route("/{bot}/{method}", post(answer))
            .with_state(fake.clone());
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("the fake serves");
        });
        fake
    }

    /// The fake's base URL, which the transport's [`ApiUrl`] takes.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base
    }

    /// A transport to the fake, as the bot with [`TOKEN`], read as a credential from `directory`:
    /// its requests bounded by [`CLIENT_TIMEOUT`], its waits noted by [`FakeBotApi::waits`].
    #[must_use]
    pub fn transport(&self, directory: &Path) -> Transport {
        let api = ApiUrl::new(&self.base).expect("a loopback URL");
        let waits: Arc<dyn Waits> = Arc::new(self.waits.clone());
        Transport::with_waits(&api, &token(directory), CLIENT_TIMEOUT, waits)
            .expect("the transport builds")
    }

    /// Every wait the transports this fake built were asked for, in order.
    #[must_use]
    pub fn waits(&self) -> Vec<Duration> {
        self.waits.noted()
    }

    /// Queues `answers` for `method`, in order, before its unscripted answer.
    pub fn script(&self, method: &str, answers: impl IntoIterator<Item = Answer>) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .scripts
            .entry(method.to_owned())
            .or_default()
            .extend(answers);
    }

    /// Every call so far, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<Call> {
        self.inner.lock().unwrap().calls.clone()
    }

    /// Every call of `method` so far, in order.
    #[must_use]
    pub fn calls_of(&self, method: &str) -> Vec<Call> {
        self.calls()
            .into_iter()
            .filter(|call| call.method == method)
            .collect()
    }

    /// Resolves once `holds` is true of the calls so far.
    pub async fn until(&self, holds: impl Fn(&[Call]) -> bool) {
        loop {
            let notified = self.changed.notified();
            if holds(&self.inner.lock().unwrap().calls) {
                return;
            }
            notified.await;
        }
    }
}

/// The token as the bot reads it: a credential file in `directory`.
#[must_use]
pub fn token(directory: &Path) -> Secret {
    std::fs::write(directory.join("telegram-bot-token"), format!("{TOKEN}\n"))
        .expect("the token's credential file");
    let credentials = CredentialsDirectory::new(directory).expect("an absolute path");
    CredentialLoader::new(credentials, Redactor::new())
        .load("telegram-bot-token")
        .expect("the token loads")
}

/// A text message from `from` in the chat `chat` of type `chat_type`, as update `update_id`.
#[must_use]
pub fn message(update_id: i64, from: i64, chat: i64, chat_type: &str, text: &str) -> Value {
    json!({
        "update_id": update_id,
        "message": {
            "message_id": update_id,
            "date": 0,
            "chat": {"id": chat, "type": chat_type},
            "from": {"id": from, "is_bot": false, "first_name": "Synthetic"},
            "text": text,
        },
    })
}

/// The owner's text message in the owner's private chat, as update `update_id`.
#[must_use]
pub fn owner_says(update_id: i64, text: &str) -> Value {
    message(update_id, OWNER, OWNER, "private", text)
}

/// A tap by `from` on a button carrying `data`, on the message `message_id`, as update
/// `update_id`; its callback query's id is `tap-<update_id>`.
#[must_use]
pub fn tap(update_id: i64, from: i64, data: &str, message_id: i64) -> Value {
    json!({
        "update_id": update_id,
        "callback_query": {
            "id": format!("tap-{update_id}"),
            "from": {"id": from, "is_bot": false, "first_name": "Synthetic"},
            "chat_instance": "synthetic-instance",
            "data": data,
            "message": {
                "message_id": message_id,
                "date": 0,
                "chat": {"id": from, "type": "private"},
            },
        },
    })
}

/// The owner's tap on a button carrying `data`, on the message `message_id`.
#[must_use]
pub fn owner_taps(update_id: i64, data: &str, message_id: i64) -> Value {
    tap(update_id, OWNER, data, message_id)
}

/// A call's body without its `chat_id`: the payload a golden's `send` holds.
#[must_use]
pub fn payload(call: &Call) -> Value {
    let mut body = call.body.clone();
    if let Some(fields) = body.as_object_mut() {
        fields.remove("chat_id");
    }
    body
}

/// The bench's clock starts at 2025-01-15T03:30:10Z, in epoch milliseconds: under the default
/// rule, still the study day 2025-01-14.
pub const BENCH_STARTED_AT: i64 = 1_736_911_810_000;

/// The Mini App URL the tests configure: the neutral example `.env.example` names.
pub const APP_URL: &str = "https://deckstreak.example/app";

/// The directory the committed golden messages live in.
#[must_use]
pub fn messages_directory() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/messages")
}

/// The committed golden message `name`'s `send` payload.
#[must_use]
pub fn golden_send(name: &str) -> Value {
    let path = messages_directory().join(format!("{name}.msg.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let envelope: Value = serde_json::from_str(&text).expect("a golden is JSON");
    assert_eq!(
        envelope["schema"], "phx.duty.message.v1",
        "{name}: the envelope's schema"
    );
    envelope["send"].clone()
}

/// An owner's sync that answers as the test scripts it, and counts its calls.
#[derive(Clone, Default)]
pub struct ScriptedSync {
    answers: Arc<Mutex<VecDeque<Result<SyncAnswer, SyncRefusal>>>>,
    calls: Arc<AtomicUsize>,
}

impl ScriptedSync {
    /// A sync that answers `answers`, in order, then refuses.
    #[must_use]
    pub fn answering(answers: impl IntoIterator<Item = Result<SyncAnswer, SyncRefusal>>) -> Self {
        Self {
            answers: Arc::new(Mutex::new(answers.into_iter().collect())),
            calls: Arc::default(),
        }
    }

    /// How many times the bot ran it.
    #[must_use]
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl OwnerSync for ScriptedSync {
    fn sync_now(&self) -> impl Future<Output = Result<SyncAnswer, SyncRefusal>> + Send {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let answer = self
            .answers
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Err(SyncRefusal {
                reason: "nothing_scripted",
            }));
        async move { answer }
    }
}

/// What a command test runs on: the fake, a temporary directory holding the token and the
/// database, the transport, the database, and the clock the handlers read the study day on.
pub struct Bench {
    /// The fake Bot API.
    pub fake: FakeBotApi,
    /// The temporary directory.
    pub directory: tempfile::TempDir,
    /// The transport to the fake.
    pub transport: Arc<Transport>,
    /// The service's database, migrated.
    pub db: Db,
    /// The handlers' clock, at [`BENCH_STARTED_AT`] until a test moves it.
    pub clock: Arc<ManualClock>,
}

impl Bench {
    /// A fresh fake, directory, transport and database.
    pub async fn start() -> Self {
        let fake = FakeBotApi::start().await;
        let directory = tempfile::tempdir().expect("a temporary directory");
        let transport = Arc::new(fake.transport(directory.path()));
        let db = Db::open(&directory.path().join("deck_streak.db"))
            .await
            .expect("the database opens");
        Self {
            fake,
            directory,
            transport,
            db,
            clock: Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
                BENCH_STARTED_AT,
            ))),
        }
    }

    /// The owner's handlers over this bench, with `sync` as the owner's sync.
    #[must_use]
    pub fn commands(&self, sync: ScriptedSync) -> Commands<ScriptedSync> {
        Commands::new(
            Arc::clone(&self.transport),
            owner(),
            MiniAppUrl::new(APP_URL).expect("an https URL"),
            self.db.clone(),
            sync,
            StudyDayRule::default(),
            self.clock.clone(),
        )
    }
}

/// The owner the tests configure.
#[must_use]
pub fn owner() -> Owner {
    Owner::new(TelegramUserId::new(OWNER))
}

/// `update` as the poll hands it to the handlers.
#[must_use]
pub fn incoming(update: Value) -> Incoming {
    Incoming::from_value(update)
}

async fn answer(
    State(fake): State<FakeBotApi>,
    UrlPath((bot, method)): UrlPath<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let body = parse(&headers, &body);
    let (answer, message_id) = {
        let mut inner = fake.inner.lock().unwrap();
        let scripted = inner.scripts.get_mut(&method).and_then(VecDeque::pop_front);
        inner.next_message_id += 1;
        let message_id = inner.next_message_id;
        let sends = matches!(
            method.as_str(),
            "sendMessage" | "editMessageText" | "sendDocument" | "sendDice" | "sendPhoto"
        );
        inner.calls.push(Call {
            method: method.clone(),
            body: body.clone(),
            at: Instant::now(),
            token_ok: bot == format!("bot{TOKEN}"),
            message_id: (sends && scripted.is_none()).then_some(message_id),
        });
        (scripted, message_id)
    };
    fake.changed.notify_waiters();
    match answer {
        Some(Answer::Result(result)) => ok(&result),
        Some(Answer::Refused {
            status,
            description,
            retry_after,
        }) => refused(status, description, retry_after),
        Some(Answer::Garbage(status)) => (
            StatusCode::from_u16(status).expect("a status"),
            "<html>not the Bot API</html>",
        )
            .into_response(),
        Some(Answer::Silence) => {
            tokio::time::sleep(SILENCE).await;
            ok(&Value::Bool(true))
        }
        None => unscripted(&method, &body, message_id).await,
    }
}

/// Telegram's own answer when nothing is scripted.
async fn unscripted(method: &str, body: &Value, message_id: i64) -> Response {
    match method {
        "getUpdates" => {
            let timeout = body.get("timeout").and_then(Value::as_u64).unwrap_or(0);
            tokio::time::sleep(Duration::from_secs(timeout).min(LONG_POLL_HOLD)).await;
            ok(&Value::Array(Vec::new()))
        }
        "savePreparedInlineMessage" => ok(&json!({"id": "prepared-1", "expiration_date": 0})),
        "sendPhoto" => {
            let chat = body
                .get("chat_id")
                .and_then(|chat| chat.as_i64().or_else(|| chat.as_str()?.parse().ok()))
                .unwrap_or(OWNER);
            // The sizes the Bot API answers come smallest to largest; here the largest is in the
            // middle, so a reader that takes the first or the last size takes the wrong one.
            ok(&json!({
                "message_id": message_id,
                "date": 0,
                "chat": {"id": chat, "type": "private"},
                "photo": [
                    {"file_id": "size-small", "file_unique_id": "a", "width": 90, "height": 60},
                    {"file_id": "size-large", "file_unique_id": "b", "width": 1280, "height": 853},
                    {"file_id": "size-medium", "file_unique_id": "c", "width": 320, "height": 213},
                ],
            }))
        }
        "sendMessage" | "editMessageText" | "sendDocument" | "sendDice" => {
            let chat = body
                .get("chat_id")
                .and_then(|chat| chat.as_i64().or_else(|| chat.as_str()?.parse().ok()))
                .unwrap_or(OWNER);
            ok(&json!({
                "message_id": message_id,
                "date": 0,
                "chat": {"id": chat, "type": "private"},
            }))
        }
        _ => ok(&Value::Bool(true)),
    }
}

fn ok(result: &Value) -> Response {
    let body = json!({"ok": true, "result": result}).to_string();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body,
    )
        .into_response()
}

fn refused(status: u16, description: &str, retry_after: Option<u64>) -> Response {
    let mut body = json!({"ok": false, "error_code": status, "description": description});
    if let Some(seconds) = retry_after {
        body["parameters"] = json!({"retry_after": seconds});
    }
    (
        StatusCode::from_u16(status).expect("a status"),
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// A request's parameters: its JSON body, or its `multipart/form-data` fields.
fn parse(headers: &HeaderMap, body: &Bytes) -> Value {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    match content_type.split_once("boundary=") {
        Some((_, boundary)) => multipart(boundary.trim_matches('"'), body),
        None => serde_json::from_slice(body).unwrap_or(Value::Null),
    }
}

/// A `multipart/form-data` body's fields: each text field as its text, and a file's as its
/// `filename`, `content_type` and `text`.
fn multipart(boundary: &str, body: &Bytes) -> Value {
    let text = String::from_utf8_lossy(body);
    let delimiter = format!("--{boundary}");
    let mut fields = Map::new();
    for part in text.split(&delimiter) {
        let part = part.strip_prefix("\r\n").unwrap_or(part);
        let Some((head, content)) = part.split_once("\r\n\r\n") else {
            continue;
        };
        let content = content.strip_suffix("\r\n").unwrap_or(content);
        let Some(name) = quoted(head, "name=\"") else {
            continue;
        };
        let value = match quoted(head, "filename=\"") {
            Some(filename) => json!({
                "filename": filename,
                "content_type": head
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Type: "))
                    .or_else(|| head.lines().find_map(|line| line.strip_prefix("content-type: ")))
                    .unwrap_or_default(),
                "text": content,
            }),
            None => Value::String(content.to_owned()),
        };
        fields.insert(name, value);
    }
    Value::Object(fields)
}

/// The quoted value that follows `key` in `head`.
fn quoted(head: &str, key: &str) -> Option<String> {
    let start = head.find(key)? + key.len();
    let end = head[start..].find('"')?;
    Some(head[start..start + end].to_owned())
}
