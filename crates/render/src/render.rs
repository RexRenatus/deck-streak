//! Drawing a card from its parsed templates (SPEC-603 R2 to R5, ADR-624 D3): the field map with
//! its special fields, sections and inverted sections, `{{FrontSide}}`, the empty-front rule, and
//! the note-type check.

use std::collections::{HashMap, HashSet};

use crate::filters;
use crate::template::{Node, parse};
use crate::{CardContext, NoteContent, NoteType, NoteTypeKind, RenderIssue, Rendered};

/// The names a template may use beside the note type's own fields.
const SPECIAL: [&str; 8] = [
    "FrontSide",
    "Card",
    "CardFlag",
    "Deck",
    "Subdeck",
    "Tags",
    "Type",
    "CardID",
];

/// The name of the special field that shows the question on the answer side.
const FRONT_SIDE: &str = "FrontSide";

/// Whether `name` is a cloze number's section name: `c` and one or more digits.
fn is_cloze_number(name: &str) -> bool {
    name.strip_prefix('c')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether a field's text shows nothing: only ASCII whitespace and line-break or `div` tags.
fn is_blank(text: &str) -> bool {
    let mut rest = text;
    while let Some(first) = rest.chars().next() {
        if matches!(first, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r') {
            rest = rest.get(1..).unwrap_or_default();
            continue;
        }
        match blank_tag_length(rest) {
            Some(length) => rest = rest.get(length..).unwrap_or_default(),
            None => return false,
        }
    }
    true
}

/// The length of the `br` or `div` tag, opening or closing, that starts `text`, if one does.
fn blank_tag_length(text: &str) -> Option<usize> {
    let after = text.strip_prefix('<')?;
    let after = after.strip_prefix('/').unwrap_or(after);
    let name = ["br", "div"].into_iter().find(|name| {
        after
            .get(..name.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(name))
    })?;
    let after = after.get(name.len()..)?;
    let after = after.strip_prefix(' ').unwrap_or(after);
    let after = after.strip_prefix('/').unwrap_or(after);
    let after = after.strip_prefix('>')?;
    Some(text.len() - after.len())
}

/// Every value one card can show, by name: the note's fields, then each special field the note
/// type does not shadow with a field of its own.
fn field_map(
    note_type: &NoteType,
    note: &NoteContent<'_>,
    card: &CardContext<'_>,
    template: &str,
) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = note_type
        .fields
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let value = note.fields.get(index).cloned().unwrap_or_default();
            (name.clone(), value)
        })
        .collect();
    let subdeck = card.deck_path.rsplit("::").next().unwrap_or_default();
    let special = [
        ("Tags".to_owned(), note.tags.join(" ")),
        ("Type".to_owned(), note_type.name.clone()),
        ("Deck".to_owned(), card.deck_path.to_owned()),
        ("Subdeck".to_owned(), subdeck.to_owned()),
        ("CardFlag".to_owned(), format!("flag{}", card.flag)),
        ("Card".to_owned(), template.to_owned()),
        ("CardID".to_owned(), card.card_id.to_string()),
        (format!("c{}", u32::from(card.ordinal) + 1), "1".to_owned()),
    ];
    for (name, value) in special {
        map.entry(name).or_insert(value);
    }
    map
}

/// What one side is drawn from.
struct Side<'a> {
    /// The values by name.
    map: &'a HashMap<String, String>,
    /// The names whose values show something.
    filled: &'a HashSet<&'a str>,
    /// The drawn question, on the answer side; `None` on the question side.
    question: Option<&'a str>,
}

/// The issue for a name the note type does not have.
fn unknown(name: &str) -> RenderIssue {
    RenderIssue::UnknownField {
        name: name.to_owned(),
    }
}

/// Draws `nodes` onto `out`. A hidden section's nodes are still drawn, onto a throwaway, so a
/// problem inside one is reported all the same.
fn draw(nodes: &[Node], side: &Side<'_>, out: &mut String) -> Result<(), RenderIssue> {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::Comment(inside) => {
                out.push_str("<!--");
                out.push_str(inside);
                out.push_str("-->");
            }
            Node::Var { name, .. } if name == FRONT_SIDE => {
                out.push_str(side.question.unwrap_or_default());
            }
            Node::Var { name, filters } if name.is_empty() && !filters.is_empty() => {}
            Node::Var { name, filters } => {
                let value = side.map.get(name).ok_or_else(|| unknown(name))?;
                out.push_str(&filters::apply(name, value, filters));
            }
            Node::Section {
                inverted,
                name,
                children,
            } => {
                if !side.map.contains_key(name) && !is_cloze_number(name) {
                    return Err(unknown(name));
                }
                if side.filled.contains(name.as_str()) == *inverted {
                    draw(children, side, &mut String::new())?;
                } else {
                    draw(children, side, out)?;
                }
            }
        }
    }
    Ok(())
}

