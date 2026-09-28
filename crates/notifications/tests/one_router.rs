//! The compiler holds the router's port, and a census reads the ways around it (SPEC-041 A2, A15;
//! R1).
//!
//! A2: each bot transport call takes the router's `Pass`, which no other module can make, so a
//! call of the port anywhere else does not compile. One compile-fail case for each way a pass
//! could be had outside the router, each with the refusal its `.stderr` records: its private field
//! (`tests/ui/push_outside_the_router.rs`), its `Default` (`tests/ui/pass_by_default.rs`), and a
//! clone of the pass a transport borrows (`tests/ui/pass_kept_by_a_clone.rs`). The records move
//! with the pinned toolchain, and are regenerated in the change that bumps it
//! (`TRYBUILD=overwrite`).
//!
//! A15: a delivery can go around the port and never take a pass. The census reads every shipped
//! source of these kinds: the Rust, Python and web source files and the shell scripts by their
//! extensions, a script with none by its `#!` first line, and the systemd units and their drop-ins.
//! It leaves out test files, test directories outside a `src/`, and in a Rust file its comments and
//! `#[cfg(test)]` modules. Outside the bot's sources nothing may name the Bot API's host, one of its
//! send methods, in the API's spelling or a client's, or the bot's `DEFAULT_API_URL`, SPEC-031's
//! alert path aside; inside them a send method is named only in the named send that makes its
//! request. The bot's `send_html`, its `edit_html` and its command handler are used only at named
//! call sites, and each of the seven named sends is found exactly once. Only the router's ledger,
//! router and data-rights modules, which own their writes, name the Mini App's feed or the held
//! queue, which a flush delivers; and because the ledger's writes to the queue are private to the
//! notifications crate, only they name the ledger in that crate's sources, the root's declaration
//! of it aside. The refusals are proved on the reviews' deliveries around the port, which the test
//! holds as text, and on a tree it writes for the walker. A text census reads names, so a request
//! or a table's name assembled from parts, in which no name it reads appears, and a source of a
//! kind it does not read, go unread (#297).

// An integration test is test code: its helpers panic on a failed fixture, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::io::Read;
use std::path::Path;

/// The directories the census never enters: version control, dependencies, build output, caches.
const SKIPPED_DIRECTORIES: [&str; 14] = [
    ".git",
    "node_modules",
    "target",
    ".svelte-kit",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".turbo",
    "coverage",
    ".output",
    ".cache",
];

/// The directories that hold tests, which ship nothing.
const TEST_DIRECTORIES: [&str; 7] = [
    "tests",
    "test",
    "__tests__",
    "e2e",
    "fixtures",
    "__mocks__",
    "testdata",
];

/// A shipped source's extensions: the Rust, Python and web sources, the shell scripts and the
/// systemd units.
const SHIPPED_EXTENSIONS: [&str; 15] = [
    "rs", "py", "ts", "js", "mjs", "cjs", "mts", "tsx", "jsx", "svelte", "vue", "astro", "sh",
    "service", "timer",
];

/// The Bot API's host.
const BOT_API_HOST: &str = "api.telegram.org";

/// The bot's constant for the Bot API's base URL, which no other crate can read (it is
/// `pub(crate)`), and no other source may name.
const BOT_API_URL: &str = "DEFAULT_API_URL";

/// The Bot API's send methods, in its own spelling; a client library spells each in snake case
/// (`send_message`).
const SEND_METHODS: [&str; 22] = [
    "sendMessage",
    "sendPhoto",
    "sendAudio",
    "sendDocument",
    "sendVideo",
    "sendAnimation",
    "sendVoice",
    "sendVideoNote",
    "sendPaidMedia",
    "sendMediaGroup",
    "sendLocation",
    "sendVenue",
    "sendContact",
    "sendPoll",
    "sendChecklist",
    "sendDice",
    "sendChatAction",
    "sendSticker",
    "sendInvoice",
    "sendGame",
    "sendGift",
    "sendMessageDraft",
];

/// The bot's own send, which takes no pass.
const BOT_SEND: &str = "send_html";

/// The bot's sources, where the Bot API is named: a send method only by its own named send.
const BOT_SOURCES: &str = "crates/bot/src/";

/// The bot's own ways to reach the owner's chat that take no pass, each with the file that defines
/// it: every use of one is its definition there or at a named call site. Its send; its edit of a
/// message, which no shipped source calls, so it has no named call site; and its command handler,
/// which answers an update with a reply the router never decides.
const GUARDED: [(&str, &str); 3] = [
    (BOT_SEND, "crates/bot/src/transport.rs"),
    ("edit_html", "crates/bot/src/transport.rs"),
    ("handle", "crates/bot/src/commands.rs"),
];

/// The one use of the bot's command handler: the bot's entry, the long poll, hands it each update
/// the Bot API delivers to the bot.
const HANDLER_ENTRY: (&str, &str, &str) = ("crates/bot/src/poll.rs", "run", "handle");

/// The router's modules, the only sources that may name the Mini App's feed: the ledger, which
/// writes and reads it; the router, whose `push_in_app` appends to it; and the data-rights port,
/// which exports and erases it.
const FEED_MODULES: [&str; 3] = [
    "crates/notifications/src/ledger.rs",
    "crates/notifications/src/router.rs",
    "crates/notifications/src/data_rights.rs",
];

/// The Mini App's feed's table, named in SQL in any case.
const FEED_TABLE_NAME: &str = "in_app_feed";

/// The router's names for the feed: the ledger's constant for its table, and its append.
const FEED_NAMES: [&str; 2] = ["FEED_TABLE", "append_feed"];

