//! The drill notes' contract (SPEC-110 R1 to R11): reading a drill note, listing and viewing the
//! Active folder, the queue and the rollup, the answer's append, the graded parse and the key rule.
//!
//! Every function here is the predecessor's (`vault_bridge.py` at `27ee2bc`, `nudges.py:
//! NudgesLayer.drill_queue`) written over text. The predecessor read a note through Python's
//! universal newlines and split it with `str.splitlines`, stripped with `str.strip`, and matched
//! the ready marker with a regular expression whose `\s` crosses lines: each of those is written
//! out below, because `str::lines`, `str::trim` and a plain search differ from them on a note the
//! owner's editor can produce. Two limits are the port's own and are documented where they sit:
//! a digit is an ASCII digit, and the subject fold lowercases where the predecessor casefolds.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::StudyDay;

use crate::sha256;
use crate::unicode_other::is_other;

/// The folder inside the drills folder that holds the drills waiting for the owner or the grader.
pub const ACTIVE: &str = "Active";
/// The folder inside the drills folder that holds the graded drills.
pub const GRADED: &str = "Graded";
/// The XP a graded drill pays when its note names none that parses.
pub const POSTBACK_XP: i64 = 15;
/// The least XP a graded drill pays: an XP the note names below it is raised to it.
pub const XP_MIN: i64 = 10;
/// The most XP a graded drill pays: an XP the note names above it is lowered to it.
pub const XP_MAX: i64 = 25;
/// The most characters a deferral reason keeps, the cut's `…` included.
pub const DEFER_REASON_MAX_LEN: usize = 120;
/// The unticked ready marker the answer ticks.
pub const READY_UNTICKED: &str = "- [ ] **Ready for grading**";
/// The ticked ready marker.
pub const READY_TICKED: &str = "- [x] **Ready for grading**";
/// The text after the marker's checkbox, which the ready marker line always carries.
const READY_LABEL: &str = "**Ready for grading**";
/// The headings that begin a drill's answer area, across the four template shapes.
const ANSWER_HEADINGS: [&str; 5] = [
    "## Free Recall",
    "## Issue",
    "## Court & Year",
    "## From-Memory Outline",
    "## Self-Check",
];
/// The heading of the checklist, which is not an answer section.
const SELF_CHECK: &str = "Self-Check";
/// The grant grammar's own limit on a source, in bytes (`deck_streak_progression::grant::
/// SOURCE_MAX_LEN`, SPEC-040 R2). The vault depends on the kernel only, so the limit is mirrored
/// here and a test in the coordination crate, which sees both, asserts the two are equal.
pub const GRANT_SOURCE_MAX: usize = 128;
/// The prefix of every drill's grant key: one name for the key builder and the id bound.
pub const KEY_PREFIX: &str = "drill:";
/// The longest a grade key's id may be to be kept as it is: the whole key `drill:<id>` must fit the
/// grant grammar's limit (SPEC-040 R2), so the id holds the limit less the prefix (122 bytes).
pub const KEY_ID_MAX: usize = GRANT_SOURCE_MAX - KEY_PREFIX.len();
/// The hex characters of the hash a hashed key carries.
const KEY_HASH_HEX: usize = 32;

/// Whether Python's `str.isspace` holds for `c`: Unicode white space, and the four separators
/// `\x1c` to `\x1f`, which Rust's `char::is_whitespace` leaves out.
fn py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python's `str.strip()`.
#[must_use]
pub fn py_strip(text: &str) -> &str {
    text.trim_matches(py_space)
}

/// Python's `str.splitlines()`: the line breaks Python knows, and no empty last line.
fn py_splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        let is_break = matches!(
            c,
            '\n' | '\r'
                | '\u{0b}'
                | '\u{0c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if !is_break {
            continue;
        }
        lines.push(&text[start..at]);
        let mut end = at + c.len_utf8();
        if c == '\r' && matches!(chars.peek(), Some((_, '\n'))) {
            chars.next();
            end += 1;
        }
        start = end;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// The text as Python's `read_text` gives it: `\r\n` and a lone `\r` are one `\n`.
