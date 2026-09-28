//! The content rails (SPEC-042 R3): every byte the adapter writes into the vault passes the rails
//! of the vault-duties pack's `no-executable` class, read from the crate's own `data/rails.json`
//! (ADR-069), plus
//! one rail of the adapter's own that refuses NUL and every other control character but the tab,
//! the line feed and the carriage return.
//!
//! The rails are data: a new rail is a row in `rails.json`, never a code branch here. Each refusal
//! names its rail row and the line it stands on, and never echoes the text; nothing is sanitised.
//! The scan reads a note as the pack's probe does, so the two agree on every planted fixture
//! (SPEC-042 A2 and A3); where this port differs from the probe it is only ever stricter (ADR-042).

use std::collections::BTreeMap;
use std::fmt;

use deck_streak_kernel::Verdict;
use serde_json::Value;

use crate::note::{is_python_space, python_strip};

/// The rails the adapter enforces: the crate's own `data/rails.json`, which keeps the fields of the
/// vault-duties pack's rails that the port reads. The box run compares it with the pack's own, so
/// the two cannot drift apart (ADR-069).
pub const VENDORED: &str = include_str!("../data/rails.json");

/// The schema `rails.json` carries.
pub const SCHEMA: &str = "phx.duty.vault.rails.v1";

/// Every key the owned `rails.json` holds: its schema and the ten the `no-executable` class reads.
/// A key the pack's own rails gain is a new kind of rail, which needs code before the port can
/// honour it; the box run's drift check names it (ADR-042, ADR-069).
pub const KNOWN_KEYS: [&str; 11] = [
    "schema",
    "fence_allow",
    "fence_known",
    "fence_prefixes",
    "templater_open",
    "inline_query_prefixes",
    "html_allow",
    "html_attributes_allow",
    "executable_schemes",
    "math_macros_refused",
    "dynamic_embed_extensions",
];

/// The adapter's own rail: a control character other than a tab, a line feed or a carriage return.
pub const CONTROL_CHARACTER: &str = "control_character";

/// Why `rails.json` could not be read as rails.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RailsError {
    /// The text is not JSON.
    #[error("rails.json is not JSON")]
    NotJson,
    /// The document carries another schema.
    #[error("rails.json does not carry the schema {SCHEMA}")]
    Schema,
    /// A key the port reads is missing or has the wrong shape.
    #[error("rails.json's {0} is missing or has the wrong shape")]
    Key(&'static str),
}

/// One rail row: a refusal the rails can make, named by the `rails.json` key it comes from and,
/// where that key holds a list or a map, by its entry: `fence_known:dataviewjs`, `templater_open`,
/// `executable_schemes:javascript`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailRow(String);

impl RailRow {
    /// The row of `key` alone.
    fn of(key: &str) -> Self {
        Self(key.to_owned())
    }

    /// The row of `entry` under `key`.
    fn entry(key: &str, entry: &str) -> Self {
        Self(format!("{key}:{entry}"))
    }

    /// The row's name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RailRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A refused text: the rail row that refused it, and the 1-based line it refused. It never holds
/// the text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
#[error("the rail {row} refuses line {line}")]
pub struct RailRefusal {
    /// The rail row.
    pub row: RailRow,
    /// The line, counted from 1.
    pub line: usize,
}

/// The rails, read from `rails.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rails {
    fence_allow: Vec<String>,
    fence_known: BTreeMap<String, String>,
    fence_prefixes: BTreeMap<String, String>,
    templater_open: String,
    inline_query_prefixes: Vec<String>,
    html_allow: Vec<String>,
    html_attributes_allow: Vec<String>,
    executable_schemes: Vec<String>,
    math_macros_refused: Vec<String>,
    dynamic_embed_extensions: Vec<String>,
}

impl Rails {
    /// The rails of the crate's own `data/rails.json`.
    ///
    /// # Errors
    ///
    /// [`RailsError`] when the owned file is not the rails' shape, which a build of the adapter
    /// would never ship: a test reads it.
    pub fn vendored() -> Result<Self, RailsError> {
        Self::from_json(VENDORED)
    }

