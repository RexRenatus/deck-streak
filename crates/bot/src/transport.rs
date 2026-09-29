//! The bot's one transport to the Telegram Bot API (SPEC-026 R1, R6, R8, R10; ADR-026).
//!
//! Every request the bot makes goes through [`Transport`], which holds frankenstein's async client.
//! Outbound text is HTML, never legacy Markdown: every dynamic value is escaped with
//! [`escape_html`] where it enters the markup, link previews are off, and a text longer than
//! [`MAX_TEXT_UTF16`] is split by [`crate::chunk`] before it is sent. A chunk goes in at most
//! [`SEND_ATTEMPTS`] attempts, as the predecessor's `telegram.py:TelegramNotifier.send_html` sent
//! one (`goldens/send_retry.json`): a 429 is waited out for its `parameters.retry_after` seconds, or
//! [`DEFAULT_RETRY_AFTER`] when the answer names none, on tokio's timer ([`TokioTimer`]), before the
//! same request goes again; any other failed attempt is followed by the next at once.
//!
//! The transport counts the messages it attempted and the ones it delivered, a delivery being a
//! message whose every chunk came back with a message id. Both counts only grow; the daemon joins
//! them to coordination's `DeliveryMarker` (SPEC-027 R6).
//!
//! The bot token is part of every request's URL, so nothing here logs a URL, a request or an
//! answer's body: a line names the method, the attempt and the Bot API's error code, and never a
//! message's text.
//!
//! [`OwnerChat`] is the bot's side of the notification router's transport port (SPEC-041 R13): the
//! router's pushes go to the owner's chat through the same [`Transport::send_html`], and the
//! ladder's renders (SPEC-084 R8; ADR-084) through the transport's reveal, dice, reaction and pin.

use std::fmt;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use deck_streak_identity::Owner;
use deck_streak_kernel::{Environment, Secret, Setting, SettingsError};
use deck_streak_notifications::{
    BotTransport, FileId, Pass, Photo, PhotoFuture, PhotoPushed, Prepared, PushFuture, Pushed,
    ShareFuture,
};
use frankenstein::client_reqwest::Bot;
use frankenstein::inline_mode::{InlineQueryResult, InlineQueryResultCachedPhoto, MaybeCached};
use frankenstein::methods::{
    AnswerCallbackQueryParams, DeleteMyCommandsParams, DeleteWebhookParams, EditMessageTextParams,
    GetUpdatesParams, PinChatMessageParams, SavePreparedInlineMessageParams, SendChatActionParams,
    SendDiceParams, SendMessageParams, SetMessageReactionParams, SetMyCommandsParams,
};
use frankenstein::reqwest;
use frankenstein::response::{ErrorResponse, MethodResponse};
use frankenstein::types::{
    AllowedUpdate, BotCommand, BotCommandScope, BotCommandScopeChat, ChatAction,
    InlineKeyboardMarkup, LinkPreviewOptions, Message, ReactionType, ReactionTypeEmoji,
    ReplyMarkup,
};
use frankenstein::updates::Update;
use frankenstein::{AsyncTelegramApi, ParseMode};
use serde_json::Value;

use crate::chunk;

/// The Bot API's base URL, without the `/bot<token>` part. Unset, it is Telegram's own; set, it
/// must be `https:`, or `http:` to a loopback host (a local Bot API server, or a test's fake).
pub const API_URL: &str = "DECKSTREAK_BOT_API_URL";
/// Telegram's own Bot API. Private to the bot, so no other crate can build a request on it
/// (SPEC-041 A15).
pub(crate) const DEFAULT_API_URL: &str = "https://api.telegram.org";

/// The longest text one message may carry, in UTF-16 units after entity parsing (the Bot API's
/// `sendMessage`; the telegram-platform pack). The predecessor's `constants.py:TELEGRAM_MAX_LEN`,
/// proved by `goldens/bot.constants.json`.
pub const MAX_TEXT_UTF16: usize = 4096;
/// The longest caption a document may carry, in UTF-16 units after entity parsing.
pub const MAX_CAPTION_UTF16: usize = 1024;
/// How many times one chunk is sent before the send gives up: the predecessor's
/// `telegram.py:TelegramNotifier.send_html` default, proved by `goldens/bot.constants.json`.
pub const SEND_ATTEMPTS: u32 = 3;
/// How long a 429 whose answer names no `retry_after` is waited out: the predecessor's
/// `telegram.py:_DEFAULT_RETRY_AFTER`, proved by `goldens/bot.constants.json`.
pub const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(1);
/// How long one request may take, connecting included. It outlasts the long poll's own timeout
/// ([`crate::poll::LONG_POLL_SECONDS`]), so a poll that returns empty is never cut off: the
/// predecessor's `bot.py:CommandBot` client, proved by `goldens/bot.timeouts.json`.
pub const HTTP_TIMEOUT: Duration = Duration::from_mins(1);

