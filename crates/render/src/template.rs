//! The template language's tokenizer and parser (SPEC-603 R7, ADR-624 D1).
//!
//! A tag is `{{` up to the first `}}` after it; an HTML comment is `<!--` up to the first `-->`
//! after it, and its inside is never read as tags. An unterminated tag is text. The parser keeps
//! an explicit stack, so a template's nesting costs no call stack.

use crate::RenderIssue;

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

/// Splits `template` into tokens. Total: every input gives a token list.
#[must_use]
pub fn tokenize(template: &str) -> Vec<Token<'_>> {
    let _ = template;
    Vec::new()
}

/// Parses `template` into a tree of nodes.
///
/// # Errors
///
/// [`RenderIssue::TooLarge`] for a template over [`crate::MAX_TEMPLATE_BYTES`],
/// [`RenderIssue::StrayClose`] and [`RenderIssue::UnclosedSection`] for unbalanced sections, and
/// [`RenderIssue::TooDeep`] past [`crate::MAX_DEPTH`].
pub fn parse(template: &str) -> Result<Vec<Node>, RenderIssue> {
    let _ = template;
    Ok(Vec::new())
}