/// The modules that own the held queue's writes, the only sources that may name it: the ledger,
/// which holds the SQL of each write (`hold`, `abandon`, `relatch`, `settle`) and reads it; the
/// router, their one caller, which holds, flushes and retries; and the data-rights port, which
/// exports and erases it. A write to the queue anywhere else is a delivery around the router,
/// because a flush delivers what the queue holds.
const QUEUE_MODULES: [&str; 3] = [
    "crates/notifications/src/ledger.rs",
    "crates/notifications/src/router.rs",
    "crates/notifications/src/data_rights.rs",
];

/// The held queue's table, named in SQL in any case.
const QUEUE_TABLE_NAME: &str = "notification_queue";

/// The router's names for the queue: the ledger's constant for its table.
const QUEUE_NAMES: [&str; 1] = ["QUEUE_TABLE"];

/// The ledger, as (the notifications crate's sources, its root, the ledger's name): the ledger's
/// writes to the queue are private to the crate, so a call of one in the crate's sources names the
/// ledger, which only the root's declaration of it names outside the queue's modules.
const LEDGER: (&str, &str, &str) = (
    "crates/notifications/src/",
    "crates/notifications/src/lib.rs",
    "ledger",
);

/// SPEC-031's alert path: it pages the owner that a unit failed, the daemon among them, so it
/// cannot go through the daemon's router. The one shipped source outside the bot that names the
/// Bot API.
const ALERT_PATH: &str = "deploy/scripts/alert-telegram.sh";

/// Every send, and where it is made: (file, function, send). The census finds each once.
const NAMED_SENDS: [(&str, &str, &str); 7] = [
    // The bot's command replies (#257): the erase prompt, every other reply, and the export.
    (
        "crates/bot/src/commands.rs",
        "Commands::ask_erase",
        "send_html",
    ),
    (
        "crates/bot/src/commands.rs",
        "Commands::export",
        "send_document",
    ),
    ("crates/bot/src/commands.rs", "Commands::send", "send_html"),
    // The port's implementation, which delivers the router's pushes.
    (
        "crates/bot/src/transport.rs",
        "OwnerChat::push_message",
        "send_html",
    ),
    // The transport's own requests.
    (
        "crates/bot/src/transport.rs",
        "Transport::send_html",
        "send_message",
    ),
    (
        "crates/bot/src/transport.rs",
        "Transport::send_typing",
        "send_chat_action",
    ),
    // The export's document, posted by the transport's own request.
    (
        "crates/bot/src/transport.rs",
        "Transport::send_document",
        "sendDocument",
    ),
];

