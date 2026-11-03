//! The media names a note type's CSS writes (SPEC-513 R5; ADR-531 D5).
//!
//! A name is the argument of a `url(`, matched in any ASCII case, up to the first `)`: trimmed,
//! without one pair of matching quotes, its `%XX` escapes decoded, and a plain file name. A URL
//! with a scheme or a path, and a `data:` URL, hold a separator, so they name no file. This is the
//! reading the engine core's font pass gives a face's CSS (`crates/engine-core/src/media.rs`):
//! every font it inlines or empties is a name here, and so is any other plain name, an image's
//! among them, since a sweep must keep whatever the CSS loads.

use crate::name::plain_name;

/// The opening of a CSS `url(` reference, matched in any ASCII case.
const URL_OPEN: &str = "url(";

/// Every media name `css` writes in a `url(`, in the order written. A `url(` with no `)` after it
/// ends the reading.
#[must_use]
pub fn scan_css(css: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = css;
    while let Some(start) = rest
        .as_bytes()
        .windows(URL_OPEN.len())
        .position(|window| window.eq_ignore_ascii_case(URL_OPEN.as_bytes()))
    {
        let open = start + URL_OPEN.len();
        let Some(length) = rest[open..].find(')') else {
            break;
        };
        if let Some(name) = url_name(&rest[open..open + length]) {
            names.push(name);
        }
        rest = &rest[open + length + 1..];
    }
    names
}

/// The file a `url(` argument names: trimmed, unquoted, decoded and plain.
fn url_name(argument: &str) -> Option<String> {
    let name = decoded_name(unquoted(argument.trim()))?;
    plain_name(&name).then_some(name)
}

/// `argument` without one pair of matching quotes around it, when it has them.
fn unquoted(argument: &str) -> &str {
    ['"', '\'']
        .into_iter()
        .find_map(|quote| argument.strip_prefix(quote)?.strip_suffix(quote))
        .unwrap_or(argument)
}

/// `name` with its `%XX` escapes decoded. A `%` not followed by two hex digits stands for itself.
/// `None` when the decoded bytes are not UTF-8: no stored file name can spell them.
#[must_use]
pub fn decoded_name(name: &str) -> Option<String> {
    let mut decoded = Vec::with_capacity(name.len());
    let mut rest = name.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'%'
            && let [high, low, after @ ..] = tail
            && let (Some(high), Some(low)) = (hex(*high), hex(*low))
        {
            decoded.push(high * 16 + low);
            rest = after;
        } else {
            decoded.push(byte);
            rest = tail;
        }
    }
    String::from_utf8(decoded).ok()
}

/// One hex digit's value, in either case.
fn hex(digit: u8) -> Option<u8> {
    char::from(digit)
        .to_digit(16)
        .and_then(|value| u8::try_from(value).ok())
}