    /// The rails of a `rails.json` text.
    ///
    /// # Errors
    ///
    /// [`RailsError::NotJson`], [`RailsError::Schema`], or [`RailsError::Key`] naming a key the
    /// port reads that is missing or of the wrong shape.
    pub fn from_json(text: &str) -> Result<Self, RailsError> {
        let document: Value = serde_json::from_str(text).map_err(|_| RailsError::NotJson)?;
        if document.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
            return Err(RailsError::Schema);
        }
        Ok(Self {
            fence_allow: strings(&document, "fence_allow")?,
            fence_known: string_map(&document, "fence_known")?,
            fence_prefixes: string_map(&document, "fence_prefixes")?,
            templater_open: string(&document, "templater_open")?,
            inline_query_prefixes: strings(&document, "inline_query_prefixes")?,
            html_allow: strings(&document, "html_allow")?,
            html_attributes_allow: strings(&document, "html_attributes_allow")?,
            executable_schemes: strings(&document, "executable_schemes")?,
            math_macros_refused: strings(&document, "math_macros_refused")?,
            dynamic_embed_extensions: strings(&document, "dynamic_embed_extensions")?,
        })
    }

    /// Every rail row the data declares: one for each known fence, fence prefix, inline query
    /// prefix, executable scheme, refused macro and dynamic embed extension, and one each for a
    /// fence off the allow-list, Templater's opener, an HTML tag off the allow-list and an HTML
    /// attribute off it. The adapter's own control-character rail is not a row of `rails.json`.
    #[must_use]
    pub fn rows(&self) -> Vec<RailRow> {
        let mut rows = Vec::new();
        rows.extend(
            self.fence_known
                .keys()
                .map(|fence| RailRow::entry("fence_known", fence)),
        );
        rows.extend(
            self.fence_prefixes
                .keys()
                .map(|prefix| RailRow::entry("fence_prefixes", prefix)),
        );
        rows.push(RailRow::of("fence_allow"));
        rows.push(RailRow::of("templater_open"));
        rows.extend(
            self.inline_query_prefixes
                .iter()
                .map(|prefix| RailRow::entry("inline_query_prefixes", prefix)),
        );
        rows.push(RailRow::of("html_allow"));
        rows.push(RailRow::of("html_attributes_allow"));
        rows.extend(
            self.executable_schemes
                .iter()
                .map(|scheme| RailRow::entry("executable_schemes", scheme)),
        );
        rows.extend(
            self.math_macros_refused
                .iter()
                .map(|name| RailRow::entry("math_macros_refused", name)),
        );
        rows.extend(
            self.dynamic_embed_extensions
                .iter()
                .map(|extension| RailRow::entry("dynamic_embed_extensions", extension)),
        );
        rows.sort();
        rows
    }

    /// Every refusal the rails make of `text`, sorted by line and then by row.
    #[must_use]
    pub fn refusals(&self, text: &str) -> Vec<RailRefusal> {
        let mut found = Vec::new();
        let mut refuse = |row: RailRow, line: usize| found.push(RailRefusal { row, line });
        // Lines as the probe reads the whole text: split at `\n`, every character kept.
        let raw: Vec<&str> = text.split('\n').collect();
        for (index, line) in raw.iter().enumerate() {
            if line.contains(self.templater_open.as_str()) {
                refuse(RailRow::of("templater_open"), index + 1);
            }
            if fold(line).contains("obsidian://") {
                refuse(RailRow::entry("executable_schemes", "obsidian"), index + 1);
            }
            if line
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
            {
                refuse(RailRow::of(CONTROL_CHARACTER), index + 1);
            }
        }
        // The note as the probe parses it: one trailing `\r` off each line, code fenced or spanned.
        let lines: Vec<&str> = raw
            .iter()
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
            .collect();
        let code = mask_code(&lines);
        for (line, info) in &code.fences {
            if let Some(row) = self.fence_row(info) {
                refuse(row, *line);
            }
        }
        let mut prefixes: Vec<&String> = self.inline_query_prefixes.iter().collect();
        prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
        for (line, span) in &code.spans {
            let stripped = python_strip(span);
            if let Some(prefix) = prefixes
                .iter()
                .find(|prefix| stripped.starts_with(prefix.as_str()))
            {
                refuse(RailRow::entry("inline_query_prefixes", prefix), *line);
            }
        }
        let prose = blank_between(
            &blank_between(&code.lines.join("\n"), "<!--", "-->"),
            "%%",
            "%%",
        );
        for (index, line) in prose.split('\n').enumerate() {
            let number = index + 1;
            for tag in html_tags(line) {
                if !self.html_allow.contains(&fold(&tag.name)) {
                    refuse(RailRow::of("html_allow"), number);
                }
                if attribute_names(&tag.attrs)
                    .iter()
                    .any(|name| !self.html_attributes_allow.contains(&fold(name)))
                {
                    refuse(RailRow::of("html_attributes_allow"), number);
                }
            }
            for name in &self.math_macros_refused {
                if has_macro(line, name) {
                    refuse(RailRow::entry("math_macros_refused", name), number);
                }
            }
            for link in links(line) {
                if self.executable_schemes.contains(&link.scheme) {
                    refuse(RailRow::entry("executable_schemes", &link.scheme), number);
                }
                let extension = fold(extension(&link.target));
                if link.embed && self.dynamic_embed_extensions.contains(&extension) {
                    refuse(
                        RailRow::entry("dynamic_embed_extensions", &extension),
                        number,
                    );
                }
            }
        }
        found.sort_by(|a, b| (a.line, &a.row).cmp(&(b.line, &b.row)));
        found.dedup();
        found
    }

    /// The row that refuses a fence whose info string is `info`, or `None` when its language is on
    /// the allow-list; the language is the info's first word, case-folded.
    fn fence_row(&self, info: &str) -> Option<RailRow> {
        let language = info
            .split(is_python_space)
            .find(|word| !word.is_empty())
            .map(fold)
            .unwrap_or_default();
        if self.fence_allow.contains(&language) {
            return None;
        }
        if self.fence_known.contains_key(&language) {
            return Some(RailRow::entry("fence_known", &language));
        }
        if let Some(prefix) = self
            .fence_prefixes
            .keys()
            .find(|prefix| language.starts_with(prefix.as_str()))
        {
            return Some(RailRow::entry("fence_prefixes", prefix));
        }
        Some(RailRow::of("fence_allow"))
    }

    /// Whether `text` passes the rails: [`Verdict::Refuse`] names the first refusal by line.
    pub fn check(&self, text: &str) -> Verdict<RailRefusal> {
        match self.refusals(text).into_iter().next() {
            Some(refusal) => Verdict::Refuse(refusal),
            None => Verdict::Pass,
        }
    }
}

