//! The reading note's text (SPEC-042 R5 to R7, R9, R10): its topic key, its render in the
//! predecessor's format, the byte-preserving roll, the two box lines and the body replacement.
//!
//! Everything here is a pure function of text, so the roll can be proved against the golden of
//! `reading_notes.py:_roll_note_text` (the predecessor at `27ee2bc`) without a file system. The
//! roll reads a note as the predecessor's Python read it: lines split where `str.splitlines` splits
//! them (at `\n`, `\r`, `\r\n` and seven more boundaries), a delimiter line compared after
//! `rstrip("\r\n")`, and a value stripped of every character `str.isspace` counts.

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

/// The frontmatter's delimiter line.
const DELIMITER: &str = "---";

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
        let slug = !key.is_empty()
            && key.split(['-', '/']).all(|group| {
                !group.is_empty()
                    && group
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            });
        if slug {
            Ok(Self(key.to_owned()))
        } else {
            Err(VaultError::InvalidTopicKey)
        }
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
    /// `rolls` is not a whole number this port reads: an optional sign and ASCII digits, within a
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
/// when it is absent; a key the note lacks is appended to its frontmatter. The two delimiter lines
/// and every patched line end with the line ending of the text's first line feed, and every other
/// byte is kept.
///
/// # Errors
///
/// [`Malformed`] for a note without both frontmatter delimiters, or whose `rolls` is not a whole
/// number this port reads.
pub fn roll(text: &str, today: StudyDay) -> Result<String, Malformed> {
    let lines = split_lines(text);
    let close = closing_delimiter(&lines)?;
    let mut header: Vec<String> = lines[1..close]
        .iter()
        .map(|line| (*line).to_owned())
        .collect();
    let body = lines[close + 1..].concat();
    let newline = delimiter_newline(text);
    let rolls = match top_level_key(&header, "rolls") {
        Some((_, value)) => parse_rolls(value)?,
        None => 0,
    };
    let rolls = rolls
        .checked_add(1)
        .ok_or(Malformed::RollsNotAWholeNumber)?;
    set_top_level_key(&mut header, "rolls", &rolls.to_string(), newline);
    let day = today.to_string();
    set_top_level_key(&mut header, "last_rolled", &day, newline);
    if top_level_key(&header, "first_generated").is_none() {
        set_top_level_key(&mut header, "first_generated", &day, newline);
    }
    Ok(format!(
        "{DELIMITER}{newline}{}{DELIMITER}{newline}{body}",
        header.concat()
    ))
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
    let lines = split_lines(text);
    let mut anchors = lines.iter().enumerate().filter(|(_, line)| {
        let (content, _) = content_and_ending(line);
        content == which.unticked() || content == which.ticked()
    });
    let (index, line) = anchors.next().ok_or(VaultError::BoxAnchorMissing)?;
    if anchors.next().is_some() {
        return Err(VaultError::BoxAnchorAmbiguous);
    }
    let (content, ending) = content_and_ending(line);
    if content == which.ticked() {
        return Ok(None);
    }
    let mut ticked = String::with_capacity(text.len() + 1);
    for (at, other) in lines.iter().enumerate() {
        if at == index {
            ticked.push_str(which.ticked());
            ticked.push_str(ending);
        } else {
            ticked.push_str(other);
        }
    }
    Ok(Some(ticked))
}

/// The body of a note the adapter wrote: the text between the frontmatter's closing delimiter and
/// the two box lines. `None` when the note no longer has that shape.
#[must_use]
pub fn body(text: &str) -> Option<&str> {
    let (start, end) = body_span(text)?;
    Some(&text[start..end])
}

/// `text` with its body replaced by `body` (R10): the frontmatter and both box lines, ticked or
/// not, are kept byte for byte. `None` when the note no longer has the shape [`body`] reads.
#[must_use]
pub fn with_body(text: &str, body: &str) -> Option<String> {
    let (start, end) = body_span(text)?;
    Some(format!("{}{body}{}", &text[..start], &text[end..]))
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
    if body.trim_matches(is_python_space).is_empty() {
        return Err(VaultError::InvalidBody("is empty"));
    }
    let boxes = [STUDIED_UNTICKED, STUDIED_TICKED, READ_UNTICKED, READ_TICKED];
    if split_lines(body)
        .iter()
        .any(|line| boxes.contains(&content_and_ending(line).0))
    {
        return Err(VaultError::InvalidBody("holds a box line of its own"));
    }
    Ok(())
}

/// Whether `c` is whitespace to Python's `str.isspace` and `re`'s `\s`: Unicode's `White_Space`,
/// and the four information separators U+001C to U+001F, which Python counts and Rust does not.
pub(crate) fn is_python_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// `text` without the leading and trailing characters Python's `str.strip()` removes.
pub(crate) fn python_strip(text: &str) -> &str {
    text.trim_matches(is_python_space)
}

/// Whether `c` ends a line to Python's `str.splitlines`.
fn is_line_boundary(c: char) -> bool {
    matches!(
        u32::from(c),
        0x0a | 0x0b | 0x0c | 0x0d | 0x1c | 0x1d | 0x1e | 0x85 | 0x2028 | 0x2029
    )
}

