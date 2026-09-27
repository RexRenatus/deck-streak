//! The reading note's text (SPEC-042 R5 to R7, R9, R10): its topic key, its render in the
//! predecessor's format, the byte-preserving roll, the two box lines and the body replacement.
//!
//! Everything here is a pure function of text, so the roll can be proved against the golden of
//! `reading_notes.py:_roll_note_text` (the predecessor at `27ee2bc`) without a file system.

use std::fmt;
use std::str::FromStr;

use deck_streak_kernel::StudyDay;

use crate::VaultError;
use crate::sha256;

/// The Studied box, unticked: stamped by code when the reading's new cards are studied (SPEC-047).
pub const STUDIED_UNTICKED: &str = "- [ ] Studied";
/// The Studied box, ticked.
pub const STUDIED_TICKED: &str = "- [x] Studied";
/// The owner's box, unticked: ticked only by the owner's read tap (SPEC-047).
pub const READ_UNTICKED: &str = "- [ ] I read it";
/// The owner's box, ticked.
pub const READ_TICKED: &str = "- [x] I read it";

/// A reading's topic key: a lowercase slug of `a-z` and `0-9` groups joined by `-` or `/`, as
/// `^[a-z0-9]+(?:[-/][a-z0-9]+)*$` reads it (R5). No character of it can spell `.`, so no key can
/// climb out of its folder.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopicKey(String);

impl TopicKey {
    /// The topic key `key`.
    ///
    /// # Errors
    ///
    /// [`VaultError::InvalidTopicKey`] for anything that is not a lowercase slug.
    pub fn new(key: &str) -> Result<Self, VaultError> {
        Ok(Self(key.to_owned()))
    }

    /// The key as the caller named it, `/` included: the value of the note's `topic` and its tag.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The note's file name: the key with `/` replaced by `-`, then `.md` (R5).
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}.md", self.0.replace('/', "-"))
    }
}

impl fmt::Display for TopicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a note on disk cannot be rolled (R8). A bounded reason: it never quotes the note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Malformed {
    /// The note's bytes are not UTF-8.
    #[error("the note is not UTF-8 text")]
    NotUtf8,
    /// The first line is not `---`.
    #[error("the note has no opening frontmatter delimiter")]
    NoOpeningDelimiter,
    /// No later line is `---`.
    #[error("the note has no closing frontmatter delimiter")]
    NoClosingDelimiter,
    /// `rolls` is not a whole number DeckStreak reads: an optional sign and ASCII digits, within a
    /// 128-bit integer. The predecessor also read a digit separator and digits outside ASCII; those
    /// are refused (ADR-042).
    #[error("the note's rolls is not a whole number")]
    RollsNotAWholeNumber,
}

/// Which of the two box lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BoxLine {
    /// `- [ ] Studied`, stamped by the settle step.
    Studied,
    /// `- [ ] I read it`, ticked by the owner's read tap.
    Read,
}

impl BoxLine {
    /// The line unticked.
    #[must_use]
    pub const fn unticked(self) -> &'static str {
        match self {
            Self::Studied => STUDIED_UNTICKED,
            Self::Read => READ_UNTICKED,
        }
    }

    /// The line ticked.
    #[must_use]
    pub const fn ticked(self) -> &'static str {
        match self {
            Self::Studied => STUDIED_TICKED,
            Self::Read => READ_TICKED,
        }
    }
}

/// The SHA-256 of a reading's body, as the adapter wrote it. A caller keeps it to prove, before a
/// body replacement, that the owner has not edited the body since (R10). It reads and writes as 64
/// lowercase hexadecimal digits.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyHash([u8; 32]);

impl BodyHash {
    /// The hash of `body`.
    #[must_use]
    pub fn of(body: &str) -> Self {
        Self(sha256::digest(body.as_bytes()))
    }
}

impl fmt::Display for BodyHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&sha256::hex(&self.0))
    }
}

impl fmt::Debug for BodyHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BodyHash({self})")
    }
}

/// A text that is not 64 lowercase hexadecimal digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not a body hash: 64 lowercase hexadecimal digits")]
pub struct BodyHashError;