/// The list of strings at `key`.
fn strings(document: &Value, key: &'static str) -> Result<Vec<String>, RailsError> {
    document
        .get(key)
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or(RailsError::Key(key))
}

/// The map of strings at `key`.
fn string_map(document: &Value, key: &'static str) -> Result<BTreeMap<String, String>, RailsError> {
    document
        .get(key)
        .and_then(Value::as_object)
        .and_then(|items| {
            items
                .iter()
                .map(|(name, what)| what.as_str().map(|what| (name.clone(), what.to_owned())))
                .collect::<Option<BTreeMap<_, _>>>()
        })
        .ok_or(RailsError::Key(key))
}

/// The string at `key`.
fn string(document: &Value, key: &'static str) -> Result<String, RailsError> {
    document
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(RailsError::Key(key))
}

/// Python's `str.casefold`, as far as a rail compares with it: Unicode lowercase, and the
/// characters whose case folding spells ASCII letters (the long s, the sharp s and the Latin
/// ligatures), which lowercase alone keeps as they are.
fn fold(text: &str) -> String {
    const FOLDS: [(u32, &str); 9] = [
        (0x017f, "s"),
        (0x00df, "ss"),
        (0xfb00, "ff"),
        (0xfb01, "fi"),
        (0xfb02, "fl"),
        (0xfb03, "ffi"),
        (0xfb04, "ffl"),
        (0xfb05, "st"),
        (0xfb06, "st"),
    ];
    let mut folded = String::with_capacity(text.len());
    for c in text.chars().flat_map(char::to_lowercase) {
        match FOLDS.iter().find(|(code, _)| *code == u32::from(c)) {
            Some((_, ascii)) => folded.push_str(ascii),
            None => folded.push(c),
        }
    }
    folded
}

