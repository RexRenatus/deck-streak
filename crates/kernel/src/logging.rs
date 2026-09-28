//! The one log setup of a process: JSON events on stdout, each line opening with its sd-daemon
//! priority, at `RUST_LOG`'s level, through a writer that redacts every line before it leaves the
//! process (SPEC-020 R13; the observability pack's `logging.rs.template`, ADR-020), and a panic
//! hook that logs a panic as one of those events (SPEC-031 R1).
//!
//! journald strips a line's `<N>` prefix and files the line at that priority
//! (`SyslogLevelPrefix=` defaults to true), so `journalctl -p err` finds the errors; without it
//! every line lands at the unit's `SyslogLevel`, info. The JSON carries no timestamp: journald
//! stamps every line it receives, and the system time is read by the kernel's clock alone.
//!
//! Rust's default panic hook writes a panic's message to stderr as plain text, past the redacting
//! writer, so a credential the message carried would reach the journal whole. The setup replaces it,
//! never chains to it: a panic, on whatever thread, is one ERROR event through the same writer.

use std::fmt;
use std::panic::{self, PanicHookInfo};

use tracing::{Event, Level, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::{FmtContext, MakeWriter};
use tracing_subscriber::registry::LookupSpan;

use crate::error::KernelError;
use crate::redact::{RedactingMakeWriter, Redactor};

/// Installs the process's one subscriber: JSON on stdout at `RUST_LOG`'s level (info when unset),
/// every line redacted through `redactor`, whose registrations it reads live; then replaces the
/// panic hook, so a panic is logged through that subscriber and never printed around it.
///
/// # Errors
///
/// [`KernelError::LoggingInstalled`] when the process already installed one: a second install
/// returns an error and never panics, and leaves the hook the first one set.
pub fn install(redactor: &Redactor) -> Result<(), KernelError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing::subscriber::set_global_default(subscriber(redactor.clone(), std::io::stdout, filter))
        .map_err(|_| KernelError::LoggingInstalled)?;
    panic::set_hook(Box::new(log_panic));
    Ok(())
}

/// Logs a panic as one ERROR event: its message, where it happened and on which thread. The event
/// passes the redacting writer like every other line, so a registered secret the message carries
/// leaves the process as the redaction marker, and journald files it at error priority, where the
/// alert unit quotes it (SPEC-031 R1, R3).
fn log_panic(info: &PanicHookInfo<'_>) {
    let message = info
        .payload_as_str()
        .unwrap_or("a panic whose payload is not text");
    let location = info
        .location()
        .map_or_else(String::new, ToString::to_string);
    let thread = std::thread::current();
    tracing::error!(
        panic = message,
        location = %location,
        thread = thread.name().unwrap_or("unnamed"),
        "a thread panicked"
    );
}

/// The subscriber [`install`] installs, writing to the writers `make_writer` makes, so a test can
/// read exactly what a process would print.
pub fn subscriber<M>(
    redactor: Redactor,
    make_writer: M,
    filter: EnvFilter,
) -> impl Subscriber + Send + Sync + 'static
where
    M: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    let format = tracing_subscriber::fmt::format()
        .json()
        .flatten_event(true)
        .without_time();
    tracing_subscriber::fmt()
        .json()
        .event_format(JournalPriority(format))
        .with_writer(RedactingMakeWriter::new(redactor, make_writer))
        .with_env_filter(filter)
        .finish()
}

/// Opens each line with its event's sd-daemon(3) priority, so journald files it at that level.
struct JournalPriority<F>(F);

impl<S, N, F> FormatEvent<S, N> for JournalPriority<F>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    F: FormatEvent<S, N>,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let priority = match *event.metadata().level() {
            Level::ERROR => "<3>",
            Level::WARN => "<4>",
            Level::INFO => "<6>",
            Level::DEBUG | Level::TRACE => "<7>",
        };
        writer.write_str(priority)?;
        self.0.format_event(ctx, writer, event)
    }
}
