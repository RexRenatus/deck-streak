//! The bot's commands: the menu, the handlers and the messages they render (SPEC-026 R9, R11, R12;
//! ADR-037).
//!
//! The owner has six commands. `/start` says first that the coach is an AI, and offers the Mini App
//! through a `web_app` button whose URL is [`MINI_APP_URL`]. `/score` answers the current study
//! day's score through coordination's score reads, the ones the Mini App's score screen reads too
//! (SPEC-071 R21; [`crate::score_commands`]). `/privacy` links the published privacy policy. `/export` sends the owner's data, `coordination::export_all`, as one JSON
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
use deck_streak_coordination::drills::{self, DrillMeta, DrillNotes, RealFs, Surface};
use deck_streak_coordination::instruments::InstrumentService;
use deck_streak_coordination::progression::badges_view::earned_badges;
use deck_streak_coordination::progression::level_view::level_view;
use deck_streak_coordination::progression::records_view::records_now;
use deck_streak_coordination::score::day_score;
use deck_streak_coordination::streak_views::streak_view;
use deck_streak_identity::Owner;
use deck_streak_kernel::{Clock, Db, Environment, Setting, SettingsError, StudyDayRule};
use deck_streak_notifications::owner_message;
use frankenstein::types::{BotCommand, InlineKeyboardButton, InlineKeyboardMarkup, WebAppInfo};

use crate::badges_commands::{
    badges_failed_reply, badges_reply, records_failed_reply, records_reply,
};
use crate::drill_commands::{self, ANSWER_PREFIX, VIEW_PREFIX};
use crate::gate::{self, Admission, OwnerCallback, OwnerMessage};
use crate::score_commands::{score_failed_reply, score_reply};
use crate::streak_commands::{streak_failed_reply, streak_reply};
use crate::transport::{Incoming, Sent, Transport, escape_attribute, escape_html};
use crate::xp_commands::{level_failed_reply, level_reply};

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
pub const MENU: [MenuEntry; 11] = [
    MenuEntry {
        command: "score",
        description: "Show today's score",
    },
    MenuEntry {
        command: "level",
        description: "Show your level and XP",
    },
    MenuEntry {
        command: "streak",
        description: "Show your streaks",
    },
    MenuEntry {
        command: "badges",
        description: "Show your badges",
    },
    MenuEntry {
        command: "records",
        description: "Show your personal records",
    },
    MenuEntry {
        command: "drills",
        description: "Answer a law drill",
    },
    MenuEntry {
        command: "drill",
        description: "Pick a law drill by type",
    },
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
    /// The job was asked and had not finished when the bounded wait ended: the owner is told so
    /// instead of being left without an answer (SPEC-059 R5).
    StillRunning,
}

/// What the recompute after the sync did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scores {
    /// The scores were recomputed from the copy.
    Recomputed,
    /// Nothing the recompute reads had changed, so the scores stand.
    Unchanged,
    /// The job refused the recompute after the sync ran (SPEC-128); `reason` is the refusal's code.
    Refused {
        /// The refusal's code, one of the closed set.
        reason: String,
    },
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
    /// A reply of `text` alone, with no button.
    pub(crate) const fn text(text: String) -> Self {
        Self {
            text,
            keyboard: None,
        }
    }
}