/// The reviews' deliveries around the port, as they planted them. The first review's: the bot's own
/// send and a raw request to the Bot API in the bot's role, and a raw request in the API; the bot's
/// role also carries a comment and a test module that name and call a send, which the census leaves
/// out. The second review's: the bot's own send after a field compiled for tests alone, beside a
/// test module under stacked attributes, which the census leaves out; the bot's own send named
/// through a variable; a raw request from a new module of the bot; the bot's edit of a message; a
/// fabricated update handed to the bot's command handler; a raw request built on the bot's base
/// URL, its method assembled from parts; and three writes to the Mini App's feed around the router:
/// through the ledger's table name from the API, in SQL from a script, and through the ledger's
/// append from a module beside the router. The architect's ruling on the second review: four writes
/// to the held queue around the router, which a flush delivers: through the ledger's table name
/// from the flush step's crate, in SQL from a script, through the ledger's own writes from a module
/// beside the router, and through one of them re-exported at the crate's root.
const AROUND_THE_PORT: [(&str, &str); 15] = [
    (
        "crates/daemon/src/role_bot.rs",
        r#"/// A celebration sent straight to the owner's chat through the bot's transport, around the router.
async fn celebrate_around_the_router(transport: &Transport, owner: Owner) {
    let _sent = transport
        .send_html(owner.user().get(), "a celebration the router never decided", None)
        .await;
}

/// A nudge posted straight to the Bot API, around the router and the bot's transport.
async fn nudge_over_raw_http(token: &str, chat: i64) {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let body = serde_json::json!({ "chat_id": chat, "text": "a nudge the router never decided" });
    let _answer = reqwest::Client::new().post(url).json(&body).send().await;
}

// A comment may name sendMessage and api.telegram.org, and a test module may send.
#[cfg(test)]
mod tests {
    async fn a_test_sends(transport: &Transport) {
        let _sent = transport.send_html(1, "synthetic", None).await;
    }
}
"#,
    ),
    (
        "crates/api/src/notifications_routes.rs",
        r#"/// A celebration posted straight to the Bot API from the API, around the router.
async fn celebrate_over_raw_http(token: &str, chat: i64) {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let body = serde_json::json!({ "chat_id": chat, "text": "a celebration the router never decided" });
    let _answer = reqwest::Client::new().post(url).json(&body).send().await;
}
"#,
    ),
    (
        "crates/daemon/src/role_job.rs",
        r#"/// A probe with a field compiled for tests alone.
struct Probe {
    #[cfg(test)]
    calls: u8,
    kept: u8,
}

/// A celebration sent straight to the owner's chat through the bot's transport, around the router.
async fn celebrate_around_the_router(transport: &Transport, owner: Owner) {
    let _sent = transport
        .send_html(owner.user().get(), "a celebration the router never decided", None)
        .await;
}

// A test module under stacked attributes may send.
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    async fn a_test_sends(transport: &Transport) {
        let _sent = transport.send_html(1, "synthetic", None).await;
    }
}
"#,
    ),
    (
        "crates/daemon/src/role_data.rs",
        r#"/// A celebration through the bot's own send, the method named through a variable.
async fn celebrate_through_a_variable(transport: &Transport, owner: Owner) {
    let send = Transport::send_html;
    let _sent = send(transport, owner.user().get(), "a celebration the router never decided", None).await;
}
"#,
    ),
    (
        "crates/bot/src/celebrate.rs",
        r#"//! A celebration from inside the bot crate, around the transport's named sends.

/// A celebration posted straight to the Bot API by a new module of the bot, around the router.
pub async fn celebrate(api_url: &str, token: &str, chat: i64) {
    let url = format!("{api_url}/bot{token}/sendMessage");
    let body = serde_json::json!({ "chat_id": chat, "text": "a celebration the router never decided" });
    let _answer = reqwest::Client::new().post(url).json(&body).send().await;
}
"#,
    ),
    (
        "crates/daemon/src/lifecycle.rs",
        r#"/// A celebration written over a message already in the owner's chat, around the router.
async fn celebrate_by_an_edit(transport: &Transport, owner: Owner, message_id: i32) {
    let _edited = transport
        .edit_html(owner.user().get(), message_id, "a celebration the router never decided")
        .await;
}
"#,
    ),
    (
        "crates/daemon/src/main.rs",
        r#"/// A reply the router never decided: a fabricated owner command, answered by the bot.
async fn celebrate_by_a_fabricated_command<S: OwnerSync>(commands: &mut Commands<S>, chat: i64) {
    let update = serde_json::json!({ "update_id": 1, "message": { "message_id": 1, "date": 0,
        "chat": { "id": chat, "type": "private" }, "from": { "id": chat, "is_bot": false, "first_name": "o" },
        "text": "/help" } });
    commands.handle(Incoming::from_value(update)).await;
}
"#,
    ),
    (
        "crates/daemon/src/wiring.rs",
        r#"/// A celebration posted to the Bot API on the bot crate's base URL, its method assembled.
async fn celebrate_over_a_built_url(token: &str, chat: i64) {
    let method = ["send", "Message"].concat();
    let url = format!("{}/bot{token}/{method}", deck_streak_bot::transport::DEFAULT_API_URL);
    let body = serde_json::json!({ "chat_id": chat, "text": "a celebration the router never decided" });
    let _answer = reqwest::Client::new().post(url).json(&body).send().await;
}
"#,
    ),
    (
        "crates/api/src/router.rs",
        r#"/// A celebration written straight into the Mini App's feed, around the router: the feed route
/// serves it to the owner's session as if the router had decided it.
async fn celebrate_in_the_feed(db: &Db, now: i64) -> Result<(), sqlx::Error> {
    let insert = format!(
        "INSERT INTO {} (dedupe_key, kind, tier, text, seen_at, created_at) VALUES (?, ?, ?, ?, NULL, ?)",
        deck_streak_notifications::ledger::FEED_TABLE
    );
    let mut write = db.write().await.map_err(|_| sqlx::Error::PoolClosed)?;
    sqlx::query(&insert)
        .bind("celebration:around")
        .bind("celebration")
        .bind("T2")
        .bind("a celebration the router never decided")
        .bind(now)
        .execute(&mut *write)
        .await?;
    write.commit().await
}
"#,
    ),
    (
        "deploy/scripts/celebrate.py",
        r#""""A celebration written straight into the Mini App's feed, around the router."""

import sqlite3


def celebrate(database: str, now: int) -> None:
    with sqlite3.connect(database) as db:
        db.execute(
            "INSERT INTO IN_APP_FEED (dedupe_key, kind, tier, text, seen_at, created_at)"
            " VALUES ('celebration:around', 'celebration', 'T2', 'a celebration', NULL, ?)",
            (now,),
        )
"#,
    ),
    (
        "crates/notifications/src/occasion.rs",
        r#"/// A celebration appended to the Mini App's feed beside the router, around its decision.
async fn celebrate_beside_the_router(write: &mut SqliteConnection, now: UtcMillis) -> Result<(), KernelError> {
    ledger::append_feed(write, "celebration:around", "celebration", Tier::T2, "a celebration", now).await
}
"#,
    ),
    (
        "crates/coordination/src/sync_cycle.rs",
        r#"/// A celebration held straight on the queue from the flush step's crate, around the router: the
/// next flush delivers it as if the router had held it.
async fn celebrate_on_the_queue(db: &Db, now: i64) -> Result<(), sqlx::Error> {
    let insert = format!(
        "INSERT INTO {} (kind, dedupe_key, surface, tier_requested, tier_pending, text, hold, tries, state, deferred_at, study_day, created_at) VALUES (?, ?, 'bot', 'T2', 'T2', ?, 'quiet', 0, 'held', ?, 0, ?)",
        deck_streak_notifications::ledger::QUEUE_TABLE
    );
    let mut write = db.write().await.map_err(|_| sqlx::Error::PoolClosed)?;
    sqlx::query(&insert)
        .bind("celebration")
        .bind("celebration:around")
        .bind("a celebration the router never decided")
        .bind(now)
        .bind(now)
        .execute(&mut *write)
        .await?;
    write.commit().await
}
"#,
    ),
    (
        "deploy/scripts/hold.py",
        r#""""A celebration held straight on the queue from a script, around the router."""

import sqlite3


def hold(database: str, now: int) -> None:
    with sqlite3.connect(database) as db:
        db.execute(
            "INSERT INTO Notification_Queue (kind, dedupe_key, surface, tier_requested,"
            " tier_pending, text, hold, tries, state, deferred_at, study_day, created_at)"
            " VALUES ('celebration', 'celebration:around', 'bot', 'T2', 'T2', 'a celebration',"
            " 'quiet', 0, 'held', ?, 0, ?)",
            (now, now),
        )
"#,
    ),
    (
        "crates/notifications/src/quiet.rs",
        r"use crate::ledger::hold as keep;

/// A celebration held on the queue beside the router, around its decision, and its retry latched
/// without the router: the next flush delivers it.
async fn celebrate_on_the_queue(write: &mut SqliteConnection, row: &HeldRow, now: UtcMillis) -> Result<(), KernelError> {
    keep(write, row, now).await?;
    crate::ledger::relatch(write, 1, 0).await
}
",
    ),
    (
        "crates/notifications/src/lib.rs",
        r"pub mod ledger;
pub mod quiet;

/// The ledger's hold, re-exported at the crate's root, so a module beside the router holds a
/// celebration on the queue without naming the ledger.
pub(crate) use ledger::hold;
",
    ),
];

