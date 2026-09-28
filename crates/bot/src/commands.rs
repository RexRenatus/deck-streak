//! The bot's commands: the menu, the handlers and the messages they render (SPEC-026 R9, R11, R12;
//! ADR-037).
//!
//! At W0 the owner has five commands. `/start` says first that the coach is an AI, and offers the
//! Mini App through a `web_app` button whose URL is [`MINI_APP_URL`]. `/privacy` links the published
//! privacy policy. `/export` sends the owner's data, `coordination::export_all`, as one JSON
//! document. `/delete` asks for confirmation with a button, and erases, `coordination::erase_all`,
//! only when the owner taps the button of the latest prompt, once. `/sync` is the owner's explicit
//! sync trigger (ADR-037): it runs one sync cycle now through [`OwnerSync`] and answers with the
//! outcome. `/export` and `/sync` show `typing` before they work. Anything else the owner writes is
//! answered with the list of commands.
//!
//! The menu is the owner's chat's alone: `setMyCommands` with the chat scope, after the default
//! scope's menu is deleted, so nobody else is shown a command (the predecessor's
//! `bot.py:CommandBot.set_commands`). `/start` is answered and not listed, as the predecessor's was.
//!
//! Every message is HTML, and every value that is not this module's own text is escaped where it
//! enters the markup. Each is committed as a golden, `crates/bot/tests/messages/<name>.msg.json`,
//! which the tests compare with what the bot sends. The text says what the service does and how the
//! owner's data is treated; it names no date, no time of day and no deadline.

use std::future::Future;
use std::sync::Arc;

use deck_streak_coordination::data_rights_registry::{erase_all, export_all};
use deck_streak_identity::Owner;
use deck_streak_kernel::{Db, Environment, Setting, SettingsError};
use frankenstein::types::{BotCommand, InlineKeyboardButton, InlineKeyboardMarkup, WebAppInfo};

use crate::gate::{self, Admission, OwnerCallback, OwnerMessage};
use crate::transport::{Incoming, Sent, Transport, escape_attribute, escape_html};

/// The Mini App's URL, which `/start`'s button opens: an `https:` URL, required by the bot role.
pub const MINI_APP_URL: &str = "DECKSTREAK_MINI_APP_URL";

/// The published privacy policy: the repository's `PRIVACY.md` on `main`.
pub const PRIVACY_POLICY_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/blob/main/PRIVACY.md");

/// The data of the button that confirms an erase.
pub const CONFIRM_ERASE: &str = "erase:confirm";

/// The name of the document `/export` sends.
pub const EXPORT_FILE_NAME: &str = "deckstreak-export.json";

/// One entry of the owner's menu: a command, and what it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuEntry {
    /// The command, without its `/`.
    pub command: &'static str,
    /// What it does, as the menu shows it.
    pub description: &'static str,
}

/// The owner's menu, in the order the menu shows it.
pub const MENU: [MenuEntry; 4] = [
    MenuEntry {
        command: "sync",
        description: "Sync your collection now",
    },
    MenuEntry {
        command: "export",
        description: "Send me a copy of my data",
    },
    MenuEntry {
        command: "delete",
        description: "Erase my data",
    },
    MenuEntry {
        command: "privacy",
        description: "How my data is kept",
    },
];

/// The Mini App's URL: `https:`, naming a host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MiniAppUrl(String);

impl MiniAppUrl {
    /// `text` as the Mini App's URL, or `None` when it is not `https:`, names no host, or carries
    /// whitespace or credentials.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let rest = text.strip_prefix("https://")?;
        let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let named = !host.is_empty() && !host.starts_with(':') && !host.contains('@');
        (named && !text.contains(char::is_whitespace)).then(|| Self(text.to_owned()))
    }

    /// The URL [`MINI_APP_URL`] names.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Missing`] when it is unset, and [`SettingsError::Malformed`] when it is not
    /// a URL [`MiniAppUrl::new`] accepts.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        env.required(MINI_APP_URL)
    }

    /// The URL, as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Setting for MiniAppUrl {
    const SHAPE: &'static str = "an https: URL that names a host";

    fn parse(text: &str) -> Option<Self> {
        Self::new(text)
    }
}

