//! The coverage gates (SPEC-046 R6): the anchors, the roster, completeness, list markers, and the
//! order the first failure decides in.
//!
//! The anchor is the predecessor's own definition, ported and proved against goldens generated
//! from its functions (`preread.py:anchor_for_note` and `preread.py:is_anchor_usable` at
//! `27ee2bc`): entities decoded, markup stripped, NFKC, the rail's characters removed, whitespace
//! collapsed, and the first 48 characters kept.

use std::collections::BTreeSet;

use unicode_normalization::UnicodeNormalization;

use crate::form::Form;
use crate::seed::{Seed, Track};
use crate::state::ReadingGate;

/// The longest an anchor is.
pub const ANCHOR_MAX_CHARS: usize = 48;
/// The shortest a usable anchor is.
pub const ANCHOR_MIN_USABLE_CHARS: usize = 8;

/// The character-reference names HTML decodes without a closing semicolon.
const LEGACY_ENTITIES: &[&str] = &[
    "AElig", "AMP", "Aacute", "Acirc", "Agrave", "Aring", "Atilde", "Auml", "COPY", "Ccedil",
    "ETH", "Eacute", "Ecirc", "Egrave", "Euml", "GT", "Iacute", "Icirc", "Igrave", "Iuml", "LT",
    "Ntilde", "Oacute", "Ocirc", "Ograve", "Oslash", "Otilde", "Ouml", "QUOT", "REG", "THORN",
    "Uacute", "Ucirc", "Ugrave", "Uuml", "Yacute", "aacute", "acirc", "acute", "aelig", "agrave",
    "amp", "aring", "atilde", "auml", "brvbar", "ccedil", "cedil", "cent", "copy", "curren", "deg",
    "divide", "eacute", "ecirc", "egrave", "eth", "euml", "frac12", "frac14", "frac34", "gt",
    "iacute", "icirc", "iexcl", "igrave", "iquest", "iuml", "laquo", "lt", "macr", "micro",
    "middot", "nbsp", "not", "ntilde", "oacute", "ocirc", "ograve", "ordf", "ordm", "oslash",
    "otilde", "ouml", "para", "plusmn", "pound", "quot", "raquo", "reg", "sect", "shy", "sup1",
    "sup2", "sup3", "szlig", "thorn", "times", "uacute", "ucirc", "ugrave", "uml", "uuml",
    "yacute", "yen", "yuml",
];

/// The longest legacy name is the lookup's ceiling.
const LEGACY_LONGEST: usize = 6;

/// Adds the semicolons the `html-escape` crate needs to decode what HTML decodes without one:
/// a numeric reference, and a legacy name (the longest prefix that is one).
fn close_references(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 8);
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        out.push(c);
        at += 1;
        if c != '&' {
            continue;
        }
        if chars.get(at) == Some(&'#') {
            let mut end = at + 1;
            let hex = matches!(chars.get(end), Some('x' | 'X'));
            if hex {
                end += 1;
            }
            let start = end;
            while chars.get(end).is_some_and(|d| {
                if hex {
                    d.is_ascii_hexdigit()
                } else {
                    d.is_ascii_digit()
                }
            }) {
                end += 1;
            }
            if end > start {
                out.extend(&chars[at..end]);
                if chars.get(end) != Some(&';') {
                    out.push(';');
                }
                at = end;
            }
            continue;
        }
        let mut end = at;
        while chars.get(end).is_some_and(char::is_ascii_alphanumeric) {
            end += 1;
        }
        let run: String = chars[at..end].iter().collect();
        let terminated = chars.get(end) == Some(&';');
        if terminated
            && html_escape::decode_html_entities(&format!("&{run};")) != format!("&{run};")
        {
            continue;
        }
        let longest = run.len().min(LEGACY_LONGEST);
        let prefix = (2..=longest)
            .rev()
            .find(|length| LEGACY_ENTITIES.contains(&&run[..*length]));
        if let Some(length) = prefix {
            out.push_str(&run[..length]);
            out.push(';');
            out.push_str(&run[length..]);
            at = end;
        }
    }
    out
}

/// `text` with every `<...>` replaced by a space; a `<` with no `>` after it stays.
fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        if let Some(close) = rest[open..].find('>') {
            out.push(' ');
            rest = &rest[open + close + 1..];
        } else {
            out.push('<');
            rest = &rest[open + 1..];
        }
    }
    out.push_str(rest);
    out
}

