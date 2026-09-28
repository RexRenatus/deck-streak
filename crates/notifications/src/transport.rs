//! The bot transport port (SPEC-041 R1, R13): the delivery calls of the bot surface. The bot
//! implements them on its transport, and the composition root joins that to the router.
//!
//! Each call takes the router's [`Pass`], which only the router module can make, so a call of the
//! port anywhere else does not compile: the policy's one-router rule, held by the type system
//! (SPEC-041 A2). A delivery that never calls the port, through the bot's own send or a raw request
//! to the Bot API, is refused by a census of the shipped sources of the kinds it reads (SPEC-041
//! A15). The port carries `push_message` alone, because the router renders nothing beyond a line;
//! the ladder's dice, reaction and pin calls join it with the ladder's renders (#120).

use std::future::Future;
use std::pin::Pin;

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
}

/// The bot's delivery calls.
pub trait BotTransport: Send + Sync {
    /// Sends `text`, as the bot's HTML, to the owner's chat.
    fn push_message<'a>(&'a self, pass: &'a Pass, text: &'a str) -> PushFuture<'a>;
}
