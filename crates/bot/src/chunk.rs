//! Splitting a long HTML text into messages Telegram accepts (SPEC-026 R7; the telegram-platform
//! pack's formatting rules).
//!
//! Telegram measures a text after entity parsing, in UTF-16 units: a tag counts nothing, an entity
//! such as `&amp;` counts as the one character it stands for, and a character outside the Basic
//! Multilingual Plane counts two. A text over the bound is split at the last paragraph that fits,
//! else the last line, else the last word, and only when none fits, between two characters; a cut
//! never falls inside a tag or an entity, and never between the two halves of a character. The
//! whitespace at a cut stays at the end of the chunk before it, so the chunks' visible texts, joined,
//! are the text's. A tag open at a cut is closed at the end of its chunk and opened again, with the
//! same attributes, at the start of the next, so every chunk parses on its own. The predecessor cut
//! at a character count between lines (`telegram.py:_chunk`); a cut inside a tag makes Telegram
//! refuse the chunk.

/// One piece of the markup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Piece<'a> {
    /// One character or one entity, as written, with the UTF-16 units it counts after parsing and
    /// the character it stands for.
    Visible {
        raw: &'a str,
        units: usize,
        character: char,
    },
    /// An opening tag, as written, and its name.
    Open { raw: &'a str, name: &'a str },
    /// A closing tag's name.
    Close { name: &'a str },
    /// A tag that opens and closes itself, which no stack follows.
    Alone { raw: &'a str },
}

impl Piece<'_> {
    /// The units this piece counts after entity parsing.
    const fn units(&self) -> usize {
        match self {
            Self::Visible { units, .. } => *units,
            Self::Open { .. } | Self::Close { .. } | Self::Alone { .. } => 0,
        }
    }

    /// The character this piece shows, when it shows one.
    const fn shown(&self) -> Option<char> {
        match self {
            Self::Visible { character, .. } => Some(*character),
            Self::Open { .. } | Self::Close { .. } | Self::Alone { .. } => None,
        }
    }
}

/// The UTF-16 units `html` counts after entity parsing: what Telegram's bounds measure.
#[must_use]
pub fn visible_units(html: &str) -> usize {
    pieces(html).iter().map(Piece::units).sum()
}

