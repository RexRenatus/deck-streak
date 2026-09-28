//! The owner gate and the inbound caps (SPEC-026 R4, R5; CHARTER 14; ADR-006).
//!
//! The service answers one owner. A message is dispatched only when it comes from the owner, in
//! the owner's private chat; a callback only when it comes from the owner. Anything else is
//! dropped: no reply, no callback answer, and a log line that names the update's kind and one
//! reason code, never its content. The caps are the predecessor's (`bot.py:_MAX_INBOUND_TEXT`,
//! `_MAX_CALLBACK_DATA`, proved by `goldens/bot.constants.json`): the owner's text over the cap is
//! dropped, and the owner's callback whose data is over its cap is answered, so the client's
//! progress indicator stops, and not dispatched.
//!
//! The owner is identity's: the Telegram user id the credential `owner-user-id` names (ADR-006).
//! In a private chat the chat's id is the user's, so the owner's private chat is the chat whose id
//! is the owner's and whose type is private.

use deck_streak_identity::Owner;
use deck_streak_kernel::TelegramUserId;
use frankenstein::types::{CallbackQuery, ChatType, MaybeInaccessibleMessage, Message, User};
use frankenstein::updates::UpdateContent;

/// The longest text the owner's message may carry, in characters: longer is dropped.
pub const MAX_INBOUND_TEXT: usize = 4096;
/// The most bytes a callback's data may carry: more is answered and not dispatched.
pub const MAX_CALLBACK_DATA_BYTES: usize = 64;

/// The owner's text message, from the owner's private chat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerMessage {
    /// The text as sent.
    pub text: String,
}

/// The owner's callback, from a button the bot sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerCallback {
    /// The callback query's id, which its answer names.
    pub id: String,
    /// The button's data, when it carried any.
    pub data: Option<String>,
    /// The id of the message the button was on, when Telegram says.
    pub message_id: Option<i32>,
}

/// Why an update is not dispatched: its kind and one reason code, all the log may say of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dropped {
    /// The update's kind: `message`, `callback_query` or `update`.
    pub kind: &'static str,
    /// The one reason code.
    pub reason: &'static str,
}

/// What the gate decided for one update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admission {
    /// The owner's message: dispatch it.
    Message(OwnerMessage),
    /// The owner's callback: answer it, then dispatch it.
    Callback(OwnerCallback),
    /// The owner's callback whose data is over its cap: answer it, and dispatch nothing.
    AnswerOnly {
        /// The callback query's id.
        callback_id: String,
        /// Why it is not dispatched.
        dropped: Dropped,
    },
    /// Anything else: no reply and no answer.
    Dropped(Dropped),
}

/// What the gate decides for `content`, an update's one kind, for `owner`.
#[must_use]
pub fn admit(content: &UpdateContent, owner: Owner) -> Admission {
    match content {
        UpdateContent::Message(message) => admit_message(message, owner),
        UpdateContent::CallbackQuery(callback) => admit_callback(callback, owner),
        _ => Admission::Dropped(Dropped {
            kind: "update",
            reason: "kind_not_handled",
        }),
    }
}

fn dropped(kind: &'static str, reason: &'static str) -> Admission {
    Admission::Dropped(Dropped { kind, reason })
}

fn admit_message(message: &Message, owner: Owner) -> Admission {
    let from_owner = message
        .from
        .as_deref()
        .is_some_and(|user| is_owner(user, owner));
    if !from_owner {
        return dropped("message", "not_owner");
    }
    let private_chat = message.chat.type_field == ChatType::Private;
    let owners_chat = message.chat.id == owner.user().get();
    if !(private_chat && owners_chat) {
        return dropped("message", "not_owners_private_chat");
    }
    let Some(text) = &message.text else {
        return dropped("message", "no_text");
    };
    if text.chars().count() > MAX_INBOUND_TEXT {
        return dropped("message", "text_over_cap");
    }
    Admission::Message(OwnerMessage { text: text.clone() })
}

fn admit_callback(callback: &CallbackQuery, owner: Owner) -> Admission {
    if !is_owner(&callback.from, owner) {
        return dropped("callback_query", "not_owner");
    }
    let data = callback.data.clone();
    if data
        .as_ref()
        .is_some_and(|data| data.len() > MAX_CALLBACK_DATA_BYTES)
    {
        return Admission::AnswerOnly {
            callback_id: callback.id.clone(),
            dropped: Dropped {
                kind: "callback_query",
                reason: "data_over_cap",
            },
        };
    }
    let message_id = callback.message.as_ref().map(|message| match message {
        MaybeInaccessibleMessage::Message(message) => message.message_id,
        MaybeInaccessibleMessage::InaccessibleMessage(message) => message.message_id,
    });
    Admission::Callback(OwnerCallback {
        id: callback.id.clone(),
        data,
        message_id,
    })
}

/// Whether `user` is the owner.
fn is_owner(user: &User, owner: Owner) -> bool {
    i64::try_from(user.id).is_ok_and(|id| owner.is(TelegramUserId::new(id)))
}
