//! The kernel's identifiers (SPEC-020 R7).
//!
//! An id is a newtype with no arithmetic and no conversion from a bare integer except its named
//! constructor, so an id of one kind can never be passed where another is meant.

/// A Telegram user's id: the owner's, the bot's, or a sender's.
///
/// It lives in the kernel because identity, the bot and the notification router all name it, and
/// none of them may depend on another (docs/CONTEXT-MAP.md, layer 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TelegramUserId(i64);

impl TelegramUserId {
    /// The user id Telegram gave as `raw`.
    #[must_use]
    pub const fn new(raw: i64) -> Self {
        Self(raw)
    }

    /// The id as Telegram writes it.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::TelegramUserId;

    #[test]
    fn a_user_id_keeps_the_value_it_was_named_with() {
        assert_eq!(TelegramUserId::new(424_242).get(), 424_242);
        assert!(TelegramUserId::new(7) < TelegramUserId::new(8));
    }
}