/// The code a note holds, as the probe reads it: each line with its fenced code and its code spans
/// blanked, each fence's info string and each code span's text, with their 1-based lines.
struct Code {
    lines: Vec<String>,
    fences: Vec<(usize, String)>,
    spans: Vec<(usize, String)>,
}

/// The code of `lines`, fences first: a fence runs to its closing line or the end of the note.
fn mask_code(lines: &[&str]) -> Code {
    let mut code = Code {
        lines: Vec::with_capacity(lines.len()),
        fences: Vec::new(),
        spans: Vec::new(),
    };
    let mut open: Option<(char, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        if let Some((fence, length)) = open {
            if closes(line, fence, length) {
                open = None;
            }
            code.lines.push(String::new());
            continue;
        }
        if let Some((fence, length, info)) = fence_opening(line) {
            // A backtick fence whose info holds a backtick is not a fence (CommonMark).
            if !(fence == '`' && info.contains('`')) {
                open = Some((fence, length));
                code.fences.push((index + 1, python_strip(info).to_owned()));
                code.lines.push(String::new());
                continue;
            }
        }
        let (masked, spans) = code_spans(line);
        code.spans
            .extend(spans.into_iter().map(|span| (index + 1, span)));
        code.lines.push(masked);
    }
    code
}

/// A fence's opening line: up to three spaces, then three or more backticks or tildes; its fence
/// character, its length and the info string after it.
fn fence_opening(line: &str) -> Option<(char, usize, &str)> {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let fence = rest.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let length = rest.chars().take_while(|c| *c == fence).count();
    (length >= 3).then(|| (fence, length, &rest[length..]))
}

/// Whether `line` closes a fence of `length` or more `fence` characters.
fn closes(line: &str, fence: char, length: usize) -> bool {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return false;
    }
    let rest = &line[indent..];
    let run = rest.chars().take_while(|c| *c == fence).count();
    run >= length && rest[run..].chars().all(|c| c == ' ' || c == '\t')
}

/// `line` with each code span blanked, and each span's code, left to right.
fn code_spans(line: &str) -> (String, Vec<String>) {
    let chars: Vec<char> = line.chars().collect();
    let mut masked = chars.clone();
    let mut spans = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        if chars[at] != '`' {
            at += 1;
            continue;
        }
        match span_at(&chars, at) {
            Some((ticks, end)) => {
                spans.push(chars[at + ticks..end].iter().collect());
                masked[at..end + ticks].fill(' ');
                at = end + ticks;
            }
            None => at += 1,
        }
    }
    (masked.into_iter().collect(), spans)
}

/// The code span the probe's `` (`+)(.+?)(?<!`)\1(?!`) `` matches at `start`, as its tick count and
/// the index its code ends at: the longest run of backticks first, then the fewest characters (at
/// least one) after which the same run closes, not preceded or followed by another backtick.
fn span_at(chars: &[char], start: usize) -> Option<(usize, usize)> {
    let run = chars[start..].iter().take_while(|c| **c == '`').count();
    (1..=run).rev().find_map(|ticks| {
        (start + ticks + 1..=chars.len().saturating_sub(ticks)).find_map(|end| {
            let closed = chars[end - 1] != '`'
                && chars[end..end + ticks].iter().all(|c| *c == '`')
                && chars.get(end + ticks) != Some(&'`');
            closed.then_some((ticks, end))
        })
    })
}