#[must_use]
pub fn universal_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Whether `stem` names a note directly inside the Active folder: not empty, and with no `/`, no
/// `\` and no `..` (`_safe_stem`).
#[must_use]
pub fn safe_stem(stem: &str) -> bool {
    !stem.is_empty() && !stem.contains('/') && !stem.contains('\\') && !stem.contains("..")
}

/// The flat `key: value` scalars of the note's leading frontmatter block (`_flat_frontmatter`): a
/// note that does not start with `---` has none, a line whose stripped text is `---` ends the
/// block, an indented, dashed or tabbed key is skipped and the last of a repeated key wins.
#[must_use]
pub fn flat_frontmatter(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if !text.starts_with("---") {
        return out;
    }
    for line in py_splitlines(text).into_iter().skip(1) {
        if py_strip(line) == "---" {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let first = key.chars().next();
        if !py_strip(key).is_empty() && !matches!(first, Some(' ' | '-' | '\t')) {
            out.insert(py_strip(key).to_owned(), py_strip(value).to_owned());
        }
    }
    out
}

/// The note's body with a leading `---` block removed (`_strip_frontmatter`).
#[must_use]
pub fn strip_frontmatter(text: &str) -> String {
    if !text.starts_with("---") {
        return text.to_owned();
    }
    let lines = py_splitlines(text);
    for (index, line) in lines.iter().enumerate().skip(1) {
        if py_strip(line) == "---" {
            return lines[index + 1..].join("\n");
        }
    }
    text.to_owned()
}

/// The first `# ` heading of `body`, stripped (`_drill_title`), or nothing.
fn title_of(body: &str) -> String {
    py_splitlines(body)
        .into_iter()
        .find_map(|line| line.strip_prefix("# "))
        .map(|title| py_strip(title).to_owned())
        .unwrap_or_default()
}

/// Where the prompt ends: the earliest answer heading, or the end of the body.
fn prompt_cut(body: &str) -> usize {
    ANSWER_HEADINGS
        .iter()
        .filter_map(|heading| body.find(&format!("\n{heading}")))
        .min()
        .unwrap_or(body.len())
}

/// `text` without its `<!-- ... -->` comments, which may span lines (`re.sub` with `DOTALL`).
fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("<!--") {
        let after = &rest[open + 4..];
        let Some(close) = after.find("-->") else {
            break;
        };
        out.push_str(&rest[..open]);
        rest = &after[close + 3..];
    }
    out.push_str(rest);
    out
}

/// The prompt the owner reads: the body up to the earliest answer heading, comments dropped,
/// stripped (`_drill_prompt`).
#[must_use]
pub fn prompt_of(body: &str) -> String {
    py_strip(&without_comments(&body[..prompt_cut(body)])).to_owned()
}

/// The first `YYYY-MM-DD` of `text` that reads as a calendar date from year 1, or nothing.
///
/// The predecessor's `\d` also matched other scripts' digits; here a digit is an ASCII digit.
fn first_date(text: &str) -> Option<Result<StudyDay, ()>> {
    let bytes = text.as_bytes();
    let shaped = |at: usize| -> bool {
        bytes.get(at..at + 10).is_some_and(|w| {
            w.iter().enumerate().all(|(i, b)| match i {
                4 | 7 => *b == b'-',
                _ => b.is_ascii_digit(),
            })
        })
    };
    let at = (0..bytes.len()).find(|&at| shaped(at))?;
    let candidate = &text[at..at + 10];
    if candidate.starts_with("0000") {
        return Some(Err(()));
    }
    Some(candidate.parse::<StudyDay>().map_err(|_| ()))
}

