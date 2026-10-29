//! The stub of Rust-side matching: nothing matches.

/// The stub's `LIKE`: never a match.
#[must_use]
pub fn like(pattern: &str, text: &str) -> bool {
    let _ = (pattern, text);
    false
}

/// The stub's `LIKE` pattern: empty.
#[must_use]
pub fn to_like(text: &str) -> String {
    let _ = text;
    String::new()
}

/// The stub's glob.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glob;

impl Glob {
    /// Compiles nothing.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let _ = text;
        Self
    }

    /// Never wild.
    #[must_use]
    pub const fn is_wild(&self) -> bool {
        false
    }

    /// Never a match.
    #[must_use]
    pub fn matches(&self, text: &str) -> bool {
        let _ = text;
        false
    }

    /// Never a match.
    #[must_use]
    pub fn matches_up_to(&self, text: &str, separator: &str) -> bool {
        let _ = (text, separator);
        false
    }
}

/// The stub's sort field: empty.
#[must_use]
pub fn strip_html(field: &str) -> String {
    let _ = field;
    String::new()
}