/// `text` with every span from `open` to the nearest `close` after it blanked to spaces, its line
/// breaks kept: the probe's `<!--.*?-->` and `%%.*?%%`, whose `.` crosses lines.
fn blank_between(text: &str, open: &str, close: &str) -> String {
    let mut blanked = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(open) {
        let after = at + open.len();
        let Some(end) = rest[after..].find(close) else {
            break;
        };
        let stop = after + end + close.len();
        blanked.push_str(&rest[..at]);
        blanked.extend(
            rest[at..stop]
                .chars()
                .map(|c| if c == '\n' { '\n' } else { ' ' }),
        );
        rest = &rest[stop..];
    }
    blanked.push_str(rest);
    blanked
}

/// An HTML tag as the probe reads it: its name and its attribute text.
struct Tag {
    name: String,
    attrs: String,
}

/// Every HTML tag of `line`, left to right.
fn html_tags(line: &str) -> Vec<Tag> {
    let chars: Vec<char> = line.chars().collect();
    let mut tags = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        match tag_at(&chars, at) {
            Some((tag, end)) => {
                tags.push(tag);
                at = end;
            }
            None => at += 1,
        }
    }
    tags
}

/// The tag `<(/?)([A-Za-z][A-Za-z0-9-]*)((?:\s[^<>]*?)?)\s*/?>` at `start`, and the index after it.
fn tag_at(chars: &[char], start: usize) -> Option<(Tag, usize)> {
    if chars[start] != '<' {
        return None;
    }
    let mut at = start + 1;
    if chars.get(at) == Some(&'/') {
        at += 1;
    }
    if !chars.get(at).is_some_and(char::is_ascii_alphabetic) {
        return None;
    }
    let name_start = at;
    at += 1;
    while chars
        .get(at)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '-')
    {
        at += 1;
    }
    let name_end = at;
    let close = (name_end..chars.len()).find(|&index| matches!(chars[index], '<' | '>'))?;
    if chars[close] == '<' {
        return None;
    }
    let rest = &chars[name_end..close];
    let attrs = if rest.first().is_some_and(|c| is_python_space(*c)) {
        // The lazy group: from one space, the fewest characters that leave only `\s*/?`.
        let split = (1..=rest.len()).find(|&index| only_trailer(&rest[index..]))?;
        rest[..split].iter().collect()
    } else if only_trailer(rest) {
        String::new()
    } else {
        return None;
    };
    let name = chars[name_start..name_end].iter().collect();
    Some((Tag { name, attrs }, close + 1))
}

/// Whether `chars` is `\s*/?` and nothing else.
fn only_trailer(chars: &[char]) -> bool {
    let spaces = chars.iter().take_while(|c| is_python_space(**c)).count();
    matches!(&chars[spaces..], [] | ['/'])
}

/// The attribute names in `attrs`, as the probe's `HTML_ATTRIBUTE` finds them left to right.
fn attribute_names(attrs: &str) -> Vec<String> {
    let chars: Vec<char> = attrs.chars().collect();
    let mut names = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        if !(c.is_ascii_alphabetic() || c == '_' || c == ':') {
            at += 1;
            continue;
        }
        let mut end = at + 1;
        while chars
            .get(end)
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'))
        {
            end += 1;
        }
        names.push(chars[at..end].iter().collect());
        at = end + attribute_value(&chars[end..]);
    }
    names
}

/// How many characters of `rest` an attribute's optional `\s*=\s*` and value take: none when they
/// do not match. A value is double-quoted, single-quoted, or a run of characters that are not
/// whitespace, a quote, `=`, `<`, `>` or a backtick.
fn attribute_value(rest: &[char]) -> usize {
    let before = rest.iter().take_while(|c| is_python_space(**c)).count();
    if rest.get(before) != Some(&'=') {
        return 0;
    }
    let start = before
        + 1
        + rest[before + 1..]
            .iter()
            .take_while(|c| is_python_space(**c))
            .count();
    let length = match rest.get(start) {
        Some(quote @ ('"' | '\'')) => rest[start + 1..]
            .iter()
            .position(|c| c == quote)
            .map(|close| close + 2),
        Some(_) => {
            let length = rest[start..]
                .iter()
                .take_while(|c| {
                    !is_python_space(**c) && !matches!(c, '"' | '\'' | '=' | '<' | '>' | '`')
                })
                .count();
            (length > 0).then_some(length)
        }
        None => None,
    };
    length.map_or(0, |length| start + length)
}