/// The drill's creation day: the `created` value if it holds a date, else the stem's
/// (`_drill_date`). A first candidate that holds a date that is no date falls to the next.
fn created_of(stem: &str, frontmatter: &BTreeMap<String, String>) -> Option<StudyDay> {
    let created = frontmatter.get("created").map_or("", String::as_str);
    for candidate in [created, stem] {
        if let Some(Ok(day)) = first_date(candidate) {
            return Some(day);
        }
    }
    None
}

/// The drill's type: `type: drill-<kind>` with its quotes and prefix removed, else the stem before
/// its date (`_drill_type`).
fn type_of(stem: &str, frontmatter: &BTreeMap<String, String>) -> String {
    let raw = frontmatter
        .get("type")
        .map_or("", |value| py_strip(value).trim_matches('"'));
    if !raw.is_empty() {
        return raw.strip_prefix("drill-").unwrap_or(raw).to_owned();
    }
    let bytes = stem.as_bytes();
    let at = (0..bytes.len()).find(|&at| {
        bytes.get(at..at + 10).is_some_and(|w| {
            w.iter().enumerate().all(|(i, b)| match i {
                4 | 7 => *b == b'-',
                _ => b.is_ascii_digit(),
            })
        })
    });
    match at {
        Some(at) if at > 0 => stem[..at].trim_end_matches('-').to_owned(),
        _ => String::new(),
    }
}

/// Where the ready marker sits in `raw`, and whether its box is ticked (`_READY_MARKER_RE`).
///
/// The pattern is `^[ \t]*-\s*\[([ xX])\]\s*\*\*Ready for grading\*\*` under `MULTILINE`, whose
/// `\s` also crosses a line break, so a bullet and its box may sit on two lines.
fn ready_marker(raw: &str) -> Option<(usize, bool)> {
    let starts = std::iter::once(0).chain(raw.match_indices('\n').map(|(at, _)| at + 1));
    for start in starts {
        let mut rest = raw[start..].trim_start_matches([' ', '\t']);
        let Some(after_dash) = rest.strip_prefix('-') else {
            continue;
        };
        rest = after_dash.trim_start_matches(py_space);
        let Some(after_open) = rest.strip_prefix('[') else {
            continue;
        };
        let mut chars = after_open.chars();
        let Some(mark) = chars.next().filter(|m| matches!(m, ' ' | 'x' | 'X')) else {
            continue;
        };
        let Some(after_box) = chars.as_str().strip_prefix(']') else {
            continue;
        };
        if after_box
            .trim_start_matches(py_space)
            .starts_with(READY_LABEL)
        {
            return Some((start, mark != ' '));
        }
    }
    None
}

/// Whether the ready marker is ticked (`_drill_answered`); no marker reads as unanswered.
#[must_use]
pub fn answered(raw: &str) -> bool {
    ready_marker(raw).is_some_and(|(_, ticked)| ticked)
}

/// A drill's metadata: what a list, a queue and a rollup carry (`_read_drill_meta`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrillMeta {
    /// The note's stem, which is the drill's id.
    pub drill_id: String,
    /// The drill's type, `irac`, `outline` and so on.
    pub kind: String,
    /// The drill's subject, unquoted.
    pub subject: String,
    /// The first `# ` heading, or the id.
    pub title: String,
    /// The day the drill was made, when the note or its stem says.
    pub created: Option<StudyDay>,
    /// The study days since it was made, never below zero.
    pub age_days: Option<i64>,
    /// Whether the ready marker is ticked.
    pub answered: bool,
    /// Whether the note carries a `defer_reason` key, whatever its value.
    pub deferred: bool,
}