/// What Telegram answers an edit that would change nothing: a 400 an edit counts as delivered.
const NOT_MODIFIED: &str = "message is not modified";
/// The one status that carries a wait.
const TOO_MANY_REQUESTS: u64 = 429;
/// The status of a request the Bot API refused as malformed.
const BAD_REQUEST: u64 = 400;

/// The Bot API's base URL: `https:`, or `http:` to a loopback host, with no path, query or
/// credentials of its own. The token is appended when the transport is built, to the host's root
/// and nowhere else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiUrl(String);

impl ApiUrl {
    /// `text` as a base URL, or `None` when it is not `https:` or loopback `http:`, names no host,
    /// or carries a path, a query, a fragment, credentials or whitespace. The root's `/` is dropped;
    /// any path beyond it is refused.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let text = text.strip_suffix('/').unwrap_or(text);
        let (scheme, authority) = text.split_once("://")?;
        if authority.is_empty()
            || authority.contains(['/', '?', '#', '@'])
            || authority.contains(char::is_whitespace)
        {
            return None;
        }
        let host = host_of(authority)?;
        let secure = scheme == "https";
        let local = scheme == "http" && is_loopback(host);
        (secure || local).then(|| Self(text.to_owned()))
    }

    /// The base URL [`API_URL`] names, or Telegram's own when it is unset.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] when it is set and is not a base URL [`ApiUrl::new`] accepts.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        Ok(env
            .optional(API_URL)?
            .unwrap_or_else(|| Self(DEFAULT_API_URL.to_owned())))
    }

    /// The URL, as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Setting for ApiUrl {
    const SHAPE: &'static str = "an https: URL, or an http: URL of a loopback host, with no path";

    fn parse(text: &str) -> Option<Self> {
        Self::new(text)
    }
}

/// The host of an authority, `host`, `host:port` or `[v6]:port`, or `None` when it names none.
fn host_of(authority: &str) -> Option<&str> {
    let host = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split(']').next()?
    } else {
        authority.split(':').next()?
    };
    (!host.is_empty()).then_some(host)
}

/// Whether `host` is `localhost` or a loopback address.
fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// Escapes `text` for Telegram's HTML: `&`, `<` and `>` become `&amp;`, `&lt;` and `&gt;`. Each
/// character is written once, so an `&` the escape itself writes is never escaped again, which is
/// what escaping `&` first does in a chain of replacements.
#[must_use]
pub fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Escapes `value` for a double-quoted attribute of Telegram's HTML, such as a link's `href`: what
/// [`escape_html`] escapes, and `"` as `&quot;`.
#[must_use]
pub fn escape_attribute(value: &str) -> String {
    escape_html(value).replace('"', "&quot;")
}

/// Why a request to the Bot API did not succeed. It names no URL and no body: the token is in
/// every URL, and a body may echo a message.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The HTTP client could not be built.
    #[error("the Bot API's HTTP client could not be built")]
    Client(#[source] reqwest::Error),
    /// The Bot API refused the request with this error code.
    #[error("the Bot API refused the request with error code {code}")]
    Api {
        /// The Bot API's `error_code`.
        code: u64,
    },
    /// The request did not complete: it could not connect, timed out, or was cut off.
    #[error("the request to the Bot API did not complete")]
    Http(#[source] reqwest::Error),
    /// The answer could not be read as the Bot API's.
    #[error("the Bot API's answer could not be read")]
    Unreadable,
}

impl From<frankenstein::Error> for TransportError {
    fn from(error: frankenstein::Error) -> Self {
        match error {
            frankenstein::Error::Api(answer) => Self::Api {
                code: answer.error_code,
            },
            // frankenstein removes the URL, and with it the token, before it hands the error on.
            frankenstein::Error::HttpReqwest(error) => Self::Http(error),
            _ => Self::Unreadable,
        }
    }
}

/// The transport's counts since its process started. Both only ever grow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SendCounts {
    /// Messages the transport tried to send.
    pub attempted: u64,
    /// Messages whose every chunk came back with a message id.
    pub delivered: u64,
}