/// Whether `line` holds the macro `\name` followed, after any whitespace, by `{`.
fn has_macro(line: &str, name: &str) -> bool {
    let needle = format!("\\{name}");
    line.match_indices(needle.as_str()).any(|(at, _)| {
        line[at + needle.len()..]
            .trim_start_matches(is_python_space)
            .starts_with('{')
    })
}

/// A link as the probe reads it: whether it embeds, its target, and its scheme (case-folded, or
/// empty for a wikilink or a path).
struct Link {
    embed: bool,
    target: String,
    scheme: String,
}

/// Every wikilink, markdown link and autolink of `line`, each form found on its own.
fn links(line: &str) -> Vec<Link> {
    let chars: Vec<char> = line.chars().collect();
    let mut all = Vec::new();
    for find in [wikilink_at, markdown_link_at, autolink_at] {
        let mut at = 0;
        while at < chars.len() {
            match find(&chars, at) {
                Some((hit, end)) => {
                    all.push(hit);
                    at = end;
                }
                None => at += 1,
            }
        }
    }
    all
}

/// The wikilink `(!?)\[\[([^\[\]\n]*?)\]\]` at `start`, and the index after it. Its target is the
/// text before any `|` and `#`, stripped.
fn wikilink_at(chars: &[char], start: usize) -> Option<(Link, usize)> {
    let embed = chars[start] == '!';
    let open = start + usize::from(embed);
    if chars.get(open) != Some(&'[') || chars.get(open + 1) != Some(&'[') {
        return None;
    }
    let inner = open + 2;
    let end = inner
        + chars[inner..]
            .iter()
            .take_while(|c| !matches!(c, '[' | ']' | '\n'))
            .count();
    if chars.get(end) != Some(&']') || chars.get(end + 1) != Some(&']') {
        return None;
    }
    let text: String = chars[inner..end].iter().collect();
    let target = text.split('|').next().unwrap_or_default();
    let path = target.split('#').next().unwrap_or_default();
    let link = Link {
        embed,
        target: python_strip(path).to_owned(),
        scheme: String::new(),
    };
    Some((link, end + 2))
}

/// The markdown link `(!?)\[([^\]\n]*)\]\((<[^>\n]*>|[^)\s]*)(?:[ \t]+("[^"\n]*"|'[^'\n]*'))?\)` at
/// `start`, and the index after it.
fn markdown_link_at(chars: &[char], start: usize) -> Option<(Link, usize)> {
    let embed = chars[start] == '!';
    let open = start + usize::from(embed);
    if chars.get(open) != Some(&'[') {
        return None;
    }
    let label_end = open
        + 1
        + chars[open + 1..]
            .iter()
            .take_while(|c| !matches!(c, ']' | '\n'))
            .count();
    if chars.get(label_end) != Some(&']') || chars.get(label_end + 1) != Some(&'(') {
        return None;
    }
    let destination = label_end + 2;
    let angled = if chars.get(destination) == Some(&'<') {
        let close = destination
            + 1
            + chars[destination + 1..]
                .iter()
                .take_while(|c| !matches!(c, '>' | '\n'))
                .count();
        (chars.get(close) == Some(&'>')).then_some(close + 1)
    } else {
        None
    };
    let bare = destination
        + chars[destination..]
            .iter()
            .take_while(|c| **c != ')' && !is_python_space(**c))
            .count();
    angled.into_iter().chain([bare]).find_map(|end| {
        let after = link_tail(chars, end)?;
        let text: String = chars[destination..end].iter().collect();
        Some((markdown_link(embed, &text), after))
    })
}

