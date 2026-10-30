//! The images the engine renders from a note's LaTeX, and the names it gives them (SPEC-513 R9;
//! ADR-531 D8).
//!
//! A note writes LaTeX as `[latex]…[/latex]`, as `[$]…[/$]` (the expression `$…$`) or as
//! `[$$]…[/$$]` (the expression `\begin{displaymath}…\end{displaymath}`). The markers match in any
//! case, and an expression is at least one character, up to the first closing marker after it; at
//! each `[` the three forms are tried in that order, and the search resumes after a match. In the
//! expression, `<br>`, `<br/>` and `<div>` (in any case) become a newline, then every comment, every
//! `style` and `script` element and every other tag is removed, and the entities are decoded as the
//! grammar decodes a name ([`crate::scan::decode_entities`]). The image is named `latex-`, the SHA-1
//! of the expression's UTF-8 in lowercase hex, `.` and the image's extension.
//!
//! What it does not do, each a divergence the decision records: reveal a cloze before reading (the
//! engine's media check reveals a field's clozes first, where its renderer does not), and decode a
//! named entity the grammar's decoder does not know (the engine's decoder knows every HTML name).

use crate::scan::{decode_entities, match_ignoring_case};
use crate::sha1::sha1_hex;

/// The type of image a note type renders its LaTeX to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LatexFormat {
    /// A PNG image, the engine's default.
    #[default]
    Png,
    /// An SVG image, for a note type that asks for one.
    Svg,
}

impl LatexFormat {
    /// The image's extension.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
        }
    }
}

/// One LaTeX expression a text writes, and the name of the image the engine renders it to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatexImage {
    /// The expression the image renders: its markup removed and its entities decoded.
    pub expression: String,
    /// `latex-`, the SHA-1 of the expression in lowercase hex, `.` and the image's extension.
    pub name: String,
}

/// Every LaTeX image `text` writes, in the order written, named for `format`. An expression written
/// twice is listed twice.
#[must_use]
pub fn latex_images(text: &str, format: LatexFormat) -> Vec<LatexImage> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].find('[') {
        let start = from + offset;
        match latex_at(text, start) {
            Some((written, end)) => {
                let expression = strip_markup(&written);
                let name = latex_name(&expression, format);
                found.push(LatexImage { expression, name });
                from = end;
            }
            None => from = start + 1,
        }
    }
    found
}

/// The name of the image `expression` renders to, in `format`.
#[must_use]
pub fn latex_name(expression: &str, format: LatexFormat) -> String {
    format!(
        "latex-{}.{}",
        sha1_hex(expression.as_bytes()),
        format.extension()
    )
}

/// The three forms a note writes LaTeX in, in the order the engine tries them at a `[`: the opening
/// marker, the closing marker, and what the expression puts before and after the text between
/// them.
const FORMS: [(&str, &str, &str, &str); 3] = [
    ("[latex]", "[/latex]", "", ""),
    ("[$]", "[/$]", "$", "$"),
    (
        "[$$]",
        "[/$$]",
        "\\begin{displaymath}",
        "\\end{displaymath}",
    ),
];

/// The tags that end a line of an expression, in any case. The engine's expression for them
/// ignores its own whitespace, so `<br />` is not one: it is removed as any other tag is.
const NEWLINES: [&str; 3] = ["<br/>", "<br>", "<div>"];

/// The elements removed with their content, each an opening up to its first `>` and the first
/// closing tag after it, in the order the engine tries them.
const ELEMENTS: [(&str, &str); 2] = [("<style", "</style>"), ("<script", "</script>")];

/// The expression written at `start` and where its match ends: the first form whose opening marker
/// is there and whose closing marker follows at least one character after it.
fn latex_at(text: &str, start: usize) -> Option<(String, usize)> {
    FORMS.iter().find_map(|&(open, close, before, after)| {
        let body = match_ignoring_case(text, start, open)?;
        let first = text[body..].chars().next()?.len_utf8();
        let closing = find_ignoring_case(text, body + first, close)?;
        let end = match_ignoring_case(text, closing, close)?;
        Some((format!("{before}{}{after}", &text[body..closing]), end))
    })
}

/// The expression as the engine names it: each line-ending tag a newline, then every comment,
/// `style` and `script` element and other tag removed, then its entities decoded.
fn strip_markup(written: &str) -> String {
    let lines = replace_newlines(written);
    let bare = strip_tags(&lines);
    decode_entities(&bare).into_owned()
}

/// `text` with each line-ending tag replaced by a newline.
fn replace_newlines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut from = 0;
    while let Some(offset) = text[from..].find('<') {
        let start = from + offset;
        match NEWLINES
            .iter()
            .find_map(|tag| match_ignoring_case(text, start, tag))
        {
            Some(end) => {
                out.push_str(&text[copied..start]);
                out.push('\n');
                copied = end;
                from = end;
            }
            None => from = start + 1,
        }
    }
    out.push_str(&text[copied..]);
    out
}

/// `text` with every comment, `style` and `script` element and other tag removed.
fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut from = 0;
    while let Some(offset) = text[from..].find('<') {
        let start = from + offset;
        // No match here means no `>` follows, so none can match later either.
        let Some(end) = tag_end(text, start) else {
            break;
        };
        out.push_str(&text[copied..start]);
        copied = end;
        from = end;
    }
    out.push_str(&text[copied..]);
    out
}

/// Where the markup opening at `start` ends, tried in the engine's order: a comment up to its first
/// `-->`, a `style` or `script` element, then any tag up to its first `>`.
fn tag_end(text: &str, start: usize) -> Option<usize> {
    comment_end(text, start)
        .or_else(|| {
            ELEMENTS
                .iter()
                .find_map(|&(open, close)| element_end(text, start, open, close))
        })
        .or_else(|| {
            text[start + 1..]
                .find('>')
                .map(|close| start + 1 + close + 1)
        })
}

/// Where a comment opening at `start` ends: after the first `-->` that follows its `<!--`.
fn comment_end(text: &str, start: usize) -> Option<usize> {
    let body = match_ignoring_case(text, start, "<!--")?;
    text[body..]
        .find("-->")
        .map(|close| body + close + "-->".len())
}

/// Where an element opening at `start` with `open` ends: after the first `close` that follows the
/// opening's first `>`.
fn element_end(text: &str, start: usize, open: &str, close: &str) -> Option<usize> {
    let after = match_ignoring_case(text, start, open)?;
    let opened = after + text[after..].find('>')? + 1;
    let closing = find_ignoring_case(text, opened, close)?;
    match_ignoring_case(text, closing, close)
}

/// The first position at or after `from` where `literal` matches `text` with case ignored.
fn find_ignoring_case(text: &str, from: usize, literal: &str) -> Option<usize> {
    text[from..]
        .char_indices()
        .map(|(offset, _)| from + offset)
        .find(|&at| match_ignoring_case(text, at, literal).is_some())
}
