//! The untrusted fence (SPEC-043 R9): text the learner controls is data, never instruction.
//!
//! An untrusted input stands alone between `<untrusted source="...">` and `</untrusted>` on lines
//! of their own, JSON-encoded, with `<` and `>` written as the JSON escapes `<` and `>`,
//! so no text inside can close the fence or open another.

/// Where an untrusted input comes from. The set is the one this SPEC's tasks read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The learner's cards.
    Cards,
    /// The learner's memory.
    Memory,
}

impl Source {
    /// The source's name in the fence and in `ai-safety.json`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cards => "cards",
            Self::Memory => "memory",
        }
    }
}

/// `text` as a JSON string with `<` and `>` escaped: one line, and no angle bracket.
#[must_use]
pub fn encode(text: &str) -> String {
    let json = serde_json::to_string(text).unwrap_or_else(|_| String::from("\"\""));
    json.replace('<', "\\u003c").replace('>', "\\u003e")
}

/// One untrusted input, fenced alone.
#[must_use]
pub fn fence(source: Source, text: &str) -> String {
    format!(
        "<untrusted source=\"{}\">\n{}\n</untrusted>",
        source.as_str(),
        encode(text)
    )
}
