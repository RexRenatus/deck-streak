//! The face rewrite (SPEC-601 R9; ADR-621 D8): every media reference the one grammar finds in a
//! note's HTML, rewritten in place by the caller, every other byte kept. A face reads its media
//! names through the same grammar the store, the sweep and the sync read them through.
//!
//! [`crate::scan`] answers names and this module answers places. Each place is found by the
//! grammar's own rules, over its public constants, [`each_match`] and its case rule: a `src` or
//! `data` attribute of a tag in [`TAGS`], then a sound tag's name. A remote reference is visited
//! too, because the caller decides what each name reads.

use core::ops::{ControlFlow, Range};

use crate::scan::{ATTRIBUTES, SOUND_OPEN, TAGS, decode_entities, each_match, match_ignoring_case};

/// The quotes an attribute's value may be written in, in the order the grammar tries them.
const QUOTES: [u8; 2] = *b"\"'";

/// `html` with each media reference the grammar finds, in order, replaced by what `to` answers for
/// its name; every other byte of `html` is kept. `to` is asked for each tag's name in the order
/// written, then for each sound tag's, each name with its HTML entities decoded. A place inside an
/// earlier place keeps the earlier answer.
pub fn rewrite(html: &str, mut to: impl FnMut(&str) -> String) -> String {
    let mut answers: Vec<(Range<usize>, String)> = places(html)
        .into_iter()
        .map(|place| {
            let answer = to(decode_entities(&html[place.start..place.end]).as_ref());
            (place, answer)
        })
        .collect();
    answers.sort_by_key(|(place, _)| place.start);
    let mut written = String::with_capacity(html.len());
    let mut kept = 0;
    for (place, answer) in answers {
        if place.start < kept {
            continue;
        }
        written.push_str(&html[kept..place.start]);
        written.push_str(&answer);
        kept = place.end;
    }
    written.push_str(&html[kept..]);
    written
}

/// Where each media name `html` writes lies, in the grammar's order: each tag's name, in the order
/// written, then each sound tag's.
fn places(html: &str) -> Vec<Range<usize>> {
    let mut marks = vec![0; html.len() + 1];
    let tags = each_match(html, "<", |start| {
        ControlFlow::Continue(tag_at(html, start, &mut marks))
    });
    let sounds = each_match(html, SOUND_OPEN, |start| {
        ControlFlow::Continue(sound_at(html, start + SOUND_OPEN.len()))
    });
    tags.into_iter().chain(sounds).collect()
}

/// Where the name of the tag opening at `start` lies, and where its match ends, by the grammar's
/// rule: after the tag's name, at least one attribute item (a character other than `>`, or a
/// quoted run of at least one character) comes before the attribute, tried depth first, the
/// attribute first, then one character, then a quoted run. A position this start already tried
/// failed, so it is not tried again.
fn tag_at(text: &str, start: usize, marks: &mut [usize]) -> Option<(Range<usize>, usize)> {
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

/// Where the name an attribute starting at `at` writes lies, and where the match ends: `src` or
/// `data` at the start of a word, `=`, then the value.
fn attribute_at(text: &str, at: usize) -> Option<(Range<usize>, usize)> {
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

/// Where an attribute's value at `at` lies: a double-quoted name of at least one character, then
/// a single-quoted one, each followed by the tag's `>`; else an unquoted name of at least one
/// character other than a space or `>`, followed by `>` or by a space and the tag's `>`.
fn value_at(text: &str, at: usize) -> Option<(Range<usize>, usize)> {
    let bytes = text.as_bytes();
    for quote in QUOTES {
        if bytes.get(at) == Some(&quote)
            && let Some(close) = find_byte(bytes, at + 1, quote)
            && close > at + 1
            && let Some(end) = find_byte(bytes, close + 1, b'>')
        {
            return Some((at + 1..close, end + 1));
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
        Some(b'>') => Some((at..stop, stop + 1)),
        Some(b' ') => find_byte(bytes, stop + 1, b'>').map(|end| (at..stop, end + 1)),
        _ => None,
    }
}

/// Where a sound tag's name at `open` lies: at least one character, up to the first `]` after the
/// first.
fn sound_at(text: &str, open: usize) -> Option<(Range<usize>, usize)> {
    let first = text[open..].chars().next()?.len_utf8();
    let close = open + first + text[open + first..].find(']')?;
    Some((open..close, close + 1))
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