/// `text` split as Python's `str.splitlines(keepends=True)` splits it: each line keeps its ending,
/// `\r\n` is one ending, and a last line without an ending is kept.
pub(crate) fn split_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if !is_line_boundary(c) {
            continue;
        }
        let mut end = at + c.len_utf8();
        if c == '\r' && chars.peek().is_some_and(|&(_, next)| next == '\n') {
            chars.next();
            end += 1;
        }
        lines.push(&text[start..end]);
        start = end;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// A line without the trailing `\r` and `\n` characters Python's `rstrip("\r\n")` removes.
fn without_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

/// A line's content and its own ending: `\r\n`, `\n`, or none.
fn content_and_ending(line: &str) -> (&str, &str) {
    if let Some(content) = line.strip_suffix("\r\n") {
        (content, "\r\n")
    } else if let Some(content) = line.strip_suffix('\n') {
        (content, "\n")
    } else {
        (line, "")
    }
}

/// The index of the frontmatter's closing delimiter: the first line after the first that is `---`,
/// when the first is `---` too.
fn closing_delimiter(lines: &[&str]) -> Result<usize, Malformed> {
    match lines.first() {
        Some(first) if without_line_ending(first) == DELIMITER => {}
        _ => return Err(Malformed::NoOpeningDelimiter),
    }
    lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| without_line_ending(line) == DELIMITER)
        .map(|(index, _)| index)
        .ok_or(Malformed::NoClosingDelimiter)
}

/// The line ending of the text's first line feed: `\r\n` when a carriage return comes before it,
/// and `\n` otherwise, as `_detect_delimiter_newline` reads it.
fn delimiter_newline(text: &str) -> &'static str {
    match text.find('\n') {
        Some(at) if at > 0 && text.as_bytes()[at - 1] == b'\r' => "\r\n",
        _ => "\n",
    }
}

/// The first top-level `key: value` line of `header` whose key is `key`: its index and its value,
/// stripped. An indented line or a list item is never top-level.
fn top_level_key<'a>(header: &'a [String], key: &str) -> Option<(usize, &'a str)> {
    header.iter().enumerate().find_map(|(index, line)| {
        let content = without_line_ending(line);
        if content.is_empty() || content.starts_with([' ', '\t', '-']) {
            return None;
        }
        let (name, value) = key_line(content)?;
        (name == key).then(|| (index, python_strip(value)))
    })
}

/// A line of the shape `^([A-Za-z_][A-Za-z0-9_]*):(.*)$`, as its key and its raw value.
fn key_line(content: &str) -> Option<(&str, &str)> {
    let bytes = content.as_bytes();
    let first = *bytes.first()?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let end = bytes
        .iter()
        .position(|byte| !(byte.is_ascii_alphanumeric() || *byte == b'_'))
        .unwrap_or(bytes.len());
    (bytes.get(end) == Some(&b':')).then(|| (&content[..end], &content[end + 1..]))
}

/// Replaces the first top-level `key` line's value, or appends `key: value` when there is none.
fn set_top_level_key(header: &mut Vec<String>, key: &str, value: &str, newline: &str) {
    let line = format!("{key}: {value}{newline}");
    match top_level_key(header, key) {
        Some((index, _)) => header[index] = line,
        None => header.push(line),
    }
}

/// `rolls`, already stripped, as a whole number: an optional sign and ASCII digits.
fn parse_rolls(value: &str) -> Result<i128, Malformed> {
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Malformed::RollsNotAWholeNumber);
    }
    value.parse().map_err(|_| Malformed::RollsNotAWholeNumber)
}

/// The byte span of a note's body: from after the frontmatter's closing delimiter to the blank line
/// before the two box lines, which end the note.
fn body_span(text: &str) -> Option<(usize, usize)> {
    let lines = split_lines(text);
    let close = closing_delimiter(&lines).ok()?;
    let start: usize = lines[..=close].iter().map(|line| line.len()).sum();
    for studied in [STUDIED_UNTICKED, STUDIED_TICKED] {
        for read in [READ_UNTICKED, READ_TICKED] {
            let tail = format!("\n\n{studied}\n{read}\n");
            if text.len() >= start + tail.len() && text.ends_with(&tail) {
                return Some((start, text.len() - tail.len()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{is_python_space, split_lines};

    #[test]
    fn lines_split_where_python_splits_them() {
        let text = "a\r\nb\rc\u{0b}d\u{0c}e\u{1c}f\u{85}g\u{2028}h\u{2029}i\nj";
        assert_eq!(
            split_lines(text),
            vec![
                "a\r\n",
                "b\r",
                "c\u{0b}",
                "d\u{0c}",
                "e\u{1c}",
                "f\u{85}",
                "g\u{2028}",
                "h\u{2029}",
                "i\n",
                "j"
            ]
        );
        assert_eq!(split_lines("one\n"), vec!["one\n"]);
        assert_eq!(split_lines(""), Vec::<&str>::new());
    }

    #[test]
    fn the_information_separators_are_python_whitespace() {
        assert!(('\x1c'..='\x1f').all(is_python_space));
        assert!(is_python_space('\u{3000}'));
        assert!(!is_python_space('\u{180e}'));
    }
}
