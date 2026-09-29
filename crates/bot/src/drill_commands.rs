//! The law drill commands' pure parts (SPEC-110 R13, R14): the callback tokens, the keyboard's
//! limit and the four drill codes. The handlers that send are `commands.rs`'s.

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

/// The token of the drill `id` under `prefix`: the whole data.
#[must_use]
pub fn encode_token(prefix: &str, id: &str) -> String {
    let _ = (prefix, id);
    String::new()
}

/// The drill of `offered` that `data` names under `prefix`: only when it names exactly one.
#[must_use]
pub fn resolve_token(prefix: &str, data: &str, offered: &[String]) -> Option<String> {
    let _ = (prefix, data, offered);
    None
}