/// The second review's trees for the walker, each file as (path, text): a shipped module in a
/// directory named as tests are, which the walker reads because it is under `src/`; the same text
/// in a test directory outside `src/`, which the walker leaves out; a script with no extension but
/// a `#!` first line; and a drop-in of the bot's unit.
const WALKED: [(&str, &str); 4] = [
    (
        "crates/daemon/src/fixtures/celebrate.rs",
        AROUND_THE_PORT_IN_A_MODULE,
    ),
    ("crates/daemon/tests/around.rs", AROUND_THE_PORT_IN_A_MODULE),
    (
        "deploy/scripts/celebrate",
        r#"#!/usr/bin/env bash
# A celebration paged straight to the owner, around the router.
set -euo pipefail
curl -fsS "https://api.telegram.org/bot${BOT_TOKEN}/sendMessage" -d chat_id="${OWNER_CHAT}" -d text="a celebration"
"#,
    ),
    (
        "deploy/systemd/deck-streak-bot.service.d/celebrate.conf",
        r#"[Service]
ExecStartPost=/usr/bin/curl -fsS "https://api.telegram.org/bot${BOT_TOKEN}/sendMessage" -d chat_id=${OWNER_CHAT} -d text=celebrate
"#,
    ),
];

/// A shipped module that sends through the bot's own send, around the router.
const AROUND_THE_PORT_IN_A_MODULE: &str = r#"//! Shipped helpers, in a directory named as tests are.

/// A celebration sent straight to the owner's chat through the bot's transport, around the router.
pub async fn celebrate_around_the_router(transport: &Transport, owner: Owner) {
    let _sent = transport
        .send_html(owner.user().get(), "a celebration the router never decided", None)
        .await;
}
"#;

/// The attribute of an item that is compiled for tests alone.
const CFG_TEST: &str = "#[cfg(test)]";

/// What a census of some sources found.
#[derive(Debug, Default)]
struct Census {
    /// Each refusal, as (path, line, what), in order.
    refusals: Vec<(String, usize, String)>,
    /// Each call of a send, as (path, function, send), in order.
    sends: Vec<(String, String, String)>,
}

impl Census {
    /// The refusals, each as `path:line: what`.
    fn refused(&self) -> Vec<String> {
        self.refusals
            .iter()
            .map(|(path, line, what)| format!("{path}:{line}: {what}"))
            .collect()
    }
}

/// The census of `sources`, each a path from the tree's root and its text.
fn census(sources: &[(String, String)]) -> Census {
    let mut found = Census::default();
    for (path, text) in sources {
        let rust = Path::new(path)
            .extension()
            .is_some_and(|extension| extension == "rs");
        let (code, structure) = if rust {
            rust_code(text)
        } else {
            (text.clone(), text.clone())
        };
        if path.starts_with(BOT_SOURCES) {
            for (at, name) in names_of_a_send(&code) {
                if defined(&structure, at) {
                    continue;
                }
                let function = enclosing(&structure, at);
                if !by_its_named_send(path, &function, &name) {
                    found.refusals.push((
                        path.clone(),
                        line_of(&code, at),
                        format!("names {name} in {function}, not a named call site"),
                    ));
                }
            }
        } else if path != ALERT_PATH {
            for (at, name) in names_of_the_bot_api(&code) {
                found
                    .refusals
                    .push((path.clone(), line_of(&code, at), format!("names {name}")));
            }
        }
        if !FEED_MODULES.contains(&path.as_str()) {
            for (at, name) in names_of_the_feed(&code) {
                found
                    .refusals
                    .push((path.clone(), line_of(&code, at), format!("names {name}")));
            }
        }
        if !QUEUE_MODULES.contains(&path.as_str()) {
            for (at, name) in names_of_the_queue(path, &code, &structure) {
                found
                    .refusals
                    .push((path.clone(), line_of(&code, at), format!("names {name}")));
            }
        }
        if rust {
            for (name, definer) in GUARDED {
                for at in identifiers(&structure, name) {
                    if path == definer && defined(&structure, at) {
                        continue;
                    }
                    let function = enclosing(&structure, at);
                    if !at_a_named_site(path, &function, name) {
                        let how = if called(&structure, at, name) {
                            "calls"
                        } else {
                            "uses"
                        };
                        found.refusals.push((
                            path.clone(),
                            line_of(&code, at),
                            format!("{how} {name} in {function}, not a named call site"),
                        ));
                    }
                }
            }
            for (at, send) in sends(&code, &structure) {
                let function = enclosing(&structure, at);
                found.sends.push((path.clone(), function, send));
            }
        }
    }
    found.refusals.sort();
    found.sends.sort();
    found
}

/// Whether `name` is used in `function` of `path` at one of its named call sites: a named send, or
/// the command handler's entry.
fn at_a_named_site(path: &str, function: &str, name: &str) -> bool {
    NAMED_SENDS.contains(&(path, function, name)) || HANDLER_ENTRY == (path, function, name)
}