/// What one message's send came to.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sent {
    /// Every chunk came back with a message id; this is the first one's.
    Delivered {
        /// The first chunk's message id.
        message_id: i32,
    },
    /// A chunk gave up after its attempts, or there was nothing to send.
    Failed,
}

/// One update as the poll received it: its id, and the update when it could be read. An update
/// frankenstein cannot read is still confirmed by its id, so it can never block the poll.
#[derive(Clone, Debug)]
pub struct Incoming {
    /// The update's `update_id`, when it has one.
    pub update_id: Option<i64>,
    /// The update, when it could be read.
    pub update: Option<Update>,
}

impl Incoming {
    /// The update `value` as Telegram sent it.
    #[must_use]
    pub fn from_value(value: Value) -> Self {
        let update_id = value.get("update_id").and_then(Value::as_i64);
        Self {
            update_id,
            update: serde_json::from_value(value).ok(),
        }
    }
}

/// What one attempt at a request came to.
enum Attempt<T> {
    /// It succeeded.
    Done(T),
    /// A 429: the same request may go again after this wait.
    RateLimited(Duration),
    /// Any other failure: the next attempt goes at once.
    Failed(u64),
}

/// The status a failed attempt is logged with when the Bot API gave none: no answer came back.
const NO_ANSWER: u64 = 0;

impl<T> Attempt<T> {
    /// The attempt a refusal of the Bot API comes to.
    fn refused(answer: &ErrorResponse) -> Self {
        if answer.error_code == TOO_MANY_REQUESTS {
            let wait = answer
                .parameters
                .and_then(|parameters| parameters.retry_after)
                .map_or(DEFAULT_RETRY_AFTER, |seconds| {
                    Duration::from_secs(u64::from(seconds))
                });
            Self::RateLimited(wait)
        } else {
            Self::Failed(answer.error_code)
        }
    }

    /// The attempt a failed frankenstein call comes to.
    fn from_error(error: &frankenstein::Error) -> Self {
        match error {
            frankenstein::Error::Api(answer) => Self::refused(answer),
            _ => Self::Failed(NO_ANSWER),
        }
    }
}

/// Whether `answer` is an edit's "message is not modified": the text was already what the edit
/// asked for, so the edit is delivered (the predecessor's `telegram.py:_is_noop_edit`).
fn not_modified(answer: &ErrorResponse) -> bool {
    answer.error_code == BAD_REQUEST && answer.description.to_lowercase().contains(NOT_MODIFIED)
}

/// Link previews off: a message never unfurls a URL it carries.
fn no_preview() -> LinkPreviewOptions {
    LinkPreviewOptions::builder().is_disabled(true).build()
}

/// How the bot waits out a 429 and a failed poll: tokio's timer in the service. A test hands the
/// transport a recorder instead, since tokio's paused time moves its clock to the next timer
/// whenever the runtime parks, a request in flight on a loopback socket included, so the HTTP
/// client's own timeout would fire on every request (ADR-026, "Decided at delivery").
pub trait Waits: Send + Sync {
    /// Waits `duration`.
    fn wait(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// tokio's timer: the service's waits.
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioTimer;

impl Waits for TokioTimer {
    fn wait(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(tokio::time::sleep(duration))
    }
}

/// The Bot API transport: frankenstein's async client, the counts of what it sent, and its waits.
pub struct Transport {
    bot: Bot,
    waits: Arc<dyn Waits>,
    attempted: AtomicU64,
    delivered: AtomicU64,
}

impl fmt::Debug for Transport {
    /// Never the client: its URL holds the token.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Transport")
            .field("counts", &self.counts())
            .finish_non_exhaustive()
    }
}

impl Transport {
    /// The transport to the Bot API at `api`, as the bot whose token is `token`: every request
    /// bounded by [`HTTP_TIMEOUT`], every wait on tokio's timer.
    ///
    /// # Errors
    ///
    /// [`TransportError::Client`] when the HTTP client cannot be built.
    pub fn new(api: &ApiUrl, token: &Secret) -> Result<Self, TransportError> {
        Self::with_waits(api, token, HTTP_TIMEOUT, Arc::new(TokioTimer))
    }

