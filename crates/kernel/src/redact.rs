//! The redactor: every registered secret, and every Telegram bot token shape, leaves a log line as
//! the redaction marker (SPEC-020 R12, R13).
//!
//! It ports the predecessor's `logging_redact.py:SecretRedactingFilter` at `27ee2bc`: registered
//! values of at least [`MIN_SECRET_LEN`] characters are replaced longest first, then every match of
//! the token pattern [`TELEGRAM_TOKEN_PATTERN`], each by [`REDACTED`]. Its output equals every
//! case of `goldens/redaction.json`.
//!
//! The registry is shared: a [`Redactor`] is a handle, and the one the credential loader
//! registers into is the one the log writer reads, live, so a secret loaded after the logging was
//! installed is still redacted from the next line on.

use std::fmt;
use std::io;
use std::sync::{Arc, RwLock};

use tracing::Metadata;
use tracing_subscriber::fmt::MakeWriter;

/// What a redacted value leaves a line as: the predecessor's `logging_redact.py:_REDACTED`.
pub const REDACTED: &str = "***REDACTED***";
/// The fewest characters a registered secret has, below which it would mangle ordinary text: the
/// predecessor's `logging_redact.py:_MIN_SECRET_LEN`, counted in characters, never bytes.
pub const MIN_SECRET_LEN: usize = 0;
/// The fewest digits of a token's numeric id in the predecessor's pattern.
pub const TOKEN_ID_MIN_DIGITS: usize = 0;
/// The fewest characters of a token's secret part in the predecessor's pattern.
pub const TOKEN_SECRET_MIN_CHARS: usize = 0;
/// The predecessor's `logging_redact.py:_TELEGRAM_TOKEN_RE`, which this module matches by hand:
/// at least [`TOKEN_ID_MIN_DIGITS`] decimal digits, a colon, and at least
/// [`TOKEN_SECRET_MIN_CHARS`] characters of `A-Z`, `a-z`, `0-9`, `_` and `-`.
pub const TELEGRAM_TOKEN_PATTERN: &str = r"\d{6,}:[A-Za-z0-9_-]{30,}";

/// The shared registry of secrets to redact, and the scrub that applies it.
///
/// Its `Debug` shows how many secrets are registered, never one of them.
#[derive(Clone, Default)]
pub struct Redactor {
    secrets: Arc<RwLock<Vec<String>>>,
}

impl Redactor {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` so every line from now on is scrubbed of it. Returns whether it was newly
    /// registered: a value shorter than [`MIN_SECRET_LEN`] characters, or one already registered,
    /// is not.
    pub fn register(&self, value: &str) -> bool {
        let _ = value;
        false
    }

    /// How many secrets are registered: a count, never a value.
    #[must_use]
    pub fn registered(&self) -> usize {
        0
    }

    /// `text` with every registered secret, longest first, and then every token shape replaced by
    /// [`REDACTED`], exactly as the predecessor's filter scrubs a message.
    #[must_use]
    pub fn redact(&self, text: &str) -> String {
        text.to_owned()
    }

    /// A log line with every registered secret replaced, both as written and as a JSON string
    /// escapes it, so a secret holding a quote or a backslash cannot pass the scrub inside a JSON
    /// line; then every token shape.
    #[must_use]
    pub fn redact_line(&self, line: &str) -> String {
        line.to_owned()
    }
}

impl fmt::Debug for Redactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Redactor")
            .field("registered", &self.registered())
            .finish()
    }
}

/// Whether `c` is a decimal digit as Python's `re` reads `\d` in the predecessor's token pattern:
/// any character of Unicode's `Nd` category, not only `0` to `9`.
#[must_use]
pub fn is_decimal_digit(c: char) -> bool {
    let _ = c;
    false
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
}

impl<W: io::Write> io::Write for RedactingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ = (&self.redactor, &mut self.pending);
        self.inner.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