/// Whether the Bot API method `name`, in either spelling, is named in `function` of `path` by the
/// named send that makes that method's request.
fn by_its_named_send(path: &str, function: &str, name: &str) -> bool {
    NAMED_SENDS
        .iter()
        .any(|&(file, site, send)| file == path && site == function && snake(send) == snake(name))
}

/// What `code` names of the Bot API: its host, in any case, the bot's constant for its base URL,
/// and its send methods in either spelling, each as the byte it starts at and the name.
fn names_of_the_bot_api(code: &str) -> Vec<(usize, String)> {
    let lower = code.to_ascii_lowercase();
    let mut named: Vec<(usize, String)> = lower
        .match_indices(BOT_API_HOST)
        .map(|(at, host)| (at, host.to_owned()))
        .collect();
    named.extend(identifiers(code, BOT_API_URL).map(|at| (at, BOT_API_URL.to_owned())));
    named.extend(names_of_a_send(code));
    named
}

/// What `code` names of the Mini App's feed: its table, in any case, and the router's names for it,
/// each as the byte it starts at and the name.
fn names_of_the_feed(code: &str) -> Vec<(usize, String)> {
    let lower = code.to_ascii_lowercase();
    let mut named: Vec<(usize, String)> = identifiers(&lower, FEED_TABLE_NAME)
        .map(|at| (at, FEED_TABLE_NAME.to_owned()))
        .collect();
    for name in FEED_NAMES {
        named.extend(identifiers(code, name).map(|at| (at, name.to_owned())));
    }
    named
}

/// What the source at `path` names of the held queue: its table, in any case, the router's names
/// for it, and in the notifications crate's sources the ledger, whose writes to the queue are
/// private to that crate, the root's declaration of it aside. Each is read from its `code`, as the
/// byte it starts at and the name; a declaration is read from its `structure`.
fn names_of_the_queue(path: &str, code: &str, structure: &str) -> Vec<(usize, String)> {
    let lower = code.to_ascii_lowercase();
    let mut named: Vec<(usize, String)> = identifiers(&lower, QUEUE_TABLE_NAME)
        .map(|at| (at, QUEUE_TABLE_NAME.to_owned()))
        .collect();
    for name in QUEUE_NAMES {
        named.extend(identifiers(code, name).map(|at| (at, name.to_owned())));
    }
    let (sources, root, ledger) = LEDGER;
    if path.starts_with(sources) {
        named.extend(
            identifiers(code, ledger)
                .filter(|&at| !(path == root && declared(structure, at)))
                .map(|at| (at, ledger.to_owned())),
        );
    }
    named
}

/// Each name of a Bot API send method in `code`, in either spelling, as the byte it starts at and
/// the name.
fn names_of_a_send(code: &str) -> Vec<(usize, String)> {
    let mut named = Vec::new();
    for method in SEND_METHODS {
        for spelling in [method.to_owned(), snake(method)] {
            named.extend(identifiers(code, &spelling).map(|at| (at, spelling.clone())));
        }
    }
    named
}

/// Each send in a Rust source, as the byte it starts at and the send: a call of the bot's own or of
/// a Bot API send method in a client's spelling, a definition aside, read from its `structure`; and
/// a raw request, a send method in the Bot API's spelling right after a `/`, read from its `code`,
/// where a request's URL is a literal.
fn sends(code: &str, structure: &str) -> Vec<(usize, String)> {
    let mut found = send_calls(structure);
    for method in SEND_METHODS {
        found.extend(
            identifiers(code, method)
                .filter(|&at| code[..at].ends_with('/'))
                .map(|at| (at, method.to_owned())),
        );
    }
    found
}

/// Each call of a send in `structure`, a definition aside, as the byte it starts at and the send:
/// the bot's own, and the Bot API's in a client's spelling.
fn send_calls(structure: &str) -> Vec<(usize, String)> {
    let sends = std::iter::once(BOT_SEND.to_owned()).chain(SEND_METHODS.map(snake));
    let mut calls = Vec::new();
    for send in sends {
        for at in identifiers(structure, &send) {
            if called(structure, at, &send) && !defined(structure, at) {
                calls.push((at, send.clone()));
            }
        }
    }
    calls
}

/// Whether the identifier `name` at the byte `at` of `structure` is called.
fn called(structure: &str, at: usize, name: &str) -> bool {
    structure[at + name.len()..].trim_start().starts_with('(')
}

/// Whether the identifier at the byte `at` of `structure` is the name a `fn` defines.
fn defined(structure: &str, at: usize) -> bool {
    structure[..at]
        .trim_end()
        .strip_suffix("fn")
        .is_some_and(|rest| rest.chars().next_back().is_none_or(|c| !ident(c)))
}

/// Whether the identifier at the byte `at` of `structure` is the name a `mod` declares.
fn declared(structure: &str, at: usize) -> bool {
    structure[..at]
        .trim_end()
        .strip_suffix("mod")
        .is_some_and(|rest| rest.chars().next_back().is_none_or(|c| !ident(c)))
}

/// A Bot API method's name as a client library spells it: `sendMessage` as `send_message`.
fn snake(method: &str) -> String {
    let mut spelled = String::with_capacity(method.len() + 4);
    for c in method.chars() {
        if c.is_ascii_uppercase() {
            spelled.push('_');
            spelled.push(c.to_ascii_lowercase());
        } else {
            spelled.push(c);
        }
    }
    spelled
}

