//! The template language's tokenizer and parser (SPEC-603 R7, ADR-624 D1).
//!
//! A tag is `{{` up to the first `}}` after it; an HTML comment is `<!--` up to the first `-->`
//! after it, and its inside is never read as tags. An unterminated tag is text. The parser keeps
//! an explicit stack, so a template's nesting costs no call stack.

use std::ops::Range;

use crate::{MAX_DEPTH, MAX_TEMPLATE_BYTES, RenderIssue};

/// One token of a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    /// Text outside any tag.
    Text(&'a str),
    /// The inside of an HTML comment.
    Comment(&'a str),
    /// A replacement tag's inside: filters and a name, joined by colons.
    Var(&'a str),
    /// A section's opening tag.
    Open {
        /// `^`: the section renders when its field is empty.
        inverted: bool,
        /// The section's name.
        name: &'a str,
    },
    /// A section's closing tag, by name.
    Close(&'a str),
}

/// One node of a parsed template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// Text, kept as written.
    Text(String),
    /// An HTML comment's inside, written back around its markers.
    Comment(String),
    /// A field replacement.
    Var {
        /// The field's name.
        name: String,
        /// The filters, in the order they apply (the one nearest the name first).
        filters: Vec<String>,
    },
    /// A section, or an inverted section.
    Section {
        /// `^`: the section renders when its field is empty.
        inverted: bool,
        /// The section's name.
        name: String,
        /// What the section holds.
        children: Vec<Node>,
    },
}

/// Where one tag or comment sits in the text that is left: its start, its inside and its end.
struct Found {
    /// The byte where its opening marker starts.
    start: usize,
    /// The bytes between its markers.
    inside: Range<usize>,
    /// The byte after its closing marker.
    end: usize,
    /// An HTML comment rather than a tag.
    comment: bool,
}

/// The first complete span opened by `open` and closed by `close` in `text`.
fn span(text: &str, open: &str, close: &str, comment: bool) -> Option<Found> {
    let start = text.find(open)?;
    let from = start + open.len();
    let length = text.get(from..)?.find(close)?;
    Some(Found {
        start,
        inside: from..from + length,
        end: from + length + close.len(),
        comment,
    })
}

/// The earliest complete tag or comment in `text`.
fn next_span(text: &str) -> Option<Found> {
    let tag = span(text, "{{", "}}", false);
    let comment = span(text, "<!--", "-->", true);
    match (tag, comment) {
        (Some(tag), Some(comment)) if comment.start < tag.start => Some(comment),
        (tag, comment) => tag.or(comment),
    }
}

/// What a tag's inside opens, closes or replaces.
fn classify(inside: &str) -> Token<'_> {
    let tag = inside.trim_start_matches('{').trim();
    if tag.len() < 2 {
        return Token::Var(tag);
    }
    if let Some(name) = tag.strip_prefix('#') {
        Token::Open {
            inverted: false,
            name: name.trim_start(),
        }
    } else if let Some(name) = tag.strip_prefix('^') {
        Token::Open {
            inverted: true,
            name: name.trim_start(),
        }
    } else if let Some(name) = tag.strip_prefix('/') {
        Token::Close(name.trim_start())
    } else {
        Token::Var(tag)
    }
}

/// Splits `template` into tokens. Total: every input gives a token list.
#[must_use]
pub fn tokenize(template: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut rest = template;
    while let Some(found) = next_span(rest) {
        if let Some(text) = rest.get(..found.start).filter(|text| !text.is_empty()) {
            tokens.push(Token::Text(text));
        }
        let inside = rest.get(found.inside).unwrap_or_default();
        tokens.push(if found.comment {
            Token::Comment(inside)
        } else {
            classify(inside)
        });
        rest = rest.get(found.end..).unwrap_or_default();
    }
    if !rest.is_empty() {
        tokens.push(Token::Text(rest));
    }
    tokens
}

/// A replacement's node: the last colon-separated part names the field, and the parts before it
/// are its filters, the nearest first.
fn replacement(inside: &str) -> Node {
    let mut parts = inside.rsplit(':');
    let name = parts.next().unwrap_or_default().to_owned();
    Node::Var {
        name,
        filters: parts.map(str::to_owned).collect(),
    }
}

/// A section the parser has opened and not yet closed.
struct Open {
    /// `^`: an inverted section.
    inverted: bool,
    /// The section's name.
    name: String,
    /// What it holds so far.
    children: Vec<Node>,
}

/// Parses `template` into a tree of nodes.
///
/// # Errors
///
/// [`RenderIssue::TooLarge`] for a template over [`crate::MAX_TEMPLATE_BYTES`],
/// [`RenderIssue::StrayClose`] and [`RenderIssue::UnclosedSection`] for unbalanced sections, and
/// [`RenderIssue::TooDeep`] past [`crate::MAX_DEPTH`].
pub fn parse(template: &str) -> Result<Vec<Node>, RenderIssue> {
    if template.len() > MAX_TEMPLATE_BYTES {
        return Err(RenderIssue::TooLarge);
    }
    let mut stack: Vec<Open> = Vec::new();
    let mut top = Vec::new();
    for token in tokenize(template) {
        let node = match token {
            Token::Text(text) => Node::Text(text.to_owned()),
            Token::Comment(inside) => Node::Comment(inside.to_owned()),
            Token::Var(inside) => replacement(inside),
            Token::Open { inverted, name } => {
                if stack.len() >= MAX_DEPTH {
                    return Err(RenderIssue::TooDeep);
                }
                stack.push(Open {
                    inverted,
                    name: name.to_owned(),
                    children: Vec::new(),
                });
                continue;
            }
            Token::Close(name) => match stack.pop() {
                Some(open) if open.name == name => Node::Section {
                    inverted: open.inverted,
                    name: open.name,
                    children: open.children,
                },
                _ => {
                    return Err(RenderIssue::StrayClose {
                        name: name.to_owned(),
                    });
                }
            },
        };
        match stack.last_mut() {
            Some(open) => open.children.push(node),
            None => top.push(node),
        }
    }
    match stack.pop() {
        Some(open) => Err(RenderIssue::UnclosedSection { name: open.name }),
        None => Ok(top),
    }
}
