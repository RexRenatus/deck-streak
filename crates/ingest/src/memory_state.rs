//! A card's memory state: what the scheduler stored in `cards.data` about how durable a memory is.
//!
//! The predecessor's `parse_fsrs` is the oracle (SPEC-077 R1): the parse is total, so a card the
//! scheduler never gave a state to, or gave a state we cannot read, is simply a card with none.

/// The decay a card gets when its stored one is missing or not positive.
pub const DEFAULT_DECAY: f64 = 0.2;

/// How durable one card's memory is, as the scheduler stored it.
///
/// Equality compares each float's bits, so it is an equivalence relation and `Card` keeps `Eq`.
#[derive(Clone, Copy, Debug)]
pub struct MemoryState {
    /// Days for recall to fall to the target retention: finite and positive.
    pub stability: f64,
    /// How hard the card is; 0.0 when the scheduler stored none.
    pub difficulty: f64,
    /// The forgetting curve's decay: positive, `DEFAULT_DECAY` when none was stored.
    pub decay: f64,
    /// The retention the card was scheduled for, when stored.
    pub desired_retention: Option<f64>,
    /// The last review's time in epoch seconds, when stored.
    pub last_review_sec: Option<i64>,
}

impl PartialEq for MemoryState {
    fn eq(&self, other: &Self) -> bool {
        self.stability.to_bits() == other.stability.to_bits()
            && self.difficulty.to_bits() == other.difficulty.to_bits()
            && self.decay.to_bits() == other.decay.to_bits()
            && self.desired_retention.map(f64::to_bits) == other.desired_retention.map(f64::to_bits)
            && self.last_review_sec == other.last_review_sec
    }
}

impl Eq for MemoryState {}

/// Reads `text` the way the predecessor's JSON reader does: the bare words `NaN`, `Infinity` and
/// `-Infinity` and a number too large for a float read as values that no field accepts, so the
/// field is dropped and the rest of the object survives, rather than the whole text failing.
fn leniently(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    let mut in_string = false;
    while at < bytes.len() {
        let byte = bytes[at];
        if in_string {
            let first = text[at..].chars().next().map_or(1, char::len_utf8);
            let escaped = if byte == b'\\' {
                text[at + first..].chars().next().map_or(0, char::len_utf8)
            } else {
                0
            };
            out.push_str(&text[at..at + first + escaped]);
            in_string = byte != b'"';
            at += first + escaped;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            out.push('"');
            at += 1;
        } else if let Some(word) = ["-Infinity", "Infinity", "NaN"]
            .iter()
            .find(|word| text[at..].starts_with(**word))
        {
            out.push_str("null");
            at += word.len();
        } else if byte == b'-' || byte.is_ascii_digit() {
            let run = text[at..]
                .find(|c: char| !matches!(c, '0'..='9' | '-' | '+' | '.' | 'e' | 'E'))
                .map_or(text.len(), |n| at + n);
            let token = &text[at..run];
            if token.parse::<f64>().is_ok_and(f64::is_infinite) {
                out.push_str("null");
            } else {
                out.push_str(token);
            }
            at = run;
        } else {
            let width = text[at..].chars().next().map_or(1, char::len_utf8);
            out.push_str(&text[at..at + width]);
            at += width;
        }
    }
    out
}

/// A finite number under `key`; a JSON boolean counts as 1 or 0, as it does in the predecessor.
fn number(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<f64> {
    match object.get(key)? {
        serde_json::Value::Number(n) => n.as_f64().filter(|v| v.is_finite()),
        serde_json::Value::Bool(flag) => Some(f64::from(u8::from(*flag))),
        _ => None,
    }
}

/// The memory state in a card's `data` text, or none.
///
/// None for missing, empty, unparsable or non-object text, and for a stability that is missing,
/// not finite or not positive. A decay that is missing or not positive is [`DEFAULT_DECAY`].
#[must_use]
pub fn parse(data: Option<&str>) -> Option<MemoryState> {
    let text = data.filter(|text| !text.is_empty())?;
    let value: serde_json::Value = serde_json::from_str(&leniently(text)).ok()?;
    let object = value.as_object()?;
    let stability = number(object, "s").filter(|s| *s > 0.0)?;
    let decay = number(object, "decay")
        .filter(|d| *d > 0.0)
        .unwrap_or(DEFAULT_DECAY);
    Some(MemoryState {
        stability,
        difficulty: number(object, "d").unwrap_or(0.0),
        decay,
        desired_retention: number(object, "dr"),
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the predecessor's int() of a float"
        )]
        last_review_sec: number(object, "lrt").map(|seconds| seconds.trunc() as i64),
    })
}