    /// The transport to the Bot API at `api`, as the bot whose token is `token`, every request
    /// bounded by `timeout`, and waiting through `waits`.
    ///
    /// # Errors
    ///
    /// [`TransportError::Client`] when the HTTP client cannot be built.
    pub fn with_waits(
        api: &ApiUrl,
        token: &Secret,
        timeout: Duration,
        waits: Arc<dyn Waits>,
    ) -> Result<Self, TransportError> {
        let client = reqwest::Client::builder()
            .connect_timeout(timeout)
            .timeout(timeout)
            .build()
            .map_err(TransportError::Client)?;
        let bot = Bot::builder()
            .api_url(format!("{}/bot{}", api.as_str(), token.expose()))
            .client(client)
            .build();
        Ok(Self {
            bot,
            waits,
            attempted: AtomicU64::new(0),
            delivered: AtomicU64::new(0),
        })
    }

    /// Waits `duration`, as every wait of the bot does: on tokio's timer in the service.
    pub async fn wait(&self, duration: Duration) {
        self.waits.wait(duration).await;
    }

    /// The counts now.
    #[must_use]
    pub fn counts(&self) -> SendCounts {
        SendCounts {
            attempted: self.attempted.load(Ordering::Relaxed),
            delivered: self.delivered.load(Ordering::Relaxed),
        }
    }

