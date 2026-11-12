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
//!
//! The passkey library's events never leave the process at any level (SPEC-359 R13): the setup
//! drops every event under [`SILENCED_TARGETS`] in a filter of its own, which `RUST_LOG` never
//! reaches.

use std::fmt;
use std::panic::{self, PanicHookInfo};

use tracing::{Event, Level, Metadata, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::{FmtContext, MakeWriter};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::registry::LookupSpan;

use crate::error::KernelError;
use crate::redact::{RedactingMakeWriter, Redactor};

/// Installs the process's one subscriber: JSON on stdout at `RUST_LOG`'s level (info when unset),
/// every line redacted through `redactor`, whose registrations it reads live, and nothing under
/// [`SILENCED_TARGETS`] at any level; then replaces the panic hook, so a panic is logged through
/// that subscriber and never printed around it.
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
/// read exactly what a process would print. Nothing under [`SILENCED_TARGETS`] reaches it, whatever
/// `filter` enables ([`silence`]).
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
    silence(
        tracing_subscriber::fmt()
            .json()
            .event_format(JournalPriority(format))
            .with_writer(RedactingMakeWriter::new(redactor, make_writer))
            .with_env_filter(filter)
            .finish(),
    )
}

/// The crates whose events and spans never leave the process, whatever `RUST_LOG` says: the passkey
/// library's two (SPEC-359 R13). Their debug and trace events carry a ceremony's state, the
/// credential id and the public key, and their error and warn events would pass the default
/// filter. A target is silenced when it is one of these crates or a module under one.
pub const SILENCED_TARGETS: [&str; 2] = ["webauthn_rs", "webauthn_rs_core"];

/// `subscriber` behind a filter of its own, which drops every event and span under
/// [`SILENCED_TARGETS`] before `subscriber` sees it. The silence is no directive of the
/// `EnvFilter`: that filter ranks a longer target above a shorter one, and enables an event inside a
/// span a span directive names before it reads any target, so a `RUST_LOG` naming a module of the
/// library, or a span its calls run in, would outrank a fixed `off` there. A test that captures
/// the library's events wraps its capture in this, so it reads them as a log would.
pub fn silence<S>(subscriber: S) -> impl Subscriber + Send + Sync + 'static
where
    S: Subscriber + Send + Sync + 'static,
{
    subscriber.with(filter_fn(admitted))
}

/// Whether an event or span may leave the process: its target is no crate of
/// [`SILENCED_TARGETS`] and no module under one.
fn admitted(metadata: &Metadata<'_>) -> bool {
    let target = metadata.target();
    !SILENCED_TARGETS.iter().any(|silenced| {
        target
            .strip_prefix(silenced)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))
    })
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