/// The end of a markdown link after its destination: an optional title, then `)`.
fn link_tail(chars: &[char], at: usize) -> Option<usize> {
    let spaces = chars[at..]
        .iter()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .count();
    if spaces > 0 {
        let quote = at + spaces;
        if let Some(mark @ ('"' | '\'')) = chars.get(quote) {
            let close = quote
                + 1
                + chars[quote + 1..]
                    .iter()
                    .take_while(|c| *c != mark && **c != '\n')
                    .count();
            if chars.get(close) == Some(mark) && chars.get(close + 1) == Some(&')') {
                return Some(close + 2);
            }
        }
    }
    (chars.get(at) == Some(&')')).then_some(at + 1)
}

/// A markdown link's target and scheme: a URL keeps its whole text and its scheme; a path is
/// percent-decoded, cut at `#` and stripped.
fn markdown_link(embed: bool, destination: &str) -> Link {
    let destination = destination
        .strip_prefix('<')
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(destination);
    if let Some(scheme) = scheme_of(destination).filter(|scheme| scheme.len() > 1) {
        return Link {
            embed,
            target: destination.to_owned(),
            scheme: fold(scheme),
        };
    }
    let path = destination.split('#').next().unwrap_or_default();
    Link {
        embed,
        target: python_strip(&unquote(path)).to_owned(),
        scheme: String::new(),
    }
}

/// The autolink `<([A-Za-z][A-Za-z0-9+.-]{1,31}:[^<>\s]*)>` at `start`, and the index after it.
fn autolink_at(chars: &[char], start: usize) -> Option<(Link, usize)> {
    if chars[start] != '<' || !chars.get(start + 1).is_some_and(char::is_ascii_alphabetic) {
        return None;
    }
    let run = chars[start + 2..]
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
        .count();
    let colon = start + 2 + run;
    if !(1..=31).contains(&run) || chars.get(colon) != Some(&':') {
        return None;
    }
    let close = colon
        + 1
        + chars[colon + 1..]
            .iter()
            .take_while(|c| !matches!(c, '<' | '>') && !is_python_space(**c))
            .count();
    if chars.get(close) != Some(&'>') {
        return None;
    }
    let url: String = chars[start + 1..close].iter().collect();
    let scheme = scheme_of(&url).map(fold).unwrap_or_default();
    let link = Link {
        embed: false,
        target: url,
        scheme,
    };
    Some((link, close + 1))
}

/// The scheme `^[A-Za-z][A-Za-z0-9+.-]*:` names at the start of `text`, without its colon.
fn scheme_of(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if !bytes.first()?.is_ascii_alphabetic() {
        return None;
    }
    let end = bytes
        .iter()
        .position(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-')))
        .unwrap_or(bytes.len());
    (bytes.get(end) == Some(&b':')).then(|| &text[..end])
}

/// `text` with its percent escapes decoded as UTF-8, invalid bytes replaced, as Python's
/// `urllib.parse.unquote` decodes a path.
fn unquote(text: &str) -> String {
    if !text.contains('%') {
        return text.to_owned();
    }
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escape = match bytes.get(at..at + 3) {
            Some([b'%', high, low]) if high.is_ascii_hexdigit() && low.is_ascii_hexdigit() => {
                Some(hex_value(*high) * 16 + hex_value(*low))
            }
            _ => None,
        };
        if let Some(byte) = escape {
            decoded.push(byte);
            at += 3;
        } else {
            decoded.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// The value of one hexadecimal digit, either case.
fn hex_value(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => digit
            .to_ascii_lowercase()
            .wrapping_sub(b'a')
            .wrapping_add(10),
    }
}

/// The extension `posixpath.splitext` reads from `path`: from its last dot, when the dot is in the
/// last component and something other than a dot comes before it there.
fn extension(path: &str) -> &str {
    let name = path.rfind('/').map_or(0, |slash| slash + 1);
    match path.rfind('.') {
        Some(dot) if dot > name && !path[name..dot].bytes().all(|byte| byte == b'.') => {
            &path[dot..]
        }
        _ => "",
    }
}
