//! The one log setup of a process: JSON events on stdout, each line opening with its sd-daemon
//! priority, at `RUST_LOG`'s level, through a writer that redacts every line before it leaves the
//! process (SPEC-020 R13; the observability pack's `logging.rs.template`, ADR-020).
//!
//! journald strips a line's `<N>` prefix and files the line at that priority
//! (`SyslogLevelPrefix=` defaults to true), so `journalctl -p err` finds the errors; without it
//! every line lands at the unit's `SyslogLevel`, info. The JSON carries no timestamp: journald
//! stamps every line it receives, and the system time is read by the kernel's clock alone.

use std::fmt;

use tracing::{Event, Level, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::{FmtContext, MakeWriter};
use tracing_subscriber::registry::LookupSpan;

use crate::error::KernelError;
use crate::redact::{RedactingMakeWriter, Redactor};

/// Installs the process's one subscriber: JSON on stdout at `RUST_LOG`'s level (info when unset),
/// every line redacted through `redactor`, whose registrations it reads live.
///
/// # Errors
///
/// [`KernelError::LoggingInstalled`] when the process already installed one: a second install
/// returns an error and never panics.
pub fn install(redactor: &Redactor) -> Result<(), KernelError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing::subscriber::set_global_default(subscriber(redactor.clone(), std::io::stdout, filter))
        .map_err(|_| KernelError::LoggingInstalled)
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