/// Whether `c` can be part of an identifier.
fn ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The byte each whole identifier `name` in `text` starts at.
fn identifiers<'a>(text: &'a str, name: &'a str) -> impl Iterator<Item = usize> + 'a {
    text.match_indices(name)
        .map(|(at, _)| at)
        .filter(move |&at| {
            let before = text[..at].chars().next_back();
            let after = text[at + name.len()..].chars().next();
            before.is_none_or(|c| !ident(c)) && after.is_none_or(|c| !ident(c))
        })
}

/// The line the byte `at` of `text` is on, from 1.
fn line_of(text: &str, at: usize) -> usize {
    text[..at].matches('\n').count() + 1
}

/// The function that holds the byte `at` of `structure`: `Type::function` inside an `impl` of
/// `Type`, else `function`.
fn enclosing(structure: &str, at: usize) -> String {
    let Some((start, _)) = innermost(structure, "fn", at) else {
        return String::from("no function");
    };
    let function: String = structure[start + 2..]
        .trim_start()
        .chars()
        .take_while(|&c| ident(c))
        .collect();
    match innermost(structure, "impl", start) {
        Some((header, open)) => format!("{}::{function}", self_type(&structure[header + 4..open])),
        None => function,
    }
}

/// The innermost `fn` or `impl` item whose body holds the byte `at`: where its keyword starts, and
/// its body's opening brace. An `impl` in a type (`-> impl Future`) is no item.
fn innermost(structure: &str, keyword: &str, at: usize) -> Option<(usize, usize)> {
    identifiers(structure, keyword)
        .filter(|&start| start < at)
        .filter(|&start| {
            let after = structure[start + keyword.len()..].trim_start();
            let before = structure[..start].trim_end().chars().next_back();
            match keyword {
                "fn" => after.starts_with(|c: char| c.is_alphabetic() || c == '_'),
                _ => before.is_none_or(|c| matches!(c, '}' | ';' | '{' | ']')),
            }
        })
        .filter_map(|start| body(structure, start).map(|(open, close)| (start, open, close)))
        .filter(|&(_, open, close)| open < at && at < close)
        .map(|(start, open, _)| (start, open))
        .max()
}

/// The body of the item whose keyword starts at `start`: its opening brace and the byte past its
/// closing one, or none when a `;` outside brackets ends the item first.
fn body(structure: &str, start: usize) -> Option<(usize, usize)> {
    let mut depth = 0_i64;
    for (at, byte) in structure.bytes().enumerate().skip(start) {
        match byte {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b';' if depth == 0 => return None,
            b'{' if depth == 0 => return Some((at, closing(structure, at))),
            _ => {}
        }
    }
    None
}

/// The byte past the brace that closes the one at `open`.
fn closing(structure: &str, open: usize) -> usize {
    let mut depth = 0_i64;
    for (at, byte) in structure.bytes().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return at + 1;
                }
            }
            _ => {}
        }
    }
    structure.len()
}

/// The type an `impl` header names: `Type` from `<T> Trait for Type<T>` or from `Type`.
fn self_type(header: &str) -> String {
    let header = header.trim_start();
    let header = if header.starts_with('<') {
        &header[closing_angle(header)..]
    } else {
        header
    };
    let named = header.rsplit(" for ").next().unwrap_or(header).trim_start();
    named.chars().take_while(|&c| ident(c)).collect()
}

/// The byte past the `>` that closes the `<` `text` starts with; an arrow's `>` closes nothing.
fn closing_angle(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 0_i64;
    for (at, &byte) in bytes.iter().enumerate() {
        match byte {
            b'<' => depth += 1,
            b'>' if at == 0 || bytes[at - 1] != b'-' => {
                depth -= 1;
                if depth == 0 {
                    return at + 1;
                }
            }
            _ => {}
        }
    }
    text.len()
}

/// A Rust source's code, its comments and `#[cfg(test)]` modules blanked, and its structure, its
/// string and character literals blanked too. Both keep the text's bytes and lines in place. Only a
/// module is left out: any other item compiled for tests alone, a field among them, is read.
fn rust_code(text: &str) -> (String, String) {
    let (mut code, mut structure) = lex(text);
    let mut from = 0;
    while let Some(found) = structure[from..].find(CFG_TEST) {
        let start = from + found;
        let after = start + CFG_TEST.len();
        if !a_module_follows(&structure[after..]) {
            from = after;
            continue;
        }
        let end = body(&structure, after).map_or_else(
            || {
                structure[after..]
                    .find(';')
                    .map_or(structure.len(), |at| after + at + 1)
            },
            |(_, close)| close,
        );
        code = blank(&code, start, end);
        structure = blank(&structure, start, end);
        from = end;
    }
    (code, structure)
}

/// Whether the item `rest` starts with is a module, any further attributes on it passed over.
fn a_module_follows(rest: &str) -> bool {
    let mut rest = rest.trim_start();
    while rest.starts_with("#[") {
        rest = rest[attribute_end(rest)..].trim_start();
    }
    rest.strip_prefix("mod")
        .is_some_and(|name| name.starts_with(char::is_whitespace))
}

/// The byte past the `]` that closes the attribute `text` starts with.
fn attribute_end(text: &str) -> usize {
    let mut depth = 0_i64;
    for (at, byte) in text.bytes().enumerate() {
        match byte {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return at + 1;
                }
            }
            _ => {}
        }
    }
    text.len()
}

/// What a stretch of a Rust source is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Token {
    /// Code, one character.
    Code,
    /// A line or block comment.
    Comment,
    /// A string or character literal.
    Literal,
}