/// Whether a front shows at least one filled field.
fn shows_a_field(nodes: &[Node], filled: &HashSet<&str>) -> bool {
    nodes.iter().any(|node| match node {
        Node::Var { name, .. } => filled.contains(name.as_str()),
        Node::Section {
            inverted,
            name,
            children,
        } => filled.contains(name.as_str()) != *inverted && shows_a_field(children, filled),
        Node::Text(_) | Node::Comment(_) => false,
    })
}

/// Renders one card; see [`crate::render`].
pub(crate) fn card(
    note_type: &NoteType,
    note: &NoteContent<'_>,
    card: &CardContext<'_>,
) -> Result<Rendered, RenderIssue> {
    let index = match note_type.kind {
        NoteTypeKind::Standard => usize::from(card.ordinal),
        NoteTypeKind::Cloze => 0,
    };
    let template = note_type
        .templates
        .get(index)
        .ok_or(RenderIssue::NoTemplate {
            ordinal: card.ordinal,
        })?;
    let front = parse(&template.front)?;
    let back = parse(&template.back)?;
    let map = field_map(note_type, note, card, &template.name);
    let filled: HashSet<&str> = map
        .iter()
        .filter(|(_, value)| !is_blank(value))
        .map(|(name, _)| name.as_str())
        .collect();
    let mut question = String::new();
    draw(
        &front,
        &Side {
            map: &map,
            filled: &filled,
            question: None,
        },
        &mut question,
    )?;
    let mut answer = String::new();
    draw(
        &back,
        &Side {
            map: &map,
            filled: &filled,
            question: Some(&question),
        },
        &mut answer,
    )?;
    if note_type.kind == NoteTypeKind::Standard && !shows_a_field(&front, &filled) {
        return Err(RenderIssue::EmptyFront);
    }
    Ok(Rendered { question, answer })
}

/// Every name `nodes` use: replacement names and section names, cloze numbers left out.
fn names<'a>(nodes: &'a [Node], found: &mut Vec<&'a str>) {
    for node in nodes {
        match node {
            Node::Var { name, .. } => found.push(name),
            Node::Section { name, children, .. } => {
                if !is_cloze_number(name) {
                    found.push(name);
                }
                names(children, found);
            }
            Node::Text(_) | Node::Comment(_) => {}
        }
    }
}

/// Whether `nodes` hold a replacement with a cloze filter.
fn has_cloze(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| match node {
        Node::Var { filters, .. } => filters.iter().any(|filter| filter == "cloze"),
        Node::Section { children, .. } => has_cloze(children),
        Node::Text(_) | Node::Comment(_) => false,
    })
}

/// Whether a front shows the `Card` special field, which lets two fronts read the same.
fn names_the_card(front: &str) -> bool {
    front.match_indices("{{").any(|(at, _)| {
        front
            .get(at + 2..)
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix("Card"))
            .map(str::trim_start)
            .is_some_and(|rest| rest.starts_with("}}"))
    })
}

/// Checks a note type; see [`crate::check_note_type`].
pub(crate) fn check(note_type: &NoteType) -> Result<(), RenderIssue> {
    for (ordinal, template) in note_type.templates.iter().enumerate() {
        let earlier = note_type.templates.get(..ordinal).unwrap_or_default();
        if earlier.iter().any(|other| other.front == template.front)
            && !names_the_card(&template.front)
        {
            return Err(RenderIssue::DuplicateFront { ordinal });
        }
    }
    for (ordinal, template) in note_type.templates.iter().enumerate() {
        let front = parse(&template.front)?;
        let back = parse(&template.back)?;
        let mut used = Vec::new();
        names(&front, &mut used);
        if used.is_empty() {
            return Err(RenderIssue::FrontNamesNoField { ordinal });
        }
        names(&back, &mut used);
        let stranger = used.into_iter().find(|name| {
            !name.is_empty()
                && !SPECIAL.contains(name)
                && !note_type.fields.iter().any(|field| field == name)
        });
        if let Some(name) = stranger {
            return Err(unknown(name));
        }
    }
    if note_type.kind == NoteTypeKind::Cloze {
        let template = note_type.templates.first();
        let front = template.map(|t| parse(&t.front)).transpose()?;
        let back = template.map(|t| parse(&t.back)).transpose()?;
        let holds = |side: Option<Vec<Node>>| side.is_some_and(|nodes| has_cloze(&nodes));
        if !(holds(front) && holds(back)) {
            return Err(RenderIssue::NoClozeFilter);
        }
    }
    Ok(())
}