/// The metadata of the note `stem` whose text is `raw` (already read with universal newlines),
/// as of study day `today`.
#[must_use]
pub fn read_meta(stem: &str, raw: &str, today: StudyDay) -> DrillMeta {
    let frontmatter = flat_frontmatter(raw);
    let body = strip_frontmatter(raw);
    let created = created_of(stem, &frontmatter);
    let title = title_of(&body);
    DrillMeta {
        drill_id: stem.to_owned(),
        kind: type_of(stem, &frontmatter),
        subject: frontmatter
            .get("subject")
            .map_or("", |value| py_strip(value).trim_matches('"'))
            .to_owned(),
        title: if title.is_empty() {
            stem.to_owned()
        } else {
            title
        },
        created,
        age_days: created.map(|day| (today.epoch_day() - day.epoch_day()).max(0)),
        answered: answered(raw),
        deferred: frontmatter.contains_key("defer_reason"),
    }
}

/// The single view of one drill: its metadata, the prompt, the sanitised deferral reason and, for
/// the drill workspace, the answer sections and the self-check (R2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrillView {
    /// The metadata.
    pub meta: DrillMeta,
    /// The prompt the owner reads.
    pub prompt: String,
    /// The deferral reason, sanitised; empty when the note names none.
    pub defer_reason: String,
    /// The note's `## ` headings from the prompt's cut, in order, without `Self-Check` and without
    /// any heading after the ready marker.
    pub sections: Vec<String>,
    /// The text of each checklist line under `## Self-Check` other than the ready marker.
    pub self_check: Vec<String>,
}

/// The view of the note `stem` whose text is `raw`, as of `today`.
#[must_use]
pub fn read_view(stem: &str, raw: &str, today: StudyDay) -> DrillView {
    let frontmatter = flat_frontmatter(raw);
    let body = strip_frontmatter(raw);
    let (sections, self_check) = outline(&body);
    DrillView {
        meta: read_meta(stem, raw, today),
        prompt: prompt_of(&body),
        defer_reason: sanitise_defer_reason(
            frontmatter.get("defer_reason").map_or("", String::as_str),
        ),
        sections,
        self_check,
    }
}

/// The answer sections and the self-check of a body (R2).
fn outline(body: &str) -> (Vec<String>, Vec<String>) {
    let cut = prompt_cut(body);
    let tail = body[cut..].trim_start_matches('\n');
    let mut sections = Vec::new();
    let mut checks = Vec::new();
    let mut in_self_check = false;
    let mut ready_seen = false;
    for line in py_splitlines(tail) {
        if ready_marker(line).is_some() {
            ready_seen = true;
        }
        if ready_seen {
            break;
        }
        if let Some(title) = line.strip_prefix("## ") {
            let title = py_strip(title);
            in_self_check = title == SELF_CHECK;
            if !in_self_check {
                sections.push(title.to_owned());
            }
            continue;
        }
        if in_self_check && let Some(item) = checklist_text(line) {
            checks.push(item);
        }
    }
    (sections, checks)
}

/// The text of a checklist line `- [ ] text` or `- [x] text`, or nothing for any other line.
fn checklist_text(line: &str) -> Option<String> {
    let rest = line.trim_start_matches([' ', '\t']).strip_prefix('-')?;
    let rest = rest.trim_start_matches(py_space).strip_prefix('[')?;
    let mut chars = rest.chars();
    chars.next().filter(|m| matches!(m, ' ' | 'x' | 'X'))?;
    let text = py_strip(chars.as_str().strip_prefix(']')?);
    (!text.is_empty()).then(|| text.to_owned())
}

/// The deferral reason as safe plain text (`_sanitise_defer_reason`): a bare block scalar
/// indicator is nothing, every control, format, unassigned and private-use character is dropped,
/// every other space becomes one ASCII space, runs collapse, and the reason is cut to
/// [`DEFER_REASON_MAX_LEN`] characters with a `…`.
#[must_use]
pub fn sanitise_defer_reason(raw: &str) -> String {
    let stripped = py_strip(raw);
    let mut indicator = stripped.chars();
    if matches!(indicator.next(), Some('>' | '|')) && matches!(indicator.as_str(), "" | "+" | "-") {
        return String::new();
    }
    let mut collapsed = String::new();
    let mut previous_space = false;
    for c in raw.chars().filter(|c| !is_other(*c)) {
        let c = if py_space(c) { ' ' } else { c };
        if c == ' ' && previous_space {
            continue;
        }
        previous_space = c == ' ';
        collapsed.push(c);
    }
    let collapsed = py_strip(&collapsed);
    if collapsed.chars().count() <= DEFER_REASON_MAX_LEN {
        return collapsed.to_owned();
    }
    let cut: String = collapsed.chars().take(DEFER_REASON_MAX_LEN - 1).collect();
    format!("{}…", cut.trim_end_matches(py_space))
}