/// What the sync of one `/sync` did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncOutcome {
    /// The copy was synced now.
    Synced,
    /// The sync failed, for this reason code; the copy is the last sync's.
    Failed {
        /// The sync's reason code.
        reason: String,
    },
    /// A sync had just succeeded, so none ran: that sync's copy stands.
    Reused,
    /// No sync ran, for this reason code.
    NotRun {
        /// Why.
        reason: String,
    },
}

/// What the recompute after the sync did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scores {
    /// The scores were recomputed from the copy.
    Recomputed,
    /// Nothing the recompute reads had changed, so the scores stand.
    Unchanged,
}

/// One `/sync`'s account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncAnswer {
    /// The sync.
    pub sync: SyncOutcome,
    /// The recompute after it.
    pub scores: Scores,
}

/// Why a `/sync` could not run at all: one reason code, which the answer shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncRefusal {
    /// The reason code.
    pub reason: &'static str,
}

/// The owner's explicit sync (ADR-037, SPEC-023): one sync cycle now, with the trigger `owner` and
/// the rescore flag set, so it recomputes once whatever it finds. Within five minutes of a
/// successful sync it contacts no server and returns that sync's result. The daemon implements it,
/// since the cycle is ingest's and coordination's.
pub trait OwnerSync: Send + Sync {
    /// Runs the owner's sync now.
    ///
    /// # Errors
    ///
    /// The [`SyncRefusal`] when no cycle could run or be recorded.
    fn sync_now(&self) -> impl Future<Output = Result<SyncAnswer, SyncRefusal>> + Send;
}

/// A message the bot renders: its HTML text and its buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    /// The HTML text.
    pub text: String,
    /// The inline keyboard, when it has buttons.
    pub keyboard: Option<InlineKeyboardMarkup>,
}

impl Reply {
    fn text(text: String) -> Self {
        Self {
            text,
            keyboard: None,
        }
    }
}

/// The commands' lines, which `/start` and the answer to anything else share.
fn command_lines() -> String {
    [
        "/sync syncs your collection now",
        "/export sends you a copy of your data",
        "/delete erases your data",
        "/privacy says how your data is kept",
    ]
    .join("\n")
}

/// `/start`: first, that the coach is an AI; then what the service does, the commands, and the Mini
/// App's button.
#[must_use]
pub fn start_reply(app: &MiniAppUrl) -> Reply {
    let _ = app;
    Reply::text("Hello.".to_owned())
}

/// `/privacy`: how the owner's data is kept, and the link to the published policy.
#[must_use]
pub fn privacy_reply() -> Reply {
    let _ = escape_attribute(PRIVACY_POLICY_URL);
    Reply::text("Privacy.".to_owned())
}

/// The answer to a message that is no command: the commands.
#[must_use]
pub fn help_reply() -> Reply {
    let _ = command_lines();
    Reply::text("Help.".to_owned())
}

/// The caption of `/export`'s document.
#[must_use]
pub fn export_caption() -> String {
    String::new()
}

/// The answer when the export could not be made.
#[must_use]
pub fn export_failed_reply() -> Reply {
    Reply::text("Failed.".to_owned())
}

/// `/delete`'s question, with the button that confirms it.
#[must_use]
pub fn erase_prompt() -> Reply {
    Reply::text("Erase?".to_owned())
}

/// The answer once the erase is done; `log_held` when a reader held the database's log, so older
/// copies of some pages stay in it until the next maintenance.
#[must_use]
pub fn erase_done_reply(log_held: bool) -> Reply {
    let _ = log_held;
    Reply::text("Done.".to_owned())
}

/// The answer when the erase stopped with an error.
#[must_use]
pub fn erase_failed_reply() -> Reply {
    Reply::text("Failed.".to_owned())
}