/// A Rust source's code, its comments blanked, and its structure, its literals blanked too.
fn lex(text: &str) -> (String, String) {
    let chars: Vec<char> = text.chars().collect();
    let mut code = String::with_capacity(text.len());
    let mut structure = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        let (token, end) = token(&chars, at);
        for &c in &chars[at..end] {
            keep(&mut code, c, token == Token::Comment);
            keep(&mut structure, c, token != Token::Code);
        }
        at = end;
    }
    (code, structure)
}

/// The token that starts at `at`, and the character past its end.
fn token(chars: &[char], at: usize) -> (Token, usize) {
    let c = chars[at];
    let next = chars.get(at + 1).copied();
    if c == '/' && next == Some('/') {
        let end = chars[at..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(chars.len(), |length| at + length);
        return (Token::Comment, end);
    }
    if c == '/' && next == Some('*') {
        return (Token::Comment, block_comment_end(chars, at));
    }
    let after_ident = at > 0 && ident(chars[at - 1]);
    if !after_ident && let Some(end) = raw_string_end(chars, at) {
        return (Token::Literal, end);
    }
    if c == '"' || (c == '\'' && (next == Some('\\') || chars.get(at + 2) == Some(&'\''))) {
        return (Token::Literal, quoted_end(chars, at));
    }
    (Token::Code, at + 1)
}

/// The character past the block comment that opens at `at`, nested comments included.
fn block_comment_end(chars: &[char], at: usize) -> usize {
    let mut depth = 0_usize;
    let mut end = at;
    while end < chars.len() {
        if chars[end] == '/' && chars.get(end + 1) == Some(&'*') {
            depth += 1;
            end += 2;
        } else if chars[end] == '*' && chars.get(end + 1) == Some(&'/') {
            depth = depth.saturating_sub(1);
            end += 2;
            if depth == 0 {
                return end;
            }
        } else {
            end += 1;
        }
    }
    chars.len()
}

/// The character past the raw string (`r"..."`, `r#"..."#`, or either after `b`) that starts at
/// `at`, if one starts there.
fn raw_string_end(chars: &[char], at: usize) -> Option<usize> {
    let mut cursor = at + usize::from(chars[at] == 'b');
    if chars.get(cursor) != Some(&'r') {
        return None;
    }
    cursor += 1;
    let mut hashes = 0;
    while chars.get(cursor) == Some(&'#') {
        hashes += 1;
        cursor += 1;
    }
    if chars.get(cursor) != Some(&'"') {
        return None;
    }
    let closes = |end: usize| (1..=hashes).all(|offset| chars.get(end + offset) == Some(&'#'));
    Some(
        chars
            .iter()
            .enumerate()
            .skip(cursor + 1)
            .find(|&(end, &c)| c == '"' && closes(end))
            .map_or(chars.len(), |(end, _)| end + hashes + 1),
    )
}

/// The character past the literal whose quote is at `at`; a backslash escapes the next character.
fn quoted_end(chars: &[char], at: usize) -> usize {
    let quote = chars[at];
    let mut end = at + 1;
    while end < chars.len() && chars[end] != quote {
        end += if chars[end] == '\\' { 2 } else { 1 };
    }
    (end + 1).min(chars.len())
}

/// Pushes `c`, or as many spaces as its bytes when `blanked`; a line break is always kept.
fn keep(out: &mut String, c: char, blanked: bool) {
    if blanked && c != '\n' {
        out.extend(std::iter::repeat_n(' ', c.len_utf8()));
    } else {
        out.push(c);
    }
}

/// `text` with every character from the byte `start` to the byte `end` blanked.
fn blank(text: &str, start: usize, end: usize) -> String {
    let mut out = String::with_capacity(text.len());
    for (at, c) in text.char_indices() {
        keep(&mut out, c, (start..end).contains(&at));
    }
    out
}

/// Every shipped source under `root`, as its path from the root and its text, in path order: a
/// file of a shipped kind by its name, a script by its `#!` first line, and a unit's drop-in. A
/// directory under a `src/` is always entered, whatever its name; elsewhere the skipped and the
/// test directories are left out.
fn shipped_sources(root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let under_sources = relative(root, &directory).iter().any(|part| part == "src");
        for entry in fs::read_dir(&directory).expect("a readable directory") {
            let entry = entry.expect("a directory entry");
            let kind = entry.file_type().expect("a file type");
            let name = entry.file_name().to_string_lossy().into_owned();
            if kind.is_dir() {
                let left_out = !under_sources
                    && (SKIPPED_DIRECTORIES.contains(&name.as_str())
                        || TEST_DIRECTORIES.contains(&name.as_str()));
                if !left_out {
                    pending.push(entry.path());
                }
            } else if kind.is_file()
                && (shipped(&name)
                    || a_unit_drop_in(&directory, &name)
                    || starts_with_a_shebang(&entry.path()))
            {
                let path = entry.path();
                let text = fs::read_to_string(&path).expect("a shipped source is UTF-8");
                found.push((relative(root, &path).join("/"), text));
            }
        }
    }
    found.sort();
    found
}

/// The parts of `path` below `root`, each as text.
fn relative(root: &Path, path: &Path) -> Vec<String> {
    path.strip_prefix(root)
        .expect("a path under the root")
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect()
}

/// Whether a file named `name` is a shipped source: one of the shipped extensions, and no test.
fn shipped(name: &str) -> bool {
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    let python_test = extension == "py"
        && (stem.starts_with("test_") || stem.ends_with("_test") || stem == "conftest");
    let script_test = Path::new(stem)
        .extension()
        .is_some_and(|inner| inner == "test" || inner == "spec");
    SHIPPED_EXTENSIONS.contains(&extension) && !python_test && !script_test
}