/// The drills a list, a queue and a rollup count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Queue {
    /// The unanswered drills, in the list's order.
    pub unanswered: Vec<DrillMeta>,
    /// How many are answered and wait for the grader.
    pub awaiting_grading: usize,
    /// How many of those are deferred.
    pub deferred: usize,
}

/// The queue of `all` (`NudgesLayer.drill_queue`).
#[must_use]
pub fn queue(all: &[DrillMeta]) -> Queue {
    Queue {
        unanswered: all.iter().filter(|d| !d.answered).cloned().collect(),
        awaiting_grading: all.iter().filter(|d| d.answered).count(),
        deferred: all.iter().filter(|d| d.answered && d.deferred).count(),
    }
}

/// The counts a rollup reports, integers and dates only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rollup {
    /// The unanswered drills.
    pub active: usize,
    /// The answered drills.
    pub awaiting_grading: usize,
    /// The greatest age among the unanswered.
    pub oldest_age_days: Option<i64>,
    /// The day the oldest unanswered drill was made.
    pub oldest_created: Option<StudyDay>,
    /// The drills whose subject is none of the known subjects.
    pub unmatched_active: usize,
    /// The answered drills that are deferred.
    pub deferred: usize,
}

/// A subject as a comparison key (`_fold_subject_key`): `&` is `and`, white space is gone, and the
/// text is lowercased. The predecessor casefolds, which also folds `ß` to `ss`; that is the one
/// case-fold this port adds, and no other differs on the subjects a law vocabulary holds.
fn fold_subject(name: &str) -> String {
    name.replace('&', "and")
        .chars()
        .filter(|c| !py_space(*c))
        .collect::<String>()
        .to_lowercase()
        .replace('ß', "ss")
}

/// The rollup of `all` against the known `subjects` (`_active_drill_rollup`).
#[must_use]
pub fn rollup(all: &[DrillMeta], subjects: &BTreeSet<String>) -> Rollup {
    let known: BTreeSet<String> = subjects.iter().map(|s| fold_subject(s)).collect();
    let mut oldest: Option<(i64, Option<StudyDay>)> = None;
    for drill in all.iter().filter(|d| !d.answered) {
        if let Some(age) = drill.age_days
            && oldest.is_none_or(|(best, _)| age > best)
        {
            oldest = Some((age, drill.created));
        }
    }
    let unanswered = all.iter().filter(|d| !d.answered).count();
    Rollup {
        active: unanswered,
        awaiting_grading: all.len() - unanswered,
        oldest_age_days: oldest.map(|(age, _)| age),
        oldest_created: oldest.and_then(|(_, created)| created),
        unmatched_active: all
            .iter()
            .filter(|d| !known.contains(&fold_subject(&d.subject)))
            .count(),
        deferred: all.iter().filter(|d| d.answered && d.deferred).count(),
    }
}

/// What an answer's append made of a note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Appended {
    /// The marker was ticked and the answer added; `text` is the note as written, `title` its
    /// title or its id.
    Written {
        /// The note's new text.
        text: String,
        /// The drill's title, or its id.
        title: String,
    },
    /// The ready marker is already ticked: nothing is written.
    AlreadyAnswered,
}