/// `html` split into chunks of at most `limit` UTF-16 units each after entity parsing, each of
/// which parses on its own. A text that fits is one chunk, exactly as given; a text with nothing
/// but whitespace to show is no chunk at all, since Telegram refuses an empty message.
#[must_use]
pub fn chunks(html: &str, limit: usize) -> Vec<String> {
    let pieces = pieces(html);
    if !pieces
        .iter()
        .any(|piece| piece.shown().is_some_and(|shown| !shown.is_whitespace()))
    {
        return Vec::new();
    }
    if pieces.iter().map(Piece::units).sum::<usize>() <= limit {
        return vec![html.to_owned()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut open: Vec<Piece<'_>> = Vec::new();
    while start < pieces.len() {
        let end = cut(&pieces, start, limit);
        let text = render(&open, &pieces[start..end]);
        open = after(open, &pieces[start..end]);
        if pieces[start..end]
            .iter()
            .any(|piece| piece.shown().is_some_and(|shown| !shown.is_whitespace()))
        {
            chunks.push(text);
        }
        start = end;
    }
    chunks
}

/// Where the chunk starting at `start` ends: after the last paragraph break that fits in `limit`,
/// else the last line break, else the last word break; else before the first piece that does not
/// fit. A chunk always takes at least one visible piece, and its end never leaves an opening tag
/// last or a closing tag first.
fn cut(pieces: &[Piece<'_>], start: usize, limit: usize) -> usize {
    let mut units = 0;
    let mut seen_text = false;
    // The last cut of each kind: a paragraph, a line, a word.
    let mut breaks: [Option<usize>; 3] = [None; 3];
    let mut end = pieces.len();
    for (index, piece) in pieces.iter().enumerate().skip(start) {
        // A piece that would carry the chunk over the bound ends it, unless the chunk has nothing
        // yet: a lone piece over the bound goes alone rather than never.
        let counted = piece.units();
        if units > 0 && units + counted > limit {
            end = index;
            break;
        }
        units += counted;
        let Some(shown) = piece.shown() else {
            continue;
        };
        if !shown.is_whitespace() {
            seen_text = true;
            continue;
        }
        if !seen_text {
            continue;
        }
        // Past the chunk's first text, so the piece before this one is in the chunk.
        let after = index + 1;
        if shown == '\n' {
            let paragraph = pieces[index - 1].shown() == Some('\n');
            breaks[usize::from(!paragraph)] = Some(after);
        } else if shown == ' ' {
            breaks[2] = Some(after);
        }
    }
    if end == pieces.len() {
        return end;
    }
    let chosen = breaks.into_iter().flatten().next().unwrap_or(end);
    settle(pieces, start, chosen)
}

/// Moves a cut at `end` off an opening tag it would leave last, and past the closing tags that
/// follow it, so no chunk ends by opening a tag or starts by closing one.
fn settle(pieces: &[Piece<'_>], start: usize, mut end: usize) -> usize {
    while end > start + 1 && matches!(pieces[end - 1], Piece::Open { .. }) {
        end -= 1;
    }
    while end < pieces.len() && matches!(pieces[end], Piece::Close { .. }) {
        end += 1;
    }
    end
}

/// A chunk's markup: the tags open at its start opened again, its pieces, and the tags still open
/// at its end closed, innermost first.
fn render(open: &[Piece<'_>], pieces: &[Piece<'_>]) -> String {
    let mut text = String::new();
    for tag in open {
        if let Piece::Open { raw, .. } = tag {
            text.push_str(raw);
        }
    }
    for piece in pieces {
        match piece {
            Piece::Visible { raw, .. } | Piece::Open { raw, .. } | Piece::Alone { raw } => {
                text.push_str(raw);
            }
            Piece::Close { name } => {
                text.push_str("</");
                text.push_str(name);
                text.push('>');
            }
        }
    }
    for tag in after(open.to_vec(), pieces).iter().rev() {
        if let Piece::Open { name, .. } = tag {
            text.push_str("</");
            text.push_str(name);
            text.push('>');
        }
    }
    text
}

/// The tags open after `pieces`, given the tags `open` before them.
fn after<'a>(mut open: Vec<Piece<'a>>, pieces: &[Piece<'a>]) -> Vec<Piece<'a>> {
    for piece in pieces {
        match piece {
            Piece::Open { .. } => open.push(*piece),
            Piece::Close { name } => {
                let innermost = open.iter().rposition(
                    |tag| matches!(tag, Piece::Open { name: opened, .. } if opened == name),
                );
                if let Some(position) = innermost {
                    open.truncate(position);
                }
            }
            Piece::Visible { .. } | Piece::Alone { .. } => {}
        }
    }
    open
}

/// `html` as its pieces. A `<` that starts no tag and an `&` that starts no entity are one
/// character each, so no text is ever lost, even from markup Telegram would refuse.
fn pieces(html: &str) -> Vec<Piece<'_>> {
    let mut pieces = Vec::new();
    let mut rest = html;
    while let Some(first) = rest.chars().next() {
        let piece = match first {
            '<' => tag(rest),
            '&' => entity(rest),
            _ => None,
        }
        .unwrap_or_else(|| {
            let raw = &rest[..first.len_utf8()];
            Piece::Visible {
                raw,
                units: first.len_utf16(),
                character: first,
            }
        });
        let length = match piece {
            Piece::Visible { raw, .. } | Piece::Open { raw, .. } | Piece::Alone { raw } => {
                raw.len()
            }
            Piece::Close { name } => name.len() + 3,
        };
        pieces.push(piece);
        rest = &rest[length..];
    }
    pieces
}

/// The tag `text` starts with: `<name ...>`, `</name>` or `<name .../>`, whose name starts with an
/// ASCII letter.
fn tag(text: &str) -> Option<Piece<'_>> {
    let end = text.find('>')?;
    let raw = &text[..=end];
    let inner = &raw[1..end];
    if let Some(name) = inner.strip_prefix('/') {
        return (is_name(name) && raw.len() == name.len() + 3).then_some(Piece::Close { name });
    }
    let name_end = inner
        .find(|character: char| character.is_whitespace() || character == '/')
        .unwrap_or(inner.len());
    let name = &inner[..name_end];
    if !is_name(name) {
        return None;
    }
    if inner.ends_with('/') {
        Some(Piece::Alone { raw })
    } else {
        Some(Piece::Open { raw, name })
    }
}

/// Whether `name` is a tag name: an ASCII letter, then letters, digits or `-`.
fn is_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|next| next.is_ascii_alphanumeric() || next == '-')
}

/// The longest entity this reads, `&#x10FFFF;` and `&#1114111;`: a longer run before a `;` is no
/// entity, so the search for one never runs past it.
const LONGEST_ENTITY: usize = 10;

/// The entity `text` starts with: `&lt;`, `&gt;`, `&amp;`, `&quot;`, or a numeric `&#N;` or
/// `&#xH;` naming a character.
fn entity(text: &str) -> Option<Piece<'_>> {
    let end = text
        .bytes()
        .take(LONGEST_ENTITY)
        .position(|byte| byte == b';')?;
    let raw = &text[..=end];
    let body = &raw[1..end];
    let character = match body {
        "lt" => '<',
        "gt" => '>',
        "amp" => '&',
        "quot" => '"',
        _ => {
            let number = body.strip_prefix('#')?;
            let code = match number.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => number.parse().ok()?,
            };
            char::from_u32(code)?
        }
    };
    Some(Piece::Visible {
        raw,
        units: character.len_utf16(),
        character,
    })
}
