//! The redactor: every registered secret, and every Telegram bot token shape, leaves a log line as
//! the redaction marker (SPEC-020 R12, R13).
//!
//! It ports the predecessor's `logging_redact.py:SecretRedactingFilter` at `27ee2bc`: registered
//! values of at least [`MIN_SECRET_LEN`] characters are replaced longest first, then every match of
//! the token pattern [`TELEGRAM_TOKEN_PATTERN`], each by [`REDACTED`]. Its output equals every
//! case of `goldens/redaction.json`. The pattern is matched by hand, because no regular-expression
//! crate is admitted, and its `\d` is Python's: every decimal digit of Unicode, not only `0` to `9`.
//!
//! The registry is shared: a [`Redactor`] is a handle, and the one the credential loader
//! registers into is the one the log writer reads, live, so a secret loaded after the logging was
//! installed is still redacted from the next line on.

use std::cmp::Ordering;
use std::fmt;
use std::io;
use std::sync::{Arc, PoisonError, RwLock};

use tracing::Metadata;
use tracing_subscriber::fmt::MakeWriter;

/// What a redacted value leaves a line as: the predecessor's `logging_redact.py:_REDACTED`.
pub const REDACTED: &str = "***REDACTED***";
/// The fewest characters a registered secret has, below which it would mangle ordinary text: the
/// predecessor's `logging_redact.py:_MIN_SECRET_LEN`, counted in characters, never bytes.
pub const MIN_SECRET_LEN: usize = 4;
/// The fewest digits of a token's numeric id in the predecessor's pattern.
pub const TOKEN_ID_MIN_DIGITS: usize = 6;
/// The fewest characters of a token's secret part in the predecessor's pattern.
pub const TOKEN_SECRET_MIN_CHARS: usize = 30;
/// The predecessor's `logging_redact.py:_TELEGRAM_TOKEN_RE`, which this module matches by hand:
/// at least [`TOKEN_ID_MIN_DIGITS`] decimal digits, a colon, and at least
/// [`TOKEN_SECRET_MIN_CHARS`] characters of `A-Z`, `a-z`, `0-9`, `_` and `-`.
pub const TELEGRAM_TOKEN_PATTERN: &str = r"\d{6,}:[A-Za-z0-9_-]{30,}";

/// The first code point of each run of ten decimal digits (Unicode `Nd`) that Python's `re` reads
/// as `\d`, as the predecessor's interpreter lists them (Unicode 15.0.0). Decimal digits come in
/// runs of ten from their zero (Unicode's stability policy), so a start names its whole run;
/// `the_redactor_reads_exactly_the_predecessors_decimal_digits` proves the table against the
/// redaction golden, character by character.
const DECIMAL_DIGIT_RUNS: [u32; 68] = [
    0x0030, 0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66, 0x0BE6, 0x0C66, 0x0CE6,
    0x0D66, 0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80,
    0x1A90, 0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0,
    0xFF10, 0x104A0, 0x10D30, 0x11066, 0x110F0, 0x11136, 0x111D0, 0x112F0, 0x11450, 0x114D0,
    0x11650, 0x116C0, 0x11730, 0x118E0, 0x11950, 0x11C50, 0x11D50, 0x11DA0, 0x11F50, 0x16A60,
    0x16AC0, 0x16B50, 0x1D7CE, 0x1D7D8, 0x1D7E2, 0x1D7EC, 0x1D7F6, 0x1E140, 0x1E2F0, 0x1E4F0,
    0x1E950, 0x1FBF0,
];

/// The secrets a [`Redactor`] replaces, each list longest first.
#[derive(Default)]
struct Registry {
    /// Every registered value, as the predecessor scrubs a message.
    values: Vec<String>,
    /// Every registered value, and each of its escaped spellings a log line can carry.
    line_forms: Vec<String>,
}

/// The shared registry of secrets to redact, and the scrub that applies it.
///
/// Its `Debug` shows how many secrets are registered, never one of them.
#[derive(Clone, Default)]
pub struct Redactor {
    registry: Arc<RwLock<Registry>>,
}

