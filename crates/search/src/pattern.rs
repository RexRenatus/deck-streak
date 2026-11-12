//! Rust-side name and text matching (SPEC-514 R7): the engine's `LIKE` and its globs, computed
//! here over the row values, never in SQL.
//!
//! - [`like`] is `SQLite`'s `LIKE` with `\` as its escape: `%` any run, `_` one character, and
//!   only ASCII letters folded, which is what the engine's text and field searches compile to.
//! - [`Glob`] is the engine's wildcard text (`*` any run, `_` one character, `\*`, `\_` and `\\`
//!   literal), matched whole and with Unicode case folding, as the engine's `(?i)` expressions
//!   match deck, note type, template, field and tag names.

/// `pattern LIKE text ESCAPE '\'`.
#[must_use]
pub fn like(pattern: &str, text: &str) -> bool {
    let tokens = like_tokens(pattern);
    let text: Vec<char> = text.chars().collect();
    wildcard(&tokens, &text, |want, got| want.eq_ignore_ascii_case(&got))
}

/// The `LIKE` pattern of a search text (the engine's `to_sql`): `*` is `%`, a literal `%` is
/// escaped, `\*` is a literal `*`, and `_`, `\_` and `\\` pass to `LIKE` as they are.
#[must_use]
pub fn to_like(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.peek().copied() {
                Some('\\') => {
                    chars.next();
                    out.push_str("\\\\");
                }
                Some('*') => {
                    chars.next();
                    out.push('*');
                }
                _ => out.push('\\'),
            },
            '*' => out.push('%'),
            '%' => out.push_str("\\%"),
            other => out.push(other),
        }
    }
    out
}

/// One token of a compiled pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
    /// Exactly this character.
    Literal(char),
    /// Any one character.
    One,
    /// Any run of characters, the empty run included.
    Many,
}

fn like_tokens(pattern: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        tokens.push(match c {
            '%' => Token::Many,
            '_' => Token::One,
            '\\' => Token::Literal(chars.next().unwrap_or('\\')),
            other => Token::Literal(other),
        });
    }
    tokens
}

/// Whether `tokens` match all of `text`, `same` comparing a literal with a character.
fn wildcard(tokens: &[Token], text: &[char], same: impl Fn(char, char) -> bool) -> bool {
    let (mut t, mut s) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while s < text.len() {
        match tokens.get(t) {
            Some(Token::Many) => {
                star = Some((t, s));
                t += 1;
            }
            Some(Token::One) => {
                t += 1;
                s += 1;
            }
            Some(Token::Literal(want)) if same(*want, text[s]) => {
                t += 1;
                s += 1;
            }
            _ => match star {
                Some((star_t, star_s)) => {
                    t = star_t + 1;
                    s = star_s + 1;
                    star = Some((star_t, star_s + 1));
                }
                None => return false,
            },
        }
    }
    tokens[t.min(tokens.len())..]
        .iter()
        .all(|token| *token == Token::Many)
}

/// A character folded to its one-character lower case, or kept when it has none.
fn fold(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(one), None) => one,
        _ => c,
    }
}

/// The engine's wildcard text, compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glob {
    tokens: Vec<Token>,
    wild: bool,
}

impl Glob {
    /// Compiles `text`.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let mut tokens = Vec::new();
        let mut wild = false;
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            tokens.push(match c {
                '*' => {
                    wild = true;
                    Token::Many
                }
                '_' => {
                    wild = true;
                    Token::One
                }
                '\\' => Token::Literal(chars.next().unwrap_or('\\')),
                other => Token::Literal(fold(other)),
            });
        }
        Self { tokens, wild }
    }

    /// Whether the text holds an unescaped wildcard.
    #[must_use]
    pub const fn is_wild(&self) -> bool {
        self.wild
    }

    /// Whether the glob matches all of `text`, case folded.
    #[must_use]
    pub fn matches(&self, text: &str) -> bool {
        let text: Vec<char> = text.chars().collect();
        self.matches_chars(&text)
    }

    fn matches_chars(&self, text: &[char]) -> bool {
        wildcard(&self.tokens, text, |want, got| want == fold(got))
    }

    /// Whether the glob matches all of `text`, or the part of it before a `separator`.
    #[must_use]
    pub fn matches_up_to(&self, text: &str, separator: &str) -> bool {
        let chars: Vec<char> = text.chars().collect();
        let sep: Vec<char> = separator.chars().collect();
        (0..=chars.len()).any(|end| {
            (end == chars.len() || chars[end..].starts_with(&sep))
                && self.matches_chars(&chars[..end])
        })
    }
}

/// The sort field's text as the engine stores it: the field with its markup removed.
#[must_use]
pub fn strip_html(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut in_tag = false;
    for c in field.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}