/// The note `raw` with `answer` appended under a `## Your Answer (via Telegram {when})` heading
/// and its ready marker ticked (`append_drill_answer`), or the refusal when it is ticked already.
/// The status stays as it was.
#[must_use]
pub fn append_answer(stem: &str, raw: &str, answer: &str, when: &str) -> Appended {
    if answered(raw) {
        return Appended::AlreadyAnswered;
    }
    let ticked = raw.replacen(READY_UNTICKED, READY_TICKED, 1);
    let text = format!(
        "{}\n\n## Your Answer (via Telegram {when})\n\n{}\n",
        ticked.trim_end_matches('\n'),
        py_strip(answer)
    );
    let title = title_of(&strip_frontmatter(raw));
    Appended::Written {
        text,
        title: if title.is_empty() {
            stem.to_owned()
        } else {
            title
        },
    }
}

/// A graded drill: its id and the XP it pays, already clamped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GradedDrill {
    /// The note's stem.
    pub drill_id: String,
    /// The drill's type, as a list shows it; the grade row keeps it.
    pub kind: String,
    /// The drill's subject, unquoted; the grade row keeps it.
    pub subject: String,
    /// The XP, between [`XP_MIN`] and [`XP_MAX`].
    pub xp: i64,
}

/// Python's `int(text)` for the text a frontmatter value can hold: an optional sign, then digits
/// with a single underscore allowed between two digits. A digit is an ASCII digit here, where
/// Python also takes other scripts' digits. A value too large for `i64` saturates, which the
/// clamp reads as Python's unbounded integer.
fn py_int(text: &str) -> Option<i64> {
    let text = py_strip(text);
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let bytes = digits.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() || !bytes[bytes.len() - 1].is_ascii_digit() {
        return None;
    }
    let mut value: i64 = 0;
    let mut previous_underscore = false;
    for &byte in bytes {
        match byte {
            b'_' if previous_underscore => return None,
            b'_' => previous_underscore = true,
            b'0'..=b'9' => {
                previous_underscore = false;
                value = value
                    .saturating_mul(10)
                    .saturating_add(i64::from(byte - b'0'));
            }
            _ => return None,
        }
    }
    Some(if negative { -value } else { value })
}

/// The graded drill in the note `stem` whose text is `raw` (`_parse_graded_drill`), or nothing for
/// a note without frontmatter, one whose `status` is not `graded`, and one with a blank stem. The
/// XP is the note's `xp` when it parses, else [`POSTBACK_XP`], clamped to the band.
#[must_use]
pub fn parse_graded(stem: &str, raw: &str) -> Option<GradedDrill> {
    let frontmatter = flat_frontmatter(raw);
    if frontmatter.is_empty() || frontmatter.get("status").map(String::as_str) != Some("graded") {
        return None;
    }
    if py_strip(stem).is_empty() {
        return None;
    }
    let xp = frontmatter
        .get("xp")
        .and_then(|value| py_int(value))
        .unwrap_or(POSTBACK_XP);
    Some(GradedDrill {
        drill_id: stem.to_owned(),
        kind: type_of(stem, &frontmatter),
        subject: frontmatter
            .get("subject")
            .map_or("", |value| py_strip(value).trim_matches('"'))
            .to_owned(),
        xp: xp.clamp(XP_MIN, XP_MAX),
    })
}

/// The source key a drill's grant carries (R10): `drill:<id>` when that key fits
/// `^[a-z0-9][a-z0-9:._-]{0,127}$` and the id does not begin `h.`, else `drill:h.` and the first 32
/// hex characters of the SHA-256 of the id. A hashed key can never equal a kept one.
#[must_use]
pub fn drill_key(id: &str) -> String {
    let mut chars = id.chars();
    let fits = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && id.len() <= KEY_ID_MAX
        && chars.all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, ':' | '.' | '_' | '-')
        });
    if fits && !id.starts_with("h.") {
        return format!("{KEY_PREFIX}{id}");
    }
    let hex = sha256::hex(&sha256::digest(id.as_bytes()));
    format!("{KEY_PREFIX}h.{}", &hex[..KEY_HASH_HEX])
}