impl Redactor {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` so every line from now on is scrubbed of it. Returns whether it was newly
    /// registered: a value shorter than [`MIN_SECRET_LEN`] characters, or one already registered,
    /// is not, as the predecessor's `logging_redact.py:register` decides.
    #[allow(
        clippy::must_use_candidate,
        reason = "registering is the effect; whether the value was new is only a report"
    )]
    pub fn register(&self, value: &str) -> bool {
        if value.chars().count() < MIN_SECRET_LEN {
            return false;
        }
        let mut registry = self
            .registry
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if registry.values.iter().any(|known| known == value) {
            return false;
        }
        registry.values.push(value.to_owned());
        registry.values.sort_by(|a, b| longest_first(a, b));
        for form in escaped_forms(value) {
            if !registry.line_forms.contains(&form) {
                registry.line_forms.push(form);
            }
        }
        registry.line_forms.sort_by(|a, b| longest_first(a, b));
        true
    }

    /// How many secrets are registered: a count, never a value.
    #[must_use]
    pub fn registered(&self) -> usize {
        self.registry
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .values
            .len()
    }

    /// `text` with every registered secret, longest first, and then every token shape replaced by
    /// [`REDACTED`], exactly as the predecessor's filter scrubs a message.
    #[must_use]
    pub fn redact(&self, text: &str) -> String {
        let registry = self.registry.read().unwrap_or_else(PoisonError::into_inner);
        redact_tokens(&replace_longest_first(text, &registry.values))
    }

    /// A log line with every registered secret replaced, both as written and as a JSON string or a
    /// Rust debug string escapes it, so a secret holding a quote or a backslash cannot pass the
    /// scrub inside a JSON line; then every token shape.
    #[must_use]
    pub fn redact_line(&self, line: &str) -> String {
        let registry = self.registry.read().unwrap_or_else(PoisonError::into_inner);
        redact_tokens(&replace_longest_first(line, &registry.line_forms))
    }
}

impl fmt::Debug for Redactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Redactor")
            .field("registered", &self.registered())
            .finish()
    }
}

/// The predecessor's order: the longer value first, counted in characters, then by value.
fn longest_first(a: &str, b: &str) -> Ordering {
    b.chars()
        .count()
        .cmp(&a.chars().count())
        .then_with(|| a.cmp(b))
}

/// `value` as written, as a JSON string escapes it, and as a JSON string escapes its Rust debug
/// form: the spellings a JSON log line can carry it in.
fn escaped_forms(value: &str) -> Vec<String> {
    let mut forms = vec![value.to_owned()];
    let debug = format!("{value:?}");
    let debug = &debug[1..debug.len() - 1];
    for text in [value, debug] {
        if let Ok(quoted) = serde_json::to_string(text) {
            let escaped = quoted[1..quoted.len() - 1].to_owned();
            if !forms.contains(&escaped) {
                forms.push(escaped);
            }
        }
    }
    forms
}

/// `text` with each of `values`, in the order given, replaced wherever it stands by [`REDACTED`].
fn replace_longest_first(text: &str, values: &[String]) -> String {
    let mut text = text.to_owned();
    for value in values {
        if text.contains(value.as_str()) {
            text = text.replace(value.as_str(), REDACTED);
        }
    }
    text
}

/// Whether `c` is a decimal digit as Python's `re` reads `\d` in the predecessor's token pattern:
/// any character of Unicode's `Nd` category, not only `0` to `9`.
#[must_use]
pub fn is_decimal_digit(c: char) -> bool {
    if c.is_ascii_digit() {
        return true;
    }
    let code = u32::from(c);
    match DECIMAL_DIGIT_RUNS.binary_search(&code) {
        Ok(_) => true,
        Err(0) => false,
        Err(next) => code - DECIMAL_DIGIT_RUNS[next - 1] < 10,
    }
}

/// A character of a token's secret part: `[A-Za-z0-9_-]`, ASCII only.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// `text` with every non-overlapping match of the token pattern, leftmost first and each as long
/// as it runs, replaced by [`REDACTED`], as Python's `re.sub` replaces them.
fn redact_tokens(text: &str) -> String {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut at = 0;
    while at < chars.len() {
        if !is_decimal_digit(chars[at].1) {
            at += 1;
            continue;
        }
        let mut colon = at;
        while colon < chars.len() && is_decimal_digit(chars[colon].1) {
            colon += 1;
        }
        if colon - at >= TOKEN_ID_MIN_DIGITS && chars.get(colon).is_some_and(|&(_, c)| c == ':') {
            let mut end = colon + 1;
            while end < chars.len() && is_token_char(chars[end].1) {
                end += 1;
            }
            if end - (colon + 1) >= TOKEN_SECRET_MIN_CHARS {
                let start = chars[at].0;
                out.push_str(&text[copied..start]);
                out.push_str(REDACTED);
                copied = chars.get(end).map_or(text.len(), |&(byte, _)| byte);
                at = end;
                continue;
            }
        }
        // No match starts inside this run of digits: each later start has fewer digits before
        // the same colon and the same secret part.
        at = colon;
    }
    out.push_str(&text[copied..]);
    out
}

/// Makes each event's writer a [`RedactingWriter`] over the writer `inner` makes.
#[derive(Debug)]
pub struct RedactingMakeWriter<M> {
    redactor: Redactor,
    inner: M,
}

impl<M> RedactingMakeWriter<M> {
    /// Redacts, through `redactor`, every line written to the writers `inner` makes.
    #[must_use]
    pub const fn new(redactor: Redactor, inner: M) -> Self {
        Self { redactor, inner }
    }
}

impl<'a, M> MakeWriter<'a> for RedactingMakeWriter<M>
where
    M: MakeWriter<'a>,
{
    type Writer = RedactingWriter<'a, M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter::new(&self.redactor, self.inner.make_writer())
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        RedactingWriter::new(&self.redactor, self.inner.make_writer_for(meta))
    }
}

/// A writer that holds what it is given until a line is complete, then writes the line redacted,
/// so a secret split across two writes still meets the scrub whole.
pub struct RedactingWriter<'a, W: io::Write> {
    redactor: &'a Redactor,
    inner: W,
    pending: Vec<u8>,
}

impl<'a, W: io::Write> RedactingWriter<'a, W> {
    fn new(redactor: &'a Redactor, inner: W) -> Self {
        Self {
            redactor,
            inner,
            pending: Vec::new(),
        }
    }

    /// Writes `bytes`, redacted, to the inner writer.
    fn emit(&mut self, bytes: &[u8]) -> io::Result<()> {
        let line = self.redactor.redact_line(&String::from_utf8_lossy(bytes));
        self.inner.write_all(line.as_bytes())
    }
}

impl<W: io::Write> io::Write for RedactingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buf);
        if let Some(last) = self.pending.iter().rposition(|&byte| byte == b'\n') {
            let complete: Vec<u8> = self.pending.drain(..=last).collect();
            self.emit(&complete)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: io::Write> Drop for RedactingWriter<'_, W> {
    /// Writes, redacted, what an event left without a newline.
    fn drop(&mut self) {
        if !self.pending.is_empty() {
            let rest = std::mem::take(&mut self.pending);
            // An event's writer is dropped by the subscriber, which has nowhere to report to.
            let _unreported = self.emit(&rest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DECIMAL_DIGIT_RUNS, REDACTED, Redactor};

    #[test]
    fn every_run_of_decimal_digits_is_ten_numeric_characters_in_order() {
        assert!(DECIMAL_DIGIT_RUNS.is_sorted());
        for start in DECIMAL_DIGIT_RUNS {
            for digit in 0..10 {
                let c = char::from_u32(start + digit).expect("a character");
                assert!(c.is_numeric(), "U+{:04X} is not numeric", start + digit);
            }
        }
    }

    #[test]
    fn a_secret_split_across_writes_leaves_as_the_marker() {
        let redactor = Redactor::new();
        assert!(redactor.register("lantern-quiet-river"));
        let mut out = Vec::new();
        {
            let mut writer = super::RedactingWriter::new(&redactor, &mut out);
            std::io::Write::write_all(&mut writer, b"{\"value\":\"lantern-qu").expect("written");
            std::io::Write::write_all(&mut writer, b"iet-river\"}\n{\"next\":1}").expect("written");
        }
        assert_eq!(
            String::from_utf8(out).expect("UTF-8"),
            format!("{{\"value\":\"{REDACTED}\"}}\n{{\"next\":1}}")
        );
    }
}