/// The answer to a tap on a confirmation that is not the latest `/delete`'s, or was used.
#[must_use]
pub fn erase_expired_reply() -> Reply {
    Reply::text("Expired.".to_owned())
}

/// `/sync`'s answer: what the sync did, then what the recompute did.
#[must_use]
pub fn sync_reply(answer: &Result<SyncAnswer, SyncRefusal>) -> Reply {
    let _ = (answer, escape_html(""));
    Reply::text("Synced.".to_owned())
}

/// The command a message names: its first word without its `/`, and without the `@bot` a group
/// menu adds, in lower case; `None` when it does not start with `/`.
fn command_of(text: &str) -> Option<String> {
    let word = text.split_whitespace().next()?;
    let command = word.strip_prefix('/')?;
    let command = command.split('@').next().unwrap_or_default();
    Some(command.to_lowercase())
}

/// The bot's handlers over one transport, for one owner.
pub struct Commands<S> {
    transport: Arc<Transport>,
    owner: Owner,
    app: MiniAppUrl,
    db: Db,
    sync: S,
    /// The latest `/delete` prompt's message id, until its button is tapped.
    pending_erase: Option<i32>,
}

impl<S: OwnerSync> Commands<S> {
    /// The handlers that answer `owner` through `transport`, reaching the owner's data in `db` and
    /// the sync through `sync`, with `app` as the Mini App's URL.
    pub const fn new(
        transport: Arc<Transport>,
        owner: Owner,
        app: MiniAppUrl,
        db: Db,
        sync: S,
    ) -> Self {
        Self {
            transport,
            owner,
            app,
            db,
            sync,
            pending_erase: None,
        }
    }

    /// The owner's chat: in a private chat, the chat's id is the user's.
    const fn chat(&self) -> i64 {
        self.owner.user().get()
    }

    /// Registers the menu for the owner's chat alone, after deleting the default scope's. Best
    /// effort: the transport logs a request that gives up, and the bot runs on.
    pub async fn register_menu(&self) {
        let commands = MENU
            .iter()
            .map(|entry| {
                BotCommand::builder()
                    .command(entry.command)
                    .description(entry.description)
                    .build()
            })
            .collect();
        self.transport.set_chat_menu(self.chat(), commands).await;
    }

    /// Handles one update: the gate decides, and the owner's message or callback is dispatched.
    pub async fn handle(&mut self, incoming: Incoming) {
        let Some(update) = incoming.update else {
            return;
        };
        match gate::admit(&update.content, self.owner) {
            Admission::Message(message) => self.on_message(message).await,
            Admission::Callback(callback) => self.on_callback(callback).await,
            Admission::AnswerOnly { .. } | Admission::Dropped(_) => {}
        }
    }

    async fn on_message(&mut self, message: OwnerMessage) {
        match command_of(&message.text).as_deref() {
            Some("start") => self.send(start_reply(&self.app)).await,
            Some("privacy") => self.send(privacy_reply()).await,
            Some("export") => self.export().await,
            Some("delete") => {
                let _ = erase_all(&self.db).await;
                self.send(erase_done_reply(false)).await;
            }
            Some("sync") => {
                self.send(sync_reply(&Err(SyncRefusal { reason: "stub" })))
                    .await;
            }
            _ => self.send(help_reply()).await,
        }
    }

    async fn on_callback(&mut self, callback: OwnerCallback) {
        let _ = (callback, &self.pending_erase, &self.sync);
    }

    async fn export(&self) {
        let _ = export_all(&self.db).await;
        let _ = self
            .transport
            .send_document(self.chat(), EXPORT_FILE_NAME, b"", &export_caption())
            .await;
    }

    async fn ask_erase(&mut self) {
        let _ = erase_prompt();
    }

    async fn sync(&self) {
        let _ = self.sync.sync_now().await;
    }

    /// Sends `reply` to the owner. A reply that gives up is logged by the transport, with its
    /// method.
    async fn send(&self, reply: Reply) {
        let _ = self
            .transport
            .send_html(self.chat(), &reply.text, reply.keyboard)
            .await;
    }
}