/// Whether a file named `name` in `directory` is a systemd drop-in: a `.conf` in a `.d` directory.
fn a_unit_drop_in(directory: &Path, name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension == "conf")
        && directory
            .extension()
            .is_some_and(|extension| extension == "d")
}

/// Whether the file at `path` starts with `#!`, as a script run by its interpreter does.
fn starts_with_a_shebang(path: &Path) -> bool {
    let mut first = [0_u8; 2];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut first))
        .is_ok()
        && first == *b"#!"
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[test]
fn a_delivery_call_outside_the_router_does_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/push_outside_the_router.rs");
    cases.compile_fail("tests/ui/pass_by_default.rs");
    cases.compile_fail("tests/ui/pass_kept_by_a_clone.rs");
}

#[test]
fn no_delivery_goes_around_the_port() {
    // The first review's deliveries around the port are refused, each where it was planted, and
    // nothing a comment or a test module says is.
    let planted = census(&AROUND_THE_PORT.map(|(path, text)| (path.to_owned(), text.to_owned())));
    assert_eq!(
        planted.refused(),
        [
            "crates/api/src/notifications_routes.rs:3: names api.telegram.org",
            "crates/api/src/notifications_routes.rs:3: names sendMessage",
            "crates/api/src/router.rs:6: names FEED_TABLE",
            "crates/bot/src/celebrate.rs:5: names sendMessage in celebrate, not a named call site",
            "crates/coordination/src/sync_cycle.rs:6: names QUEUE_TABLE",
            "crates/daemon/src/lifecycle.rs:4: calls edit_html in celebrate_by_an_edit, \
             not a named call site",
            "crates/daemon/src/main.rs:6: calls handle in celebrate_by_a_fabricated_command, \
             not a named call site",
            "crates/daemon/src/role_bot.rs:4: calls send_html in celebrate_around_the_router, \
             not a named call site",
            "crates/daemon/src/role_bot.rs:10: names api.telegram.org",
            "crates/daemon/src/role_bot.rs:10: names sendMessage",
            "crates/daemon/src/role_data.rs:3: uses send_html in celebrate_through_a_variable, \
             not a named call site",
            "crates/daemon/src/role_job.rs:11: calls send_html in celebrate_around_the_router, \
             not a named call site",
            "crates/daemon/src/wiring.rs:4: names DEFAULT_API_URL",
            "crates/notifications/src/lib.rs:6: names ledger",
            "crates/notifications/src/occasion.rs:3: names append_feed",
            "crates/notifications/src/occasion.rs:3: names ledger",
            "crates/notifications/src/quiet.rs:1: names ledger",
            "crates/notifications/src/quiet.rs:7: names ledger",
            "deploy/scripts/celebrate.py:9: names in_app_feed",
            "deploy/scripts/hold.py:9: names notification_queue",
        ],
        "the bot's own send, named or called, its edit and its command handler, raw requests to \
         the Bot API and on its base URL, a raw request from inside the bot, and writes to the \
         Mini App's feed and to the held queue, around the port"
    );

    // The walker reads a shipped module in a directory named as tests are, because it is under
    // `src/`, a script by its `#!` first line and a unit's drop-in, and leaves out a test directory
    // outside `src/`.
    let tree = tempfile::tempdir().expect("a temporary tree");
    for (path, text) in WALKED {
        let file = tree.path().join(path);
        fs::create_dir_all(file.parent().expect("a planted file's directory"))
            .expect("a planted directory");
        fs::write(&file, text).expect("a planted source");
    }
    let walked = shipped_sources(tree.path());
    let paths: Vec<&str> = walked.iter().map(|(path, _)| path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "crates/daemon/src/fixtures/celebrate.rs",
            "deploy/scripts/celebrate",
            "deploy/systemd/deck-streak-bot.service.d/celebrate.conf",
        ],
        "the walker reads every directory under src/, a script by its first line and a drop-in, \
         and no test directory outside src/"
    );
    assert_eq!(
        census(&walked).refused(),
        [
            "crates/daemon/src/fixtures/celebrate.rs:6: calls send_html in \
             celebrate_around_the_router, not a named call site",
            "deploy/scripts/celebrate:4: names api.telegram.org",
            "deploy/scripts/celebrate:4: names sendMessage",
            "deploy/systemd/deck-streak-bot.service.d/celebrate.conf:2: names api.telegram.org",
            "deploy/systemd/deck-streak-bot.service.d/celebrate.conf:2: names sendMessage",
        ],
        "a send around the port in a shipped module under src/, a script and a drop-in"
    );

    // Every shipped source of the tree: nothing goes around the port, and each named call site is
    // found where it is named, once.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the tree's root, two levels above the crate");
    let sources = examined("shipped source(s)", shipped_sources(root));
    let tree = census(&sources);
    assert_eq!(
        tree.refused(),
        Vec::<String>::new(),
        "a delivery goes around the port"
    );
    let mut named: Vec<(String, String, String)> = NAMED_SENDS
        .iter()
        .map(|&(path, function, send)| (path.to_owned(), function.to_owned(), send.to_owned()))
        .collect();
    named.sort();
    assert_eq!(
        tree.sends, named,
        "every call of a send, at its named call site"
    );

    // The one exception is needed: the alert path's text is refused anywhere else.
    let (_, alert) = sources
        .iter()
        .find(|(path, _)| path == ALERT_PATH)
        .expect("the alert path is a shipped source");
    let elsewhere = census(&[(
        String::from("deploy/scripts/another-page.sh"),
        alert.clone(),
    )]);
    let what: Vec<&str> = elsewhere
        .refusals
        .iter()
        .map(|(_, _, what)| what.as_str())
        .collect();
    assert_eq!(what, ["names api.telegram.org", "names sendMessage"]);
}
