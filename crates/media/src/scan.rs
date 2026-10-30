//! The one grammar that finds every media name a note's text writes (SPEC-513 R4; ADR-531 D4).
//!
//! It reads what the engine's media check reads: a `src` or `data` attribute of an `img`, `audio`,
//! `video`, `object` or `source` tag, and a `[sound:…]` tag. The tag names and the attribute names
//! match in any case, `src` and `data` only where a word starts, quoted or unquoted, and a tag's
//! run of attributes may hold a quoted `>`; the sound tag matches in lower case only and names
//! everything up to its first `]`. A remote reference (`http://` or `https://`, in any case) is
//! dropped, and a name's HTML entities are decoded. The names come in the engine's order: every
//! tag's, in the order written, then every sound tag's.
//!
//! What it does not do, each a divergence the decision records: render LaTeX into the image names
//! the engine derives from it, write an inline `data:image/…;base64,` image to a file, normalize a
//! name to NFC, and decode a named entity other than `&amp;`, `&lt;`, `&gt;`, `&quot;` and
//! `&nbsp;` (a numeric one is decoded).

use std::borrow::Cow;

/// The tags whose `src` or `data` attribute names a file, in the order the grammar tries them.
pub const TAGS: [&str; 5] = ["img", "audio", "video", "object", "source"];

/// The attributes that name a file, in the order the grammar tries them.
pub const ATTRIBUTES: [&str; 2] = ["src", "data"];

/// The opening of a sound tag, matched in lower case only.
pub const SOUND_OPEN: &str = "[sound:";

/// The long s, which matches `s` when case is ignored, as Unicode's simple case folding has it.
const LONG_S: char = '\u{17f}';

/// The schemes of a remote reference, matched in any case at the start of a written name.
const REMOTE: [&str; 2] = ["https://", "http://"];

/// The quotes an attribute's value may be written in, in the order the grammar tries them. A
/// quoted run in a tag's attributes starts with either; at most one applies at a position.
const QUOTES: [u8; 2] = *b"\"'";

/// Every media name `text` writes, in the engine's order: each tag's name as written, then each
/// sound tag's, remote references dropped and entities decoded. A name written twice is listed
/// twice.
#[must_use]
pub fn scan(text: &str) -> Vec<String> {
    tag_names(text)
        .into_iter()
        .chain(sound_names(text))
        .filter(|written| !is_remote(written))
        .map(|written| decode_entities(written).into_owned())
        .collect()
}

/// Whether a written name is remote: it starts with `http://` or `https://`, in any case.
#[must_use]
pub fn is_remote(written: &str) -> bool {
    REMOTE
        .iter()
        .any(|scheme| match_ignoring_case(written, 0, scheme).is_some())
}

/// `written` with its HTML entities decoded, as the engine decodes a name, a no-break space then
/// read as a space. A name with no `&` is returned as written, and so is one holding an entity the
/// decoder does not know or cannot close: the engine keeps such a name whole.
#[must_use]
pub fn decode_entities(written: &str) -> Cow<'_, str> {
    if !written.contains('&') {
        return Cow::Borrowed(written);
    }
    decode_html(written).map_or(Cow::Borrowed(written), |decoded| {
        Cow::Owned(decoded.replace('\u{a0}', " "))
    })
}

/// Every entity of `text` decoded, or `None` when one is unknown, malformed or unclosed.
fn decode_html(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let close = after.find(';')?;
        out.push(entity(&after[..close])?);
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// The character an entity's body names: `#` and decimal digits, `#x` and hex digits, or one of
/// the five names the decoder knows.
fn entity(body: &str) -> Option<char> {
    if let Some(number) = body.strip_prefix('#') {
        let (digits, radix) = number
            .strip_prefix('x')
            .map_or((number, 10), |hex| (hex, 16));
        if digits.is_empty() || !digits.chars().all(|digit| digit.is_digit(radix)) {
            return None;
        }
        return u32::from_str_radix(digits, radix)
            .ok()
            .and_then(char::from_u32);
    }
    match body {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "nbsp" => Some('\u{a0}'),
        _ => None,
    }
}

/// The names written in tags, in the order written: each match starts at a `<`, and the search
/// resumes after it. One mark per position records which start last tried it.
fn tag_names(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut marks = vec![0; text.len() + 1];
    let mut from = 0;
    while let Some(offset) = text[from..].find('<') {
        let start = from + offset;
        match tag_at(text, start, &mut marks) {
            Some((name, end)) => {
                found.push(name);
                from = end;
            }
            None => from = start + 1,
        }
    }
    found
}

/// The name the tag opening at `start` writes, and where its match ends. After the tag's name, at
/// least one attribute item (a character other than `>`, or a quoted run of at least one
/// character) comes before the attribute, and the items are tried as the engine's lazy match tries
/// them: the attribute first, then one character, then a quoted run, depth first. A position this
/// start already tried failed, so it is not tried again.
fn tag_at<'t>(text: &'t str, start: usize, marks: &mut [usize]) -> Option<(&'t str, usize)> {
    let after = start + 1;
    if !text[after..].chars().next().is_some_and(is_word) {
        return None;
    }
    let open = TAGS.iter().find_map(|tag| {
        let end = match_ignoring_case(text, after, tag)?;
        ends_word(text, end).then_some(end)
    })?;
    let stamp = after;
    let mut stack = Vec::new();
    push_items(text, open, &mut stack);
    while let Some(at) = stack.pop() {
        if marks[at] == stamp {
            continue;
        }
        marks[at] = stamp;
        if let Some(found) = attribute_at(text, at) {
            return Some(found);
        }
        push_items(text, at, &mut stack);
    }
    None
}

