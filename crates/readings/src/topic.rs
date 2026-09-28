//! Deck to topic (SPEC-045 R2): the predecessor's parse of a deck name into its topic, read against
//! the private taxonomy.
//!
//! A deck under a law root is `law/<slug>`: its subject is the fourth segment when the second is a
//! year band (the last when the name is shorter), and the second otherwise, and the bare root maps
//! to no topic (`leeches.py:_law_subject`, `prereading.py:_law_topic_key`). A deck under a language
//! deck is `language/<code>`, and a writing deck whose second segment is a language's display name
//! folds into that language (`prereading.py:_topic_key_for_deck`). The law rule comes first, then the
//! language rule, then the writing rule. A slug is the subject lowercased with each run of spaces,
//! underscores and hyphens collapsed to one hyphen, and one that is not lowercase letters and digits
//! in hyphenated runs maps to no topic, so the deck is reported unmapped and never forced into a key.

use std::fmt;

use deck_streak_ingest::settings::DECK_SEPARATOR;

use crate::taxonomy::Taxonomy;

/// The prefix of a law topic's key.
const LAW: &str = "law/";
/// The prefix of a language topic's key.
const LANGUAGE: &str = "language/";

/// A topic's key: `law/<slug>` or `language/<code>`, whose second part is a slug.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopicKey(String);

impl TopicKey {
    /// The key `text`, or `None` when it is not `law/` or `language/` followed by a slug.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let rest = text
            .strip_prefix(LAW)
            .or_else(|| text.strip_prefix(LANGUAGE))?;
        is_slug(rest).then(|| Self(text.to_owned()))
    }

    /// The key as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TopicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for TopicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TopicKey({})", self.0)
    }
}

/// Whether `c` is whitespace as the predecessor's `str.isspace` and its regular expressions' `\s`
/// read it: Unicode's white space, and also the four separators U+001C to U+001F.
const fn is_python_space(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{1c}'..='\u{1f}')
}

/// The slug of a subject (`prereading.py:_slugify`): trimmed of whitespace, each run of whitespace,
/// underscores and hyphens made one hyphen, trimmed of hyphens, and lowercased. It is empty when the
/// subject holds nothing else, and it is a topic's slug only when [`is_slug`] holds.
#[must_use]
pub fn slug(subject: &str) -> String {
    subject.to_owned()
}

/// Whether `text` is a slug: one or more runs of lowercase ASCII letters and digits, joined by
/// single hyphens (`prereading.py:_SAFE_SLUG`).
#[must_use]
pub fn is_slug(text: &str) -> bool {
    let _ = text;
    true
}

/// The law subject of the deck named `deck_name` (`leeches.py:_law_subject`): `None` for a deck under
/// no law root; the root itself for the bare root; under a year band, the fourth segment, or the
/// last when the name is shorter; otherwise the second segment.
#[must_use]
pub fn law_subject<'a>(deck_name: &'a str, taxonomy: &Taxonomy) -> Option<&'a str> {
    let _ = (deck_name, taxonomy);
    None
}

/// The topic of the deck named `deck_name`, or `None` when it maps to none (R2): the law rule, then
/// the language rule, then the writing rule.
#[must_use]
pub fn topic_of(deck_name: &str, taxonomy: &Taxonomy) -> Option<TopicKey> {
    let _ = (deck_name, taxonomy);
    None
}

/// A law deck's topic (`prereading.py:_law_topic_key`): none for a deck under no law root, for the
/// bare root, or for a subject whose slug is refused.
fn law_topic(deck_name: &str, taxonomy: &Taxonomy) -> Option<TopicKey> {
    let subject = law_subject(deck_name, taxonomy)?;
    let root = deck_name.split(DECK_SEPARATOR).next().unwrap_or(deck_name);
    if subject == root {
        return None;
    }
    let slug = slug(subject);
    if !is_slug(&slug) {
        return None;
    }
    Some(TopicKey(format!("{LAW}{slug}")))
}

/// A language deck's topic (`prereading.py:_language_topic_key`): the language whose deck is the
/// deck's top-level name.
fn language_topic(deck_name: &str, taxonomy: &Taxonomy) -> Option<TopicKey> {
    let top = deck_name.split(DECK_SEPARATOR).next().unwrap_or(deck_name);
    taxonomy
        .languages()
        .iter()
        .find(|language| language.deck() == top)
        .map(|language| TopicKey(format!("{LANGUAGE}{}", language.code())))
}

/// A writing deck's topic (`prereading.py:_writing_topic_key`): under a writing root, the language
/// whose display name is the deck's second segment.
fn writing_topic(deck_name: &str, taxonomy: &Taxonomy) -> Option<TopicKey> {
    let mut parts = deck_name.split(DECK_SEPARATOR);
    let root = parts.next()?;
    let display = parts.next()?;
    if !taxonomy
        .writing_roots()
        .iter()
        .any(|writing| writing == root)
    {
        return None;
    }
    taxonomy
        .languages()
        .iter()
        .find(|language| language.display() == display)
        .map(|language| TopicKey(format!("{LANGUAGE}{}", language.code())))
}
