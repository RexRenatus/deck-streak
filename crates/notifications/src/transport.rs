//! The bot transport port (SPEC-041 R1, R13): the delivery calls of the bot surface. The bot
//! implements them on its transport, and the composition root joins that to the router.
//!
//! Each call takes the router's [`Pass`], which only the router module can make, so a call of the
//! port anywhere else does not compile: the policy's one-router rule, held by the type system
//! (SPEC-041 A2). A delivery that never calls the port, through the bot's own send, edit or
//! command replies, or a raw request or a client's call of one of the send or delivery methods of
//! the pinned client's table, is refused by a census (SPEC-041 A15) by a name it holds, in the
//! shipped sources of the kinds it reads; code written to evade it goes unread, and review
//! catches it (#297).
//!
//! The ladder's renders (SPEC-084 R8; ADR-084) join `push_message` as calls of their own: the
//! reveal, the dice, the reaction and the pin. Each has a default body that answers
//! [`Pushed::Unsupported`], never a silent delivery, so a transport that renders only lines stays a
//! transport, and the router records each unsupported call by name as a degraded render. The bot's
//! transport implements every one.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::photo::{FileId, Photo};
use crate::router::Pass;

/// A push's future: boxed, so the port stays a trait object the router can hold.
pub type PushFuture<'a> = Pin<Box<dyn Future<Output = Pushed> + Send + 'a>>;

/// What one push came to.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pushed {
    /// The message reached the owner's chat.
    Delivered,
    /// It did not: the Bot API refused it or could not be reached.
    Failed,
    /// The transport has no such call, so nothing was attempted: the router degrades the render.
    Unsupported,
}

/// The answer of a call a transport does not implement.
fn unsupported<'a>() -> PushFuture<'a> {
    Box::pin(std::future::ready(Pushed::Unsupported))
}

/// A photo push's future.
pub type PhotoFuture<'a> = Pin<Box<dyn Future<Output = PhotoPushed> + Send + 'a>>;

/// A prepared share's future.
pub type ShareFuture<'a> = Pin<Box<dyn Future<Output = Prepared> + Send + 'a>>;

/// What one photo push came to.
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhotoPushed {
    /// The photo reached the owner's chat; Telegram holds it by this file id.
    Delivered {
        /// The file id of the largest size the Bot API answered.
        file_id: FileId,
    },
    /// It did not: the Bot API refused it or could not be reached.
    Failed,
    /// The transport has no such call, so nothing was attempted.
    Unsupported,
}

/// What one prepared share came to.
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prepared {
    /// The message is prepared, under this id.
    Ready {
        /// The prepared message's id.
        id: String,
    },
    /// The Bot API refused it or could not be reached.
    Failed,
    /// The transport has no such call, so nothing was attempted.
    Unsupported,
}

/// The bot's delivery calls.
pub trait BotTransport: Send + Sync {
    /// Sends `text`, as the bot's HTML, to the owner's chat.
    fn push_message<'a>(&'a self, pass: &'a Pass, text: &'a str) -> PushFuture<'a>;

    /// The T3 reveal: sends `placeholder`, waits `pause`, then edits the placeholder into `text`,
    /// sending `text` anew when the placeholder or the edit fails. Delivered when `text` reached
    /// the owner's chat either way.
    fn push_reveal<'a>(
        &'a self,
        _pass: &'a Pass,
        _placeholder: &'a str,
        _text: &'a str,
        _pause: Duration,
    ) -> PushFuture<'a> {
        unsupported()
    }

    /// Sends a dice with `emoji`, the topper of a T4 and a T5. Its outcome is never a delivery's.
    fn push_dice<'a>(&'a self, _pass: &'a Pass, _emoji: &'a str) -> PushFuture<'a> {
        unsupported()
    }

    /// Reacts with `emoji` to the owner's message `message_id`: the T1 render.
    fn push_reaction<'a>(
        &'a self,
        _pass: &'a Pass,
        _message_id: i64,
        _emoji: &'a str,
    ) -> PushFuture<'a> {
        unsupported()
    }

    /// Sends `text` and pins it without a notification: the T5 render. A pin that fails leaves the
    /// message delivered; a send that fails is sent anew once as a line.
    fn push_pin<'a>(&'a self, _pass: &'a Pass, _text: &'a str) -> PushFuture<'a> {
        unsupported()
    }

    /// Sends `photo` with `caption` to the owner's chat.
    fn push_photo<'a>(
        &'a self,
        _pass: &'a Pass,
        _photo: &'a Photo,
        _caption: &'a str,
    ) -> PhotoFuture<'a> {
        Box::pin(std::future::ready(PhotoPushed::Failed))
    }

    /// Prepares the photo Telegram holds as `file`, with `caption`, for the owner to share.
    fn prepare_share<'a>(
        &'a self,
        _pass: &'a Pass,
        _file: &'a FileId,
        _caption: &'a str,
    ) -> ShareFuture<'a> {
        Box::pin(std::future::ready(Prepared::Failed))
    }
}