    /// Runs `attempt` at most [`SEND_ATTEMPTS`] times, until one succeeds: a 429 waits its
    /// `retry_after` before the next, and any other failure is followed by the next at once. `None`
    /// when every attempt failed.
    async fn with_attempts<T, F, A>(&self, method: &'static str, mut attempt: F) -> Option<T>
    where
        F: FnMut() -> A,
        A: Future<Output = Attempt<T>>,
    {
        for number in 1..=SEND_ATTEMPTS {
            match attempt().await {
                Attempt::Done(value) => return Some(value),
                Attempt::RateLimited(wait) => {
                    let wait_ms = u64::try_from(wait.as_millis()).unwrap_or(u64::MAX);
                    tracing::warn!(
                        method,
                        attempt = number,
                        wait_ms,
                        "the Bot API asked for a wait"
                    );
                    if number < SEND_ATTEMPTS {
                        self.wait(wait).await;
                    }
                }
                Attempt::Failed(code) => {
                    tracing::warn!(method, attempt = number, code, "a Bot API request failed");
                }
            }
        }
        tracing::error!(
            method,
            attempts = SEND_ATTEMPTS,
            "a Bot API request gave up"
        );
        None
    }

    /// Sends `html` to `chat` as HTML with link previews off, split into chunks of at most
    /// [`MAX_TEXT_UTF16`] units, the keyboard on the last. Counts one attempted message, and one
    /// delivered when every chunk came back with a message id.
    pub async fn send_html(
        &self,
        chat: i64,
        html: &str,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Sent {
        self.attempted.fetch_add(1, Ordering::Relaxed);
        let chunks = chunk::chunks(html, MAX_TEXT_UTF16);
        let last = chunks.len().saturating_sub(1);
        let mut first = None;
        for (index, text) in chunks.into_iter().enumerate() {
            let markup = (index == last)
                .then(|| keyboard.clone())
                .flatten()
                .map(ReplyMarkup::InlineKeyboardMarkup);
            let params = SendMessageParams::builder()
                .chat_id(chat)
                .text(text)
                .parse_mode(ParseMode::Html)
                .link_preview_options(no_preview())
                .maybe_reply_markup(markup)
                .build();
            let (bot, params) = (&self.bot, &params);
            let sent = self
                .with_attempts("sendMessage", move || async move {
                    match bot.send_message(params).await {
                        Ok(answer) => Attempt::Done(answer.result.message_id),
                        Err(error) => Attempt::from_error(&error),
                    }
                })
                .await;
            let Some(message_id) = sent else {
                return Sent::Failed;
            };
            first.get_or_insert(message_id);
        }
        let Some(message_id) = first else {
            tracing::warn!(
                method = "sendMessage",
                "a message with no visible text was not sent"
            );
            return Sent::Failed;
        };
        self.delivered.fetch_add(1, Ordering::Relaxed);
        Sent::Delivered { message_id }
    }

    /// Replaces the text of the message `message_id` in `chat` with `html`, link previews off.
    /// An edit carries one message, so a text longer than [`MAX_TEXT_UTF16`] is refused before any
    /// request. An answer of "message is not modified" counts as delivered (R8). Counted as
    /// [`Transport::send_html`] counts.
    pub async fn edit_html(&self, chat: i64, message_id: i32, html: &str) -> Sent {
        self.attempted.fetch_add(1, Ordering::Relaxed);
        if chunk::chunks(html, MAX_TEXT_UTF16).len() != 1 {
            tracing::warn!(
                method = "editMessageText",
                "an edit that is not one chunk was not sent"
            );
            return Sent::Failed;
        }
        let params = EditMessageTextParams::builder()
            .chat_id(chat)
            .message_id(message_id)
            .text(html)
            .parse_mode(ParseMode::Html)
            .link_preview_options(no_preview())
            .build();
        let (bot, params) = (&self.bot, &params);
        let edited = self
            .with_attempts("editMessageText", move || async move {
                match bot.edit_message_text(params).await {
                    Ok(_) => Attempt::Done(message_id),
                    Err(frankenstein::Error::Api(answer)) if not_modified(&answer) => {
                        Attempt::Done(message_id)
                    }
                    Err(error) => Attempt::from_error(&error),
                }
            })
            .await;
        if edited.is_none() {
            return Sent::Failed;
        }
        self.delivered.fetch_add(1, Ordering::Relaxed);
        Sent::Delivered { message_id }
    }

    /// Sends a dice with `emoji` to `chat`, the topper of a T4 and a T5 celebration (SPEC-084 R8).
    /// Counted as [`Transport::send_html`] counts: a dice is a message.
    pub async fn send_dice(&self, chat: i64, emoji: &str) -> Sent {
        self.attempted.fetch_add(1, Ordering::Relaxed);
        let params = SendDiceParams::builder().chat_id(chat).emoji(emoji).build();
        let (bot, params) = (&self.bot, &params);
        let sent = self
            .with_attempts("sendDice", move || async move {
                match bot.send_dice(params).await {
                    Ok(answer) => Attempt::Done(answer.result.message_id),
                    Err(error) => Attempt::from_error(&error),
                }
            })
            .await;
        let Some(message_id) = sent else {
            return Sent::Failed;
        };
        self.delivered.fetch_add(1, Ordering::Relaxed);
        Sent::Delivered { message_id }
    }

    /// Reacts with `emoji` to the message `message_id` in `chat`, the T1 celebration (SPEC-084 R9).
    /// A reaction is no message, so it is not counted. Whether the Bot API took it.
    pub async fn set_message_reaction(&self, chat: i64, message_id: i32, emoji: &str) -> bool {
        let reaction = ReactionType::Emoji(ReactionTypeEmoji::builder().emoji(emoji).build());
        let params = SetMessageReactionParams::builder()
            .chat_id(chat)
            .message_id(message_id)
            .reaction(vec![reaction])
            .build();
        let (bot, params) = (&self.bot, &params);
        self.with_attempts("setMessageReaction", move || async move {
            match bot.set_message_reaction(params).await {
                Ok(_) => Attempt::Done(()),
                Err(error) => Attempt::from_error(&error),
            }
        })
        .await
        .is_some()
    }

    /// Pins the message `message_id` in `chat` without a notification, the T5 celebration's card
    /// (SPEC-084 R8). A pin is no message, so it is not counted. Whether the Bot API took it.
    pub async fn pin_chat_message(&self, chat: i64, message_id: i32) -> bool {
        let params = PinChatMessageParams::builder()
            .chat_id(chat)
            .message_id(message_id)
            .disable_notification(true)
            .build();
        let (bot, params) = (&self.bot, &params);
        self.with_attempts("pinChatMessage", move || async move {
            match bot.pin_chat_message(params).await {
                Ok(_) => Attempt::Done(()),
                Err(error) => Attempt::from_error(&error),
            }
        })
        .await
        .is_some()
    }

    /// Sends `bytes` to `chat` as the document `file_name`, with the HTML caption `caption` when it
    /// fits [`MAX_CAPTION_UTF16`]. The document is uploaded from memory as `multipart/form-data`,
    /// through frankenstein's own client and its re-export of reqwest, because frankenstein's
    /// `sendDocument` uploads only from a path on disk. Counted as [`Transport::send_html`] counts.
    pub async fn send_document(
        &self,
        chat: i64,
        file_name: &str,
        bytes: &[u8],
        caption: &str,
    ) -> Sent {
        self.attempted.fetch_add(1, Ordering::Relaxed);
        let caption = if chunk::visible_units(caption) <= MAX_CAPTION_UTF16 {
            caption
        } else {
            tracing::warn!(
                method = "sendDocument",
                "a caption over its bound was left off"
            );
            ""
        };
        let url = format!("{}/sendDocument", self.bot.api_url);
        let (client, url) = (&self.bot.client, url.as_str());
        let sent = self
            .with_attempts("sendDocument", move || async move {
                let Ok(form) = document_form(chat, file_name, bytes, caption) else {
                    return Attempt::Failed(NO_ANSWER);
                };
                match client.post(url).multipart(form).send().await {
                    Ok(answer) => message_id_of(answer).await,
                    Err(_) => Attempt::Failed(NO_ANSWER),
                }
            })
            .await;
        let Some(message_id) = sent else {
            return Sent::Failed;
        };
        self.delivered.fetch_add(1, Ordering::Relaxed);
        Sent::Delivered { message_id }
    }

    /// Sends `bytes` to `chat` as a photo with the HTML caption `caption`, in ONE request: a photo
    /// is never retried, because a lost answer to a send that arrived would send it twice, and the
    /// router already holds the failure. The image is uploaded from memory as `multipart/form-data`
    /// (frankenstein's `sendPhoto` uploads only from a path on disk). The answer is the file id of
    /// the largest size Telegram holds, or `None` on any refusal, rate limit, garbage or silence.
    pub async fn send_photo(&self, chat: i64, bytes: &[u8], caption: &str) -> Option<String> {
        self.attempted.fetch_add(1, Ordering::Relaxed);
        let url = format!("{}/sendPhoto", self.bot.api_url);
        let form = photo_form(chat, bytes, caption).ok()?;
        let answer = self
            .bot
            .client
            .post(url)
            .multipart(form)
            .send()
            .await
            .ok()?;
        if !answer.status().is_success() {
            return None;
        }
        let body = answer.text().await.ok()?;
        let sent = serde_json::from_str::<MethodResponse<Message>>(&body).ok()?;
        let file_id = sent
            .result
            .photo?
            .into_iter()
            .max_by_key(|size| u64::from(size.width) * u64::from(size.height))?
            .file_id;
        self.delivered.fetch_add(1, Ordering::Relaxed);
        Some(file_id)
    }

    /// Prepares the photo `file_id` with the HTML caption `caption` as an inline message the owner
    /// `user` can share, in ONE call: a cached photo result the owner can send to a user, a bot, a
    /// group or a channel. The id it is prepared under, or `None` on any failure.
    pub async fn save_prepared_inline_message(
        &self,
        user: u64,
        file_id: &str,
        caption: &str,
    ) -> Option<String> {
        let photo = InlineQueryResultCachedPhoto::builder()
            .id("share")
            .photo_file_id(file_id)
            .caption(caption)
            .parse_mode(ParseMode::Html)
            .build();
        let params = SavePreparedInlineMessageParams::builder()
            .user_id(user)
            .result(InlineQueryResult::Photo(MaybeCached::Cached(photo)))
            .allow_user_chats(true)
            .allow_bot_chats(true)
            .allow_group_chats(true)
            .allow_channel_chats(true)
            .build();
        self.bot
            .save_prepared_inline_message(&params)
            .await
            .ok()
            .map(|answer| answer.result.id)
    }

    /// Answers the callback query `callback_id`, so the client stops its progress indicator (R9).
    /// Whether the Bot API took the answer.
    pub async fn answer_callback(&self, callback_id: &str) -> bool {
        let params = AnswerCallbackQueryParams::builder()
            .callback_query_id(callback_id)
            .build();
        let (bot, params) = (&self.bot, &params);
        self.with_attempts("answerCallbackQuery", move || async move {
            match bot.answer_callback_query(params).await {
                Ok(_) => Attempt::Done(()),
                Err(error) => Attempt::from_error(&error),
            }
        })
        .await
        .is_some()
    }

    /// Shows `typing` in `chat` while a slow command works. Best effort: one request, and a
    /// failure is logged, never retried.
    pub async fn send_typing(&self, chat: i64) {
        let params = SendChatActionParams::builder()
            .chat_id(chat)
            .action(ChatAction::Typing)
            .build();
        if let Err(error) = self.bot.send_chat_action(&params).await {
            let error = TransportError::from(error);
            tracing::warn!(method = "sendChatAction", %error, "the chat action was not shown");
        }
    }

    /// Registers `commands` as the menu of `chat` alone (the chat scope), so nobody else sees them.
    /// Whether the Bot API took them.
    pub async fn set_chat_menu(&self, chat: i64, commands: Vec<BotCommand>) -> bool {
        let scope = BotCommandScope::Chat(BotCommandScopeChat::builder().chat_id(chat).build());
        let params = SetMyCommandsParams::builder()
            .commands(commands)
            .scope(scope)
            .build();
        let (bot, params) = (&self.bot, &params);
        self.with_attempts("setMyCommands", move || async move {
            match bot.set_my_commands(params).await {
                Ok(_) => Attempt::Done(()),
                Err(error) => Attempt::from_error(&error),
            }
        })
        .await
        .is_some()
    }

    /// Deletes the default-scope menu, the one every chat sees. Whether the Bot API took it.
    pub async fn delete_default_menu(&self) -> bool {
        let params = DeleteMyCommandsParams::builder()
            .scope(BotCommandScope::Default)
            .build();
        let (bot, params) = (&self.bot, &params);
        self.with_attempts("deleteMyCommands", move || async move {
            match bot.delete_my_commands(params).await {
                Ok(_) => Attempt::Done(()),
                Err(error) => Attempt::from_error(&error),
            }
        })
        .await
        .is_some()
    }

    /// Removes any webhook, so `getUpdates` is answered (ADR-026). One request.
    ///
    /// # Errors
    ///
    /// The [`TransportError`] the request came to.
    pub async fn delete_webhook(&self) -> Result<(), TransportError> {
        let params = DeleteWebhookParams::builder().build();
        self.bot.delete_webhook(&params).await?;
        Ok(())
    }

    /// One `getUpdates` request: the updates after `offset` of the kinds in `allowed_updates`,
    /// waiting up to `timeout` seconds for one, at most `limit` of them. Each update is read on its
    /// own, so one that cannot be read still carries its `update_id`.
    ///
    /// # Errors
    ///
    /// The [`TransportError`] the request came to; a refusal, `ok: false` included, is
    /// [`TransportError::Api`].
    pub async fn get_updates(
        &self,
        offset: i64,
        timeout: u32,
        limit: Option<u32>,
        allowed_updates: &[AllowedUpdate],
    ) -> Result<Vec<Incoming>, TransportError> {
        let params = GetUpdatesParams::builder()
            .offset(offset)
            .timeout(timeout)
            .maybe_limit(limit)
            .allowed_updates(allowed_updates.to_vec())
            .build();
        let answer: MethodResponse<Vec<Value>> =
            self.bot.request("getUpdates", Some(&params)).await?;
        Ok(answer
            .result
            .into_iter()
            .map(Incoming::from_value)
            .collect())
    }
}

/// The bot's side of the notification router's transport port (SPEC-041 R13): every push goes to
/// the owner's private chat, whose id is the owner's user id, through [`Transport::send_html`], so
/// a pushed text is HTML, chunked and retried as every message of the bot is. The composition root
/// joins it to the router; only the router can call it (the router's `Pass`).
pub struct OwnerChat {
    transport: Arc<Transport>,
    chat: i64,
}

impl OwnerChat {
    /// The owner's chat, over `transport`.
    #[must_use]
    pub const fn new(transport: Arc<Transport>, owner: Owner) -> Self {
        Self {
            transport,
            chat: owner.user().get(),
        }
    }

    /// Sends `text` as a line: what a reveal or a pinned message falls back to when its own message
    /// did not arrive (SPEC-084 R8).
    async fn line(&self, text: &str) -> Pushed {
        match self.transport.send_html(self.chat, text, None).await {
            Sent::Delivered { .. } => Pushed::Delivered,
            Sent::Failed => Pushed::Failed,
        }
    }
}

impl BotTransport for OwnerChat {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            match self.transport.send_html(self.chat, text, None).await {
                Sent::Delivered { .. } => Pushed::Delivered,
                Sent::Failed => Pushed::Failed,
            }
        })
    }

