//! The law drill commands' pure parts (SPEC-110 R13, R14): the callback tokens, the keyboard's
//! limit, the four drill codes and the replies. The handlers that send are `commands.rs`'s.
//!
//! A button's data is at most 64 bytes, and a drill's id can be longer, so a long id is carried
//! as a short hash and resolved against the drills the owner can answer at the moment of the tap.
//! The rule is the predecessor's `drill_callback_token` one.

use std::fmt::Write as _;

use deck_streak_coordination::drills::{AnswerOutcome, DrillMeta, DrillView, sha256};
use frankenstein::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::commands::Reply;
use crate::transport::escape_html;

/// The data prefix of a button that opens a drill's view.
pub const VIEW_PREFIX: &str = "dv:";

/// The data prefix of a button that asks for a drill's answer.
pub const ANSWER_PREFIX: &str = "da:";

/// The most bytes a button's data may carry (the Bot API's limit).
pub const MAX_CALLBACK_DATA: usize = 64;

/// The hex characters of a hashed token.
pub const TOKEN_HASH_LEN: usize = 20;

/// The most drill buttons `/drills` puts on its keyboard.
pub const KEYBOARD_LIMIT: usize = 12;

/// The four drill types and the code that picks each, in the order `/drill` names them.
pub const CODES: [(&str, &str); 4] = [
    ("c", "case-brief"),
    ("i", "irac"),
    ("o", "outline"),
    ("u", "rule-statement"),
];

/// The most characters of a drill's title a button shows.
const LABEL_LIMIT: usize = 40;

/// The most characters of a drill's prompt a view shows.
const PROMPT_LIMIT: usize = 3_000;

/// The token of the drill `id` under `prefix`: the whole data. The id itself when the data fits
/// the limit, and otherwise `#` and the first hex characters of the id's digest.
#[must_use]
pub fn encode_token(prefix: &str, id: &str) -> String {
    if prefix.len() + id.len() <= MAX_CALLBACK_DATA {
        return format!("{prefix}{id}");
    }
    let digest = sha256::hex(&sha256::digest(id.as_bytes()));
    format!("{prefix}#{}", &digest[..TOKEN_HASH_LEN])
}

/// The drill of `offered` that `data` names under `prefix`: only when it names exactly one.
#[must_use]
pub fn resolve_token(prefix: &str, data: &str, offered: &[String]) -> Option<String> {
    let body = data.strip_prefix(prefix)?;
    let mut named = offered
        .iter()
        .filter(|id| encode_token(prefix, id).strip_prefix(prefix) == Some(body));
    let first = named.next()?;
    if named.next().is_some() {
        return None;
    }
    Some(first.clone())
}

/// The drill type a code picks.
#[must_use]
pub fn kind_of(code: &str) -> Option<&'static str> {
    CODES
        .iter()
        .find(|(known, _)| *known == code)
        .map(|(_, kind)| *kind)
}

fn button(text: &str, data: String) -> InlineKeyboardButton {
    InlineKeyboardButton::builder()
        .text(text)
        .callback_data(data)
        .build()
}

fn label(meta: &DrillMeta) -> String {
    meta.title.chars().take(LABEL_LIMIT).collect()
}

/// The unanswered drills as a keyboard: one button each, at most [`KEYBOARD_LIMIT`], and the count
/// left out said in the text.
#[must_use]
pub fn list_reply(heading: &str, drills: &[DrillMeta]) -> Reply {
    if drills.is_empty() {
        return Reply::text(format!("{heading}\nNo drill waits for an answer."));
    }
    let shown = drills.iter().take(KEYBOARD_LIMIT);
    let rows: Vec<Vec<InlineKeyboardButton>> = shown
        .map(|meta| {
            vec![button(
                &label(meta),
                encode_token(VIEW_PREFIX, &meta.drill_id),
            )]
        })
        .collect();
    let mut text = format!("<b>{}</b>", escape_html(heading));
    if drills.len() > KEYBOARD_LIMIT {
        let _ = write!(text, "\n\u{2026}and {} more", drills.len() - KEYBOARD_LIMIT);
    }
    Reply {
        text,
        keyboard: Some(
            InlineKeyboardMarkup::builder()
                .inline_keyboard(rows)
                .build(),
        ),
    }
}

/// `/drill` alone: the four types and the code that picks each.
#[must_use]
pub fn types_reply() -> Reply {
    let mut lines = vec!["Send /drill and a code to see one type's drills:".to_owned()];
    lines.extend(
        CODES
            .iter()
            .map(|(code, kind)| format!("/drill {code} {kind}")),
    );
    Reply::text(lines.join("\n"))
}

/// `/drill` with a code that picks no type.
#[must_use]
pub fn refusal_reply() -> Reply {
    let codes: Vec<&str> = CODES.iter().map(|(code, _)| *code).collect();
    Reply::text(format!(
        "That is not a drill type. The codes are {}.",
        codes.join(", ")
    ))
}

/// One drill's view: its prompt, and the button that asks for the answer when it has none.
#[must_use]
pub fn view_reply(view: &DrillView) -> Reply {
    let prompt: String = view.prompt.chars().take(PROMPT_LIMIT).collect();
    let mut text = format!(
        "<b>{}</b>\n{}",
        escape_html(&view.meta.title),
        escape_html(&prompt)
    );
    if !view.defer_reason.is_empty() {
        let _ = write!(text, "\nDeferred: {}", escape_html(&view.defer_reason));
    }
    let keyboard = (!view.meta.answered).then(|| {
        InlineKeyboardMarkup::builder()
            .inline_keyboard(vec![vec![button(
                "Answer",
                encode_token(ANSWER_PREFIX, &view.meta.drill_id),
            )]])
            .build()
    });
    Reply { text, keyboard }
}

/// The ask: the owner's next message is the answer.
#[must_use]
pub fn ask_reply(title: &str) -> Reply {
    Reply::text(format!(
        "Send your answer to <b>{}</b> as your next message. Any command cancels it.",
        escape_html(title)
    ))
}

/// What the answer became.
#[must_use]
pub fn outcome_reply(outcome: &AnswerOutcome) -> Reply {
    match outcome {
        AnswerOutcome::Appended { title } => Reply::text(format!(
            "Your answer to <b>{}</b> is recorded.",
            escape_html(title)
        )),
        AnswerOutcome::EmptyAnswer => {
            Reply::text("That answer was empty, so nothing was recorded.".to_owned())
        }
        AnswerOutcome::NotActive => gone_reply(),
        AnswerOutcome::AlreadyAnswered => {
            Reply::text("That drill has an answer already.".to_owned())
        }
        AnswerOutcome::RailRefused => Reply::text("That answer could not be recorded.".to_owned()),
    }
}

/// The drill a tap named is not among those that can be answered now.
#[must_use]
pub fn gone_reply() -> Reply {
    Reply::text(
        "That drill is not waiting for an answer any more. Send /drills for the list.".to_owned(),
    )
}

/// The vault is not wired, or could not be read.
#[must_use]
pub fn unavailable_reply() -> Reply {
    Reply::text("The drills could not be read. Try again later.".to_owned())
}