/// Pushes where each attribute item at `at` ends, the one tried first pushed last.
fn push_items(text: &str, at: usize, stack: &mut Vec<usize>) {
    let bytes = text.as_bytes();
    let Some(&first) = bytes.get(at) else {
        return;
    };
    for quote in QUOTES {
        if first == quote
            && let Some(close) = find_byte(bytes, at + 1, quote)
            && close > at + 1
        {
            stack.push(close + 1);
        }
    }
    if first != b'>'
        && let Some(character) = text[at..].chars().next()
    {
        stack.push(at + character.len_utf8());
    }
}

/// The name an attribute starting at `at` writes, and where the match ends: `src` or `data` at
/// the start of a word, `=`, then the value.
fn attribute_at(text: &str, at: usize) -> Option<(&str, usize)> {
    if text[..at].chars().next_back().is_none_or(is_word) {
        return None;
    }
    ATTRIBUTES.iter().find_map(|attribute| {
        let end = match_ignoring_case(text, at, attribute)?;
        if text.as_bytes().get(end) != Some(&b'=') {
            return None;
        }
        value_at(text, end + 1)
    })
}

/// An attribute's value at `at`: a double-quoted name of at least one character, then a single-
/// quoted one, each followed by the tag's `>`; else an unquoted name of at least one character
/// other than a space or `>`, followed by `>` or by a space and the tag's `>`.
fn value_at(text: &str, at: usize) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    for quote in QUOTES {
        if bytes.get(at) == Some(&quote)
            && let Some(close) = find_byte(bytes, at + 1, quote)
            && close > at + 1
            && let Some(end) = find_byte(bytes, close + 1, b'>')
        {
            return Some((&text[at + 1..close], end + 1));
        }
    }
    let stop = bytes[at..]
        .iter()
        .position(|&byte| byte == b' ' || byte == b'>')
        .map_or(bytes.len(), |offset| at + offset);
    if stop == at {
        return None;
    }
    match bytes.get(stop) {
        Some(b'>') => Some((&text[at..stop], stop + 1)),
        Some(b' ') => find_byte(bytes, stop + 1, b'>').map(|end| (&text[at..stop], end + 1)),
        _ => None,
    }
}

/// The names written in sound tags, in the order written.
fn sound_names(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].find(SOUND_OPEN) {
        let start = from + offset;
        match sound_at(text, start + SOUND_OPEN.len()) {
            Some((name, end)) => {
                found.push(name);
                from = end;
            }
            None => from = start + 1,
        }
    }
    found
}

/// A sound tag's name at `open`: at least one character, up to the first `]` after the first.
fn sound_at(text: &str, open: usize) -> Option<(&str, usize)> {
    let first = text[open..].chars().next()?.len_utf8();
    let close = open + first + text[open + first..].find(']')?;
    #[cfg(not(feature = "kani-witness-scan"))]
    let name = &text[open..close];
    // The witness's mutant: the sound tag's name is cut before its first character.
    #[cfg(feature = "kani-witness-scan")]
    let name = &text[open..open];
    Some((name, close + 1))
}

/// Where `literal`, ASCII lower case, matches `text` at `at` with case ignored, `ſ` matching `s`.
fn match_ignoring_case(text: &str, at: usize, literal: &str) -> Option<usize> {
    let mut found = text[at..].char_indices();
    for expected in literal.chars() {
        let (_, character) = found.next()?;
        let same =
            character.eq_ignore_ascii_case(&expected) || (expected == 's' && character == LONG_S);
        if !same {
            return None;
        }
    }
    Some(found.next().map_or(text.len(), |(offset, _)| at + offset))
}

/// Whether a word ends at `at`: the text ends there or the next character is not a word's.
fn ends_word(text: &str, at: usize) -> bool {
    !text[at..].chars().next().is_some_and(is_word)
}

/// Whether `character` is a word's: a letter, a number or `_`.
fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// The first `byte` of `bytes` at or after `from`.
fn find_byte(bytes: &[u8], from: usize, byte: u8) -> Option<usize> {
    bytes
        .get(from..)?
        .iter()
        .position(|&found| found == byte)
        .map(|offset| from + offset)
}