    fn push_reveal<'a>(
        &'a self,
        _pass: &'a Pass,
        placeholder: &'a str,
        text: &'a str,
        pause: Duration,
    ) -> PushFuture<'a> {
        Box::pin(async move {
            let Sent::Delivered { message_id } =
                self.transport.send_html(self.chat, placeholder, None).await
            else {
                return self.line(text).await;
            };
            self.transport.wait(pause).await;
            match self.transport.edit_html(self.chat, message_id, text).await {
                Sent::Delivered { .. } => Pushed::Delivered,
                Sent::Failed => self.line(text).await,
            }
        })
    }

    fn push_dice<'a>(&'a self, _pass: &'a Pass, emoji: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            match self.transport.send_dice(self.chat, emoji).await {
                Sent::Delivered { .. } => Pushed::Delivered,
                Sent::Failed => Pushed::Failed,
            }
        })
    }

    fn push_photo<'a>(
        &'a self,
        _pass: &'a Pass,
        photo: &'a Photo,
        caption: &'a str,
    ) -> PhotoFuture<'a> {
        Box::pin(async move {
            match self
                .transport
                .send_photo(self.chat, photo.bytes(), caption)
                .await
                .and_then(|id| FileId::new(id).ok())
            {
                Some(file_id) => PhotoPushed::Delivered { file_id },
                None => PhotoPushed::Failed,
            }
        })
    }

    fn prepare_share<'a>(
        &'a self,
        _pass: &'a Pass,
        file: &'a FileId,
        caption: &'a str,
    ) -> ShareFuture<'a> {
        Box::pin(async move {
            let Ok(user) = u64::try_from(self.chat) else {
                return Prepared::Failed;
            };
            match self
                .transport
                .save_prepared_inline_message(user, file.as_str(), caption)
                .await
            {
                Some(id) => Prepared::Ready { id },
                None => Prepared::Failed,
            }
        })
    }

    fn push_reaction<'a>(
        &'a self,
        _pass: &'a Pass,
        message_id: i64,
        emoji: &'a str,
    ) -> PushFuture<'a> {
        Box::pin(async move {
            // A message id the Bot API's type cannot hold names no message of the owner's chat.
            let Ok(message_id) = i32::try_from(message_id) else {
                return Pushed::Failed;
            };
            if self
                .transport
                .set_message_reaction(self.chat, message_id, emoji)
                .await
            {
                Pushed::Delivered
            } else {
                Pushed::Failed
            }
        })
    }

    fn push_pin<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            let Sent::Delivered { message_id } =
                self.transport.send_html(self.chat, text, None).await
            else {
                return self.line(text).await;
            };
            // The pin is the card's frame, not its message: the message arrived either way.
            let _pinned = self.transport.pin_chat_message(self.chat, message_id).await;
            Pushed::Delivered
        })
    }
}