/// The commands' lines, which `/start` and the answer to anything else share.
fn command_lines() -> String {
    [
        "/score shows today's score",
        "/level shows your level and XP",
        "/streak shows your streaks",
        "/badges shows your badges",
        "/records shows your personal records",
        "/drills lists the law drills to answer",
        "/drill picks a law drill by type",
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
    let text = format!(
        "Your coach here is an AI, not a person.\n\nDeckStreak reads the reviews you do in Anki \
         from a private copy of your collection. Open the app to see your study.\n\n{}",
        command_lines()
    );
    let open = InlineKeyboardButton::builder()
        .text("Open DeckStreak")
        .web_app(WebAppInfo::builder().url(app.as_str()).build())
        .build();
    Reply {
        text,
        keyboard: Some(
            InlineKeyboardMarkup::builder()
                .inline_keyboard(vec![vec![open]])
                .build(),
        ),
    }
}

/// `/privacy`: how the owner's data is kept, and the link to the published policy.
#[must_use]
pub fn privacy_reply() -> Reply {
    Reply::text(format!(
        "<b>Your data</b>\nDeckStreak keeps only what it needs to run your study, in its own \
         database. <a href=\"{}\">The privacy policy</a> says what that is, why, and for how \
         long.\n\n/export sends you a copy of your data, and /delete erases it.",
        escape_attribute(PRIVACY_POLICY_URL)
    ))
}

/// The answer to a message that is no command: the commands.
#[must_use]
pub fn help_reply() -> Reply {
    Reply::text(format!(
        "These are the commands I answer:\n{}",
        command_lines()
    ))
}

/// The caption of `/export`'s document.
#[must_use]
pub fn export_caption() -> String {
    "Your DeckStreak data, as one JSON document.".to_owned()
}

/// The answer when the export could not be made.
#[must_use]
pub fn export_failed_reply() -> Reply {
    Reply::text(
        "The export could not be made, so nothing was sent. Send /export to try again.".to_owned(),
    )
}

/// `/delete`'s question, with the button that confirms it.
#[must_use]
pub fn erase_prompt() -> Reply {
    let confirm = InlineKeyboardButton::builder()
        .text("Erase my data")
        .callback_data(CONFIRM_ERASE)
        .build();
    Reply {
        text: "<b>Erase your data?</b>\nThis erases everything DeckStreak keeps about your \
               study, and it cannot be undone. /privacy says what an erase does not reach."
            .to_owned(),
        keyboard: Some(
            InlineKeyboardMarkup::builder()
                .inline_keyboard(vec![vec![confirm]])
                .build(),
        ),
    }
}

/// The answer once the erase is done; `log_held` when a reader held the database's log, so older
/// copies of some pages stay in it until the next maintenance.
#[must_use]
pub fn erase_done_reply(log_held: bool) -> Reply {
    let mut text = "Your DeckStreak data was erased.".to_owned();
    if log_held {
        text.push_str(
            "\nOlder copies of some of it stay in the database's log until its next maintenance.",
        );
    }
    Reply::text(text)
}

/// The answer when the erase stopped with an error.
#[must_use]
pub fn erase_failed_reply() -> Reply {
    Reply::text(
        "The erase stopped with an error, so some of your data may remain. Send /export to see \
         what DeckStreak keeps, or /delete to try again."
            .to_owned(),
    )
}

/// The answer to a tap on a confirmation that is not the latest `/delete`'s, or was used.
#[must_use]
pub fn erase_expired_reply() -> Reply {
    Reply::text(
        "That confirmation is no longer valid, so nothing was erased. Send /delete to start \
         again."
            .to_owned(),
    )
}

/// `/sync`'s answer: what the sync did, then what the recompute did.
#[must_use]
pub fn sync_reply(answer: &Result<SyncAnswer, SyncRefusal>) -> Reply {
    let answer = match answer {
        Ok(answer) => answer,
        Err(refusal) => {
            return Reply::text(format!(
                "The sync could not run (<code>{}</code>), so nothing was changed.",
                escape_html(refusal.reason)
            ));
        }
    };
    let sync = match &answer.sync {
        SyncOutcome::Synced => "Synced with your Anki sync server.".to_owned(),
        SyncOutcome::Failed { reason } => format!(
            "The sync failed (<code>{}</code>), so the copy here was not refreshed.",
            escape_html(reason)
        ),
        SyncOutcome::Reused => "A sync had just succeeded, so no new one ran.".to_owned(),
        SyncOutcome::NotRun { reason } => {
            format!("No sync ran (<code>{}</code>).", escape_html(reason))
        }
        SyncOutcome::StillRunning => {
            return Reply::text(
                "The sync is still running. Send /sync again in a minute for its outcome."
                    .to_owned(),
            );
        }
    };
    let scores = match &answer.scores {
        Scores::Recomputed => "Your scores were recomputed from the copy here.".to_owned(),
        Scores::Unchanged => "Nothing they read had changed, so your scores stand.".to_owned(),
        Scores::Refused { reason } => format!(
            "Your scores were not recomputed (<code>{}</code>), so they stand.",
            escape_html(reason)
        ),
    };
    Reply::text(format!("{sync}\n{scores}"))
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
    rule: StudyDayRule,
    clock: Arc<dyn Clock>,
    /// The latest `/delete` prompt's message id, until its button is tapped.
    pending_erase: Option<i32>,
    /// The on-demand instruments, for the commands of the instruments' specs (SPEC-094 R8).
    instruments: Option<Arc<dyn InstrumentService>>,
    /// The drill notes' reader and the answer's writer, when the daemon wired them (SPEC-110).
    drills: Option<Arc<DrillNotes<RealFs>>>,
    /// The one drill the owner's next message answers, in memory only (R13).
    pending_drill: Option<String>,
}

impl<S: OwnerSync> Commands<S> {
    /// The handlers that answer `owner` through `transport`, reaching the owner's data in `db` and
    /// the sync through `sync`, with `app` as the Mini App's URL, and reading the current study day
    /// by `rule` on `clock`.
    pub const fn new(
        transport: Arc<Transport>,
        owner: Owner,
        app: MiniAppUrl,
        db: Db,
        sync: S,
        rule: StudyDayRule,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            transport,
            owner,
            app,
            db,
            sync,
            rule,
            clock,
            pending_erase: None,
            instruments: None,
            drills: None,
            pending_drill: None,
        }
    }

    /// These handlers, holding the on-demand run of the instruments. The bot cannot name the
    /// context that reads the copy, so the daemon hands it the port (SPEC-094 R8).
    #[must_use]
    pub fn with_instruments(mut self, instruments: Arc<dyn InstrumentService>) -> Self {
        self.instruments = Some(instruments);
        self
    }

    /// The on-demand instruments, when the role has them.
    #[must_use]
    pub fn instruments(&self) -> Option<&Arc<dyn InstrumentService>> {
        self.instruments.as_ref()
    }

    /// These handlers, answering the law drills through `notes` (SPEC-110 R13).
    #[must_use]
    pub fn with_drills(mut self, notes: Arc<DrillNotes<RealFs>>) -> Self {
        self.drills = Some(notes);
        self
    }

    /// The owner's chat: in a private chat, the chat's id is the user's.
    const fn chat(&self) -> i64 {
        self.owner.user().get()
    }

    /// Registers the menu for the owner's chat alone, after deleting the default scope's. Best
    /// effort: the transport logs a request that gives up, and the bot runs on.
    pub async fn register_menu(&self) {
        self.transport.delete_default_menu().await;
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
            tracing::info!(
                kind = "update",
                reason = "unreadable",
                "an update was dropped"
            );
            return;
        };
        match gate::admit(&update.content, self.owner) {
            Admission::Message(message) => self.on_message(message).await,
            Admission::Callback(callback) => {
                self.transport.answer_callback(&callback.id).await;
                self.on_callback(callback).await;
            }
            Admission::AnswerOnly {
                callback_id,
                dropped,
            } => {
                tracing::info!(
                    kind = dropped.kind,
                    reason = dropped.reason,
                    "an update was dropped"
                );
                self.transport.answer_callback(&callback_id).await;
            }
            Admission::Dropped(dropped) => {
                tracing::info!(
                    kind = dropped.kind,
                    reason = dropped.reason,
                    "an update was dropped"
                );
            }
        }
    }

    async fn on_message(&mut self, message: OwnerMessage) {
        // The owner's latest message, which a T1 celebration reacts to (SPEC-084 R13).
        let at = self.clock.now();
        if let Err(error) = owner_message::record(&self.db, i64::from(message.message_id), at).await
        {
            tracing::warn!(%error, "the owner's latest message was not recorded");
        }
        let command = command_of(&message.text);
        if command.is_some() {
            self.pending_drill = None;
        }
        match command.as_deref() {
            Some("start") => self.send(start_reply(&self.app)).await,
            Some("privacy") => self.send(privacy_reply()).await,
            Some("export") => self.export().await,
            Some("delete") => self.ask_erase().await,
            Some("sync") => self.sync().await,
            Some("score") => self.score().await,
            Some("level") => self.level().await,
            Some("streak") => self.streak().await,
            Some("badges") => self.badges().await,
            Some("records") => self.records().await,
            Some("drills") => self.drills().await,
            Some("drill") => self.drill(&message.text).await,
            None if self.pending_drill.is_some() => self.drill_answer(&message.text).await,
            _ => self.send(help_reply()).await,
        }
    }

    async fn on_callback(&mut self, callback: OwnerCallback) {
        if let Some(data) = callback.data.as_deref() {
            if data.starts_with(VIEW_PREFIX) {
                return self.drill_view(data).await;
            }
            if data.starts_with(ANSWER_PREFIX) {
                return self.drill_ask(data).await;
            }
        }
        if callback.data.as_deref() != Some(CONFIRM_ERASE) {
            tracing::info!(
                kind = "callback_query",
                reason = "data_unknown",
                "a callback did nothing"
            );
            return;
        }
        let latest = self.pending_erase.is_some() && self.pending_erase == callback.message_id;
        if !latest {
            tracing::info!(
                kind = "callback_query",
                reason = "confirmation_stale",
                "nothing was erased"
            );
            self.send(erase_expired_reply()).await;
            return;
        }
        self.pending_erase = None;
        match erase_all(&self.db).await {
            Ok(erasure) => {
                tracing::info!(
                    emptied = erasure.emptied.len(),
                    reset = erasure.reset.len(),
                    kept = erasure.kept.len(),
                    checkpoint_busy = erasure.checkpoint_busy,
                    "the owner's data was erased"
                );
                self.send(erase_done_reply(erasure.checkpoint_busy)).await;
            }
            Err(error) => {
                tracing::error!(%error, "the owner's erase stopped");
                self.send(erase_failed_reply()).await;
            }
        }
    }

    async fn export(&self) {
        self.transport.send_typing(self.chat()).await;
        match export_all(&self.db).await {
            Ok(export) => {
                let document = export.to_line();
                // A document that gives up is logged by the transport, with its method.
                let _ = self
                    .transport
                    .send_document(
                        self.chat(),
                        EXPORT_FILE_NAME,
                        document.as_bytes(),
                        &export_caption(),
                    )
                    .await;
            }
            Err(error) => {
                tracing::error!(%error, "the owner's export could not be made");
                self.send(export_failed_reply()).await;
            }
        }
    }

    async fn ask_erase(&mut self) {
        let prompt = erase_prompt();
        self.pending_erase = match self
            .transport
            .send_html(self.chat(), &prompt.text, prompt.keyboard)
            .await
        {
            Sent::Delivered { message_id } => Some(message_id),
            Sent::Failed => None,
        };
    }

    async fn sync(&self) {
        self.transport.send_typing(self.chat()).await;
        let answer = self.sync.sync_now().await;
        self.send(sync_reply(&answer)).await;
    }

    /// `/score`: the current study day's score, through coordination's score reads (SPEC-071
    /// R21).
    async fn score(&self) {
        let today = self.rule.study_day(self.clock.now());
        let reply = match day_score(&self.db, today).await {
            Ok(score) => score_reply(score.as_ref()),
            Err(error) => {
                tracing::error!(%error, "the owner's score could not be read");
                score_failed_reply()
            }
        };
        self.send(reply).await;
    }

    /// `/level`: the level and the day's XP, through coordination's level view (SPEC-072 R25).
    async fn level(&self) {
        let today = self.rule.study_day(self.clock.now());
        let reply = match level_view(&self.db, today).await {
            Ok(view) => level_reply(&view),
            Err(error) => {
                tracing::error!(%error, "the owner's level could not be read");
                level_failed_reply()
            }
        };
        self.send(reply).await;
    }

    /// `/streak`: both tracks, the law track first when it has activity (SPEC-076 R22).
    async fn streak(&self) {
        let today = self.rule.study_day(self.clock.now());
        let reply = match streak_view(&self.db, today).await {
            Ok(view) => streak_reply(&view),
            Err(error) => {
                tracing::error!(%error, "the owner's streaks could not be read");
                streak_failed_reply()
            }
        };
        self.send(reply).await;
    }

    /// `/badges`: the twenty most recently awarded badges, newest first (SPEC-073 R18).
    async fn badges(&self) {
        let reply = match earned_badges(&self.db).await {
            Ok(earned) => badges_reply(&earned),
            Err(error) => {
                tracing::error!(%error, "the owner's badges could not be read");
                badges_failed_reply()
            }
        };
        self.send(reply).await;
    }

    /// `/records`: each record, then the record to chase (SPEC-073 R18).
    async fn records(&self) {
        let today = self.rule.study_day(self.clock.now());
        let reply = match records_now(&self.db, today).await {
            Ok(view) => records_reply(&view),
            Err(error) => {
                tracing::error!(%error, "the owner's records could not be read");
                records_failed_reply()
            }
        };
        self.send(reply).await;
    }

    /// `/drills`: the unanswered drills (SPEC-110 R13).
    async fn drills(&self) {
        let reply = match self.unanswered() {
            Some(unanswered) => drill_commands::list_reply("Unanswered drills", &unanswered),
            None => drill_commands::unavailable_reply(),
        };
        self.send(reply).await;
    }

    /// `/drill [code]`: the four types, or one type's unanswered drills (R14).
    async fn drill(&self, text: &str) {
        let reply = match text.split_whitespace().nth(1) {
            None => drill_commands::types_reply(),
            Some(code) => match drill_commands::kind_of(code) {
                None => drill_commands::refusal_reply(),
                Some(kind) => match self.unanswered() {
                    Some(all) => {
                        let of_kind: Vec<_> = all.into_iter().filter(|m| m.kind == kind).collect();
                        drill_commands::list_reply(kind, &of_kind)
                    }
                    None => drill_commands::unavailable_reply(),
                },
            },
        };
        self.send(reply).await;
    }

    /// A tap on a drill's button: its single view (R13).
    async fn drill_view(&self, data: &str) {
        let reply = match self.named(drill_commands::VIEW_PREFIX, data) {
            Some(id) => {
                let today = self.rule.study_day(self.clock.now());
                self.drills
                    .as_ref()
                    .and_then(|notes| notes.view(&id, today))
                    .map_or_else(drill_commands::gone_reply, |view| {
                        drill_commands::view_reply(&view)
                    })
            }
            None => drill_commands::gone_reply(),
        };
        self.send(reply).await;
    }

    /// A tap on a view's Answer button: the next message is the answer (R13).
    async fn drill_ask(&mut self, data: &str) {
        let reply = match self.named(drill_commands::ANSWER_PREFIX, data) {
            Some(id) => {
                let today = self.rule.study_day(self.clock.now());
                let view = self
                    .drills
                    .as_ref()
                    .and_then(|notes| notes.view(&id, today))
                    .filter(|view| !view.meta.answered);
                if let Some(view) = view {
                    self.pending_drill = Some(id);
                    drill_commands::ask_reply(&view.meta.title)
                } else {
                    drill_commands::gone_reply()
                }
            }
            None => drill_commands::gone_reply(),
        };
        self.send(reply).await;
    }

    /// The owner's answer to the pending drill (R13).
    async fn drill_answer(&mut self, text: &str) {
        let pending = self.pending_drill.take();
        let reply = match (pending, self.drills.as_ref()) {
            (Some(id), Some(notes)) => {
                let at = self.clock.now();
                match drills::answer(notes, &self.db, &id, text, Surface::Bot, self.rule, at).await
                {
                    Ok(outcome) => drill_commands::outcome_reply(&outcome),
                    Err(error) => {
                        tracing::error!(%error, "the owner's drill answer could not be recorded");
                        drill_commands::unavailable_reply()
                    }
                }
            }
            _ => drill_commands::gone_reply(),
        };
        self.send(reply).await;
    }

    /// The unanswered drills now, or none when the vault is not wired or cannot be read.
    fn unanswered(&self) -> Option<Vec<DrillMeta>> {
        let today = self.rule.study_day(self.clock.now());
        let listed = self.drills.as_ref()?.list_active(today).ok()?;
        Some(listed.into_iter().filter(|meta| !meta.answered).collect())
    }

    /// The drill `data` names among those unanswered now (a hashed token needs the list).
    fn named(&self, prefix: &str, data: &str) -> Option<String> {
        let offered: Vec<String> = self
            .unanswered()?
            .into_iter()
            .map(|meta| meta.drill_id)
            .collect();
        drill_commands::resolve_token(prefix, data, &offered)
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