/// `text` with the rail's characters, a run of three tildes or more, and `javascript:` replaced by
/// a space.
fn remove_rail_characters(text: &str) -> String {
    const SCHEME: &str = "javascript:";
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        if matches!(c, '\0' | '`' | '<' | '[' | ']') {
            out.push(' ');
            at += 1;
        } else if c == '~' {
            let mut end = at;
            while chars.get(end) == Some(&'~') {
                end += 1;
            }
            if end - at >= 3 {
                out.push(' ');
            } else {
                out.extend(&chars[at..end]);
            }
            at = end;
        } else if chars.len() - at >= SCHEME.len()
            && chars[at..at + SCHEME.len()]
                .iter()
                .zip(SCHEME.chars())
                .all(|(have, want)| have.eq_ignore_ascii_case(&want))
        {
            out.push(' ');
            at += SCHEME.len();
        } else {
            out.push(c);
            at += 1;
        }
    }
    out
}

/// Whether Python's `str.split()` splits on `c`.
fn is_split_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `text` with runs of whitespace collapsed to one space and the ends trimmed.
fn collapse(text: &str) -> String {
    text.split(is_split_space)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A note's text with entities decoded, markup removed and the rail's characters dropped.
#[must_use]
pub fn normalise(text: &str) -> String {
    let decoded = html_escape::decode_html_entities(&close_references(text)).into_owned();
    let stripped = strip_tags(&decoded);
    let composed: String = stripped.nfkc().collect();
    collapse(&remove_rail_characters(&composed))
}

/// The anchor of a note: its normalised text, cut to [`ANCHOR_MAX_CHARS`] characters.
#[must_use]
pub fn anchor_for_note(text: &str) -> String {
    normalise(text).chars().take(ANCHOR_MAX_CHARS).collect()
}

/// Whether an anchor is long enough to verify against.
#[must_use]
pub fn is_anchor_usable(anchor: &str) -> bool {
    anchor.chars().count() >= ANCHOR_MIN_USABLE_CHARS
}

/// A reading split into its frontmatter and its sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    /// The text between the frontmatter fences.
    pub frontmatter: String,
    /// The text after the frontmatter.
    pub body: String,
    /// Each section's name and text, in order.
    pub sections: Vec<(String, String)>,
}

/// The opening of a section marker inside a heading.
const SECTION_OPEN: &str = "<!-- section:";
/// The close of a section marker.
const SECTION_CLOSE: &str = "-->";

/// The section a heading line opens, if it does.
fn section_of(line: &str) -> Option<String> {
    if !line.starts_with('#') {
        return None;
    }
    let after = &line[line.find(SECTION_OPEN)? + SECTION_OPEN.len()..];
    let name = after[..after.find(SECTION_CLOSE)?].trim();
    Some(name.to_owned())
}

impl Document {
    /// Splits `text`.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut frontmatter = String::new();
        let mut body = text;
        if let Some(rest) = text.strip_prefix("---\n")
            && let Some(close) = rest.find("\n---")
        {
            rest[..close].clone_into(&mut frontmatter);
            let after = &rest[close + "\n---".len()..];
            body = after.strip_prefix('\n').unwrap_or(after);
        }
        let mut sections: Vec<(String, String)> = Vec::new();
        for line in body.lines() {
            if let Some(name) = section_of(line) {
                sections.push((name, String::new()));
            } else if let Some((_, section)) = sections.last_mut() {
                section.push_str(line);
                section.push('\n');
            }
        }
        Self {
            frontmatter,
            body: body.to_owned(),
            sections,
        }
    }

    /// The text of the section `name`, if it is there.
    #[must_use]
    pub fn section(&self, name: &str) -> Option<&str> {
        self.sections
            .iter()
            .find(|(have, _)| have == name)
            .map(|(_, text)| text.as_str())
    }
}

/// The findings of the engine's own gates; an empty list passes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OwnChecks {
    /// Completeness.
    pub complete: Vec<String>,
    /// The roster.
    pub roster: Vec<String>,
    /// The anchors.
    pub anchors: Vec<String>,
    /// List markers.
    pub no_list_markers: Vec<String>,
    /// The advisory of unusable anchors, when some were excluded.
    pub advisory: Option<String>,
}