/// The `multipart/form-data` body of one `sendDocument`: the chat, the caption as HTML when there
/// is one, and the document, as JSON.
fn document_form(
    chat: i64,
    file_name: &str,
    bytes: &[u8],
    caption: &str,
) -> Result<reqwest::multipart::Form, reqwest::Error> {
    let document = reqwest::multipart::Part::bytes(bytes.to_vec())
        .file_name(file_name.to_owned())
        .mime_str("application/json")?;
    let form = reqwest::multipart::Form::new().text("chat_id", chat.to_string());
    let form = if caption.is_empty() {
        form
    } else {
        form.text("caption", caption.to_owned())
            .text("parse_mode", "HTML")
    };
    Ok(form.part("document", document))
}

/// The multipart form of a `sendPhoto`: the chat, the HTML caption when there is one, and the image
/// as a file part whose type is read from its signature.
fn photo_form(
    chat: i64,
    bytes: &[u8],
    caption: &str,
) -> Result<reqwest::multipart::Form, reqwest::Error> {
    let (name, mime) = if bytes.starts_with(&[0xFF, 0xD8]) {
        ("photo.jpg", "image/jpeg")
    } else {
        ("photo.png", "image/png")
    };
    let photo = reqwest::multipart::Part::bytes(bytes.to_vec())
        .file_name(name)
        .mime_str(mime)?;
    let form = reqwest::multipart::Form::new().text("chat_id", chat.to_string());
    let form = if caption.is_empty() {
        form
    } else {
        form.text("caption", caption.to_owned())
            .text("parse_mode", "HTML")
    };
    Ok(form.part("photo", photo))
}

/// The attempt a `sendDocument` answer comes to: its message id, or its refusal.
async fn message_id_of(answer: reqwest::Response) -> Attempt<i32> {
    let success = answer.status().is_success();
    let Ok(body) = answer.text().await else {
        return Attempt::Failed(NO_ANSWER);
    };
    if success {
        serde_json::from_str::<MethodResponse<Message>>(&body)
            .map_or(Attempt::Failed(NO_ANSWER), |sent| {
                Attempt::Done(sent.result.message_id)
            })
    } else {
        serde_json::from_str::<ErrorResponse>(&body).map_or(Attempt::Failed(NO_ANSWER), |refusal| {
            Attempt::refused(&refusal)
        })
    }
}