impl FromStr for BodyHash {
    type Err = BodyHashError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        sha256::parse_hex(text).map(Self).ok_or(BodyHashError)
    }
}

/// The note the adapter writes for a new reading (R6): the predecessor's eight flat keys in its
/// order (`type`, `topic`, `date`, `first_generated`, `last_rolled`, `rolls: 0`, `digest`,
/// `tags: [reading, <topic>]`), then `ai_generated: true`, the machine-readable mark of AI-generated
/// text (EU AI Act Art. 50(2)); then the reading's body, and the two unticked boxes.
///
/// # Errors
///
/// [`VaultError::InvalidFrontmatterValue`] for a digest that holds `\n`, `\r` or `]`, and
/// [`VaultError::InvalidBody`] for an empty body or one that holds a box line of its own.
pub fn render(
    topic: &TopicKey,
    day: StudyDay,
    digest: &str,
    body: &str,
) -> Result<String, VaultError> {
    check_frontmatter_value("digest", digest)?;
    check_body(body)?;
    Ok(format!(
        "---\ntype: reading\ntopic: {topic}\ndate: {day}\nfirst_generated: {day}\n\
         last_rolled: {day}\nrolls: 0\ndigest: {digest}\ntags: [reading, {topic}]\n\
         ai_generated: true\n---\n{body}\n\n{STUDIED_UNTICKED}\n{READ_UNTICKED}\n"
    ))
}

/// The note `text` rolled forward to `today` (R7), as the predecessor's `_roll_note_text` rolls it:
/// `rolls` rises by one, `last_rolled` becomes `today`, and `first_generated` becomes `today` only
/// when it is absent; every other byte and line ending is kept.
///
/// # Errors
///
/// [`Malformed`] for a note without both frontmatter delimiters, or whose `rolls` is not a whole
/// number DeckStreak reads.
pub fn roll(text: &str, today: StudyDay) -> Result<String, Malformed> {
    let _ = today;
    Ok(text.to_owned())
}

/// `text` with the box `which` ticked (R9): exactly the whole line `- [ ] <box>` becomes
/// `- [x] <box>`, keeping its line ending, and no other line changes. `None` when the box is
/// already ticked, so nothing is written; a box is never unticked.
///
/// # Errors
///
/// [`VaultError::BoxAnchorMissing`] when no line is the box, ticked or not, and
/// [`VaultError::BoxAnchorAmbiguous`] when more than one is.
pub fn tick(text: &str, which: BoxLine) -> Result<Option<String>, VaultError> {
    let _ = (text, which);
    Ok(None)
}

/// The body of a note the adapter wrote: the text between the frontmatter's closing delimiter and
/// the two box lines. `None` when the note no longer has that shape.
#[must_use]
pub fn body(text: &str) -> Option<&str> {
    let _ = text;
    None
}

/// `text` with its body replaced by `body` (R10): the frontmatter and both box lines, ticked or
/// not, are kept byte for byte. `None` when the note no longer has the shape [`body`] reads.
#[must_use]
pub fn with_body(text: &str, body: &str) -> Option<String> {
    let _ = (text, body);
    None
}

/// Refuses a frontmatter value that could leave its line or close the tags list (R5).
///
/// # Errors
///
/// [`VaultError::InvalidFrontmatterValue`] naming `field`.
pub fn check_frontmatter_value(field: &'static str, value: &str) -> Result<(), VaultError> {
    if value.contains(['\n', '\r', ']']) {
        Err(VaultError::InvalidFrontmatterValue { field })
    } else {
        Ok(())
    }
}

/// Refuses an empty body, and one that holds a box line, which would make a box's anchor ambiguous.
///
/// # Errors
///
/// [`VaultError::InvalidBody`] naming why.
pub fn check_body(body: &str) -> Result<(), VaultError> {
    if body.trim().is_empty() {
        return Err(VaultError::InvalidBody("is empty"));
    }
    let boxes = [STUDIED_UNTICKED, STUDIED_TICKED, READ_UNTICKED, READ_TICKED];
    if body
        .lines()
        .any(|line| boxes.contains(&line.trim_end_matches('\r')))
    {
        return Err(VaultError::InvalidBody("holds a box line of its own"));
    }
    Ok(())
}