/// The citation keys a text holds, as `[@key]`.
fn citations(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find("[@") {
        let after = &rest[open + 2..];
        match after.find(']') {
            Some(close) => {
                found.insert(after[..close].trim().to_owned());
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    found
}

/// Whether a line begins a bullet or a numbered item.
fn begins_list_marker(line: &str) -> bool {
    let line = line.trim_start_matches([' ', '\t']);
    let mut chars = line.chars();
    match chars.next() {
        Some('-' | '*' | '+') => matches!(chars.next(), Some(' ' | '\t')),
        Some(first) if first.is_ascii_digit() => {
            let rest = line.trim_start_matches(|c: char| c.is_ascii_digit());
            let mut rest = rest.chars();
            matches!(rest.next(), Some('.' | ')')) && matches!(rest.next(), Some(' ' | '\t'))
        }
        _ => false,
    }
}

/// Runs the engine's own gates over `document`.
#[must_use]
pub fn check_own(form: &Form, seed: &Seed, document: &Document) -> OwnChecks {
    let mut checks = OwnChecks::default();
    for name in form.sections() {
        match document.section(name) {
            None => checks
                .complete
                .push(format!("the {name} section is missing")),
            Some(text) if text.trim().is_empty() => {
                checks.complete.push(format!("the {name} section is empty"));
            }
            Some(_) => {}
        }
    }
    if checks.complete.is_empty() {
        let have: Vec<&str> = document.sections.iter().map(|(n, _)| n.as_str()).collect();
        if have != form.sections() {
            checks
                .complete
                .push("the sections are not in the form's order".to_owned());
        }
    }
    match form.track() {
        Track::Law => check_law(seed, document, &mut checks),
        Track::Language => check_language(seed, document, &mut checks),
    }
    for name in form.list_checked() {
        let text = document.section(name).unwrap_or_default();
        for (index, line) in text.lines().enumerate() {
            if begins_list_marker(line) {
                checks.no_list_markers.push(format!(
                    "line {} of the {name} section begins with a list marker",
                    index + 1
                ));
            }
        }
    }
    checks
}

/// The law roster and anchors.
fn check_law(seed: &Seed, document: &Document, checks: &mut OwnChecks) {
    let cited = citations(&document.body);
    let expected: BTreeSet<String> = seed.source_keys().into_iter().collect();
    for key in expected.difference(&cited) {
        checks.roster.push(format!("note {key} is not cited"));
    }
    let outside = cited.difference(&expected).count();
    if outside > 0 {
        checks
            .roster
            .push(format!("{outside} citation(s) name no note of the seed"));
    }
    let prose: String = document
        .sections
        .iter()
        .filter(|(name, _)| name != "retrieval")
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let haystack = normalise(&prose).to_lowercase();
    let mut unusable = 0_usize;
    for note in &seed.notes {
        let anchor = anchor_for_note(&note.text);
        if !is_anchor_usable(&anchor) {
            unusable += 1;
        } else if !haystack.contains(&anchor.to_lowercase()) {
            checks.anchors.push(format!(
                "the anchor of note {} is not in the prose",
                Seed::key_of(note.id)
            ));
        }
    }
    if unusable > 0 {
        checks.advisory = Some(format!(
            "anchors_partially_unverifiable:{unusable}/{}",
            seed.notes.len()
        ));
    }
}

/// The language roster: each new word glossed and used.
fn check_language(seed: &Seed, document: &Document, checks: &mut OwnChecks) {
    let glosses = document
        .section("glosses")
        .unwrap_or_default()
        .to_lowercase();
    let reading = document
        .section("reading")
        .unwrap_or_default()
        .to_lowercase();
    for (index, word) in seed.new_words.iter().enumerate() {
        let word = word.to_lowercase();
        if !glosses.contains(&word) {
            checks
                .roster
                .push(format!("new word {} is not glossed", index + 1));
        }
        if !reading.contains(&word) {
            checks
                .roster
                .push(format!("new word {} is not used in the reading", index + 1));
        }
    }
}

/// What the pack gate refused: its class and its finding lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackFailure {
    /// The failing class.
    pub class: String,
    /// Its finding lines.
    pub findings: Vec<String>,
}

/// The pack class that judges the word band.
pub const BAND_CLASS: &str = "reading-length";
/// The pack classes that judge the roster.
pub const ROSTER_CLASSES: [&str; 2] = ["citations-resolve", "i1-glosses"];

/// The gate that failed first and its findings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateFailure {
    /// The gate.
    pub gate: ReadingGate,
    /// Its finding lines.
    pub findings: Vec<String>,
}

/// The first failure across the own gates and the pack gate, in the gate order: complete, roster,
/// anchors, band, no list markers, contract.
#[must_use]
pub fn first_failure(own: &OwnChecks, pack: Option<&PackFailure>) -> Option<GateFailure> {
    let failure = |gate, findings: &[String]| {
        Some(GateFailure {
            gate,
            findings: findings.to_vec(),
        })
    };
    let pack_of = |classes: &[&str]| pack.filter(|p| classes.contains(&p.class.as_str()));
    if !own.complete.is_empty() {
        return failure(ReadingGate::Complete, &own.complete);
    }
    if !own.roster.is_empty() {
        return failure(ReadingGate::Roster, &own.roster);
    }
    if let Some(p) = pack_of(&ROSTER_CLASSES) {
        return failure(ReadingGate::Roster, &p.findings);
    }
    if !own.anchors.is_empty() {
        return failure(ReadingGate::Anchors, &own.anchors);
    }
    if let Some(p) = pack_of(&[BAND_CLASS]) {
        return failure(ReadingGate::Band, &p.findings);
    }
    if !own.no_list_markers.is_empty() {
        return failure(ReadingGate::NoListMarkers, &own.no_list_markers);
    }
    pack.map(|p| GateFailure {
        gate: ReadingGate::Contract,
        findings: p.findings.clone(),
    })
}
